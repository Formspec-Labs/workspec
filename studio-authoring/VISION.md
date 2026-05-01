# WOS Studio (Authoring) — VISION

**Status:** draft — vision
**Date:** 2026-04-30
**Folder:** `studio-authoring/`
**Branch:** `claude/wos-studio-setup-zFFDC`

## How to read this document

This is the durable product vision for **WOS Studio**, the non-technical authoring / review / validation / change-management layer for WOS workflows. It is a transcription of the original PRD with a short framing preface. Section numbers (1–19) match the PRD as authored so external references survive.

## Relationship to existing `studio/`

There is already a top-level `studio/` folder in this repo. That folder is a **runtime case-management UI** built on React 19 / Vite / Express. It consumes `$wosWorkflow` documents and fixtures and exposes inbox, workflow designer, audit trail, applicant portal, and report-builder surfaces. It is a *reference implementation that runs published workflows*.

This folder, `studio-authoring/`, is a **sibling product**. Different audience (program managers, policy/legal/compliance staff, service designers — not case workers). Different problem (transforming source documents and institutional knowledge into reviewed, source-backed, WOS-aligned workflow specifications — not running cases against an already-published workflow). The two products share a name family but have no runtime dependency on each other. See `../studio/README.md` for the case-management product.

## Relationship to wos-spec

WOS Studio (Authoring) is **wos-spec dependent**. Its terminal output is a formal `$wosWorkflow` document plus tooling/scenario artifacts that conform to the schemas published from this repo:

- `../schemas/wos-workflow.schema.json` — the author-time core (with embedded `governance`, `agents`, `aiOversight`, `signature`, `custody`, `advanced`, `assurance` blocks).
- `../schemas/wos-tooling.schema.json` — for scenario / conformance trace artifacts.
- `../specs/kernel/spec.md`, `../specs/governance/`, `../specs/ai/`, `../specs/advanced/`, `../specs/profiles/`, `../specs/sidecars/` — the normative behavioral specs that Studio's mappings target.

Studio does not invent a parallel semantics layer. Every approved Studio object either maps to an existing WOS concept, is explicitly marked authoring-only, identifies a need for a controlled extension, or is unmapped-but-approved with a documented rationale (PRD §6).

## Boundary at this stage

This folder is **docs-only**. There is no `package.json`, no Cargo workspace entry, no `Makefile` integration, no schemas, no compiler, no scenario engine, no UI. The Implementation Roadmap (§17) gates those behind later stages. Stage 0 (this VISION) and Stages 1–2 (Concept Model and Specs) come first; tech posture is deliberately deferred until the specs stabilize. This mirrors how `kernel/`, `governance/`, `ai/`, and `advanced/` were spec-first in this repo.

## Decision record

- **Folder name:** `studio-authoring/` chosen over `wos-studio/` to avoid visual collision with the existing `studio/` folder in command-line and path contexts.
- **PRD location:** kept inside the product folder (`studio-authoring/VISION.md`) following the `crates/wos-server/VISION.md` pattern referenced in the parent `CLAUDE.md` "Read first" table. A short pointer lives at `thoughts/plans/2026-04-30-studio-authoring-product-pointer.md` for discoverability.
- **Stages 3+:** schemas, lint engine, compiler, scenario engine, reference architecture, vertical slice — all deferred to subsequent turns.

---

<!-- BEGIN PRD TRANSCRIPTION -->

# WOS Studio Product Requirements Document and Roadmap

## Executive Summary

WOS Studio is a source-backed workflow intelligence platform for WOS. It helps non-technical teams transform policy, procedures, forms, operational documents, and institutional knowledge into reviewed structured interpretations, validated workflow designs, durable scenarios, and formal WOS artifacts.

The product should feel like a policy-aware workflow copilot and review environment, not a technical state-machine editor. Most users should work with concepts like sources, requirements, notices, deadlines, appeals, decisions, reviews, evidence, roles, assumptions, and scenarios. WOS internals should remain available to technical users through expert mode, exports, mappings, and validation details.

The central product promise is simple:

> **WOS Studio lets non-technical teams extract policy meaning, approve structured interpretations, map them onto WOS semantics, validate scenarios, and manage workflow change over time.**

Studio should be built with the same disciplined pattern used for WOS-spec and Formspec: a comprehensive vision document, a concept model, W3C-style internal specs, schemas, readiness rules, compiler/runtime tools, reference architecture, and vertical slices. The difference is that Studio begins in an interpretive world: messy source documents, incomplete context, conflicting guidance, and human review. Its publication boundary must become deterministic, WOS-aligned, and scenario-tested.

---

# 1. Product Thesis

WOS Studio is the non-technical authoring, review, validation, and change-management layer for WOS workflows.

It is not a generic workflow builder, BPMN clone, knowledge base, or JSON editor with a chatbot attached. It is a system for transforming institutional policy and operational knowledge into workflows that are:

* source-backed;
* reviewable;
* explainable;
* WOS-aligned;
* scenario-tested;
* change-aware;
* suitable for formal export and downstream technical implementation.

The structured policy model in Studio is a human-facing authoring and traceability layer. It should not become a competing semantic system. Approved Studio objects should either map to existing WOS concepts, remain explicitly authoring-only, or identify a need for a controlled extension.

## Core product flow

```text
Sources
→ Studio Structured Policy Objects
→ Studio-to-WOS Mappings
→ Workflow Intent Model
→ WOS Artifact
→ Validation + Scenario Simulation
→ Review + Approval
→ Publish / Integrate
→ Observe Runtime
→ Iterate
```

## Development artifact flow

Studio should be developed through a spec-and-tooling pipeline similar to WOS-spec and Formspec, with an added interpretation and concept-model layer.

```text
Vision / PRD
→ Studio Concept Model
→ Studio Specs
→ Studio Schemas
→ Studio Readiness Rules
→ Studio-to-WOS Compiler
→ Scenario Runtime
→ Reference Architecture
→ Vertical Slices
→ Iteration
```

Studio should be flexible before review, structured after review, and deterministic at publication.

---

# 2. Product Goals

## Primary goal

Enable non-technical teams to create, validate, review, and iterate WOS-aligned workflows from source-backed policy and operational knowledge.

## Secondary goals

* Reduce ambiguity between policy and process.
* Preserve traceability from source material to workflow behavior.
* Make workflow design reviewable by policy, legal, compliance, operations, and technical stakeholders.
* Produce formal WOS artifacts without requiring most users to understand WOS internals.
* Support scenario-based validation and change impact analysis.
* Create a durable bridge between workflow design and runtime observation.
* Provide a repeatable development path from product vision to specs, schemas, validation rules, compiler behavior, and reference architecture.

---

# 3. Primary Users

## Program and operations managers

Need to model, review, and improve workflows without writing code or directly editing formal specifications.

## Policy, legal, and compliance staff

Need to verify that workflows reflect authoritative requirements, deadlines, notices, appeal rights, review obligations, source hierarchy, and policy caveats.

## Service designers and business analysts

Need to translate operational processes into clear, structured workflows that can be reviewed by stakeholders.

## Technical implementers

Need validated WOS artifacts, mappings, scenarios, and conformance-oriented outputs that can be integrated downstream.

