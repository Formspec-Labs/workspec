//! HTTP `/api/auth/logout` uses the Bearer access JWT and must end the whole
//! login session (access + refresh), not only the access `jti`.

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
use wos_server::{AppState, auth, http, realtime, services::AppServices};
use wos_server::storage::{SqliteStorage, Storage, UserRow};

async fn jwt_app_state() -> AppState {
    jwt_app_state_with(false).await
}

async fn jwt_app_state_with(bearer_strict: bool) -> AppState {
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
        bearer_strict,
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

#[tokio::test]
async fn http_logout_revokes_access_via_bearer_jwt() {
    let state = jwt_app_state().await;
    let app = http::router(state);

    let login_body = serde_json::json!({
        "email": "user@example.com",
        "password": "wos-dev"
    });
    let login_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(login_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login_res.status(), StatusCode::OK);
    let login_bytes = axum::body::to_bytes(login_res.into_body(), 8192)
        .await
        .unwrap();
    let pair: serde_json::Value = serde_json::from_slice(&login_bytes).unwrap();
    let access = pair
        .get("accessToken")
        .and_then(|v| v.as_str())
        .expect("accessToken in login response")
        .to_string();
    let refresh = pair
        .get("refreshToken")
        .and_then(|v| v.as_str())
        .expect("refreshToken in login response")
        .to_string();

    let me_ok = app
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
    assert_eq!(me_ok.status(), StatusCode::OK);

    let logout_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/logout")
                .header("authorization", format!("Bearer {access}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(logout_res.status(), StatusCode::OK);

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

    let refresh_body = serde_json::json!({ "refreshToken": refresh });
    let refresh_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/refresh")
                .header("content-type", "application/json")
                .body(Body::from(refresh_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(refresh_res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn put_kernel_requires_bearer_jwt() {
    let state = jwt_app_state().await;
    let app = http::router(state);

    let no_auth = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/bundles/any/kernel")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(no_auth.status(), StatusCode::UNAUTHORIZED);

    let login_body = serde_json::json!({
        "email": "user@example.com",
        "password": "wos-dev"
    });
    let login_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(login_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login_res.status(), StatusCode::OK);
    let login_bytes = axum::body::to_bytes(login_res.into_body(), 8192)
        .await
        .unwrap();
    let pair: serde_json::Value = serde_json::from_slice(&login_bytes).unwrap();
    let access = pair
        .get("accessToken")
        .and_then(|v| v.as_str())
        .expect("accessToken");

    let with_auth = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/bundles/any/kernel")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {access}"))
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        with_auth.status(),
        StatusCode::BAD_REQUEST,
        "invalid kernel must fail validation after auth succeeds"
    );
}

#[tokio::test]
async fn put_kernel_forbidden_for_non_supervisor() {
    let state = jwt_app_state().await;
    let app = http::router(state);

    let login_body = serde_json::json!({
        "email": "applicant@example.com",
        "password": "wos-dev"
    });
    let login_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(login_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login_res.status(), StatusCode::OK);
    let login_bytes = axum::body::to_bytes(login_res.into_body(), 8192)
        .await
        .unwrap();
    let pair: serde_json::Value = serde_json::from_slice(&login_bytes).unwrap();
    let access = pair
        .get("accessToken")
        .and_then(|v| v.as_str())
        .expect("accessToken");

    let put = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/bundles/any/kernel")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {access}"))
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(put.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn bearer_strict_rejects_invalid_jwt_on_anonymous_route() {
    let state = jwt_app_state_with(true).await;
    let app = http::router(state);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/healthz")
                .header("authorization", "Bearer not-a-real-jwt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn bearer_strict_off_invalid_bearer_does_not_block_anonymous_route() {
    let state = jwt_app_state_with(false).await;
    let app = http::router(state);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/healthz")
                .header("authorization", "Bearer not-a-real-jwt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
