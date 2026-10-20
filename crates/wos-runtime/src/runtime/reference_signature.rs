// Rust guideline compliant 2026-02-21
//! Authenticates the actual Response subject alongside normal signature admission.
use std::sync::Arc;

use base64::{Engine as _, engine::general_purpose::STANDARD};
#[cfg(test)]
use integrity_canonical::build_signed_payload;
use integrity_canonical::{DigestAlgorithm, ResponseJson};
use integrity_cose::{decode_cose_sign1, decode_protected_header};
use integrity_signature::{
    FixedClock, KeyRef, RegistryEntry, SignatureMethodRegistry, VerificationResult, Verifier,
    VerifyRequest,
};
use integrity_signature_ring::{RingVerifier, RingVerifierConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wos_core::ProvenanceKind;
use wos_core::model::due_process::{
    DeterminationSignature, DeterminationState, DueProcessExecution, RefusalCode,
};
use wos_core::model::kernel::KernelDocument;

use super::governance::ReferenceDeterminationSignatureVerifier;
use crate::store::{RuntimeRecord, TaskArtifactKind};

const METHOD: &str = "urn:formspec:sig-method:ed25519-cose-sign1@1";
const REGISTRY_VERSION: &str = "1.0.0";

/// One immutable source signing key, separate from receipt-signing trust.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectedDocumentSigningKey {
    /// Stable selected key identity.
    pub id: String,
    /// Exact native task declaration this key may sign.
    pub task_ref: String,
    /// Canonical admitted human owner of this signing key.
    pub signer_human: String,
    /// Canonical base64 COSE protected key identifier.
    pub kid_base64: String,
    /// Canonical base64 Ed25519 public key.
    pub public_key_base64: String,
    /// Exact registered source-signature method.
    pub method_uri: String,
}

/// Verifies selected source keys against actual retained accepted Responses.
#[derive(Debug)]
pub struct SelectedDeterminationSignatureVerifier {
    keys: Vec<SelectedDocumentSigningKey>,
}

impl SelectedDeterminationSignatureVerifier {
    /// Admits immutable task-bound document keys.
    ///
    /// # Errors
    /// Rejects duplicate identities, unsupported methods or malformed key bytes.
    pub fn new(keys: Vec<SelectedDocumentSigningKey>) -> Result<Self, RefusalCode> {
        let mut ids = std::collections::HashSet::new();
        let mut scopes = std::collections::HashSet::new();
        for key in &keys {
            if key.id.is_empty()
                || !ids.insert(&key.id)
                || key.task_ref.is_empty()
                || !key.signer_human.starts_with("actor:human:")
                || key.method_uri != METHOD
                || decode(&key.kid_base64)?.is_empty()
                || decode(&key.public_key_base64)?.len() != 32
                || !scopes.insert((&key.task_ref, &key.signer_human, &key.kid_base64))
            {
                return Err(RefusalCode::InvalidConfiguration);
            }
        }
        Ok(Self { keys })
    }
}

fn decode(value: &str) -> Result<Vec<u8>, RefusalCode> {
    let bytes = STANDARD
        .decode(value)
        .map_err(|_| RefusalCode::SignatureRejected)?;
    if STANDARD.encode(&bytes) != value {
        return Err(RefusalCode::SignatureRejected);
    }
    Ok(bytes)
}

fn text<'a>(value: &'a Value, pointer: &str) -> Result<&'a str, RefusalCode> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or(RefusalCode::SignatureRejected)
}

fn timestamp(value: &str) -> Result<u64, RefusalCode> {
    let millis = chrono::DateTime::parse_from_rfc3339(value)
        .map_err(|_| RefusalCode::SignatureRejected)?
        .timestamp_millis();
    u64::try_from(millis).map_err(|_| RefusalCode::SignatureRejected)
}

