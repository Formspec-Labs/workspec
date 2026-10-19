// Rust guideline compliant 2026-02-21

//! Exact candidate controls through the actual MCP dispatch entry.

use serde_json::{Value, json};
use wos_mcp::{dispatch::dispatch, registry::ProjectRegistry};

fn workflow() -> Value {
    json!({"$wosWorkflow":"1.0","url":"urn:test:mcp-exact","version":"1.0.0","title":"MCP candidate control","impactLevel":"operational",
      "actors":[{"id":"reviewer","type":"human"}],"lifecycle":{"initialState":"open","states":{
        "open":{"type":"atomic","transitions":[{"event":{"kind":"message","name":"decide"},"target":"done","guard":"true"}]},"done":{"type":"final"}}}})
}
fn scenario() -> Value {
    json!({"id":"mcp-independent-success","rule":"control","description":"Reviewed expectation independent of project edits",
      "documents":{},"event_sequence":[{"event":"decide","actor":"reviewer"}],
      "expected_transitions":[{"from":"open","target":"done","event":"decide"}]})
}
async fn load(registry: &mut ProjectRegistry, document: Value) -> String {
    dispatch(
        registry,
        "wos_load_document",
        "",
        json!({"json":document.to_string()}),
    )
    .await
    .unwrap()["project_id"]
        .as_str()
        .unwrap()
        .into()
}
#[tokio::test]
async fn actual_mcp_project_mutation_changes_the_executed_candidate() {
    let mut registry = ProjectRegistry::new();
    let project_id = load(&mut registry, workflow()).await;
    let args = json!({"project_id":project_id,"fixture_json":scenario().to_string()});
    let first = dispatch(
        &mut registry,
        "wos_run_conformance",
        &project_id,
        args.clone(),
    )
    .await
    .unwrap();
    assert_eq!(first["passed"], true, "{first}");
    dispatch(
        &mut registry,
        "wos_set_initial_state",
        &project_id,
        json!({"project_id":project_id,"state_id":"done"}),
    )
    .await
    .unwrap();
    let second = dispatch(&mut registry, "wos_run_conformance", &project_id, args)
        .await
        .unwrap();
    assert_eq!(
        second["passed"], false,
        "unchanged scenario must fail on changed current project: {second}"
    );
    assert_ne!(first["identity"], second["identity"]);
    assert_eq!(
        second["attempts"][0]["before"]["configuration"],
        json!(["done"])
    );
    assert_eq!(second["trace"]["steps"][0]["stateAfter"], "done");
}
#[tokio::test]
async fn actual_mcp_refuses_substituted_workflow_and_unsupported_observable() {
    let mut registry = ProjectRegistry::new();
    let project_id = load(&mut registry, workflow()).await;
    let mut fixture = scenario();
    let mut substitute = workflow();
    substitute["url"] = json!("urn:test:substitute");
    fixture["documents"] = json!({"kernel":"inline"});
    fixture["inline_documents"] = json!({"kernel":substitute});
    let result = dispatch(
        &mut registry,
        "wos_run_conformance",
        &project_id,
        json!({"project_id":project_id,"fixture_json":fixture.to_string()}),
    )
    .await;
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("substituted-fixture-document")
    );
    let result=dispatch(&mut registry,"wos_run_conformance",&project_id,json!({"project_id":project_id,"fixture_json":scenario().to_string(),"required_observations":["realFormspecValidation"]})).await;
    assert!(result.is_err(), "unsupported observation must refuse");
    let result = dispatch(
        &mut registry,
        "wos_run_conformance",
        "unknown",
        json!({"project_id":"unknown","fixture_json":scenario().to_string()}),
    )
    .await;
    assert!(result.is_err());
}
#[tokio::test]
async fn actual_mcp_repeats_and_compares_only_actual_output_values() {
    let mut registry = ProjectRegistry::new();
    let project_id = load(&mut registry, workflow()).await;
    let args = json!({"project_id":project_id,"fixture_json":scenario().to_string(),"clock_start_ms":12345,
        "expected_observations":[{"pointer":"/finalSnapshot/configuration","expected":["wrong"]}]});
    let first = dispatch(
        &mut registry,
        "wos_run_conformance",
        &project_id,
        args.clone(),
    )
    .await
    .unwrap();
    let second = dispatch(&mut registry, "wos_run_conformance", &project_id, args)
        .await
        .unwrap();
    assert_eq!(first["passed"], false);
    assert_eq!(first["differences"][0]["actual"], json!(["done"]));
    assert_eq!(first["trace"], second["trace"]);
    assert_eq!(first["identity"], second["identity"]);
    assert_eq!(first["attempts"][0]["clockMs"], 12345);
    assert_eq!(first["execution"]["hosted"], false);
    assert_eq!(first["execution"]["durable"], false);
}

