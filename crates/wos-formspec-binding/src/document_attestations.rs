// Rust guideline compliant 2026-02-21
//! Reuses native Formspec semantics and Integrity verification for committed claims.
//!
//! WOS authority is an assertion in signed `extensions.x-wos-signer-authority`.
//! The owning WOS schema validates its shape; runtime authorization and intent
//! floors remain independent. No key is selected from untrusted signature bytes.

use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use integrity_canonical::{DigestAlgorithm, build_signed_payload};
use integrity_signature::{KeyRef, MethodRegistry, VerificationResult, Verifier, VerifyRequest};
use serde::Deserialize;
use serde_json::Value;
use wos_runtime::binding::{
    DocumentSignatureClaims, SignatureAdmissionFailure, SignatureAdmissionFailureReason,
    SignatureEvidence, SignaturePrimitiveStatus, SourceSignatureRef,
};

use super::{
    BindingError, FormspecBinding, FormspecProcessor, method_uri_from_cose_sign1_b64,
    parse_authored_signatures_for_evidence, response_path,
};

const COMMITMENT: &str = "x-formspec-document-attestations";
const AUTHORITY: &str = "x-wos-signer-authority";

/// Immutable verifier and independently selected signer-key policy.
#[derive(Clone)]
pub struct DocumentSignatureVerification {
    verifier: Arc<dyn Verifier + Send + Sync>,
    registry: MethodRegistry,
    keys: BTreeMap<String, KeyRef>,
}

impl std::fmt::Debug for DocumentSignatureVerification {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DocumentSignatureVerification")
            .field("signers", &self.keys.keys())
            .finish_non_exhaustive()
    }
}

impl DocumentSignatureVerification {
    /// Selects the verifier, registry and trusted expected signer keys.
    #[must_use]
    pub fn new<V: Verifier + Send + Sync + 'static>(
        verifier: V,
        registry: MethodRegistry,
        keys: BTreeMap<String, KeyRef>,
    ) -> Self {
        Self {
            verifier: Arc::new(verifier),
            registry,
            keys,
        }
    }
}

fn invalid(message: impl Into<String>) -> BindingError {
    BindingError::InvalidInput(message.into())
}

pub(super) fn present_witness<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<SourceSignatureRef>>, D::Error> {
    Option::<SourceSignatureRef>::deserialize(deserializer).map(Some)
}

pub(super) fn has_document_claims(response: &Value) -> bool {
    response.get("resolvedDocuments").is_some()
        || response
            .get("extensions")
            .and_then(|v| v.get(COMMITMENT))
            .is_some()
        || response["authoredSignatures"]
            .as_array()
            .is_some_and(|signatures| {
                signatures.iter().any(|signature| {
                    [
                        "signingActId",
                        "documentRef",
                        "presentationHash",
                        "witnessedSignatureRef",
                    ]
                    .iter()
                    .any(|field| signature.get(*field).is_some())
                })
            })
}

