# ADR 0093 — A Case Is Its Trellis Ledger; Cases and Workflow Processes Are Distinct Identities

**Status:** Proposed
**Date:** 2026-05-11
**Scope:** WOS — case identity, workflow-process identity, durable case state, governed output emission, provenance event family, direct ledger-append surface, multiple concurrent workflows per case.
**Decision basis:** [`../analysis/case-boundary-decision-report.md`](../analysis/case-boundary-decision-report.md). This ADR encodes the **Option B** path (dual identity from day one) selected by owner directive after explicit comparison with Option A (defer N:1) and Option C (one workflow per case ever). Acknowledged in §4 as a values-driven structural front-load, not a strictly data-driven optimum.

**Related:**
[ADR 0070 (failure and compensation)](../../../thoughts/adr/0070-stack-failure-and-compensation.md) D-1 (Trellis is the commit point);
[ADR 0071 (cross-layer migration and versioning)](../../../thoughts/adr/0071-stack-cross-layer-migration-and-versioning.md) D-1 (four-field `CaseOpenPin`);
[ADR 0073 (case initiation and intake handoff)](../../../thoughts/adr/0073-stack-case-initiation-and-intake-handoff.md) D-1 (WOS owns `wos.kernel.case_created`);
[ADR 0074 (formspec-native field-level transparency)](../../../thoughts/adr/0074-formspec-native-field-level-transparency.md) (Proposed; per-class encryption);
[ADR 0080 (governed output-commit pipeline)](../../../thoughts/adr/0080-governed-output-commit-pipeline.md) (Proposed; `$defs/OutputBinding`);
[ADR 0082 (public API contract and schema discipline)](../../../thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md);
[`work-spec/specs/kernel/spec.md`](../../specs/kernel/spec.md) §5 (case state), §8.2.3 (governance-owned creation path), §9.2.18 (OutputBinding overview), §10 (six extension seams; archived [ADR 0077](../../../formspec/thoughts/archive/adr/0077-canonical-kernel-extension-seams.md), Implemented);
[`work-spec/schemas/wos-provenance-log.schema.json`](../../schemas/wos-provenance-log.schema.json);
[`work-spec/schemas/wos-workflow.schema.json`](../../schemas/wos-workflow.schema.json) `$defs/OutputBinding`;
[`work-spec/schemas/api/_common.schema.json`](../../schemas/api/_common.schema.json) `WosResourceUrn`;
[`trellis/specs/trellis-core.md`](../../../trellis/specs/trellis-core.md) §1.2 (case ledger), §10.1, §10.4, §15, §23.2 item 2, §23.4 (`wos.*` namespace), §14.5 (registry migration discipline);
[`workspec-server/crates/wos-server/VISION.md`](../../../workspec-server/crates/wos-server/VISION.md) `canonical`/`projections` schema split;
companion: [`../../../thoughts/plans/2026-05-09-signature-wire-convergence-plan.md`](../../../thoughts/plans/2026-05-09-signature-wire-convergence-plan.md) (byte-primitive scope; F-11 / F-12 / F-13 / §17 step 0a alignment pins);
synthesis: [`../analysis/case-management-aggregate-synthesis.md`](../analysis/case-management-aggregate-synthesis.md) v2.

---

## 1. Context

### 1.1 The conflation we are removing

Until this ADR, the stack treated the durable domain **case** as identical to the WOS runtime artifact `CaseInstance`. A `CaseInstance` is a *running workflow execution*: it carries lifecycle state, cursor position, scheduled timers, retry counters, completion semantics. Calling it "the case" was a category error.

Real matters outlive any single workflow. Intake produces a case; an appeal three months later attaches a *second* workflow to the *same* case; a compliance review later attaches a *third*. A fraud investigation may run interview + audit + sanction workflows concurrently on one case. The status of "the case" is not the lifecycle state of "the workflow currently running on it." Conflating the two pushed product-level concerns (notes, participants, related matters, decisions, history) into the workflow runtime, where they sat awkwardly and bloated `caseState` into a junk drawer.

### 1.2 What the architecture already encoded

Two prior commitments pointed at the resolution:

- **Trellis owns the durable record.** `trellis-core.md` §1.2 names the **case ledger** as a hash-chained sequence composing sealed response-ledger heads with WOS governance events into one matter. §10.1 + §10.4 + §23.2 item 5 establish strict-linear authoritative order; §23.4 reserves the `wos.*` event namespace; §15 + §2.1 class 4 + Operational Companion §14.2 establish that projections derive from canonical truth, never carry their own authority.
- **The reference server already separates canonical and projection.** [`workspec-server/crates/wos-server/VISION.md`](../../../workspec-server/crates/wos-server/VISION.md) lines 98–101 define two Postgres schemas: `canonical` (Trellis events, immutable, signed, encrypted) and `projections` (derived metadata, mutable, rebuildable, plaintext-content-free).

The case primitive we need already exists. We just have to admit it.

### 1.3 What we got wrong on the way here

This ADR replaces two prior 2026-05 drafts under the same number, both preserved in git history:

- A predecessor `0093-case-process-boundary.md` (Proposed 2026-05-10) introduced a separate `Case` aggregate sitting above WOS, materialized as a projection, with its own TypeID prefix (`casefile_`), its own schema, its own materialization engine, and a `target` discriminator on `$defs/OutputBinding`. That design addressed the original conflation but created new structural cost — a second source of identity, a dual-state crash-recovery problem, a kernel §5 bifurcation, and a follow-up `0073-bis` for manual creation.
- A first revision (also 2026-05-11) collapsed to single-identity case=ledger, but **overpromised N:1** without delivering the runtime infrastructure to make it routable. Codex adversarial review caught two issues: (i) `POST /api/v1/instances/{id}/events` was misrepresented as a direct-ledger-append surface when the handler is actually a workflow-event-enqueue path that requires an existing instance and runs the workflow state machine; (ii) the runtime is single-ID-keyed (`create_instance`, `enqueue_event`, `drain_once`) so two workflows sharing one ledger ID collide.

This ADR keeps the case=ledger architectural commitment and **adds** the dual-identity model that makes N:1 routable from day one.

### 1.4 Decision posture

The owner-directive context: pre-release window (per `work-spec/CLAUDE.md` and the platform decision register, *no backwards compatibility / nothing is released*). The migration cost of front-loading the structural identity decision is bounded — no customer data depends on the current single-identity shape.

