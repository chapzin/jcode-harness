#[test]
fn test_fallback_sequence_includes_all_providers() {
    assert_eq!(
        MultiProvider::fallback_sequence(ActiveProvider::Claude),
        vec![
            ActiveProvider::Claude,
            ActiveProvider::OpenAI,
            ActiveProvider::Copilot,
            ActiveProvider::Gemini,
            ActiveProvider::Cursor,
            ActiveProvider::OpenRouter,
        ]
    );
    assert_eq!(
        MultiProvider::fallback_sequence(ActiveProvider::OpenAI),
        vec![
            ActiveProvider::OpenAI,
            ActiveProvider::Claude,
            ActiveProvider::Copilot,
            ActiveProvider::Gemini,
            ActiveProvider::Cursor,
            ActiveProvider::OpenRouter,
        ]
    );
    assert_eq!(
        MultiProvider::fallback_sequence(ActiveProvider::Copilot),
        vec![
            ActiveProvider::Copilot,
            ActiveProvider::Claude,
            ActiveProvider::OpenAI,
            ActiveProvider::Antigravity,
            ActiveProvider::Gemini,
            ActiveProvider::Cursor,
            ActiveProvider::OpenRouter,
        ]
    );
    assert_eq!(
        MultiProvider::fallback_sequence(ActiveProvider::Gemini),
        vec![
            ActiveProvider::Gemini,
            ActiveProvider::Claude,
            ActiveProvider::OpenAI,
            ActiveProvider::Antigravity,
            ActiveProvider::Copilot,
            ActiveProvider::Cursor,
            ActiveProvider::OpenRouter,
        ]
    );
    assert_eq!(
        MultiProvider::fallback_sequence(ActiveProvider::OpenRouter),
        vec![
            ActiveProvider::OpenRouter,
            ActiveProvider::Claude,
            ActiveProvider::OpenAI,
            ActiveProvider::Copilot,
            ActiveProvider::Antigravity,
            ActiveProvider::Gemini,
            ActiveProvider::Cursor,
        ]
    );
}

#[test]
fn provider_retry_after_parser_accepts_seconds_http_dates_and_headers() {
    use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};

    let now = chrono::DateTime::parse_from_rfc3339("2026-05-08T15:00:00Z")
        .expect("valid fixture time")
        .with_timezone(&chrono::Utc);

    assert_eq!(parse_retry_after_secs("12", now), Some(12));
    assert_eq!(
        parse_retry_after_secs("Fri, 08 May 2026 15:00:09 GMT", now),
        Some(9)
    );
    assert_eq!(
        parse_retry_after_secs("Friday, 08-May-26 14:59:59 GMT", now),
        Some(0)
    );

    let mut headers = HeaderMap::new();
    headers.insert(RETRY_AFTER, HeaderValue::from_static("31"));
    assert_eq!(retry_after_secs_from_headers(&headers, now), Some(31));
    assert_eq!(retry_after_suffix(Some(31)), " (retry after 31s)");
    assert_eq!(retry_after_suffix(None), "");
}

#[test]
fn provider_retry_backoff_caps_exponential_delay() {
    assert_eq!(retry_backoff_max_delay_ms(0, 1_000, 10_000), 0);
    assert_eq!(retry_backoff_max_delay_ms(1, 1_000, 10_000), 1_000);
    assert_eq!(retry_backoff_max_delay_ms(2, 1_000, 10_000), 2_000);
    assert_eq!(retry_backoff_max_delay_ms(3, 1_000, 10_000), 4_000);
    assert_eq!(retry_backoff_max_delay_ms(9, 1_000, 10_000), 10_000);
    assert_eq!(retry_backoff_max_delay_ms(63, u64::MAX / 2, 10_000), 10_000);
}

#[test]
fn provider_retry_backoff_full_jitter_stays_within_cap() {
    for attempt in 1..=8 {
        let max_delay = retry_backoff_max_delay_ms(attempt, 1_000, 10_000);
        for nonce in [0, 1, 2, 7, 42, u64::MAX] {
            let delay = retry_backoff_delay_ms_for_nonce(attempt, 1_000, 10_000, nonce);
            assert!(
                delay <= max_delay,
                "attempt={attempt} nonce={nonce} delay={delay} max={max_delay}"
            );
        }
    }
    assert_eq!(retry_backoff_delay_ms_for_nonce(0, 1_000, 10_000, 42), 0);
}

