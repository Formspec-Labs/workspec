# WOS Vision Model

**Captured:** 2026-04-20
**Scope:** Project-specific operating guidance for WOS (the governance layer in the Formspec + WOS + Trellis three-spec stack). Generic user preferences live in [`user_profile.md`](../../../.claude/user_profile.md); read it alongside this document. Trellis-specific vision lives in the Trellis repo (see [Relationship to Trellis](#relationship-to-trellis) below).
**Maintenance rule:** update only when the owner gives explicit signals that conflict with current content. Do NOT update speculatively. Treat each answer as frozen until the owner overrides.

---

## Read this first

This document captures the WOS-specific vision model the owner validated on 2026-04-20, after explicitly requesting that existing written content (specs, plans, ADRs) not be trusted by default — everything was written by AI.

**When to consult:**
- Before making any architectural decision that crosses more than one WOS subsystem.
- When a new WOS design question arises that isn't directly answered by existing specs.
- When re-reading TODO.md or plan docs after a context break.
- Before dispatching parallel agents on load-bearing WOS work.

**When NOT to use:**
- For local tactical decisions inside a single file/function — just follow the code.
- For questions about Trellis primitives (content-addressed storage, signed event envelopes, checkpoint seals, export bundles, SCITT transparency log) — those are Trellis-vision territory.
- As a substitute for asking the owner when a genuinely new question arises.

**Paired with [`user_profile.md`](../../../.claude/user_profile.md):** that file captures generic operating principles (economic model, opinionatedness preference, communication style, authoring workflow, collaboration heuristics, technology preferences). This file captures WOS-specific architectural commitments. Apply both together; when they conflict, direct owner signals override both.

---

## The foundational answers (canonical signals)

Four questions fixed the vision model. Owner's answers in their own words; my interpretation flagged.

### Q1 — First adopter

**Owner's answer (verbatim):**
> "the SBA where they just want to start out with docusign + adobe forms replacement proof of concept + we want to do a public SaaS to replace jotforms/googleforms whatever and include the AI agent stuff and also the validation ledger for docusign stuff so people could, in theory, do free docusign using our ledger, technically, and/or have a shared ledger system thing?"

**Interpretation (mine, labeled):**

- **Dual adopter:** SBA (Small Business Administration) as a specific first customer, plus a public SaaS aimed at the consumer/SMB form-builder market.
- **SBA PoC scope:** DocuSign + Adobe Forms replacement for government workflows. Rights-impacting context; due-process machinery is load-bearing.
- **SaaS scope:** competes with Jotform/Google Forms/Typeform. Differentiators: (a) AI-agent governance built-in, (b) cryptographically-verifiable signature ledger ("free DocuSign via our ledger") — **note:** the ledger itself is a Trellis concern; WOS emits the provenance records Trellis anchors, (c) shared trust-anchor network across adopters — **also Trellis.**
- **AI integration is product-central, not seam-optional.** For the consumer SaaS, AI agents are the feature.
- **Signature workflow is a load-bearing pattern for WOS.** DocuSign parity at the workflow-semantics level — multi-party routing, intent capture, evidence emission. Cryptographic integrity of that evidence is Trellis; WOS handles the workflow.

### Q2 — Spec vs. runtime authority

**Owner's answer (verbatim):**
> "it's supposed to be spec-led, but this is a non-deterministic process, and often, real-world runtime changes drive progress and need to be integrated, so C" (co-authoritative).

**Interpretation:**

- **Default is spec-led.** When the runtime doesn't implement what the spec says (like F3b continuous-mode §10.3), the runtime is fixed to match.
- **But drift is bidirectional and managed.** When the runtime discovers semantics the spec didn't capture, the spec is updated.
- **Both sides are reviewed together.** The session-8/9 pattern — spec prose + runtime code + tests landing in one PR with semi-formal review — is canonical.
- **"Closed" is closed-as-of-a-date, not permanently.** v1.0 means "closed at release; drift managed in 1.x."

### Q3 — Opinionatedness

**Owner's answer:** "A" — opinionated / principled.

**Interpretation (WOS-specific consequences; generic opinionated-design preference is in [`user_profile.md`](../../../.claude/user_profile.md)):**

- **No `named: string` escape hatch at core taxonomy keys.** The 5-kind event taxonomy is closed; vendors extend via payload `x-*` fields.
- **Rejection list is load-bearing.** DAG processing, FEEL, SHACL, BPMN parity as authoring goal, FEL conformance profiles — all rejected. Don't re-litigate.
- **Named seams only.** Six kernel seams (`actorExtension | contractHook | provenanceLayer | lifecycleHook | custodyHook | extensions`) plus `x-` patternProperties. Nothing else.
- **Single source of authority.** Registry conflicts at load time; no declaration-order precedence.
- **FEL only.** No alternate expression languages at any spec layer.
- **Center vs. adapter is the native frame.** The center declares the shape; adapters implement it. Work is split along that line rather than along feature lines.

### Q4 — Verifiability threshold

**Owner's answer (verbatim):**
> "the entire point of the reference architecture is for us to use it and also for it itself to be a way to test/validate the spec, not sure where that lands"

**Interpretation:**

- **The reference implementation is the oracle.** Spec MUSTs are verified by: (i) constructing a fixture that exercises the claim; (ii) running it against the reference implementation; (iii) asserting the spec's predicted outcome.
- **Every MUST gets a passing fixture at 1.0.** Under the user's minutes-not-days economics, closing the current 12/91 Tested/Draft gap is cheap and high-value for the reference-architecture claim.
- **Conformance runs against every durable-execution adapter.** In-memory reference + production adapter (Temporal or Restate) both pass. Three-way agreement (spec + reference + production adapter) is the strongest attainable verification posture.

---

## Relationship to Trellis

WOS is the **governance layer**; Trellis is the **integrity layer**. They integrate through the `custodyHook` seam defined in Kernel §10 and already partially spec'd in the extension registry (session 6, commit `3550fad` included a Trellis custody shape).

**WOS owns:**
- Workflow lifecycle semantics (kernel, states, transitions, actions, events).
- Governance semantics (deontic constraints, due process, autonomy, oversight).
- AI integration semantics (agent actors, confidence, fallback chains, Narrative tier).
- Provenance record CONSTRUCTION — what records to emit, their fields, their timing.
- Signature workflow SEMANTICS — multi-party routing, intent capture timing, signer roles, evidence emission.

**Trellis owns (not in this vision model's scope):**
- Content-addressed storage over provenance records.
- COSE-signed event envelopes.
- Checkpoint seals / Merkle tree heads.
- Offline-verifiable export bundles (certificate-of-completion artifacts live here).
- SCITT-aligned transparency log + federation primitives.
- Cryptographic proof-of-inclusion / verification tooling.

**The seam:** WOS emits a provenance record (shape defined by the Kernel Provenance schema + tier refinements); the `custodyHook` delivers it to Trellis, which handles integrity. WOS does not know or care about Merkle roots, COSE envelopes, or transparency-log submission. Trellis does not know or care about FEL evaluation, deontic constraints, or AI autonomy policy.

**Practical consequences for the WOS vision:**
- **#48 Merkle provenance chains** — NOT a WOS concern. Moves to Trellis scope entirely.
- **SCITT alignment** (β uncertainty from prior pass) — NOT a WOS concern. Trellis decides SCITT strictness.
- **Federation Profile** (cooperative trust-anchor network) — NOT a WOS concern. Trellis spec.
- **Signature Profile in WOS** shrinks to workflow-semantics-only: signer roles as actor-extension, signing flow patterns as lifecycle tags, intent capture as a provenance event kind, evidence trail shape. The cryptographic integrity of those artifacts and the certificate-of-completion packaging are Trellis deliverables.
- **"Free DocuSign" product claim** rests on the combined WOS-emitted-events + Trellis-anchored-signatures stack. Neither spec claims the feature alone.

---

## Product stack (the three-spec stack)

Per [`user_profile.md`](../../../.claude/user_profile.md): **Formspec + WOS + Trellis** compose one stack for rights-impacting workflows.

| Spec | Layer | Responsibility | 1.0 status |
|---|---|---|---|
| **Formspec** | Intake | Fields, FEL, validation, behavior. JSON-native. | Parent monorepo; not this doc's scope. |
| **WOS** | Governance | Lifecycle, AI constraints, review protocols, provenance emission, deontic modalities. | Active; v1.0 closure in flight (see below). |
| **Trellis** | Integrity | Content-addressed signed events, checkpoint seals, offline-verifiable export bundles, SCITT federation. | Separate submodule; not this doc's scope. |

**Value proposition of the combined stack:** Jotform-tier forms UX + governed-AI-agents + cryptographically-verifiable signature ledger.

**Delivery vehicles:**
- **SBA PoC** — DocuSign + Adobe Forms replacement. Validates the end-to-end flow with a specific rights-impacting customer.
- **Public SaaS** — multi-tenant Jotform competitor. Commercializes the three-spec stack.

---

## Technical stack (layering commitment)

```
┌─────────────────────────────────────────────────────────┐
│  WOS semantics library  (wos-runtime — stays)           │
│  • Kernel evaluator (transitions, guards, case state)   │
│  • FEL engine (parse, evaluate, dependency extraction)  │
│  • Deontic constraint enforcement                       │
│  • Autonomy + confidence + fallback semantics           │
│  • Provenance record CONSTRUCTION (shape + content)     │
│  • Formspec coprocessor integration                     │
│  • Companion policies + Assurance + Assertion Library   │
│  • Signature workflow SEMANTICS (not cryptography)      │
│  • custodyHook delegation → Trellis                     │
├─────────────────────────────────────────────────────────┤
│  DurableRuntime trait  (the center-vs-adapter seam)     │
│  • start_workflow(instance_id) → handle                 │
│  • signal(handle, event) → Result                       │
│  • schedule_timer(deadline, event) → TimerHandle        │
│  • persist_case_state(state) → Result                   │
│  • await_resume() → Event                               │
├─────────────────────────────────────────────────────────┤
│  Adapters (the concrete sides of the seam)              │
│  ├─ in-memory        (dev/test, hermetic)               │
│  ├─ postgres-simple  (single-tenant, minimal ops)       │
│  ├─ temporal         (production option A, self-hosted) │
│  └─ restate          (production option B, self-hosted) │
└─────────────────────────────────────────────────────────┘
          │ custodyHook (provenance records out)
          ▼
┌─────────────────────────────────────────────────────────┐
│  Trellis (separate submodule; not this doc's scope)     │
│  • Content-addressed storage                            │
│  • COSE-signed event envelopes                          │
│  • Checkpoint seals / Merkle tree heads                 │
│  • Offline-verifiable export bundles                    │
│  • SCITT transparency log + federation                  │
└─────────────────────────────────────────────────────────┘
```

Everything in the WOS box is domain logic no durable-execution substrate can replace. The `DurableRuntime` trait is the center-vs-adapter seam for workflow execution. The `custodyHook` is the center-vs-adapter seam for integrity.

**Backend choice (γ):** owner prefers self-hosted; Rust-primary. Spike Temporal + Restate against the trait; pick based on Rust SDK ergonomics + ops fit. Neither is pre-chosen. Leaning Restate on first-principles grounds (Rust-first, simpler ops); not a commitment.

**`wos-runtime` long-term role:** semantics library + in-memory reference adapter + spec-conformance test oracle. The in-process durable-execution plumbing currently tangled into `wos-runtime/src/runtime.rs` (~4451 lines, mostly timer/retry/persistence) gets extracted below the trait line.

**Position in the 7-stage authoring workflow** (per [`user_profile.md`](../../../.claude/user_profile.md)): WOS is at stages 2-5 — formal spec + schemas + lint/conformance + runtime libraries — with Studio-style reference UI (stage 6) still ahead. The v1.0 line corresponds roughly to "stages 2-5 complete, stage 6 starting."

---

## Settled architectural commitments

Decisions the owner has signed off on (directly or via transitive implication from Q1-Q4 + the user's opinionated design preference):

**Event taxonomy (#20):**
- OQ1 — `$join` is engine-synthesized only. Authors catching a join write `{kind: "signal", scope: "instance", name: "$join"}`.
- OQ4 — 5-kind closed enum (`timer | message | signal | condition | error`); vendor extension via payload `x-*` fields, not a sixth `kind: "vendor"` variant.
- OQ2 — Flat event names at the kernel layer; hierarchical subscription is a registry-tier concern.
- OQ3 — Full §7 evaluation context minus `event` for condition-kind events.

**Cross-reference shapes (session-9 drift):**
- `<entity>Ref: URI` — cross-document references (sidecar URIs; `calendarRef`, `assertionRef`).
- `<entity>Key: string` — in-document local keys (map keys in the same sidecar).
- `<entity>Id: string` — in-document sibling-object references (id-bearing peers).
- ADR names the three patterns; doesn't try to unify them.

**Opinionated rejections:**
- DAG processing model — rejected (append-only event-stream folding only).
- FEEL / DMN expression language — rejected (FEL is purpose-built).
- SHACL — rejected (Rust lint covers cross-doc validation).
- BPMN parity as authoring goal — rejected (export target only).
- FEL conformance profiles — rejected (single grammar).
- JSON-LD authoring surface — export-only at 1.0.

**Governance semantics:**
- Registry composition — conflict rejection at document-load time. No declaration-order precedence.
- Defeasibility — in `workflow-governance` with explicit `priority: integer`; composes with `sourceAuthority` as lexicographic `(sourceAuthority, priority)`.
- Equity expression language — FEL with a restricted-domain profile; no windowing escape hatch.
- Assurance × impact-level — minimum floor per impact level (rights-impacting ≥ `high`; safety-impacting ≥ `high`; operational ≥ `standard`).

**Signature Profile (WOS-scope only — workflow semantics):**
- DocuSign parity at 1.0 (per owner: "We need docusign parity").
- Parity bar (flagged α): ESIGN/UETA/eIDAS compliance + DocuSign's top ~80% common-case workflow features. NOT DocuSign's administrative UX surface — that's product scope.
- Signer roles as `actorExtension` seam entries: signer, in-person-signer, certified-recipient, witness, notary, approver, form-filler, viewer, approver.
- Signing flow patterns: sequential, parallel, routed (via FEL guards), free-for-all — expressed as kernel lifecycle with hold-tagged `awaiting-signature` states.
- Intent capture: ESIGN/UETA consent step, identity binding, signer certificate reference.
- Evidence emission: new `SignatureAffirmation` provenance record kind carrying `{ signerId, documentHash, timestamp, identityBinding, consentReference }`. **Trellis anchors this record; WOS only emits it.**
- Workflow features: reminders (timer events), expiry (timer + `$timeout.signature`), void/decline/reassign (existing lifecycle + governance primitives).
- Signer-authentication policies: policy schema at the Signature Profile level (admin-portal-driven per session-9 discussion).
- **Certificate-of-completion artifact:** Trellis export-bundle responsibility, not WOS.

**Engine adapters:**
- #49b Temporal OR Restate — IN for 1.0 as the production SaaS runtime. Pick via spike.
- #49a Camunda — trigger-gated (commercial request).
- #49c AWS Step Functions — trigger-gated (commercial request).

**Admin portal:**
- Product scope, not WOS spec scope. Admin features compose from existing primitives. Two exceptions pulled new spec work:
  - **Bulk Operations** future-spec moves to 1.0 scope (admin-portal-driven).
  - **Signer authentication policies** within Signature Profile (see above).

---

## v1.0 spec-freeze line

Under minutes-not-days + "no defer on greenfield" (per [`user_profile.md`](../../../.claude/user_profile.md)), scope is set by architectural prerequisites, not calendar. The "defer" bucket is minimal; items either land at 1.0 or are explicitly trigger-gated.

**Must close for 1.0:**

*Kernel closure:*
- #20 Typed event meta-vocabulary
- #F3b Runtime §10.3 conformance (Task 3 already landed out-of-band `a683c03`; Tasks 1, 2, 4, 5 remain)
- #22a ProvenanceKind tier-typing (enables adapter parallelism + clean Trellis custodyHook delivery)
- Cross-reference shape ADR + schema harmonization

*Structural:*
- §4.5 three merges (assertion-library → workflow-governance; verification-report → advanced-governance; due-process-config residue → workflow-governance)
- §4.4 release trains Tasks 4-5 (Changesets + GitHub Actions workflow)

*Durable execution (the center-vs-adapter split):*
- `DurableRuntime` trait extraction from `wos-runtime`
- Production adapter: Temporal OR Restate (spike both, pick one)
- In-memory adapter (dev/test + conformance oracle)
- Tenant-scope contract finalized (#3)

*Trellis integration:*
- `custodyHook` shape finalized for Trellis ingestion (coordinate with Trellis vision model)
- Provenance record emission completeness — every WOS MUST that produces an audit event actually emits the record
- Signature Profile workflow semantics (signer roles, flow patterns, intent capture, evidence emission shape)

*Behavioral backlog (all at 1.0 under minutes-not-days):*
- #35 Equity enforcement semantics (after #36 Equity language resolution)
- #26a + #26b Access control (canRead enforcement + caseFieldPolicy schema)
- #24b + #25 Defeasibility + rule-firing trace (after ADR)
- #38 G-064 Assertion Library resolution lint (implementation)
- #40 Task SLA runtime implementation
- Bulk Operations spec (admin-portal-driven)

*Verifiability:*
- Every normative MUST across Kernel + Governance + AI Integration has a passing Tested fixture
- Kernel-Basic conformance profile declared LoadBearing
- Conformance runs against all durable-execution adapters (in-memory + production + optional postgres-simple)

*Separate-spec deliverables:*
- EU AI Act alignment document
- OMB M-24-10 compliance document

**Trigger-gated (explicit commercial/external signal required):**

- Additional engine adapters (Camunda, Step Functions) — trigger: commercial adopter request
- SCXML interoperability — trigger: ecosystem demand
- #51 Statutory deadline chains — trigger: first production deployment exposes a concrete need

**NOT 1.0-scoped because they're Trellis-scope, not WOS-scope:**

- #48 Merkle provenance chains (Trellis)
- SCITT strictness / full SCITT alignment (Trellis)
- Federation Profile (cooperative trust-anchor network) (Trellis)
- Checkpoint seal protocol, transparency-log submission, proof-of-inclusion tooling (Trellis)
- Certificate-of-completion export bundle format (Trellis)

---

## Active uncertainties (WOS-scope only)

Trellis-facing uncertainties (β SCITT strictness, multi-log aggregation, witness protocol) moved out of WOS scope.

- **α — DocuSign parity bar.** Default: ESIGN/UETA/eIDAS compliance + top ~80% of DocuSign common-case workflow features; NOT administrative UX surface. Confirm when Signature Profile drafting begins.
- **γ — Durable-execution backend.** Default: spike Temporal + Restate against the extracted trait; pick based on Rust SDK ergonomics + ops fit. Leaning Restate; not a commitment.
- **δ — `wos-runtime` long-term role.** Resolved to model (iii) — semantics library + in-memory adapter + spec-conformance oracle; the in-process durable-execution implementation extracts below the trait line.
- **SBA PoC timeline.** Not specified. Under minutes-not-days, probably doesn't gate 1.0 scope; confirm when concrete.
- **Multi-tenant model on Temporal/Restate.** Likely solved by namespaces (Temporal) or partitions (Restate) + per-tenant provenance log scoping. Confirm during adapter spike.
- **Rendering service for signature artifacts.** Formspec Definition + signature overlays → signed PDF. Who renders? Likely a separate service (Chromium-based). Product-implementation concern, not WOS-spec concern.
- **`custodyHook` contract with Trellis.** The exact provenance-record shape Trellis expects. Requires joint design with the Trellis spec; probably an ADR spanning both submodules. This is the load-bearing new uncertainty surfaced by the Trellis boundary.

---

## WOS-specific decision heuristics

Complement the generic heuristics in [`user_profile.md`](../../../.claude/user_profile.md). Apply in order:

1. **Consult this file first; `user_profile.md` second.** If the question is answered here (directly or transitively from Q1-Q4), apply that answer. Generic principles (opinionatedness, minutes-not-days, interop-over-proprietary) come from user_profile.
2. **Is the question about cryptographic integrity, content-addressed storage, signed envelopes, checkpoint seals, export bundles, or federation/transparency logs?** If yes — it's a Trellis concern, not a WOS concern. Defer to Trellis vision; don't invent WOS-side primitives.
3. **Check Q1-Q4 compatibility.** If the proposed action would override Q1 first-adopter / Q2 spec-runtime authority / Q3 opinionated character / Q4 reference-as-oracle — STOP and ask the owner.
4. **Scope to the SBA + SaaS product stack (Q1).** Before adding work to 1.0, ask: does this directly serve the SBA PoC or the public SaaS within its first year? If no, defer (trigger-gate or out-of-scope).
5. **Maintain the three-way verification posture (Q4).** Any new runtime capability must be implementable in the in-memory adapter + the production adapter; conformance fixtures pass against both.
6. **Minimize module-bottleneck serialization.** Before adding a feature to a bottleneck file (e.g., `provenance.rs` pre-tier-split), sequence the structural refactor first.
7. **Respect named-seams invariant.** New extension points live at one of the six kernel seams or use `x-` patternProperties. Inventing new seams is a Q3 violation.
8. **Spec + runtime land together (Q2).** A PR touching runtime behavior updates spec prose + tests in the same commit-series. Session 8-9 pattern.
9. **Conformance fixtures are the currency of verifiability (Q4).** New MUST adds a Tested fixture in the same PR. Draft MUSTs should be rare and explicitly justified.
10. **Center-vs-adapter framing (Q3).** When architecting a new module, name the center (the shape declared) and the adapter (the implementation) separately. The trait/interface is the center; concrete implementations are adapters.

---

## What this doc is NOT

- A spec. Specs live in `specs/`.
- A plan. Plans live in `thoughts/plans/`.
- A marketing artifact. [POSITIONING.md](../../POSITIONING.md) covers public-facing framing.
- A user profile. [`user_profile.md`](../../../.claude/user_profile.md) captures generic preferences.
- The Trellis vision. Trellis has its own; WOS only declares the `custodyHook` contract it hands off through.
- Immutable. Updated on explicit owner signals; cautiously.

---

## Changelog

- **2026-04-20** — Initial capture. Vision model probed via Q1-Q4 (adopter / spec-runtime / opinionatedness / verifiability); model constructed; 11 outstanding design questions + 4 follow-ups (α β γ δ) answered. Temporal/Restate layering + Federation Profile + Signature Profile + v1.0 scope expansion under minutes-not-days economics captured.
- **2026-04-20 (later)** — Generic user preferences split out into sibling `user_profile.md` at owner's request.
- **2026-04-20 (latest)** — Trellis boundary clarified after parent `user_profile.md` update revealed Trellis as the integrity layer (content-addressed signed events, checkpoint seals, offline-verifiable export bundles, SCITT federation). Merkle/SCITT/Federation moved OUT of WOS scope into Trellis; Signature Profile in WOS shrinks to workflow semantics only (signer roles, flow patterns, intent capture, evidence emission). New heuristic (#2) routes cryptographic-integrity questions to Trellis. New uncertainty — `custodyHook` contract with Trellis — surfaced as the load-bearing joint-design item between the two specs. Three-spec stack context (Formspec + WOS + Trellis) + 7-stage authoring workflow position + center-vs-adapter vocabulary propagated from parent profile.
