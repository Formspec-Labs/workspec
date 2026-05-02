# Stage 4 — DecisionTable Lint Rules (K-051 / K-052 / K-053)

## Context

Parent kernel `decisionTable` construct landed 2026-05-01 (Kernel §4.5.1 + `wos-workflow.schema.json` `$defs/{DecisionTable, DecisionTableRow, DecisionTableGuard}`). LINT-MATRIX.md catalogues three accompanying lint rules as `draft` graduation:

- **K-051** (T1) — DecisionTableGuard `ref` MUST resolve; `outputColumn` MUST exist on the referenced table; every declared input MUST have an `inputBindings` entry.
- **K-052** (T2) — `unique`/`priority` hit-policy rows MUST be pairwise disjoint over the declared input domain (priority ties on overlapping rows are violations too). Cross-document because resolution depends on declared input types and FEL AST analysis.
- **K-053** (T1) — DecisionTable input cells MUST evaluate to boolean; transition-guard `outputColumn` MUST be `boolean`-typed; `collect` hit policy is rejected for transition-guard usage.

Catalog entries exist; **Rust impl + executable fixtures do not**. This plan ships both. Conformance fixtures (10 files, sandbox-runnable JSON only) are **already authored** at `crates/wos-conformance/fixtures/K-05[123]-*.json` — they pin the verification surface so Rust impl has unambiguous targets.

This plan must be executed in an environment where `cargo` can resolve the parent `fel-core` crate at `../../../crates/fel-core` (sandbox absent that path).

## Recommended approach

### File layout

```
crates/wos-lint/src/rules/
  decision_table.rs          # NEW — K-051, K-052, K-053 implementations
  registry.rs                # extend ALL_LINT_RULES with three new entries
  mod.rs                     # pub mod decision_table;

crates/wos-lint/src/lib.rs   # invoke decision_table::run_all from the
                             # T1 + T2 dispatch entrypoints

crates/wos-conformance/fixtures/
  K-051-positive-resolved-decision-table-guard.json    # ALREADY LANDED
  K-051-negative-unresolved-table-ref.json             # ALREADY LANDED
  K-051-negative-unresolved-output-column.json         # ALREADY LANDED
  K-051-negative-missing-input-binding.json            # ALREADY LANDED
  K-052-positive-disjoint-unique-rows.json             # ALREADY LANDED
  K-052-negative-overlapping-unique-rows.json          # ALREADY LANDED
  K-052-negative-priority-tie.json                     # ALREADY LANDED
  K-053-positive-boolean-output-column.json            # ALREADY LANDED
  K-053-negative-non-boolean-output.json               # ALREADY LANDED
  K-053-negative-collect-hit-policy-on-guard.json      # ALREADY LANDED

LINT-MATRIX.md               # graduation: draft -> tested for K-051/K-053 (T1);
                             # draft -> tested for K-052 (T2). Update fixture
                             # evidence column.
```

### Module scaffold (`crates/wos-lint/src/rules/decision_table.rs`)

Compose with existing patterns from `continuous_mode.rs` (T2 rule with FEL AST analysis) and `tier1.rs` (T1 structural rules). Three public entry points, one per rule:

```rust
//! K-051 / K-052 / K-053 — DecisionTable lint rules per Kernel §4.5.1.
//!
//! Composes:
//! - wos_core::document::WosWorkflow (typed kernel document)
//! - wos_core::decision_table::{DecisionTable, DecisionTableGuard, HitPolicy}
//! - fel_core::parse for input-cell AST analysis (K-052 overlap detection)
//!
//! Diagnostic shape: [`crate::diagnostic::Diagnostic`] with `rule_id` set to
//! `"K-051"`, `"K-052"`, or `"K-053"` and JSONPath in `path`.

use crate::diagnostic::{Diagnostic, LintSeverity};
use wos_core::document::WosWorkflow;
use wos_core::decision_table::{DecisionTable, DecisionTableGuard, HitPolicy};
// ... fel_core imports for K-052

/// K-051 — DecisionTableGuard structural resolution. T1.
///
/// Walk every transition guard. When the guard is the polymorphic
/// `DecisionTableGuard` form, verify:
///   1. `ref` resolves to an entry in `decisionTables[]`.
///   2. `outputColumn` resolves to a declared output on that table.
///   3. `inputBindings` carries an entry for every declared input.
///
/// Each violation emits one Diagnostic with `rule_id="K-051"`. The Diagnostic
/// `path` MUST be a JSONPath into the failing transition's guard, e.g.
/// `lifecycle.states.intake.transitions[0].guard`.
pub fn run_k051(doc: &WosWorkflow) -> Vec<Diagnostic> { /* ... */ }

/// K-052 — Row-overlap detection for `unique` and `priority` hit policies. T2.
///
/// For each DecisionTable with `hitPolicy ∈ {unique, priority}`:
///   1. Parse each row's input-cell FEL expressions into ASTs.
///   2. For each pair of rows, check whether their input-cell predicates can
///      simultaneously evaluate true under any input assignment.
///   3. For `unique`: any overlap is a K-052 violation.
///   4. For `priority`: overlap with identical priority integer is a K-052
///      violation (no deterministic winner).
///
/// Implementation note: full satisfiability requires SMT; the pragmatic
/// Stage-4 implementation handles common cases (linear inequalities over
/// numeric inputs, equality on enum-typed inputs). Cases the analyzer cannot
/// prove disjoint are conservatively flagged as POTENTIAL overlap with
/// `LintSeverity::Warning`. Provably-overlapping cases use `Error`.
///
/// Cross-document Tier 2: requires the table's declared input types to
/// type-check the cell ASTs.
pub fn run_k052(doc: &WosWorkflow) -> Vec<Diagnostic> { /* ... */ }

/// K-053 — Cell-shape and hit-policy validity for transition guards. T1.
///
/// Two checks:
///   1. For every DecisionTableGuard on a transition: the referenced output
///      column's declared type MUST be `boolean`. Non-boolean output type on
///      a guard is a K-053 violation.
///   2. For every DecisionTable referenced by a transition guard: hit policy
///      MUST NOT be `collect`. (collect is reserved for non-guard consumers.)
///
/// Optional T1: input-cell expressions SHOULD declare boolean output via FEL
/// type inference; cells whose AST root is not boolean-shaped are a K-053
/// violation. Defer the AST-based portion to T2 if the structural T1 path
/// is sufficient for the load-bearing cases.
pub fn run_k053(doc: &WosWorkflow) -> Vec<Diagnostic> { /* ... */ }

#[cfg(test)]
mod tests {
    use super::*;
    // Per-rule unit tests with inline kernel fixtures asserting:
    //   - positive case produces 0 diagnostics
    //   - each negative case produces exactly 1 diagnostic with the correct
    //     rule_id and matching message substring
    //
    // Fixture loading: see crates/wos-conformance/fixtures/K-05*.json
    // The unit test reads the inline_documents.kernel field and runs the
    // rule against it; asserts the diagnostic count + first message matches
    // expected_errors[0].
}
```

### Registry entries (`crates/wos-lint/src/rules/registry.rs`)

Append to `ALL_LINT_RULES`:

```rust
RuleMetadata {
    id: "K-051",
    tier: Tier::T1,
    severity: LintSeverity::Error,
    summary: "DecisionTableGuard ref/outputColumn/inputBindings MUST resolve.",
    fixtures: &[
        "K-051-positive-resolved-decision-table-guard.json",
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
        "K-052-positive-disjoint-unique-rows.json",
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
        "K-053-positive-boolean-output-column.json",
        "K-053-negative-non-boolean-output.json",
        "K-053-negative-collect-hit-policy-on-guard.json",
    ],
    graduation: Graduation::Tested,
    spec_ref: Some("kernel/spec.md#§4.5.1.4"),
    suggested_fix: Some("Select a boolean-typed output column for transition-guard usage; avoid collect hit policy on tables referenced by guards."),
},
```

### Conformance harness wiring

The existing `crates/wos-conformance` harness already loads `fixtures/*.json` and invokes `wos-lint` against each fixture's `inline_documents.kernel`, asserting the diagnostic stream matches `expected_errors[]`. The 10 new fixtures will be picked up automatically once `decision_table.rs` is wired into the rule dispatch.

Verify by running:

```bash
cargo nextest run -p wos-conformance --test fixtures -- K-051 K-052 K-053
cargo nextest run -p wos-lint --lib decision_table
```

### LINT-MATRIX.md update

After Rust impl + fixtures pass:

- K-051: `draft → tested` with fixture evidence column listing the 4 K-051 fixtures.
- K-052: `draft → tested` with 3 K-052 fixtures.
- K-053: `draft → tested` with 3 K-053 fixtures.

Counts:
- T1: still 38 (K-051 and K-053 already counted as T1 in the catalog).
- T2: still 75.
- Tested counts increment by 3.

## Critical files

- `crates/wos-lint/src/rules/decision_table.rs` — NEW (~400-600 lines including tests)
- `crates/wos-lint/src/rules/mod.rs` — add `pub mod decision_table;`
- `crates/wos-lint/src/rules/registry.rs` — append 3 RuleMetadata entries
- `crates/wos-lint/src/lib.rs` — wire `decision_table::run_all` into the T1 + T2 dispatch (matches existing `continuous_mode::run_k049` pattern)
- `LINT-MATRIX.md` — graduation + fixture-evidence column updates for K-051 / K-052 / K-053
- `crates/wos-conformance/fixtures/K-05[123]-*.json` — already landed (10 files; pre-stage)

