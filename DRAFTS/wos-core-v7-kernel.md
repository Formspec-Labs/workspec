---
title: Workflow Orchestration Standard (WOS) Core Specification
version: 7.0.0-draft.1
date: 2026-04-09
status: draft
---

# Workflow Orchestration Standard (WOS) Core Specification v7.0

**Version:** 7.0.0-draft.1
**Date:** 2026-04-09
**Editors:** Formspec Working Group
**Companion to:** Formspec v1.0 -- A JSON-Native Declarative Form Standard

---

## Abstract

The Workflow Orchestration Standard (WOS) is a declarative governance layer for high-stakes, long-running workflows in which humans and AI agents collaborate on consequential decisions. WOS defines an actor model, impact classification, deontic constraint framework, autonomy levels, structured oversight protocols, audit architecture, due process requirements, and durable execution guarantees. It does not define lifecycle topology, task management, decision logic, or integration mechanics -- those are delegated to tier specifications.

WOS uses Formspec Definitions as its recommended interface contract and FEL as its expression language. A Formspec processor that does not implement WOS remains fully conformant to Formspec. WOS is additive: it governs the orchestration envelope without altering core Formspec processing semantics.

---

## Status of This Document

This document is a **draft specification**. It is a companion framework to Formspec v1.0 and does not modify that specification's processing model. Implementors are encouraged to experiment with this specification and provide feedback, but MUST NOT treat it as stable for production use until a 1.0.0 release is published.

---

## 1. Introduction

### 1.1 Background

High-stakes workflows -- grants processing, benefits adjudication, licensing, inspections, investigations, compliance review -- share requirements that no existing standard adequately integrates. They are long-running, human-centric, evidence-driven, heavily regulated, and increasingly involve AI agents.

Empirical research constrains the design of any standard governing these workflows. A meta-analysis of 106 experiments demonstrates that naive human-AI combinations degrade decision quality compared to either humans or AI alone (Vaccaro et al., Nature Human Behaviour, 2024). Research on chain-of-thought faithfulness demonstrates that model-generated explanations are systematically post-hoc rationalizations (Turpin et al., NeurIPS 2023; Lanham et al., Anthropic, 2023). Documented government AI failures -- Michigan's MiDAS (93% false positive rate), Arkansas's RUGs algorithm, the Dutch childcare benefits scandal, Australia's Robodebt -- demand due process protections and appeal mechanisms.

This specification encodes these findings as structural requirements.

### 1.2 Design Goals

1. **Human authority is supreme.** Agent recommendations MUST NOT be the sole factor in adverse decisions affecting individual rights.
2. **Structured oversight, not checkbox review.** Human oversight MUST produce genuine cognitive engagement via empirically grounded protocols.
3. **Accountability requires specificity.** Every action MUST be traceable to a specific actor, authority, inputs, outputs, and rule version.
4. **Constraints are external to the agent.** The agent is outside the trust boundary. Guardrails are enforced by the WOS Processor.
5. **Graceful degradation is mandatory.** Every workflow MUST function without any agent participation.

### 1.3 Scope

**Within scope:** actor model; impact level classification; deontic constraint framework; autonomy levels; structured oversight protocols; four-layer audit architecture; due process requirements; durable execution guarantees; interface contract abstraction; separation principles; conformance profiles.

**Out of scope (with tier spec ownership):**

- Lifecycle and topology (statecharts, transitions, guards, milestones, constraint zones) -- WOS-Lifecycle
- Decision logic, temporal parameters, decision tables -- WOS-Decision
- Task management, assignment, SLAs, separation of duties -- WOS-Task
- Agent configuration, confidence framework, drift monitoring, multi-step sessions, tool use governance, attestations -- WOS-Agent
- Integration types, Arazzo sequences, CWL tools, event envelopes -- WOS-Integration
- Provenance record schemas, tamper evidence, OCEL, RO-Crate -- WOS-Provenance
- Retry policies, timeout categories, compensation mechanics -- WOS-Execution
- JSON-LD context, SHACL shapes, PROV-AGENT alignment -- WOS-Semantic profile
- Formal verification, SMT encoding, verifiable constraint subset -- WOS-Verification profile
- Typed patch operations, compositional authoring -- WOS-Authoring profile
- Formspec-specific bindings, Mapping DSL mechanics -- WOS-Formspec binding spec
- Instance migration, version evolution -- WOS-Lifecycle or WOS-Migration

