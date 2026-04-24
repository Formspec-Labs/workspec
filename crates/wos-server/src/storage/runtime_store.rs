//! `wos_runtime::RuntimeStore` impl over the server's async `Storage` trait.
//!
//! `RuntimeStore` itself is synchronous. To bridge to our async storage we
//! call `tokio::runtime::Handle::block_on` inside each method. This is safe
//! as long as callers are on a thread that can block — in practice, the
//! server always invokes `WosRuntime` from inside `tokio::task::spawn_blocking`
//! via the [`AppRuntime`](crate::runtime::AppRuntime) wrapper. Calling these
//! methods directly from a tokio async worker will panic.

use std::sync::Arc;

use chrono::Utc;
use tokio::runtime::Handle;
use wos_core::instance::CaseInstance;
use wos_core::provenance::ProvenanceRecord;
use wos_runtime::store::{
    IntakeRecord, ReplayKey, ReplayOperation, ReplayValue, RuntimeRecord, RuntimeStore,
    StepResultRecord, StoreError, TaskArtifact, TaskArtifactKind,
};
use wos_runtime::{PersistDraftResult, TaskSubmissionResult};

use super::{InstanceRow, IntakeRecordRow, StorageHandle};
use crate::services::provenance_service::ProvenanceService;

pub struct SqliteRuntimeStore {
    storage: StorageHandle,
    provenance: Arc<ProvenanceService>,
    handle: Handle,
}

impl SqliteRuntimeStore {
    pub fn new(
        storage: StorageHandle,
        provenance: Arc<ProvenanceService>,
        handle: Handle,
    ) -> Self {
        Self {
            storage,
            provenance,
            handle,
        }
    }
}

fn intake_status_str(s: &wos_runtime::intake::IntakeRecordStatus) -> &'static str {
    use wos_runtime::intake::IntakeRecordStatus::*;
    match s {
        Pending => "pending",
        Prepared => "prepared",
        Applied => "applied",
    }
}

impl RuntimeStore for SqliteRuntimeStore {
    fn create_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
        let instance = record.instance.clone();
        let instance_json = serde_json::to_value(&instance)
            .map_err(|e| StoreError::Failed(format!("serialise instance: {e}")))?;
        let aux_json = aux_to_json(&record);
        let impact_level = impact_level_hint(&instance);
        let now = Utc::now();

        let row = InstanceRow {
            instance_id: instance.instance_id.clone(),
            definition_url: instance.definition_url.clone(),
            definition_version: instance.definition_version.clone(),
            status: status_str(&instance.status).to_string(),
            impact_level,
            instance_json,
            runtime_aux_json: aux_json,
            created_at: now,
            updated_at: now,
        };

        let storage = self.storage.clone();
        let provenance = self.provenance.clone();
        let log_snapshot = record.provenance_log.clone();
        let instance_id = row.instance_id.clone();