## Governance reviewers and executives

Need concise reports, approval packages, change summaries, risk visibility, and audit trails.

---

# 4. Jobs To Be Done

Users need to:

1. Upload policy and operational source materials.
2. Extract structured requirements and operational concepts from those materials.
3. Review, correct, and approve extracted interpretations.
4. Resolve assumptions, ambiguities, and source conflicts.
5. Map approved interpretations to WOS concepts.
6. Generate a workflow draft.
7. Understand the workflow visually and in plain language.
8. Validate whether the workflow is complete, consistent, source-backed, and WOS-conformant.
9. Simulate typical and edge-case scenarios.
10. Compare workflow versions and understand policy-change impacts.
11. Approve and publish formal workflow artifacts.
12. Eventually compare designed workflows against runtime behavior.

---

# 5. Product Principles

## 1. Source-backed by default

Every meaningful workflow behavior should trace to source material or an explicitly approved assumption.

## 2. WOS-aligned, not WOS-exposed

Studio should map to WOS semantics without forcing most users to understand WOS internals.

## 3. Review before authority

AI extraction and workflow generation are drafts until reviewed and approved.

## 4. Validation is layered

Source validity, policy coherence, mapping completeness, workflow correctness, WOS conformance, and scenario behavior are different checks.

## 5. Scenarios are first-class

A workflow is not credible until key real-world scenarios are exercised and reviewed.

## 6. Change impact is core

Policy changes should propagate through source objects, mappings, workflows, scenarios, releases, and eventually runtime observations.

## 7. Graph inside, plain language outside

Use a graph for traceability and impact analysis. Expose questions and answers, not graph mechanics.

## 8. Studio objects must be projection-friendly

Every approved Studio object should either map to an existing WOS concept or be explicitly marked as authoring-only.

## 9. AI proposes; the system records; humans approve

The AI copilot can extract, draft, summarize, and propose. Durable workflow behavior requires structured records, validation, and review.

## 10. Deterministic publication boundary

Once reviewed inputs are fixed, compilation to WOS artifacts and validation outputs should be deterministic and reproducible.

---

# 6. Studio-to-WOS Mapping Contract

Studio maintains higher-level authoring objects for non-technical users. Each approved object must declare how it maps to WOS, or why it remains authoring-only.

## Mapping states

```text
mapsToWos
  Object maps to one or more WOS concepts or JSON paths.

authoringOnly
  Object supports review, explanation, or change management but is not emitted into WOS.

requiresSpecExtension
  Object captures a need not currently represented by WOS and may require an extension.

unmappedButApproved
  Object is intentionally left unmapped with a documented rationale. This should be rare and noisy.
```

## Example mappings

| Studio object           | WOS mapping                                                                                        |
| ----------------------- | -------------------------------------------------------------------------------------------------- |
| Notice Requirement      | Governance due process notice                                                                      |
| Appeal Right            | Governance appeal mechanism                                                                        |
| Explanation Requirement | Governance explanation, reasoning, or counterfactual tier                                          |
| Decision Rule           | lifecycle guard, reasoning `RuleReference`, or external policy engine integration                  |
| Deadline                | timer, temporal parameter, due-process grace period, or task SLA                                   |
| Evidence Requirement    | caseFile field, validation pipeline, document task, or reasoning evidence reference                |
| Actor Authority         | actor declaration, delegation, override authority, or separation-of-duties rule                    |
| AI Use                  | agent declaration, action-site autonomy, deontic constraint, confidence policy, or fallback policy |
| Flexible Case Phase     | Advanced constraint zone                                                                           |
| Scenario                | WOS Tooling scenario / conformance trace relationship                                              |
| Reviewer Resolution     | authoring provenance or governance rationale                                                       |
| Policy Source Citation  | RuleReference citation, source authority, or authoring provenance                                  |

---

# 7. Studio Concept Model

Before deriving formal Studio specs and schemas, define a compact concept model. This document bridges the PRD and implementation-facing specifications.

Suggested deliverable:

```text
docs/studio/STUDIO-CONCEPT-MODEL.md
```

or:

```text
specs/studio/concept-model.md
```

> **Note (folder adoption):** in this repo the concept model is landed at `studio-authoring/CONCEPT-MODEL.md` per the docs-only / everything-in-folder posture documented in this file's preface.

## Core entities

The concept model should define:

* Workspace
* SourceDocument
* SourceVersion
* SourceSection
* SourceCitation
* ExtractedClaim
* PolicyObject
* Assumption
* Conflict
* ReviewerResolution
* StudioToWosMapping
* WorkflowIntent
* Scenario
* ValidationFinding
* ApprovalDecision
* PublishedWorkflowPackage
* RuntimeObservation
* ChangeImpactReport

## Source lifecycle

```text
uploaded
→ parsed
→ indexed
→ classified
→ current | superseded | preliminary | disputed
```

## Extracted claim lifecycle

```text
candidate
→ normalized
→ needsReview
→ approved | rejected | merged | split
```

## Policy object lifecycle

```text
draft
→ reviewed
→ approved
→ mapped | authoringOnly | requiresSpecExtension | unmappedButApproved
→ validated
→ published
→ superseded
```

## Workflow lifecycle

```text
draft
→ mapped
→ validationReady
→ scenarioTested
→ approved
→ published
→ deprecated
```

## Scenario lifecycle

```text
generated
→ reviewed
→ passing | failing
→ acceptedAsKnownGap
→ regression
```

## State boundaries

Studio should distinguish three state layers.

```text
Session state
  Ephemeral chat context, temporary candidates, scratch assumptions, draft summaries, and uncommitted AI proposals.

Workspace state
  Uploaded sources, reviewed objects, mappings, workflow drafts, validation findings, scenarios, comments, and approvals.

Published state
  Approved WOS artifacts, scenario suites, approval packages, release notes, and exported packages.
```

This prevents each context-building session from becoming a bespoke artifact with no reusable structure.

---

> **Editorial note (adopted spec set, 2026-04-30, v4):** the PRD's "Suggested" spec list below names 9 specs. After persona-driven review (program-manager / compliance-reviewer / technical-implementer), an **architect persona pass** (gov-tools sales+build veteran), and a **conceptual-completeness restructure** (v4), the **adopted** spec set is **16 specs across 7 families**. v4 ADDED: `workflow-intent.md`, `compiler-contract.md`, `workspace.md`, `binding-and-integration.md`, `runtime-observation-seam.md` (seam contract only), `effectiveness-and-applicability.md` (composable jurisdictional+temporal scope), `identity-and-attestation.md` (composes parent PLN-0381), `terminology-and-canonical-vocabulary.md` (DPV adoption + plain-English layer). v4 selectively adopted external standards (JSON-LD, DPV, PROV-O, OASIS LegalRuleML, DMN one-way import) where they unlock real value. Definitive index: [`specs/README.md`](specs/README.md). CM §5 names "WOS as canonical substrate" as a load-bearing principle (target-neutral authoring vocabulary; WOS scales down via embedded-block conditionality for low-risk workflows; runtime-target neutrality inherited from WOS). Schema composition strategy (CM §6.1) reduces Stage-3 schemas from ~33 to ~10 by composition with parent WOS schemas. The PRD's prose below is preserved verbatim as authored.