### 1.4 Relationship to Formspec

This specification is a **companion framework** to Formspec v1.0. Formspec governs the data-collection instrument -- what data is collected, how it behaves reactively, how it is validated. WOS governs the orchestration envelope -- who does the work, when, under what authority, with what agent assistance, with what oversight, and what gets recorded.

WOS MUST NOT alter core Formspec processing semantics. A Formspec processor that does not implement WOS remains fully conformant to Formspec.

WOS processors MUST delegate Formspec Definition evaluation to a Formspec-conformant processor (Core S1.4). The WOS processor provides orchestration context; the Formspec processor provides Definition evaluation.

### 1.5 Notational Conventions

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [BCP 14][RFC 2119] [RFC 8174] when, and only when, they appear in ALL CAPITALS, as shown here.

JSON syntax and data types are as defined in [RFC 8259]. URI syntax is as defined in [RFC 3986].

Terms defined in the Formspec v1.0 core specification -- including *Definition*, *Item*, *Bind*, *FEL*, and *conformant processor* -- retain their core-specification meanings throughout this document unless explicitly redefined.

---

## 2. Conformance

### 2.1 Conformance Classes

**WOS Document.** A serialized workflow definition conforming to the structural and semantic requirements of this specification and any applicable tier specifications.

**WOS Processor.** A software system that consumes WOS Documents and produces behavior consistent with the semantics defined herein. A WOS Processor MUST support at least one Conformance Profile (S2.2).

### 2.2 Conformance Profiles

Three profiles are defined:

**Structural.** Parse, validate against JSON Schema, round-trip without loss, and resolve interface contract references.

**Governance.** Structural conformance plus: enforce deontic constraints (S6), enforce autonomy levels (S5), enforce structured oversight protocols (S7), enforce due process requirements (S9), and produce audit records conforming to the four-layer architecture (S8).

**Full.** Governance conformance plus: implement all applicable tier specifications, enforce durable execution guarantees (S8.2), and satisfy the companion specification contracts defined by each adopted tier spec.

---

## 3. Actor Model

This section is normative.

WOS recognizes three types of actors that participate in workflows:

| Type | Description | Provenance Requirements |
|------|-------------|------------------------|
| `human` | A person who performs tasks, makes decisions, and exercises judgment. | Identity, role, timestamp. |
| `system` | A deterministic software component that executes automated actions, integrations, and rule evaluations. | Component identifier, version, timestamp. |
| `agent` | An AI system that performs reasoning, classification, recommendation, or decision-making. Outputs carry confidence metadata and are non-deterministic. | Model identifier, model version, confidence, input summary, timestamp. |

### 3.1 Actor Type Determination

An actor's type is determined by who bears decision authority for the specific action, not by the presence or absence of AI tooling. A human using an AI tool remains a `human` actor if the human reviews and commits the output. An AI system operating autonomously is an `agent` actor even if a human configured it.

An actor's type is immutable for a given action.

### 3.2 Trust Boundary

The agent is outside the trust boundary of the governance envelope. The WOS Processor -- not the agent -- enforces constraints, validates outputs, and controls workflow progression. This separation ensures governance survives agent changes, prompt injection, and behavioral drift.

### 3.3 Normative Constraints

1. Agents MUST NOT override human decisions.
2. Provenance records MUST include the actor type.
3. Records for `agent` actors MUST include model identifier and model version.
4. Cascading autonomous agents (an agent at `autonomous` level invoking another agent at `autonomous` level) MUST be explicitly declared in the workflow definition.

