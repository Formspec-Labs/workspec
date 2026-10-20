//! Shared authoring decisions through actual native checks and MCP dispatch.
use serde_json::{Value, json};
use wos_mcp::{dispatch::dispatch, registry::ProjectRegistry};

fn candidate() -> Value {
    serde_json::from_str(include_str!(
        "../../wos-lint/tests/fixtures/shared-validation/positive.json"
    ))
    .unwrap()
}
fn scenario() -> Value {
    json!({"id":"independent","rule":"control","description":"Reviewed typed event",
        "documents":{},"event_sequence":[{"event":"decide","actor":"reviewer"}],
        "expected_transitions":[{"from":"review","target":"done","event":"decide"}]})
}
async fn combined(candidate: Value, options: Value) -> Value {
    let mut registry = ProjectRegistry::new();
    let pid = registry
        .insert(wos_authoring::WosProject::from_native_document(candidate).unwrap())
        .unwrap()
        .to_string();
    let mut args = options;
    args["project_id"] = json!(pid);
    dispatch(&mut registry, "wos_check_authoring", &pid, args)
        .await
        .unwrap()
}
#[tokio::test]
async fn combined_pass_requires_actual_static_and_independent_replay() {
    let report = combined(candidate(), json!({"validation":{"scope":"project"},"replays":[{"id":"reviewed","arguments":{"fixture_json":scenario().to_string()}}]})).await;
    assert_eq!(
        report["decision"],
        json!({"status":"verified","reasons":[]})
    );
    assert_eq!(
        report["validation"]["result"]["report"]["validation"]["status"],
        "pass"
    );
    assert_eq!(report["replays"][0]["result"]["report"]["passed"], true);
    assert_eq!(
        report["replays"][0]["result"]["report"]["finalSnapshot"]["configuration"],
        json!(["done"])
    );
}

#[tokio::test]
async fn combined_retains_errors_and_blocks_duplicate_scenarios_and_document_scope() {
    let mut wrong = scenario();
    wrong["expected_transitions"][0]["target"] = json!("wrong");
    let args = json!({"validation":{"scope":"document"},"replays":[
        {"id":"duplicate","arguments":{"fixture_json":scenario().to_string()}},
        {"id":"duplicate","arguments":{"fixture_json":wrong.to_string()}},
        {"id":"refused","arguments":{"fixture_json":"{}"}}
    ]});
    let report = combined(candidate(), args).await;
    assert_eq!(report["decision"]["status"], "blocked");
    let codes: Vec<_> = report["decision"]["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|reason| reason["code"].as_str().unwrap())
        .collect();
    assert_eq!(
        codes,
        vec![
            "project-scope-required",
            "duplicate-result",
            "invalid-required-inventory",
            "duplicate-result",
            "failed",
            "error"
        ]
    );
    assert_eq!(report["replays"][0]["result"]["report"]["passed"], true);
    assert_eq!(report["replays"][1]["result"]["report"]["passed"], false);
    assert_eq!(report["replays"][2]["result"]["state"], "error");
}