# 8. Studio Specs and Schemas

Studio should derive internal W3C-style specs from this PRD and the Studio Concept Model.

## Suggested Studio specs

```text
specs/studio/source-vault.md
specs/studio/policy-object-model.md
specs/studio/authoring-provenance.md
specs/studio/studio-to-wos-mapping.md
specs/studio/readiness-validation.md
specs/studio/scenario-authoring.md
specs/studio/review-and-approval.md
specs/studio/change-impact.md
specs/studio/runtime-observation.md
```

> **Note (folder adoption):** in this repo the specs are landed at `studio-authoring/specs/<name>.md` per the docs-only / everything-in-folder posture documented in this file's preface.

Each spec should include:

* status;
* scope;
* terminology;
* data model;
* lifecycle states;
* normative behavior;
* validation/readiness rules;
* WOS mappings;
* examples;
* open issues.

## Suggested Studio schemas

```text
studio-workspace.schema.json
studio-source.schema.json
studio-policy-object.schema.json
studio-authoring-provenance.schema.json
studio-mapping.schema.json
studio-scenario.schema.json
studio-validation-finding.schema.json
studio-review.schema.json
studio-change-impact.schema.json
studio-package.schema.json
```

## Schema hardness levels

### Hard schemas

Hard schemas are for durable reviewed artifacts:

* SourceDocument
* PolicyObject
* ReviewerResolution
* StudioToWosMapping
* Scenario
* ValidationFinding
* ApprovalDecision
* PublishedWorkflowPackage

### Flexible schemas

Flexible schemas are for AI extraction candidates:

* ExtractedClaim
* CandidatePolicyObject
* CandidateWorkflowStep
* CandidateScenario

Candidate schemas should tolerate uncertainty, partial fields, and messy source evidence. Reviewed artifacts should be stricter.

---

# 9. Core Product Modules

## 9.1 Source Vault

### Purpose

Collect, organize, version, and cite the materials that define the workflow.

### Inputs

* policy manuals
* standard operating procedures
* regulations
* memos
* forms
* decision guides
* service blueprints
* case documentation
* diagrams or screenshots
* existing system exports
* API and integration documents

### Capabilities

* Upload and import source materials.
* Preserve source versions and metadata.
* Tag sources by program, workflow, jurisdiction, effective date, and policy area.
* Link extracted objects to source excerpts.
* Compare source versions.
* Track superseded, disputed, preliminary, and current sources.
* Respect permissions for sensitive documents.

### MVP requirement

Users can upload source documents and see which extracted objects came from which source excerpts.

---

## 9.2 Policy Extraction Review

### Purpose

Convert source material into structured, reviewable policy objects that can drive workflow design, validation, scenario generation, and change impact analysis.

The product artifact is not raw LLM output. The product artifact is a reviewed structured interpretation.

### Capabilities

* AI-assisted extraction of requirements, deadlines, notices, actors, evidence needs, decision rules, exceptions, authorities, and applicability conditions.
* Review queue for extracted objects.
* Approve, reject, edit, merge, or split extracted objects.
* Confidence indicators and source citations.
* Open-question and assumption tracking.
* Conflict detection across extracted objects.
* Authority/source ranking where multiple sources govern the same requirement.
* Effective-date and applicability-scope capture.
* Reviewer resolution records for ambiguous or conflicting interpretations.

### Structured object families

#### Source and authority objects

* PolicySource
* SourceCitation
* AuthorityRank
* ApplicabilityScope
* EffectivePeriod
* Supersession

#### Requirement objects

* Requirement
* Obligation
* Permission
* Prohibition
* Condition
* ExceptionRule
* DecisionRule
* EvidenceRequirement
* DataElement
* Outcome

#### Due-process objects

* NoticeRequirement
* AppealRight
* ExplanationRequirement
* ContinuationOfServicesRequirement
* CounterfactualRequirement

#### Workflow mapping objects

* WorkflowStepMapping
* LifecycleTagMapping
* TransitionMapping
* TimerMapping
* ActorMapping
* TaskMapping
* CaseFileMapping
* ScenarioMapping

#### Review and uncertainty objects

* Assumption
* OpenQuestion
* Conflict
* ReviewerResolution
* ApprovalDecision

### MVP requirement

Users can review and approve extracted policy objects before they are used to generate or update workflow behavior.

---

## 9.3 Policy Knowledge Map

### Purpose

Provide an internal graph-backed traceability layer connecting sources, policy objects, WOS concepts, workflow elements, validation findings, scenarios, reviewer decisions, and versions.

The graph is infrastructure, not the user interface.

Users should experience:

* policy maps;
* linked requirements;
* traceability;
* dependency views;
* impact analysis;
* "why this exists" explanations.

They should not need to understand graph database mechanics.

### Core graph chain

```text
SourceCitation
→ PolicyObject
→ WosConcept
→ WosJsonPath
→ ValidationFinding
→ Scenario
→ ReviewDecision
→ PublishedVersion
```

### Key questions it should answer

* Why does this workflow step exist?
* Which policy source supports this deadline?
* Which workflows depend on this source paragraph?
* Which requirements are not yet mapped to workflow behavior?
* Which workflow steps lack source support?
* Which scenarios test this appeal right?
* What changed when this policy source changed?

### Suggested relationships

* sourceSupports
* derivedFrom
* requires
* permits
* prohibits
* conditions
* mapsTo
* validatedBy
* conflictsWith
* supersedes
* effectiveDuring
* implementedBy
* observedIn
* resolvedBy

### MVP requirement

Users can ask, "Why does this workflow step exist?" and get a source-backed answer.

---

## 9.4 Workflow Builder

### Purpose

Let users generate, review, and refine workflow designs without directly editing WOS JSON or thinking in statechart terms.

### Default user-facing concepts

* phase
* step
* decision
* review
* notice
* deadline
* appeal
* exception
* hold
* data collection
* evidence request
* system check
* AI assistance
* manual override
* completion outcome

### Expert-mode concepts

* state
* transition
* guard
* event
* action
* timer
* caseFile path
* provenance record
* extension seam

### Capabilities

* Generate draft workflow from approved policy objects.
* Show plain-language workflow summary.
* Show step-by-step workflow outline.
* Show visual process map.
* Show decisions and outcomes.
* Show actor/responsibility assignments.
* Show data and document requirements.
* Show deadlines, notices, appeals, exceptions, and holds.
* Allow guided edits in operational language.
* Preserve mappings from workflow elements to policy objects and source material.
* Compile workflow intent into WOS lifecycle, governance, AI, advanced, and tooling artifacts as appropriate.

### MVP requirement

Users can generate and edit a basic workflow from approved policy objects and see a visual map plus plain-language explanation.

---

## 9.5 Data and Document Requirements Designer

### Purpose

Ensure workflows collect, validate, and use the data and evidence they depend on.

### Capabilities

* Identify required data fields.
* Identify required documents and evidence.
* Show where fields are collected, used, and updated.
* Detect data used in decisions but never collected.
* Detect required evidence with no collection step.
* Track basic sensitivity labels.
* Map data elements to forms, case fields, validation pipelines, or system sources.
* Link data and evidence requirements to source-backed policy objects.

