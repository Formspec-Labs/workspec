// Rust guideline compliant 2026-02-21

//! Replays frozen authoring candidates through the maintained in-memory runtime.
//!
//! Preparation binds all workflow and dependency bytes before execution. Fixture
//! document declarations assert these selections and cannot replace them. Results
//! identify selected native or fixture Formspec processing and fixture services;
//! they establish neither
//! hosted execution nor durable recovery. Studio consumers use this module only.

use std::collections::{BTreeMap, BTreeSet};

use integrity_canonical::{DigestAlgorithm, canonical_json_bytes, compute_digest};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
pub use wos_formspec_binding::selected::DefinitionSelection;
use wos_formspec_binding::selected::{
    ImmutableFormspecProcessor, SelectedDocument, SelectedDocumentKind,
};

use crate::{
    ConformanceError, ConformanceFixture, ConformanceTrace, Delta, Event, Outcome, TraceStep,
    WorkflowEngine,
};

const FIXTURE_ACCESS_POLICY: &str =
    "fixture assignee-only task submission; no delegation; transition/read access permissive";

/// Native observation categories supported by this fixture replay configuration.
pub const SUPPORTED_OBSERVATIONS: &[&str] = &[
    "configuration",
    "caseState",
    "stepResults",
    "activeTasks",
    "timers",
    "provenance",
    "noticeProvenance",
    "guards",
    "drains",
    "taskSubmission",
    "aiPolicy",
];

/// Explicitly selected dependency bytes and their resolution identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolvedDependency {
    /// Existing engine role, for example `kernel@2.0.0` or `businessCalendar`.
    pub role: String,
    /// Exact document locator used for resolution.
    pub url: String,
    /// Selected version, when the document declares one.
    pub version: Option<String>,
    /// Original immutable JSON bytes.
    pub bytes: Vec<u8>,
    /// Exact Definition pairing for a selected native Mapping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition: Option<DefinitionSelection>,
}

/// Independently reviewed expected value at an observed JSON pointer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationExpectation {
    /// JSON pointer into observations, such as `/finalSnapshot/caseState/answer`.
    pub pointer: String,
    /// Reviewed expectation, never used to populate observations.
    pub expected: Value,
}

/// Complete inputs for one authoring scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplayRequest {
    /// Actual selected workflow snapshot bytes.
    pub candidate_bytes: Vec<u8>,
    /// Fully resolved selected dependency bytes.
    pub dependencies: Vec<ResolvedDependency>,
    /// Reviewed fixture inputs and expectations.
    pub fixture_json: String,
    /// Controlled Unix time in milliseconds.
    pub clock_start_ms: u64,
    /// Required observation categories; unsupported categories refuse preparation.
    pub required_observations: Vec<String>,
    /// Independent expected values to compare with native observations.
    pub expectations: Vec<ObservationExpectation>,
}

/// Digests distinguish retained byte identity from canonical JSON content.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentIdentity {
    /// Selected engine role.
    pub role: String,
    /// Exact resolution locator.
    pub url: String,
    /// Selected document version.
    pub version: Option<String>,
    /// SHA-256 of the original bytes.
    pub raw_sha256: String,
    /// SHA-256 of canonical JSON bytes.
    pub canonical_sha256: String,
    /// Exact Definition pairing, independent of the Mapping version range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<DefinitionSelection>,
}

/// Frozen inputs accepted by preparation; execution never rereads paths.
#[derive(Debug, Clone)]
pub struct PreparedReplay {
    fixture: ConformanceFixture,
    identity: String,
    inputs: Vec<DocumentIdentity>,
    clock_start_ms: u64,
    expectations: Vec<ObservationExpectation>,
    runtime_identity: Value,
    retained_bytes: BTreeMap<(String, String, Option<String>), Vec<u8>>,
    selected_formspec: Option<ImmutableFormspecProcessor>,
    assert_transitions: bool,
    reference_adapters: Option<std::sync::Arc<crate::reference_replay::ReferenceReplayAdapters>>,
}

impl PreparedReplay {
    /// Returns the complete immutable native preparation identity.
    pub fn identity(&self) -> &str {
        &self.identity
    }
    /// Returns the selected native source and configuration identity.
    pub fn runtime_identity(&self) -> &Value {
        &self.runtime_identity
    }
    /// Returns the exact retained bytes selected for a document role.
    pub fn input_bytes(&self, role: &str) -> Option<&[u8]> {
        let mut matches = self
            .retained_bytes
            .iter()
            .filter(|((selected, _, _), _)| selected == role);
        let (_, bytes) = matches.next()?;
        matches.next().is_none().then_some(bytes.as_slice())
    }

    /// Returns bytes for one exact selected document identity.
    pub fn input_bytes_for(&self, role: &str, url: &str, version: Option<&str>) -> Option<&[u8]> {
        self.retained_bytes
            .get(&(role.into(), url.into(), version.map(str::to_owned)))
            .map(Vec::as_slice)
    }
}

