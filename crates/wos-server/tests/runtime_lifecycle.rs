//! Phase 1 end-to-end: boot a minimal AppState with AppRuntime, create a
//! case instance through the HTTP surface, and confirm it persists and is
//! readable through the instance endpoints.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use tower::ServiceExt;
use wos_server::config::{AiChatKind, AuthKind, ServerConfig, StorageKind};
use wos_server::runtime::AppRuntime;
use wos_server::storage::{KernelRow, Storage};
use wos_server::{AppState, auth, http, realtime, services::AppServices, storage};

/// Minimal kernel stub sufficient for `Evaluator::new(kernel)` to succeed
/// when wos-runtime enters the initial state.
fn stub_kernel_document(url: &str, version: &str) -> serde_json::Value {
    serde_json::json!({
        "$wosKernel": "1.0.0",
        "url": url,
        "version": version,
        "title": "Test Kernel",
        "status": "active",
        "lifecycle": {
            "initialState": "intake",
            "states": {
                "intake": { "type": "atomic" }
            }
        },
        "actors": [
            { "id": "applicant", "type": "human" }
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
        seed: false,
        ai_chat: AiChatKind::Disabled,
        gemini_api_key: String::new(),
        cursor_throttle_ms: 50,
        timer_poll_ms: 1000,
    });
    let storage = storage::build(&cfg).await.unwrap();

    // Plant a kernel row so AppRuntime's resolver can find it.
    storage
        .upsert_kernel(&KernelRow {
            url: "urn:wos:workflow:test:1.0.0".into(),
            title: "Test Kernel".into(),
            version: "1.0.0".into(),
            status: "active".into(),
            impact_level: "operational".into(),
            document: stub_kernel_document("urn:wos:workflow:test:1.0.0", "1.0.0"),
            updated_at: Utc::now(),
        })
        .await
        .unwrap();

    let auth = auth::build(&cfg, storage.clone());
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
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_instance_via_http_roundtrips_through_runtime() {
    let state = bring_up().await;
    let app = http::router(state.clone());

    let body = serde_json::json!({
        "definitionUrl": "urn:wos:workflow:test:1.0.0",
        "definitionVersion": "1.0.0",
        "instanceId": "urn:wos:instance:test:smoke"
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/instances")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK, "POST /api/instances should succeed");

    // Fetch back through GET /instances/:id
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/instances/urn%3Awos%3Ainstance%3Atest%3Asmoke")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 8192).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        v.get("instanceId").and_then(|x| x.as_str()),
        Some("urn:wos:instance:test:smoke")
    );
    // The runtime should have entered the initial "intake" state.
    let config = v.get("configuration").and_then(|x| x.as_array()).unwrap();
    assert_eq!(config.len(), 1);
    assert_eq!(config[0].as_str(), Some("intake"));
}
