// Rust guideline compliant 2026-02-21

//! Prompt builders for the synthesis loop.
//!
//! Two functions cover the two LLM call sites:
//!
//! - [`build_generate_prompt`] — first-pass generation from a plain-English
//!   problem statement.  Embeds the WOS kernel JSON Schema and a compact spec
//!   summary so the model has everything it needs in one context window.
//! - [`build_repair_prompt`] — repair pass that presents the previous attempt
//!   and a numbered list of lint diagnostics.  Instructs the model to return
//!   only the corrected JSON.
//!
//! Both functions are pure and have no side effects; they can be unit-tested
//! without network access.

/// WOS kernel JSON Schema, embedded at compile time.
///
/// Path is relative to this source file (`src/prompts.rs`): three levels up
/// reaches the workspace root, then into `schemas/kernel/`.
const KERNEL_SCHEMA: &str = include_str!("../../../schemas/kernel/wos-kernel.schema.json");

/// Compact spec summary for the kernel tier.
///
/// The LLM spec guide distils the normative behaviour in a size that fits
/// comfortably inside a single context window.
const KERNEL_SPEC_SUMMARY: &str = include_str!("../../../specs/kernel/spec.llm.md");

/// Build the initial generation prompt from a plain-English problem statement.
///
/// The returned string is a complete system+user message body ready to send as
/// the `user` turn in an Anthropic `/v1/messages` call.
///
/// # Examples
///
/// ```
/// use wos_synth_spike::prompts::build_generate_prompt;
///
/// let problem = "An employee submits a purchase order for manager approval.";
/// let prompt = build_generate_prompt(problem);
/// assert!(prompt.contains("$wosKernel"));
/// ```
pub fn build_generate_prompt(problem: &str) -> String {
    format!(
        r#"You are a workflow modelling expert who produces WOS (Workflow Orchestration Standard) kernel documents.

## Your task

Given the problem statement below, produce a single valid WOS kernel document as a JSON object.

## Requirements

- Output ONLY the JSON object.  No markdown fences, no prose, no explanation.
- The document MUST include the field `"$wosKernel": "1.0"`.
- Every state MUST have a `"type"` field (`"atomic"`, `"compound"`, `"parallel"`, or `"final"`).
- Every transition MUST have an `"event"` and a `"target"`.
- The `"lifecycle"` MUST have an `"initialState"` and a `"states"` map.
- Actor declarations MUST include `"id"` and `"type"` (`"human"` or `"system"`).
- Include a `"caseFile"` with typed `"fields"` for every data element mentioned.
- Include `"url"`, `"version"`, `"title"`, `"status"`, and `"impactLevel"`.

## WOS Kernel JSON Schema

```json
{KERNEL_SCHEMA}
```

## WOS Kernel Specification Summary

{KERNEL_SPEC_SUMMARY}

## Problem statement

{problem}

Produce the JSON document now."#,
        KERNEL_SCHEMA = KERNEL_SCHEMA,
        KERNEL_SPEC_SUMMARY = KERNEL_SPEC_SUMMARY,
        problem = problem,
    )
}

/// Build a repair prompt from a previous attempt and its lint diagnostics.
///
/// Each diagnostic in `diagnostics` is rendered as a numbered list item.
/// The model is instructed to correct every listed error and return only the
/// corrected JSON — no prose, no fences.
///
/// # Examples
///
/// ```
/// use wos_synth_spike::prompts::build_repair_prompt;
///
/// let attempt = r#"{"$wosKernel": "1.0"}"#;
/// let diagnostics = vec!["[K-001] error at /lifecycle: required field missing".to_string()];
/// let prompt = build_repair_prompt(attempt, &diagnostics);
/// assert!(prompt.contains("K-001"));
/// assert!(prompt.contains(attempt));
/// ```
pub fn build_repair_prompt(prior_attempt: &str, diagnostics: &[String]) -> String {
    let numbered_diagnostics = diagnostics
        .iter()
        .enumerate()
        .map(|(i, d)| format!("{}. {}", i + 1, d))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"The WOS kernel document you produced has lint errors that must be fixed.

## Lint errors to correct

{numbered_diagnostics}

## Prior attempt (contains errors)

```json
{prior_attempt}
```

## Instructions

- Fix every error listed above.
- Output ONLY the corrected JSON object.
- No markdown fences, no prose, no explanation — just the raw JSON.
- Preserve all parts of the document that were already correct.

Produce the corrected JSON now."#,
        numbered_diagnostics = numbered_diagnostics,
        prior_attempt = prior_attempt,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_prompt_contains_schema_marker() {
        let prompt = build_generate_prompt("test problem");
        assert!(
            prompt.contains("$wosKernel"),
            "generate prompt must reference the $wosKernel marker"
        );
    }

    #[test]
    fn generate_prompt_embeds_problem() {
        let problem = "unique-sentinel-string-12345";
        let prompt = build_generate_prompt(problem);
        assert!(
            prompt.contains(problem),
            "generate prompt must embed the problem text"
        );
    }

    #[test]
    fn repair_prompt_numbers_diagnostics() {
        let diagnostics = vec![
            "error A".to_string(),
            "error B".to_string(),
            "error C".to_string(),
        ];
        let prompt = build_repair_prompt("{}", &diagnostics);
        assert!(prompt.contains("1. error A"));
        assert!(prompt.contains("2. error B"));
        assert!(prompt.contains("3. error C"));
    }

    #[test]
    fn repair_prompt_embeds_prior_attempt() {
        let attempt = r#"{"$wosKernel":"1.0","unique":true}"#;
        let prompt = build_repair_prompt(attempt, &[]);
        assert!(
            prompt.contains(attempt),
            "repair prompt must embed the prior attempt JSON"
        );
    }

    #[test]
    fn repair_prompt_empty_diagnostics() {
        // Edge case: called with no diagnostics — should not panic.
        let prompt = build_repair_prompt("{}", &[]);
        assert!(!prompt.is_empty());
    }
}
