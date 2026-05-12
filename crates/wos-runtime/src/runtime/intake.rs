// Rust guideline compliant 2026-02-21

//! Host-side intake-acceptance command handling for the reference runtime.

use std::slice;

use serde_json::{Map, Value};
use wos_core::instance::WorkflowProcess;
use wos_core::model::kernel::{ActorKind, KernelDocument};
use wos_core::provenance::{ProvenanceKind, ProvenanceRecord};

use crate::intake::{
    IntakeAcceptanceAdapter, IntakeAcceptanceDecision, IntakeAcceptanceOutcome,
    IntakeAcceptanceRequest, IntakeCaseDefinition, IntakeCaseDisposition, IntakeCaseIntent,
    IntakeInterpretation, IntakePolicyContext, IntakeRecordStatus,
};
use crate::store::{IntakeRecord, StoreError};

use super::{
    CreateInstanceRequest, RuntimeError, WosRuntime, format_timestamp,
    populate_provenance_record_fields, stamp_provenance,
};

impl WosRuntime {
    /// Accept a binding-native intake handoff through the host-side intake seam.
    ///
    /// This command is intentionally separate from task submission. It handles
    /// boundary documents such as Formspec Intake Handoff that arrive from an
    /// external intake surface rather than from an active WOS task.
    ///
    /// # Errors
    /// Returns an error when the intake binding is unsupported or the adapter
    /// rejects the handoff.
    pub fn accept_intake_handoff(
        &mut self,
        binding: &str,
        request: IntakeAcceptanceRequest,
    ) -> Result<IntakeAcceptanceDecision, RuntimeError> {
        let adapter = self.intake_acceptors.get(binding).ok_or_else(|| {
            RuntimeError::UnsupportedBinding(format!(
                "intake acceptance binding unsupported: {binding}"
            ))
        })?;
        let interpretation = adapter.interpret_intake_handoff(&request)?;

        let persisted =
            match self.load_replayable_intake_record(binding, &interpretation, &request)? {
                Some(record) => record,
                None => {
                    let decision =
                        self.intake_policy
                            .evaluate_intake_acceptance(&IntakePolicyContext {
                                binding: binding.to_string(),
                                request: request.clone(),
                                interpretation: interpretation.clone(),
                            })?;
                    self.persist_pending_intake_record(
                        binding,
                        &interpretation,
                        &request,
                        decision,
                    )?
                }
            };

        let prepared = self.prepare_intake_record(
            binding,
            &request,
            &interpretation,
            adapter.as_ref(),
            persisted,
        )?;
        let applied = self.apply_prepared_intake_record(prepared)?;
        Ok(IntakeAcceptanceDecision {
            outcome: applied.outcome,
            provenance: applied.provenance_log,
        })
    }

    fn load_replayable_intake_record(
        &self,
        binding: &str,
        interpretation: &IntakeInterpretation,
        request: &IntakeAcceptanceRequest,
    ) -> Result<Option<IntakeRecord>, RuntimeError> {
        match self
            .store
            .load_intake_record(binding, &interpretation.intake_id)
        {
            Ok(record) => {
                if record.request != *request {
                    return Err(RuntimeError::IntakeConflict(format!(
                        "{binding}:{} request mismatch",
                        interpretation.intake_id
                    )));
                }
                Ok(Some(record))
            }
            Err(StoreError::NotFound(_)) => Ok(None),
            Err(error) => Err(RuntimeError::from(error)),
        }
    }

    fn persist_pending_intake_record(
        &mut self,
        binding: &str,
        interpretation: &IntakeInterpretation,
        request: &IntakeAcceptanceRequest,
        decision: IntakeAcceptanceDecision,
    ) -> Result<IntakeRecord, RuntimeError> {
        let now_iso = format_timestamp(self.clock.now_ms())?;
        let IntakeAcceptanceDecision {
            outcome,
            provenance,
        } = decision;
        let record = IntakeRecord {
            binding: binding.to_string(),
            intake_id: interpretation.intake_id.clone(),
            request: request.clone(),
            outcome,
            provenance_log: provenance,
            status: IntakeRecordStatus::Pending,
            recorded_at: now_iso.clone(),
            updated_at: now_iso,
        };
        self.store.create_intake_record(record.clone())?;
        Ok(record)
    }