/// Native process values read after an operation.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSnapshot {
    /// Active state configuration in native document order.
    pub configuration: Vec<String>,
    /// Actual retained native governance state.
    pub governance_state: Value,
    /// Actual current case values.
    pub case_state: Value,
    /// Service outputs read from the saved native runtime record.
    pub step_results: Vec<wos_runtime::store::StepResultRecord>,
    /// Actual native creation/completion/artifact association for every task.
    pub task_history: Value,
    /// Native active-task records.
    pub active_tasks: Value,
    /// Native pending-timer records.
    pub timers: Value,
    /// Actual ordered timer creation/firing/cancellation associations.
    pub timer_history: Value,
    /// Actual selected process definition version.
    pub definition_version: String,
    /// Observed provenance cursor used to retain each attempt's records.
    pub provenance_position: usize,
}

/// One attempted operation, including rejections and operations with no transition.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayAttempt {
    /// Original fixture event index, shared with its preceding clock advancement.
    pub event_index: usize,
    /// `clock`, `event`, `taskSubmission`, or `migration`.
    pub kind: String,
    /// Requested event name; clock advancements use `$clock`.
    pub event: String,
    /// Actual submitted actor, when supplied.
    pub actor: Option<String>,
    /// Submitted event payload.
    pub payload: Option<Value>,
    /// Controlled execution time.
    pub clock_ms: u64,
    /// Actual process values before the operation.
    pub before: Option<RuntimeSnapshot>,
    /// Actual process values after the operation.
    pub after: Option<RuntimeSnapshot>,
    /// Actual native drain results, preserving ordering and zero-transition results.
    pub drains: Vec<wos_runtime::DrainOnceResult>,
    /// Actual task submission result, including `Rejected` and `Failed`.
    pub submission_result: Option<Value>,
    /// Native migration outcome, retained before any later readback failure.
    pub migration_result: Option<Value>,
    /// Actual typed governed operation result, including refusals.
    pub governance_result: Option<Value>,
    /// Original and resolved native recipe/signature inputs.
    pub recipe_resolution: Option<Value>,
    /// Actual typed unsupported-operation refusals; never inferred from payload fields.
    pub refusals: Vec<String>,
    /// Native runtime error, when the operation rejected with an error.
    pub error: Option<String>,
    /// Actual native provenance produced during this operation.
    pub provenance: Option<Vec<crate::ProvenanceRecord>>,
    /// Failed observation reads, independent of the original operation error.
    pub observation_errors: Vec<String>,
}

/// Independently expected value differs from an actual observation.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationDifference {
    /// Location in the observation document.
    pub pointer: String,
    /// Reviewed expected value.
    pub expected: Value,
    /// Actual value; absent observations stay absent.
    pub actual: Option<Value>,
}

/// Native replay evidence with explicit execution and observation limits.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayResult {
    /// True only if every supported expectation passed and no observation refused.
    pub passed: bool,
    /// `pass`, `fail`, or `refused`.
    pub status: String,
    /// Digest identifying the exact frozen inputs and runtime configuration.
    pub identity: String,
    /// Original-byte and canonical identities for candidate, dependencies, and fixture.
    pub inputs: Vec<DocumentIdentity>,
    /// Compile-time scoped native source identity and execution configuration.
    pub runtime_identity: Value,
    /// Labels explaining fixture and native execution boundaries.
    pub execution: Value,
    /// Native expectation failures.
    pub failures: Vec<String>,
    /// Unsupported or missing observations, never treated as passing values.
    pub refusals: Vec<String>,
    /// Independent expected-versus-observed differences.
    pub differences: Vec<ObservationDifference>,
    /// Ordered native attempt observations.
    pub attempts: Vec<ReplayAttempt>,
    /// Final native process values.
    pub final_snapshot: Option<RuntimeSnapshot>,
    /// Native provenance, without expected-value supplementation.
    pub provenance: Option<Vec<crate::ProvenanceRecord>>,
    /// Maintained trace, including no-transition attempts.
    pub trace: ConformanceTrace,
    /// Native process creation request/result, or its original error.
    pub initialization: Value,
    /// Observation readback failures; none replace the operation error.
    pub observation_errors: Vec<String>,
}

fn refused(reason: impl Into<String>) -> ConformanceError {
    ConformanceError::Parse(format!("replay-refused: {}", reason.into()))
}

fn parse(bytes: &[u8], role: &str) -> Result<Value, ConformanceError> {
    serde_json::from_slice(bytes).map_err(|error| refused(format!("invalid-json {role}: {error}")))
}

