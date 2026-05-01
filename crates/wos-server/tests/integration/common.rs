//! Shared helpers for HTTP integration tests (`http_coverage_*`, etc.).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use axum::body::Body;
use axum::http::Request;
use chrono::Utc;
use rand::rngs::OsRng;
use tempfile::TempDir;
use tower::ServiceExt;
use uuid::Uuid;
use wos_core::provenance::ProvenanceRecord;
use wos_server::config::{
    AiChatKind, AuditSinkKind, AuthKind, RuntimeKind, ServerConfig, SignerKind, StorageKind,
};
use wos_server::services::provenance_service::chain_hash;
use wos_server::storage::{InstanceRow, ProvenanceRow, SqliteStorage, Storage, UserRow};
use wos_server::{AppState, auth, realtime, services::AppServices, storage};

pub const ZERO_HASH: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";

pub fn stub_config(fixtures_dir: PathBuf) -> Arc<ServerConfig> {
    let _ = fixtures_dir;
    let database_url = format!(
        "sqlite://file:test-{}?mode=memory&cache=shared",
        Uuid::now_v7()
    );
    Arc::new(ServerConfig {
        port: 0,
        fixtures_dir,
        storage: StorageKind::Sqlite,
        database_url,
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

    // Business calendar fixture body. Per ADR 0076 D-3 the canonical surface
    // is `$wosDelivery.calendar`, but `wos-server`'s `bundle_service` still
    // reads the legacy per-sidecar subdirectory layout (`business-calendar/{slug}.json`)
    // and projects the file contents into `BundleView.business_calendar` —
    // see `crates/wos-server/src/services/bundle_service.rs` SIDECARS list.
    // Fixture omits the legacy `$wosBusinessCalendar` marker (dead decoration);
    // bundle_service migration to the merged delivery sidecar is tracked
    // separately as ADR 0076 D-3 follow-on.
    let cal = serde_json::json!({
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
        root.join("notification-template")
            .join(format!("{slug}.json")),
        serde_json::to_vec_pretty(&tmpl).unwrap(),
    )
    .unwrap();

    std::fs::create_dir_all(root.join("integration-profile")).unwrap();
    std::fs::write(
        root.join("integration-profile")
            .join(format!("{slug}.json")),
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

pub async fn seed_instance_with_one_provenance(store: &storage::StorageHandle, instance_id: &str) {
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
    path_encode(SLICE_B_WORKFLOW)
}

pub fn int_consume_001_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../wos-conformance/tests/fixtures/INT-CONSUME-001-happy.json")
}

pub fn path_encode(raw: &str) -> String {
    url::form_urlencoded::byte_serialize(raw.as_bytes()).collect()
}

pub fn fixed_time() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
        .expect("valid fixed timestamp")
        .with_timezone(&chrono::Utc)
}

pub async fn jwt_state(fixtures_dir: PathBuf) -> AppState {
    let _ = fixtures_dir;
    let database_url = format!(
        "sqlite://file:jwt-test-{}?mode=memory&cache=shared",
        Uuid::now_v7()
    );
    let store = Arc::new(
        SqliteStorage::connect(&database_url)
            .await
            .expect("sqlite connect"),
    );
    store.migrate().await.expect("sqlite migrate");

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(b"wos-dev", &salt)
        .expect("hash fixture password")
        .to_string();
    for (id, role) in [
        ("sup", "Supervisor"),
        ("adj", "Adjudicator"),
        ("app", "Applicant"),
    ] {
        store
            .upsert_user(&UserRow {
                id: id.into(),
                email: format!("{id}@example.com"),
                name: id.into(),
                role: role.into(),
                password_hash: hash.clone(),
                avatar: None,
                auth_epoch: 0,
                created_at: fixed_time(),
            })
            .await
            .expect("seed user");
    }

    let cfg = Arc::new(ServerConfig {
        port: 0,
        fixtures_dir,
        storage: StorageKind::Sqlite,
        database_url,
        auth: AuthKind::Jwt,
        jwt_secret: "test-secret-common".into(),
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
    });

    let st: storage::StorageHandle = store.clone();
    let au = auth::build(&cfg, st.clone()).expect("auth build");
    let svc = Arc::new(
        AppServices::new(cfg.clone(), st.clone())
            .await
            .expect("services"),
    );
    let (_layer, io) = realtime::build_io_only();
    let rt = wos_server::runtime::AppRuntime::build(
        st.clone(),
        svc.provenance.clone(),
        svc.bundle.clone(),
        io,
    );
    AppState {
        cfg,
        storage: st,
        auth: au,
        services: svc,
        runtime: rt,
        event_idempotency: Arc::new(Mutex::new(HashMap::new())),
    }
}

pub async fn login_access_token(app: axum::Router, email: &str) -> String {
    let body = serde_json::json!({ "email": email, "password": "wos-dev" });
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("login request"),
        )
        .await
        .expect("login response");
    let bytes = axum::body::to_bytes(res.into_body(), 8192)
        .await
        .expect("login bytes");
    let pair: serde_json::Value = serde_json::from_slice(&bytes).expect("login json");
    pair["accessToken"]
        .as_str()
        .expect("access token")
        .to_string()
}