        self.handle.block_on(async move {
            if storage
                .get_instance(&instance_id)
                .await
                .map_err(storage_err)?
                .is_some()
            {
                return Err(StoreError::AlreadyExists(instance_id));
            }
            storage.create_instance(&row).await.map_err(storage_err)?;

            if !log_snapshot.is_empty() {
                let rows = provenance
                    .prepare_batch(&instance_id, &log_snapshot)
                    .await
                    .map_err(storage_err)?;
                let update_ctx = instance_id.clone();
                storage
                    .update_instance_atomic(
                        &instance_id,
                        &move |_current| Ok(rows.clone()),
                    )
                    .await
                    .map_err(|e| storage_err_with(e, &update_ctx))?;
            }
            Ok(())
        })
    }

    fn load_record(&self, instance_id: &str) -> Result<RuntimeRecord, StoreError> {
        let storage = self.storage.clone();
        let id = instance_id.to_string();
        self.handle.block_on(async move {
            let (row_opt, prov_rows) = tokio::try_join!(
                storage.get_instance(&id),
                storage.list_provenance(&id),
            )
            .map_err(storage_err)?;
            let row = row_opt.ok_or_else(|| StoreError::NotFound(id.clone()))?;
            let instance: CaseInstance = serde_json::from_value(row.instance_json.clone())
                .map_err(|e| StoreError::Failed(format!("deserialise instance: {e}")))?;
            let provenance_log: Vec<ProvenanceRecord> = prov_rows
                .iter()
                .map(|r| serde_json::from_value::<ProvenanceRecord>(r.payload.clone()))
                .collect::<Result<_, _>>()
                .map_err(|e| StoreError::Failed(format!("deserialise provenance: {e}")))?;
            let aux = aux_from_json(&row.runtime_aux_json);
            Ok(RuntimeRecord {
                instance,
                provenance_log,
                step_results: aux.step_results,
                artifacts: aux.artifacts,
                replay_entries: aux.replay_entries,
            })
        })
    }

    fn save_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
        let instance = record.instance.clone();
        let instance_json = serde_json::to_value(&instance)
            .map_err(|e| StoreError::Failed(format!("serialise instance: {e}")))?;
        let aux_json = aux_to_json(&record);
        let impact_level = impact_level_hint(&instance);
        let status = status_str(&instance.status).to_string();
        let log_snapshot = record.provenance_log.clone();
        let instance_id = instance.instance_id.clone();

        let storage = self.storage.clone();
        let provenance = self.provenance.clone();
        self.handle.block_on(async move {
            let stored = storage
                .list_provenance(&instance_id)
                .await
                .map_err(storage_err)?;
            let stored_len = stored.len();
            let new_tail: Vec<ProvenanceRecord> =
                log_snapshot.iter().skip(stored_len).cloned().collect();
            let appended_rows = if new_tail.is_empty() {
                Vec::new()
            } else {
                provenance
                    .prepare_batch(&instance_id, &new_tail)
                    .await
                    .map_err(storage_err)?
            };
            let instance_json_shared = std::sync::Arc::new(instance_json);
            let aux_json_shared = std::sync::Arc::new(aux_json);
            let appended_rows_shared = std::sync::Arc::new(appended_rows);
            let update_ctx = instance_id.clone();
            storage
                .update_instance_atomic(&instance_id, &move |current| {
                    current.instance_json = (*instance_json_shared).clone();
                    current.runtime_aux_json = (*aux_json_shared).clone();
                    current.status = status.clone();
                    current.impact_level = impact_level.clone();
                    Ok((*appended_rows_shared).clone())
                })
                .await
                .map_err(|e| storage_err_with(e, &update_ctx))?;
            Ok(())
        })
    }

    fn create_intake_record(&mut self, record: IntakeRecord) -> Result<(), StoreError> {
        let row = build_intake_row(&record)?;
        let storage = self.storage.clone();
        let ctx = format!("intake:{}:{}", record.binding, record.intake_id);
        self.handle
            .block_on(async move { storage.insert_intake_record(&row).await })
            .map_err(|e| storage_err_with(e, &ctx))
    }

    fn load_intake_record(
        &self,
        binding: &str,
        intake_id: &str,
    ) -> Result<IntakeRecord, StoreError> {
        let storage = self.storage.clone();
        let binding = binding.to_string();
        let intake_id = intake_id.to_string();
        let ctx = format!("intake:{binding}:{intake_id}");
        let row_opt = self
            .handle
            .block_on(async move { storage.get_intake_record(&binding, &intake_id).await })
            .map_err(|e| storage_err_with(e, &ctx))?;
        let row = row_opt.ok_or_else(|| StoreError::NotFound(ctx.clone()))?;
        serde_json::from_value(row.record_json)
            .map_err(|e| StoreError::Failed(format!("deserialise {ctx}: {e}")))
    }

    fn save_intake_record(&mut self, record: IntakeRecord) -> Result<(), StoreError> {
        let row = build_intake_row(&record)?;
        let storage = self.storage.clone();
        let ctx = format!("intake:{}:{}", record.binding, record.intake_id);
        self.handle
            .block_on(async move { storage.update_intake_record(&row).await })
            .map_err(|e| storage_err_with(e, &ctx))
    }
}

