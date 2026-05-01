# WOS Studio (Authoring) — Concept Model

**Status:** draft — Stage 1 of the [Implementation Roadmap](VISION.md#17-implementation-roadmap).
**Date:** 2026-04-30
**Reads as:** the bridge between [`VISION.md`](VISION.md) (product vision, prose) and [`specs/`](specs/) (W3C-style normative specifications).

## Scope

This document fixes the **noun catalog** for WOS Studio (Authoring): the entities the product manipulates, the lifecycles those entities pass through, the boundaries between session / workspace / published state, the four mapping states every approved object must declare, and the cross-reference from Studio concepts to existing WOS concepts.

It is **not**:

- A schema. Schemas are Stage 3.
- A normative spec. Normative behavior lives in [`specs/`](specs/).
- An exhaustive ontology. Studio's structured policy model deliberately tracks only what real workflow authoring requires; new entities are added when real customer documents demand them, not speculatively.

Section numbering here is local; cross-references to PRD use `VISION §N`.

## Out of scope

- Runtime observation data structures beyond what is needed for designed-vs-observed comparison (PRD §9.11; Phase 4).
- The Studio→WOS compiler's internal data structures (Stage 5).
- Reference-architecture component boundaries (Stage 7).
- Any UI or storage choice.

## 1. Core entities

The 23 entities below are the durable authoring vocabulary. Each entity has: a definition, the load-bearing fields, and the relationships it participates in. Field lists are illustrative — Stage 3 schemas will fix exact shapes. (§1.17 RuntimeObservation is a Phase-4 placeholder name; §1.20–1.23 are introduced by `specs/binding-and-integration.md`; §1.19 WorkflowIntent has its data-model home in `specs/workflow-intent.md`; §1.24 Workspace has its data-model home in `specs/workspace.md`.)

### 1.1 Workspace

A bounded authoring environment for one or more workflows. Holds source documents, structured objects, mappings, scenarios, validation findings, reviewer assignments, and publication packages. Permissions, audit log, and access control attach at the Workspace.

**Key fields:** `id`, `title`, `owners[]`, `createdAt`, `permissionsRef`, `programs[]` (jurisdictions / programs the workspace covers).
**Relationships:** owns SourceDocument*, PolicyObject*, StudioToWosMapping*, WorkflowIntent*, Scenario*, ValidationFinding*, ApprovalDecision*, PublishedWorkflowPackage*.

### 1.2 SourceDocument

A primary input artifact: policy manual, SOP, regulation, memo, form, decision guide, service blueprint, case documentation, diagram, screenshot, system export, integration document. The original of record.

**Key fields:** `id`, `title`, `type` (policy | sop | regulation | memo | form | …), `program`, `jurisdiction`, `language`, `permissions`, `originUrl`, `uploadedBy`, `uploadedAt`.
**Relationships:** has SourceVersion*, contains SourceSection*, cited by SourceCitation*.

### 1.3 SourceVersion

A specific version of a SourceDocument. Carries effective-date metadata, supersession lineage, and the raw + parsed payload for that version.

**Key fields:** `id`, `documentId`, `versionLabel`, `effectiveStart`, `effectiveEnd`, `supersedes` (prior version ref), `payloadRef` (raw bytes), `parsedRef` (text + structure), `pageMap` (page → offset table).
**Relationships:** belongs to SourceDocument; supersedes prior SourceVersion; cited by SourceCitation*; compared in ChangeImpactReport*.

### 1.4 SourceSection

A semantically meaningful chunk of a SourceVersion: section, paragraph, table cell, form field, diagram region. The unit at which extracted objects anchor.

**Key fields:** `id`, `sourceVersionId`, `path` (hierarchical anchor, e.g. `§3.2.1` or `page=4,para=2`), `text`, `kind` (heading | paragraph | table | list-item | form-field | …), `pageRange`.
**Relationships:** belongs to SourceVersion; anchors SourceCitation*; basis for ExtractedClaim*.

### 1.5 SourceCitation

A typed reference from a Studio object back to a SourceSection (or a span within one). Every meaningful workflow behavior should ultimately resolve to a SourceCitation chain or to an explicitly approved Assumption.

**Key fields:** `id`, `subjectRef` (the Studio object citing), `sourceSectionId`, `span` (start/end offsets within the section, optional), `quote` (verbatim excerpt), `relation` (`supports` | `derivedFrom` | `conflictsWith` | `supersedes`).
**Relationships:** subject → ExtractedClaim | PolicyObject | WorkflowIntent | Scenario; object → SourceSection.

### 1.6 ExtractedClaim

An AI-proposed structured interpretation of a SourceSection. Carries confidence, candidate type, and reviewer state. ExtractedClaim is the **flexible** intermediate form: tolerates uncertainty, partial fields, and messy evidence (PRD §8 — flexible schemas).

**Key fields:** `id`, `sourceCitationRef`, `candidateType` (Requirement | Deadline | NoticeRequirement | …), `extractedFields` (a partially-filled candidate object), `confidence` (0..1), `reviewerState` (`candidate` | `normalized` | `needsReview` | `approved` | `rejected` | `merged` | `split`), `proposedBy` (model id + run id), `reviewerNotes[]`.
**Relationships:** derived from SourceSection (via SourceCitation); on approval, becomes (or merges into) PolicyObject; rejection is a terminal state.

### 1.7 PolicyObject

The reviewed-and-approved structured interpretation. Stable, citable, mappable. PolicyObject is the **hard** form (PRD §8 — hard schemas) and the unit on which mapping, validation, scenario authoring, and change impact operate.

**Key fields:** `id`, `kind` (one of the structured object families enumerated in PRD §9.2 and detailed in [`specs/policy-object-model.md`](specs/policy-object-model.md): PolicySource, AuthorityRank, ApplicabilityScope, EffectivePeriod, Supersession, Requirement, Obligation, Permission, Prohibition, Condition, ExceptionRule, DecisionRule, EvidenceRequirement, DataElement, Outcome, NoticeRequirement, AppealRight, ExplanationRequirement, ContinuationOfServicesRequirement, CounterfactualRequirement, WorkflowStepMapping, LifecycleTagMapping, TransitionMapping, TimerMapping, ActorMapping, TaskMapping, CaseFileMapping, ScenarioMapping), `body` (kind-specific fields), `citations[]` (SourceCitation refs), `effectivePeriod`, `applicabilityScope`, `authorityRank`, `lifecycleState`, `mappingState` (one of the four — see §3 below), `provenance` (AuthoringProvenance ref).
**Relationships:** derived from ExtractedClaim*; cites SourceSection* via SourceCitation*; subject of StudioToWosMapping*; referenced by WorkflowIntent and Scenario; subject of ValidationFinding*; tracked by ChangeImpactReport*.

### 1.8 Assumption

An explicit, reviewer-approved gap-fill where source material does not establish behavior. Assumptions are tracked **separately** from extracted requirements so that unsupported workflow behavior cannot silently sneak into the process (PRD §5 Principle 1, §15 Risk #3).

**Key fields:** `id`, `narrative` (plain language statement of the assumption), `affects[]` (PolicyObject refs or workflow elements), `rationale`, `proposedBy`, `approvedBy`, `approvedAt`, `lifecycleState` (`proposed` | `approved` | `rejected` | `superseded`), `severity` (`low` | `medium` | `high` — high blocks publication).
**Relationships:** affects PolicyObject* / WorkflowIntent / Scenario; resolved by ReviewerResolution; tracked in ChangeImpactReport when underlying source changes.

### 1.9 Conflict

A detected inconsistency between two or more PolicyObjects, ExtractedClaims, or SourceVersions on the same subject (e.g. two sources prescribing different appeal deadlines). Surfaces an explicit resolution requirement.

**Key fields:** `id`, `subjects[]` (the conflicting object refs), `axis` (e.g. `deadline-duration` | `actor-authority` | `notice-content`), `kind` (`source-vs-source` | `source-vs-assumption` | `interpretation-vs-interpretation`), `severity`, `lifecycleState` (`unresolved` | `resolved` | `waived`), `detectedAt`.
**Relationships:** participates with PolicyObject* or SourceVersion*; resolved by ReviewerResolution.

### 1.10 ReviewerResolution

The durable record of how a reviewer resolved a Conflict, Assumption, or open interpretive question. Captures the rationale, the chosen outcome, the reviewer identity, and the timestamp. Mapped to authoring provenance and (where applicable) to governance rationale on the WOS side.

**Key fields:** `id`, `subjectRef` (Conflict | Assumption | OpenQuestion), `decision` (free-form + structured outcome), `rationale`, `reviewerId`, `reviewerRole`, `decidedAt`, `evidenceCitations[]` (which sources were consulted).
**Relationships:** resolves Conflict | Assumption | OpenQuestion; recorded in AuthoringProvenance; surfaced in ApprovalDecision.

### 1.11 StudioToWosMapping

The contract record connecting an approved Studio object to its WOS counterpart. Every approved PolicyObject **MUST** carry a mapping state (one of the four below); when state is `mapsToWos`, the mapping record carries the concrete target.

**Key fields:** `id`, `subjectRef` (PolicyObject id), `state` (`mapsToWos` | `authoringOnly` | `requiresSpecExtension` | `unmappedButApproved`), `targets[]` (when `mapsToWos`: list of `{wosConceptId, wosJsonPath, mappingNotes}`), `extensionRecordRef` (when `requiresSpecExtension`), `unmappedRationale` (when `unmappedButApproved`), `approvedBy`, `approvedAt`.
**Relationships:** subject is PolicyObject; targets reference WOS concepts at `../../specs/kernel/`, `../../specs/governance/`, `../../specs/ai/`, `../../specs/advanced/`, etc., or JSON paths inside `../../schemas/wos-workflow.schema.json`; participates in WorkflowIntent compilation; subject of mapping-tier ValidationFinding*.

### 1.12 WorkflowIntent

The Studio-internal model of a workflow draft: phases, steps, decisions, deadlines, notices, appeals, exceptions, holds, data collection, evidence requests, system checks, AI assistance, manual overrides, completion outcomes (PRD §9.4 default user-facing concepts). Compiles to a `$wosWorkflow` artifact.

**Key fields:** `id`, `workspaceId`, `title`, `lifecycleState` (`draft` | `mapped` | `validationReady` | `scenarioTested` | `approved` | `published` | `deprecated`), `elements[]` (phases / steps / decisions / …), `policyObjectRefs[]` (which approved objects back the workflow), `mappingRefs[]`, `compiledArtifactRef` (WOS document blob), `expertModeView` (mapping from intent elements to WOS JSON paths), `version`, `parentVersion`.
**Relationships:** derived from PolicyObject* via StudioToWosMapping*; subject of ValidationFinding*; exercised by Scenario*; compared with prior versions in ChangeImpactReport; produces PublishedWorkflowPackage on approval.

### 1.13 Scenario

A durable, authorable, reviewable test artifact: one concrete case path through a WorkflowIntent. Captures inputs, event sequence, time advances, expected workflow path, expected notice, expected appeal branch, expected task, expected timer, expected decision/outcome, expected provenance observations (PRD §9.7).

**Key fields:** `id`, `name`, `purpose`, `workflowIntentRef`, `scenarioType` (`happy-path` | `incomplete-application` | `deadline-missed` | `adverse-determination` | `notice-generated` | `appeal-filed` | `exception-applies` | `supporting-document-missing` | `manual-override` | `system-failure-fallback` | `agent-fallback` | `policy-change`), `linkedPolicyObjects[]`, `initialCaseState`, `inputs`, `eventSequence[]`, `timeAdvances[]`, `expectedPath`, `expectedOutcomes`, `lifecycleState` (`generated` | `reviewed` | `passing` | `failing` | `acceptedAsKnownGap` | `regression`), `traceRef` (last simulation result).
**Relationships:** exercises WorkflowIntent; cites PolicyObject* (via `linkedPolicyObjects`); produces a WOS conformance trace at compile/run time (mapping target: `../../schemas/wos-tooling.schema.json`); subject of ValidationFinding*.

### 1.14 ValidationFinding

A tier-tagged readiness finding produced by Studio's readiness/lint engine (PRD §9.6 tiers S1–S6). The unit on which the Validation Center, approval blockers, and reviewer attention all hinge.

**Key fields:** `id`, `tier` (`S1` source-and-extraction | `S2` policy-object | `S3` mapping | `S4` workflow | `S5` scenario | `S6` publication), `ruleId` (Studio readiness rule identifier), `severity` (`info` | `warn` | `error` | `block`), `subjectRef`, `message` (plain-language), `suggestedFix` (optional), `lifecycleState` (`open` | `acknowledged` | `resolved` | `waived`), `waivedBy`, `waivedRationale`.
**Relationships:** subject can be SourceDocument | PolicyObject | StudioToWosMapping | WorkflowIntent | Scenario | ApprovalDecision; surfaced in approval package; gating in PublishedWorkflowPackage.

### 1.15 ApprovalDecision

A reviewer's structured sign-off on a unit of authoring work: an extracted requirement, a conflict resolution, a mapping, a workflow draft, a scenario, or a publication. Carries identity, role, timestamp, scope, and any conditions/observations.

**Key fields:** `id`, `subjectRef`, `subjectKind`, `reviewerId`, `reviewerRole`, `decision` (`approved` | `rejected` | `approved-with-conditions`), `conditions[]`, `observedFindings[]` (ValidationFinding refs the reviewer reviewed), `decidedAt`, `signatureRef` (when binding signature is required).
**Relationships:** subject can be ExtractedClaim | PolicyObject | StudioToWosMapping | Conflict | Assumption | WorkflowIntent | Scenario | PublishedWorkflowPackage; aggregated into ApprovalPackage at publication.

### 1.16 PublishedWorkflowPackage

The terminal Studio artifact: an approved, version-stamped bundle that ships outside the workspace. Contains the `$wosWorkflow` document, scenario suite, validation report, approval certificate, source-citation manifest, and release notes.

**Key fields:** `id`, `workflowIntentRef`, `version`, `wosArtifact` (or ref), `scenarioSuiteRef[]`, `validationReportRef`, `approvalCertificate` (aggregated ApprovalDecisions + signatures), `sourceManifest` (every SourceVersion citation), `releaseNotes`, `publishedBy`, `publishedAt`, `supersededBy` (later package, when applicable).
**Relationships:** derives from WorkflowIntent at the moment of publication; references every contributing PolicyObject + Mapping + Scenario + ApprovalDecision; subject of ChangeImpactReport when sources change post-publication.

### 1.17 RuntimeObservation (Phase-4 placeholder)

**Status:** Phase-4 forward-looking entity. Removed from the Stage-2 spec set; no `specs/runtime-observation.md` exists at this stage. The entity name is reserved here so other specs may forward-reference it (e.g., `originClass = runtime-observed` in `authoring-provenance.md`, `triggerKind = runtime-observation-cluster` in `change-impact.md`); the data model and normative contract are deferred to Phase-4 work. When Phase 4 is real, write the spec then.

### 1.18 ChangeImpactReport

The artifact connecting a source/policy change to its downstream effects. Drives Phase-3 change management (PRD §9.8). Full data model in [`specs/change-impact.md`](specs/change-impact.md).

**Key fields:** `id`, `triggerKind` (`source-version-change` | `policy-object-edit` | `mapping-update` | `runtime-observation-cluster` (Phase-4 placeholder)), `triggerRef`, `affectedPolicyObjects[]`, `affectedMappings[]`, `affectedWorkflowElements[]`, `affectedScenarios[]`, `affectedPublishedPackages[]`, `affectedAssumptions[]`, `affectedReviewerResolutions[]`, `summary`, `producedAt`, `acknowledgedBy`, `acknowledgedAt`, `closedAt?`, `closureRationale?`.
**Relationships:** triggered by SourceVersion supersession, PolicyObject edit, or mapping update; references the entire downstream chain; closes when each affected artifact is reviewed/updated/waived.

### 1.19 WorkflowIntent (data model home: [`specs/workflow-intent.md`](specs/workflow-intent.md))

The user-facing draft of a workflow before compilation to `$wosWorkflow`. WorkflowIntent is composed of `WorkflowElement`s of the 16 user-facing kinds (PRD §9.4: phase / step / decision / review / notice / deadline / appeal / exception / hold / data-collection / evidence-request / system-check / AI-assistance / manual-override / completion-outcome / phase-end). Each element has a `bridge` field that determines how it compiles to kernel constructs (state, transition, timer, task, guard).

**Key fields:** `id`, `workspaceId`, `title`, `impactLevel`, `lifecycleState`, `elements[]` (WorkflowElement), `policyObjectRefs[]`, `mappingRefs[]`, `bindingRefs[]`, `compiledArtifactRef?`, `expertModeView?`, `version`, `parentVersion?`, `provenance`.
**Relationships:** derives from PolicyObject* via StudioToWosMapping*; carries Bindings (Service / Event / PolicyEngine / DecisionTable); compiled by the Studio→WOS compiler ([`specs/compiler-contract.md`](specs/compiler-contract.md)); exercised by Scenario*; produces PublishedWorkflowPackage on approval.

### 1.20 ServiceBinding (data model home: [`specs/binding-and-integration.md`](specs/binding-and-integration.md))

A workflow step ↔ external API operation binding. Maps a `system-check`, `data-collection`, `evidence-request`, or `notice` WorkflowElement to an OpenAPI operation, an Arazzo step, or a custom integration ref.

**Key fields:** `operationRef`, `operationKind` (openapi | arazzo | custom), `apiSpecRef`, `inputBindings[]`, `outputBindings[]`, `errorHandling`, `sensitivityHandling`, `sequencePosition?`.
**Relationships:** attached to a WorkflowElement (kind-specific); compiles to `$.integration.bindings[*]` of type `openapi-call` or `arazzo-step`; attaches at `contractHook` kernel seam.

### 1.21 EventBinding (data model home: [`specs/binding-and-integration.md`](specs/binding-and-integration.md))

A workflow event ↔ kernel event binding. Carries CloudEvents extension attributes (`woscausationeventid`, `woscorrelationkey`) for case correlation and causal chains.

**Key fields:** `eventName`, `direction` (consumed | emitted), `payloadShape[]`, `cloudEventsExtensions`, `channel?`, `bindsTo` (workflow attachment).
**Relationships:** attached to a WorkflowIntent trigger, transition, or action; compiles to `$.integration.bindings[*]` of type `event-consume` or `event-emit`; attaches at `lifecycleHook` kernel seam.

### 1.22 PolicyEngineBinding (data model home: [`specs/binding-and-integration.md`](specs/binding-and-integration.md))

A workflow check ↔ external policy engine (OPA / Cedar / XACML) binding. The engine's response is normalized to `{decision, reasons[], obligations[]}` per the WOS PolicyDecision contract; composition is `deny-overrides` (engine deny overrides Studio-side permit).

**Key fields:** `engineKind`, `engineEndpointRef`, `policyRef`, `inputContract`, `outputNormalization`, `composition`.
**Relationships:** attached at a transition guard or output-validation boundary; compiles to `$.integration.bindings[*]` of type `policy-engine`; attaches at `contractHook` kernel seam.

### 1.23 DecisionTable (data model home: [`specs/binding-and-integration.md`](specs/binding-and-integration.md))

An extension to the existing `DecisionRule` PolicyObject kind for multi-row decision authoring. Compiles to a chained-FEL-guard sequence; **no DMN export** per the audit findings.

**Key fields:** `form: "table"`, `inputs[]`, `outputs[]`, `rows[]`, `hitPolicy` (first-match | priority | unique | output-merge), `completenessRequirement`, `fallback?`.
**Relationships:** referenced by `decision` WorkflowElements; compiles to `$.lifecycle.transitions[*].guard` (chained FEL).

### 1.24 Workspace (extended; data model home: [`specs/workspace.md`](specs/workspace.md))

The bounded authoring environment that owns one or more workflows. The data model is detailed in `specs/workspace.md`; this entry summarizes the entities the Workspace owns:

**Owned entities:** ReviewerRole registry (workspace-scoped), WorkspacePolicy (administrator-configured behavior), WorkspaceAuditLogEntry (queryable view over AuthoringProvenanceRecords + non-provenance events), permissions surface, identity model.
**Lifecycle:** `created → active → { archived | suspended }`; `suspended → active`; `archived` is terminal (read-only).

## 2. Lifecycles

The five lifecycles below are normative — Stage-2 specs and (later) Stage-3 schemas enforce these state machines. Allowed transitions are listed; any other transition is invalid.

### 2.1 SourceDocument lifecycle

```text
uploaded → parsed → indexed → classified → { current | superseded | preliminary | disputed }
```

- `uploaded → parsed`: text extraction succeeded.
- `parsed → indexed`: full-text index built; sections identified.
- `indexed → classified`: document type, program, jurisdiction, effective dates assigned.
- `classified → current`: this is the active version for its scope.
- `classified → preliminary`: not yet effective (e.g., proposed regulation).
- `classified → disputed`: another source contradicts; resolution pending.
- `current → superseded`: a later version replaced this one.

### 2.2 ExtractedClaim lifecycle

```text
candidate → normalized → needsReview → { approved | rejected | merged | split }
```

- `candidate → normalized`: AI normalization (deduplication, field-shape) ran.
- `normalized → needsReview`: queued for human review.
- `needsReview → approved`: reviewer accepted; promoted to PolicyObject.
- `needsReview → rejected`: reviewer rejected; terminal.
- `needsReview → merged`: combined with another claim into one PolicyObject.
- `needsReview → split`: divided into multiple PolicyObjects.

### 2.3 PolicyObject lifecycle

```text
draft → reviewed → approved → { mapped | authoringOnly | requiresSpecExtension | unmappedButApproved } → validated → published → superseded
```

The branch after `approved` is the **mapping state** (see §3 below). Every approved PolicyObject MUST occupy exactly one of those four states before it can advance to `validated`.

### 2.4 WorkflowIntent lifecycle

```text
draft → mapped → validationReady → scenarioTested → approved → published → deprecated
```

- `draft`: structure exists; mappings incomplete.
- `mapped`: every required workflow element resolves to a Studio-to-WOS mapping or to an `authoringOnly` policy object.
- `validationReady`: tier S1–S4 readiness checks pass (or are explicitly waived).
- `scenarioTested`: tier S5 readiness checks pass.
- `approved`: required reviewers signed off; tier S6 publication blockers cleared.
- `published`: shipped as a PublishedWorkflowPackage.
- `deprecated`: superseded by a later version or formally retired.

### 2.5 Scenario lifecycle

```text
generated → reviewed → { passing | failing } → acceptedAsKnownGap → regression
```

- `generated`: created (AI-proposed or hand-authored).
- `reviewed`: a reviewer confirmed the scenario expresses the intended case.
- `passing`: last simulation matched expected outcomes.
- `failing`: last simulation diverged from expected outcomes.
- `acceptedAsKnownGap`: failing, but accepted with rationale (no fix yet, e.g., known limitation).
- `regression`: previously passing; now failing — escalated.

## 3. Mapping states

Every approved PolicyObject **MUST** declare exactly one of the four mapping states below. This is the load-bearing invariant of the Studio-to-WOS contract (PRD §6, [`specs/studio-to-wos-mapping.md`](specs/studio-to-wos-mapping.md)).

### 3.1 `mapsToWos`

The PolicyObject maps to one or more WOS concepts or JSON paths. The mapping record carries the targets. **Default expectation** for due-process objects (NoticeRequirement, AppealRight, ExplanationRequirement), workflow-shape objects (Requirement, Deadline, EvidenceRequirement, Outcome), and AI-use objects.

### 3.2 `authoringOnly`

The PolicyObject supports review, explanation, or change management but is **not emitted** into the WOS artifact. Examples: PolicySource, AuthorityRank, ApplicabilityScope, EffectivePeriod, Supersession, OpenQuestion, Assumption (when not yet promoted to a workflow effect), and most ReviewerResolutions. Authoring-only objects must be **explicitly marked** so they are not mistaken for runtime behavior (PRD §12 user stories).

### 3.3 `requiresSpecExtension`

The PolicyObject captures a need that current WOS does not represent. The mapping record carries an `extensionRecordRef` describing the candidate extension (target seam, proposed semantics, motivating evidence). Repeated extension records on the same axis become candidates for upstream WOS spec extensions (PRD §6 Epic 1.3 user story).

### 3.4 `unmappedButApproved`

The PolicyObject is intentionally left unmapped with a documented rationale. **Expected to be rare and noisy** (PRD §6). Each `unmappedButApproved` object is logged in the workflow's approval package and surfaces as a tier-S3 ValidationFinding so reviewers see it before publication.

### 3.5 Precedence rule

If multiple states could apply (e.g., an object both `mapsToWos` and `requiresSpecExtension`), the precedence is:

```
mapsToWos > authoringOnly > requiresSpecExtension > unmappedButApproved
```

Reasoning: mapping to existing WOS is always preferable to extending; extending is preferable to leaving unmapped; `authoringOnly` sits above extension because objects whose role is purely authorial (sources, citations, provenance) should not pretend to be runtime behavior even if they could be force-fit into a WOS field.

The four states are **mutually exclusive** at any given moment but not over time — an object may evolve from `unmappedButApproved` to `requiresSpecExtension` to `mapsToWos` as upstream WOS gains representation.

## 4. State boundaries

Studio distinguishes three state layers (PRD §7). The boundary rules are normative.

### 4.1 Session state

Ephemeral chat context, temporary candidates, scratch assumptions, draft summaries, and uncommitted AI proposals. **Not persisted** beyond the session. **MUST NOT** be cited as evidence by any approved object.

### 4.2 Workspace state

Uploaded sources, reviewed objects, mappings, workflow drafts, validation findings, scenarios, comments, and approvals. Persisted, durable, the substrate for review and iteration. Workspace state is the **only** layer in which PolicyObject lifecycles `draft → reviewed → approved → …` advance.

### 4.3 Published state

Approved WOS artifacts, scenario suites, approval packages, release notes, and exported packages. **Immutable** at the version-stamped boundary; subsequent edits produce new versions. Published artifacts are reproducible from reviewed inputs (PRD §13 Reproducibility).

### 4.4 Cross-boundary rules

- **Session → Workspace**: a session-state candidate becomes workspace-state only when explicitly committed (e.g., reviewer approves an ExtractedClaim → PolicyObject promotion).
- **Workspace → Published**: a workspace-state WorkflowIntent becomes published only when (a) tier S6 publication readiness is satisfied, (b) all required ApprovalDecisions are recorded, and (c) compilation to WOS succeeds.
- **No backwards leakage**: a published artifact never gains new content retroactively. Edits produce a new published version with a ChangeImpactReport.
- **Citations cross only one boundary**: a Published artifact may cite Workspace state (sources, policy objects, mappings, scenarios), but Workspace state may not cite Session state.

## 5. WOS concept cross-reference

The table below extends PRD §6 with concrete pointers into existing wos-spec docs. Studio object → WOS concept → spec path → schema location.

| Studio object | WOS concept | Spec path | Schema location |
|---|---|---|---|
| NoticeRequirement | Governance due-process notice | [`../specs/governance/workflow-governance.md`](../specs/governance/workflow-governance.md), [`../specs/governance/due-process-config.md`](../specs/governance/due-process-config.md) | `wos-workflow.schema.json` → `governance.dueProcess` |
| AppealRight | Governance appeal mechanism | [`../specs/governance/workflow-governance.md`](../specs/governance/workflow-governance.md) | `wos-workflow.schema.json` → `governance.appeals` |
| ExplanationRequirement | Governance explanation / reasoning / counterfactual tier | [`../specs/governance/workflow-governance.md`](../specs/governance/workflow-governance.md) | `wos-workflow.schema.json` → `governance.explanations` |
| ContinuationOfServicesRequirement | Due-process grace period / continued benefits | [`../specs/governance/due-process-config.md`](../specs/governance/due-process-config.md) | `wos-workflow.schema.json` → `governance.dueProcess.continuation` |
| CounterfactualRequirement | Counterfactual reasoning tier | [`../specs/governance/workflow-governance.md`](../specs/governance/workflow-governance.md), [`../specs/ai/ai-integration.md`](../specs/ai/ai-integration.md) | `wos-workflow.schema.json` → `governance.explanations.counterfactual` / `aiOversight` |
| DecisionRule | Lifecycle guard / `RuleReference` / external policy engine integration | [`../specs/kernel/spec.md`](../specs/kernel/spec.md), [`../specs/governance/assertion-library.md`](../specs/governance/assertion-library.md) | `wos-workflow.schema.json` → `lifecycle.transitions[].guard` |
| Deadline | Timer / temporal parameter / due-process grace period / task SLA | [`../specs/kernel/spec.md`](../specs/kernel/spec.md), [`../specs/governance/policy-parameters.md`](../specs/governance/policy-parameters.md) | `wos-workflow.schema.json` → `lifecycle.timers` / `governance.policyParameters` |
| EvidenceRequirement | caseFile field / validation pipeline / document task / reasoning evidence reference | [`../specs/kernel/spec.md`](../specs/kernel/spec.md), [`../specs/governance/workflow-governance.md`](../specs/governance/workflow-governance.md) | `wos-workflow.schema.json` → `caseFile` / `governance.validationPipelines` |
| ActorMapping (authority) | Actor declaration / delegation / override authority / separation-of-duties rule | [`../specs/kernel/spec.md`](../specs/kernel/spec.md), [`../specs/governance/workflow-governance.md`](../specs/governance/workflow-governance.md) | `wos-workflow.schema.json` → `actors[]` / `governance.delegation` |
| AI Use (an `agents`-block PolicyObject) | Agent declaration / autonomy / deontic constraint / confidence policy / fallback policy | [`../specs/ai/ai-integration.md`](../specs/ai/ai-integration.md), [`../specs/ai/agent-config.md`](../specs/ai/agent-config.md), [`../specs/ai/drift-monitor.md`](../specs/ai/drift-monitor.md) | `wos-workflow.schema.json` → `agents[]` / `aiOversight` |
| Flexible Case Phase | Advanced constraint zone | [`../specs/advanced/advanced-governance.md`](../specs/advanced/advanced-governance.md) | `wos-workflow.schema.json` → `advanced.constraintZones` |
| Scenario | WOS Tooling scenario / conformance trace | [`../specs/profiles/`](../specs/profiles/), conformance fixtures (`crates/wos-conformance`) | `wos-tooling.schema.json` → `scenarios[]` |
| ReviewerResolution | Authoring provenance / governance rationale | [`../specs/kernel/spec.md`](../specs/kernel/spec.md) §provenance, [`../specs/governance/workflow-governance.md`](../specs/governance/workflow-governance.md) | `wos-workflow.schema.json` → provenance config + `governance.rationale` |
| PolicySource / SourceCitation | RuleReference citation / source authority / authoring provenance | [`../specs/kernel/spec.md`](../specs/kernel/spec.md) §provenance, [`../specs/governance/assertion-library.md`](../specs/governance/assertion-library.md) | `wos-workflow.schema.json` → `RuleReference` / authoring provenance |
| ServiceBinding | OpenAPI / Arazzo integration binding | [`specs/binding-and-integration.md`](specs/binding-and-integration.md), `WOS-FEATURE-MATRIX.md` §12.1, §12.3 | `wos-workflow.schema.json` → `integration.bindings[*]` (binding type: `openapi-call` / `arazzo-step`) |
| EventBinding | CloudEvents kernel event | [`specs/binding-and-integration.md`](specs/binding-and-integration.md), `WOS-FEATURE-MATRIX.md` §12.2 | `wos-workflow.schema.json` → `integration.bindings[*]` (binding type: `event-consume` / `event-emit`) |
| PolicyEngineBinding | OPA / Cedar / XACML policy engine bridge | [`specs/binding-and-integration.md`](specs/binding-and-integration.md), `WOS-FEATURE-MATRIX.md` §12.5, [`../specs/ai/ai-integration.md`](../specs/ai/ai-integration.md) §4.6 | `wos-workflow.schema.json` → `integration.bindings[*]` (binding type: `policy-engine`) |
| DecisionTable (DecisionRule.form=table) | Lifecycle guard (chained FEL) | [`specs/binding-and-integration.md`](specs/binding-and-integration.md), [`specs/compiler-contract.md`](specs/compiler-contract.md) | `wos-workflow.schema.json` → `lifecycle.transitions[*].guard` (chained FEL sequence; **not DMN**) |
| WorkflowIntent | (the user-facing draft itself; compiles to `$wosWorkflow`) | [`specs/workflow-intent.md`](specs/workflow-intent.md) | not emitted directly; element kinds project per `specs/workflow-intent.md` §"WOS mappings" |
| Workspace, SourceDocument, SourceVersion, SourceSection, ExtractedClaim, Assumption, Conflict, Workflow Health Dashboard, Improvement Backlog | (no WOS counterpart) | — | `authoringOnly` — Studio-internal; never emitted to `$wosWorkflow` |
| StudioToWosMapping | (Studio control plane) | — | `authoringOnly` — the mapping record itself is metadata, not WOS content |
| ChangeImpactReport | (no WOS counterpart) | [`specs/change-impact.md`](specs/change-impact.md) | `authoringOnly` — change-management is a Studio concern; release notes derived from SemanticDiff project compactly |
| RuntimeObservation (Phase-4 placeholder) | (no current WOS counterpart) | — | `authoringOnly` — Phase-4 future-track |
| ReviewerRole, WorkspacePolicy | (no WOS counterpart) | [`specs/workspace.md`](specs/workspace.md) | `authoringOnly` — workspace metadata; role names project compactly via ApprovalDecision provenance |

Pointers into [`../schemas/`](../schemas/) reference the consolidated `wos-workflow.schema.json` per [ADR-0076 (product-tier consolidation)](../thoughts/adr/0076-product-tier-consolidation.md). Embedded blocks (`governance`, `agents`, `aiOversight`, `signature`, `custody`, `advanced`, `assurance`) live inside that single schema; their behavioral semantics live in the per-stream specs.

## 6. Open issues

The following questions are **deliberately deferred** to Stage-2 specs or later. They are listed here so reviewers see what is unsettled.

- **Extension-vs-authoring-only boundary.** When a PolicyObject's needs do not fit current WOS, is it `authoringOnly` (forever) or `requiresSpecExtension` (candidate for upstream)? Today the distinction is reviewer judgement. [`specs/studio-to-wos-mapping.md`](specs/studio-to-wos-mapping.md) will define the heuristic.
- **PolicyObject kind closure.** PRD §9.2 lists 30+ structured object kinds. Whether this list is closed at Stage 3 (schema) or open via an `x-` extension axis is unsettled. [`specs/policy-object-model.md`](specs/policy-object-model.md) will decide.
- **Scenario-to-conformance-trace identity.** A Studio Scenario is the authorable input; a WOS conformance trace is the observed output. The exact field-by-field mapping (and whether they share an id space) is not yet fixed. [`specs/scenario-authoring.md`](specs/scenario-authoring.md) will resolve.
- **Authoring provenance vs. WOS provenance.** Studio's authoring provenance (who reviewed what, when, with what rationale) overlaps WOS's runtime provenance records. The boundary at compilation — what Studio emits into the published artifact's provenance vs. what stays in Studio metadata — is unsettled. [`specs/authoring-provenance.md`](specs/authoring-provenance.md) will define.
- **RuntimeObservation data model.** Phase-4 territory; only the entity name is reserved here (see §1.17). The dedicated spec will be written when Phase 4 begins; until then, no `specs/runtime-observation.md` exists.
- **Cross-workspace reuse.** PRD §9.8 / §16 Phase-3 imply approved PolicyObjects could be reused across workspaces. The reuse model — copy, reference, federate — is unsettled. [`specs/change-impact.md`](specs/change-impact.md) will note this as Phase-3 work.

## 7. Cross-references

- Product vision: [`VISION.md`](VISION.md).
- Specs derived from this concept model: [`specs/`](specs/).
- WOS schema this product compiles to: [`../schemas/wos-workflow.schema.json`](../schemas/wos-workflow.schema.json).
- WOS tooling schema (scenarios / conformance traces): [`../schemas/wos-tooling.schema.json`](../schemas/wos-tooling.schema.json).
- Repo conventions: [`../CONVENTIONS.md`](../CONVENTIONS.md).
- Sibling product (runtime case-management UI): [`../studio/README.md`](../studio/README.md).