#[test]
fn provider_retry_delay_extracts_retry_after_hint_from_errors() {
    assert_eq!(
        retry_after_delay_ms_from_error("Rate limited (retry after 17s): slow down", 60_000),
        Some(17_000)
    );
    assert_eq!(
        retry_after_delay_ms_from_error("HTTP 429 retry-after: 2", 60_000),
        Some(2_000)
    );
    assert_eq!(
        retry_after_delay_ms_from_error("status=429 retry_after=4 seconds", 60_000),
        Some(4_000)
    );
}

#[test]
fn provider_retry_delay_caps_retry_after_and_ignores_missing_hint() {
    assert_eq!(
        retry_after_delay_ms_from_error("Rate limited (retry after 90s)", 10_000),
        Some(10_000)
    );
    assert_eq!(
        retry_after_delay_ms_from_error("transient timeout without server pacing", 10_000),
        None
    );
}

#[test]
fn provider_rate_limit_cooldown_records_and_clears_scoped_delay() {
    clear_provider_rate_limit_cooldown("OpenAI", "gpt-test");
    assert_eq!(
        provider_rate_limit_cooldown_remaining_ms("openai", "gpt-test"),
        None
    );

    assert_eq!(
        record_provider_rate_limit_cooldown_for_error(
            "OpenAI",
            "gpt-test",
            "Rate limited (retry after 2s): slow down",
            2_000,
        ),
        Some(2_000)
    );

    let remaining = provider_rate_limit_cooldown_remaining_ms("openai", "gpt-test")
        .expect("cooldown should be visible for same normalized scope");
    assert!(
        (1..=2_000).contains(&remaining),
        "unexpected cooldown remaining: {remaining}"
    );

    clear_provider_rate_limit_cooldown("openai", "gpt-test");
    assert_eq!(
        provider_rate_limit_cooldown_remaining_ms("openai", "gpt-test"),
        None
    );
}

#[test]
fn provider_rate_limit_cooldown_ignores_non_rate_limit_errors() {
    clear_provider_rate_limit_cooldown("anthropic", "claude-test");
    assert_eq!(
        record_provider_rate_limit_cooldown_for_error(
            "anthropic",
            "claude-test",
            "transient timeout without explicit throttling",
            1_000,
        ),
        None
    );
    assert_eq!(
        provider_rate_limit_cooldown_remaining_ms("anthropic", "claude-test"),
        None
    );
}

#[test]
fn provider_concurrency_backpressure_limit_one_blocks_until_release() {
    with_env_var("JCODE_PROVIDER_MAX_CONCURRENT_PER_MODEL", "1", || {
        clear_provider_concurrency_limiters();
        enter_test_runtime().block_on(async {
            let first = acquire_provider_concurrency_permit("OpenAI", "gpt-test")
                .await
                .expect("first permit should acquire immediately");
            assert_eq!(first.provider(), "OpenAI");
            assert_eq!(first.model(), "gpt-test");
            assert_eq!(first.limit(), 1);

            let blocked = tokio::time::timeout(
                std::time::Duration::from_millis(20),
                acquire_provider_concurrency_permit("openai", "gpt-test"),
            )
            .await;
            assert!(blocked.is_err(), "second scoped permit should wait");

            drop(first);
            let second = tokio::time::timeout(
                std::time::Duration::from_secs(1),
                acquire_provider_concurrency_permit("openai", "gpt-test"),
            )
            .await
            .expect("released permit should unblock")
            .expect("second permit should acquire");
            assert_eq!(second.limit(), 1);
        });
        clear_provider_concurrency_limiters();
    });
}

#[test]
fn provider_concurrency_backpressure_zero_disables_permit() {
    with_env_var("JCODE_PROVIDER_MAX_CONCURRENT_PER_MODEL", "0", || {
        clear_provider_concurrency_limiters();
        enter_test_runtime().block_on(async {
            assert!(
                acquire_provider_concurrency_permit("openai", "gpt-test")
                    .await
                    .is_none()
            );
        });
        clear_provider_concurrency_limiters();
    });
}

