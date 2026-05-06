# WOS TODO

Active backlog. Completed work → [COMPLETED.md](COMPLETED.md). Stack architecture → [`VISION.md`](../VISION.md).

**Last audited:** 2026-05-06 — scout-swarm validated all uncompleted items against HEAD; stale paths/counts/gates corrected. ADR 0082 and ADR 0076 closure history → COMPLETED.md. ADR 0092 (TypeID-in-URN identity) proposed 2026-05-06.

## Snapshot

| Health | Value |
|---|---|
| Specs / schemas | 41+18 spec/docs under `specs/` (`specs/api/` adds 17 ADR 0082 docs) · 22 schemas (4 core + 2 sidecars + 16 under `schemas/api/`) · 0 SCHEMA-DOC-001 violations |
| Crates | 6 production (`wos-core`, `wos-lint`, `wos-conformance`, `wos-runtime`, `wos-formspec-binding`, `wos-export`) · 6 MVP (`wos-authoring`, `wos-mcp`, `wos-synth-core/-mock/-anthropic/-cli`) · 1 spike (`wos-synth-spike`, keep-with-deletion-horizon) |
| Tests | Latest targeted gates: `cargo check --workspace` green; `cargo nextest run -p wos-core --lib` green; `cargo nextest run -p wos-runtime --lib` green; `cargo nextest run -p wos-lint` green; `cargo nextest run -p wos-conformance --test signature_profile` 13 green; `pytest tests/schemas -q` 376 passed / 1 xfailed (post-ADR-0082; was 255 pre-session). API discipline test 15/15 (incl. cross-schema `$ref` resolution + facts-record-kind kernel parity + open-string-via-`oneOf`-arm recursion). |
| Lint matrix | 116 rules (35 T1 · 72 T2 · 9 T3 · 1 LoadBearing · 11 Tested · 104 Draft) |
| CI gates | `schema_doc_zero_regression` · `every_promoted_*_rule_has_executable_or_annotated_evidence` · `every_load_bearing_conformance_rule_has_at_least_two_executable_fixtures` · `discover_and_report_promotion_candidates` ratchet · **ADR 0082 D-13 Gates 1–6** under `.github/workflows/api-contract-guardrails.yml`: schema validity (ajv), OpenAPI `$ref` discipline, route coverage, oasdiff breaking-change, response conformance (server + portal), mirror byte-parity |

**Navigation:** [**User profile** (read first)](../.claude/user_profile.md) · [**Stack vision**](../VISION.md) (canonical; WOS §X) · [`work-spec/CLAUDE.md`](CLAUDE.md) · [LINT-MATRIX](LINT-MATRIX.md) · [Feature Matrix](WOS-FEATURE-MATRIX.md) · [Implementation Status](WOS-IMPLEMENTATION-STATUS.md) · [IDEA_SCRATCH](IDEA_SCRATCH.md) · [POSITIONING](POSITIONING.md) · [CONVENTIONS](CONVENTIONS.md) · [Runtime Companion](specs/companions/runtime.md) · [ADRs](../thoughts/adr/) · [Plans](thoughts/plans/) · [Parallel-agent dispatch discipline](thoughts/practices/2026-04-17-parallel-agent-dispatch.md)

---

## Do next

Pick from the top. Each item has a gate (what unblocks it) and a plan or ADR.

**Scoring note.** Per [`user_profile.md`](../.claude/user_profile.md) economic model: dev/time is free, architectural drift is expensive. Ordering uses **`Imp × Debt`**; Cx is preserved as a scheduling dimension but does not change priority. Debt values trend **up** between sessions on pre-1.0 work. Score notation: `[Imp / Cx / Debt]`; the number in parentheses is `Imp × Debt`.

