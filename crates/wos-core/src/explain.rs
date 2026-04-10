// Rust guideline compliant 2026-02-21

//! Explanation assembly algorithm (Runtime Companion S9).
//!
//! When a transition tagged `adverse-decision` fires, the processor
//! assembles a structured explanation from provenance. This module
//! implements the deterministic assembly algorithm.
//!
//! The explanation is a JSON structure, not rendered text. Rendering
//! is the host's responsibility via [`crate::traits::ReportRenderer`].

use serde::{Deserialize, Serialize};

use crate::model::governance::SourceAuthority;
use crate::provenance::ProvenanceRecord;

/// Assembled explanation for an adverse decision (Runtime S9.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Explanation {
    /// Transition that produced the adverse decision.
    pub transition_id: String,

    /// Semantic tags from the transition.
    pub determination: Vec<String>,

    /// Ordered reasoning elements (by authority rank, then chronological).
    pub reasoning: Vec<ReasoningRecord>,

    /// What the affected individual could change to alter the outcome.
    pub positive_counterfactual: Vec<CounterfactualRecord>,

    /// What did NOT affect the outcome (e.g., protected characteristics).
    pub negative_counterfactual: Vec<CounterfactualRecord>,

    /// ISO 8601 timestamp of assembly.
    pub assembled_at: String,
}

/// A reasoning record in the explanation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningRecord {
    /// Rule that was applied.
    pub rule_id: String,

    /// Authority level of the rule source.
    #[serde(default)]
    pub authority: Option<SourceAuthority>,

    /// Human-readable explanation of this reasoning step.
    pub description: String,

    /// Provenance record timestamp.
    pub timestamp: String,
}

/// A counterfactual record in the explanation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterfactualRecord {
    /// What factor is described.
    pub factor: String,

    /// Explanation of the counterfactual.
    pub description: String,

    /// Whether this is positive (could change outcome) or negative (did not affect).
    #[serde(rename = "type")]
    pub kind: CounterfactualKind,
}

/// Counterfactual type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CounterfactualKind {
    Positive,
    Negative,
}

/// Assemble an explanation from provenance records (Runtime S9.2).
///
/// Filters provenance to records related to the given transition,
/// separates reasoning and counterfactual tiers, and orders reasoning
/// by authority rank (statute > regulation > policy > guideline).
///
/// This is a stub implementation — full assembly requires provenance
/// records with tier annotations, which Phase 4 fixtures will exercise.
pub fn assemble_explanation(
    _provenance: &[ProvenanceRecord],
    transition_id: &str,
    transition_tags: &[String],
    assembled_at: &str,
) -> Explanation {
    Explanation {
        transition_id: transition_id.to_string(),
        determination: transition_tags.to_vec(),
        reasoning: Vec::new(),
        positive_counterfactual: Vec::new(),
        negative_counterfactual: Vec::new(),
        assembled_at: assembled_at.to_string(),
    }
}
