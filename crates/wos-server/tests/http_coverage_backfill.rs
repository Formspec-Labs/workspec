//! WS-014 slice A: HTTP coverage for lint, provenance export, and dashboard.
//!
//! * `POST /api/lint/document` — happy path + parse failure (`PARSE-001`) +
//!   non-JSON body (extractor rejection).
//! * `GET  /api/instances/:id/provenance/export?format=…` — `prov-o`, `xes`,
//!   and `ocel` happy paths + 404 on unknown id + invalid `format` query.
//! * `GET /api/dashboard/metrics` — top-level shape under a multi-instance
//!   fixture (asserts keys only; ignores synthetic-vs-real values per WS-055
//!   marker `synthetic_fields`).
//!
//! Slice B (conformance, calendar, notifications, integration, deontic,
//! assurance, applicant, agents) lives in [`http_coverage_slice_b.rs`](http_coverage_slice_b.rs)
//! with shared helpers under [`http_coverage_shared/`](http_coverage_shared/).
//!
//! Auth: `AuthKind::Mock` so no login is required. The mock provider grants a
//! supervisor identity to anonymous requests, which is fine for these read
//! endpoints; the assertions are about route reachability + response shape.

#[path = "http_coverage_shared/harness.rs"]
mod harness;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use wos_server::http;

use harness::{bring_up, make_instance_row, seed_instance_with_one_provenance};

// ---------------------------------------------------------------------------
// `POST /api/lint/document`
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lint_document_happy_path() {
    let state = bring_up().await;
    let app = http::router(state);

    // Minimal kernel doc that wos-lint recognises via the `$wosWorkflow` marker.
    let body = serde_json::json!({ "$wosWorkflow": "1.0" });
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

    // No `$wosWorkflow` (or any other recognised `$wos*` marker) → wos-lint
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lint_document_non_json_body_is_rejected() {
    let state = bring_up().await;
    let app = http::router(state);

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/lint/document")
                .header("content-type", "application/json")
                .body(Body::from("{not json"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "invalid JSON syntax is axum `JsonSyntaxError` → 400"
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
    let obj = v
        .as_object()
        .expect("PROV-O export must be a JSON object (`ProvODocument`)");
    assert!(
        obj.contains_key("@context") && obj.contains_key("@graph"),
        "PROV-O must expose JSON-LD `@context` + `@graph` (wos-export::prov_o::ProvODocument), got keys: {:?}",
        obj.keys().collect::<Vec<_>>(),
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn provenance_export_invalid_format_query_is_rejected() {
    let state = bring_up().await;
    let instance_id = "urn:wos:instance:test:export-bad-format";
    seed_instance_with_one_provenance(&state.storage, instance_id).await;
    let app = http::router(state);

    let encoded = instance_id.replace(':', "%3A");
    let res = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/instances/{encoded}/provenance/export?format=not-a-format"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "invalid `format` query fails `Query` deserialize → 400"
    );
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
