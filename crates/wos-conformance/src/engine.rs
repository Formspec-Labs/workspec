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
use wos_core::eval::GuardEvaluation;
use wos_core::eval::parse_iso_duration_to_ms;
use wos_core::instance::{FormspecTaskContext, PendingEvent};
use wos_core::model::ai::AIIntegrationDocument;
use wos_core::model::governance::GovernanceDocument;
use wos_core::model::kernel::KernelDocument;
use wos_core::provenance::{ProvenanceKind, ProvenanceRecord};
use wos_core::traits::{DocumentResolver, TaskPresenter};
use wos_runtime::{
    BindingRegistry, BusinessCalendarDocument, Clock, CreateInstanceRequest, DrainOnceResult,
    IntegrationProfileDocument, MigrationMap, ReferenceCompanionPolicy, RuntimeError, WosRuntime,
    stamp_provenance,
};

use crate::ConformanceError;
use crate::fixture::ConformanceFixture;
use crate::formspec_processor::FixtureFormspecProcessor;
use crate::stubs::{StubService, StubValidator};

const CONFORMANCE_INSTANCE_ID: &str = "conformance-instance";

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
];

fn kernel_version_role(role: &str) -> Option<&str> {
    role.strip_prefix("kernel@")
}

// ── Public types ─────────────────────────────────────────────────

