---
title: WOS Runtime S15 — Formspec Coprocessor Protocol (concrete proposal)
date: 2026-04-11
status: superseded
merged_into: thoughts/specs/2026-04-11-formspec-wos-phase11-integration-master.md
superseded_by: thoughts/specs/2026-04-11-formspec-wos-phase11-integration-master.md
sources: .claude-plugin/agents/spec-expert.md + wos-spec-author.md personas; thoughts/archive/specs/2026-04-10-formspec-integration-gaps.md; work-spec/specs/companions/runtime.md; schemas/response.schema.json
---

> **Superseded as the single handoff doc by** [`2026-04-11-formspec-wos-phase11-integration-master.md`](../../specs/2026-04-11-formspec-wos-phase11-integration-master.md) **(2026-04-11).** This file remains as the detailed §15 paste source; the master merges plan + integration gaps + this draft.

# Concrete proposal: Runtime Companion §15 — Formspec Coprocessor Protocol

This document is a **drop-in outline** for new normative material in `work-spec/specs/companions/runtime.md` **after §14** (renumber subsequent sections if your publication process requires strict global section IDs; internally references stay “Runtime Companion §15” / “S15”).

It merges:

- **Spec-expert (Formspec) constraints:** Response document shape (`response.schema.json`), VP-01 pinning on `(definitionUrl, definitionVersion)`, and the fact that validation semantics live in Formspec Core — WOS must **cite and delegate**, not restate.
- **WOS-spec-author constraints:** Additive-only Formspec rule; host/engine boundary (ADR-0057); extension of existing Runtime **S12** host interfaces; kernel schema extensions for `ContractReference` / `createTask` where the kernel already owns those types.

---

## Executive decisions (resolve open design tensions)

