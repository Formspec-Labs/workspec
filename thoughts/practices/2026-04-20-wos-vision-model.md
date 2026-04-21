# WOS Vision Model

**Captured:** 2026-04-20
**Status:** Operating guidance. Canonical signals from the project owner, captured verbatim where load-bearing; interpretations labeled as such.
**Maintenance rule:** Update only when the owner gives explicit signals that conflict with current content. Do NOT update speculatively. Treat each answer as frozen until the owner overrides.

---

## Read this first

This document captures the vision model the owner validated on 2026-04-20, after explicitly requesting that I not assume anything written in the existing codebase / specs / commit history is "right" (because it was all written by AI) and to construct the model from probing questions instead.

**When to consult:**
- Before making any architectural decision that crosses more than one subsystem.
- When a new design question arises that isn't directly answered by existing specs.
- When re-reading TODO.md or plan docs after a context break.
- Before dispatching parallel agents on load-bearing work.

**When NOT to use:**
- For local tactical decisions inside a single file/function — just follow the code.
- As a substitute for asking the owner when a genuinely new question arises. This doc answers the questions it was built to answer; it doesn't answer questions the owner never weighed in on.

**Meta-rule the owner asserted:** "Don't assume anything written is 'right' — everything was written by AI." Corollary: when this document disagrees with a spec/plan/ADR written earlier, this document wins until the owner says otherwise. The inverse does not apply — a newly-written AI-generated plan does NOT override this document without explicit owner sign-off.

---

## The foundational answers (canonical signals)

Four questions were asked to fix the vision model. Owner's answers, in their own words, with my interpretation flagged.

### Q1 — First adopter (pins audience, AI centrality, regulatory weight, engine-adapter urgency, v1.0 scope)

**Owner's answer (verbatim):**
> "the SBA where they just want to start out with docusign + adobe forms replacement proof of concept + we want to do a public SaaS to replace jotforms/googleforms whatever and include the AI agent stuff and also the validation ledger for docusign stuff so people could, in theory, do free docusign using our ledger, technically, and/or have a shared ledger system thing?"

**Interpretation (mine, labeled):**

- **Dual adopter:** a specific government customer (SBA — Small Business Administration) simultaneously with a public SaaS aimed at consumer/SMB market share.
- **SBA PoC scope:** DocuSign + Adobe Forms replacement for government workflows. Rights-impacting context; due-process machinery is load-bearing.
- **SaaS scope:** competes in the Jotform/Google Forms/Typeform category. Differentiators: (a) AI-agent governance built-in, (b) cryptographically-verifiable signature ledger ("free DocuSign via our ledger"), (c) shared trust-anchor network across adopters.
- **AI integration is product-central, not seam-optional.** For the consumer SaaS, AI agents ARE the feature.
- **Federation / shared ledger is a 1.0 concern.** Not "Future specs trigger-gated"; the "shared ledger system thing" is part of the product thesis.
- **Signature workflow is a load-bearing pattern.** DocuSign parity, not "composes from existing primitives."

### Q2 — Spec vs. runtime authority (pins drift handling)

**Owner's answer (verbatim):**
> "it's supposed to be spec-led, but this is a non-deterministic process, and often, real-world runtime changes drive progress and need to be integrated, so C" (co-authoritative).

**Interpretation:**

- **Default is spec-led.** When the runtime doesn't implement what the spec says (like F3b continuous-mode §10.3), the runtime is fixed to match.
- **But drift is bidirectional and managed.** When the runtime discovers semantics the spec didn't capture (like `parse_iso_duration_to_ms` rejecting unknown unit letters — a runtime-drives-spec case), the spec is updated to reflect the better semantic.
- **Both sides are reviewed together.** The session-8/9 pattern — spec prose + runtime code + tests landing in one PR with semi-formal review — is the right pattern to keep.
- **"Closed" is closed-as-of-a-date, not permanently.** v1.0 means "closed at release; drift managed in 1.x." Drift is not forbidden; it's acknowledged and negotiated.

### Q3 — Opinionatedness (pins most taxonomy + design decisions)