    /// Prepare a pending intake receipt: canonicalize the durable outcome, merge
    /// provenance, and call the binding finalizer.
    ///
    /// Provenance order is intentional: entries copied from the pending record
    /// (policy-emitted) first, then the runtime intake outcome record, then rows
    /// from the binding finalizer (`finalize_intake_acceptance` on the intake adapter).
    fn prepare_intake_record(
        &mut self,
        binding: &str,
        request: &IntakeAcceptanceRequest,
        interpretation: &IntakeInterpretation,
        adapter: &dyn IntakeAcceptanceAdapter,
        mut record: IntakeRecord,
    ) -> Result<IntakeRecord, RuntimeError> {
        if matches!(
            record.status,
            IntakeRecordStatus::Prepared | IntakeRecordStatus::Applied
        ) {
            return Ok(record);
        }

        let policy_outcome = record.outcome.clone();
        let final_outcome = self.canonicalize_intake_outcome(&policy_outcome)?;
        let finalize_outcome = outcome_for_binding_finalize(&policy_outcome, &final_outcome);
        let mut provenance = record.provenance_log.clone();
        provenance.push(intake_outcome_provenance(
            binding,
            request,
            interpretation,
            &final_outcome,
        ));
        provenance.extend(adapter.finalize_intake_acceptance(request, &finalize_outcome)?);

        record.outcome = final_outcome;
        record.provenance_log = provenance;
        record.status = IntakeRecordStatus::Prepared;
        record.updated_at = format_timestamp(self.clock.now_ms())?;
        self.store.save_intake_record(record.clone())?;
        Ok(record)
    }

    fn canonicalize_intake_outcome(
        &mut self,
        outcome: &IntakeAcceptanceOutcome,
    ) -> Result<IntakeAcceptanceOutcome, RuntimeError> {
        match outcome {
            IntakeAcceptanceOutcome::Accepted { case_disposition } => match case_disposition {
                IntakeCaseDisposition::AttachToExistingCase { case_ref } => {
                    let case = self.load_record_for_case_ref(case_ref)?.instance;
                    Ok(IntakeAcceptanceOutcome::Accepted {
                        case_disposition: IntakeCaseDisposition::AttachToExistingCase {
                            case_ref: case.case_ledger_id,
                        },
                    })
                }
                IntakeCaseDisposition::CreateGovernedCase {
                    case_ref,
                    definition,
                    initial_case_state,
                } => {
                    let case = match self.load_record_for_case_ref(case_ref) {
                        Ok(existing) => {
                            self.ensure_matching_case_definition(
                                &existing.instance,
                                definition,
                                case_ref,
                            )?;
                            existing.instance
                        }
                        Err(RuntimeError::Store(StoreError::NotFound(_))) => self
                            .create_instance_bound_to_case(
                                CreateInstanceRequest {
                                    process_id: String::new(),
                                    tenant: None,
                                    definition_url: definition.definition_url.clone(),
                                    definition_version: definition.definition_version.clone(),
                                    initial_case_state: initial_case_state.clone(),
                                },
                                case_ref.clone(),
                            )?,
                        Err(error) => return Err(error),
                    };
                    Ok(IntakeAcceptanceOutcome::Accepted {
                        case_disposition: IntakeCaseDisposition::CreateGovernedCase {
                            case_ref: case.case_ledger_id,
                            definition: definition.clone(),
                            initial_case_state: initial_case_state.clone(),
                        },
                    })
                }
            },
            IntakeAcceptanceOutcome::Rejected { code } => {
                Ok(IntakeAcceptanceOutcome::Rejected { code: code.clone() })
            }
            IntakeAcceptanceOutcome::Deferred { code } => {
                Ok(IntakeAcceptanceOutcome::Deferred { code: code.clone() })
            }
        }
    }

    fn ensure_matching_case_definition(
        &self,
        case: &wos_core::instance::WorkflowProcess,
        definition: &IntakeCaseDefinition,
        requested_case_ref: &str,
    ) -> Result<(), RuntimeError> {
        if case.definition_url != definition.definition_url
            || case.definition_version != definition.definition_version
        {
            return Err(RuntimeError::IntakeConflict(format!(
                "existing case for {requested_case_ref} does not match requested definition {}@{}",
                definition.definition_url, definition.definition_version
            )));
        }
        Ok(())
    }

