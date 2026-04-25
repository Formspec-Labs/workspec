//! Verify the `/instances/{id}/signature-affirmations` endpoint returns only
//! `SignatureAffirmation` provenance records, filtered from the full chain.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use tower::ServiceExt;
use wos_core::provenance::ProvenanceRecord;
use wos_server::config::{AiChatKind, AuthKind, ServerConfig, StorageKind};
use wos_server::storage::ProvenanceRow;
use wos_server::{AppState, auth, http, realtime, services::AppServices, storage};
use wos_server::services::provenance_service::chain_hash;

const INSTANCE_ID: &str = "urn:wos:instance:test:sig-affirmations";

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
        signer_kind: wos_server::config::SignerKind::Noop,
    })
}

fn provenance_row(seq: i64, record: &ProvenanceRecord, prev_hash: &str) -> ProvenanceRow {
    let ts = Utc::now();
    let tier = record.audit_layer.as_deref().unwrap_or("facts");
    let payload = serde_json::to_value(record).unwrap();
    let hash = chain_hash(prev_hash, INSTANCE_ID, seq, &ts, tier, &payload);
    ProvenanceRow {
        id: format!("rec-sig-{seq}"),
        instance_id: INSTANCE_ID.into(),
        seq,
        timestamp: ts,
        tier: tier.into(),
        payload,
        hash,
        previous_hash: prev_hash.into(),
    }
}

fn make_instance_row(id: &str) -> storage::InstanceRow {
    storage::InstanceRow {
        instance_id: id.into(),
        definition_url: "urn:wos:workflow:test:1.0.0".into(),
        definition_version: "1.0.0".into(),
        status: "active".into(),
        impact_level: "operational".into(),
        instance_json: serde_json::json!({
            "instanceId": id,
            "definitionUrl": "urn:wos:workflow:test:1.0.0",
            "status": "active",
            "configuration": ["signing"],
        }),
        runtime_aux_json: serde_json::Value::Null,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

async fn seed_mixed_provenance(store: &storage::StorageHandle) {
    store.create_instance(&make_instance_row(INSTANCE_ID)).await.unwrap();

    let mut transition_record = ProvenanceRecord::state_transition(
        "draft",
        "signing",
        "submit",
        Some("applicant"),
    );
    transition_record.audit_layer = Some("facts".into());

    let mut sig_record = ProvenanceRecord::signature_affirmation(
        wos_core::provenance::SignatureAffirmationInput {
            ceremony_id: "ceremony-1",
            role_id: "applicantSigner",
            role: "signer",
            document_id: "application",
            document_hash: "abc123",
            document_hash_algorithm: "sha-256",
            signer_id: "applicant",
            signed_at: "2026-04-24T10:00:00Z",
            identity_binding: serde_json::json!("urn:test:identity:applicant"),
            consent_reference: serde_json::json!({
                "consentTextRef": "urn:test:consent:esign",
                "consentVersion": "1.0.0",
            }),
            signature_provider: "wos-reference",
            profile_ref: None,
            profile_key: None,
            formspec_response_ref: "urn:test:formspec-response:application",
            custody_hook_eligible: true,
        },
    );
    sig_record.actor_id = Some("applicant".into());
    sig_record.audit_layer = Some("facts".into());

    let mut another_transition = ProvenanceRecord::state_transition(
        "signing",
        "complete",
        "allSigned",
        Some("system"),
    );
    another_transition.audit_layer = Some("facts".into());

    const ZERO_HASH: &str =
        "sha256:0000000000000000000000000000000000000000000000000000000000000000";

    let row1 = provenance_row(1, &transition_record, ZERO_HASH);
    let row2 = provenance_row(2, &sig_record, &row1.hash);
    let row3 = provenance_row(3, &another_transition, &row2.hash);

    let rows = vec![row1, row2, row3];
    store
        .update_instance_atomic(INSTANCE_ID, &move |_row| Ok(rows.clone()))
        .await
        .unwrap();
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn signature_affirmations_filters_from_mixed_provenance() {
    let state = bring_up().await;
    seed_mixed_provenance(&state.storage).await;
    let app = http::router(state);

    let encoded = INSTANCE_ID.replace(':', "%3A");
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/instances/{encoded}/signature-affirmations").as_str())
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
    let items = body.as_array().expect("response should be an array");
    assert_eq!(items.len(), 1, "should return exactly 1 signature affirmation");
    let sig = &items[0];
    assert_eq!(
        sig.get("recordKind").and_then(|v| v.as_str()),
        Some("signatureAffirmation"),
    );
    let data = sig.get("data").expect("should have data field");
    assert_eq!(
        data.get("ceremonyId").and_then(|v| v.as_str()),
        Some("ceremony-1"),
    );
    assert_eq!(
        data.get("signerId").and_then(|v| v.as_str()),
        Some("applicant"),
    );
    assert_eq!(
        data.get("roleId").and_then(|v| v.as_str()),
        Some("applicantSigner"),
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn signature_affirmations_returns_empty_for_instance_without_signing() {
    let state = bring_up().await;
    state
        .storage
        .create_instance(&make_instance_row("urn:wos:instance:test:no-sig"))
        .await
        .unwrap();
    let app = http::router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/instances/urn%3Awos%3Ainstance%3Atest%3Ano-sig/signature-affirmations")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 8192)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(
        body.as_array().map(|a| a.is_empty()).unwrap_or(false),
        "should return empty array for instance without signing records"
    );
}
