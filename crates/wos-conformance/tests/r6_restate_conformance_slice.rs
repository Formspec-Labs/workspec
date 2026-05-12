// Rust guideline compliant 2026-05-01

//! WS-094 Phase 4 / spec R-6.2 slice: Tier-3 conformance remains observable, and the
//! in-memory [`wos_server_runtime_restate::RestateRuntimeAdapter`] matches the shared
//! `restate_signature_fixture_runtime` reference for the same create + start + drain path.
//!
//! C.0 (configuration + active_tasks.len parity) and C.1 (full DrainOnceResult shape +
//! CaseInstance field parity) live here.

use std::sync::{Arc, Mutex};

use wos_core::instance::PendingEvent;
use wos_core::provenance::ProvenanceRecord;
use wos_core::typeid;
use wos_runtime::restate_signature_fixture_runtime;
use wos_runtime::runtime::CreateInstanceRequest;
use wos_runtime::{InMemoryStore, SharedInMemoryStore};
use wos_server_ports::runtime::{RuntimeOps, SeamAccess};
use wos_server_runtime_restate::RestateRuntimeAdapter;

fn signature_start_request(process_id: &str) -> CreateInstanceRequest {
    CreateInstanceRequest {
        definition_url: "urn:test:signature-runtime".into(),
        definition_version: "1.0.0".into(),
        instance_id: process_id.to_string(),
        tenant: None,
        initial_case_state: None,
    }
}

fn start_pending() -> PendingEvent {
    serde_json::from_value(serde_json::json!({
        "event": "start",
        "actorId": "system:test",
        "data": {},
        "timestamp": "2026-01-01T00:00:00Z"
    }))
    .expect("start pending event shape")
}

#[test]
fn r6_sig013_tier3_conformance_negative_still_observable() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let path = format!("{manifest}/tests/fixtures/SIG-013-policy-assurance-below-floor.json");
    let base = format!("{manifest}/tests/fixtures");
    let fixture_json =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read SIG-013 fixture: {e}"));
    match wos_conformance::run_fixture(&fixture_json, &base) {
        Ok(result) => {
            assert!(
                !result.passed,
                "SIG-013 should fail closed on low assurance"
            );
            assert!(
                result.failures.iter().any(|f| {
                    f.contains("emailOtp") || f.contains("email-otp") || f.contains("assurance")
                }),
                "expected policy-floor messaging, got {:?}",
                result.failures
            );
        }
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("emailOtp") || msg.contains("email-otp") || msg.contains("assurance"),
                "unexpected engine error: {msg}"
            );
        }
    }
}

#[tokio::test]
async fn r6_memory_parity_signature_start_drain_matches_reference_runtime() {
    let process_id = typeid::mint_process_id();
    let req = signature_start_request(&process_id);

    let shared = SharedInMemoryStore(Arc::new(Mutex::new(InMemoryStore::new())));
    let mut rt = restate_signature_fixture_runtime(shared.clone());
    rt.create_instance(req.clone()).expect("reference create");
    rt.enqueue_event(&process_id, start_pending())
        .expect("reference enqueue");
    rt.drain_until_idle(&process_id).expect("reference drain");
    let ref_inst = rt.load_instance(&process_id).expect("reference load");

    let adapter = RestateRuntimeAdapter::new();
    adapter.create_instance(req).await.expect("adapter create");
    adapter
        .enqueue_event(
            &process_id,
            serde_json::json!({
                "event": "start",
                "actorId": "system:test",
                "data": {},
                "timestamp": "2026-01-01T00:00:00Z"
            }),
        )
        .await
        .expect("adapter enqueue");
    adapter
        .drain_until_idle(&process_id)
        .await
        .expect("adapter drain");
    let ad_inst = adapter
        .load_instance(&process_id)
        .await
        .expect("adapter load");

    assert_eq!(
        ref_inst.configuration, ad_inst.configuration,
        "R-6.2: configuration after start+drain should match between reference WosRuntime and RestateRuntimeAdapter memory backend"
    );
    assert_eq!(
        ref_inst.active_tasks.len(),
        ad_inst.active_tasks.len(),
        "R-6.2: active task count should match after start+drain"
    );
}

