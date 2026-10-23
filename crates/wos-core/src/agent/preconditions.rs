// Rust guideline compliant 2026-02-21

//! Capability precondition evaluation (AI §3.3.1).
//!
//! A capability MAY declare a `preconditions` array of FEL expressions.
//! Before the processor invokes the capability, every expression is evaluated
//! against the current evaluation context: `caseFile`, `@agent`, `@event`, and `inputs`.
//!
//! # Semantics
//!
//! 1. All entries MUST evaluate to boolean `true` for the capability to be invocable.
//! 2. If any entry evaluates to `false`, or fails to evaluate to a boolean, the capability
//!    is skipped: the processor does NOT invoke the agent and instead records a
//!    `capabilityInvocation` record with `invocationBlocked = true` and
//!    `outcome = "preconditionNotSatisfied"`.
//! 3. Absent or empty `preconditions` (the default) means the capability is always invocable.
//! 4. Every precondition evaluation MUST produce a provenance record with
//!    `recordKind: "capabilityInvocation"`.

use std::collections::HashMap;

use fel_core::{MapEnvironment, evaluate, json_to_fel, parse, types::Value as FelValue};
use stack_common_typeid::TypeIdMinter;
use wos_events::provenance::ProvenanceRecord;

use crate::model::ai::Capability;

/// Evaluation outcome for capability preconditions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreconditionEvaluation {
    /// `true` if all preconditions evaluated to boolean `true`, or if none were declared.
    pub passed: bool,
    /// The specific expression that failed, evaluated to false, or caused an error.
    pub failed_expression: Option<String>,
    /// Evaluator or parse diagnostic messages.
    pub diagnostics: Vec<String>,
}

/// Evaluates a capability's FEL preconditions against the current context per AI §3.3.1.
///
/// Context exposes `caseFile`, `@agent`, and optionally `@event` and `inputs`/`input`.
#[must_use]
pub fn evaluate_capability_preconditions(
    capability: &Capability,
    agent_id: &str,
    case_file: &serde_json::Value,
    event: Option<&serde_json::Value>,
    inputs: Option<&serde_json::Value>,
) -> PreconditionEvaluation {
    if capability.preconditions.is_empty() {
        return PreconditionEvaluation {
            passed: true,
            failed_expression: None,
            diagnostics: Vec::new(),
        };
    }

    let mut fields = HashMap::new();
    fields.insert("caseFile".to_string(), json_to_fel(case_file));
    fields.insert(
        "@agent".to_string(),
        FelValue::String(agent_id.to_string()),
    );
    if let Some(event_val) = event {
        fields.insert("@event".to_string(), json_to_fel(event_val));
    }
    if let Some(inputs_val) = inputs {
        fields.insert("inputs".to_string(), json_to_fel(inputs_val));
        fields.insert("input".to_string(), json_to_fel(inputs_val));
    }

    let env = MapEnvironment::with_fields(fields);

    for expr in &capability.preconditions {
        let ast = match parse(expr) {
            Ok(ast) => ast,
            Err(e) => {
                return PreconditionEvaluation {
                    passed: false,
                    failed_expression: Some(expr.clone()),
                    diagnostics: vec![format!("FEL parse error: {e}")],
                };
            }
        };

        let result = evaluate(&ast, &env);
        if !result.diagnostics.is_empty() {
            return PreconditionEvaluation {
                passed: false,
                failed_expression: Some(expr.clone()),
                diagnostics: result.diagnostics.iter().map(|d| format!("{d:?}")).collect(),
            };
        }

        match result.value {
            FelValue::Boolean(true) => {
                // Precondition satisfied; proceed to next.
            }
            FelValue::Boolean(false) => {
                return PreconditionEvaluation {
                    passed: false,
                    failed_expression: Some(expr.clone()),
                    diagnostics: vec!["precondition evaluated to false".to_string()],
                };
            }
            other => {
                return PreconditionEvaluation {
                    passed: false,
                    failed_expression: Some(expr.clone()),
                    diagnostics: vec![format!(
                        "precondition must evaluate to boolean, observed {other:?}"
                    )],
                };
            }
        }
    }

    PreconditionEvaluation {
        passed: true,
        failed_expression: None,
        diagnostics: Vec::new(),
    }
}