#[tokio::test]
async fn common_literal_vectors_match_the_owner_through_actual_mcp() {
    use wos_conformance::studio_api::{ReplayRequest, ResolvedDependency, prepare_replay, replay};
    let corpus: Value = serde_json::from_str(include_str!(
        "../../wos-conformance/fixtures/authoring-replay/vectors.json"
    ))
    .unwrap();
    for vector in corpus["vectors"].as_array().unwrap() {
        let mut registry = ProjectRegistry::new();
        let project_id = load(&mut registry, vector["candidate"].clone()).await;
        let required = vector
            .get("requiredObservations")
            .cloned()
            .unwrap_or(json!([]));
        let expectations = vector.get("expectations").cloned().unwrap_or(json!([]));
        let dependencies: Vec<Value> = vector
            .get("dependencies")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let owner = prepare_replay(ReplayRequest {
            candidate_bytes: serde_json::to_vec(&vector["candidate"]).unwrap(),
            fixture_json: vector["fixture"].to_string(),
            clock_start_ms: 0,
            required_observations: serde_json::from_value(required.clone()).unwrap(),
            expectations: serde_json::from_value(expectations.clone()).unwrap(),
            dependencies: dependencies
                .iter()
                .map(|dependency| ResolvedDependency {
                    role: dependency["role"].as_str().unwrap().into(),
                    url: dependency["url"].as_str().unwrap().into(),
                    version: dependency["version"].as_str().map(str::to_string),
                    bytes: serde_json::to_vec(&dependency["document"]).unwrap(),
                })
                .collect(),
        });
        let adapter=dispatch(&mut registry,"wos_run_conformance",&project_id,json!({"project_id":project_id,
            "fixture_json":vector["fixture"].to_string(),"required_observations":required,"expected_observations":expectations,
            "resolved_dependencies":dependencies.iter().map(|dependency|json!({"role":dependency["role"],"url":dependency["url"],"version":dependency["version"],"document_json":dependency["document"].to_string()})).collect::<Vec<_>>()
        })).await;
        match owner {
            Err(error) => assert!(
                adapter.is_err(),
                "vector {} must refuse through both boundaries: owner {error}; adapter {adapter:?}",
                vector["id"]
            ),
            Ok(prepared) => {
                let owner = replay(&prepared).unwrap();
                let adapter = adapter.unwrap();
                assert_eq!(
                    owner.status,
                    adapter["status"].as_str().unwrap(),
                    "vector {}",
                    vector["id"]
                );
                assert_eq!(
                    owner.identity,
                    adapter["identity"].as_str().unwrap(),
                    "same selected native inputs for vector {}",
                    vector["id"]
                );
                assert_eq!(
                    serde_json::to_value(owner.final_snapshot).unwrap(),
                    adapter["finalSnapshot"],
                    "vector {}",
                    vector["id"]
                );
                assert_eq!(
                    serde_json::to_value(owner.trace).unwrap(),
                    adapter["trace"],
                    "vector {}",
                    vector["id"]
                );
            }
        }
    }
}

#[tokio::test]
async fn replay_identity_binds_native_only_metadata_without_typed_reconstruction() {
    use wos_conformance::studio_api::{ReplayRequest, prepare_replay, replay};
    let mut registry = ProjectRegistry::new();
    let mut native = workflow();
    native["x-review-context"] =
        json!({"source":"independent-review","decision":"keep exact native fields"});
    native["lifecycle"]["states"]["open"]["x-evidence"] = json!(["source-1",{"page":9}]);
    let project_id = load(&mut registry, native.clone()).await;
    let owner = replay(
        &prepare_replay(ReplayRequest {
            candidate_bytes: serde_json::to_vec(&native).unwrap(),
            dependencies: vec![],
            fixture_json: scenario().to_string(),
            clock_start_ms: 0,
            required_observations: vec![],
            expectations: vec![],
        })
        .unwrap(),
    )
    .unwrap();
    let actual = dispatch(
        &mut registry,
        "wos_run_conformance",
        &project_id,
        json!({"project_id":project_id,"fixture_json":scenario().to_string()}),
    )
    .await
    .unwrap();
    assert_eq!(actual["identity"], owner.identity);
    assert_eq!(
        actual["inputs"],
        serde_json::to_value(owner.inputs).unwrap()
    );
}

