use reqwest::header::{HeaderMap, RETRY_AFTER};
use std::collections::HashMap;
use std::sync::{
    Arc, LazyLock, Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub(crate) const DEFAULT_RETRY_BACKOFF_CAP_MS: u64 = 10_000;
pub(crate) const DEFAULT_PROVIDER_RATE_LIMIT_COOLDOWN_CAP_MS: u64 = 5 * 60 * 1_000;
pub(crate) const DEFAULT_PROVIDER_CONCURRENCY_LIMIT: usize = 2;
const PROVIDER_RATE_LIMIT_COOLDOWN_CAP_ENV: &str = "JCODE_PROVIDER_RATE_LIMIT_COOLDOWN_CAP_MS";
const PROVIDER_CONCURRENCY_LIMIT_ENV: &str = "JCODE_PROVIDER_MAX_CONCURRENT_PER_MODEL";

static RETRY_JITTER_COUNTER: AtomicU64 = AtomicU64::new(1);
static PROVIDER_RUNTIME_STATE_REVISION: AtomicU64 = AtomicU64::new(1);
static PROVIDER_RATE_LIMIT_COOLDOWNS: LazyLock<Mutex<HashMap<String, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static PROVIDER_CONCURRENCY_LIMITERS: LazyLock<Mutex<HashMap<String, Arc<Semaphore>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) fn should_eager_detect_copilot_tier() -> bool {
    std::env::var("JCODE_NON_INTERACTIVE").is_err()
}

pub(crate) fn retry_after_secs_from_headers(
    headers: &HeaderMap,
    now: chrono::DateTime<chrono::Utc>,
) -> Option<u64> {
    headers
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| parse_retry_after_secs(value, now))
}

pub(crate) fn parse_retry_after_secs(raw: &str, now: chrono::DateTime<chrono::Utc>) -> Option<u64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(seconds) = trimmed.parse::<u64>() {
        return Some(seconds);
    }

    let retry_at = parse_retry_after_http_date(trimmed)?;
    if retry_at <= now {
        return Some(0);
    }

    retry_at
        .signed_duration_since(now)
        .num_seconds()
        .try_into()
        .ok()
}

fn parse_retry_after_http_date(raw: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    if let Ok(parsed) = chrono::DateTime::parse_from_rfc2822(raw) {
        return Some(parsed.with_timezone(&chrono::Utc));
    }

    // RFC 9110 Retry-After uses HTTP-date. IMF-fixdate is current, but
    // accepting obsolete formats keeps old intermediaries from collapsing into
    // an immediate retry.
    const HTTP_DATE_FORMATS: &[&str] = &[
        "%a, %d %b %Y %H:%M:%S GMT",
        "%A, %d-%b-%y %H:%M:%S GMT",
        "%a %b %e %H:%M:%S %Y",
    ];

    HTTP_DATE_FORMATS.iter().find_map(|format| {
        chrono::NaiveDateTime::parse_from_str(raw, format)
            .ok()
            .map(|naive| chrono::DateTime::from_naive_utc_and_offset(naive, chrono::Utc))
    })
}

pub(crate) fn retry_after_suffix(retry_after_secs: Option<u64>) -> String {
    retry_after_secs
        .map(|seconds| format!(" (retry after {}s)", seconds))
        .unwrap_or_default()
}

pub(crate) fn provider_wait_status_duration(delay_ms: u64) -> String {
    let duration = Duration::from_millis(delay_ms);
    let secs = duration.as_secs();
    if secs >= 3600 {
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        format!("{}h {}m", hours, mins)
    } else if secs >= 60 {
        let mins = secs / 60;
        let rem_secs = secs % 60;
        format!("{}m {}s", mins, rem_secs)
    } else if secs > 0 {
        format!("{}s", secs)
    } else if delay_ms > 0 {
        "<1s".to_string()
    } else {
        "0s".to_string()
    }
}

pub(crate) fn retry_backoff_delay_ms(attempt: u32, base_delay_ms: u64, cap_delay_ms: u64) -> u64 {
    retry_backoff_delay_ms_for_nonce(attempt, base_delay_ms, cap_delay_ms, retry_jitter_nonce())
}

