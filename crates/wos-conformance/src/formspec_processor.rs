// Rust guideline compliant 2026-04-14

//! Test-double `FormspecProcessor` for conformance fixtures.
//!
//! Implements the four `FormspecProcessor` methods with deterministic,
//! fixture-driven behaviour: envelope fields are checked by presence,
//! definition validation is a no-op (returns `None`), prefill is identity
//! over `case_state`, and response mapping returns the envelope `data`
//! as a single field update.

use wos_formspec_binding::FormspecProcessor;
use wos_runtime::binding::{BindingError, CaseMutationBundle};

/// Fixture-driven `FormspecProcessor` for conformance test harnesses.
///
/// Envelope validation checks for the presence of required fields.
/// Definition validation is a no-op. Prefill mirrors case state.
/// Response mapping extracts `data` into a `field_updates` map.
#[derive(Debug, Clone)]
pub struct FixtureFormspecProcessor {
    pinned_url: String,
    pinned_version: String,
}

impl FixtureFormspecProcessor {
    /// Create a processor with the given pinned definition URL and version.
    pub fn new(pinned_url: impl Into<String>, pinned_version: impl Into<String>) -> Self {
        Self {
            pinned_url: pinned_url.into(),
            pinned_version: pinned_version.into(),
        }
    }
}

impl FormspecProcessor for FixtureFormspecProcessor {
    fn validate_envelope(
        &self,
        response: &serde_json::Value,
    ) -> Result<Vec<serde_json::Value>, BindingError> {
        let mut errs = Vec::new();
        for field in &["status", "definitionUrl", "definitionVersion", "data"] {
            if response.get(*field).is_none() {
                errs.push(serde_json::json!({
                    "code": "envelope_missing_field",
                    "field": field,
                }));
            }
        }
        Ok(errs)
    }

    fn validate_definition(
        &self,
        _definition_url: &str,
        _definition_version: &str,
        _data: &serde_json::Value,
    ) -> Result<Option<Vec<serde_json::Value>>, BindingError> {
        Ok(None)
    }

    fn compute_prefill(
        &self,
        _mapping_ref: Option<&str>,
        case_state: &serde_json::Value,
    ) -> Result<Option<serde_json::Value>, BindingError> {
        Ok(Some(case_state.clone()))
    }

    fn map_response(
        &self,
        _mapping_ref: &str,
        response: &serde_json::Value,
    ) -> Result<Option<CaseMutationBundle>, BindingError> {
        let data = response.get("data").cloned().unwrap_or_default();
        let mut field_updates = serde_json::Map::new();
        field_updates.insert("mapped".to_string(), data);
        Ok(Some(CaseMutationBundle { field_updates }))
    }
}