---

## 4. Impact Level Classification

This section is normative.

Every WOS Document MUST declare an `impactLevel` classifying the consequence level of decisions made within the workflow. When `impactLevel` is not specified, the effective default is `operational`.

| Level | Definition | Governance Requirements |
|-------|-----------|------------------------|
| `rights-impacting` | Decisions affect individual legal rights, benefits, services, or obligations. | Full due process (S9). Agent autonomy capped at `assistive` unless elevated with attestation. Dual-readability narrative required for adverse decisions. Counterfactual audit layer required. |
| `safety-impacting` | Decisions affect individual or public safety. | Full due process (S9). Agent autonomy capped at `assistive`. |
| `operational` | Organizational operations without direct individual impact. | Due process RECOMMENDED. Agent autonomy up to `autonomous` with deontic constraints. |
| `informational` | Informational outputs; no binding decisions. | Due process OPTIONAL. No autonomy restrictions. |

The impact level governs the proportionality of governance mechanisms.

---

## 5. Autonomy Levels

This section is normative.

Every action in a WOS workflow operates at a declared autonomy level. Autonomy is a property of the action site -- the point in the workflow where an agent is invoked -- not a property of the agent itself. The same agent MAY operate at different autonomy levels in different workflow contexts.

| Level | Semantics | Fallback Requirement |
|-------|-----------|---------------------|
| `autonomous` | Agent output committed without human review. REQUIRES deontic constraints. | MUST define fallback to human. |
| `supervisory` | Agent output provisionally committed. Human reviews within a defined `reviewWindow`. If the review window expires without intervention, the output becomes final and a provenance record noting implicit confirmation is produced. | MUST define fallback to human. |
| `assistive` | Agent produces a recommendation. Human reviews, may modify, and explicitly confirms before commitment. The human owns confirmed output; provenance records the agent recommendation. | MUST define fallback to human. |
| `manual` | Action performed entirely by a human. Agent MAY provide contextual assistance on demand, but output is solely the human's. | N/A. |

### 5.1 Autonomy Constraints

1. An action with `autonomy: "autonomous"` MUST have associated deontic constraints (S6). Autonomous actions without deontic constraints are a structural error.
2. An action with `autonomy: "assistive"` MUST create a human task for confirmation.
3. An action with `autonomy: "supervisory"` MUST define a `reviewWindow` (an ISO 8601 duration).
4. The effective autonomy MUST NOT exceed the workflow's impact-level cap. For `rights-impacting` and `safety-impacting` workflows, the default cap is `assistive`.
5. Every agent invocation MUST have a reachable path to workflow completion that does not require any agent to succeed.

### 5.2 Example

```yaml
lifecycle:
  defaultAutonomy: "assistive"

  states:
    intake:
      onEntry:
        - action: "invokeAgent"
          agentRef: "documentClassifier"
          autonomy: "supervisory"
          reviewWindow: "PT4H"
      transitions:
        - event: "agent.complete"
          target: "review"

    review:
      onEntry:
        - action: "createTask"
          activityRef: "manualReview"
          oversight:
            protocol: "independentFirst"
```

---

## 6. Deontic Constraint Framework

This section is normative.

Agent constraints use four deontic types adopted from OASIS LegalRuleML. Deontic constraints are evaluated after interface contract validation and before output is committed to workflow state. The WOS Processor is the Policy Enforcement Point; the deontic constraint definitions are the Policy Decision Point.

### 6.1 Permission

A Permission bounds what the agent is allowed to produce. Agent outputs within permission bounds are accepted; outputs outside are violations.

```yaml
deonticConstraints:
  permissions:
    - id: "outputFieldScope"
      allowedFields: ["eligible", "reason", "confidence"]
      onViolation: "reject"

    - id: "eligibilityRange"
      field: "eligible"
      bounds: "value = true or value = false"
      onViolation: "reject"
```

### 6.2 Prohibition

A Prohibition forbids specified agent outputs or actions regardless of confidence.

