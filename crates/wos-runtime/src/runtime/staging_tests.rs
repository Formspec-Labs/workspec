// Rust guideline compliant 2026-02-21

// Native preparation tests use actual commands and retained native records.

use super::*;
use crate::store::{InMemoryStore, IntakeRecord, RuntimeRecord, RuntimeStore, StoreError};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, Default)]
struct OperationalStoreProbe(Arc<AtomicUsize>);
impl OperationalStoreProbe {
    fn refused<T>(&self) -> Result<T, StoreError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(StoreError::Failed(
            "operational store must not be called during preparation".into(),
        ))
    }
}
impl RuntimeStore for OperationalStoreProbe {
    fn create_record(&mut self, _: RuntimeRecord) -> Result<(), StoreError> {
        self.refused()
    }
    fn load_record(&self, _: &str) -> Result<RuntimeRecord, StoreError> {
        self.refused()
    }
    fn processes_for_case(&self, _: &str) -> Vec<String> {
        self.0.fetch_add(1, Ordering::SeqCst);
        vec![]
    }
    fn append_provenance_for_case(
        &mut self,
        _: &str,
        _: &str,
        _: ProvenanceRecord,
    ) -> Result<(), StoreError> {
        self.refused()
    }
    fn save_record(&mut self, _: RuntimeRecord) -> Result<(), StoreError> {
        self.refused()
    }
    fn create_intake_record(&mut self, _: IntakeRecord) -> Result<(), StoreError> {
        self.refused()
    }
    fn load_intake_record(&self, _: &str, _: &str) -> Result<IntakeRecord, StoreError> {
        self.refused()
    }
    fn save_intake_record(&mut self, _: IntakeRecord) -> Result<(), StoreError> {
        self.refused()
    }
}

fn stage_policy() -> NativePreparationPolicy {
    let mut policy = NativePreparationPolicy::default();
    for dependency in [
        NativeReadOnlyDependency::Clock,
        NativeReadOnlyDependency::Resolver,
        NativeReadOnlyDependency::AccessControl,
        NativeReadOnlyDependency::ContractValidator,
        NativeReadOnlyDependency::IntakePolicy,
        NativeReadOnlyDependency::CompanionPolicy,
        NativeReadOnlyDependency::PostureResolver,
    ] {
        policy
            .read_only_dependency_ids
            .insert(dependency, "actual-qualified-fixture".into());
    }
    for operation in [
        NativeTaskBindingOperation::Prepare,
        NativeTaskBindingOperation::Validate,
        NativeTaskBindingOperation::Map,
        NativeTaskBindingOperation::SignatureEvidence,
    ] {
        policy
            .read_only_task_binding_ids
            .entry(operation)
            .or_default()
            .insert("formspec".into());
    }
    for operation in [
        NativeIntakeBindingOperation::Interpret,
        NativeIntakeBindingOperation::Finalize,
    ] {
        policy
            .read_only_intake_binding_ids
            .entry(operation)
            .or_default()
            .insert("formspec".into());
    }
    policy
}

fn stage_kernel() -> KernelDocument {
    serde_json::from_value(serde_json::json!({
        "$wosWorkflow":"1.0", "url":"urn:test:native-stage", "version":"1.0.0",
        "contracts":{"review":{"binding":"formspec","ref":"urn:formspec:stage-review","definitionVersion":"1.0.0"}},
        "lifecycle":{"initialState":"open","states":{"open":{"type":"atomic", "onEntry":[
            {"action":"createTask","taskRef":"review","assignTo":"reviewer","contractRef":"review"}
        ]}}}
    }))
    .unwrap()
}
fn stage_allocation() -> crate::ProcessAllocation {
    crate::ProcessAllocation {
        process_id: test_process_id("staged").into(),
        case_ledger_id: test_case_ledger_id("staged").into(),
        tenant: "staged".into(),
    }
}
fn stage_create_command() -> NativeRuntimeCommand {
    NativeRuntimeCommand::CreateProcess {
        request: CreateProcessRequest {
            process_id: test_process_id("staged").into(),
            tenant: Some("staged".into()),
            definition_url: "urn:test:native-stage".into(),
            definition_version: "1.0.0".into(),
            initial_case_state: None,
        },
        allocation: stage_allocation(),
        actor_bindings: None,
    }
}
fn stage_empty_snapshot() -> NativeRuntimeSnapshot {
    NativeRuntimeSnapshot {
        process_records: vec![],
        intake_records: vec![],
        complete_case_membership: vec![CaseProcessMembership {
            case_ledger_id: test_case_ledger_id("staged").into(),
            process_ids: vec![],
        }],
    }
}
fn stage_created_record(prepared: &PreparedNativeCommand) -> RuntimeRecord {
    match &prepared.process_changes()[0] {
        NativeProcessChange::Create(record) => record.clone(),
        _ => panic!("native create must return create delta"),
    }
}
fn stage_snapshot(record: RuntimeRecord) -> NativeRuntimeSnapshot {
    NativeRuntimeSnapshot {
        complete_case_membership: vec![CaseProcessMembership {
            case_ledger_id: record.process.case_ledger_id.clone(),
            process_ids: vec![record.process.process_id.clone()],
        }],
        process_records: vec![record],
        intake_records: vec![],
    }
}
fn prepared_create() -> PreparedNativeCommand {
    runtime_with_kernel(stage_kernel())
        .prepare_native_command(
            NativeCommandKey("create-1".into()),
            stage_create_command(),
            stage_empty_snapshot(),
            stage_policy(),
        )
        .expect("prepared native create")
}

#[test]
fn native_stage_binds_actual_create_and_intent_without_operational_calls_or_presentation_claim() {
    let store = OperationalStoreProbe::default();
    let calls = store.0.clone();
    let presenter = RecordingPresenter::default();
    let presented = presenter.presented.clone();
    let mut runtime = runtime_with_store(store, stage_kernel());
    runtime.presenter = Box::new(presenter);
    let command = stage_create_command();
    let prepared = runtime
        .prepare_native_command(
            NativeCommandKey("create-1".into()),
            command.clone(),
            stage_empty_snapshot(),
            stage_policy(),
        )
        .expect("native stage");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(presented.lock().unwrap().is_empty());
    assert_eq!(
        serde_json::to_value(prepared.executed_command()).unwrap(),
        serde_json::to_value(command).unwrap()
    );
    assert!(matches!(
        prepared.outcome(),
        NativeRuntimeOutcome::ProcessCreated(_)
    ));
    let record = stage_created_record(&prepared);
    record.validate_creation().unwrap();
    assert_eq!(record.process.process_id, test_process_id("staged"));
    assert_eq!(
        record.process.active_tasks[0].status,
        wos_core::instance::ActiveTaskStatus::Assigned
    );
    assert!(
        record
            .provenance_log
            .iter()
            .all(|p| p.record_kind != ProvenanceKind::TaskPresented)
    );
    let intent = &prepared.presentation_intents()[0];
    assert_eq!(
        serde_json::to_value(&intent.context).unwrap(),
        serde_json::to_value(record.process.active_tasks[0].context.as_ref().unwrap()).unwrap()
    );
    assert!(
        record
            .provenance_log
            .iter()
            .any(|p| p.id == intent.task_created_record_id
                && p.record_kind == ProvenanceKind::TaskCreated)
    );
}

