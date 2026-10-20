// Rust guideline compliant 2026-02-21
//! Rejects declared required operations absent from the reference adapter.

use crate::runtime::RuntimeError;
use serde_json::Value;
use std::collections::HashMap;
use wos_core::model::kernel::{KernelDocument, State};

fn refuse(pointer: &str, obligation: &str) -> RuntimeError {
    RuntimeError::UnsupportedAction(format!(
        "{pointer}: unsupported reference operation: {obligation}; no operation executed"
    ))
}

fn nonempty_array(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
}

// This checks for an execution requirement; it never resolves a parameter.
fn temporal_table_pointer(document: &Value) -> Option<String> {
    for prefix in ["", "/policyParameters"] {
        for family in ["parameters", "bindings"] {
            let pointer = format!("{prefix}/{family}");
            if document
                .pointer(&pointer)
                .and_then(Value::as_object)
                .is_some_and(|entries| {
                    entries
                        .values()
                        .any(|entry| nonempty_array(entry.get("values")))
                })
            {
                return Some(pointer);
            }
        }
    }
    None
}

fn has_adverse_transition(state: &State) -> bool {
    state
        .transitions
        .iter()
        .any(|transition| transition.tags.iter().any(|tag| tag == "adverse-decision"))
        || state.states.values().any(has_adverse_transition)
        || state
            .regions
            .values()
            .any(|region| region.states.values().any(has_adverse_transition))
        || state.body.as_deref().is_some_and(has_adverse_transition)
}

pub(super) fn validate(
    kernel: &KernelDocument,
    selected_governance: Option<&Value>,
    companions: &HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    validate_selected(kernel, selected_governance, companions, false)
}

pub(super) fn validate_reference_due_process(
    kernel: &KernelDocument,
    selected_governance: Option<&Value>,
    companions: &HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    validate_selected(kernel, selected_governance, companions, true)
}