fn build_intake_row(record: &IntakeRecord) -> Result<IntakeRecordRow, StoreError> {
    let record_json = serde_json::to_value(record)
        .map_err(|e| StoreError::Failed(format!("serialise intake record: {e}")))?;
    let now = Utc::now();
    Ok(IntakeRecordRow {
        binding: record.binding.clone(),
        intake_id: record.intake_id.clone(),
        status: intake_status_str(&record.status).to_string(),
        record_json,
        created_at: now,
        updated_at: now,
    })
}

fn storage_err(e: crate::storage::StorageError) -> StoreError {
    storage_err_with(e, "")
}

fn storage_err_with(e: crate::storage::StorageError, ctx: &str) -> StoreError {
    match e {
        crate::storage::StorageError::NotFound => StoreError::NotFound(ctx.to_string()),
        crate::storage::StorageError::Conflict(m) => StoreError::AlreadyExists(m),
        other => StoreError::Failed(other.to_string()),
    }
}

fn status_str(s: &wos_core::instance::InstanceStatus) -> &'static str {
    use wos_core::instance::InstanceStatus::*;
    match s {
        Active => "active",
        Suspended => "suspended",
        Migrating => "migrating",
        Completed => "completed",
        Terminated => "terminated",
    }
}

fn impact_level_hint(inst: &CaseInstance) -> String {
    inst.case_state
        .get("__impactLevel")
        .and_then(|v| v.as_str())
        .unwrap_or("operational")
        .to_string()
}

// ── Aux field (de)serialisation ────────────────────────────────────────

pub(super) struct DecodedAux {
    pub step_results: Vec<StepResultRecord>,
    pub artifacts: std::collections::HashMap<String, TaskArtifact>,
    pub replay_entries: std::collections::HashMap<ReplayKey, ReplayValue>,
}

pub(super) fn aux_to_json(record: &RuntimeRecord) -> serde_json::Value {
    let step_results: Vec<_> = record
        .step_results
        .iter()
        .map(|s| {
            serde_json::json!({
                "serviceRef": s.service_ref,
                "idempotencyKey": s.idempotency_key,
                "output": s.output,
                "recordedAt": s.recorded_at,
            })
        })
        .collect();
    let artifacts: serde_json::Map<String, serde_json::Value> = record
        .artifacts
        .iter()
        .map(|(k, a)| {
            let kind = match a.kind {
                TaskArtifactKind::Draft => "draft",
                TaskArtifactKind::Accepted => "accepted",
            };
            (
                k.clone(),
                serde_json::json!({
                    "artifactId": a.artifact_id,
                    "taskId": a.task_id,
                    "kind": kind,
                    "response": a.response,
                    "actorId": a.actor_id,
                    "recordedAt": a.recorded_at,
                }),
            )
        })
        .collect();
    let replay_entries: Vec<serde_json::Value> = record
        .replay_entries
        .iter()
        .map(|(k, v)| {
            let op = match k.operation {
                ReplayOperation::PersistDraft => "persistDraft",
                ReplayOperation::SubmitTaskResponse => "submitTaskResponse",
                ReplayOperation::AcceptIntakeHandoff => "acceptIntakeHandoff",
            };
            let value_json = match v {
                ReplayValue::Draft(d) => serde_json::json!({
                        "kind": "draft",
                        "artifactId": d.artifact_id,
                    }),
                ReplayValue::Submission(s) => match s {
                    TaskSubmissionResult::Completed {
                        artifact_id,
                        case_mutated,
                        emitted_event,
                    } => serde_json::json!({
                        "kind": "completed",
                        "artifactId": artifact_id,
                        "caseMutated": case_mutated,
                        "emittedEvent": emitted_event,
                    }),
                    TaskSubmissionResult::Failed {
                        code,
                        emitted_event,
                    } => serde_json::json!({
                        "kind": "failed",
                        "code": code,
                        "emittedEvent": emitted_event,
                    }),
                    TaskSubmissionResult::Rejected { code } => {
                        serde_json::json!({ "kind": "rejected", "code": code })
                    }
                },
                ReplayValue::Intake(decision) => {
                    let decision_json = serde_json::to_value(decision).unwrap_or(serde_json::json!({}));
                    serde_json::json!({
                        "kind": "intake",
                        "decision": decision_json,
                    })
                }
            };
            serde_json::json!({
                "key": {
                    "operation": op,
                    "taskId": k.task_id,
                    "actorId": k.actor_id,
                    "token": k.token,
                },
                "value": value_json,
            })
        })
        .collect();
    serde_json::json!({
        "stepResults": step_results,
        "artifacts": artifacts,
        "replayEntries": replay_entries,
    })
}

