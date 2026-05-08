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

/// Binding-neutral verified signature evidence for WOS Signature Profile
/// admission.
#[derive(Debug, Clone, PartialEq)]
pub struct SignatureEvidence {
    /// Binding or provider family that produced the verified evidence.
    pub source_system: String,

    /// Stable signature id from the source system.
    pub source_signature_id: String,

    /// Optional source response or evidence artifact reference.
    pub source_response_ref: Option<String>,

    /// Source document id.
    pub document_id: String,

    /// Signer id, when supplied by the binding.
    pub signer_id: Option<String>,

    /// WOS signing-intent URI carried by the source evidence.
    pub signing_intent: String,

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
