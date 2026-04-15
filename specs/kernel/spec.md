---
title: WOS Kernel Specification
version: 1.0.0-draft.1
date: 2026-04-09
status: draft
---

# WOS Kernel Specification v1.0

**Version:** 1.0.0-draft.1
**Date:** 2026-04-09
**Editors:** Formspec Working Group
**Companion to:** Formspec v1.0 -- A JSON-Native Declarative Form Standard

---

## Abstract

The WOS Kernel Specification defines the minimal orchestration substrate for the Workflow Orchestration Standard (WOS). A Kernel Document -- itself a JSON document -- declares a workflow's lifecycle topology (states, transitions, events, milestones), case state model (typed data with append-only mutation history), actor model (human and system), impact level classification, contract validation interface, provenance Facts tier, durable execution guarantees, and five named extension seams. The kernel is self-sufficient: a kernel-only deployment orchestrates workflows without requiring any governance layer.

WOS is a companion framework to Formspec v1.0. WOS MUST NOT alter core Formspec processing semantics. A Formspec processor that does not implement WOS remains fully conformant to Formspec. WOS processors MUST delegate Formspec Definition evaluation to a Formspec-conformant processor (Core S1.4).

---

## Status of This Document

This document is a **draft specification**. It is the foundation layer of the Workflow Orchestration Standard, a companion framework to Formspec v1.0 that does not modify Formspec's processing model. Implementors are encouraged to experiment with this specification and provide feedback, but MUST NOT treat it as stable for production use until a 1.0.0 release is published.

---

## 1. Introduction

### 1.1 Background

High-stakes workflows -- grants processing, benefits adjudication, licensing, inspections, investigations, compliance review -- share requirements that no existing standard adequately integrates. They are long-running, human-centric, evidence-driven, heavily regulated, and increasingly involve AI agents.

This specification defines the orchestration kernel: the minimal substrate that every WOS workflow requires. The kernel orchestrates. Layers govern. Contracts validate. Sidecars enrich.

### 1.2 Design Goals

1. **Human workflows are first-class.** A complex, multi-agency, rights-impacting workflow with zero AI is fully supported by Kernel alone or Kernel + governance layers. No AI layer is ever required.
2. **Each layer's schema is self-contained.** A governance document targets a kernel workflow. Neither requires the other to validate.
3. **Sidecars are pure metadata.** They enrich without affecting processing.
4. **Provenance grows upward.** Layer 0 records facts. Each higher layer adds interpretive structure. Lower layers are never modified.
5. **Complexity is opt-in.** Kernel-only is a valid deployment.

### 1.3 Scope

**Within scope:** lifecycle topology; case state model; actor model (human and system); impact level classification; contract validation interface; provenance Facts tier; durable execution guarantees; evaluation context; separation principles; named extension seams; conformance profiles.

**Out of scope:** due process, review protocols, data validation pipelines (Layer 1: Workflow Governance). Agent registration, deontic constraints, autonomy levels (Layer 2: AI Integration). SMT verification, equity guardrails, constraint zones (Layer 3: Advanced Governance). Full saga execution semantics (Lifecycle Detail companion).

### 1.4 Relationship to Formspec

This specification is a **companion framework** to Formspec v1.0. Formspec governs the data-collection instrument. WOS governs the orchestration envelope -- who does the work, when, under what authority, and what gets recorded.

WOS MUST NOT alter core Formspec processing semantics. A Formspec processor that does not implement WOS remains fully conformant to Formspec.

WOS processors MUST delegate Formspec Definition evaluation to a Formspec-conformant processor (Core S1.4). The WOS processor provides orchestration context; the Formspec processor provides Definition evaluation.

### 1.5 Notational Conventions

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [BCP 14][RFC 2119] [RFC 8174] when, and only when, they appear in ALL CAPITALS, as shown here.

JSON syntax and data types are as defined in [RFC 8259]. URI syntax is as defined in [RFC 3986].

Terms defined in the Formspec v1.0 core specification -- including *Definition*, *Item*, *Bind*, *FEL*, and *conformant processor* -- retain their core-specification meanings throughout this document unless explicitly redefined.

---

## 2. Conformance

### 2.1 Conformance Classes

**WOS Kernel Document.** A serialized workflow definition conforming to the structural and semantic requirements of this specification.

**WOS Kernel Processor.** A software system that consumes WOS Kernel Documents and produces behavior consistent with the semantics defined herein.

### 2.2 Conformance Profiles

Two profiles are defined:

**Kernel Structural.** Parse, validate against the WOS Kernel JSON Schema, round-trip without loss, and resolve contract references. The processor MUST reject documents that fail schema validation, producing diagnostics for each violation.

**Kernel Complete.** Structural conformance plus: execute lifecycle semantics (S4), maintain case state (S5), produce provenance records (S8) for every state transition and action, enforce durable execution guarantees (S9), and correctly evaluate transition guards using the evaluation context (S7).

---

## 3. Actor Model

This section is normative.

The kernel recognizes two types of actors that participate in workflows:

| Type | Description | Provenance Requirements |
|------|-------------|------------------------|
| `human` | A person who performs tasks, makes decisions, and exercises judgment. | Identity, role, timestamp. |
| `system` | A deterministic software component that executes automated actions, integrations, and rule evaluations. | Component identifier, version, timestamp. |

### 3.1 Actor Type Determination

An actor's type is determined by who bears decision authority for the specific action. A human using a software tool remains a `human` actor if the human reviews and commits the output. An actor's type is immutable for a given action.

### 3.2 Extensibility

The `actorExtension` seam (S10.1) allows higher layers to register additional actor types. Layer 2 (AI Integration) uses this seam to register the `agent` actor type with additional provenance requirements (model identifier, model version, confidence, input summary).

### 3.3 Normative Constraints

1. Every action MUST be attributed to a declared actor.
2. Provenance records MUST include the actor type and actor identifier.
3. Actor declarations MUST have unique identifiers within a Kernel Document.

### 3.4 Actor Assignment

Actor assignment to specific workflow actions is implementation-defined in kernel-only deployments. The kernel declares which actors exist and their types; the runtime determines which specific actor performs which action based on implementation-specific policies (role-based routing, load balancing, manual assignment, etc.).

---

## 4. Lifecycle Topology

This section is normative.

### 4.1 Overview

The lifecycle defines the statechart governing a workflow instance's progression. The semantics are based on Harel statecharts as formalized in W3C SCXML, adapted for case-oriented workflows.

### 4.2 Deterministic Evaluation Algorithm

The lifecycle is a **pure function** of (current states x event x guards -> next states). Two conformant Kernel Processors given the same Kernel Document and the same sequence of events MUST produce the same sequence of state transitions.

Events MUST be processed serially per instance. Concurrent event delivery MUST be serialized -- two events arriving simultaneously for the same instance MUST be queued and processed sequentially. Multiple actors MAY append to case state concurrently (S5.4), but lifecycle transitions MUST be serialized. Multiple instances MAY process events concurrently; the serialization requirement is per-instance, not global.

### 4.3 States

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `type` | enum | REQUIRED | `atomic`, `compound`, `parallel`, or `final`. |
| `onEntry` | array of Action | OPTIONAL | Actions executed on state entry. |
| `onExit` | array of Action | OPTIONAL | Actions executed on state exit. |
| `transitions` | array of Transition | OPTIONAL | Outgoing transitions from this state. |
| `tags` | array of string | OPTIONAL | Semantic tags for governance attachment via `lifecycleHook` (S10.4). |
| `initialState` | string | CONDITIONAL | Required for `compound` states. |
| `regions` | map of Region | CONDITIONAL | Required for `parallel` states. |
| `cancellationPolicy` | enum | OPTIONAL | For `parallel` states only: `cancel-siblings`, `wait-all`, or `fail-fast`. Default: `wait-all`. |

**Atomic states** have no substates.

**Compound states** contain substates with a designated `initialState`. When entered, execution proceeds to the initial substate.

**Parallel states** contain named regions executing concurrently. A parallel state is not exited until all regions reach a final state, unless the `cancellationPolicy` overrides this behavior.

**Final states** indicate completion of the enclosing scope. A top-level final state indicates workflow completion. Final states MUST NOT have outgoing transitions.

### 4.4 Cancellation Policy

For `parallel` states, the `cancellationPolicy` governs behavior when any region completes or fails:

| Policy | Behavior |
|--------|----------|
| `wait-all` | The parallel state is not exited until all regions reach a final state. Default behavior. |
| `cancel-siblings` | When any region reaches a final state, all other regions are cancelled. The parallel state exits. |
| `fail-fast` | When any region reaches an error final state, all other regions are cancelled immediately. The parallel state exits with the error. |

