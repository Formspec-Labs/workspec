// Rust guideline compliant 2026-02-21

//! Formspec binding adapter for `wos-runtime`.

use integrity_canonical::{CANONICALIZATION_PROFILE, DigestAlgorithm, build_signed_payload};
use wos_core::{
    instance::{ActiveTask, ValidationOutcome, WorkflowProcess},
    provenance::{ProvenanceKind, ProvenanceRecord},
};
use wos_runtime::binding::{
    BindingError, CaseMutationBundle, ContractBindingAdapter, PreparedTask,
    SignatureAdmissionFailure, SignatureAdmissionFailureReason, SignatureEvidence,
    SignaturePrimitiveStatus, SubmissionValidation,
};
use wos_runtime::intake::{
    IntakeAcceptanceAdapter, IntakeAcceptanceOutcome, IntakeAcceptanceRequest,
    IntakeCaseDisposition, IntakeCaseIntent, IntakeInterpretation,
};

const FORMSPEC_SIGNATURE_METHOD_REGISTRY_VERSION: &str = "1.0.0";
const WOS_KERNEL_CASE_CREATED_EVENT: &str = "wos.kernel.case_created";

/// Stable reason emitted when the reference Formspec binding has parsed and
/// pre-checked an authored signature but has not yet run the cryptographic
/// primitive (pending the Formspec signing helper, `FORMSPEC-SIGN-HELPER-001`).
pub const FORMSPEC_SIGNING_HELPER_PENDING_REASON: &str = "formspec-signing-helper-pending";

/// Case action implied by a Formspec intake handoff.
///
/// This is a WOS-side interpretation of the Formspec handoff mode. It does not
/// create a case by itself; runtime policy decides whether an accepted public
/// intake becomes a governed case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntakeHandoffCaseIntent {
    /// Attach the intake evidence to an already-governed case.
    AttachToExistingCase {
        /// Existing governed case reference.
        case_ref: String,
    },

    /// Create a governed case after accepting the intake evidence.
    CreateCaseAfterAcceptance,
}

/// Formspec intake initiation topology.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IntakeHandoffInitiationMode {
    /// A workflow task or existing case requested this intake.
    WorkflowInitiated,

    /// A respondent started from an open intake surface.
    PublicIntake,
}

impl IntakeHandoffInitiationMode {
    fn as_str(&self) -> &'static str {
        match self {
            IntakeHandoffInitiationMode::WorkflowInitiated => "workflowInitiated",
            IntakeHandoffInitiationMode::PublicIntake => "publicIntake",
        }
    }
}

/// Pinned Formspec definition identity for intake acceptance.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IntakeDefinitionRef {
    /// Canonical Formspec Definition URL.
    pub url: String,

    /// Exact Formspec Definition version.
    pub version: String,
}

/// Formspec-to-WOS intake handoff boundary record.
///
/// The structure mirrors `schemas/intake-handoff.schema.json` and keeps WOS
/// case ownership explicit. Use [`parse_intake_handoff`] to deserialize and
/// validate mode-specific invariants before applying workflow policy.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IntakeHandoff {
    /// Intake Handoff schema version.
    #[serde(rename = "$formspecIntakeHandoff")]
    pub schema_version: String,

    /// Stable idempotency and trace handle for this handoff.
    pub handoff_id: String,

    /// Case initiation topology.
    pub initiation_mode: IntakeHandoffInitiationMode,

    /// Existing governed case reference, when one exists.
    #[serde(default)]
    pub case_ref: Option<String>,

    /// Pinned Formspec Definition identity.
    pub definition_ref: IntakeDefinitionRef,

    /// Reference to the canonical Formspec Response.
    pub response_ref: String,

    /// Algorithm-prefixed digest of the Response envelope.
    pub response_hash: String,

    /// Reference to the immutable ValidationReport snapshot.
    pub validation_report_ref: String,

    /// Intake session that produced the handoff.
    pub intake_session_id: String,

    /// Actor that submitted or caused the handoff.
    #[serde(default)]
    pub actor_ref: Option<String>,

    /// Person, organization, asset, or matter the intake concerns.
    #[serde(default)]
    pub subject_ref: Option<String>,

    /// Respondent-ledger head event or checkpoint at handoff time.
    pub ledger_head_ref: String,

    /// Timestamp when the handoff was produced.
    pub occurred_at: String,

    /// Namespaced extension data.
    #[serde(default)]
    pub extensions: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Formspec signed-payload pin consumed by WOS signature binding code.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormspecSignedPayloadRef {
    /// Canonicalization profile used to hash the signed payload.
    pub canonicalization: String,

    /// Digest algorithm used for `digest`.
    pub digest_algorithm: String,

    /// Digest of the Formspec Signed Response Payload.
    pub digest: String,

    /// Response id pinned by the signature.
    pub response_id: String,

    /// Definition URL pinned by the signature.
    pub definition_url: String,

    /// Definition version pinned by the signature.
    pub definition_version: String,
}

/// Minimal authored-signature shape WOS needs to bind Formspec evidence.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormspecAuthoredSignatureRef {
    /// Stable signature identifier.
    pub signature_id: String,

    /// Signable document identifier.
    pub document_id: String,

    /// Legal-effect class authored into the Formspec response.
    pub signing_intent: String,

    /// Cryptographic signature method URI.
    #[serde(default)]
    pub signature_method: Option<String>,

    /// Signer display name required by Formspec schema.
    pub signer_name: String,

    /// Signature timestamp.
    pub signed_at: String,

    /// Consent flag accepted by the signer.
    pub consent_accepted: bool,

    /// Consent text reference.
    pub consent_text_ref: String,

    /// Consent text version.
    pub consent_version: String,

    /// Signed-payload digest pins.
    pub signed_payload: FormspecSignedPayloadRef,

    /// Signing-surface or rendered-document hash.
    pub document_hash: String,

    /// Signing-surface digest algorithm.
    pub document_hash_algorithm: String,

    /// Provider that supplied the signature ceremony.
    pub signature_provider: String,

    /// Provider or adapter ceremony identifier.
    pub ceremony_id: String,

    /// Stable signer identifier, when present.
    #[serde(default)]
    pub signer_id: Option<String>,

    /// Provider-neutral identity binding, when Formspec captured it.
    #[serde(default)]
    pub identity_binding: Option<serde_json::Value>,

    /// Base64-encoded COSE_Sign1 VerificationReceipt bytes.
    #[serde(default)]
    pub verification_receipt: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct FormspecAuthoredSignatureEvidence {
    signature: FormspecAuthoredSignatureRef,
    admission_failure: Option<SignatureAdmissionFailure>,
}

impl IntakeHandoff {
    /// Return the WOS case intent represented by this handoff.
    ///
    /// # Errors
    ///
    /// Returns [`BindingError::InvalidInput`] if the handoff was manually
    /// constructed without satisfying the schema-level mode invariants.
    pub fn case_intent(&self) -> Result<IntakeHandoffCaseIntent, BindingError> {
        self.validate()?;
        match self.initiation_mode {
            IntakeHandoffInitiationMode::WorkflowInitiated => {
                Ok(IntakeHandoffCaseIntent::AttachToExistingCase {
                    case_ref: self.case_ref.clone().ok_or_else(|| {
                        BindingError::InvalidInput(
                            "workflowInitiated intake handoff requires caseRef".to_string(),
                        )
                    })?,
                })
            }
            IntakeHandoffInitiationMode::PublicIntake => {
                Ok(IntakeHandoffCaseIntent::CreateCaseAfterAcceptance)
            }
        }
    }

