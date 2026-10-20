// Rust guideline compliant 2026-02-21
//! Authored stable recipes execute the actual signed governed native journey.
use serde_json::{Value, json};
use wos_conformance::studio_api::{
    ReferenceReplayAdapters, ReplayRequest, prepare_reference_replay, replay_reference,
};
fn inputs() -> (ReplayRequest, ReferenceReplayAdapters) {
    let original: Value =
        serde_json::from_str(include_str!("fixtures/reference-due-process-replay.json")).unwrap();
    (
        serde_json::from_value(original["request"].clone()).unwrap(),
        serde_json::from_value(original["adapters"].clone()).unwrap(),
    )
}

#[test]
fn only_exact_empty_custody_forms_admit_and_preserve_distinct_bytes() {
    let (request, adapters) = inputs();
    let mut identities = Vec::new();
    for custody in [json!({}), json!({"anchorRequirements": []})] {
        let mut request = request.clone();
        let mut candidate: Value = serde_json::from_slice(&request.candidate_bytes).unwrap();
        candidate["custody"] = custody;
        request.candidate_bytes = serde_json::to_vec(&candidate).unwrap();
        let prepared = prepare_reference_replay(request, adapters.clone()).unwrap();
        identities.push(prepared.identity().to_owned());
        assert!(replay_reference(&prepared).unwrap().passed);
    }
    assert_ne!(identities[0], identities[1]);
    for custody in [
        Value::Null,
        json!({"anchorRequirements": null}),
        json!({"anchorRequirements": {}}),
        json!({"anchorRequirements": [{"on": "effect", "anchor": "required"}]}),
        json!({"trustProfileRef": ""}),
        json!({"exportBundleRef": ""}),
        json!({"unknown": []}),
    ] {
        let mut request = request.clone();
        let mut candidate: Value = serde_json::from_slice(&request.candidate_bytes).unwrap();
        candidate["custody"] = custody;
        request.candidate_bytes = serde_json::to_vec(&candidate).unwrap();
        assert!(prepare_reference_replay(request, adapters.clone()).is_err());
    }
}

fn service_inputs() -> (ReplayRequest, ReferenceReplayAdapters) {
    let (mut request, mut adapters) = inputs();
    let mut candidate: Value = serde_json::from_slice(&request.candidate_bytes).unwrap();
    candidate["caseFile"]["fields"]["external"] = json!({"type":"number"});
    candidate["caseFile"]["fields"]["fraud"] = json!({"type":"string"});
    let actions = candidate["lifecycle"]["states"]["pending"]["onEntry"]
        .as_array_mut()
        .unwrap();
    actions.push(json!({"action":"invokeService","serviceRef":"income-service","idempotencyKey":"income-one"}));
    actions.push(json!({"action":"invokeService","serviceRef":"fraud-policy"}));
    candidate["extensions"] = json!({"x-wos-reference-invocations":{"profile":"referenceInvocations/1","bindings":[
        {"id":"income-service","kind":"requestResponse","descriptor":{"kind":"openapi","documentRef":"urn:fixture:income-openapi","operationRef":"verifyIncome"},"inputMapping":{"probe":"42"},"outputBinding":{"caseFile.external":"$.verified"}},
        {"id":"fraud-policy","kind":"opaPolicy","descriptor":{"kind":"opa","endpoint":"urn:fixture:fraud-opa"},"inputMapping":{"income":"caseFile.income"},"outputBinding":{"caseFile.fraud":"$.decision"}}
    ]}});
    request.candidate_bytes = serde_json::to_vec(&candidate).unwrap();
    adapters.invocations=serde_json::from_value(json!([
        {"id":"income-fixture","serviceRef":"income-service","expectedInput":{"probe":42},"response":{"verified":145},"maxInvocations":1},
        {"id":"fraud-fixture","serviceRef":"fraud-policy","expectedInput":{"income":150},"response":{"result":true,"reasons":[],"obligations":[]},"maxInvocations":1}
    ])).unwrap();
    (request, adapters)
}

