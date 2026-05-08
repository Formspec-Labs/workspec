// Rust guideline compliant 2026-02-21

//! Task command handling for the reference runtime.
//!
//! This module contains task-oriented durable commands and the helper
//! functions they depend on. Keeping them separate from event-drain logic
//! reduces the size of `runtime.rs` without changing the current adapter
//! behavior.

use crate::binding::SubmissionValidation;
use crate::milestones::evaluate_milestones;
use crate::store::{
    ReplayKey, ReplayOperation, ReplayValue, RuntimeRecord, TaskArtifact, TaskArtifactKind,
};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use wos_core::instance::{ActiveTask, CaseInstance, PendingEvent};
use wos_core::model::governance::DelegationScope;
use wos_core::provenance::{ProvenanceKind, ProvenanceRecord, SignatureAdmissionFailedInput};
use wos_core::traits::AccessControl;

use super::{
    AdmissionOutcome, COMPLETION_EVENT_EXTENSION_KEY, FAILURE_EVENT_EXTENSION_KEY,
    PersistDraftResult, RuntimeError, TaskSubmissionResult, WosRuntime, contract_validation_record,
    format_timestamp, impact_level_label, merge_case_state, populate_provenance_record_fields,
    stamp_provenance,
};

impl WosRuntime {
    /// Persists a task draft artifact.
    ///
    /// # Errors
    /// Returns an error when authorization, task lookup, validation, or
    /// persistence fails.
    pub fn persist_task_draft(
        &mut self,
        task_id: &str,
        response: serde_json::Value,
        actor_id: &str,
        idempotency_token: Option<&str>,
    ) -> Result<PersistDraftResult, RuntimeError> {
        let now_ms = self.clock.now_ms();
        let now_iso = format_timestamp(now_ms)?;
        let mut record = self.load_record_for_task_id(task_id)?;
        if let Some(token) = idempotency_token {
            let replay_key = ReplayKey {
                operation: ReplayOperation::PersistDraft,
                task_id: task_id.to_string(),
                actor_id: actor_id.to_string(),
                token: token.to_string(),
            };
            if let Some(ReplayValue::Draft(result)) = record.replay_entries.get(&replay_key) {
                return Ok(result.clone());
            }
        }
        let task_index = find_task_index(&record, task_id)
            .ok_or_else(|| RuntimeError::TaskNotFound(task_id.to_string()))?;

        let task = record.instance.active_tasks[task_index].clone();
        authorize_actor(&*self.access_control, &task, actor_id)?;
        let status = response
            .get("status")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| RuntimeError::InvalidResponseStatus("missing draft status".to_string()))?
            .to_string();
        if !matches!(status.as_str(), "in-progress" | "amended" | "stopped") {
            return Err(RuntimeError::InvalidResponseStatus(status));
        }

        let artifact = build_artifact(
            &record,
            task_id,
            TaskArtifactKind::Draft,
            response,
            actor_id,
            &now_iso,
        );
        let result = PersistDraftResult {
            artifact_id: artifact.artifact_id.clone(),
        };
        record
            .artifacts
            .insert(artifact.artifact_id.clone(), artifact.clone());
        let mut provenance = ProvenanceRecord::task_lifecycle(
            ProvenanceKind::TaskDraftPersisted,
            task_id,
            Some(actor_id),
            Some(serde_json::json!({
                "artifactId": artifact.artifact_id,
                "status": status,
            })),
        );
        provenance.timestamp = now_iso.clone();
        let kernel = self.resolver.resolve_kernel(
            &record.instance.definition_url,
            &record.instance.definition_version,
        )?;
        populate_provenance_record_fields(
            std::slice::from_mut(&mut provenance),
            &kernel,
            &record.instance.definition_version,
        );
        record.instance.provenance_position += 1;
        record.provenance_log.push(provenance);
        record.instance.updated_at = now_iso;

        if let Some(token) = idempotency_token {
            record.replay_entries.insert(
                ReplayKey {
                    operation: ReplayOperation::PersistDraft,
                    task_id: task_id.to_string(),
                    actor_id: actor_id.to_string(),
                    token: token.to_string(),
                },
                ReplayValue::Draft(result.clone()),
            );
        }