    fn validate(&self) -> Result<(), BindingError> {
        if self.schema_version != "1.0" {
            return Err(BindingError::InvalidInput(
                "intake handoff $formspecIntakeHandoff must be '1.0'".to_string(),
            ));
        }

        ensure_non_empty("handoffId", &self.handoff_id)?;
        ensure_non_empty("definitionRef.url", &self.definition_ref.url)?;
        ensure_non_empty("definitionRef.version", &self.definition_ref.version)?;
        ensure_non_empty("responseRef", &self.response_ref)?;
        ensure_non_empty("responseHash", &self.response_hash)?;
        ensure_non_empty("validationReportRef", &self.validation_report_ref)?;
        ensure_non_empty("intakeSessionId", &self.intake_session_id)?;
        ensure_non_empty("ledgerHeadRef", &self.ledger_head_ref)?;
        ensure_non_empty("occurredAt", &self.occurred_at)?;

        if !is_valid_hash_string(&self.response_hash) {
            return Err(BindingError::InvalidInput(
                "intake handoff responseHash must match the Formspec HashString pattern"
                    .to_string(),
            ));
        }

        if let Some(actor_ref) = &self.actor_ref {
            ensure_non_empty("actorRef", actor_ref)?;
        }
        if let Some(subject_ref) = &self.subject_ref {
            ensure_non_empty("subjectRef", subject_ref)?;
        }

        match self.initiation_mode {
            IntakeHandoffInitiationMode::WorkflowInitiated => {
                let Some(case_ref) = &self.case_ref else {
                    return Err(BindingError::InvalidInput(
                        "workflowInitiated intake handoff requires caseRef".to_string(),
                    ));
                };
                ensure_non_empty("caseRef", case_ref)?;
            }
            IntakeHandoffInitiationMode::PublicIntake => {
                if self.case_ref.is_some() {
                    return Err(BindingError::InvalidInput(
                        "publicIntake intake handoff must not include caseRef".to_string(),
                    ));
                }
            }
        }

        if let Some(extensions) = &self.extensions {
            for key in extensions.keys() {
                if !key.starts_with("x-") {
                    return Err(BindingError::InvalidInput(format!(
                        "intake handoff extension '{key}' must start with x-"
                    )));
                }
            }
        }

        Ok(())
    }
}

/// Parse Formspec `authoredSignatures` and validate their response-pin fields.
///
/// The binding crate owns this Formspec-specific shape so WOS runtime and
/// provenance code do not need to duplicate Formspec JSON field names at every
/// boundary. Digest recomputation is a separate verifier concern; this parser
/// checks that the signature pins agree with the Response envelope identity.
pub fn parse_authored_signatures(
    response: &serde_json::Value,
) -> Result<Vec<FormspecAuthoredSignatureRef>, BindingError> {
    let signatures = authored_signature_refs(response)?;
    if signatures.is_empty() {
        return Ok(signatures);
    }
    let response_id = response
        .get("id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            BindingError::InvalidInput(
                "Formspec Response with authoredSignatures requires id".to_string(),
            )
        })?;
    let definition_url = response
        .get("definitionUrl")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let definition_version = response
        .get("definitionVersion")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();

    for signature in &signatures {
        validate_authored_signature_shape(signature)?;
        if signature.signed_payload.response_id != response_id {
            return Err(BindingError::InvalidInput(
                "authoredSignatures signedPayload.responseId must match Response id".to_string(),
            ));
        }
        if signature.signed_payload.definition_url != definition_url {
            return Err(BindingError::InvalidInput(
                "authoredSignatures signedPayload.definitionUrl must match Response definitionUrl"
                    .to_string(),
            ));
        }
        if signature.signed_payload.definition_version != definition_version {
            return Err(BindingError::InvalidInput(
                "authoredSignatures signedPayload.definitionVersion must match Response definitionVersion"
                    .to_string(),
            ));
        }
        let computed_digest = compute_formspec_signed_payload_digest(
            response,
            &signature.signed_payload.digest_algorithm,
        )?;
        if computed_digest != signature.signed_payload.digest {
            return Err(BindingError::InvalidInput(
                "authoredSignatures signedPayload.digest does not match signed Response payload"
                    .to_string(),
            ));
        }
    }
    Ok(signatures)
}

fn authored_signature_refs(
    response: &serde_json::Value,
) -> Result<Vec<FormspecAuthoredSignatureRef>, BindingError> {
    let Some(items) = response.get("authoredSignatures") else {
        return Ok(Vec::new());
    };
    serde_json::from_value(items.clone()).map_err(|error| {
        BindingError::InvalidInput(format!("invalid Formspec authoredSignatures: {error}"))
    })
}

fn parse_authored_signatures_for_evidence(
    response: &serde_json::Value,
) -> Result<Vec<FormspecAuthoredSignatureEvidence>, BindingError> {
    let signatures = authored_signature_refs(response)?;
    if signatures.is_empty() {
        return Ok(Vec::new());
    }
    let response_id = response
        .get("id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            BindingError::InvalidInput(
                "Formspec Response with authoredSignatures requires id".to_string(),
            )
        })?;
    let definition_url = response
        .get("definitionUrl")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let definition_version = response
        .get("definitionVersion")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();

    let mut out = Vec::with_capacity(signatures.len());
    for signature in signatures {
        validate_authored_signature_shape(&signature)?;
        let mut admission_failure = None;
        if signature.signed_payload.response_id != response_id {
            admission_failure = Some(evidence_divergence_failure(
                "signedPayload.responseId",
                response_id,
                &signature.signed_payload.response_id,
            ));
        }
        if admission_failure.is_none() && signature.signed_payload.definition_url != definition_url
        {
            admission_failure = Some(evidence_divergence_failure(
                "signedPayload.definitionUrl",
                definition_url,
                &signature.signed_payload.definition_url,
            ));
        }
        if admission_failure.is_none()
            && signature.signed_payload.definition_version != definition_version
        {
            admission_failure = Some(evidence_divergence_failure(
                "signedPayload.definitionVersion",
                definition_version,
                &signature.signed_payload.definition_version,
            ));
        }
        let computed_digest = compute_formspec_signed_payload_digest(
            response,
            &signature.signed_payload.digest_algorithm,
        )?;
        if admission_failure.is_none() && computed_digest != signature.signed_payload.digest {
            admission_failure = Some(evidence_divergence_failure(
                "signedPayload.digest",
                &computed_digest,
                &signature.signed_payload.digest,
            ));
        }
        out.push(FormspecAuthoredSignatureEvidence {
            signature,
            admission_failure,
        });
    }
    Ok(out)
}

