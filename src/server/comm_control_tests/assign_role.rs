#[tokio::test]
async fn assign_role_self_promotion_not_replayed_after_coordinator_cycles() {
    let (_env, _runtime_dir) = RuntimeEnvGuard::new();
    let swarm_id = "swarm-assign-role-replay";
    let coordinator = "coord";
    let headless = "headless-worker";
    let (client_tx, _client_rx) = mpsc::unbounded_channel();
    let (swarm_event_tx, _swarm_event_rx) = broadcast::channel(32);

    let mut coordinator_member = member(coordinator, swarm_id, "running");
    coordinator_member.role = "coordinator".to_string();
    coordinator_member.is_headless = false;
    let mut headless_member = member(headless, swarm_id, "ready");
    headless_member.role = "agent".to_string();
    headless_member.is_headless = true;

    let sessions = Arc::new(RwLock::new(HashMap::new()));
    let swarm_members = Arc::new(RwLock::new(HashMap::from([
        (coordinator.to_string(), coordinator_member),
        (headless.to_string(), headless_member),
    ])));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        HashSet::from([coordinator.to_string(), headless.to_string()]),
    )])));
    let swarm_coordinators = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        coordinator.to_string(),
    )])));
    let swarm_plans = Arc::new(RwLock::new(HashMap::new()));
    let event_history = Arc::new(RwLock::new(VecDeque::new()));
    let event_counter = Arc::new(AtomicU64::new(1));
    let mutation_runtime = SwarmMutationRuntime::default();

    // Record a successful self-promotion while `coord` is already the coordinator.
    handle_comm_assign_role(
        1,
        coordinator.to_string(),
        coordinator.to_string(),
        "coordinator".to_string(),
        &client_tx,
        &sessions,
        &swarm_members,
        &swarms_by_id,
        &swarm_coordinators,
        &swarm_plans,
        &event_history,
        &event_counter,
        &swarm_event_tx,
        &mutation_runtime,
    )
    .await;

    // Move the coordinator to a headless worker. This mirrors a recovered/stale
    // orchestration state where the caller must self-promote again before spawn.
    handle_comm_assign_role(
        2,
        coordinator.to_string(),
        headless.to_string(),
        "coordinator".to_string(),
        &client_tx,
        &sessions,
        &swarm_members,
        &swarms_by_id,
        &swarm_coordinators,
        &swarm_plans,
        &event_history,
        &event_counter,
        &swarm_event_tx,
        &mutation_runtime,
    )
    .await;
    assert_eq!(
        swarm_coordinators
            .read()
            .await
            .get(swarm_id)
            .map(String::as_str),
        Some(headless)
    );
    {
        let members = swarm_members.read().await;
        assert_eq!(
            members.get(coordinator).map(|member| member.role.as_str()),
            Some("agent")
        );
        assert_eq!(
            members.get(headless).map(|member| member.role.as_str()),
            Some("coordinator")
        );
    }

    // This must not replay an earlier self-promotion. Role assignment is
    // idempotent, but stale replay is unsafe once coordinator state changes.
    handle_comm_assign_role(
        3,
        coordinator.to_string(),
        coordinator.to_string(),
        "coordinator".to_string(),
        &client_tx,
        &sessions,
        &swarm_members,
        &swarms_by_id,
        &swarm_coordinators,
        &swarm_plans,
        &event_history,
        &event_counter,
        &swarm_event_tx,
        &mutation_runtime,
    )
    .await;

    assert_eq!(
        swarm_coordinators
            .read()
            .await
            .get(swarm_id)
            .map(String::as_str),
        Some(coordinator)
    );
    {
        let members = swarm_members.read().await;
        assert_eq!(
            members.get(coordinator).map(|member| member.role.as_str()),
            Some("coordinator")
        );
        assert_eq!(
            members.get(headless).map(|member| member.role.as_str()),
            Some("agent")
        );
    }

    // Repeat the same coordinator cycle. The second self-promotion has the same
    // logical inputs as the previous one, so a persisted mutation replay would
    // incorrectly skip the state change and leave the headless worker in charge.
    handle_comm_assign_role(
        4,
        coordinator.to_string(),
        headless.to_string(),
        "coordinator".to_string(),
        &client_tx,
        &sessions,
        &swarm_members,
        &swarms_by_id,
        &swarm_coordinators,
        &swarm_plans,
        &event_history,
        &event_counter,
        &swarm_event_tx,
        &mutation_runtime,
    )
    .await;
    assert_eq!(
        swarm_coordinators
            .read()
            .await
            .get(swarm_id)
            .map(String::as_str),
        Some(headless)
    );
    {
        let members = swarm_members.read().await;
        assert_eq!(
            members.get(coordinator).map(|member| member.role.as_str()),
            Some("agent")
        );
        assert_eq!(
            members.get(headless).map(|member| member.role.as_str()),
            Some("coordinator")
        );
    }

    handle_comm_assign_role(
        5,
        coordinator.to_string(),
        coordinator.to_string(),
        "coordinator".to_string(),
        &client_tx,
        &sessions,
        &swarm_members,
        &swarms_by_id,
        &swarm_coordinators,
        &swarm_plans,
        &event_history,
        &event_counter,
        &swarm_event_tx,
        &mutation_runtime,
    )
    .await;

    assert_eq!(
        swarm_coordinators
            .read()
            .await
            .get(swarm_id)
            .map(String::as_str),
        Some(coordinator)
    );
    {
        let members = swarm_members.read().await;
        assert_eq!(
            members.get(coordinator).map(|member| member.role.as_str()),
            Some("coordinator")
        );
        assert_eq!(
            members.get(headless).map(|member| member.role.as_str()),
            Some("agent")
        );
    }
}
