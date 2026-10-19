// Rust guideline compliant 2026-02-21
//! Shared literal inputs and assertion helpers for native operation evidence.

use serde_json::{Value, json};
use wos_conformance::studio_api::{ReplayRequest, ResolvedDependency};

pub fn corpus() -> Value {
    serde_json::from_str(include_str!(
        "../../fixtures/operation-provenance/vectors.json"
    ))
    .unwrap()
}

pub fn request(vector: &Value) -> ReplayRequest {
    ReplayRequest {
        candidate_bytes: serde_json::to_vec(&vector["candidate"]).unwrap(),
        dependencies: vector
            .get("dependencies")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|dependency| ResolvedDependency {
                role: dependency["role"].as_str().unwrap().into(),
                url: dependency["url"].as_str().unwrap().into(),
                version: dependency["version"].as_str().map(str::to_string),
                bytes: serde_json::to_vec(&dependency["document"]).unwrap(),
            })
            .collect(),
        fixture_json: vector["fixture"].to_string(),
        clock_start_ms: 0,
        required_observations: vec![],
        expectations: vec![],
    }
}

pub fn semantic_observations(result: &Value) -> Value {
    let snapshot = &result["finalSnapshot"];
    let records = result["provenance"].as_array().unwrap();
    let mut counts = serde_json::Map::new();
    for record in records {
        let kind = record["recordKind"].as_str().unwrap();
        let count = counts.entry(kind).or_insert(json!(0));
        *count = json!(count.as_u64().unwrap() + 1);
    }
    json!({
        "status":result["status"],
        "configuration":snapshot["configuration"],
        "caseState":snapshot["caseState"],
        "definitionVersion":snapshot["definitionVersion"],
        "stepResults":snapshot["stepResults"],
        "taskRefs":snapshot["activeTasks"].as_array().unwrap().iter().map(|task|task["taskRef"].clone()).collect::<Vec<_>>(),
        "validationResults":records.iter().filter(|record|record["recordKind"]=="contractValidation").map(|record|json!({"valid":record["data"]["valid"],"errors":record["data"]["errors"]})).collect::<Vec<_>>(),
        "provenanceCounts":counts,
        "refusals":result["refusals"],
        "attemptErrors":result["attempts"].as_array().unwrap().iter().filter_map(|attempt|attempt["error"].as_str()).collect::<Vec<_>>()
    })
}

pub fn assert_observations(vector: &Value, result: &Value) {
    let expected = &vector["expected"];
    let actual = semantic_observations(result);
    let id = vector["id"].as_str().unwrap();
    assert_eq!(actual["status"], expected["status"], "{id}: {result}");
    for key in [
        "configuration",
        "caseState",
        "definitionVersion",
        "stepResults",
        "taskRefs",
        "validationResults",
    ] {
        if let Some(value) = expected.get(key) {
            assert_eq!(&actual[key], value, "{id} {key}: {result}");
        }
    }
    if let Some(outputs) = expected.get("stepOutputs") {
        let results = actual["stepResults"]
            .as_array()
            .expect("saved step-results observation");
        assert_eq!(
            json!(
                results
                    .iter()
                    .map(|step| step["output"].clone())
                    .collect::<Vec<_>>()
            ),
            *outputs,
            "{id}: {result}"
        );
        for step in results {
            assert_eq!(step["serviceRef"], "verify");
            assert_eq!(step["idempotencyKey"], "native-key");
        }
    }
    if let Some(counts) = expected["provenanceCounts"].as_object() {
        for (kind, count) in counts {
            assert_eq!(
                actual["provenanceCounts"]
                    .get(kind)
                    .cloned()
                    .unwrap_or(json!(0)),
                *count,
                "{id} {kind}: {result}"
            );
        }
    }
    if let Some(message) = expected
        .get("operationErrorContains")
        .and_then(Value::as_str)
    {
        assert!(
            actual["attemptErrors"]
                .as_array()
                .unwrap()
                .iter()
                .any(|error| error.as_str().unwrap().contains(message)),
            "{id}: {result}"
        );
    }
    // Every retained task is backed by the actual native record naming its ID.
    for task in result["finalSnapshot"]["activeTasks"].as_array().unwrap() {
        assert!(
            result["provenance"]
                .as_array()
                .unwrap()
                .iter()
                .any(|record| record["recordKind"] == "taskCreated"
                    && record["data"]["taskId"] == task["taskId"]),
            "{id}: task without its native creation witness"
        );
    }
}

// Opt-in evidence contains actual returned values, including read failures and
// refusals; expectations never supply any part of the captured result.
pub fn retain_observation(channel: &str, vector: &Value, result: &Value) {
    if let Some(root) = std::env::var_os("WOS_PROVENANCE_EVIDENCE_DIR") {
        let path = std::path::PathBuf::from(root).join(channel);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(
            path.join(format!("{}.json", vector["id"].as_str().unwrap())),
            serde_json::to_vec_pretty(result).unwrap(),
        )
        .unwrap();
    }
}
