// Rust guideline compliant 2026-02-21
//! Lowers explicit fixture invocation declarations to maintained native handlers.
use super::{RuntimeError, WosRuntime};
use crate::integration::{
    IntegrationBinding, IntegrationBindingKind, IntegrationProfileDocument, TargetWorkflow,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use wos_core::model::due_process::DueProcessExecution;
use wos_core::model::kernel::{ActionKind, KernelDocument, State};

/// Closed selected software-fixture invocation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReferenceInvocations {
    /// Exactly referenceInvocations/1.
    pub profile: String,
    /// Native action bindings selected by the workflow itself.
    pub bindings: Vec<ReferenceInvocationBinding>,
}
/// Closed selected invocation kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReferenceInvocationKind {
    RequestResponse,
    OpaPolicy,
}
/// Retained original interface descriptor; no endpoint is invoked by this profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ReferenceInvocationDescriptor {
    /// Selected original OpenAPI operation reference.
    Openapi {
        document_ref: String,
        operation_ref: String,
    },
    /// Selected original OPA endpoint reference.
    Opa { endpoint: String },
}
/// Exact selected native request and response mappings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReferenceInvocationBinding {
    /// Exact invokeService.serviceRef.
    pub id: String,
    /// Maintained native handler to select.
    pub kind: ReferenceInvocationKind,
    /// Retained original authored descriptor.
    pub descriptor: ReferenceInvocationDescriptor,
    /// Native FEL request input expressions.
    pub input_mapping: HashMap<String, String>,
    /// Native response JSONPath to case-file mappings.
    pub output_binding: HashMap<String, String>,
}

/// Validates selected declarations and lowers the maintained handler configuration.
///
/// # Errors
/// Refuses unknown declarations, unreachable missing bindings, invalid expressions,
/// descriptor conflicts and writes overlapping governed service values.
pub fn reference_invocation_profile(
    kernel: &KernelDocument,
) -> Result<Option<IntegrationProfileDocument>, RuntimeError> {
    let Some(raw) = kernel.extensions.get("x-wos-reference-invocations") else {
        return Ok(None);
    };
    let declaration: ReferenceInvocations = serde_json::from_value(raw.clone())
        .map_err(|error| RuntimeError::Integration(error.to_string()))?;
    if declaration.profile != "referenceInvocations/1"
        || declaration.bindings.is_empty()
        || declaration.bindings.len() > 1000
    {
        return Err(RuntimeError::Integration(
            "invalid reference invocation profile".into(),
        ));
    }
    let governed: Option<DueProcessExecution> = kernel
        .governance
        .as_ref()
        .and_then(|value| value.pointer("/dueProcess/execution"))
        .map(|value| serde_json::from_value(value.clone()))
        .transpose()
        .map_err(|error| RuntimeError::Integration(error.to_string()))?;
    let protected: Vec<_> = governed
        .iter()
        .flat_map(|profile| &profile.determinations)
        .flat_map(|binding| &binding.service_pointers)
        .collect();
    let mut bindings = HashMap::new();
    for binding in declaration.bindings {
        if binding.id.is_empty() || bindings.contains_key(&binding.id) {
            return Err(RuntimeError::Integration(
                "duplicate reference invocation".into(),
            ));
        }
        let kind = match (&binding.kind, &binding.descriptor) {
            (
                ReferenceInvocationKind::RequestResponse,
                ReferenceInvocationDescriptor::Openapi {
                    document_ref,
                    operation_ref,
                },
            ) if !document_ref.trim().is_empty() && !operation_ref.trim().is_empty() => {
                IntegrationBindingKind::RequestResponse
            }
            (
                ReferenceInvocationKind::OpaPolicy,
                ReferenceInvocationDescriptor::Opa { endpoint },
            ) if !endpoint.trim().is_empty() => IntegrationBindingKind::PolicyEngine,
            _ => {
                return Err(RuntimeError::Integration(
                    "reference invocation descriptor mismatch".into(),
                ));
            }
        };
        for expression in binding.input_mapping.values() {
            fel_core::parse(expression).map_err(|error| {
                RuntimeError::Integration(format!("invalid reference input expression: {error}"))
            })?;
        }
        for (target, path) in &binding.output_binding {
            let target = target.strip_prefix("caseFile.").unwrap_or(target);
            if target.is_empty() || target.split('.').any(str::is_empty) || path.is_empty() {
                return Err(RuntimeError::Integration(
                    "invalid reference output binding".into(),
                ));
            }
            let pointer = format!(
                "/{}",
                target
                    .split('.')
                    .map(|segment| segment.replace('~', "~0").replace('/', "~1"))
                    .collect::<Vec<_>>()
                    .join("/")
            );
            if protected.iter().any(|protected| {
                pointer.as_str() == protected.as_str()
                    || pointer.starts_with(&format!("{protected}/"))
                    || protected.starts_with(&format!("{pointer}/"))
            }) {
                return Err(RuntimeError::Integration(
                    "reference invocation overlaps protected service value".into(),
                ));
            }
        }
        let mut value = json!({"type":kind,"inputMapping":binding.input_mapping,"outputBinding":binding.output_binding,
            "x-reference-profile":"referenceInvocations/1","x-reference-descriptor":binding.descriptor});
        if kind == IntegrationBindingKind::PolicyEngine {
            value["engineType"] = json!("opa");
        }
        let native: IntegrationBinding = serde_json::from_value(value)
            .map_err(|error| RuntimeError::Integration(error.to_string()))?;
        bindings.insert(binding.id, native);
    }
    fn collect(state: &State, refs: &mut Vec<String>) {
        for action in state.on_entry.iter().chain(&state.on_exit).chain(
            state
                .transitions
                .iter()
                .flat_map(|transition| &transition.actions),
        ) {
            if action.action == ActionKind::InvokeService {
                refs.push(action.service_ref.clone().unwrap_or_default());
            }
        }
        for state in state.states.values() {
            collect(state, refs);
        }
        for region in state.regions.values() {
            for state in region.states.values() {
                collect(state, refs);
            }
        }
        if let Some(body) = state.body.as_ref() {
            collect(body, refs);
        }
    }
    let mut refs = Vec::new();
    for state in kernel.lifecycle.states.values() {
        collect(state, &mut refs);
    }
    let used: HashSet<_> = refs.iter().collect();
    if refs
        .iter()
        .any(|reference| !bindings.contains_key(reference))
        || bindings.keys().any(|id| !used.contains(id))
    {
        return Err(RuntimeError::Integration(
            "missing or unused selected invocation declaration".into(),
        ));
    }
    Ok(Some(IntegrationProfileDocument {
        schema: None,
        target_workflow: TargetWorkflow {
            url: kernel.url.clone().unwrap_or_default(),
            compatible_versions: kernel.version.as_ref().map(|version| format!("={version}")),
        },
        version: None,
        title: None,
        description: None,
        bindings,
        extensions: HashMap::new(),
    }))
}