#[test]
fn selected_service_and_opa_execute_native_handlers_and_retain_actual_associations() {
    let (request, adapters) = service_inputs();
    let prepared = prepare_reference_replay(request, adapters).unwrap();
    let actual = replay_reference(&prepared).unwrap();
    assert!(actual.passed, "{actual:#?}");
    let saved = actual.final_snapshot.unwrap();
    assert_eq!(saved.case_state["external"], json!(145));
    assert_eq!(saved.case_state["fraud"], json!("allow"));
    assert_eq!(saved.step_results.len(), 2);
    let association = saved
        .step_results
        .iter()
        .find(|step| step.service_ref == "income-service")
        .unwrap()
        .invocation_association
        .as_ref()
        .unwrap();
    assert_eq!(association.input, json!({"probe":42}));
    assert!(!association.adapter_identity.is_empty());
}

#[test]
fn selected_service_cannot_grant_capabilities_or_write_protected_effects() {
    let (request, adapters) = service_inputs();
    for mutate in 0..4 {
        let mut request = request.clone();
        let mut adapters = adapters.clone();
        let mut candidate: Value = serde_json::from_slice(&request.candidate_bytes).unwrap();
        match mutate {
            0 => {
                candidate["extensions"]["x-wos-reference-invocations"]["bindings"][0]["outputBinding"] =
                    json!({"caseFile.level":"$.verified"})
            }
            1 => adapters.invocations[1].response = json!({"decision":"permit"}),
            2 => {
                adapters.invocations.pop();
            }
            _ => {
                candidate["extensions"]["x-wos-reference-invocations"]["bindings"][0]["descriptor"]
                    ["kind"] = json!("opa")
            }
        }
        request.candidate_bytes = serde_json::to_vec(&candidate).unwrap();
        assert!(
            prepare_reference_replay(request, adapters).is_err(),
            "mutation {mutate}"
        );
    }
    let (request, mut adapters) = service_inputs();
    adapters.invocations[0].expected_input = json!({"probe":43});
    let actual = replay_reference(&prepare_reference_replay(request, adapters).unwrap()).unwrap();
    assert!(!actual.passed);
    assert!(
        actual
            .final_snapshot
            .is_none_or(|saved| saved.case_state.get("external").is_none_or(Value::is_null))
    );
}

#[test]
fn declared_native_time_advances_retain_original_bytes_and_execute_real_clock() {
    use wos_conformance::studio_api::NativeTimeAdvance;
    let (mut request, mut adapters) = inputs();
    let mut fixture: Value = serde_json::from_str(&request.fixture_json).unwrap();
    fixture["event_sequence"][4]
        .as_object_mut()
        .unwrap()
        .remove("delay");
    request.fixture_json = fixture.to_string();
    let original = request.fixture_json.clone();
    adapters.time_advances.push(NativeTimeAdvance {
        before_event_index: 4,
        duration: "P10D".into(),
    });
    let prepared = prepare_reference_replay(request.clone(), adapters.clone()).unwrap();
    assert_eq!(prepared.request().fixture_json, original);
    let actual = replay_reference(&prepared).unwrap();
    assert!(actual.passed, "{actual:#?}");
    assert!(
        actual
            .inputs
            .iter()
            .any(|input| input.role == "loweredFixture")
    );
    assert!(
        actual
            .attempts
            .iter()
            .any(|attempt| attempt.kind == "clock" && attempt.clock_ms == 864_000_000)
    );
    for bad in ["P0D", "P01D", "PT24H", "P99999999999999999999999D"] {
        adapters.time_advances[0].duration = bad.into();
        assert!(prepare_reference_replay(request.clone(), adapters.clone()).is_err());
    }
    adapters.time_advances[0].duration = "P10D".into();
    adapters.time_advances[0].before_event_index = 8;
    assert!(prepare_reference_replay(request.clone(), adapters.clone()).is_err());
    adapters.time_advances[0].before_event_index = 4;
    adapters
        .time_advances
        .push(adapters.time_advances[0].clone());
    assert!(prepare_reference_replay(request, adapters).is_err());
    let (request, mut adapters) = inputs();
    adapters.time_advances.push(NativeTimeAdvance {
        before_event_index: 4,
        duration: "P10D".into(),
    });
    assert!(prepare_reference_replay(request, adapters).is_err());
}

