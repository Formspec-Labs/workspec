# Case Boundary Refactor: Final Decision Report

**Date:** 2026-05-11
**Decision:** Option B — dual identity (`case_<ulid>` ledger + `process_<ulid>` runtime) from day one.
**Status:** Decision made and applied to ADR-0093 + the v2 synthesis (within their respective scopes). Implementation work to follow per ADR-0093 §5.
**Working-tree truth note:** statements describing v3.1 corrections as "applied" refer to **working-tree state**, not to commits beyond parent `1d2cd72` / work-spec `56f78473`. Where commit state is mentioned in this report, treat as current evidence in the working tree; the git history is the authoritative record.
**Scope:** Captures the full session's exploration arc, the chosen path, implementation implications, and the meta-lessons that justify pinning them now while the context is fresh.
**Authority note:** This report is THE source of truth for all case-management decisions (D-1..D-4 + D-17 closed taxonomy). Its authority is permanent — not contingent on whether ADR-0093, the v2 synthesis, or any downstream artifact has been patched into alignment. Other artifacts derive their position from this report; this report does not derive its position from them. When a downstream artifact disagrees with the decisions recorded here, the downstream artifact is wrong and needs patching, not the report.
**Companion artifact:** `thoughts/plans/2026-05-09-signature-wire-convergence-plan.md` (Integrity Stack Primitive Extraction Plan, recast 2026-05-11) — byte-primitive scope sibling. Both artifacts are **platform-critical at different architectural scopes**: this report owns the workflow-domain layer (dual identity, case-as-ledger, direct-append surface); the plan owns the byte-primitive layer (`integrity-stack/` extraction, profile-spec rebinding, verifier plugin host). They reconcile at four explicit pins — **plan F-11** (identity profile-payload scope), **F-12** (direct-append shape), **F-13** (event-type naming convention), and **plan §17 step 0a** (sequencing interlock). Both are required for v1; neither supersedes the other. See §6.7 for the full alignment block.

---

## 1. Executive summary

The session started with v1 of `case-management-aggregate-synthesis.md` carrying 30+ open `CASE-SYNTH-*` review items and a predecessor ADR-0093 proposing a separate `Case` aggregate above WOS. Through a five-stage validation-and-collapse arc, we arrived at:

- **Architectural truth:** A case *is* its Trellis ledger. No second source of truth, no parallel `Case` aggregate.
- **Identity model:** Dual URN family. `case_<ulid>` is the ledger (durable, outlives any workflow). `process_<ulid>` is the runtime workflow execution (ephemeral; N per case allowed from day one).
- **Write paths:** Two surfaces. Workflow processes emit events via `$defs/OutputBinding` per ADR-0080. A separate direct-ledger-append API handles non-workflow emissions (ad-hoc notes, manual `wos.kernel.case_created`).
- **Read path:** One architectural commitment, two audience-appropriate routes (staff `/cases/{case_id}`, applicant `/applicant/cases/{case_id}`). Implementation per deployment (replay or projection).

The supporting documents are: synthesis v2 (`case-management-aggregate-synthesis.md`), ADR (`0093-case-is-its-trellis-ledger.md`, rewritten 2026-05-11 to encode Option B from scratch), five reviewer-validation files, this report. The implementation plan in §4 describes the work surfaces; the pre-release window absorbs the migration surface (no customer data dependent on the current single-identity shape).

---

## 2. The journey

### 2.1 The starting point — v1 synthesis

When the session began, the working state was:

- `case-management-aggregate-synthesis.md` v1 (2026-05-10): 30+ open `CASE-SYNTH-*` review items, five reviewer files (R1–R5), an 8-agent red-team swarm digest, and an integrated-validation note documenting cross-reads against live sources.
- ADR-0093 predecessor (`0093-case-process-boundary.md`, Proposed 2026-05-10) proposing a separate `Case` aggregate sitting above WOS, materialized as a projection from the Trellis Case Ledger, with its own TypeID prefix (`casefile_`), its own schema, its own materialization engine, and a new `target` discriminator on `$defs/OutputBinding` to route writes between "process-scoped" and "case-scoped" `caseState` partitions.
- Stack-level dangling references to an ADR-0077 ("Canonical kernel extension seams") that turned out to be archived at `formspec/thoughts/archive/adr/` with status Implemented.

The first task was reviewer-style validation against live sources.

### 2.2 First validation pass — 10 FINDINGS

Four parallel agents (`trellis-expert`, `wos-expert`, `spec-expert`, `cross-stack-scout`) cross-checked every load-bearing claim. They surfaced ten findings:

| # | Finding | Severity |
|---|---------|----------|
| 1 | Kernel §9.2.22 mis-cited as home of `$defs/OutputBinding`. The canonical pin is §9.2.18; §9.2.22 is "Request-Response Bindings" *type*, whose `outputBinding` flat-map property is a different concept. | High |
| 2 | "Process-scoped" wording for kernel §5.1 — spec actually says "associated with a workflow instance" (instance-scoped). | Medium |
| 3 | ADR-0071 D-1 case-open pin is **four fields** (`formspec.definitionVersion`, `wos.$wosWorkflowVersion`, `trellis.envelopeVersion`, `trellis.conformanceClass`), not just `definitionUrl`/`definitionVersion`. | High |
| 4 | ADR-0074 status is Proposed/Not started; scope broader than Response wire shape. Synthesis treated as already-normative. | Medium |
| 5 | `formspec-bucketing` flagged as "non-canonical" — false positive. The term IS canonical (planned package per ADR-0074 §5 line 480). | False positive |
| 6 | `wos-core::WorkflowProcess` doesn't emit provenance from the struct itself — emission seam lives in runtime/exporter. | Low |
| 7 | ADR-0073 manual-creation seam is narrower than admitted; kernel §8.2.3 already leaves room. | Low |
| 8 | Multiple dangling `(ADR 0077)` references in CLAUDE.md, ADR-0080, ADR-0076, ADR-0078. | Medium |
| 9 | Tasks-route "inconsistency" framing wrong — `/instances/{id}/tasks` doesn't exist on either side. | Low |
| 10 | Trellis §15 framing — synthesis labeled "projection discipline" but §15 heading is "Snapshot and Watermark Discipline"; "projection discipline" is Operational Companion vocabulary. | Medium |

All ten were folded back into the synthesis as patches with file:line citations. ADR-0093 inherited Findings 1, 2, 3, 6, 7, 10 and was patched in step.

### 2.3 The owner intervention

After applying the findings, owner directed: *"Is there any way we can be more DRY/KISS/BOYSCOUT in this greenfield situation? Assume every ADR/SPEC/PLAN and even stated 'ACCEPTED/RESOLVED' decisions are just a disposable suggestion. What is the ideal end-state in terms of stories and user value and why? Work back from there."*

This was the inflection point. Working from user stories (applicant, caseworker, regulator, developer) backwards, the answer surfaced: every user story is satisfied by **one primitive we already have** — the Trellis case ledger. The "separate `Case` aggregate" was an unnecessary layer between the user-visible matter and the durable record that already encoded it.

Two specific claims from the v1 synthesis literally contradicted each other:

1. *"The product `Case` aggregate is NOT a second parallel source of truth."*
2. *"Mint new domain `Case` under a net-new prefix; define basic `Case` projection schema; build a projection materialization engine; handle dual-state crash recovery."*

If (1) is true, (2) doesn't make sense. The contradiction was the source of every `CASE-SYNTH-*` item that wasn't trivial. Removing the contradiction (case = ledger) dissolved most of the items.

### 2.4 The v2 synthesis — case = ledger

`case-management-aggregate-synthesis.md` was rewritten ground-up. Structure:

1. **The collapse** (one-paragraph architectural position)
2. **User-value derivation** (six stories driving the model)
3. **Architecture** (two-tier diagram: durable ledger + derived view)
4. **Event family** (closed `wos.*` enum: lifecycle / process / domain content / extension)
5. **Minimum viable spine** (five items; no Phase 1/2 split)
6. **Boy-scout withdrawals** (~⅔ of the `CASE-SYNTH-*` register retired)
7. **Tradeoffs**
8. **Recommended ADR**
9. **What changed from v1**
10. **Decisions log** (D-1 through D-16, closed taxonomy)

