//! F5 HTTP→runtime tenant pass-through integration tests.
//!
//! Guards three cases for `POST /api/instances` tenant validation per ADR 0068 D-1.1:
//!   - Negative: explicit `tenant` that doesn't match the TypeID prefix → HTTP 400.
//!   - Positive (derivation): TypeID `instanceId` + no `tenant` → 200, derived tenant.
//!   - Positive (explicit match): TypeID `instanceId` + matching `tenant` → 200.
//!
//! `TenantMismatch` and `TenantInvalid` from `wos-runtime` MUST map to HTTP 400
//! (bad_request), not 503 (service_unavailable).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use rand::rngs::OsRng;
use tower::ServiceExt;
use wos_server::config::{AuthKind, ServerConfig, SignerKind, StorageKind};
use wos_server::runtime::AppRuntime;
use wos_server::storage::{KernelRow, SqliteStorage, Storage, UserRow};
use wos_server::{AppState, auth, http, realtime, services::AppServices};

const KERNEL_URL: &str = "urn:wos:workflow:tenant-test:1.0.0";
const KERNEL_VERSION: &str = "1.0.0";

/// A TypeID-format instance id with prefix `sba-poc` — valid ADR 0068 case TypeID.
/// Format: `{tenant}_{type}_{uuidv7_base32}` where type must be `case`.
const SBA_POC_INSTANCE_ID: &str = "sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsd";

fn stub_kernel() -> serde_json::Value {
    serde_json::json!({
        "$wosWorkflow": "1.0",
        "url": KERNEL_URL,
        "version": KERNEL_VERSION,
        "title": "Tenant Pass-through Test Kernel",
        "status": "active",
        "impactLevel": "operational",
        "actors": [{"id": "applicant", "type": "human"}],
        "lifecycle": {
            "initialState": "open",
            "states": {
                "open": {"type": "atomic"}
            }
        },
        "contracts": {}
    })
}

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
            id: "sup".into(),
            email: "sup@example.com".into(),
            name: "sup".into(),
            role: "Supervisor".into(),
            password_hash: hash,
            avatar: None,
            auth_epoch: 0,
            created_at: Utc::now(),
        })
        .await
        .unwrap();
    store
        .upsert_kernel(&KernelRow {
            url: KERNEL_URL.into(),
            title: "Tenant Pass-through Test Kernel".into(),
            version: KERNEL_VERSION.into(),
            status: "active".into(),
            impact_level: "operational".into(),
            document: stub_kernel(),
            updated_at: Utc::now(),
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
        runtime: wos_server::config::RuntimeKind::Local,
        audit_sink: wos_server::config::AuditSinkKind::None,
        audit_database_url: String::new(),
        session_sweep_enabled: false,
        signer_kind: SignerKind::Noop,
    });

    let storage_handle: wos_server::storage::StorageHandle = store.clone();
    let auth = auth::build(&cfg, storage_handle.clone()).expect("auth build");
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

async fn login_supervisor(app: axum::Router) -> String {
    let body = serde_json::json!({ "email": "sup@example.com", "password": "wos-dev" });
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
    assert_eq!(res.status(), StatusCode::OK, "login must succeed");
    let bytes = axum::body::to_bytes(res.into_body(), 8192).await.unwrap();
    let pair: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    pair.get("accessToken")
        .and_then(|v| v.as_str())
        .unwrap()
        .to_string()
}

// ---------------------------------------------------------------------------
// Negative: wrong-prefix tenant → HTTP 400 (TenantMismatch)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_instance_with_wrong_tenant_prefix_returns_400() {
    let state = jwt_app_state().await;
    let app = http::router(state);
    let token = login_supervisor(app.clone()).await;

    // TypeID prefix is `sba-poc`; explicit tenant says `wrong-prefix` → TenantMismatch
    let body = serde_json::json!({
        "definitionUrl": KERNEL_URL,
        "instanceId": SBA_POC_INSTANCE_ID,
        "tenant": "wrong-prefix"
    });
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/instances")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "tenant mismatch (wrong-prefix vs sba-poc TypeID prefix) must return HTTP 400 per ADR 0068 D-1.1"
    );
}

// ---------------------------------------------------------------------------
// Positive: TypeID instanceId + no tenant → 200, tenant derived from prefix
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_instance_with_typeid_derives_tenant_from_prefix() {
    let state = jwt_app_state().await;
    let app = http::router(state);
    let token = login_supervisor(app.clone()).await;

    let body = serde_json::json!({
        "definitionUrl": KERNEL_URL,
        "instanceId": SBA_POC_INSTANCE_ID
        // no `tenant` field — runtime MUST derive from TypeID prefix
    });
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/instances")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 16384).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
    assert_eq!(
        status,
        StatusCode::OK,
        "TypeID instanceId with no explicit tenant must succeed: {v}"
    );

    let tenant = v
        .get("tenant")
        .and_then(|t| t.as_str())
        .unwrap_or_default();
    assert_eq!(
        tenant, "sba-poc",
        "tenant in response must be derived from TypeID prefix `sba-poc`: {v}"
    );
}

// ---------------------------------------------------------------------------
// Positive: TypeID instanceId + matching explicit tenant → 200
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_instance_with_typeid_and_matching_tenant_succeeds() {
    let state = jwt_app_state().await;
    let app = http::router(state);
    let token = login_supervisor(app.clone()).await;

    // Use a distinct instance id to avoid AlreadyExists conflict with the derivation test
    let instance_id = "sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqse";
    let body = serde_json::json!({
        "definitionUrl": KERNEL_URL,
        "instanceId": instance_id,
        "tenant": "sba-poc"  // matches TypeID prefix exactly
    });
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/instances")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 16384).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
    assert_eq!(
        status,
        StatusCode::OK,
        "TypeID instanceId with matching tenant must succeed: {v}"
    );

    let tenant = v
        .get("tenant")
        .and_then(|t| t.as_str())
        .unwrap_or_default();
    assert_eq!(
        tenant, "sba-poc",
        "tenant in response must be sba-poc: {v}"
    );
}
