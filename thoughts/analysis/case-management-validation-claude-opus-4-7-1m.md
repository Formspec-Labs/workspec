# Validation of `case-management.md` — Claude Opus 4.7 (1M context)

> **Superseded validation artifact.** This R-file is retained as derivation history. The controlling source of truth for case-management decisions is [`case-boundary-decision-report.md`](case-boundary-decision-report.md); when this file disagrees with that report, the report controls.

**Reviewer:** Claude Opus 4.7 (1M context), via `formspec-specs:spec-expert` + `formspec-specs:cross-stack-scout` dispatched in parallel.
**Subject:** [`case-management.md`](case-management.md) — Case/CaseProcess boundary refactor proposal.
**Date:** 2026-05-10.

## Verdict

Both agents converge: **boundary direction is correct (~70%), but the proposal is structurally misframed and underestimates three load-bearing collisions.** Adopt the refactor, but rewrite the premise before writing the ADR.

## The single structural error

The proposal treats `Case` as a **new authoritative aggregate**. Under wos-server's zero-trust end-state ([`workspec-server/crates/wos-server/VISION.md`](../../../workspec-server/crates/wos-server/VISION.md)), the only authoritative store is the Trellis case ledger; the only co-located store is the plaintext-content-free `projections` schema. **`Case` must be specified as a projection**, materialized by replay from the case-scoped event stream — not as a parallel store.

This is not cosmetic. It changes every API shape in §"Suggested API direction": `GET /api/v1/cases/{caseId}` cannot return plaintext `notes`/`communications`/`decisions`/`artifacts`. Those fields are content; under ADR-0074 per-class encryption they must be opaque refs + key-bag-fragment release, decrypted client-side. The proposal's `artifactsSummary?`/`decisionsSummary?` hints gesture this way but never commit. **This is the largest unflagged hole.**

## Three concrete normative collisions

1. **ADR 0073 D-1 ownership.** WOS is the *only* layer that emits `case.created`. The proposal's "Case MAY exist with zero processes" + manual creation + deferred intake silently demote WOS to owning *process* identity only. Either ADR 0073 amends to admit non-handoff manual origination through WOS, or a new layer above WOS gains emission authority (rejected). Proposal does not pick.

2. **Three-way "case" naming collision.** Product `Case` / Trellis `Case Ledger` (Phase-3, [`trellis-core.md §1.2`](../../../trellis/specs/trellis-core.md)) / WOS `$wosCaseInstance` runtime marker. Proposal acknowledges only the third. When the Trellis case-ledger spec ships, "case" becomes triply loaded in one commit unless the ADR pins it now. Plus the TypeID family prefix `_case_` for `instanceId` must be re-decided (probably `_process_`).

3. **"Governed output path" is not a seam.** The six named kernel seams (ADR 0077) do not include it. `CaseStateMutation`/`CaseArtifact`/`CaseDecision`/`Timeline append` are new output-binding *kinds*. **Right answer**: extend `outputBinding` ([kernel §9.2.21](../../specs/kernel/spec.md)) with a `target` discriminator (`processCaseState` | `caseArtifact` | `caseDecision` | `caseTimeline`). Do not invent a seventh seam.

## What the proposal reinvents that already exists

- **`CaseRelationshipKind`** — kernel §5.5 already has `parent|child|sibling|related|supersedes` with `bidirectional` + `x-` extensibility. Extend, don't replace.
- **Supersession / amendment / correction / rescission** — exists as `governance.amendmentTaxonomy` in `wos-workflow.schema.json`. `CaseDecision.supersedesDecisionId` should map onto this, not parallel it.
- **`correlationKey` / cross-case fan-out** — kernel §9.4, §14 (relationship-triggered events, depth-cap 3).
- **Process migration preserving caseId** — kernel §9.6 already has `instanceVersioning: pinned|migrateable`; just add `caseId` invariant under migration.
- **Intake handoff attach-to-existing** — ADR 0073 D-4 `workflowInitiated` mode + `IntakeAccepted.caseDisposition: attachToExistingCase` already landed in `wos-runtime`.

## Tactical disagreements between the two agents

- **`$wosCaseInstance` rename blast radius.** spec-expert reads it as large (touches lint, conformance discovery, CI gates, fixtures). cross-stack-scout grep'd it: 14 hits in `work-spec/` + case-portal SDK regen — *~20-30 files, one ADR, no stable reason to leave it misnamed.* The codebase favors cross-stack-scout's count. Do the rename in one shot.

- **Kernel §5 "Case State" semantics.** spec-expert flags that kernel §5 today defines only *one* `caseState` (workflow business data, append-only log per §5.1). The proposal's `CaseState` (durable case-domain) vs `ProcessState` (runtime) distinction does not exist in the kernel and would require a kernel spec extension, not just an API-layer addition.

## Root-domino dependency order

1. **Trellis** — register `wos.case-created`, `wos.case-closed`, `wos.case-reopened`, `wos.note`, `wos.communication`, `wos.artifact-attached` in the bound `event_type` registry (registry binding, no byte change).
2. **WOS kernel** — extend `outputBinding` with `target` discriminator; admit non-transition governance event paths (for ad-hoc notes — caseworker note still hits the chain, signed by staff actor, anchored via `custodyHook`); rename `$wosCaseInstance` → `$wosProcessInstance`; extend kernel §5 to distinguish process-scoped from case-scoped state.
3. **ADR 0073 amendment** — admit manual (no-handoff) case origination; enumerate validation discipline; pin three-way naming.
4. **wos-server EventStore** — `case_projection` reducer + metadata-only `/api/v1/cases/*` routes. **Cannot land before the zero-trust EventStore refactor.**
5. **Formspec** — minimal; existing `IntakeHandoff` continues; attach-to-existing path already exists.

