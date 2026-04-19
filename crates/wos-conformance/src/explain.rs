//! Human-readable and machine-diffable rendering of [`ConformanceTrace`] values.
//!
//! Two entry points:
//!
//! - [`render_trace`] — formats a trace as prose for human/LLM consumption (used
//!   by `wos-conformance-explain`).
//! - [`diff_traces`] — compares an expected trace against a fresh one and returns
//!   the first point of divergence (used by `wos-conformance-diff`).

use serde::{Deserialize, Serialize};

use crate::trace::{ConformanceTrace, Delta, Outcome, TraceStep};

// ── Prose rendering ──────────────────────────────────────────────────────────

/// Format a [`ConformanceTrace`] as human-readable prose.
///
/// Output is scannable line-by-line and diff-friendly. Format mirrors the
/// shape from the §5.3 plan:
///
/// ```text
/// Fixture: benefits-adjudication (kernel 1.0)
///   step 1: initial → application-received (event: application.submitted) ✓
///   step 2: review → rejected (event: approver.decide)
///     ✗ expected: approved
///       actual:   rejected
///       reason:   guard G-02 evaluated false
///                 expression: `caseFile.benefit_amount <= caseFile.income_limit`
///                 inputs:     { "benefit_amount": 520, "income_limit": 500 }
///
/// Summary: FAIL | 2 steps | first divergence at step 2
/// ```
pub fn render_trace(trace: &ConformanceTrace) -> String {
    let mut out = String::new();

    // Header
    out.push_str(&format!(
        "Fixture: {} (kernel {})\n",
        trace.fixture_id, trace.kernel_version
    ));

    let first_divergence = trace
        .steps
        .iter()
        .find(|s| s.delta.is_some())
        .map(|s| s.step_index);

    if trace.steps.is_empty() {
        out.push_str("  (no transitions recorded)\n");
    } else {
        for step in &trace.steps {
            out.push_str(&render_step(step));
        }
    }

    out.push('\n');
    out.push_str(&render_summary(trace, first_divergence));
    out.push('\n');

    out
}

fn render_step(step: &TraceStep) -> String {
    let mut out = String::new();

    let step_num = step.step_index + 1;
    let has_delta = step.delta.is_some();
    let tick = if has_delta { "✗" } else { "✓" };

    let actor_part = step
        .event
        .source_actor
        .as_deref()
        .map(|a| format!(" actor:{a}"))
        .unwrap_or_default();

    out.push_str(&format!(
        "  step {}: {} → {} (event: {}{}){}\n",
        step_num, step.state_before, step.state_after, step.event.name, actor_part, if has_delta { "" } else { &format!(" {tick}") }
    ));

    // Guards evaluated
    if !step.guards_evaluated.is_empty() {
        for guard in &step.guards_evaluated {
            let result_mark = if guard.result { "true " } else { "FALSE" };
            out.push_str(&format!(
                "    guard {}: [{}] `{}`\n",
                guard.guard_id, result_mark, guard.expression
            ));
            if guard.result || step.delta.is_some() {
                let inputs_str = compact_json(&guard.inputs);
                out.push_str(&format!("          inputs: {inputs_str}\n"));
            }
        }
    }

    // Policies applied
    if !step.policies_applied.is_empty() {
        for policy in &step.policies_applied {
            let bindings_str = compact_json(&policy.parameter_bindings);
            out.push_str(&format!(
                "    policy {}: {bindings_str}\n",
                policy.policy_id
            ));
        }
    }

    // Delta detail
    if let Some(delta) = &step.delta {
        out.push_str(&format!("    {tick} expected: {}\n", expected_state_label(step)));
        out.push_str(&format!("      actual:   {}\n", step.state_after));
        out.push_str(&render_delta_reason(delta));
    }

    out
}

fn expected_state_label(step: &TraceStep) -> &str {
    step.expected_state_after
        .as_deref()
        .unwrap_or("(none)")
}

