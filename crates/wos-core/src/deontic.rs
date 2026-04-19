// Rust guideline compliant 2026-04-11

//! Deontic constraint evaluation for agent outputs (AI S4).
//!
//! Evaluates permissions, prohibitions, obligations, and rights against
//! agent output using FEL expressions. Produces provenance records for
//! violations and resolution of conflicting enforcement actions.
//!
//! Evaluation order per AI S4.6:
//! 1. Permissions (bounds check)
//! 2. Prohibitions (condition check)
//! 3. Obligations (requirement check)
//! 4. Confidence floor
//! 5. Volume constraints
//! 6. Review sampling

use std::collections::HashMap;

use fel_core::{MapEnvironment, evaluate, json_to_fel, parse, types::FelValue};

use crate::model::ai::{AIIntegrationDocument, DeonticConstraints, NullBehavior, ViolationAction};
use crate::model::kernel::ImpactLevel;
use crate::provenance::{ProvenanceKind, ProvenanceRecord};

/// Result of evaluating all deontic constraints for a single agent invocation.
#[derive(Debug, Clone)]
pub struct DeonticResult {
    /// Provenance records generated during evaluation.
    pub provenance: Vec<ProvenanceRecord>,

    /// The effective enforcement action (most restrictive).
    pub effective_action: Option<ViolationAction>,
}

