// Rust guideline compliant 2026-04-14

//! Handler for `tool` integration bindings.
//!
//! Tools are non-HTTP services described by a CWL-informed descriptor. The
//! transport differs from request-response, but the binding lifecycle is
//! identical: input mapping → (optional) request contract → service invoke →
//! idempotency replay → (optional) response contract → output binding.
//!
//! The only behavioral differences from `RequestResponseHandler` are:
//! - The provenance variant is `ToolInvoked` instead of `StepResultPersisted`.
//! - A `toolId` field is read from `binding.extensions.toolId` for the
//!   provenance record (falls back to `service_ref` if absent).

use wos_core::eval::ObservedAction;
use wos_core::model::kernel::KernelDocument;
use wos_core::provenance::{ProvenanceKind, ProvenanceRecord};

use crate::integration::{IntegrationBinding, IntegrationBindingKind};
use crate::milestones::evaluate_milestones;
use crate::runtime::RuntimeError;
use crate::store::RuntimeRecord;

use super::{IntegrationBindingHandler, value_to_idempotency_key};
use super::request_response::{
    InvocationContext, apply_output_binding, build_integration_input,
    evaluate_integration_expression, load_or_invoke_service_result, validate_integration_contract,
};

/// Handler for non-HTTP tool invocation bindings.
pub(crate) struct ToolHandler;

impl IntegrationBindingHandler for ToolHandler {
    fn kind(&self) -> IntegrationBindingKind {
        IntegrationBindingKind::Tool
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

        // The `toolId` is the logical tool identifier. It defaults to `service_ref`
        // when the binding does not declare an explicit `extensions.toolId`.
        let tool_id = binding
            .extensions
            .get("toolId")
            .and_then(|v| v.as_str())
            .unwrap_or(service_ref)
            .to_string();

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

        let idempotency_key = match observed.action.idempotency_key.clone() {
            Some(key) => Some(key),
            None => match binding.idempotency_key_expression.as_deref() {
                Some(expression) => Some(value_to_idempotency_key(evaluate_integration_expression(
                    expression,
                    kernel,
                    &record.instance,
                    observed,
                )?)?),
                None => None,
            },
        };

        let (step_result, reused_persisted_result) = load_or_invoke_service_result(
            ctx.service,
            record,
            service_ref,
            &input,
            idempotency_key.as_deref(),
            now_iso,
        )?;

        if reused_persisted_result {
            provenance.push(ProvenanceRecord {
                record_kind: ProvenanceKind::IdempotencyDedup,
                timestamp: String::new(),
                actor_id: observed.actor_id.clone(),
                from_state: None,
                to_state: None,
                event: None,
                data: Some(serde_json::json!({
                    "toolId": tool_id,
                    "serviceRef": service_ref,
                    "integrationType": binding.kind,
                    "idempotencyKey": idempotency_key,
                    "stepResultRecordedAt": step_result.recorded_at,
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
        } else {
            provenance.push(ProvenanceRecord {
                record_kind: ProvenanceKind::ToolInvoked,
                timestamp: String::new(),
                actor_id: observed.actor_id.clone(),
                from_state: None,
                to_state: None,
                event: None,
                data: Some(serde_json::json!({
                    "toolId": tool_id,
                    "serviceRef": service_ref,
                    "outcome": "ok",
                    "idempotencyKey": idempotency_key,
                    "input": input,
                    "output": step_result.output,
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

        if let Some(prov_record) = validate_integration_contract(
            ctx.validator,
            service_ref,
            "response",
            binding.response_contract.as_ref(),
            &step_result.output,
            observed.actor_id.as_deref(),
        )? {
            provenance.push(prov_record);
        }

        let updates = apply_output_binding(
            &mut record.instance.case_state,
            &binding.output_binding,
            &step_result.output,
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
                    "toolId": tool_id,
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

