// Rust guideline compliant 2026-04-10

//! Conformance test engine — thin harness over `wos_core::Evaluator`.
//!
//! Reads a kernel document from a conformance fixture, deserializes it into
//! a typed `KernelDocument`, and delegates lifecycle evaluation to
//! `wos_core::Evaluator`. The engine handles fixture-level concerns:
//! initial case state seeding, event sequence dispatching, delay-based
//! timer advancement, and assertion checking against expected transitions
//! and provenance records.
//!
//! All lifecycle semantics (guard evaluation, state entry/exit, timer
//! management, provenance recording, parallel regions, compound states)
//! are implemented in `wos_core::eval`.

use wos_core::eval::{Evaluator, parse_iso_duration_to_ms};
use wos_core::model::kernel::KernelDocument;
use wos_core::provenance::{ProvenanceKind, ProvenanceRecord};

use crate::fixture::ConformanceFixture;
use crate::ConformanceError;

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

/// Conformance test engine wrapping `wos_core::Evaluator`.
///
/// The engine is a thin harness: it reads the kernel document, seeds initial
/// case state, dispatches events (with simulated delays), and then checks
/// the evaluator's transitions and provenance against fixture expectations.
pub struct WorkflowEngine {
    /// The typed lifecycle evaluator from wos-core.
    evaluator: Evaluator,
}

impl WorkflowEngine {
    /// Initialize the engine from a conformance fixture.
    ///
    /// Reads the kernel document referenced by `fixture.documents["kernel"]`,
    /// deserializes it into a typed `KernelDocument`, and initializes the
    /// `wos_core::Evaluator` (which enters the initial state and executes
    /// its `onEntry` actions).
    ///
    /// # Errors
    ///
    /// Returns `ConformanceError::DocumentNotFound` if the kernel document path
    /// cannot be read, or `ConformanceError::Parse` if the JSON is invalid.
    pub fn new(fixture: &ConformanceFixture) -> Result<Self, ConformanceError> {
        let kernel_path = fixture
            .documents
            .get("kernel")
            .ok_or_else(|| {
                ConformanceError::Parse("fixture must declare a 'kernel' document".into())
            })?;

        let kernel_json = std::fs::read_to_string(kernel_path)
            .map_err(|_| ConformanceError::DocumentNotFound(kernel_path.clone()))?;

        let kernel: KernelDocument = serde_json::from_str(&kernel_json)
            .map_err(|e| ConformanceError::Parse(e.to_string()))?;

        let evaluator = Evaluator::new(kernel)
            .map_err(|e| ConformanceError::Engine(e.to_string()))?;

        Ok(Self { evaluator })
    }

    /// Execute the fixture's event sequence and return conformance results.
    ///
    /// Applies `initial_case_state` first (if present), then advances simulated
    /// time for `delay` entries before processing each event.
    ///
    /// # Errors
    ///
    /// Returns `ConformanceError::Engine` for internal processing failures.
    pub fn execute(
        &mut self,
        fixture: &ConformanceFixture,
    ) -> Result<crate::ConformanceResult, ConformanceError> {
        // Pre-seed case state from fixture declarations.
        for (key, value) in &fixture.initial_case_state {
            self.evaluator
                .case_state_mut()
                .insert(key.clone(), value.clone());
        }

        for event_entry in &fixture.event_sequence {
            // Advance simulated clock if the fixture declares a delay.
            if let Some(delay) = &event_entry.delay {
                let ms = match parse_iso_duration_to_ms(delay) {
                    Ok(ms) => ms,
                    Err(raw) => {
                        // Unknown duration format — record in provenance and treat as 0 ms.
                        self.evaluator.record_provenance(
                            ProvenanceRecord::invalid_duration(raw, "delay"),
                        );
                        0
                    }
                };
                self.evaluator
                    .advance_time(ms, event_entry.actor.as_deref())
                    .map_err(|e| ConformanceError::Engine(e.to_string()))?;
            }

            self.evaluator
                .process_event(
                    &event_entry.event,
                    event_entry.actor.as_deref(),
                    event_entry.data.as_ref(),
                )
                .map_err(|e| ConformanceError::Engine(e.to_string()))?;
        }

        // Collect results from the evaluator.
        let transitions: Vec<Transition> = self
            .evaluator
            .transitions()
            .iter()
            .map(|t| Transition {
                from: t.from.clone(),
                to: t.to.clone(),
                event: t.event.clone(),
            })
            .collect();

        let provenance: Vec<ProvenanceRecord> = self
            .evaluator
            .provenance()
            .records()
            .to_vec();

        // Check assertions.
        let mut failures = Vec::new();

        // Transition assertions.
        for (i, expected) in fixture.expected_transitions.iter().enumerate() {
            match transitions.get(i) {
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
        if transitions.len() > fixture.expected_transitions.len()
            && !fixture.expected_transitions.is_empty()
        {
            let extra = transitions.len() - fixture.expected_transitions.len();
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
            transitions,
            provenance,
        })
    }
}

// ── Module-level helpers ─────────────────────────────────────────

/// Check whether an expected provenance record (a JSON object with arbitrary fields)
/// is a partial match for an actual `ProvenanceRecord`.
///
/// A partial match requires every field present in `expected` to equal the
/// corresponding field in the serialized actual record. Fields absent from
/// `expected` are not checked (wildcard). The `record_type` field in `expected`
/// matches the camelCase serialization of `actual.record_kind`.
fn provenance_partial_match(expected: &serde_json::Value, actual: &ProvenanceRecord) -> bool {
    // Serialize the actual record so field names and values are comparable.
    let actual_json = match serde_json::to_value(actual) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let expected_obj = match expected.as_object() {
        Some(o) => o,
        None => return false,
    };

    for (key, expected_val) in expected_obj {
        // Support both "record_kind" (Rust field name) and "record_type" (legacy alias)
        // so fixture authors can use either.
        let actual_val = if key == "record_type" {
            actual_json.get("record_kind")
        } else {
            actual_json.get(key)
        };

        match actual_val {
            None => return false,
            Some(av) => {
                if av != expected_val {
                    return false;
                }
            }
        }
    }

    true
}
