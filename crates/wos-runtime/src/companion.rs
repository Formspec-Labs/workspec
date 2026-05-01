// Rust guideline compliant 2026-02-21

//! Reference companion-policy evaluation for runtime event gating.

use std::collections::{BTreeMap, HashMap, HashSet};

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use wos_core::autonomy;
use wos_core::confidence;
use wos_core::deontic;
use wos_core::eval::parse_iso_duration_to_ms;
use wos_core::event_handler::{self, AdverseDecisionNoticeInput};
use wos_core::instance::CaseInstance;
use wos_core::model::ai::{AIIntegrationDocument, ViolationAction};
use wos_core::model::kernel::{ImpactLevel, KernelDocument, State, Transition, TransitionEvent};
use wos_core::provenance::{CaseFileSnapshot, ProvenanceKind, ProvenanceRecord};

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
        let adverse_notice = deterministic_adverse_decision_notice_input(
            &context.kernel,
            &context.instance,
            &event.event,
            context.now_ms,
            self.governance_json.as_ref(),
            &self.companion_docs,
        );
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
            adverse_notice.as_ref(),
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

fn deterministic_adverse_decision_notice_input(
    kernel: &KernelDocument,
    instance: &CaseInstance,
    event_name: &str,
    now_ms: u64,
    governance: Option<&serde_json::Value>,
    companion_docs: &HashMap<String, serde_json::Value>,
) -> Option<AdverseDecisionNoticeInput> {
    let policy = governance?
        .pointer("/dueProcess/adverseDecisionPolicy")
        .filter(|policy| {
            policy
                .get("noticeRequired")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
        })?;
    let (from_state, transition) = active_adverse_transition(kernel, instance, event_name)?;
    let snapshot = CaseFileSnapshot::from_case_state(&instance.case_state);
    let timing = policy
        .get("noticeTiming")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("beforeEffective")
        .to_string();
    let grace_period = policy
        .get("noticeGracePeriod")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let template_key = policy
        .get("noticeTemplateKey")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let appeal = appeal_details(policy);
    let render_context = notice_render_context(
        instance,
        &snapshot,
        transition,
        grace_period.as_deref(),
        &appeal,
        now_ms,
    );
    let resolution = resolve_notice_template(
        companion_docs,
        kernel.url.as_deref(),
        template_key.as_deref(),
    );
    let (template, resolved_template_key, template_resolution_source) = match &resolution {
        NoticeTemplateResolution::Explicit { key, template } => {
            (Some(*template), Some(key.clone()), "explicit")
        }
        NoticeTemplateResolution::CategoryFallback { key, template } => {
            (Some(*template), Some(key.clone()), "categoryFallback")
        }
        NoticeTemplateResolution::NotFound => (None, None, "notFound"),
    };
    let human_readable = render_human_notice(template, &render_context, transition);

    Some(AdverseDecisionNoticeInput {
        from_state: from_state.to_string(),
        to_state: transition.target.clone(),
        event: event_name.to_string(),
        transition_tags: transition.tags.clone(),
        case_file_snapshot: snapshot,
        timing,
        grace_period,
        resolved_template_key,
        template_resolution_source: template_resolution_source.to_string(),
        template_key,
        human_readable,
        appeal,
    })
}

fn active_adverse_transition<'a>(
    kernel: &'a KernelDocument,
    instance: &CaseInstance,
    event_name: &str,
) -> Option<(String, &'a Transition)> {
    for active_state in &instance.configuration {
        let Some(state) = find_state(kernel, active_state) else {
            continue;
        };
        if let Some(transition) = state.transitions.iter().find(|transition| {
            transition
                .event
                .as_ref()
                .is_some_and(|ev| ev.matches_runtime_dispatch(event_name))
                && transition.tags.iter().any(|tag| tag == "adverse-decision")
        }) {
            return Some((active_state.clone(), transition));
        }
    }
    None
}

fn find_state<'a>(kernel: &'a KernelDocument, state_id: &str) -> Option<&'a State> {
    for (id, state) in &kernel.lifecycle.states {
        if id == state_id {
            return Some(state);
        }
        if let Some(found) = find_nested_state(state, state_id) {
            return Some(found);
        }
    }
    None
}

fn find_nested_state<'a>(state: &'a State, state_id: &str) -> Option<&'a State> {
    for (id, child) in &state.states {
        if id == state_id {
            return Some(child);
        }
        if let Some(found) = find_nested_state(child, state_id) {
            return Some(found);
        }
    }
    for region in state.regions.values() {
        for (id, child) in &region.states {
            if id == state_id {
                return Some(child);
            }
            if let Some(found) = find_nested_state(child, state_id) {
                return Some(found);
            }
        }
    }
    None
}

