// Rust guideline compliant 2026-04-11

//! Conformance test engine — runtime-backed fixture harness.
//!
//! Reads kernel, governance, and AI integration documents from a conformance
//! fixture, deserializes them into typed models, and delegates lifecycle
//! evaluation to `wos_runtime::WosRuntime`. The engine handles fixture-level
//! concerns: initial case state seeding, event sequence dispatching,
//! delay-based timer advancement, companion-policy configuration, and assertion
//! checking against expected transitions and provenance records.
//!
//! All lifecycle semantics (guard evaluation, state entry/exit, timer
//! management, provenance recording, parallel regions, compound states)
//! are exercised through the runtime boundary.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Deserialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use wos_core::ai_policy::load_native_ai_policy;
use wos_core::eval::GuardEvaluation;
use wos_core::eval::parse_iso_duration_to_ms;
use wos_core::instance::{FormspecTaskContext, PendingEvent};
use wos_core::model::governance::GovernanceDocument;
use wos_core::model::kernel::KernelDocument;
use wos_core::traits::{DocumentResolver, TaskPresenter};
use wos_core::{ProvenanceKind, ProvenanceRecord};
use wos_runtime::{
    BindingRegistry, BusinessCalendarDocument, Clock, CreateProcessRequest, DrainOnceResult,
    IntegrationProfileDocument, MigrationMap, ReferenceCompanionPolicy, RuntimeError, WosRuntime,
    stamp_provenance,
};

use crate::ConformanceError;
use crate::fixture::ConformanceFixture;
use crate::formspec_processor::FixtureFormspecProcessor;
use crate::stubs::{StubService, StubValidator};

const CONFORMANCE_INSTANCE_ID: &str = "default_process_01hw7rm71vfay8vvw14d2pf2db";

/// Document roles that the engine handles explicitly.
///
/// Any key NOT in this list is passed through as a raw companion document.
/// When adding a new first-class role, add it here once — the filter loop in
/// `WorkflowEngine::new` will pick it up automatically.
const RESERVED_DOCUMENT_ROLES: &[&str] = &[
    "kernel",
    "ai",
    "governance",
    "integration",
    "businessCalendar",
    "signatureProfile",
    "postureDeclaration",
];

fn kernel_version_role(role: &str) -> Option<&str> {
    role.strip_prefix("kernel@")
}

fn native_runtime_error(error: RuntimeError) -> ConformanceError {
    match error {
        RuntimeError::UnsupportedAction(reason) => ConformanceError::UnsupportedOperation(reason),
        other => ConformanceError::Engine(other.to_string()),
    }
}

// ── Public types ─────────────────────────────────────────────────

/// Observed state transition during execution.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Transition {
    /// Source state identifier.
    pub from: String,

    /// Target state identifier.
    pub to: String,

    /// Triggering event name.
    pub event: String,
}

// ── Engine ───────────────────────────────────────────────────────

/// Conformance test engine wrapping `wos_runtime::WosRuntime`.
///
/// The engine is a thin harness: it reads documents, seeds initial case state,
/// dispatches events (with simulated delays), runs companion policies on
/// runtime events, and checks runtime transitions and provenance against
/// fixture expectations.
pub struct WorkflowEngine {
    /// Runtime under conformance.
    runtime: WosRuntime,

    /// Mutable simulated runtime clock.
    clock: SharedClock,

    /// Runtime definition URL.
    definition_url: String,

    /// Runtime definition version.
    definition_version: String,

    /// Monotonic fixture event token sequence.
    next_fixture_event_token: u64,

    /// Binding discriminator used for this fixture.
    binding_used: String,

    /// Whether runtime rejections are retained for authoring assertions.
    authoring_replay: bool,

    reference_adapters: Option<crate::reference_replay::ReferenceReplayAdapters>,

    /// Native result of the most recent task submission.
    submission_result: Option<serde_json::Value>,
    migration_result: Option<serde_json::Value>,
    governance_result: Option<serde_json::Value>,
    recipe_resolution: Option<serde_json::Value>,
    pending_observation_errors: Vec<String>,

    /// Successful native drains retained even if a later drain rejects.
    observed_drains: Vec<DrainOnceResult>,

    /// Captured native process creation request, result, and error.
    initialization: serde_json::Value,
}

impl WorkflowEngine {
    /// Initialize the engine from a conformance fixture.
    ///
    /// Reads the kernel document referenced by `fixture.documents["kernel"]`,
    /// and optionally the AI integration document from `fixture.documents["ai"]`.
    ///
    /// # Errors
    ///
    /// Returns `ConformanceError::DocumentNotFound` if a referenced document
    /// cannot be read, or `ConformanceError::Parse` if the JSON is invalid.
    pub fn new(fixture: &ConformanceFixture) -> Result<Self, ConformanceError> {
        let frozen = freeze_fixture(fixture)?;
        Self::from_resolved(&frozen, 0, false)
    }

    pub(crate) fn from_resolved(
        fixture: &ConformanceFixture,
        clock_start_ms: u64,
        authoring_replay: bool,
    ) -> Result<Self, ConformanceError> {
        Self::from_selected(fixture, clock_start_ms, authoring_replay, None)
    }

    pub(crate) fn from_selected(
        fixture: &ConformanceFixture,
        clock_start_ms: u64,
        authoring_replay: bool,
        selected_formspec: Option<&wos_formspec_binding::selected::ImmutableFormspecProcessor>,
    ) -> Result<Self, ConformanceError> {
        Self::from_reference(
            fixture,
            clock_start_ms,
            authoring_replay,
            selected_formspec,
            None,
            &[],
        )
    }

