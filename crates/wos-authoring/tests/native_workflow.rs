// Rust guideline compliant 2026-02-21
//! Lossless native edits preserve authored values and collection identity.

use serde_json::{Value, json};
use wos_authoring::{ActorKind, ImpactLevel, WosProject};

fn source() -> Value {
    json!({"$wosWorkflow":"1.0","url":"urn:test:native","version":"1.0.0","title":"Native fidelity","impactLevel":"informational",
        "actors":[{"id":"A","type":"agent","x-owner":"A"},{"id":"B","type":"agent","x-owner":"B"}],
        "lifecycle":{"initialState":"root","states":{"root":{"type":"compound","initialState":"child","states":{"child":{"type":"atomic","stateTimeout":"P1D","x-state":"child"}},"x-state":"root"}}},
        "agents":[{"id":"A","agentType":"deterministic","modelIdentifier":"model-A","modelVersion":"1.0.0","x-owner":"agent-A"},{"id":"B","agentType":"deterministic","modelIdentifier":"model-B","modelVersion":"1.0.0","x-owner":"agent-B"}],
        "caseFile":{"fields":{"income":{"type":"number","canonicalTermRef":"urn:term:income","dpvSensitivity":"dpv:FinancialData"}}},
        "extensions":{"x-wos-ai":{"historical":"retain verbatim"}},"x-root":{"source":"authored"}})
}

#[test]
fn native_edit_and_undo_preserve_complete_source_and_array_identity() {
    let original = source();
    let mut project = WosProject::from_native_document(original.clone()).unwrap();
    assert_eq!(project.native_snapshot(), original);
    project.remove_actor("A").unwrap();
    assert_eq!(
        project.native_snapshot()["actors"],
        json!([{"id":"B","type":"agent","x-owner":"B"}])
    );
    project.remove_agent("A".into()).unwrap();
    assert_eq!(
        project.native_snapshot()["agents"],
        json!([{"id":"B","agentType":"deterministic","modelIdentifier":"model-B","modelVersion":"1.0.0","x-owner":"agent-B"}])
    );
    project.undo().unwrap();
    project.undo().unwrap();
    assert_eq!(project.native_snapshot(), original);
    project.redo().unwrap();
    assert_eq!(project.native_snapshot()["actors"][0]["x-owner"], "B");
    assert_eq!(project.native_snapshot()["x-root"], original["x-root"]);
    assert_eq!(project.native_snapshot()["caseFile"], original["caseFile"]);
    assert_eq!(
        project.native_snapshot()["extensions"],
        original["extensions"]
    );
}

#[test]
fn explicit_native_state_edit_preserves_siblings_and_failed_payload_is_atomic() {
    let mut project = WosProject::from_native_document(source()).unwrap();
    project
        .put_state(
            "/lifecycle/states/root/states/second".into(),
            json!({"type":"atomic","x-state":"second"}),
        )
        .unwrap();
    let before = project.native_snapshot();
    assert!(
        project
            .put_state("/actors/0".into(), json!({"type":"atomic"}))
            .is_err()
    );
    assert!(
        project
            .put_case_field("bad".into(), json!({"type":"not-a-field-type"}))
            .is_err()
    );
    assert_eq!(project.native_snapshot(), before);
    assert_eq!(
        before["lifecycle"]["states"]["root"]["states"]["child"]["x-state"],
        "child"
    );
    assert_eq!(before["lifecycle"]["states"]["root"]["x-state"], "root");
}

#[test]
fn native_source_remains_editable_when_runtime_model_cannot_represent_it() {
    let mut original = source();
    original["maxRelationshipEventDepth"] = json!(4_294_967_296_u64);
    let validator = wos_authoring::native_schema::workflow_validator().unwrap();
    assert!(validator.is_valid(&original));
    let mut project = WosProject::from_native_document(original.clone()).unwrap();
    assert!(project.typed_snapshot().is_err());
    project.set_impact_level(ImpactLevel::Operational).unwrap();
    original["impactLevel"] = json!("operational");
    assert_eq!(project.native_snapshot(), original);
}