        self.store.save_record(record)?;
        Ok(result)
    }

    /// Records a task dismissal.
    ///
    /// # Errors
    /// Returns an error when the task cannot be found, presenter delivery
    /// fails, or persistence fails.
    pub fn dismiss_task(&mut self, task_id: &str, reason: &str) -> Result<(), RuntimeError> {
        let now_iso = format_timestamp(self.clock.now_ms())?;
        let mut record = self.load_record_for_task_id(task_id)?;
        let task_index = find_task_index(&record, task_id)
            .ok_or_else(|| RuntimeError::TaskNotFound(task_id.to_string()))?;
        let task = &record.instance.active_tasks[task_index];
        if task.context.is_some() {
            self.presenter.dismiss_task(task_id, reason)?;
        }

        record.instance.provenance_position += 1;
        let mut dismissal = ProvenanceRecord::task_lifecycle(
            ProvenanceKind::TaskDismissed,
            task_id,
            task.assigned_actor.as_deref(),
            Some(serde_json::json!({ "reason": reason })),
        );
        dismissal.timestamp = now_iso.clone();
        let kernel = self.resolver.resolve_kernel(
            &record.instance.definition_url,
            &record.instance.definition_version,
        )?;
        populate_provenance_record_fields(
            std::slice::from_mut(&mut dismissal),
            &kernel,
            &record.instance.definition_version,
        );
        record.provenance_log.push(dismissal);
        record.instance.updated_at = now_iso;
        self.store.save_record(record)?;
        Ok(())
    }

    /// Submits a completed task response.
    ///
    /// # Errors
    /// Returns an error when authorization, resolution, validation,
    /// evaluation, or persistence fails.
    pub fn submit_task_response(
        &mut self,
        task_id: &str,
        response: serde_json::Value,
        actor_id: &str,
        idempotency_token: Option<&str>,
    ) -> Result<TaskSubmissionResult, RuntimeError> {
        let now_ms = self.clock.now_ms();
        let now_iso = format_timestamp(now_ms)?;
        let mut record = self.load_record_for_task_id(task_id)?;
        if let Some(token) = idempotency_token {
            let replay_key = ReplayKey {
                operation: ReplayOperation::SubmitTaskResponse,
                task_id: task_id.to_string(),
                actor_id: actor_id.to_string(),
                token: token.to_string(),
            };
            if let Some(ReplayValue::Submission(result)) = record.replay_entries.get(&replay_key) {
                return Ok(result.clone());
            }
        }
        let task_index = find_task_index(&record, task_id)
            .ok_or_else(|| RuntimeError::TaskNotFound(task_id.to_string()))?;

        let task = record.instance.active_tasks[task_index].clone();
        authorize_actor(&*self.access_control, &task, actor_id)?;
        let status = response
            .get("status")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                RuntimeError::InvalidResponseStatus("missing response status".to_string())
            })?
            .to_string();
        if status != "completed" {
            if let Some(result) = self.handle_signature_non_completion(
                &mut record,
                task_index,
                &response,
                actor_id,
                &now_iso,
                &status,
            )? {
                if let Some(token) = idempotency_token {
                    record.replay_entries.insert(
                        ReplayKey {
                            operation: ReplayOperation::SubmitTaskResponse,
                            task_id: task_id.to_string(),
                            actor_id: actor_id.to_string(),
                            token: token.to_string(),
                        },
                        ReplayValue::Submission(result.clone()),
                    );
                }
                self.store.save_record(record)?;
                return Ok(result);
            }
            let result = TaskSubmissionResult::Rejected {
                code: "taskResponseStatusNotCompleted".to_string(),
            };
            self.record_submission_rejection(
                &mut record,
                task_id,
                actor_id,
                "taskResponseStatusNotCompleted",
                &now_iso,
                idempotency_token,
                result.clone(),
            )?;
            return Ok(result);
        }

        let binding = task
            .binding
            .as_deref()
            .ok_or_else(|| RuntimeError::UnsupportedBinding("task has no binding".to_string()))?;
        let adapter = self
            .bindings
            .get(binding)
            .ok_or_else(|| RuntimeError::UnsupportedBinding(binding.to_string()))?;
        let validation = adapter.validate_submission(&task, &response)?;
        let mut provenance = vec![ProvenanceRecord::task_lifecycle(
            ProvenanceKind::TaskResponseSubmitted,
            task_id,
            Some(actor_id),
            None,
        )];
        provenance.push(contract_validation_record(
            task_id,
            actor_id,
            &response,
            &validation,
        ));

        if !validation_passed(&validation) {
            let emitted_event = remove_task_with_event(
                &mut record.instance,
                task_index,
                FAILURE_EVENT_EXTENSION_KEY,
                actor_id,
                &now_iso,
            );
            provenance.push(ProvenanceRecord::task_lifecycle(
                ProvenanceKind::TaskFailed,
                task_id,
                Some(actor_id),
                Some(serde_json::json!({
                    "code": "validationFailed",
                    "validationOutcome": validation.validation_outcome,
                })),
            ));
            let kernel = self.resolver.resolve_kernel(
                &record.instance.definition_url,
                &record.instance.definition_version,
            )?;
            populate_provenance_record_fields(
                &mut provenance,
                &kernel,
                &record.instance.definition_version,
            );
            stamp_provenance(&mut provenance, &now_iso);
            record.instance.provenance_position += provenance.len() as u64;
            record.provenance_log.extend(provenance);
            record.instance.updated_at = now_iso;
            let result = TaskSubmissionResult::Failed {
                code: "validationFailed".to_string(),
                emitted_event,
            };
            if let Some(token) = idempotency_token {
                record.replay_entries.insert(
                    ReplayKey {
                        operation: ReplayOperation::SubmitTaskResponse,
                        task_id: task_id.to_string(),
                        actor_id: actor_id.to_string(),
                        token: token.to_string(),
                    },
                    ReplayValue::Submission(result.clone()),
                );
            }
            self.store.save_record(record)?;
            return Ok(result);
        }

        let signature_evidence = if Self::is_signature_task(&task) {
            adapter.signature_evidence(&task, &response)?
        } else {
            None
        };
        let signature_outcome = self.signature_affirmation_for_submission(
            &record,
            &task,
            &response,
            signature_evidence.as_deref(),
            actor_id,
            &now_iso,
        )?;
        if let Some(AdmissionOutcome::AdmissionFailed(failed)) = signature_outcome.as_ref() {
            let reason = serde_json::to_value(&failed.reason)
                .ok()
                .and_then(|value| value.as_str().map(str::to_string))
                .unwrap_or_else(|| "signature_admission_failed".to_string());
            provenance.push(ProvenanceRecord::signature_admission_failed(
                SignatureAdmissionFailedInput {
                    reason: &reason,
                    response_id: &failed.evidence_bindings.response_id,
                    signed_payload_digest: &failed.evidence_bindings.signed_payload_digest,
                    signature_id: &failed.evidence_bindings.signature_id,
                    signing_intent: &failed.evidence_bindings.signing_intent,
                    signer_id: failed.signer_id.as_deref(),
                    signer_authority: failed.signer_authority.clone(),
                    failure_context: None,
                    emitted_at: &failed.emitted_at,
                },
            ));
            provenance.push(ProvenanceRecord::task_lifecycle(
                ProvenanceKind::TaskFailed,
                &task.task_id,
                Some(actor_id),
                Some(serde_json::json!({
                    "admissionOutcome": "admissionFailed",
                    "reason": failed.reason,
                    "evidenceBindings": failed.evidence_bindings,
                    "signerId": failed.signer_id,
                    "signerAuthority": failed.signer_authority,
                    "emittedAt": failed.emitted_at,
                })),
            ));
            let kernel = self.resolver.resolve_kernel(
                &record.instance.definition_url,
                &record.instance.definition_version,
            )?;
            populate_provenance_record_fields(
                &mut provenance,
                &kernel,
                &record.instance.definition_version,
            );
            stamp_provenance(&mut provenance, &now_iso);
            record.instance.provenance_position += provenance.len() as u64;
            record.provenance_log.extend(provenance);
            record.instance.updated_at = now_iso;
            let result = TaskSubmissionResult::Failed {
                code: "signatureAdmissionFailed".to_string(),
                emitted_event: None,
            };
            if let Some(token) = idempotency_token {
                record.replay_entries.insert(
                    ReplayKey {
                        operation: ReplayOperation::SubmitTaskResponse,
                        task_id: task_id.to_string(),
                        actor_id: actor_id.to_string(),
                        token: token.to_string(),
                    },
                    ReplayValue::Submission(result.clone()),
                );
            }
            self.store.save_record(record)?;
            return Ok(result);
        }

        let accepted_artifact = build_artifact(
            &record,
            task_id,
            TaskArtifactKind::Accepted,
            response.clone(),
            actor_id,
            &now_iso,
        );
        record.artifacts.insert(
            accepted_artifact.artifact_id.clone(),
            accepted_artifact.clone(),
        );
        let mutation = adapter.compute_case_mutation(&task, &response)?;
        let case_mutated = mutation
            .as_ref()
            .is_some_and(|bundle| !bundle.field_updates.is_empty());
        if let Some(bundle) = mutation
            && !bundle.field_updates.is_empty()
        {
            merge_case_state(
                &mut record.instance.case_state,
                &serde_json::Value::Object(bundle.field_updates.clone()),
            );
            provenance.push(ProvenanceRecord::task_lifecycle(
                ProvenanceKind::DataMapping,
                task_id,
                Some(actor_id),
                Some(serde_json::json!({
                    "artifactId": accepted_artifact.artifact_id,
                    "mappingRef": task.response_mapping_ref,
                })),
            ));
        }

        let kernel = self.resolver.resolve_kernel(
            &record.instance.definition_url,
            &record.instance.definition_version,
        )?;
        let post_state = record.instance.case_state.clone();
        let milestone_records = evaluate_milestones(&kernel, &mut record.instance, &post_state);
        provenance.extend(milestone_records);

        let completion_event_key = if self.signature_flow_complete_after(&record.instance, &task)? {
            COMPLETION_EVENT_EXTENSION_KEY
        } else {
            "x-wos-runtime-no-completion-event"
        };
        let emitted_event = remove_task_with_event(
            &mut record.instance,
            task_index,
            completion_event_key,
            actor_id,
            &now_iso,
        );
        if let Some(outcome) = signature_outcome {
            match outcome {
                AdmissionOutcome::Affirmation(outcome) => {
                    // Completion `signed_at` flows from the same verified
                    // evidence that produced the `SignatureAffirmation` record
                    // (review F4): the case-ledger entry and
                    // `x-wos-signature-completions` cannot disagree for a given
                    // signature event.
                    self.record_signature_completion(
                        &mut record.instance,
                        &task,
                        &outcome.signer_id,
                        &outcome.signed_at,
                    )?;
                    provenance.push(outcome.record);
                    provenance.push(ProvenanceRecord::task_lifecycle(
                        ProvenanceKind::TaskCompleted,
                        task_id,
                        Some(actor_id),
                        Some(serde_json::json!({
                            "artifactId": accepted_artifact.artifact_id,
                            "caseMutated": case_mutated,
                        })),
                    ));
                }
                AdmissionOutcome::AdmissionFailed(_) => {
                    unreachable!("admission failures return before task removal and completion")
                }
            }
        } else {
            provenance.push(ProvenanceRecord::task_lifecycle(
                ProvenanceKind::TaskCompleted,
                task_id,
                Some(actor_id),
                Some(serde_json::json!({
                    "artifactId": accepted_artifact.artifact_id,
                    "caseMutated": case_mutated,
                })),
            ));
        }
        populate_provenance_record_fields(
            &mut provenance,
            &kernel,
            &record.instance.definition_version,
        );
        stamp_provenance(&mut provenance, &now_iso);
        record.instance.provenance_position += provenance.len() as u64;
        record.provenance_log.extend(provenance);
        record.instance.updated_at = now_iso;

        let result = TaskSubmissionResult::Completed {
            artifact_id: accepted_artifact.artifact_id,
            case_mutated,
            emitted_event,
        };
        if let Some(token) = idempotency_token {
            record.replay_entries.insert(
                ReplayKey {
                    operation: ReplayOperation::SubmitTaskResponse,
                    task_id: task_id.to_string(),
                    actor_id: actor_id.to_string(),
                    token: token.to_string(),
                },
                ReplayValue::Submission(result.clone()),
            );
        }
        self.store.save_record(record)?;
        Ok(result)
    }

    /// Loads a provenance window by cursor and limit.
    ///
    /// # Errors
    /// Returns an error when the instance cannot be found or loaded.
    pub fn load_provenance_window(
        &self,
        instance_id: &str,
        cursor: usize,
        limit: usize,
    ) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
        let record = self.store.load_record(instance_id)?;
        Ok(record
            .provenance_log
            .iter()
            .skip(cursor)
            .take(limit)
            .cloned()
            .collect())
    }

    pub(super) fn load_record_for_task_id(
        &self,
        task_id: &str,
    ) -> Result<RuntimeRecord, RuntimeError> {
        let instance_id = task_instance_id(task_id)?;
        Ok(self.store.load_record(&instance_id)?)
    }

    pub(super) fn record_submission_rejection(
        &mut self,
        record: &mut RuntimeRecord,
        task_id: &str,
        actor_id: &str,
        code: &str,
        updated_at: &str,
        idempotency_token: Option<&str>,
        result: TaskSubmissionResult,
    ) -> Result<(), RuntimeError> {
        record.instance.provenance_position += 1;
        let mut rejection = ProvenanceRecord::task_lifecycle(
            ProvenanceKind::TaskResponseRejected,
            task_id,
            Some(actor_id),
            Some(serde_json::json!({ "code": code })),
        );
        rejection.timestamp = updated_at.to_string();
        let kernel = self.resolver.resolve_kernel(
            &record.instance.definition_url,
            &record.instance.definition_version,
        )?;
        populate_provenance_record_fields(
            std::slice::from_mut(&mut rejection),
            &kernel,
            &record.instance.definition_version,
        );
        record.provenance_log.push(rejection);
        record.instance.updated_at = updated_at.to_string();
        if let Some(token) = idempotency_token {
            record.replay_entries.insert(
                ReplayKey {
                    operation: ReplayOperation::SubmitTaskResponse,
                    task_id: task_id.to_string(),
                    actor_id: actor_id.to_string(),
                    token: token.to_string(),
                },
                ReplayValue::Submission(result),
            );
        }
        self.store.save_record(record.clone())?;
        Ok(())
    }
}

