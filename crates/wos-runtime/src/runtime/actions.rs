// Rust guideline compliant 2026-02-21

//! Action execution for the reference runtime.
//!
//! This module keeps observed-action realization and presentation staging
//! separate from the core event-drain loop. The extracted methods still
//! operate on `WosRuntime` directly, but the split makes the center seam
//! explicit before alternate durable adapters are introduced.

use semver::{Version, VersionReq};
use wos_core::eval::ObservedAction;
use wos_core::instance::{
    ActiveTask, ActiveTaskStatus, CaseInstance, FormspecTaskContext, PendingEvent,
};
use wos_core::model::kernel::{ActionKind, KernelDocument};
use wos_core::provenance::{ProvenanceKind, ProvenanceRecord};

use crate::integration::IntegrationBinding;
use crate::integration_handlers::{
    InvocationContext, dispatch_integration_binding, load_or_invoke_service_result,
};
use crate::store::RuntimeRecord;

use super::{
    COMPLETION_EVENT_EXTENSION_KEY, FAILURE_EVENT_EXTENSION_KEY, RuntimeError, WosRuntime,
    impact_level_label, make_task_id, normalize_semver_range_expression,
    signature::append_signature_task_extensions,
};

impl WosRuntime {
    pub(super) fn apply_observed_actions(
        &mut self,
        kernel: &KernelDocument,
        record: &mut RuntimeRecord,
        actions: &[ObservedAction],
        now_iso: &str,
    ) -> Result<(Vec<String>, Vec<String>, Vec<ProvenanceRecord>), RuntimeError> {
        let mut created_task_ids = Vec::new();
        let mut emitted_events = Vec::new();
        let mut provenance = Vec::new();

        for observed in actions {
            match observed.action.action {
                ActionKind::CreateTask => {
                    let task = self.create_active_task(kernel, record, observed, now_iso)?;
                    created_task_ids.push(task.task_id.clone());
                    provenance.push(ProvenanceRecord::task_lifecycle(
                        ProvenanceKind::TaskCreated,
                        &task.task_id,
                        observed.actor_id.as_deref(),
                        Some(serde_json::json!({
                            "taskRef": task.task_ref,
                            "binding": task.binding,
                        })),
                    ));
                    record.instance.active_tasks.push(task);
                }
                ActionKind::EmitEvent => {
                    let event_name = observed.action.event_type.clone().ok_or_else(|| {
                        RuntimeError::UnsupportedAction("emitEvent missing eventType".to_string())
                    })?;
                    record.instance.pending_events.push(PendingEvent {
                        event: event_name.clone(),
                        actor_id: observed.actor_id.clone(),
                        data: observed.action.data.clone(),
                        timestamp: now_iso.to_string(),
                        idempotency_token: None,
                    });
                    emitted_events.push(event_name);
                }
                ActionKind::InvokeService => {
                    let service_ref = observed.action.service_ref.clone().ok_or_else(|| {
                        RuntimeError::UnsupportedAction(
                            "invokeService missing serviceRef".to_string(),
                        )
                    })?;
                    let integration_binding = self
                        .integration_profile
                        .as_ref()
                        .and_then(|profile| profile.bindings.get(&service_ref))
                        .cloned();
                    if let Some(binding) = integration_binding {
                        provenance.extend(self.invoke_integration_binding(
                            record,
                            kernel,
                            observed,
                            &service_ref,
                            &binding,
                            now_iso,
                        )?);
                        continue;
                    }

                    let input = observed
                        .action
                        .data
                        .clone()
                        .unwrap_or_else(|| serde_json::json!({}));
                    let idempotency_key = observed.action.idempotency_key.as_deref();
                    let (step_result, reused_persisted_result) = load_or_invoke_service_result(
                        self.service.as_ref(),
                        record,
                        &service_ref,
                        &input,
                        idempotency_key,
                        now_iso,
                    )?;

                    if reused_persisted_result {
                        provenance.push(ProvenanceRecord {
                            id: ProvenanceRecord::mint_id(),
                            record_kind: ProvenanceKind::IdempotencyDedup,
                            timestamp: String::new(),
                            actor_id: observed.actor_id.clone(),
                            from_state: None,
                            to_state: None,
                            event: None,
                            data: Some(serde_json::json!({
                                "serviceRef": service_ref,
                                "idempotencyKey": idempotency_key,
                                "stepResultRecordedAt": step_result.recorded_at,
                            })),
                            audit_layer: None,
                            actor_type: None,
                            lifecycle_state: None,
                            definition_version: None,
                            inputs: Vec::new(),
                            outputs: Vec::new(),
                            input_digest: None,
                            output_digest: None,
                            canonical_event_hash: None,
                            transition_tags: Vec::new(),
                            case_file_snapshot: None,
                            outcome: None,
                        });
                    } else {
                        provenance.push(ProvenanceRecord {
                            id: ProvenanceRecord::mint_id(),
                            record_kind: ProvenanceKind::StepResultPersisted,
                            timestamp: String::new(),
                            actor_id: observed.actor_id.clone(),
                            from_state: None,
                            to_state: None,
                            event: None,
                            data: Some(serde_json::json!({
                                "serviceRef": service_ref,
                                "idempotencyKey": idempotency_key,
                                "output": step_result.output,
                                "persistedBeforeAdvance": true,
                            })),
                            audit_layer: None,
                            actor_type: None,
                            lifecycle_state: None,
                            definition_version: None,
                            inputs: Vec::new(),
                            outputs: Vec::new(),
                            input_digest: None,
                            output_digest: None,
                            canonical_event_hash: None,
                            transition_tags: Vec::new(),
                            case_file_snapshot: None,
                            outcome: None,
                        });
                    }

                    if let Some(contract_ref) = observed.action.contract_ref.as_deref() {
                        let validation_result =
                            self.validator.validate(contract_ref, &step_result.output)?;
                        provenance.push(ProvenanceRecord {
                            id: ProvenanceRecord::mint_id(),
                            record_kind: ProvenanceKind::ContractValidation,
                            timestamp: String::new(),
                            actor_id: observed.actor_id.clone(),
                            from_state: None,
                            to_state: None,
                            event: None,
                            data: Some(serde_json::json!({
                                "contractRef": contract_ref,
                                "structured": true,
                                "valid": validation_result.valid,
                                "errors": validation_result.errors,
                            })),
                            audit_layer: None,
                            actor_type: None,
                            lifecycle_state: None,
                            definition_version: None,
                            inputs: Vec::new(),
                            outputs: Vec::new(),
                            input_digest: None,
                            output_digest: None,
                            canonical_event_hash: None,
                            transition_tags: Vec::new(),
                            case_file_snapshot: None,
                            outcome: None,
                        });
                    }
                }
                ActionKind::SetData
                | ActionKind::StartTimer
                | ActionKind::CancelTimer
                | ActionKind::Log => {}
            }
        }

        Ok((created_task_ids, emitted_events, provenance))
    }

