// Rust guideline compliant 2026-02-21
//! Native AI editing and draft navigation have independently authored expectations.

#[path = "../../wos-conformance/tests/support/local_schemas.rs"]
mod test_schemas;

use serde_json::{Value, json};
use wos_authoring::WosProject;
use wos_mcp::{dispatch::dispatch, errors::DispatchError, registry::ProjectRegistry};

fn native_agent() -> Value {
    json!({
        "id":"reviewBot","agentType":"deterministic","modelIdentifier":"review-model","modelVersion":"1.0.0",
        "description":"Review case data","capabilities":[{"id":"assess","preconditions":["true"]}],
        "invoker":{"kind":"stub","responses":[{"output":{"assessment":"review"},"confidenceReport":{"overall":0.9,"method":"declared","fieldLevel":{"assessment":0.8}}}]},
        "x-authored":{"owner":"reviewBot"}
    })
}

#[tokio::test]
async fn retired_agent_constructor_is_unknown_and_does_not_mutate_a_project() {
    let mut registry = ProjectRegistry::new();
    let created = dispatch(&mut registry, "wos_create_kernel", "", json!({}))
        .await
        .unwrap();
    let pid = created["project_id"].as_str().unwrap();
    let before = registry.get(pid).unwrap().native_snapshot();
    let error=dispatch(&mut registry,"wos_add_ai_agent",pid,json!({
        "project_id":pid,"agent_id":"reviewBot","role":"review","model":"model","capabilities":["assess"]
    })).await.unwrap_err();
    assert!(matches!(error,DispatchError::UnknownTool(ref name) if name=="wos_add_ai_agent"));
    assert_eq!(registry.get(pid).unwrap().native_snapshot(), before);
    assert!(wos_mcp::catalog::help("wos_add_ai_agent").is_none());
}

#[tokio::test]
async fn native_product_edits_export_and_reopen_without_reconstructing_agent_policy() {
    let mut registry = ProjectRegistry::new();
    let created = dispatch(&mut registry, "wos_create_kernel", "", json!({}))
        .await
        .unwrap();
    assert_eq!(created["draft"], true);
    let pid = created["project_id"].as_str().unwrap();
    dispatch(&mut registry,"wos_edit_workflow",pid,json!({"project_id":pid,"fields":{
        "url":"urn:test:native-ai-editor","version":"1.0.0","title":"Native AI editor","impactLevel":"informational"
    }})).await.unwrap();
    // The actor edit is deliberately an intermediate draft with no declaration yet.
    dispatch(
        &mut registry,
        "wos_add_actor",
        pid,
        json!({"project_id":pid,"actor_id":"reviewBot","kind":"agent"}),
    )
    .await
    .unwrap();
    dispatch(
        &mut registry,
        "wos_add_state",
        pid,
        json!({"project_id":pid,"state_id":"open","kind":"final"}),
    )
    .await
    .unwrap();
    dispatch(
        &mut registry,
        "wos_set_initial_state",
        pid,
        json!({"project_id":pid,"state_id":"open"}),
    )
    .await
    .unwrap();
    dispatch(
        &mut registry,
        "wos_edit_workflow",
        pid,
        json!({"project_id":pid,"action":"agent","value":native_agent()}),
    )
    .await
    .unwrap();
    let rules = [
        json!({"id":"assessment-only","kind":"permission","allowedFields":["/assessment"],"onViolation":"reject"}),
        json!({"id":"no-adverse-output","kind":"prohibition","condition":"output.adverse = true","onViolation":"reject"}),
        json!({"id":"include-assessment","kind":"obligation","requirement":"output.assessment != null","onViolation":"flag"}),
        json!({"id":"application-input","kind":"right","entitlement":"/input/application"}),
    ];
    for rule in &rules {
        dispatch(&mut registry,"wos_edit_workflow",pid,json!({"project_id":pid,"action":"agent_constraint","agent_id":"reviewBot","constraint":rule})).await.unwrap();
    }
    dispatch(&mut registry,"wos_edit_workflow",pid,json!({"project_id":pid,"action":"ai_oversight","value":{"defaultAutonomy":"assistive","confidenceFloor":{"threshold":0.8,"onViolation":"reject"}}})).await.unwrap();
    let expected = registry.get(pid).unwrap().native_snapshot();
    assert_eq!(expected["agents"][0]["deonticConstraints"], json!(rules));
    assert_eq!(
        expected["agents"][0]["capabilities"],
        json!([{"id":"assess","preconditions":["true"]}])
    );
    assert_eq!(expected["agents"][0]["x-authored"]["owner"], "reviewBot");
    // Editing an explicit response proves source fidelity, not agent invocation.
    assert_eq!(
        expected["agents"][0]["invoker"]["responses"][0]["confidenceReport"],
        json!({"overall":0.9,"method":"declared","fieldLevel":{"assessment":0.8}})
    );
    assert!(expected["extensions"].get("x-wos-ai").is_none());
    let schema = wos_authoring::native_schema::workflow_schema();
    let validator = test_schemas::validator(schema);
    let errors: Vec<_> = validator
        .iter_errors(&expected)
        .map(|error| error.to_string())
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    let exported = dispatch(
        &mut registry,
        "wos_export_document",
        pid,
        json!({"project_id":pid}),
    )
    .await
    .unwrap();
    let actual: Value = serde_json::from_str(exported["document"].as_str().unwrap()).unwrap();
    assert_eq!(actual, expected);
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
        expected
    );

    let mut missing_version = native_agent();
    missing_version
        .as_object_mut()
        .unwrap()
        .remove("modelVersion");
    let mut string_capability = native_agent();
    string_capability["capabilities"] = json!(["assess"]);
    let mut scalar_floor = native_agent();
    scalar_floor["confidenceFloor"] = json!(0.8);
    let mut missing_report = native_agent();
    missing_report["invoker"]["responses"][0]
        .as_object_mut()
        .unwrap()
        .remove("confidenceReport");
    let mut scalar_report = native_agent();
    scalar_report["invoker"]["responses"][0]["confidenceReport"] = json!(0.9);
    let mut duplicate_model = native_agent();
    duplicate_model["invoker"] = json!({"kind":"anthropic","model":"second-model"});
    for value in [
        missing_version,
        string_capability,
        scalar_floor,
        missing_report,
        scalar_report,
        duplicate_model,
    ] {
        assert!(
            dispatch(
                &mut registry,
                "wos_edit_workflow",
                pid,
                json!({"project_id":pid,"action":"agent","value":value})
            )
            .await
            .is_err()
        );
        assert_eq!(registry.get(pid).unwrap().native_snapshot(), expected);
    }
    for rule in [
        json!({"id":"old-shape","kind":"prohibition","expression":"true"}),
        json!({"id":"cross-kind","kind":"right","entitlement":"/input/application","onViolation":"reject"}),
    ] {
        assert!(dispatch(&mut registry,"wos_edit_workflow",pid,json!({"project_id":pid,"action":"agent_constraint","agent_id":"reviewBot","constraint":rule})).await.is_err());
        assert_eq!(registry.get(pid).unwrap().native_snapshot(), expected);
    }
    assert!(
        dispatch(
            &mut registry,
            "wos_edit_workflow",
            pid,
            json!({"project_id":pid,"action":"ai_oversight","value":{"driftDetection":{}}})
        )
        .await
        .is_err()
    );
    assert_eq!(registry.get(pid).unwrap().native_snapshot(), expected);
}

