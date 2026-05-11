# Case Management Boundary Refactor: Aggregate Synthesis

**Date:** 2026-05-10
**Subject:** Synthesis of 5 independent AI consultant reviews and codebase investigations regarding the `Case` vs `CaseProcess` boundary refactor proposal.

**Provenance:** Five consultant markdowns are listed in §7. Three additional written validations exist on disk (`case-management-validation-claude-opus-4-7-1m.md`, `case-management-validation-glm-5.1.md`, plus R1/R3/R5 paths noted there). Convergent review is signal, not proof—each load-bearing claim below should carry file or spec pins when drafting the ADR.

**Integrated validation (2026-05-10):** Cross-read against live artifacts using `code-scout` (repo paths, OpenAPI vs server, schemas), `spec-expert` (Formspec Core §2.1.6.1, ADR-0074/0071), `wos-expert` (kernel `spec.md`, `wos-provenance-log.schema.json`, `wos-workflow.schema.json`, ADR-0073/0080), `trellis-expert` (`trellis-core.md` §1.2, §4, §15, §22.4, §23.2–23.4), and `cross-stack-scout` (seams, `GOAL.md`, `VISION.md`, `workspec-server/.../VISION.md`). Outcomes are woven into §2–§5; §7 P0/P1 items remain the dispute register where synthesis intentionally stayed open.

## 1. Executive Summary

The foundational premise of the original consultant proposal is **architecturally correct**: conflating the durable domain `Case` with the workflow execution `CaseInstance` is a critical flaw. `CaseInstance` is a running workflow process; using it as the root product abstraction creates bloated life-cycles and overloaded state containers.

However, the original proposal harbored critical structural blind spots—treating `Case` as a new, parallel authoritative data store, inventing new kernel seams, and violating existing deployment security commitments.

This document synthesizes all reviews and Trellis codebase investigations to establish the corrected, executable architectural boundary. **Post-validation edit:** §2–§3 were tightened so WOS governed-case authority, provenance event literals, kernel section numbers, ADR-0080 vs phantom ADR-0077, and Formspec handoff wording match the stack today.

## 2. The Architectural Triad: Trellis ↔ WOS ↔ Case

The solution is not creating a "fourth center" or a parallel database. The architecture must compose across the three existing layers:

### A. Trellis: The Cryptographic "Case Ledger" (Infrastructure)

Trellis Core defines what the substrate is *not* (not a workflow engine, not Formspec semantics) and what it *is* authoritative for: envelope bytes, hash chain, checkpoint, export, verification—for **nothing else** in the product sense (`trellis/specs/trellis-core.md` §1.1, §1.3, §4). **Implementation check:** `trellis-store-postgres` migrations are event-canonical tables, not casework CRM tables—consistent with “no product case-management schema inside Trellis,” but the concrete “no notes / participants” list is **engineering judgment**, not a quoted Trellis MUST.

Under **Trellis Core §1.2 (Phase 3)**, the **case ledger** is a hash-chained sequence composing **sealed response-ledger heads** (Formspec) with **WOS governance** into one matter (`trellis-core.md` §1.2, §22.4). **Formspec pin:** Intake Handoff binds a **`ledgerHeadRef`** to the **respondent-ledger** head at handoff; Trellis **MAY** anchor that evidence, but the handoff **must not** be treated as case-creation authority—**WOS** owns the governed boundary (`formspec/specs/core/spec.md` §2.1.6.1). Do **not** paraphrase that as “Formspec intake heads” in normative prose; use **respondent-ledger / response-ledger head** language. Stack docs still note a pending **respondent-ledger → case-ledger** spec rename (`work-spec/CLAUDE.md`); Trellis Core already says **case ledger** in §1.2—keep ratified vs doc-drift explicit in the ADR audit trail (CASE-SYNTH-08).

### B. WOS: Process Governance (The Instrument)