fn validate_selected(
    kernel: &KernelDocument,
    selected_governance: Option<&Value>,
    companions: &HashMap<String, Value>,
    native_due_process: bool,
) -> Result<(), RuntimeError> {
    // Kernel §9.5 requires actual compensating action execution. The former
    // name-based record generator was not a compensation owner.
    if kernel
        .execution
        .as_ref()
        .is_some_and(|execution| execution.compensable)
    {
        return Err(refuse(
            "/execution/compensable",
            "Kernel §9.5 compensating action execution",
        ));
    }
    if let (Some(embedded), Some(selected)) = (&kernel.governance, selected_governance) {
        if embedded != selected {
            return Err(refuse(
                "/governance",
                "selected companion conflicts with the governing workflow",
            ));
        }
    }
    let governance = kernel.governance.as_ref().or(selected_governance);
    if !native_due_process
        && kernel
            .impact_level
            .is_some_and(|impact| impact.requires_due_process())
        && kernel.lifecycle.states.values().any(has_adverse_transition)
    {
        // Governance §3 applies to rights/safety adverse capabilities even if a
        // caller omits the policy or disables a required switch. Scope execution
        // belongs to the missing governance owner, not this rejection function.
        return Err(refuse(
            "/governance/dueProcess",
            "Governance §3 notice, explanation, appeal and independent human review",
        ));
    }
    for (document, base) in governance
        .into_iter()
        .map(|document| (document, "/governance"))
        .chain(
            companions
                .get("policyParameters")
                .map(|document| (document, "/companions/policyParameters")),
        )
    {
        if let Some(pointer) = temporal_table_pointer(document) {
            return Err(refuse(
                &format!("{base}{pointer}"),
                "Governance §13 effective-date parameter and binding resolution",
            ));
        }
    }
    if let Some(governance) = governance {
        let due = governance.get("dueProcess");
        if due.and_then(|due| due.get("scope")).is_some() {
            return Err(refuse(
                "/governance/dueProcess/scope",
                "Governance §3.7 scoped due-process enforcement",
            ));
        }
        if let Some(policy) = due.and_then(|due| due.get("adverseDecisionPolicy")) {
            if !native_due_process
                && policy.get("noticeRequired").and_then(Value::as_bool) == Some(true)
            {
                return Err(refuse(
                    "/governance/dueProcess/adverseDecisionPolicy/noticeRequired",
                    "Governance §3.2 delivery before effect, qualified receipt and grace timing (fs-hltb)",
                ));
            }
            if !native_due_process
                && policy
                    .get("counterfactualRequired")
                    .and_then(Value::as_bool)
                    == Some(true)
            {
                return Err(refuse(
                    "/governance/dueProcess/adverseDecisionPolicy/counterfactualRequired",
                    "Governance §3.4 counterfactual explanation",
                ));
            }
            if !native_due_process
                && policy
                    .get("explanationLevel")
                    .and_then(Value::as_str)
                    .is_some_and(|level| level == "individualized")
            {
                return Err(refuse(
                    "/governance/dueProcess/adverseDecisionPolicy/explanationLevel",
                    "Governance §3.3 individualized explanation",
                ));
            }
            if let Some(appeal) = policy.get("appealMechanism") {
                if !native_due_process
                    && (appeal.get("enabled").and_then(Value::as_bool) == Some(true)
                        || appeal
                            .get("continuationOfServices")
                            .and_then(Value::as_bool)
                            == Some(true))
                {
                    return Err(refuse(
                        "/governance/dueProcess/adverseDecisionPolicy/appealMechanism",
                        "Governance §3.5–§3.6 appeal filing, independent review and continuation",
                    ));
                }
            }
        }
        if let Some(protocols) = governance.get("reviewProtocols").and_then(Value::as_array) {
            for (index, binding) in protocols.iter().enumerate() {
                if nonempty_array(binding.get("protocols")) || binding.get("scope").is_some() {
                    return Err(refuse(
                        &format!("/governance/reviewProtocols/{index}"),
                        "Governance §4 actual review work and access ordering",
                    ));
                }
            }
        }
        // Each pointer identifies a configured operation family with no owning
        // reference execution. Empty arrays/objects and descriptive metadata do
        // not activate a family. Native lifecycle tasks, timers, validation and
        // service operations remain available through their existing owners.
        for (pointer, obligation) in [
            (
                "/pipelines",
                "Governance §5 pipeline gates and §8 remediation",
            ),
            ("/delegations", "Governance §11 authority verification"),
            (
                "/holdPolicies",
                "Governance §12 typed hold-policy enforcement",
            ),
            (
                "/holds/policies",
                "Governance §12 typed hold-policy enforcement",
            ),
            ("/escalationLevels", "Governance §10 escalation work"),
        ] {
            if nonempty_array(governance.pointer(pointer)) {
                return Err(refuse(&format!("/governance{pointer}"), obligation));
            }
        }
        for (pointer, obligation) in [
            (
                "/qualityControls/reviewSampling/rate",
                "Governance §7.1 actual review sampling",
            ),
            (
                "/qualityControls/separationOfDuties/scope",
                "Governance §7.2 verified reviewer identity",
            ),
            (
                "/delegation/authority",
                "Governance §11 authority verification",
            ),
            (
                "/delegation/maxDepth",
                "Governance §11 delegation-chain enforcement",
            ),
            (
                "/holds/holdType",
                "Governance §12 typed hold-policy enforcement",
            ),
        ] {
            if governance.pointer(pointer).is_some() {
                return Err(refuse(&format!("/governance{pointer}"), obligation));
            }
        }
        if let Some(authority) = governance
            .pointer("/qualityControls/overrideAuthority")
            .and_then(Value::as_object)
        {
            if !(native_due_process
                && governance
                    .pointer("/dueProcess/execution/preNoticeExtension")
                    .is_some())
                && authority
                    .values()
                    .any(|value| value.as_bool() == Some(true))
            {
                return Err(refuse(
                    "/governance/qualityControls/overrideAuthority",
                    "Governance §7.3 verified override authority and evidence",
                ));
            }
        }
        if let Some(audit) = governance.get("audit").and_then(Value::as_object) {
            for (tier, requirements) in audit {
                let implemented_native_tier = native_due_process
                    && matches!(tier.as_str(), "reasoning" | "counterfactual")
                    && requirements
                        .get("requiredForTags")
                        .and_then(Value::as_array)
                        .is_some_and(|tags| {
                            tags.iter()
                                .all(|tag| tag.as_str() == Some("adverse-decision"))
                        });
                if nonempty_array(requirements.get("requiredForTags")) && !implemented_native_tier {
                    return Err(refuse(
                        &format!("/governance/audit/{tier}"),
                        "Governance §6 actual reasoning/counterfactual evidence",
                    ));
                }
            }
        }
        if let Some(tasks) = governance.get("taskCatalog").and_then(Value::as_array) {
            for (index, task) in tasks.iter().enumerate() {
                if ["slaDefinitions", "warningThresholds", "escalationChain"]
                    .iter()
                    .any(|field| nonempty_array(task.get(*field)))
                    || task
                        .get("breachPolicy")
                        .and_then(Value::as_object)
                        .is_some_and(|policy| !policy.is_empty())
                {
                    return Err(refuse(
                        &format!("/governance/taskCatalog/{index}"),
                        "Governance §10 task SLA, verification and breach operations",
                    ));
                }
            }
        }
    }
    if let (Some(embedded), Some(selected)) = (&kernel.advanced, companions.get("advanced")) {
        if embedded != selected {
            return Err(refuse(
                "/advanced",
                "selected companion conflicts with the governing workflow",
            ));
        }
    }
    let advanced = kernel
        .advanced
        .as_ref()
        .or_else(|| companions.get("advanced"));
    if let Some(advanced) = advanced {
        for (pointer, obligation) in [
            ("/constraintZones", "Advanced §3 DCR activity execution"),
            (
                "/verifiableConstraints",
                "Advanced verification and activation evidence",
            ),
        ] {
            if nonempty_array(advanced.pointer(pointer)) {
                return Err(refuse(&format!("/advanced{pointer}"), obligation));
            }
        }
    }
    Ok(())
}
