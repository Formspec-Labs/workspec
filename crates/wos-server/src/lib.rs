//! `wos-server` — reference HTTP + Socket.IO backend for WOS.
//!
//! This crate is the server-side counterpart to the `studio/` React app.
//! It wraps `wos-core`'s evaluator and exposes the REST + Socket.IO contract
//! defined by `studio/src/services/WosBackend.ts` and `WosPorts.ts`.

pub mod auth;
pub mod config;
pub mod domain;
pub mod error;
pub mod export;
pub mod http;
pub mod realtime;
pub mod runtime;
pub mod seed;
pub mod services;
pub mod storage;

pub use config::ServerConfig;
pub use error::{ApiError, ApiResult};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::auth::AuthHandle;
use crate::runtime::AppRuntime;
use crate::services::AppServices;
use crate::storage::StorageHandle;

use crate::domain::EvaluationResultView;

/// Shared application state injected into every handler and Socket.IO namespace.
#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<ServerConfig>,
    pub storage: StorageHandle,
    pub auth: AuthHandle,
    pub services: Arc<AppServices>,
    pub runtime: AppRuntime,
    /// HTTP-layer event idempotency cache: `(instance_id, idempotency_token) → EvaluationResultView`.
    /// The Restate adapter handles dedup natively via journaled execution; this
    /// cache is the reference-server defense-in-depth.
    pub event_idempotency: Arc<Mutex<HashMap<String, EvaluationResultView>>>,
}

/// Boot the server with the given config, wiring storage, auth, services, and
/// both the HTTP and realtime layers. Returns once the server has shut down.
pub async fn run(cfg: ServerConfig) -> anyhow::Result<()> {
    let cfg = Arc::new(cfg);

    let storage = storage::build(&cfg).await?;
    let auth = auth::build(&cfg, storage.clone());
    let services = Arc::new(AppServices::new(cfg.clone(), storage.clone()).await?);

    // Build the Socket.IO layer before AppRuntime so the TaskPresenter can
    // broadcast task events.
    let (io_layer, io) = realtime::build_io_only();

    let app_runtime = AppRuntime::build(
        storage.clone(),
        services.provenance.clone(),
        services.bundle.clone(),
        io.clone(),
    );

    let state = AppState {
        cfg: cfg.clone(),
        storage,
        auth,
        services,
        runtime: app_runtime,
        event_idempotency: Arc::new(Mutex::new(HashMap::new())),
    };

    // Now that the state exists, register the realtime namespace handlers
    // (they need `AppState`).
    realtime::attach_namespaces(&io, state.clone());

    if cfg.seed {
        if let Err(e) = seed::run(&state).await {
            tracing::warn!(error = %e, "seed step failed");
        }
    }

    // Start the timer-polling task alongside the HTTP/WS server.
    let _timer_task = services::timer_task::spawn(state.clone());

    let router = http::router(state.clone());
    let app = router.layer(io_layer);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", cfg.port)).await?;
    tracing::info!("wos-server listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
