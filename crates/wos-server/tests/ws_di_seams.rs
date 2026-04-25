//! Unit tests for WS-024 (NoopSigner), WS-025 (JsonRenderer),
//! WS-026 (PolicyLayeredValidator), WS-027 (RoleBasedAccessControl).

use wos_core::model::governance::DelegationScope;
use wos_core::provenance::ProvenanceRecord;
use wos_core::traits::{AccessControl, ContractValidator, ProvenanceSigner, ReportRenderer};
use wos_server::runtime::access::RoleBasedAccessControl;
use wos_server::runtime::renderer::JsonRenderer;
use wos_server::runtime::signer::NoopSigner;
use wos_server::runtime::validator::{PermissiveValidator, PolicyLayeredValidator};

fn stub_record() -> ProvenanceRecord {
    ProvenanceRecord {
        id: "test".into(),
        record_kind: wos_core::provenance::ProvenanceKind::StateTransition,
        timestamp: "2026-04-24T00:00:00Z".into(),
        actor_id: None,
        from_state: None,
        to_state: None,
        event: Some("submit".into()),
        data: None,
        audit_layer: None,
        actor_type: None,
        lifecycle_state: None,
        definition_version: None,
        inputs: Vec::new(),
        outputs: Vec::new(),
        input_digest: None,
        output_digest: None,
        canonical_event_hash: None,
        transition_tags: Vec::new(),
        case_file_snapshot: None,
        outcome: None,
    }
}

#[test]
fn noop_signer_produces_empty_signature() {
    let signer = NoopSigner;
    let sig = signer.sign(&stub_record()).unwrap();
    assert!(sig.is_empty());
}

#[test]
fn noop_signer_verifies_any_signature() {
    let signer = NoopSigner;
    assert!(signer.verify(&stub_record(), b"anything").unwrap());
}

#[test]
fn json_renderer_produces_valid_json_explanation() {
    let renderer = JsonRenderer;
    let input = serde_json::json!({"steps": ["a", "b"]});
    let output = renderer.render_explanation(&input, "test").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["steps"][0], "a");
}

#[test]
fn json_renderer_produces_valid_json_audit() {
    let renderer = JsonRenderer;
    let records = vec![stub_record()];
    let output = renderer.render_audit(&records, "json").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(parsed.is_array());
}

#[test]
fn permissive_validator_accepts_anything() {
    let v = PermissiveValidator;
    let result = v.validate("any", &serde_json::json!({})).unwrap();
    assert!(result.valid);
    assert!(result.errors.is_empty());
}

#[test]
fn layered_validator_rejects_rights_impacting_without_ledger() {
    let v = PolicyLayeredValidator::new(PermissiveValidator);
    let result = v
        .validate(
            "contract",
            &serde_json::json!({
                "impactLevel": "rights-impacting"
            }),
        )
        .unwrap();
    assert!(!result.valid);
    assert!(result.errors.iter().any(|e| e.contains("§15.7")));
}

#[test]
fn layered_validator_accepts_rights_impacting_with_ledger() {
    let v = PolicyLayeredValidator::new(PermissiveValidator);
    let result = v
        .validate(
            "contract",
            &serde_json::json!({
                "impactLevel": "rights-impacting",
                "respondentLedgerRef": "ledger-123"
            }),
        )
        .unwrap();
    assert!(result.valid);
}

#[test]
fn layered_validator_accepts_low_impact_without_ledger() {
    let v = PolicyLayeredValidator::new(PermissiveValidator);
    let result = v
        .validate(
            "contract",
            &serde_json::json!({
                "impactLevel": "low"
            }),
        )
        .unwrap();
    assert!(result.valid);
}

#[test]
fn layered_validator_rejects_safety_impacting_without_ledger() {
    let v = PolicyLayeredValidator::new(PermissiveValidator);
    let result = v
        .validate(
            "contract",
            &serde_json::json!({
                "impactLevel": "safety-impacting"
            }),
        )
        .unwrap();
    assert!(!result.valid);
}

