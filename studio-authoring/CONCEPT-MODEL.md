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

The 34 entities below are the durable authoring vocabulary. Each entity has: a definition, the load-bearing fields, and the relationships it participates in. Field lists are illustrative — Stage 3 schemas will fix exact shapes. Per §6.1 Schema composition strategy, ~12 entities (PolicyObject kinds, Bridge kinds, Bindings) are projected as `{Studio metadata wrapper + WOS schema $ref}`, reducing Stage-3 schema count substantially. The substrate this projects to is named in §5 (WOS as canonical substrate).

**Source-of-truth notes:**
- §1.17 RuntimeObservation = the Phase-4 placeholder *entity*; §1.26 RuntimeObservationSeam = the *seam contract* (which IS specified now in `specs/runtime-observation-seam.md`, even though the entity is Phase-4).
- §1.20–§1.23 (Bindings, DecisionTable) are introduced by `specs/binding-and-integration.md`.
- §1.19 WorkflowIntent has its data-model home in `specs/workflow-intent.md`.
- §1.24 Workspace has its data-model home in `specs/workspace.md`.
- §1.27 IdentitySubject composes parent **PLN-0381** (identity attestation, P0 WOS-side commitment per [TODO.md](../TODO.md) 2026-04-27 synthesis-merge); Studio does NOT re-define attestation primitives.
- §1.32 ProtectedCategory composes parent [`specs/advanced/equity-config.md`](../specs/advanced/equity-config.md); equity semantics live in WOS advanced stream.
- §1.33 MigrationPath composes parent [`RELEASE-STREAMS.md`](../RELEASE-STREAMS.md) + [`COMPATIBILITY-MATRIX.md`](../COMPATIBILITY-MATRIX.md).
- AuthoringProvenanceRecord audit-event tags compose parent **PLN-0384** (`wos-event-types.md` taxonomy, ratifying `wos.signing.*` / `wos.identity.*` / `wos.governance.access-*` namespace; Studio adds `wos.authoring.*` namespace).
- Cryptographic anchoring composes parent [`specs/kernel/custody-hook-encoding.md`](../specs/kernel/custody-hook-encoding.md) (parent **PLN-0385**) — the four-field append wire surface.

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

An extension to the existing `DecisionRule` PolicyObject kind for multi-row decision authoring. Projects to parent kernel `decisionTables[*]` + a `DecisionTableGuard` on the relevant transition (Kernel §4.5.1; landed 2026-05-01). Round-trip is structurally lossless — row ids preserved through projection. **No DMN export** per parent CLAUDE.md; one-way DMN import is supported via FEEL→FEL transpilation.

**Key fields:** `form: "table"`, `inputs[]`, `outputs[]`, `rows[]`, `hitPolicy` (first-match | priority | unique | output-merge), `completenessRequirement`, `fallback?`.
**Relationships:** referenced by `decision` WorkflowElements; projects to `$.decisionTables[*]` (table catalog) + `$.lifecycle.transitions[*].guard` of form `DecisionTableGuard`.

### 1.24 Workspace (extended; data model home: [`specs/workspace.md`](specs/workspace.md))

The bounded authoring environment that owns one or more workflows. The data model is detailed in `specs/workspace.md`; this entry summarizes the entities the Workspace owns:

**Owned entities:** ReviewerRole registry (workspace-scoped), WorkspacePolicy (administrator-configured behavior), WorkspaceAuditLogEntry (queryable view over AuthoringProvenanceRecords + non-provenance events), permissions surface, identity model.
**Lifecycle:** `created → active → { archived | suspended }`; `suspended → active`; `archived` is terminal (read-only).

### 1.25 Effectiveness (data model home: [`specs/effectiveness-and-applicability.md`](specs/effectiveness-and-applicability.md))