## Recommendation before writing the proposed ADR

Write a **half-page reframing memo** at `thoughts/specs/2026-05-10-case-is-a-projection-not-an-aggregate.md` that:

- Reframes `Case` as a projection over the case-scoped Trellis event stream (not a new aggregate).
- Classifies `subjects`/`participants`/`notes`/`communications`/`decisions`/`artifacts` under ADR-0074 access classes; commits to ciphertext-plus-wrapped-DEKs on subresource reads.
- Pins the three-way "case" naming (product Case / Trellis Case Ledger / `$wosProcessInstance`) and the TypeID prefix change.
- Names the `outputBinding.target` discriminator as the WOS-layer structural change (no new seam).
- Confirms `$wosCaseInstance` → `$wosProcessInstance` rename ships in one ADR, not as a perpetual alias.

After that memo lands, the proposed Step-1 ADR can be written with the right structural premise. Without it, the ADR risks ratifying a parallel store that contradicts the zero-trust commitment.

---

## Appendix A — spec-expert report (full)

### Preamble

The proposal is an AI-generated architectural analysis and consultant recommendation note. It is well-reasoned and identifies a real structural tension. The assessment below validates it against the current normative spec and schema surface.

### 1. Naming/Identity Collisions

**Three distinct "case" names exist and the proposal navigates them unequally.**

**WOS `CaseInstance`** is the runtime artifact. Its schema marker is `$wosCaseInstance: "1.0"` in `wos-case-instance.schema.json`. The description states explicitly: "A CaseInstance is the serialization format for a running workflow instance — it captures the complete runtime state needed to resume processing after a crash, migrate between processors, or audit past behavior." Its `instanceId` uses `TypeID` pattern `[tenant]_case_[ulid]`. This is scoped entirely to workflow runtime.

**Trellis "Case Ledger"** is defined at `trellis-core.md §1.2` as: "A hash-chained sequence of governance events composing one or more sealed response-ledger heads with WOS governance events into one adjudicatory matter. Phase 3." The `work-spec/CLAUDE.md` (Architecture section) says "'Case Ledger' (Trellis Core §1.2 term) is the canonical name for what was called 'Subject Ledger' or extended 'Respondent Ledger.' Spec rewrite from `respondent-ledger-spec.md` → `case-ledger-spec.md` is pending." The `trellis/CLAUDE.md` confirms: "'case ledger' (Core §1.2) is the canonical scope name." Critically, the Trellis case ledger does not yet have a finalized spec — `case-ledger-spec.md` does not exist at `trellis/specs/`. It is a Phase 3 superset concept.

**The proposal's "Case"** is a new product-level domain aggregate above both.

The proposal does acknowledge the naming risk in edge case 32 ("caseState currently means workflow process business data") but does NOT acknowledge the Trellis Case Ledger collision. The proposed `Case` aggregate uses the bare word "case" — the same word Trellis uses for its integrity-anchored adjudicatory container, which will eventually be the cryptographic wrapper for all durable case events. There is a genuine **three-way naming collision**: product `Case` / Trellis `Case Ledger` / WOS `CaseInstance`, and the proposal underestimates the Trellis one because the Trellis Phase 3 spec has not yet shipped. When it does ship, "case" will be doubly loaded in a single stack commit: the product domain aggregate and the Trellis cryptographic ledger scope. This is manageable if the ADR explicitly pins the distinction — Trellis Case Ledger is the integrity substrate for case events; the product Case is the domain aggregate — but the current proposal does not do that pinning work.

The `instanceId` TypeID in `wos-case-instance.schema.json` uses the family prefix `case` (`[tenant]_case_[ulid]`). The proposal recommends renaming the WOS runtime object to `CaseProcess` but does not address whether the TypeID family prefix also changes. If it stays `_case_`, the TypeID shape of a CaseProcess remains `[tenant]_case_[ulid]`, which will confuse consumers once the product `Case` aggregate gets its own IDs. The ADR must decide the CaseProcess TypeID prefix, likely `[tenant]_process_[ulid]`.

### 2. Existing Case-Initiation Contract

ADR 0073 (Accepted, 2026-04-23) establishes D-1 with no ambiguity: "WOS is the only layer that emits the governed case boundary event... the ownership is closed: Formspec MUST NOT emit `case.created` or an equivalent governed-case event." D-4 enumerates two first-class modes: `workflowInitiated` and `publicIntake`. D-7 defines `intakeAccepted`, `intakeRejected`, `intakeDeferred` as the possible WOS acceptance outcomes.

These are normatively confirmed in `wos-provenance-log.schema.json`, which defines `$defs/CaseCreatedRecord` with `event: "case.created"` as a required constant, and `$defs/IntakeAcceptedRecord` with `event: "case.intake.accepted"`. These record kinds are already landed in production schema and runtime.