    pub(crate) fn from_reference(
        fixture: &ConformanceFixture,
        clock_start_ms: u64,
        authoring_replay: bool,
        selected_formspec: Option<&wos_formspec_binding::selected::ImmutableFormspecProcessor>,
        reference: Option<&crate::reference_replay::ReferenceReplayAdapters>,
        workflow_bytes: &[u8],
    ) -> Result<Self, ConformanceError> {
        if fixture.documents.values().any(|path| path != "inline") {
            return Err(ConformanceError::Parse(
                "resolved fixture contains a mutable path".into(),
            ));
        }
        let kernel_json = fixture_document_json(fixture, "kernel")?;
        let kernel: KernelDocument = serde_json::from_value(kernel_json.clone())
            .map_err(|e| ConformanceError::Parse(format!("kernel parse error: {e}")))?;
        let definition_url = kernel
            .url
            .clone()
            .unwrap_or_else(|| "urn:wos-conformance:kernel".to_string());
        let definition_version = kernel.version.clone().unwrap_or_default();

        let mut kernels_by_version = HashMap::new();
        kernels_by_version.insert(
            (definition_url.clone(), definition_version.clone()),
            kernel.clone(),
        );
        // Migration fixtures can add extra kernel documents under `kernel@<version>`.
        for key in fixture.documents.keys() {
            if key == "kernel" || kernel_version_role(key).is_none() {
                continue;
            }
            let raw = fixture_document_json(fixture, key)?;
            let parsed: KernelDocument = serde_json::from_value(raw)
                .map_err(|e| ConformanceError::Parse(format!("{key} parse error: {e}")))?;
            let version = parsed.version.clone().unwrap_or_default();
            let url = parsed.url.clone().unwrap_or_else(|| definition_url.clone());
            if kernels_by_version.insert((url, version), parsed).is_some() {
                return Err(ConformanceError::Parse(format!(
                    "duplicate resolved kernel identity: {key}"
                )));
            }
        }

        // One native owner reads the actual selected workflow without injecting
        // agent identities or constructing another authored AI document.
        if fixture.documents.contains_key("ai") {
            let asserted = fixture_document_json(fixture, "ai")?;
            if asserted != kernel_json {
                return Err(ConformanceError::Parse(
                    "substituted legacy AI document: policy must belong to selected kernel".into(),
                ));
            }
        }
        let ai_doc = load_native_ai_policy(&kernel_json)
            .map_err(|error| ConformanceError::Parse(format!("native AI policy: {error}")))?;

        // Load governance content (the `governance` block of a $wosWorkflow
        // document per ADR 0076 D-1). Existing consumers expect a raw JSON
        // value; extract the embedded block so they see the same shape.
        let governance_json = if fixture.documents.contains_key("governance") {
            let raw = fixture_document_json(fixture, "governance")?;
            let block = extract_embedded_block(&raw, "governance").ok_or_else(|| {
                ConformanceError::Parse(
                    "governance document is missing the `governance` embedded block (ADR 0076 D-1)"
                        .to_string(),
                )
            })?;
            Some(block)
        } else {
            None
        };

        // Load integration content (the `bindings` block of a $wosWorkflow
        // document per ADR 0076 D-1; integration content is the OutputBinding
        // array). Existing consumers expect IntegrationProfileDocument shape;
        // for now load the raw block and let downstream consumers handle the
        // shape (the integration-profile standalone type is on track to be
        // absorbed similarly to AI/Governance/BusinessCalendar/Notification).
        let integration_profile = if fixture.documents.contains_key("integration") {
            let ip_json = fixture_document_json(fixture, "integration")?;
            let block = extract_embedded_block(&ip_json, "bindings").ok_or_else(|| {
                ConformanceError::Parse(
                    "integration document is missing the `bindings` embedded block (ADR 0076 D-1)"
                        .to_string(),
                )
            })?;
            let parsed: Result<IntegrationProfileDocument, _> = serde_json::from_value(block);
            Some(parsed.map_err(|e| {
                ConformanceError::Parse(format!("integration profile parse error: {e}"))
            })?)
        } else {
            None
        };

        // Load business calendar content (the `calendar` block of a $wosDelivery
        // sidecar per ADR 0076 D-3). Conformance fixtures express this as an
        // inline $wosDelivery doc whose `calendar` field is the calendar content.
        let business_calendar = if fixture.documents.contains_key("businessCalendar") {
            let cal_json = fixture_document_json(fixture, "businessCalendar")?;
            let block = extract_embedded_block(&cal_json, "calendar").ok_or_else(|| {
                ConformanceError::Parse(
                    "business-calendar document is missing the `calendar` embedded block (ADR 0076 D-3)"
                        .to_string(),
                )
            })?;
            let cal: BusinessCalendarDocument = serde_json::from_value(block).map_err(|e| {
                ConformanceError::Parse(format!("business calendar parse error: {e}"))
            })?;
            Some(cal)
        } else {
            None
        };

        // Load Signature Profile content (the `signature` block of a $wosWorkflow
        // document per ADR 0076 D-1).
        let signature_profile = if fixture.documents.contains_key("signatureProfile") {
            let sig_json = fixture_document_json(fixture, "signatureProfile")?;
            let block = extract_embedded_block(&sig_json, "signature").ok_or_else(|| {
                ConformanceError::Parse(
                    "signature-profile document is missing the `signature` embedded block (ADR 0076 D-1)"
                        .to_string(),
                )
            })?;
            let parsed: Result<wos_runtime::SignatureProfileDocument, _> =
                serde_json::from_value(block);
            Some(parsed.map_err(|e| {
                ConformanceError::Parse(format!("signature profile parse error: {e}"))
            })?)
        } else {
            None
        };

        let posture_declaration = if fixture.documents.contains_key("postureDeclaration") {
            let posture_json = fixture_document_json(fixture, "postureDeclaration")?;
            let parsed: Result<wos_runtime::PostureDeclaration, _> =
                serde_json::from_value(posture_json);
            Some(parsed.map_err(|e| {
                ConformanceError::Parse(format!("posture declaration parse error: {e}"))
            })?)
        } else {
            None
        };

        // Load any remaining companion documents as raw JSON. When a role's
        // canonical form is now an embedded block of a $wosWorkflow / $wosDelivery
        // / $wosOntologyAlignment / $wosTooling envelope per ADR 0076, extract
        // the relevant block so downstream consumers see the legacy interior
        // shape they were written against.
        let mut companion_docs = std::collections::HashMap::new();
        for key in fixture.documents.keys() {
            if RESERVED_DOCUMENT_ROLES.contains(&key.as_str()) || kernel_version_role(key).is_some()
            {
                continue;
            }
            let raw = fixture_document_json(fixture, key)?;
            let block_key = embedded_block_for_role(key);
            let doc_json = match block_key {
                Some(bk) => extract_embedded_block(&raw, bk).unwrap_or(raw),
                None => raw,
            };
            companion_docs.insert(key.clone(), doc_json);
        }

        let stub_service = match &fixture.service_response {
            Some(response) => StubService::with_response(response.clone()),
            None => StubService::null_response(),
        };

        if reference.is_some() && fixture.service_response.is_some() {
            return Err(ConformanceError::Parse(
                "selected adapters cannot mix canned service_response".into(),
            ));
        }
        let (stub_service, selected_integration) =
            crate::reference_invocations::ReplayService::new(
                &kernel,
                reference.map(|adapters| adapters.invocations.as_slice()),
                stub_service,
            )?;
        if integration_profile.is_some() && selected_integration.is_some() {
            return Err(ConformanceError::Parse(
                "ambiguous integration profile".into(),
            ));
        }
        let integration_profile = selected_integration.or(integration_profile);
        let clock = SharedClock::new(clock_start_ms);
        let mut bindings = BindingRegistry::new();
        let binding_used = match fixture.binding.as_deref() {
            Some("formspec") | Some("conformance") | None => {
                if let Some(processor) = selected_formspec {
                    for selected in kernels_by_version.values() {
                        processor
                            .validate_workflow(selected)
                            .map_err(|error| ConformanceError::Parse(error.to_string()))?;
                    }
                    let trusted = reference
                        .map(|reference| {
                            reference
                                .trusted_receipt_signers
                                .iter()
                                .map(|selected| {
                                    use base64::{Engine as _, engine::general_purpose::STANDARD};
                                    let kid =
                                        STANDARD.decode(&selected.kid_base64).map_err(|error| {
                                            ConformanceError::Parse(error.to_string())
                                        })?;
                                    let public =
                                        STANDARD.decode(&selected.public_key_base64).map_err(
                                            |error| ConformanceError::Parse(error.to_string()),
                                        )?;
                                    if selected.suite != "ed25519"
                                        || STANDARD.encode(&kid) != selected.kid_base64
                                        || STANDARD.encode(&public) != selected.public_key_base64
                                    {
                                        return Err(ConformanceError::Parse(
                                            "invalid selected receipt signer".into(),
                                        ));
                                    }
                                    wos_formspec_binding::TrustedReceiptSigner::ed25519(
                                        kid,
                                        public.as_slice().try_into().map_err(|_| {
                                            ConformanceError::Parse(
                                                "invalid Ed25519 receipt key".into(),
                                            )
                                        })?,
                                        selected.method_uri.clone(),
                                    )
                                    .map_err(|error| ConformanceError::Parse(error.to_string()))
                                })
                                .collect::<Result<Vec<_>, ConformanceError>>()
                        })
                        .transpose()?
                        .unwrap_or_default();
                    bindings.register(
                        wos_formspec_binding::FormspecBinding::new_with_trusted_receipt_signers(
                            processor.clone().with_clock(clock.clone()),
                            trusted,
                        ),
                    );
                } else {
                    let processor = if fixture.definition_errors.is_empty() {
                        FixtureFormspecProcessor::new(
                            definition_url.clone(),
                            definition_version.clone(),
                        )
                    } else {
                        FixtureFormspecProcessor::with_definition_errors(
                            definition_url.clone(),
                            definition_version.clone(),
                            fixture.definition_errors.clone(),
                        )
                    };
                    bindings.register(wos_formspec_binding::FormspecBinding::new(processor));
                }
                "formspec".to_string()
            }
            Some(other) => {
                return Err(ConformanceError::Parse(format!(
                    "unknown binding selector '{other}' — expected 'formspec', 'conformance', or omitted"
                )));
            }
        };
        let reference_capabilities = if let Some(reference) = reference.filter(|_| {
            kernel
                .governance
                .as_ref()
                .and_then(|governance| governance.pointer("/dueProcess/execution"))
                .is_some()
        }) {
            if selected_formspec.is_none() || kernels_by_version.len() != 1 {
                return Err(ConformanceError::Parse("reference due process requires selected native Formspec and one immutable workflow version".into()));
            }
            if reference.notice_transport.id.is_empty()
                || reference.notice_transport.mode != "sameInstantQualifiedReceipt"
                || !reference
                    .notice_transport
                    .retained_results_identity
                    .starts_with("sha256:")
                || reference
                    .notice_transport
                    .failure_schedule
                    .iter()
                    .any(|failure| failure.attempt == 0 || failure.outcome != "noEffectFailure")
            {
                return Err(ConformanceError::Parse(
                    "unsupported selected notice transport".into(),
                ));
            }
            let verifier = wos_runtime::SelectedDeterminationSignatureVerifier::new(
                reference.document_signing_keys.clone(),
            )
            .map_err(|error| ConformanceError::Parse(format!("signature capability: {error:?}")))?;
            Some(
                wos_runtime::runtime::governance::ReferenceDueProcessCapabilities {
                    workflow_bytes: workflow_bytes.to_vec(),
                    retained_evidence: reference.retained_evidence.clone(),
                    templates: reference.templates.clone(),
                    notice_transport: Box::new(
                        wos_runtime::runtime::governance::DeterministicNoticeTransport::new(
                            reference
                                .notice_transport
                                .failure_schedule
                                .iter()
                                .map(|failure| failure.attempt),
                        ),
                    ),
                    signature_verifier: Box::new(verifier),
                },
            )
        } else {
            None
        };
        // Every selected kernel version is admitted by its actual implemented owner.
        for selected in kernels_by_version.values() {
            if let Some(capabilities) = reference_capabilities.as_ref() {
                ReferenceCompanionPolicy::validate_reference_due_process_configuration(
                    selected,
                    None,
                    &companion_docs,
                    capabilities,
                )
                .map_err(native_runtime_error)?;
            } else {
                ReferenceCompanionPolicy::validate_configuration(selected, None, &companion_docs)
                    .map_err(native_runtime_error)?;
            }
        }
        let companion_policy = if let Some(capabilities) = reference_capabilities.as_ref() {
            ReferenceCompanionPolicy::new_reference_due_process(
                &kernel,
                ai_doc.clone(),
                governance_json.clone(),
                companion_docs.clone(),
                capabilities,
            )
        } else {
            ReferenceCompanionPolicy::new(
                &kernel,
                ai_doc.clone(),
                governance_json.clone(),
                companion_docs.clone(),
            )
        }
        .map_err(native_runtime_error)?;
        let mut runtime = WosRuntime::new(
            wos_runtime::InMemoryStore::new(),
            FixtureResolver {
                kernels_by_version,
                governance_json: governance_json.clone(),
                sidecars: companion_docs.clone(),
            },
            NoopPresenter,
            FixtureAccessControl {
                allow_delegation: !authoring_replay,
            },
            stub_service,
            StubValidator::from_contract_outcomes(&fixture.contract_outcomes),
            clock.clone(),
            bindings,
        )
        .with_companion_policy(companion_policy);
        if let Some(capabilities) = reference_capabilities {
            runtime
                .configure_reference_due_process(capabilities)
                .map_err(native_runtime_error)?;
        }
        if let Some(profile) = integration_profile {
            runtime = runtime.with_integration_profile(profile);
        }
        if let Some(calendar) = business_calendar {
            runtime = runtime.with_business_calendar(calendar);
        }
        if let Some(profile) = signature_profile {
            runtime = runtime.with_signature_profile("signatureProfile", profile);
        }
        if let Some(posture) = posture_declaration {
            runtime = runtime.with_posture_declaration(posture);
        }

        Ok(Self {
            runtime,
            clock,
            definition_url,
            definition_version,
            next_fixture_event_token: 0,
            binding_used,
            authoring_replay,
            reference_adapters: reference.cloned(),
            submission_result: None,
            migration_result: None,
            governance_result: None,
            recipe_resolution: None,
            pending_observation_errors: Vec::new(),
            observed_drains: Vec::new(),
            initialization: serde_json::Value::Null,
        })
    }