1. **Signature Profile workflow semantics** `[7 / 5 / 5]` (**35**) — **WOS-T4 ACTIVE (cross-repo closeout).** Workflow-tier slice of the **DocuSign 100% parity bar** per VISION §X (parent PLN-0370 reframe holds the marketing line until full parity lands; PLN-0355 ESIGN/UETA gate Trigger): signer roles via `actorExtension`, sequential/parallel/routed/free-for-all flows, intent capture, identity binding, signer-authentication policy schema, reminders, expiry, decline, void, reassignment, and `SignatureAffirmation` provenance. Cryptographic integrity + certificate-of-completion live in Trellis; WOS only emits the semantic evidence record. **Path back to original DocuSign-100% framing** lands via parent **PLN-0380** (`signature.md` §1.3 scope reopen + signing-intent URI registry + signer-authority claim shape distinct from §2.6 authentication-method) + parent **PLN-0398** (Trigger — administrative surface: template libraries, bulk-send, send-for-signature dashboards, signer status views, reminder cadence configuration, audit history view). Trellis user-content Attestation primitive at parent **PLN-0379** (Trellis ADR 0010) composes for byte-level signing-intent URI carriage. **Execution plan:** [T4-TODO.md](T4-TODO.md). **Landed 2026-04-22 (WOS center):** [ADR-0062](thoughts/adr/0062-signature-profile-workflow-semantics.md), [Signature Profile spec](specs/profiles/signature.md), Signature Profile schema (embedded `signature` block in `wos-workflow.schema.json` per ADR 0076), schema fixtures/tests, Studio generated type binding, `ProvenanceKind::SignatureAffirmation`, schema-constrained `SignatureAffirmation` payload, Rust constructor/helper, Facts-tier classification, custody append inclusion, SIG-001..SIG-012 + WOS-SIG-COVER-001 lint (13 total), runtime profile loading, signing task evidence validation, `SignatureAffirmation` emission, sequential/parallel/routed/free-for-all/witness/notary/decline/void/reassignment/expiry semantics, and 13 SIG-* conformance tests. **Cross-repo landed 2026-04-22:** parent Formspec canonical signed-response / `authoredSignatures` fields, signed-response fixture, WOS-facing mapping seed, server-side revalidation preservation, and Trellis `append/019` + export `006` / verify `014` / tamper `014` + Core verifier extension for `062-signature-affirmations.cbor`. **Next T4 slice:** shared fixture bundle end-to-end (design doc at parent [`thoughts/specs/2026-04-24-shared-cross-seam-fixture-bundle-design.md`](../thoughts/specs/2026-04-24-shared-cross-seam-fixture-bundle-design.md)), Studio authoring/validation UX, Trellis human certificate-of-completion composition per [Trellis ADR 0007](../trellis/thoughts/adr/0007-certificate-of-completion-composition.md) (accepted 2026-04-24) ([T4-TODO.md](T4-TODO.md) T4-10–T4-12).

   **WOS-T4 -COMPLETE- criteria:** Formspec captures ✓ · WOS routes ✓ · WOS emits `SignatureAffirmation` ✓ · Trellis machine-verifiable export accepts ✓ · Conformance proves patterns ✓ · COC rendering (T4-10, [`trellis/TODO.md`](../trellis/TODO.md) item #4) ✗ · vendor `x-*` assurance floor (fail-open gap, gated on PLN-0384) ✗ · Studio authoring UI (T4-11, 11 items) ✗. Execution detail moved into this file (T4-TODO.md merged 2026-05-06; replaced with redirect pointer).
2. **AI-runtime capability-precondition emission wiring** `[6 / 5 / 4]` (**24**) — typed Rust path landed 2026-04-28; 5 unit tests + 6 Python schema tests pass. **Still open:** (a) runtime emission site — AI §3.3.1 step 1-3 specifies precondition evaluation but no runtime path actually evaluates `Capability.preconditions` (`crates/wos-core/src/model/ai.rs:197`); the field is declarable but not fired. (b) JSON conformance fixture pair (blocked + permitted) under `fixtures/conformance/`. (c) Ergonomic constructor variant once call-site count justifies. (d) **New blocker (2026-05-06 scout):** ADR 0064 `AgentInvoker` port landed (trait + 6 adapter crates: `wos-agent-stub`/`-anthropic`/`-mcp`/`-a2a`/`-http`/`-claude-sdk`) but the **orchestrator** that calls `AgentInvoker::invoke()` — and would be the natural precondition-evaluation site — doesn't exist. `DurableRuntime` has no `invoke_agent` method; a new `AgentRuntime` trait or method addition may be required. **Gate: AI-runtime invocation seam design — port half-landed, orchestrator missing.** Discovered 2026-04-28 audit ([`thoughts/audit-2026-04-28-provenance-emission-completeness.md`](thoughts/audit-2026-04-28-provenance-emission-completeness.md) Gap 1). **Debt bumped 3→4** (ADR 0064 orchestrator adds surface area).
3. **Actor authorization shape (`AuthorizationAttestation`)** `[7 / 4 / 5]` (**35**) — stack contract per ADR 0066 D-2. WOS-center provenance already landed: `ProvenanceKind::AuthorizationAttestation` at `kind.rs:350`, schema `$defs/AuthorizationAttestationRecord`, Facts-tier classification, export adapters (PROV-O/XES; OCEL pending). Remaining: governance policy sections + runtime emission wiring (gated). **Gate: ADR 0066 still Proposed (de facto accepted — cluster-ratification gate satisfied; status flip pending).** Tracked in [ADR 0066 execution checklist](#adr-0066-exec-checklist) items 1-2.
4. **ADR 0066 implementation — amendment / supersession / rescission / correction** `[7 / 6 / 5]` (**35**) — seven provenance record kinds (6 listed + `Reinstated` from maximalist cluster), `caseRelationship.type = supersedes`, Workflow Governance policy sections, exporter coverage. WOS-center provenance + export (2/3 paths) landed; governance policies + runtime wiring + conformance remain open. **Gate: ADR 0066 still Proposed (de facto accepted).** Full WOS-scoped breakdown: [ADR 0066 — execution checklist](#adr-0066-exec-checklist).
5. **ADR 0067 implementation — statutory clocks** `[7 / 5 / 5]` (**35**) — `clockStarted` / `clockResolved` provenance kinds, `Clock` `$def`, four-kind runtime wiring, `#40` / `#51` composition, export mappings, conformance. WOS-center provenance + schema $defs landed; runtime emission + export + conformance remain open. **Gate: ADR 0067 still Proposed (de facto accepted).** Execution checklist: [ADR 0067 — execution checklist](#adr-0067-exec-checklist).
6. **ADR 0092 — TypeID-in-URN identity landing** `[7 / 5 / 5]` (**35**) — Narrow `WosResourceUrn` from 5-segment `urn:wos:<entity-type>:<scope>:<date>:<hash>` to 3-segment `urn:wos:<typeid>`. Strip `urn:wos:` → canonical TypeID. One identity from DB through API through durable execution through Trellis. Greenfield: no backwards compat, no dual-format acceptance. **ADR:** [`thoughts/adr/0092-api-typeid-urn-identity.md`](thoughts/adr/0092-api-typeid-urn-identity.md) (Proposed — amends ADR 0082 D-4/D-14). **Spec:** [`thoughts/specs/2026-05-06-api-typeid-identity.md`](thoughts/specs/2026-05-06-api-typeid-identity.md). **Plan:** [`thoughts/plans/2026-05-06-adr0092-typeid-urn-identity-landing.md`](thoughts/plans/2026-05-06-adr0092-typeid-urn-identity-landing.md) — 8 work streams, ~42 files. **Gate: ADR 0092 acceptance.**
7. **Multi-step session DAG topology (P2, §21 from API coverage audit)** `[5 / 2 / 4]` (**20**) — schema-only close of the single deferred gap from the 2026-05-06 WOS Runtime API coverage audit ([`wos-api-coverage-findings.md`](wos-api-coverage-findings.md) item #21). Two-phase schema restoration. **Revised 2026-05-06 (scout findings):**
   - **Phase 1 — Author-time schema (wos-workflow.schema.json):** `MultiStepSession` and `SessionStep` `$defs` were lost during ADR 0076 consolidation — recover from git `e7c46c0f^:schemas/advanced/wos-advanced.schema.json`. Add `multiStepSessions` property to `Advanced` block. **CRITICAL:** `Advanced` $def has no `additionalProperties: false` — the fixture currently validates because extra properties pass through silently. Adding `additionalProperties: false` is required for meaningful validation, which means all other missing old-schema properties (`toolGovernance`, `agentLifecycle`, `calibration`, `driftDetection`) must be reconciled or explicitly deferred with a note. Decision needed: expand scope or use permissive `"additionalProperties": { "not": {} }` pattern.
   - **Phase 2 — Runtime API topology surface:** `MultiStepSessionState` carries live state but no DAG topology. Add `SessionStepStatus` $def (closed enum: `pending | in-progress | completed | failed | blocked`), `SessionStepState` $def (per-step projection), extend `MultiStepSessionState` with optional `steps: SessionStepState[]`. **Endpoint placement:** add to `specs/api/instance.md` (instance-scoped sub-resources belong there, not `governance.md` which is cross-case). Use `GET /api/v1/instances/{id}/sessions/{sessionId}` — `sessionId` is a plain identifier (not a `WosResourceUrn`), so no `_common.schema.json` URN entity-type change needed.
   - **Validation:** `python3 -m pytest tests/schemas -q`, `cargo check --workspace`. No Rust types added. Rust types + runtime wiring follow-up passes.
   - **Gate: none (plan needs owner decision on scope).**

### Agent task extract (from this file)

| Task ID | Tracks | Deliverable | Depends on |
|---------|--------|-------------|------------|
| **WOS-T4** | Do next **#1** | Signature Profile end-to-end — WOS center landed; cross-repo: Trellis COC rendering + Studio UI + vendor assurance floor | T4-10 (COC renderer) · T4-11 (Studio UI) · T4-12 (shared bundle) · vendor x-* assurance floor (gated on PLN-0384) |
| **WOS-T5** | Do next **#6** | Multi-step session DAG topology — author-time schema restoration + runtime API topology surface | None — schema-only; definitions recoverable from git `e7c46c0f^` |
| **WOS-B1** | Backlog | §4.5 structural merges (1 vs 3 PRs) | Owner packaging decision |
| **WOS-B2** | Backlog | Kernel-Basic profile **LoadBearing** declaration + lint-matrix wire | None |
| **WOS-B3** | Backlog | [ADR 0065](thoughts/adr/0065-wos-authoring-stack-mirrors-formspec.md) authoring-stack closure — MCP↔synth `ToolContext` seam, spike/Q-V0 follow-ups, `wos-bench`, conformance/lint API hygiene | See **ADR 0065 — authoring stack closure** in Backlog |
| **WOS-B4** | Backlog | Runtime Companion **§15** / Phase 11 — `wos-runtime` parity vs published MUSTs (`#66`–`#66g`), PARITY drift (**WS-074**), HTTP §15 fixtures (**WS-075**); ADR 0066 server slice **WS-072**; ADR 0067 clock prove-out **WS-073** | [§ Runtime Companion §15 / Phase 11](#runtime-companion--15--phase-11--reference-wos-runtime-parity) · [`crates/wos-server/TODO.md`](crates/wos-server/TODO.md) **WS-011**, **WS-072–WS-073**, **WS-074–WS-075** |

*Falling off Do next at Imp × Debt < 30:* §4.5 structural merges (owner decision needed), **ADR 0065 authoring-stack closure** (Backlog — MCP/synth seam + spike follow-ups + `wos-bench`; consolidated 2026-04-24), §4.4 release-trains Tasks 4-5 (15). All live in Backlog.

### ADR 0066 — execution checklist (WOS center) {#adr-0066-exec-checklist}

**Gate:** [ADR 0066](../thoughts/adr/0066-stack-amendment-and-supersession.md) **still Proposed (de facto accepted — cluster-ratification gate satisfied; status flip pending).** Formspec Respondent Ledger work and Trellis vectors/verifier/export stay owned in parent [`TODO-STACK.md`](../TODO-STACK.md) and [`../trellis/TODO.md`](../trellis/TODO.md) item **7**; this block is the **WOS spec + schema + runtime + export** slice.

1. **Kernel / provenance**
   - [x] Add **seven** `ProvenanceKind` variants (6 + `Reinstated` from maximalist cluster) + schema `recordKind` registrations in `wos-workflow.schema.json` (not `wos-provenance-record.schema.json` — that path no longer exists post-ADR 0076): `correctionAuthorized`, `amendmentAuthorized`, `determinationAmended`, `rescissionAuthorized`, `determinationRescinded`, `reinstated`, `authorizationAttestation`. **Landed** — `crates/wos-core/src/provenance/kind.rs:304-359`, schema $defs at lines 4739-5301.
   - [x] Payload `$defs` / `allOf` guards — all 7 record shapes landed: `CorrectionAuthorizedRecord`, `AmendmentAuthorizedRecord`, `DeterminationAmendedRecord`, `RescissionAuthorizedRecord`, `DeterminationRescindedRecord`, `ReinstatedRecord`, `AuthorizationAttestationRecord`.
   - [x] Tier map: **Facts** for all seven — `crates/wos-core/src/provenance/audit_tier.rs:160-166`.
   - [x] `wos-core` `audit_layer_for_kind` / conformance tests — enumeration at `tests.rs:553-559` + per-variant assertions at lines 945-1373.
2. **Workflow Governance**
   - [ ] Normative policy sections: `amendmentPolicy`, `rescissionPolicy`, `reinstatementPolicy`, `correctionPolicy`, `supersessionPolicy` — each binding an `AppealMechanism`-shaped gate; impact-level assurance floor (rights-impacting → authorizing actor `Assurance ≥ high`) per D-2. **Zero landed.**
   - [ ] Lint rules + fixtures: K-A-010 (closed five-mode amendment taxonomy). **Zero landed.**
3. **`caseRelationship.type = supersedes`**
   - [x] Schema enum includes `supersedes` at `schemas/api/instance.schema.json:945,957`. Kernel spec prose landed at `specs/kernel/spec.md:772`. K-048 lint enforces `x-` prefix for non-standard values.
   - [ ] Companion examples + fixtures emitting superseding case instance + runtime validation of `targetCase` URI shape remain open.
4. **Runtime + binding**
   - [ ] `wos-runtime` (and `wos-formspec-binding` where intake/custody intersects): emit the new records on governed transitions; ensure intake paths never silently mutate prior responses when a correction lineage exists (ADR Context). **Zero landed — gated by #72.**
5. **Export**
   - [x] `wos-export`: PROV-O + XES event types for all seven kinds landed (`prov_o.rs:685-691`, `xes.rs:683-689`).
   - [ ] OCEL event types remain unlanded.
6. **Conformance + docs**
   - [ ] `wos-conformance` fixtures per kind; update `WOS-IMPLEMENTATION-STATUS.md` / matrix rows as applicable. **Zero landed.**

**Note:** #71 (`ReinstatementPolicy` schema $def + K-A-010 lint) belongs as item 7 in this checklist — tracked under Backlog behavioral items.

**Related:** statutory “may this still be amended?” is [ADR 0067](../thoughts/adr/0067-stack-statutory-clocks.md) (separate acceptance). D-5 only requires composition, not 0067 implementation inside 0066.

### ADR 0067 — execution checklist (WOS center) {#adr-0067-exec-checklist}

**Gate:** [ADR 0067](../thoughts/adr/0067-stack-statutory-clocks.md) **still Proposed (de facto accepted — cluster-ratification gate satisfied; status flip pending).** Trellis `open-clocks.json`, verifier advisories, and append vectors **014–018** stay in parent [`TODO-STACK.md`](../TODO-STACK.md) and [`../trellis/TODO.md`](../trellis/TODO.md) item **8**; Formspec **StatuteClock** origination on respondent acts stays in parent [`TODO-STACK.md`](../TODO-STACK.md); reference-server prove-out is [`crates/wos-server/TODO.md`](crates/wos-server/TODO.md) **WS-073**.

1. **Kernel / provenance**
   - [x] Add `ProvenanceKind` variants + schema `recordKind` branches in `wos-workflow.schema.json`: `clockStarted`, `clockResolved`. **Landed** — `crates/wos-core/src/provenance/kind.rs:361-377`, schema enum at lines 3857/3859.
   - [x] Payload `$defs` / `allOf`: **ClockStarted** (clockId, clockKind [AppealClock|ProcessingSLA|GrantExpiry|StatuteClock|x-*], originEventHash, duration, calendarRef, statuteReference, computedDeadline) and **ClockResolved** (clockId, originClockHash, resolution [satisfied|elapsed|paused|cancelled], resolvingEventHash, resolvedAt) — both landed at `schemas/wos-workflow.schema.json:5415-5691`.
   - [x] Facts-tier classification — `crates/wos-core/src/provenance/audit_tier.rs:167-168`.
2. **Runtime emission (ADR D-2 — four kinds)**
   - [ ] **AppealClock** — adverse-decision / deterministic notice path (composes with Gov §4.1 #2).
   - [ ] **ProcessingSLA** — intake accepted / intake-complete workflow event. Note: ADR 0067 D-2.1 deprecates `SlaDefinition` in favor of `ProcessingSLA` with `kind` discriminator — migration not yet done.
   - [ ] **GrantExpiry** — benefit award issued transition.
   - [ ] **StatuteClock** — WOS-owned triggers only on this slice; Formspec-originated statute clocks use the respondent-ledger path (parent repo).
   - [ ] **Pause / resume (D-4):** `clockResolved` with `resolution: paused` plus a new `clockStarted` carrying **residual** duration — no separate `ClockPaused` record kind.
3. **Task SLA (#40)**
   - [x] Authoring surface landed (`schemas/wos-workflow.schema.json:7762-7789` — `slaDefinitions`, `SlaDefinition`, `warningThresholds`, `breachPolicy`, `escalationChain`).
   - [ ] Runtime SLA implementation (TODO #40; `specs/governance/workflow-governance.md` §10.3).
   - [ ] Cross-reference clock contract where Task SLA durations overlap rights-impacting deadlines (gated on ADR 0067 D-2.1 migration).
4. **#51 statutory deadline chains**
   - [x] Business calendar §7.1 infrastructure exists (`specs/sidecars/business-calendar.md:185-196` — 6-step normative algorithm).
   - [ ] Compose with §7.1 business calendars + typed kernel events; revisit trigger-gate once center contract ships (i.e., ADR 0067 accepted + D-1/D-2 shipped).
5. **`wos-export`**
   - [ ] Distinct PROV-O / OCEL / XES event types or annotations for `clockStarted` / `clockResolved`. **Zero landed — all three export paths empty for clock kinds.**
6. **Conformance + normative closure**
   - [ ] `wos-conformance` fixtures (start, satisfied, elapsed, paused segment); kernel / companion prose for MUST-level behavior.
   - [ ] Encode or explicitly defer ADR §Open questions: envelope timestamp granularity; post-hoc synthetic `elapsed` vs leave-open + verifier-only; multi-jurisdictional independent emits vs single jurisdiction.

---

## Backlog

### Envelope-stack enablement (§4.7)

- [ ] **#58 Envelope (instance-level) status extension** `[Imp 7 / Cx 3 / Debt 5]` — Extend `CaseInstance.status` (or adjacent schema surface) with first-class `declined | voided | expired` discriminators, each carrying required metadata (`declineReason`, `voidedBy`, `voidedAt`, `expiredAt`). Current status taxonomy (`active | suspended | migrating | completed | terminated`) can't distinguish "envelope signer declined" from "processor terminated the instance" — a material legal distinction. Companions to #30: #30 is task-level, #58 is instance-level. **Debt 5** because every envelope shipped without this forces integrators to encode the distinction in case_state, creating diverging conventions that later have to be migrated.
- [ ] **#59 CloudEvent envelope-flow type catalog** `[Imp 6 / Cx 3 / Debt 4]` — Normative event-type catalog in `integration.md` for cross-system envelope coordination: `envelopeCreated`, `signerInvited`, `signerAuthenticated`, `signerSigned`, `signerDeclined`, `envelopeCompleted`, `envelopeVoided`, `envelopeExpired`, `reminderDue`. Distinct from #20 (which normalises **kernel-internal** event vocabulary per transition). #59 is the **cross-system wire contract** that identity providers, email adapters, and webhook consumers speak. Without it, every WOS-based signature stack defines its own event names and the integration ecosystem fragments.
- [ ] **#60 Envelope reference fixtures** `[Imp 5 / Cx 3 / Debt 3]` — Three to five canonical kernel documents under `fixtures/kernel/envelope-*.json` demonstrating the composition patterns: `envelope-2signer-sequential.json`, `envelope-parallel-witness.json`, `envelope-decline-reroute.json`, `envelope-with-approver.json`, `envelope-reminder-expire.json`. Plus matching conformance fixtures exercising the full lifecycle (create → invite → sign → complete; create → invite → decline → void). **Fixture-only work** — no new schema surface, but critical for lock-in: locked patterns prevent divergent re-inventions across vendors building on WOS. Depends on #20 typed events and #30 task-lifecycle for the decline fixture.
- [ ] **#61 Separation-of-duties conformance fixture batch** `[Imp 5 / Cx 2 / Debt 3]` — Two to three fixtures under `fixtures/conformance/` exercising the AccessControl seam's separation-of-duties rejection path: (1) agent attempts to review its own output → rejected; (2) delegated human attempts to re-review as the original author → rejected; (3) separation-of-duties bypass with authority override → recorded as provenance with `OverrideRecord`. Pairs with #23 OverrideRecord schema landing. Shape of the AccessControl seam is already in wos-core traits; what's missing is the conformance contract that reference processors MUST reject these attempts.

### Structural merges (§4.5) — blocked on owner decision

Three merges ratified by the 2026-04-20 [sidecar audit](thoughts/reviews/2026-04-20-sidecar-contract-audit.md). **Gate: user decision — one PR or three?** `VISION.md` / practice recommendation: three discrete PRs for review hygiene; audit recommended one. Either is acceptable; owner picks.

- **Assertion Library → Workflow Governance** `[4 / 2 / 5]` (**20**) — `AssertionUse` seam already landed session 8; merge is mechanical file-move.
- **Verification Report → Advanced Governance** `[3 / 2 / 2]` (**6**) — it's a processor **output**, miscategorized as a sidecar.
- **Due Process Config partial merge → Workflow Governance** `[5 / 3 / 4]` (**20**) — residual sections duplicate Governance §3.1/§3.5.

Companion decisions from session-9 agent dispatch: M-1 Drift Monitor + Agent Config merge remains BLOCKED (standalone fixture); M-2 Notification Template + Due Process merge remains REJECTED (categories don't align).

### Release + benchmarking — ready, lower priority

- **§4.4 Release trains Tasks 4-5** `[5 / 4 / 3]` (**15**) — Changesets tooling + GitHub Actions release workflow. Plan: [2026-04-16](thoughts/plans/2026-04-16-wos-release-trains.md). Tasks 1-3 landed session 8.
- **§5.5 `wos-bench` synthesis benchmark** `[6 / 5 / 3]` (**18**) — Claim A falsification harness; pairs with [ADR 0065](thoughts/adr/0065-wos-authoring-stack-mirrors-formspec.md) Q6 / synth split. Plan: [2026-04-16](thoughts/plans/2026-04-16-wos-synthesis-benchmark.md). Spike open questions: [2026-04-20](thoughts/research/2026-04-20-wos-synth-v0-spike-findings.md#open-questions) (Q-V0-1..4 need **live** Anthropic runs; update that doc with numbers). **Sub-deliverables:** scaffold `crates/wos-bench` (`wos-synth-core` + `wos-synth-mock`, optional Anthropic flag); problem statements + `benchmarks/runs/<date>-<model>/results.json`; rubric library + CLI; `BENCHMARK.md` leaderboard + methodology; scheduled/manual CI with secrets; pick **inline `ConformanceFixture` wrapper vs** upstream `wos_conformance::smoke_test_document`-style API (spike Option B — reduces duplication with synth-core / spike).

### ADR 0065 — authoring stack closure (MCP / synth / spike)

**Anchors:** [ADR 0065](thoughts/adr/0065-wos-authoring-stack-mirrors-formspec.md) · MCP plan [2026-04-17](thoughts/plans/2026-04-17-wos-mcp-crate.md) · Synth plan [2026-04-16](thoughts/plans/2026-04-16-wos-synth-crate.md) · Spike retrospective [2026-04-20](thoughts/research/2026-04-20-wos-synth-v0-spike-findings.md). Plan markdown checkboxes in those files are **stale vs `main`** in places; this subsection is the working backlog until checkboxes are rebased.

**Production seam (ADR D-3; MCP plan completion §2)** `[7 / 5 / 4]` (**28**)

- [ ] **#65a `ToolContext` via shared MCP handlers** — Implement a **second** `ToolContext` implementation whose lint/conformance behavior matches `wos_mcp` tools (`wos_lint`, `wos_run_conformance` code paths), not a parallel copy in `wos-synth-core` only. Needs a small adapter design: synth loop holds **document JSON**; `wos_mcp::dispatch` expects **`ProjectRegistry` + `project_id`**. Options: implicit scratch project per session, or a thin `wos_mcp` (or adjacent) type that implements `ToolContext` by delegating to existing `tools::*` internals.
- [ ] **#65b `wos-synth-cli` default wiring** — Switch CLI from **`DirectToolContext`** to the MCP-aligned **`ToolContext`** from #65a (ADR: production wiring injects MCP-backed dispatch, not stopgap-only).
- [ ] **#65c Optional purity pass** — Once #65a/#65b are real, consider **removing direct `wos-lint` / `wos-conformance` dependencies from `wos-synth-core`** so the loop crate stays provider- and lint-free at the crate edge (ADR D-2/D-3 intent). Only if no in-crate stopgap remains required for tests.

**MCP transport + docs** `[5 / 4 / 3]` (**15**)

- [ ] **#65e SDK migration follow-up** — `wos-mcp` `Cargo.toml` **TODO**: revisit `rust-mcp-sdk` with `default-features = false` + minimal features vs current hand-rolled stdio (~transport swap).
- [ ] **#65f Real MCP client validation** — Exercise `wos-mcp` binary under a **real** MCP host (e.g. Claude Desktop). Plan addendum + spike both state: **v0 spike never touched MCP**; silence is not proof of dual-entry correctness.

**Spike + synth quality (research 2026-04-20; synth plan addendum)** `[6 / 4 / 4]` (**24**)

- [ ] **#65g Q-V0-1..4 live closure** — Run synth against Anthropic on the PO fixture; record iteration counts, dominant first-pass diagnostics, whether conformance repair fires, schema vs FEL vs governance fix mix. Update [spike findings](thoughts/research/2026-04-20-wos-synth-v0-spike-findings.md) in place.
- [ ] **#65h Structured repair prompt** — `wos-synth-core`: feed **`rule_id`**, **`suggested_fix`**, **`spec_ref`** (structured block or JSON), not only `LintDiagnostic` `Display` text (cheapest prompt-engineering win per spike).
- [ ] **#65i Conformance document gate API** — Prefer upstream **`wos_conformance::smoke_test_document` (or equivalent)** over ad-hoc inline `ConformanceFixture` wrappers duplicated across spike, synth-core, and future bench (spike Option B).
- [ ] **#65j `wos-lint` parse error typing** — Replace **substring** matching for missing `$wos*` marker with a typed discriminant or stable error code (spike finding).
- [ ] **`wos-synth-spike` disposition** — Per spike doc: crate-level **`[spike — do not extend]`** on entrypoint, **port** inline-fixture + `classify_lint_error` coverage to `wos-synth-core` tests, **delete** spike on 2–3 month horizon. Snapshot row already notes keep-with-deletion.

**Synth provider + schema hygiene** `[5 / 3 / 3]` (**15**)

- [ ] **#65k Anthropic prompt caching** — `AnthropicPrompter` currently folds `CacheAnchor` data into the system prompt verbatim until the Anthropic SDK exposes cache control; wire real cache blocks when available (`crates/wos-synth-anthropic/src/lib.rs`).
- [ ] **#65l `SynthTrace` schema drift test** — `schemas/synth/wos-synth-trace.schema.json` exists; add/verify **schemars (or equivalent) round-trip validation test** per synth plan Task 7 if not already present.
- [ ] **#65m `ToolContext` trait discipline** — Synth plan addendum: **do not extend** `ToolContext` with speculative methods until a second implementation (#65a) proves the shape.

**Authoring plan vs shipped crate** `[4 / 3 / 2]` (**8**)

- [ ] **#65n Reconcile `2026-04-17-wos-authoring-crate.md`** — Plan file layout (`handlers/*.rs`, long checkbox ladder) **diverges** from shipped `raw.rs` / `command.rs` / `project.rs`. Either update the plan to match reality or extract a **gap list** (MCP tools ↔ `WosProject` helpers) so obsolete steps are not re-executed.

**Hygiene**

- [ ] **#65o Plan checkbox refresh** — After #65n, mark landed MCP/synth/authoring plan `- [x]` steps against `main` (or add banner: *checkboxes frozen — see `work-spec/TODO.md` ADR 0065 section*).

### Behavioral / governance (1.0 scope under minutes-not-days)

Per repo-root [`VISION.md`](../VISION.md) operating frame: no "defer to 1.1" bucket on greenfield. These all land at 1.0 unless explicit architectural prerequisite unresolved.

**Stack contracts (ADRs 0066, 0067):**

- **Identity attestation shape — generalize beyond signatures** `[5 / 3 / 4]` (**20**) — WOS-T4 runtime emission now has `SignatureAffirmation.identityBinding` as the first concrete shape. This item generalizes that shape for reuse across non-signature evidence (reviewer-policy assurance refs, amendment-authority attestations, review-gate credentials). **Coordinates with parent PLN-0381, PLN-0380, PLN-0384. Gate: T4 runtime emission landed; parent stack ADR ratification pending.**
- **ADR 0066 implementation** and **ADR 0067 implementation** are tracked as Do-next items #3–#5 above — not duplicated here. The Actor authorization shape is Do-next #3.

**Maximalist cluster follow-ups (post-Session 14–16):**

The 2026-04-28 cluster ratification landed 14 new `ProvenanceKind` variants + closed enums + DNS-tenant cap + five-mode amendment taxonomy + `InstanceStatus::Stalled` declaratively at HEAD. The items below close the **declarable-but-not-fired** gap at the runtime/adapter boundary — same shape as #2 (capability-precondition emission) and #67 ConfigurationWarning. Without them, schemas + lint ratchet ahead of the runtime and the next conformance-suite expansion will surface a wave of "declared-but-never-emitted" gaps.

- [ ] **#70 `DurableRuntime::AppendFailure` typed enum** `[6 / 4 / 5]` (**30**) — Replace `Result<_, RuntimeError>` failure surface in the `DurableRuntime` adapter contract with a closed `AppendFailure { Retryable, BudgetExhausted, Terminal }` enum carrying typed reason codes. Today every adapter (in-memory + Restate + future) uses `RuntimeError` (not `String` as prior TODO claimed) but classification of commit-attempt outcomes still string-matches into branching logic. **Why:** [ADR 0070](../thoughts/adr/0070-stack-failure-and-compensation.md) D-4.3 pins commit-failure taxonomy as substrate-classified, retry-budget-aware, with `Stalled` as the terminal lifecycle state. **Not started:** no `AppendFailure` enum exists; `CommitFailureKind` enum exists in provenance layer only (`record.rs:96` — `NetworkTimeout|SubstrateDown|HashConflict|Other`); no `CommitAttemptFailure` conformance fixture. Composes with **#72**. **Gate: ADR 0070 still Proposed (cluster-ratification pending).**
- [ ] **#71 `ReinstatementPolicy` schema $def + lint K-A-010** `[6 / 3 / 4]` (**24**) — Add `ReinstatementPolicy` $def to `wos-workflow.schema.json` Workflow Governance embedded block (parallel to `amendmentPolicy` / `rescissionPolicy`); register lint K-A-010 enforcing the closed five-mode amendment taxonomy. **Not started — `Reinstated` provenance kind exists (`kind.rs:350`) but no governance policy shape or lint.** **Gate: ADR 0066 still Proposed.**
- [ ] **#72 Reference-runtime emission wiring for cluster variants** `[6 / 6 / 4]` (**24**) — Wire the 14 cluster `ProvenanceKind` variants into runtime emission sites. Constructors exist; schema guards exist; audit-tier dispatch exhaustive. **Zero runtime emission sites — blocked by #70 + ADR 0070 gate.** **Gate: ADR 0070 still Proposed + #70 not started.**

**Prior behavioral items:**

- **#35 Equity Config enforcement semantics** `[7 / 5 / 4]` (**28**) — processor obligations for `RemediationTrigger.action`; wire `DisparityMethod` to runtime. Prerequisite: #36 resolved (stack vision: FEL + restricted-domain profile).
- **#36 Equity RemediationTrigger expression language** `[6 / 4 / 4]` (**24**) — FEL + restricted-domain profile per [`VISION.md`](../VISION.md) / WOS §X; no windowing escape hatch. Implementation.
- **#26a `AccessControl.canRead` enforcement semantics** `[6 / 3 / 4]` (**24**) — normative processor behavior on `canRead → false`: redact / null / raise / skip. Prerequisite to #26b.
- **#26b `caseFieldPolicy` schema** `[6 / 6 / 4]` (**24**) — per-field read/write scopes by actor role.
- **#43 Assurance × impact-level composition** `[6 / 5 / 4]` (**24**) — minimum Assurance floor per impact level (rights-impacting ≥ `high`; safety-impacting ≥ `high`; operational ≥ `standard`) per stack vision. **§4.7:** normative home for the signature-class ↔ assurance-level binding (ESIGN=L1, eIDAS-advanced=L3, QES=L4+QSCD); resolves Open Q15.
- **#24b + #25 joint design** `[#24b 7/6/4 · #25 6/7/6]` — Reasoning tier rule-firing trace + Catala-style defeasibility. Vision model: `workflow-governance` with `(sourceAuthority, priority)` lexicographic. After ADR.
- **#38 G-064 Assertion Library resolution lint** `[5 / 3 / 3]` (**15**) — implementation of the lint designed in session 8.
- **#40 Task SLA runtime implementation** — beyond the session-8 authoring surface; wire §10.3 runtime obligations. **§4.7:** the spec home for envelope reminders + expirations once runtime fires `slaDefinitions` / `warningThresholds` / `breachPolicy`.
- **Bulk Operations spec** (relocated from Future specs) — admin-portal-driven; parallel case instantiation + bulk state transitions.
- **#28 Claim-check artifact references** `[4 / 4 / 5]` (**20**) — typed `ExternalArtifactRef { uri, contentHash, hashAlgorithm, mediaType }`.
- **#30 WS-HumanTask lifecycle completion** `[5 / 5 / 2]` (**10**) — task-level `Suspended`, distinct `Cancelled`, explicit `Return` with rework counter. **§4.7:** task-level decline / return is half of signer-decline semantics; pairs with #58 envelope-status for instance-level decline / void / expire.
- **#27 Cancellation regions** `[4 / 6 / 3]` (**12**) — YAWL-style named regions distinct from `cancellationPolicy` join policy.
- **#29b Milestone reactive transition firing (GSM-style)** `[6 / 5 / 2]` (**12**) — ships after #29a (landed session 4).
- **#3 Policy-based migration routing** `[5 / 6 / 2]` (**10**) — `migrationPolicy: grandfather | migrateAll | migrateByState | expression`. Tenant-scope sub-question finalizes with `DurableRuntime` tenant contract. **§4.7:** tenant-scope sub-question blocks multi-tenant envelope deployments (Open Q7 refers).

### Hygiene / refactors

Sequenced for module-bottleneck relief, not delayed by it.

- **#22 Crate split along tier boundaries** `[5 / 3 / 3]` (**15**) — `wos-core` → `wos-{kernel,governance,ai,advanced}`; `wos-runtime/runtime.rs` (still a large single module; ≈3.7k lines) split along action-kind dispatch; CI fence. (**#22a** provenance module split + `ProvenanceAuditTier` landed 2026-04-21 — see [COMPLETED.md](COMPLETED.md).)

### Reference server

Work items, architecture, and adapter sequencing → [`crates/wos-server/TODO.md`](crates/wos-server/TODO.md) and [`crates/wos-server/VISION.md`](crates/wos-server/VISION.md).

**Active:** ADR 0082 wholesale greenfield landing bundle — plan at [`thoughts/plans/2026-05-06-adr0082-wholesale-greenfield-landing.md`](thoughts/plans/2026-05-06-adr0082-wholesale-greenfield-landing.md). Server WS-1 Phases A–C landed (98 utoipa annotations, `domain/` deleted, 210/210 tests green). WS-2 portal rebuild pending.

### Runtime Companion parity

- **#66 Runtime §15 processor parity** `[7 / 5 / 5]` (**35**) — umbrella; decomposed into #66a–#66g below. Full context at [`crates/wos-server/TODO.md`](crates/wos-server/TODO.md) (WS-011, WS-074, WS-075).
  - [ ] **#66a Typed submit rejections + replay**
  - [ ] **#66b Agent submitters**
  - [ ] **#66c `ledgerEvidenceMissing` placement**
  - [ ] **#66d `contractHook` / Governance S5 post-pass**
  - [ ] **#66e Abandonment + skip semantics** — `ProvenanceKind::TaskSkipped` is the only variant with zero live emission.
  - [ ] **#66f Amendment task linkage** — coordinates with ADR 0066.
  - [ ] **#66g Conformance fixtures** — auth reject, agent reject, ledger missing, hook fail, skip vs fail.

### Verifiability

- **K-DET-001 determination-snapshot conformance + fixture migration** `[6 / 3 / 5]` (**30**) — conformance gate for Facts-tier snapshots on determination transitions.
- **Seeded LoadBearing-promotion batch + rule-coverage CI** `[6 / 4 / 4]` (**24**) — 1 LoadBearing rule today; land promotion set + CI gate together.
- **#52 Simulation trace format** `[4 / 3 / 2]` (**8**)

### ADR 0064 residual

- [ ] **Structured `LintDiagnostic` output contract** `[6 / 5 / 4]` (**24**) — machine-stable JSON per rule; prerequisite for LLM repair loops.
- [ ] **Trace-emitting conformance** `[6 / 5 / 5]` (**30**) — teachable traces/deltas, not only pass/fail.
- [ ] **COMP-001 companion drift lint** `[4 / 2 / 4]` (**8**) — trigger-gated.

### Regulatory (1.0)

- **#50 EU AI Act alignment** `[7 / 5 / 4]` (**28**) — Art. 13-14 alignment spec.
- **#53 OMB M-24-10 compliance** `[6 / 4 / 3]` (**18**) — process-documentation-shaped; overlaps Assurance + impact-level plumbing.

- [ ] **§5.6 Repositioning/demo artifacts — gap closure vs handoff** `[4 / 2 / 2]` (**8**) — Verify README.md leads with two-claim framing (not "AI-native" tagline) per handoff §5.6, and author a demo narrative (requirement → workflow trace) once `wos-synth` is stable. Partially satisfied by `POSITIONING.md`; gap closure never explicitly verified. Gate: none (lightweight docs).

### Interoperability + speculative (trigger-gated)

- **SCXML interoperability** `[3 / 6 / 2]` (**6**) — bidirectional WOS ↔ SCXML mapping. Trigger: ecosystem demand.
- **#51 Statutory deadline chains** `[4 / 7 / 5]` (**20**) — must compose with #31 business calendars + typed kernel events (`TransitionEvent`, #20). Trigger: first production deployment exposes concrete need.

---

## Moved to Trellis (scope-out)

Per [`VISION.md`](../VISION.md) §XI, Trellis is the integrity layer and owns these concerns. WOS emits records via `custodyHook`; Trellis anchors them. Tracked here only to close the loop on items that used to be listed as WOS work.

- **#48 Merkle provenance chains** — Trellis. Hash-chaining + SCITT alignment are Trellis primitives.
- **Federation Profile** (cooperative trust-anchor network) — Trellis. Previously tracked as WOS Future spec.
- **SCITT strictness** (full vs. adjacent) — Trellis decides.
- **Checkpoint seal protocol** — Trellis.
- **Proof-of-inclusion + transparency-log submission tooling** — Trellis.
- **Certificate-of-completion export bundle format** — Trellis export-bundle primitive.

---

## Blocked / needs decision

Items that can't move without a verdict or an external trigger.

### §4.5 PR packaging

`VISION.md` / practice recommends three discrete PRs; sidecar audit recommended one. Owner picks. See Do-next-adjacent "Structural merges" section above.

### Engine adapters — trigger-gated (commercial request)

WOS's first production runtime target is now the Restate adapter selected by WOS-T3. Additional adapters are trigger-gated on commercial adopter request or SDK maturity.

- **#49a Camunda 8 Worker** `[5 / 8 / 3]` — BPMN target; broadest external fixture diversity.
- **#49c AWS Step Functions** `[5 / 8 / 3]` — broadest commercial reach; narrowest semantic fit.

(#49b Temporal was evaluated by WOS-T3 and deferred until the Rust workflow API stabilizes.)

### Ontology field identity — design not started

`ontology-spec.md` does not exist. [ADR 0076 (product-tier consolidation)](../thoughts/adr/0076-product-tier-consolidation.md) settles the lane: semantic projection/import belongs to the `wos-ontology-alignment` sidecar, not Kernel substrate. (The earlier "ADR 0082" citation here was a stale draft-number reference; ADR 0082 is now the [Stack Public REST API Contract](../thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md) and is unrelated to ontology work.) Remaining prerequisite design: semantic-field-identity protocol, cross-document alignment, and executable projection/import conformance. Move to active only once a draft exists.

---

## Deferred (with triggers)

Captured but not active; re-score when the trigger fires.

| IDEA # | Item | Imp/Cx/Debt | Trigger |
|---|---|---|---|
| #1 | Agent Behavioral Attestations | 2/7/1 | SLSA-style AI-agent attestation ecosystem matures. |
| #4 | Tripartite Object Model | 2/9/3 | Activity-definition reuse across workflows becomes a real pattern. |
| #6 | Typed Patch Operations | 1/8/0 | Authoring tool ships structural edits. |
| #7 | OCEL 2.0 Object-Centric Case Model | 2/9/5 | Multi-object mutation emerges, or flat→OCEL export shows systematic loss. |
| #9 | JSON-LD Projection/Import Surface | 5/5/3 | Ontology spec drafts begin OR shipped PROV-O pulls `@context` into authoring. |
| #32 | Multi-Instance Iteration | 6/7/5 | #20 landed — unblocked. Highest-priority deferred item. |
| #33 | Inclusive-OR / Event-Choice / Boundary Events | 3/5/2 | Authoring frustration with workarounds (externally observable signal). |

---

## Future specs (trigger-gated)

Federation Profile and Bulk Operations relocated — see "Moved to Trellis" and Backlog / behavioral items respectively.

| Spec | Description | Trigger |
|---|---|---|
| Learning Profile | Retraining governance | Long-lived AI agents need retraining policy. |

---

## Rejected

Decisions locked; do not re-litigate.

| IDEA # | Item | Reason |
|---|---|---|
| #5 | DAG Processing Model | Contradicts axis 4 (append-only event-stream folding); reactive re-evaluation explicitly rejected. |
| #8 | FEL Conformance Profiles | Kernel §7.4 rejects grammar extensions. |
| #10 | WCOS + FEEL | Rename + DMN expression language both abandoned. |
| #17 | SHACL | Existing Rust lint (55 T2 rules) covers cross-doc validation; shipped PROV-O is JSON-LD. |
| #18 | Minimal Governance Envelope | Strip lifecycle from kernel → doc that cannot be understood in isolation. |
| #19 | FEEL Expression Language | FEL is purpose-built; FEEL carries DMN assumptions. |
| — | BPMN Parity as Authoring Goal | Export target, not authoring surface. Event taxonomy adopted normatively via #20. |

---

## Parked

- Full lifecycle soundness verification (e.g. linear-time logic). Advanced Governance SMT is the path.
- JSON Patch for fine-grained provenance.
- FEEL-to-FEL migration guide — on-demand, write when first DMN shop asks.

---

## Open architectural questions

Most prior open questions (OQ1, OQ4, #21 Registry composition, #25 Defeasibility, #36 Equity expression, #43 Assurance × impact-level, #9 JSON-LD authoring) are now **resolved** per [`VISION.md`](../VISION.md) §X (WOS settled commitments) and linked ADRs. Remaining genuinely-open decisions:

1. **§4.5 PR packaging** (sidecar-audit Q1). One PR (audit recommendation) or three (`VISION.md` / historical practice: discrete PRs for review hygiene)?
2. **`custodyHook` evidence path to Trellis** — WOS→Trellis **authored append wire** is fixed by [ADR-0061](thoughts/adr/0061-custody-hook-trellis-wire-format.md) and WOS-T1 closeout (four-field input, `(caseId, recordId)` idempotency, `CustodyAppendReceipt` → `canonical_event_hash` on provenance). **Open** is cross-stack **proof**: the landed Formspec canonical signed-response artifacts still need Trellis append/export vectors staying byte-aligned with live WOS emitters, plus Studio authoring gates. This is the same bundle as WOS-T4 “Next slice” / Trellis verification maintenance, not an undecided WOS wire-format ADR.

For stack-wide active uncertainties (DocuSign parity scope, multi-tenant on Restate/Temporal, rendering service for signature artifacts), see [`VISION.md`](../VISION.md) §X *Active uncertainties (WOS-scope)*.

---

*Closed-out work is archived in [COMPLETED.md](COMPLETED.md). Append there, not here.*