#[tokio::test]
async fn constraint_search_preserves_raw_draft_values_and_exact_scope_locations() {
    let source = json!({
        "$wosWorkflow":"1.0","title":"Searchable draft","impactLevel":"informational",
        "maxRelationshipEventDepth":4_294_967_296_u64,
        "actors":[],"lifecycle":{"initialState":"open","states":{"open":{"type":"atomic"}}},
        "aiOversight":{"deonticConstraints":[{"id":"limit","kind":"unfinished","x-note":"Workflow LIMIT"}]},
        "agents":[{"id":"draftAgent","deonticConstraints":[{"id":"limit","kind":"permission"}],
            "actionOverrides":[{"actionRef":"/not/resolved","deonticConstraints":["LIMIT incomplete rule"]}],
            "x-metadata":{"deonticConstraints":[{"id":"limit","notNative":true}]}}],
        "extensions":{"x-wos-ai":{"deonticConstraints":[{"id":"limit","historical":true}]}}
    });
    let project = WosProject::from_native_document(source.clone()).unwrap();
    assert!(project.typed_snapshot().is_err());
    let mut registry = ProjectRegistry::new();
    let pid = registry.insert(project).unwrap().to_string();
    let result = dispatch(
        &mut registry,
        "wos_search",
        &pid,
        json!({"project_id":pid,"kind":"constraint","query":"LiMiT"}),
    )
    .await
    .unwrap();
    let matches = result["matches"].as_array().unwrap();
    let paths: Vec<_> = matches
        .iter()
        .map(|entry| entry["path"].as_str().unwrap())
        .collect();
    assert_eq!(
        paths,
        vec![
            "/aiOversight/deonticConstraints/0",
            "/agents/0/deonticConstraints/0",
            "/agents/0/actionOverrides/0/deonticConstraints/0"
        ]
    );
    for entry in matches {
        let pointer = entry["path"].as_str().unwrap();
        assert_eq!(source.pointer(pointer), Some(&entry["value"]));
        assert_eq!(entry.as_object().unwrap().len(), 2);
    }
    assert!(
        dispatch(
            &mut registry,
            "wos_search",
            &pid,
            json!({"project_id":pid,"kind":"state","query":"open"})
        )
        .await
        .is_err()
    );
    assert_eq!(registry.get(&pid).unwrap().native_snapshot(), source);
}
