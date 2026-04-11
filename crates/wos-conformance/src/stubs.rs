// Rust guideline compliant 2026-04-10

//! Stub implementations of `wos_core::traits` for conformance testing.
//!
//! These stubs provide the minimum behavior needed to run conformance
//! fixtures without real infrastructure.
//!
//! **Note:** `wos_core::traits::DefaultRuntime` provides similar in-memory
//! stubs bundled with wos-core. These conformance-specific stubs exist for
//! future `Evaluator` parameterization by host interfaces. When that
//! integration lands, consolidate rather than maintaining both sets.

use std::collections::HashMap;

use wos_core::instance::CaseInstance;
use wos_core::traits::{ContractValidator, ExternalService, InstanceStore, ValidationResult};

// ── InMemoryStore ───────────────────────────────────────────────

/// In-memory instance store for conformance tests.
///
/// Stores `CaseInstance` documents in a `HashMap` keyed by instance ID.
/// All state is lost when the store is dropped.
#[derive(Debug, Default)]
pub struct InMemoryStore {
    instances: HashMap<String, CaseInstance>,
}

impl InMemoryStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of stored instances.
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }
}

/// Error type for in-memory store operations.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// Instance not found.
    #[error("instance not found: {0}")]
    NotFound(String),
}

impl InstanceStore for InMemoryStore {
    type Error = StoreError;

    fn load(&self, instance_id: &str) -> Result<CaseInstance, Self::Error> {
        self.instances
            .get(instance_id)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(instance_id.to_string()))
    }

    fn save(&mut self, instance: &CaseInstance) -> Result<(), Self::Error> {
        self.instances
            .insert(instance.instance_id.clone(), instance.clone());
        Ok(())
    }
}

// ── StubValidator ───────────────────────────────────────────────

/// Stub contract validator for conformance tests.
///
/// Returns a configurable pass/fail result for all validations.
#[derive(Debug)]
pub struct StubValidator {
    /// If `true`, all validations pass. If `false`, all fail.
    pass_all: bool,
}

impl StubValidator {
    /// Create a validator that passes all validations.
    pub fn passing() -> Self {
        Self { pass_all: true }
    }

    /// Create a validator that fails all validations.
    pub fn failing() -> Self {
        Self { pass_all: false }
    }
}

/// Error type for stub validator (never actually produced).
#[derive(Debug, thiserror::Error)]
pub enum ValidatorError {
    /// Placeholder — stub never errors, only returns pass/fail.
    #[error("stub validator error: {0}")]
    Stub(String),
}

impl ContractValidator for StubValidator {
    type Error = ValidatorError;

    fn validate(
        &self,
        contract_ref: &str,
        _data: &serde_json::Value,
    ) -> Result<ValidationResult, Self::Error> {
        Ok(ValidationResult {
            valid: self.pass_all,
            errors: if self.pass_all {
                Vec::new()
            } else {
                vec![format!("stub rejection for contract '{contract_ref}'")]
            },
        })
    }
}

// ── StubService ─────────────────────────────────────────────────

/// Stub external service for conformance tests.
///
/// Returns a configurable response for all invocations.
#[derive(Debug)]
pub struct StubService {
    /// Response returned for all invocations.
    response: serde_json::Value,
}

impl StubService {
    /// Create a stub that returns the given response for all invocations.
    pub fn with_response(response: serde_json::Value) -> Self {
        Self { response }
    }

    /// Create a stub that returns `null` for all invocations.
    pub fn null_response() -> Self {
        Self::with_response(serde_json::Value::Null)
    }
}

/// Error type for stub service (never actually produced).
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    /// Placeholder — stub never errors.
    #[error("stub service error: {0}")]
    Stub(String),
}

impl ExternalService for StubService {
    type Error = ServiceError;

    fn invoke(
        &self,
        _service_ref: &str,
        _input: &serde_json::Value,
        _idempotency_key: Option<&str>,
    ) -> Result<serde_json::Value, Self::Error> {
        Ok(self.response.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_store_save_and_load() {
        let mut store = InMemoryStore::new();
        assert!(store.is_empty());

        let instance = CaseInstance {
            instance_id: "test-001".to_string(),
            definition_url: "https://example.com/workflow".to_string(),
            definition_version: "1.0".to_string(),
            configuration: vec!["open".to_string()],
            case_state: serde_json::json!({}),
            provenance_position: 0,
            timers: Vec::new(),
            history_store: None,
            compensation_logs: None,
            status: wos_core::instance::InstanceStatus::Active,
            pending_events: Vec::new(),
            governance_state: None,
            volume_counters: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            extensions: HashMap::new(),
        };

        store.save(&instance).unwrap();
        assert_eq!(store.len(), 1);

        let loaded = store.load("test-001").unwrap();
        assert_eq!(loaded.instance_id, "test-001");
        assert_eq!(loaded.configuration, vec!["open"]);
    }

    #[test]
    fn in_memory_store_load_nonexistent_returns_error() {
        let store = InMemoryStore::new();
        let result = store.load("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn stub_validator_passing_returns_valid() {
        let validator = StubValidator::passing();
        let result = validator
            .validate("test-contract", &serde_json::json!({"field": "value"}))
            .unwrap();
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn stub_validator_failing_returns_invalid() {
        let validator = StubValidator::failing();
        let result = validator
            .validate("test-contract", &serde_json::json!({"field": "value"}))
            .unwrap();
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn stub_service_returns_configured_response() {
        let service = StubService::with_response(serde_json::json!({"status": "ok"}));
        let result = service
            .invoke("test-service", &serde_json::json!({}), None)
            .unwrap();
        assert_eq!(result, serde_json::json!({"status": "ok"}));
    }

    #[test]
    fn stub_service_null_response() {
        let service = StubService::null_response();
        let result = service
            .invoke("test-service", &serde_json::json!({}), Some("key-1"))
            .unwrap();
        assert!(result.is_null());
    }
}
