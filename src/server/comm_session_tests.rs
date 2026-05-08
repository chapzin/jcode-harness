use super::{
    ensure_spawn_coordinator_swarm, handle_comm_stop, prepare_visible_spawn_session,
    register_visible_spawned_member, require_coordinator_swarm, resolve_spawn_working_dir,
    resolve_stop_target_session, spawn_mutation_key, swarm_stop_allowed_by_owner,
};
use crate::agent::Agent;
use crate::message::{Message, ToolDefinition};
use crate::protocol::{NotificationType, ServerEvent};
use crate::provider::{EventStream, Provider};
use crate::server::{
    SessionInterruptQueues, SwarmEventType, SwarmMember, SwarmMutationRuntime, VersionedPlan,
};
use crate::tool::Registry;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock, broadcast, mpsc};

struct MockProvider;

#[async_trait]
impl Provider for MockProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolDefinition],
        _system: &str,
        _resume_session_id: Option<&str>,
    ) -> Result<EventStream> {
        Err(anyhow::anyhow!("mock provider should not be called"))
    }

    fn name(&self) -> &str {
        "mock"
    }

    fn fork(&self) -> Arc<dyn Provider> {
        Arc::new(MockProvider)
    }
}

fn member(
    session_id: &str,
    swarm_id: Option<&str>,
    role: &str,
) -> (SwarmMember, mpsc::UnboundedReceiver<ServerEvent>) {
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    (
        SwarmMember {
            session_id: session_id.to_string(),
            event_tx,
            event_txs: HashMap::new(),
            working_dir: None,
            swarm_id: swarm_id.map(|id| id.to_string()),
            swarm_enabled: true,
            status: "ready".to_string(),
            detail: None,
            friendly_name: Some(session_id.to_string()),
            report_back_to_session_id: None,
            run_id: None,
            latest_completion_report: None,
            role: role.to_string(),
            joined_at: Instant::now(),
            last_status_change: Instant::now(),
            is_headless: false,
        },
        event_rx,
    )
}

async fn test_agent_with_working_dir(session_id: &str, working_dir: &str) -> Arc<Mutex<Agent>> {
    let provider: Arc<dyn Provider> = Arc::new(MockProvider);
    let registry = Registry::new(provider.clone()).await;
    let mut session = crate::session::Session::create_with_id(session_id.to_string(), None, None);
    session.model = Some("mock".to_string());
    let agent = Arc::new(Mutex::new(Agent::new_with_session(
        provider, registry, session, None,
    )));
    agent.lock().await.set_working_dir(working_dir);
    agent
}

#[tokio::test]
async fn resolve_spawn_working_dir_prefers_explicit_then_spawner_agent_dir() {
    let sessions = Arc::new(RwLock::new(HashMap::new()));
    sessions.write().await.insert(
        "req".to_string(),
        test_agent_with_working_dir("req", "/tmp/spawner-agent").await,
    );
    let swarm_members = Arc::new(RwLock::new(HashMap::new()));

    assert_eq!(
        resolve_spawn_working_dir(
            Some("/tmp/explicit".to_string()),
            "req",
            &sessions,
            &swarm_members,
        )
        .await
        .as_deref(),
        Some("/tmp/explicit")
    );
    assert_eq!(
        resolve_spawn_working_dir(None, "req", &sessions, &swarm_members)
            .await
            .as_deref(),
        Some("/tmp/spawner-agent")
    );
}

#[tokio::test]
async fn resolve_spawn_working_dir_falls_back_to_member_dir() {
    let sessions = Arc::new(RwLock::new(HashMap::new()));
    let swarm_members = Arc::new(RwLock::new(HashMap::new()));
    let (mut req_member, _rx) = member("req", Some("swarm-1"), "coordinator");
    req_member.working_dir = Some(std::path::PathBuf::from("/tmp/member-dir"));
    swarm_members
        .write()
        .await
        .insert("req".to_string(), req_member);

    assert_eq!(
        resolve_spawn_working_dir(None, "req", &sessions, &swarm_members)
            .await
            .as_deref(),
        Some("/tmp/member-dir")
    );
}

