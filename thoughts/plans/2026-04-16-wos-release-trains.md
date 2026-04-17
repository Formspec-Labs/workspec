# WOS Release Trains by Layer — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split WOS releases into four independent version streams — `wos-kernel`, `wos-governance`, `wos-ai`, `wos-advanced` — in one repo, so kernel vendors can pin stability while AI consumers ride monthly churn. Vendor claims become `wos-kernel@1.0 + wos-ai@0.5` instead of `wos@1.0`.

**Architecture:** Four git tag prefixes + four CHANGELOG files + a compatibility matrix. The workspace stays unified; release tooling keys off path to decide which stream is being tagged. Per-tier conformance numbers (from [rule-coverage plan](./2026-04-16-wos-rule-coverage-conformance.md)) attach to per-tier releases.

**Tech Stack:** Git tags, GitHub Actions release workflow, existing Cargo workspace, markdown CHANGELOGs.

**Spec anchor:** [architecture-review-handoff.md §4.4](../reviews/2026-04-16-architecture-review-handoff.md) — split release trains by layer.

**Related:** Parent repo [ADR 0063](../../../thoughts/adr/0063-release-trains-by-tier.md) does the same split for Formspec packages. This plan extends the pattern to WOS layers.

---

## Prerequisites

- [§4.2 rule-coverage plan](./2026-04-16-wos-rule-coverage-conformance.md) landed — per-tier conformance numbers must be honest before publishing them as stream metadata.
- Existing release workflow (if any) documented so the new one can replace it cleanly.
- All four streams currently co-versioned. Consumers have zero awareness that they will be split.

## Completion criteria

1. Four CHANGELOG files in `wos-spec/changelogs/{kernel,governance,ai,advanced}.md`.
2. Tag scheme: `wos-kernel-v1.0.0`, `wos-governance-v1.0.0`, `wos-ai-v0.5.0`, `wos-advanced-v0.1.0` (research).
3. `COMPATIBILITY-MATRIX.md` declares which stream version ranges are known-good together.
4. Release workflow has four jobs (one per stream); each job only runs when files matching that stream changed.
5. Vendor-facing doc explains: "claim `wos-kernel@X + wos-ai@Y`; see matrix for pairings."

## File structure

- **Create:** `wos-spec/changelogs/kernel.md`, `governance.md`, `ai.md`, `advanced.md`.
- **Create:** `wos-spec/COMPATIBILITY-MATRIX.md` — cross-stream version ranges.
- **Create:** `.github/workflows/wos-release.yml` — per-stream tagging.
- **Modify:** `wos-spec/README.md` — update to reference the stream model.
- **Delete:** any monolithic `wos-spec/CHANGELOG.md` (replaced by the four stream files).

---

## Task 1: Define stream → path mapping

**Files:**
- Create: `wos-spec/RELEASE-STREAMS.md`

- [ ] **Step 1.1:** Author a table mapping each stream to its source paths:

| Stream | Paths | Cadence | Stability |
|--------|-------|---------|-----------|
| `wos-kernel` | `specs/kernel/**`, `specs/companions/**`, `schemas/kernel/**`, `schemas/companions/**`, `crates/wos-core/**`, `crates/wos-runtime/**` | 6–12 months | semver-strict after 1.0 |
| `wos-governance` | `specs/governance/**`, `schemas/governance/**`, `fixtures/governance/**` | 3–6 months | semver-strict at 1.0+ |
| `wos-ai` | `specs/ai/**`, `schemas/ai/**`, `fixtures/ai/**` | monthly/quarterly | pre-1.0, no stability guarantee |
| `wos-advanced` | `specs/advanced/**`, `schemas/advanced/**`, `fixtures/advanced/**`, `specs/assurance/**`, `schemas/assurance/**` | research | no GA commitment |

- [ ] **Step 1.2:** Sidecars, profiles, and the `wos-lint` + `wos-conformance` crates are cross-cutting — document which stream owns them. Default: lint/conformance follow kernel because they define the checking surface.

