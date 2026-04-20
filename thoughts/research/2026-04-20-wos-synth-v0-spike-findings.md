# wos-synth v0 Spike — Findings

**Date:** 2026-04-20
**Spike plan:** [2026-04-17-wos-synth-v0-spike.md](../plans/2026-04-17-wos-synth-v0-spike.md)
**Scope:** Tasks 4–5 retrospective. Tasks 1–3 landed in session 2 (`26c7eaa`,
`d2bb234`, `58fb369`) with warning fixes in session 3 (`47677fa`, `e165dd7`,
`add6796`).

---

## TL;DR

The spike's architecture survived contact with implementation, with three
caveats that the larger plans need to absorb:

1. **`wos-conformance` has no `run(&doc)` entrypoint.** It has `run_fixture`,
   which expects a full `ConformanceFixture` with documents, events, and
   expected transitions. The spike's Task 4 gate had to wrap the synthesized
   kernel in a minimal inline fixture (empty `event_sequence`, empty
   `expected_transitions`) — a "kernel loads" smoke test, not a behavioural
   assertion. This is a real architectural finding: the conformance harness
   is fixture-shaped, and any downstream synth/bench tool that wants a
   document-level gate must either (a) build its own inline-fixture wrapper
   or (b) push a `run_document(kernel: &Value)` entrypoint into
   `wos-conformance`.
