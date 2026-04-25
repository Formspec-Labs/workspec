//! Tests for WS-029 (explain endpoint), WS-037 (assurance chain continuity),
//! WS-038 (calibration expiry), WS-041 (JSON-LD context), WS-013
//! (eval-service parse-failure surface), and WS-035 (hold CRUD).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use tower::ServiceExt;
use wos_server::config::{AuthKind, ServerConfig, StorageKind};
use wos_server::runtime::AppRuntime;
use wos_server::{AppState, auth, http, realtime, services::AppServices, storage};
use wos_server::storage::{IdentityFactRow, InstanceRow, SqliteStorage, Storage, UserRow};

async fn test_app_state() -> AppState {
    let store = Arc::new(
        SqliteStorage::connect("sqlite::memory:?cache=shared")
            .await
            .unwrap(),
    );
    store.migrate().await.unwrap();

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
        signer_kind: wos_server::config::SignerKind::Noop,
    });

    let storage_handle: storage::StorageHandle = store.clone();
    let auth = auth::build(&cfg, storage_handle.clone());

    use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
    use rand::rngs::OsRng;
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(b"wos-dev", &salt)
        .unwrap()
        .to_string();
    store
        .upsert_user(&UserRow {
            id: "u1".into(),
            email: "supervisor@example.com".into(),
            name: "Supervisor".into(),
            role: "Supervisor".into(),
            password_hash: hash,
            avatar: None,
            auth_epoch: 0,
            created_at: Utc::now(),
        })
        .await
        .unwrap();

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

fn app(state: AppState) -> axum::Router {
    http::router(state)
}

