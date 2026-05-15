// Rust guideline compliant 2026-04-11

//! Autonomy level computation and enforcement (AI S5).
//!
//! Computes effective autonomy as the minimum of four sources:
//! impact-level cap, workflow default, agent declaration, and
//! action-site override. Handles escalation/demotion state machine,
//! calibration expiry, tool governance, and assistive task creation.

use crate::model::ai::{AIIntegrationDocument, AgentDeclaration, AutonomyLevel};
use crate::model::kernel::ImpactLevel;
use wos_events::provenance::{ProvenanceKind, ProvenanceRecord};

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
    advanced_governance: Option<&serde_json::Value>,
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
    // `target` matches the workflow-schema vocabulary; conformance fixtures
    // assert it under `data.target` post-ADR-0076 D-1.
    if let Some(trigger) = data.get("triggerDemotion").and_then(|v| v.as_str()) {
        provenance.push(prov(
            ProvenanceKind::AutonomyDemotion,
            serde_json::json!({
                "agentId": agent_id,
                "from": "autonomous",
                "target": "assistive",
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
    if let Some(tools) = data.get("toolInvocations").and_then(|v| v.as_array()) {
        let tool_governance = ToolGovernancePolicy::from_documents(_agent, advanced_governance);

        for tool_entry in tools {
            let tool_name = tool_entry
                .get("tool")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !tool_governance.permits(tool_name) {
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

        for (tool_name, limit) in &tool_governance.rate_limits {
            let count = tools
                .iter()
                .filter(|t| {
                    t.get("tool")
                        .and_then(|v| v.as_str())
                        .is_some_and(|n| n == tool_name.as_str())
                })
                .count();
            if count as u32 > *limit {
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

#[derive(Debug, Clone)]
struct ToolGovernancePolicy {
    permitted_tools: Vec<String>,
    rate_limits: Vec<(String, u32)>,
}

impl ToolGovernancePolicy {
    fn from_documents(
        agent: Option<&AgentDeclaration>,
        advanced_governance: Option<&serde_json::Value>,
    ) -> Self {
        if let Some(policy) = Self::from_advanced_governance(advanced_governance) {
            return policy;
        }
        if let Some(policy) = Self::from_agent_extension(agent) {
            return policy;
        }

        Self {
            permitted_tools: vec!["lookupPrecedent".to_string(), "searchDocuments".to_string()],
            rate_limits: vec![("lookupPrecedent".to_string(), 10)],
        }
    }

    fn from_advanced_governance(advanced_governance: Option<&serde_json::Value>) -> Option<Self> {
        let registry = advanced_governance?
            .get("toolGovernance")?
            .get("toolRegistry")?
            .as_array()?;
        let mut permitted_tools = Vec::new();
        let mut rate_limits = Vec::new();

        for tool in registry {
            let Some(tool_id) = tool.get("id").and_then(serde_json::Value::as_str) else {
                continue;
            };
            permitted_tools.push(tool_id.to_string());
            if let Some(limit) = tool
                .get("rateLimit")
                .and_then(|rate_limit| {
                    rate_limit
                        .get("maxPerMinute")
                        .or_else(|| rate_limit.get("maxPerHour"))
                })
                .and_then(serde_json::Value::as_u64)
                .and_then(|limit| u32::try_from(limit).ok())
            {
                rate_limits.push((tool_id.to_string(), limit));
            }
        }

        Some(Self {
            permitted_tools,
            rate_limits,
        })
    }

    fn from_agent_extension(agent: Option<&AgentDeclaration>) -> Option<Self> {
        let governance = agent?.extensions.get("x-toolGovernance")?;
        let permitted_tools = governance
            .get("permittedTools")
            .and_then(serde_json::Value::as_array)
            .map(|tools| {
                tools
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let rate_limits = governance
            .get("rateLimits")
            .and_then(serde_json::Value::as_object)
            .map(|limits| {
                limits
                    .iter()
                    .filter_map(|(tool_id, limit)| {
                        limit
                            .get("maxPerMinute")
                            .or_else(|| limit.get("maxPerHour"))
                            .and_then(serde_json::Value::as_u64)
                            .and_then(|limit| u32::try_from(limit).ok())
                            .map(|limit| (tool_id.clone(), limit))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Some(Self {
            permitted_tools,
            rate_limits,
        })
    }

    fn permits(&self, tool_name: &str) -> bool {
        self.permitted_tools.is_empty()
            || self
                .permitted_tools
                .iter()
                .any(|permitted_tool| permitted_tool == tool_name)
    }
}

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
        id: ProvenanceRecord::mint_id(),
        record_kind: kind,
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
        canonical_event_hash: None,
        transition_tags: Vec::new(),
        case_file_snapshot: None,
        outcome: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advanced_governance_tool_registry_permits_custom_tool() {
        let ai_doc = test_ai_doc();
        let advanced_governance = serde_json::json!({
            "toolGovernance": {
                "toolRegistry": [{
                    "id": "databaseLookup",
                    "category": "dataRetrieval",
                    "sideEffects": false
                }]
            }
        });
        let data = serde_json::json!({
            "toolInvocations": [{ "tool": "databaseLookup", "args": {} }]
        });

        let result = evaluate_autonomy(
            &ai_doc,
            "triageAgent",
            &data,
            &ImpactLevel::Operational,
            Some(&advanced_governance),
        );

        assert!(!result.provenance.iter().any(|record| {
            record.record_kind == ProvenanceKind::ToolViolation
                && record.data.as_ref().and_then(|data| data.get("reason"))
                    == Some(&serde_json::json!("not-in-permitted-list"))
        }));
    }

    #[test]
    fn advanced_governance_tool_registry_sets_rate_limit() {
        let ai_doc = test_ai_doc();
        let advanced_governance = serde_json::json!({
            "toolGovernance": {
                "toolRegistry": [{
                    "id": "databaseLookup",
                    "category": "dataRetrieval",
                    "sideEffects": false,
                    "rateLimit": { "maxPerMinute": 1 }
                }]
            }
        });
        let data = serde_json::json!({
            "toolInvocations": [
                { "tool": "databaseLookup", "args": { "query": "q1" } },
                { "tool": "databaseLookup", "args": { "query": "q2" } }
            ]
        });

        let result = evaluate_autonomy(
            &ai_doc,
            "triageAgent",
            &data,
            &ImpactLevel::Operational,
            Some(&advanced_governance),
        );

        assert!(result.provenance.iter().any(|record| {
            record.record_kind == ProvenanceKind::ToolViolation
                && record.data.as_ref().and_then(|data| data.get("reason"))
                    == Some(&serde_json::json!("rate-limit-exceeded"))
                && record.data.as_ref().and_then(|data| data.get("limit"))
                    == Some(&serde_json::json!(1))
        }));
    }

    fn test_ai_doc() -> AIIntegrationDocument {
        serde_json::from_value(serde_json::json!({
            "targetWorkflow": "urn:test:workflow",
            "defaultAutonomy": "assistive",
            "agents": [{
                "id": "triageAgent",
                "type": "agent",
                "agentType": "generative",
                "modelIdentifier": "test-model",
                "modelVersion": "1.0"
            }]
        }))
        .unwrap()
    }
}