    fn apply_prepared_intake_record(
        &mut self,
        mut record: IntakeRecord,
    ) -> Result<IntakeRecord, RuntimeError> {
        if record.status == IntakeRecordStatus::Applied {
            return Ok(record);
        }

        record.provenance_log = match &record.outcome {
            IntakeAcceptanceOutcome::Accepted { case_disposition } => match case_disposition {
                IntakeCaseDisposition::AttachToExistingCase { case_ref }
                | IntakeCaseDisposition::CreateGovernedCase { case_ref, .. } => {
                    self.append_intake_provenance_to_case(case_ref, &record.provenance_log)?
                }
            },
            IntakeAcceptanceOutcome::Rejected { .. } | IntakeAcceptanceOutcome::Deferred { .. } => {
                self.populate_detached_intake_provenance(&record.request, &record.provenance_log)?
            }
        };

        record.status = IntakeRecordStatus::Applied;
        record.updated_at = format_timestamp(self.clock.now_ms())?;
        self.store.save_intake_record(record.clone())?;
        Ok(record)
    }

    fn append_intake_provenance_to_case(
        &mut self,
        case_ref: &str,
        provenance: &[ProvenanceRecord],
    ) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
        let mut record = self.load_record_for_case_ref(case_ref)?;
        let now_iso = format_timestamp(self.clock.now_ms())?;
        let kernel = self.resolver.resolve_kernel(
            &record.instance.definition_url,
            &record.instance.definition_version,
        )?;

        let mut appended = Vec::new();
        let mut final_provenance = Vec::with_capacity(provenance.len());
        let lifecycle_state = record
            .instance
            .configuration
            .first()
            .cloned()
            .unwrap_or_else(|| kernel.lifecycle.initial_state.clone());
        for original in provenance {
            if let Some(existing) = record
                .provenance_log
                .iter()
                .find(|entry| entry.id == original.id)
            {
                final_provenance.push(existing.clone());
                continue;
            }

            let mut prepared = original.clone();
            stamp_intake_provenance_fields(
                slice::from_mut(&mut prepared),
                &kernel,
                &lifecycle_state,
            );
            stamp_case_boundary_identity(slice::from_mut(&mut prepared), &record.instance);
            populate_provenance_record_fields(
                slice::from_mut(&mut prepared),
                &kernel,
                &record.instance.definition_version,
            );
            stamp_provenance(slice::from_mut(&mut prepared), &now_iso);
            appended.push(prepared.clone());
            final_provenance.push(prepared);
        }

        if !appended.is_empty() {
            record.instance.provenance_position += appended.len() as u64;
            record.instance.updated_at = now_iso;
            record.provenance_log.extend(appended);
            self.store.save_record(record)?;
        }

