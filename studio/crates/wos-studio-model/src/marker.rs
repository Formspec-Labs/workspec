// Rust guideline compliant 2026-05-02

//! `$wosStudio*` document marker discriminator.

use serde::{Deserialize, Serialize};

/// The 14 Studio (Authoring) document markers. `wos-studio-common.schema.json`
/// is unmarked (it's a `$defs` library) and therefore has no entry here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StudioMarker {
    #[serde(rename = "$wosStudioApproval")]
    Approval,
    #[serde(rename = "$wosStudioBinding")]
    Binding,
    #[serde(rename = "$wosStudioEffectiveness")]
    Effectiveness,
    #[serde(rename = "$wosStudioIdentitySubject")]
    IdentitySubject,
    #[serde(rename = "$wosStudioMapping")]
    Mapping,
    #[serde(rename = "$wosStudioMigrationPath")]
    MigrationPath,
    #[serde(rename = "$wosStudioPolicyObject")]
    PolicyObject,
    #[serde(rename = "$wosStudioProvenance")]
    Provenance,
    #[serde(rename = "$wosStudioReadiness")]
    Readiness,
    #[serde(rename = "$wosStudioScenario")]
    Scenario,
    #[serde(rename = "$wosStudioSource")]
    Source,
    #[serde(rename = "$wosStudioTerminologyMap")]
    TerminologyMap,
    #[serde(rename = "$wosStudioWorkflowIntent")]
    WorkflowIntent,
    #[serde(rename = "$wosStudioWorkspace")]
    Workspace,
}

impl StudioMarker {
    /// The string form ("$wosStudioApproval", etc.) as it appears as a
    /// top-level key in a JSON document.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Approval => "$wosStudioApproval",
            Self::Binding => "$wosStudioBinding",
            Self::Effectiveness => "$wosStudioEffectiveness",
            Self::IdentitySubject => "$wosStudioIdentitySubject",
            Self::Mapping => "$wosStudioMapping",
            Self::MigrationPath => "$wosStudioMigrationPath",
            Self::PolicyObject => "$wosStudioPolicyObject",
            Self::Provenance => "$wosStudioProvenance",
            Self::Readiness => "$wosStudioReadiness",
            Self::Scenario => "$wosStudioScenario",
            Self::Source => "$wosStudioSource",
            Self::TerminologyMap => "$wosStudioTerminologyMap",
            Self::WorkflowIntent => "$wosStudioWorkflowIntent",
            Self::Workspace => "$wosStudioWorkspace",
        }
    }
}

/// Return the first `$wosStudio*` marker key in a JSON document, or `None`.
///
/// Mirrors the Python `classify` helper used by Studio's pytest harness so
/// Rust + Python tooling agree on which document is which.
pub fn classify(doc: &serde_json::Value) -> Option<StudioMarker> {
    let object = doc.as_object()?;
    for key in object.keys() {
        if let Ok(marker) = serde_json::from_value::<StudioMarker>(
            serde_json::Value::String(key.clone()),
        ) {
            return Some(marker);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn classify_recognizes_marker() {
        let doc = json!({"$wosStudioPolicyObject": "1.0", "id": "po-1"});
        assert_eq!(classify(&doc), Some(StudioMarker::PolicyObject));
    }

    #[test]
    fn classify_returns_none_for_unmarked() {
        let doc = json!({"id": "po-1", "version": "1.0"});
        assert_eq!(classify(&doc), None);
    }

    #[test]
    fn marker_round_trip() {
        let marker = StudioMarker::Mapping;
        let s = serde_json::to_string(&marker).expect("write");
        assert_eq!(s, "\"$wosStudioMapping\"");
        let back: StudioMarker = serde_json::from_str(&s).expect("parse");
        assert_eq!(back, StudioMarker::Mapping);
    }

    #[test]
    fn marker_as_str_matches_serde() {
        for marker in [
            StudioMarker::Approval,
            StudioMarker::PolicyObject,
            StudioMarker::Workspace,
        ] {
            let serialized = serde_json::to_string(&marker).expect("write");
            assert_eq!(serialized, format!("\"{}\"", marker.as_str()));
        }
    }
}