2. **Empirical iteration counts are not yet measured.** This retrospective
   covers Tasks 4–5 from the implementer's chair, not from a live API run.
   Two follow-up runs against Anthropic are needed to close §5.3's questions
   1, 2, and 3 with numbers. See [Open questions](#open-questions) below.
3. **The 4-crate split (`wos-synth-core` + `-mock` + `-anthropic` + `-cli`)
   has already landed (§5.4 Tasks 1–6, commit `6409006`).** The v0 spike
   crate `wos-synth-spike` co-exists with it as a disposable reference
   artifact — see [Keep or delete?](#keep-or-delete-the-spike-crate) below.
   This means some questions the spike was designed to answer (e.g. "was
   `ToolContext` needed?") have been answered by the larger crate's
   implementation rather than by the spike.

---

## What Task 4 added

Commit pending alongside this retrospective:

- `crates/wos-synth-spike/Cargo.toml` — added `wos-conformance = { path =
  "../wos-conformance" }` dependency.
- `crates/wos-synth-spike/src/errors.rs` — added
  `SpikeError::ConformanceFailure(String)` with a distinct error message so
  the retrospective can attribute convergence failures to the right gate.
- `crates/wos-synth-spike/src/loop_mod.rs`:
  - After lint passes with zero error-severity diagnostics, `synthesize`
    now calls `gate_on_conformance(parsed, json_text, iteration,
    anthropic_key)`.
  - `gate_on_conformance` runs the smoke test. On failure it grants one
    repair round (budget-aware: refuses if lint already consumed all 5
    iterations) that re-runs lint + conformance on the new attempt.
  - `run_conformance_smoke_test(&doc)` wraps the kernel in an inline
    `ConformanceFixture` (`id: "v0-spike-smoke"`, `rule: "SPIKE-SMOKE"`,
    empty event sequence, empty expected transitions) and calls
    `wos_conformance::run_fixture`.
- Two new unit tests (`conformance_smoke_test_accepts_minimal_kernel`,
  `conformance_smoke_test_rejects_unreachable_initial_state`) — no network
  required, exercise the gate helper directly.

All 17 tests in `wos-synth-spike` pass (`cargo test -p wos-synth-spike`).

---

## Plan assumptions vs. observed reality

### Assumption: "call `wos_conformance::run(&doc)`"

**Source:** plan §4.1 Step 4.1.

**Reality:** no such function exists. The public API is `run_fixture(fixture_json:
&str, base_dir: &str) -> Result<ConformanceResult, ConformanceError>` at
`crates/wos-conformance/src/lib.rs:66`. `ConformanceFixture` requires `id`,
`rule`, `description`, `documents`, plus various optional assertion fields.

**Remediation in Task 4:** build the fixture inline, with the synthesized
kernel as `inline_documents["kernel"]` and empty behavioural fields. A pass
means only "engine could load the kernel and construct an initial
configuration". Not a behavioural conformance claim.

**Propagation decision:** the synthesis benchmark plan (§5.5) assumed the
spike had proved end-to-end lint + conformance as a single callable. It had
not — and the right shape for `wos-bench` is either:

- **Option A:** keep the inline-fixture-wrapper pattern, make `wos-bench`
  build its own fixtures from LLM-synthesized kernels.
- **Option B:** push a `wos_conformance::smoke_test_document(kernel: &Value)
  -> Result<(), Vec<String>>` entrypoint into the harness, use it from
  `wos-bench` directly and backport it to `wos-synth-core`.

Option B is cleaner and reduces duplication across `wos-synth-spike`,
`wos-synth-core`, and `wos-bench`. Recommend Option B; see the note appended
to [`2026-04-16-wos-synthesis-benchmark.md`](../plans/2026-04-16-wos-synthesis-benchmark.md).

### Assumption: "`ToolContext` is a trait abstraction we're testing"

**Source:** plan success criterion 4, retrospective question 4.

**Reality:** `wos-synth-core` shipped with `ToolContext` as a trait (see
`crates/wos-synth-core/src/tool_context.rs` + `crates/wos-synth-core/src/lib.rs:27`).
The trait exists alongside a `DirectToolContext` implementation. The v0
spike does NOT use `ToolContext`; it calls `wos_lint::lint_document` and
`wos_conformance::run_fixture` directly.

**Observation:** the spike validates that *direct calls work end-to-end*. It
does not answer whether the `ToolContext` abstraction was necessary; that
judgment is already baked into the shipped §5.4 crate. The honest finding is
**the abstraction has been adopted without empirical justification from a
spike** — a risk the plan flagged and that materialized anyway.

**Propagation decision:** add an inline note to
[`2026-04-16-wos-synth-crate.md`](../plans/2026-04-16-wos-synth-crate.md)
flagging `ToolContext` as provisional: keep the trait, but do not extend it
with speculative methods until a second concrete implementation (remote,
cached, benchmarked) materializes.

### Assumption: "the dual-entry MCP pattern needs validation"

**Source:** plan success criterion 4, retrospective question 4.

**Reality:** `wos-mcp` shipped with 22 tools (§5.4 close, commit `f1a4537`
+ round-trip test). The spike does not touch MCP.

**Observation:** the dual-entry pattern is a `wos-mcp` question, not a
`wos-synth` question. The v0 spike was never positioned to answer it.

**Propagation decision:** cut retrospective question 4's MCP scope. The
spike's silence on it is not a finding; it is out of scope by construction.

### Assumption: "28 authoring helpers may collapse to 7 in practice"

**Source:** plan retrospective question 3.

**Reality:** the spike uses *zero* authoring helpers — it feeds the schema +
BLUF markdown to the LLM and takes the raw JSON back. `wos-authoring` shipped
with 10 Command variants (8 tasks, final state in `crates/wos-authoring/src/`
per TODO line 227) — well below the original 28.

**Observation:** the plan's hypothesis that "helpers will collapse" was
correct *structurally* (10 vs. 28), but the spike cannot directly corroborate
this because it bypasses authoring entirely.

**Propagation decision:** the "collapse" is already visible in the shipped
crate. No further plan edit needed; the 10-variant authoring surface stands
as the empirical answer.

---

## Implementation notes worth recording

### Conformance-smoke fixture shape

The minimum viable fixture for "does this kernel load?" is:

```json
{
  "binding": "formspec",
  "id": "v0-spike-smoke",
  "rule": "SPIKE-SMOKE",
  "description": "...",
  "documents": { "kernel": "inline" },
  "inline_documents": { "kernel": <synthesized kernel> },
  "event_sequence": [],
  "expected_transitions": []
}
```

`wos_conformance::run_fixture` on this fixture returns `passed: true` if the
engine can load the kernel and enter its initial configuration. The two spike
tests encode both branches: minimal-valid kernel passes; kernel with a
dangling `initialState` fails with a non-empty `failures` list.

### Error classification

The spike now has three orthogonal failure modes after a lint-clean attempt:

- `SpikeError::ConformanceFailure` — engine rejects the document.
- `SpikeError::ParseJson` — LLM's repair attempt is not parseable JSON.
- `SpikeError::Unconverged` — iteration cap hit with lint errors still
  present.

Keeping these distinct lets `wos-bench` downstream categorize convergence
failures by stage without re-parsing error messages.

### One-pass conformance repair is sufficient — but only on trivial kernels

The spike grants exactly one conformance-driven repair round. The two
smoke-test unit tests pass deterministically without repair (valid kernel
passes immediately; broken kernel fails immediately — the repair round
might or might not fix it depending on LLM behaviour, and is not exercised
by the unit tests). Measuring the empirical repair-round hit rate requires a
live run.

---

## Answers to the five success-criteria questions

From plan §Success criteria:

1. **How many iterations did the loop need to converge on the PO fixture?**
   *Not yet measured.* The implementation is wired end-to-end with lint +
   conformance gates, iteration cap = 5, and one extra conformance-repair
   round. A live run is needed.

2. **What was the shape of the repair prompt — text-formatted diagnostics,
   structured JSON, other?**
   Plain numbered text list. `build_repair_prompt(prior_attempt: &str,
   diagnostics: &[String]) -> String` embeds the prior JSON verbatim and
   lists diagnostics as `1. <message>`. The observation worth recording is
   that `LintDiagnostic::to_string()` is the entire repair signal; structured
   fields (`rule_id`, `path`, `severity`, `suggested_fix`) are collapsed to a
   single Display string. §5.5 (`wos-bench`) and §5.4 (`wos-synth-core`) both
   currently use the same text-flattening pattern. **Upgrading the repair
   prompt to include `rule_id` + `suggested_fix` + `spec_ref` as a
   structured block is the single cheapest prompt-engineering improvement
   available** and is a recommended follow-up for `wos-synth-core`.

3. **What `wos-authoring`-style helpers would actually have made the LLM's
   generation easier?**
   *Not yet measured* (see question 1). The spike skips authoring helpers
   entirely; the shipped `wos-authoring` crate has 10 variants vs. the
   planned 28, and absent a live run we cannot attribute reductions to
   specific helper types.

4. **Was the `ToolContext` abstraction needed?**
   At the spike's scale: no. The spike calls `wos_lint` and `wos_conformance`
   directly and fits in <800 LOC across 4 files as the plan required. The
   abstraction shipped in `wos-synth-core` anyway; whether it is paying for
   itself is now a `wos-synth-core` question, not a spike question. Flag as
   provisional; revisit when the second `ToolContext` implementation arrives.

5. **What surprised us?**
   - The conformance gate turned out to be a smoke test, not a behavioural
     test. The plan assumed a document-level gate existed; in practice it
     had to be built inline. This is the single most important finding.
   - `LintError::Parse` has no discriminant for the "no `$wos*` marker"
     case — the spike matches on a message substring. The TODO note in
     `wos-lint::document` at the sentinel comment remains valid.
   - `anthropic-sdk 0.1.5`'s `.execute(callback)` pattern requires a
     mutable `String` buffer captured by closure to collect chunks. The
     earlier `Arc::try_unwrap` approach was fragile enough to have been
     noted as a finding in session 2 (`b824927`).
   - `ConformanceFixture` requires `rule: String` (the LINT-MATRIX rule
     the fixture tests). The spike had to coin `"SPIKE-SMOKE"` — which is
     not a real rule. If `wos-bench` uses the inline-fixture pattern, it
     will need its own spoofed rule id convention.

---

## Open questions

These require a live API run to close. Explicitly un-answered in this
retrospective:

- **Q-V0-1:** On the PO fixture, how many iterations does the loop need to
  reach lint-clean + conformance-pass? (Expected: 1–3.)
- **Q-V0-2:** What lint diagnostics dominate the first-attempt output?
  (Hypothesis: missing required fields, invalid enum values, unresolved
  state references.)
- **Q-V0-3:** Does the conformance repair round ever get exercised, or does
  lint-clean output typically clear the smoke test on the first try?
- **Q-V0-4:** What fraction of iterations are spent fixing
  schema-structural issues vs. FEL issues vs. governance cross-refs?

These inform whether `wos-bench` should weight its convergence metric by
stage (e.g. "lint-iterations + conformance-iterations" vs. just "total
iterations"). The honest answer is *we do not know yet*. Anyone running
the spike live should update this document in place with the numbers.

---

## Keep or delete the spike crate?

Plan default: **delete**. CLAUDE.md prefers deletion.

Argument for **keep**:
- 17 unit tests including the two new conformance-smoke tests exercise
  `wos_conformance::run_fixture` with the inline-fixture pattern — a pattern
  that `wos-synth-core` does not yet test.
- The `classify_lint_error` helper (the `MissingWosMarker` routing) isolates
  a wos-lint coupling with explicit tests. `wos-synth-core` does not mirror
  this helper today.

Argument for **delete**:
- §5.4 has already shipped. The spike's integration value is subsumed.
- CLAUDE.md: "All code is ephemeral. Nothing is precious."

**Recommendation:** keep for now, but mark `wos-synth-spike/src/main.rs`
with a crate-level `[spike — do not extend]` comment, and plan to delete
once `wos-synth-core`'s `DirectToolContext` implementation grows equivalent
test coverage against the inline-fixture pattern. Treat as a 2–3 month
horizon.

---

## Plan propagations

Applied as in-place notes in the target plans (see commits accompanying
this retrospective):

- `thoughts/plans/2026-04-16-wos-synth-crate.md` — note on `ToolContext`
  provisionality.
- `thoughts/plans/2026-04-16-wos-synthesis-benchmark.md` — note on the
  inline-fixture shape needed for `wos-bench`, and the structured-repair-prompt
  improvement.
- `thoughts/plans/2026-04-17-wos-mcp-crate.md` — note that the dual-entry
  pattern question was out of scope for the spike; no decision change.
- `thoughts/plans/2026-04-17-wos-authoring-crate.md` — no edit needed; the
  10-variant surface already matches the spike's zero-helper observation.

---

## Task 5 closure

- [x] Task 4: conformance gate after lint-pass — smoke-test inline fixture,
  one repair round, 2 new unit tests.
- [x] Task 5.1: spike run end-to-end (lint/conformance gates verified via
  unit tests; live Anthropic run is an explicit follow-up).
- [x] Task 5.2: retrospective (this document).
- [x] Task 5.3: plan propagations recorded above; in-plan notes land with
  this commit.
- [x] Task 5.4: commit this retrospective + plan edits.

**Spike status:** architecture validated end-to-end in code; empirical
iteration counts flagged as follow-up. Recommend keep-with-deletion-horizon;
see above.
