// Rust guideline compliant 2026-04-14

//! Handler for `arazzo-sequence` integration bindings.
//!
//! An Arazzo sequence is a multi-step API orchestration. Each step is declared
//! in `binding.extensions.steps` as an array of `ArazzoStepSpec` objects. Steps
//! execute in order; each step's outputs are accumulated into a `StepContext`
//! so subsequent steps can reference them via `$.steps.<stepId>.output`.
//!
//! The step context passed to `apply_output_binding` is structured as:
//! `{ "steps": { "<stepId>": { "output": <step_response> } } }`
//! This matches the spec path convention `$.steps.<stepId>.output` (singular).
//!
//! Failure semantics: if a step fails, `ArazzoStep { outcome: "failed" }` is
//! emitted for that step, the sequence halts, and the handler returns `Err`.
//! Subsequent steps are not attempted.
//!
//! After all steps succeed, the binding-level `output_binding` may compose
//! final case-state values from the `StepContext` using `$.steps.<stepId>.output`.
//!
//! **WOS v1.0 limitation:** step inputs cannot reference prior step outputs via
//! FEL (`$.steps[...]`). Cross-step data flow is through the sequence-level
//! output binding only. Inter-step references are reserved for Arazzo Engine
//! Binding (§2 of TODO).

use std::collections::HashMap;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use wos_core::eval::ObservedAction;
use wos_core::model::kernel::KernelDocument;
use wos_core::provenance::{ProvenanceKind, ProvenanceRecord};

use crate::integration::{IntegrationBinding, IntegrationBindingKind};
use crate::milestones::evaluate_milestones;
use crate::runtime::RuntimeError;
use crate::store::RuntimeRecord;

use super::IntegrationBindingHandler;
use super::request_response::{
    InvocationContext, apply_output_binding, build_integration_input,
    load_or_invoke_service_result, validate_integration_contract,
};

/// A single step declared in `binding.extensions.steps`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ArazzoStepSpec {
    /// Stable step identifier — used as the key in `$.steps[stepId].outputs`.
    pub step_id: String,

    /// Service reference to invoke for this step.
    pub service_ref: String,

    /// Input mapping for this step (same FEL-over-case-state as top-level).
    #[serde(default)]
    pub input_mapping: HashMap<String, String>,

    /// Output mapping from step response back to case state.
    /// Applied after the step succeeds.
    #[serde(default)]
    pub output_mapping: HashMap<String, String>,

    /// Optional request contract for this step.
    #[serde(default)]
    pub request_contract: Option<crate::integration::IntegrationContractRef>,

    /// Optional response contract for this step.
    #[serde(default)]
    pub response_contract: Option<crate::integration::IntegrationContractRef>,
}

/// Handler for multi-step Arazzo API sequences.
pub(crate) struct ArazzoHandler;

