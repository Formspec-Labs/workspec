// Rust guideline compliant 2026-02-21

//! Explicit confidence reports and restriction-preserving scoped floor checks.
//!
//! Reported values remain untrusted and unchanged. Calculations do not establish
//! calibration, decay history, cumulative tracking, invocation or review.

use crate::ai_policy::{NativeAiPolicy, PolicyEvaluation, PolicyExerciseContext, RuleObservation};
use crate::model::ai::{
    ActionOverride, AgentDeclaration, ConfidenceReport, ConfidenceViolationAction, ViolationAction,
};

pub(crate) fn evaluate_report_and_floors(
    policy: &NativeAiPolicy,
    agent: &AgentDeclaration,
    action: Option<(usize, &ActionOverride)>,
    context: &PolicyExerciseContext<'_>,
    result: &mut PolicyEvaluation,
) {
    let Some(raw) = context.confidence_report else {
        result
            .errors
            .push("missing required canonical confidenceReport".into());
        return;
    };
    let report: ConfidenceReport = match serde_json::from_value(raw.clone()) {
        Ok(report) => report,
        Err(error) => {
            result
                .errors
                .push(format!("invalid canonical confidenceReport: {error}"));
            return;
        }
    };
    if let Err(error) = report.validate() {
        result.errors.push(error);
        return;
    }
    let agent_index = policy
        .agents()
        .iter()
        .position(|v| v.id == agent.id)
        .expect("agent belongs to policy");
    let mut floors = Vec::new();
    if let Some(floor) = &policy.oversight().confidence_floor {
        floors.push(("/aiOversight/confidenceFloor".into(), floor));
    }
    if let Some(floor) = &agent.confidence_floor {
        floors.push((format!("/agents/{agent_index}/confidenceFloor"), floor));
    }
    if let Some((index, action)) = action {
        if let Some(floor) = &action.confidence_floor {
            floors.push((
                format!("/agents/{agent_index}/actionOverrides/{index}/confidenceFloor"),
                floor,
            ));
        }
    }
    for (pointer, floor) in floors {
        let violated = report.overall < floor.threshold;
        let effect = violated.then_some(match floor.on_violation {
            ConfidenceViolationAction::Reject => ViolationAction::Reject,
            ConfidenceViolationAction::EscalateToHuman => ViolationAction::EscalateToHuman,
        });
        if let Some(effect) = effect {
            result.restrict(effect);
        }
        result.observations.push(RuleObservation {
            pointer,
            rule_id: "confidenceFloor".into(),
            kind: "confidenceFloor".into(),
            outcome: format!(
                "reported {} {} threshold {}",
                report.overall,
                if violated { "below" } else { "meets" },
                floor.threshold
            ),
            action: effect,
            attribution: "agent".into(),
        });
    }
    result.confidence_report = Some(report);
}
/// Calculate cumulative confidence from observed reports without inventing history.
///
/// # Errors
/// Refuses an empty history or invalid report; callers own durable history.
pub fn cumulative_confidence(reports: &[ConfidenceReport]) -> Result<f64, String> {
    if reports.is_empty() {
        return Err("missing confidence history".into());
    }
    reports.iter().try_fold(1.0, |score, report| {
        report.validate()?;
        Ok(score * report.overall)
    })
}
