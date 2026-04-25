//! WS-002: HTTP `/api/auth/change-password` rotates the hash, bumps
//! `auth_epoch`, and revokes existing sessions in one transaction. Old
//! tokens MUST stop verifying as soon as the rotation completes.

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
use wos_server::storage::{SqliteStorage, Storage, UserRow};
use wos_server::{AppState, auth, http, realtime, services::AppServices};

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
            email: "user@example.com".into(),
            name: "User One".into(),
            role: "Supervisor".into(),
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
        session_sweep_enabled: true,
        signer_kind: wos_server::config::SignerKind::Noop,
    });

    let storage_handle: wos_server::storage::StorageHandle = store.clone();
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

async fn login(app: axum::Router, email: &str, password: &str) -> (StatusCode, Option<String>) {
    let body = serde_json::json!({ "email": email, "password": password });
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    if status != StatusCode::OK {
        return (status, None);
    }
    let bytes = axum::body::to_bytes(res.into_body(), 8192).await.unwrap();
    let pair: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let access = pair
        .get("accessToken")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    (status, access)
}

#[tokio::test]
async fn change_password_rejects_wrong_current_password() {
    let state = jwt_app_state().await;
    let app = http::router(state);
    let (_, access) = login(app.clone(), "user@example.com", "wos-dev").await;
    let access = access.unwrap();

    let body = serde_json::json!({
        "currentPassword": "not-the-real-password",
        "newPassword": "another-strong-password"
    });
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/change-password")
                .header("authorization", format!("Bearer {access}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn change_password_rejects_short_new_password() {
    let state = jwt_app_state().await;
    let app = http::router(state);
    let (_, access) = login(app.clone(), "user@example.com", "wos-dev").await;
    let access = access.unwrap();

    let body = serde_json::json!({
        "currentPassword": "wos-dev",
        "newPassword": "short"
    });
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/change-password")
                .header("authorization", format!("Bearer {access}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn change_password_rejects_anonymous() {
    let state = jwt_app_state().await;
    let app = http::router(state);

    let body = serde_json::json!({
        "currentPassword": "wos-dev",
        "newPassword": "another-strong-password"
    });
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/change-password")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn change_password_rotates_hash_and_invalidates_old_token() {
    let state = jwt_app_state().await;
    let app = http::router(state);
    let (_, access) = login(app.clone(), "user@example.com", "wos-dev").await;
    let access = access.unwrap();

    let body = serde_json::json!({
        "currentPassword": "wos-dev",
        "newPassword": "next-strong-password"
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/change-password")
                .header("authorization", format!("Bearer {access}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Old access token must be rejected — `set_user_password_hash` bumps
    // `auth_epoch` and revokes sessions in the same transaction.
    let me_after = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
                .header("authorization", format!("Bearer {access}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(me_after.status(), StatusCode::UNAUTHORIZED);

    // Old password no longer logs in.
    let (status_old, _) = login(app.clone(), "user@example.com", "wos-dev").await;
    assert_eq!(status_old, StatusCode::UNAUTHORIZED);

    // New password works.
    let (status_new, new_access) =
        login(app.clone(), "user@example.com", "next-strong-password").await;
    assert_eq!(status_new, StatusCode::OK);
    assert!(new_access.is_some());
}
