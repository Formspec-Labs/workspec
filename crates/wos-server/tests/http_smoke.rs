//! End-to-end HTTP smoke test.
//!
//! Builds a real `AppState` (in-memory SQLite + mock auth) and hits the
//! axum router through `tower::ServiceExt::oneshot`, bypassing the TCP
//! listener so the test stays fast and deterministic.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use wos_server::config::{AuthKind, ServerConfig, StorageKind};
use wos_server::{AppState, auth, http, services::AppServices, storage};

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
        seed: false,
        ai_chat: wos_server::config::AiChatKind::Disabled,
        gemini_api_key: String::new(),
        cursor_throttle_ms: 50,
        timer_poll_ms: 1000,
    });
    let storage = storage::build(&cfg).await.unwrap();
    let auth = auth::build(&cfg, storage.clone());
    let services = Arc::new(AppServices::new(cfg.clone(), storage.clone()).await.unwrap());
    AppState {
        cfg,
        storage,
        auth,
        services,
    }
}

fn app(state: AppState) -> axum::Router {
    let (router, io_layer) = http::router(state);
    router.layer(io_layer)
}

#[tokio::test]
async fn health_endpoint_returns_ok_status() {
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
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    // Governance service returns an empty ServiceHealthView[] when no
    // bundles are loaded. The response is a JSON array, not an object.
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
