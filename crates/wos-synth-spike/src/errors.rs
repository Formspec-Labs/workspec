// Rust guideline compliant 2026-02-21

//! Error types for the WOS synth spike.
//!
//! A flat enum covering every failure mode the spike can encounter:
//! API errors, JSON parse failures, lint-driven non-convergence, and
//! missing configuration.  All variants implement [`std::error::Error`]
//! via [`thiserror`].

/// All failure modes produced by the synthesis loop.
#[derive(Debug, thiserror::Error)]
pub enum SpikeError {
    /// The Anthropic API call failed at the HTTP or transport layer.
    #[error("Anthropic API error: {0}")]
    AnthropicApi(String),

    /// The LLM produced JSON without a recognized `$wos*` document-type marker.
    ///
    /// This is the single most likely non-convergence failure mode — the model
    /// emits structurally valid JSON that the linter cannot classify because no
    /// `$wosKernel` / `$wosTheme` / etc. field is present.  Kept distinct from
    /// [`SpikeError::LintFailure`] so callers can detect and surface it
    /// specifically (e.g., tighten the generation prompt).
    #[error(
        "LLM output is missing a $wos* document-type marker \
         (e.g. \"$wosKernel\": \"1.0\"): {0}"
    )]
    MissingWosMarker(String),

    /// The lint pipeline itself failed for a reason other than a missing marker.
    ///
    /// Covers malformed JSON reaching the linter, filesystem errors from
    /// `wos_lint::lint_project`, and any other [`wos_lint::LintError`] variant
    /// that does not indicate a missing document-type marker.
    #[error("lint pipeline error: {0}")]
    LintFailure(String),

    /// The conformance harness rejected the document after lint had passed.
    ///
    /// The spike wraps the synthesized kernel in a minimal smoke-test fixture
    /// (empty `event_sequence`, empty `expected_transitions`) and runs it
    /// through [`wos_conformance::run_fixture`]. Any failure the engine
    /// surfaces (document-load errors, engine-internal errors, fixture parse
    /// errors) ends up here when it persists past the one allowed repair
    /// pass. Kept distinct from [`SpikeError::LintFailure`] so the retrospective
    /// can attribute convergence failures to the right gate.
    #[error("conformance gate failed: {0}")]
    ConformanceFailure(String),

    /// The LLM produced output that is not parseable as JSON.
    #[error("JSON parse error after {iterations} iteration(s): {source}\nRaw attempt:\n{attempt}")]
    ParseJson {
        /// Raw text returned by the LLM.
        attempt: String,
        /// How many synthesis iterations had run before the parse failed.
        iterations: u32,
        /// The underlying parse error.
        source: serde_json::Error,
    },

    /// The loop exhausted its iteration cap without reaching zero lint errors.
    #[error(
        "did not converge after {iterations} iteration(s); \
         last diagnostics:\n{last_diagnostics}"
    )]
    Unconverged {
        /// Number of iterations attempted.
        iterations: u32,
        /// Diagnostic messages from the final attempt, joined by newlines.
        last_diagnostics: String,
    },

    /// `ANTHROPIC_API_KEY` was not set in the environment.
    #[error(
        "ANTHROPIC_API_KEY is not set — export it before running the spike:\n  \
         export ANTHROPIC_API_KEY=sk-ant-..."
    )]
    MissingApiKey,

    /// A filesystem operation failed (reading the problem file or writing output).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