#[test]
fn canonical_agents_constraints_and_governance_do_not_write_obsolete_extensions() {
    let mut project = WosProject::from_native_document(source()).unwrap();
    project
        .put_agent(json!({"id":"newAgent","agentType":"deterministic","modelIdentifier":"model","modelVersion":"1.0.0"}))
        .unwrap();
    project
        .add_agent_constraint(
            "newAgent".into(),
            json!({"id":"no-adverse-action","kind":"prohibition","condition":"@transition.tags includes 'adverseAction'","onViolation":"reject"}),
        )
        .unwrap();
    project.set_native_fields(json!({"governance":{"dueProcess":{"scope":"true"}},"aiOversight":{"x-disclosure":"authored"}}).as_object().unwrap().clone()).unwrap();
    let snapshot = project.native_snapshot();
    assert_eq!(
        snapshot["agents"][2]["deonticConstraints"][0]["kind"],
        "prohibition"
    );
    assert_eq!(snapshot["extensions"], source()["extensions"]);
    assert!(snapshot["extensions"].get("x-wos-governance").is_none());
}

#[test]
fn obsolete_commands_are_explicit_refusals_with_no_partial_write() {
    let mut project = WosProject::from_native_document(source()).unwrap();
    let original = project.native_snapshot();
    assert!(
        project
            .add_due_process_path("old", "unsupported legacy shape", vec![])
            .is_err()
    );
    assert!(
        project
            .add_assertion_gate("old", "true", "approve")
            .is_err()
    );
    assert!(
        project
            .add_actor_deontic("old", "natural language is not a deontic expression")
            .is_err()
    );
    assert!(
        project
            .add_deontic_constraint("old", "A", "must_not", "approve")
            .is_err()
    );
    assert_eq!(project.native_snapshot(), original);
    assert!(!project.can_undo());
    project.add_actor("new", ActorKind::Human).unwrap();
    assert!(project.can_undo());
}

fn shadowed_state_workflow() -> Value {
    json!({
        "$wosWorkflow":"1.0","url":"urn:test:shadowed-state","version":"1.0.0","title":"State ownership","impactLevel":"operational",
        "actors":[{"id":"reviewer","type":"human"}],
        "lifecycle":{"initialState":"root","states":{
            "root":{"type":"compound","initialState":"pending","x-owner":"root","transitions":[{"event":{"kind":"message","name":"finish"},"target":"done","x-owner":"root-transition"},{"event":{"kind":"message","name":"loop"},"target":"root","x-owner":"surviving-transition"}],
                "states":{"pending":{"type":"atomic","transitions":[{"event":{"kind":"message","name":"finish"},"target":"done","x-owner":"nested-transition"}]},"done":{"type":"final","x-owner":"nested-done"}}},
            "done":{"type":"final","x-owner":"root-done"}
        }}
    })
}

#[test]
fn removing_root_state_preserves_shadowed_nested_state_targets_and_metadata() {
    let source = shadowed_state_workflow();
    assert!(
        wos_authoring::native_schema::workflow_validator()
            .unwrap()
            .is_valid(&source)
    );
    let mut project = WosProject::from_native_document(source.clone()).unwrap();
    project.remove_state("done").unwrap();
    let actual = project.native_snapshot();
    assert!(actual.pointer("/lifecycle/states/done").is_none());
    assert_eq!(
        actual.pointer("/lifecycle/states/root/transitions"),
        Some(
            &json!([{"event":{"kind":"message","name":"loop"},"target":"root","x-owner":"surviving-transition"}])
        )
    );
    assert_eq!(
        actual.pointer("/lifecycle/states/root/states"),
        source.pointer("/lifecycle/states/root/states")
    );
    project.undo().unwrap();
    assert_eq!(project.native_snapshot(), source);
}

#[test]
fn renaming_root_state_preserves_shadowed_nested_state_targets_and_metadata() {
    let source = shadowed_state_workflow();
    let mut project = WosProject::from_native_document(source.clone()).unwrap();
    project.rename_state("done", "closed").unwrap();
    let actual = project.native_snapshot();
    assert_eq!(
        actual.pointer("/lifecycle/states/root/transitions/0/target"),
        Some(&json!("closed"))
    );
    assert_eq!(
        actual.pointer("/lifecycle/states/root/transitions/0/x-owner"),
        Some(&json!("root-transition"))
    );
    assert_eq!(
        actual.pointer("/lifecycle/states/closed/x-owner"),
        Some(&json!("root-done"))
    );
    assert_eq!(
        actual.pointer("/lifecycle/states/root/states"),
        source.pointer("/lifecycle/states/root/states")
    );
    project.undo().unwrap();
    assert_eq!(project.native_snapshot(), source);
}