/// Evaluate deontic constraints against an agent's output.
///
/// Processes the constraint hierarchy at three composition levels:
/// workflow-level, action-site-level, and agent-level (AI S4.7).
/// Returns provenance records documenting the evaluation.
pub fn evaluate_deontic_constraints(
    ai_doc: &AIIntegrationDocument,
    agent_id: &str,
    output: &serde_json::Value,
    case_state: &HashMap<String, serde_json::Value>,
    impact_level: &ImpactLevel,
    bypass: Option<&str>,
    escalation_active: bool,
    invocation_source: Option<&str>,
) -> DeonticResult {
    let mut provenance = Vec::new();
    let mut violations: Vec<(String, ViolationAction)> = Vec::new();

    let agent = ai_doc.agents.iter().find(|a| a.id == agent_id);

    // Record evaluation order (AI S4.6).
    provenance.push(ProvenanceRecord {
        record_kind: ProvenanceKind::DeonticEvaluation,
        timestamp: String::new(),
        actor_id: None,
        from_state: None,
        to_state: None,
        event: None,
        data: Some(serde_json::json!({
            "order": ["permissions", "prohibitions", "obligations", "confidence", "volume", "sampling"]
        })),
        audit_layer: None,
        actor_type: None,
        lifecycle_state: None,
        definition_version: None,
        inputs: Vec::new(),
        outputs: Vec::new(),
        input_digest: None,
        output_digest: None,
        transition_tags: Vec::new(),
        case_file_snapshot: None,
    });

    // Evaluate at three composition levels (AI S4.7).
    // Track violation counts per level to determine cross-level resolution.
    let mut agent_violation_count = 0;
    let mut workflow_violation_count = 0;

    // Level 1: Agent-level constraints
    if let Some(agent_decl) = agent {
        if let Some(ref constraints) = agent_decl.deontic_constraints {
            let before = violations.len();
            evaluate_constraint_set(
                constraints,
                output,
                case_state,
                impact_level,
                bypass,
                escalation_active,
                invocation_source,
                &mut provenance,
                &mut violations,
            );
            agent_violation_count = violations.len() - before;
        }
    }

    // Level 2: Action-site-level (capability-specific constraints, if any)
    // Currently defers to agent-level constraints.

    // Level 3: Workflow-level constraints
    if let Some(ref constraints) = ai_doc.deontic_constraints {
        let before = violations.len();
        evaluate_constraint_set(
            constraints,
            output,
            case_state,
            impact_level,
            bypass,
            escalation_active,
            invocation_source,
            &mut provenance,
            &mut violations,
        );
        workflow_violation_count = violations.len() - before;
    }

    // Record evaluation at all three composition levels (AI S4.7).
    for level in &["agent", "action-site", "workflow"] {
        provenance.push(ProvenanceRecord {
            record_kind: ProvenanceKind::DeonticEvaluation,
            timestamp: String::new(),
            actor_id: None,
            from_state: None,
            to_state: None,
            event: None,
            data: Some(serde_json::json!({ "level": level })),
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
            transition_tags: Vec::new(),
            case_file_snapshot: None,
        });
    }

    // Check rights (AI S4.5).
    // Rights entitlements are data references — the value must be present
    // and non-null for the right to be satisfied (AI S4.5). If the
    // entitlement evaluates to null or false, the right is violated.
    // Rights violations are NEVER attributed to the agent.
    if let Some(agent_decl) = agent {
        if let Some(ref constraints) = agent_decl.deontic_constraints {
            for right in &constraints.rights {
                let entitled_expr = &right.entitlement;
                let result = evaluate_fel_expression(entitled_expr, output, case_state);
                if matches!(result, FelResult::False | FelResult::Null) {
                    provenance.push(ProvenanceRecord {
                        record_kind: ProvenanceKind::RightsViolation,
                        timestamp: String::new(),
                        actor_id: None,
                        from_state: None,
                        to_state: None,
                        event: None,
                        data: Some(serde_json::json!({
                            "rightId": right.id,
                            "attributedToAgent": false,
                        })),
                        audit_layer: None,
                        actor_type: None,
                        lifecycle_state: None,
                        definition_version: None,
                        inputs: Vec::new(),
                        outputs: Vec::new(),
                        input_digest: None,
                        output_digest: None,
                        transition_tags: Vec::new(),
                        case_file_snapshot: None,
                    });
                }
            }
        }
    }

    // Consistency check (AI S4.7): detect contradictions between output and case state.
    check_consistency(output, case_state, &mut provenance);

    // Resolve effective action: most restrictive wins (AI S4.6).
    let effective_action = resolve_most_restrictive(&violations);

    if violations.len() > 1 {
        if let Some(action) = effective_action {
            // Use "cross-level-most-restrictive" when violations came from both
            // agent-level and workflow-level constraints. Otherwise "most-restrictive".
            let reason = if agent_violation_count > 0 && workflow_violation_count > 0 {
                "cross-level-most-restrictive"
            } else {
                "most-restrictive"
            };
            provenance.push(ProvenanceRecord {
                record_kind: ProvenanceKind::DeonticResolution,
                timestamp: String::new(),
                actor_id: None,
                from_state: None,
                to_state: None,
                event: None,
                data: Some(serde_json::json!({
                    "effectiveAction": violation_action_str(action),
                    "reason": reason,
                })),
                audit_layer: None,
                actor_type: None,
                lifecycle_state: None,
                definition_version: None,
                inputs: Vec::new(),
                outputs: Vec::new(),
                input_digest: None,
                output_digest: None,
                transition_tags: Vec::new(),
                case_file_snapshot: None,
            });
        }
    }

    DeonticResult {
        provenance,
        effective_action,
    }
}