```yaml
  prohibitions:
    - id: "noFinalDenial"
      condition: >
        output.eligible = false
        and instance.impactLevel = 'rights-impacting'
      reason: "Agent may not render final denial in rights-impacting workflow."
      onViolation: "escalateToHuman"

    - id: "noInconsistentApproval"
      condition: >
        output.eligible = true
        and caseFile.application.income
            > parameters.incomeThreshold(caseFile.application.submittedDate) * 1.5
      reason: "Approval with income far above threshold requires human review."
      onViolation: "escalateToHuman"
```

### 6.3 Obligation

An Obligation requires the agent to perform a specified action or include specified content. Checked after output, before commit.

```yaml
  obligations:
    - id: "citesRegulation"
      requirement: >
        contains(output.reason, 'CFR')
        or contains(output.reason, 'USC')
      reason: "Determination must cite regulatory authority."
      onViolation: "reject"

    - id: "perFieldConfidence"
      requirement: "output.confidence.fieldLevel != null"
      reason: "Per-field confidence required for this capability."
      onViolation: "reject"
```

### 6.4 Right

A Right specifies what the agent is entitled to receive from the WOS Processor as input context. The WOS Processor has an Obligation to provide data specified in agent Rights. A Rights violation (the processor fails to provide entitled data) MUST NOT be attributed to the agent and MUST trigger a system-level error rather than an agent fallback.

```yaml
  rights:
    - id: "receivesApplicationData"
      entitlement: "caseFile.application"
      description: "Agent must receive application data as input."

    - id: "receivesParameterValues"
      entitlement: "parameters.incomeThreshold"
      description: "Agent must receive current temporal parameter values."
```

### 6.5 Enforcement Ordering

Deontic constraints MUST be evaluated in the following order:

1. **Permissions** -- structural bounds on allowed outputs.
2. **Prohibitions** -- forbidden output patterns.
3. **Obligations** -- required output elements.
4. **Confidence floor** -- minimum certainty threshold (defined in WOS-Agent).
5. **Volume constraints** -- rate limits on autonomous actions (defined in WOS-Agent).
6. **Human review sampling** -- quality assurance selection (defined in WOS-Agent).

Items 4-6 are included for ordering completeness. Their schemas and mechanics are defined in the WOS-Agent tier specification.

When multiple constraints are violated simultaneously, the most restrictive enforcement action applies. The restriction ordering is: `reject` > `escalateToHuman` > `switchToAssistive` > `flag`.

### 6.6 Null Propagation for Deontic Constraints

When a deontic constraint expression evaluates to `null` (due to missing data or unresolvable references), the behavior is determined by the workflow's impact level:

| Impact Level | Null Behavior | Rationale |
|---|---|---|
| `rights-impacting` | `escalateToHuman` | Unknown constraint state on a rights-affecting decision requires human review. |
| `safety-impacting` | `escalateToHuman` | Safety constraints that cannot be evaluated must not silently pass. |
| `operational` | `true` (pass) | Operational constraints failing to evaluate should not block workflow. |
| `informational` | `true` (pass) | Low stakes; passing is reasonable. |

A constraint MAY override the default null behavior with an explicit `nullBehavior` property: `"pass"`, `"deny"`, or `"escalate"`.

---

## 7. Structured Oversight Protocols

This section is normative.

When a task involves AI agent assistance, the workflow MUST specify a structured oversight protocol. These protocols address the empirical finding that naive human-AI review degrades decision quality (Vaccaro et al., 2024).

