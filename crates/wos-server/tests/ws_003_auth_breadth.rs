//! WS-003: every mutator demands auth (Supervisor or Adjudicator), with
//! `/applicant/:id/appeal` carrying interim `RequireAuth` until per-actor
//! scoping (WS-091) lands. The full sweep relies on the `RequireRole<R>`
//! extractor (WS-083) — this fixture spot-checks routes the pre-existing
//! suite does not already lock.

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

async fn jwt_state() -> AppState {
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
    for (id, role) in [
        ("sup", "Supervisor"),
        ("adj", "Adjudicator"),
        ("app", "Applicant"),
    ] {
        store
            .upsert_user(&UserRow {
                id: id.into(),
                email: format!("{id}@example.com"),
                name: id.into(),
                role: role.into(),
                password_hash: hash.clone(),
                avatar: None,
                auth_epoch: 0,
                created_at: Utc::now(),
            })
            .await
            .unwrap();
    }

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

async fn login_for(app: axum::Router, email: &str) -> String {
    let body = serde_json::json!({ "email": email, "password": "wos-dev" });
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
    let bytes = axum::body::to_bytes(res.into_body(), 8192).await.unwrap();
    let pair: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    pair["accessToken"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn drain_rejects_anonymous_and_non_supervisor() {
    let state = jwt_state().await;
    let app = http::router(state);

    let anon = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/instances/any-id/drain")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(anon.status(), StatusCode::UNAUTHORIZED);

    let adj_token = login_for(app.clone(), "adj@example.com").await;
    let wrong_role = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/instances/any-id/drain")
                .header("authorization", format!("Bearer {adj_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(wrong_role.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn agents_register_rejects_anonymous_and_non_supervisor() {
    let state = jwt_state().await;
    let app = http::router(state);

    let anon = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(anon.status(), StatusCode::UNAUTHORIZED);

    let app_token = login_for(app.clone(), "app@example.com").await;
    let wrong_role = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents")
                .header("authorization", format!("Bearer {app_token}"))
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(wrong_role.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn equity_evaluate_rejects_anonymous() {
    let state = jwt_state().await;
    let app = http::router(state);

    let anon = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/equity/evaluate")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(anon.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn applicant_appeal_requires_auth_until_ws_091_lands() {
    let state = jwt_state().await;
    let app = http::router(state);

    let anon = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/applicant/any/appeal")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"reason":"x"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(anon.status(), StatusCode::UNAUTHORIZED);

    // Any authenticated role is accepted at this layer; per-actor scoping
    // (own-case applicant only) lands with WS-091. Until then, an
    // authenticated request gets past the auth check and surfaces whatever
    // the underlying service produces (here: 404 because the instance is
    // synthetic), confirming the gate is auth-only and not role-only.
    let app_token = login_for(app.clone(), "app@example.com").await;
    let authed = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/applicant/any/appeal")
                .header("authorization", format!("Bearer {app_token}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"reason":"x"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(authed.status(), StatusCode::UNAUTHORIZED);
    assert_ne!(authed.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn integration_inbound_rejects_anonymous() {
    let state = jwt_state().await;
    let app = http::router(state);

    let anon = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/events/inbound")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(anon.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn assurance_record_rejects_anonymous() {
    let state = jwt_state().await;
    let app = http::router(state);

    let anon = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/instances/x/identity-facts")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(anon.status(), StatusCode::UNAUTHORIZED);
}
