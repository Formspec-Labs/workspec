// Rust guideline compliant 2026-02-21

//! Atomic runtime persistence for workflow-process state and task artifacts.

use std::collections::HashMap;

use wos_core::ProvenanceRecord;
use wos_core::instance::WorkflowProcess;

use crate::intake::{
    IntakeAcceptanceDecision, IntakeAcceptanceOutcome, IntakeAcceptanceRequest, IntakeRecordStatus,
};
use crate::runtime::{PersistDraftResult, TaskSubmissionResult};

/// Atomic runtime record for a single workflow process.
#[derive(Debug, Clone)]
pub struct RuntimeRecord {
    /// Canonical WOS workflow process state.
    pub process: WorkflowProcess,
    /// Append-only provenance for the process.
    pub provenance_log: Vec<ProvenanceRecord>,
    /// Persisted results from `invokeService` actions.
    pub step_results: Vec<StepResultRecord>,
    /// Draft and accepted task response artifacts.
    pub artifacts: HashMap<String, TaskArtifact>,
    /// Idempotency replay entries for task operations.
    pub replay_entries: HashMap<ReplayKey, ReplayValue>,
}

impl RuntimeRecord {
    /// Create a new record around a freshly-created process.
    pub fn new(instance: WorkflowProcess) -> Self {
        Self {
            process: instance,
            provenance_log: Vec::new(),
            step_results: Vec::new(),
            artifacts: HashMap::new(),
            replay_entries: HashMap::new(),
        }
    }
}

/// Persisted output from an `invokeService` action.
#[derive(Debug, Clone)]
pub struct StepResultRecord {
    /// Service reference declared by the kernel action.
    pub service_ref: String,
    /// Idempotency key used for the invocation, if any.
    pub idempotency_key: Option<String>,
    /// Serialized service output persisted before workflow advance.
    pub output: serde_json::Value,
    /// ISO 8601 persistence timestamp.
    pub recorded_at: String,
}

/// Stored task artifact.
#[derive(Debug, Clone)]
pub struct TaskArtifact {
    /// Stable artifact identifier.
    pub artifact_id: String,
    /// Owning task identifier.
    pub task_id: String,
    /// Artifact kind.
    pub kind: TaskArtifactKind,
    /// Stored response payload.
    pub response: serde_json::Value,
    /// Actor that created the artifact.
    pub actor_id: String,
    /// ISO 8601 persistence timestamp.
    pub recorded_at: String,
}

/// Artifact kind stored by the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskArtifactKind {
    /// A draft or amendment artifact.
    Draft,
    /// An accepted completed response.
    Accepted,
}

/// Replay operation category.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReplayOperation {
    /// Draft persistence replay.
    PersistDraft,
    /// Completed submission replay.
    SubmitTaskResponse,
    /// Intake acceptance replay.
    AcceptIntakeHandoff,
}

/// Idempotency replay key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReplayKey {
    /// Operation category.
    pub operation: ReplayOperation,
    /// Stable task identifier.
    pub task_id: String,
    /// Actor attempting the operation.
    pub actor_id: String,
    /// User-supplied idempotency token.
    pub token: String,
}

/// Idempotent replay value.
#[derive(Debug, Clone)]
pub enum ReplayValue {
    /// Replay of draft persistence.
    Draft(PersistDraftResult),
    /// Replay of task submission.
    Submission(TaskSubmissionResult),
    /// Replay of intake acceptance.
    Intake(IntakeAcceptanceDecision),
}

/// Persisted intake-acceptance result.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntakeRecord {
    /// Binding discriminator that interpreted the intake handoff.
    pub binding: String,
    /// Stable idempotency identifier for the intake handoff.
    pub intake_id: String,
    /// Original host-side intake request.
    pub request: IntakeAcceptanceRequest,
    /// Final host-visible outcome.
    pub outcome: IntakeAcceptanceOutcome,
    /// Provenance emitted for this intake decision.
    pub provenance_log: Vec<ProvenanceRecord>,
    /// Persistence state of this intake record.
    pub status: IntakeRecordStatus,
    /// Timestamp when the intake record was first persisted.
    pub recorded_at: String,
    /// Timestamp of the latest intake-record update.
    pub updated_at: String,
}

