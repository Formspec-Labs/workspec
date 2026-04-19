// Rust guideline compliant 2026-02-21

//! Reference companion-policy evaluation for runtime event gating.

use std::collections::{HashMap, HashSet};

use wos_core::autonomy;
use wos_core::confidence;
use wos_core::deontic;
use wos_core::event_handler;
use wos_core::instance::CaseInstance;
use wos_core::model::ai::{AIIntegrationDocument, ViolationAction};
use wos_core::model::kernel::ImpactLevel;
use wos_core::provenance::{ProvenanceKind, ProvenanceRecord};

use crate::runtime::{CompanionPolicy, RuntimeError, RuntimeEventContext, RuntimeEventDecision};

/// Reference runtime policy for WOS companion documents.
#[derive(Debug, Clone, Default)]
pub struct ReferenceCompanionPolicy {
    ai_doc: Option<AIIntegrationDocument>,
    governance_json: Option<serde_json::Value>,
    companion_docs: HashMap<String, serde_json::Value>,
    dcr_executed_activities: Vec<String>,
    seen_idempotency_keys_by_instance: HashMap<String, HashSet<String>>,
}

impl ReferenceCompanionPolicy {
    /// Create a policy from resolved companion documents.
    pub fn new(
        ai_doc: Option<AIIntegrationDocument>,
        governance_json: Option<serde_json::Value>,
        companion_docs: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            ai_doc,
            governance_json,
            companion_docs,
            dcr_executed_activities: Vec::new(),
            seen_idempotency_keys_by_instance: HashMap::new(),
        }
    }
}

impl CompanionPolicy for ReferenceCompanionPolicy {
    fn evaluate_event(
        &mut self,
        context: RuntimeEventContext,
    ) -> Result<RuntimeEventDecision, RuntimeError> {
        let mut provenance = Vec::new();
        let mut event = context.event;
        let mut effective_event = Some(event.event.clone());
        let event_data = event.data.clone();
        let actor_id = event.actor_id.as_deref().unwrap_or("");
        let instance_id = context.instance.instance_id.clone();

        if let (Some(ai_doc), Some(data)) = (&self.ai_doc, &event_data) {
            if let Some(output) = data.get("output") {
                let impact_level = context
                    .kernel
                    .impact_level
                    .unwrap_or(ImpactLevel::Operational);
                let bypass = data
                    .get("deonticBypass")
                    .or_else(|| data.get("bypass"))
                    .and_then(|bypass| bypass.get("rationale"))
                    .and_then(serde_json::Value::as_str);
                let escalation_active = data
                    .get("escalationActive")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let invocation_source = data
                    .get("invocationSource")
                    .and_then(serde_json::Value::as_str);

                let deontic_result = deontic::evaluate_deontic_constraints(
                    ai_doc,
                    actor_id,
                    output,
                    &case_state_from_value(&context.instance),
                    &impact_level,
                    bypass,
                    escalation_active,
                    invocation_source,
                );
                provenance.extend(deontic_result.provenance);

                match deontic_result.effective_action {
                    Some(ViolationAction::EscalateToHuman) => {
                        effective_event = Some("escalated".to_string());
                    }
                    Some(ViolationAction::Reject) => {
                        effective_event = None;
                    }
                    _ => {}
                }
            }
        }

        if let (Some(ai_doc), Some(data)) = (&self.ai_doc, &event_data) {
            let is_agent_event = ai_doc.agents.iter().any(|agent| agent.id == actor_id);
            if is_agent_event {
                let impact_level = context
                    .kernel
                    .impact_level
                    .unwrap_or(ImpactLevel::Operational);
                let autonomy_result = autonomy::evaluate_autonomy(
                    ai_doc,
                    actor_id,
                    data,
                    &impact_level,
                    self.companion_docs.get("advanced"),
                );
                let autonomy_blocked = autonomy_result.provenance.iter().any(|record| {
                    matches!(
                        record.record_kind,
                        ProvenanceKind::AutonomyViolation | ProvenanceKind::ToolViolation
                    )
                });
                let mut autonomy_provenance = autonomy_result.provenance;
                if let Some(policy_ref) =
                    resolve_drift_demotion_policy_ref(actor_id, data, &self.companion_docs)
                {
                    annotate_autonomy_demotion_policy_ref(&mut autonomy_provenance, &policy_ref);
                }
                provenance.extend(autonomy_provenance);

                let confidence_result = confidence::evaluate_confidence(ai_doc, actor_id, data);
                if confidence_result.requires_escalation && effective_event.is_some() {
                    effective_event = Some("escalated".to_string());
                }
                provenance.extend(confidence_result.provenance);

                if autonomy_blocked {
                    effective_event = None;
                }
            } else {
                provenance.extend(confidence::evaluate_review_ground_truth(data, actor_id));
            }
        }

        let data = event_data
            .clone()
            .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
        let is_agent = self
            .ai_doc
            .as_ref()
            .is_some_and(|ai_doc| ai_doc.agents.iter().any(|agent| agent.id == actor_id));
        let handler_result = event_handler::evaluate_event(
            &event.event,
            actor_id,
            &data,
            is_agent,
            self.governance_json.as_ref(),
            &self.companion_docs,
            self.seen_idempotency_keys_by_instance
                .entry(instance_id)
                .or_default(),
        );
        if handler_result.requires_escalation {
            effective_event = Some("escalated".to_string());
        }
        if handler_result.blocked {
            effective_event = None;
        }
        provenance.extend(handler_result.provenance);

        self.evaluate_constraint_zone_activity(&event.event, &data, &mut provenance);

        let event = effective_event.map(|event_name| {
            event.event = event_name;
            event
        });
        Ok(RuntimeEventDecision { event, provenance })
    }
}

