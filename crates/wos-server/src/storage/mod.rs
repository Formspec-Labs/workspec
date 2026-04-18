//! Storage trait and concrete backends.
//!
//! All persistence goes through the [`Storage`] trait so storage engines
//! (SQLite, Postgres, JSONFS, ledger sinks) can be swapped behind the same
//! service layer. SQLite is the default and only backend shipped today.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::{ServerConfig, StorageKind};

pub mod runtime_store;
pub mod sqlite;

pub use runtime_store::SqliteRuntimeStore;
pub use sqlite::SqliteStorage;

pub type StorageHandle = Arc<dyn Storage>;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("not found")]
    NotFound,

    #[error("conflict: {0}")]
    Conflict(String),

    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),

    #[error(transparent)]
    Serde(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

pub type StorageResult<T> = Result<T, StorageError>;

/// A stored kernel document (`$wosKernel` definition + metadata).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelRow {
    pub url: String,
    pub title: String,
    pub version: String,
    pub status: String,
    pub impact_level: String,
    pub document: serde_json::Value,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// A stored case instance.
///
/// `instance_json` holds the serialised `wos-core::CaseInstance` so the server
/// can round-trip through `Evaluator::from_instance` without losing runtime
/// bookkeeping (history_store, fired_milestones, pending_events,
/// compensation_logs, volume_counters, extensions, etc.). The remaining
/// columns are denormalised search indexes populated from `instance_json`
/// at write time — `build_instance_row` in the eval service is the single
/// writer of truth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceRow {
    pub instance_id: String,
    pub definition_url: String,
    pub definition_version: String,
    pub status: String,
    pub impact_level: String,
    pub instance_json: serde_json::Value,
    /// Auxiliary `wos_runtime::RuntimeRecord` fields
    /// (step_results, artifacts, replay_entries) serialised as JSON.
    #[serde(default)]
    pub runtime_aux_json: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl InstanceRow {
    /// Convenience: look up the configuration (active states) from the
    /// embedded instance JSON.
    pub fn configuration(&self) -> Vec<String> {
        self.instance_json
            .get("configuration")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn case_state(&self) -> serde_json::Value {
        self.instance_json
            .get("caseState")
            .cloned()
            .unwrap_or(serde_json::json!({}))
    }

    pub fn active_tasks(&self) -> &serde_json::Value {
        self.instance_json
            .get("activeTasks")
            .unwrap_or(&serde_json::Value::Null)
    }

    pub fn timers(&self) -> &serde_json::Value {
        self.instance_json
            .get("timers")
            .unwrap_or(&serde_json::Value::Null)
    }

    pub fn governance_state(&self) -> Option<&serde_json::Value> {
        self.instance_json.get("governanceState").filter(|v| !v.is_null())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceRow {
    pub id: String,
    pub instance_id: String,
    pub seq: i64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tier: String,
    pub payload: serde_json::Value,
    pub hash: String,
    pub previous_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationRow {
    pub id: String,
    pub workflow_url: String,
    pub delegator: String,
    pub delegate: String,
    pub scope: String,
    pub authority: Option<String>,
    pub legal_instrument: Option<String>,
    pub start_date: chrono::DateTime<chrono::Utc>,
    pub end_date: Option<chrono::DateTime<chrono::Utc>>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRow {
    pub id: String,
    pub email: String,
    pub name: String,
    pub role: String,
    pub password_hash: String,
    pub avatar: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRow {
    pub jti: String,
    pub user_id: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityFactRow {
    pub id: String,
    pub instance_id: String,
    pub subject_ref: String,
    pub assurance_level: String,
    pub disclosure_posture: String,
    pub fact_json: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upgraded_from: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboundCloudEventRow {
    pub cloud_event_id: String,
    pub instance_id: String,
    pub binding: String,
    pub received_at: chrono::DateTime<chrono::Utc>,
    pub payload_json: serde_json::Value,
}

#[derive(Debug, Clone, Default)]
pub struct InstanceQuery {
    pub status: Option<Vec<String>>,
    pub impact_level: Option<Vec<String>>,
    pub definition_url: Option<Vec<String>>,
    pub page: u32,      // 1-indexed to match the studio contract
    pub page_size: u32, // capped downstream
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

/// Mutation passed to [`Storage::update_instance_atomic`]. Returning `Err`
/// aborts the transaction; returning `Ok` commits the new row plus any
/// provenance appended via the same handle inside the closure.
pub type InstanceMutator<'a> =
    &'a (dyn Fn(&mut InstanceRow) -> Result<Vec<ProvenanceRow>, StorageError> + Send + Sync);

#[async_trait]
pub trait Storage: Send + Sync + 'static {
    // --- Kernel registry ---
    async fn list_kernels(&self) -> StorageResult<Vec<KernelRow>>;
    async fn get_kernel(&self, url: &str) -> StorageResult<Option<KernelRow>>;
    async fn upsert_kernel(&self, row: &KernelRow) -> StorageResult<()>;

    // --- Instances ---
    async fn create_instance(&self, row: &InstanceRow) -> StorageResult<()>;
    async fn get_instance(&self, id: &str) -> StorageResult<Option<InstanceRow>>;
    async fn list_instances(&self, q: InstanceQuery) -> StorageResult<Page<InstanceRow>>;
    async fn update_instance_atomic(
        &self,
        id: &str,
        mutator: InstanceMutator<'_>,
    ) -> StorageResult<InstanceRow>;

    // --- Provenance ---
    async fn list_provenance(&self, instance_id: &str) -> StorageResult<Vec<ProvenanceRow>>;
    async fn last_provenance(&self, instance_id: &str) -> StorageResult<Option<ProvenanceRow>>;

    // --- Delegations ---
    async fn list_delegations(&self, workflow_url: &str) -> StorageResult<Vec<DelegationRow>>;
    async fn upsert_delegation(&self, row: &DelegationRow) -> StorageResult<()>;
    async fn revoke_delegation(&self, workflow_url: &str, id: &str) -> StorageResult<()>;

    // --- Identity facts (assurance) ---
    async fn insert_identity_fact(&self, row: &IdentityFactRow) -> StorageResult<()>;
    async fn get_identity_fact(&self, id: &str) -> StorageResult<Option<IdentityFactRow>>;
    async fn list_identity_facts(&self, instance_id: &str) -> StorageResult<Vec<IdentityFactRow>>;
    async fn list_assurance_chain(&self, subject_ref: &str) -> StorageResult<Vec<IdentityFactRow>>;

    // --- Inbound CloudEvents idempotency ---
    async fn get_inbound_cloud_event(
        &self,
        cloud_event_id: &str,
    ) -> StorageResult<Option<InboundCloudEventRow>>;
    async fn insert_inbound_cloud_event(&self, row: &InboundCloudEventRow) -> StorageResult<()>;

    // --- Auth ---
    async fn get_user_by_email(&self, email: &str) -> StorageResult<Option<UserRow>>;
    async fn get_user(&self, id: &str) -> StorageResult<Option<UserRow>>;
    async fn upsert_user(&self, row: &UserRow) -> StorageResult<()>;
    async fn upsert_session(&self, row: &SessionRow) -> StorageResult<()>;
    async fn revoke_session(&self, jti: &str) -> StorageResult<()>;
    async fn session_is_valid(&self, jti: &str) -> StorageResult<bool>;
}

/// Build the storage backend selected by the config.
pub async fn build(cfg: &ServerConfig) -> anyhow::Result<StorageHandle> {
    match cfg.storage {
        StorageKind::Sqlite => {
            let store = SqliteStorage::connect(&cfg.database_url).await?;
            store.migrate().await?;
            Ok(Arc::new(store))
        }
    }
}
