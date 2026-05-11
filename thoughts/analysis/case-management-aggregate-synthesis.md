# Case Management Boundary: Synthesis (v2)

**Date:** 2026-05-11
**Status:** Fully replaces v1 (2026-05-10). v1 is preserved in git history; this document is the new pre-ADR source of truth.
**Supersedes:** prior `work-spec/thoughts/adr/0093-case-process-boundary.md` (Proposed, deleted 2026-05-11). Replacement landed at [`work-spec/thoughts/adr/0093-case-is-its-trellis-ledger.md`](../adr/0093-case-is-its-trellis-ledger.md) (Proposed, 2026-05-11). Withdraws ~⅔ of the v1 CASE-SYNTH register.
**Author intent:** Greenfield re-think. Per owner direction (2026-05-11), every ADR/spec/plan/decision from v1 is treated as disposable. What follows reasons from user value first, then collapses architecture to the minimum that serves it.

**Validation pass (2026-05-11, post-draft):** An independent code/spec audit (Explore subagent against `work-spec/api/wos-public-api.openapi.json`, `workspec-server/`, `work-spec/specs/kernel/spec.md`, `trellis/specs/trellis-core.md`, `trellis/crates/`) confirmed the architectural spine but caught six precision issues that have been folded back in: (1) the `GET /case/{case_id}` path was fictional — real routes are staff `/api/v1/instances/{id}` and applicant `/api/v1/applicant/cases/{id}`; (2) kernel §5.1's *lifecycle vs case-state independence* rule is real and preserved — what v2 declines is a second `caseState` aggregate boundary, not §5.1 itself; (3) `GOAL.md:48` is general posture, not an ADR-0074 pin — re-anchored to ADR-0074 first; (4) Trellis §14.5 *Registry migration discipline* adds precision to the registry-binding citation; (5) the direct-event-append endpoint exists at `POST /api/v1/instances/{id}/events`, role-gated to `Adjudicator` — `note.added` write surface is named; (6) `case_<ulid>` today mints workflow-instance identity (`mint_case_id`); the transition to ledger identity is named explicitly in the ADR. The architectural claims (case = ledger; one read path; one write path; closed event family) stand unchanged.

---

## 1. The collapse

**A case IS its Trellis ledger.** Nothing more.

- **One entity:** the **case ledger** (Trellis-shaped, hash-chained, append-only, per-class encrypted).
- **One identity:** `case_<ulid>` — the ledger ID *is* the case ID.
- **One write path:** append a typed event via `custodyHook`.
- **One read path:** derive the current view by event replay (or read a denormalized projection — operational, not architectural).

A **workflow** is a runtime process (Restate / Temporal / in-memory adapter) that **binds** to a ledger and emits events on it. The process is not the case. When the process ends, its events remain on the ledger. The ledger keeps existing.

There is no separate `Case` aggregate. There is no `CaseProcess` domain type. There is no second source of truth. There is no projection-vs-canonical distinction at the type layer — only at the operational layer, where projections are rebuild-on-demand views.

---

## 2. Why this is true (user-value derivation)

Working backwards from what users actually need:

| Actor | Story | Satisfied by |
|-------|-------|--------------|
| **Applicant** | "I submit a benefits form. Three months later I appeal. The system knows it's the same matter." | One ledger ID per matter; survives all workflow lifecycles. |
| **Caseworker** | "I open Jane's case. I see her notes, history, every decision, regardless of which workflow ran." | Replay or projection of the ledger. |
| **Caseworker** | "I add a note. It persists after the intake workflow ended." | Direct event append (`note.added`). |
| **Caseworker** | "Jane's appeal starts. I attach a new workflow to her existing case." | Bind a new process to the existing ledger. |
| **Regulator** | "Hand me Jane's full case file. Verify integrity. Show ordering." | Trellis export bundle = the ledger. |
| **Developer** | "I write a workflow that updates `determination`. There's exactly one way to do that." | Typed event append; single read-side API. |

