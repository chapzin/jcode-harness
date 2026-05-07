use crate::test_support::*;
use anyhow::Context;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Stdio;

const ENABLE_ENV: &str = "JCODE_HARNESS_LIVE_PROVIDER_SMOKE";
const PROVIDER_ENV: &str = "JCODE_HARNESS_LIVE_PROVIDER";
const PROVIDER_PROFILE_ENV: &str = "JCODE_HARNESS_LIVE_PROVIDER_PROFILE";
const PROVIDER_CONFIG_ENV: &str = "JCODE_HARNESS_LIVE_PROVIDER_CONFIG";
const MODEL_ENV: &str = "JCODE_HARNESS_LIVE_MODEL";
const PROMPT_ENV: &str = "JCODE_HARNESS_LIVE_PROMPT";

fn truthy_env(name: &str) -> bool {
    std::env::var(name)
        .map(|value| {
            let value = value.trim();
            !value.is_empty()
                && value != "0"
                && !value.eq_ignore_ascii_case("false")
                && !value.eq_ignore_ascii_case("no")
        })
        .unwrap_or(false)
}

fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[test]
fn harness_run_live_provider_smoke_is_explicit_opt_in_and_isolated() -> Result<()> {
    if !truthy_env(ENABLE_ENV) {
        eprintln!(
            "skipping live provider smoke; set {ENABLE_ENV}=1 plus {PROVIDER_ENV} or {PROVIDER_PROFILE_ENV}, and {MODEL_ENV} to run"
        );
        return Ok(());
    }

    let provider = non_empty_env(PROVIDER_ENV);
    let provider_profile = non_empty_env(PROVIDER_PROFILE_ENV);
    let provider_config = non_empty_env(PROVIDER_CONFIG_ENV);
    let model = non_empty_env(MODEL_ENV)
        .ok_or_else(|| anyhow::anyhow!("{MODEL_ENV} is required for live provider smoke"))?;
    if provider.is_none() && provider_profile.is_none() {
        anyhow::bail!("either {PROVIDER_ENV} or {PROVIDER_PROFILE_ENV} is required");
    }
    let provider_config = match (&provider_profile, provider_config) {
        (Some(_), Some(path)) => Some(PathBuf::from(path)),
        (Some(_), None) => anyhow::bail!(
            "{PROVIDER_CONFIG_ENV} is required when {PROVIDER_PROFILE_ENV} is set so the reviewed config.toml can be copied into the isolated JCODE_HOME"
        ),
        (None, path) => path.map(PathBuf::from),
    };

    let prompt = non_empty_env(PROMPT_ENV)
        .unwrap_or_else(|| "Reply with exactly: jcode-harness-live-smoke-ok".to_string());

    let temp = tempfile::Builder::new()
        .prefix("jcode-harness-live-provider-")
        .tempdir()?;
    let home = temp.path().join("isolated-home");
    let runtime = home.join("runtime");
    let cwd = temp.path().join("workspace");
    std::fs::create_dir_all(&runtime)?;
    std::fs::create_dir_all(&cwd)?;
    if let Some(config_path) = provider_config {
        std::fs::copy(&config_path, home.join("config.toml")).with_context(|| {
            format!(
                "copying live-provider smoke config from {} into isolated JCODE_HOME",
                config_path.display()
            )
        })?;
    }

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_jcode-harness"));
    cmd.env("JCODE_HOME", &home)
        .env("JCODE_RUNTIME_DIR", &runtime)
        .env_remove("JCODE_TEST_SESSION")
        .env_remove("JCODE_DEBUG_CONTROL")
        .current_dir(&cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .arg("run")
        .arg(&prompt)
        .arg("--cwd")
        .arg(&cwd)
        .arg("--json")
        .arg("--skills")
        .arg("off")
        .arg("--max-turns")
        .arg("1")
        .arg("--model")
        .arg(&model);

    if let Some(provider) = &provider {
        cmd.args(["--provider", provider]);
    }
    if let Some(profile) = &provider_profile {
        cmd.args(["--provider-profile", profile]);
    }

    let output = cmd.output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "live provider smoke failed. stdout: {stdout}\nstderr: {stderr}"
    );

    let report: Value = serde_json::from_str(&stdout)?;
    assert!(
        report["session_id"]
            .as_str()
            .is_some_and(|id| !id.is_empty()),
        "stdout: {stdout}"
    );
    assert!(
        report["provider"]
            .as_str()
            .is_some_and(|name| !name.is_empty()),
        "stdout: {stdout}"
    );
    assert_eq!(report["model"].as_str(), Some(model.as_str()));
    assert!(
        report["text"]
            .as_str()
            .is_some_and(|text| !text.trim().is_empty()),
        "stdout: {stdout}"
    );

    Ok(())
}
