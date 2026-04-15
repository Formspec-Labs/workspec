// Rust guideline compliant 2026-02-21

//! Handler for `request-response` integration bindings.
//!
//! All logic that was previously in `WosRuntime::invoke_request_response_binding`
//! and its private helpers lives here. The handler receives an `InvocationContext`
//! borrow from the runtime; it does not hold any owned runtime state.

use std::collections::HashMap;

use fel_core::{evaluate, fel_to_json, has_error_diagnostics, parse};
use wos_core::eval::{ObservedAction};
use wos_core::instance::CaseInstance;
use wos_core::model::kernel::KernelDocument;
use wos_core::provenance::{ProvenanceKind, ProvenanceRecord};
use wos_core::EvalContext;

use crate::integration::{IntegrationBinding, IntegrationBindingKind, IntegrationContractRef};
use crate::milestones::evaluate_milestones;
use crate::runtime::{RuntimeError, InvokeServicesDyn, ValidateContractsDyn};
use crate::store::{RuntimeRecord, StepResultRecord};

use super::IntegrationBindingHandler;

/// Handler for synchronous request/response HTTP-style bindings.
pub(crate) struct RequestResponseHandler;

impl IntegrationBindingHandler for RequestResponseHandler {
    fn kind(&self) -> IntegrationBindingKind {
        IntegrationBindingKind::RequestResponse
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
                Some(expression) => {
                    Some(value_to_idempotency_key(evaluate_integration_expression(
                        expression,
                        kernel,
                        &record.instance,
                        observed,
                    )?)?)
                }
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
                actor_id: observed.actor_id.clone(),
                from_state: None,
                to_state: None,
                event: None,
                data: Some(serde_json::json!({
                    "serviceRef": service_ref,
                    "integrationType": binding.kind,
                    "idempotencyKey": idempotency_key,
                    "stepResultRecordedAt": step_result.recorded_at,
                })),
            });
        } else {
            provenance.push(ProvenanceRecord {
                record_kind: ProvenanceKind::StepResultPersisted,
                actor_id: observed.actor_id.clone(),
                from_state: None,
                to_state: None,
                event: None,
                data: Some(serde_json::json!({
                    "serviceRef": service_ref,
                    "integrationType": binding.kind,
                    "idempotencyKey": idempotency_key,
                    "input": input,
                    "output": step_result.output,
                    "persistedBeforeAdvance": true,
                })),
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
                actor_id: observed.actor_id.clone(),
                from_state: None,
                to_state: None,
                event: None,
                data: Some(serde_json::json!({
                    "serviceRef": service_ref,
                    "integrationType": binding.kind,
                    "updatedPaths": updates,
                })),
            });
        }

        // Milestone firing: evaluate after durable case-state write from output
        // binding, before any reactive transitions drain (Kernel S4.13).  Records
        // follow any DataMapping record so the provenance stream reads:
        // data changed → milestone fired.
        let post_state = record.instance.case_state.clone();
        let milestone_records = evaluate_milestones(kernel, &mut record.instance, &post_state);
        provenance.extend(milestone_records);

        Ok(provenance)
    }
}

// --- Shared runtime dependencies threaded through from WosRuntime ---

/// Borrowed runtime dependencies needed by integration binding handlers.
///
/// This is a borrow-only view — handlers never own runtime state.
/// The `'r` lifetime ties all borrows to the same runtime call frame.
pub(crate) struct InvocationContext<'r> {
    pub(crate) service: &'r dyn InvokeServicesDyn,
    pub(crate) validator: &'r dyn ValidateContractsDyn,
}

// --- Contract validation helper (extracted from WosRuntime method) ---

pub(crate) fn validate_integration_contract(
    validator: &dyn ValidateContractsDyn,
    service_ref: &str,
    phase: &str,
    contract: Option<&IntegrationContractRef>,
    data: &serde_json::Value,
    actor_id: Option<&str>,
) -> Result<Option<ProvenanceRecord>, RuntimeError> {
    let Some(contract) = contract else {
        return Ok(None);
    };
    let validation_result = validator.validate(&contract.definition_ref, data)?;
    if !validation_result.valid {
        return Err(RuntimeError::ContractValidation(format!(
            "{phase} contract '{}' failed for integration binding '{service_ref}'",
            contract.definition_ref
        )));
    }

    Ok(Some(ProvenanceRecord {
        record_kind: ProvenanceKind::ContractValidation,
        actor_id: actor_id.map(str::to_string),
        from_state: None,
        to_state: None,
        event: None,
        data: Some(serde_json::json!({
            "serviceRef": service_ref,
            "phase": phase,
            "contractRef": contract.definition_ref,
            "structured": true,
            "valid": validation_result.valid,
            "errors": validation_result.errors,
        })),
    }))
}

// --- Service invocation with idempotency replay ---

