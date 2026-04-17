# WOS Structured Lint Diagnostics — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn `wos-lint` output from rendered log lines into a structured `LintDiagnostic` JSON type that LLMs (and downstream tooling) can consume directly. The same diagnostic stream renders as human prose through a presentation layer.

**Architecture:** Introduce a public `LintDiagnostic` struct with stable JSON serialization. All rules emit this type from their checker function. A thin formatter renders `text`, `pretty`, and `json` output modes over the same stream. Breaking change for CLI consumers; the `text` mode remains the human-friendly default.

**Tech Stack:** Rust (`wos-lint`), `serde` for JSON, existing rule infrastructure.

**Spec anchor:** [architecture-review-handoff.md §5.2](../archive/reviews/2026-04-16-architecture-review-handoff.md).

---

## Prerequisites

- [§4.2 rule-coverage plan](./2026-04-16-wos-rule-coverage-conformance.md) — rule metadata already includes `id`, `tier`, `severity`; diagnostic references these.
- Existing lint plumbing in `crates/wos-lint/`.

## Completion criteria

1. `LintDiagnostic` struct publicly exported from `wos-lint`, stably JSON-serializable.
2. All existing rules emit `LintDiagnostic` rather than free-form strings.
3. CLI has `--format=text|pretty|json` (default `text`).
4. JSON schema for `LintDiagnostic` is published so consumers can pin it (`schemas/lint/lint-diagnostic.schema.json`).
5. Fixture-based golden tests assert the JSON structure on representative rule failures.

## `LintDiagnostic` shape

```json
{
  "ruleId": "K-023",
  "severity": "error",
  "tier": "T1",
  "path": "$.states.approved",
  "message": "state 'approved' has no outbound transition and is not terminal",
  "suggestedFix": {
    "kind": "add-property",
    "path": "$.states.approved.type",
    "value": "terminal"
  },
  "relatedDocs": [
    "specs/kernel/spec.md#S4.2",
    "LINT-MATRIX.md#K-023"
  ],
  "source": {
    "document": "workflow.json",
    "line": 42,
    "column": 5
  }
}
```

- `suggestedFix` is optional; not every rule can propose one.
- `relatedDocs` is optional but strongly encouraged for rules that cite spec sections.
- `source` is absent when lint runs against an in-memory tree without a backing file.

## File structure

- **Create:** `crates/wos-lint/src/diagnostic.rs` — the struct + `serde` derive.
- **Create:** `schemas/lint/lint-diagnostic.schema.json` — public schema.
- **Modify:** every rule under `crates/wos-lint/src/rules/` — return `Vec<LintDiagnostic>`.
- **Modify:** `crates/wos-lint/src/main.rs` — `--format` flag and renderers.
- **Create:** `crates/wos-lint/tests/diagnostic_format.rs` — golden tests.

---

## Task 1: Failing test for diagnostic JSON shape

**Files:**
- Create: `crates/wos-lint/tests/diagnostic_format.rs`

- [ ] **Step 1.1:** Write a failing test that runs a known-bad fixture through lint and asserts the JSON output matches a golden:

```rust
#[test]
fn k023_emits_structured_diagnostic() {
    let doc = load_fixture("kernel/terminal-missing-transition-bad.json");
    let diagnostics = wos_lint::lint_document(&doc);
    let json = serde_json::to_value(&diagnostics[0]).unwrap();
    assert_eq!(json["ruleId"], "K-023");
    assert_eq!(json["severity"], "error");
    assert_eq!(json["tier"], "T1");
    assert!(json["path"].as_str().unwrap().starts_with("$."));
    assert!(json["relatedDocs"].as_array().unwrap().len() >= 1);
}
```

- [ ] **Step 1.2:** Run it — expect failure (LintDiagnostic does not yet exist).

## Task 2: Implement `LintDiagnostic`

**Files:**
- Create: `crates/wos-lint/src/diagnostic.rs`
- Modify: `crates/wos-lint/src/lib.rs` (export)

