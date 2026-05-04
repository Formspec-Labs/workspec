# `studio-authoring/` Product — Pointer & Decision Record

**Date:** 2026-04-30
**Branch:** `claude/wos-studio-setup-zFFDC`
**Type:** discoverability pointer + decision record (not an implementation plan).

## What this is

A new top-level product folder, [`../../studio-authoring/`](../../studio-authoring/), is being introduced to host **WOS Studio (Authoring)** — a non-technical authoring / review / validation / change-management layer for WOS workflows. The terminal output is a formal `$wosWorkflow` document plus tooling/scenario artifacts.

The full product vision lives at [`../../studio-authoring/VISION.md`](../../studio-authoring/VISION.md) (PRD §§1–19). The folder entry-point is [`../../studio-authoring/README.md`](../../studio-authoring/README.md).

## Why a sibling folder, not a fold-in

The repo already has a top-level [`../../studio/`](../../studio/) folder. That folder is a runtime case-management UI — a React/Vite/Express reference implementation that consumes published kernels. WOS Studio (Authoring) is a different product with a different audience (program managers, policy/legal/compliance staff, service designers, not case workers) and a different problem (transforming source documents into reviewed, source-backed, WOS-aligned workflow specifications, not running cases against an already-published workflow).

The two share a name family but have **no runtime dependency** on each other. Folding the new product into `studio/` would conflate two separable products with separable release cadences. Splitting into `studio-authoring/` keeps each product internally coherent.

## Decisions captured

- **Folder name:** `studio-authoring/` (not `wos-studio/` — too easily confused with `studio/` in commands and paths).
- **Stage scope (this turn):** Implementation Roadmap Stages 0–2 only — VISION + Concept Model + nine internal specs. Stages 3–8 (schemas, lint engine, compiler, scenario engine, reference architecture, vertical slice) deferred.
- **Tech posture:** docs-only at this stage. No `package.json`, no Cargo workspace entry, no `Makefile` integration, no schemas. Tech choice is deferred until specs stabilize. This mirrors `kernel/`, `governance/`, `ai/`, `advanced/` — all of which were spec-first.
- **PRD location:** `studio-authoring/VISION.md` is the durable product document, modeled on [`../../crates/wos-server/VISION.md`](../../crates/wos-server/VISION.md). This pointer file is the secondary entry-point for `thoughts/plans/` browsers.
- **Spec discipline:** every Stage-2 spec follows the three-section rubric in [`../../CONVENTIONS.md`](../../CONVENTIONS.md) (Normative Contract / Composition / Conformance). Sidecar exemption does **not** apply — these specs encode behavioral semantics (review lifecycles, mapping precedence, readiness rule firing, scenario-vs-trace comparison, change propagation) that schemas alone cannot encode.

## Out of scope for this pointer

This is not an implementation plan. There are no checkbox tasks, no agent assignments, no CI gates. When a Stage 3+ implementation plan is needed, it will be authored as a separate `thoughts/plans/YYYY-MM-DD-...md` file.

## Cross-references

- Product vision: [`../../studio-authoring/VISION.md`](../../studio-authoring/VISION.md)
- Folder entry: [`../../studio-authoring/README.md`](../../studio-authoring/README.md)
- Concept model: [`../../studio-authoring/CONCEPT-MODEL.md`](../../studio-authoring/CONCEPT-MODEL.md)
- Specs: [`../../studio-authoring/specs/`](../../studio-authoring/specs/)
- Sibling product: [`../../studio/README.md`](../../studio/README.md)
- Repo conventions: [`../../CONVENTIONS.md`](../../CONVENTIONS.md)
