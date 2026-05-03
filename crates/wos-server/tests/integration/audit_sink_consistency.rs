use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use tower::ServiceExt;
use wos_server::config::{
    AiChatKind, AuditSinkKind, AuthKind, RuntimeKind, ServerConfig, SignerKind, StorageKind,
};
use wos_server::runtime::{AppRuntime, AppRuntimeConfig};
use wos_server::storage::KernelRow;
use wos_server::{AppState, auth, http, realtime, services::AppServices, storage};
use wos_server_ports::audit::{AuditError, AuditResult, AuditSink, ExportEnvelope};
use wos_server_ports::storage::ProvenanceRow;

fn stub_kernel_document(url: &str, version: &str) -> serde_json::Value {
    serde_json::json!({
        "$wosWorkflow": "1.0.0",
        "url": url,
        "version": version,
        "title": "Test Kernel",
        "status": "active",
        "lifecycle": { "initialState": "intake", "states": { "intake": { "type": "atomic" } } },
        "actors": [{ "id": "applicant", "type": "human" }],
        "contracts": {}
    })
}

#[derive(Debug)]
struct AlwaysFailAuditSink;

#[async_trait::async_trait]
impl AuditSink for AlwaysFailAuditSink {
    async fn append_provenance(&self, _records: &[ProvenanceRow]) -> AuditResult<()> {
        Err(AuditError::Backend(
            "intentional audit failure for consistency test".into(),
        ))
    }

    async fn append_export(&self, _envelope: ExportEnvelope) -> AuditResult<()> {
        Ok(())
    }
}

async fn state_with_failing_audit_sink() -> AppState {
    let cfg = Arc::new(ServerConfig {
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
        runtime: RuntimeKind::Local,
        audit_sink: AuditSinkKind::None,
        audit_database_url: String::new(),
        session_sweep_enabled: true,
        signer_kind: SignerKind::Noop,
    });
    let storage = storage::build(&cfg).await.expect("storage build");
    storage
        .upsert_kernel(&KernelRow {
            url: "urn:wos:workflow:test:1.0.0".into(),
            title: "Test Kernel".into(),
            version: "1.0.0".into(),
            status: "active".into(),
            impact_level: "operational".into(),
            document: stub_kernel_document("urn:wos:workflow:test:1.0.0", "1.0.0"),
            updated_at: Utc::now(),
        })
        .await
        .expect("kernel upsert");

    let auth = auth::build(&cfg, storage.clone()).expect("auth build");
    let services = Arc::new(
        AppServices::new(cfg.clone(), storage.clone())
            .await
            .expect("services"),
    );
    let (_layer, io) = realtime::build_io_only();
    let runtime = AppRuntime::build_with(
        storage.clone(),
        services.provenance.clone(),
        services.bundle.clone(),
        io,
        AppRuntimeConfig {
            audit_sink: Arc::new(AlwaysFailAuditSink),
            ..AppRuntimeConfig::default()
        },
    );
    AppState {
        cfg,
        storage,
        auth,
        services,
        runtime,
        event_idempotency: Arc::new(Mutex::new(HashMap::new())),
        migrate_idempotency: Arc::new(tokio::sync::Mutex::new(wos_server::MigrateIdempotencyCache::default())),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_instance_succeeds_even_when_audit_sink_fails() {
    let state = state_with_failing_audit_sink().await;
    let app = http::router(state.clone());
    let body = serde_json::json!({
        "definitionUrl": "urn:wos:workflow:test:1.0.0",
        "definitionVersion": "1.0.0",
        "instanceId": "urn:wos:instance:test:audit-fail"
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/instances")
                .header("content-type", "application/json")
                .header("authorization", "Bearer mock-access")
                .body(Body::from(body.to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
}