/// Evaluate a single constraint set (permissions + prohibitions + obligations).
#[expect(
    clippy::too_many_arguments,
    reason = "constraint evaluation requires context from multiple sources"
)]
fn evaluate_constraint_set(
    constraints: &DeonticConstraints,
    output: &serde_json::Value,
    case_state: &HashMap<String, serde_json::Value>,
    impact_level: &ImpactLevel,
    bypass: Option<&str>,
    escalation_active: bool,
    invocation_source: Option<&str>,
    provenance: &mut Vec<ProvenanceRecord>,
    violations: &mut Vec<(String, ViolationAction)>,
) {
    // Emergency bypass: when explicitly provided in event data, bypasses
    // all constraints with provenance recording (AI S4.7).
    // Every bypassed constraint — permissions, prohibitions, and obligations —
    // gets an individual provenance record for full audit trail.
    if let Some(rationale) = bypass {
        for perm in &constraints.permissions {
            provenance.push(ProvenanceRecord {
                record_kind: ProvenanceKind::DeonticBypass,
                timestamp: String::new(),
                actor_id: None,
                from_state: None,
                to_state: None,
                event: None,
                data: Some(serde_json::json!({
                    "constraintId": perm.id,
                    "constraintType": "permission",
                    "rationale": rationale,
                })),
                audit_layer: None,
                actor_type: None,
                lifecycle_state: None,
                definition_version: None,
                inputs: Vec::new(),
                outputs: Vec::new(),
                input_digest: None,
                output_digest: None,
                transition_tags: Vec::new(),
                case_file_snapshot: None,
            });
        }
        for prohib in &constraints.prohibitions {
            provenance.push(ProvenanceRecord {
                record_kind: ProvenanceKind::DeonticBypass,
                timestamp: String::new(),
                actor_id: None,
                from_state: None,
                to_state: None,
                event: None,
                data: Some(serde_json::json!({
                    "constraintId": prohib.id,
                    "constraintType": "prohibition",
                    "rationale": rationale,
                })),
                audit_layer: None,
                actor_type: None,
                lifecycle_state: None,
                definition_version: None,
                inputs: Vec::new(),
                outputs: Vec::new(),
                input_digest: None,
                output_digest: None,
                transition_tags: Vec::new(),
                case_file_snapshot: None,
            });
        }
        for oblig in &constraints.obligations {
            provenance.push(ProvenanceRecord {
                record_kind: ProvenanceKind::DeonticBypass,
                timestamp: String::new(),
                actor_id: None,
                from_state: None,
                to_state: None,
                event: None,
                data: Some(serde_json::json!({
                    "constraintId": oblig.id,
                    "constraintType": "obligation",
                    "rationale": rationale,
                })),
                audit_layer: None,
                actor_type: None,
                lifecycle_state: None,
                definition_version: None,
                inputs: Vec::new(),
                outputs: Vec::new(),
                input_digest: None,
                output_digest: None,
                transition_tags: Vec::new(),
                case_file_snapshot: None,
            });
        }
        return;
    }

    // Permissions (AI S4.2): bounds must be satisfied.
    for perm in &constraints.permissions {
        if let Some(ref bounds_expr) = perm.bounds {
            let result = evaluate_fel_expression(bounds_expr, output, case_state);
            let (violated, null_escalation) = match result {
                FelResult::True => (false, false),
                FelResult::False => (true, false),
                FelResult::Null => {
                    let escalate = handle_null_propagation(
                        perm.null_behavior.as_ref(),
                        impact_level,
                        &perm.id,
                        provenance,
                    );
                    (escalate, escalate)
                }
                FelResult::Error => (true, false),
            };

            if violated {
                // Null propagation overrides the onViolation action to
                // escalateToHuman (AI S4.9).
                let effective_action = if null_escalation {
                    ViolationAction::EscalateToHuman
                } else {
                    perm.on_violation
                };

                let mut data = serde_json::json!({
                    "constraintId": perm.id,
                    "action": violation_action_str(effective_action),
                });
                if escalation_active {
                    data["escalationActive"] = serde_json::json!(true);
                }
                if let Some(source) = invocation_source {
                    data["invocationSource"] = serde_json::json!(source);
                }
                provenance.push(ProvenanceRecord {
                    record_kind: ProvenanceKind::DeonticViolation,
                    timestamp: String::new(),
                    actor_id: None,
                    from_state: None,
                    to_state: None,
                    event: None,
                    data: Some(data),
                    audit_layer: None,
                    actor_type: None,
                    lifecycle_state: None,
                    definition_version: None,
                    inputs: Vec::new(),
                    outputs: Vec::new(),
                    input_digest: None,
                    output_digest: None,
                    transition_tags: Vec::new(),
                    case_file_snapshot: None,
                });
                violations.push((perm.id.clone(), effective_action));
            }
        }
    }

    // Prohibitions (AI S4.3): condition must NOT be true.
    // Null means "prohibited condition is not detectable" — default is pass
    // (absence of evidence is not evidence of violation). Explicit null_behavior
    // overrides this default.
    for prohib in &constraints.prohibitions {
        let result = evaluate_fel_expression(&prohib.condition, output, case_state);
        let violated = match result {
            FelResult::True => true,
            FelResult::False | FelResult::Error => false,
            FelResult::Null => {
                // Prohibitions default to pass on null unless explicitly configured.
                match prohib.null_behavior.as_ref() {
                    Some(NullBehavior::Deny) => true,
                    Some(NullBehavior::Escalate) => true,
                    _ => false,
                }
            }
        };

        if violated {
            provenance.push(ProvenanceRecord {
                record_kind: ProvenanceKind::DeonticViolation,
                timestamp: String::new(),
                actor_id: None,
                from_state: None,
                to_state: None,
                event: None,
                data: Some(serde_json::json!({
                    "constraintId": prohib.id,
                    "action": violation_action_str(prohib.on_violation),
                })),
                audit_layer: None,
                actor_type: None,
                lifecycle_state: None,
                definition_version: None,
                inputs: Vec::new(),
                outputs: Vec::new(),
                input_digest: None,
                output_digest: None,
                transition_tags: Vec::new(),
                case_file_snapshot: None,
            });
            violations.push((prohib.id.clone(), prohib.on_violation));
        }
    }

    // Obligations (AI S4.4): requirement must be true.
    // Null means "requirement not verifiable" — treated as violated, because
    // the obligation imposes an affirmative duty. The constraint's onViolation
    // action applies (not null propagation escalation).
    for oblig in &constraints.obligations {
        let result = evaluate_fel_expression(&oblig.requirement, output, case_state);
        let violated = match result {
            FelResult::True => false,
            FelResult::False | FelResult::Null | FelResult::Error => true,
        };

        if violated {
            provenance.push(ProvenanceRecord {
                record_kind: ProvenanceKind::DeonticViolation,
                timestamp: String::new(),
                actor_id: None,
                from_state: None,
                to_state: None,
                event: None,
                data: Some(serde_json::json!({
                    "constraintId": oblig.id,
                    "action": violation_action_str(oblig.on_violation),
                })),
                audit_layer: None,
                actor_type: None,
                lifecycle_state: None,
                definition_version: None,
                inputs: Vec::new(),
                outputs: Vec::new(),
                input_digest: None,
                output_digest: None,
                transition_tags: Vec::new(),
                case_file_snapshot: None,
            });
            violations.push((oblig.id.clone(), oblig.on_violation));
        }
    }
}

