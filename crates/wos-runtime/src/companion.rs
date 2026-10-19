// Rust guideline compliant 2026-02-21

//! Reference admission and the isolated supplied-output AI policy exercise.

use crate::runtime::{CompanionPolicy, RuntimeError, RuntimeEventContext, RuntimeEventDecision};
use std::collections::HashMap;
use wos_core::ai_policy::{NativeAiPolicy, PolicyEvaluation, PolicyExerciseContext, PolicyPhase};
use wos_core::deontic;
use wos_core::event_handler;
use wos_core::model::ai::{ConfidenceMethod, ViolationAction};
use wos_core::model::kernel::KernelDocument;
use wos_core::{ConfigurationWarningInput, ProvenanceKind, ProvenanceRecord};

mod admission;

/// Reference policy with admitted native AI behavior.
#[derive(Debug, Clone, Default)]
pub struct ReferenceCompanionPolicy {
    ai_doc: Option<NativeAiPolicy>,
}

impl ReferenceCompanionPolicy {
    /// Admit the selected workflow and construct its reference policy.
    ///
    /// # Errors
    /// Refuses unsupported mandatory governance operations or conflicting selected governance.
    pub fn new(
        kernel: &KernelDocument,
        ai_doc: Option<NativeAiPolicy>,
        governance: Option<serde_json::Value>,
        companions: HashMap<String, serde_json::Value>,
    ) -> Result<Self, RuntimeError> {
        Self::validate_configuration(kernel, governance.as_ref(), &companions)?;
        Ok(Self { ai_doc })
    }

    /// Validate declared operation requirements before reference process creation.
    ///
    /// This also admits selected migration targets through the same requirement checks.
    ///
    /// # Errors
    /// Refuses unsupported mandatory operations or a companion replacing embedded governance.
    pub fn validate_configuration(
        kernel: &KernelDocument,
        governance: Option<&serde_json::Value>,
        companions: &HashMap<String, serde_json::Value>,
    ) -> Result<(), RuntimeError> {
        admission::validate(kernel, governance, companions)
    }
}

