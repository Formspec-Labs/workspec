// Rust guideline compliant 2026-04-14

//! Assist Governance Proxy observation (AI S14, AI-050, AI-051, AI-052).
//!
//! Compares direct agent execution against proxied execution to verify
//! the proxy does not weaken enforcement. Owns the behavior observation
//! logic so both `wos-conformance` and `wos-runtime` can use it without
//! circular dependencies.

use std::collections::{BTreeSet, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::deontic;
use crate::event_handler;
use crate::model::ai::{AIIntegrationDocument, ViolationAction};
use crate::model::kernel::ImpactLevel;
use crate::provenance::ProvenanceKind;

/// Evidence that an Assist Governance Proxy preserves required constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistGovernanceProxyEvidence {
    /// Whether proxy-on vs proxy-off behavior produced compatible results.
    #[serde(alias = "differentialCheckPassed")]
    pub differential_check_passed: bool,
    /// Whether the proxy remained identical or stricter on required checks.
    #[serde(alias = "strictnessPreserved")]
    pub strictness_preserved: bool,
    /// Whether required provenance remained present with the proxy enabled.
    #[serde(alias = "provenancePreserved")]
    pub provenance_preserved: bool,
}

#[derive(Debug)]
struct ProxyBehavior {
    blocked: bool,
    requires_escalation: bool,
    violation_ids: BTreeSet<String>,
    proxy_invocation_recorded: bool,
    invocation_source_preserved: bool,
}

/// Compare direct and proxied agent execution to derive AI-050 evidence.
///
/// The proxy path must not weaken enforcement. It may add provenance and may
/// be stricter, but it cannot reduce violations or relax escalation/blocking.
///
/// `differential_check_passed` is computed from the comparison: it is `true`
/// only when the proxy does not introduce fewer violations, does not lower
/// severity, and preserves provenance records.
pub fn observe_assist_governance_proxy(
    ai_doc: &AIIntegrationDocument,
    actor_id: &str,
    event_name: &str,
    event_data: &serde_json::Value,
    case_state: &HashMap<String, serde_json::Value>,
    impact_level: ImpactLevel,
) -> AssistGovernanceProxyEvidence {
    let direct = observe_proxy_behavior(
        ai_doc,
        actor_id,
        event_name,
        event_data,
        case_state,
        impact_level,
        None,
    );
    let proxy = observe_proxy_behavior(
        ai_doc,
        actor_id,
        event_name,
        event_data,
        case_state,
        impact_level,
        Some("assist-proxy"),
    );

    let strictness_preserved = severity_rank(&proxy) >= severity_rank(&direct)
        && proxy.violation_ids.is_superset(&direct.violation_ids);
    let provenance_preserved = proxy.proxy_invocation_recorded && proxy.invocation_source_preserved;

    let differential_check_passed = strictness_preserved && provenance_preserved;

    AssistGovernanceProxyEvidence {
        differential_check_passed,
        strictness_preserved,
        provenance_preserved,
    }
}

fn observe_proxy_behavior(
    ai_doc: &AIIntegrationDocument,
    actor_id: &str,
    event_name: &str,
    event_data: &serde_json::Value,
    case_state: &HashMap<String, serde_json::Value>,
    impact_level: ImpactLevel,
    invocation_source: Option<&str>,
) -> ProxyBehavior {
    let mut data = event_data.clone();
    let Some(data_object) = data.as_object_mut() else {
        return ProxyBehavior {
            blocked: false,
            requires_escalation: false,
            violation_ids: BTreeSet::new(),
            proxy_invocation_recorded: false,
            invocation_source_preserved: false,
        };
    };

    if let Some(source) = invocation_source {
        data_object.insert("invocationSource".to_string(), serde_json::json!(source));
    } else {
        data_object.remove("invocationSource");
    }

    let output = data.get("output").unwrap_or(&serde_json::Value::Null);
    let bypass = data
        .get("deonticBypass")
        .or_else(|| data.get("bypass"))
        .and_then(|value| value.get("rationale"))
        .and_then(|value| value.as_str());
    let escalation_active = data
        .get("escalationActive")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);

    let deontic_result = deontic::evaluate_deontic_constraints(
        ai_doc,
        actor_id,
        output,
        case_state,
        &impact_level,
        bypass,
        escalation_active,
        invocation_source,
    );

    let mut seen_idempotency_keys = HashSet::new();
    let handler_result = event_handler::evaluate_event(
        event_name,
        actor_id,
        &data,
        ai_doc.agents.iter().any(|agent| agent.id == actor_id),
        None,
        &HashMap::new(),
        &mut seen_idempotency_keys,
    );

    let violation_ids = deontic_result
        .provenance
        .iter()
        .filter(|record| record.record_kind == ProvenanceKind::DeonticViolation)
        .filter_map(|record| {
            record
                .data
                .as_ref()
                .and_then(|data| data.get("constraintId"))
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned)
        })
        .collect();

    let invocation_source_preserved = if let Some(source) = invocation_source {
        deontic_result
            .provenance
            .iter()
            .filter(|record| record.record_kind == ProvenanceKind::DeonticViolation)
            .all(|record| {
                record
                    .data
                    .as_ref()
                    .and_then(|data| data.get("invocationSource"))
                    .and_then(|value| value.as_str())
                    == Some(source)
            })
    } else {
        true
    };

    let proxy_invocation_recorded = invocation_source.is_some_and(|source| {
        handler_result.provenance.iter().any(|record| {
            record.record_kind == ProvenanceKind::ProxyInvocation
                && record
                    .data
                    .as_ref()
                    .and_then(|data| data.get("source"))
                    .and_then(|value| value.as_str())
                    == Some(source)
        })
    });

    ProxyBehavior {
        blocked: handler_result.blocked
            || matches!(
                deontic_result.effective_action,
                Some(ViolationAction::Reject)
            ),
        requires_escalation: handler_result.requires_escalation
            || matches!(
                deontic_result.effective_action,
                Some(ViolationAction::EscalateToHuman)
            ),
        violation_ids,
        proxy_invocation_recorded,
        invocation_source_preserved,
    }
}

fn severity_rank(behavior: &ProxyBehavior) -> u8 {
    if behavior.blocked {
        return 2;
    }
    if behavior.requires_escalation {
        return 1;
    }
    0
}
