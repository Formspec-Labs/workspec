// Rust guideline compliant 2026-02-21
//! Resolves authored recipes using actual saved native facts and signing primitives.
use crate::ConformanceError;
use crate::reference_replay::ReferenceReplayAdapters;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use integrity_canonical::{DigestAlgorithm, build_signed_payload};
use integrity_signature::{
    FixedClock, KeyRef, RegistryEntry, SignatureMethodRegistry, VerificationResult, Verifier,
    VerifyRequest,
};
use integrity_signature_ring::{InProcessReceiptSigner, RingVerifier, RingVerifierConfig};
use ring::signature::KeyPair;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use wos_core::instance::ActiveTask;
use wos_core::model::due_process::{DeterminationState, GovernanceActor, GovernanceRequest};
use wos_runtime::store::RuntimeRecord;

/// Stable reference resolved against actual saved native firings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeterminationTarget {
    /// Exact selected native determination binding.
    pub binding_ref: String,
    /// Zero-based actual saved firing occurrence under that binding.
    pub firing_occurrence: usize,
}
/// Authored native governance request with stable references.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GovernanceRecipe {
    /// Exact selected native operation.
    pub operation_ref: String,
    /// Actual prior determination, omitted only for pre-notice extension.
    pub target: Option<DeterminationTarget>,
    /// Native idempotency token.
    pub idempotency_token: String,
    /// Explicit independently admitted caller identity.
    pub actor: GovernanceActor,
    /// Closed command grammar validated before native dispatch.
    pub command: Value,
}
/// Explicit reviewed sign act over the actual generated task and determination.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SignatureRecipe {
    /// Actual prior saved determination.
    pub target: DeterminationTarget,
    /// Selected synthetic signing capability, separate from event data.
    pub signing_capability_ref: String,
    /// Normal authored-signature template with explicit consent and action.
    pub signature: Value,
}
fn fail(message: impl Into<String>) -> ConformanceError {
    ConformanceError::Parse(format!("native-recipe-refused: {}", message.into()))
}
fn fields(value: &Value, allowed: &[&str]) -> Result<(), ConformanceError> {
    let object = value
        .as_object()
        .ok_or_else(|| fail("expected closed object"))?;
    if object
        .keys()
        .any(|field| !allowed.contains(&field.as_str()))
    {
        return Err(fail("unknown recipe field"));
    }
    Ok(())
}
fn decode(value: &str) -> Result<Vec<u8>, ConformanceError> {
    let bytes = STANDARD
        .decode(value)
        .map_err(|error| fail(error.to_string()))?;
    if STANDARD.encode(&bytes) != value {
        return Err(fail("noncanonical key bytes"));
    }
    Ok(bytes)
}
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, ConformanceError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| fail(format!("missing {key}")))
}
/// Finds one actual saved native determining firing without predicting IDs.
pub(crate) fn resolve_target<'a>(
    record: &'a RuntimeRecord,
    target: &DeterminationTarget,
) -> Result<&'a DeterminationState, ConformanceError> {
    record
        .process
        .governance_state
        .as_ref()
        .and_then(|state| state.due_process.as_ref())
        .and_then(|state| {
            state
                .determinations
                .iter()
                .filter(|determination| determination.binding_ref == target.binding_ref)
                .nth(target.firing_occurrence)
        })
        .ok_or_else(|| fail("saved target absent"))
}
/// Builds the exact native request; the governor still independently admits it.
pub(crate) fn resolve_governance(
    record: &RuntimeRecord,
    recipe: &GovernanceRecipe,
) -> Result<GovernanceRequest, ConformanceError> {
    let kind = string(&recipe.command, "kind")?;
    let allowed: &[&str] = match kind {
        "issueNotice" | "applyAdverseEffect" => &["kind"],
        "fileAppeal" => &["kind", "channel", "continuationChoice"],
        "scheduleHearing" => &["kind", "reviewerActor", "calendarDaysFromFiling"],
        "holdHearing" => &["kind", "evidenceRefs"],
        "recordDisposition" => &["kind", "outcome", "rationale", "evidenceRefs"],
        "requestServiceTermination" => &["kind", "rationale"],
        "extendPreNoticeWindow" => &["kind", "calendarDays", "rationale", "evidenceRefs"],
        _ => return Err(fail("unsupported command")),
    };
    fields(&recipe.command, allowed)?;
    let target = recipe
        .target
        .as_ref()
        .map(|target| resolve_target(record, target))
        .transpose()?;
    if (kind == "extendPreNoticeWindow") != target.is_none() {
        return Err(fail("command determination target mismatch"));
    }
    let mut command = recipe.command.clone();
    if matches!(
        kind,
        "scheduleHearing" | "holdHearing" | "recordDisposition" | "requestServiceTermination"
    ) {
        let target = target.ok_or_else(|| fail("appeal target absent"))?;
        let mut appeals = record
            .process
            .governance_state
            .as_ref()
            .and_then(|state| state.due_process.as_ref())
            .ok_or_else(|| fail("appeal state absent"))?
            .appeals
            .iter()
            .filter(|appeal| appeal.determination_id == target.id);
        let appeal = appeals.next().ok_or_else(|| fail("saved appeal absent"))?;
        if appeals.next().is_some() {
            return Err(fail("saved appeal ambiguous"));
        }
        command["appealId"] = json!(appeal.id);
        if kind == "scheduleHearing" {
            let days = command
                .get("calendarDaysFromFiling")
                .and_then(Value::as_u64)
                .filter(|days| *days <= 36500)
                .ok_or_else(|| fail("invalid hearing offset"))?;
            let filed = time::OffsetDateTime::parse(
                &appeal.filed_at,
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|error| fail(error.to_string()))?;
            let scheduled = filed
                .checked_add(time::Duration::days(
                    i64::try_from(days).map_err(|_| fail("invalid offset"))?,
                ))
                .ok_or_else(|| fail("hearing time overflow"))?;
            command
                .as_object_mut()
                .ok_or_else(|| fail("invalid command"))?
                .remove("calendarDaysFromFiling");
            command["scheduledFor"] = json!(
                scheduled
                    .format(&time::format_description::well_known::Rfc3339)
                    .map_err(|error| fail(error.to_string()))?
            );
        }
    }
    let command = serde_json::from_value(command).map_err(|error| fail(error.to_string()))?;
    Ok(GovernanceRequest {
        operation_ref: recipe.operation_ref.clone(),
        determination_id: target.map(|target| target.id.clone()),
        idempotency_token: recipe.idempotency_token.clone(),
        actor: recipe.actor.clone(),
        command,
    })
}

