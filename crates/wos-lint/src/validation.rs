// Rust guideline compliant 2026-02-21
//! Canonical static validation over immutable candidates and explicitly selected dependencies.
//!
//! Schema enforcement, performed semantic checks and skipped check groups remain distinct.
//! This module performs no replay and makes no readiness or publication claim.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{DocumentKind, LintDiagnostic, LintSeverity, WosDocument, WosProject};

/// Requested static validation boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationScope {
    /// Validate the candidate without claiming composed project checks.
    Document,
    /// Validate the candidate and its explicitly selected project dependencies.
    Project,
}

/// The outcome of one required or optional check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CheckStatus {
    /// Every applicable check in the reported group ran successfully.
    Pass,
    /// A performed check found an error or blocking diagnostic.
    Fail,
    /// A requested check could not run or was outside the requested scope.
    NotRun,
}

/// Supported selected dependency kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyRole {
    /// Native Formspec Definition with its exact body identity.
    Definition,
    /// Native Formspec Mapping with an external selected locator.
    Mapping,
    /// Canonical Delivery sidecar joined by targetWorkflow.
    Delivery,
    /// Canonical Ontology Alignment sidecar joined by targetWorkflow.
    OntologyAlignment,
}

/// An exact selected Definition associated with a Mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefinitionSelection {
    /// Canonical Definition URL.
    pub url: String,
    /// Exact selected Definition version, distinct from the Mapping range.
    pub version: String,
}

/// An immutable caller-selected project document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedDependency {
    /// Owning document kind.
    pub role: DependencyRole,
    /// Selected locator; a Mapping has no body URL property.
    pub url: String,
    /// Exact selected document version.
    pub version: String,
    /// Complete native document, never rewritten by validation.
    pub document_json: Value,
    /// Explicit Definition pairing for a Definition-associated Mapping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition: Option<DefinitionSelection>,
}

/// Inputs to canonical static validation.
#[derive(Debug)]
pub struct ValidationRequest<'a> {
    /// Complete immutable candidate document.
    pub candidate: &'a Value,
    /// Explicit immutable selections; validation never discovers or fetches documents.
    pub dependencies: &'a [ResolvedDependency],
    /// Requested boundary, also recorded in the result.
    pub scope: ValidationScope,
}

/// One performed or refused check group with its owning diagnostics.
#[derive(Debug, Clone, Serialize)]
pub struct ValidationStage {
    /// Stable check-group name.
    pub name: String,
    /// Selected source locator, when checking a dependency.
    pub document: Option<String>,
    /// Exact selected dependency version, when checking a dependency.
    pub document_version: Option<String>,
    /// Canonical schema identity read from the owning resource.
    pub schema_id: Option<String>,
    /// Whether this group must pass for the requested static scope.
    pub required: bool,
    /// Pass, fail or explicit not-run.
    pub status: CheckStatus,
    /// Refusal or incomplete-coverage reasons, including actual skipped branch locations.
    pub reasons: Vec<String>,
    /// Exact native WOS diagnostics produced by this check group.
    pub diagnostics: Vec<LintDiagnostic>,
    /// Native Formspec report, preserving its codes, JSONPath and repair metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formspec: Option<Value>,
}

/// Static validation results; success never implies execution or readiness.
#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    /// Scope actually requested by the caller.
    pub scope: ValidationScope,
    /// Aggregate status of required static groups, with failure preceding not-run.
    pub status: CheckStatus,
    /// Deterministic ordered results, including explicit out-of-scope groups.
    pub stages: Vec<ValidationStage>,
    /// Exact native WOS findings, aggregated without recoding them.
    pub diagnostics: Vec<LintDiagnostic>,
}

/// Coverage reported at an actual semantic conversion branch.
#[derive(Debug)]
pub(crate) struct SemanticSkip {
    name: &'static str,
    path: String,
    reason: String,
}
impl SemanticSkip {
    pub(crate) fn new(name: &'static str, path: impl Into<String>, reason: String) -> Self {
        Self {
            name,
            path: path.into(),
            reason,
        }
    }
}

