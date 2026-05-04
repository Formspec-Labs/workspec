// Rust guideline compliant 2026-05-03

//! Typed PolicyObject body fragments lifted out of the per-document
//! permissive body map once their shape is spec-pinned.
//!
//! The first inhabitant is [`RetentionPolicy`] (ADR-0083 r2), which
//! pins the closed shape attached at
//! `EvidenceRequirement.body.retentionPolicy` and at
//! `Workspace.policy.retentionPolicies[<DPV-IRI>]`. Future fragments
//! follow the same pattern: spec pin → schema `$def` → typed struct
//! here → typed accessor on the relevant document → consumer
//! migration.

use serde::{Deserialize, Serialize};

/// Terminal disposition action when retention expires (ADR-0083 §2.1).
/// `transfer` is reserved for a future revision (out of scope for v1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DisposalAction {
    Archive,
    CryptoErase,
    Redact,
    Purge,
}

/// Whether retention is bounded by a duration or open-ended.
/// `Indefinite` mode forbids `duration` (enforced by both schema and
/// `RetentionPolicy::shape_violations`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum RetentionMode {
    #[default]
    Bounded,
    Indefinite,
}

/// Event that starts the disposal clock. Vendor-specific triggers go
/// under `^x-` patternProperties (the schema preserves them; this
/// enum doesn't enumerate them).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum TriggerEvent {
    #[default]
    CaseClosure,
    LastInteraction,
    EvidenceCollection,
    OutcomeFinalization,
}

/// Typed retention policy per ADR-0083 r2.
///
/// Closed shape; `additionalProperties: false` on the schema side, but
/// the deserializer here permits unknown fields so vendor extensions
/// (`^x-` keys) and `$comment` round-trip without loss. Validation of
/// `^(\$|x-)`-only extras is the schema's job; this struct only
/// captures the spec-pinned fields plus a catch-all for extensions.
///
/// # Composition (workspace defaults)
///
/// A workspace MAY declare `Workspace.policy.retentionPolicies[<DPV-IRI>]`
/// defaults; an EvidenceRequirement that collects a DataElement of
/// that sensitivity inherits the default unless it declares its own
/// `retentionPolicy`. Override resolution is field-by-field:
///
/// - Scalar fields (`duration`, `mode`, `trigger_event`,
///   `disposal_action`, `respects_legal_hold`): EvidenceRequirement
///   value replaces workspace value if present; otherwise workspace
///   value applies.
/// - `regulatory_basis`: workspace + EvidenceRequirement values
///   **merge** (union, deduplicated by SourceCitation id).
///
/// Resolution is the consumer's responsibility today
/// (`WF-LINT-006` walks the resolved view). A future helper
/// `RetentionPolicy::resolve(workspace_default, evidence_override)`
/// may land here once consumers stabilize.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionPolicy {
    /// ISO-8601 duration string (`P7Y`, `P30D`). REQUIRED unless
    /// `mode = Indefinite`. Forbidden when `mode = Indefinite`
    /// (validated by `shape_violations`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,

    /// Default `Bounded`. When `Indefinite`, `duration` MUST be absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<RetentionMode>,

    /// Terminal disposition action. No default — every contract names
    /// its disposition. REQUIRED.
    pub disposal_action: DisposalAction,

    /// Event that starts the disposal clock. Default `CaseClosure`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger_event: Option<TriggerEvent>,

    /// Default `true`. Delegation flag: when `true`, kernel
    /// `holdType: legal-hold` per `workflow-governance.md` §7.15
    /// (1) suspends the disposal clock and (2) rejects
    /// `disposalAction` execution with the hold reference recorded
    /// in rejection provenance. When `false`, the EvidenceRequirement
    /// disclaims kernel delegation and MUST carry `regulatory_basis`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub respects_legal_hold: Option<bool>,

    /// Cited authority documenting the regulatory basis. REQUIRED on
    /// the resolved (post-merge) policy when
    /// `respects_legal_hold = false`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regulatory_basis: Option<Vec<serde_json::Value>>,

    /// Catch-all for `^x-` patternProperties + `$comment`. Schema
    /// validates these; the struct just preserves them across
    /// round-trip.
    #[serde(flatten)]
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

impl RetentionPolicy {
    /// Returns the resolved `mode`, defaulting to `Bounded` when
    /// absent.
    pub fn effective_mode(&self) -> RetentionMode {
        self.mode.unwrap_or_default()
    }

