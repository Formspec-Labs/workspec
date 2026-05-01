# Studio Spec: Workflow Intent

**Status:** draft (Stage 2 of [Implementation Roadmap](../VISION.md#17-implementation-roadmap))
**Date:** 2026-04-30
**Concept-model anchor:** [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.12 WorkflowIntent, §2.4 WorkflowIntent lifecycle.
**PRD anchor:** [`../VISION.md`](../VISION.md) §9.4 (Workflow Builder).
**Depends on:** [`policy-object-model.md`](policy-object-model.md), [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md).

## Scope

WorkflowIntent is the **user-facing draft** of the workflow — phases, steps, decisions, deadlines, notices, appeals, exceptions, holds, data-collection, evidence-requests, system-checks, AI-assistance, manual-overrides, completion-outcomes — that compiles to a `$wosWorkflow` file. Until now it has been referenced in every other spec but defined only in `CONCEPT-MODEL.md §1.12`. This spec is its home.

This spec defines:

- the WorkflowIntent envelope and lifecycle;
- the **16 user-facing element kinds** (PRD §9.4) and their body shapes;
- the **bridge from each element kind to kernel constructs** (state, transition, timer, task, guard) so the compiler ([`compiler-contract.md`](compiler-contract.md)) has a deterministic mapping;
- the relationship between WorkflowIntent and the policy objects, mappings, scenarios, validation findings, and bindings that anchor it;
- composition with the upstream and downstream specs.

The phrase "WorkflowIntent" (not "Workflow") is deliberate: this is the *Studio-side authoring model*. The compiled `$wosWorkflow` is the technical-side artifact. They are distinct artifacts with different audiences, different lifecycles, and different review surfaces.

## Out of scope

- The Workflow Builder UX (PRD §9.4 capability).
- The compiler implementation (Stage 5; spec is in [`compiler-contract.md`](compiler-contract.md)).
- WOS lifecycle semantics — those live in [`../../specs/kernel/spec.md`](../../specs/kernel/spec.md). This spec defines how Studio's user-facing concepts MAP to those semantics, not the semantics themselves.

## Terminology

- **WorkflowIntent** — the Studio-side authoring artifact.
- **Element** — a single user-facing unit in a WorkflowIntent (phase, step, decision, …).
- **Element kind** — one of the 16 categories below.
- **Bridge** — the mapping from an element to a kernel construct (state / transition / timer / task / guard).
- **Compiled artifact** — the `$wosWorkflow` document produced by the Studio→WOS compiler.
- **WorkflowIntent vs. workflow** — when the spec says "workflow," it means the user-facing concept; when it says "compiled artifact" or `$wosWorkflow`, it means the WOS document.

## Data model

### `WorkflowIntent` (CM §1.12, extended)

```text
WorkflowIntent {
  id, workspaceId, title, description?,
  impactLevel,                 // mirrors $wosWorkflow.impactLevel: rights-impacting | safety-impacting | informational | other
  lifecycleState,              // draft | mapped | validationReady | scenarioTested | approved | published | deprecated
  wosVersionPin,               // claim string per parent RELEASE-STREAMS.md (e.g., "kernel@1.0, governance@1.0, ai@0.5, signature@1.0, custody@1.0, advanced@0.3, assurance@1.0"); per CM §1.33 MigrationPath
  effectivenessRef?,           // workspace-level Effectiveness per effectiveness-and-applicability.md §1.25; the workflow's overall jurisdictional + temporal scope
  elements[],                  // ordered or graph-structured set of WorkflowElements
  policyObjectRefs[],          // PolicyObjects this intent is derived from
  mappingRefs[],               // StudioToWosMappings active in this intent
  bindingRefs[],               // Bindings (Service / Event / PolicyEngine / DecisionTable) referenced
  compiledArtifactRef?,        // pointer to last compiled $wosWorkflow (when validated or later)
  expertModeView?,             // mapping from intent elements to compiled $wosWorkflow JSON paths
  version,                     // monotonic per WorkflowIntent
  parentVersion?,
  provenance,                  // AuthoringProvenanceRecord references
  createdBy, createdAt, lastEditedBy, lastEditedAt
}
```

**`wosVersionPin`** is load-bearing for reproducibility (per `compiler-contract.md` `SA-MUST-cmp-050`) AND for migration (per `change-impact.md` `triggerKind = wos-version-deprecation`). When the parent stack ratifies a new minor stream version, existing WorkflowIntents continue to compile against their pinned version; the pin is updated only via explicit reviewer migration action.

**`effectivenessRef`** at the workflow level declares the workflow's overall jurisdictional + temporal scope. Individual elements MAY narrow further via element-level effectiveness on bridges (see WorkflowElement extension below).

### `WorkflowElement`

Every element has the common envelope:

```text
WorkflowElement {
  id,                          // unique within the WorkflowIntent
  kind,                        // one of the 16 user-facing kinds (see below)
  name,                        // reviewer-readable label (e.g., "Verify household income")
  body,                        // kind-specific structure
  position,                    // structural placement (phase id, sequence index, parent step ref)
  policyObjectRefs[],          // approved PolicyObjects backing this element
  citations[],                 // SourceCitation refs (typically inherited from policyObjectRefs)
  bridge,                      // BridgeAssertion: how this element compiles to kernel constructs
  effectivenessRef?,           // optional element-level Effectiveness narrowing the workflow-level effectiveness; per effectiveness-and-applicability.md
  reviewState,                 // draft | reviewed | approved
  workspaceId
}
```

The `bridge` field is load-bearing — it's how the compiler knows what to emit. It is NOT free-form: it is one of a closed set of forms, one per element kind (defined below).

**`effectivenessRef` on an element** narrows the workflow-level effectiveness for cases matching the element. Example: a NoticeRequirement element narrowing to `{jurisdictions: [{kind: "state", code: "US-TX"}]}` means the notice fires only for Texas cases. Compilation translates this to a derived FEL `appliesWhen` on the corresponding `governance.notices[*]` (per `effectiveness-and-applicability.md` §"WOS mappings"). Elements without `effectivenessRef` inherit the workflow's effectiveness.

### The 16 user-facing element kinds

Each kind has a body shape and a bridge. Listed in approximate authoring order (top-down), not strict precedence.

#### 1. `phase`

A coarse grouping of steps (intake / decision / appeal / closure).

- **Body:** `{description, contains: WorkflowElement.id[]}`.
- **Bridge:** kernel **compound state** containing the inner elements' states.

#### 2. `step`

A unit of work performed by an actor (or a system) inside a phase.

- **Body:** `{description, performedBy: ActorMappingRef, expectedDurationHint?, blockingOrInformational}`.
- **Bridge:** kernel **atomic state**, or a state with a single transition action.

#### 3. `decision`

A point where a determination is made.

- **Body:** `{question, decisionRuleRef: DecisionRule|DecisionTable PolicyObject ref, possibleOutcomes: OutcomeRef[]}`.
- **Bridge:** kernel **transition with FEL guard** (when DecisionRule is single-rule), or **chained guards** (when DecisionTable). Cross-references [`binding-and-integration.md`](binding-and-integration.md).

#### 4. `review`

A human review of one or more case-file fields, decisions, or evidence items.

- **Body:** `{reviewer: ActorMappingRef, scope: caseFilePath[]|decisionRef[], outcome: ReviewerResolution-shape}`.
- **Bridge:** kernel **task** (per `WS-HumanTask` adoption in `README.md` §110).

#### 5. `notice`

A communication to the applicant — typically pre-, alongside-, or post-decision.

- **Body:** `{noticeRequirementRef: NoticeRequirement PolicyObject ref, deliveryBindingRef?: ServiceBinding ref (when delivered via API)}`.
- **Bridge:** governance-block entry in compiled `$wosWorkflow.governance.notices[*]`; optionally a ServiceBinding emission per [`binding-and-integration.md`](binding-and-integration.md).

#### 6. `deadline`

A temporal commitment — to applicant or agency.

- **Body:** `{deadlinePolicyObjectRef: Deadline PolicyObject ref, attachedTo: WorkflowElement.id, expiryAction: notify | escalate | auto-decide | terminate}`.
- **Bridge:** kernel **timer** (via TimerMapping); compiled to `$wosWorkflow.lifecycle.timers`.

#### 7. `appeal`

The branch invoked when an applicant exercises an AppealRight.

- **Body:** `{appealRightRef: AppealRight PolicyObject ref, branchEntryPoint: WorkflowElement.id, deadlineRef: Deadline PolicyObject ref}`.
- **Bridge:** kernel **separate sub-flow** (compound state) entered via a transition triggered by `appealFiled` event.

#### 8. `exception`

A path that diverges from the default flow under an ExceptionRule.

- **Body:** `{exceptionRuleRef: ExceptionRule PolicyObject ref, divertsFrom: WorkflowElement.id, divertsTo: WorkflowElement.id, conditionRef?: Condition PolicyObject ref}`.
- **Bridge:** kernel **alternate transition** with guard from the ExceptionRule's condition.

#### 9. `hold`

A pause in workflow progress pending an external condition.

- **Body:** `{holdReason, releaseCondition: Condition|EventRef, maxDuration?, escalation?}`.
- **Bridge:** kernel **state with no automatic transitions** until the release condition fires.

#### 10. `data-collection`

A point at which case-file fields are populated (typically from applicant input).

- **Body:** `{collected: DataElement ref[], formRef?: Formspec form id, requiredFields, optionalFields}`.
- **Bridge:** Formspec coprocessor invocation per [`../../crates/wos-formspec-binding`](../../crates/wos-formspec-binding) and ADR-0073; case-file paths populated per CaseFileMapping.

#### 11. `evidence-request`

A point at which an EvidenceRequirement is triggered (request to applicant for documentation).

- **Body:** `{evidenceRequirementRef: EvidenceRequirement PolicyObject ref, deadlineRef?: Deadline PolicyObject ref}`.
- **Bridge:** kernel task + caseFile evidence slot + optional deadline timer.

#### 12. `system-check`

A read from an external system (federal data broker, identity service, match service).

- **Body:** `{checkPurpose, serviceBindingRef: ServiceBinding ref}`.
- **Bridge:** ServiceBinding invocation per [`binding-and-integration.md`](binding-and-integration.md); compiled to `$wosWorkflow.integration.bindings[*]` of type `openapi-call` or `arazzo-step`.

#### 13. `AI-assistance`

A point at which an agent is invoked — for triage, recommendation, classification, or extraction.

- **Body:** `{aiUsePolicyObjectRef: AI Use PolicyObject ref, actorMappingRef: ActorMapping (with actorKind = agent), confidenceFloor, fallbackRef: WorkflowElement.id}`.
- **Bridge:** agent declaration in compiled `$wosWorkflow.agents[*]`; fallback chain wired to the fallback element via transition.

#### 14. `manual-override`

A path explicitly available to a staff actor with override authority.

- **Body:** `{overridingActor: ActorMapping ref, defaultPath: WorkflowElement.id, overridePath: WorkflowElement.id, justificationRequired: true | false, recordRationale: true}`.
- **Bridge:** kernel transition with guard `actorRole.hasOverrideAuthority`; emits `manualOverride` provenance event.

#### 15. `completion-outcome`

A terminal state of the workflow (favorable, adverse, or neutral).

- **Body:** `{outcomeRef: Outcome PolicyObject ref, postClosureActions[]}`.
- **Bridge:** kernel terminal state (`$wosWorkflow.lifecycle.states[?(@.terminal)]`); polarity from the Outcome PolicyObject; due-process linkage (Notice + Appeal) per `SA-MUST-pom-030`.

#### 16. `phase-end`

A marker that closes a phase and transitions to the next.

- **Body:** `{phaseRef, nextPhaseRef? | terminalRef?}`.
- **Bridge:** kernel transition closing one compound state and entering the next.

## Lifecycle

The WorkflowIntent lifecycle (CM §2.4):

```text
draft → mapped → validationReady → scenarioTested → approved → published → deprecated
```

Allowed transitions:

| From | To | Trigger |
|---|---|---|
| `draft` | `mapped` | every required element has a Mapping (per [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md) `SA-MUST-map-001`) and tier-S3 readiness passes |
| `mapped` | `validationReady` | tier-S1, S2, S3, S4 readiness pass (or are explicitly waived) |
| `validationReady` | `scenarioTested` | every critical-path scenario simulated and tier-S5 readiness passes |
| `scenarioTested` | `approved` | required reviewers (per workspace policy) signed off and tier-S6 readiness passes |
| `approved` | `published` | publication gate passes (per [`review-and-approval.md`](review-and-approval.md) `SA-MUST-ra-040`) |
| `published` | `deprecated` | superseded by a later version |
| any | `draft` (demotion) | a referenced PolicyObject was demoted (per [`policy-object-model.md`](policy-object-model.md) `SA-MUST-pom-022`); cascade per [`change-impact.md`](change-impact.md) |

`deprecated` is terminal; the WorkflowIntent remains queryable but no further authoring operations advance its lifecycle.

## Normative Contract

### Element integrity

- **`SA-MUST-wfi-001`** — Every WorkflowElement MUST carry `kind` (one of the 16) and `bridge` (the kind-specific bridge form). Elements with unknown kind or missing bridge MUST be rejected at creation. *(schema-pending: discriminated `oneOf`.)*
- **`SA-MUST-wfi-002`** — Every WorkflowElement MUST carry at least one `policyObjectRefs[]` entry — backing the element with at least one approved PolicyObject. The exception: `phase` and `phase-end` elements which are structural-only. Elements without policy backing MUST be flagged as tier-S4 ValidationFindings (`WF-LINT-008`). *(lint-pending: cross-cutting with [`authoring-provenance.md`](authoring-provenance.md) `SA-MUST-prov-020`.)*
- **`SA-MUST-wfi-003`** — Element `id`s MUST be unique within a WorkflowIntent. *(schema-pending.)*
- **`SA-MUST-wfi-004`** — `position` references (e.g., `phase.contains[*]`, `exception.divertsFrom`, `manual-override.defaultPath`) MUST resolve to existing elements within the same WorkflowIntent. Dangling references MUST be rejected. *(schema-pending; runtime-pending.)*

### Element-kind rules

- **`SA-MUST-wfi-010`** — A `decision` element MUST carry a `decisionRuleRef`. The referenced PolicyObject MUST be `kind = DecisionRule` (single-rule) or `kind = DecisionRule, body.form = "table"` (DecisionTable). *(schema-pending; runtime-pending.)*
- **`SA-MUST-wfi-011`** — A `notice` element MUST carry a `noticeRequirementRef` whose referenced NoticeRequirement is approved and mapped. *(lint-pending: tier-S4 cross-cutting.)*
- **`SA-MUST-wfi-012`** — An `appeal` element MUST carry an `appealRightRef` whose referenced AppealRight is linked to the same Outcome as the corresponding `notice` element (when the appeal flows from a notice). *(lint-pending: tier-S4.)*
- **`SA-MUST-wfi-013`** — A `system-check` element MUST carry a `serviceBindingRef`. *(lint-pending: tier-S4 cross-cutting `WF-LINT-007`.)*
- **`SA-MUST-wfi-014`** — An `AI-assistance` element MUST carry a `fallbackRef` to another element (typically a `review` or `manual-override`); fallback chains terminating without a human reviewer step MUST be rejected per parent CLAUDE.md ("fallback chain terminating in human review"). *(lint-pending: tier-S4 cross-cutting `SA-MUST-pom-036`.)*
- **`SA-MUST-wfi-015`** — A `completion-outcome` element whose `outcomeRef.polarity = adverse` AND `triggersDueProcess = true` MUST be reachable from at least one `notice` element AND at least one `appeal` element. (Cross-cutting [`policy-object-model.md`](policy-object-model.md) `SA-MUST-pom-030`.) *(lint-pending: tier-S4 `WF-LINT-001`.)*
- **`SA-MUST-wfi-016`** — A `deadline` element's `expiryAction` MUST be one of `notify | escalate | auto-decide | terminate`. *(schema-pending: enum.)*
- **`SA-MUST-wfi-017`** — A `manual-override` element MUST identify an overridingActor whose ActorMapping has authority (per the deontic kinds). Overrides without a documented authority basis MUST be flagged as tier-S4 ValidationFindings. *(lint-pending.)*

### Lifecycle integrity

- **`SA-MUST-wfi-020`** — A WorkflowIntent MUST NOT advance to `mapped` until every non-structural element has at least one Mapping (per `studio-to-wos-mapping.md`). *(lint-pending: tier-S3 readiness rule.)*
- **`SA-MUST-wfi-021`** — A WorkflowIntent MUST NOT advance to `validationReady` until tier-S1, S2, S3, S4 readiness rules on its referenced subjects all pass or are waived. *(lint-pending: tier-S5 cross-cutting `SA-MUST-rv-040`.)*
- **`SA-MUST-wfi-022`** — A WorkflowIntent MUST NOT advance to `scenarioTested` until tier-S5 readiness rules pass (every adverse Outcome has a Scenario; failing scenarios are accepted-as-known-gap or resolved). *(lint-pending: tier-S6 cross-cutting `SA-MUST-rv-041`.)*
- **`SA-MUST-wfi-023`** — A WorkflowIntent MUST NOT advance to `approved` until tier-S6 readiness rules pass and required-role ApprovalDecisions are recorded. *(lint-pending: cross-cutting `SA-MUST-ra-040`.)*
- **`SA-MUST-wfi-024`** — A WorkflowIntent MUST NOT advance to `published` until the publication gate passes (`SA-MUST-ra-040` + `SA-MUST-ra-042`). *(runtime-pending.)*

### Demotion cascade

- **`SA-MUST-wfi-030`** — When a referenced PolicyObject is demoted (per `SA-MUST-pom-022`), every WorkflowElement listing it in `policyObjectRefs[]` MUST be marked as needing re-review. The WorkflowIntent's `lifecycleState` is demoted accordingly: from `approved` or `validationReady` back to `draft` (the most-conservative demotion target). *(runtime-pending.)*
- **`SA-MUST-wfi-031`** — Demotion MUST emit AuthoringProvenanceRecords with `eventKind = demoted` (cross-cutting `SA-MUST-prov-001`). *(runtime-pending.)*

### Bridge contract

- **`SA-MUST-wfi-040`** — Every element's `bridge` MUST be a closed-form structure matching its kind. The compiler ([`compiler-contract.md`](compiler-contract.md)) MUST refuse to compile elements with non-conforming bridges. *(schema-pending; runtime-pending.)*
- **`SA-MUST-wfi-041`** — Compilation MUST be **deterministic**: identical WorkflowIntent + identical referenced PolicyObjects + identical Mappings ⇒ identical compiled `$wosWorkflow`. Non-deterministic bridges (e.g., elements that reference other elements in unstable order) MUST be rejected. *(fixture-pending.)*

## Composition

### Attachment point

A WorkflowIntent attaches to a Workspace and references the Workspace's PolicyObjects, Mappings, Scenarios, and Bindings. It is the **integration point** for the entire spec set — every other Studio spec contributes to the WorkflowIntent's compilation in some way.

### Precedence

When a single workflow concept could be modeled as more than one element kind (a borderline "deadline triggered by an event" could be modeled as `deadline` + `notice` or as `event-triggered deadline`), reviewer judgment governs. Studio does not auto-pick. The companion PRD §4 AI flow recommends; humans approve.

When a PolicyEngineBinding's deny conflicts with a WorkflowIntent's default-flow expectation, the deny wins (see [`binding-and-integration.md`](binding-and-integration.md) §"Composition"). The WorkflowIntent MUST have a corresponding fallback path.

### Conflict handling

Two WorkflowElements with identical bridges (same kind, same `bridge` content) within the same WorkflowIntent are tier-S4 collisions (`WF-LINT-012`). Reviewer must consolidate.

### Versioning / migration

- Adding new element kinds to the canonical 16: schema-breaking; coordinated with the kernel spec.
- Renaming an element kind: schema-breaking.
- Adding optional fields to an element kind's body: non-breaking.
- Changing a kind's bridge mapping (e.g., `step` no longer maps to atomic state): schema-breaking; requires re-compilation of every WorkflowIntent.

## Conformance

### Schema validation (Stage 3)

- WorkflowIntent required fields and lifecycle enum.
- WorkflowElement common envelope + per-kind body discriminator.
- Bridge structure per kind.
- Element-id uniqueness within a WorkflowIntent.

### Lint rules (Stage 4)

Tier-S4 (Workflow readiness) rules planned:

- `WF-LINT-001` — adverse outcomes link Notice + Appeal (cross-cutting `SA-MUST-wfi-015`).
- `WF-LINT-007` — system-check elements have a ServiceBinding (cross-cutting `SA-MUST-wfi-013`).
- `WF-LINT-008` — every element has policy backing (`SA-MUST-wfi-002`).
- `WF-LINT-011` — workflow transitions reference defined events (cross-cutting [`binding-and-integration.md`](binding-and-integration.md) `SA-MUST-bind-023`).
- `WF-LINT-012` — no duplicate bridges within a WorkflowIntent.
- `WF-LINT-013` — AI-assistance fallback chains terminate in human review (`SA-MUST-wfi-014`).

### Runtime conformance fixtures (Stage 4–5)

- WorkflowIntent compiles deterministically given identical inputs (`SA-MUST-wfi-041`).
- Adverse-outcome chain (Outcome → Notice → Appeal) compiles to governance-block entries plus the appeal sub-flow.
- AI-assistance fallback compiles to a transition the runtime can take when confidence-floor breaches.
- Demotion cascade: demoting a referenced PolicyObject re-demotes the WorkflowIntent.

### Current limitations

- The closed list of 16 element kinds is provisional. Real workflows may surface kinds that don't fit; opening the list via `x-` extension is deferred.
- Graph-structured WorkflowIntents (parallel branches, multi-entry sub-flows) are sketched but not fully specified; the spec assumes mostly-linear flow with branching at decision/exception/manual-override points.
- The bridge to kernel constructs is one-to-one (one element → one or two kernel constructs); more complex bridges (e.g., a single user-facing element that maps to a composite of kernel constructs) are deferred to compiler-contract.

## WOS mappings

WorkflowIntent itself is **`authoringOnly`** — it never appears in the published artifact. Its elements project (via the bridges above) into specific `$wosWorkflow` paths:

| Element kind | Mapping state | WOS path (projected) |
|---|---|---|
| `phase` | `mapsToWos` | `$.lifecycle.states[?(@.kind=='compound')]` |
| `step` | `mapsToWos` | `$.lifecycle.states[?(@.kind=='atomic')]` |
| `decision` | `mapsToWos` | `$.lifecycle.transitions[*].guard` (FEL) |
| `review` | `mapsToWos` | `$.lifecycle.tasks[*]` |
| `notice` | `mapsToWos` | `$.governance.notices[*]` (+ optional ServiceBinding emission per [`binding-and-integration.md`](binding-and-integration.md)) |
| `deadline` | `mapsToWos` | `$.lifecycle.timers` |
| `appeal` | `mapsToWos` | sub-flow under `$.lifecycle.states[*]` + `$.governance.appeals[*]` |
| `exception` | `mapsToWos` | alternate transitions with guards |
| `hold` | `mapsToWos` | states with no automatic transitions |
| `data-collection` | `mapsToWos` | Formspec coprocessor (per `wos-formspec-binding` per ADR-0073) |
| `evidence-request` | `mapsToWos` | `$.lifecycle.tasks[*]` + `$.caseFile.evidence[*]` |
| `system-check` | `mapsToWos` | `$.integration.bindings[*]` (ServiceBinding) |
| `AI-assistance` | `mapsToWos` | `$.agents[*]` + `$.aiOversight` + fallback transition |
| `manual-override` | `mapsToWos` | transition with override-authority guard |
| `completion-outcome` | `mapsToWos` | `$.lifecycle.states[?(@.terminal)]` |
| `phase-end` | `mapsToWos` | transitions closing one compound state and entering the next |

## Examples

### Example 1: Minimal SNAP redetermination WorkflowIntent

```text
WorkflowIntent {
  title: "SNAP Redetermination 2026",
  impactLevel: "rights-impacting",
  elements: [
    { kind: "phase",          name: "Intake",                  contains: [el-2, el-3, el-4] },
    { kind: "data-collection", name: "Applicant submits form",  collected: [household.size, income.monthlyGross, ...] },
    { kind: "system-check",   name: "Federal income verify",    serviceBindingRef: sb-fed-broker },
    { kind: "review",         name: "Caseworker reviews",       reviewer: actor-caseworker },
    { kind: "decision",       name: "Eligibility determination", decisionRuleRef: dt-eligibility (DecisionTable) },
    { kind: "completion-outcome", name: "Approved",            outcomeRef: outcome-approved (favorable) },
    { kind: "completion-outcome", name: "Denied",              outcomeRef: outcome-denied (adverse, triggers due process) },
    { kind: "notice",         name: "Denial notice",            noticeRequirementRef: nr-denial-notice },
    { kind: "appeal",         name: "Appeal branch",            appealRightRef: ar-fair-hearing,
                              branchEntryPoint: el-9, deadlineRef: deadline-90d },
    ...
  ]
}
```

The denial element triggers due process; `WF-LINT-001` requires the notice + appeal pair, both present. ✓

### Example 2: AI-assistance with fallback

```text
{
  kind: "AI-assistance",
  name: "Triage application completeness",
  body: {
    aiUsePolicyObjectRef: aiu-triage,
    actorMappingRef: actor-triage-agent,  // actorKind = agent
    confidenceFloor: 0.7,
    fallbackRef: el-manual-review-step
  }
}
```

`SA-MUST-wfi-014` requires `fallbackRef`; the fallback element is a `review` element manned by a human caseworker. ✓ Aligns with parent CLAUDE.md "fallback chain terminating in human review."

### Example 3: Demotion cascade

A reviewer demotes `nr-denial-notice` (NoticeRequirement) from `approved` back to `draft` to add a Spanish-translation requirement. The cascade:

1. `SA-MUST-pom-022` demotes the NoticeRequirement to `draft`.
2. `SA-MUST-wfi-030` finds the WorkflowIntent referencing it; demotes the WorkflowIntent to `draft`.
3. AuthoringProvenanceRecord with `eventKind = demoted` emitted on both the NoticeRequirement and the WorkflowIntent.
4. ChangeImpactReport produced (`SA-MUST-ci-010`) enumerating affected scenarios, mappings, and the published package (if any).
5. Compliance reviewer's prior ApprovalDecision on the WorkflowIntent transitions from `active → superseded`.
6. Re-review and re-approval cycle resumes.

## Open issues

- **Closed vs. open element-kind list.** The 16 kinds cover most public-services workflows. Whether to support `x-` extensions for novel kinds (e.g., a `live-translation` kind for real-time interpreter calls) is unsettled.
- **Graph vs. linear flow.** Real workflows have parallel branches and multi-entry sub-flows. The current spec is mostly linear with branching points; richer graph semantics are deferred.
- **Element reuse.** A "review" step that appears in both happy-path and appeal branches is currently authored twice. Whether elements can be shared across positions is unsettled.
- **Sub-workflow composition.** Some workflows compose smaller workflows (e.g., a SNAP application includes a citizenship-verification sub-workflow). Composition semantics are deferred.

## Cross-references

- Concept model: [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.12, §2.4.
- PRD: [`../VISION.md`](../VISION.md) §9.4 (Workflow Builder).
- Upstream: [`policy-object-model.md`](policy-object-model.md), [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md), [`workspace.md`](workspace.md).
- Downstream: [`compiler-contract.md`](compiler-contract.md), [`scenario-authoring.md`](scenario-authoring.md), [`binding-and-integration.md`](binding-and-integration.md), [`readiness-validation.md`](readiness-validation.md).
- WOS: [`../../specs/kernel/spec.md`](../../specs/kernel/spec.md) (lifecycle semantics), `wos-workflow.schema.json` paths cited in §"WOS mappings."
- Repo conventions: [`../../CONVENTIONS.md`](../../CONVENTIONS.md).