fn authority(response: &Value) -> Result<Option<Value>, BindingError> {
    let selected = response
        .get("extensions")
        .and_then(|extensions| extensions.get(AUTHORITY));
    for previous in [
        response_path(response, "signerAuthority"),
        response_path(response, "signature.signerAuthority"),
    ]
    .into_iter()
    .flatten()
    {
        if Some(previous) != selected {
            return Err(invalid(
                "signed authority extension conflicts with other authority evidence",
            ));
        }
    }
    let Some(selected) = selected else {
        return Ok(None);
    };
    // This cache only avoids recompiling the immutable owning schema.
    static VALIDATOR: OnceLock<Result<jsonschema::Validator, String>> = OnceLock::new();
    let validator = VALIDATOR
        .get_or_init(|| {
            let schema: Value = serde_json::from_str(include_str!(
                "../../../schemas/wos-provenance-log.schema.json"
            ))
            .map_err(|error| error.to_string())?;
            jsonschema::options()
                .should_validate_formats(true)
                .build(&schema["$defs"]["SignatureSignerAuthority"])
                .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|_| invalid("owning authority schema initialization failed"))?;
    validator.validate(selected).map_err(|error| {
        invalid(format!(
            "signed authority shape invalid at {}",
            error.instance_path()
        ))
    })?;
    Ok(Some(selected.clone()))
}

impl<P> FormspecBinding<P> {
    /// Configures actual verification of declared-document source signatures.
    #[must_use]
    pub fn with_document_verification(
        mut self,
        verification: DocumentSignatureVerification,
    ) -> Self {
        self.document_verification = Some(verification);
        self
    }
}

impl<P: FormspecProcessor> FormspecBinding<P> {
    pub(super) fn document_signature_evidence(
        &self,
        response: &Value,
    ) -> Result<Vec<SignatureEvidence>, BindingError> {
        if response
            .get("extensions")
            .and_then(|v| v.get(COMMITMENT))
            .is_none()
        {
            return Err(invalid(
                "document admission requires the signed typed commitment",
            ));
        }
        let errors = self
            .processor
            .validate_document_attestation_response(response)?;
        if !errors.is_empty() {
            return Err(invalid("native declared-document evidence refused"));
        }
        let signatures = parse_authored_signatures_for_evidence(response)?;
        let first = signatures
            .first()
            .ok_or_else(|| invalid("prepared unsigned document evidence cannot be admitted"))?;
        if signatures
            .iter()
            .any(|signature| signature.admission_failure.is_some())
        {
            return Err(invalid(
                "declared-document source pins or canonical digest diverged",
            ));
        }
        let verification = self.document_verification.as_ref().ok_or_else(|| {
            BindingError::Unsupported("actual document verifier/key policy unavailable".into())
        })?;
        let signer = first
            .signature
            .signer_id
            .as_ref()
            .ok_or_else(|| invalid("committed signer identifier is required"))?;
        if response_path(response, "signerId")
            .and_then(Value::as_str)
            .is_some_and(|response_signer| response_signer != signer)
        {
            return Err(invalid(
                "authoredSignatures signerId must match response signerId",
            ));
        }
        let key_ref = verification
            .keys
            .get(signer)
            .ok_or_else(|| invalid("no trusted expected key selected for committed signer"))?;
        let signature_bytes = STANDARD
            .decode(
                first
                    .signature
                    .signature_value
                    .as_deref()
                    .unwrap_or_default(),
            )
            .map_err(|_| invalid("invalid document COSE base64"))?;
        if STANDARD.encode(&signature_bytes)
            != first
                .signature
                .signature_value
                .as_deref()
                .unwrap_or_default()
        {
            return Err(invalid("noncanonical document COSE base64"));
        }
        let method = method_uri_from_cose_sign1_b64(first.signature.signature_value.as_deref())
            .into_option()
            .ok_or_else(|| invalid("document COSE method unavailable"))?;
        if !method.starts_with("urn:formspec:sig-method:") {
            return Err(invalid(
                "document COSE method is outside the Formspec response domain",
            ));
        }
        let payload = build_signed_payload(response, DigestAlgorithm::Sha256)
            .map_err(|_| invalid("canonical document source refused"))?;
        // Native validation requires one identical COSE and digest for every record.
        let receipt = verification
            .verifier
            .verify(
                &VerifyRequest {
                    signed_bytes: payload.canonical_bytes,
                    signature_bytes,
                    method_uri: method.clone().into(),
                    key_ref: key_ref.clone(),
                },
                &verification.registry,
            )
            .map_err(|error| {
                invalid(format!("document verifier did not reach a result: {error}"))
            })?;
        let evidence_diverged = receipt.method.as_str() != method
            || receipt.method_registry_version != verification.registry.version;
        let failure_reason = if evidence_diverged {
            Some(SignatureAdmissionFailureReason::EvidenceDivergence)
        } else {
            match receipt.result {
                VerificationResult::Verified => None,
                VerificationResult::Failed => {
                    Some(SignatureAdmissionFailureReason::PrimitiveVerificationFailed)
                }
                VerificationResult::Unsupported => {
                    Some(SignatureAdmissionFailureReason::MethodUnsupported)
                }
            }
        };
        let primitive_verification = if evidence_diverged {
            SignaturePrimitiveStatus::DeferredPendingHelper {
                reason: "document-verification-evidence-divergence".into(),
            }
        } else {
            match receipt.result {
                VerificationResult::Verified => SignaturePrimitiveStatus::Verified,
                VerificationResult::Failed => SignaturePrimitiveStatus::Failed {
                    reason: "document-primitive-rejected".into(),
                },
                VerificationResult::Unsupported => {
                    SignaturePrimitiveStatus::DeferredPendingHelper {
                        reason: "document-primitive-unsupported".into(),
                    }
                }
            }
        };
        let admission_failure = failure_reason.map(|reason| SignatureAdmissionFailure {
            reason,
            failure_context: Some(serde_json::Map::from_iter([
                (
                    "verificationResult".into(),
                    serde_json::json!(receipt.result),
                ),
                ("methodUri".into(), serde_json::json!(method)),
                (
                    "registryVersion".into(),
                    serde_json::json!(verification.registry.version),
                ),
            ])),
        });
        // A reached negative verdict is auditable, but asserts no valid authority.
        let signer_authority = if admission_failure.is_none() {
            authority(response)?
        } else {
            None
        };
        let response_id = response["id"]
            .as_str()
            .filter(|id| !id.is_empty())
            .ok_or_else(|| invalid("intrinsic source Response id is required"))?;
        signatures
            .into_iter()
            .map(|evidence| {
                let signature = evidence.signature;
                let claims = DocumentSignatureClaims {
                    response_id: response_id.into(),
                    signing_act_id: signature
                        .signing_act_id
                        .ok_or_else(|| invalid("committed signing act is missing"))?,
                    document_ref: signature
                        .document_ref
                        .ok_or_else(|| invalid("committed document selection is missing"))?,
                    presentation_hash: signature.presentation_hash.ok_or_else(|| {
                        invalid("WOS requires an explicit committed presentation hash")
                    })?,
                    witnessed_signature_ref: signature
                        .witnessed_signature_ref
                        .ok_or_else(|| invalid("committed witness selection is missing"))?,
                };
                Ok(SignatureEvidence {
                    signing_act_id: Some(claims.signing_act_id.clone()),
                    presentation_hash: Some(claims.presentation_hash.clone()),
                    document_claims: Some(claims),
                    source_system: "formspec".into(),
                    source_signature_id: signature.signature_id,
                    source_response_ref: None,
                    document_id: signature.document_id,
                    signer_id: signature.signer_id,
                    signing_intent: signature.signing_intent,
                    signature_method: Some(method.clone()),
                    signed_payload_digest: signature.signed_payload.digest,
                    signed_payload_digest_algorithm: signature.signed_payload.digest_algorithm,
                    signed_at: signature.signed_payload.signed_at,
                    document_hash: signature.document_hash,
                    document_hash_algorithm: signature.document_hash_algorithm,
                    signature_provider: Some(signature.signature_provider),
                    ceremony_id: Some(signature.ceremony_id),
                    identity_binding: signature.identity_binding,
                    signer_authority: signer_authority.clone(),
                    primitive_verification: primitive_verification.clone(),
                    verification_receipt: receipt.receipt_bytes.clone(),
                    admission_failure: admission_failure.clone(),
                })
            })
            .collect()
    }
}
