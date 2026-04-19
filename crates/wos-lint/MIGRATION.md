# wos-lint: LintDiagnostic migration guide

`wos-lint` now emits structured `LintDiagnostic` objects instead of the
minimal `Diagnostic` type it used previously. This guide explains what
changed, why it changed, and how to update code that depends on this crate.

## What changed

### Old shape (`Diagnostic`)

Previously all lint functions returned `Vec<Diagnostic>`, where `Diagnostic`
carried only four fields:

```rust
pub struct Diagnostic {
    pub rule_id: &'static str,  // e.g. "K-023"
    pub path:    String,        // e.g. "/lifecycle/states/submitted"
    pub message: String,        // free-form prose
    pub severity: Severity,     // Error | Warning | Info
}
```

JSON consumers had to parse unstructured text — there was no verification
tier, no machine-readable fix, and no spec reference. The path used JSON
Pointer syntax (`/lifecycle/states/submitted`), which differs from JSONPath.

### New shape (`LintDiagnostic`)

The new type is a richer, JSON-serializable struct:

```rust
pub struct LintDiagnostic {
    pub rule_id:       &'static str,        // stable rule identifier
    pub severity:      LintSeverity,        // Error | Warning | Info
    pub tier:          Tier,                // T1 | T2 | T3
    pub path:          String,              // JSONPath, e.g. "$.states.approved"
    pub message:       String,              // human-readable prose
    pub suggested_fix: Option<SuggestedFix>,// machine-readable fix, if deterministic
    pub related_docs:  Vec<String>,         // spec sections + LINT-MATRIX refs
    pub source:        Option<SourceLocation>, // file + line + column, if available
}
```

Serialized to JSON with camelCase field names (stable contract):

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
  "relatedDocs": ["specs/kernel/spec.md#S4.2", "LINT-MATRIX.md#K-023"]
}
```

The full JSON Schema is published at
`schemas/lint/wos-lint-diagnostic.schema.json`.

## Why we migrated

The lint output is consumed by the `wos-synth-core` LLM repair loop across
two crate boundaries. LLMs consume JSON; humans consume rendered prose.
Keeping both in the same diagnostic stream — structured JSON for machines,
formatted text for humans — requires a stable typed intermediate form.
Free-form text strings are not that form.

The change was also a prerequisite for SARIF output (consumed by GitHub code
scanning) and the `json-lines` streaming format (consumed by LLM agents that
process diagnostics incrementally).

## Field-by-field mapping

| Old field | New field | Notes |
|-----------|-----------|-------|
| `rule_id` | `ruleId` | Same value, camelCase in JSON |
| `path` | `path` | Path syntax changed — see below |
| `message` | `message` | Same prose |
| `severity` | `severity` | New enum type; same variants |
| _(none)_ | `tier` | New: `"T1"` / `"T2"` / `"T3"` |
| _(none)_ | `suggestedFix` | New: machine-readable fix proposal |
| _(none)_ | `relatedDocs` | New: spec section references |
| _(none)_ | `source` | New: file + line + column |

**Path syntax changed:** the old `Diagnostic.path` used JSON Pointer
(`/lifecycle/states/submitted`). The new `LintDiagnostic.path` uses JSONPath
(`$.lifecycle.states.submitted`). Code that constructs JSON Pointer strings
to match diagnostic paths must be updated.

**Severity enum type changed:** old code compared against `wos_lint::Severity`;
new code compares against `wos_lint::LintSeverity`. Both are exported for the
transition period.

## How to update your code

### Option 1 — use the new structured API (recommended)

Switch from `lint_document` to `lint_document_structured`:

```rust
// Before
use wos_lint::{lint_document, Diagnostic, Severity};
let diagnostics: Vec<Diagnostic> = lint_document(&json)?;
for d in &diagnostics {
    if d.severity == Severity::Error { ... }
}

// After
use wos_lint::{lint_document_structured, LintDiagnostic, LintSeverity};
let diagnostics: Vec<LintDiagnostic> = lint_document_structured(&json)?;
for d in &diagnostics {
    if d.severity == LintSeverity::Error { ... }
    // d.tier, d.suggested_fix, d.related_docs now available
}
```

The structured equivalents for all three public lint functions:

| Legacy | Structured |
|--------|------------|
| `lint_document(json)` | `lint_document_structured(json)` |
| `lint_project(dir)` | `lint_project_structured(dir)` |
| `lint_schema(schema_json)` | `lint_schema_structured(schema_json)` |

### Option 2 — keep the legacy API (no immediate changes needed)

The legacy `lint_document`, `lint_project`, and `lint_schema` functions still
exist and still return `Vec<Diagnostic>`. They now delegate to the structured
versions and convert results via `From<LintDiagnostic> for Diagnostic`. No
callsite changes are required for code that only reads `rule_id`, `path`,
`message`, and `severity`.

Note: the legacy `Diagnostic` type loses `tier`, `suggested_fix`,
`related_docs`, and `source` in the conversion. Migrate to the structured API
when you need those fields.

### Output formatting

The new `wos_lint::output` module provides three formatters over
`&[LintDiagnostic]`:

```rust
use wos_lint::output;

// Plain text — one line per diagnostic (backward-compatible default)
let text = output::format_text(&diagnostics);

// JSON array — stable schema, suitable for piping to jq
let json = output::format_json(&diagnostics);

// SARIF 2.1.0 — consumed by GitHub code scanning
let sarif = output::format_sarif(&diagnostics);
```

## Backward-compatible callers

The following crates in this workspace use the legacy API and have been
verified to compile without changes:

- `crates/wos-mcp` — uses `lint_document` → `Vec<Diagnostic>`
- `crates/wos-conformance` — uses `lint_document` → `Vec<Diagnostic>`

When those crates are ready to consume structured diagnostics (e.g., to
surface `suggested_fix` in repair loops), switch to the `_structured` variants.