async fn body_str(res: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(res.into_body(), 16384)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

// ── WS-041: JSON-LD context ──────────────────────────────────────────

#[tokio::test]
async fn jsonld_context_returns_valid_context() {
    let state = test_app_state().await;
    let app = app(state);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/semantic/jsonld-context")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = body_str(res).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let ctx = json.get("@context").expect("@context key");
    assert!(ctx.get("wos").is_some());
    assert!(ctx.get("prov").is_some());
    assert!(ctx.get("instanceId").is_some());
    assert!(ctx.get("recordKind").is_some());
}

// ── WS-029: Explain endpoint ─────────────────────────────────────────

#[tokio::test]
async fn explain_returns_404_for_missing_instance() {
    let state = test_app_state().await;
    let app = app(state);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/instances/no-such-id/explain?transitionId=t1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn explain_returns_empty_explanation_for_instance_without_matching_provenance() {
    let state = test_app_state().await;
    let app = app(state.clone());

    let now = Utc::now();
    let instance_id = "urn:wos:instance:explain-test-1";
    state
        .storage
        .create_instance(&InstanceRow {
            instance_id: instance_id.into(),
            definition_url: "test://kernel".into(),
            definition_version: "1.0".into(),
            status: "active".into(),
            impact_level: "rights-impacting".into(),
            instance_json: serde_json::json!({
                "instanceId": instance_id,
                "definitionUrl": "test://kernel",
                "definitionVersion": "1.0",
                "configuration": ["review"],
                "caseState": {},
                "governanceState": null,
            }),
            runtime_aux_json: serde_json::json!({}),
            created_at: now,
            updated_at: now,
        })
        .await
        .unwrap();

    let res = app
        .oneshot(
            Request::builder()
                .uri(&format!(
                    "/api/instances/{instance_id}/explain?transitionId=deny-1&tags=adverse-decision,denial"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = body_str(res).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["transitionId"], "deny-1");
    assert_eq!(json["determination"][0], "adverse-decision");
    assert_eq!(json["determination"][1], "denial");
    assert!(json["reasoning"].as_array().unwrap().is_empty());
    assert!(json["rendered"].is_string());
}

// ── WS-037: Assurance chain continuity ────────────────────────────────

#[tokio::test]
async fn assurance_chain_valid_on_clean_chain() {
    let state = test_app_state().await;
    let storage = &state.storage;
    let now = Utc::now();

    let fact1_id = "urn:wos:identity-fact:chain-1";
    let fact2_id = "urn:wos:identity-fact:chain-2";

    storage
        .insert_identity_fact(&IdentityFactRow {
            id: fact1_id.into(),
            instance_id: "inst-1".into(),
            subject_ref: "subject-alice".into(),
            assurance_level: "l1".into(),
            disclosure_posture: "open".into(),
            fact_json: serde_json::json!({"name": "Alice"}),
            upgraded_from: None,
            created_at: now,
        })
        .await
        .unwrap();

    storage
        .insert_identity_fact(&IdentityFactRow {
            id: fact2_id.into(),
            instance_id: "inst-1".into(),
            subject_ref: "subject-alice".into(),
            assurance_level: "l2".into(),
            disclosure_posture: "open".into(),
            fact_json: serde_json::json!({"name": "Alice", "method": "document"}),
            upgraded_from: Some(fact1_id.into()),
            created_at: now,
        })
        .await
        .unwrap();

    let app = app(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/subjects/subject-alice/assurance-chain")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = body_str(res).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["chainValid"], true);
    assert!(json["brokenAt"].is_null());
    assert_eq!(json["facts"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn assurance_chain_detects_broken_upgrade_link() {
    let state = test_app_state().await;
    let storage = &state.storage;
    let now = Utc::now();

    storage
        .insert_identity_fact(&IdentityFactRow {
            id: "urn:wos:identity-fact:broken-1".into(),
            instance_id: "inst-1".into(),
            subject_ref: "subject-bob".into(),
            assurance_level: "l2".into(),
            disclosure_posture: "open".into(),
            fact_json: serde_json::json!({"name": "Bob"}),
            upgraded_from: Some("urn:wos:identity-fact:missing-fact".into()),
            created_at: now,
        })
        .await
        .unwrap();

    let app = app(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/subjects/subject-bob/assurance-chain")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = body_str(res).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["chainValid"], false);
    assert_eq!(
        json["brokenAt"].as_str().unwrap(),
        "urn:wos:identity-fact:broken-1"
    );
}

// ── WS-038: Calibration expiry ────────────────────────────────────────

#[tokio::test]
async fn tool_invocation_rejected_when_calibration_expired() {
    let state = test_app_state().await;
    let storage = &state.storage;
    let now = Utc::now();

    let agent_id = "urn:wos:agent:expired-agent";
    storage
        .upsert_agent(&storage::AgentRow {
            id: agent_id.into(),
            workflow_url: "test://workflow".into(),
            name: "Expired Agent".into(),
            kind: "generative".into(),
            version: "1.0".into(),
            status: "active".into(),
            autonomy: Some("supervised".into()),
            confidence_floor: Some(0.8),
            config_json: serde_json::json!({
                "calibrationExpiresAt": "2020-01-01T00:00:00Z"
            }),
            deployment_state: "production".into(),
            created_at: now,
            updated_at: now,
        })
        .await
        .unwrap();

    let app = app(state);
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/agents/{agent_id}/tool-invocation-check"))
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = body_str(res).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["allowed"], false);
    assert!(json["reason"].as_str().unwrap().contains("calibration"));
}

#[tokio::test]
async fn tool_invocation_allowed_when_not_expired() {
    let state = test_app_state().await;
    let storage = &state.storage;
    let now = Utc::now();

    let agent_id = "urn:wos:agent:valid-agent";
    storage
        .upsert_agent(&storage::AgentRow {
            id: agent_id.into(),
            workflow_url: "test://workflow".into(),
            name: "Valid Agent".into(),
            kind: "generative".into(),
            version: "1.0".into(),
            status: "active".into(),
            autonomy: Some("supervised".into()),
            confidence_floor: Some(0.8),
            config_json: serde_json::json!({
                "calibrationExpiresAt": "2099-12-31T23:59:59Z"
            }),
            deployment_state: "production".into(),
            created_at: now,
            updated_at: now,
        })
        .await
        .unwrap();

    let app = app(state);
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/agents/{agent_id}/tool-invocation-check"))
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = body_str(res).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["allowed"], true);
}

#[tokio::test]
async fn lifecycle_activation_blocked_when_calibration_expired() {
    let state = test_app_state().await;
    let storage = &state.storage;
    let now = Utc::now();

    let agent_id = "urn:wos:agent:suspended-expired";
    storage
        .upsert_agent(&storage::AgentRow {
            id: agent_id.into(),
            workflow_url: "test://workflow".into(),
            name: "Suspended Expired".into(),
            kind: "generative".into(),
            version: "1.0".into(),
            status: "suspended".into(),
            autonomy: Some("supervised".into()),
            confidence_floor: Some(0.8),
            config_json: serde_json::json!({
                "calibrationExpiresAt": "2020-01-01T00:00:00Z"
            }),
            deployment_state: "production".into(),
            created_at: now,
            updated_at: now,
        })
        .await
        .unwrap();

    let bearer = supervisor_bearer(&state).await;
    let res = app(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/agents/{agent_id}/lifecycle-transition"))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {bearer}"))
                .body(Body::from(
                    serde_json::json!({
                        "targetState": "active",
                        "reason": "recalibrated"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body = body_str(res).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json["message"]
        .as_str()
        .unwrap()
        .contains("calibration"));
}

// ── WS-013: EvalService parse-failure surface ─────────────────────────

#[tokio::test]
async fn transitions_returns_503_on_malformed_instance_json() {
    let state = test_app_state().await;
    let storage = &state.storage;
    let now = Utc::now();

    let definition_url = "test://kernel-ws013";
    let instance_id = "urn:wos:instance:malformed";

    storage
        .upsert_kernel(&storage::KernelRow {
            url: definition_url.into(),
            title: "Test Kernel".into(),
            version: "1.0".into(),
            status: "active".into(),
            impact_level: "rights-impacting".into(),
            document: serde_json::json!({
                "lifecycle": {
                    "states": {
                        "review": {
                            "transitions": []
                        }
                    }
                }
            }),
            updated_at: now,
        })
        .await
        .unwrap();
    state.services.bundle.hydrate().await.unwrap();

    storage
        .create_instance(&InstanceRow {
            instance_id: instance_id.into(),
            definition_url: definition_url.into(),
            definition_version: "1.0".into(),
            status: "active".into(),
            impact_level: "rights-impacting".into(),
            instance_json: serde_json::json!("not an object"),
            runtime_aux_json: serde_json::json!({}),
            created_at: now,
            updated_at: now,
        })
        .await
        .unwrap();

    let app = app(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri(&format!("/api/instances/{instance_id}/transitions"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = body_str(res).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let msg = json["message"].as_str().unwrap();
    assert!(
        msg.contains(instance_id) || msg.contains("failed to deserialise"),
        "expected instance id or deserialise error in message: {msg}"
    );
}

// ── WS-035: Hold CRUD ────────────────────────────────────────────────

async fn supervisor_bearer(state: &AppState) -> String {
    let app = app(state.clone());
    let body = serde_json::json!({
        "email": "supervisor@example.com",
        "password": "wos-dev",
    });
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
    assert_eq!(res.status(), StatusCode::OK, "supervisor login failed");
    let bytes = axum::body::to_bytes(res.into_body(), 8192).await.unwrap();
    let pair: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    pair["accessToken"].as_str().unwrap().to_string()
}

async fn seed_instance_with_state(state: &AppState, id: &str, configuration: Vec<&str>) {
    let now = Utc::now();
    let cfg_json: Vec<serde_json::Value> = configuration
        .iter()
        .map(|s| serde_json::Value::String((*s).into()))
        .collect();
    state
        .storage
        .create_instance(&InstanceRow {
            instance_id: id.into(),
            definition_url: "test://kernel".into(),
            definition_version: "1.0".into(),
            status: "active".into(),
            impact_level: "rights-impacting".into(),
            instance_json: serde_json::json!({
                "instanceId": id,
                "definitionUrl": "test://kernel",
                "definitionVersion": "1.0",
                "configuration": cfg_json,
                "caseState": {},
                "governanceState": null,
            }),
            runtime_aux_json: serde_json::json!({}),
            created_at: now,
            updated_at: now,
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn list_holds_returns_empty_for_instance_without_governance_state() {
    let state = test_app_state().await;
    seed_instance_with_state(&state, "urn:wos:instance:hold-empty", vec!["intake"]).await;

    let res = app(state)
        .oneshot(
            Request::builder()
                .uri("/api/instances/urn:wos:instance:hold-empty/holds")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_str(res).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn create_hold_requires_auth() {
    let state = test_app_state().await;
    seed_instance_with_state(&state, "urn:wos:instance:hold-auth", vec!["review"]).await;

    let body = serde_json::json!({
        "holdType": "evidence-pending",
        "resumeTrigger": "documentReceived",
    });
    let res = app(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/instances/urn:wos:instance:hold-auth/holds")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn hold_lifecycle_create_list_release_round_trips() {
    let state = test_app_state().await;
    let instance_id = "urn:wos:instance:hold-round-trip";
    seed_instance_with_state(&state, instance_id, vec!["evidence-review"]).await;
    let bearer = supervisor_bearer(&state).await;

    // Create hold #0
    let create_body = serde_json::json!({
        "holdType": "evidence-pending",
        "resumeTrigger": "documentReceived",
    });
    let res = app(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/instances/{instance_id}/holds"))
                .header("authorization", format!("Bearer {bearer}"))
                .header("content-type", "application/json")
                .body(Body::from(create_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_str(res).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["holdIndex"], 0);

    // Create a second hold to confirm append + indexing
    let create_body2 = serde_json::json!({
        "holdType": "supervisor-review",
        "resumeTrigger": "supervisorApproved",
        "expectedEnd": "2026-05-01T00:00:00Z",
        "holdState": "review",
    });
    let res = app(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/instances/{instance_id}/holds"))
                .header("authorization", format!("Bearer {bearer}"))
                .header("content-type", "application/json")
                .body(Body::from(create_body2.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_str(res).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["holdIndex"], 1);

    // List holds: expect 2, with default hold_state derived from configuration
    let res = app(state.clone())
        .oneshot(
            Request::builder()
                .uri(&format!("/api/instances/{instance_id}/holds"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_str(res).await;
    let holds: serde_json::Value = serde_json::from_str(&body).unwrap();
    let arr = holds.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["holdType"], "evidence-pending");
    assert_eq!(arr[0]["holdState"], "evidence-review"); // defaulted from configuration[0]
    assert_eq!(arr[1]["holdType"], "supervisor-review");
    assert_eq!(arr[1]["holdState"], "review"); // explicitly provided

    // Release hold at index 0
    let res = app(state.clone())
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/instances/{instance_id}/holds/0"))
                .header("authorization", format!("Bearer {bearer}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_str(res).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["released"]["holdType"], "evidence-pending");

    // List again: expect 1 remaining
    let res = app(state)
        .oneshot(
            Request::builder()
                .uri(&format!("/api/instances/{instance_id}/holds"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_str(res).await;
    let holds: serde_json::Value = serde_json::from_str(&body).unwrap();
    let arr = holds.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["holdType"], "supervisor-review");
}

#[tokio::test]
async fn release_hold_returns_404_for_out_of_range_index() {
    let state = test_app_state().await;
    let instance_id = "urn:wos:instance:hold-bad-idx";
    seed_instance_with_state(&state, instance_id, vec!["intake"]).await;
    let bearer = supervisor_bearer(&state).await;

    let res = app(state)
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/instances/{instance_id}/holds/7"))
                .header("authorization", format!("Bearer {bearer}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
