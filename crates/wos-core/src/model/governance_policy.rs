// Rust guideline compliant 2026-02-21
//! Original governance Rules and pure selected-association checks.
use super::notice_requirements::NoticeRequirement;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use wos_events::governance::RuleReference;
/// Exact authored transition selected by a finite governance binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GovernanceTransition {
    /// Original global source state identifier.
    pub from: String,
    /// Original global target state identifier.
    pub to: String,
    /// Original authored or actual native event label.
    pub event: String,
}

/// Human decision evidence selected from an accepted native task artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HumanDecisionBinding {
    /// Actual transition, not a state-name approximation.
    pub transition: GovernanceTransition,
    /// Task declaration whose accepted artifact supplies the judgment.
    pub task_ref: String,
    /// JSON pointer to structured DecisionEvidence in the accepted response.
    pub evidence_pointer: String,
    /// Optional pinned owning Mapping applied to the unchanged accepted Response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_mapping: Option<GovernanceEvidenceMapping>,
    /// Exact permitted rule references from the selected source policy.
    pub rules: Vec<RuleReference>,
    /// Complete selected criterion identifiers.
    pub criteria: Vec<String>,
    /// Snapshot paths this finite policy permits as decision evidence.
    pub case_field_paths: Vec<String>,
    /// Controllable criterion/factor identifiers permitted for positive evidence.
    pub positive_factors: Vec<String>,
    /// Irrelevant factors the actual human review must explicitly exclude.
    pub negative_factors: Vec<String>,
}

/// Immutable owning Mapping selected by the workflow's decision policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GovernanceEvidenceMapping {
    /// Existing selected Formspec Mapping reference.
    #[serde(rename = "ref")]
    pub reference: String,
    /// Exact raw-file SHA-256 from the existing selected processor.
    pub sha256: String,
}

/// A business-field value that cannot become adverse without its original notice.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProtectedGovernanceEffect {
    /// Exact JSON pointer into the actual case file.
    pub path: String,
    /// Finite selected values representing effective adverse treatment.
    pub adverse_values: Vec<Value>,
}

/// Finite before-effective notice capability selected by the workflow owner.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DueProcessBinding {
    /// Owning delivery callback binding, independently qualified by the host.
    pub notice_binding_id: String,
    /// Actual transitions allowed to make the original proposal effective.
    pub effective_transitions: Vec<GovernanceTransition>,
    /// Complete selected adverse business writes, checked also on task Mapping.
    pub protected_effects: Vec<ProtectedGovernanceEffect>,
}

/// Finite no-response rule executed from actual time and complete retained artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimeoutDecisionBinding {
    /// Actual unconditional native timeout transition.
    pub transition: GovernanceTransition,
    /// Original retained native timer identifier.
    pub timer_id: String,
    /// Selected whole-calendar-day response period, such as `P90D`.
    pub response_period: String,
    /// Actual qualified original request-notice binding.
    pub request_notice_binding_id: String,
    /// Complete correction-task declarations relevant to this no-response rule.
    pub correction_task_refs: Vec<String>,
    /// Explicit selected demo rule; this does not assert a legal source.
    pub rule: RuleReference,
    /// Actual maintained late-evidence/reconsideration route described by the rule.
    pub reconsideration_route: String,
    /// Factors excluded by this finite rule's restricted input dependencies.
    pub excluded_factors: Vec<String>,
}

