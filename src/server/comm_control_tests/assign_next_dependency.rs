#[tokio::test]
async fn assign_next_prefers_worker_with_dependency_context() {
    let (_env, _runtime) = RuntimeEnvGuard::new();
    let swarm_id = "swarm-context-score";
    let requester = "coord";
    let context_worker = "worker-context";
    let other_worker = "worker-other";
    let (client_tx, mut client_rx) = mpsc::unbounded_channel();
    let sessions = Arc::new(RwLock::new(HashMap::new()));
    let soft_interrupt_queues = Arc::new(RwLock::new(HashMap::new()));
    let client_connections = Arc::new(RwLock::new(HashMap::new()));
    let swarm_members = Arc::new(RwLock::new(HashMap::from([
        (requester.to_string(), {
            let mut member = member(requester, swarm_id, "ready");
            member.role = "coordinator".to_string();
            member
        }),
        (
            context_worker.to_string(),
            member(context_worker, swarm_id, "ready"),
        ),
        (
            other_worker.to_string(),
            member(other_worker, swarm_id, "ready"),
        ),
    ])));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        HashSet::from([
            requester.to_string(),
            context_worker.to_string(),
            other_worker.to_string(),
        ]),
    )])));
    let mut dependency = plan_item("dep", "completed", "high", &[]);
    dependency.assigned_to = Some(context_worker.to_string());
    let swarm_plans = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        VersionedPlan {
            items: vec![dependency, plan_item("next", "queued", "high", &["dep"])],
            version: 1,
            participants: HashSet::from([
                requester.to_string(),
                context_worker.to_string(),
                other_worker.to_string(),
            ]),
            task_progress: HashMap::new(),
        },
    )])));
    let swarm_coordinators = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        requester.to_string(),
    )])));
    let event_history = Arc::new(RwLock::new(VecDeque::new()));
    let event_counter = Arc::new(AtomicU64::new(1));
    let (swarm_event_tx, _swarm_event_rx) = broadcast::channel(32);
    let mutation_runtime = SwarmMutationRuntime::default();
    let provider: Arc<dyn Provider> = Arc::new(TestProvider);
    let global_session_id = Arc::new(RwLock::new(String::new()));
    let mcp_pool = Arc::new(crate::mcp::SharedMcpPool::from_default_config());

    handle_comm_assign_next(
        102,
        requester.to_string(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        &client_tx,
        &sessions,
        &global_session_id,
        &provider,
        &soft_interrupt_queues,
        &client_connections,
        &swarm_members,
        &swarms_by_id,
        &swarm_plans,
        &swarm_coordinators,
        &event_history,
        &event_counter,
        &swarm_event_tx,
        &mcp_pool,
        &mutation_runtime,
    )
    .await;

    match client_rx.recv().await.expect("response") {
        ServerEvent::CommAssignTaskResponse {
            id,
            task_id,
            target_session,
        } => {
            assert_eq!(id, 102);
            assert_eq!(task_id, "next");
            assert_eq!(target_session, context_worker);
        }
        other => panic!("expected CommAssignTaskResponse, got {other:?}"),
    }
}

