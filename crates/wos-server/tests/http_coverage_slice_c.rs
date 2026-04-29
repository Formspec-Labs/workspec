//! WS-014 slice C: advanced verify, constraint zones, equity evaluate,
//! integration invoke + inbound, assurance record + upgrade, adverse notice
//! (WS-036). Extends slices A + B.

#[path = "http_coverage_shared/harness.rs"]
mod harness;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use rand::rngs::OsRng;
use tower::ServiceExt;
use wos_server::config::{AiChatKind, AuthKind, ServerConfig, SignerKind, StorageKind};
use wos_server::http;
use wos_server::storage::{InstanceRow, SqliteStorage, Storage, UserRow};
use wos_server::{AppState, auth, realtime, services::AppServices, storage};

use harness::{bring_up_with_cfg, make_instance_row, stub_config};

const SLUG: &str = "ws014slicec";
const WORKFLOW_URL: &str = "urn:wos:workflow:ws014slicec:1.0.0";

fn workflow_path_encoded() -> String {
    WORKFLOW_URL.replace(':', "%3A")
}

fn setup_tempdir() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    let kernel = serde_json::json!({
        "$wosWorkflow": "1.0",
        "url": WORKFLOW_URL,
        "version": "1.0.0",
        "title": "WS-014 slice C",
        "status": "active",
        "impactLevel": "operational",
        "actors": [{ "id": "sys", "type": "system" }],
        "lifecycle": {
            "initialState": "done",
            "states": { "done": { "type": "final" } }
        },
        "contracts": {}
    });
    std::fs::create_dir_all(root.join("kernel")).unwrap();
    std::fs::write(
        root.join("kernel").join(format!("{SLUG}.json")),
        serde_json::to_vec_pretty(&kernel).unwrap(),
    )
    .unwrap();

    let advanced = serde_json::json!({
        "verifiableConstraints": [
            { "id": "c1", "expression": "x > 0" },
            { "id": "c2", "expression": "y < 10" }
        ],
        "constraintZones": [
            {
                "id": "zone-a",
                "description": "Primary zone",
                "activities": [
                    { "id": "act-1", "label": "Submit" },
                    { "id": "act-2", "label": "Review" }
                ],
                "relations": [
                    { "source": "act-1", "target": "act-2", "type": "condition" }
                ]
            }
        ]
    });
    std::fs::create_dir_all(root.join("advanced")).unwrap();
    std::fs::write(
        root.join("advanced").join(format!("{SLUG}.json")),
        serde_json::to_vec_pretty(&advanced).unwrap(),
    )
    .unwrap();

    let ip = serde_json::json!({
        "bindings": [
            { "id": "adjudicate", "type": "http", "url": "https://example.com/adj" }
        ]
    });
    std::fs::create_dir_all(root.join("integration-profile")).unwrap();
    std::fs::write(
        root.join("integration-profile").join(format!("{SLUG}.json")),
        serde_json::to_vec_pretty(&ip).unwrap(),
    )
    .unwrap();

    dir
}

