// Rust guideline compliant 2026-02-21
//! Canonical authoring preserves the complete workflow outside an explicit edit.

use serde_json::{Value, json};
use wos_mcp::{dispatch::dispatch, registry::ProjectRegistry};

fn owning_validator() -> jsonschema::Validator {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/wos-workflow.schema.json")).unwrap();
    jsonschema::validator_for(&schema).unwrap()
}

fn authored_workflow() -> Value {
    json!({
        "$wosWorkflow":"1.0", "url":"urn:test:canonical-authoring", "version":"1.0.0",
        "title":"Preserve the complete native workflow", "impactLevel":"informational",
        "actors":[{"id":"reviewer", "type":"human", "role":"caseReviewer", "x-owner":{"id":"human-A"}},{"id":"summaryBot","type":"agent"}],
        "caseFile":{"fields":{"income":{"type":"number", "required":true, "canonicalTermRef":"urn:term:income", "dpvSensitivity":"dpv:FinancialData", "x-field":{"source":"authored"}}}},
        "lifecycle":{"initialState":"review", "states":{"review":{"type":"atomic", "stateTimeout":"P1D", "x-state":{"retained":true}}}},
        "agents":[{"id":"summaryBot", "agentType":"deterministic", "modelIdentifier":"test-model", "modelVersion":"1.0.0", "x-agent":{"retained":true}}],
        "aiOversight":{"x-policy":{"retained":true}},
        "extensions":{"x-historical":{"retained":true},"x-wos-governance":{"old":true},"x-wos-ai":{"agents":[{"id":"historicalAgent"}]}},
        "x-test-context":{"retained":[1,true,"authored"]}
    })
}

#[tokio::test]
async fn native_import_edit_export_reopen_preserves_all_authored_values() {
    let mut expected = authored_workflow();
    assert!(owning_validator().is_valid(&expected));
    let mut registry = ProjectRegistry::new();
    let loaded = dispatch(
        &mut registry,
        "wos_load_document",
        "",
        json!({"json":expected.to_string()}),
    )
    .await
    .unwrap();
    let pid = loaded["project_id"].as_str().unwrap();
    dispatch(
        &mut registry,
        "wos_set_impact_level",
        pid,
        json!({"project_id":pid,"level":"operational"}),
    )
    .await
    .unwrap();
    expected["impactLevel"] = json!("operational");
    let exported = dispatch(
        &mut registry,
        "wos_export_document",
        pid,
        json!({"project_id":pid}),
    )
    .await
    .unwrap();
    let actual: Value = serde_json::from_str(exported["document"].as_str().unwrap()).unwrap();
    assert_eq!(
        actual["x-test-context"], expected["x-test-context"],
        "native root extension must survive an unrelated edit"
    );
    assert_eq!(
        actual, expected,
        "the complete authored document must survive outside the requested change"
    );
    let reopened = dispatch(
        &mut registry,
        "wos_load_document",
        "",
        json!({"json":actual.to_string()}),
    )
    .await
    .unwrap();
    let reopened_pid = reopened["project_id"].as_str().unwrap();
    let reexported = dispatch(
        &mut registry,
        "wos_export_document",
        reopened_pid,
        json!({"project_id":reopened_pid}),
    )
    .await
    .unwrap();
    let reexported: Value = serde_json::from_str(reexported["document"].as_str().unwrap()).unwrap();
    assert_eq!(reexported, expected);
    assert!(owning_validator().is_valid(&reexported));
}

