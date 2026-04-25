//! End-to-end HTTP smoke test.
//!
//! Builds a real `AppState` (in-memory SQLite + mock auth) and hits the
//! axum router through `tower::ServiceExt::oneshot`, bypassing the TCP
//! listener so the test stays fast and deterministic.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;
use wos_server::config::{AuthKind, ServerConfig, StorageKind};
use wos_server::runtime::AppRuntime;
use wos_server::{AppState, auth, http, realtime, services::AppServices, storage};

async fn build_app_state() -> AppState {
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
        ai_chat: wos_server::config::AiChatKind::Disabled,
        gemini_api_key: String::new(),
        cursor_throttle_ms: 50,
        timer_poll_ms: 1000,
        signer_kind: wos_server::config::SignerKind::Noop,
    });
    let storage = storage::build(&cfg).await.unwrap();
    let auth = auth::build(&cfg, storage.clone());
    let services = Arc::new(AppServices::new(cfg.clone(), storage.clone()).await.unwrap());
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

fn app(state: AppState) -> axum::Router {
    http::router(state)
}

#[tokio::test]
async fn healthz_returns_infra_liveness() {
    let state = build_app_state().await;
    let response = app(state)
        .oneshot(
            Request::builder()
                .uri("/api/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json.get("status").and_then(|v| v.as_str()), Some("ok"));
}

#[tokio::test]
async fn health_returns_service_health_array() {
    let state = build_app_state().await;
    let response = app(state)
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    // Governance /health returns a `ServiceHealthView[]`; empty when no
    // bundles are loaded.
    assert!(json.is_array(), "health response must be an array, got {json}");
}

#[tokio::test]
async fn bundles_list_is_empty_without_fixtures() {
    let state = build_app_state().await;
    let response = app(state)
        .oneshot(
            Request::builder()
                .uri("/api/bundles")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(arr.len(), 0);
}

#[tokio::test]
async fn dashboard_metrics_returns_deterministic_shape() {
    let state = build_app_state().await;
    let response = app(state)
        .oneshot(
            Request::builder()
                .uri("/api/dashboard/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    let m: serde_json::Value = serde_json::from_slice(&body).unwrap();
    // Shape fields (studio `DashboardMetrics` contract).
    for key in [
        "activeInstances",
        "completed7d",
        "slaCompliance",
        "avgProcessingTimeDays",
        "aiAcceptanceRate",
    ] {
        assert!(
            m.get(key).is_some(),
            "dashboard metrics missing `{key}`: {m}"
        );
    }
    let synthetic = m
        .get("syntheticFields")
        .and_then(|v| v.as_array())
        .expect("dashboard metrics should include syntheticFields array");
    assert!(
        !synthetic.is_empty(),
        "syntheticFields should list stub metric keys for studio consumers: {m:?}"
    );
}

#[tokio::test]
async fn nonexistent_instance_returns_404() {
    let state = build_app_state().await;
    let response = app(state)
        .oneshot(
            Request::builder()
                .uri("/api/instances/nope")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn put_kernel_requires_bearer_token() {
    let state = build_app_state().await;
    let response = app(state)
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/bundles/foo/kernel")
                .header("content-type", "application/json")
                .body(Body::from(json!({"url": "x"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "PUT /api/bundles/.../kernel must require Authorization"
    );
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let state = build_app_state().await;
    let response = app(state)
        .oneshot(
            Request::builder()
                .uri("/api/does-not-exist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
