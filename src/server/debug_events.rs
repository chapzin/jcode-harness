use super::state::MAX_EVENT_HISTORY;
use super::{SwarmEvent, SwarmEventType};
use anyhow::Result;
use std::sync::Arc;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::sync::{RwLock, broadcast};

pub(super) async fn maybe_handle_event_query_command(
    cmd: &str,
    event_history: &Arc<RwLock<std::collections::VecDeque<SwarmEvent>>>,
) -> Option<String> {
    if cmd == "events:recent" || cmd.starts_with("events:recent:") {
        let count: usize = cmd
            .strip_prefix("events:recent:")
            .and_then(|s| s.parse().ok())
            .unwrap_or(50);

        let history = event_history.read().await;
        let events: Vec<serde_json::Value> = history
            .iter()
            .rev()
            .take(count)
            .map(event_payload)
            .collect();
        return Some(serde_json::to_string_pretty(&events).unwrap_or_else(|_| "[]".to_string()));
    }

    if cmd.starts_with("events:since:") {
        let since_id: u64 = cmd
            .strip_prefix("events:since:")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let history = event_history.read().await;
        let events: Vec<serde_json::Value> = history
            .iter()
            .filter(|event| event.id > since_id)
            .map(event_payload)
            .collect();
        return Some(serde_json::to_string_pretty(&events).unwrap_or_else(|_| "[]".to_string()));
    }

    if cmd == "events:types" {
        return Some(
            serde_json::json!({
                "types": [
                    "file_touch",
                    "notification",
                    "plan_update",
                    "plan_proposal",
                    "context_update",
                    "status_change",
                    "member_change"
                ],
                "description": "Use events:recent, events:since:<id>, or events:subscribe to get events"
            })
            .to_string(),
        );
    }

    if cmd == "events:count" {
        let history = event_history.read().await;
        let latest_id = history.back().map(|event| event.id).unwrap_or(0);
        return Some(
            serde_json::json!({
                "count": history.len(),
                "latest_id": latest_id,
                "max_history": MAX_EVENT_HISTORY,
            })
            .to_string(),
        );
    }

    None
}