A single composable object describing **when and where** a SourceVersion / PolicyObject / Mapping applies. SourceVersion / PolicyObject (where appropriate) / Mapping carry an `effectivenessRef` pointing to a single canonical Effectiveness instance — never copied — so that updating jurisdictional or temporal scope is one edit, not three. Models court injunctions in single circuits, errata memos that supersede paragraphs, and partial-supersession transitions that policy actually exhibits.

**Key fields:** `id`, `jurisdictions[]` (e.g., `{kind: "federal" | "state" | "circuit" | "district" | "local", code, displayName}`), `temporalScope` (effective intervals; sunsets), `appellateState` (`final` | `on-appeal` | `enjoined` | `provisional`), `enjoinedScope?` (when appellateState=enjoined: which jurisdictions are enjoined), `supersedingRef?`, `supersededBy?`, `notesRef?`.
**Relationships:** referenced by SourceVersion (every version), PolicyObject (where applicability matters), Mapping (where applicability narrows or widens); cited by Conflict resolution; tracked in ChangeImpactReport when shape changes.

### 1.26 RuntimeObservationSeam (data model home: [`specs/runtime-observation-seam.md`](specs/runtime-observation-seam.md))

The **seam contract** by which Studio receives runtime case-trace observations from the runtime tier. Implementation deferred to Phase 4; the contract itself is named now to close dangling references in `change-impact.md` (`triggerKind = runtime-observation-cluster`), `authoring-provenance.md` (`originClass = runtime-observed`), and the prior §1.17 placeholder.

**Key fields:** `wireFormat` (case trace: caseId, eventSequence, decisions, manualOverrides, timeBuckets), `ingestPath` (subscription | poll | batch), `triggers[]` (cluster heuristics that produce ChangeImpactReport entries), `replayContract` (replay against the published workflow for divergence detection).
**Relationships:** observation-cluster events trigger ChangeImpactReport (§1.18); observed traces become candidate Scenarios (per `scenario-authoring.md` runtime-observation-replay scenario type); `runtime-observed` claims become PolicyObjects via the same review pipeline as source-derived claims.

### 1.27 IdentitySubject (data model home: [`specs/identity-and-attestation.md`](specs/identity-and-attestation.md))

The Studio-side surface for identity claims that authorize authoring actions. Composes parent `PLN-0381` (identity attestation as P0 WOS-side commitment, supersedes prior PLN-0310) — Studio does NOT re-define attestation primitives; it references and binds to them.

**Key fields:** `subjectId`, `claims[]` (signed claim envelope), `attestationRef` (reference to parent attestation primitive), `validFrom/validUntil`, `revocation?`, `bindingScope` (workspace | workflow | object).
**Relationships:** referenced by every AuthoringProvenanceRecord (`recordedBy` resolves to a SubjectId); referenced by ApprovalDecision (`signatureRef`); cross-referenced from `wos-event-types.md` `wos.identity.*` namespace (parent PLN-0384).

### 1.28 ComplianceAttestation (section in [`specs/workspace.md`](specs/workspace.md); attached to ApprovalPackage in [`specs/review-and-approval.md`](specs/review-and-approval.md))

A structured attestation that a Workspace's policies and a workflow's published artifact satisfy a named compliance regime (SOC 2 type II, FedRAMP Moderate/High, StateRAMP Moderate/High, NIST 800-53 Rev. 5 control families, HIPAA, GDPR DPIA, etc.).

**Key fields:** `regime`, `regimeVersion`, `controls[]` (`{controlId, controlName, status, evidenceRef, attestor}`), `attestedAt`, `expiresAt?`, `auditorRef?`.
**Relationships:** attaches to Workspace (the workspace's controls baseline) and to ApprovalPackage (the workflow's compliance derivation); does not appear in `$wosWorkflow` body content; projects compactly in release notes.

### 1.29 AuthorityGrant (section in [`specs/workspace.md`](specs/workspace.md))

A workspace-administrator-issued grant authorizing a specific role/subject to perform a specific authoring action — finer-grained than ReviewerRole. Examples: "compliance-reviewer may attest `originClass = local-practice` for Workspace W"; "workflow-owner may waive `WF-LINT-001` block findings on rights-impacting workflows".

