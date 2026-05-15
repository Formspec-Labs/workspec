// Rust guideline compliant 2026-02-21

//! Contract binding adapters for runtime task flows.
//!
//! `wos-runtime` owns WOS orchestration. Binding adapters own
//! binding-specific validation and projection semantics.
//! This seam currently covers task presentation, task submission validation,
//! and task-to-case mutation. Host-side intake-handoff acceptance is a
//! separate boundary and does not yet have a dedicated runtime hook here.

use std::collections::HashMap;
use std::sync::Arc;

use wos_core::instance::{ActiveTask, ValidationOutcome};

/// Prepared binding-specific task context.
#[derive(Debug, Clone, Default)]
pub struct PreparedTask {
    /// Prefill payload for the task presenter.
    pub prefill_data: Option<serde_json::Value>,
}

/// Binding-specific submission validation result.
#[derive(Debug, Clone)]
pub struct SubmissionValidation {
    /// WOS wrapper around binding validation outcomes.
    pub validation_outcome: ValidationOutcome,
}

/// Proposed case mutation from a completed task response.
#[derive(Debug, Clone, Default)]
pub struct CaseMutationBundle {
    /// Top-level case-state field updates.
    pub field_updates: serde_json::Map<String, serde_json::Value>,
}

/// Cryptographic primitive-verification status reported by the binding for
/// each authored signature.
///
/// `Verified` requires that the signature primitive (canonical-digest +
/// signature-suite check over the binding's signature value/method, e.g.
/// Formspec `signatureValue`/`signatureMethod`) actually executed and passed.
/// Until the Formspec signing helper (`FORMSPEC-SIGN-HELPER-001`) ships, the
/// reference Formspec binding emits
/// [`SignaturePrimitiveStatus::DeferredPendingHelper`] because pin/consent/
/// digest pre-checks have run but the primitive itself has not. `Failed`
/// indicates the primitive was attempted and rejected; WOS admission MUST NOT
/// emit a `SignatureAffirmation` whose primitive status is `Failed`.
///
/// JSON encoding uses `{ "status": "verified" | "deferredPendingHelper" |
/// "failed", "reason": "..." }` (the `reason` field is required for the latter
/// two and absent for `verified`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum SignaturePrimitiveStatus {
    /// The cryptographic signature primitive ran and passed.
    Verified,

    /// The primitive could not run because the signing helper is not
    /// available; pin, consent, and digest pre-checks succeeded.
    DeferredPendingHelper {
        /// Stable machine-readable reason identifier (e.g.
        /// `formspec-signing-helper-pending`).
        reason: String,
    },

    /// The primitive ran and rejected the signature.
    Failed {
        /// Stable machine-readable reason identifier.
        reason: String,
    },
}

/// Closed reason for binding-reported signature admission failure.
///
/// These values mirror the `signatureAdmissionFailed.reason` schema enum.
/// Bindings use this type only after they have enough source evidence for WOS
/// to build the required evidence bindings; shape errors without those
/// bindings should remain regular binding errors.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatureAdmissionFailureReason {
    /// The cryptographic signature primitive ran and rejected the signature.
    PrimitiveVerificationFailed,

    /// The identity method or signing intent is unsupported by this deployment.
    MethodUnsupported,

    /// The signature method URI is not registered.
    MethodUnregistered,

    /// Source evidence diverged from the binding-verified evidence.
    EvidenceDivergence,

    /// The active posture floor is not met.
    PostureFloorUnmet,

    /// The signature-method registry document cannot be trusted.
    RegistryUnrecognizedMethod,

    /// The required verification adapter is unavailable.
    AdapterUnavailable,
}

/// Binding-reported signature admission failure.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureAdmissionFailure {
    /// Closed failure reason consumed by WOS provenance emission.
    pub reason: SignatureAdmissionFailureReason,

    /// Reason-specific structured context for the provenance record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Binding-neutral verified signature evidence for WOS Signature Profile
/// admission.
#[derive(Debug, Clone, PartialEq)]
pub struct SignatureEvidence {
    /// Binding or provider family that produced the verified evidence.
    pub source_system: String,

    /// Stable signature id from the source system.
    pub source_signature_id: String,

    /// Opaque signing-act identifier when distinct from [`source_signature_id`].
    #[allow(clippy::option_option)]
    pub signing_act_id: Option<String>,

    /// Rendered-presentation digest when distinct from [`document_hash`].
    #[allow(clippy::option_option)]
    pub presentation_hash: Option<String>,

    /// Optional source response or evidence artifact reference.
    pub source_response_ref: Option<String>,

    /// Source document id.
    pub document_id: String,

    /// Signer id, when supplied by the binding.
    pub signer_id: Option<String>,

    /// WOS signing-intent URI carried by the source evidence.
    pub signing_intent: String,

    /// Cryptographic signature method URI from the source evidence.
    ///
    /// Formspec authored signatures carry this as `signatureMethod`. WOS posture
    /// admission uses the value to compare against `allowedMethods`.
    pub signature_method: Option<String>,

    /// Digest of the signed payload verified by the binding.
    pub signed_payload_digest: String,