WOS governs process transitions, AI oversight, and accountability, and emits governance events that append to the Trellis chain. **Wording discipline:** *“WOS is not the case”* is only safe when **“the case”** means the **integrity spine plus derived projection**—not the **governed case boundary**. Normatively, **WOS owns governed case identity and the `case.created` provenance event** (ADR-0073 D-1; `work-spec/CLAUDE.md` key rules; `wos-provenance-log.schema.json` → `CaseCreatedRecord`). WOS **executes workflow processes in the shell of a governed case**; it does **not** replace Trellis as the commit-order authority for anchored bytes (ADR-0070 D-1).

### C. Case: The Domain Projection (The View)

**CRITICAL CORRECTION:** The product `Case` aggregate is **NOT** a second parallel **source of truth** for governed identity or append order. **Split authorities (post-validation):** (1) **Trellis** — authoritative **linear order** for committed canonical events and integrity artifacts (`trellis-core.md` §23.2.5); projections **derive from** canonical truth (§2.1 class 4, §15). (2) **WOS** — **governed case identity** and **`case.created`** emission per ADR-0073 D-1. (3) **`Case` projection** — rebuildable **metadata-first** state in the operator’s `projections` schema (see `workspec-server/crates/wos-server/VISION.md`: `canonical` vs `projections`). The ADR must argue **“Case as projection”** against a credible alternative (e.g. WOS-centered domain model with Trellis anchoring—`wos-core` `CaseInstance` already carries `case_state: serde_json::Value` and provenance paths), not treat projection as the only coherent pattern (CASE-SYNTH-10).

## 3. Critical Corrections to the Original Proposal

Before an ADR can be written, the following flaws in the original proposal must be explicitly resolved:

### 3.1. The ADR-0073 Case Origination Collision

The proposal suggested a Case could be created manually "with zero processes," silently bypassing WOS.

* **The Conflict:** ADR-0073 mandates that *WOS is the only layer that emits `case.created`*.
* **The Resolution:** Every Case origination MUST emit a provenance record whose **`event` literal is `case.created`** into the canonical ledger—the locked contract today is `wos-provenance-log.schema.json` → `$defs/CaseCreatedRecord` (`const: "case.created"`). A deliberate rename would require coordinated updates to provenance schema, Trellis **`wos.*` event_type** registration, OpenAPI, and fixtures—do not introduce a second string such as `wos.case-created` in prose without that ADR (CASE-SYNTH-16). ADR-0073 (or **`0073-bis`**) must define a **manual case creation** path (no `IntakeHandoff`) that still emits this governed boundary with tenant/class invariants. **Phase 1 step-zero:** any **new** `wos.*` `event_type` values must be **registered in the bound registry** per Trellis **`§23.2` item 2** cross-referencing **`§14`** (namespace rules additionally in **`§23.4`**—do not cite §23.4 alone; CASE-SYNTH-20).

### 3.2. ADR-0074 Per-Class Encryption Violation

The proposal designed the `Case` API to return `notes`, `communications`, and `participants` as flat JSON fields.

* **The Conflict:** These fields represent classified domain content. Serving them as plaintext violates the SBA strict per-class encryption mandate.
* **The Resolution:** The `Case` projection carries **metadata only**. Domain content (`notes`, `artifacts`, `decisions`) are subresources whose bodies are fetched separately as ciphertext plus wrapped key-bag fragments; **routine read path** stays client-decrypt where the deployment profile demands it. **`GOAL.md`** prod-MVP and **`wos-server/VISION.md`** explicitly allow **audited server-side decryption** for bounded processing—no contradiction **if** the Case API never flattens classified bodies into the projection document (CASE-SYNTH-17). ADR-0074 normatively targets **Formspec Response** wire shape and processing; stack Case API posture should **cite ADR-0074 + deployment profile** together, not collapse them into one sentence.

### 3.3. The "Governed Output Path" Seam

The proposal invented new output paths for workflows to write to the Case (`CaseStateMutation`, `CaseArtifact`, etc.).