#[test]
fn native_stage_missing_or_incomplete_membership_never_becomes_qualified_empty() {
    let mut snapshot = stage_empty_snapshot();
    snapshot.complete_case_membership.clear();
    assert!(
        runtime_with_kernel(stage_kernel())
            .prepare_native_command(
                NativeCommandKey("missing".into()),
                stage_create_command(),
                snapshot,
                stage_policy()
            )
            .is_err()
    );
    let record = stage_created_record(&prepared_create());
    let mut snapshot = stage_snapshot(record);
    snapshot.complete_case_membership[0]
        .process_ids
        .push(test_process_id("missing").into());
    assert!(
        runtime_with_kernel(stage_kernel())
            .prepare_native_command(
                NativeCommandKey("incomplete".into()),
                NativeRuntimeCommand::DrainOnce {
                    process_id: test_process_id("staged").into()
                },
                snapshot,
                stage_policy()
            )
            .is_err()
    );
}

#[test]
fn native_stage_records_actual_draft_and_preserves_rejected_outcome_and_replay() {
    let record = stage_created_record(&prepared_create());
    let task_id = record.process.active_tasks[0].task_id.clone();
    let draft = runtime_with_kernel(stage_kernel())
        .prepare_native_command(
            NativeCommandKey("draft".into()),
            NativeRuntimeCommand::TaskDraft {
                task_id: task_id.clone(),
                response: integrity_canonical::ResponseJson::from_value(
                    &(serde_json::json!({"status":"in-progress","data":{"photo":"draft"}})),
                )
                .unwrap(),
                actor_id: "reviewer".into(),
                idempotency_token: "draft-token".into(),
            },
            stage_snapshot(record.clone()),
            stage_policy(),
        )
        .unwrap();
    assert!(matches!(
        draft.outcome(),
        NativeRuntimeOutcome::DraftPersisted(_)
    ));
    assert!(
        matches!(&draft.process_changes()[0],NativeProcessChange::Replace {before,after} if before.artifacts.is_empty() && after.artifacts.len()==1)
    );
    let rejected = runtime_with_kernel(stage_kernel())
        .prepare_native_command(
            NativeCommandKey("reject".into()),
            NativeRuntimeCommand::TaskSubmit {
                task_id,
                response: integrity_canonical::ResponseJson::from_value(
                    &(serde_json::json!({"status":"in-progress"})),
                )
                .unwrap(),
                actor_id: "reviewer".into(),
                idempotency_token: "submit-token".into(),
            },
            stage_snapshot(record),
            stage_policy(),
        )
        .unwrap();
    assert!(matches!(
        rejected.outcome(),
        NativeRuntimeOutcome::TaskSubmitted(TaskSubmissionResult::Rejected { .. })
    ));
    assert_eq!(rejected.process_changes().len(), 1);
}

#[test]
fn native_stage_acknowledgement_is_exact_replay_and_unknown_does_not_claim_presentation() {
    let created = prepared_create();
    let record = stage_created_record(&created);
    let intent = created.presentation_intents()[0].clone();
    let unknown = runtime_with_kernel(stage_kernel())
        .prepare_native_command(
            NativeCommandKey("ack-unknown".into()),
            NativeRuntimeCommand::AcknowledgePresentation {
                intent: intent.clone(),
                invocation: NativePresentationInvocationOutcome::Unknown {
                    attempt_id: "attempt-1".into(),
                },
            },
            stage_snapshot(record.clone()),
            stage_policy(),
        )
        .unwrap();
    assert!(matches!(
        unknown.outcome(),
        NativeRuntimeOutcome::PresentationAcknowledged(
            NativePresentationAcknowledgement::RecoveryRequired { .. }
        )
    ));
    assert!(unknown.process_changes().is_empty());
    for occurred_at in ["2023-01-01T00:00:00Z", "2300-01-01T00:00:00Z", "invalid"] {
        assert!(
            runtime_with_kernel(stage_kernel())
                .prepare_native_command(
                    NativeCommandKey("invalid-time".into()),
                    NativeRuntimeCommand::AcknowledgePresentation {
                        intent: intent.clone(),
                        invocation: NativePresentationInvocationOutcome::Invoked {
                            attempt_id: "attempt-1".into(),
                            occurred_at: occurred_at.into(),
                        },
                    },
                    stage_snapshot(record.clone()),
                    stage_policy()
                )
                .is_err()
        );
    }
    let command = NativeRuntimeCommand::AcknowledgePresentation {
        intent: intent.clone(),
        invocation: NativePresentationInvocationOutcome::Invoked {
            attempt_id: "attempt-1".into(),
            occurred_at: "2024-03-09T16:00:00Z".into(),
        },
    };
    let ack = runtime_with_kernel(stage_kernel())
        .prepare_native_command(
            NativeCommandKey("ack".into()),
            command.clone(),
            stage_snapshot(record),
            stage_policy(),
        )
        .unwrap();
    let after = match &ack.process_changes()[0] {
        NativeProcessChange::Replace { after, .. } => after.clone(),
        _ => panic!("ack replaces"),
    };
    assert_eq!(
        after
            .provenance_log
            .iter()
            .filter(|p| p.record_kind == ProvenanceKind::TaskPresented)
            .count(),
        1
    );
    let replay = runtime_with_kernel(stage_kernel())
        .prepare_native_command(
            NativeCommandKey("ack".into()),
            command,
            stage_snapshot(after.clone()),
            stage_policy(),
        )
        .unwrap();
    assert_eq!(
        serde_json::to_value(ack.outcome()).unwrap(),
        serde_json::to_value(replay.outcome()).unwrap()
    );
    assert!(replay.process_changes().is_empty());
    // Stored acknowledgement rows remain strict after process reload and later
    // native saves. The original operational previous record fences deletion.
    for corrupt in ["id", "timestamp", "creation", "invocation", "duplicate"] {
        let mut damaged = after.clone();
        let index = damaged
            .provenance_log
            .iter()
            .position(|p| p.record_kind == ProvenanceKind::TaskPresented)
            .unwrap();
        match corrupt {
            "id" => damaged.provenance_log[index].id = "not-a-native-id".into(),
            "timestamp" => damaged.provenance_log[index].timestamp = "invalid".into(),
            "creation" => {
                damaged.provenance_log[index].data.as_mut().unwrap()["details"]["taskCreatedRecordId"] =
                    serde_json::json!("missing")
            }
            "invocation" => {
                damaged.provenance_log[index].data.as_mut().unwrap()["details"]["nativePresentationInvocation"] =
                    serde_json::json!({"kind":"unknown","attemptId":"attempt-1"})
            }
            "duplicate" => {
                let duplicate = damaged.provenance_log[index].clone();
                damaged.provenance_log.push(duplicate);
            }
            _ => unreachable!(),
        }
        assert!(damaged.validate_creation().is_err(), "{corrupt}");
        assert!(
            damaged.validate_creation_update(&after).is_err(),
            "{corrupt}"
        );
    }
    let mut erased = after.clone();
    erased
        .provenance_log
        .retain(|p| p.record_kind != ProvenanceKind::TaskPresented);
    assert!(erased.validate_creation_update(&after).is_err());
    let mut marker_erased = after.clone();
    let row = marker_erased
        .provenance_log
        .iter_mut()
        .find(|p| p.record_kind == ProvenanceKind::TaskPresented)
        .unwrap();
    row.data.as_mut().unwrap()["details"] = serde_json::json!({});
    assert!(marker_erased.validate_creation_update(&after).is_err());
    let mut stamped = after.clone();
    let row = stamped
        .provenance_log
        .iter_mut()
        .find(|p| p.record_kind == ProvenanceKind::TaskPresented)
        .unwrap();
    crate::runtime::stamp_custody_receipt(
        row,
        &wos_events::custody::CustodyAppendReceipt::new("sha256:test-native-presentation"),
    )
    .unwrap();
    stamped.validate_creation_update(&after).unwrap();
    let mut altered = intent;
    altered.context.prefill_data = Some(serde_json::json!({"altered":true}));
    assert!(
        runtime_with_kernel(stage_kernel())
            .prepare_native_command(
                NativeCommandKey("conflict".into()),
                NativeRuntimeCommand::AcknowledgePresentation {
                    intent: altered,
                    invocation: NativePresentationInvocationOutcome::Invoked {
                        attempt_id: "attempt-1".into(),
                        occurred_at: "2024-03-09T16:00:00Z".into()
                    }
                },
                stage_snapshot(after),
                stage_policy()
            )
            .is_err()
    );
}