/// Authenticates one source envelope over the complete actual Response.
fn verify_subject(
    source: &ResponseJson,
    signature: &Value,
    key: &SelectedDocumentSigningKey,
    task_id: &str,
    determination: &DeterminationState,
    now_ms: u64,
) -> Result<String, RefusalCode> {
    let response = source.value();
    if text(response, "/data/determinationId")? != determination.id
        || text(response, "/data/determinationSnapshotSha256")? != determination.snapshot.sha256
        || text(response, "/data/determinationFiringRecordId")?
            != determination.firing.provenance_record_id
        || text(response, "/data/signatureTaskId")? != task_id
        || text(signature, "/signerId")? != key.signer_human
        || signature.get("consentAccepted") != Some(&Value::Bool(true))
    {
        return Err(RefusalCode::SignatureRejected);
    }
    let payload = source
        .signed_payload(DigestAlgorithm::Sha256)
        .map_err(|_| RefusalCode::SignatureRejected)?;
    if text(signature, "/signedPayload/canonicalization")?
        != integrity_canonical::CANONICALIZATION_PROFILE
        || text(signature, "/signedPayload/digestAlgorithm")? != "sha-256"
        || text(signature, "/signedPayload/digest")? != payload.digest
        || text(signature, "/signedPayload/responseId")? != text(response, "/id")?
        || text(signature, "/signedPayload/definitionUrl")? != text(response, "/definitionUrl")?
        || text(signature, "/signedPayload/definitionVersion")?
            != text(response, "/definitionVersion")?
    {
        return Err(RefusalCode::SignatureRejected);
    }
    let signature_bytes = decode(text(signature, "/signatureValue")?)?;
    let envelope =
        decode_cose_sign1(&signature_bytes).map_err(|_| RefusalCode::SignatureRejected)?;
    let header = decode_protected_header(envelope.protected_header())
        .map_err(|_| RefusalCode::SignatureRejected)?;
    if envelope.alg() != Some(-8)
        || envelope.kid() != Some(decode(&key.kid_base64)?.as_slice())
        || header.method_uri.as_deref() != Some(key.method_uri.as_str())
    {
        return Err(RefusalCode::SignatureRejected);
    }
    let now = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
        i64::try_from(now_ms).map_err(|_| RefusalCode::ClockRejected)?,
    )
    .ok_or(RefusalCode::ClockRejected)?
    .to_rfc3339();
    let clock = FixedClock::at_rfc3339(&now).map_err(|_| RefusalCode::ClockRejected)?;
    let verifier = RingVerifier::new_with_clock_and_config(
        Arc::new(clock),
        RingVerifierConfig::new("urn:formspec:sig-method:")
            .with_adapter_id("urn:formspec:adapter:ring@1")
            .with_adapter_version("0.1.0")
            .with_receipt_payload_domain("formspec.verification.receipt.v1"),
    );
    let receipt = verifier
        .verify(
            &VerifyRequest {
                signed_bytes: payload.canonical_bytes,
                signature_bytes,
                method_uri: key.method_uri.clone().into(),
                key_ref: KeyRef::RawPublicKey(decode(&key.public_key_base64)?),
            },
            &SignatureMethodRegistry {
                version: REGISTRY_VERSION.into(),
                entries: vec![RegistryEntry {
                    id: METHOD.into(),
                    suite: "ed25519".into(),
                    wire: "cose-sign1".into(),
                    alg: Some(-8),
                    status: "registered".into(),
                    deprecation_notice: None,
                }],
            },
        )
        .map_err(|_| RefusalCode::SignatureRejected)?;
    if receipt.result != VerificationResult::Verified {
        return Err(RefusalCode::SignatureRejected);
    }
    Ok(format!("sha256:{}", payload.digest))
}