impl CompanionPolicy for ReferenceCompanionPolicy {
    fn evaluate_event(
        &mut self,
        context: RuntimeEventContext,
    ) -> Result<RuntimeEventDecision, RuntimeError> {
        let mut provenance = Vec::new();
        let event = context.event;
        let event_data = event.data.clone();
        let actor_id = event.actor_id.as_deref().unwrap_or("");

        let mut policy_evaluation = None;
        if self.ai_doc.is_none()
            && context
                .kernel
                .actors
                .iter()
                .any(|actor| actor.id == actor_id && actor.kind == wos_core::ActorKind::Agent)
        {
            let mut observed = PolicyEvaluation::default();
            observed
                .refusals
                .push("native agent event requires its admitted native policy".into());
            provenance.push(policy_record(ProvenanceKind::DeonticEvaluation, serde_json::json!({"exercise":"nativeAgentEventRefusal","actorId":actor_id,"disposition":"unsupported","refusals":observed.refusals,"executionReadinessAssessed":false,"unexecutedStages":["invocation","inputDefinitionValidation","outputDefinitionValidation","humanReview","governedCommit"]})));
            return Ok(RuntimeEventDecision {
                event: None,
                provenance,
                policy_evaluation: Some(observed),
            });
        }
        if let Some(policy) = &self.ai_doc {
            if (policy.agent(actor_id).is_some()
                || context
                    .kernel
                    .actors
                    .iter()
                    .any(|actor| actor.id == actor_id && actor.kind == wos_core::ActorKind::Agent))
                && event_data
                    .as_ref()
                    .is_none_or(|data| data.get("output").is_none())
            {
                let mut observed = PolicyEvaluation::default();
                observed.refusals.push("native agent event without supplied output requires an executed owning operation".into());
                provenance.extend(native_policy_provenance(
                    &observed,
                    actor_id,
                    policy,
                    "nativeAgentEventRefusal",
                ));
                return Ok(RuntimeEventDecision {
                    event: None,
                    provenance,
                    policy_evaluation: Some(observed),
                });
            }
        }
        if let (Some(policy), Some(data)) = (&self.ai_doc, &event_data) {
            if policy.agent(actor_id).is_some() {
                if let Some(feature) = event_handler::unsupported_agent_hint(data) {
                    let mut observed = PolicyEvaluation::default();
                    observed.refusals.push(format!(
                        "unsupported agent event hint {feature}: owning operation was not executed"
                    ));
                    provenance.extend(native_policy_provenance(
                        &observed,
                        actor_id,
                        policy,
                        "suppliedOutput",
                    ));
                    return Ok(RuntimeEventDecision {
                        event: None,
                        provenance,
                        policy_evaluation: Some(observed),
                    });
                }
            }
            if let Some(output) = data.get("output").filter(|_| {
                policy.agent(actor_id).is_some()
                    || context.kernel.actors.iter().any(|actor| {
                        actor.id == actor_id && actor.kind == wos_core::ActorKind::Agent
                    })
            }) {
                let mut observed = PolicyEvaluation::default();
                if let Some(agent) = policy.agent(actor_id) {
                    if let Err(error) = policy.check_supplied_output_exercise(&agent.id) {
                        observed.refusals.push(error.to_string());
                    }
                    if !policy
                        .matches_kernel(&context.kernel)
                        .map_err(|error| RuntimeError::Evaluator(error.to_string()))?
                    {
                        observed.refusals.push(
                            "native policy differs from the full admitted kernel snapshot".into(),
                        );
                    }
                    if let Some(object) = data.as_object() {
                        for field in object.keys().filter(|field| {
                            !matches!(
                                field.as_str(),
                                "output"
                                    | "confidenceReport"
                                    | "bypass"
                                    | "deonticBypass"
                                    | "rationale"
                            )
                        }) {
                            observed.refusals.push(format!("unsupported supplied-output payload field {field}; it cannot establish a processor context or executed operation"));
                        }
                    }
                    let case_file = context.instance.case_state.clone();
                    // The native event interface provides no admitted invocation input
                    // or lifecycle action locator. Output fields cannot manufacture them.
                    let mut exercise = PolicyExerciseContext {
                        agent_id: actor_id,
                        action_ref: None,
                        input: None,
                        case_file: &case_file,
                        output,
                        confidence_report: data.get("confidenceReport"),
                        waiver: data
                            .get("deonticBypass")
                            .or_else(|| data.get("bypass"))
                            .or_else(|| data.get("rationale")),
                        phase: PolicyPhase::InputRights,
                    };
                    let rights = deontic::evaluate_native_policy(policy, &exercise);
                    observed.observations.extend(rights.observations);
                    observed.errors.extend(rights.errors);
                    observed.refusals.extend(rights.refusals);
                    observed.processor_error |= rights.processor_error;
                    observed.waiver_refused |= rights.waiver_refused;
                    exercise.phase = PolicyPhase::Output;
                    let output_policy = deontic::evaluate_native_policy(policy, &exercise);
                    observed.observations.extend(output_policy.observations);
                    observed.errors.extend(output_policy.errors);
                    observed.refusals.extend(output_policy.refusals);
                    observed.waiver_refused |= output_policy.waiver_refused;
                    observed.effective_action = output_policy.effective_action;
                    observed.confidence_report = output_policy.confidence_report;
                    // AI §7.2 requires calibration for these methods; the label itself
                    // is not trusted calibration evidence (AI §7.1).
                    if observed.confidence_report.as_ref().is_some_and(|report| {
                        matches!(
                            report.method,
                            ConfidenceMethod::ModelNative
                                | ConfidenceMethod::Calibrated
                                | ConfidenceMethod::Conformal
                        )
                    }) {
                        observed.refusals.push("trusted calibration operation unavailable for reported confidence method".into());
                    }
                    if matches!(
                        observed.effective_action,
                        Some(ViolationAction::SwitchToAssistive | ViolationAction::EscalateToHuman)
                    ) {
                        observed.refusals.push("required human confirmation/review operation not executed by supplied-output policy exercise".into());
                    }
                } else {
                    observed.errors.push(
                        "supplied agent output requires an admitted native agent actor".into(),
                    );
                }
                provenance.extend(native_policy_provenance(
                    &observed,
                    actor_id,
                    policy,
                    "suppliedOutput",
                ));
                let blocked = !observed.errors.is_empty()
                    || !observed.refusals.is_empty()
                    || matches!(observed.effective_action, Some(ViolationAction::Reject));
                policy_evaluation = Some(observed);
                // The scoped exercise returns before generic event hints can
                // manufacture validation, fallback, recovery or other proof.
                let event = if blocked { None } else { Some(event) };
                return Ok(RuntimeEventDecision {
                    event,
                    provenance,
                    policy_evaluation,
                });
            }
        }

        // Ordinary business payloads remain inputs. Only an executed native
        // operation can establish task, persistence, validation or governance evidence.
        if matches!(event.event.as_str(), "$restart" | "$migrate") {
            return Err(RuntimeError::UnsupportedAction(format!(
                "{} requires an owning operation; no recovery or migration was executed",
                event.event
            )));
        }
        Ok(RuntimeEventDecision {
            event: Some(event),
            provenance,
            policy_evaluation,
        })
    }
}