    /// Execute the fixture's event sequence and return conformance results.
    ///
    /// Applies `initial_case_state` first (if present), then advances simulated
    /// time for `delay` entries before processing each event. For events with
    /// agent output data and an AI document, runs deontic evaluation.
    ///
    /// # Errors
    ///
    /// Returns `ConformanceError::Engine` for internal processing failures.
    pub fn execute(
        &mut self,
        fixture: &ConformanceFixture,
    ) -> Result<crate::ConformanceResult, ConformanceError> {
        let creation_request = CreateProcessRequest {
            process_id: CONFORMANCE_INSTANCE_ID.to_string(),
            tenant: None,
            definition_url: self.definition_url.clone(),
            definition_version: self.definition_version.clone(),
            initial_case_state: Some(
                serde_json::to_value(&fixture.initial_case_state)
                    .map_err(|error| ConformanceError::Parse(error.to_string()))?,
            ),
        };
        self.initialization =
            serde_json::json!({"request":creation_request,"result":null,"error":null});
        let created = if let Some(reference) = &self.reference_adapters {
            self.runtime.create_process_with_actor_bindings(
                creation_request,
                reference.human_actor_bindings.clone(),
            )
        } else {
            self.runtime.create_process(creation_request)
        };
        match created {
            Ok(created) => {
                self.initialization["result"] = serde_json::to_value(created)
                    .map_err(|error| ConformanceError::Engine(error.to_string()))?
            }
            Err(error) => {
                self.initialization["error"] = serde_json::Value::String(error.to_string());
                return Err(native_runtime_error(error));
            }
        }

        // Fixture-level provenance is limited to harness concerns, such as
        // invalid fixture delay strings. Runtime behavior must be observed
        // from the runtime provenance log.
        let mut auxiliary_provenance: Vec<ProvenanceRecord> = Vec::new();
        let mut observed_transitions: Vec<Transition> = Vec::new();
        let mut observed_guards: Vec<GuardEvaluation> = Vec::new();

        let mut attempts = Vec::new();
        let mut operation_errors = Vec::new();
        for (event_index, event_entry) in fixture.event_sequence.iter().enumerate() {
            if let Some(delay) = &event_entry.delay {
                self.begin_attempt();
                let before = self.snapshot();
                let ms = match parse_iso_duration_to_ms(delay) {
                    Ok(ms) => ms,
                    Err(raw) => {
                        auxiliary_provenance.push(ProvenanceRecord::invalid_duration(raw, "delay"));
                        0
                    }
                };
                self.clock.advance(ms);
                let operation = self.drain_runtime_until_idle();
                let attempt = self.finish_attempt(
                    event_index,
                    "clock",
                    "$clock",
                    None,
                    None,
                    before,
                    operation,
                )?;
                self.append_attempt(
                    attempt,
                    &mut attempts,
                    &mut operation_errors,
                    &mut observed_transitions,
                    &mut observed_guards,
                );
            }
            self.begin_attempt();
            let before = self.snapshot();
            let (kind, operation) = if event_entry.event == "$migrate" {
                (
                    "migration",
                    self.process_migration_event(event_entry).map(|_| ()),
                )
            } else if event_entry.governance_operation.is_some()
                || event_entry.governance_recipe.is_some()
            {
                (
                    "governanceOperation",
                    self.process_governance_entry(event_entry),
                )
            } else if let Some(submission) = &event_entry.task_submission {
                (
                    "taskSubmission",
                    self.process_task_submission(submission, event_entry.actor.as_deref()),
                )
            } else {
                (
                    "event",
                    self.process_runtime_event(
                        &event_entry.event,
                        event_entry.actor.as_deref(),
                        event_entry.data.as_ref(),
                    ),
                )
            };
            let attempt = self.finish_attempt(
                event_index,
                kind,
                &event_entry.event,
                event_entry.actor.clone(),
                event_entry.data.clone().or_else(|| {
                    event_entry
                        .task_submission
                        .as_ref()
                        .map(|submission| submission.response.clone())
                }),
                before,
                operation,
            )?;
            self.append_attempt(
                attempt,
                &mut attempts,
                &mut operation_errors,
                &mut observed_transitions,
                &mut observed_guards,
            );
        }

        // Auxiliary provenance (invalid fixture delay strings, etc.) is
        // constructed here rather than by the runtime, so it escapes the
        // runtime's stamp_provenance path. Stamp it against the engine's
        // simulated clock so downstream consumers see a uniformly stamped
        // log (no record with an empty timestamp). Records produced before
        // any `delay` entry fires land at the clock's initial moment; that
        // is fine — the contract is "non-empty ISO-8601 timestamp", not
        // "strictly monotonic per record".
        let now_iso = format_timestamp_millis(self.clock.now_ms())?;
        stamp_provenance(&mut auxiliary_provenance, &now_iso);

        // Merge lifecycle provenance with auxiliary provenance.
        let mut observation_errors: Vec<String> = attempts
            .iter()
            .flat_map(|attempt| attempt.observation_errors.iter().cloned())
            .collect();
        let final_snapshot =
            capture_readback(self.snapshot(), "final snapshot", &mut observation_errors);
        let complete_provenance = capture_readback(
            self.load_runtime_provenance_window(0, usize::MAX),
            "final provenance",
            &mut observation_errors,
        );
        let provenance_complete = complete_provenance.is_some();
        let mut provenance = complete_provenance.unwrap_or_else(|| {
            attempts
                .iter()
                .flat_map(|attempt| {
                    attempt
                        .drains
                        .iter()
                        .flat_map(|drain| drain.provenance.clone())
                })
                .collect()
        });
        provenance.extend(auxiliary_provenance);

        // Check assertions.
        let mut failures: Vec<String> = operation_errors
            .iter()
            .filter(|error| {
                !fixture
                    .expected_errors
                    .iter()
                    .any(|expected| error.contains(expected))
            })
            .map(|error| format!("unexpected runtime rejection: {error}"))
            .collect();

        // Transition assertions. `expected.target` matches the canonical
        // workflow-schema vocabulary; `actual.to` belongs to the runtime
        // observation type.
        for (i, expected) in fixture.expected_transitions.iter().enumerate() {
            match observed_transitions.get(i) {
                Some(actual) => {
                    if actual.from != expected.from
                        || actual.to != expected.target
                        || actual.event != expected.event
                    {
                        failures.push(format!(
                            "transition {i}: expected {}->{} on '{}', got {}->{} on '{}'",
                            expected.from,
                            expected.target,
                            expected.event,
                            actual.from,
                            actual.to,
                            actual.event,
                        ));
                    }
                }
                None => {
                    failures.push(format!(
                        "transition {i}: expected {}->{} on '{}', but no transition occurred",
                        expected.from, expected.target, expected.event,
                    ));
                }
            }
        }

        // Report extra (unexpected) transitions.
        if observed_transitions.len() > fixture.expected_transitions.len()
            && !fixture.expected_transitions.is_empty()
        {
            let extra = observed_transitions.len() - fixture.expected_transitions.len();
            failures.push(format!(
                "{extra} unexpected extra transition(s) fired after the expected sequence"
            ));
        }

        // Provenance assertions: each expected record must partially match an actual one.
        for (i, expected_prov) in fixture.expected_provenance.iter().enumerate() {
            let matched = provenance
                .iter()
                .any(|actual| provenance_partial_match(expected_prov, actual));
            if !matched {
                failures.push(format!(
                    "expected_provenance[{i}]: no actual provenance record matched {expected_prov}"
                ));
            }
        }

        // Error assertions: each expected error must appear in failures or provenance.
        for (i, expected_err) in fixture.expected_errors.iter().enumerate() {
            let in_failures = failures.iter().any(|f| f.contains(expected_err.as_str()));
            let in_provenance = provenance.iter().any(|p| {
                p.record_kind == ProvenanceKind::InvalidDuration
                    && p.data
                        .as_ref()
                        .and_then(|d| d.get("rawDuration"))
                        .and_then(|v| v.as_str())
                        .is_some_and(|s| s.contains(expected_err.as_str()))
            });
            let in_operations = operation_errors
                .iter()
                .any(|error| error.contains(expected_err));
            if !in_failures && !in_provenance && !in_operations {
                failures.push(format!(
                    "expected_errors[{i}]: no engine failure or provenance record matched '{expected_err}'"
                ));
            }
        }

        // Unsupported native policy is never a successful skipped stage,
        // even if a fixture expects no lifecycle transition.
        for attempt in &attempts {
            for drain in &attempt.drains {
                if let Some(policy) = &drain.policy_evaluation {
                    failures.extend(
                        policy
                            .refusals
                            .iter()
                            .map(|refusal| format!("unsupported native AI policy: {refusal}")),
                    );
                }
            }
        }

        Ok(crate::ConformanceResult {
            passed: failures.is_empty() && observation_errors.is_empty(),
            failures,
            transitions: observed_transitions,
            provenance,
            guard_evaluations: observed_guards,
            binding_used: Some(self.binding_used.clone()),
            attempts,
            final_snapshot,
            definition_version: self.definition_version.clone(),
            initialization: self.initialization.clone(),
            observation_errors,
            provenance_complete,
        })
    }