**The proposal's "A Case MAY have zero CaseProcesses" and "deferred intake may create a Case without a CaseProcess" stance directly creates a tension with D-1.** If a Case can come into existence without WOS processing (e.g., a staff member creates a Case through a product UI that does not invoke a WOS workflow), then something other than WOS would emit the Case creation event. The proposal is silent on who emits `case.created` when a Case is created with zero processes. It says "Case management lives one layer above WOS" but does not say which layer owns `case.created` for manually-created cases.

This is the proposal's most important unresolved dependency. ADR 0073 D-1 says WOS owns governed case identity. The proposal is implicitly demoting WOS from "owns case identity for all cases" to "owns process identity." If the new Case layer is above WOS, and Cases can be created directly by the product layer without WOS involvement, a new ADR must either:
- Extend D-1 to say "Case creation is also a governed event emitted through WOS, even for zero-process cases" — i.e., WOS gains a `createCase` action that is distinct from starting a process; or
- Explicitly amend D-1 so WOS owns process creation (`process.created`) but a new layer above it owns case creation (`case.created`), and the old `case.created` provenance record kind is retired or scoped to WOS-process-initiated cases only.

This is not a minor ADR clarification. Amending D-1 of ADR 0073 requires cross-spec work (Formspec §2.1.6.1, the `intake-handoff.schema.json`, WOS kernel §8.2.3, `wos-provenance-log.schema.json`, `wos-runtime` acceptance path). The implementation status section of ADR 0073 notes `accept_intake_handoff(...)`, `caseCreated` versus `instanceCreated` separation, and case-attach/create application are already landed in `wos-runtime`. That code will need new branches for zero-process case creation.

### 3. Schema/Conformance Impact

**Schemas requiring change:**

`wos-case-instance.schema.json` — the entire schema represents what the proposal calls `CaseProcess`. Its root marker is `$wosCaseInstance: "1.0"`, its `instanceId` format annotation is `"format": "wos-case-typeid"`. Its `status` enum includes `active | suspended | migrating | completed | terminated | stalled | declined | voided | expired`. The proposal's suggestion of `pub type CaseInstance = CaseProcess` in Rust is internally viable as a type alias, but the `$wosCaseInstance` marker is a **JSON envelope discriminant**, not just a Rust type. The lint parser uses it to detect runtime artifacts uniformly (per ADR 0063 §2.3). A rename requires: (a) bumping the schema `$id`, (b) updating lint detection, (c) updating all serialized instances in test fixtures, and (d) updating `wos-runtime`'s `accept_intake_handoff` and `create_instance` code paths. The proposal correctly identifies the marker issue in "Compatibility stance" but understates the coupling depth.

`wos-workflow.schema.json` — defines `$defs/InstanceStatus` (the closed status enum mirrored from `wos-case-instance.schema.json`) and `$defs/CaseFile` (the `caseState` analog for author-time). Adding a `caseId` field to the instance schema is a schema-breaking change for any conformance suite that validates instance documents against `wos-case-instance.schema.json` (currently `additionalProperties: false`).

`wos-provenance-log.schema.json` — the `$defs/CaseCreatedRecord` uses `event: "case.created"` as a hardcoded constant. If the Case aggregate above WOS uses the same event name with different semantics, the provenance record shape is overloaded. A new event name (`process.created` or `case.process.attached`) will be needed.

**New schemas needed:**

The proposed `schemas/api/case.schema.json` is entirely new territory with no current equivalent. The `schemas/api/instance.schema.json` (referenced in `wos-case-instance.schema.json#/$defs/FormspecTaskContext` under `instanceId`) and `wos-workflow.schema.json#/$defs/InstanceStatus` are the public API surface projection for runtime instances.

**Conformance suite impact:**

The `RELEASE-STREAMS.md` conformance streams (signature, governance, AI deontic, advanced equity) all operate on the `$wosWorkflow` envelope and `$wosCaseInstance` runtime artifact as their anchor documents. Renaming `$wosCaseInstance` → `$wosCaseProcess` or adding `caseId` to the instance document will require: re-running all signature-profile fixtures (since provenance records reference `instanceId` values), updating governance conformance fixtures, and verifying that the `wos-conformance` test suite's discovery logic (which finds runtime artifacts by `$wosCaseInstance` marker) still works. The `every_load_bearing_conformance_rule_has_at_least_two_executable_fixtures` CI gate will trip if fixture documents carry the old marker.

### 4. `caseState` Semantics

The kernel spec §5 is named "Case State" and §5.1 says: "Case state is the structured data container associated with a workflow instance... Case state is an **append-only log** that grows regardless of lifecycle transitions. Lifecycle state (where in the workflow) and case state (what data exists) are independent."

The `caseState` property in `wos-case-instance.schema.json` is described as: "Current case file field values. The keys are field names declared in the Kernel Document's caseFile.fields (Kernel S5.2). Values conform to the declared field types (Kernel S5.3). Mutated by setData actions and completed task response mappings. This is the authoritative business-data snapshot at the current point in processing."

So currently `caseState` is: workflow-declared business data, scoped to a single workflow instance, mutated by `setData` actions, projected back via `responseMappingRef`. It is the canonical runtime projection of the author-time `caseFile.fields`.

The proposal correctly diagnoses the risk: without refactoring, `caseState` would be expected to serve as both the workflow's runtime data container AND the durable product-domain fact store for the broader Case. These are semantically different scopes.