#[test]
fn spawn_mutation_key_uses_nonce_as_idempotency_key_despite_run_and_payload_drift() {
    let operation_key = spawn_mutation_key(
        "coord",
        "swarm-1",
        &Some("/repo-a".to_string()),
        &Some("first prompt".to_string()),
        &Some(" op:issue-13-spawn-1 ".to_string()),
        &Some("run-1".to_string()),
    );
    let retry_with_drifted_payload = spawn_mutation_key(
        "coord",
        "swarm-1",
        &Some("/repo-b".to_string()),
        &Some("retry prompt drift".to_string()),
        &Some("op:issue-13-spawn-1".to_string()),
        &Some("run-1".to_string()),
    );
    let different_run = spawn_mutation_key(
        "coord",
        "swarm-1",
        &Some("/repo-b".to_string()),
        &Some("retry prompt drift".to_string()),
        &Some("op:issue-13-spawn-1".to_string()),
        &Some("run-2".to_string()),
    );
    let no_nonce = spawn_mutation_key(
        "coord",
        "swarm-1",
        &Some("/repo-a".to_string()),
        &Some("first prompt".to_string()),
        &None,
        &Some("run-1".to_string()),
    );
    let no_nonce_payload_drift = spawn_mutation_key(
        "coord",
        "swarm-1",
        &Some("/repo-b".to_string()),
        &Some("retry prompt drift".to_string()),
        &None,
        &Some("run-1".to_string()),
    );

    assert_eq!(operation_key, retry_with_drifted_payload);
    assert_eq!(operation_key, different_run);
    assert_ne!(no_nonce, no_nonce_payload_drift);
}

#[test]
fn stop_permission_defaults_to_sessions_spawned_by_requesting_coordinator() {
    let (mut owned, _owned_rx) = member("worker-owned", Some("swarm-1"), "agent");
    owned.report_back_to_session_id = Some("coord".to_string());
    let (mut user_created, _user_rx) = member("worker-user", Some("swarm-1"), "agent");
    user_created.report_back_to_session_id = None;
    let (mut other_owned, _other_rx) = member("worker-other", Some("swarm-1"), "agent");
    other_owned.report_back_to_session_id = Some("other-coord".to_string());

    assert!(swarm_stop_allowed_by_owner("coord", &owned, false));
    assert!(!swarm_stop_allowed_by_owner("coord", &user_created, false));
    assert!(!swarm_stop_allowed_by_owner("coord", &other_owned, false));
    assert!(swarm_stop_allowed_by_owner("coord", &user_created, true));
}

#[tokio::test]
async fn stop_target_resolves_unique_friendly_name_and_suffix() {
    let swarm_members = Arc::new(RwLock::new(HashMap::new()));
    let (mut worker, _worker_rx) = member("session_jellyfish_1234_abcd", Some("swarm-1"), "agent");
    worker.friendly_name = Some("jellyfish".to_string());
    swarm_members
        .write()
        .await
        .insert(worker.session_id.clone(), worker);

    assert_eq!(
        resolve_stop_target_session("swarm-1", "jellyfish", &swarm_members)
            .await
            .as_deref(),
        Ok("session_jellyfish_1234_abcd")
    );
    assert_eq!(
        resolve_stop_target_session("swarm-1", "abcd", &swarm_members)
            .await
            .as_deref(),
        Ok("session_jellyfish_1234_abcd")
    );
}

#[tokio::test]
async fn stop_target_rejects_ambiguous_friendly_name() {
    let swarm_members = Arc::new(RwLock::new(HashMap::new()));
    let (mut first, _first_rx) = member("session_bear_1", Some("swarm-1"), "agent");
    first.friendly_name = Some("bear".to_string());
    let (mut second, _second_rx) = member("session_bear_2", Some("swarm-1"), "agent");
    second.friendly_name = Some("bear".to_string());
    let mut members = swarm_members.write().await;
    members.insert(first.session_id.clone(), first);
    members.insert(second.session_id.clone(), second);
    drop(members);

    let err = resolve_stop_target_session("swarm-1", "bear", &swarm_members)
        .await
        .expect_err("ambiguous friendly names should be rejected");
    assert!(err.contains("Ambiguous swarm session 'bear'"));
}

