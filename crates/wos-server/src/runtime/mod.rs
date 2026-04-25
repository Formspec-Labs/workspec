//! `wos-runtime::WosRuntime` integration.
//!
//! `AppRuntime` wraps a concrete `WosRuntime` in an `Arc<Mutex<…>>` and
//! exposes **async** methods that dispatch the underlying sync runtime
//! through `tokio::task::spawn_blocking`. HTTP handlers see an honest
//! async API; the sync block happens off the tokio async worker pool.
//!
//! Why this shape: the upstream `wos-runtime` and `wos-core` trait hooks
//! are synchronous, and the runtime owns mutable state (drain/submit
//! methods take `&mut self`). Wrapping in `Arc<Mutex>` + `spawn_blocking`
//! is the minimum-cost bridge that preserves async end-to-end at the
//! server boundary. If the upstream runtime goes async in the future,
//! the only change needed is dropping `spawn_blocking` — the
//! `AppRuntime` surface stays identical.

use std::sync::{Arc, Mutex};

use socketioxide::SocketIo;
use tokio::runtime::Handle;
use wos_core::instance::CaseInstance;
use wos_core::provenance::ProvenanceRecord;
use wos_runtime::runtime::{CreateInstanceRequest, DrainOnceResult, WosRuntime};
use wos_runtime::{BindingRegistry, PersistDraftResult, RuntimeError, SystemClock, TaskSubmissionResult};

use wos_core::traits::{ProvenanceSigner, ReportRenderer};

use crate::services::bundle_service::BundleService;
use crate::services::provenance_service::ProvenanceService;
use crate::storage::{SqliteRuntimeStore, StorageHandle};

pub mod access;
pub mod presenter;
pub mod renderer;
pub mod resolver;
pub mod service;
pub mod signer;
pub mod validator;

use access::RoleBasedAccessControl;
use presenter::SocketIoTaskPresenter;
use renderer::JsonRenderer;
use resolver::BundleServiceResolver;
use service::EchoExternalService;
use signer::NoopSigner;
use validator::{PermissiveValidator, PolicyLayeredValidator};

/// The server's runtime handle. Clone freely — it's backed by an `Arc`.
#[derive(Clone)]
pub struct AppRuntime {
    inner: Arc<Mutex<WosRuntime>>,
    signer: Arc<dyn ProvenanceSigner<Error = signer::SignerError> + Send + Sync>,
    renderer: Arc<dyn ReportRenderer<Error = renderer::RendererError> + Send + Sync>,
}

impl AppRuntime {
    /// Assemble the runtime from the server's service + storage handles.
    /// Must be called from inside a tokio runtime (it uses `Handle::current`).
    pub fn build(
        storage: StorageHandle,
        provenance: Arc<ProvenanceService>,
        bundle: Arc<BundleService>,
        io: SocketIo,
    ) -> Self {
        let handle = Handle::current();
        let store = SqliteRuntimeStore::new(storage.clone(), provenance.clone(), handle.clone());
        let resolver = BundleServiceResolver::new(bundle.clone(), handle.clone());
        let presenter = SocketIoTaskPresenter::new(storage, io, handle);
        let bindings = BindingRegistry::new();
        let rt = WosRuntime::new(
            store,
            resolver,
            presenter,
            RoleBasedAccessControl::new(),
            EchoExternalService,
            PolicyLayeredValidator::new(PermissiveValidator),
            SystemClock,
            bindings,
        );
        Self {
            inner: Arc::new(Mutex::new(rt)),
            signer: Arc::new(NoopSigner),
            renderer: Arc::new(JsonRenderer),
        }
    }

    /// Access the injected provenance signer (exported for use by provenance
    /// export and future attestation surfaces).
    pub fn signer(&self) -> &Arc<dyn ProvenanceSigner<Error = signer::SignerError> + Send + Sync> {
        &self.signer
    }

    /// Access the injected report renderer (exported for use by `/explain`
    /// and future rendered-output surfaces).
    pub fn renderer(
        &self,
    ) -> &Arc<dyn ReportRenderer<Error = renderer::RendererError> + Send + Sync> {
        &self.renderer
    }

    /// Create a fresh instance from a kernel+version.
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

    /// Load the canonical CaseInstance snapshot.
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

    /// Enqueue an event for processing. Does not drain; pair with
    /// [`Self::drain_once`] for synchronous round-trip behaviour.
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

    /// Process a single queued event, returning transitions + provenance.
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

    /// Drain every queued event until the queue is empty.
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

    /// Persist a task draft. Matches `wos_runtime::WosRuntime::persist_task_draft`.
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

    /// Submit a completed task response. Returns `Completed`, `Failed`,
    /// or `Rejected` per `TaskSubmissionResult`.
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

    /// Dismiss a pending task without advancing lifecycle state.
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

    /// Load a window of provenance records.
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


