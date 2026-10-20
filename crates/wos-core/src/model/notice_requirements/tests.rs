// Rust guideline compliant 2026-02-21
//! Declaration meaning stays separate from installed capabilities.
use super::*;
use serde_json::{Value, json};
fn requirement(method: &str, scope: &str, anchor: &str) -> NoticeRequirement {
    serde_json::from_value(json!({"observation":method,"scope":scope,"anchor":anchor})).unwrap()
}
#[test]
fn notice_requirements_four_closed_methods_and_no_legacy_aliases() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../../../schemas/wos-notice-requirements.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for method in [
        "transportAccepted",
        "providerDelivered",
        "openObserved",
        "recipientAcknowledged",
        "recipientReceived",
        "fixtureQualifiedReceipt",
        "read",
    ] {
        let raw = json!({"observation":method,"scope":"production","anchor":"none"});
        let valid = [
            "transportAccepted",
            "providerDelivered",
            "openObserved",
            "recipientAcknowledged",
        ]
        .contains(&method);
        assert_eq!(validator.is_valid(&raw), valid, "schema {method}");
        assert_eq!(
            serde_json::from_value::<NoticeRequirement>(raw).is_ok(),
            valid,
            "typed {method}"
        );
    }
    for raw in [
        json!({"observation":"transportAccepted","scope":"test","anchor":"none"}),
        json!({"observation":"transportAccepted","scope":"production"}),
        json!({"observation":"transportAccepted","scope":"production","anchor":"none","passing":true}),
    ] {
        assert!(!validator.is_valid(&raw));
        assert!(serde_json::from_value::<NoticeRequirement>(raw).is_err());
    }
}
#[test]
fn notice_requirements_selected_send_can_anchor_each_actual_consequence() {
    for consequence in [
        NoticeConsequence::ResponseDeadline,
        NoticeConsequence::AdverseEffect,
        NoticeConsequence::AppealWindow,
    ] {
        let r = requirement("transportAccepted", "production", "qualifiedObservation");
        assert!(
            check_notice_support(
                &r,
                consequence,
                Some(NoticeTiming::BeforeEffective),
                &[NoticeCapability::Smtp],
                "/notice"
            )
            .is_empty()
        );
    }
}
#[test]
fn notice_requirements_other_methods_are_valid_but_not_installed() {
    for method in ["providerDelivered", "openObserved", "recipientAcknowledged"] {
        let r = requirement(method, "production", "qualifiedObservation");
        assert!(
            check_notice_declaration(&r, NoticeConsequence::ResponseDeadline, None, "/notice")
                .is_empty()
        );
        for capabilities in [
            vec![],
            vec![NoticeCapability::Smtp],
            vec![NoticeCapability::LocalSmtpTestCapture],
        ] {
            let issues = check_notice_support(
                &r,
                NoticeConsequence::ResponseDeadline,
                None,
                &capabilities,
                "/notice",
            );
            assert_eq!(issues.len(), 1);
            assert_eq!(issues[0].rule_id, "NOTICE-004");
            assert_eq!(issues[0].path, "/notice/observation");
        }
    }
}
#[test]
fn notice_requirements_capture_scope_and_status_do_not_grant_consequences() {
    let r = requirement(
        "transportAccepted",
        "softwareFixture",
        "qualifiedObservation",
    );
    assert!(
        check_notice_support(
            &r,
            NoticeConsequence::ResponseDeadline,
            None,
            &[NoticeCapability::LocalSmtpTestCapture],
            "/notice"
        )
        .is_empty()
    );
    let production = requirement("transportAccepted", "production", "qualifiedObservation");
    assert_eq!(
        check_notice_support(
            &production,
            NoticeConsequence::ResponseDeadline,
            None,
            &[NoticeCapability::LocalSmtpTestCapture],
            "/notice"
        )[0]
        .rule_id,
        "NOTICE-004"
    );
    let status = requirement("transportAccepted", "production", "none");
    assert!(
        check_notice_declaration(&status, NoticeConsequence::OrdinaryStatus, None, "/status")
            .is_empty()
    );
    assert_eq!(
        check_notice_declaration(
            &status,
            NoticeConsequence::ResponseDeadline,
            None,
            "/status"
        )[0]
        .rule_id,
        "NOTICE-003"
    );
    assert_eq!(
        check_notice_declaration(
            &production,
            NoticeConsequence::OrdinaryStatus,
            None,
            "/status"
        )[0]
        .rule_id,
        "NOTICE-003"
    );
}
#[test]
fn notice_requirements_adverse_timing_remains_before_effective() {
    let r = requirement("transportAccepted", "production", "qualifiedObservation");
    for timing in [None, Some(NoticeTiming::AfterEffective)] {
        assert!(
            check_notice_declaration(
                &r,
                NoticeConsequence::AdverseEffect,
                timing,
                "/policy/timing"
            )
            .iter()
            .any(|i| i.rule_id == "G-001")
        );
    }
}