fn render_delta_reason(delta: &Delta) -> String {
    match delta {
        Delta::GuardFalse {
            guard_id,
            expression,
            inputs,
        } => {
            let inputs_str = compact_json(inputs);
            format!(
                "      reason:   guard `{guard_id}` evaluated false\n\
                 \t\t  expression: `{expression}`\n\
                 \t\t  inputs:     {inputs_str}\n"
            )
        }
        Delta::StateMismatch {
            expected,
            actual,
            cause,
        } => {
            let cause_line = cause
                .as_deref()
                .map(|c| format!("\n      cause:    {c}"))
                .unwrap_or_default();
            format!(
                "      reason:   state mismatch (expected `{expected}`, got `{actual}`){cause_line}\n"
            )
        }
        Delta::PolicyOverride {
            policy_id,
            expected_without_policy,
            actual_with_policy,
        } => {
            format!(
                "      reason:   policy `{policy_id}` changed outcome\n\
                 \t\t  without policy: {expected_without_policy}\n\
                 \t\t  with policy:    {actual_with_policy}\n"
            )
        }
    }
}

fn render_summary(trace: &ConformanceTrace, first_divergence: Option<u32>) -> String {
    let outcome_label = match trace.outcome {
        Outcome::Pass => "PASS",
        Outcome::Fail => "FAIL",
        Outcome::Error => "ERROR",
    };
    let step_count = trace.steps.len();
    let step_word = if step_count == 1 { "step" } else { "steps" };

    let divergence_part = match first_divergence {
        Some(idx) => format!(" | first divergence at step {}", idx + 1),
        None if trace.outcome != Outcome::Pass => " | no recorded divergence (lint/parse failure)".to_string(),
        None => String::new(),
    };

    format!(
        "Summary: {outcome_label} | {step_count} {step_word}{divergence_part}"
    )
}

/// Format a JSON value compactly on one line, falling back to `"…"` on error.
fn compact_json(value: &serde_json::Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "…".to_string())
}

// ── Structural diff ──────────────────────────────────────────────────────────

/// The result of comparing two [`ConformanceTrace`] values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TraceDiffResult {
    /// Traces match — no divergence.
    Match,
    /// Traces diverge at the described point.
    Divergence(TraceDivergence),
}

/// Structured description of the first point where two traces diverge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TraceDivergence {
    /// One-based step number where the divergence occurs, if inside a step.
    /// `None` means the divergence is at the top level (e.g., outcome or step count).
    pub differs_at_step: Option<u32>,
    /// Expected state at the diverging step (from the expected trace).
    pub expected_state: Option<String>,
    /// Actual state at the diverging step (from the actual trace).
    pub actual_state: Option<String>,
    /// Structured cause, if one can be determined.
    pub cause: Option<DivergenceCause>,
    /// Human-readable hint about what likely caused the divergence.
    pub suggested_hypothesis: String,
    /// Free-form description when divergence is structural (mismatched step
    /// counts, mismatched outcomes, or fixture id mismatch).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Structured cause embedded in a [`TraceDivergence`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum DivergenceCause {
    /// A guard evaluated false when the expected trace had it pass (or vice versa).
    #[serde(rename = "guard-false")]
    GuardFalse {
        guard_id: String,
        expression: String,
        inputs: serde_json::Value,
    },
    /// The states match but the outcome labels differ.
    #[serde(rename = "outcome-mismatch")]
    OutcomeMismatch { expected: String, actual: String },
    /// The step counts differ.
    #[serde(rename = "step-count-mismatch")]
    StepCountMismatch { expected: usize, actual: usize },
    /// A state value on a step differs.
    #[serde(rename = "state-mismatch")]
    StateMismatch { expected: String, actual: String },
    /// The `source_actor` field on a step differs.
    ///
    /// `source_actor` is normative for governance/audit use cases: a trace
    /// where states match but actors differ represents a real divergence (e.g.
    /// a human-approver step was fulfilled by an AI agent). `event.payload` is
    /// intentionally NOT compared — payload contents are often optional,
    /// environment-specific, or noisy, and are not load-bearing for the
    /// state-machine conformance signal.
    #[serde(rename = "actor-mismatch")]
    ActorMismatch { expected: String, actual: String },
}

