# Studio Spec: Policy Object Model

**Status:** draft (Stage 2 of [Implementation Roadmap](../VISION.md#17-implementation-roadmap))
**Date:** 2026-04-30
**Concept-model anchor:** [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.6 ExtractedClaim, §1.7 PolicyObject, §1.8 Assumption, §1.9 Conflict, §1.10 ReviewerResolution, §2.2 ExtractedClaim lifecycle, §2.3 PolicyObject lifecycle.
**PRD anchor:** [`../VISION.md`](../VISION.md) §9.2 (Policy Extraction Review).
**Depends on:** [`source-vault.md`](source-vault.md).

## Scope

The Policy Object Model defines the **structured object families** Studio uses to represent reviewed policy as data. PolicyObject is the durable, citable, mappable unit on which mapping, validation, scenario authoring, and change impact operate.

This spec defines:

- the family of PolicyObject `kind`s and the field shapes each kind carries;
- the ExtractedClaim → PolicyObject promotion path;
- the lifecycle a PolicyObject passes through from `draft` to `published` / `superseded`;
- the normative contract for object shape, citation requirements, mapping declaration, and conflict surface;
- composition with the Source Vault upstream and with mapping/validation/scenario/change-impact downstream;
- conformance expectations.

This is the **central content model** of Studio — five later specs ([`authoring-provenance.md`](authoring-provenance.md), [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md), [`readiness-validation.md`](readiness-validation.md), [`scenario-authoring.md`](scenario-authoring.md), [`change-impact.md`](change-impact.md)) directly depend on the kinds enumerated here.

## Out of scope

- The AI extraction pipeline itself (a tooling/runtime concern; this spec defines the *output* shape, not the model or prompts that produce it).
- The visual review queue UX (PRD §9.2 capability; not normative).
- WOS embedded-block schemas (those live in `../../schemas/wos-workflow.schema.json`).

## Terminology

- **Kind** — the discriminator that identifies which structured family a PolicyObject belongs to (e.g., `Requirement`, `NoticeRequirement`, `ActorMapping`).
- **Body** — the kind-specific subset of fields.
- **Family** — a logical grouping of kinds (source-and-authority, requirement, due-process, workflow-mapping, review-and-uncertainty).
- **Promotion** — the act of converting an ExtractedClaim into a PolicyObject (or merging into an existing one).
- **Demotion** — moving an approved PolicyObject back to a workshop state (e.g., when a source supersession invalidates its citation).

## Data model

### Common envelope

Every PolicyObject carries a common envelope (CM §1.7 fields) plus a kind-specific `body`.

```text
PolicyObject {
  id, kind, body, citations[], effectivePeriod, applicabilityScope,
  authorityRank, lifecycleState, mappingState, provenance,
  workspaceId, version, createdBy, createdAt, lastEditedBy, lastEditedAt
}
```

`kind` is one of the values enumerated in §"Source-and-authority objects" through §"Review-and-uncertainty objects" below. The `body` is constrained by `kind` (Stage-3 schema enforces this via `oneOf`/discriminator).

### Families

Five families, 30+ kinds (PRD §9.2). Each family is described in its own section below with the fields each kind adds to the common envelope.

- **Source-and-authority** — establish what counts as authoritative.
- **Requirement** — describe what must, may, or must not happen.
- **Due-process** — encode rights-affecting protections.
- **Workflow-mapping** — bridge structured policy to workflow shape.
- **Review-and-uncertainty** — track gaps, conflicts, and resolutions.

## Source-and-authority objects

These objects do not themselves describe workflow behavior; they establish the **authority and applicability** under which other PolicyObjects are evaluated.

### `PolicySource`

Wraps a SourceDocument with policy-relevant metadata (the legal nature of the source).

**Body fields:** `sourceDocumentRef`, `legalNature` (`statute` | `regulation` | `agency-guidance` | `internal-policy` | `procedure` | `form` | `case-law` | `other`), `issuingAuthority`, `governingJurisdiction`.

### `AuthorityRank`

Establishes precedence among PolicySources for a particular subject. Used to resolve Conflicts deterministically.

**Body fields:** `subject` (free-text or structured topic), `rankedSources[]` (ordered list of PolicySource refs), `rationale`, `scope` (workspace-wide | workflow-scoped | step-scoped).

### `ApplicabilityScope`

Defines when a Requirement / Obligation / etc. applies — by program, jurisdiction, case characteristic, or date range.

**Body fields:** `programs[]`, `jurisdictions[]`, `caseFilters[]` (structured predicates over case-file facts), `dateRange`.

### `EffectivePeriod`

A reusable period record. Many PolicyObjects carry an inline `effectivePeriod`; an `EffectivePeriod` PolicyObject lets a workspace define a named period (e.g., "Pandemic-Era Waivers") referenced by many objects.

**Body fields:** `name`, `start`, `end?`, `triggerCondition?` (FEL expression — see [`../specs/`](../specs/) for FEL anchor), `description`.

### `Supersession`

Records that one PolicySource (or PolicyObject) supersedes another in a specific scope. Distinct from SourceVersion supersession in [`source-vault.md`](source-vault.md): SourceVersion supersession is *document-level*; this object is *policy-level* (e.g., the new SNAP regulation supersedes the old one *for SNAP eligibility*, but not for other programs).

**Body fields:** `superseder` (PolicySource | PolicyObject ref), `superseded` (PolicySource | PolicyObject ref), `effectiveAt`, `scope`.

## Requirement objects

These describe *what the workflow must, may, or must not do*. The deontic kinds (Obligation / Permission / Prohibition) follow the OASIS LegalRuleML convention referenced in the parent [`../../CLAUDE.md`](../../CLAUDE.md).

### `Requirement`

A general-purpose policy claim; reviewers may further specialize into Obligation / Permission / Prohibition.

**Body fields:** `narrative`, `subject` (`agency` | `applicant` | `staff-member` | `system`), `verb`, `object`, `condition?`, `severity` (`mandatory` | `recommended` | `optional`).

### `Obligation`

A duty: the subject MUST do X under condition C.

**Body fields:** `subject`, `action`, `condition?`, `deadline?` (Deadline ref or inline duration), `consequenceOfBreach?`.

### `Permission`

A right: the subject MAY do X under condition C. Distinct from `Right` (which is a due-process construct on the *applicant* side).

**Body fields:** `subject`, `action`, `condition?`, `limits?` (e.g., frequency, amount).

### `Prohibition`

A negative duty: the subject MUST NOT do X under condition C.

**Body fields:** `subject`, `action`, `condition?`, `consequenceOfViolation?`.

### `Condition`

A reusable structured condition (FEL expression — Field Expression Language; see [`../../crates/fel-core`](../../crates/fel-core)). Many Requirements refer to a shared Condition rather than inlining.

**Body fields:** `name`, `expression` (FEL string), `description`, `inputs[]` (DataElement refs the expression depends on).

### `ExceptionRule`

Carves an exception out of a parent rule (Requirement / Obligation / Prohibition).

**Body fields:** `parentRuleRef`, `exceptionWhen` (Condition ref or inline FEL), `effect` (`waive` | `modify` | `delay`), `modifiedRule?` (when `effect = modify`).

### `DecisionRule`

A structured rule that determines a workflow outcome — the closest Studio cousin to a kernel guard / `RuleReference`.

**Body fields:** `inputs[]` (DataElement refs), `logic` (FEL or rule-table-style structured logic), `outputs[]` (Outcome refs), `governingPolicySources[]`.

### `EvidenceRequirement`

What evidence (documents, attestations, system data) is needed to substantiate a fact or decision.

**Body fields:** `factSubject` (DataElement ref or free-text), `acceptableForms[]` (e.g., "W-2", "self-attestation", "agency record"), `verificationMethod`, `retentionPeriod?`.

### `DataElement`

A discrete data field the workflow uses or collects (e.g., "household income", "SSN", "applicant signature").

**Body fields:** `name`, `dataType` (`string` | `number` | `date` | `boolean` | `enum<...>` | `document` | `structured`), `sensitivity` (`public` | `internal` | `pii` | `phi` | `restricted`), `definition`, `derivation?` (if computed).

### `Outcome`

A possible workflow termination state — favorable, unfavorable, partial, deferred.

**Body fields:** `name`, `polarity` (`favorable` | `adverse` | `neutral` | `mixed`), `description`, `triggersDueProcess` (boolean — if `true`, an `adverse` outcome MUST link a NoticeRequirement and AppealRight under tier-S4 readiness).

## Due-process objects

These objects encode rights-affecting protections owed to the applicant when a workflow makes adverse determinations. They map directly to WOS governance (`workflow-governance.md`, `due-process-config.md`).

### `NoticeRequirement`

A required communication to the applicant — typically before, alongside, or after an adverse decision.

**Body fields:** `trigger` (Outcome ref or Condition), `audience` (`applicant` | `representative` | `third-party`), `content[]` (required content elements: reason, citation, effective date, appeal rights, contact, …), `deliveryMethod[]`, `timing` (relative to trigger), `language[]`.

### `AppealRight`

The applicant's right to challenge an adverse determination.

**Body fields:** `triggerOutcomeRef`, `appealKind` (`reconsideration` | `administrative-hearing` | `judicial-review` | `ombudsperson`), `deadline` (filing deadline relative to NoticeRequirement delivery), `forum`, `representationAllowed`, `evidenceProcedure`.

### `ExplanationRequirement`

Mandates that the workflow produce a reasoned explanation for an adverse determination.

**Body fields:** `triggerOutcomeRef`, `explanationTier` (`reason` | `reasoning` | `counterfactual`), `audience`, `formatRequirements`, `governingPolicySources[]`.

### `ContinuationOfServicesRequirement`

Mandates that benefits / services continue during a defined window after an adverse determination (e.g., during appeal). Maps to WOS due-process grace periods.

**Body fields:** `triggerOutcomeRef`, `duration`, `coveredServices[]`, `terminationCondition`.

### `CounterfactualRequirement`

A specialization of ExplanationRequirement — the applicant is owed an explanation that includes "what change in input would have produced a favorable outcome."

**Body fields:** `triggerOutcomeRef`, `counterfactualScope` (which inputs may be varied), `governingPolicySources[]`.

## Workflow-mapping objects

These objects are the **bridge** between structured policy and workflow shape. Reviewers author them after the underlying Requirements / due-process objects are approved; the Studio→WOS compiler reads them to emit the WOS `lifecycle`, `actors`, `caseFile`, and embedded blocks.

### `WorkflowStepMapping`

A user-facing workflow step (PRD §9.4: phase / step / decision / review / notice / deadline / appeal / exception / hold / data collection / evidence request / system check / AI assistance / manual override / completion outcome) and the policy objects it implements.

**Body fields:** `stepName`, `stepKind` (one of the user-facing concepts above), `derivedFrom[]` (PolicyObject refs), `position` (sequence within phase).

### `LifecycleTagMapping`

Maps a workflow step to a kernel lifecycle state name.

**Body fields:** `workflowStepRef`, `kernelStateName`, `stateKind` (`atomic` | `compound` | `parallel`).

### `TransitionMapping`

Maps a movement between workflow steps to a kernel transition.

**Body fields:** `fromStepRef`, `toStepRef`, `triggerEvent`, `guardConditionRef?` (Condition or DecisionRule ref), `actions[]`.

### `TimerMapping`

Maps a Deadline or temporal Requirement to a kernel timer.

**Body fields:** `deadlineSource` (PolicyObject ref carrying the deadline), `kernelStateRef`, `duration` (FEL or absolute), `expiryAction` (`fire-event` | `notify` | `escalate` | `auto-decide`).

### `ActorMapping`

Maps an applicant, staff role, or system actor to a WOS `actor` declaration. When the actor is an agent, links the Studio `AI Use` PolicyObject.

**Body fields:** `actorName`, `actorKind` (`human-applicant` | `human-staff` | `system` | `agent`), `authority[]` (which Obligations/Permissions/Prohibitions this actor exercises), `delegationAllowed`, `agentConfigRef?` (when `actorKind = agent`).

### `TaskMapping`

Maps a unit of human work to a kernel task definition.

**Body fields:** `taskName`, `assignedToActorRef`, `requiredEvidenceRefs[]`, `slaSource?` (Deadline ref).

### `CaseFileMapping`

Maps a DataElement to a caseFile path / shape.

**Body fields:** `dataElementRef`, `caseFilePath`, `cardinality` (`one` | `many`), `validationPipelineRef?`.

### `ScenarioMapping`

Identifies the WOS conformance trace correspondence for a Studio Scenario. Defined here for completeness; full semantics in [`scenario-authoring.md`](scenario-authoring.md).

**Body fields:** `scenarioRef`, `traceTemplateRef`, `expectedKernelEvents[]`.

## Review-and-uncertainty objects

These objects make ambiguity, conflict, and reviewer decisions **first-class** rather than implicit. They are the antidote to the "polished but wrong" failure mode (PRD §15 Risk #5).

### `Assumption` (CM §1.8)

An explicit reviewer-approved gap-fill where source material does not establish behavior.

**Body fields:** `narrative`, `affects[]`, `rationale`, `severity` (`low` | `medium` | `high`), `proposedBy`, `approvedBy?`.

### `OpenQuestion`

A flagged ambiguity awaiting reviewer resolution. Distinct from Assumption: an OpenQuestion is *unresolved*; an Assumption is a *reviewed-and-accepted gap-fill*.

**Body fields:** `narrative`, `affects[]`, `proposedBy`, `assignedTo?`, `priority`, `lifecycleState` (`open` | `under-review` | `answered` | `withdrawn`).

### `Conflict` (CM §1.9)

A detected inconsistency between two or more PolicyObjects, ExtractedClaims, or SourceVersions on the same subject.

**Body fields:** `subjects[]`, `axis`, `kind`, `severity`.

### `ReviewerResolution` (CM §1.10)

The durable record of how a reviewer resolved a Conflict, Assumption, or OpenQuestion.

**Body fields:** `subjectRef`, `decision`, `rationale`, `evidenceCitations[]`.

### `ApprovalDecision` (CM §1.15)

A reviewer's structured sign-off. Defined here for completeness; full semantics in [`review-and-approval.md`](review-and-approval.md).

**Body fields:** `subjectRef`, `decision`, `conditions[]`, `observedFindings[]`.

## Lifecycle (normative)

The PolicyObject lifecycle from CM §2.3:

```text
draft → reviewed → approved
  → { mapsToWos | authoringOnly | requiresSpecExtension | unmappedButApproved }
  → validated → published → superseded
```

Allowed transitions:

| From | To | Trigger |
|---|---|---|
| `draft` | `reviewed` | a reviewer has examined the body and citations |
| `reviewed` | `approved` | a reviewer has signed off (ApprovalDecision created) |
| `reviewed` | `draft` | the reviewer requested edits; back to authoring |
| `approved` | `mapsToWos` / `authoringOnly` / `requiresSpecExtension` / `unmappedButApproved` | mapping declaration recorded (StudioToWosMapping created) |
| any mapping state | `validated` | tier S1–S3 readiness checks pass |
| `validated` | `published` | a PublishedWorkflowPackage that includes this object is published |
| `published` | `superseded` | a later PolicyObject in a later workflow version replaces this one |
| any state ≥ `approved` | `draft` (demotion) | a SourceVersion supersession invalidated the citation; reviewer must re-affirm |

`superseded` is terminal; demotion to `draft` is the only path back.

The four mapping states after `approved` are mutually exclusive (CM §3.5 precedence rule). A PolicyObject MUST occupy exactly one before it can advance to `validated`.

The ExtractedClaim lifecycle (CM §2.2) feeds into PolicyObject `draft`:

```text
candidate → normalized → needsReview → approved (= PolicyObject draft)
                              → rejected   (terminal, no PolicyObject created)
                              → merged     (folded into existing PolicyObject)
                              → split      (multiple PolicyObjects created)
```

Promotion from `needsReview → approved` MUST atomically create the PolicyObject in `draft` state.

## Normative Contract

### Common envelope

- **`SA-MUST-pom-001`** — Every PolicyObject MUST carry `kind`, `body`, `lifecycleState`, `mappingState` (NULL until `approved`, exactly one of the four states once `approved` or later), `workspaceId`, `version`, `createdBy`, `createdAt`. *(schema-pending: required-fields constraint.)*
- **`SA-MUST-pom-002`** — `kind` MUST be one of the kinds enumerated in this spec. Unknown kinds MUST be rejected at object creation. *(schema-pending: enum.)*
- **`SA-MUST-pom-003`** — `body` MUST conform to the kind-specific shape defined here. *(schema-pending: discriminator-based `oneOf`.)*
- **`SA-MUST-pom-004`** — Every approved PolicyObject MUST carry at least one SourceCitation (per [`source-vault.md`](source-vault.md) `SA-MUST-source-020`) **OR** at least one approved Assumption listed in `provenance.basisAssumptions[]`. PolicyObjects with neither MUST be flagged as tier-S2 ValidationFindings. *(lint-pending.)*

### Extraction → promotion

- **`SA-MUST-pom-010`** — An ExtractedClaim with `confidence < 0.5` MUST NOT auto-advance from `needsReview` to `approved`; reviewer action is required. *(workflow-pending; AI policy.)*
- **`SA-MUST-pom-011`** — Promoting an ExtractedClaim to PolicyObject MUST: (a) carry over the SourceCitation; (b) preserve `proposedBy` in the new PolicyObject's provenance; (c) leave the original ExtractedClaim in lifecycle state `approved` for traceability. *(runtime-pending.)*
- **`SA-MUST-pom-012`** — Merging two ExtractedClaims into a single PolicyObject MUST aggregate their citations (deduplicated by `{sourceVersionId, sectionAnchor}`) and record the merge in provenance. *(runtime-pending.)*
- **`SA-MUST-pom-013`** — Splitting one ExtractedClaim into multiple PolicyObjects MUST replicate the citation onto each resulting object, preserving the same `excerpt` so re-verification still applies. *(runtime-pending.)*

### Lifecycle integrity

- **`SA-MUST-pom-020`** — A PolicyObject MUST NOT be advanced past `approved` without a recorded ApprovalDecision (CM §1.15). *(lint-pending: tier S2 readiness rule.)*
- **`SA-MUST-pom-021`** — A PolicyObject's `mappingState` MUST be set exactly when the object enters `approved`; it cannot be NULL once `approved` or later. *(schema-pending: state-dependent required field.)*
- **`SA-MUST-pom-022`** — Citation supersession (per [`source-vault.md`](source-vault.md) `SA-MUST-source-021`) MUST demote the affected PolicyObject to `draft` if the cited section's text changes materially **AND** the change is not waived as immaterial by a ReviewerResolution. *(runtime-pending; cross-spec coupling with source-vault.)*
- **`SA-SHOULD-pom-023`** — A PolicyObject SHOULD record `lastEditedBy` and `lastEditedAt` on every body edit. Edit history is preserved through provenance.

### Kind-specific MUSTs

- **`SA-MUST-pom-030`** — An `Outcome` whose `polarity = adverse` AND `triggersDueProcess = true` MUST be referenced by at least one approved `NoticeRequirement` and at least one approved `AppealRight` in the same workspace before any WorkflowIntent containing that Outcome can advance to `validationReady`. *(lint-pending: tier S4 readiness rule; cross-cutting with [`readiness-validation.md`](readiness-validation.md).)*
- **`SA-MUST-pom-031`** — A `DecisionRule` MUST list every DataElement its `logic` reads in its `inputs[]`. A WorkflowIntent that references a DecisionRule whose `inputs[]` are not all collected before the rule fires MUST surface a tier-S4 ValidationFinding. *(lint-pending.)*
- **`SA-MUST-pom-032`** — A `NoticeRequirement` MUST link an `Outcome` (`triggerOutcomeRef` indirectly via `trigger`) when the trigger is outcome-driven, OR a `Condition` when the trigger is condition-driven. Free-text-only triggers MUST be rejected. *(schema-pending: oneOf for `trigger`.)*
- **`SA-MUST-pom-033`** — An `AppealRight` MUST link the same Outcome as its corresponding NoticeRequirement (or be explicitly waived by ReviewerResolution as a separate-procedure case, e.g., emergency action). *(lint-pending: tier S4 readiness rule.)*
- **`SA-MUST-pom-034`** — An `ExceptionRule` MUST reference an existing parent rule. Orphan ExceptionRules MUST be rejected. *(schema-pending: foreign-key constraint.)*
- **`SA-MUST-pom-035`** — A `Condition` referenced by multiple objects MUST evaluate consistently — i.e., its FEL expression and `inputs[]` are immutable once approved. Edits to an approved Condition produce a new version; dependent objects must re-validate. *(runtime-pending.)*
- **`SA-MUST-pom-036`** — An `ActorMapping` whose `actorKind = agent` MUST link an AI-Use PolicyObject (the agent's declaration carrying autonomy, deontic constraints, confidence policy, fallback chain) before any WorkflowIntent referencing that ActorMapping can advance to `validationReady`. *(lint-pending: tier S4 readiness rule; cross-cutting with WOS `agents` block.)*
- **`SA-MUST-pom-037`** — A `DataElement` whose `sensitivity` is `pii`, `phi`, or `restricted` MUST carry a documented retention policy on every EvidenceRequirement that collects or uses it; missing retention surfaces a tier-S4 ValidationFinding. *(lint-pending.)*
- **`SA-SHOULD-pom-038`** — A `Permission` SHOULD be paired with at least one `Condition` describing when the permission applies; pure unconstrained permissions are rare in real policy and warrant reviewer attention.
- **`SA-MUST-pom-039`** — A `Supersession` PolicyObject (the policy-level supersession; distinct from [`source-vault.md`](source-vault.md) SourceVersion supersession) MUST identify both `superseder` and `superseded`; circular supersession (A supersedes B which supersedes A) MUST be detected and rejected. *(lint-pending; cross-spec with [`change-impact.md`](change-impact.md).)*

### Conflict surface

- **`SA-MUST-pom-040`** — When two approved PolicyObjects within the same workspace and overlapping ApplicabilityScope contradict (e.g., two Deadlines with different durations on the same trigger), the implementation MUST create a Conflict entity. The contradiction MUST NOT be silently merged. *(lint-pending; runtime-pending: conflict-detection algorithm.)*
- **`SA-MUST-pom-041`** — A Conflict MUST be either resolved (via ReviewerResolution) or waived before any dependent WorkflowIntent advances past `mapped`. *(lint-pending: tier S2 readiness rule.)*
- **`SA-SHOULD-pom-042`** — Conflict detection SHOULD use AuthorityRank to *suggest* a resolution to the reviewer, but MUST NOT auto-apply the resolution.

## Composition

### Attachment point

PolicyObjects live at the workspace layer. They are produced by promotion from ExtractedClaims (which derive from SourceSections), and consumed by:

- [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md) — to record the WOS counterpart for each approved object.
- [`readiness-validation.md`](readiness-validation.md) — to evaluate tier S2–S4 readiness rules.
- [`scenario-authoring.md`](scenario-authoring.md) — to link scenarios to the policy claims they exercise.
- [`change-impact.md`](change-impact.md) — to propagate source-version changes downstream.
- The Studio→WOS compiler (Stage 5) — to emit `$wosWorkflow` content.

### Precedence

When two kinds could equally describe the same source claim (e.g., a single sentence is both a Requirement *and* a Prohibition), reviewer judgment governs the kind choice. Studio does not algorithmically pick a kind. The default heuristic that reviewers may rely on:

1. **Prohibition** wins over **Obligation** when both could apply (negative duties are stricter and easier to enforce).
2. **DecisionRule** wins over **Condition** when the structured logic is outcome-determining rather than merely a precondition.
3. **NoticeRequirement** wins over **Requirement** when the obligation is communicative.
4. **EvidenceRequirement** wins over **DataElement** when the field exists *because* it must be substantiated, not merely captured.

These are guidance, not normative — reviewer notes captured in provenance record the rationale.

### Conflict handling

Two contradictory approved PolicyObjects MUST produce a Conflict entity (`SA-MUST-pom-040`). The implementation MUST NOT silently pick one or merge them. AuthorityRank guides resolution but does not auto-apply.

### Versioning / migration

- Adding a new `kind` to the closed enum is a **schema-breaking** change; it requires a version bump on the Studio Policy Object schema (Stage 3) and a migration plan for existing workspaces.
- Adding a new optional field to a kind's `body` is **non-breaking**.
- Removing or renaming a field is **schema-breaking**.
- Strengthening a kind-specific MUST (e.g., adding a new required field to `Outcome.body`) is **schema-breaking** and triggers re-validation of every approved PolicyObject of that kind.

## Conformance

### Schema validation (Stage 3)

Planned schema gates:

- Discriminated `oneOf` on `kind` for `body` shape.
- Required-fields-by-state (`mappingState` required iff `lifecycleState >= approved`).
- Citation-or-assumption gate (`SA-MUST-pom-004`).
- Foreign-key validation for `parentRuleRef` (ExceptionRule), `triggerOutcomeRef` (NoticeRequirement / AppealRight), `dataElementRef` (CaseFileMapping), `agentConfigRef` (ActorMapping when agent), etc.

### Lint rules (Stage 4)

Tier-S2 ("Policy object readiness") rules planned:

- `POM-LINT-001` — every approved PolicyObject has at least one citation or basis-assumption (SA-MUST-pom-004).
- `POM-LINT-002` — adverse Outcomes link Notice + Appeal (SA-MUST-pom-030).
- `POM-LINT-003` — DecisionRule inputs are collected before evaluation (SA-MUST-pom-031).
- `POM-LINT-004` — agent ActorMappings link an AI-Use object (SA-MUST-pom-036).
- `POM-LINT-005` — sensitive DataElements have retention policy (SA-MUST-pom-037).
- `POM-LINT-006` — no orphan ExceptionRules (SA-MUST-pom-034).
- `POM-LINT-007` — no circular Supersession (SA-MUST-pom-039).
- `POM-LINT-008` — every Conflict resolved or waived before downstream advance (SA-MUST-pom-041).

### Runtime conformance fixtures (Stage 4–5)

- ExtractedClaim → PolicyObject promotion preserves citation, provenance, and original-claim trace.
- Conflict detection between two contradictory Deadlines on the same trigger.
- Citation supersession demotes the dependent PolicyObject to `draft`.
- Mapping-state precedence (CM §3.5) is enforced at state assignment.

### Current limitations

The kind enumeration is **not yet closed**. PRD §9.2 names ~30 kinds; this spec enumerates them but does not foreclose extensions. Whether a workspace may declare custom kinds (via `x-` extension axis) is deferred to Stage 3 schema work.

## WOS mappings

The full Studio→WOS mapping table lives in [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §5 and is normatively elaborated in [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md). For this spec, the relevant defaults per family:

| Family | Default mapping state | Notes |
|---|---|---|
| Source-and-authority | `authoringOnly` | Authority and applicability are workspace metadata; only citations project (compactly) into provenance. |
| Requirement | `mapsToWos` (most kinds) | Obligation/Permission/Prohibition map to deontic constraints; DecisionRule maps to lifecycle guard / RuleReference; Outcome maps to lifecycle terminal state. |
| Due-process | `mapsToWos` (always) | NoticeRequirement, AppealRight, ExplanationRequirement, ContinuationOfServicesRequirement, CounterfactualRequirement → embedded `governance` block. |
| Workflow-mapping | `mapsToWos` (always) | The kinds in this family *are* the bridge; their entire purpose is mapping. |
| Review-and-uncertainty | `authoringOnly` (mostly) | Assumption / OpenQuestion / Conflict are workspace-only. ReviewerResolution and ApprovalDecision project compactly into provenance. |

`requiresSpecExtension` is the right state when a real customer policy concept does not fit any existing WOS embedded block. `unmappedButApproved` is reserved for explicit, documented exceptions and is expected to be rare.

## Examples

### Example 1: SNAP eligibility — Outcome + Notice + Appeal trio

A SNAP-eligibility workflow has three Outcomes: `approved` (favorable), `denied` (adverse, triggersDueProcess=true), `pending-info` (neutral). The reviewer authoring the workflow:

1. Promotes a Requirement extracted from 7 CFR §273.10 into a Requirement PolicyObject. State: `draft → reviewed → approved → mapsToWos`.
2. Promotes the `denied` Outcome similarly. Because `triggersDueProcess = true`, `SA-MUST-pom-030` requires a NoticeRequirement and an AppealRight referencing this Outcome before validationReady.
3. Authors a NoticeRequirement (`trigger = denied`, `content = [reason, citation, effective-date, appeal-rights, contact]`, `timing = within 10 calendar days`, `language = [en, es]`). State: `mapsToWos`.
4. Authors an AppealRight (`triggerOutcomeRef = denied`, `appealKind = administrative-hearing`, `deadline = 90 days from notice delivery`, `forum = state agency`). State: `mapsToWos`.
5. Tier-S4 readiness for the WorkflowIntent now passes for this Outcome.

### Example 2: Conflict between two source versions

Two regulatory citations both describe the appeal deadline for the same workflow: one says 60 days, the other 90 days. Both are extracted as ExtractedClaims, both promoted to AppealRight PolicyObjects.

1. Conflict detection (`SA-MUST-pom-040`) fires: a Conflict entity is created with `axis = deadline-duration`, `subjects = [appealRight-A, appealRight-B]`.
2. The reviewer reviews. AuthorityRank suggests the later regulation (90 days) wins; reviewer authors a ReviewerResolution adopting that suggestion.
3. The 60-day AppealRight is demoted to `superseded`; the 90-day AppealRight remains `approved → mapsToWos`.
4. Tier-S2 readiness for the workspace now passes (the Conflict is resolved).

### Example 3: Agent declaration

An eligibility workflow uses an AI agent to triage incomplete applications. The reviewer:

1. Authors an `AI Use` PolicyObject (kind not enumerated in this spec's families because it lives in a Studio "agents" sub-family, captured in [`scenario-authoring.md`](scenario-authoring.md) and the Studio→WOS mapping for `agents[]`). It carries: `model`, `autonomy`, `deontic-constraints`, `confidence-floor`, `fallback-chain`.
2. Authors an ActorMapping with `actorKind = agent` and `agentConfigRef → AI Use object`. Tier-S4 readiness rule `SA-MUST-pom-036` is satisfied.
3. Authors a fallback Obligation: "If agent confidence < 0.7, route to human review." Maps to the kernel's deontic constraint enforcement.

## Open issues

- **Kind enum closure.** Whether the kind enumeration is closed (Stage-3 schema rejects unknown kinds) or open via an `x-` axis is unsettled. Closure is safer; openness lets domain-specific extensions emerge from real customer workflows. Decision deferred to Stage 3.
- **`Right` as a kind.** PRD §6 mentions "right" (in the OASIS LegalRuleML deontic family). This spec covers `Permission`/`Prohibition`/`Obligation` but not `Right` directly; whether `AppealRight` and `ContinuationOfServicesRequirement` cover it or whether a generic `Right` kind is needed is unsettled.
- **DataElement vs. EvidenceRequirement boundary.** Some fields are inherently both (e.g., "applicant's W-2"). The spec leans on reviewer judgment; a clearer rule may emerge from Stage-8 vertical slices.
- **Inline vs. referenced Conditions.** Many Requirements carry condition-shaped fields inline; the spec offers a separate `Condition` kind for shared logic. The migration path from inline to shared is unspecified.
- **Object versioning model.** When an approved PolicyObject is edited (via demotion → re-approval), is its `version` field a monotonic counter, a content hash, or a workspace-scoped sequence? Decision deferred to Stage 3.

## Cross-references

- Concept model: [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.6–§1.10, §2.2, §2.3, §3.
- PRD: [`../VISION.md`](../VISION.md) §9.2, §12 (user stories).
- Upstream: [`source-vault.md`](source-vault.md).
- Downstream: [`authoring-provenance.md`](authoring-provenance.md), [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md), [`readiness-validation.md`](readiness-validation.md), [`scenario-authoring.md`](scenario-authoring.md), [`change-impact.md`](change-impact.md), [`review-and-approval.md`](review-and-approval.md).
- Repo conventions: [`../../CONVENTIONS.md`](../../CONVENTIONS.md).