    pub(super) fn invoke_integration_binding(
        &mut self,
        record: &mut RuntimeRecord,
        kernel: &KernelDocument,
        observed: &ObservedAction,
        service_ref: &str,
        binding: &IntegrationBinding,
        now_iso: &str,
    ) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
        self.validate_integration_profile_target(kernel, &record.instance)?;
        let ctx = InvocationContext {
            service: self.service.as_ref(),
            validator: self.validator.as_ref(),
        };
        dispatch_integration_binding(
            &ctx,
            record,
            kernel,
            observed,
            service_ref,
            binding,
            now_iso,
        )
    }

    pub(super) fn validate_integration_profile_target(
        &self,
        kernel: &KernelDocument,
        instance: &CaseInstance,
    ) -> Result<(), RuntimeError> {
        let Some(profile) = self.integration_profile.as_ref() else {
            return Ok(());
        };

        if profile.target_workflow.url != instance.definition_url {
            return Err(RuntimeError::Integration(format!(
                "integration profile targets '{}' but instance uses '{}'",
                profile.target_workflow.url, instance.definition_url
            )));
        }

        if let Some(compatible_versions) = profile.target_workflow.compatible_versions.as_deref() {
            let requested_version =
                Version::parse(&instance.definition_version).map_err(|error| {
                    RuntimeError::Integration(format!(
                        "instance definition version '{}' is not valid semver: {error}",
                        instance.definition_version
                    ))
                })?;
            let normalized_versions = normalize_semver_range_expression(compatible_versions);
            let version_req = VersionReq::parse(&normalized_versions).map_err(|error| {
                RuntimeError::Integration(format!(
                    "integration profile compatibleVersions '{}' is not valid semver: {error}",
                    compatible_versions
                ))
            })?;
            if !version_req.matches(&requested_version) {
                return Err(RuntimeError::Integration(format!(
                    "integration profile compatibleVersions '{}' do not include instance version '{}'",
                    compatible_versions, instance.definition_version
                )));
            }
        }

        if kernel.url.as_deref() != Some(instance.definition_url.as_str()) {
            return Err(RuntimeError::Integration(format!(
                "kernel document url '{}' does not match instance definition url '{}'",
                kernel.url.as_deref().unwrap_or_default(),
                instance.definition_url
            )));
        }

        Ok(())
    }

    pub(super) fn stage_pending_tasks_for_presentation(
        &mut self,
        record: &mut RuntimeRecord,
        now_iso: &str,
    ) -> Result<(Vec<FormspecTaskContext>, Vec<ProvenanceRecord>), RuntimeError> {
        let mut pending_presentations = Vec::new();
        let mut provenance = Vec::new();

        for task in &mut record.instance.active_tasks {
            let Some(context) = task.context.as_ref() else {
                continue;
            };
            if task.binding.as_deref() != Some("formspec") {
                continue;
            }
            if task.status != ActiveTaskStatus::Created {
                continue;
            }

            task.status = ActiveTaskStatus::Assigned;
            task.updated_at = now_iso.to_string();
            pending_presentations.push(context.clone());
            provenance.push(ProvenanceRecord::task_lifecycle(
                ProvenanceKind::TaskPresented,
                &task.task_id,
                task.assigned_actor.as_deref(),
                Some(serde_json::json!({
                    "definitionUrl": task.definition_url,
                    "definitionVersion": task.definition_version,
                })),
            ));
        }

        Ok((pending_presentations, provenance))
    }

    pub(super) fn deliver_pending_presentations(
        &mut self,
        contexts: &[FormspecTaskContext],
    ) -> Result<(), RuntimeError> {
        for context in contexts {
            self.presenter.present_task(context)?;
        }
        Ok(())
    }

    pub(super) fn create_active_task(
        &mut self,
        kernel: &KernelDocument,
        record: &mut RuntimeRecord,
        observed: &ObservedAction,
        now_iso: &str,
    ) -> Result<ActiveTask, RuntimeError> {
        let action = &observed.action;
        let task_ref = action.task_ref.clone().ok_or_else(|| {
            RuntimeError::MissingMetadata("createTask missing taskRef".to_string())
        })?;
        let task_sequence = record.instance.next_task_sequence + 1;
        record.instance.next_task_sequence = task_sequence;
        let task_id = make_task_id(&record.instance.instance_id, task_sequence, &task_ref);

        let mut task = ActiveTask {
            task_id,
            task_ref,
            status: ActiveTaskStatus::Created,
            assigned_actor: action.assign_to.clone(),
            contract_ref: action.contract_ref.clone(),
            binding: None,
            definition_url: None,
            definition_version: None,
            prefill_mapping_ref: action.prefill_mapping_ref.clone(),
            response_mapping_ref: action.response_mapping_ref.clone(),
            deadline: None,
            impact_level: kernel.impact_level,
            context: None,
            last_validation_outcome: None,
            created_at: now_iso.to_string(),
            updated_at: now_iso.to_string(),
            extensions: Default::default(),
        };

        append_signature_task_extensions(&mut task, &action.extensions);

        if let Some(completion_event) = &action.completion_event {
            task.extensions.insert(
                COMPLETION_EVENT_EXTENSION_KEY.to_string(),
                serde_json::Value::String(completion_event.clone()),
            );
        }
        if let Some(failure_event) = &action.failure_event {
            task.extensions.insert(
                FAILURE_EVENT_EXTENSION_KEY.to_string(),
                serde_json::Value::String(failure_event.clone()),
            );
        }

        if let Some(contract_key) = &task.contract_ref {
            let contract = kernel
                .contracts
                .get(contract_key)
                .ok_or_else(|| RuntimeError::ContractNotFound(contract_key.clone()))?;
            task.binding = Some(contract.binding.clone());
            task.definition_url = Some(contract.reference.clone());
            task.definition_version = Some(kernel.version.clone().ok_or_else(|| {
                RuntimeError::MissingMetadata("kernel version required".to_string())
            })?);
            if task.prefill_mapping_ref.is_none() {
                task.prefill_mapping_ref = contract.prefill_mapping_ref.clone();
            }
            if task.response_mapping_ref.is_none() {
                task.response_mapping_ref = contract.response_mapping_ref.clone();
            }

            if contract.binding == "formspec" {
                let assigned_actor = task.assigned_actor.clone().ok_or_else(|| {
                    RuntimeError::MissingMetadata(
                        "formspec task requires assigned actor".to_string(),
                    )
                })?;
                let adapter = self
                    .bindings
                    .get(&contract.binding)
                    .ok_or_else(|| RuntimeError::UnsupportedBinding(contract.binding.clone()))?;
                let prepared = adapter.prepare_task(&task, &record.instance.case_state)?;
                task.context = Some(FormspecTaskContext {
                    task_id: task.task_id.clone(),
                    instance_id: record.instance.instance_id.clone(),
                    contract_ref: contract_key.clone(),
                    definition_url: task.definition_url.clone().unwrap_or_default(),
                    definition_version: task.definition_version.clone().unwrap_or_default(),
                    binding: contract.binding.clone(),
                    assigned_actor,
                    prefill_data: prepared.prefill_data,
                    prefill_mapping_ref: task.prefill_mapping_ref.clone(),
                    response_mapping_ref: task.response_mapping_ref.clone(),
                    deadline: task.deadline.clone(),
                    impact_level: task.impact_level.map(impact_level_label),
                    extensions: task.extensions.clone(),
                });
            } else {
                return Err(RuntimeError::UnsupportedBinding(contract.binding.clone()));
            }
        }

        Ok(task)
    }
}
