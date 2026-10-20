// Rust guideline compliant 2026-02-21
//! Contextual authoring and native qualification use the same selected analysis.
use serde_json::{Value, json};
use wos_core::model::governance_policy::GovernancePolicyRules;
use wos_core::model::integration::IntegrationProfileDocument;
use wos_lint::studio_api::{
    CheckStatus, SelectedGovernanceContext, ValidationRequest, ValidationScope, validate,
    validate_with_governance,
};
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../wos-core/tests/fixtures/notice_policy.json"
    ))
    .unwrap()
}
#[test]
fn actual_shared_context_accepts_explicit_original_notice_associations() {
    let raw = fixture();
    let rules: GovernancePolicyRules = serde_json::from_value(raw["rules"].clone()).unwrap();
    let profile: IntegrationProfileDocument =
        serde_json::from_value(raw["integration"].clone()).unwrap();
    let report = validate_with_governance(
        ValidationRequest {
            candidate: &raw["kernel"],
            dependencies: &[],
            scope: ValidationScope::Document,
        },
        SelectedGovernanceContext {
            rules: &rules,
            integration: Some(&profile),
        },
    );
    assert_eq!(
        report.status,
        CheckStatus::Pass,
        "{}",
        serde_json::to_string_pretty(&report).unwrap()
    );
}
#[test]
fn actual_shared_context_reports_missing_and_ambiguous_required_relations() {
    for mode in ["integration", "producer", "ambiguous"] {
        let mut raw = fixture();
        if mode == "producer" {
            raw["kernel"]["lifecycle"]["states"]["sendRequest"]["onEntry"] = json!([]);
        }
        if mode == "ambiguous" {
            let consumer =
                raw["kernel"]["lifecycle"]["states"]["sendRequest"]["transitions"][0].clone();
            raw["kernel"]["lifecycle"]["states"]["sendRequest"]["transitions"]
                .as_array_mut()
                .unwrap()
                .push(consumer);
        }
        let rules: GovernancePolicyRules = serde_json::from_value(raw["rules"].clone()).unwrap();
        let profile: IntegrationProfileDocument =
            serde_json::from_value(raw["integration"].clone()).unwrap();
        let report = validate_with_governance(
            ValidationRequest {
                candidate: &raw["kernel"],
                dependencies: &[],
                scope: ValidationScope::Document,
            },
            SelectedGovernanceContext {
                rules: &rules,
                integration: if mode == "integration" {
                    None
                } else {
                    Some(&profile)
                },
            },
        );
        assert_eq!(report.status, CheckStatus::Fail, "{mode}");
        assert!(
            report
                .diagnostics
                .iter()
                .any(|i| i.rule_id == "NOTICE-GRAPH-001"),
            "{mode}"
        );
    }
}
#[test]
fn plain_authoring_reports_notice_context_not_run_instead_of_qualification() {
    let raw = fixture();
    let report = validate(ValidationRequest {
        candidate: &raw["kernel"],
        dependencies: &[],
        scope: ValidationScope::Document,
    });
    assert!(
        report.stages.iter().any(|s| s.required
            && s.status == CheckStatus::NotRun
            && s.name.contains("selected-governance")),
        "{}",
        serde_json::to_string_pretty(&report).unwrap()
    );
}
#[test]
fn actual_shared_context_keeps_graph_refusal_when_native_caller_is_independent() {
    let mut raw = fixture();
    let action = raw["kernel"]["lifecycle"]["states"]["sendRequest"]["onEntry"].take();
    raw["kernel"]["lifecycle"]["states"]["sendRequest"]["onEntry"] = json!([]);
    raw["kernel"]["lifecycle"]["states"]["correction"]["onEntry"]
        .as_array_mut()
        .unwrap()
        .extend(action.as_array().unwrap().iter().cloned());
    let rules: GovernancePolicyRules = serde_json::from_value(raw["rules"].clone()).unwrap();
    let profile: IntegrationProfileDocument =
        serde_json::from_value(raw["integration"].clone()).unwrap();
    let report = validate_with_governance(
        ValidationRequest {
            candidate: &raw["kernel"],
            dependencies: &[],
            scope: ValidationScope::Document,
        },
        SelectedGovernanceContext {
            rules: &rules,
            integration: Some(&profile),
        },
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|i| i.rule_id == "NOTICE-GRAPH-006"
                && i.path == "/rules/noticeRequirements/request")
    );
}
#[test]
fn original_rules_schema_has_maintained_documentation_and_string_classifications() {
    let diagnostics = wos_lint::lint_schema(include_str!(
        "../../../schemas/wos-governance-policy-rules.schema.json"
    ))
    .unwrap();
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn nested_clock_actual_shared_context_checks_whole_callback_and_missing_rules() {
    for missing_rules in [false, true] {
        let mut raw = fixture();
        raw["integration"]["bindings"]["request"]["outputBinding"] =
            json!({"caseFile.requestNotice":"$"});
        raw["kernel"]["lifecycle"]["states"]["correction"]["onEntry"][0]["startAtExpression"] =
            json!("caseFile.requestNotice.clock.governedSentAt");
        if missing_rules {
            raw["rules"]["timeoutDecisions"] = json!([]);
            raw["rules"]["noticeRequirements"]["request"]["anchor"] = json!("none");
        }
        let rules = serde_json::from_value(raw["rules"].clone()).unwrap();
        let profile = serde_json::from_value(raw["integration"].clone()).unwrap();
        let report = validate_with_governance(
            ValidationRequest {
                candidate: &raw["kernel"],
                dependencies: &[],
                scope: ValidationScope::Document,
            },
            SelectedGovernanceContext {
                rules: &rules,
                integration: Some(&profile),
            },
        );
        assert_eq!(
            report.status,
            if missing_rules {
                CheckStatus::Fail
            } else {
                CheckStatus::Pass
            },
            "missing_rules={missing_rules}: {}",
            serde_json::to_string_pretty(&report).unwrap()
        );
        if missing_rules {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|d| d.rule_id == "NOTICE-GRAPH-001")
            );
        }
    }
}
