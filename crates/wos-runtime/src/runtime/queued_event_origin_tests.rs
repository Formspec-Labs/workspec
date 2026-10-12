// Rust guideline compliant 2026-02-21

fn origin_kernel(states: serde_json::Value, initial: &str) -> KernelDocument {
    serde_json::from_value(serde_json::json!({
        "$wosWorkflow":"1.0", "url":"urn:test:queued-event-origin", "version":"1.0.0",
        "actors":[{"id":"worker","type":"human"}],
        "lifecycle":{"initialState":initial,"states":states}
    }))
    .unwrap()
}

fn origin_process(runtime: &mut WosRuntime, label: &str) -> String {
    let id = test_process_id(label).to_string();
    runtime
        .create_process(CreateProcessRequest {
            process_id: id.clone(),
            tenant: None,
            definition_url: "urn:test:queued-event-origin".into(),
            definition_version: "1.0.0".into(),
            initial_case_state: None,
        })
        .unwrap();
    id
}

fn origin_event(event: &str) -> PendingEvent {
    PendingEvent {
        event: event.into(),
        actor_id: Some("worker".into()),
        data: None,
        timestamp: String::new(),
        idempotency_token: Some("original-token".into()),
        origin: None,
    }
}

#[test]
fn queued_event_origin_retains_exact_transition_across_later_drain_and_restore() {
    let kernel = origin_kernel(
        serde_json::json!({
            "waiting":{"type":"atomic","transitions":[{"event":"record","target":"recorded","tags":["determination"]}]},
            "recorded":{"type":"atomic","onEntry":[{"action":"emitEvent","eventType":"notify"}],
                "transitions":[{"event":"notify","target":"notified"}]},
            "notified":{"type":"final"}
        }),
        "waiting",
    );
    let mut runtime = runtime_with_kernel(kernel);
    let id = origin_process(&mut runtime, "origin-delayed");
    runtime.enqueue_event(&id, origin_event("record")).unwrap();
    let first = runtime.drain_once(&id).unwrap();
    let before = runtime.store.load_record(&id).unwrap();
    let queued = &before.process.pending_events[0];
    let origin = queued
        .origin
        .as_ref()
        .expect("native emit has exact origin");
    let transition = before
        .provenance_log
        .iter()
        .find(|r| Some(&r.id) == origin.transition_record_id.as_ref())
        .unwrap();
    assert_eq!(transition.to_state.as_deref(), Some("recorded"));
    assert_eq!(
        transition.event.as_deref(),
        Some("wos.kernel.state_transition")
    );
    assert_eq!(
        transition.data.as_ref().unwrap()["transitionEvent"],
        "record"
    );
    assert!(
        transition
            .transition_tags
            .iter()
            .any(|tag| tag == "determination")
    );
    assert_eq!(queued.timestamp, transition.timestamp);
    let action = before
        .provenance_log
        .iter()
        .find(|r| r.id == origin.action_record_id)
        .unwrap();
    assert_eq!(action.record_kind, ProvenanceKind::ActionExecuted);
    assert_eq!(action.data.as_ref().unwrap()["eventType"], "notify");
    assert_eq!(
        first.processed_event_token.as_deref(),
        Some("original-token")
    );
    let restored: WorkflowProcess =
        serde_json::from_value(serde_json::to_value(&before.process).unwrap()).unwrap();
    assert_eq!(restored.pending_events[0].origin, queued.origin);
    let mut restored_record = before.clone();
    restored_record.process = restored;
    runtime.store.save_record(restored_record).unwrap();
    runtime.clock = Box::new(FixedClock {
        now_ms: 1_710_000_060_000,
    });
    let later = runtime.drain_once(&id).unwrap();
    let later_transition = later
        .provenance
        .iter()
        .find(|r| {
            r.to_state.as_deref() == Some("notified")
                && r.record_kind == ProvenanceKind::StateTransition
        })
        .unwrap();
    assert_ne!(transition.timestamp, later_transition.timestamp);
    assert_eq!(later.processed_event.as_deref(), Some("notify"));
    assert!(runtime.load_process(&id).unwrap().pending_events.is_empty());
}

#[test]
fn queued_event_origin_distinguishes_same_named_emissions_and_initial_entry() {
    let kernel = origin_kernel(
        serde_json::json!({
            "ready":{"type":"atomic","onEntry":[{"action":"emitEvent","eventType":"same"},{"action":"emitEvent","eventType":"same"}]}
        }),
        "ready",
    );
    let mut runtime = runtime_with_kernel(kernel);
    let id = origin_process(&mut runtime, "origin-initial");
    let record = runtime.store.load_record(&id).unwrap();
    let events = &record.process.pending_events;
    assert_eq!(events.len(), 2);
    let a = events[0].origin.as_ref().unwrap();
    let b = events[1].origin.as_ref().unwrap();
    assert_ne!(a.action_record_id, b.action_record_id);
    assert_eq!(a.transition_record_id, None);
    assert_eq!(b.transition_record_id, None);
    assert_eq!(events[0].event, "same");
    assert_eq!(events[1].event, "same");
}