/// Errors from runtime persistence.
#[derive(Debug, Clone, thiserror::Error)]
pub enum StoreError {
    /// Record not found.
    #[error("runtime record not found: {0}")]
    NotFound(String),

    /// Record already exists.
    #[error("runtime record already exists: {0}")]
    AlreadyExists(String),

    /// Store-specific failure.
    #[error("runtime store failure: {0}")]
    Failed(String),
}

/// Atomic runtime record store.
pub trait RuntimeStore {
    /// Create a brand-new workflow-process record.
    fn create_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError>;

    /// Load the current record for a workflow process.
    fn load_record(&self, process_id: &str) -> Result<RuntimeRecord, StoreError>;

    /// Legacy last-resort lookup for one record bound to a case ledger.
    ///
    /// This accessor is unsafe for normal case-ledger routing because one case
    /// ledger may bind multiple workflow processes. New callers should use
    /// [`Self::processes_for_case`] and disambiguate explicitly.
    fn load_record_by_case_ledger_id(
        &self,
        case_ledger_id: &str,
    ) -> Result<RuntimeRecord, StoreError> {
        Err(StoreError::NotFound(case_ledger_id.to_string()))
    }

    /// Return the `process_id`s of every record bound to `case_ledger_id`.
    ///
    /// Empty vector when no records match.
    fn processes_for_case(&self, case_ledger_id: &str) -> Vec<String>;

    /// Return every provenance record across every workflow process bound to
    /// `case_ledger_id`, merged in `(timestamp, encounter_order)` order.
    ///
    /// This is the N:1 traversal primitive — one case file aggregating events
    /// from many workflow processes (see case-boundary report §4.3). Records
    /// `encounter_order` is the case-index process traversal order plus each
    /// process log's insertion order. Records with an empty `timestamp`
    /// (unit-test fixtures that never reached the runtime stamper) sort to the
    /// front in encounter order — exporters should treat empty timestamps as
    /// "unknown" per
    /// [`ProvenanceRecord`] docs.
    ///
    /// Empty vector when no processes are bound to the case ledger or when
    /// every bound process has an empty provenance log. Default implementation
    /// returns empty; in-memory and storage-backed adapters override.
    fn provenance_for_case(&self, _case_ledger_id: &str) -> Vec<ProvenanceRecord> {
        Vec::new()
    }

    /// Append a provenance record to the workflow process identified by
    /// `process_id` after validating that the process is bound to
    /// `case_ledger_id`.
    ///
    /// Records are stored on the per-process `provenance_log`; this method
    /// exists as the case-scoped writer counterpart to [`Self::provenance_for_case`]
    /// so callers traversing case-keyed routes (ADR-0093 §2.8) need not load
    /// the record first. Returns [`StoreError::NotFound`] when the process is
    /// missing or bound to a different case ledger. Storage adapters MUST
    /// implement this explicitly so their atomicity and index-consistency
    /// behavior is visible at the adapter boundary.
    ///
    /// # Errors
    /// Returns an error when the record cannot be loaded, the case-ledger
    /// binding mismatches, or the save fails.
    fn append_provenance_for_case(
        &mut self,
        case_ledger_id: &str,
        process_id: &str,
        record: ProvenanceRecord,
    ) -> Result<(), StoreError>;

    /// Atomically replace a workflow-process record.
    fn save_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError>;

    /// Create a brand-new intake record.
    fn create_intake_record(&mut self, record: IntakeRecord) -> Result<(), StoreError>;

    /// Load a persisted intake record.
    fn load_intake_record(
        &self,
        binding: &str,
        intake_id: &str,
    ) -> Result<IntakeRecord, StoreError>;

    /// Atomically replace an intake record.
    fn save_intake_record(&mut self, record: IntakeRecord) -> Result<(), StoreError>;
}