impl ReferenceDeterminationSignatureVerifier for SelectedDeterminationSignatureVerifier {
    fn validate_configuration(
        &self,
        kernel: &KernelDocument,
        execution: &DueProcessExecution,
    ) -> Result<(), RefusalCode> {
        fn actions<'a>(
            state: &'a wos_core::model::kernel::State,
            out: &mut Vec<&'a wos_core::model::kernel::Action>,
        ) {
            out.extend(
                state.on_entry.iter().chain(&state.on_exit).chain(
                    state
                        .transitions
                        .iter()
                        .flat_map(|transition| &transition.actions),
                ),
            );
            for state in state.states.values() {
                actions(state, out);
            }
            for region in state.regions.values() {
                for state in region.states.values() {
                    actions(state, out);
                }
            }
            if let Some(body) = state.body.as_ref() {
                actions(body, out);
            }
        }
        let mut declared = Vec::new();
        for state in kernel.lifecycle.states.values() {
            actions(state, &mut declared);
        }
        for determination in &execution.determinations {
            let tasks: Vec<_> = declared
                .iter()
                .copied()
                .filter(|action| {
                    action.action == wos_core::model::kernel::ActionKind::CreateTask
                        && action.task_ref.as_deref()
                            == Some(determination.signature_task_ref.as_str())
                })
                .collect();
            if tasks.is_empty()
                || !self
                    .keys
                    .iter()
                    .any(|key| key.task_ref == determination.signature_task_ref)
            {
                return Err(RefusalCode::SignatureRejected);
            }
            for task in tasks {
                let contract = task
                    .contract_ref
                    .as_ref()
                    .and_then(|reference| kernel.contracts.get(reference))
                    .ok_or(RefusalCode::SignatureRejected)?;
                let actor = task
                    .assign_to
                    .as_ref()
                    .and_then(|reference| kernel.actors.iter().find(|actor| actor.id == *reference))
                    .ok_or(RefusalCode::SignatureRejected)?;
                if contract.binding != "formspec"
                    || contract
                        .definition_version
                        .as_deref()
                        .is_none_or(str::is_empty)
                    || actor.kind != wos_core::model::kernel::ActorKind::Human
                    || !["x-wos-signature-profile-key", "x-wos-signature-profile-ref"]
                        .iter()
                        .any(|field| {
                            task.extensions
                                .get(*field)
                                .and_then(Value::as_str)
                                .is_some_and(|value| !value.is_empty())
                        })
                    || task
                        .extensions
                        .get("x-wos-signature-step-id")
                        .and_then(Value::as_str)
                        .is_none_or(str::is_empty)
                {
                    return Err(RefusalCode::SignatureRejected);
                }
            }
        }
        Ok(())
    }
    fn verify(
        &self,
        record: &RuntimeRecord,
        kernel: &KernelDocument,
        determination: &DeterminationState,
        now_ms: u64,
    ) -> Result<DeterminationSignature, RefusalCode> {
        let execution: DueProcessExecution = serde_json::from_value(
            kernel
                .governance
                .as_ref()
                .and_then(|value| value.pointer("/dueProcess/execution"))
                .ok_or(RefusalCode::SignatureRejected)?
                .clone(),
        )
        .map_err(|_| RefusalCode::SignatureRejected)?;
        let binding = execution
            .determinations
            .iter()
            .find(|binding| binding.id == determination.binding_ref)
            .ok_or(RefusalCode::SignatureRejected)?;
        let mut candidates = Vec::new();
        for artifact in record
            .artifacts
            .values()
            .filter(|artifact| artifact.kind == TaskArtifactKind::Accepted)
        {
            let task_origins: Vec<_> = record
                .provenance_log
                .iter()
                .filter(|provenance| {
                    provenance.record_kind == ProvenanceKind::TaskCreated
                        && provenance
                            .data
                            .as_ref()
                            .and_then(|value| value.get("taskId"))
                            .and_then(Value::as_str)
                            == Some(&artifact.task_id)
                        && provenance
                            .data
                            .as_ref()
                            .and_then(|value| value.pointer("/details/taskRef"))
                            .and_then(Value::as_str)
                            == Some(&binding.signature_task_ref)
                })
                .collect();
            if task_origins.len() != 1 || artifact.actor_id == determination.determining_human {
                continue;
            }
            if timestamp(&artifact.recorded_at)? < timestamp(&determination.determined_at)?
                || timestamp(&artifact.recorded_at)? > now_ms
            {
                continue;
            }
            let signatures = match artifact
                .response
                .value()
                .get("authoredSignatures")
                .and_then(Value::as_array)
            {
                Some(signatures) => signatures,
                None => continue,
            };
            for signature in signatures {
                let matching_keys: Vec<_> = self
                    .keys
                    .iter()
                    .filter(|key| {
                        key.task_ref == binding.signature_task_ref
                            && key.signer_human == artifact.actor_id
                    })
                    .collect();
                for key in matching_keys {
                    let Ok(digest) = verify_subject(
                        &artifact.response,
                        signature,
                        key,
                        &artifact.task_id,
                        determination,
                        now_ms,
                    ) else {
                        continue;
                    };
                    let signature_id = text(signature, "/signatureId")?;
                    let affirmations: Vec<_> = record
                        .provenance_log
                        .iter()
                        .filter(|provenance| {
                            provenance.record_kind == ProvenanceKind::SignatureAffirmation
                                && provenance.actor_id.as_deref() == Some(&artifact.actor_id)
                                && provenance
                                    .data
                                    .as_ref()
                                    .and_then(|value| value.get("sourceResponseRef"))
                                    .and_then(Value::as_str)
                                    == Some(&artifact.task_id)
                                && provenance
                                    .data
                                    .as_ref()
                                    .and_then(|value| value.get("sourceSignatureId"))
                                    .and_then(Value::as_str)
                                    == Some(signature_id)
                                && provenance
                                    .data
                                    .as_ref()
                                    .and_then(|value| value.get("signedPayloadDigest"))
                                    .and_then(Value::as_str)
                                    == digest.strip_prefix("sha256:")
                        })
                        .collect();
                    if affirmations.len() != 1
                        || affirmations[0]
                            .data
                            .as_ref()
                            .and_then(|value| value.pointer("/primitiveVerification/status"))
                            .and_then(Value::as_str)
                            != Some("verified")
                    {
                        continue;
                    }
                    candidates.push(DeterminationSignature {
                        task_id: artifact.task_id.clone(),
                        affirmation_record_id: affirmations[0].id.clone(),
                        supervisor_human: artifact.actor_id.clone(),
                        signed_content_sha256: digest,
                    });
                }
            }
        }
        match candidates.as_slice() {
            [only] => Ok(only.clone()),
            _ => Err(RefusalCode::SignatureRejected),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::signature::KeyPair;
    use serde_json::json;

    fn signed() -> (Value, Value, SelectedDocumentSigningKey, DeterminationState) {
        let rng = ring::rand::SystemRandom::new();
        let pkcs8 = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let pair = ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
        let key = SelectedDocumentSigningKey {
            id: "supervisor-key".into(),
            task_ref: "supervisor".into(),
            signer_human: "actor:human:supervisor".into(),
            kid_base64: STANDARD.encode(b"supervisor-key"),
            public_key_base64: STANDARD.encode(pair.public_key().as_ref()),
            method_uri: METHOD.into(),
        };
        let determination:DeterminationState=serde_json::from_value(json!({
            "id":"actual-determination","bindingRef":"determination",
            "firing":{"definitionSha256":format!("sha256:{}","a".repeat(64)),"location":{"statePointer":"/lifecycle/states/review","transitionIndex":0},"provenanceRecordId":"actual-firing"},
            "determinedAt":"2026-01-01T00:00:00.000Z","snapshot":{"recordId":"snapshot","sha256":"b".repeat(64)},
            "determiningHuman":"actor:human:caseworker","servicePointers":["/benefitAmount"]
        })).unwrap();
        let response = json!({"id":"actual-response","definitionUrl":"urn:test:sign","definitionVersion":"1.0.0","status":"completed",
"data":{"determinationId":"actual-determination",
                "determinationSnapshotSha256":"b".repeat(64),"determinationFiringRecordId":"actual-firing","signatureTaskId":"actual-task"}});
        let payload =
            build_signed_payload(response.to_string().as_bytes(), DigestAlgorithm::Sha256).unwrap();
        let protected =
            integrity_cose::detached_signature_protected_header(-8, b"supervisor-key", METHOD);
        let structure = integrity_cose::sig_structure_bytes(&protected, &payload.canonical_bytes);
        let signature = pair.sign(&structure);
        let cose = integrity_cose::encode_cose_sign1(&protected, None, signature.as_ref());
        let signature = json!({"signerId":key.signer_human,"consentAccepted":true,
            "signatureValue":STANDARD.encode(cose),"signedPayload":{"canonicalization":integrity_canonical::CANONICALIZATION_PROFILE,
                "digestAlgorithm":"sha-256","digest":payload.digest,"responseId":"actual-response",
                "definitionUrl":"urn:test:sign","definitionVersion":"1.0.0"}});
        (response, signature, key, determination)
    }

    #[test]
    fn exact_subject_signature_verifies_and_modified_subject_with_valid_receipt_refuses() {
        let (response, signature, key, determination) = signed();
        assert!(
            verify_subject(
                &integrity_canonical::ResponseJson::from_value(&response).unwrap(),
                &signature,
                &key,
                "actual-task",
                &determination,
                1_800_000_000_000
            )
            .is_ok()
        );
        let mut changed = response.clone();
        changed["data"]["amount"] = json!(0);
        let mut transplanted = signature.clone();
        transplanted["verificationReceipt"] = json!({"result":"Verified"});
        transplanted["signedPayload"]["digest"] = json!(
            build_signed_payload(changed.to_string().as_bytes(), DigestAlgorithm::Sha256)
                .unwrap()
                .digest
        );
        assert!(
            verify_subject(
                &integrity_canonical::ResponseJson::from_value(&changed).unwrap(),
                &transplanted,
                &key,
                "actual-task",
                &determination,
                1_800_000_000_000
            )
            .is_err()
        );
    }

    #[test]
    fn signatures_cannot_move_to_another_task_determination_snapshot_or_firing() {
        let (response, signature, key, determination) = signed();
        assert!(
            verify_subject(
                &integrity_canonical::ResponseJson::from_value(&response).unwrap(),
                &signature,
                &key,
                "different-task",
                &determination,
                1_800_000_000_000
            )
            .is_err()
        );
        for pointer in [
            "/data/determinationId",
            "/data/determinationSnapshotSha256",
            "/data/determinationFiringRecordId",
            "/data/signatureTaskId",
        ] {
            let mut changed = response.clone();
            *changed.pointer_mut(pointer).unwrap() = json!("different");
            let mut transplanted = signature.clone();
            transplanted["signedPayload"]["digest"] = json!(
                build_signed_payload(changed.to_string().as_bytes(), DigestAlgorithm::Sha256)
                    .unwrap()
                    .digest
            );
            assert!(
                verify_subject(
                    &integrity_canonical::ResponseJson::from_value(&changed).unwrap(),
                    &transplanted,
                    &key,
                    "actual-task",
                    &determination,
                    1_800_000_000_000
                )
                .is_err(),
                "{pointer}"
            );
        }
        let mut other = determination.clone();
        other.id = "different-determination".into();
        assert!(
            verify_subject(
                &integrity_canonical::ResponseJson::from_value(&response).unwrap(),
                &signature,
                &key,
                "actual-task",
                &other,
                1_800_000_000_000
            )
            .is_err()
        );
    }
}
