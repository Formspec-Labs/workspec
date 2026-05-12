// Rust guideline compliant 2026-05-01

//! Shared kernel + formspec binding for Restate WS-094 durable drain (ADR R-4.1).
//!
//! Exposes the same `signature-runtime.json` + minimal `formspec` adapter pairing
//! used by `wos-runtime` unit tests, without pulling `#[cfg(test)]` symbols.

use std::sync::{Arc, Mutex};

use crate::binding::{
    BindingError, BindingRegistry, CaseMutationBundle, ContractBindingAdapter, PreparedTask,
    SubmissionValidation,
};
use crate::intake::IntakeAcceptanceRegistry;
use crate::runtime::{SignatureProfileDocument, SystemClock, WosRuntime};
use crate::store::{InMemoryStore, RuntimeRecord, RuntimeStore, StoreError};
use wos_core::instance::{ActiveTask, ValidationOutcome};
use wos_core::model::kernel::KernelDocument;
use wos_core::traits::{DocumentResolver, ExternalService};

/// In-memory store behind `Arc<Mutex<_>>` for embedding [`WosRuntime`] in Restate handlers.
#[derive(Clone)]
pub struct SharedInMemoryStore(pub Arc<Mutex<InMemoryStore>>);

impl Default for SharedInMemoryStore {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(InMemoryStore::new())))
    }
}

impl RuntimeStore for SharedInMemoryStore {
    fn create_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
        self.0
            .lock()
            .expect("store mutex poisoned")
            .create_record(record)
    }

    fn load_record(&self, process_id: &str) -> Result<RuntimeRecord, StoreError> {
        self.0
            .lock()
            .expect("store mutex poisoned")
            .load_record(process_id)
    }

    fn load_record_by_case_ledger_id(
        &self,
        case_ledger_id: &str,
    ) -> Result<RuntimeRecord, StoreError> {
        self.0
            .lock()
            .expect("store mutex poisoned")
            .load_record_by_case_ledger_id(case_ledger_id)
    }

    fn save_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
        self.0
            .lock()
            .expect("store mutex poisoned")
            .save_record(record)
    }

    fn create_intake_record(
        &mut self,
        record: crate::store::IntakeRecord,
    ) -> Result<(), StoreError> {
        self.0
            .lock()
            .expect("store mutex poisoned")
            .create_intake_record(record)
    }

    fn load_intake_record(
        &self,
        binding: &str,
        intake_id: &str,
    ) -> Result<crate::store::IntakeRecord, StoreError> {
        self.0
            .lock()
            .expect("store mutex poisoned")
            .load_intake_record(binding, intake_id)
    }

    fn save_intake_record(&mut self, record: crate::store::IntakeRecord) -> Result<(), StoreError> {
        self.0
            .lock()
            .expect("store mutex poisoned")
            .save_intake_record(record)
    }
}

/// Minimal `formspec` adapter matching `wos-runtime` test `TestAdapter` (signature fixtures).
#[derive(Debug, Default, Clone, Copy)]
pub struct MinimalFixtureFormspecAdapter;

impl ContractBindingAdapter for MinimalFixtureFormspecAdapter {
    fn binding(&self) -> &'static str {
        "formspec"
    }

    fn prepare_task(
        &self,
        _task: &ActiveTask,
        case_state: &serde_json::Value,
    ) -> Result<PreparedTask, BindingError> {
        Ok(PreparedTask {
            prefill_data: Some(serde_json::json!({
                "approved": case_state
                    .get("approved")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null)
            })),
        })
    }

    fn validate_submission(
        &self,
        task: &ActiveTask,
        response: &serde_json::Value,
    ) -> Result<SubmissionValidation, BindingError> {
        let pin_match = response
            .get("definitionUrl")
            .and_then(serde_json::Value::as_str)
            == task.definition_url.as_deref()
            && response
                .get("definitionVersion")
                .and_then(serde_json::Value::as_str)
                == task.definition_version.as_deref();
        let valid = response
            .get("data")
            .and_then(|data| data.get("approved"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        Ok(SubmissionValidation {
            validation_outcome: ValidationOutcome {
                envelope_valid: true,
                pin_match,
                definition_valid: valid,
                errors: if valid && pin_match {
                    Vec::new()
                } else {
                    vec![serde_json::json!({ "code": "invalid" })]
                },
                validation_results: None,
            },
        })
    }

    fn compute_case_mutation(
        &self,
        task: &ActiveTask,
        response: &serde_json::Value,
    ) -> Result<Option<CaseMutationBundle>, BindingError> {
        if task.response_mapping_ref.is_none() {
            return Ok(None);
        }
        let mut field_updates = serde_json::Map::new();
        field_updates.insert("decision".to_string(), response["data"]["approved"].clone());
        Ok(Some(CaseMutationBundle { field_updates }))
    }
}

/// Registers [`MinimalFixtureFormspecAdapter`] under the `formspec` binding name.
pub fn restate_signature_fixture_bindings() -> BindingRegistry {
    let mut bindings = BindingRegistry::new();
    bindings.register(MinimalFixtureFormspecAdapter);
    bindings
}

/// Loads [`fixtures/kernel/signature-runtime.json`](../../fixtures/kernel/signature-runtime.json).
pub fn signature_runtime_fixture_kernel() -> KernelDocument {
    const KERNEL_JSON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/kernel/signature-runtime.json"
    ));
    serde_json::from_str(KERNEL_JSON).expect("signature-runtime fixture kernel parses")
}