fn append_provenance_via_process_round_trip<S: RuntimeStore + ?Sized>(
    store: &mut S,
    case_ledger_id: &str,
    process_id: &str,
    record: ProvenanceRecord,
) -> Result<(), StoreError> {
    let mut runtime_record = store.load_record(process_id)?;
    if runtime_record.process.case_ledger_id != case_ledger_id {
        return Err(StoreError::NotFound(format!(
            "process `{process_id}` is not bound to case ledger `{case_ledger_id}`"
        )));
    }
    runtime_record.provenance_log.push(record);
    runtime_record.process.provenance_position = runtime_record.provenance_log.len() as u64;
    store.save_record(runtime_record)
}

/// In-memory runtime record store.
///
/// Maintains a secondary `case_ledger_id` → `Vec<process_id>` index so
/// case-scoped traversals (N:1 case-to-processes) run in O(processes-per-case)
/// rather than O(all-processes-in-store). Insertion order is preserved within
/// each case for callers that opt out of timestamp sorting.
#[derive(Debug, Default)]
pub struct InMemoryStore {
    records: HashMap<String, RuntimeRecord>,
    case_index: HashMap<String, Vec<String>>,
    intake_records: HashMap<(String, String), IntakeRecord>,
}

impl InMemoryStore {
    /// Create an empty in-memory store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl RuntimeStore for InMemoryStore {
    fn create_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
        let process_id = record.process.process_id.clone();
        if self.records.contains_key(&process_id) {
            return Err(StoreError::AlreadyExists(process_id));
        }

        let case_ledger_id = record.process.case_ledger_id.clone();
        self.records.insert(process_id.clone(), record);
        self.case_index
            .entry(case_ledger_id)
            .or_default()
            .push(process_id);
        Ok(())
    }

    fn load_record(&self, process_id: &str) -> Result<RuntimeRecord, StoreError> {
        self.records
            .get(process_id)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(process_id.to_string()))
    }

    fn load_record_by_case_ledger_id(
        &self,
        case_ledger_id: &str,
    ) -> Result<RuntimeRecord, StoreError> {
        let Some(process_ids) = self.case_index.get(case_ledger_id) else {
            return Err(StoreError::NotFound(case_ledger_id.to_string()));
        };

        match process_ids.as_slice() {
            [] => Err(StoreError::NotFound(case_ledger_id.to_string())),
            [process_id] => self.load_record(process_id),
            _ => Err(StoreError::Failed(format!(
                "case ledger `{case_ledger_id}` is bound to multiple processes; use processes_for_case"
            ))),
        }
    }

    fn processes_for_case(&self, case_ledger_id: &str) -> Vec<String> {
        self.case_index
            .get(case_ledger_id)
            .cloned()
            .unwrap_or_default()
    }

    fn provenance_for_case(&self, case_ledger_id: &str) -> Vec<ProvenanceRecord> {
        let Some(process_ids) = self.case_index.get(case_ledger_id) else {
            return Vec::new();
        };
        // (timestamp, encounter_order, record) — encounter order preserves
        // intra-process insertion order and keeps the sort stable for the
        // empty-timestamp fixture case.
        let mut merged: Vec<(String, usize, ProvenanceRecord)> = Vec::new();
        let mut counter: usize = 0;
        for process_id in process_ids {
            let Some(record) = self.records.get(process_id) else {
                continue;
            };
            for provenance in &record.provenance_log {
                merged.push((provenance.timestamp.clone(), counter, provenance.clone()));
                counter += 1;
            }
        }
        merged.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        merged.into_iter().map(|(_, _, record)| record).collect()
    }

    fn append_provenance_for_case(
        &mut self,
        case_ledger_id: &str,
        process_id: &str,
        record: ProvenanceRecord,
    ) -> Result<(), StoreError> {
        append_provenance_via_process_round_trip(self, case_ledger_id, process_id, record)
    }

    fn save_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
        let process_id = record.process.process_id.clone();
        let Some(existing) = self.records.get(&process_id) else {
            return Err(StoreError::NotFound(process_id));
        };
        if existing.process.case_ledger_id != record.process.case_ledger_id {
            return Err(StoreError::Failed(format!(
                "case ledger binding for process `{process_id}` is immutable"
            )));
        }

        self.records.insert(process_id, record);
        Ok(())
    }

    fn create_intake_record(&mut self, record: IntakeRecord) -> Result<(), StoreError> {
        let key = (record.binding.clone(), record.intake_id.clone());
        if self.intake_records.contains_key(&key) {
            return Err(StoreError::AlreadyExists(format!(
                "intake:{}:{}",
                key.0, key.1
            )));
        }
        self.intake_records.insert(key, record);
        Ok(())
    }

    fn load_intake_record(
        &self,
        binding: &str,
        intake_id: &str,
    ) -> Result<IntakeRecord, StoreError> {
        self.intake_records
            .get(&(binding.to_string(), intake_id.to_string()))
            .cloned()
            .ok_or_else(|| StoreError::NotFound(format!("intake:{binding}:{intake_id}")))
    }

    fn save_intake_record(&mut self, record: IntakeRecord) -> Result<(), StoreError> {
        let key = (record.binding.clone(), record.intake_id.clone());
        if !self.intake_records.contains_key(&key) {
            return Err(StoreError::NotFound(format!("intake:{}:{}", key.0, key.1)));
        }
        self.intake_records.insert(key, record);
        Ok(())
    }
}

