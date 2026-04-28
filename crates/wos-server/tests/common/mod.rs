//! Shared helpers for HTTP integration tests (`http_coverage_*`, etc.).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use tempfile::TempDir;
use wos_core::provenance::ProvenanceRecord;
use wos_server::config::{AiChatKind, AuthKind, ServerConfig, SignerKind, StorageKind};
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
        session_sweep_enabled: false,
        signer_kind: SignerKind::Noop,
    })
}

pub async fn bring_up_with_cfg(cfg: Arc<ServerConfig>) -> AppState {
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

pub async fn bring_up() -> AppState {
    bring_up_with_cfg(stub_config(PathBuf::from("."))).await
}

/// Temp fixture tree: kernel `urn:wos:workflow:{slug}:1.0.0` plus business-calendar,
/// notification-template, and integration-profile sidecars keyed by [`slug`].
pub fn slice_b_tempdir() -> TempDir {
    let dir = TempDir::new().unwrap();
    let slug = "ws014sliceb";
    let workflow_url = format!("urn:wos:workflow:{slug}:1.0.0");
    let root = dir.path();

    let kernel = serde_json::json!({
        "$wosWorkflow": "1.0",
        "url": workflow_url,
        "version": "1.0.0",
        "title": "WS-014 slice B",
        "status": "active",
        "impactLevel": "operational",
        "actors": [{ "id": "sys", "type": "system" }],
        "lifecycle": {
            "initialState": "done",
            "states": { "done": { "type": "final" } }
        },
        "contracts": {}
    });
    std::fs::create_dir_all(root.join("kernel")).unwrap();
    std::fs::write(
        root.join("kernel").join(format!("{slug}.json")),
        serde_json::to_vec_pretty(&kernel).unwrap(),
    )
    .unwrap();

    let cal = serde_json::json!({
        "$wosBusinessCalendar": "1.0",
        "targetWorkflow": workflow_url,
        "timezone": "UTC",
        "workWeek": ["monday", "tuesday", "wednesday", "thursday", "friday"],
        "holidays": []
    });
    std::fs::create_dir_all(root.join("business-calendar")).unwrap();
    std::fs::write(
        root.join("business-calendar").join(format!("{slug}.json")),
        serde_json::to_vec_pretty(&cal).unwrap(),
    )
    .unwrap();

    let tmpl = serde_json::json!({
        "templates": [
            {
                "id": "notice",
                "subject": "Hello",
                "body": "Hello ${user}",
                "channels": ["email"]
            }
        ]
    });
    std::fs::create_dir_all(root.join("notification-template")).unwrap();
    std::fs::write(
        root.join("notification-template").join(format!("{slug}.json")),
        serde_json::to_vec_pretty(&tmpl).unwrap(),
    )
    .unwrap();

    std::fs::create_dir_all(root.join("integration-profile")).unwrap();
    std::fs::write(
        root.join("integration-profile").join(format!("{slug}.json")),
        br#"{"bindings":[]}"#,
    )
    .unwrap();

    dir
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

pub const SLICE_B_WORKFLOW: &str = "urn:wos:workflow:ws014sliceb:1.0.0";

pub fn slice_b_workflow_path_encoded() -> String {
    SLICE_B_WORKFLOW.replace(':', "%3A")
}

pub fn int_consume_001_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../wos-conformance/tests/fixtures/INT-CONSUME-001-happy.json")
}
