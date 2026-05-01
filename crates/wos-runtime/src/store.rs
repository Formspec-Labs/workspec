// Rust guideline compliant 2026-02-21

//! Atomic runtime persistence for instance state and task artifacts.

use std::collections::HashMap;

use wos_core::instance::CaseInstance;
use wos_core::provenance::ProvenanceRecord;

use crate::intake::{
    IntakeAcceptanceDecision, IntakeAcceptanceOutcome, IntakeAcceptanceRequest, IntakeRecordStatus,
};
use crate::runtime::{PersistDraftResult, TaskSubmissionResult};

/// Atomic runtime record for a single instance.
#[derive(Debug, Clone)]
pub struct RuntimeRecord {
    /// Canonical WOS case instance state.
    pub instance: CaseInstance,
    /// Append-only provenance for the instance.
    pub provenance_log: Vec<ProvenanceRecord>,
    /// Persisted results from `invokeService` actions.
    pub step_results: Vec<StepResultRecord>,
    /// Draft and accepted task response artifacts.
    pub artifacts: HashMap<String, TaskArtifact>,
    /// Idempotency replay entries for task operations.
    pub replay_entries: HashMap<ReplayKey, ReplayValue>,
}

impl RuntimeRecord {
    /// Create a new record around a freshly-created instance.
    pub fn new(instance: CaseInstance) -> Self {
        Self {
            instance,
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
    /// Create a brand-new instance record.
    fn create_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError>;

    /// Load the current record for an instance.
    fn load_record(&self, instance_id: &str) -> Result<RuntimeRecord, StoreError>;

    /// Atomically replace an instance record.
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

/// In-memory runtime record store.
#[derive(Debug, Default)]
pub struct InMemoryStore {
    records: HashMap<String, RuntimeRecord>,
    aliases: HashMap<String, String>,
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
        let instance_id = record.instance.instance_id.clone();
        if self.records.contains_key(&instance_id) {
            return Err(StoreError::AlreadyExists(instance_id));
        }

        if let Some(alias) = legacy_instance_alias(&record.instance)
            && alias != instance_id
        {
            self.aliases.insert(alias, instance_id.clone());
        }

        self.records.insert(instance_id, record);
        Ok(())
    }

    fn load_record(&self, instance_id: &str) -> Result<RuntimeRecord, StoreError> {
        let canonical_id = self
            .aliases
            .get(instance_id)
            .map_or(instance_id, String::as_str);
        self.records
            .get(canonical_id)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(instance_id.to_string()))
    }

    fn save_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
        let instance_id = record.instance.instance_id.clone();
        if !self.records.contains_key(&instance_id) {
            return Err(StoreError::NotFound(instance_id));
        }

        if let Some(alias) = legacy_instance_alias(&record.instance)
            && alias != instance_id
        {
            self.aliases.insert(alias, instance_id.clone());
        }

        self.records.insert(instance_id, record);
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

fn legacy_instance_alias(instance: &CaseInstance) -> Option<String> {
    instance
        .extensions
        .get(CaseInstance::LEGACY_INSTANCE_ALIAS_EXTENSION_KEY)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
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
