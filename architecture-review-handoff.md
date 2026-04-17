# WOS Architecture Review — Handoff

**Date:** 2026-04-16
**Branch:** `claude/review-architecture-specs-B05oy`
**Scope:** Critical review of specs, schemas, crates, fixtures, and the spec-development loop. Incorporates pushback from maintainer and the refined AI-native positioning.

---

## 1. Refined Understanding of the Approach

### The development loop (maintainer-stated)
`spec → schema → lint → conformance → runtime → reference implementation → spec iteration (repeat)`

This is the methodology the repo is built around. It is the primary reason to resist surface-level "simplification" critiques: fine-grained schemas exist so lint can target precise failure modes, which feeds precise conformance signal, which feeds precise spec iteration. The granularity is a feature of the loop, not bloat.

### The two-part AI-native thesis (the actual differentiator)

**Claim A — LLM-authored workflows (the generation story).**
Workflows are structured data. An LLM can generate them directly, lint gives immediate structural feedback, conformance gives immediate behavioral feedback, the author sees impact before deployment. The spec→schema→lint→conformance loop is *also* the LLM's authoring loop, compressed to seconds. Other workflow standards (BPMN, SCXML, XPDL, CMMN) were designed for human modelers with canvases. WOS is accidentally — and should deliberately become — a standard whose reference authoring tool is an LLM.

**Claim B — Agents as first-class runtime actors (the execution story).**
When the workflow runs, agents are declarable participants alongside humans and services with autonomy levels, confidence gates, deontic constraints, drift monitoring. Optional, but native to the design via the `actorExtension` seam.

**Why separation matters.** Claim A is about *authoring* and addresses every org that writes workflows. Claim B is about *execution* and addresses the subset wanting agent-executed workflows. The current repo conflates them under "AI-native" and markets only Claim B via a 666-line AI spec. Claim A is the bigger market and is not currently surfaced.

### The sharpest one-line positioning

> Other workflow standards were designed for humans with canvases. WOS is designed for LLMs with schemas. Agents are a reference extension of the same design.

Incumbents cannot copy this without redesigning their standard. BPMN cannot retrofit schema-first. SCXML cannot retrofit JSON. Temporal is code-first, not spec-first. This is a defensible lane.

---

## 2. What the Repo Got Right (Preserve)

1. **Kernel seam-based extension model.** The five named seams (`actorExtension`, `contractHook`, `provenanceLayer`, `lifecycleHook`, `extensions`) are the right way to attach governance and AI without kernel coupling. Exemplary design.
2. **Three-tier verification framework (T1 static / T2 cross-doc / T3 dynamic).** Unusual clarity of separation between structural, semantic, and behavioral validation. `LINT-MATRIX.md` is the right artifact.
3. **Real crates, not stubs.** `wos-core`, `wos-lint`, `wos-conformance` — ~20 KLOC Rust, 334 tests, typed evaluator. The hardest part is done.
4. **Formspec boundary respected.** WOS delegates forms/definitions/expressions to Formspec and stays an orchestration layer. Correct separation.
5. **SCXML/Harel statecharts borrowed, not reinvented.** The right theory foundation.
6. **Real domain fixtures** (`purchase-order-approval`, `benefits-adjudication`, `medicaid-redetermination`) make the spec concrete and runnable.
7. **Fine-grained schemas (18) are justified by the loop methodology.** Initial critique to collapse to 4 was wrong; granularity enables precise lint and precise conformance.

---

## 3. Architectural Decisions Validated (Initial Critique Retracted)

After maintainer pushback and re-examination of the loop methodology:

- **18 schemas is correct**, not bloat. Each schema = cohesive lint surface = fixture family.
- **Named sidecars (`correspondence`, `policy`, `assertion`)** carry semantic intent a generic attachment loses. Keep them named.
- **Lifecycle-detail vs. runtime split** mirrors a real conceptual distinction (design-time structure vs. instance dynamics). BPMN conflates these and suffers for it. Keep both, but see §4 for the missing precedence clause.
- **DRAFTS/ v2–v7** are loop artifacts — iteration made visible. Move to `history/` with ADRs rather than deleting.
- **Layer 3 (advanced governance, equity, verification)** being "research-grade optional" is disclosed, not hidden. Piecemeal adoption is a feature.

---

## 4. Remaining Execution-Hygiene Work (The Real Backlog)

These are bugs and hygiene gaps, not architectural restructuring. Four items, ~one engineer-month total.

### 4.1 Fix the `additionalProperties: false` vs. `x-` extension contradiction

**Bug.** Every schema in `schemas/` sets `"additionalProperties": false` at root. Kernel §10.5 mandates `x-` prefixed extension keys as the forward-compatibility seam. Schemas reject the spec's own extension model.

**Fix.** Replace with `patternProperties` permitting `^x-`:

```json
"patternProperties": {
  "^x-": { "$comment": "Reserved for vendor extensions per §10.5" }
},
"additionalProperties": false
```

