// Rust guideline compliant 2026-04-14

//! Aggregate conformance profile tests.
//!
//! Delegates to `wos_conformance::run_profile_against_fixtures` so that profile
//! aggregation logic lives in one place (meta.rs), not duplicated in tests.

use std::path::{Path, PathBuf};

use wos_conformance::{
    AI_CONFIDENCE_BATCHES, AI_REGISTRATION_BATCHES, GOVERNANCE_BASIC_RULES,
    run_profile_against_fixtures, validate_ai_family_batch_coverage,
};

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn governance_basic_profile_reports_missing_operation_owners() {
    let (passed, message) = run_profile_against_fixtures(
        &fixtures_dir(),
        "Governance Basic",
        |fixture| GOVERNANCE_BASIC_RULES.contains(&fixture.rule.as_str()),
        Some(GOVERNANCE_BASIC_RULES),
    );
    assert!(!passed, "{message}");
    assert!(message.contains("unsupported"), "{message}");
}

#[test]
fn governance_complete_profile_reports_missing_operation_owners() {
    let (passed, message) = run_profile_against_fixtures(
        &fixtures_dir(),
        "Governance Complete",
        |fixture| fixture.rule.starts_with("G-"),
        None,
    );
    assert!(!passed, "{message}");
    assert!(message.contains("unsupported"), "{message}");
}

#[test]
fn native_stage_inventory_reports_missing_complete_registration_batches() {
    let missing =
        validate_ai_family_batch_coverage(&fixtures_dir(), AI_REGISTRATION_BATCHES).unwrap_err();
    assert!(
        missing.contains("missing fixture coverage for batches"),
        "{missing}"
    );
}

#[test]
fn native_policy_subset_cannot_verify_complete_registration() {
    let (passed, message) = run_profile_against_fixtures(
        &fixtures_dir(),
        "AI Registration",
        |fixture| fixture.rule.starts_with("AI-"),
        Some(&["AI-005", "AI-006", "AI-008", "AI-009", "AI-019", "AI-034"]),
    );
    assert!(!passed);
    assert!(message.contains("missing fixture coverage"), "{message}");
}

#[test]
fn native_report_and_floor_cases_pass_without_claiming_full_confidence_framework() {
    validate_ai_family_batch_coverage(&fixtures_dir(), AI_CONFIDENCE_BATCHES).unwrap();
    let (passed, message) = run_profile_against_fixtures(
        &fixtures_dir(),
        "Native report/floor stages",
        |fixture| matches!(fixture.rule.as_str(), "AI-034" | "AI-036"),
        Some(&["AI-034", "AI-036"]),
    );
    assert!(passed, "{message}");
    let (passed, message) = run_profile_against_fixtures(
        &fixtures_dir(),
        "Complete confidence framework",
        |fixture| fixture.batch == Some(5),
        Some(&["AI-034", "AI-035", "AI-036", "AI-037", "AI-038"]),
    );
    assert!(!passed);
    assert!(message.contains("missing fixture coverage"), "{message}");
}
