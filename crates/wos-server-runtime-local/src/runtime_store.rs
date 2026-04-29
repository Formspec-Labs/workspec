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
use wos_server_ports::audit::AuditSink;
use wos_server_ports::runtime::ProvenancePort;
use wos_server_ports::storage::{InstanceRow, IntakeRecordRow, StorageError, StorageHandle};

pub struct StorageBackedRuntimeStore {
    storage: StorageHandle,
    provenance: Arc<dyn ProvenancePort>,
    audit_sink: Arc<dyn AuditSink>,
    handle: Handle,
}

impl StorageBackedRuntimeStore {
    pub fn new(
        storage: StorageHandle,
        provenance: Arc<dyn ProvenancePort>,
        audit_sink: Arc<dyn AuditSink>,
        handle: Handle,
    ) -> Self {
        Self {
            storage,
            provenance,
            audit_sink,
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

impl RuntimeStore for StorageBackedRuntimeStore {
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
        let audit_sink = self.audit_sink.clone();
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
                    .map_err(|e| StoreError::Failed(e.to_string()))?;
                let rows_for_mutator = rows.clone();
                let update_ctx = instance_id.clone();
                storage
                    .update_instance_atomic(
                        &instance_id,
                        &move |_current| Ok(rows_for_mutator.clone()),
                    )
                    .await
                    .map_err(|e| storage_err_with(e, &update_ctx))?;
                if let Err(e) = audit_sink.append_provenance(&rows).await {
                    tracing::warn!(
                        instance_id = %instance_id,
                        error = %e,
                        "audit sink append failed after operational commit; continuing",
                    );
                }
            }
            Ok(())
        })
    }

    fn load_record(&self, instance_id: &str) -> Result<RuntimeRecord, StoreError> {
        let storage = self.storage.clone();
        let id = instance_id.to_string();
        self.handle.block_on(async move {
            let (row_opt, prov_rows) =
                tokio::try_join!(storage.get_instance(&id), storage.list_provenance(&id),)
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
        let audit_sink = self.audit_sink.clone();
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
                    .map_err(|e| StoreError::Failed(e.to_string()))?
            };
            let instance_json_shared = std::sync::Arc::new(instance_json);
            let aux_json_shared = std::sync::Arc::new(aux_json);
            let appended_rows_shared = std::sync::Arc::new(appended_rows);
            let appended_rows_for_mutator = appended_rows_shared.clone();
            let update_ctx = instance_id.clone();
            storage
                .update_instance_atomic(&instance_id, &move |current| {
                    current.instance_json = (*instance_json_shared).clone();
                    current.runtime_aux_json = (*aux_json_shared).clone();
                    current.status = status.clone();
                    current.impact_level = impact_level.clone();
                    Ok((*appended_rows_for_mutator).clone())
                })
                .await
                .map_err(|e| storage_err_with(e, &update_ctx))?;
            if !appended_rows_shared.is_empty() {
                if let Err(e) = audit_sink.append_provenance(appended_rows_shared.as_ref()).await {
                    tracing::warn!(
                        instance_id = %instance_id,
                        error = %e,
                        "audit sink append failed after operational commit; continuing",
                    );
                }
            }
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

fn storage_err(e: StorageError) -> StoreError {
    storage_err_with(e, "")
}

fn storage_err_with(e: StorageError, ctx: &str) -> StoreError {
    match e {
        StorageError::NotFound => StoreError::NotFound(ctx.to_string()),
        StorageError::Conflict(m) => StoreError::AlreadyExists(m),
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
        Stalled => "stalled",
    }
}

fn impact_level_hint(inst: &CaseInstance) -> String {
    inst.case_state
        .get("__impactLevel")
        .and_then(|v| v.as_str())
        .unwrap_or("operational")
        .to_string()
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use wos_core::instance::{CaseInstance, InstanceStatus};
    use wos_core::provenance::ProvenanceRecord;
    use wos_runtime::store::{RuntimeRecord, RuntimeStore, StepResultRecord};
    use wos_server_ports::audit::{AuditError, AuditResult, AuditSink, ExportEnvelope};
    use wos_server_ports::runtime::RuntimeResult;
    use wos_server_ports::storage::{ProvenanceRow, Storage};

    #[derive(Debug)]
    struct FailingAuditSink {
        calls: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl AuditSink for FailingAuditSink {
        async fn append_provenance(&self, _records: &[ProvenanceRow]) -> AuditResult<()> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Err(AuditError::Backend("intentional audit sink failure".into()))
        }

        async fn append_export(&self, _envelope: ExportEnvelope) -> AuditResult<()> {
            Ok(())
        }
    }

    struct PassthroughProvenancePort;

    #[async_trait::async_trait]
    impl wos_server_ports::runtime::ProvenancePort for PassthroughProvenancePort {
        async fn prepare_batch(
            &self,
            instance_id: &str,
            records: &[ProvenanceRecord],
        ) -> RuntimeResult<Vec<ProvenanceRow>> {
            Ok(records
                .iter()
                .enumerate()
                .map(|(idx, record)| ProvenanceRow {
                    id: format!("{instance_id}-{}", idx + 1),
                    instance_id: instance_id.to_string(),
                    seq: (idx as i64) + 1,
                    timestamp: chrono::Utc::now(),
                    tier: "facts".into(),
                    payload: serde_json::to_value(record).expect("serialise provenance"),
                    hash: format!("hash-{}", idx + 1),
                    previous_hash: if idx == 0 {
                        "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                            .into()
                    } else {
                        format!("hash-{idx}")
                    },
                })
                .collect())
        }
    }

    fn minimal_case_instance() -> CaseInstance {
        CaseInstance {
            instance_id: "case_01".into(),
            definition_url: "urn:wos:test:workflow".into(),
            definition_version: "1.0.0".into(),
            configuration: vec!["intake".into()],
            case_state: serde_json::json!({"__impactLevel":"operational"}),
            provenance_position: 0,
            next_task_sequence: 1,
            timers: Vec::new(),
            active_tasks: Vec::new(),
            history_store: HashMap::new(),
            compensation_logs: HashMap::new(),
            status: InstanceStatus::Active,
            stalled_since: None,
            pending_events: Vec::new(),
            governance_state: None,
            volume_counters: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            fired_milestones: HashSet::new(),
            pending_callbacks: HashMap::new(),
            extensions: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn save_record_succeeds_when_audit_sink_fails() {
        let temp_db = tempfile::NamedTempFile::new().expect("temp db");
        let dsn = format!("sqlite://{}", temp_db.path().display());
        let storage = Arc::new(
            wos_server_sqlite::SqliteStorage::connect(&dsn)
                .await
                .expect("sqlite connect"),
        );
        storage.migrate().await.expect("sqlite migrate");
        let provenance = Arc::new(PassthroughProvenancePort);
        let audit = Arc::new(FailingAuditSink {
            calls: AtomicUsize::new(0),
        });
        let handle = tokio::runtime::Handle::current();
        let mut store = StorageBackedRuntimeStore::new(storage.clone(), provenance, audit.clone(), handle);

        let mut record = RuntimeRecord::new(minimal_case_instance());
        tokio::task::spawn_blocking(move || {
            store.create_record(record.clone()).expect("create record");
            record.step_results.push(StepResultRecord {
                service_ref: "service.ref".into(),
                idempotency_key: Some("idempotency-1".into()),
                output: serde_json::json!({"ok": true}),
                recorded_at: "2026-01-01T00:00:01Z".into(),
            });
            record.provenance_log.push(ProvenanceRecord::state_transition(
                "intake",
                "review",
                "submit",
                Some("sys"),
            ));
            store
                .save_record(record.clone())
                .expect("save record should not fail");
            record
        })
        .await
        .expect("save task join");

        assert_eq!(audit.calls.load(Ordering::SeqCst), 1);
        let persisted = storage
            .get_instance("case_01")
            .await
            .expect("instance read")
            .expect("instance exists");
        assert_eq!(persisted.status, "active");
    }

    #[test]
    fn aux_json_roundtrip_preserves_step_results() {
        let mut record = RuntimeRecord::new(minimal_case_instance());
        record.step_results.push(StepResultRecord {
            service_ref: "service.ref".into(),
            idempotency_key: Some("idempotency-1".into()),
            output: serde_json::json!({"value": 7}),
            recorded_at: "2026-01-01T00:00:01Z".into(),
        });

        let aux = aux_to_json(&record);
        let decoded = aux_from_json(&aux);
        assert_eq!(decoded.step_results.len(), 1);
        assert_eq!(decoded.step_results[0].service_ref, "service.ref");
        assert_eq!(decoded.step_results[0].idempotency_key.as_deref(), Some("idempotency-1"));
        assert_eq!(decoded.step_results[0].output["value"], 7);
    }
}
