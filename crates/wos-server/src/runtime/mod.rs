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

/// Selectable runtime seams that `AppRuntime` owns directly. WS-080: today
/// covers `signer` (Runtime §12.6 ProvenanceSigner) and `renderer` (Runtime
/// §12.7 ReportRenderer); the other Runtime §12 seams (validator, access,
/// external, clock) are constructed inline by `build_with` and remain
/// hard-coded until upstream `wos-core::traits` carries `Box<dyn>` blanket
/// impls. Their planned shape lives in `validator` / `access` / `external`
/// / `clock` fields below as `Option`s — `None` keeps today's defaults; a
/// future commit can flip them on without changing this surface.
pub struct AppRuntimeConfig {
    pub signer: Arc<dyn ProvenanceSigner<Error = signer::SignerError> + Send + Sync>,
    pub renderer: Arc<dyn ReportRenderer<Error = renderer::RendererError> + Send + Sync>,
    /// Contract binding adapters for task submission. Production registers
    /// nothing here (matching pre-WS-011 behaviour); tests inject a
    /// `formspec` adapter so HTTP-level task submission fixtures can drive
    /// `submit_task_response` to a `Completed` outcome without adding a
    /// runtime-tier seam.
    pub bindings: BindingRegistry,
}

impl Default for AppRuntimeConfig {
    fn default() -> Self {
        Self {
            signer: Arc::new(NoopSigner),
            renderer: Arc::new(JsonRenderer),
            bindings: BindingRegistry::new(),
        }
    }
}

impl AppRuntimeConfig {
    /// Read [`crate::config::ServerConfig::signer_kind`] (env `WOS_SIGNER`)
    /// and pick a concrete signer. **Today only `noop` is wired.**
    /// `Ed25519File` (WS-043) and `External` will be added with their impls;
    /// `WOS_SIGNER=ed25519-file` (or any other non-`noop` value) currently
    /// produces a clap error at startup — there is no fallback.
    pub fn from_server_config(cfg: &crate::config::ServerConfig) -> Self {
        let signer: Arc<dyn ProvenanceSigner<Error = signer::SignerError> + Send + Sync> =
            match cfg.signer_kind {
                crate::config::SignerKind::Noop => Arc::new(NoopSigner),
            };
        Self {
            signer,
            renderer: Arc::new(JsonRenderer),
            bindings: BindingRegistry::new(),
        }
    }
}

/// The server's runtime handle. Clone freely — it's backed by an `Arc`.
#[derive(Clone)]
pub struct AppRuntime {
    inner: Arc<Mutex<WosRuntime>>,
    signer: Arc<dyn ProvenanceSigner<Error = signer::SignerError> + Send + Sync>,
    renderer: Arc<dyn ReportRenderer<Error = renderer::RendererError> + Send + Sync>,
}

impl AppRuntime {
    /// Assemble the runtime from the server's service + storage handles
    /// using default seam impls. Equivalent to
    /// `build_with(..., AppRuntimeConfig::default())`. Must be called from
    /// inside a tokio runtime (it uses `Handle::current`).
    pub fn build(
        storage: StorageHandle,
        provenance: Arc<ProvenanceService>,
        bundle: Arc<BundleService>,
        io: SocketIo,
    ) -> Self {
        Self::build_with(storage, provenance, bundle, io, AppRuntimeConfig::default())
    }

    /// Assemble the runtime with an explicit seam config. Today the only
    /// swappable seams are `signer` and `renderer`; the rest are still
    /// hard-coded inline (see `AppRuntimeConfig` doc-comment). Tests use
    /// this entry point to substitute fakes; production calls
    /// [`Self::build`] which reads [`crate::config::ServerConfig::signer_kind`]
    /// via `AppRuntimeConfig::from_server_config`.
    pub fn build_with(
        storage: StorageHandle,
        provenance: Arc<ProvenanceService>,
        bundle: Arc<BundleService>,
        io: SocketIo,
        config: AppRuntimeConfig,
    ) -> Self {
        let handle = Handle::current();
        let store = SqliteRuntimeStore::new(storage.clone(), provenance.clone(), handle.clone());
        let resolver = BundleServiceResolver::new(bundle.clone(), handle.clone());
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