#[test]
fn test_parse_provider_hint_supports_known_values() {
    assert_eq!(
        MultiProvider::parse_provider_hint("claude"),
        Some(ActiveProvider::Claude)
    );
    assert_eq!(
        MultiProvider::parse_provider_hint("Anthropic"),
        Some(ActiveProvider::Claude)
    );
    assert_eq!(
        MultiProvider::parse_provider_hint("openai"),
        Some(ActiveProvider::OpenAI)
    );
    assert_eq!(
        MultiProvider::parse_provider_hint("copilot"),
        Some(ActiveProvider::Copilot)
    );
    assert_eq!(
        MultiProvider::parse_provider_hint("gemini"),
        Some(ActiveProvider::Gemini)
    );
    assert_eq!(
        MultiProvider::parse_provider_hint("openrouter"),
        Some(ActiveProvider::OpenRouter)
    );
    assert_eq!(
        MultiProvider::parse_provider_hint("cursor"),
        Some(ActiveProvider::Cursor)
    );
}

#[test]
fn test_cursor_models_are_included_in_available_models_display_when_configured() {
    with_clean_provider_test_env(|| {
        let provider = test_multi_provider_with_cursor();
        let models = provider.available_models_display();
        assert!(models.iter().any(|model| model == "composer-2-fast"));
        assert!(models.iter().any(|model| model == "composer-2"));
    });
}

#[test]
fn test_cursor_models_are_included_in_model_routes_when_configured() {
    with_clean_provider_test_env(|| {
        let provider = test_multi_provider_with_cursor();
        let routes = provider.model_routes();
        assert!(routes.iter().any(|route| {
            route.model == "composer-2-fast"
                && route.provider == "Cursor"
                && route.api_method == "cursor"
                && route.available
        }));
    });
}

#[test]
fn test_set_model_switches_to_cursor_for_cursor_models() {
    with_clean_provider_test_env(|| {
        let provider = test_multi_provider_with_cursor();
        *provider.active.write().unwrap() = ActiveProvider::Claude;

        provider
            .set_model("composer-2-fast")
            .expect("cursor model should route to Cursor");

        assert_eq!(provider.active_provider(), ActiveProvider::Cursor);
        assert_eq!(provider.model(), "composer-2-fast");
    });
}

#[test]
fn test_set_model_supports_explicit_cursor_prefix() {
    with_clean_provider_test_env(|| {
        let provider = test_multi_provider_with_cursor();
        *provider.active.write().unwrap() = ActiveProvider::OpenAI;

        provider
            .set_model("cursor:gpt-5")
            .expect("explicit cursor prefix should force Cursor route");

        assert_eq!(provider.active_provider(), ActiveProvider::Cursor);
        assert_eq!(provider.model(), "gpt-5");
    });
}

#[test]
fn test_forced_provider_disables_cross_provider_fallback_sequence() {
    assert_eq!(
        MultiProvider::fallback_sequence_for(ActiveProvider::Claude, Some(ActiveProvider::OpenAI)),
        vec![ActiveProvider::OpenAI]
    );
    assert_eq!(
        MultiProvider::fallback_sequence_for(ActiveProvider::OpenAI, Some(ActiveProvider::OpenAI)),
        vec![ActiveProvider::OpenAI]
    );
    assert_eq!(
        MultiProvider::fallback_sequence_for(ActiveProvider::Claude, None),
        MultiProvider::fallback_sequence(ActiveProvider::Claude)
    );
}

#[test]
fn test_set_model_rejects_cross_provider_without_creds() {
    let _guard = crate::storage::lock_test_env();
    crate::subscription_catalog::clear_runtime_env();
    crate::env::remove_var("JCODE_ACTIVE_PROVIDER");
    crate::env::remove_var("JCODE_FORCE_PROVIDER");

    let provider = MultiProvider {
        claude: RwLock::new(None),
        anthropic: RwLock::new(None),
        openai: RwLock::new(None),
        copilot_api: RwLock::new(None),
        antigravity: RwLock::new(None),
        gemini: RwLock::new(None),
        cursor: RwLock::new(None),
        bedrock: RwLock::new(None),
        openrouter: RwLock::new(None),
        active: RwLock::new(ActiveProvider::OpenAI),
        use_claude_cli: false,
        startup_notices: RwLock::new(Vec::new()),
        forced_provider: Some(ActiveProvider::OpenAI),
    };

    let err = provider
        .set_model("claude-sonnet-4-6")
        .expect_err("forced provider should reject when the forced provider has no creds");
    assert!(
        err.to_string().contains("OpenAI credentials not available"),
        "expected credentials error, got: {}",
        err
    );
}

