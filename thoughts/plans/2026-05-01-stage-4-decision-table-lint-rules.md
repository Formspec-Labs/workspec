# Stage 4 — DecisionTable Lint Rules (K-051 / K-052 / K-053)

> **Note (2026-05-02):** This plan was rewritten in remediation Wave 4
> following the semi-formal code review of the original plan. The original
> plan claimed (1) the conformance harness already runs lint and (2) the
> wos-core typed model only "may need to catch up" — both wrong. The
> conformance harness has zero `wos_lint` imports; `Transition.guard` is
> `Option<String>` not polymorphic; there is no `wos_core::decision_table`
> module. The corrected plan adds **Wave 0 (wos-core surface area)** and
> **Wave 1' (new lint-test target)** as prerequisites, then ships the lint
> rule itself in Wave 2'.

## Context

Parent kernel `decisionTable` construct landed 2026-05-01 (Kernel §4.5.1 +
`wos-workflow.schema.json` `$defs/{DecisionTable, DecisionTableRow,
DecisionTableGuard}`). LINT-MATRIX.md catalogues three accompanying lint
rules as `draft` graduation:

- **K-051** (T1) — DecisionTableGuard `ref` MUST resolve; `outputColumn`
  MUST exist on the referenced table; every declared input MUST have an
  `inputBindings` entry.
- **K-052** (T2) — `unique`/`priority` hit-policy rows MUST be pairwise
  disjoint over the declared input domain (priority ties on overlapping
  rows are violations too).
- **K-053** (T1) — DecisionTable input cells MUST evaluate to boolean;
  transition-guard `outputColumn` MUST be `boolean`-typed; `collect` hit
  policy is rejected for transition-guard usage.

Catalog entries exist; Rust impl + executable fixtures do not. **10
conformance fixtures land at `crates/wos-conformance/fixtures/K-05[123]-*.json`** —
they pin the verification surface. After Wave 4 of the review remediation
(2026-05-02): the witness-coupling phrasing in K-052 fixtures is decoupled;
the `positive-` prefix is dropped to match the existing K-001/K-033 bare-
name convention.

This plan must be executed in an environment where `cargo` can resolve the
parent `fel-core` crate at `../../../crates/fel-core` (sandbox absent that
path).

## Recommended approach

Three sequential waves. Each is a real undertaking; the original "single
follow-on commit closes Stage 4 first cut" framing was wrong.

### Wave 0 — `wos-core` Rust surface area for DecisionTable (~600-1000 lines)

The schema landed 2026-05-01 but the Rust typed model never caught up.
Verified facts:

- `crates/wos-core/src/model/kernel.rs:565` declares `pub guard:
  Option<String>` on `Transition`. The schema makes guard polymorphic
  (`oneOf [string, DecisionTableGuard]`); the Rust enum to distinguish
  does not exist.
- `KernelDocument` (kernel.rs:13-83) has no `decision_tables` field. The
  schema's top-level `decisionTables[]` deserializes to nothing.
- `find crates/wos-core/src -name "*.rs" | xargs grep "decision_table"`
  returns zero hits. There is no module to import from.
- `crates/wos-core/src/model/kernel.rs:14-15` does NOT use
  `#[serde(deny_unknown_fields)]`. So unknown `decisionTables`
  silently drops AND structured guards silently deserialize as `None`.
  Positive fixtures would pass for the wrong reason.

**Wave 0 deliverables:**