Net diff: 462 lines changed, +230/-229. Trimmed from a debugging-the-self-inflicted-modeling-problem document into an architectural commitment document with a decisions log.

### 2.5 The fresh ADR

The predecessor ADR-0093 (`0093-case-process-boundary.md`) was deleted. A new ADR was authored from scratch at `0093-case-is-its-trellis-ledger.md` — 289 lines, standard ADR shape (Context / Decision / Consequences / Alternatives Considered / Implementation / Verification / Revision history).

Five alternatives were considered and rejected explicitly: (1) Case as separate aggregate (the predecessor); (2) Case as WOS-CRUD-with-audit-on-the-side; (3) Case as relational table with event-sourced log; (4) Case as a CRDT replicated across regions; (5) status quo (`WorkflowProcess` is the case).

### 2.6 External AI review

An outside AI analysis was provided with mixed-quality feedback. A focused `Explore` subagent verified each factual claim against live sources. Six real precision issues were caught and folded back:

| # | Catch | Status |
|---|-------|--------|
| 1 | `GET /case/{case_id}` was fictional — real routes are staff `/api/v1/instances/{id}` and applicant `/api/v1/applicant/cases/{id}` | Real |
| 2 | Kernel §5.1 has a real "lifecycle vs case-state independence" rule; "no kernel §5 bifurcation" was misleading | Real |
| 3 | `GOAL.md:48` is general posture, not an ADR-0074 pin | Real |
| 4 | Trellis §14.5 *Registry migration discipline* should be cited for "registration precedes emission" | Real |
| 5 | The direct-event-append endpoint at `POST /api/v1/instances/{id}/events` is role-gated to `Adjudicator`; "any authorized actor" was under-specified | Real (but later found incomplete by Codex — see 2.7) |
| 6 | `case_<ulid>` today mints workflow-instance identity via `mint_case_id()`; the transition story to ledger identity was hand-waved | Real |

Pedantic critiques (e.g., "every audit MUST → §4 enum" as a current-spec claim) were set aside as forward-looking commitments, not factual errors.

### 2.7 Codex adversarial review

A Codex adversarial review run challenged the chosen design, not just defect-checked it. Three findings:

| # | Finding | Severity | Status after independent verification |
|---|---------|----------|----------------------------------------|
| 1 | ADR §2.4 misrepresents `POST /api/v1/instances/{id}/events` as a direct ledger-append surface. The handler actually requires an existing instance, enqueues to the runtime queue, drains the workflow, and derives provenance by diffing case-state before/after. It's a workflow-driven path, full stop. | High | **Confirmed.** Read the handler (`instances.rs:380-449`) myself: lines 409 (requires instance), 427 (`enqueue_event`), 428 (`drain_once`), 443 (`diff_to_mutations`). Codex was right; the earlier Explore validator missed it because it didn't read the handler body. |
| 2 | Single `case_` identity leaves no routable durable process identity for N:1 workflows. Runtime is keyed by instance ID (`create_instance`, `load_instance`, `enqueue_event`, `drain_once`); two workflows sharing one ledger ID collide. | High | **Confirmed.** Runtime architecture is genuinely single-ID-keyed; N:1 isn't executable without identity infrastructure. |
| 3 | The original consultant memo `case-management.md` is still live in `thoughts/analysis/` with no supersession banner, and still describes the rejected `Case` aggregate design. Future agents globbing for `case-management*.md` would find it next to the synthesis with no indication of which is current. | Medium | **Confirmed.** Untracked file in `thoughts/analysis/`; reads as if live. |

The Codex review reached "needs-attention / no ship" — appropriate given the load-bearing factual error in Finding 1.

### 2.8 The strategic question

After the Codex review, the strategic question surfaced: should we go bigger? Three options for the runtime-identity gap:

| Option | What it gives up | What it keeps |
|--------|------------------|---------------|
| **A** — 1:1 hard constraint Phase 1, defer N:1 to Phase 2 | "Multiple workflows per case" claim in Phase 1 | Single identity; simple runtime; ships fastest |
| **B** — Dual identity (`case_<ulid>` + `process_<ulid>`) from day one | Speed; single-identity simplicity | N:1 from day one; routing works; no future migration |
| **C** — One workflow per case ever; appeals are new cases linked via `case.related_to` | "Cases outlive workflows" claim | Single identity; simple runtime; no Phase-2 work ever |

### 2.9 The decision — Option B

Owner chose Option B. The implications and rationale are the subject of §3 onward.

---

## 3. The architectural decision: Option B

### 3.1 What "case = ledger" still means (preserved from v2)

The architectural spine survives unchanged:

- **A case is a Trellis ledger.** The ledger is the durable record. All durable case state is encoded as typed events appended to it. The current state of a case is *derived* by event replay or read from a denormalized projection.
- **No separate `Case` aggregate.** The ledger is the only authoritative store for case state. Projections are operational, rebuildable, non-authoritative.
- **No dual-source-of-truth failure modes.** Projection lag is not a bug class.

### 3.2 What dual identity means (the change from v2 §2.2)

Two URN families, both first-class:

| Family | Purpose | Lifetime | Cardinality |
|--------|---------|----------|-------------|
| `case_<ulid>` | **Case ledger identity.** Durable. The thing callers reference as "the matter." Survives all workflows. | Years (matter lifetime) | 1 per case |
| `process_<ulid>` | **Workflow runtime instance identity.** Ephemeral relative to the ledger; durable relative to runtime substrate. What the runtime keys on for event routing, timers, tasks, callbacks. | Days to months (workflow execution lifetime) | N per case (0..many) |

A workflow process is bound to a case ledger at `wos.kernel.process_started` time. The process emits events into the bound ledger. The process_id is recorded on every workflow-emitted event payload (for audit traceability and runtime routing). The case ledger persists independently of any process; processes complete, fail, or are terminated; the ledger keeps existing.

**Identity transitions from the current codebase:**

- `wos-core::typeid::mint_case_id()` is renamed `mint_case_ledger_id()` (or similar — name TBD per implementation). Continues minting `case_<ulid>` IDs, but the IDs now name *ledgers*, not workflow instances.
- New `mint_process_id()` returns `process_<ulid>` for workflow instances.
- The current `WorkflowProcess` struct is renamed to `WorkflowProcess` (or `Process`). Its `process_id` field becomes `process_id`; it gains a `case_ledger_id` foreign-key field bound at start.
- `WosResourceUrn` pattern in `_common.schema.json:20` adds `process` as a family literal alongside `case`, `prov`, `gov`, `ai`, `assurance`, and `x-<vendor>-<name>`.

### 3.3 Why Option B over A or C

**Vs Option A (1:1 hard constraint, deferred N:1):** Pre-release window is the lowest-cost moment to land dual identity. Deferring it means doing the same work later, with the bonus risk that the dual-identity model has design flaws we discover only when a real N:1 customer arrives. Better to design it now while the runtime is still pliable and no customer data depends on it. The 3-4 weeks of upfront work in Phase 1 buys forever-future compatibility.

**Vs Option C (one workflow per case ever; appeals are new cases):** C forecloses too much. Several real product domains require simultaneous N:1 — fraud investigations with concurrent interview + audit + sanction workflows; compliance reviews with parallel verification + remediation threads; benefits programs with overlapping eligibility + appeal workflows. C also imposes a product semantic ("appeals are new cases") that operators in court-system-style domains may resist. Once `case.related_to` links are in customer-facing data, walking back to "actually appeals are the same case" is hard.

**The deciding asymmetry:** Option B's cost is bounded (3-4 weeks of pre-release engineering). Option A's tail cost (Phase-2 refactor under customer use) and Option C's tail cost (semantic migration after data lands) are both unbounded. The pre-release window favors front-loading the structural choice.

