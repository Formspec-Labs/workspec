// Rust guideline compliant 2026-04-14

//! Canonical policy-engine decision shape, independent of vendor.
//!
//! External policy engines (OPA, Cedar, etc.) each return decisions in
//! engine-specific formats. This module provides a single canonical shape
//! and `from_*` constructors that normalize each supported format into it.
//! After the binding boundary the runtime never sees engine-specific shapes.

use serde::{Deserialize, Serialize};

/// The three possible outcomes of a policy evaluation.
///
/// `Indeterminate` MUST NOT be silently coerced to either `Allow` or `Deny`.
/// The caller is responsible for downstream handling of an indeterminate result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DecisionEffect {
    Allow,
    Deny,
    Indeterminate,
}

/// A human-readable explanation contributed by a policy rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reason {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// An obligation that must be fulfilled when a decision is rendered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Obligation {
    pub id: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

/// Canonical policy-engine decision.
///
/// All `from_*` constructors normalize engine-specific response shapes into
/// this struct. The `policy-engine` binding handler uses this as its output
/// contract regardless of the `engineType` configured on the binding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub decision: DecisionEffect,
    #[serde(default)]
    pub reasons: Vec<Reason>,
    #[serde(default)]
    pub obligations: Vec<Obligation>,
}

impl PolicyDecision {
    /// Normalize an OPA-style result into the canonical shape.
    ///
    /// OPA returns `{"result": true|false, "reasons"?: [{"code": "...", "message"?: "..."}]}`.
    /// `true` → Allow, `false` → Deny. Anything else → `None`.
    pub fn from_opa(value: &serde_json::Value) -> Option<Self> {
        let result = value.get("result")?;
        let decision = match result {
            serde_json::Value::Bool(true) => DecisionEffect::Allow,
            serde_json::Value::Bool(false) => DecisionEffect::Deny,
            _ => return None,
        };

        let reasons = parse_reasons(value.get("reasons"))?;
        let obligations = parse_obligations(value.get("obligations"))?;

        Some(Self { decision, reasons, obligations })
    }