1. New module `crates/wos-core/src/model/decision_table.rs` declaring:
   ```rust
   pub struct DecisionTable {
       pub id: String,
       pub description: Option<String>,
       pub inputs: Vec<DecisionTableInput>,
       pub outputs: Vec<DecisionTableOutput>,
       pub rows: Vec<DecisionTableRow>,
       pub hit_policy: HitPolicy,
   }
   pub struct DecisionTableInput { name: String, type_: FelType, description: Option<String> }
   pub struct DecisionTableOutput { name: String, type_: FelType, description: Option<String> }
   pub struct DecisionTableRow {
       pub id: String,
       pub input_cells: Vec<String>,
       pub output_cells: Vec<String>,
       pub priority: Option<i64>,
       pub rationale: Option<String>,
   }
   pub enum HitPolicy { Unique, First, Priority, Collect }
   pub struct DecisionTableGuard {
       pub kind: DecisionTableGuardKind,  // const "decisionTable"
       pub r#ref: String,
       pub output_column: String,
       pub input_bindings: BTreeMap<String, String>,
       pub on_no_match: Option<OnNoMatch>,  // false (default) | fail
   }
   pub enum Guard { FelString(String), DecisionTable(DecisionTableGuard) }
   ```
   Use `#[serde(untagged)]` on `Guard` so it deserializes from the polymorphic
   schema shape. Add `#[serde(rename_all = "camelCase")]` to match schema casing.

2. Refactor `crates/wos-core/src/model/kernel.rs:565`:
   `pub guard: Option<String>` → `pub guard: Option<Guard>`.
   Touches every consumer of `transition.guard` in:
   - `crates/wos-core/src/...` (any direct field access)
   - `crates/wos-runtime/src/...` (guard evaluation — needs to call FEL OR
     decision-table evaluator)
   - `crates/wos-lint/src/rules/...` (continuous_mode.rs and tier2.rs walk
     guards as strings; update to handle both variants)
   - `crates/wos-conformance/src/...` (engine.rs:347-349 evaluates guards;
     update similarly)

3. Add `pub decision_tables: Option<Vec<DecisionTable>>` to
   `KernelDocument` (kernel.rs:13-83).

4. Acceptance: `cargo check --workspace` passes; existing K-001/K-049
   fixtures still pass; `cargo nextest run -p wos-core --lib` passes.

### Wave 1' — New lint-test target (~150 lines)

The original plan claimed `cargo nextest run -p wos-conformance` would
exercise K-051/52/53 fixtures because the harness "auto-discovers" them.
False:

- `crates/wos-conformance/src/engine.rs:1-50` imports `wos_runtime`,
  `wos_core`, but NEVER `wos_lint`.
- Engine asserts `expected_errors[]` against runtime engine failures
  (engine.rs:455-470 substring-match), not lint diagnostics.
- The only place lint runs over fixtures is `crates/wos-lint/tests/
  tier2_rules.rs` (per registry.rs:1028-1030) which loads K-049 fixtures
  from a different path (`fixtures/validation/`) with a totally different
  fixture format (bare kernel doc, not the wrapped `inline_documents.kernel`
  shape).

**Wave 1' deliverable:** new file `crates/wos-lint/tests/decision_table_fixtures.rs`
that:

1. Loads each `crates/wos-conformance/fixtures/K-05[123]-*.json` (10 fixtures).
2. For each fixture: deserializes `inline_documents.kernel` as
   `KernelDocument` (using the Wave-0 typed model).
3. Runs the appropriate lint rule (K-051/052/053) against the typed kernel.
4. Asserts `expected_errors[i]` substrings appear in the diagnostic stream
   (use `Diagnostic::message` substring-match — same convention as
   conformance engine).

Acceptance: `cargo nextest run -p wos-lint --test decision_table_fixtures`
passes for all 10 fixtures.

### Wave 2' — DecisionTable lint rule implementation (~400-600 lines)

Module `crates/wos-lint/src/rules/decision_table.rs` with three public
entry points:

```rust
//! K-051 / K-052 / K-053 — DecisionTable lint rules per Kernel §4.5.1.

use crate::diagnostic::{Diagnostic, LintSeverity};
use wos_core::model::kernel::KernelDocument;          // CORRECTED: was
                                                      // wos_core::document::WosWorkflow
use wos_core::model::decision_table::{                // CORRECTED: was
    DecisionTable, DecisionTableGuard, Guard, HitPolicy,  // wos_core::decision_table::*
};

pub fn run_k051(doc: &KernelDocument) -> Vec<Diagnostic> { /* ... */ }
pub fn run_k052(doc: &KernelDocument) -> Vec<Diagnostic> { /* ... */ }
pub fn run_k053(doc: &KernelDocument) -> Vec<Diagnostic> { /* ... */ }
```

