// Rust guideline compliant 2026-02-21
//! Executes immutable selected documents through maintained native Formspec APIs.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::Arc,
};

use formspec_core::{DocumentType, MappingDirection, MappingDocument};
use formspec_eval::{
    EvalTrigger, ResolvedOptionLists, exact_json::Value as ExactValue, filed_response_data_matches,
    recheck_eval_options, validated_extension_constraints_from_registry_documents,
};
use formspec_lint::{LintOptions, LintSeverity};
use integrity_canonical::ResponseJson;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use wos_core::{
    instance::ActiveTask,
    model::kernel::{ActionKind, KernelDocument, State},
};
use wos_runtime::{BindingError, CaseMutationBundle, Clock};

use crate::FormspecProcessor;

const _: () = assert!(!formspec_eval::CREATION_COMPILED);

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
    /// Definition URL, Mapping locator, or Registry locator.
    pub url: String,
    /// Exact selected version.
    pub version: String,
    /// Complete original JSON bytes.
    pub bytes: Vec<u8>,
    /// Mandatory exact Definition pairing for a Mapping; optional scoping for a Registry.
    pub definition: Option<DefinitionSelection>,
}

/// Document kinds supported by immutable Formspec task processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedDocumentKind {
    /// Native Formspec Definition.
    Definition,
    /// Native Formspec Mapping.
    Mapping,
    /// Native Formspec Extension Registry document pinned for authoritative recheck.
    Registry,
}

/// Host-supplied remote option lists optionally pinned to an expected canonical digest.
#[derive(Debug, Clone)]
struct TrustedOptionSnapshot {
    lists: ResolvedOptionLists,
    expected_digest: Option<String>,
}

impl TrustedOptionSnapshot {
    fn unpinned(lists: ResolvedOptionLists) -> Self {
        Self {
            lists,
            expected_digest: None,
        }
    }

    fn pinned(lists: ResolvedOptionLists, expected_digest: String) -> Result<Self, BindingError> {
        let snapshot = Self {
            lists,
            expected_digest: Some(expected_digest),
        };
        snapshot.verify()?;
        Ok(snapshot)
    }

    fn verify(&self) -> Result<ResolvedOptionLists, BindingError> {
        if let Some(expected) = &self.expected_digest {
            let actual = self.lists.snapshot_digest().map_err(|error| {
                invalid(format!("resolved option snapshot digest failed: {error}"))
            })?;
            if actual != *expected {
                return Err(invalid(
                    "OPTIONS_UNAVAILABLE: resolved option snapshot digest mismatch",
                ));
            }
        }
        Ok(self.lists.clone())
    }
}

#[derive(Debug, Clone)]
struct SelectedDefinition {
    json: Value,
    exact: ExactValue,
    registry_documents: Vec<ExactValue>,
    resolved_option_lists: Option<TrustedOptionSnapshot>,
}

#[derive(Debug, Clone)]
struct SelectedMapping {
    document: MappingDocument,
    definition: DefinitionSelection,
}

/// Native Formspec processor with fixed selected documents and an injected clock.
///
/// Native evaluation is limited to self-contained Definitions plus explicitly
/// pinned Extension Registry documents and host-supplied remote option lists.
/// External instance, module, and changed-Response reconstruction requirements refuse.
#[derive(Clone)]
pub struct ImmutableFormspecProcessor {
    definitions: BTreeMap<(String, String), SelectedDefinition>,
    mappings: BTreeMap<String, SelectedMapping>,
    registry_documents: Vec<ExactValue>,
    resolved_option_lists: Option<TrustedOptionSnapshot>,
    clock: Option<Arc<dyn Clock + Send + Sync>>,
}