#[test]
fn queued_event_origin_rejects_external_claim_before_any_queue_write() {
    let kernel = origin_kernel(serde_json::json!({"ready":{"type":"atomic"}}), "ready");
    let mut runtime = runtime_with_kernel(kernel);
    let id = origin_process(&mut runtime, "origin-forged");
    let before = serde_json::to_value(runtime.store.load_record(&id).unwrap().process).unwrap();
    let mut event = origin_event("anything");
    event.origin = Some(wos_core::instance::PendingEventOrigin {
        action_record_id: "forged".into(),
        transition_record_id: Some("foreign".into()),
    });
    assert!(runtime.enqueue_event(&id, event).is_err());
    assert_eq!(
        before,
        serde_json::to_value(runtime.store.load_record(&id).unwrap().process).unwrap()
    );
}

#[test]
fn queued_event_origin_old_serialized_event_keeps_absence() {
    let event: PendingEvent = serde_json::from_value(
        serde_json::json!({"event":"old","timestamp":"2026-10-10T00:00:00Z"}),
    )
    .unwrap();
    assert_eq!(event.origin, None);
    assert!(serde_json::to_value(event).unwrap().get("origin").is_none());
}

fn check_origin_actions(evaluator: &Evaluator, expected: &[(&str, &str)]) {
    let emitted: Vec<_> = evaluator
        .executed_actions()
        .iter()
        .filter(|a| a.action.action == wos_core::model::kernel::ActionKind::EmitEvent)
        .collect();
    assert_eq!(emitted.len(), expected.len());
    for (action, (event, target)) in emitted.iter().zip(expected) {
        assert_eq!(action.action.event_type.as_deref(), Some(*event));
        let transition = evaluator
            .provenance()
            .records()
            .iter()
            .find(|r| Some(&r.id) == action.transition_record_id.as_ref())
            .expect("exact transition binding");
        assert_eq!(transition.to_state.as_deref(), Some(*target));
    }
}

#[test]
fn queued_event_origin_nested_foreach_keeps_nearest_transition() {
    let kernel = origin_kernel(
        serde_json::json!({
            "ready":{"type":"atomic","transitions":[{"event":"go","target":"iteration"}]},
            "iteration":{"type":"foreach","collection":"[]","body":{"type":"atomic"},
                "onEntry":[{"action":"emitEvent","eventType":"outer"}],"transitions":[{"target":"done"}]},
            "done":{"type":"atomic","onEntry":[{"action":"emitEvent","eventType":"inner"}]}
        }),
        "ready",
    );
    let mut evaluator = Evaluator::new(kernel).unwrap();
    evaluator.process_event("go", Some("worker"), None).unwrap();
    check_origin_actions(&evaluator, &[("outer", "iteration"), ("inner", "done")]);
}

#[test]
fn queued_event_origin_parallel_cancellation_keeps_causing_transition() {
    let kernel = origin_kernel(
        serde_json::json!({
            "work":{"type":"parallel","cancellationPolicy":"cancel-siblings","regions":{
                "first":{"initialState":"a","states":{"a":{"type":"atomic","transitions":[{"event":"finish","target":"done"}]},
                    "done":{"type":"final"}}},
                "second":{"initialState":"b","states":{"b":{"type":"atomic","onExit":[{"action":"emitEvent","eventType":"cancelled"}]}}}
            }}
        }),
        "work",
    );
    let mut evaluator = Evaluator::new(kernel).unwrap();
    evaluator
        .process_event("finish", Some("worker"), None)
        .unwrap();
    check_origin_actions(&evaluator, &[("cancelled", "done")]);
}

#[test]
fn queued_event_origin_continuous_transition_binds_synthetic_event() {
    let kernel = origin_kernel(
        serde_json::json!({
            "ready":{"type":"atomic","transitions":[{"guard":"true","target":"done"}]},
            "done":{"type":"atomic","onEntry":[{"action":"emitEvent","eventType":"changed"}]}
        }),
        "ready",
    );
    let mut evaluator = Evaluator::new(kernel).unwrap();
    assert!(evaluator.rescan_on_mutation().unwrap());
    check_origin_actions(&evaluator, &[("changed", "done")]);
    let id = evaluator.executed_actions()[0]
        .transition_record_id
        .as_ref()
        .unwrap();
    let record = evaluator
        .provenance()
        .records()
        .iter()
        .find(|r| &r.id == id)
        .unwrap();
    assert_eq!(
        record.data.as_ref().unwrap()["transitionEvent"],
        "$postMutationRescan"
    );
}

#[test]
fn queued_event_origin_rejects_present_null_and_invalid_references() {
    for origin in [
        serde_json::json!(null),
        serde_json::json!({}),
        serde_json::json!({"actionRecordId":""}),
        serde_json::json!({"actionRecordId":"  "}),
        serde_json::json!({"actionRecordId":"action", "transitionRecordId":null}),
        serde_json::json!({"actionRecordId":"action", "transitionRecordId":""}),
        serde_json::json!({"actionRecordId":"action", "transitionRecordId":" \t"}),
        serde_json::json!({"actionRecordId":"action", "unknown":true}),
    ] {
        let value =
            serde_json::json!({"event":"event","timestamp":"2026-10-10T00:00:00Z","origin":origin});
        assert!(
            serde_json::from_value::<PendingEvent>(value.clone()).is_err(),
            "accepted invalid origin: {value}"
        );
    }
}
