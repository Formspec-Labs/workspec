//! WS-034: `GET /api/policy/{url}/resolve?asOf=ISO8601` returns the resolved
//! `policy-parameters` entry active at the requested instant. Fixtures cover
//! the post-boundary case (later entry wins after its `effectiveDate`) and the
//! gap case (no entry yet → 404).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tempfile::TempDir;
use tower::ServiceExt;
use wos_server::config::{AiChatKind, AuthKind, ServerConfig, SignerKind, StorageKind};
use wos_server::{AppState, auth, http, realtime, services::AppServices, storage};

const WORKFLOW_URL: &str = "urn:wos:workflow:policy-resolve-test:1.0.0";
const SLUG: &str = "policy-resolve-test";

fn write_fixture(dir: &std::path::Path, subdir: &str, body: serde_json::Value) {
    let target = dir.join(subdir);
    std::fs::create_dir_all(&target).unwrap();
    let file = target.join(format!("{SLUG}.json"));
    std::fs::write(&file, serde_json::to_vec_pretty(&body).unwrap()).unwrap();
}

fn seed_kernel(dir: &std::path::Path) {
    write_fixture(
        dir,
        "kernel",
        serde_json::json!({
            "url": WORKFLOW_URL,
            "title": "Policy Resolve Test",
            "version": "1.0.0",
            "status": "draft",
            "impactLevel": "operational",
        }),
    );
}

fn seed_policy_parameters(dir: &std::path::Path, versions: serde_json::Value) {
    write_fixture(
        dir,
        "policy-parameters",
        serde_json::json!({
            "targetWorkflow": WORKFLOW_URL,
            "versions": versions,
        }),
    );
}

fn default_versions() -> serde_json::Value {
    serde_json::json!([
        {
            "id": "v1",
            "label": "2025 schedule",
            "effectiveDate": "2025-01-01T00:00:00Z",
            "expiryDate": "2026-01-01T00:00:00Z",
            "parameters": {
                "incomeCeiling": 30000,
                "benefitFactor": 0.10
            }
        },
        {
            "id": "v2",
            "label": "2026 schedule",
            "effectiveDate": "2026-01-01T00:00:00Z",
            "parameters": {
                "incomeCeiling": 32000,
                "benefitFactor": 0.12
            }
        }
    ])
}

fn seed_fixtures(dir: &std::path::Path) {
    seed_kernel(dir);
    seed_policy_parameters(dir, default_versions());
}

fn stub_config(fixtures_dir: std::path::PathBuf) -> Arc<ServerConfig> {
    Arc::new(ServerConfig {
        port: 0,
        fixtures_dir,
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
        ai_chat: AiChatKind::Disabled,
        gemini_api_key: String::new(),
        cursor_throttle_ms: 50,
        timer_poll_ms: 1000,
        session_sweep_enabled: true,
        signer_kind: SignerKind::Noop,
    })
}

