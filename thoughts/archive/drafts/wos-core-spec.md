# Workflow Orchestration Standard (WOS) Core Specification

## W3C First Public Working Draft

**Latest published version:**
: https://wos-spec.org/TR/wos-core/

**Editor's Draft:**
: https://wos-spec.org/ed/wos-core/

**Editors:**
: Mike (TealWolf Consulting LLC)

**Version:**
: 0.1.0

**Date:**
: 8 April 2026

**Status:**
: First Public Working Draft

---

## Abstract

This specification defines the Workflow Orchestration Standard (WOS), a declarative, machine-readable language for describing high-stakes, long-running, human-in-the-loop workflows. WOS provides a layered architecture separating process lifecycle, decision logic, human task management, case state, integration, provenance, and durable execution into independently evolvable concerns. The standard is designed to support consequential workflows — including grants processing, benefits adjudication, licensing, inspections, investigations, compliance, and case review — where auditability, explainability, human judgment, and formal verifiability are essential requirements. WOS is serialized in YAML/JSON, validated by JSON Schema, and optimized for AI-assisted authoring, validation, and simulation.

---

## Status of This Document

This section describes the status of this document at the time of its publication.

This document is a First Public Working Draft. It has not been endorsed by any standards body and has no formal standing. It is published to solicit feedback from implementers, domain experts, standards practitioners, and the broader workflow and case management community.

This is a living specification. The editors intend to iterate on it based on implementation experience and community review. Substantive changes will be tracked in a public changelog.

Comments on this specification are welcome and may be submitted as issues at the specification's repository.