fn native_policy_provenance(
    observed: &PolicyEvaluation,
    actor_id: &str,
    policy: &NativeAiPolicy,
    exercise: &str,
) -> Vec<ProvenanceRecord> {
    let mut records = Vec::new();
    for observation in &observed.observations {
        let kind = if observation.kind == "right" && observation.outcome == "missing-entitlement" {
            ProvenanceKind::RightsViolation
        } else if observation.action.is_some() {
            match observation.kind.as_str() {
                "permission" => ProvenanceKind::DeonticViolation,
                "prohibition" => ProvenanceKind::DeonticViolation,
                "obligation" => ProvenanceKind::DeonticViolation,
                "confidenceFloor" => ProvenanceKind::ConfidenceViolation,
                _ => ProvenanceKind::DeonticEvaluation,
            }
        } else {
            ProvenanceKind::DeonticEvaluation
        };
        let mut record = policy_record(
            kind,
            serde_json::json!({"ruleId":observation.rule_id,"constraintId":observation.rule_id,"pointer":observation.pointer,"kind":observation.kind,"outcome":observation.outcome,"action":observation.action,"attribution":observation.attribution}),
        );
        if observation.attribution != "processor" {
            record.actor_id = Some(actor_id.into());
        }
        records.push(record);
    }
    records.push(policy_record(ProvenanceKind::DeonticEvaluation,serde_json::json!({"exercise":exercise,"workflowUrl":policy.workflow_url(),"workflowVersion":policy.workflow_version(),"actorId":actor_id,"modelIdentifier":policy.agent(actor_id).map(|agent|agent.model_identifier.as_str()),"modelVersion":policy.agent(actor_id).map(|agent|agent.model_version.as_str()),"disposition":observed.disposition(),"errors":observed.errors,"refusals":observed.refusals,"confidenceReport":observed.confidence_report,"unexecutedStages":["invocation","inputDefinitionValidation","outputDefinitionValidation","humanReview","governedCommit"],"executionReadinessAssessed":false})));
    records
}
fn policy_record(kind: ProvenanceKind, data: serde_json::Value) -> ProvenanceRecord {
    let mut record = ProvenanceRecord::configuration_warning(ConfigurationWarningInput {
        subject: "native-ai-policy",
        unresolved_ref: None,
        context: None,
    });
    record.record_kind = kind;
    record.data = Some(data);
    record
}