/// Step results, artifacts, and replay map split out of [`RuntimeRecord`] for auxiliary storage.
#[derive(Debug, Clone, Default)]
pub struct RuntimeAuxFields {
    /// Persisted `invokeService` outputs.
    pub step_results: Vec<StepResultRecord>,
    /// Task response artifacts.
    pub artifacts: HashMap<String, TaskArtifact>,
    /// Idempotent replay entries.
    pub replay_entries: HashMap<ReplayKey, ReplayValue>,
}

/// Serializes non-instance fields of [`RuntimeRecord`] for auxiliary JSON storage (SBA `runtime_aux_json` shape).
pub fn runtime_aux_to_json(record: &RuntimeRecord) -> serde_json::Value {
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
                    let decision_json =
                        serde_json::to_value(decision).unwrap_or(serde_json::json!({}));
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

/// Deserializes auxiliary JSON produced by [`runtime_aux_to_json`].
pub fn runtime_aux_from_json(v: &serde_json::Value) -> RuntimeAuxFields {
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
                    let val = entry.get("value")?;
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
                    let value = match val.get("kind").and_then(|x| x.as_str())? {
                        "draft" => ReplayValue::Draft(PersistDraftResult {
                            artifact_id: val.get("artifactId")?.as_str()?.to_string(),
                        }),
                        "completed" => ReplayValue::Submission(TaskSubmissionResult::Completed {
                            artifact_id: val.get("artifactId")?.as_str()?.to_string(),
                            case_mutated: val
                                .get("caseMutated")
                                .and_then(|x| x.as_bool())
                                .unwrap_or(false),
                            emitted_event: val
                                .get("emittedEvent")
                                .and_then(|x| x.as_str())
                                .map(String::from),
                        }),
                        "failed" => ReplayValue::Submission(TaskSubmissionResult::Failed {
                            code: val.get("code")?.as_str()?.to_string(),
                            emitted_event: val
                                .get("emittedEvent")
                                .and_then(|x| x.as_str())
                                .map(String::from),
                        }),
                        "rejected" => ReplayValue::Submission(TaskSubmissionResult::Rejected {
                            code: val.get("code")?.as_str()?.to_string(),
                        }),
                        "intake" => {
                            let decision_val = val.get("decision")?.clone();
                            let decision: IntakeAcceptanceDecision =
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
    RuntimeAuxFields {
        step_results,
        artifacts,
        replay_entries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wos_core::ProvenanceRecord;
    use wos_core::instance::InstanceStatus;

    fn record_with(process_id: &str, case_ledger_id: &str) -> RuntimeRecord {
        let instance = WorkflowProcess {
            process_id: process_id.to_string(),
            case_ledger_id: case_ledger_id.to_string(),
            tenant: stack_common_typeid::DEFAULT_TENANT.to_string(),
            definition_url: "urn:test:case-scoped-store".to_string(),
            definition_version: "1.0.0".to_string(),
            configuration: Vec::new(),
            case_state: serde_json::Value::Null,
            provenance_position: 0,
            next_task_sequence: 0,
            timers: Vec::new(),
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
            created_at: "1970-01-01T00:00:00Z".to_string(),
            updated_at: "1970-01-01T00:00:00Z".to_string(),
            extensions: Default::default(),
        };
        RuntimeRecord::new(instance)
    }

    fn stamped_event(timestamp: &str, event: &str) -> ProvenanceRecord {
        let mut record =
            ProvenanceRecord::state_transition("from", "to", event, Some("actor:test"));
        record.timestamp = timestamp.to_string();
        record
    }

    fn stamped(timestamp: &str) -> ProvenanceRecord {
        stamped_event(timestamp, "evt")
    }

    #[test]
    fn provenance_for_case_merges_records_from_multiple_processes_in_time_order() {
        let mut store = InMemoryStore::new();
        let case_ledger_id = "case_01h_ledger";
        let process_a = "process_01h_a";
        let process_b = "process_01h_b";

        store
            .create_record(record_with(process_a, case_ledger_id))
            .unwrap();
        store
            .create_record(record_with(process_b, case_ledger_id))
            .unwrap();

        // Interleave timestamps across both processes — t1 (a), t2 (b), t3 (a), t4 (b).
        let mut record_a = store.load_record(process_a).unwrap();
        record_a
            .provenance_log
            .push(stamped("2026-05-12T10:00:01Z"));
        record_a
            .provenance_log
            .push(stamped("2026-05-12T10:00:03Z"));
        store.save_record(record_a).unwrap();

        let mut record_b = store.load_record(process_b).unwrap();
        record_b
            .provenance_log
            .push(stamped("2026-05-12T10:00:02Z"));
        record_b
            .provenance_log
            .push(stamped("2026-05-12T10:00:04Z"));
        store.save_record(record_b).unwrap();

        let merged = store.provenance_for_case(case_ledger_id);
        let timestamps: Vec<&str> = merged.iter().map(|r| r.timestamp.as_str()).collect();
        assert_eq!(
            timestamps,
            vec![
                "2026-05-12T10:00:01Z",
                "2026-05-12T10:00:02Z",
                "2026-05-12T10:00:03Z",
                "2026-05-12T10:00:04Z",
            ],
            "merged provenance MUST be in timestamp order across all processes bound to the case ledger",
        );

        let mut process_ids = store.processes_for_case(case_ledger_id);
        process_ids.sort();
        assert_eq!(
            process_ids,
            vec![process_a.to_string(), process_b.to_string()]
        );
    }

    #[test]
    fn provenance_for_case_preserves_encounter_order_for_identical_timestamps() {
        let mut store = InMemoryStore::new();
        let case_ledger_id = "case_01h_same_timestamp";
        let process_a = "process_01h_a";
        let process_b = "process_01h_b";
        let timestamp = "2026-05-12T10:00:00Z";

        // Create B before A to prove the tie-break is encounter order, not
        // process-id lexical ordering.
        store
            .create_record(record_with(process_b, case_ledger_id))
            .unwrap();
        store
            .create_record(record_with(process_a, case_ledger_id))
            .unwrap();

        let mut record_a = store.load_record(process_a).unwrap();
        record_a.provenance_log.push(stamped_event(timestamp, "a1"));
        record_a.provenance_log.push(stamped_event(timestamp, "a2"));
        store.save_record(record_a).unwrap();

        let mut record_b = store.load_record(process_b).unwrap();
        record_b.provenance_log.push(stamped_event(timestamp, "b1"));
        record_b.provenance_log.push(stamped_event(timestamp, "b2"));
        store.save_record(record_b).unwrap();

        let merged = store.provenance_for_case(case_ledger_id);
        let events: Vec<_> = merged
            .iter()
            .map(|record| {
                record
                    .data
                    .as_ref()
                    .and_then(|data| data.get("transitionEvent"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
            })
            .collect();
        assert_eq!(events, vec!["b1", "b2", "a1", "a2"]);
    }

    #[test]
    fn provenance_for_case_returns_empty_for_unknown_case_ledger() {
        let mut store = InMemoryStore::new();
        store
            .create_record(record_with("process_01h_a", "case_01h_known"))
            .unwrap();

        assert!(
            store
                .provenance_for_case("case_01h_does_not_exist")
                .is_empty(),
            "unknown case ledger MUST yield an empty Vec, not panic or NotFound",
        );
        assert!(
            store
                .processes_for_case("case_01h_does_not_exist")
                .is_empty()
        );
    }

    #[test]
    fn legacy_case_ledger_lookup_rejects_multiple_bound_processes() {
        let mut store = InMemoryStore::new();
        let case_ledger_id = "case_01h_ambiguous";

        store
            .create_record(record_with("process_01h_a", case_ledger_id))
            .unwrap();
        store
            .create_record(record_with("process_01h_b", case_ledger_id))
            .unwrap();

        let err = store
            .load_record_by_case_ledger_id(case_ledger_id)
            .expect_err("legacy single-record lookup must not choose an arbitrary process");

        assert!(
            matches!(err, StoreError::Failed(_)),
            "ambiguous case ledger should fail closed, got {err:?}",
        );
        assert!(
            err.to_string().contains("multiple processes"),
            "unexpected error: {err}",
        );
    }

    #[test]
    fn save_record_rejects_case_ledger_id_mutation_without_reindexing() {
        let mut store = InMemoryStore::new();
        let process_id = "process_01h_reindex";
        let original_case = "case_01h_original";
        let mutated_case = "case_01h_mutated";
        store
            .create_record(record_with(process_id, original_case))
            .unwrap();

        let mut record = store.load_record(process_id).unwrap();
        record.process.case_ledger_id = mutated_case.to_string();
        let err = store
            .save_record(record)
            .expect_err("case-ledger binding mutation must fail");

        assert!(
            matches!(err, StoreError::Failed(_)),
            "case-ledger binding mutation should fail closed, got {err:?}",
        );
        assert!(
            err.to_string().contains("immutable"),
            "unexpected error: {err}",
        );
        assert_eq!(
            store.processes_for_case(original_case),
            vec![process_id.to_string()]
        );
        assert!(store.processes_for_case(mutated_case).is_empty());
        assert_eq!(
            store
                .load_record(process_id)
                .unwrap()
                .process
                .case_ledger_id,
            original_case
        );
    }

    #[test]
    fn append_provenance_for_case_writes_to_bound_process() {
        let mut store = InMemoryStore::new();
        let case_ledger_id = "case_01h_append";
        let process_id = "process_01h_writer";
        store
            .create_record(record_with(process_id, case_ledger_id))
            .unwrap();

        store
            .append_provenance_for_case(case_ledger_id, process_id, stamped("2026-05-12T11:00:00Z"))
            .expect("case-scoped append succeeds when binding matches");

        let merged = store.provenance_for_case(case_ledger_id);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].timestamp, "2026-05-12T11:00:00Z");

        let record = store.load_record(process_id).unwrap();
        assert_eq!(
            record.process.provenance_position, 1,
            "position tracks log length"
        );
    }

    #[test]
    fn append_provenance_for_case_rejects_mismatched_binding() {
        let mut store = InMemoryStore::new();
        store
            .create_record(record_with("process_01h_w", "case_01h_correct"))
            .unwrap();

        let err = store
            .append_provenance_for_case(
                "case_01h_wrong",
                "process_01h_w",
                stamped("2026-05-12T11:00:00Z"),
            )
            .expect_err("mismatched case ledger MUST be refused");
        assert!(
            matches!(err, StoreError::NotFound(_)),
            "binding mismatch surfaces as NotFound, got {err:?}",
        );
    }
}
