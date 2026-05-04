//! `wos-runtime::WosRuntime` local adapter.

use std::sync::{Arc, Mutex};

use socketioxide::SocketIo;
use tokio::runtime::Handle;
use wos_core::instance::CaseInstance;
use wos_core::provenance::ProvenanceRecord;
use wos_core::traits::{ProvenanceSigner, ReportRenderer};
use wos_runtime::runtime::{CreateInstanceRequest, DrainOnceResult, WosRuntime};
use wos_runtime::{BindingRegistry, PersistDraftResult, RuntimeError, SystemClock, TaskSubmissionResult};
use wos_server_ports::audit::{AuditSink, NoopAuditSink};
use wos_server_ports::runtime::{RuntimeAdapterError, RuntimeOps, RuntimeResult, SeamAccess, TimerCoord};
use wos_server_ports::storage::StorageHandle;

pub mod access;
pub mod presenter;
pub mod renderer;
pub mod resolver;
pub mod runtime_store;
pub mod service;
pub mod signer;
pub mod validator;

use access::RoleBasedAccessControl;
use presenter::SocketIoTaskPresenter;
use renderer::JsonRenderer;
use resolver::BundleServiceResolver;
use runtime_store::StorageBackedRuntimeStore;
use service::EchoExternalService;
use signer::NoopSigner;
use validator::{PermissiveValidator, PolicyLayeredValidator};

pub struct AppRuntimeConfig {
    pub signer: Arc<dyn ProvenanceSigner<Error = signer::SignerError> + Send + Sync>,
    pub renderer: Arc<dyn ReportRenderer<Error = renderer::RendererError> + Send + Sync>,
    pub bindings: BindingRegistry,
    pub audit_sink: Arc<dyn AuditSink>,
}

impl Default for AppRuntimeConfig {
    fn default() -> Self {
        Self {
            signer: Arc::new(NoopSigner),
            renderer: Arc::new(JsonRenderer),
            bindings: BindingRegistry::new(),
            audit_sink: Arc::new(NoopAuditSink),
        }
    }
}

#[derive(Clone)]
pub struct AppRuntime {
    inner: Arc<Mutex<WosRuntime>>,
    signer: Arc<dyn ProvenanceSigner<Error = signer::SignerError> + Send + Sync>,
    renderer: Arc<dyn ReportRenderer<Error = renderer::RendererError> + Send + Sync>,
}

impl AppRuntime {
    pub fn build(
        storage: StorageHandle,
        provenance: Arc<dyn wos_server_ports::runtime::ProvenancePort>,
        resolver_port: Arc<dyn wos_server_ports::runtime::BundleResolverPort>,
        io: SocketIo,
    ) -> Self {
        Self::build_with(
            storage,
            provenance,
            resolver_port,
            io,
            AppRuntimeConfig::default(),
        )
    }

    pub fn build_with(
        storage: StorageHandle,
        provenance: Arc<dyn wos_server_ports::runtime::ProvenancePort>,
        resolver_port: Arc<dyn wos_server_ports::runtime::BundleResolverPort>,
        io: SocketIo,
        config: AppRuntimeConfig,
    ) -> Self {
        let handle = Handle::current();
        let store = StorageBackedRuntimeStore::new(
            storage.clone(),
            provenance,
            config.audit_sink.clone(),
            handle.clone(),
        );
        let resolver = BundleServiceResolver::new(resolver_port, handle.clone());
        let presenter = SocketIoTaskPresenter::new(storage, io, handle);
        let rt = WosRuntime::new(
            store,
            resolver,
            presenter,
            RoleBasedAccessControl::new(),
            EchoExternalService,
            PolicyLayeredValidator::new(PermissiveValidator),
            SystemClock,
            config.bindings,
        );
        Self {
            inner: Arc::new(Mutex::new(rt)),
            signer: config.signer,
            renderer: config.renderer,
        }
    }

    pub fn signer(&self) -> &Arc<dyn ProvenanceSigner<Error = signer::SignerError> + Send + Sync> {
        &self.signer
    }