#[test]
fn actual_subject_selectors_require_native_facts_and_preserve_task_artifact() {
    use wos_conformance::studio_api::{
        NativeComparison, NativeSelector, ObservationExpectation, compare_observation,
    };
    let (request, adapters) = inputs();
    let actual = replay_reference(&prepare_reference_replay(request, adapters).unwrap()).unwrap();
    let selector:NativeSelector=serde_json::from_value(json!({"attempt":{"eventIndex":1,"kind":"taskSubmission","occurrence":0},"subject":{"kind":"task","taskRef":"supervisor-signature","taskOccurrence":0}})).unwrap();
    assert!(matches!(
        compare_observation(
            &actual,
            &selector,
            &ObservationExpectation {
                pointer: "/status".into(),
                expected: json!("completed")
            }
        ),
        NativeComparison::Pass
    ));
    let NativeComparison::Fail { difference } = compare_observation(
        &actual,
        &selector,
        &ObservationExpectation {
            pointer: "/status".into(),
            expected: json!("invented-status"),
        },
    ) else {
        panic!("independent wrong expectation must fail");
    };
    assert_eq!(difference.actual, Some(json!("completed")));
    assert_eq!(difference.expected, json!("invented-status"));
    assert!(matches!(
        compare_observation(
            &actual,
            &selector,
            &ObservationExpectation {
                pointer: "/acceptedArtifact/response/data/affirmed".into(),
                expected: json!(true)
            }
        ),
        NativeComparison::Pass
    ));
    let mut missing = selector.clone();
    missing.attempt.occurrence = 1;
    assert!(matches!(
        compare_observation(
            &actual,
            &missing,
            &ObservationExpectation {
                pointer: "/status".into(),
                expected: json!("completed")
            }
        ),
        NativeComparison::Refused { .. }
    ));
    let notice:NativeSelector=serde_json::from_value(json!({"attempt":{"eventIndex":2,"kind":"governanceOperation","occurrence":0},"subject":{"kind":"notice","target":{"bindingRef":"adverse","firingOccurrence":0}}})).unwrap();
    assert!(matches!(
        compare_observation(
            &actual,
            &notice,
            &ObservationExpectation {
                pointer: "/status".into(),
                expected: json!("qualified")
            }
        ),
        NativeComparison::Pass
    ));
    let wrong_kind:NativeSelector=serde_json::from_value(json!({"attempt":{"eventIndex":2,"kind":"event","occurrence":0},"subject":{"kind":"notice","target":{"bindingRef":"adverse","firingOccurrence":0}}})).unwrap();
    assert!(matches!(
        compare_observation(
            &actual,
            &wrong_kind,
            &ObservationExpectation {
                pointer: "/status".into(),
                expected: json!("qualified")
            }
        ),
        NativeComparison::Refused { .. }
    ));
}