/// Handle null propagation for deontic constraints (AI S4.9).
///
/// Impact-dependent behavior:
/// - Rights/safety impacting: escalate to human (null = danger)
/// - Operational/informational: pass (null = no data, not a violation)
fn handle_null_propagation(
    behavior: Option<&NullBehavior>,
    impact_level: &ImpactLevel,
    constraint_id: &str,
    provenance: &mut Vec<ProvenanceRecord>,
) -> bool {
    if let Some(behavior) = behavior {
        return match behavior {
            NullBehavior::Pass => false,
            NullBehavior::Deny => true,
            NullBehavior::Escalate => {
                provenance.push(ProvenanceRecord {
                    record_kind: ProvenanceKind::DeonticViolation,
                    timestamp: String::new(),
                    actor_id: None,
                    from_state: None,
                    to_state: None,
                    event: None,
                    data: Some(serde_json::json!({
                        "reason": "null-expression-escalation",
                        "constraintId": constraint_id,
                        "impactLevel": impact_level_str(impact_level),
                    })),
                    audit_layer: None,
                    actor_type: None,
                    lifecycle_state: None,
                    definition_version: None,
                    inputs: Vec::new(),
                    outputs: Vec::new(),
                    input_digest: None,
                    output_digest: None,
                    transition_tags: Vec::new(),
                    case_file_snapshot: None,
                });
                true
            }
        };
    }

    // Default: impact-dependent.
    match impact_level {
        ImpactLevel::RightsImpacting | ImpactLevel::SafetyImpacting => {
            provenance.push(ProvenanceRecord {
                record_kind: ProvenanceKind::DeonticViolation,
                timestamp: String::new(),
                actor_id: None,
                from_state: None,
                to_state: None,
                event: None,
                data: Some(serde_json::json!({
                    "reason": "null-expression-escalation",
                    "impactLevel": impact_level_str(impact_level),
                })),
                audit_layer: None,
                actor_type: None,
                lifecycle_state: None,
                definition_version: None,
                inputs: Vec::new(),
                outputs: Vec::new(),
                input_digest: None,
                output_digest: None,
                transition_tags: Vec::new(),
                case_file_snapshot: None,
            });
            true
        }
        ImpactLevel::Operational | ImpactLevel::Informational => false,
    }
}