    pub(crate) fn initialization(&self) -> serde_json::Value {
        self.initialization.clone()
    }

    fn snapshot(&self) -> Result<crate::studio_api::RuntimeSnapshot, ConformanceError> {
        let process = self
            .runtime
            .load_process(CONFORMANCE_INSTANCE_ID)
            .map_err(native_runtime_error)?;
        let record = self
            .runtime
            .load_reference_record(CONFORMANCE_INSTANCE_ID)
            .map_err(native_runtime_error)?;
        Ok(crate::studio_api::RuntimeSnapshot {
            task_history: native_task_history(&record)?,
            timer_history: native_timer_history(&record)?,
            configuration: process.configuration,
            governance_state: serde_json::to_value(process.governance_state)
                .map_err(|error| ConformanceError::Engine(error.to_string()))?,
            case_state: process.case_state,
            step_results: self
                .runtime
                .load_step_results(CONFORMANCE_INSTANCE_ID)
                .map_err(native_runtime_error)?,
            active_tasks: serde_json::to_value(process.active_tasks)
                .map_err(|error| ConformanceError::Engine(error.to_string()))?,
            timers: serde_json::to_value(process.timers)
                .map_err(|error| ConformanceError::Engine(error.to_string()))?,
            definition_version: process.definition_version,
            provenance_position: usize::try_from(process.provenance_position)
                .map_err(|error| ConformanceError::Engine(format!("provenance cursor: {error}")))?,
        })
    }

    fn begin_attempt(&mut self) {
        self.observed_drains.clear();
        self.submission_result = None;
        self.migration_result = None;
        self.governance_result = None;
        self.recipe_resolution = None;
        self.pending_observation_errors.clear();
    }