### 4.5 Transitions

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `event` | string | REQUIRED | Triggering event identifier. |
| `target` | string | REQUIRED | Target state identifier. |
| `guard` | string (FEL) | OPTIONAL | FEL expression that must evaluate to `true` for the transition to fire. |
| `actions` | array of Action | OPTIONAL | Actions executed during the transition. |
| `tags` | array of string | OPTIONAL | Semantic tags for governance attachment via `lifecycleHook` (S10.4). |
| `description` | string | OPTIONAL | Human-readable explanation of this transition. |

### 4.6 Transition Resolution

When an event occurs:

1. Collect all transitions from current active states whose `event` property matches the triggering event.
2. Evaluate guards in **document order**. The first transition whose guard evaluates to `true` (or has no guard) wins.
3. If no transition matches, the event is recorded in provenance but does not change lifecycle state.

This is deterministic: document order is the tiebreaker. Two conformant processors given the same document and the same event from the same state MUST select the same transition.

### 4.7 Transition Execution Sequence

When a transition fires:

1. Execute `onExit` actions of the source state, innermost first.
2. Execute transition `actions`.
3. Execute `onEntry` actions of the target state, outermost first.
4. Emit a provenance record for the transition.

### 4.8 Fork and Join

**Fork (entering a parallel state):** All regions are activated simultaneously. Each region begins in its `initialState`.

**Join (exiting a parallel state):** Governed by the `cancellationPolicy`. Under `wait-all` (default), when all regions reach a final state, the processor generates a synthetic `$join` event. Outgoing transitions from the parallel state MUST use `$join` as their event. Under `cancel-siblings`, the synthetic event fires when any region reaches a final state; remaining regions are cancelled. Under `fail-fast`, the synthetic event fires when any region reaches a state tagged `error`; remaining regions are cancelled.

The `$join` event is kernel-defined. Workflow authors MUST NOT use `$join` as a user-defined event name.

### 4.9 Event Handling

Events that match no transition from any current active state are recorded in provenance but do not change lifecycle state. This is not an error condition.

### 4.10 Kernel-Generated Events

The kernel generates synthetic events in response to internal conditions. Kernel-generated event names are prefixed with `$` to distinguish them from document-authored events. Workflow authors MUST NOT define events with the `$` prefix.

| Event | Source | Description |
| ----- | ------ | ----------- |
| `$join` | Parallel state | Fired when a parallel state's join condition is met (S4.8). |
| `$timeout.task` | Timer | Fired when a `taskTimeout` duration expires (S9.7). |
| `$timeout.service` | Timer | Fired when a `serviceTimeout` duration expires (S9.7). |
| `$timeout.state` | Timer | Fired when a `stateTimeout` duration expires (S9.7). |
| `$timeout.signal` | Timer | Fired when a `signalTimeout` duration expires (S9.7). |
| `$timeout.workflow` | Timer | Fired when a `workflowTimeout` duration expires (S9.7). |
| `$error` | Processor | Fired when an action fails and no action-level error handling applies. |
| `$compensation.complete` | Processor | Fired when a compensation sequence completes (S9.5). |
| `$related.stateChanged` | Processor | Fired when a related case transitions to a new state (S5.5). Payload: `{ relatedInstanceId, fromState, toState, event }`. |
| `$related.resolved` | Processor | Fired when a related case reaches a top-level final state (S5.5). Payload: `{ relatedInstanceId, finalState, resolution }`. |
| `$related.holdReleased` | Processor | Fired when a related case exits a state tagged `hold` (S5.5). Payload: `{ relatedInstanceId, holdState, releaseEvent }`. |

Kernel-generated events follow the same lifecycle rules as document-authored events: they match transitions, fire provenance records, and are ignored (with provenance) if no matching transition exists.

**Relationship event cascade prevention.** Relationship-triggered events (`$related.*`) can cause cascading effects when related cases react to each other's state changes. To prevent unbounded cascading, the processor MUST enforce a `maxRelationshipEventDepth` limit. The default depth is **3**: a state change in case A may trigger a `$related.*` event in case B (depth 1), which may trigger a `$related.*` event in case C (depth 2), which may trigger a `$related.*` event in case D (depth 3), but no further `$related.*` events are generated beyond the cap. When the cap is reached, the processor MUST record a `relationshipDepthCapReached` provenance record and MUST NOT generate further `$related.*` events for that cascade chain.

### 4.11 Reentry

Entering a state fires `onEntry` behavior regardless of prior visits. Context from prior visits is preserved in the case state history (S5), not in the lifecycle model.

### 4.12 Semantic Transition Tags

Transitions and states carry semantic `tags` that declare their nature (e.g., `["determination", "review"]`). Governance documents from higher layers match on tags to attach governance rules via the `lifecycleHook` seam (S10.4).