### MVP requirement

Users can see which data and documents are required for each decision or step.

---

## 9.6 Validation Center

### Purpose

Give users a single place to understand workflow quality, completeness, WOS alignment, traceability, scenario coverage, and risk.

### Validation stack

```text
1. Source extraction validation
   Did the system extract policy objects faithfully?

2. Policy object validation
   Are objects internally coherent?

3. Mapping validation
   Are approved objects mapped to WOS concepts?

4. WOS artifact validation
   Does generated $wosWorkflow pass schema/lint/conformance-oriented checks?

5. Scenario validation
   Do representative cases behave as expected?

6. Review validation
   Are required reviewers and resolutions complete?

7. Runtime validation, later
   Does observed behavior match designed behavior?
```

### Readiness rule tiers

Internally, Studio should have lint-like readiness rules. Externally, these should appear as readiness checks, validation findings, review blockers, and publication blockers.

#### Tier S1 — Source and extraction readiness

* Source has no citation anchors.
* Candidate object has no source citation.
* Source is preliminary but object is marked final.
* Extracted object has low confidence and no reviewer.

#### Tier S2 — Policy object readiness

* Approved object missing required fields.
* Conflict unresolved.
* Assumption unapproved.
* Authority rank missing for conflicting sources.
* Effective date missing for source with versioned applicability.

#### Tier S3 — Mapping readiness

* Approved object has no WOS mapping.
* Workflow step has no policy backing.
* Mapping points to invalid WOS path.
* Object marked `requiresSpecExtension` without an extension record.

#### Tier S4 — Workflow readiness

* Adverse decision has no notice path.
* Appeal right has no appeal branch.
* Deadline has no timer or review obligation.
* Data used in decision is never collected.
* Actor lacks authority for assigned step.

#### Tier S5 — Scenario readiness

* Critical path has no scenario.
* Scenario lacks expected outcome.
* Scenario failing without waiver.
* New policy change did not rerun affected scenarios.

#### Tier S6 — Publication readiness

* Critical findings unresolved.
* Required reviewers missing.
* WOS artifact fails schema/lint.
* Approval package incomplete.

### Validation categories

* Source traceability checks
* Requirement coverage checks
* Workflow completeness checks
* Mapping completeness checks
* Actor and authority checks
* Notice and appeal checks
* Deadline and timer checks
* Data coverage checks
* Contradiction checks
* Assumption and open-question checks
* Reviewer-resolution checks
* Scenario coverage checks
* WOS schema/lint/conformance checks

### Example findings

* A denial path exists without a notice requirement.
* An appeal right exists but has no workflow branch.
* A deadline exists but is not represented in the workflow.
* A decision relies on data that is never collected.
* A required actor has no documented authority.
* A requirement has no mapped workflow behavior.
* A workflow step has no source support.
* Two source documents conflict on the same deadline.
* A workflow element depends on an unapproved assumption.
* A critical scenario has no test case.

### MVP requirement

Users can see validation findings grouped by severity, understand why each finding matters, and review suggested fixes.

---

## 9.7 Scenario Simulation and Scenario Artifacts

### Purpose

Let users play through real-world scenarios and understand how the workflow behaves.

Scenarios should be durable artifacts, not one-off demos.

### Product concept

```text
Studio Scenario = authorable input
WOS Conformance Trace = observed output
Simulation Report = user-facing explanation
```

### Scenario artifact should capture

* Scenario name and purpose
* Linked policy requirements
* Initial case state
* Input data and documents
* Event sequence
* Time advances
* Expected workflow path
* Expected notice
* Expected appeal branch
* Expected task
* Expected timer
* Expected decision or outcome
* Expected provenance observations
* Review status

### Core scenario types

* happy path
* incomplete application
* deadline missed
* adverse determination
* notice generated
* appeal filed
* exception applies
* supporting document missing
* manual override
* system failure / fallback
* agent fallback
* policy change scenario

### MVP requirement

Users can run a small set of generated scenarios and receive a plain-language explanation of each outcome.

---

## 9.8 Versioning and Change Impact

### Purpose

Help teams manage workflow evolution as policy and operations change.

This should become a flagship feature, not an afterthought.

### Change impact chain

```text
changed source passages
→ affected policy objects
→ affected WOS concepts
→ affected workflow steps
→ affected scenarios
→ affected published versions
→ potentially affected active cases
```

### Capabilities

* Workflow version history.
* Source document change detection.
* Source-to-policy object impact analysis.
* Policy object-to-workflow impact analysis.
* Scenario regression after changes.
* Semantic diff between workflow versions.
* Identification of changed steps, decisions, actors, deadlines, notices, appeals, data fields, mappings, assumptions, and reviewer resolutions.
* Change rationale and approval record.
* Release notes generation.

### MVP requirement

Users can compare two workflow versions and see meaningful changes in plain language.

---

## 9.9 Review, Governance, and Approval

### Purpose

Support organizational trust, accountability, and auditability.

### Review levels

| Level            | Review states                           |
| ---------------- | --------------------------------------- |
| source           | imported / current / superseded         |
| extracted object | draft / approved / rejected             |
| mapping          | unmapped / mapped / approved            |
| workflow         | draft / reviewed / approved / published |
| scenario         | generated / passed / reviewed           |
| conflict         | unresolved / resolved / waived          |

### Capabilities

* Assign reviewers by role.
* Comment on source excerpts, policy objects, workflow steps, validation findings, mappings, conflicts, assumptions, and scenarios.
* Track approval status.
* Capture reviewer rationale.
* Maintain audit trail.
* Generate approval package.
* Block or warn before publication when critical unresolved findings remain.

### Studio should record

* who approved extracted requirements;
* who approved policy conflict resolutions;
* who approved workflow mappings;
* who approved scenario behavior;
* who approved publication;
* what sources they reviewed;
* what assumptions remained.

### MVP requirement

A workflow can move through draft, review, approved, and published states with visible unresolved findings.

---

## 9.10 WOS Expert Mode

### Purpose

Support technical users without overwhelming non-technical users.

### Capabilities

* View generated WOS artifact.
* Inspect mappings from Studio objects to WOS concepts and JSON paths.
* View schema/lint/conformance details.
* Export WOS artifacts.
* Inspect scenario/conformance trace relationship.
* Review generated patches before integration.

### MVP requirement

Technical users can inspect and export the generated WOS document.

---

## 9.11 Runtime Observation and Iteration

### Purpose

Eventually close the loop between designed workflow and actual operations.

### Initial runtime scope

Start with imported traces, not live orchestration.

### Capabilities

* Import case/event histories.
* Normalize runtime traces.
* Compare designed paths to observed paths.
* Detect unmodeled steps.
* Detect bottlenecks and repeated manual overrides.
* Detect cases stuck in states.
* Generate scenarios from real cases.
* Propose improvements backed by runtime evidence.

### Deferred capabilities

* Full live adapter marketplace.
* Bidirectional orchestration.
* Deep process mining.
* Automated runtime deployment across engines.

