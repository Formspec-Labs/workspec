// Rust guideline compliant 2026-02-21
//! Exact native operation inputs pass through the real MCP dispatch.

#[path = "../../wos-conformance/tests/support/provenance_controls.rs"]
mod controls;

use serde_json::{Value, json};
use wos_conformance::studio_api::{prepare_replay, replay};
use wos_mcp::{dispatch::dispatch, registry::ProjectRegistry};

#[tokio::test]
async fn literal_provenance_inputs_match_shared_api_through_actual_mcp() {
    let corpus = controls::corpus();
    for vector in corpus["vectors"].as_array().unwrap() {
        let mut registry = ProjectRegistry::new();
        let loaded = dispatch(
            &mut registry,
            "wos_load_document",
            "",
            json!({"json":vector["candidate"].to_string()}),
        )
        .await
        .unwrap();
        let pid = loaded["project_id"].as_str().unwrap();
        assert_eq!(
            registry.get(pid).unwrap().native_snapshot(),
            vector["candidate"]
        );
        let dependencies:Vec<Value>=vector.get("dependencies").and_then(Value::as_array).into_iter().flatten().map(|dependency|json!({"role":dependency["role"],"url":dependency["url"],"version":dependency["version"],"document_json":dependency["document"].to_string()})).collect();
        let actual=dispatch(&mut registry,"wos_run_conformance",pid,json!({"project_id":pid,"fixture_json":vector["fixture"].to_string(),"resolved_dependencies":dependencies})).await;
        let prepared = prepare_replay(controls::request(vector));
        if let Some(reason) = vector["expected"]["preparationRefusalContains"].as_str() {
            assert!(prepared.unwrap_err().to_string().contains(reason));
            let error = actual.unwrap_err().to_string();
            assert!(error.contains(reason));
            controls::retain_observation("mcp", vector, &json!({"preparationError":error}));
            continue;
        }
        let owner = serde_json::to_value(replay(&prepared.unwrap()).unwrap()).unwrap();
        let actual = actual.unwrap();
        controls::retain_observation("mcp", vector, &actual);
        controls::assert_observations(vector, &owner);
        controls::assert_observations(vector, &actual);
        assert_eq!(owner["identity"], actual["identity"], "{}", vector["id"]);
        assert_eq!(owner["inputs"], actual["inputs"], "{}", vector["id"]);
        assert_eq!(
            controls::semantic_observations(&owner),
            controls::semantic_observations(&actual),
            "{}",
            vector["id"]
        );
        let tool = wos_mcp::catalog::help("wos_run_conformance").unwrap();
        let validator = jsonschema::validator_for(&tool["outputSchema"]).unwrap();
        assert!(
            validator.is_valid(&actual),
            "{}: output schema rejected {actual}",
            vector["id"]
        );
    }
}