impl IntegrationBindingHandler for ArazzoHandler {
    fn kind(&self) -> IntegrationBindingKind {
        IntegrationBindingKind::ArazzoSequence
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
        let steps = parse_steps(binding, service_ref)?;
        let mut provenance = Vec::new();
        // Accumulated per-step outputs: stepId → step output JSON.
        // Used for building the binding-level step context ($.steps.<stepId>.output).
        let mut step_outputs: HashMap<String, serde_json::Value> = HashMap::new();

        for step in &steps {
            let (step_result_prov, step_output) =
                execute_step(ctx, record, kernel, observed, step, &step_outputs, now_iso);
            let step_provenance = step_result_prov;

            match step_output {
                Ok(output) => {
                    // Accumulate step output for subsequent steps.
                    step_outputs.insert(step.step_id.clone(), output.clone());
                    provenance.extend(step_provenance);

                    // Apply per-step output mapping to case state immediately.
                    if !step.output_mapping.is_empty() {
                        let updates = apply_output_binding(
                            &mut record.instance.case_state,
                            &step.output_mapping,
                            &output,
                        )?;
                        if !updates.is_empty() {
                            provenance.push(ProvenanceRecord {
                                id: ProvenanceRecord::mint_id(),
                                record_kind: ProvenanceKind::DataMapping,
                                timestamp: String::new(),
                                actor_id: observed.actor_id.clone(),
                                from_state: None,
                                to_state: None,
                                event: None,
                                data: Some(serde_json::json!({
                                    "serviceRef": service_ref,
                                    "stepId": step.step_id,
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
                                canonical_event_hash: None,
                                transition_tags: Vec::new(),
                                case_file_snapshot: None,
                                outcome: None,
                            });
                        }
                    }
                }
                Err(err) => {
                    // Step failed — emit failed provenance and halt the sequence.
                    provenance.extend(step_provenance);
                    return Err(RuntimeError::Integration(format!(
                        "Arazzo sequence '{service_ref}': step '{}' failed: {err}",
                        step.step_id
                    )));
                }
            }
        }

        // Binding-level output mapping composes final outputs from all step results.
        // The step context is structured as `{ "steps": { "<stepId>": { "output": <value> } } }`
        // so callers can address step outputs as `$.steps.<stepId>.output` (spec §3.5).
        if !binding.output_binding.is_empty() {
            let steps_map: HashMap<String, serde_json::Value> = step_outputs
                .into_iter()
                .map(|(id, output)| (id, serde_json::json!({ "output": output })))
                .collect();
            let step_context_value = serde_json::json!({ "steps": steps_map });
            let updates = apply_output_binding(
                &mut record.instance.case_state,
                &binding.output_binding,
                &step_context_value,
            )?;
            if !updates.is_empty() {
                provenance.push(ProvenanceRecord {
                    id: ProvenanceRecord::mint_id(),
                    record_kind: ProvenanceKind::DataMapping,
                    timestamp: String::new(),
                    actor_id: observed.actor_id.clone(),
                    from_state: None,
                    to_state: None,
                    event: None,
                    data: Some(serde_json::json!({
                        "serviceRef": service_ref,
                        "integrationType": binding.kind,
                        "phase": "binding-level",
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
                    canonical_event_hash: None,
                    transition_tags: Vec::new(),
                    case_file_snapshot: None,
                    outcome: None,
                });
            }
        }

        let post_state = record.instance.case_state.clone();
        let milestone_records = evaluate_milestones(kernel, &mut record.instance, &post_state);
        provenance.extend(milestone_records);

        Ok(provenance)
    }
}

/// Execute a single Arazzo step, returning the provenance records it produces and
/// either the step output (on success) or an error (on failure).
///
/// The `ArazzoStep` provenance record is always emitted — with `outcome: "ok"` on
/// success and `outcome: "failed"` on error. This ensures the provenance stream
/// always reflects the attempted steps.
fn execute_step(
    ctx: &InvocationContext<'_>,
    record: &mut RuntimeRecord,
    kernel: &KernelDocument,
    observed: &ObservedAction,
    step: &ArazzoStepSpec,
    step_context: &HashMap<String, serde_json::Value>,
    now_iso: &str,
) -> (
    Vec<ProvenanceRecord>,
    Result<serde_json::Value, RuntimeError>,
) {
    let start = Instant::now();
    let mut step_provenance = Vec::new();

    // Build an ephemeral binding for this step using the step's own mappings.
    // We reuse the shared helpers (build_integration_input, etc.) but with
    // a step-scoped IntegrationBinding so input_mapping and contracts apply per-step.
    let step_binding = build_step_binding(step, step_context, kernel, observed, &record.instance);

    let step_binding = match step_binding {
        Ok(b) => b,
        Err(err) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            step_provenance.push(arazzo_step_record(
                observed,
                &step.step_id,
                "failed",
                duration_ms,
            ));
            return (step_provenance, Err(err));
        }
    };

    // Request contract validation.
    let contract_result = validate_integration_contract(
        ctx.validator,
        &step.service_ref,
        "request",
        step.request_contract.as_ref(),
        &step_binding,
        observed.actor_id.as_deref(),
    );
    match contract_result {
        Ok(Some(prov)) => step_provenance.push(prov),
        Ok(None) => {}
        Err(err) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            step_provenance.push(arazzo_step_record(
                observed,
                &step.step_id,
                "failed",
                duration_ms,
            ));
            return (step_provenance, Err(err));
        }
    }

    // Invoke the service.
    // Per-step idempotency keys are not implemented in WOS v1.0.
    // The sequence-level `idempotencyKeyExpression` covers replay at the binding level.
    // Per-step idempotency is deferred to Arazzo Engine Binding (§2 of TODO).
    let invoke_result = load_or_invoke_service_result(
        ctx.service,
        record,
        &step.service_ref,
        &step_binding,
        None,
        now_iso,
    );

    let step_result = match invoke_result {
        Ok((result, _reused)) => result,
        Err(err) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            step_provenance.push(arazzo_step_record(
                observed,
                &step.step_id,
                "failed",
                duration_ms,
            ));
            return (step_provenance, Err(err));
        }
    };

    // Response contract validation.
    let response_contract_result = validate_integration_contract(
        ctx.validator,
        &step.service_ref,
        "response",
        step.response_contract.as_ref(),
        &step_result.output,
        observed.actor_id.as_deref(),
    );
    match response_contract_result {
        Ok(Some(prov)) => step_provenance.push(prov),
        Ok(None) => {}
        Err(err) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            step_provenance.push(arazzo_step_record(
                observed,
                &step.step_id,
                "failed",
                duration_ms,
            ));
            return (step_provenance, Err(err));
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    step_provenance.push(ProvenanceRecord {
        id: ProvenanceRecord::mint_id(),
        record_kind: ProvenanceKind::ArazzoStep,
        timestamp: String::new(),
        actor_id: observed.actor_id.clone(),
        from_state: None,
        to_state: None,
        event: None,
        data: Some(serde_json::json!({
            "stepId": step.step_id,
            "serviceRef": step.service_ref,
            "outcome": "ok",
            "durationMs": duration_ms,
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

    (step_provenance, Ok(step_result.output))
}

/// Build an ephemeral step-scoped input payload.
///
/// The step's `input_mapping` may reference `$.steps[stepId].outputs.*` via
/// the action data field, but since FEL expression evaluation here runs over
/// the current `instance.case_state` (not the step context), we merge the step
/// context into a synthetic `observed` data payload so expressions can access it.
///
/// In this implementation, steps with no `input_mapping` receive the action's
/// own data as input (same as the base request-response handler). Steps with an
/// `input_mapping` evaluate expressions against case state as usual.
fn build_step_binding(
    step: &ArazzoStepSpec,
    step_context: &HashMap<String, serde_json::Value>,
    kernel: &KernelDocument,
    observed: &ObservedAction,
    instance: &wos_core::instance::WorkflowProcess,
) -> Result<serde_json::Value, RuntimeError> {
    if step.input_mapping.is_empty() {
        // No mapping declared: pass the action data as-is.
        return Ok(observed
            .action
            .data
            .clone()
            .unwrap_or_else(|| serde_json::json!({})));
    }

    // Evaluate each expression against the case state.
    // We create a synthetic IntegrationBinding so we can call `build_integration_input`.
    let mut ephemeral_binding = IntegrationBinding {
        kind: IntegrationBindingKind::ArazzoSequence,
        description: None,
        request_contract: None,
        response_contract: None,
        input_mapping: step.input_mapping.clone(),
        context_mapping: HashMap::new(),
        data_mapping: HashMap::new(),
        output_binding: HashMap::new(),
        idempotency_key_expression: None,
        extensions: HashMap::new(),
    };

    // Inject the step context into the extensions so it is accessible via
    // the action data path. In a full Arazzo implementation, step-context
    // references (`$.steps[id].outputs.*`) would be resolved via a dedicated
    // FEL extension; here we surface them in extensions for traceability.
    if !step_context.is_empty() {
        let context_value = serde_json::to_value(step_context).map_err(|e| {
            RuntimeError::Integration(format!("failed to serialize step context: {e}"))
        })?;
        ephemeral_binding
            .extensions
            .insert("stepsContext".to_string(), context_value);
    }

    build_integration_input(&ephemeral_binding, kernel, observed, instance)
}

fn arazzo_step_record(
    observed: &ObservedAction,
    step_id: &str,
    outcome: &str,
    duration_ms: u64,
) -> ProvenanceRecord {
    ProvenanceRecord {
        id: ProvenanceRecord::mint_id(),
        record_kind: ProvenanceKind::ArazzoStep,
        timestamp: String::new(),
        actor_id: observed.actor_id.clone(),
        from_state: None,
        to_state: None,
        event: None,
        data: Some(serde_json::json!({
            "stepId": step_id,
            "outcome": outcome,
            "durationMs": duration_ms,
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
    }
}

fn parse_steps(
    binding: &IntegrationBinding,
    service_ref: &str,
) -> Result<Vec<ArazzoStepSpec>, RuntimeError> {
    let steps_value = binding.extensions.get("steps").ok_or_else(|| {
        RuntimeError::Integration(format!(
            "Arazzo binding '{service_ref}' must declare 'steps' in extensions"
        ))
    })?;

    serde_json::from_value(steps_value.clone()).map_err(|e| {
        RuntimeError::Integration(format!(
            "Arazzo binding '{service_ref}': failed to parse 'steps': {e}"
        ))
    })
}