    /// Digest algorithm used for `signed_payload_digest`.
    pub signed_payload_digest_algorithm: String,

    /// Signing timestamp supplied by the source evidence.
    pub signed_at: String,

    /// Signing-surface or rendered-document digest.
    pub document_hash: String,

    /// Digest algorithm for `document_hash`.
    pub document_hash_algorithm: String,

    /// Source signature provider, when distinct from `source_system`.
    pub signature_provider: Option<String>,

    /// Provider or adapter ceremony id.
    pub ceremony_id: Option<String>,

    /// Provider-neutral identity binding.
    pub identity_binding: Option<serde_json::Value>,

    /// WOS signer-authority claim supplied by the source or response.
    pub signer_authority: Option<serde_json::Value>,

    /// Cryptographic primitive-verification status reported by the binding.
    ///
    /// Bindings that have not yet executed the cryptographic signature
    /// primitive (e.g. the reference Formspec binding while
    /// `FORMSPEC-SIGN-HELPER-001` is unshipped) MUST emit
    /// [`SignaturePrimitiveStatus::DeferredPendingHelper`] so admission
    /// records the verification gap honestly. Bindings that have run and
    /// passed the primitive emit [`SignaturePrimitiveStatus::Verified`].
    pub primitive_verification: SignaturePrimitiveStatus,

    /// Base64-encoded COSE_Sign1 VerificationReceipt bytes, when a verifier
    /// has produced a signed receipt for this signature.
    pub verification_receipt: Option<String>,

    /// Terminal admission failure reported by the binding.
    ///
    /// This is used for verifier/registry outcomes such as unregistered
    /// methods, corrupt registries, unavailable adapters, and source-evidence
    /// divergence. When present, WOS emits `SignatureAdmissionFailed` and does
    /// not continue toward `SignatureAffirmation`.
    pub admission_failure: Option<SignatureAdmissionFailure>,
}

impl SignatureEvidence {
    /// Returns the K-2 signing-act id, preferring an explicit binding value.
    #[must_use]
    pub fn effective_signing_act_id(&self) -> &str {
        self.signing_act_id
            .as_deref()
            .unwrap_or(self.source_signature_id.as_str())
    }

    /// Returns the K-2 presentation digest, preferring an explicit binding value.
    #[must_use]
    pub fn effective_presentation_hash(&self) -> &str {
        self.presentation_hash
            .as_deref()
            .unwrap_or(self.document_hash.as_str())
    }
}

/// Errors produced by binding adapters.
#[derive(Debug, Clone, thiserror::Error)]
pub enum BindingError {
    /// The binding-specific processor is unavailable or retriable.
    #[error("binding processor unavailable: {0}")]
    ProcessorUnavailable(String),

    /// The adapter rejected invalid input or shape.
    #[error("binding input invalid: {0}")]
    InvalidInput(String),

    /// The binding adapter is unsupported.
    #[error("binding unsupported: {0}")]
    Unsupported(String),
}

/// Binding-specific task adapter.
///
/// This trait is intentionally scoped to task-bound contract flows. It does not
/// currently model host-side intake-handoff acceptance or binding-owned
/// auxiliary provenance emission outside task submission.
pub trait ContractBindingAdapter: Send + Sync {
    /// Binding discriminator handled by this adapter.
    fn binding(&self) -> &'static str;

    /// Prepare task presentation data.
    fn prepare_task(
        &self,
        task: &ActiveTask,
        case_state: &serde_json::Value,
    ) -> Result<PreparedTask, BindingError>;

    /// Validate a completed submission.
    fn validate_submission(
        &self,
        task: &ActiveTask,
        response: &serde_json::Value,
    ) -> Result<SubmissionValidation, BindingError>;

    /// Compute the proposed case mutation for a completed submission.
    fn compute_case_mutation(
        &self,
        task: &ActiveTask,
        response: &serde_json::Value,
    ) -> Result<Option<CaseMutationBundle>, BindingError>;

    /// Extract verified signature evidence from a completed submission.
    ///
    /// Base WOS consumes this binding-neutral evidence when a Signature Profile
    /// task is admitted. Adapters that do not own signature evidence use the
    /// default `None`.
    fn signature_evidence(
        &self,
        _task: &ActiveTask,
        _response: &serde_json::Value,
    ) -> Result<Option<Vec<SignatureEvidence>>, BindingError> {
        Ok(None)
    }
}

/// Registry of available contract binding adapters.
#[derive(Clone, Default)]
pub struct BindingRegistry {
    adapters: HashMap<String, Arc<dyn ContractBindingAdapter>>,
}

impl BindingRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an adapter by its binding discriminator.
    pub fn register<A>(&mut self, adapter: A)
    where
        A: ContractBindingAdapter + 'static,
    {
        self.adapters
            .insert(adapter.binding().to_string(), Arc::new(adapter));
    }

    /// Resolve an adapter for the requested binding.
    pub fn get(&self, binding: &str) -> Option<Arc<dyn ContractBindingAdapter>> {
        self.adapters.get(binding).cloned()
    }
}
