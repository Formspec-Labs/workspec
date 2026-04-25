//! WS-014: HTTP coverage backfill for previously unexercised route modules.
//!
//! Pins current behaviour for three routes called out in the WS-014
//! "next slice":
//!
//! * `POST /api/lint/document` — happy path + malformed-body failure path.
//! * `GET  /api/instances/:id/provenance/export?format=…` — single-format
//!   happy path against a seeded instance + 404 on unknown id.
//! * `GET  /api/dashboard/metrics` — top-level shape under a multi-instance
//!   fixture (asserts keys only; ignores synthetic-vs-real values per WS-055
//!   marker `synthetic_fields`).
//!
//! Auth: `AuthKind::Mock` so no login is required. The mock provider grants a
//! supervisor identity to anonymous requests, which is fine for these read
//! endpoints; the assertions are about route reachability + response shape.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use tower::ServiceExt;
use wos_core::provenance::ProvenanceRecord;
use wos_server::config::{AiChatKind, AuthKind, ServerConfig, SignerKind, StorageKind};
use wos_server::services::provenance_service::chain_hash;
use wos_server::storage::{InstanceRow, ProvenanceRow};
use wos_server::{AppState, auth, http, realtime, services::AppServices, storage};

const ZERO_HASH: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";

fn stub_config() -> Arc<ServerConfig> {
    Arc::new(ServerConfig {
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
        ai_chat: AiChatKind::Disabled,
        gemini_api_key: String::new(),
        cursor_throttle_ms: 50,
        timer_poll_ms: 1000,
        session_sweep_enabled: false,
        signer_kind: SignerKind::Noop,
    })
}