struct Offline;
impl jsonschema::Retrieve for Offline {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Err(format!("schema resource unavailable offline: {uri}").into())
    }
}

const WORKFLOW: &str = include_str!("../../../schemas/wos-workflow.schema.json");
const DELIVERY: &str = include_str!("../../../schemas/sidecars/wos-delivery.schema.json");
const ONTOLOGY: &str = include_str!("../../../schemas/sidecars/wos-ontology-alignment.schema.json");
struct CanonicalSchema {
    id: String,
    validator: Result<jsonschema::Validator, String>,
}
fn compile(schema: &Value) -> Result<jsonschema::Validator, String> {
    // Canonical WOS resources declare Draft 2020-12; inline source schemas retain
    // their own declared draft instead of silently being reinterpreted.
    jsonschema::options()
        .should_validate_formats(true)
        .should_ignore_unknown_formats(false)
        .with_retriever(Offline)
        .build(schema)
        .map_err(|error| error.to_string())
}
fn canonical(text: &str) -> CanonicalSchema {
    let resource: Value = serde_json::from_str(text).expect("canonical schema is valid JSON");
    CanonicalSchema {
        id: resource["$id"]
            .as_str()
            .expect("canonical schema declares its identity")
            .into(),
        validator: compile(&resource),
    }
}
static WORKFLOW_SCHEMA: LazyLock<CanonicalSchema> = LazyLock::new(|| canonical(WORKFLOW));
static DELIVERY_SCHEMA: LazyLock<CanonicalSchema> = LazyLock::new(|| canonical(DELIVERY));
static ONTOLOGY_SCHEMA: LazyLock<CanonicalSchema> = LazyLock::new(|| canonical(ONTOLOGY));

fn stage(name: &str, required: bool, diagnostics: Vec<LintDiagnostic>) -> ValidationStage {
    let status = if diagnostics
        .iter()
        .any(|item| item.severity >= LintSeverity::Error)
    {
        CheckStatus::Fail
    } else {
        CheckStatus::Pass
    };
    ValidationStage {
        name: name.into(),
        document: None,
        document_version: None,
        schema_id: None,
        required,
        status,
        reasons: vec![],
        diagnostics,
        formspec: None,
    }
}
fn not_run(name: &str, required: bool, reason: impl Into<String>) -> ValidationStage {
    let mut result = stage(name, required, vec![]);
    result.status = CheckStatus::NotRun;
    result.reasons.push(reason.into());
    result
}
fn schema_stage(name: &str, document: &Value, schema: &CanonicalSchema) -> ValidationStage {
    let mut result = stage(name, true, vec![]);
    result.schema_id = Some(schema.id.clone());
    match &schema.validator {
        Ok(validator) => {
            result.diagnostics = validator
                .iter_errors(document)
                .map(|error| {
                    let mut diagnostic = LintDiagnostic::t1_error(
                        "WOS-SCHEMA-001",
                        error.instance_path().to_string(),
                        error.to_string(),
                    );
                    diagnostic
                        .related_docs
                        .push(format!("{}#{}", schema.id, error.schema_path()));
                    diagnostic
                })
                .collect();
            if !result.diagnostics.is_empty() {
                result.status = CheckStatus::Fail;
            }
        }
        Err(reason) => {
            result.status = CheckStatus::NotRun;
            result.reasons.push(format!(
                "canonical schema could not compile offline: {reason}"
            ));
        }
    }
    result
}
fn kind(value: &Value) -> Option<DocumentKind> {
    if value.get("$wosWorkflow").is_some() {
        Some(DocumentKind::Workflow)
    } else if value.get("$wosDelivery").is_some() {
        Some(DocumentKind::Delivery)
    } else if value.get("$wosOntologyAlignment").is_some() {
        Some(DocumentKind::OntologyAlignment)
    } else {
        None
    }
}
fn append_skips(stages: &mut Vec<ValidationStage>, skipped: Vec<SemanticSkip>) {
    for skip in skipped {
        stages.push(not_run(
            skip.name,
            true,
            format!("{}: {}", skip.path, skip.reason),
        ));
    }
}
fn sorted_diagnostics(mut diagnostics: Vec<LintDiagnostic>) -> Vec<LintDiagnostic> {
    diagnostics.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.rule_id.cmp(b.rule_id))
            .then(a.message.cmp(&b.message))
    });
    diagnostics
}
fn aggregate(stages: &[ValidationStage]) -> CheckStatus {
    if stages
        .iter()
        .any(|item| item.required && item.status == CheckStatus::Fail)
    {
        CheckStatus::Fail
    } else if stages
        .iter()
        .any(|item| item.required && item.status == CheckStatus::NotRun)
    {
        CheckStatus::NotRun
    } else {
        CheckStatus::Pass
    }
}

