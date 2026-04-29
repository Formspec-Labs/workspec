//! WS-036: Adverse-decision notice rendering (Gov §3.2).
//!
//! Tests `POST /api/governance/:url/notices/:template/render` which joins a
//! notification template with the `dueProcess` sidecar to stamp grace-period,
//! appeal-window, and right-to-contest fields on the rendered output.

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tempfile::TempDir;
use tower::ServiceExt;
use wos_server::http;

#[path = "http_coverage_shared/harness.rs"]
mod harness;

use harness::{bring_up_with_cfg, stub_config};

const SLUG: &str = "ws036adverse";
const WORKFLOW_URL: &str = "urn:wos:workflow:ws036adverse:1.0.0";

fn workflow_path_encoded() -> String {
    WORKFLOW_URL.replace(':', "%3A")
}

fn setup_tempdir() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    let kernel = serde_json::json!({
        "$wosWorkflow": "1.0",
        "url": WORKFLOW_URL,
        "version": "1.0.0",
        "title": "WS-036 adverse notice test",
        "status": "active",
        "impactLevel": "rightsImpacting",
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

    let tmpl = serde_json::json!({
        "templates": [
            {
                "id": "adverse-decision",
                "subject": "Adverse Decision — ${caseId}",
                "body": "Your application has been denied. Grace period: ${gracePeriod}. Appeal window: ${appealWindow}. ${rightToContest}.",
                "channels": ["email", "mail"]
            },
            {
                "id": "no-due-process",
                "subject": "Plain notice",
                "body": "Status update for ${caseId}.",
                "channels": ["email"]
            }
        ]
    });
    std::fs::create_dir_all(root.join("notification-template")).unwrap();
    std::fs::write(
        root.join("notification-template").join(format!("{SLUG}.json")),
        serde_json::to_vec_pretty(&tmpl).unwrap(),
    )
    .unwrap();

    let due = serde_json::json!({
        "gracePeriod": "P10D",
        "appealWindow": "P30D",
        "rightToContest": "You have the right to contest this decision before the review board"
    });
    std::fs::create_dir_all(root.join("due-process")).unwrap();
    std::fs::write(
        root.join("due-process").join(format!("{SLUG}.json")),
        serde_json::to_vec_pretty(&due).unwrap(),
    )
    .unwrap();

    dir
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn adverse_notice_render_joins_template_with_due_process() {
    let tmp = setup_tempdir();
    let state = bring_up_with_cfg(stub_config(tmp.path().to_path_buf())).await;
    let app = http::router(state);
    let enc = workflow_path_encoded();

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/governance/{enc}/notices/adverse-decision/render"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "context": { "caseId": "CASE-001" } }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 64 * 1024)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(v["templateId"], "adverse-decision");
    assert_eq!(v["subject"], "Adverse Decision — CASE-001");
    assert_eq!(v["gracePeriod"], "P10D");
    assert_eq!(v["appealWindow"], "P30D");
    assert_eq!(
        v["rightToContest"],
        "You have the right to contest this decision before the review board"
    );

    let body = v["body"].as_str().unwrap();
    assert!(
        body.contains("P10D"),
        "interpolated body must contain grace period; got: {body}"
    );
    assert!(
        body.contains("P30D"),
        "interpolated body must contain appeal window; got: {body}"
    );
    assert!(
        body.contains("right to contest"),
        "interpolated body must contain right-to-contest text; got: {body}"
    );

    let subject = v["subject"].as_str().unwrap();
    assert!(
        subject.contains("CASE-001"),
        "subject must contain caller context caseId; got: {subject}"
    );

    let channels = v["channels"].as_array().unwrap();
    assert_eq!(channels.len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn adverse_notice_different_template_same_due_process() {
    let tmp = setup_tempdir();
    let state = bring_up_with_cfg(stub_config(tmp.path().to_path_buf())).await;
    let app = http::router(state);
    let enc = workflow_path_encoded();

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/governance/{enc}/notices/no-due-process/render"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "context": { "caseId": "CASE-002" } }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 64 * 1024)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(v["templateId"], "no-due-process");
    assert_eq!(
        v["gracePeriod"], "P10D",
        "due-process fields are workflow-level, present on all templates; got: {v}"
    );
    assert_eq!(v["body"], "Status update for CASE-002.");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn adverse_notice_unknown_template_returns_404() {
    let tmp = setup_tempdir();
    let state = bring_up_with_cfg(stub_config(tmp.path().to_path_buf())).await;
    let app = http::router(state);
    let enc = workflow_path_encoded();

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/governance/{enc}/notices/nonexistent/render"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "context": {} }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn adverse_notice_unknown_workflow_returns_404() {
    let state = bring_up_with_cfg(stub_config(PathBuf::from("."))).await;
    let app = http::router(state);

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/governance/urn%3Awos%3Aworkflow%3Aghost%3A1/notices/any/render")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "context": {} }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