- [ ] **Step 1.3:** Commit. `docs: declare WOS release-stream path mapping`.

## Task 2: Create four CHANGELOG files

**Files:**
- Create: `wos-spec/changelogs/kernel.md`
- Create: `wos-spec/changelogs/governance.md`
- Create: `wos-spec/changelogs/ai.md`
- Create: `wos-spec/changelogs/advanced.md`

- [ ] **Step 2.1:** Each file starts with:

```markdown
# <Stream> CHANGELOG

All changes scoped to the `<stream>` release train. See `RELEASE-STREAMS.md` for scope.
Versions follow semver where applicable; see header of each version block for stability commitment.

## [Unreleased]

## [1.0.0] — YYYY-MM-DD
Initial release.
```

- [ ] **Step 2.2:** If a monolithic `CHANGELOG.md` exists, move its entries into the four streams by path.

- [ ] **Step 2.3:** Commit. `docs: split WOS CHANGELOG into per-stream files`.

## Task 3: Compatibility matrix

**Files:**
- Create: `wos-spec/COMPATIBILITY-MATRIX.md`

- [ ] **Step 3.1:** Document which versions pair safely:

```markdown
# WOS Stream Compatibility Matrix

| kernel | governance | ai | advanced |
|--------|-----------|-----|----------|
| 1.0.x  | 1.0.x      | 0.4–0.6 | 0.1.x (research) |

**Rule:** `wos-ai@0.5` requires `wos-kernel@>=1.0`. A processor claiming multi-stream conformance MUST publish which versions it implements from each stream.

Breaking-change criteria per stream are documented in each CHANGELOG header.
```

- [ ] **Step 3.2:** Commit. `docs: WOS cross-stream compatibility matrix`.

## Task 4: Release workflow per stream

**Files:**
- Create: `.github/workflows/wos-release.yml`

- [ ] **Step 4.1:** Workflow triggers on push of tag matching `wos-{kernel,governance,ai,advanced}-v*`. For each stream:
  - Run the matching conformance tier (`cargo test -p wos-conformance -- --tier kernel` etc).
  - Build release artifacts (schemas bundle, generated LINT-MATRIX slice, changelog excerpt).
  - Publish a GitHub release with the stream name in the title.

- [ ] **Step 4.2:** Add a pre-tag check: the stream's CHANGELOG must have an `[Unreleased]` → versioned entry for the tag being pushed.

- [ ] **Step 4.3:** Commit. `build: per-stream WOS release workflow`.

## Task 5: Update consumer-facing docs

**Files:**
- Modify: `wos-spec/README.md`
- Modify: `wos-spec/POSITIONING.md`

- [ ] **Step 5.1:** In README, after the two-line pitch, add a "Versioning" section:

```markdown
## Versioning

WOS ships four independent release streams. Pin only what you consume:

- **wos-kernel** — 6–12 month cadence; semver-strict.
- **wos-governance** — 3–6 months; semver-strict.
- **wos-ai** — monthly/quarterly; pre-1.0, APIs may churn.
- **wos-advanced** — research; no GA commitment.

See `COMPATIBILITY-MATRIX.md` for known-good pairings.
```

- [ ] **Step 5.2:** Commit. `docs: README publishes WOS release-stream model`.

---

## Self-review checklist

- Stream → path mapping published (Task 1).
- Four CHANGELOGs exist and any monolithic history is migrated (Task 2).
- Compatibility matrix published (Task 3).
- CI enforces per-stream release hygiene (Task 4).
- Consumer docs updated (Task 5).
- Per-tier conformance numbers reference the rule-coverage matrix from [§4.2](./2026-04-16-wos-rule-coverage-conformance.md).

## Deferral

This plan should land AFTER the rule-coverage plan. Splitting release trains without honest per-stream conformance numbers reproduces the original problem at a finer granularity.

**Estimated effort:** ~2 engineer-weeks.