The proposal's distinction between `CaseState` (durable case-domain) and `ProcessState` (runtime workflow) is **not currently supported** in any kernel spec section. Kernel §5 uses only one term — "case state" — for the workflow instance's business data. The author-time schema (`wos-workflow.schema.json#/$defs/CaseFile`) is the contract; the runtime snapshot (`wos-case-instance.schema.json#/properties/caseState`) is the materialized value. Neither has a concept of "process-scoped state" vs "case-scoped state." The proposal invents new semantics the kernel does not yet define, which means kernel §5 itself would need to be extended or bifurcated, not just the API layer. This is a kernel spec change, not a product-layer addition.

### 5. Governed Output Path Claim

The kernel's six seams (ADR 0077, normatively confirmed in kernel spec abstract and `wos-workflow.schema.json` header comment) are: `actorExtension`, `contractHook`, `provenanceLayer`, `lifecycleHook`, `custodyHook`, `extensions/x-` keys. There is no "governed output path" seam in this list.

What exists that is closest: `wos-workflow.schema.json` has a top-level `bindings` property (type `array`, items `OutputBinding`). The `State` properties include `outputPath` and `mergeStrategy` for `foreach` states (added in kernel §4.3.1), where "Per ADR 0078, the write goes through the governed output-commit pipeline (ADR 0080) with `mutationSource: computed`." ADR 0078 and 0080 describe the `outputPath` + `mergeStrategy` mechanism for foreach iteration results — but this is scoped to foreach state body outputs, not a generalized "governed output" seam for cross-Case writes.

The proposal's "CaseStateMutation / CaseArtifact creation / CaseDecision creation / Timeline append" output taxonomy is entirely new — none of these are currently defined anywhere in the WOS kernel, governance, or advanced governance specs. `CaseDecision` is not an existing WOS construct. The `Governance` block in `wos-workflow.schema.json` has `amendmentTaxonomy` with values like `correction`, `amendment`, `supersession`, `rescission`, `reinstatement` (visible in the schema examples at offset 222–237) — but these are governance amendment types for provenance records on the existing `CaseInstance`, not a separate `CaseDecision` aggregate.

ADR 0066 (referenced by the proposal) does not exist at `thoughts/adr/0066-stack-provenance-record-amendment-and-supersession.md` — that file is missing from the tree. The amendment/supersession vocabulary referenced is visible only in the `governance.amendmentTaxonomy` example in `wos-workflow.schema.json`. The proposal's "CaseDecision" concept partially overlaps with what the current specs call "determination" (a `transitionTags` value used with `caseFileSnapshot` per kernel §8.2.1), but determination is a transition-level concept inside a workflow, not a top-level Case resource.

In summary: there is no pre-existing "governed output path" concept the proposal can attach to. It is inventing a new seam. This is not automatically wrong — the stack's six named seams (ADR 0077) say "new extension points live at one of the six kernel seams or use x- patternProperties. Inventing new seams is a Q3 violation" (from `work-spec/CLAUDE.md` decision heuristic 3). The proposal would need either a new ADR amending ADR 0077 to add a seventh seam (`caseOutputHook` or similar), or it would route the governed output mechanism through the existing `contractHook` seam.

### 6. Adaptive Case Management / DCR Overlap

Advanced governance spec §4 (Constraint Zones) states at §4.1: "A constraint zone is a governance overlay on a kernel `compound` state, providing declarative internal behavior governed by relations between activities rather than explicit transitions. Constraint zones enable adaptive case management phases where the valid next actions are not predetermined."

§4.7 clarifies: "Constraint zones do not introduce a new kernel state type. They are a governance overlay on existing `compound` states."

The proposal acknowledges this ("DCR constraint zones handle adaptive work") and says "do not force all adaptive work into DCR." The implicit claim is that some adaptive case work — ad hoc notes, manual caseworker activity outside a workflow — should live in the new Case layer directly, not in a constraint zone.