/// Signs the actual resolved complete Response through maintained primitives.
pub(crate) fn resolve_signature(
    record: &RuntimeRecord,
    task: &ActiveTask,
    actor: &str,
    response: &Value,
    recipe: &SignatureRecipe,
    adapters: &ReferenceReplayAdapters,
    now_ms: u64,
) -> Result<(Value, Value), ConformanceError> {
    let target = resolve_target(record, &recipe.target)?;
    let capability = adapters
        .fixture_signing_capabilities
        .iter()
        .find(|capability| capability.id == recipe.signing_capability_ref)
        .ok_or_else(|| fail("signing capability absent"))?;
    let key = adapters
        .document_signing_keys
        .iter()
        .find(|key| key.id == capability.document_key_ref)
        .ok_or_else(|| fail("document key absent"))?;
    let receipt_key = adapters
        .trusted_receipt_signers
        .iter()
        .find(|key| key.id == capability.receipt_signer_ref)
        .ok_or_else(|| fail("receipt key absent"))?;
    if key.task_ref != task.task_ref
        || key.signer_human != actor
        || task.assigned_actor.as_deref() != Some(actor)
    {
        return Err(fail("task/signer association differs"));
    }
    fields(
        &recipe.signature,
        &[
            "signatureId",
            "documentId",
            "signingIntent",
            "documentHash",
            "documentHashAlgorithm",
            "signatureProvider",
            "ceremonyId",
            "signerId",
            "signerName",
            "consentAccepted",
            "consentTextRef",
            "consentVersion",
            "affirmationText",
            "identityBinding",
        ],
    )?;
    if recipe.signature.get("consentAccepted") != Some(&json!(true))
        || string(&recipe.signature, "signerId")? != actor
    {
        return Err(fail("explicit sign consent/actor missing"));
    }
    for field in [
        "signatureId",
        "documentId",
        "signingIntent",
        "documentHash",
        "documentHashAlgorithm",
        "signatureProvider",
        "ceremonyId",
        "signerName",
        "consentTextRef",
        "consentVersion",
        "affirmationText",
    ] {
        string(&recipe.signature, field)?;
    }
    if response.get("authoredSignatures").is_some()
        || response.get("sourceResponseRef").is_some()
        || response
            .pointer("/extensions/x-formspec-signing-content")
            .is_some()
    {
        return Err(fail("unsupported preauthored signature/commitment"));
    }
    let mut unsigned = response.clone();
    let data = unsigned
        .get_mut("data")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| fail("complete Response data absent"))?;
    for (field, value) in [
        ("determinationId", &target.id),
        ("determinationSnapshotSha256", &target.snapshot.sha256),
        (
            "determinationFiringRecordId",
            &target.firing.provenance_record_id,
        ),
        ("signatureTaskId", &task.task_id),
    ] {
        if data.contains_key(field) {
            return Err(fail("reserved signature association already supplied"));
        }
        data.insert(field.into(), json!(value));
    }
    let source_pair =
        ring::signature::Ed25519KeyPair::from_pkcs8(&decode(&capability.pkcs8_base64)?)
            .map_err(|_| fail("invalid fixture source private key"))?;
    let receipt_pair =
        ring::signature::Ed25519KeyPair::from_pkcs8(&decode(&capability.receipt_pkcs8_base64)?)
            .map_err(|_| fail("invalid fixture receipt private key"))?;
    if source_pair.public_key().as_ref() != decode(&key.public_key_base64)?
        || receipt_pair.public_key().as_ref() != decode(&receipt_key.public_key_base64)?
    {
        return Err(fail("fixture private/public keys differ"));
    }
    let instant = time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(now_ms) * 1_000_000)
        .map_err(|error| fail(error.to_string()))?
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|error| fail(error.to_string()))?;
    let payload = build_signed_payload(unsigned.to_string().as_bytes(), DigestAlgorithm::Sha256)
        .map_err(fail)?;
    let protected = integrity_cose::detached_signature_protected_header(
        -8,
        &decode(&key.kid_base64)?,
        &key.method_uri,
    );
    let signature = source_pair.sign(&integrity_cose::sig_structure_bytes(
        &protected,
        &payload.canonical_bytes,
    ));
    let cose = integrity_cose::encode_cose_sign1(&protected, None, signature.as_ref());
    let signer = InProcessReceiptSigner::new_with_identity(
        receipt_pair,
        Some(&decode(&receipt_key.kid_base64)?),
        receipt_key.id.clone(),
        receipt_key.method_uri.clone(),
    );
    let clock = FixedClock::at_rfc3339(&instant).map_err(|error| fail(error.to_string()))?;
    let verifier = RingVerifier::new_with_clock_config_and_receipt_signer(
        Arc::new(clock),
        RingVerifierConfig::new("urn:formspec:sig-method:")
            .with_adapter_id("urn:formspec:adapter:ring@1")
            .with_adapter_version("0.1.0")
            .with_receipt_payload_domain("formspec.verification.receipt.v1"),
        Arc::new(signer),
    );
    let receipt = verifier
        .verify(
            &VerifyRequest {
                signed_bytes: payload.canonical_bytes,
                signature_bytes: cose.clone(),
                method_uri: key.method_uri.clone().into(),
                key_ref: KeyRef::RawPublicKey(decode(&key.public_key_base64)?),
            },
            &SignatureMethodRegistry {
                version: "1.0.0".into(),
                entries: vec![RegistryEntry {
                    id: key.method_uri.clone().into(),
                    suite: "ed25519".into(),
                    wire: "cose-sign1".into(),
                    alg: Some(-8),
                    status: "registered".into(),
                    deprecation_notice: None,
                }],
            },
        )
        .map_err(|error| fail(error.to_string()))?;
    if receipt.result != VerificationResult::Verified {
        return Err(fail("actual fixture subject did not verify"));
    }
    let mut signature = recipe.signature.clone();
    signature["signatureValue"] = json!(STANDARD.encode(cose));
    signature["verificationReceipt"] =
        serde_json::to_value(&receipt).map_err(|error| fail(error.to_string()))?;
    signature["signedPayload"] = json!({"canonicalization":integrity_canonical::CANONICALIZATION_PROFILE,"digestAlgorithm":"sha-256","digest":payload.digest,
        "responseId":string(&unsigned,"id")?,"definitionUrl":string(&unsigned,"definitionUrl")?,"definitionVersion":string(&unsigned,"definitionVersion")?,"signedAt":instant,"signingIntent":string(&recipe.signature,"signingIntent")?});
    let mut signed = unsigned.clone();
    signed["authoredSignatures"] = json!([signature]);
    let resolution = json!({"recipe":recipe,"unsignedResponse":unsigned,"signedResponse":signed,"documentKeyRef":key.id,"receiptSignerRef":receipt_key.id,"scope":"softwareFixture","signingTime":instant,"receipt":receipt});
    Ok((signed, resolution))
}
