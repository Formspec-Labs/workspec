//! Tests for WS-001 (ai/chat Supervisor gate), WS-031 (chain-verify endpoint),
//! and WS-032 (event idempotency). Builds a JWT-backed `AppState` and hits
//! the axum router through `tower::ServiceExt::oneshot`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use rand::rngs::OsRng;
use tower::ServiceExt;
use wos_server::config::{AuthKind, ServerConfig, StorageKind};
use wos_server::runtime::AppRuntime;
use wos_server::{AppState, auth, http, realtime, services::AppServices, storage};
use wos_server::storage::{SqliteStorage, Storage, UserRow};

async fn jwt_app_state() -> AppState {
    let store = Arc::new(
        SqliteStorage::connect("sqlite::memory:?cache=shared")
            .await
            .unwrap(),
    );
    store.migrate().await.unwrap();
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(b"wos-dev", &salt)
        .unwrap()
        .to_string();
    store
        .upsert_user(&UserRow {
            id: "u1".into(),
            email: "supervisor@example.com".into(),
            name: "Supervisor".into(),
            role: "Supervisor".into(),
            password_hash: hash.clone(),
            avatar: None,
            auth_epoch: 0,
            created_at: Utc::now(),
        })
        .await
        .unwrap();
    store
        .upsert_user(&UserRow {
            id: "u2".into(),
            email: "applicant@example.com".into(),
            name: "Applicant".into(),
            role: "Applicant".into(),
            password_hash: hash,
            avatar: None,
            auth_epoch: 0,
            created_at: Utc::now(),
        })
        .await
        .unwrap();

    let cfg = Arc::new(ServerConfig {
        port: 0,
        fixtures_dir: std::path::PathBuf::from("."),
        storage: StorageKind::Sqlite,
        database_url: "sqlite::memory:?cache=shared".into(),
        auth: AuthKind::Jwt,
        jwt_secret: "test-secret-not-for-prod".into(),
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
    });

    let storage_handle: storage::StorageHandle = store.clone();
    let auth = auth::build(&cfg, storage_handle.clone());
    let services = Arc::new(
        AppServices::new(cfg.clone(), storage_handle.clone())
            .await
            .unwrap(),
    );
    let (_layer, io) = realtime::build_io_only();
    let runtime = AppRuntime::build(
        storage_handle.clone(),
        services.provenance.clone(),
        services.bundle.clone(),
        io,
    );

    AppState {
        cfg,
        storage: storage_handle,
        auth,
        services,
        runtime,
        event_idempotency: Arc::new(Mutex::new(HashMap::new())),
    }
}

async fn login_as(app: &axum::Router, email: &str) -> String {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "email": email, "password": "wos-dev" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 8192)
        .await
        .unwrap();
    let pair: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    pair.get("accessToken")
        .and_then(|v| v.as_str())
        .expect("accessToken")
        .to_string()
}

#[tokio::test]
async fn ai_chat_returns_503_when_disabled() {
    let state = jwt_app_state().await;
    let app = http::router(state);
    let token = login_as(&app, "supervisor@example.com").await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ai/chat")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"prompt":"hello"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn ai_chat_forbidden_for_non_supervisor() {
    let state = jwt_app_state().await;
    let app = http::router(state);
    let token = login_as(&app, "applicant@example.com").await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ai/chat")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"prompt":"hello"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn ai_chat_unauthorized_without_token() {
    let state = jwt_app_state().await;
    let app = http::router(state);

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ai/chat")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"prompt":"hello"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn provenance_verify_returns_valid_on_empty() {
    let state = jwt_app_state().await;
    let app = http::router(state);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/instances/no-such-instance/provenance/verify")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 4096)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["valid"], true);
    assert!(json["brokenAt"].is_null());
}