/// Evaluates capability preconditions and emits the canonical `capabilityInvocation`
/// provenance record per AI §3.3.1.
#[must_use]
pub fn evaluate_and_emit_capability_preconditions(
    minter: &TypeIdMinter,
    capability: &Capability,
    agent_id: &str,
    case_file: &serde_json::Value,
    event: Option<&serde_json::Value>,
    inputs: Option<&serde_json::Value>,
) -> (PreconditionEvaluation, ProvenanceRecord) {
    let eval = evaluate_capability_preconditions(capability, agent_id, case_file, event, inputs);
    let record = if eval.passed {
        ProvenanceRecord::capability_precondition_permitted(minter, &capability.id, agent_id)
    } else {
        ProvenanceRecord::capability_precondition_blocked(
            minter,
            &capability.id,
            agent_id,
            eval.failed_expression.as_deref(),
        )
    };
    (eval, record)
}

#[cfg(test)]
mod tests {
    use super::*;
    use stack_common_typeid::testing::fixture_minter;

    fn sample_capability(preconditions: Vec<String>) -> Capability {
        Capability {
            id: "evaluateLoan".into(),
            description: Some("Loan eligibility evaluation".into()),
            input_contract_ref: None,
            output_contract_ref: None,
            preconditions,
            metadata: Default::default(),
        }
    }

    #[test]
    fn empty_preconditions_passes_and_emits_permitted_record() {
        let minter = fixture_minter();
        let cap = sample_capability(vec![]);
        let case_file = serde_json::json!({ "income": 50000 });

        let (eval, record) = evaluate_and_emit_capability_preconditions(
            &minter,
            &cap,
            "loan-agent",
            &case_file,
            None,
            None,
        );

        assert!(eval.passed);
        assert_eq!(record.actor_id.as_deref(), Some("loan-agent"));
        assert_eq!(record.outcome, None);
        let data = record.data.expect("data object present");
        assert_eq!(data["capabilityId"], "evaluateLoan");
        assert_eq!(data["invocationBlocked"], false);
    }

    #[test]
    fn satisfied_preconditions_passes_and_emits_permitted_record() {
        let minter = fixture_minter();
        let cap = sample_capability(vec![
            "caseFile.income >= 40000".into(),
            "caseFile.active == true".into(),
        ]);
        let case_file = serde_json::json!({
            "income": 50000,
            "active": true
        });

        let (eval, record) = evaluate_and_emit_capability_preconditions(
            &minter,
            &cap,
            "loan-agent",
            &case_file,
            None,
            None,
        );

        assert!(eval.passed);
        assert_eq!(record.outcome, None);
        let data = record.data.unwrap();
        assert_eq!(data["invocationBlocked"], false);
    }

    #[test]
    fn failed_precondition_blocks_and_emits_precondition_not_satisfied() {
        let minter = fixture_minter();
        let cap = sample_capability(vec![
            "caseFile.income >= 60000".into(),
        ]);
        let case_file = serde_json::json!({
            "income": 50000,
        });

        let (eval, record) = evaluate_and_emit_capability_preconditions(
            &minter,
            &cap,
            "loan-agent",
            &case_file,
            None,
            None,
        );

        assert!(!eval.passed);
        assert_eq!(eval.failed_expression.as_deref(), Some("caseFile.income >= 60000"));
        assert_eq!(record.outcome.as_deref(), Some("preconditionNotSatisfied"));
        let data = record.data.unwrap();
        assert_eq!(data["capabilityId"], "evaluateLoan");
        assert_eq!(data["invocationBlocked"], true);
        assert_eq!(data["failedPrecondition"], "caseFile.income >= 60000");
    }

    #[test]
    fn non_boolean_precondition_blocks_invocation() {
        let minter = fixture_minter();
        let cap = sample_capability(vec![
            "caseFile.name".into(),
        ]);
        let case_file = serde_json::json!({
            "name": "Acme Corp",
        });

        let (eval, record) = evaluate_and_emit_capability_preconditions(
            &minter,
            &cap,
            "loan-agent",
            &case_file,
            None,
            None,
        );

        assert!(!eval.passed);
        assert_eq!(record.outcome.as_deref(), Some("preconditionNotSatisfied"));
        let data = record.data.unwrap();
        assert_eq!(data["invocationBlocked"], true);
    }
}
