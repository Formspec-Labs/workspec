// Rust guideline compliant 2026-02-21

//! DurableRuntime implementation for the reference runtime.
//!
//! The trait impl is intentionally mechanical: it forwards the backend-neutral
//! command contract to `WosRuntime` inherent methods. Keeping this in a small
//! module makes future adapter comparisons focus on command semantics rather
//! than reference-runtime internals.

use wos_core::ProvenanceRecord;
use wos_core::instance::{PendingEvent, WorkflowProcess};

use crate::durable::DurableRuntime;
use crate::intake::{IntakeAcceptanceDecision, IntakeAcceptanceRequest};
use wos_events::custody::{CustodyAppendContext, CustodyAppendInput, CustodyAppendReceipt};

use super::{
    CreateProcessRequest, DrainOnceResult, PersistDraftResult, RuntimeError, TaskSubmissionResult,
    WosRuntime,
};

impl DurableRuntime for WosRuntime {
    fn create_process(
        &mut self,
        request: CreateProcessRequest,
    ) -> Result<WorkflowProcess, RuntimeError> {
        WosRuntime::create_process(self, request)
    }

    fn load_process(&self, process_id: &str) -> Result<WorkflowProcess, RuntimeError> {
        WosRuntime::load_process(self, process_id)
    }

    fn enqueue_event(&mut self, process_id: &str, event: PendingEvent) -> Result<(), RuntimeError> {
        WosRuntime::enqueue_event(self, process_id, event)
    }

    fn drain_once(&mut self, process_id: &str) -> Result<DrainOnceResult, RuntimeError> {
        WosRuntime::drain_once(self, process_id)
    }

    fn drain_until_idle(&mut self, process_id: &str) -> Result<Vec<DrainOnceResult>, RuntimeError> {
        WosRuntime::drain_until_idle(self, process_id)
    }

    fn persist_task_draft(
        &mut self,
        task_id: &str,
        response: serde_json::Value,
        actor_id: &str,
        idempotency_token: Option<&str>,
    ) -> Result<PersistDraftResult, RuntimeError> {
        WosRuntime::persist_task_draft(self, task_id, response, actor_id, idempotency_token)
    }

    fn dismiss_task(&mut self, task_id: &str, reason: &str) -> Result<(), RuntimeError> {
        WosRuntime::dismiss_task(self, task_id, reason)
    }

    fn submit_task_response(
        &mut self,
        task_id: &str,
        response: serde_json::Value,
        actor_id: &str,
        idempotency_token: Option<&str>,
    ) -> Result<TaskSubmissionResult, RuntimeError> {
        WosRuntime::submit_task_response(self, task_id, response, actor_id, idempotency_token)
    }

    fn accept_intake_handoff(
        &mut self,
        binding: &str,
        request: IntakeAcceptanceRequest,
    ) -> Result<IntakeAcceptanceDecision, RuntimeError> {
        WosRuntime::accept_intake_handoff(self, binding, request)
    }

    fn load_provenance_window(
        &self,
        process_id: &str,
        cursor: usize,
        limit: usize,
    ) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
        WosRuntime::load_provenance_window(self, process_id, cursor, limit)
    }

    fn load_custody_append_window(
        &self,
        process_id: &str,
        cursor: usize,
        limit: usize,
        context: CustodyAppendContext,
    ) -> Result<Vec<CustodyAppendInput>, RuntimeError> {
        WosRuntime::load_custody_append_window(self, process_id, cursor, limit, context)
    }

    fn apply_custody_receipt(
        &mut self,
        process_id: &str,
        record_id: &str,
        receipt: CustodyAppendReceipt,
    ) -> Result<(), RuntimeError> {
        WosRuntime::apply_custody_receipt(self, process_id, record_id, receipt)
    }
}