**Owner's answer (verbatim):** "A" — opinionated / principled.

**Interpretation:**

- **Few right ways to do things.** Extension points are clearly bounded: the six named kernel seams + `x-` patternProperties only.
- **Conformance is strict.** Alternative-expression patterns that already exist elsewhere (BPMN topology, FEEL, SHACL, DAG models) are rejected, not accommodated.
- **No escape hatches at core taxonomy keys.** The 5-kind event taxonomy is closed; vendors extend via payload `x-*` fields, not by adding new kinds.
- **Single-source-authority invariants apply.** Registry conflicts become errors, not merged-via-declaration-order.
- **FEL is the single expression language.** No FEL conformance profiles; no alternative expression languages.

### Q4 — Verifiability threshold (pins what "reference architecture" requires)

**Owner's answer (verbatim):**
> "the entire point of the reference architecture is for us to use it and also for it itself to be a way to test/validate the spec, not sure where that lands"

**Interpretation:**

- **The reference implementation IS the oracle.** Spec MUSTs are verified by: (i) constructing a test fixture that exercises the claim; (ii) running it against the reference implementation; (iii) asserting the spec's predicted outcome. If the runtime agrees, the MUST is validated.
- **Every MUST gets a passing fixture at 1.0.** Under the owner's "dev is free, debt is expensive" economics (see below), closing the current 12/91 Tested/Draft gap is trivial calendar-wise and high-value for the reference-architecture claim. Strong verifiability, not moderate.
- **Conformance tests run against every durable-execution adapter.** In-memory reference + production adapter (Temporal or Restate) must both pass. Three-way agreement (spec + reference + production adapter) is the strongest attainable verification posture.

---

## The economic model

Owner asserted (2026-04-20, Q(i)): **"What is time? The rules of development have changed. Time/processing/development is free. Architectural Tech Debt, Burden, large scale refactoring, are extremely expensive. Assume development happens in minutes instead of days."**

Operational consequences:

- **Priority ordering is `Imp × Debt`**, not a three-dimensional tradeoff. Cx (complexity / engineer-days) becomes a scheduling input, not a cost weight. A `[Cx 2]` item and a `[Cx 8]` item with equal Imp × Debt are equal-priority; they just schedule differently.
- **Gates are decisions, never engineering capacity.** When an item is "blocked," it's blocked on a human decision or an architectural prerequisite — never on "we don't have the engineer-days."
- **Debt values trend UP between sessions on a pre-1.0 spec** as downstream code calcifies around loose shapes. Scoring discipline: re-audit Debt values when item rank shifts; if a schema/type/field has been referenced by new consumers since last audit, Debt should rise.
- **Scope is "all architectural decisions" not "what fits in calendar time."** v1.0 includes everything except items gated on decisions that aren't yet made.
- **Prefer aggressive scope closure to incremental slippage.** Under minutes-not-days, "defer to 1.1" is only correct if an architectural prerequisite isn't resolved. Otherwise, include it in 1.0.
- **Parallelism is the constraining axis, not capacity.** The ceiling on parallel agents is determined by file-scope overlap and shared-crate contention (e.g., `provenance.rs` as module bottleneck), not by engineering headcount. Module structure directly determines throughput.

---

## Product stack (what WOS is for)

| Layer | Purpose | Status |
|---|---|---|
| **Spec (kernel, governance, AI integration, profiles, sidecars)** | Normative definition; verifiable via conformance fixtures | Active; v1.0 closure in flight |
| **Reference implementation (`wos-runtime` Rust)** | Semantics library + test oracle + in-memory adapter | Active; will be split per layering below |
| **Production SaaS** | WOS semantics on top of durable-execution backend, offering DocuSign replacement + AI-governed forms | Planning |
| **SBA PoC** | Specific government customer deployment; validation ground for the SaaS architecture | Planning |
| **Shared ledger (SCITT-aligned)** | Cross-tenant cryptographic trust substrate — "free DocuSign" claim rests on this | 1.0 scope |