    pub fn renderer(
        &self,
    ) -> &Arc<dyn ReportRenderer<Error = renderer::RendererError> + Send + Sync> {
        &self.renderer
    }

    pub async fn create_instance(
        &self,
        request: CreateInstanceRequest,
    ) -> Result<CaseInstance, RuntimeError> {
        let inner = self.inner.clone();
        tokio::task::spawn_blocking(move || {
            let mut guard = inner.lock().expect("AppRuntime mutex poisoned");
            guard.create_instance(request)
        })
        .await
        .expect("wos-runtime blocking task panicked")
    }

    pub async fn load_instance(&self, instance_id: &str) -> Result<CaseInstance, RuntimeError> {
        let inner = self.inner.clone();
        let id = instance_id.to_string();
        tokio::task::spawn_blocking(move || {
            let guard = inner.lock().expect("AppRuntime mutex poisoned");
            guard.load_instance(&id)
        })
        .await
        .expect("wos-runtime blocking task panicked")
    }

    pub async fn enqueue_event(
        &self,
        instance_id: &str,
        event: serde_json::Value,
    ) -> Result<(), RuntimeError> {
        let inner = self.inner.clone();
        let id = instance_id.to_string();
        tokio::task::spawn_blocking(move || {
            let pending: wos_core::instance::PendingEvent = serde_json::from_value(event)
                .map_err(|e| RuntimeError::Resolver(format!("invalid event payload: {e}")))?;
            let mut guard = inner.lock().expect("AppRuntime mutex poisoned");
            guard.enqueue_event(&id, pending)
        })
        .await
        .expect("wos-runtime blocking task panicked")
    }

    pub async fn drain_once(&self, instance_id: &str) -> Result<DrainOnceResult, RuntimeError> {
        let inner = self.inner.clone();
        let id = instance_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut guard = inner.lock().expect("AppRuntime mutex poisoned");
            guard.drain_once(&id)
        })
        .await
        .expect("wos-runtime blocking task panicked")
    }

    pub async fn drain_until_idle(
        &self,
        instance_id: &str,
    ) -> Result<Vec<DrainOnceResult>, RuntimeError> {
        let inner = self.inner.clone();
        let id = instance_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut guard = inner.lock().expect("AppRuntime mutex poisoned");
            guard.drain_until_idle(&id)
        })
        .await
        .expect("wos-runtime blocking task panicked")
    }

    pub async fn persist_task_draft(
        &self,
        task_id: &str,
        response: serde_json::Value,
        actor_id: &str,
        idempotency_token: Option<&str>,
    ) -> Result<PersistDraftResult, RuntimeError> {
        let inner = self.inner.clone();
        let task_id = task_id.to_string();
        let actor_id = actor_id.to_string();
        let idempotency_token = idempotency_token.map(str::to_string);
        tokio::task::spawn_blocking(move || {
            let mut guard = inner.lock().expect("AppRuntime mutex poisoned");
            guard.persist_task_draft(
                &task_id,
                response,
                &actor_id,
                idempotency_token.as_deref(),
            )
        })
        .await
        .expect("wos-runtime blocking task panicked")
    }

    pub async fn submit_task_response(
        &self,
        task_id: &str,
        response: serde_json::Value,
        actor_id: &str,
        idempotency_token: Option<&str>,
    ) -> Result<TaskSubmissionResult, RuntimeError> {
        let inner = self.inner.clone();
        let task_id = task_id.to_string();
        let actor_id = actor_id.to_string();
        let idempotency_token = idempotency_token.map(str::to_string);
        tokio::task::spawn_blocking(move || {
            let mut guard = inner.lock().expect("AppRuntime mutex poisoned");
            guard.submit_task_response(
                &task_id,
                response,
                &actor_id,
                idempotency_token.as_deref(),
            )
        })
        .await
        .expect("wos-runtime blocking task panicked")
    }

    pub async fn dismiss_task(&self, task_id: &str, reason: &str) -> Result<(), RuntimeError> {
        let inner = self.inner.clone();
        let task_id = task_id.to_string();
        let reason = reason.to_string();
        tokio::task::spawn_blocking(move || {
            let mut guard = inner.lock().expect("AppRuntime mutex poisoned");
            guard.dismiss_task(&task_id, &reason)
        })
        .await
        .expect("wos-runtime blocking task panicked")
    }

    pub async fn load_provenance_window(
        &self,
        instance_id: &str,
        offset: u64,
        limit: usize,
    ) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
        let inner = self.inner.clone();
        let id = instance_id.to_string();
        tokio::task::spawn_blocking(move || {
            let guard = inner.lock().expect("AppRuntime mutex poisoned");
            guard.load_provenance_window(&id, offset as usize, limit)
        })
        .await
        .expect("wos-runtime blocking task panicked")
    }
}