**Key fields:** `grantId`, `grantedTo` (RoleRef | SubjectId), `action` (`attestOrigin:{class}` | `waive:{ruleId}` | `override:{category}` | `approve:{subjectKind}`), `scope` (workspace | workflow-class | per-object), `grantedBy`, `grantedAt`, `revokedAt?`, `expiresAt?`.
**Relationships:** consulted by readiness engine when resolving "who may waive" / "who may attest" decisions; emits AuthoringProvenanceRecord on every grant/revoke; appears in compliance audit trail.

### 1.30 TerminologyMap (data model home: [`specs/terminology-and-canonical-vocabulary.md`](specs/terminology-and-canonical-vocabulary.md))

Canonical-term registry that resolves cross-workspace DataElement identity. When two workspaces both have "household income," the TerminologyMap declares whether they're the same canonical term. Also carries the **plain-English projection** for every CONCEPT-MODEL entity (Sarah's "I won't learn 23 entities" concern as a structural artifact).

**Key fields:** `canonicalId`, `displayName`, `definition`, `synonyms[]`, `dataType`, `sensitivity` (DPV controlled vocabulary; see §1.32 footnote on DPV), `dprUnit?` (units of measure), `references[]` (URIs to canonical definitions, e.g., USDA SNAP terminology, HHS HIPAA terminology, DPV concepts), `plainEnglishLabel?`, `operationalDescription?`.
**Relationships:** every Studio DataElement and entity carries an optional `canonicalTermRef`; cross-workspace federation (§1.34) requires terminology agreement; reviewer-friendly UI renders entities via `plainEnglishLabel`.

### 1.31 CanonicalSourceRef (extension in [`specs/source-vault.md`](specs/source-vault.md))

