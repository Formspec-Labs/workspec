# WOS Synthesis Benchmark — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pair every fixture in `wos-spec/fixtures/` with a natural-language problem statement. The set becomes a workflow-synthesis benchmark analogous to SWE-bench: given requirement R, did the LLM + WOS toolchain produce a conformant workflow? Track the rate monthly.

**Architecture:** A separate `crates/wos-bench/` crate that depends on [`wos-synth`](./2026-04-16-wos-synth-crate.md) as a library. `wos-bench` owns fixture sets, scoring, regression tracking, and result artifacts. `wos-synth` owns the provider trait, prompt templates, trace types, and outcome enum. The two-crate split (resolved in [open questions Q6](../reviews/2026-04-16-architecture-review-open-questions.md#q6-is-wos-synth-54-and-the-authoring-benchmark-55-one-project-or-two)) keeps the authoring demo separate from the measurement infrastructure; shared primitives prevent drift.

Problem statements live in `benchmarks/problems/*.md`; per-run outputs in `benchmarks/runs/<date>-<provider>-<model>/results.json`; a `BENCHMARK.md` leaderboard summarizes results over time.

**Tech Stack:** Rust (new `crates/wos-bench/` crate depending on `wos-synth --features synth`), markdown for problem statements, JSON for results.

**Spec anchor:** [architecture-review-handoff.md §5.5](../archive/reviews/2026-04-16-architecture-review-handoff.md) — fixture corpus doubles as synthesis benchmark. Two-crate split resolved in [open questions Q6](../reviews/2026-04-16-architecture-review-open-questions.md#q6-is-wos-synth-54-and-the-authoring-benchmark-55-one-project-or-two). Claim A first-class status resolved in [open questions Q1](../reviews/2026-04-16-architecture-review-open-questions.md#q1-is-claim-a-llm-authoring-an-accepted-first-class-goal).

---

## Prerequisites

- [§5.4 `wos-synth` crate](./2026-04-16-wos-synth-crate.md) landed — the benchmark is the harness for that crate.
- [§5.3 trace-emitting conformance](./2026-04-16-wos-trace-emitting-conformance.md) — scoring uses trace deltas.
- Existing fixtures under `wos-spec/fixtures/`.

## Completion criteria

1. Every kernel, governance, and AI fixture has a paired problem statement.
2. A benchmark runner produces a scoring JSON per run: converged-rate, step-accuracy, T3-pass-rate.
3. A `BENCHMARK.md` document tracks results over time with one row per monthly run.
4. A public leaderboard surface (even a committed markdown file) makes the numbers falsifiable.
5. Results include model metadata so Opus vs. Sonnet vs. Haiku comparisons are possible.

## Scoring rubric

Per problem, score:

| Dimension | Measurement |
|-----------|-------------|
| **Converged** | Did `wos-synth` exit with outcome `Converged` within the iteration cap? |
| **Structural match** | Fraction of top-level properties that match the reference fixture (Jaccard on paths). |
| **Behavioral match** | T3 conformance pass-rate against the reference fixture's committed expected trace. |
| **Iteration efficiency** | Number of repair iterations the loop needed (lower is better). |
| **Token cost** | Total prompt+completion tokens consumed (when provider reports it). |

Per run (all problems together), summarize: converged %, mean step-accuracy, mean iterations-to-converge, median token cost.

## File structure

- **Create:** `benchmarks/problems/<fixture-slug>.md` — one per fixture, the NL problem statement.
- **Create:** `crates/wos-synth/src/bench.rs` — benchmark runner (or new `wos-bench` crate).
- **Create:** `benchmarks/runs/` — committed result JSON per run.
- **Create:** `BENCHMARK.md` — leaderboard-style summary.

---

## Task 1: Author problem statements for existing fixtures

**Files:**
- Create: `benchmarks/problems/benefits-adjudication.md`
- Create: `benchmarks/problems/purchase-order-approval.md`
- Create: `benchmarks/problems/medicaid-redetermination.md`
- … one per fixture that `wos-synth` should be able to author.

- [ ] **Step 1.1:** Each statement is 1–3 paragraphs of plain English describing the workflow goal, the actors involved, the decision points, and any known constraints. Not a spec; a product brief.

- [ ] **Step 1.2:** Structure:

```markdown
# Benefits Adjudication — Problem Statement

## Goal
Adjudicate a state benefits application end-to-end: intake, eligibility, review, decision, notice of adverse action (if rejected), appeal path.

## Actors
- Applicant (human)
- Intake clerk (human)
- Eligibility examiner (human or agent)
- Adjudicator (human)

## Decision points
- Income threshold check (policy-driven)
- Identity verification gate
- Conflict-of-interest screen

## Constraints
- Rights-impacting: must support appeal and adverse-action notice.
- Must maintain 4-tier provenance.
- Must declare deontic constraints on any AI participant.

## What success looks like
A valid WOS kernel document plus governance sidecars (due-process, assertion-gate) that passes T1/T2/T3 conformance against the reference fixture.
```

- [ ] **Step 1.3:** Commit per tier. `docs: benchmark problem statements for <tier> fixtures`.

## Task 2: Benchmark runner as new `wos-bench` crate

**Files:**
- Create: `crates/wos-bench/Cargo.toml`, `src/lib.rs`, `src/main.rs`, `README.md`.
- Modify: root `Cargo.toml` workspace members.

- [ ] **Step 2.1:** Scaffold `crates/wos-bench/` depending on `wos-synth = { path = "../wos-synth", features = ["synth"] }` plus `serde`/`serde_json`/`clap`. The binary is `wos-bench`; the library exposes the scoring rubric as a reusable module (for future external runners).

- [ ] **Step 2.2:** `crates/wos-bench/README.md` must include:
  1. **Boundary statement** (per Q6): *This crate consumes `wos-synth` as a library. It owns fixture sets, scoring, regression tracking. It does NOT own the provider abstraction, prompt templates, or trace types — those live in `wos-synth`. Future contributors should not duplicate the provider abstraction here.*
  2. **Benchmark-regressions-do-not-motivate-spec-changes policy** (per Q1): *Benchmark regressions do not motivate normative-spec changes unless the benchmark is exercising a claim the spec actually makes. Spec PRs whose motivation cites a benchmark failure must be reviewed against this rule.*

- [ ] **Step 2.3:** For each problem statement, invoke `wos_synth::synthesize(...)` as a library call (not a subprocess), capture outcome + trace. Score against the reference fixture using the rubric above.

- [ ] **Step 2.4:** Write per-problem result JSON to `benchmarks/runs/<date>-<model>/results.json`.

- [ ] **Step 2.5:** Commit. `feat: wos-bench crate — runner with rubric-based scoring, imports wos-synth as library`.

## Task 3: Results aggregation and BENCHMARK.md

**Files:**
- Create: `BENCHMARK.md`
- Modify: benchmark runner to append to `BENCHMARK.md`.

- [ ] **Step 3.1:** Leaderboard-style table:

```markdown
| Date       | Model              | Provider   | Converged | Step Acc | T3 Pass | Mean Iters |
|------------|---------------------|-----------|-----------|----------|---------|-----------|
| 2026-05-01 | claude-opus-4-7     | anthropic  | 18/20     | 0.91     | 0.85    | 2.3       |
| 2026-05-01 | claude-sonnet-4-6   | anthropic  | 14/20     | 0.84     | 0.70    | 3.1       |
```

- [ ] **Step 3.2:** Add a section for methodology + caveats.

- [ ] **Step 3.3:** Commit. `docs: BENCHMARK.md leaderboard scaffold`.

## Task 4: CI integration (optional but recommended)

**Files:**
- Create: `.github/workflows/wos-bench.yml`

- [ ] **Step 4.1:** Scheduled monthly run. Uses secrets for API keys. Posts results as a PR or commit under `benchmarks/runs/`.

- [ ] **Step 4.2:** Manual dispatch supported so researchers can trigger off-cycle runs against specific models.

- [ ] **Step 4.3:** Commit. `build: scheduled WOS synthesis benchmark CI`.

---

## Self-review checklist

- Every benchmarkable fixture has a problem statement (Task 1).
- Runner scores rubric-by-rubric (Task 2).
- Leaderboard exists and updates (Task 3).
- Monthly rhythm is automated (Task 4).

## Why this matters

"WOS is designed for LLMs" becomes a measurable claim once the benchmark exists. The number can rise or fall with schema quality, prompt quality, and model capability — all three become tunable with a signal to optimize against. This is the artifact that makes Claim A falsifiable.

**Estimated effort:** ongoing; ~1 engineer-week to bootstrap, ~2 days per fixture to author good problem statements.
