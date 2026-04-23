# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working on the WOS (Workflow Orchestration Standard) spec — the **governance layer** of the three-spec stack.

## HIGH PRIORITY — Writing backlog / TODO / task items

**Every backlog entry, TODO, or task description MUST carry its own context.** A reader (human or agent) opening the item cold — no surrounding conversation, no memory of the session that produced it — must know *what the work is*, *why it matters*, and *what "done" looks like*, from the words on the page alone.

Write dense, not verbose. The model is a poem or a well-contextualized meme: few words, heavy payload, still easy to read. Every sentence pulls weight — if a phrase can be cut without losing meaning, cut it; if a phrase that looks redundant is actually the anchor that makes the rest make sense, keep it. No orphan pronouns, no "see above", no "the thing we discussed" — name the thing.

**The test:** if this item sat untouched for six weeks and a different agent picked it up, could they act on it without asking a clarifying question? If no, rewrite until yes.

Applies to `TODO.md`, `T*-TODO.md`, plan files in `thoughts/plans/`, ADR follow-ups, lint-rule backlog entries, conformance-fixture stubs, and any inline `// TODO` comments that escape a single session.

## Project Overview

WOS is the governance layer between **Formspec** (intake) and **Trellis** (integrity). It is a JSON-native specification for sensitive workflows — benefits adjudication, permit reviews, fraud investigations, any process where a decision affects someone's rights. It defines what protections apply, what constraints bind AI agents, what the audit trail must contain, and what the reasoning was behind each determination.

WOS ships as **four independent release streams**: `wos-kernel`, `wos-governance`, `wos-ai`, `wos-advanced`. Compliance claims reference a pair of stream versions (e.g. `wos-kernel@1.0 + wos-ai@0.5`). See [`RELEASE-STREAMS.md`](RELEASE-STREAMS.md) and [`COMPATIBILITY-MATRIX.md`](COMPATIBILITY-MATRIX.md).

Two separable claims:

- **Claim A — LLM-authored workflows.** Workflows are structured data. The spec → schema → lint → conformance loop is the LLM's authoring loop. 18 schemas, 116 lint rules, and rule-coverage conformance fixtures make signal precise enough to author against.
- **Claim B — Agents as first-class runtime actors.** When the workflow runs, agents are declarable participants alongside humans and services, with autonomy levels, confidence gates, deontic constraints, and drift monitoring. Disclosed via the kernel `actorExtension` seam.

WOS does NOT replace the workflow engine. It targets Temporal / Restate / Camunda / Step Functions as execution substrates; the engine handles persistence, timers, crash recovery. WOS governs the transitions that matter for rights, audit, and AI oversight.

At stack end state, WOS contributes governance truth to one portable case record. Durable engines orchestrate; they do not become evidentiary truth. Retries, stalls, resumes, compensation, human overrides, AI recommendations, signature affirmations, and policy-relevant transitions must remain exportable through provenance and Trellis custody when they affect the case.

## Operating Context — READ THESE BEFORE DECIDING

WOS is one spec in a three-spec stack. Architectural decisions routinely cross spec boundaries. Consult in this order before any non-trivial decision:

1. **[`../.claude/user_profile.md`](../.claude/user_profile.md)** — Owner's operating preferences. Economic model (minutes-not-days × Imp × Debt); design philosophy (opinionated, closed taxonomies, named seams); communication style (terse, opinionated, hedges labeled); and the **maximalist one-shot delivery** rule — no stubs, no `TODO: implement later`, no placeholder returns. If AI builds it, it ships complete and working in one pass; iterate on working code, not half-built code. Surface blockers instead of papering over with stubs.
2. **[`../.claude/vision-model.md`](../.claude/vision-model.md)** — Stack-wide vision captured 2026-04-20. The **WOS section is fully populated**: settled architectural commitments, v1.0 scope snapshot, active uncertainties (α DocuSign parity bar, γ durable-execution backend, δ `wos-runtime` role), and WOS-specific decision heuristics (Trellis-boundary check, SBA+SaaS scoping, named-seams invariant, module-bottleneck sequencing). Consult before any decision that crosses subsystems, spec boundaries, or re-opens a foundational question.
3. **[`../STACK.md`](../STACK.md)** — Public-facing integrative doc covering the three-spec stack and the five cross-layer contracts. Canonical source for how Formspec + WOS + Trellis compose.
4. **[`../thoughts/specs/2026-04-22-platform-decisioning-forks-and-options.md`](../thoughts/specs/2026-04-22-platform-decisioning-forks-and-options.md)** — Active platform decision register for end-state commitments, implementation leans, forks, kill criteria, and organizational/product constraints. Consult before changing durable-runtime assumptions, signing semantics, custody/export behavior, product-vs-engineering proof claims, or ledger-visible workflow truth.
5. **[`../CLAUDE.md`](../CLAUDE.md)** — Parent repo guide. Filemap conventions, TypeScript tier, Python tier, worktree rules, and the Formspec-side spec authoring contract apply wherever cross-spec work touches the parent tree.