Apply uniformly across all 18 schemas.

**New lint rules.**
- `K-EXT-001` (T1): unknown property not matching `^x-` is a structural error.
- `K-EXT-002` (T2): `x-wos-*` namespace reserved for future spec use.

**Fixtures.** One valid `x-vendor-*` doc, one invalid `x-wos-reserved` doc.

**Cost.** ~3 lines × 18 schemas. Half a day. Ship as a fix, not a breaking change.

**Priority:** Ship first — smallest, unblocks vendors.

### 4.2 Make conformance numbers honest

**Bug.** `LINT-MATRIX.md` claims "99 green fixtures, 0 T3 red." Fixture tree has ~41 documents. `wos-conformance` exercises ~9 of 26 T3 rules. The metric (fixture count) is not the metric that matters (rule coverage).

**Fix — three layers.**

**a) Redefine the reported metric to rule coverage.**
```
T1: 89/89 rules covered (100%)  — 142 fixtures
T2: 74/80 rules covered (92.5%) — 98 fixtures
T3: 9/26 rules covered (34.6%)  — 41 fixtures
Overall: 172/195 (88.2%)
```
Every rule links to ≥1 fixture. No link = no coverage credit.

**b) Enforce in CI.** `wos-conformance` fails the build if a rule has no linked fixture. Walk the rule registry in a `#[test]`, assert `fixtures_for(rule).len() >= 1`.

**c) Rule graduation ladder.**
Each rule carries a state:
- `draft` — no fixture
- `tested` — one fixture
- `stable` — passing for 3 consecutive releases
- `load-bearing` — removing would break a reference impl

Publish the ladder in `LINT-MATRIX.md`.

**Cost.** ~1 week of harness work. Honesty dividend compounds on every external eval.

**Priority:** Ship third — prerequisite for split release trains.

### 4.3 Declare a conflict-resolution rule between `lifecycle-detail` and `runtime`

**Bug.** Both companions are "normative." Both cover determinism, timer scoping, compensation. An implementer hitting a conflict has no precedence rule. Mature specs (IETF, W3C) always declare precedence.

**Fix.** Add §1 "Normative Precedence" to both companions:

> Where this document and [other companion] appear to conflict:
> - On **state structure, transitions, and guards**: `lifecycle-detail.md` is authoritative.
> - On **instance behavior, event queuing, durability, and timer firing**: `runtime.md` is authoritative.
> - On anything else: file an issue; a conflict is a spec defect.

**New lint rule.** `COMP-001` (T2): scan for identical claims across both files, report drift.

**Alternative (heavier, cleaner).** Extract overlap into `execution-semantics.md`; both companions reference it. Three files, one source of truth, zero conflict possible.

**Cost.** Half a day for the clause. One week for the extracted shared doc.

**Priority:** Ship second — one paragraph, eliminates latent defect.

### 4.4 Split release trains by layer

**Bug.** Kernel, Governance, AI, Advanced all ride one repo, one CHANGELOG, one cadence. Kernel needs stability (vendors pin for years). AI needs agility (MCP, agent protocols, autonomy taxonomies move monthly). Governance moves on policy cycles. One cadence = kernel drags AI, AI destabilizes kernel.

**Fix — three version streams, one repo.**
```
wos-kernel      v1.0, v1.1, v2.0  — slow, semver-strict, 6–12 month cadence
wos-governance  v1.0, v1.1, v1.2  — medium, 3–6 month cadence
wos-ai          v0.4, v0.5, v0.6  — fast, pre-1.0, monthly/quarterly
wos-advanced    research track, no GA commitment
```

Each stream: own CHANGELOG, own conformance target, own release notes. Vendors claim: "processor X implements `wos-kernel@1.0` + `wos-ai@0.5`."

**Compatibility matrix** in repo: `wos-ai@0.5` requires `wos-kernel@>=1.0`. Enables clean deprecation.

**Cost.** ~2 weeks of release-process work (CI matrices, version tags, CHANGELOG splits).

**Priority:** Ship fourth — only worthwhile after honest per-layer conformance numbers (§4.2).

---

## 5. New Work Implied by the AI-Native Positioning

If Claim A (LLM authoring) is surfaced as a first-class goal, several new work items follow. These are additive to §4.

### 5.1 Schema descriptions become load-bearing

LLM authoring depends on `description` fields as prompt material. Audit every property across 18 schemas: if an LLM reading only the schema cannot generate a valid instance, the description is underspecified.

**New lint rule.** `SCHEMA-DOC-001` (T1): every property must have a non-empty description of ≥N chars and ≥1 example.

### 5.2 Lint output becomes an API, not a log

Define a structured `LintDiagnostic`:
```json
{
  "ruleId": "K-023",
  "severity": "error",
  "path": "$.states.approved",
  "message": "state 'approved' has no transition and is not terminal",
  "suggestedFix": { ... },
  "relatedDocs": ["specs/kernel/spec.md#S4.2"]
}
```
Every rule emits this shape. LLMs consume JSON; humans consume rendered form. Breaking change for `wos-lint` output, but prerequisite for the authoring loop.