#[test]
fn recipe_and_adapter_forgery_refuse_before_execution() {
    let (request, adapters) = inputs();
    for mutate in 0..4 {
        let mut request = request.clone();
        let mut adapters = adapters.clone();
        let mut fixture: Value = serde_json::from_str(&request.fixture_json).unwrap();
        match mutate {
            0 => fixture["event_sequence"][2]["governance_recipe"]["target"] = Value::Null,
            1 => {
                fixture["event_sequence"][1]["task_submission"]["signature_recipe"]["signature"]["verified"] =
                    json!(true)
            }
            2 => {
                adapters.fixture_signing_capabilities[0].receipt_pkcs8_base64 = adapters
                    .fixture_signing_capabilities[0]
                    .pkcs8_base64
                    .clone()
            }
            _ => {
                fixture["event_sequence"][1]["task_submission"]["response"]["data"]["signatureTaskId"] =
                    json!("predicted-task")
            }
        }
        request.fixture_json = fixture.to_string();
        assert!(
            prepare_reference_replay(request, adapters).is_err(),
            "mutation {mutate}"
        );
    }
}
#[test]
fn actual_signed_reference_recipe_reaches_applied_after_real_hearing() {
    let original: Value =
        serde_json::from_str(include_str!("fixtures/reference-due-process-replay.json")).unwrap();
    let request: ReplayRequest = serde_json::from_value(original["request"].clone()).unwrap();
    let adapters: ReferenceReplayAdapters =
        serde_json::from_value(original["adapters"].clone()).unwrap();
    let prepared = prepare_reference_replay(request, adapters).unwrap();
    let actual = replay_reference(&prepared).unwrap();
    if let Ok(path) = std::env::var("WOS_REFERENCE_REPLAY_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&actual).unwrap()).unwrap();
    }
    assert!(
        actual.passed,
        "{}",
        serde_json::to_string_pretty(&actual).unwrap()
    );
    assert_eq!(
        actual.execution["notifications"],
        "selected deterministic local fixture transport; native qualified-receipt admission; no hosted/provider delivery qualification"
    );
    assert_eq!(actual.execution["hosted"], false);
    assert_eq!(actual.execution["durable"], false);
    let saved = actual.final_snapshot.unwrap().governance_state;
    assert_eq!(
        saved["dueProcess"]["notices"][0]["status"],
        json!("qualified")
    );
    assert_eq!(
        saved["dueProcess"]["appeals"][0]["hearing"]["reviewerHuman"],
        json!("actor:human:reviewer")
    );
    assert!(actual.attempts[1].recipe_resolution.as_ref().unwrap()["signedResponse"]["authoredSignatures"].is_array());
}

#[test]
fn canonical_audit_project_lint_and_actual_replay_agree() {
    use wos_lint::studio_api::{self as lint, DependencyRole, ResolvedDependency};
    let (request, adapters) = inputs();
    let candidate: Value = serde_json::from_slice(&request.candidate_bytes).unwrap();
    let dependencies: Vec<_> = request
        .dependencies
        .iter()
        .map(|selected| ResolvedDependency {
            role: match selected.role.as_str() {
                "definition" => DependencyRole::Definition,
                "delivery" => DependencyRole::Delivery,
                _ => panic!("unexpected fixture role"),
            },
            url: selected.url.clone(),
            version: selected.version.clone().unwrap(),
            document_json: serde_json::from_slice(&selected.bytes).unwrap(),
            definition: None,
        })
        .collect();
    let forbidden = ["G-004", "G-005", "G-014", "G-015", "G-063"];
    let report = lint::validate(lint::ValidationRequest {
        candidate: &candidate,
        dependencies: &dependencies,
        scope: lint::ValidationScope::Project,
    });
    assert!(
        !report
            .diagnostics
            .iter()
            .any(|d| forbidden.contains(&d.rule_id)),
        "{report:#?}"
    );
    assert_eq!(report.status, lint::CheckStatus::Pass, "{report:#?}");
    for (pointer, rule) in [
        (
            "/governance/dueProcess/adverseDecisionPolicy/explanationLevel",
            "G-004",
        ),
        (
            "/governance/dueProcess/execution/determinations/0/explanation/positive",
            "G-005",
        ),
        (
            "/governance/dueProcess/execution/determinations/0/explanation/negative",
            "G-005",
        ),
        ("/governance/audit/reasoningTier/requiredForTags", "G-014"),
        (
            "/governance/audit/counterfactualTier/requiredForTags",
            "G-015",
        ),
    ] {
        let mut changed = candidate.clone();
        *changed.pointer_mut(pointer).unwrap() = Value::Null;
        let report = lint::validate(lint::ValidationRequest {
            candidate: &changed,
            dependencies: &dependencies,
            scope: lint::ValidationScope::Project,
        });
        assert!(
            report.diagnostics.iter().any(|d| d.rule_id == rule),
            "missing {rule}: {report:#?}"
        );
    }
    let without_delivery = &dependencies[..dependencies.len() - 1];
    let report = lint::validate(lint::ValidationRequest {
        candidate: &candidate,
        dependencies: without_delivery,
        scope: lint::ValidationScope::Project,
    });
    assert!(report.diagnostics.iter().any(|d| d.rule_id == "G-063"));
    assert_eq!(report.scope, lint::ValidationScope::Project);
    let prepared = prepare_reference_replay(request, adapters).unwrap();
    assert!(replay_reference(&prepared).unwrap().passed);
}