#[test]
fn native_stage_refuses_plain_service_before_provider_invocation() {
    let mut value = serde_json::to_value(stage_kernel()).unwrap();
    value["lifecycle"]["states"]["open"]["onEntry"] =
        serde_json::json!([{"action":"invokeService","serviceRef":"sendNotice"}]);
    let kernel = serde_json::from_value(value).unwrap();
    let service = RecordingService::with_response(serde_json::json!({"sent":true}));
    let calls = service.calls.clone();
    let mut runtime = runtime_with_kernel(kernel);
    runtime.service = Box::new(service);
    assert!(
        runtime
            .prepare_native_command(
                NativeCommandKey("effect".into()),
                stage_create_command(),
                stage_empty_snapshot(),
                stage_policy()
            )
            .is_err()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

fn record_json(record: &RuntimeRecord) -> serde_json::Value {
    serde_json::json!({"process": record.process, "provenance": record.provenance_log,
        "aux": crate::store::runtime_aux_to_json(record)})
}

#[test]
fn native_stage_full_case_membership_preserves_native_ambiguity() {
    let first = stage_created_record(&prepared_create());
    let mut other = stage_create_command();
    if let NativeRuntimeCommand::CreateProcess {
        request,
        allocation,
        ..
    } = &mut other
    {
        request.process_id = "staged_process_01hw7rm71vfay8vvw14d2pf2dc".into();
        allocation.process_id = request.process_id.clone();
    }
    let second = stage_created_record(
        &runtime_with_kernel(stage_kernel())
            .prepare_native_command(
                NativeCommandKey("second-process".into()),
                other,
                stage_empty_snapshot(),
                stage_policy(),
            )
            .unwrap(),
    );
    let snapshot = NativeRuntimeSnapshot {
        process_records: vec![second.clone(), first.clone()],
        intake_records: vec![],
        complete_case_membership: vec![CaseProcessMembership {
            case_ledger_id: first.process.case_ledger_id.clone(),
            process_ids: vec![
                first.process.process_id.clone(),
                second.process.process_id.clone(),
            ],
        }],
    };
    let command = NativeRuntimeCommand::AcceptIntake {
        binding: "formspec".into(),
        request: workflow_intake_request("ambiguous-stage", test_case_ledger_id("staged")),
        allocation: None,
    };
    let error = runtime_with_kernel(stage_kernel())
        .prepare_native_command(
            NativeCommandKey("ambiguous".into()),
            command,
            snapshot,
            stage_policy(),
        )
        .err()
        .expect("N:1 case membership must refuse");
    // Compare the actual public intake path: it translates the native case
    // lookup conflict into IntakeConflict while preserving its full detail.
    let mut direct_store = InMemoryStore::new();
    direct_store.restore_record(first.clone()).unwrap();
    direct_store.restore_record(second.clone()).unwrap();
    let mut direct = runtime_with_store(direct_store, stage_kernel());
    let direct_error = direct
        .accept_intake_handoff(
            "formspec",
            workflow_intake_request("ambiguous-stage", test_case_ledger_id("staged")),
        )
        .unwrap_err();
    match (&direct_error, &error) {
        (
            RuntimeError::IntakeConflict(expected),
            NativePreparationFailure::Refused {
                error: RuntimeError::IntakeConflict(actual),
            },
        ) => {
            assert_eq!(actual, expected);
            assert!(actual.contains(&first.process.process_id));
            assert!(actual.contains(&second.process.process_id));
        }
        _ => panic!("native intake {direct_error:?}; staged intake {error:?}"),
    }
    assert_eq!(
        record_json(&first),
        record_json(&direct.store.load_record(&first.process.process_id).unwrap())
    );
    assert_eq!(
        record_json(&second),
        record_json(
            &direct
                .store
                .load_record(&second.process.process_id)
                .unwrap()
        )
    );
}

#[test]
fn native_stage_intake_faults_discard_fresh_writes_and_retain_acknowledged_recovery() {
    let request = public_intake_request(
        "staged-intake",
        test_case_ledger_urn("staged"),
        "urn:test:native-stage",
        "1.0.0",
    );
    // Actual Pending/create/Prepared/process-append/Applied writes are exercised
    // by the maintained intake command, with one isolated collector fault each.
    for at in 1..=5 {
        let probe = OperationalStoreProbe::default();
        let calls = probe.0.clone();
        let mut runtime = runtime_with_store(probe, stage_kernel())
            .with_intake_policy(AutoCreatePublicIntakePolicy);
        runtime.preparation_fault_at_write = Some(at);
        let command = NativeRuntimeCommand::AcceptIntake {
            binding: "formspec".into(),
            request: request.clone(),
            allocation: Some(stage_allocation()),
        };
        assert!(matches!(
            runtime.prepare_native_command(
                NativeCommandKey(format!("fault-{at}")),
                command,
                stage_empty_snapshot(),
                stage_policy()
            ),
            Err(NativePreparationFailure::Refused {
                error: RuntimeError::Store(StoreError::Failed(_))
            })
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
    for fail_at in [1, 2] {
        let mut original = runtime_with_store(IntakeSaveFailingStore::new(fail_at), stage_kernel())
            .with_intake_policy(AutoCreatePublicIntakePolicy);
        assert!(
            original
                .accept_intake_handoff_with_allocation(
                    "formspec",
                    request.clone(),
                    Some(stage_allocation())
                )
                .is_err()
        );
        let process = original
            .store
            .load_record(test_process_id("staged"))
            .unwrap();
        let intake = original
            .store
            .load_intake_record("formspec", "staged-intake")
            .unwrap();
        let original_process = record_json(&process);
        let original_intake = serde_json::to_value(&intake).unwrap();
        let mut snapshot = stage_snapshot(process.clone());
        snapshot.intake_records = vec![intake.clone()];
        let mut runtime =
            runtime_with_kernel(stage_kernel()).with_intake_policy(AutoCreatePublicIntakePolicy);
        runtime.preparation_fault_at_write = Some(1);
        assert!(
            matches!(runtime.prepare_native_command(NativeCommandKey("acknowledged-fault".into()),
            NativeRuntimeCommand::AcceptIntake {binding:"formspec".into(),request:request.clone(),allocation:Some(stage_allocation())},
            snapshot,stage_policy()),Err(NativePreparationFailure::RecoveryRequired {work,error:RuntimeError::Store(StoreError::Failed(_))}) if work.len()==1)
        );
        assert_eq!(record_json(&process), original_process);
        assert_eq!(serde_json::to_value(&intake).unwrap(), original_intake);
        let mut snapshot = stage_snapshot(process.clone());
        snapshot.intake_records = vec![intake.clone()];
        let prepared = runtime_with_kernel(stage_kernel())
            .with_intake_policy(AutoCreatePublicIntakePolicy)
            .prepare_native_command(
                NativeCommandKey("acknowledged-retry".into()),
                NativeRuntimeCommand::AcceptIntake {
                    binding: "formspec".into(),
                    request: request.clone(),
                    allocation: Some(stage_allocation()),
                },
                snapshot,
                stage_policy(),
            )
            .unwrap();
        assert!(
            matches!(prepared.outcome(),NativeRuntimeOutcome::IntakeAccepted(decision) if decision.creation.as_ref()==process.creation.as_ref())
        );
        assert!(prepared.presentation_intents().is_empty());
    }
}

#[test]
fn native_stage_acknowledgement_rechecks_assignment_and_generation() {
    let prepared = prepared_create();
    let mut record = stage_created_record(&prepared);
    let intent = prepared.presentation_intents()[0].clone();
    record.process.active_tasks[0].assigned_actor = Some("replacement".into());
    record.process.active_tasks[0]
        .context
        .as_mut()
        .unwrap()
        .assigned_actor = "replacement".into();
    record.process.active_tasks[0].updated_at = "2024-03-09T16:00:01Z".into();
    let ack = runtime_with_kernel(stage_kernel())
        .prepare_native_command(
            NativeCommandKey("obsolete".into()),
            NativeRuntimeCommand::AcknowledgePresentation {
                intent,
                invocation: NativePresentationInvocationOutcome::Invoked {
                    attempt_id: "old-attempt".into(),
                    occurred_at: "2024-03-09T16:00:00Z".into(),
                },
            },
            stage_snapshot(record),
            stage_policy(),
        )
        .unwrap();
    assert!(matches!(
        ack.outcome(),
        NativeRuntimeOutcome::PresentationAcknowledged(
            NativePresentationAcknowledgement::Obsolete { .. }
        )
    ));
    assert!(ack.process_changes().is_empty());
}

#[test]
fn native_stage_operation_selection_does_not_qualify_signature_extraction_or_intake_finalize() {
    let mut policy = stage_policy();
    policy
        .read_only_intake_binding_ids
        .remove(&NativeIntakeBindingOperation::Finalize);
    let request = public_intake_request(
        "finalizer-refused",
        test_case_ledger_urn("staged"),
        "urn:test:native-stage",
        "1.0.0",
    );
    let probe = OperationalStoreProbe::default();
    let calls = probe.0.clone();
    let result = runtime_with_store(probe, stage_kernel())
        .with_intake_policy(AutoCreatePublicIntakePolicy)
        .prepare_native_command(
            NativeCommandKey("finalizer-refused".into()),
            NativeRuntimeCommand::AcceptIntake {
                binding: "formspec".into(),
                request,
                allocation: Some(stage_allocation()),
            },
            stage_empty_snapshot(),
            policy,
        );
    assert!(matches!(
        result,
        Err(NativePreparationFailure::Refused {
            error: RuntimeError::NativePreparation(
                NativePreparationError::UnqualifiedIntakeBinding {
                    operation: NativeIntakeBindingOperation::Finalize,
                    ..
                }
            )
        })
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    #[derive(Debug)]
    struct SigningProbe(Arc<AtomicUsize>);
    impl crate::ContractBindingAdapter for SigningProbe {
        fn binding(&self) -> &'static str {
            "formspec"
        }
        fn prepare_task(
            &self,
            task: &wos_core::instance::ActiveTask,
            state: &serde_json::Value,
        ) -> Result<crate::PreparedTask, crate::BindingError> {
            TestAdapter.prepare_task(task, state)
        }
        fn validate_submission(
            &self,
            task: &wos_core::instance::ActiveTask,
            response: &serde_json::Value,
        ) -> Result<crate::SubmissionValidation, crate::BindingError> {
            TestAdapter.validate_submission(task, response)
        }
        fn compute_case_mutation(
            &self,
            task: &wos_core::instance::ActiveTask,
            response: &serde_json::Value,
        ) -> Result<Option<crate::CaseMutationBundle>, crate::BindingError> {
            TestAdapter.compute_case_mutation(task, response)
        }
        fn signature_evidence(
            &self,
            _: &wos_core::instance::ActiveTask,
            _: &integrity_canonical::ResponseJson,
        ) -> Result<Option<Vec<crate::SignatureEvidence>>, crate::BindingError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(None)
        }
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = crate::BindingRegistry::new();
    registry.register(SigningProbe(calls.clone()));
    let mut kernel_value = serde_json::to_value(stage_kernel()).unwrap();
    kernel_value["lifecycle"]["states"]["open"]["onEntry"][0]["extensions"] =
        serde_json::json!({"x-wos-signature-profile-key":"staged-signature"});
    let kernel: KernelDocument = serde_json::from_value(kernel_value).unwrap();
    let created = runtime_with_kernel(kernel.clone())
        .prepare_native_command(
            NativeCommandKey("signature-create".into()),
            stage_create_command(),
            stage_empty_snapshot(),
            stage_policy(),
        )
        .unwrap();
    let record = stage_created_record(&created);
    let task_id = record.process.active_tasks[0].task_id.clone();
    let mut runtime = runtime_with_kernel(kernel);
    runtime.bindings = registry;
    let mut policy = stage_policy();
    policy
        .read_only_task_binding_ids
        .remove(&NativeTaskBindingOperation::SignatureEvidence);
    let response = serde_json::json!({"status":"completed","definitionUrl":"urn:formspec:stage-review","definitionVersion":"1.0.0","data":{"approved":true}});
    assert!(matches!(
        runtime.prepare_native_command(
            NativeCommandKey("no-signing".into()),
            NativeRuntimeCommand::TaskSubmit {
                task_id,
                response: integrity_canonical::ResponseJson::from_value(&response).unwrap(),
                actor_id: "reviewer".into(),
                idempotency_token: "signing-token".into()
            },
            stage_snapshot(record),
            policy
        ),
        Err(NativePreparationFailure::Refused {
            error: RuntimeError::NativePreparation(
                NativePreparationError::UnqualifiedTaskBinding {
                    operation: NativeTaskBindingOperation::SignatureEvidence,
                    ..
                }
            )
        })
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn native_stage_deferred_work_uses_native_subject_snapshot_and_does_not_reemit_on_idle() {
    let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
        "$wosWorkflow":"1.0","url":"urn:test:native-stage","version":"1.0.0",
        "lifecycle":{"initialState":"open","states":{"open":{"type":"atomic","onEntry":[
            {"action":"invokeService","serviceRef":"notice","callbackPhase":"outbound"}
        ]}}}
    }))
    .unwrap();
    let profile:crate::IntegrationProfileDocument=serde_json::from_value(serde_json::json!({
        "targetWorkflow":{"url":"urn:test:native-stage"},"bindings":{"notice":{
            "type":"callback","dispatchMode":"deferred","callbackEventType":"com.example.receipt",
            "eventType":"com.example.request","dataMapping":{"reference":"caseFile.reference"},"outputBinding":{"caseFile.receipt":"$"}
        }}
    })).unwrap();
    let service = RecordingService::with_response(serde_json::json!({"sent":true}));
    let calls = service.calls.clone();
    let mut runtime = runtime_with_kernel(kernel.clone()).with_integration_profile(profile.clone());
    runtime.service = Box::new(service);
    let mut policy = stage_policy();
    policy.deferred_callback_binding_ids.insert("notice".into());
    let mut command = stage_create_command();
    if let NativeRuntimeCommand::CreateProcess { request, .. } = &mut command {
        request.initial_case_state = Some(serde_json::json!({"reference":"native-selected"}));
    }
    let prepared = runtime
        .prepare_native_command(
            NativeCommandKey("deferred-create".into()),
            command,
            stage_empty_snapshot(),
            policy.clone(),
        )
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(prepared.deferred_outbound_events().len(), 1);
    let record = stage_created_record(&prepared);
    let work = &prepared.deferred_outbound_events()[0];
    assert_eq!(work.process_id, record.process.process_id);
    assert_eq!(
        record
            .process
            .pending_outbound_events
            .get(&work.event.subject),
        Some(&work.event)
    );
    assert!(
        record
            .process
            .pending_callbacks
            .contains_key(&work.event.subject)
    );
    let idle = runtime_with_kernel(kernel)
        .with_integration_profile(profile)
        .prepare_native_command(
            NativeCommandKey("idle".into()),
            NativeRuntimeCommand::DrainOnce {
                process_id: record.process.process_id.clone(),
            },
            stage_snapshot(record),
            policy,
        )
        .unwrap();
    assert!(
        matches!(idle.outcome(),NativeRuntimeOutcome::Drained(result) if result.processed_event.is_none())
    );
    assert!(idle.deferred_outbound_events().is_empty());
}

#[test]
fn native_stage_completed_and_failed_are_successful_native_outcomes_with_saved_evidence() {
    for approved in [false, true] {
        let record = stage_created_record(&prepared_create());
        let task_id = record.process.active_tasks[0].task_id.clone();
        let prepared=runtime_with_kernel(stage_kernel()).prepare_native_command(NativeCommandKey(format!("submit-{approved}")),
            NativeRuntimeCommand::TaskSubmit {task_id,response:integrity_canonical::ResponseJson::from_value(&(serde_json::json!({"status":"completed","definitionUrl":"urn:formspec:stage-review","definitionVersion":"1.0.0","data":{"approved":approved}}))).unwrap(),
                actor_id:"reviewer".into(),idempotency_token:"actual-submit".into()},stage_snapshot(record),stage_policy()).unwrap();
        assert!(if approved {
            matches!(
                prepared.outcome(),
                NativeRuntimeOutcome::TaskSubmitted(TaskSubmissionResult::Completed { .. })
            )
        } else {
            matches!(
                prepared.outcome(),
                NativeRuntimeOutcome::TaskSubmitted(TaskSubmissionResult::Failed { .. })
            )
        });
        assert!(
            matches!(&prepared.process_changes()[0],NativeProcessChange::Replace {after,..} if !after.replay_entries.is_empty())
        );
    }
}

#[test]
fn native_stage_empty_policy_refuses_before_any_adapter_or_presenter_calls() {
    let probe = OperationalStoreProbe::default();
    let calls = probe.0.clone();
    let presenter = RecordingPresenter::default();
    let presented = presenter.presented.clone();
    let mut runtime = runtime_with_store(probe, stage_kernel());
    runtime.presenter = Box::new(presenter);
    assert!(matches!(
        runtime.prepare_native_command(
            NativeCommandKey("unqualified".into()),
            stage_create_command(),
            stage_empty_snapshot(),
            NativePreparationPolicy::default()
        ),
        Err(NativePreparationFailure::Refused {
            error: RuntimeError::NativePreparation(NativePreparationError::UnqualifiedDependency {
                dependency: NativeReadOnlyDependency::Clock
            })
        })
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(presented.lock().unwrap().is_empty());
}

fn bounded_stage_kernel() -> KernelDocument {
    let mut value = serde_json::to_value(stage_kernel()).unwrap();
    let create = serde_json::json!({"action":"createTask","taskRef":"review","assignTo":"reviewer","contractRef":"review","expiresWithTimer":"response"});
    value["lifecycle"]["states"]["open"]["onEntry"] = serde_json::json!([
        {"action":"startTimer","timerId":"response","duration":"PT1S","event":"expired"},create,
        {"action":"cancelTimer","timerId":"response"}
    ]);
    serde_json::from_value(value).unwrap()
}

#[test]
fn native_stage_expired_ack_is_obsolete_and_idle_retirement_still_returns_native_write() {
    let kernel = bounded_stage_kernel();
    let created = runtime_with_kernel(kernel.clone())
        .prepare_native_command(
            NativeCommandKey("bounded-create".into()),
            stage_create_command(),
            stage_empty_snapshot(),
            stage_policy(),
        )
        .unwrap();
    let record = stage_created_record(&created);
    let intent = created.presentation_intents()[0].clone();
    assert!(record.process.timers.is_empty());
    assert!(record.process.active_tasks[0].expires_at.is_some());
    let mut runtime = runtime_with_kernel(kernel.clone());
    runtime.clock = Box::new(FixedClock {
        now_ms: 1_710_000_001_000,
    });
    let ack = runtime
        .prepare_native_command(
            NativeCommandKey("expired-ack".into()),
            NativeRuntimeCommand::AcknowledgePresentation {
                intent,
                invocation: NativePresentationInvocationOutcome::Invoked {
                    attempt_id: "late-attempt".into(),
                    occurred_at: "2024-03-09T16:00:00Z".into(),
                },
            },
            stage_snapshot(record.clone()),
            stage_policy(),
        )
        .unwrap();
    assert!(matches!(
        ack.outcome(),
        NativeRuntimeOutcome::PresentationAcknowledged(
            NativePresentationAcknowledgement::Obsolete { .. }
        )
    ));
    assert!(ack.process_changes().is_empty());
    let mut runtime = runtime_with_kernel(kernel);
    runtime.clock = Box::new(FixedClock {
        now_ms: 1_710_000_001_000,
    });
    let idle = runtime
        .prepare_native_command(
            NativeCommandKey("idle-expiry".into()),
            NativeRuntimeCommand::DrainOnce {
                process_id: record.process.process_id.clone(),
            },
            stage_snapshot(record),
            stage_policy(),
        )
        .unwrap();
    assert!(
        matches!(idle.outcome(),NativeRuntimeOutcome::Drained(result) if result.processed_event.is_none())
    );
    assert!(
        matches!(&idle.process_changes()[0],NativeProcessChange::Replace {before,after} if before.process.active_tasks.len()==1 && after.process.active_tasks.is_empty()
        && after.provenance_log.iter().any(|p|p.record_kind==ProvenanceKind::TaskFailed && p.data.as_ref().and_then(|d|d.pointer("/details/taskStatus")).and_then(serde_json::Value::as_str)==Some("expired")))
    );
}

#[test]
fn native_stage_rpc_requires_exact_read_only_selection_and_keeps_actual_native_result() {
    for qualified in [false, true] {
        let mut value = serde_json::to_value(stage_kernel()).unwrap();
        value["lifecycle"]["states"]["open"]["onEntry"] =
            serde_json::json!([{"action":"invokeService","serviceRef":"readonlyCheck"}]);
        let kernel = serde_json::from_value(value).unwrap();
        let profile=serde_json::from_value(serde_json::json!({"targetWorkflow":{"url":"urn:test:native-stage"},"bindings":{"readonlyCheck":{"type":"request-response"}}})).unwrap();
        let service = RecordingService::with_response(serde_json::json!({"checked":true}));
        let calls = service.calls.clone();
        let mut runtime = runtime_with_kernel(kernel).with_integration_profile(profile);
        runtime.service = Box::new(service);
        let mut policy = stage_policy();
        if qualified {
            policy
                .read_only_rpc_binding_ids
                .insert("readonlyCheck".into());
        }
        let result = runtime.prepare_native_command(
            NativeCommandKey("check".into()),
            stage_create_command(),
            stage_empty_snapshot(),
            policy,
        );
        if qualified {
            let prepared = result.unwrap();
            let record = stage_created_record(&prepared);
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            assert!(
                record
                    .step_results
                    .iter()
                    .any(|result| result.output == serde_json::json!({"checked":true}))
            );
        } else {
            assert!(result.is_err());
            assert_eq!(calls.load(Ordering::SeqCst), 0);
        }
    }
}

#[test]
fn native_stage_work_command_and_acknowledgement_serde_preserve_native_values() {
    let prepared = prepared_create();
    let intent = prepared.presentation_intents()[0].clone();
    let encoded = serde_json::to_value(&intent).unwrap();
    let decoded: PendingNativePresentationIntent = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(serde_json::to_value(&decoded).unwrap(), encoded);
    let command = NativeRuntimeCommand::AcknowledgePresentation {
        intent: decoded,
        invocation: NativePresentationInvocationOutcome::Failed {
            attempt_id: "failed-attempt".into(),
            code: "renderUnavailable".into(),
        },
    };
    let encoded = serde_json::to_value(&command).unwrap();
    let decoded: NativeRuntimeCommand = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), encoded);
    let record = stage_created_record(&prepared);
    let failed = runtime_with_kernel(stage_kernel())
        .prepare_native_command(
            NativeCommandKey("failed-ack".into()),
            command,
            stage_snapshot(record),
            stage_policy(),
        )
        .unwrap();
    assert!(matches!(
        failed.outcome(),
        NativeRuntimeOutcome::PresentationAcknowledged(
            NativePresentationAcknowledgement::RecoveryRequired { .. }
        )
    ));
    assert!(failed.process_changes().is_empty());
}

#[test]
fn native_stage_immediate_and_external_integration_kinds_refuse_before_provider_calls() {
    for kind in [
        "callback",
        "event-emit",
        "event-consume",
        "arazzo-sequence",
        "tool",
        "policy-engine",
    ] {
        let mut value = serde_json::to_value(stage_kernel()).unwrap();
        value["lifecycle"]["states"]["open"]["onEntry"] = serde_json::json!([
            {"action":"invokeService","serviceRef":"irreversible"}
        ]);
        let kernel = serde_json::from_value(value).unwrap();
        let profile = serde_json::from_value(serde_json::json!({
            "targetWorkflow":{"url":"urn:test:native-stage"},
            "bindings":{"irreversible":{"type":kind}}
        }))
        .unwrap();
        let service = RecordingService::with_response(serde_json::json!({"effect":true}));
        let calls = service.calls.clone();
        let mut runtime = runtime_with_kernel(kernel).with_integration_profile(profile);
        runtime.service = Box::new(service);
        let mut policy = stage_policy();
        // Selecting the name in both supported operation categories must not
        // qualify a different integration kind or old immediate callback.
        policy
            .read_only_rpc_binding_ids
            .insert("irreversible".into());
        policy
            .deferred_callback_binding_ids
            .insert("irreversible".into());
        assert!(
            runtime
                .prepare_native_command(
                    NativeCommandKey(format!("deny-{kind}")),
                    stage_create_command(),
                    stage_empty_snapshot(),
                    policy
                )
                .is_err(),
            "{kind}"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0, "{kind}");
    }
}

#[test]
fn native_stage_confirmed_ack_replays_after_native_completion_without_new_presentation() {
    let created = prepared_create();
    let intent = created.presentation_intents()[0].clone();
    let command = NativeRuntimeCommand::AcknowledgePresentation {
        intent: intent.clone(),
        invocation: NativePresentationInvocationOutcome::Invoked {
            attempt_id: "confirmed-before-completion".into(),
            occurred_at: "2024-03-09T16:00:00Z".into(),
        },
    };
    let presenter = RecordingPresenter::default();
    let mut runtime = runtime_with_kernel(stage_kernel());
    runtime.presenter = Box::new(presenter.clone());
    let ack = runtime
        .prepare_native_command(
            NativeCommandKey("confirmed".into()),
            command.clone(),
            stage_snapshot(stage_created_record(&created)),
            stage_policy(),
        )
        .unwrap();
    let recorded = serde_json::to_value(ack.outcome()).unwrap();
    let after_ack = match &ack.process_changes()[0] {
        NativeProcessChange::Replace { after, .. } => after.clone(),
        _ => panic!("acknowledgement must replace actual record"),
    };
    let mut runtime = runtime_with_kernel(stage_kernel());
    runtime.presenter = Box::new(presenter.clone());
    let completion = runtime.prepare_native_command(NativeCommandKey("complete-after-ack".into()),
        NativeRuntimeCommand::TaskSubmit {
            task_id: intent.task_id.clone(), actor_id: "reviewer".into(), idempotency_token: "complete-after-ack".into(),
            response: integrity_canonical::ResponseJson::from_value(&(serde_json::json!({"status":"completed","definitionUrl":"urn:formspec:stage-review","definitionVersion":"1.0.0","data":{"approved":true}}))).unwrap(),
        }, stage_snapshot(after_ack), stage_policy()).unwrap();
    assert!(matches!(
        completion.outcome(),
        NativeRuntimeOutcome::TaskSubmitted(TaskSubmissionResult::Completed { .. })
    ));
    let completed = match &completion.process_changes()[0] {
        NativeProcessChange::Replace { after, .. } => after.clone(),
        _ => panic!("completion must replace actual record"),
    };
    assert!(
        completed
            .process
            .active_tasks
            .iter()
            .all(|task| task.task_id != intent.task_id)
    );
    let before_replay = record_json(&completed);
    let mut runtime = runtime_with_kernel(stage_kernel());
    runtime.presenter = Box::new(presenter.clone());
    let replay = runtime
        .prepare_native_command(
            NativeCommandKey("confirmed".into()),
            command,
            stage_snapshot(completed.clone()),
            stage_policy(),
        )
        .unwrap();
    assert_eq!(serde_json::to_value(replay.outcome()).unwrap(), recorded);
    assert!(replay.process_changes().is_empty());
    assert!(replay.presentation_intents().is_empty());
    assert_eq!(record_json(&completed), before_replay);
    assert_eq!(
        completed
            .provenance_log
            .iter()
            .filter(|row| row.record_kind == ProvenanceKind::TaskPresented)
            .count(),
        1
    );
    let mut altered = intent;
    altered.context.prefill_data = Some(serde_json::json!({"changedAfterRemoval":true}));
    let mut runtime = runtime_with_kernel(stage_kernel());
    runtime.presenter = Box::new(presenter.clone());
    assert!(matches!(
        runtime.prepare_native_command(
            NativeCommandKey("conflict-after-removal".into()),
            NativeRuntimeCommand::AcknowledgePresentation {
                intent: altered,
                invocation: NativePresentationInvocationOutcome::Invoked {
                    attempt_id: "confirmed-before-completion".into(),
                    occurred_at: "2024-03-09T16:00:00Z".into(),
                },
            },
            stage_snapshot(completed),
            stage_policy()
        ),
        Err(NativePreparationFailure::Refused {
            error: RuntimeError::NativePreparation(
                NativePreparationError::PresentationConflict { .. }
            )
        })
    ));
    assert!(presenter.presented.lock().unwrap().is_empty());
    assert!(presenter.dismissed.lock().unwrap().is_empty());
}

#[derive(Debug)]
struct StageClockProbe(Arc<AtomicUsize>);
impl Clock for StageClockProbe {
    fn now_ms(&self) -> u64 {
        self.0.fetch_add(1, Ordering::SeqCst);
        1_710_000_000_000
    }
}
struct StageResolverProbe {
    calls: Arc<AtomicUsize>,
    kernel: KernelDocument,
}
impl super::super::ResolveDocumentsDyn for StageResolverProbe {
    fn resolve_kernel(&self, url: &str, version: &str) -> Result<KernelDocument, RuntimeError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(self.kernel.url.as_deref(), Some(url));
        assert_eq!(self.kernel.version.as_deref(), Some(version));
        Ok(self.kernel.clone())
    }
}
#[derive(Debug)]
struct StageAccessProbe(Arc<AtomicUsize>);
impl wos_core::traits::AccessControl for StageAccessProbe {
    fn can_transition(&self, _: &str, _: &str) -> bool {
        true
    }
    fn can_read(&self, _: &str, _: &str) -> bool {
        true
    }
    fn can_delegate(
        &self,
        delegator: &str,
        delegate: &str,
        _: &wos_core::model::governance::DelegationScope,
    ) -> bool {
        assert_eq!(delegator, "reviewer");
        assert_eq!(delegate, "native-qualified-delegate");
        self.0.fetch_add(1, Ordering::SeqCst);
        true
    }
}

#[test]
fn native_stage_dependency_selection_reaches_only_the_selected_injected_call() {
    for dependency in [
        NativeReadOnlyDependency::Clock,
        NativeReadOnlyDependency::Resolver,
        NativeReadOnlyDependency::AccessControl,
    ] {
        for selected in [false, true] {
            let calls = Arc::new(AtomicUsize::new(0));
            let mut runtime = runtime_with_kernel(stage_kernel());
            let (command, snapshot) = if dependency == NativeReadOnlyDependency::AccessControl {
                runtime.access_control = Box::new(StageAccessProbe(calls.clone()));
                let record = stage_created_record(&prepared_create());
                (
                    NativeRuntimeCommand::TaskSubmit {
                        task_id: record.process.active_tasks[0].task_id.clone(),
                        actor_id: "native-qualified-delegate".into(),
                        idempotency_token: "qualified-delegation".into(),
                        response: integrity_canonical::ResponseJson::from_value(&(serde_json::json!({"status":"completed","definitionUrl":"urn:formspec:stage-review","definitionVersion":"1.0.0","data":{"approved":true}}))).unwrap(),
                    },
                    stage_snapshot(record),
                )
            } else {
                if dependency == NativeReadOnlyDependency::Clock {
                    runtime.clock = Box::new(StageClockProbe(calls.clone()));
                } else {
                    runtime.resolver = Box::new(StageResolverProbe {
                        calls: calls.clone(),
                        kernel: stage_kernel(),
                    });
                }
                (stage_create_command(), stage_empty_snapshot())
            };
            let mut policy = stage_policy();
            if !selected {
                policy.read_only_dependency_ids.remove(&dependency);
            }
            let result = runtime.prepare_native_command(
                NativeCommandKey(format!("dependency-{dependency:?}-{selected}")),
                command,
                snapshot,
                policy,
            );
            if selected {
                let prepared = result.unwrap();
                assert!(calls.load(Ordering::SeqCst) > 0, "{dependency:?}");
                assert!(matches!(
                    prepared.outcome(),
                    NativeRuntimeOutcome::ProcessCreated(_)
                        | NativeRuntimeOutcome::TaskSubmitted(
                            TaskSubmissionResult::Completed { .. }
                        )
                ));
            } else {
                assert!(
                    matches!(result, Err(NativePreparationFailure::Refused {
                    error: RuntimeError::NativePreparation(NativePreparationError::UnqualifiedDependency { dependency: actual })
                }) if actual == dependency),
                    "{dependency:?}"
                );
                assert_eq!(calls.load(Ordering::SeqCst), 0, "{dependency:?}");
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
struct StageProcessorProbe {
    prepare: Arc<AtomicUsize>,
    validate: Arc<AtomicUsize>,
    map: Arc<AtomicUsize>,
}
impl crate::ContractBindingAdapter for StageProcessorProbe {
    fn binding(&self) -> &'static str {
        "formspec"
    }
    fn prepare_task(
        &self,
        task: &ActiveTask,
        state: &serde_json::Value,
    ) -> Result<crate::PreparedTask, crate::BindingError> {
        self.prepare.fetch_add(1, Ordering::SeqCst);
        TestAdapter.prepare_task(task, state)
    }
    fn validate_submission(
        &self,
        task: &ActiveTask,
        response: &serde_json::Value,
    ) -> Result<crate::SubmissionValidation, crate::BindingError> {
        self.validate.fetch_add(1, Ordering::SeqCst);
        TestAdapter.validate_submission(task, response)
    }
    fn compute_case_mutation(
        &self,
        task: &ActiveTask,
        response: &serde_json::Value,
    ) -> Result<Option<crate::CaseMutationBundle>, crate::BindingError> {
        self.map.fetch_add(1, Ordering::SeqCst);
        TestAdapter.compute_case_mutation(task, response)
    }
}

#[test]
fn native_stage_processor_operation_selection_has_positive_counting_controls() {
    for operation in [
        NativeTaskBindingOperation::Prepare,
        NativeTaskBindingOperation::Validate,
        NativeTaskBindingOperation::Map,
    ] {
        for selected in [false, true] {
            let probe = StageProcessorProbe::default();
            let counter = match operation {
                NativeTaskBindingOperation::Prepare => probe.prepare.clone(),
                NativeTaskBindingOperation::Validate => probe.validate.clone(),
                NativeTaskBindingOperation::Map => probe.map.clone(),
                _ => unreachable!(),
            };
            let mut runtime = runtime_with_kernel(stage_kernel());
            let mut registry = crate::BindingRegistry::new();
            registry.register(probe.clone());
            runtime.bindings = registry;
            let (command, snapshot) = if operation == NativeTaskBindingOperation::Prepare {
                (stage_create_command(), stage_empty_snapshot())
            } else {
                let record = stage_created_record(&prepared_create());
                (
                    NativeRuntimeCommand::TaskSubmit {
                        task_id: record.process.active_tasks[0].task_id.clone(),
                        actor_id: "reviewer".into(),
                        idempotency_token: "processor-qualified".into(),
                        response: integrity_canonical::ResponseJson::from_value(&(serde_json::json!({"status":"completed","definitionUrl":"urn:formspec:stage-review","definitionVersion":"1.0.0","data":{"approved":true}}))).unwrap(),
                    },
                    stage_snapshot(record),
                )
            };
            let mut policy = stage_policy();
            if !selected {
                policy.read_only_task_binding_ids.remove(&operation);
            }
            let result = runtime.prepare_native_command(
                NativeCommandKey(format!("processor-{operation:?}-{selected}")),
                command,
                snapshot,
                policy,
            );
            if selected {
                let prepared = result.unwrap();
                assert!(counter.load(Ordering::SeqCst) > 0, "{operation:?}");
                assert!(matches!(
                    prepared.outcome(),
                    NativeRuntimeOutcome::ProcessCreated(_)
                        | NativeRuntimeOutcome::TaskSubmitted(
                            TaskSubmissionResult::Completed { .. }
                        )
                ));
            } else {
                assert!(
                    matches!(result, Err(NativePreparationFailure::Refused {
                    error: RuntimeError::NativePreparation(NativePreparationError::UnqualifiedTaskBinding { operation: actual, .. })
                }) if actual == operation),
                    "{operation:?}"
                );
                assert_eq!(counter.load(Ordering::SeqCst), 0, "{operation:?}");
                if operation == NativeTaskBindingOperation::Map {
                    assert_eq!(probe.validate.load(Ordering::SeqCst), 1);
                }
            }
        }
    }
}

#[test]
fn native_stage_lifecycle_control_returns_its_change_and_gates_later_evaluation() {
    let record = stage_created_record(&prepared_create());
    let process_id = record.process.process_id.clone();
    let suspend = NativeRuntimeCommand::Lifecycle {
        process_id: process_id.clone(),
        control: wos_core::LifecycleControl::Suspend,
        actor_id: "actor:human:reviewer".into(),
        reason: Some("hold".into()),
        idempotency_token: "suspend-token".into(),
    };
    let prepared = runtime_with_kernel(stage_kernel())
        .prepare_native_command(
            NativeCommandKey("suspend-1".into()),
            suspend,
            stage_snapshot(record),
            stage_policy(),
        )
        .expect("prepared suspend");
    let NativeRuntimeOutcome::LifecycleChanged(process) = prepared.outcome() else {
        panic!("suspend returns the changed process");
    };
    assert_eq!(process.status, wos_core::instance::InstanceStatus::Suspended);
    let [NativeProcessChange::Replace { before, after }] = prepared.process_changes() else {
        panic!("suspend replaces exactly its process");
    };
    assert_eq!(before.process.status, wos_core::instance::InstanceStatus::Active);
    assert_eq!(
        after.provenance_log.last().map(|record| record.record_kind),
        Some(ProvenanceKind::InstanceSuspended)
    );
    let refused = runtime_with_kernel(stage_kernel()).prepare_native_command(
        NativeCommandKey("drain-1".into()),
        NativeRuntimeCommand::DrainOnce { process_id },
        stage_snapshot(after.clone()),
        stage_policy(),
    );
    assert!(matches!(
        refused,
        Err(NativePreparationFailure::Refused {
            error: RuntimeError::InstanceStatusConflict { .. }
        })
    ));
}