fn resolve_drift_demotion_policy_ref(
    actor_id: &str,
    data: &serde_json::Value,
    companion_docs: &HashMap<String, serde_json::Value>,
) -> Option<String> {
    let policy_ref = data
        .get("driftAlert")
        .and_then(|drift| drift.get("policyRef"))
        .and_then(serde_json::Value::as_str)?;
    if !drift_monitor_declares_policy_ref(actor_id, policy_ref, companion_docs) {
        return None;
    }
    if !agent_config_declares_demotion_rule(actor_id, policy_ref, companion_docs) {
        return None;
    }
    Some(policy_ref.to_string())
}

fn drift_monitor_declares_policy_ref(
    actor_id: &str,
    policy_ref: &str,
    companion_docs: &HashMap<String, serde_json::Value>,
) -> bool {
    companion_docs.values().any(|doc| {
        doc.get("$wosDriftMonitor").is_some()
            && doc
                .get("monitors")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|monitors| {
                    monitors.iter().any(|monitor| {
                        monitor.get("agentRef").and_then(serde_json::Value::as_str)
                            == Some(actor_id)
                            && monitor
                                .get("alertThresholds")
                                .and_then(serde_json::Value::as_array)
                                .is_some_and(|thresholds| {
                                    thresholds.iter().any(|threshold| {
                                        threshold
                                            .get("policyRef")
                                            .and_then(serde_json::Value::as_str)
                                            == Some(policy_ref)
                                    })
                                })
                    })
                })
    })
}

fn agent_config_declares_demotion_rule(
    actor_id: &str,
    policy_ref: &str,
    companion_docs: &HashMap<String, serde_json::Value>,
) -> bool {
    companion_docs.values().any(|doc| {
        doc.get("$wosAgentConfig").is_some()
            && doc.get("targetAgent").and_then(serde_json::Value::as_str) == Some(actor_id)
            && doc
                .pointer("/autonomyPolicy/demotion")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|rules| {
                    rules.iter().any(|rule| {
                        rule.get("id").and_then(serde_json::Value::as_str) == Some(policy_ref)
                    })
                })
    })
}

fn annotate_autonomy_demotion_policy_ref(records: &mut [ProvenanceRecord], policy_ref: &str) {
    for record in records {
        if record.record_kind != ProvenanceKind::AutonomyDemotion {
            continue;
        }
        let data = record
            .data
            .get_or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        let Some(object) = data.as_object_mut() else {
            continue;
        };
        object.insert(
            "policyRef".to_string(),
            serde_json::Value::String(policy_ref.to_string()),
        );
        object.insert(
            "demotionRuleId".to_string(),
            serde_json::Value::String(policy_ref.to_string()),
        );
    }
}

