// Rust guideline compliant 2026-04-14

//! Handler for `policy-engine` integration bindings.
//!
//! Dispatches a policy evaluation request to an external engine and normalizes
//! its response into the canonical `PolicyDecision` shape.
//!
//! Engine selection is driven by `binding.extensions.engineType`:
//! - `"opa"` — OPA `{result: true|false, reasons?: [...]}` format
//! - `"cedar"` — Cedar `{decision: "Allow"|"Deny", determining_policies: [...]}` format
//! - `"canonical"` (default) — the canonical `{decision: "allow"|"deny"|"indeterminate"}` format
//!
//! `Indeterminate` decisions are emitted as-is. The handler does NOT coerce
//! them to Allow or Deny — the caller governs downstream behavior.

use wos_core::eval::ObservedAction;
use wos_core::model::kernel::KernelDocument;
use wos_core::provenance::{ProvenanceKind, ProvenanceRecord};

use crate::integration::{IntegrationBinding, IntegrationBindingKind};
use crate::milestones::evaluate_milestones;
use crate::policy_decision::PolicyDecision;
use crate::runtime::RuntimeError;
use crate::store::RuntimeRecord;

use super::IntegrationBindingHandler;
use super::request_response::{
    InvocationContext, apply_output_binding, build_integration_input,
    load_or_invoke_service_result, validate_integration_contract,
};

/// Handler for external policy engine evaluation bindings.
pub(crate) struct PolicyEngineHandler;

impl IntegrationBindingHandler for PolicyEngineHandler {
    fn kind(&self) -> IntegrationBindingKind {
        IntegrationBindingKind::PolicyEngine
    }

    fn execute(
        &self,
        ctx: &InvocationContext<'_>,
        record: &mut RuntimeRecord,
        kernel: &KernelDocument,
        observed: &ObservedAction,
        service_ref: &str,
        binding: &IntegrationBinding,
        now_iso: &str,
    ) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
        let mut provenance = Vec::new();

        // Build the policy input (context) from the input_mapping expressions.
        let input = build_integration_input(binding, kernel, observed, &record.instance)?;

        if let Some(prov_record) = validate_integration_contract(
            ctx.validator,
            service_ref,
            "request",
            binding.request_contract.as_ref(),
            &input,
            observed.actor_id.as_deref(),
        )? {
            provenance.push(prov_record);
        }

        let (step_result, _reused) = load_or_invoke_service_result(
            ctx.service,
            record,
            service_ref,
            &input,
            None, // policy evaluation is not idempotency-keyed at the binding level
            now_iso,
        )?;

        // Determine the engine adapter from `extensions.engineType`.
        let engine_type = binding
            .extensions
            .get("engineType")
            .and_then(|v| v.as_str())
            .unwrap_or("canonical");

        let decision = normalize_decision(engine_type, &step_result.output, service_ref)?;

        // Emit PolicyDecision provenance with the canonical shape.
        provenance.push(ProvenanceRecord {
            record_kind: ProvenanceKind::PolicyDecision,
            timestamp: String::new(),
            actor_id: observed.actor_id.clone(),
            from_state: None,
            to_state: None,
            event: None,
            data: Some(serde_json::json!({
                "serviceRef": service_ref,
                "engineType": engine_type,
                "decision": decision.decision,
                "reasonsCount": decision.reasons.len(),
                "obligationsCount": decision.obligations.len(),
                "reasons": decision.reasons,
                "obligations": decision.obligations,
            })),
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
        });

        // Apply the output binding using the canonical decision as the source document.
        // Callers may map `$.decision` to a case-state field (e.g. `caseFile.policyAllowed`).
        // An Indeterminate decision propagates as-is — null coercion is the caller's choice.
        let decision_value = serde_json::to_value(&decision).map_err(|e| {
            RuntimeError::Integration(format!(
                "policy-engine '{service_ref}': failed to serialize decision: {e}"
            ))
        })?;

        let updates = apply_output_binding(
            &mut record.instance.case_state,
            &binding.output_binding,
            &decision_value,
        )?;
        if !updates.is_empty() {
            provenance.push(ProvenanceRecord {
                record_kind: ProvenanceKind::DataMapping,
                timestamp: String::new(),
                actor_id: observed.actor_id.clone(),
                from_state: None,
                to_state: None,
                event: None,
                data: Some(serde_json::json!({
                    "serviceRef": service_ref,
                    "integrationType": binding.kind,
                    "updatedPaths": updates,
                })),
                audit_layer: None,
                actor_type: None,
                lifecycle_state: None,
                definition_version: None,
                inputs: Vec::new(),
                outputs: Vec::new(),
                input_digest: None,
                output_digest: None,
            });
        }

        let post_state = record.instance.case_state.clone();
        let milestone_records = evaluate_milestones(kernel, &mut record.instance, &post_state);
        provenance.extend(milestone_records);

        Ok(provenance)
    }
}

/// Select and invoke the correct `PolicyDecision::from_*` constructor based on engine type.
fn normalize_decision(
    engine_type: &str,
    raw_response: &serde_json::Value,
    service_ref: &str,
) -> Result<PolicyDecision, RuntimeError> {
    let decision = match engine_type {
        "opa" => PolicyDecision::from_opa(raw_response),
        "cedar" => PolicyDecision::from_cedar(raw_response),
        "canonical" => PolicyDecision::from_canonical(raw_response),
        other => {
            return Err(RuntimeError::Integration(format!(
                "policy-engine '{service_ref}': unknown engineType '{other}' \
                 (expected opa|cedar|canonical)"
            )));
        }
    };

    decision.ok_or_else(|| {
        RuntimeError::Integration(format!(
            "policy-engine '{service_ref}': failed to normalize response using engine type \
             '{engine_type}': response was malformed or missing required fields"
        ))
    })
}
