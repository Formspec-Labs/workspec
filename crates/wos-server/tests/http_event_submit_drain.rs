//! WS-012: `POST /api/instances/{id}/events` drives `AppRuntime::enqueue_event`
//! + `drain_once`, derives `head_record` from `ProvenanceService` seq math,
//! and short-circuits on `idempotencyToken` so duplicate submits return the
//! cached `EvaluationResultView` without writing fresh provenance rows.
//!
//! `runtime_lifecycle.rs` covers create + fetch only — this fixture closes
//! the F6 review gap by exercising the drain path end-to-end through HTTP.

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

const KERNEL_URL: &str = "urn:wos:workflow:ws012:1.0.0";
const KERNEL_VERSION: &str = "1.0.0";

/// Two-state kernel reachable by an unguarded `advance` event.
fn stub_kernel_document() -> serde_json::Value {
    serde_json::json!({
        "$wosKernel": "1.0",
        "url": KERNEL_URL,
        "version": KERNEL_VERSION,
        "title": "WS-012 Drain Kernel",
        "status": "active",
        "impactLevel": "operational",
        "actors": [
            { "id": "applicant", "type": "human" },
            { "id": "adjudicator", "type": "human" }
        ],
        "lifecycle": {
            "initialState": "intake",
            "states": {
                "intake": {
                    "type": "atomic",
                    "transitions": [
                        {
                            "event": { "kind": "message", "name": "advance" },
                            "target": "reviewed"
                        }
                    ]
                },
                "reviewed": { "type": "atomic" }
            }
        },
        "contracts": {}
    })
}

/// JWT-backed AppState seeded with Supervisor + Adjudicator users so the
/// fixture can hit `RequireRole<Supervisor>` (create instance) and
/// `RequireRole<Adjudicator>` (submit event) in the same flow.
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
    for (id, role) in [("sup", "Supervisor"), ("adj", "Adjudicator")] {
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

    store
        .upsert_kernel(&KernelRow {
            url: KERNEL_URL.into(),
            title: "WS-012 Drain Kernel".into(),
            version: KERNEL_VERSION.into(),
            status: "active".into(),
            impact_level: "operational".into(),
            document: stub_kernel_document(),
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
        session_sweep_enabled: true,
        signer_kind: SignerKind::Noop,
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

async fn login(app: axum::Router, email: &str) -> String {
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
    assert_eq!(res.status(), StatusCode::OK, "login should succeed");
    let bytes = axum::body::to_bytes(res.into_body(), 8192).await.unwrap();
    let pair: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    pair.get("accessToken")
        .and_then(|v| v.as_str())
        .unwrap()
        .to_string()
}

async fn create_instance(app: axum::Router, supervisor_token: &str) -> String {
    let body = serde_json::json!({
        "definitionUrl": KERNEL_URL,
        "definitionVersion": KERNEL_VERSION
    });
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/instances")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {supervisor_token}"))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "POST /api/instances should succeed for Supervisor"
    );
    let bytes = axum::body::to_bytes(res.into_body(), 16384).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    v.get("instanceId")
        .and_then(|x| x.as_str())
        .expect("instanceId in create response")
        .to_string()
}

