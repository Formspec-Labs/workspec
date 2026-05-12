# Workflow Orchestration Standard (WOS) Core Specification

## W3C First Public Working Draft

**Latest published version:**
: https://wos-spec.org/TR/wos-core/

**Editor's Draft:**
: https://wos-spec.org/ed/wos-core/

**Editors:**
: Mike (TealWolf Consulting LLC)

**Version:**
: 2.0.0

**Date:**
: 8 April 2026

**Status:**
: First Public Working Draft

---

## Abstract

This specification defines the Workflow Orchestration Standard (WOS), a declarative, machine-readable language for describing high-stakes, long-running workflows in which humans and AI agents collaborate on consequential decisions. WOS provides an eight-layer architecture separating lifecycle topology, decision logic, human task management, agent governance, case state, integration, provenance, and durable execution into independently evolvable concerns. The standard treats human authority as supreme, AI participation as governed, and audit as foundational — not afterthoughts. It is designed for workflows where errors carry consequences for individuals and the public interest: grants processing, benefits adjudication, licensing, inspections, investigations, compliance review, and similar regulated processes.

WOS is informed by empirical research demonstrating that naive human-in-the-loop designs degrade decision quality, that model-generated explanations are unreliable audit evidence, that behavioral drift between model versions can be catastrophic, and that no single defense prevents prompt injection. The standard encodes these findings as structural requirements rather than advisory guidance.

---

## Status of This Document

This section describes the status of this document at the time of its publication.

This document is a First Public Working Draft. It has not been endorsed by any standards body and has no formal standing. It is published to solicit feedback from implementers, domain experts, standards practitioners, and the broader workflow, case management, and AI governance community.