pub(crate) fn retry_delay_ms_for_error(
    attempt: u32,
    base_delay_ms: u64,
    cap_delay_ms: u64,
    error_str: &str,
) -> u64 {
    if attempt == 0 {
        return 0;
    }

    retry_after_delay_ms_from_error(error_str, cap_delay_ms)
        .unwrap_or_else(|| retry_backoff_delay_ms(attempt, base_delay_ms, cap_delay_ms))
}

pub(crate) fn retry_after_delay_ms_from_error(error_str: &str, cap_delay_ms: u64) -> Option<u64> {
    retry_after_secs_from_error(error_str)
        .map(|seconds| seconds.saturating_mul(1_000).min(cap_delay_ms))
}

pub(crate) fn provider_rate_limit_cooldown_delay_ms_for_error(
    error_str: &str,
    retry_attempt: u32,
    base_delay_ms: u64,
    retry_cap_delay_ms: u64,
) -> u64 {
    let cooldown_cap_ms = provider_rate_limit_cooldown_cap_ms();
    if cooldown_cap_ms == 0 {
        return 0;
    }

    retry_after_secs_from_error(error_str)
        .map(|seconds| seconds.saturating_mul(1_000).min(cooldown_cap_ms))
        .unwrap_or_else(|| {
            retry_delay_ms_for_error(retry_attempt, base_delay_ms, retry_cap_delay_ms, error_str)
                .min(cooldown_cap_ms)
        })
}

pub(crate) fn provider_rate_limit_cooldown_remaining_ms(
    provider: &str,
    model: &str,
) -> Option<u64> {
    let key = provider_rate_limit_key(provider, model)?;
    let now = Instant::now();
    let mut guard = PROVIDER_RATE_LIMIT_COOLDOWNS.lock().ok()?;
    let until = guard.get(&key).copied()?;
    if until <= now {
        guard.remove(&key);
        bump_provider_runtime_state_revision();
        return None;
    }

    Some(duration_to_ms_ceil(until.duration_since(now)))
}

pub(crate) fn record_provider_rate_limit_cooldown_for_retry(
    provider: &str,
    model: &str,
    error_str: &str,
    retry_attempt: u32,
    base_delay_ms: u64,
    cap_delay_ms: u64,
) -> Option<u64> {
    if !error_has_rate_limit_signal(error_str) {
        return None;
    }

    let delay_ms = provider_rate_limit_cooldown_delay_ms_for_error(
        error_str,
        retry_attempt,
        base_delay_ms,
        cap_delay_ms,
    );
    record_provider_rate_limit_cooldown_ms(provider, model, delay_ms)
}

#[cfg(test)]
pub(crate) fn clear_provider_rate_limit_cooldown(provider: &str, model: &str) {
    let Some(key) = provider_rate_limit_key(provider, model) else {
        return;
    };
    if let Ok(mut guard) = PROVIDER_RATE_LIMIT_COOLDOWNS.lock() {
        if guard.remove(&key).is_some() {
            bump_provider_runtime_state_revision();
        }
    }
}

pub(crate) struct ProviderConcurrencyPermit {
    provider: String,
    model: String,
    limit: usize,
    waited_ms: u64,
    _permit: OwnedSemaphorePermit,
}

impl ProviderConcurrencyPermit {
    pub(crate) fn provider(&self) -> &str {
        &self.provider
    }

    pub(crate) fn model(&self) -> &str {
        &self.model
    }

    pub(crate) fn limit(&self) -> usize {
        self.limit
    }

    pub(crate) fn waited_ms(&self) -> u64 {
        self.waited_ms
    }
}

impl Drop for ProviderConcurrencyPermit {
    fn drop(&mut self) {
        bump_provider_runtime_state_revision();
    }
}

pub(crate) fn provider_runtime_state_revision() -> u64 {
    prune_expired_provider_rate_limit_cooldowns();
    PROVIDER_RUNTIME_STATE_REVISION.load(Ordering::Relaxed)
}

