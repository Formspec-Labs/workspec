// Rust guideline compliant 2026-02-21

//! Typed human and rule evidence for the governance audit tiers.
//!
//! These values describe attributable decisions, not resource authority. Runtime
//! policy validates them against actual accepted artifacts and transition Facts.

use serde::{Deserialize, Serialize};

/// Rule reference with authority ranking (Governance S6.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleReference {
    /// Rule identifier.
    pub rule_id: String,

    /// Human-readable summary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Authority level for explanation ordering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_authority: Option<SourceAuthority>,

    /// Formal citation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub citation: Option<String>,
}

/// Source authority ranking (Governance S6.2).
///
/// Determines ordering in explanation assembly: statute (rank 1)
/// through guideline (rank 4). Default: policy (rank 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceAuthority {
    Statute,
    Regulation,
    Policy,
    Guideline,
}

impl SourceAuthority {
    /// Numeric rank for sorting (lower = higher authority).
    pub fn rank(self) -> u8 {
        match self {
            Self::Statute => 1,
            Self::Regulation => 2,
            Self::Policy => 3,
            Self::Guideline => 4,
        }
    }
}

/// Evidence inspected in making a decision (Governance §6.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceReference {
    /// Owning evidence category, following the canonical API vocabulary.
    pub kind: String,
    /// Path in the exact determination snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_field_path: Option<String>,
    /// URI of the retained immutable artifact or external record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    /// Attributable description of the evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// Actual criterion result recorded by the decision maker.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CriterionResult {
    /// Selected criterion identifier.
    pub criterion: String,
    /// Attributable outcome of the check.
    pub satisfied: bool,
    /// Individualized explanation of that result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Closed counterfactual polarity; unknown values are never interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CounterfactualPolarity {
    /// A controllable change that would alter the decision.
    Positive,
    /// An irrelevant factor that did not affect the decision.
    Negative,
}

/// An attributable counterfactual tied to actual decision evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Counterfactual {
    /// Whether the factor changes the outcome or did not affect it.
    pub polarity: CounterfactualPolarity,
    /// Selected factor identifier.
    pub factor: String,
    /// Individualized explanation of the effect or non-effect.
    pub effect: String,
}

/// Structured decision evidence collected in an accepted human task response.
///
/// Runtime verifies references and attribution; this records a human judgment,
/// not a claim that schema validation establishes its substantive correctness.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DecisionEvidence {
    /// Specific explanation addressed to the affected individual.
    pub reason: String,
    /// Actual cited rules applied in this decision.
    pub rules_applied: Vec<RuleReference>,
    /// Actual evidence references inspected for this decision.
    pub evidence_consulted: Vec<EvidenceReference>,
    /// Results of the selected rule's criteria.
    pub criteria_checked: Vec<CriterionResult>,
    /// Controllable factors that would change an adverse outcome.
    #[serde(default)]
    pub positive_counterfactuals: Vec<Counterfactual>,
    /// Irrelevant factors excluded from an adverse outcome.
    #[serde(default)]
    pub negative_counterfactuals: Vec<Counterfactual>,
}

/// A deterministic adverse proposal, before any claim of notice delivery.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdverseNoticeProposal {
    /// Actual original determination Facts, also the stable proposal reference.
    pub facts_record_ref: String,
    /// Exact original pre-decision case snapshot.
    pub snapshot_sha256: String,
    /// Canonical selected notification template.
    pub template_key: String,
    /// Canonical digest of the selected template object.
    pub template_sha256: String,
    /// Deterministic structured content, including the individualized explanation.
    pub machine_readable: serde_json::Value,
    /// Deterministic human text derived from the same selected content.
    pub human_readable: String,
    /// Canonical digest of machineReadable and humanReadable together.
    pub content_sha256: String,
}

/// Reasoning data emitted for one actual transition Facts record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReasoningEvidence {
    /// Original immutable Facts record interpreted by this evidence.
    pub facts_record_ref: String,
    /// Hash of the exact pre-transition case snapshot.
    pub snapshot_sha256: String,
    /// Case-specific explanation from the actual decision source.
    pub reason: String,
    /// Rules applied, retaining their actual source authority and citation.
    pub rules_applied: Vec<RuleReference>,
    /// Retained evidence locators examined by the decision maker.
    pub evidence_consulted: Vec<EvidenceReference>,
    /// Criteria actually checked and their results.
    pub criteria_checked: Vec<CriterionResult>,
    /// Accepted task artifact or finite native rule occurrence providing evidence.
    pub source_ref: String,
    /// A retained proposal is not a sent receipt or an effective decision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adverse_notice: Option<AdverseNoticeProposal>,
}

/// Counterfactual data emitted for the same actual determination snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CounterfactualEvidence {
    /// Original immutable Facts record analyzed by these counterfactuals.
    pub facts_record_ref: String,
    /// Hash of the exact pre-transition case snapshot.
    pub snapshot_sha256: String,
    /// Controllable changes tied to the selected rule and case evidence.
    pub positive_counterfactuals: Vec<Counterfactual>,
    /// Factors excluded from this actual decision.
    pub negative_counterfactuals: Vec<Counterfactual>,
    /// Same original source as the related Reasoning record.
    pub source_ref: String,
}

impl DecisionEvidence {
    /// Validates original values against the owning vocabulary before decoding.
    ///
    /// # Errors
    /// Returns a located shape refusal without exposing original answer values.
    pub fn from_json(value: &serde_json::Value) -> Result<Self, String> {
        use std::sync::OnceLock;
        static VALIDATOR: OnceLock<Result<jsonschema::Validator, String>> = OnceLock::new();
        let validator = VALIDATOR
            .get_or_init(|| {
                let owner: serde_json::Value = serde_json::from_str(include_str!(
                    "../../../schemas/api/provenance.schema.json"
                ))
                .map_err(|_| "invalid owning evidence schema".to_owned())?;
                let mut definitions = serde_json::Map::new();
                for name in [
                    "RuleReference",
                    "SourceAuthority",
                    "EvidenceReference",
                    "EvidenceKind",
                    "CriterionResult",
                    "Counterfactual",
                    "CounterfactualPolarity",
                ] {
                    definitions.insert(name.into(), owner["$defs"][name].clone());
                }
                let array = |definition: &str| {
                    serde_json::json!({
                "type":"array", "items":{"$ref":format!("#/$defs/{definition}")}})
                };
                let schema = serde_json::json!({
                "$schema":"https://json-schema.org/draft/2020-12/schema",
                "$defs":definitions,"type":"object","additionalProperties":false,
                "required":["reason","rulesApplied","evidenceConsulted","criteriaChecked"],
                "properties":{
                    "reason":{"type":"string","minLength":1},
                    "rulesApplied":array("RuleReference"),
                    "evidenceConsulted":array("EvidenceReference"),
                    "criteriaChecked":array("CriterionResult"),
                    "positiveCounterfactuals":array("Counterfactual"),
                    "negativeCounterfactuals":array("Counterfactual")
                }});
                jsonschema::options()
                    .should_validate_formats(true)
                    .build(&schema)
                    .map_err(|_| "invalid owning evidence validator".to_owned())
            })
            .as_ref()
            .map_err(Clone::clone)?;
        validator.validate(value).map_err(|error| {
            format!(
                "decision evidence schema violation at {}",
                error.instance_path()
            )
        })?;
        serde_json::from_value(value.clone())
            .map_err(|_| "decision evidence decoding refused".into())
    }
}