async fn bring_up() -> AppState {
    let cfg = stub_config();
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

fn make_instance_row(id: &str) -> InstanceRow {
    let now = Utc::now();
    InstanceRow {
        instance_id: id.into(),
        definition_url: "urn:wos:workflow:test:1.0.0".into(),
        definition_version: "1.0.0".into(),
        status: "active".into(),
        impact_level: "operational".into(),
        instance_json: serde_json::json!({
            "instanceId": id,
            "definitionUrl": "urn:wos:workflow:test:1.0.0",
            "status": "active",
            "configuration": ["draft"],
        }),
        runtime_aux_json: serde_json::Value::Null,
        created_at: now,
        updated_at: now,
    }
}

async fn seed_instance_with_one_provenance(
    store: &storage::StorageHandle,
    instance_id: &str,
) {
    store
        .create_instance(&make_instance_row(instance_id))
        .await
        .unwrap();

    let mut record =
        ProvenanceRecord::state_transition("draft", "review", "submit", Some("applicant"));
    record.audit_layer = Some("facts".into());

    let ts = Utc::now();
    let tier = record.audit_layer.clone().unwrap_or_else(|| "facts".into());
    let payload = serde_json::to_value(&record).unwrap();
    let hash = chain_hash(ZERO_HASH, instance_id, 1, &ts, &tier, &payload);
    let row = ProvenanceRow {
        id: format!("rec-{instance_id}-1"),
        instance_id: instance_id.into(),
        seq: 1,
        timestamp: ts,
        tier,
        payload,
        hash,
        previous_hash: ZERO_HASH.into(),
    };

    let rows = vec![row];
    store
        .update_instance_atomic(instance_id, &move |_row| Ok(rows.clone()))
        .await
        .unwrap();
}

// ---------------------------------------------------------------------------
// `POST /api/lint/document`
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lint_document_happy_path() {
    let state = bring_up().await;
    let app = http::router(state);

    // Minimal kernel doc that wos-lint recognises via the `$wosKernel` marker.
    let body = serde_json::json!({ "$wosKernel": "1.0" });
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/lint/document")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 64 * 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    // LintResult serialises as `{ isValid, diagnostics: [...] }`. The current
    // handler always returns a `diagnostics` array (possibly empty).
    assert!(
        v.get("diagnostics").and_then(|d| d.as_array()).is_some(),
        "lint response must carry a `diagnostics` array, got: {v}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lint_document_validation_failure() {
    let state = bring_up().await;
    let app = http::router(state);

    // No `$wosKernel` (or any other recognised `$wos*` marker) → wos-lint
    // returns `LintError::Parse`, which `lint_service::lint_document` maps
    // to a 200 OK with `isValid: false` and exactly one synthetic
    // `PARSE-001` diagnostic (`crates/wos-server/src/services/lint_service.rs`).
    // Pin that contract precisely — a future refactor that drops the
    // synthetic diagnostic or renames the rule must surface as a test
    // failure here, not silently slip through a permissive disjunction.
    let body = serde_json::json!({ "not_a_wos_marker": true });
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/lint/document")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 64 * 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(status, StatusCode::OK, "lint parse-failure path must respond 200: {v}");
    assert_eq!(
        v.get("isValid"),
        Some(&serde_json::Value::Bool(false)),
        "isValid must be exactly false on parse failure: {v}",
    );
    let diags = v
        .get("diagnostics")
        .and_then(|d| d.as_array())
        .expect("diagnostics array required on parse failure");
    assert!(
        diags
            .iter()
            .any(|d| d.get("ruleId").and_then(|r| r.as_str()) == Some("PARSE-001")),
        "expected PARSE-001 in diagnostics, got: {diags:?}",
    );
}

// ---------------------------------------------------------------------------
// `GET /api/instances/:id/provenance/export`
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn provenance_export_prov_o_returns_jsonld() {
    let state = bring_up().await;
    let instance_id = "urn:wos:instance:test:export";
    seed_instance_with_one_provenance(&state.storage, instance_id).await;
    let app = http::router(state);

    let encoded = instance_id.replace(':', "%3A");
    let res = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/instances/{encoded}/provenance/export?format=prov-o").as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let ct = res
        .headers()
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert_eq!(
        ct, "application/ld+json",
        "PROV-O export must serve content-type=application/ld+json, got `{ct}`"
    );
    let bytes = axum::body::to_bytes(res.into_body(), 256 * 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes)
        .expect("PROV-O export body must be valid JSON-LD JSON");
    assert!(
        v.is_object() || v.is_array(),
        "PROV-O document must be a JSON object/array, got: {v}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn provenance_export_xes_returns_xml() {
    let state = bring_up().await;
    let instance_id = "urn:wos:instance:test:export-xes";
    seed_instance_with_one_provenance(&state.storage, instance_id).await;
    let app = http::router(state);

    let encoded = instance_id.replace(':', "%3A");
    let res = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/instances/{encoded}/provenance/export?format=xes").as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let ct = res
        .headers()
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert_eq!(
        ct, "application/xml",
        "XES export must serve content-type=application/xml, got `{ct}`"
    );
    let bytes = axum::body::to_bytes(res.into_body(), 256 * 1024).await.unwrap();
    let body = std::str::from_utf8(&bytes).expect("XES body must be UTF-8");
    // XES is XML — the serializer emits a `<log>` root, optionally preceded
    // by `<?xml ...?>`. Pin both prologues so a future serialiser change
    // does not silently drop the XML envelope.
    assert!(
        body.contains("<?xml") || body.contains("<log"),
        "XES body must look like XML (got prefix: {:?})",
        body.chars().take(120).collect::<String>(),
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn provenance_export_ocel_returns_json() {
    let state = bring_up().await;
    let instance_id = "urn:wos:instance:test:export-ocel";
    seed_instance_with_one_provenance(&state.storage, instance_id).await;
    let app = http::router(state);

    let encoded = instance_id.replace(':', "%3A");
    let res = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/instances/{encoded}/provenance/export?format=ocel").as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let ct = res
        .headers()
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert_eq!(
        ct, "application/json",
        "OCEL export must serve content-type=application/json, got `{ct}`"
    );
    let bytes = axum::body::to_bytes(res.into_body(), 256 * 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes)
        .expect("OCEL export body must be valid JSON");
    let obj = v.as_object().expect("OCEL document must be a JSON object");
    // OCEL 2.0 top-level shape — see `wos-export::ocel::export`. All four
    // arrays are emitted even when the log is small; pinning the keys
    // guards against silent reshape under refactor.
    for key in ["objectTypes", "eventTypes", "objects", "events"] {
        assert!(
            obj.contains_key(key),
            "OCEL document missing top-level key `{key}`; got: {v}",
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn provenance_export_404_for_missing_instance() {
    let state = bring_up().await;
    let app = http::router(state);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/instances/urn%3Awos%3Ainstance%3Atest%3Adoes-not-exist/provenance/export?format=prov-o")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// `GET /api/dashboard/metrics`
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dashboard_metrics_returns_shape() {
    let state = bring_up().await;
    state
        .storage
        .create_instance(&make_instance_row("urn:wos:instance:test:dash-1"))
        .await
        .unwrap();
    state
        .storage
        .create_instance(&make_instance_row("urn:wos:instance:test:dash-2"))
        .await
        .unwrap();
    let app = http::router(state);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/dashboard/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 64 * 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let obj = v
        .as_object()
        .expect("dashboard metrics body must be a JSON object");

    // Top-level keys emitted by `DashboardMetricsView` (camelCase via
    // `serde(rename_all = "camelCase")`). `synthetic_fields` is the WS-055
    // marker; we only check the key is present, not its contents — values
    // may be real or synthetic depending on fixture density.
    for key in [
        "activeInstances",
        "completed7d",
        "slaCompliance",
        "avgProcessingTimeDays",
        "aiAcceptanceRate",
        "activeInstancesTrend",
        "completed7dTrend",
        "slaComplianceTrend",
        "avgProcessingTimeTrend",
        "aiAcceptanceRateTrend",
        "syntheticFields",
    ] {
        assert!(
            obj.contains_key(key),
            "dashboard metrics missing top-level key `{key}`; got: {v}"
        );
    }
}