fn validate_authored_signature_shape(
    signature: &FormspecAuthoredSignatureRef,
) -> Result<(), BindingError> {
    ensure_non_empty("authoredSignatures.signatureId", &signature.signature_id)?;
    ensure_non_empty(
        "authoredSignatures.signingIntent",
        &signature.signing_intent,
    )?;
    ensure_non_empty(
        "authoredSignatures.signedPayload.digest",
        &signature.signed_payload.digest,
    )?;
    if signature.signed_payload.canonicalization != CANONICALIZATION_PROFILE {
        return Err(BindingError::InvalidInput(format!(
            "authoredSignatures signedPayload.canonicalization must be {CANONICALIZATION_PROFILE}"
        )));
    }
    if !signature.consent_accepted {
        return Err(BindingError::InvalidInput(
            "authoredSignatures consentAccepted must be true".to_string(),
        ));
    }
    Ok(())
}

fn evidence_divergence_failure(
    field: &str,
    expected: &str,
    actual: &str,
) -> SignatureAdmissionFailure {
    SignatureAdmissionFailure {
        reason: SignatureAdmissionFailureReason::EvidenceDivergence,
        failure_context: Some(serde_json::Map::from_iter([
            (
                "field".to_string(),
                serde_json::Value::String(field.to_string()),
            ),
            (
                "expected".to_string(),
                serde_json::Value::String(expected.to_string()),
            ),
            (
                "actual".to_string(),
                serde_json::Value::String(actual.to_string()),
            ),
        ])),
    }
}

fn compute_formspec_signed_payload_digest(
    response: &serde_json::Value,
    algorithm: &str,
) -> Result<String, BindingError> {
    let digest_algorithm = DigestAlgorithm::from_str(algorithm).map_err(|_| {
        BindingError::InvalidInput(format!(
            "unsupported Formspec signedPayload.digestAlgorithm '{algorithm}'"
        ))
    })?;
    let signed_payload = build_signed_payload(response, digest_algorithm).map_err(|error| {
        BindingError::InvalidInput(format!("canonicalize Formspec signed payload: {error}"))
    })?;
    Ok(signed_payload.digest)
}

fn signature_method_admission_failure(
    signature_method: Option<&str>,
) -> Option<SignatureAdmissionFailure> {
    let method = signature_method?;
    if !method.starts_with("urn:formspec:sig-method:") || registered_signature_method(method) {
        return None;
    }

    Some(SignatureAdmissionFailure {
        reason: SignatureAdmissionFailureReason::MethodUnregistered,
        failure_context: Some(serde_json::Map::from_iter([
            (
                "signatureMethod".to_string(),
                serde_json::Value::String(method.to_string()),
            ),
            (
                "registryVersion".to_string(),
                serde_json::Value::String(FORMSPEC_SIGNATURE_METHOD_REGISTRY_VERSION.to_string()),
            ),
        ])),
    })
}

fn registered_signature_method(method: &str) -> bool {
    matches!(
        method,
        "urn:formspec:sig-method:ed25519-cose-sign1@1"
            | "urn:formspec:sig-method:ecdsa-p256-cose-sign1@1"
            | "urn:formspec:sig-method:rsa-pss-sha256-cose-sign1@1"
            | "urn:formspec:sig-method:ml-dsa-65-cose-sign1@1"
            | "urn:formspec:sig-method:slh-dsa-128s-cose-sign1@1"
    )
}

fn response_path<'a>(response: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    if let Some(data) = response.get("data")
        && let Some(value) = value_at_path(data, path)
    {
        return Some(value);
    }
    value_at_path(response, path)
}

fn value_at_path<'a>(root: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = root;
    for segment in path.split('.') {
        if segment.is_empty() {
            return None;
        }
        current = current.get(segment)?;
    }
    Some(current)
}

/// Parse and validate a Formspec intake handoff.
///
/// This validates the WOS boundary invariants that determine case ownership:
/// `workflowInitiated` handoffs attach to an existing case, while
/// `publicIntake` handoffs request case creation only after acceptance.
pub fn parse_intake_handoff(document: &serde_json::Value) -> Result<IntakeHandoff, BindingError> {
    let handoff: IntakeHandoff = serde_json::from_value(document.clone())
        .map_err(|error| BindingError::InvalidInput(format!("invalid intake handoff: {error}")))?;
    handoff.validate()?;
    Ok(handoff)
}

/// Create a WOS `caseCreated` provenance record from a validated intake handoff.
///
/// This stays in the Formspec seam because the evidence refs and data keys are
/// Formspec-specific even though the resulting provenance kind is WOS-native.
/// It is intended for host-side intake-acceptance paths and is called from the
/// Formspec intake finalizer after host policy chooses `CreateGovernedCase`.
///
/// # Errors
///
/// Returns [`BindingError::InvalidInput`] when the handoff violates its
/// schema-level mode invariants, or if `case_ref` is empty or not a case
/// TypeID.
pub fn case_created_provenance(
    handoff: &IntakeHandoff,
    case_ref: &str,
    actor_id: Option<&str>,
) -> Result<ProvenanceRecord, BindingError> {
    handoff.validate()?;
    ensure_non_empty("caseRef", case_ref)?;
    if !WorkflowProcess::is_case_id(case_ref) {
        return Err(BindingError::InvalidInput(
            "caseRef must be a canonical case ledger TypeID".to_string(),
        ));
    }

    let mut data = serde_json::Map::from_iter([
        (
            "caseRef".to_string(),
            serde_json::Value::String(case_ref.to_string()),
        ),
        (
            "caseLedgerId".to_string(),
            serde_json::Value::String(case_ref.to_string()),
        ),
        (
            "intakeHandoffRef".to_string(),
            serde_json::Value::String(handoff.handoff_id.clone()),
        ),
        (
            "formspecResponseRef".to_string(),
            serde_json::Value::String(handoff.response_ref.clone()),
        ),
        (
            "validationReportRef".to_string(),
            serde_json::Value::String(handoff.validation_report_ref.clone()),
        ),
        (
            "ledgerHeadRef".to_string(),
            serde_json::Value::String(handoff.ledger_head_ref.clone()),
        ),
        (
            "initiationMode".to_string(),
            serde_json::Value::String(handoff.initiation_mode.as_str().to_string()),
        ),
    ]);

    if let Some(subject_ref) = &handoff.subject_ref {
        data.insert(
            "subjectRef".to_string(),
            serde_json::Value::String(subject_ref.clone()),
        );
    }

    Ok(ProvenanceRecord {
        id: ProvenanceRecord::mint_id(),
        record_kind: ProvenanceKind::CaseCreated,
        timestamp: String::new(),
        actor_id: actor_id.map(String::from),
        from_state: None,
        to_state: None,
        event: Some(WOS_KERNEL_CASE_CREATED_EVENT.to_string()),
        data: Some(serde_json::Value::Object(data)),
        audit_layer: None,
        actor_type: None,
        lifecycle_state: None,
        definition_version: None,
        inputs: vec![
            handoff.handoff_id.clone(),
            handoff.response_ref.clone(),
            handoff.validation_report_ref.clone(),
            handoff.ledger_head_ref.clone(),
        ],
        outputs: vec![case_ref.to_string()],
        input_digest: None,
        output_digest: None,
        canonical_event_hash: None,
        transition_tags: Vec::new(),
        case_file_snapshot: None,
        outcome: None,
    })
}

fn ensure_non_empty(field: &str, value: &str) -> Result<(), BindingError> {
    if value.trim().is_empty() {
        return Err(BindingError::InvalidInput(format!(
            "{field} must not be empty"
        )));
    }
    Ok(())
}