/// Validate canonical schemas and the applicable static checks without mutating inputs.
///
/// Document scope explicitly leaves project checks not-run. Project scope uses only
/// supplied selections, one primary workflow and native Formspec validation. Unsupported
/// typed branches remain required not-run stages even when other checks succeed.
/// Replay, reviewed scenario coverage and readiness are outside this operation.
///
/// # Examples
/// ```
/// use serde_json::json;
/// use wos_lint::studio_api::{validate, ValidationRequest, ValidationScope, CheckStatus};
/// let candidate = json!([]);
/// let result = validate(ValidationRequest { candidate: &candidate, dependencies: &[], scope: ValidationScope::Document });
/// assert_eq!(result.status, CheckStatus::Fail);
/// assert_eq!(result.diagnostics[0].path, "");
/// ```
#[must_use]
pub fn validate(request: ValidationRequest<'_>) -> ValidationReport {
    let mut stages = vec![];
    let candidate_kind = kind(request.candidate);
    // Unknown/non-object candidates are tested against the author-time workflow root,
    // preserving the genuine empty JSON Pointer for a whole-document schema error.
    let schema = match candidate_kind {
        Some(DocumentKind::Delivery) => &*DELIVERY_SCHEMA,
        Some(DocumentKind::OntologyAlignment) => &*ONTOLOGY_SCHEMA,
        _ => &*WORKFLOW_SCHEMA,
    };
    stages.push(schema_stage("candidate-schema", request.candidate, schema));
    let mut project = WosProject::default();
    if let Some(kind) = candidate_kind {
        project.push(WosDocument {
            kind,
            value: request.candidate.clone(),
            source: None,
        });
        let mut diagnostics = vec![];
        let mut skipped = vec![];
        crate::rules::tier1::check_with_coverage(
            &project.documents()[0],
            &mut diagnostics,
            &mut skipped,
        );
        stages.push(stage("t1", true, sorted_diagnostics(diagnostics)));
        append_skips(&mut stages, skipped);
    } else {
        stages.push(not_run(
            "t1",
            true,
            "candidate has no supported author-time WOS marker",
        ));
    }
    if request.scope == ValidationScope::Document {
        stages.push(not_run(
            "project-resolution",
            false,
            "document scope does not request composed dependency checks",
        ));
        stages.push(not_run(
            "t2",
            false,
            "document scope does not request project semantic checks",
        ));
    } else {
        let mut selected: Vec<_> = request.dependencies.iter().collect();
        selected.sort_by(|a, b| {
            a.role
                .cmp(&b.role)
                .then(a.url.cmp(&b.url))
                .then(a.version.cmp(&b.version))
                .then_with(|| {
                    a.document_json
                        .to_string()
                        .cmp(&b.document_json.to_string())
                })
                .then_with(|| {
                    a.definition
                        .as_ref()
                        .map(|pair| (&pair.url, &pair.version))
                        .cmp(&b.definition.as_ref().map(|pair| (&pair.url, &pair.version)))
                })
        });
        let mut resolution = vec![];
        let mut unsupported = vec![];
        let mut identities = BTreeSet::new();
        for dependency in &selected {
            let identity = (
                dependency.role,
                dependency.url.as_str(),
                dependency.version.as_str(),
            );
            if !identities.insert(identity) {
                resolution.push(reference_error(
                    "",
                    format!(
                        "duplicate/conflicting selected dependency {}@{}",
                        dependency.url, dependency.version
                    ),
                ));
            }
            if dependency.url.trim().is_empty() || dependency.version.trim().is_empty() {
                resolution.push(reference_error(
                    "",
                    "selected dependency URL/version must be nonempty",
                ));
            }
            validate_dependency(
                dependency,
                &selected,
                request.candidate,
                &mut stages,
                &mut resolution,
                &mut unsupported,
                &mut project,
            );
        }
        if let Some(document) = project.kernel() {
            resolve_workflow(
                &document.value,
                &selected,
                &mut resolution,
                &mut unsupported,
            );
        } else {
            unsupported.push("project T2 requires exactly one primary workflow candidate".into());
        }
        stages.push(stage(
            "project-resolution",
            true,
            sorted_diagnostics(resolution),
        ));
        for reason in unsupported {
            stages.push(not_run("project-resolution-coverage", true, reason));
        }
        if candidate_kind == Some(DocumentKind::Workflow) {
            let mut diagnostics = vec![];
            let mut skipped = vec![];
            crate::rules::tier2::check_with_coverage(&project, &mut diagnostics, &mut skipped);
            stages.push(stage("t2", true, sorted_diagnostics(diagnostics)));
            append_skips(&mut stages, skipped);
        } else {
            stages.push(not_run(
                "t2",
                true,
                "project T2 requires one primary workflow candidate; sidecars cannot substitute it",
            ));
        }
    }
    stages.push(not_run(
        "runtime-replay",
        false,
        "static validation does not run replay, reviewed scenarios or readiness checks",
    ));
    let diagnostics = sorted_diagnostics(
        stages
            .iter()
            .flat_map(|item| item.diagnostics.iter().cloned())
            .collect(),
    );
    ValidationReport {
        scope: request.scope,
        status: aggregate(&stages),
        stages,
        diagnostics,
    }
}