pub(super) async fn maybe_handle_event_subscription_command<W: AsyncWrite + Unpin>(
    id: u64,
    cmd: &str,
    swarm_event_tx: &broadcast::Sender<SwarmEvent>,
    writer: &mut W,
) -> Result<bool> {
    if cmd != "events:subscribe" && !cmd.starts_with("events:subscribe:") {
        return Ok(false);
    }

    let type_filter: Option<Vec<String>> = cmd
        .strip_prefix("events:subscribe:")
        .map(|s| s.split(',').map(|t| t.trim().to_string()).collect());

    let ack = crate::protocol::ServerEvent::DebugResponse {
        id,
        ok: true,
        output: serde_json::json!({
            "subscribed": true,
            "filter": type_filter.as_ref().map(|f| f.join(",")),
        })
        .to_string(),
    };
    let json = crate::protocol::encode_event(&ack);
    writer.write_all(json.as_bytes()).await?;

    let mut rx = swarm_event_tx.subscribe();
    loop {
        match rx.recv().await {
            Ok(event) => {
                let event_type = match &event.event {
                    SwarmEventType::FileTouch { .. } => "file_touch",
                    SwarmEventType::Notification { .. } => "notification",
                    SwarmEventType::PlanUpdate { .. } => "plan_update",
                    SwarmEventType::PlanProposal { .. } => "plan_proposal",
                    SwarmEventType::ContextUpdate { .. } => "context_update",
                    SwarmEventType::StatusChange { .. } => "status_change",
                    SwarmEventType::MemberChange { .. } => "member_change",
                };
                if let Some(ref filter) = type_filter
                    && !filter.iter().any(|f| f == event_type)
                {
                    continue;
                }
                let event_json = subscription_event_payload(&event);
                let mut line = serde_json::to_string(&event_json).unwrap_or_default();
                line.push('\n');
                if writer.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Lagged(missed)) => {
                let lag_json = serde_json::json!({
                    "type": "lag",
                    "missed": missed,
                });
                let mut line = serde_json::to_string(&lag_json).unwrap_or_default();
                line.push('\n');
                if writer.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }

    Ok(true)
}

fn event_payload(event: &SwarmEvent) -> serde_json::Value {
    let mut payload = base_event_payload(event);
    payload.insert(
        "age_secs".to_string(),
        serde_json::json!(event.timestamp.elapsed().as_secs()),
    );
    serde_json::Value::Object(payload)
}

fn subscription_event_payload(event: &SwarmEvent) -> serde_json::Value {
    let mut payload = base_event_payload(event);
    payload.insert("type".to_string(), serde_json::json!("event"));
    serde_json::Value::Object(payload)
}

fn base_event_payload(event: &SwarmEvent) -> serde_json::Map<String, serde_json::Value> {
    let timestamp_unix = event
        .absolute_time
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut payload = serde_json::Map::new();
    payload.insert("id".to_string(), serde_json::json!(event.id));
    payload.insert(
        "session_id".to_string(),
        serde_json::json!(event.session_id),
    );
    payload.insert(
        "session_name".to_string(),
        serde_json::json!(event.session_name),
    );
    payload.insert("swarm_id".to_string(), serde_json::json!(event.swarm_id));
    payload.insert("event".to_string(), serde_json::json!(event.event));
    payload.insert(
        "timestamp_unix".to_string(),
        serde_json::json!(timestamp_unix),
    );
    if let Some(member) = &event.member {
        if let Some(run_id) = member.run_id.as_deref() {
            payload.insert("run_id".to_string(), serde_json::json!(run_id));
        }
        if let Some(role) = member.role.as_deref() {
            payload.insert("role".to_string(), serde_json::json!(role));
        }
        if let Some(status) = member.status.as_deref() {
            payload.insert("status".to_string(), serde_json::json!(status));
        }
        if let Some(working_dir) = member.working_dir.as_deref() {
            payload.insert("working_dir".to_string(), serde_json::json!(working_dir));
        }
    }
    payload
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::SwarmEventMemberMetadata;
    use std::time::{Instant, SystemTime};

    fn event(member: Option<SwarmEventMemberMetadata>) -> SwarmEvent {
        SwarmEvent {
            id: 7,
            session_id: "session-worker".to_string(),
            session_name: Some("bear".to_string()),
            swarm_id: Some("swarm-test".to_string()),
            member,
            event: SwarmEventType::StatusChange {
                old_status: "running".to_string(),
                new_status: "ready".to_string(),
            },
            timestamp: Instant::now(),
            absolute_time: SystemTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn debug_event_payload_includes_member_metadata_when_present() {
        let payload = event_payload(&event(Some(SwarmEventMemberMetadata {
            run_id: Some("run-123".to_string()),
            role: Some("agent".to_string()),
            status: Some("ready".to_string()),
            working_dir: Some("/tmp/jcode-worktrees/bear".to_string()),
        })));

        assert_eq!(payload["run_id"], "run-123");
        assert_eq!(payload["role"], "agent");
        assert_eq!(payload["status"], "ready");
        assert_eq!(payload["working_dir"], "/tmp/jcode-worktrees/bear");
    }

    #[test]
    fn debug_event_payload_omits_member_metadata_when_absent() {
        let payload = event_payload(&event(None));

        assert!(payload.get("run_id").is_none());
        assert!(payload.get("role").is_none());
        assert!(payload.get("status").is_none());
        assert!(payload.get("working_dir").is_none());
    }

    #[test]
    fn subscription_event_payload_reuses_member_metadata() {
        let payload = subscription_event_payload(&event(Some(SwarmEventMemberMetadata {
            run_id: Some("run-stream".to_string()),
            role: Some("coordinator".to_string()),
            status: Some("running".to_string()),
            working_dir: None,
        })));

        assert_eq!(payload["type"], "event");
        assert_eq!(payload["run_id"], "run-stream");
        assert_eq!(payload["role"], "coordinator");
        assert_eq!(payload["status"], "running");
        assert!(payload.get("working_dir").is_none());
    }
}
