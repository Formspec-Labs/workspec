//! WS-011: Axum-level coverage for `POST /api/tasks/{id}/draft`,
//! `/response`, and `/dismiss`.
//!
//! Exercises the three task-binding routes through `http::router` with
//! real JWT auth (Adjudicator for draft/response, Supervisor for dismiss
//! after WS-083), a hydrated `BundleService`, and an injected `formspec`
//! `ContractBindingAdapter` so `submit_task_response` can reach a
//! `Completed` outcome.
//!
//! Test set:
//!   1. `task_draft_replays_on_repeat_idempotency_token` — re-POST with
//!      the same `idempotencyToken` returns the cached artifact id via
//!      `ReplayOperation::PersistDraft`.
//!   2. `task_submit_response_returns_completed_view` — `status:
//!      "completed"` round-trips to `TaskSubmissionView::Completed`
//!      with `caseMutated: true`.
//!   3. `task_dismiss_then_respond_locks_runtime_behavior` — `/dismiss`
//!      returns `{ ok: true }` and a follow-up `/response` reflects
//!      *current* `wos-runtime` semantics: `dismiss_task` records a
//!      `TaskDismissed` provenance row but does **not** remove the task
//!      from `active_tasks`, so a subsequent `submit_task_response`
//!      still completes. The TODO entry's "task-missing" wording is
//!      aspirational — the runtime behaviour change must land first.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use rand::rngs::OsRng;
use tower::ServiceExt;
use wos_core::instance::{ActiveTask, ValidationOutcome};
use wos_runtime::runtime::CreateInstanceRequest;
use wos_runtime::{
    BindingError, BindingRegistry, CaseMutationBundle, ContractBindingAdapter, PreparedTask,
    SubmissionValidation,
};
use wos_server::config::{AuthKind, ServerConfig, SignerKind, StorageKind};
use wos_server::runtime::{AppRuntime, AppRuntimeConfig};
use wos_server::storage::{KernelRow, SqliteStorage, Storage, UserRow};
use wos_server::{AppState, auth, http, realtime, services::AppServices};

// ── Test binding adapter (mirrors `runtime.rs::TestAdapter`) ──────────

#[derive(Debug, Default)]
struct FormspecTestAdapter;

impl ContractBindingAdapter for FormspecTestAdapter {
    fn binding(&self) -> &'static str {
        "formspec"
    }

    fn prepare_task(
        &self,
        _task: &ActiveTask,
        _case_state: &serde_json::Value,
    ) -> Result<PreparedTask, BindingError> {
        Ok(PreparedTask::default())
    }

    fn validate_submission(
        &self,
        _task: &ActiveTask,
        _response: &serde_json::Value,
    ) -> Result<SubmissionValidation, BindingError> {
        // Always pass — WS-011 exercises the HTTP-layer submission
        // contract, not binding-specific validation. Anything stricter
        // would entangle the test with the runtime's pin/envelope shape.
        Ok(SubmissionValidation {
            validation_outcome: ValidationOutcome {
                envelope_valid: true,
                pin_match: true,
                definition_valid: true,
                errors: Vec::new(),
                validation_results: None,
            },
        })
    }

    fn compute_case_mutation(
        &self,
        _task: &ActiveTask,
        response: &serde_json::Value,
    ) -> Result<Option<CaseMutationBundle>, BindingError> {
        let mut field_updates = serde_json::Map::new();
        field_updates.insert(
            "decision".to_string(),
            response
                .get("data")
                .and_then(|d| d.get("approved"))
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        );
        Ok(Some(CaseMutationBundle { field_updates }))
    }
}

// ── Fixture helpers ───────────────────────────────────────────────────

const KERNEL_URL: &str = "urn:wos:workflow:test:tasks-lifecycle";
const KERNEL_VERSION: &str = "1.0.0";

fn kernel_document() -> serde_json::Value {
    serde_json::json!({
        "$wosWorkflow": "1.0",
        "url": KERNEL_URL,
        "version": KERNEL_VERSION,
        "title": "Tasks Lifecycle Test Kernel",
        "status": "active",
        "lifecycle": {
            "initialState": "open",
            "states": {
                "open": {
                    "type": "atomic",
                    "transitions": [{
                        "event": "start",
                        "target": "open",
                        "actions": [{
                            "action": "createTask",
                            "taskRef": "review",
                            "assignTo": "reviewer",
                            "contractRef": "reviewForm",
                            "responseMappingRef": "urn:mapping:response",
                            "completionEvent": "review.completed",
                            "failureEvent": "review.failed"
                        }]
                    }]
                }
            }
        },
        "actors": [
            { "id": "reviewer", "type": "human" }
        ],
        "contracts": {
            "reviewForm": {
                "binding": "formspec",
                "ref": "urn:formspec:review"
            }
        }
    })
}

