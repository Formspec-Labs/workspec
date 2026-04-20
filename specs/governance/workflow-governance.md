---
title: WOS Workflow Governance Specification
version: 1.0.0-draft.1
date: 2026-04-09
status: draft
---

# WOS Workflow Governance Specification v1.0

**Version:** 1.0.0-draft.1
**Date:** 2026-04-09
**Editors:** Formspec Working Group
**Companion to:** WOS Kernel Specification v1.0

---

## Abstract

The WOS Workflow Governance Specification defines Layer 1 of the Workflow Orchestration Standard: the governance structures that complex, regulated, and high-stakes human workflows require. A Workflow Governance Document -- itself a JSON document -- targets a WOS Kernel Document and declares due process requirements, review protocols, data validation pipelines with assertion gates, structured audit (Reasoning and Counterfactual tiers), quality controls (review sampling, separation of duties, override authority), rejection and remediation policies, a task catalog with verifiability matrix, and screener integration for intake routing.

Layer 1 exists because human workflows need it. A pure-human benefits adjudication workflow needs due process, dual-blind review, separation of duties, data validation, and structured reasoning traces -- with zero AI involved. When AI arrives (Layer 2), it plugs into governance structures defined here.

WOS MUST NOT alter core Formspec processing semantics. WOS processors MUST delegate Formspec Definition evaluation to a Formspec-conformant processor (Core S1.4).

---

## Status of This Document

This document is a **draft specification**. It is a companion to the WOS Kernel Specification v1.0 and does not modify the kernel's processing model. It defines governance structures that attach to kernel workflows through the `lifecycleHook`, `contractHook`, and `provenanceLayer` seams. Implementors are encouraged to experiment with this specification and provide feedback, but MUST NOT treat it as stable for production use until a 1.0.0 release is published.

---

## 1. Introduction

### 1.1 Background

Government agencies, regulated industries, and organizations handling high-stakes decisions share governance requirements that predate AI:

- **Due process:** Constitutional and statutory requirements for notice, explanation, appeal, and continuation of service when adverse decisions affect individual rights (State v. Loomis, 881 N.W.2d 749 (Wis. 2016); Houston Federation of Teachers v. Houston ISD, 251 F. Supp. 3d 1168 (S.D. Tex. 2017); APA; ECOA Regulation B; OMB M-24-10; EU AI Act Art. 13-14).
- **Review protocols:** Empirically grounded procedures that produce genuine cognitive engagement during review, not checkbox confirmation (Vaccaro et al., Nature Human Behaviour, 2024; Bucinca et al., CSCW 2021; Li et al., 2024).
- **Data validation:** Staged processing with assertion gates to verify external data before consequential decisions.
- **Structured audit:** Decision traces that record which rules were applied, which evidence was consulted, and what would change the outcome.

These are human governance requirements. This specification defines them independently of AI. Layer 2 (AI Integration) extends these structures for AI-specific concerns.

**Legal sufficiency.** Governance rules defined in this specification contribute to, but do not guarantee, legal admissibility. Implementations MUST comply with the legal-sufficiency disclosure obligations in the WOS Assurance Layer §6. In particular, implementations MUST NOT imply that structured governance alone guarantees evidentiary sufficiency in any particular jurisdiction.

### 1.2 Design Goals

1. **Human-first governance.** Every structure in this layer serves pure-human workflows. AI plugs in later.
2. **Tag-based attachment.** Governance rules match on semantic transition tags from the kernel, not on specific transition identifiers.
3. **Proportional governance.** The kernel's impact level determines the strength of governance controls.

### 1.3 Scope

**Within scope:** due process requirements; review protocols; data validation pipelines; assertion gates; structured audit (Reasoning and Counterfactual tiers); quality controls; rejection and remediation policies; task catalog with verifiability matrix; delegation of authority; typed hold policies; screener integration; temporal parameter resolution; conformance profiles.

**Out of scope:** agent registration, deontic constraints, autonomy levels, confidence framework (Layer 2: AI Integration). SMT verification, equity guardrails (Layer 3: Advanced Governance). Lifecycle topology, case state, provenance Facts tier (Layer 0: Kernel).

### 1.4 Relationship to the Kernel

A Workflow Governance Document targets a WOS Kernel Document via the `targetWorkflow` property. Governance attaches to the kernel through three seams:

- **`lifecycleHook`:** Review protocols, due process, quality controls, delegation of authority, typed hold policies, and temporal parameter resolution attach to transitions via semantic tags.
- **`contractHook`:** Data validation pipelines validate external data against contracts.
- **`provenanceLayer`:** The Reasoning and Counterfactual tiers extend the kernel's Facts tier.

### 1.6 Section Numbering Note

Sections S2.9, S4.9, and S7.15 use identifier numbers that match the WOS Feature Matrix capability rows they specify, rather than the document's sequential hierarchy. This preserves stable cross-document citation anchors. In the document body, S2.9 appears within S2 Conformance, S4.9 appears within S11 Delegation of Authority, and S7.15 appears within S12 Typed Hold Policies.

### 1.5 Notational Conventions

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [BCP 14][RFC 2119] [RFC 8174] when, and only when, they appear in ALL CAPITALS, as shown here.

---

## 2. Conformance

### 2.1 Conformance Profiles

Two profiles are defined:

**Workflow Governance Basic.** Due process requirements (S3) and review protocols (S4) are enforced. The processor correctly attaches governance rules to kernel transitions via tag matching.

**Workflow Governance Complete.** Basic conformance plus: data validation pipelines (S5), structured audit (S6), quality controls (S7), rejection and remediation policies (S8), task management (S10), delegation of authority (S11), typed hold policies (S12), and temporal parameter resolution (S13).

### 2.9 Schema Upgrade as Named Lifecycle Operation

A **schema upgrade** is an explicit migration of a workflow instance (or of referenced Formspec Definitions) to a newer definition version. Schema upgrades are named lifecycle operations distinct from ordinary instance migration.

A schema upgrade MUST:

- Be recorded as a canonical fact in the Facts tier.
- Declare the prior definition version and the new definition version.
- Declare the migration mechanism (Formspec Changelog reference, custom migration map, or declared equivalence).
- Preserve enough interpretation material to verify historical records under the definition version in effect when they were produced (cf. Kernel S9.6).
- NOT silently reinterpret historical records under newer rules.

Schema upgrades MAY apply to an individual instance, to all instances of a workflow, or to all instances within a tenant scope. The scope MUST be declared.

---

## 3. Due Process

This section is normative for workflows whose kernel declares `impactLevel` of `rights-impacting` or `safety-impacting`.

### 3.1 Adverse Decision Policy

When a workflow produces an adverse decision (denial, reduction, termination, or other unfavorable determination), the following requirements apply:

### 3.2 Notice

The affected individual MUST receive notice before the decision takes effect. The notice MUST include:

1. The specific determination made.
2. The factual basis using individualized reason codes, not generic statements.
3. The individual's right to appeal, including the deadline and process.
4. If applicable, disclosure that an AI system assisted in the determination (extended by Layer 2).

A `noticeGracePeriod` (ISO 8601 duration) defines the minimum delay between notice delivery and the decision taking effect.

Notice assembly is deterministic. When an `adverse-decision` transition fires under an `AdverseDecisionPolicy` with `noticeRequired: true`, the processor MUST derive the notice from the pre-transition Facts-tier `caseFileSnapshot` captured for the same determination transition, the policy's appeal configuration, and the Notification Template referenced by `noticeTemplateRef`. The emitted `noticeSent` provenance record MUST carry `data.source = "deterministic"`, a machine-readable `data.machineReadable.kind = "adverseDecisionNotice"` artifact, the same `snapshotSha256` used by the Facts-tier snapshot, and human-readable prose rendered from the same inputs. Identical snapshot, policy, template, transition, appeal, and transition-firing-timestamp inputs MUST produce byte-identical machine-readable content and byte-identical human-readable prose. The transition-firing timestamp (the processor wall-clock value captured when the adverse transition is drained) is a determining input because it is used to derive the concrete `appealDeadline` rendered into human-readable prose; holding it constant alongside the other inputs is required for byte-identity. Processors MUST NOT use model-generated Narrative-tier content as the authoritative adverse-decision notice.

For `rights-impacting` workflows, the processor MUST maintain a respondent ledger that records notice delivery, receipt confirmation, and appeal deadlines for each affected individual. Formspec Definitions that collect personal data in rights-impacting workflows MUST use the Respondent Ledger (Formspec Response S4) to track consent and data subject rights.

### 3.3 Explanation

| Level | Description | When Required |
|-------|-------------|---------------|
| `individualized` | Specific factual reasons tied to the individual's case. | REQUIRED for `rights-impacting`. |
| `categorical` | Category-level explanation. | RECOMMENDED for `operational`. |
| `aggregate` | System-level transparency without individual explanation. | Minimum for `informational`. |

### 3.4 Counterfactual Explanation

When the kernel's `impactLevel` is `rights-impacting`, adverse decision explanations MUST include counterfactuals:

- **Positive counterfactual:** What controllable factors would change the outcome.
- **Negative counterfactual:** What irrelevant factors, including protected characteristics, did NOT affect the outcome.

This is a legal and due process requirement that predates AI. It exists because an affected individual has a right to understand what they can change and what did not matter.

### 3.5 Appeal

1. An appeal MUST be reviewed by a human adjudicator independent of the original determination.
2. Filing an appeal MUST produce a provenance record.
3. The appeal process MUST be described in the adverse decision notice.

### 3.6 Continuation of Service

When the appeal mechanism's `continuationOfServices` property is `true`, the workflow MUST include topology that freezes adverse impacts and maintains current service levels during the appeal window. This is a structural workflow requirement, not a policy preference.

The exact services maintained — and for how long — are declared by `AppealMechanism.continuationPolicyRef`, which resolves to a `ContinuationPolicy.id` in the targeted Due Process Config sidecar (`schemas/governance/wos-due-process.schema.json`). The processor MUST resolve the reference against ContinuationPolicy entries in any Due Process Config sidecar targeting this governance document. An unresolvable reference MUST emit a configuration warning in provenance; the processor MAY then fall back to the looser `AppealMechanism.continuationScope` string when present, or to implementation-defined behavior. When `continuationOfServices` is true and neither `continuationPolicyRef` nor `continuationScope` resolves, the processor MUST emit a configuration error — silently shipping a workflow that promises continuation without specifying its scope is a due-process failure.

### 3.7 Governance Attachment