fn is_valid_hash_string(value: &str) -> bool {
    let Some((algorithm, digest)) = value.split_once(':') else {
        return false;
    };
    !algorithm.is_empty()
        && !digest.is_empty()
        && algorithm
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | ':' | '+' | '-'))
}

/// Formspec processor abstraction used by the binding adapter.
pub trait FormspecProcessor {
    /// Validate a full Formspec response envelope.
    fn validate_envelope(
        &self,
        response: &serde_json::Value,
    ) -> Result<Vec<serde_json::Value>, BindingError>;

    /// Validate `response.data` against the pinned Definition.
    fn validate_definition(
        &self,
        definition_url: &str,
        definition_version: &str,
        data: &serde_json::Value,
    ) -> Result<Option<Vec<serde_json::Value>>, BindingError>;

    /// Compute prefill data for a task.
    fn compute_prefill(
        &self,
        mapping_ref: Option<&str>,
        case_state: &serde_json::Value,
    ) -> Result<Option<serde_json::Value>, BindingError>;

    /// Compute a case mutation from a completed response.
    fn map_response(
        &self,
        mapping_ref: &str,
        response: &serde_json::Value,
    ) -> Result<Option<CaseMutationBundle>, BindingError>;
}

/// Formspec-backed binding adapter.
#[derive(Debug, Clone)]
pub struct FormspecBinding<P> {
    processor: P,
}

impl<P> FormspecBinding<P> {
    /// Create a binding adapter from a Formspec processor.
    pub fn new(processor: P) -> Self {
        Self { processor }
    }
}

impl<P> FormspecBinding<P>
where
    P: FormspecProcessor,
{
    /// Re-validate a previously submitted response envelope against the current
    /// task pin (definition URL + version).
    ///
    /// This method performs the same envelope structure checks, pin equality
    /// assertion, and definition validation as `validate_submission`. It does
    /// **not** trust any stored `pin_match` record — pin equality is recomputed
    /// fresh from `task.definition_url` and `task.definition_version` every
    /// time this is called.  Use this on replay, audit, and review paths where
    /// an already-stored response must be re-examined.
    pub fn revalidate_submission(
        &self,
        task: &ActiveTask,
        previously_submitted_response: &serde_json::Value,
    ) -> Result<SubmissionValidation, BindingError> {
        self.run_validation(task, previously_submitted_response)
    }

    /// Shared validation logic used by both `validate_submission` and
    /// `revalidate_submission`.  Keeps pin enforcement in one place so both
    /// paths are guaranteed to behave identically.
    fn run_validation(
        &self,
        task: &ActiveTask,
        response: &serde_json::Value,
    ) -> Result<SubmissionValidation, BindingError> {
        let mut errors = validate_required_envelope_fields(response)?;
        errors.extend(self.processor.validate_envelope(response)?);

        let response_definition_url = response
            .get("definitionUrl")
            .and_then(serde_json::Value::as_str);
        let response_definition_version = response
            .get("definitionVersion")
            .and_then(serde_json::Value::as_str);
        let pin_match = response_definition_url == task.definition_url.as_deref()
            && response_definition_version == task.definition_version.as_deref();

        let mut validation_results = None;
        let definition_valid = if errors.is_empty() && pin_match {
            let data = response
                .get("data")
                .ok_or_else(|| BindingError::InvalidInput("response.data missing".to_string()))?;
            validation_results = self.processor.validate_definition(
                task.definition_url.as_deref().unwrap_or_default(),
                task.definition_version.as_deref().unwrap_or_default(),
                data,
            )?;
            validation_results
                .as_ref()
                .is_none_or(std::vec::Vec::is_empty)
        } else {
            // When pin_match is false, we deliberately skip definition validation
            // — the stored definition at the submitted pin may differ from the current pin,
            // so validating against the current pin would produce misleading diagnostics.
            // definition_valid is marked false to signal "not validated at this pin" rather
            // than "validated and failed"; validation_results stays None.
            false
        };

        if !pin_match {
            errors.push(serde_json::json!({
                "code": "pinMismatch",
                "message": "response pin does not match task pin",
            }));
        }

        Ok(SubmissionValidation {
            validation_outcome: ValidationOutcome {
                envelope_valid: errors
                    .iter()
                    .all(|error| error.get("code") != Some(&serde_json::json!("invalidEnvelope"))),
                pin_match,
                definition_valid,
                errors,
                validation_results,
            },
        })
    }
}

impl<P> ContractBindingAdapter for FormspecBinding<P>
where
    P: FormspecProcessor + Send + Sync,
{
    fn binding(&self) -> &'static str {
        "formspec"
    }

    fn prepare_task(
        &self,
        task: &ActiveTask,
        case_state: &serde_json::Value,
    ) -> Result<PreparedTask, BindingError> {
        Ok(PreparedTask {
            prefill_data: self
                .processor
                .compute_prefill(task.prefill_mapping_ref.as_deref(), case_state)?,
        })
    }

    fn validate_submission(
        &self,
        task: &ActiveTask,
        response: &serde_json::Value,
    ) -> Result<SubmissionValidation, BindingError> {
        self.run_validation(task, response)
    }

    fn compute_case_mutation(
        &self,
        task: &ActiveTask,
        response: &serde_json::Value,
    ) -> Result<Option<CaseMutationBundle>, BindingError> {
        let Some(mapping_ref) = task.response_mapping_ref.as_deref() else {
            return Ok(None);
        };
        self.processor.map_response(mapping_ref, response)
    }

    fn signature_evidence(
        &self,
        _task: &ActiveTask,
        response: &serde_json::Value,
    ) -> Result<Option<Vec<SignatureEvidence>>, BindingError> {
        let signatures = parse_authored_signatures_for_evidence(response)?;
        if signatures.is_empty() {
            return Ok(None);
        }
        let source_response_ref = response
            .get("sourceResponseRef")
            .or_else(|| response.get("formspecResponseRef"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);
        let response_signer_id =
            response_path(response, "signerId").and_then(serde_json::Value::as_str);
        let signer_authority = response_path(response, "signerAuthority")
            .or_else(|| response_path(response, "signature.signerAuthority"))
            .cloned();

        let mut evidence = Vec::with_capacity(signatures.len());
        for signature_evidence in signatures {
            let signature = signature_evidence.signature;
            let admission_failure = signature_evidence.admission_failure.or_else(|| {
                signature_method_admission_failure(signature.signature_method.as_deref())
            });
            if let (Some(response_signer_id), Some(signature_signer_id)) =
                (response_signer_id, signature.signer_id.as_deref())
                && response_signer_id != signature_signer_id
            {
                return Err(BindingError::InvalidInput(
                    "authoredSignatures signerId must match response signerId".to_string(),
                ));
            }
            evidence.push(SignatureEvidence {
                source_system: "formspec".to_string(),
                source_signature_id: signature.signature_id,
                source_response_ref: source_response_ref.clone(),
                document_id: signature.document_id,
                signer_id: signature
                    .signer_id
                    .or_else(|| response_signer_id.map(str::to_string)),
                signing_intent: signature.signing_intent,
                signature_method: signature.signature_method,
                signed_payload_digest: signature.signed_payload.digest,
                signed_payload_digest_algorithm: signature.signed_payload.digest_algorithm,
                signed_at: signature.signed_at,
                document_hash: signature.document_hash,
                document_hash_algorithm: signature.document_hash_algorithm,
                signature_provider: Some(signature.signature_provider),
                ceremony_id: Some(signature.ceremony_id),
                identity_binding: signature.identity_binding,
                signer_authority: signer_authority.clone(),
                // The reference Formspec binding parses pins, consent, signing
                // intent, and the signed-payload digest, but it does not yet
                // execute the cryptographic primitive over `signatureValue` /
                // `signatureMethod`. That work ships with
                // `FORMSPEC-SIGN-HELPER-001`. Until then, the binding reports
                // `DeferredPendingHelper` so downstream WOS provenance honestly
                // records that the primitive has not run.
                primitive_verification: SignaturePrimitiveStatus::DeferredPendingHelper {
                    reason: FORMSPEC_SIGNING_HELPER_PENDING_REASON.to_string(),
                },
                verification_receipt: signature.verification_receipt,
                admission_failure,
            });
        }
        Ok(Some(evidence))
    }
}

