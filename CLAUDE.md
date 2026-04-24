# CLAUDE.md — WOS

Governance layer between Formspec (intake) and Trellis (integrity). Parent [`../CLAUDE.md`](../CLAUDE.md) carries stack-wide conventions (HIGH PRIORITY writing rule, dev philosophy, worktrees, Red-Green-Refactor, Test-Before-Fix, commit convention). This file carries only WOS-specific deltas and pointers.

## Read first

| For | Read |
|---|---|
| Behavioral interrupts before any task | [`../.claude/operating-mode.md`](../.claude/operating-mode.md) |
| Owner operating preferences | [`../.claude/user_profile.md`](../.claude/user_profile.md) |
| Stack vision + fully-populated WOS section | [`../.claude/vision-model.md`](../.claude/vision-model.md) |
| Platform decision register | [`../thoughts/specs/2026-04-22-platform-decisioning-forks-and-options.md`](../thoughts/specs/2026-04-22-platform-decisioning-forks-and-options.md) |
| Parent repo guide | [`../CLAUDE.md`](../CLAUDE.md) |
| Current tactical work | [`TODO.md`](TODO.md) |
| Signature Profile active track | [`T4-TODO.md`](T4-TODO.md) |
| Closed work | [`COMPLETED.md`](COMPLETED.md) |
| Conventions (three-section spec rubric) | [`CONVENTIONS.md`](CONVENTIONS.md) |
| Stream / compatibility / lint / feature matrices | [`RELEASE-STREAMS.md`](RELEASE-STREAMS.md), [`COMPATIBILITY-MATRIX.md`](COMPATIBILITY-MATRIX.md), [`LINT-MATRIX.md`](LINT-MATRIX.md), [`WOS-FEATURE-MATRIX.md`](WOS-FEATURE-MATRIX.md) |
| Case initiation cross-spec contract | [`../thoughts/adr/0073-stack-case-initiation-and-intake-handoff.md`](../thoughts/adr/0073-stack-case-initiation-and-intake-handoff.md) |

For public-facing stack framing, see [`../STACK.md`](../STACK.md) — lookup-only.

**Conflict resolution:** see [`../.claude/operating-mode.md`](../.claude/operating-mode.md).

## Identity

WOS is a JSON-native specification for sensitive workflows — benefits adjudication, permit reviews, fraud investigations, any process where a decision affects someone's rights. Two separable claims:

- **Claim A — LLM-authored workflows.** Workflows are structured data. Spec → schema → lint → conformance is the LLM's authoring loop. 18 schemas, 116 lint rules, rule-coverage conformance fixtures.
- **Claim B — Agents as first-class runtime actors.** Agents are declarable participants alongside humans and services, with autonomy levels, confidence gates, deontic constraints, drift monitoring. Disclosed via kernel `actorExtension` seam.

WOS does NOT replace the workflow engine. Targets Temporal / Restate / Camunda / Step Functions as execution substrates; the engine handles persistence, timers, crash recovery. WOS governs the transitions that matter for rights, audit, and AI oversight.

Ships as four independent release streams: `wos-kernel`, `wos-governance`, `wos-ai`, `wos-advanced`. Compliance claims reference a pair of stream versions.

## Layer structure

- **Layer 0 — Kernel (required).** States, transitions, guards, case data, actors, relationships. Every transition emits provenance. Two conformant processors given the same kernel and same events produce the same result.
- **Layer 1 — Governance (optional).** Due process, five structured review protocols (independent-first, consider-opposite, calibrated confidence, dual-blind, unassisted), validation pipelines, delegation of authority, hold policies, authority-ranked reasoning traces.
- **Layer 2 — AI Integration (optional).** Agent registration with deontic constraints, autonomy levels capped by impact classification, confidence thresholds with decay, mandatory fallback chains terminating in human review, drift monitoring, disclosure requirements (EU AI Act Article 13, OMB M-24-10).
- **Layer 3 — Advanced Governance (optional).** DCR-style constraint zones, equity guardrails, SMT verification reports.
- **Cross-cutting profiles:** Integration, Semantic, Signature.
- **Companions:** Lifecycle Detail, Runtime.

Six kernel seams are the only extension surface: `actorExtension`, `attachmentExtension`, `caseFieldExtension`, `eventExtension`, `outcomeExtension`, `sidecarExtension`.

## Decision heuristics

Apply after stack-wide heuristics (in vision-model.md):