### MVP requirement

Not required for first release, but the data model should anticipate runtime trace import and designed-vs-observed comparison.

---

# 10. AI Copilot Requirements

The AI copilot should be a guided domain assistant that produces typed, reviewable actions. It should not be an unconstrained chatbot; it should operate over source material, Studio objects, WOS mappings, validation findings, and scenarios.

## Core AI roles

### Extraction assistant

Reads source materials and proposes structured policy objects.

### Workflow drafting assistant

Creates a draft workflow from approved policy objects and mappings.

### Explanation assistant

Explains why a step, decision, deadline, exception, actor responsibility, or data requirement exists.

### Validation assistant

Explains findings and proposes reviewable fixes.

### Simulation assistant

Explains scenario outcomes.

### Change assistant

Summarizes policy changes and workflow impacts.

### Resolution assistant

Helps users resolve conflicts, assumptions, and open questions by presenting evidence, affected behavior, and recommended reviewer actions.

## Typed AI actions

The copilot should produce typed actions such as:

* CreatePolicyObject
* RevisePolicyObject
* ProposeWorkflowMapping
* GenerateScenario
* ExplainValidationFinding
* ResolveConflictProposal
* GenerateWosPatch
* SummarizeChangeImpact
* CreateReviewerQuestion

## Every AI action should include

* source evidence;
* confidence;
* affected objects;
* review status;
* diff or proposed change;
* reasoning summary;
* WOS mapping impact, where applicable.

## AI behavior requirements

The AI should:

* cite sources when making claims;
* separate extracted facts from assumptions;
* show confidence where useful;
* identify authority/source hierarchy when sources disagree;
* ask clarifying questions when ambiguity affects workflow behavior;
* create reviewable proposals rather than silently changing approved artifacts;
* preserve traceability;
* distinguish draft interpretation, reviewed interpretation, approved interpretation, and published workflow behavior.

The AI should not:

* invent unsupported policy;
* hide uncertainty;
* make final policy interpretations without human review;
* bypass validation checks;
* directly publish unreviewed workflow changes;
* collapse source conflict into a single answer without showing competing sources.

---

# 11. Compiler, Scenario Runtime, and Reference Architecture

## Studio-to-WOS Compiler

Studio's core runtime tool should be a compiler, not a workflow engine.

```text
Studio workspace
→ WOS workflow draft
→ WOS tooling/scenario artifacts
→ validation reports
→ approval package
```

## Compiler passes

```text
1. Select approved policy objects
2. Resolve mapping records
3. Generate workflow intent
4. Emit WOS lifecycle/governance/AI/advanced blocks
5. Emit scenario artifacts
6. Run Studio readiness checks
7. Run WOS schema/lint/conformance checks
8. Produce review package
```

Once reviewed inputs are fixed, compilation should be deterministic. AI may propose; the compiler should not improvise.

## Scenario Runtime

Before deep external runtime integration, Studio should support a scenario runtime.

```text
Scenario artifact
+ WOS workflow draft
+ initial case state
+ event sequence
→ simulated trace
→ expected-vs-actual comparison
→ user-facing explanation
```

This is the bridge between non-technical review and formal conformance.

## Reference Architecture

The reference architecture should define system components rather than product vision.

Suggested docs:

```text
reference-architecture/studio-overview.md
reference-architecture/knowledge-layer.md
reference-architecture/object-store.md
reference-architecture/graph-traceability.md
reference-architecture/extraction-pipeline.md
reference-architecture/studio-compiler.md
reference-architecture/readiness-engine.md
reference-architecture/scenario-engine.md
reference-architecture/wos-export.md
```

## Core components

```text
Source Store
  Raw docs, parsed text, page refs, citations.

Knowledge Index
  Embeddings, chunk metadata, retrieval.

Graph Layer
  Source → object → mapping → WOS path → scenario → validation.

Studio Object Store
  Typed durable Studio artifacts.

Review System
  Comments, approvals, reviewer resolutions.

Compiler
  Studio objects → WOS artifacts.

Readiness Engine
  Studio lint/readiness rules.

Scenario Engine
  Scenario execution and trace comparison.

WOS Validator
  Schema/lint/conformance.

Export/Package Layer
  WOS artifact, scenarios, approval package.
```

---

# 12. Key User Stories

## Source Vault

### As an operations manager

I want to upload policy and procedure documents so the system can analyze them.

### As a compliance reviewer

I want to see which source document supports each extracted requirement so I can verify it.

### As a policy owner

I want to compare versions of a source document so I can see what may affect existing workflows.

---

## Structured Policy Model

### As a policy analyst

I want extracted policy requirements to become structured objects so I can review, edit, and approve them before they affect workflow behavior.

### As a legal reviewer

I want to see the authority rank and effective date of each source so I can resolve conflicts correctly.

### As an operations manager

I want assumptions to be tracked explicitly so unsupported workflow behavior does not sneak into the process.

### As a workflow owner

I want each workflow step to point back to approved policy objects so I can defend why the process works the way it does.

---

## Studio-to-WOS Mapping

### As a technical reviewer

I want to see how approved policy objects map to WOS concepts and JSON paths.

### As a workflow owner

I want to see which approved objects remain unmapped so I can resolve gaps before publication.

### As a product administrator

I want authoring-only objects to be explicitly marked so they are not mistaken for runtime behavior.

---

## Authoring Provenance

### As a reviewer

I want to see the chain from source passage to policy object to workflow element so I can audit the authoring process.

### As a program manager

I want to know whether a workflow element comes from policy, local practice, or an assumption.

### As an auditor

I want to see who approved an interpretation and when.

---

## Conflict and Assumption Resolution

### As a policy owner

I want the system to flag conflicting source material so I can resolve it before publication.

### As a legal reviewer

I want to record the rationale for choosing one source interpretation over another.

### As an operations manager

I want unresolved assumptions to block or warn before publication depending on severity.

---

## Workflow Builder

### As an operations manager

I want the system to draft a workflow from approved requirements so I do not need to design it from scratch.

### As a service designer

I want to edit the workflow in a visual and plain-language interface so I can collaborate with non-technical stakeholders.

### As a technical reviewer

I want to see how workflow steps map to formal WOS elements so I can assess implementation readiness.

---

## Data and Document Requirements

### As a program manager

I want to know what information is required for each decision so I can ensure the process collects it.

### As a compliance reviewer

I want to see where sensitive information is used so I can evaluate privacy and access risks.

### As an analyst

I want the system to flag decisions that depend on data that is never collected.

---

## Validation Center

### As an operations manager

I want validation findings in plain language so I know what needs to be fixed.

### As a compliance reviewer

I want to know whether every adverse outcome has the required notice and review path.

### As a technical reviewer

I want schema and conformance-oriented findings so I can confirm the formal artifact is usable downstream.

---

## Scenario Simulation

### As a program manager

I want to simulate a typical case so I can confirm the workflow matches operational expectations.

### As a policy reviewer

I want to simulate adverse, appeal, and exception scenarios so I can verify policy coverage.

### As a service designer

I want the system to explain why a scenario followed a certain path.

### As a technical reviewer

I want scenarios to produce durable traces that can be reused as regression tests.