        Ok(final_provenance)
    }

    fn populate_detached_intake_provenance(
        &self,
        request: &IntakeAcceptanceRequest,
        provenance: &[ProvenanceRecord],
    ) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
        let (kernel, definition_version) = self.intake_provenance_context(request)?;
        let lifecycle_state = if let Some(case_ref) = request.governed_case_ref.as_deref() {
            match self.load_record_for_case_ref(case_ref) {
                Ok(record) => record
                    .instance
                    .configuration
                    .first()
                    .cloned()
                    .unwrap_or_else(|| kernel.lifecycle.initial_state.clone()),
                Err(RuntimeError::Store(StoreError::NotFound(_))) => {
                    kernel.lifecycle.initial_state.clone()
                }
                Err(error) => return Err(error),
            }
        } else {
            kernel.lifecycle.initial_state.clone()
        };
        let now_iso = format_timestamp(self.clock.now_ms())?;
        let mut detached = provenance.to_vec();
        stamp_intake_provenance_fields(&mut detached, &kernel, &lifecycle_state);
        populate_provenance_record_fields(&mut detached, &kernel, &definition_version);
        stamp_provenance(&mut detached, &now_iso);
        Ok(detached)
    }

    fn intake_provenance_context(
        &self,
        request: &IntakeAcceptanceRequest,
    ) -> Result<(KernelDocument, String), RuntimeError> {
        if let Some(case_ref) = request.governed_case_ref.as_deref() {
            match self.load_record_for_case_ref(case_ref) {
                Ok(record) => {
                    let kernel = self.resolver.resolve_kernel(
                        &record.instance.definition_url,
                        &record.instance.definition_version,
                    )?;
                    return Ok((kernel, record.instance.definition_version));
                }
                Err(RuntimeError::Store(StoreError::NotFound(_))) => {}
                Err(error) => return Err(error),
            }
        }

        if let Some(definition) = &request.governed_case_definition {
            let kernel = self
                .resolver
                .resolve_kernel(&definition.definition_url, &definition.definition_version)?;
            return Ok((kernel, definition.definition_version.clone()));
        }

        Err(RuntimeError::MissingMetadata(
            "governedCaseDefinition or existing governedCaseRef required for detached intake provenance"
                .to_string(),
        ))
    }

    fn load_record_for_case_ref(
        &self,
        case_ref: &str,
    ) -> Result<crate::store::RuntimeRecord, RuntimeError> {
        match self.store.load_record(case_ref) {
            Ok(record) => Ok(record),
            Err(StoreError::NotFound(_)) => {
                let normalized = WorkflowProcess::extract_urn_type_id(case_ref).unwrap_or(case_ref);
                self.store
                    .load_record_by_case_ledger_id(normalized)
                    .map_err(RuntimeError::from)
            }
            Err(error) => Err(RuntimeError::from(error)),
        }
    }
}

fn stamp_intake_provenance_fields(
    records: &mut [ProvenanceRecord],
    kernel: &KernelDocument,
    lifecycle_state: &str,
) {
    for record in records {
        if !matches!(
            record.record_kind,
            ProvenanceKind::IntakeAccepted
                | ProvenanceKind::IntakeRejected
                | ProvenanceKind::IntakeDeferred
                | ProvenanceKind::CaseCreated
        ) {
            continue;
        }

        if record.actor_id.is_none() {
            record.actor_id = Some("system".to_string());
        }

        if record.actor_type.is_none()
            && let Some(actor_id) = record.actor_id.as_deref()
            && let Some(actor) = kernel.actors.iter().find(|actor| actor.id == actor_id)
        {
            record.actor_type = Some(match actor.kind {
                ActorKind::Human => "human".to_string(),
                ActorKind::System => "system".to_string(),
                ActorKind::Agent => "agent".to_string(),
            });
        }

        if record.actor_type.is_none() {
            record.actor_type = Some("system".to_string());
        }

        if record.lifecycle_state.is_none() {
            record.lifecycle_state = Some(lifecycle_state.to_string());
        }
    }
}

fn stamp_case_boundary_identity(records: &mut [ProvenanceRecord], instance: &WorkflowProcess) {
    let case_ledger_id = &instance.case_ledger_id;
    if !WorkflowProcess::is_case_id(case_ledger_id) {
        return;
    }

    for record in records {
        if !matches!(
            record.record_kind,
            ProvenanceKind::CaseCreated | ProvenanceKind::IntakeAccepted
        ) {
            continue;
        }

        if record.outputs.is_empty() {
            record.outputs.push(case_ledger_id.to_string());
        }

        let Some(data) = record
            .data
            .as_mut()
            .and_then(serde_json::Value::as_object_mut)
        else {
            continue;
        };
        data.entry("caseLedgerId".to_string())
            .or_insert_with(|| serde_json::Value::String(case_ledger_id.to_string()));
    }
}

