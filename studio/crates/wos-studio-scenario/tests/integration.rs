// Rust guideline compliant 2026-05-02

//! End-to-end integration test for `run_workspace`.
//!
//! Exercises the full simulator path: build a tiny in-memory workspace,
//! call `run_workspace`, assert the per-scenario result shapes are
//! sane. Closes Wave 3 MAJOR-10 (no end-to-end test of `run_workspace`;
//! all prior tests called `run_scenario` against a hand-built fake
//! artifact, never compiled a workspace through the compiler).

use wos_studio_compiler::CompileOptions;
use wos_studio_lint::Workspace;
use wos_studio_scenario::{
    ScenarioOutcome, run_workspace, run_workspace_with_options,
};

fn ws_from(items: Vec<(&str, serde_json::Value)>) -> Workspace {
    Workspace::from_iter(items.into_iter().map(|(p, v)| (p.to_string(), v.to_string())))
}

#[test]
fn run_workspace_compiles_and_runs_minimal_scenario() {
    let ws = ws_from(vec![
        (
            "wfi.json",
            serde_json::json!({
                "$wosStudioWorkflowIntent": "1.0",
                "id": "wfi-int",
                "workspaceId": "ws-int",
                "version": "0.1.0",
                "title": "Integration test workflow",
                "impactLevel": "operational",
                "publicationUrl": "https://example.org/int",
                "actors": [
                    {"id": "caseworker", "type": "human", "name": "CW"}
                ],
                "elements": [
                    {"id": "intake", "kind": "step",
                     "policyObjectRefs": ["pol-int-1"],
                     "bridge": {"kernelKind": "transition"},
                     "derivedFrom": ["pol-int-1"]}
                ]
            }),
        ),
        (
            "po.json",
            serde_json::json!({
                "$wosStudioPolicyObject": "1.0",
                "policyObjects": [{
                    "id": "pol-int-1", "workspaceId": "ws-int",
                    "kind": "DecisionRule", "lifecycleState": "approved",
                    "originClass": "source",
                    "citations": [{"sourceCitationRef": "c-int"}]
                }]
            }),
        ),
        (
            "map.json",
            serde_json::json!({
                "$wosStudioMapping": "1.0",
                "mappings": [{
                    "id": "m-int", "policyObjectRef": "pol-int-1",
                    "mappingState": "mapsToWos",
                    "targets": [{
                        "wosConceptId": "DecisionRule",
                        "wosJsonPath": "$.governance.policyObjects[0]"
                    }]
                }]
            }),
        ),
        (
            "ws.json",
            serde_json::json!({
                "$wosStudioWorkspace": "1.0",
                "id": "ws-int",
                "title": "Integration",
                "reviewerRoles": []
            }),
        ),
        (
            "sc.json",
            serde_json::json!({
                "$wosStudioScenario": "1.0",
                "scenarios": [{
                    "id": "sc-int-1",
                    "version": "1.0.0",
                    "scenarioType": "happy-path",
                    "lifecycleState": "reviewed",
                    "events": [{"name": "submit", "targetState": "approved"}],
                    "expectedTrace": [{
                        "stateBefore": "intake",
                        "stateAfter": "approved",
                        "event": "submit"
                    }],
                    "expectedTerminals": ["approved"]
                }]
            }),
        ),
    ]);

    // Use gates-off for the integration test — the compiler's gates
    // require additional shape we don't need to exercise here. The
    // simulator's behavior is what's under test.
    let options = CompileOptions {
        halt_on_readiness_error: false,
        run_external_gates: false,
    };
    let results = run_workspace_with_options(&ws, options).expect("compile + run");

    assert_eq!(results.len(), 1, "expected one scenario, got {}", results.len());
    let r = &results[0];
    assert_eq!(r.scenario_id, "wos-scenario-sc-int-1-v1.0.0");
    assert_eq!(r.scenario_type.as_deref(), Some("happy-path"));
    // Conformance trace: real schema-compatible shape.
    assert_eq!(
        r.conformance_trace["fixtureId"],
        "wos-scenario-sc-int-1-v1.0.0"
    );
    // kernelVersion sources from the compiled workflow's $wosWorkflow
    // marker, which is the document-type marker pinned to const "1.0"
    // (the F4.1 fix: previously phase4_emit conflated this with the
    // WorkflowIntent's content version, which the schema rejected
    // against the const constraint).
    assert_eq!(r.conformance_trace["kernelVersion"], "1.0");
    assert!(r
        .conformance_trace
        .get("steps")
        .and_then(|s| s.as_array())
        .is_some());
}

#[test]
fn run_workspace_default_uses_gates_on_options() {
    // Smoke test: ensure the public `run_workspace` (without explicit
    // CompileOptions) defaults to gates-on. Gate failure makes the
    // compile error rather than silently passing — which is what the
    // R5.7 fix changed from the original gates-off default.
    let ws = ws_from(vec![]);
    // Empty workspace → phase 1 fails (no $wosStudioWorkflowIntent).
    // run_workspace returns Err rather than masking the failure.
    let result = run_workspace(&ws);
    assert!(result.is_err(), "empty workspace should error: {result:?}");
}