**Conflict resolution:** direct owner signals in the current conversation > these docs > this CLAUDE.md > generic defaults. If any of these docs conflicts with owner signals, update the doc — don't work around it.

## Foundational answers (stack Q1-Q4, specialized for WOS)

From the vision model:

- **Q1 First adopter:** SBA (DocuSign + Adobe Forms replacement, rights-impacting) + public SaaS (Jotform competitor with AI-governance + verifiable signature ledger). WOS governs the transitions that matter for both.
- **Q2 Spec-runtime authority:** Co-authoritative. Default spec-led; runtime feedback propagates back. Spec + runtime land in the same commit-series.
- **Q3 Opinionated:** Few right ways to do things. Extension points bounded. Rejection list is a feature. One mechanism per concern (FEL for expressions, not FEL+FEEL+SHACL). Center-vs-adapter is the native frame.
- **Q4 Verifiability threshold:** Reference implementation is the oracle. Every normative MUST at 1.0 has a passing Tested fixture. Conformance runs against every durable-execution adapter.

## Development Philosophy — READ THIS FIRST

**Code is cheap. Time is cheap. Good architecture is invaluable.** Pre-release, no users, no backwards-compatibility obligation. Architecture decisions compound; implementations within clean seams are cheap to redo.

**Write code for humans first.** Every crate, module, and function should be immediately legible. Names reveal intent. Comments explain *why*, never *what*. With AI the cost of clean code equals the cost of messy code — always choose clean.

**Prioritize by value added.** Before spending effort, ask: does this close a 1.0 scope item, unlock adapter parallelism, or directly serve the SBA/SaaS product stack within its first year? If not, deprioritize.

**All code is ephemeral.** Prefer rewrites over refactors when something is fighting us. Learn, then rebuild.

