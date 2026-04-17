# K-012 / K-017 Load-Bearing Audit — 2026-04-17

## Methodology

Per [open-questions Q4, 2026-04-17](../archive/reviews/2026-04-16-architecture-review-open-questions.md#q4-which-rules-today-are-load-bearing-on-the-graduation-ladder)
and [rule-coverage plan Step 2.7](../plans/2026-04-16-wos-rule-coverage-conformance.md),
promotion to `LoadBearing` requires all four parts of the mechanical test:

1. Normative `spec_ref` citing a canonical spec `§`.
2. Imperative `suggested_fix`.
3. `fixtures.len() >= 1`.
4. Removing the rule from the active set causes at least one conformance test to fail.

The solutions architect's challenge for K-012 and K-017 sharpens part 4 into a question:
**"name the fixture that breaks when this rule is disabled."** A rule earns
`LoadBearing` only if an existing, ship-as-part-of-the-spec fixture would
silently validate without it. Per plan Step 2.7, we MUST NOT fabricate a
fixture whose only purpose is to justify promotion — that defeats the
pre-1.0 ratchet.

Scope: `wos-spec/` only. Read-only audit — no lint code changes, no fixture
changes. Evidence is drawn from:

- Rule implementations: `crates/wos-lint/src/rules/fel_analysis.rs`
- Rule inline unit tests: same file, `#[cfg(test)]` module
- Cross-project tier-2 tests: `crates/wos-lint/tests/tier2_rules.rs`
- All fixtures: `fixtures/{kernel,governance,ai,advanced,sidecars,companions,profiles,validation}/`
- Normative anchor: `specs/kernel/spec.md` §5.5

---

## K-012 — Guards on transitions MUST be valid FEL

### Rule behavior

`check_guard_expression` in
[`crates/wos-lint/src/rules/fel_analysis.rs:166-184`](../../crates/wos-lint/src/rules/fel_analysis.rs)
calls `fel_core::parse(guard)` on every `transitions[].guard` string (recursing
through compound substates and parallel regions) and, on parse failure, emits
a `K-012` **error** with the parser's message. No other K-012 semantics — it
is strictly a syntax gate for FEL strings at guard positions.

Spec anchor: Kernel §4.6 (guards are FEL expressions).

### Existing fixtures exercising it

Every fixture containing a `"guard"` key (see `grep -rn '"guard"' fixtures/`):

- `fixtures/kernel/purchase-order-approval.json:46,53` — `caseFile.amount <= 50000` / `> 50000`
- `fixtures/kernel/purchase-order-provenance.json:37` — `caseFile.amount <= 50000`
- `fixtures/kernel/benefits-adjudication.json:71,77,129,211,217` — various `caseFile.*` comparisons
- `fixtures/kernel/medicaid-redetermination.json:61` — `caseFile.application.isComplete = true`
- `fixtures/validation/procurement-approval-llm-test.json:76` — a binary-AND FEL guard

All are syntactically valid FEL. K-012 fires zero diagnostics against the
entire ship-as-part-of-the-spec fixture set.

Inline unit coverage exists in `fel_analysis.rs:919-940` (`k012_invalid_guard_emits_error`),
but the input is a `json!` literal constructed in-test, not a tracked fixture file.

### Fixture that breaks when disabled

**None found.** No file under `fixtures/` contains a malformed FEL guard
string that K-012 would reject. `fixtures/kernel/invalid-documents.json`
holds the only "invalid kernel document" collection, and every entry there
targets schema-level errors (missing `lifecycle`, wrong `$wosKernel`
version, bad `impactLevel` enum, etc.) — not FEL syntax errors inside
guards.

### Decision

`Stable` — hold.

### Reasoning

K-012 is a real structural gate at the spec level — Kernel §4.6 requires
guards to be FEL, and a kernel whose guard fails to parse is unevaluable by
construction. But the promotion test is deliberately narrower than
"structural": it asks whether the shipped fixture set today would silently
pass without the rule. It would not catch anything because no shipped
fixture carries an invalid guard. Pre-1.0, the honest answer is that K-012
protects against a regression class for which we have no repro in the fixture
tree. Per plan Step 2.7, we MUST NOT add one just to justify promotion.

**Proposed (but not authored, per guidance):** a minimal negative-kernel
fixture like `fixtures/kernel/invalid-guard-fel.json` whose single
transition carries `guard: ">>> not fel <<<"` would make K-012 observably
load-bearing. The right place to author that is when the first conformance
fixture legitimately exercises a negative-K-012 expectation — not as a
justification-only file. Revisit at that point.

---

## K-017 — FEL guards MUST NOT reference related case state

### Rule behavior

`check_no_related_case_refs` in
[`crates/wos-lint/src/rules/fel_analysis.rs:353-418`](../../crates/wos-lint/src/rules/fel_analysis.rs)
walks the parsed AST of each transition guard and emits `K-017` **error** for
any `FieldRef`, `ContextRef`, or bare-`$` `PostfixAccess` whose leading
identifier matches `relatedCase` / `relatedCases` (or a dotted prefix of
those). It fires only when K-012 has already succeeded (unparseable guards
short-circuit before K-017 runs).

Spec anchor: Kernel §5.5 (`specs/kernel/spec.md:320`):

> "A conformant processor MUST NOT evaluate FEL guard expressions that
> reference data in a related case's case state -- cross-case guards would
> break the deterministic evaluation algorithm (Kernel S4.2) because the
> related case's state is not under this instance's control."

The MUST NOT is normative and structural — violating it breaks K-011
determinism guarantees. This is stronger than "ergonomic."

### Existing fixtures exercising it

`grep -rn 'relatedCase' fixtures/` returns **zero matches.** Including
`fixtures/kernel/case-relationship-appeal.json`, the only kernel fixture
that models cross-case interaction — it uses `caseFile.relationships[]`
metadata plus `correlationKey` / `emitEvent` for cross-case signals
(precisely the pattern the spec prescribes), never a `$relatedCase.*` guard.
The related-case appeal workflow is deliberately written the right way,
which is great for positive conformance but leaves K-017 with no negative
fixture.

Inline unit coverage exists (`k017_guard_with_related_case_ref`,
`fel_analysis.rs:965-991`) using a `json!` literal, but again not a
tracked fixture file.

### Fixture that breaks when disabled

**None found.** Disabling K-017 today does not cause any fixture-backed test
to fail.

### Decision

`Stable` — hold.

### Reasoning

K-017 protects a strictly-structural invariant (cross-case determinism,
Kernel §5.5 / §S4.2), not an ergonomic concern — the solutions architect's
challenge "some FEL guards are ergonomic rather than structural" applies
less cleanly here than to K-012. But the mechanical promotion test is
outcome-based, not motivation-based. Today's `case-relationship-appeal.json`
fixture deliberately models cross-case interaction the *correct* way, so
removing K-017 produces no observable regression in the conformance suite.
Pre-1.0, without an offending fixture, the rule stays at `Stable`.

**Proposed (but not authored, per guidance):** when someone later adds an
"anti-pattern" companion to `case-relationship-appeal.json` — e.g.,
`fixtures/kernel/invalid-cross-case-guard.json` whose appeal workflow carries
`guard: "$relatedCase.status = 'closed'"` on one transition — K-017 becomes
observably load-bearing and should be promoted at that point. Until then,
the honest ladder state is `Stable`.

---

## Summary

| Rule  | Decision | Fixture that breaks when disabled |
|-------|----------|-----------------------------------|
| K-012 | `Stable` | None found. No shipped fixture carries an invalid-FEL guard. |
| K-017 | `Stable` | None found. `case-relationship-appeal.json` deliberately uses the correct `correlationKey` pattern; no fixture carries a `$relatedCase.*` guard. |

Both rules remain correct, tested by inline unit tests, and aligned with
their normative spec anchors. Neither clears the mechanical promotion test
against the current fixture tree. Per open-questions Q4 and plan Step 2.7,
this is the right outcome: the ratchet holds pre-1.0, and promotion is
available the moment a legitimate negative fixture enters the tree for an
independent reason.

Follow-up (not in scope of this audit): when the rule-coverage plan lands,
seed K-012 and K-017 in the rule registry at `Graduation::Stable` with their
`spec_ref` (`kernel/spec.md#§4.6` and `kernel/spec.md#§5.5` respectively) and
leave `fixtures: &[]`. Revisit if/when a negative fixture appears.
