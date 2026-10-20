// Rust guideline compliant 2026-02-21
//! Executes immutable selected documents through maintained native Formspec APIs.

use std::{collections::BTreeMap, sync::Arc};

use formspec_core::{DocumentType, MappingDirection, MappingDocument};
use formspec_eval::{EvalContext, EvalOptions, EvalTrigger};
use formspec_lint::{LintOptions, LintSeverity};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use wos_core::{
    instance::ActiveTask,
    model::kernel::{ActionKind, KernelDocument, State},
};
use wos_runtime::{BindingError, CaseMutationBundle, Clock};

use crate::FormspecProcessor;

/// An exact Definition selected independently of a Mapping's version range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefinitionSelection {
    /// Canonical Definition URL.
    pub url: String,
    /// Exact selected Definition version.
    pub version: String,
}

/// Immutable document supplied by the replay owner, without file resolution.
#[derive(Debug, Clone)]
pub struct SelectedDocument {
    /// Owning native document kind.
    pub kind: SelectedDocumentKind,
    /// Definition URL or external Mapping resolution locator.
    pub url: String,
    /// Exact selected version.
    pub version: String,
    /// Complete original JSON bytes.
    pub bytes: Vec<u8>,
    /// Mandatory exact Definition pairing for a Mapping.
    pub definition: Option<DefinitionSelection>,
}

/// Document kinds supported by immutable Formspec task processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedDocumentKind {
    /// Native Formspec Definition.
    Definition,
    /// Native Formspec Mapping.
    Mapping,
}

#[derive(Debug, Clone)]
struct SelectedMapping {
    document: MappingDocument,
    definition: DefinitionSelection,
}

/// Native Formspec processor with fixed selected documents and an injected clock.
///
/// Native evaluation is limited to self-contained Definitions. External instance,
/// extension-provider and changed-Response reconstruction requirements refuse.
#[derive(Clone)]
pub struct ImmutableFormspecProcessor {
    definitions: BTreeMap<(String, String), Value>,
    mappings: BTreeMap<String, SelectedMapping>,
    clock: Option<Arc<dyn Clock + Send + Sync>>,
}

impl std::fmt::Debug for ImmutableFormspecProcessor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ImmutableFormspecProcessor")
            .field("definitions", &self.definitions.keys())
            .field("mappings", &self.mappings.keys())
            .field("clock_bound", &self.clock.is_some())
            .finish()
    }
}

fn invalid(message: impl Into<String>) -> BindingError {
    BindingError::InvalidInput(message.into())
}