fn digest(bytes: &[u8]) -> String {
    compute_digest(bytes, DigestAlgorithm::Sha256)
}
fn content_digest(value: &Value) -> Result<String, ConformanceError> {
    canonical_json_bytes(value)
        .map(|bytes| digest(&bytes))
        .map_err(refused)
}
fn document_identity(
    role: &str,
    url: &str,
    version: Option<String>,
    bytes: &[u8],
    value: &Value,
) -> Result<DocumentIdentity, ConformanceError> {
    Ok(DocumentIdentity {
        role: role.into(),
        url: url.into(),
        version,
        raw_sha256: digest(bytes),
        canonical_sha256: content_digest(value)?,
        definition: None,
    })
}

fn check_fields(value: &Value, allowed: &[&str], context: &str) -> Result<(), ConformanceError> {
    let fields = value
        .as_object()
        .ok_or_else(|| refused(format!("invalid-object {context}")))?;
    if let Some(field) = fields.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(refused(format!(
            "unsupported-scenario-field {context}.{field}"
        )));
    }
    Ok(())
}

fn check_supported_workflow(value: &Value, role: &str) -> Result<(), ConformanceError> {
    let kernel: wos_core::KernelDocument = serde_json::from_value(value.clone())
        .map_err(|error| refused(format!("invalid-workflow {role}: {error}")))?;
    if kernel.wos_workflow != "1.0"
        || kernel.url.as_deref().is_none_or(str::is_empty)
        || kernel.version.as_deref().is_none_or(str::is_empty)
        || !kernel
            .lifecycle
            .states
            .contains_key(&kernel.lifecycle.initial_state)
    {
        return Err(refused(format!(
            "invalid-workflow-identity-or-initial-state {role}"
        )));
    }
    if !kernel.bindings.is_empty() {
        return Err(refused(
            "unsupported-governed-output-binding: canonical OutputBinding execution is unavailable; task Formspec mapping is a separate binding capability",
        ));
    }
    if let Some(custody) = value.get("custody") {
        let neutral = custody.as_object().is_some_and(|block| {
            block.is_empty()
                || (block.len() == 1
                    && block
                        .get("anchorRequirements")
                        .is_some_and(|anchors| anchors.as_array().is_some_and(Vec::is_empty)))
        });
        if !neutral {
            return Err(refused(format!(
                "unsupported-embedded-behavior {role}.custody"
            )));
        }
    }
    for field in ["assurance", "intake"] {
        if value.get(field).is_some_and(|block| !block.is_null()) {
            return Err(refused(format!(
                "unsupported-embedded-behavior {role}.{field}"
            )));
        }
    }
    Ok(())
}

/// Freezes exact candidate, dependencies, reviewed expectations, and controlled time.
///
/// Fixture document values may be `candidate`, `selected`, or matching inline JSON.
/// Paths are refused on this authoring surface; legacy file fixtures are frozen by
/// `WorkflowEngine::new` before its execution. Canonical schema validation belongs
/// to the shared project-check API and is not inferred from successful replay.
///
/// # Errors
///
/// Returns a named refusal for substituted/unresolved inputs, unsupported behavior,
/// invalid JSON, missing workflow identity, unsupported observations, or clock overflow.
pub fn prepare_replay(request: ReplayRequest) -> Result<PreparedReplay, ConformanceError> {
    prepare_replay_with_reference(request, None)
}