#[tokio::test]
async fn stop_replay_does_not_send_duplicate_close_request() {
    let _guard = crate::storage::lock_test_env();
    let temp_home = tempfile::TempDir::new().expect("temp home");
    crate::env::set_var("JCODE_HOME", temp_home.path());

    let sessions = Arc::new(RwLock::new(HashMap::new()));
    let soft_interrupt_queues: SessionInterruptQueues = Arc::new(RwLock::new(HashMap::new()));
    let swarm_members = Arc::new(RwLock::new(HashMap::new()));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
        "swarm-1".to_string(),
        HashSet::from(["coord".to_string(), "worker-1".to_string()]),
    )])));
    let swarm_coordinators = Arc::new(RwLock::new(HashMap::from([(
        "swarm-1".to_string(),
        "coord".to_string(),
    )])));
    let swarm_plans = Arc::new(RwLock::new(HashMap::<String, VersionedPlan>::new()));
    let channel_subscriptions = Arc::new(RwLock::new(HashMap::new()));
    let channel_subscriptions_by_session = Arc::new(RwLock::new(HashMap::new()));
    let event_history = Arc::new(RwLock::new(VecDeque::new()));
    let event_counter = Arc::new(AtomicU64::new(0));
    let (swarm_event_tx, _swarm_event_rx) = broadcast::channel(8);
    let swarm_mutation_runtime = SwarmMutationRuntime::default();
    let (client_event_tx, mut client_event_rx) = mpsc::unbounded_channel();

    let (coord, _coord_rx) = member("coord", Some("swarm-1"), "coordinator");
    let (mut worker, mut worker_rx) = member("worker-1", Some("swarm-1"), "agent");
    worker.report_back_to_session_id = Some("coord".to_string());
    swarm_members.write().await.extend([
        ("coord".to_string(), coord),
        ("worker-1".to_string(), worker),
    ]);

    handle_comm_stop(
        1,
        "coord".to_string(),
        "worker-1".to_string(),
        false,
        &client_event_tx,
        &sessions,
        &swarm_members,
        &swarms_by_id,
        &swarm_coordinators,
        &swarm_plans,
        &channel_subscriptions,
        &channel_subscriptions_by_session,
        &event_history,
        &event_counter,
        &swarm_event_tx,
        &soft_interrupt_queues,
        &swarm_mutation_runtime,
    )
    .await;

    assert!(matches!(
        worker_rx.recv().await,
        Some(ServerEvent::SessionCloseRequested { reason })
            if reason == "Stopped by coordinator coord"
    ));
    assert!(matches!(
        client_event_rx.recv().await,
        Some(ServerEvent::Done { id: 1 })
    ));
    assert!(
        !swarm_members.read().await.contains_key("worker-1"),
        "first stop should remove worker membership"
    );

    handle_comm_stop(
        2,
        "coord".to_string(),
        "worker-1".to_string(),
        false,
        &client_event_tx,
        &sessions,
        &swarm_members,
        &swarms_by_id,
        &swarm_coordinators,
        &swarm_plans,
        &channel_subscriptions,
        &channel_subscriptions_by_session,
        &event_history,
        &event_counter,
        &swarm_event_tx,
        &soft_interrupt_queues,
        &swarm_mutation_runtime,
    )
    .await;

    assert!(matches!(
        client_event_rx.recv().await,
        Some(ServerEvent::Done { id: 2 })
    ));
    assert!(
        worker_rx.try_recv().is_err(),
        "replayed stop must not send a duplicate close request"
    );

    crate::env::remove_var("JCODE_HOME");
}