impl std::fmt::Debug for ImmutableFormspecProcessor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ImmutableFormspecProcessor")
            .field("definitions", &self.definitions.keys())
            .field("mappings", &self.mappings.keys())
            .field("registries", &self.registry_documents.len())
            .field(
                "resolved_option_lists_bound",
                &self.resolved_option_lists.is_some(),
            )
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
    /// Refuses invalid documents, duplicate identities, invalid/duplicate registry
    /// constraints, ambiguous locators or pairing.
    pub fn new(documents: &[SelectedDocument]) -> Result<Self, BindingError> {
        let mut global_registries_json = Vec::new();
        let mut global_registries_exact = Vec::new();
        let mut scoped_registries_json: BTreeMap<(String, String), Vec<Value>> = BTreeMap::new();
        let mut scoped_registries_exact: BTreeMap<(String, String), Vec<ExactValue>> =
            BTreeMap::new();
        let mut seen_registries: BTreeSet<(Option<(String, String)>, String, String)> =
            BTreeSet::new();

        for selected in documents
            .iter()
            .filter(|document| document.kind == SelectedDocumentKind::Registry)
        {
            let (document, exact) = parse_registry(selected)?;
            admit(&document, DocumentType::Registry, None, &[])?;
            let scope = selected
                .definition
                .as_ref()
                .map(|pin| (pin.url.clone(), pin.version.clone()));
            if !seen_registries.insert((
                scope.clone(),
                selected.url.clone(),
                selected.version.clone(),
            )) {
                return Err(invalid("duplicate selected Registry identity"));
            }
            if let Some(pin) = scope {
                scoped_registries_json
                    .entry(pin.clone())
                    .or_default()
                    .push(document);
                scoped_registries_exact.entry(pin).or_default().push(exact);
            } else {
                global_registries_json.push(document);
                global_registries_exact.push(exact);
            }
        }
        validated_extension_constraints_from_registry_documents(&global_registries_exact)
            .map_err(|error| invalid(format!("invalid pinned registry documents: {error}")))?;

        let mut definitions: BTreeMap<(String, String), SelectedDefinition> = BTreeMap::new();
        for selected in documents
            .iter()
            .filter(|document| document.kind == SelectedDocumentKind::Definition)
        {
            let (document, exact) = parse_exact(selected)?;
            if selected.definition.is_some() || document["url"] != selected.url {
                return Err(invalid("selected Definition identity or pairing differs"));
            }
            let pin = (selected.url.clone(), selected.version.clone());
            let mut lint_registries = global_registries_json.clone();
            if let Some(scoped) = scoped_registries_json.get(&pin) {
                lint_registries.extend(scoped.iter().cloned());
            }
            let scoped_exact = scoped_registries_exact.remove(&pin).unwrap_or_default();
            if !scoped_exact.is_empty() {
                let mut combined_exact = global_registries_exact.clone();
                combined_exact.extend(scoped_exact.iter().cloned());
                validated_extension_constraints_from_registry_documents(&combined_exact).map_err(
                    |error| invalid(format!("invalid pinned registry documents: {error}")),
                )?;
            }
            admit(&document, DocumentType::Definition, None, &lint_registries)?;
            for field in ["instances", "extensions", "modules"] {
                if document.get(field).is_some_and(nonempty) {
                    return Err(BindingError::Unsupported(format!(
                        "selected Definition {field} qualification is unavailable"
                    )));
                }
            }
            if !document["items"]
                .as_array()
                .is_some_and(|items| response_items_supported(items))
            {
                return Err(BindingError::Unsupported(
                    "selected Definition item structure or creation sources are unsupported".into(),
                ));
            }
            for expression in formspec_lint::expressions::compile_expressions(&document).compiled {
                admit_expression(&expression.expression, true)?;
            }
            if definitions
                .insert(
                    pin,
                    SelectedDefinition {
                        json: document,
                        exact,
                        registry_documents: scoped_exact,
                        resolved_option_lists: None,
                    },
                )
                .is_some()
            {
                return Err(invalid("duplicate selected Definition identity"));
            }
        }
        if !scoped_registries_exact.is_empty() {
            return Err(invalid("Registry Definition is not selected"));
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
            admit(&document, DocumentType::Mapping, Some(&definition.json), &[])?;
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
            registry_documents: global_registries_exact,
            resolved_option_lists: None,
            clock: None,
        })
    }

    /// Binds the same moving clock used by the owning workflow runtime.
    #[must_use]
    pub fn with_clock(mut self, clock: impl Clock + Send + Sync + 'static) -> Self {
        self.clock = Some(Arc::new(clock));
        self
    }

    /// Pins additional raw Extension Registry JSON bytes for authoritative rechecks.
    ///
    /// Preserves exact decimal bounds from raw JSON bytes and fails closed on
    /// malformed registries, duplicate extension names, or unsupported constraints.
    ///
    /// # Errors
    /// Refuses invalid registry documents, duplicate extension names, or unsupported constraints.
    pub fn with_registry_documents(
        mut self,
        registry_documents_bytes: &[&[u8]],
    ) -> Result<Self, BindingError> {
        let mut combined = self.registry_documents.clone();
        let mut added = Vec::with_capacity(registry_documents_bytes.len());
        for raw in registry_documents_bytes {
            let doc_json: Value =
                serde_json::from_slice(raw).map_err(|error| invalid(error.to_string()))?;
            admit(&doc_json, DocumentType::Registry, None, &[])?;
            let exact: ExactValue =
                serde_json::from_slice(raw).map_err(|error| invalid(error.to_string()))?;
            combined.push(exact.clone());
            added.push(exact);
        }
        validated_extension_constraints_from_registry_documents(&combined)
            .map_err(|error| invalid(format!("invalid pinned registry documents: {error}")))?;
        for selected_def in self.definitions.values() {
            if !selected_def.registry_documents.is_empty() {
                let mut per_def = combined.clone();
                per_def.extend(selected_def.registry_documents.iter().cloned());
                validated_extension_constraints_from_registry_documents(&per_def).map_err(
                    |error| invalid(format!("invalid pinned registry documents: {error}")),
                )?;
            }
        }
        self.registry_documents.extend(added);
        Ok(self)
    }

    /// Binds host-supplied resolved option lists for remote `optionSets` and URI `Item.options`.
    #[must_use]
    pub fn with_resolved_option_lists(
        mut self,
        resolved_option_lists: ResolvedOptionLists,
    ) -> Self {
        self.resolved_option_lists = Some(TrustedOptionSnapshot::unpinned(resolved_option_lists));
        self
    }

    /// Binds host-supplied resolved option lists verified against an expected canonical digest.
    ///
    /// # Errors
    /// Refuses snapshots whose canonical [`ResolvedOptionLists::snapshot_digest`] does not match.
    pub fn with_trusted_resolved_option_lists(
        mut self,
        resolved_option_lists: ResolvedOptionLists,
        expected_digest: &str,
    ) -> Result<Self, BindingError> {
        let snapshot =
            TrustedOptionSnapshot::pinned(resolved_option_lists, expected_digest.to_owned())?;
        self.resolved_option_lists = Some(snapshot);
        Ok(self)
    }

    /// Binds resolved option lists (optionally digest-pinned) to a single selected Definition.
    ///
    /// # Errors
    /// Refuses unselected Definitions or mismatched snapshot digests.
    pub fn with_definition_resolved_option_lists(
        mut self,
        definition: &DefinitionSelection,
        resolved_option_lists: ResolvedOptionLists,
        expected_digest: Option<&str>,
    ) -> Result<Self, BindingError> {
        let selected = self
            .definitions
            .get_mut(&(definition.url.clone(), definition.version.clone()))
            .ok_or_else(|| invalid("task Definition is not selected"))?;
        let snapshot = match expected_digest {
            Some(digest) => {
                TrustedOptionSnapshot::pinned(resolved_option_lists, digest.to_owned())?
            }
            None => TrustedOptionSnapshot::unpinned(resolved_option_lists),
        };
        selected.resolved_option_lists = Some(snapshot);
        Ok(self)
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

    fn evaluate(&self, url: &str, version: &str, response: &Value) -> Result<Value, BindingError> {
        let exact_response: ExactValue = response.clone().into();
        self.evaluate_exact(url, version, &exact_response)
    }

    fn evaluate_raw_json(
        &self,
        url: &str,
        version: &str,
        response: &ResponseJson,
    ) -> Result<Value, BindingError> {
        let exact_response: ExactValue = serde_json::from_str(response.as_str())
            .map_err(|error| invalid(format!("invalid exact Response JSON: {error}")))?;
        self.evaluate_exact(url, version, &exact_response)
    }

    fn evaluate_exact(
        &self,
        url: &str,
        version: &str,
        response: &ExactValue,
    ) -> Result<Value, BindingError> {
        let selected_def = self
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
        let fields: HashMap<String, ExactValue> = response
            .get("data")
            .and_then(ExactValue::as_object)
            .ok_or_else(|| invalid("Response data must be an object"))?
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        let stored_subject = response.get("subject");
        let subject = formspec_eval::EvalSubject::for_response(&selected_def.exact, stored_subject);
        let mut registries = self.registry_documents.clone();
        registries.extend(selected_def.registry_documents.iter().cloned());
        let resolved_options = match selected_def
            .resolved_option_lists
            .as_ref()
            .or(self.resolved_option_lists.as_ref())
        {
            Some(snapshot) => snapshot.verify()?,
            None => ResolvedOptionLists::default(),
        };
        let options = recheck_eval_options(
            EvalTrigger::Submit,
            Some(now),
            HashMap::new(),
            &registries,
            resolved_options,
        )
        .map_err(|error| invalid(format!("invalid pinned registry documents: {error}")))?
        .with_subject(subject);
        let result = formspec_eval::evaluate(&selected_def.exact, &fields, &options);
        let filed = ExactValue::Object(fields.into_iter().collect());
        if !result.refused && !filed_response_data_matches(&selected_def.exact, &filed, &result.data)
        {
            return Err(BindingError::Unsupported(
                "filed Response data differs from its evaluation".into(),
            ));
        }
        if !result.diagnostics.is_empty() {
            return Err(invalid(format!(
                "native Definition diagnostics: {:?}",
                result.diagnostics
            )));
        }
        serde_json::from_str(&formspec_eval::evaluation_result_to_json_value(&result).to_string())
            .map_err(|error| invalid(error.to_string()))
    }
}