pub(crate) fn prepare_replay_with_reference(
    request: ReplayRequest,
    reference: Option<&crate::reference_replay::ReferenceReplayAdapters>,
) -> Result<PreparedReplay, ConformanceError> {
    for required in &request.required_observations {
        if !SUPPORTED_OBSERVATIONS.contains(&required.as_str()) {
            return Err(refused(format!("unsupported-observation {required}")));
        }
    }
    for expected in &request.expectations {
        if !expected.pointer.starts_with('/') {
            return Err(refused(format!(
                "invalid-observation-pointer {}",
                expected.pointer
            )));
        }
    }
    let candidate = parse(&request.candidate_bytes, "kernel")?;
    check_supported_workflow(&candidate, "kernel")?;
    let url = candidate["url"]
        .as_str()
        .ok_or_else(|| refused("missing-candidate-url"))?;
    let version = candidate["version"]
        .as_str()
        .ok_or_else(|| refused("missing-candidate-version"))?;
    let original_fixture_value = parse(request.fixture_json.as_bytes(), "fixture")?;
    let mut fixture_value = original_fixture_value.clone();
    if let Some(reference) = reference {
        crate::reference_replay::lower_time_advances(&mut fixture_value, &reference.time_advances)?;
    }
    check_fields(
        &fixture_value,
        &[
            "binding",
            "id",
            "rule",
            "batch",
            "description",
            "documents",
            "inline_documents",
            "initial_case_state",
            "event_sequence",
            "expected_transitions",
            "expected_provenance",
            "expected_errors",
            "contract_outcomes",
            "definition_errors",
            "service_response",
        ],
        "fixture",
    )?;
    let mut fixture: ConformanceFixture = serde_json::from_value(fixture_value.clone())
        .map_err(|error| refused(format!("invalid-fixture: {error}")))?;
    if fixture.event_sequence.is_empty() {
        return Err(refused("no-scenarios: event_sequence is empty"));
    }
    let mut clock = request.clock_start_ms;
    for entry in fixture_value["event_sequence"]
        .as_array()
        .into_iter()
        .flatten()
    {
        check_fields(
            entry,
            &[
                "event",
                "actor",
                "data",
                "delay",
                "task_submission",
                "governance_operation",
                "governance_recipe",
            ],
            "event",
        )?;
        let governed = entry
            .get("governance_operation")
            .or_else(|| entry.get("governance_recipe"));
        if let Some(recipe) = entry.get("governance_recipe") {
            crate::reference_replay::validate_reference_fragment("GovernanceRecipe", recipe)?;
        }
        if governed.is_some() {
            if reference.is_none()
                || ["task_submission", "data"]
                    .iter()
                    .any(|field| entry.get(*field).is_some())
                || (entry.get("governance_operation").is_some()
                    && entry.get("governance_recipe").is_some())
                || governed.is_some_and(Value::is_null)
            {
                return Err(refused(
                    "native governance request requires exclusive selected reference capability",
                ));
            }
        }
        if let Some(submission) = entry.get("task_submission") {
            check_fields(
                submission,
                &[
                    "task_ref",
                    "response",
                    "idempotency_token",
                    "signature_recipe",
                ],
                "task_submission",
            )?;
            if submission.get("signature_recipe").is_some()
                && (reference.is_none() || submission["signature_recipe"].is_null())
            {
                return Err(refused(
                    "native signature recipe requires selected reference capability",
                ));
            }
            if let Some(recipe) = submission.get("signature_recipe") {
                crate::reference_replay::validate_reference_fragment("SignatureRecipe", recipe)?;
                let capability = recipe["signingCapabilityRef"]
                    .as_str()
                    .expect("schema-validated recipe reference");
                if reference.is_none_or(|adapters| {
                    !adapters
                        .fixture_signing_capabilities
                        .iter()
                        .any(|selected| selected.id == capability)
                }) {
                    return Err(refused("unselected signing recipe capability"));
                }
                let response = &submission["response"];
                if response.get("authoredSignatures").is_some()
                    || response.get("sourceResponseRef").is_some()
                    || [
                        "determinationId",
                        "determinationSnapshotSha256",
                        "determinationFiringRecordId",
                        "signatureTaskId",
                    ]
                    .iter()
                    .any(|field| response["data"].get(*field).is_some())
                {
                    return Err(refused(
                        "signature recipe cannot prepopulate native associations or verdicts",
                    ));
                }
            }
            if entry
                .get("actor")
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
            {
                return Err(refused(
                    "missing-submitted-actor: task replay requires an explicit actor",
                ));
            }
        }
        if let Some(delay) = entry.get("delay").and_then(Value::as_str) {
            let ms = wos_core::eval::parse_iso_duration_to_ms(delay)
                .map_err(|error| refused(format!("invalid-delay {error}")))?;
            clock = clock
                .checked_add(ms)
                .ok_or_else(|| refused("clock-overflow"))?;
        }
    }
    for timestamp in [request.clock_start_ms, clock] {
        time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(timestamp) * 1_000_000)
            .map_err(|error| refused(format!("unsupported-clock-range: {error}")))?;
    }
    let mut selected = BTreeMap::from([("kernel".to_string(), candidate.clone())]);
    let mut inputs = vec![document_identity(
        "kernel",
        url,
        Some(version.into()),
        &request.candidate_bytes,
        &candidate,
    )?];
    for (role, block) in [
        ("governance", "governance"),
        ("ai", "aiOversight"),
        ("signatureProfile", "signature"),
        ("advanced", "advanced"),
    ] {
        if candidate.get(block).is_some_and(|value| !value.is_null()) {
            let value = candidate.clone();
            selected.insert(role.into(), value);
        }
    }
    let mut retained_bytes = BTreeMap::from([
        (
            ("kernel".into(), url.into(), Some(version.into())),
            request.candidate_bytes.clone(),
        ),
        (
            ("fixture".into(), fixture.id.clone(), None),
            request.fixture_json.as_bytes().to_vec(),
        ),
    ]);
    let mut identities = BTreeSet::from([(url.to_string(), Some(version.to_string()))]);
    let mut formspec_identities = BTreeSet::new();
    let mut formspec_documents = Vec::new();
    for dependency in request.dependencies {
        let role = &dependency.role;
        if matches!(role.as_str(), "definition" | "mapping") {
            let value = parse(&dependency.bytes, role)?;
            let selected_version = dependency
                .version
                .as_ref()
                .ok_or_else(|| refused("selected Formspec version missing"))?;
            if !formspec_identities.insert((
                role.clone(),
                dependency.url.clone(),
                dependency.version.clone(),
            )) {
                return Err(refused(format!("duplicate-dependency-identity {role}")));
            }
            let mut identity = document_identity(
                role,
                &dependency.url,
                dependency.version.clone(),
                &dependency.bytes,
                &value,
            )?;
            identity.definition = dependency.definition.clone();
            inputs.push(identity);
            retained_bytes.insert(
                (
                    role.clone(),
                    dependency.url.clone(),
                    dependency.version.clone(),
                ),
                dependency.bytes.clone(),
            );
            formspec_documents.push(SelectedDocument {
                kind: if role == "definition" {
                    SelectedDocumentKind::Definition
                } else {
                    SelectedDocumentKind::Mapping
                },
                url: dependency.url,
                version: selected_version.clone(),
                bytes: dependency.bytes,
                definition: dependency.definition,
            });
            continue;
        }
        if dependency.definition.is_some() {
            return Err(refused(format!("unexpected-definition-pairing {role}")));
        }
        if selected.contains_key(role)
            || role == "integration"
            || role == "ai"
            || role == "governance"
            || role == "signatureProfile"
        {
            return Err(refused(format!(
                "substituted-or-duplicate-dependency-role {role}"
            )));
        }
        let value = parse(&dependency.bytes, role)?;
        if dependency.url.is_empty()
            || value
                .get("url")
                .and_then(Value::as_str)
                .is_some_and(|actual| actual != dependency.url)
            || value.get("version").and_then(Value::as_str) != dependency.version.as_deref()
        {
            return Err(refused(format!("dependency-identity-mismatch {role}")));
        }
        if !identities.insert((dependency.url.clone(), dependency.version.clone())) {
            return Err(refused(format!("duplicate-dependency-identity {role}")));
        }
        if let Some(target_version) = role.strip_prefix("kernel@") {
            check_supported_workflow(&value, role)?;
            if dependency.url != url || dependency.version.as_deref() != Some(target_version) {
                return Err(refused(format!(
                    "dependency-workflow-identity-mismatch {role}"
                )));
            }
            for field in [
                "governance",
                "agents",
                "aiOversight",
                "signature",
                "advanced",
                "bindings",
            ] {
                if value.get(field) != candidate.get(field) {
                    return Err(refused(format!(
                        "unsupported-policy-changing-migration {role}.{field}"
                    )));
                }
            }
        } else {
            if !matches!(
                role.as_str(),
                "businessCalendar" | "notifications" | "postureDeclaration"
            ) {
                return Err(refused(format!("unsupported-dependency-role {role}")));
            }
            let target = value.get("targetWorkflow").and_then(|value| {
                value
                    .as_str()
                    .or_else(|| value.get("url").and_then(Value::as_str))
            });
            if role == "postureDeclaration" {
                // Posture declarations are deployment-wide. Their selector is the
                // native signature posturePolicy reference, not targetWorkflow.
                if value.get("$postureDeclaration").and_then(Value::as_str) != Some("1.0") {
                    return Err(refused(
                        "unsupported-dependency-document postureDeclaration",
                    ));
                }
                serde_json::from_value::<wos_runtime::PostureDeclaration>(value.clone())
                    .map_err(|error| refused(format!("invalid-posture-declaration: {error}")))?;
                let policy = &candidate["signature"]["posturePolicy"];
                if policy.get("url").and_then(Value::as_str) != Some(dependency.url.as_str())
                    || policy
                        .get("version")
                        .and_then(Value::as_str)
                        .is_some_and(|version| Some(version) != dependency.version.as_deref())
                {
                    return Err(refused("dependency-posture-policy-mismatch"));
                }
            } else if target != Some(url) {
                return Err(refused(format!("dependency-target-mismatch {role}")));
            }
            if role != "postureDeclaration"
                && value.get("$wosDelivery").and_then(Value::as_str) != Some("1.0")
            {
                return Err(refused(format!("unsupported-dependency-document {role}")));
            }
            if role == "businessCalendar" && value.get("calendar").is_none() {
                return Err(refused("missing-calendar-block"));
            }
            if role == "notifications" && value.get("notifications").is_none() {
                return Err(refused("missing-notifications-block"));
            }
        }
        retained_bytes.insert(
            (
                role.clone(),
                dependency.url.clone(),
                dependency.version.clone(),
            ),
            dependency.bytes.clone(),
        );
        inputs.push(document_identity(
            role,
            &dependency.url,
            dependency.version,
            &dependency.bytes,
            &value,
        )?);
        selected.insert(role.clone(), value);
    }
    if candidate["signature"]
        .get("posturePolicy")
        .is_some_and(|value| !value.is_null())
        && !selected.contains_key("postureDeclaration")
    {
        return Err(refused("unresolved-posture-policy-dependency"));
    }
    for entry in &fixture.event_sequence {
        if entry.event == "$migrate" {
            let target = entry
                .data
                .as_ref()
                .and_then(|data| data.get("targetDefinitionVersion"))
                .and_then(Value::as_str)
                .ok_or_else(|| refused("missing-migration-target"))?;
            if !selected.contains_key(&format!("kernel@{target}")) {
                return Err(refused(format!(
                    "unresolved-workflow-dependency {url}@{target}"
                )));
            }
        }
    }
    // Every authored selector is an assertion over the chosen immutable bytes.
    for (role, selector) in &fixture.documents {
        let actual = selected
            .get(role)
            .ok_or_else(|| refused(format!("unresolved-fixture-document {role}")))?;
        match selector.as_str() {
            "candidate" if role == "kernel" => {}
            "selected" => {}
            "inline"
                if fixture.inline_documents.get(role) == Some(actual)
                    || (matches!(
                        role.as_str(),
                        "ai" | "governance" | "signatureProfile" | "advanced"
                    ) && fixture.inline_documents.get(role) == Some(&candidate)) => {}
            _ => return Err(refused(format!("substituted-fixture-document {role}"))),
        }
    }
    if let Some(role) = fixture
        .inline_documents
        .keys()
        .find(|role| !fixture.documents.contains_key(*role))
    {
        return Err(refused(format!("unselected-inline-document {role}")));
    }
    fixture.documents = selected
        .keys()
        .map(|role| (role.clone(), "inline".into()))
        .collect();
    fixture.inline_documents = selected.into_iter().collect();
    // Construct now to reject unsupported embedded input shapes before execution.
    let selected_formspec = if formspec_documents.is_empty() {
        None
    } else {
        if !fixture.definition_errors.is_empty() || !fixture.contract_outcomes.is_empty() {
            return Err(refused(
                "selected Formspec cannot use canned validation outcomes",
            ));
        }
        Some(
            ImmutableFormspecProcessor::new(&formspec_documents)
                .map_err(|error| refused(error.to_string()))?,
        )
    };
    WorkflowEngine::from_reference(
        &fixture,
        request.clock_start_ms,
        true,
        selected_formspec.as_ref(),
        reference,
        &request.candidate_bytes,
    )
    .map_err(|error| match error {
        ConformanceError::UnsupportedOperation(_) => error,
        other => refused(format!("unsupported-runtime-configuration: {other}")),
    })?;
    inputs.push(document_identity(
        "fixture",
        &fixture.id,
        None,
        request.fixture_json.as_bytes(),
        &original_fixture_value,
    )?);
    if reference.is_some_and(|reference| !reference.time_advances.is_empty()) {
        let lowered =
            serde_json::to_vec(&fixture_value).map_err(|error| refused(error.to_string()))?;
        inputs.push(document_identity(
            "loweredFixture",
            &fixture.id,
            None,
            &lowered,
            &fixture_value,
        )?);
        retained_bytes.insert(("loweredFixture".into(), fixture.id.clone(), None), lowered);
    }
    inputs.sort_by(|left, right| {
        (&left.role, &left.url, &left.version).cmp(&(&right.role, &right.url, &right.version))
    });
    let schema = include_bytes!("../../../schemas/wos-workflow.schema.json");
    let schema_value = parse(schema, "workflow-schema")?;
    let mut runtime_identity = json!({
        "engine":"in-memory WosRuntime", "packageVersion":env!("CARGO_PKG_VERSION"),
        "compiledSource":serde_json::from_str::<Value>(env!("WOS_REPLAY_BUILD_IDENTITY")).map_err(|error| refused(error.to_string()))?,
        "schema":{"id":schema_value["$id"], "rawSha256":digest(schema)},
        "canonicalization":integrity_canonical::CANONICALIZATION_PRIMITIVE,
        "digestAlgorithm":"sha-256", "fixtureFormspecProcessor":selected_formspec.is_none(), "fixtureService":true, "fixtureContractValidator":true,
        "taskPresenter":"no-op", "agentOutputs":"fixture event payloads; native policy stage only; no model invocation, definition validation, human review or governed output commit", "accessControl":FIXTURE_ACCESS_POLICY, "clockStartMs":request.clock_start_ms
    });
    if let Some(reference) = reference {
        runtime_identity["referenceAdapters"] =
            serde_json::to_value(reference).map_err(|error| refused(error.to_string()))?;
    }
    let identity = content_digest(&json!({"inputs":inputs,"runtime":runtime_identity,
        "requiredObservations":request.required_observations,"expectations":request.expectations}))?;
    Ok(PreparedReplay {
        fixture,
        identity,
        inputs,
        clock_start_ms: request.clock_start_ms,
        expectations: request.expectations,
        runtime_identity,
        retained_bytes,
        selected_formspec,
        assert_transitions: reference.is_none()
            || fixture_value.get("expected_transitions").is_some(),
        reference_adapters: reference.cloned().map(std::sync::Arc::new),
    })
}