    /// Normalize a Cedar-style result into the canonical shape.
    ///
    /// Cedar returns `{"decision": "Allow"|"Deny", "determining_policies": [...]}`.
    /// The determining policies are mapped into reasons (each policy id becomes a `code`).
    /// An unrecognized `decision` string → `None`.
    pub fn from_cedar(value: &serde_json::Value) -> Option<Self> {
        let decision_str = value.get("decision")?.as_str()?;
        let decision = match decision_str {
            "Allow" => DecisionEffect::Allow,
            "Deny" => DecisionEffect::Deny,
            _ => return None,
        };

        // Cedar's determining_policies is an array of policy-id strings.
        let reasons = if let Some(policies) = value.get("determining_policies") {
            policies
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(|id| Reason { code: id.to_string(), message: None })
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let obligations = parse_obligations(value.get("obligations"))?;

        Some(Self { decision, reasons, obligations })
    }

    /// Direct deserialization for an already-canonical decision shape.
    ///
    /// Expects `{"decision": "allow"|"deny"|"indeterminate", "reasons"?: [...], "obligations"?: [...]}`.
    /// Returns `None` if the `decision` field is missing or unrecognized.
    pub fn from_canonical(value: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(value.clone()).ok()
    }
}

/// Parse an optional array of reason objects into `Vec<Reason>`.
///
/// Returns `None` only when the value is present but is not an array,
/// allowing callers to distinguish "field absent" (treated as empty) from
/// "field present but malformed" (returns `None`).
fn parse_reasons(value: Option<&serde_json::Value>) -> Option<Vec<Reason>> {
    match value {
        None => Some(Vec::new()),
        Some(serde_json::Value::Array(arr)) => {
            let reasons: Option<Vec<Reason>> = arr
                .iter()
                .map(|item| {
                    let code = item.get("code")?.as_str()?.to_string();
                    let message = item.get("message").and_then(|v| v.as_str()).map(str::to_string);
                    Some(Reason { code, message })
                })
                .collect();
            reasons
        }
        Some(_) => None, // present but not an array — malformed
    }
}

/// Parse an optional array of obligation objects into `Vec<Obligation>`.
///
/// Returns `None` only when the value is present but is not an array.
fn parse_obligations(value: Option<&serde_json::Value>) -> Option<Vec<Obligation>> {
    match value {
        None => Some(Vec::new()),
        Some(serde_json::Value::Array(arr)) => {
            let obligations: Option<Vec<Obligation>> = arr
                .iter()
                .map(|item| {
                    let id = item.get("id")?.as_str()?.to_string();
                    let data = item.get("data").cloned().unwrap_or(serde_json::Value::Null);
                    Some(Obligation { id, data })
                })
                .collect();
            obligations
        }
        Some(_) => None, // present but not an array — malformed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── OPA adapter ─────────────────────────────────────────────────

    #[test]
    fn opa_true_result_becomes_allow() {
        let raw = json!({ "result": true });
        let decision = PolicyDecision::from_opa(&raw).expect("should parse");
        assert_eq!(decision.decision, DecisionEffect::Allow);
        assert!(decision.reasons.is_empty());
        assert!(decision.obligations.is_empty());
    }

    #[test]
    fn opa_false_result_becomes_deny() {
        let raw = json!({ "result": false, "reasons": [{ "code": "policy-001", "message": "access denied" }] });
        let decision = PolicyDecision::from_opa(&raw).expect("should parse");
        assert_eq!(decision.decision, DecisionEffect::Deny);
        assert_eq!(decision.reasons.len(), 1);
        assert_eq!(decision.reasons[0].code, "policy-001");
        assert_eq!(decision.reasons[0].message.as_deref(), Some("access denied"));
    }

    #[test]
    fn opa_non_bool_result_returns_none() {
        let raw = json!({ "result": "maybe" });
        assert!(PolicyDecision::from_opa(&raw).is_none());
    }

    #[test]
    fn opa_malformed_reasons_returns_none() {
        // reasons is present but is a string, not an array
        let raw = json!({ "result": true, "reasons": "bad" });
        assert!(PolicyDecision::from_opa(&raw).is_none());
    }

    #[test]
    fn opa_missing_result_returns_none() {
        let raw = json!({ "something_else": true });
        assert!(PolicyDecision::from_opa(&raw).is_none());
    }

    // ── Cedar adapter ────────────────────────────────────────────────

    #[test]
    fn cedar_allow_decision_parses() {
        let raw = json!({
            "decision": "Allow",
            "determining_policies": ["policy-1", "policy-2"]
        });
        let decision = PolicyDecision::from_cedar(&raw).expect("should parse");
        assert_eq!(decision.decision, DecisionEffect::Allow);
        assert_eq!(decision.reasons.len(), 2);
        assert_eq!(decision.reasons[0].code, "policy-1");
        assert_eq!(decision.reasons[1].code, "policy-2");
    }

    #[test]
    fn cedar_deny_decision_parses() {
        let raw = json!({ "decision": "Deny", "determining_policies": [] });
        let decision = PolicyDecision::from_cedar(&raw).expect("should parse");
        assert_eq!(decision.decision, DecisionEffect::Deny);
        assert!(decision.reasons.is_empty());
    }

    #[test]
    fn cedar_unrecognized_decision_returns_none() {
        let raw = json!({ "decision": "Unknown" });
        assert!(PolicyDecision::from_cedar(&raw).is_none());
    }

    #[test]
    fn cedar_missing_decision_returns_none() {
        let raw = json!({ "determining_policies": ["p1"] });
        assert!(PolicyDecision::from_cedar(&raw).is_none());
    }

    // ── Canonical pass-through ───────────────────────────────────────

    #[test]
    fn canonical_allow_parses() {
        let raw = json!({ "decision": "allow", "reasons": [], "obligations": [] });
        let decision = PolicyDecision::from_canonical(&raw).expect("should parse");
        assert_eq!(decision.decision, DecisionEffect::Allow);
    }

    #[test]
    fn canonical_deny_with_reasons_parses() {
        let raw = json!({
            "decision": "deny",
            "reasons": [{ "code": "rule-block", "message": "blocked by rule" }],
            "obligations": [{ "id": "log-event", "data": { "level": "warn" } }]
        });
        let decision = PolicyDecision::from_canonical(&raw).expect("should parse");
        assert_eq!(decision.decision, DecisionEffect::Deny);
        assert_eq!(decision.reasons.len(), 1);
        assert_eq!(decision.obligations.len(), 1);
        assert_eq!(decision.obligations[0].id, "log-event");
    }

    #[test]
    fn canonical_indeterminate_parses() {
        let raw = json!({ "decision": "indeterminate" });
        let decision = PolicyDecision::from_canonical(&raw).expect("should parse");
        assert_eq!(decision.decision, DecisionEffect::Indeterminate);
    }

    #[test]
    fn canonical_unrecognized_decision_returns_none() {
        let raw = json!({ "decision": "maybe" });
        assert!(PolicyDecision::from_canonical(&raw).is_none());
    }

    #[test]
    fn canonical_missing_decision_returns_none() {
        let raw = json!({ "reasons": [] });
        assert!(PolicyDecision::from_canonical(&raw).is_none());
    }
}