Due process requirements attach to transitions tagged `adverse-decision` via the `lifecycleHook` seam (Kernel S10.4). When a transition tagged `adverse-decision` fires, the processor MUST enforce the due process policy declared in the Workflow Governance Document.

Due process, review protocol bindings, and hold policies MAY include a `scope` property -- a FEL expression evaluated against the evaluation context (Runtime Companion S8.2). When a scope expression evaluates to `false`, the governance rule is not enforced for that instance. When absent, the rule applies unconditionally.

---

## 4. Review Protocols

This section is normative.

Review protocols address the empirical finding that naive review (presenting a recommendation and asking for confirmation) degrades decision quality. These protocols apply to ANY review step -- human reviewing human work, human reviewing external data, or (with Layer 2) human reviewing agent output.

### 4.1 Protocol Definitions

| Protocol | Semantics | Empirical Basis |
|----------|-----------|-----------------|
| `independentFirst` | Reviewer forms and records an independent assessment before any recommendation is revealed. The interface MUST enforce this ordering. | Bucinca et al. (CSCW 2021): cognitive forcing functions reduce overreliance on recommendations. |
| `considerOpposite` | After viewing a recommendation, the reviewer articulates reasons the recommendation might be wrong before confirming. | Anchoring bias research: consider-the-opposite debiases initial judgments. |
| `calibratedConfidence` | Calibrated confidence scores displayed alongside the recommendation. Per-field scores shown when available. Low-confidence fields visually highlighted. | Li et al. (2024): miscalibrated confidence impairs appropriate reliance. |
| `dualBlind` | Two independent reviewers assess the case without seeing each other's assessment. Results are reconciled. | Standard practice for high-stakes adjudication. |
| `unassisted` | No recommendation or assistance is provided. The task is performed entirely by the reviewer's professional judgment. | Baseline for tasks requiring unmediated professional judgment. |

### 4.2 Protocol Combination

Multiple protocols MAY be combined on a single review step. When `independentFirst` is specified, the processor MUST enforce that the reviewer's independent assessment is recorded before any recommendation is accessible.

### 4.3 Governance Attachment

Review protocols attach to transitions tagged `review` via the `lifecycleHook` seam. A governance document declares: "all transitions tagged `review` use protocol `independentFirst`." This applies across the workflow without naming specific transitions. Transition-specific overrides are available when tag-based governance is not specific enough.

---

## 5. Data Validation Pipelines

This section is normative.

### 5.1 Overview

A data validation pipeline is a staged processing chain where each stage validates data against contracts with assertion gates between stages. Pipelines validate ANY untrusted data source -- external documents, third-party data feeds, imported records, or (with Layer 2) agent output.

### 5.2 Pipeline Structure

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Unique pipeline identifier. |
| `stages` | array of Stage | REQUIRED | Ordered processing stages. |
| `description` | string | OPTIONAL | Human-readable pipeline description. |

### 5.3 Stage Types

| Type | Description |
|------|-------------|
| `contract-validation` | Validate data against a Formspec Definition or JSON Schema contract. |
| `assertion-gate` | Apply assertion checks between processing stages. |
| `transform` | Transform data using Formspec Mapping DSL (Mapping S2.4) or implementation-defined transformation. |
| `human-review` | Route to a human for review. |

### 5.4 Assertion Gates

Assertion gates are deterministic validation checks applied between pipeline stages. Each gate produces a pass/fail result with structured evidence.

| Gate Type | Description | Verification Method |
|-----------|-------------|-------------------|
| `source-grounded` | Extracted values must appear in the source document. | String/value matching against source. |
| `arithmetic` | Computed values must satisfy arithmetic constraints. | Recomputation. |
| `range` | Values must fall within declared ranges. | Bounds check. |
| `consistency` | Values must not contradict values from prior stages. | Cross-reference. |
| `format` | Values must conform to declared format constraints. | Parse check. |
| `cross-document` | Values must be consistent across multiple documents. | Cross-document matching. |
| `temporal` | Date/time values must satisfy temporal ordering constraints. | Temporal comparison. |

### 5.5 Pipeline Provenance

Each pipeline stage MUST record its inputs, outputs, and gate results in provenance. The pipeline's risk profile is determined by the weakest validation gate, not the most powerful processing stage.

### 5.6 Governance Attachment

Data validation pipelines attach via the `contractHook` seam (Kernel S10.2). Pipelines are triggered when data enters the workflow from external sources or when contract validation is required at a processing boundary.

---

## 6. Structured Audit

This section is normative.

### 6.1 Overview

Structured audit extends the kernel's Facts tier (Kernel S8) with two interpretive tiers. These tiers serve human decisions as much as any future AI decisions.

Audit claims must be **testable** in the sense of Kernel §1.2 Design Goal 6 (verifiability): every normative Reasoning or Counterfactual tier requirement in this section MUST either (a) be enforceable by schema constraint, (b) produce a lint rule that rejects documents violating it, or (c) reduce to a conformance fixture demonstrating compliant vs. non-compliant behavior. Prose describing audit shape without a corresponding test is not normative — it belongs in the companion or in non-normative commentary. Authors adding a new audit obligation here MUST also add the corresponding test artifact.

### 6.2 Reasoning Tier

