use axum::body::Body;
use axum::http::{Request, StatusCode};
use tempfile::TempDir;
use tower::ServiceExt;
use wos_server::http;

#[path = "common/mod.rs"]
mod common;

fn setup_tempdir() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();

    let kernel = serde_json::json!({
        "$wosWorkflow": "1.0",
        "url": "urn:wos:workflow:apisurface:1.0.0",
        "version": "1.0.0",
        "title": "API surface fixture",
        "status": "active",
        "impactLevel": "operational",
        "actors": [{ "id": "sys", "type": "system" }],
        "lifecycle": {
            "initialState": "done",
            "states": { "done": { "type": "final" } }
        },
        "contracts": {}
    });
    std::fs::create_dir_all(root.join("kernel")).expect("kernel dir");
    std::fs::write(
        root.join("kernel").join("apisurface.json"),
        serde_json::to_vec_pretty(&kernel).expect("kernel json"),
    )
    .expect("kernel write");

    let integration_profile = serde_json::json!({
        "bindings": [{ "id": "adjudicate", "type": "http", "url": "https://example.com" }]
    });
    std::fs::create_dir_all(root.join("integration-profile")).expect("integration dir");
    std::fs::write(
        root.join("integration-profile").join("apisurface.json"),
        serde_json::to_vec_pretty(&integration_profile).expect("integration json"),
    )
    .expect("integration write");

    dir
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_detail_canary_shadow_and_drift_endpoints() {
    let tmp = setup_tempdir();
    let state = common::bring_up_with_cfg(common::stub_config(tmp.path().to_path_buf())).await;
    let app = http::router(state);

    let register_body = serde_json::json!({
        "workflowUrl": "urn:wos:workflow:apisurface:1.0.0",
        "name": "Assist Agent",
        "kind": "generative",
        "version": "1.2.3",
        "autonomy": "recommend",
        "confidenceFloor": 0.8,
        "config": { "model": "test" }
    });
    let register = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents")
                .header("authorization", "Bearer mock-access")
                .header("content-type", "application/json")
                .body(Body::from(register_body.to_string()))
                .expect("register request"),
        )
        .await
        .expect("register response");
    assert_eq!(register.status(), StatusCode::OK);
    let register_bytes = axum::body::to_bytes(register.into_body(), 64 * 1024)
        .await
        .expect("register bytes");
    let register_json: serde_json::Value =
        serde_json::from_slice(&register_bytes).expect("register json");
    let id = register_json["id"].as_str().expect("agent id");
    let id_enc = common::path_encode(id);

    let get_one = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/agents/{id_enc}"))
                .body(Body::empty())
                .expect("get request"),
        )
        .await
        .expect("get response");
    assert_eq!(get_one.status(), StatusCode::OK);

    let no_auth_canary = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/agents/{id_enc}/canary"))
                .body(Body::empty())
                .expect("canary request"),
        )
        .await
        .expect("canary response");
    assert_eq!(no_auth_canary.status(), StatusCode::UNAUTHORIZED);

    let canary = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/agents/{id_enc}/canary"))
                .header("authorization", "Bearer mock-access")
                .body(Body::empty())
                .expect("canary request"),
        )
        .await
        .expect("canary response");
    assert_eq!(canary.status(), StatusCode::OK);
    let canary_bytes = axum::body::to_bytes(canary.into_body(), 64 * 1024)
        .await
        .expect("canary bytes");
    let canary_json: serde_json::Value = serde_json::from_slice(&canary_bytes).expect("canary");
    assert_eq!(canary_json["deploymentState"], "canary");

    let shadow = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/agents/{id_enc}/shadow"))
                .header("authorization", "Bearer mock-access")
                .body(Body::empty())
                .expect("shadow request"),
        )
        .await
        .expect("shadow response");
    assert_eq!(shadow.status(), StatusCode::OK);
    let shadow_bytes = axum::body::to_bytes(shadow.into_body(), 64 * 1024)
        .await
        .expect("shadow bytes");
    let shadow_json: serde_json::Value = serde_json::from_slice(&shadow_bytes).expect("shadow");
    assert_eq!(shadow_json["deploymentState"], "shadow");

    let drift = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/agents/{id_enc}/drift"))
                .body(Body::empty())
                .expect("drift request"),
        )
        .await
        .expect("drift response");
    assert_eq!(drift.status(), StatusCode::OK);
    let drift_bytes = axum::body::to_bytes(drift.into_body(), 64 * 1024)
        .await
        .expect("drift bytes");
    let drift_json: serde_json::Value = serde_json::from_slice(&drift_bytes).expect("drift");
    assert_eq!(drift_json["agentId"], id);
    assert!(drift_json.get("windowDays").is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lint_validate_dashboard_tasks_and_auth_probe_routes() {
    let tmp = setup_tempdir();
    let state = common::bring_up_with_cfg(common::stub_config(tmp.path().to_path_buf())).await;
    let app = http::router(state);

    let has_role_anon = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/has-role/Supervisor")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(has_role_anon.status(), StatusCode::OK);
    let anon_bytes = axum::body::to_bytes(has_role_anon.into_body(), 4096)
        .await
        .expect("bytes");
    let anon_json: serde_json::Value = serde_json::from_slice(&anon_bytes).expect("json");
    assert_eq!(anon_json["hasRole"], false);

    let has_role_auth = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/has-role/Supervisor")
                .header("authorization", "Bearer mock-access")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(has_role_auth.status(), StatusCode::OK);
    let auth_bytes = axum::body::to_bytes(has_role_auth.into_body(), 4096)
        .await
        .expect("bytes");
    let auth_json: serde_json::Value = serde_json::from_slice(&auth_bytes).expect("json");
    assert_eq!(auth_json["hasRole"], true);

    let lint_schema = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/lint/schema")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({"type":"object"}).to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(lint_schema.status(), StatusCode::OK);
    let lint_schema_bytes = axum::body::to_bytes(lint_schema.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    let lint_schema_json: serde_json::Value =
        serde_json::from_slice(&lint_schema_bytes).expect("json");
    assert!(lint_schema_json.get("isValid").is_some());
    assert!(lint_schema_json.get("diagnostics").is_some());

    let lint_rules = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/lint/rules")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(lint_rules.status(), StatusCode::OK);
    let lint_rules_bytes = axum::body::to_bytes(lint_rules.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    let lint_rules_json: serde_json::Value =
        serde_json::from_slice(&lint_rules_bytes).expect("json");
    assert!(
        !lint_rules_json.as_array().expect("array").is_empty(),
        "lint rule catalog should not be empty"
    );

    let validate_kernel = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/kernel/validate")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "$wosWorkflow": "1.0",
                        "url": "urn:wos:workflow:apisurface:1.0.0",
                        "version": "1.0.0"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(validate_kernel.status(), StatusCode::OK);
    let validate_bytes = axum::body::to_bytes(validate_kernel.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    let validate_json: serde_json::Value = serde_json::from_slice(&validate_bytes).expect("json");
    assert!(validate_json.get("isValid").is_some());

    for path in [
        "/api/dashboard/stage-metrics",
        "/api/dashboard/alerts",
        "/api/dashboard/drift-data",
        "/api/dashboard/pipeline-data",
    ] {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(res.into_body(), 64 * 1024)
            .await
            .expect("bytes");
        let body: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert!(body.is_array(), "dashboard route should return list");
    }

    let tasks_list = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/tasks")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(tasks_list.status(), StatusCode::OK);
    let tasks_list_bytes = axum::body::to_bytes(tasks_list.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    let tasks_list_json: serde_json::Value =
        serde_json::from_slice(&tasks_list_bytes).expect("json");
    assert!(tasks_list_json.get("items").is_some());
    assert!(tasks_list_json.get("total").is_some());

    let missing_task = app
        .oneshot(
            Request::builder()
                .uri("/api/tasks/task-does-not-exist")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(missing_task.status(), StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn inbound_events_success_path_and_dedup_ack() {
    let tmp = setup_tempdir();
    let state = common::jwt_state(tmp.path().to_path_buf()).await;
    let app = http::router(state);
    let adjudicator = common::login_access_token(app.clone(), "adj@example.com").await;

    let inbound = serde_json::json!({
        "id": "evt-apisurface-1",
        "source": "urn:test:source",
        "type": "adjudicate.completed",
        "specversion": "1.0",
        "time": "2026-01-01T00:00:00Z",
        "data": { "case": "A-1" }
    });

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/events/inbound")
                .header("authorization", format!("Bearer {adjudicator}"))
                .header("content-type", "application/json")
                .body(Body::from(inbound.to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(first.status(), StatusCode::OK);
    let first_bytes = axum::body::to_bytes(first.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    let first_json: serde_json::Value = serde_json::from_slice(&first_bytes).expect("json");
    assert_eq!(first_json["enqueued"], false);
    assert_eq!(first_json["deduplicated"], false);

    let second = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/events/inbound")
                .header("authorization", format!("Bearer {adjudicator}"))
                .header("content-type", "application/json")
                .body(Body::from(inbound.to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(second.status(), StatusCode::OK);
    let second_bytes = axum::body::to_bytes(second.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    let second_json: serde_json::Value = serde_json::from_slice(&second_bytes).expect("json");
    assert_eq!(second_json["deduplicated"], true);
    assert_eq!(second_json["enqueued"], false);
}
