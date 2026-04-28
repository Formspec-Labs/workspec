//! WS-014 slice B: conformance, calendar, notifications, integration, deontic,
//! assurance, applicant, agents — see [`http_coverage_backfill.rs`](http_coverage_backfill.rs) for slice A.

#[path = "http_coverage_shared/harness.rs"]
mod harness;
#[path = "http_coverage_shared/slice_b.rs"]
mod slice_b;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use wos_server::http;

use harness::{
    bring_up, bring_up_with_cfg, make_instance_row, seed_instance_with_one_provenance, stub_config,
};
use slice_b::{
    int_consume_001_fixture_path, slice_b_tempdir, slice_b_workflow_path_encoded,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conformance_fixture_int_consume_happy_path() {
    let state = bring_up().await;
    let app = http::router(state);
    let path = int_consume_001_fixture_path();
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let fixture: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let base_dir = path
        .parent()
        .and_then(|p| p.to_str())
        .expect("fixture path has parent")
        .to_string();
    let body = serde_json::json!({ "fixture": fixture, "baseDir": base_dir });
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/conformance/fixture")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        v.get("passed"),
        Some(&serde_json::Value::Bool(true)),
        "INT-CONSUME-001 must pass; body={v}"
    );
    let tc = v
        .get("transitionCount")
        .and_then(|x| x.as_u64())
        .expect("`transitionCount` must be a JSON number (u64)");
    assert!(
        tc >= 1,
        "INT-CONSUME-001 must report at least one transition; got transitionCount={tc}, body={v}"
    );
    let failures = v
        .get("failures")
        .and_then(|f| f.as_array())
        .expect("`failures` must be a JSON array");
    assert!(
        failures.is_empty(),
        "happy-path fixture must have zero conformance failures; got {failures:?} in {v}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conformance_fixture_null_returns_bad_request() {
    let state = bring_up().await;
    let app = http::router(state);
    let body = serde_json::json!({ "fixture": null });
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/conformance/fixture")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn calendar_compute_deadline_unknown_workflow_returns_404() {
    let state = bring_up().await;
    let app = http::router(state);
    let url = "urn%3Awos%3Aworkflow%3Anot-registered%3A1.0.0";
    let body = serde_json::json!({ "duration": "PT1H" });
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/calendar/{url}/compute-deadline"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn calendar_compute_deadline_happy_path_slice_b() {
    let tmp = slice_b_tempdir();
    let state = bring_up_with_cfg(stub_config(tmp.path().to_path_buf())).await;
    let app = http::router(state);
    let enc = slice_b_workflow_path_encoded();

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/calendar/{enc}/compute-deadline"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "duration": "PT24H" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 64 * 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    for key in ["deadline", "from", "duration", "calendarUrl", "calendarTimezone"] {
        assert!(
            v.get(key).is_some(),
            "compute-deadline response missing `{key}`: {v}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn calendar_compute_deadline_invalid_duration_returns_400() {
    let tmp = slice_b_tempdir();
    let state = bring_up_with_cfg(stub_config(tmp.path().to_path_buf())).await;
    let app = http::router(state);
    let enc = slice_b_workflow_path_encoded();
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/calendar/{enc}/compute-deadline"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "duration": "not-an-iso-duration" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn notifications_render_happy_path_slice_b() {
    let tmp = slice_b_tempdir();
    let state = bring_up_with_cfg(stub_config(tmp.path().to_path_buf())).await;
    let app = http::router(state);
    let enc = slice_b_workflow_path_encoded();

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/notifications/{enc}/render"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "templateId": "notice",
                        "context": { "user": "Ada" }
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
    assert_eq!(v.get("templateId"), Some(&serde_json::json!("notice")));
    assert!(
        v.get("body").and_then(|b| b.as_str()).is_some_and(|s| s.contains("Ada")),
        "expected interpolated body, got {v}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn notifications_render_unknown_template_returns_404() {
    let tmp = slice_b_tempdir();
    let state = bring_up_with_cfg(stub_config(tmp.path().to_path_buf())).await;
    let app = http::router(state);
    let enc = slice_b_workflow_path_encoded();
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/notifications/{enc}/render"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "templateId": "does-not-exist", "context": {} })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn integration_profile_happy_path_slice_b() {
    let tmp = slice_b_tempdir();
    let state = bring_up_with_cfg(stub_config(tmp.path().to_path_buf())).await;
    let app = http::router(state);
    let enc = slice_b_workflow_path_encoded();

    let res = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/integration/{enc}/profile"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 64 * 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(
        v.get("bindings").is_some(),
        "integration profile JSON must include bindings array/object: {v}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn integration_profile_unknown_workflow_returns_404() {
    let state = bring_up().await;
    let app = http::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/integration/urn%3Awos%3Aworkflow%3Aghost%3A1/profile")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deontic_violations_returns_json_array_for_instance() {
    let state = bring_up().await;
    let iid = "urn:wos:instance:test:deontic-http";
    seed_instance_with_one_provenance(&state.storage, iid).await;
    let app = http::router(state);
    let enc = iid.replace(':', "%3A");
    let res = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/instances/{enc}/deontic-violations"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 64 * 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(
        v.is_array(),
        "deontic-violations must return a JSON array, got: {v}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn assurance_identity_facts_list_returns_array() {
    let state = bring_up().await;
    let iid = "urn:wos:instance:test:assurance-facts";
    state
        .storage
        .create_instance(&make_instance_row(iid))
        .await
        .unwrap();
    let app = http::router(state);
    let enc = iid.replace(':', "%3A");
    let res = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/instances/{enc}/identity-facts"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 64 * 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(v.is_array(), "identity-facts must return a JSON array: {v}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn applicant_determination_returns_404_when_missing() {
    let state = bring_up().await;
    let app = http::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/applicant/urn%3Awos%3Ainstance%3Atest%3Ano-determination/determination")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_list_requires_workflow_url_query() {
    let state = bring_up().await;
    let app = http::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/agents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}