The Reasoning tier records how a decision was reached:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `rulesApplied` | array of RuleReference | REQUIRED | Which rules were applied, with authority citation. |
| `evidenceConsulted` | array of EvidenceReference | REQUIRED | Which evidence was examined. |
| `criteriaChecked` | array of CriterionResult | REQUIRED | Which criteria were checked and their results. |
| `decisionRequirements` | DecisionRequirements | OPTIONAL | Declaration of required inputs, applicable rules, and citing authority. |

A **RuleReference** identifies a rule that was applied in a decision, with its authoritative source:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `ruleId` | string | REQUIRED | Identifier of the rule (e.g., statute section, regulation paragraph, policy ID). |
| `description` | string | OPTIONAL | Human-readable summary of the rule. |
| `sourceAuthority` | enum | OPTIONAL | Authority level of the rule's source. One of: `statute` (rank 1), `regulation` (rank 2), `policy` (rank 3), `guideline` (rank 4). Default: `policy`. |
| `citation` | string | OPTIONAL | Formal citation to the authoritative source (e.g., "42 CFR § 435.916"). |

The `sourceAuthority` enum determines ordering in explanation assembly (Runtime Companion S9.3): reasoning elements from higher-authority sources appear first. This is distinct from the `authority` property on Delegation (S11), which declares the scope of delegated power.

### 6.3 Decision Requirements

Decision requirements are metadata declarations -- they record which inputs were required, which rules from which authority, and which date-effective thresholds. They are NOT a decision engine; they declare what was needed, not how to evaluate it.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `requiredInputs` | array of string | REQUIRED | Data fields required for this decision. |
| `applicableRules` | array of RuleReference | REQUIRED | Rules governing this decision, with regulatory citations. |
| `effectiveDate` | string (date) | OPTIONAL | Date as of which rules and thresholds apply. |

### 6.4 Counterfactual Tier

The Counterfactual tier records what would change the outcome. REQUIRED for adverse decisions in `rights-impacting` workflows.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `positiveCounterfactuals` | array of Counterfactual | REQUIRED | What controllable factors would change the outcome. |
| `negativeCounterfactuals` | array of Counterfactual | REQUIRED | What irrelevant factors, including protected characteristics, did NOT affect the outcome. |

This is a due process requirement -- not AI-specific. When a caseworker applies eligibility criteria, the counterfactual records what the applicant could change to qualify (positive) and confirms that protected characteristics did not factor into the decision (negative).

### 6.5 Governance Attachment

Audit tiers attach via the `provenanceLayer` seam (Kernel S10.3). The Reasoning tier is REQUIRED for all transitions tagged `determination`. The Counterfactual tier is REQUIRED for transitions tagged `adverse-decision` in `rights-impacting` workflows.

---

## 7. Quality Controls

This section is normative.

### 7.1 Review Sampling

A configurable percentage of decisions MUST be randomly selected for quality review. The sampling configuration declares:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `rate` | number (0-1) | REQUIRED | Fraction of decisions to sample. |
| `method` | enum | OPTIONAL | Sampling method: `random` (default), `stratified`. |
| `scope` | enum | OPTIONAL | Sampling scope: `workflow` (default), `actor`. |

### 7.2 Separation of Duties

The reviewer of a decision MUST NOT be the same actor who made the original decision.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `scope` | enum | REQUIRED | `sameInstance` (within one workflow instance) or `global` (across all instances). |
| `excludeRoles` | array of string | OPTIONAL | Roles excluded from reviewing their own category of work. |

### 7.3 Override Authority

When a reviewer overrides a prior decision, the override MUST include:

1. **Structured rationale** explaining the basis for the override.
2. **Authority verification** confirming the reviewer has override authority.
3. **Supporting evidence** referenced by the rationale.

The runtime record shape is the `OverrideRecord` `$def` in the [Workflow Governance schema](../../schemas/governance/wos-workflow-governance.schema.json) (`#/$defs/OverrideRecord`). Each accepted override appends one OverrideRecord to provenance; records are immutable. The three required fields (`rationale`, `authorityVerification`, `supportingEvidence`) correspond to the three switches on `OverrideAuthority`: when a switch is `true`, the corresponding field MUST be non-empty in every emitted record.

### 7.4 Governance Attachment

Quality controls attach to transitions tagged `quality-check` via the `lifecycleHook` seam. Review sampling and separation of duties apply to transitions tagged `review` and `determination`.

---

## 8. Rejection and Remediation

This section is normative.

### 8.1 Rejection Policy

Pipeline stages and assertion gates declare what happens when validation fails:

| Policy | Behavior |
|--------|----------|
| `retryWithCorrections` | Return to submitter with structured explanation of what failed. |
| `escalateToSupervisor` | Route to authority with override capability. |
| `holdPendingData` | Suspend pending external data resolution. |
| `failWithExplanation` | Terminate with structured rejection record. |

### 8.2 Rejection Provenance

Every rejection MUST record:

1. Which gate or validation failed.
2. What the input was.
3. What the threshold or constraint was.
4. What would pass (when determinable).

### 8.3 Governance Attachment

Rejection policies are declared on pipeline stages and assertion gates. They compose with the kernel's lifecycle -- a rejection event triggers a lifecycle transition, which the kernel handles deterministically.

---

## 9. Task Catalog

This section is informative.

### 9.1 Verifiability Matrix

The task catalog classifies task patterns by whether their outputs can be independently verified:

