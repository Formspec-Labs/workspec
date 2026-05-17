// Rust guideline compliant 2026-02-21

//! Signature Profile runtime conformance fixtures.

use std::path::Path;

use wos_conformance::run_fixture;

fn fixture_json(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "could not read signature fixture {}: {error}",
            path.display()
        )
    })
}

fn assert_signature_fixture_passes(name: &str) {
    let base_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let result = run_fixture(
        &fixture_json(name),
        base_dir.to_str().expect("utf-8 fixture path"),
    )
    .unwrap_or_else(|error| panic!("signature fixture {name} errored: {error}"));
    assert!(
        result.passed,
        "signature fixture {name} failed:\n{}",
        result.failures.join("\n")
    );
}

#[test]
fn sig001_sequential_single_signer() {
    assert_signature_fixture_passes("SIG-001-sequential-single-signer.json");
}

#[test]
fn sig002_parallel_signers_any_order() {
    assert_signature_fixture_passes("SIG-002-parallel-signers-any-order.json");
}

#[test]
fn sig003_routed_signer_fel_guard() {
    assert_signature_fixture_passes("SIG-003-routed-signer-fel-guard.json");
}

#[test]
fn sig004_expiry_timer() {
    assert_signature_fixture_passes("SIG-004-expiry-timer.json");
}

#[test]
fn sig005_decline_path() {
    assert_signature_fixture_passes("SIG-005-decline-path.json");
}

#[test]
fn sig006_reassignment_accountability() {
    assert_signature_fixture_passes("SIG-006-reassignment-accountability.json");
}

#[test]
fn sig007_witness_countersignature() {
    assert_signature_fixture_passes("SIG-007-witness-countersignature.json");
}

#[test]
fn sig008_notary_in_person_auth() {
    assert_signature_fixture_passes("SIG-008-notary-in-person-auth.json");
}

#[test]
fn sig009_missing_consent_blocks_affirmation() {
    let base_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let error = run_fixture(
        &fixture_json("SIG-009-missing-consent-blocks-affirmation.json"),
        base_dir.to_str().expect("utf-8 fixture path"),
    )
    .expect_err("missing consent must produce a runtime error");
    assert!(
        error
            .to_string()
            .contains("missing signature evidence field"),
        "unexpected missing-consent error: {error}"
    );
}

#[test]
fn sig010_custody_append_window() {
    assert_signature_fixture_passes("SIG-010-custody-append-window.json");
}

#[test]
fn sig011_free_for_all_signers_any_order() {
    assert_signature_fixture_passes("SIG-011-free-for-all-signers-any-order.json");
}

#[test]
fn sig012_void_path() {
    assert_signature_fixture_passes("SIG-012-void-path.json");
}

#[test]
fn sig013_policy_assurance_below_floor_blocks_affirmation() {
    let base_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let error = run_fixture(
        &fixture_json("SIG-013-policy-assurance-below-floor.json"),
        base_dir.to_str().expect("utf-8 fixture path"),
    )
    .expect_err("identity binding below the policy assurance floor must reject");
    assert!(
        error.to_string().contains("is below policy 'emailOtp'"),
        "unexpected policy-floor rejection error: {error}"
    );
}

#[test]
fn sig014_signed_payload_digest_mismatch_blocks_affirmation() {
    assert_signature_fixture_passes("SIG-014-signed-payload-digest-mismatch.json");
}

#[test]
fn sig015_signing_intent_mismatch_blocks_affirmation() {
    assert_signature_fixture_passes("SIG-015-signing-intent-mismatch.json");
}

#[test]
fn sig016_signer_authority_floor_failure_blocks_affirmation() {
    assert_signature_fixture_passes("SIG-016-signer-authority-floor-failure.json");
}

#[test]
fn sig018_undecodable_signature_value_fails_closed() {
    // ADR 0109 requires a present but undecodable `signatureValue` envelope to
    // fail closed before runtime can fall back to identityBinding.method.
    assert_signature_fixture_passes("SIG-018-tampered-signature-value.json");
}