/// Executes frozen inputs and compares reviewed expectations with native observations.
///
/// Repeated calls construct fresh runtimes with the same controlled time. Native
/// provenance identifiers remain native evidence; the source identity, trace and
/// process observations provide deterministic semantic comparisons.
///
/// # Errors
///
/// Returns an error when frozen runtime configuration cannot be constructed.
/// Creation failures, operation rejections, and readback failures remain in the
/// returned evidence; absent observations are nullable and make the run refuse.
pub fn replay(prepared: &PreparedReplay) -> Result<ReplayResult, ConformanceError> {
    let original_workflow = prepared
        .input_bytes("kernel")
        .ok_or_else(|| refused("selected workflow bytes absent"))?;
    let mut engine = WorkflowEngine::from_reference(
        &prepared.fixture,
        prepared.clock_start_ms,
        true,
        prepared.selected_formspec.as_ref(),
        prepared.reference_adapters.as_deref(),
        original_workflow,
    )?;
    let result = match engine.execute(&prepared.fixture) {
        Ok(result) => result,
        Err(error) => return Ok(creation_failure(prepared, error, engine.initialization())),
    };
    let mut failures = result.failures;
    if prepared.assert_transitions
        && result.transitions.len() != prepared.fixture.expected_transitions.len()
    {
        failures.push(format!(
            "transition count: expected {}, observed {}",
            prepared.fixture.expected_transitions.len(),
            result.transitions.len()
        ));
    }
    let provenance = result.provenance_complete.then_some(result.provenance);
    let observations = json!({"attempts":result.attempts,"finalSnapshot":result.final_snapshot,"provenance":provenance,"initialization":result.initialization});
    let mut differences = Vec::new();
    let mut refusals: Vec<String> = result
        .observation_errors
        .iter()
        .map(|error| format!("missing-observation: {error}"))
        .collect();
    for attempt in &result.attempts {
        refusals.extend(attempt.refusals.iter().cloned());
        for drain in &attempt.drains {
            if let Some(policy) = &drain.policy_evaluation {
                refusals.extend(
                    policy
                        .refusals
                        .iter()
                        .map(|refusal| format!("unsupported-native-ai-policy: {refusal}")),
                );
            }
        }
    }
    for expected in &prepared.expectations {
        let actual = observations.pointer(&expected.pointer).cloned();
        if actual.is_none() {
            refusals.push(format!("missing-observation {}", expected.pointer));
        }
        if actual.as_ref() != Some(&expected.expected) {
            differences.push(ObservationDifference {
                pointer: expected.pointer.clone(),
                expected: expected.expected.clone(),
                actual,
            });
        }
    }
    let passed = failures.is_empty() && refusals.is_empty() && differences.is_empty();
    let mut trace = trace_attempts(
        &prepared.fixture,
        &result.attempts,
        &result.definition_version,
    );
    trace.outcome = if passed { Outcome::Pass } else { Outcome::Fail };
    Ok(ReplayResult {
        passed,
        status: if !refusals.is_empty() {
            "refused"
        } else if passed {
            "pass"
        } else {
            "fail"
        }
        .into(),
        identity: prepared.identity.clone(),
        inputs: prepared.inputs.clone(),
        runtime_identity: prepared.runtime_identity.clone(),
        execution: execution_labels(prepared),
        failures,
        refusals,
        differences,
        attempts: result.attempts,
        final_snapshot: result.final_snapshot,
        provenance,
        trace,
        initialization: result.initialization,
        observation_errors: result.observation_errors,
    })
}

