// Rust guideline compliant 2026-02-21

//! Instance lifecycle commands for the reference runtime.
//!
//! This module owns the durable command methods that create, load, enqueue,
//! and expose append windows for workflow processes. The methods still operate on
//! `WosRuntime`, but keeping them separate from the type declarations makes the
//! adapter boundary easier to audit.

use wos_core::eval::{Evaluator, validate_migration_configuration};
use wos_core::instance::{InstanceStatus, PendingEvent, WorkflowProcess};
use wos_core::provenance::{InstanceMigratedInput, ProvenanceRecord};
use wos_core::typeid;

use crate::custody::{CustodyAppendContext, CustodyAppendInput};
use crate::store::RuntimeRecord;

use super::timers::{
    annotate_timer_created_with_calendar_version, annotate_timer_created_with_convergence_error,
    timers_to_state,
};
use super::{
    CreateInstanceRequest, MigrationMap, MigrationOutcome, RuntimeError, WosRuntime,
    format_timestamp, populate_provenance_record_fields, stamp_custody_receipt, stamp_provenance,
};

fn apply_migration_map(
    case_state: &mut serde_json::Value,
    map: &MigrationMap,
) -> Result<(), RuntimeError> {
    let obj = case_state.as_object_mut().ok_or_else(|| {
        RuntimeError::MigrationRejected(
            "caseState must be a JSON object for migration map application".into(),
        )
    })?;
    for key in &map.field_removals {
        obj.remove(key);
    }
    for (old_key, new_key) in &map.field_renames {
        if let Some(value) = obj.remove(old_key) {
            obj.insert(new_key.clone(), value);
        }
    }
    for (key, default_value) in &map.field_defaults {
        obj.entry(key.clone())
            .or_insert_with(|| default_value.clone());
    }
    for (field, kind) in &map.field_coercions {
        let Some(value) = obj.get_mut(field) else {
            continue;
        };
        match kind.as_str() {
            "number" => {
                let n = match &*value {
                    serde_json::Value::Number(n) => n.as_f64().ok_or_else(|| {
                        RuntimeError::MigrationRejected(format!(
                            "fieldCoercion number: field `{field}` is not a finite number"
                        ))
                    })?,
                    serde_json::Value::String(s) => s.parse::<f64>().map_err(|_| {
                        RuntimeError::MigrationRejected(format!(
                            "fieldCoercion number: cannot parse field `{field}`"
                        ))
                    })?,
                    _ => {
                        return Err(RuntimeError::MigrationRejected(format!(
                            "fieldCoercion number: unsupported JSON type on field `{field}`"
                        )));
                    }
                };
                if !n.is_finite() {
                    return Err(RuntimeError::MigrationRejected(format!(
                        "fieldCoercion number: field `{field}` must be finite"
                    )));
                }
                *value = serde_json::json!(n);
            }
            "string" => {
                let s = match &*value {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                *value = serde_json::Value::String(s);
            }
            "boolean" => {
                let b = match &*value {
                    serde_json::Value::Bool(b) => *b,
                    serde_json::Value::String(s) if s.eq_ignore_ascii_case("true") => true,
                    serde_json::Value::String(s) if s.eq_ignore_ascii_case("false") => false,
                    _ => {
                        return Err(RuntimeError::MigrationRejected(format!(
                            "fieldCoercion boolean: cannot coerce field `{field}`"
                        )));
                    }
                };
                *value = serde_json::Value::Bool(b);
            }
            other => {
                return Err(RuntimeError::MigrationRejected(format!(
                    "unknown fieldCoercion `{other}` for field `{field}`"
                )));
            }
        }
    }
    Ok(())
}

fn requested_type_id(value: &str) -> Option<&str> {
    if typeid::is_valid_type_id(value, None) {
        Some(value)
    } else {
        WorkflowProcess::extract_urn_type_id(value)
    }
}

impl WosRuntime {
    /// Create and persist a new workflow process.
    ///
    /// # Errors
    /// Returns an error when kernel resolution, evaluation, task staging, or
    /// persistence fails.
    pub fn create_instance(
        &mut self,
        request: CreateInstanceRequest,
    ) -> Result<WorkflowProcess, RuntimeError> {
        self.create_instance_inner(request, None)
    }

    /// Create and persist a new workflow process bound to an existing case ledger.
    ///
    /// # Errors
    /// Returns an error when the case ledger id is invalid, its tenant disagrees
    /// with the process id, or normal instance creation fails.
    pub fn create_instance_bound_to_case(
        &mut self,
        request: CreateInstanceRequest,
        case_ledger_id: String,
    ) -> Result<WorkflowProcess, RuntimeError> {
        self.create_instance_inner(request, Some(case_ledger_id))
    }

    fn create_instance_inner(
        &mut self,
        request: CreateInstanceRequest,
        bound_case_ledger_id: Option<String>,
    ) -> Result<WorkflowProcess, RuntimeError> {
        let now_ms = self.clock.now_ms();
        let now_iso = format_timestamp(now_ms)?;
        let CreateInstanceRequest {
            process_id,
            tenant: requested_tenant,
            definition_url,
            definition_version,
            initial_case_state,
        } = request;

        // ADR 0068 D-1.2: resolve the authoritative tenant.
        // 1. If the caller supplied an explicit tenant, validate it.
        // 2. If process_id is a valid TypeID, extract the prefix.
        // 3. If both are present, they must match.
        // 4. Fall back to the deployment-default tenant.
        let type_id_tenant = typeid::extract_tenant(&process_id)
            .map(String::from)
            .or_else(|| {
                wos_core::instance::WorkflowProcess::extract_urn_parts(&process_id)
                    .map(String::from)
            });
        let bound_case_ledger_id = bound_case_ledger_id
            .map(|value| {
                let normalized = requested_type_id(&value)
                    .map(str::to_string)
                    .unwrap_or_else(|| value.clone());
                if !typeid::is_valid_type_id(&normalized, Some(typeid::CASE_PREFIX)) {
                    return Err(RuntimeError::MigrationRejected(format!(
                        "case ledger id `{value}` is not a case TypeID"
                    )));
                }
                Ok(normalized)
            })
            .transpose()?;
        let bound_case_tenant = bound_case_ledger_id
            .as_deref()
            .and_then(typeid::extract_tenant)
            .map(String::from);
        let inferred_tenant = type_id_tenant.as_ref().or_else(|| {
            if process_id.trim().is_empty() {
                bound_case_tenant.as_ref()
            } else {
                None
            }
        });
        let tenant = match (requested_tenant, inferred_tenant) {
            (Some(explicit), Some(prefix)) => {
                if !typeid::is_valid_tenant(&explicit) {
                    return Err(RuntimeError::TenantInvalid(explicit));
                }
                if explicit != *prefix {
                    return Err(RuntimeError::TenantMismatch {
                        explicit,
                        type_id_prefix: prefix.clone(),
                    });
                }
                explicit
            }
            (Some(explicit), None) => {
                if !typeid::is_valid_tenant(&explicit) {
                    return Err(RuntimeError::TenantInvalid(explicit));
                }
                explicit
            }
            (None, Some(prefix)) => prefix.clone(),
            (None, None) => typeid::DEFAULT_TENANT.to_string(),
        };
        let bound_case_ledger_id = bound_case_ledger_id
            .map(|value| {
                if let Some(prefix) = typeid::extract_tenant(&value)
                    && prefix != tenant
                {
                    return Err(RuntimeError::TenantMismatch {
                        explicit: tenant.clone(),
                        type_id_prefix: prefix.to_string(),
                    });
                }
                Ok(value)
            })
            .transpose()?;

        let process_id = if process_id.trim().is_empty() {
            typeid::mint_type_id(&tenant, typeid::PROCESS_PREFIX)
        } else if let Some(input_type_id) = requested_type_id(&process_id) {
            if !typeid::is_valid_type_id(input_type_id, Some(typeid::PROCESS_PREFIX)) {
                return Err(RuntimeError::MigrationRejected(format!(
                    "process id `{process_id}` is not a process TypeID"
                )));
            }
            input_type_id.to_string()
        } else {
            return Err(RuntimeError::MigrationRejected(format!(
                "process id `{process_id}` is not a WOS TypeID"
            )));
        };
        let case_ledger_id = bound_case_ledger_id
            .unwrap_or_else(|| typeid::mint_type_id(&tenant, typeid::CASE_PREFIX));
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
        let instance = WorkflowProcess {
            process_id,
            case_ledger_id,
            tenant,
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
            decline_reason: None,
            voided_by: None,
            voided_at: None,
            expired_at: None,
            pending_events: Vec::new(),
            governance_state: None,
            volume_counters: None,
            fired_milestones: Default::default(),
            pending_callbacks: Default::default(),
            created_at: now_iso.clone(),
            updated_at: now_iso.clone(),
            extensions: Default::default(),
        };
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

    /// Load the canonical workflow process state.
    ///
    /// # Errors
    /// Returns an error when the instance cannot be found or loaded.
    pub fn load_instance(&self, process_id: &str) -> Result<WorkflowProcess, RuntimeError> {
        Ok(self
            .load_record_by_process_or_case_ref(process_id)?
            .instance)
    }

    pub(super) fn load_record_by_process_or_case_ref(
        &self,
        id: &str,
    ) -> Result<RuntimeRecord, RuntimeError> {
        match self.store.load_record(id) {
            Ok(record) => return Ok(record),
            Err(crate::store::StoreError::NotFound(_)) => {}
            Err(error) => return Err(RuntimeError::from(error)),
        }

        let normalized = WorkflowProcess::extract_urn_type_id(id).unwrap_or(id);
        if normalized != id {
            match self.store.load_record(normalized) {
                Ok(record) => return Ok(record),
                Err(crate::store::StoreError::NotFound(_)) => {}
                Err(error) => return Err(RuntimeError::from(error)),
            }
        }

        self.store
            .load_record_by_case_ledger_id(normalized)
            .map_err(RuntimeError::from)
    }

    /// Append an event to the instance queue.
    ///
    /// # Errors
    /// Returns an error when the instance cannot be found, timestamping fails,
    /// or persistence fails.
    pub fn enqueue_event(
        &mut self,
        process_id: &str,
        mut event: PendingEvent,
    ) -> Result<(), RuntimeError> {
        let mut record = self.store.load_record(process_id)?;
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
        process_id: &str,
        cursor: usize,
        limit: usize,
        context: CustodyAppendContext,
    ) -> Result<Vec<CustodyAppendInput>, RuntimeError> {
        let record = self.load_record_by_process_or_case_ref(process_id)?;
        record
            .provenance_log
            .iter()
            .enumerate()
            .skip(cursor)
            .take(limit)
            .map(|(position, provenance)| {
                let metadata = context.metadata_for_provenance_record(
                    &record.instance.case_ledger_id,
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
        process_id: &str,
        record_id: &str,
        receipt: crate::custody::CustodyAppendReceipt,
    ) -> Result<(), RuntimeError> {
        let mut record = self.store.load_record(process_id)?;
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

    /// Migrate a running instance to a new kernel `definitionVersion` for the
    /// same `definitionUrl` (Kernel S11.2, ADR 0083 D1).
    ///
    /// Validates active states against the target kernel, applies
    /// [`MigrationMap`] to `caseState`, appends one `instanceMigrated`
    /// provenance record, and persists atomically via [`RuntimeStore::save_record`].
    ///
    /// **Idempotency:** When `target_definition_version` already matches the
    /// instance's current version, this returns [`MigrationOutcome`] immediately
    /// with no store write and no new provenance. HTTP `Idempotency-Key` replay
    /// for successful version bumps is handled in `wos-server` (ADR 0083 D5).
    ///
    /// # Errors
    /// Returns [`RuntimeError::MigrationRejected`] when validation or map
    /// application fails; the instance is not modified.
    pub fn migrate(
        &mut self,
        process_id: &str,
        target_definition_version: &str,
        migration_map: MigrationMap,
        operator_actor_id: Option<&str>,
    ) -> Result<MigrationOutcome, RuntimeError> {
        let now_ms = self.clock.now_ms();
        let now_iso = format_timestamp(now_ms)?;
        let mut record = self.store.load_record(process_id)?;
        if record.instance.definition_version == target_definition_version {
            return Ok(MigrationOutcome {
                process_id: process_id.to_string(),
                previous_definition_version: record.instance.definition_version.clone(),
                new_definition_version: target_definition_version.to_string(),
                migration_map,
            });
        }

        let url = record.instance.definition_url.clone();
        let previous_definition_version = record.instance.definition_version.clone();
        let target_kernel = self
            .resolver
            .resolve_kernel(&url, target_definition_version)?;
        validate_migration_configuration(&target_kernel, &record.instance.configuration)
            .map_err(|e| RuntimeError::MigrationRejected(e.to_string()))?;

        let mut new_case_state = record.instance.case_state.clone();
        apply_migration_map(&mut new_case_state, &migration_map)?;

        let migration_map_json = serde_json::to_value(&migration_map)
            .map_err(|e| RuntimeError::MigrationRejected(e.to_string()))?;
        let mut appended = vec![ProvenanceRecord::instance_migrated(InstanceMigratedInput {
            from_version: previous_definition_version.as_str(),
            to_version: target_definition_version,
            migration_map: migration_map_json,
            actor_id: operator_actor_id,
            context: None,
        })];
        populate_provenance_record_fields(&mut appended, &target_kernel, target_definition_version);
        stamp_provenance(&mut appended, &now_iso);

        record.instance.case_state = new_case_state;
        record.instance.definition_version = target_definition_version.to_string();
        record.instance.updated_at = now_iso;
        record.provenance_log.extend(appended);
        record.instance.provenance_position = record.provenance_log.len() as u64;
        self.store.save_record(record)?;

        Ok(MigrationOutcome {
            process_id: process_id.to_string(),
            previous_definition_version,
            new_definition_version: target_definition_version.to_string(),
            migration_map,
        })
    }
}