    /// Returns the resolved `respects_legal_hold`, defaulting to
    /// `true` when absent (per ADR-0083 §2.4).
    pub fn effective_respects_legal_hold(&self) -> bool {
        self.respects_legal_hold.unwrap_or(true)
    }

    /// Walk the policy's shape invariants; emit a list of human-readable
    /// violations. Mirrors the schema's allOf if/then guards so consumers
    /// (lint engine) can validate post-resolution policies that were
    /// merged in memory and never traveled through schema validation.
    pub fn shape_violations(&self) -> Vec<String> {
        let mut errs = Vec::new();
        match self.effective_mode() {
            RetentionMode::Indefinite => {
                if self.duration.is_some() {
                    errs.push("mode=indefinite forbids duration".to_string());
                }
            }
            RetentionMode::Bounded => {
                if self.duration.is_none() {
                    errs.push("mode=bounded requires duration (ISO-8601)".to_string());
                }
            }
        }
        if !self.effective_respects_legal_hold()
            && self.regulatory_basis.as_ref().is_none_or(Vec::is_empty)
        {
            errs.push(
                "respectsLegalHold=false requires non-empty regulatoryBasis[]".to_string(),
            );
        }
        errs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_minimal_bounded_policy() {
        let v = json!({"duration": "P7Y", "disposalAction": "cryptoErase"});
        let p: RetentionPolicy = serde_json::from_value(v).expect("parse");
        assert_eq!(p.duration.as_deref(), Some("P7Y"));
        assert_eq!(p.disposal_action, DisposalAction::CryptoErase);
        assert_eq!(p.effective_mode(), RetentionMode::Bounded);
        assert!(p.effective_respects_legal_hold());
        assert!(p.shape_violations().is_empty());
    }

    #[test]
    fn parses_indefinite_policy_without_duration() {
        let v = json!({"mode": "indefinite", "disposalAction": "archive"});
        let p: RetentionPolicy = serde_json::from_value(v).expect("parse");
        assert_eq!(p.effective_mode(), RetentionMode::Indefinite);
        assert!(p.duration.is_none());
        assert!(p.shape_violations().is_empty());
    }

    #[test]
    fn shape_violations_indefinite_with_duration() {
        let p = RetentionPolicy {
            duration: Some("P7Y".to_string()),
            mode: Some(RetentionMode::Indefinite),
            disposal_action: DisposalAction::Archive,
            trigger_event: None,
            respects_legal_hold: None,
            regulatory_basis: None,
            extensions: Default::default(),
        };
        let errs = p.shape_violations();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("indefinite forbids duration"));
    }

    #[test]
    fn shape_violations_bounded_without_duration() {
        let p = RetentionPolicy {
            duration: None,
            mode: None,
            disposal_action: DisposalAction::Purge,
            trigger_event: None,
            respects_legal_hold: None,
            regulatory_basis: None,
            extensions: Default::default(),
        };
        let errs = p.shape_violations();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("bounded requires duration"));
    }

    #[test]
    fn shape_violations_no_legal_hold_without_basis() {
        let p = RetentionPolicy {
            duration: Some("P7Y".to_string()),
            mode: None,
            disposal_action: DisposalAction::Purge,
            trigger_event: None,
            respects_legal_hold: Some(false),
            regulatory_basis: None,
            extensions: Default::default(),
        };
        let errs = p.shape_violations();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("respectsLegalHold=false requires"));
    }

    #[test]
    fn no_legal_hold_with_basis_passes() {
        let p = RetentionPolicy {
            duration: Some("P7Y".to_string()),
            mode: None,
            disposal_action: DisposalAction::Purge,
            trigger_event: None,
            respects_legal_hold: Some(false),
            regulatory_basis: Some(vec![json!("src-hipaa")]),
            extensions: Default::default(),
        };
        assert!(p.shape_violations().is_empty());
    }

    #[test]
    fn round_trips_with_x_extension() {
        let v = json!({
            "duration": "P7Y",
            "disposalAction": "cryptoErase",
            "x-vendor": {"region": "us-east"},
            "$comment": "tested"
        });
        let p: RetentionPolicy = serde_json::from_value(v.clone()).expect("parse");
        assert!(p.extensions.contains_key("x-vendor"));
        assert!(p.extensions.contains_key("$comment"));
        let back = serde_json::to_value(&p).expect("serialize");
        // Re-parse to confirm round-trip equivalence.
        let p2: RetentionPolicy = serde_json::from_value(back).expect("re-parse");
        assert_eq!(p, p2);
    }
}