#[test]
fn test_auto_default_prefers_openai_over_claude_when_both_available() {
    let active = MultiProvider::auto_default_provider(ProviderAvailability {
        openai: true,
        claude: true,
        copilot: false,
        antigravity: false,
        gemini: false,
        cursor: false,
        bedrock: false,
        openrouter: false,
        copilot_premium_zero: false,
    });
    assert_eq!(active, ActiveProvider::OpenAI);
}

#[test]
fn test_auto_default_prefers_copilot_when_zero_premium_mode_enabled() {
    let active = MultiProvider::auto_default_provider(ProviderAvailability {
        openai: true,
        claude: true,
        copilot: true,
        antigravity: true,
        gemini: true,
        cursor: true,
        bedrock: false,
        openrouter: true,
        copilot_premium_zero: true,
    });
    assert_eq!(active, ActiveProvider::Copilot);
}

#[test]
fn test_should_failover_on_403_forbidden() {
    let err = anyhow::anyhow!(
        "Copilot token exchange failed (HTTP 403 Forbidden): not accessible by integration"
    );
    assert!(MultiProvider::classify_failover_error(&err).should_failover());
}

#[test]
fn test_should_failover_on_token_exchange_failed() {
    let msg = r#"Copilot token exchange failed (HTTP 403 Forbidden): {"error_details":{"title":"Contact Support"}}"#;
    let err = anyhow::anyhow!("{}", msg);
    assert!(MultiProvider::classify_failover_error(&err).should_failover());
}

#[test]
fn test_should_failover_on_access_denied() {
    let err = anyhow::anyhow!("Access denied: account suspended");
    assert!(MultiProvider::classify_failover_error(&err).should_failover());
}

#[test]
fn test_should_failover_when_status_code_starts_message() {
    let err = anyhow::anyhow!("401 unauthorized");
    assert!(MultiProvider::classify_failover_error(&err).should_failover());
    assert_eq!(
        MultiProvider::classify_failover_error(&err),
        FailoverDecision::RetryAndMarkUnavailable
    );
}

#[test]
fn test_should_not_failover_on_non_independent_status_digits() {
    let err = anyhow::anyhow!("backend returned code 14290");
    assert!(!MultiProvider::classify_failover_error(&err).should_failover());
}

#[test]
fn test_context_limit_error_fails_over_without_marking_provider_unavailable() {
    let err = anyhow::anyhow!("Context length exceeded maximum context window");
    assert!(MultiProvider::classify_failover_error(&err).should_failover());
    assert_eq!(
        MultiProvider::classify_failover_error(&err),
        FailoverDecision::RetryNextProvider
    );
}

#[test]
fn test_should_not_failover_on_generic_error() {
    let err = anyhow::anyhow!("Connection timed out");
    assert!(!MultiProvider::classify_failover_error(&err).should_failover());
}

#[test]
fn test_no_provider_error_mentions_tokens_and_details() {
    let provider = MultiProvider {
        claude: RwLock::new(None),
        anthropic: RwLock::new(None),
        openai: RwLock::new(None),
        copilot_api: RwLock::new(None),
        antigravity: RwLock::new(None),
        gemini: RwLock::new(None),
        cursor: RwLock::new(None),
        bedrock: RwLock::new(None),
        openrouter: RwLock::new(None),
        active: RwLock::new(ActiveProvider::OpenAI),
        use_claude_cli: false,
        startup_notices: RwLock::new(Vec::new()),
        forced_provider: None,
    };
    let err = provider.no_provider_available_error(&[
        "OpenAI: rate limited".to_string(),
        "GitHub Copilot: not configured".to_string(),
    ]);
    let text = err.to_string();
    assert!(text.contains("No tokens/providers left"));
    assert!(text.contains("OpenAI: rate limited"));
    assert!(text.contains("GitHub Copilot: not configured"));
}
