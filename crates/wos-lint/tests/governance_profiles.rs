// Rust guideline compliant 2026-02-21
//! Declaration checks distinguish general obligations from selected execution.
use serde_json::{Value, json};
use wos_core::KernelDocument;
use wos_core::model::governance_checks::{
    GovernanceIssueKind, LifecycleIndex, check_governance, check_reference_governance,
};
use wos_lint::studio_api::{
    CheckStatus, DependencyRole, ResolvedDependency, ValidationRequest, ValidationScope, validate,
};
#[path = "../../wos-conformance/tests/support/local_schemas.rs"]
mod local_schemas;

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/shared-validation/rights-human.json")).unwrap()
}

#[test]
fn general_explanation_obligations_do_not_select_reference_execution() {
    let input = fixture();
    let original = input.clone();
    let kernel: KernelDocument = serde_json::from_value(input.clone()).unwrap();
    let checks = check_governance(&kernel, &input["governance"], &LifecycleIndex::new(&kernel));
    assert!(checks.issues.is_empty(), "{:?}", checks.issues);
    let delivery = ResolvedDependency {
        role: DependencyRole::Delivery,
        url: "urn:test:human-explanation:delivery".into(),
        version: "1.0.0".into(),
        document_json: json!({
            "$wosDelivery": "1.0",
            "targetWorkflow": "urn:test:human-explanation",
            "version": "1.0.0",
            "notifications": {
                "templates": {
                    "adverse-decision-notice": {
                        "category": "adverse-decision",
                        "sections": [
                            {"id": "determination", "contentType": "structured", "content": "d"},
                            {"id": "reasons", "contentType": "structured", "content": "r"},
                            {"id": "appealRights", "contentType": "appeal-rights", "content": "a"},
                            {"id": "appealInstructions", "contentType": "action-required", "content": "i"}
                        ]
                    }
                }
            }
        }),
        definition: None,
    };
    let report = validate(ValidationRequest {
        candidate: &input,
        dependencies: &[delivery],
        scope: ValidationScope::Project,
    });
    assert_eq!(report.status, CheckStatus::NotRun, "{report:?}");
    let stage = |name: &str| {
        report
            .stages
            .iter()
            .find(|stage| stage.name == name)
            .unwrap_or_else(|| panic!("missing stage {name}: {report:?}"))
    };
    assert_eq!(stage("candidate-schema").status, CheckStatus::Pass, "{report:?}");
    assert_eq!(stage("t1").status, CheckStatus::Pass, "{report:?}");
    assert_eq!(stage("project-resolution").status, CheckStatus::Pass, "{report:?}");
    assert_eq!(stage("t2").status, CheckStatus::Pass, "{report:?}");
    assert_eq!(stage("selected-governance").status, CheckStatus::NotRun, "{report:?}");
    assert_eq!(input, original);
}

#[test]
fn missing_or_false_counterfactual_flag_remains_a_located_failure() {
    for value in [None, Some(json!(false))] {
        let mut input = fixture();
        let policy = input["governance"]["dueProcess"]["adverseDecisionPolicy"]
            .as_object_mut()
            .unwrap();
        policy.remove("counterfactualRequired");
        if let Some(value) = value {
            policy.insert("counterfactualRequired".into(), value);
        }
        let kernel: KernelDocument = serde_json::from_value(input.clone()).unwrap();
        let checks = check_governance(&kernel, &input["governance"], &LifecycleIndex::new(&kernel));
        assert!(
            checks.issues.iter().any(|issue| {
                issue.kind == GovernanceIssueKind::CounterfactualRequired
                    && issue.path == "/dueProcess/adverseDecisionPolicy/counterfactualRequired"
            }),
            "{:?}",
            checks.issues
        );
    }
}

#[test]
fn general_policy_and_actual_audit_tags_remain_required() {
    for (pointer, value, kind) in [
        (
            "/governance/dueProcess/adverseDecisionPolicy/explanationLevel",
            json!("categorical"),
            GovernanceIssueKind::ExplanationLevel,
        ),
        (
            "/governance/audit/counterfactualTier/requiredForTags",
            json!(["adverse-decision", "not-authored"]),
            GovernanceIssueKind::UnknownAuditTag,
        ),
    ] {
        let mut input = fixture();
        *input.pointer_mut(pointer).unwrap() = value;
        let kernel: KernelDocument = serde_json::from_value(input.clone()).unwrap();
        let checks = check_governance(&kernel, &input["governance"], &LifecycleIndex::new(&kernel));
        assert!(
            checks.issues.iter().any(|issue| issue.kind == kind),
            "{:?}",
            checks.issues
        );
    }
}