* **The Conflict (reframed):** New write paths must not bypass the **governed output commit pipeline** or sprawl unnamed kernel semantics. **`thoughts/adr/0077-canonical-kernel-extension-seams.md` is not present** under stack `thoughts/adr/` (glob empty); an **archived** copy lives under `formspec/thoughts/archive/adr/`. **`work-spec/CLAUDE.md`** still names “ADR 0077” and the **six kernel extension seams** (`actorExtension`, `contractHook`, `provenanceLayer`, `lifecycleHook`, `custodyHook`, `extensions` / `x-` keys)—that list is **verified**. Do **not** conflate those **seams** with ADR-0080’s **six writer surfaces** (capability, service, signal, task, parallel, foreach); that was a **category error** in earlier synthesis wording (CASE-SYNTH-03).
* **The Resolution:** **Anchor on ADR-0080** (*Governed output commit pipeline*, **Proposed**) and **`work-spec/schemas/wos-workflow.schema.json` → `$defs/OutputBinding`**—today’s object has `on`, `contractRef`, `projection`, `writeScope`, `mutationSource`, `verificationLevel`; there is **no `target` property yet** (proposal-only). Extend **`OutputBinding` / `writeScope` / kernel §5 `caseFile` together** (CASE-SYNTH-21)—kernel **§5.1** still describes a **single** process-scoped `caseState` container until §5 admits process-scoped vs case-scoped partitions. **Kernel citations:** JSONPath profile for binding path *values* is **`§9.2.21.1`**; the workflow **`outputBinding`** property mapping responses → case state paths is **`§9.2.22`** (Arazzo uses **`§9.2.23`**). Saying only “§9.2.21” without **.1** or **§9.2.22** mis-pins readers. Extending **`$defs/OutputBinding`** is **not** inventing a **seventh kernel seam**—it evolves one declarative pipeline shape ADR-0080 already unifies.
* **Optional discriminator language for the ADR:** e.g. `processCaseState` (today’s case-state writes) vs. `caseArtifact` / `caseDecision` / `caseTimeline`—but treat as **new schema + spec + runtime** work, not a field that already exists in HEAD.

## 4. Identity, Naming, and Blast Radius

The refactor cannot rely on lazy aliases without accruing unacceptable technical debt.

1. **JSON Marker:** Renaming `$wosCaseInstance` → `$wosProcessInstance` is a **coordinated schema + lint + fixture** event (`wos-case-instance.schema.json` requires the marker today; `work-spec/crates/wos-lint/src/document.rs` maps it). **Open choice** (§7 CASE-SYNTH-01): one-shot rename vs keep-as-legacy—the ADR must **argue** the path, not assert it.
2. **TypeID Prefix:** Today `case_` mints workflow-instance identity. **Prefer (CASE-SYNTH-02):** mint domain **`Case` under a new prefix** (`casefile_` / `matter_` / `cf_`) and reserve `case_` for `CaseProcess`, **or** document a **forced migration** with cut-over—**not** silent in-place reassignment while reusing the `case` URN segment validated by **`work-spec/schemas/api/_common.schema.json`** `WosResourceUrn`.
3. **caseRelationships:** Do not invent a new relationship taxonomy from scratch. Extend the existing Kernel §5.5 vocabulary (`parent | child | sibling | related | supersedes`) via `x-` prefix extensions.

## 5. Phased Execution Plan (MVP vs. Post-MVP)

To align with `GOAL.md` (prefer work that makes the seed deployment real), the execution must be strictly phased:

### Phase 1: MVP (Structural Realignment)