Value proposition for SaaS: **Jotform-tier forms UX + governed-AI-agents + cryptographically-verifiable signature ledger** — a combination no current product offers.

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
│  • Signature Profile semantics                          │
│  • Merkle / SCITT anchor emission logic                 │
├─────────────────────────────────────────────────────────┤
│  DurableRuntime trait  (abstraction to extract)         │
│  • start_workflow(instance_id) → handle                 │
│  • signal(handle, event) → Result                       │
│  • schedule_timer(deadline, event) → TimerHandle        │
│  • persist_case_state(state) → Result                   │
│  • await_resume() → Event                               │
├─────────────────────────────────────────────────────────┤
│  Adapters                                               │
│  ├─ in-memory        (dev/test, hermetic)               │
│  ├─ postgres-simple  (single-tenant, minimal ops)       │
│  ├─ temporal         (production option A, self-hosted) │
│  └─ restate          (production option B, self-hosted) │
└─────────────────────────────────────────────────────────┘
```

Everything above the trait line is WOS domain logic that no durable-execution substrate can replace. Everything below the trait line is pluggable.

**Backend choice:** owner prefers self-hosted; Rust-primary. Open decision γ — spike Temporal + Restate against the extracted trait, pick based on direct observation of Rust SDK ergonomics + ops fit. Do not lock blind.

**`wos-runtime` long-term role (γ, semi-resolved):** model (iii) — WOS semantics library + in-memory reference adapter + spec-conformance test oracle. The durable-execution implementation currently tangled into `wos-runtime/src/runtime.rs` (~4451 lines, mostly timer/retry/persistence plumbing) gets extracted below the trait line.

---

## Settled architectural commitments

Decisions the owner has signed off on (either via explicit answer or via transitive implication from the Q1-Q4 answers + economic model):

**Event taxonomy (#20):**
- OQ1 — `$join` is engine-synthesized only. Authors catching a join write `{kind: "signal", scope: "instance", name: "$join"}`; they don't mint `$join` events themselves.
- OQ4 — 5-kind closed enum (`timer | message | signal | condition | error`); vendor extension via payload `x-*` fields, not a sixth `kind: "vendor"` variant.
- OQ2 — Flat event names at the kernel layer; hierarchical subscription is a registry-tier concern.
- OQ3 — Full §7 evaluation context minus `event` for condition-kind events (consistency with guards).

**Cross-reference shapes (session-9 drift):**
- `<entity>Ref: URI` — cross-document references (sidecar URIs; `calendarRef`, `assertionRef`).
- `<entity>Key: string` — in-document local keys (map keys in the same sidecar).
- `<entity>Id: string` — in-document sibling-object references (id-bearing peers).
- ADR pins the three patterns; doesn't try to unify them — they denote different things.

**Opinionated rejections (per Q3):**
- DAG processing model — rejected (append-only event-stream folding only).
- FEEL / DMN expression language — rejected (FEL is purpose-built).
- SHACL — rejected (Rust lint covers cross-doc validation).
- BPMN parity as authoring goal — rejected (export target only).
- FEL conformance profiles — rejected (single grammar).
- JSON-LD authoring surface — export-only at 1.0.

**Governance semantics:**
- Registry composition — conflict rejection at document-load time. No declaration-order precedence.
- Defeasibility — in `workflow-governance` (not a distinct companion) with explicit `priority: integer`; composes with `sourceAuthority` as lexicographic `(sourceAuthority, priority)`.
- Equity expression language — FEL with a restricted-domain profile; no windowing escape hatch.
- Assurance × impact-level — minimum floor per impact level (rights-impacting ≥ `high`; safety-impacting ≥ `high`; operational ≥ `standard`).

**Federation / ledger:**
- Shape A cooperative trust-anchor network (not SaaS-operated single log). SCITT-aligned.
- SCITT strictness default: full SCITT at 1.0 — pursue the IETF standard unambiguously for ecosystem legibility. Fallback if IETF WG volatility blocks spec write: RFC 9162 log structure + SCITT-compat leaf format, witness protocol + registration policies deferred to 1.1.

**Signature Profile:**
- DocuSign parity at 1.0 (per owner Q(iii) = "We need docusign parity").
- Parity bar (my reading, flagged α): ESIGN/UETA/eIDAS compliance + DocuSign's top ~80% common-case features (multi-party, routed, evidence, cert-of-completion, signer-auth policies). NOT DocuSign's administrative UX surface (dashboards, bulk-management, enterprise SSO, template marketplace) — those are PRODUCT scope, not spec scope.

**Engine adapters:**
- #49b Temporal (or Restate) — IN for 1.0 as the production SaaS runtime.
- #49a Camunda — OUT; deferred pending commercial trigger.
- #49c AWS Step Functions — OUT; deferred pending commercial trigger.

**Admin portal:**
- Product scope, not spec scope. Admin features compose from existing primitives. Two exceptions pull new spec work:
  - **Bulk Operations** future-spec moves to 1.0 scope (admin-portal-driven).
  - **Signer authentication policies** land within Signature Profile (not a separate admin-portal spec).

---

## v1.0 spec-freeze line

Under the owner's economic model (dev is free, only architectural prerequisites gate), 1.0 is expansive.

**Must close for 1.0:**

*Kernel closure:*
- #20 Typed event meta-vocabulary
- #F3b Runtime §10.3 conformance (all tasks — Task 3 already landed out-of-band)
- #22a ProvenanceKind tier-typing (enables parallelism + clean Temporal/Restate adapter)
- Cross-reference shape ADR + schema harmonization

*Structural:*
- §4.5 three merges (assertion-library → workflow-governance; verification-report → advanced-governance; due-process-config residue → workflow-governance)
- §4.4 release trains Tasks 4-5 (Changesets + GitHub Actions workflow)

*Durable execution:*
- `DurableRuntime` trait extraction from `wos-runtime`
- Production adapter: Temporal OR Restate (spike both, pick one)
- In-memory adapter (dev/test)
- Tenant-scope contract finalized (#3)

*Ledger:*
- #48 Merkle provenance with SCITT alignment (full SCITT ideally; RFC 9162 + SCITT-compat leaves as fallback)

*Profiles:*
- **Federation Profile** — full Shape A substance (cooperative SCITT-aligned transparency log, multi-log aggregation, witness protocol, registration policies, proof-of-inclusion verification)
- **Signature Profile** — DocuSign parity (signer roles; sequential/parallel/routed flows; intent capture; evidence trail; cert-of-completion; ESIGN/UETA/eIDAS compliance sections; signer-auth policies)

*Previously-deferred behavioral items (now in 1.0 under minutes-not-days):*
- #35 Equity enforcement semantics (after #36 Equity language resolution — which can happen in minutes)
- #26a + #26b Access control (canRead enforcement + caseFieldPolicy schema)
- #24b + #25 Defeasibility + rule-firing trace (after ADR, which is minutes)
- #38 G-064 Assertion Library resolution lint (implementation, not just design sketch)
- #40 Task SLA runtime implementation (beyond the session-8 authoring surface)
- Bulk Operations spec (admin-portal-driven)

*Verifiability:*
- Every normative MUST across Kernel + Governance + AI Integration specs has a passing Tested fixture (strong verifiability, per Q4 + minutes-not-days)
- Kernel-Basic conformance profile declared LoadBearing
- Conformance runs against all durable-execution adapters (in-memory + production + optional postgres-simple)

*Separate-spec deliverables:*
- EU AI Act alignment document
- OMB M-24-10 compliance document

**Defer to 1.x:**

- Additional adapters (Camunda, Step Functions) — pending commercial triggers
- Multi-log aggregation beyond the 1.0 Federation Profile basics
- SCXML interoperability (informative, low value)
- Engine adapters other than the production-backend choice

**Out of scope indefinitely unless trigger fires:**

- #51 Statutory deadline chains (too speculative)
- Engine adapters for runtimes no adopter has asked for
- Any feature that contradicts the opinionated character

---

## Active uncertainties

Unresolved decisions, flagged for future sessions to surface when work reaches them:

- **α — DocuSign parity bar.** My default: ESIGN/UETA/eIDAS compliance + top ~80% of DocuSign common-case workflow features; NOT administrative UX surface. Owner not explicitly confirmed. Confirm when Signature Profile drafting begins.
- **β — SCITT strictness.** Default: full SCITT at 1.0; fallback to RFC 9162 + SCITT-compat-leaves if IETF WG volatility blocks spec write-up. Revisit when Federation Profile drafting begins and current IETF draft status is checked.
- **γ — Durable-execution backend.** Default: spike Temporal + Restate against the trait, pick based on Rust SDK ergonomics + ops fit. Neither is pre-chosen. Owner prefers self-hosted; Rust-primary; operational simplicity matters. Leaning Restate on first-principles grounds; not a commitment.
- **δ — `wos-runtime` long-term role.** Resolved to (iii) — semantics library + in-memory adapter + spec-conformance oracle; the in-process durable-execution implementation gets extracted below the trait line. Retire role as "canonical single-tenant production runtime."
- **SBA PoC timeline.** Not specified. Affects whether Signature Profile + Federation Profile can both land as full 1.0 deliverables vs. one being MVP-first. Under minutes-not-days, probably both fit regardless; confirm when timeline becomes concrete.
- **Multi-tenant model on Temporal/Restate.** Likely solved by namespaces (Temporal) or partitions (Restate) + per-tenant provenance log scoping + cross-tenant SCITT log submission. Confirm during adapter spike.
- **Rendering service for signature artifacts.** Who generates the signed-PDF? Formspec + signature overlays + rendering engine. Dependency: probably a separate service (Chromium-based PDF generation or similar). Not a WOS-spec concern but a product-implementation concern. Note for when Signature Profile drafting begins.

---

## Decision heuristics

When a new design question arises in a future session, apply in order:

1. **Check this document.** If the question is answered here (directly or by transitive implication from Q1-Q4 + the economic model), apply that answer.
2. **Check whether it conflicts with the foundational answers.** If the proposed action would require overriding Q1/Q2/Q3/Q4 or the economic model — STOP and ask the owner.
3. **Apply the Imp × Debt lens.** Ask: what's the cost of deciding wrong? What's the cost of deferring the decision? Under minutes-not-days, the latter is often higher.
4. **Default to opinionated (Q3).** When two options are both defensible, pick the more restrictive / more principled one. Justify the less-restrictive option actively if you take it.
5. **Default to spec-led with runtime-informed updates (Q2).** The default direction of repair is spec-is-right-runtime-catches-up. But if the runtime discovered a better semantic the spec didn't capture, propagate back to the spec.
6. **Scope to the SBA + SaaS product stack (Q1).** Before adding work to 1.0, ask: does this directly serve the SBA PoC or the public SaaS within its first year? If no, defer.
7. **Minimize module-bottleneck serialization.** Before adding a feature to a file that's already a parallelism bottleneck (e.g., `provenance.rs` pre-split), sequence the structural refactor first.
8. **Prefer abstraction-discipline over premature implementation choice.** When a question is "which backend?" (Temporal vs. Restate vs. other), design the abstraction cleanly first; the concrete choice can defer to a spike.

---

## What this doc is NOT

- A spec. Specs live in `specs/`; this is operating guidance.
- A plan. Plans live in `thoughts/plans/`.
- A marketing artifact. POSITIONING.md covers public-facing framing; this is internal frame.
- A record of everything the owner has said. It's a distilled vision model built from four foundational probing questions + follow-ups. Owner may have said many other things in conversation; those aren't canonical unless reflected here.
- Immutable. When the owner gives explicit new signals, this gets updated. But cautiously — speculative updates drift.

---

## Changelog

- **2026-04-20** — Initial capture. Vision model probed via four foundational questions (Q1 adopter, Q2 spec/runtime, Q3 opinionatedness, Q4 verifiability); owner answered; model constructed and validated across 11 outstanding design questions plus 4 follow-ups (α DocuSign parity, β SCITT strictness, γ backend choice, δ wos-runtime role). Temporal/Restate layering + Federation Profile + Signature Profile + v1.0 scope expansion under "minutes-not-days" economics captured. Open uncertainties flagged.