| Task Pattern | Verifiable? | Verification Method |
|-------------|-------------|-------------------|
| Field extraction from document | Yes | Source-grounding: value appears in source. |
| Completeness checking | Yes | Field presence/absence against schema. |
| Document classification | Yes | Keyword/heuristic sanity check. |
| Arithmetic validation | Yes | Recomputation. |
| Date/format validation | Yes | Parse check. |
| Priority scoring | Partially | Range check, outlier detection. |
| Case summarization | Partially | Consistency check against extracted facts. |
| Sufficiency judgment | No | Requires domain expertise. |
| Credibility assessment | No | Requires domain expertise. |

Tasks in the "No" rows are determination tasks. Layer 2 SHOULD flag any attempt to assign them to an agent without appropriate governance.

### 9.2 Screener Integration

Workflows that include intake classification and routing MAY use Formspec Screener Documents (Screener S4-S8) for intake routing via Determination Records.

---

## 10. Task Management

This section is normative. Task management defines processor-side semantic requirements for task lifecycle, assignment, and SLA enforcement. These requirements are enforced by a Workflow Governance Complete processor; they are not declared in the Workflow Governance Document (the governance schema does not include task management properties). Task management is inherent in the processor's implementation of `createTask` actions (Kernel S9.2).

### 10.1 Task Lifecycle

Tasks created by kernel `createTask` actions follow this lifecycle:

| State | Description |
| ----- | ----------- |
| `created` | Task has been created but not yet assigned to an actor. |
| `assigned` | Task has been assigned to an actor but work has not started. |
| `claimed` | Actor has claimed the task and work is in progress. |
| `completed` | Actor has finished the task with a result. |
| `failed` | Task could not be completed. Triggers rejection policy if configured. |
| `delegated` | Task has been reassigned to a different actor. |
| `escalated` | Task has been escalated to a higher authority. |
| `skipped` | Task was skipped (e.g., not applicable for this case). Requires a structured rationale. |

Valid task state transitions:

- `created` -> `assigned`, `skipped`
- `assigned` -> `claimed`, `delegated`, `escalated`, `skipped`
- `claimed` -> `completed`, `failed`, `delegated`, `escalated`
- `delegated` -> `assigned`
- `escalated` -> `assigned`

Tasks in `completed`, `failed`, or `skipped` are terminal. All task state transitions MUST be recorded in provenance.

### 10.2 Assignment Roles

Tasks declare assignment roles that control who can act on the task:

| Role | Description |
| ---- | ----------- |
| `owner` | The actor responsible for the task's completion. Has full control over the task lifecycle. |
| `nominee` | An actor nominated to perform the task. Must claim the task before working on it. |
| `potentialOwner` | A pool of actors who may claim the task. First-claim-wins semantics. |
| `businessAdministrator` | An actor who can reassign, delegate, or escalate the task. Cannot perform the task itself. |
| `excludedOwner` | An actor explicitly excluded from assignment. Used to enforce separation of duties (S7.2). |

When multiple roles are specified, the following precedence applies: `excludedOwner` overrides all other roles. An actor appearing in both `potentialOwner` and `excludedOwner` is excluded.

### 10.3 Task SLA Definitions

Task SLA definitions declare time-based targets for task completion. SLAs compose with the kernel's timeout categories (Kernel S9.7) but operate at the governance level -- a timeout event fires when a kernel duration expires, while SLA violations trigger governance responses.

| Property | Type | Required | Description |
| -------- | ---- | -------- | ----------- |
| `targetDuration` | string (ISO 8601 duration) | REQUIRED | The expected completion time for this task. |
| `warningThreshold` | number (0-1) | OPTIONAL | Fraction of `targetDuration` at which a warning is generated. Default: 0.75. |
| `breachPolicy` | enum | REQUIRED | Action taken when `targetDuration` is exceeded. |

Breach policies:

| Policy | Description |
| ------ | ----------- |
| `escalate` | Escalate the task to the `businessAdministrator`. |
| `reassign` | Reassign the task from the task pool (`potentialOwner`). |
| `notify` | Notify the `businessAdministrator` but take no automated action. |
| `extend` | Automatically extend the deadline and record the extension in provenance. |

SLA evaluation uses business calendar days when a Business Calendar sidecar is present. Otherwise, SLA evaluation uses wall-clock time.

### 10.4 Task SLA Authoring

§10.3 specifies normative processor behaviour for SLAs, but without authoring surface those obligations are only fulfillable by out-of-band configuration. This subsection adds four first-class properties on `TaskPattern` (governance schema `TaskPattern` $def) so governance documents can declaratively author the SLA policy the §10.3 processor will enforce. All four properties are OPTIONAL on `TaskPattern`; when present, they MUST conform to the shapes below.

| Property | Type | Required | Description |
| -------- | ---- | -------- | ----------- |
| `slaDefinitions` | array of `SlaDefinition` | OPTIONAL | Named SLA windows measured against this task pattern. Multiple entries MAY coexist on one task (e.g. `firstResponse` plus `fullResolution`). |
| `warningThresholds` | array of `WarningThreshold` | OPTIONAL | Pre-breach notifications fired at declared lead times before any matching `slaDefinitions` entry elapses. |
| `breachPolicy` | `BreachPolicy` | OPTIONAL | Processor action when an SLA window elapses without completion: `notify`, `escalate`, `autoReassign`, or `fail`. |
| `escalationChain` | array of `EscalationStep` | OPTIONAL | Ordered ladder activated when `breachPolicy.action = escalate` or when a prior step's grace period exhausts. |

