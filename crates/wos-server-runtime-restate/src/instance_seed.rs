//! Shared [`CaseInstance`] construction for Restate handlers and the in-memory adapter.

use chrono::Utc;
use wos_core::instance::{CaseInstance, InstanceStatus};
use wos_core::typeid;
use wos_runtime::runtime::CreateInstanceRequest;
use wos_server_ports::runtime::{RuntimeAdapterError, RuntimeResult};

/// Builds the initial [`CaseInstance`] for a create request (matches in-memory adapter).
pub fn case_instance_from_create_request(
    request: &CreateInstanceRequest,
) -> RuntimeResult<CaseInstance> {
    let now = Utc::now().to_rfc3339();
    let tenant = request.tenant.clone().unwrap_or_else(|| {
        if CaseInstance::is_case_id(&request.instance_id) {
            typeid::extract_tenant(&request.instance_id)
                .unwrap_or(typeid::DEFAULT_TENANT)
                .to_string()
        } else {
            typeid::DEFAULT_TENANT.to_string()
        }
    });
    serde_json::from_value(serde_json::json!({
        "instanceId": request.instance_id,
        "tenant": tenant,
        "definitionUrl": request.definition_url,
        "definitionVersion": request.definition_version,
        "configuration": ["intake"],
        "caseState": request.initial_case_state.clone().unwrap_or_else(|| serde_json::json!({})),
        "provenancePosition": 0,
        "nextTaskSequence": 0,
        "timers": [],
        "activeTasks": [],
        "historyStore": {},
        "compensationLogs": {},
        "status": InstanceStatus::Active,
        "pendingEvents": [],
        "createdAt": now,
        "updatedAt": now,
        "firedMilestones": [],
        "pendingCallbacks": {},
        "extensions": {}
    }))
    .map_err(|e| RuntimeAdapterError::Message(format!("failed to build instance: {e}")))
}

/// Restate K/V key for the materialized [`CaseInstance`] snapshot on a virtual object.
pub const STATE_INSTANCE: &str = "wos.caseInstance.v1";
/// Restate K/V key for the pending-event queue (`Vec<PendingEvent>`), legacy Phase 1–2 only.
pub const STATE_QUEUE: &str = "wos.eventQueue.v1";
/// Append-only provenance log JSON (`Vec<ProvenanceRecord>`), Phase 3+ durable record split.
pub const STATE_PROVENANCE_V1: &str = "wos.provenanceLog.v1";
/// Step results, artifacts, and replay map (same wire shape as SBA `runtime_aux_json`).
pub const STATE_AUX_V1: &str = "wos.runtimeAux.v1";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_matches_fixture_shape() {
        let req = CreateInstanceRequest {
            definition_url: "urn:wos:workflow:test:1.0.0".into(),
            definition_version: "1.0.0".into(),
            instance_id: "urn:wos:instance:seed:test".into(),
            tenant: None,
            initial_case_state: None,
        };
        let inst = case_instance_from_create_request(&req).expect("seed");
        assert_eq!(inst.instance_id, "urn:wos:instance:seed:test");
        assert_eq!(inst.configuration, vec!["intake".to_string()]);
        assert_eq!(inst.status, InstanceStatus::Active);
        assert!(inst.pending_events.is_empty());
        assert!(inst.history_store.is_empty());
    }
}
