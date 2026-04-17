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
pub mod formspec_processor;
mod meta;
mod provenance;
pub mod rules;
pub mod stubs;
pub mod trace;

pub use engine::WorkflowEngine;
pub use fixture::{
    ConformanceFixture, ContractOutcome, EventEntry, ExpectedTransition, TaskSubmission,
};
pub use meta::{
    observe_delegated_formspec_evaluation, run_profile_against_fixtures,
    validate_ai_family_batch_coverage, verify_processor_manifest, AssistGovernanceProxyEvidence,
    ClaimStatus, ClaimVerification, DelegatedFormspecEvaluationEvidence, ProcessorClaims,
    ProcessorConformanceReport, ProcessorEvidence, ProcessorManifest, AI_CONFIDENCE_BATCHES,
    AI_REGISTRATION_BATCHES, GOVERNANCE_BASIC_RULES,
};
pub use provenance::{ProvenanceKind, ProvenanceRecord};
pub use stubs::{StubService, StubValidator};
pub use trace::{
    ConformanceTrace, Delta, Event, GuardEvaluation, Outcome, PolicyApplication, TraceStep,
};
pub use wos_core::proxy::observe_assist_governance_proxy;

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
pub fn run_fixture(
    fixture_json: &str,
    base_dir: &str,
) -> Result<ConformanceResult, ConformanceError> {
    let mut fixture: ConformanceFixture =
        serde_json::from_str(fixture_json).map_err(|e| ConformanceError::Parse(e.to_string()))?;

    // Resolve file-backed document paths relative to base_dir. The sentinel
    // value "inline" is resolved by `WorkflowEngine` from `inline_documents`.
    for path in fixture.documents.values_mut() {
        if path == "inline" {
            continue;
        }
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

    /// Binding discriminator used during execution.
    pub binding_used: Option<String>,
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
