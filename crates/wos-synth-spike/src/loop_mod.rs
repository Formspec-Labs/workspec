// Rust guideline compliant 2026-02-21

//! Lint- and conformance-gated synthesis loop.
//!
//! [`synthesize`] drives the full generate-lint-conformance-repair cycle:
//!
//! 1. Call the Anthropic API with a generation prompt built from the problem text.
//! 2. Parse the response as JSON.
//! 3. Run [`wos_lint::lint_document`] on the raw JSON string.
//! 4. If error-severity diagnostics remain and the iteration cap has not been
//!    hit, build a repair prompt and loop back to step 1.
//! 5. Once lint passes, run [`wos_conformance::run_fixture`] on a minimal
//!    smoke-test fixture wrapping the synthesized kernel. Failures become
//!    diagnostics fed to one additional repair pass; if the next attempt
//!    still fails conformance return [`SpikeError::ConformanceFailure`].
//! 6. If the cap is reached with lint errors still present, return
//!    [`SpikeError::Unconverged`].
//!
//! The loop deliberately skips `Warning` and `Info` diagnostics — only
//! `Error` severity blocks convergence, consistent with the spike's goal of
//! producing a structurally valid document rather than a polished one.

use serde_json::Value;

use crate::errors::SpikeError;
use crate::prompts::{build_generate_prompt, build_repair_prompt};

/// Maximum number of LLM calls before giving up.
///
/// Five iterations matches the spike plan's convergence cap.  Increasing this
/// value costs API time linearly; decreasing it reduces the chance of recovery
/// from a bad first attempt.
const MAX_ITERATIONS: u32 = 5;

/// Synthesize a valid WOS kernel document from a plain-English problem statement.
///
/// Calls the Anthropic API up to [`MAX_ITERATIONS`] times, applying lint
/// diagnostics as repair instructions between calls.  Returns the first
/// document that passes lint with zero error-severity findings.
///
/// # Errors
///
/// - [`SpikeError::AnthropicApi`] — the HTTP call failed or the API returned
///   an error status.
/// - [`SpikeError::ParseJson`] — the final LLM response is not valid JSON.
/// - [`SpikeError::Unconverged`] — the loop hit the iteration cap with lint
///   errors still present.
pub async fn synthesize(problem: &str, anthropic_key: &str) -> Result<Value, SpikeError> {
    let initial_prompt = build_generate_prompt(problem);
    let mut current_prompt = initial_prompt;
    let mut last_diagnostics: Vec<String> = Vec::new();

    for iteration in 1..=MAX_ITERATIONS {
        let response_text = call_anthropic(anthropic_key, &current_prompt).await?;
        let last_attempt = response_text.trim().to_string();

        // Strip markdown fences if the model added them despite instructions.
        let json_text = strip_fences(&last_attempt);

        let parsed: Value = match serde_json::from_str(json_text) {
            Ok(v) => v,
            Err(source) => {
                // Treat the parse failure as a single diagnostic and try to repair.
                let parse_diagnostic = format!(
                    "JSON parse error: {} — ensure the output is a bare JSON object with no \
                     markdown fences",
                    source
                );
                last_diagnostics = vec![parse_diagnostic.clone()];

                if iteration == MAX_ITERATIONS {
                    return Err(SpikeError::ParseJson {
                        attempt: last_attempt.clone(),
                        iterations: iteration,
                        source,
                    });
                }

                current_prompt = build_repair_prompt(json_text, &[parse_diagnostic]);
                continue;
            }
        };

        // Re-serialise to a canonical string for the linter (it needs `&str`).
        let canonical = serde_json::to_string(&parsed)
            .expect("re-serialising a parsed Value must not fail");

        let diagnostics = wos_lint::lint_document(&canonical)
            .map_err(classify_lint_error)?;

        // Collect only error-severity findings — warnings and info are fine.
        let errors: Vec<String> = diagnostics
            .iter()
            .filter(|d| d.severity == wos_lint::Severity::Error)
            .map(|d| d.to_string())
            .collect();

        if errors.is_empty() {
            return gate_on_conformance(parsed, json_text, iteration, anthropic_key).await;
        }

        last_diagnostics = errors.clone();

        if iteration == MAX_ITERATIONS {
            break;
        }

        current_prompt = build_repair_prompt(json_text, &errors);
    }

    Err(SpikeError::Unconverged {
        iterations: MAX_ITERATIONS,
        last_diagnostics: last_diagnostics.join("\n"),
    })
}