#[tokio::test]
async fn shared_catalog_and_replay_refuse_mutable_paths_and_unrecognized_inputs() {
    let mut registry = ProjectRegistry::new();
    let project_id = load(&mut registry, workflow()).await;
    let base = json!({"project_id":project_id,"fixture_json":scenario().to_string()});
    for (field, value) in [
        ("base_dir", json!(".")),
        ("expected_output", json!({"pretend":"success"})),
        ("clock_start_ms", json!(-1)),
    ] {
        let mut args = base.clone();
        args[field] = value;
        assert!(
            dispatch(&mut registry, "wos_run_conformance", &project_id, args)
                .await
                .is_err(),
            "unrecognized or invalid input {field} must refuse"
        );
    }
    let mut scenario = scenario();
    scenario["documents"] = json!({"kernel":"mutable-candidate.json"});
    let mut args = base.clone();
    args["fixture_json"] = json!(scenario.to_string());
    assert!(
        dispatch(&mut registry, "wos_run_conformance", &project_id, args)
            .await
            .unwrap_err()
            .to_string()
            .contains("substituted-fixture-document")
    );
    let mut args = base.clone();
    args["expected_observations"] = json!([{"pointer":"/finalSnapshot/configuration"}]);
    assert!(
        dispatch(&mut registry, "wos_run_conformance", &project_id, args)
            .await
            .is_err(),
        "expectation value must be explicit"
    );
    let mut args = base;
    args["resolved_dependencies"] = json!([{"role":"businessCalendar","url":"urn:test:calendar","version":"1.0.0","document_json":"{}","path":"mutable.json"}]);
    assert!(
        dispatch(&mut registry, "wos_run_conformance", &project_id, args)
            .await
            .is_err(),
        "dependencies must contain resolved bytes, without path substitution"
    );
}

#[tokio::test]
async fn actual_mcp_binds_deployment_posture_to_the_selected_signature_policy() {
    let vector: Value = serde_json::from_str(include_str!(
        "../../wos-conformance/fixtures/authoring-replay/posture-policy.json"
    ))
    .unwrap();
    let mut registry = ProjectRegistry::new();
    let project_id = load(&mut registry, vector["candidate"].clone()).await;
    let mut dependency = json!({"role":"postureDeclaration","url":vector["dependency"]["url"],"version":"1.0.0","document_json":vector["dependency"]["document"].to_string()});
    let args = |dependency: Value| json!({"project_id":project_id,"fixture_json":vector["fixture"].to_string(),"resolved_dependencies":[dependency]});
    let result = dispatch(
        &mut registry,
        "wos_run_conformance",
        &project_id,
        args(dependency.clone()),
    )
    .await
    .unwrap();
    assert_eq!(result["passed"], true, "{result}");
    assert_eq!(
        result["finalSnapshot"]["configuration"],
        vector["expectedConfiguration"]
    );
    let mut document = vector["dependency"]["document"].clone();
    document["url"] = json!("https://example.gov/posture/alternate.json");
    dependency["url"] = document["url"].clone();
    dependency["document_json"] = json!(document.to_string());
    assert!(
        dispatch(
            &mut registry,
            "wos_run_conformance",
            &project_id,
            args(dependency)
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("dependency-posture-policy-mismatch")
    );
}

#[tokio::test]
async fn actual_mcp_refuses_retired_agent_shapes_before_creating_a_native_project() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../wos-conformance/fixtures/authoring-replay/unsupported-agent-shapes.json"
    ))
    .unwrap();
    for vector in corpus["vectors"].as_array().unwrap() {
        let mut registry = ProjectRegistry::new();
        let original = vector["candidate"].to_string();
        let result = dispatch(
            &mut registry,
            "wos_load_document",
            "",
            json!({"json":original}),
        )
        .await;
        assert!(
            result.is_err(),
            "{}: retired schema shape cannot become a current native project",
            vector["id"]
        );
        assert_eq!(vector["candidate"].to_string(), original);
    }
}