| Protocol | Semantics | Empirical Basis |
|----------|-----------|-----------------|
| `independentFirst` | Reviewer forms and records an independent assessment before the agent's recommendation is revealed. The interface MUST enforce this ordering. | Buçinca et al. (CSCW 2021): cognitive forcing functions reduce overreliance on AI recommendations. |
| `considerOpposite` | After viewing the agent's recommendation, the reviewer articulates reasons the recommendation might be wrong before confirming. | Anchoring bias research: consider-the-opposite debiases initial judgments. |
| `calibratedConfidence` | Calibrated confidence displayed alongside the recommendation. Per-field scores shown when available. Low-confidence fields visually highlighted. | Li et al. (2024): miscalibrated confidence impairs appropriate reliance. |
| `dualBlind` | Two independent reviewers assess the case without seeing each other's or the agent's assessment. Results are reconciled. | Standard practice for high-stakes adjudication. |
| `unassisted` | No agent assistance is provided. The task is performed entirely by a human. | Baseline for tasks requiring unmediated professional judgment. |

Multiple protocols MAY be combined. When `independentFirst` is specified, the WOS Processor MUST enforce that the reviewer's independent assessment is recorded before the agent's output is accessible.

### 7.1 Example

```yaml
oversight:
  agentAssistance:
    agentRef: "eligibilityScreener"
    capability: "eligibilityPreScreen"
    timing: "beforeClaim"
    protocol: "independentFirst"
    presentation:
      showConfidence: true
      showAlternatives: true
      highlightLowConfidenceFields: true
      showDiffFromIndependent: true
```

When `showDiffFromIndependent` is `true`, the interface highlights differences between the reviewer's independent assessment and the agent's recommendation, focusing attention on points of disagreement.

---

## 8. Four-Layer Audit Architecture

This section is normative.

Every provenance record MUST be structured in a four-layer architecture that separates epistemic status:

| Layer | Name | Content | Authority |
|-------|------|---------|-----------|
| 1 | Immutable Facts | Timestamp, actor, model version, inputs, outputs, policy version, confidence, reviewer ID, attestation references. | **Authoritative.** Admissible evidence. |
| 2 | Structured Reasoning | Rules applied, evidence consulted, criteria checked, decision table trace. For adverse decisions in `rights-impacting` workflows: dual-readability narrative. | **Authoritative** for deterministic logic. |
| 3 | Generated Narrative | Model's natural language explanation of its reasoning. | **Informational only. Non-authoritative.** |
| 4 | Counterfactual | What would change the outcome (positive). What did NOT affect it (negative, including protected characteristics). | Informational. **Required** for adverse decisions in `rights-impacting` workflows. |

### 8.1 Epistemic Status Separation

Every provenance record that includes Layer 3 content MUST label it non-authoritative. This requirement exists because model-generated explanations are systematically unfaithful to actual reasoning processes (Turpin et al., NeurIPS 2023; Lanham et al., Anthropic, 2023). A conformant implementation MUST NOT treat Layer 3 content as dispositive evidence in any adjudicative, audit, or due process context.

Layer 1 and Layer 2 content is the authoritative basis for all due process notices, explanations, and appeal proceedings.

### 8.2 Durable Execution Guarantees

WOS defines five abstract guarantees. Tier specifications elaborate the mechanics.

| Guarantee | Requirement |
|-----------|-------------|
| **G1: Crash Recovery** | Non-terminal workflow instances MUST resume from the last durable state after a crash. |
| **G2: Persistent State** | Lifecycle state, case file data, task states, and timer registrations MUST be durably persisted. |
| **G3: Deterministic Replay** | Every action invoking a non-deterministic external service (including AI agent invocations) MUST persist the output as an immutable step result before advancing workflow state. During recovery or audit replay, the WOS Processor MUST use the persisted output rather than re-invoking the service. Re-invocation of a non-deterministic service during replay is a conformance violation. |
| **G4: Durable Timers** | Timers MUST survive restarts, fire within tolerance, and consume no runtime resources while waiting. |
| **G5: External Signal Delivery** | Signals addressed to inactive instances MUST be durably enqueued. |

Retry policies, timeout categories, and compensation mechanics are defined in WOS-Execution.

---

## 9. Due Process Requirements