/// Check consistency between agent output and case state (AI S4.7).
///
/// Implementation-defined thresholds per AI S4.7 consistency requirement:
/// - Relative threshold (50%): catches meaningful discrepancies proportional
///   to the case value.
/// - Absolute threshold ($1): prevents false positives on small values where
///   a 50% difference is trivial (e.g., $0.50 vs $0.30).
///
/// Both thresholds must be exceeded for a violation. These are conservative
/// defaults suitable for benefits adjudication. Production deployments may
/// configure tighter thresholds via workflow-specific parameters.
const CONSISTENCY_RELATIVE_THRESHOLD: f64 = 0.5;
const CONSISTENCY_ABSOLUTE_THRESHOLD: f64 = 1.0;

fn check_consistency(
    output: &serde_json::Value,
    case_state: &HashMap<String, serde_json::Value>,
    provenance: &mut Vec<ProvenanceRecord>,
) {
    if let serde_json::Value::Object(output_map) = output {
        for (field, agent_value) in output_map {
            if let Some(case_value) = case_state.get(field) {
                // Only flag numeric contradictions with significant difference.
                if let (Some(agent_num), Some(case_num)) =
                    (agent_value.as_f64(), case_value.as_f64())
                {
                    let diff = (agent_num - case_num).abs();
                    let threshold = case_num.abs() * CONSISTENCY_RELATIVE_THRESHOLD;
                    if diff > threshold && diff > CONSISTENCY_ABSOLUTE_THRESHOLD {
                        provenance.push(ProvenanceRecord {
                            record_kind: ProvenanceKind::ConsistencyViolation,
                            timestamp: String::new(),
                            actor_id: None,
                            from_state: None,
                            to_state: None,
                            event: None,
                            data: Some(serde_json::json!({
                                "field": field,
                                "agentValue": agent_value,
                                "caseValue": case_value,
                            })),
                            audit_layer: None,
                            actor_type: None,
                            lifecycle_state: None,
                            definition_version: None,
                            inputs: Vec::new(),
                            outputs: Vec::new(),
                            input_digest: None,
                            output_digest: None,
                            transition_tags: Vec::new(),
                            case_file_snapshot: None,
                        });
                    }
                }
            }
        }
    }
}

// ── FEL evaluation helpers ──────────────────────────────────────

/// Simplified FEL result for deontic evaluation.
enum FelResult {
    True,
    False,
    Null,
    Error,
}

/// Evaluate a FEL expression in the context of agent output and case state.
///
/// The expression can reference `output.*` and `caseFile.*` variables.
fn evaluate_fel_expression(
    expression: &str,
    output: &serde_json::Value,
    case_state: &HashMap<String, serde_json::Value>,
) -> FelResult {
    let parsed = match parse(expression) {
        Ok(ast) => ast,
        Err(_) => return FelResult::Error,
    };

    let mut fields = HashMap::new();

    // Flatten output into `output.field` variables.
    if let serde_json::Value::Object(output_map) = output {
        for (key, value) in output_map {
            fields.insert(format!("output.{key}"), json_to_fel(value));
        }
    }

    // Flatten case state into `caseFile.field` variables.
    for (key, value) in case_state {
        fields.insert(format!("caseFile.{key}"), json_to_fel(value));
    }

    let env = MapEnvironment::with_fields(fields);
    let result = evaluate(&parsed, &env);
    match result.value {
        FelValue::Boolean(true) => FelResult::True,
        FelValue::Boolean(false) => FelResult::False,
        FelValue::Null => FelResult::Null,
        _ => FelResult::False,
    }
}