#[tokio::test]
async fn stop_friendly_name_reuse_uses_current_resolved_target() {
    let _guard = crate::storage::lock_test_env();
    let temp_home = tempfile::TempDir::new().expect("temp home");
    crate::env::set_var("JCODE_HOME", temp_home.path());

    let sessions = Arc::new(RwLock::new(HashMap::new()));
    let soft_interrupt_queues: SessionInterruptQueues = Arc::new(RwLock::new(HashMap::new()));
    let swarm_members = Arc::new(RwLock::new(HashMap::new()));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
        "swarm-1".to_string(),
        HashSet::from(["coord".to_string(), "worker-old".to_string()]),
    )])));
    let swarm_coordinators = Arc::new(RwLock::new(HashMap::from([(
        "swarm-1".to_string(),
        "coord".to_string(),
    )])));
    let swarm_plans = Arc::new(RwLock::new(HashMap::<String, VersionedPlan>::new()));
    let channel_subscriptions = Arc::new(RwLock::new(HashMap::new()));
    let channel_subscriptions_by_session = Arc::new(RwLock::new(HashMap::new()));
    let event_history = Arc::new(RwLock::new(VecDeque::new()));
    let event_counter = Arc::new(AtomicU64::new(0));
    let (swarm_event_tx, _swarm_event_rx) = broadcast::channel(8);
    let swarm_mutation_runtime = SwarmMutationRuntime::default();
    let (client_event_tx, mut client_event_rx) = mpsc::unbounded_channel();

    let (coord, _coord_rx) = member("coord", Some("swarm-1"), "coordinator");
    let (mut old_worker, mut old_worker_rx) = member("worker-old", Some("swarm-1"), "agent");
    old_worker.friendly_name = Some("bear".to_string());
    old_worker.report_back_to_session_id = Some("coord".to_string());
    swarm_members.write().await.extend([
        ("coord".to_string(), coord),
        ("worker-old".to_string(), old_worker),
    ]);

    handle_comm_stop(
        1,
        "coord".to_string(),
        "bear".to_string(),
        false,
        &client_event_tx,
        &sessions,
        &swarm_members,
        &swarms_by_id,
        &swarm_coordinators,
        &swarm_plans,
        &channel_subscriptions,
        &channel_subscriptions_by_session,
        &event_history,
        &event_counter,
        &swarm_event_tx,
        &soft_interrupt_queues,
        &swarm_mutation_runtime,
    )
    .await;
    assert!(matches!(
        old_worker_rx.recv().await,
        Some(ServerEvent::SessionCloseRequested { .. })
    ));
    assert!(matches!(
        client_event_rx.recv().await,
        Some(ServerEvent::Done { id: 1 })
    ));

    let (mut new_worker, mut new_worker_rx) = member("worker-new", Some("swarm-1"), "agent");
    new_worker.friendly_name = Some("bear".to_string());
    new_worker.report_back_to_session_id = Some("coord".to_string());
    swarm_members
        .write()
        .await
        .insert("worker-new".to_string(), new_worker);
    swarms_by_id
        .write()
        .await
        .entry("swarm-1".to_string())
        .or_default()
        .insert("worker-new".to_string());

    handle_comm_stop(
        2,
        "coord".to_string(),
        "bear".to_string(),
        false,
        &client_event_tx,
        &sessions,
        &swarm_members,
        &swarms_by_id,
        &swarm_coordinators,
        &swarm_plans,
        &channel_subscriptions,
        &channel_subscriptions_by_session,
        &event_history,
        &event_counter,
        &swarm_event_tx,
        &soft_interrupt_queues,
        &swarm_mutation_runtime,
    )
    .await;

    assert!(matches!(
        new_worker_rx.recv().await,
        Some(ServerEvent::SessionCloseRequested { reason })
            if reason == "Stopped by coordinator coord"
    ));
    assert!(matches!(
        client_event_rx.recv().await,
        Some(ServerEvent::Done { id: 2 })
    ));
    assert!(
        !swarm_members.read().await.contains_key("worker-new"),
        "friendly-name reuse should resolve and stop the current matching member"
    );

    crate::env::remove_var("JCODE_HOME");
}

#[tokio::test]
async fn register_visible_spawned_member_marks_startup_as_running() {
    let swarm_members = Arc::new(RwLock::new(HashMap::new()));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::new()));
    let event_history = Arc::new(RwLock::new(VecDeque::new()));
    let event_counter = Arc::new(AtomicU64::new(0));
    let (swarm_event_tx, _swarm_event_rx) = broadcast::channel(8);

    register_visible_spawned_member(
        "child-1",
        "swarm-1",
        Some("/tmp/worktree"),
        true,
        Some("owner"),
        Some("run-visible"),
        &swarm_members,
        &swarms_by_id,
        &event_history,
        &event_counter,
        &swarm_event_tx,
    )
    .await;

    let members = swarm_members.read().await;
    let member = members.get("child-1").expect("spawned member should exist");
    assert_eq!(member.status, "running");
    assert_eq!(member.detail.as_deref(), Some("startup queued"));
    assert_eq!(member.swarm_id.as_deref(), Some("swarm-1"));
    assert_eq!(member.run_id.as_deref(), Some("run-visible"));
    assert_eq!(
        member.working_dir.as_deref(),
        Some(std::path::Path::new("/tmp/worktree"))
    );
    drop(members);

    assert!(
        swarms_by_id
            .read()
            .await
            .get("swarm-1")
            .is_some_and(|members| members.contains("child-1"))
    );

    let history = event_history.read().await;
    assert!(history.iter().any(|event| {
            event.session_id == "child-1"
                && matches!(event.event, SwarmEventType::MemberChange { ref action } if action == "joined")
        }));
}