    fn finish_attempt(
        &mut self,
        event_index: usize,
        kind: &str,
        event: &str,
        actor: Option<String>,
        payload: Option<serde_json::Value>,
        before: Result<crate::studio_api::RuntimeSnapshot, ConformanceError>,
        operation: Result<(), ConformanceError>,
    ) -> Result<crate::studio_api::ReplayAttempt, ConformanceError> {
        let mut refusals = Vec::new();
        let error = match operation {
            Ok(()) => None,
            Err(error) if self.authoring_replay => {
                if let ConformanceError::UnsupportedOperation(ref reason) = error {
                    refusals.push(reason.clone());
                }
                Some(error.to_string())
            }
            Err(error) => return Err(error),
        };
        let provenance = match &before {
            Ok(before) => {
                self.load_runtime_provenance_window(before.provenance_position, usize::MAX)
            }
            Err(_) => Err(ConformanceError::Engine(
                "provenance window unavailable without initial snapshot".into(),
            )),
        };
        let mut attempt = crate::studio_api::ReplayAttempt {
            event_index,
            kind: kind.into(),
            event: event.into(),
            actor,
            payload,
            clock_ms: self.clock.now_ms(),
            before: None,
            after: None,
            drains: std::mem::take(&mut self.observed_drains),
            submission_result: self.submission_result.take(),
            migration_result: self.migration_result.take(),
            governance_result: self.governance_result.take(),
            recipe_resolution: self.recipe_resolution.take(),
            error,
            refusals,
            provenance: None,
            observation_errors: std::mem::take(&mut self.pending_observation_errors),
        };
        capture_attempt_readbacks(&mut attempt, before, self.snapshot(), provenance);
        Ok(attempt)
    }

    fn append_attempt(
        &self,
        attempt: crate::studio_api::ReplayAttempt,
        attempts: &mut Vec<crate::studio_api::ReplayAttempt>,
        errors: &mut Vec<String>,
        transitions: &mut Vec<Transition>,
        guards: &mut Vec<GuardEvaluation>,
    ) {
        if let Some(error) = &attempt.error {
            errors.push(error.clone());
        }
        if attempt.kind == "migration" && attempt.error.is_some() {
            if let (Some(before), Some(after)) = (&attempt.before, &attempt.after) {
                if before.definition_version != after.definition_version {
                    errors.push(format!(
                        "migration rejection mutated instance version: before={}, after={}",
                        before.definition_version, after.definition_version
                    ));
                }
            }
        }
        if self.authoring_replay {
            if let Some(result) = &attempt.submission_result {
                for kind in ["rejected", "failed"] {
                    if let Some(rejected) = result.get(kind) {
                        errors.push(format!("task submission {kind}: {rejected}"));
                    }
                }
            }
        }
        for drain in &attempt.drains {
            if let Some(policy) = &drain.policy_evaluation {
                errors.extend(
                    policy
                        .errors
                        .iter()
                        .map(|error| format!("native AI policy: {error}")),
                );
            }
            transitions.extend(drain.transitions.iter().map(|transition| Transition {
                from: transition.from.clone(),
                to: transition.to.clone(),
                event: transition.event.clone(),
            }));
            guards.extend(drain.guard_evaluations.iter().cloned());
        }
        attempts.push(attempt);
    }

    fn process_runtime_event(
        &mut self,
        event_name: &str,
        actor_id: Option<&str>,
        data: Option<&serde_json::Value>,
    ) -> Result<(), ConformanceError> {
        self.next_fixture_event_token += 1;
        let fixture_event_token = format!("fixture-event-{}", self.next_fixture_event_token);
        self.runtime
            .enqueue_event(
                CONFORMANCE_INSTANCE_ID,
                PendingEvent {
                    origin: None,
                    event: event_name.to_string(),
                    actor_id: actor_id.map(str::to_string),
                    data: data.cloned(),
                    timestamp: String::new(),
                    idempotency_token: Some(fixture_event_token.clone()),
                },
            )
            .map_err(native_runtime_error)?;

        loop {
            let result = self
                .runtime
                .drain_once(CONFORMANCE_INSTANCE_ID)
                .map_err(native_runtime_error)?;
            let processed_event = result.processed_event.clone();
            let processed_event_token = result.processed_event_token.clone();
            self.observed_drains.push(result);
            if processed_event_token.as_deref() == Some(fixture_event_token.as_str())
                || processed_event.is_none()
            {
                break;
            }
        }

        Ok(())
    }

    fn process_governance_entry(
        &mut self,
        entry: &crate::fixture::EventEntry,
    ) -> Result<(), ConformanceError> {
        let record = self
            .runtime
            .load_reference_record(CONFORMANCE_INSTANCE_ID)
            .map_err(native_runtime_error)?;
        let request = if let Some(recipe) = &entry.governance_recipe {
            let resolved = crate::reference_recipes::resolve_governance(&record, recipe)?;
            self.recipe_resolution =
                Some(serde_json::json!({"recipe":recipe,"resolvedRequest":resolved}));
            resolved
        } else {
            entry
                .governance_operation
                .clone()
                .ok_or_else(|| ConformanceError::Parse("governance request absent".into()))?
        };
        let actor = entry
            .actor
            .as_deref()
            .ok_or_else(|| ConformanceError::Parse("governance actual actor absent".into()))?;
        if actor != request.actor.acting_actor {
            return Err(ConformanceError::Parse(
                "governance outer actor conflicts".into(),
            ));
        }
        let (outcome, observation) = self
            .runtime
            .governance_operation_with_observation(CONFORMANCE_INSTANCE_ID, actor, request)
            .map_err(native_runtime_error)?;
        if let Some(observation) = observation {
            self.observed_drains.push(observation);
        }
        self.governance_result = Some(
            serde_json::to_value(&outcome)
                .map_err(|error| ConformanceError::Parse(error.to_string()))?,
        );
        if let wos_core::model::due_process::GovernanceResult::Refused { code } = outcome {
            return Err(ConformanceError::UnsupportedOperation(format!(
                "native governance refused: {code:?}"
            )));
        }
        self.runtime
            .deliver_pending_notices(CONFORMANCE_INSTANCE_ID)
            .map_err(native_runtime_error)?;
        self.drain_runtime_until_idle()
    }

    fn process_task_submission(
        &mut self,
        submission: &crate::fixture::TaskSubmission,
        submitted_actor: Option<&str>,
    ) -> Result<(), ConformanceError> {
        let instance = self
            .runtime
            .load_process(CONFORMANCE_INSTANCE_ID)
            .map_err(native_runtime_error)?;
        let mut matching = instance
            .active_tasks
            .iter()
            .filter(|task| task.task_ref == submission.task_ref);
        let task = matching.next().ok_or_else(|| {
            ConformanceError::Engine(format!(
                "no active task with task_ref '{}'",
                submission.task_ref
            ))
        })?;
        if matching.next().is_some() {
            return Err(ConformanceError::Engine(
                "active task reference is ambiguous".into(),
            ));
        }
        let task_id = task.task_id.clone();
        let actor_id = submitted_actor
            .map(str::to_string)
            .or_else(|| task.assigned_actor.clone())
            .ok_or_else(|| ConformanceError::Engine("task has no assigned actor".to_string()))?;

        let response = if let Some(recipe) = &submission.signature_recipe {
            let record = self
                .runtime
                .load_reference_record(CONFORMANCE_INSTANCE_ID)
                .map_err(native_runtime_error)?;
            let adapters = self.reference_adapters.as_ref().ok_or_else(|| {
                ConformanceError::Parse("selected signing capability absent".into())
            })?;
            let (response, resolution) = crate::reference_recipes::resolve_signature(
                &record,
                task,
                &actor_id,
                &submission.response,
                recipe,
                adapters,
                self.clock.now_ms(),
            )?;
            self.recipe_resolution = Some(resolution);
            response
        } else {
            submission.response.clone()
        };
        let submission_result = self
            .runtime
            .submit_task_response(
                &task_id,
                response,
                &actor_id,
                submission.idempotency_token.as_deref(),
            )
            .map_err(native_runtime_error)?;

        self.submission_result = Some(
            serde_json::to_value(&submission_result)
                .map_err(|error| ConformanceError::Engine(error.to_string()))?,
        );

        // Submission provenance is read from the native log by finish_attempt;
        // do not manufacture a DrainOnceResult for a non-drain operation.
        self.drain_runtime_until_idle()
    }