pub(crate) fn load_or_invoke_service_result(
    service: &dyn InvokeServicesDyn,
    record: &mut RuntimeRecord,
    service_ref: &str,
    input: &serde_json::Value,
    idempotency_key: Option<&str>,
    recorded_at: &str,
) -> Result<(StepResultRecord, bool), RuntimeError> {
    if let Some(existing) = idempotency_key.and_then(|key| {
        record.step_results.iter().find(|result| {
            result.service_ref == service_ref && result.idempotency_key.as_deref() == Some(key)
        })
    }) {
        return Ok((existing.clone(), true));
    }

    let output = service.invoke(service_ref, input, idempotency_key)?;
    let step_result = StepResultRecord {
        service_ref: service_ref.to_string(),
        idempotency_key: idempotency_key.map(str::to_string),
        output,
        recorded_at: recorded_at.to_string(),
    };
    record.step_results.push(step_result.clone());
    Ok((step_result, false))
}

// --- Input construction from case state + observed action ---

fn build_integration_input(
    binding: &IntegrationBinding,
    kernel: &KernelDocument,
    observed: &ObservedAction,
    instance: &CaseInstance,
) -> Result<serde_json::Value, RuntimeError> {
    let mapping = &binding.input_mapping;
    if mapping.is_empty() {
        return Ok(observed
            .action
            .data
            .clone()
            .unwrap_or_else(|| serde_json::json!({})));
    }

    let mut input = serde_json::Map::new();
    for (key, expression) in mapping {
        let value = evaluate_integration_expression(expression, kernel, instance, observed)?;
        input.insert(key.clone(), value);
    }
    Ok(serde_json::Value::Object(input))
}

fn evaluate_integration_expression(
    expression: &str,
    kernel: &KernelDocument,
    instance: &CaseInstance,
    observed: &ObservedAction,
) -> Result<serde_json::Value, RuntimeError> {
    let case_state = case_state_map(&instance.case_state)?;
    let event = integration_event_context(kernel, observed);
    let mut context = EvalContext::from_case_state(&case_state, event.as_ref());
    context.instance.insert(
        "id".to_string(),
        serde_json::Value::String(instance.instance_id.clone()),
    );
    context.instance.insert(
        "definitionUrl".to_string(),
        serde_json::Value::String(instance.definition_url.clone()),
    );
    context.instance.insert(
        "definitionVersion".to_string(),
        serde_json::Value::String(instance.definition_version.clone()),
    );

    let parsed = parse(expression).map_err(|error| {
        RuntimeError::Integration(format!(
            "integration expression '{expression}' failed to parse: {error}"
        ))
    })?;
    let result = evaluate(&parsed, &context.to_fel_environment());
    if has_error_diagnostics(&result.diagnostics) {
        return Err(RuntimeError::Integration(format!(
            "integration expression '{expression}' produced evaluation errors"
        )));
    }

    let value = fel_to_json(&result.value);
    if value.is_null() {
        return Err(RuntimeError::Integration(format!(
            "integration expression '{expression}' resolved to no value"
        )));
    }

    Ok(value)
}

fn integration_event_context(
    kernel: &KernelDocument,
    observed: &ObservedAction,
) -> Option<serde_json::Value> {
    let mut event = serde_json::Map::new();
    if let Some(actor_id) = observed.actor_id.as_deref() {
        event.insert(
            "actorId".to_string(),
            serde_json::Value::String(actor_id.to_string()),
        );
        if let Some(actor_kind) = kernel
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .map(|actor| actor.kind)
        {
            event.insert(
                "actorType".to_string(),
                serde_json::Value::String(actor_kind_to_string(actor_kind).to_string()),
            );
        }
    }
    if let Some(data) = &observed.action.data {
        event.insert("data".to_string(), data.clone());
    }

    if event.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(event))
    }
}

fn actor_kind_to_string(kind: wos_core::model::kernel::ActorKind) -> &'static str {
    match kind {
        wos_core::model::kernel::ActorKind::Human => "human",
        wos_core::model::kernel::ActorKind::System => "system",
    }
}

fn case_state_map(
    case_state: &serde_json::Value,
) -> Result<HashMap<String, serde_json::Value>, RuntimeError> {
    case_state
        .as_object()
        .cloned()
        .map(|object| object.into_iter().collect())
        .ok_or_else(|| RuntimeError::Integration("case state is not an object".to_string()))
}

fn value_to_idempotency_key(value: serde_json::Value) -> Result<String, RuntimeError> {
    match value {
        serde_json::Value::Null => Err(RuntimeError::Integration(
            "idempotency expression resolved to no value".to_string(),
        )),
        serde_json::Value::String(value) => Ok(value),
        serde_json::Value::Bool(_) | serde_json::Value::Number(_) => Ok(value.to_string()),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => Ok(value.to_string()),
    }
}

// --- Output binding: apply service response back to case state ---

fn apply_output_binding(
    case_state: &mut serde_json::Value,
    output_binding: &HashMap<String, String>,
    output: &serde_json::Value,
) -> Result<Vec<String>, RuntimeError> {
    let mut updated_paths = Vec::new();
    let mut bindings: Vec<_> = output_binding.iter().collect();
    bindings.sort_by(|(left, _), (right, _)| left.cmp(right));

    for (case_path, output_path) in bindings {
        let value = resolve_json_path(output, output_path)?;
        set_case_state_path(case_state, case_path, value.clone())?;
        updated_paths.push((*case_path).clone());
    }
    Ok(updated_paths)
}