impl FormspecProcessor for ImmutableFormspecProcessor {
    fn validate_envelope(&self, response: &Value) -> Result<Vec<Value>, BindingError> {
        let definition = if crate::document_attestations::has_document_claims(response) {
            match (
                response["definitionUrl"].as_str(),
                response["definitionVersion"].as_str(),
            ) {
                (Some(url), Some(version)) => self
                    .definitions
                    .get(&(url.into(), version.into()))
                    .map(|selected| selected.json.clone()),
                _ => None,
            }
        } else {
            None
        };
        let result = formspec_lint::lint_with_options(
            response,
            &LintOptions {
                definition_document: definition,
                ..LintOptions::default()
            },
        );
        if result.document_type != Some(DocumentType::Response) {
            return Err(invalid("submission must be a native Formspec Response"));
        }
        Ok(result.diagnostics.into_iter().filter(|diagnostic| diagnostic.severity == LintSeverity::Error)
            .map(|diagnostic| json!({"code":format!("{:?}",diagnostic.code),"path":diagnostic.path,"message":diagnostic.message})).collect())
    }

    fn validate_document_attestation_response(
        &self,
        response: &Value,
    ) -> Result<Vec<Value>, BindingError> {
        self.validate_envelope(response)
    }

    fn validate_response_definition(
        &self,
        url: &str,
        version: &str,
        response: &Value,
    ) -> Result<Option<Vec<Value>>, BindingError> {
        let evaluation = self.evaluate(url, version, response)?;
        Ok(blocking_errors(&evaluation))
    }

