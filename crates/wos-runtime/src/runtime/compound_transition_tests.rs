// Rust guideline compliant 2026-02-21
// Durable leaf configuration and actual compound entry/exit behavior.
use super::*;

fn kernel() -> KernelDocument {
    serde_json::from_value(serde_json::json!({"$wosWorkflow":"1.0","url":"urn:test:compound-runtime","version":"1.0.0",
        "lifecycle":{"initialState":"phaseA","states":{
            "phaseA":{"type":"compound","initialState":"a","onEntry":[{"action":"startTimer","timerId":"parent-timer","duration":"P30D","event":"expired"}],"onExit":[{"action":"cancelTimer","timerId":"parent-timer"}],"states":{
                "a":{"type":"atomic","transitions":[{"event":"repeat","target":"a"},{"event":"go","target":"b2"}]}}},
            "phaseB":{"type":"compound","initialState":"wrong","onEntry":[{"action":"log","message":"entered B"}],"onExit":[{"action":"log","message":"exited B"}],"states":{
                "wrong":{"type":"atomic","onEntry":[{"action":"log","message":"wrong default"}]},
                "b2":{"type":"atomic","transitions":[{"event":"finish","target":"done"}]}}},
            "done":{"type":"final"}
        }}})).unwrap()
}
fn event(runtime: &mut WosRuntime, id: &str, event: &str) {
    runtime
        .enqueue_event(
            id,
            PendingEvent {
                origin: None,
                event: event.into(),
                actor_id: None,
                data: None,
                timestamp: String::new(),
                idempotency_token: None,
            },
        )
        .unwrap();
    runtime.drain_until_idle(id).unwrap();
}
fn reload(record: RuntimeRecord, kernel: KernelDocument) -> WosRuntime {
    let mut store = InMemoryStore::new();
    store.restore_record(record).unwrap();
    runtime_with_store_kernels(store, vec![kernel])
}
fn entries(record: &RuntimeRecord, state: &str) -> usize {
    record
        .provenance_log
        .iter()
        .filter(|p| {
            p.record_kind == ProvenanceKind::OnEntry
                && p.data.as_ref().is_some_and(|data| {
                    data.get("state").and_then(serde_json::Value::as_str) == Some(state)
                })
        })
        .count()
}
#[test]
fn compound_paths_and_parent_timer_survive_actual_runtime_reloads_without_reentry() {
    let definition = kernel();
    let mut runtime = runtime_with_kernel(definition.clone());
    let id = stack_common_typeid::mint_process_id();
    runtime
        .create_process(CreateProcessRequest {
            process_id: id.clone(),
            tenant: None,
            definition_url: definition.url.clone().unwrap(),
            definition_version: "1.0.0".into(),
            initial_case_state: None,
        })
        .unwrap();
    let initial = runtime.store.load_record(&id).unwrap();
    assert_eq!(initial.process.configuration, ["a"]);
    assert_eq!(entries(&initial, "phaseA"), 1);
    let original_timer = serde_json::to_value(&initial.process.timers).unwrap();
    let mut runtime = reload(initial, definition.clone());
    event(&mut runtime, &id, "repeat");
    let repeated = runtime.store.load_record(&id).unwrap();
    assert_eq!(repeated.process.configuration, ["a"]);
    assert_eq!(entries(&repeated, "phaseA"), 1);
    assert_eq!(
        serde_json::to_value(&repeated.process.timers).unwrap(),
        original_timer
    );
    event(&mut runtime, &id, "go");
    let entered = runtime.store.load_record(&id).unwrap();
    assert_eq!(entered.process.configuration, ["b2"]);
    assert_eq!(entries(&entered, "phaseB"), 1);
    assert_eq!(entries(&entered, "wrong"), 0);
    assert!(
        entered.process.timers.is_empty(),
        "authored parent cancelTimer must run on full exit path"
    );
    let mut runtime = reload(entered, definition);
    event(&mut runtime, &id, "finish");
    let final_record = runtime.store.load_record(&id).unwrap();
    assert_eq!(final_record.process.configuration, ["done"]);
    assert_eq!(entries(&final_record, "phaseA"), 1);
    assert_eq!(entries(&final_record, "phaseB"), 1);
    // Process-status finalization is a separate preexisting runtime boundary;
    // this control verifies the repaired configuration and retained hook behavior.
}