#### 10.4.1 `slaDefinitions`

Each `SlaDefinition` declares one named SLA window: `{ id, expectedDuration, calendarType, calendarRef?, startAt, startEvent? }`. `expectedDuration` MUST be an ISO 8601 duration (for example `P1D`, `PT4H`); the `P<N>BD` business-day form is a WOS extension resolved against `calendarRef`. The `"indefinite"` form is deliberately rejected here (it is valid on `HoldPolicy.expectedDuration` but not on SLAs: an indefinite SLA has no elapse point for `warningThresholds` or `breachPolicy` to fire against). `calendarType` is `wall-clock` or `business`; when `business`, the processor SHOULD resolve `calendarRef` to a Business Calendar sidecar (lint G-023). `startAt` selects the clock origin -- `assignment`, `activation`, or `custom-event`. When `startAt = custom-event`, `startEvent` is REQUIRED and MUST name a kernel event. Event-name resolution is left to a future T2 lint.

Examples:

```json
{ "id": "firstResponse", "expectedDuration": "PT4H", "calendarType": "wall-clock", "startAt": "assignment" }
```

```json
{ "id": "fullResolution", "expectedDuration": "P5BD", "calendarType": "business", "calendarRef": "urn:wos:sidecar:business-calendar:fy2026-federal", "startAt": "custom-event", "startEvent": "applicantResponseReceived" }
```

#### 10.4.2 `warningThresholds`

Each `WarningThreshold` is `{ beforeBreach, templateRef, notify }`. `beforeBreach` is an ISO 8601 duration specifying the lead time before any matching `slaDefinitions.expectedDuration` elapses at which the warning fires; the processor evaluates every threshold independently, so escalating leads (for example `P1D`, `PT4H`, `PT30M`) fan out multiple notifications. `templateRef` MUST resolve into a Notification Template sidecar (lint G-063) and SHOULD carry the `sla-warning` category. `notify` is a non-empty array of actor identifiers resolved per Governance §10.2 / §11.

Examples:

```json
{ "beforeBreach": "P1D", "templateRef": "slaWarning1Day", "notify": ["taskOwner"] }
```

```json
{ "beforeBreach": "PT30M", "templateRef": "firstResponseImminentBreach", "notify": ["taskOwner", "caseSupervisor"] }
```

#### 10.4.3 `breachPolicy`

`BreachPolicy` is `{ action, templateRef?, escalationChainRef?, timeoutPolicy? }`. `action` is `notify` (send breach template, no state change), `escalate` (advance through `escalationChain`), `autoReassign` (rotate to another `potentialOwner`), or `fail` (transition task to `failed`, invoking the rejection policy). `templateRef` is a Notification Template sidecar reference rendered on breach. `escalationChainRef` is meaningful only when `action = escalate`; it names a step in this task's `escalationChain`. `timeoutPolicy.onRepeatedBreach` controls behaviour when the same pattern breaches repeatedly (`suspend`, `fail`, or `continue`). Cross-reference integrity for the template and chain refs is deferred to a future T2 lint.

Examples:

```json
{ "action": "escalate", "escalationChainRef": "level-1", "timeoutPolicy": { "onRepeatedBreach": "suspend" } }
```

```json
{ "action": "notify", "templateRef": "slaBreachNotice" }
```

#### 10.4.4 `escalationChain`