fn bump_provider_runtime_state_revision() {
    PROVIDER_RUNTIME_STATE_REVISION.fetch_add(1, Ordering::Relaxed);
}

fn prune_expired_provider_rate_limit_cooldowns() {
    let now = Instant::now();
    let Ok(mut guard) = PROVIDER_RATE_LIMIT_COOLDOWNS.lock() else {
        return;
    };
    let before = guard.len();
    guard.retain(|_, until| *until > now);
    if guard.len() != before {
        bump_provider_runtime_state_revision();
    }
}

pub(crate) async fn acquire_provider_concurrency_permit(
    provider: &str,
    model: &str,
) -> Option<ProviderConcurrencyPermit> {
    let limit = provider_concurrency_limit();
    if limit == 0 {
        return None;
    }

    let key = provider_rate_limit_key(provider, model)?;
    let semaphore = {
        let mut guard = PROVIDER_CONCURRENCY_LIMITERS.lock().ok()?;
        Arc::clone(
            guard
                .entry(key)
                .or_insert_with(|| Arc::new(Semaphore::new(limit))),
        )
    };

    let wait_started = Instant::now();
    let permit = semaphore.acquire_owned().await.ok()?;
    bump_provider_runtime_state_revision();
    Some(ProviderConcurrencyPermit {
        provider: provider.trim().to_string(),
        model: model.trim().to_string(),
        limit,
        waited_ms: duration_to_ms_floor(wait_started.elapsed()),
        _permit: permit,
    })
}

pub(crate) fn provider_concurrency_backpressure_limit(
    provider: &str,
    model: &str,
) -> Option<usize> {
    let limit = provider_concurrency_limit();
    if limit == 0 {
        return None;
    }

    let key = provider_rate_limit_key(provider, model)?;
    let guard = PROVIDER_CONCURRENCY_LIMITERS.lock().ok()?;
    let semaphore = guard.get(&key)?;
    (semaphore.available_permits() == 0).then_some(limit)
}