pub(super) fn aux_from_json(v: &serde_json::Value) -> DecodedAux {
    let step_results = v
        .get("stepResults")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| {
                    Some(StepResultRecord {
                        service_ref: s.get("serviceRef")?.as_str()?.to_string(),
                        idempotency_key: s
                            .get("idempotencyKey")
                            .and_then(|x| x.as_str())
                            .map(String::from),
                        output: s.get("output").cloned().unwrap_or(serde_json::json!({})),
                        recorded_at: s
                            .get("recordedAt")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let artifacts = v
        .get("artifacts")
        .and_then(|x| x.as_object())
        .map(|m| {
            m.iter()
                .filter_map(|(k, a)| {
                    let kind = match a.get("kind").and_then(|x| x.as_str())? {
                        "draft" => TaskArtifactKind::Draft,
                        "accepted" => TaskArtifactKind::Accepted,
                        _ => return None,
                    };
                    Some((
                        k.clone(),
                        TaskArtifact {
                            artifact_id: a.get("artifactId")?.as_str()?.to_string(),
                            task_id: a.get("taskId")?.as_str()?.to_string(),
                            kind,
                            response: a.get("response").cloned().unwrap_or(serde_json::json!({})),
                            actor_id: a
                                .get("actorId")
                                .and_then(|x| x.as_str())
                                .unwrap_or("")
                                .to_string(),
                            recorded_at: a
                                .get("recordedAt")
                                .and_then(|x| x.as_str())
                                .unwrap_or("")
                                .to_string(),
                        },
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    let replay_entries = v
        .get("replayEntries")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|entry| {
                    let k = entry.get("key")?;
                    let v = entry.get("value")?;
                    let op = match k.get("operation").and_then(|x| x.as_str())? {
                        "persistDraft" => ReplayOperation::PersistDraft,
                        "submitTaskResponse" => ReplayOperation::SubmitTaskResponse,
                        "acceptIntakeHandoff" => ReplayOperation::AcceptIntakeHandoff,
                        _ => return None,
                    };
                    let key = ReplayKey {
                        operation: op,
                        task_id: k.get("taskId")?.as_str()?.to_string(),
                        actor_id: k.get("actorId")?.as_str()?.to_string(),
                        token: k.get("token")?.as_str()?.to_string(),
                    };
                    let value = match v.get("kind").and_then(|x| x.as_str())? {
                        "draft" => ReplayValue::Draft(PersistDraftResult {
                            artifact_id: v.get("artifactId")?.as_str()?.to_string(),
                        }),
                        "completed" => ReplayValue::Submission(TaskSubmissionResult::Completed {
                            artifact_id: v.get("artifactId")?.as_str()?.to_string(),
                            case_mutated: v
                                .get("caseMutated")
                                .and_then(|x| x.as_bool())
                                .unwrap_or(false),
                            emitted_event: v
                                .get("emittedEvent")
                                .and_then(|x| x.as_str())
                                .map(String::from),
                        }),
                        "failed" => ReplayValue::Submission(TaskSubmissionResult::Failed {
                            code: v.get("code")?.as_str()?.to_string(),
                            emitted_event: v
                                .get("emittedEvent")
                                .and_then(|x| x.as_str())
                                .map(String::from),
                        }),
                        "rejected" => ReplayValue::Submission(TaskSubmissionResult::Rejected {
                            code: v.get("code")?.as_str()?.to_string(),
                        }),
                        "intake" => {
                            let decision_val = v.get("decision")?.clone();
                            let decision: wos_runtime::intake::IntakeAcceptanceDecision =
                                serde_json::from_value(decision_val).ok()?;
                            ReplayValue::Intake(decision)
                        }
                        _ => return None,
                    };
                    Some((key, value))
                })
                .collect()
        })
        .unwrap_or_default();
    DecodedAux {
        step_results,
        artifacts,
        replay_entries,
    }
}