#[test]
fn prepare_visible_spawn_session_persists_startup_before_launch() {
    let _guard = crate::storage::lock_test_env();
    let temp_home = tempfile::TempDir::new().expect("temp home");
    crate::env::set_var("JCODE_HOME", temp_home.path());

    let worktree = tempfile::TempDir::new().expect("temp worktree");
    let startup = "Please start by auditing prompt delivery.";

    let (session_id, launched) = prepare_visible_spawn_session(
        Some(worktree.path().to_str().expect("utf8 worktree path")),
        None,
        false,
        Some(startup),
        |session_id, _cwd: &std::path::Path, _selfdev| {
            let path = crate::storage::jcode_dir()
                .expect("jcode dir")
                .join(format!("client-input-{}", session_id));
            let data = std::fs::read_to_string(&path).expect("startup file should exist");
            assert!(
                data.contains(startup),
                "startup payload should be written before launch"
            );
            assert!(
                data.contains(r#""submit_on_restore":true"#),
                "startup payload should auto-submit on restore"
            );
            Ok(true)
        },
    )
    .expect("visible spawn preparation should succeed");

    assert!(launched);
    let path = crate::storage::jcode_dir()
        .expect("jcode dir")
        .join(format!("client-input-{}", session_id));
    assert!(
        path.exists(),
        "startup file should remain for launched visible session"
    );

    crate::env::remove_var("JCODE_HOME");
}

#[test]
fn prepare_visible_spawn_session_cleans_startup_when_launch_not_started() {
    let _guard = crate::storage::lock_test_env();
    let temp_home = tempfile::TempDir::new().expect("temp home");
    crate::env::set_var("JCODE_HOME", temp_home.path());

    let worktree = tempfile::TempDir::new().expect("temp worktree");

    let (session_id, launched) = prepare_visible_spawn_session(
        Some(worktree.path().to_str().expect("utf8 worktree path")),
        None,
        false,
        Some("Do the thing."),
        |_session_id, _cwd: &std::path::Path, _selfdev| Ok(false),
    )
    .expect("visible spawn preparation should succeed even when launch is skipped");

    assert!(!launched);
    let path = crate::storage::jcode_dir()
        .expect("jcode dir")
        .join(format!("client-input-{}", session_id));
    assert!(
        !path.exists(),
        "startup file should be removed when visible launch does not start"
    );
    assert!(
        !crate::session::session_exists(&session_id),
        "prepared session should be cleaned up when visible launch does not start"
    );

    crate::env::remove_var("JCODE_HOME");
}

#[test]
fn prepare_visible_spawn_session_cleans_session_when_launch_errors() {
    let _guard = crate::storage::lock_test_env();
    let temp_home = tempfile::TempDir::new().expect("temp home");
    crate::env::set_var("JCODE_HOME", temp_home.path());

    let worktree = tempfile::TempDir::new().expect("temp worktree");

    let error = prepare_visible_spawn_session(
        Some(worktree.path().to_str().expect("utf8 worktree path")),
        None,
        false,
        Some("Do the thing."),
        |_session_id, _cwd: &std::path::Path, _selfdev| Err(anyhow::anyhow!("launch failed")),
    )
    .expect_err("visible spawn preparation should surface launch error");

    assert!(error.to_string().contains("launch failed"));
    let sessions_dir = crate::storage::jcode_dir()
        .expect("jcode dir")
        .join("sessions");
    let remaining_sessions = std::fs::read_dir(&sessions_dir)
        .map(|entries| entries.count())
        .unwrap_or(0);
    assert_eq!(
        remaining_sessions, 0,
        "failed visible launch should not leave orphan prepared sessions"
    );

    crate::env::remove_var("JCODE_HOME");
}

#[tokio::test]
async fn spawn_bootstraps_coordinator_when_swarm_has_none() {
    let swarm_members = Arc::new(RwLock::new(HashMap::new()));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
        "swarm-1".to_string(),
        HashSet::from(["req".to_string()]),
    )])));
    let swarm_coordinators = Arc::new(RwLock::new(HashMap::new()));
    let swarm_plans = Arc::new(RwLock::new(HashMap::<String, VersionedPlan>::new()));
    let (req_member, _req_rx) = member("req", Some("swarm-1"), "agent");
    swarm_members
        .write()
        .await
        .insert("req".to_string(), req_member);
    let (client_event_tx, mut client_event_rx) = mpsc::unbounded_channel();

    let swarm_id = ensure_spawn_coordinator_swarm(
        1,
        "req",
        "Only the coordinator can spawn new agents.",
        &client_event_tx,
        &swarm_members,
        &swarms_by_id,
        &swarm_coordinators,
        &swarm_plans,
    )
    .await;

    assert_eq!(swarm_id.as_deref(), Some("swarm-1"));
    assert_eq!(
        swarm_coordinators
            .read()
            .await
            .get("swarm-1")
            .map(String::as_str),
        Some("req")
    );
    assert_eq!(
        swarm_members
            .read()
            .await
            .get("req")
            .map(|member| member.role.as_str()),
        Some("coordinator")
    );
    assert!(matches!(
        client_event_rx.recv().await,
        Some(ServerEvent::Notification {
            notification_type: NotificationType::Message { .. },
            message,
            ..
        }) if message == "You are the coordinator for this swarm."
    ));
}

