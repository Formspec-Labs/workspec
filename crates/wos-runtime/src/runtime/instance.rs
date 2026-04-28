// Rust guideline compliant 2026-02-21

//! Instance lifecycle commands for the reference runtime.
//!
//! This module owns the durable command methods that create, load, enqueue,
//! and expose append windows for case instances. The methods still operate on
//! `WosRuntime`, but keeping them separate from the type declarations makes the
//! adapter boundary easier to audit.

use wos_core::eval::Evaluator;
use wos_core::instance::{CaseInstance, InstanceStatus, PendingEvent};

use crate::custody::{CustodyAppendContext, CustodyAppendInput};
use crate::store::RuntimeRecord;

use super::timers::{
    annotate_timer_created_with_calendar_version, annotate_timer_created_with_convergence_error,
    timers_to_state,
};
use super::{
    CreateInstanceRequest, RuntimeError, WosRuntime, format_timestamp,
    populate_provenance_record_fields, stamp_custody_receipt, stamp_provenance,
};

impl WosRuntime {
    /// Create and persist a new case instance.
    ///
    /// # Errors
    /// Returns an error when kernel resolution, evaluation, task staging, or
    /// persistence fails.
    pub fn create_instance(
        &mut self,
        request: CreateInstanceRequest,
    ) -> Result<CaseInstance, RuntimeError> {
        let now_ms = self.clock.now_ms();
        let now_iso = format_timestamp(now_ms)?;
        let CreateInstanceRequest {
            instance_id,
            definition_url,
            definition_version,
            initial_case_state,
        } = request;
        let (instance_id, legacy_alias) = if CaseInstance::is_case_id(&instance_id) {
            (instance_id, None)
        } else if instance_id.trim().is_empty() {
            (CaseInstance::mint_id(), None)
        } else {
            (CaseInstance::mint_id(), Some(instance_id))
        };
        let kernel = self
            .resolver
            .resolve_kernel(&definition_url, &definition_version)?;
        let mut evaluator = Evaluator::with_time_and_case_state(
            kernel.clone(),
            now_ms,
            initial_case_state.as_ref(),
        )
        .map_err(|error| RuntimeError::Evaluator(error.to_string()))?;

        let (timer_states, convergence_error_ids) =
            timers_to_state(evaluator.timers(), self.business_calendar.as_ref())?;
        let instance = CaseInstance {
            instance_id,
            definition_url,
            definition_version,
            configuration: evaluator.configuration().active_states().to_vec(),
            case_state: evaluator.case_state_json(),
            provenance_position: 0,
            next_task_sequence: 0,
            timers: timer_states,
            active_tasks: Vec::new(),
            history_store: Default::default(),
            compensation_logs: Default::default(),
            status: InstanceStatus::Active,
            stalled_since: None,
            pending_events: Vec::new(),
            governance_state: None,
            volume_counters: None,
            fired_milestones: Default::default(),
            pending_callbacks: Default::default(),
            created_at: now_iso.clone(),
            updated_at: now_iso.clone(),
            extensions: Default::default(),
        };
        let mut instance = instance;
        if let Some(alias) = legacy_alias {
            instance.extensions.insert(
                CaseInstance::LEGACY_INSTANCE_ALIAS_EXTENSION_KEY.to_string(),
                serde_json::Value::String(alias),
            );
        }

        let mut record = RuntimeRecord::new(instance);
        let mut appended_provenance = evaluator.provenance().records().to_vec();
        // Annotate any timers created during instance initialization with calendarVersion.
        if let Some(cal) = &self.business_calendar {
            annotate_timer_created_with_calendar_version(&mut appended_provenance, cal);
        }
        // Annotate TimerCreated records for any timers whose calendar deadline did not converge.
        annotate_timer_created_with_convergence_error(
            &mut appended_provenance,
            &convergence_error_ids,
        );
        let actions = evaluator.take_executed_actions();
        let (created_task_ids, emitted_events, runtime_provenance) =
            self.apply_observed_actions(&kernel, &mut record, &actions, &now_iso)?;
        appended_provenance.extend(runtime_provenance);
        let (pending_presentations, presentation_provenance) =
            self.stage_pending_tasks_for_presentation(&mut record, &now_iso)?;
        appended_provenance.extend(presentation_provenance);
        populate_provenance_record_fields(
            &mut appended_provenance,
            &kernel,
            &record.instance.definition_version,
        );
        stamp_provenance(&mut appended_provenance, &now_iso);
        record.instance.provenance_position = appended_provenance.len() as u64;
        record.provenance_log.extend(appended_provenance);
        self.store.create_record(record.clone())?;

        self.deliver_pending_presentations(&pending_presentations)?;

        let _ = (created_task_ids, emitted_events);
        Ok(record.instance)
    }

    /// Load the canonical case instance state.
    ///
    /// # Errors
    /// Returns an error when the instance cannot be found or loaded.
    pub fn load_instance(&self, instance_id: &str) -> Result<CaseInstance, RuntimeError> {
        Ok(self.store.load_record(instance_id)?.instance)
    }

    /// Append an event to the instance queue.
    ///
    /// # Errors
    /// Returns an error when the instance cannot be found, timestamping fails,
    /// or persistence fails.
    pub fn enqueue_event(
        &mut self,
        instance_id: &str,
        mut event: PendingEvent,
    ) -> Result<(), RuntimeError> {
        let mut record = self.store.load_record(instance_id)?;
        if event.timestamp.is_empty() {
            event.timestamp = format_timestamp(self.clock.now_ms())?;
        }
        record.instance.pending_events.push(event);
        record.instance.updated_at = format_timestamp(self.clock.now_ms())?;
        self.store.save_record(record.clone())?;
        Ok(())
    }

    /// Load ADR-0061 custody append inputs by provenance-log window.
    ///
    /// # Errors
    /// Returns an error when the instance cannot be found or when a provenance
    /// record cannot be canonicalized into custody append input form.
    pub fn load_custody_append_window(
        &self,
        instance_id: &str,
        cursor: usize,
        limit: usize,
        context: CustodyAppendContext,
    ) -> Result<Vec<CustodyAppendInput>, RuntimeError> {
        let record = self.store.load_record(instance_id)?;
        record
            .provenance_log
            .iter()
            .enumerate()
            .skip(cursor)
            .take(limit)
            .map(|(position, provenance)| {
                let metadata = context.metadata_for_provenance_record(
                    &record.instance.instance_id,
                    position,
                    provenance,
                )?;
                CustodyAppendInput::from_provenance_record(provenance, &context, metadata)
                    .map_err(RuntimeError::from)
            })
            .collect()
    }

    /// Stamps the Trellis custody receipt onto the matching provenance record.
    ///
    /// # Errors
    /// Returns an error when the instance or provenance record cannot be
    /// located, or when persistence fails.
    pub fn apply_custody_receipt(
        &mut self,
        instance_id: &str,
        record_id: &str,
        receipt: crate::custody::CustodyAppendReceipt,
    ) -> Result<(), RuntimeError> {
        let mut record = self.store.load_record(instance_id)?;
        let Some(provenance) = record
            .provenance_log
            .iter_mut()
            .find(|entry| entry.id == record_id)
        else {
            return Err(RuntimeError::ProvenanceRecordNotFound(
                record_id.to_string(),
            ));
        };
        stamp_custody_receipt(provenance, &receipt)?;
        record.instance.updated_at = format_timestamp(self.clock.now_ms())?;
        self.store.save_record(record)?;
        Ok(())
    }
}