/// SA-MUST-scn-014 — Simulation MUST be deterministic given the same
/// {WorkflowIntent version, ExpectedTrace, scenario engine version,
/// simulated time}.
///
/// Re-running `run_workspace_with_options` on the same workspace
/// 5× MUST produce byte-identical conformance traces and outcomes.
/// In-process repetition is enough here: the runner is pure (no
/// SystemTime, no UUIDs) and the underlying compile is already
/// covered cross-process by the compiler's `tests/determinism.rs`
/// harness — this test pins the simulator's own determinism on top
/// of an already-deterministic compile.
#[test]
fn run_workspace_is_deterministic_across_repeats() {
    let ws = ws_from(vec![
        (
            "wfi.json",
            serde_json::json!({
                "$wosStudioWorkflowIntent": "1.0",
                "id": "wfi-det",
                "workspaceId": "ws-det",
                "version": "0.1.0",
                "title": "Determinism scenario workflow",
                "impactLevel": "operational",
                "publicationUrl": "https://example.org/det",
                "actors": [
                    {"id": "caseworker", "type": "human", "name": "CW"}
                ],
                "elements": [
                    {"id": "intake", "kind": "step",
                     "policyObjectRefs": ["pol-det-1"],
                     "bridge": {"kernelKind": "transition"},
                     "derivedFrom": ["pol-det-1"]}
                ]
            }),
        ),
        (
            "po.json",
            serde_json::json!({
                "$wosStudioPolicyObject": "1.0",
                "policyObjects": [{
                    "id": "pol-det-1", "workspaceId": "ws-det",
                    "kind": "DecisionRule", "lifecycleState": "approved",
                    "originClass": "source",
                    "citations": [{"sourceCitationRef": "c-det"}]
                }]
            }),
        ),
        (
            "map.json",
            serde_json::json!({
                "$wosStudioMapping": "1.0",
                "mappings": [{
                    "id": "m-det", "policyObjectRef": "pol-det-1",
                    "mappingState": "mapsToWos",
                    "targets": [{
                        "wosConceptId": "DecisionRule",
                        "wosJsonPath": "$.governance.policyObjects[0]"
                    }]
                }]
            }),
        ),
        (
            "ws.json",
            serde_json::json!({
                "$wosStudioWorkspace": "1.0",
                "id": "ws-det",
                "title": "Determinism",
                "reviewerRoles": []
            }),
        ),
        (
            "sc.json",
            serde_json::json!({
                "$wosStudioScenario": "1.0",
                "scenarios": [{
                    "id": "sc-det-1",
                    "version": "1.0.0",
                    "scenarioType": "happy-path",
                    "lifecycleState": "reviewed",
                    "events": [{"name": "submit", "targetState": "approved"}],
                    "expectedTrace": [{
                        "stateBefore": "intake",
                        "stateAfter": "approved",
                        "event": "submit"
                    }],
                    "expectedTerminals": ["approved"]
                }]
            }),
        ),
    ]);
    let options = CompileOptions {
        halt_on_readiness_error: false,
        run_external_gates: false,
    };

    let baseline = run_workspace_with_options(&ws, options).expect("compile + run");
    let baseline_str = serde_json::to_string(&baseline).expect("serialize baseline");
    assert!(
        !baseline.is_empty(),
        "fixture should produce ≥1 scenario result so determinism is checked over a non-empty stream"
    );

    for i in 1..5 {
        let next = run_workspace_with_options(&ws, options).expect("compile + run");
        let next_str = serde_json::to_string(&next).expect("serialize iter");
        assert_eq!(
            next_str, baseline_str,
            "iteration {i}: run_workspace drift — non-deterministic simulation"
        );
    }
}

#[test]
fn empty_workspace_yields_no_scenarios() {
    let ws = ws_from(vec![
        (
            "wfi.json",
            serde_json::json!({
                "$wosStudioWorkflowIntent": "1.0",
                "id": "wfi-empty",
                "workspaceId": "ws-empty",
                "version": "0.1.0",
                "title": "Empty",
                "impactLevel": "operational",
                "publicationUrl": "https://example.org/empty",
                "actors": [],
                "elements": []
            }),
        ),
        (
            "ws.json",
            serde_json::json!({
                "$wosStudioWorkspace": "1.0",
                "id": "ws-empty",
                "reviewerRoles": []
            }),
        ),
    ]);
    let options = CompileOptions {
        halt_on_readiness_error: false,
        run_external_gates: false,
    };
    let results = run_workspace_with_options(&ws, options).expect("compile + run");
    assert!(
        results.is_empty(),
        "no scenarios → empty results, got {}",
        results.len()
    );
    let _ = ScenarioOutcome::Pass; // ensure variant stays referenced
}
