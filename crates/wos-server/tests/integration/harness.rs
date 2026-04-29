//! Shared AppState boot + provenance seeding for `http_coverage_*` integration tests.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use wos_core::provenance::ProvenanceRecord;
use wos_server::config::{
    AiChatKind, AuditSinkKind, AuthKind, RuntimeKind, ServerConfig, SignerKind, StorageKind,
};
use wos_server::services::provenance_service::chain_hash;
use wos_server::storage::{InstanceRow, ProvenanceRow};
use wos_server::{AppState, auth, realtime, services::AppServices, storage};

pub const ZERO_HASH: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";

pub fn stub_config(fixtures_dir: PathBuf) -> Arc<ServerConfig> {
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
        runtime: RuntimeKind::Local,
        audit_sink: AuditSinkKind::None,
        audit_database_url: String::new(),
        session_sweep_enabled: false,
        signer_kind: SignerKind::Noop,
    })
}

pub async fn bring_up_with_cfg(cfg: Arc<ServerConfig>) -> AppState {
    let storage = storage::build(&cfg).await.unwrap();
    let auth = auth::build(&cfg, storage.clone()).expect("auth build");
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

pub async fn bring_up() -> AppState {
    bring_up_with_cfg(stub_config(PathBuf::from("."))).await
}

pub fn make_instance_row(id: &str) -> InstanceRow {
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

pub async fn seed_instance_with_one_provenance(
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