---

## Versioning and Change Impact

### As a policy owner

I want to see what changed between workflow versions so I can approve updates confidently.

### As an operations manager

I want to understand which steps are affected by a policy update.

### As a reviewer

I want release notes generated from approved changes so stakeholders can understand the update.

### As a program manager

I want to know which scenarios need to be rerun after a policy change.

---

## Review and Approval

### As a workflow owner

I want to route a workflow for review so legal, compliance, operations, and technical reviewers can approve it.

### As an approver

I want to see unresolved findings and assumptions before approving a workflow.

### As an auditor

I want to see who approved which interpretation and when.

---

## Runtime Observation

### As an operations manager

I want to compare actual case behavior against the designed workflow so I can identify drift.

### As a program manager

I want to see bottlenecks and repeated exceptions so I can improve the process.

### As a policy owner

I want runtime behavior to generate evidence-backed improvement proposals.

---

# 13. Non-Functional Requirements

## Trust and explainability

* Important generated workflow behavior should be explainable.
* Source-backed claims should include citations.
* AI-generated proposals should be distinguishable from approved objects.
* Studio-to-WOS mappings should be inspectable.

## Security and permissions

* Source materials may be sensitive.
* Access controls should apply to documents, workflows, review actions, exports, and published artifacts.
* Audit logs should capture key changes, approvals, and generated outputs.

## Extensibility

* The structured policy model should support domain-specific extensions.
* Extensions should not silently become runtime behavior without mapping and review.
* Future WOS spec extensions should be representable through the mapping contract.

## Usability

* The default interface should be understandable to non-technical users.
* Technical concepts should be progressively disclosed.
* Validation and simulation outputs should use plain language.

## Portability

* The platform should generate formal WOS artifacts.
* Exports should be versioned and traceable to approved source-backed models.
* Scenarios should be reusable as review and regression assets.

## Reproducibility

* Published artifacts should be reproducible from reviewed inputs.
* Compiler outputs should be deterministic once approved Studio objects and mappings are fixed.

---

# 14. Success Metrics

## Adoption metrics

* Time from source upload to first workflow draft.
* Percentage of workflows completed without technical assistance.
* Number of workflows moved from draft to approved.
* Number of repeat users per workspace.

## Quality metrics

* Percentage of workflow elements with source traceability.
* Percentage of approved policy objects mapped to WOS concepts.
* Number of validation findings resolved before approval.
* Number of unresolved assumptions at approval time.
* Number of policy conflicts detected and resolved.
* Scenario coverage for critical paths.

## Trust metrics

* Percentage of AI-extracted policy objects edited or rejected.
* Reviewer confidence scores.
* Approval cycle time.
* Number of source-backed explanations viewed.
* Number of mapping inspections by technical reviewers.

## Operational metrics

* Reduction in workflow design cycle time.
* Faster workflow updates after source changes.
* Improved documentation completeness.
* Reduction in unmapped or unsupported workflow behavior.

---

# 15. Risks and Mitigations

## Risk: Studio becomes a parallel semantics system

### Mitigation

Require every approved object to declare its WOS mapping state.

## Risk: The product becomes too technical

### Mitigation

Default to guided, plain-language flows. Keep WOS expert mode separate.

## Risk: Users overtrust AI extraction

### Mitigation

Require review and approval for structured policy objects before generation or publication.

## Risk: The policy model becomes too broad too early

### Mitigation

Start with a compact authoring model aligned to WOS concepts. Add extensions based on real customer workflows.

## Risk: Generated workflows look polished but are wrong

### Mitigation

Require validation, traceability, and scenario testing before approval.

## Risk: The graph layer leaks into UX

### Mitigation

Expose traceability and impact analysis, not graph mechanics.

## Risk: Technical artifacts and non-technical views drift apart

### Mitigation

Use the Studio-to-WOS mapping contract and synchronized views.

## Risk: Chat context becomes the source of truth

### Mitigation

Persist durable Studio objects, mappings, validations, scenarios, and approvals. Treat chat/session context as exploratory until committed.

---

# 16. Product Roadmap

## Phase 1 — Source-backed workflow drafting

### Goal

Prove that a non-technical user can go from source documents to a reviewable workflow draft.

### Primary users

* operations manager
* policy analyst
* technical reviewer

### Must include

* Source upload
* Source citation/excerpt viewer
* AI extraction into structured policy objects
* Human review of extracted objects
* Basic mapping to WOS concepts
* Workflow outline
* Visual workflow map
* Plain-language workflow summary
* WOS artifact preview/export
* Basic validation: unmapped requirement, unsupported workflow step, missing source

### Success question

Can the user answer: **"Where did this workflow step come from?"**

## Phase 1 epics

### Epic 1.1: Source Vault MVP

Capabilities:

* Upload documents.
* Store document metadata.
* Track source versions.
* View parsed text and excerpts.
* Link extracted objects to source excerpts.

Representative user stories:

* As a user, I can upload a policy document so the system can analyze it.
* As a reviewer, I can see the source excerpt behind an extracted requirement.
* As a workflow owner, I can group source documents into a workflow source set.

### Epic 1.2: Policy Extraction MVP

Capabilities:

* Extract candidate requirements, deadlines, notices, actors, evidence needs, decision rules, and exceptions.
* Present extracted objects in a review queue.
* Support approve, reject, and edit.
* Track confidence and source citation.
* Capture assumptions and open questions.

Representative user stories:

* As a policy analyst, I can review extracted requirements one by one.
* As a compliance reviewer, I can reject an unsupported interpretation.
* As an operations manager, I can mark an ambiguous requirement as an open question.

### Epic 1.3: Studio-to-WOS Mapping MVP

Capabilities:

* Map approved Studio objects to WOS concepts.
* Track mapping state: `mapsToWos`, `authoringOnly`, `requiresSpecExtension`, `unmappedButApproved`.
* Show unmapped approved objects.
* Show unsupported workflow behavior.

Representative user stories:

* As a technical reviewer, I can see how approved policy objects map to WOS.
* As a workflow owner, I can resolve approved but unmapped objects before publication.
* As a product owner, I can identify candidate WOS extensions from repeated mapping gaps.

### Epic 1.4: Workflow Draft Generation

Capabilities:

* Generate a basic workflow from approved policy objects and mappings.
* Generate steps, decisions, actors, outcomes, deadlines, and exceptions.
* Preserve mappings from workflow elements to policy objects.
* Identify unmapped requirements.

Representative user stories:

* As an operations manager, I can generate a draft workflow from approved requirements.
* As a reviewer, I can see which requirements were used to generate each step.
* As a user, I can see requirements that were not mapped into the workflow.

### Epic 1.5: Workflow Review Views

Capabilities:

* Plain-language workflow summary.
* Step outline.
* Visual process map.
* Decisions and outcomes view.
* Roles and responsibilities view.
* Data and documents view.

Representative user stories:

* As a program manager, I can read a plain-language summary of the workflow.
* As a service designer, I can review the workflow visually.
* As a compliance reviewer, I can view notices, deadlines, and appeal points.

### Epic 1.6: WOS Artifact Generation

Capabilities:

* Generate a formal WOS artifact from workflow intent.
* Inspect generated artifact in expert mode.
* Export generated artifact.
* Preserve mapping from WOS elements back to workflow intent and policy objects.

Representative user stories:

* As a technical reviewer, I can inspect the generated WOS artifact.
* As a platform engineer, I can export the WOS artifact for downstream validation.
* As an operations manager, I can avoid seeing the WOS artifact unless I need it.

## Phase 1 exit criteria

Phase 1 is successful when a user can:

1. upload a policy document;
2. review extracted policy objects;
3. approve a subset of those objects;
4. map approved objects to WOS concepts;
5. generate a workflow draft;
6. review the draft visually and in plain language;
7. inspect or export the formal WOS artifact.

---

## Phase 2 — Trust, validation, and scenario testing

### Goal

Make workflows credible enough for policy, legal, compliance, operations, and technical review.

### Must include

* Validation Center
* Due-process coverage checks
* Authority/source ranking display
* Conflict and assumption management
* Reviewer resolution records
* Scenario artifact generation
* Scenario simulation
* Approval workflow
* Semantic diff between workflow versions

### Success question

Can the user answer: **"Does this workflow correctly handle denial, notice, appeal, timeout, and missing information?"**

## Phase 2 epics

### Epic 2.1: Validation Center

Capabilities:

* Display validation findings by severity.
* Detect unmapped requirements.
* Detect workflow steps without source support.
* Detect missing notice, appeal, deadline, actor, or data mappings.

* Detect unresolved assumptions and source conflicts.
* Detect missing reviewer resolutions for high-impact ambiguities.
* Explain why findings matter.
* Suggest reviewable fixes.

Representative user stories:

* As a compliance reviewer, I can see if every adverse outcome has notice and appeal coverage.
* As an operations manager, I can understand validation findings without technical language.

* As a technical reviewer, I can inspect formal validation findings when needed.

### Epic 2.2: Data and Evidence Coverage

Capabilities:

* Map data elements to workflow steps and decisions.
* Map evidence requirements to collection steps.
* Identify data used but never collected.
* Identify required evidence with no collection path.

* Support basic sensitivity labels.
* Link evidence requirements to source-backed policy objects.

Representative user stories:

* As a reviewer, I can see which fields are required before a decision.
* As an analyst, I can identify a decision that depends on missing data.
* As a compliance reviewer, I can see which sensitive fields are used in adverse decisions.

### Epic 2.3: Scenario Artifacts and Simulation

Capabilities:

* Generate baseline scenarios from the workflow.
* Run typical and edge-case simulations.
* Save scenarios as durable test artifacts.
* Show path taken, decisions made, deadlines triggered, and required notices.
* Explain outcomes in plain language.
* Link scenario behavior back to policy objects and WOS mappings.
* Track scenario review status.

Representative user stories:

* As an operations manager, I can simulate a typical case.
* As a compliance reviewer, I can simulate an adverse decision and appeal.
* As a service designer, I can test what happens when required information is missing.
* As a technical reviewer, I can export scenario artifacts for regression testing.

### Epic 2.4: Review and Approval Workflow

Capabilities:

* Assign reviewers.
* Comment on policy objects, workflow steps, mappings, validation findings, assumptions, conflicts, and scenarios.
* Track approval status.
* Prevent publication with unresolved critical findings unless explicitly overridden.
* Generate approval package.

Representative user stories:

* As a workflow owner, I can send a workflow for legal and operations review.
* As an approver, I can see unresolved findings before signing off.
* As an auditor, I can see who approved a workflow and what they reviewed.

### Epic 2.5: Semantic Version Compare

Capabilities:

* Compare workflow versions by meaningful changes.
* Highlight changed steps, decisions, actors, deadlines, notices, appeals, data fields, mappings, assumptions, and reviewer resolutions.
* Generate release notes.

Representative user stories:

* As a policy owner, I can see what changed between two workflow versions.
* As a reviewer, I can focus only on changed policy-sensitive areas.
* As an operations manager, I can generate a summary of changes for stakeholders.

### Epic 2.6: Authoring Provenance

Capabilities:

* Preserve source-to-policy-to-workflow traceability.
* Track whether a workflow element came from source, approved interpretation, local practice, or assumption.
* Record reviewer decisions and rationale.
* Show authoring provenance in workflow review and approval packages.

Representative user stories:

* As a reviewer, I can trace a workflow step back to its supporting source material.
* As a program manager, I can identify workflow behavior based on assumptions rather than approved policy.
* As an auditor, I can see the review trail for a policy interpretation.

### Epic 2.7: Conflict and Assumption Management

Capabilities:

* Detect conflicting structured policy objects.
* Track assumptions separately from extracted requirements.
* Route conflicts to appropriate reviewers.
* Record resolution decisions and rationale.
* Block or warn on unresolved high-impact ambiguity.

Representative user stories:

* As a legal reviewer, I can resolve conflicting appeal deadlines with a recorded rationale.
* As an operations manager, I can see assumptions that still need confirmation.
* As a workflow owner, I can prevent unresolved ambiguity from silently becoming workflow behavior.

## Phase 2 exit criteria

Phase 2 is successful when a workflow can move from draft to reviewed and approved with:

1. validation findings resolved or acknowledged;
2. core scenarios simulated and reviewed;
3. reviewers recorded;
4. source traceability preserved;
5. Studio-to-WOS mappings inspected where needed;
6. version changes explained.

---

## Phase 3 — Change impact and portfolio management

### Goal

Make workflows living governed assets.

### Must include

* Source version comparison
* Policy object change detection
* Impact graph
* Workflow health dashboard
* Scenario regression after change
* Improvement backlog
* Cross-workflow reuse of approved policy objects
* Release notes and approval package generation

### Success question

Can the user answer: **"This policy changed. Which workflows and scenarios are affected?"**

## Phase 3 epics

### Epic 3.1: Source Change Detection

Capabilities:

* Compare source document versions.
* Identify changed sections.
* Re-extract affected policy objects.
* Flag policy objects needing review.
* Preserve source version lineage.

Representative user stories:

* As a policy owner, I can see which requirements changed after uploading a new policy version.
* As a reviewer, I can approve or reject updated extractions.
* As a workflow owner, I can see whether a source change affects my workflow.

### Epic 3.2: Change Impact Analysis

Capabilities:

* Map source changes to policy objects.
* Map policy object changes to WOS concepts.
* Map WOS concept changes to workflow elements.
* Identify affected assumptions and reviewer resolutions.
* Identify affected simulations.
* Identify affected data requirements.
* Produce impact report.

Representative user stories:

* As a program manager, I can see which workflow steps are affected by a policy change.
* As a compliance reviewer, I can see whether appeal handling changed.
* As a service designer, I can identify scenarios that need to be rerun.

### Epic 3.3: Improvement Backlog

Capabilities:

* Capture suggested workflow improvements.
* Link suggestions to validation findings, simulations, source changes, reviewer comments, runtime observations, or unresolved assumptions.
* Prioritize by risk, effort, and impact.
* Track resolution.

Representative user stories:

* As an operations manager, I can track workflow improvement ideas.
* As a reviewer, I can link a validation finding to a backlog item.
* As a program manager, I can prioritize changes before the next release.