/// Shape the outcome passed to the intake adapter's `finalize_intake_acceptance`.
///
/// Workflow-initiated Formspec finalization compares the accepted attach
/// disposition's `caseRef` to the handoff document's `caseRef` string (Core
/// intake-handoff semantics). The runtime canonicalizes attach targets to the
/// governed-case id for the persisted intake record outcome and case
/// provenance; this helper keeps the **policy / handoff** attach string for the
/// adapter call so finalization stays aligned with the emitted handoff.
fn outcome_for_binding_finalize(
    policy_outcome: &IntakeAcceptanceOutcome,
    canonical_outcome: &IntakeAcceptanceOutcome,
) -> IntakeAcceptanceOutcome {
    match (policy_outcome, canonical_outcome) {
        (
            IntakeAcceptanceOutcome::Accepted {
                case_disposition: policy_disp,
            },
            IntakeAcceptanceOutcome::Accepted {
                case_disposition: IntakeCaseDisposition::AttachToExistingCase { .. },
            },
        ) => {
            if let IntakeCaseDisposition::AttachToExistingCase { case_ref } = policy_disp {
                return IntakeAcceptanceOutcome::Accepted {
                    case_disposition: IntakeCaseDisposition::AttachToExistingCase {
                        case_ref: case_ref.clone(),
                    },
                };
            }
            canonical_outcome.clone()
        }
        _ => canonical_outcome.clone(),
    }
}

fn intake_outcome_provenance(
    binding: &str,
    request: &IntakeAcceptanceRequest,
    interpretation: &IntakeInterpretation,
    outcome: &IntakeAcceptanceOutcome,
) -> ProvenanceRecord {
    let mut data = Map::from_iter([
        ("binding".to_string(), Value::String(binding.to_string())),
        (
            "intakeId".to_string(),
            Value::String(interpretation.intake_id.clone()),
        ),
        (
            "caseIntent".to_string(),
            Value::String(case_intent_label(&interpretation.case_intent).to_string()),
        ),
    ]);

    let (record_kind, event, outputs) = match outcome {
        IntakeAcceptanceOutcome::Accepted { case_disposition } => {
            let (case_ref, disposition_label) = match case_disposition {
                IntakeCaseDisposition::AttachToExistingCase { case_ref } => {
                    (case_ref.clone(), "attachToExistingCase")
                }
                IntakeCaseDisposition::CreateGovernedCase {
                    case_ref,
                    definition,
                    ..
                } => {
                    data.insert(
                        "definitionUrl".to_string(),
                        Value::String(definition.definition_url.clone()),
                    );
                    data.insert(
                        "definitionVersion".to_string(),
                        Value::String(definition.definition_version.clone()),
                    );
                    (case_ref.clone(), "createGovernedCase")
                }
            };
            data.insert(
                "caseDisposition".to_string(),
                Value::String(disposition_label.to_string()),
            );
            data.insert("caseRef".to_string(), Value::String(case_ref.clone()));
            data.insert("caseLedgerId".to_string(), Value::String(case_ref.clone()));
            (
                ProvenanceKind::IntakeAccepted,
                "wos.kernel.intake_accepted",
                vec![case_ref],
            )
        }
        IntakeAcceptanceOutcome::Rejected { code } => {
            data.insert("code".to_string(), Value::String(code.clone()));
            (
                ProvenanceKind::IntakeRejected,
                "wos.kernel.intake_rejected",
                Vec::new(),
            )
        }
        IntakeAcceptanceOutcome::Deferred { code } => {
            data.insert("code".to_string(), Value::String(code.clone()));
            (
                ProvenanceKind::IntakeDeferred,
                "wos.kernel.intake_deferred",
                Vec::new(),
            )
        }
    };

    ProvenanceRecord {
        id: ProvenanceRecord::mint_id(),
        record_kind,
        timestamp: String::new(),
        actor_id: request.actor_id.clone(),
        from_state: None,
        to_state: None,
        event: Some(event.to_string()),
        data: Some(Value::Object(data)),
        audit_layer: None,
        actor_type: None,
        lifecycle_state: None,
        definition_version: None,
        inputs: vec![interpretation.intake_id.clone()],
        outputs,
        input_digest: None,
        output_digest: None,
        canonical_event_hash: None,
        transition_tags: Vec::new(),
        case_file_snapshot: None,
        outcome: None,
    }
}

fn case_intent_label(intent: &IntakeCaseIntent) -> &'static str {
    match intent {
        IntakeCaseIntent::AttachToExistingCase { .. } => "attachToExistingCase",
        IntakeCaseIntent::RequestGovernedCaseCreation => "requestGovernedCaseCreation",
    }
}
