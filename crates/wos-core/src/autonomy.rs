// Rust guideline compliant 2026-02-21

//! Authored autonomy ceilings and processor-owned operational calculations.
//!
//! No event or model field supplies operational authority. These calculations
//! consume explicit processor facts; they do not implement authorization,
//! calibration, review or durable storage operations.

use crate::ai_policy::{AiPolicyError, NativeAiPolicy};
use crate::model::ai::AutonomyLevel;
use crate::model::kernel::ImpactLevel;
use std::collections::BTreeMap;

/// Trusted calibration facts supplied by the maintained operation.
#[derive(Debug, Clone)]
pub struct CalibrationEvidence {
    /// Trusted completion time from the processor clock.
    pub completed_at_ms: u64,
    /// Trusted calibration expiry, exclusive.
    pub expires_at_ms: u64,
}
/// Trusted approval facts; ordinary events cannot construct runtime authority.
#[derive(Debug, Clone)]
pub struct ApprovalEvidence {
    /// Admitted revision for this approval.
    pub workflow_url: String,
    /// Admitted revision version.
    pub workflow_version: String,
    /// Selected native agent.
    pub agent_id: String,
    /// Exact selected escalation rule occurrence.
    pub rule_pointer: String,
    /// Authenticated human actor slot verified by the owning operation.
    pub approver_ref: String,
    /// Trusted approval time.
    pub approved_at_ms: u64,
    /// Exclusive expiry from the owning clock.
    pub expires_at_ms: u64,
}
/// Processor-owned retained state; not an authored or deserializable value.
#[derive(Debug, Clone, Default)]
pub struct AgentOperationalState {
    current: Option<AutonomyLevel>,
    reductions: BTreeMap<String, AutonomyLevel>,
    last_demotion_ms: Option<u64>,
    /// Trusted optional calibration facts.
    pub calibration: Option<CalibrationEvidence>,
    /// Trusted pending recalibration reduction.
    pub pending_recalibration: bool,
    /// Trusted optional approval facts; caller owns authenticated resolution.
    pub approval: Option<ApprovalEvidence>,
}
impl AgentOperationalState {
    /// Create uninitialized processor state without favorable invented facts.
    pub fn new() -> Self {
        Self::default()
    }
    /// Read the retained agent level, excluding transient action restrictions.
    pub fn current(&self) -> Option<AutonomyLevel> {
        self.current
    }
    /// Retain a real operational reduction supplied by its owning operation.
    pub fn reduce(&mut self, key: impl Into<String>, level: AutonomyLevel, now_ms: u64) {
        self.reductions.insert(key.into(), level);
        if self.current.is_none_or(|old| level < old) {
            self.current = Some(level);
            self.last_demotion_ms = Some(now_ms);
        }
    }
    /// Clear one resolved reduction without promoting the retained agent level.
    pub fn clear_reduction(&mut self, key: &str) {
        self.reductions.remove(key);
    }
}
/// Observed calculation; it establishes neither review nor invocation readiness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutonomyObservation {
    /// Authored limit for this site.
    pub authored_ceiling: AutonomyLevel,
    /// Retained agent level independent of the selected site.
    pub retained_level: AutonomyLevel,
    /// Effective local level after all active restrictions.
    pub effective_level: AutonomyLevel,
    /// Whether active trusted approval affected this calculation.
    pub approval_active: bool,
}
/// Compute a level using admitted policy and explicit processor-owned state.
///
/// # Errors
/// Refuses unknown actors/actions and invalid or mismatched trusted facts.
/// The caller must establish operational authority; runtime event fields do not.
pub fn evaluate_autonomy(
    policy: &NativeAiPolicy,
    agent_id: &str,
    action_ref: Option<&str>,
    requested: Option<AutonomyLevel>,
    state: &mut AgentOperationalState,
    now_ms: u64,
) -> Result<AutonomyObservation, AiPolicyError> {
    let agent = policy
        .agent(agent_id)
        .ok_or_else(|| AiPolicyError::new("/agents", "unknown admitted agent"))?;
    let impact = if matches!(
        policy.impact_level(),
        ImpactLevel::RightsImpacting | ImpactLevel::SafetyImpacting
    ) {
        AutonomyLevel::Assistive
    } else {
        AutonomyLevel::Autonomous
    };
    let workflow = policy
        .oversight()
        .default_autonomy
        .unwrap_or(AutonomyLevel::Assistive);
    let ceiling = workflow.min(impact).min(
        agent
            .autonomy_policy
            .as_ref()
            .and_then(|v| v.max_autonomy)
            .unwrap_or(AutonomyLevel::Autonomous),
    );
    let action = if let Some(pointer) = action_ref {
        policy.action(pointer)?;
        agent
            .action_overrides
            .iter()
            .find(|v| v.action_ref == pointer)
    } else {
        if !agent.action_overrides.is_empty() {
            return Err(AiPolicyError::new(
                "/agents",
                "admitted action context required",
            ));
        }
        None
    };
    let authored = ceiling.min(
        action
            .and_then(|v| v.autonomy)
            .unwrap_or(AutonomyLevel::Autonomous),
    );
    let mut operational = ceiling;
    for reduction in state.reductions.values() {
        operational = operational.min(*reduction);
    }
    if state.pending_recalibration {
        operational = operational.min(AutonomyLevel::Assistive);
    }
    if agent.calibration.as_ref().is_some_and(|v| v.required) {
        if let Some(calibration) = &state.calibration {
            if calibration.completed_at_ms > calibration.expires_at_ms
                || calibration.completed_at_ms > now_ms
            {
                return Err(AiPolicyError::new(
                    "/agents",
                    "invalid processor calibration clock facts",
                ));
            }
        }
        if state
            .calibration
            .as_ref()
            .is_none_or(|v| now_ms >= v.expires_at_ms)
        {
            operational = operational.min(AutonomyLevel::Assistive);
        }
    }
    let old = state.current.unwrap_or(ceiling);
    let retained = old.min(operational);
    if retained < old {
        state.last_demotion_ms = Some(now_ms);
    }
    state.current = Some(retained);
    let mut approval_active = false;
    let mut current = retained;
    if let Some(approval) = &state.approval {
        if approval.workflow_url != policy.workflow_url()
            || approval.workflow_version != policy.workflow_version()
            || approval.agent_id != agent_id
            || approval.approved_at_ms > now_ms
            || approval.expires_at_ms <= approval.approved_at_ms
        {
            return Err(AiPolicyError::new(
                "/agents",
                "mismatched or invalid processor approval facts",
            ));
        }
        let agent_index = policy
            .agents()
            .iter()
            .position(|v| v.id == agent_id)
            .expect("agent belongs to policy");
        let rule = agent
            .autonomy_policy
            .as_ref()
            .and_then(|v| {
                v.escalation.iter().enumerate().find(|(index, _)| {
                    approval.rule_pointer
                        == format!("/agents/{agent_index}/autonomyPolicy/escalation/{index}")
                })
            })
            .map(|(_, v)| v)
            .ok_or_else(|| {
                AiPolicyError::new(
                    &approval.rule_pointer,
                    "approval does not select a native escalation rule",
                )
            })?;
        if rule.required_approver_ref != approval.approver_ref {
            return Err(AiPolicyError::new(
                &approval.rule_pointer,
                "approval actor slot mismatch",
            ));
        }
        if now_ms < approval.expires_at_ms
            && state
                .last_demotion_ms
                .is_none_or(|time| approval.approved_at_ms > time)
        {
            approval_active = true;
            current = current.max(rule.target_level.min(operational));
        }
    }
    Ok(AutonomyObservation {
        authored_ceiling: authored,
        retained_level: retained,
        effective_level: current
            .min(authored)
            .min(requested.unwrap_or(AutonomyLevel::Autonomous)),
        approval_active,
    })
}