#[test]
fn present_malformed_execution_is_never_treated_as_absent() {
    for impact in ["rights-impacting", "operational"] {
        for execution in [Value::Null, json!({}), json!({"profile": "unknown"})] {
            let mut input = fixture();
            input["impactLevel"] = json!(impact);
            input["governance"]["dueProcess"]["execution"] = execution;
            let kernel: KernelDocument = serde_json::from_value(input.clone()).unwrap();
            let checks =
                check_governance(&kernel, &input["governance"], &LifecycleIndex::new(&kernel));
            assert!(
                checks.issues.iter().any(|issue| {
                    issue.kind == GovernanceIssueKind::MalformedDeclaration
                        && issue.path == "/dueProcess/execution"
                }),
                "{impact}: {:?}",
                checks.issues
            );
        }
    }
}

#[test]
fn selected_reference_execution_requires_its_inputs_at_every_impact_level() {
    for impact in [Some("rights-impacting"), Some("operational"), None] {
        let mut input = fixture();
        input.as_object_mut().unwrap().remove("impactLevel");
        if let Some(impact) = impact {
            input["impactLevel"] = json!(impact);
        }
        let kernel: KernelDocument = serde_json::from_value(input.clone()).unwrap();
        let checks = check_reference_governance(
            &kernel,
            &input["governance"],
            &LifecycleIndex::new(&kernel),
        );
        assert!(
            checks.issues.iter().any(|issue| {
                issue.kind == GovernanceIssueKind::CounterfactualInputs
                    && issue.path == "/dueProcess/execution"
            }),
            "{impact:?}: {:?}",
            checks.issues
        );
    }
}

#[test]
fn required_adverse_notice_timing_agrees_in_schema_shared_check_and_actual_lint() {
    let schema = local_schemas::validator(&json!({
        "$ref": "https://wos-spec.org/schemas/wos-workflow.schema.json"
    }));
    for impact in [
        "rights-impacting",
        "safety-impacting",
        "operational",
        "informational",
    ] {
        for notice_required in [true, false] {
            for timing in [
                None,
                Some("beforeEffective"),
                Some("onEffective"),
                Some("afterEffective"),
            ] {
                let mut input = fixture();
                input["impactLevel"] = json!(impact);
                let policy = input["governance"]["dueProcess"]["adverseDecisionPolicy"]
                    .as_object_mut()
                    .unwrap();
                policy.insert("noticeRequired".into(), json!(notice_required));
                policy.insert("noticeTemplateKey".into(), json!("adverseNotice"));
                policy
                    .get_mut("appealMechanism")
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .insert(
                        "reviewerConstraint".into(),
                        json!("independentFromOriginal"),
                    );
                policy.remove("noticeTiming");
                if let Some(timing) = timing {
                    policy.insert("noticeTiming".into(), json!(timing));
                }
                let refuse = matches!(impact, "rights-impacting" | "safety-impacting")
                    && notice_required
                    && timing != Some("beforeEffective");
                assert_eq!(
                    schema.is_valid(&input),
                    !refuse,
                    "schema {impact}/{notice_required}/{timing:?}: {:?}",
                    schema
                        .iter_errors(&input)
                        .map(|error| error.to_string())
                        .collect::<Vec<_>>()
                );
                let kernel: KernelDocument = serde_json::from_value(input.clone()).unwrap();
                let checks =
                    check_governance(&kernel, &input["governance"], &LifecycleIndex::new(&kernel));
                assert_eq!(
                    checks
                        .issues
                        .iter()
                        .any(|issue| issue.kind == GovernanceIssueKind::NoticeTiming
                            && issue.path == "/dueProcess/adverseDecisionPolicy/noticeTiming"),
                    refuse
                );
                let native = wos_lint::lint_workflow_with_project(&input.to_string()).unwrap();
                assert_eq!(
                    native.iter().any(|diagnostic| diagnostic.rule_id == "G-001"
                        && diagnostic
                            .path
                            .ends_with("/dueProcess/adverseDecisionPolicy/noticeTiming")),
                    refuse,
                    "lint {impact}/{notice_required}/{timing:?}: {native:?}"
                );
            }
        }
    }
}
