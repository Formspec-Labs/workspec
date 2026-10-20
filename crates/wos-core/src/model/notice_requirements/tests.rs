// Rust guideline compliant 2026-02-21
//! Literal notice rows distinguish declaration meaning from installed capability.

use super::*;
use serde_json::{Value, json};

fn requirement(value: Value) -> NoticeRequirement {
    serde_json::from_value(value).unwrap()
}

#[test]
fn notice_requirements_closed_shape_matches_canonical_schema() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../../../schemas/wos-notice-requirements.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for (raw, valid) in [
        (
            json!({"observation":"transportAccepted","scope":"production","anchor":"none"}),
            true,
        ),
        (
            json!({"observation":"recipientReceived","scope":"production","anchor":"qualifiedObservation"}),
            true,
        ),
        (
            json!({"observation":"recipientAcknowledged","scope":"production","anchor":"qualifiedObservation"}),
            true,
        ),
        (
            json!({"observation":"fixtureQualifiedReceipt","scope":"softwareFixture","anchor":"qualifiedObservation"}),
            true,
        ),
        (
            json!({"observation":"read","scope":"production","anchor":"none"}),
            false,
        ),
        (
            json!({"observation":"transportAccepted","scope":"test","anchor":"none"}),
            false,
        ),
        (
            json!({"observation":"transportAccepted","scope":"production","anchor":"commandNow"}),
            false,
        ),
        (
            json!({"observation":"transportAccepted","scope":"production"}),
            false,
        ),
        (
            json!({"observation":"transportAccepted","scope":"production","anchor":null}),
            false,
        ),
        (
            json!({"observation":"transportAccepted","scope":"production","anchor":"none","allowNotice":true}),
            false,
        ),
    ] {
        assert_eq!(validator.is_valid(&raw), valid, "schema: {raw}");
        let decoded = serde_json::from_value::<NoticeRequirement>(raw.clone());
        assert_eq!(decoded.is_ok(), valid, "typed: {raw}");
        if let Ok(decoded) = decoded {
            assert_eq!(serde_json::to_value(decoded).unwrap(), raw);
        }
    }
}

#[test]
fn notice_requirements_status_is_no_clock_and_never_authorizes_a_consequence() {
    let status = requirement(
        json!({"observation":"transportAccepted","scope":"production","anchor":"none"}),
    );
    assert!(
        check_notice_declaration(&status, NoticeConsequence::OrdinaryStatus, None, "/status")
            .is_empty()
    );
    assert!(
        check_notice_support(
            &status,
            NoticeConsequence::OrdinaryStatus,
            None,
            &[NoticeCapability::Smtp],
            "/status"
        )
        .is_empty()
    );
    assert_eq!(
        check_notice_support(
            &status,
            NoticeConsequence::OrdinaryStatus,
            None,
            &[],
            "/status"
        )[0]
        .rule_id,
        "NOTICE-004"
    );
    let mut clocked = status;
    clocked.anchor = NoticeAnchor::QualifiedObservation;
    let refused =
        check_notice_declaration(&clocked, NoticeConsequence::OrdinaryStatus, None, "/status");
    assert_eq!(refused[0].path, "/status/anchor");
    for consequence in [
        NoticeConsequence::ResponseDeadline,
        NoticeConsequence::AdverseEffect,
        NoticeConsequence::AppealWindow,
    ] {
        let refused = check_notice_support(
            &status,
            consequence,
            Some(NoticeTiming::BeforeEffective),
            &[NoticeCapability::Smtp],
            "/status",
        );
        assert!(
            refused
                .iter()
                .any(|issue| issue.rule_id == "NOTICE-002" && issue.path == "/status/observation")
        );
        assert!(
            refused
                .iter()
                .any(|issue| issue.rule_id == "NOTICE-003" && issue.path == "/status/anchor")
        );
    }
    let captured = NoticeRequirement {
        scope: NoticeScope::SoftwareFixture,
        ..status
    };
    assert!(
        check_notice_support(
            &captured,
            NoticeConsequence::OrdinaryStatus,
            None,
            &[NoticeCapability::LocalSmtpTestCapture],
            "/status"
        )
        .is_empty()
    );
    assert!(
        !check_notice_support(
            &status,
            NoticeConsequence::OrdinaryStatus,
            None,
            &[NoticeCapability::LocalSmtpTestCapture],
            "/status"
        )
        .is_empty()
    );
}