Publication as a Working Draft does not imply endorsement. This is a draft document and may be updated, replaced, or obsoleted at any time.

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
9. [Layer 4: Case State and Evidence](#9-layer-4-case-state-and-evidence)
10. [Layer 5: Integration and Eventing](#10-layer-5-integration-and-eventing)
11. [Layer 6: Provenance and Audit](#11-layer-6-provenance-and-audit)
12. [Layer 7: Durable Execution Contract](#12-layer-7-durable-execution-contract)
13. [Expression Language](#13-expression-language)
14. [Versioning and Evolution](#14-versioning-and-evolution)
15. [Security and Access Control](#15-security-and-access-control)
16. [Extensibility](#16-extensibility)
17. [Serialization](#17-serialization)
18. [Conformance Profiles](#18-conformance-profiles)
19. [Privacy Considerations](#19-privacy-considerations)
20. [Security Considerations](#20-security-considerations)
21. [References](#21-references)

**Appendices**

- [A. JSON Schema for WOS Core](#appendix-a-json-schema-for-wos-core)
- [B. Complete Example: Grant Application Workflow](#appendix-b-complete-example)
- [C. Relationship to Existing Standards](#appendix-c-relationship-to-existing-standards)
- [D. Changelog](#appendix-d-changelog)

---

## 1. Introduction

### 1.1 Background

High-stakes workflows — grants processing, benefits adjudication, regulatory licensing, inspections, investigations, compliance review, and similar consequential processes — share a set of requirements poorly served by existing standards and systems.

These workflows are long-running: a single case may span weeks, months, or years. They are human-centric: professional judgment, discretionary action, and override authority are not exceptions but core operating modes. They are evidence-driven: decisions depend on accumulated documents, data, and findings that evolve over the case's lifetime. They are heavily regulated: every action, decision, and delegation must be auditable, explainable, and traceable to specific authority. And they are high-stakes: errors carry consequences for individuals, organizations, and the public interest.

Existing workflow standards address subsets of these requirements. BPMN provides rich process flow modeling but treats human judgment as a peripheral concern. CMMN models discretionary casework but lacks integration and execution semantics. DMN separates decision logic from process flow but does not address task management or provenance. WS-HumanTask defines a comprehensive task lifecycle but is coupled to SOAP-era transport assumptions. No existing standard integrates the full stack — lifecycle, decisions, tasks, evidence, integration, audit, and durability — into a coherent whole designed for modern systems and AI-assisted authoring.

### 1.2 Design Goals

WOS is designed to satisfy the following goals, listed in priority order:

1. **Correctness.** Workflow definitions MUST be formally verifiable for soundness properties including deadlock-freedom, livelock-freedom, proper termination, and reachability. The specification defines what constitutes a sound workflow and how soundness is verified.

2. **Auditability.** Every state transition, decision evaluation, task operation, and data mutation MUST produce an immutable, tamper-evident audit record sufficient to answer: what happened, when, by whom, under what authority, using what data, and why.

3. **Explainability.** Every automated decision, routing choice, and policy evaluation MUST be traceable to specific rule versions, input data, and evaluation logic. Human overrides MUST include structured rationale.

4. **Human judgment as a first-class concern.** Discretionary action, professional judgment, override authority, delegation, escalation, and exception handling MUST be modeled as core capabilities, not escape hatches.

5. **Separation of concerns.** Process topology, decision logic, task management, case state, integration, provenance, and execution guarantees are distinct concerns. Each MUST be independently authorable, versionable, testable, and evolvable.

6. **Interoperability.** Workflow definitions MUST be portable across conformant implementations. The standard defines serialization formats, expression languages, and interface contracts sufficient for portability.

7. **AI-native authoring.** The serialization format, object model, and expression language MUST be designed for reliable generation, validation, transformation, and explanation by large language models and other AI systems.

8. **Incremental adoption.** Implementations MAY conform to subsets of the specification via defined conformance profiles. Simple workflows MUST NOT require engagement with the full specification.

### 1.3 Scope

This specification defines the core object model, layered architecture, serialization format, expression language, and conformance requirements for WOS.

The following are within scope of this specification:

- The seven-layer conceptual architecture and the interfaces between layers.
- The metamodel: the objects, relationships, and constraints that constitute a WOS workflow definition.
- Lifecycle semantics based on hierarchical state machines (Harel statecharts).
- Decision service interfaces and decision table structure.
- Human task lifecycle, assignment model, and escalation semantics.
- Case file structure, evidence model, and data validation.
- Event envelope format and correlation architecture for external integration.
- Provenance record structure and tamper-evidence requirements.
- Abstract durable execution guarantees.
- The expression language profile for guards, conditions, and data transformation.
- Serialization in YAML and JSON with JSON Schema validation.
- Conformance profiles and testing requirements.

The following are out of scope:

- User interface rendering and form specification.
- Specific persistence or storage mechanisms.
- Specific transport protocols (HTTP, gRPC, messaging) beyond interface contracts.
- Process mining algorithms (the standard defines compatible event emission, not mining logic).
- Machine learning model training or inference specification.
- Document management systems.
- Notification delivery mechanisms.
- General-purpose computation.

### 1.4 Relationship to This Specification and Tier Specifications

This document is the **Core Specification**. It defines the complete architecture, object model, and normative semantics for WOS.

Subsequent **Tier Specifications** elaborate individual layers in detail. Each Tier Specification is normative for its layer but MUST NOT contradict the Core Specification. Where a Tier Specification provides more detailed semantics than the Core Specification, the Tier Specification governs.

Planned Tier Specifications include:

- **WOS-Lifecycle** — Detailed statechart semantics, transition resolution, and verification algorithms.
- **WOS-Decision** — Decision table semantics, hit policies, defeasible rules, and temporal parameters.
- **WOS-Task** — Human task lifecycle, assignment, SLA enforcement, and separation of duties.
- **WOS-CaseState** — Case file schema, evidence management, and selective visibility.
- **WOS-Integration** — Event consumption/production, correlation, and interface contracts.
- **WOS-Provenance** — Audit record schema, tamper evidence, and process mining interoperability.
- **WOS-Execution** — Durable execution guarantees, retry policies, and compensation semantics.
- **WOS-Conformance** — Test suite, canonical fixtures, and certification procedures.

### 1.5 Notational Conventions

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in BCP 14 [RFC 2119] [RFC 8174] when, and only when, they appear in all capitals, as shown here.

The key words "normative" and "informative" indicate whether a section contains requirements that conformant implementations MUST satisfy or supplementary material provided for context.

---

## 2. Conformance

### 2.1 Conformance Classes

This specification defines two conformance classes:

**WOS Document.** A WOS Document is a serialized workflow definition that conforms to the structural and semantic requirements of this specification. A conformant WOS Document MUST validate against the WOS Core JSON Schema (Appendix A) and MUST satisfy the static semantic constraints defined in this specification.

**WOS Processor.** A WOS Processor is a software system that consumes WOS Documents and produces behavior consistent with the semantics defined in this specification. A WOS Processor MAY conform to one or more Conformance Profiles (§18).

### 2.2 Conformance Requirements for WOS Documents

A conformant WOS Document:

1. MUST be serialized in either YAML (as defined by YAML 1.2 [YAML]) or JSON (as defined by RFC 8259 [RFC8259]).
2. MUST validate against the WOS Core JSON Schema without errors.
3. MUST satisfy all static semantic constraints defined in this specification, including but not limited to: unique identifiers within scope, valid state references in transitions, type-correct guard expressions, and resolvable references to decision services and task definitions.
4. MUST include all REQUIRED properties as defined by the object model.
5. SHOULD pass soundness verification as defined in §6.8.

### 2.3 Conformance Requirements for WOS Processors

A conformant WOS Processor:

1. MUST accept any conformant WOS Document without error.
2. MUST reject any document that fails JSON Schema validation, producing diagnostics identifying each violation.
3. MUST execute lifecycle semantics (§6) consistent with this specification.
4. MUST produce audit records (§11) for every state transition, task operation, and decision evaluation.
5. MUST support at least one Conformance Profile (§18).
6. SHOULD provide static soundness verification for workflow definitions.
7. SHOULD support round-trip serialization between YAML and JSON without semantic loss.

### 2.4 Partial Conformance

An implementation that conforms to a subset of this specification MUST declare which Conformance Profile(s) it supports and MUST NOT claim general WOS conformance.

---

## 3. Terminology

This section is normative.

**Activity.** A unit of work within a workflow. Activities are either automated (performed by a system) or human (performed by a person via a Task).

**Case.** An instance of a workflow applied to a specific subject. A case has a lifecycle, accumulates data and evidence, and produces audit records.

**Case File.** The structured data container associated with a Case. The Case File holds all typed data items, evidence references, and computed values relevant to the case.

**Case File Item.** A single typed datum within a Case File, defined by a JSON Schema and subject to access control.

**Compensation.** A semantically meaningful reversal of a completed activity, used to undo the effects of work when a workflow must partially roll back.

**Conformance Profile.** A named subset of this specification that an implementation may claim to support.

**Decision Record.** An immutable audit entry recording the evaluation of a Decision Service, including the rule version, input data snapshot, evaluation trace, output, and any override.

**Decision Service.** An encapsulated unit of decision logic with defined inputs and outputs, independently versionable and invocable from any point in the workflow.

**Defeasible Rule.** A rule that may be overridden by a more specific rule with higher priority. The standard uses structured exception handling to model the "general rule with exceptions" pattern common in regulatory contexts.

**Deferred Choice.** A modeling pattern where the next step in a workflow is determined by an external event or human action at runtime, rather than predetermined by the workflow designer.

**Durable Timer.** A timer that persists across system restarts and consumes no runtime resources while waiting. Durable timers may span arbitrarily long periods.

**Evidence.** A document, dataset, image, or other artifact attached to a Case File Item with content integrity verification.

**Guard.** A boolean expression evaluated against case data and context that controls whether a transition may fire. Guards are expressed in WOS Expression Language (§13).

**History State.** A mechanism that records the last active substate within a compound state, enabling the workflow to resume its prior internal configuration after suspension.

**Lifecycle.** The set of states and transitions that define how a Case or Task progresses from creation to completion.

**Milestone.** A named condition on case data that, when satisfied, indicates meaningful progress in the case. Milestones are declarative checkpoints, not activities.

**Override.** A human action that supersedes an automated decision or system recommendation. Overrides MUST include structured rationale and are recorded in the audit trail.

**Parallel Region.** An independently executing concurrent track within a compound state. All parallel regions must reach their final states before the compound state can transition.

**Provenance Record.** An immutable audit entry conforming to the W3C PROV data model, recording what happened, when, by whom, and why.

**Sentry.** A combination of an event trigger and a guard condition that controls the activation of a stage or the achievement of a milestone, adopted from CMMN.

**Soundness.** A formal property of a workflow definition guaranteeing: (a) from the initial state, the final state is always reachable (no deadlocks); (b) no infinite non-productive loops exist (no livelocks); (c) when the final state is reached, no tokens remain in other states (proper termination); and (d) every defined state and transition can potentially be reached (no dead elements).

**Stage.** A compound lifecycle phase that may contain substates, activities, milestones, and sentries. Stages may be nested hierarchically.

**State.** A named condition in the lifecycle of a Case, Task, or Stage. States are the nodes of the statechart.

**Task.** A unit of human work with a defined lifecycle, assignment model, and data contract. Tasks are the primary mechanism for human participation in workflows.

**Transition.** A directed edge between two states, triggered by an event and optionally gated by a guard condition.

**Workflow Definition.** A complete, self-contained specification of a workflow's lifecycle, decisions, tasks, case structure, integrations, and audit requirements. A Workflow Definition is a WOS Document.

**Workflow Instance.** A running case governed by a Workflow Definition.

---

## 4. Architecture Overview

This section is normative.

### 4.1 Layered Architecture

WOS defines seven conceptual layers. Each layer addresses a distinct concern, has a well-defined interface to adjacent layers, and is independently authorable and versionable.

```
┌─────────────────────────────────────────────────────┐
│  Layer 7: Durable Execution Contract                │
│  Abstract guarantees for runtime resilience          │
├─────────────────────────────────────────────────────┤
│  Layer 6: Provenance and Audit                      │
│  Immutable records, decision traces, tamper evidence │
├─────────────────────────────────────────────────────┤
│  Layer 5: Integration and Eventing                  │
│  CloudEvents, correlation, interface contracts       │
├─────────────────────────────────────────────────────┤
│  Layer 4: Case State and Evidence                   │
│  Typed case data, evidence, selective visibility     │
├─────────────────────────────────────────────────────┤
│  Layer 3: Human Task Management                     │
│  Task lifecycle, assignment, escalation, SLAs        │
├─────────────────────────────────────────────────────┤
│  Layer 2: Decision and Policy                       │
│  Decision tables, rules, temporal parameters         │
├─────────────────────────────────────────────────────┤
│  Layer 1: Lifecycle and Topology                    │
│  Statechart, states, transitions, guards, milestones │
└─────────────────────────────────────────────────────┘
```

The layers are ordered by dependency: higher layers depend on lower layers but not the reverse. Layer 1 has no dependencies on other layers. Layers 2 and 3 depend on Layer 1 for lifecycle context but not on each other. Layer 4 is referenced by Layers 1–3 for data access. Layer 5 provides the external boundary. Layer 6 observes all other layers. Layer 7 constrains the runtime environment for all layers.

### 4.2 Separation Principles

The following separation principles are normative.

**Process topology MUST be separated from decision logic.** The lifecycle (Layer 1) defines the structure of states and transitions. Decision Services (Layer 2) evaluate conditions and produce routing recommendations. A transition guard MAY invoke a Decision Service, but the Decision Service MUST NOT contain process topology.

**Decision logic MUST be separated from task management.** A Decision Service determines what should happen. A Task definition determines who does it and how the work is managed. These are independently versionable.

**Case data MUST be separated from process state.** The Case File (Layer 4) holds business data. The lifecycle state (Layer 1) tracks process progress. A workflow's state is the combination of its lifecycle state and its case data, but these are modeled separately.

**Audit MUST be separated from execution.** Provenance records (Layer 6) are produced as a consequence of execution but are not part of the execution model. The audit layer observes; it does not participate in control flow.

**Execution guarantees MUST be separated from execution mechanisms.** Layer 7 defines what guarantees a conformant runtime provides (durability, at-least-once execution, durable timers). It does not define how those guarantees are achieved.

### 4.3 Cross-Cutting Concerns

Certain concerns span all layers:

**Identity.** Every object in a WOS Document has a unique identifier within its scope. Cross-document references use URIs.

**Versioning.** The Workflow Definition, each Decision Service, and each Task Definition carry independent version identifiers. Running instances record the version under which they were created.

**Expressions.** Guard conditions, data transformations, and computed values throughout all layers use the WOS Expression Language (§13).

**Access Control.** Authorization rules may govern who can view or modify objects at any layer. The access control model is defined in §15.

---

## 5. Document Structure

This section is normative.

### 5.1 Top-Level Structure

A WOS Document is a YAML or JSON document with the following top-level structure:

```yaml
wos: "0.1.0"                          # REQUIRED
id: "urn:wos:example.org:grant-review" # REQUIRED
name: "Grant Application Review"       # REQUIRED
version: "2.3.0"                       # REQUIRED
status: "draft"                        # REQUIRED

metadata:                              # OPTIONAL
  description: "..."
  authors: [...]
  created: "2026-04-08T00:00:00Z"
  modified: "2026-04-08T00:00:00Z"
  tags: [...]
  jurisdiction: "..."
  authority: "..."

lifecycle:                             # REQUIRED
  # Layer 1: States, transitions, guards, milestones
  ...

decisions:                             # OPTIONAL
  # Layer 2: Decision services
  ...

tasks:                                 # OPTIONAL
  # Layer 3: Human task definitions
  ...

caseFile:                              # OPTIONAL
  # Layer 4: Case data schema and evidence model
  ...

integrations:                          # OPTIONAL
  # Layer 5: External system interfaces and events
  ...

provenance:                            # OPTIONAL
  # Layer 6: Audit configuration
  ...

execution:                             # OPTIONAL
  # Layer 7: Retry policies, timeouts, compensation
  ...

extensions:                            # OPTIONAL
  # Extension points
  ...
```

### 5.2 Property: `wos`

The `wos` property is REQUIRED. Its value is a string identifying the version of this specification to which the document conforms. The value MUST be a semantic version string matching the pattern `MAJOR.MINOR.PATCH`.

A WOS Processor MUST reject a document whose `wos` version has a major version it does not support.

### 5.3 Property: `id`

The `id` property is REQUIRED. Its value is a URI [RFC 3986] that uniquely identifies this Workflow Definition. The URI SHOULD use the `urn:wos:` scheme for definitions not associated with a retrievable resource.

### 5.4 Property: `version`

The `version` property is REQUIRED. Its value is a semantic version string [SemVer] identifying the version of this Workflow Definition. When a Workflow Instance is created, the `version` of the governing definition is recorded immutably in the instance metadata.

### 5.5 Property: `status`

The `status` property is REQUIRED. Its value MUST be one of:

| Value | Meaning |
|-------|---------|
| `draft` | Under development. MUST NOT be used for production workflow instances. |
| `active` | Approved for production use. New workflow instances MAY be created against this version. |
| `deprecated` | Superseded by a newer version. Existing instances continue; new instances SHOULD NOT be created. |
| `retired` | No longer in use. New instances MUST NOT be created. |

### 5.6 Property: `metadata`

The `metadata` property is OPTIONAL. When present, it provides descriptive information about the Workflow Definition. The following subproperties are defined:

| Property | Type | Description |
|----------|------|-------------|
| `description` | string | Human-readable description of the workflow's purpose and scope. |
| `authors` | array of string | Identifiers or names of the definition's authors. |
| `created` | string (datetime) | RFC 3339 timestamp of initial creation. |
| `modified` | string (datetime) | RFC 3339 timestamp of last modification. |
| `tags` | array of string | Categorization tags. |
| `jurisdiction` | string | Legal or regulatory jurisdiction governing this workflow. |
| `authority` | string | The statute, regulation, or policy authorizing this workflow. |
| `effectiveDate` | string (date) | The date on which this version becomes effective. |
| `sunsetDate` | string (date) | The date after which this version is no longer effective. |

Implementations MAY define additional metadata properties via the extension mechanism (§16).

---

## 6. Layer 1: Lifecycle and Topology

This section is normative.

### 6.1 Overview

The Lifecycle layer defines the statechart that governs a workflow instance's progression from creation through completion or termination. The semantics are based on Harel statecharts [Harel1987] as formalized in W3C SCXML [SCXML], adapted for case-oriented workflows.

A lifecycle is a directed graph of states connected by transitions. States may be simple (atomic) or compound (containing substates). Compound states may contain parallel regions for concurrent execution. Transitions are triggered by events and gated by guard conditions.

### 6.2 Property: `lifecycle`

The `lifecycle` property is REQUIRED. It defines the top-level statechart for the workflow.

```yaml
lifecycle:
  initialState: "intake"

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
          initialState: "technicalPending"
          states:
            technicalPending:
              # ...
            technicalComplete:
              type: "final"
        financialReview:
          initialState: "financialPending"
          states:
            financialPending:
              # ...
            financialComplete:
              type: "final"
      transitions:
        - event: "regions.allFinal"
          target: "adjudication"

    adjudication:
      type: "compound"
      initialState: "decisionPending"
      historyState: "shallow"
      states:
        # ...

    completed:
      type: "final"

  milestones:
    applicationReceived:
      condition: "caseFile.application != null"
    eligibilityConfirmed:
      condition: "caseFile.eligibility.decision = 'eligible'"
    fundingApproved:
      condition: "caseFile.award.status = 'approved'"
```

### 6.3 States

A state represents a condition in the lifecycle of a workflow instance. Every state has the following properties:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `type` | enum | REQUIRED | One of `atomic`, `compound`, `parallel`, `final`. |
| `onEntry` | array of Action | OPTIONAL | Actions executed when the state is entered. |
| `onExit` | array of Action | OPTIONAL | Actions executed when the state is exited. |
| `transitions` | array of Transition | OPTIONAL | Outgoing transitions from this state. |
| `historyState` | enum | OPTIONAL | One of `shallow`, `deep`. Only valid for `compound` states. |
| `metadata` | object | OPTIONAL | Descriptive metadata. |

#### 6.3.1 Atomic States

An atomic state has no substates. It represents a single phase of the workflow. Atomic states MAY have onEntry/onExit actions and outgoing transitions.

#### 6.3.2 Compound States

A compound state contains an ordered set of substates, exactly one of which is designated as the `initialState`. When a compound state is entered, execution proceeds to its initial substate (unless a history state directs otherwise).

```yaml
adjudication:
  type: "compound"
  initialState: "review"
  historyState: "shallow"
  states:
    review:
      type: "atomic"
      # ...
    supervisorApproval:
      type: "atomic"
      # ...
    decided:
      type: "final"
```

A compound state with `historyState: "shallow"` records the last active direct substate. When the compound state is re-entered (for example, after suspension and resumption), execution resumes in the recorded substate rather than the initial substate.

A compound state with `historyState: "deep"` records the full active state configuration, including nested substates at all levels.

#### 6.3.3 Parallel States

A parallel state contains two or more named regions that execute concurrently. Each region has its own initial state and state machine. A parallel state is not exited until all regions have reached a final state, unless an explicit transition overrides this behavior.

```yaml
evaluation:
  type: "parallel"
  regions:
    technicalReview:
      initialState: "pending"
      states:
        pending:
          type: "atomic"
          # ...
        complete:
          type: "final"
    financialReview:
      initialState: "pending"
      states:
        pending:
          type: "atomic"
          # ...
        complete:
          type: "final"
```

When all regions reach a final state, the parallel state's implicit `regions.allFinal` event is raised. Transitions from the parallel state MAY use this event as a trigger.

#### 6.3.4 Final States

A final state indicates the completion of the enclosing compound or parallel region. A final state at the top level of the lifecycle indicates workflow completion. Final states MUST NOT have outgoing transitions.

### 6.4 Transitions

A transition is a directed edge from a source state to a target state. Transitions are triggered by events and optionally gated by guard conditions.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `event` | string | REQUIRED | The triggering event identifier. |
| `target` | string | REQUIRED | The identifier of the target state. |
| `guard` | string (expression) | OPTIONAL | A WOS Expression (§13) that must evaluate to `true` for the transition to fire. |
| `actions` | array of Action | OPTIONAL | Actions executed when the transition fires, after exiting the source state and before entering the target state. |
| `priority` | integer | OPTIONAL | Resolution priority when multiple transitions are eligible. Lower values indicate higher priority. Default: 0. |
| `description` | string | OPTIONAL | Human-readable explanation of this transition's purpose. |

#### 6.4.1 Transition Resolution

When an event occurs, a WOS Processor MUST evaluate transitions as follows:

1. Collect all transitions from the current active state(s) whose `event` matches the occurring event.
2. Evaluate the `guard` condition of each collected transition against the current case data and context. Discard transitions whose guard evaluates to `false` or raises an error.
3. If exactly one transition remains, fire it.
4. If multiple transitions remain, fire the one with the lowest `priority` value. If multiple transitions share the same lowest priority, the WOS Document is ill-formed and the WOS Processor MUST raise a diagnostic.
5. If no transitions remain, the event is discarded. This is not an error.

#### 6.4.2 Transition Execution Sequence

When a transition fires, the following sequence MUST be executed:

1. Execute the `onExit` actions of the source state, innermost first.
2. Execute the transition's `actions`.
3. Execute the `onEntry` actions of the target state, outermost first.
4. Emit a provenance record (§11) for the transition.

### 6.5 Actions

Actions are operations executed on state entry, state exit, or transition firing. An action is an object with the following structure:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `action` | enum | REQUIRED | The action type. |
| `description` | string | OPTIONAL | Human-readable explanation. |

The following action types are defined by this specification. Tier Specifications and extensions MAY define additional action types.

| Action Type | Description | Key Properties |
|-------------|-------------|----------------|
| `createTask` | Creates a human task instance. | `taskRef`: reference to a task definition (§8). |
| `invokeDecision` | Invokes a decision service. | `decisionRef`: reference to a decision service (§7). `outputBinding`: case file path for the result. |
| `setData` | Sets a value in the case file. | `path`: case file path. `value`: expression or literal. |
| `emitEvent` | Emits an event to the integration layer. | `eventType`: CloudEvents type. `data`: event payload expression. |
| `startTimer` | Starts a durable timer. | `timerId`: identifier. `duration` or `deadline`: temporal expression. `event`: event to raise on expiry. |
| `cancelTimer` | Cancels a running timer. | `timerId`: identifier. |
| `compensate` | Triggers compensation for a named scope. | `scope`: state or region identifier. |
| `log` | Writes an informational entry to the audit log. | `message`: expression producing a string. `level`: `info`, `warn`, `error`. |

### 6.6 Events

Events trigger transitions. This specification defines two categories of events:

**Internal events** are raised by the workflow engine itself:

| Event | Raised When |
|-------|------------|
| `task.completed` | A task reaches the Completed state. |
| `task.failed` | A task reaches the Failed state. |
| `task.escalated` | A task is escalated. |
| `timer.expired` | A durable timer expires. |
| `regions.allFinal` | All regions of a parallel state reach final states. |
| `error` | An unhandled error occurs. |
| `milestone.achieved` | A milestone condition becomes true. |

**External events** originate from outside the workflow instance and are delivered via the Integration layer (§10). External events are matched to workflow instances via correlation (§10.4).

Events MAY carry data accessible in guard expressions and actions via the `event.data` context variable.

### 6.7 Milestones

A milestone is a named, declarative checkpoint that is achieved when a condition on case data becomes true. Milestones do not direct control flow — they provide observable progress indicators and may be referenced in guard conditions.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `condition` | string (expression) | REQUIRED | A WOS Expression that, when it evaluates to `true`, marks the milestone as achieved. |
| `description` | string | OPTIONAL | Human-readable description of what this milestone signifies. |

A milestone, once achieved, remains achieved unless the condition subsequently evaluates to `false` (for example, if the supporting data is modified or withdrawn). Implementations MUST re-evaluate milestone conditions whenever referenced case data changes.

When a milestone transitions from unachieved to achieved, the internal event `milestone.achieved` is raised with the milestone's identifier. This event MAY trigger transitions.

### 6.8 Soundness Verification

This section is normative.

A WOS Document SHOULD be verifiable for the following soundness properties:

**Deadlock-freedom.** For every reachable non-final state, there exists at least one sequence of events that leads to a final state.

**Livelock-freedom.** No infinite cycle exists in which progress toward a final state is never made.

**Proper termination.** When the top-level final state is reached, no parallel region has active (non-final) states and no tasks remain in a non-terminal state.

**No dead elements.** Every state, transition, and task definition is reachable from the initial state under some combination of guard evaluations.

A WOS Processor that supports the Verification Conformance Profile (§18) MUST implement static analysis that checks these properties. The verification algorithm MAY be conservative (rejecting some valid definitions) but MUST NOT accept definitions that violate soundness.

> **Note (informative).** Full soundness verification is undecidable for statecharts with unbounded data-dependent guards. Conformant verifiers SHOULD support a decidable fragment (for example, treating data-dependent guards as nondeterministic) and MAY report inconclusive results for definitions outside the decidable fragment. Inconclusive verification MUST be reported as a warning, not as a pass.

---

## 7. Layer 2: Decision and Policy

This section is normative.

### 7.1 Overview

The Decision and Policy layer encapsulates all logic that evaluates conditions, determines routing, assesses eligibility, and applies rules. Decision logic is separated from process topology so that rules can be authored, versioned, tested, and audited independently of the lifecycle.

The primary abstraction is the **Decision Service**: an independently invocable unit of decision logic with defined inputs, defined outputs, and a versioned implementation.

### 7.2 Decision Services

A Decision Service is defined under the top-level `decisions` property:

```yaml
decisions:
  eligibilityDetermination:
    version: "1.2.0"
    description: "Determines applicant eligibility based on program criteria."

    inputs:
      - name: "applicantIncome"
        schema:
          type: "number"
      - name: "householdSize"
        schema:
          type: "integer"
      - name: "applicationDate"
        schema:
          type: "string"
          format: "date"

    outputs:
      - name: "eligible"
        schema:
          type: "boolean"
      - name: "reason"
        schema:
          type: "string"
      - name: "applicableThreshold"
        schema:
          type: "number"

    logic:
      type: "decisionTable"
      # ...

    effectiveDate: "2026-01-01"
    sunsetDate: "2026-12-31"
```

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `version` | string (semver) | REQUIRED | Independent version of this decision service. |
| `description` | string | OPTIONAL | Human-readable description. |
| `inputs` | array of InputDef | REQUIRED | Typed input parameters. |
| `outputs` | array of OutputDef | REQUIRED | Typed output parameters. |
| `logic` | DecisionLogic | REQUIRED | The decision implementation. |
| `effectiveDate` | string (date) | OPTIONAL | Date from which this version is effective. |
| `sunsetDate` | string (date) | OPTIONAL | Date after which this version is no longer effective. |

### 7.3 Decision Logic Types

The `logic` property specifies the implementation of a Decision Service. This specification defines the following logic types:

#### 7.3.1 Decision Tables

A decision table evaluates input values against a set of rules and produces outputs based on the first (or all) matching rules.

```yaml
logic:
  type: "decisionTable"
  hitPolicy: "first"
  rules:
    - when:
        applicantIncome: "<= parameters.incomeThreshold(applicationDate)"
        householdSize: ">= 1"
      then:
        eligible: true
        reason: "Meets income threshold for household size"
        applicableThreshold: "parameters.incomeThreshold(applicationDate)"

    - when:
        applicantIncome: "> parameters.incomeThreshold(applicationDate)"
      then:
        eligible: false
        reason: "Income exceeds threshold"
        applicableThreshold: "parameters.incomeThreshold(applicationDate)"
```

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `hitPolicy` | enum | REQUIRED | Determines how matching rules are handled. |
| `rules` | array of Rule | REQUIRED | Ordered list of rules. |

**Hit Policies.** The following hit policies are defined, consistent with DMN [DMN]:

| Policy | Meaning |
|--------|---------|
| `unique` | Exactly one rule matches. Multiple matches are an error. |
| `first` | The first matching rule (in document order) is used. |
| `priority` | All matching rules are collected; the one with the highest output priority is used. |
| `collect` | All matching rules fire; outputs are collected into an array. |
| `collectSum` | Numeric outputs of all matching rules are summed. |
| `collectMin` | The minimum numeric output is returned. |
| `collectMax` | The maximum numeric output is returned. |
| `collectCount` | The count of matching rules is returned. |

#### 7.3.2 Expression Logic

A single WOS Expression (§13) that computes the output directly.

```yaml
logic:
  type: "expression"
  expression: |
    {
      eligible: applicantIncome <= parameters.incomeThreshold(applicationDate),
      reason: if applicantIncome <= parameters.incomeThreshold(applicationDate)
              then "Meets threshold" else "Exceeds threshold"
    }
```

#### 7.3.3 Decision Requirement Graphs

A directed acyclic graph of sub-decisions where each node is itself a Decision Service. This enables composition of complex decisions from simpler, independently testable components.

```yaml
logic:
  type: "decisionGraph"
  nodes:
    incomeCheck:
      decisionRef: "incomeEligibility"
    programCheck:
      decisionRef: "programEligibility"
    finalDetermination:
      decisionRef: "combinedEligibility"
      dependsOn: ["incomeCheck", "programCheck"]
  outputNode: "finalDetermination"
```

#### 7.3.4 External Decision Reference

A reference to an externally hosted decision service, accessed via the Integration layer.

```yaml
logic:
  type: "external"
  integrationRef: "eligibilityService"
  operation: "evaluate"
```

### 7.4 Defeasible Rules

For regulatory and policy contexts, Decision Services MAY use defeasible rule logic — rules with structured exceptions where more specific provisions override general ones.

```yaml
logic:
  type: "defeasibleRules"
  rules:
    - id: "generalEligibility"
      description: "General eligibility: income below threshold"
      condition: "applicantIncome <= parameters.incomeThreshold(applicationDate)"
      conclusion:
        eligible: true

    - id: "veteranException"
      description: "Veterans receive a 20% higher threshold"
      overrides: "generalEligibility"
      condition: "applicant.veteranStatus = true"
      conclusion:
        eligible: true
        applicableThreshold: "parameters.incomeThreshold(applicationDate) * 1.2"

    - id: "disqualification"
      description: "Prior fraud disqualifies regardless"
      overrides: ["generalEligibility", "veteranException"]
      condition: "applicant.priorFraudFinding = true"
      conclusion:
        eligible: false
        reason: "Disqualified due to prior fraud finding"
```

The `overrides` property establishes a priority relationship. When multiple rules match, a rule that overrides another takes precedence. Override relationships MUST form a directed acyclic graph.

### 7.5 Temporal Parameters

Decision Services MAY reference temporal parameters — values that change on specific dates. This models the common regulatory pattern where thresholds, rates, and criteria are periodically updated.

```yaml
decisions:
  # ...

parameters:
  incomeThreshold:
    description: "Federal poverty level income threshold"
    type: "number"
    values:
      - effectiveDate: "2025-01-01"
        value: 31200
      - effectiveDate: "2026-01-01"
        value: 32760
      - effectiveDate: "2027-01-01"
        value: 34100
```

When a Decision Service references a temporal parameter, it MUST specify or receive the reference date. The parameter value effective on that date is used. If no value is effective on the reference date, the WOS Processor MUST raise an error.

### 7.6 Decision Invocation and Provenance

When a Decision Service is invoked (via an `invokeDecision` action or a guard expression), the WOS Processor MUST produce a Decision Record (§11.4) capturing:

1. The decision service identifier and version.
2. The complete input data at time of evaluation.
3. Which rules matched and which rule(s) produced the output.
4. The output values.
5. The timestamp and workflow context (instance ID, current state, triggering event).

---

## 8. Layer 3: Human Task Management

This section is normative.

### 8.1 Overview

The Human Task Management layer defines how work is assigned to, managed by, and completed by human participants. This layer treats human judgment, discretionary action, and exception handling as first-class concerns — not secondary add-ons to an automation-first model.

The primary abstraction is the **Task**: a unit of human work with a defined lifecycle, data contract, assignment model, and accountability structure.

### 8.2 Task Definitions

Task definitions are declared under the top-level `tasks` property:

```yaml
tasks:
  initialReview:
    description: "Review submitted application for completeness."
    version: "1.0.0"

    form:
      inputSchema:
        $ref: "#/caseFile/items/application/schema"
      outputSchema:
        type: "object"
        properties:
          isComplete:
            type: "boolean"
          missingItems:
            type: "array"
            items:
              type: "string"
          reviewerNotes:
            type: "string"
        required: ["isComplete"]

    assignment:
      potentialOwners:
        roles: ["intakeSpecialist"]
        skills: ["applicationReview"]
      businessAdministrators:
        roles: ["intakeSupervisor"]

    priority:
      expression: |
        if caseFile.application.expedited = true then 1 else 5

    sla:
      dueIn: "P2BD"
      warningAt: "P1BD"
      escalateOnBreach:
        to:
          roles: ["intakeSupervisor"]
        action: "reassign"

    separation:
      excludeFrom: ["finalApproval"]
      constraint: "sameInstance"
```

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `description` | string | OPTIONAL | Human-readable purpose of this task. |
| `version` | string (semver) | REQUIRED | Independent version of this task definition. |
| `form` | FormContract | REQUIRED | The data contract: input data provided to the worker and output data expected. |
| `assignment` | AssignmentModel | REQUIRED | Who may perform this task. |
| `priority` | PriorityDef | OPTIONAL | How priority is computed. |
| `sla` | SLADef | OPTIONAL | Service-level expectations and escalation. |
| `separation` | SeparationDef | OPTIONAL | Separation of duties constraints. |
| `metadata` | object | OPTIONAL | Descriptive metadata. |

### 8.3 Task Lifecycle

Every Task instance follows a defined lifecycle. The following states are normative:

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

| State | Description |
|-------|-------------|
| `Available` | Created and visible in the work queue of potential owners. No individual has taken responsibility. |
| `Claimed` | A specific individual has claimed the task but has not begun work. |
| `InProgress` | The claimant is actively working on the task. |
| `Completed` | The task is finished. Output data has been submitted. Terminal state. |
| `Failed` | The task could not be completed. Terminal state. |
| `Returned` | The task has been returned for rework with a revision request. Transitions to `Available` for reclaim. |
| `Escalated` | The task has been escalated to a higher authority due to SLA breach, exception, or manual escalation. |
| `Suspended` | The task is paused, typically because the parent workflow is suspended. |
| `Cancelled` | The task has been cancelled, typically because the workflow took an alternative path. Terminal state. |

Terminal states are `Completed`, `Failed`, and `Cancelled`. A task in a terminal state MUST NOT transition to any other state.

### 8.4 Task Operations

The following operations change a task's state. Each operation MUST produce a provenance record (§11).

| Operation | From State(s) | To State | Actor | Description |
|-----------|---------------|----------|-------|-------------|
| `create` | — | Available | System | Creates a new task instance. |
| `claim` | Available | Claimed | Potential Owner | An individual takes responsibility for the task. |
| `release` | Claimed | Available | Actual Owner | The claimant releases the task back to the queue. |
| `start` | Claimed | InProgress | Actual Owner | Work begins. |
| `complete` | InProgress | Completed | Actual Owner | Work is finished; output data submitted. |
| `fail` | InProgress | Failed | Actual Owner | Work cannot be completed; reason provided. |
| `delegate` | Claimed, InProgress | Claimed | Actual Owner | Task is transferred to a specific individual. |
| `forward` | Available, Claimed | Available | Actual Owner, Admin | Task is forwarded to a different group. |
| `returnForRework` | InProgress | Returned | Reviewer | Task output is rejected; revision required. |
| `escalate` | Any non-terminal | Escalated | System, Admin | Task is escalated due to SLA or exception. |
| `suspend` | Any non-terminal | Suspended | System, Admin | Task is paused. |
| `resume` | Suspended | (prior state) | System, Admin | Task resumes from the state it was in before suspension. |
| `cancel` | Any non-terminal | Cancelled | System, Admin | Task is cancelled. |

### 8.5 Assignment Model

The assignment model determines who may perform a task. It defines five role categories:

| Role | Description |
|------|-------------|
| `potentialOwners` | Individuals or groups who may claim and perform the task. |
| `excludedOwners` | Individuals explicitly barred from this task (for conflict of interest or separation of duties). |
| `businessAdministrators` | Individuals who may reassign, escalate, or cancel the task. |
| `taskStakeholders` | Individuals who may view task progress and data but not perform actions. |
| `notificationRecipients` | Individuals who receive notifications on task state changes. |

Each role is specified by a combination of:

- `roles`: Named roles (mapped to individuals by the runtime environment).
- `skills`: Required competencies (matched against worker profiles).
- `individuals`: Specific identified users.
- `expression`: A WOS Expression evaluated against case data (for dynamic assignment).

```yaml
assignment:
  potentialOwners:
    roles: ["seniorReviewer"]
    skills: ["financialAnalysis", "grantPrograms"]
  excludedOwners:
    expression: "caseFile.conflicts.excludedReviewers"
  businessAdministrators:
    roles: ["reviewSupervisor"]
```

### 8.6 Service Level Agreements

SLA definitions specify expected completion times, warning thresholds, and automatic escalation behavior.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `dueIn` | string (duration) | OPTIONAL | ISO 8601 duration or business duration (suffix `BD` for business days). |
| `dueBy` | string (expression) | OPTIONAL | Expression computing an absolute deadline. |
| `warningAt` | string (duration) | OPTIONAL | Duration before the deadline at which a warning is raised. |
| `escalateOnBreach` | EscalationDef | OPTIONAL | Automatic escalation action on SLA breach. |
| `businessCalendar` | string | OPTIONAL | Reference to a named business calendar for business-day computation. |

Business durations use the suffix `BD` (business days) or `BH` (business hours). When a `businessCalendar` is specified, the calendar defines which days and hours are considered working time. When no business calendar is specified, all days and hours are considered working time.

### 8.7 Separation of Duties

Separation of duties constraints prevent the same individual from performing specified task combinations within a workflow instance. This enforces principles such as the four-eyes rule and conflict-of-interest avoidance.

```yaml
separation:
  excludeFrom: ["finalApproval"]
  constraint: "sameInstance"
```

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `excludeFrom` | array of string | REQUIRED | Task definition identifiers from which this task's actual owner is excluded. |
| `constraint` | enum | REQUIRED | Scope of the exclusion: `sameInstance` (within the same workflow instance) or `global` (across all instances). |

A WOS Processor MUST enforce separation of duties constraints at claim time. If a potential owner is excluded by a separation constraint, the `claim` operation MUST be rejected and the reason MUST be recorded in the audit log.

### 8.8 Override Authority

In high-stakes workflows, authorized individuals may override system recommendations or normal processing rules. Overrides are modeled as a structured operation with mandatory accountability.

```yaml
tasks:
  supervisorOverride:
    description: "Supervisor override of automated eligibility determination."
    overrideTarget: "eligibilityDetermination"
    requiredAuthority:
      roles: ["programSupervisor"]
      minimumLevel: 3
    requiredFields:
      - name: "rationale"
        schema:
          type: "string"
          minLength: 50
        description: "Detailed justification for overriding the automated determination."
      - name: "supportingEvidence"
        schema:
          type: "array"
          items:
            $ref: "#/caseFile/evidenceRef"
        description: "Evidence supporting the override decision."
      - name: "overrideDecision"
        schema:
          $ref: "#/decisions/eligibilityDetermination/outputs"
```

The `overrideTarget` property references the Decision Service whose output is being overridden. When an override is executed, the WOS Processor MUST produce an Override Record (§11.5) that captures both the original automated result and the override values, rationale, authority, and evidence.

---

## 9. Layer 4: Case State and Evidence

This section is normative.

### 9.1 Overview

The Case State layer defines the typed data container — the Case File — that accumulates all business data, evidence, and computed values associated with a workflow instance. The Case File is the central artifact: the process exists to serve the case, not the other way around.

### 9.2 Case File Definition

The Case File is defined under the top-level `caseFile` property:

```yaml
caseFile:
  schema:
    type: "object"
    properties:
      application:
        $ref: "#/caseFile/items/application"
      eligibility:
        $ref: "#/caseFile/items/eligibility"
      award:
        $ref: "#/caseFile/items/award"
      correspondence:
        $ref: "#/caseFile/items/correspondence"

  items:
    application:
      schema:
        type: "object"
        properties:
          applicantName:
            type: "string"
          submittedDate:
            type: "string"
            format: "date"
          requestedAmount:
            type: "number"
          expedited:
            type: "boolean"
          documents:
            type: "array"
            items:
              $ref: "#/caseFile/evidenceSchema"
        required: ["applicantName", "submittedDate", "requestedAmount"]
      visibility:
        default: "restricted"
        overrides:
          - roles: ["applicant"]
            fields: ["applicantName", "submittedDate", "requestedAmount"]
            access: "read"
          - roles: ["reviewer", "supervisor"]
            access: "readWrite"
      multiplicity: "one"

    eligibility:
      schema:
        type: "object"
        properties:
          decision:
            type: "string"
            enum: ["eligible", "ineligible", "pendingReview"]
          determinedDate:
            type: "string"
            format: "date"
          determinedBy:
            type: "string"
          decisionRecordRef:
            type: "string"
            format: "uri"
      multiplicity: "one"

    correspondence:
      schema:
        type: "object"
        properties:
          date:
            type: "string"
            format: "date-time"
          direction:
            type: "string"
            enum: ["inbound", "outbound"]
          summary:
            type: "string"
          evidenceRef:
            $ref: "#/caseFile/evidenceSchema"
      multiplicity: "many"

  evidenceSchema:
    type: "object"
    properties:
      id:
        type: "string"
        format: "uri"
      contentType:
        type: "string"
      contentHash:
        type: "object"
        properties:
          algorithm:
            type: "string"
            enum: ["sha-256", "sha-384", "sha-512"]
          value:
            type: "string"
        required: ["algorithm", "value"]
      receivedDate:
        type: "string"
        format: "date-time"
      description:
        type: "string"
      claimCheckUri:
        type: "string"
        format: "uri"
    required: ["id", "contentType", "contentHash"]
```

### 9.3 Case File Items

Each Case File Item has the following properties:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `schema` | JSON Schema | REQUIRED | The data schema for this item. |
| `visibility` | VisibilityDef | OPTIONAL | Access control for this item's fields. |
| `multiplicity` | enum | REQUIRED | `one` (singleton) or `many` (collection). |
| `metadata` | object | OPTIONAL | Descriptive metadata. |

### 9.4 Data Mutation Semantics

Every mutation to the Case File MUST be recorded as an immutable event in the provenance stream. The provenance record for a data mutation MUST include:

1. The case file item and field path that changed.
2. The prior value (or absence of value).
3. The new value.
4. The actor (human user, system component, or Decision Service) that caused the change.
5. The triggering context (state, transition, task, or decision invocation).
6. The timestamp.

A WOS Processor MUST be capable of reconstructing the Case File state at any prior point in time by replaying the mutation events from the beginning through the desired timestamp.

### 9.5 Evidence Management

Evidence represents documents, datasets, images, and other artifacts attached to the case. Evidence items are referenced in Case File Items via the `evidenceSchema` structure.

Evidence MUST NOT be stored inline in the Case File. The Case File holds an evidence reference containing the content hash, content type, and a claim check URI pointing to the actual content. This follows the Claim Check integration pattern: the workflow carries a reference and integrity verification, not the payload.

Content hashing MUST use SHA-256 or stronger. A WOS Processor MUST verify content hash integrity when evidence is accessed and MUST record hash verification failures in the audit log.

### 9.6 Selective Visibility

Case File Items MAY define visibility rules that restrict which roles can view or modify which fields. Visibility is defined per-item and may be overridden per-field.

| Property | Type | Description |
|----------|------|-------------|
| `default` | enum | `open` (all roles may read), `restricted` (only explicitly authorized roles). |
| `overrides` | array of VisibilityOverride | Per-role access grants. |

Each `VisibilityOverride` specifies:

| Property | Type | Description |
|----------|------|-------------|
| `roles` | array of string | Roles to which this override applies. |
| `fields` | array of string | Specific fields (if omitted, applies to all fields). |
| `access` | enum | `read`, `readWrite`, `none`. |

A WOS Processor MUST enforce visibility rules when presenting case data to human task participants and when returning data via the query interface.

---

## 10. Layer 5: Integration and Eventing

This section is normative.

### 10.1 Overview

The Integration layer defines how a WOS workflow interacts with external systems: consuming events, producing events, invoking external services, and receiving callbacks. The layer is built on CloudEvents [CloudEvents] for event envelopes and OpenAPI [OpenAPI] / AsyncAPI [AsyncAPI] for interface contracts.

### 10.2 Integration Definitions

Integrations are declared under the top-level `integrations` property:

```yaml
integrations:
  backgroundCheckService:
    type: "request-response"
    interface:
      $ref: "https://api.example.gov/background-checks/openapi.yaml"
    operation: "submitCheck"
    timeout: "PT30M"
    retry:
      maxAttempts: 3
      backoff: "exponential"
      initialInterval: "PT10S"

  applicantNotification:
    type: "event-emit"
    eventType: "org.example.grants.notification"
    channel: "email"

  documentReceived:
    type: "event-consume"
    eventType: "org.example.documents.received"
    correlation:
      attribute: "subject"
      caseFileMapping: "caseFile.application.applicationId"
```

### 10.3 Integration Types

| Type | Description |
|------|-------------|
| `request-response` | Synchronous invocation of an external service. Interface defined by an OpenAPI reference. |
| `event-emit` | Production of an outbound event. |
| `event-consume` | Subscription to inbound events from external sources. |
| `callback` | A long-running external interaction: the workflow sends a request and later receives a callback event with the result. |

### 10.4 Correlation

Correlation is the mechanism by which an inbound external event is matched to the correct running workflow instance. A correlation definition specifies:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `attribute` | string | REQUIRED | The CloudEvents attribute or extension attribute containing the correlation value. |
| `caseFileMapping` | string (path) | REQUIRED | The Case File path whose value must match the correlation attribute. |

When an external event arrives, the WOS Processor MUST:

1. Extract the correlation attribute value from the event.
2. Find all running workflow instances whose mapped Case File value matches.
3. Deliver the event to the matched instance(s).
4. If no match is found, the event MUST be logged and MAY be queued for retry.

Multiple correlation attributes MAY be specified. When multiple attributes are defined, all MUST match (logical AND).

### 10.5 Event Envelope

All events produced by a WOS workflow MUST conform to the CloudEvents 1.0 specification [CloudEvents] with the following WOS extension attributes:

| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `wosinstanceid` | string (URI) | REQUIRED | The workflow instance identifier. |
| `wosdefid` | string (URI) | REQUIRED | The workflow definition identifier. |
| `wosdefversion` | string | REQUIRED | The workflow definition version. |
| `wosstate` | string | OPTIONAL | The current lifecycle state at the time of event emission. |
| `wostaskid` | string | OPTIONAL | The task identifier, if the event relates to a task. |
| `woscorrelationkey` | string | OPTIONAL | The primary business correlation key. |
| `woscausationeventid` | string | OPTIONAL | The `id` of the event that triggered this event. |

### 10.6 Idempotency

All event consumption MUST be idempotent. A WOS Processor MUST handle duplicate delivery of the same event (identified by the CloudEvents `id` attribute) without producing duplicate effects. The standard mechanism is to record processed event identifiers and ignore events that have already been processed.

---

## 11. Layer 6: Provenance and Audit

This section is normative.

### 11.1 Overview

The Provenance and Audit layer produces an immutable, tamper-evident record of everything that happens in a workflow instance. This layer is observational — it records the effects of all other layers but does not participate in control flow.

The provenance model is based on the W3C PROV data model [PROV-DM], extended with workflow-specific record types for decisions, overrides, and task operations.

### 11.2 Provenance Record Structure

Every provenance record MUST include the following fields:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string (URI) | REQUIRED | Globally unique identifier for this record. |
| `timestamp` | string (datetime) | REQUIRED | RFC 3339 timestamp with timezone. |
| `processId` | string (URI) | REQUIRED | The workflow instance this record belongs to. |
| `recordType` | enum | REQUIRED | The type of record (see §11.3–11.6). |
| `actor` | ActorRef | REQUIRED | The human user, system component, or Decision Service that caused the recorded action. |
| `authority` | string | OPTIONAL | The role, rule, or policy under which the actor operated. |
| `traceContext` | TraceContext | OPTIONAL | W3C Trace Context [TraceContext] for distributed tracing correlation. |
| `data` | object | REQUIRED | Record-type-specific payload. |

### 11.3 Transition Records

Produced when a lifecycle transition fires.

```yaml
recordType: "transition"
data:
  fromState: "intake"
  toState: "evaluation"
  event: "task.completed"
  guardExpression: "caseFile.application.isComplete = true"
  guardResult: true
  actionsExecuted:
    - action: "createTask"
      taskRef: "technicalReview"
    - action: "createTask"
      taskRef: "financialReview"
```

### 11.4 Decision Records

Produced when a Decision Service is invoked.

```yaml
recordType: "decision"
data:
  decisionRef: "eligibilityDetermination"
  decisionVersion: "1.2.0"
  inputs:
    applicantIncome: 28500
    householdSize: 3
    applicationDate: "2026-03-15"
  matchedRules: ["generalEligibility"]
  outputs:
    eligible: true
    reason: "Meets income threshold for household size"
    applicableThreshold: 32760
  evaluationDurationMs: 12
  parametersUsed:
    incomeThreshold:
      effectiveDate: "2026-01-01"
      value: 32760
```

### 11.5 Override Records

Produced when a human overrides an automated decision or system recommendation.

```yaml
recordType: "override"
data:
  overrideTarget: "eligibilityDetermination"
  originalResult:
    eligible: false
    reason: "Income exceeds threshold"
  overrideResult:
    eligible: true
    reason: "Hardship exception granted"
  rationale: "Applicant experienced sudden income loss due to
    documented medical emergency. Supporting documentation
    verified. Income at time of crisis was below threshold."
  supportingEvidence:
    - ref: "urn:evidence:medical-records-2026-03"
    - ref: "urn:evidence:employer-letter-2026-03"
  authorityLevel: 3
  overridingActor:
    id: "urn:user:jsmith"
    role: "programSupervisor"
```

### 11.6 Task Operation Records

Produced for every task state change.

```yaml
recordType: "taskOperation"
data:
  taskRef: "initialReview"
  taskInstanceId: "urn:task:abc123"
  operation: "complete"
  fromState: "InProgress"
  toState: "Completed"
  actor:
    id: "urn:user:mwilson"
    role: "intakeSpecialist"
  outputData:
    isComplete: true
    reviewerNotes: "All required documents present."
  durationMs: 1845000
```

### 11.7 Data Mutation Records

Produced for every change to the Case File.

```yaml
recordType: "dataMutation"
data:
  path: "caseFile.eligibility.decision"
  previousValue: null
  newValue: "eligible"
  cause:
    type: "decisionInvocation"
    ref: "eligibilityDetermination"
    decisionRecordId: "urn:provenance:dec-xyz789"
```

### 11.8 Tamper Evidence

A conformant WOS Processor at the Full Conformance Profile (§18) MUST provide tamper evidence for the provenance stream. The RECOMMENDED mechanism is Merkle tree hash-chaining:

1. Each provenance record is hashed using SHA-256.
2. Records are organized into a Merkle tree, with a new tree head produced at configurable intervals (RECOMMENDED: every 100 records or every 60 seconds, whichever comes first).
3. The tree head is signed by the WOS Processor.
4. An **inclusion proof** can be generated for any record, demonstrating that it is part of a specific tree head.
5. A **consistency proof** can be generated between any two tree heads, demonstrating that the later tree is an extension of the earlier tree (no records were removed or altered).

Implementations MAY use alternative tamper-evidence mechanisms provided they satisfy the same properties: append-only semantics, inclusion verifiability, and consistency verifiability.

---

## 12. Layer 7: Durable Execution Contract

This section is normative.

### 12.1 Overview

The Durable Execution Contract defines the abstract guarantees that a WOS Processor MUST provide for workflow execution resilience. This layer specifies what guarantees must hold, not how they are achieved. Implementations may use event-sourced replay, checkpointing, journaling, database-backed persistence, or any other mechanism that satisfies the requirements.

### 12.2 Durability Guarantees

A conformant WOS Processor at the Execution Conformance Profile (§18) or above MUST satisfy the following guarantees:

**G1: Crash Recovery.** If the WOS Processor fails and restarts, all workflow instances that were in a non-terminal state MUST resume from their last durable state. No committed state transitions, task operations, or case data mutations are lost.

**G2: Persistent State.** The current lifecycle state, case file contents, task states, and timer registrations of every workflow instance MUST be durably persisted. "Durable" means survivable across process restart, machine failure, and (in distributed deployments) single-node failure.

**G3: At-Least-Once Execution.** Every action associated with a state entry, exit, or transition MUST execute at least once. Actions SHOULD be designed for idempotent execution to ensure that at-least-once delivery produces correct results.

**G4: Durable Timers.** Timers registered by the workflow MUST survive process restarts. A timer MUST fire within a reasonable tolerance (implementation-defined, RECOMMENDED: 1 second) of its scheduled time, even if the process was not running at the scheduled time. Timers MUST consume no computational resources while waiting.

**G5: External Signal Delivery.** Events and signals sent to a workflow instance MUST be durably enqueued. If the instance is not currently active (for example, it is waiting on a timer or external callback), the signal MUST be delivered when the instance next becomes active.

### 12.3 Retry Policy

Retry policies may be defined at the workflow level, per-integration, or per-action.

```yaml
execution:
  defaultRetry:
    maxAttempts: 3
    backoff: "exponential"
    initialInterval: "PT1S"
    maxInterval: "PT5M"
    multiplier: 2.0
    nonRetryableErrors:
      - "ValidationError"
      - "AuthorizationDenied"
```

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `maxAttempts` | integer | REQUIRED | Maximum number of execution attempts (including the first). |
| `backoff` | enum | REQUIRED | `fixed`, `linear`, `exponential`. |
| `initialInterval` | string (duration) | REQUIRED | Wait time before the first retry. |
| `maxInterval` | string (duration) | OPTIONAL | Maximum wait time between retries. |
| `multiplier` | number | OPTIONAL | Multiplier for exponential/linear backoff. |
| `nonRetryableErrors` | array of string | OPTIONAL | Error types that should not be retried. |

### 12.4 Timeout Categories

The following timeout categories are defined:

| Category | Scope | Description |
|----------|-------|-------------|
| `stepTimeout` | Per-action | Maximum duration for a single action execution. |
| `taskTimeout` | Per-task | Maximum duration for a human task (from creation to completion). Typically the SLA. |
| `instanceTimeout` | Per-instance | Maximum total duration for a workflow instance. |
| `heartbeatTimeout` | Per-action | For long-running actions, the maximum interval between heartbeat signals. |
| `queueTimeout` | Per-task | Maximum time a task may remain in the Available state before escalation. |

### 12.5 Compensation Registration

Activities that produce externally visible side effects SHOULD register compensation handlers. A compensation handler is an action (or sequence of actions) that semantically reverses the effect of the original activity.

```yaml
lifecycle:
  states:
    fundsDisbursed:
      type: "atomic"
      onEntry:
        - action: "invokeIntegration"
          integrationRef: "paymentService"
          operation: "disburse"
          compensation:
            action: "invokeIntegration"
            integrationRef: "paymentService"
            operation: "reversePayment"
```

When a `compensate` action is executed for a scope, the WOS Processor MUST invoke the compensation handlers of all completed activities within that scope, in reverse order of their completion.

---

## 13. Expression Language

This section is normative.

### 13.1 Overview

WOS defines a profile of an expression language for use in guard conditions, data transformations, computed values, and dynamic assignments. The expression language is based on a subset of FEEL (Friendly Enough Expression Language) as defined in DMN [DMN], chosen for its balance of readability by non-developers and executability by machines.

### 13.2 WOS Expression Language Profile

The WOS Expression Language supports the following constructs:

**Literals.** Strings (`"hello"`), numbers (`42`, `3.14`), booleans (`true`, `false`), null (`null`), dates (`date("2026-04-08")`), date-times (`dateTime("2026-04-08T12:00:00Z")`), durations (`duration("P2D")`).

**Paths.** Dot-separated property access (`caseFile.application.requestedAmount`). Array access (`caseFile.correspondence[0].summary`).

**Arithmetic.** `+`, `-`, `*`, `/`, `%` with standard precedence.

**Comparison.** `=`, `!=`, `<`, `<=`, `>`, `>=`.

**Boolean.** `and`, `or`, `not`.

**Conditionals.** `if <condition> then <value> else <value>`.

**In (membership).** `x in [1, 2, 3]`, `x in (10..20]`.

**String operations.** `contains(s, sub)`, `starts with(s, prefix)`, `ends with(s, suffix)`, `upper case(s)`, `lower case(s)`, `string length(s)`.

**List operations.** `count(list)`, `sum(list)`, `min(list)`, `max(list)`, `all(list)`, `any(list)`, `append(list, item)`, `flatten(list)`.

**Date/time operations.** `now()`, `today()`, `year(d)`, `month(d)`, `day(d)`.

**Context construction.** `{ key1: value1, key2: value2 }`.

**Null safety.** Expressions MUST NOT throw errors on null values. Accessing a property of null produces null. Comparing null with any value (except null) produces false.

### 13.3 Expression Context

Expressions are evaluated against a context containing:

| Variable | Description |
|----------|-------------|
| `caseFile` | The current Case File data. |
| `event` | The triggering event's data (only available in transition guards and actions). |
| `task` | The current task's data (only available in task-related expressions). |
| `instance` | Metadata about the workflow instance (id, creation time, current state). |
| `parameters` | Temporal parameters defined in the Decision layer. |
| `env` | Implementation-defined environment variables. |

### 13.4 Expression Safety

WOS Expressions MUST be pure functions: they MUST NOT produce side effects, modify state, perform I/O, or access resources outside the expression context. Implementations MUST enforce this constraint.

WOS Expressions MUST terminate. The expression language does not include general recursion or unbounded loops. Every expression MUST evaluate in bounded time.

---

## 14. Versioning and Evolution

This section is normative.

### 14.1 Workflow Definition Versioning

Workflow Definitions are versioned using Semantic Versioning [SemVer]:

- **Major** version changes indicate breaking changes: structural modifications to the lifecycle that affect running instances, removed states or transitions, incompatible case file schema changes.
- **Minor** version changes indicate backward-compatible additions: new states, new decision services, new task definitions, additive case file schema changes.
- **Patch** version changes indicate corrections: typos in descriptions, metadata updates, guard expression corrections that do not alter behavior.

### 14.2 Instance Migration

When a Workflow Definition is updated, running instances created under a prior version are not automatically affected. A WOS Processor MUST support at least one of the following strategies:

1. **Pinned execution.** Running instances continue under the version that created them. New instances use the new version. This is the default.
2. **Forward migration.** Running instances are migrated to the new version. The WOS Processor MUST verify that the migration is safe (the instance's current state exists in the new version, case file schema changes are backward-compatible) before applying it. Migration MUST produce a provenance record.

### 14.3 Decision Service Versioning

Decision Services are independently versioned. When a Decision Service version changes, the workflow definition MAY be updated to reference the new version or MAY be configured to always use the latest active version. Decision Records (§11.4) always record the specific version that was evaluated.

### 14.4 Schema Evolution

Case File schemas evolve over the lifecycle of a workflow program. The following schema changes are considered backward-compatible:

- Adding a new optional property.
- Widening a type constraint (e.g., increasing a `maxLength`).
- Adding a new enum value.
- Adding a new Case File Item.

The following changes are considered breaking:

- Removing a property.
- Adding a required property without a default.
- Narrowing a type constraint.
- Renaming a property.
- Changing a property's type.

Breaking schema changes MUST increment the major version of the Workflow Definition.

---

## 15. Security and Access Control

This section is normative.

### 15.1 Access Control Model

WOS defines a layered access control model combining role-based and attribute-based access control:

**Roles** are named sets of permissions assigned to individuals or groups. The following structural roles are defined by this specification:

| Role | Scope | Description |
|------|-------|-------------|
| `workflowAdministrator` | Workflow Definition | May create, modify, and retire workflow definitions. |
| `instanceInitiator` | Workflow Instance | May create new workflow instances. |
| `caseParticipant` | Workflow Instance | May view case data and task status (subject to visibility rules). |
| `taskWorker` | Task Instance | May claim and perform assigned tasks. |
| `taskAdministrator` | Task Instance | May reassign, escalate, or cancel tasks. |
| `auditor` | Workflow Instance | May view all provenance records and case data. |

**Attribute-based constraints** further restrict access based on contextual attributes such as jurisdiction, clearance level, organizational unit, and case-specific properties (e.g., conflict of interest flags).

**Relationship-based constraints** restrict access based on the relationship between the accessor and the case (e.g., the assigned reviewer, the applicant, the supervising organization).

### 15.2 Authorization Enforcement

Authorization checks MUST be enforced at the following points:

1. Workflow instance creation.
2. Task claim, delegation, and other operations.
3. Case File data access (read and write).
4. Decision Service invocation.
5. Provenance record access.
6. Override execution.

Authorization failures MUST be logged in the provenance stream with the denied action, the requesting actor, and the reason for denial.

---

## 16. Extensibility

This section is normative.

### 16.1 Extension Mechanism

WOS supports extensions via namespaced properties throughout the document. Extension properties MUST use a namespace prefix followed by a colon:

```yaml
metadata:
  x-agency:classification: "CUI"
  x-agency:programCode: "93.558"

lifecycle:
  states:
    review:
      type: "atomic"
      x-analytics:expectedDuration: "P3D"
      x-analytics:bottleneckRisk: "high"
```

### 16.2 Extension Rules

1. Extension properties MUST NOT alter the semantics of core WOS properties.
2. A WOS Processor MUST preserve extension properties during serialization round-trips (it MUST NOT strip unrecognized properties).
3. A WOS Processor MUST NOT reject a document solely because it contains extension properties.
4. Extension properties MUST NOT use the `wos:` prefix, which is reserved for future specification use.
5. Tier Specifications MAY promote extension patterns to core properties in future versions.

---

## 17. Serialization

This section is normative.

### 17.1 Canonical Formats

WOS Documents MAY be serialized in YAML 1.2 [YAML] or JSON [RFC8259]. YAML is the RECOMMENDED format for human authoring. JSON is the RECOMMENDED format for machine interchange.

A conformant WOS Document serialized in YAML MUST be losslessly convertible to JSON and vice versa. YAML-specific features that have no JSON equivalent (anchors, aliases, tags, comments) MAY be used for authoring convenience but are not part of the semantic model. A WOS Processor MUST interpret YAML documents by first resolving them to their JSON equivalent.

### 17.2 Character Encoding

WOS Documents MUST be encoded in UTF-8.

### 17.3 Identifiers

All object identifiers within a WOS Document MUST conform to the pattern `[a-zA-Z][a-zA-Z0-9_-]*`. Identifiers are case-sensitive. Identifiers MUST be unique within their containing scope (for example, state identifiers must be unique within their parent compound or parallel region).

Cross-document references MUST use URIs [RFC3986].

### 17.4 Timestamps

All timestamps MUST conform to RFC 3339 [RFC3339] and MUST include a timezone designator. The UTC timezone (designated by `Z`) is RECOMMENDED for provenance records.

### 17.5 Durations

Durations MUST conform to ISO 8601 [ISO8601] duration format. Business durations use the non-standard but defined suffixes `BD` (business days) and `BH` (business hours), which MUST be interpreted in the context of the applicable business calendar.

---

## 18. Conformance Profiles

This section is normative.

### 18.1 Overview

WOS defines conformance profiles that allow implementations to support subsets of the specification. An implementation MUST declare which profile(s) it supports.

### 18.2 Profile: Structural

A Structural conformant implementation:

1. MUST parse and validate WOS Documents against the JSON Schema.
2. MUST produce diagnostic messages for schema violations.
3. MUST support round-trip serialization between YAML and JSON.
4. MUST preserve extension properties.

This profile enables editors, validators, linters, and migration tools without requiring execution capabilities.

### 18.3 Profile: Lifecycle

A Lifecycle conformant implementation satisfies Structural conformance and additionally:

1. MUST execute lifecycle semantics: state entry/exit, transition firing with guard evaluation, parallel region synchronization, and history state restoration.
2. MUST produce Transition Records (§11.3) for every transition.
3. MUST support all state types (atomic, compound, parallel, final).
4. MUST correctly implement transition resolution (§6.4.1) and execution sequence (§6.4.2).
5. MUST support milestones.

### 18.4 Profile: Task Management

A Task Management conformant implementation satisfies Lifecycle conformance and additionally:

1. MUST implement the full task lifecycle (§8.3).
2. MUST support all task operations (§8.4).
3. MUST enforce separation of duties constraints (§8.7).
4. MUST support SLA timers and escalation.
5. MUST produce Task Operation Records (§11.6).

### 18.5 Profile: Decision

A Decision conformant implementation satisfies Lifecycle conformance and additionally:

1. MUST evaluate decision tables with all defined hit policies.
2. MUST support the WOS Expression Language profile (§13).
3. MUST support temporal parameters (§7.5).
4. MUST produce Decision Records (§11.4).

### 18.6 Profile: Full

A Full conformant implementation satisfies all of Lifecycle, Task Management, and Decision conformance, and additionally:

1. MUST implement case file management with data mutation tracking (§9.4).
2. MUST implement integration semantics including event correlation (§10.4).
3. MUST produce all provenance record types (§11.3–11.7).
4. MUST implement tamper evidence for the provenance stream (§11.8).
5. MUST satisfy all durable execution guarantees (§12.2).
6. MUST enforce access control (§15).

### 18.7 Profile: Verification

A Verification conformant implementation satisfies Structural conformance and additionally:

1. MUST perform static soundness analysis (§6.8) including deadlock-freedom, livelock-freedom, proper termination, and dead element detection.
2. MUST produce diagnostic reports identifying specific violations.
3. SHOULD support simulation (execution of workflow definitions against synthetic inputs).

---

## 19. Privacy Considerations

This section is informative.

WOS workflow definitions and instances may process personal data subject to privacy regulations such as GDPR, CCPA, HIPAA, and the Privacy Act. Implementations SHOULD consider the following:

1. Case File Items containing personal data SHOULD be identified and handled in accordance with applicable privacy regulations. The `visibility` model (§9.6) provides a mechanism for restricting access.

2. Provenance records may contain personal data (actor identifiers, case data snapshots in Decision Records). Implementations SHOULD support configurable retention periods for provenance data and MAY support anonymization of provenance records after a defined period.

3. Evidence containing personal data should be managed in accordance with data minimization principles. The Claim Check pattern (§9.5) helps by keeping actual content in dedicated, access-controlled storage rather than in the workflow engine.

4. The right to erasure may conflict with audit and provenance requirements. Implementations SHOULD provide a mechanism for marking personal data as redacted while preserving the structural integrity of the audit trail (recording that data existed and was redacted, without retaining the data itself).

---

## 20. Security Considerations

This section is informative.

1. **Expression injection.** WOS Expressions (§13) MUST be evaluated in a sandboxed context with no access to system resources, network, or filesystem. Implementations MUST NOT allow expression evaluation to produce side effects.

2. **Event spoofing.** External events consumed by a workflow instance should be authenticated. Implementations SHOULD verify event signatures using a mechanism such as Standard Webhooks [StandardWebhooks] signing.

3. **Provenance integrity.** The tamper-evidence mechanism (§11.8) protects against post-hoc alteration of audit records. Implementations SHOULD store signed tree heads in an independent system from the provenance records themselves.

4. **Case File confidentiality.** Data at rest (Case File contents, provenance records, evidence) and data in transit (events, integration calls) SHOULD be encrypted. The visibility model (§9.6) enforces access control but does not substitute for encryption.

5. **Separation of duties bypass.** Implementations MUST ensure that separation of duties constraints (§8.7) cannot be bypassed by administrative interfaces or direct database access. Constraint enforcement SHOULD be implemented at the application layer, not solely at the UI layer.

---

## 21. References

### 21.1 Normative References

**[RFC2119]** Bradner, S., "Key words for use in RFCs to Indicate Requirement Levels", BCP 14, RFC 2119, March 1997.

**[RFC3339]** Klyne, G. and C. Newman, "Date and Time on the Internet: Timestamps", RFC 3339, July 2002.

**[RFC3986]** Berners-Lee, T., Fielding, R., and L. Masinter, "Uniform Resource Identifier (URI): Generic Syntax", RFC 3986, January 2005.

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

### 21.2 Informative References

**[Harel1987]** Harel, D., "Statecharts: A Visual Formalism for Complex Systems", Science of Computer Programming, 8(3), pp. 231–274, 1987.

**[SCXML]** W3C, "State Chart XML (SCXML): State Machine Notation for Control Abstraction", W3C Recommendation, September 2015.

**[BPMN]** OMG, "Business Process Model and Notation Version 2.0", ISO/IEC 19510:2013.

**[CMMN]** OMG, "Case Management Model and Notation Version 1.1", December 2016.

**[WS-HumanTask]** OASIS, "Web Services – Human Task Version 1.1", August 2012.

**[XACML]** OASIS, "eXtensible Access Control Markup Language Version 3.0", January 2013.

**[Sagas]** Garcia-Molina, H. and Salem, K., "Sagas", Proceedings of the 1987 ACM SIGMOD International Conference on Management of Data, 1987.

**[WorkflowPatterns]** van der Aalst, W.M.P., ter Hofstede, A.H.M., Kiepuszewski, B., and Barros, A.P., "Workflow Patterns", Distributed and Parallel Databases, 14(1), pp. 5–51, 2003.

**[StandardWebhooks]** Standard Webhooks Project, "Standard Webhooks Specification", 2024.

**[RFC9162]** Laurie, B., Langley, A., Kasper, E., Messeri, E., and Stradling, R., "Certificate Transparency Version 2.0", RFC 9162, December 2021.

**[NIST-SP-800-53]** NIST, "Security and Privacy Controls for Information Systems and Organizations", NIST Special Publication 800-53 Revision 5, September 2020.

**[XES]** IEEE, "IEEE Standard for eXtensible Event Stream (XES) for Achieving Interoperability in Event Logs and Event Streams", IEEE Std 1849-2016.

---

## Appendix A. JSON Schema for WOS Core

This appendix is normative. The complete JSON Schema for WOS Core is maintained at the specification repository. The following is an abbreviated structural schema showing the top-level document shape:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://wos-spec.org/schema/core/0.1.0",
  "title": "WOS Core Document",
  "description": "A Workflow Orchestration Standard core document.",
  "type": "object",
  "required": ["wos", "id", "name", "version", "status", "lifecycle"],
  "properties": {
    "wos": {
      "type": "string",
      "pattern": "^[0-9]+\\.[0-9]+\\.[0-9]+$",
      "description": "WOS specification version."
    },
    "id": {
      "type": "string",
      "format": "uri",
      "description": "Unique identifier for this workflow definition."
    },
    "name": {
      "type": "string",
      "minLength": 1,
      "description": "Human-readable name."
    },
    "version": {
      "type": "string",
      "pattern": "^[0-9]+\\.[0-9]+\\.[0-9]+$",
      "description": "Workflow definition version (SemVer)."
    },
    "status": {
      "type": "string",
      "enum": ["draft", "active", "deprecated", "retired"]
    },
    "metadata": {
      "type": "object",
      "properties": {
        "description": { "type": "string" },
        "authors": { "type": "array", "items": { "type": "string" } },
        "created": { "type": "string", "format": "date-time" },
        "modified": { "type": "string", "format": "date-time" },
        "tags": { "type": "array", "items": { "type": "string" } },
        "jurisdiction": { "type": "string" },
        "authority": { "type": "string" },
        "effectiveDate": { "type": "string", "format": "date" },
        "sunsetDate": { "type": "string", "format": "date" }
      },
      "additionalProperties": true
    },
    "lifecycle": { "$ref": "#/$defs/Lifecycle" },
    "decisions": {
      "type": "object",
      "additionalProperties": { "$ref": "#/$defs/DecisionService" }
    },
    "tasks": {
      "type": "object",
      "additionalProperties": { "$ref": "#/$defs/TaskDefinition" }
    },
    "caseFile": { "$ref": "#/$defs/CaseFile" },
    "integrations": {
      "type": "object",
      "additionalProperties": { "$ref": "#/$defs/Integration" }
    },
    "provenance": { "$ref": "#/$defs/ProvenanceConfig" },
    "execution": { "$ref": "#/$defs/ExecutionConfig" },
    "extensions": {
      "type": "object",
      "additionalProperties": true
    }
  },
  "additionalProperties": true,
  "$defs": {
    "Lifecycle": {
      "type": "object",
      "required": ["initialState", "states"],
      "properties": {
        "initialState": { "type": "string" },
        "states": {
          "type": "object",
          "additionalProperties": { "$ref": "#/$defs/State" }
        },
        "milestones": {
          "type": "object",
          "additionalProperties": { "$ref": "#/$defs/Milestone" }
        }
      }
    },
    "State": {
      "type": "object",
      "required": ["type"],
      "properties": {
        "type": {
          "type": "string",
          "enum": ["atomic", "compound", "parallel", "final"]
        },
        "initialState": { "type": "string" },
        "states": {
          "type": "object",
          "additionalProperties": { "$ref": "#/$defs/State" }
        },
        "regions": {
          "type": "object",
          "additionalProperties": { "$ref": "#/$defs/Region" }
        },
        "historyState": {
          "type": "string",
          "enum": ["shallow", "deep"]
        },
        "onEntry": {
          "type": "array",
          "items": { "$ref": "#/$defs/Action" }
        },
        "onExit": {
          "type": "array",
          "items": { "$ref": "#/$defs/Action" }
        },
        "transitions": {
          "type": "array",
          "items": { "$ref": "#/$defs/Transition" }
        },
        "metadata": { "type": "object" }
      }
    },
    "Region": {
      "type": "object",
      "required": ["initialState", "states"],
      "properties": {
        "initialState": { "type": "string" },
        "states": {
          "type": "object",
          "additionalProperties": { "$ref": "#/$defs/State" }
        }
      }
    },
    "Transition": {
      "type": "object",
      "required": ["event", "target"],
      "properties": {
        "event": { "type": "string" },
        "target": { "type": "string" },
        "guard": { "type": "string" },
        "actions": {
          "type": "array",
          "items": { "$ref": "#/$defs/Action" }
        },
        "priority": { "type": "integer", "default": 0 },
        "description": { "type": "string" }
      }
    },
    "Action": {
      "type": "object",
      "required": ["action"],
      "properties": {
        "action": { "type": "string" },
        "description": { "type": "string" }
      },
      "additionalProperties": true
    },
    "Milestone": {
      "type": "object",
      "required": ["condition"],
      "properties": {
        "condition": { "type": "string" },
        "description": { "type": "string" }
      }
    },
    "DecisionService": {
      "type": "object",
      "required": ["version", "inputs", "outputs", "logic"],
      "properties": {
        "version": { "type": "string" },
        "description": { "type": "string" },
        "inputs": { "type": "array" },
        "outputs": { "type": "array" },
        "logic": { "type": "object" },
        "effectiveDate": { "type": "string", "format": "date" },
        "sunsetDate": { "type": "string", "format": "date" }
      }
    },
    "TaskDefinition": {
      "type": "object",
      "required": ["version", "form", "assignment"],
      "properties": {
        "version": { "type": "string" },
        "description": { "type": "string" },
        "form": { "type": "object" },
        "assignment": { "type": "object" },
        "priority": { "type": "object" },
        "sla": { "type": "object" },
        "separation": { "type": "object" },
        "metadata": { "type": "object" }
      }
    },
    "CaseFile": {
      "type": "object",
      "properties": {
        "schema": { "type": "object" },
        "items": { "type": "object" },
        "evidenceSchema": { "type": "object" }
      }
    },
    "Integration": {
      "type": "object",
      "required": ["type"],
      "properties": {
        "type": {
          "type": "string",
          "enum": ["request-response", "event-emit", "event-consume", "callback"]
        }
      },
      "additionalProperties": true
    },
    "ProvenanceConfig": {
      "type": "object",
      "properties": {
        "tamperEvidence": {
          "type": "object",
          "properties": {
            "enabled": { "type": "boolean" },
            "algorithm": { "type": "string" },
            "treeHeadInterval": { "type": "integer" }
          }
        },
        "retentionPeriod": { "type": "string" }
      }
    },
    "ExecutionConfig": {
      "type": "object",
      "properties": {
        "defaultRetry": { "$ref": "#/$defs/RetryPolicy" }
      }
    },
    "RetryPolicy": {
      "type": "object",
      "required": ["maxAttempts", "backoff", "initialInterval"],
      "properties": {
        "maxAttempts": { "type": "integer", "minimum": 1 },
        "backoff": {
          "type": "string",
          "enum": ["fixed", "linear", "exponential"]
        },
        "initialInterval": { "type": "string" },
        "maxInterval": { "type": "string" },
        "multiplier": { "type": "number" },
        "nonRetryableErrors": {
          "type": "array",
          "items": { "type": "string" }
        }
      }
    }
  }
}
```

---

## Appendix B. Complete Example

This appendix is informative. The following is a complete WOS Document for a simplified grant application review workflow.

```yaml
wos: "0.1.0"
id: "urn:wos:grants.example.gov:community-development-grant"
name: "Community Development Block Grant Review"
version: "1.0.0"
status: "active"

metadata:
  description: >
    Review and adjudication workflow for Community Development
    Block Grant applications under 24 CFR Part 570.
  authors: ["grants-program-office@example.gov"]
  created: "2026-04-01T00:00:00Z"
  modified: "2026-04-08T00:00:00Z"
  jurisdiction: "US-Federal"
  authority: "24 CFR Part 570"
  effectiveDate: "2026-04-15"
  tags: ["grants", "CDBG", "HUD"]

# ── Layer 1: Lifecycle ────────────────────────────────────

lifecycle:
  initialState: "submitted"

  states:
    submitted:
      type: "atomic"
      onEntry:
        - action: "createTask"
          taskRef: "completenessCheck"
        - action: "startTimer"
          timerId: "submissionSLA"
          duration: "P5BD"
          event: "timer.expired"
      transitions:
        - event: "task.completed"
          target: "eligibilityReview"
          guard: "caseFile.intake.isComplete = true"
        - event: "task.completed"
          target: "returnedToApplicant"
          guard: "caseFile.intake.isComplete = false"
        - event: "timer.expired"
          target: "submitted"
          actions:
            - action: "emitEvent"
              eventType: "org.example.grants.sla-warning"
              data: "{ processId: instance.id, stage: 'submitted' }"

    returnedToApplicant:
      type: "atomic"
      onEntry:
        - action: "emitEvent"
          eventType: "org.example.grants.returned"
          data: "{ reason: caseFile.intake.missingItems }"
        - action: "startTimer"
          timerId: "resubmissionDeadline"
          duration: "P30D"
          event: "timer.expired"
      transitions:
        - event: "org.example.documents.resubmitted"
          target: "submitted"
        - event: "timer.expired"
          target: "closed"
          actions:
            - action: "setData"
              path: "caseFile.outcome.status"
              value: "'closedIncomplete'"

    eligibilityReview:
      type: "atomic"
      onEntry:
        - action: "invokeDecision"
          decisionRef: "eligibilityDetermination"
          outputBinding: "caseFile.eligibility"
      transitions:
        - event: "decision.complete"
          target: "technicalReview"
          guard: "caseFile.eligibility.eligible = true"
        - event: "decision.complete"
          target: "ineligibleNotification"
          guard: "caseFile.eligibility.eligible = false"

    ineligibleNotification:
      type: "atomic"
      onEntry:
        - action: "createTask"
          taskRef: "notifyIneligible"
      transitions:
        - event: "task.completed"
          target: "appealWindow"

    appealWindow:
      type: "atomic"
      onEntry:
        - action: "startTimer"
          timerId: "appealDeadline"
          duration: "P30D"
          event: "timer.expired"
      transitions:
        - event: "org.example.grants.appeal-filed"
          target: "eligibilityReview"
          actions:
            - action: "cancelTimer"
              timerId: "appealDeadline"
            - action: "setData"
              path: "caseFile.eligibility.decision"
              value: "'pendingReview'"
        - event: "timer.expired"
          target: "closed"
          actions:
            - action: "setData"
              path: "caseFile.outcome.status"
              value: "'ineligible'"

    technicalReview:
      type: "parallel"
      regions:
        programReview:
          initialState: "programPending"
          states:
            programPending:
              type: "atomic"
              onEntry:
                - action: "createTask"
                  taskRef: "programReview"
              transitions:
                - event: "task.completed"
                  target: "programDone"
            programDone:
              type: "final"
        financialReview:
          initialState: "financialPending"
          states:
            financialPending:
              type: "atomic"
              onEntry:
                - action: "createTask"
                  taskRef: "financialReview"
              transitions:
                - event: "task.completed"
                  target: "financialDone"
            financialDone:
              type: "final"
        environmentalReview:
          initialState: "envPending"
          states:
            envPending:
              type: "atomic"
              onEntry:
                - action: "createTask"
                  taskRef: "environmentalReview"
              transitions:
                - event: "task.completed"
                  target: "envDone"
            envDone:
              type: "final"
      transitions:
        - event: "regions.allFinal"
          target: "adjudication"

    adjudication:
      type: "compound"
      initialState: "recommendation"
      historyState: "shallow"
      states:
        recommendation:
          type: "atomic"
          onEntry:
            - action: "createTask"
              taskRef: "prepareRecommendation"
          transitions:
            - event: "task.completed"
              target: "supervisorDecision"
        supervisorDecision:
          type: "atomic"
          onEntry:
            - action: "createTask"
              taskRef: "finalApproval"
          transitions:
            - event: "task.completed"
              target: "decided"
              guard: >
                caseFile.adjudication.decision = 'approved'
                or caseFile.adjudication.decision = 'denied'
            - event: "task.completed"
              target: "recommendation"
              guard: "caseFile.adjudication.decision = 'returnForRevision'"
        decided:
          type: "final"
      transitions:
        - event: "adjudication.complete"
          target: "notification"

    notification:
      type: "atomic"
      onEntry:
        - action: "createTask"
          taskRef: "issueDecisionLetter"
      transitions:
        - event: "task.completed"
          target: "closed"

    closed:
      type: "final"

  milestones:
    applicationReceived:
      condition: "caseFile.intake != null"
      description: "Application has been received and logged."
    completenessConfirmed:
      condition: "caseFile.intake.isComplete = true"
      description: "Application confirmed as complete."
    eligibilityDetermined:
      condition: "caseFile.eligibility.decision != null"
      description: "Eligibility determination rendered."
    reviewsComplete:
      condition: >
        caseFile.reviews.program.complete = true
        and caseFile.reviews.financial.complete = true
        and caseFile.reviews.environmental.complete = true
      description: "All technical reviews are complete."
    decisionRendered:
      condition: >
        caseFile.adjudication.decision = 'approved'
        or caseFile.adjudication.decision = 'denied'
      description: "Final funding decision has been made."

# ── Layer 2: Decisions ────────────────────────────────────

decisions:
  eligibilityDetermination:
    version: "2.1.0"
    description: >
      Determines whether an applicant meets basic eligibility
      criteria for CDBG funding under 24 CFR 570.200.
    inputs:
      - name: "populationServed"
        schema:
          type: "integer"
      - name: "areaMedianIncome"
        schema:
          type: "number"
      - name: "proposedActivities"
        schema:
          type: "array"
          items:
            type: "string"
      - name: "applicationDate"
        schema:
          type: "string"
          format: "date"
    outputs:
      - name: "eligible"
        schema:
          type: "boolean"
      - name: "reason"
        schema:
          type: "string"
    logic:
      type: "defeasibleRules"
      rules:
        - id: "baseEligibility"
          description: "Base: meets national objective and eligible activity"
          condition: >
            populationServed > 0
            and some activity in proposedActivities
                satisfies activity in parameters.eligibleActivities(applicationDate)
          conclusion:
            eligible: true
            reason: "Meets national objective and proposes eligible activities."

        - id: "incomeTargeting"
          description: "At least 51% low-moderate income benefit"
          overrides: "baseEligibility"
          condition: >
            areaMedianIncome > parameters.amiThreshold(applicationDate)
          conclusion:
            eligible: false
            reason: "Area median income exceeds low-moderate threshold."

        - id: "exemptActivity"
          description: "Certain activities exempt from income targeting"
          overrides: "incomeTargeting"
          condition: >
            some activity in proposedActivities
                satisfies activity in parameters.exemptActivities(applicationDate)
          conclusion:
            eligible: true
            reason: "Proposes exempt activity; income targeting waived."

parameters:
  amiThreshold:
    description: "Area median income threshold for low-mod targeting"
    type: "number"
    values:
      - effectiveDate: "2025-01-01"
        value: 78000
      - effectiveDate: "2026-01-01"
        value: 82500
  eligibleActivities:
    description: "CDBG eligible activity types"
    type: "array"
    values:
      - effectiveDate: "2025-01-01"
        value:
          - "publicFacilities"
          - "housingRehabilitation"
          - "economicDevelopment"
          - "publicServices"
          - "planning"
  exemptActivities:
    description: "Activities exempt from income targeting"
    type: "array"
    values:
      - effectiveDate: "2025-01-01"
        value:
          - "urgentNeed"
          - "slumBlightElimination"

# ── Layer 3: Tasks ────────────────────────────────────────

tasks:
  completenessCheck:
    version: "1.0.0"
    description: "Review application package for completeness."
    form:
      inputSchema:
        $ref: "#/caseFile/items/application/schema"
      outputSchema:
        type: "object"
        properties:
          isComplete:
            type: "boolean"
          missingItems:
            type: "array"
            items:
              type: "string"
          notes:
            type: "string"
        required: ["isComplete"]
    assignment:
      potentialOwners:
        roles: ["intakeSpecialist"]
      businessAdministrators:
        roles: ["intakeSupervisor"]
    priority:
      expression: "if caseFile.application.expedited = true then 1 else 5"
    sla:
      dueIn: "P2BD"
      warningAt: "P1BD"
      businessCalendar: "federalWorkdays"
      escalateOnBreach:
        to:
          roles: ["intakeSupervisor"]
        action: "reassign"

  programReview:
    version: "1.0.0"
    description: "Evaluate program design and national objective alignment."
    form:
      inputSchema:
        type: "object"
        properties:
          application:
            $ref: "#/caseFile/items/application/schema"
          eligibility:
            $ref: "#/caseFile/items/eligibility/schema"
      outputSchema:
        type: "object"
        properties:
          score:
            type: "integer"
            minimum: 0
            maximum: 100
          findings:
            type: "array"
            items:
              type: "object"
              properties:
                category:
                  type: "string"
                severity:
                  type: "string"
                  enum: ["info", "concern", "deficiency"]
                description:
                  type: "string"
          recommendation:
            type: "string"
            enum: ["approve", "conditionalApprove", "deny", "needsInfo"]
        required: ["score", "recommendation"]
    assignment:
      potentialOwners:
        roles: ["programAnalyst"]
        skills: ["cdbgProgram"]
      businessAdministrators:
        roles: ["reviewSupervisor"]
    sla:
      dueIn: "P10BD"
      warningAt: "P7BD"
      businessCalendar: "federalWorkdays"
      escalateOnBreach:
        to:
          roles: ["reviewSupervisor"]
        action: "reassign"
    separation:
      excludeFrom: ["finalApproval"]
      constraint: "sameInstance"

  financialReview:
    version: "1.0.0"
    description: "Evaluate budget, cost reasonableness, and financial capacity."
    form:
      inputSchema:
        $ref: "#/caseFile/items/application/schema"
      outputSchema:
        type: "object"
        properties:
          score:
            type: "integer"
            minimum: 0
            maximum: 100
          budgetIssues:
            type: "array"
            items:
              type: "string"
          recommendation:
            type: "string"
            enum: ["approve", "conditionalApprove", "deny", "needsInfo"]
        required: ["score", "recommendation"]
    assignment:
      potentialOwners:
        roles: ["financialAnalyst"]
        skills: ["grantFinance"]
      businessAdministrators:
        roles: ["reviewSupervisor"]
    sla:
      dueIn: "P10BD"
      businessCalendar: "federalWorkdays"
    separation:
      excludeFrom: ["finalApproval"]
      constraint: "sameInstance"

  environmentalReview:
    version: "1.0.0"
    description: "NEPA environmental review per 24 CFR Part 58."
    form:
      inputSchema:
        $ref: "#/caseFile/items/application/schema"
      outputSchema:
        type: "object"
        properties:
          classification:
            type: "string"
            enum: ["exempt", "categoricalExclusion", "EA", "EIS"]
          findings:
            type: "string"
          conditionsRequired:
            type: "array"
            items:
              type: "string"
        required: ["classification"]
    assignment:
      potentialOwners:
        roles: ["environmentalSpecialist"]
      businessAdministrators:
        roles: ["reviewSupervisor"]
    sla:
      dueIn: "P15BD"
      businessCalendar: "federalWorkdays"
    separation:
      excludeFrom: ["finalApproval"]
      constraint: "sameInstance"

  prepareRecommendation:
    version: "1.0.0"
    description: "Prepare funding recommendation based on review results."
    form:
      inputSchema:
        type: "object"
        properties:
          reviews:
            type: "object"
          eligibility:
            $ref: "#/caseFile/items/eligibility/schema"
      outputSchema:
        type: "object"
        properties:
          recommendation:
            type: "string"
            enum: ["approve", "conditionalApprove", "deny"]
          conditions:
            type: "array"
            items:
              type: "string"
          recommendedAmount:
            type: "number"
          justification:
            type: "string"
            minLength: 100
        required: ["recommendation", "justification"]
    assignment:
      potentialOwners:
        roles: ["seniorAnalyst"]
      businessAdministrators:
        roles: ["programDirector"]
    sla:
      dueIn: "P5BD"
      businessCalendar: "federalWorkdays"
    separation:
      excludeFrom: ["finalApproval"]
      constraint: "sameInstance"

  finalApproval:
    version: "1.0.0"
    description: "Final funding decision by authorized approving official."
    form:
      inputSchema:
        type: "object"
        properties:
          recommendation:
            $ref: "#/tasks/prepareRecommendation/form/outputSchema"
          reviews:
            type: "object"
      outputSchema:
        type: "object"
        properties:
          decision:
            type: "string"
            enum: ["approved", "denied", "returnForRevision"]
          approvedAmount:
            type: "number"
          conditions:
            type: "array"
            items:
              type: "string"
          rationale:
            type: "string"
            minLength: 50
        required: ["decision", "rationale"]
    assignment:
      potentialOwners:
        roles: ["approvingOfficial"]
        expression: >
          caseFile.application.requestedAmount <= 100000
            then roles('programDirector')
            else roles('regionalAdministrator')
      businessAdministrators:
        roles: ["chiefGrantsOfficer"]
    sla:
      dueIn: "P3BD"
      businessCalendar: "federalWorkdays"
    separation:
      excludeFrom:
        - "completenessCheck"
        - "programReview"
        - "financialReview"
        - "environmentalReview"
        - "prepareRecommendation"
      constraint: "sameInstance"

  notifyIneligible:
    version: "1.0.0"
    description: "Prepare and send ineligibility notification."
    form:
      inputSchema:
        $ref: "#/caseFile/items/eligibility/schema"
      outputSchema:
        type: "object"
        properties:
          letterSent:
            type: "boolean"
          sentDate:
            type: "string"
            format: "date"
        required: ["letterSent"]
    assignment:
      potentialOwners:
        roles: ["correspondenceSpecialist"]

  issueDecisionLetter:
    version: "1.0.0"
    description: "Prepare and send final decision notification."
    form:
      inputSchema:
        type: "object"
        properties:
          adjudication:
            type: "object"
      outputSchema:
        type: "object"
        properties:
          letterSent:
            type: "boolean"
          sentDate:
            type: "string"
            format: "date"
        required: ["letterSent"]
    assignment:
      potentialOwners:
        roles: ["correspondenceSpecialist"]

# ── Layer 4: Case File ────────────────────────────────────

caseFile:
  schema:
    type: "object"
    properties:
      intake:
        $ref: "#/caseFile/items/intake"
      application:
        $ref: "#/caseFile/items/application"
      eligibility:
        $ref: "#/caseFile/items/eligibility"
      reviews:
        $ref: "#/caseFile/items/reviews"
      adjudication:
        $ref: "#/caseFile/items/adjudication"
      outcome:
        $ref: "#/caseFile/items/outcome"

  items:
    intake:
      schema:
        type: "object"
        properties:
          receivedDate:
            type: "string"
            format: "date"
          isComplete:
            type: "boolean"
          missingItems:
            type: "array"
            items:
              type: "string"
      multiplicity: "one"

    application:
      schema:
        type: "object"
        properties:
          applicantName:
            type: "string"
          applicantId:
            type: "string"
          requestedAmount:
            type: "number"
          projectDescription:
            type: "string"
          proposedActivities:
            type: "array"
            items:
              type: "string"
          populationServed:
            type: "integer"
          areaMedianIncome:
            type: "number"
          expedited:
            type: "boolean"
          documents:
            type: "array"
            items:
              $ref: "#/caseFile/evidenceSchema"
        required:
          - "applicantName"
          - "requestedAmount"
          - "proposedActivities"
      visibility:
        default: "restricted"
        overrides:
          - roles: ["intakeSpecialist", "programAnalyst",
                     "financialAnalyst", "environmentalSpecialist",
                     "seniorAnalyst", "approvingOfficial"]
            access: "read"
      multiplicity: "one"

    eligibility:
      schema:
        type: "object"
        properties:
          eligible:
            type: "boolean"
          reason:
            type: "string"
          decision:
            type: "string"
            enum: ["eligible", "ineligible", "pendingReview"]
          determinedDate:
            type: "string"
            format: "date"
          decisionRecordRef:
            type: "string"
            format: "uri"
      multiplicity: "one"

    reviews:
      schema:
        type: "object"
        properties:
          program:
            type: "object"
          financial:
            type: "object"
          environmental:
            type: "object"
      multiplicity: "one"

    adjudication:
      schema:
        type: "object"
        properties:
          recommendation:
            type: "string"
          decision:
            type: "string"
            enum: ["approved", "denied", "returnForRevision"]
          approvedAmount:
            type: "number"
          conditions:
            type: "array"
            items:
              type: "string"
          rationale:
            type: "string"
      multiplicity: "one"

    outcome:
      schema:
        type: "object"
        properties:
          status:
            type: "string"
            enum: ["approved", "denied", "ineligible", "closedIncomplete"]
          closedDate:
            type: "string"
            format: "date"
      multiplicity: "one"

  evidenceSchema:
    type: "object"
    properties:
      id:
        type: "string"
        format: "uri"
      contentType:
        type: "string"
      contentHash:
        type: "object"
        properties:
          algorithm:
            type: "string"
            enum: ["sha-256", "sha-384", "sha-512"]
          value:
            type: "string"
        required: ["algorithm", "value"]
      receivedDate:
        type: "string"
        format: "date-time"
      description:
        type: "string"
      claimCheckUri:
        type: "string"
        format: "uri"
    required: ["id", "contentType", "contentHash"]

# ── Layer 5: Integrations ─────────────────────────────────

integrations:
  documentReceived:
    type: "event-consume"
    eventType: "org.example.documents.resubmitted"
    correlation:
      attribute: "subject"
      caseFileMapping: "caseFile.application.applicantId"

  appealFiled:
    type: "event-consume"
    eventType: "org.example.grants.appeal-filed"
    correlation:
      attribute: "subject"
      caseFileMapping: "caseFile.application.applicantId"

  grantNotification:
    type: "event-emit"
    eventType: "org.example.grants.notification"

  slaWarning:
    type: "event-emit"
    eventType: "org.example.grants.sla-warning"

# ── Layer 6: Provenance ───────────────────────────────────

provenance:
  tamperEvidence:
    enabled: true
    algorithm: "sha-256"
    treeHeadInterval: 100
  retentionPeriod: "P7Y"

# ── Layer 7: Execution ────────────────────────────────────

execution:
  defaultRetry:
    maxAttempts: 3
    backoff: "exponential"
    initialInterval: "PT5S"
    maxInterval: "PT5M"
    multiplier: 2.0
    nonRetryableErrors:
      - "ValidationError"
      - "AuthorizationDenied"
      - "SchemaViolation"
```

---

## Appendix C. Relationship to Existing Standards

This appendix is informative.

| Standard | Relationship to WOS |
|----------|-------------------|
| **BPMN 2.0** | WOS adopts BPMN's event taxonomy and error handling concepts but replaces flowchart-based topology with statecharts. WOS is not a superset or subset of BPMN; it is a distinct standard addressing different design priorities. |
| **CMMN 1.1** | WOS adopts CMMN's case file model, discretionary items concept, sentries, and milestones. CMMN's planning table is replaced by declarative guard conditions. |
| **DMN 1.4** | WOS adopts DMN's decision table structure, hit policies, and FEEL expression language (as a profile). WOS extends DMN with defeasible rules and temporal parameters. |
| **SCXML 1.0** | WOS adopts SCXML's statechart semantics (hierarchical states, parallel regions, history states, transitions with events and guards). WOS replaces SCXML's XML serialization with YAML/JSON. |
| **WS-HumanTask 1.1** | WOS adopts WS-HumanTask's task lifecycle states and role model, simplified and modernized. SOAP/WSDL coupling is removed. |
| **CloudEvents 1.0** | WOS adopts CloudEvents as the event envelope format, with WOS-specific extension attributes. |
| **W3C PROV** | WOS adopts the Entity-Activity-Agent triad from PROV as the provenance data model, extended with decision records and override records. |
| **OpenAPI 3.1 / AsyncAPI 3.0** | WOS references OpenAPI and AsyncAPI for integration interface contracts. WOS does not redefine API description. |
| **JSON Schema 2020-12** | WOS uses JSON Schema for all data validation: document structure, case file items, task inputs/outputs, and decision inputs/outputs. |
| **Temporal.io** | WOS's durable execution guarantees (§12) describe the same properties that Temporal provides, expressed as abstract requirements rather than implementation mechanisms. |

---

## Appendix D. Changelog

| Date | Version | Description |
|------|---------|-------------|
| 2026-04-08 | 0.1.0 | First Public Working Draft. |