**K-051 — structural resolution (T1, ~150 lines).** Walk every transition
guard. When `Guard::DecisionTable(g)` variant: verify `g.r#ref` resolves to
a `decision_tables[]` entry; `g.output_column` resolves to a declared
output; every declared input has an entry in `g.input_bindings`. One
diagnostic per violation.

**K-052 — row overlap detection (T2, ~250 lines).** For each
`DecisionTable` with `hit_policy ∈ {Unique, Priority}`:

- Parse input-cell FEL expressions via `fel_core::parse` (already a wos-lint
  dependency).
- For each pair of rows: try to prove their input-cell predicates are
  disjoint. Pragmatic implementation handles single-variable linear
  inequalities (`<`, `<=`, `>`, `>=`) and equality predicates over
  enum-typed inputs. Cases the analyzer cannot prove disjoint emit
  `LintSeverity::Warning` ("potential overlap; manual review"); cases it
  proves overlap emit `LintSeverity::Error`.
- For `Priority`: when two overlapping rows share `priority`, emit
  `LintSeverity::Error` (no deterministic winner).

**K-053 — cell shape + hit-policy validity (T1, ~100 lines).** Two checks:

1. For every transition's `Guard::DecisionTable(g)`: the referenced
   table's `output_column` MUST be `FelType::Boolean`. Non-boolean →
   K-053 error.
2. For every transition's `Guard::DecisionTable(g)`: the referenced
   table's `hit_policy` MUST NOT be `Collect`. Collect-on-guard → K-053
   error.

Optional (defer if scope balloons): input-cell AST root must be boolean-
shaped via FEL type inference. Defer to Wave 2'.5 if not in initial impl.

### JSONPath shape decision

Three formats currently disagree:

- Fixtures predict `lifecycle.states.intake.transitions[0].guard` (dotted,
  no leading slash).
- K-049 impl emits `/lifecycle/states/idle/transitions/0` (slash, leading
  slash) per `crates/wos-lint/src/rules/continuous_mode.rs:206-209,274`.
- `LintDiagnostic::path` doc-string at `crates/wos-lint/src/diagnostic.rs:104`
  says `$.states.approved` (JSONPath with `$.`).

**Decision:** use slash format to match K-049 impl. Update
`crates/wos-lint/src/diagnostic.rs:104` doc-string accordingly. Update the
10 fixtures' `expected_errors` substrings (where the path is referenced)
to use slash format. Document the format choice in `diagnostic.rs` so
future rules don't drift.

The conformance harness uses `f.contains(expected_err.as_str())` substring
matching, so as long as the rule emits the slash format AND fixture
expected strings contain the slash format substring, validation passes.

### Registry entries

Append to `crates/wos-lint/src/rules/registry.rs:ALL_LINT_RULES`:

```rust
RuleMetadata {
    id: "K-051",
    tier: Tier::T1,
    severity: LintSeverity::Error,
    summary: "DecisionTableGuard ref/outputColumn/inputBindings MUST resolve.",
    fixtures: &[
        "K-051-resolved-decision-table-guard.json",
        "K-051-negative-unresolved-table-ref.json",
        "K-051-negative-unresolved-output-column.json",
        "K-051-negative-missing-input-binding.json",
    ],
    graduation: Graduation::Tested,
    spec_ref: Some("kernel/spec.md#§4.5.1"),
    suggested_fix: Some("Ensure DecisionTableGuard.ref names a top-level decisionTables[] entry, outputColumn names a declared output, and inputBindings covers every declared input."),
},
RuleMetadata {
    id: "K-052",
    tier: Tier::T2,
    severity: LintSeverity::Error,
    summary: "DecisionTable rows for hitPolicy=unique/priority MUST be pairwise disjoint (or have distinct priorities under priority).",
    fixtures: &[
        "K-052-disjoint-unique-rows.json",
        "K-052-negative-overlapping-unique-rows.json",
        "K-052-negative-priority-tie.json",
    ],
    graduation: Graduation::Tested,
    spec_ref: Some("kernel/spec.md#§4.5.1.4"),
    suggested_fix: Some("Make rows pairwise disjoint, switch to hitPolicy=first, or assign distinct priority integers."),
},
RuleMetadata {
    id: "K-053",
    tier: Tier::T1,
    severity: LintSeverity::Error,
    summary: "DecisionTable cell-shape: input cells boolean; transition-guard outputColumn boolean-typed; no collect hit policy on guards.",
    fixtures: &[
        "K-053-boolean-output-column.json",
        "K-053-negative-non-boolean-output.json",
        "K-053-negative-collect-hit-policy-on-guard.json",
    ],
    graduation: Graduation::Tested,
    spec_ref: Some("kernel/spec.md#§4.5.1.4"),
    suggested_fix: Some("Select a boolean-typed output column for transition-guard usage; avoid collect hit policy on tables referenced by guards."),
},
```