impl<P> IntakeAcceptanceAdapter for FormspecBinding<P>
where
    P: FormspecProcessor + Send + Sync,
{
    fn binding(&self) -> &'static str {
        "formspec"
    }

    fn interpret_intake_handoff(
        &self,
        request: &IntakeAcceptanceRequest,
    ) -> Result<IntakeInterpretation, BindingError> {
        let handoff = parse_intake_handoff(&request.document)?;
        let case_intent = match handoff.case_intent()? {
            IntakeHandoffCaseIntent::AttachToExistingCase { case_ref } => {
                IntakeCaseIntent::AttachToExistingCase { case_ref }
            }
            IntakeHandoffCaseIntent::CreateCaseAfterAcceptance => {
                IntakeCaseIntent::RequestGovernedCaseCreation
            }
        };
        Ok(IntakeInterpretation {
            intake_id: handoff.handoff_id,
            case_intent,
        })
    }

    /// Emit binding-owned provenance and enforce handoff consistency.
    ///
    /// For **`workflowInitiated`** attach acceptance, the accepted disposition's
    /// attach `case_ref` MUST equal the handoff's `caseRef` string (see Formspec
    /// Core §2.1.6.1 and `schemas/intake-handoff.schema.json`). Hosts that
    /// canonicalize governed-case ids for durable storage MUST pass an outcome
    /// whose attach ref still matches that handoff string when calling this
    /// method (the WOS reference runtime supplies such an outcome via
    /// `outcome_for_binding_finalize` in `wos-runtime`).
    ///
    /// For accepted **`CreateGovernedCase`**, emits `CaseCreated` provenance
    /// using the canonical `case_ref` from the outcome.
    fn finalize_intake_acceptance(
        &self,
        request: &IntakeAcceptanceRequest,
        outcome: &IntakeAcceptanceOutcome,
    ) -> Result<Vec<ProvenanceRecord>, BindingError> {
        let handoff = parse_intake_handoff(&request.document)?;
        match outcome {
            IntakeAcceptanceOutcome::Accepted { case_disposition } => match case_disposition {
                IntakeCaseDisposition::AttachToExistingCase { case_ref } => {
                    if let IntakeHandoffCaseIntent::AttachToExistingCase {
                        case_ref: expected_case_ref,
                    } = handoff.case_intent()?
                    {
                        if case_ref != &expected_case_ref {
                            return Err(BindingError::InvalidInput(
                                "accepted caseRef must match workflowInitiated intake handoff"
                                    .to_string(),
                            ));
                        }
                    }
                    Ok(Vec::new())
                }
                IntakeCaseDisposition::CreateGovernedCase { case_ref, .. } => {
                    Ok(vec![case_created_provenance(
                        &handoff,
                        case_ref,
                        request.actor_id.as_deref(),
                    )?])
                }
            },
            IntakeAcceptanceOutcome::Rejected { .. } | IntakeAcceptanceOutcome::Deferred { .. } => {
                Ok(Vec::new())
            }
        }
    }
}