This is structurally clean, not competing. DCR constraint zones live inside a `CaseProcess` (a running workflow with a compound state overlay). The `Case` layer above would host truly ad hoc, non-workflow activity. There is no contradiction in the spec — constraint zones are process-internal; the Case aggregate is process-external. The proposal could be more precise about this distinction: it currently says "do not force all adaptive work into DCR" without explaining what the alternative mechanism is. The answer (ad hoc Case-level activity via direct CaseArtifact/note writes that don't require a workflow) is implied but not made explicit. This is a spec authoring gap, not a conceptual conflict.

### 7. Edge Cases with Settled Spec Answers

Several edge cases in the proposal are already settled normatively and the proposal appears unaware of the existing machinery:

**Edge case 12 (Supersession/amendment/correction/rescission):** The `governance.amendmentTaxonomy` in `wos-workflow.schema.json` already defines this vocabulary as `["correction", "amendment", "supersession", "rescission", "reinstatement"]`. Kernel §8.2 defines `determination`-tagged transitions with `caseFileSnapshot`. The proposal should map its `CaseDecision.supersedesDecisionId` field against the existing `amendmentTaxonomy` rather than invent parallel vocabulary.

**Edge case 15 (Cross-case correlation/fan-out):** Kernel §5.5 (`caseRelationships`) and §9.4 (`correlationKey`) already define the cross-case event mechanism. Kernel §14 (relationship-triggered events) defines `$related.stateChanged`, `$related.resolved`, `$related.holdReleased` with a cascade-depth cap (`maxRelationshipEventDepth`, default 3). The proposal's "decide whether case-level correlationKey is separate from process-level event fan-out" is already answered: `correlationKey` in the kernel is instance-scoped for event routing. The proposal needs to decide whether the new Case aggregate gets its own correlation primitive or delegates to the CaseProcess level.

**Edge case 16 (Related cases — directional vs symmetric):** Kernel §5.5 already defines `bidirectional: boolean` (default `false`) and `type` as `parent | child | sibling | related | supersedes` with `x-` extensibility. The proposal's `CaseRelationshipKind` of `parent | child | sibling | predecessor | successor | appeals | appealed-by | related | supersedes | duplicate-of | duplicated-by | split-from | split-into | merged-into | merged-from` is a superset. It must decide whether to replace the kernel's `caseRelationships` vocabulary or extend it via `x-` prefixed kinds.

**Edge case 17 (Process migration):** Kernel §9.6 defines `instanceVersioning: "pinned" | "migrateable"` with "Running instances remain on their creation-time version unless explicitly migrated (Runtime Companion §11)." The `wos-case-instance.schema.json` property `definitionVersion` is described as "pinned at instance creation." The proposal's "Process migration must preserve caseId" is correct in direction, but the spec already has migration semantics; the proposal just needs to add `caseId` as an invariant under migration.

**Edge case 28 (Case status vs process status):** This is the proposal's strongest point from a spec gap perspective. The `wos-case-instance.schema.json#/properties/status` enum `active | suspended | migrating | completed | terminated | stalled | declined | voided | expired` is entirely workflow-lifecycle vocabulary. There is no current concept of a separate Case status. The proposal is right that `open | on-hold | closed | archived` is semantically distinct and does not exist anywhere in the current schemas.

**Edge case 6 (Case closure with active processes):** The kernel has no concept of "case closure" at all — only workflow terminal states (`completed`, `terminated`). This edge case is entirely unaddressed by the current spec and will require new normative work regardless.

**Edge case 13 (Intake handoff — attach-to-existing):** ADR 0073 D-4 already defines `workflowInitiated` mode where "`caseRef` is already known when intake starts or before handoff acceptance." The `IntakeAccepted` provenance record in `wos-provenance-log.schema.json` carries `caseDisposition` which can be `attachToExistingCase` (per the existing `wos-runtime` implementation). This edge case is already partially settled; the proposal should reference ADR 0073 directly.

### 8. Overall Verdict

This proposal is **(b) — structurally sound but requires renaming existing normative terms and would force cross-stack ADRs** — with a partial **(c)** caveat for several edge cases.

**What is structurally sound:** The core insight — that `CaseInstance` is a workflow runtime artifact and is architecturally wrong as the root product domain object for case management — is correct and the kernel spec confirms it. The `wos-case-instance.schema.json` title says "A WOS CaseInstance document... a running workflow instance." The conflation risk is real. Separating Case (domain aggregate) from CaseProcess (runtime instance) is a structurally correct direction.

**What requires normative surgery:**

1. ADR 0073 D-1 ("WOS owns governed case identity and `case.created`") must be amended to answer "who emits `case.created` for a Case created with zero CaseProcesses." The current normative answer is: WOS does, always. The proposal silently overrides this.

2. The `$wosCaseInstance` marker rename is not just a Rust type alias. It is a JSON envelope discriminant used by lint, conformance, and the runtime. Every conformance fixture referencing it must be updated and the `discover_and_report_promotion_candidates` CI gate will need adjustment.

3. The TypeID family prefix `_case_` for instance IDs must be changed if the instance object is renamed to `CaseProcess`, or left as `_case_` with documented ambiguity. The ADR must make this call explicitly.

4. Kernel §5 ("Case State") will need to be extended to distinguish process-scoped runtime data from durable case-domain data. This is a kernel spec change.

5. The "governed output path" seam does not exist and cannot be added without amending ADR 0077 (six-seam invariant).

**What is partially redundant (the (c) component):** The kernel already has `caseRelationships` (§5.5), relationship-triggered events (§14), `correlationKey` (§9.4), `amendmentTaxonomy` in governance, migration semantics (§9.6), and the intake handoff contract (ADR 0073 D-4). The proposal re-invents some of this under different names rather than building on it. The CaseRelationshipKind vocabulary should be an extension of kernel §5.5's closed enum plus `x-` kinds. The CaseDecision concept should be mapped against `amendmentTaxonomy` + `determination`-tagged transitions.

**The Trellis collision is the most underestimated risk.** The proposal is being written at a moment when Trellis Phase 3 (`case-ledger-spec.md`) has not yet shipped. When it does, "Case Ledger" becomes the canonical cryptographic integrity scope for case events. The product-level `Case` aggregate and the Trellis `Case Ledger` must have explicitly separated names, IDs, and spec sections, or every future cross-layer discussion will be ambiguous. The ADR should name this collision and close it before any code ships.

### Key file references for the ADR author

- `thoughts/adr/0073-stack-case-initiation-and-intake-handoff.md` — D-1, D-4, D-7 ownership pins
- `work-spec/schemas/wos-case-instance.schema.json` — `$wosCaseInstance` marker, `status` enum, `instanceId` TypeID pattern, `caseState` definition
- `work-spec/schemas/wos-provenance-log.schema.json` — `CaseCreatedRecord` (`event: "case.created"` constant), `IntakeAcceptedRecord`
- `work-spec/specs/kernel/spec.md` §5 ("Case State") — the term "caseState" is workflow-instance scoped; §5.5 (`caseRelationships`) — existing related-case vocabulary; §8.2.3 — `caseCreated` / `intakeAccepted` record kind definitions
- `work-spec/specs/advanced/advanced-governance.md` §4 — constraint zones as process-internal adaptive work overlay, not a case-aggregate concept
- `trellis/specs/trellis-core.md` §1.2 — "Case Ledger" as Phase 3 Trellis scope name; currently unrealized but reserved
- `work-spec/schemas/wos-workflow.schema.json` — `governance.amendmentTaxonomy` examples; `$defs/InstanceStatus`; `bindings` top-level property; no `caseOutputHook` seam exists

---

## Appendix B — cross-stack-scout report (full)

**Stance:** structurally sound; **misframed at the storage layer** in a way that — uncorrected — silently demands a new authoritative aggregate Trellis explicitly does not admit.

### 1. Trellis Case Ledger seam — projection, not aggregate

The proposal's `Case` shape ([`case-management.md:83-102`](case-management.md)) reads like an aggregate root. Under the wos-server end-state ([`workspec-server/crates/wos-server/VISION.md:96-105`](../../../workspec-server/crates/wos-server/VISION.md)), there is no place to land it as such:

- `canonical` schema = Trellis-shaped events. Hash-chained, COSE-signed, payloads encrypted per access class. Append-only. **The case ledger is exactly this stream scoped to one case.**
- `projections` schema = derived metadata, mutable, rebuildable from events by replay, **plaintext-content-free**.

[`trellis-core.md:2391`](../../../trellis/specs/trellis-core.md) is normative: *"The case ledger is defined as a Trellis-shaped hash-chained sequence of events whose admitted facts are (a) sealed response-ledger heads plus (b) WOS governance events."* The case ledger event format **IS** the §6 event format. There is no third type of fact admissible into the chain.

Under that constraint, the proposed `Case` aggregate has exactly one valid home: **the projections schema, as a derived view rebuildable from the case-scoped Trellis event stream.** Every field on the `Case` body — `subjects`, `participants`, `notes`, `decisions`, `artifacts`, `timeline` — is content. None of it lives in the canonical chain except as event payloads.

The proposal does not name this. It speaks of "Case as a durable domain aggregate" and lists "introduce `Case` as a first-class durable domain aggregate" ([`case-management.md:198`](case-management.md)) — language that on its face suggests a new authoritative store. **It must be reframed as a projection-class abstraction** for Trellis Core §22.4 to accept it.

The good news: the proposal's "Case MAY exist with zero processes" invariant ([`case-management.md:428`](case-management.md)) is fine *as a projection*, provided the canonical chain admits a `wos.case-created` (or equivalent registered `wos.*`) event — which today is implied by [`api/dashboard.md:61`](../../specs/api/dashboard.md) `case-created` literal but is **not yet registered** in the bound registry per [`trellis-core.md:2436-2442`](../../../trellis/specs/trellis-core.md). The proposal does not flag this gap.

### 2. ADR-0074 per-class encryption — the proposed Case body cannot be served plaintext

This is the **largest unflagged hole** in the proposal. The proposed `Case` carries `notes`, `communications`, `decisions`, `artifacts` as flat fields ([`case-management.md:91-95`](case-management.md)) on a route response (`GET /api/v1/cases/{caseId}`).

Under [`workspec-server/crates/wos-server/VISION.md:103`](../../../workspec-server/crates/wos-server/VISION.md): *"clients decrypt; servers broker."* The server returns ciphertext plus wrapped key-bag entries; the browser unwraps DEKs via WebAuthn PRF or hardware token. The server **structurally cannot** populate plaintext `notes` / `communications` / `decisions` for a routine read — that route would violate the SBA-and-stricter posture commitment.

What the proposal actually needs (and does not say):

- `Case` projection carries **metadata only**: IDs, status, counts, timestamps, opaque content-class refs, key-bag-entry references.
- `notes`, `communications`, `decisions`, `artifacts` are subresources whose *bodies* are ciphertext events fetched separately (`/api/v1/cases/{caseId}/artifacts/{id}` → returns wrapped event + key-bag fragment), decrypted client-side per the access class the requesting identity holds.
- The proposal's "summary" hint ([`case-management.md:514-516`](case-management.md) — `artifactsSummary?`, `decisionsSummary?`) gestures toward this but does not commit. It MUST commit, or the API design contradicts the deployment commitment.

`Case.subjects` and `Case.participants` are themselves content (PII) and need access-class classification per ADR-0074 — likely `respondent.identity` or `staff.identity` classes. The proposal treats them as schema-level fields, not as classified content. **This is the proposal's most concrete architectural error.**

### 3. IntakeHandoff and `case.created` ownership (ADR-0073)

[`thoughts/adr/0073-…:25-27`](../../../thoughts/adr/0073-stack-case-initiation-and-intake-handoff.md): *"WOS is the only layer that emits the governed case boundary event ... Formspec MUST NOT emit `case.created`."* The proposal's edge case 1 ("staff manually creates case, no workflow active yet") and edge case 14 ("deferred intake may create artifact without CaseProcess") ([`case-management.md:622-682`](case-management.md)) collide with this directly.

Three options; the proposal does not pick one:

**(a) Third creator** — a new "case-management" service emits `case.created`. **Rejected** by ADR-0073 D-1 unless WOS is reframed to *include* the case-management layer.

**(b) WOS emits `case.created` without a bound WorkflowDocument.** This requires an ADR-0073 amendment: today every emission path in [ADR-0073 §D-2](../../../thoughts/adr/0073-stack-case-initiation-and-intake-handoff.md) (lines 45-58) terminates in WOS validating-then-emitting in the context of an intake handoff. A "manual case creation" path emits `case.created` with no `IntakeHandoff` and no `WorkflowDocument`. What enforces tenant + impactLevel + class taxonomy? The proposal punts; ADR amendment must answer.

**(c) The "Case" projection materializes from a non-canonical signal** — a record outside the Trellis chain. **Rejected** by [VISION.md:139](../../../workspec-server/crates/wos-server/VISION.md): "verifier independence is structural; canonical schema is read-only at the deployment role level." A Case visible in `/api/v1/cases` but absent from the chain breaks the stranger-test posture.

**Verdict:** option (b) is the only architecturally consistent path. The proposal needs an explicit invariant — *"every Case origination MUST emit a `wos.case-created` event into the canonical case ledger before the Case projection materializes; no non-workflow path bypasses this"* — and an ADR-0073 amendment naming the no-handoff manual creation flow.

### 4. wos-server EventStore — the routes already presume a missing aggregate

Today's wos-server (actual code, not VISION end-state): [`workspec-server/crates/wos-server/src/http/instances.rs`](../../../workspec-server/crates/wos-server/src/http/instances.rs) is the one running route family; case content is fetched per-instance. There is **no `cases` aggregate route, no `case_projection` table, no `Case`-shaped query path.** The crate set ([`workspec-server/crates/`](../../../workspec-server/crates/)) shows `wos-server-postgres`, not the VISION's `eventstore-postgres` — current state is pre-zero-trust.

The proposal's seven `/api/v1/cases/*` routes ([`case-management.md:147-156`](case-management.md)) silently demand:

1. A `case_projection` table in the projections schema.
2. A reducer/replay function from the case-scoped event stream into that projection.
3. A query layer that joins projection metadata with key-bag-fragment release per requesting identity per access class.
4. Provenance that `GET /cases/{id}/artifacts` returns ciphertext-plus-wrapped-DEKs, not plaintext.

None of this exists today. The proposal's Phase F ([`case-management.md:366-374`](case-management.md)) describes the *behavior* but does not enumerate the *machinery*. Either the proposal must defer routes until the EventStore zero-trust refactor lands, or it must include explicit dependency on it.

### 5. `$wosCaseInstance` marker rename — blast radius is tractable but real

Grepping `$wosCaseInstance` across `work-spec/`: **14 hits** across schema, lint, tests, plans, ADRs, and one governance spec ([`workflow-governance.md:791`](../../specs/governance/workflow-governance.md)). Plus runtime artifact identity discipline at [ADR-0063](../adr/0063-embedded-vs-sidecar-identity-boundary.md) (recognizes only six author-time + two runtime markers).

True blast radius if renamed to `$wosProcessInstance`:

- Schema: 1 file rename + `$id` change + `const` value.
- Lint: `crates/wos-lint/src/document.rs:88` (DocumentKind dispatch table), `crates/wos-lint/tests/tier2_rules.rs:109`.
- ADR-0063 itself: textual update naming the recognized markers.
- Conformance fixtures: any fixture carrying the marker (low — runtime artifacts are not heavily fixtured).
- wos-server adapters: internal Rust types use `CaseInstance` symbol but that's separable from JSON marker.
- Trellis export bundles: the marker doesn't appear in Trellis envelopes — Trellis sees governance events, not case-instance JSON.
- Studio (policy-studio) authoring: zero — Studio authors Workflow Documents, not runtime artifacts.
- case-portal generated SDK: the response schema's `$wosCaseInstance` field flows to the generated TS type; rename = generated-type rename.

**Total: ~20-30 files, all in `work-spec/` + `case-portal/` generated-SDK refresh.** The proposal's "keep marker temporarily, document as legacy" stance ([`case-management.md:920-922`](case-management.md)) is can-kicking. Per parent [`VISION.md`](../../../VISION.md) "no backwards compatibility / nothing is released," there is no legitimate reason to leave a misnamed marker. **Rename in one ADR; it's a 30-minute rebuild, not a stable rest position.**

### 6. WOS↔governed output binding — yes, this requires a kernel-seam clarification

Today's `outputBinding` ([`kernel/spec.md:1186-1218`](../../specs/kernel/spec.md)) targets *the workflow's own case state*. JSONPath into a service response, written into the workflow instance's `caseState`. There is no current concept of "write a CaseArtifact attached to a Case different from this workflow's instance." The proposal's `CaseStateMutation` / `CaseArtifact creation` / `CaseDecision creation` / `Timeline append` ([`case-management.md:286-291`](case-management.md)) is a **new output-binding kind**, not an existing one.

Where it lands in the six-seam invariant ([`kernel/spec.md:19`](../../specs/kernel/spec.md): `actorExtension`, `contractHook`, `provenanceLayer`, `lifecycleHook`, `custodyHook`, `extensions`):

- Not `actorExtension` (about who acts).
- Not `contractHook` (about validation gates).
- Not `lifecycleHook` (about transition-time effects on this instance).
- Not `custodyHook` (about Trellis envelope binding).
- Not `provenanceLayer` (about audit-record shape).
- **`extensions` / `x-` keys** — would house it as vendor extension, but cross-aggregate writes are not vendor-specific.

This argues for **either** (a) clarifying that `outputBinding` already supports cross-aggregate writes when the target is the parent Case (a §9.2.21 amendment), **or** (b) adding a normative seventh seam, which violates ADR-0077's six-seam closure. The proposal needs to say which. Recommendation: extend `outputBinding` in §9.2.21 with a `target` discriminator distinguishing `processCaseState` (today's behavior) from `caseArtifact` / `caseDecision` / `caseTimeline` (new). No new seam needed.