impl WosRuntime {
    /// Admits only the exact selected fixture binding before prospective effects.
    pub(super) fn qualified_reference_invocation(
        &self,
        kernel: &KernelDocument,
        record: &crate::store::RuntimeRecord,
        observed: &wos_core::eval::ObservedAction,
        prospective_case: &Value,
    ) -> Result<bool, RuntimeError> {
        let Some(service_ref) = observed.action.service_ref.as_deref() else {
            return Ok(false);
        };
        let Some(profile) = reference_invocation_profile(kernel)? else {
            return Ok(false);
        };
        let Some(selected) = profile.bindings.get(service_ref) else {
            return Ok(false);
        };
        if self
            .integration_profile
            .as_ref()
            .and_then(|profile| profile.bindings.get(service_ref))
            != Some(selected)
        {
            return Ok(false);
        }
        let mut process = record.process.clone();
        process.case_state = prospective_case.clone();
        let input = crate::integration_handlers::request_response::build_integration_input(
            selected, kernel, observed, &process,
        )?;
        let binding = serde_json::to_value(selected)
            .map_err(|error| RuntimeError::Integration(error.to_string()))?;
        Ok(self
            .service
            .invocation_identity(service_ref, &input, Some(&binding))?
            .is_some())
    }
}

/// Validates an explicit fixture result with the actual native handler semantics.
///
/// # Errors
/// Rejects malformed provider decisions and unsupported/missing output paths.
pub fn validate_reference_invocation_response(
    profile: &IntegrationProfileDocument,
    service_ref: &str,
    response: &Value,
) -> Result<(), RuntimeError> {
    let binding = profile
        .bindings
        .get(service_ref)
        .ok_or_else(|| RuntimeError::Integration("unselected invocation fixture".into()))?;
    let output = if binding.kind == IntegrationBindingKind::PolicyEngine {
        let decision = crate::integration_handlers::policy_engine::normalize_decision(
            "opa",
            response,
            service_ref,
        )?;
        serde_json::to_value(decision)
            .map_err(|error| RuntimeError::Integration(error.to_string()))?
    } else {
        response.clone()
    };
    crate::integration_handlers::request_response::apply_output_binding(
        &mut json!({}),
        &binding.output_binding,
        &output,
    )?;
    Ok(())
}
