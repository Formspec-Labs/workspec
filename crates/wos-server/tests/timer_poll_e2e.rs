//! WS-010: Timer poll end-to-end. Drives `timer_task::tick_once` against a
//! real `AppState` + `AppRuntime` to lock the wire path
//! `expired-deadline → enqueue → drain → configuration advance`. The
//! existing `tests/timer_list_pagination.rs` only covers paged
//! `list_instances` traversal, not the runtime side-effects of a fired
//! timer.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{Duration, Utc};
use wos_runtime::runtime::CreateInstanceRequest;
use wos_server::config::{AiChatKind, AuthKind, ServerConfig, SignerKind, StorageKind};
use wos_server::runtime::AppRuntime;
use wos_server::services::timer_task;
use wos_server::storage::{KernelRow, StorageError};
use wos_server::{AppState, auth, realtime, services::AppServices, storage};

fn timer_kernel(url: &str, version: &str) -> serde_json::Value {
    // Two-state lifecycle. `intake` carries one unguarded transition on
    // event `timer.fire` → `done`; that's the transition the timer's
    // event payload will satisfy.
    serde_json::json!({
        "$wosWorkflow": "1.0.0",
        "url": url,
        "version": version,
        "title": "Timer Poll Kernel",
        "status": "active",
        "lifecycle": {
            "initialState": "intake",
            "states": {
                "intake": {
                    "type": "atomic",
                    "transitions": [
                        {
                            "event": { "kind": "message", "name": "timer.fire" },
                            "target": "done"
                        }
                    ]
                },
                "done": { "type": "atomic" }
            }
        },
        "actors": [
            { "id": "system", "type": "system" }
        ],
        "contracts": {}
    })
}

async fn bring_up() -> AppState {
    let cfg = Arc::new(ServerConfig {
        port: 0,
        fixtures_dir: std::path::PathBuf::from("."),
        storage: StorageKind::Sqlite,
        database_url: "sqlite::memory:?cache=shared".into(),
        auth: AuthKind::Mock,
        jwt_secret: String::new(),
        jwt_access_ttl_secs: 900,
        jwt_refresh_ttl_secs: 7 * 24 * 3600,
        cors_origin: "http://localhost:3000".into(),
        cors_strict: false,
        bearer_strict: false,
        seed: false,
        ai_chat: AiChatKind::Disabled,
        gemini_api_key: String::new(),
        cursor_throttle_ms: 50,
        timer_poll_ms: 1000,
        runtime: wos_server::config::RuntimeKind::Local,
        audit_sink: wos_server::config::AuditSinkKind::None,
        audit_database_url: String::new(),
        session_sweep_enabled: true,
        signer_kind: SignerKind::Noop,
    });
    let storage = storage::build(&cfg).await.unwrap();

    storage
        .upsert_kernel(&KernelRow {
            url: "urn:wos:workflow:timer-poll-e2e:1.0.0".into(),
            title: "Timer Poll Kernel".into(),
            version: "1.0.0".into(),
            status: "active".into(),
            impact_level: "operational".into(),
            document: timer_kernel("urn:wos:workflow:timer-poll-e2e:1.0.0", "1.0.0"),
            updated_at: Utc::now(),
        })
        .await
        .unwrap();

    let auth = auth::build(&cfg, storage.clone()).expect("auth build");
    let services = Arc::new(
        AppServices::new(cfg.clone(), storage.clone())
            .await
            .unwrap(),
    );
    let (_layer, io) = realtime::build_io_only();
    let runtime = AppRuntime::build(
        storage.clone(),
        services.provenance.clone(),
        services.bundle.clone(),
        io,
    );
    AppState {
        cfg,
        storage,
        auth,
        services,
        runtime,
        event_idempotency: Arc::new(Mutex::new(HashMap::new())),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tick_once_fires_expired_timer_and_advances_instance() {
    let state = bring_up().await;

    // Mint a fresh instance through the runtime so the stored
    // `instance_json` matches the live `CaseInstance` schema verbatim.
    let created = state
        .runtime
        .create_instance(CreateInstanceRequest {
            instance_id: "urn:wos:instance:timer-poll-e2e:one".into(),
            definition_url: "urn:wos:workflow:timer-poll-e2e:1.0.0".into(),
            definition_version: "1.0.0".into(),
            initial_case_state: None,
        })
        .await
        .expect("create instance");
    let instance_id = created.instance_id.clone();

    // Sanity: brand-new instance is in `intake`.
    let before = state
        .runtime
        .load_instance(&instance_id)
        .await
        .expect("load before");
    assert_eq!(before.configuration, vec!["intake".to_string()]);

    // Inject one already-expired timer pointing at the unguarded
    // `timer.fire` transition. Five-minutes-ago is comfortably past
    // the `tick_once` `now > deadline` check without flake risk.
    let expired_deadline = (Utc::now() - Duration::minutes(5)).to_rfc3339();
    state
        .storage
        .update_instance_atomic(
            &instance_id,
            &|row| {
                let timers = row
                    .instance_json
                    .get_mut("timers")
                    .ok_or_else(|| StorageError::Conflict("timers field missing".into()))?;
                *timers = serde_json::json!([
                    {
                        "timerId": "t-1",
                        "deadline": expired_deadline,
                        "event": "timer.fire",
                        "scopeState": "intake"
                    }
                ]);
                Ok(Vec::new())
            },
        )
        .await
        .expect("inject timer");

    // Drive one timer-poll sweep. This should: (a) walk the instance
    // page, (b) detect the expired timer, (c) `enqueue_event` then
    // `drain_once`, advancing the instance to `done`.
    timer_task::tick_once(&state).await.expect("tick_once");

    // (b) Configuration advanced from `intake` to `done`.
    let after = state
        .runtime
        .load_instance(&instance_id)
        .await
        .expect("load after");
    assert_eq!(
        after.configuration,
        vec!["done".to_string()],
        "expired timer should have driven intake → done"
    );

    // (a) + (c) Provenance carries the timer-derived event. We look at
    // the runtime-visible window — at minimum we expect a record whose
    // payload mentions the `timer.fire` event and the `system:timer`
    // actor that `tick_once` synthesises.
    let provenance = state
        .runtime
        .load_provenance_window(&instance_id, 0, 64)
        .await
        .expect("load provenance");
    let serialized: Vec<serde_json::Value> = provenance
        .iter()
        .map(|r| serde_json::to_value(r).expect("serialize record"))
        .collect();
    let mentions_timer_event = serialized
        .iter()
        .any(|v| v.to_string().contains("timer.fire"));
    assert!(
        mentions_timer_event,
        "expected provenance to record the timer-derived `timer.fire` event; got: {serialized:?}"
    );
    let mentions_timer_actor = serialized
        .iter()
        .any(|v| v.to_string().contains("system:timer"));
    assert!(
        mentions_timer_actor,
        "expected provenance to attribute the event to actor `system:timer`; got: {serialized:?}"
    );
}