/// Actual filing and independent human review for the selected demo appeal rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppealBinding {
    /// Explicit selected policy label; this does not assert a legal source.
    pub policy_label: String,
    /// Authored name of the independent reconsideration body.
    pub review_body: String,
    /// Authored filing instructions included in the original notice.
    pub filing_instructions: String,
    /// Actual transition consuming a real applicant filing task completion.
    pub filed_transition: GovernanceTransition,
    /// Authored applicant filing task.
    pub filed_task_ref: String,
    /// Pointer to the original proposal Facts reference in its accepted artifact.
    pub proposal_reference_pointer: String,
    /// Authored independently assigned human review task.
    pub review_task_ref: String,
    /// Workflow-local declared human selected separately from the original reviewer.
    pub reviewer_actor: String,
    /// Exact actual review transitions, also covered by human decision bindings.
    pub review_transitions: Vec<GovernanceTransition>,
}
/// Original closed governance policy; validation never rewrites its declarations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GovernancePolicyRules {
    /// Canonical Rules version, exactly two.
    pub version: u8,
    /// Complete human determination coverage.
    pub human_decisions: Vec<HumanDecisionBinding>,
    /// Exact requirement for each selected notice binding.
    pub notice_requirements: BTreeMap<String, NoticeRequirement>,
    /// Selected adverse-effect protection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_process: Option<DueProcessBinding>,
    /// Actual no-response rules.
    #[serde(default)]
    pub timeout_decisions: Vec<TimeoutDecisionBinding>,
    /// Independently assigned appeal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appeal: Option<AppealBinding>,
}

use super::control_flow::{self, ActionLocation, FlowAnalysis, FlowIssue, NoticePath};
use super::integration::{
    CallbackPhase, IntegrationBindingKind, IntegrationDispatchMode, IntegrationProfileDocument,
    validate_callback_actions,
};
use super::kernel::{self, Action, ActionKind, KernelDocument, TransitionEvent};
use super::notice_requirements::{NoticeConsequence, check_notice_declaration};