Each `EscalationStep` is `{ level, assignTo, gracePeriod, onExhaustion }`. `level` is an integer `>= 1`; levels SHOULD be contiguous starting at 1 and the processor walks them in ascending order. `assignTo` is the actor the task reassigns to when this step activates; the reassignment is recorded in provenance as a delegated task. `gracePeriod` is an ISO 8601 duration (`P<N>BD` also permitted, resolved against the SLA's declared calendar). `onExhaustion` is `escalate` (advance to the next step), `fail` (transition task to `failed`), or `ticketCreate` (open an out-of-band ticket and park the task pending manual intervention).

Examples:

```json
{ "level": 1, "assignTo": "teamLead", "gracePeriod": "PT4H", "onExhaustion": "escalate" }
```

```json
{ "level": 2, "assignTo": "divisionDirector", "gracePeriod": "P1D", "onExhaustion": "ticketCreate" }
```

#### 10.4.5 Future work

Cross-reference integrity across the four SLA authoring shapes is currently unenforced and tracked as a future T2 lint:

- `SlaDefinition.startEvent` MUST name a kernel event declared on the target Kernel Document (when `startAt = custom-event`).
- `BreachPolicy.escalationChainRef` MUST resolve to an `EscalationStep` (by `level` or id) on the same `TaskPattern`.
- `WarningThreshold.templateRef` and `BreachPolicy.templateRef` MUST resolve through a Notification Template sidecar (lint **G-063**).
- When `SlaDefinition.calendarType = business`, `calendarRef` SHOULD be present and resolvable to a Business Calendar sidecar (lint **G-023**).

Schema enforcement in this release covers shape only; resolvability is an authoring-time lint concern. The §10.3 processor obligations remain normative regardless of which of the four properties a governance document chooses to author.

---

## 11. Delegation of Authority

This section is normative.

### 11.1 Overview

Delegation defines who is **authorized** to make specific types of determinations. This is distinct from task assignment (Kernel S3.4, which determines who performs work) and separation of duties (S7.2, which determines who may NOT review their own work). Delegation answers: "does this actor have the legal authority to sign this determination?"

In government workflows, determination authority flows from statutory grants through chains of delegation. A department head delegates signing authority to a division director, who may sub-delegate to a senior caseworker. Each delegation has a scope (impact levels, case types, dollar thresholds) and a legal basis.

### 11.2 Delegation Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Unique delegation identifier. |
| `delegator` | string | REQUIRED | Actor reference of the authority granting the delegation. |
| `delegate` | string | REQUIRED | Actor reference receiving the delegated authority. |
| `scope` | DelegationScope | REQUIRED | Limits on the delegated authority. |
| `authority` | enum | REQUIRED | Type of authority delegated: `signing`, `determination`, `review`, or `override`. |
| `legalInstrument` | string | OPTIONAL | Reference to the legal delegation order, memorandum, or statutory citation. |
| `effectiveDate` | string (date) | OPTIONAL | Date the delegation becomes active. |
| `expirationDate` | string (date) | OPTIONAL | Date the delegation expires. |
| `revocable` | boolean | OPTIONAL | Whether the delegation can be revoked. Default: `true`. |
| `revokedDate` | string (date) | OPTIONAL | Date the delegation was revoked, if applicable. |

### 11.3 Delegation Scope

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `impactLevels` | array of enum | OPTIONAL | Impact levels this delegation covers (e.g., `["operational"]`). |
| `caseTypes` | array of string | OPTIONAL | Case types this delegation covers. |
| `maxDollarThreshold` | number | OPTIONAL | Maximum dollar value for determinations under this delegation. |
| `conditions` | string (FEL) | OPTIONAL | Additional FEL expression narrowing the delegation scope. |

### 11.4 Enforcement

When a transition tagged `determination` fires (via `lifecycleHook`, Kernel S10.4), the processor MUST verify that the acting actor has a valid, non-expired, non-revoked delegation covering the determination's scope. Determinations made without a valid delegation are conformance errors.

The delegation used for a determination MUST be referenced in the provenance record for that transition.

### 11.5 Sub-Delegation

A delegate MAY further delegate authority (sub-delegation) only if the original delegation permits it. The `maxDelegationDepth` property on the governance document controls the maximum chain length. Default: `1` (no sub-delegation). This prevents unbounded delegation chains that obscure accountability.

### 4.9 Quorum-Based Delegation

A delegation chain MAY require a quorum — that is, authorization by N of M distinct authorities — rather than a single delegated authority. Quorum-based delegation is a governance capability applicable to any high-stakes operation (adverse decision, irreversible lifecycle fact, exceptional access grant).

A quorum-based delegation MUST declare:

- `quorumCount`: the minimum number of distinct authorities required (N).
- `quorumPool`: the set of eligible authorities (M).
- The requirement that each counted authority be a distinct principal (not the same principal exercising multiple roles).

A quorum-based delegation MUST NOT:

- Count the same principal more than once toward quorum.
- Silently reduce the quorum count. Reductions MUST be recorded as explicit policy transitions.

The cryptographic mechanism for proving quorum participation (threshold signatures, multi-party computation, manual countersigning) is implementation-defined and binding-specific. A monolithic implementation MAY satisfy quorum purely through database-recorded approvals.

### 11.6 Governance Attachment

Delegation enforcement attaches via `lifecycleHook` on `determination`-tagged transitions.

---

## 12. Typed Hold Policies

This section is normative.

### 12.1 Overview

Government workflows frequently enter hold states -- cases suspended pending external conditions. A benefits case may be held pending applicant response, external income verification, legal review, pending legislation, or a related case's resolution. Each hold type has different expected duration, resume conditions, timeout behavior, and notification requirements.

Typed hold policies attach to kernel states tagged `hold` via the `lifecycleHook` seam (Kernel S10.4).

### 12.2 Hold Policy Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `holdType` | enum | REQUIRED | The reason for the hold. Standard values: `pending-applicant-response`, `pending-external-verification`, `pending-legal-review`, `pending-legislation`, `pending-related-case`, `voluntary-hold`, `legal-hold`. Extensible via `x-` prefixed values. |
| `expectedDuration` | string | REQUIRED | ISO 8601 duration or the string `"indefinite"`. |
| `resumeTrigger` | string | REQUIRED except `legal-hold` | Event name that resumes the case from this hold. The processor listens for this event -- NOT a FEL condition polled on a schedule. `legal-hold` has no event-based resume trigger and is released only by an explicit legal-hold-release fact. |
| `timeoutAction` | enum | REQUIRED except `legal-hold` | Action when `expectedDuration` expires without the resume trigger: `escalate`, `auto-resume`, or `cancel`. `legal-hold` has no timeout action. |
| `notificationTemplateRef` | string | OPTIONAL | Reference to a Notification Template sidecar for hold notifications. |
| `description` | string | OPTIONAL | Human-readable description of this hold policy. |

### 12.3 Hold Type Semantics

| Hold Type | Typical Duration | Resume Trigger | Timeout Action |
|-----------|-----------------|----------------|----------------|
| `pending-applicant-response` | P30D | `applicantResponse` | `escalate` |
| `pending-external-verification` | P14D | `verificationComplete` | `escalate` |
| `pending-legal-review` | P60D | `legalReviewComplete` | `escalate` |
| `pending-legislation` | indefinite | `legislationEnacted` | N/A |
| `pending-related-case` | indefinite | `relatedCaseResolved` | N/A |
| `voluntary-hold` | P90D | `holdReleased` | `auto-resume` |
| `legal-hold` | indefinite | N/A | N/A |

### 12.4 Interaction with Timers

Hold policies compose with the kernel's timer mechanism (Kernel S9.7). When a case enters a `hold`-tagged state, the processor SHOULD start a timer with the `expectedDuration`. When the timer fires, the processor executes the `timeoutAction`. When the `resumeTrigger` event arrives before the timer fires, the timer is cancelled.

### 12.5 Governance Attachment

Hold policies attach via `lifecycleHook` on `hold`-tagged transitions and states.

### 7.15 Legal Hold (Distinct from Workflow Hold)

A **legal hold** is a distinct hold type with statutory-override semantics. Unlike workflow holds (S12 Typed Hold Policies), which suspend a workflow pending an event or condition and expect eventual resumption, a legal hold:

- Blocks data destruction, retention expiry, and scheduled lifecycle operations regardless of ordinary workflow state.
- Survives terminal workflow states. A case under legal hold MUST NOT be purged, archived, or cryptographically erased even if the workflow has otherwise concluded.
- Does NOT have an event-based resume trigger. Release requires an explicit legal-hold-release fact, typically tied to external legal authority.
- Takes precedence over retention policies when both apply.

Implementations MUST:

- Record legal-hold placement and release as canonical facts with the authority (court order, agency directive, statutory trigger) recorded in the fact.
- Propagate legal-hold state to derived artifacts (exports, projections) so that downstream systems honor the hold.
- Log any attempt to destroy, archive, or export data under legal hold as a rejected operation with the hold reference in the rejection provenance.

Legal hold is an ORTHOGONAL dimension to workflow lifecycle state. A case MAY simultaneously be in a terminal workflow state and under an active legal hold.

---

## 13. Temporal Parameter Resolution

This section is normative.

### 13.1 Overview

Government workflows apply rules effective at specific dates, not today's date. Income thresholds, eligibility criteria, benefit rates, and filing deadlines change on known dates. The Policy Parameter Config sidecar declares date-indexed parameter values; this section defines how they resolve.

### 13.2 Resolution Mechanism

Temporal parameter resolution is a specific case of the kernel's evaluation context enrichment (Kernel S7.3). The workflow:

1. The **Policy Parameter Config sidecar** declares date-indexed parameter values and their resolution date references (e.g., `eligibilityThreshold` resolves against the `applicationDate` case state field).
2. **Layer 1** resolves each parameter to its date-effective value by looking up the parameter table at the referenced date.
3. **Layer 1** injects the resolved values into the evaluation context via `lifecycleHook`.
4. By the time any FEL expression evaluates, the context contains the correct date-effective values.

The workflow author writes `caseFile.income < parameters.eligibilityThreshold`. The resolution is automatic.

### 13.3 Composition with Business Calendar

Temporal parameter resolution composes with the Business Calendar sidecar: "the income threshold effective on the application date, adjusted for the applicable business calendar."

---

## 14. Inter-Document Referencing

A Workflow Governance Document targets a WOS Kernel Document via the `targetWorkflow` property. This follows the Formspec sidecar pattern -- a companion document that targets a parent by URL reference.

```json
{
  "$wosWorkflowGovernance": "1.0",
  "targetWorkflow": "https://agency.gov/workflows/benefits-adjudication",
  "...": "..."
}
```

The `targetWorkflow` value MUST match the `url` property of the target Kernel Document. When the target Kernel Document does not declare a `url`, the governance document MAY use an implementation-defined reference mechanism.

---

## References

### Normative References

- [WOS Kernel] Formspec Working Group, "WOS Kernel Specification v1.0".
- [Formspec Core] Formspec Working Group, "Formspec Core Specification v1.0".
- [Screener] Formspec Working Group, "Formspec Screener Specification v1.0".
- [Mapping] Formspec Working Group, "Formspec Mapping DSL Specification v1.0".

### Informative References

- State v. Loomis, 881 N.W.2d 749 (Wis. 2016).
- Houston Federation of Teachers v. Houston ISD, 251 F. Supp. 3d 1168 (S.D. Tex. 2017).
- Administrative Procedure Act (APA), 5 U.S.C. 551-559.
- Equal Credit Opportunity Act (ECOA) Regulation B, 12 C.F.R. Part 1002.
- OMB Memorandum M-24-10, "Advancing Governance, Innovation, and Risk Management for Agency Use of Artificial Intelligence", March 2024.
- EU AI Act, Regulation (EU) 2024/1689.
- Vaccaro, M. et al., "When combinations of humans and AI are useful: A systematic review and meta-analysis", Nature Human Behaviour, 2024.
- Bucinca, Z. et al., "To trust or to think: Cognitive forcing functions can reduce overreliance on AI in AI-assisted decision-making", CSCW, 2021.
- Li, B. et al., "Calibrated confidence in AI decision-making", 2024.
- OpenFisca, "Open Source Platform for Tax and Benefit Systems", https://openfisca.org.