impl ImmutableFormspecProcessor {
    /// Admits exact selected documents without fetching or rewriting them.
    ///
    /// # Errors
    /// Refuses invalid documents, duplicate identities, ambiguous locators or pairing.
    pub fn new(documents: &[SelectedDocument]) -> Result<Self, BindingError> {
        let mut definitions = BTreeMap::new();
        for selected in documents
            .iter()
            .filter(|document| document.kind == SelectedDocumentKind::Definition)
        {
            let document = parse(selected)?;
            if selected.definition.is_some() || document["url"] != selected.url {
                return Err(invalid("selected Definition identity or pairing differs"));
            }
            admit(&document, DocumentType::Definition, None)?;
            for field in ["instances", "extensions", "modules"] {
                if document.get(field).is_some_and(nonempty) {
                    return Err(BindingError::Unsupported(format!(
                        "selected Definition {field} qualification is unavailable"
                    )));
                }
            }
            if !document["items"].as_array().is_some_and(|items| {
                items.iter().all(|item| {
                    matches!(item["type"].as_str(), Some("field" | "display"))
                        && ["children", "repeatable", "prePopulate"]
                            .iter()
                            .all(|field| item.get(*field).is_none())
                })
            }) {
                return Err(BindingError::Unsupported(
                    "selected Definition nested/repeated response reconstruction is unavailable"
                        .into(),
                ));
            }
            for expression in formspec_lint::expressions::compile_expressions(&document).compiled {
                admit_expression(&expression.expression, true)?;
            }
            if definitions
                .insert((selected.url.clone(), selected.version.clone()), document)
                .is_some()
            {
                return Err(invalid("duplicate selected Definition identity"));
            }
        }
        let mut mappings = BTreeMap::new();
        for selected in documents
            .iter()
            .filter(|document| document.kind == SelectedDocumentKind::Mapping)
        {
            let document = parse(selected)?;
            let pin = selected
                .definition
                .as_ref()
                .ok_or_else(|| invalid("selected Mapping requires exact Definition pairing"))?;
            let definition = definitions
                .get(&(pin.url.clone(), pin.version.clone()))
                .ok_or_else(|| invalid("Mapping Definition is not selected"))?;
            if document["definitionRef"] != pin.url
                || !document["definitionVersion"].as_str().is_some_and(|range| {
                    matches!(semver_rs::satisfies(&pin.version, range, None), Ok(true))
                })
            {
                return Err(invalid(
                    "selected Mapping Definition pins or version range differ",
                ));
            }
            admit(&document, DocumentType::Mapping, Some(definition))?;
            admit_mapping_expressions(&document)?;
            let parsed =
                formspec_core::parse_mapping_document_from_value(&document).map_err(invalid)?;
            if mappings
                .insert(
                    selected.url.clone(),
                    SelectedMapping {
                        document: parsed,
                        definition: pin.clone(),
                    },
                )
                .is_some()
            {
                return Err(invalid("ambiguous selected Mapping locator"));
            }
        }
        Ok(Self {
            definitions,
            mappings,
            clock: None,
        })
    }

    /// Binds the same moving clock used by the owning workflow runtime.
    #[must_use]
    pub fn with_clock(mut self, clock: impl Clock + Send + Sync + 'static) -> Self {
        self.clock = Some(Arc::new(clock));
        self
    }

    /// Checks every task route and its actual Definition/Mapping relationship.
    ///
    /// # Errors
    /// Refuses missing artifacts, unsupported directions or task pairing drift.
    pub fn validate_workflow(&self, workflow: &KernelDocument) -> Result<(), BindingError> {
        fn state(
            processor: &ImmutableFormspecProcessor,
            workflow: &KernelDocument,
            current: &State,
        ) -> Result<(), BindingError> {
            for action in current
                .on_entry
                .iter()
                .chain(&current.on_exit)
                .chain(
                    current
                        .transitions
                        .iter()
                        .flat_map(|transition| &transition.actions),
                )
                .filter(|action| action.action == ActionKind::CreateTask)
            {
                let Some(contract_ref) = &action.contract_ref else {
                    continue;
                };
                let contract = workflow
                    .contracts
                    .get(contract_ref)
                    .ok_or_else(|| invalid("task contract is missing"))?;
                if contract.binding != "formspec" {
                    continue;
                }
                let pin = DefinitionSelection {
                    url: contract.reference.clone(),
                    version: contract
                        .definition_version
                        .clone()
                        .ok_or_else(|| invalid("task Definition version missing"))?,
                };
                if !processor
                    .definitions
                    .contains_key(&(pin.url.clone(), pin.version.clone()))
                {
                    return Err(invalid("task Definition is not selected"));
                }
                for (reference, direction) in [
                    (
                        action
                            .prefill_mapping_ref
                            .as_ref()
                            .or(contract.prefill_mapping_ref.as_ref()),
                        MappingDirection::Reverse,
                    ),
                    (
                        action
                            .response_mapping_ref
                            .as_ref()
                            .or(contract.response_mapping_ref.as_ref()),
                        MappingDirection::Forward,
                    ),
                ] {
                    if let Some(reference) = reference {
                        if processor.mapping(reference, direction)?.definition != pin {
                            return Err(invalid("task Mapping Definition pairing differs"));
                        }
                    }
                }
            }
            for child in current.states.values() {
                state(processor, workflow, child)?;
            }
            for region in current.regions.values() {
                for child in region.states.values() {
                    state(processor, workflow, child)?;
                }
            }
            if let Some(body) = current.body.as_deref() {
                state(processor, workflow, body)?;
            }
            Ok(())
        }
        for current in workflow.lifecycle.states.values() {
            state(self, workflow, current)?;
        }
        Ok(())
    }