This section is normative for `rights-impacting` and `safety-impacting` workflows. These requirements are informed by constitutional due process principles established in case law (*State v. Loomis*, 881 N.W.2d 749 (Wis. 2016); *Houston Federation of Teachers v. Houston ISD*, 251 F. Supp. 3d 1168 (S.D. Tex. 2017)), statutory requirements (APA, ECOA Regulation B), and regulatory frameworks (OMB M-24-10, EU AI Act Art. 13-14).

### 9.1 Notice

When a workflow produces an adverse decision (denial, reduction, termination, or other unfavorable determination), the affected individual MUST receive notice before the decision takes effect. The notice MUST include:

1. The specific determination made.
2. The factual basis using individualized reason codes, not generic statements.
3. The individual's right to appeal, including the deadline and process.
4. Disclosure that an AI system assisted in the determination, if applicable.

The notice SHOULD be derived from the provenance record's Layer 1 facts and Layer 2 structured reasoning. A `noticeGracePeriod` (ISO 8601 duration) defines the minimum delay between notice and effect.

### 9.2 Explanation

Explanation levels:

| Level | Description | When Required |
|-------|-------------|---------------|
| `individualized` | Specific factual reasons tied to the individual's case. | REQUIRED for `rights-impacting`. |
| `categorical` | Category-level explanation. | RECOMMENDED for `operational`. |
| `aggregate` | System-level transparency without individual explanation. | Minimum for `informational`. |

When the impact level is `rights-impacting`, the explanation MUST include counterfactuals: positive (what controllable factors would change the outcome) and negative (what irrelevant factors, including protected characteristics, did NOT affect the outcome).

### 9.3 Appeal

1. An appeal MUST be reviewed by a human adjudicator independent of the original determination.
2. AI agents MAY assist in preparing information for the adjudicator but MUST NOT serve as the appeal decision-maker.
3. Filing an appeal MUST produce an `appealFiled` provenance record.

### 9.4 Continuation of Service

When `continuationOfServices` is `true` in the due process configuration, the workflow MUST include topology that freezes adverse impacts and maintains current service levels during the appeal window. This is a structural workflow requirement, not a policy preference.

### 9.5 Agent Disclosure

For `rights-impacting` workflows, `discloseThatAgentAssisted` MUST be `true`. The disclosure requirement is consistent with OMB M-24-10 and EU AI Act Art. 13.

### 9.6 Example

```yaml
dueProcess:
  adverseDecisionPolicy:
    noticeRequired: true
    noticeTemplate: "adverseDecisionNotice"
    noticeTiming: "beforeEffective"
    noticeGracePeriod: "P30D"

    explanationRequired: true
    explanationLevel: "individualized"
    counterfactualRequired: true

    appealMechanism:
      enabled: true
      appealWindow: "P30D"
      appealReviewedBy:
        roles: ["humanAdjudicator"]
        constraint: "independentFromOriginal"
      continuationOfServices: true
      continuationScope: "currentLevel"

    agentDisclosure:
      discloseThatAgentAssisted: true
      discloseModelIdentity: false
      discloseConfidence: false
```

---

## 10. Interface Contract Abstraction

This section is normative.

WOS uses interface contracts to define typed data exchanges between workflow participants. An interface contract MUST provide:

1. **Typed fields** with declared data types.
2. **Cross-field validation** rules producing structured results.
3. **Structured validation results** with severity (error, warning, info), field path, message, and constraint identifier.

Two conformant bindings are defined:

| Binding | Capabilities | When to Use |
|---------|-------------|-------------|
| **JSON Schema** (baseline) | Structural validation only. No reactive behavior, no cross-field rules, no Mapping DSL. | Minimum viable integration. Teams adopting WOS governance on existing infrastructure. |
| **Formspec** (recommended) | Full reactive behavior, cross-field Shapes, Mapping DSL for data flow, sidecar ecosystem (References, Ontology, Assist, Respondent Ledger, Screener). | WOS-native implementations. Human task contracts. Any context needing calculated defaults, conditional validation, or structured AI interaction. |

Formspec is the recommended binding because it is strictly more capable. JSON Schema is the baseline for adoption.

