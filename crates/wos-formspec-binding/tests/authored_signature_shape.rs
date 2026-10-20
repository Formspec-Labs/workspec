// Rust guideline compliant 2026-02-21

//! Checks canonical Formspec signature structure without claiming cryptographic verification.
//!
//! Synthetic fixtures contain no personal data. The ignored private-evidence
//! diagnostic borrows the authorized Response unchanged and emits no field values.

use integrity_canonical::{DigestAlgorithm, build_signed_payload};
use serde_json::{Value, json};
use wos_core::instance::ActiveTask;
use wos_formspec_binding::{FormspecBinding, FormspecProcessor, parse_authored_signatures};
use wos_runtime::{BindingError, CaseMutationBundle, ContractBindingAdapter};

const SIGNED_AT: &str = "2026-04-22T12:00:00Z";
const PRIVATE_EVIDENCE_SHA256: &str =
    "7e48590a16798ddcd1f5a84c6b43e17d58c1b86fc64e8dd9c1c92cd7cca93b2d";

fn canonical_response() -> Value {
    let mut response: Value =
        serde_json::from_str(include_str!("fixtures/canonical-authored-response.json"))
            .expect("synthetic canonical fixture must be JSON");
    let digest = build_signed_payload(response.to_string().as_bytes(), DigestAlgorithm::Sha256)
        .expect("synthetic fixture must canonicalize")
        .digest;
    response["authoredSignatures"][0]["signedPayload"]["digest"] = digest.into();
    response
}

struct UnusedProcessor;
impl FormspecProcessor for UnusedProcessor {
    fn validate_envelope(&self, _: &Value) -> Result<Vec<Value>, BindingError> {
        Ok(vec![])
    }
    fn validate_definition(
        &self,
        _: &str,
        _: &str,
        _: &Value,
    ) -> Result<Option<Vec<Value>>, BindingError> {
        Ok(None)
    }
    fn compute_prefill(&self, _: Option<&str>, _: &Value) -> Result<Option<Value>, BindingError> {
        Ok(None)
    }
    fn map_response(&self, _: &str, _: &Value) -> Result<Option<CaseMutationBundle>, BindingError> {
        Ok(None)
    }
}
fn task() -> ActiveTask {
    serde_json::from_value(
        json!({"taskId":"task-synthetic","taskRef":"syntheticReview",
        "status":"assigned","createdAt":SIGNED_AT,"updatedAt":SIGNED_AT}),
    )
    .unwrap()
}

#[test]
fn canonical_nested_signature_time_is_read_without_outer_timestamp() {
    let response = canonical_response();
    assert!(response["authoredSignatures"][0].get("signedAt").is_none());
    let result = parse_authored_signatures(
        &integrity_canonical::ResponseJson::from_value(&response).unwrap(),
    );
    assert!(
        result.is_ok(),
        "Canonical nested signature timestamp must parse"
    );
    let parsed = result.unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].signed_at(), SIGNED_AT);
}

#[test]
fn signature_evidence_reads_canonical_nested_signature_time() {
    let response = canonical_response();
    let result = FormspecBinding::new(UnusedProcessor).signature_evidence(
        &task(),
        &integrity_canonical::ResponseJson::from_value(&response).unwrap(),
    );
    assert!(
        result.is_ok(),
        "Canonical signature must reach evidence construction"
    );
    let evidence = result.unwrap().unwrap();
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].signed_at, SIGNED_AT);
    // AA== is an intentionally unqualified signature value. Shape acceptance
    // does not turn its undecodable cryptographic method into verified evidence.
    assert!(evidence[0].admission_failure.is_some());
}

#[test]
fn canonical_signature_refuses_outer_nested_intent_divergence() {
    let mut response = canonical_response();
    assert!(
        parse_authored_signatures(
            &integrity_canonical::ResponseJson::from_value(&response).unwrap()
        )
        .is_ok(),
        "Canonical control must parse before testing divergence"
    );
    response["authoredSignatures"][0]["signingIntent"] = "urn:test:signing-intent:foreign".into();
    assert!(
        parse_authored_signatures(
            &integrity_canonical::ResponseJson::from_value(&response).unwrap()
        )
        .is_err(),
        "Mirrored canonical signing intent must agree"
    );
    let result = FormspecBinding::new(UnusedProcessor).signature_evidence(
        &task(),
        &integrity_canonical::ResponseJson::from_value(&response).unwrap(),
    );
    assert!(
        result.is_err(),
        "Evidence reader must reject mirrored intent divergence"
    );
}