fn task_instance_id(task_id: &str) -> Result<String, RuntimeError> {
    let Some(encoded_instance_id) = task_id
        .strip_prefix("wos-task:")
        .and_then(|rest| rest.split_once(':'))
        .map(|(encoded_instance_id, _)| encoded_instance_id)
    else {
        return Err(RuntimeError::TaskNotFound(task_id.to_string()));
    };

    let decoded = URL_SAFE_NO_PAD
        .decode(encoded_instance_id)
        .map_err(|_| RuntimeError::TaskNotFound(task_id.to_string()))?;
    std::str::from_utf8(&decoded)
        .map(str::to_owned)
        .map_err(|_| RuntimeError::TaskNotFound(task_id.to_string()))
}

fn find_task_index(record: &RuntimeRecord, task_id: &str) -> Option<usize> {
    record
        .instance
        .active_tasks
        .iter()
        .position(|task| task.task_id == task_id)
}

fn authorize_actor(
    access_control: &dyn AccessControl,
    task: &ActiveTask,
    actor_id: &str,
) -> Result<(), RuntimeError> {
    let assigned_actor = task
        .assigned_actor
        .as_deref()
        .ok_or_else(|| RuntimeError::Unauthorized("task has no assigned actor".to_string()))?;
    if actor_id == assigned_actor {
        return Ok(());
    }

    let mut scope = DelegationScope {
        impact_levels: Vec::new(),
        case_types: Vec::new(),
        max_dollar_threshold: None,
        conditions: None,
    };
    if let Some(impact_level) = &task.impact_level {
        scope.impact_levels.push(impact_level_label(*impact_level));
    }
    if access_control.can_delegate(assigned_actor, actor_id, &scope) {
        Ok(())
    } else {
        Err(RuntimeError::Unauthorized(actor_id.to_string()))
    }
}