Tags are free-form strings. The following tags are conventionally recognized by Layer 1 (Workflow Governance):

| Tag | Conventional Meaning |
|-----|---------------------|
| `determination` | A step that produces a consequential decision. |
| `review` | A step where work is reviewed by another actor. |
| `adverse-decision` | A step that may produce an unfavorable outcome for an individual. |
| `quality-check` | A step subject to quality assurance sampling. |
| `intake` | An initial submission or intake step. |
| `appeal` | An appeal or reconsideration step. |
| `notification` | A step that produces notices to affected individuals. |
| `hold` | A state where the case is suspended pending an external condition. |

### 4.13 Milestones

Milestones are named conditions on case state that, when satisfied, indicate meaningful progress. Milestones do not affect lifecycle state directly -- they are observable conditions. The milestone identifier is the map key in `lifecycle.milestones`.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `condition` | string (FEL) | REQUIRED | FEL expression evaluated against case state. |
| `description` | string | OPTIONAL | Human-readable description. |

### 4.14 History States

Compound states MAY declare a `historyState` property (`shallow` or `deep`). When present, reentry to the compound state resumes the last active substate instead of the `initialState`, overriding the default reentry behavior (S4.11).

- **`shallow`**: Resumes the last active direct substate of this compound state.
- **`deep`**: Restores the full nested configuration at all nesting levels.

History state semantics (algorithms, clearing rules, interactions with parallel states) are defined in the Lifecycle Detail Companion (S3). A Kernel Structural processor MAY ignore `historyState`; a Kernel Complete processor MUST implement it per the companion's algorithms.

---

## 5. Case State

This section is normative.

### 5.1 Overview

Case state is the structured data container associated with a workflow instance. Case state is an **append-only log** that grows regardless of lifecycle transitions. Lifecycle state (where in the workflow) and case state (what data exists) are independent.

This separation is what makes governance attachment clean -- governance injects at transitions without touching the state machine's determinism.

### 5.2 Case File

The `caseFile` property defines the typed data schema for the case. Each field has a type, optional default value, and optional description.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `fields` | map of FieldDefinition | REQUIRED | Named fields with type declarations. |

### 5.3 Field Types

The kernel supports the following field types: `string`, `number`, `integer`, `boolean`, `date`, `datetime`, `object`, `array`.

### 5.4 Mutation History

Every mutation to case state MUST be recorded with:

1. The field path that was modified.
2. The previous value (or `null` if the field was unset).
3. The new value.
4. The actor who made the mutation.
5. The timestamp of the mutation.
6. The lifecycle state at the time of the mutation.

The mutation history is append-only. Previous entries MUST NOT be modified or deleted.

### 5.5 Case Relationships

A workflow instance MAY declare typed relationships to other case instances. Relationships are metadata -- they do NOT affect lifecycle evaluation. Cross-case behavioral interaction (e.g., "when the appeal case enters determination, notify the original case") uses the existing `correlationKey` mechanism (Kernel S9.4): the related case emits an event that this case receives via correlation.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `type` | enum | REQUIRED | Relationship type: `parent`, `child`, `sibling`, `related`, or `supersedes`. Extensible via `x-` prefixed values. |
| `targetCase` | string (URI) | REQUIRED | URI reference to the related case instance. |
| `relationship` | string | OPTIONAL | Semantic label describing the relationship (e.g., `"appeal-of"`, `"household-member"`, `"consolidated-with"`). |
| `bidirectional` | boolean | OPTIONAL | When `true`, the target case SHOULD also record the inverse relationship. Default: `false`. |

Case relationship creation and removal MUST be recorded as provenance events. A conformant processor MUST NOT evaluate FEL guard expressions that reference data in a related case's case state -- cross-case guards would break the deterministic evaluation algorithm (Kernel S4.2) because the related case's state is not under this instance's control.

---

## 6. Impact Level Classification

This section is normative.

Every Kernel Document MUST declare an `impactLevel` classifying the consequence level of decisions made within the workflow. When `impactLevel` is not specified, the effective default is `operational`.

| Level | Definition | Governance Implication |
|-------|-----------|----------------------|
| `rights-impacting` | Decisions affect individual legal rights, benefits, services, or obligations. | Full due process required (Layer 1). |
| `safety-impacting` | Decisions affect individual or public safety. | Full due process required (Layer 1). |
| `operational` | Organizational operations without direct individual impact. | Due process RECOMMENDED. |
| `informational` | Informational outputs; no binding decisions. | Due process OPTIONAL. |