- **Architecture over code** — spend thinking time on seams, traits, data flow, provenance tiers, kernel extension points.
- **Delete, don't preserve** — no legacy, no users to migrate. Wrong code gets thrown away, not patched.
- **KISS always** — fewer lines = fewer bugs = faster iteration on the spec surface.
- **Right-sized files** — one coherent concept per file. `provenance.rs` splitting into tier modules (#22a) is the template for bottleneck relief.
- **DRY when natural** — three similar lines are better than one confusing helper. With AI, duplicate clear code is near-free; a bad abstraction is paid every time it's read.
- **Extensibility where the spec demands it** — the six kernel seams (`actorExtension`, `attachmentExtension`, `caseFieldExtension`, `eventExtension`, `outcomeExtension`, `sidecarExtension`) are the only extension surface. Inventing new seams is a Q3 violation.
- **The spec is the source of truth** — do NOT implement behavior in Rust that the spec doesn't describe, and do NOT describe spec behavior the Rust can't verify. Normative MUSTs get Tested fixtures.
- **No "defer" on greenfield** — audit finds something wrong, fix it. No "fix later" tags. The cost of fixing now is the lowest it will ever be.
- **Maximalist one-shot delivery** — ship complete. Stubs / `unimplemented!()` / `todo!()` / `NotImplementedError` are forbidden unless the blocker is an unresolved architectural decision, in which case STOP and surface it.

## Three-spec layering — what WOS owns vs. doesn't

| Concern | Layer | Owner |
|---|---|---|
| Form fields, FEL, validation, response shape | Intake | **Formspec** |
| Canonical response (Formspec → WOS) | Seam 1 | Formspec declares; WOS consumes |
| Governance coprocessor (WOS ↔ Formspec prefill/validate + intake handoff) | Seam 2 | WOS + Formspec jointly |
| Lifecycle, transitions, actors, case data | Governance — Kernel | **WOS** |
| Due process, review protocols, deontic rules, provenance, signature workflow | Governance — L1/L2/L3 | **WOS** |
| Governance custody hook (WOS → Trellis) | Seam 5 | **WOS** declares the record; Trellis anchors |
| Event hash chain, content-addressed envelopes, signed events | Integrity | **Trellis** |
| Checkpoint seals, transparency-log anchoring, export bundles | Integrity | **Trellis** |
| Certificate-of-completion PDF artifact | Integrity | **Trellis** |
| Merkle provenance chains, SCITT alignment, Federation Profile | Integrity | **Trellis** |

**Trellis-boundary check (first heuristic):** is the question about cryptographic integrity, content-addressed storage, signed envelopes, checkpoint seals, export bundles, or federation/transparency logs? If yes — Trellis concern. Don't invent WOS-side primitives. WOS emits `SignatureAffirmation` and other provenance records; Trellis anchors them through `custodyHook`.

**Case initiation rule:** WOS owns governed case identity and `case.created`. Formspec may start an intake session and hand off validated public intake, but it does not emit the governed case boundary. Support both workflow-initiated and public-intake-initiated routes via the accepted `IntakeHandoff` contract in [ADR 0073](../thoughts/adr/0073-stack-case-initiation-and-intake-handoff.md); the reference parser/classifier lives in `crates/wos-formspec-binding`, and WOS provenance has a factual `caseCreated` constructor with `event = "case.created"`.

**Signature shortcut rule:** product shortcuts may exist only as workflow-lite paths over the same WOS `SignatureAffirmation` semantics and Trellis custody/export path. Do not create a second meaning of "signed" in product code, intake-only code, or exporter glue.

## Layer structure

WOS has one required layer and three optional ones. Cross-cutting profiles and companions attach without new kernel extension points.

- **Layer 0 — Kernel (required).** States, transitions, guards, case data, actors, relationships. Every transition emits provenance. Two conformant processors given the same kernel and the same events produce the same result.
- **Layer 1 — Governance (optional).** Due process, five structured review protocols (independent-first, consider-opposite, calibrated confidence, dual-blind, unassisted), validation pipelines, delegation of authority, hold policies, authority-ranked reasoning traces. Where most of the invention lives.
- **Layer 2 — AI Integration (optional).** Agent registration with deontic constraints, autonomy levels capped by impact classification, confidence thresholds with decay, mandatory fallback chains terminating in human review, drift monitoring, disclosure requirements (EU AI Act Article 13, OMB M-24-10).
- **Layer 3 — Advanced Governance (optional).** DCR-style constraint zones, equity guardrails, SMT verification reports.
- **Cross-cutting profiles:** Integration (external APIs, OpenAPI/Arazzo, CloudEvents, OPA/Cedar); Semantic (JSON-LD, PROV-O, SHACL, XES); **Signature** (signer roles, flows, intent capture, `SignatureAffirmation` emission — workflow semantics only; crypto lives in Trellis).
- **Companions:** Lifecycle Detail (evaluation order, nested entry/exit, parallel regions, compensation, history resumption, SCXML mapping); Runtime (case instance serialization, event delivery contract, Formspec coprocessor handoff).

## Repo structure

- **`specs/`** — Normative markdown specs organized by layer (`kernel/`, `governance/`, `ai/`, `advanced/`, `profiles/`, `sidecars/`, `companions/`). Canonical source of behavioral semantics.
- **`schemas/`** — JSON Schema files mirroring `specs/` structure. Structural truth.
- **`crates/`** — Rust workspace (`resolver = "3"`):
  - **`wos-core`** — typed models, lifecycle evaluation, deontic rules, provenance, contract ordering. Semantics library.
  - **`wos-lint`** — static analysis; 116 rules across three tiers, all with test witnesses. See [`LINT-MATRIX.md`](LINT-MATRIX.md).
  - **`wos-conformance`** — dynamic scenario runner; JSON test fixtures drive the runtime and assert correct behavior.
  - **`wos-runtime`** — orchestration layer; persistence, queues, simulated time, milestone evaluation. The `DurableRuntime` trait extracts below the center-vs-adapter line; Restate is the initial default reference adapter, while Temporal and other engines remain replaceable adapter choices behind the same trait.
  - **`wos-formspec-binding`** — Formspec coprocessor; prefill, response validation, mapping form data into case state. Seam 2 implementation.
  - **`wos-export`** — exporter; provenance → Trellis `custodyHook` records.
  - **`wos-authoring`**, **`wos-mcp`**, **`wos-synth-*`** — MVP authoring/tooling surfaces.
- **`tests/`** — Python schema-conformance tests (pytest + jsonschema).
- **`fixtures/`** — Conformance fixture library.
- **`thoughts/`** — ADRs, plans, practices, reviews (mirrors parent convention). Next free ADR id lives in parent [`thoughts/README.md`](../thoughts/README.md).
- **`changelogs/`** — Per-stream changelogs.
- **`TODO.md`** — Active backlog. **[`COMPLETED.md`](COMPLETED.md)** holds session narratives and closed items.
- **`T4-TODO.md`** — Active per-track execution plan for Signature Profile closeout. Closed track narratives belong in [`COMPLETED.md`](COMPLETED.md).
- **Operational docs:** [`CONVENTIONS.md`](CONVENTIONS.md), [`POSITIONING.md`](POSITIONING.md), [`RELEASE-STREAMS.md`](RELEASE-STREAMS.md), [`COMPATIBILITY-MATRIX.md`](COMPATIBILITY-MATRIX.md), [`LINT-MATRIX.md`](LINT-MATRIX.md), [`WOS-FEATURE-MATRIX.md`](WOS-FEATURE-MATRIX.md), [`WOS-IMPLEMENTATION-STATUS.md`](WOS-IMPLEMENTATION-STATUS.md).

## Architecture

### Logic ownership — Rust is the spec authority

WOS business logic lives in Rust crates. The `wos-core` crate is the semantics library; `wos-runtime` is the in-memory durable-execution adapter and conformance oracle; Restate is the initial default reference adapter below the `DurableRuntime` trait. Temporal, trigger-gated Camunda / Step Functions, or another engine may implement the same trait later. Do NOT add spec behavior in a scripting layer when it belongs in the Rust center; extend `wos-core` and expose it through the trait.

### FEL reuse

WOS uses **FEL** (Formspec Expression Language) via `fel-core` from the parent monorepo (`../crates/fel-core` when this tree is `formspec/wos-spec`). No alternative expression language — FEEL / DMN / SHACL are all on the rejection list. FEL drives guards, equity expressions, condition events, and the restricted-domain equity profile. One grammar per concern.

### Provenance architecture

Provenance records are tiered (`ProvenanceKind` tier-typing, WOS-T1 closed). Every WOS MUST that produces an audit event emits a provenance record. The exporter (`wos-export`) packages records into `custodyHook` four-field append shape for Trellis ingestion. Trellis anchors; WOS emits.

### Center-vs-adapter discipline

- **Center:** `wos-core` + `wos-runtime` (semantics + in-memory oracle).
- **Trait:** `DurableRuntime` (below runtime; the line between spec-authoritative semantics and adapter-tier orchestration).
- **Adapters:** in-memory (dev/test + conformance oracle), Restate (initial default reference adapter), Temporal (alternate/future), Camunda / Step Functions (trigger-gated).

New runtime capabilities MUST be implementable in the in-memory adapter AND the production adapter; conformance fixtures pass against both. Three-way agreement (spec + reference + production adapter) is the verification posture.

## Spec Authoring Contract

- **Use the `formspec-specs` skill family** for normative spec lookups across the whole stack. Invoke via `Skill` tool with `skill: "formspec-specs:formspec-specs"` for Formspec, or the WOS-specific skills (e.g. `formspec-specs:wos-expert`, `formspec-specs:wos-spec-author`) for WOS. Do not guess from Rust code — the skills have authoritative spec knowledge.
- Structural truth lives in `schemas/*.json`.
- Behavioral semantics that schemas cannot encode live in canonical spec markdown (`specs/**/*.md`).
- Every new or materially revised spec MUST include the three sections enforced by [`CONVENTIONS.md`](CONVENTIONS.md):
  1. **Normative Contract** — explicit MUST/SHOULD/MAY, each enforced by schema / lint / conformance fixture, or explicitly flagged as a gap with tracking ID.
  2. **Composition** — attachment point, precedence, conflict handling, versioning/migration rules, named seams.
  3. **Conformance** — enumerates schema / lint / runtime coverage; every non-trivial normative behavior has at least one executable fixture.
- Sidecar independence earns independence — apply the three-question rubric in `CONVENTIONS.md` (Structure, Semantics, Composition) before ratifying a new sidecar.
- Nodes marked `x-lm.critical=true` in schemas MUST include both `description` and at least one `examples` entry.

## Build & test commands

```bash
# Targeted gates (run whichever applies to what changed)
cargo check --workspace
cargo test -p wos-core --lib
cargo test -p wos-runtime --lib
cargo test -p wos-lint
cargo test -p wos-conformance
cargo test -p wos-conformance --test signature_profile   # Signature Profile suite

# Python schema-conformance
python3 -m pytest tests/schemas -q

# Full workspace tests
cargo test --workspace
```

**Dependencies:** the workspace depends on `fel-core` at `../crates/fel-core`. This tree is normally checked out as `formspec/wos-spec` inside the parent Formspec repo.

**Coverage ratchets (CI gates):**

- `schema_doc_zero_regression`
- `every_promoted_*_rule_has_executable_or_annotated_evidence`
- `every_load_bearing_conformance_rule_has_at_least_two_executable_fixtures`
- `discover_and_report_promotion_candidates`

## v1.0 scope snapshot

"v1.0" is a coherent-state label, not a freeze. Nothing is released. If a change prevents future architectural debt — kernel shape, seam surface, provenance tier model, governance semantics — make it, even if current specs or schemas already describe something different. The only expensive debt is debt we'd have to unwind after adopters show up, and we have no adopters.

Current 1.0 scope, active uncertainties, and trigger-gated items are canonically listed in [`../.claude/vision-model.md`](../.claude/vision-model.md) under the `## WOS` section. Do NOT duplicate that list here — update the vision model instead.

Highlights (see vision model for full list):

- **Must close:** Kernel closure (#20, #F3b §10.3 Tasks 1/2/4/5, #22a provenance tier-typing **closed**, cross-reference shape ADR); §4.5 structural merges (owner decision pending); durable-execution trait plus Restate as the initial default reference adapter; `custodyHook` shape (**WOS-T1 closed**); Signature Profile (**WOS-T4 active**); every normative MUST has a passing Tested fixture.
- **Trigger-gated:** additional engine adapters (Camunda, Step Functions), SCXML interop, additional statutory-deadline profiles beyond the shared stack clock contract.
- **NOT WOS scope (Trellis territory):** Merkle provenance chains, full SCITT strictness, Federation Profile, checkpoint seals, transparency-log submission, certificate-of-completion export bundle format.

## WOS-specific decision heuristics

Apply after stack-wide heuristics (see [`../.claude/vision-model.md`](../.claude/vision-model.md) § "Stack-wide decision heuristics"):

1. **Trellis-boundary check.** Cryptographic integrity / content-addressing / signed envelopes / checkpoint seals / export bundles / federation → Trellis. Don't invent WOS-side primitives.
2. **Scope to the SBA + SaaS product stack (Q1).** Before adding 1.0 work, ask: does this directly serve SBA PoC or public SaaS within its first year? If no, defer (trigger-gate or out-of-scope).
3. **Named-seams invariant.** New extension points live at one of the six kernel seams or use `x-` patternProperties. Inventing new seams is a Q3 violation.
4. **Module-bottleneck sequencing.** Before piling work onto a bottleneck file (e.g., `provenance.rs` pre-tier-split), sequence the structural refactor first.

## Development Workflow — Red-Green-Refactor

Every feature or bugfix follows this loop. Do NOT write implementation before a failing test exists.

1. **Red** — Write one minimal failing test (unit / lint-rule / conformance fixture / schema assertion — whichever layer the behavior lives at). Run it, confirm it fails for the right reason.
2. **Green** — Make it pass with the simplest change that works.
3. **Expand** — Add tests for edge cases and the full requirement.
4. **Verify** — Run the full relevant suite (`cargo test -p ...` + `pytest tests/schemas`) to confirm zero regressions.

**Test locations:**

- `crates/wos-core/tests/` — core semantics.
- `crates/wos-lint/tests/` — lint rule witnesses.
- `crates/wos-conformance/tests/` — scenario fixtures.
- `crates/wos-runtime/tests/` — orchestration behavior.
- `crates/wos-formspec-binding/tests/` — coprocessor integration.
- `tests/schemas/` — Python JSON Schema conformance.
- `fixtures/` — scenario inputs and expected outputs.

## Code Review Workflow — Test Before Fix

When review identifies a bug: write a failing test FIRST, then fix, then expand coverage, then verify full suite. Every bug — correctness, safety, silent drift, off-by-one — gets a test as proof it existed and proof it's fixed. A fix without a test is an unverified claim.

## Commit Convention

Use semantic prefixes: `feat:`, `fix:`, `build:`, `docs:`, `test:`, `refactor:`. Commit at logical stopping points — a passing suite, a complete bugfix, a self-contained refactor. Not mid-refactor, not after one file of a multi-file change. Each commit is a meaningful, self-contained unit.

**Co-Author footer when AI-authored:**

```
Co-Authored-By: Claude <noreply@anthropic.com>
```

**Never** use `--amend`, `--force`, or `--no-verify` unless explicitly sanctioned by the owner. No commits on behalf of someone else.