fn reference_error(path: &str, message: impl Into<String>) -> LintDiagnostic {
    LintDiagnostic::t2_error("WOS-REF-001", path, message)
}
fn validate_dependency(
    dependency: &ResolvedDependency,
    selected: &[&ResolvedDependency],
    candidate: &Value,
    stages: &mut Vec<ValidationStage>,
    resolution: &mut Vec<LintDiagnostic>,
    unsupported: &mut Vec<String>,
    project: &mut WosProject,
) {
    let document = &dependency.document_json;
    if dependency.role != DependencyRole::Mapping && dependency.definition.is_some() {
        resolution.push(reference_error(
            "",
            format!(
                "only a Mapping may select a paired Definition: {}",
                dependency.url
            ),
        ));
    }
    if matches!(dependency.role, DependencyRole::Definition)
        && document["url"].as_str() != Some(&dependency.url)
    {
        resolution.push(reference_error(
            "",
            format!(
                "selected Definition URL {} does not match its body",
                dependency.url
            ),
        ));
    }
    if document.get("version").is_some()
        || matches!(
            dependency.role,
            DependencyRole::Definition | DependencyRole::Mapping
        )
    {
        if document["version"].as_str() != Some(&dependency.version) {
            resolution.push(reference_error(
                "",
                format!(
                    "selected {}@{} does not match its body version",
                    dependency.url, dependency.version
                ),
            ));
        }
    }
    match dependency.role {
        DependencyRole::Delivery | DependencyRole::OntologyAlignment => {
            let (expected, schema) = if dependency.role == DependencyRole::Delivery {
                (DocumentKind::Delivery, &*DELIVERY_SCHEMA)
            } else {
                (DocumentKind::OntologyAlignment, &*ONTOLOGY_SCHEMA)
            };
            let mut check = schema_stage("dependency-schema", document, schema);
            check.document = Some(dependency.url.clone());
            check.document_version = Some(dependency.version.clone());
            stages.push(check);
            if document["targetWorkflow"] != candidate["url"] {
                resolution.push(reference_error(
                    "/targetWorkflow",
                    format!(
                        "sidecar {} targets a different primary workflow",
                        dependency.url
                    ),
                ));
            }
            if kind(document) == Some(expected) {
                let doc = WosDocument {
                    kind: expected,
                    value: document.clone(),
                    source: Some(dependency.url.clone()),
                };
                let mut diagnostics = vec![];
                crate::rules::tier1::check(&doc, &mut diagnostics);
                let mut check = stage("dependency-t1", true, sorted_diagnostics(diagnostics));
                check.document = Some(dependency.url.clone());
                check.document_version = Some(dependency.version.clone());
                stages.push(check);
                project.push(doc);
            }
        }
        DependencyRole::Definition | DependencyRole::Mapping => {
            let mut options = formspec_lint::LintOptions {
                mode: formspec_lint::LintMode::Runtime,
                ..Default::default()
            };
            if dependency.role == DependencyRole::Mapping {
                if let Some(source) = document.get("sourceSchema") {
                    if dependency.definition.is_some() {
                        resolution.push(reference_error(
                            "",
                            format!(
                                "external Mapping {} cannot select a Definition pairing",
                                dependency.url
                            ),
                        ));
                    }
                    if let Some(schema) = source.get("schema") {
                        if let Err(reason) = compile(schema) {
                            let mut check = not_run(
                                "formspec-mapping",
                                true,
                                format!("external source schema cannot compile offline: {reason}"),
                            );
                            check.document = Some(dependency.url.clone());
                            check.document_version = Some(dependency.version.clone());
                            stages.push(check);
                            return;
                        }
                    }
                } else {
                    let target = document["definitionRef"].as_str();
                    let definitions: Vec<_> = selected
                        .iter()
                        .copied()
                        .filter(|item| {
                            item.role == DependencyRole::Definition
                                && Some(item.url.as_str()) == target
                                && dependency.definition.as_ref().is_none_or(|pair| {
                                    pair.url == item.url && pair.version == item.version
                                })
                        })
                        .collect();
                    if definitions.len() != 1 {
                        resolution.push(reference_error(
                            "",
                            format!(
                                "Mapping {} requires one explicitly selected associated Definition",
                                dependency.url
                            ),
                        ));
                        unsupported.push(format!(
                            "Mapping {} item-aware checks lack a unique selected Definition",
                            dependency.url
                        ));
                    } else {
                        let definition = definitions[0];
                        if let Some(pair) = &dependency.definition {
                            if Some(pair.url.as_str()) != target {
                                resolution.push(reference_error("", format!("Mapping {} selected pairing conflicts with native definitionRef", dependency.url)));
                            }
                        }
                        match document["definitionVersion"]
                            .as_str()
                            .map(|range| semver_rs::satisfies(&definition.version, range, None))
                        {
                            Some(Ok(true)) => {}
                            Some(Ok(false)) => resolution.push(reference_error(
                                "",
                                format!(
                                    "Mapping {} range does not admit selected Definition {}@{}",
                                    dependency.url, definition.url, definition.version
                                ),
                            )),
                            Some(Err(reason)) => unsupported.push(format!(
                                "Mapping {} version association could not run: {reason}",
                                dependency.url
                            )),
                            None => unsupported.push(format!(
                                "Mapping {} lacks its native Definition version range",
                                dependency.url
                            )),
                        }
                        options.definition_document = Some(definition.document_json.clone());
                    }
                }
            }
            let expected_marker = if dependency.role == DependencyRole::Definition {
                "$formspec"
            } else {
                "$formspecMapping"
            };
            if document.get(expected_marker).is_none() {
                resolution.push(reference_error(
                    "",
                    format!(
                        "selected {} body does not match role {:?}",
                        dependency.url, dependency.role
                    ),
                ));
            }
            let native = formspec_lint::lint_with_options(document, &options);
            let mut check = stage("formspec", true, vec![]);
            check.document = Some(dependency.url.clone());
            check.document_version = Some(dependency.version.clone());
            check.status = if native.valid {
                CheckStatus::Pass
            } else {
                CheckStatus::Fail
            };
            check.formspec = Some(formspec_lint::lint_result_to_json_value(
                &native,
                formspec_core::JsonWireStyle::JsCamel,
            ));
            stages.push(check);
        }
    }
}