The impact level serves as a **proportionality index**: higher layers use it to determine the strength of governance controls. The kernel declares the level; governance layers act on it.

---

## 7. Evaluation Context

This section is normative.

### 7.1 Overview

All FEL evaluation -- guards, milestone conditions, action parameters, and any layer-injected expressions -- happens within an **evaluation context**: a flat namespace of named variables.

### 7.2 Base Context

The kernel provides the base evaluation context:

| Variable | Source | Availability | Description |
|----------|--------|-------------|-------------|
| `caseFile` | Kernel | Always | Current case file data. |
| `event` | Kernel | Transition guards and actions only | Triggering event data. |
| `task` | Kernel | Task-related expressions only | Current task data. |
| `instance` | Kernel | Always | Workflow instance metadata (id, creation time, current states, definition version). |
| `env` | Kernel | Always | Implementation-defined environment variables. |

### 7.3 Context Enrichment

Higher layers enrich the evaluation context by adding their variables through named seams:

| Variable | Source | Added Via |
|----------|--------|----------|
| `parameters` | Layer 1 | `lifecycleHook` -- temporal parameters resolved to date-effective values. |
| `agent` | Layer 2 | `actorExtension` -- agent operational state including calibration metrics. |
| `output` | Layer 2 | `contractHook` -- agent output being evaluated. |
| `custody` | Binding | `custodyHook` — current custody posture declaration, if any. |

By the time any FEL expression evaluates, the context contains all enriched values. The workflow author writes expressions like `caseFile.income < parameters.eligibilityThreshold` -- the resolution is automatic.

### 7.4 FEL Usage

WOS uses FEL (Formspec Expression Language) for all expressions. WOS MUST use only FEL built-in functions (Core S3.5) and extension functions (Core S3.12). WOS MUST NOT define new FEL grammar.

**Rejected alternative:** Earlier WOS drafts proposed FEL grammar extensions (quantified expressions with `satisfies` syntax, range literals `[0..32760]`, filter expressions `$list[$.field > 5]`). These are rejected. The same capabilities are achieved without grammar changes: `every` and `some` as built-in functions (Core S3.5), duration as a built-in function, and filtering via extension functions (Core S3.12). Grammar stability is a non-negotiable Formspec constraint.

---

## 8. Provenance: Facts Tier

This section is normative.

### 8.1 Overview

The kernel defines the **Facts tier** -- the foundational, immutable provenance layer. Every action that changes lifecycle state or case state MUST produce a Facts tier record.

Higher layers add interpretive provenance tiers through the `provenanceLayer` seam (S10.3):
- Layer 1 adds the **Reasoning tier** (rules applied, evidence consulted) and **Counterfactual tier** (what would change the outcome).
- Layer 2 adds the **Narrative tier** (model-generated explanation; non-authoritative).

### 8.2 Facts Tier Record

Every provenance record MUST include:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | REQUIRED | Unique record identifier. |
| `timestamp` | datetime | REQUIRED | When the action occurred (ISO 8601). |
| `actorId` | string | REQUIRED | Identifier of the actor who performed the action. |
| `actorType` | enum | REQUIRED | `human` or `system` (extensible via `actorExtension`). |
| `action` | string | REQUIRED | What action was performed. |
| `inputs` | object | OPTIONAL | Input data for the action. |
| `outputs` | object | OPTIONAL | Output data from the action. |
| `inputDigest` | string | OPTIONAL | Cryptographic digest of inputs for tamper detection. |
| `outputDigest` | string | OPTIONAL | Cryptographic digest of outputs for tamper detection. |
| `definitionVersion` | string | REQUIRED | Version of the Kernel Document governing this action. |
| `lifecycleState` | string | REQUIRED | Lifecycle state at the time of the action. |
| `extensions` | object | OPTIONAL | Extension data. All keys MUST be prefixed with `x-`. |

### 8.3 Tamper Detection

The optional `inputDigest` and `outputDigest` fields provide lightweight tamper detection. When present, a conformant processor SHOULD verify the digest matches the referenced data. The digest algorithm is implementation-defined but MUST be recorded in the provenance record's extensions.

### 8.4 PROV-DM Compatibility

The Facts tier is designed for compatibility with the W3C PROV Data Model (PROV-DM). Each Facts tier record maps to a PROV Activity with associated PROV Entities (inputs, outputs) and a PROV Agent (actor). Detailed vocabulary mapping is deferred to the Semantic Profile (Phase 3).