    fn mapping(
        &self,
        reference: &str,
        direction: MappingDirection,
    ) -> Result<&SelectedMapping, BindingError> {
        let selected = self
            .mappings
            .get(reference)
            .ok_or_else(|| invalid("task Mapping is not selected"))?;
        if selected
            .document
            .direction_restriction
            .is_some_and(|allowed| allowed != direction)
        {
            return Err(invalid("task Mapping direction is not permitted"));
        }
        Ok(selected)
    }

    fn evaluate(&self, url: &str, version: &str, data: &Value) -> Result<Value, BindingError> {
        let definition = self
            .definitions
            .get(&(url.into(), version.into()))
            .ok_or_else(|| invalid("task Definition is not selected"))?;
        let clock = self
            .clock
            .as_ref()
            .ok_or_else(|| invalid("selected Formspec runtime clock missing"))?;
        let now = OffsetDateTime::from_unix_timestamp_nanos(i128::from(clock.now_ms()) * 1_000_000)
            .map_err(|error| invalid(error.to_string()))?
            .format(&Rfc3339)
            .map_err(|error| invalid(error.to_string()))?;
        let fields = data
            .as_object()
            .ok_or_else(|| invalid("Response data must be an object"))?
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        let result = formspec_eval::evaluate(
            definition,
            &fields,
            &EvalOptions::default()
                .trigger(EvalTrigger::Submit)
                .context(EvalContext {
                    now_iso: Some(now),
                    ..EvalContext::default()
                }),
        );
        if result.values != fields {
            return Err(BindingError::Unsupported(
                "selected evaluation changed Response values; reconstruction is unavailable".into(),
            ));
        }
        if !result.diagnostics.is_empty() {
            return Err(invalid(format!(
                "native Definition diagnostics: {:?}",
                result.diagnostics
            )));
        }
        Ok(formspec_eval::evaluation_result_to_json_value(&result))
    }
}

impl FormspecProcessor for ImmutableFormspecProcessor {
    fn validate_envelope(&self, response: &Value) -> Result<Vec<Value>, BindingError> {
        let result = formspec_lint::lint(response);
        if result.document_type != Some(DocumentType::Response) {
            return Err(invalid("submission must be a native Formspec Response"));
        }
        Ok(result.diagnostics.into_iter().filter(|diagnostic| diagnostic.severity == LintSeverity::Error)
            .map(|diagnostic| json!({"code":format!("{:?}",diagnostic.code),"path":diagnostic.path,"message":diagnostic.message})).collect())
    }

    fn validate_definition(
        &self,
        url: &str,
        version: &str,
        data: &Value,
    ) -> Result<Option<Vec<Value>>, BindingError> {
        let evaluation = self.evaluate(url, version, data)?;
        let errors: Vec<_> = evaluation["validations"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|validation| validation["severity"] == "error")
            .cloned()
            .collect();
        Ok((!errors.is_empty()).then_some(errors))
    }

    fn compute_prefill(
        &self,
        mapping: Option<&str>,
        case_state: &Value,
    ) -> Result<Option<Value>, BindingError> {
        mapping
            .map(|reference| {
                execute(
                    self.mapping(reference, MappingDirection::Reverse)?,
                    case_state,
                    MappingDirection::Reverse,
                )
            })
            .transpose()
    }

    fn map_task_response(
        &self,
        task: &ActiveTask,
        response: &Value,
    ) -> Result<Option<CaseMutationBundle>, BindingError> {
        let Some(reference) = &task.response_mapping_ref else {
            return Ok(None);
        };
        let selected = self.mapping(reference, MappingDirection::Forward)?;
        if task.definition_url.as_deref() != Some(selected.definition.url.as_str())
            || task.definition_version.as_deref() != Some(selected.definition.version.as_str())
        {
            return Err(invalid("actual task Mapping Definition pins differ"));
        }
        self.map_response(reference, response)
    }