#[tokio::test]
async fn create_complete_native_workflow_export_and_reopen() {
    let mut registry = ProjectRegistry::new();
    let created = dispatch(&mut registry, "wos_create_kernel", "", json!({}))
        .await
        .unwrap();
    assert_eq!(created["draft"], true);
    let pid = created["project_id"].as_str().unwrap();
    dispatch(
        &mut registry,
        "wos_add_actor",
        pid,
        json!({"project_id":pid,"actor_id":"reviewer","kind":"human"}),
    )
    .await
    .unwrap();
    dispatch(
        &mut registry,
        "wos_add_actor",
        pid,
        json!({"project_id":pid,"actor_id":"summaryBot","kind":"agent"}),
    )
    .await
    .unwrap();
    let edits = [
        json!({"fields":{"url":"urn:test:incremental-workflow","version":"1.0.0","title":"Native review workflow","wosVersionPin":{"envelopeVersion":"1.0"}}}),
        json!({"action":"case_field","name":"income","definition":{"type":"number","canonicalTermRef":"urn:term:income","x-authored":{"keep":true}}}),
        json!({"action":"contract","name":"reviewForm","contract":{"binding":"formspec","ref":"urn:formspec:review","definitionVersion":"2.0.0"}}),
        json!({"action":"state","pointer":"/lifecycle/states/review","state":{"type":"compound","initialState":"pending","states":{"pending":{"type":"atomic","x-authored":{"keep":true}},"done":{"type":"final"}}}}),
        json!({"action":"state_action","pointer":"/lifecycle/states/review/states/pending","phase":"onEntry","value":{"action":"createTask","taskRef":"reviewForm","x-task":{"keep":true}}}),
        json!({"action":"governance","value":{"dueProcess":{"scope":"true"},"reviewProtocols":[{"tags":["review"],"protocols":["independentFirst"]}],"taskCatalog":[{"id":"reviewForm","title":"Review"}]}}),
        json!({"action":"agent","value":{"id":"summaryBot","agentType":"deterministic","modelIdentifier":"native-model","modelVersion":"1.0.0","x-authored":{"keep":true}}}),
        json!({"action":"agent_constraint","agent_id":"summaryBot","constraint":{"id":"no-automatic-decision","kind":"prohibition","condition":"true","onViolation":"reject"}}),
        json!({"action":"ai_oversight","value":{"disclosure":{"discloseThatAgentAssisted":true},"x-authored":{"keep":true}}}),
    ];
    for mut edit in edits {
        edit["project_id"] = json!(pid);
        dispatch(&mut registry, "wos_edit_workflow", pid, edit.clone())
            .await
            .unwrap_or_else(|error| panic!("{edit}: {error}"));
    }
    dispatch(
        &mut registry,
        "wos_set_initial_state",
        pid,
        json!({"project_id":pid,"state_id":"review"}),
    )
    .await
    .unwrap();
    let export = dispatch(
        &mut registry,
        "wos_export_document",
        pid,
        json!({"project_id":pid}),
    )
    .await
    .unwrap();
    let actual: Value = serde_json::from_str(export["document"].as_str().unwrap()).unwrap();
    let validator = owning_validator();
    let errors: Vec<_> = validator
        .iter_errors(&actual)
        .map(|error| error.to_string())
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(
        actual.pointer("/lifecycle/states/review/states/pending/onEntry/0/taskRef"),
        Some(&json!("reviewForm"))
    );
    assert_eq!(
        actual["contracts"]["reviewForm"]["definitionVersion"],
        "2.0.0"
    );
    assert_eq!(actual["governance"]["taskCatalog"][0]["id"], "reviewForm");
    assert_eq!(
        actual["agents"][0]["deonticConstraints"][0]["kind"],
        "prohibition"
    );
    assert!(actual["extensions"].get("x-wos-ai").is_none());
    let reopened = dispatch(
        &mut registry,
        "wos_load_document",
        "",
        json!({"json":actual.to_string()}),
    )
    .await
    .unwrap();
    assert_eq!(
        registry
            .get(reopened["project_id"].as_str().unwrap())
            .unwrap()
            .native_snapshot(),
        actual
    );
}

#[test]
fn generated_catalog_and_coverage_match_executable_declarations() {
    let catalog: Value = serde_json::from_str(include_str!("../tool-catalog.json")).unwrap();
    let coverage: Value = serde_json::from_str(include_str!("../WORKFLOW-COVERAGE.json")).unwrap();
    assert_eq!(catalog, wos_mcp::catalog::descriptors());
    assert_eq!(coverage, wos_mcp::catalog::workflow_coverage());
    let properties = wos_authoring::native_schema::workflow_schema()["properties"]
        .as_object()
        .unwrap();
    let covered = coverage["fields"].as_object().unwrap();
    assert_eq!(
        properties.keys().collect::<Vec<_>>(),
        covered.keys().collect::<Vec<_>>()
    );
    let mut stale_catalog = catalog.clone();
    stale_catalog["tools"][0]["name"] = json!("unregistered_tool");
    assert_ne!(
        stale_catalog,
        wos_mcp::catalog::descriptors(),
        "missing-handler artifact mutation must be detected"
    );
    let mut stale_coverage = coverage.clone();
    stale_coverage["fields"]["newNativeField"] = json!({"support":"incremental"});
    assert_ne!(
        stale_coverage,
        wos_mcp::catalog::workflow_coverage(),
        "uncovered root-field mutation must be detected"
    );
}