fn appeal_details(policy: &serde_json::Value) -> serde_json::Value {
    let appeal = policy
        .get("appealMechanism")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    serde_json::json!({
        "enabled": appeal.get("enabled").and_then(serde_json::Value::as_bool).unwrap_or(false),
        "appealWindow": appeal.get("appealWindow").and_then(serde_json::Value::as_str),
        "reviewerConstraint": appeal.get("reviewerConstraint").and_then(serde_json::Value::as_str),
        "reviewerRoles": appeal.get("reviewerRoles").cloned().unwrap_or_else(|| serde_json::json!([])),
        "continuationOfServices": appeal.get("continuationOfServices").and_then(serde_json::Value::as_bool).unwrap_or(false),
        "continuationScope": appeal.get("continuationScope").and_then(serde_json::Value::as_str),
        "continuationPolicyRef": appeal.get("continuationPolicyRef").and_then(serde_json::Value::as_str),
    })
}

/// Outcome of notice-template resolution. Surfaces whether the processor
/// used the explicit `noticeTemplateKey` key, fell back to the first
/// category=`adverse-decision` template, or could not find one at all.
/// Callers propagate this into the `noticeSent` provenance so that a
/// fallback selection is auditable rather than silent (Review Finding 4).
enum NoticeTemplateResolution<'a> {
    Explicit {
        key: String,
        template: &'a serde_json::Value,
    },
    CategoryFallback {
        key: String,
        template: &'a serde_json::Value,
    },
    NotFound,
}

fn resolve_notice_template<'a>(
    companion_docs: &'a HashMap<String, serde_json::Value>,
    target_workflow: Option<&str>,
    template_key: Option<&str>,
) -> NoticeTemplateResolution<'a> {
    let mut docs = companion_docs.iter().collect::<Vec<_>>();
    docs.sort_by(|(left, _), (right, _)| left.cmp(right));
    for (_, doc) in docs {
        if doc.get("notifications").is_none() || doc.get("$wosDelivery").is_none() {
            continue;
        }
        if let Some(target) = target_workflow {
            if doc
                .get("targetWorkflow")
                .and_then(serde_json::Value::as_str)
                != Some(target)
            {
                continue;
            }
        }
        let Some(templates) = doc.get("templates").and_then(serde_json::Value::as_object) else {
            continue;
        };
        if let Some(template_key) = template_key {
            if let Some(template) = templates.get(template_key) {
                return NoticeTemplateResolution::Explicit {
                    key: template_key.to_string(),
                    template,
                };
            }
        } else if let Some((key, template)) = templates.iter().find(|(_, template)| {
            template.get("category").and_then(serde_json::Value::as_str) == Some("adverse-decision")
        }) {
            return NoticeTemplateResolution::CategoryFallback {
                key: key.clone(),
                template,
            };
        }
    }
    NoticeTemplateResolution::NotFound
}

fn notice_render_context(
    instance: &CaseInstance,
    snapshot: &CaseFileSnapshot,
    transition: &Transition,
    grace_period: Option<&str>,
    appeal: &serde_json::Value,
    now_ms: u64,
) -> BTreeMap<String, String> {
    let mut context = BTreeMap::new();
    context.insert("caseId".to_string(), instance.instance_id.clone());
    context.insert(
        "decisionEvent".to_string(),
        transition
            .event
            .as_ref()
            .map(TransitionEvent::authoring_display_label)
            .unwrap_or_else(|| "(none)".to_string()),
    );
    context.insert("determination".to_string(), transition.target.clone());
    context.insert("snapshotSha256".to_string(), snapshot.sha256.clone());
    context.insert(
        "noticeGracePeriod".to_string(),
        grace_period.unwrap_or("not specified").to_string(),
    );
    context.insert(
        "appealWindow".to_string(),
        appeal
            .get("appealWindow")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("not specified")
            .to_string(),
    );
    context.insert(
        "appealDeadline".to_string(),
        appeal
            .get("appealWindow")
            .and_then(serde_json::Value::as_str)
            .and_then(|window| deadline_from_window(now_ms, window))
            .unwrap_or_else(|| "unavailable".to_string()),
    );
    context.insert("appealBody".to_string(), appeal_body(appeal));

    if let Some(case_fields) = instance.case_state.as_object() {
        for (key, value) in case_fields {
            context.insert(key.clone(), render_value(value));
        }
    }
    context
}

fn deadline_from_window(now_ms: u64, window: &str) -> Option<String> {
    let delta_ms = parse_iso_duration_to_ms(window).ok()?;
    let deadline_ms = now_ms.checked_add(delta_ms)?;
    let nanos_i128 = i128::from(deadline_ms) * 1_000_000;
    let timestamp = OffsetDateTime::from_unix_timestamp_nanos(nanos_i128).ok()?;
    timestamp.format(&Rfc3339).ok()
}