#[test]
fn role_based_allows_non_review_transition() {
    let ac = RoleBasedAccessControl::new();
    assert!(ac.can_transition("actor-1", "submit"));
    assert!(ac.can_transition("actor-1", "approve"));
}

#[test]
fn role_based_rejects_self_review() {
    let ac = RoleBasedAccessControl::new();
    assert!(!ac.can_transition("actor-1", "review:actor-1"));
}

#[test]
fn role_based_allows_review_of_different_author() {
    let ac = RoleBasedAccessControl::new();
    assert!(ac.can_transition("reviewer", "review:author-1"));
}

#[test]
fn role_based_delegation_permits_review() {
    let ac = RoleBasedAccessControl::new();
    ac.record_delegation("author-1", "delegate-1");
    assert!(ac.can_transition("delegate-1", "review:author-1"));
}

// ---------------------------------------------------------------------------
// WS-080: AppRuntimeConfig — `signer` and `renderer` are pluggable through
// `AppRuntime::build_with(...)`; `build()` is a delegate over default config.
// Wiring the validator/access/external/clock seams the same way needs
// upstream `Box<dyn>` blanket impls in wos-core::traits and is tracked as
// a follow-up. The fixture below proves the pluggability path for the two
// seams that today route through the config without any upstream change.
// ---------------------------------------------------------------------------

#[derive(Default)]
struct CountingSigner {
    invocations: std::sync::atomic::AtomicUsize,
}

impl ProvenanceSigner for CountingSigner {
    type Error = wos_server::runtime::signer::SignerError;

    fn sign(
        &self,
        _record: &ProvenanceRecord,
    ) -> Result<Vec<u8>, Self::Error> {
        self.invocations
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(b"COUNTED".to_vec())
    }

    fn verify(
        &self,
        _record: &ProvenanceRecord,
        _signature: &[u8],
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

#[tokio::test]
async fn app_runtime_build_with_swaps_in_custom_signer() {
    use std::sync::Arc;
    use wos_server::config::{AuthKind, ServerConfig, StorageKind};
    use wos_server::realtime;
    use wos_server::runtime::{AppRuntime, AppRuntimeConfig};
    use wos_server::services::AppServices;
    use wos_server::storage::SqliteStorage;

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
        auth: AuthKind::Mock,
        jwt_secret: "x".into(),
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
        session_sweep_enabled: true,
        signer_kind: wos_server::config::SignerKind::Noop,
    });
    let storage_handle: wos_server::storage::StorageHandle = store.clone();
    let services = Arc::new(
        AppServices::new(cfg.clone(), storage_handle.clone())
            .await
            .unwrap(),
    );
    let (_layer, io) = realtime::build_io_only();

    let counting = Arc::new(CountingSigner::default());
    let signer_ref: Arc<
        dyn ProvenanceSigner<Error = wos_server::runtime::signer::SignerError> + Send + Sync,
    > = counting.clone();
    let runtime = AppRuntime::build_with(
        storage_handle,
        services.provenance.clone(),
        services.bundle.clone(),
        io,
        AppRuntimeConfig {
            signer: signer_ref,
            renderer: Arc::new(JsonRenderer),
        },
    );

    // Sign one record through the runtime's accessor — proves the injected
    // signer is the one in flight, not the default `NoopSigner`.
    let sig = runtime.signer().sign(&stub_record()).unwrap();
    assert_eq!(sig, b"COUNTED");
    assert_eq!(
        counting
            .invocations
            .load(std::sync::atomic::Ordering::Relaxed),
        1
    );
}

#[test]
fn role_based_delegation_rejects_self_delegation() {
    let ac = RoleBasedAccessControl::new();
    let scope = DelegationScope {
        impact_levels: Vec::new(),
        case_types: Vec::new(),
        max_dollar_threshold: None,
        conditions: None,
    };
    assert!(!ac.can_delegate("a", "a", &scope));
    assert!(ac.can_delegate("a", "b", &scope));
}