impl ReferenceCompanionPolicy {
    fn evaluate_constraint_zone_activity(
        &mut self,
        event_name: &str,
        data: &serde_json::Value,
        provenance: &mut Vec<ProvenanceRecord>,
    ) {
        if event_name != "zoneAction" {
            return;
        }
        let Some(activity) = data.get("activity").and_then(serde_json::Value::as_str) else {
            return;
        };
        self.dcr_executed_activities.push(activity.to_string());

        let Some(zones) = self
            .companion_docs
            .get("advanced")
            .and_then(|advanced| advanced.get("constraintZones"))
            .and_then(serde_json::Value::as_array)
        else {
            return;
        };

        for zone in zones {
            if let Some(relations) = zone.get("relations").and_then(serde_json::Value::as_array) {
                self.evaluate_dcr_resolution(activity, relations, provenance);
            }
            if let Some(activities) = zone.get("activities").and_then(serde_json::Value::as_array) {
                self.evaluate_zone_satisfaction(zone, activities, provenance);
            }
        }
    }

    fn evaluate_dcr_resolution(
        &self,
        activity: &str,
        relations: &[serde_json::Value],
        provenance: &mut Vec<ProvenanceRecord>,
    ) {
        for relation in relations {
            let excludes_pending_activity =
                relation.get("type").and_then(serde_json::Value::as_str) == Some("exclude")
                    && relation.get("source").and_then(serde_json::Value::as_str) == Some(activity);
            if !excludes_pending_activity {
                continue;
            }

            let target = relation
                .get("target")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let target_has_pending_response = relations.iter().any(|candidate| {
                candidate.get("type").and_then(serde_json::Value::as_str) == Some("response")
                    && candidate.get("source").and_then(serde_json::Value::as_str) == Some(target)
                    && !self.dcr_executed_activities.iter().any(|executed| {
                        candidate
                            .get("target")
                            .and_then(serde_json::Value::as_str)
                            .is_some_and(|response_target| response_target == executed)
                    })
            });
            if target_has_pending_response {
                provenance.push(ProvenanceRecord {
                    record_kind: ProvenanceKind::DcrResolutionError,
                    timestamp: String::new(),
                    actor_id: None,
                    from_state: None,
                    to_state: None,
                    event: None,
                    data: Some(serde_json::json!({
                        "activity": target,
                        "reason": "excluded-while-pending",
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

    fn evaluate_zone_satisfaction(
        &self,
        zone: &serde_json::Value,
        activities: &[serde_json::Value],
        provenance: &mut Vec<ProvenanceRecord>,
    ) {
        let pending: Vec<&str> = activities
            .iter()
            .filter(|activity| {
                activity
                    .get("initialPending")
                    .and_then(serde_json::Value::as_bool)
                    == Some(true)
            })
            .filter_map(|activity| activity.get("id").and_then(serde_json::Value::as_str))
            .collect();
        let all_pending_done = pending.iter().all(|pending_activity| {
            self.dcr_executed_activities
                .iter()
                .any(|executed| executed == pending_activity)
        });
        if !all_pending_done || pending.is_empty() {
            return;
        }
        if provenance
            .iter()
            .any(|record| record.record_kind == ProvenanceKind::ZoneSatisfied)
        {
            return;
        }
        let zone_id = zone
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        provenance.push(ProvenanceRecord {
            record_kind: ProvenanceKind::ZoneSatisfied,
            timestamp: String::new(),
            actor_id: None,
            from_state: None,
            to_state: None,
            event: None,
            data: Some(serde_json::json!({ "zoneId": zone_id })),
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

fn case_state_from_value(instance: &CaseInstance) -> HashMap<String, serde_json::Value> {
    instance
        .case_state
        .as_object()
        .map(|object| {
            object
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        })
        .unwrap_or_default()
}