/// Compare an expected trace against an actual trace and return the first
/// divergence, or [`TraceDiffResult::Match`] if they are equivalent.
///
/// Comparison is structural: fixture_id, outcome, step count, and per-step
/// (state_before, state_after, event.name, source_actor) are checked in order.
/// Guards and policies are intentionally NOT compared field-by-field; if the
/// states match, runtime internals are considered non-normative for diff
/// purposes. This matches the teaching-signal contract: the spec asserts on
/// *what* happened (states), not *how* (which internal guard fired).
///
/// `source_actor` IS compared because actor identity is normative for
/// governance/audit use cases: matching states with mismatched actors
/// constitutes a real behavioral divergence.
///
/// `event.payload` is intentionally NOT compared — payload contents are often
/// optional, environment-specific, or noisy, and are not load-bearing for the
/// state-machine conformance signal.
///
/// When the actual step has a `Delta::GuardFalse`, that guard detail is
/// surfaced in the `cause` field so a repair prompt gets it directly.
pub fn diff_traces(expected: &ConformanceTrace, actual: &ConformanceTrace) -> TraceDiffResult {
    // Top-level fixture id mismatch is reported as a structural divergence.
    if expected.fixture_id != actual.fixture_id {
        return TraceDiffResult::Divergence(TraceDivergence {
            differs_at_step: None,
            expected_state: None,
            actual_state: None,
            cause: None,
            suggested_hypothesis: format!(
                "fixture id mismatch: expected `{}`, got `{}`",
                expected.fixture_id, actual.fixture_id
            ),
            detail: Some(format!(
                "fixture_id: expected `{}`, got `{}`",
                expected.fixture_id, actual.fixture_id
            )),
        });
    }

    // Step count mismatch — report before per-step comparison.
    if expected.steps.len() != actual.steps.len() {
        let hypothesis = if actual.steps.len() < expected.steps.len() {
            format!(
                "actual trace has fewer steps ({}) than expected ({}); \
                 a guard or event may have blocked a transition",
                actual.steps.len(),
                expected.steps.len()
            )
        } else {
            format!(
                "actual trace has more steps ({}) than expected ({}); \
                 an extra transition may have fired",
                actual.steps.len(),
                expected.steps.len()
            )
        };
        return TraceDiffResult::Divergence(TraceDivergence {
            differs_at_step: None,
            expected_state: None,
            actual_state: None,
            cause: Some(DivergenceCause::StepCountMismatch {
                expected: expected.steps.len(),
                actual: actual.steps.len(),
            }),
            suggested_hypothesis: hypothesis,
            detail: None,
        });
    }

    // Outcome mismatch.
    if expected.outcome != actual.outcome {
        let exp_str = outcome_label(expected.outcome);
        let act_str = outcome_label(actual.outcome);
        return TraceDiffResult::Divergence(TraceDivergence {
            differs_at_step: None,
            expected_state: None,
            actual_state: None,
            cause: Some(DivergenceCause::OutcomeMismatch {
                expected: exp_str.to_string(),
                actual: act_str.to_string(),
            }),
            suggested_hypothesis: format!(
                "overall outcome differs: expected `{exp_str}`, got `{act_str}`; \
                 check the step sequence for a blocking guard or assertion failure"
            ),
            detail: None,
        });
    }

    // Per-step comparison.
    for (exp_step, act_step) in expected.steps.iter().zip(actual.steps.iter()) {
        let step_num = exp_step.step_index + 1;

        if exp_step.state_before != act_step.state_before
            || exp_step.state_after != act_step.state_after
        {
            let exp_state = format!(
                "{} → {}",
                exp_step.state_before, exp_step.state_after
            );
            let act_state = format!(
                "{} → {}",
                act_step.state_before, act_step.state_after
            );
            let cause = extract_guard_cause(act_step)
                .or_else(|| Some(DivergenceCause::StateMismatch {
                    expected: exp_state.clone(),
                    actual: act_state.clone(),
                }));

            let hypothesis = build_state_hypothesis(exp_step, act_step);
            return TraceDiffResult::Divergence(TraceDivergence {
                differs_at_step: Some(step_num),
                expected_state: Some(exp_step.state_after.clone()),
                actual_state: Some(act_step.state_after.clone()),
                cause,
                suggested_hypothesis: hypothesis,
                detail: None,
            });
        }

        // Event name mismatch (same step index, different event).
        if exp_step.event.name != act_step.event.name {
            return TraceDiffResult::Divergence(TraceDivergence {
                differs_at_step: Some(step_num),
                expected_state: Some(exp_step.state_after.clone()),
                actual_state: Some(act_step.state_after.clone()),
                cause: None,
                suggested_hypothesis: format!(
                    "step {step_num} fired on event `{}` but expected `{}`; \
                     check event routing or guard preconditions",
                    act_step.event.name, exp_step.event.name
                ),
                detail: Some(format!(
                    "event: expected `{}`, got `{}`",
                    exp_step.event.name, act_step.event.name
                )),
            });
        }

        // source_actor mismatch — normative for governance/audit.
        if exp_step.event.source_actor != act_step.event.source_actor {
            let expected_actor = exp_step
                .event
                .source_actor
                .clone()
                .unwrap_or_else(|| "(none)".to_string());
            let actual_actor = act_step
                .event
                .source_actor
                .clone()
                .unwrap_or_else(|| "(none)".to_string());
            return TraceDiffResult::Divergence(TraceDivergence {
                differs_at_step: Some(step_num),
                expected_state: Some(exp_step.state_after.clone()),
                actual_state: Some(act_step.state_after.clone()),
                cause: Some(DivergenceCause::ActorMismatch {
                    expected: expected_actor.clone(),
                    actual: actual_actor.clone(),
                }),
                suggested_hypothesis: format!(
                    "step {step_num} was fulfilled by actor `{actual_actor}` \
                     but expected `{expected_actor}`; \
                     check actor assignment or delegation rules",
                ),
                detail: Some(format!(
                    "source_actor: expected `{expected_actor}`, got `{actual_actor}`"
                )),
            });
        }
    }

    TraceDiffResult::Match
}