/// Smoke-test a lint-clean kernel document through the conformance engine and,
/// on failure, grant one additional repair round before surrendering.
///
/// This implements Task 4 of the v0 spike plan: "conformance gate after
/// lint-pass". The gate builds a minimal fixture -- no events, no expected
/// transitions -- that exercises the document-load path and the engine's
/// initial-configuration construction without asserting any runtime
/// behavior. A document that passes this smoke test is structurally valid
/// for the conformance harness; richer behavioral assertions stay out of
/// scope for the spike.
///
/// `initial_iteration` is the 1-based lint iteration index that produced the
/// lint-clean document; the repair round is attributed as
/// `initial_iteration + 1` so the retrospective can distinguish lint-phase
/// iterations from conformance-phase iterations.
async fn gate_on_conformance(
    doc: Value,
    doc_text: &str,
    initial_iteration: u32,
    anthropic_key: &str,
) -> Result<Value, SpikeError> {
    let first_failures = match run_conformance_smoke_test(&doc) {
        Ok(()) => return Ok(doc),
        Err(failures) => failures,
    };

    // The plan allows exactly one conformance-driven repair pass before giving
    // up -- we've already spent `initial_iteration` iterations on lint, so this
    // is the (initial_iteration + 1)th call.  Refuse to loop indefinitely if
    // lint took the full budget.
    if initial_iteration >= MAX_ITERATIONS {
        return Err(SpikeError::ConformanceFailure(first_failures.join("\n")));
    }

    let repair_prompt = build_repair_prompt(doc_text, &first_failures);
    let response_text = call_anthropic(anthropic_key, &repair_prompt).await?;
    let repaired_raw = response_text.trim().to_string();
    let repaired_text = strip_fences(&repaired_raw);

    let repaired: Value = serde_json::from_str(repaired_text).map_err(|source| {
        SpikeError::ParseJson {
            attempt: repaired_raw.clone(),
            iterations: initial_iteration + 1,
            source,
        }
    })?;

    // Re-run lint on the repair attempt -- a repair that re-introduces lint
    // errors must not be silently accepted.  Reuse the same Error-severity
    // filter the main loop applies so behaviour stays consistent.
    let canonical = serde_json::to_string(&repaired)
        .expect("re-serialising a parsed Value must not fail");
    let lint_errors: Vec<String> = wos_lint::lint_document(&canonical)
        .map_err(classify_lint_error)?
        .into_iter()
        .filter(|d| d.severity == wos_lint::Severity::Error)
        .map(|d| d.to_string())
        .collect();
    if !lint_errors.is_empty() {
        return Err(SpikeError::ConformanceFailure(format!(
            "conformance repair re-introduced lint errors: {}",
            lint_errors.join("; ")
        )));
    }

    match run_conformance_smoke_test(&repaired) {
        Ok(()) => Ok(repaired),
        Err(failures) => Err(SpikeError::ConformanceFailure(failures.join("\n"))),
    }
}

/// Wrap `doc` in a minimal inline conformance fixture and run it through the
/// engine; return the failures list when the fixture does not pass.
///
/// The fixture has empty `event_sequence` and empty `expected_transitions`,
/// so a "pass" means only that the engine could load the kernel and enter
/// its initial configuration -- not that any runtime behaviour was validated.
/// That is deliberate: the spike is proving the shape of the loop, not
/// proving the kernel's behavioural correctness.
fn run_conformance_smoke_test(doc: &Value) -> Result<(), Vec<String>> {
    let fixture = serde_json::json!({
        "binding": "formspec",
        "id": "v0-spike-smoke",
        "rule": "SPIKE-SMOKE",
        "description": "v0 spike conformance smoke test: kernel loads + initial config is reachable.",
        "documents": { "kernel": "inline" },
        "inline_documents": { "kernel": doc },
        "event_sequence": [],
        "expected_transitions": []
    });

    let fixture_text = serde_json::to_string(&fixture)
        .expect("fixture JSON must serialise");

    match wos_conformance::run_fixture(&fixture_text, ".") {
        Ok(result) if result.passed => Ok(()),
        Ok(result) => Err(result.failures),
        Err(err) => Err(vec![err.to_string()]),
    }
}

