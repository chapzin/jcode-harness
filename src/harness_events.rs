use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tokio::sync::broadcast;

pub const HARNESS_EVENT_SCHEMA_VERSION: u16 = 1;
const DEFAULT_EVENT_BUS_CAPACITY: usize = 1024;

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
    pub payload: Value,
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
            payload,
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
}

#[derive(Clone, Debug, PartialEq)]
pub struct HarnessEventDraft {
    run_id: String,
    session_id: Option<String>,
    parent_event_id: Option<String>,
    level: HarnessEventLevel,
    kind: HarnessEventKind,
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
            payload: draft.payload,
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
        assert_eq!(value["payload"]["tool"], "cargo test");
        assert_eq!(value["payload"]["status"], "passed");
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