fn creation_failure(
    prepared: &PreparedReplay,
    error: ConformanceError,
    initialization: Value,
) -> ReplayResult {
    let error = error.to_string();
    let mut trace = ConformanceTrace::new(
        &prepared.fixture.id,
        prepared.fixture.inline_documents["kernel"]["version"]
            .as_str()
            .unwrap_or("unknown"),
    );
    trace.outcome = Outcome::Error;
    ReplayResult {
        passed: false,
        status: "refused".into(),
        identity: prepared.identity.clone(),
        inputs: prepared.inputs.clone(),
        runtime_identity: prepared.runtime_identity.clone(),
        execution: execution_labels(prepared),
        failures: vec![error.clone()],
        refusals: vec!["not-run: native process creation failed".into()],
        differences: Vec::new(),
        attempts: Vec::new(),
        final_snapshot: None,
        provenance: None,
        trace,
        initialization,
        observation_errors: Vec::new(),
    }
}

fn execution_labels(prepared: &PreparedReplay) -> Value {
    let selected = prepared.selected_formspec.is_some();
    json!({"runtime":"in-memory WosRuntime","fixtureFormspecProcessor":!selected,
        "formspec":if selected {"FormspecBinding + ImmutableFormspecProcessor (selected native Definition evaluation and Mapping)"} else {"FixtureFormspecProcessor (canned definition validation and response mapping)"},
        "services":"fixture-provided canned response", "contractValidator":"fixture-configured port results; no external provider qualification", "notifications":if prepared.reference_adapters.is_some() {"selected deterministic local fixture transport; native qualified-receipt admission; no hosted/provider delivery qualification"} else {"required notice delivery unsupported; reference admission refuses before process creation (fs-hltb)"}, "taskPresentation":"no-op", "agentOutputs":"fixture event payloads; native policy stage only; no model invocation, definition validation, human review or governed output commit", "accessControl":FIXTURE_ACCESS_POLICY, "hosted":false,"durable":false})
}