fn appeal_body(appeal: &serde_json::Value) -> String {
    let Some(roles) = appeal
        .get("reviewerRoles")
        .and_then(serde_json::Value::as_array)
    else {
        return "the appeal review body".to_string();
    };
    let rendered = roles
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    if rendered.is_empty() {
        "the appeal review body".to_string()
    } else {
        rendered
    }
}

fn render_human_notice(
    template: Option<&serde_json::Value>,
    context: &BTreeMap<String, String>,
    transition: &Transition,
) -> String {
    let Some(template) = template else {
        return fallback_human_notice(context, transition);
    };

    let mut parts = Vec::new();
    if let Some(subject) = template.get("subject").and_then(serde_json::Value::as_str) {
        parts.push(format!(
            "Subject: {}",
            render_template_text(subject, context)
        ));
    }
    if let Some(sections) = template
        .get("sections")
        .and_then(serde_json::Value::as_array)
    {
        for section in sections {
            let title = section
                .get("title")
                .and_then(serde_json::Value::as_str)
                .or_else(|| section.get("id").and_then(serde_json::Value::as_str))
                .unwrap_or("Notice section");
            let content = section
                .get("content")
                .and_then(serde_json::Value::as_str)
                .map(|content| render_template_text(content, context))
                .unwrap_or_else(|| title.to_string());
            parts.push(format!("{title}\n{content}"));
        }
    }
    if parts.is_empty() {
        fallback_human_notice(context, transition)
    } else {
        parts.join("\n\n")
    }
}

fn fallback_human_notice(context: &BTreeMap<String, String>, transition: &Transition) -> String {
    let transition_event = transition
        .event
        .as_ref()
        .map(TransitionEvent::authoring_display_label)
        .unwrap_or_else(|| "(none)".to_string());
    format!(
        "Decision\nYour case moved to {determination} after event {event}.\n\nFactual basis\nThe decision used the case-file snapshot with SHA-256 digest {snapshot}.\n\nAppeal rights\nYou may appeal within {appeal_window}. Submit the appeal to {appeal_body}.",
        determination = context
            .get("determination")
            .map(String::as_str)
            .unwrap_or(transition.target.as_str()),
        event = context
            .get("decisionEvent")
            .map(String::as_str)
            .unwrap_or(transition_event.as_str()),
        snapshot = context
            .get("snapshotSha256")
            .map(String::as_str)
            .unwrap_or("unavailable"),
        appeal_window = context
            .get("appealWindow")
            .map(String::as_str)
            .unwrap_or("not specified"),
        appeal_body = context
            .get("appealBody")
            .map(String::as_str)
            .unwrap_or("the appeal review body"),
    )
}

fn render_template_text(template: &str, context: &BTreeMap<String, String>) -> String {
    let mut rendered = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        rendered.push_str(&rest[..start]);
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find("}}") else {
            rendered.push_str(&rest[start..]);
            return rendered;
        };
        let key = after_start[..end].trim();
        rendered.push_str(
            context
                .get(key)
                .map(String::as_str)
                .unwrap_or("unavailable"),
        );
        rest = &after_start[end + 2..];
    }
    rendered.push_str(rest);
    rendered
}

fn render_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Array(values) => values
            .iter()
            .map(render_value)
            .collect::<Vec<_>>()
            .join(", "),
        other => other.to_string(),
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

// Per ADR 0076 D-1, drift-monitor and agent-config companion values are the
// extracted `agents` JSON array from the $wosWorkflow `agents` embedded block.

