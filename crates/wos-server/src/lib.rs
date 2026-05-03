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

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use crate::auth::AuthHandle;
use crate::config::{AuditSinkKind, RuntimeKind};
use crate::runtime::{AppRuntime, AppRuntimeConfig};
use crate::services::AppServices;
use crate::storage::StorageHandle;
use wos_server_ports::audit::{AuditSink, NoopAuditSink};

use crate::domain::EvaluationResultView;
use wos_runtime::MigrationOutcome;

/// Replay cache for `POST /api/instances/:id/migrate` when `Idempotency-Key` is present.
///
/// FIFO-evicted when size exceeds [`Self::MAX_ENTRIES`] so the reference server does not grow
/// this map without bound. Replacing an existing key moves that key to the back of the FIFO queue
/// so eviction stays LRU-like for hot keys.
#[derive(Debug)]
pub struct MigrateIdempotencyCache {
    entries: HashMap<String, MigrationOutcome>,
    fifo_keys: VecDeque<String>,
}

impl Default for MigrateIdempotencyCache {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            fifo_keys: VecDeque::new(),
        }
    }
}

impl MigrateIdempotencyCache {
    pub const MAX_ENTRIES: usize = 4096;

    pub fn get(&self, key: &str) -> Option<MigrationOutcome> {
        self.entries.get(key).cloned()
    }

    /// Inserts or replaces `key`. New keys may evict the oldest entries until under `MAX_ENTRIES`.
    pub fn insert(&mut self, key: String, value: MigrationOutcome) {
        if self.entries.contains_key(&key) {
            self.entries.insert(key.clone(), value);
            if let Some(pos) = self.fifo_keys.iter().position(|k| k == &key) {
                self.fifo_keys.remove(pos);
            }
            self.fifo_keys.push_back(key);
            return;
        }
        while self.entries.len() >= Self::MAX_ENTRIES {
            match self.fifo_keys.pop_front() {
                Some(old) => {
                    self.entries.remove(&old);
                }
                None => {
                    if let Some(k) = self.entries.keys().next().cloned() {
                        self.entries.remove(&k);
                    } else {
                        break;
                    }
                }
            }
        }
        self.fifo_keys.push_back(key.clone());
        self.entries.insert(key, value);
    }
}

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
    /// `POST /api/instances/:id/migrate` replay cache when `Idempotency-Key` is set.
    ///
    /// Tokio mutex so the handler can hold the guard across `migrate_instance().await` and
    /// preserve idempotency under concurrent duplicate keys (reference server only).
    pub migrate_idempotency: Arc<tokio::sync::Mutex<MigrateIdempotencyCache>>,
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
            anyhow::bail!(
                "WOS_RUNTIME=restate: Axum composition root is still `AppRuntime` (runtime-local). \
                 `RestateRuntimeAdapter` exists for tests/CI (`wos-restate-worker`, ingress smoke); \
                 wiring it as the server `AppState.runtime` is **WS-104** (see `crates/wos-server/TODO.md`). \
                 Until then, use WOS_RUNTIME=local. \
                 Adapter ops still unsupported for full server parity: persist_task_draft, \
                 submit_task_response, dismiss_task, load_provenance_window, migrate_instance \
                 (see wos-spec/thoughts/plans/2026-05-01-pln0333-ws094-acceptance-checklist.md)."
            );
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
        migrate_idempotency: Arc::new(tokio::sync::Mutex::new(MigrateIdempotencyCache::default())),
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

#[cfg(test)]
mod migrate_idempotency_cache_tests {
    use super::MigrateIdempotencyCache;
    use wos_runtime::{MigrationMap, MigrationOutcome};

    #[test]
    fn insert_evicts_oldest_when_over_cap() {
        let cap = MigrateIdempotencyCache::MAX_ENTRIES;
        let mut c = MigrateIdempotencyCache::default();
        let mk = |prev: &str, new: &str| MigrationOutcome {
            instance_id: "i".into(),
            previous_definition_version: prev.into(),
            new_definition_version: new.into(),
            migration_map: MigrationMap::default(),
        };
        for i in 0..cap {
            c.insert(format!("k{i}"), mk("0", "1"));
        }
        assert!(c.get("k0").is_some());
        c.insert("k_new".into(), mk("1", "2"));
        assert!(c.get("k0").is_none());
        assert!(c.get("k_new").is_some());
    }

    #[test]
    fn insert_replace_refreshes_fifo_position() {
        let cap = MigrateIdempotencyCache::MAX_ENTRIES;
        let mut c = MigrateIdempotencyCache::default();
        let mk = |tag: &str| MigrationOutcome {
            instance_id: "i".into(),
            previous_definition_version: "0".into(),
            new_definition_version: tag.into(),
            migration_map: MigrationMap::default(),
        };
        for i in 0..cap {
            c.insert(format!("k{i}"), mk("fill"));
        }
        assert!(c.get("k0").is_some());
        c.insert("k0".into(), mk("refresh-k0"));
        c.insert("k_new".into(), mk("brand-new"));
        assert!(
            c.get("k0").is_some(),
            "k0 should survive eviction after insert-refresh moved it to the FIFO back"
        );
        assert!(
            c.get("k1").is_none(),
            "k1 stayed at the FIFO front and should have been evicted before k0"
        );
        assert!(c.get("k_new").is_some());
    }
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
