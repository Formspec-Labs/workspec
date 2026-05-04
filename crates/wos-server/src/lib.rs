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
use crate::config::{AuditSinkKind, RuntimeKind};
use crate::runtime::{AppRuntime, AppRuntimeConfig};
use crate::services::AppServices;
use crate::storage::StorageHandle;
use wos_server_ports::audit::{AuditSink, NoopAuditSink};

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
    let auth = auth::build(&cfg, storage.clone())?;
    let services = Arc::new(AppServices::new(cfg.clone(), storage.clone()).await?);
    let audit_sink = build_audit_sink(&cfg)?;

    // Build the Socket.IO layer before AppRuntime so the TaskPresenter can
    // broadcast task events.
    let (io_layer, io) = realtime::build_io_only();

    let app_runtime = match cfg.runtime {
        RuntimeKind::Local => {
            #[cfg(feature = "runtime-local")]
            {
                AppRuntime::build_with(
                    storage.clone(),
                    services.provenance.clone(),
                    services.bundle.clone(),
                    io.clone(),
                    AppRuntimeConfig {
                        audit_sink,
                        ..AppRuntimeConfig::default()
                    },
                )
            }
            #[cfg(not(feature = "runtime-local"))]
            anyhow::bail!(
                "WOS_RUNTIME=local requested but crate built without feature `runtime-local`"
            )
        }
        RuntimeKind::Restate => {
            #[cfg(any(feature = "runtime-restate", feature = "runtime-restate-stub"))]
            anyhow::bail!("WOS_RUNTIME=restate scaffold loaded; WS-094 adapter wiring still pending");
            #[cfg(not(any(feature = "runtime-restate", feature = "runtime-restate-stub")))]
            anyhow::bail!(
                "WOS_RUNTIME=restate requested but crate built without feature `runtime-restate` or `runtime-restate-stub`"
            )
        }
    };

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

fn build_audit_sink(cfg: &ServerConfig) -> anyhow::Result<Arc<dyn AuditSink>> {
    match cfg.audit_sink {
        AuditSinkKind::None => Ok(Arc::new(NoopAuditSink)),
        AuditSinkKind::Postgres => {
            #[cfg(feature = "audit-postgres")]
            {
                let dsn = if cfg.audit_database_url.trim().is_empty() {
                    &cfg.database_url
                } else {
                    &cfg.audit_database_url
                };
                let sink = wos_server_audit_postgres::PostgresAuditSink::connect(dsn)
                    .map_err(|e| anyhow::anyhow!("failed to connect audit sink: {e}"))?;
                Ok(Arc::new(sink))
            }
            #[cfg(not(feature = "audit-postgres"))]
            anyhow::bail!(
                "WOS_AUDIT_SINK=postgres requested but crate built without feature `audit-postgres`"
            )
        }
    }
}
