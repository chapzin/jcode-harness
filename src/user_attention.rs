//! Configurable, local-only user-attention alerts.
//!
//! This module intentionally starts with a conservative terminal-bell backend so
//! tests can validate routing without requiring a real audio stack or desktop
//! notification service. The default is silent for CI/headless environments.

use serde::Serialize;
use std::io::{self, Write};

pub const USER_ATTENTION_ENV: &str = "JCODE_USER_ATTENTION";
pub const NOTIFY_SOUND_ENV: &str = "JCODE_NOTIFY_SOUND";
const TERMINAL_BELL: &[u8] = b"\x07";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserAttentionMode {
    Off,
    Bell,
}

impl UserAttentionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Bell => "bell",
        }
    }

    pub fn backend(self) -> Option<&'static str> {
        match self {
            Self::Off => None,
            Self::Bell => Some("terminal_bell"),
        }
    }

    pub fn enabled(self) -> bool {
        !matches!(self, Self::Off)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserAttentionConfig {
    mode: UserAttentionMode,
    source: &'static str,
    warning: Option<String>,
}

impl UserAttentionConfig {
    pub fn from_env() -> Self {
        let user_attention = std::env::var(USER_ATTENTION_ENV).ok();
        let notify_sound = std::env::var(NOTIFY_SOUND_ENV).ok();
        Self::from_env_values(user_attention.as_deref(), notify_sound.as_deref())
    }

    pub fn from_env_values(user_attention: Option<&str>, notify_sound: Option<&str>) -> Self {
        if let Some(value) = user_attention {
            return parse_user_attention(value, USER_ATTENTION_ENV);
        }
        if let Some(value) = notify_sound {
            return parse_notify_sound(value);
        }
        Self {
            mode: UserAttentionMode::Off,
            source: "default",
            warning: None,
        }
    }

    pub fn mode(&self) -> UserAttentionMode {
        self.mode
    }

    pub fn source(&self) -> &'static str {
        self.source
    }

    pub fn warning(&self) -> Option<&str> {
        self.warning.as_deref()
    }

    pub fn diagnostic(&self) -> UserAttentionDiagnostic {
        UserAttentionDiagnostic {
            enabled: self.mode.enabled(),
            mode: self.mode.as_str(),
            backend: self.mode.backend(),
            source: self.source,
            warning: self.warning.clone(),
        }
    }

    pub fn dry_run_delivery(&self) -> UserAttentionDelivery {
        UserAttentionDelivery {
            backend: self.mode.backend(),
            would_emit: self.mode.enabled(),
            attempted: false,
            delivered: false,
            dry_run: true,
            bytes_written: 0,
        }
    }

    pub fn notify_with_writer<W: Write>(
        &self,
        writer: &mut W,
    ) -> io::Result<UserAttentionDelivery> {
        match self.mode {
            UserAttentionMode::Off => Ok(UserAttentionDelivery {
                backend: None,
                would_emit: false,
                attempted: false,
                delivered: false,
                dry_run: false,
                bytes_written: 0,
            }),
            UserAttentionMode::Bell => {
                writer.write_all(TERMINAL_BELL)?;
                writer.flush()?;
                Ok(UserAttentionDelivery {
                    backend: Some("terminal_bell"),
                    would_emit: true,
                    attempted: true,
                    delivered: true,
                    dry_run: false,
                    bytes_written: TERMINAL_BELL.len(),
                })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserAttentionDiagnostic {
    pub enabled: bool,
    pub mode: &'static str,
    pub backend: Option<&'static str>,
    pub source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserAttentionDelivery {
    pub backend: Option<&'static str>,
    pub would_emit: bool,
    pub attempted: bool,
    pub delivered: bool,
    pub dry_run: bool,
    pub bytes_written: usize,
}

fn parse_user_attention(value: &str, source: &'static str) -> UserAttentionConfig {
    let normalized = normalize(value);
    let mode = match normalized.as_str() {
        "bell" | "terminal-bell" | "terminal_bell" | "on" | "1" | "true" | "yes" => {
            Some(UserAttentionMode::Bell)
        }
        "off" | "0" | "false" | "no" | "none" | "disabled" | "disable" | "" => {
            Some(UserAttentionMode::Off)
        }
        _ => None,
    };

    match mode {
        Some(mode) => UserAttentionConfig {
            mode,
            source,
            warning: None,
        },
        None => unsupported_value(source, value, "expected `bell` or `off`; defaulting to off"),
    }
}

fn parse_notify_sound(value: &str) -> UserAttentionConfig {
    let normalized = normalize(value);
    let mode = match normalized.as_str() {
        "1" | "true" | "yes" | "on" | "bell" => Some(UserAttentionMode::Bell),
        "0" | "false" | "no" | "off" | "none" | "disabled" | "disable" | "" => {
            Some(UserAttentionMode::Off)
        }
        _ => None,
    };

    match mode {
        Some(mode) => UserAttentionConfig {
            mode,
            source: NOTIFY_SOUND_ENV,
            warning: None,
        },
        None => unsupported_value(
            NOTIFY_SOUND_ENV,
            value,
            "expected a truthy value like `1` or a falsy value like `0`; defaulting to off",
        ),
    }
}

fn normalize(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn unsupported_value(source: &'static str, value: &str, guidance: &str) -> UserAttentionConfig {
    UserAttentionConfig {
        mode: UserAttentionMode::Off,
        source,
        warning: Some(format!("unsupported {source} value {value:?}; {guidance}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_silent() {
        let config = UserAttentionConfig::from_env_values(None, None);

        assert_eq!(config.mode(), UserAttentionMode::Off);
        assert_eq!(config.source(), "default");
        assert_eq!(config.warning(), None);
        assert_eq!(config.diagnostic().backend, None);
    }

    #[test]
    fn notify_sound_truthy_enables_terminal_bell() {
        let config = UserAttentionConfig::from_env_values(None, Some("1"));

        assert_eq!(config.mode(), UserAttentionMode::Bell);
        assert_eq!(config.source(), NOTIFY_SOUND_ENV);
        assert_eq!(config.diagnostic().backend, Some("terminal_bell"));
    }

    #[test]
    fn user_attention_env_overrides_notify_sound() {
        let config = UserAttentionConfig::from_env_values(Some("off"), Some("1"));

        assert_eq!(config.mode(), UserAttentionMode::Off);
        assert_eq!(config.source(), USER_ATTENTION_ENV);
    }

    #[test]
    fn invalid_values_default_to_off_with_warning() {
        let config = UserAttentionConfig::from_env_values(Some("loud"), None);

        assert_eq!(config.mode(), UserAttentionMode::Off);
        assert!(
            config
                .warning()
                .is_some_and(|warning| warning.contains("loud"))
        );
    }

    #[test]
    fn bell_backend_writes_only_terminal_bell() {
        let config = UserAttentionConfig::from_env_values(Some("bell"), None);
        let mut output = Vec::new();

        let delivery = config.notify_with_writer(&mut output).unwrap();

        assert_eq!(output, TERMINAL_BELL);
        assert!(delivery.would_emit);
        assert!(delivery.attempted);
        assert!(delivery.delivered);
        assert_eq!(delivery.bytes_written, 1);
    }

    #[test]
    fn off_backend_writes_nothing() {
        let config = UserAttentionConfig::from_env_values(Some("off"), None);
        let mut output = Vec::new();

        let delivery = config.notify_with_writer(&mut output).unwrap();

        assert!(output.is_empty());
        assert!(!delivery.would_emit);
        assert!(!delivery.attempted);
        assert!(!delivery.delivered);
        assert_eq!(delivery.bytes_written, 0);
    }

    #[test]
    fn dry_run_reports_without_writing() {
        let config = UserAttentionConfig::from_env_values(Some("bell"), None);

        let delivery = config.dry_run_delivery();

        assert!(delivery.would_emit);
        assert!(!delivery.attempted);
        assert!(!delivery.delivered);
        assert!(delivery.dry_run);
        assert_eq!(delivery.backend, Some("terminal_bell"));
    }
}