fn drift_monitor_declares_policy_ref(
    actor_id: &str,
    policy_ref: &str,
    companion_docs: &HashMap<String, serde_json::Value>,
) -> bool {
    companion_docs.values().any(|doc| {
        doc.as_array().is_some_and(|agents| {
            agents.iter().any(|agent| {
                agent.get("id").and_then(serde_json::Value::as_str) == Some(actor_id)
                    && agent
                        .get("driftMonitoring")
                        .and_then(|dm| dm.get("alertThresholds"))
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
        doc.as_array().is_some_and(|agents| {
            agents.iter().any(|agent| {
                agent.get("id").and_then(serde_json::Value::as_str) == Some(actor_id)
                    && agent
                        .pointer("/autonomyPolicy/demotion")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|rules| {
                            rules.iter().any(|rule| {
                                rule.get("id").and_then(serde_json::Value::as_str)
                                    == Some(policy_ref)
                            })
                        })
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
                    id: ProvenanceRecord::mint_id(),
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
                    canonical_event_hash: None,
                    transition_tags: Vec::new(),
                    case_file_snapshot: None,
                    outcome: None,
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
            id: ProvenanceRecord::mint_id(),
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
            canonical_event_hash: None,
            transition_tags: Vec::new(),
            case_file_snapshot: None,
            outcome: None,
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

#[cfg(test)]
mod tests {
    //! Unit tests for the deterministic adverse-decision notice renderer.

    use super::*;
    use wos_core::model::kernel::{Transition, TransitionEvent};

    /// Build a fresh renderer context + transition for byte-identity tests.
    fn fixture() -> (BTreeMap<String, String>, Transition, serde_json::Value) {
        let case_state = serde_json::json!({
            "applicationId": "APP-2026-0042",
            "eligibilityEvidence": "below-threshold",
        });
        let snapshot = CaseFileSnapshot::from_case_state(&case_state);
        let transition = Transition {
            event: Some(TransitionEvent::from_authoring_trigger("denied")),
            target: "adverseNotice".to_string(),
            guard: None,
            actions: Vec::new(),
            actor: None,
            description: None,
            tags: vec!["adverse-decision".to_string()],
        };
        let appeal = serde_json::json!({
            "enabled": true,
            "appealWindow": "P30D",
            "reviewerRoles": ["appealReviewer"],
        });

        // Synthesize a minimal CaseInstance via deserialization so we exercise
        // notice_render_context exactly as production does.
        let instance: CaseInstance = serde_json::from_value(serde_json::json!({
            "instanceId": "case-42",
            "definitionUrl": "https://test.wos-spec.org/workflows/due-process",
            "definitionVersion": "1.0.0",
            "configuration": ["pendingDetermination"],
            "caseState": case_state,
            "provenancePosition": 0,
            "timers": [],
            "activeTasks": [],
            "status": "active",
            "createdAt": "2026-04-20T00:00:00Z",
            "updatedAt": "2026-04-20T00:00:00Z",
        }))
        .expect("test case instance deserialization must succeed");

        let now_ms: u64 = 1_700_000_000_000;
        let context = notice_render_context(
            &instance,
            &snapshot,
            &transition,
            Some("P30D"),
            &appeal,
            now_ms,
        );
        (context, transition, appeal)
    }

    /// Finding 2: identical snapshot/policy/template/transition/appeal/firing-timestamp
    /// inputs MUST render byte-identical `humanReadable` prose. The fallback
    /// branch (template=None) is the observable surface for spec prose determinism;
    /// we lock it down because the template branch is exercised by the G-002
    /// conformance fixture.
    #[test]
    fn fallback_human_readable_is_byte_identical_for_equal_inputs() {
        let (context_a, transition_a, _) = fixture();
        let (context_b, transition_b, _) = fixture();
        let rendered_a = render_human_notice(None, &context_a, &transition_a);
        let rendered_b = render_human_notice(None, &context_b, &transition_b);
        assert_eq!(
            rendered_a, rendered_b,
            "deterministic renderer must produce byte-identical prose"
        );
        // Also assert the rendered prose embeds the snapshot digest and appeal
        // window, guarding against regressions that drop a determining input.
        assert!(
            rendered_a.contains("7c6c9f0425dd8135fe6bbe81f876ddc1a6c5478bec3967dec1a40c93a6f8a749")
        );
        assert!(rendered_a.contains("P30D"));
    }

    /// Companion check: changing only the firing timestamp MUST change the
    /// rendered `appealDeadline`, confirming the spec's enumeration of
    /// transition-firing-timestamp as a determining input (Finding 3).
    #[test]
    fn firing_timestamp_participates_in_rendered_deadline() {
        let (context_early, transition, _) = fixture();
        // Rebuild a second context with a one-hour-later firing time.
        let case_state = serde_json::json!({
            "applicationId": "APP-2026-0042",
            "eligibilityEvidence": "below-threshold",
        });
        let snapshot = CaseFileSnapshot::from_case_state(&case_state);
        let appeal = serde_json::json!({
            "enabled": true,
            "appealWindow": "P30D",
            "reviewerRoles": ["appealReviewer"],
        });
        let instance: CaseInstance = serde_json::from_value(serde_json::json!({
            "instanceId": "case-42",
            "definitionUrl": "https://test.wos-spec.org/workflows/due-process",
            "definitionVersion": "1.0.0",
            "configuration": ["pendingDetermination"],
            "caseState": case_state,
            "provenancePosition": 0,
            "timers": [],
            "activeTasks": [],
            "status": "active",
            "createdAt": "2026-04-20T00:00:00Z",
            "updatedAt": "2026-04-20T00:00:00Z",
        }))
        .unwrap();
        let later_ms = 1_700_000_000_000u64 + 60 * 60 * 1000;
        let context_late = notice_render_context(
            &instance,
            &snapshot,
            &transition,
            Some("P30D"),
            &appeal,
            later_ms,
        );
        assert_ne!(
            context_early.get("appealDeadline"),
            context_late.get("appealDeadline"),
            "firing-timestamp must flow through to appealDeadline"
        );
    }
}
