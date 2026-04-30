# WOS Studio (Authoring)

**Status:** Stage 0–2 in progress — docs only, no code.
**Sibling product to:** [`../studio/`](../studio/README.md) (the runtime case-management UI).

## What this is

WOS Studio (Authoring) is a source-backed workflow intelligence platform for WOS. It transforms institutional policy and operational knowledge into validated, explainable, reviewable, WOS-aligned workflows. Most users work with sources, requirements, notices, deadlines, appeals, decisions, evidence, roles, assumptions, and scenarios — not WOS JSON.

## Disambiguation: the two studios

This repo has two top-level folders that both say "studio":

- [`../studio/`](../studio/) — **runtime case-management UI**. React 19 / Vite / Express. Consumes published `$wosWorkflow` documents and fixtures. Surfaces inbox, designer, audit trail, applicant portal, reports.
- `studio-authoring/` (this folder) — **authoring / review / change-management layer**. Different audience (program managers, policy/legal/compliance), different problem (sources → reviewed objects → WOS).

The two products share a name family but have **no runtime dependency on each other**. Both depend on `wos-spec` schemas at `../schemas/`.

## What's in this folder right now

- [`VISION.md`](VISION.md) — durable product vision (the PRD as authored, with a short framing preface). 19 numbered sections; cite as `VISION §N`.
- [`CONCEPT-MODEL.md`](CONCEPT-MODEL.md) — entity catalog, lifecycles, state boundaries, mapping states. Bridge between VISION and specs. *(Stage 1 — pending.)*
- [`specs/`](specs/) — nine internal W3C-style specs that derive from the concept model. Each follows the three-section rubric (Normative Contract / Composition / Conformance) per [`../CONVENTIONS.md`](../CONVENTIONS.md). *(Stage 2 — pending.)*

## What's NOT in this folder yet (and why)

By design, this folder is **docs-only** at this stage. The following are deliberately deferred:

- No `package.json`, no `Cargo.toml` entry, no `Makefile` targets — tech posture is deferred until specs stabilize.
- No JSON Schemas under `schemas/` — Stage 3 of the Implementation Roadmap (see [`VISION §17`](VISION.md#17-implementation-roadmap)).
- No readiness/lint engine — Stage 4.
- No Studio→WOS compiler — Stage 5.
- No scenario engine — Stage 6.
- No reference architecture docs — Stage 7.
- No vertical slice (e.g., FAFSA ISIR) — Stage 8.

This mirrors how `../specs/kernel/`, `../specs/governance/`, `../specs/ai/`, and `../specs/advanced/` were built: prose-first, schemas next, tooling last.

## Pointers

- Product vision: [`VISION.md`](VISION.md).
- Decision record: [`../thoughts/plans/2026-04-30-studio-authoring-product-pointer.md`](../thoughts/plans/2026-04-30-studio-authoring-product-pointer.md).
- Repo conventions for spec structure: [`../CONVENTIONS.md`](../CONVENTIONS.md).
- WOS schemas this product compiles to: [`../schemas/wos-workflow.schema.json`](../schemas/wos-workflow.schema.json), [`../schemas/wos-tooling.schema.json`](../schemas/wos-tooling.schema.json).

## License

This folder inherits the parent repo's licensing. Authoring tooling components fall under BSL-1.1 per the repo-root [`LICENSING.md`](../LICENSING.md), converting to Apache-2.0 in April 2030.
