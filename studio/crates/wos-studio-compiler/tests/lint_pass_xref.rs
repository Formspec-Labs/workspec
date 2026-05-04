//! Cross-pass companion to the `phase7_gates::schema_pass_silent_on_*` and
//! `schema_pass_does_not_yet_catch_*` sentinels.
//!
//! The schema-pass sentinels over there assert the **current** behavior of
//! the schema (silent on cross-property gaps that JSON Schema Draft 2020-12
//! cannot natively express; the remaining open one is STUDIO-DEFER-003
//! Tranche C). This file asserts that the **lint** pass independently
//! catches the same shapes — layered defense.
//!
//! Inputs are minimal-clean envelopes (boon-valid except for the deliberate
//! cross-property violation).

use serde_json::{json, Value};
use wos_lint::studio_api::{lint_workflow_with_project, LintSeverity};

fn minimal_valid_envelope() -> Value {
    json!({
        "$wosWorkflow": "1.0",
        "url": "https://example.org/wf-1",
        "version": "1.0.0",
        "title": "Test",
        "impactLevel": "operational",
        "actors": [
            {"id": "actor-1", "type": "human", "name": "Test actor"}
        ],
        "lifecycle": {
            "initialState": "intake",
            "states": {
                "intake": {"type": "atomic", "transitions": []}
            }
        }
    })
}

#[test]
fn lint_catches_actor_id_collision() {
    // Mirror of phase7_gates::schema_pass_does_not_yet_catch_actor_id_collision.
    // K-009 is the parent-tier rule that catches duplicate actor ids.
    let mut wf = minimal_valid_envelope();
    wf["actors"] = json!([
        {"id": "a1", "type": "human", "name": "X"},
        {"id": "a1", "type": "human", "name": "Y"}
    ]);
    let diagnostics = lint_workflow_with_project(&serde_json::to_string(&wf).unwrap())
        .expect("lint should succeed on structurally valid input");
    let blocking: Vec<&'static str> = diagnostics
        .iter()
        .filter(|d| matches!(d.severity, LintSeverity::Error | LintSeverity::Block))
        .map(|d| d.rule_id)
        .collect();
    assert!(
        blocking.contains(&"K-009"),
        "lint MUST catch duplicate actor ids via K-009; got blocking rules {blocking:?}"
    );
}

#[test]
fn lint_catches_unknown_initial_state() {
    // Mirror of phase7_gates::schema_pass_silent_on_unknown_initial_state_lint_catches_it.
    // K-016 is the parent-tier rule that catches unknown initialState
    // (DEFER-003 Tranche B closeout).
    let mut wf = minimal_valid_envelope();
    wf["lifecycle"]["initialState"] = json!("nonexistent");
    let diagnostics = lint_workflow_with_project(&serde_json::to_string(&wf).unwrap())
        .expect("lint should succeed on structurally valid input");
    let blocking: Vec<&'static str> = diagnostics
        .iter()
        .filter(|d| matches!(d.severity, LintSeverity::Error | LintSeverity::Block))
        .map(|d| d.rule_id)
        .collect();
    assert!(
        blocking.contains(&"K-016"),
        "lint MUST catch unknown initialState via K-016; got blocking rules {blocking:?}"
    );
}

#[test]
fn lint_baseline_is_clean_on_minimal_envelope() {
    // Sanity — confirms the fixture is otherwise lint-clean so the
    // cross-check above is testing what it claims to.
    let wf = minimal_valid_envelope();
    let diagnostics = lint_workflow_with_project(&serde_json::to_string(&wf).unwrap())
        .expect("lint should succeed on structurally valid input");
    let blocking: Vec<String> = diagnostics
        .iter()
        .filter(|d| matches!(d.severity, LintSeverity::Error | LintSeverity::Block))
        .map(|d| format!("{}: {}", d.rule_id, d.message))
        .collect();
    assert!(
        blocking.is_empty(),
        "minimal envelope should produce zero blocking diagnostics; got {blocking:?}"
    );
}