/// Call the Anthropic API and return the text content of the first response block.
///
/// Uses `anthropic-sdk` builder pattern.  The API key is passed explicitly
/// rather than read from the environment here so the caller controls injection.
///
/// # Errors
///
/// Returns [`SpikeError::AnthropicApi`] if the HTTP call fails or the SDK
/// returns an error.
async fn call_anthropic(api_key: &str, prompt: &str) -> Result<String, SpikeError> {
    use anthropic_sdk::Client;
    use serde_json::json;

    let mut collected = String::new();

    // The SDK's `execute` method takes a callback that receives text chunks.
    // We collect all chunks into a single string.
    Client::new()
        .auth(api_key)
        // claude-sonnet-4-6 is the current recommended model for code generation
        // tasks that require strong instruction following.  Opus is more expensive
        // and Haiku is faster but less reliable for structured JSON output.
        .model("claude-sonnet-4-6")
        .max_tokens(4096)
        // Temperature 0 maximises determinism — we want JSON, not creativity.
        .temperature(0.0)
        .messages(&json!([
            {
                "role": "user",
                "content": prompt
            }
        ]))
        .build()
        .map_err(|e| SpikeError::AnthropicApi(e.to_string()))?
        .execute(|chunk| {
            let text = chunk;
            collected.push_str(&text);
            async {}
        })
        .await
        .map_err(|e| SpikeError::AnthropicApi(e.to_string()))?;

    Ok(collected)
}

/// Sentinel substring produced by `wos_lint` when a document is parseable JSON
/// but lacks any `$wos*` document-type marker.
///
/// `wos_lint::LintError` does not expose a distinct discriminant for this case
/// — the condition is conveyed through the message string on
/// [`wos_lint::LintError::Parse`].  Matching the substring is fragile but is
/// the only available signal today; the spike accepts that coupling rather
/// than upstreaming a new variant.  See TODO in wos-lint document.rs.
const MISSING_MARKER_SENTINEL: &str = "no recognized $wos*";

/// Map a [`wos_lint::LintError`] to the appropriate [`SpikeError`] variant.
///
/// Routes the missing-`$wos*`-marker case to [`SpikeError::MissingWosMarker`]
/// and every other failure (malformed JSON reaching the linter, I/O errors)
/// to [`SpikeError::LintFailure`].  Kept as a free function so the mapping
/// can be unit-tested without running the full synthesis loop.
fn classify_lint_error(err: wos_lint::LintError) -> SpikeError {
    let message = err.to_string();
    if message.contains(MISSING_MARKER_SENTINEL) {
        SpikeError::MissingWosMarker(message)
    } else {
        SpikeError::LintFailure(message)
    }
}

