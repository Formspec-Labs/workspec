#[cfg(feature = "runtime-local")]
pub use wos_server_runtime_local::*;

#[cfg(not(feature = "runtime-local"))]
#[derive(Clone)]
pub struct AppRuntime;

#[cfg(not(feature = "runtime-local"))]
pub struct AppRuntimeConfig {
    pub audit_sink: std::sync::Arc<dyn wos_server_ports::audit::AuditSink>,
}

#[cfg(not(feature = "runtime-local"))]
impl Default for AppRuntimeConfig {
    fn default() -> Self {
        Self {
            audit_sink: std::sync::Arc::new(wos_server_ports::audit::NoopAuditSink),
        }
    }
}

#[cfg(not(feature = "runtime-local"))]
impl AppRuntime {
    fn disabled_error() -> wos_runtime::RuntimeError {
        wos_runtime::RuntimeError::Store(wos_runtime::store::StoreError::Failed(
            "runtime-local feature disabled".into(),
        ))
    }

    fn migrate_disabled_error() -> wos_runtime::RuntimeError {
        wos_runtime::RuntimeError::FeatureDisabled(
            "instance migration requires the wos-server `runtime-local` feature; rebuild with `--features runtime-local`.",
        )
    }

    pub fn build_with(
        _storage: wos_server_ports::storage::StorageHandle,
        _provenance: std::sync::Arc<dyn wos_server_ports::runtime::ProvenancePort>,
        _resolver_port: std::sync::Arc<dyn wos_server_ports::runtime::BundleResolverPort>,
        _io: socketioxide::SocketIo,
        _config: AppRuntimeConfig,
    ) -> Self {
        Self
    }

    pub async fn create_instance(
        &self,
        _request: wos_runtime::runtime::CreateInstanceRequest,
    ) -> Result<wos_core::instance::CaseInstance, wos_runtime::RuntimeError> {
        Err(Self::disabled_error())
    }

    pub async fn enqueue_event(
        &self,
        _instance_id: &str,
        _event: serde_json::Value,
    ) -> Result<(), wos_runtime::RuntimeError> {
        Err(Self::disabled_error())
    }

    pub async fn drain_once(
        &self,
        _instance_id: &str,
    ) -> Result<wos_runtime::runtime::DrainOnceResult, wos_runtime::RuntimeError> {
        Err(Self::disabled_error())
    }

    pub async fn drain_until_idle(
        &self,
        _instance_id: &str,
    ) -> Result<Vec<wos_runtime::runtime::DrainOnceResult>, wos_runtime::RuntimeError> {
        Err(Self::disabled_error())
    }

    pub async fn persist_task_draft(
        &self,
        _task_id: &str,
        _response: serde_json::Value,
        _actor_id: &str,
        _idempotency_token: Option<&str>,
    ) -> Result<wos_runtime::PersistDraftResult, wos_runtime::RuntimeError> {
        Err(Self::disabled_error())
    }

    pub async fn submit_task_response(
        &self,
        _task_id: &str,
        _response: serde_json::Value,
        _actor_id: &str,
        _idempotency_token: Option<&str>,
    ) -> Result<wos_runtime::TaskSubmissionResult, wos_runtime::RuntimeError> {
        Err(Self::disabled_error())
    }

    pub async fn dismiss_task(
        &self,
        _task_id: &str,
        _reason: &str,
    ) -> Result<(), wos_runtime::RuntimeError> {
        Err(Self::disabled_error())
    }

    pub async fn migrate_instance(
        &self,
        _instance_id: &str,
        _target_definition_version: &str,
        _migration_map: wos_runtime::MigrationMap,
        _operator_actor_id: Option<&str>,
    ) -> Result<wos_runtime::MigrationOutcome, wos_runtime::RuntimeError> {
        Err(Self::migrate_disabled_error())
    }

    pub fn renderer(
        &self,
    ) -> &std::sync::Arc<
        dyn wos_core::traits::ReportRenderer<Error = std::convert::Infallible> + Send + Sync,
    > {
        panic!("runtime-local feature disabled")
    }
}
