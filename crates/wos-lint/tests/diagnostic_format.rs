//! Integration test asserting the LintDiagnostic JSON shape.

use wos_lint::{LintDiagnostic, LintSeverity, SourceLocation, SuggestedFix, Tier};

#[test]
fn lint_diagnostic_serializes_to_expected_json_shape() {
    let diag = LintDiagnostic {
        rule_id: "K-023",
        severity: LintSeverity::Error,
        tier: Tier::T1,
        path: "$.states.approved".to_string(),
        message: "state 'approved' has no outbound transition and is not terminal".to_string(),
        suggested_fix: Some(SuggestedFix::AddProperty {
            path: "$.states.approved.type".to_string(),
            value: serde_json::json!("terminal"),
        }),
        related_docs: vec!["specs/kernel/spec.md#S4.2".to_string()],
        source: Some(SourceLocation {
            document: "workflow.json".to_string(),
            line: 42,
            column: 5,
        }),
    };

    let json = serde_json::to_value(&diag).unwrap();
    assert_eq!(json["ruleId"], "K-023");
    assert_eq!(json["severity"], "error");
    assert_eq!(json["tier"], "T1");
    assert_eq!(json["path"], "$.states.approved");
    assert_eq!(json["suggestedFix"]["kind"], "add-property");
    assert_eq!(json["relatedDocs"][0], "specs/kernel/spec.md#S4.2");
    assert_eq!(json["source"]["line"], 42);
}

#[test]
fn suggested_fix_custom_round_trips() {
    let fix = SuggestedFix::Custom {
        hint: "consult the NoticeTemplate reconciliation plan".to_string(),
    };
    let json = serde_json::to_value(&fix).expect("Custom must serialize without error");
    assert_eq!(json["kind"], "custom");
    assert_eq!(json["hint"], "consult the NoticeTemplate reconciliation plan");

    let back: SuggestedFix =
        serde_json::from_value(json).expect("Custom must deserialize without error");
    match back {
        SuggestedFix::Custom { hint } => {
            assert_eq!(hint, "consult the NoticeTemplate reconciliation plan")
        }
        _ => panic!("expected Custom variant"),
    }
}

#[test]
fn optional_fields_are_omitted_when_empty() {
    let diag = LintDiagnostic {
        rule_id: "K-023",
        severity: LintSeverity::Error,
        tier: Tier::T1,
        path: "$.foo".to_string(),
        message: "msg".to_string(),
        suggested_fix: None,
        related_docs: vec![],
        source: None,
    };
    let json = serde_json::to_value(&diag).unwrap();
    assert!(!json.as_object().unwrap().contains_key("suggestedFix"));
    assert!(!json.as_object().unwrap().contains_key("relatedDocs"));
    assert!(!json.as_object().unwrap().contains_key("source"));
}
