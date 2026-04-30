// Rust guideline compliant 2026-02-21

//! Handler for `request-response` integration bindings.
//!
//! All logic that was previously in `WosRuntime::invoke_request_response_binding`
//! and its private helpers lives here. The handler receives an `InvocationContext`
//! borrow from the runtime; it does not hold any owned runtime state.

use std::collections::HashMap;

use fel_core::{evaluate, fel_to_json, has_error_diagnostics, parse};
use wos_core::EvalContext;
use wos_core::eval::ObservedAction;
use wos_core::instance::CaseInstance;
use wos_core::model::kernel::KernelDocument;
use wos_core::provenance::{ProvenanceKind, ProvenanceRecord};

use crate::integration::{IntegrationBinding, IntegrationBindingKind, IntegrationContractRef};
use crate::milestones::evaluate_milestones;
use crate::runtime::{InvokeServicesDyn, RuntimeError, ValidateContractsDyn};
use crate::store::{RuntimeRecord, StepResultRecord};

use super::{IntegrationBindingHandler, value_to_idempotency_key};

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
                id: ProvenanceRecord::mint_id(),
                record_kind: ProvenanceKind::IdempotencyDedup,
                timestamp: String::new(),
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
        } else {
            provenance.push(ProvenanceRecord {
                id: ProvenanceRecord::mint_id(),
                record_kind: ProvenanceKind::StepResultPersisted,
                timestamp: String::new(),
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
        id: ProvenanceRecord::mint_id(),
        record_kind: ProvenanceKind::ContractValidation,
        timestamp: String::new(),
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

/// Build the CloudEvent `data` payload from a binding's `data_mapping` expressions.
///
/// Uses the same FEL evaluation pipeline as `build_integration_input` but reads
/// from `binding.data_mapping` rather than `binding.input_mapping`. When
/// `data_mapping` is empty the action's own `data` field is used as-is.
pub(crate) fn build_event_data_from_binding(
    binding: &IntegrationBinding,
    kernel: &KernelDocument,
    observed: &ObservedAction,
    instance: &CaseInstance,
) -> Result<serde_json::Value, RuntimeError> {
    let mapping = &binding.data_mapping;
    if mapping.is_empty() {
        return Ok(observed
            .action
            .data
            .clone()
            .unwrap_or_else(|| serde_json::json!({})));
    }

    let mut data = serde_json::Map::new();
    for (key, expression) in mapping {
        let value = evaluate_integration_expression(expression, kernel, instance, observed)?;
        data.insert(key.clone(), value);
    }
    Ok(serde_json::Value::Object(data))
}

/// Build the request body from a binding's `input_mapping` expressions.
///
/// Exposed for reuse by event-style handlers that need the same FEL evaluation
/// pipeline but read from `data_mapping` instead of `input_mapping`.
pub(crate) fn build_integration_input(
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

pub(crate) fn evaluate_integration_expression(
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
        wos_core::model::kernel::ActorKind::Agent => "agent",
    }
}

pub(crate) fn case_state_map(
    case_state: &serde_json::Value,
) -> Result<HashMap<String, serde_json::Value>, RuntimeError> {
    case_state
        .as_object()
        .cloned()
        .map(|object| object.into_iter().collect())
        .ok_or_else(|| RuntimeError::Integration("case state is not an object".to_string()))
}

// --- Output binding: apply service response back to case state ---

/// Apply an output binding map: for each `(case_path, json_path)` entry, resolve
/// the `json_path` against `output` and write the value into `case_state`.
///
/// Exposed for reuse by event-consume and callback handlers.
pub(crate) fn apply_output_binding(
    case_state: &mut serde_json::Value,
    output_binding: &HashMap<String, String>,
    output: &serde_json::Value,
) -> Result<Vec<String>, RuntimeError> {
    let mut updated_paths = Vec::new();
    let mut bindings: Vec<_> = output_binding.iter().collect();
    bindings.sort_by(|(left, _), (right, _)| left.cmp(right));

    for (case_path, output_path) in bindings {
        let value = resolve_json_path(output, output_path)?;
        set_case_state_path(case_state, case_path, value)?;
        updated_paths.push((*case_path).clone());
    }
    Ok(updated_paths)
}

/// Resolve a JSONPath expression against a JSON value using the RFC 9535 output-binding profile.
///
/// The profile supports: root (`$`), member access (`.key`, `['key']`, `["key"]`),
/// index (`[n]`), wildcard (`[*]`), and slice (`[start:end]`, `[start:end:step]`).
/// Filter expressions (`[?(...)]`) and recursive descent (`..`) are rejected at parse
/// time; calling this function only fails at runtime for missing paths.
fn resolve_json_path(
    value: &serde_json::Value,
    json_path: &str,
) -> Result<serde_json::Value, RuntimeError> {
    let segments = parse_json_path(json_path)?;
    resolve_segments(value, &segments, json_path)
}

/// Walk `segments` against `root`, fanning out on Wildcard and Slice.
fn resolve_segments(
    root: &serde_json::Value,
    segments: &[JsonPathSegment],
    json_path: &str,
) -> Result<serde_json::Value, RuntimeError> {
    if segments.is_empty() {
        return Ok(root.clone());
    }

    let (head, tail) = segments.split_first().expect("checked non-empty above");
    match head {
        JsonPathSegment::Key(key) => {
            let next = root.get(key.as_str()).ok_or_else(|| {
                RuntimeError::Integration(format!(
                    "output binding path '{json_path}' resolved to no value"
                ))
            })?;
            resolve_segments(next, tail, json_path)
        }
        JsonPathSegment::Index(index) => {
            let next = root
                .as_array()
                .and_then(|items| items.get(*index))
                .ok_or_else(|| {
                    RuntimeError::Integration(format!(
                        "output binding path '{json_path}' resolved to no value"
                    ))
                })?;
            resolve_segments(next, tail, json_path)
        }
        JsonPathSegment::Wildcard => {
            // Fan out over all elements (array) or values (object).
            let items: Vec<&serde_json::Value> = match root {
                serde_json::Value::Array(arr) => arr.iter().collect(),
                serde_json::Value::Object(obj) => obj.values().collect(),
                _ => {
                    return Err(RuntimeError::Integration(format!(
                        "output binding path '{json_path}': wildcard applied to non-array/object"
                    )));
                }
            };
            if tail.is_empty() {
                Ok(serde_json::Value::Array(
                    items.into_iter().cloned().collect(),
                ))
            } else {
                let results: Result<Vec<serde_json::Value>, _> = items
                    .into_iter()
                    .map(|item| resolve_segments(item, tail, json_path))
                    .collect();
                Ok(serde_json::Value::Array(results?))
            }
        }
        JsonPathSegment::Slice { start, end, step } => {
            let arr = root.as_array().ok_or_else(|| {
                RuntimeError::Integration(format!(
                    "output binding path '{json_path}': slice applied to non-array"
                ))
            })?;
            let len = arr.len() as i64;
            let step = step.unwrap_or(1);
            if step == 0 {
                return Err(RuntimeError::Integration(format!(
                    "output binding path '{json_path}': slice step must not be zero"
                )));
            }
            // Resolve negative/open bounds using Python-style semantics.
            let (start_idx, end_idx) = if step > 0 {
                let s = resolve_slice_bound(*start, len, 0);
                let e = resolve_slice_bound(*end, len, len);
                (s, e)
            } else {
                let s = resolve_slice_bound(*start, len, len - 1);
                let e = resolve_slice_bound(*end, len, -1);
                (s, e)
            };

            let selected: Vec<&serde_json::Value> = if step > 0 {
                (start_idx..end_idx)
                    .step_by(step as usize)
                    .filter_map(|i| arr.get(i as usize))
                    .collect()
            } else {
                let mut result = Vec::new();
                let mut i = start_idx;
                while i > end_idx {
                    if let Some(item) = arr.get(i as usize) {
                        result.push(item);
                    }
                    i += step; // step is negative here
                }
                result
            };

            if tail.is_empty() {
                Ok(serde_json::Value::Array(
                    selected.into_iter().cloned().collect(),
                ))
            } else {
                let results: Result<Vec<serde_json::Value>, _> = selected
                    .into_iter()
                    .map(|item| resolve_segments(item, tail, json_path))
                    .collect();
                Ok(serde_json::Value::Array(results?))
            }
        }
    }
}

/// Normalize a slice bound following RFC 9535 / Python conventions.
///
/// - `None` → `default_value`
/// - Negative → `len + value` (clamped to `[0, len]`)
/// - Non-negative → clamped to `[0, len]`
fn resolve_slice_bound(bound: Option<i64>, len: i64, default_value: i64) -> i64 {
    let raw = match bound {
        None => return default_value,
        Some(v) => v,
    };
    let absolute = if raw < 0 { len + raw } else { raw };
    absolute.clamp(0, len)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum JsonPathSegment {
    Key(String),
    Index(usize),
    Wildcard,
    Slice {
        start: Option<i64>,
        end: Option<i64>,
        step: Option<i64>,
    },
}

/// Parse a JSONPath string into segments using the RFC 9535 output-binding profile.
///
/// Supported: root (`$`), `.key`, `['key']`, `["key"]`, `[n]`, `[*]`,
/// `[start:end]`, `[start:end:step]`.
///
/// Rejected at parse time (returns `RuntimeError::Integration`):
/// - Recursive descent: `..`
/// - Filter expressions: `[?(...)]`
pub(crate) fn parse_json_path(json_path: &str) -> Result<Vec<JsonPathSegment>, RuntimeError> {
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
        // Reject recursive descent at the top of the loop so `$..foo` is caught.
        if cursor.starts_with("..") {
            return Err(RuntimeError::Integration(format!(
                "output binding path '{json_path}': recursive descent (..) is not supported \
                 in the outputBinding profile (RFC 9535 §2.5 feature; use explicit paths instead)"
            )));
        }

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

            // Reject filter expressions.
            if token.starts_with('?') {
                return Err(RuntimeError::Integration(format!(
                    "output binding path '{json_path}': filter expressions ([?(...)]) are not \
                     supported in the outputBinding profile (RFC 9535 §2.6 feature; \
                     use a dedicated binding or post-process the response)"
                )));
            }

            let segment = if token == "*" {
                // Wildcard: [*]
                JsonPathSegment::Wildcard
            } else if let Some(quoted) = token
                .strip_prefix('\'')
                .and_then(|inner| inner.strip_suffix('\''))
                .or_else(|| {
                    token
                        .strip_prefix('"')
                        .and_then(|inner| inner.strip_suffix('"'))
                })
            {
                // Quoted key: ['key'] or ["key"]
                JsonPathSegment::Key(unescape_json_path_key(quoted))
            } else if token.contains(':') {
                // Slice: [start:end] or [start:end:step]
                parse_slice_token(token, json_path)?
            } else {
                // Plain integer index: [n]
                let index = token.parse::<usize>().map_err(|_| {
                    RuntimeError::Integration(format!(
                        "output binding path '{json_path}' contains invalid bracket segment \
                         '{token}': expected an integer index, a quoted key, '*', or a slice"
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

/// Parse a slice token of the form `start:end` or `start:end:step`.
fn parse_slice_token(token: &str, json_path: &str) -> Result<JsonPathSegment, RuntimeError> {
    let parts: Vec<&str> = token.splitn(3, ':').collect();
    let parse_opt = |s: &str| -> Result<Option<i64>, RuntimeError> {
        if s.is_empty() {
            Ok(None)
        } else {
            s.parse::<i64>().map(Some).map_err(|_| {
                RuntimeError::Integration(format!(
                    "output binding path '{json_path}' contains invalid slice bound '{s}': \
                     expected an integer or empty"
                ))
            })
        }
    };
    let start = parse_opt(parts[0])?;
    let end = parse_opt(parts[1])?;
    let step = if parts.len() == 3 {
        parse_opt(parts[2])?
    } else {
        None
    };
    Ok(JsonPathSegment::Slice { start, end, step })
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

#[cfg(test)]
mod jsonpath_tests {
    use super::{JsonPathSegment, parse_json_path, resolve_json_path};
    use serde_json::json;

    // --- Parser tests ---

    #[test]
    fn parse_root_only() {
        assert_eq!(parse_json_path("$").unwrap(), vec![]);
    }

    #[test]
    fn parse_member_dot() {
        assert_eq!(
            parse_json_path("$.foo").unwrap(),
            vec![JsonPathSegment::Key("foo".to_string())]
        );
    }

    #[test]
    fn parse_member_bracket_single_quote() {
        assert_eq!(
            parse_json_path("$['foo']").unwrap(),
            vec![JsonPathSegment::Key("foo".to_string())]
        );
    }

    #[test]
    fn parse_member_bracket_double_quote() {
        assert_eq!(
            parse_json_path("$[\"foo\"]").unwrap(),
            vec![JsonPathSegment::Key("foo".to_string())]
        );
    }

    #[test]
    fn parse_integer_index() {
        assert_eq!(
            parse_json_path("$.items[0]").unwrap(),
            vec![
                JsonPathSegment::Key("items".to_string()),
                JsonPathSegment::Index(0)
            ]
        );
    }

    #[test]
    fn parse_wildcard() {
        assert_eq!(
            parse_json_path("$.items[*]").unwrap(),
            vec![
                JsonPathSegment::Key("items".to_string()),
                JsonPathSegment::Wildcard
            ]
        );
    }

    #[test]
    fn parse_slice_start_end() {
        assert_eq!(
            parse_json_path("$.items[0:2]").unwrap(),
            vec![
                JsonPathSegment::Key("items".to_string()),
                JsonPathSegment::Slice {
                    start: Some(0),
                    end: Some(2),
                    step: None
                }
            ]
        );
    }

    #[test]
    fn parse_slice_open_start() {
        assert_eq!(
            parse_json_path("$.items[-2:]").unwrap(),
            vec![
                JsonPathSegment::Key("items".to_string()),
                JsonPathSegment::Slice {
                    start: Some(-2),
                    end: None,
                    step: None
                }
            ]
        );
    }

    #[test]
    fn parse_slice_with_step() {
        assert_eq!(
            parse_json_path("$.items[::2]").unwrap(),
            vec![
                JsonPathSegment::Key("items".to_string()),
                JsonPathSegment::Slice {
                    start: None,
                    end: None,
                    step: Some(2)
                }
            ]
        );
    }

    #[test]
    fn parse_rejects_recursive_descent() {
        let err = parse_json_path("$..deep").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("recursive descent"),
            "expected recursive descent error, got: {msg}"
        );
    }

    #[test]
    fn parse_rejects_filter_expression() {
        let err = parse_json_path("$[?(@.x>0)]").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("filter expressions"),
            "expected filter expression error, got: {msg}"
        );
    }

    // --- Resolver tests ---

    #[test]
    fn wildcard_last_segment_returns_array_elements() {
        let data = json!({ "items": [1, 2, 3] });
        let result = resolve_json_path(&data, "$.items[*]").unwrap();
        assert_eq!(result, json!([1, 2, 3]));
    }

    #[test]
    fn wildcard_then_key_fans_out() {
        let data = json!({
            "items": [
                { "name": "a" },
                { "name": "b" }
            ]
        });
        let result = resolve_json_path(&data, "$.items[*].name").unwrap();
        assert_eq!(result, json!(["a", "b"]));
    }

    #[test]
    fn slice_start_end() {
        let data = json!({ "items": [10, 20, 30, 40] });
        let result = resolve_json_path(&data, "$.items[0:2]").unwrap();
        assert_eq!(result, json!([10, 20]));
    }

    #[test]
    fn slice_negative_start_open_end() {
        let data = json!({ "items": [10, 20, 30, 40] });
        let result = resolve_json_path(&data, "$.items[-2:]").unwrap();
        assert_eq!(result, json!([30, 40]));
    }

    #[test]
    fn slice_step_two() {
        let data = json!({ "items": [10, 20, 30, 40] });
        let result = resolve_json_path(&data, "$.items[::2]").unwrap();
        assert_eq!(result, json!([10, 30]));
    }

    #[test]
    fn resolver_rejects_recursive_descent_at_parse() {
        let data = json!({ "deep": { "value": 1 } });
        let err = resolve_json_path(&data, "$..deep").unwrap_err();
        assert!(
            err.to_string().contains("recursive descent"),
            "expected recursive descent error"
        );
    }

    #[test]
    fn resolver_rejects_filter_expression_at_parse() {
        let data = json!([{ "x": 1 }]);
        let err = resolve_json_path(&data, "$[?(@.x>0)]").unwrap_err();
        assert!(
            err.to_string().contains("filter expressions"),
            "expected filter expressions error"
        );
    }
}