/// Remove markdown code fences from LLM output if present.
///
/// The generation prompt instructs the model not to add fences, but some
/// models include them regardless.  This function strips the outermost
/// ` ```json ... ``` ` or ` ``` ... ``` ` wrapper if present.
fn strip_fences(text: &str) -> &str {
    let trimmed = text.trim();

    // Try ```json ... ``` first, then plain ``` ... ```.
    for prefix in &["```json\n", "```json\r\n", "```\n", "```\r\n", "```"] {
        if let Some(inner) = trimmed.strip_prefix(prefix) {
            if let Some(body) = inner.strip_suffix("```") {
                return body.trim();
            }
        }
    }

    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_fences_removes_json_fence() {
        let input = "```json\n{\"key\": 1}\n```";
        assert_eq!(strip_fences(input), "{\"key\": 1}");
    }

    #[test]
    fn strip_fences_removes_plain_fence() {
        let input = "```\n{\"key\": 1}\n```";
        assert_eq!(strip_fences(input), "{\"key\": 1}");
    }

    #[test]
    fn strip_fences_leaves_bare_json_unchanged() {
        let input = "{\"key\": 1}";
        assert_eq!(strip_fences(input), "{\"key\": 1}");
    }

    #[test]
    fn strip_fences_trims_surrounding_whitespace() {
        let input = "  \n  {\"key\": 1}  \n  ";
        assert_eq!(strip_fences(input), "{\"key\": 1}");
    }

    /// Feeding the linter a structurally valid JSON object with no `$wos*`
    /// marker must classify as [`SpikeError::MissingWosMarker`], not as a
    /// generic API or lint failure.  This is the dominant non-convergence
    /// mode — a canned LLM response that looks plausible but skips the
    /// discriminator field.
    #[test]
    fn classify_lint_error_routes_missing_marker() {
        // Stand-in for a canned LLM response: valid JSON, valid root object,
        // fields that look workflow-y, but no $wosKernel / $wosTheme / ...
        let canned_llm_response = r#"{
            "title": "Purchase order approval",
            "lifecycle": {"initialState": "draft", "states": {}}
        }"#;

        let err = wos_lint::lint_document(canned_llm_response)
            .expect_err("lint must fail when the document has no $wos* marker");

        match classify_lint_error(err) {
            SpikeError::MissingWosMarker(message) => {
                assert!(
                    message.contains("$wos"),
                    "MissingWosMarker message should mention $wos*, got: {message}"
                );
            }
            other => panic!(
                "expected SpikeError::MissingWosMarker for a marker-less document, \
                 got: {other:?}"
            ),
        }
    }

    /// A minimal well-formed kernel -- no events, no transitions -- must
    /// pass the conformance smoke test. This locks the Task 4 happy path:
    /// a lint-clean kernel that the engine can load and initialize flows
    /// through without a repair pass.
    #[test]
    fn conformance_smoke_test_accepts_minimal_kernel() {
        let doc = serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:formspec:test:spike-smoke:1.0.0",
            "actors": [{ "id": "operator", "type": "human" }],
            "impactLevel": "operational",
            "lifecycle": {
                "initialState": "idle",
                "states": { "idle": { "type": "final" } }
            }
        });

        super::run_conformance_smoke_test(&doc).expect("minimal kernel should pass");
    }

    /// A structurally broken kernel -- `initialState` names a state that does
    /// not exist -- must be rejected by the conformance engine. This proves
    /// the gate surfaces real errors, not just syntactic JSON failures.
    #[test]
    fn conformance_smoke_test_rejects_unreachable_initial_state() {
        let doc = serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:formspec:test:spike-broken:1.0.0",
            "actors": [{ "id": "operator", "type": "human" }],
            "impactLevel": "operational",
            "lifecycle": {
                "initialState": "nonexistent",
                "states": { "idle": { "type": "final" } }
            }
        });

        let failures = super::run_conformance_smoke_test(&doc)
            .expect_err("unreachable initial state must fail the smoke test");
        assert!(
            !failures.is_empty(),
            "expected at least one failure message, got: {failures:?}"
        );
    }

    /// Non-marker lint failures (e.g. malformed JSON reaching the linter)
    /// must route to [`SpikeError::LintFailure`] so the missing-marker
    /// channel stays specific.
    #[test]
    fn classify_lint_error_routes_generic_parse_failure() {
        let err = wos_lint::lint_document("not json at all")
            .expect_err("lint must fail on non-JSON input");

        match classify_lint_error(err) {
            SpikeError::LintFailure(_) => {}
            SpikeError::MissingWosMarker(message) => panic!(
                "malformed JSON should not be classified as a missing marker, \
                 got message: {message}"
            ),
            other => panic!("expected SpikeError::LintFailure, got: {other:?}"),
        }
    }
}