    fn map_response(
        &self,
        reference: &str,
        response: &Value,
    ) -> Result<Option<CaseMutationBundle>, BindingError> {
        let selected = self.mapping(reference, MappingDirection::Forward)?;
        if !self.validate_envelope(response)?.is_empty()
            || response["definitionUrl"] != selected.definition.url
            || response["definitionVersion"] != selected.definition.version
        {
            return Err(invalid(
                "Mapping Response envelope or Definition pins differ",
            ));
        }
        if self
            .validate_definition(
                &selected.definition.url,
                &selected.definition.version,
                &response["data"],
            )?
            .is_some()
        {
            return Err(invalid("Mapping Response Definition validation failed"));
        }
        let output = execute(selected, &response["data"], MappingDirection::Forward)?;
        Ok(Some(CaseMutationBundle {
            field_updates: output
                .as_object()
                .ok_or_else(|| invalid("Mapping case output must be an object"))?
                .clone(),
        }))
    }
}

fn parse(selected: &SelectedDocument) -> Result<Value, BindingError> {
    let value: Value =
        serde_json::from_slice(&selected.bytes).map_err(|error| invalid(error.to_string()))?;
    if selected.url.trim().is_empty()
        || selected.version.trim().is_empty()
        || value["version"] != selected.version
    {
        return Err(invalid("selected document identity differs"));
    }
    Ok(value)
}

fn nonempty(value: &Value) -> bool {
    match value {
        Value::Array(items) => !items.is_empty(),
        Value::Object(fields) => !fields.is_empty(),
        Value::Null => false,
        _ => true,
    }
}

fn admit(
    document: &Value,
    expected: DocumentType,
    definition: Option<&Value>,
) -> Result<(), BindingError> {
    let report = formspec_lint::lint_with_options(
        document,
        &LintOptions {
            definition_document: definition.cloned(),
            ..LintOptions::default()
        },
    );
    if report.document_type != Some(expected) || !report.valid {
        return Err(invalid(format!(
            "selected native document rejected: {:?}",
            report.diagnostics
        )));
    }
    Ok(())
}

fn execute(
    selected: &SelectedMapping,
    source: &Value,
    direction: MappingDirection,
) -> Result<Value, BindingError> {
    let result = formspec_core::execute_mapping_doc(&selected.document, source, direction);
    if !result.diagnostics.is_empty() {
        return Err(invalid(format!(
            "native Mapping diagnostics: {:?}",
            result.diagnostics
        )));
    }
    Ok(result.output)
}

fn admit_expression(expression: &str, clock_available: bool) -> Result<(), BindingError> {
    let ast = fel_core::parse(expression).map_err(|error| invalid(error.to_string()))?;
    let dependencies = fel_core::extract_dependencies(&ast);
    if !dependencies.context_refs.is_empty()
        || !dependencies.instance_refs.is_empty()
        || dependencies.uses_prev_next
    {
        return Err(BindingError::Unsupported(
            "selected expression requires unavailable host context".into(),
        ));
    }
    let functions = formspec_core::analyze_fel(expression).functions;
    if functions.iter().any(|function| {
        !fel_core::extensions::builtin_function_catalog()
            .iter()
            .any(|entry| {
                entry.name == function
                    && (entry.deterministic
                        || (clock_available && matches!(function.as_str(), "now" | "today")))
            })
    }) {
        return Err(BindingError::Unsupported(
            "selected expression requires unavailable function or clock context".into(),
        ));
    }
    Ok(())
}

fn admit_mapping_expressions(value: &Value) -> Result<(), BindingError> {
    match value {
        Value::Array(items) => {
            for item in items {
                admit_mapping_expressions(item)?;
            }
        }
        Value::Object(fields) => {
            for (key, value) in fields {
                if matches!(key.as_str(), "expression" | "condition") {
                    if let Some(expression) = value.as_str() {
                        admit_expression(expression, false)?;
                    }
                } else {
                    admit_mapping_expressions(value)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}