---

## 9. Durable Execution

This section is normative.

### 9.1 Guarantees

A Kernel Complete processor MUST provide the following guarantees:

| Guarantee | Requirement |
|-----------|-------------|
| **G1: Crash Recovery** | Non-terminal workflow instances MUST resume from the last durable state after a crash. |
| **G2: Persistent State** | Lifecycle state, case file data, and timer registrations MUST be durably persisted. |
| **G3: Deterministic Replay** | Every action invoking a non-deterministic external service MUST persist the output as an immutable step result before advancing workflow state. During recovery or audit replay, the processor MUST use the persisted output rather than re-invoking the service. |
| **G4: Durable Timers** | Timers MUST survive restarts, fire within tolerance, and consume no runtime resources while waiting. |
| **G5: External Signal Delivery** | Signals addressed to inactive instances MUST be durably enqueued. |

### 9.2 Actions

The kernel defines the following action types:

| Action Type | Description | Key Properties |
|-------------|-------------|----------------|
| `createTask` | Creates a human or Formspec-backed task instance. | `taskRef`, `assignTo`, `contractRef` |
| `invokeService` | Invokes an external service. | `serviceRef`, `idempotencyKey` |
| `setData` | Sets a case file value. | `path`, `value` |
| `emitEvent` | Emits an event. | `eventType`, `data` |
| `startTimer` | Starts a durable timer. | `timerId`, `duration` or `deadline`, `event` |
| `cancelTimer` | Cancels a running timer. | `timerId` |
| `log` | Writes an entry to provenance. | `message`, `data` |

**Execution ordering.** Actions within a single state's `onEntry` or `onExit` execute sequentially in document order. The processor MUST NOT reorder actions within a state or transition. Actions across parallel regions MAY execute concurrently; provenance MUST record the actual execution order regardless of whether execution was concurrent or sequential.

**Formspec-backed tasks.** A `createTask` action MAY include `contractRef` when the task is backed by a ContractReference. If that ContractReference has `binding: "formspec"`, Runtime Companion S15 defines the presentation, draft, submit, validation, mapping, and provenance behavior. `prefillMappingRef` and `responseMappingRef` MAY appear on either the ContractReference or the action. The action-level value overrides the ContractReference value for that task. `completionEvent` and `failureEvent` MAY name lifecycle events emitted after the task reaches `completed` or `failed`.

### 9.3 Idempotency Keys

The `invokeService` action supports an optional `idempotencyKey` property. When present, the processor MUST use this key to deduplicate service invocations. This closes the crash-between-invoke-and-persist window: if the processor crashes after invoking the service but before persisting the result, the service can be safely re-invoked with the same idempotency key.

### 9.4 Correlation Keys

External signals and callbacks carry a `correlationKey` that the processor uses to route the response to the correct workflow instance. Correlation keys are fundamental for any workflow awaiting external callbacks.

### 9.5 Compensation Seam

Actions MAY declare a `compensatingAction` -- a semantically meaningful reversal. Scopes MAY be marked `compensable: true`. This defines the compensation seam only; detailed execution semantics (reverse ordering, pivot steps, forward/backward recovery) are deferred to the Lifecycle Detail companion.

### 9.6 Instance Versioning

A workflow instance is bound to its creation-time definition version unless explicitly migrated. When a Kernel Document is updated, existing instances continue executing under the version that created them. Instance migration is an explicit operation that MUST be recorded in provenance.

Version pinning applies equally to Formspec Definitions referenced as contracts. When a Kernel Document references a Formspec Definition via `contractRef`, the version of that Definition is pinned at instance creation time (Formspec Changelog VP-01, VP-02). Instance migration SHOULD use the Formspec Changelog (Changelog S4) to generate migration maps for contract changes between versions.

### 9.7 Timeout Categories

The kernel recognizes five timeout categories. All are expressed as ISO 8601 durations.

| Category | Description |
|----------|-------------|
| `taskTimeout` | Maximum time for a human task to complete. |
| `serviceTimeout` | Maximum time for an external service invocation. |
| `stateTimeout` | Maximum time a workflow may remain in a state. |
| `signalTimeout` | Maximum time to wait for an external signal. |
| `workflowTimeout` | Maximum total workflow duration. |

Timeouts generate kernel `$timeout.*` events (S4.10) that the lifecycle handles like any other event.

---

## 10. Named Extension Seams

This section is normative.

The kernel defines six extension seams. Higher layers attach governance and capabilities through these seams.

