# WOS Trace-Emitting Conformance — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `wos-conformance` emits structured execution traces, not pass/fail. An LLM that generated a workflow gets back: "at step 4 the expected next state was `review`; actual was `rejected` because guard `G-02` evaluated false when policy `P-11` applied." That delta is learnable; a red X is not.

**Architecture:** A `ConformanceTrace` type capturing expected vs. actual per step, plus the reason (guard/policy/event reference). Runtime tests emit traces into a new artifact directory on both pass and fail. A new CLI subcommand serves traces for consumption by authoring tools.

**Tech Stack:** Rust (`wos-conformance`, `wos-runtime`), `serde` for JSON, existing fixture infrastructure.

**Spec anchor:** [architecture-review-handoff.md §5.3](../archive/reviews/2026-04-16-architecture-review-handoff.md).

---

## Prerequisites

- [§4.2 rule-coverage plan](./2026-04-16-wos-rule-coverage-conformance.md) — trace emissions link back to rule IDs.
- [§5.2 structured diagnostics plan](./2026-04-16-wos-structured-lint-diagnostics.md) — traces reuse the source-location and diagnostic patterns.
- Existing runtime + conformance crates.

## Completion criteria

1. `ConformanceTrace` type with step-by-step state transitions, expected-vs-actual deltas, and cause references.
2. Every T3 conformance test emits a trace to `target/conformance-traces/<fixture-slug>.json`.
3. `wos-conformance explain <fixture>` prints a prose rendering of the trace.
4. `wos-conformance diff <actual-trace> <expected-trace>` prints a minimal delta suitable for LLM consumption.
5. Schema published for the trace type.

## File structure

- **Create:** `crates/wos-conformance/src/trace.rs` — trace type and emission.
- **Create:** `schemas/conformance/conformance-trace.schema.json` — public schema.
- **Modify:** `crates/wos-conformance/src/runner.rs` — emit traces regardless of pass/fail.
- **Create:** `crates/wos-conformance/src/commands/{explain,diff}.rs` — CLI subcommands.
- **Create:** `fixtures/conformance/expected-traces/` — committed golden traces for each fixture.

---

## Task 1: Define the trace type

**Files:**
- Create: `crates/wos-conformance/src/trace.rs`

- [ ] **Step 1.1:** Struct definitions:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConformanceTrace {
    pub fixture_id: String,
    pub kernel_version: String,
    pub steps: Vec<TraceStep>,
    pub outcome: Outcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceStep {
    pub step_index: u32,
    pub event: Event,
    pub state_before: String,
    pub state_after: String,
    pub expected_state_after: Option<String>,
    pub guards_evaluated: Vec<GuardEvaluation>,
    pub policies_applied: Vec<PolicyApplication>,
    pub delta: Option<Delta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardEvaluation {
    pub guard_id: String,
    pub expression: String,
    pub result: bool,
    pub inputs: serde_json::Value,
}

pub struct Delta { /* expected vs actual summary */ }
```

- [ ] **Step 1.2:** Commit. `feat: ConformanceTrace type for structured T3 output`.

## Task 2: Emit traces from the runner

**Files:**
- Modify: `crates/wos-conformance/src/runner.rs`

- [ ] **Step 2.1:** Every test run instruments the runtime to capture `TraceStep` entries. Serialize to `target/conformance-traces/<fixture>.json` at the end of each run, whether the test passed or failed.

- [ ] **Step 2.2:** The runner writes a diff artifact when `actual.steps != expected.steps`. `target/conformance-traces/<fixture>.diff.json`.

- [ ] **Step 2.3:** Commit. `feat: conformance runner emits structured traces per fixture`.

## Task 3: Commit expected traces

**Files:**
- Create: `fixtures/conformance/expected-traces/<fixture-slug>.json` for each T3 fixture.

- [ ] **Step 3.1:** For each fixture currently exercised by `wos-conformance`, capture a trace from a known-good run and commit it as the expected baseline.

- [ ] **Step 3.2:** Add a regression test: actual trace must equal expected trace (modulo timestamps). If a spec change legitimately alters a trace, the commit must update both the code and the expected trace.

- [ ] **Step 3.3:** Commit. `test: commit expected conformance traces for T3 fixtures`.

## Task 4: `explain` and `diff` CLI subcommands

**Files:**
- Create: `crates/wos-conformance/src/commands/explain.rs`
- Create: `crates/wos-conformance/src/commands/diff.rs`

- [ ] **Step 4.1:** `wos-conformance explain <fixture>`:

```
Fixture: benefits-adjudication (kernel 1.0)
  step 1: initial → application-received (event: application.submitted)
  step 2: application-received → review (event: clerk.claimed) ✓
  step 3: review → approved (event: approver.decide)
    ✗ expected: approved
      actual:   rejected
      reason:   guard G-02 evaluated false
                expression: `case.data.benefit_amount <= case.data.income_limit`
                inputs:     { benefit_amount: 520, income_limit: 500 }
                policy applied: P-11 (income-threshold v2)
```

- [ ] **Step 4.2:** `wos-conformance diff <actual.json> <expected.json>`:

```json
{
  "differs_at_step": 3,
  "expected_state": "approved",
  "actual_state":   "rejected",
  "cause": {
    "kind": "guard-false",
    "guard_id": "G-02",
    "policy_id": "P-11",
    "inputs": { "benefit_amount": 520, "income_limit": 500 }
  },
  "suggested_hypothesis": "benefit_amount exceeds income_limit under policy v2; workflow may need pre-review eligibility screen"
}
```

- [ ] **Step 4.3:** Commit. `feat: wos-conformance explain and diff commands`.

## Task 5: Publish the trace schema

**Files:**
- Create: `schemas/conformance/conformance-trace.schema.json`

- [ ] **Step 5.1:** Schema for `ConformanceTrace` with `patternProperties ^x-` per §4.1.
- [ ] **Step 5.2:** schemars-derived verification test.
- [ ] **Step 5.3:** Commit. `feat: publish conformance-trace schema`.

---

## Self-review checklist

- Trace type defined and serializable (Task 1).
- Runner emits traces on every run (Task 2).
- Expected traces committed and regression-tested (Task 3).
- `explain` and `diff` give humans and LLMs a readable signal (Task 4).
- Schema published (Task 5).

## Why this matters

The handoff says: "conformance becomes a teaching signal." An LLM in a generate-lint-conformance loop needs to know *why* a test failed, not *that* it failed. Structured traces are the teaching signal. This plan is a prerequisite for [§5.4 `wos-synth`](./2026-04-16-wos-synth-crate.md) — the synthesizer cannot self-correct from pass/fail alone.

**Estimated effort:** ~2 engineer-weeks.