* **Registry / types (step-zero):** Confirm Trellis **bound `event_type`** registry coverage for any **new** `wos.*` verbs the ADR introduces (`trellis-core.md` **§23.2** + **§14**; **§23.4** for namespace rules).
* **ADR & Naming:** Rename `$wosCaseInstance` → `$wosProcessInstance` **only after** explicit cost/benefit (§7 CASE-SYNTH-01); if renamed, coordinate `wos-case-instance.schema.json`, `wos-lint`, conformance, fixtures. Define TypeID prefixes per §4.2 / CASE-SYNTH-02 (**prefer new prefix for domain `Case`**, not silent `case_` reassignment).
* **Governed output:** Ship **`$defs/OutputBinding` + kernel §5** evolution per **ADR-0080** (and kernel **§9.2.21.1 / §9.2.22** citations)—including any **`target`/`writeScope` discriminator** as **net-new schema + spec** work (HEAD has no `target` today).
* **The Projection:** Define the basic `Case` projection schema (**metadata only**—enumerate fields per CASE-SYNTH-13); materialize from replay with **fixture-backed** idempotency/watermark tests (CASE-SYNTH-23).
* **1:1 Constraint (deployment profile only):** The seed deployment **may** enforce one active `CaseProcess` per `Case` to reduce orchestration risk, but the **normative** model must still allow **zero or many** processes per ADR-0073 initiation modes—otherwise Phase 1 **re-conflates** Case with process in product behavior (CASE-SYNTH-18). Prefer at least **one asymmetric** acceptance scenario in Phase 1 (manual case with zero processes **or** case open while a bound process completes).
* **API / server parity:** Phase 1 ADR should either fix or explicitly inherit **OpenAPI vs `workspec-server` route drift** (`/explanation` vs `/explain`, suspend/resume/terminate in OpenAPI but absent from `instances.rs`, tasks path skew—CASE-SYNTH-05/19) and the **9 vs 6 lifecycle enum** projection rule (`wos-case-instance.schema.json` vs public API schema—CASE-SYNTH-06).

### Phase 2: Post-MVP (Full Ontology)

* Support for multiple `CaseProcess` instances per `Case`.
* Ad-hoc `notes` and `communications` (emitted as WOS events outside a workflow transition).
* Complex Case splitting and merging logic.
* Schema evolution for durable case domain data.

## 6. Next Step: The ADR

The assigned engineer must now write **ADR 00XX — Case / Process Boundary and Case Projection Introduction**, incorporating the constraints and resolutions defined in this synthesis document. The ADR must clearly present the **"Case as a Projection"** mechanism as its central architectural decision under an **“Alternatives Considered”** section (projection vs WOS-centered domain model with Trellis anchoring—CASE-SYNTH-10), and must **resolve Case↔ledger cardinality** (CASE-SYNTH-11). **Dual-durable recovery:** acknowledge projection lag vs committed chain when `Case` and `CaseProcess` diverge (CASE-SYNTH-12; compose with ADR-0070 ordering). **CaseArtifacts:** pin **Formspec `definitionUrl` + `definitionVersion`** (and ADR-0074 **profile** when bucketed)—not vague “schema version” alone (CASE-SYNTH-24; Formspec Core §6.4 / §2.1.6.1; ADR-0071 D-1 pins).

---

## 7. Reviewer Feedback

### Reviewers

* **R1** — Claude Opus 4.7 (Cursor IDE), 2026-05-10 — [`case-management-validation-claude-opus-4.7.md`](./case-management-validation-claude-opus-4.7.md)
* **R2** — GLM-5.1 (opencode), 2026-05-10 — [`case-management-validation-glm-5.1.md`](./case-management-validation-glm-5.1.md)
* **R3** — GPT-5 Codex, 2026-05-10 — live-doc assessment against ADR-0073, ADR-0074, ADR-0080, Trellis core, `GOAL.md`, `wos-server/VISION.md`
* **R4** — Claude Opus 4.7 (1M context, Claude Code), 2026-05-10 — [`case-management-validation-claude-opus-4-7-1m.md`](./case-management-validation-claude-opus-4-7-1m.md); dispatched `spec-expert` + `cross-stack-scout` in parallel against the original, then re-reviewed the synthesis
* **R5** — Gemini CLI (Agent), 2026-05-10 — live-doc assessment focusing on structural integrity, event-sourced projections, and validation strategy

### Preserved (do not regress)

Case-as-projection with **split Trellis / WOS / projection authorities** (§2.C), **metadata-only Case surface** with **profile-aware** encryption story (§3.2), and **ADR-0080–aligned extension of `$defs/OutputBinding` + kernel §5**—not phantom ADR-0077 / seventh-seam language (§3.3)—are the load-bearing wins. **`target` is a planned discriminator, not a present schema field.** The **1:1** rule stays only as **MVP deployment profile**, per CASE-SYNTH-18.

