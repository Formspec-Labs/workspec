// Rust guideline compliant 2026-02-21

//! Durable runtime command contracts.
//!
//! This module exposes the storage- and backend-neutral command surface for a
//! WOS runtime. Concrete adapters can implement the trait directly, while the
//! current `WosRuntime` remains the reference in-memory adapter.

use wos_core::instance::{CaseInstance, PendingEvent};
use wos_core::provenance::ProvenanceRecord;

use crate::custody::{CustodyAppendContext, CustodyAppendInput, CustodyAppendReceipt};
use crate::runtime::{
    CreateInstanceRequest, DrainOnceResult, PersistDraftResult, RuntimeError, TaskSubmissionResult,
};

/// Executes durable WOS runtime commands.
///
/// This trait is the center seam for runtime backends. It captures the
/// spec-facing command surface without committing callers to the current
/// in-memory `WosRuntime` implementation.
pub trait DurableRuntime {
    /// Creates and persists a new workflow instance.
    ///
    /// # Errors
    /// Returns an error when instance creation, kernel resolution, evaluation,
    /// or persistence fails.
    fn create_instance(
        &mut self,
        request: CreateInstanceRequest,
    ) -> Result<CaseInstance, RuntimeError>;

    /// Loads the canonical instance state.
    ///
    /// # Errors
    /// Returns an error when the instance cannot be found or loaded.
    fn load_instance(&self, instance_id: &str) -> Result<CaseInstance, RuntimeError>;

    /// Appends an event to the durable queue.
    ///
    /// # Errors
    /// Returns an error when the instance cannot be found, timestamping fails,
    /// or persistence fails.
    fn enqueue_event(&mut self, instance_id: &str, event: PendingEvent)
    -> Result<(), RuntimeError>;

    /// Drains a single queued event.
    ///
    /// # Errors
    /// Returns an error when loading, evaluation, host interaction, or
    /// persistence fails.
    fn drain_once(&mut self, instance_id: &str) -> Result<DrainOnceResult, RuntimeError>;

    /// Drains until the instance becomes idle.
    ///
    /// # Errors
    /// Returns an error when any `drain_once` step fails.
    fn drain_until_idle(&mut self, instance_id: &str)
    -> Result<Vec<DrainOnceResult>, RuntimeError>;

    /// Persists a task draft artifact.
    ///
    /// # Errors
    /// Returns an error when authorization, task lookup, validation, or
    /// persistence fails.
    fn persist_task_draft(
        &mut self,
        task_id: &str,
        response: serde_json::Value,
        actor_id: &str,
        idempotency_token: Option<&str>,
    ) -> Result<PersistDraftResult, RuntimeError>;

    /// Records a task dismissal.
    ///
    /// # Errors
    /// Returns an error when the task cannot be found, presenter delivery
    /// fails, or persistence fails.
    fn dismiss_task(&mut self, task_id: &str, reason: &str) -> Result<(), RuntimeError>;

    /// Submits a completed task response.
    ///
    /// # Errors
    /// Returns an error when authorization, resolution, validation,
    /// evaluation, or persistence fails.
    fn submit_task_response(
        &mut self,
        task_id: &str,
        response: serde_json::Value,
        actor_id: &str,
        idempotency_token: Option<&str>,
    ) -> Result<TaskSubmissionResult, RuntimeError>;

    /// Loads an append-only provenance window.
    ///
    /// # Errors
    /// Returns an error when the instance cannot be found or loaded.
    fn load_provenance_window(
        &self,
        instance_id: &str,
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
        instance_id: &str,
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
        instance_id: &str,
        record_id: &str,
        receipt: CustodyAppendReceipt,
    ) -> Result<(), RuntimeError>;
}
