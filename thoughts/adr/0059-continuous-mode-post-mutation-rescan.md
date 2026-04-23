# ADR-0059: Continuous-mode post-mutation re-scan driver

**Status:** Proposed
**Date:** 2026-04-20
**Deciders:** Formspec Working Group
**Author:** Mike (TealWolf Consulting LLC)
**Supersedes:** None
**Related:**

- K-049 lint rule (`crates/wos-lint/src/rules/continuous_mode.rs`)
- TODO.md §4.3a #F3b (this ADR scopes that item)
- WOS Runtime Companion §10.3 (`specs/companions/runtime.md:510-524`)
- ADR-0057 §"Streaming evaluation mode" — where continuous mode entered the spec
- #F5a (kernel `ProvenanceOutcome` enum + `outcome` property on `FactsTierRecord`) — **LANDED** 2026-04-20 in commit `2d890d3` (`feat(kernel): ProvenanceOutcome $def + optional outcome on FactsTierRecord (§4.3a #F5a)`)

---

## 1. Context

### 1.1 The drift

The WOS Runtime Companion §10.3 (`specs/companions/runtime.md:510-524`) is normative and unambiguous:

> "In `continuous` mode, after any case state mutation -- whether from a `setData` action, a contract validation result, or an external signal -- the processor re-evaluates all guards in the current configuration. If any guard that was previously `false` now evaluates to `true`, the corresponding transition fires."

The current runtime at `crates/wos-core/src/eval.rs:412-421` does not do this. It re-fires only transitions whose `event` is literally the string `"$continuous"`:

```rust
// crates/wos-core/src/eval.rs:412-421
/// Try to fire a `$continuous` transition in the current configuration.
pub fn try_fire_guardless_transition(&mut self) -> Result<bool, EvalError> {
    self.try_fire_transition("$continuous", None, None)
}
```

`"$continuous"` is an ad-hoc sentinel — the spec never reserves it alongside `$timeout.*`, `$related.*`, or the other `$`-prefixed kernel-generated events enumerated in Kernel S4.10. Authors must know to write `"event": "$continuous"` on every transition intended to re-fire on mutation, which:

1. Inverts §10.3's promise (the spec says "all guards re-evaluate"; the runtime says "only guards on transitions opted in via a magic string re-evaluate").
2. Is unreachable through authoring docs because the sentinel is documented only in `eval_mode.rs` and `try_fire_guardless_transition`'s rustdoc — never in `specs/**`.
3. Makes the newly-landed K-049 lint (`crates/wos-lint/src/rules/continuous_mode.rs:3-15`) structurally misleading: K-049 warns on `setData → guard` cycles that §10.3 prescribes the runtime will exhibit, but under today's runtime the warned-about cycle shape is not actually reachable unless authors also opt transitions in with `"event": "$continuous"`.

The 2026-04-20 semi-formal review caught this in the K-049 landing PR. The wos-expert consultation of the same date recommended option (c): fix the runtime rather than fix the lint.

### 1.2 Why nothing else caught it

No conformance fixture exercises §10.3 in the spec-faithful shape — every continuous-mode fixture on disk uses the `"$continuous"` sentinel because that is the only shape that currently fires. Tests are therefore green against the wrong contract. `eval_mode.rs::continuous_reevaluate` at `crates/wos-core/src/eval_mode.rs:55-125` faithfully implements the cycle-cap side of §10.3 (100-cycle `CONVERGENCE_CAP`, `convergenceCapReached` provenance emission via the existing `ProvenanceKind::ConvergenceCapReached` variant at `provenance.rs:70`), but its "try to fire any newly-enabled transition" inner loop delegates to `try_fire_guardless_transition` — inheriting the sentinel-only scan.

This has been drift for the full life of the runtime. WOS is pre-1.0 and pre-adoption; the cost of fixing it will never be lower than now.

### 1.3 Evidence

