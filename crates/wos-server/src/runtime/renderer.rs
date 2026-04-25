// Rust guideline compliant 2026-02-21

//! `ReportRenderer` implementations. `JsonRenderer` is the default — it
//! serialises explanation and audit payloads as pretty-printed JSON so every
//! deployment has a functional renderer from day one. Richer formatters
//! (HTML, PDF) slot in behind the same `ReportRenderer` trait when needed.

use thiserror::Error;
use wos_core::provenance::ProvenanceRecord;
use wos_core::traits::ReportRenderer;

#[derive(Debug, Error)]
pub enum RendererError {
    #[error("renderer error: {0}")]
    Other(String),
}

/// JSON report renderer. Produces pretty-printed JSON for both explanation
/// and audit payloads — the lowest-common-denominator format that every
/// consumer can parse.
#[derive(Debug, Default)]
pub struct JsonRenderer;

impl ReportRenderer for JsonRenderer {
    type Error = RendererError;

    fn render_explanation(
        &self,
        explanation: &serde_json::Value,
        _template: &str,
    ) -> Result<String, Self::Error> {
        serde_json::to_string_pretty(explanation)
            .map_err(|e| RendererError::Other(format!("explanation serialise: {e}")))
    }

    fn render_audit(
        &self,
        provenance_log: &[ProvenanceRecord],
        _format: &str,
    ) -> Result<String, Self::Error> {
        serde_json::to_string_pretty(provenance_log)
            .map_err(|e| RendererError::Other(format!("audit serialise: {e}")))
    }
}
