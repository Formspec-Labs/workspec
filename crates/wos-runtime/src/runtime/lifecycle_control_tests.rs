// Rust guideline compliant 2026-02-21

// Kernel S11.3 / S11.5: lifecycle control, the status table and `processEvent`.

use super::*;
use wos_core::LifecycleControl;
use wos_core::instance::InstanceStatus;

const URL: &str = "urn:test:lifecycle-control";

fn kernel(version: &str, reason_required: bool) -> KernelDocument {
    serde_json::from_value(serde_json::json!({
        "$wosWorkflow": "1.0", "url": URL, "version": version,
        "actors": [{"id": "worker", "type": "human"}],
        "lifecycle": {
            "initialState": "open",
            "suspend": {"reasonRequired": reason_required},
            "terminate": {"reasonRequired": reason_required},
            "states": {
                "open": {"type": "atomic", "transitions": [{"event": "advance", "target": "review"}]},
                "review": {"type": "atomic", "transitions": [{"event": "close", "target": "done"}]},
                "done": {"type": "final"}
            }
        }
    }))
    .unwrap()
}

fn created(runtime: &mut WosRuntime, label: &str) -> String {
    let id = test_process_id(label).to_string();
    runtime
        .create_process(CreateProcessRequest {
            process_id: id.clone(),
            tenant: None,
            definition_url: URL.into(),
            definition_version: "1.0.0".into(),
            initial_case_state: Some(serde_json::json!({"x": 1})),
        })
        .unwrap();
    id
}

fn event(name: &str) -> PendingEvent {
    PendingEvent {
        event: name.into(),
        actor_id: Some("worker".into()),
        data: None,
        timestamp: String::new(),
        idempotency_token: Some(format!("token-{name}")),
        origin: None,
    }
}

fn status_conflict(result: Result<impl std::fmt::Debug, RuntimeError>) -> String {
    match result {
        Err(RuntimeError::InstanceStatusConflict { status, .. }) => status,
        other => panic!("expected a status conflict, got {other:?}"),
    }
}

#[test]
fn lifecycle_control_follows_the_status_table_and_records_each_operation() {
    let mut runtime = runtime_with_kernels(vec![kernel("1.0.0", false)]);
    let id = created(&mut runtime, "lc-table");
    assert_eq!(
        status_conflict(runtime.control_lifecycle(&id, LifecycleControl::Resume, "op", None)),
        "active"
    );

    let suspended = runtime
        .control_lifecycle(
            &id,
            LifecycleControl::Suspend,
            "actor:human:op",
            Some("hold for review"),
        )
        .unwrap();
    assert_eq!(suspended.status, InstanceStatus::Suspended);
    assert_eq!(
        status_conflict(runtime.control_lifecycle(&id, LifecycleControl::Suspend, "op", None)),
        "suspended"
    );

    assert_eq!(suspended.resumed_at, None);
    let resumed = runtime
        .control_lifecycle(&id, LifecycleControl::Resume, "actor:human:op", None)
        .unwrap();
    assert_eq!(resumed.status, InstanceStatus::Active);
    // The resume epoch a durable adapter keys post-resume deadline wakeups by.
    assert_eq!(
        resumed.resumed_at.as_deref(),
        Some(resumed.updated_at.as_str())
    );
    let terminated = runtime
        .control_lifecycle(&id, LifecycleControl::Terminate, "actor:human:op", None)
        .unwrap();
    assert_eq!(terminated.status, InstanceStatus::Terminated);
    for control in [
        LifecycleControl::Suspend,
        LifecycleControl::Resume,
        LifecycleControl::Terminate,
    ] {
        assert_eq!(
            status_conflict(runtime.control_lifecycle(&id, control, "op", None)),
            "terminated"
        );
    }

    let log = runtime.store.load_record(&id).unwrap().provenance_log;
    let lifecycle: Vec<_> = log
        .iter()
        .filter(|record| {
            matches!(
                record.record_kind,
                ProvenanceKind::InstanceSuspended
                    | ProvenanceKind::InstanceResumed
                    | ProvenanceKind::InstanceTerminated
            )
        })
        .collect();
    assert_eq!(
        lifecycle
            .iter()
            .map(|record| record.record_kind)
            .collect::<Vec<_>>(),
        [
            ProvenanceKind::InstanceSuspended,
            ProvenanceKind::InstanceResumed,
            ProvenanceKind::InstanceTerminated
        ]
    );
    assert!(lifecycle.iter().all(
        |record| record.actor_id.as_deref() == Some("actor:human:op")
            && record.audit_layer.as_deref() == Some("facts")
            && !record.timestamp.is_empty()
    ));
    assert_eq!(
        lifecycle[0].data,
        Some(serde_json::json!({"reason": "hold for review"}))
    );
    assert_eq!(lifecycle[1].data, None);
}

