# wos-lint: LintDiagnostic migration guide

`wos-lint` emits structured `LintDiagnostic` values. This guide documents the
shape, why it exists, and how to migrate older call sites that still assumed
JSON Pointer paths or a slimmer diagnostic struct.

## What changed

### Historical shape (removed)

Older releases returned a minimal `Diagnostic` with only four fields:

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

**Severity enum:** compare against `wos_lint::LintSeverity` (`Error`, `Warning`, `Info`).

## How to update your code

`lint_document`, `lint_project`, and `lint_schema` return `Vec<LintDiagnostic>` directly.
The legacy `Diagnostic` and `Severity` types and the `*_structured` function names were removed.

```rust
use wos_lint::{lint_document, LintDiagnostic, LintSeverity};

let diagnostics: Vec<LintDiagnostic> = lint_document(&json)?;
for d in &diagnostics {
    if d.severity == LintSeverity::Error { /* ... */ }
    // d.tier, d.suggested_fix, d.related_docs, d.source
}
```

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

## Workspace callers

Downstream crates (`wos-mcp`, `wos-server`, `wos-synth-core`, `wos-conformance`, etc.)
use `lint_document` / `lint_project` / `lint_schema` with `LintDiagnostic` and `LintSeverity`.