#[test]
#[ignore = "Requires explicitly authorized private source evidence; run via parent-owned native window"]
fn authorized_private_response_is_borrowed_losslessly_by_signature_reader() {
    let path = std::env::var_os("WOS_FORMSPEC_TEST_SOURCE_EVIDENCE_PATH")
        .expect("Authorized private evidence path must be supplied");
    let bytes = std::fs::read(path).expect("Authorized private evidence must be readable");
    let digest = integrity_canonical::compute_digest(&bytes, DigestAlgorithm::Sha256);
    assert!(
        digest == PRIVATE_EVIDENCE_SHA256,
        "Authorized evidence digest must match"
    );
    let document: Value = serde_json::from_slice(&bytes).expect("Authorized evidence must be JSON");
    let response = document
        .get("response")
        .expect("Authorized evidence must contain Response");
    let signatures = response
        .get("authoredSignatures")
        .and_then(Value::as_array)
        .expect("Authorized Response must contain signatures");
    assert!(
        !signatures.is_empty(),
        "Authorized signature diagnostic must be nonvacuous"
    );
    let result = parse_authored_signatures(
        &integrity_canonical::ResponseJson::from_value(response).unwrap(),
    );
    assert!(
        result.is_ok(),
        "Authorized canonical Response must parse without rewriting"
    );
    let parsed = result.unwrap();
    assert!(
        parsed.len() == signatures.len(),
        "Reader must retain signature count"
    );
    for (item, original) in parsed.iter().zip(signatures) {
        assert!(
            original.get("signedAt").is_none(),
            "Canonical outer timestamp must be absent"
        );
        assert!(
            original["signedPayload"]["signedAt"].as_str() == Some(item.signed_at()),
            "Reader must use canonical nested timestamp"
        );
    }
    let evidence = FormspecBinding::new(UnusedProcessor).signature_evidence(
        &task(),
        &integrity_canonical::ResponseJson::from_value(response).unwrap(),
    );
    assert!(
        evidence.is_ok(),
        "Authorized canonical Response must reach evidence construction"
    );
    assert!(
        integrity_canonical::compute_digest(&bytes, DigestAlgorithm::Sha256)
            == PRIVATE_EVIDENCE_SHA256,
        "Borrowed source bytes must remain unchanged"
    );
}

#[test]
fn canonical_signature_rejects_missing_nested_time_and_outer_only_time() {
    for with_outer in [false, true] {
        let mut response = canonical_response();
        response["authoredSignatures"][0]["signedPayload"]
            .as_object_mut()
            .unwrap()
            .remove("signedAt");
        if with_outer {
            response["authoredSignatures"][0]["signedAt"] = SIGNED_AT.into();
        }
        assert!(
            parse_authored_signatures(
                &integrity_canonical::ResponseJson::from_value(&response).unwrap()
            )
            .is_err()
        );
        assert!(
            FormspecBinding::new(UnusedProcessor)
                .signature_evidence(
                    &task(),
                    &integrity_canonical::ResponseJson::from_value(&response).unwrap()
                )
                .is_err()
        );
    }
}

#[test]
fn canonical_signature_rejects_outer_timestamp_even_when_nested_time_agrees() {
    for outer in [json!(SIGNED_AT), Value::Null] {
        let mut response = canonical_response();
        response["authoredSignatures"][0]["signedAt"] = outer;
        assert!(
            parse_authored_signatures(
                &integrity_canonical::ResponseJson::from_value(&response).unwrap()
            )
            .is_err()
        );
        assert!(
            FormspecBinding::new(UnusedProcessor)
                .signature_evidence(
                    &task(),
                    &integrity_canonical::ResponseJson::from_value(&response).unwrap()
                )
                .is_err()
        );
    }
}

#[test]
fn canonical_signature_requires_nested_intent_and_nonblank_time() {
    let mut missing_intent = canonical_response();
    missing_intent["authoredSignatures"][0]["signedPayload"]
        .as_object_mut()
        .unwrap()
        .remove("signingIntent");
    assert!(
        parse_authored_signatures(
            &integrity_canonical::ResponseJson::from_value(&missing_intent).unwrap()
        )
        .is_err()
    );
    assert!(
        FormspecBinding::new(UnusedProcessor)
            .signature_evidence(
                &task(),
                &integrity_canonical::ResponseJson::from_value(&missing_intent).unwrap()
            )
            .is_err()
    );
    let mut blank_time = canonical_response();
    blank_time["authoredSignatures"][0]["signedPayload"]["signedAt"] = "".into();
    assert!(
        parse_authored_signatures(
            &integrity_canonical::ResponseJson::from_value(&blank_time).unwrap()
        )
        .is_err()
    );
    assert!(
        FormspecBinding::new(UnusedProcessor)
            .signature_evidence(
                &task(),
                &integrity_canonical::ResponseJson::from_value(&blank_time).unwrap()
            )
            .is_err()
    );
}

#[test]
fn canonical_signature_round_trip_has_one_timestamp_and_reader_preserves_response() {
    let response = canonical_response();
    let original = response.clone();
    let parsed = parse_authored_signatures(
        &integrity_canonical::ResponseJson::from_value(&response).unwrap(),
    )
    .unwrap();
    let encoded = serde_json::to_value(&parsed).unwrap();
    assert!(encoded[0].get("signedAt").is_none());
    assert_eq!(encoded[0]["signedPayload"]["signedAt"], SIGNED_AT);
    let decoded: Vec<wos_formspec_binding::FormspecAuthoredSignatureRef> =
        serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, parsed);
    let evidence = FormspecBinding::new(UnusedProcessor)
        .signature_evidence(
            &task(),
            &integrity_canonical::ResponseJson::from_value(&response).unwrap(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(evidence[0].signed_at, SIGNED_AT);
    assert!(evidence[0].admission_failure.is_some());
    assert_eq!(response, original);
}