/// Determine the most restrictive enforcement action.
///
/// Severity order (most to least restrictive):
/// reject > escalateToHuman > switchToAssistive > flag
fn resolve_most_restrictive(violations: &[(String, ViolationAction)]) -> Option<ViolationAction> {
    violations
        .iter()
        .map(|(_, action)| *action)
        .max_by_key(|action| match action {
            ViolationAction::Reject => 4,
            ViolationAction::EscalateToHuman => 3,
            ViolationAction::SwitchToAssistive => 2,
            ViolationAction::Flag => 1,
        })
}

/// Serialize an impact level to its canonical JSON string form.
fn impact_level_str(level: &ImpactLevel) -> &'static str {
    match level {
        ImpactLevel::RightsImpacting => "rights-impacting",
        ImpactLevel::SafetyImpacting => "safety-impacting",
        ImpactLevel::Operational => "operational",
        ImpactLevel::Informational => "informational",
    }
}

/// Serialize a violation action to its JSON string form.
fn violation_action_str(action: ViolationAction) -> &'static str {
    match action {
        ViolationAction::Reject => "reject",
        ViolationAction::EscalateToHuman => "escalateToHuman",
        ViolationAction::SwitchToAssistive => "switchToAssistive",
        ViolationAction::Flag => "flag",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn most_restrictive_reject_wins_over_escalate() {
        let violations = vec![
            ("a".into(), ViolationAction::EscalateToHuman),
            ("b".into(), ViolationAction::Reject),
        ];
        assert_eq!(
            resolve_most_restrictive(&violations),
            Some(ViolationAction::Reject)
        );
    }

    #[test]
    fn empty_violations_returns_none() {
        let violations: Vec<(String, ViolationAction)> = vec![];
        assert_eq!(resolve_most_restrictive(&violations), None);
    }

    #[test]
    fn fel_expression_evaluates_output_bounds() {
        let output = serde_json::json!({ "income": 50000 });
        let case_state = HashMap::new();
        let result = super::evaluate_fel_expression(
            "output.income >= 0 and output.income <= 500000",
            &output,
            &case_state,
        );
        assert!(matches!(result, super::FelResult::True));
    }

    #[test]
    fn fel_expression_negative_income_violates_bounds() {
        let output = serde_json::json!({ "income": -1000 });
        let case_state = HashMap::new();
        let result = super::evaluate_fel_expression(
            "output.income >= 0 and output.income <= 500000",
            &output,
            &case_state,
        );
        assert!(matches!(result, super::FelResult::False));
    }

    #[test]
    fn clean_output_no_violations() {
        use crate::model::ai::AIIntegrationDocument;
        use crate::model::kernel::ImpactLevel;

        let ai_json = r#"{
            "$wosAIIntegration": "1.0",
            "targetWorkflow": "test",
            "agents": [{
                "id": "agent1",
                "type": "agent",
                "agentType": "generative",
                "modelIdentifier": "test",
                "modelVersion": "1.0",
                "deonticConstraints": {
                    "permissions": [{
                        "id": "perm-1",
                        "bounds": "output.income >= 0 and output.income <= 500000",
                        "onViolation": "reject"
                    }],
                    "obligations": [{
                        "id": "oblig-1",
                        "requirement": "output.confidenceReport != null",
                        "onViolation": "reject"
                    }]
                }
            }]
        }"#;
        let ai_doc: AIIntegrationDocument = serde_json::from_str(ai_json).unwrap();

        let output = serde_json::json!({
            "income": 50000,
            "confidenceReport": { "overall": 0.90 }
        });
        let mut case_state = HashMap::new();
        case_state.insert("income".into(), serde_json::json!(50000));

        let result = evaluate_deontic_constraints(
            &ai_doc,
            "agent1",
            &output,
            &case_state,
            &ImpactLevel::RightsImpacting,
            None,
            false,
            None,
        );

        assert!(
            result.effective_action.is_none(),
            "expected no violations with clean output"
        );
    }
}