Three options were considered explicitly in [`../analysis/case-boundary-decision-report.md`](../analysis/case-boundary-decision-report.md) §2.8:

- **Option A** — 1:1 hard constraint, defer N:1.
- **Option B** — dual identity (`case_<ulid>` + `process_<ulid>`) from day one. **Chosen.**
- **Option C** — one workflow per case ever; appeals are new cases linked via `case.related_to`.

Honest acknowledgment from the decision report §3.3: Option B is a *values-driven* choice (front-load the structural identity decision while the migration surface is empty) rather than a data-driven optimum. SBA prod-MVP (the seed deployment) is structurally 1:1. The defense of Option B is identity-decisions-have-long-tails plus the pre-release-window-is-narrow argument. The Negative-Consequences section below names this trade explicitly.

---

## 2. Decision

### 2.1 A case is a Trellis ledger

A case is the Trellis ledger scoped to one matter. All durable case state is encoded as typed events appended to this ledger. The current state of a case is **derived** by replaying the ledger or by reading a denormalized projection — an operational choice, not an architectural distinction.

There is no separate `Case` aggregate. There is no projection-vs-canonical distinction at the type layer. There is no second `caseState` aggregate boundary. Kernel §5.1's existing rule — *lifecycle state and case state are independent* — is preserved; what this ADR declines to add is a second `caseState` axis (instance-scoped vs case-scoped variants) on top of it.

### 2.2 Identity: two URN families

Two first-class identifiers, with distinct purposes and lifetimes:

| URN family | Names | Lifetime | Cardinality | Runtime role |
|------------|-------|----------|-------------|--------------|
| `case_<ulid>` | The case ledger (durable, the matter). | Matter lifetime. | Exactly one per case. | Storage partition key; ledger scope. |
| `process_<ulid>` | A workflow runtime execution bound to a case ledger. | Workflow execution lifetime. | 0..N per case. | Runtime key — `create_process`, `load_process`, `enqueue_event`, `drain_once`, timers, tasks, callbacks. |

A workflow process is bound to a case ledger at `wos.kernel.process_started` time. Multiple processes per case ledger are admitted **from day one** — there is no 1:1 ontology rule; the seed deployment may *operate* in 1:1 mode but the model admits N processes per case.

**Renames in `wos-core`:**

- `mint_case_id()` in [`work-spec/crates/wos-core/src/typeid.rs`](../../crates/wos-core/src/typeid.rs) is renamed `mint_case_ledger_id()` and continues minting `case_<ulid>` — but the IDs now name *ledgers*, not workflow instances.
- A new `mint_process_id()` returns `process_<ulid>` for workflow instances.
- The `CaseInstance` struct in [`work-spec/crates/wos-core/src/instance.rs`](../../crates/wos-core/src/instance.rs) is renamed `WorkflowProcess`; its `instance_id` field becomes `process_id`; it gains a `case_ledger_id` foreign-key field bound at process start.
- The `$wosCaseInstance` schema marker becomes `$wosProcess` ([`work-spec/schemas/wos-case-instance.schema.json`](../../schemas/wos-case-instance.schema.json) renamed to `wos-process.schema.json`).
- [`work-spec/schemas/api/_common.schema.json`](../../schemas/api/_common.schema.json:20) `WosResourceUrn.pattern` adds `process` as a family literal alongside the existing `case`, `prov`, `gov`, `ai`, `assurance`, and `x-<vendor>-<name>`.

The pre-release context absorbs the fixture-and-test rewrites that follow; no customer data exists to migrate.

### 2.3 The event family

A closed enum under the `wos.*` namespace, registered in the Trellis bound registry per `trellis-core.md` **§23.2 item 2** + **§14** (namespace rules in **§23.4**; registration-precedes-emission discipline in **§14.5** *Registry migration discipline*).

**Event-type naming convention (F-13).** All `wos.*` event types follow `custody-hook-encoding.md` §1.5 normative form: `wos.<layer>.<record_kind>` snake_case, where layer ∈ {`kernel`, `governance`, `ai`, `assurance`}. The convention is set in [plan §11 F-13] ([`../../../thoughts/plans/2026-05-09-signature-wire-convergence-plan.md`](../../../thoughts/plans/2026-05-09-signature-wire-convergence-plan.md)) and applies in lockstep across the Trellis registry constants (`trellis/crates/trellis-verify-wos/src/event_types.rs`) and the WOS schema (`wos-provenance-log.schema.json`). The earlier 5-axis attempt (`{lifecycle, process, content, signature, extension}`) was rejected as a normative wire-format change without ADR — layer = WOS spec layer that owns the semantics (architectural anchor), not a concept/functional axis. This supersedes both the bare-flat pattern (`case.created`) the schema currently uses and the Trellis registry's existing dotted-camel pattern (`wos.kernel.caseCreated`). The schema home for WOS-side records is [`wos-provenance-log.schema.json`](../../schemas/wos-provenance-log.schema.json); the existing `$defs/CaseCreatedRecord` is the prototype shape (its `event const` literal rebinds to `wos.kernel.case_created` under F-13).

**Authoritative dispatch (D26).** Two discriminators, two scopes:
- **`profile_id`** — COSE protected-header integer label per plan O-2. Cross-profile dispatch. Selects which profile plugin (Trellis profile / Workflow profile / Formspec profile) handles the event.
- **`event_type`** — CBOR payload map field per plan F-13 (`wos.<layer>.<record_kind>` form). Intra-profile dispatch. Selects which validator within the profile handles the event.

Both are authoritative within their scopes. The redundant inner `recordKind` field (currently in `trellis-verify-wos/src/records.rs:310-313` as a tautological re-check of `event_type`) is deprecated alongside D8.

**Extension events.** Vendor extension events take the form `wos.<layer>.x_<vendor>_<name>` — placed within an existing closed layer (kernel/governance/ai/assurance), with the `x_` prefix marking vendor-extended. The `custody-hook-encoding.md §1.5` closed-layer rule is preserved. The earlier `wos.extension.*` form is rejected. Kernel §10.6 `x-`-prefixed keys remain the syntactic seam for extension presence.

Every event payload carries `caseLedgerId` (REQUIRED). Workflow-emitted events additionally carry `processId` (REQUIRED for `wos.kernel.process_*` runtime events, `wos.governance.decision_recorded` when emitted by a workflow, and any other workflow-attributed event; absent for direct-append events such as ad-hoc `wos.kernel.note_added` emitted via §2.5).

