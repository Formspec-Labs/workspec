// Rust guideline compliant 2026-02-21

//! Lint-driven synthesis loop.
//!
//! [`synthesize`] drives the full generate-lint-repair cycle:
//!
//! 1. Call the Anthropic API with a generation prompt built from the problem text.
//! 2. Parse the response as JSON.
//! 3. Run [`wos_lint::lint_document`] on the raw JSON string.
//! 4. If there are zero error-severity diagnostics, return the parsed document.
//! 5. If errors remain and the iteration cap has not been hit, build a repair
//!    prompt and loop back to step 1.
//! 6. If the cap is reached with errors still present, return
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
            .map_err(|e| SpikeError::AnthropicApi(format!("lint pipeline error: {e}")))?;

        // Collect only error-severity findings — warnings and info are fine.
        let errors: Vec<String> = diagnostics
            .iter()
            .filter(|d| d.severity == wos_lint::Severity::Error)
            .map(|d| d.to_string())
            .collect();

        if errors.is_empty() {
            return Ok(parsed);
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
}