/// One selected binding and its actual derived business consequences.
#[derive(Debug)]
pub struct ResolvedNotice<'a> {
    /// Original binding key, never a synthetic association.
    pub binding_id: &'a str,
    /// Original selected requirement.
    pub requirement: &'a NoticeRequirement,
    /// Consequences derived from original Rules rather than caller assertions.
    pub consequences: Vec<NoticeConsequence>,
}
/// Shared selected analysis, without provider or runtime authority.
#[derive(Debug)]
pub struct GovernancePolicyAnalysis<'a> {
    /// One configuration-graph analysis plus selected association findings.
    pub flow: FlowAnalysis,
    /// Located original requirements for installed-capability checks.
    pub notices: Vec<ResolvedNotice<'a>>,
}
fn state_id(path: &str) -> String {
    path.rsplit('/')
        .next()
        .unwrap_or("")
        .replace("~1", "/")
        .replace("~0", "~")
}
fn pointer(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}
fn issue(issues: &mut Vec<FlowIssue>, path: &str, message: impl Into<String>) {
    issues.push(FlowIssue::error("NOTICE-GRAPH-001", path, message));
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum TimerBindingReference {
    Unrelated,
    Unsupported,
    ExactAnchor,
}

// Detection is broader than qualification: every mapped descendant remains
// notice-derived, but only the original scalar or governed callback clock qualifies.
fn timer_binding_reference(
    action: &Action,
    binding: &super::integration::IntegrationBinding,
) -> TimerBindingReference {
    if action.action != ActionKind::StartTimer {
        return TimerBindingReference::Unrelated;
    }
    let Some(expression) = action
        .start_at_expression
        .as_ref()
        .and_then(|s| fel_core::parser::parse(s).ok())
    else {
        return TimerBindingReference::Unrelated;
    };
    let dependencies = fel_core::dependencies::extract_dependencies(&expression);
    let mut reference = TimerBindingReference::Unrelated;
    for (key, source) in &binding.output_binding {
        let target = format!("caseFile.{}", key.strip_prefix("caseFile.").unwrap_or(key));
        if !dependencies.fields.iter().any(|field| {
            field == &target
                || field
                    .strip_prefix(&target)
                    .is_some_and(|tail| tail.starts_with('.') || tail.starts_with('['))
        }) {
            continue;
        }
        reference = TimerBindingReference::Unsupported;
        let exact_scalar = fel_core::parser::parse(&target).is_ok_and(|e| e == expression);
        // The maintained host maps the complete callback with `$`; its clock
        // owner authenticates this governed value separately from physical sentAt.
        let exact_callback_clock = source == "$"
            && fel_core::parser::parse(&format!("{target}.clock.governedSentAt"))
                .is_ok_and(|e| e == expression);
        if exact_scalar || exact_callback_clock {
            return TimerBindingReference::ExactAnchor;
        }
    }
    reference
}
fn notice_uses<'a>(
    kernel: &KernelDocument,
    rules: &'a GovernancePolicyRules,
    integration: Option<&IntegrationProfileDocument>,
) -> BTreeMap<&'a str, Vec<NoticeConsequence>> {
    let mut uses: BTreeMap<&str, Vec<NoticeConsequence>> = BTreeMap::new();
    if let Some(due) = &rules.due_process {
        uses.entry(&due.notice_binding_id)
            .or_default()
            .push(NoticeConsequence::AdverseEffect);
        if rules.appeal.is_some() {
            uses.entry(&due.notice_binding_id)
                .or_default()
                .push(NoticeConsequence::AppealWindow);
        }
    }
    for timeout in &rules.timeout_decisions {
        uses.entry(&timeout.request_notice_binding_id)
            .or_default()
            .push(NoticeConsequence::ResponseDeadline);
    }
    // Only original selected notice bindings are classified as notice. Other
    // integration callbacks and their unrelated timer origins remain untouched.
    for key in rules.notice_requirements.keys() {
        if let Some(binding) = integration.and_then(|p| p.bindings.get(key)) {
            let mut clocked = false;
            kernel::visit_actions(kernel, |_, action| {
                clocked |=
                    timer_binding_reference(action, binding) != TimerBindingReference::Unrelated;
            });
            if clocked
                && !uses
                    .get(key.as_str())
                    .is_some_and(|u| u.contains(&NoticeConsequence::ResponseDeadline))
            {
                uses.entry(key)
                    .or_default()
                    .push(NoticeConsequence::ResponseDeadline);
            }
        }
    }
    uses
}
/// Resolve declared requirements and their actual original Rules uses.
#[must_use]
fn resolve_notice_requirements<'a>(
    kernel: &KernelDocument,
    rules: &'a GovernancePolicyRules,
    integration: Option<&IntegrationProfileDocument>,
) -> Vec<ResolvedNotice<'a>> {
    let mut uses = notice_uses(kernel, rules, integration);
    rules
        .notice_requirements
        .iter()
        .map(|(binding_id, requirement)| ResolvedNotice {
            binding_id,
            requirement,
            consequences: uses
                .remove(binding_id.as_str())
                .unwrap_or_else(|| vec![NoticeConsequence::OrdinaryStatus]),
        })
        .collect()
}
// Retain actual action locations through the explicit callback continuation.
fn callback_actions<'a>(
    kernel: &'a KernelDocument,
    consumers: &[(String, usize, &'a kernel::Transition)],
) -> Vec<(String, Option<usize>, &'a Action)> {
    let mut states = BTreeMap::new();
    kernel::visit_states(kernel, |path, state| {
        states.insert(state_id(path), (path.to_owned(), state));
    });
    let mut actions = Vec::new();
    for (_, _, consumer) in consumers {
        if let Some((target_path, target)) = states.get(consumer.target.as_str()) {
            // Mapping output is retained before the emitted event's next drain.
            for emitted in consumer
                .actions
                .iter()
                .filter(|a| a.action == ActionKind::EmitEvent)
                .filter_map(|a| a.event_type.as_deref())
            {
                for (index, continuation) in target.transitions.iter().enumerate() {
                    if continuation
                        .event
                        .as_ref()
                        .is_some_and(|e| e.matches_runtime_dispatch(emitted))
                    {
                        actions.extend(
                            continuation
                                .actions
                                .iter()
                                .map(|a| (target_path.clone(), Some(index), a)),
                        );
                        if let Some((next_path, next)) = states.get(continuation.target.as_str()) {
                            actions
                                .extend(next.on_entry.iter().map(|a| (next_path.clone(), None, a)));
                        }
                    }
                }
            }
        }
    }
    actions
}
/// Resolve original Rules and analyze the actual selected configuration once.
///
/// This checks possible structure only. It authenticates no callback or provider
/// result and makes no claim about guards, eventual completion or delivery.
#[must_use]
pub fn analyze_governance_policy<'a>(
    kernel: &KernelDocument,
    rules: &'a GovernancePolicyRules,
    integration: Option<&IntegrationProfileDocument>,
) -> GovernancePolicyAnalysis<'a> {
    if let Err(gap) = control_flow::check_selected_resources(kernel) {
        return GovernancePolicyAnalysis {
            flow: control_flow::coverage_refusal(gap),
            notices: Vec::new(),
        };
    }
    if rules.notice_requirements.len() > 4096 || rules.timeout_decisions.len() > 4096 {
        return GovernancePolicyAnalysis {
            flow: control_flow::coverage_refusal(control_flow::CoverageGap {
                path: "/rules".into(),
                reason: "selected notice declaration bound exceeded".into(),
            }),
            notices: Vec::new(),
        };
    }
    let mut issues = Vec::new();
    if rules.version != 2 {
        issue(
            &mut issues,
            "/rules/version",
            "GovernancePolicyRules version must be 2",
        );
    }
    if kernel
        .governance
        .as_ref()
        .and_then(|g| g.pointer("/dueProcess/adverseDecisionPolicy/noticeRequired"))
        == Some(&Value::Bool(true))
        && rules.due_process.is_none()
    {
        issue(
            &mut issues,
            "/rules/dueProcess",
            "actual Workflow adverse-notice obligation has no selected Rules association",
        );
    }
    let uses = notice_uses(kernel, rules, integration);
    for key in uses.keys() {
        if !rules.notice_requirements.contains_key(*key) {
            issue(
                &mut issues,
                &format!("/rules/noticeRequirements/{}", pointer(key)),
                "actual consequence has no original notice requirement",
            );
        }
    }
    let timing = kernel
        .governance
        .as_ref()
        .and_then(|g| g.pointer("/dueProcess/adverseDecisionPolicy/noticeTiming"))
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok());
    let notices = resolve_notice_requirements(kernel, rules, integration);
    let mut paths = Vec::new();
    if !rules.notice_requirements.is_empty() {
        if let Some(profile) = integration {
            if let Err(error) = profile.validate_target(
                kernel.url.as_deref().unwrap_or(""),
                kernel.version.as_deref().unwrap_or(""),
            ) {
                issue(&mut issues, "/integration/targetWorkflow", error);
            }
            if let Err(error) = validate_callback_actions(kernel, Some(profile)) {
                issue(&mut issues, "/integration/bindings", error);
            }
        } else {
            issue(
                &mut issues,
                "/integration",
                "selected notice requires its actual IntegrationProfile",
            );
        }
    }
    for notice in &notices {
        let binding_id = notice.binding_id;
        let requirement = notice.requirement;
        let path = format!("/rules/noticeRequirements/{}", pointer(binding_id));
        for &consequence in &notice.consequences {
            issues.extend(
                check_notice_declaration(requirement, consequence, timing, &path)
                    .into_iter()
                    .map(|i| {
                        FlowIssue::error(
                            i.rule_id,
                            if i.rule_id == "G-001" {
                                "/governance/dueProcess/adverseDecisionPolicy/noticeTiming"
                            } else {
                                &i.path
                            },
                            i.message,
                        )
                    }),
            );
        }
        let Some(binding) = integration.and_then(|p| p.bindings.get(binding_id)) else {
            issue(
                &mut issues,
                &path,
                "notice binding is absent from selected integration",
            );
            continue;
        };
        if binding.kind != IntegrationBindingKind::Callback
            || binding.dispatch_mode != Some(IntegrationDispatchMode::Deferred)
        {
            issue(
                &mut issues,
                &format!("/integration/bindings/{}", pointer(binding_id)),
                "selected notice must use an explicit deferred callback",
            );
            continue;
        }
        let Some(_event) = binding
            .extensions
            .get("callbackEventType")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
        else {
            issue(&mut issues, &path, "required callback event is absent");
            continue;
        };
        kernel::visit_actions(kernel, |action_path, action| {
            if timer_binding_reference(action, binding) != TimerBindingReference::Unrelated {
                let timeout = rules.timeout_decisions.iter().any(|t| {
                    t.request_notice_binding_id == binding_id
                        && action.timer_id.as_deref() == Some(t.timer_id.as_str())
                });
                let grace = rules
                    .due_process
                    .as_ref()
                    .is_some_and(|due| due.notice_binding_id == binding_id)
                    && kernel
                        .governance
                        .as_ref()
                        .and_then(|g| {
                            g.pointer("/dueProcess/adverseDecisionPolicy/noticeGracePeriod")
                        })
                        .and_then(Value::as_str)
                        .is_some_and(|period| action.duration.as_deref() == Some(period));
                if !timeout && !grace {
                    issue(
                        &mut issues,
                        action_path,
                        "actual notice-derived timer has no original Rules timeout or adverse grace association",
                    );
                }
                if timer_binding_reference(action, binding) != TimerBindingReference::ExactAnchor {
                    issue(
                        &mut issues,
                        action_path,
                        "dynamic notice-derived timer origin cannot establish an exact original observation association",
                    );
                }
            }
        });
        let mut producers = Vec::new();
        let mut consumers = Vec::new();
        kernel::visit_states(kernel, |state_path, state| {
            // Count actual actions, not merely states containing at least one.
            // Compensation and on-exit associations are not guessed.
            for action in &state.on_entry {
                if action.action == ActionKind::InvokeService
                    && action.service_ref.as_deref() == Some(binding_id)
                    && action.callback_phase == Some(CallbackPhase::Outbound)
                {
                    producers.push((state_path.to_owned(), None));
                }
            }
            for (index, transition) in state.transitions.iter().enumerate() {
                for action in &transition.actions {
                    if action.action == ActionKind::InvokeService
                        && action.service_ref.as_deref() == Some(binding_id)
                    {
                        if action.callback_phase == Some(CallbackPhase::Outbound) {
                            producers.push((state_path.to_owned(), Some(index)));
                        }
                        if action.callback_phase==Some(CallbackPhase::Inbound) && transition.event.as_ref().is_some_and(|e|matches!(e,TransitionEvent::Message{name,..} if !name.trim().is_empty())) {consumers.push((state_path.to_owned(),index,transition));}
                    }
                }
            }
        });
        if producers.len() != 1 {
            issue(
                &mut issues,
                &path,
                "mandatory notice requires one unambiguous outbound action",
            );
        }
        if consumers.len() != 1 {
            issue(
                &mut issues,
                &path,
                "mandatory notice requires one exact inbound callback action and message consumer",
            );
        }
        let base = if let (
            [(producer, producer_transition)],
            [(consumer, consumer_transition, _)],
        ) = (producers.as_slice(), consumers.as_slice())
        {
            if producer == consumer && *producer_transition == Some(*consumer_transition) {
                issue(
                    &mut issues,
                    &path,
                    "one transition cannot establish both its fresh outbound request and its inbound receipt",
                );
                None
            } else {
                let actions = vec![
                    ActionLocation {
                        state: producer.clone(),
                        transition: *producer_transition,
                    },
                    ActionLocation {
                        state: consumer.clone(),
                        transition: Some(*consumer_transition),
                    },
                ];
                paths.push(NoticePath {
                    actions: actions.clone(),
                    path: path.clone(),
                });
                Some(actions)
            }
        } else {
            None
        };
        let associated = callback_actions(kernel, &consumers);
        kernel::visit_actions(kernel, |action_path, action| {
            // Pointer comparison identifies this actual borrowed authored node;
            // equal values in a different action list are not the same action.
            if timer_binding_reference(action, binding) != TimerBindingReference::Unrelated
                && !associated
                    .iter()
                    .any(|(_, _, selected)| std::ptr::eq(*selected, action))
            {
                issue(
                    &mut issues,
                    action_path,
                    "notice callback-output clock must start in an explicit post-mapping emitted-event continuation",
                );
            }
        });
        if let Some(due) = rules
            .due_process
            .as_ref()
            .filter(|d| d.notice_binding_id == binding_id)
        {
            for selected in &due.effective_transitions {
                let direct = consumers.iter().any(|(path, _, t)| {
                    state_id(path) == selected.from
                        && t.target == selected.to
                        && t.event
                            .as_ref()
                            .is_some_and(|e| e.matches_runtime_dispatch(&selected.event))
                });
                let emitted = consumers.iter().any(|(_, _, t)| {
                    t.actions.iter().any(|a| {
                        a.action == ActionKind::EmitEvent
                            && a.event_type.as_deref() == Some(selected.event.as_str())
                    })
                });
                let mut actual = Vec::new();
                kernel::visit_states(kernel, |path, state| {
                    if state_id(path) == selected.from {
                        for (index, t) in state.transitions.iter().enumerate() {
                            if t.target == selected.to
                                && t.event
                                    .as_ref()
                                    .is_some_and(|e| e.matches_runtime_dispatch(&selected.event))
                            {
                                actual.push(ActionLocation {
                                    state: path.to_owned(),
                                    transition: Some(index),
                                });
                            }
                        }
                    }
                });
                if emitted {
                    if let (Some(base), [consumer]) = (&base, actual.as_slice()) {
                        let mut actions = base.clone();
                        actions.push(consumer.clone());
                        paths.push(NoticePath {
                            actions,
                            path: "/rules/dueProcess/effectiveTransitions".into(),
                        });
                    }
                }
                let period = kernel
                    .governance
                    .as_ref()
                    .and_then(|g| g.pointer("/dueProcess/adverseDecisionPolicy/noticeGracePeriod"))
                    .and_then(Value::as_str);
                let mut timed = false;
                if let Some(period) = period {
                    kernel::visit_states(kernel, |path, state| {
                        if state_id(path) != selected.from {
                            return;
                        }
                        for (index, transition) in state.transitions.iter().enumerate() {
                            if transition.target != selected.to
                                || !transition
                                    .event
                                    .as_ref()
                                    .is_some_and(|e| e.matches_runtime_dispatch(&selected.event))
                            {
                                continue;
                            }
                            if let Some(TransitionEvent::Timer { timer_id, .. }) = &transition.event
                            {
                                let starts = associated
                                    .iter()
                                    .filter(|(_, _, a)| {
                                        a.action == ActionKind::StartTimer
                                            && a.timer_id.as_deref() == Some(timer_id.as_str())
                                            && a.duration.as_deref() == Some(period)
                                            && timer_binding_reference(a, binding)
                                                == TimerBindingReference::ExactAnchor
                                    })
                                    .collect::<Vec<_>>();
                                if let [(producer, producer_transition, _)] = starts.as_slice() {
                                    if let Some(base) = &base {
                                        let mut actions = base.clone();
                                        actions.push(ActionLocation {
                                            state: producer.clone(),
                                            transition: *producer_transition,
                                        });
                                        actions.push(ActionLocation {
                                            state: path.to_owned(),
                                            transition: Some(index),
                                        });
                                        paths.push(NoticePath {
                                            actions,
                                            path: "/rules/dueProcess/effectiveTransitions".into(),
                                        });
                                        timed = true;
                                    }
                                }
                            }
                        }
                    });
                }
                if !direct && !(emitted && actual.len() == 1) && !timed {
                    issue(
                        &mut issues,
                        "/rules/dueProcess/effectiveTransitions",
                        "protected continuation has no explicit callback, emitted event or original grace-timer association",
                    );
                }
            }
        }
        for (index, timeout) in rules
            .timeout_decisions
            .iter()
            .enumerate()
            .filter(|(_, t)| t.request_notice_binding_id == binding_id)
        {
            let timer_path = format!("/rules/timeoutDecisions/{index}/timerId");
            let mut timer_consumers = Vec::new();
            kernel::visit_states(kernel, |state_path, state| {
                for (transition_index, t) in state.transitions.iter().enumerate() {
                    if state_id(state_path)==timeout.transition.from && t.target==timeout.transition.to && t.event.as_ref().is_some_and(|e|matches!(e,TransitionEvent::Timer{timer_id,..} if timer_id==&timeout.timer_id) && e.matches_runtime_dispatch(&timeout.transition.event)) {
                        timer_consumers.push(ActionLocation{state:state_path.to_owned(),transition:Some(transition_index)});
                    }
                }
            });
            let starts = associated
                .iter()
                .filter(|(_, _, a)| {
                    a.action == ActionKind::StartTimer
                        && a.timer_id.as_deref() == Some(timeout.timer_id.as_str())
                })
                .collect::<Vec<_>>();
            let anchored = starts.len() == 1
                && starts[0].2.duration.as_deref() == Some(timeout.response_period.as_str())
                && timer_binding_reference(starts[0].2, binding)
                    == TimerBindingReference::ExactAnchor;
            if timer_consumers.len() != 1 || !anchored {
                issue(
                    &mut issues,
                    &timer_path,
                    "required timer or original selected callback-output anchor cannot be resolved",
                );
            } else if let (Some(base), [consumer]) = (&base, timer_consumers.as_slice()) {
                let mut actions = base.clone();
                actions.push(ActionLocation {
                    state: starts[0].0.clone(),
                    transition: starts[0].1,
                });
                actions.push(consumer.clone());
                paths.push(NoticePath {
                    actions,
                    path: timer_path,
                });
            }
        }
    }
    let mut flow = control_flow::analyze_with_notice_paths(kernel, &paths);
    flow.issues.extend(issues);
    GovernancePolicyAnalysis { flow, notices }
}