fn trace_attempts(
    fixture: &ConformanceFixture,
    attempts: &[ReplayAttempt],
    version: &str,
) -> ConformanceTrace {
    let mut trace = ConformanceTrace::new(&fixture.id, version);
    let mut transition_index = 0;
    for (attempt_index, attempt) in attempts.iter().enumerate() {
        let guards: Vec<_> = attempt
            .drains
            .iter()
            .flat_map(|drain| drain.guard_evaluations.clone())
            .collect();
        let transitions: Vec<_> = attempt
            .drains
            .iter()
            .flat_map(|drain| drain.transitions.iter())
            .collect();
        // Transition expectations describe the complete ordered sequence. Do not
        // assign the next expected transition to an earlier clock/no-op attempt.
        let expected = if transitions.is_empty() {
            fixture
                .expected_transitions
                .get(transition_index)
                .filter(|expected| {
                    expected.event == attempt.event
                        && !attempts[attempt_index + 1..].iter().any(|later| {
                            later.event == expected.event
                                || later
                                    .drains
                                    .iter()
                                    .any(|drain| !drain.transitions.is_empty())
                        })
                })
        } else {
            fixture
                .expected_transitions
                .get(transition_index + transitions.len() - 1)
        };
        let expected_state_after = expected.map(|expected| expected.target.clone());
        // A missing readback remains absent in rich observations. Emit a trace
        // step only when native snapshots or transitions provide actual states.
        let state_before = attempt
            .before
            .as_ref()
            .map(|snapshot| snapshot.configuration.join(","))
            .or_else(|| {
                transitions
                    .first()
                    .map(|transition| transition.from.clone())
            });
        let state_after = attempt
            .after
            .as_ref()
            .map(|snapshot| snapshot.configuration.join(","))
            .or_else(|| transitions.last().map(|transition| transition.to.clone()));
        let (Some(state_before), Some(state_after)) = (state_before, state_after) else {
            continue;
        };
        let delta = expected_state_after.as_ref().and_then(|expected| {
            if *expected == state_after {
                return None;
            }
            guards
                .iter()
                .find(|guard| !guard.result && guard.target_state == *expected)
                .map(|guard| Delta::GuardFalse {
                    guard_id: guard.guard_id.clone(),
                    expression: guard.expression.clone(),
                    inputs: guard.inputs.clone(),
                })
                .or_else(|| {
                    Some(Delta::StateMismatch {
                        expected: expected.clone(),
                        actual: state_after.clone(),
                        cause: attempt.error.clone(),
                    })
                })
        });
        trace.push_step(TraceStep {
            step_index: trace.steps.len() as u32,
            event: Event {
                name: attempt.event.clone(),
                source_actor: attempt.actor.clone(),
                payload: attempt.payload.clone(),
            },
            state_before,
            state_after,
            expected_state_after,
            guards_evaluated: guards,
            policies_applied: crate::policy_applications_for_event(
                attempt.provenance.as_deref().unwrap_or_default(),
                &attempt.event,
            ),
            delta,
        });
        transition_index += transitions.len();
    }
    trace
}