### 10.1 Contract Contexts

WOS uses interface contracts in four contexts:

1. **Human task contracts** -- the full Formspec stack, including Mapping DSL for data flow.
2. **Agent capability contracts** -- headless Formspec Definitions (Items, Binds, Shapes only; no Presentation) specifying agent input/output schemas.
3. **Decision service contracts** -- headless Formspec Definitions for inputs and outputs.
4. **Integration contracts** -- headless Formspec Definitions for request and response schemas.

### 10.2 Processing Delegation

WOS processors MUST delegate interface contract evaluation to a conformant contract processor. For Formspec bindings, this means a Formspec-conformant processor (Core S1.4). The WOS processor provides orchestration context; the contract processor provides Definition evaluation.

Deontic constraint evaluation (S6) occurs outside the contract processing model. The WOS Processor evaluates deontic constraints against the contract processor's output (Response and ValidationReport), not during the processing cycle. The Formspec processing model (Core S2.4) is a closed system; WOS governance wraps it.

---

## 11. Separation Principles

This section is normative.

1. **Process topology MUST be separated from decision logic.** The lifecycle defines structure. Decision Services evaluate conditions. A guard MAY invoke a Decision Service, but the Decision Service MUST NOT contain process topology.

2. **Decision logic MUST be separated from task management.** Decisions determine what should happen. Tasks determine who does it and how the work is managed. These are independently versionable.

3. **Agent governance MUST be separated from agent implementation.** Deontic constraints, autonomy levels, and confidence requirements are properties of the workflow definition, enforced by the WOS Processor. They are not properties of the agent and MUST NOT be delegable to the agent.

4. **Case data MUST be separated from process state.** The case file holds business data. The lifecycle state tracks process progress. These are modeled separately.

5. **Audit MUST be separated from execution.** Provenance records are produced as a consequence of execution but do not participate in control flow. The audit layer observes; it does not control.

6. **Execution guarantees MUST be separated from execution mechanisms.** The specification defines what guarantees hold, not how they are achieved.

7. **Syntax validation MUST be separated from semantic governance.** JSON Schema validates structure. Deontic constraints and impact-level rules validate policy.

8. **Reusable templates MUST be separated from instantiation context.** ActivityDefinitions are independent of the WorkflowDefinitions that reference them.

9. **Interface contracts MUST be separated from the systems that produce and consume them.** The same Formspec Definition serves as a human task form, an agent capability contract, or a decision service interface.

---

## 12. Planned Tier Specifications

Each tier specification elaborates one domain of WOS and MUST NOT contradict this Core Specification.

| Tier Specification | Scope |
|--------------------|-------|
| **WOS-Lifecycle** | Statechart semantics, state types, transitions, guards, milestones, constraint zones (DCR-style), processing model, soundness verification. |
| **WOS-Decision** | Decision services, decision tables with hit policies, defeasible rules, temporal parameters, decision requirement graphs. |
| **WOS-Task** | Human task lifecycle, assignment model, SLAs, separation of duties, Formspec-driven task creation, override authority. |
| **WOS-Agent** | Agent configuration, capability contracts, confidence framework with decay and calibration, fallback chains, input preparation and isolation, multi-step sessions, tool use governance, behavioral attestations, drift monitoring, agent lifecycle states. |
| **WOS-Integration** | Integration types, Arazzo sequences, CWL tool descriptors, CloudEvents envelope with WOS extensions, correlation semantics, MCP/A2A alignment. |
| **WOS-Provenance** | Provenance record schemas for all 21 record types, tamper evidence (Merkle tree), Formspec provenance ingestion (ValidationReport, Respondent Ledger), OCEL 2.0 event logging, RO-Crate packaging. |
| **WOS-Execution** | Retry policies, timeout categories (step, task, instance, heartbeat, queue), compensation semantics, idempotency keys. |
| **WOS-Migration** | Instance migration contract, state mapping, case file transformation, policy-based routing (grandfather, migrateAll, migrateByState). |

