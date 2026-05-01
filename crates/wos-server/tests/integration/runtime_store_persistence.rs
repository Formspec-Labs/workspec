//! StorageBackedRuntimeStore persistence round-trips across a simulated restart.
//!
//! Covers review findings F1 (aux codec round-trips AcceptIntakeHandoff +
//! Intake replay values), F4 (IntakeRecord survives a fresh store handle),
//! and F5 (NotFound errors carry the missing resource id as context).

use std::sync::Arc;

use chrono::Utc;
use wos_core::instance::{CaseInstance, InstanceStatus};
use wos_runtime::intake::{
    IntakeAcceptanceDecision, IntakeAcceptanceOutcome, IntakeAcceptanceRequest,
    IntakeCaseDefinition, IntakeCaseDisposition, IntakeRecordStatus,
};
use wos_runtime::store::{
    IntakeRecord, ReplayKey, ReplayOperation, ReplayValue, RuntimeRecord, RuntimeStore, StoreError,
};
use wos_server::runtime::runtime_store::StorageBackedRuntimeStore;
use wos_server::services::provenance_service::ProvenanceService;
use wos_server::storage::{SqliteStorage, StorageHandle};
use wos_server_ports::audit::NoopAuditSink;

async fn fresh() -> StorageHandle {
    let store = SqliteStorage::connect("sqlite::memory:?cache=shared")
        .await
        .expect("connect sqlite");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn mk_instance(id: &str) -> CaseInstance {
    serde_json::from_value(serde_json::json!({
        "instanceId": id,
        "definitionUrl": "urn:wos:workflow:intake-demo:1",
        "definitionVersion": "1",
        "configuration": ["intake"],
        "caseState": {},
        "provenancePosition": 0,
        "nextTaskSequence": 0,
        "timers": [],
        "activeTasks": [],
        "historyStore": {},
        "compensationLogs": {},
        "status": "active",
        "pendingEvents": [],
        "createdAt": "2026-04-24T00:00:00Z",
        "updatedAt": "2026-04-24T00:00:00Z",
    }))
    .expect("build CaseInstance fixture")
}

#[allow(dead_code)]
fn _assert_status_enum_shape() {
    // Force a compile-time reference so a breaking enum rename trips the test build.
    let _ = InstanceStatus::Active;
}

fn sample_decision() -> IntakeAcceptanceDecision {
    IntakeAcceptanceDecision {
        outcome: IntakeAcceptanceOutcome::Accepted {
            case_disposition: IntakeCaseDisposition::CreateGovernedCase {
                case_ref: "case-abc".into(),
                definition: IntakeCaseDefinition {
                    definition_url: "urn:wos:workflow:intake-demo:1".into(),
                    definition_version: "1".into(),
                },
                initial_case_state: Some(serde_json::json!({"applicantId": "a-1"})),
            },
        },
        provenance: Vec::new(),
    }
}

fn sample_intake_record(binding: &str, intake_id: &str) -> IntakeRecord {
    IntakeRecord {
        binding: binding.into(),
        intake_id: intake_id.into(),
        request: IntakeAcceptanceRequest {
            document: serde_json::json!({"kind": "formspecIntake"}),
            actor_id: Some("agent-intake".into()),
            governed_case_ref: Some("case-abc".into()),
            governed_case_definition: Some(IntakeCaseDefinition {
                definition_url: "urn:wos:workflow:intake-demo:1".into(),
                definition_version: "1".into(),
            }),
            initial_case_state: Some(serde_json::json!({"applicantId": "a-1"})),
        },
        outcome: IntakeAcceptanceOutcome::Accepted {
            case_disposition: IntakeCaseDisposition::AttachToExistingCase {
                case_ref: "case-abc".into(),
            },
        },
        provenance_log: Vec::new(),
        status: IntakeRecordStatus::Prepared,
        recorded_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
    }
}

/// F1: aux codec must round-trip `ReplayOperation::AcceptIntakeHandoff` +
/// `ReplayValue::Intake(IntakeAcceptanceDecision)`. Previously decode silently
/// dropped these entries.
#[tokio::test]
async fn aux_replay_round_trips_intake_handoff_entries() {
    let storage = fresh().await;
    let provenance = Arc::new(ProvenanceService::new(storage.clone()));
    let handle = tokio::runtime::Handle::current();

    let instance = mk_instance("inst-intake-replay");
    let mut record = RuntimeRecord::new(instance);
    let key = ReplayKey {
        operation: ReplayOperation::AcceptIntakeHandoff,
        task_id: "intake-task".into(),
        actor_id: "agent-intake".into(),
        token: "tok-1".into(),
    };
    record
        .replay_entries
        .insert(key.clone(), ReplayValue::Intake(sample_decision()));

    let storage_for_task = storage.clone();
    let provenance_for_task = provenance.clone();
    let audit_for_task = Arc::new(NoopAuditSink);
    let handle_for_task = handle.clone();
    tokio::task::spawn_blocking(move || {
        let mut store = StorageBackedRuntimeStore::new(
            storage_for_task,
            provenance_for_task,
            audit_for_task,
            handle_for_task,
        );
        store.create_record(record).expect("create record");
    })
    .await
    .unwrap();

    let storage_for_load = storage.clone();
    let audit_for_load = Arc::new(NoopAuditSink);
    let loaded = tokio::task::spawn_blocking(move || {
        let store =
            StorageBackedRuntimeStore::new(storage_for_load, provenance, audit_for_load, handle);
        store
            .load_record("inst-intake-replay")
            .expect("load record")
    })
    .await
    .unwrap();

    let replay = loaded
        .replay_entries
        .get(&key)
        .expect("intake replay entry survives round-trip");
    match replay {
        ReplayValue::Intake(decision) => match &decision.outcome {
            IntakeAcceptanceOutcome::Accepted { case_disposition } => match case_disposition {
                IntakeCaseDisposition::CreateGovernedCase {
                    case_ref,
                    definition,
                    initial_case_state,
                } => {
                    assert_eq!(case_ref, "case-abc");
                    assert_eq!(definition.definition_url, "urn:wos:workflow:intake-demo:1");
                    assert_eq!(
                        initial_case_state
                            .as_ref()
                            .and_then(|v| v.get("applicantId"))
                            .and_then(|v| v.as_str()),
                        Some("a-1"),
                    );
                }
                other => panic!("unexpected disposition: {other:?}"),
            },
            other => panic!("unexpected outcome: {other:?}"),
        },
        other => panic!("expected Intake replay value, got {other:?}"),
    }
}

/// F4: an intake record created on one StorageBackedRuntimeStore handle must be
/// observable on a fresh handle backed by the same SQLite database.
/// Previously the record lived in an in-memory HashMap tied to the store
/// handle, so process restart (or any new store instance) dropped it.
#[tokio::test]
async fn intake_records_persist_across_store_handles() {
    let storage = fresh().await;
    let provenance = Arc::new(ProvenanceService::new(storage.clone()));
    let handle = tokio::runtime::Handle::current();

    let original = sample_intake_record("formspecIntake", "intake-42");

    let storage_c = storage.clone();
    let provenance_c = provenance.clone();
    let audit_c = Arc::new(NoopAuditSink);
    let handle_c = handle.clone();
    let original_c = original.clone();
    tokio::task::spawn_blocking(move || {
        let mut store = StorageBackedRuntimeStore::new(storage_c, provenance_c, audit_c, handle_c);
        store
            .create_intake_record(original_c)
            .expect("create intake");
    })
    .await
    .unwrap();

    let storage_c = storage.clone();
    let provenance_c = provenance.clone();
    let audit_c = Arc::new(NoopAuditSink);
    let handle_c = handle.clone();
    let loaded = tokio::task::spawn_blocking(move || {
        let store = StorageBackedRuntimeStore::new(storage_c, provenance_c, audit_c, handle_c);
        store
            .load_intake_record("formspecIntake", "intake-42")
            .expect("load intake after restart")
    })
    .await
    .unwrap();
    assert_eq!(loaded.binding, "formspecIntake");
    assert_eq!(loaded.intake_id, "intake-42");
    assert_eq!(loaded.request, original.request);
    assert_eq!(loaded.status, original.status);

    let storage_c = storage.clone();
    let provenance_c = provenance.clone();
    let audit_c = Arc::new(NoopAuditSink);
    let handle_c = handle.clone();
    let mut updated = loaded.clone();
    updated.status = IntakeRecordStatus::Applied;
    let updated_clone = updated.clone();
    tokio::task::spawn_blocking(move || {
        let mut store = StorageBackedRuntimeStore::new(storage_c, provenance_c, audit_c, handle_c);
        store
            .save_intake_record(updated_clone)
            .expect("save intake");
    })
    .await
    .unwrap();

    let storage_c = storage.clone();
    let provenance_c = provenance.clone();
    let audit_c = Arc::new(NoopAuditSink);
    let handle_c = handle.clone();
    let reloaded = tokio::task::spawn_blocking(move || {
        let store = StorageBackedRuntimeStore::new(storage_c, provenance_c, audit_c, handle_c);
        store
            .load_intake_record("formspecIntake", "intake-42")
            .expect("reload intake")
    })
    .await
    .unwrap();
    assert_eq!(reloaded.status, IntakeRecordStatus::Applied);
}

/// F5: NotFound errors from the underlying storage must carry the resource
/// id so logs and responses can identify which record was missing. A missing
/// intake record must surface its `(binding, intake_id)` in the error.
#[tokio::test]
async fn not_found_error_preserves_resource_id() {
    let storage = fresh().await;
    let provenance = Arc::new(ProvenanceService::new(storage.clone()));
    let audit = Arc::new(NoopAuditSink);
    let handle = tokio::runtime::Handle::current();

    let err = tokio::task::spawn_blocking(move || {
        let store = StorageBackedRuntimeStore::new(storage, provenance, audit, handle);
        store
            .load_intake_record("formspecIntake", "missing-77")
            .unwrap_err()
    })
    .await
    .unwrap();

    match err {
        StoreError::NotFound(ctx) => {
            assert!(
                ctx.contains("missing-77") && ctx.contains("formspecIntake"),
                "NotFound should carry binding + intake_id, got: {ctx}"
            );
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
}
