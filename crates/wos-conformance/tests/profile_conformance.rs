// Rust guideline compliant 2026-04-14

//! Aggregate conformance profile tests.
//!
//! Delegates to `wos_conformance::run_profile_against_fixtures` so that profile
//! aggregation logic lives in one place (meta.rs), not duplicated in tests.

use std::path::{Path, PathBuf};

use wos_conformance::{
    run_profile_against_fixtures, validate_ai_family_batch_coverage, AI_CONFIDENCE_BATCHES,
    AI_REGISTRATION_BATCHES, GOVERNANCE_BASIC_RULES,
};

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn governance_basic_profile_passes() {
    let (passed, message) = run_profile_against_fixtures(
        &fixtures_dir(),
        "Governance Basic",
        |fixture| GOVERNANCE_BASIC_RULES.contains(&fixture.rule.as_str()),
        Some(GOVERNANCE_BASIC_RULES),
    );
    assert!(passed, "{message}");
}

#[test]
fn governance_complete_profile_passes() {
    let (passed, message) = run_profile_against_fixtures(
        &fixtures_dir(),
        "Governance Complete",
        |fixture| fixture.rule.starts_with("G-"),
        None,
    );
    assert!(passed, "{message}");
}

#[test]
fn ai_family_fixtures_declare_batch_metadata() {
    validate_ai_family_batch_coverage(&fixtures_dir(), AI_REGISTRATION_BATCHES)
        .expect("AI-family batch metadata and coverage");
}

#[test]
fn ai_registration_profile_passes() {
    validate_ai_family_batch_coverage(&fixtures_dir(), AI_REGISTRATION_BATCHES)
        .expect("AI Registration batch coverage");

    let (passed, message) = run_profile_against_fixtures(
        &fixtures_dir(),
        "AI Registration",
        |fixture| {
            fixture
                .batch
                .is_some_and(|batch| AI_REGISTRATION_BATCHES.contains(&batch))
        },
        None,
    );
    assert!(passed, "{message}");
}

#[test]
fn ai_confidence_framework_profile_passes() {
    validate_ai_family_batch_coverage(&fixtures_dir(), AI_CONFIDENCE_BATCHES)
        .expect("AI Confidence batch coverage");

    let (passed, message) = run_profile_against_fixtures(
        &fixtures_dir(),
        "AI Confidence Framework",
        |fixture| {
            fixture
                .batch
                .is_some_and(|batch| AI_CONFIDENCE_BATCHES.contains(&batch))
        },
        None,
    );
    assert!(passed, "{message}");
}
