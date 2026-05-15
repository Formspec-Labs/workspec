// Rust guideline compliant 2026-04-11

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
use wos_events::provenance::ProvenanceRecord;

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
/// The algorithm is deterministic: two conformant processors MUST produce
/// the same explanation structure from the same provenance log.
///
/// # Provenance Record Format
///
/// Records are expected to carry tier and relation data in their `data` field:
///
/// - Reasoning: `{ "tier": "reasoning", "relatedTransition": "<id>", "ruleId": "...", "authority": "...", "description": "...", "timestamp": "..." }`
/// - Counterfactual: `{ "tier": "counterfactual", "relatedTransition": "<id>", "type": "positive"|"negative", "factor": "...", "description": "..." }`
///
/// # Spec Reference
///
/// Runtime Companion S9.2 (Assembly Algorithm), S9.3 (Authority Ranking), S9.4 (Explanation Structure).
pub fn assemble_explanation(
    provenance: &[ProvenanceRecord],
    transition_id: &str,
    transition_tags: &[String],
    assembled_at: &str,
) -> Explanation {
    let mut reasoning: Vec<ReasoningRecord> = Vec::new();
    let mut positive_counterfactual: Vec<CounterfactualRecord> = Vec::new();
    let mut negative_counterfactual: Vec<CounterfactualRecord> = Vec::new();

    for record in provenance {
        let Some(data) = &record.data else {
            continue;
        };

        // Check if this record is related to our transition.
        let related_transition = data
            .get("relatedTransition")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if related_transition != transition_id {
            continue;
        }

        let tier = data.get("tier").and_then(|v| v.as_str()).unwrap_or("");

        match tier {
            "reasoning" => {
                let rule_id = data
                    .get("ruleId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let authority_str = data.get("authority").and_then(|v| v.as_str()).unwrap_or("");

                let authority = parse_authority(authority_str);

                let description = data
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let timestamp = data
                    .get("timestamp")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                reasoning.push(ReasoningRecord {
                    rule_id,
                    authority,
                    description,
                    timestamp,
                });
            }
            "counterfactual" => {
                let cf_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("");

                let factor = data
                    .get("factor")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let description = data
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let kind = match cf_type {
                    "positive" => CounterfactualKind::Positive,
                    _ => CounterfactualKind::Negative,
                };

                let record = CounterfactualRecord {
                    factor,
                    description,
                    kind,
                };

                match kind {
                    CounterfactualKind::Positive => positive_counterfactual.push(record),
                    CounterfactualKind::Negative => negative_counterfactual.push(record),
                }
            }
            _ => {
                // Not a reasoning or counterfactual record — skip.
            }
        }
    }

    // Step 4: Sort reasoning by authority rank (lower = higher authority),
    // then chronologically within the same authority level (Runtime S9.3).
    reasoning.sort_by(|a, b| {
        let rank_a = authority_rank(a.authority);
        let rank_b = authority_rank(b.authority);
        rank_a
            .cmp(&rank_b)
            .then_with(|| a.timestamp.cmp(&b.timestamp))
    });

    Explanation {
        transition_id: transition_id.to_string(),
        determination: transition_tags.to_vec(),
        reasoning,
        positive_counterfactual,
        negative_counterfactual,
        assembled_at: assembled_at.to_string(),
    }
}

/// Parse an authority string into a `SourceAuthority` option.
///
/// When authority is not specified, it defaults to `None` (which the
/// ranking function treats as `policy`, rank 3, per Runtime S9.3).
fn parse_authority(s: &str) -> Option<SourceAuthority> {
    match s {
        "statute" => Some(SourceAuthority::Statute),
        "regulation" => Some(SourceAuthority::Regulation),
        "policy" => Some(SourceAuthority::Policy),
        "guideline" => Some(SourceAuthority::Guideline),
        _ => None,
    }
}

/// Numeric rank for authority sorting (Runtime S9.3).
///
/// Lower rank = higher authority. When authority is `None`, defaults
/// to rank 3 (policy), per the spec: "When an authority type is not
/// specified on a reasoning record, it defaults to policy (rank 3)."
fn authority_rank(authority: Option<SourceAuthority>) -> u8 {
    match authority {
        Some(a) => a.rank(),
        None => 3, // Default: policy rank.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wos_events::provenance::{ProvenanceKind, ProvenanceRecord};

    fn make_reasoning_record(
        transition_id: &str,
        rule_id: &str,
        authority: &str,
        description: &str,
        timestamp: &str,
    ) -> ProvenanceRecord {
        ProvenanceRecord {
            id: ProvenanceRecord::mint_id(),
            record_kind: ProvenanceKind::ActionExecuted,
            timestamp: String::new(),
            actor_id: None,
            from_state: None,
            to_state: None,
            event: None,
            data: Some(serde_json::json!({
                "tier": "reasoning",
                "relatedTransition": transition_id,
                "ruleId": rule_id,
                "authority": authority,
                "description": description,
                "timestamp": timestamp,
            })),
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
            canonical_event_hash: None,
            transition_tags: Vec::new(),
            case_file_snapshot: None,
            outcome: None,
        }
    }

    fn make_counterfactual_record(
        transition_id: &str,
        cf_type: &str,
        factor: &str,
        description: &str,
    ) -> ProvenanceRecord {
        ProvenanceRecord {
            id: ProvenanceRecord::mint_id(),
            record_kind: ProvenanceKind::ActionExecuted,
            timestamp: String::new(),
            actor_id: None,
            from_state: None,
            to_state: None,
            event: None,
            data: Some(serde_json::json!({
                "tier": "counterfactual",
                "relatedTransition": transition_id,
                "type": cf_type,
                "factor": factor,
                "description": description,
            })),
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
            canonical_event_hash: None,
            transition_tags: Vec::new(),
            case_file_snapshot: None,
            outcome: None,
        }
    }

    #[test]
    fn assembles_empty_explanation_when_no_matching_records() {
        let provenance = vec![make_reasoning_record(
            "other-transition",
            "R1",
            "statute",
            "Irrelevant",
            "2026-01-01T00:00:00Z",
        )];

        let explanation = assemble_explanation(
            &provenance,
            "my-transition",
            &["adverse-decision".to_string(), "denial".to_string()],
            "2026-04-11T10:00:00Z",
        );

        assert_eq!(explanation.transition_id, "my-transition");
        assert_eq!(
            explanation.determination,
            vec!["adverse-decision", "denial"]
        );
        assert!(explanation.reasoning.is_empty());
        assert!(explanation.positive_counterfactual.is_empty());
        assert!(explanation.negative_counterfactual.is_empty());
        assert_eq!(explanation.assembled_at, "2026-04-11T10:00:00Z");
    }

    #[test]
    fn orders_reasoning_by_authority_rank_then_timestamp() {
        let provenance = vec![
            make_reasoning_record(
                "t1",
                "R-POLICY",
                "policy",
                "Policy check failed",
                "2026-01-01T12:00:00Z",
            ),
            make_reasoning_record(
                "t1",
                "R-STATUTE",
                "statute",
                "Statutory requirement not met",
                "2026-01-01T11:00:00Z",
            ),
            make_reasoning_record(
                "t1",
                "R-REG",
                "regulation",
                "Regulatory threshold exceeded",
                "2026-01-01T10:00:00Z",
            ),
            make_reasoning_record(
                "t1",
                "R-GUIDE",
                "guideline",
                "Best practice not followed",
                "2026-01-01T09:00:00Z",
            ),
        ];

        let explanation = assemble_explanation(
            &provenance,
            "t1",
            &["adverse-decision".to_string()],
            "2026-04-11T10:00:00Z",
        );

        assert_eq!(explanation.reasoning.len(), 4);
        // Statute (rank 1) first.
        assert_eq!(explanation.reasoning[0].rule_id, "R-STATUTE");
        // Regulation (rank 2) second.
        assert_eq!(explanation.reasoning[1].rule_id, "R-REG");
        // Policy (rank 3) third.
        assert_eq!(explanation.reasoning[2].rule_id, "R-POLICY");
        // Guideline (rank 4) last.
        assert_eq!(explanation.reasoning[3].rule_id, "R-GUIDE");
    }

    #[test]
    fn chronological_within_same_authority() {
        let provenance = vec![
            make_reasoning_record(
                "t1",
                "R-2",
                "regulation",
                "Second reg check",
                "2026-01-02T00:00:00Z",
            ),
            make_reasoning_record(
                "t1",
                "R-1",
                "regulation",
                "First reg check",
                "2026-01-01T00:00:00Z",
            ),
        ];

        let explanation = assemble_explanation(
            &provenance,
            "t1",
            &["adverse-decision".to_string()],
            "2026-04-11T10:00:00Z",
        );

        assert_eq!(explanation.reasoning.len(), 2);
        assert_eq!(explanation.reasoning[0].rule_id, "R-1"); // Earlier timestamp.
        assert_eq!(explanation.reasoning[1].rule_id, "R-2");
    }

    #[test]
    fn unspecified_authority_defaults_to_policy_rank() {
        let provenance = vec![
            make_reasoning_record(
                "t1",
                "R-NO-AUTH",
                "",
                "No authority specified",
                "2026-01-01T10:00:00Z",
            ),
            make_reasoning_record(
                "t1",
                "R-STATUTE",
                "statute",
                "Statutory",
                "2026-01-01T11:00:00Z",
            ),
            make_reasoning_record(
                "t1",
                "R-GUIDELINE",
                "guideline",
                "Guideline",
                "2026-01-01T09:00:00Z",
            ),
        ];

        let explanation = assemble_explanation(
            &provenance,
            "t1",
            &["adverse-decision".to_string()],
            "2026-04-11T10:00:00Z",
        );

        // Statute (1), then no-auth (treated as 3 = policy), then guideline (4).
        assert_eq!(explanation.reasoning[0].rule_id, "R-STATUTE");
        assert_eq!(explanation.reasoning[1].rule_id, "R-NO-AUTH");
        assert_eq!(explanation.reasoning[2].rule_id, "R-GUIDELINE");
    }

    #[test]
    fn separates_positive_and_negative_counterfactuals() {
        let provenance = vec![
            make_counterfactual_record(
                "t1",
                "positive",
                "income",
                "Increasing income above $30k would qualify",
            ),
            make_counterfactual_record(
                "t1",
                "negative",
                "race",
                "Race did not affect the determination",
            ),
            make_counterfactual_record(
                "t1",
                "positive",
                "residency",
                "Establishing 6-month residency would qualify",
            ),
            make_counterfactual_record(
                "t1",
                "negative",
                "gender",
                "Gender did not affect the determination",
            ),
        ];

        let explanation = assemble_explanation(
            &provenance,
            "t1",
            &["adverse-decision".to_string()],
            "2026-04-11T10:00:00Z",
        );

        assert_eq!(explanation.positive_counterfactual.len(), 2);
        assert_eq!(explanation.negative_counterfactual.len(), 2);

        assert_eq!(explanation.positive_counterfactual[0].factor, "income");
        assert_eq!(explanation.positive_counterfactual[1].factor, "residency");
        assert_eq!(explanation.negative_counterfactual[0].factor, "race");
        assert_eq!(explanation.negative_counterfactual[1].factor, "gender");
    }

    #[test]
    fn full_explanation_assembly() {
        let provenance = vec![
            // Reasoning records (mixed order).
            make_reasoning_record(
                "deny-1",
                "42-USC-1396a",
                "statute",
                "Income exceeds 138% FPL per 42 USC 1396a",
                "2026-03-15T09:00:00Z",
            ),
            make_reasoning_record(
                "deny-1",
                "STATE-REG-2024-01",
                "regulation",
                "State income limit set at $2,500/month",
                "2026-03-15T09:01:00Z",
            ),
            make_reasoning_record(
                "deny-1",
                "AGENCY-POLICY-7",
                "policy",
                "No waiver available for income excess",
                "2026-03-15T09:02:00Z",
            ),
            // Counterfactual records.
            make_counterfactual_record(
                "deny-1",
                "positive",
                "income",
                "Reducing monthly income to $2,500 or below would qualify",
            ),
            make_counterfactual_record(
                "deny-1",
                "negative",
                "disability_status",
                "Disability status did not affect the income determination",
            ),
            // Unrelated record (different transition).
            make_reasoning_record(
                "other-transition",
                "R-X",
                "statute",
                "Unrelated",
                "2026-01-01T00:00:00Z",
            ),
        ];

        let explanation = assemble_explanation(
            &provenance,
            "deny-1",
            &["adverse-decision".to_string(), "denial".to_string()],
            "2026-04-11T10:00:00Z",
        );

        assert_eq!(explanation.transition_id, "deny-1");
        assert_eq!(
            explanation.determination,
            vec!["adverse-decision", "denial"]
        );
        assert_eq!(explanation.assembled_at, "2026-04-11T10:00:00Z");

        // Reasoning: statute first, then regulation, then policy.
        assert_eq!(explanation.reasoning.len(), 3);
        assert_eq!(explanation.reasoning[0].rule_id, "42-USC-1396a");
        assert_eq!(
            explanation.reasoning[0].authority,
            Some(SourceAuthority::Statute)
        );
        assert_eq!(explanation.reasoning[1].rule_id, "STATE-REG-2024-01");
        assert_eq!(
            explanation.reasoning[1].authority,
            Some(SourceAuthority::Regulation)
        );
        assert_eq!(explanation.reasoning[2].rule_id, "AGENCY-POLICY-7");
        assert_eq!(
            explanation.reasoning[2].authority,
            Some(SourceAuthority::Policy)
        );

        // Counterfactuals.
        assert_eq!(explanation.positive_counterfactual.len(), 1);
        assert_eq!(explanation.positive_counterfactual[0].factor, "income");
        assert_eq!(explanation.negative_counterfactual.len(), 1);
        assert_eq!(
            explanation.negative_counterfactual[0].factor,
            "disability_status"
        );
    }
}