    fn process_migration_event(
        &mut self,
        event_entry: &crate::fixture::EventEntry,
    ) -> Result<Vec<ProvenanceRecord>, ConformanceError> {
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct MigrationEventData {
            target_definition_version: String,
            #[serde(default)]
            migration_map: MigrationMap,
            #[serde(default)]
            operator_actor_id: Option<String>,
        }

        let data = event_entry.data.as_ref().ok_or_else(|| {
            ConformanceError::Parse("$migrate event requires a data payload".to_string())
        })?;
        let migration: MigrationEventData =
            serde_json::from_value(data.clone()).map_err(|error| {
                ConformanceError::Parse(format!("migration event parse error: {error}"))
            })?;

        let before_instance = self
            .runtime
            .load_process(CONFORMANCE_INSTANCE_ID)
            .map_err(native_runtime_error)?;
        let previous_definition_version = before_instance.definition_version.clone();
        let provenance_before = self.load_runtime_provenance_window(0, usize::MAX)?.len();
        let operator_actor_id = event_entry
            .actor
            .as_deref()
            .or(migration.operator_actor_id.as_deref());

        let outcome = match self.runtime.migrate(
            CONFORMANCE_INSTANCE_ID,
            migration.target_definition_version.as_str(),
            migration.migration_map.clone(),
            operator_actor_id,
        ) {
            Ok(outcome) => outcome,
            Err(RuntimeError::MigrationRejected(message)) => {
                return Err(ConformanceError::Engine(format!(
                    "migration rejected: {message}"
                )));
            }
            Err(error) => return Err(ConformanceError::Engine(error.to_string())),
        };

        self.migration_result = Some(
            serde_json::to_value(&outcome)
                .map_err(|error| ConformanceError::Engine(error.to_string()))?,
        );
        if outcome.process_id != CONFORMANCE_INSTANCE_ID {
            return Err(ConformanceError::Engine(format!(
                "migration outcome instance mismatch: expected {CONFORMANCE_INSTANCE_ID}, got {}",
                outcome.process_id
            )));
        }
        if outcome.previous_definition_version != previous_definition_version {
            return Err(ConformanceError::Engine(format!(
                "migration outcome previous version mismatch: expected {}, got {}",
                previous_definition_version, outcome.previous_definition_version
            )));
        }
        if outcome.new_definition_version != migration.target_definition_version {
            return Err(ConformanceError::Engine(format!(
                "migration outcome target version mismatch: expected {}, got {}",
                migration.target_definition_version, outcome.new_definition_version
            )));
        }
        let outcome_migration_map = serde_json::to_value(&outcome.migration_map)
            .map_err(|error| ConformanceError::Engine(error.to_string()))?;
        let expected_migration_map = serde_json::to_value(&migration.migration_map)
            .map_err(|error| ConformanceError::Engine(error.to_string()))?;
        if outcome_migration_map != expected_migration_map {
            return Err(ConformanceError::Engine(
                "migration outcome map did not round-trip".to_string(),
            ));
        }

        let after_instance = match self.runtime.load_process(CONFORMANCE_INSTANCE_ID) {
            Ok(process) => process,
            Err(error) => {
                self.pending_observation_errors
                    .push(format!("migration after snapshot readback failed: {error}"));
                return Ok(Vec::new());
            }
        };
        if after_instance.definition_version != migration.target_definition_version {
            return Err(ConformanceError::Engine(format!(
                "migration did not persist target version: expected {}, got {}",
                migration.target_definition_version, after_instance.definition_version
            )));
        }

        let provenance_after = match self.load_runtime_provenance_window(0, usize::MAX) {
            Ok(records) => records,
            Err(error) => {
                self.pending_observation_errors
                    .push(format!("migration provenance readback failed: {error}"));
                return Ok(Vec::new());
            }
        };
        let delta: Vec<ProvenanceRecord> = provenance_after
            .into_iter()
            .skip(provenance_before)
            .collect();
        if delta.len() != 1 {
            return Err(ConformanceError::Engine(format!(
                "migration must append exactly one provenance record, got {}",
                delta.len()
            )));
        }
        if delta[0].record_kind != ProvenanceKind::InstanceMigrated {
            return Err(ConformanceError::Engine(format!(
                "migration provenance kind mismatch: expected instanceMigrated, got {:?}",
                delta[0].record_kind
            )));
        }

        Ok(delta)
    }

    fn drain_runtime_until_idle(&mut self) -> Result<(), ConformanceError> {
        loop {
            let result = self
                .runtime
                .drain_once(CONFORMANCE_INSTANCE_ID)
                .map_err(native_runtime_error)?;
            let idle = result.processed_event.is_none();
            self.observed_drains.push(result);
            if self.reference_adapters.is_some() {
                self.runtime
                    .deliver_pending_notices(CONFORMANCE_INSTANCE_ID)
                    .map_err(native_runtime_error)?;
            }
            if idle {
                return Ok(());
            }
        }
    }

    fn load_runtime_provenance_window(
        &self,
        cursor: usize,
        limit: usize,
    ) -> Result<Vec<ProvenanceRecord>, ConformanceError> {
        self.runtime
            .load_provenance_window(CONFORMANCE_INSTANCE_ID, cursor, limit)
            .map_err(|error| ConformanceError::Engine(error.to_string()))
    }
}

fn capture_readback<T>(
    value: Result<T, ConformanceError>,
    label: &str,
    errors: &mut Vec<String>,
) -> Option<T> {
    match value {
        Ok(value) => Some(value),
        Err(error) => {
            errors.push(format!("{label} readback failed: {error}"));
            None
        }
    }
}

fn capture_attempt_readbacks(
    attempt: &mut crate::studio_api::ReplayAttempt,
    before: Result<crate::studio_api::RuntimeSnapshot, ConformanceError>,
    after: Result<crate::studio_api::RuntimeSnapshot, ConformanceError>,
    provenance: Result<Vec<ProvenanceRecord>, ConformanceError>,
) {
    attempt.before = capture_readback(before, "before snapshot", &mut attempt.observation_errors);
    attempt.after = capture_readback(after, "after snapshot", &mut attempt.observation_errors);
    attempt.provenance = capture_readback(
        provenance,
        "attempt provenance",
        &mut attempt.observation_errors,
    );
}

// ── Module-level helpers ─────────────────────────────────────────

#[derive(Debug, Clone)]
struct SharedClock {
    now_ms: Arc<AtomicU64>,
}

impl SharedClock {
    fn new(now_ms: u64) -> Self {
        Self {
            now_ms: Arc::new(AtomicU64::new(now_ms)),
        }
    }

    fn advance(&self, delta_ms: u64) {
        self.now_ms.fetch_add(delta_ms, Ordering::SeqCst);
    }
}

impl Clock for SharedClock {
    fn now_ms(&self) -> u64 {
        self.now_ms.load(Ordering::SeqCst)
    }
}

/// Format a Unix-millisecond timestamp as RFC 3339, matching the runtime's
/// private `format_timestamp` logic. Kept local so the conformance engine can
/// stamp auxiliary (non-runtime) provenance on its own clock without reaching
/// into a non-public runtime helper.
fn format_timestamp_millis(timestamp_ms: u64) -> Result<String, ConformanceError> {
    let nanos_i128 = i128::from(timestamp_ms) * 1_000_000;
    let timestamp = OffsetDateTime::from_unix_timestamp_nanos(nanos_i128)
        .map_err(|error| ConformanceError::Engine(format!("clock timestamp: {error}")))?;
    timestamp
        .format(&Rfc3339)
        .map_err(|error| ConformanceError::Engine(format!("clock timestamp format: {error}")))
}