/// Loads `fixtures/profiles/signature-runtime-sequential.json` (SIG-013 harness).
pub fn signature_runtime_fixture_profile() -> SignatureProfileDocument {
    const PROFILE_JSON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/profiles/signature-runtime-sequential.json"
    ));
    let profile_root: serde_json::Value =
        serde_json::from_str(PROFILE_JSON).expect("signature profile fixture parses");
    let sig_block = profile_root
        .get("signature")
        .cloned()
        .expect("profile fixture has top-level signature block");
    serde_json::from_value(sig_block).expect("signature block parses as SignatureProfileDocument")
}

/// Reference [`WosRuntime`] wired for `signature-runtime.json` + sequential signature profile.
pub fn restate_signature_fixture_runtime(store: SharedInMemoryStore) -> WosRuntime {
    let profile = signature_runtime_fixture_profile();
    WosRuntime::new(
        store,
        SignatureFixtureResolver::new(),
        wos_core::traits::DefaultRuntime::new(),
        wos_core::traits::DefaultRuntime::new(),
        NullExternalService,
        wos_core::traits::DefaultRuntime::new(),
        SystemClock,
        restate_signature_fixture_bindings(),
    )
    .with_signature_profile("signatureProfile", profile)
    .with_intake_acceptors(IntakeAcceptanceRegistry::new())
}

/// Resolver that only serves [`signature_runtime_fixture_kernel`].
#[derive(Clone)]
pub struct SignatureFixtureResolver {
    kernel: KernelDocument,
}

impl SignatureFixtureResolver {
    #[must_use]
    pub fn new() -> Self {
        Self {
            kernel: signature_runtime_fixture_kernel(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("fixture resolver: {0}")]
pub struct FixtureResolverError(String);

impl DocumentResolver for SignatureFixtureResolver {
    type Error = FixtureResolverError;

    fn resolve_kernel(&self, url: &str, version: &str) -> Result<KernelDocument, Self::Error> {
        let expected_url = self.kernel.url.clone().unwrap_or_default();
        let expected_version = self.kernel.version.clone().unwrap_or_default();
        if url == expected_url.as_str() && version == expected_version.as_str() {
            Ok(self.kernel.clone())
        } else {
            Err(FixtureResolverError(format!(
                "no kernel for {url}@{version} (fixture only resolves {expected_url}@{expected_version})"
            )))
        }
    }

    fn resolve_governance(
        &self,
        _url: &str,
        _version: &str,
    ) -> Result<wos_core::model::governance::GovernanceDocument, Self::Error> {
        Err(FixtureResolverError(
            "governance not in fixture resolver".into(),
        ))
    }

    fn resolve_sidecar(
        &self,
        _url: &str,
        _anchor_date: Option<&str>,
    ) -> Result<serde_json::Value, Self::Error> {
        Err(FixtureResolverError(
            "sidecar not in fixture resolver".into(),
        ))
    }
}

/// No-op external service for fixture / Restate drains that never invoke remote HTTP.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct NullExternalService;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub(crate) struct NullServiceError(String);

impl ExternalService for NullExternalService {
    type Error = NullServiceError;

    fn invoke(
        &self,
        _service_ref: &str,
        _input: &serde_json::Value,
        _idempotency_key: Option<&str>,
    ) -> Result<serde_json::Value, Self::Error> {
        Ok(serde_json::Value::Null)
    }
}