### 3.4 What it costs vs what it buys

**Buys:**
- N:1 product capability from day one. Fraud investigations, multi-track compliance, parallel adjudication.
- Implementation-honest "cases outlive workflows" claim.
- Single architectural model that scales from seed deployment (1:1) to complex domains (N:1) without refactor.
- Clean URN family separation: callers always know whether they're referencing the matter or the execution.

**Costs:**
- ~3-4 weeks of focused engineering across schemas, runtime, server, conformance, OpenAPI, fixtures.
- One round of fixture/test rewrites (acceptable pre-release).
- More mental load for developers: two URN families to understand, not one.
- More API routes: `/cases/{case_id}/processes/{process_id}/...` is more verbose than `/instances/{id}/...`.

The mental-load and verbosity costs are durable; the engineering cost is one-time. Pre-release context makes the engineering cost cheap. The capability cost would be permanent under Option A or C.

### 3.5 Reversion clause — documented for engineering hygiene, not anticipated

ADR-0093 §4.2 preserves a bounded reversion path: if external conditions materially change post-decision, the runtime + API remain designed for N:1 but a deployment-configuration flag could declare 1:1 mandatory. The implementation is forward-compatible with Option A reversion at *deployment-policy scope*, not at architecture scope — the dual-identity runtime, schemas, routes, and conformance fixtures all continue to ship per §4 below.

This subsection exists because every irreversible-by-default decision should name its reversion mechanism as standard engineering documentation. It is explicitly **not** a hedging commitment: Option B is the chosen path; identity infrastructure, runtime refactor, HTTP API rewrite, direct-append API, schema updates, and N:1 conformance fixtures all proceed against the dual-identity model on the timeline in §4.8.

External signals that would prompt *re-examination* (not unilateral reversion — the clause is invocable only via a follow-up ADR that demonstrates the original §3.3 defense of B over A no longer holds):

- SBA timeline pressure converts the pre-1.0 architectural-cost flexibility into delivery-cost pressure that dominates the original "front-load the structural decision" rationale.
- 6+ months pass post-prod-MVP with no committed N:1 customer named in the deployment pipeline.
- A committed N:1 customer arrives and the chosen dual-identity *design* (independent ULIDs; runtime keyed on `process_id`; `processId` carried in workflow-emitted event payloads) does not match their operational N:1 shape — in which case the redesign is forward, not backward, and Option A reversion is not the answer.

Absent such signals, the path forward is Option B as committed. This clause is the safety valve every load-bearing decision should have on record; it is not a fork in the roadmap. Implementation teams should not treat its existence as license to operate against the dual-identity contract.

**Scope of rollback if invoked.** If the clause is invoked via a follow-up ADR, the implementation is forward-compatible with Option A reversion at **deployment-policy scope**:
- **Configuration flag:** `wos.deployment.case_process_cardinality = "1:1"` declared in deployment config.
- **Routes preserved:** `/cases/{case_id}/processes/{process_id}/...` routes continue to ship (runtime is dual-identity-keyed regardless). The deployment configuration enforces 1:1 by rejecting `POST /api/v1/cases/{case_id}/processes` when a process already exists.
- **Schemas preserved:** `wos-process.schema.json` and the `process` URN family in `WosResourceUrn` continue to ship. They are non-invocable in 1:1 mode but available in the substrate.
- **Conformance fixtures preserved:** `n-to-one-concurrent` / `cross-process-attribution` fixtures continue to ship and pass; they exercise the runtime in N:1 mode separately from any production-deployed posture.

The rollback is **deployment-policy-level only** — the architecture continues to ship N:1-capable. This preserves the option to flip back to N:1 by removing the config flag, without re-implementing identity infrastructure. Pre-1.0 framing rejects calendar-based triggers; this clause names *scope*, not *dates*.

---

## 4. Implementation implications

### 4.1 Identity infrastructure

**Files affected:**

- `work-spec/crates/wos-core/src/typeid.rs` — add `PROCESS_PREFIX = "process"`, `mint_process_id()`. Rename `CASE_PREFIX` → keep as `case` but reframe purpose (ledger identity, not workflow instance). Add `is_process_id()`, `parse_process_id()` helpers mirroring case-side ones.
- `work-spec/crates/wos-core/src/instance.rs` — rename `WorkflowProcess` → `WorkflowProcess` (or `Process`). Rename `process_id` → `process_id`. Add `case_ledger_id` field (FK). Update all consumers.
- `work-spec/schemas/api/_common.schema.json:20` — update `WosResourceUrn.pattern` to add `process` family literal.
- `work-spec/schemas/wos-process.schema.json` — rename to `wos-process.schema.json`; rename top-level marker `$wosProcess` → `$wosProcess`; update consumers.
- `work-spec/crates/wos-lint/src/document.rs:84-90` — update `DocumentKind` mapping.

### 4.2 Storage refactor

**Files affected:**

- `workspec-server/crates/wos-server-sqlite/migrations/*.sql` — currently has `instances` table. Either:
  - (a) Add a `case_ledgers` table; rename `instances` → `processes`; add FK from `processes.case_ledger_id` → `case_ledgers.id`. Existing `provenance` table partitions by case_ledger_id.
  - (b) Implicit ledger identity (no `case_ledgers` table); `processes` table keyed on `process_id` with `case_ledger_id` column; `provenance` partitions on `case_ledger_id`.
- Same migration pattern for any Postgres adapters (`wos-server-postgres` if present).

Migration is pre-release so destructive DROP+CREATE is acceptable.

### 4.3 Runtime refactor

**Files affected:**

- `work-spec/crates/wos-runtime/src/runtime.rs` — every method keyed on an opaque ID becomes keyed on `process_id`. `create_instance` → `create_process`. `load_instance` → `load_process`. `enqueue_event(process_id, …)`, `drain_once(process_id, …)`. Add `processes_for_case(case_id)` query for routing.
- `work-spec/crates/wos-runtime/src/runtime/instance.rs` — likely renamed to `process.rs`. Storage representation gains `case_ledger_id` field; all event-emission paths include `process_id` in the resulting provenance record.
- `work-spec/crates/wos-runtime/src/store.rs` — storage interface gains `case_ledger_id`-scoped methods alongside process-scoped ones.

### 4.4 HTTP API surface

**Files affected:**