#[test]
fn canonical_audit_omission_and_unsupported_requirements_refuse_direct_replay() {
    let (request, adapters) = inputs();
    for pointer in [
        "/governance/audit",
        "/governance/audit/reasoningTier",
        "/governance/audit/counterfactualTier",
        "/governance/dueProcess/execution/determinations/0/explanation/protectedCharacteristics",
    ] {
        let mut request = request.clone();
        let mut candidate: Value = serde_json::from_slice(&request.candidate_bytes).unwrap();
        *candidate.pointer_mut(pointer).unwrap() = Value::Null;
        request.candidate_bytes = serde_json::to_vec(&candidate).unwrap();
        assert!(
            prepare_reference_replay(request, adapters.clone()).is_err(),
            "accepted missing {pointer}"
        );
    }
}

#[test]
fn canonical_delivery_selection_and_actual_template_checks_refuse_substitution() {
    use wos_lint::studio_api::{self as lint, DependencyRole, ResolvedDependency};
    let (request, adapters) = inputs();
    let delivery_index = request
        .dependencies
        .iter()
        .position(|d| d.role == "delivery")
        .unwrap();
    for mutation in 0..8 {
        let mut request = request.clone();
        let mut adapters = adapters.clone();
        let mut delivery: Value =
            serde_json::from_slice(&request.dependencies[delivery_index].bytes).unwrap();
        match mutation {
            0 => {
                request.dependencies.remove(delivery_index);
            }
            1 => {
                delivery["targetWorkflow"] = json!("urn:fixture:other-workflow");
            }
            2 => {
                adapters.templates[0]
                    .templates
                    .get_mut("notice")
                    .unwrap()
                    .sections[0]
                    .content = Some("Substituted actual template".into());
            }
            3 => {
                adapters.templates.push(adapters.templates[0].clone());
            }
            4 => {
                delivery["calendar"] = json!({"timezone":"UTC","workWeek":["monday"]});
            }
            5 => {
                request
                    .dependencies
                    .push(request.dependencies[delivery_index].clone());
            }
            6 => {
                let mut overlap = request.dependencies[delivery_index].clone();
                overlap.role = "notifications".into();
                overlap.url = "urn:fixture:overlap".into();
                request.dependencies.push(overlap);
            }
            _ => {
                delivery["notificationTemplates"] = delivery["notifications"]["templates"].clone();
                delivery.as_object_mut().unwrap().remove("notifications");
            }
        }
        if mutation != 0 {
            request.dependencies[delivery_index].bytes = serde_json::to_vec(&delivery).unwrap();
        }
        assert!(
            prepare_reference_replay(request, adapters).is_err(),
            "accepted delivery mutation {mutation}"
        );
    }
    for (mutation, rule) in [(0, "G-062"), (1, "G-065"), (2, "G-063")] {
        let mut delivery: Value =
            serde_json::from_slice(&request.dependencies[delivery_index].bytes).unwrap();
        if mutation == 0 {
            delivery["notifications"]["templates"]["notice"]["sections"]
                .as_array_mut()
                .unwrap()
                .remove(0);
        }
        if mutation == 1 {
            let section = delivery["notifications"]["templates"]["notice"]["sections"][0].clone();
            delivery["notifications"]["templates"]["notice"]["sections"]
                .as_array_mut()
                .unwrap()
                .push(section);
        }
        if mutation == 2 {
            delivery["targetWorkflow"] = json!("urn:fixture:other-workflow");
        }
        let candidate: Value = serde_json::from_slice(&request.candidate_bytes).unwrap();
        let dependencies = [ResolvedDependency {
            role: DependencyRole::Delivery,
            url: request.dependencies[delivery_index].url.clone(),
            version: "1.0.0".into(),
            document_json: delivery,
            definition: None,
        }];
        let report = lint::validate(lint::ValidationRequest {
            candidate: &candidate,
            dependencies: &dependencies,
            scope: lint::ValidationScope::Project,
        });
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule_id == rule),
            "missing {rule}: {report:#?}"
        );
    }
}

