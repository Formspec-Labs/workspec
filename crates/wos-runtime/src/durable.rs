// Rust guideline compliant 2026-02-21

//! Durable runtime command contracts.
//!
//! This module exposes the storage- and backend-neutral command surface for a
//! WOS runtime. Concrete adapters can implement the trait directly, while the
//! current `WosRuntime` remains the reference in-memory adapter.

use wos_core::ProvenanceRecord;
use wos_core::instance::{PendingEvent, WorkflowProcess};

use crate::intake::{IntakeAcceptanceDecision, IntakeAcceptanceRequest};
use crate::runtime::{
    CreateProcessRequest, DismissTaskResult, DrainOnceResult, PersistDraftResult, RuntimeError,
    TaskSubmissionResult,
};
use wos_events::custody::{CustodyAppendContext, CustodyAppendInput, CustodyAppendReceipt};

/// Executes durable WOS runtime commands.
///
/// This trait is the center seam for runtime backends. It captures the
/// spec-facing command surface without committing callers to the current
/// in-memory `WosRuntime` implementation.
pub trait DurableRuntime {
    /// Creates and persists a new workflow process.
    ///
    /// # Errors
    /// Returns an error when process creation, kernel resolution, evaluation,
    /// or persistence fails.
    fn create_process(
        &mut self,
        request: CreateProcessRequest,
    ) -> Result<WorkflowProcess, RuntimeError>;

    /// Creates a process with explicit host-resolved human assignments.
    ///
    /// # Errors
    /// Refuses on backends without explicit binding support, including empty bindings.
    fn create_process_with_actor_bindings(
        &mut self,
        _request: CreateProcessRequest,
        _bindings: Vec<wos_core::instance::HumanActorBinding>,
    ) -> Result<WorkflowProcess, RuntimeError> {
        Err(RuntimeError::UnsupportedBinding(
            "explicit human assignments".into(),
        ))
    }

    /// Loads the canonical workflow process state.
    ///
    /// # Errors
    /// Returns an error when the process cannot be found or loaded.
    fn load_process(&self, process_id: &str) -> Result<WorkflowProcess, RuntimeError>;

    /// Appends an event to the durable queue.
    ///
    /// # Errors
    /// Returns an error when the instance cannot be found, timestamping fails,
    /// or persistence fails.
    fn enqueue_event(&mut self, process_id: &str, event: PendingEvent) -> Result<(), RuntimeError>;

    /// Drains a single queued event.
    ///
    /// # Errors
    /// Returns an error when loading, evaluation, host interaction, or
    /// persistence fails.
    fn drain_once(&mut self, process_id: &str) -> Result<DrainOnceResult, RuntimeError>;

    /// Drains until the instance becomes idle.
    ///
    /// # Errors
    /// Returns an error when any `drain_once` step fails.
    fn drain_until_idle(&mut self, process_id: &str) -> Result<Vec<DrainOnceResult>, RuntimeError>;

    /// Persists a task draft artifact.
    ///
    /// Responses retain their original admitted text through storage and replay.
    ///
    /// # Errors
    /// Returns an error when authorization, task lookup, validation, or
    /// persistence fails.
    fn persist_task_draft(
        &mut self,
        task_id: &str,
        response: integrity_canonical::ResponseJson,
        actor_id: &str,
        idempotency_token: Option<&str>,
    ) -> Result<PersistDraftResult, RuntimeError>;

    /// Records an authorized UI dismissal while keeping the task resumable.
    ///
    /// Atomically retains exact reason, original provenance ID/time and replay evidence.
    /// The required token is preserved across retries. Replay precedes historical
    /// engine authorization; the host still checks current access and output policy.
    /// This command never calls the presentation sink.
    ///
    /// # Errors
    /// Returns an error when authorization, task lookup, resolution, timestamp
    /// formatting, replay evidence validation, key conflict or persistence fails.
    fn dismiss_task(
        &mut self,
        task_id: &str,
        reason: &str,
        actor_id: &str,
        idempotency_token: &str,
    ) -> Result<DismissTaskResult, RuntimeError>;

    /// Submits a completed task response.
    ///
    /// Signature commitments read the admitted text, never its parsed numeric view.
    ///
    /// # Errors
    /// Returns an error when authorization, resolution, validation,
    /// evaluation, or persistence fails.
    fn submit_task_response(
        &mut self,
        task_id: &str,
        response: integrity_canonical::ResponseJson,
        actor_id: &str,
        idempotency_token: Option<&str>,
    ) -> Result<TaskSubmissionResult, RuntimeError>;

    /// Accepts a binding-native intake handoff through the host-side intake seam.
    ///
    /// # Errors
    /// Returns an error when the binding is unsupported or the adapter rejects
    /// the intake handoff.
    fn accept_intake_handoff(
        &mut self,
        binding: &str,
        request: IntakeAcceptanceRequest,
    ) -> Result<IntakeAcceptanceDecision, RuntimeError>;

    /// Loads an append-only provenance window.
    ///
    /// # Errors
    /// Returns an error when the instance cannot be found or loaded.
    fn load_provenance_window(
        &self,
        process_id: &str,
        cursor: usize,
        limit: usize,
    ) -> Result<Vec<ProvenanceRecord>, RuntimeError>;

    /// Loads an ADR-0061 custody append window.
    ///
    /// # Errors
    /// Returns an error when the instance cannot be found or a provenance
    /// record cannot be canonicalized into custody append input form.
    fn load_custody_append_window(
        &self,
        process_id: &str,
        cursor: usize,
        limit: usize,
        context: CustodyAppendContext,
    ) -> Result<Vec<CustodyAppendInput>, RuntimeError>;

    /// Stamps a custody receipt onto the matching provenance record.
    ///
    /// # Errors
    /// Returns an error when the instance or provenance record cannot be
    /// located, or when persistence fails.
    fn apply_custody_receipt(
        &mut self,
        process_id: &str,
        record_id: &str,
        receipt: CustodyAppendReceipt,
    ) -> Result<(), RuntimeError>;
}