- **Spec:** `specs/companions/runtime.md:510-524` (§10.3 Continuous Mode — "after any case state mutation [...] the processor re-evaluates all guards in the current configuration").
- **Runtime under-implementation:** `crates/wos-core/src/eval.rs:412-421` (`try_fire_guardless_transition` hardcodes `"$continuous"`).
- **Lint rule that newly depends on the spec-faithful runtime:** `crates/wos-lint/src/rules/continuous_mode.rs:3-15` (K-049 docstring describes the cycle shape §10.3 would produce; today's runtime never produces it without the sentinel).
- **Pre-condition infra already in place:** `crates/wos-core/src/provenance.rs:67-70` — `ProvenanceKind::ConvergenceCapReached` variant already exists. `crates/wos-core/src/eval_mode.rs:20` — `CONVERGENCE_CAP = 100` already exists. `crates/wos-core/src/eval_mode.rs:75-106` — cap-hit provenance emission already exists. The only missing piece is what `try_fire_guardless_transition` actually scans.

---

## 2. Options considered

### Option A — Fix K-049 and fixtures to use `$continuous`-event transitions; leave the runtime as-is

**Pros:** Cheapest. Hours, not days. No ADR needed.
**Cons:** Ratifies the spec↔runtime divergence on a greenfield project where the cost of fixing the runtime is at an all-time low. Every future continuous-mode author must learn an undocumented sentinel convention. K-049's diagnostic message becomes implementation-accurate but spec-blind; any reader who opens Runtime §10.3 and tries to use continuous mode without the sentinel will get silent no-ops.

**REJECTED.** Locking in a §10.3 divergence before there is a single adopter is the textbook anti-pattern CLAUDE.md warns against: "Delete, don't preserve — do not work around problems."

### Option B — Reword K-049's diagnostic to be spec-faithful, defer the runtime fix

**Pros:** Immediately stops the bleeding. K-049's message stays honest with respect to the spec. Preserves implementation flexibility for the runtime fix.
**Cons:** K-049 then warns about a cycle shape the processor never actually exhibits (unless authors also use the sentinel). First adopters of continuous mode get a warning about a risk that isn't really there — a "boy who cried wolf" problem that erodes trust in the lint.

**TEMPORARY; NOT a final decision.** This is exactly what #F3a does as the immediate patch — preserved as "stop the bleeding" while F3b lands. F3a is correct as a stopgap; it is incorrect as an end state. F3b supersedes F3a.

### Option C — Rewrite `try_fire_guardless_transition` as a post-mutation re-scan; match §10.3 exactly

**Pros:**

- Closes the drift. The runtime conforms to its normative spec for the first time.
- K-049 becomes load-bearing — cycle warnings correspond to real runtime risk.
- Unblocks §10.3 conformance fixtures (currently none exist in the spec-faithful shape).
- Straightforward to scope; the fan-out is bounded to `eval.rs` + tests + fixture migration.

**Cons:**

- ~300 LOC of new runtime code plus test coverage. 3-5 engineer-days.
- Invalidates the `"$continuous"` sentinel for its current use. Back-compat shim is easy; the sentinel was never a documented public contract.
- Creates an intentional coupling with #20 (typed events) because the re-scan must decide how `condition`-kind events interact with guard-driven re-firing. This is not additional work — #20 must answer this question anyway; doing it alongside F3b just forces the decision to surface.

**SELECTED.** This is the wos-expert recommended option from the 2026-04-20 consultation.

---

## 3. Decision

Adopt Option C. Rewrite `try_fire_guardless_transition` (and rename it to `rescan_on_mutation` to match what it actually does) as a two-phase re-scan that matches Runtime §10.3 exactly.

### 3.1 Phase 1 — Collect candidates

After any mutation, walk the active configuration and collect every transition whose event satisfies any of:

1. **No event declared** (guard-only transitions). This is the default §10.3 shape: "after any case state mutation [...] re-evaluates all guards". A transition with no event and a guard over case state is exactly the authoring surface §10.3 promises.
2. **Event is a typed `condition`-kind event** (once #20 lands). Typed events whose payload carries a guard-like predicate are, semantically, guard-only transitions dressed up with event metadata — they participate in the same re-scan.
3. **Event is literally `"$continuous"`** — preserved as a no-op backwards-compat alias for the one release cycle after this ADR lands. Intended for fixture migration only; deprecation warning ships alongside.

### 3.2 Phase 2 — Fire in document order until stable or capped

Evaluate each candidate's guard in document order; fire the first whose guard now evaluates true. Re-enter Phase 1 after firing (because firing mutates state, which may enable further transitions). Stop when:

- No candidate fires → stable configuration, return normally.
- `CONVERGENCE_CAP` (100 cycles) is reached → emit a `ProvenanceKind::ConvergenceCapReached` record with `data.cyclesUsed: 100`, `data.triggeringMutation: <path-or-event>`, and return.

The convergence-cap side of this loop is already implemented at `crates/wos-core/src/eval_mode.rs:75-106`. The rewrite replaces only what the inner `try_fire_guardless_transition` call scans.

### 3.3 Preconditions for F3b execution

Two items must be settled before F3b code lands. Both are now resolved:

1. **#F5a (kernel `ProvenanceOutcome` enum + `outcome` property on `FactsTierRecord`) — LANDED.** Shipped in commit `2d890d3` (2026-04-20). The `convergenceCapReached` provenance record that F3b emits now has a schema home: `FactsTierRecord.outcome` typed by the kernel-level `ProvenanceOutcome` enum. F3b writes the string `"convergenceCapReached"` into `outcome` rather than carrying it in the `data` map. The `ProvenanceKind::ConvergenceCapReached` Rust variant already exists at `provenance.rs:70`. All schema plumbing is in place.
2. **`$continuous` sentinel fate.** Keep as a one-release-cycle no-op alias for unmigrated fixtures; K-049 grows a deprecation warning; removed in the following release. This is the decision — not an open question.

---

## 4. Migration

### 4.1 Fixture migration

Any conformance fixture using `"event": "$continuous"` as the authoring opt-in for continuous-mode re-evaluation migrates to either:

- A guard-only transition (no `event` property), or
- A typed `condition`-kind event (once #20 lands — if F3b ships first, guard-only is the only migration target).

`grep -r '"\$continuous"' crates/wos-conformance/tests/fixtures/ fixtures/` before F3b commit 1 to enumerate candidates. TODO currently estimates 0–1 such fixtures.

### 4.2 Back-compat shim

Preserve `"$continuous"` as a no-op event alias for one release cycle. K-049 grows a `draft`-severity lint warning: `"event: \"$continuous\" is deprecated; use a guard-only transition (Runtime §10.3)"`. The shim ships with F3b commit 2 and is removed in the release after.

### 4.3 Runtime test coverage

Add at least three new unit tests in `crates/wos-core/src/eval.rs` tests module:

- **Test A:** A guard on a transition without `"$continuous"` (guard-only) that becomes true only after a `setData` fires correctly. This is the spec-faithful §10.3 shape that today's runtime cannot satisfy.
- **Test B:** Convergence cap triggers on a genuine non-sentinel cycle (two guard-only transitions whose `setData` actions re-enable each other). Pair with a K-049 lint fixture so lint and runtime agree on the cycle shape.
- **Test C:** A mutation to a path that no guard reads does NOT enter the re-scan loop at all (must be a fast path — the common case must not pay cycle-detection cost).

### 4.4 K-049 promotion

Post-F3b, K-049's diagnostic message returns to implementation-faithful phrasing (reverting F3a's spec-neutral rewording), and K-049 is promoted to LoadBearing in the rule registry. This requires the standard LoadBearing promotion gate: `spec_ref`, `suggested_fix`, and at least 2 load-bearing fixtures. The fixture from §4.3 Test B counts as one; one more is added alongside the promotion commit.

---

## 5. Consequences

### Positive

- WOS runtime conforms to Runtime §10.3 for the first time. The spec-implementation gap closes on the one evaluation mode where it was most visible.
- K-049 becomes load-bearing. The lint's warning corresponds 1:1 to a real runtime-reachable cycle.
- Post-F5a, convergence-cap events carry structured `outcome` data that export cleanly to PROV-O / XES / OCEL (the existing export crate machinery already handles the `data` map; only the `outcome` field discriminator is new).
- Unblocks §10.3 conformance fixtures. Today, zero fixtures exercise §10.3 in its spec-faithful shape; post-F3b, every continuous-mode fixture by default does.
- Forces #20 (typed events) to resolve how `condition`-kind events interact with re-scan, which is load-bearing architecture that will have to be decided eventually. Surfacing it now is better than surfacing it later under release pressure.

### Negative

- ~300 LOC of new runtime code plus tests. 3-5 engineer-days.
- Creates an intentional coupling with #20. If #20 is delayed, F3b lands without typed-event support in the re-scan, which is fine for the guard-only default path but leaves a known extension point open.
- `$continuous` sentinel deprecation could surprise any external runtime consumer. WOS has no such consumers (greenfield pre-1.0); if one appears mid-flight, the one-release-cycle shim is the escape hatch.

### Neutral

- LINT-MATRIX counts shift: K-049 promotes from Tested → LoadBearing. AI-058 (a different line of work in §4.3a) stays Tested. Net: +1 LoadBearing, -1 Tested; the matrix total is unchanged.
- No schema changes from F3b proper. Schema changes belong to #F5a and are scoped there.

---

## 6. Ordered task list for F3b implementation

Five tasks, each one commit. Estimates reflect the coupling with #F5a (which may land first or may not).

1. **Rename + introduce re-scan behind a feature flag.** Rename `try_fire_guardless_transition` → `rescan_on_mutation`; introduce Phase 1 / Phase 2 structure behind a `continuous-rescan` Cargo feature flag. Old behavior (sentinel-only) remains the default. No test changes.
   - Commit: `feat(wos-core): scaffold continuous-mode re-scan driver`
   - LOC: ~120 (rename + structural skeleton).

2. **Flip default.** Remove the feature flag; the re-scan path becomes the only path. Migrate any fixtures using `"$continuous"` to guard-only transitions. Back-compat shim activates.
   - Commit: `feat(wos-core): continuous-mode re-scan is the default (§10.3)`
   - LOC: ~60 (flag removal, shim, fixture migration).

3. **Convergence-cap provenance through `outcome`.** Migrate the existing `data`-map `convergenceCapReached` emission (`eval_mode.rs:85-89`) to the landed #F5a `outcome` field. Wire through the kernel `ProvenanceOutcome` enum from `schemas/kernel/wos-provenance-record.schema.json`.
   - Commit: `feat(wos-core): ConvergenceCapReached provenance via outcome field (§10.3)`
   - LOC: ~40 (provenance wiring + one fixture).

4. **Tests.** Add Tests A / B / C from §4.3 above.
   - Commit: `test(wos-core): continuous-mode §10.3 conformance`
   - LOC: ~80 (three unit tests + shared test helpers).

5. **K-049 message reset + promotion.** Revert F3a's spec-neutral wording to implementation-faithful phrasing now that the runtime matches. Add the second load-bearing fixture. Update the rule registry with the LoadBearing promotion.
   - Commit: `fix(wos-lint): K-049 back to implementation-faithful + promote to LoadBearing`
   - LOC: ~30 (lint message + fixture + registry entry).

**Total estimate: 3-5 engineer-days, ~330 LOC.**

---

## 7. Alternatives not taken

### Make continuous mode opt-in per-transition only (status quo + sentinel)

Rejected. Runtime §10.3 is normative, not optional. Codifying the sentinel would require amending §10.3 to say "after any mutation, re-evaluate guards on transitions whose event is `$continuous`" — which defeats the entire purpose of continuous mode (namely, that authors shouldn't have to opt in to guard re-firing; that's what `event-driven` mode is for).

### Rewrite the entire evaluator to be fully reactive (Harel-statechart-first, BPMN-less)

Rejected as out of scope. The evaluator is already statechart-first (see `eval.rs` parallel-region handling at lines 434-449 and the `Configuration` type). Only the continuous re-scan driver is broken. A full evaluator rewrite is a much larger ADR; this one solves the narrow drift.

### Extend K-049 to cover the `$continuous`-event path and leave the runtime wrong

Rejected for the same reason as Option A. Ratifies a divergence from §10.3 that has zero adopters to break and zero reason to preserve.

---

## 8. References

- WOS Runtime Companion §10.3 — `specs/companions/runtime.md:510-524` (normative re-scan requirement)
- K-049 lint rule — `crates/wos-lint/src/rules/continuous_mode.rs:3-15` (docstring describes the cycle shape §10.3 produces)
- Current runtime under-implementation — `crates/wos-core/src/eval.rs:412-421`
- Convergence-cap machinery — `crates/wos-core/src/eval_mode.rs:20, 75-106` (cap constant + provenance emission; already correct)
- `ProvenanceKind::ConvergenceCapReached` variant — `crates/wos-core/src/provenance.rs:67-70` (Rust-side variant already exists; schema-side is #F5a)
- ADR-0057 — `thoughts/archive/adr/0057-wos-core-implementation-boundary.md` §"Streaming evaluation mode" (the original decision that introduced continuous mode)
- TODO §4.3a #F3b — `TODO.md:125` (the item this ADR scopes)
- TODO §4.3a #F5a — `TODO.md:127` (prerequisite, landed 2026-04-20 in `2d890d3`)
- 2026-04-20 wos-expert consultation — recommended Option C (embedded in §4.3a item descriptions)