#[doc(inline)]
pub use crate::native_observations::{
    NativeAttemptKind, NativeAttemptSelector, NativeComparison, NativeResolution, NativeSelector,
    NativeSubject, compare_observation, compare_provenance_observation, resolve_observation,
};

#[doc(inline)]
pub use crate::reference_replay::{
    FixtureInvocation, FixtureNoticeTransport, FixtureReceiptSigner, FixtureSigningCapability,
    FixtureTransportFailure, NativeTimeAdvance, PreparedReferenceReplay, ReferenceReplayAdapters,
    ReferenceScope, prepare_reference_replay, replay_reference,
};

#[doc(inline)]
pub use crate::reference_recipes::{DeterminationTarget, GovernanceRecipe, SignatureRecipe};

#[doc(inline)]
pub use crate::reference_comparisons::{
    CohortClassification, CohortMember, CohortMemberResult, EngineeringClaim,
    EngineeringNoticeReview, EngineeringObservation, EngineeringReviewer, EngineeringVerdict,
    MetricDescriptor, MetricMemberDescriptor, MetricReview, NativeReplaySession,
    PairedApprovalRateRequest, PairedApprovalRateResult, PairedReplayMembers,
    RenderedNoticeComparison, RenderedNoticeReviewAssertion, ReviewedNoticeVariant,
    begin_reference_replay, compare_paired_approval_rates, compare_rendered_notice,
    paired_metric_descriptor,
};
