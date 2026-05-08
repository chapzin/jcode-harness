use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tokio::sync::broadcast;

pub const HARNESS_EVENT_SCHEMA_VERSION: u16 = 1;
pub const HARNESS_EVENT_REDACTED: &str = "[redacted]";
pub const HARNESS_EVENT_TRUNCATED: &str = "...[truncated]";
pub const DEFAULT_MAX_PAYLOAD_STRING_CHARS: usize = 4096;
const DEFAULT_EVENT_BUS_CAPACITY: usize = 1024;
const HARNESS_EVENT_LOG_DIR: &str = "harness-events";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessEventLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessEventKind {
    RunStarted,
    RunCompleted,
    RunFailed,
    SkillSelected,
    MemoryLookupStarted,
    MemoryLookupFinished,
    ToolStarted,
    ToolFinished,
    FileChanged,
    TestStarted,
    TestPassed,
    TestFailed,
    GatePassed,
    GateFailed,
    HumanApprovalRequired,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessEventPayloadClass {
    SafeMetadata,
    SensitiveMetadata,
    Secret,
    UserContent,
    ArtifactReference,
}

impl HarnessEventPayloadClass {
    pub fn redacts_whole_payload(self) -> bool {
        matches!(self, Self::Secret | Self::UserContent)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct HarnessEvent {
    pub schema_version: u16,
    pub event_id: String,
    pub run_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_event_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub sequence: u64,
    pub level: HarnessEventLevel,
    pub kind: HarnessEventKind,
    pub payload_class: HarnessEventPayloadClass,
    pub payload: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HarnessEventLogSummary {
    pub run_id: String,
    pub path: String,
    pub events: usize,
    pub status: String,
    pub first_timestamp: Option<DateTime<Utc>>,
    pub last_timestamp: Option<DateTime<Utc>>,
    pub duration_ms: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl HarnessEvent {
    pub fn new(
        event_id: impl Into<String>,
        run_id: impl Into<String>,
        timestamp: DateTime<Utc>,
        sequence: u64,
        level: HarnessEventLevel,
        kind: HarnessEventKind,
        payload: Value,
    ) -> Self {
        Self {
            schema_version: HARNESS_EVENT_SCHEMA_VERSION,
            event_id: event_id.into(),
            run_id: run_id.into(),
            session_id: None,
            parent_event_id: None,
            timestamp,
            sequence,
            level,
            kind,
            payload_class: HarnessEventPayloadClass::SafeMetadata,
            payload: redact_harness_event_payload(payload, HarnessEventPayloadClass::SafeMetadata),
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_parent_event_id(mut self, parent_event_id: impl Into<String>) -> Self {
        self.parent_event_id = Some(parent_event_id.into());
        self
    }

    pub fn with_payload_class(mut self, payload_class: HarnessEventPayloadClass) -> Self {
        self.payload_class = payload_class;
        self.payload = redact_harness_event_payload(self.payload, payload_class);
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HarnessEventDraft {
    run_id: String,
    session_id: Option<String>,
    parent_event_id: Option<String>,
    level: HarnessEventLevel,
    kind: HarnessEventKind,
    payload_class: HarnessEventPayloadClass,
    payload: Value,
}

impl HarnessEventDraft {
    pub fn new(run_id: impl Into<String>, kind: HarnessEventKind) -> Self {
        Self {
            run_id: run_id.into(),
            session_id: None,
            parent_event_id: None,
            level: HarnessEventLevel::Info,
            kind,
            payload_class: HarnessEventPayloadClass::SafeMetadata,
            payload: json!({}),
        }
    }

    pub fn run_started(run_id: impl Into<String>) -> Self {
        Self::new(run_id, HarnessEventKind::RunStarted)
    }

    pub fn run_completed(run_id: impl Into<String>) -> Self {
        Self::new(run_id, HarnessEventKind::RunCompleted)
    }

    pub fn run_failed(run_id: impl Into<String>) -> Self {
        Self::new(run_id, HarnessEventKind::RunFailed).with_level(HarnessEventLevel::Error)
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_parent_event_id(mut self, parent_event_id: impl Into<String>) -> Self {
        self.parent_event_id = Some(parent_event_id.into());
        self
    }

    pub fn with_level(mut self, level: HarnessEventLevel) -> Self {
        self.level = level;
        self
    }

    pub fn with_payload(mut self, payload: Value) -> Self {
        self.payload = payload;
        self
    }

    pub fn with_payload_class(mut self, payload_class: HarnessEventPayloadClass) -> Self {
        self.payload_class = payload_class;
        self
    }
}

pub fn redact_harness_event_payload(
    payload: Value,
    payload_class: HarnessEventPayloadClass,
) -> Value {
    if payload_class.redacts_whole_payload() {
        return json!({
            "redacted": true,
            "payload_class": payload_class,
        });
    }

    redact_value(payload)
}

fn redact_value(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    if is_sensitive_payload_key(&key) {
                        (key, Value::String(HARNESS_EVENT_REDACTED.to_string()))
                    } else {
                        (key, redact_value(value))
                    }
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(redact_value).collect()),
        Value::String(value) if looks_like_secret_value(&value) => {
            Value::String(HARNESS_EVENT_REDACTED.to_string())
        }
        Value::String(value) => Value::String(truncate_payload_string(&value)),
        other => other,
    }
}

fn is_sensitive_payload_key(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();

    if is_safe_token_metric_key(&normalized) {
        return false;
    }

    matches!(
        normalized.as_str(),
        "apikey"
            | "authorization"
            | "authtoken"
            | "cookie"
            | "credential"
            | "credentials"
            | "filecontent"
            | "input"
            | "password"
            | "privatekey"
            | "prompt"
            | "rawprompt"
            | "refreshtoken"
            | "secret"
            | "setcookie"
            | "stderr"
            | "stdout"
            | "token"
            | "tooloutput"
            | "usercontent"
    ) || normalized.contains("apikey")
        || normalized.contains("privatekey")
        || normalized.contains("secret")
        || normalized.contains("token")
}

fn is_safe_token_metric_key(normalized_key: &str) -> bool {
    matches!(
        normalized_key,
        "cachecreationinputtokens"
            | "cachereadinputtokens"
            | "completiontokens"
            | "inputtokens"
            | "outputtokens"
            | "prompttokens"
            | "tokencount"
            | "tokensused"
            | "totaltokens"
    )
}

fn looks_like_secret_value(value: &str) -> bool {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    lower.starts_with("bearer ")
        || trimmed.starts_with("ghp_")
        || trimmed.starts_with("gho_")
        || trimmed.starts_with("github_pat_")
        || trimmed.starts_with("sk-")
        || trimmed.contains("-----BEGIN ")
}

fn truncate_payload_string(value: &str) -> String {
    let mut chars = value.chars();
    let truncated: String = chars
        .by_ref()
        .take(DEFAULT_MAX_PAYLOAD_STRING_CHARS)
        .collect();
    if chars.next().is_none() {
        value.to_string()
    } else {
        format!("{truncated}{HARNESS_EVENT_TRUNCATED}")
    }
}

pub fn default_harness_event_log_dir() -> PathBuf {
    crate::storage::runtime_dir().join(HARNESS_EVENT_LOG_DIR)
}

pub fn harness_event_log_path(run_id: &str) -> PathBuf {
    default_harness_event_log_dir().join(format!("{}.ndjson", sanitize_run_id(run_id)))
}

pub fn write_harness_event_ndjson<W: Write>(
    writer: &mut W,
    event: &HarnessEvent,
) -> anyhow::Result<()> {
    serde_json::to_writer(&mut *writer, event)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

pub fn append_harness_event_ndjson(
    path: impl AsRef<Path>,
    event: &HarnessEvent,
) -> anyhow::Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    write_harness_event_ndjson(&mut file, event)
}

pub fn read_harness_event_ndjson(path: impl AsRef<Path>) -> anyhow::Result<Vec<HarnessEvent>> {
    let path = path.as_ref();
    let file = std::fs::File::open(path).map_err(|err| {
        anyhow::anyhow!(
            "failed to open harness event log {}: {}",
            path.display(),
            err
        )
    })?;
    let reader = BufReader::new(file);
    let mut events = Vec::new();

    for (line_index, line) in reader.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line.map_err(|err| {
            anyhow::anyhow!(
                "failed to read harness event log {} line {}: {}",
                path.display(),
                line_number,
                err
            )
        })?;
        if line.trim().is_empty() {
            anyhow::bail!(
                "invalid harness event NDJSON at {} line {}: blank line",
                path.display(),
                line_number
            );
        }
        let event = serde_json::from_str::<HarnessEvent>(&line).map_err(|err| {
            anyhow::anyhow!(
                "invalid harness event NDJSON at {} line {}: {}",
                path.display(),
                line_number,
                err
            )
        })?;
        events.push(event);
    }

    Ok(events)
}

pub fn summarize_harness_event_log(
    path: impl AsRef<Path>,
) -> anyhow::Result<HarnessEventLogSummary> {
    let path = path.as_ref();
    let events = read_harness_event_ndjson(path)?;
    Ok(summarize_harness_events(path, &events))
}

pub fn summarize_harness_events(
    path: impl AsRef<Path>,
    events: &[HarnessEvent],
) -> HarnessEventLogSummary {
    let path = path.as_ref();
    let first_timestamp = events.iter().map(|event| event.timestamp).min();
    let last_timestamp = events.iter().map(|event| event.timestamp).max();
    let run_id = events
        .first()
        .map(|event| event.run_id.clone())
        .unwrap_or_else(|| run_id_from_event_log_path(path));
    let status = infer_harness_event_status(events).to_string();
    let duration_ms = first_timestamp
        .zip(last_timestamp)
        .and_then(|(first, last)| (last - first).num_milliseconds().try_into().ok());

    HarnessEventLogSummary {
        run_id,
        path: path.display().to_string(),
        events: events.len(),
        status,
        first_timestamp,
        last_timestamp,
        duration_ms,
        error: None,
    }
}

pub fn list_harness_event_logs() -> anyhow::Result<Vec<HarnessEventLogSummary>> {
    let dir = default_harness_event_log_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut summaries = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("ndjson") {
            continue;
        }
        match summarize_harness_event_log(&path) {
            Ok(summary) => summaries.push(summary),
            Err(err) => summaries.push(HarnessEventLogSummary {
                run_id: run_id_from_event_log_path(&path),
                path: path.display().to_string(),
                events: 0,
                status: "corrupt".to_string(),
                first_timestamp: None,
                last_timestamp: None,
                duration_ms: None,
                error: Some(err.to_string()),
            }),
        }
    }

    summaries.sort_by(|a, b| {
        b.last_timestamp
            .cmp(&a.last_timestamp)
            .then_with(|| a.run_id.cmp(&b.run_id))
    });
    Ok(summaries)
}

pub fn render_harness_event_replay_markdown(events: &[HarnessEvent]) -> String {
    let summary = summarize_harness_events("<memory>", events);
    let mut output = String::new();
    output.push_str(&format!("# Harness event replay: {}\n\n", summary.run_id));
    output.push_str("## Summary\n\n");
    output.push_str(&format!("- Status: `{}`\n", summary.status));
    output.push_str(&format!("- Events: {}\n", summary.events));
    if let Some(first) = summary.first_timestamp {
        output.push_str(&format!("- Started: `{}`\n", first));
    }
    if let Some(last) = summary.last_timestamp {
        output.push_str(&format!("- Last event: `{}`\n", last));
    }
    if let Some(duration_ms) = summary.duration_ms {
        output.push_str(&format!("- Duration: {} ms\n", duration_ms));
    }
    output.push_str("\n## Timeline\n\n");
    output.push_str("| Seq | Time | Level | Kind | Event | Details |\n");
    output.push_str("| ---: | --- | --- | --- | --- | --- |\n");
    for event in events {
        output.push_str(&format!(
            "| {} | `{}` | `{}` | `{}` | `{}` | {} |\n",
            event.sequence,
            event.timestamp,
            event_label(&event.level),
            event_label(&event.kind),
            escape_markdown_table_cell(&event.event_id),
            escape_markdown_table_cell(&event_payload_summary(&event.payload)),
        ));
    }
    output
}

fn infer_harness_event_status(events: &[HarnessEvent]) -> &'static str {
    if events.iter().any(|event| {
        matches!(
            event.kind,
            HarnessEventKind::RunFailed
                | HarnessEventKind::TestFailed
                | HarnessEventKind::GateFailed
        )
    }) {
        "failed"
    } else if events
        .iter()
        .any(|event| matches!(event.kind, HarnessEventKind::RunCompleted))
    {
        "completed"
    } else if events.is_empty() {
        "empty"
    } else {
        "partial"
    }
}

fn run_id_from_event_log_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("run")
        .to_string()
}

fn event_label<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

fn event_payload_summary(payload: &Value) -> String {
    match payload {
        Value::Object(map) => {
            let mut parts = Vec::new();
            for key in ["status", "tool", "source", "duration_ms", "text_chars"] {
                if let Some(value) = map.get(key) {
                    parts.push(format!("{key}={}", short_json_value(value)));
                }
            }
            if parts.is_empty() {
                "".to_string()
            } else {
                parts.join(", ")
            }
        }
        _ => short_json_value(payload),
    }
}

fn short_json_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        other => other.to_string(),
    }
}

fn escape_markdown_table_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn sanitize_run_id(run_id: &str) -> String {
    let sanitized: String = run_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .take(120)
        .collect();
    if sanitized.is_empty() {
        "run".to_string()
    } else {
        sanitized
    }
}

pub struct HarnessEventBus {
    sender: broadcast::Sender<HarnessEvent>,
    sequences: Mutex<HashMap<String, u64>>,
}

impl HarnessEventBus {
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<HarnessEventBus> = OnceLock::new();
        INSTANCE.get_or_init(|| Self::with_capacity(DEFAULT_EVENT_BUS_CAPACITY))
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity.max(1));
        Self {
            sender,
            sequences: Mutex::new(HashMap::new()),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<HarnessEvent> {
        self.sender.subscribe()
    }

    pub fn publish(&self, draft: HarnessEventDraft) -> HarnessEvent {
        let sequence = self.next_sequence(&draft.run_id);
        let event = HarnessEvent {
            schema_version: HARNESS_EVENT_SCHEMA_VERSION,
            event_id: crate::id::new_id("hevt"),
            run_id: draft.run_id,
            session_id: draft.session_id,
            parent_event_id: draft.parent_event_id,
            timestamp: Utc::now(),
            sequence,
            level: draft.level,
            kind: draft.kind,
            payload_class: draft.payload_class,
            payload: redact_harness_event_payload(draft.payload, draft.payload_class),
        };
        let _ = self.sender.send(event.clone());
        event
    }

    pub fn publish_run_started(&self, run_id: impl Into<String>) -> HarnessEvent {
        self.publish(HarnessEventDraft::run_started(run_id))
    }

    pub fn publish_run_completed(&self, run_id: impl Into<String>) -> HarnessEvent {
        self.publish(HarnessEventDraft::run_completed(run_id))
    }

    fn next_sequence(&self, run_id: &str) -> u64 {
        let mut sequences = self
            .sequences
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let sequence = sequences.entry(run_id.to_string()).or_insert(0);
        *sequence += 1;
        *sequence
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{Duration, timeout};

    struct EnvVarRestore {
        key: &'static str,
        value: Option<std::ffi::OsString>,
    }

    impl EnvVarRestore {
        fn set_runtime_dir(path: &Path) -> Self {
            let key = "JCODE_RUNTIME_DIR";
            let value = std::env::var_os(key);
            crate::env::set_var(key, path);
            Self { key, value }
        }
    }

    impl Drop for EnvVarRestore {
        fn drop(&mut self) {
            if let Some(value) = self.value.as_ref() {
                crate::env::set_var(self.key, value);
            } else {
                crate::env::remove_var(self.key);
            }
        }
    }

    #[test]
    fn event_serializes_stable_common_fields() {
        let timestamp = DateTime::parse_from_rfc3339("2026-05-08T03:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let event = HarnessEvent::new(
            "hevt_test",
            "run_test",
            timestamp,
            7,
            HarnessEventLevel::Info,
            HarnessEventKind::ToolFinished,
            json!({"tool": "cargo test", "status": "passed"}),
        )
        .with_session_id("session_test")
        .with_parent_event_id("hevt_parent");

        let value = serde_json::to_value(&event).unwrap();
        assert_eq!(value["schema_version"], HARNESS_EVENT_SCHEMA_VERSION);
        assert_eq!(value["event_id"], "hevt_test");
        assert_eq!(value["run_id"], "run_test");
        assert_eq!(value["session_id"], "session_test");
        assert_eq!(value["parent_event_id"], "hevt_parent");
        assert_eq!(value["timestamp"], "2026-05-08T03:00:00Z");
        assert_eq!(value["sequence"], 7);
        assert_eq!(value["level"], "info");
        assert_eq!(value["kind"], "tool_finished");
        assert_eq!(value["payload_class"], "safe_metadata");
        assert_eq!(value["payload"]["tool"], "cargo test");
        assert_eq!(value["payload"]["status"], "passed");
    }

    #[tokio::test]
    async fn event_bus_redacts_sensitive_keys_by_default() {
        let bus = HarnessEventBus::with_capacity(8);

        let event = bus.publish(
            HarnessEventDraft::new("run_redact", HarnessEventKind::ToolStarted).with_payload(
                json!({
                    "tool": "deploy",
                    "api_key": "sk-live-secret",
                    "nested": {
                        "Authorization": "Bearer should-not-leak",
                        "safe": "metadata"
                    },
                    "items": [{"password": "hunter2"}]
                }),
            ),
        );

        let serialized = serde_json::to_string(&event).unwrap();
        assert!(!serialized.contains("sk-live-secret"));
        assert!(!serialized.contains("Bearer should-not-leak"));
        assert!(!serialized.contains("hunter2"));
        assert_eq!(event.payload["tool"], "deploy");
        assert_eq!(event.payload["api_key"], HARNESS_EVENT_REDACTED);
        assert_eq!(
            event.payload["nested"]["Authorization"],
            HARNESS_EVENT_REDACTED
        );
        assert_eq!(event.payload["nested"]["safe"], "metadata");
        assert_eq!(
            event.payload["items"][0]["password"],
            HARNESS_EVENT_REDACTED
        );
    }

    #[tokio::test]
    async fn token_usage_metrics_are_not_redacted_but_auth_tokens_are() {
        let bus = HarnessEventBus::with_capacity(8);

        let event = bus.publish(
            HarnessEventDraft::new("run_token_metrics", HarnessEventKind::RunCompleted)
                .with_payload(json!({
                    "input_tokens": 10,
                    "output_tokens": 4,
                    "cache_read_input_tokens": 2,
                    "auth_token": "ghp_should_still_be_redacted",
                })),
        );

        assert_eq!(event.payload["input_tokens"], 10);
        assert_eq!(event.payload["output_tokens"], 4);
        assert_eq!(event.payload["cache_read_input_tokens"], 2);
        assert_eq!(event.payload["auth_token"], HARNESS_EVENT_REDACTED);
    }

    #[tokio::test]
    async fn user_content_payload_is_redacted_wholesale() {
        let bus = HarnessEventBus::with_capacity(8);

        let event = bus.publish(
            HarnessEventDraft::new("run_user_content", HarnessEventKind::HumanApprovalRequired)
                .with_payload_class(HarnessEventPayloadClass::UserContent)
                .with_payload(json!({
                    "prompt": "please do a private thing",
                    "file_content": "secret source text"
                })),
        );

        let serialized = serde_json::to_string(&event).unwrap();
        assert!(!serialized.contains("private thing"));
        assert!(!serialized.contains("secret source text"));
        assert_eq!(event.payload_class, HarnessEventPayloadClass::UserContent);
        assert_eq!(event.payload["redacted"], true);
        assert_eq!(event.payload["payload_class"], "user_content");
    }

    #[test]
    fn long_payload_strings_are_truncated() {
        let long = "x".repeat(DEFAULT_MAX_PAYLOAD_STRING_CHARS + 8);

        let redacted = redact_harness_event_payload(
            json!({"summary": long}),
            HarnessEventPayloadClass::SafeMetadata,
        );
        let summary = redacted["summary"].as_str().unwrap();

        assert!(summary.ends_with(HARNESS_EVENT_TRUNCATED));
        assert_eq!(
            summary.chars().count(),
            DEFAULT_MAX_PAYLOAD_STRING_CHARS + HARNESS_EVENT_TRUNCATED.chars().count()
        );
    }

    #[test]
    fn direct_event_constructor_redacts_secret_values() {
        let timestamp = Utc::now();
        let event = HarnessEvent::new(
            "hevt_secret",
            "run_secret",
            timestamp,
            1,
            HarnessEventLevel::Info,
            HarnessEventKind::GatePassed,
            json!({"token": "ghp_should_not_escape", "status": "ok"}),
        );

        let serialized = serde_json::to_string(&event).unwrap();
        assert!(!serialized.contains("ghp_should_not_escape"));
        assert_eq!(event.payload["token"], HARNESS_EVENT_REDACTED);
        assert_eq!(event.payload["status"], "ok");
    }

    #[test]
    fn ndjson_writer_emits_one_parseable_line() {
        let timestamp = DateTime::parse_from_rfc3339("2026-05-08T03:10:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let event = HarnessEvent::new(
            "hevt_ndjson",
            "run_ndjson",
            timestamp,
            1,
            HarnessEventLevel::Info,
            HarnessEventKind::RunStarted,
            json!({"status": "ok"}),
        );
        let mut output = Vec::new();

        write_harness_event_ndjson(&mut output, &event).unwrap();

        let text = String::from_utf8(output).unwrap();
        assert!(text.ends_with('\n'));
        assert_eq!(text.lines().count(), 1);
        let parsed: HarnessEvent = serde_json::from_str(text.trim_end()).unwrap();
        assert_eq!(parsed.event_id, "hevt_ndjson");
        assert_eq!(parsed.payload["status"], "ok");
    }

    #[test]
    fn ndjson_append_creates_file_and_preserves_redaction() {
        let temp = tempfile::Builder::new()
            .prefix("jcode-harness-events-ndjson-")
            .tempdir()
            .unwrap();
        let path = temp.path().join("events").join("run.ndjson");
        let bus = HarnessEventBus::with_capacity(8);

        let first = bus.publish(
            HarnessEventDraft::new("run_file", HarnessEventKind::ToolStarted).with_payload(json!({
                "tool": "bash",
                "token": "ghp_never_write_me"
            })),
        );
        let second = bus.publish(HarnessEventDraft::run_completed("run_file"));

        append_harness_event_ndjson(&path, &first).unwrap();
        append_harness_event_ndjson(&path, &second).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("ghp_never_write_me"));
        let lines = text.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        let first_parsed: HarnessEvent = serde_json::from_str(lines[0]).unwrap();
        let second_parsed: HarnessEvent = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(first_parsed.payload["token"], HARNESS_EVENT_REDACTED);
        assert_eq!(first_parsed.sequence, 1);
        assert_eq!(second_parsed.sequence, 2);
    }

    #[test]
    fn ndjson_reader_round_trips_multiple_events() {
        let temp = tempfile::Builder::new()
            .prefix("jcode-harness-events-read-")
            .tempdir()
            .unwrap();
        let path = temp.path().join("run.ndjson");
        let bus = HarnessEventBus::with_capacity(8);
        let first = bus.publish(HarnessEventDraft::run_started("run_read"));
        let second = bus.publish(HarnessEventDraft::run_completed("run_read"));

        append_harness_event_ndjson(&path, &first).unwrap();
        append_harness_event_ndjson(&path, &second).unwrap();

        let events = read_harness_event_ndjson(&path).unwrap();
        assert_eq!(events, vec![first, second]);
    }

    #[test]
    fn ndjson_reader_reports_corrupt_line_number() {
        let temp = tempfile::Builder::new()
            .prefix("jcode-harness-events-corrupt-")
            .tempdir()
            .unwrap();
        let path = temp.path().join("run.ndjson");
        let bus = HarnessEventBus::with_capacity(8);

        append_harness_event_ndjson(
            &path,
            &bus.publish(HarnessEventDraft::run_started("run_bad")),
        )
        .unwrap();
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"not-json\n")
            .unwrap();

        let err = read_harness_event_ndjson(&path).unwrap_err().to_string();
        assert!(err.contains("line 2"), "unexpected error: {err}");
        assert!(
            err.contains("invalid harness event NDJSON"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn event_log_path_sanitizes_run_id() {
        let path = harness_event_log_path("run/with spaces/and:*chars");
        let file_name = path.file_name().and_then(|name| name.to_str()).unwrap();

        assert_eq!(file_name, "run_with_spaces_and__chars.ndjson");
        assert!(path.ends_with("harness-events/run_with_spaces_and__chars.ndjson"));
    }

    #[test]
    fn event_log_summary_detects_completed_status_and_duration() {
        let temp = tempfile::Builder::new()
            .prefix("jcode-harness-events-summary-")
            .tempdir()
            .unwrap();
        let path = temp.path().join("run_summary.ndjson");
        let started = HarnessEvent::new(
            "hevt_start",
            "run_summary",
            DateTime::parse_from_rfc3339("2026-05-08T03:40:00Z")
                .unwrap()
                .with_timezone(&Utc),
            1,
            HarnessEventLevel::Info,
            HarnessEventKind::RunStarted,
            json!({"status": "started"}),
        );
        let completed = HarnessEvent::new(
            "hevt_done",
            "run_summary",
            DateTime::parse_from_rfc3339("2026-05-08T03:40:02Z")
                .unwrap()
                .with_timezone(&Utc),
            2,
            HarnessEventLevel::Info,
            HarnessEventKind::RunCompleted,
            json!({"status": "ok", "duration_ms": 2000}),
        );

        append_harness_event_ndjson(&path, &started).unwrap();
        append_harness_event_ndjson(&path, &completed).unwrap();

        let summary = summarize_harness_event_log(&path).unwrap();
        assert_eq!(summary.run_id, "run_summary");
        assert_eq!(summary.events, 2);
        assert_eq!(summary.status, "completed");
        assert_eq!(summary.duration_ms, Some(2000));
    }

    #[test]
    fn list_event_logs_includes_corrupt_diagnostics() {
        let _lock = crate::storage::lock_test_env();
        let temp = tempfile::Builder::new()
            .prefix("jcode-harness-events-list-")
            .tempdir()
            .unwrap();
        let _env = EnvVarRestore::set_runtime_dir(temp.path());
        std::fs::create_dir_all(default_harness_event_log_dir()).unwrap();
        let bus = HarnessEventBus::with_capacity(8);
        append_harness_event_ndjson(
            harness_event_log_path("run_good"),
            &bus.publish(HarnessEventDraft::run_completed("run_good")),
        )
        .unwrap();
        std::fs::write(
            default_harness_event_log_dir().join("run_bad.ndjson"),
            "bad\n",
        )
        .unwrap();

        let summaries = list_harness_event_logs().unwrap();
        assert!(summaries.iter().any(|summary| summary.run_id == "run_good"));
        let corrupt = summaries
            .iter()
            .find(|summary| summary.run_id == "run_bad")
            .expect("corrupt summary");
        assert_eq!(corrupt.status, "corrupt");
        assert!(
            corrupt
                .error
                .as_deref()
                .unwrap_or_default()
                .contains("line 1")
        );
    }

    #[test]
    fn replay_markdown_contains_summary_and_timeline() {
        let event = HarnessEvent::new(
            "hevt_tool",
            "run_replay",
            DateTime::parse_from_rfc3339("2026-05-08T03:41:00Z")
                .unwrap()
                .with_timezone(&Utc),
            1,
            HarnessEventLevel::Info,
            HarnessEventKind::ToolFinished,
            json!({"tool": "cargo test", "status": "ok"}),
        );

        let markdown = render_harness_event_replay_markdown(&[event]);

        assert!(markdown.contains("# Harness event replay: run_replay"));
        assert!(markdown.contains("## Summary"));
        assert!(markdown.contains("| Seq | Time | Level | Kind | Event | Details |"));
        assert!(markdown.contains("tool=cargo test"));
    }

    #[tokio::test]
    async fn event_bus_assigns_monotonic_sequences_per_run() {
        let bus = HarnessEventBus::with_capacity(8);
        let mut rx = bus.subscribe();

        let first = bus.publish(HarnessEventDraft::run_started("run_a"));
        let second = bus.publish(HarnessEventDraft::run_completed("run_a"));
        let other_run = bus.publish(HarnessEventDraft::run_started("run_b"));

        assert_eq!(first.sequence, 1);
        assert_eq!(second.sequence, 2);
        assert_eq!(other_run.sequence, 1);

        assert_eq!(rx.recv().await.unwrap().sequence, 1);
        assert_eq!(rx.recv().await.unwrap().sequence, 2);
        assert_eq!(rx.recv().await.unwrap().run_id, "run_b");
    }

    #[tokio::test]
    async fn event_bus_fans_out_to_multiple_subscribers() {
        let bus = HarnessEventBus::with_capacity(8);
        let mut first_rx = bus.subscribe();
        let mut second_rx = bus.subscribe();

        let event = bus.publish(
            HarnessEventDraft::new("run_fanout", HarnessEventKind::HumanApprovalRequired)
                .with_payload(json!({"reason": "approval required"})),
        );

        let first_seen = timeout(Duration::from_secs(1), first_rx.recv())
            .await
            .unwrap()
            .unwrap();
        let second_seen = timeout(Duration::from_secs(1), second_rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(first_seen.event_id, event.event_id);
        assert_eq!(second_seen.event_id, event.event_id);
        assert_eq!(first_seen.kind, HarnessEventKind::HumanApprovalRequired);
        assert_eq!(second_seen.payload["reason"], "approval required");
    }

    #[test]
    fn publishing_without_subscribers_still_returns_event() {
        let bus = HarnessEventBus::with_capacity(8);

        let event = bus.publish(HarnessEventDraft::run_started("run_no_subscribers"));

        assert_eq!(event.sequence, 1);
        assert_eq!(event.kind, HarnessEventKind::RunStarted);
        assert!(event.event_id.starts_with("hevt_"));
    }
}