### 10.1 `actorExtension`

**Purpose:** Actor model extensibility.

Higher layers register additional actor types through this seam. The kernel defines `human` and `system`. Layer 2 (AI Integration) registers `agent` with additional provenance requirements.

### 10.2 `contractHook`

**Purpose:** Data validation injection.

The kernel defines an abstract contract validation interface. Formspec Definitions are the recommended binding; JSON Schema is the baseline.

Layer 1 uses this seam to attach data validation pipelines -- validating external data against contracts with assertion gates between stages.

Layer 2 uses this seam for Formspec-as-validator -- treating agent output as untrusted input validated against the same Formspec contract a human would submit against.

### 10.3 `provenanceLayer`

**Purpose:** Audit tier injection.

The kernel provides the Facts tier. Higher layers add interpretive tiers:

- Layer 1: Reasoning tier (rules applied, evidence consulted, decision table trace) and Counterfactual tier (what would change the outcome).
- Layer 2: Narrative tier (model-generated explanation; non-authoritative).

### 10.4 `lifecycleHook`

**Purpose:** Governance attachment at transitions.

This is the primary governance seam. Governance documents from higher layers declare rules that match on semantic transition tags (S4.12). The kernel publishes tags; layers declare rules against them.

**Tag-based governance (default):** A Layer 1 document says "all transitions tagged `determination` require dual-blind review." This applies across the workflow without naming specific transitions.

**Transition-specific overrides:** When tag-based governance is not specific enough, governance documents MAY target specific transitions by identifier.

### 10.5 `custodyHook`

**Purpose:** Custody posture declaration.

Every WOS deployment handles protected content under a declared custody posture. The kernel itself makes no assumption about custody — a trust-the-host monolith and a multi-party distributed binding both conform to the kernel unchanged. Higher layers and bindings attach custody semantics here.

Custody postures are declared, not inferred. Bindings that populate this seam MUST declare, at minimum: who may read content during ordinary operation, whether recovery can occur without the user, and whether delegated compute exposes content to ordinary service components. Custody transitions (changes to any of those answers) are recorded as canonical lifecycle facts (Governance S2.9).

The kernel does NOT define the concrete Trust Profile object. Trellis (the distributed-trust binding) defines that object and binds it to this seam. A monolithic binding may populate this seam with a single declared posture (e.g., "provider-readable, no recovery without user, no delegated compute") and satisfy conformance.

### 10.6 `extensions`

**Purpose:** Standard escape hatch.

The `extensions` property accepts arbitrary data with keys prefixed by `x-`. This is the standard extensibility mechanism for implementation-specific or experimental features.

---

## 11. Contract Validation

This section is normative.

### 11.1 Contract Interface

WOS uses interface contracts to define typed data exchanges between workflow participants. A contract MUST provide:

1. **Typed fields** with declared data types.
2. **Validation rules** producing structured results.
3. **Structured validation results** with severity, field path, message, and constraint identifier.

### 11.2 Conformant Bindings

Two conformant bindings are defined:

| Binding | Capabilities | When to Use |
|---------|-------------|-------------|
| **Formspec** (recommended) | Full reactive behavior, cross-field validation (Shapes), Mapping DSL for data flow. | WOS-native implementations. Human task contracts. |
| **JSON Schema** (baseline) | Structural validation only. No reactive behavior or cross-field rules. | Minimum viable integration. |

### 11.3 Processing Delegation

WOS processors MUST delegate contract evaluation to a conformant contract processor. For Formspec bindings, this means a Formspec-conformant processor (Core S1.4). The WOS processor provides orchestration context; the contract processor provides Definition evaluation.

For Formspec-backed tasks, a ContractReference MAY also declare `prefillMappingRef` and `responseMappingRef`. `prefillMappingRef` identifies the Mapping document used to populate the task's initial Response. `responseMappingRef` identifies the Mapping document used by Runtime Companion S15 to project a completed Response into case state. When no `responseMappingRef` is supplied, Runtime Companion S15 forbids automatic host-defined Response-to-case projection.

---

## 12. Separation Principles

This section is normative.

1. **Lifecycle state MUST be separated from case state.** The lifecycle tracks where the workflow is. Case state tracks what data exists. These are independent.

2. **Process topology MUST be separated from decision logic.** The lifecycle defines structure. Decision services evaluate conditions. A guard MAY invoke a decision service, but the decision service MUST NOT contain process topology.

3. **Audit MUST be separated from execution.** Provenance records are produced as a consequence of execution but do not participate in control flow.

