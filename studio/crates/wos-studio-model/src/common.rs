// Rust guideline compliant 2026-05-02

//! Shared Studio types (`wos-studio-common.schema.json#/$defs`).
//!
//! Mirrors the canonical enums + helpers that every per-marker schema
//! cross-references via `$ref`. Lifecycle enums are carved one per host
//! entity so each entity's prose-defined state machine has a typed
//! representation.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// `$defs/OriginClass`. Provenance class for an approved Studio object —
/// where it came from in the policy/operations supply chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OriginClass {
    Source,
    ApprovedInterpretation,
    LocalPractice,
    Assumption,
    RuntimeObserved,
}

/// `$defs/MappingState`. Whether a Studio object projects to WOS, is
/// authoring-only, requires a controlled WOS extension, or is approved
/// without a WOS mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MappingState {
    #[serde(rename = "mapsToWos")]
    MapsToWos,
    #[serde(rename = "authoringOnly")]
    AuthoringOnly,
    #[serde(rename = "requiresSpecExtension")]
    RequiresSpecExtension,
    #[serde(rename = "unmappedButApproved")]
    UnmappedButApproved,
}

/// `$defs/PolicyObjectLifecycleState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PolicyObjectLifecycleState {
    Draft,
    Reviewed,
    Approved,
    Mapped,
    Validated,
    Published,
    Superseded,
    Deprecated,
    Demoted,
}

/// `$defs/WorkflowIntentLifecycleState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkflowIntentLifecycleState {
    #[serde(rename = "draft")]
    Draft,
    #[serde(rename = "mapped")]
    Mapped,
    #[serde(rename = "validationReady")]
    ValidationReady,
    #[serde(rename = "scenarioTested")]
    ScenarioTested,
    #[serde(rename = "approved")]
    Approved,
    #[serde(rename = "published")]
    Published,
    #[serde(rename = "deprecated")]
    Deprecated,
}

/// `$defs/ScenarioLifecycleState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScenarioLifecycleState {
    #[serde(rename = "generated")]
    Generated,
    #[serde(rename = "reviewed")]
    Reviewed,
    #[serde(rename = "passing")]
    Passing,
    #[serde(rename = "failing")]
    Failing,
    #[serde(rename = "acceptedAsKnownGap")]
    AcceptedAsKnownGap,
    #[serde(rename = "regression")]
    Regression,
}

/// `$defs/SourceVersionLifecycleState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceVersionLifecycleState {
    Ingested,
    Parsed,
    Indexed,
    Classified,
    Approved,
    Superseded,
}

/// `$defs/ExtractedClaimReviewState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExtractedClaimReviewState {
    #[serde(rename = "candidate")]
    Candidate,
    #[serde(rename = "normalized")]
    Normalized,
    #[serde(rename = "needsReview")]
    NeedsReview,
    #[serde(rename = "approved")]
    Approved,
    #[serde(rename = "rejected")]
    Rejected,
    #[serde(rename = "merged")]
    Merged,
    #[serde(rename = "split")]
    Split,
}

/// `$defs/AuthorityGrantApplied`. Audit-trail block recording which
/// authority grant authorized a state transition or operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorityGrantApplied {
    /// Reference to the granted authority (workspace-scoped).
    pub grant_id: String,

    /// When the grant was exercised.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_at: Option<String>,

    /// Free-form `$`/`x-` extension keys.
    #[serde(flatten)]
    pub extensions: IndexMap<String, serde_json::Value>,
}

/// `$defs/Iri`. An IRI / IRI-reference string. Intentionally typed as a
/// newtype so call sites can distinguish from arbitrary strings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Iri(pub String);

impl Iri {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for Iri {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Iri {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_class_round_trips() {
        let v: OriginClass =
            serde_json::from_str("\"approved-interpretation\"").expect("parse");
        assert_eq!(v, OriginClass::ApprovedInterpretation);
        assert_eq!(
            serde_json::to_string(&v).expect("write"),
            "\"approved-interpretation\""
        );
    }

    #[test]
    fn mapping_state_round_trips() {
        let v: MappingState =
            serde_json::from_str("\"mapsToWos\"").expect("parse");
        assert_eq!(v, MappingState::MapsToWos);
    }

    #[test]
    fn iri_transparent() {
        let iri = Iri::from("https://example.com/x");
        let s = serde_json::to_string(&iri).expect("write");
        assert_eq!(s, "\"https://example.com/x\"");
    }
}