- [ ] **Step 2.1:** Struct with serde derive:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LintDiagnostic {
    pub rule_id: &'static str,
    pub severity: Severity,
    pub tier: Tier,
    pub path: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_fix: Option<SuggestedFix>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_docs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceLocation>,
}
```

- [ ] **Step 2.2:** Define `SuggestedFix` as an enum (`AddProperty`, `RemoveProperty`, `ReplaceValue`, `Rename`, `Custom(String)`), `serde(tag = "kind")`.

- [ ] **Step 2.3:** Define `SourceLocation` with `document`, `line`, `column`.

- [ ] **Step 2.4:** Commit. `feat: LintDiagnostic struct with stable JSON serialization`.

## Task 3: Migrate rules to emit `LintDiagnostic`

**Files:**
- Modify: every file under `crates/wos-lint/src/rules/`.

- [ ] **Step 3.1:** Change rule signatures from `fn check(doc: &Document) -> Vec<String>` to `fn check(doc: &Document) -> Vec<LintDiagnostic>`.

- [ ] **Step 3.2:** For each rule, populate:
  - `rule_id` — from existing metadata.
  - `tier`, `severity` — from existing metadata.
  - `path` — JSONPath to the offending node.
  - `message` — existing prose.
  - `related_docs` — spec sections already referenced in rule comments.
  - `suggested_fix` — only where the rule can propose a deterministic fix.

- [ ] **Step 3.3:** Run the existing test suite — every rule that previously asserted on string output now asserts on `.message`. Update as needed.

- [ ] **Step 3.4:** Commit per-file or per-rule-family. `refactor(wos-lint): migrate <family> rules to LintDiagnostic`.

## Task 4: Output formatters

**Files:**
- Modify: `crates/wos-lint/src/main.rs`
- Create: `crates/wos-lint/src/format/{text,pretty,json}.rs`

- [ ] **Step 4.1:** `--format=text` (default) — single-line per diagnostic, backward-compatible with current output format within reason.
- [ ] **Step 4.2:** `--format=pretty` — multiline human output with code excerpts from `source`.
- [ ] **Step 4.3:** `--format=json` — `serde_json::to_string_pretty` over `Vec<LintDiagnostic>`.
- [ ] **Step 4.4:** `--format=json-lines` — one diagnostic per line for streaming consumers (LLMs).
- [ ] **Step 4.5:** Commit. `feat: wos-lint --format=text|pretty|json|json-lines`.

## Task 5: Publish the diagnostic schema

**Files:**
- Create: `schemas/lint/lint-diagnostic.schema.json`

- [ ] **Step 5.1:** JSON Schema for `LintDiagnostic`. Include `patternProperties: {"^x-": ...}` and `additionalProperties: false` consistent with §4.1.

- [ ] **Step 5.2:** Add a generator test: derive the schema from the Rust type (via `schemars` crate) and assert it matches the committed schema. This prevents drift.

- [ ] **Step 5.3:** Commit. `feat: publish wos-lint diagnostic schema`.

## Task 6: Document the migration

**Files:**
- Create: `wos-spec/docs/lint-output-migration.md`

- [ ] **Step 6.1:** Explain the breaking change. Previous consumers parsed text output; they now consume JSON or accept the new text format (which is shape-compatible where possible).

- [ ] **Step 6.2:** Commit. `docs: wos-lint output migration guide`.

---

## Self-review checklist

- Diagnostic struct is public and stable (Task 2).
- All rules migrated; no rule left emitting raw strings (Task 3).
- Formatter covers text, pretty, json, json-lines (Task 4).
- Schema is published and prevented from drifting (Task 5).
- Migration doc exists (Task 6).

## Why this matters

LLMs consume JSON; humans consume rendered prose. The handoff calls this "the lint output becomes an API, not a log." This is a prerequisite for [§5.4 `wos-synth`](./2026-04-16-wos-synth-crate.md): the LLM authoring loop needs machine-readable diagnostics to close the feedback cycle.

**Estimated effort:** ~1 engineer-week.
