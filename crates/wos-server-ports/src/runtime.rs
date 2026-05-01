//! Runtime adapter traits shared across runtime backends.
// Rust guideline compliant 2026-02-21

use crate::storage::ProvenanceRow;
use async_trait::async_trait;
use wos_core::instance::CaseInstance;
use wos_core::provenance::ProvenanceRecord;
use wos_core::traits::{ProvenanceSigner, ReportRenderer};
use wos_runtime::runtime::{CreateInstanceRequest, DrainOnceResult, MigrationMap, MigrationOutcome};
use wos_runtime::{PersistDraftResult, TaskSubmissionResult};

#[derive(Debug, Clone, thiserror::Error)]
pub enum RuntimeAdapterError {
    #[error("{0}")]
    Message(String),
}

pub type RuntimeResult<T> = Result<T, RuntimeAdapterError>;

#[async_trait]
pub trait RuntimeOps: Send + Sync + 'static {
    async fn create_instance(&self, request: CreateInstanceRequest) -> RuntimeResult<CaseInstance>;
    async fn load_instance(&self, instance_id: &str) -> RuntimeResult<CaseInstance>;
    async fn enqueue_event(&self, instance_id: &str, event: serde_json::Value)
    -> RuntimeResult<()>;
    async fn drain_once(&self, instance_id: &str) -> RuntimeResult<DrainOnceResult>;
    async fn drain_until_idle(&self, instance_id: &str) -> RuntimeResult<Vec<DrainOnceResult>>;
    async fn persist_task_draft(
        &self,
        task_id: &str,
        response: serde_json::Value,
        actor_id: &str,
        idempotency_token: Option<&str>,
    ) -> RuntimeResult<PersistDraftResult>;
    async fn submit_task_response(
        &self,
        task_id: &str,
        response: serde_json::Value,
        actor_id: &str,
        idempotency_token: Option<&str>,
    ) -> RuntimeResult<TaskSubmissionResult>;
    async fn dismiss_task(&self, task_id: &str, reason: &str) -> RuntimeResult<()>;
    async fn load_provenance_window(
        &self,
        instance_id: &str,
        offset: u64,
        limit: usize,
    ) -> RuntimeResult<Vec<ProvenanceRecord>>;

    async fn migrate_instance(
        &self,
        instance_id: &str,
        target_definition_version: &str,
        migration_map: MigrationMap,
        operator_actor_id: Option<&str>,
    ) -> RuntimeResult<MigrationOutcome>;
}

pub trait SeamAccess: Send + Sync + 'static {
    type SignerError: std::error::Error + Send + Sync + 'static;
    type RendererError: std::error::Error + Send + Sync + 'static;

    fn signer(&self) -> &(dyn ProvenanceSigner<Error = Self::SignerError> + Send + Sync);
    fn renderer(&self) -> &(dyn ReportRenderer<Error = Self::RendererError> + Send + Sync);
}

#[async_trait]
pub trait TimerCoord: Send + Sync + 'static {
    async fn tick_once(&self) -> RuntimeResult<usize>;
    async fn register_timer(
        &self,
        _instance_id: &str,
        _timer_id: &str,
        _at_unix_seconds: i64,
    ) -> RuntimeResult<()> {
        Ok(())
    }
}

#[async_trait]
pub trait BundleResolverPort: Send + Sync + 'static {
    async fn resolve_kernel_bundle(&self, workflow_url: &str) -> RuntimeResult<serde_json::Value>;
    async fn resolve_governance_bundle(
        &self,
        workflow_url: &str,
    ) -> RuntimeResult<serde_json::Value>;
    async fn resolve_sidecar_bundle(&self, workflow_url: &str) -> RuntimeResult<serde_json::Value>;
}

#[async_trait]
pub trait ProvenancePort: Send + Sync + 'static {
    async fn prepare_batch(
        &self,
        instance_id: &str,
        records: &[ProvenanceRecord],
    ) -> RuntimeResult<Vec<ProvenanceRow>>;
}