async fn jwt_state(fixtures_dir: PathBuf) -> AppState {
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
        fixtures_dir,
        storage: StorageKind::Sqlite,
        database_url: "sqlite::memory:?cache=shared".into(),
        auth: AuthKind::Jwt,
        jwt_secret: "test-secret-slice-c".into(),
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
        session_sweep_enabled: false,
        signer_kind: SignerKind::Noop,
    });

    let st: storage::StorageHandle = store.clone();
    let au = auth::build(&cfg, st.clone());
    let svc = Arc::new(AppServices::new(cfg.clone(), st.clone()).await.unwrap());
    let (_layer, io) = realtime::build_io_only();
    let rt = wos_server::runtime::AppRuntime::build(
        st.clone(),
        svc.provenance.clone(),
        svc.bundle.clone(),
        io,
    );
    AppState {
        cfg,
        storage: st,
        auth: au,
        services: svc,
        runtime: rt,
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

async fn bring_up_with_fixtures() -> (tempfile::TempDir, AppState) {
    let tmp = setup_tempdir();
    let state = bring_up_with_cfg(stub_config(tmp.path().to_path_buf())).await;
    (tmp, state)
}

// ── Advanced: verification ──────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn verify_returns_inconclusive_for_known_workflow() {
    let (_tmp, state) = bring_up_with_fixtures().await;
    let app = http::router(state);

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/verification/verify")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "workflowUrl": WORKFLOW_URL,
                        "constraintSubset": ["c1"]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 64 * 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["results"][0]["constraintRef"], "c1");
    assert_eq!(v["results"][0]["result"], "inconclusive");
    assert_eq!(v["summary"]["inconclusive"], 1);
    assert_eq!(v["solver"]["name"], "noop");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn verify_unknown_workflow_returns_404() {
    let state = bring_up_with_cfg(stub_config(PathBuf::from("."))).await;
    let app = http::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/verification/verify")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "workflowUrl": "urn:wos:workflow:ghost:1.0.0" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

// ── Advanced: constraint zones ──────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn constraint_zones_returns_zones_for_workflow() {
    let (_tmp, state) = bring_up_with_fixtures().await;
    let app = http::router(state);
    let enc = workflow_path_encoded();

    let res = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/governance/{enc}/constraint-zones"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 64 * 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let zones = v.as_array().unwrap();
    assert_eq!(zones.len(), 1);
    assert_eq!(zones[0]["id"], "zone-a");
    assert_eq!(zones[0]["activities"].as_array().unwrap().len(), 2);
    assert_eq!(zones[0]["relations"].as_array().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn valid_actions_in_zone_returns_stubbed_activities() {
    let (_tmp, state) = bring_up_with_fixtures().await;
    let enc = workflow_path_encoded();
    let iid = "urn:wos:instance:test:zone-actions";
    state
        .storage
        .create_instance(&make_instance_row(iid))
        .await
        .unwrap();
    let app = http::router(state);
    let iid_enc = iid.replace(':', "%3A");

    let res = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/instances/{iid_enc}/constraint-zones/zone-a/valid-actions?workflowUrl={enc}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 64 * 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["zoneId"], "zone-a");
    assert!(v["validActions"].as_array().unwrap().len() >= 1);
}

// ── Equity evaluate (requires Supervisor JWT) ──────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn equity_evaluate_requires_supervisor_jwt() {
    let tmp = setup_tempdir();
    let state = jwt_state(tmp.path().to_path_buf()).await;
    let app = http::router(state.clone());

    let sup_token = login_for(app.clone(), "sup@example.com").await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/equity/evaluate")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {sup_token}"))
                .body(Body::from(
                    serde_json::json!({
                        "workflowUrl": WORKFLOW_URL,
                        "groupByPath": "applicant.zip"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 64 * 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["workflowUrl"], WORKFLOW_URL);
    assert!(v["groups"].is_array());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn equity_evaluate_outcome_predicate_returns_400() {
    let tmp = setup_tempdir();
    let state = jwt_state(tmp.path().to_path_buf()).await;
    let app = http::router(state.clone());
    let sup_token = login_for(app.clone(), "sup@example.com").await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/equity/evaluate")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {sup_token}"))
                .body(Body::from(
                    serde_json::json!({
                        "workflowUrl": WORKFLOW_URL,
                        "groupByPath": "applicant.zip",
                        "outcomePredicate": "status == 'approved'"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

// ── Integration invoke (requires Adjudicator JWT) ───────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn integration_invoke_echoes_binding() {
    let tmp = setup_tempdir();
    let state = jwt_state(tmp.path().to_path_buf()).await;
    let app = http::router(state.clone());
    let enc = workflow_path_encoded();
    let adj_token = login_for(app.clone(), "adj@example.com").await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/integration/{enc}/invoke/adjudicate"))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {adj_token}"))
                .body(Body::from(serde_json::json!({ "caseData": "test" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 64 * 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["binding"], "adjudicate");
    assert_eq!(v["output"]["status"], "echoed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn integration_invoke_rejects_anonymous() {
    let (_tmp, state) = bring_up_with_fixtures().await;
    let app = http::router(state);
    let enc = workflow_path_encoded();

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/integration/{enc}/invoke/adjudicate"))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

// ── Assurance: record + upgrade + chain ─────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn assurance_record_and_upgrade_round_trip() {
    let tmp = setup_tempdir();
    let state = jwt_state(tmp.path().to_path_buf()).await;
    let app = http::router(state.clone());
    let adj_token = login_for(app.clone(), "adj@example.com").await;
    let sup_token = login_for(app.clone(), "sup@example.com").await;

    let iid = "urn:wos:instance:test:assurance-rt";
    state
        .storage
        .create_instance(&make_instance_row(iid))
        .await
        .unwrap();
    let iid_enc = iid.replace(':', "%3A");

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/instances/{iid_enc}/identity-facts"))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {adj_token}"))
                .body(Body::from(
                    serde_json::json!({
                        "subjectRef": "sub-001",
                        "assuranceLevel": "l1",
                        "disclosurePosture": "open",
                        "fact": { "name": "Ada Lovelace" }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 64 * 1024).await.unwrap();
    let fact: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(fact["subjectRef"], "sub-001");
    assert_eq!(fact["assuranceLevel"], "l1");
    assert!(fact["upgradedFrom"].is_null());

    let fact_id = fact["id"].as_str().unwrap();
    let fact_id_enc = fact_id.replace(':', "%3A");

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/instances/{iid_enc}/identity-facts/{fact_id_enc}/upgrade"))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {sup_token}"))
                .body(Body::from(
                    serde_json::json!({
                        "newAssuranceLevel": "l3",
                        "basis": { "method": "document-verification" }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 64 * 1024).await.unwrap();
    let upgraded: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(upgraded["assuranceLevel"], "l3");
    assert_eq!(upgraded["upgradedFrom"], fact_id);

    let sub_enc = "sub-001";
    let res = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/subjects/{sub_enc}/assurance-chain"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 64 * 1024).await.unwrap();
    let chain: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(chain["facts"].as_array().unwrap().len(), 2);
    assert!(chain["chainValid"].as_bool().unwrap());
}