#[test]
fn matching_delivery_and_adapter_cannot_bypass_required_section_checks() {
    for duplicate in [false, true] {
        let (mut request, mut adapters) = inputs();
        let delivery_index = request
            .dependencies
            .iter()
            .position(|dependency| dependency.role == "delivery")
            .unwrap();
        let mut delivery: Value =
            serde_json::from_slice(&request.dependencies[delivery_index].bytes).unwrap();
        let sections = &mut delivery["notifications"]["templates"]["notice"]["sections"];
        if duplicate {
            let section = sections[0].clone();
            sections.as_array_mut().unwrap().push(section);
        } else {
            sections.as_array_mut().unwrap().remove(0);
        }
        adapters.templates[0] = serde_json::from_value(delivery["notifications"].clone()).unwrap();
        request.dependencies[delivery_index].bytes = serde_json::to_vec(&delivery).unwrap();
        let error = prepare_reference_replay(request, adapters)
            .unwrap_err()
            .to_string();
        let expected = if duplicate { "G-065" } else { "G-062" };
        assert!(
            error.contains(expected),
            "wrong direct refusal for {expected}: {error}"
        );
    }
}

fn project_report(request: &ReplayRequest) -> wos_lint::studio_api::ValidationReport {
    use wos_lint::studio_api::{self as lint, DependencyRole, ResolvedDependency};
    let candidate: Value = serde_json::from_slice(&request.candidate_bytes).unwrap();
    let dependencies: Vec<_> = request
        .dependencies
        .iter()
        .map(|selected| ResolvedDependency {
            role: match selected.role.as_str() {
                "definition" => DependencyRole::Definition,
                "delivery" => DependencyRole::Delivery,
                _ => panic!("unexpected fixture role"),
            },
            url: selected.url.clone(),
            version: selected.version.clone().unwrap(),
            document_json: serde_json::from_slice(&selected.bytes).unwrap(),
            definition: None,
        })
        .collect();
    lint::validate(lint::ValidationRequest {
        candidate: &candidate,
        dependencies: &dependencies,
        scope: lint::ValidationScope::Project,
    })
}

#[test]
fn canonical_static_governance_failures_agree_at_lint_and_direct_preparation() {
    let (original, adapters) = inputs();
    for (pointer, value, rule) in [
        (
            "/governance/dueProcess/adverseDecisionPolicy/explanationLevel",
            json!("categorical"),
            "G-004",
        ),
        (
            "/governance/dueProcess/adverseDecisionPolicy/counterfactualRequired",
            json!(false),
            "G-005",
        ),
        (
            "/governance/dueProcess/execution/determinations/0/explanation/positive",
            json!([]),
            "G-005",
        ),
        (
            "/governance/dueProcess/execution/determinations/0/explanation/negative",
            json!([]),
            "G-005",
        ),
        ("/governance/audit/reasoningTier", Value::Null, "G-014"),
        ("/governance/audit/counterfactualTier", Value::Null, "G-015"),
        (
            "/governance/audit/reasoningTier/requiredForTags",
            json!([]),
            "G-014",
        ),
        (
            "/governance/audit/reasoningTier/requiredForTags",
            json!(["determination", "determination"]),
            "G-014",
        ),
        (
            "/governance/audit/counterfactualTier/requiredForTags",
            json!(["adverse-decision", "absent"]),
            "G-015",
        ),
        (
            "/governance/audit/counterfactualTier/requireProtectedCharacteristics",
            json!(false),
            "G-015",
        ),
    ] {
        let mut request = original.clone();
        let mut candidate: Value = serde_json::from_slice(&request.candidate_bytes).unwrap();
        *candidate.pointer_mut(pointer).unwrap() = value;
        request.candidate_bytes = serde_json::to_vec(&candidate).unwrap();
        let report = project_report(&request);
        assert!(
            report.diagnostics.iter().any(|d| d.rule_id == rule),
            "{pointer}: {report:#?}"
        );
        assert!(
            prepare_reference_replay(request, adapters.clone()).is_err(),
            "direct preparation accepted {pointer}"
        );
    }
}