/// Render a [`TraceDiffResult`] as a human-readable diff output.
///
/// Prints `OK` for a match. Prints a structured prose report for a divergence.
pub fn render_diff(result: &TraceDiffResult) -> String {
    match result {
        TraceDiffResult::Match => "OK\n".to_string(),
        TraceDiffResult::Divergence(div) => render_divergence(div),
    }
}

fn render_divergence(div: &TraceDivergence) -> String {
    let mut out = String::new();
    out.push_str("DIVERGENCE\n");

    if let Some(step) = div.differs_at_step {
        out.push_str(&format!("  at step: {step}\n"));
    }
    if let Some(exp) = &div.expected_state {
        out.push_str(&format!("  expected state: {exp}\n"));
    }
    if let Some(act) = &div.actual_state {
        out.push_str(&format!("  actual state:   {act}\n"));
    }
    if let Some(cause) = &div.cause {
        out.push_str("  cause:\n");
        match cause {
            DivergenceCause::GuardFalse { guard_id, expression, inputs } => {
                let inputs_str = compact_json(inputs);
                out.push_str(&format!("    kind:       guard-false\n"));
                out.push_str(&format!("    guard_id:   {guard_id}\n"));
                out.push_str(&format!("    expression: `{expression}`\n"));
                out.push_str(&format!("    inputs:     {inputs_str}\n"));
            }
            DivergenceCause::OutcomeMismatch { expected, actual } => {
                out.push_str(&format!("    kind:     outcome-mismatch\n"));
                out.push_str(&format!("    expected: {expected}\n"));
                out.push_str(&format!("    actual:   {actual}\n"));
            }
            DivergenceCause::StepCountMismatch { expected, actual } => {
                out.push_str(&format!("    kind:     step-count-mismatch\n"));
                out.push_str(&format!("    expected: {expected} steps\n"));
                out.push_str(&format!("    actual:   {actual} steps\n"));
            }
            DivergenceCause::StateMismatch { expected, actual } => {
                out.push_str(&format!("    kind:     state-mismatch\n"));
                out.push_str(&format!("    expected: {expected}\n"));
                out.push_str(&format!("    actual:   {actual}\n"));
            }
            DivergenceCause::ActorMismatch { expected, actual } => {
                out.push_str(&format!("    kind:     actor-mismatch\n"));
                out.push_str(&format!("    expected: {expected}\n"));
                out.push_str(&format!("    actual:   {actual}\n"));
            }
        }
    }
    if let Some(detail) = &div.detail {
        out.push_str(&format!("  detail: {detail}\n"));
    }
    out.push_str(&format!(
        "  hypothesis: {}\n",
        div.suggested_hypothesis
    ));

    out
}

