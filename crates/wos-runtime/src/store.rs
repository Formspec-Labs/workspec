// Rust guideline compliant 2026-02-21

//! Atomic runtime persistence for instance state and task artifacts.

use std::collections::HashMap;

use wos_core::instance::CaseInstance;
use wos_core::provenance::ProvenanceRecord;

use crate::runtime::{PersistDraftResult, TaskSubmissionResult};

const LEGACY_INSTANCE_ALIAS_EXTENSION_KEY: &str = "x-wos-legacy-instance-alias";

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
}

/// In-memory runtime record store.
#[derive(Debug, Default)]
pub struct InMemoryStore {
    records: HashMap<String, RuntimeRecord>,
    aliases: HashMap<String, String>,
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
}

fn legacy_instance_alias(instance: &CaseInstance) -> Option<String> {
    instance
        .extensions
        .get(LEGACY_INSTANCE_ALIAS_EXTENSION_KEY)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
}