### 12.1 Optional Profiles

| Profile | Scope |
|---------|-------|
| **WOS-Semantic** | JSON-LD context document, SHACL governance shapes, PROV-O and PROV-AGENT alignment, SPARQL querying. |
| **WOS-Verification** | Verifiable constraint subset (decidable FEL fragment), SMT encoding, formal proof of deontic constraint properties before deployment. |
| **WOS-Authoring** | Typed patch operations against document AST, four-stage validation pipeline, compositional authoring via ActivityDefinition registries. |
| **WOS-Formspec** | Formspec-specific binding detail, Mapping DSL mechanics for `inputMapping`/`outputBinding`, Assist Governance Proxy, sidecar integration. |

---

## 13. Security Considerations

1. **Agent impersonation.** Implementations MUST authenticate agent actors with the same rigor as human actors. An agent MUST NOT claim a human actor identity. A human MUST NOT impersonate an agent to bypass deontic constraint enforcement.

2. **Prompt injection.** Adversarial content in case data may manipulate agent behavior. Implementations SHOULD sanitize agent inputs. Deontic constraints are the authoritative boundary regardless of agent output.

3. **Model version drift.** When an agent's model version changes, behavior may change. Implementations using unpinned model versions SHOULD monitor output distribution shifts.

4. **Cascading autonomy.** Unsupervised chains of autonomous agent actions compound risk. Cascading autonomy MUST be declared (S3.3).

---

## 14. Privacy Considerations

WOS provenance records contain detailed operational data including actor identities, decision inputs, and case file mutations. Implementations MUST apply appropriate access controls to provenance data. Role-based visibility restrictions MUST prevent unauthorized access to case file data. Personal data in provenance records is subject to applicable data protection regulations.

---

## References

### Normative References

- [RFC 2119] Bradner, S. "Key words for use in RFCs to Indicate Requirement Levels." BCP 14, RFC 2119, March 1997.
- [RFC 8174] Leiba, B. "Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words." BCP 14, RFC 8174, May 2017.
- [RFC 8259] Bray, T. "The JavaScript Object Notation (JSON) Data Interchange Format." RFC 8259, December 2017.
- [RFC 3986] Berners-Lee, T., Fielding, R., Masinter, L. "Uniform Resource Identifier (URI): Generic Syntax." RFC 3986, January 2005.
- [Formspec] Formspec Working Group. "Formspec v1.0 -- A JSON-Native Declarative Form Standard."
- [FEL] Formspec Working Group. "Formspec Expression Language (FEL) Normative Grammar."

### Informative References

- [Vaccaro2024] Vaccaro, M. et al. "When combinations of humans and AI are useful." Nature Human Behaviour, 2024.
- [Turpin2023] Turpin, M. et al. "Language Models Don't Always Say What They Think." NeurIPS, 2023.
- [Lanham2023] Lanham, T. et al. "Measuring Faithfulness in Chain-of-Thought Reasoning." Anthropic, 2023.
- [Buçinca2021] Buçinca, Z. et al. "To Trust or to Think: Cognitive Forcing Functions Can Reduce Overreliance on AI." CSCW, 2021.
- [Li2024] Li, M. et al. "Calibrated confidence in human-AI decision making." 2024.
- [LegalRuleML] OASIS. "LegalRuleML Core Specification." 2021.
- [StateLoomis] *State v. Loomis*, 881 N.W.2d 749 (Wis. 2016). Risk assessment tool use in sentencing, due process requirements.
- [HoustonFed] *Houston Federation of Teachers v. Houston ISD*, 251 F. Supp. 3d 1168 (S.D. Tex. 2017). Algorithmic evaluation, due process notice.
- [OMB-M-24-10] OMB. "Memorandum M-24-10: Advancing Governance, Innovation, and Risk Management for Agency Use of Artificial Intelligence." March 2024.
- [EUAIACT] European Parliament. "Regulation (EU) 2024/1689 (Artificial Intelligence Act)." 2024.
