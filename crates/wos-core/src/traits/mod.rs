// Rust guideline compliant 2026-02-21

//! Host interface traits (Runtime Companion S12).
//!
//! The processor expects its host to provide implementations of these
//! interfaces. Each trait is a named behavioral contract — implementations
//! map them to their deployment's infrastructure (database, message queue,
//! key store, etc.).
//!
//! See [`DefaultRuntime`] for a bundled set of in-memory stubs suitable
//! for testing and single-user deployments.

use std::collections::HashMap;

use crate::instance::{CaseInstance, FormspecTaskContext};
use crate::model::governance::{DelegationScope, GovernanceDocument};
use crate::model::kernel::KernelDocument;
use crate::provenance::ProvenanceRecord;

/// Persists CaseInstance documents between events (Runtime S12.1).
pub trait InstanceStore {
    /// Error type for store operations.
    type Error: std::error::Error;

    /// Load an instance by ID.
    fn load(&self, instance_id: &str) -> Result<CaseInstance, Self::Error>;

    /// Durably persist an instance. Must be atomic.
    fn save(&mut self, instance: &CaseInstance) -> Result<(), Self::Error>;

    /// List instances that currently include the requested state.
    fn list_by_state(&self, _state_id: &str) -> Result<Vec<String>, Self::Error> {
        Ok(Vec::new())
    }

    /// List instances for a pinned definition version.
    fn list_by_definition(
        &self,
        _definition_url: &str,
        _definition_version: &str,
    ) -> Result<Vec<String>, Self::Error> {
        Ok(Vec::new())
    }
}

/// Loads WOS documents from storage (Runtime S12.2).
pub trait DocumentResolver {
    /// Error type for resolver operations.
    type Error: std::error::Error;

    /// Resolve a Kernel Document by URL and version.
    fn resolve_kernel(&self, url: &str, version: &str) -> Result<KernelDocument, Self::Error>;

    /// Resolve a Governance Document by URL and version.
    fn resolve_governance(
        &self,
        url: &str,
        version: &str,
    ) -> Result<GovernanceDocument, Self::Error>;

    /// Resolve a sidecar document. The returned JSON stays opaque at this seam.
    fn resolve_sidecar(
        &self,
        url: &str,
        anchor_date: Option<&str>,
    ) -> Result<serde_json::Value, Self::Error>;
}

/// Validates data against a Formspec Definition or JSON Schema (Runtime S12.3).
pub trait ContractValidator {
    /// Error type for validation operations.
    type Error: std::error::Error;

    /// Validate data against the referenced contract.
    fn validate(
        &self,
        contract_ref: &str,
        data: &serde_json::Value,
    ) -> Result<ValidationResult, Self::Error>;
}

/// Result of a contract validation.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the data passed validation.
    pub valid: bool,
    /// Validation errors, if any.
    pub errors: Vec<String>,
}

/// Fulfills `invokeService` actions (Runtime S12.4).
pub trait ExternalService {
    /// Error type for service operations.
    type Error: std::error::Error;

    /// Invoke a referenced service.
    fn invoke(
        &self,
        service_ref: &str,
        input: &serde_json::Value,
        idempotency_key: Option<&str>,
    ) -> Result<serde_json::Value, Self::Error>;
}

/// Controls which actors can perform which operations (Runtime S12.5).
pub trait AccessControl {
    /// Whether the actor can trigger this transition.
    fn can_transition(&self, actor_id: &str, transition_event: &str) -> bool;

    /// Whether the actor can read the specified case state field.
    fn can_read(&self, actor_id: &str, field_path: &str) -> bool;

    /// Whether the delegator can delegate work to the delegate within scope.
    fn can_delegate(&self, delegator_id: &str, delegate_id: &str, scope: &DelegationScope) -> bool;
}

/// Signs and verifies provenance records (Runtime S12.6).
pub trait ProvenanceSigner {
    /// Error type for signing operations.
    type Error: std::error::Error;

    /// Sign a provenance record.
    fn sign(&self, record: &ProvenanceRecord) -> Result<Vec<u8>, Self::Error>;

    /// Verify a signed provenance record.
    fn verify(&self, record: &ProvenanceRecord, signature: &[u8]) -> Result<bool, Self::Error>;
}

/// Renders provenance into human-readable formats (Runtime S12.7).
pub trait ReportRenderer {
    /// Error type for render operations.
    type Error: std::error::Error;

    /// Render an explanation structure.
    fn render_explanation(
        &self,
        explanation: &serde_json::Value,
        template: &str,
    ) -> Result<String, Self::Error>;

    /// Render an audit trail into an implementation-defined format.
    fn render_audit(
        &self,
        provenance_log: &[ProvenanceRecord],
        format: &str,
    ) -> Result<String, Self::Error>;
}

/// Manages the per-instance event queue (Runtime S12.8).
pub trait EventQueue {
    /// Error type for queue operations.
    type Error: std::error::Error;