### P0 — must resolve before ADR drafting

**CASE-SYNTH-01 — Revert §4.1 `$wosCaseInstance` → `$wosProcessInstance` rename, or surface the argument.** *[R1; +1 R2]*

Original memo, `spec-expert`, and validation review all converged on keep-as-legacy. Synthesis flips with a one-sentence claim that doesn't survive scrutiny: `work-spec/crates/wos-lint/src/document.rs:84-90` maps `"$wosCaseInstance"` → `DocumentKind::CaseInstance`; rename is a schema-version-bump event for every fixture, conformance trace, and authored workflow. Pre-release VISION.md §II ("no backwards compatibility / nothing is released") makes one-shot rename defensible — but the argument must be made, not asserted.

**CASE-SYNTH-02 — Replace §4.2 in-place TypeID reassignment with a new prefix for the Case aggregate.** *[R1; +1 R2, R4, R5]*

Reassigning `case_` keeps lexical form while changing the referent — every existing ID in fixtures, Trellis export bundles, and provenance records silently drifts. TypeID prefixes are value-level identifiers embedded in every cross-reference; **`work-spec/schemas/api/_common.schema.json`** `WosResourceUrn.pattern` includes a literal **`case`** segment among allowed families—reuse without migration defeats parse-time safety. Keep `case_` minting workflow instances (→ `CaseProcess`); mint Case under a new family (`casefile_` / `matter_` / `cf_`), or name the reassignment as a forced migration with a cut-over commit. Synthesis treats it as notational; it is not. *(R5: Reusing a prefix is a critical data corruption risk in any event-sourced projection. New prefix is mandatory.)*

**CASE-SYNTH-03 — Fix the ADR-0077 citation in §3.3.** *[R1; +1 R2, R3]*

Normative pins: **`§9.2.21.1`** (JSONPath profile for binding path values), **`§9.2.22`** (workflow `outputBinding` property), **`§9.2.23`** (Arazzo)—not “§9.2.21” alone. The six **kernel extension seams** are exactly `work-spec/CLAUDE.md` heuristic 3; **`thoughts/adr/0077*`** under the stack ADR tree is **empty** (phantom at that path; archived 0077 under `formspec/`). **ADR-0080** + **`$defs/OutputBinding`** are the governed-output anchor; reframe as “extend the existing pipeline object and kernel §5, do not invent a new seam.”

**CASE-SYNTH-04 — Carve §3.1 manual case creation into a follow-up ADR (`0073-bis`).** *[R1; +1 R3, R4, R5]*

Amendment is non-trivial: actor surface (no `ActorRef` for a UI caseworker today), authority chain (acceptance policy exists; ad-hoc creation needs an analog), tenant/scope source absent `IntakeHandoff`, and a **`case.created`** payload shape admitting "no WorkflowDocument bound." Folding all four into a one-line ADR-0073 amendment smuggles four real seam decisions into a one-line exception.

> **Addendum (was CASE-SYNTH-16) — Normalize the case-creation event name before drafting.** *[R3; +1 R4]* §3.1 introduces `wos.case-created` while ADR-0073 and `wos-provenance-log.schema.json $defs/CaseCreatedRecord` use `case.created`. Keep the existing contract name or propose a deliberate rename in the same ADR; do not start with accidental event-contract drift.

**CASE-SYNTH-10 — Present Case-as-projection as a design decision with alternatives, not self-evident truth.** *[R2; +1 R3, R4]*

Synthesis asserts "the only authoritative store is the Trellis ledger, therefore Case is a projection." Both clauses overstate. (R3) Trellis owns integrity/audit authority; WOS still owns governed case-identity emission per ADR-0073 D-1, and server-side canonical event storage exists operationally. (R4) §2.B reads WOS as pure instrument; it is also the case-boundary authority — tighten wording. (R2) Alternative: Case is a WOS-center domain model whose mutations produce Trellis events — the pattern `CaseInstance` already follows (Rust struct has `case_state: serde_json::Value`, emits provenance through `custodyHook`). Both are consistent with zero-trust. The ADR's "Alternatives Considered" must show why projection beats domain-model. Synthesis skips this analysis entirely.