fn provider_concurrency_limit() -> usize {
    std::env::var(PROVIDER_CONCURRENCY_LIMIT_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(DEFAULT_PROVIDER_CONCURRENCY_LIMIT)
}

pub(crate) fn provider_rate_limit_cooldown_cap_ms() -> u64 {
    std::env::var(PROVIDER_RATE_LIMIT_COOLDOWN_CAP_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_PROVIDER_RATE_LIMIT_COOLDOWN_CAP_MS)
}

#[cfg(test)]
pub(crate) fn clear_provider_concurrency_limiters() {
    if let Ok(mut guard) = PROVIDER_CONCURRENCY_LIMITERS.lock() {
        guard.clear();
        bump_provider_runtime_state_revision();
    }
}

fn record_provider_rate_limit_cooldown_ms(
    provider: &str,
    model: &str,
    delay_ms: u64,
) -> Option<u64> {
    if delay_ms == 0 {
        return None;
    }
    let key = provider_rate_limit_key(provider, model)?;
    let until = Instant::now() + Duration::from_millis(delay_ms);
    let mut guard = PROVIDER_RATE_LIMIT_COOLDOWNS.lock().ok()?;
    guard
        .entry(key)
        .and_modify(|existing| {
            if *existing < until {
                *existing = until;
            }
        })
        .or_insert(until);
    bump_provider_runtime_state_revision();
    Some(delay_ms)
}

fn provider_rate_limit_key(provider: &str, model: &str) -> Option<String> {
    let provider = provider.trim().to_ascii_lowercase();
    let model = model.trim().to_ascii_lowercase();
    if provider.is_empty() || model.is_empty() {
        return None;
    }
    Some(format!("{provider}::{model}"))
}

fn error_has_rate_limit_signal(error_str: &str) -> bool {
    let lower = error_str.to_ascii_lowercase();
    lower.contains("429")
        || lower.contains("too many requests")
        || lower.contains("rate limited")
        || lower.contains("rate_limit")
        || lower.contains("rate limit")
}

fn duration_to_ms_ceil(duration: Duration) -> u64 {
    let millis = duration.as_millis();
    if millis == 0 && !duration.is_zero() {
        return 1;
    }
    millis.try_into().unwrap_or(u64::MAX)
}

fn duration_to_ms_floor(duration: Duration) -> u64 {
    duration.as_millis().try_into().unwrap_or(u64::MAX)
}

pub(crate) fn retry_backoff_max_delay_ms(
    attempt: u32,
    base_delay_ms: u64,
    cap_delay_ms: u64,
) -> u64 {
    if attempt == 0 || base_delay_ms == 0 || cap_delay_ms == 0 {
        return 0;
    }

    let shift = attempt.saturating_sub(1).min(63);
    let exponential = base_delay_ms.checked_shl(shift).unwrap_or(u64::MAX);
    exponential.min(cap_delay_ms)
}

pub(crate) fn retry_backoff_delay_ms_for_nonce(
    attempt: u32,
    base_delay_ms: u64,
    cap_delay_ms: u64,
    nonce: u64,
) -> u64 {
    let max_delay_ms = retry_backoff_max_delay_ms(attempt, base_delay_ms, cap_delay_ms);
    if max_delay_ms == 0 {
        return 0;
    }

    splitmix64(nonce) % max_delay_ms.saturating_add(1)
}

fn retry_jitter_nonce() -> u64 {
    let counter = RETRY_JITTER_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or_default();
    counter ^ nanos.rotate_left(17)
}

fn retry_after_secs_from_error(error_str: &str) -> Option<u64> {
    let lower = error_str.to_ascii_lowercase();
    ["retry after", "retry-after", "retry_after"]
        .iter()
        .find_map(|marker| parse_retry_after_secs_after_marker(&lower, marker))
}

fn parse_retry_after_secs_after_marker(error_str: &str, marker: &str) -> Option<u64> {
    let (_, tail) = error_str.split_once(marker)?;
    let tail = tail
        .trim_start_matches(|ch: char| ch.is_ascii_whitespace() || matches!(ch, ':' | '=' | '('));
    let digits: String = tail.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }

    digits.parse::<u64>().ok()
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = value;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

pub(crate) fn is_transient_transport_error(error_str: &str) -> bool {
    let lower = error_str.to_ascii_lowercase();
    lower.contains("connection reset")
        || lower.contains("connection closed")
        || lower.contains("connection refused")
        || lower.contains("connection aborted")
        || lower.contains("broken pipe")
        || lower.contains("timed out")
        || lower.contains("timeout")
        || lower.contains("operation timed out")
        || lower.contains("error decoding")
        || lower.contains("error reading")
        || lower.contains("unexpected eof")
        || lower.contains("tls handshake eof")
        || lower.contains("badrecordmac")
        || lower.contains("bad_record_mac")
        || lower.contains("fatal alert: badrecordmac")
        || lower.contains("fatal alert: bad_record_mac")
        || lower.contains("received fatal alert: badrecordmac")
        || lower.contains("received fatal alert: bad_record_mac")
        || lower.contains("decryption failed or bad record mac")
        || lower.contains("temporary failure in name resolution")
        || lower.contains("failed to lookup address information")
        || lower.contains("dns error")
        || lower.contains("name or service not known")
        || lower.contains("no route to host")
        || lower.contains("network is unreachable")
        || lower.contains("host is unreachable")
}

pub(crate) fn anthropic_oauth_route_availability(model: &str) -> (bool, String) {
    if model.ends_with("[1m]") && !crate::usage::has_extra_usage() {
        (false, "requires extra usage".to_string())
    } else if model.contains("opus") && !crate::auth::claude::is_max_subscription() {
        (false, "requires Max subscription".to_string())
    } else {
        (true, String::new())
    }
}

pub(crate) fn anthropic_api_key_route_availability(model: &str) -> (bool, String) {
    if model.ends_with("[1m]") && !crate::usage::has_extra_usage() {
        (false, "requires extra usage".to_string())
    } else {
        (true, String::new())
    }
}