### 5.3 Conformance becomes a teaching signal

`wos-conformance` emits traces, not pass/fail. An LLM that generated a workflow gets back "at step 4 expected `review`, actual `rejected` because guard `G-02` evaluated false when policy `P-11` applied." That delta is learnable.

### 5.4 New artifact: `wos-synth`

Reference LLM-authoring harness as a fourth crate alongside `wos-core`/`wos-lint`/`wos-conformance`:
- Input: natural language + optional context.
- Output: workflow document.
- Loop: generate → lint → fix → conformance → iterate until stable.
- Publishes prompt templates per layer (kernel, governance, AI).

This is the reference impl for Claim A the way `wos-core` is the reference impl for Claim B.

### 5.5 Fixture corpus doubles as synthesis benchmark

Pair each fixture with a natural-language problem statement. The set becomes a workflow-synthesis benchmark tracked like SWE-bench tracks coding benchmarks. "Given requirement R, did the LLM + WOS toolchain produce a conformant workflow?" — track monthly.

### 5.6 Repositioning artifacts

- Update `README.md` to lead with the two-claim framing, not "AI-native" as a tagline.
- Publish a `POSITIONING.md` or ADR: "WOS is designed to be authored by LLMs and optionally executed with AI agents."
- Add a demo reel: "requirement → workflow in 30 seconds with zero hand-editing." Falsifiable, demonstrable, not a tagline.

---

## 6. Rollout Sequence

Ordered by dependency and cost:

1. **4.1 Extension fix** — half a day, unblocks vendors, ship this week.
2. **4.3 Precedence clause** — half a day, eliminates latent defect.
3. **4.2 Honest conformance numbers** — one week, prerequisite for #4.
4. **4.4 Split release trains** — two weeks, only after per-layer honesty.
5. **5.1 Schema description audit** — ongoing, can start in parallel with #1.
6. **5.2 Structured lint diagnostics** — one week, can start after #3.
7. **5.3 Trace-emitting conformance** — two weeks, stacks on #4.2.
8. **5.4 `wos-synth` crate** — multi-week, the flagship Claim A artifact.
9. **5.5 Synthesis benchmark** — ongoing, builds on existing fixture corpus.
10. **5.6 Repositioning docs** — anytime, but most impactful after #5.4 exists as a demo.

§4 total: ~one engineer-month. §5 total: ~one engineer-quarter.

---

## 7. What Was Wrong in My First Pass (For Context)

Initial critique recommended collapsing 18 schemas to 4, deleting drafts, demoting AI to informative, and merging companions. The maintainer correctly pushed back on two grounds:

1. **Fine-grained schemas are load-bearing for the loop methodology.** Coarse schemas = weaker T1/T2 signal. Retracted the "collapse" recommendation.
2. **AI-native is the differentiator, not scope creep.** Demoting the AI spec undermines the pitch. Retracted the "demote AI" recommendation — refined into the two-claim framing instead.

What survived from the first pass and remains valid:
- Extension-seam contradiction (§4.1) — a real bug.
- Conformance-reporting gap (§4.2) — a real truth-in-advertising issue.
- Dual-companion precedence gap (§4.3) — a real latent defect.
- Release-cadence mismatch (§4.4) — a real structural constraint on AI iteration velocity.

Lesson for future reviewers: the repo's granularity and layering are considered design under a specific methodology. Critique should engage with the methodology, not pattern-match to "this looks big."

---

## 8. Open Questions for the Maintainer

1. Is Claim A (LLM authoring) an accepted first-class goal, or an emergent property the repo should leave implicit?
2. Should `wos-synth` be in this repo or a sibling repo? (Coupling vs. clean separation.)
3. What's the release-versioning tool of choice — independent tags per layer, or a monorepo release manifest?
4. Which rules today are candidates for the `load-bearing` state on the graduation ladder? (Needed to seed the ladder.)
5. Is there an existing compat-matrix convention to adopt, or should we design one?

---

## 9. File Map Referenced

- Specs: `specs/kernel/spec.md`, `specs/companions/{lifecycle-detail,runtime}.md`, `specs/ai/*`, `specs/governance/*`, `specs/advanced/*`, `specs/profiles/*`, `specs/sidecars/*`
- Schemas: `schemas/wos-*.schema.json` (18 files)
- Crates: `crates/wos-core`, `crates/wos-lint`, `crates/wos-conformance`
- Matrices: `LINT-MATRIX.md`, `WOS-FEATURE-MATRIX.md`
- Drafts: `DRAFTS/wos-core-v{2..7}.md` + variants
- Roadmap: `enterprise-feature-gaps.md`, `enterprise-implementation-roadmap.md`, `TODO.md`
- Fixtures: `fixtures/{kernel,ai,governance,advanced,companions,profiles,sidecars,validation}`