### 7. Adaptive/ad hoc work — provenance must still anchor

[`case-management.md:716-719`](case-management.md): *"a caseworker note should not require a workflow transition."* Under zero-trust, the note still hits the canonical chain — every event is hash-chained and COSE-signed. So:

- **Who signs?** A staff actor profile; signing key is the staff identity's COSE key. WOS already declares actors with `actorExtension`; ad-hoc events use the same actor identities, just outside any WorkflowDocument's transitions.
- **What `event_type`?** A registered `wos.note` or `wos.communication` (not yet registered per Trellis §23.4 outcome-neutrality).
- **Where's the `wos.governance` source-of-truth?** This is the load-bearing concern. WOS today emits governance events from *transitions*. Ad-hoc events have no transition. Either (a) WOS spec admits non-transition governance events (a real spec change), or (b) an ad-hoc note path is defined that emits a `wos.note` event without a transition record.

The proposal does not name (a) vs (b). **Both require WOS spec edits.** Custody anchoring (`custodyHook`) is per-event in [`custody-hook-encoding.md`](../../specs/kernel/custody-hook-encoding.md), so anchoring works either way; what's missing is the WOS authoring path that emits the event in the first place.

### 8. Tenant + class consistency — invariant is fine; federation is out of scope

[`case-management.md:439`](case-management.md): "Case tenant MUST match attached CaseProcess tenant." Today's `wos-case-instance.schema.json:53-66` enforces tenant in the TypeID prefix. Postgres-per-tenant + OpenFGA enforces at storage and authz layers. Case-vs-process tenancy is a derived invariant — a Case projection in tenant A's database cannot reference a Process in tenant B's database, since they are physically separate. The proposal's invariant is a check on the projection reducer, not new infrastructure.