fn validate_required_envelope_fields(
    response: &serde_json::Value,
) -> Result<Vec<serde_json::Value>, BindingError> {
    let Some(object) = response.as_object() else {
        return Ok(vec![serde_json::json!({
            "code": "invalidEnvelope",
            "message": "response must be a JSON object",
        })]);
    };

    let mut errors = Vec::new();
    for required in ["status", "definitionUrl", "definitionVersion", "data"] {
        if !object.contains_key(required) {
            errors.push(serde_json::json!({
                "code": "invalidEnvelope",
                "message": format!("missing required property '{required}'"),
            }));
        }
    }

    Ok(errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_CASE_LEDGER_ID: &str = "sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsc";

    #[derive(Debug, Clone, Default)]
    struct StubProcessor;

    impl FormspecProcessor for StubProcessor {
        fn validate_envelope(
            &self,
            response: &serde_json::Value,
        ) -> Result<Vec<serde_json::Value>, BindingError> {
            if response
                .get("meta")
                .and_then(|meta| meta.get("rejectEnvelope"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
                return Ok(vec![serde_json::json!({
                    "code": "invalidEnvelope",
                    "message": "processor rejected envelope",
                })]);
            }
            Ok(Vec::new())
        }

        fn validate_definition(
            &self,
            _definition_url: &str,
            _definition_version: &str,
            data: &serde_json::Value,
        ) -> Result<Option<Vec<serde_json::Value>>, BindingError> {
            let valid = data
                .get("approved")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            Ok(if valid {
                None
            } else {
                Some(vec![serde_json::json!({
                    "code": "definitionInvalid",
                    "message": "approved must be true",
                })])
            })
        }

        fn compute_prefill(
            &self,
            mapping_ref: Option<&str>,
            case_state: &serde_json::Value,
        ) -> Result<Option<serde_json::Value>, BindingError> {
            Ok(mapping_ref.map(|mapping_ref| {
                serde_json::json!({
                    "mappingRef": mapping_ref,
                    "caseState": case_state,
                })
            }))
        }

        fn map_response(
            &self,
            mapping_ref: &str,
            response: &serde_json::Value,
        ) -> Result<Option<CaseMutationBundle>, BindingError> {
            let mut field_updates = serde_json::Map::new();
            field_updates.insert(
                "mappingRef".to_string(),
                serde_json::Value::String(mapping_ref.to_string()),
            );
            field_updates.insert("decision".to_string(), response["data"]["approved"].clone());
            Ok(Some(CaseMutationBundle { field_updates }))
        }
    }

    fn formspec_task() -> ActiveTask {
        ActiveTask {
            task_id: "task-1".to_string(),
            task_ref: "review".to_string(),
            status: wos_core::instance::ActiveTaskStatus::Assigned,
            assigned_actor: Some("reviewer".to_string()),
            contract_ref: Some("reviewForm".to_string()),
            binding: Some("formspec".to_string()),
            definition_url: Some("urn:formspec:review".to_string()),
            definition_version: Some("1.0.0".to_string()),
            prefill_mapping_ref: Some("urn:mapping:prefill".to_string()),
            response_mapping_ref: Some("urn:mapping:response".to_string()),
            deadline: None,
            impact_level: None,
            context: None,
            last_validation_outcome: None,
            created_at: "2024-03-09T00:00:00Z".to_string(),
            updated_at: "2024-03-09T00:00:00Z".to_string(),
            extensions: Default::default(),
        }
    }

    #[test]
    fn prepare_task_returns_prefill_only() {
        let adapter = FormspecBinding::new(StubProcessor);
        let prepared = adapter
            .prepare_task(&formspec_task(), &serde_json::json!({ "seed": 1 }))
            .unwrap();
        assert_eq!(
            prepared.prefill_data,
            Some(serde_json::json!({
                "mappingRef": "urn:mapping:prefill",
                "caseState": { "seed": 1 }
            }))
        );
    }

    #[test]
    fn registers_as_formspec_binding() {
        let mut registry = wos_runtime::binding::BindingRegistry::new();
        registry.register(FormspecBinding::new(StubProcessor));

        let adapter = registry
            .get("formspec")
            .expect("formspec adapter should register");
        assert_eq!(adapter.binding(), "formspec");
    }

    #[test]
    fn validate_submission_reports_pin_mismatch() {
        let adapter = FormspecBinding::new(StubProcessor);
        let validation = adapter
            .validate_submission(
                &formspec_task(),
                &serde_json::json!({
                    "status": "completed",
                    "definitionUrl": "urn:formspec:other",
                    "definitionVersion": "1.0.0",
                    "data": { "approved": true }
                }),
            )
            .unwrap();

        assert!(!validation.validation_outcome.pin_match);
        assert!(!validation.validation_outcome.definition_valid);
    }

    #[test]
    fn validate_submission_returns_definition_results() {
        let adapter = FormspecBinding::new(StubProcessor);
        let validation = adapter
            .validate_submission(
                &formspec_task(),
                &serde_json::json!({
                    "status": "completed",
                    "definitionUrl": "urn:formspec:review",
                    "definitionVersion": "1.0.0",
                    "data": { "approved": false }
                }),
            )
            .unwrap();

        assert!(validation.validation_outcome.envelope_valid);
        assert!(validation.validation_outcome.pin_match);
        assert!(!validation.validation_outcome.definition_valid);
        assert_eq!(
            validation.validation_outcome.validation_results,
            Some(vec![serde_json::json!({
                "code": "definitionInvalid",
                "message": "approved must be true",
            })])
        );
    }

    #[test]
    fn compute_case_mutation_is_side_effect_free() {
        let adapter = FormspecBinding::new(StubProcessor);
        let first = adapter
            .compute_case_mutation(
                &formspec_task(),
                &serde_json::json!({
                    "status": "completed",
                    "definitionUrl": "urn:formspec:review",
                    "definitionVersion": "1.0.0",
                    "data": { "approved": true }
                }),
            )
            .unwrap()
            .unwrap();
        let second = adapter
            .compute_case_mutation(
                &formspec_task(),
                &serde_json::json!({
                    "status": "completed",
                    "definitionUrl": "urn:formspec:review",
                    "definitionVersion": "1.0.0",
                    "data": { "approved": true }
                }),
            )
            .unwrap()
            .unwrap();

        assert_eq!(first.field_updates, second.field_updates);
    }

    fn signed_response() -> serde_json::Value {
        let mut response = serde_json::json!({
            "id": "resp-2026-0001",
            "status": "completed",
            "definitionUrl": "urn:formspec:review",
            "definitionVersion": "1.0.0",
            "data": { "approved": true },
            "authoredSignatures": [
                {
                    "signatureId": "sig-2026-0001",
                    "documentId": "benefitsApplication",
                    "signingIntent": "urn:wos:signing-intent:applicant-signature",
                    "signatureValue": "urn:test:signature:1",
                    "signatureMethod": "provider-managed",
                    "signerId": "applicant",
                    "signerName": "Ada Lovelace",
                    "signedAt": "2026-04-22T12:00:00Z",
                    "consentAccepted": true,
                    "consentTextRef": "urn:test:consent:v1",
                    "consentVersion": "1.0.0",
                    "affirmationText": "I certify this response.",
                    "signedPayload": {
                        "canonicalization": "formspec-response-signing-v1",
                        "digestAlgorithm": "sha-256",
                        "digest": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                        "responseId": "resp-2026-0001",
                        "definitionUrl": "urn:formspec:review",
                        "definitionVersion": "1.0.0"
                    },
                    "documentHash": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                    "documentHashAlgorithm": "sha-256",
                    "signatureProvider": "formspec",
                    "ceremonyId": "ceremony-2026-0001"
                }
            ]
        });
        let digest = compute_formspec_signed_payload_digest(&response, "sha-256").unwrap();
        response["authoredSignatures"][0]["signedPayload"]["digest"] =
            serde_json::Value::String(digest);
        response
    }

    fn legacy_no_nul_signed_payload_digest(response: &serde_json::Value) -> String {
        let canonical = integrity_canonical::canonicalize_response(response).unwrap();
        let canonical_bytes = integrity_canonical::canonical_json_bytes(&canonical).unwrap();
        let mut payload = Vec::with_capacity(
            integrity_canonical::DOMAIN_SEPARATION.len() + canonical_bytes.len(),
        );
        payload.extend_from_slice(integrity_canonical::DOMAIN_SEPARATION.as_bytes());
        payload.extend_from_slice(&canonical_bytes);
        integrity_canonical::compute_digest(&payload, integrity_canonical::DigestAlgorithm::Sha256)
    }

    #[test]
    fn parse_authored_signatures_accepts_new_formspec_shape() {
        let response = signed_response();
        let expected_signed_payload_digest =
            response["authoredSignatures"][0]["signedPayload"]["digest"]
                .as_str()
                .expect("signed-payload digest is present")
                .to_string();
        let signatures = parse_authored_signatures(&response).unwrap();

        assert_eq!(signatures.len(), 1);
        assert_eq!(signatures[0].signature_id, "sig-2026-0001");
        assert_eq!(
            signatures[0].signature_method.as_deref(),
            Some("provider-managed")
        );
        assert_eq!(
            signatures[0].signed_payload.digest,
            expected_signed_payload_digest
        );
        assert_eq!(
            signatures[0].document_hash,
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        );
    }

    #[test]
    fn parse_authored_signatures_rejects_legacy_no_nul_digest() {
        let mut response = signed_response();
        let legacy_digest = legacy_no_nul_signed_payload_digest(&response);
        response["authoredSignatures"][0]["signedPayload"]["digest"] =
            serde_json::Value::String(legacy_digest);

        let error = parse_authored_signatures(&response).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("signedPayload.digest does not match"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn signature_evidence_emits_deferred_pending_helper_status() {
        // The reference Formspec binding does not yet run the cryptographic
        // primitive over signatureValue / signatureMethod (FORMSPEC-SIGN-
        // HELPER-001 pending). It MUST therefore emit
        // SignaturePrimitiveStatus::DeferredPendingHelper so downstream WOS
        // provenance records the verification gap honestly instead of
        // implying a verified signature.
        let adapter = FormspecBinding::new(StubProcessor);
        let response = signed_response();
        let evidence = adapter
            .signature_evidence(&formspec_task(), &response)
            .expect("signature evidence parses")
            .expect("signature evidence is present");

        assert_eq!(evidence.len(), 1);
        assert_eq!(
            evidence[0].signature_method.as_deref(),
            Some("provider-managed")
        );
        assert_eq!(
            evidence[0].primitive_verification,
            SignaturePrimitiveStatus::DeferredPendingHelper {
                reason: FORMSPEC_SIGNING_HELPER_PENDING_REASON.to_string(),
            },
            "Formspec binding must emit DeferredPendingHelper while \
             FORMSPEC-SIGN-HELPER-001 is unshipped"
        );
        assert!(
            evidence[0].admission_failure.is_none(),
            "legacy provider-managed method remains deferred until fixture recast"
        );
        assert!(evidence[0].verification_receipt.is_none());
    }

    #[test]
    fn signature_evidence_carries_verification_receipt_bytes() {
        let adapter = FormspecBinding::new(StubProcessor);
        let mut response = signed_response();
        response["authoredSignatures"][0]["verificationReceipt"] =
            serde_json::json!("0oRWoQExiQEFQnNpZ25lZA==");

        let evidence = adapter
            .signature_evidence(&formspec_task(), &response)
            .expect("signature evidence parses")
            .expect("signature evidence is present");

        assert_eq!(
            evidence[0].verification_receipt.as_deref(),
            Some("0oRWoQExiQEFQnNpZ25lZA==")
        );
    }

    #[test]
    fn signature_evidence_reports_unregistered_registry_method() {
        let adapter = FormspecBinding::new(StubProcessor);
        let mut response = signed_response();
        response["authoredSignatures"][0]["signatureMethod"] =
            serde_json::json!("urn:formspec:sig-method:unknown@1");

        let evidence = adapter
            .signature_evidence(&formspec_task(), &response)
            .expect("signature evidence parses")
            .expect("signature evidence is present");
        assert_eq!(
            evidence[0].signature_method.as_deref(),
            Some("urn:formspec:sig-method:unknown@1")
        );
        let admission_failure = evidence[0]
            .admission_failure
            .as_ref()
            .expect("unknown registry method must produce admission failure");

        assert_eq!(
            admission_failure.reason,
            SignatureAdmissionFailureReason::MethodUnregistered
        );
        let context = admission_failure
            .failure_context
            .as_ref()
            .expect("method_unregistered should carry failure context");
        assert_eq!(
            context
                .get("signatureMethod")
                .and_then(serde_json::Value::as_str),
            Some("urn:formspec:sig-method:unknown@1")
        );
        assert_eq!(
            context
                .get("registryVersion")
                .and_then(serde_json::Value::as_str),
            Some(FORMSPEC_SIGNATURE_METHOD_REGISTRY_VERSION)
        );
    }

    #[test]
    fn signature_evidence_reports_signed_payload_digest_divergence() {
        let adapter = FormspecBinding::new(StubProcessor);
        let mut response = signed_response();
        response["authoredSignatures"][0]["signedPayload"]["digest"] =
            serde_json::json!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");

        let evidence = adapter
            .signature_evidence(&formspec_task(), &response)
            .expect("signature evidence parses")
            .expect("signature evidence is present");
        let admission_failure = evidence[0]
            .admission_failure
            .as_ref()
            .expect("digest divergence should produce admission failure");
        assert_eq!(
            admission_failure.reason,
            SignatureAdmissionFailureReason::EvidenceDivergence
        );
        let context = admission_failure
            .failure_context
            .as_ref()
            .expect("evidence divergence carries failure context");
        assert_eq!(
            context.get("field").and_then(serde_json::Value::as_str),
            Some("signedPayload.digest")
        );
        assert_eq!(
            context.get("actual").and_then(serde_json::Value::as_str),
            Some("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")
        );
    }

    #[test]
    fn signature_evidence_reports_signed_payload_response_pin_divergence() {
        let adapter = FormspecBinding::new(StubProcessor);
        let mut response = signed_response();
        response["authoredSignatures"][0]["signedPayload"]["responseId"] =
            serde_json::json!("resp-stale");

        let evidence = adapter
            .signature_evidence(&formspec_task(), &response)
            .expect("signature evidence parses")
            .expect("signature evidence is present");
        let admission_failure = evidence[0]
            .admission_failure
            .as_ref()
            .expect("response pin divergence should produce admission failure");
        assert_eq!(
            admission_failure.reason,
            SignatureAdmissionFailureReason::EvidenceDivergence
        );
        let context = admission_failure
            .failure_context
            .as_ref()
            .expect("evidence divergence carries failure context");
        assert_eq!(
            context.get("field").and_then(serde_json::Value::as_str),
            Some("signedPayload.responseId")
        );
        assert_eq!(
            context.get("expected").and_then(serde_json::Value::as_str),
            Some("resp-2026-0001")
        );
        assert_eq!(
            context.get("actual").and_then(serde_json::Value::as_str),
            Some("resp-stale")
        );
    }

    #[test]
    fn parse_authored_signatures_rejects_response_pin_mismatch() {
        let mut response = signed_response();
        response["authoredSignatures"][0]["signedPayload"]["responseId"] =
            serde_json::json!("resp-stale");

        let error = parse_authored_signatures(&response).unwrap_err();
        assert!(
            error.to_string().contains("signedPayload.responseId"),
            "unexpected error: {error}"
        );
    }

    fn public_intake_handoff() -> serde_json::Value {
        serde_json::json!({
            "$formspecIntakeHandoff": "1.0",
            "handoffId": "handoff-public-2026-0001",
            "initiationMode": "publicIntake",
            "definitionRef": {
                "url": "https://example.gov/forms/benefits-intake",
                "version": "1.0.0"
            },
            "responseRef": "urn:formspec:response:resp-2026-0001",
            "responseHash": "sha256:0123456789abcdef",
            "validationReportRef": "urn:formspec:validation-report:vr-2026-0001",
            "intakeSessionId": "session-2026-0001",
            "ledgerHeadRef": "urn:formspec:respondent-ledger-event:evt-2026-0003",
            "occurredAt": "2026-04-22T17:15:00Z"
        })
    }

    #[test]
    fn public_intake_handoff_requests_case_creation_after_acceptance() {
        let handoff = parse_intake_handoff(&public_intake_handoff()).unwrap();

        assert_eq!(
            handoff.case_intent().unwrap(),
            IntakeHandoffCaseIntent::CreateCaseAfterAcceptance
        );
    }

    #[test]
    fn workflow_initiated_handoff_attaches_to_existing_case() {
        let mut doc = public_intake_handoff();
        let object = doc.as_object_mut().unwrap();
        object.insert(
            "initiationMode".to_string(),
            serde_json::json!("workflowInitiated"),
        );
        object.insert(
            "caseRef".to_string(),
            serde_json::json!("urn:wos:case:case-2026-0042"),
        );

        let handoff = parse_intake_handoff(&doc).unwrap();

        assert_eq!(
            handoff.case_intent().unwrap(),
            IntakeHandoffCaseIntent::AttachToExistingCase {
                case_ref: "urn:wos:case:case-2026-0042".to_string()
            }
        );
    }

    #[test]
    fn public_intake_handoff_rejects_existing_case_ref() {
        let mut doc = public_intake_handoff();
        doc.as_object_mut().unwrap().insert(
            "caseRef".to_string(),
            serde_json::json!("urn:wos:case:case-2026-0042"),
        );

        let err = parse_intake_handoff(&doc).unwrap_err();

        assert!(err.to_string().contains("publicIntake"));
        assert!(err.to_string().contains("caseRef"));
    }

    #[test]
    fn workflow_initiated_handoff_requires_case_ref() {
        let mut doc = public_intake_handoff();
        doc.as_object_mut().unwrap().insert(
            "initiationMode".to_string(),
            serde_json::json!("workflowInitiated"),
        );

        let err = parse_intake_handoff(&doc).unwrap_err();

        assert!(err.to_string().contains("workflowInitiated"));
        assert!(err.to_string().contains("caseRef"));
    }

    #[test]
    fn intake_interpretation_attaches_workflow_initiated_handoff() {
        let adapter = FormspecBinding::new(StubProcessor);
        let mut doc = public_intake_handoff();
        let object = doc.as_object_mut().unwrap();
        object.insert(
            "initiationMode".to_string(),
            serde_json::json!("workflowInitiated"),
        );
        object.insert(
            "caseRef".to_string(),
            serde_json::json!("urn:wos:case:case-2026-0042"),
        );

        let result = adapter
            .interpret_intake_handoff(&IntakeAcceptanceRequest {
                document: doc,
                actor_id: Some("urn:iam:actor:intake-service".to_string()),
                governed_case_ref: None,
                governed_case_definition: None,
                initial_case_state: None,
            })
            .unwrap();

        assert_eq!(result.intake_id, "handoff-public-2026-0001".to_string());
        assert_eq!(
            result.case_intent,
            IntakeCaseIntent::AttachToExistingCase {
                case_ref: "urn:wos:case:case-2026-0042".to_string()
            }
        );
    }

    #[test]
    fn public_intake_interpretation_requests_case_creation() {
        let adapter = FormspecBinding::new(StubProcessor);

        let result = adapter
            .interpret_intake_handoff(&IntakeAcceptanceRequest {
                document: public_intake_handoff(),
                actor_id: Some("urn:iam:actor:intake-service".to_string()),
                governed_case_ref: None,
                governed_case_definition: None,
                initial_case_state: None,
            })
            .unwrap();

        assert_eq!(result.intake_id, "handoff-public-2026-0001".to_string());
        assert_eq!(
            result.case_intent,
            IntakeCaseIntent::RequestGovernedCaseCreation
        );
    }

    #[test]
    fn finalizing_public_intake_acceptance_emits_case_created_provenance() {
        let adapter = FormspecBinding::new(StubProcessor);
        let mut doc = public_intake_handoff();
        doc.as_object_mut().unwrap().insert(
            "handoffId".to_string(),
            serde_json::json!("urn:formspec:intake-handoff:handoff-public-2026-0001"),
        );

        let provenance = adapter
            .finalize_intake_acceptance(
                &IntakeAcceptanceRequest {
                    document: doc,
                    actor_id: Some("urn:iam:actor:intake-service".to_string()),
                    governed_case_ref: None,
                    governed_case_definition: None,
                    initial_case_state: None,
                },
                &IntakeAcceptanceOutcome::Accepted {
                    case_disposition: IntakeCaseDisposition::CreateGovernedCase {
                        case_ref: TEST_CASE_LEDGER_ID.to_string(),
                        definition: wos_runtime::IntakeCaseDefinition {
                            definition_url: "urn:test:intake".to_string(),
                            definition_version: "1.0.0".to_string(),
                        },
                        initial_case_state: None,
                    },
                },
            )
            .unwrap();

        assert_eq!(provenance.len(), 1);
        assert_eq!(provenance[0].record_kind, ProvenanceKind::CaseCreated);
    }

    #[test]
    fn finalizing_workflow_acceptance_rejects_case_ref_mismatch() {
        let adapter = FormspecBinding::new(StubProcessor);
        let mut doc = public_intake_handoff();
        let object = doc.as_object_mut().unwrap();
        object.insert(
            "initiationMode".to_string(),
            serde_json::json!("workflowInitiated"),
        );
        object.insert(
            "caseRef".to_string(),
            serde_json::json!("urn:wos:case:case-2026-0042"),
        );

        let err = adapter
            .finalize_intake_acceptance(
                &IntakeAcceptanceRequest {
                    document: doc,
                    actor_id: Some("urn:iam:actor:intake-service".to_string()),
                    governed_case_ref: None,
                    governed_case_definition: None,
                    initial_case_state: None,
                },
                &IntakeAcceptanceOutcome::Accepted {
                    case_disposition: IntakeCaseDisposition::AttachToExistingCase {
                        case_ref: "urn:wos:case:other".to_string(),
                    },
                },
            )
            .unwrap_err();

        assert!(err.to_string().contains("accepted caseRef must match"));
    }

    #[test]
    fn intake_handoff_rejects_hashes_that_fail_schema_pattern() {
        let mut doc = public_intake_handoff();
        doc.as_object_mut().unwrap().insert(
            "responseHash".to_string(),
            serde_json::json!("sha 256:0123456789abcdef"),
        );

        let err = parse_intake_handoff(&doc).unwrap_err();

        assert!(
            err.to_string()
                .contains("responseHash must match the Formspec HashString pattern")
        );
    }

    #[test]
    fn case_created_provenance_serializes_intake_handoff_evidence() {
        let mut doc = public_intake_handoff();
        doc.as_object_mut().unwrap().insert(
            "subjectRef".to_string(),
            serde_json::json!("urn:party:person:applicant-456"),
        );
        doc.as_object_mut().unwrap().insert(
            "handoffId".to_string(),
            serde_json::json!("urn:formspec:intake-handoff:handoff-public-2026-0001"),
        );
        let handoff = parse_intake_handoff(&doc).unwrap();

        let record = case_created_provenance(
            &handoff,
            TEST_CASE_LEDGER_ID,
            Some("urn:iam:actor:intake-service"),
        )
        .unwrap();
        let json = serde_json::to_value(&record).expect("serialize");

        assert_eq!(json["recordKind"], "caseCreated");
        assert_eq!(json["event"], WOS_KERNEL_CASE_CREATED_EVENT);
        assert_eq!(json["actorId"], "urn:iam:actor:intake-service");
        assert_eq!(json["data"]["caseRef"], TEST_CASE_LEDGER_ID);
        assert_eq!(json["data"]["caseLedgerId"], TEST_CASE_LEDGER_ID);
        assert_eq!(
            json["data"]["intakeHandoffRef"],
            "urn:formspec:intake-handoff:handoff-public-2026-0001"
        );
        assert_eq!(json["data"]["initiationMode"], "publicIntake");
        assert_eq!(
            json["inputs"][0],
            "urn:formspec:intake-handoff:handoff-public-2026-0001"
        );
        assert_eq!(json["outputs"][0], TEST_CASE_LEDGER_ID);
    }

    #[test]
    fn case_created_provenance_rejects_non_typeid_case_ref() {
        let handoff = parse_intake_handoff(&public_intake_handoff()).unwrap();

        let err = case_created_provenance(
            &handoff,
            "urn:wos:case:case-2026-0042",
            Some("urn:iam:actor:intake-service"),
        )
        .unwrap_err();

        assert!(err.to_string().contains("case ledger TypeID"));
    }

    #[test]
    fn intake_handoff_rejects_unknown_fields() {
        let mut doc = public_intake_handoff();
        doc.as_object_mut()
            .unwrap()
            .insert("caseCreated".to_string(), serde_json::json!(true));

        let err = parse_intake_handoff(&doc).unwrap_err();

        assert!(err.to_string().contains("unknown field"));
    }
}