#[tokio::test]
async fn spawn_requires_existing_coordinator_when_one_is_set() {
    let swarm_members = Arc::new(RwLock::new(HashMap::new()));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
        "swarm-1".to_string(),
        HashSet::from(["req".to_string(), "coord".to_string()]),
    )])));
    let swarm_coordinators = Arc::new(RwLock::new(HashMap::from([(
        "swarm-1".to_string(),
        "coord".to_string(),
    )])));
    let swarm_plans = Arc::new(RwLock::new(HashMap::<String, VersionedPlan>::new()));
    let (req_member, _req_rx) = member("req", Some("swarm-1"), "agent");
    let (coord_member, _coord_rx) = member("coord", Some("swarm-1"), "coordinator");
    let mut members = swarm_members.write().await;
    members.insert("req".to_string(), req_member);
    members.insert("coord".to_string(), coord_member);
    drop(members);
    let (client_event_tx, mut client_event_rx) = mpsc::unbounded_channel();

    let swarm_id = ensure_spawn_coordinator_swarm(
        2,
        "req",
        "Only the coordinator can spawn new agents.",
        &client_event_tx,
        &swarm_members,
        &swarms_by_id,
        &swarm_coordinators,
        &swarm_plans,
    )
    .await;

    assert!(swarm_id.is_none());
    assert!(matches!(
        client_event_rx.recv().await,
        Some(ServerEvent::Error { message, .. })
            if message == "Only the coordinator can spawn new agents."
    ));
    assert_eq!(
        swarm_members
            .read()
            .await
            .get("req")
            .map(|member| member.role.as_str()),
        Some("agent")
    );
}

#[tokio::test]
async fn coordinator_actions_self_promote_when_recorded_coordinator_is_stale() {
    let swarm_members = Arc::new(RwLock::new(HashMap::new()));
    let swarm_coordinators = Arc::new(RwLock::new(HashMap::from([(
        "swarm-1".to_string(),
        "old-coord".to_string(),
    )])));
    let (req_member, _req_rx) = member("req", Some("swarm-1"), "agent");
    let (mut old_coord, _old_rx) = member("old-coord", Some("swarm-1"), "coordinator");
    old_coord.status = "crashed".to_string();
    let mut members = swarm_members.write().await;
    members.insert("req".to_string(), req_member);
    members.insert("old-coord".to_string(), old_coord);
    drop(members);
    let (client_event_tx, mut client_event_rx) = mpsc::unbounded_channel();

    let swarm_id = require_coordinator_swarm(
        3,
        "req",
        "Only the coordinator can stop agents.",
        &client_event_tx,
        &swarm_members,
        &swarm_coordinators,
    )
    .await;

    assert_eq!(swarm_id.as_deref(), Some("swarm-1"));
    assert_eq!(
        swarm_coordinators
            .read()
            .await
            .get("swarm-1")
            .map(String::as_str),
        Some("req")
    );
    assert_eq!(
        swarm_members
            .read()
            .await
            .get("req")
            .map(|member| member.role.as_str()),
        Some("coordinator")
    );
    assert!(client_event_rx.try_recv().is_err());
}