Cross-tenant federation is a Trellis concern ([`trellis-core.md` Phase-3 federation log](../../../trellis/specs/trellis-core.md)) and explicitly out of scope for SBA Q1. Defer.

### Aggregate verdict — the root domino

The missing primitive is **NOT** a "Case aggregate." It is:

> **A case-scoped projection, materialized from the canonical Trellis case ledger by replay, exposed as the product API root, with no plaintext content in the projection itself — only opaque per-class refs, key-bag-fragment release flowing through the normal client-decrypt path.**

The proposal correctly identifies the boundary problem (`Case ≠ CaseInstance`, multiple processes, ad hoc work) and correctly insists on governed output paths. It is **wrong at one structural premise**: it talks as though `Case` is a new authoritative store, when under [`workspec-server/crates/wos-server/VISION.md`](../../../workspec-server/crates/wos-server/VISION.md) the only authoritative store is the Trellis case ledger and the only co-located store is plaintext-content-free projections.

The dominoes that fall, in dependency order:

1. **Trellis** (low-risk, mostly registry work) — register `wos.case-created`, `wos.case-closed`, `wos.case-reopened`, `wos.note`, `wos.communication`, `wos.artifact-attached` in the bound registry per [`trellis-core.md:2436-2442`](../../../trellis/specs/trellis-core.md). This is registry binding, not byte change.
2. **WOS kernel** — extend `outputBinding` §9.2.21 with `target` discriminator (`processCaseState` | `caseArtifact` | `caseDecision` | `caseTimeline`); admit non-transition governance event paths for ad-hoc notes/communications; rename `$wosCaseInstance` → `$wosProcessInstance` in one go.
3. **WOS governed-case ownership** — amend ADR-0073 to admit manual case origination (no `IntakeHandoff`); enumerate the validation discipline.
4. **wos-server EventStore** — define `case_projection` reducer and `/api/v1/cases/*` route shape, all metadata-only; commit to ciphertext-plus-key-bag-fragments for content subresources. **Cannot land before the zero-trust EventStore refactor.**
5. **Formspec** — minimal change; `IntakeHandoff` continues to operate per ADR-0073 with the addition that an intake-handoff path may attach to an existing Case.
6. **`$wosCaseInstance` rename** — execute as one ADR.

**The proposal as written is approximately 70% correct.** Adopt the boundary refactor, but rewrite the `Case` aggregate spec as a *projection spec*, classify the content fields (`subjects` / `participants` / `notes` / etc.) under ADR-0074 access classes, and make the `target` discriminator on `outputBinding` the named structural change at the WOS layer instead of a vague "governed output path."

**Single-line recommendation to owner:** before writing the proposed ADR, write a **half-page reframing memo** under `thoughts/specs/` titled *"Case is a projection, not an aggregate"* — and circulate it with the cross-stack-scout's verdict attached. If the owner agrees with the reframe, the ADR work proceeds with the right structural premise; if not, the ADR risks ratifying a parallel store that contradicts the zero-trust commitment.