### LINT-MATRIX.md update

After Rust impl + fixtures pass:

- K-051: `draft → tested` with fixture evidence column listing the 4 K-051 fixtures.
- K-052: `draft → tested` with 3 K-052 fixtures.
- K-053: `draft → tested` with 3 K-053 fixtures.

Counts: T1: still 38 (K-051 and K-053 already counted as T1 in the catalog).
T2: still 75. Tested counts increment by 3.

## Critical files

Wave 0 (~600-1000 lines):
- `crates/wos-core/src/model/decision_table.rs` — NEW
- `crates/wos-core/src/model/kernel.rs` — Transition.guard type change;
  KernelDocument.decision_tables field added
- `crates/wos-core/src/model/mod.rs` — `pub mod decision_table;`
- `crates/wos-core/Cargo.toml` — no new deps expected (serde + indexmap
  already present)
- consumers: `crates/wos-runtime/...`, `crates/wos-lint/src/rules/...`,
  `crates/wos-conformance/src/engine.rs` — guard-evaluation pattern
  match against `Guard` enum variants

Wave 1' (~150 lines):
- `crates/wos-lint/tests/decision_table_fixtures.rs` — NEW

Wave 2' (~400-600 lines):
- `crates/wos-lint/src/rules/decision_table.rs` — NEW
- `crates/wos-lint/src/rules/mod.rs` — `pub mod decision_table;`
- `crates/wos-lint/src/rules/registry.rs` — append 3 RuleMetadata entries
- `crates/wos-lint/src/lib.rs` — wire `decision_table::run_all` into the
  T1 + T2 dispatch (matches existing `continuous_mode::run_k049` pattern)
- `crates/wos-lint/src/diagnostic.rs:104` — JSONPath shape doc-string
  decision (slash format)
- `LINT-MATRIX.md` — graduation + fixture-evidence column updates for
  K-051 / K-052 / K-053

Reference shapes (read-only):
- `crates/wos-lint/src/rules/continuous_mode.rs` — exemplar T2 rule with
  FEL AST analysis (path format ground truth)
- `crates/wos-lint/src/rules/tier1.rs` — exemplar T1 structural rules
- `crates/wos-lint/tests/tier2_rules.rs` — exemplar test target loading
  K-049 fixtures (note: different fixture path + format than
  conformance fixtures)

## Verification

End-to-end test plan:

1. **Schema validity (sandbox-runnable, already verified):** All 10 new
   fixture files parse as JSON and their `inline_documents.kernel` blocks
   validate clean against `wos-workflow.schema.json` — confirmed locally
   before this plan was written.
2. **Wave 0:** `cargo check --workspace` passes; `cargo nextest run -p
   wos-core --lib` passes; the new types serialize round-trip through serde.
