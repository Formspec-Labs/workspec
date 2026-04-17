# WOS Rule-Coverage Conformance — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace fixture-count as the reported conformance metric with **rule coverage**. Every lint rule and T3 runtime rule must link to ≥1 passing fixture. CI enforces the link. `LINT-MATRIX.md` publishes per-tier coverage and a rule-graduation ladder.

**Architecture:** The rule registry already exists in `crates/wos-lint/src/rules/` and `crates/wos-conformance/`. The missing piece is the inverse index (rule → fixture) and a compile-time or test-time check that every rule appears in that index. The ladder (`draft`, `tested`, `stable`, `load-bearing`) is a metadata field on each rule, read by a new CLI subcommand that regenerates the matrix.

**Tech Stack:** Rust (`wos-lint`, `wos-conformance`), markdown generation from rule registry, GitHub Actions for CI gating.

**Spec anchor:** [architecture-review-handoff.md §4.2](../archive/reviews/2026-04-16-architecture-review-handoff.md) — honest conformance numbers.

---

## Prerequisites

- `crates/wos-lint/src/rules/` — existing rule registry (197 rules).
- `crates/wos-conformance/` — existing T3 runtime rule harness (currently ~9/26 covered).
- `LINT-MATRIX.md` — current matrix document (reports fixture count, not rule coverage).
- Fixture tree: `fixtures/{kernel,ai,governance,advanced,companions,profiles,sidecars,validation}/`.

## Completion criteria

1. Every rule in `wos-lint` and `wos-conformance` carries an explicit `fixtures: &[&str]` field pointing at fixture files that exercise it.
2. A `#[test]` in each crate walks the registry and asserts `rule.fixtures.len() >= 1` for all rules that have graduated past `draft`.
3. A CLI command `wos-conformance coverage` prints per-tier coverage and the ladder state of every rule, consuming the same registry.
4. `LINT-MATRIX.md` is regenerated from that CLI (Task 6) and replaces the current fixture-count metric with rule coverage.
5. Pre-commit hook or CI job runs the coverage CLI and fails if the matrix is stale.

## File structure

- **Modify:** `crates/wos-lint/src/rules/mod.rs` — add `fixtures` + `graduation` fields to rule metadata struct.
- **Modify:** every file under `crates/wos-lint/src/rules/` — populate `fixtures` for each rule.
- **Modify:** `crates/wos-conformance/src/rules.rs` — same fields, same population.
- **Create:** `crates/wos-conformance/src/coverage.rs` — coverage computation + CLI subcommand.
- **Create:** `crates/wos-conformance/tests/fixture_links.rs` — test that all non-`draft` rules have ≥1 fixture.
- **Modify:** `LINT-MATRIX.md` — regenerated header; include ladder section.
- **Create:** `.github/workflows/coverage.yml` — CI gate running `wos-conformance coverage --fail-on-stale`.

---

## Task 1: Extend rule metadata with `fixtures` and `graduation`

**Files:**
- Modify: `crates/wos-lint/src/rules/mod.rs`
- Modify: `crates/wos-conformance/src/rules.rs`

- [ ] **Step 1.1:** Add to the rule metadata struct:

```rust
pub struct RuleMetadata {
    pub id: &'static str,
    pub tier: Tier,               // T1 | T2 | T3
    pub severity: Severity,
    pub summary: &'static str,
    pub fixtures: &'static [&'static str],  // NEW — paths relative to fixtures/
    pub graduation: Graduation,              // NEW — Draft | Tested | Stable | LoadBearing
}

pub enum Graduation {
    Draft,        // no fixture yet
    Tested,       // ≥1 fixture
    Stable,       // Tested + passing 3+ consecutive releases
    LoadBearing,  // removing would break a reference impl
}
```

- [ ] **Step 1.2:** Default every existing rule to `graduation: Graduation::Draft, fixtures: &[]`. This is the "everything starts undocumented" baseline.

- [ ] **Step 1.3:** `cargo check -p wos-lint -p wos-conformance` — expect green after adding the fields.

- [ ] **Step 1.4:** Commit. `build: add fixtures and graduation metadata to WOS rule registry`.

## Task 2: Backfill fixture links + seed load-bearing set

**Files:**
- Modify: every file under `crates/wos-lint/src/rules/` and `crates/wos-conformance/src/rules/`.