    /// Add an event to the instance's processing queue.
    fn enqueue(&mut self, instance_id: &str, event: serde_json::Value) -> Result<(), Self::Error>;

    /// Remove and return the next event for processing.
    fn dequeue(&mut self, instance_id: &str) -> Result<Option<serde_json::Value>, Self::Error>;

    /// Return the next event without removing it.
    fn peek(&self, instance_id: &str) -> Result<Option<serde_json::Value>, Self::Error>;
}

/// Presents Formspec-backed tasks to a host user interface.
pub trait TaskPresenter {
    /// Error type for presentation operations.
    type Error: std::error::Error;

    /// Present a task to the assigned actor.
    fn present_task(&mut self, context: &FormspecTaskContext) -> Result<(), Self::Error>;

    /// Dismiss a task without advancing lifecycle state.
    fn dismiss_task(&mut self, task_id: &str, reason: &str) -> Result<(), Self::Error>;
}

/// Executes actions that the engine delegates to the host.
///
/// This covers `createTask` and other actions whose side effects
/// are host-specific.
pub trait ActionExecutor {
    /// Error type for executor operations.
    type Error: std::error::Error;

    /// Execute a host-managed action.
    fn execute(
        &mut self,
        action_kind: &str,
        data: &serde_json::Value,
        actor: Option<&str>,
    ) -> Result<serde_json::Value, Self::Error>;
}

// ── Default implementations ─────────────────────────────────────

/// Bundled in-memory stubs for all host interfaces.
///
/// Suitable for testing and single-user deployments. All state is
/// in-memory and lost on restart.
#[derive(Debug, Default)]
pub struct DefaultRuntime {
    /// In-memory instance store.
    instances: HashMap<String, CaseInstance>,
    /// In-memory event queues per instance.
    queues: HashMap<String, Vec<serde_json::Value>>,
}

impl DefaultRuntime {
    /// Create a new default runtime.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Error for default runtime operations.
#[derive(Debug, thiserror::Error)]
pub enum DefaultRuntimeError {
    /// Instance not found.
    #[error("instance not found: {0}")]
    InstanceNotFound(String),
    /// Operation not supported.
    #[error("not supported: {0}")]
    NotSupported(String),
}

impl InstanceStore for DefaultRuntime {
    type Error = DefaultRuntimeError;

    fn load(&self, instance_id: &str) -> Result<CaseInstance, Self::Error> {
        self.instances
            .get(instance_id)
            .cloned()
            .ok_or_else(|| DefaultRuntimeError::InstanceNotFound(instance_id.to_string()))
    }

    fn save(&mut self, instance: &CaseInstance) -> Result<(), Self::Error> {
        self.instances
            .insert(instance.instance_id.clone(), instance.clone());
        Ok(())
    }
}

impl AccessControl for DefaultRuntime {
    fn can_transition(&self, _actor_id: &str, _transition_event: &str) -> bool {
        true // Permissive default.
    }

    fn can_read(&self, _actor_id: &str, _field_path: &str) -> bool {
        true // Permissive default.
    }

    fn can_delegate(
        &self,
        _delegator_id: &str,
        _delegate_id: &str,
        _scope: &DelegationScope,
    ) -> bool {
        true // Permissive default.
    }
}

impl ContractValidator for DefaultRuntime {
    type Error = DefaultRuntimeError;

    fn validate(
        &self,
        _contract_ref: &str,
        _data: &serde_json::Value,
    ) -> Result<ValidationResult, Self::Error> {
        Ok(ValidationResult {
            valid: true,
            errors: Vec::new(),
        })
    }
}

impl EventQueue for DefaultRuntime {
    type Error = DefaultRuntimeError;

    fn enqueue(&mut self, instance_id: &str, event: serde_json::Value) -> Result<(), Self::Error> {
        self.queues
            .entry(instance_id.to_string())
            .or_default()
            .push(event);
        Ok(())
    }

    fn dequeue(&mut self, instance_id: &str) -> Result<Option<serde_json::Value>, Self::Error> {
        Ok(self.queues.get_mut(instance_id).and_then(|q| {
            if q.is_empty() {
                None
            } else {
                Some(q.remove(0))
            }
        }))
    }

    fn peek(&self, instance_id: &str) -> Result<Option<serde_json::Value>, Self::Error> {
        Ok(self
            .queues
            .get(instance_id)
            .and_then(|queue| queue.first().cloned()))
    }
}

impl TaskPresenter for DefaultRuntime {
    type Error = DefaultRuntimeError;

    fn present_task(&mut self, _context: &FormspecTaskContext) -> Result<(), Self::Error> {
        Ok(())
    }

    fn dismiss_task(&mut self, _task_id: &str, _reason: &str) -> Result<(), Self::Error> {
        Ok(())
    }
}