**Kernel** (case identity, lifecycle, runtime process, attachment surface)

| Event type | Emitter | Notes |
|------------|---------|-------|
| `wos.kernel.case_created` | WOS only (ADR-0073 D-1). Either workflow-initiated (via `IntakeHandoff` or `wos.kernel.process_started`) or direct via §2.5. | Opens a new ledger. Payload: tenant, class, optional `IntakeHandoff` reference, optional bound first-process ID. Kernel: case identity §5. When `IntakeHandoff` is present in `workflowInitiated` mode, the handoff MUST include a non-null `caseRef` per `intake-handoff.schema.json` allOf condition. In `publicIntake` mode, `caseRef` is absent. |
| `wos.kernel.case_closed` | WOS | Terminal-but-optional. Closure is a state, not a requirement. Kernel: case lifecycle. |
| `wos.kernel.case_status_changed` | WOS | Application-defined status transitions, distinct from process lifecycle. Kernel: case state. |
| `wos.kernel.case_related_to` | WOS | Relationship edge using kernel §5.5 taxonomy (`parent \| child \| sibling \| related \| supersedes`); extensible via `wos.<layer>.x_<vendor>_<name>`. |
| `wos.kernel.process_started` | Workflow runtime | A workflow process binds to this ledger. Payload: `process_id`, workflow definition URL+version, initial state, four-field `CaseOpenPin`. Kernel: runtime instance. Carries `processId`. |
| `wos.kernel.process_transitioned` | Workflow runtime | Lifecycle state change within a process. Carries `processId`. |
| `wos.kernel.process_completed` / `process_failed` / `process_suspended` / `process_resumed` / `process_terminated` | Workflow runtime | Terminal-or-pause states of an individual process. The case ledger continues regardless. Carry `processId`. |
| `wos.kernel.note_added` | Authorized role (via §2.5 direct append) or Workflow runtime (via §2.4) | Free-form annotation. Kernel: attachment surface. |
| `wos.kernel.artifact_attached` | Authorized role or Workflow runtime | Wraps a Formspec response or external document. Carries the four-field `CaseOpenPin` (§2.7). Kernel: attachment surface. |
| `wos.kernel.signature_affirmation` | WOS Signature Profile processor | Surfaces existing WOS `SignatureAffirmation` semantics into the ledger. Signature Profile is a *profile*, not a *layer*; the emission is a kernel-layer record per `custody-hook-encoding.md §1.5`. No second meaning of "signed." Preserves `work-spec/CLAUDE.md` Signature-shortcut rule. |

**Governance** (adjudicatory outputs)

| Event type | Emitter | Notes |
|------------|---------|-------|
| `wos.governance.decision_recorded` | Workflow runtime or authorized role | Adjudicatory output. Carries `verificationLevel` + signature affirmation reference. Governance: adjudicatory outputs (Kernel §13.9 amendment taxonomy). |

**Vendor extension**

| Event type | Emitter | Notes |
|------------|---------|-------|
| `wos.<layer>.x_<vendor>_<name>` | Vendor (within existing layer) | Place vendor extension events within an existing closed layer; the `x_` prefix marks vendor-extended. Examples: `wos.kernel.x_acme_correlation_added`, `wos.governance.x_thirdparty_witness_attested`. The closed-layer rule from `custody-hook-encoding.md §1.5` is preserved. The earlier `wos.extension.*` form is rejected. |

Every WOS MUST that produces an audit event maps to exactly one of the above. The list is closed; growth requires an ADR that adds a row.

### 2.4 Workflow event writes

Workflow processes emit events via the existing **`$defs/OutputBinding`**, canonically pinned at **kernel §9.2.18 Overview** (`work-spec/specs/kernel/spec.md:1127–1129`: *"Each binding is an `OutputBinding` entry … through the validated output-commit pipeline (ADR 0080)"*). The shape remains `{ on, contractRef, projection, writeScope, mutationSource, verificationLevel }`. **No new property.** **No `target` discriminator.** The event type — declared by the binding's contract — is the discriminator.

The HTTP surface for workflow event submission is:

```
POST /api/v1/cases/{case_id}/processes/{process_id}/events
```

The handler routes the event into the specified process's runtime queue (`enqueue_event(process_id, …)`), drains that process, and the binding emissions append to the case ledger via `custodyHook` (kernel §10.5, four-field append shape). Each binding emission produces exactly one ledger event whose payload carries `caseLedgerId = case_id` and `processId = process_id`, with `event` literal per the F-13 convention.

The current `/instances/{id}/events` route is **replaced** by the route above. Pre-release allows hard replacement; an alias may exist transitionally for fixtures but does not survive to first release.

### 2.5 Direct ledger append writes

A second write surface, distinct from workflow event submission and not present in HEAD today:

```
POST /api/v1/cases/{case_id}/events
```

**Authorization model.** Two distinct authorization rules, applied per the event type being emitted:

- **Pre-ledger creation** (only for `wos.kernel.case_created`): authorizes on **tenant scope + role + create-permission**. There is no existing case ledger to relate to; relationship-based ReBAC checks are not applicable. The current `/instances` create handler in [`workspec-server/crates/wos-server/src/http/instances.rs`](../../../workspec-server/crates/wos-server/src/http/instances.rs:228) uses `RequireRole<Supervisor>` for exactly this reason; the new surface generalizes to *tenant + role + create-permission per OpenFGA tuple*. The handler MUST reject `wos.kernel.case_created` if a ledger at `case_id` already exists.
- **Post-ledger append** (every other event type via this surface): authorizes on **role + ReBAC relationship to the existing case** + the event-type contract's permission policy. Relationship checks resolve against the ledger that already exists at `case_id`.

The two rules are mechanically distinct: pre-creation cannot consult a relationship to a not-yet-existing entity. Implementations MUST split the authorization branch by event type *before* the relationship check is attempted; collapsing them risks either authorizing creation against a phantom relationship or denying creation that has no relationship to check against.

**Other semantics:**