fn outcome_label(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Pass => "pass",
        Outcome::Fail => "fail",
        Outcome::Error => "error",
    }
}

/// Extract a [`DivergenceCause::GuardFalse`] from the actual step's `delta`
/// field, if it is a [`Delta::GuardFalse`].
fn extract_guard_cause(step: &TraceStep) -> Option<DivergenceCause> {
    match step.delta.as_ref()? {
        Delta::GuardFalse {
            guard_id,
            expression,
            inputs,
        } => Some(DivergenceCause::GuardFalse {
            guard_id: guard_id.clone(),
            expression: expression.clone(),
            inputs: inputs.clone(),
        }),
        _ => None,
    }
}

/// Build a human-readable hypothesis for a state divergence.
fn build_state_hypothesis(exp_step: &TraceStep, act_step: &TraceStep) -> String {
    if let Some(Delta::GuardFalse { guard_id, expression, inputs }) = &act_step.delta {
        let inputs_str = compact_json(inputs);
        return format!(
            "guard `{guard_id}` evaluated false; expression: `{expression}`; \
             inputs: {inputs_str}; workflow may need guard condition repair"
        );
    }
    format!(
        "step {} expected state `{}` but got `{}`; \
         check guard conditions or event routing for event `{}`",
        exp_step.step_index + 1,
        exp_step.state_after,
        act_step.state_after,
        act_step.event.name
    )
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::{ConformanceTrace, Delta, Event, GuardEvaluation, Outcome, PolicyApplication, TraceStep};
    use serde_json::json;

    fn make_step(index: u32, from: &str, to: &str, event: &str) -> TraceStep {
        TraceStep {
            step_index: index,
            event: Event {
                name: event.to_string(),
                source_actor: None,
                payload: None,
            },
            state_before: from.to_string(),
            state_after: to.to_string(),
            expected_state_after: Some(to.to_string()),
            guards_evaluated: Vec::new(),
            policies_applied: Vec::new(),
            delta: None,
        }
    }

    fn make_trace(id: &str, outcome: Outcome, steps: Vec<TraceStep>) -> ConformanceTrace {
        ConformanceTrace {
            fixture_id: id.to_string(),
            kernel_version: "1.0".to_string(),
            steps,
            outcome,
        }
    }

    // ── render_trace ──────────────────────────────────────────────────────────

    /// Passing trace renders fixture header, step lines, and PASS summary.
    #[test]
    fn render_trace_passing_shows_header_steps_and_pass_summary() {
        let steps = vec![
            make_step(0, "submitted", "approved", "approve"),
            make_step(1, "approved", "completed", "orderProcessed"),
        ];
        let trace = make_trace("K-011-determinism", Outcome::Pass, steps);
        let output = render_trace(&trace);

        assert!(
            output.contains("Fixture: K-011-determinism (kernel 1.0)"),
            "header missing: {output}"
        );
        assert!(
            output.contains("step 1: submitted → approved (event: approve"),
            "step 1 missing: {output}"
        );
        assert!(
            output.contains("step 2: approved → completed (event: orderProcessed"),
            "step 2 missing: {output}"
        );
        assert!(output.contains("PASS"), "summary missing PASS: {output}");
        assert!(output.contains("2 steps"), "step count missing: {output}");
    }

    /// Guard detail lines appear for each guard evaluation on a step.
    #[test]
    fn render_trace_shows_guard_detail_when_present() {
        let guard = GuardEvaluation {
            guard_id: "submitted->approved:approve".to_string(),
            source_state: "submitted".to_string(),
            target_state: "approved".to_string(),
            event: "approve".to_string(),
            expression: "caseFile.amount <= 50000".to_string(),
            result: true,
            inputs: json!({ "caseFile": { "amount": 3000 } }),
        };
        let mut step = make_step(0, "submitted", "approved", "approve");
        step.guards_evaluated = vec![guard];

        let trace = make_trace("K-011-determinism", Outcome::Pass, vec![step]);
        let output = render_trace(&trace);

        assert!(
            output.contains("submitted->approved:approve"),
            "guard_id missing: {output}"
        );
        assert!(
            output.contains("caseFile.amount <= 50000"),
            "guard expression missing: {output}"
        );
    }

    /// Failing step shows expected/actual and GuardFalse reason.
    #[test]
    fn render_trace_failing_step_shows_divergence_and_guard_false() {
        let mut step = make_step(0, "submitted", "pendingDirectorApproval", "approve");
        step.expected_state_after = Some("approved".to_string());
        step.delta = Some(Delta::GuardFalse {
            guard_id: "submitted->approved:approve".to_string(),
            expression: "caseFile.amount <= 50000".to_string(),
            inputs: json!({ "caseFile": { "amount": 75000 } }),
        });

        let trace = make_trace("K-GUARD-FALSE", Outcome::Fail, vec![step]);
        let output = render_trace(&trace);

        assert!(output.contains("expected: approved"), "expected state missing: {output}");
        assert!(output.contains("actual:   pendingDirectorApproval"), "actual state missing: {output}");
        assert!(output.contains("submitted->approved:approve"), "guard_id missing: {output}");
        assert!(output.contains("caseFile.amount <= 50000"), "expression missing: {output}");
        assert!(output.contains("75000"), "input value missing: {output}");
        assert!(output.contains("FAIL"), "FAIL missing: {output}");
        assert!(output.contains("first divergence at step 1"), "divergence marker missing: {output}");
    }

    /// Policy application lines appear for each policy on a step.
    #[test]
    fn render_trace_shows_policy_application_when_present() {
        let policy = PolicyApplication {
            policy_id: "P-income-threshold".to_string(),
            parameter_bindings: json!({ "threshold": 50000 }),
        };
        let mut step = make_step(0, "submitted", "approved", "decide");
        step.policies_applied = vec![policy];

        let trace = make_trace("AI-014", Outcome::Pass, vec![step]);
        let output = render_trace(&trace);

        assert!(
            output.contains("P-income-threshold"),
            "policy_id missing: {output}"
        );
    }

    /// Empty trace renders "(no transitions recorded)" and appropriate summary.
    #[test]
    fn render_trace_empty_shows_no_transitions_recorded() {
        let trace = make_trace("K-001-negative", Outcome::Fail, vec![]);
        let output = render_trace(&trace);

        assert!(
            output.contains("(no transitions recorded)"),
            "empty-trace message missing: {output}"
        );
        assert!(output.contains("FAIL"), "FAIL missing: {output}");
    }

    // ── diff_traces ───────────────────────────────────────────────────────────

    /// Identical traces produce Match.
    #[test]
    fn diff_traces_identical_produces_match() {
        let steps = vec![make_step(0, "submitted", "approved", "approve")];
        let trace = make_trace("K-011", Outcome::Pass, steps);
        assert_eq!(diff_traces(&trace, &trace), TraceDiffResult::Match);
    }

    /// Step count mismatch surfaces StepCountMismatch cause.
    #[test]
    fn diff_traces_step_count_mismatch_surfaces_cause() {
        let expected = make_trace(
            "K-011",
            Outcome::Pass,
            vec![
                make_step(0, "submitted", "approved", "approve"),
                make_step(1, "approved", "completed", "orderProcessed"),
            ],
        );
        let actual = make_trace(
            "K-011",
            Outcome::Pass,
            vec![make_step(0, "submitted", "approved", "approve")],
        );
        let result = diff_traces(&expected, &actual);
        match result {
            TraceDiffResult::Divergence(div) => {
                assert!(
                    matches!(div.cause, Some(DivergenceCause::StepCountMismatch { .. })),
                    "expected StepCountMismatch cause"
                );
                assert!(
                    div.suggested_hypothesis.contains("fewer steps"),
                    "hypothesis should mention fewer steps: {}",
                    div.suggested_hypothesis
                );
            }
            TraceDiffResult::Match => panic!("expected divergence"),
        }
    }

    /// State mismatch at step N surfaces differs_at_step and expected/actual state.
    ///
    /// Use the same outcome on both sides so the outcome-check does not fire
    /// first and mask the per-step comparison.
    #[test]
    fn diff_traces_state_mismatch_surfaces_step_and_states() {
        let expected = make_trace(
            "K-011",
            Outcome::Pass,
            vec![make_step(0, "submitted", "approved", "approve")],
        );
        let mut actual_step = make_step(0, "submitted", "rejected", "approve");
        actual_step.expected_state_after = Some("approved".to_string());
        // Same outcome so the outcome check does not fire before the step check.
        let actual = make_trace("K-011", Outcome::Pass, vec![actual_step]);
        let result = diff_traces(&expected, &actual);
        match result {
            TraceDiffResult::Divergence(div) => {
                assert_eq!(div.differs_at_step, Some(1), "step number should be 1-based");
                assert_eq!(div.expected_state.as_deref(), Some("approved"));
                assert_eq!(div.actual_state.as_deref(), Some("rejected"));
            }
            TraceDiffResult::Match => panic!("expected divergence"),
        }
    }

    /// State mismatch with GuardFalse delta surfaces GuardFalse cause with guard details.
    ///
    /// Use the same outcome on both sides so the outcome-check does not fire
    /// first and mask the per-step comparison.
    #[test]
    fn diff_traces_guard_false_delta_surfaces_guard_cause() {
        let expected = make_trace(
            "K-GUARD",
            Outcome::Pass,
            vec![make_step(0, "submitted", "approved", "approve")],
        );
        let mut act_step = make_step(0, "submitted", "pendingDirectorApproval", "approve");
        act_step.expected_state_after = Some("approved".to_string());
        act_step.delta = Some(Delta::GuardFalse {
            guard_id: "submitted->approved:approve".to_string(),
            expression: "caseFile.amount <= 50000".to_string(),
            inputs: json!({ "caseFile": { "amount": 75000 } }),
        });
        // Same outcome on both sides so outcome-check does not short-circuit.
        let actual = make_trace("K-GUARD", Outcome::Pass, vec![act_step]);

        let result = diff_traces(&expected, &actual);
        match &result {
            TraceDiffResult::Divergence(div) => {
                assert_eq!(div.differs_at_step, Some(1));
                match &div.cause {
                    Some(DivergenceCause::GuardFalse { guard_id, expression, inputs }) => {
                        assert_eq!(guard_id, "submitted->approved:approve");
                        assert!(expression.contains("caseFile.amount"));
                        assert_eq!(inputs["caseFile"]["amount"], json!(75000));
                    }
                    other => panic!("expected GuardFalse cause, got {:?}", other),
                }
            }
            TraceDiffResult::Match => panic!("expected divergence"),
        }
    }

    /// Outcome mismatch surfaces OutcomeMismatch cause.
    #[test]
    fn diff_traces_outcome_mismatch_surfaces_outcome_cause() {
        let steps = vec![make_step(0, "submitted", "approved", "approve")];
        let expected = make_trace("K-011", Outcome::Pass, steps.clone());
        let actual = make_trace("K-011", Outcome::Fail, steps);
        let result = diff_traces(&expected, &actual);
        match result {
            TraceDiffResult::Divergence(div) => {
                assert!(
                    matches!(div.cause, Some(DivergenceCause::OutcomeMismatch { .. })),
                    "expected OutcomeMismatch"
                );
            }
            TraceDiffResult::Match => panic!("expected divergence"),
        }
    }

    /// Fixture id mismatch surfaces a divergence with hypothesis message.
    #[test]
    fn diff_traces_fixture_id_mismatch_surfaces_divergence() {
        let expected = make_trace("K-011", Outcome::Pass, vec![]);
        let actual = make_trace("K-999", Outcome::Pass, vec![]);
        let result = diff_traces(&expected, &actual);
        assert!(
            matches!(result, TraceDiffResult::Divergence(_)),
            "expected divergence on id mismatch"
        );
    }

    /// Two traces identical except source_actor → ActorMismatch divergence.
    ///
    /// Actor identity is normative for governance/audit use cases. Matching
    /// states with mismatched actors is a real behavioral divergence and must
    /// not silently pass.
    #[test]
    fn diff_traces_source_actor_mismatch_surfaces_actor_mismatch_cause() {
        let mut expected_step = make_step(0, "submitted", "approved", "approve");
        expected_step.event.source_actor = Some("human-approver".to_string());

        let mut actual_step = make_step(0, "submitted", "approved", "approve");
        actual_step.event.source_actor = Some("ai-agent".to_string());

        let expected = make_trace("K-ACTOR", Outcome::Pass, vec![expected_step]);
        let actual = make_trace("K-ACTOR", Outcome::Pass, vec![actual_step]);

        let result = diff_traces(&expected, &actual);
        match result {
            TraceDiffResult::Divergence(div) => {
                assert_eq!(div.differs_at_step, Some(1), "step number should be 1-based");
                match &div.cause {
                    Some(DivergenceCause::ActorMismatch { expected, actual }) => {
                        assert_eq!(expected, "human-approver");
                        assert_eq!(actual, "ai-agent");
                    }
                    other => panic!("expected ActorMismatch cause, got {other:?}"),
                }
                assert!(
                    div.suggested_hypothesis.contains("ai-agent"),
                    "hypothesis should name the actual actor: {}",
                    div.suggested_hypothesis
                );
                assert!(
                    div.suggested_hypothesis.contains("human-approver"),
                    "hypothesis should name the expected actor: {}",
                    div.suggested_hypothesis
                );
            }
            TraceDiffResult::Match => panic!("expected Divergence on actor mismatch"),
        }
    }

    // ── render_diff ───────────────────────────────────────────────────────────

    /// Match renders "OK".
    #[test]
    fn render_diff_match_renders_ok() {
        assert_eq!(render_diff(&TraceDiffResult::Match), "OK\n");
    }

    /// Divergence renders "DIVERGENCE" with step, states, and hypothesis.
    #[test]
    fn render_diff_divergence_renders_structured_output() {
        let div = TraceDivergence {
            differs_at_step: Some(3),
            expected_state: Some("approved".to_string()),
            actual_state: Some("rejected".to_string()),
            cause: Some(DivergenceCause::GuardFalse {
                guard_id: "G-02".to_string(),
                expression: "case.data.benefit_amount <= case.data.income_limit".to_string(),
                inputs: json!({ "benefit_amount": 520, "income_limit": 500 }),
            }),
            suggested_hypothesis: "benefit_amount exceeds income_limit".to_string(),
            detail: None,
        };
        let output = render_diff(&TraceDiffResult::Divergence(div));

        assert!(output.contains("DIVERGENCE"), "missing DIVERGENCE: {output}");
        assert!(output.contains("at step: 3"), "missing step: {output}");
        assert!(output.contains("expected state: approved"), "missing expected: {output}");
        assert!(output.contains("actual state:   rejected"), "missing actual: {output}");
        assert!(output.contains("guard-false"), "missing cause kind: {output}");
        assert!(output.contains("G-02"), "missing guard_id: {output}");
        assert!(output.contains("benefit_amount"), "missing expression: {output}");
        assert!(output.contains("benefit_amount exceeds income_limit"), "missing hypothesis: {output}");
    }
}
