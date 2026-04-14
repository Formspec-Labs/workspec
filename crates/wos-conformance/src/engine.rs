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
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use wos_core::eval::parse_iso_duration_to_ms;
use wos_core::instance::{FormspecTaskContext, PendingEvent, ValidationOutcome};
use wos_core::model::ai::AIIntegrationDocument;
use wos_core::model::governance::GovernanceDocument;
use wos_core::model::kernel::KernelDocument;
use wos_core::provenance::{ProvenanceKind, ProvenanceRecord};
use wos_core::traits::{DocumentResolver, TaskPresenter};
use wos_runtime::{
    BindingError, BindingRegistry, CaseMutationBundle, Clock, ContractBindingAdapter,
    CreateInstanceRequest, DrainOnceResult, PreparedTask, ReferenceCompanionPolicy, WosRuntime,
};

use crate::fixture::ConformanceFixture;
use crate::stubs::{StubService, StubValidator};
use crate::ConformanceError;

const CONFORMANCE_INSTANCE_ID: &str = "conformance-instance";

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
        let kernel_path = fixture.documents.get("kernel").ok_or_else(|| {
            ConformanceError::Parse("fixture must declare a 'kernel' document".into())
        })?;

        let kernel_json = std::fs::read_to_string(kernel_path)
            .map_err(|_| ConformanceError::DocumentNotFound(kernel_path.clone()))?;

        let kernel: KernelDocument = serde_json::from_str(&kernel_json)
            .map_err(|e| ConformanceError::Parse(format!("kernel parse error: {e}")))?;
        let definition_url = kernel
            .url
            .clone()
            .unwrap_or_else(|| "urn:wos-conformance:kernel".to_string());
        let definition_version = kernel.version.clone().unwrap_or_default();

        // Load AI integration document if present.
        let ai_doc = if let Some(ai_path) = fixture.documents.get("ai") {
            let ai_json = std::fs::read_to_string(ai_path)
                .map_err(|_| ConformanceError::DocumentNotFound(ai_path.clone()))?;
            let doc: AIIntegrationDocument = serde_json::from_str(&ai_json)
                .map_err(|e| ConformanceError::Parse(format!("AI doc parse error: {e}")))?;
            Some(doc)
        } else {
            None
        };

        // Load governance document if present.
        let governance_json = if let Some(gov_path) = fixture.documents.get("governance") {
            let gov_str = std::fs::read_to_string(gov_path)
                .map_err(|_| ConformanceError::DocumentNotFound(gov_path.clone()))?;
            Some(
                serde_json::from_str(&gov_str)
                    .map_err(|e| ConformanceError::Parse(format!("governance parse error: {e}")))?,
            )
        } else {
            None
        };

        // Load any remaining companion documents as raw JSON.
        let mut companion_docs = std::collections::HashMap::new();
        for (key, path) in &fixture.documents {
            if key == "kernel" || key == "ai" || key == "governance" {
                continue;
            }
            let doc_str = std::fs::read_to_string(path)
                .map_err(|_| ConformanceError::DocumentNotFound(path.clone()))?;
            let doc_json: serde_json::Value = serde_json::from_str(&doc_str)
                .map_err(|e| ConformanceError::Parse(format!("{key} parse error: {e}")))?;
            companion_docs.insert(key.clone(), doc_json);
        }

        let clock = SharedClock::new(0);
        let mut bindings = BindingRegistry::new();
        bindings.register(ConformanceBinding);
        let runtime = WosRuntime::new(
            wos_runtime::InMemoryStore::new(),
            FixtureResolver {
                kernel: kernel.clone(),
                governance_json: governance_json.clone(),
                sidecars: companion_docs.clone(),
            },
            NoopPresenter,
            wos_core::traits::DefaultRuntime::new(),
            StubService::null_response(),
            StubValidator::from_contract_outcomes(&fixture.contract_outcomes),
            clock.clone(),
            bindings,
        )
        .with_companion_policy(ReferenceCompanionPolicy::new(
            ai_doc.clone(),
            governance_json.clone(),
            companion_docs.clone(),
        ));

        Ok(Self {
            runtime,
            clock,
            definition_url,
            definition_version,
            next_fixture_event_token: 0,
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
                    );
                }
            }

            let results = self.process_runtime_event(
                &event_entry.event,
                event_entry.actor.as_deref(),
                event_entry.data.as_ref(),
            )?;
            for result in results {
                append_runtime_result(result, &mut observed_transitions, &mut lifecycle_provenance);
            }
        }

        // Merge lifecycle provenance with auxiliary provenance.
        let mut provenance = lifecycle_provenance;
        provenance.extend(auxiliary_provenance);

        // Check assertions.
        let mut failures = Vec::new();

        // Transition assertions.
        for (i, expected) in fixture.expected_transitions.iter().enumerate() {
            match observed_transitions.get(i) {
                Some(actual) => {
                    if actual.from != expected.from
                        || actual.to != expected.to
                        || actual.event != expected.event
                    {
                        failures.push(format!(
                            "transition {i}: expected {}->{} on '{}', got {}->{} on '{}'",
                            expected.from,
                            expected.to,
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
                        expected.from, expected.to, expected.event,
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

#[derive(Debug, Clone)]
struct FixtureResolver {
    kernel: KernelDocument,
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

    fn resolve_kernel(&self, _url: &str, _version: &str) -> Result<KernelDocument, Self::Error> {
        Ok(self.kernel.clone())
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

#[derive(Debug, Clone, Copy)]
struct ConformanceBinding;

/// Conformance test binding adapter for the "formspec" contract type.
///
/// **Intentionally permissive:** always returns valid submissions and no case
/// mutations. This lets conformance fixtures exercise the runtime task lifecycle
/// (create → present → submit → complete) without requiring a real Formspec
/// engine. When a conformance test needs to assert on validation outcomes, it
/// uses the fixture-level `contract_outcomes` map and the `StubValidator` instead.
///
/// `compute_case_mutation` returns `None` because case-state mutations under
/// task submission are tested via the runtime's `submit_task_response` integration
/// tests in `wos-runtime`, not through conformance fixtures.
impl ContractBindingAdapter for ConformanceBinding {
    fn binding(&self) -> &'static str {
        "formspec"
    }

    fn prepare_task(
        &self,
        _task: &wos_core::instance::ActiveTask,
        _case_state: &serde_json::Value,
    ) -> Result<PreparedTask, BindingError> {
        Ok(PreparedTask::default())
    }

    fn validate_submission(
        &self,
        _task: &wos_core::instance::ActiveTask,
        _response: &serde_json::Value,
    ) -> Result<wos_runtime::SubmissionValidation, BindingError> {
        Ok(wos_runtime::SubmissionValidation {
            validation_outcome: ValidationOutcome {
                envelope_valid: true,
                pin_match: true,
                definition_valid: true,
                errors: Vec::new(),
                validation_results: None,
            },
        })
    }

    fn compute_case_mutation(
        &self,
        _task: &wos_core::instance::ActiveTask,
        _response: &serde_json::Value,
    ) -> Result<Option<CaseMutationBundle>, BindingError> {
        Ok(None)
    }
}

fn append_runtime_result(
    result: DrainOnceResult,
    transitions: &mut Vec<Transition>,
    provenance: &mut Vec<ProvenanceRecord>,
) {
    transitions.extend(result.transitions.into_iter().map(|transition| Transition {
        from: transition.from,
        to: transition.to,
        event: transition.event,
    }));
    provenance.extend(result.provenance);
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
