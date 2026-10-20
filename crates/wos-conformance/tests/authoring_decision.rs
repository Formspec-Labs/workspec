//! Actual invocation binding and required inventory at the shared decision owner.
use serde_json::{Value, json};
use wos_conformance::studio_api::{self as native, authoring::*};
use wos_lint::studio_api::{ResolvedDependency, ValidationRequest, ValidationScope};
fn candidate() -> Value {
    serde_json::from_str(include_str!(
        "../../wos-lint/tests/fixtures/shared-validation/positive.json"
    ))
    .unwrap()
}
fn request(candidate: &Value) -> native::ReplayRequest {
    native::ReplayRequest{candidate_bytes:serde_json::to_vec(candidate).unwrap(),dependencies:vec![],
        fixture_json:json!({"id":"independent","rule":"control","description":"Reviewed typed event","documents":{},"event_sequence":[{"event":"decide","actor":"reviewer"}],"expected_transitions":[{"from":"review","target":"done","event":"decide"}]}).to_string(),clock_start_ms:0,required_observations:vec![],expectations:vec![]}
}
fn static_request<'a>(
    candidate: &'a Value,
    dependencies: &'a [ResolvedDependency],
) -> ValidationRequest<'a> {
    ValidationRequest {
        candidate,
        dependencies,
        scope: ValidationScope::Project,
    }
}
fn decide(
    candidate: &Value,
    check: &StaticCheck,
    ids: &[String],
    replays: &[ReplayEvidence<'_>],
    additional: &[AdditionalRequirement<'_>],
) -> AuthoringDecision {
    decide_authoring(
        AuthoringRequirements {
            required_scenarios: ids,
            draft_only: false,
        },
        StaticEvidence::Completed {
            request: static_request(candidate, &[]),
            check,
        },
        replays,
        additional,
    )
}
#[test]
fn actual_pass_failure_and_missing_required_are_distinct() {
    let candidate = candidate();
    let check = validate_static(static_request(&candidate, &[])).unwrap();
    let prepared = native::prepare_replay(request(&candidate)).unwrap();
    let result = native::replay(&prepared).unwrap();
    assert!(result.passed);
    assert_eq!(
        result.final_snapshot.as_ref().unwrap().configuration,
        vec!["done"]
    );
    let selection = ReplaySelection::native(&prepared);
    let actual = ReplayCheck::native(&prepared, &result);
    let ids = vec!["reviewed".into()];
    let records = [ReplayEvidence {
        id: "reviewed",
        outcome: ReplayOutcome::Completed {
            selection: &selection,
            check: &actual,
        },
    }];
    assert_eq!(
        decide(&candidate, &check, &ids, &records, &[]),
        AuthoringDecision {
            status: AuthoringStatus::Verified,
            reasons: vec![]
        }
    );
    let missing = decide(&candidate, &check, &ids, &[], &[]);
    assert_eq!(missing.status, AuthoringStatus::Blocked);
    assert_eq!(missing.reasons[0].code, "missing-required-result");
    let mut changed = candidate.clone();
    changed["lifecycle"]["states"]["review"]["transitions"][0]["guard"] = json!("false");
    let current = validate_static(static_request(&changed, &[])).unwrap();
    let prepared = native::prepare_replay(request(&changed)).unwrap();
    let failed = native::replay(&prepared).unwrap();
    assert!(!failed.passed);
    let current_selection = ReplaySelection::native(&prepared);
    let failed_check = ReplayCheck::native(&prepared, &failed);
    let records = [ReplayEvidence {
        id: "reviewed",
        outcome: ReplayOutcome::Completed {
            selection: &current_selection,
            check: &failed_check,
        },
    }];
    assert_eq!(
        decide(&changed, &current, &ids, &records, &[]).reasons[0].code,
        "failed"
    );
    let records = [ReplayEvidence {
        id: "reviewed",
        outcome: ReplayOutcome::Completed {
            selection: &current_selection,
            check: &actual,
        },
    }];
    assert_eq!(
        decide(&changed, &current, &ids, &records, &[]).reasons[0].code,
        "selection-mismatch"
    );
}
#[test]
fn same_version_static_dependency_mutation_rejects_old_actual_check_with_distinct_replay_selection()
{
    let candidate = candidate();
    let dependencies:Vec<ResolvedDependency>=serde_json::from_value(json!([
        {"role":"definition","url":"urn:test:definition","version":"1.2.0","document_json":{"$formspec":"1.0","url":"urn:test:definition","version":"1.2.0","status":"draft","title":"Selected Definition","items":[{"key":"name","type":"field","dataType":"string","label":"Name"}]}},
        {"role":"mapping","url":"urn:test:mapping","version":"3.0.0","definition":{"url":"urn:test:definition","version":"1.2.0"},"document_json":{"$formspecMapping":"1.0","version":"3.0.0","definitionRef":"urn:test:definition","definitionVersion":"^1.0.0","targetSchema":{"format":"json"},"rules":[{"sourcePath":"name","targetPath":"name","transform":"preserve"}]}}
    ])).unwrap();
    let old = validate_static(static_request(&candidate, &dependencies)).unwrap();
    assert_eq!(old.report().status, wos_lint::studio_api::CheckStatus::Pass);
    let prepared = native::prepare_replay(request(&candidate)).unwrap();
    let result = native::replay(&prepared).unwrap();
    let selection = ReplaySelection::native(&prepared);
    let check = ReplayCheck::native(&prepared, &result);
    let ids = vec!["separate-replay".into()];
    let records = [ReplayEvidence {
        id: "separate-replay",
        outcome: ReplayOutcome::Completed {
            selection: &selection,
            check: &check,
        },
    }];
    let run = |dependencies: &[ResolvedDependency], check: &StaticCheck| {
        decide_authoring(
            AuthoringRequirements {
                required_scenarios: &ids,
                draft_only: false,
            },
            StaticEvidence::Completed {
                request: static_request(&candidate, dependencies),
                check,
            },
            &records,
            &[],
        )
    };
    assert_eq!(run(&dependencies, &old).status, AuthoringStatus::Verified);
    let mut changed = dependencies.clone();
    changed[1].document_json["rules"][0]["sourcePath"] = json!("missing");
    let stale = run(&changed, &old);
    assert_eq!(stale.status, AuthoringStatus::Blocked);
    assert_eq!(stale.reasons[0].code, "selection-mismatch");
    let fresh = validate_static(static_request(&candidate, &changed)).unwrap();
    assert_ne!(fresh.identity(), old.identity());
    assert_eq!(
        fresh.report().status,
        wos_lint::studio_api::CheckStatus::Fail
    );
    assert_eq!(run(&changed, &fresh).reasons[0].code, "failed");
}
#[test]
fn explicit_absence_duplicate_inventory_optional_failure_and_required_gap_remain_visible() {
    let candidate = candidate();
    let check = validate_static(static_request(&candidate, &[])).unwrap();
    let ids = vec!["required".into()];
    for outcome in [
        ReplayOutcome::Error("actual preparation refused"),
        ReplayOutcome::NotRun("disabled"),
    ] {
        assert_eq!(
            decide(
                &candidate,
                &check,
                &ids,
                &[ReplayEvidence {
                    id: "required",
                    outcome
                }],
                &[]
            )
            .status,
            AuthoringStatus::Blocked
        );
    }
    let records = [
        ReplayEvidence {
            id: "required",
            outcome: ReplayOutcome::NotRun("disabled"),
        },
        ReplayEvidence {
            id: "required",
            outcome: ReplayOutcome::NotRun("disabled"),
        },
    ];
    assert_eq!(
        decide(&candidate, &check, &ids, &records, &[]).reasons[0].code,
        "duplicate-result"
    );
    assert_eq!(
        decide(
            &candidate,
            &check,
            &["required".into(), "required".into()],
            &records,
            &[]
        )
        .status,
        AuthoringStatus::Blocked
    );
    let prepared = native::prepare_replay(request(&candidate)).unwrap();
    let result = native::replay(&prepared).unwrap();
    let selection = ReplaySelection::native(&prepared);
    let actual = ReplayCheck::native(&prepared, &result);
    let records = [
        ReplayEvidence {
            id: "required",
            outcome: ReplayOutcome::Completed {
                selection: &selection,
                check: &actual,
            },
        },
        ReplayEvidence {
            id: "optional",
            outcome: ReplayOutcome::NotRun("optional unavailable"),
        },
    ];
    let optional = decide(&candidate, &check, &ids, &records, &[]);
    assert_eq!(optional.status, AuthoringStatus::Verified);
    assert!(!optional.reasons[0].blocking);
    let additional = [AdditionalRequirement {
        id: "review-gap",
        source: "urn:studio:review",
        required: true,
        outcome: RequirementOutcome::AcceptedGap,
    }];
    let gap = decide(&candidate, &check, &ids, &records, &additional);
    assert_eq!(gap.status, AuthoringStatus::Blocked);
    assert_eq!(gap.reasons[1].code, "accepted-gap");
    let draft = decide_authoring(
        AuthoringRequirements {
            required_scenarios: &[],
            draft_only: true,
        },
        StaticEvidence::Completed {
            request: static_request(&candidate, &[]),
            check: &check,
        },
        &[],
        &[],
    );
    assert_eq!(draft.status, AuthoringStatus::Draft);
    assert_eq!(draft.reasons[0].code, "static-only-draft");
    assert_eq!(
        decide(&candidate, &check, &[], &[], &[]).reasons[0].code,
        "no-required-scenarios"
    );
}

#[test]
fn specialized_actual_observation_keeps_its_selector_expectation_and_outcome() {
    let mut candidate = candidate();
    candidate["lifecycle"]["states"]["review"]["transitions"][0]["guard"] = json!("true");
    let static_check = validate_static(static_request(&candidate, &[])).unwrap();
    let prepared = native::prepare_replay(request(&candidate)).unwrap();
    let result = native::replay(&prepared).unwrap();
    assert!(result.passed);
    let selection = ReplaySelection::native(&prepared);
    let actual = ReplayCheck::native(&prepared, &result);
    let records = [ReplayEvidence {
        id: "required",
        outcome: ReplayOutcome::Completed {
            selection: &selection,
            check: &actual,
        },
    }];
    let selector:native::NativeSelector=serde_json::from_value(json!({"attempt":{"eventIndex":0,"kind":"event","occurrence":0},"subject":{"kind":"decision","transition":{"statePointer":"/lifecycle/states/review","transitionIndex":0},"evaluationOccurrence":0}})).unwrap();
    let expected = native::ObservationExpectation {
        pointer: "/result".into(),
        expected: json!(true),
    };
    let selected =
        ComparisonSelection::observation_native(&prepared, &selector, &expected).unwrap();
    let (comparison, proof) = check_observation(&result, &selector, &expected).unwrap();
    assert!(matches!(comparison, native::NativeComparison::Pass));
    let ids = vec!["required".into()];
    let additional = |selected: &ComparisonSelection, proof: &ComparisonCheck| {
        decide(
            &candidate,
            &static_check,
            &ids,
            &records,
            &[AdditionalRequirement {
                id: "decision",
                source: "urn:studio:assertion",
                required: true,
                outcome: RequirementOutcome::Comparison {
                    selection: selected,
                    check: proof,
                },
            }],
        )
    };
    assert_eq!(
        additional(&selected, &proof).status,
        AuthoringStatus::Verified
    );
    let wrong = native::ObservationExpectation {
        pointer: "/result".into(),
        expected: json!(false),
    };
    let current = ComparisonSelection::observation_native(&prepared, &selector, &wrong).unwrap();
    assert_eq!(
        additional(&current, &proof).reasons[0].code,
        "selection-mismatch"
    );
    let (comparison, failed) = check_observation(&result, &selector, &wrong).unwrap();
    assert!(matches!(comparison, native::NativeComparison::Fail { .. }));
    assert_eq!(additional(&current, &failed).reasons[0].code, "failed");
    let mut missing = selector.clone();
    missing.attempt.event_index = 99;
    let selected = ComparisonSelection::observation_native(&prepared, &missing, &expected).unwrap();
    let (comparison, refused) = check_observation(&result, &missing, &expected).unwrap();
    assert!(matches!(
        comparison,
        native::NativeComparison::Refused { .. }
    ));
    assert_eq!(additional(&selected, &refused).reasons[0].code, "refused");
}

#[test]
fn actual_reference_session_receipt_remains_bound_to_current_preparation() {
    let input: Value =
        serde_json::from_str(include_str!("fixtures/reference-due-process-replay.json")).unwrap();
    let request: native::ReplayRequest = serde_json::from_value(input["request"].clone()).unwrap();
    let adapters: native::ReferenceReplayAdapters =
        serde_json::from_value(input["adapters"].clone()).unwrap();
    let prepared = native::prepare_reference_replay(request.clone(), adapters.clone()).unwrap();
    let selection = ReplaySelection::reference(&prepared);
    let session = native::begin_reference_replay(prepared).unwrap();
    assert!(session.result().passed);
    let actual = ReplayCheck::session(&session);
    let candidate: Value = serde_json::from_slice(&request.candidate_bytes).unwrap();
    let static_check = validate_static(static_request(&candidate, &[])).unwrap();
    let ids = vec!["reference".into()];
    let records = [ReplayEvidence {
        id: "reference",
        outcome: ReplayOutcome::Completed {
            selection: &selection,
            check: &actual,
        },
    }];
    let original = decide(&candidate, &static_check, &ids, &records, &[]);
    assert!(
        !original
            .reasons
            .iter()
            .any(|reason| reason.check_id == "reference")
    );
    let mut changed = request;
    changed.clock_start_ms += 1;
    let current = native::prepare_reference_replay(changed, adapters).unwrap();
    let current_selection = ReplaySelection::reference(&current);
    let records = [ReplayEvidence {
        id: "reference",
        outcome: ReplayOutcome::Completed {
            selection: &current_selection,
            check: &actual,
        },
    }];
    let stale = decide(&candidate, &static_check, &ids, &records, &[]);
    assert!(
        stale
            .reasons
            .iter()
            .any(|reason| reason.check_id == "reference" && reason.code == "selection-mismatch")
    );
}