#[tokio::test]
async fn assign_next_operation_id_replays_original_assignment_without_advancing_plan() {
    let (_env, _runtime) = RuntimeEnvGuard::new();
    let swarm_id = "swarm-assign-next-op";
    let requester = "coord";
    let worker = "worker-ready";
    let (client_tx, mut client_rx) = mpsc::unbounded_channel();
    let sessions = Arc::new(RwLock::new(HashMap::new()));
    let soft_interrupt_queues = Arc::new(RwLock::new(HashMap::new()));
    let client_connections = Arc::new(RwLock::new(HashMap::new()));
    let swarm_members = Arc::new(RwLock::new(HashMap::from([
        (requester.to_string(), {
            let mut member = member(requester, swarm_id, "ready");
            member.role = "coordinator".to_string();
            member
        }),
        (worker.to_string(), member(worker, swarm_id, "ready")),
    ])));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        HashSet::from([requester.to_string(), worker.to_string()]),
    )])));
    let swarm_plans = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        VersionedPlan {
            items: vec![
                plan_item("task-one", "queued", "high", &[]),
                plan_item("task-two", "queued", "medium", &[]),
            ],
            version: 1,
            participants: HashSet::from([requester.to_string(), worker.to_string()]),
            task_progress: HashMap::new(),
        },
    )])));
    let swarm_coordinators = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        requester.to_string(),
    )])));
    let event_history = Arc::new(RwLock::new(VecDeque::new()));
    let event_counter = Arc::new(AtomicU64::new(1));
    let (swarm_event_tx, _swarm_event_rx) = broadcast::channel(32);
    let mutation_runtime = SwarmMutationRuntime::default();
    let provider: Arc<dyn Provider> = Arc::new(TestProvider);
    let global_session_id = Arc::new(RwLock::new(String::new()));
    let mcp_pool = Arc::new(crate::mcp::SharedMcpPool::from_default_config());

    handle_comm_assign_next(
        201,
        requester.to_string(),
        None,
        None,
        None,
        None,
        None,
        Some("op:assign-next-retry".to_string()),
        None,
        &client_tx,
        &sessions,
        &global_session_id,
        &provider,
        &soft_interrupt_queues,
        &client_connections,
        &swarm_members,
        &swarms_by_id,
        &swarm_plans,
        &swarm_coordinators,
        &event_history,
        &event_counter,
        &swarm_event_tx,
        &mcp_pool,
        &mutation_runtime,
    )
    .await;

    match client_rx.recv().await.expect("first response") {
        ServerEvent::CommAssignTaskResponse {
            id,
            task_id,
            target_session,
        } => {
            assert_eq!(id, 201);
            assert_eq!(task_id, "task-one");
            assert_eq!(target_session, worker);
        }
        other => panic!("expected first CommAssignTaskResponse, got {other:?}"),
    }

    handle_comm_assign_next(
        202,
        requester.to_string(),
        None,
        None,
        None,
        None,
        None,
        Some("op:assign-next-retry".to_string()),
        None,
        &client_tx,
        &sessions,
        &global_session_id,
        &provider,
        &soft_interrupt_queues,
        &client_connections,
        &swarm_members,
        &swarms_by_id,
        &swarm_plans,
        &swarm_coordinators,
        &event_history,
        &event_counter,
        &swarm_event_tx,
        &mcp_pool,
        &mutation_runtime,
    )
    .await;

    match client_rx.recv().await.expect("replay response") {
        ServerEvent::CommAssignTaskResponse {
            id,
            task_id,
            target_session,
        } => {
            assert_eq!(id, 202);
            assert_eq!(task_id, "task-one");
            assert_eq!(target_session, worker);
        }
        other => panic!("expected replay CommAssignTaskResponse, got {other:?}"),
    }

    let plans = swarm_plans.read().await;
    let plan = plans.get(swarm_id).expect("plan should exist");
    let task_two = plan
        .items
        .iter()
        .find(|item| item.id == "task-two")
        .expect("task two should remain");
    assert_eq!(task_two.status, "queued");
    assert_eq!(task_two.assigned_to, None);
}