- **Validates** request body against the event-type contract (lookup by `event` literal in the F-13-named closed enum from §2.3).
- **Checks** Trellis bound-registry presence for the event type (`trellis-verify-wos/src/event_types.rs` constants).
- **Enforces idempotency** via `idempotency_token` (cached per `(case_id, token)` for post-ledger; per `(tenant, token)` for pre-ledger).
- For `wos.kernel.case_created` specifically: requires the case ledger to NOT yet exist; creates the ledger as the genesis event. WOS authority (ADR-0073 D-1) is preserved — the API caller is acting as a WOS-boundary actor with create-permission authorization.
- For all other events: requires the case ledger to exist.
- **Emits** the event directly via `custodyHook` (no runtime drain; no workflow state machine).
- **Returns** a provenance receipt with `caseLedgerId`, `eventId`, `eventHash`, `sequence`.

Use cases satisfied by this surface:

- Manual case creation (an authorized API caller with create-permission, no `IntakeHandoff`).
- Ad-hoc notes (`wos.kernel.note_added` outside any active workflow process).
- Out-of-band corrections (`wos.governance.decision_recorded` issued by an authorized adjudicator outside a workflow transition gate).
- Future: any event type whose authorization model doesn't require a workflow state machine in the loop.

This surface does NOT replace `$defs/OutputBinding`. Workflow-driven emission stays at §2.4. The two surfaces co-exist; they share the event-family taxonomy, the four-field `CaseOpenPin` requirement, the Trellis registry binding, and the per-class encryption envelope. They differ in *who is allowed to emit what* and in *whether a runtime is involved*.

### 2.6 Reads

One read surface per audience; both implement the same derivation contract:

| Audience | Route | Source |
|----------|-------|--------|
| Staff / adjudicators | `GET /api/v1/cases/{case_id}` | New route under this ADR. Replaces today's `GET /api/v1/instances/{id}` (see §5.4) staff-side semantics. |
| Applicants | `GET /api/v1/applicant/cases/{case_id}` | Already in OpenAPI at `work-spec/api/wos-public-api.openapi.json:4277`; preserved. |

Both return a JSON document conforming to a new [`case-view.schema.json`](../../schemas/api/case-view.schema.json). The implementations may use:

1. **On-demand replay** — walk the ledger up to the latest committed event, fold into the view. Reference implementation; correct by construction.
2. **Denormalized projection** — read from a projection table in the `projections` schema, maintained by a background materializer that subscribes to ledger commits. Plaintext-content-free. Used for hot reads.

Both MUST agree against a conformance fixture: same `case_id`, identical view (modulo audience-appropriate field projection). The projection has **no authority**; on a crash that leaves it stale, the recovery procedure is: drop the projection, replay the ledger.

Per-process state is read at a process-scoped sub-route:

```
GET /api/v1/cases/{case_id}/processes/{process_id}
```

This returns workflow-execution state (lifecycle, current configuration, pending tasks). It is distinct from the case-view route, which returns case-level state aggregated across all processes plus direct-append events.

### 2.7 Pinning

Every event payload that wraps a Formspec response — notably `wos.kernel.artifact_attached` and `wos.governance.decision_recorded` — MUST carry the full **four-field `CaseOpenPin`** from ADR-0071 D-1:

| Axis | Field |
|------|-------|
| Formspec definition version | `formspec.definitionVersion` |
| WOS workflow document version | `wos.$wosWorkflowVersion` |
| Trellis envelope version | `trellis.envelopeVersion` |
| Trellis conformance class | `trellis.conformanceClass` |

Plus the Formspec-axis detail (`definitionUrl`+`definitionVersion` for Response per Formspec Core §6.4; `definitionRef.url`+`definitionRef.version` for Intake Handoff per Formspec Core §2.1.6.1). All four `CaseOpenPin` axes are **co-required**; validation MUST reject payloads missing any axis.

`wos.kernel.process_started` events also carry the four-field `CaseOpenPin` so that workflow-bound replay can resolve the right WOS semantic version for the bound process.

### 2.8 Process management

Workflow processes are managed via dedicated routes scoped to a case:

