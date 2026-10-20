// Rust guideline compliant 2026-02-21
//! Declaration checks distinguish general obligations from selected execution.
use serde_json::{Value, json};
use wos_core::KernelDocument;
use wos_core::model::governance_checks::{
    GovernanceIssueKind, LifecycleIndex, check_governance, check_reference_governance,
};
use wos_lint::studio_api::{CheckStatus, ValidationRequest, ValidationScope, validate};

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
    let report = validate(ValidationRequest {
        candidate: &input,
        dependencies: &[],
        scope: ValidationScope::Project,
    });
    assert_eq!(report.status, CheckStatus::Pass, "{report:?}");
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
