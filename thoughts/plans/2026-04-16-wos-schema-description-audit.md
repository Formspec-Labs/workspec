# WOS Schema Description Audit — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every property in every WOS schema has a `description` long enough for an LLM to generate a valid instance and at least one concrete `examples` entry. Introduce lint rule `SCHEMA-DOC-001` that enforces both conditions. Schema descriptions become load-bearing prompt material for [Claim A (LLM-authored workflows)](../../POSITIONING.md).

**Architecture:** Audit existing schemas; add a minimum-length check and example presence check as a T1 lint rule over the schema files themselves (schema-of-schemas pattern). `x-lm.critical=true` properties get a higher bar.

**Tech Stack:** Rust (`wos-lint`), JSON Schema 2020-12, existing `x-lm` annotation convention.

**Spec anchor:** [architecture-review-handoff.md §5.1](../reviews/2026-04-16-architecture-review-handoff.md).

---

## Prerequisites

- 19 schemas under `wos-spec/schemas/**/*.json`.
- Existing `x-lm` annotation with `critical`, `intent` fields (see `wos-kernel.schema.json` for examples).
- Existing lint infrastructure in `crates/wos-lint/`.

## Completion criteria

1. `SCHEMA-DOC-001` lint rule implemented and tested.
2. Every leaf property in every schema has `description` ≥ 60 characters **and** `examples: [...]` with ≥1 entry.
3. `x-lm.critical=true` properties have `description` ≥ 140 characters and ≥ 2 examples (already a documented expectation in `CLAUDE.md`; this promotes it from soft to enforced).
4. Existing schemas are audited and backfilled to pass the rule. No description shortening to game the threshold — if something has nothing to say, it should not be a leaf property.
5. Coverage contribution: rule is linked to at least 2 fixtures (one passing, one failing).

## File structure

- **Create:** `crates/wos-lint/src/rules/schema_doc.rs` — the `SCHEMA-DOC-001` rule implementation.
- **Modify:** schemas under `wos-spec/schemas/**` — backfill descriptions and examples.
- **Create:** `fixtures/validation/schema-doc-good.json` — schema document passing SCHEMA-DOC-001.
- **Create:** `fixtures/validation/schema-doc-bad.json` — schema document failing (missing examples on a leaf).
- **Modify:** `LINT-MATRIX.md` — register the new rule.

---

## Task 1: Write the failing schema lint rule

**Files:**
- Create: `crates/wos-lint/src/rules/schema_doc.rs`
- Modify: `crates/wos-lint/src/rules/mod.rs` (register rule)

- [ ] **Step 1.1:** Add rule metadata:

```rust
pub const SCHEMA_DOC_001: RuleMetadata = RuleMetadata {
    id: "SCHEMA-DOC-001",
    tier: Tier::T1,
    severity: Severity::Error,
    summary: "Every leaf schema property must have a description and ≥1 example; critical properties must be richer.",
    fixtures: &[
        "validation/schema-doc-good.json",
        "validation/schema-doc-bad.json",
    ],
    graduation: Graduation::Tested,
};
```

- [ ] **Step 1.2:** Implement the walker: recurse `properties` and `items`, skip sub-schemas that are themselves `$ref`. For each leaf (no `properties`, no `items`), assert:
  - `description` exists and `.len() >= 60` (baseline) or `.len() >= 140` (when `x-lm.critical == true`).
  - `examples` is a non-empty array (baseline) or ≥ 2 entries (critical).

- [ ] **Step 1.3:** Unit tests cover: baseline leaf missing description → error; baseline leaf missing examples → error; critical leaf with only 1 example → error; non-leaf without description → ignored.

- [ ] **Step 1.4:** Commit. `feat: SCHEMA-DOC-001 lint rule (baseline + critical thresholds)`.

## Task 2: Run the rule against the existing schemas

**Files:**
- No file changes this task — diagnostic pass only.

- [ ] **Step 2.1:** Run `cargo run -p wos-lint -- lint-schemas schemas/` and capture the full offender list. Expect hundreds of violations on first pass. Triage into three buckets:
  - **Backfill** — property is load-bearing, needs real documentation.
  - **Reshape** — property is sparse enough that it should probably be a `$ref` to a reusable `$defs` entry.
  - **Delete** — property is not actually used anywhere and should be removed.

- [ ] **Step 2.2:** Document the triage in `thoughts/research/2026-04-16-schema-doc-audit-triage.md`.

- [ ] **Step 2.3:** Commit. `docs: triage of WOS schema documentation gaps`.

## Task 3: Backfill descriptions and examples, tier by tier

**Files:**
- Modify: schemas per triage.

Work through tiers in this order to align with release-stream cadence (see [release-trains plan](./2026-04-16-wos-release-trains.md)):

1. `schemas/kernel/**`
2. `schemas/companions/**`
3. `schemas/governance/**`
4. `schemas/ai/**`
5. `schemas/profiles/**`, `schemas/sidecars/**`, `schemas/assurance/**`
6. `schemas/advanced/**` (research-grade, lower bar acceptable)

For each tier:

- [ ] **Step 3.X.1:** Apply backfills; no copy-paste descriptions across unrelated properties.
- [ ] **Step 3.X.2:** Run `SCHEMA-DOC-001` and confirm the tier passes.
- [ ] **Step 3.X.3:** Run `cargo test -p wos-lint -p wos-conformance` — zero regressions.
- [ ] **Step 3.X.4:** Commit with tier name: `docs(kernel): backfill schema descriptions and examples for LLM authoring`.

## Task 4: Register the rule in LINT-MATRIX and CI

**Files:**
- Modify: `LINT-MATRIX.md`
- Modify: `.github/workflows/` whichever runs lint.

- [ ] **Step 4.1:** After [§4.2 rule-coverage](./2026-04-16-wos-rule-coverage-conformance.md) lands, the matrix generator will pick this rule up automatically. If that plan is not landed yet, add the row manually and note the dependency.

- [ ] **Step 4.2:** CI runs `wos-lint lint-schemas schemas/` on every PR. Any SCHEMA-DOC-001 violation blocks merge.

- [ ] **Step 4.3:** Commit. `build: CI gate for WOS schema-doc coverage`.

---

## Self-review checklist

- Rule exists and is tested (Task 1).
- Every schema in every tier passes (Task 3).
- Rule is visible in `LINT-MATRIX.md` and CI enforces it (Task 4).
- Fixtures link the rule for coverage credit (declared in Task 1.1).

## Why this matters

The Claim A thesis is that LLMs can author WOS documents by reading schemas. The schema descriptions are the prompt. An LLM reading `"description": "timestamp"` cannot distinguish `Instant` from `ISO8601 with timezone`. Backfilling this surface is the single highest-leverage investment in Claim A feasibility.

**Estimated effort:** ~1 engineer-week for the rule; ~2 engineer-weeks distributed across tiers for backfill.