A reference pattern letting a SourceDocument declare it IS a canonical document (e.g., "this workspace's copy of 7 CFR §273") OR REFERENCES a canonical document published authoritatively elsewhere (e.g., eCFR.gov's JSON-LD-published version of 7 CFR §273). A federation precursor — full federation is §1.34 (deferred); CanonicalSourceRef is the cite-don't-copy primitive that 50 states can share.

**Key fields:** `referencedUri`, `referencedHash?`, `referencedAt`, `localCopyOf?` (when the source is a local archive of the canonical), `canonicalPublisher?`, `jsonLdContext?` (when canonical is JSON-LD published).
**Relationships:** attached to SourceDocument; consumed by reviewers verifying citation integrity; consumed by ChangeImpactReport when canonical publisher updates.

### 1.32 ProtectedCategory (PolicyObject kind defined in [`specs/policy-object-model.md`](specs/policy-object-model.md))

A demographic category for which the workflow conducts equity monitoring (Title VI race/ethnicity, ADA disability, ECOA-protected groups, language-spoken, jurisdictional sub-population). Sourced from compliance/policy documents (e.g., USDA Title VI regs at 7 CFR §15.2); maps to the parent `wos-workflow.schema.json` `advanced.equity.protectedCategories[*]` per `specs/advanced/equity-config.md`. Studio is the authoring layer; equity semantics live in the WOS advanced stream — Studio does NOT re-implement.

**Key fields:** `categoryId`, `dimensionName` (race/ethnicity/disability/language/...), `legalBasis` (citationRef), `monitoringMethod`, `disparityThreshold?`, `remediationTriggerRef?` (Deadline + Notice combination when threshold crossed).
**Relationships:** referenced by equity-probe Scenarios (§1.13); cited by ProtectedCategory readiness rules in `readiness-validation.md`; projects to `advanced.equity.protectedCategories[*]` in `$wosWorkflow`.

> **DPV (Data Privacy Vocabulary) footnote.** Studio's DataElement `sensitivity` field uses [W3C DPV](https://www.w3.org/TR/dpv/) controlled-vocabulary IRIs (e.g., `dpv:HealthData`, `dpv:FinancialPreference`, `dpv:Identifier`) instead of a hand-rolled `pii | phi | restricted` enum. This unlocks: (a) automated retention/access-policy derivation from DPV's policy machinery, (b) GDPR/CCPA/HIPAA mapping, (c) interop with privacy-engineering tools that already speak DPV. Hand-rolled equivalents (`pii`/`phi`/`restricted`) remain as machine-readable aliases for backward continuity but the DPV IRI is the canonical truth.

### 1.33 MigrationPath (cross-cutting; concept lives in [`specs/compiler-contract.md`](specs/compiler-contract.md), [`specs/workflow-intent.md`](specs/workflow-intent.md), [`specs/change-impact.md`](specs/change-impact.md))

The contract by which a published WorkflowIntent migrates when WOS schemas evolve. Composes parent [`RELEASE-STREAMS.md`](../RELEASE-STREAMS.md) + [`COMPATIBILITY-MATRIX.md`](../COMPATIBILITY-MATRIX.md). Studio's responsibility is **tracking and alerting** (when a published workflow targets a deprecated stream version, surface a SchemaVersionDeprecationAlert); the versioning model itself is parent-defined.

**Key fields:** `wosVersionPin` (claim string per RELEASE-STREAMS.md, e.g., `kernel@1.0, governance@1.0, ai@0.5, signature@1.0, custody@1.0, advanced@0.3`), `pinnedAt`, `deprecationAlerts[]`, `migrationStrategy` (regenerate | preserve | manual-review).
**Relationships:** stored on WorkflowIntent; carried in compile manifest (per `compiler-contract.md`); triggers ChangeImpactReport via `triggerKind = wos-version-deprecation`.

### 1.34 FederationLink (reserved; concept noted in [`specs/workspace.md`](specs/workspace.md))

**Status:** Deferred. Reserved as `x-federation` extensibility slot in `workspace.md`. Documents the future capability for one Workspace in Tenant A to declare a dependency on a SourceDocument or PolicyObject from Tenant B (e.g., "this workflow implements 7 CFR §273 as published by USDA; track upstream changes"). When implementation begins, this entity gains its own data model home.

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

## 5. WOS as canonical substrate

Studio's authoring vocabulary (the 16 user-facing WorkflowElement kinds, the PolicyObject family, Scenarios, Bindings, Workspace, ReviewerResolution, etc.) is **target-neutral and authoring-domain**. The compile step projects that authoring vocabulary into a single canonical substrate: the consolidated WOS workflow envelope (`wos-workflow.schema.json` per [ADR-0076](../thoughts/adr/0076-product-tier-consolidation.md)). This section names that choice as a load-bearing principle and defines why it does not constrain low-risk workflows or alternative runtime targets.

### 5.1 Why one substrate, not many

Studio does not maintain a pluggable projection-target registry. The advisor framing — "project into the appropriate artifact shape for the workflow's risk profile, runtime target, and implementation context" — is satisfied **inside** the WOS substrate, not by swapping substrates:

- **Risk-profile differentiation is internal to WOS.** Per [ADR-0076](../thoughts/adr/0076-product-tier-consolidation.md), the embedded blocks (`governance`, `agents`, `aiOversight`, `signature`, `custody`, `advanced`, `assurance`) are **conditionally required** by `impactLevel`. A workflow with `impactLevel: "operational"` projects to a thin `$wosWorkflow` carrying kernel + lifecycle and no other blocks; a `rights-impacting` workflow projects to a fuller envelope. The risk-appropriate output shape emerges from block presence, not from selecting a different substrate.
- **Runtime-target neutrality is inherited from WOS.** Parent [`../CLAUDE.md`](../../CLAUDE.md) fixes WOS as execution-substrate-agnostic — Temporal, Restate, Camunda, and Step Functions are all valid `DurableRuntime` adapters. Studio gets execution-target plurality for free; it does not need to encode adapter selection in the authoring layer.
- **Implementation context is carried in workspace metadata.** ServiceBinding, EventBinding, PolicyEngineBinding, and DecisionTable (CM §1.20–§1.23) carry the workspace-local "how this connects to our infrastructure" detail. None of that bleeds into the WOS artifact's *semantics*; it lands in `integration.bindings[*]`, where the binding shape is the same regardless of deployment context.

The result: one substrate, three orthogonal axes of variation (risk profile via embedded-block conditionality, runtime target via adapter selection, implementation context via bindings).

### 5.2 What "doesn't restrict you" means concretely

A non-rights-impacting workflow authored in Studio compiles to a thin `$wosWorkflow` with no `governance`, no `agents`, no `aiOversight`, no `signature`, no `custody`, no `advanced`, no `assurance` blocks — just the kernel envelope (`url`, `version`, `title`, `impactLevel`, `actors`, `lifecycle`) and any required `caseFile` / `integration` content. The full WOS apparatus is not dragged along; it is opt-in by `impactLevel` and by author intent. This is the property that makes WOS-as-substrate non-restrictive.

`compiler-contract.md` is normative on the thin-projection path; this section frames it conceptually.

### 5.3 What stays in the workspace (authoring vocabulary that does not project)

Studio's authoring vocabulary is intentionally broader than WOS. Several PolicyObject kinds and entities have no WOS counterpart and are explicitly `authoringOnly` per CM §3.2:

- **Source-and-authority**: PolicySource, AuthorityRank, ApplicabilityScope, EffectivePeriod (the latter two also surface as `requiresSpecExtension` candidates per `studio-to-wos-mapping.md`).
- **Review-and-uncertainty**: Assumption, Conflict, Supersession, ReviewerResolution.
- **Workspace control plane**: StudioToWosMapping itself, AuthorityGrant, ComplianceAttestation (the workspace-level baseline; ApprovalPackage carries per-workflow derivations), Workspace, SourceDocument, SourceVersion, SourceSection, ExtractedClaim, ChangeImpactReport, Workflow Health Dashboard, Improvement Backlog.

These exist because rights-impacting authoring requires modeling things WOS does not need to evaluate at runtime — *who said what, when, with what authority, what we assumed, what was superseded*. They project as **provenance and rationale** alongside the artifact (via ApprovalPackage citations, AuthoringProvenanceRecords, and reviewer-resolution tags), not as schema content. This separation is enforced by the four mapping states (CM §3); the precedence rule keeps the substrate clean.

### 5.4 When this principle would need to change

This single-substrate posture is correct **as long as**:

1. WOS continues to scale down (kernel-only workflows remain a first-class shape; embedded-block conditionality holds).
2. WOS remains execution-substrate-agnostic (no kernel feature ties to a specific runtime).
3. No Studio authoring concept arises that *cannot* be carried as either a projected WOS field, an `authoringOnly` workspace artifact, a `requiresSpecExtension` candidate, or workspace-local provenance.

If any of those breaks, this section is the right place to revisit. Until then, "WOS is the canonical substrate" is the architectural simplifier that lets Studio stay focused on authoring discipline rather than projection-target management.

## 6. WOS concept cross-reference

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
| DecisionTable (DecisionRule.form=table) | First-class kernel decision table per parent Kernel §4.5.1 | [`specs/binding-and-integration.md`](specs/binding-and-integration.md), [`specs/compiler-contract.md`](specs/compiler-contract.md), [`../specs/kernel/spec.md`](../specs/kernel/spec.md) §4.5.1 | `wos-workflow.schema.json` → `decisionTables[*]` (catalog) + `lifecycle.transitions[*].guard` of form `DecisionTableGuard`; row ids preserved; **no DMN export** |
| WorkflowIntent | (the user-facing draft itself; compiles to `$wosWorkflow`) | [`specs/workflow-intent.md`](specs/workflow-intent.md) | not emitted directly; element kinds project per `specs/workflow-intent.md` §"WOS mappings" |
| Workspace, SourceDocument, SourceVersion, SourceSection, ExtractedClaim, Assumption, Conflict, Workflow Health Dashboard, Improvement Backlog | (no WOS counterpart) | — | `authoringOnly` — Studio-internal; never emitted to `$wosWorkflow` |
| StudioToWosMapping | (Studio control plane) | — | `authoringOnly` — the mapping record itself is metadata, not WOS content |
| ChangeImpactReport | (no WOS counterpart) | [`specs/change-impact.md`](specs/change-impact.md) | `authoringOnly` — change-management is a Studio concern; release notes derived from SemanticDiff project compactly |
| RuntimeObservation (Phase-4 placeholder) | (no current WOS counterpart) | — | `authoringOnly` — Phase-4 future-track |
| ReviewerRole, WorkspacePolicy | (no WOS counterpart) | [`specs/workspace.md`](specs/workspace.md) | `authoringOnly` — workspace metadata; role names project compactly via ApprovalDecision provenance |

Pointers into [`../schemas/`](../schemas/) reference the consolidated `wos-workflow.schema.json` per [ADR-0076 (product-tier consolidation)](../thoughts/adr/0076-product-tier-consolidation.md). Embedded blocks (`governance`, `agents`, `aiOversight`, `signature`, `custody`, `advanced`, `assurance`) live inside that single schema; their behavioral semantics live in the per-stream specs.

## 6.1 Schema composition strategy (Stage-3 design)

A naive Stage-3 implementation would author one JSON Schema per CONCEPT-MODEL entity — ~33 schemas. That is wasteful where the entity's *structural* content is already defined by a parent WOS schema. This section defines the layered-view design that Stage-3 follows, reducing the Studio Stage-3 schema count from ~33 to ~10 by composition.

### The pattern

For every Studio entity that **projects to a WOS schema target**, the Stage-3 schema is:

```text
StudioEntity = {
  studioMetadataEnvelope: {  // Studio-only
    id, workspaceId, version, parentVersion?,
    citations[], provenance, originClass,
    lifecycleState, reviewState,
    mappingRef, mappingState,
    canonicalTermRef?,        // §1.30 TerminologyMap
    effectivenessRef?,        // §1.25 Effectiveness
    authorityGrantsApplied[]  // §1.29 AuthorityGrant
  },
  wosTargetContent: $ref → wos-workflow.schema.json#/$defs/<TargetType>
}
```

The structural truth lives in `wos-workflow.schema.json` (the WOS schema's `$defs`). Studio's schema declares the *envelope* + the *reference*. When the WOS schema evolves a target shape (e.g., a new field on `governance.notices[*]`), Studio inherits automatically; no Studio schema edit needed for backward-compatible additions.

### Composition table

| Studio entity | Composition strategy | Stage-3 schema |
|---|---|---|
| **PolicyObject kinds projecting to WOS** (NoticeRequirement, AppealRight, ExplanationRequirement, Deadline, ActorMapping, EvidenceRequirement, Outcome, DecisionRule) | Layered view: envelope + `$ref` to wos-workflow.schema.json `$defs` | **1 polymorphic schema:** `wos-studio-policy-object.schema.json` (oneOf discriminated by `kind`) |
| **Bridge kinds** (WorkflowStepMapping, LifecycleTagMapping, TransitionMapping, TimerMapping, TaskMapping, CaseFileMapping) | Layered view: envelope + `$ref` to wos-workflow.schema.json target paths | Folded into `wos-studio-policy-object.schema.json` |
| **Bindings** (ServiceBinding, EventBinding, PolicyEngineBinding) | Layered view: envelope + `$ref` to `wos-workflow.schema.json#/integration/bindings[*]` shape | **1 schema:** `wos-studio-binding.schema.json` |
| **Studio-only PolicyObject kinds** (PolicySource, AuthorityRank, ApplicabilityScope, EffectivePeriod, Supersession, Conflict, Assumption, OpenQuestion, ProtectedCategory) | Studio-defined — no WOS counterpart | Folded into `wos-studio-policy-object.schema.json` (the `oneOf` extends to cover authoring-only kinds) |
| **WorkflowIntent** | Genuinely different from `$wosWorkflow` (16 user-facing element kinds + bridges + authoring metadata) | **1 schema:** `wos-studio-workflow-intent.schema.json` |
| **Scenario** | Layered view: envelope + `$ref` to `wos-tooling.schema.json` `scenarios[*]` shape | **1 schema:** `wos-studio-scenario.schema.json` |
| **AuthoringProvenanceRecord** | Studio-defined; AI-extraction subtype + audit event-type tags compose parent **PLN-0384** `wos-event-types.md` | **1 schema:** `wos-studio-provenance.schema.json` |
| **DecisionTable** (the table form of DecisionRule) | Studio-defined authoring shape (rows + hit policy + completeness) projects 1:1 to parent kernel `decisionTables[*]` + `DecisionTableGuard` on the relevant transition (Kernel §4.5.1, landed 2026-05-01). Round-trip lossless; row ids preserved. Scenarios cite row ids directly. | Folded into `wos-studio-policy-object.schema.json` |
| **Source vault** (SourceDocument, SourceVersion, SourceSection, SourceCitation, ExtractedClaim, CanonicalSourceRef) | Studio-defined; canonical sources may carry JSON-LD `@context` + content (when source is published in JSON-LD, e.g., eCFR.gov) | **1 schema:** `wos-studio-source.schema.json` |
| **Workspace + ReviewerRole + WorkspacePolicy + AuthorityGrant + ComplianceAttestation** | Studio-defined | **1 schema:** `wos-studio-workspace.schema.json` |
| **ApprovalDecision + ApprovalPackage + ChangeImpactReport** | Studio-defined; approval-package composes (or attaches) Effectiveness + ComplianceAttestation | **1 schema:** `wos-studio-approval.schema.json` |
| **ValidationFinding + readiness rule registry** | Studio-defined | **1 schema:** `wos-studio-readiness.schema.json` |
| **Effectiveness, IdentitySubject, TerminologyMap, MigrationPath** | Studio-defined; Effectiveness is referenced by ref (one canonical home, never copied) | **1 schema each (4 small):** `wos-studio-effectiveness.schema.json`, `wos-studio-identity-subject.schema.json`, `wos-studio-terminology-map.schema.json`, `wos-studio-migration-path.schema.json` |

Net Stage-3 Studio schemas: **~10** (from ~33 naive). Studio inherits WOS structural truth via `$ref` everywhere a target exists.

### Slight WOS-side extension proposals (queued)

Two minor WOS-side extensions would further reduce Studio complexity. Listed as **ExtensionRecord candidates** in [`specs/studio-to-wos-mapping.md`](specs/studio-to-wos-mapping.md); not implemented Studio-side until WOS-side ratifies:

1. **`wos-tooling.scenarios[*].decisionTable`** — a row-coverage shape so that a DecisionTable's rows can each be traced to scenario coverage. Today scenarios reference guard expressions; with this extension, a DecisionTable's row #3 can be marked "covered by scenario S-007."
2. **`wos-workflow.x-wos-studio`** — a reserved extension envelope on the published `$wosWorkflow` artifact where Studio's compact provenance / citation manifest projects. Today this content is in the ApprovalPackage; a reserved slot in the artifact itself would allow downstream consumers to fetch it without a separate package retrieval.

Both are within ADR-0077's `x-` extensibility patternProperties; neither requires a new kernel seam.

### External-standard reuse (selective)

| Standard | Adopted? | Where | What it unlocks |
|---|---|---|---|
| **W3C JSON-LD** | YES | `source-vault.md` ingest path | Native ingest of regs published in JSON-LD (eCFR.gov, increasingly common); preserves semantic context across re-parsing |
| **W3C DPV (Data Privacy Vocabulary)** | YES | `policy-object-model.md` DataElement.sensitivity | Replaces hand-rolled `pii \| phi \| restricted` enum; unlocks GDPR/CCPA/HIPAA mapping; interop with privacy-engineering tools |
| **W3C PROV-O** | YES | `authoring-provenance.md` export format | First-class auditor interop (auditors already understand PROV-O); leverages existing `wos-ontology-alignment.schema.json` sidecar |
| **OASIS LegalRuleML** | YES | `policy-object-model.md` deontic kinds | Direct on-the-wire shape for Obligation/Permission/Prohibition/Right; legal-tech tool interop; already cited in parent CLAUDE.md |
| **DMN one-way import** | YES | `binding-and-integration.md` import binding kind | Procurement-friendly: many state agencies have DMN tables; one-way import → DecisionTable → FEL chain at compile. **No DMN export** (rejection stands per CLAUDE.md). |
| **BPMN one-way import** | DEFERRED | — | Lossy + large transpiler; revisit when concrete agency demand surfaces |
| **W3C SHACL** | DEFERRED | — | Reconsider when JSON-LD ingest matures + an agency publishes shapes |
| **RDF / Turtle export** | DEFERRED | — | JSON-LD covers the export need; revisit only if a customer needs Turtle |
| **AsyncAPI** | REJECTED (parent CLAUDE.md) | — | Superseded by CloudEvents; rejection holds |
| **DMN export, FEEL, SHACL as authority languages** | REJECTED (parent CLAUDE.md) | — | FEL is the WOS authority; rejection holds for *export* and *authority*; only one-way *import* paths are adopted |

## 7. Open issues

The following questions are **deliberately deferred** to Stage-2 specs or later. They are listed here so reviewers see what is unsettled.

- **Extension-vs-authoring-only boundary.** When a PolicyObject's needs do not fit current WOS, is it `authoringOnly` (forever) or `requiresSpecExtension` (candidate for upstream)? Today the distinction is reviewer judgement. [`specs/studio-to-wos-mapping.md`](specs/studio-to-wos-mapping.md) will define the heuristic.
- **PolicyObject kind closure.** PRD §9.2 lists 30+ structured object kinds. Whether this list is closed at Stage 3 (schema) or open via an `x-` extension axis is unsettled. [`specs/policy-object-model.md`](specs/policy-object-model.md) will decide.
- **Scenario-to-conformance-trace identity.** A Studio Scenario is the authorable input; a WOS conformance trace is the observed output. The exact field-by-field mapping (and whether they share an id space) is not yet fixed. [`specs/scenario-authoring.md`](specs/scenario-authoring.md) will resolve.
- **Authoring provenance vs. WOS provenance.** Studio's authoring provenance (who reviewed what, when, with what rationale) overlaps WOS's runtime provenance records. The boundary at compilation — what Studio emits into the published artifact's provenance vs. what stays in Studio metadata — is unsettled. [`specs/authoring-provenance.md`](specs/authoring-provenance.md) will define.
- **RuntimeObservation data model.** Phase-4 territory; only the entity name is reserved here (see §1.17). The dedicated spec will be written when Phase 4 begins; until then, no `specs/runtime-observation.md` exists.
- **Cross-workspace reuse.** PRD §9.8 / §16 Phase-3 imply approved PolicyObjects could be reused across workspaces. The reuse model — copy, reference, federate — is unsettled. [`specs/change-impact.md`](specs/change-impact.md) will note this as Phase-3 work.

## 8. Cross-references

- Product vision: [`VISION.md`](VISION.md).
- Specs derived from this concept model: [`specs/`](specs/).
- WOS schema this product compiles to: [`../schemas/wos-workflow.schema.json`](../schemas/wos-workflow.schema.json).
- WOS tooling schema (scenarios / conformance traces): [`../schemas/wos-tooling.schema.json`](../schemas/wos-tooling.schema.json).
- Repo conventions: [`../CONVENTIONS.md`](../CONVENTIONS.md).
- Sibling product (runtime case-management UI): [`../studio/README.md`](../studio/README.md).