Every story is satisfied by **the single primitive we already have**: a Trellis case ledger. No story requires a separate `Case` aggregate. The reverse — modeling `Case` separately — *creates* failure modes (dual-state crash recovery, projection-lag-as-bug, two-source-of-truth ambiguity) without serving any story.

---

## 3. The architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│                      Case Ledger (Trellis, durable)                  │
│                          — hash-chained                              │
│                          — per-class encrypted (ADR-0074)            │
│                          — one ID: case_<ulid>                       │
│                                                                      │
│   case.created → process.started → note.added → process.transitioned │
│   → artifact.attached → decision.recorded → process.completed →      │
│   process.started (appeal) → ... → case.closed                       │
└──────────────────────────────────────────────────────────────────────┘
              ▲                                          │
              │ (typed event append via custodyHook)     │ (replay or read)
              │                                          ▼
┌──────────────────────────────┐         ┌──────────────────────────────┐
│  Workflow runtime            │         │  Read-side projection        │
│  (Restate / Temporal /       │         │  (operational, rebuildable,  │
│   in-memory adapter)         │         │   plaintext-content-free per │
│  — ephemeral state           │         │   wos-server VISION.md:98–101│
│  — crash-recoverable         │         │   canonical/projections split│
│  — emits ledger events       │         │  )                           │
│    via $defs/OutputBinding   │         │  Staff: GET /instances/{id}  │
│  Direct surface (today):     │         │  Applicant: GET /applicant/  │
│   POST /instances/{id}/events│         │           cases/{id}         │
│   (role: Adjudicator)        │         │                              │
└──────────────────────────────┘         └──────────────────────────────┘
```

Three tiers, two of them operational:

1. **Source of truth** — the ledger (durable, authoritative; Trellis-shaped per `trellis-core.md` §10.1, §10.4, §23.2 item 5).
2. **Working state** — workflow runtime state (ephemeral, in the chosen substrate; `DurableRuntime` adapter).
3. **Read-side views** — projections (rebuildable, replaceable; per-deployment choice).

The v1 three-headed "Trellis + WOS Case identity + Case projection" model collapses to **two layers** (durable ledger + derived view). Workflows are runtime executions, not architectural entities.

---

## 4. The event family (the entire domain model)

Closed enum of canonical event types, all under the `wos.*` namespace (Trellis-reserved per `trellis-core.md` §23.4), all registered in the bound registry per §23.2 item 2 + §14.

**Lifecycle events**
- `case.created` — WOS-only emitter (preserves ADR-0073 D-1 ownership). Payload: tenant, class, optional `IntakeHandoff` reference, optional bound process ID.
- `case.closed` — terminal-but-optional (cases may remain open indefinitely; closure is a state, not a requirement).
- `case.status_changed` — application-defined status transitions, distinct from workflow process lifecycle.
- `case.related_to` — relationship edge (`parent | child | sibling | related | supersedes` per current kernel §5.5 taxonomy; extensible via `x-`).

**Process events** (workflow-runtime emissions)
- `process.started`
- `process.transitioned`
- `process.completed` / `process.failed` / `process.suspended` / `process.resumed` / `process.terminated`

**Domain-content events**
- `note.added`
- `artifact.attached` — wraps a Formspec response or external document; carries the four-field `CaseOpenPin` from ADR-0071 D-1 for replay-safe versioning.
- `decision.recorded` — adjudicatory output; carries `verificationLevel` + signature affirmation reference.
- `signature.affirmed` — surfaces existing WOS `SignatureAffirmation` semantics into the ledger event stream (no second meaning of "signed"; preserves work-spec/CLAUDE.md Signature-shortcut rule).

**Extension events**
- `x-<namespace>-<name>` — vendor extension per kernel §10.6; no registry binding required.

Every WOS MUST that produces an audit event maps to exactly one of the above. `wos-provenance-log.schema.json` is the schema home; the existing `$defs/CaseCreatedRecord` (with `event const: "case.created"` at line 123) is the prototype for every sibling event type.

---

## 5. The minimum viable spine

Five items. The entire refactor.

1. **Event-schema family** — extend `wos-provenance-log.schema.json` with the closed event-type enum from §4. Existing `CaseCreatedRecord` is the prototype shape.
2. **Trellis registry binding** — register each new `wos.*` event_type in the Trellis bound registry per §23.2 item 2 + §14 + §23.4. Phase-1 step-zero. Edits land in the Trellis repo.
3. **Workflow runtime → ledger emission** — workflow processes use the existing `$defs/OutputBinding` (canonically pinned at kernel **§9.2.18 Overview**, `kernel/spec.md:1127–1129`: "Each binding is an `OutputBinding` entry … through the validated output-commit pipeline (ADR 0080)"). No schema change. No new `target` discriminator. Discipline-only.
4. **Read-side API** — *architectural commitment:* one read path returning a derived view. *Implementation today:* two surfaces in `work-spec/api/wos-public-api.openapi.json` — staff `GET /api/v1/instances/{id}` (line 516) and applicant `GET /api/v1/applicant/cases/{id}` (line 4277). Both return a derived view of the same case ledger. Resource-naming convergence (rename staff route under `/cases/{id}`) is a follow-up per ADR 0082.
5. **Per-class encryption (ADR-0074, Proposed)** — wraps event payloads. Already designed; no new work in this scope. Stack Case API posture cites ADR-0074 + deployment profile + ratification gate together.

No Phase 1 / Phase 2 split. The whole thing is one slice.

---

## 6. What this supersedes / withdraws (boy-scout list)

Every item below is **WITHDRAWN — superseded by this synthesis v2**. Most were artifacts of debugging a self-inflicted modeling problem.

### v1 architectural decisions

| Decision | Status | Why withdrawn |
|----------|--------|---------------|
| Two-TypeID model (`case_` for `CaseProcess`, `casefile_` for `Case`) | WITHDRAWN | Single `case_` identity. `WosResourceUrn` already encodes `case` as a family literal (`_common.schema.json:20`). |
| `CaseProcess` as a renamed `CaseInstance` domain type | WITHDRAWN | Workflow processes are runtime constructs, not domain types. |
| `target` discriminator on `$defs/OutputBinding` | WITHDRAWN | Event types ARE the write discriminator. No schema property needed. |
| Kernel §5 process-scoped vs case-scoped `caseState` bifurcation | WITHDRAWN | `caseState` is a derived view; no bifurcation needed. |
| `ADR-0073-bis` follow-up ADR for manual case creation | WITHDRAWN | Manual creation = direct API emission of `case.created`. Same operation. |
| `ADR-0093` ("Case / Process Boundary," Proposed) | SUPERSEDED | Replaced by ADR 00YY (draft in §8). |
| v1 §2 "Architectural Triad" (three-headed Trellis / WOS Case / Projection model) | WITHDRAWN | Collapsed to two-tier (ledger + derived view). |
| v1 §5 "Phased Execution Plan (MVP vs. Post-MVP)" | WITHDRAWN | Refactor is small enough to land as one slice. |

### v1 CASE-SYNTH register

| Item | Old framing | Why withdrawn |
|------|-------------|---------------|
| CASE-SYNTH-01 | `$wosCaseInstance` → `$wosProcessInstance` rename debate | Marker becomes runtime-checkpoint metadata, not domain truth. Debate moot. |
| CASE-SYNTH-02 | New `casefile_` / `matter_` TypeID prefix for `Case` | Single identity: `case_<ulid>`. |
| CASE-SYNTH-04 | ADR-0073-bis for manual case creation | Single creation path with single event. |
| CASE-SYNTH-07 | Preserve 35-edge-case matrix as Phase-1 failing fixtures | Most edges only existed because of dual-aggregate model. Re-derive only edges that survive collapse (process emission ordering, replay determinism). |
| CASE-SYNTH-10 | "Case-as-projection vs WOS-centered domain model" alternatives | Both alternatives share the false premise. Both withdrawn. |
| CASE-SYNTH-11 | Case ↔ Trellis-ledger cardinality | 1:1 by definition (the ledger IS the case). Not a decision. |
| CASE-SYNTH-12 | Dual-state crash recovery for Case + CaseProcess | Projection has no authority. Crash → drop projection, replay. No new failure mode. |
| CASE-SYNTH-13 | Enumerate "metadata only" Case projection fields | Projection schema is operational, per-deployment. |
| CASE-SYNTH-14 | Phase 2 Case-projection schema evolution | Same — operational, not architectural. |
| CASE-SYNTH-18 / 27 | 1:1 deployment profile vs ontology | A ledger may carry N concurrent processes. Not a profile choice. |
| CASE-SYNTH-21 | Kernel §5 instance-scoped vs case-scoped bifurcation | `caseState` is a view; no bifurcation. |
| CASE-SYNTH-22 | Phase 2 multi-process write-conflict policy | Standard append-log resolution. Not a new design problem. |
| CASE-SYNTH-29 | Source-authority map for "schema version" pinning | Replaced by direct citation discipline (D-12 below). |

### v1 reviewer findings (R1–R5 + 8-agent swarm)

The convergent claim across all reviewers — *Case is not a second source of truth* — is **preserved**. Every recommendation that proceeded *as if* `Case` were a separate aggregate is **withdrawn**. The 10 findings from the 2026-05-11 revalidation pass were all corrections *to the broken aggregate model*; in this collapsed model, the equivalent claims become trivially true and need no remediation:

| v1 FINDING | v2 status |
|-----------|-----------|
| 1 — Kernel §9.2.22 mis-cited as home of `$defs/OutputBinding` | Carried forward as D-5 (pin at §9.2.18). |
| 2 — "Process-scoped" vs spec's "instance-scoped" | Reframed in D-4: kernel §5.1 *lifecycle vs case-state independence* is preserved; what v2 declines is a *second `caseState` aggregate boundary* (instance-scoped vs case-scoped). |
| 3 — Four-field `CaseOpenPin` (not just `definitionUrl`/`definitionVersion`) | Carried forward as D-12. |
| 4 — ADR-0074 status (Proposed, broader than Response) | Carried forward as D-10. |
| 5 — `formspec-bucketing` false-positive | Stays withdrawn (was already a false alarm). |
| 6 — `custodyHook` emission seam location | Moot: no separate Case aggregate to claim a struct emission point. |
| 7 — Manual-creation seam narrower than admitted | Moot: no ADR-0073-bis. |
| 8 — Dangling ADR-0077 references | Patched 2026-05-11; carried forward as D-14. |
| 9 — Tasks-route framing | Carried forward as D-13. |
| 10 — Trellis §15 framing ("watermark/rebuild" not "projection discipline") | Carried forward as D-1 supporting citations. |

---

## 7. Tradeoffs

What we give up by collapsing:

- **The "Case is a first-class typed domain object" mental model.** *Mitigation:* present `Case` *at the API surface* as a coherent JSON document (D-6) — today via the existing `instance.schema.json` view served at staff `GET /api/v1/instances/{id}` and applicant `GET /api/v1/applicant/cases/{id}` routes. Truth layer is the ledger; presentation layer is what callers see.
- **Compile-time validation against a single "Case schema."** Replaced by per-event-type JSON Schema validation on payloads. Equivalent expressive power; events are inherently versioned via Trellis envelope versioning + ADR-0071 D-1 pins.
- **A small risk** that some future requirement genuinely demands an aggregate boundary (e.g., distributed transactions across cases). Vanishingly unlikely in this domain (one case = one matter = one tenant scope); resolvable later by promoting a projection to authoritative iff proven necessary.

What we get:

- One concept, one identity, one write path, one read path.
- ~⅔ of the v1 CASE-SYNTH register dissolves.
- Both pending ADRs (0093 + 0073-bis) collapse to one short ADR.
- No new failure modes (dual-state recovery, projection-lag-as-bug, write-conflict policy) to design.
- Alignment with what the architecture *already* says (Trellis = canonical events; projections = derived views per `wos-server/VISION.md`).
- **Pre-release window leveraged.** Per `work-spec/CLAUDE.md` and platform decision register: no backwards compatibility, nothing shipped. The cost of collapsing today is editing some specs and one ADR. The cost in 12 months is migrating fixtures, projections, downstream tools, partner integrations, customer data. **The asymmetry is the entire reason to do it now.**

---

## 8. Recommended ADR (replaces ADR-0093)

Landed 2026-05-11 at [`work-spec/thoughts/adr/0093-case-is-its-trellis-ledger.md`](../adr/0093-case-is-its-trellis-ledger.md) (replaces the deleted `0093-case-process-boundary.md` of the same ADR number). The §8 sketch below is preserved here for traceability; the authored ADR is more thorough but architecturally identical.

> ### ADR 0093 — A Case Is Its Trellis Ledger
>
> **Status:** Proposed (replaces the prior ADR-0093 of the same number, 2026-05-11).
> **Date:** 2026-05-11
> **Scope:** WOS — case identity, governed output, provenance event family.
> **Related:** ADR-0070 D-1 (Trellis commit point); ADR-0071 D-1 (four-field `CaseOpenPin`); ADR-0073 D-1 (`case.created` ownership); ADR-0074 (per-class encryption, Proposed); ADR-0080 (governed output-commit pipeline, Proposed); kernel §10 six extension seams (archived ADR-0077, Implemented); Trellis Core §10.1, §10.4, §15, §23.2, §23.4.
>
> **Decision.** A case is the Trellis ledger scoped to one matter. Nothing more. The ledger ID is the case ID (`case_<ulid>`). All durable case state is encoded as typed events appended to this ledger. Workflows are runtime processes that bind to a ledger and emit events on it. Current case state is derived by event replay or by reading a denormalized projection (operational concern). There is no separate `Case` aggregate, no separate `CaseProcess` domain type, no second TypeID family, no projection-vs-canonical distinction at the type layer, and no kernel-§5 `caseState` bifurcation.
>
> **Event family.** Closed enum under `wos.*`: lifecycle (`case.created`, `case.closed`, `case.status_changed`, `case.related_to`), process (`process.{started,transitioned,completed,failed,suspended,resumed,terminated}`), domain content (`note.added`, `artifact.attached`, `decision.recorded`, `signature.affirmed`), vendor extension (`x-*-*`). All types except `x-*-*` MUST be registered in the Trellis bound registry per §23.2 item 2 + §14 + §23.4 before emission. Registry edits land in the Trellis repo.
>
> **Writes.** Workflow processes emit events via existing `$defs/OutputBinding` (canonically pinned at kernel **§9.2.18 Overview**). No new schema property. No `target` discriminator.
>
> **Reads.** Architectural commitment: one read path returning a derived view. Current implementation: staff `GET /api/v1/instances/{id}` (`work-spec/api/wos-public-api.openapi.json:516`) and applicant `GET /api/v1/applicant/cases/{id}` (line 4277). Both implement the same derivation contract — replay or projection per deployment, with projections in `wos-server/VISION.md`'s `projections` schema, plaintext-content-free. Resource-naming convergence (staff → `/cases/{id}`) is an ADR 0082 follow-up.
>
> **Pinning.** Every event payload that wraps a Formspec response (notably `artifact.attached` and `decision.recorded`) MUST carry the four-field `CaseOpenPin` from ADR-0071 D-1 (`formspec.definitionVersion`, `wos.$wosWorkflowVersion`, `trellis.envelopeVersion`, `trellis.conformanceClass`) plus the Formspec-axis details (`definitionUrl`+`definitionVersion` for Response, `definitionRef.url`+`definitionRef.version` for Intake Handoff).
>
> **Identity.** Single TypeID prefix: `case_`. `WosResourceUrn` already encodes `case` as a family literal (`work-spec/schemas/api/_common.schema.json:20`); no change.
>
> **Manual creation.** Direct API emission of `case.created` is equivalent to workflow-initiated emission. No follow-up ADR.
>
> **Supersedes.** The prior ADR-0093 (Proposed, 2026-05-10). Withdraws CASE-SYNTH-01, 02, 04, 07, 10, 11, 12, 13, 14, 18, 21, 22, 27, 29 from the v1 synthesis register.
>
> **Preserves.** ADR-0073 D-1; ADR-0070 D-1; ADR-0071 D-1; ADR-0074 (Proposed); ADR-0080 `$defs/OutputBinding` shape and §9.2.18 pin; kernel §10 six extension seams.

---

## 9. What changed from v1

This document fully supersedes the v1 synthesis as it stood on 2026-05-11 (post-FINDING patches). The v1 history — R1–R5 reviewers, 8-agent swarm, 30+ CASE-SYNTH items, 10 revalidation findings, prior ADR-0093 — is preserved in git for archaeology. **No content from v1 is normative going forward.**

Dependent-document follow-ups:

- **Authored 2026-05-11:** [`work-spec/thoughts/adr/0093-case-is-its-trellis-ledger.md`](../adr/0093-case-is-its-trellis-ledger.md) (Proposed). Replaces the deleted `0093-case-process-boundary.md`.
- **Drop** the ADR-0073-bis placeholder anywhere it appears in stack TODO / PLANNING / thoughts.
- **Drop** the "Case projection schema" / new-TypeID-prefix / kernel-§5-bifurcation / `OutputBinding`-`target`-discriminator work items wherever they appear (PLANNING.md, TODO.md, work-spec/TODO.md).
- **Preserve** ADR-0073 D-1, ADR-0070 D-1, ADR-0071 D-1, ADR-0074, ADR-0080, kernel §10 seams, archived ADR-0077 citation discipline (D-14).

The v1 reviewer files remain on disk under `work-spec/thoughts/analysis/case-management-validation-*.md` as historical record. They are not cited normatively from v2.

---

## 10. Decisions log

Every decision this synthesis makes, named, with rationale and what it replaces. Closed taxonomy.

| # | Decision | Replaces | Rationale |
|---|----------|----------|-----------|
| **D-1** | **Case = Trellis ledger.** A case is its ledger; no separate aggregate, no second source of truth. Trellis authority pins: §10.1 (strict linear order per scope), §10.4 (no competing canonical orders), §23.2 item 5 (chain is authoritative order); projection authority: §15 (snapshot/watermark/rebuild) + §2.1 class 4 (Derived Processor) + Operational Companion §14.2 (No Second Canonical Truth). | v1 §2 "Architectural Triad" three-headed model; v1 CASE-SYNTH-10 alternatives debate. | The ledger is already authoritative for events, ordering, integrity, export. A second authority creates dual-state failure modes without serving any user story. |
| **D-2** | **Single identity `case_<ulid>`** is both ledger ID and case ID. | v1 CASE-SYNTH-02 (`casefile_` new prefix proposal). | TypeID prefix proliferation served only the false aggregate boundary; `WosResourceUrn` already encodes `case` as a family literal. |
| **D-3** | **Workflow processes are runtime constructs, not domain types.** No `CaseProcess` rename of `CaseInstance` at the domain layer. `$wosCaseInstance` (if it survives at all) is a runtime-checkpoint document, not a domain marker. | v1 §4.1 / CASE-SYNTH-01 marker-rename debate. | A process is what executes a workflow against a ledger; the ledger holds the durable record. |
| **D-4** | **Closed typed-event family** under `wos.*` (lifecycle, process, content, extension). The event-type IS the write discriminator; no schema property captures it separately. The existing kernel §5.1 *lifecycle vs case-state independence* rule is **preserved**; what is declined is a second `caseState` aggregate boundary (instance-scoped vs case-scoped variants). | v1 §3.3 "governed output path" debate; v1 CASE-SYNTH-21 kernel §5 bifurcation (the instance-vs-case axis proposal); v1 proposed `target` discriminator on `OutputBinding`. | Event types are inherently discriminating, versionable, registry-bindable. Schema-level discriminators duplicate work the type system already does. Kernel §5.1's existing rule covers what needs covering. |
| **D-5** | **One write path:** typed event append via `$defs/OutputBinding`, canonically pinned at kernel **§9.2.18 Overview** (`kernel/spec.md:1127–1129`). | v1 §3.3 mis-pin to kernel §9.2.22 (which is "Request-Response Bindings" type, not the `OutputBinding` $def). v1 FINDING 1. | `OutputBinding` already exists with `on, contractRef, projection, writeScope, mutationSource, verificationLevel`. No change needed; discipline only. |
| **D-6** | **One read path (architectural commitment), two surfaces (today):** staff `GET /api/v1/instances/{id}` and applicant `GET /api/v1/applicant/cases/{id}` (`work-spec/api/wos-public-api.openapi.json:516, 4277`). Both implement the same derivation contract — event replay or denormalized projection per deployment. Projections are plaintext-content-free per `wos-server/VISION.md:98–101`. Resource-naming alignment (staff → `/cases/{id}`) is a follow-up per ADR 0082. | v1 CASE-SYNTH-13 (enumerate projection fields); CASE-SYNTH-14 (Phase 2 projection schema evolution). | Projection is operational, not architectural. Rebuilds from the ledger; schema evolution is a deployment concern. The "one path" claim is architectural — implementation may surface it through audience-specific routes. |
| **D-7** | **Manual case creation is direct API emission of `case.created`.** Same operation as workflow-initiated; same event; same registry binding; same authority chain (ADR-0073 D-1 ownership preserved). | v1 CASE-SYNTH-04 (ADR-0073-bis follow-up). | One operation, one ADR (this synthesis's §8 ADR). |
| **D-8** | **Multiple processes on one ledger.** A ledger accepts events from concurrent / sequential workflow processes. Conflicts resolve at the read-side (time-ordered events; last-writer-wins, or merge function, or FEL-guarded reject). | v1 CASE-SYNTH-18/27 (1:1 deployment-profile-vs-ontology); CASE-SYNTH-22 (Phase 2 write-conflict policy). | Standard append-log semantics. Not a new design problem. |
| **D-9** | **Crash recovery: drop projection, replay ledger.** Projection lag is not a failure mode; projection has no authority. | v1 CASE-SYNTH-12 (dual-state crash recovery for Case + CaseProcess). | A non-authoritative view doesn't need crash semantics; it's rebuildable. |
| **D-10** | **Per-class encryption on event payloads** per ADR-0074 (Proposed, Not started; normative authority). Deployment-profile context: `wos-server/VISION.md:78–82` (SBA-tier audited-decryption pattern) + `wos-server/VISION.md:98–105` (canonical/projections; clients-decrypt / servers-broker). Case API never flattens classified bodies into a top-level document. | v1 §3.2 "per-class encryption violation" framing. | Event payloads are exactly the bucketing unit ADR-0074 designs around. `GOAL.md:48` states the prod-MVP posture in general terms ("audited server-side decryption only; no Federal/Sovereign confidential-compute claim") and does **not** reference ADR-0074 by name — treat it as deployment-target context, not a normative pin. |
| **D-11** | **Trellis registry binding is Phase-1 step-zero.** Every new `wos.*` event_type registered in the Trellis bound registry per §23.2 item 2 + §14 + §23.4 before emission. Edits land in the Trellis repo. | v1 CASE-SYNTH-20 (preserved unchanged). | Trellis owns event-type namespace. WOS depends; doesn't self-register. |
| **D-12** | **Four-field `CaseOpenPin`** (ADR-0071 D-1) on every Formspec-wrapping event (`artifact.attached`, `decision.recorded`, etc.). Co-required: `formspec.definitionVersion`, `wos.$wosWorkflowVersion`, `trellis.envelopeVersion`, `trellis.conformanceClass`. Formspec axis carries `definitionUrl`+`definitionVersion` for Response (Formspec Core §6.4) or `definitionRef.url`+`definitionRef.version` for Intake Handoff (Formspec Core §2.1.6.1). | v1 CASE-SYNTH-24/29 ("Formspec versioning pinned to `definitionUrl`+`definitionVersion` alone"). v1 FINDING 3. | Replay correctness requires all four axes pinned; Formspec-only pin allows WOS/Trellis semantic drift. |
| **D-13** | **API ↔ schema drift policy.** `instance.schema.json` and `provenance.schema.json` are contract authority per ADR 0082. Server registers `suspend`/`resume`/`terminate` (currently absent from `instances.rs`); server renames `/explain` → `/explanation` (schema canonical at `provenance.schema.json:630`). `/instances/{id}/tasks` does not exist on either side. 9-vs-6 lifecycle enum: pick truth; document the projection rule. | v1 CASE-SYNTH-05/06/19. v1 FINDING 9. | Schema is authority; server follows. Drift items are routing changes, not new design. |
| **D-14** | **Archive citation discipline.** Archived ADR-0077 ("Canonical kernel extension seams," status Implemented) is cited as `formspec/thoughts/archive/adr/0077-*` with canon location pointing at kernel §10 + `work-spec/CLAUDE.md` heuristic 3. No stack-level `thoughts/adr/0077-*` file exists. Same pattern for any future-archived ADR. | v1 FINDING 8. Dangling references patched 2026-05-11 in `work-spec/CLAUDE.md`, `thoughts/adr/0076`, `0078`, `0080`. | Citation rot is itself a category of inherited bug; this is the standing fix pattern. |
| **D-15** | **Six kernel extension seams remain the only extension surface.** `actorExtension`, `contractHook`, `provenanceLayer`, `lifecycleHook`, `custodyHook`, `extensions`/`x-`. We do not invent a seventh. | v1 §3.3 "governed output path seam" concern that the original consultant proposal would have sprawled new seams. | Already canonical (kernel §10; CLAUDE.md heuristic 3; archived ADR-0077). Preserved unchanged. |
| **D-16** | **One slice, no phasing.** The five-item spine in §5 is the entire scope of the boundary refactor. | v1 §5 "Phased Execution Plan (MVP vs. Post-MVP)". | The collapsed model is small enough; phasing was an artifact of the over-engineered v1 design. |

**Preserved upstream commitments (re-stated for clarity, not new decisions):**

- ADR-0073 D-1 — WOS is the only emitter of `case.created`.
- ADR-0070 D-1 — Trellis is the commit point.
- ADR-0071 D-1 — four-field `CaseOpenPin` is the cross-layer replay anchor.
- ADR-0074 — per-class encryption (Proposed; ratification is a release gate for D-10).
- ADR-0080 — governed output-commit pipeline; `$defs/OutputBinding` is unchanged.
- Kernel §10 — six extension seams; `custodyHook` is the Trellis attachment.
- Trellis byte authority — ADR 0004 (Rust > CDDL §28 > prose > matrix > Python > archives).

**Open follow-ups (not blocking the ADR):**

1. ~~Rewrite `work-spec/thoughts/adr/0093-case-process-boundary.md` with the §8 text.~~ **Done 2026-05-11:** authored fresh at [`work-spec/thoughts/adr/0093-case-is-its-trellis-ledger.md`](../adr/0093-case-is-its-trellis-ledger.md); predecessor file deleted.
2. Re-derive any `formspec/`, `case-portal/`, `policy-studio/` work items that referenced the v1 dual-aggregate model.
3. Add a one-page `Case` ledger event-schema specification under `work-spec/specs/` (or extend the existing provenance spec) enumerating the §4 event family with payload schemas.
4. Coordinate the Trellis-side PR registering the new `wos.*` event types (D-11).