#[tokio::test]
async fn shared_native_ai_corpus_executes_identical_candidates_through_owner_and_mcp() {
    use wos_conformance::studio_api::{ReplayRequest, prepare_replay, replay};
    let corpus: Value = serde_json::from_str(include_str!(
        "../../wos-conformance/fixtures/native-ai-replay/vectors.json"
    ))
    .unwrap();
    for vector in corpus["vectors"].as_array().unwrap() {
        let mut registry = ProjectRegistry::new();
        let candidate_bytes = serde_json::to_vec(&vector["candidate"]).unwrap();
        let project_id = load(&mut registry, vector["candidate"].clone()).await;
        let owner = replay(
            &prepare_replay(ReplayRequest {
                candidate_bytes,
                dependencies: vec![],
                fixture_json: vector["fixture"].to_string(),
                clock_start_ms: 1234,
                required_observations: serde_json::from_value(
                    vector["requiredObservations"].clone(),
                )
                .unwrap(),
                expectations: serde_json::from_value(vector["expectations"].clone()).unwrap(),
            })
            .unwrap(),
        )
        .unwrap();
        assert_eq!(vector["dependencies"], json!([]));
        let actual = dispatch(&mut registry,"wos_run_conformance",&project_id,json!({"project_id":project_id,"fixture_json":vector["fixture"].to_string(),"clock_start_ms":1234,"resolved_dependencies":[],"required_observations":vector["requiredObservations"],"expected_observations":vector["expectations"]})).await.unwrap();
        for (observed, expected) in [
            (&actual["status"], &vector["expected"]["status"]),
            (&actual["passed"], &vector["expected"]["passed"]),
            (
                &actual["finalSnapshot"]["configuration"],
                &vector["expected"]["configuration"],
            ),
            (
                &actual["finalSnapshot"]["caseState"],
                &vector["expected"]["caseState"],
            ),
        ] {
            assert_eq!(observed, expected, "{}: {actual}", vector["id"]);
        }
        assert_eq!(owner.identity, actual["identity"], "{}", vector["id"]);
        assert_eq!(
            serde_json::to_value(&owner.inputs).unwrap(),
            actual["inputs"],
            "{}",
            vector["id"]
        );
        assert_eq!(
            serde_json::to_value(&owner.final_snapshot).unwrap(),
            actual["finalSnapshot"],
            "{}",
            vector["id"]
        );
        assert_eq!(
            serde_json::to_value(&owner.trace).unwrap(),
            actual["trace"],
            "{}",
            vector["id"]
        );
        let owner_policy = owner.attempts[0].drains[0]
            .policy_evaluation
            .as_ref()
            .unwrap();
        assert_eq!(
            owner_policy.disposition(),
            vector["expected"]["policyDisposition"],
            "{}",
            vector["id"]
        );
        assert_eq!(
            serde_json::to_value(owner_policy).unwrap(),
            actual["attempts"][0]["drains"][0]["policyEvaluation"],
            "{}",
            vector["id"]
        );
        assert_eq!(actual["execution"]["hosted"], false);
        assert!(
            actual["execution"]["agentOutputs"]
                .as_str()
                .unwrap()
                .contains("native policy stage only")
        );
        assert_eq!(
            actual["attempts"][0]["drains"][0]["createdTaskIds"],
            json!([])
        );
        let summary = actual["provenance"]
            .as_array()
            .unwrap()
            .iter()
            .find(|record| record["data"]["exercise"] == "suppliedOutput")
            .unwrap();
        assert_eq!(summary["data"]["executionReadinessAssessed"], false);
        assert_eq!(
            summary["data"]["unexecutedStages"],
            json!([
                "invocation",
                "inputDefinitionValidation",
                "outputDefinitionValidation",
                "humanReview",
                "governedCommit"
            ])
        );
    }
}

#[tokio::test]
async fn typed_authored_and_documented_stored_events_match_owner_dispatch() {
    use wos_conformance::studio_api::{ReplayRequest, prepare_replay, replay};
    let pair: Value = serde_json::from_str(include_str!(
        "../../wos-conformance/fixtures/native-ai-replay/event-shapes.json"
    ))
    .unwrap();
    let mut identities = Vec::new();
    for key in ["candidate", "storedCandidate"] {
        let source = pair[key].clone();
        let owner = replay(
            &prepare_replay(ReplayRequest {
                candidate_bytes: serde_json::to_vec(&source).unwrap(),
                dependencies: vec![],
                fixture_json: pair["fixture"].to_string(),
                clock_start_ms: 1234,
                required_observations: vec!["aiPolicy".into()],
                expectations: vec![],
            })
            .unwrap(),
        )
        .unwrap();
        let mut registry = ProjectRegistry::new();
        let project_id = load(&mut registry, source).await;
        let actual = dispatch(&mut registry, "wos_run_conformance", &project_id,
            json!({"project_id":project_id,"fixture_json":pair["fixture"].to_string(),"clock_start_ms":1234,"required_observations":["aiPolicy"]})
        ).await.unwrap();
        assert_eq!(actual["passed"], true, "{key}: {actual}");
        assert_eq!(
            actual["finalSnapshot"]["configuration"],
            pair["expectedConfiguration"]
        );
        assert_eq!(
            actual["finalSnapshot"]["caseState"],
            pair["expectedCaseState"]
        );
        assert_eq!(actual["identity"], owner.identity);
        assert_eq!(
            actual["inputs"],
            serde_json::to_value(&owner.inputs).unwrap()
        );
        assert_eq!(actual["trace"], serde_json::to_value(&owner.trace).unwrap());
        assert_eq!(
            actual["attempts"][0]["drains"][0]["policyEvaluation"],
            serde_json::to_value(&owner.attempts[0].drains[0].policy_evaluation).unwrap()
        );
        assert_eq!(
            actual["attempts"][0]["drains"][0]["createdTaskIds"],
            json!([])
        );
        identities.push(actual["identity"].clone());
    }
    assert_ne!(identities[0], identities[1]);
}