#[tokio::test]
async fn fill_slots_operation_id_slot_nonces_replay_without_filling_extra_tasks() {
    let (_env, _runtime) = RuntimeEnvGuard::new();
    let swarm_id = "swarm-fill-slots-op";
    let requester = "coord";
    let worker_a = "worker-a";
    let worker_b = "worker-b";
    let (client_tx, mut client_rx) = mpsc::unbounded_channel();
    let sessions = Arc::new(RwLock::new(HashMap::new()));
    let soft_interrupt_queues = Arc::new(RwLock::new(HashMap::new()));
    let client_connections = Arc::new(RwLock::new(HashMap::new()));
    let swarm_members = Arc::new(RwLock::new(HashMap::from([
        (requester.to_string(), {
            let mut member = member(requester, swarm_id, "ready");
            member.role = "coordinator".to_string();
            member
        }),
        (worker_a.to_string(), member(worker_a, swarm_id, "ready")),
        (worker_b.to_string(), member(worker_b, swarm_id, "ready")),
    ])));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        HashSet::from([
            requester.to_string(),
            worker_a.to_string(),
            worker_b.to_string(),
        ]),
    )])));
    let swarm_plans = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        VersionedPlan {
            items: vec![
                plan_item("task-one", "queued", "high", &[]),
                plan_item("task-two", "queued", "medium", &[]),
                plan_item("task-three", "queued", "low", &[]),
            ],
            version: 1,
            participants: HashSet::from([
                requester.to_string(),
                worker_a.to_string(),
                worker_b.to_string(),
            ]),
            task_progress: HashMap::new(),
        },
    )])));
    let swarm_coordinators = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        requester.to_string(),
    )])));
    let event_history = Arc::new(RwLock::new(VecDeque::new()));
    let event_counter = Arc::new(AtomicU64::new(1));
    let (swarm_event_tx, _swarm_event_rx) = broadcast::channel(32);
    let mutation_runtime = SwarmMutationRuntime::default();
    let provider: Arc<dyn Provider> = Arc::new(TestProvider);
    let global_session_id = Arc::new(RwLock::new(String::new()));
    let mcp_pool = Arc::new(crate::mcp::SharedMcpPool::from_default_config());

    for (id, nonce) in [
        (301, "op:fill-slots-retry:slot:0"),
        (302, "op:fill-slots-retry:slot:1"),
    ] {
        handle_comm_assign_next(
            id,
            requester.to_string(),
            None,
            None,
            None,
            None,
            None,
            Some(nonce.to_string()),
            Some("run:op:fill-slots-retry".to_string()),
            &client_tx,
            &sessions,
            &global_session_id,
            &provider,
            &soft_interrupt_queues,
            &client_connections,
            &swarm_members,
            &swarms_by_id,
            &swarm_plans,
            &swarm_coordinators,
            &event_history,
            &event_counter,
            &swarm_event_tx,
            &mcp_pool,
            &mutation_runtime,
        )
        .await;
    }

    let mut original_assignments = Vec::new();
    for expected_id in [301, 302] {
        match client_rx.recv().await.expect("initial fill_slots response") {
            ServerEvent::CommAssignTaskResponse {
                id,
                task_id,
                target_session,
            } => {
                assert_eq!(id, expected_id);
                original_assignments.push((task_id, target_session));
            }
            other => panic!("expected initial CommAssignTaskResponse, got {other:?}"),
        }
    }
    assert_eq!(original_assignments[0].0, "task-one");
    assert_eq!(original_assignments[1].0, "task-two");

    for (id, nonce) in [
        (303, "op:fill-slots-retry:slot:0"),
        (304, "op:fill-slots-retry:slot:1"),
    ] {
        handle_comm_assign_next(
            id,
            requester.to_string(),
            None,
            None,
            None,
            None,
            None,
            Some(nonce.to_string()),
            Some("run:op:fill-slots-retry".to_string()),
            &client_tx,
            &sessions,
            &global_session_id,
            &provider,
            &soft_interrupt_queues,
            &client_connections,
            &swarm_members,
            &swarms_by_id,
            &swarm_plans,
            &swarm_coordinators,
            &event_history,
            &event_counter,
            &swarm_event_tx,
            &mcp_pool,
            &mutation_runtime,
        )
        .await;
    }

    for (expected_id, expected_assignment) in [
        (303, original_assignments[0].clone()),
        (304, original_assignments[1].clone()),
    ] {
        match client_rx.recv().await.expect("replayed fill_slots response") {
            ServerEvent::CommAssignTaskResponse {
                id,
                task_id,
                target_session,
            } => {
                assert_eq!(id, expected_id);
                assert_eq!((task_id, target_session), expected_assignment);
            }
            other => panic!("expected replay CommAssignTaskResponse, got {other:?}"),
        }
    }

    let plans = swarm_plans.read().await;
    let plan = plans.get(swarm_id).expect("plan should exist");
    let task_three = plan
        .items
        .iter()
        .find(|item| item.id == "task-three")
        .expect("task three should remain");
    assert_eq!(task_three.status, "queued");
    assert_eq!(task_three.assigned_to, None);
}