async fn fetch_provenance(app: axum::Router, instance_id: &str) -> Vec<serde_json::Value> {
    let encoded = instance_id.replace(':', "%3A");
    let res = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/instances/{encoded}/provenance"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 65536).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn submit_event(
    app: axum::Router,
    instance_id: &str,
    adjudicator_token: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let encoded = instance_id.replace(':', "%3A");
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/instances/{encoded}/events"))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {adjudicator_token}"))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 65536).await.unwrap();
    let v: serde_json::Value = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, v)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn submit_event_advances_configuration_and_emits_provenance() {
    let state = jwt_app_state().await;
    let app = http::router(state);

    let sup_token = login(app.clone(), "sup@example.com").await;
    let adj_token = login(app.clone(), "adj@example.com").await;
    let instance_id = create_instance(app.clone(), &sup_token).await;

    let prov_before = fetch_provenance(app.clone(), &instance_id).await;
    let prov_before_len = prov_before.len();

    let (status, view) = submit_event(
        app.clone(),
        &instance_id,
        &adj_token,
        serde_json::json!({
            "event": "advance",
            "actorId": "adj",
            "data": null
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "submit_event should succeed: {view}");

    // (a) head_record present and non-null.
    let head = view
        .get("headRecord")
        .expect("headRecord key in EvaluationResultView");
    assert!(
        !head.is_null(),
        "headRecord should be non-null after a drain emits provenance: {view:?}"
    );

    // (b) configuration advanced from initial `intake` to `reviewed`.
    let prev: Vec<String> = view
        .get("previousConfiguration")
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let next: Vec<String> = view
        .get("newConfiguration")
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(prev, vec!["intake".to_string()], "previous config = intake");
    assert_eq!(
        next,
        vec!["reviewed".to_string()],
        "new config = reviewed (drain advanced state)"
    );
    assert_ne!(prev, next, "configuration must change across drain");

    // (c) provenance fetch returns at least one new row vs baseline.
    let prov_after = fetch_provenance(app.clone(), &instance_id).await;
    assert!(
        prov_after.len() > prov_before_len,
        "drain should emit at least one new provenance row \
         (before={prov_before_len}, after={})",
        prov_after.len()
    );

    // (d) typed-field assertions on the new rows. Walk the rows added by
    // this drain and pin the exact `event` + `actorId` shape — substring
    // matches are fragile, this contract pins the canonical field names.
    let new_rows = &prov_after[prov_before_len..];
    let advance_row = new_rows
        .iter()
        .find(|r| r.get("event").and_then(|v| v.as_str()) == Some("advance"))
        .unwrap_or_else(|| {
            panic!(
                "expected at least one new provenance row with `event == \"advance\"`; got new_rows={new_rows:?}"
            )
        });
    assert_eq!(
        advance_row.get("actorId").and_then(|v| v.as_str()),
        Some("adj"),
        "advance row must record actorId == \"adj\"; row={advance_row:?}"
    );
    assert_eq!(
        advance_row.get("event").and_then(|v| v.as_str()),
        Some("advance"),
        "advance row event must be exactly \"advance\"; row={advance_row:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn submit_event_dedup_replays_with_idempotency_token() {
    let state = jwt_app_state().await;
    let app = http::router(state);

    let sup_token = login(app.clone(), "sup@example.com").await;
    let adj_token = login(app.clone(), "adj@example.com").await;
    let instance_id = create_instance(app.clone(), &sup_token).await;

    let body = serde_json::json!({
        "event": "advance",
        "actorId": "adj",
        "data": null,
        "idempotencyToken": "tok-1"
    });

    let (status_a, first) =
        submit_event(app.clone(), &instance_id, &adj_token, body.clone()).await;
    assert_eq!(status_a, StatusCode::OK);

    let prov_after_first = fetch_provenance(app.clone(), &instance_id).await;
    let prov_len_after_first = prov_after_first.len();
    assert!(
        prov_len_after_first >= 1,
        "first submit must have produced provenance rows"
    );

    let (status_b, second) =
        submit_event(app.clone(), &instance_id, &adj_token, body.clone()).await;
    assert_eq!(status_b, StatusCode::OK);

    // Byte-equivalent response (cache returns clone of cached view).
    assert_eq!(
        first, second,
        "duplicate idempotencyToken must return the cached EvaluationResultView verbatim"
    );

    // No additional provenance rows written by the replayed call.
    let prov_after_second = fetch_provenance(app.clone(), &instance_id).await;
    assert_eq!(
        prov_after_second.len(),
        prov_len_after_first,
        "replay branch must not write duplicate provenance rows \
         (after_first={prov_len_after_first}, after_second={})",
        prov_after_second.len()
    );
}