fn as_runtime_error(e: RuntimeError) -> RuntimeAdapterError {
    RuntimeAdapterError::Message(e.to_string())
}

#[async_trait::async_trait]
impl RuntimeOps for AppRuntime {
    async fn create_instance(&self, request: CreateInstanceRequest) -> RuntimeResult<CaseInstance> {
        AppRuntime::create_instance(self, request)
            .await
            .map_err(as_runtime_error)
    }

    async fn load_instance(&self, instance_id: &str) -> RuntimeResult<CaseInstance> {
        AppRuntime::load_instance(self, instance_id)
            .await
            .map_err(as_runtime_error)
    }

    async fn enqueue_event(&self, instance_id: &str, event: serde_json::Value) -> RuntimeResult<()> {
        AppRuntime::enqueue_event(self, instance_id, event)
            .await
            .map_err(as_runtime_error)
    }

    async fn drain_once(&self, instance_id: &str) -> RuntimeResult<DrainOnceResult> {
        AppRuntime::drain_once(self, instance_id)
            .await
            .map_err(as_runtime_error)
    }

    async fn drain_until_idle(&self, instance_id: &str) -> RuntimeResult<Vec<DrainOnceResult>> {
        AppRuntime::drain_until_idle(self, instance_id)
            .await
            .map_err(as_runtime_error)
    }

    async fn persist_task_draft(
        &self,
        task_id: &str,
        response: serde_json::Value,
        actor_id: &str,
        idempotency_token: Option<&str>,
    ) -> RuntimeResult<PersistDraftResult> {
        AppRuntime::persist_task_draft(self, task_id, response, actor_id, idempotency_token)
            .await
            .map_err(as_runtime_error)
    }

    async fn submit_task_response(
        &self,
        task_id: &str,
        response: serde_json::Value,
        actor_id: &str,
        idempotency_token: Option<&str>,
    ) -> RuntimeResult<TaskSubmissionResult> {
        AppRuntime::submit_task_response(self, task_id, response, actor_id, idempotency_token)
            .await
            .map_err(as_runtime_error)
    }

    async fn dismiss_task(&self, task_id: &str, reason: &str) -> RuntimeResult<()> {
        AppRuntime::dismiss_task(self, task_id, reason)
            .await
            .map_err(as_runtime_error)
    }

    async fn load_provenance_window(
        &self,
        instance_id: &str,
        offset: u64,
        limit: usize,
    ) -> RuntimeResult<Vec<ProvenanceRecord>> {
        AppRuntime::load_provenance_window(self, instance_id, offset, limit)
            .await
            .map_err(as_runtime_error)
    }
}

impl SeamAccess for AppRuntime {
    type SignerError = signer::SignerError;
    type RendererError = renderer::RendererError;

    fn signer(&self) -> &(dyn ProvenanceSigner<Error = Self::SignerError> + Send + Sync) {
        AppRuntime::signer(self).as_ref()
    }

    fn renderer(&self) -> &(dyn ReportRenderer<Error = Self::RendererError> + Send + Sync) {
        AppRuntime::renderer(self).as_ref()
    }
}

#[async_trait::async_trait]
impl TimerCoord for AppRuntime {
    async fn tick_once(&self) -> RuntimeResult<usize> {
        Ok(0)
    }
}