fn resolve_json_path<'a>(
    value: &'a serde_json::Value,
    json_path: &str,
) -> Result<&'a serde_json::Value, RuntimeError> {
    let segments = parse_json_path(json_path)?;
    let mut current = value;
    for segment in segments {
        current = match segment {
            JsonPathSegment::Key(key) => current.get(&key).ok_or_else(|| {
                RuntimeError::Integration(format!(
                    "output binding path '{json_path}' resolved to no value"
                ))
            })?,
            JsonPathSegment::Index(index) => current
                .as_array()
                .and_then(|items| items.get(index))
                .ok_or_else(|| {
                    RuntimeError::Integration(format!(
                        "output binding path '{json_path}' resolved to no value"
                    ))
                })?,
        };
    }
    Ok(current)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum JsonPathSegment {
    Key(String),
    Index(usize),
}

fn parse_json_path(json_path: &str) -> Result<Vec<JsonPathSegment>, RuntimeError> {
    let json_path = json_path.trim();
    if json_path == "$" {
        return Ok(Vec::new());
    }
    let Some(rest) = json_path.strip_prefix('$') else {
        return Err(RuntimeError::Integration(format!(
            "output binding path '{json_path}' must start with '$'"
        )));
    };

    let mut segments = Vec::new();
    let mut cursor = rest;
    while !cursor.is_empty() {
        if let Some(next) = cursor.strip_prefix('.') {
            cursor = next;
            if cursor.is_empty() {
                return Err(RuntimeError::Integration(format!(
                    "output binding path '{json_path}' has a trailing '.'"
                )));
            }
            let split_at = cursor
                .char_indices()
                .find(|(_, ch)| *ch == '.' || *ch == '[')
                .map(|(index, _)| index)
                .unwrap_or(cursor.len());
            let key = &cursor[..split_at];
            if key.is_empty() {
                return Err(RuntimeError::Integration(format!(
                    "output binding path '{json_path}' contains an empty field name"
                )));
            }
            segments.push(JsonPathSegment::Key(key.to_string()));
            cursor = &cursor[split_at..];
            continue;
        }

        if let Some(next) = cursor.strip_prefix('[') {
            cursor = next;
            let Some(end) = cursor.find(']') else {
                return Err(RuntimeError::Integration(format!(
                    "output binding path '{json_path}' is missing a closing ']'"
                )));
            };
            let token = &cursor[..end];
            if token.is_empty() {
                return Err(RuntimeError::Integration(format!(
                    "output binding path '{json_path}' contains an empty bracket segment"
                )));
            }
            let segment = if let Some(quoted) = token
                .strip_prefix('\'')
                .and_then(|inner| inner.strip_suffix('\''))
                .or_else(|| {
                    token
                        .strip_prefix('"')
                        .and_then(|inner| inner.strip_suffix('"'))
                }) {
                JsonPathSegment::Key(unescape_json_path_key(quoted))
            } else {
                let index = token.parse::<usize>().map_err(|error| {
                    RuntimeError::Integration(format!(
                        "output binding path '{json_path}' contains invalid array index '{token}': {error}"
                    ))
                })?;
                JsonPathSegment::Index(index)
            };
            segments.push(segment);
            cursor = &cursor[end + 1..];
            continue;
        }

        return Err(RuntimeError::Integration(format!(
            "output binding path '{json_path}' has invalid syntax near '{cursor}'"
        )));
    }

    Ok(segments)
}

fn unescape_json_path_key(key: &str) -> String {
    let mut unescaped = String::with_capacity(key.len());
    let mut chars = key.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                unescaped.push(next);
            }
        } else {
            unescaped.push(ch);
        }
    }
    unescaped
}

fn set_case_state_path(
    case_state: &mut serde_json::Value,
    case_path: &str,
    value: serde_json::Value,
) -> Result<(), RuntimeError> {
    let path = case_path.strip_prefix("caseFile.").unwrap_or(case_path);
    if path.is_empty() {
        return Err(RuntimeError::Integration(
            "output binding target path is empty".to_string(),
        ));
    }

    let segments: Vec<&str> = path.split('.').collect();
    let Some((leaf, parents)) = segments.split_last() else {
        return Err(RuntimeError::Integration(
            "output binding target path is empty".to_string(),
        ));
    };

    let mut current = case_state;
    for segment in parents {
        if !current.is_object() {
            *current = serde_json::json!({});
        }
        let object = current.as_object_mut().ok_or_else(|| {
            RuntimeError::Integration(format!(
                "output binding target path '{case_path}' cannot be represented as an object"
            ))
        })?;
        current = object
            .entry((*segment).to_string())
            .or_insert_with(|| serde_json::json!({}));
    }

    if !current.is_object() {
        *current = serde_json::json!({});
    }
    let object = current.as_object_mut().ok_or_else(|| {
        RuntimeError::Integration(format!(
            "output binding target path '{case_path}' cannot be represented as an object"
        ))
    })?;
    object.insert((*leaf).to_string(), value);
    Ok(())
}
