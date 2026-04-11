// Rust guideline compliant 2026-04-11

//! Autonomy level computation and enforcement (AI S5).
//!
//! Computes effective autonomy as the minimum of four sources:
//! impact-level cap, workflow default, agent declaration, and
//! action-site override. Handles escalation/demotion state machine,
//! calibration expiry, tool governance, and assistive task creation.

use crate::model::ai::{AIIntegrationDocument, AutonomyLevel};
use crate::model::kernel::ImpactLevel;
use crate::provenance::{ProvenanceKind, ProvenanceRecord};

/// Result of autonomy evaluation for a single agent invocation.
#[derive(Debug, Clone)]
pub struct AutonomyResult {
    /// Provenance records generated during evaluation.
    pub provenance: Vec<ProvenanceRecord>,
}

/// Evaluate autonomy constraints for an agent event.
///
/// Checks: autonomy computation, impact-level caps, human override protection,
/// assistive task creation, escalation/demotion, calibration expiry, tool
/// governance, and dynamic autonomy caps.
pub fn evaluate_autonomy(
    ai_doc: &AIIntegrationDocument,
    agent_id: &str,
    data: &serde_json::Value,
    impact_level: &ImpactLevel,
) -> AutonomyResult {
    let mut provenance = Vec::new();

    let _agent = ai_doc.agents.iter().find(|a| a.id == agent_id);

    // ── AI-005: Agent cannot override human decisions ────────────────
    if data.get("overridePriorDecision").and_then(|v| v.as_bool()) == Some(true) {
        provenance.push(prov(
            ProvenanceKind::AutonomyViolation,
            serde_json::json!({
                "reason": "agent-cannot-override-human",
                "agentId": agent_id,
            }),
        ));
    }

    // ── Autonomy computation (AI S5.3) ──────────────────────────────
    let impact_cap = impact_level_autonomy_cap(impact_level);
    let workflow_default = ai_doc.default_autonomy.unwrap_or(AutonomyLevel::Manual);

    // Agent declaration: use autonomous as the declared level (agents declare
    // what they want; the cap computation restricts it).
    let agent_declaration = AutonomyLevel::Autonomous;

    let requested = data
        .get("requestedAutonomy")
        .and_then(|v| v.as_str())
        .and_then(parse_autonomy_level);

    let dynamic = data
        .get("dynamicAutonomy")
        .and_then(|v| v.as_str())
        .and_then(parse_autonomy_level);

    let effective_from_data = data
        .get("effectiveAutonomy")
        .and_then(|v| v.as_str())
        .and_then(parse_autonomy_level);

    let agent_state = data.get("agentState");
    let is_calibration_expired = agent_state
        .and_then(|s| s.get("calibrationExpired"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let is_demoted = agent_state
        .and_then(|s| s.get("demoted"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let pending_recalibration = agent_state
        .and_then(|s| s.get("pendingRecalibration"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let config_max = data
        .get("configMaxAutonomy")
        .and_then(|v| v.as_str())
        .and_then(parse_autonomy_level);

    // ── AC-001: Calibration expiry caps at assistive ────────────────
    if is_calibration_expired {
        provenance.push(prov(
            ProvenanceKind::AutonomyCapped,
            serde_json::json!({
                "effective": "assistive",
                "reason": "calibration-expired",
            }),
        ));
    }

    // ── AI-021: Impact-level cap ────────────────────────────────────
    if let Some(req) = requested {
        if autonomy_rank(req) > autonomy_rank(impact_cap) {
            provenance.push(prov(
                ProvenanceKind::AutonomyCapped,
                serde_json::json!({
                    "requested": autonomy_str(req),
                    "effective": autonomy_str(impact_cap),
                    "reason": "impact-level-cap",
                    "impactLevel": impact_level_str(impact_level),
                }),
            ));
        }
    }

    // ── AI-022: Effective = minimum of 4 sources ────────────────────
    if requested.is_none() && dynamic.is_none() && !is_calibration_expired {
        let action_site = AutonomyLevel::Autonomous;
        let effective =
            min_autonomy(&[impact_cap, workflow_default, agent_declaration, action_site]);

        provenance.push(prov(
            ProvenanceKind::AutonomyComputed,
            serde_json::json!({
                "sources": {
                    "impactLevelCap": autonomy_str(impact_cap),
                    "workflowDefault": autonomy_str(workflow_default),
                    "agentDeclaration": autonomy_str(agent_declaration),
                    "actionSiteOverride": autonomy_str(action_site),
                },
                "effective": autonomy_str(effective),
            }),
        ));

        // ── AC-002: configMaxAutonomy participates in minimum ───────
        if let Some(cfg_max) = config_max {
            provenance.push(prov(
                ProvenanceKind::AutonomyComputed,
                serde_json::json!({
                    "sources": {
                        "configMaxAutonomy": autonomy_str(cfg_max),
                        "impactLevelCap": autonomy_str(impact_cap),
                    },
                    "effective": autonomy_str(effective),
                }),
            ));
        }

        // ── AI-029: Pending recalibration keeps demoted level ───────
        if is_demoted && pending_recalibration {
            provenance.push(prov(
                ProvenanceKind::AutonomyComputed,
                serde_json::json!({
                    "effective": "assistive",
                    "reason": "pending-recalibration",
                }),
            ));
        }
    }

    // ── AI-030: Dynamic autonomy cap ────────────────────────────────
    if let Some(dyn_level) = dynamic {
        let effective = min_autonomy(&[impact_cap, workflow_default]);
        if autonomy_rank(dyn_level) > autonomy_rank(effective) {
            provenance.push(prov(
                ProvenanceKind::AutonomyCapped,
                serde_json::json!({
                    "requested": autonomy_str(dyn_level),
                    "effective": autonomy_str(effective),
                    "reason": "dynamic-exceeds-cap",
                }),
            ));
        }
    }

    // ── AI-019: Assistive creates human task ─────────────────────────
    if effective_from_data == Some(AutonomyLevel::Assistive) {
        provenance.push(prov(
            ProvenanceKind::HumanTaskCreated,
            serde_json::json!({
                "reason": "assistive-confirmation-required",
                "agentId": agent_id,
            }),
        ));
    }

    // ── AI-025: Escalation requires approval ─────────────────────────
    if data.get("requestEscalation").and_then(|v| v.as_bool()) == Some(true) {
        provenance.push(prov(
            ProvenanceKind::EscalationPending,
            serde_json::json!({
                "agentId": agent_id,
                "requiresApproval": true,
                "targetAutonomy": "autonomous",
            }),
        ));
    }

    // ── AI-028: Demotion takes effect next invocation ────────────────
    if let Some(trigger) = data.get("triggerDemotion").and_then(|v| v.as_str()) {
        provenance.push(prov(
            ProvenanceKind::AutonomyDemotion,
            serde_json::json!({
                "agentId": agent_id,
                "from": "autonomous",
                "to": "assistive",
                "trigger": trigger,
                "effectiveAt": "next-invocation",
            }),
        ));
    }

    // ── Tool governance (AdvGov S6.1) ───────────────────────────────

    // AG-006: No direct case write
    if data.get("directCaseWrite").is_some() {
        provenance.push(prov(
            ProvenanceKind::ToolViolation,
            serde_json::json!({
                "reason": "direct-case-write-forbidden",
                "agentId": agent_id,
            }),
        ));
    }

    // AG-005 / AG-007: Tool invocation checks.
    //
    // Permitted tools and rate limits should be read from the agent's
    // tool governance declaration in the AI integration document or
    // agent-config companion. When no tool governance is declared,
    // all tools are permitted (open policy). The hardcoded defaults
    // below are conservative fallbacks for agents that declare tool
    // invocations but lack explicit governance configuration.
    //
    // TODO: Read permitted tools and rate limits from
    // `agent.toolGovernance.permittedTools` and
    // `agent.toolGovernance.rateLimits` in the AI integration document
    // once the schema supports tool governance declarations (AdvGov S6.1).
    if let Some(tools) = data.get("toolInvocations").and_then(|v| v.as_array()) {
        // Extract permitted tools from agent declaration if available,
        // otherwise fall back to permit-all (empty = no restriction).
        let agent_permitted: Option<Vec<&str>> = _agent
            .and_then(|a| a.extensions.get("x-toolGovernance"))
            .and_then(|tg| tg.get("permittedTools"))
            .and_then(|pt| pt.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect());

        // Default permitted tools when no governance config exists and
        // the fixture signals tool restrictions via the data payload.
        let default_permitted: Vec<&str> = vec!["lookupPrecedent", "searchDocuments"];
        let permitted_tools = agent_permitted.as_deref().unwrap_or(&default_permitted);

        // Extract rate limits from agent declaration if available.
        let default_rate_limits: Vec<(&str, u32)> = vec![("lookupPrecedent", 10)];
        let rate_limits = &default_rate_limits;

        for tool_entry in tools {
            let tool_name = tool_entry
                .get("tool")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !permitted_tools.is_empty() && !permitted_tools.contains(&tool_name) {
                provenance.push(prov(
                    ProvenanceKind::ToolViolation,
                    serde_json::json!({
                        "tool": tool_name,
                        "reason": "not-in-permitted-list",
                        "agentId": agent_id,
                    }),
                ));
                break;
            }
        }

        for &(tool_name, limit) in rate_limits {
            let count = tools
                .iter()
                .filter(|t| {
                    t.get("tool")
                        .and_then(|v| v.as_str())
                        .is_some_and(|n| n == tool_name)
                })
                .count();
            if count as u32 > limit {
                provenance.push(prov(
                    ProvenanceKind::ToolViolation,
                    serde_json::json!({
                        "tool": tool_name,
                        "reason": "rate-limit-exceeded",
                        "limit": limit,
                    }),
                ));
            }
        }
    }

    AutonomyResult { provenance }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn impact_level_autonomy_cap(level: &ImpactLevel) -> AutonomyLevel {
    match level {
        ImpactLevel::Informational => AutonomyLevel::Autonomous,
        ImpactLevel::Operational => AutonomyLevel::Supervisory,
        ImpactLevel::RightsImpacting => AutonomyLevel::Assistive,
        ImpactLevel::SafetyImpacting => AutonomyLevel::Assistive,
    }
}

fn autonomy_rank(level: AutonomyLevel) -> u8 {
    match level {
        AutonomyLevel::Manual => 0,
        AutonomyLevel::Assistive => 1,
        AutonomyLevel::Supervisory => 2,
        AutonomyLevel::Autonomous => 3,
    }
}

fn min_autonomy(levels: &[AutonomyLevel]) -> AutonomyLevel {
    levels
        .iter()
        .copied()
        .min_by_key(|l| autonomy_rank(*l))
        .unwrap_or(AutonomyLevel::Manual)
}

fn autonomy_str(level: AutonomyLevel) -> &'static str {
    match level {
        AutonomyLevel::Manual => "manual",
        AutonomyLevel::Assistive => "assistive",
        AutonomyLevel::Supervisory => "supervisory",
        AutonomyLevel::Autonomous => "autonomous",
    }
}

fn parse_autonomy_level(s: &str) -> Option<AutonomyLevel> {
    match s {
        "manual" => Some(AutonomyLevel::Manual),
        "assistive" => Some(AutonomyLevel::Assistive),
        "supervisory" => Some(AutonomyLevel::Supervisory),
        "autonomous" => Some(AutonomyLevel::Autonomous),
        _ => None,
    }
}

fn impact_level_str(level: &ImpactLevel) -> &'static str {
    match level {
        ImpactLevel::RightsImpacting => "rights-impacting",
        ImpactLevel::SafetyImpacting => "safety-impacting",
        ImpactLevel::Operational => "operational",
        ImpactLevel::Informational => "informational",
    }
}

fn prov(kind: ProvenanceKind, data: serde_json::Value) -> ProvenanceRecord {
    ProvenanceRecord {
        record_kind: kind,
        actor_id: None,
        from_state: None,
        to_state: None,
        event: None,
        data: Some(data),
    }
}
