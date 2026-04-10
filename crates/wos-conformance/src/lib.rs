// Rust guideline compliant 2026-02-21

//! Dynamic conformance test runner for WOS workflows.
//!
//! Executes event sequences against WOS kernel documents using the
//! deterministic evaluation algorithm from the Lifecycle Detail Companion,
//! and asserts on state transitions, provenance records, timer behavior,
//! compensation ordering, and deontic enforcement.
//!
//! This crate covers Tier 3 of the WOS Verification Matrix (98 rules).
//! See `LINT-MATRIX.md` for the complete constraint catalog.

mod engine;
mod fixture;
mod provenance;

pub use engine::WorkflowEngine;
pub use fixture::{ConformanceFixture, EventEntry, ExpectedTransition};
pub use provenance::{ProvenanceKind, ProvenanceRecord};

/// Run a conformance fixture and return the results.
///
/// Document paths in the fixture are resolved relative to `base_dir`.
/// Pass the directory containing the fixture file so that relative paths
/// like `../../../../fixtures/kernel/example.json` resolve correctly.
///
/// # Examples
///
/// ```no_run
/// use wos_conformance::run_fixture;
///
/// let fixture_json = std::fs::read_to_string("fixture.json").unwrap();
/// let result = run_fixture(&fixture_json, ".").unwrap();
/// assert!(result.passed, "fixture failed: {:?}", result.failures);
/// ```
///
/// # Errors
///
/// Returns `ConformanceError::Parse` if the fixture or documents cannot
/// be parsed, or `ConformanceError::Engine` if the workflow engine
/// encounters an internal error.
pub fn run_fixture(fixture_json: &str, base_dir: &str) -> Result<ConformanceResult, ConformanceError> {
    let mut fixture: ConformanceFixture = serde_json::from_str(fixture_json)
        .map_err(|e| ConformanceError::Parse(e.to_string()))?;

    // Resolve document paths relative to base_dir.
    for path in fixture.documents.values_mut() {
        if !std::path::Path::new(path).is_absolute() {
            let resolved = std::path::Path::new(base_dir).join(&*path);
            *path = resolved
                .to_str()
                .ok_or_else(|| ConformanceError::Parse("document path is not valid UTF-8".into()))?
                .to_string();
        }
    }

    let mut engine = engine::WorkflowEngine::new(&fixture)?;
    let result = engine.execute(&fixture)?;

    Ok(result)
}

/// Result of running a conformance fixture.
#[derive(Debug)]
pub struct ConformanceResult {
    /// Whether all assertions passed.
    pub passed: bool,

    /// Failures, if any.
    pub failures: Vec<String>,

    /// Actual state transitions observed.
    pub transitions: Vec<engine::Transition>,

    /// Actual provenance records produced.
    pub provenance: Vec<ProvenanceRecord>,
}

/// Errors from the conformance runner.
#[derive(Debug, thiserror::Error)]
pub enum ConformanceError {
    /// Fixture or document parsing failed.
    #[error("parse error: {0}")]
    Parse(String),

    /// Workflow engine internal error.
    #[error("engine error: {0}")]
    Engine(String),

    /// Referenced document file not found.
    #[error("document not found: {0}")]
    DocumentNotFound(String),
}