struct RulesResources;
impl jsonschema::Retrieve for RulesResources {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let source = match uri.as_str() {
            "https://wos-spec.org/schemas/wos-notice-requirements.schema.json" => {
                include_str!("../../../../schemas/wos-notice-requirements.schema.json")
            }
            "https://wos-spec.org/schemas/wos-workflow.schema.json" => {
                include_str!("../../../../schemas/wos-workflow.schema.json")
            }
            "https://wos-spec.org/schemas/wos-reference-due-process.schema.json" => {
                include_str!("../../../../schemas/wos-reference-due-process.schema.json")
            }
            "https://wos-spec.org/schemas/wos-reference-replay.schema.json" => {
                include_str!("../../../../schemas/wos-reference-replay.schema.json")
            }
            _ => return Err(format!("unselected Rules schema resource: {uri}").into()),
        };
        Ok(serde_json::from_str(source)?)
    }
}
/// Return the canonical offline validator for the original closed Rules.
#[must_use]
pub fn rules_validator() -> &'static jsonschema::Validator {
    static VALIDATOR: std::sync::LazyLock<jsonschema::Validator> = std::sync::LazyLock::new(|| {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../../schemas/wos-governance-policy-rules.schema.json"
        ))
        .expect("canonical Rules schema is JSON");
        jsonschema::options()
            .with_retriever(RulesResources)
            .build(&schema)
            .expect("canonical Rules schema compiles offline")
    });
    &VALIDATOR
}
/// Check installed capabilities without rebuilding the structural graph.
#[must_use]
pub fn check_policy_notice_support(
    kernel: &KernelDocument,
    rules: &GovernancePolicyRules,
    integration: Option<&IntegrationProfileDocument>,
    installed: &[super::notice_requirements::NoticeCapability],
) -> Vec<super::notice_requirements::NoticeRequirementIssue> {
    if let Err(gap) = control_flow::check_selected_resources(kernel) {
        return vec![super::notice_requirements::NoticeRequirementIssue {
            rule_id: "WOS-FLOW-005",
            path: gap.path,
            message: "selected policy exceeds structural resource coverage",
        }];
    }
    let timing = kernel
        .governance
        .as_ref()
        .and_then(|g| g.pointer("/dueProcess/adverseDecisionPolicy/noticeTiming"))
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok());
    let mut issues = Vec::new();
    for notice in resolve_notice_requirements(kernel, rules, integration) {
        for consequence in notice.consequences {
            issues.extend(super::notice_requirements::check_notice_support(
                notice.requirement,
                consequence,
                timing,
                installed,
                &format!("/rules/noticeRequirements/{}", pointer(notice.binding_id)),
            ));
        }
    }
    issues
}

#[cfg(test)]
mod tests;