**Background (from [open questions Q4 decision, 2026-04-17](../reviews/2026-04-16-architecture-review-open-questions.md#q4-which-rules-today-are-load-bearing-on-the-graduation-ladder)):** The ladder uses a four-state graduation (`Draft` → `Tested` → `Stable` → `LoadBearing`). Promotion to `LoadBearing` uses a four-part mechanical test (specRef + suggestedFix + fixture + removal-breaks-conformance). Initial seeded `LoadBearing` set is the union of all three reviewers' proposals, with K-012 / K-017 held at `Stable` pending an explicit audit.

- [ ] **Step 2.1:** For each rule that already has a passing fixture in the tree, promote it to `graduation: Graduation::Tested` and set `fixtures` to the fixture paths. Use `grep -r <rule-id>` in fixtures/ + tests to find the links.

- [ ] **Step 2.2:** Record untouched rules as still `Draft`. Do NOT fabricate fixture links — the honesty dividend depends on the ladder reflecting reality.

- [ ] **Step 2.3:** Apply the mechanical promotion test to every candidate. A rule promotes to `LoadBearing` only if ALL four parts are met:
  1. Rule metadata has a normative `spec_ref: &'static str` citing a `§` in a canonical spec file (e.g., `"kernel/spec.md#§5.3"`).
  2. Rule metadata has an imperative `suggested_fix: &'static str` (non-empty).
  3. `fixtures.len() >= 1`.
  4. Removing the rule from the active set causes at least one conformance test to fail. **Verified by Task 3 automation, not by inspection.**

- [ ] **Step 2.4:** Seeded initial `LoadBearing` set on first promotion pass (exactly these rules; everything else stays at `Tested` or below):
  - **K-023** (terminal-without-transition)
  - **K-030** (extension-prefix)
  - **K-016** (mutation history append-only)
  - **K-020** (every mutation produces Facts provenance)
  - **K-047** (case relationships MUST NOT affect lifecycle evaluation)
  - **G-037**, **G-042**, **G-043** (governance structural invariants)
  - **G-044**, **G-045** (delegation date ordering)
  - **AI-024** (agent-reference condition)

- [ ] **Step 2.5:** Held at `Stable` pending Step 2.7 audit: **K-012**, **K-017**. Do not promote these in this task.

- [ ] **Step 2.6:** Commit. `docs: backfill fixture links + seed load-bearing graduation ladder`.

- [ ] **Step 2.7:** **K-012 / K-017 audit — separate commit.** For each of these two rules, answer in writing: "name the fixture that breaks when this rule is disabled." If an existing fixture breaks, record that fixture name in rule metadata `fixtures` and promote. If no existing fixture breaks, either (a) author a new fixture that breaks without the rule — then promote — or (b) leave at `Stable`. Do not write a fixture whose only purpose is to justify the promotion — that defeats the ratchet.

- [ ] **Step 2.8:** Commit the audit outcome. `docs: K-012/K-017 load-bearing audit — <promoted|held>`.

## Task 3: Failing CI test — no un-linked rules past Draft

**Files:**
- Create: `crates/wos-conformance/tests/fixture_links.rs`

- [ ] **Step 3.1:** Write the failing test:

```rust
use wos_conformance::rules::all_rules;
use wos_lint::rules::all_lint_rules;

#[test]
fn every_non_draft_rule_has_at_least_one_fixture() {
    let mut offenders: Vec<&str> = vec![];
    for rule in all_rules().iter().chain(all_lint_rules().iter()) {
        let draftish = matches!(rule.graduation, wos_lint::Graduation::Draft);
        if !draftish && rule.fixtures.is_empty() {
            offenders.push(rule.id);
        }
    }
    assert!(
        offenders.is_empty(),
        "rules promoted past Draft but missing fixture links: {:?}",
        offenders
    );
}
```

- [ ] **Step 3.2:** Run it — it should pass if Task 2 was honest. If it fails, demote the offending rule to `Draft`. The test *asserts* honesty; don't game it.

- [ ] **Step 3.3:** Commit. `test: guard WOS rule promotion with fixture-link assertion`.

## Task 4: Coverage CLI subcommand

**Files:**
- Create: `crates/wos-conformance/src/coverage.rs`
- Modify: `crates/wos-conformance/src/main.rs` (add subcommand)

- [ ] **Step 4.1:** Implement `wos-conformance coverage` that walks both registries, prints:

```
T1: 89/89 rules covered (100.0%)   — 142 fixtures linked
T2: 74/80 rules covered (92.5%)    — 98 fixtures linked
T3: 9/26 rules covered (34.6%)     — 41 fixtures linked
Overall: 172/195 (88.2%)

Graduation ladder:
  load-bearing: 14
  stable:       58
  tested:      100
  draft:        23
```

- [ ] **Step 4.2:** Add `--json` flag for machine consumption.
- [ ] **Step 4.3:** Add `--fail-on-stale` flag that compares output against `LINT-MATRIX.md` and exits non-zero if drifted.
- [ ] **Step 4.4:** Commit. `feat: wos-conformance coverage CLI with graduation ladder`.

## Task 5: Regenerate LINT-MATRIX.md from CLI

**Files:**
- Modify: `LINT-MATRIX.md` (full regeneration; preserve narrative preamble)
- Create or modify: a generator script that pipes `wos-conformance coverage --json` into the matrix template.

- [ ] **Step 5.1:** Replace the "counts" header with rule-coverage numbers and graduation breakdown.
- [ ] **Step 5.2:** Add a ladder table with one row per rule and its current state.
- [ ] **Step 5.3:** Mark the matrix with a `<!-- generated: do not edit -->` guard.
- [ ] **Step 5.4:** Commit. `docs: regenerate LINT-MATRIX.md with rule-coverage metric and graduation ladder`.

## Task 6: CI gate

**Files:**
- Create: `.github/workflows/wos-coverage.yml` (or extend existing WOS workflow)

- [ ] **Step 6.1:** Add a job that runs `cargo run -p wos-conformance -- coverage --fail-on-stale`. Fails the PR if the matrix is stale or any promoted rule is un-linked.
- [ ] **Step 6.2:** Commit. `build: CI gate for WOS rule-coverage drift`.

## Task 7: Automate criterion (iv) of the `LoadBearing` promotion test

**Files:**
- Create: `crates/wos-conformance/src/bin/ratchet-check.rs` (or a subcommand on an existing binary).
- Modify: `.github/workflows/wos-coverage.yml` (add a slow-path job).

**Background (from [Q4 Action item](../reviews/2026-04-16-architecture-review-open-questions.md#q4-which-rules-today-are-load-bearing-on-the-graduation-ladder)):** "Removing the rule permits a conformance-suite regression" is the hardest of the four criteria to verify by inspection. Without automation it collapses to judgment, and the ratchet becomes aspirational. Run this check only at promotion time (when a PR changes a rule's `graduation` to `LoadBearing`), not per-PR — the job is O(n × conformance-suite) and expensive.

- [ ] **Step 7.1:** Implement `wos-conformance ratchet-check --rule <id>`:
  1. Disable the named rule in the active registry.
  2. Run the full conformance suite.
  3. Exit 0 (ratchet-safe) iff at least one test fails with the rule disabled; non-zero otherwise.

- [ ] **Step 7.2:** Add a CI job that detects `graduation: Graduation::LoadBearing` additions in the diff and runs `ratchet-check` for each newly-promoted rule. Fails the PR if any check returns non-zero.

- [ ] **Step 7.3:** Commit. `feat: ratchet-check automates criterion (iv) of LoadBearing promotion test`.

---

## Self-review checklist

- Every rule has `graduation` and `fixtures` fields — yes (Task 1).
- CI enforces the link — yes (Task 3, Task 6).
- Matrix document regenerates from the same source — yes (Task 5).
- No hand-edits to generated regions — yes (guard comment in Task 5.3).
- Ladder publishes rule-by-rule state — yes (Task 4.1).

## Adjacent rules absorbed into this plan

The following rules from the architecture review (§4.1, §4.3) are small enough to add during the ladder backfill. They do not warrant separate plans.

- **K-EXT-001** (T1, error) — **note: largely redundant** with the §4.1 schema patch (`additionalProperties: false` + `patternProperties: ^x-`). Schema validation now rejects unknown non-`x-` properties at every level. Before re-implementing as a lint rule, confirm that consumers see an actionable error from the schema validator. If yes, record K-EXT-001 as `Draft` + `subsumedBy: "JSON Schema 2020-12 patternProperties"` in the registry rather than implementing it twice.
- **K-EXT-002** (T2, warning) — reserve `x-wos-*` namespace for future spec use. Novel rule (schema cannot enforce prefix-within-prefix). Implementation: walk the document tree, for every key matching `^x-wos-`, emit a warning citing "reserved for future normative use." Link to fixtures `fixtures/validation/x-wos-reserved-warn.json` (bad) and `fixtures/validation/x-vendor-custom-ok.json` (good).
- **COMP-001** (T2, warning) — drift detector between `lifecycle-detail.md` and `runtime.md`. Scan both files for sentences matching normative pattern `(state|event|timer|guard|compensation|action) (MUST|SHOULD|MAY) …`, extract the subject, and warn when the same subject appears with different modals. Depends on [§5.2 structured diagnostics](./2026-04-16-wos-structured-lint-diagnostics.md) for the output shape; implement after that plan lands.

## Rollout

This plan is a prerequisite for [§4.4 release-train split](./2026-04-16-wos-release-trains.md) — you cannot publish per-tier coverage numbers for independent release trains until coverage itself is honest.

**Estimated effort:** ~1 engineer-week (excluding K-EXT-002 and COMP-001, which add ~2 days each).