fn selected_reference(
    role: DependencyRole,
    url: &str,
    version: Option<&str>,
    path: &str,
    dependencies: &[&ResolvedDependency],
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let found: Vec<_> = dependencies
        .iter()
        .filter(|item| {
            item.role == role
                && item.url == url
                && version.is_none_or(|version| item.version == version)
        })
        .collect();
    if found.len() != 1 {
        diagnostics.push(reference_error(
            path,
            format!(
                "reference {url}{} requires exactly one selected {role:?}; found {}",
                version.map(|value| format!("@{value}")).unwrap_or_default(),
                found.len()
            ),
        ));
    }
}
fn pointer_token(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}
fn resolve_workflow(
    workflow: &Value,
    dependencies: &[&ResolvedDependency],
    diagnostics: &mut Vec<LintDiagnostic>,
    unsupported: &mut Vec<String>,
) {
    if let Some(contracts) = workflow["contracts"].as_object() {
        for (name, contract) in contracts {
            let path = format!("/contracts/{}", pointer_token(name));
            if contract["binding"] == "formspec" {
                if let Some(url) = contract["ref"].as_str() {
                    selected_reference(
                        DependencyRole::Definition,
                        url,
                        contract["definitionVersion"].as_str(),
                        &format!("{path}/ref"),
                        dependencies,
                        diagnostics,
                    );
                }
            } else {
                unsupported.push(format!(
                    "{path}: unsupported contract binding {}; no semantic validation performed",
                    contract["binding"]
                ));
            }
            for field in ["prefillMappingRef", "responseMappingRef"] {
                if let Some(url) = contract[field].as_str() {
                    selected_reference(
                        DependencyRole::Mapping,
                        url,
                        None,
                        &format!("{path}/{field}"),
                        dependencies,
                        diagnostics,
                    );
                }
            }
        }
    }
    for (value, path, version_field) in [
        (&workflow["intake"], "/intake", "contractVersion"),
        (&workflow["caseFile"], "/caseFile", "contractVersion"),
    ] {
        if let Some(url) = value["contractRef"].as_str() {
            selected_reference(
                DependencyRole::Definition,
                url,
                value[version_field].as_str(),
                &format!("{path}/contractRef"),
                dependencies,
                diagnostics,
            );
        }
    }
    if let Some(bindings) = workflow["bindings"].as_array() {
        for (index, binding) in bindings.iter().enumerate() {
            if let Some(url) = binding["contractRef"].as_str() {
                selected_reference(
                    DependencyRole::Definition,
                    url,
                    None,
                    &format!("/bindings/{index}/contractRef"),
                    dependencies,
                    diagnostics,
                );
            }
        }
    }
    // Lifecycle actions have nested states/regions/transition actions/compensation.
    // Follow the owning authored structure and inspect only declared action objects.
    resolve_states(
        &workflow["lifecycle"]["states"],
        "/lifecycle/states",
        workflow,
        dependencies,
        diagnostics,
    );
}
fn resolve_states(
    states: &Value,
    path: &str,
    workflow: &Value,
    dependencies: &[&ResolvedDependency],
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if let Some(states) = states.as_object() {
        for (name, state) in states {
            resolve_state(
                state,
                &format!("{path}/{}", pointer_token(name)),
                workflow,
                dependencies,
                diagnostics,
            );
        }
    }
}
fn resolve_state(
    state: &Value,
    path: &str,
    workflow: &Value,
    dependencies: &[&ResolvedDependency],
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    for phase in ["onEntry", "onExit"] {
        if let Some(actions) = state[phase].as_array() {
            for (index, action) in actions.iter().enumerate() {
                resolve_action(
                    action,
                    &format!("{path}/{phase}/{index}"),
                    workflow,
                    dependencies,
                    diagnostics,
                );
            }
        }
    }
    if let Some(transitions) = state["transitions"].as_array() {
        for (index, transition) in transitions.iter().enumerate() {
            if let Some(actions) = transition["actions"].as_array() {
                for (action_index, action) in actions.iter().enumerate() {
                    resolve_action(
                        action,
                        &format!("{path}/transitions/{index}/actions/{action_index}"),
                        workflow,
                        dependencies,
                        diagnostics,
                    );
                }
            }
        }
    }
    resolve_states(
        &state["states"],
        &format!("{path}/states"),
        workflow,
        dependencies,
        diagnostics,
    );
    if let Some(regions) = state["regions"].as_object() {
        for (name, region) in regions {
            resolve_states(
                &region["states"],
                &format!("{path}/regions/{}/states", pointer_token(name)),
                workflow,
                dependencies,
                diagnostics,
            );
        }
    }
    if let Some(body) = state.get("body") {
        resolve_state(
            body,
            &format!("{path}/body"),
            workflow,
            dependencies,
            diagnostics,
        );
    }
}
fn resolve_action(
    action: &Value,
    path: &str,
    workflow: &Value,
    dependencies: &[&ResolvedDependency],
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if action["action"] == "createTask" {
        if let Some(reference) = action["contractRef"].as_str() {
            if workflow["contracts"].get(reference).is_none() {
                diagnostics.push(reference_error(
                    &format!("{path}/contractRef"),
                    format!("task contract entry {reference} is not declared"),
                ));
            } else if workflow["contracts"][reference]["binding"] == "formspec"
                && workflow["contracts"][reference]["definitionVersion"]
                    .as_str()
                    .is_none_or(|version| version.trim().is_empty())
            {
                diagnostics.push(reference_error(
                    &format!("{path}/contractRef"),
                    "Formspec task contract requires an exact authored Definition version pin",
                ));
            }
        }
        for field in ["prefillMappingRef", "responseMappingRef"] {
            if let Some(url) = action[field].as_str() {
                selected_reference(
                    DependencyRole::Mapping,
                    url,
                    None,
                    &format!("{path}/{field}"),
                    dependencies,
                    diagnostics,
                );
            }
        }
    }
    if let Some(compensation) = action.get("compensatingAction") {
        resolve_action(
            compensation,
            &format!("{path}/compensatingAction"),
            workflow,
            dependencies,
            diagnostics,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn constructed_block_is_a_failed_stage_without_losing_native_fields() {
        let diagnostic: LintDiagnostic = serde_json::from_str(include_str!(
            "../tests/fixtures/shared-validation/constructed-block.json"
        ))
        .unwrap();
        let check = stage("constructed-control", true, vec![diagnostic.clone()]);
        assert_eq!(aggregate(std::slice::from_ref(&check)), CheckStatus::Fail);
        assert_eq!(
            serde_json::to_value(&check.diagnostics[0]).unwrap(),
            serde_json::to_value(diagnostic).unwrap()
        );
    }
}