4. **Governance MUST be separated from orchestration.** The kernel orchestrates. Layers govern. The kernel provides seams; governance attaches through them.

---

## 13. Conformance Fixtures

### 13.1 Kernel-Only Smoke Test

A valid kernel-only deployment: a purchase order approval workflow with three states, two actors, and no governance structures. See `fixtures/kernel/purchase-order-approval.json`.

### 13.2 Validation Requirements

A Kernel Structural processor MUST accept the purchase order approval fixture without error. A Kernel Complete processor MUST additionally execute the lifecycle and produce the expected provenance records.

### 13.3 Schema Limitations

The kernel JSON Schema validates structural correctness but cannot enforce all semantic constraints. The following constraints MUST be enforced by a Kernel Complete processor:

- **Final states MUST NOT have outgoing transitions.** A `final` state with a `transitions` array is structurally valid but semantically invalid.
- **Compound states MUST have `initialState` and `states`.** A `compound` state without substates is structurally valid but semantically invalid.
- **Parallel states MUST have `regions`.** A `parallel` state without regions is structurally valid but semantically invalid.
- **Kernel-generated event names (`$` prefix) MUST NOT be used as document-authored event names** (S4.10).

These constraints are not expressed in the schema because JSON Schema's conditional validation (if/then keyed on `type`) would significantly increase schema complexity without proportionate benefit for the LLM-authoring use case.

---

## References

### Normative References

- [RFC 2119] Bradner, S., "Key words for use in RFCs to Indicate Requirement Levels", BCP 14, RFC 2119, March 1997.
- [RFC 8174] Leiba, B., "Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words", BCP 14, RFC 8174, May 2017.
- [RFC 8259] Bray, T., "The JavaScript Object Notation (JSON) Data Interchange Format", STD 90, RFC 8259, December 2017.
- [RFC 3986] Berners-Lee, T., Fielding, R., and L. Masinter, "Uniform Resource Identifier (URI): Generic Syntax", STD 66, RFC 3986, January 2005.
- [Formspec Core] Formspec Working Group, "Formspec Core Specification v1.0".

### Informative References

- [SCXML] W3C, "State Chart XML (SCXML): State Machine Notation for Control Abstraction", W3C Recommendation, 2015.
- [Harel1987] Harel, D., "Statecharts: a visual formalism for complex systems", Science of Computer Programming, 8(3), 1987.
- [PROV-DM] W3C, "PROV-DM: The PROV Data Model", W3C Recommendation, 2013.

---

## Appendix A: Relationship to Existing Standards (informative)

This appendix is informative.

| Standard | WOS Relationship |
| -------- | ---------------- |
| BPMN 2.0 | Adopts event taxonomy and error handling. Replaces flowchart topology with Harel statecharts. WOS is a distinct standard, not a superset or subset of BPMN. |
| CMMN 1.1 | Adopts case file model, sentries, and milestones. Constraint zones (Layer 3) provide DCR-based adaptive case management in place of CMMN's planning table. |
| DMN 1.4 | WOS delegates decision logic to external decision services via `invokeService`. WOS does not embed a decision table engine. |
| SCXML 1.0 | Adopts statechart semantics (hierarchical states, parallel regions, transitions). Replaces XML serialization with JSON. |
| WS-HumanTask 1.1 | Adopts task lifecycle states and role model (simplified). Removes SOAP/WSDL coupling. See Governance S10. |
| CloudEvents 1.0 | Adopted as event envelope format. WOS-specific extension attributes deferred to Integration Profile. |
| W3C PROV-DM | Adopts Entity-Activity-Agent triad as a design constraint. Provenance records are mappable to PROV-DM. Full PROV-O vocabulary deferred to Semantic Profile. |
| OpenAPI 3.1 / AsyncAPI 3.0 | Referenced for integration interface contracts. WOS does not redefine API description formats. |
| JSON Schema 2020-12 | Used for all document validation (kernel, governance, AI integration, advanced governance, sidecars). |
| Temporal.io / Durable Task | Durable execution guarantees (G1-G5) are expressed as abstract requirements, not implementation mechanisms. Any durable execution runtime is valid. |
| XACML / OPA / Cedar | External policy engines invocable via `invokeService`. WOS deontic constraints (Layer 2) provide a governance-native alternative for agent-specific policies. |
| OpenFisca / Catala | External rules engines for temporal parameter evaluation. WOS temporal parameters (Governance S13) declare date-indexed values; evaluation is delegated. |