| Route | Purpose |
|-------|---------|
| `POST /api/v1/cases/{case_id}/processes` | Start a new workflow on a case. Body: workflow definition URL+version, initial bindings. Returns `process_id`. Emits `wos.kernel.process_started`. |
| `GET /api/v1/cases/{case_id}/processes` | List processes bound to a case (current and historical). |
| `GET /api/v1/cases/{case_id}/processes/{process_id}` | Read process state (lifecycle, configuration, pending tasks). |
| `POST /api/v1/cases/{case_id}/processes/{process_id}/events` | Submit a workflow event (§2.4). |
| `POST /api/v1/cases/{case_id}/processes/{process_id}/drain` | Drain pending events. |
| `POST /api/v1/cases/{case_id}/processes/{process_id}/suspend` | Suspend the process. Emits `wos.kernel.process_suspended`. |
| `POST /api/v1/cases/{case_id}/processes/{process_id}/resume` | Resume a suspended process. Emits `wos.kernel.process_resumed`. |
| `POST /api/v1/cases/{case_id}/processes/{process_id}/terminate` | Terminate the process. Emits `wos.kernel.process_terminated`. |
| `GET /api/v1/cases/{case_id}/processes/{process_id}/explanation` | Assembled provenance explanation (replaces today's `/instances/{id}/explain` per ADR 0082 schema authority; see §5.4). |

Suspend / resume / terminate are currently absent from `workspec-server/crates/wos-server/src/http/instances.rs` (route absence noted in synthesis D-13); this ADR delivers them under the new case-scoped, process-scoped surface.

**Route invariant (case_id ⇄ process_id cross-check).** Any route bearing both `{case_id}` and `{process_id}` path params MUST verify that the loaded process's `case_ledger_id` equals the `{case_id}` path parameter. Mismatch returns 404 (case-process binding violation). Without this check, events emitted under a wrong-case URL prefix can leak into adjacent case ledgers via process_id-keyed runtime routing. This applies to `POST .../events`, `POST .../drain`, `POST .../suspend`/`/resume`/`/terminate`, `GET .../explanation`, and any future route with both path params.

### 2.9 Multiple concurrent workflows on one ledger

Load-bearing. A single case ledger may carry any number of concurrent or sequential workflow processes emitting into it. Each event payload carries `processId` (when workflow-emitted) so attribution is unambiguous.

Conflicts between two processes attempting to write the same logical field resolve at the **read-side**, with three permissible strategies declared per deployment or per field:

1. **Last-writer-wins** (default; the ledger's strict-linear order makes this deterministic).
2. **Merge function** declared in the projection logic (union of sets, sum of counters, etc.).
3. **FEL-guarded reject** — a binding's `contractRef` may declare a precondition that fails the write if a conflict is detected; the rejection becomes its own ledger event.

The seed deployment (SBA prod-MVP) is structurally 1:1 and exercises last-writer-wins by inertia. Conformance fixtures (§5.7) cover the N:1 case explicitly so that runtime + storage + read-side N:1 behavior is verified pre-release.

### 2.10 Per-class encryption

The normative authority is [ADR-0074](../../../thoughts/adr/0074-formspec-native-field-level-transparency.md) (**Proposed, Not started**). Event payloads are bucketed per access class and wrapped with per-class DEKs per that ADR. The Case API never flattens classified bodies into the top-level case-view document; sensitive fields are surfaced as opaque references the client decrypts with the bag of wrapped keys it possesses, or — in deployments operating per the **audited-server-side-decryption profile** — via brokered decryption with a logged purpose.

The deployment-profile context is in [`workspec-server/crates/wos-server/VISION.md`](../../../workspec-server/crates/wos-server/VISION.md) lines 78–82 (SBA-tier "Platform may decrypt for explicit, audited purposes; plaintext never persists at rest; every decryption is a KMS-logged event") and 98–105 (canonical/projections; clients-decrypt / servers-broker). [`GOAL.md`](../../../GOAL.md) line 48 states the prod-MVP posture in general terms — *"audited server-side decryption only; no Federal/Sovereign confidential-compute claim"* — without naming ADR-0074; treat it as deployment-target context, not the normative source.

ADR-0074 ratification is a release gate for the bucketed mode. Until ratified, deployments operate per the audited-server-side-decryption profile above.

---

## 3. Consequences

### 3.1 Positive

- **One conceptual model.** A case IS its ledger; nothing more to learn at the truth layer.
- **N:1 from day one.** Fraud investigations, multi-track compliance, parallel adjudication — all expressible without runtime gymnastics.
- **Two URN families, clear separation.** Callers always know whether they're referencing the matter (`case_<ulid>`) or the execution (`process_<ulid>`).
- **Real direct-append surface.** Manual case creation and ad-hoc notes have a first-class API (`POST /cases/{case_id}/events`) that doesn't depend on a workflow state machine. Authorization split — pre-ledger create-permission vs post-ledger relationship — is mechanically explicit (§2.5).
- **No dual-source-of-truth failure modes.** Projection lag is not a bug class; the projection rebuilds.
- **Cases outlive workflows.** The ledger persists past any process emitting to it.
- **API surface aligns with the case=ledger model.** Routes resource-named `/cases/{case_id}/...` rather than `/instances/{id}/...`.
- **Event-type naming is consistent across registries.** F-13's `wos.<layer>.<record_kind>` form (per `custody-hook-encoding.md §1.5`, layer ∈ {`kernel`, `governance`, `ai`, `assurance`}) supersedes both the bare-flat WOS schema literals and the dotted-camel Trellis registry entries; one convention across both sides of the cross-stack registration boundary.

### 3.2 Negative / Complexity

- **Material implementation scope** across schemas, runtime, server, conformance, OpenAPI, fixtures. Detailed in §5.
- **Two URN families = more developer mental load.** Documentation and onboarding need to be explicit about which ID is which.
- **Verbose route prefixes.** `/cases/{case_id}/processes/{process_id}/...` is longer than `/instances/{id}/...`.
- **Some operators and domain modelers may find "Case = ledger" jarring.** Mitigation: the API surface (`GET /cases/{id}`) returns a familiar-looking JSON resource; the truth-layer / presentation-layer split is internal.
- **Honest acknowledgment from the decision report:** Option B is a values-driven front-load of a structural decision. The dual-identity *design* (specifically: `case_<ulid>` and `process_<ulid>` as independent ULIDs; runtime keyed on `process_id`; `processId` present in workflow-emitted event payloads) is unproven against a real N:1 workload. SBA prod-MVP is 1:1 and won't validate the N:1 path. There is non-zero risk that fraud-investigation or other N:1 customers, when they arrive, will want refinements that we haven't anticipated. Option A (defer N:1) would have reduced this ADR's scope considerably and accepted that tail risk later; Option B accepts the additional scope to make the identity-decision irreversible-by-default while the migration surface is empty.
- **The Trellis registry binding is critical-path.** Phase-zero per Trellis §23.2 item 2 + §14 + §14.5. Cross-repo PR required before WOS emission of the new event types. F-13's naming convention must settle in lockstep across Trellis registry constants AND the WOS schema (§2.3).
- **Workflow developers must learn the closed event-type taxonomy** (§2.3) rather than writing freely into a domain struct. This is a feature: closed taxonomy keeps the model from drifting; but it is a discipline change.

---

## 4. Alternatives Considered

### 4.1 Single identity (`case_<ulid>` for both ledger and runtime) — the rejected first revision

The 2026-05-11 first revision of this ADR (preserved in git history) collapsed both ledger identity and runtime instance identity into a single `case_<ulid>` URN, with vague "address by `process_started` event ID or substrate handle" wording for N:1.

**Rejected because** Codex adversarial review demonstrated that: (i) the runtime is single-ID-keyed (`create_instance`, `enqueue_event`, `drain_once`, `load_instance` all key on a single ID), so two workflows sharing a `case_<ulid>` would collide on the same `RuntimeRecord`; (ii) the prior draft also misrepresented `POST /api/v1/instances/{id}/events` as a direct-append surface when the handler actually requires an existing instance, enqueues into the runtime queue, drains the workflow, and derives provenance by diffing case-state. Single-identity overpromised N:1 without delivering the routing infrastructure to support it.

This ADR's dual-identity model (§2.2) closes the gap by giving the runtime a separate keyable identifier.

### 4.2 Option A — 1:1 hard constraint, defer N:1

A 1:1 case-to-workflow constraint (one process per ledger, ever, in the seed deployment), with N:1 deferred to a future ADR.

**Rejected because** the owner directive prioritized front-loading the structural identity decision while the migration surface is empty. The decision report ([`../analysis/case-boundary-decision-report.md`](../analysis/case-boundary-decision-report.md) §3.3) names this honestly: Option A is the smaller-scope play and remains defensible; Option B is the architectural-ambition play that accepts more scope to make the identity decision irreversible-by-default. The defense for B is identity-decisions-have-long-tails plus pre-release-window-is-narrow.

If future signals favor reverting to A (e.g., the dual-identity design proves shaky under N:1 conformance), the constraint to revert is small: declare 1:1 mandatory in deployment configuration, leave the runtime+API as designed for N:1 but unused. The implementation does not foreclose the option.

### 4.3 Option C — One workflow per case ever; appeals are new cases

Every case carries exactly one workflow. Appeals, renewals, and compliance reviews are *new cases* linked to the original via `wos.kernel.case_related_to` edges.

**Rejected because** Option C forecloses real product domains. Fraud investigations require concurrent interview + audit + sanction workflows on one case. Multi-track compliance reviews require parallel verification + remediation. Once `case_related_to` semantics are in customer-facing data, walking back from "appeals are new cases" to "appeals share the case" is hard and disruptive. Option C is the simplest model but the least future-resilient.

### 4.4 Case as a separate aggregate (predecessor ADR-0093)

A `Case` domain aggregate above WOS, materialized from the Trellis Case Ledger, with its own TypeID prefix, its own schema, its own materialization engine, and a `target` discriminator on `$defs/OutputBinding` to route writes between "process-scoped" and "case-scoped" surfaces.

**Rejected** in the v2 synthesis. Its own framing stated that `Case` "is NOT a second parallel source of truth," yet the implementation pattern treated it as one — distinct identity, distinct schema, distinct crash-recovery story. The contradiction generated the 30+ CASE-SYNTH register that the v2 collapse dissolved.

### 4.5 Case as a WOS-centered domain entity (CRUD)

Model `Case` as a primary WOS domain entity (similar to `CaseInstance`, carrying `serde_json::Value` for state) whose mutations produce Trellis events via `custodyHook`. The CRUD database is the operational source of truth; Trellis is an audit projection.

**Rejected** because it preserves the dual-source-of-truth problem in a different shape. If WOS maintains an authoritative DB representation of the case AND Trellis maintains the event chain, the zero-trust commitment (Trellis is canonical, projections derived) inverts. ADR-0070 D-1 and ADR-0074 both presuppose Trellis canonicity.

### 4.6 Case as a relational table with event-sourced audit log on the side

A `cases` SQL table is the operational source of truth; Trellis is an append-only audit log written alongside.

**Rejected** as the classic event-sourcing-lite anti-pattern. The log and the table diverge under partial failure; integrity claims become contingent on agreement that won't hold.

### 4.7 Case as a CRDT replicated across regions

`Case` as a CRDT converging across regions; per-region Trellis logs.

**Rejected** because the SBA / Federal / Sovereign deployment matrix does not call for multi-region active-active. Append-log semantics handle single-region multi-writer concurrency adequately; CRDTs would add substantial implementation surface without a matching user story.

### 4.8 Status quo (`CaseInstance` *is* the case)

Do nothing. Keep treating the running workflow as the durable case.

**Rejected** because it re-states the original conflation that motivated this entire refactor.

---

## 5. Implementation

The work surfaces below describe **what changes**; logical ordering is captured in §5.9 (Trellis registry must precede WOS emission of new event types). Time and effort are not asserted in this ADR; the convergence plan §17 and the decision report §4 carry the sequencing artifacts.

### 5.1 Identity infrastructure

**Files:**

- [`work-spec/crates/wos-core/src/typeid.rs`](../../crates/wos-core/src/typeid.rs) — add `PROCESS_PREFIX = "process"`, `mint_process_id()`, `is_process_id()`, `parse_process_id()`. Rename `mint_case_id()` → `mint_case_ledger_id()`. Keep `CASE_PREFIX = "case"` but reframe purpose (ledger ID, not instance ID).
- [`work-spec/crates/wos-core/src/instance.rs`](../../crates/wos-core/src/instance.rs) — rename `CaseInstance` → `WorkflowProcess`. `instance_id` → `process_id`. Add `case_ledger_id` FK field. Update all consumers in `wos-core`, `wos-runtime`, downstream crates.
- [`work-spec/schemas/wos-case-instance.schema.json`](../../schemas/wos-case-instance.schema.json) → renamed `wos-process.schema.json`. Top-level marker `$wosCaseInstance` → `$wosProcess`. Update lint mapping ([`work-spec/crates/wos-lint/src/document.rs`](../../crates/wos-lint/src/document.rs) lines 84-90).
- [`work-spec/schemas/api/_common.schema.json`](../../schemas/api/_common.schema.json) line 20 — `WosResourceUrn.pattern` adds `process` family literal.

### 5.2 Storage migration

**Files:**

- [`workspec-server/crates/wos-server-sqlite/migrations/`](../../../workspec-server/crates/wos-server-sqlite/migrations/) — add `case_ledgers` table (if explicit); rename `instances` → `processes` with `case_ledger_id` FK; partition `provenance` table by `case_ledger_id`. Pre-release allows destructive DROP+CREATE.
- Same migration pattern for any Postgres adapters (e.g., `workspec-server/crates/wos-server-postgres/`).

### 5.3 Runtime refactor

**Files:**

- [`work-spec/crates/wos-runtime/src/runtime.rs`](../../crates/wos-runtime/src/runtime.rs) — every method keyed by ID becomes keyed by `process_id`. `create_instance` → `create_process`. `load_instance` → `load_process`. `enqueue_event(process_id, …)`. `drain_once(process_id, …)`. Add `processes_for_case(case_ledger_id)` query.
- [`work-spec/crates/wos-runtime/src/runtime/instance.rs`](../../crates/wos-runtime/src/runtime/instance.rs) — rename to `process.rs`. Storage representation gains `case_ledger_id` field; every event-emission path includes `processId` in the resulting provenance record.
- [`work-spec/crates/wos-runtime/src/store.rs`](../../crates/wos-runtime/src/store.rs) — storage interface gains `case_ledger_id`-scoped queries alongside process-scoped ones.
- [`work-spec/crates/wos-runtime/src/binding.rs`](../../crates/wos-runtime/src/binding.rs) — `OutputBinding` emission carries `processId` of the emitting process.

### 5.4 HTTP API surface

**Files:**

- [`workspec-server/crates/wos-server/src/http/instances.rs`](../../../workspec-server/crates/wos-server/src/http/instances.rs) → renamed (or split into `cases.rs` + `processes.rs`). Routes per §2.4, §2.6, §2.8.
- New `workspec-server/crates/wos-server/src/http/cases.rs` for case-scoped routes (`GET /cases/{case_id}`, list/create processes).
- New `workspec-server/crates/wos-server/src/http/case_events.rs` for the direct-append surface (§5.5).
- [`work-spec/api/wos-public-api.openapi.json`](../../api/wos-public-api.openapi.json) — full route rewrite. Lines 516 (`GET /instances/{id}`) and 1163 (`/instances/{id}/explanation`) updated to case-scoped, process-scoped equivalents. Applicant route at line 4277 preserved (no rename needed; already `/applicant/cases/{id}`).

### 5.5 Direct ledger append API

**Files:**

- New `workspec-server/crates/wos-server/src/http/case_events.rs`. Implements `POST /api/v1/cases/{case_id}/events` per §2.5.
- **Authorization split** per §2.5: pre-ledger branch for `wos.kernel.case_created` (tenant + role + create-permission); post-ledger branch for all other events (role + ReBAC relationship to existing case). Handler MUST dispatch the branch by event type before invoking the relationship resolver.
- Idempotency layer (cached per `(case_id, idempotency_token)` for post-ledger; per `(tenant, idempotency_token)` for pre-ledger).
- Role-authorization integration (OpenFGA/ReBAC tuple checks per event type's policy).
- Event-type-contract validation (lookup by F-13-named `event` literal in closed enum).
- Trellis registry presence check (constant lookup in `trellis-verify-wos`).
- Custody emission path (direct call to `custodyHook`, no runtime drain).
- Provenance receipt response shape.

For `wos.kernel.case_created` specifically: requires the case ledger to NOT yet exist; creates the ledger as genesis. For all other events: requires the case ledger to exist.

### 5.6 Schema updates

**Files:**

- [`work-spec/schemas/wos-provenance-log.schema.json`](../../schemas/wos-provenance-log.schema.json) — extend with the §2.3 F-13-named closed event-type enum. Each event-type record gains optional `processId` and required `caseLedgerId`. Existing `$defs/CaseCreatedRecord.event.const` rebinds from `"case.created"` to `"wos.kernel.case_created"`; sibling record definitions added for every other §2.3 event. Inner `recordKind` field deprecated per D26 (§2.3); fixture corpus regenerates atomically.
- [`work-spec/schemas/api/provenance.schema.json`](../../schemas/api/provenance.schema.json) — line 630 `AssembledExplanation` reference updated from `/instances/{id}/explanation` to `/cases/{case_id}/processes/{process_id}/explanation`.
- New [`work-spec/schemas/api/case-view.schema.json`](../../schemas/api/case-view.schema.json) — the read-side response shape from §2.6.

### 5.7 Conformance fixtures

New fixtures in `work-spec/crates/wos-conformance`:

| Fixture | Verifies |
|---------|----------|
| `one-to-one-baseline` | Single process on a case; events emit; view rebuilds correctly. |
| `n-to-one-concurrent` | Two processes started on one case ledger; both emit events; view attributes each correctly; events interleave time-ordered. |
| `direct-append-note` | `POST /cases/{id}/events` for `wos.kernel.note_added` with no active workflow; event appears in view. |
| `direct-append-case-create` | `POST /cases/{id}/events` with `wos.kernel.case_created` creates a ledger via the pre-ledger authorization branch; subsequent reads return view. |
| `direct-append-auth-split` | `wos.kernel.case_created` is rejected when no create-permission tuple exists; all other events are rejected when no relationship-to-case tuple exists; cross-contamination (relationship check applied to creation, or create-permission applied to post-ledger) fails the fixture. |
| `cross-process-attribution` | Events from process A and process B carry distinct `processId`; view correctly attributes. |
| `replay-vs-projection-parity` | Same `case_id`, both implementations return byte-identical view (modulo audience field projection). |
| `crash-recovery` | Kill projection materializer mid-run; restart; projection converges. |
| `caseopenpin-enforcement` | `wos.kernel.artifact_attached` events missing any of the four `CaseOpenPin` axes fail validation. |
| `registry-gate` | Emission of an unregistered `wos.*` event type fails at lint AND runtime. F-13-named entries (per `custody-hook-encoding.md §1.5` 4-layer form) are admitted; bare-flat, dotted-camel, or 5-axis forms are rejected. |
| `urn-family-coexistence` | `case_<ulid>` and `process_<ulid>` resolve correctly via `WosResourceUrn`; no parse-time collisions. |
| `target-no-property` | `$defs/OutputBinding` in HEAD does NOT have a `target` property (D-5 preservation). |
| `n-to-one-routing` | Event submitted to process A's route routes to process A's runtime queue, not B's. |

### 5.8 Restate-adapter parity

Three-way agreement (spec ↔ in-memory runtime ↔ Restate production adapter) per `work-spec/CLAUDE.md`. Restate's process state needs the same `process_id` keying as the in-memory runtime; durable timers and tasks need `process_id`-scoped routing.

### 5.9 Trellis-side registry binding (cross-repo prerequisite)

Coordinated PR to the [`trellis/`](../../../trellis/) repo. **This is logically prior to WOS emission of any new event types** — Trellis §14.5 *Registry migration discipline*: events using a new interpretation MUST NOT be admitted before the registry update lands.

- Add `WOS_<EVENT>_EVENT_TYPE` constants to [`trellis/crates/trellis-verify-wos/src/event_types.rs`](../../../trellis/crates/trellis-verify-wos/src/event_types.rs) for every F-13-named event type from §2.3. Existing dotted-camel constants (`wos.kernel.caseCreated` etc.) rename to F-13 form in lockstep with the WOS schema rename.
- Add accompanying conformance fixtures on the Trellis side.
- Per `trellis-core.md` §23.2 item 2 + §14 + §23.4 + §14.5.

Until that PR lands, WOS-side emission of the new event types MUST remain disabled. CI gate: a check that every `wos.*` event type emitted by `wos-export` resolves to a registered constant; an unregistered emission fails the build.

### 5.10 Out of scope for this ADR

- Migration of in-memory `CaseInstance` representations from prior development snapshots — none are in production; pre-release admits a clean cut.
- Multi-region active-active replication (rejected as Alternative 4.7).
- Cross-tenant case sharing (out of scope of the case=ledger model; would require separate tenancy ADR).

---

## 6. Verification

This ADR is verified-as-implemented when every claim below passes:

| # | Claim | Verification |
|---|-------|--------------|
| V-1 | Every §2.3 event type is registered in the Trellis bound registry under F-13 naming. | `wos-conformance` registry-gate fixture; cross-repo CI. |
| V-2 | A single workflow can run end-to-end on a case and emit `wos.kernel.process_started` → `wos.kernel.process_transitioned`* → `wos.kernel.process_completed`. | `one-to-one-baseline` fixture, in-memory + Restate runtimes. |
| V-3 | Two concurrent workflow processes can run on one case ledger and both contribute attributed events. | `n-to-one-concurrent` fixture. |
| V-4 | A `wos.kernel.note_added` event emitted via `POST /api/v1/cases/{case_id}/events` (no workflow context) appears in `GET /api/v1/cases/{case_id}` (staff) and `GET /api/v1/applicant/cases/{case_id}` (applicant, as access controls permit). | `direct-append-note` fixture + E2E test in `workspec-server`. |
| V-5 | A `wos.kernel.case_created` event emitted via `POST /api/v1/cases/{case_id}/events` creates the case ledger as the genesis event when no ledger exists, *and uses the pre-ledger authorization branch (tenant + role + create-permission), not the relationship-based branch*. | `direct-append-case-create` + `direct-append-auth-split` fixtures. |
| V-6 | `wos.kernel.artifact_attached` events with any of the four `CaseOpenPin` axes omitted fail validation. | `caseopenpin-enforcement` fixture; property-based fuzz. |
| V-7 | The read-side view returns byte-identical JSON for the same `case_id` whether served from replay or projection. | `replay-vs-projection-parity` fixture. |
| V-8 | Crash recovery: kill projection materializer mid-run; restart; projection converges to committed-ledger state. | `crash-recovery` fixture (chaos test in `workspec-server`). |
| V-9 | A workflow that emits an unregistered or non-F-13-named `wos.*` event type is rejected at lint time AND runtime. | `registry-gate` fixture. |
| V-10 | No `target` property exists on `$defs/OutputBinding` in HEAD. | `target-no-property` fixture (schema diff). |
| V-11 | `WosResourceUrn` family literals `case` and `process` both parse correctly and don't collide. | `urn-family-coexistence` fixture. |
| V-12 | An event submitted to process A's route routes to process A's runtime queue, not process B's. | `n-to-one-routing` fixture. |
| V-13 | Suspend / resume / terminate routes are present and functional under the new case-scoped, process-scoped paths. | Route existence + integration test. |
| V-14 | The `/explanation` endpoint is consistent under the new `/cases/{case_id}/processes/{process_id}/explanation` route across OpenAPI + server + provenance schema. | Schema + server parity test. |
| V-15 | The direct-append handler dispatches the authorization branch by event type *before* invoking the relationship resolver; pre-ledger creation cannot reach the relationship resolver, and post-ledger events cannot reach the create-permission resolver. | `direct-append-auth-split` fixture; static analysis of handler control flow. |
| V-16 | Route invariant: an event submitted to `/cases/case_A/processes/process_belonging_to_case_B/events` is rejected with 404; the receiver of process B's case ledger does not see the event. | `process-case-mismatch-rejection` fixture. |

Three-way agreement (spec ↔ in-memory runtime ↔ Restate production adapter) is the verification posture per [`work-spec/CLAUDE.md`](../../CLAUDE.md).

---

## 7. Revision history

- **2026-05-11 (this revision):** F-13 event-type naming convention corrected from the earlier 5-axis attempt (`{lifecycle, process, content, signature, extension}`) to the existing normative 4-layer form per `custody-hook-encoding.md §1.5`: `wos.<layer>.<record_kind>` snake_case with layer ∈ {`kernel`, `governance`, `ai`, `assurance`}. Triggered by trio-expert validation (wos-expert read `custody-hook-encoding.md §1.5` directly and surfaced the conflict). All event names in §2.3 rewritten per the layer mapping. D26 added: `event_type` is the authoritative dispatch discriminator; the inner `recordKind` field is redundant and deprecated alongside D8.
- **2026-05-11 (this draft):** Option B — dual identity (`case_<ulid>` ledger + `process_<ulid>` workflow runtime). F-13 event-type naming convention applied across §2.3 and dependent sections. Authorization split for direct-append surface formalized in §2.5 (pre-ledger create-permission vs post-ledger relationship). Time and effort assertions removed from the ADR body; sequencing carried by §5.9 (Trellis-registry precedes WOS emission) and by the convergence plan §17 + decision report §4. Replaces the earlier same-day single-identity draft of this same ADR number, which conflated ledger and runtime identity, claimed N:1 without delivering routing infrastructure, and misrepresented `POST /api/v1/instances/{id}/events` as a direct-append surface. Decision context: [`../analysis/case-boundary-decision-report.md`](../analysis/case-boundary-decision-report.md).
- **2026-05-11 (earlier same-day, superseded):** Single-identity draft. `case_<ulid>` as both ledger and runtime ID; vague N:1 transition story; conflated workflow-event-enqueue surface with direct-ledger-append; bare-flat event-type names (`case.created` etc.).
- **2026-05-10 (predecessor, superseded):** `0093-case-process-boundary.md` proposed a separate `Case` aggregate above WOS, with its own TypeID prefix (`casefile_`), its own schema, its own materialization engine, and a `target` discriminator on `$defs/OutputBinding`. Superseded by the case=ledger collapse in synthesis v2 (preserved in git history).

---

## 8. Supporting documents

- **Decision report** (full session arc, lessons learned, values-vs-data acknowledgment): [`../analysis/case-boundary-decision-report.md`](../analysis/case-boundary-decision-report.md).
- **Synthesis** (architectural derivation from user value backward): [`../analysis/case-management-aggregate-synthesis.md`](../analysis/case-management-aggregate-synthesis.md) v2.
- **Byte-primitive companion** (F-11 / F-12 / F-13 / §17 step 0a alignment pins): [`../../../thoughts/plans/2026-05-09-signature-wire-convergence-plan.md`](../../../thoughts/plans/2026-05-09-signature-wire-convergence-plan.md).
- **Validation corpus** (reviewer files R1–R5 from the v1 synthesis pass): `../analysis/case-management-validation-*.md` (5 files; preserved as historical record, not normative going forward).
- **Original consultant memo** (the input that started the whole exercise; supersession banner pending): `../analysis/case-management.md`.