- `workspec-server/crates/wos-server/src/http/instances.rs` → renamed (or sibling) `processes.rs`. Routes:
  - `POST /api/v1/cases/{case_id}/processes` — start a new workflow on a case. Returns `process_id`. Replaces today's workflow-process-create path.
  - `GET /api/v1/cases/{case_id}/processes/{process_id}` — read process state.
  - `POST /api/v1/cases/{case_id}/processes/{process_id}/inputs` — submit a workflow input (per ADR-0093 §2.4/§2.8: two routes, two verbs, two semantics — `/inputs` is the workflow-submission surface, `/events` is direct-append). Replaces today's `POST /instances/{id}/events`.
  - `POST /api/v1/cases/{case_id}/processes/{process_id}/drain` — drain. Replaces today's `POST /instances/{id}/drain`.
  - `POST /api/v1/cases/{case_id}/processes/{process_id}/suspend|resume|terminate` — process lifecycle. Replaces today's instance-keyed equivalents (which are currently absent from `instances.rs` despite being in OpenAPI per ADR-0093 §5.6).
  - `GET /api/v1/cases/{case_id}` — the case view (the read-side route the architectural commitment requires; replaces today's `GET /instances/{id}` semantic-wise for staff).
  - `GET /api/v1/applicant/cases/{case_id}` — applicant view (already exists at `wos-public-api.openapi.json:4277`; no rename needed).
  - **New direct-append surface:** `POST /api/v1/cases/{case_id}/events` — see §4.6.

- Old routes (`/instances/{id}/*`) can be aliased for one release as a transitional kindness to fixtures/tests, or hard-replaced (pre-release allows it). Recommend hard-replace.

### 4.5 Schema updates

**Files affected:**

- `work-spec/schemas/wos-provenance-log.schema.json` — extend with the §4 closed event-type enum from ADR-0093 §2.3. Each event-type record (`$defs/<EventName>Record`) gains an optional `processId` field (present for workflow-emitted events, absent for direct-append events) and a required `caseLedgerId`.
- `work-spec/schemas/wos-workflow.schema.json` — `$defs/OutputBinding` unchanged (D-5 preserved). 
- `work-spec/api/wos-public-api.openapi.json` — full route surface rewrite per §4.4.
- New `work-spec/schemas/api/case-view.schema.json` — the read-side response shape; supersedes (or merges with) today's `instance.schema.json` for the case-view representation.
- `work-spec/schemas/api/provenance.schema.json:630` — the `AssembledExplanation` `GET /api/v1/instances/{id}/explanation` route reference gets rewritten to `GET /api/v1/cases/{case_id}/processes/{process_id}/explanation` because explanation traces a specific workflow process's reasoning. **Landed 2026-05-12** as a case/process bridge route with case/process mismatch returning 404.

### 4.6 Direct ledger append API (the new surface)

The Codex Finding 1 fix. ADR-0093 §2.4 currently misrepresents `/instances/{id}/events` as a ledger-append surface. Replace with a real one:

**Route:** `POST /api/v1/cases/{case_id}/events`

**Authorization model — two branches, dispatched by event type before any check runs:**

- **Pre-ledger creation** (only `wos.kernel.case_created`): authorizes on **tenant scope + role + create-permission**. There is no existing case ledger to relate to; relationship-based ReBAC checks are not applicable and MUST NOT be invoked. The existing `/instances` create handler in [`workspec-server/crates/wos-server/src/http/instances.rs`](../../../workspec-server/crates/wos-server/src/http/instances.rs) (the `create` function — anchor by function name; HEAD has `RequireRole<Supervisor>` at line 227, drifted from the prior `:228` pin) uses `RequireRole<Supervisor>` for exactly this reason; the new surface generalizes to *tenant + role + create-permission* via OpenFGA tuple. Handler MUST reject `wos.kernel.case_created` if a ledger already exists at the URN.
- **Post-ledger append** (every other event type): authorizes on **role + ReBAC relationship to the existing case** + the event-type contract's permission policy. The relationship resolves against the ledger that already exists at `case_id`.

The two branches are mechanically distinct. Collapsing them risks either (a) authorizing creation against a phantom relationship, or (b) denying creation because there is no relationship to check against. Handler control flow MUST dispatch the branch by event type *before* invoking the relationship resolver.

**Other semantics:**
- Validates request body against the event-type contract (lookup by F-13-named `event` literal in the closed enum from §4.5 — see §6.7 F-13 for naming convention).
- Checks Trellis bound-registry presence for the event type (`trellis-verify-wos/src/event_types.rs` constants).
- Enforces idempotency via `idempotency_token`. Cached per `(case_id, token)` for the post-ledger branch; per `(tenant, token)` for the pre-ledger branch.
- For `wos.kernel.case_created` specifically: requires the case ledger to NOT yet exist (`get_case_ledger(case_id) == None`); creates the ledger as the genesis event.
- For all other events: requires the case ledger to exist.
- Emits the event via `custodyHook` directly (no runtime drain; no workflow state machine).
- Returns a provenance receipt with `caseLedgerId`, `eventId`, `eventHash`, `sequence`.

**Use cases:**
- Manual case creation via the pre-ledger branch (an authorized API caller with create-permission, not via `IntakeHandoff`).
- Ad-hoc notes (`wos.kernel.note_added` events without an active workflow process) via the post-ledger branch.
- Out-of-band corrections (`wos.governance.decision_recorded` issued by an adjudicator outside any workflow's transition gating — provided role + relationship authorization permits) via the post-ledger branch.
- Future: any event type whose authorization model doesn't require a workflow state machine in the loop.

**Implementation home:** new `workspec-server/crates/wos-server/src/http/case_events.rs` or extend `cases.rs`. The two-branch authorization MUST be visible in handler control flow (dispatched by event type before the resolver call), not buried inside a single resolver.

### 4.7 Conformance and verification

New conformance fixtures required:

- **N:1 fixture:** two processes started on one case ledger, both emit events, events interleave time-ordered correctly, view rebuild reflects both contributions.
- **Direct-append fixture:** `POST /cases/{case_id}/events` for `wos.kernel.note_added`, event appears in the view without going through any workflow drain.
- **Manual `wos.kernel.case_created` fixture:** `POST /cases/{case_id}/events` with `wos.kernel.case_created` payload creates the ledger and is verifiable as the genesis event.
- **Cross-process audit fixture:** events from process A and process B both carry distinct `processId` values, view correctly attributes each event to its emitting process.
- **Replay-vs-projection fixture:** for the same case_id, reading via replay and reading via projection return byte-identical case-view JSON (modulo audience field projection).
- **Crash recovery fixture:** kill projection materializer mid-run; restart; projection converges.
- **Trellis registry gate:** emission of an unregistered `wos.*` event type fails at lint-time AND runtime.

### 4.8 Estimated effort

| Workstream | Effort |
|------------|--------|
| Identity infrastructure (typeid, schemas, lint) | 3-4 days |
| Storage migration (SQLite + Postgres adapters) | 3-4 days |
| Runtime refactor (RuntimeRecord keying on process_id) | 5-7 days |
| HTTP API surface rewrite | 5-7 days |
| Direct ledger append API | 4-6 days |
| OpenAPI + schema authoring | 2-3 days |
| Conformance fixtures (8 new test scenarios) | 4-5 days |
| Trellis-side cross-repo registry PR | 1-2 days (Trellis-side) |
| End-to-end integration + Restate-adapter parity | 3-5 days |

Total: ~3-4 weeks for one engineer focused, less with parallelism.

---

## 5. Lessons learned

Listed in approximate order of leverage: the highest-leverage lessons are first.

### 5.1 Don't trust file-level validation for behavioral claims

The Explore validator (Claim 11 in the external-AI-review pass) said: *"Route exists: `POST /api/v1/instances/{id}/events` ... this is a direct event-append surface, but restricted to Adjudicator role."* That was wrong. The route exists, but the handler body shows it's a workflow-driven path that requires an existing instance, enqueues to the runtime queue, drains the workflow, and derives provenance by diffing case-state. Codex caught this because it read the handler body. The Explore validator only checked the route registration and role gate.

**The rule:** when a claim is about *what an endpoint does* or *what a function does*, you have to read the implementation, not just the signature/registration. File-level validation is appropriate for "the file exists" or "the property is named X"; it is insufficient for "the call has effect Y."

### 5.2 Multiple AIs agreeing isn't truth

The v1 synthesis carried five reviewer files (R1–R5) plus an 8-agent red-team swarm digest. None of them caught the same handler-body issue Codex caught. The v1 synthesis itself explicitly named this risk: *"Convergent AI reviews on the wrong axis are amplified bias, not validated truth."* Then it proceeded to treat its own convergent reviews as load-bearing.

**The rule:** AI consensus is a hypothesis, not evidence. Each load-bearing claim needs an independent ground-truth read against the actual artifact (code, spec, schema). The number of AIs that agreed on a claim before that read is irrelevant.

### 5.3 Architectural collapses can over-collapse

The v2 collapse to "case = ledger" was correct on the truth-layer question. But it under-specified runtime identity by implying that `case_<ulid>` could simultaneously be the durable ledger ID and the runtime workflow ID without infrastructure to support N:1. The collapse worked at the *conceptual* layer but failed at the *operational* layer. Codex caught the mismatch.

**The rule:** when you collapse N concepts to 1, separately verify that every operational layer (runtime, storage, API, identity) can actually carry the collapsed model. The conceptual collapse and the operational support are independent questions.

### 5.4 Pre-release is a tax credit, not amnesty

"Nothing is released" means migration cost is bounded, not zero. Option A (defer N:1 to Phase 2) would still cost 3-4 weeks of work later — same work, just timed differently. Pre-release means we can pay that cost now without customer disruption, but the cost is real either way.

**The rule:** pre-release windows close. The work you defer doesn't disappear; it just moves into a future where the cost-to-customer of doing it goes from zero to non-zero. Front-load structural decisions while the cost is still cheap.

### 5.5 Citation hygiene rots fast

The session uncovered citation rot at multiple sites: phantom `§23.2.5` in Trellis (didn't exist; AI-authored); wrong `§9.2.22` for `$defs/OutputBinding` (real section but wrong subject); dangling `(ADR 0077)` references at four sites (ADR file moved to archive but citations didn't update); fictional `GET /case/{case_id}` route (AI-extrapolated from the model, not from the codebase); wrong `provenance.schema.json` path (path was right but I had to be told).

**The rule:** every load-bearing citation should be verified at write time. Don't rely on prior-synthesis citations as if they were vouched-for. Specs that cite spec sections, ADRs that cite ADRs, prose that cites file:line — all of it has to be checked. AI-authored citations rot particularly fast because the model often confabulates plausible-sounding pins.

### 5.6 User-value-first beats artifact-driven design

The v1 synthesis had 30+ open `CASE-SYNTH-*` items. Owner's "treat everything as disposable, work from user value" intervention dissolved ~⅔ of them in one move. The items weren't fake — they were real consequences of a flawed premise. Once the premise was corrected (case = ledger, not Case-as-separate-aggregate), the consequences evaporated.

**The rule:** when a register of open issues balloons past comprehension, suspect the premise. Working from user value (what stories does this enable?) frames the architectural choice; working from the artifact (what does the spec say?) frames the bug-fix loop. The first is upstream of the second. Operate upstream when possible.

### 5.7 Iteration fatigue is real

The ADR-0093 went through three major versions in one day: predecessor (Case as separate aggregate, 2026-05-10), v1 of `0093-case-is-its-trellis-ledger` (case=ledger with hand-waved N:1), v2 (Codex fixes + Option B dual identity). Each revision was justified by real issues. But the marginal architectural insight per revision dropped: revision 1 was a structural collapse; revision 2 was citation-precision fixes; revision 3 was scope-honesty fixes.

**The rule:** there's a point where the design is good enough to start implementing, and further iteration costs more than it adds. Knowing when to stop iterating and start building is a skill. Codex's "needs-attention" verdict was appropriately load-bearing; if a future review reaches "needs-attention" on yet-narrower issues, that's signal to ship.

### 5.8 Greenfield ≠ blank slate

Even when owner declared "every ADR/SPEC/PLAN is disposable," upstream commitments shaped the answer:
- ADR-0073 D-1 (WOS owns case-creation; literal `case.created` in the ADR text rebinds to `wos.kernel.case_created` under F-13 in the same rename train — see §6.7) — ownership commitment preserved; literal updated.
- ADR-0074 (per-class encryption; ADR-0074 status: Proposed; per-class encryption not yet a spec-layer contract) — preserved as the target encryption pattern for event payloads.
- ADR-0080 (governed output-commit pipeline; `$defs/OutputBinding`) — preserved without schema changes.
- Kernel §10 six extension seams — preserved.
- Trellis byte authority (ADR 0004) — preserved.
- Trellis "Case ledger" as a named concept (§1.2) — exactly what we built the collapse around.

The collapse to case=ledger worked precisely *because* these constraints already pointed at it. The "disposable" framing applied to v1's *interpretations* of these commitments, not to the commitments themselves.

**The rule:** greenfield licenses you to throw away your own prior framings, not the underlying invariants the architecture depends on. Identify what's truly negotiable (synthesis decisions, ADR choices) vs what's structurally given (cross-stack contracts, byte-level commitments) before reasoning about scope.

### 5.9 The validator gap pattern

Reviewer/Explore agents check that a claim has *something* to anchor on (file exists, route exists, schema field exists). Adversarial reviewers (Codex) check what the anchor actually *does*. Both layers are needed; they answer different questions.

**The rule:** validation pipelines need both layers. A "claim → anchor exists" check is cheap and catches phantom citations. A "claim → anchor behaves as claimed" check is expensive and catches semantic errors. Skipping the second layer is how factual-but-wrong specs end up in ADRs.

### 5.10 Identity decisions are load-bearing for years

TypeID prefixes, URN families, ID minting — these get baked into fixtures, exports, signatures, partner integrations, customer-facing URLs, audit trails. Getting them right in the pre-release window is high-leverage. The dual-identity vs single-identity question (Option B vs A/C) is exactly the kind of decision that's cheap to make now and expensive to revise later.

**The rule:** identity decisions deserve disproportionate attention in early-architecture work. They have the longest tail of any architectural choice. Pre-release is the only time you get to mint identities cheaply.

### 5.11 Don't conflate decisions that look similar but have different reversibility

Three "boundary" decisions sat at different reversibility tiers:
- **Aggregate boundary** (one entity vs N entities): high reversibility — projection logic and code structure.
- **Identity boundary** (one URN family vs N families): low reversibility — embedded in every event payload, export bundle, partner integration.
- **Storage boundary** (canonical vs projection): medium reversibility — schema migration possible but coordinated.

The v1 synthesis treated all three as similar-shape decisions. The v2 collapse correctly handled the aggregate-boundary one (collapse). Option B correctly handles the identity-boundary one (split now while cheap). Storage boundary is unchanged from `wos-server/VISION.md` and inherits its existing canonical/projections split.

**The rule:** before grouping decisions together as "the boundary refactor," classify them by reversibility. Make low-reversibility decisions early; defer high-reversibility ones until you have product feedback.

---

## 6. What needs to happen next

In dependency order:

### 6.1 ADR-0093 patches — APPLIED 2026-05-11

The ADR was rewritten from scratch on 2026-05-11 (predecessor file deleted; new file authored at the same path `work-spec/thoughts/adr/0093-case-is-its-trellis-ledger.md`) to fully encode Option B. The rewrite incorporated, in their final form:

1. **§2.2 Identity:** dual URN families (`case_<ulid>` ledger + `process_<ulid>` workflow runtime), with the rename plan for `mint_case_id` → `mint_case_ledger_id` and addition of `mint_process_id` named explicitly.
2. **§2.4 Workflow writes + §2.5 Direct ledger append writes:** two routes, two verbs, two semantics. `POST /api/v1/cases/{case_id}/processes/{process_id}/inputs` for workflow submission; `POST /api/v1/cases/{case_id}/events` for direct ledger appends. The Codex Finding-1 misrepresentation removed.
3. **§2.5 Authorization split** (post-Codex critique): pre-ledger creation authorizes on tenant + role + create-permission; post-ledger append authorizes on role + ReBAC relationship to the existing case. Two branches dispatched by event type before any resolver runs.
4. **§2.9 Multiple concurrent workflows:** lifted from claim-with-hand-wave to load-bearing commitment with §5 implementation pointers.
5. **§5 Implementation:** ten work-surface subsections describing what changes; time and effort assertions removed from the ADR body per owner directive (decision-basis and sequencing carried here in §4 and in the convergence-plan §17 / step 0a).
6. **§6 Verification:** fifteen claims (V-1 through V-15), including V-15 covering the authorization-branch dispatch.
7. **§2.3 Event family:** F-13 naming convention (`wos.<layer>.<record_kind>` snake_case, layer ∈ {`kernel`, `governance`, `ai`, `assurance`} per `custody-hook-encoding.md §1.5`) applied throughout, in lockstep with the convergence-plan's F-13 commitment.

Additional v3.2 amendments to ADR-0093 are queued in [`thoughts/analysis/2026-05-11-proof-stack-five-reviewer-aggregate.md`](../../../thoughts/analysis/2026-05-11-proof-stack-five-reviewer-aggregate.md) (the source-of-truth patch driver) — covering F-13 amendment framing, identity-collision resolution, `/inputs` route rename, idempotency three-strand disambiguation, the §5.9 cross-repo amendment list, and V-15 negative-fixture replacement for unnamed static analysis. Future modifications track through that patch driver, through this report's §6, or through the convergence-plan §17 sequencing artifacts.

### 6.2 Synthesis update

Update `case-management-aggregate-synthesis.md`:

- §1 "The collapse" — preserve the case=ledger commitment.
- §10 Decisions log — update D-2 (Identity) to reflect dual identity. Add D-17: "Two URN families: `case_<ulid>` (ledger) and `process_<ulid>` (workflow runtime). Phase 1 ships both."
- §11.3 "Recommended next pre-ADR steps" — replace with the §6 sequence from this report.

### 6.3 Banner the original consultant memo

`work-spec/thoughts/analysis/case-management.md` gets a top-of-file supersession banner pointing at ADR-0093 + the v2 synthesis. Or move to `thoughts/archive/`. Either fixes Codex Finding 3.

### 6.4 F-13 registry binding and WOS-owned completion train

The original Trellis-side framing is now stale. The Trellis/substrate slice of the F-13 train has landed: the WOS verifier constants, Python parity tests, Trellis prose, `profile_id` allocation, WOS-facing generator literals, regenerated fixture bytes, digest-named registry blobs, and `wos.assurance.identity_attestation` identity pin are at target. The remaining blocker is WOS-owned completion, not another Trellis constants or fixture pass.

Completed 2026-05-12:

- `custody-hook-encoding.md` §1.5 uses `<record_kind>` and §1.4 reserves the `process` family.
- Trellis Core §23.4 (with custody-hook-encoding §1.5 as the F-13 vocabulary home), `trellis-verify-wos` Rust constants, Trellis Python WOS constants/tests, and WOS-facing fixture generators use F-13 snake_case literals.
- `profile_id` is allocated in Trellis Core §7.4 and mirrored through the protected-header CDDL/Rust/golden-vector path.
- WOS-facing Trellis fixture bytes and digest-named registry blobs were regenerated and verified.
- WOS schema/API/registry/workflow/producer/runtime custody D26 seed: `record-kind-registry.json` now carries optional `eventLiteral` metadata for eleven registry-seeded WOS overlay literals, `wos-provenance-log.schema.json` dispatches the case-creation, intake-decision, `SignatureAffirmation`, `signatureAdmissionFailed`, determination rescission/reinstatement, statutory-clock, and identity-attestation overlays from `event` while compatibility guards keep inner `recordKind` and `event` in agreement, API `FactsTierRecord` requires the same event agreement for those facts records, workflow `FactsTierRecord` rejects wrong explicit event literals without requiring `event` on legacy fragments, WOS producers emit F-13 literals for case creation, signature decisions, governance, clocks, and identity, runtime custody event-type derivation rejects missing or mismatched `event` values for the eleven seeded kinds, and `GET /cases/{case_id}` `latestEvent` exposes the D26 event literal without a redundant `recordKind` projection.
- Case/process list/tasks/explanation/provenance/correspondence/holds/migrate bridges: `GET /api/v1/cases/{case_id}/processes`, `GET /api/v1/cases/{case_id}/processes/{process_id}/tasks`, `GET /api/v1/cases/{case_id}/processes/{process_id}/explanation`, `GET /api/v1/cases/{case_id}/processes/{process_id}/provenance`, `GET /api/v1/cases/{case_id}/processes/{process_id}/correspondence`, `GET /api/v1/cases/{case_id}/processes/{process_id}/holds`, and `POST /api/v1/cases/{case_id}/processes/{process_id}/migrate` are present in server, OpenAPI paths, and API docs. Internal case/process aliases for provenance chain verification, semantic provenance export, available-transition listing, and hold create/release also validate the case/process binding before delegating to the legacy instance helper. Routes with both `{case_id}` and `{process_id}` validate the case/process binding and return 404 on mismatch.
- ADR-0078 iteration literals were reconciled against live registry/runtime state: the emitted record kinds are `forEachIterationStarted`, `forEachIterationCompleted`, and `forEachCompleted`, with custody/export event types `wos.kernel.for_each_iteration_started`, `wos.kernel.for_each_iteration_completed`, and `wos.kernel.for_each_completed`. The review suggestion to add `iteration_failed` / `iteration_skipped` was stale and is not release scope unless a future runtime change emits those records.
- High-traffic kernel/API/ADR prose now labels bare `caseCreated` / `intakeAccepted` names as inner `recordKind` values rather than F-13 event-type literals. Full removal of those inner names remains part of D26, not a prose-only sweep.

Remaining WOS-owned work (pre-release framing — no deprecation periods, no version bumps; legacy aliases get deleted outright):

- Complete alignment of `wos-provenance-log.schema.json`, `wos-workflow.schema.json`, API provenance schemas, and `record-kind-registry.json` (132 kinds; 22 schema-validated; 110 flat): outer `event` literals and schema overlays must agree everywhere, and the inner `recordKind` field is dropped outright per D26 (no parallel-discriminator transition).
- Finish `wos-core` / runtime identity vocabulary (`WorkflowProcess` → `WorkflowProcess`, `process_id` → `process_id`, `case_ledger_id` as the durable case ledger link). Delete legacy aliases (`mint_case_id`) and compatibility fallbacks outright; pre-release framing rejects deprecation periods.
- Sweep prose so bare `caseCreated` / `intakeAccepted` names are only ever inner `recordKind` values, never F-13 event-type literals. Inner field removal proceeds in the same train.
- Delete the remaining workspec-server `/instances/...` routes outright (no aliasing, no `#[deprecated]` attributes, no transition window). Update OpenAPI accordingly. The process list, task, explanation, provenance, correspondence, holds, and migrate bridges have landed on the public surface; internal provenance verify/export/transitions plus hold create/release aliases now exist under the case/process family; remaining legacy routes get deleted in this sweep. The reference server now dispatches post-ledger direct append through a relationship-authorization port, fails closed with a real 403 denial under the default deny-all implementation, and persists event-contract-valid direct appends after a configured allow decision when the generic direct writer can enforce the event contract. Remaining server work is the concrete ReBAC/OpenFGA adapter plus specialized writer paths such as `SignatureAffirmation` and governance determination.
- ~~Decide whether the D26 payload-dispatch change requires a `$wosWorkflow` / registry version bump.~~ **Moot 2026-05-12 (owner-ratified):** pre-release framing — no version bump, no Facts-tier migration record retroactively required. Migration ships under HEAD; the D26 dispatch change is a coordinated greenfield replace, not a versioned migration.

Per Trellis Core §23.2 item 2 + §14 + §14.5, WOS emission of the new types depends on a coherent registry snapshot and fixture set. That dependency is now satisfied on the Trellis side; WOS still owns the schema/runtime/server convergence needed before the same literals are release-grade end to end.

### 6.5 Implementation work (per §4 above)

Approximately 3-4 weeks for one focused engineer. Recommended sequencing:

- Week 1: identity infrastructure (typeid, schemas, lint) → storage migration (SQLite + Postgres adapters).
- Week 2: runtime refactor (`RuntimeRecord` keyed on `process_id`) → conformance fixtures for 1:1 baseline.
- Week 3: HTTP API surface rewrite → direct ledger append API → OpenAPI authoring.
- Week 4: N:1 conformance fixtures → Restate-adapter parity → end-to-end integration verification.

The WOS-owned F-13/D26 schema and registry cleanup should land at the start of Week 1 so emission isn't blocked; the Trellis-side registry/fixture slice is already complete.

### 6.6 Decision-archaeology hygiene

After the implementation ships:

- Move all v1 reviewer-validation files (R1–R5) to `thoughts/archive/analysis/2026-05-10/` with a top-of-folder README pointing at the v2 synthesis + ADR-0093 as the live artifacts.
- Move v1 `case-management.md` to the same archive folder (or banner-mark as superseded if archival is preferred to be done later).
- The v2 synthesis becomes the durable artifact alongside the ADR.

### 6.7 Cross-stack alignment with the integrity-stack primitive extraction plan

The byte-primitive scope companion is `thoughts/plans/2026-05-09-signature-wire-convergence-plan.md` (the Integrity Stack Primitive Extraction Plan, recast 2026-05-11). It runs in parallel with this report's implementation and reconciles at four explicit pins:

- **Plan F-11 — Identity placement.** `caseLedgerId` (required) + `processId` (optional, present when workflow-emitted) live in `DecisionEvent` profile payload (`wos-provenance-log.schema.json` per-event-type records), NOT in the byte-primitive `integrity-event` envelope. Honors this report's §4.5 per-event-type record placement; prevents the primitive layer from re-coupling to workflow-domain semantics.
- **Plan F-12 — Direct-append shape.** Both this report's `POST /api/v1/cases/{case_id}/events` (§4.6) and workflow-emitted events compose `integrity-canonical-json-v1` + `integrity-cose` + `integrity-event` to produce a single `SignedInput`-shaped `DecisionEvent`. No parallel emission paths; admission distinguishes origin via posture rules + presence/absence of `processId`. Pre-extraction caveat: this is the target contract; current code still produces shape-divergent artifacts through two persistence seams (`ProvenanceRecord` for direct append vs `EvaluationResult` plus runtime emission for workflow output), and full `SignedInput` parity gates on the companion plan's steps 5-8.
- **Plan F-13 — Event-type naming convention (CORRECTED).** `wos.<layer>.<record_kind>` snake_case, layer ∈ {`kernel`, `governance`, `ai`, `assurance`} per `custody-hook-encoding.md §1.5`. This preserves the existing closed taxonomy; the rename is snake_case within the layers, NOT a reinvention of the layer set. Historical literals map as follows; the Trellis-side constants/prose/fixture bytes have landed, while WOS schema/runtime/server cleanup remains:
  - `wos.kernel.caseCreated` → `wos.kernel.case_created`
  - `wos.kernel.signatureAffirmation` → `wos.kernel.signature_affirmation`
  - `wos.kernel.intakeAccepted` → `wos.kernel.intake_accepted`
  - `wos.identity.identityAttestation` → `wos.assurance.identity_attestation` (placement by elimination given the closed layer set {kernel, governance, ai, assurance}; ADR-0068 D-3.1 defines the `IdentityAttestation` record shape only and makes no layer-ownership claim — see ADR-0093's "Identity-layer collision resolution" block)
  - `wos.governance.determinationRescinded` → `wos.governance.determination_rescinded`
  - `wos.governance.reinstated` → `wos.governance.reinstated`

  **This convention is a hard prerequisite for §6.4's WOS-owned completion train** — the Trellis-side registration/fixture slice has landed, and WOS schemas/runtime/server prose must now converge on the same literals.
- **Plan §17 step 0a interlock.** Report weeks 1-2 (identity + storage + runtime) interleave with plan steps 1-4 (lift-only primitives) since file sets are disjoint; report weeks 3-4 (API + conformance) interleave with plan steps 5-7; plan steps 11-12 (profile spec rewrites + adapter stratification) **gate on report renames being settled**.

**Blocker-grade (profile/schema/vector correctness, not hygiene):**

- **SHA-256 vector pin (A1).** Landed 2026-05-12 for the signed-payload surface: `AuthoredSignatureSignedPayload.digestAlgorithm` is now structurally `sha-256` in Formspec's response schema, lint schema mirror, generated TS type, engine input type, runtime response assembly guard, and focused schema/engine negative tests. `documentHashAlgorithm` remains broader by design; it is not the A1 signed-payload commitment. Keep distinguishing the schema-visible profile name `formspec-response-signing-v1` from the substrate primitive `integrity-canonical-json-v1` (Q-14 — same bytes, two layered names).
- **`recordKind` migration enumerated against `record-kind-registry.json`.** D26 commits replace-only dispatch through outer `event_type`; the migration is **enumerated against** [`work-spec/schemas/record-kind-registry.json`](../../schemas/record-kind-registry.json) (132 kinds; 22 schema-validated; 110 flat), not via ripgrep heuristics. The 2026-05-12 seed moved eleven registry-seeded overlays toward event dispatch, added API and workflow event-agreement guards, centralized producer literals for case creation, signature decisions, governance, clocks, and identity, and made runtime custody reject missing or mismatched `event` values for those eleven seed kinds, but full D26 remains open until workflow schema replace-only cleanup and runtime fixture regeneration are complete.
- **V-15 verification.** ADR-0093's unnamed "static analysis" placeholder has named behavioral fixtures for the current reference-server posture: pre-ledger creation rejects before relationship authorization, post-ledger append invokes the relationship-authorization port and receives a 403 from the default deny-all resolver without appending a second provenance row, an allowed event-contract-valid post-ledger append persists and replays idempotently, and allowed generic `SignatureAffirmation` / governance determination direct appends are rejected instead of minting malformed specialized records. Full deployment acceptance remains open until a concrete ReBAC/OpenFGA adapter is configured.

**Forward promises (queued, not blocker-grade):**

- This report's §4.5 should adopt F-13 for the closed event-type enum naming when next touched. The synthesis's §4 and D-4 should adopt F-13 in place of the current bare-flat enum, alongside the §6.2 D-2 amendment + D-17 addition. The synthesis's D-3 hedge (*"`$wosProcess` (if it survives at all)"*) should be reconciled with this report's §4.1 (which assumes it survives and renames it to `$wosProcess`).
- **Q-14 — Canonical-bytes substrate-vs-profile-shape split.** Landed 2026-05-12 on the Formspec side: Core §2.1.6, `response.schema.json`, the lint schema mirror, generated TS response types, and the feature matrix now state that `formspec-response-signing-v1` is the wire-visible Formspec profile shape while `integrity-canonical-json-v1` is the consumed substrate primitive. The Formspec layer owns the `authoredSignatures` omission rule and `formspec.response.signed-payload.v1` domain tag; the substrate owns reusable canonical-byte construction.
- **D26 — `event_type` is authoritative dispatch; drop inner `recordKind` field.** The current `WosRecordKind` discriminator inside payload bytes is redundant — current Trellis/WOS parsers still validate local inner `recordKind` literals after event-type dispatch. Drop the inner field outright in one coordinated train; no deprecation window, no parallel-discriminator transition (pre-release framing). Rely on the COSE protected-header `profile_id` (plan O-2) for cross-profile dispatch and `event_type` for intra-profile dispatch. The migration is atomic with the remaining WOS schema/runtime fixture regeneration.

### 6.9 Owner ratifications 2026-05-12

Following the post-swarm code-review pass and a focused owner-decision session, the following decisions are ratified and unblock §6.4's WOS-owned completion train + the convergence-plan extraction (steps 1-5):

- **O-1 (canonical wire format) — RATIFIED.** `integrity-canonical-json-v1` = RFC 8785 JCS + NUL framing + domain-prefix. The rigorous WOS-side rule at `wos-formspec-binding::compute_formspec_signed_payload_digest` is promoted to substrate; `formspec-canonical` either wraps the substrate or is deleted.
- **O-3 (event-type taxonomy amendment) — RATIFIED.** F-13 `wos.<layer>.<record_kind>` snake_case with closed layer set `{kernel, governance, ai, assurance}`. Trellis-side fixture/constant rotation has landed; WOS schema/runtime convergence pending per §6.4.
- **O-4 (verifier CLI) — RATIFIED.** New `integrity-verify-cli` crate inside `integrity-stack/`. `trellis-cli` remains for envelope-only operations.
- **A-1 (PostureResolver) — RATIFIED.** Introduce `PostureResolver` trait per ADR-0086 dependency-inversion direction; cache keys are content digests of fetched bytes, not URIs. Closes the live posture-substitution attack surface on the admission hot path.
- **A-3 (`CoseSign1` unification) — RATIFIED.** Unify on Trellis-cose shape (`alg: i128`, carries `suite_id` and `profile_id`). Formspec adapts upward; lands inside `integrity-cose` extraction (plan step 5).
- **A-4 (EventStore atomicity) — RATIFIED at the EventStore-port level.** The atomicity contract belongs at the EventStore trait per `workspec-server/crates/wos-server/VISION.md` ("Single transaction per write"); not at the SQLite adapter. Trait grows an `append_with_receipt(event, idempotency, recovery_hooks)`-shaped method that adapters MUST commit atomically. SQLite adapter wraps in one transaction now; the `recovery_hooks` parameter preserves the DI seam for richer WAL-recovery later without breaking the contract. Direct addresses the durability gap at `case_events.rs:207, 271`.
- **D26 `$wosWorkflow` version-bump — DROPPED (moot).** Pre-release; no version bump needed.
- **Legacy/deprecation framing — DROPPED stack-wide.** Pre-release rejects deprecation periods. Legacy aliases (`mint_case_id`), legacy routes (`/instances/*`), legacy types (`WorkflowProcess`), and the inner `recordKind` field all get deleted outright in their respective sweeps. No `#[deprecated]` attributes, no sunset windows, no compatibility shims.
- **ADR status promotion (ADR-0074, ADR-0093) — MOOT pre-release.** "Proposed vs Accepted" does not change citeability when there is no released version to stabilize against. Status promotions deferred to a future release window when they will carry user-visible value.

These ratifications collapse PLN-0419 (version-bump) and several legacy-handling bullets; they unblock PLN-0420 / PLN-0421 / PLN-0422 / PLN-0423 / PLN-0424 / PLN-0425 / PLN-0426 from owner-decision to execution.

### 6.8 Sister-artifact banner-mark reminder (M-3 follow-up)

This report is THE source of truth for case-management decisions; its sister analysis artifacts in `work-spec/thoughts/analysis/` need supersession banners so future agents globbing the folder don't read them as live. Hygiene work; not blocking implementation.

- `case-management-aggregate-synthesis.md` — banner-mark as superseded by this report (the synthesis's D-1..D-16 closed taxonomy is now mirrored and extended in this report's authoritative decisions log; the synthesis remains useful as derivation history but should not be cited as the source of truth for new work).
- The five validation files (R1–R5: `case-management-validation-claude-opus-4.7.md`, `case-management-validation-glm-5.1.md`, `case-management-validation-gpt-5-codex.md`, `case-management-validation-claude-opus-4-7-1m.md`, `case-management-validation-gemini.md`) — banner-mark or archive to `thoughts/archive/analysis/2026-05-10/` per §6.6. R1-R5 served their validation role; their factual claims have been folded back into the synthesis and this report.
- `case-management.md` (the original consultant memo) — already noted in §6.3 as needing banner or archive; reiterated here as part of the same hygiene sweep.

These banners are reminders, not blockers. Implementation work proceeds against this report's §4 surfaces regardless of whether the banners have landed.

---

## 7. Artifact inventory

The session's working set, as of 2026-05-11:

### Live (normative or pre-implementation)

| File | Status | Role |
|------|--------|------|
| `work-spec/thoughts/adr/0093-case-is-its-trellis-ledger.md` | Proposed | The ADR. Decision basis for this report; §6.1 patches applied 2026-05-11. v3.2 amendments pending per the patch driver below. |
| `work-spec/thoughts/analysis/case-management-aggregate-synthesis.md` | v2 (current) | Synthesis. Needs §6.2 update. |
| `work-spec/thoughts/analysis/case-boundary-decision-report.md` | This file | Source-of-truth report for D-1..D-4 + D-17 case-management decisions. Durable. |
| `thoughts/analysis/2026-05-11-proof-stack-five-reviewer-aggregate.md` | Patch driver (not source of truth) | Validated patch queue across this report, ADR-0093, the convergence plan, and the cross-cutting findings. Treat as execution artifact; not a parallel decision authority. |
| `thoughts/analysis/2026-05-11-proof-stack-five-reviewer-aggregate-archive.md` | Archive | Prior reviewer-aggregate narrative; preserved for history. Not execution input. |

### Historical (validation corpus)

| File | Role |
|------|------|
| `case-management-validation-claude-opus-4.7.md` | R1 |
| `case-management-validation-glm-5.1.md` | R2 |
| `case-management-validation-gpt-5-codex.md` | R3 (renamed 2026-05-11) |
| `case-management-validation-claude-opus-4-7-1m.md` | R4 |
| `case-management-validation-gemini.md` | R5 (renamed 2026-05-11) |
| `case-management.md` | Original consultant memo. Superseded. Needs §6.3 banner or archive. |

### Cross-references updated this session

| File | Change |
|------|--------|
| `work-spec/CLAUDE.md` | Dangling `(ADR 0077)` reference in Schema-structure paragraph replaced with archive-aware citation. |
| `thoughts/adr/0080-governed-output-commit-pipeline.md` | Frontmatter line 6 + body line 92: ADR-0077 link/reference replaced with archive-aware citation + canonical kernel §10.3 pin. |
| `thoughts/adr/0078-foreach-topology.md` | Frontmatter line 6: ADR-0077 link replaced. |
| `thoughts/adr/0076-product-tier-consolidation.md` | Frontmatter line 6: ADR-0077 link replaced. |

### Deleted

| File | Replacement |
|------|-------------|
| `work-spec/thoughts/adr/0093-case-process-boundary.md` (predecessor ADR) | `work-spec/thoughts/adr/0093-case-is-its-trellis-ledger.md` |

### Files that will change during implementation (per §4)

- `work-spec/crates/wos-core/src/typeid.rs`
- `work-spec/crates/wos-core/src/instance.rs`
- `work-spec/crates/wos-lint/src/document.rs`
- `work-spec/crates/wos-runtime/src/runtime.rs`
- `work-spec/crates/wos-runtime/src/runtime/instance.rs` (likely renamed)
- `work-spec/crates/wos-runtime/src/store.rs`
- `work-spec/crates/wos-runtime/src/binding.rs`
- `work-spec/schemas/wos-provenance-log.schema.json`
- `work-spec/schemas/wos-process.schema.json` (renamed)
- `work-spec/schemas/wos-workflow.schema.json`
- `work-spec/schemas/api/_common.schema.json`
- `work-spec/schemas/api/instance.schema.json` (consolidated or renamed)
- `work-spec/schemas/api/provenance.schema.json`
- `work-spec/schemas/api/case-view.schema.json` (new)
- `work-spec/api/wos-public-api.openapi.json`
- `workspec-server/crates/wos-server/src/http/instances.rs` (renamed)
- `workspec-server/crates/wos-server/src/http/cases.rs` (new or extended)
- `workspec-server/crates/wos-server/src/http/case_events.rs` (new)
- `workspec-server/crates/wos-server-sqlite/migrations/*.sql`
- `workspec-server/crates/wos-server-postgres/migrations/*.sql` (if present)
- `trellis/crates/trellis-verify-wos/src/event_types.rs` (cross-repo PR)

---

## 8. Closing observation

Two genuine architectural insights came out of this session:

1. **Case is its Trellis ledger.** We had this primitive all along; v1 manufactured a second aggregate that fought it. The collapse to one durable record dissolves ~⅔ of v1's CASE-SYNTH register without giving up any user-facing capability.

2. **The collapsed-truth-layer needs uncollapsed operational layers.** Conceptual unity (one durable record) coexists with operational pluralism (one URN family for the durable record, a different URN family for ephemeral runtime instances). Trying to collapse both layers simultaneously was the v2-only-pass error Codex caught.

The lesson behind both: **simplify where simplification serves user stories; don't simplify where simplification erases capability.** Case=ledger serves every story (no second source of truth, integrity-first audit). Identity-collapse erased N:1 capability that fraud-investigation and multi-track-compliance domains require. The right answer is asymmetric: collapse one layer, separate another.

Pre-release made this learnable cheaply. Three rewrites in a day cost time but no customer disruption. Post-release, the same iteration would have been painful. The platform decision register's "no backwards compatibility" stance turned the cost of being-wrong into manageable rework. Future architectural work should consciously exploit this window before it closes.

---

*End of report.*