### Epic 3.4: Workflow Health Dashboard

Capabilities:

* Show traceability coverage.
* Show WOS mapping coverage.
* Show unresolved assumptions.
* Show unresolved conflicts.
* Show validation health.
* Show scenario coverage.
* Show review status.
* Show recent source changes.
* Show workflow elements without approved policy backing.

Representative user stories:

* As a program manager, I can see which workflows need attention.
* As a governance reviewer, I can see workflows with unresolved high-risk findings.
* As an operations leader, I can track workflow readiness across a portfolio.

### Epic 3.5: Policy Knowledge Graph Expansion

Capabilities:

* Maintain canonical relationships among sources, policy objects, WOS mappings, workflow elements, scenarios, validations, and versions.
* Support dependency and impact queries.
* Support source authority ranking and supersession relationships.
* Support cross-workflow reuse of approved policy objects where appropriate.

Representative user stories:

* As a policy owner, I can see all workflows that depend on a specific requirement.
* As a reviewer, I can see which source superseded another source.
* As an analyst, I can identify common policy objects reused across workflows.

## Phase 3 exit criteria

Phase 3 is successful when users can manage workflows as living assets, not one-time diagrams. A policy change should trigger a clear, reviewable chain from changed source to affected workflow behavior and scenarios.

---

## Phase 4 — Runtime observation

### Goal

Close the loop between designed workflow and real operations.

### Must include

* Runtime trace import
* Designed-vs-observed comparison
* Bottleneck and drift detection
* Manual override detection
* Real-case-to-scenario generation
* Evidence-backed improvement proposals

### Success question

Can the user answer: **"Where does actual practice diverge from the approved workflow?"**

## Phase 4 epics

### Epic 4.1: Runtime Trace Import

Capabilities:

* Import case/event histories from available systems.
* Normalize runtime events into comparable workflow traces.
* Link runtime behavior to workflow versions where possible.

Representative user stories:

* As an analyst, I can import case histories for a workflow.
* As an operations manager, I can see which workflow version a case followed.
* As a technical user, I can map external events to workflow events.

### Epic 4.2: Designed vs Observed Comparison

Capabilities:

* Compare actual paths against designed workflow paths.
* Identify unmodeled steps.
* Identify repeated manual overrides.
* Identify skipped or delayed steps.
* Identify cases stuck in states.

Representative user stories:

* As an operations manager, I can see where actual operations differ from the model.
* As a service designer, I can identify shadow processes.
* As a program manager, I can determine whether the workflow needs revision.

### Epic 4.3: Bottleneck and Exception Analysis

Capabilities:

* Detect high-delay steps.
* Detect high-rework branches.
* Detect repeated missing data or documents.
* Detect frequent escalations.
* Identify common scenario failures.

Representative user stories:

* As an operations manager, I can see where cases slow down.
* As a program manager, I can identify recurring issues that should become workflow changes.
* As a service designer, I can use runtime evidence to simplify the process.

### Epic 4.4: Runtime-informed Iteration Proposals

Capabilities:

* Suggest workflow improvements based on observed behavior.
* Link suggestions to runtime evidence.
* Create reviewable change proposals.
* Feed proposals into the improvement backlog.
* Generate new scenarios from observed edge cases.

Representative user stories:

* As a workflow owner, I can review suggested changes based on actual operations.
* As a policy reviewer, I can distinguish operational improvements from policy changes.
* As a program manager, I can plan the next workflow version using evidence.

## Phase 4 exit criteria

Phase 4 is successful when the product supports a closed loop:

```text
Designed workflow
→ runtime behavior
→ observed drift / friction
→ proposed improvement
→ reviewed change
→ new workflow version
```

---

# 17. Implementation Roadmap

This roadmap describes how to build Studio using the same disciplined pattern as WOS-spec and Formspec.

## Stage 0 — Maintain this PRD as the vision document

This document should remain product-facing. It should not become the formal object model.

## Stage 1 — Studio Concept Model

Deliverable:

```text
STUDIO-CONCEPT-MODEL.md
```

Output:

* entities;
* relationships;
* lifecycle states;
* mapping states;
* artifact classes;
* boundaries between session, workspace, and published state.

## Stage 2 — Studio Specs

Deliverables:

```text
source-vault.md
policy-object-model.md
authoring-provenance.md
studio-to-wos-mapping.md
readiness-validation.md
scenario-authoring.md
review-and-approval.md
change-impact.md
runtime-observation.md
```

## Stage 3 — Studio Schemas

Deliverables:

```text
studio-source.schema.json
studio-policy-object.schema.json
studio-mapping.schema.json
studio-scenario.schema.json
studio-validation.schema.json
studio-review.schema.json
studio-package.schema.json
```

## Stage 4 — Readiness / lint engine

Deliverables:

```text
studio-lint-rule-registry
studio-readiness-report
initial rule set
fixtures
```

Initial rules should focus on source traceability, mapping coverage, unresolved conflicts, scenario coverage, and publish blockers.

## Stage 5 — Compiler to WOS

Deliverables:

```text
studio-to-wos compiler
mapping resolver
WOS artifact generator
round-trip examples
```

## Stage 6 — Scenario engine

Deliverables:

```text
scenario runner
expected-vs-actual trace comparison
scenario report
```

## Stage 7 — Reference architecture

Deliverables:

```text
reference architecture docs
component boundaries
storage model
graph model
compiler boundaries
validation flow
```

## Stage 8 — First vertical slice

Use a real document such as the FAFSA ISIR Guide.

Build a slice:

```text
upload PDF
→ extract candidates
→ approve several objects
→ map to WOS
→ generate draft workflow
→ run readiness checks
→ generate one scenario
→ export WOS draft
```

---

# 18. Suggested MVP Boundaries

## Must have

* Source upload
* Source excerpt/citation viewer
* AI-assisted extraction
* Structured policy object review
* Basic Studio-to-WOS mapping
* Workflow draft generation
* Plain-language workflow summary
* Visual workflow map
* Basic validation center
* Basic scenario simulation
* WOS artifact generation/export
* Review status tracking

## Should have

* Version compare
* Open question tracking
* Assumption tracking
* Basic conflict detection
* Basic data/document requirements view
* Traceability map
* Authoring provenance from source excerpt to workflow element
* Saved scenario artifacts
* Approval package generation

## Could defer

* Runtime observation
* Process mining
* Deep integration adapters
* Advanced formal verification UX
* Large domain-specific ontology packs
* Portfolio-level analytics
* Full bidirectional deployment automation

---

# 19. Product Positioning

WOS Studio should be positioned as a source-backed workflow intelligence platform for WOS.

It is not just a workflow builder. It is not a generic knowledge base. It is not a BPMN clone. It is not a raw JSON editor. It is a system for transforming institutional policy and operational knowledge into validated, explainable, reviewable, WOS-aligned workflows.

## Key differentiator

The structured policy model and traceability graph between source material and WOS artifacts.

## Final product statement

**WOS Studio is the non-technical authoring, review, validation, and change-management layer for WOS workflows.**

<!-- END PRD TRANSCRIPTION -->