fn freeze_fixture(fixture: &ConformanceFixture) -> Result<ConformanceFixture, ConformanceError> {
    let mut frozen = fixture.clone();
    for role in fixture.documents.keys() {
        let value = fixture_document_json(fixture, role)?;
        frozen.documents.insert(role.clone(), "inline".into());
        frozen.inline_documents.insert(role.clone(), value);
    }
    Ok(frozen)
}

fn fixture_document_json(
    fixture: &ConformanceFixture,
    role: &str,
) -> Result<serde_json::Value, ConformanceError> {
    let document_ref = fixture.documents.get(role).ok_or_else(|| {
        ConformanceError::Parse(format!("fixture must declare a '{role}' document"))
    })?;

    let (value, _origin) = if document_ref == "inline" {
        let inline = fixture.inline_documents.get(role).cloned().ok_or_else(|| {
            ConformanceError::Parse(format!(
                "fixture declares inline '{role}' document but omits inline_documents.{role}"
            ))
        })?;
        (inline, format!("inline:{role}"))
    } else {
        let document_text = std::fs::read_to_string(document_ref)
            .map_err(|_| ConformanceError::DocumentNotFound(document_ref.clone()))?;
        let parsed: serde_json::Value = serde_json::from_str(&document_text)
            .map_err(|e| ConformanceError::Parse(format!("{role} parse error: {e}")))?;
        (parsed, document_ref.clone())
    };

    Ok(value)
}

/// Extract an embedded block (e.g., `aiOversight`, `governance`, `signature`,
/// `calendar`, `notifications`) from a parsed `$wosWorkflow` envelope per
/// ADR 0076 D-1/D-3. Returns `None` if the field is absent.
fn extract_embedded_block(
    envelope: &serde_json::Value,
    block_key: &str,
) -> Option<serde_json::Value> {
    envelope
        .as_object()
        .and_then(|obj| obj.get(block_key))
        .cloned()
}

/// Map a fixture document role to its embedded-block key per ADR 0076 D-1/D-3.
/// Returns `None` for roles whose document remains a flat shape (kernel itself)
/// or for roles with no canonical migration mapping.
///
/// Role keys are camelCase — same convention as `$wosWorkflow`, `signingFlow`,
/// and the merged-schema property names. The fixture corpus is canonical
/// camelCase (kebab-case keys retired post-ADR 0076 absorption sweep).
fn embedded_block_for_role(role: &str) -> Option<&'static str> {
    match role {
        // ADR 0076 D-1: $wosWorkflow embedded blocks.
        "advanced" => Some("advanced"),
        "agentConfig" | "agent" => Some("agents"),
        "driftMonitor" | "drift" => Some("agents"),
        "assertionLibrary" => Some("governance"),
        "policyParameters" => Some("governance"),
        "equityConfig" | "equity" => Some("advanced"),
        "advancedGovernance" => Some("advanced"),
        "verificationReport" => Some("advanced"),
        // ADR 0076 D-3: $wosDelivery embedded blocks.
        "correspondenceMetadata" | "correspondence" => Some("correspondence"),
        // ADR 0076 D-5 ($wosTooling) sub-views.
        "extensionRegistry" => Some("extensionRegistry"),
        // Roles with bespoke load paths handled above (kernel / ai /
        // governance / integration / businessCalendar / signatureProfile) are
        // not routed through this helper.
        _ => None,
    }
}

#[derive(Debug, Clone)]
struct FixtureResolver {
    kernels_by_version: HashMap<(String, String), KernelDocument>,
    governance_json: Option<serde_json::Value>,
    sidecars: HashMap<String, serde_json::Value>,
}

#[derive(Debug, thiserror::Error)]
enum FixtureResolverError {
    #[error("governance document unavailable")]
    GovernanceUnavailable,

    #[error("workflow document unavailable: {0}@{1}")]
    KernelUnavailable(String, String),

    #[error("sidecar document unavailable: {0}")]
    SidecarUnavailable(String),
}

impl DocumentResolver for FixtureResolver {
    type Error = FixtureResolverError;

    fn resolve_kernel(&self, url: &str, version: &str) -> Result<KernelDocument, Self::Error> {
        self.kernels_by_version
            .get(&(url.to_string(), version.to_string()))
            .cloned()
            .ok_or_else(|| FixtureResolverError::KernelUnavailable(url.into(), version.into()))
    }

    fn resolve_governance(
        &self,
        url: &str,
        version: &str,
    ) -> Result<GovernanceDocument, Self::Error> {
        let kernel = self.resolve_kernel(url, version)?;
        let governance = kernel.governance.or_else(|| self.governance_json.clone());
        governance
            .as_ref()
            .cloned()
            .ok_or(FixtureResolverError::GovernanceUnavailable)
            .and_then(|governance_json| {
                serde_json::from_value(governance_json)
                    .map_err(|_| FixtureResolverError::GovernanceUnavailable)
            })
    }

    fn resolve_sidecar(
        &self,
        url: &str,
        _anchor_date: Option<&str>,
    ) -> Result<serde_json::Value, Self::Error> {
        self.sidecars
            .get(url)
            .cloned()
            .ok_or_else(|| FixtureResolverError::SidecarUnavailable(url.to_string()))
    }
}

/// Fixture policy: authoring replay allows assigned task actors, without delegation.
/// Legacy fixture execution retains its existing permissive access-control adapter.
#[derive(Debug, Clone, Copy)]
struct FixtureAccessControl {
    allow_delegation: bool,
}

impl wos_core::traits::AccessControl for FixtureAccessControl {
    fn can_transition(&self, _actor_id: &str, _event: &str) -> bool {
        true
    }
    fn can_read(&self, _actor_id: &str, _field_path: &str) -> bool {
        true
    }
    fn can_delegate(
        &self,
        _delegator: &str,
        _delegate: &str,
        _scope: &wos_core::model::governance::DelegationScope,
    ) -> bool {
        self.allow_delegation
    }
}

#[derive(Debug, Clone, Copy)]
struct NoopPresenter;

#[derive(Debug, thiserror::Error)]
#[error("presentation unavailable")]
struct NoopPresenterError;

impl TaskPresenter for NoopPresenter {
    type Error = NoopPresenterError;

    fn present_task(&mut self, _context: &FormspecTaskContext) -> Result<(), Self::Error> {
        Ok(())
    }

    fn dismiss_task(&mut self, _task_id: &str, _reason: &str) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Check whether an expected provenance record partially matches an actual one.
///
/// A partial match requires every field present in `expected` to exist in
/// the actual record with a matching value. Fields absent from `expected`
/// are not checked (wildcard). When both sides are objects, matching is
/// recursive — the actual object may contain extra fields beyond what the
/// fixture asserts.
///
/// Fixtures may use `record_kind` or `record_type` (snake_case aliases);
/// the Rust struct serializes as `recordKind` (camelCase via serde).
/// This function normalizes all three forms to `recordKind` before matching.
pub(crate) fn provenance_partial_match(
    expected: &serde_json::Value,
    actual: &ProvenanceRecord,
) -> bool {
    // Normalize fixture field names to match serde's camelCase output.
    // `record_type` and `record_kind` both map to `recordKind`.
    // This normalization is top-level only — it must NOT recurse into nested objects.
    let normalized = if let serde_json::Value::Object(map) = expected {
        let mut new_map = map.clone();
        // Accept both snake_case aliases and map to the serde camelCase key.
        let kind_val = new_map
            .remove("record_type")
            .or_else(|| new_map.remove("record_kind"));
        if let Some(val) = kind_val {
            new_map.insert("recordKind".to_string(), val);
        }
        serde_json::Value::Object(new_map)
    } else {
        expected.clone()
    };

    // Serialize the actual record so field names and values are comparable.
    let actual_json = match serde_json::to_value(actual) {
        Ok(v) => v,
        Err(_) => return false,
    };

    json_partial_match(&normalized, &actual_json)
}

/// Recursive partial match: every field in `expected` must exist in `actual`
/// with a matching value. Objects are compared field-by-field (actual may have
/// extras). Arrays and scalars use exact equality.
///
/// This is a generic recursive matcher with no domain-specific aliases.
/// Any field normalization (e.g., `record_type` -> `record_kind`) must be
/// performed by the caller before invoking this function.
fn json_partial_match(expected: &serde_json::Value, actual: &serde_json::Value) -> bool {
    match (expected, actual) {
        (serde_json::Value::Object(exp_obj), serde_json::Value::Object(act_obj)) => {
            for (key, exp_val) in exp_obj {
                match act_obj.get(key) {
                    None => return false,
                    Some(av) => {
                        if !json_partial_match(exp_val, av) {
                            return false;
                        }
                    }
                }
            }
            true
        }
        // Non-object values use exact equality.
        _ => expected == actual,
    }
}

#[cfg(test)]
mod observation_failure_tests {
    use super::*;

