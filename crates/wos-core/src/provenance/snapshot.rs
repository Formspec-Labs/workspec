// Rust guideline compliant 2026-02-21

use serde::{Deserialize, Serialize};

/// Canonical case-file snapshot captured for a determination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseFileSnapshot {
    /// Case-file value observed at determination fire time.
    pub value: serde_json::Value,

    /// JCS-style canonical JSON representation of `value`.
    pub jcs_canonical: String,

    /// SHA-256 hex digest of `jcs_canonical`.
    pub sha256: String,
}

impl CaseFileSnapshot {
    /// Create a canonical snapshot from case state.
    pub fn from_case_state(state: &serde_json::Value) -> Self {
        let jcs_canonical = serde_json_canonicalizer::to_string(state)
            .expect("serde_json::Value serializes to JCS");
        let sha256 = {
            use sha2::{Digest, Sha256};
            format!("{:x}", Sha256::digest(jcs_canonical.as_bytes()))
        };
        Self {
            value: state.clone(),
            jcs_canonical,
            sha256,
        }
    }
}