#[test]
fn notice_requirements_recipient_declaration_does_not_invent_installed_support() {
    for observation in ["recipientReceived", "recipientAcknowledged"] {
        let selected = requirement(
            json!({"observation":observation,"scope":"production","anchor":"qualifiedObservation"}),
        );
        for consequence in [
            NoticeConsequence::ResponseDeadline,
            NoticeConsequence::AdverseEffect,
            NoticeConsequence::AppealWindow,
        ] {
            assert!(
                check_notice_declaration(
                    &selected,
                    consequence,
                    Some(NoticeTiming::BeforeEffective),
                    "/requirements/original"
                )
                .is_empty()
            );
            for installed in [
                vec![],
                vec![NoticeCapability::Smtp],
                vec![NoticeCapability::LocalSmtpTestCapture],
                vec![NoticeCapability::SoftwareFixtureReceipt],
                vec![
                    NoticeCapability::Smtp,
                    NoticeCapability::LocalSmtpTestCapture,
                    NoticeCapability::SoftwareFixtureReceipt,
                ],
            ] {
                let issues = check_notice_support(
                    &selected,
                    consequence,
                    Some(NoticeTiming::BeforeEffective),
                    &installed,
                    "/requirements/original",
                );
                assert_eq!(issues.len(), 1);
                assert_eq!(issues[0].rule_id, "NOTICE-004");
                assert_eq!(issues[0].path, "/requirements/original/observation");
            }
        }
    }
}

#[test]
fn notice_requirements_fixture_positive_requires_exact_class_scope_and_anchor() {
    let selected = requirement(
        json!({"observation":"fixtureQualifiedReceipt","scope":"softwareFixture","anchor":"qualifiedObservation"}),
    );
    for consequence in [
        NoticeConsequence::ResponseDeadline,
        NoticeConsequence::AdverseEffect,
        NoticeConsequence::AppealWindow,
    ] {
        assert!(
            check_notice_support(
                &selected,
                consequence,
                Some(NoticeTiming::BeforeEffective),
                &[NoticeCapability::SoftwareFixtureReceipt],
                "/requirements/adverse"
            )
            .is_empty()
        );
        for installed in [
            vec![],
            vec![NoticeCapability::Smtp],
            vec![NoticeCapability::LocalSmtpTestCapture],
        ] {
            assert_eq!(
                check_notice_support(
                    &selected,
                    consequence,
                    Some(NoticeTiming::BeforeEffective),
                    &installed,
                    "/requirements/adverse"
                )[0]
                .rule_id,
                "NOTICE-004"
            );
        }
    }
    let production = NoticeRequirement {
        scope: NoticeScope::Production,
        ..selected
    };
    let refused = check_notice_support(
        &production,
        NoticeConsequence::AdverseEffect,
        Some(NoticeTiming::BeforeEffective),
        &[NoticeCapability::SoftwareFixtureReceipt],
        "/requirements/adverse",
    );
    assert!(
        refused
            .iter()
            .any(|issue| issue.rule_id == "NOTICE-001"
                && issue.path == "/requirements/adverse/scope")
    );
    assert!(refused.iter().any(|issue| issue.rule_id == "NOTICE-004"));
    let unanchored = NoticeRequirement {
        anchor: NoticeAnchor::None,
        ..selected
    };
    assert_eq!(
        check_notice_declaration(
            &unanchored,
            NoticeConsequence::AppealWindow,
            None,
            "/appeal"
        )[0]
        .path,
        "/appeal/anchor"
    );
    for observation in [
        NoticeObservation::RecipientReceived,
        NoticeObservation::RecipientAcknowledged,
    ] {
        let relabeled = NoticeRequirement {
            observation,
            ..selected
        };
        assert!(
            check_notice_support(
                &relabeled,
                NoticeConsequence::AdverseEffect,
                Some(NoticeTiming::BeforeEffective),
                &[NoticeCapability::SoftwareFixtureReceipt],
                "/requirements/adverse"
            )
            .iter()
            .any(|issue| issue.rule_id == "NOTICE-001")
        );
    }
}

#[test]
fn notice_requirements_before_effect_is_explicit_and_unchanged_by_provider() {
    let selected = requirement(
        json!({"observation":"fixtureQualifiedReceipt","scope":"softwareFixture","anchor":"qualifiedObservation"}),
    );
    for timing in [
        None,
        Some(NoticeTiming::OnEffective),
        Some(NoticeTiming::AfterEffective),
    ] {
        let declaration = check_notice_declaration(
            &selected,
            NoticeConsequence::AdverseEffect,
            timing,
            "/requirements/adverse",
        );
        let support = check_notice_support(
            &selected,
            NoticeConsequence::AdverseEffect,
            timing,
            &[NoticeCapability::SoftwareFixtureReceipt],
            "/requirements/adverse",
        );
        assert_eq!(declaration, support);
        assert_eq!(declaration.len(), 1);
        assert_eq!(declaration[0].rule_id, "G-001");
        assert_eq!(declaration[0].path, "/requirements/adverse");
    }
}
