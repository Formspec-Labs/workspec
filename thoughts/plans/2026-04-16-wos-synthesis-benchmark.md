# WOS Synthesis Benchmark — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pair every fixture in `wos-spec/fixtures/` with a natural-language problem statement. The set becomes a workflow-synthesis benchmark analogous to SWE-bench: given requirement R, did the LLM + WOS toolchain produce a conformant workflow? Track the rate monthly.

**Architecture:** A `benchmarks/` directory co-located with fixtures, with one problem statement per fixture. A benchmark runner invokes [`wos-synth`](./2026-04-16-wos-synth-crate.md) on each problem and scores the output against the reference fixture and the conformance suite. Results are written to `benchmark-runs/<date>-<provider>-<model>.json` and summarized in `BENCHMARK.md`.

**Tech Stack:** Rust (benchmark runner as a binary in `wos-synth` or a new `wos-bench` crate), markdown for problem statements, JSON for results.

**Spec anchor:** [architecture-review-handoff.md §5.5](../reviews/2026-04-16-architecture-review-handoff.md) — fixture corpus doubles as synthesis benchmark.

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

## Task 2: Benchmark runner

**Files:**
- Create: `crates/wos-synth/src/bench.rs` or `crates/wos-bench/`.

- [ ] **Step 2.1:** For each problem statement, invoke `wos-synth` with the problem, the target layer, and a fixed seed (model + max-iterations). Capture outcome + trace.

- [ ] **Step 2.2:** Score against the reference fixture using the rubric above.

- [ ] **Step 2.3:** Write per-problem result JSON to `benchmarks/runs/<date>-<model>/results.json`.

- [ ] **Step 2.4:** Commit. `feat: wos-bench runner with rubric-based scoring`.

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