| Topic | Proposal | Rationale |
|-------|----------|-----------|
| **ContractValidator input** | New host operation **`validateFormspecTaskResponse`** (see §15.5). It accepts the **full Formspec Response object** (must pass **full** Formspec **`response.schema.json`** validation — see §15.5) **plus** the task’s pinned **`(definitionUrl, definitionVersion)`** (same tuple as Response VP-01 — Core S1.4.3 / S6.4, cite only) for equality check. Internally the Formspec-conformant processor runs Definition validation over **`data`** per Core §5.5 as for a submission eligible for **`status: "completed"`** (Core `status` + VE-05 — cite only). | Validating **`data` alone** cannot detect wrong `definitionVersion` / wrong `definitionUrl` masquerading as success (**integration gaps review #2**). |
| **Validation failure → task state** | After a **`submitTaskResponse`** that fails Formspec validation, the processor MUST transition the task **`claimed` → `failed`** (Governance task lifecycle already allows this: `claimed` → `failed`). It MUST NOT leave the task **`claimed`** as if submission succeeded. Rejection policy (Governance S8) still governs explanations and side effects. | Removes **claimed limbo** (**Phase 11 bias #3**); aligns with `workflow-governance.md` table. |
| **Prefill mapping** | **`prefillMappingRef`** SHOULD reference a Formspec Mapping Document with **`direction`: `"both"`** when the same artifact maps case → form and form → case (**bias #6**). If only forward exists, host MAY use a second mapping URI — normative minimum: prefill MUST be reproducible from case state + declared mapping or explicit `prefillData`. | Open question in integration gaps §1.5.1 — pick **both** as default recommendation. |
| **In-progress saves** | **Phase 1:** `submitTaskResponse` accepts only **`status: "completed"`** for the validation gate that drives task completion. **`in-progress`** Responses MAY be handled entirely **inside the host** (draft storage) without processor calls, OR a future S15.1 extension adds **`persistTaskDraft`** — **not** in minimal S15. | Keeps case state free of unvalidated writes (**integration gaps §1.5.2**). |
| **`actorId` vs `assignedActor`** | **`submitTaskResponse.actorId` MUST equal the task’s `assignedActor`** unless **`AccessControl`** / delegation rules (Runtime S12.5, S8.4) explicitly allow the caller (**bias #8**). | Accountability on rights-impacting flows. |

---

## §15.1 Purpose and scope (normative intro)

**Purpose.** Define the **Formspec Coprocessor Protocol**: how a WOS processor constructs presentation context for a Formspec-backed task, how the host returns a Formspec Response, how the processor validates and maps that Response into case state, and which provenance records MUST be emitted.

**Scope.** Applies when a Kernel `createTask` action references a **`contractRef`** whose **`binding`** is **`formspec`** (kernel schema term — exact property path to match your `ContractReference` definition).

**Formspec integration rules (informative checklist).** This section MUST NOT alter Formspec processing semantics; MUST delegate Definition evaluation to a Formspec-conformant processor (Core S1.4); MUST cite Core for validation, Response shape, and Mapping execution.

**Relation to Runtime S5.5.** Existing **contract validation** pseudocode (Runtime §5.5) uses `ContractValidator.validate(contractRef, data)` for **`contractHook`** flows (e.g. pipeline / gate actions that validate a **data bag** against a contract). **Formspec-backed human tasks** under this protocol use **`validateFormspecTaskResponse`** (§15.5) inside **`submitTaskResponse`** (§15.6) when a **full Response document** is returned. A single workflow MAY exercise **both** paths: task submission (§15.6) and later **contractHook** validation (§5.5), in the order declared by the Kernel Document.

**Validation trigger (review #5, integration gaps §1.5).** The coprocessor **gate** runs at **`submitTaskResponse`** (per-task, tied to `taskId`). Governance **data validation pipelines** (`contractHook`, Governance S5) are **orthogonal**: when §15.6 step 6 runs, it extends the **single completion bundle** for that task — Formspec validation first, then configured pipeline stages on the validated payload — unless the Kernel Document explicitly sequences them otherwise.

---

## §15.2 Task creation pin (processor obligations)

When executing `createTask` with `binding: "formspec"`:

1. **Resolve** the Formspec Definition for the task (from Kernel `contractRef` / binding `formspec`). Record **`definitionUrl`** = that Definition’s canonical **`url`** (Core Definition document) and **`definitionVersion`** = pinned version (Kernel S9.6 / Core VP-01 — cite only). These two values MUST be the same tuple the host will place on the completed Response.
2. Create task in lifecycle state per Governance S10.1 (typically `created` → `assigned` → `claimed` when actor opens work).
3. Build **`FormspecTaskContext`** (§15.3).
4. Invoke host **`TaskPresenter.presentTask`** (§15.4).

---

## §15.3 FormspecTaskContext (normative object)

| Property | Type | Req | Description |
|----------|------|-----|-------------|
| `taskId` | string | R | Processor-generated task id (stable for idempotency). |
| `processId` | URI | R | WOS workflow process id. |
| `contractRef` | string (URI) | R | Resolved Kernel **contract** reference (normalized string per `wos-kernel.schema.json`) — may differ from Formspec **`definitionUrl`**. |
| `definitionUrl` | string (URI) | R | Canonical Formspec Definition **`url`** (Core Definition). MUST equal **`response.definitionUrl`** on successful submit (VP-01 tuple). |
| `definitionVersion` | string | R | Pinned Formspec Definition version. MUST equal **`response.definitionVersion`** on successful submit (VP-01 tuple). |
| `binding` | const `"formspec"` | R | Discriminator. |
| `assignedActor` | string | R | From `assignTo` / kernel task assignment. |
| `prefillData` | object | O | Field-path → value map for initial Response seeding. |
| `prefillMappingRef` | URI | O | Formspec Mapping Document; processor SHOULD use **`direction` `"both"`** when one document maps both ways (Mapping spec — cite §direction). |
| `responseMappingRef` | URI | O | Forward mapping: Response `data` → case file (Mapping spec, forward). |
| `deadline` | date-time | O | From SLA / `taskTimeout`. |
| `impactLevel` | string | O | Copy from instance / governance — drives Respondent Ledger rule (§15.10). |
| `extensions` | object | O | Keys `x-*` only. |

*Note:* `responseMappingRef` duplicates the integration-gap proposal; place it on **Kernel `ContractReference`** in `wos-kernel.schema.json` so both processor and host read one declaration.

---

## §15.4 Host interface: TaskPresenter (new §12.9)

Add to **§12 Host Interfaces**:

| Operation | Input | Output | Req | Description |
|-----------|-------|--------|-----|-------------|
| `presentTask` | `context: FormspecTaskContext` | none | R | Host renders Definition (host-specific UI). Host MUST NOT mutate WOS case state on presentation alone. |
| `dismissTask` | `taskId`, `reason` | none | O | Host released UI without valid submission (informative to processor). |

Callback pattern (normative):

- When the actor finalizes a Formspec Response with **`status: "completed"`**, the host MUST call **`processor.submitTaskResponse(...)`** (§15.6) with the **full Response document**.

---

## §15.5 ContractValidator extension: `validateFormspecTaskResponse` (normative)

**Problem:** Existing S12.3 `validate(contractRef, data)` is insufficient to normatively require **Response envelope** checks.

**Add** to **§12.3**:

| Operation | Input | Output | Description |
|-----------|-------|--------|-------------|
| `validateFormspecTaskResponse` | `pinned: { definitionUrl, definitionVersion }`, `response: object` | `ValidationOutcome` | See algorithm below. |

**`ValidationOutcome`** (normative shape, WOS-defined wrapper):

| Field | Description |
|-------|-------------|
| `envelopeValid` | boolean — `response` **passes full JSON Schema validation** against Formspec `response.schema.json` (required properties **and** `additionalProperties: false` at the root — not a “required fields only” partial check). |
| `pinMatch` | boolean — `response.definitionUrl` / `response.definitionVersion` equal `pinned`. |
| `definitionValid` | boolean — Formspec processor reports no error-severity validation results when evaluating **`data`** per **Core §5.5** (shape `timing`, global validation behavior) as for a **submission** that would allow `status: "completed"` — **not** WOS Runtime evaluation mode (§10.2–10.3); cite Core only. |
| `errors` | array of human-readable strings (aggregation layer). |
| `validationResults` | optional array — MAY echo Formspec **ValidationResult** entries (Core §5 / `validationResult.schema.json` — cite only). |

**Naming:** **`ValidationOutcome`** is a **WOS host-interface wrapper** (envelope + pin + aggregate validity). It is **not** the Formspec document type **`ValidationResult`** (single constraint result); avoid conflating the two in schemas or generated types.

**Algorithm (pseudocode):**

1. If `response` fails **full** JSON Schema validation for Formspec Response (including unknown top-level keys) → `envelopeValid: false`, stop.
2. If pin mismatch → `pinMatch: false`, stop (MUST NOT map to case).
3. Else delegate to Formspec-conformant processor with Definition pinned to `response.definitionVersion` and evaluate **`data`**; set `definitionValid` from absence of error-severity results (Core rules — cite only).
4. Emit provenance **`contractValidation`** (Runtime S5.5) including outcome flags and correlation ids.

---

## §15.6 Processor API: `submitTaskResponse` (normative)

**Input:**

| Field | Req | Description |
|-------|-----|-------------|
| `taskId` | R | From context. |
| `response` | R | Full Formspec Response object. |
| `actorId` | R | Submitter. |
| `timestamp` | R | ISO 8601. |
| `idempotencyToken` | O | Runtime S4.3 — retries MUST NOT double-apply mapping. |

**Steps:**

1. **Authorize** `actorId` vs `assignedActor` / delegation (§15 executive table).
2. **Phase 1 gate:** If `response.status` is not **`completed`**, the processor MUST NOT complete the task or map to case state. It MUST reject with a **structured error** (RECOMMENDED: `code: "taskResponseStatusNotCompleted"`, `status: response.status`) — aligns with §15.4 host callback, Core **`status`** semantics, and **VE-05** (saving not blocked; only **`completed`** implies submit eligibility — Core §5.5, schema `status` description).
3. **Idempotency** — if token seen, return same result.
4. Call **`validateFormspecTaskResponse`** with task pin + `response`.
5. If not (`envelopeValid` and `pinMatch` and `definitionValid`):  
   - Transition task **`claimed` → `failed`**.  
   - Fire **`failureEvent`** if declared on `createTask`, else synthetic **`$task.failed`**.  
   - MUST NOT run case mapping.  
   - MUST emit provenance (failure + validation snapshot).
6. If pipeline hooks exist (`contractHook`), run Governance pipeline stages (Governance S5 — cite only).
7. **Map** using `responseMappingRef` or default identity mapping (integration gaps §1.2.4 behavior).
8. Transition task to **`completed`**, fire **`completionEvent`**, emit **`taskCompleted`** provenance with Response id, mapping ref, mutation list.

**Processor unavailable (bias #7):** Host MUST retry `submitTaskResponse` with the same idempotency token until definitive success or structured failure; processor MUST dedupe.

---

## §15.7 Kernel document extensions (schema + spec cross-refs)

In **Kernel** spec / `wos-kernel.schema.json` (exact placement follows your `ContractReference` / action shapes):

| Addition | Location | Purpose |
|----------|----------|---------|
| `responseMappingRef` | `ContractReference` | URI of Mapping Document (forward). |
| `prefillMappingRef` | optional on contract or task action | URI for case → Response prefill. |
| `completionEvent` | `createTask` action | Event name on success. |
| `failureEvent` | `createTask` action | Event name on validation failure path. |

---

## §15.8 WorkflowProcess / active task tracking (informative minimum)

Processors SHOULD persist enough **active task metadata** to recover after crash: at least `{ taskId, definitionUrl, definitionVersion, assignedActor, idempotencyKeysSeen }` (mirror §15.3; use **`definitionVersion`**, not “contract version,” to match Response pinning). The artifact **`work-spec/schemas/wos-process.schema.json`** exists today but does **not** yet define an **`activeTasks`** (or equivalent) array — **schema gap:** add an optional `activeTasks` array of `ActiveTask` objects in a follow-on PR when S15 lands in prose.

---

## §15.9 Respondent Ledger (rights / safety)

If `impactLevel` is **rights-impacting** or **safety-impacting**:

- Processor MUST require an active **Respondent Ledger** reference for the task **before** accepting **`completed`** (Respondent Ledger Add-On — cite only).
- Provenance MUST link **`respondentLedgerRef`** per integration gaps §1.2.7 structure.

For **operational / informational**, ledger is OPTIONAL.

---

## §15.10 Separation: ledger vs notice (review #1)

Normative **informative** paragraph: **Respondent Ledger** evidence (who did what on the form) MUST NOT be conflated with **adverse decision notice** delivery records. Cross-reference only; do not define notice protocol here.

---

## §15.11 Single completion trigger (review #5)

State: **Formspec validation + optional pipeline** constitute the **single gating bundle** before task completion and case mutation. Processors MUST NOT advance case state from this task path without passing the bundle.

---

## Authoring checklist (wos-spec-author)

- [x] Introduction states **additive-only** Formspec invariant (§15.1 + executive WOS rules).
- [x] No FEL grammar or bind evaluation prose — only Core section pointers.
- [ ] Land `validateFormspecTaskResponse` + **`ValidationOutcome`** in published **`runtime.md` §12.3** (this file is the draft source).
- [x] Governance S8 cited for rejection/explanations; task **`claimed` → `failed`** on validation failure (executive table + §15.6).
- [ ] **Kernel:** `responseMappingRef`, `prefillMappingRef`, `completionEvent`, `failureEvent` in `wos-kernel.schema.json` + Kernel spec.
- [ ] **WorkflowProcess:** optional **`activeTasks`** in `wos-process.schema.json` (§15.8).
- [ ] Optional: **`wos-runtime`** (or kernel) schema **`$defs`** for `FormspecTaskContext` / `ValidationOutcome` for LLM-authorable contracts.

---

## Suggested insertion point

**File:** `work-spec/specs/companions/runtime.md`  
**After:** `## 14. Relationship-Triggered Events` (current file ends ~§14.5)  
**Then:** Add `## 15. Formspec Coprocessor Protocol` with subsections **15.1–15.11** as above; update **§2.2 Host Interface Requirements** to list **§12.9 TaskPresenter** (Formspec coprocessor profile) alongside existing S12 interfaces.

**Also update:** Runtime **Abstract** and **Scope** bullets to mention Formspec-backed human tasks.

---

## References (normative elsewhere)

- Formspec Core: Response §2.1.6, VP-01 (S1.4.3 / S6.4), processing (§2.4), validation / shape timing (**§5.5**), **`status` / VE-05** (§5.5 + schema `status`), Mapping spec `direction`.
- Formspec schemas: `response.schema.json`, `validationResult.schema.json`, `mapping.schema.json`.
- WOS: Runtime S12, S5.5, S8; Governance task lifecycle (`created` / `claimed` / `failed` / `completed`), S5 pipelines, S8 rejection.
- Design input: `thoughts/archive/specs/2026-04-10-formspec-integration-gaps.md` §1.2.