1. **Trellis-boundary check.** Cryptographic integrity / content-addressing / signed envelopes / checkpoint seals / export bundles / federation → Trellis. Do not invent WOS-side primitives. WOS emits `SignatureAffirmation` and other provenance records; Trellis anchors them through `custodyHook`.
2. **Scope to SBA + SaaS (Q1).** Before adding 1.0 work, ask: does this directly serve SBA PoC or public SaaS within its first year? If no, defer (trigger-gate or out-of-scope).
3. **Named-seams invariant.** New extension points live at one of the six kernel seams or use `x-` patternProperties. Inventing new seams is a Q3 violation.
4. **Module-bottleneck sequencing.** Before piling work onto a bottleneck file (e.g., `provenance.rs` pre-tier-split), sequence the structural refactor first.

## Key rules

- **Case initiation.** WOS owns governed case identity and `case.created`. Formspec may start an intake session and hand off validated public intake via `IntakeHandoff` (ADR 0073). Reference parser lives in `crates/wos-formspec-binding`.
- **Signature shortcut rule.** Product shortcuts may exist only as workflow-lite paths over the same WOS `SignatureAffirmation` semantics and Trellis custody/export path. Do not create a second meaning of "signed."
- **FEL is the only expression language.** FEEL / DMN / SHACL are on the rejection list. FEL drives guards, equity expressions, condition events, restricted-domain equity profile.
- **Rust is the spec authority.** `wos-core` is the semantics library; `wos-runtime` is in-memory durable-execution adapter + conformance oracle; Restate is initial default production adapter behind `DurableRuntime`. Do not add spec behavior in a scripting layer when it belongs in the Rust center.

## Architecture

- **Center:** `wos-core` + `wos-runtime` (semantics + in-memory oracle).
- **Trait:** `DurableRuntime` — the line between spec-authoritative semantics and adapter-tier orchestration.
- **Adapters:** in-memory (dev/test + conformance oracle), Restate (initial default reference adapter), Temporal (alternate/future), Camunda / Step Functions (trigger-gated).

New runtime capabilities MUST be implementable in the in-memory adapter AND the production adapter; conformance fixtures pass against both. Three-way agreement (spec + reference + production adapter) is the verification posture.

**FEL reuse.** WOS uses FEL via `fel-core` from the parent monorepo (`../crates/fel-core`). No alternative expression language.

**Provenance architecture.** Records are tiered (`ProvenanceKind` tier-typing, WOS-T1 closed). Every WOS MUST that produces an audit event emits a provenance record. The exporter (`wos-export`) packages records into `custodyHook` four-field append shape for Trellis ingestion. Trellis anchors; WOS emits.

## Spec authoring contract

- Use `formspec-specs:wos-expert` / `formspec-specs:wos-spec-author` skills for normative lookups.
- Structural truth lives in `schemas/*.json`.
- Behavioral semantics that schemas cannot encode live in `specs/**/*.md`.
- Every new or materially revised spec needs the three sections in `CONVENTIONS.md`: Normative Contract, Composition, Conformance.
- Sidecar independence earns independence — apply the three-question rubric before ratifying a new sidecar.
- Nodes marked `x-lm.critical=true` MUST include both `description` and at least one `examples` entry.

## Build & test

```bash
# Targeted
cargo check --workspace
cargo test -p wos-core --lib
cargo test -p wos-runtime --lib
cargo test -p wos-lint
cargo test -p wos-conformance
cargo test -p wos-conformance --test signature_profile   # Signature Profile suite

# Python schema-conformance
python3 -m pytest tests/schemas -q

# Full workspace
cargo test --workspace
```

Workspace depends on `fel-core` at `../crates/fel-core`. Normally checked out as `formspec/wos-spec/` inside parent Formspec repo.

**Coverage ratchets (CI gates):** `schema_doc_zero_regression`, `every_promoted_*_rule_has_executable_or_annotated_evidence`, `every_load_bearing_conformance_rule_has_at_least_two_executable_fixtures`, `discover_and_report_promotion_candidates`.

## Submodule awareness

Checked out as `formspec/wos-spec/` inside parent repo. Commits here are separate; bump parent submodule pointer when landing meaningful work. Never `--amend`, `--force`, or `--no-verify` without owner sanction. AI-authored commits end with:

```
Co-Authored-By: Claude <noreply@anthropic.com>
```