#[test]
fn a_workflow_that_requires_a_reason_refuses_suspend_and_terminate_without_one() {
    let mut runtime = runtime_with_kernels(vec![kernel("1.0.0", true)]);
    let id = created(&mut runtime, "lc-reason");
    for (control, blank) in [
        (LifecycleControl::Suspend, None),
        (LifecycleControl::Terminate, Some("  ")),
    ] {
        assert!(matches!(
            runtime.control_lifecycle(&id, control, "op", blank),
            Err(RuntimeError::LifecycleReasonRequired(_))
        ));
    }
    let before = runtime.store.load_record(&id).unwrap();
    assert_eq!(before.process.status, InstanceStatus::Active);
    runtime
        .control_lifecycle(&id, LifecycleControl::Suspend, "op", Some("audit"))
        .unwrap();
    // Resume carries no workflow policy (Kernel S11.3 input "(none)").
    runtime
        .control_lifecycle(&id, LifecycleControl::Resume, "op", None)
        .unwrap();
}

#[test]
fn a_suspended_instance_queues_but_evaluates_nothing_until_resumed() {
    let mut runtime = runtime_with_kernels(vec![kernel("1.0.0", false)]);
    let id = created(&mut runtime, "lc-queue");
    runtime
        .control_lifecycle(&id, LifecycleControl::Suspend, "op", None)
        .unwrap();
    runtime.enqueue_event(&id, event("advance")).unwrap();
    assert_eq!(status_conflict(runtime.drain_once(&id)), "suspended");
    assert_eq!(
        status_conflict(runtime.process_event(&id, event("advance"))),
        "suspended"
    );
    assert_eq!(
        status_conflict(runtime.migrate(&id, "1.0.0", MigrationMap::default(), None)),
        "suspended"
    );
    let held = runtime.load_process(&id).unwrap();
    assert_eq!(held.configuration, vec!["open".to_string()]);
    assert_eq!(held.pending_events.len(), 1);

    runtime
        .control_lifecycle(&id, LifecycleControl::Resume, "op", None)
        .unwrap();
    let step = runtime.drain_once(&id).unwrap();
    assert_eq!(step.processed_event.as_deref(), Some("advance"));
    assert_eq!(
        runtime.load_process(&id).unwrap().configuration,
        vec!["review".to_string()]
    );
}

#[test]
fn process_event_evaluates_only_the_submitted_event_of_an_idle_active_instance() {
    let mut runtime = runtime_with_kernels(vec![kernel("1.0.0", false)]);
    let id = created(&mut runtime, "lc-process");
    let step = runtime.process_event(&id, event("advance")).unwrap();
    assert_eq!(step.processed_event.as_deref(), Some("advance"));
    assert_eq!(step.processed_event_token.as_deref(), Some("token-advance"));
    assert_eq!(
        runtime.load_process(&id).unwrap().configuration,
        vec!["review".to_string()]
    );

    // Queued work keeps its own continuation: an inline evaluation would take its turn.
    runtime.enqueue_event(&id, event("close")).unwrap();
    assert_eq!(
        status_conflict(runtime.process_event(&id, event("close"))),
        "busy"
    );
    assert_eq!(runtime.load_process(&id).unwrap().pending_events.len(), 1);

    runtime.drain_once(&id).unwrap();
    assert_eq!(
        runtime.load_process(&id).unwrap().configuration,
        vec!["done".to_string()]
    );
    runtime
        .control_lifecycle(&id, LifecycleControl::Terminate, "op", None)
        .unwrap();
    assert_eq!(
        status_conflict(runtime.process_event(&id, event("advance"))),
        "terminated"
    );
    assert_eq!(
        status_conflict(runtime.enqueue_event(&id, event("advance"))),
        "terminated"
    );
}

#[test]
fn migration_refuses_a_final_instance_and_keeps_its_version() {
    let mut runtime = runtime_with_kernels(vec![kernel("1.0.0", false), kernel("1.1.0", false)]);
    let id = created(&mut runtime, "lc-migrate");
    runtime
        .control_lifecycle(&id, LifecycleControl::Terminate, "op", None)
        .unwrap();
    assert_eq!(
        status_conflict(runtime.migrate(&id, "1.1.0", MigrationMap::default(), Some("op"))),
        "terminated"
    );
    assert_eq!(
        runtime.load_process(&id).unwrap().definition_version,
        "1.0.0"
    );
}

#[test]
fn migration_waits_for_queued_events_to_drain() {
    let mut runtime = runtime_with_kernels(vec![kernel("1.0.0", false), kernel("1.1.0", false)]);
    let id = created(&mut runtime, "lc-migrate-busy");
    runtime.enqueue_event(&id, event("advance")).unwrap();
    assert_eq!(
        status_conflict(runtime.migrate(&id, "1.1.0", MigrationMap::default(), Some("op"))),
        "busy"
    );
    assert_eq!(
        runtime.load_process(&id).unwrap().definition_version,
        "1.0.0"
    );
    runtime.drain_once(&id).unwrap();
    runtime
        .migrate(&id, "1.1.0", MigrationMap::default(), Some("op"))
        .unwrap();
    assert_eq!(
        runtime.load_process(&id).unwrap().definition_version,
        "1.1.0"
    );
}