/// Observed state transition during execution.
#[derive(Debug, Clone)]
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
        let kernel_json = fixture_document_json(fixture, "kernel")?;
        let kernel: KernelDocument = serde_json::from_value(kernel_json)
            .map_err(|e| ConformanceError::Parse(format!("kernel parse error: {e}")))?;
        let definition_url = kernel
            .url
            .clone()
            .unwrap_or_else(|| "urn:wos-conformance:kernel".to_string());
        let definition_version = kernel.version.clone().unwrap_or_default();

        let mut kernels_by_version = HashMap::new();
        kernels_by_version.insert(definition_version.clone(), kernel.clone());
        // Migration fixtures can add extra kernel documents under `kernel@<version>`.
        for key in fixture.documents.keys() {
            if key == "kernel" || kernel_version_role(key).is_none() {
                continue;
            }
            let raw = fixture_document_json(fixture, key)?;
            let parsed: KernelDocument = serde_json::from_value(raw)
                .map_err(|e| ConformanceError::Parse(format!("{key} parse error: {e}")))?;
            let version = parsed.version.clone().unwrap_or_default();
            kernels_by_version.insert(version, parsed);
        }

        // Load AI integration content (the embedded `aiOversight` block of a
        // $wosWorkflow document, per ADR 0076 D-1). Conformance fixtures express
        // this as an inline `$wosWorkflow` doc whose `aiOversight` field is the
        // AI content; extract that block before parsing into the typed shape.
        let ai_doc = if fixture.documents.contains_key("ai") {
            let ai_json = fixture_document_json(fixture, "ai")?;
            let block = extract_embedded_block(&ai_json, "aiOversight").ok_or_else(|| {
                ConformanceError::Parse(
                    "AI document is missing the `aiOversight` embedded block (ADR 0076 D-1)"
                        .to_string(),
                )
            })?;
            let block = normalize_ai_oversight_block(&ai_json, block);
            let doc: AIIntegrationDocument = serde_json::from_value(block)
                .map_err(|e| ConformanceError::Parse(format!("AI doc parse error: {e}")))?;
            Some(doc)
        } else {
            None
        };

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

        let clock = SharedClock::new(0);
        let mut bindings = BindingRegistry::new();
        let binding_used = match fixture.binding.as_deref() {
            Some("formspec") | Some("conformance") | None => {
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
                "formspec".to_string()
            }
            Some(other) => {
                return Err(ConformanceError::Parse(format!(
                    "unknown binding selector '{other}' — expected 'formspec', 'conformance', or omitted"
                )));
            }
        };
        let mut runtime = WosRuntime::new(
            wos_runtime::InMemoryStore::new(),
            FixtureResolver {
                kernel: kernel.clone(),
                kernels_by_version,
                governance_json: governance_json.clone(),
                sidecars: companion_docs.clone(),
            },
            NoopPresenter,
            wos_core::traits::DefaultRuntime::new(),
            stub_service,
            StubValidator::from_contract_outcomes(&fixture.contract_outcomes),
            clock.clone(),
            bindings,
        )
        .with_companion_policy(ReferenceCompanionPolicy::new(
            ai_doc.clone(),
            governance_json.clone(),
            companion_docs.clone(),
        ));
        if let Some(profile) = integration_profile {
            runtime = runtime.with_integration_profile(profile);
        }
        if let Some(calendar) = business_calendar {
            runtime = runtime.with_business_calendar(calendar);
        }
        if let Some(profile) = signature_profile {
            runtime = runtime.with_signature_profile("signatureProfile", profile);
        }

        Ok(Self {
            runtime,
            clock,
            definition_url,
            definition_version,
            next_fixture_event_token: 0,
            binding_used,
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
        self.runtime
            .create_instance(CreateInstanceRequest {
                instance_id: CONFORMANCE_INSTANCE_ID.to_string(),
                tenant: None,
                definition_url: self.definition_url.clone(),
                definition_version: self.definition_version.clone(),
                initial_case_state: Some(
                    serde_json::to_value(&fixture.initial_case_state)
                        .map_err(|error| ConformanceError::Parse(error.to_string()))?,
                ),
            })
            .map_err(|error| ConformanceError::Engine(error.to_string()))?;

        let mut lifecycle_provenance = self.load_runtime_provenance_window(0, usize::MAX)?;

        // Fixture-level provenance is limited to harness concerns, such as
        // invalid fixture delay strings. Runtime behavior must be observed
        // from the runtime provenance log.
        let mut auxiliary_provenance: Vec<ProvenanceRecord> = Vec::new();
        let mut observed_transitions: Vec<Transition> = Vec::new();
        let mut observed_guards: Vec<GuardEvaluation> = Vec::new();

        for event_entry in &fixture.event_sequence {
            // Advance simulated clock if the fixture declares a delay.
            if let Some(delay) = &event_entry.delay {
                let ms = match parse_iso_duration_to_ms(delay) {
                    Ok(ms) => ms,
                    Err(raw) => {
                        auxiliary_provenance.push(ProvenanceRecord::invalid_duration(raw, "delay"));
                        0
                    }
                };
                self.clock.advance(ms);
                let due_results = self.drain_runtime_until_idle()?;
                for result in due_results {
                    append_runtime_result(
                        result,
                        &mut observed_transitions,
                        &mut lifecycle_provenance,
                        &mut observed_guards,
                    );
                }
            }

            if event_entry.event == "$migrate" {
                let results = self.process_migration_event(event_entry)?;
                lifecycle_provenance.extend(results);
            } else if let Some(submission) = &event_entry.task_submission {
                let results = self.process_task_submission(submission)?;
                for result in results {
                    append_runtime_result(
                        result,
                        &mut observed_transitions,
                        &mut lifecycle_provenance,
                        &mut observed_guards,
                    );
                }
            } else {
                let results = self.process_runtime_event(
                    &event_entry.event,
                    event_entry.actor.as_deref(),
                    event_entry.data.as_ref(),
                )?;
                for result in results {
                    append_runtime_result(
                        result,
                        &mut observed_transitions,
                        &mut lifecycle_provenance,
                        &mut observed_guards,
                    );
                }
            }
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
        let mut provenance = lifecycle_provenance;
        provenance.extend(auxiliary_provenance);

        // Check assertions.
        let mut failures = Vec::new();

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
            if !in_failures && !in_provenance {
                failures.push(format!(
                    "expected_errors[{i}]: no engine failure or provenance record matched '{expected_err}'"
                ));
            }
        }

        Ok(crate::ConformanceResult {
            passed: failures.is_empty(),
            failures,
            transitions: observed_transitions,
            provenance,
            guard_evaluations: observed_guards,
            binding_used: Some(self.binding_used.clone()),
        })
    }

    fn process_runtime_event(
        &mut self,
        event_name: &str,
        actor_id: Option<&str>,
        data: Option<&serde_json::Value>,
    ) -> Result<Vec<DrainOnceResult>, ConformanceError> {
        self.next_fixture_event_token += 1;
        let fixture_event_token = format!("fixture-event-{}", self.next_fixture_event_token);
        self.runtime
            .enqueue_event(
                CONFORMANCE_INSTANCE_ID,
                PendingEvent {
                    event: event_name.to_string(),
                    actor_id: actor_id.map(str::to_string),
                    data: data.cloned(),
                    timestamp: String::new(),
                    idempotency_token: Some(fixture_event_token.clone()),
                },
            )
            .map_err(|error| ConformanceError::Engine(error.to_string()))?;

        let mut results = Vec::new();
        loop {
            let result = self
                .runtime
                .drain_once(CONFORMANCE_INSTANCE_ID)
                .map_err(|error| ConformanceError::Engine(error.to_string()))?;
            let processed_event = result.processed_event.clone();
            let processed_event_token = result.processed_event_token.clone();
            results.push(result);
            if processed_event_token.as_deref() == Some(fixture_event_token.as_str())
                || processed_event.is_none()
            {
                break;
            }
        }

        Ok(results)
    }

    fn process_task_submission(
        &mut self,
        submission: &crate::fixture::TaskSubmission,
    ) -> Result<Vec<DrainOnceResult>, ConformanceError> {
        let provenance_before = self.load_runtime_provenance_window(0, usize::MAX)?.len();

        let instance = self
            .runtime
            .load_instance(CONFORMANCE_INSTANCE_ID)
            .map_err(|error| ConformanceError::Engine(error.to_string()))?;
        let task = instance
            .active_tasks
            .iter()
            .find(|t| t.task_ref == submission.task_ref)
            .ok_or_else(|| {
                ConformanceError::Engine(format!(
                    "no active task with task_ref '{}'",
                    submission.task_ref
                ))
            })?;
        let task_id = task.task_id.clone();
        let actor_id = task
            .assigned_actor
            .clone()
            .ok_or_else(|| ConformanceError::Engine("task has no assigned actor".to_string()))?;

        let _result = self
            .runtime
            .submit_task_response(
                &task_id,
                submission.response.clone(),
                &actor_id,
                submission.idempotency_token.as_deref(),
            )
            .map_err(|error| ConformanceError::Engine(error.to_string()))?;

        // submit_task_response writes provenance directly to the store,
        // not through the drain mechanism. Capture the submission provenance
        // delta before draining any triggered events.
        let provenance_after_submit = self.load_runtime_provenance_window(0, usize::MAX)?;
        let submission_provenance: Vec<ProvenanceRecord> = provenance_after_submit
            .into_iter()
            .skip(provenance_before)
            .collect();

        let mut results = Vec::new();

        if !submission_provenance.is_empty() {
            results.push(DrainOnceResult {
                provenance: submission_provenance,
                ..DrainOnceResult::default()
            });
        }

        let mut drain_results = self.drain_runtime_until_idle()?;
        results.append(&mut drain_results);

        Ok(results)
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
            .load_instance(CONFORMANCE_INSTANCE_ID)
            .map_err(|error| ConformanceError::Engine(error.to_string()))?;
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
                let after_instance = self
                    .runtime
                    .load_instance(CONFORMANCE_INSTANCE_ID)
                    .map_err(|error| ConformanceError::Engine(error.to_string()))?;
                if after_instance.definition_version != previous_definition_version {
                    return Err(ConformanceError::Engine(format!(
                        "migration rejection mutated instance version: before={}, after={}",
                        previous_definition_version, after_instance.definition_version
                    )));
                }
                return Err(ConformanceError::Engine(format!(
                    "migration rejected: {message}"
                )));
            }
            Err(error) => return Err(ConformanceError::Engine(error.to_string())),
        };

        if outcome.instance_id != CONFORMANCE_INSTANCE_ID {
            return Err(ConformanceError::Engine(format!(
                "migration outcome instance mismatch: expected {CONFORMANCE_INSTANCE_ID}, got {}",
                outcome.instance_id
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

        let after_instance = self
            .runtime
            .load_instance(CONFORMANCE_INSTANCE_ID)
            .map_err(|error| ConformanceError::Engine(error.to_string()))?;
        if after_instance.definition_version != migration.target_definition_version {
            return Err(ConformanceError::Engine(format!(
                "migration did not persist target version: expected {}, got {}",
                migration.target_definition_version, after_instance.definition_version
            )));
        }

        let provenance_after = self.load_runtime_provenance_window(0, usize::MAX)?;
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

    fn drain_runtime_until_idle(&mut self) -> Result<Vec<DrainOnceResult>, ConformanceError> {
        self.runtime
            .drain_until_idle(CONFORMANCE_INSTANCE_ID)
            .map_err(|error| ConformanceError::Engine(error.to_string()))
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

fn normalize_ai_oversight_block(
    envelope: &serde_json::Value,
    mut block: serde_json::Value,
) -> serde_json::Value {
    let Some(obj) = block.as_object_mut() else {
        return block;
    };
    if !obj.contains_key("targetWorkflow") {
        if let Some(url) = envelope.get("url").and_then(serde_json::Value::as_str) {
            obj.insert(
                "targetWorkflow".to_string(),
                serde_json::Value::String(url.to_string()),
            );
        }
    }
    if !obj.contains_key("agents") {
        if let Some(agents) = obj.get("x-transportAgentDetails").cloned() {
            obj.insert("agents".to_string(), agents);
        }
    }
    block
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
    kernel: KernelDocument,
    kernels_by_version: HashMap<String, KernelDocument>,
    governance_json: Option<serde_json::Value>,
    sidecars: HashMap<String, serde_json::Value>,
}

#[derive(Debug, thiserror::Error)]
enum FixtureResolverError {
    #[error("governance document unavailable")]
    GovernanceUnavailable,

    #[error("sidecar document unavailable: {0}")]
    SidecarUnavailable(String),
}

impl DocumentResolver for FixtureResolver {
    type Error = FixtureResolverError;

    fn resolve_kernel(&self, _url: &str, version: &str) -> Result<KernelDocument, Self::Error> {
        Ok(self
            .kernels_by_version
            .get(version)
            .cloned()
            .unwrap_or_else(|| self.kernel.clone()))
    }

    fn resolve_governance(
        &self,
        _url: &str,
        _version: &str,
    ) -> Result<GovernanceDocument, Self::Error> {
        self.governance_json
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

fn append_runtime_result(
    result: DrainOnceResult,
    transitions: &mut Vec<Transition>,
    provenance: &mut Vec<ProvenanceRecord>,
    guard_evaluations: &mut Vec<GuardEvaluation>,
) {
    transitions.extend(result.transitions.into_iter().map(|transition| Transition {
        from: transition.from,
        to: transition.to,
        event: transition.event,
    }));
    provenance.extend(result.provenance);
    guard_evaluations.extend(result.guard_evaluations);
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
fn provenance_partial_match(expected: &serde_json::Value, actual: &ProvenanceRecord) -> bool {
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