async fn bring_up(fixtures_dir: std::path::PathBuf) -> AppState {
    let cfg = stub_config(fixtures_dir);
    let storage = storage::build(&cfg).await.unwrap();
    let auth = auth::build(&cfg, storage.clone());
    let services = Arc::new(
        AppServices::new(cfg.clone(), storage.clone())
            .await
            .unwrap(),
    );
    let (_layer, io) = realtime::build_io_only();
    let runtime = wos_server::runtime::AppRuntime::build(
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

fn resolve_uri(as_of: &str) -> String {
    let url_enc = WORKFLOW_URL.replace(':', "%3A");
    let as_of_enc = as_of.replace(':', "%3A");
    format!("/api/policy/{url_enc}/resolve?asOf={as_of_enc}")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn policy_resolve_get_returns_active_set_at_boundary_date() {
    let tmp = TempDir::new().unwrap();
    seed_fixtures(tmp.path());
    let state = bring_up(tmp.path().to_path_buf()).await;
    let app = http::router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(resolve_uri("2026-01-15T00:00:00Z").as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 16384)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(body.get("id").and_then(|v| v.as_str()), Some("v2"));
    assert_eq!(body.get("label").and_then(|v| v.as_str()), Some("2026 schedule"));
    let params = body.get("parameters").expect("parameters present");
    assert_eq!(
        params.get("incomeCeiling").and_then(|v| v.as_i64()),
        Some(32000),
    );
    assert_eq!(
        params.get("benefitFactor").and_then(|v| v.as_f64()),
        Some(0.12),
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn policy_resolve_get_returns_404_for_gap_date() {
    let tmp = TempDir::new().unwrap();
    seed_fixtures(tmp.path());
    let state = bring_up(tmp.path().to_path_buf()).await;
    let app = http::router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(resolve_uri("2024-06-01T00:00:00Z").as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// `effectiveDate` is the inclusive lower bound: `as_of == effectiveDate`
/// hits that version. Locks the half-open `[effectiveDate, expiryDate)`
/// semantics from the `resolve_policy` doc.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn policy_resolve_get_inclusive_at_effective_date() {
    let tmp = TempDir::new().unwrap();
    seed_fixtures(tmp.path());
    let state = bring_up(tmp.path().to_path_buf()).await;
    let app = http::router(state);

    // v1.effectiveDate exactly — must hit v1.
    let response = app
        .oneshot(
            Request::builder()
                .uri(resolve_uri("2025-01-01T00:00:00Z").as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 16384).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body.get("id").and_then(|v| v.as_str()), Some("v1"));
}

/// `expiryDate` is the exclusive upper bound: `as_of == expiryDate` does NOT
/// hit that version. With v1.expiryDate == v2.effectiveDate, an `as_of` at
/// that instant must hit v2 (the next version takes over).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn policy_resolve_get_exclusive_at_expiry_date() {
    let tmp = TempDir::new().unwrap();
    seed_fixtures(tmp.path());
    let state = bring_up(tmp.path().to_path_buf()).await;
    let app = http::router(state);

    // Exactly v1.expiryDate (== v2.effectiveDate) — must hit v2, not v1.
    let response = app
        .oneshot(
            Request::builder()
                .uri(resolve_uri("2026-01-01T00:00:00Z").as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 16384).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body.get("id").and_then(|v| v.as_str()), Some("v2"));
}

/// Reviewer FINDING 2: the precedence rule is "latest `effectiveDate` wins,"
/// not "last array entry wins." Seed `[v2, v1]` (out of date order). At an
/// `as_of` after both effectiveDates, v2 must win because its effectiveDate
/// is later — even though it is the FIRST array entry.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn policy_resolve_get_picks_latest_effective_when_versions_out_of_order() {
    let tmp = TempDir::new().unwrap();
    seed_kernel(tmp.path());
    seed_policy_parameters(
        tmp.path(),
        serde_json::json!([
            {
                "id": "v2",
                "label": "2026 schedule",
                "effectiveDate": "2026-01-01T00:00:00Z",
                "parameters": {
                    "incomeCeiling": 32000,
                    "benefitFactor": 0.12
                }
            },
            {
                "id": "v1",
                "label": "2025 schedule",
                "effectiveDate": "2025-01-01T00:00:00Z",
                "expiryDate": "2026-01-01T00:00:00Z",
                "parameters": {
                    "incomeCeiling": 30000,
                    "benefitFactor": 0.10
                }
            }
        ]),
    );
    let state = bring_up(tmp.path().to_path_buf()).await;
    let app = http::router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(resolve_uri("2026-06-01T00:00:00Z").as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 16384).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body.get("id").and_then(|v| v.as_str()), Some("v2"));
}

/// Mid-gap between two non-contiguous versions returns 404. v1 expires
/// 2024-12-31; v2 starts 2025-06-01. `as_of = 2025-03-01` falls inside the
/// gap and no version covers it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn policy_resolve_get_returns_404_in_mid_gap() {
    let tmp = TempDir::new().unwrap();
    seed_kernel(tmp.path());
    seed_policy_parameters(
        tmp.path(),
        serde_json::json!([
            {
                "id": "v1",
                "label": "2024 schedule",
                "effectiveDate": "2024-01-01T00:00:00Z",
                "expiryDate": "2024-12-31T00:00:00Z",
                "parameters": {
                    "incomeCeiling": 28000
                }
            },
            {
                "id": "v2",
                "label": "Mid-2025 schedule",
                "effectiveDate": "2025-06-01T00:00:00Z",
                "parameters": {
                    "incomeCeiling": 33000
                }
            }
        ]),
    );
    let state = bring_up(tmp.path().to_path_buf()).await;
    let app = http::router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(resolve_uri("2025-03-01T00:00:00Z").as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