    #[test]
    fn injected_readback_failure_preserves_native_error_result_and_successful_drain() {
        // This tests observation collection with injected read failures. The
        // integration control separately exercises actual native partial drains.
        let mut attempt = crate::studio_api::ReplayAttempt {
            event_index: 0,
            kind: "taskSubmission".into(),
            event: "submit".into(),
            actor: Some("intruder".into()),
            payload: None,
            clock_ms: 0,
            before: None,
            after: None,
            drains: vec![DrainOnceResult {
                processed_event: Some("earlier-native-event".into()),
                ..Default::default()
            }],
            submission_result: Some(serde_json::json!({"rejected":{"code":"native-rejection"}})),
            migration_result: None,
            governance_result: None,
            recipe_resolution: None,
            error: Some("original native operation error".into()),
            refusals: Vec::new(),
            provenance: None,
            observation_errors: Vec::new(),
        };
        let after = crate::studio_api::RuntimeSnapshot {
            configuration: vec!["observed-state".into()],
            governance_state: serde_json::Value::Null,
            task_history: serde_json::json!([]),
            timer_history: serde_json::json!([]),
            case_state: serde_json::json!({"actual":42}),
            step_results: Vec::new(),
            active_tasks: serde_json::json!([]),
            timers: serde_json::json!([]),
            definition_version: "1.0.0".into(),
            provenance_position: 1,
        };
        capture_attempt_readbacks(
            &mut attempt,
            Err(ConformanceError::Engine(
                "injected before-read failure".into(),
            )),
            Ok(after.clone()),
            Err(ConformanceError::Engine(
                "injected provenance-read failure".into(),
            )),
        );
        assert_eq!(
            attempt.error.as_deref(),
            Some("original native operation error")
        );
        assert_eq!(
            attempt.drains[0].processed_event.as_deref(),
            Some("earlier-native-event")
        );
        assert_eq!(
            attempt.submission_result.as_ref().unwrap()["rejected"]["code"],
            "native-rejection"
        );
        assert!(attempt.before.is_none());
        assert!(attempt.provenance.is_none());
        assert_eq!(attempt.after, Some(after));
        assert_eq!(attempt.observation_errors.len(), 2);
    }
}

/// Associates native task creation/completion with actual saved accepted artifacts.
fn native_task_history(
    record: &wos_runtime::store::RuntimeRecord,
) -> Result<serde_json::Value, ConformanceError> {
    let mut observations = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for creation in record
        .provenance_log
        .iter()
        .filter(|item| item.record_kind == ProvenanceKind::TaskCreated)
    {
        let data = creation
            .data
            .as_ref()
            .ok_or_else(|| ConformanceError::Engine("task creation data missing".into()))?;
        let task_id = data
            .get("taskId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ConformanceError::Engine("task creation id missing".into()))?;
        if !seen.insert(task_id) {
            return Err(ConformanceError::Engine("ambiguous task creation".into()));
        }
        let task_ref = data
            .pointer("/details/taskRef")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ConformanceError::Engine("task reference missing".into()))?;
        let active = record
            .process
            .active_tasks
            .iter()
            .find(|task| task.task_id == task_id);
        let completions: Vec<_> = record
            .provenance_log
            .iter()
            .filter(|item| {
                item.record_kind == ProvenanceKind::TaskCompleted
                    && item
                        .data
                        .as_ref()
                        .and_then(|data| data.get("taskId"))
                        .and_then(serde_json::Value::as_str)
                        == Some(task_id)
            })
            .collect();
        if completions.len() > 1 || (active.is_some() && !completions.is_empty()) {
            return Err(ConformanceError::Engine(
                "inconsistent task lifecycle".into(),
            ));
        }
        let accepted = if let Some(completion) = completions.first() {
            let artifact_id = completion
                .data
                .as_ref()
                .and_then(|data| data.pointer("/details/artifactId"))
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    ConformanceError::Engine(
                        "completed task has no accepted artifact reference".into(),
                    )
                })?;
            let artifact = record
                .artifacts
                .get(artifact_id)
                .filter(|artifact| {
                    artifact.task_id == task_id
                        && artifact.kind == wos_runtime::store::TaskArtifactKind::Accepted
                })
                .ok_or_else(|| {
                    ConformanceError::Engine("completed task accepted artifact missing".into())
                })?;
            serde_json::json!({"artifactId":artifact.artifact_id,"taskId":artifact.task_id,"response":artifact.response,"actorId":artifact.actor_id,"recordedAt":artifact.recorded_at})
        } else {
            serde_json::Value::Null
        };
        observations.push(serde_json::json!({"taskId":task_id,"taskRef":task_ref,
            "status":if completions.is_empty(){active.map(|task|serde_json::to_value(&task.status).unwrap_or(serde_json::Value::Null)).unwrap_or(serde_json::Value::Null)}else{serde_json::json!("completed")},
            "creation":creation,"active":active,"completion":completions.first(),"acceptedArtifact":accepted}));
    }
    Ok(serde_json::Value::Array(observations))
}

/// Retains each actual native timer lifetime in creation order, including firing.
fn native_timer_history(
    record: &wos_runtime::store::RuntimeRecord,
) -> Result<serde_json::Value, ConformanceError> {
    let mut result = Vec::new();
    for (index, creation) in record
        .provenance_log
        .iter()
        .enumerate()
        .filter(|(_, record)| record.record_kind == ProvenanceKind::TimerCreated)
    {
        let id = creation
            .data
            .as_ref()
            .and_then(|data| data.get("timerId"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ConformanceError::Engine("native timer creation missing id".into()))?;
        let next = record
            .provenance_log
            .iter()
            .enumerate()
            .skip(index + 1)
            .find(|(_, record)| {
                record.record_kind == ProvenanceKind::TimerCreated
                    && record
                        .data
                        .as_ref()
                        .and_then(|data| data.get("timerId"))
                        .and_then(serde_json::Value::as_str)
                        == Some(id)
            })
            .map(|(index, _)| index)
            .unwrap_or(record.provenance_log.len());
        let events: Vec<_> = record.provenance_log[index + 1..next]
            .iter()
            .filter(|record| {
                matches!(
                    record.record_kind,
                    ProvenanceKind::TimerFired | ProvenanceKind::TimerCancelled
                ) && record
                    .data
                    .as_ref()
                    .and_then(|data| data.get("timerId"))
                    .and_then(serde_json::Value::as_str)
                    == Some(id)
            })
            .collect();
        let active = if next == record.provenance_log.len() {
            record
                .process
                .timers
                .iter()
                .find(|timer| timer.timer_id == id)
        } else {
            None
        };
        result.push(serde_json::json!({"timerId":id,"creation":creation,"events":events,"active":active,
            "fired":events.iter().any(|event|event.record_kind==ProvenanceKind::TimerFired),"cancelled":events.iter().any(|event|event.record_kind==ProvenanceKind::TimerCancelled)}));
    }
    Ok(serde_json::Value::Array(result))
}