> **Addendum (was CASE-SYNTH-11) — Define Case-to-Trellis-case-ledger cardinality and binding.** *[R2; +1 R3]* One Case = one ledger? Can a Case span ledgers? Does case split create a new ledger? Determines whether mutations flow through `custodyHook` and how the projection proves it replayed the right ledger scope. Blocking design choice — flagged independently by R2 and R3.

**CASE-SYNTH-12 — Address dual-state crash recovery for Case + CaseProcess as separate durable artifacts.** *[R2]*

ADR-0070 defines crash recovery for a single `CaseInstance`. With Case and CaseProcess as separate durable artifacts, a process crash mid-`CaseStateMutation` can leave the Trellis chain committed but the Case projection stale. New failure mode the synthesis doesn't mention. Phase 1's 1:1 constraint should at minimum acknowledge the risk even if the full solution is post-MVP.

**CASE-SYNTH-20 — List Trellis event-type registry binding as Phase 1 step-zero.** *[R4]*

Phase 1 cannot emit **new** `wos.*` event types (e.g. additional lifecycle or artifact verbs beyond what the bound registry already lists) until each identifier is **registered in the bound registry**—the hard MUST is **`trellis-core.md §23.2` item 2** + **`§14`**, with namespace discipline in **`§23.4`** (do not cite §23.4 alone). No byte change in Trellis to “wish” types into existence; cross-stack dependency surfaces in WOS + Trellis docs together. If unlisted, it is discovered mid-implementation.

**CASE-SYNTH-21 — Bifurcate kernel §5 `caseState` semantics alongside the `outputBinding.target` work.** *[R4]*

§3.3 adds `target` discriminator with values `processCaseState | caseArtifact | caseDecision | caseTimeline` (correct), but kernel §5 today defines only one notion of `caseState` (workflow business data, append-only log per §5.1). The new targets are nameless until §5 admits a process-scoped vs case-scoped distinction. ADR must pair the schema change with the kernel §5 extension, or the targets reference nothing.

### P1 — required for executable ADR

**CASE-SYNTH-07 — Preserve original memo's 35-edge-case matrix (lines 622-799) as Phase 1 failing-fixture skeleton.** *[R1]*

"1:1 MVP, splitting/merging Post-MVP" decides *when* cases land, not *whether they're covered*. Failing fixtures for deferred cases keep the test surface honest and prevent silent scope creep.

> **Addendum (was CASE-SYNTH-15) — Preserve the original's 15 invariants and 12 acceptance criteria as ADR input.** *[R2]* Several survived all three validations without challenge (invariant 4: CaseProcess lifecycle MUST NOT be treated as Case status; invariant 6: process completion MUST NOT imply Case closure). Inherit; do not re-derive.

**CASE-SYNTH-13 — Enumerate the "metadata only" projection schema fields explicitly.** *[R2]*

Phase 1 line 71 says "define the basic Case projection schema (metadata only)" without enumerating fields. Original analysis proposed ~25 (id, caseType, title, status, subjects, participants, processes[], timestamps, etc.). Which are metadata vs content requiring per-class encryption? Without this, Phase 1 scope is unbounded.

> **Addendum (was CASE-SYNTH-14) — Add Case projection schema evolution to Phase 2 scope.** *[R2]* When Case is a projection materialized from event replay, what happens when the projection schema adds a field that didn't exist at emission? ADR-0071 covers WorkflowDocument version pins; projection schema evolution is a distinct concern.

**CASE-SYNTH-17 — Split end-state privacy from prod-MVP privacy posture.** *[R3]*

§3.2 requires client-side decryption for Case subresources, but `GOAL.md` permits audited server-side decryption for prod-MVP. Preserve the ADR-0074 target while stating which deployment profile must satisfy it now.

**CASE-SYNTH-18 — Recast the 1:1 Case/CaseProcess rule as an MVP deployment profile, not ontology.** *[R3; +1 R4, R5]*

