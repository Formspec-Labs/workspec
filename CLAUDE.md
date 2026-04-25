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

## Schema structure

One author-time core schema, three sidecars, two runtime artifact schemas, one tooling schema. See [ADR 0076 (product-tier consolidation)](../thoughts/adr/0076-product-tier-consolidation.md).

- **Author-time core: `wos-workflow.schema.json`.** Required: `$wosWorkflow`, `url`, `version`, `title`, `impactLevel`, `actors`, `lifecycle`. Carries the workflow lifecycle, case file, contracts, output bindings, and provenance config in one document.
- **Optional embedded blocks** (appear when product behavior demands them; conditional schema rules + lint enforce presence):
  - `governance` — due process, review protocols, validation pipelines, task catalog, delegation, holds, policy parameters, escalation. Required for `rightsImpacting` and `safetyImpacting` workflows.
  - `agents[]` — per-agent declarations: model identity, autonomy (capped by impact), deontic constraints (`permission`/`prohibition`/`obligation`/`right` per OASIS LegalRuleML), confidence floor with decay, fallback chain terminating in human review, capabilities, drift monitoring. Required when any actor has `type == "agent"`.
  - `aiOversight` — disclosure (EU AI Act Art. 13, OMB M-24-10), drift detection, volume constraints, narrative-tier templates. Paired with `agents`.
  - `signature` — signers, order, identity verification, consent, reminders, void conditions, audit certificate. Required when any transition gates on `kind: "signature"` (signing order is load-bearing for DocuSign-tier workflows).
  - `custody` — Trellis trust-profile binding, per-transition or per-signature-event anchor requirements, export-bundle reference. Load-bearing whenever a workflow claims anchoring.
  - `advanced` — DCR constraint zones, equity guardrails, SMT verifiable constraints, circuit breaker, shadow mode.
  - `assurance` — assurance level, attestation, subject continuity.
- **Sidecars (deployment-environment configuration; join by `targetWorkflow` URI):**
  - `wos-delivery.schema.json` — business calendar, notification templates, correspondence metadata.
  - `wos-ontology-alignment.schema.json` — JSON-LD `@context`, SHACL shapes, PROV-O / XES / OCEL export.
- **Runtime artifacts (produced by processors):** `wos-case-instance.schema.json` (running-instance state), `wos-provenance-log.schema.json` (append-only audit log).
- **Tooling:** `wos-tooling.schema.json` (lint diagnostics, conformance traces, synth traces, MCP tool catalog, extension registry, authoring-tool view definitions).

Single top-level version marker: `$wosWorkflow`. Stream identity (governance, agents, signature, custody, advanced) is implicit in the workflow envelope version; compliance claims compose as `$wosWorkflow@X.Y`. The historical "we comply with `wos-kernel@1.0 + wos-governance@1.1`" four-stream form translates to "`$wosWorkflow@1.0`" plus a one-paragraph claims-map in `RELEASE-STREAMS.md` enumerating which embedded blocks are exercised. T4 signature, governance, AI deontic, advanced equity conformance suites stay operationally separate but run against the workflow envelope at the claimed version.

Specs do not physically merge: `kernel/spec.md`, `governance/spec.md`, `ai/ai-integration.md`, `advanced/spec.md` stay as separate documents with existing §-numbering preserved (citations like "Kernel §10.3" remain valid). Only the schema references inside each spec update to `wos-workflow.schema.json`.

Six canonical kernel seams remain the only extension surface (ADR 0077): `actorExtension`, `contractHook`, `provenanceLayer`, `lifecycleHook`, `custodyHook`, `extensions` / `x-` keys. Inlining governance, agents, signature, and advanced into the core schema does not alter how higher-layer concerns attach.

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
- **Reference-server auth invariants.** Global logout bumps `auth_epoch` + revokes sessions in one txn; password rotation flows only through `Storage::set_user_password_hash` (hash + epoch + revoke atomic); `upsert_user` never touches `password_hash` / `auth_epoch`; realtime `kernel:update` re-runs `verify` per event so role/revoke changes apply without waiting for token expiry. Full contract: [`crates/wos-server/PARITY.md`](crates/wos-server/PARITY.md) ▎ Auth contract, mirrored in [`crates/wos-server/README.md`](crates/wos-server/README.md) Auth + Storage + Realtime auth model.

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
