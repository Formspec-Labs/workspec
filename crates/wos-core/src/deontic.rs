// Rust guideline compliant 2026-02-21

//! Kind-specific native rule evaluation over explicitly supplied context.
//!
//! This module reuses FEL and the native loader. Rule truth, null dispositions,
//! processor input obligations and evaluation failures remain distinct. It
//! authorizes neither invocation, review completion nor case-file writes.

use crate::ai_policy::{
    NativeAiPolicy, PolicyEvaluation, PolicyExerciseContext, PolicyPhase, RuleObservation,
    pointer_segments,
};
use crate::model::ai::{DeonticConstraint, NullBehavior, ViolationAction};
use crate::model::kernel::ImpactLevel;
use fel_core::{MapEnvironment, evaluate, json_to_fel, parse, types::Value as FelValue};
use serde_json::Value;
use std::collections::HashMap;

/// Evaluate the declared phase using the sole native policy interpretation.
///
/// Context identity and inputs come from the processor or explicit harness,
/// never from proposed output. Results are stage evidence, not readiness.
pub fn evaluate_native_policy(
    policy: &NativeAiPolicy,
    context: &PolicyExerciseContext<'_>,
) -> PolicyEvaluation {
    let mut result = PolicyEvaluation::default();
    let Some(agent) = policy.agent(context.agent_id) else {
        result.errors.push("unknown admitted agent actor".into());
        return result;
    };
    if context.waiver.is_some() {
        result.waiver_refused = true;
        result
            .refusals
            .push("authenticated scoped waiver operation is unavailable".into());
    }
    let mut scopes = vec![(
        "/aiOversight/deonticConstraints".to_string(),
        policy.oversight().deontic_constraints.as_slice(),
    )];
    let agent_index = policy
        .agents()
        .iter()
        .position(|v| v.id == agent.id)
        .expect("agent came from this policy");
    scopes.push((
        format!("/agents/{agent_index}/deonticConstraints"),
        agent.deontic_constraints.as_slice(),
    ));
    let action = match context.action_ref {
        Some(pointer) => {
            if let Err(error) = policy.action(pointer) {
                result.refusals.push(error.to_string());
                return result;
            }
            agent
                .action_overrides
                .iter()
                .enumerate()
                .find(|(_, a)| a.action_ref == pointer)
        }
        None => {
            if !agent.action_overrides.is_empty() {
                result.refusals.push(
                    "declared action policy requires an admitted lifecycle action context".into(),
                );
            }
            None
        }
    };
    if let Some((index, action)) = action {
        scopes.push((
            format!("/agents/{agent_index}/actionOverrides/{index}/deonticConstraints"),
            action.deontic_constraints.as_slice(),
        ));
    }
    if context.phase == PolicyPhase::InputRights {
        for (scope, rules) in &scopes {
            for (index, rule) in rules.iter().enumerate() {
                if let DeonticConstraint::Right(right) = rule {
                    let pointer = format!("{scope}/{index}");
                    let segments = pointer_segments(&right.entitlement)
                        .expect("loader admitted entitlement pointer");
                    if segments.first().map(String::as_str) == Some("input")
                        && context.input.is_none()
                    {
                        result
                            .refusals
                            .push(format!("{pointer}: resolved input context unavailable"));
                        continue;
                    }
                    let inputs =
                        serde_json::json!({"input":context.input,"caseFile":context.case_file});
                    let present = inputs
                        .pointer(&right.entitlement)
                        .is_some_and(|value| !value.is_null());
                    result.observations.push(RuleObservation {
                        pointer,
                        rule_id: right.id.clone(),
                        outcome: if present {
                            "present"
                        } else {
                            "missing-entitlement"
                        }
                        .into(),
                        action: None,
                        attribution: "processor".into(),
                        kind: "right".into(),
                    });
                    if !present {
                        result.processor_error = true;
                        result.errors.push(format!(
                            "right {} missing required nonnull input {}",
                            right.id, right.entitlement
                        ));
                    }
                }
            }
        }
        return result;
    }
    // Kind order does not change restriction composition or skip other scopes.
    for rank in 0..3 {
        for (scope, rules) in &scopes {
            for (index, rule) in rules.iter().enumerate() {
                let (kind, expression, action, null_behavior, fields) = match rule {
                    DeonticConstraint::Permission(v) if rank == 0 => (
                        "permission",
                        v.bounds.as_deref(),
                        v.on_violation,
                        v.null_behavior,
                        v.allowed_fields.as_deref(),
                    ),
                    DeonticConstraint::Prohibition(v) if rank == 1 => (
                        "prohibition",
                        Some(v.condition.as_str()),
                        v.on_violation,
                        v.null_behavior,
                        None,
                    ),
                    DeonticConstraint::Obligation(v) if rank == 2 => (
                        "obligation",
                        Some(v.requirement.as_str()),
                        v.on_violation,
                        v.null_behavior,
                        None,
                    ),
                    _ => continue,
                };
                let pointer = format!("{scope}/{index}");
                let mut selected = None;
                let mut outcomes = Vec::new();
                if let Some(allowed) = fields {
                    let mut terminals = Vec::new();
                    output_terminals(context.output, &mut Vec::new(), &mut terminals);
                    let allowed: Vec<Vec<String>> = allowed
                        .iter()
                        .map(|v| pointer_segments(v).expect("loader admitted output pointer"))
                        .collect();
                    if terminals
                        .iter()
                        .any(|terminal| !allowed.iter().any(|path| terminal.starts_with(path)))
                    {
                        selected = Some(action);
                        outcomes.push("output-field-out-of-scope".to_string());
                    } else {
                        outcomes.push("output-fields-in-scope".into());
                    }
                }
                if let Some(expression) = expression {
                    match predicate(expression, context, policy) {
                        Ok(Some(truth)) => {
                            let violates = if kind == "prohibition" { truth } else { !truth };
                            outcomes.push(
                                if violates {
                                    "predicate-violation"
                                } else {
                                    "predicate-satisfied"
                                }
                                .into(),
                            );
                            if violates {
                                selected = Some(
                                    selected.map_or(action, |old: ViolationAction| old.max(action)),
                                );
                            }
                        }
                        Ok(None) => {
                            let behavior = null_behavior.unwrap_or(
                                if matches!(
                                    policy.impact_level(),
                                    ImpactLevel::RightsImpacting | ImpactLevel::SafetyImpacting
                                ) {
                                    NullBehavior::Escalate
                                } else {
                                    NullBehavior::Pass
                                },
                            );
                            outcomes.push(format!("null-{behavior:?}"));
                            let effect = match behavior {
                                NullBehavior::Pass => None,
                                NullBehavior::Deny => Some(action),
                                NullBehavior::Escalate => Some(ViolationAction::EscalateToHuman),
                            };
                            if let Some(effect) = effect {
                                selected = Some(selected.map_or(effect, |old| old.max(effect)));
                            }
                        }
                        Err(error) => {
                            outcomes.push("evaluation-error".into());
                            result.errors.push(format!("{pointer}: {error}"));
                        }
                    }
                }
                if let Some(effect) = selected {
                    result.restrict(effect);
                }
                result.observations.push(RuleObservation {
                    pointer,
                    rule_id: rule.id().to_string(),
                    outcome: outcomes.join(";"),
                    action: selected,
                    attribution: "agent".into(),
                    kind: kind.into(),
                });
            }
        }
    }
    crate::confidence::evaluate_report_and_floors(
        policy,
        agent,
        action.map(|(index, action)| (index, action)),
        context,
        &mut result,
    );
    result
}
fn output_terminals(value: &Value, path: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
    match value {
        Value::Object(map) if !map.is_empty() => {
            for (key, value) in map {
                path.push(key.clone());
                output_terminals(value, path, out);
                path.pop();
            }
        }
        Value::Array(array) if !array.is_empty() => {
            for (index, value) in array.iter().enumerate() {
                path.push(index.to_string());
                output_terminals(value, path, out);
                path.pop();
            }
        }
        _ => out.push(path.clone()),
    }
}
/// Evaluate FEL as boolean-or-null; retain syntax, evaluator and type errors.
///
/// # Errors
/// Returns exact failures instead of converting them to false or null.
pub fn predicate(
    expression: &str,
    context: &PolicyExerciseContext<'_>,
    policy: &NativeAiPolicy,
) -> Result<Option<bool>, String> {
    let ast = parse(expression).map_err(|error| format!("FEL parse error: {error}"))?;
    let mut fields = HashMap::from([
        ("output".into(), json_to_fel(context.output)),
        ("caseFile".into(), json_to_fel(context.case_file)),
        (
            "instance".into(),
            json_to_fel(&serde_json::json!({"impactLevel":policy.impact_level()})),
        ),
    ]);
    if let Some(input) = context.input {
        fields.insert("input".into(), json_to_fel(input));
    }
    let value = evaluate(&ast, &MapEnvironment::with_fields(fields));
    if !value.diagnostics.is_empty() {
        return Err(format!(
            "FEL evaluation diagnostics: {:?}",
            value.diagnostics
        ));
    }
    match value.value {
        FelValue::Boolean(value) => Ok(Some(value)),
        FelValue::Null => Ok(None),
        other => Err(format!(
            "FEL predicate requires boolean or null, observed {other:?}"
        )),
    }
}