#[test]
fn state_tags_cannot_satisfy_transition_audit_and_foreach_locations_stay_bound() {
    let (original, adapters) = inputs();
    let mut candidate: Value = serde_json::from_slice(&original.candidate_bytes).unwrap();
    candidate["lifecycle"]["states"]["pending"]["tags"] = json!(["state-only"]);
    candidate["governance"]["audit"]["reasoningTier"]["requiredForTags"] =
        json!(["determination", "state-only"]);
    let mut request = original.clone();
    request.candidate_bytes = serde_json::to_vec(&candidate).unwrap();
    assert!(
        project_report(&request)
            .diagnostics
            .iter()
            .any(|d| d.rule_id == "G-014")
    );
    assert!(prepare_reference_replay(request, adapters.clone()).is_err());
    let mut candidate: Value = serde_json::from_slice(&original.candidate_bytes).unwrap();
    candidate["lifecycle"]["states"]["future/~"] = json!({"type":"foreach","collection":"[1]","body":{"type":"compound","initialState":"inside/~","states":{"inside/~":{"type":"atomic","transitions":[{"event":"future","target":"done","tags":["determination","adverse-decision"]}]},"done":{"type":"final"}}}});
    let typed: wos_core::KernelDocument = serde_json::from_value(candidate.clone()).unwrap();
    let index = wos_core::model::governance_checks::LifecycleIndex::new(&typed);
    assert!(
        index
            .transition_locations("determination")
            .iter()
            .any(|location| location.state_pointer
                == "/lifecycle/states/future~1~0/body/states/inside~1~0")
    );
    let mut request = original;
    request.candidate_bytes = serde_json::to_vec(&candidate).unwrap();
    assert!(
        !project_report(&request)
            .diagnostics
            .iter()
            .any(|d| ["G-014", "G-015"].contains(&d.rule_id))
    );
    assert!(
        prepare_reference_replay(request, adapters).is_err(),
        "unselected ForEach determination admitted"
    );
}

#[test]
fn selected_profile_keeps_explanation_requirements_for_nonrights_or_absent_impact() {
    let (original, adapters) = inputs();
    for impact in [Some(json!("informational")), None] {
        let mut candidate: Value = serde_json::from_slice(&original.candidate_bytes).unwrap();
        if let Some(impact) = impact {
            candidate["impactLevel"] = impact;
        } else {
            candidate.as_object_mut().unwrap().remove("impactLevel");
        }
        let mut request = original.clone();
        request.candidate_bytes = serde_json::to_vec(&candidate).unwrap();
        assert!(prepare_reference_replay(request, adapters.clone()).is_ok());
        candidate["governance"]["dueProcess"]["adverseDecisionPolicy"]["explanationLevel"] =
            json!("categorical");
        request = original.clone();
        request.candidate_bytes = serde_json::to_vec(&candidate).unwrap();
        assert!(
            !project_report(&request)
                .diagnostics
                .iter()
                .any(|d| d.rule_id == "G-004")
        );
        assert!(prepare_reference_replay(request, adapters.clone()).is_err());
    }
}
