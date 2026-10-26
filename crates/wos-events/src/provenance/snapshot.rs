// Rust guideline compliant 2026-02-21

use integrity_canonical::{DigestAlgorithm, canonical_json_bytes, digest_bytes};
use serde::{Deserialize, Serialize};

/// Canonical case-file snapshot captured for a determination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseFileSnapshot {
    /// Case-file value observed at determination fire time.
    pub value: serde_json::Value,

    /// `integrity-canonical-json-v1` JSON representation of `value`, preserving exact integers.
    pub jcs_canonical: String,

    /// SHA-256 hex digest of `jcs_canonical`.
    pub sha256: String,
}

impl CaseFileSnapshot {
    /// Create a canonical snapshot from case state.
    pub fn from_case_state(state: &serde_json::Value) -> Self {
        let canonical_bytes =
            canonical_json_bytes(state).expect("serde_json::Value serializes to canonical JSON");
        let sha256 = digest_bytes(&canonical_bytes, DigestAlgorithm::Sha256)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let jcs_canonical = String::from_utf8(canonical_bytes).expect("canonical JSON is UTF-8");
        Self {
            value: state.clone(),
            jcs_canonical,
            sha256,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_commits_exact_integer_and_plain_canonical_json_bytes() {
        let cases = [
            (
                r#"{"count":1152921504606846976}"#,
                "9c84f2ae64594b7f4b237b8bd44964b3a6fa6d1b2d7838a4f06ff4d52f095638",
            ),
            (
                r#"{"a":"line\nquote\"","z":2}"#,
                "5b1d7a23f5976d422d00552a4590a877af7c2976c6af419fc5f69f1b90412e41",
            ),
            (
                r#"{"a":1,"b":true}"#,
                "d918e8d1a9eb1f54b583326cad9950c771474b5c8ac1072bb0f2a0ad148d6de5",
            ),
        ];
        for (expected, sha256) in cases {
            let value = serde_json::from_str(expected).unwrap();
            let snapshot = CaseFileSnapshot::from_case_state(&value);
            assert_eq!(snapshot.value, value);
            assert_eq!(snapshot.jcs_canonical, expected);
            assert_eq!(snapshot.sha256, sha256);
            let wire = serde_json::to_value(snapshot).unwrap();
            assert_eq!(wire["jcsCanonical"], expected);
        }
        let unordered = serde_json::json!({"z": 2, "a": "line\nquote\""});
        assert_eq!(
            CaseFileSnapshot::from_case_state(&unordered).jcs_canonical,
            cases[1].0
        );
    }
}