    fn validate_response_definition_json(
        &self,
        url: &str,
        version: &str,
        response: &ResponseJson,
    ) -> Result<Option<Vec<Value>>, BindingError> {
        let evaluation = self.evaluate_raw_json(url, version, response)?;
        Ok(blocking_errors(&evaluation))
    }

    fn validate_task_definition_json(
        &self,
        task: &ActiveTask,
        response: &ResponseJson,
    ) -> Result<Option<Vec<Value>>, BindingError> {
        if response.value().get("data").is_none() {
            return Err(invalid("response.data missing"));
        }
        self.validate_response_definition_json(
            task.definition_url.as_deref().unwrap_or_default(),
            task.definition_version.as_deref().unwrap_or_default(),
            response,
        )
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
            .validate_response_definition(
                &selected.definition.url,
                &selected.definition.version,
                response,
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

fn blocking_errors(evaluation: &Value) -> Option<Vec<Value>> {
    let errors: Vec<_> = evaluation["validations"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|validation| validation["severity"] == "error")
        .cloned()
        .collect();
    (!errors.is_empty()).then_some(errors)
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

fn parse_exact(selected: &SelectedDocument) -> Result<(Value, ExactValue), BindingError> {
    let value = parse(selected)?;
    let exact: ExactValue =
        serde_json::from_slice(&selected.bytes).map_err(|error| invalid(error.to_string()))?;
    Ok((value, exact))
}

fn parse_registry(selected: &SelectedDocument) -> Result<(Value, ExactValue), BindingError> {
    let value: Value =
        serde_json::from_slice(&selected.bytes).map_err(|error| invalid(error.to_string()))?;
    if selected.url.trim().is_empty()
        || selected.version.trim().is_empty()
        || value
            .get("url")
            .is_some_and(|actual| actual != selected.url.as_str())
        || value
            .get("version")
            .is_some_and(|actual| actual != selected.version.as_str())
    {
        return Err(invalid("selected Registry identity differs"));
    }
    let exact: ExactValue =
        serde_json::from_slice(&selected.bytes).map_err(|error| invalid(error.to_string()))?;
    Ok((value, exact))
}

fn nonempty(value: &Value) -> bool {
    match value {
        Value::Array(items) => !items.is_empty(),
        Value::Object(fields) => !fields.is_empty(),
        Value::Null => false,
        _ => true,
    }
}

fn response_items_supported(items: &[Value]) -> bool {
    items.iter().all(|item| {
        if item.get("prePopulate").is_some() || item.get("seedFrom").is_some() {
            return false;
        }
        match item["type"].as_str() {
            Some("group") => item["children"]
                .as_array()
                .is_some_and(|children| response_items_supported(children)),
            Some("field" | "display") => {
                item.get("children").is_none() && item.get("repeatable").is_none()
            }
            _ => false,
        }
    })
}

fn admit(
    document: &Value,
    expected: DocumentType,
    definition: Option<&Value>,
    registry_documents: &[Value],
) -> Result<(), BindingError> {
    let report = formspec_lint::lint_with_options(
        document,
        &LintOptions {
            definition_document: definition.cloned(),
            registry_documents: registry_documents.to_vec(),
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

fn admit_expression(expression: &str, in_definition: bool) -> Result<(), BindingError> {
    let ast = fel_core::parse(expression).map_err(|error| invalid(error.to_string()))?;
    let dependencies = fel_core::extract_dependencies(&ast);
    if !dependencies.context_refs.is_empty()
        || !dependencies.instance_reads.is_empty()
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
                        || (in_definition && matches!(function.as_str(), "now" | "today" | "subject")))
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
