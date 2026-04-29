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
    // INVARIANT: `create_record` is a two-phase write across two separate
    // SQLite transactions:
    //   1. `storage.create_instance(&row)` — inserts the instance row.
    //   2. `storage.update_instance_atomic(...)` — appends the seed
    //      provenance batch (skipped when `log_snapshot.is_empty()`).
    //
    // Failure mode: a process crash *between* step 1 and step 2 leaves an
    // instance row with zero provenance rows on disk. There is no recovery
    // path today — `load_record` will deserialize the instance and return an
    // empty `provenance_log`, silently presenting a non-bootstrapped instance
    // as valid.
    //
    // Recovery posture: this is a single-process dev/SBA target. Crash
    // recovery is intentionally unimplemented until durable execution lands
    // (Restate adapter, tracked as WS-094). Until then, two-phase exposure
    // is acceptable because process restart = workflow restart.
    //
    // What "done" looks like for WS-094: either (a) fold both writes into a
    // single atomic transaction (preferred — `update_instance_atomic` already
    // handles the row update + provenance append in one tx, so the create
    // path could insert an empty row first and let the same atomic update
    // attach provenance), or (b) make `create_record` idempotent on load by
    // checking provenance count and re-running step 2 from `record` if zero.
    // Either path closes the window; the choice depends on whether Restate
    // owns crash recovery by the time WS-094 is scheduled.
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

    /// Replay-entry roundtrip: every `ReplayValue` variant the runtime emits.
    ///
    /// Covers PersistDraft (Draft), SubmitTaskResponse (Completed/Failed/Rejected),
    /// and AcceptIntakeHandoff (Intake). Drift here silently corrupts replay
    /// reconciliation after a process restart — the `_ => None` arm in
    /// `aux_from_json` would drop unknown kinds without trace.
    ///
    /// Each variant exercised here proves a specific arm in `aux_from_json` is
    /// reachable and field-preserving; together they prove the `_ => None`
    /// arm is unreachable for any variant the runtime can construct today.
    /// Adding a new `ReplayValue` variant without extending this test will
    /// pass compile but silently fall through `_ => None` at runtime — extend
    /// this fixture as part of any such addition.
    #[test]
    fn aux_json_roundtrip_preserves_replay_entries() {
        use wos_runtime::intake::{
            IntakeAcceptanceDecision, IntakeAcceptanceOutcome, IntakeCaseDisposition,
        };
        use wos_runtime::store::{ReplayKey, ReplayOperation, ReplayValue};

        let mut record = RuntimeRecord::new(minimal_case_instance());

        // Variant 1: Draft (persistDraft)
        record.replay_entries.insert(
            ReplayKey {
                operation: ReplayOperation::PersistDraft,
                task_id: "task-1".into(),
                actor_id: "actor-a".into(),
                token: "tok-1".into(),
            },
            ReplayValue::Draft(PersistDraftResult {
                artifact_id: "artifact-draft-1".into(),
            }),
        );

        // Variant 2: Submission(Completed)
        record.replay_entries.insert(
            ReplayKey {
                operation: ReplayOperation::SubmitTaskResponse,
                task_id: "task-2".into(),
                actor_id: "actor-b".into(),
                token: "tok-2".into(),
            },
            ReplayValue::Submission(TaskSubmissionResult::Completed {
                artifact_id: "artifact-2".into(),
                case_mutated: true,
                emitted_event: Some("event.completed".into()),
            }),
        );

        // Variant 3: Submission(Failed) — code + emittedEvent must round-trip
        record.replay_entries.insert(
            ReplayKey {
                operation: ReplayOperation::SubmitTaskResponse,
                task_id: "task-3".into(),
                actor_id: "actor-c".into(),
                token: "tok-3".into(),
            },
            ReplayValue::Submission(TaskSubmissionResult::Failed {
                code: "validation.failed".into(),
                emitted_event: Some("event.failed".into()),
            }),
        );

        // Variant 4: Submission(Rejected) — code-only
        record.replay_entries.insert(
            ReplayKey {
                operation: ReplayOperation::SubmitTaskResponse,
                task_id: "task-4".into(),
                actor_id: "actor-d".into(),
                token: "tok-4".into(),
            },
            ReplayValue::Submission(TaskSubmissionResult::Rejected {
                code: "policy.denied".into(),
            }),
        );

        // Variant 5: Intake — full IntakeAcceptanceDecision through serde
        let intake_decision = IntakeAcceptanceDecision {
            outcome: IntakeAcceptanceOutcome::Accepted {
                case_disposition: IntakeCaseDisposition::AttachToExistingCase {
                    case_ref: "case_existing_1".into(),
                },
            },
            provenance: vec![ProvenanceRecord::state_transition(
                "intake",
                "review",
                "accept",
                Some("intake-binding"),
            )],
        };
        record.replay_entries.insert(
            ReplayKey {
                operation: ReplayOperation::AcceptIntakeHandoff,
                task_id: "intake-task-1".into(),
                actor_id: "intake-actor".into(),
                token: "intake-tok-1".into(),
            },
            ReplayValue::Intake(intake_decision.clone()),
        );

        let aux = aux_to_json(&record);
        let decoded = aux_from_json(&aux);
        assert_eq!(decoded.replay_entries.len(), 5);

        let draft_key = ReplayKey {
            operation: ReplayOperation::PersistDraft,
            task_id: "task-1".into(),
            actor_id: "actor-a".into(),
            token: "tok-1".into(),
        };
        match decoded.replay_entries.get(&draft_key) {
            Some(ReplayValue::Draft(d)) => assert_eq!(d.artifact_id, "artifact-draft-1"),
            other => panic!("expected Draft replay value, got {other:?}"),
        }

        let submit_key = ReplayKey {
            operation: ReplayOperation::SubmitTaskResponse,
            task_id: "task-2".into(),
            actor_id: "actor-b".into(),
            token: "tok-2".into(),
        };
        match decoded.replay_entries.get(&submit_key) {
            Some(ReplayValue::Submission(TaskSubmissionResult::Completed {
                artifact_id,
                case_mutated,
                emitted_event,
            })) => {
                assert_eq!(artifact_id, "artifact-2");
                assert!(*case_mutated);
                assert_eq!(emitted_event.as_deref(), Some("event.completed"));
            }
            other => panic!("expected Completed submission, got {other:?}"),
        }

        let failed_key = ReplayKey {
            operation: ReplayOperation::SubmitTaskResponse,
            task_id: "task-3".into(),
            actor_id: "actor-c".into(),
            token: "tok-3".into(),
        };
        match decoded.replay_entries.get(&failed_key) {
            Some(ReplayValue::Submission(TaskSubmissionResult::Failed {
                code,
                emitted_event,
            })) => {
                assert_eq!(code, "validation.failed");
                assert_eq!(emitted_event.as_deref(), Some("event.failed"));
            }
            other => panic!("expected Failed submission, got {other:?}"),
        }

        let rejected_key = ReplayKey {
            operation: ReplayOperation::SubmitTaskResponse,
            task_id: "task-4".into(),
            actor_id: "actor-d".into(),
            token: "tok-4".into(),
        };
        match decoded.replay_entries.get(&rejected_key) {
            Some(ReplayValue::Submission(TaskSubmissionResult::Rejected { code })) => {
                assert_eq!(code, "policy.denied");
            }
            other => panic!("expected Rejected submission, got {other:?}"),
        }

        let intake_key = ReplayKey {
            operation: ReplayOperation::AcceptIntakeHandoff,
            task_id: "intake-task-1".into(),
            actor_id: "intake-actor".into(),
            token: "intake-tok-1".into(),
        };
        match decoded.replay_entries.get(&intake_key) {
            Some(ReplayValue::Intake(decoded_decision)) => {
                // Roundtrip via serde JSON canonical form — equality on the
                // serialized shape is the contract that survives field
                // additions / reorderings on `IntakeAcceptanceDecision`.
                let original_json = serde_json::to_value(&intake_decision)
                    .expect("serialise original intake decision");
                let decoded_json = serde_json::to_value(decoded_decision)
                    .expect("serialise decoded intake decision");
                assert_eq!(original_json, decoded_json);
            }
            other => panic!("expected Intake replay value, got {other:?}"),
        }
    }

    /// `create_record` must persist the instance plus seed-batch provenance and
    /// emit them through the audit sink in the same call.
    ///
    /// This locks the "append on create" semantics — without it a freshly-created
    /// instance with bootstrap provenance would be silently dropped, and the audit
    /// fan-out would only fire on the next `save_record`.
    #[tokio::test]
    async fn create_record_persists_instance_and_seeds_audit_sink() {
        #[derive(Debug, Default)]
        struct CountingAuditSink {
            calls: AtomicUsize,
            rows: Mutex<Vec<ProvenanceRow>>,
        }

        #[async_trait::async_trait]
        impl AuditSink for CountingAuditSink {
            async fn append_provenance(&self, records: &[ProvenanceRow]) -> AuditResult<()> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                self.rows.lock().unwrap().extend_from_slice(records);
                Ok(())
            }

            async fn append_export(&self, _envelope: ExportEnvelope) -> AuditResult<()> {
                Ok(())
            }
        }

        use std::sync::Mutex;

        let temp_db = tempfile::NamedTempFile::new().expect("temp db");
        let dsn = format!("sqlite://{}", temp_db.path().display());
        let storage = Arc::new(
            wos_server_sqlite::SqliteStorage::connect(&dsn)
                .await
                .expect("sqlite connect"),
        );
        storage.migrate().await.expect("sqlite migrate");
        let provenance = Arc::new(PassthroughProvenancePort);
        let audit = Arc::new(CountingAuditSink::default());
        let handle = tokio::runtime::Handle::current();
        let mut store =
            StorageBackedRuntimeStore::new(storage.clone(), provenance, audit.clone(), handle);

        let mut record = RuntimeRecord::new(minimal_case_instance());
        record.provenance_log.push(ProvenanceRecord::state_transition(
            "intake",
            "review",
            "submit",
            Some("sys"),
        ));

        let storage_for_check = storage.clone();
        let audit_for_check = audit.clone();
        tokio::task::spawn_blocking(move || {
            store.create_record(record).expect("create record");
        })
        .await
        .expect("create task join");

        // Audit sink saw exactly one append call carrying the seed record.
        assert_eq!(audit_for_check.calls.load(Ordering::SeqCst), 1);
        assert_eq!(audit_for_check.rows.lock().unwrap().len(), 1);

        // Operational projection has the instance.
        let row = storage_for_check
            .get_instance("case_01")
            .await
            .expect("instance read")
            .expect("instance exists");
        assert_eq!(row.status, "active");

        // Provenance was appended in the same operational transaction.
        let provenance_rows = storage_for_check
            .list_provenance("case_01")
            .await
            .expect("provenance read");
        assert_eq!(provenance_rows.len(), 1);
        assert_eq!(provenance_rows[0].seq, 1);
    }

    /// `create_record` fast path: when `provenance_log` is empty, the
    /// `update_instance_atomic` + `audit_sink.append_provenance` pair is
    /// skipped entirely, but the instance row itself still lands.
    ///
    /// This locks the empty-log branch — without it, an instance created with
    /// no bootstrap provenance could either spuriously call the audit sink
    /// (waste + spurious export entries) or fail to persist the instance row
    /// (broken create semantics).
    #[tokio::test]
    async fn create_record_with_empty_provenance_skips_audit_sink() {
        #[derive(Debug, Default)]
        struct CountingAuditSink {
            calls: AtomicUsize,
        }

        #[async_trait::async_trait]
        impl AuditSink for CountingAuditSink {
            async fn append_provenance(&self, _records: &[ProvenanceRow]) -> AuditResult<()> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }

            async fn append_export(&self, _envelope: ExportEnvelope) -> AuditResult<()> {
                Ok(())
            }
        }

        let temp_db = tempfile::NamedTempFile::new().expect("temp db");
        let dsn = format!("sqlite://{}", temp_db.path().display());
        let storage = Arc::new(
            wos_server_sqlite::SqliteStorage::connect(&dsn)
                .await
                .expect("sqlite connect"),
        );
        storage.migrate().await.expect("sqlite migrate");
        let provenance = Arc::new(PassthroughProvenancePort);
        let audit = Arc::new(CountingAuditSink::default());
        let handle = tokio::runtime::Handle::current();
        let mut store =
            StorageBackedRuntimeStore::new(storage.clone(), provenance, audit.clone(), handle);

        // Empty provenance log — the fast path under test.
        let record = RuntimeRecord::new(minimal_case_instance());
        assert!(record.provenance_log.is_empty());

        let storage_for_check = storage.clone();
        let audit_for_check = audit.clone();
        tokio::task::spawn_blocking(move || {
            store.create_record(record).expect("create record");
        })
        .await
        .expect("create task join");

        // Audit sink was never called — the empty-log fast path skipped it.
        assert_eq!(audit_for_check.calls.load(Ordering::SeqCst), 0);

        // Provenance store has zero rows for this instance — `update_instance_atomic`
        // was also skipped, so no provenance records were appended.
        let provenance_rows = storage_for_check
            .list_provenance("case_01")
            .await
            .expect("provenance read");
        assert_eq!(provenance_rows.len(), 0);

        // The instance row itself still landed via `create_instance` — the
        // fast path skips the second-phase write but not the first.
        let row = storage_for_check
            .get_instance("case_01")
            .await
            .expect("instance read")
            .expect("instance exists");
        assert_eq!(row.status, "active");
        assert_eq!(row.instance_id, "case_01");
    }
}