Enforce 1:1 in the seed flow if useful, but the normative model must still allow zero/many processes per Case. Otherwise the refactor recreates Case = CaseProcess through product behavior. (R4) Phase 1's 1:1 enforces *exactly the conflation the refactor exists to break* — minimum-viable validation is at least (a) manual Case origination with zero processes, or (b) Case open while bound CaseProcess is completed. Pick one for Phase 1, defer the rest, or Phase 1 is theater. *(R5: A 1:1 constraint in Phase 1 provides false confidence. MVP must include at least one asymmetric state to prove boundary isolation.)*

> **Addendum (was CASE-SYNTH-22) — Phase 1 must explicitly defer the multi-process write-conflict policy.** *[R4]* Original-proposal edge case 20: two processes write the same Case field. Phase 1's 1:1 sidesteps it; Phase 2 inherits a versioned-mutation / conflict-detection design problem the synthesis never names.

**CASE-SYNTH-19 — Add a Phase 1 API compatibility checklist.** *[R3]*

ADR must name OpenAPI, schema, spec, and server consequences: `/instances` aliases/deprecations, exported `oneOf` models, appeal migration semantics, route registry changes, parity tests.

> **Addendum (was CASE-SYNTH-05) — Resolve workspec-server ↔ OpenAPI drift in scope or explicitly defer.** *[R1]* `suspend`/`resume`/`terminate` in OpenAPI but missing from `workspec-server/crates/wos-server/src/http/instances.rs`; `/explanation` (OpenAPI) ≠ `/explain` (server); tasks routing inconsistent (`/tasks` vs `/instances/{id}/tasks`). Refactor inherits this drift silently if unaddressed.
>
> **Addendum (was CASE-SYNTH-06) — Resolve kernel-vs-API lifecycle enum mismatch.** *[R1]* `wos-case-instance.schema.json:273-285` has 9 status values (adds `declined`, `voided`, `expired`); public API `LifecycleState` has 6. `CaseProcess` schema must pick a truth and document the projection rule.

### P2 — discipline / process

**CASE-SYNTH-08 — Add citation discipline (file:line) throughout; verify Trellis "Case Ledger" status.** *[R1; +1 R4]*

Stack [`CLAUDE.md`](../../../CLAUDE.md) notes the `respondent-ledger-spec.md` → `case-ledger-spec.md` rewrite is *pending*. §2.A presents §1.2 / "Phase 3" / "Case Ledger" as established. Distinguish ratified vs proposed in the audit trail.

**CASE-SYNTH-09 — Name the source reviews in a §0 provenance block.** *[R1; +1 R2, R4]* **(partially addressed above)**

Aggregation claims require provenance. R2/R4: three validations on disk, not five — `case-management-validation-claude-opus-4-7-1m.md`, `case-management-validation-glm-5.1.md`, and original `case-management.md`. The three "critical corrections" map 1:1 to R1's findings. Cite each input (filename, agent type, date) so reviewers can audit which input drove which conclusion and where reviews disagreed. Convergent AI reviews on the wrong axis are amplified bias, not validated truth.

**CASE-SYNTH-23 — Define empirical validation strategy for the projection mechanism.** *[R5]*
If Case is a projection, Phase 1 must include fixture tests that assert the projection accurately rebuilds state from a mocked Trellis event stream, demonstrating idempotency (e.g. handling duplicate or out-of-order events securely). Without this, the "projection" is just a buzzword.

**CASE-SYNTH-24 — Explicitly map Formspec versioning in CaseArtifacts.** *[R5]* **(spec-expert: tighten wording)**

Since `CaseArtifact`s will encapsulate Formspec responses, the projection logic must record the **`definitionUrl` + `definitionVersion`** pin (Formspec Core Response / Intake Handoff—VP-01), plus **Privacy Profile** fields when ADR-0074 bucketed shape applies—not the vague phrase “schema version” alone. ADR-0071 **D-1** case-open pins are the cross-layer precedent for “which semantics at replay.” Projections fail replay if definitions drift without those pins in the event payload.