fn validation_passed(validation: &SubmissionValidation) -> bool {
    let outcome = &validation.validation_outcome;
    outcome.envelope_valid && outcome.pin_match && outcome.definition_valid
}

fn build_artifact(
    record: &RuntimeRecord,
    task_id: &str,
    kind: TaskArtifactKind,
    response: serde_json::Value,
    actor_id: &str,
    recorded_at: &str,
) -> TaskArtifact {
    let kind_name = match kind {
        TaskArtifactKind::Draft => "draft",
        TaskArtifactKind::Accepted => "accepted",
    };
    let artifact_id = format!("{task_id}:{kind_name}:{}", record.artifacts.len() + 1);
    TaskArtifact {
        artifact_id,
        task_id: task_id.to_string(),
        kind,
        response,
        actor_id: actor_id.to_string(),
        recorded_at: recorded_at.to_string(),
    }
}

fn remove_task_with_event(
    instance: &mut CaseInstance,
    task_index: usize,
    extension_key: &str,
    actor_id: &str,
    timestamp: &str,
) -> Option<String> {
    let task = instance.active_tasks.remove(task_index);
    let emitted_event = task
        .extensions
        .get(extension_key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    if let Some(event) = &emitted_event {
        instance.pending_events.push(PendingEvent {
            event: event.clone(),
            actor_id: Some(actor_id.to_string()),
            data: Some(serde_json::json!({ "taskId": task.task_id })),
            timestamp: timestamp.to_string(),
            idempotency_token: None,
        });
    }
    emitted_event
}