async fn jwt_app_state_with_binding() -> AppState {
    let store = Arc::new(
        SqliteStorage::connect("sqlite::memory:?cache=shared")
            .await
            .unwrap(),
    );
    store.migrate().await.unwrap();

    // Seed kernel before AppServices::new so initial hydrate picks it up.
    store
        .upsert_kernel(&KernelRow {
            url: KERNEL_URL.into(),
            title: "Tasks Lifecycle Test Kernel".into(),
            version: KERNEL_VERSION.into(),
            status: "active".into(),
            impact_level: "operational".into(),
            document: kernel_document(),
            updated_at: Utc::now(),
        })
        .await
        .unwrap();

    // Two users: Adjudicator (draft/response) + Supervisor (dismiss).
    let salt_a = SaltString::generate(&mut OsRng);
    let hash_a = Argon2::default()
        .hash_password(b"wos-dev", &salt_a)
        .unwrap()
        .to_string();
    store
        .upsert_user(&UserRow {
            id: "u-adj".into(),
            email: "adjudicator@example.com".into(),
            name: "Adjudicator User".into(),
            role: "Adjudicator".into(),
            password_hash: hash_a,
            avatar: None,
            auth_epoch: 0,
            created_at: Utc::now(),
        })
        .await
        .unwrap();

    let salt_s = SaltString::generate(&mut OsRng);
    let hash_s = Argon2::default()
        .hash_password(b"wos-dev", &salt_s)
        .unwrap()
        .to_string();
    store
        .upsert_user(&UserRow {
            id: "u-sup".into(),
            email: "supervisor@example.com".into(),
            name: "Supervisor User".into(),
            role: "Supervisor".into(),
            password_hash: hash_s,
            avatar: None,
            auth_epoch: 0,
            created_at: Utc::now(),
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
        session_sweep_enabled: true,
        signer_kind: SignerKind::Noop,
    });

    let storage_handle: wos_server::storage::StorageHandle = store.clone();
    let auth = auth::build(&cfg, storage_handle.clone()).expect("auth build");
    let services = Arc::new(
        AppServices::new(cfg.clone(), storage_handle.clone())
            .await
            .unwrap(),
    );
    services.bundle.hydrate().await.unwrap();
    let (_layer, io) = realtime::build_io_only();

    let mut bindings = BindingRegistry::new();
    bindings.register(FormspecTestAdapter);
    let runtime = AppRuntime::build_with(
        storage_handle.clone(),
        services.provenance.clone(),
        services.bundle.clone(),
        io,
        AppRuntimeConfig {
            bindings,
            ..AppRuntimeConfig::default()
        },
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
    assert_eq!(res.status(), StatusCode::OK, "login for {email} failed");
    let bytes = axum::body::to_bytes(res.into_body(), 8192).await.unwrap();
    let pair: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    pair["accessToken"].as_str().unwrap().to_string()
}

/// Create an instance + drive it past one `start` event so the kernel's
/// `createTask` action puts an active task on `instance.active_tasks`.
/// Returns the runtime-minted instance id and the task id surfaced by
/// `drain_until_idle`.
async fn seed_instance_with_active_task(state: &AppState) -> (String, String) {
    let instance = state
        .runtime
        .create_instance(CreateInstanceRequest {
            instance_id: format!("urn:wos:instance:{}", uuid::Uuid::new_v4()),
            definition_url: KERNEL_URL.to_string(),
            definition_version: KERNEL_VERSION.to_string(),
            initial_case_state: Some(serde_json::json!({ "approved": false })),
        })
        .await
        .expect("create_instance");

    state
        .runtime
        .enqueue_event(
            &instance.instance_id,
            serde_json::json!({
                "event": "start",
                "actorId": "reviewer",
                "data": null,
                "timestamp": "",
            }),
        )
        .await
        .expect("enqueue_event");

    let drained = state
        .runtime
        .drain_until_idle(&instance.instance_id)
        .await
        .expect("drain_until_idle");
    let task_id = drained
        .iter()
        .flat_map(|step| step.created_task_ids.iter())
        .next()
        .cloned()
        .expect("seeded task id from drain");
    assert!(
        task_id.starts_with("wos-task:"),
        "expected wos-task: prefix, got {task_id}"
    );

    (instance.instance_id, task_id)
}

async fn body_json(res: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(res.into_body(), 32_768).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

// ── (a) Draft replay ──────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn task_draft_replays_on_repeat_idempotency_token() {
    let state = jwt_app_state_with_binding().await;
    let (_instance_id, task_id) = seed_instance_with_active_task(&state).await;
    let app = http::router(state);
    let bearer = login(app.clone(), "adjudicator@example.com").await;

    let body = serde_json::json!({
        "idempotencyToken": "tok-replay-1",
        "actorId": "reviewer",
        "response": { "status": "in-progress", "data": { "draft": true } }
    });
    let url = format!("/api/tasks/{task_id}/draft");

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&url)
                .header("authorization", format!("Bearer {bearer}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "first draft POST");
    let first = body_json(res).await;
    let first_artifact = first["artifactId"].as_str().unwrap().to_string();
    assert!(
        !first_artifact.is_empty(),
        "expected non-empty artifactId, got {first:?}"
    );

    // Replay: same body + same token must short-circuit through
    // `ReplayOperation::PersistDraft` and return the cached artifact id.
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&url)
                .header("authorization", format!("Bearer {bearer}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "replay draft POST");
    let second = body_json(res).await;
    assert_eq!(
        second["artifactId"].as_str().unwrap(),
        first_artifact,
        "replay must return original artifactId"
    );
}

// ── (b) Submit completed ──────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn task_submit_response_returns_completed_view() {
    let state = jwt_app_state_with_binding().await;
    let (_instance_id, task_id) = seed_instance_with_active_task(&state).await;
    let app = http::router(state);
    let bearer = login(app.clone(), "adjudicator@example.com").await;

    let body = serde_json::json!({
        "actorId": "reviewer",
        "response": {
            "status": "completed",
            "definitionUrl": "urn:formspec:review",
            "definitionVersion": "1.0.0",
            "data": { "approved": true }
        }
    });

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/tasks/{task_id}/response"))
                .header("authorization", format!("Bearer {bearer}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "submit response should reach Completed branch"
    );
    let json = body_json(res).await;
    assert_eq!(
        json["outcome"].as_str(),
        Some("completed"),
        "expected `TaskSubmissionView::Completed` discriminator, got: {json}"
    );
    assert_eq!(
        json["caseMutated"],
        serde_json::Value::Bool(true),
        "body: {json}"
    );
    // Regression: `TaskSubmissionView` must JSON-serialize with camelCase keys
    // for studio/TS consumers — snake_case (`case_mutated`, …) is a wire bug.
    assert!(
        json.get("case_mutated").is_none(),
        "must not emit snake_case `case_mutated`; use `caseMutated`. body={json}"
    );
    assert!(
        json.get("artifact_id").is_none(),
        "must not emit snake_case `artifact_id`; use `artifactId`. body={json}"
    );
    assert!(
        json.get("emitted_event").is_none(),
        "must not emit snake_case `emitted_event`; use `emittedEvent` when present. body={json}"
    );
    assert!(
        json["artifactId"]
            .as_str()
            .is_some_and(|s| !s.is_empty()),
        "expected non-empty artifactId in Completed view: {json}"
    );
}

// ── (c) Dismiss + follow-up response (lock current runtime behaviour) ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn task_dismiss_then_respond_locks_runtime_behavior() {
    let state = jwt_app_state_with_binding().await;
    let (_instance_id, task_id) = seed_instance_with_active_task(&state).await;
    let app = http::router(state);
    let sup_bearer = login(app.clone(), "supervisor@example.com").await;
    let adj_bearer = login(app.clone(), "adjudicator@example.com").await;

    // /dismiss is `RequireRole<Supervisor>` post-WS-083.
    let dismiss_body = serde_json::json!({ "reason": "supervisor revoked queue" });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/tasks/{task_id}/dismiss"))
                .header("authorization", format!("Bearer {sup_bearer}"))
                .header("content-type", "application/json")
                .body(Body::from(dismiss_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "dismiss should succeed");
    let dismiss_json = body_json(res).await;
    assert_eq!(dismiss_json["ok"], serde_json::Value::Bool(true));

    // Follow-up `/response` after dismiss. Per the current `wos-runtime`
    // semantics (`runtime/tasks.rs::dismiss_task` records provenance but
    // leaves `instance.active_tasks` untouched — see the trait round-trip
    // in `wos-runtime::runtime` tests, which exercises persist → dismiss
    // → submit → Completed), the task remains respondable.
    //
    // If WS-011's "task-missing" wording is later promoted to spec, the
    // runtime must remove the task on dismiss; at that point this test
    // flips to assert 404 / `TaskNotFound`. Locking current behaviour
    // here keeps the HTTP surface honest about runtime semantics.
    let response_body = serde_json::json!({
        "actorId": "reviewer",
        "response": {
            "status": "completed",
            "definitionUrl": "urn:formspec:review",
            "definitionVersion": "1.0.0",
            "data": { "approved": true }
        }
    });
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/tasks/{task_id}/response"))
                .header("authorization", format!("Bearer {adj_bearer}"))
                .header("content-type", "application/json")
                .body(Body::from(response_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let json = body_json(res).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "post-dismiss /response: current runtime keeps task in active_tasks; got body {json}"
    );
    assert_eq!(
        json["outcome"].as_str(),
        Some("completed"),
        "post-dismiss /response should still complete under current runtime semantics: {json}"
    );
}