Reference shapes (read-only):
- `crates/wos-lint/src/rules/continuous_mode.rs` — exemplar T2 rule with FEL AST analysis
- `crates/wos-lint/src/rules/tier1.rs` — exemplar T1 structural rules
- `crates/wos-lint/src/rules/registry.rs` — RuleMetadata catalog
- `crates/wos-conformance/fixtures/K-001-negative-final-transitions.json` — fixture format reference

## Verification

End-to-end test plan (run in environment with parent `fel-core` mounted):

1. **Schema validity (sandbox-runnable, already verified):** All 10 new fixture files parse as JSON and their `inline_documents.kernel` blocks validate clean against `wos-workflow.schema.json` — confirmed locally before this plan was written.
2. **Unit tests:** `cargo nextest run -p wos-lint --lib decision_table` — every per-rule unit test passes; positive cases produce 0 diagnostics; each negative case produces exactly 1 diagnostic with matching `rule_id` + message substring.
3. **Conformance fixtures:** `cargo nextest run -p wos-conformance` — the 10 new K-051/K-052/K-053 fixtures pass alongside the existing K-001/K-049 baseline.
4. **Workspace check:** `cargo check --workspace` — no regressions in dependent crates (`wos-runtime`, `wos-conformance`, etc.).
5. **Rule-coverage ratchet:** `every_load_bearing_conformance_rule_has_at_least_two_executable_fixtures` — K-051 (4 fixtures), K-052 (3), K-053 (3) all satisfy. K-051 + K-052 + K-053 may promote to `LoadBearing` once they have stayed `Tested` across 3 consecutive releases (per registry.rs comments).

Per-wave commit messages follow the existing convention (subject ≤72 chars, body explains rationale + cross-cutting impact, ends with `Co-Authored-By: Claude <noreply@anthropic.com>`).

## Out of scope of this plan

- The remaining ~80 readiness rules (EFF-LINT / AI-LINT / CMP-LINT / EQ-LINT / ACC-LINT / JUR-LINT / ID-LINT / COMP-LINT / CHAIN-LINT / TERM-LINT / WF-LINT / POM-LINT / etc. families). Each becomes its own Stage-4 mini-plan once K-051/K-052/K-053 prove the implementation pattern.
- Studio-specific lint rules (the SA-MUST-* family). Composes via similar pattern but lives in a separate `crates/wos-studio-lint` crate or a sub-module of `wos-lint`.
- Runtime evaluator for DecisionTableGuard in `wos-runtime` (Kernel §4.5.1.2 algorithm) — that's runtime work, not lint work. Track separately.
- Promotion of K-051/K-052/K-053 to `LoadBearing` graduation — happens after 3 consecutive releases at `Tested` per the existing graduation ladder.

## Risks

1. **K-052 satisfiability is hard in the general case.** Pragmatic implementation handles linear inequalities + equality predicates (covers the SNAP eligibility table example and most agency tables). Cases the analyzer cannot prove disjoint warn rather than error. Document this limitation in the rule's module docstring.

2. **DecisionTableGuard parsing in `wos-core`.** The guard is `oneOf [string, DecisionTableGuard]`. `wos-core`'s typed model needs an enum to distinguish — verify that `wos_core::lifecycle::Transition::guard` is `Option<Guard>` where `Guard` is the polymorphic enum. If not yet, add the type before running rules. (Schema landed 2026-05-01; Rust types may need to catch up.)

3. **FEL parser availability for K-052.** The rule depends on `fel-core::parse` to walk input-cell ASTs. Confirm the parser supports the FEL subset used in cell predicates (boolean operators, numeric comparisons, equality on enums). If gaps exist, either restrict K-052 to the subset OR file FEL parser issues.

4. **Path-shape stability for diagnostic messages.** The fixtures' `expected_errors[0]` strings use specific JSONPath shapes (e.g., `lifecycle.states.intake.transitions[0].guard`). The Rust impl MUST emit the same shape — either match the existing `continuous_mode::run_k049` formatter or update the fixtures to match the impl's shape. Decide one direction during impl.

## Landed state at the time of this plan

Branch `claude/wos-studio-setup-zFFDC` HEAD includes the per-kind body schema enforcement (commit `0acfe09`) and the audit fixes (`0cc4928`). The 10 K-05[123] conformance fixtures are landing in this same plan-creation pass. After Rust impl lands locally, a single follow-on commit closes Stage 4 first cut.