/// **C.1** — full deterministic adapter parity: reference `WosRuntime` vs
/// `RestateRuntimeAdapter::new()` (memory backend).
///
/// Compares the complete `DrainOnceResult` shape for the start-event step
/// (`processed_event`, transition tuples, `created_task_ids`, `emitted_events`,
/// provenance kinds, guard evaluation count) plus final `CaseInstance` fields
/// (`configuration`, `active_tasks` IDs, `pending_events` empty).
#[tokio::test]
async fn r6_c1_full_drain_result_shape_parity() {
    let process_id = typeid::mint_process_id();
    let req = signature_start_request(&process_id);

    let shared = SharedInMemoryStore(Arc::new(Mutex::new(InMemoryStore::new())));
    let mut rt = restate_signature_fixture_runtime(shared.clone());
    rt.create_instance(req.clone()).expect("reference create");
    rt.enqueue_event(&process_id, start_pending())
        .expect("reference enqueue");
    let ref_steps = rt
        .drain_until_idle(&process_id)
        .expect("reference drain_until_idle");
    let ref_inst = rt.load_instance(&process_id).expect("reference load");

    let adapter = RestateRuntimeAdapter::new();
    adapter.create_instance(req).await.expect("adapter create");
    adapter
        .enqueue_event(
            &process_id,
            serde_json::json!({
                "event": "start",
                "actorId": "system:c1-parity",
                "data": {},
                "timestamp": "2026-01-01T00:00:00Z"
            }),
        )
        .await
        .expect("adapter enqueue");
    let ad_all = adapter
        .drain_until_idle(&process_id)
        .await
        .expect("adapter drain_until_idle");
    let ad_inst = adapter
        .load_instance(&process_id)
        .await
        .expect("adapter load");

    let idle_sentinel_count = ad_all
        .iter()
        .filter(|s| s.processed_event.is_none())
        .count();
    assert_eq!(
        idle_sentinel_count, 1,
        "C.1: Restate adapter must yield exactly one idle drainOnce sentinel from drain_until_idle"
    );
    assert!(
        ref_steps.iter().all(|s| s.processed_event.is_some()),
        "C.1: reference drain steps must all be non-idle in this slice"
    );

    // Reference drain_until_idle excludes the idle sentinel; adapter includes it.
    // Strip the trailing idle sentinel so the step arrays are comparable.
    let ad_steps: Vec<_> = ad_all
        .into_iter()
        .filter(|s| s.processed_event.is_some())
        .collect();

    assert_eq!(
        ref_steps.len(),
        ad_steps.len(),
        "C.1: non-idle step count must match (reference {}, adapter {})",
        ref_steps.len(),
        ad_steps.len()
    );
    assert!(
        !ref_steps.is_empty(),
        "C.1: at least one start step expected"
    );

    for i in 0..ref_steps.len() {
        let ref_step = &ref_steps[i];
        let ad_step = &ad_steps[i];

        assert_eq!(
            ref_step.processed_event, ad_step.processed_event,
            "C.1 step {i}: processed_event mismatch"
        );

        let ref_transitions: Vec<_> = ref_step
            .transitions
            .iter()
            .map(|t| (&t.from, &t.to, &t.event))
            .collect();
        let ad_transitions: Vec<_> = ad_step
            .transitions
            .iter()
            .map(|t| (&t.from, &t.to, &t.event))
            .collect();
        assert_eq!(
            ref_transitions, ad_transitions,
            "C.1 step {i}: transition tuples (from, to, event) must match"
        );

        let mut ref_task_ids = ref_step.created_task_ids.clone();
        let mut ad_task_ids = ad_step.created_task_ids.clone();
        ref_task_ids.sort();
        ad_task_ids.sort();
        assert_eq!(
            ref_task_ids, ad_task_ids,
            "C.1 step {i}: created_task_ids must match (order-independent)"
        );

        let mut ref_emitted = ref_step.emitted_events.clone();
        let mut ad_emitted = ad_step.emitted_events.clone();
        ref_emitted.sort();
        ad_emitted.sort();
        assert_eq!(
            ref_emitted, ad_emitted,
            "C.1 step {i}: emitted_events must match (order-independent)"
        );

        let ref_kinds: Vec<_> = ref_step.provenance.iter().map(|r| r.record_kind).collect();
        let ad_kinds: Vec<_> = ad_step.provenance.iter().map(|r| r.record_kind).collect();
        assert_eq!(
            ref_kinds, ad_kinds,
            "C.1 step {i}: provenance kinds must match in order"
        );

        assert_eq!(
            ref_step.guard_evaluations.len(),
            ad_step.guard_evaluations.len(),
            "C.1 step {i}: guard evaluation count must match"
        );
    }

    // Final CaseInstance parity
    assert_eq!(
        ref_inst.configuration, ad_inst.configuration,
        "C.1: configuration must match"
    );

    let mut ref_task_refs: Vec<_> = ref_inst
        .active_tasks
        .iter()
        .map(|t| t.task_id.clone())
        .collect();
    let mut ad_task_refs: Vec<_> = ad_inst
        .active_tasks
        .iter()
        .map(|t| t.task_id.clone())
        .collect();
    ref_task_refs.sort();
    ad_task_refs.sort();
    assert_eq!(
        ref_task_refs, ad_task_refs,
        "C.1: active_tasks IDs must match (order-independent)"
    );

    assert!(
        ref_inst.pending_events.is_empty(),
        "C.1: reference pending_events must be empty after drain"
    );
    assert!(
        ad_inst.pending_events.is_empty(),
        "C.1: adapter pending_events must be empty after drain"
    );
}

#[test]
fn r6_seam_access_signer_and_renderer_are_operational_noops() {
    let adapter = RestateRuntimeAdapter::new();
    let signer = <RestateRuntimeAdapter as SeamAccess>::signer(&adapter);
    let renderer = <RestateRuntimeAdapter as SeamAccess>::renderer(&adapter);

    let rec = ProvenanceRecord::state_transition("draft", "review", "start", Some("system"));
    assert!(signer.sign(&rec).expect("noop sign").is_empty());
    assert!(signer.verify(&rec, &[1, 2, 3]).expect("noop verify"));

    let out = renderer
        .render_explanation(&serde_json::json!({"ok": true}), "ignored")
        .expect("render");
    assert!(out.contains("ok"));
}