#[test]
fn sig019_tampered_signature_method_admits_with_deferred_status() {
    // Parseable COSE `method_uri` is enough for binding pre-checks today; the
    // primitive remains deferred until FORMSPEC-SIGN-HELPER-001 lands.
    assert_signature_fixture_passes("SIG-019-tampered-signature-method.json");
}

#[test]
fn sig020_authority_missing_evidence_binding_blocks_affirmation() {
    assert_signature_fixture_passes("SIG-020-authority-missing-evidence-binding.json");
}

#[test]
fn sig021_authority_expired_validity_window_blocks_affirmation() {
    assert_signature_fixture_passes("SIG-021-authority-expired-validity-window.json");
}

#[test]
fn sig022_authority_malformed_evidence_hash_blocks_affirmation() {
    assert_signature_fixture_passes("SIG-022-authority-malformed-evidence-hash.json");
}

#[test]
fn sig023_authority_self_class_without_source_admits() {
    assert_signature_fixture_passes("SIG-023-authority-self-class-without-source.json");
}

#[test]
fn sig024_allowlisted_deployment_local_intent_admits() {
    assert_signature_fixture_passes("SIG-024-allowlisted-deployment-local-intent.json");
}

#[test]
fn sig025_unregistered_non_allowlisted_intent_blocks_affirmation() {
    assert_signature_fixture_passes("SIG-025-unregistered-non-allowlisted-intent.json");
}

#[test]
fn sig026_evidence_response_signed_at_divergence_blocks_affirmation() {
    assert_signature_fixture_passes("SIG-026-evidence-response-signed-at-divergence.json");
}

#[test]
fn sig027_posture_method_unsupported_blocks_affirmation() {
    assert_signature_fixture_passes("SIG-027-posture-method-unsupported.json");
}

#[test]
fn sig028_posture_floor_unmet_blocks_affirmation() {
    assert_signature_fixture_passes("SIG-028-posture-floor-unmet.json");
}

#[test]
fn sig029_posture_declaration_loaded_admits_signature() {
    assert_signature_fixture_passes("SIG-029-posture-declaration-loaded.json");
}

#[test]
fn sig030_method_unregistered_blocks_affirmation() {
    assert_signature_fixture_passes("SIG-030-method-unregistered.json");
}

#[test]
fn sig017_stale_response_pin_blocks_affirmation() {
    assert_signature_fixture_passes("SIG-017-stale-response-pin.json");
}

#[test]
fn sig012_void_cancels_pending_signature_tasks() {
    let base_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut fixture: serde_json::Value =
        serde_json::from_str(&fixture_json("SIG-012-void-path.json"))
            .expect("void fixture parses as JSON");
    fixture
        .get_mut("event_sequence")
        .and_then(serde_json::Value::as_array_mut)
        .expect("void fixture event_sequence is an array")
        .push(serde_json::json!({
            "event": "submit",
            "task_submission": {
                "task_ref": "applicantTask",
                "response": {
                    "status": "completed",
                    "definitionUrl": "urn:test:formspec:signature",
                    "definitionVersion": "1.0.0",
                    "data": {
                        "signerId": "applicant",
                        "identityBinding": {
                            "method": "email-otp",
                            "assuranceLevel": "standard"
                        },
                        "signatureProvider": "formspec",
                        "ceremonyId": "ceremony-after-void",
                        "signature": {
                            "acceptedAt": "2026-04-22T12:02:00Z",
                            "affirmed": true,
                            "documentHash": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                        }
                    }
                }
            }
        }));
    let error = run_fixture(
        &serde_json::to_string(&fixture).expect("void fixture serializes"),
        base_dir.to_str().expect("utf-8 fixture path"),
    )
    .expect_err("voided flow must remove pending signature tasks");
    assert!(
        error
            .to_string()
            .contains("no active task with task_ref 'applicantTask'"),
        "unexpected post-void submission error: {error}"
    );
}
