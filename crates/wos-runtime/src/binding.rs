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