3. **Wave 1':** `cargo nextest run -p wos-lint --test decision_table_fixtures`
   exists and passes with 10 fixtures. (Initially red; goes green when
   Wave 2' lint impl ships.)
4. **Wave 2':** `cargo nextest run -p wos-lint --lib decision_table` passes
   per-rule unit tests; `cargo nextest run -p wos-lint --test
   decision_table_fixtures` goes green; positive cases produce 0
   diagnostics; each negative case produces exactly 1 diagnostic with
   matching `rule_id` + message substring.
5. **Workspace check:** `cargo check --workspace` — no regressions in
   dependent crates (`wos-runtime`, `wos-conformance`, etc.) after
   `Transition.guard` type change.
6. **Rule-coverage ratchet:** `every_load_bearing_conformance_rule_has_at_
   least_two_executable_fixtures` — K-051 (4), K-052 (3), K-053 (3) all
   satisfy. K-051+K-052+K-053 may promote to `LoadBearing` after 3
   consecutive releases at `Tested` per registry.rs comments.

Per-wave commit messages follow the existing convention (subject ≤72 chars,
body explains rationale + cross-cutting impact, ends with `Co-Authored-By:
Claude <noreply@anthropic.com>`).

## Out of scope of this plan

- The remaining ~80 readiness rules (EFF-LINT / AI-LINT / CMP-LINT / EQ-LINT
  / ACC-LINT / JUR-LINT / ID-LINT / COMP-LINT / CHAIN-LINT / TERM-LINT /
  WF-LINT / POM-LINT / etc. families). Each becomes its own Stage-4
  mini-plan once K-051/K-052/K-053 prove the implementation pattern.
- Studio-specific lint rules (the SA-MUST-* family). Composes via similar
  pattern but lives in a separate `crates/wos-studio-lint` crate or a
  sub-module of `wos-lint`.
- Runtime evaluator for DecisionTableGuard in `wos-runtime` (Kernel §4.5.1.2
  algorithm) — that's runtime work, not lint work. Track separately.
- Promotion of K-051/K-052/K-053 to `LoadBearing` graduation — happens after
  3 consecutive releases at `Tested` per the existing graduation ladder.

## Risks

1. **Wave 0 is the long pole.** The original plan understated this as
   "Rust types may need to catch up." Reality: a substantive wos-core
   change touching every consumer of `transition.guard` (wos-core, wos-
   runtime, wos-lint, wos-conformance), plus serde adjacent-tagged or
   untagged deser config, plus the FEL evaluator integration. Budget
   600-1000 lines.

2. **K-052 satisfiability is hard in the general case.** Pragmatic
   implementation handles linear inequalities + equality predicates
   (covers the SNAP eligibility table example and most agency tables).
   Cases the analyzer cannot prove disjoint warn rather than error.
   Document this limitation in the rule's module docstring.

3. **FEL parser availability.** The rule depends on `fel-core::parse` to
   walk input-cell ASTs. Confirm the parser supports the FEL subset used
   in cell predicates (boolean operators, numeric comparisons, equality
   on enums). If gaps exist, either restrict K-052 to the subset OR file
   FEL parser issues.

4. **Path-shape stability for diagnostic messages — DECIDED.** Slash format
   chosen (matches K-049). Update doc-string in `diagnostic.rs:104` and
   ensure fixtures' `expected_errors` substrings contain the slash form.
   Document the decision so future rules don't drift back to dot or
   `$.` formats.

5. **Untagged Guard enum serde edge case.** `#[serde(untagged)]` is the
   right shape for `oneOf [string, DecisionTableGuard]` but can produce
   confusing error messages on type mismatch. Add a unit test specifically
   for the polymorphic deserialization (string-form vs object-form vs
   malformed-object-form) so failure modes are predictable.

## Landed state at the time of this plan

Branch `claude/wos-studio-setup-zFFDC` HEAD includes the per-kind body
schema enforcement (commit `0acfe09`), the audit fixes (`0cc4928`), Stage 3
review-remediation Waves 1-3 (`1934032`, `e81a1bf`, `30b913f`), and Wave 4
fixture cleanup (this plan). The 10 K-05[123] conformance fixtures are
landed at their final names (`K-051-resolved-decision-table-guard.json`,
`K-052-disjoint-unique-rows.json`, `K-053-boolean-output-column.json` for
positive cases — bare-name convention matching K-001/K-033/etc.; negative
cases retain the `-negative-` prefix). Witness-coupling phrasing decoupled.
After Rust impl ships locally (Waves 0/1'/2'), a follow-on commit closes
Stage 4 first cut.