This is a living specification. The editors intend to iterate on it based on implementation experience and community review. Substantive changes will be tracked in a public changelog.

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Conformance](#2-conformance)
3. [Terminology](#3-terminology)
4. [Architecture Overview](#4-architecture-overview)
5. [Document Structure](#5-document-structure)
6. [Layer 1: Lifecycle and Topology](#6-layer-1-lifecycle-and-topology)
7. [Layer 2: Decision and Policy](#7-layer-2-decision-and-policy)
8. [Layer 3: Human Task Management](#8-layer-3-human-task-management)
9. [Layer 4: Agent Governance](#9-layer-4-agent-governance)
10. [Layer 5: Case State and Evidence](#10-layer-5-case-state-and-evidence)
11. [Layer 6: Integration and Eventing](#11-layer-6-integration-and-eventing)
12. [Layer 7: Provenance and Audit](#12-layer-7-provenance-and-audit)
13. [Layer 8: Durable Execution Contract](#13-layer-8-durable-execution-contract)
14. [Actor Model](#14-actor-model)
15. [Expression Language](#15-expression-language)
16. [Due Process Requirements](#16-due-process-requirements)
17. [Versioning and Evolution](#17-versioning-and-evolution)
18. [Security and Access Control](#18-security-and-access-control)
19. [Extensibility](#19-extensibility)
20. [Serialization](#20-serialization)
21. [Conformance Profiles](#21-conformance-profiles)
22. [Privacy Considerations](#22-privacy-considerations)
23. [Security Considerations](#23-security-considerations)
24. [References](#24-references)

**Appendices**

- [A. JSON Schema for WOS Core](#appendix-a-json-schema-for-wos-core)
- [B. Complete Example: Grant Application Workflow](#appendix-b-complete-example)
- [C. Relationship to Existing Standards](#appendix-c-relationship-to-existing-standards)
- [D. Changelog](#appendix-d-changelog)

---

## 1. Introduction

### 1.1 Background

High-stakes workflows share requirements that no existing standard adequately integrates. They are long-running: a single case may span weeks, months, or years. They are human-centric: professional judgment, discretionary action, and override authority are core operating modes. They are evidence-driven: decisions depend on accumulated documents, data, and findings. They are heavily regulated: every action must be auditable, explainable, and traceable. And they increasingly involve AI agents: systems capable of classification, extraction, recommendation, and reasoning that participate alongside human workers.

The participation of AI agents in consequential workflows introduces requirements that prior workflow standards did not contemplate. Empirical research demonstrates that naive human-in-the-loop designs — presenting an AI recommendation and asking a human to confirm — actually degrade decision quality compared to either humans or AI operating independently (Vaccaro et al., Nature Human Behaviour, 2024; meta-analysis of 106 experiments). Model-generated explanations are systematically unfaithful to actual reasoning processes (Turpin et al., NeurIPS 2023; Lanham et al., Anthropic, 2023). Behavioral drift between model versions can cause near-total performance collapse on specific tasks (Chen, Zaharia, and Zou, 2023). And no single defense against prompt injection has proven robust against adaptive adversaries (Nasr et al., 2025). These are not theoretical concerns — they have produced documented harm in government deployments including Michigan's MiDAS system (93% false positive rate), Arkansas's RUGs algorithm (43% average service reduction for disabled beneficiaries), and the Dutch childcare benefits scandal (26,000+ wrongly accused families, government resignation).

This specification addresses these realities by treating agent governance, structured human oversight, formal constraint enforcement, and due process protections as foundational architectural concerns rather than optional extensions.

### 1.2 Design Goals

WOS is designed to satisfy the following goals, listed in priority order:

1. **Human authority is supreme.** No agent configuration, autonomy level, or guardrail may override, circumvent, or diminish human decision-making authority. Agents assist; humans decide. Where conflict exists between an agent's output and a human's judgment, the human's judgment governs. Agent recommendations MUST NOT be the sole or determinative factor in adverse decisions affecting individual rights.

2. **Structured oversight, not checkbox review.** Human oversight mechanisms MUST be designed to produce genuine cognitive engagement. Presenting an AI recommendation and asking a human to click "Approve" is not meaningful oversight. The standard specifies structural requirements — independent assessment, cognitive forcing, consider-the-opposite prompts — informed by empirical research on human-AI decision making.

3. **Accountability requires specificity.** Every action must be traceable to a specific actor (human, system, or agent), specific authority, specific inputs, specific outputs, and specific rule or policy version. Vague attribution is insufficient. The provenance model distinguishes immutable facts from model-generated narrative.

4. **Constraints are external to the agent.** Guardrails are enforced by the WOS Processor, not by the agent. The agent is outside the trust boundary of the governance envelope. This separation ensures governance survives agent changes, prompt injection, and behavioral drift.

5. **Graceful degradation is mandatory.** Every workflow MUST function correctly without any agent participation. Agent unavailability is a regular operating condition, not an edge case.

6. **Correctness is verifiable.** Workflow definitions MUST be formally verifiable for soundness properties. Agent behavior MUST be constrained within formally specified bounds. The standard defines what constitutes a sound workflow and how soundness is verified.

7. **Separation of concerns.** Process topology, decision logic, task management, agent governance, case state, integration, provenance, and execution guarantees are distinct concerns requiring distinct formalisms.

8. **Interoperability.** Workflow definitions MUST be portable across conformant implementations. The standard defines serialization formats, expression languages, and interface contracts sufficient for portability.

9. **AI-native authoring.** The serialization format, object model, and expression language MUST be designed for reliable generation, validation, transformation, and explanation by AI systems.

10. **Incremental adoption.** Implementations MAY conform to subsets of the specification via defined conformance profiles. Simple workflows MUST NOT require engagement with the full specification.

### 1.3 Scope

This specification defines the core object model, layered architecture, serialization format, expression language, and conformance requirements for WOS.

**Within scope:** the eight-layer architecture and inter-layer interfaces; the metamodel of objects, relationships, and constraints; lifecycle semantics based on hierarchical state machines; decision services with defeasible rules and temporal parameters; human task lifecycle with structured oversight protocols; agent governance including autonomy levels, guardrails, confidence, fallback, and monitoring; case file structure and evidence model; event envelope format and correlation; four-layer provenance model with tamper evidence; abstract durable execution guarantees; the expression language profile; due process requirements for adverse decisions; serialization in YAML/JSON with JSON Schema validation; conformance profiles and testing.

**Out of scope:** user interface rendering and form specification; specific persistence mechanisms; specific transport protocols beyond interface contracts; process mining algorithms (the standard defines compatible event emission); ML model training or inference specification; document management systems; notification delivery mechanisms; general-purpose computation.

### 1.4 Relationship to Tier Specifications

This document is the **Core Specification**. It defines the complete architecture, object model, and normative semantics. Subsequent **Tier Specifications** elaborate individual layers in detail. Each Tier Specification is normative for its layer but MUST NOT contradict the Core Specification.

Planned Tier Specifications:

- **WOS-Lifecycle** — Statechart semantics, transition resolution, verification algorithms.
- **WOS-Decision** — Decision table semantics, hit policies, defeasible rules, temporal parameters.
- **WOS-Task** — Human task lifecycle, structured oversight, SLA enforcement, separation of duties.
- **WOS-Agent** — Agent configuration, autonomy governance, guardrail system, multi-step sessions, drift monitoring.
- **WOS-CaseState** — Case file schema, evidence management, selective visibility.
- **WOS-Integration** — Event consumption/production, correlation, MCP/A2A alignment.
- **WOS-Provenance** — Four-layer audit records, tamper evidence, process mining interoperability.
- **WOS-Execution** — Durable execution guarantees, retry policies, compensation semantics.
- **WOS-Conformance** — Test suite, canonical fixtures, certification procedures.

### 1.5 Notational Conventions

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in BCP 14 [RFC 2119] [RFC 8174] when, and only when, they appear in all capitals, as shown here.

---

## 2. Conformance

### 2.1 Conformance Classes

**WOS Document.** A serialized workflow definition conforming to the structural and semantic requirements of this specification. A conformant WOS Document MUST validate against the WOS Core JSON Schema and MUST satisfy the static semantic constraints defined herein.

**WOS Processor.** A software system that consumes WOS Documents and produces behavior consistent with the semantics defined herein. A WOS Processor MAY conform to one or more Conformance Profiles (§21).

### 2.2 Conformance Requirements for WOS Documents

A conformant WOS Document:

1. MUST be serialized in YAML 1.2 [YAML] or JSON [RFC8259].
2. MUST validate against the WOS Core JSON Schema without errors.
3. MUST satisfy all static semantic constraints including unique identifiers, valid state references, type-correct expressions, and resolvable cross-references.
4. MUST include all REQUIRED properties as defined by the object model.
5. MUST define a fallback to human performance for every agent invocation point.
6. SHOULD pass soundness verification as defined in §6.8.

### 2.3 Conformance Requirements for WOS Processors

A conformant WOS Processor:

1. MUST accept any conformant WOS Document without error.
2. MUST reject documents that fail JSON Schema validation, producing diagnostics for each violation.
3. MUST execute lifecycle semantics (§6) consistent with this specification.
4. MUST produce provenance records (§12) for every state transition, task operation, decision evaluation, and agent invocation.
5. MUST enforce agent governance constraints (§9) including guardrails, autonomy limits, and fallback chains.
6. MUST support at least one Conformance Profile (§21).
7. SHOULD provide static soundness verification for workflow definitions.

---

## 3. Terminology

This section is normative.

**Activity.** A unit of work within a workflow, either automated (performed by a system or agent) or human (performed by a person via a Task).

**Actor.** An entity that performs actions. Actors are classified by type: `human`, `system`, or `agent`. See §14.

**Agent.** An AI system that participates in a workflow by performing tasks, evaluating decisions, or producing recommendations. Agents operate under declared autonomy levels and are subject to guardrail constraints, confidence reporting, and human oversight requirements. An agent is a type of Actor outside the trust boundary of the governance envelope.

**Agent Session.** A bounded interaction between a workflow instance and an agent, with a start event, zero or more checkpoints, and a terminal event.

**Autonomy Level.** A declared classification governing how much independent authority an actor has over a workflow action: `autonomous`, `supervisory`, `assistive`, or `manual`.

**Case.** An instance of a workflow applied to a specific subject, with a lifecycle, accumulated data and evidence, and audit records.

**Case File.** The structured data container associated with a Case, holding all typed data items, evidence references, and computed values.

**Compensation.** A semantically meaningful reversal of a completed activity.

**Confidence.** A structured assessment of certainty associated with an agent's output, comprising a scalar value, derivation method, optional per-field scores, and calibration status.

**Conformance Profile.** A named subset of this specification that an implementation may claim to support.

**Consequential Decision.** A determination that has legal, material, binding, or similarly significant effects on an individual's rights, benefits, services, or obligations. Consequential decisions are subject to additional due process requirements (§16).

**Decision Record.** An immutable audit entry recording the evaluation of a Decision Service. For agent-evaluated decisions, includes model version, confidence, and review outcome.

**Decision Service.** An encapsulated unit of decision logic with defined inputs and outputs, independently versionable and invocable.

**Defeasible Rule.** A rule that may be overridden by a more specific rule with higher priority, modeling the "general rule with exceptions" pattern.

**Deferred Choice.** A pattern where the next step is determined by an external event or human action at runtime, not predetermined by the designer.

**Durable Timer.** A timer that persists across restarts, may span arbitrary durations, and consumes no runtime resources while waiting.

**Evidence.** A document, dataset, image, or other artifact attached to a Case File Item with content integrity verification.

**Governance Envelope.** The set of autonomy constraints, guardrails, confidence requirements, and fallback policies surrounding every agent invocation, enforced by the WOS Processor, not by the agent.

**Guard.** A boolean expression controlling whether a transition may fire, expressed in the WOS Expression Language (§15).

**Guardrail.** A declarative constraint on an agent's behavior, enforced by the WOS Processor after the agent produces output and before the output is committed.

**History State.** A mechanism recording the last active substate within a compound state, enabling resumption of prior configuration after suspension.

**Milestone.** A named condition on case data that, when satisfied, indicates meaningful progress.

**Override.** A human action superseding an automated decision, requiring structured rationale and recorded in the audit trail.

**Parallel Region.** An independently executing concurrent track within a compound state.

**Provenance Record.** An immutable audit entry recording what happened, when, by whom, under what authority, and why.

**Sentry.** A combination of event trigger and guard condition controlling stage activation or milestone achievement.

**Soundness.** A formal property guaranteeing deadlock-freedom, livelock-freedom, proper termination, and reachability.

**Stage.** A compound lifecycle phase containing substates, activities, milestones, and sentries.

**State.** A named condition in the lifecycle of a Case, Task, or Stage.

**Structured Oversight.** Human review mechanisms designed to produce genuine cognitive engagement, including independent assessment, cognitive forcing functions, and consider-the-opposite prompts. Distinct from checkbox confirmation.

**Task.** A unit of human work with a defined lifecycle, assignment model, data contract, and oversight protocol.

**Transition.** A directed edge between states, triggered by an event and optionally gated by a guard condition.

**Workflow Definition.** A complete WOS Document specifying a workflow's lifecycle, decisions, tasks, agent governance, case structure, integrations, and audit requirements.

**Workflow Instance.** A running case governed by a Workflow Definition.

---

## 4. Architecture Overview

This section is normative.

### 4.1 Eight-Layer Architecture

WOS defines eight conceptual layers. Each layer addresses a distinct concern, has well-defined interfaces to adjacent layers, and is independently authorable and versionable.

```
┌──────────────────────────────────────────────────────────┐
│  Layer 8: Durable Execution Contract                     │
│  Abstract guarantees for runtime resilience               │
├──────────────────────────────────────────────────────────┤
│  Layer 7: Provenance and Audit                           │
│  Four-layer immutable records, tamper evidence            │
├──────────────────────────────────────────────────────────┤
│  Layer 6: Integration and Eventing                       │
│  CloudEvents, correlation, MCP/A2A alignment             │
├──────────────────────────────────────────────────────────┤
│  Layer 5: Case State and Evidence                        │
│  Typed case data, evidence, selective visibility          │
├──────────────────────────────────────────────────────────┤
│  Layer 4: Agent Governance                               │
│  Autonomy, guardrails, confidence, fallback, monitoring  │
├──────────────────────────────────────────────────────────┤
│  Layer 3: Human Task Management                          │
│  Task lifecycle, structured oversight, SLAs, separation  │
├──────────────────────────────────────────────────────────┤
│  Layer 2: Decision and Policy                            │
│  Decision tables, defeasible rules, temporal parameters  │
├──────────────────────────────────────────────────────────┤
│  Layer 1: Lifecycle and Topology                         │
│  Statechart, states, transitions, guards, milestones     │
└──────────────────────────────────────────────────────────┘
```

### 4.2 Separation Principles

The following separation principles are normative:

**Process topology MUST be separated from decision logic.** The lifecycle (Layer 1) defines structure. Decision Services (Layer 2) evaluate conditions. A guard MAY invoke a Decision Service, but the Decision Service MUST NOT contain process topology.

**Decision logic MUST be separated from task management.** Decisions determine what should happen. Tasks determine who does it and how the work is managed. These are independently versionable.

**Agent governance MUST be separated from agent implementation.** Guardrails, autonomy constraints, and confidence requirements are properties of the workflow definition, enforced by the WOS Processor. They are not properties of the agent and MUST NOT be delegable to the agent.

**Case data MUST be separated from process state.** The Case File (Layer 5) holds business data. The lifecycle state (Layer 1) tracks process progress. These are modeled separately.

**Audit MUST be separated from execution.** Provenance records (Layer 7) are produced as a consequence of execution but do not participate in control flow. The audit layer observes.

**Execution guarantees MUST be separated from execution mechanisms.** Layer 8 defines what guarantees hold, not how they are achieved.

### 4.3 Cross-Cutting Concerns

**Actor Model (§14).** Every action is attributed to a typed actor (human, system, agent) with distinct provenance requirements. The actor model spans all layers.

**Due Process (§16).** Consequential decisions are subject to notice, explanation, appeal, and continuation-of-services requirements that span lifecycle, task, decision, and provenance layers.

**Expressions (§15).** Guard conditions, data transformations, and computed values throughout all layers use the WOS Expression Language.

**Identity.** Every object has a unique identifier within its scope. Cross-document references use URIs.

**Versioning (§17).** The Workflow Definition, each Decision Service, each Task Definition, and each Agent Configuration carry independent version identifiers.

---

## 5. Document Structure

This section is normative.

### 5.1 Top-Level Structure

```yaml
wos: "2.0.0"                            # REQUIRED — spec version
id: "urn:wos:example.gov:grant-review"   # REQUIRED — definition URI
name: "Grant Application Review"         # REQUIRED — human-readable name
version: "1.0.0"                         # REQUIRED — definition version (SemVer)
status: "active"                         # REQUIRED — draft|active|deprecated|retired

metadata:                               # OPTIONAL
  description: "..."
  authors: [...]
  created: "2026-04-08T00:00:00Z"
  modified: "2026-04-08T00:00:00Z"
  tags: [...]
  jurisdiction: "US-Federal"
  authority: "24 CFR Part 570"
  effectiveDate: "2026-04-15"
  sunsetDate: null
  impactLevel: "rights-impacting"        # NEW — consequence classification

lifecycle: { ... }                       # REQUIRED — Layer 1
decisions: { ... }                       # OPTIONAL — Layer 2
parameters: { ... }                      # OPTIONAL — Layer 2 temporal parameters
tasks: { ... }                           # OPTIONAL — Layer 3
agents: { ... }                          # OPTIONAL — Layer 4
caseFile: { ... }                        # OPTIONAL — Layer 5
integrations: { ... }                    # OPTIONAL — Layer 6
provenance: { ... }                      # OPTIONAL — Layer 7 configuration
execution: { ... }                       # OPTIONAL — Layer 8 configuration
dueProcess: { ... }                      # OPTIONAL — §16 due process configuration
extensions: { ... }                      # OPTIONAL — §19
```

### 5.2 Property: `wos`

REQUIRED. A semantic version string identifying the specification version. A WOS Processor MUST reject a document whose major version it does not support.

### 5.3 Property: `id`

REQUIRED. A URI [RFC 3986] uniquely identifying this Workflow Definition. The `urn:wos:` scheme is RECOMMENDED.

### 5.4 Property: `version`

REQUIRED. A semantic version string [SemVer] identifying this definition version. Recorded immutably in instance metadata at creation time.

### 5.5 Property: `status`

REQUIRED. One of `draft` (not for production), `active` (approved for production), `deprecated` (superseded, existing instances continue), or `retired` (no new instances).

### 5.6 Property: `metadata`

OPTIONAL. Descriptive information including `description`, `authors`, `created`, `modified`, `tags`, `jurisdiction`, `authority`, `effectiveDate`, `sunsetDate`, and `impactLevel`.

The `impactLevel` property classifies the consequence level of decisions made within this workflow:

| Value | Definition | Requirements |
|-------|-----------|-------------|
| `rights-impacting` | Decisions affect individual legal rights, benefits, services, or obligations. | Full due process requirements (§16) apply. Agent autonomy capped at `assistive` unless explicitly elevated. |
| `safety-impacting` | Decisions affect individual or public safety. | Full due process requirements apply. Agent autonomy capped at `assistive`. |
| `operational` | Decisions affect organizational operations without direct individual impact. | Due process requirements are RECOMMENDED. Agent autonomy up to `autonomous` with guardrails. |
| `informational` | Outputs are informational and do not drive binding decisions. | Due process requirements are OPTIONAL. No autonomy restrictions. |

When `impactLevel` is not specified, the effective default is `operational`.

---

## 6. Layer 1: Lifecycle and Topology

This section is normative.

### 6.1 Overview

The Lifecycle layer defines the statechart governing a workflow instance's progression. The semantics are based on Harel statecharts [Harel1987] as formalized in W3C SCXML [SCXML], adapted for case-oriented workflows.

### 6.2 Property: `lifecycle`

REQUIRED. Defines the top-level statechart.

```yaml
lifecycle:
  initialState: "intake"
  defaultAutonomy: "assistive"

  states:
    intake:
      type: "atomic"
      onEntry:
        - action: "createTask"
          taskRef: "initialReview"
      transitions:
        - event: "task.completed"
          target: "evaluation"
          guard: "caseFile.application.isComplete = true"
        - event: "task.completed"
          target: "returnedToApplicant"
          guard: "caseFile.application.isComplete = false"

    evaluation:
      type: "parallel"
      regions:
        technicalReview:
          initialState: "pending"
          states:
            pending: { type: "atomic" }
            complete: { type: "final" }
        financialReview:
          initialState: "pending"
          states:
            pending: { type: "atomic" }
            complete: { type: "final" }
      transitions:
        - event: "regions.allFinal"
          target: "adjudication"

    adjudication:
      type: "compound"
      initialState: "recommendation"
      historyState: "shallow"
      states:
        recommendation: { type: "atomic" }
        supervisorDecision: { type: "atomic" }
        decided: { type: "final" }

    completed:
      type: "final"

  milestones:
    applicationReceived:
      condition: "caseFile.application != null"
    eligibilityConfirmed:
      condition: "caseFile.eligibility.decision = 'eligible'"
```

### 6.3 States

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `type` | enum | REQUIRED | `atomic`, `compound`, `parallel`, or `final`. |
| `onEntry` | array of Action | OPTIONAL | Actions on state entry. |
| `onExit` | array of Action | OPTIONAL | Actions on state exit. |
| `transitions` | array of Transition | OPTIONAL | Outgoing transitions. |
| `historyState` | enum | OPTIONAL | `shallow` or `deep`. Compound states only. |
| `initialState` | string | CONDITIONAL | Required for `compound` states. |
| `regions` | map of Region | CONDITIONAL | Required for `parallel` states. |
| `metadata` | object | OPTIONAL | Descriptive metadata. |

**Compound states** contain substates with a designated `initialState`. When entered, execution proceeds to the initial substate unless a history state directs otherwise. `historyState: "shallow"` records the last active direct substate; `"deep"` records the full nested configuration.

**Parallel states** contain named regions executing concurrently. A parallel state is not exited until all regions reach a final state (raising the `regions.allFinal` event), unless an explicit transition overrides this.

**Final states** indicate completion of the enclosing scope. A top-level final state indicates workflow completion. Final states MUST NOT have outgoing transitions.

### 6.4 Transitions

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `event` | string | REQUIRED | Triggering event identifier. |
| `target` | string | REQUIRED | Target state identifier. |
| `guard` | string (expression) | OPTIONAL | WOS Expression that must evaluate to `true`. |
| `actions` | array of Action | OPTIONAL | Actions on transition. |
| `priority` | integer | OPTIONAL | Resolution priority; lower is higher. Default: 0. |
| `description` | string | OPTIONAL | Human-readable explanation. |

**Transition resolution:** When an event occurs, collect matching transitions, evaluate guards, discard those evaluating to `false`. If one remains, fire it. If multiple remain with the same lowest priority, the document is ill-formed. If none remain, the event is discarded (not an error).

**Execution sequence:** (1) onExit of source, innermost first. (2) Transition actions. (3) onEntry of target, outermost first. (4) Provenance record emitted.

### 6.5 Actions

| Action Type | Description | Key Properties |
|-------------|-------------|----------------|
| `createTask` | Creates a human task instance. | `taskRef` |
| `invokeDecision` | Invokes a decision service. | `decisionRef`, `outputBinding`, `autonomy` |
| `invokeAgent` | Invokes an agent within its governance envelope. | `agentRef`, `capability`, `autonomy` |
| `setData` | Sets a case file value. | `path`, `value` |
| `emitEvent` | Emits an event to the integration layer. | `eventType`, `data` |
| `startTimer` | Starts a durable timer. | `timerId`, `duration` or `deadline`, `event` |
| `cancelTimer` | Cancels a running timer. | `timerId` |
| `compensate` | Triggers compensation for a scope. | `scope` |
| `log` | Writes an informational audit entry. | `message`, `level` |
| `notify` | Sends a notification. | `recipientRoles`, `template` |

The `autonomy` property on `invokeDecision` and `invokeAgent` actions specifies the autonomy level for that invocation. If omitted, the `defaultAutonomy` from the lifecycle or agent configuration applies.

### 6.6 Events

**Internal events** raised by the workflow engine:

| Event | Raised When |
|-------|------------|
| `task.completed` | A task reaches Completed. |
| `task.failed` | A task reaches Failed. |
| `task.escalated` | A task is escalated. |
| `timer.expired` | A durable timer expires. |
| `regions.allFinal` | All parallel regions reach final states. |
| `error` | An unhandled error occurs. |
| `milestone.achieved` | A milestone condition becomes true. |
| `decision.complete` | A decision service evaluation completes. |
| `agent.complete` | An agent invocation completes. |
| `agent.failed` | An agent invocation fails (after fallback exhaustion). |
| `guardrail.violated` | An agent output violates a guardrail. |

**External events** originate outside the workflow and are matched via correlation (§11.4).

### 6.7 Milestones

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `condition` | expression | REQUIRED | Expression evaluating to `true` when achieved. |
| `description` | string | OPTIONAL | Human-readable description. |

Milestones are re-evaluated when referenced case data changes. Achievement raises the `milestone.achieved` event.

### 6.8 Soundness Verification

A WOS Document SHOULD be verifiable for:

**Deadlock-freedom.** From every reachable non-final state, a final state is reachable under some event sequence.

**Livelock-freedom.** No infinite cycle exists without progress toward a final state.

**Proper termination.** When the top-level final state is reached, no parallel regions have active states and no tasks remain non-terminal.

**No dead elements.** Every state, transition, and task definition is reachable from the initial state.

**Fallback completeness.** Every `invokeAgent` action has a reachable path to workflow completion that does not require any agent to succeed. This is verifiable by treating all agent invocations as failures and checking that the workflow remains sound.

A conformant verifier MUST support a decidable fragment, MAY report inconclusive results for data-dependent guards, and MUST report inconclusive verification as a warning, not a pass.

---

## 7. Layer 2: Decision and Policy

This section is normative.

### 7.1 Decision Services

Decision Services encapsulate logic for routing, eligibility, classification, and policy evaluation, independently versionable and invocable from any workflow point.

```yaml
decisions:
  eligibilityDetermination:
    version: "2.1.0"
    description: "Determines applicant eligibility per 24 CFR 570.200."

    inputs:
      - name: "applicantIncome"
        schema: { type: "number" }
      - name: "householdSize"
        schema: { type: "integer" }
      - name: "applicationDate"
        schema: { type: "string", format: "date" }

    outputs:
      - name: "eligible"
        schema: { type: "boolean" }
      - name: "reason"
        schema: { type: "string" }
      - name: "applicableThreshold"
        schema: { type: "number" }

    logic:
      type: "decisionTable"
      hitPolicy: "first"
      rules: [...]

    effectiveDate: "2026-01-01"
    sunsetDate: "2026-12-31"
```

### 7.2 Decision Logic Types

**Decision Tables** with hit policies: `unique`, `first`, `priority`, `collect`, `collectSum`, `collectMin`, `collectMax`, `collectCount`. Consistent with DMN [DMN].

**Expression Logic** — a single WOS Expression computing the output directly.

**Decision Requirement Graphs** — a DAG of sub-decisions where each node is a Decision Service, enabling composition from independently testable components.

**Defeasible Rules** — rules with structured exceptions for regulatory contexts:

```yaml
logic:
  type: "defeasibleRules"
  rules:
    - id: "generalEligibility"
      condition: "applicantIncome <= parameters.incomeThreshold(applicationDate)"
      conclusion: { eligible: true }

    - id: "veteranException"
      overrides: "generalEligibility"
      condition: "applicant.veteranStatus = true"
      conclusion:
        eligible: true
        applicableThreshold: "parameters.incomeThreshold(applicationDate) * 1.2"

    - id: "disqualification"
      overrides: ["generalEligibility", "veteranException"]
      condition: "applicant.priorFraudFinding = true"
      conclusion: { eligible: false, reason: "Disqualified: prior fraud finding" }
```

Override relationships MUST form a directed acyclic graph. When multiple rules match, the overriding rule takes precedence.

**External Decision Reference** — delegates to an external service via the Integration layer.

### 7.3 Temporal Parameters

Parameters whose values change on specific dates, modeling the common regulatory pattern of periodically updated thresholds, rates, and criteria:

```yaml
parameters:
  incomeThreshold:
    description: "Federal poverty level threshold"
    type: "number"
    values:
      - effectiveDate: "2025-01-01"
        value: 31200
      - effectiveDate: "2026-01-01"
        value: 32760
```

When a Decision Service references a temporal parameter, the value effective on the reference date is used. If no value is effective, the WOS Processor MUST raise an error.

### 7.4 Decision Invocation and Provenance

Every Decision Service invocation MUST produce a Decision Record (§12.4) capturing: service identifier and version, complete input data, matched rules and evaluation trace, outputs, timestamp, and workflow context.

---

## 8. Layer 3: Human Task Management

This section is normative.

### 8.1 Overview

The Human Task Management layer treats human judgment, discretionary action, and exception handling as first-class concerns. It defines how work is assigned to, managed by, and completed by human participants, with explicit requirements for structured oversight when AI assistance is involved.

### 8.2 Task Definitions

```yaml
tasks:
  eligibilityReview:
    version: "1.0.0"
    description: "Review eligibility determination."

    form:
      inputSchema: { ... }
      outputSchema:
        type: "object"
        properties:
          decision: { type: "string", enum: ["eligible", "ineligible", "needsInfo"] }
          rationale: { type: "string", minLength: 50 }
        required: ["decision", "rationale"]

    assignment:
      potentialOwners:
        roles: ["eligibilitySpecialist"]
        skills: ["programEligibility"]
      businessAdministrators:
        roles: ["programSupervisor"]

    priority:
      expression: "if caseFile.application.expedited then 1 else 5"

    sla:
      dueIn: "P5BD"
      warningAt: "P3BD"
      businessCalendar: "federalWorkdays"
      escalateOnBreach:
        to: { roles: ["programSupervisor"] }
        action: "reassign"

    separation:
      excludeFrom: ["finalApproval"]
      constraint: "sameInstance"

    oversight:
      agentAssistance:
        agentRef: "eligibilityScreener"
        capability: "eligibilityPreScreen"
        timing: "beforeClaim"
        protocol: "independentFirst"
```

### 8.3 Task Lifecycle

```
                               ┌──────────┐
                         ┌────>│ Returned │──┐
                         │     └──────────┘  │
                         │                   v
┌───────────┐    ┌──────────┐    ┌────────────┐    ┌───────────┐
│ Available │───>│ Claimed  │───>│ InProgress │───>│ Completed │
└───────────┘    └──────────┘    └────────────┘    └───────────┘
      │               │               │
      │               │               ├──> ┌──────────┐
      │               │               │    │  Failed  │
      │               │               │    └──────────┘
      v               v               v
┌───────────┐   ┌───────────┐   ┌───────────┐
│ Cancelled │   │ Suspended │   │ Escalated │
└───────────┘   └───────────┘   └───────────┘
```

Terminal states: `Completed`, `Failed`, `Cancelled`. A task in a terminal state MUST NOT transition.

### 8.4 Task Operations

| Operation | From | To | Actor | Description |
|-----------|------|----|-------|-------------|
| `create` | — | Available | System | Creates a task instance. |
| `claim` | Available | Claimed | Potential Owner | Individual takes responsibility. |
| `release` | Claimed | Available | Actual Owner | Returns task to queue. |
| `start` | Claimed | InProgress | Actual Owner | Work begins. |
| `complete` | InProgress | Completed | Actual Owner | Output submitted. |
| `fail` | InProgress | Failed | Actual Owner | Cannot complete; reason required. |
| `delegate` | Claimed, InProgress | Claimed | Actual Owner | Transfer to specific individual. |
| `forward` | Available, Claimed | Available | Owner, Admin | Forward to different group. |
| `returnForRework` | InProgress | Returned | Reviewer | Output rejected; revision required. |
| `escalate` | Any non-terminal | Escalated | System, Admin | SLA breach or exception. |
| `suspend` | Any non-terminal | Suspended | System, Admin | Task paused. |
| `resume` | Suspended | (prior state) | System, Admin | Resumes from prior state. |
| `cancel` | Any non-terminal | Cancelled | System, Admin | Task cancelled. |

Every operation MUST produce a provenance record.

### 8.5 Assignment Model

Five role categories: `potentialOwners` (may claim and perform), `excludedOwners` (barred, for conflict of interest), `businessAdministrators` (may reassign, escalate, cancel), `taskStakeholders` (may view), `notificationRecipients` (receive notifications). Each specified by `roles`, `skills`, `individuals`, or `expression`.

### 8.6 Service Level Agreements

| Property | Type | Description |
|----------|------|-------------|
| `dueIn` | duration | ISO 8601 or business duration (`BD`, `BH`). |
| `dueBy` | expression | Computed absolute deadline. |
| `warningAt` | duration | Warning threshold before deadline. |
| `escalateOnBreach` | EscalationDef | Automatic escalation on SLA breach. |
| `businessCalendar` | string | Named calendar for business-day computation. |

### 8.7 Separation of Duties

Constraints preventing the same individual from performing specified task combinations. Enforced at claim time. Violations MUST be rejected and logged.

```yaml
separation:
  excludeFrom: ["finalApproval"]
  constraint: "sameInstance"  # or "global"
```

### 8.8 Structured Oversight Protocols

This section is normative. It addresses the empirical finding that naive human-AI review degrades decision quality.

When a task involves AI agent assistance, the `oversight` property specifies how the human reviewer engages with the agent's output. The `protocol` field is REQUIRED when `agentAssistance` is present.

| Protocol | Description | Empirical Basis |
|----------|-------------|----------------|
| `independentFirst` | The reviewer MUST form and record an independent assessment before the agent's recommendation is revealed. The interface MUST enforce this ordering. | Buçinca et al. (CSCW 2021): cognitive forcing functions reduce overreliance. |
| `considerOpposite` | After viewing the agent's recommendation, the reviewer is prompted to articulate reasons the recommendation might be wrong before confirming. | Anchoring bias research: consider-the-opposite debiases. |
| `calibratedConfidence` | The agent's calibrated confidence score is displayed alongside the recommendation. Per-field confidence is shown when available. Low-confidence fields are visually highlighted. | Li et al. (2024): miscalibrated confidence impairs reliance. |
| `dualBlind` | Two independent reviewers assess the case without seeing each other's or the agent's assessment. Results are reconciled. | Standard practice for high-stakes adjudication. |
| `unassisted` | No agent assistance is provided. The task is performed entirely by the human. | Baseline for tasks requiring unmediated judgment. |

Multiple protocols MAY be combined. When `independentFirst` is specified, the WOS Processor MUST enforce that the reviewer's independent assessment is recorded before the agent's output is accessible.

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

When `showDiffFromIndependent` is `true`, the interface highlights differences between the reviewer's independent assessment and the agent's recommendation, focusing the reviewer's attention on points of disagreement rather than requiring full re-review.

### 8.9 Override Authority

Authorized individuals may override automated decisions with mandatory structured accountability:

```yaml
tasks:
  supervisorOverride:
    overrideTarget: "eligibilityDetermination"
    requiredAuthority:
      roles: ["programSupervisor"]
    requiredFields:
      - name: "rationale"
        schema: { type: "string", minLength: 50 }
      - name: "supportingEvidence"
        schema: { type: "array", items: { $ref: "#/caseFile/evidenceRef" } }
      - name: "overrideDecision"
        schema: { $ref: "#/decisions/eligibilityDetermination/outputs" }
```

Override Records (§12.5) capture both the original result and the override with rationale, authority, and evidence.

---

## 9. Layer 4: Agent Governance

This section is normative.

### 9.1 Overview

The Agent Governance layer defines how AI agents participate in workflows as constrained, monitored, accountable actors. It embodies the architectural principle that **the workflow specification, not the agent, is the authority on acceptable behavior**. The governance envelope — autonomy constraints, guardrails, confidence requirements, and fallback policies — is enforced by the WOS Processor. The agent itself is outside the trust boundary.

```
┌───────────────────────────────────────────────────────────┐
│  WOS Processor (Trusted)                                  │
│  ┌─────────────────────────────────────────────────────┐  │
│  │  Governance Envelope                                 │  │
│  │  1. Pre-invocation: autonomy check, input prep       │  │
│  │  2. Invocation ──────────┐                          │  │
│  │  3. Post-invocation:     │     ┌──────────────┐     │  │
│  │     output validation    │◄────│    Agent      │     │  │
│  │     guardrail enforcement│     │  (Untrusted)  │     │  │
│  │     confidence check     │     └──────────────┘     │  │
│  │  4. Routing (commit / review / reject)              │  │
│  └─────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────┘
```

### 9.2 Agent Configuration

```yaml
agents:
  eligibilityScreener:
    version: "2.0.0"
    description: "Pre-screens applications against eligibility criteria."

    model:
      provider: "anthropic"
      identifier: "claude-sonnet-4-20250514"
      versionPolicy: "pinned"

    capabilities:
      - id: "eligibilityPreScreen"
        decisionRef: "eligibilityDetermination"

    defaultAutonomy: "assistive"

    autonomyPolicy:
      maxAutonomy: "assistive"
      reason: "Eligibility determinations affect individual rights."
      escalation:
        toSupervisory:
          conditions:
            - "agent.calibration.accuracy >= 0.97"
            - "agent.guardrailViolations(P30D) = 0"
          approval: { roles: ["programDirector"], expires: "P90D" }
      demotion:
        triggers:
          - condition: "agent.calibration.accuracy < 0.85"
            immediate: true
          - condition: "agent.guardrailViolations(P7D) >= 3"
            immediate: true
          - condition: "agent.modelVersionChanged"
            pendingRecalibration: true

    guardrails:
      outputConstraints:
        - field: "eligible"
          constraint: "value = true or value = false"
          onViolation: "reject"
        - field: "reason"
          constraint: "string length(value) >= 20"
          onViolation: "reject"
      confidenceFloor:
        threshold: 0.7
        onViolation: "escalateToHuman"
      prohibited:
        - condition: "output.eligible = true and caseFile.application.areaMedianIncome > parameters.amiThreshold(caseFile.application.submittedDate) * 1.5"
          reason: "Eligible with income far above threshold requires review."
          onViolation: "escalateToHuman"
      consistency:
        - check: "output.assessedDate >= caseFile.application.submittedDate"
          onViolation: "reject"
      humanReviewSampling:
        rate: 0.10
        method: "random"
      volumeConstraints:
        maxAutonomousPerHour: 50
        onExceeded: "switchToAssistive"

    fallback:
      primary: { onFailure: "retry", maxRetries: 2, backoff: "exponential", initialInterval: "PT5S" }
      terminal: { onFailure: "escalateToHuman", taskRef: "manualEligibilityScreening" }

    inputPreparation:
      sanitize: true
      maxInputTokens: 50000
      redactFields: ["caseFile.application.socialSecurityNumber"]
      isolateUntrustedData: true

    monitoring:
      calibrationRequired: true
      calibrationFrequency: "P30D"
      minimumEvaluationSamples: 100
      driftDetection:
        enabled: true
        method: "psi"
        threshold: 0.2
        window: "P7D"
        dimensions:
          - { field: "eligible", type: "categorical" }
        onDetection: "alert"
        alertRoles: ["programDirector"]
```

### 9.3 Autonomy Levels

| Level | Name | Semantics |
|-------|------|-----------|
| `autonomous` | Full autonomy | Agent output committed without human review. REQUIRES guardrails. PROHIBITED for `rights-impacting` and `safety-impacting` workflows unless explicitly elevated with approval. |
| `supervisory` | Human-supervised | Agent output provisionally committed. Human supervisor reviews within a defined window. No intervention = output finalized. |
| `assistive` | Human-confirmed | Agent produces recommendation. Human reviews, may modify, and explicitly confirms before commitment. Output attributed to the human; agent recommendation recorded in provenance. |
| `manual` | Human-performed | Action performed entirely by a human. Agent MAY provide contextual assistance on demand, but output is solely the human's. |

**Autonomy level constraints are normative:**

1. `autonomous` actions MUST have guardrail definitions. Autonomous actions without guardrails are a structural error and MUST be rejected by the WOS Processor.
2. `assistive` actions MUST create a human task for confirmation. The task's oversight protocol MUST be specified.
3. `supervisory` actions MUST define a `reviewWindow` duration.
4. The effective autonomy MUST NOT exceed the `maxAutonomy` in the agent's autonomy policy.
5. For workflows with `impactLevel: "rights-impacting"` or `"safety-impacting"`, the default autonomy is `assistive` regardless of agent configuration, unless the workflow definition contains an explicit `autonomyElevation` with approval requirements.

### 9.4 Confidence Framework

Every agent output MUST include a ConfidenceReport:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `overall` | number (0.0–1.0) | REQUIRED | Estimated probability of correctness. |
| `method` | enum | REQUIRED | `modelNative`, `calibrated`, `heuristic`, `conformal`, or `declared`. |
| `calibrationStatus` | enum | REQUIRED | `calibrated` (validated), `uncalibrated` (not validated), `expired` (past recalibration date). |
| `explanation` | string | OPTIONAL | Human-readable confidence factors. |
| `fieldLevel` | map | OPTIONAL | Per-output-field confidence values. |

**Calibration requirements:**

1. Agents with `calibrationRequired: true` MUST have their confidence validated against empirical accuracy using review outcome data from the `assistive` and `supervisory` modes.
2. An agent whose calibration status is `expired` MUST NOT operate above `assistive` autonomy.
3. Confidence values with `calibrationStatus: "uncalibrated"` SHOULD be treated as less reliable in routing decisions. Guard expressions MAY reference calibration status.

**Confidence decay:** Agent outputs become less reliable as case data changes. When a decay trigger fires, effective confidence is reduced. If reduced confidence falls below the confidence floor guardrail, the output is invalidated and re-evaluation or human escalation is triggered.

**Cumulative confidence in multi-step sessions:** Errors compound. A WOS Processor MUST track cumulative confidence across session steps, computed conservatively (multiplicative default). If cumulative confidence falls below the session's confidence floor, the session pauses for human review.

### 9.5 Guardrail System

Guardrails are declarative constraints on agent outputs enforced by the WOS Processor after the agent produces output and before commitment.

**Guardrail types:**

| Type | Description |
|------|-------------|
| `outputConstraints` | Structural validity: output values within declared bounds. |
| `confidenceFloor` | Minimum confidence for the declared autonomy level. |
| `prohibited` | Logically inconsistent or policy-violating output combinations. |
| `consistency` | Cross-field and temporal consistency checks. |
| `volumeConstraints` | Rate limits on autonomous actions. |
| `humanReviewSampling` | Percentage sampling for quality assurance. |
| `scope` | Output stays within the agent's declared capability bounds. |

**Enforcement order:** (1) Output constraints. (2) Prohibited outputs. (3) Consistency. (4) Confidence floor. (5) Volume constraints. (6) Human review sampling.

**Violation actions:** `reject` (discard, raise error), `escalateToHuman` (route to human review), `switchToAssistive` (demote autonomy), `flag` (accept but annotate in provenance). When multiple guardrails are violated, the most restrictive action applies. Ordering: `reject` > `escalateToHuman` > `switchToAssistive` > `flag`.

**Composition:** Guardrails defined at workflow, agent, and action levels compose by union. All applicable guardrails are evaluated. Narrower scopes take precedence for enforcement action severity.

**Guardrail bypass:** An authorized human MAY bypass a guardrail marked `bypassable: true`, with a role at or above the guardrail's `bypassAuthority`, providing structured rationale. Bypass applies to a single invocation only and produces a `guardrailBypass` provenance record.

Every guardrail violation MUST produce a `guardrailViolation` provenance record.

### 9.6 Fallback Chains

The `fallback` property defines an ordered degradation sequence. Each level specifies what happens if the prior level fails.

**Normative requirements:**

1. A fallback chain MUST terminate in either `escalateToHuman` (with a `taskRef` to a human task definition) or `fail`.
2. A fallback chain MUST NOT cycle.
3. A WOS Processor MUST validate fallback chains at document load time.
4. Every fallback attempt MUST produce a provenance record.

**Fallback testing:** A conformant WOS Processor SHOULD support degradation testing — simulating agent unavailability and verifying that fallback chains execute correctly and the workflow reaches completion without any agent participation.

### 9.7 Input Preparation and Isolation

| Property | Type | Description |
|----------|------|-------------|
| `sanitize` | boolean | Apply input sanitization for prompt injection patterns. |
| `maxInputTokens` | integer | Maximum input size. Exceeded = fallback triggered. |
| `redactFields` | array of string | Case file paths replaced with `[REDACTED]`. |
| `includeFields` | array of string | Only these paths included (mutually exclusive with `redactFields`). |
| `isolateUntrustedData` | boolean | Process untrusted case data (citizen submissions, documents) in an isolated context per the CaMeL dual-LLM pattern. |

When `isolateUntrustedData` is `true`, the WOS Processor SHOULD separate trusted control flow (from the workflow definition and system instructions) from untrusted data processing (case files, documents). Untrusted data MUST NOT influence agent behavior beyond data extraction. This follows the CaMeL architecture where a planning component processes only trusted instructions and a quarantined component processes untrusted data.

### 9.8 Monitoring and Drift Detection

**Agent lifecycle states:** `active`, `degraded` (operating at reduced autonomy), `suspended` (all invocations route to fallback), `retired` (permanently unavailable, preserved for audit).

**Model version transitions:** When the effective model version changes: (1) provenance record emitted, (2) demotion applied if configured, (3) administrators notified, (4) in-flight sessions complete under their starting version.

**Drift detection methods:** `psi` (Population Stability Index), `ks` (Kolmogorov-Smirnov), `chi2` (chi-squared), `accuracy` (accuracy trend from review data). When drift is detected, a provenance record is produced, alerts sent, and demotion applied if configured.

**Shadow deployment:** For `rights-impacting` and `safety-impacting` workflows, model version changes SHOULD follow a shadow → canary → production deployment sequence. In shadow mode, the new version runs in parallel but its outputs are not used; they are compared to the production version for drift analysis.

### 9.9 Multi-Step Agent Sessions

Complex agent tasks are modeled as sessions with defined checkpoints and intervention points:

```yaml
agents:
  evidenceAnalyst:
    sessions:
      evidenceAnalysis:
        maxSteps: 5
        maxDuration: "PT10M"
        checkpointPolicy: "afterEachStep"

        steps:
          - id: "documentInventory"
            outputSchema: { ... }
            guardrails: { ... }
          - id: "contentExtraction"
            dependsOn: ["documentInventory"]
          - id: "crossReference"
            dependsOn: ["contentExtraction"]
            interventionPoint: true
            interventionPrompt: "Review intermediate findings before recommendation."
          - id: "findingsReport"
            dependsOn: ["crossReference"]

        termination:
          onCompletion: "commitSession"
          onFailure: "rollbackToCheckpoint"
          onTimeout: "escalateToHuman"
```

Checkpoints record intermediate state for recovery, inspection, and intervention. At intervention points, execution pauses and a human review task is created. The reviewer may approve, modify intermediate output, redirect remaining steps, terminate the session, or restart from a checkpoint.

### 9.10 Agent Tool Use Governance

Agents invoking tools (APIs, databases, calculations) during reasoning are subject to additional constraints:

```yaml
agents:
  analyst:
    tools:
      permitted:
        - { id: "caseFileRead", type: "dataAccess", scope: "caseFile", access: "read" }
        - { id: "calculator", type: "computation", sideEffects: false }
      prohibited:
        - { type: "dataAccess", scope: "caseFile", access: "write", reason: "Agents may not write to case file directly." }
        - { type: "integration", integrationRef: "paymentService", reason: "No financial transactions." }
```

**Normative requirements:** Agents MUST NOT invoke non-permitted tools. Agents MUST NOT write to the case file directly. Every tool invocation MUST be recorded in provenance. Tools with side effects MUST NOT be invoked at `autonomous` level without explicit `sideEffectPolicy: "permitted"`.

---

## 10. Layer 5: Case State and Evidence

This section is normative.

### 10.1 Overview

The Case File is the central artifact: the process exists to serve the case. The Case File holds all typed data items, evidence references, and computed values, with event-sourced mutation history and role-based selective visibility.

### 10.2 Case File Definition

```yaml
caseFile:
  schema:
    type: "object"
    properties:
      application: { $ref: "#/caseFile/items/application" }
      eligibility: { $ref: "#/caseFile/items/eligibility" }
      reviews: { $ref: "#/caseFile/items/reviews" }
      adjudication: { $ref: "#/caseFile/items/adjudication" }
      outcome: { $ref: "#/caseFile/items/outcome" }

  items:
    application:
      schema: { ... }
      visibility:
        default: "restricted"
        overrides:
          - roles: ["reviewer", "supervisor"]
            access: "readWrite"
          - roles: ["applicant"]
            fields: ["applicantName", "submittedDate"]
            access: "read"
      multiplicity: "one"

  evidenceSchema:
    type: "object"
    properties:
      id: { type: "string", format: "uri" }
      contentType: { type: "string" }
      contentHash:
        type: "object"
        properties:
          algorithm: { type: "string", enum: ["sha-256", "sha-384", "sha-512"] }
          value: { type: "string" }
        required: ["algorithm", "value"]
      receivedDate: { type: "string", format: "date-time" }
      description: { type: "string" }
      claimCheckUri: { type: "string", format: "uri" }
    required: ["id", "contentType", "contentHash"]
```

### 10.3 Data Mutation Semantics

Every Case File mutation MUST be recorded as an immutable event: path, prior value, new value, actor, triggering context, and timestamp. A WOS Processor MUST reconstruct Case File state at any prior point by replaying mutation events.

### 10.4 Evidence Management

Evidence MUST NOT be stored inline. The Case File holds evidence references with content hash and claim check URI. Content hashing MUST use SHA-256 or stronger. Hash integrity verification failures MUST be logged.

### 10.5 Selective Visibility

Visibility rules restrict which roles can view or modify which fields. Access levels: `read`, `readWrite`, `none`. Enforced when presenting data to task participants and via the query interface.

---

## 11. Layer 6: Integration and Eventing

This section is normative.

### 11.1 Integration Definitions

```yaml
integrations:
  backgroundCheck:
    type: "request-response"
    interface: { $ref: "https://api.example.gov/checks/openapi.yaml" }
    timeout: "PT30M"
    retry: { maxAttempts: 3, backoff: "exponential", initialInterval: "PT10S" }

  documentReceived:
    type: "event-consume"
    eventType: "org.example.documents.received"
    correlation:
      attribute: "subject"
      caseFileMapping: "caseFile.application.applicationId"

  notification:
    type: "event-emit"
    eventType: "org.example.grants.notification"
```

Types: `request-response` (sync, OpenAPI), `event-emit` (outbound), `event-consume` (inbound with correlation), `callback` (long-running: request then callback event).

### 11.2 Event Envelope

All events MUST conform to CloudEvents 1.0 [CloudEvents] with WOS extension attributes: `wosinstanceid`, `wosdefid`, `wosdefversion`, `wosstate`, `wostaskid`, `woscorrelationkey`, `woscausationeventid`.

### 11.3 Idempotency

Event consumption MUST be idempotent. Duplicate events (same CloudEvents `id`) MUST NOT produce duplicate effects.

### 11.4 Correlation

Correlation matches inbound events to running instances via attribute-to-case-file-path mapping. Multiple attributes = logical AND. No match = logged and MAY be queued for retry.

### 11.5 Interoperability Protocol Alignment

WOS is designed for compatibility with emerging agent interoperability protocols:

**Model Context Protocol (MCP)** — for agent-to-tool integration. WOS integration definitions that reference agent tool use SHOULD align with MCP's three-primitive model (Tools, Resources, Prompts). The WOS Processor serves as the MCP host, managing agent access to workflow tools and resources within the governance envelope.

**Agent-to-Agent Protocol (A2A)** — for inter-agent and cross-workflow communication. WOS workflow instances that coordinate with external agent systems SHOULD expose capabilities as A2A Agent Cards and use A2A's task lifecycle model (including the `input-required` state for human-in-the-loop interactions).

WOS does not mandate either protocol but defines extension points (§19) for protocol-specific bindings.

---

## 12. Layer 7: Provenance and Audit

This section is normative.

### 12.1 Overview

The Provenance layer produces an immutable, tamper-evident record of everything that happens. This layer is observational — it records effects of all other layers but does not participate in control flow.

The provenance model uses a **four-layer audit architecture** informed by empirical findings that model-generated explanations are systematically unfaithful to actual reasoning:

| Layer | Name | Content | Authority |
|-------|------|---------|-----------|
| 1 | Immutable Facts | Timestamp, actor ID/type, model version, exact inputs, exact outputs, policy version, confidence score, reviewer ID. | Authoritative. Machine-generated from runtime state. |
| 2 | Structured Reasoning | Policy rules applied, evidence sources consulted, eligibility criteria checked, decision table trace. | Authoritative for deterministic logic. Descriptive for agent reasoning. |
| 3 | Generated Narrative | Model's natural language explanation of its reasoning. | **Informational only.** Explicitly labeled as model-generated. NOT authoritative for audit or legal purposes. |
| 4 | Counterfactual | What would need to change for a different outcome. Positive evidence (controllable features) and negative evidence (irrelevant attributes). | Informational. Required for adverse decisions in `rights-impacting` workflows. |

**Design rationale:** Research demonstrates that chain-of-thought explanations are post-hoc rationalizations optimized for plausibility, not faithful records of computation (Turpin et al., NeurIPS 2023; Lanham et al., Anthropic, 2023). Larger, more capable models produce less faithful reasoning on most tasks. WOS therefore mandates that Layers 1 and 2 — the deterministic, verifiable record — serve as the authoritative audit trail. Layer 3 is preserved for informational value but MUST NOT be treated as dispositive evidence of why a decision was made.

### 12.2 Base Provenance Record

Every provenance record MUST include:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | URI | REQUIRED | Globally unique. |
| `timestamp` | datetime | REQUIRED | RFC 3339 with timezone. |
| `processId` | URI | REQUIRED | Workflow instance. |
| `recordType` | enum | REQUIRED | See §12.3–12.9. |
| `actor` | ActorRef | REQUIRED | Who/what caused the action. |
| `authority` | string | OPTIONAL | Role, rule, or policy under which the actor operated. |
| `traceContext` | TraceContext | OPTIONAL | W3C Trace Context for distributed tracing. |
| `auditLayer` | integer | REQUIRED | Which layer (1–4) this record primarily represents. |
| `data` | object | REQUIRED | Record-type-specific payload. |

### 12.3 Record Types

| Type | Description | Layers |
|------|-------------|--------|
| `transition` | Lifecycle transition fired. | 1, 2 |
| `decision` | Decision Service evaluated. | 1, 2, 3, 4 |
| `agentInvocation` | Agent invoked. | 1, 2, 3 |
| `agentCheckpoint` | Multi-step session checkpoint. | 1 |
| `agentToolUse` | Agent tool invocation. | 1 |
| `taskOperation` | Task state change. | 1 |
| `dataMutation` | Case File change. | 1, 2 |
| `override` | Human override of automated decision. | 1, 2, 3 |
| `guardrailViolation` | Agent output violated guardrail. | 1 |
| `guardrailBypass` | Human bypassed guardrail. | 1, 2 |
| `autonomyChange` | Autonomy escalation or demotion. | 1 |
| `modelVersionChange` | Agent model version changed. | 1 |
| `driftAlert` | Behavioral drift detected. | 1 |
| `dueProcessNotice` | Adverse decision notice issued. | 1, 2, 4 |
| `appealFiled` | Appeal of adverse decision. | 1 |

### 12.4 Decision Records

For agent-evaluated decisions, the Decision Record MUST include:

```yaml
recordType: "decision"
auditLayer: 1
data:
  # Layer 1: Immutable Facts
  decisionRef: "eligibilityDetermination"
  decisionVersion: "2.1.0"
  actor: { type: "agent", id: "urn:agent:eligibility-screener" }
  agentModel: "claude-sonnet-4-20250514"
  agentModelVersion: "20250514"
  inputs: { applicantIncome: 28500, householdSize: 3, applicationDate: "2026-03-15" }
  outputs: { eligible: true, reason: "Meets income threshold" }
  confidence:
    overall: 0.91
    method: "calibrated"
    calibrationStatus: "calibrated"
    fieldLevel: { eligible: 0.97, reason: 0.82 }
  guardrailsEvaluated: ["outputConstraints", "confidenceFloor", "prohibited"]
  guardrailsPassed: true
  latencyMs: 4200
  autonomyLevel: "assistive"

  # Layer 2: Structured Reasoning
  matchedRules: ["generalEligibility"]
  parametersUsed:
    incomeThreshold: { effectiveDate: "2026-01-01", value: 32760 }

  # Layer 3: Generated Narrative (informational, not authoritative)
  generatedExplanation: "The applicant's household income of $28,500 is below the 2026 threshold of $32,760 for a household of 3."
  explanationFaithfulnessDisclaimer: "This explanation is model-generated and may not faithfully represent the model's actual reasoning process."

  # Layer 4: Counterfactual (required for rights-impacting)
  counterfactual:
    positiveEvidence:
      - "Eligibility would change if applicantIncome exceeded $32,760."
    negativeEvidence:
      - "Applicant name, address, and demographic information did not affect this determination."

  # Review outcome (populated after human review)
  reviewOutcome: null
```

When a human reviews the agent's output under `assistive` or `supervisory` autonomy, the `reviewOutcome` is populated:

```yaml
reviewOutcome:
  reviewer: { id: "urn:user:jchen", role: "eligibilitySpecialist" }
  action: "accepted"  # or "modified" or "rejected"
  modifications: null  # populated if "modified"
  rationale: null       # REQUIRED if "modified" or "rejected"
  independentAssessment: { eligible: true }  # from independentFirst protocol
  reviewDuration: "PT4M30S"
```

### 12.5 Override Records

```yaml
recordType: "override"
auditLayer: 1
data:
  overrideTarget: "eligibilityDetermination"
  originalResult: { eligible: false, reason: "Income exceeds threshold" }
  overrideResult: { eligible: true, reason: "Hardship exception granted" }
  rationale: "Applicant experienced documented medical emergency..."
  supportingEvidence: ["urn:evidence:medical-records-2026-03"]
  overridingActor: { id: "urn:user:jsmith", role: "programSupervisor" }
  authorityLevel: 3
```

### 12.6 Tamper Evidence

A Full conformant WOS Processor MUST provide tamper evidence for the provenance stream. The RECOMMENDED mechanism is Merkle tree hash-chaining: each record hashed (SHA-256), organized into a Merkle tree with signed tree heads at configurable intervals (RECOMMENDED: every 100 records or 60 seconds), with inclusion proofs and consistency proofs.

### 12.7 Process Mining Interoperability

A WOS Processor SHOULD emit event data compatible with IEEE XES [XES], enabling automated conformance checking: comparing actual execution against defined process models.

---

## 13. Layer 8: Durable Execution Contract

This section is normative.

### 13.1 Durability Guarantees

A conformant WOS Processor at the Execution Conformance Profile MUST satisfy:

**G1: Crash Recovery.** Non-terminal workflow instances resume from their last durable state after restart.

**G2: Persistent State.** Lifecycle state, case file, task states, and timer registrations are durably persisted.

**G3: At-Least-Once Execution.** Actions execute at least once. Actions SHOULD be idempotent.

**G4: Durable Timers.** Timers survive restarts, fire within tolerance of scheduled time, consume no resources while waiting.

**G5: External Signal Delivery.** Signals to inactive instances are durably enqueued and delivered when the instance activates.

### 13.2 Retry Policy

```yaml
execution:
  defaultRetry:
    maxAttempts: 3
    backoff: "exponential"     # fixed, linear, exponential
    initialInterval: "PT1S"
    maxInterval: "PT5M"
    multiplier: 2.0
    nonRetryableErrors: ["ValidationError", "AuthorizationDenied"]
```

### 13.3 Timeout Categories

`stepTimeout` (per action), `taskTimeout` (per task, typically = SLA), `instanceTimeout` (per instance), `heartbeatTimeout` (long-running action heartbeat), `queueTimeout` (task in Available state).

### 13.4 Compensation

Activities with side effects SHOULD register compensation handlers. On `compensate` action, handlers execute in reverse completion order.

---

## 14. Actor Model

This section is normative.

### 14.1 Actor Types

| Type | Description | Provenance Requirements |
|------|-------------|------------------------|
| `human` | A person performing tasks, making decisions, exercising judgment. | Identity, role, timestamp. |
| `system` | A deterministic software component. | Component identifier, version, timestamp. |
| `agent` | An AI system performing reasoning, classification, or recommendation. Outputs are non-deterministic and carry confidence. | Model identifier, model version, confidence report, input summary, timestamp. |

### 14.2 Normative Constraints

1. Every provenance record MUST include the actor type and identifier.
2. Agent actors MUST include model identifier, model version, and confidence report in all provenance records.
3. Human actors have override authority over agent outputs. Agent actors MUST NOT override human decisions.
4. The actor type is based on who bears decision authority for the specific action. A human using an AI tool remains a `human` actor if the human reviews and commits the output.
5. An `agent` actor operating at `autonomous` level MUST NOT invoke other agents at `autonomous` level without explicit `cascadingAutonomy: "permitted"` in the workflow definition, with `maxCascadeDepth` (default: 2).

---

## 15. Expression Language

This section is normative.

### 15.1 Overview

The WOS Expression Language is based on a profile of FEEL (DMN) chosen for readability by non-developers and executability by machines.

### 15.2 Supported Constructs

Literals (strings, numbers, booleans, null, dates, date-times, durations), dot-separated paths, array access, arithmetic, comparison, boolean operators, conditionals (`if/then/else`), membership (`in`), string operations, list operations, date/time operations, context construction, and null-safe evaluation.

### 15.3 Expression Context

| Variable | Description |
|----------|-------------|
| `caseFile` | Current case data. |
| `event` | Triggering event data (transition guards/actions only). |
| `task` | Current task data (task expressions only). |
| `instance` | Workflow instance metadata. |
| `parameters` | Temporal parameters. |
| `agent` | Agent operational state including calibration metrics (agent policy expressions only). |
| `env` | Implementation-defined environment. |

### 15.4 Expression Safety

WOS Expressions MUST be pure functions: no side effects, state modification, or I/O. Expressions MUST terminate in bounded time.

---

## 16. Due Process Requirements

This section is normative for workflows with `impactLevel: "rights-impacting"` or `"safety-impacting"` and RECOMMENDED for all workflows.

### 16.1 Overview

When a workflow produces decisions that affect individual rights, benefits, services, or obligations, due process protections are required. These requirements are informed by constitutional due process principles established in case law (State v. Loomis, Houston Federation of Teachers v. Houston ISD, Elder v. Gillespie), statutory requirements (APA, ECOA Regulation B), and regulatory frameworks (OMB M-24-10, EU AI Act).

### 16.2 Due Process Configuration

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

### 16.3 Notice Requirements

When a workflow produces an adverse decision (denial, reduction, termination, or other unfavorable determination) in a `rights-impacting` workflow:

1. The affected individual MUST receive notice before the decision takes effect.
2. The notice MUST include: the specific determination made, the factual basis for the determination using individualized reason codes (not generic statements), the individual's right to appeal, the appeal deadline and process, and disclosure that an AI system assisted in the determination (if applicable).
3. The notice MUST be issued as a `dueProcessNotice` provenance record before the adverse action is effected.

### 16.4 Explanation Requirements

| Level | Description |
|-------|-------------|
| `individualized` | Specific factual reasons tied to the individual's case. REQUIRED for `rights-impacting`. |
| `categorical` | Category-level explanation (e.g., "income exceeded threshold"). RECOMMENDED for `operational`. |
| `aggregate` | System-level transparency without individual explanation. Minimum for `informational`. |

When `counterfactualRequired` is `true`, the explanation MUST include both positive counterfactuals (what controllable factors would change the outcome) and negative counterfactuals (what irrelevant factors, including protected characteristics, did NOT affect the outcome).

### 16.5 Appeal Mechanisms

When `appealMechanism.enabled` is `true`:

1. The appeal MUST be reviewed by a human adjudicator. AI agents MAY assist in preparing information for the adjudicator but MUST NOT serve as the appeal decision-maker.
2. The adjudicator MUST be independent of the original determination (`constraint: "independentFromOriginal"`).
3. When `continuationOfServices` is `true`, current benefits or services MUST continue at `continuationScope` level during the appeal period.
4. Filing an appeal MUST produce an `appealFiled` provenance record.

### 16.6 Agent Disclosure

When AI agents participate in consequential decisions, the `agentDisclosure` configuration controls what is communicated to affected individuals. At minimum, `discloseThatAgentAssisted: true` MUST be set for `rights-impacting` workflows (consistent with OMB M-24-10 requirements and EU AI Act Art. 13 transparency obligations).

---

## 17. Versioning and Evolution

This section is normative.

### 17.1 Workflow Definition Versioning

Semantic Versioning: major (breaking), minor (backward-compatible additions), patch (corrections).

### 17.2 Instance Migration

Default: pinned execution (instances continue under their creation version). Optional: forward migration with safety verification and provenance record.

### 17.3 Schema Evolution

Backward-compatible: adding optional properties, widening constraints, adding enum values, adding items. Breaking: removing properties, adding required properties without defaults, narrowing constraints, renaming, type changes. Breaking changes MUST increment major version.

---

## 18. Security and Access Control

This section is normative.

### 18.1 Roles

| Role | Scope | Description |
|------|-------|-------------|
| `workflowAdministrator` | Definition | Create, modify, retire definitions. |
| `instanceInitiator` | Instance | Create instances. |
| `caseParticipant` | Instance | View case data and task status (subject to visibility). |
| `taskWorker` | Task | Claim and perform tasks. |
| `taskAdministrator` | Task | Reassign, escalate, cancel tasks. |
| `auditor` | Instance | View all provenance and case data. |

### 18.2 Enforcement Points

Authorization checked at: instance creation, task operations, case file access, decision invocation, provenance access, override execution, agent configuration changes, and guardrail bypass. Failures MUST be logged.

---

## 19. Extensibility

This section is normative.

Extension properties use namespaced prefixes (`x-agency:classification`). Extensions MUST NOT alter core semantics, MUST be preserved during round-trips, MUST NOT cause document rejection, and MUST NOT use the reserved `wos:` prefix.

---

## 20. Serialization

This section is normative.

**Formats:** YAML 1.2 (human authoring) or JSON (machine interchange). Lossless conversion required.

**Character encoding:** UTF-8.

**Identifiers:** `[a-zA-Z][a-zA-Z0-9_-]*`, case-sensitive, unique within scope. Cross-document: URIs.

**Timestamps:** RFC 3339 with timezone. UTC recommended for provenance.

**Durations:** ISO 8601. Business durations: `BD` (business days), `BH` (business hours).

---

## 21. Conformance Profiles

This section is normative.

### 21.1 Profile: Structural

Parse, validate, round-trip, preserve extensions. Enables editors, validators, linters, migration tools.

### 21.2 Profile: Lifecycle

Structural + execute lifecycle semantics, produce transition records, support all state types, milestones, transition resolution.

### 21.3 Profile: Task Management

Lifecycle + full task lifecycle, all operations, separation of duties, SLA timers, structured oversight protocols.

### 21.4 Profile: Decision

Lifecycle + decision tables with all hit policies, WOS Expression Language, temporal parameters, decision records.

### 21.5 Profile: Agent Governance

Lifecycle + Decision + enforce autonomy levels, produce agent provenance records (all types), enforce guardrails with violation records, enforce fallback chains, support confidence-based routing, support multi-step sessions with checkpoints, prevent agents from overriding human decisions.

### 21.6 Profile: Full

All of Lifecycle + Task Management + Decision + Agent Governance + case file with mutation tracking, integration semantics with correlation, all provenance record types, tamper evidence, durable execution guarantees, access control, due process requirements for `rights-impacting` workflows.

### 21.7 Profile: Verification

Structural + static soundness analysis (deadlock-freedom, livelock-freedom, proper termination, dead elements, fallback completeness), diagnostic reports, SHOULD support simulation.

---

## 22. Privacy Considerations

This section is informative.

Case data may be subject to privacy regulations. The `visibility` model restricts access. The `redactFields` mechanism limits agent data exposure. Provenance records may contain personal data; implementations SHOULD support configurable retention and anonymization. The Claim Check pattern keeps document content in access-controlled storage. The right to erasure may conflict with audit requirements; implementations SHOULD support redaction while preserving audit trail integrity.

---

## 23. Security Considerations

This section is informative.

1. **Expression sandboxing.** WOS Expressions evaluated in isolated context with no system access or side effects.
2. **Event authentication.** External events SHOULD be signature-verified.
3. **Provenance integrity.** Signed tree heads stored independently from provenance records.
4. **Case file encryption.** Data at rest and in transit SHOULD be encrypted.
5. **Separation of duties enforcement.** Constraints enforced at application layer, not solely UI.
6. **Agent impersonation.** Agents authenticated with same rigor as humans. Agents MUST NOT claim human identity.
7. **Prompt injection defense.** `isolateUntrustedData` implements the CaMeL dual-LLM pattern. Guardrails provide structural defense regardless of agent compromise. No single defense is sufficient; defense in depth is required.
8. **Model version drift.** Implementations using non-pinned version policies SHOULD monitor output distributions and alert on significant divergence. Shadow deployment RECOMMENDED before production changes.
9. **Cascading autonomy.** Autonomous agents invoking other autonomous agents compound risk. `cascadingAutonomy` declaration and `maxCascadeDepth` make chains visible and bounded.
10. **Tool use boundaries.** Agent tool permissions follow least-privilege. Tool invocations MUST be recorded. Side-effecting tools at autonomous level require explicit policy declaration.

---

## 24. References

### 24.1 Normative References

**[RFC2119]** Bradner, S., "Key words for use in RFCs to Indicate Requirement Levels", BCP 14, RFC 2119, March 1997.

**[RFC3339]** Klyne, G. and C. Newman, "Date and Time on the Internet: Timestamps", RFC 3339, July 2002.

**[RFC3986]** Berners-Lee, T., et al., "Uniform Resource Identifier (URI): Generic Syntax", RFC 3986, January 2005.

**[RFC8174]** Leiba, B., "Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words", BCP 14, RFC 8174, May 2017.

**[RFC8259]** Bray, T., "The JavaScript Object Notation (JSON) Data Interchange Format", RFC 8259, December 2017.

**[YAML]** Ben-Kiki, O., Evans, C., and I. döt Net, "YAML Ain't Markup Language Version 1.2", October 2021.

**[SemVer]** Preston-Werner, T., "Semantic Versioning 2.0.0".

**[ISO8601]** ISO, "ISO 8601:2019 Date and time — Representations for information interchange".

**[CloudEvents]** CNCF, "CloudEvents Specification Version 1.0.2", 2022.

**[PROV-DM]** W3C, "PROV-DM: The PROV Data Model", W3C Recommendation, April 2013.

**[TraceContext]** W3C, "Trace Context", W3C Recommendation, February 2020.

**[DMN]** OMG, "Decision Model and Notation Version 1.4", March 2021.

**[OpenAPI]** Linux Foundation, "OpenAPI Specification Version 3.1.0", February 2021.

**[AsyncAPI]** AsyncAPI Initiative, "AsyncAPI Specification Version 3.0", 2023.

**[JSON Schema]** IETF, "JSON Schema: A Media Type for Describing JSON Documents", draft-bhutton-json-schema-01, 2022.

### 24.2 Informative References

**[Harel1987]** Harel, D., "Statecharts: A Visual Formalism for Complex Systems", Science of Computer Programming, 8(3), pp. 231–274, 1987.

**[SCXML]** W3C, "State Chart XML (SCXML)", W3C Recommendation, September 2015.

**[BPMN]** OMG, "Business Process Model and Notation Version 2.0", ISO/IEC 19510:2013.

**[CMMN]** OMG, "Case Management Model and Notation Version 1.1", December 2016.

**[WS-HumanTask]** OASIS, "Web Services – Human Task Version 1.1", August 2012.

**[XACML]** OASIS, "eXtensible Access Control Markup Language Version 3.0", January 2013.

**[Sagas]** Garcia-Molina, H. and Salem, K., "Sagas", ACM SIGMOD, 1987.

**[WorkflowPatterns]** van der Aalst, W.M.P., et al., "Workflow Patterns", Distributed and Parallel Databases, 14(1), pp. 5–51, 2003.

**[XES]** IEEE, "IEEE Standard for eXtensible Event Stream (XES)", IEEE Std 1849-2016.

**[RFC9162]** Laurie, B., et al., "Certificate Transparency Version 2.0", RFC 9162, December 2021.

**[NIST-SP-800-53]** NIST, "Security and Privacy Controls for Information Systems and Organizations", SP 800-53 Rev 5, September 2020.

**[OMB-M-24-10]** OMB, "Advancing Governance, Innovation, and Risk Management for Agency Use of Artificial Intelligence", M-24-10, March 2024.

**[EU-AI-Act]** European Parliament, "Regulation (EU) 2024/1689 on Artificial Intelligence", June 2024.

**[NIST-AI-RMF]** NIST, "AI Risk Management Framework (AI RMF 1.0)", AI 100-1, January 2023.

**[Vaccaro2024]** Vaccaro, M., Almaatouq, A., and Malone, T., "When combinations of humans and AI are useful", Nature Human Behaviour, 8, pp. 2287–2297, 2024.

**[Turpin2023]** Turpin, M., et al., "Language Models Don't Always Say What They Think", NeurIPS 2023.

**[Lanham2023]** Lanham, T., et al., "Measuring Faithfulness in Chain-of-Thought Reasoning", Anthropic, 2023.

**[Chen2023]** Chen, L., Zaharia, M., and Zou, J., "How Is ChatGPT's Behavior Changing over Time?", 2023.

**[Buçinca2021]** Buçinca, Z., Malaya, M.B., and Gajos, K.Z., "To Trust or to Think: Cognitive Forcing Functions Can Reduce Overreliance on AI", CSCW 2021.

**[Nasr2025]** Nasr, M., et al., "The Attacker Moves Second", 2025.

**[CaMeL]** Debenedetti, E., et al., "CaMeL: Causal Mediation for LLM Defense", Google DeepMind, 2025.

**[FormalLLM]** Li, Y., et al., "Formal-LLM: Integrating Formal Language and Natural Language for Controllable LLM-based Agents", EMNLP 2024.

**[ABC]** "Agent Behavioral Contracts: Formal Specification and Runtime Enforcement", February 2026.

**[Feng2025]** Feng, Y., McDonald, A., and Zhang, B., "Levels of Autonomy for AI Agents", Knight First Amendment Institute, Columbia University, 2025.

**[Wachter2018]** Wachter, S., Mittelstadt, B., and Russell, C., "Counterfactual Explanations Without Opening the Black Box", Harvard Journal of Law & Technology, 31(2), 2018.

**[MCP]** Anthropic, "Model Context Protocol Specification", 2024.

**[A2A]** Google, "Agent-to-Agent Protocol", Linux Foundation, 2025.

---

## Appendix A. JSON Schema for WOS Core

This appendix is normative. The complete JSON Schema is maintained at the specification repository. An abbreviated structural schema is provided showing the top-level document shape, with full `$defs` for State, Transition, Action, DecisionService, TaskDefinition, AgentConfiguration, CaseFile, Integration, ProvenanceConfig, ExecutionConfig, DueProcessConfig, and all sub-objects.

The schema is available at: `https://wos-spec.org/schema/core/2.0.0`

---

## Appendix B. Complete Example

This appendix is informative. A complete WOS Document for a Community Development Block Grant review workflow — demonstrating all eight layers, agent governance with guardrails, structured oversight with the `independentFirst` protocol, defeasible eligibility rules with temporal parameters, parallel technical reviews with separation of duties, due process protections for adverse decisions, and the four-layer audit model — is maintained at the specification repository and published separately as the **WOS Reference Workflow**.

The Reference Workflow demonstrates every normative concept in this specification applied to a realistic government grant process under 24 CFR Part 570.

---

## Appendix C. Relationship to Existing Standards

| Standard | Relationship |
|----------|-------------|
| **BPMN 2.0** | WOS adopts event taxonomy and error handling concepts; replaces flowchart topology with statecharts. |
| **CMMN 1.1** | WOS adopts case file model, discretionary items, sentries, milestones. |
| **DMN 1.4** | WOS adopts decision tables, hit policies, FEEL expression language profile; extends with defeasible rules and temporal parameters. |
| **SCXML 1.0** | WOS adopts statechart semantics; replaces XML with YAML/JSON. |
| **WS-HumanTask 1.1** | WOS adopts task lifecycle and role model, simplified and modernized; adds structured oversight protocols. |
| **CloudEvents 1.0** | Adopted as event envelope with WOS extension attributes. |
| **W3C PROV** | WOS adopts Entity-Activity-Agent triad; extends with four-layer audit architecture and decision records. |
| **OpenAPI / AsyncAPI** | Referenced for integration interface contracts. |
| **JSON Schema 2020-12** | Used for all data validation. |
| **OMB M-24-10 / M-25-21** | WOS operationalizes M-24-10's rights-impacting AI requirements as workflow constraints. |
| **EU AI Act** | WOS's provenance, transparency, human oversight, and monitoring requirements satisfy high-risk AI system obligations (Arts. 9, 12, 13, 14). |
| **NIST AI RMF** | WOS layers map to GOVERN (Layer 4), MAP (Layer 2), MEASURE (§9.8 monitoring), MANAGE (Layers 3, 7). |
| **MCP** | WOS defines extension points for MCP tool integration within the governance envelope. |
| **A2A** | WOS defines extension points for A2A inter-agent communication. |

---

## Appendix D. Changelog

| Date | Version | Description |
|------|---------|-------------|
| 2026-04-08 | 2.0.0 | Complete redesign. Eight-layer architecture with Agent Governance as a first-class layer. Four-layer audit model. Structured oversight protocols. Due process requirements. Research-informed design throughout. |
