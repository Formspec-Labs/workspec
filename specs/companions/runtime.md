---
title: WOS Runtime Companion
version: 1.0.0-draft.1
date: 2026-04-09
status: draft
---

# WOS Runtime Companion v1.0

**Version:** 1.0.0-draft.1
**Date:** 2026-04-09
**Editors:** Formspec Working Group
**Companion to:** WOS Kernel Specification v1.0

---

## Abstract

The WOS Runtime Companion defines the behavioral contract between the WOS evaluation engine and its host environment. A processor that implements this companion can host any WOS workflow at any scale, on any infrastructure. The companion defines WHAT a conformant processor must do -- not HOW. It specifies instance serialization (CaseInstance), event delivery semantics, action execution ordering, durability guarantees, timer management, governance enforcement, explanation assembly, evaluation modes, multi-version coexistence, host interfaces, security boundaries, and relationship-triggered events.

This is a companion specification, not a layer. It elaborates kernel runtime semantics defined in the Kernel Specification (S4, S5, S8, S9) and the Lifecycle Detail Companion (S2-S6) without adding new document types, seams, or governance structures. It does not prescribe infrastructure: database technology, message queue implementation, cloud provider, or deployment architecture are host decisions, not engine concerns.

WOS MUST NOT alter core Formspec processing semantics. WOS processors MUST delegate Formspec Definition evaluation to a Formspec-conformant processor (Core S1.4).

---

## Status of This Document

This document is a **draft specification**. It is a companion to the WOS Kernel Specification v1.0 that defines the behavioral contract for runtime processors. It does not define new document types, kernel seams, or governance structures. Implementors building Kernel Complete or higher-tier processors are encouraged to use this document as the normative reference for runtime behavior.

---

## 1. Introduction

### 1.1 Purpose

The Kernel Specification defines the lifecycle topology, case state model, provenance Facts tier, and durable execution guarantees. The Lifecycle Detail Companion provides the execution algorithms (transition evaluation, compensation, timer lifecycle, history states). This companion completes the picture by defining the runtime behavioral contract: what the evaluation engine expects from its host, what guarantees the engine provides, and how instances are serialized, versioned, and governed at runtime.

The boundary between engine and host follows a single test (ADR-0057):

> Does a difference in this behavior make two processors produce different observable outcomes for the same document and event sequence?

If YES, this companion defines the behavior normatively. If NO, this companion defines a host interface and leaves the implementation to the deployment.

### 1.2 Scope

**Within scope:** CaseInstance serialization format; instance operations; event delivery contract; action execution model; durability checkpoint semantics; timer precision and persistence; governance enforcement ordering; explanation assembly algorithm; evaluation modes; multi-version coexistence; host interfaces (traits); security model; relationship-triggered events.

**Out of scope:** specific infrastructure choices (database, message queue, cloud provider); deployment architecture (serverless, container, on-premise); host interface implementations; rendered explanation formats (PDF, HTML); network protocols between processor and host.

### 1.3 Notational Conventions

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [BCP 14][RFC 2119] [RFC 8174] when, and only when, they appear in ALL CAPITALS, as shown here.

JSON syntax and data types are as defined in [RFC 8259]. URI syntax is as defined in [RFC 3986].

Terms defined in the WOS Kernel Specification -- including *Kernel Document*, *lifecycle*, *case state*, *provenance*, *Facts tier*, *transition*, *guard*, *evaluation context*, and *kernel-generated event* -- retain their kernel-specification meanings throughout this document. Terms defined in the Lifecycle Detail Companion -- including *configuration*, *compensation log*, *pivot step*, and *region cancellation* -- retain their companion meanings.

Pseudocode in this document uses Python-like syntax for readability. It is not executable code; it is a precise specification of algorithmic behavior.

---

## 2. Conformance

### 2.1 Conformance Profiles

This companion defines three conformance profiles. Each builds on the one below.

**Runtime Structural.** Validates WOS documents against their schemas (Kernel, Governance, AI Integration, Advanced Governance, sidecars). Serializes and deserializes CaseInstance documents (S3). Round-trips instances without data loss. Does not execute lifecycle semantics.

**Runtime Complete.** Structural conformance plus: full event processing (S4), action execution (S5), durability guarantees (S6), timer management (S7), explanation assembly (S9), evaluation mode support (S10), and multi-version coexistence (S11). A Runtime Complete processor implements the Kernel Complete profile defined in the Kernel Specification (Kernel S2.2).

**Runtime Governed.** Complete conformance plus: governance enforcement (S8) including deontic constraint evaluation ordering, delegation verification, and hold management. A Runtime Governed processor implements the evaluation algorithm for Layers 1 through 3 in addition to the kernel.

### 2.2 Host Interface Requirements

All conformance profiles MUST implement the host interfaces defined in S12. The conformance profile determines which interface operations are exercised at runtime. A Runtime Structural processor calls `InstanceStore.load` and `InstanceStore.save` but never calls `ExternalService.invoke`.

---

## 3. Instance Lifecycle

### 3.1 CaseInstance

A **CaseInstance** is the serialization format for a running workflow instance. It captures the complete runtime state needed to resume processing after a crash, migrate between processors, or audit past behavior.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `instanceId` | string (URI) | REQUIRED | Globally unique identifier for this instance. |
| `definitionUrl` | string (URI) | REQUIRED | Canonical URL of the Kernel Document governing this instance. |
| `definitionVersion` | string | REQUIRED | Version of the Kernel Document, pinned at creation (Kernel S9.6). |
| `configuration` | array of string | REQUIRED | Active leaf states as an ordered array. The order is deterministic: document order of state declarations within the kernel, depth-first. |
| `caseState` | object | REQUIRED | Current case file field values. |
| `provenancePosition` | integer | REQUIRED | Index into the append-only provenance log. Indicates how many provenance records have been durably persisted for this instance. |
| `timers` | array of TimerState | REQUIRED | Pending timer state. Empty array when no timers are active. |
| `historyStore` | object | OPTIONAL | Saved history state configurations, keyed by compound state identifier. Present only when the kernel document uses history states (Kernel S4.14). |
| `compensationLogs` | object | OPTIONAL | Active compensation logs, keyed by compensable scope identifier. Present only when compensable scopes are active (Lifecycle Detail S5). |
| `status` | enum | REQUIRED | Instance status: `active`, `suspended`, `migrating`, `completed`, `terminated`. |
| `createdAt` | string (datetime) | REQUIRED | ISO 8601 timestamp of instance creation. |
| `updatedAt` | string (datetime) | REQUIRED | ISO 8601 timestamp of last state change. |
| `extensions` | object | OPTIONAL | Extension data. All keys MUST be prefixed with `x-`. |

**TimerState** captures a pending timer:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `timerId` | string | REQUIRED | Timer identifier, as declared in the `startTimer` action. |
| `deadline` | string (datetime) | REQUIRED | Absolute deadline when this timer fires (ISO 8601). If the original action used a relative `duration`, the processor converts it to an absolute deadline at creation time. |
| `event` | string | REQUIRED | Event to emit when the timer fires. |
| `scopeState` | string | OPTIONAL | The state or region that scoped this timer. Used for cancellation on region exit (Lifecycle Detail S6.5). |

### 3.2 Configuration Ordering

The `configuration` array is ordered by document declaration order, depth-first. Given a kernel document where states are declared in the order `[intake, review, review.initial, review.detailed, determination, complete]`, a configuration of `[review.initial, determination]` (two parallel regions) preserves that order. Two conformant processors serializing the same runtime state MUST produce the same `configuration` array.

### 3.3 Instance Operations

A conformant processor MUST support the following operations on CaseInstance:

| Operation | Input | Effect | Provenance |
|-----------|-------|--------|------------|
| `create` | Kernel Document URL + version, initial case state | Creates a new instance in the kernel's initial state. | `instanceCreated` |
| `processEvent` | Event (name, actor, data, idempotency token) | Evaluates the transition algorithm (Lifecycle Detail S2). | `stateTransition` or `unmatchedEvent` |
| `advanceTime` | Target timestamp | Fires all timers whose deadline is at or before the target timestamp, in deadline order. | `timer.fired` per timer |
| `migrate` | New definition URL + version, migration map | Changes the governing definition (S11). | `instanceMigrated` |
| `suspend` | Reason | Sets status to `suspended`. No events are processed while suspended. | `instanceSuspended` |
| `resume` | (none) | Sets status to `active`. Pending events (if any) are processed. | `instanceResumed` |
| `terminate` | Reason | Sets status to `terminated`. Irreversible. | `instanceTerminated` |

Every operation produces at least one provenance record. A `processEvent` that fires a transition produces the transition provenance defined in Kernel S4.7 and Kernel S8.2.

### 3.4 Status Transitions

```
        create
          |
          v
       active <--------+
      /  |   \          |
     v   v    v         |
suspended migrating  (processEvent, advanceTime)
     |       |
     v       v
   active  active
     |
     v
  terminated    completed
```

| From | To | Trigger |
|------|----|---------|
| (none) | `active` | `create` |
| `active` | `active` | `processEvent`, `advanceTime` (no status change) |
| `active` | `suspended` | `suspend` |
| `active` | `migrating` | `migrate` (during migration) |
| `active` | `completed` | Lifecycle reaches a top-level final state. |
| `active` | `terminated` | `terminate` |
| `suspended` | `active` | `resume` |
| `migrating` | `active` | Migration completes. |
| `suspended` | `terminated` | `terminate` |

An instance in `completed` or `terminated` status MUST NOT accept any further operations except read-only queries. Attempting to process an event on a completed or terminated instance is a conformance error.

---

## 4. Event Delivery Contract

This section is normative.

### 4.1 Serial Processing

The deterministic evaluation algorithm (Kernel S4.2) requires that events are processed one at a time per instance. The processor MUST serialize concurrent event delivery. Two events arriving simultaneously for the same instance MUST be queued and processed sequentially. The queue order is implementation-defined (FIFO is RECOMMENDED).

Multiple instances MAY process events concurrently -- the serialization requirement is per-instance, not global.

### 4.2 Event Structure

Events carry:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `event` | string | REQUIRED | Event name matching a transition's `event` property. |
| `actorId` | string | OPTIONAL | Identifier of the actor submitting the event. |
| `data` | object | OPTIONAL | Event payload, available in the evaluation context as `event` (Kernel S7.2). |
| `timestamp` | string (datetime) | REQUIRED | ISO 8601 timestamp of event submission. |
| `idempotencyToken` | string | OPTIONAL | Token for exactly-once delivery (S4.3). |

### 4.3 Exactly-Once Semantics

The processor MUST provide at-least-once delivery and MUST deduplicate events. Deduplication uses the event's `idempotencyToken` when provided. When no token is provided, the processor MUST use its own delivery tracking mechanism to prevent duplicate processing.

The deduplication window is implementation-defined but MUST span at least the duration of a single event processing cycle (from event receipt through durability checkpoint). A deduplication window shorter than this provides no protection against the crash-and-replay scenario.

### 4.4 Unmatched Events

Events that match no transition from any current active state are recorded in provenance but do not change lifecycle state (Kernel S4.9). This is not an error. The provenance record for an unmatched event includes the event name, the configuration at the time of receipt, and the actor (if provided).

---

## 5. Action Execution Model

This section is normative.

### 5.1 Sequential Execution Within a State

Actions within a single state's `onEntry` or `onExit` execute sequentially in document order (Kernel S9.2). Transition actions execute sequentially between exit and entry (Kernel S4.7). The processor MUST NOT reorder actions within a state or transition.

### 5.2 Transition Execution Sequence

The full sequence for a fired transition (Kernel S4.7, Lifecycle Detail S2.4):

1. Execute `onExit` actions of the source state, innermost first.
2. Execute transition `actions` in document order.
3. Execute `onEntry` actions of the target state, outermost first.
4. Emit provenance records.

Each action produces a provenance record of type `actionExecuted`. The record includes the action type, inputs, outputs, executing actor, and timestamp.

### 5.3 Parallel Region Actions

Actions across parallel regions MAY execute concurrently. The processor is not required to parallelize -- sequential execution of region actions is conformant. However, provenance MUST record the actual execution order regardless of whether execution was concurrent or sequential. Two conformant processors given the same document and events MUST agree on which actions executed, even if the execution ordering differs.

### 5.4 Service Invocation

`invokeService` actions (Kernel S9.2) delegate to the host's ExternalService provider (S12.4). The processor MUST NOT implement service invocation itself. It declares the invocation (service reference, input data, idempotency key) and the host fulfills it.

```
function executeInvokeService(action, context):
    input = evaluateActionInput(action, context)
    result = host.externalService.invoke(
        serviceRef=action.serviceRef,
        input=input,
        idempotencyKey=action.idempotencyKey,
        timeout=action.timeout
    )
    # Persist result BEFORE advancing state (Kernel S9.1, G3)
    persistStepResult(action, result)
    return result
```

The step result persistence before state advancement is a durability requirement, not an optimization. It closes the window between service execution and state change where a crash would cause either duplicate invocation or lost results.

### 5.5 Contract Validation

Contract validation flows through the `contractHook` seam (Kernel S10.2). The processor delegates to the host's ContractValidator (S12.3). Results flow back as a ValidationResult (valid or errors). Validation failures trigger the rejection policy declared in the Governance Document (Governance S8).

```
function executeContractValidation(contractRef, data, context):
    result = host.contractValidator.validate(contractRef, data)
    emitProvenance("contractValidation", {
        contractRef: contractRef,
        valid: result.valid,
        errors: result.errors
    })
    if not result.valid:
        applyRejectionPolicy(context.rejectionPolicy, result)
    return result
```

---

## 6. Durability Guarantees

This section is normative.

### 6.1 Kernel Guarantees as Runtime Requirements

The Kernel Specification (Kernel S9.1) defines five durable execution guarantees. This section restates them as concrete runtime requirements.

**G1: Crash Recovery.** A non-terminal workflow instance MUST resume from the last durability checkpoint after a processor crash. The processor MUST NOT lose state that was durably persisted before the crash.

**G2: No Duplicate Action Execution.** On replay after a crash, actions with idempotency keys (Kernel S9.3) MUST NOT be re-executed if the previous execution's output was already persisted. Actions without idempotency keys MAY be re-executed -- the processor MUST document which action types are safe for re-execution.

**G3: Non-Deterministic Output Persisted Before Advancing.** Every `invokeService` action MUST persist its output as an immutable step result before the processor advances lifecycle state. During recovery, the processor MUST use the persisted output rather than re-invoking the service.

**G4: Timer Durability.** Timers MUST survive processor restarts, fire within their declared tolerance (S7.2), and consume no runtime resources while waiting. Timer state is part of the CaseInstance (S3.1) and is persisted at every durability checkpoint.

**G5: Signal Delivery.** External signals addressed to suspended or temporarily unreachable instances MUST be durably enqueued. The processor MUST process enqueued signals when the instance becomes available.

### 6.2 Checkpoint Semantics

The unit of durability is the **event**. After each event is fully processed -- all transitions fired, all actions executed, all provenance recorded -- the processor MUST durably persist the CaseInstance. The checkpoint includes the updated configuration, case state, provenance position, timer state, and history store.

```
function processEventWithDurability(instance, event):
    # Load pre-event state
    preEventState = snapshot(instance)
    try:
        # Process the event (Lifecycle Detail S2.3)
        processEvent(instance.configuration, event, document)
        # Checkpoint: persist the post-event state
        host.instanceStore.save(instance)
    except ProcessorCrash:
        # On restart: reload pre-event state, replay the event
        instance = host.instanceStore.load(instance.instanceId)
        # instance is at pre-event state; event will be replayed
```

Individual actions within an event are NOT durability boundaries. If the processor crashes after executing action A but before executing action B within the same transition, the entire event is replayed from the pre-event checkpoint. Actions that were already executed during the failed attempt are subject to the idempotency rules in S6.1 (G2).

### 6.3 Provenance Durability

The provenance log MUST survive any single-point failure (Kernel S9.1, G5). Provenance records are part of the durability checkpoint -- they are persisted atomically with the instance state. A conformant processor MUST NOT acknowledge an event as processed until both the instance state and the provenance records are durably persisted.

---

## 7. Timer Management

This section is normative.

### 7.1 Overview

The Lifecycle Detail Companion (S6) defines timer creation, cancellation, reset-on-reentry, and region scoping. This section defines the precision, persistence, and testing requirements.

### 7.2 Precision

Timers MUST fire within a declared tolerance of their deadline. The tolerance depends on the timer's duration:

| Timer Duration | Maximum Tolerance |
|----------------|-------------------|
| Under 1 hour | 1 second |
| 1 hour to under 1 day | 1 minute |
| 1 day or longer | 5 minutes |

A tolerance greater than the timer's duration is a conformance violation. A 30-second timer that fires 45 seconds late has violated its tolerance. A 24-hour timer that fires 3 minutes late is conformant.

The processor MAY declare tighter tolerances than these maximums. Tighter tolerances SHOULD be documented.

### 7.3 Persistence

Timer state is part of the CaseInstance (S3.1). Timers are persisted at every durability checkpoint (S6.2). After a processor restart, all pending timers MUST be reconstituted from the persisted state and scheduled for firing at their original deadlines. Timers whose deadlines have passed during the outage MUST fire immediately on restart, in deadline order.

### 7.4 Simulated Time

Conformance test processors MAY implement simulated time via the `advanceTime` operation (S3.3). Under simulated time, the processor does not use wall-clock time for timer firing -- instead, `advanceTime` fires all timers whose deadline is at or before the specified timestamp.

Production processors MUST use wall-clock time. A production processor that implements `advanceTime` MUST restrict it to administrative or testing contexts and MUST NOT expose it as a normal operational API.

### 7.5 Timer Ordering

When multiple timers fire at the same logical instant (same deadline, or multiple deadlines passed during an outage), the processor MUST fire them in deadline order. Ties (identical deadlines) are broken by timer creation order. This ordering is deterministic and observable via provenance.

---

## 8. Governance Enforcement

This section is normative.

### 8.1 Overview

A Runtime Governed processor (S2.1) enforces governance rules from Layers 1 through 3 at runtime. This section defines the enforcement mechanics. The governance semantics themselves are defined in the Workflow Governance Specification, AI Integration Specification, and Advanced Governance Specification; this section defines how a processor applies them.

### 8.2 Governance Scoping

Governance rules MAY include a `scope` property -- a FEL guard expression evaluated against the evaluation context (Kernel S7). When present, the rule applies only to instances where the scope expression evaluates to `true`. When absent, the rule applies to all instances.

```json
{
  "reviewProtocols": [{
    "tags": ["determination"],
    "protocols": ["independentFirst"],
    "scope": "caseFile.state = 'CA' or caseFile.state = 'NY'"
  }]
}
```

Governance scoping is a core engine concern because the scoping expression participates in the deterministic evaluation -- two conformant processors MUST agree on which governance rules apply to which instances.

### 8.3 Deontic Enforcement Ordering

When the AI Integration layer is active, deontic constraints (AI Integration S4) MUST be evaluated in the following order:

1. **Permissions** -- Is the agent permitted to perform this action? (AI Integration S4.2)
2. **Prohibitions** -- Is the agent prohibited from this action? (AI Integration S4.3)
3. **Obligations** -- Has the agent fulfilled its obligations? (AI Integration S4.4)
4. **Confidence** -- Does the agent's confidence meet the floor? (AI Integration S7)
5. **Volume** -- Has the agent exceeded volume constraints? (AI Integration S4.6)
6. **Sampling** -- Is this action selected for quality review? (Governance S7)

When multiple constraints are violated simultaneously, the processor applies the most restrictive violation action. Violation action severity, from least to most restrictive: `log`, `flag`, `requireReview`, `reject`, `escalateToHuman`.

### 8.4 Delegation Verification

On transitions tagged `determination` (Kernel S4.12), the processor MUST verify that the acting actor has valid delegation authority (Governance S11.4). Verification checks:

1. A delegation record exists for this actor.
2. The delegation has not expired (`expirationDate` is in the future or absent).
3. The delegation has not been revoked (`revokedDate` is absent).
4. The delegation's scope covers this case (impact level, case type, value threshold).
5. If the delegation was sub-delegated, the chain depth does not exceed `maxDelegationDepth` (Governance S11.5).

A determination without valid delegation is a conformance error. The delegation used MUST be referenced in the provenance record.

### 8.5 Hold Management

On entering a state tagged `hold` (Kernel S4.12), the processor:

1. Starts a hold timer using the `expectedDuration` from the Hold Policy (Governance S12.2).
2. Listens for the `resumeTrigger` event declared in the Hold Policy.
3. When the resume trigger arrives, cancels the hold timer and processes the event normally.
4. When the hold timer fires, emits a `$timeout.state` event. The lifecycle handles it like any other timeout (Kernel S4.10).

---

## 9. Explanation Assembly

This section is normative.

### 9.1 Overview

When a transition tagged `adverse-decision` (Kernel S4.12) fires, the processor MUST assemble a structured explanation from provenance. This explanation satisfies the due process requirement (Governance S3) that affected individuals receive an individualized explanation of the decision.

The assembly algorithm is deterministic -- two conformant processors MUST produce the same explanation structure from the same provenance log.

### 9.2 Assembly Algorithm

```
function assembleExplanation(provenanceLog, transition):
    # Step 1: Collect Reasoning tier records for this determination
    reasoning = provenanceLog.filter(
        tier="reasoning",
        relatedTransition=transition.id
    )

    # Step 2: Collect Counterfactual tier records
    counterfactual = provenanceLog.filter(
        tier="counterfactual",
        relatedTransition=transition.id
    )

    # Step 3: Separate positive and negative counterfactuals
    positive = counterfactual.filter(type="positive")
    negative = counterfactual.filter(type="negative")

    # Step 4: Order reasoning elements
    #   Primary sort: rule authority (statute > regulation > policy)
    #   Secondary sort: chronological order within each authority level
    reasoning.sort(
        key=lambda r: (authorityRank(r.authority), r.timestamp)
    )

    # Step 5: Assemble the explanation structure
    return {
        "transitionId": transition.id,
        "determination": transition.tags,
        "reasoning": reasoning,
        "positiveCounterfactual": positive,
        "negativeCounterfactual": negative,
        "assembledAt": now()
    }
```

### 9.3 Authority Ranking

Reasoning elements are ordered by the authority of the rule that produced them:

| Rank | Authority Type | Description |
|------|---------------|-------------|
| 1 | `statute` | Statutory or legislative authority. |
| 2 | `regulation` | Regulatory or administrative rule. |
| 3 | `policy` | Organizational policy or standard operating procedure. |
| 4 | `guideline` | Non-binding guidance or best practice. |

When an authority type is not specified on a reasoning record, it defaults to `policy` (rank 3).

### 9.4 Explanation Structure

The assembled explanation is a JSON structure, not rendered text. Rendering the explanation into a human-readable format (PDF, HTML, plain text, accessible alternative) is the host's responsibility via the ReportRenderer interface (S12.7).

The explanation structure:

| Property | Type | Description |
|----------|------|-------------|
| `transitionId` | string | Identifier of the transition that produced the adverse decision. |
| `determination` | array of string | Semantic tags from the transition. |
| `reasoning` | array of ReasoningRecord | Ordered reasoning elements. |
| `positiveCounterfactual` | array of CounterfactualRecord | What the affected individual could change to alter the outcome. |
| `negativeCounterfactual` | array of CounterfactualRecord | What did NOT affect the outcome (e.g., protected characteristics). |
| `assembledAt` | string (datetime) | ISO 8601 timestamp of assembly. |

---

## 10. Evaluation Modes

This section is normative.

### 10.1 Overview

The Kernel Document MAY declare an `evaluationMode` property on the top-level document. The evaluation mode determines when the processor evaluates transition guards.

### 10.2 Event-Driven Mode (Default)

In `event-driven` mode (the default), transition guards are evaluated only when an explicit event arrives. Case state mutations (`setData` actions, contract validation results) do not trigger guard re-evaluation. This is the standard statechart evaluation model.

### 10.3 Continuous Mode

In `continuous` mode, after any case state mutation -- whether from a `setData` action, a contract validation result, or an external signal -- the processor re-evaluates all guards in the current configuration. If any guard that was previously `false` now evaluates to `true`, the corresponding transition fires. The triggering mutation is recorded in provenance.

**Convergence cap.** To prevent infinite loops (a `setData` in `onEntry` triggers re-evaluation, which fires a transition whose `onEntry` does another `setData`), the processor imposes a convergence cap of **100 re-evaluation cycles** per triggering mutation. If the cap is reached, the processor MUST:

1. Halt re-evaluation for this mutation.
2. Record a `convergenceCapReached` provenance record with the mutation that triggered the cycle.
3. Continue processing subsequent events normally.

Transitions fired during a convergence cycle are committed -- they have already emitted provenance and mutated case state. The cap halts *further re-evaluation*, not the effects of transitions that already fired.

Timer-driven mutations (timer expiry firing a `$timeout.*` event whose actions include `setData`) trigger re-evaluation in continuous mode, subject to the same convergence cap. The `$timeout.*` event is the triggering mutation for the re-evaluation cycle.

The convergence cap value (100) matches Formspec's processing model convergence behavior for consistency across the ecosystem.

### 10.4 Mode Declaration

```json
{
  "$wosKernel": "1.0",
  "evaluationMode": "continuous",
  "lifecycle": { ... }
}
```

When `evaluationMode` is absent, the default is `event-driven`. A conformant processor MUST support both modes.

---

## 11. Multi-Version Coexistence

This section is normative.

### 11.1 Simultaneous Versions

A conformant processor MUST support multiple Kernel Document versions simultaneously. New instances use the version specified at creation time. Running instances remain on their creation-time version (Kernel S9.6).

The processor MUST NOT apply a newer definition version to a running instance unless an explicit `migrate` operation is performed.

### 11.2 Migration

Instance migration changes the governing Kernel Document version. The `migrate` operation (S3.3):

1. **State validation.** Validates that the new definition contains all states the instance is currently in. If the configuration includes a state that does not exist in the new definition, the migration fails with a `stateNotFound` error.

2. **Case state transformation.** Applies a migration map that declares field renames, removals, type coercions, and default values for new fields:

    ```json
    {
      "fieldRenames": { "old_name": "new_name" },
      "fieldRemovals": ["deprecated_field"],
      "fieldDefaults": { "new_field": "default_value" },
      "fieldCoercions": { "amount": "number" }
    }
    ```

3. **Provenance.** Records the migration in provenance with the old version, new version, migration map applied, and any case state transformations performed.

4. **Version update.** Updates the instance's `definitionVersion` to the new version.

If any step fails, the migration is aborted and the instance remains on its original version. Migration is atomic -- partial migrations MUST NOT be persisted.

When the Kernel Document references Formspec Definitions via `contractRef`, the migration operation SHOULD consult the Formspec Changelog (Changelog S4) for the referenced definitions. The changelog provides structured change objects that describe field additions, removals, renames, and type changes between versions -- these map directly to the migration map's `fieldRenames`, `fieldRemovals`, and `fieldDefaults` operations.

---

## 12. Host Interfaces

This section is normative.

The processor expects its host to provide implementations of the following interfaces. Each interface is a named behavioral contract with required operations and error semantics. These are spec-level interface definitions -- implementations map them to their language's type system (Rust traits, Java interfaces, TypeScript abstract classes, Python protocols).

### 12.1 InstanceStore

Persists CaseInstance documents between events.

| Operation | Input | Output | Required | Description |
|-----------|-------|--------|----------|-------------|
| `load` | instanceId: string | CaseInstance | REQUIRED | Load an instance by ID. Error if not found. |
| `save` | instance: CaseInstance | (none) | REQUIRED | Durably persist an instance. MUST be atomic. |
| `listByState` | stateId: string | array of instanceId | OPTIONAL | List instances with the given state in their configuration. |
| `listByDefinition` | definitionUrl: string, definitionVersion: string | array of instanceId | OPTIONAL | List instances governed by the given definition version. |

`load` and `save` are REQUIRED for all conformance profiles. `listByState` and `listByDefinition` are OPTIONAL query operations -- they enable administrative and migration workflows but are not needed for core event processing.

Error conditions: `instanceNotFound`, `storageUnavailable`, `concurrencyConflict` (when two processors attempt to save the same instance simultaneously).

**Provenance log storage.** The provenance log is a separate append-only store referenced by the CaseInstance's `provenancePosition` cursor. The processor MUST NOT embed the full provenance log in the CaseInstance document -- provenance logs grow unboundedly and would make instance serialization progressively more expensive. The `provenancePosition` field on CaseInstance records how many provenance entries have been durably persisted, enabling the processor to resume provenance collection after a crash.

### 12.2 DocumentResolver

Loads WOS documents (kernel, governance, sidecars) from storage.

| Operation | Input | Output | Description |
|-----------|-------|--------|-------------|
| `resolveKernel` | url: string, version: string | KernelDocument | Resolve a Kernel Document by URL and version. |
| `resolveGovernance` | url: string, version: string | GovernanceDocument | Resolve a Governance Document. |
| `resolveSidecar` | url: string, anchorDate: string (optional) | SidecarDocument | Resolve a sidecar document. When `anchorDate` is provided, used for temporal parameter resolution (Governance S13). |

Error conditions: `documentNotFound`, `versionNotFound`, `resolverUnavailable`.

### 12.3 ContractValidator

Validates data against a Formspec Definition or JSON Schema contract.

| Operation | Input | Output | Description |
|-----------|-------|--------|-------------|
| `validate` | contractRef: string, data: object | ValidationResult | Validate data against the referenced contract. Returns `{ valid: boolean, errors: array }`. |

For Formspec bindings, the ContractValidator MUST delegate to a Formspec-conformant processor (Core S1.4). The WOS processor provides orchestration context; the Formspec processor provides Definition evaluation.

Error conditions: `contractNotFound`, `processorUnavailable`.

### 12.4 ExternalService

Fulfills `invokeService` actions (Kernel S9.2).

| Operation | Input | Output | Description |
|-----------|-------|--------|-------------|
| `invoke` | serviceRef: string, input: object, idempotencyKey: string (optional), timeout: duration (optional) | object | Invoke the referenced service. Returns the service response. |

The processor MUST pass the `idempotencyKey` to the service when provided (Kernel S9.3). The service is responsible for deduplication at the service level; the processor is responsible for deduplication at the instance level.

Error conditions: `serviceNotFound`, `serviceTimeout`, `serviceError` (with error payload).

### 12.5 AccessControl

Controls which actors can perform which operations.

| Operation | Input | Output | Description |
|-----------|-------|--------|-------------|
| `canRead` | actorId: string, fieldPath: string | boolean | Whether the actor can read the specified case state field. |
| `canTransition` | actorId: string, transition: Transition | boolean | Whether the actor can trigger this transition. |
| `canDelegate` | delegatorId: string, delegateId: string, scope: DelegationScope | boolean | Whether the delegator can delegate authority to the delegate within the given scope. |

A processor MAY use a permissive default implementation (all operations return `true`) for single-user or testing deployments. Production deployments with multiple actors SHOULD use a restrictive implementation.

### 12.6 ProvenanceSigner

Signs and verifies provenance records for cross-organization trust.

| Operation | Input | Output | Description |
|-----------|-------|--------|-------------|
| `sign` | record: ProvenanceRecord | SignedRecord | Attach a cryptographic signature to a provenance record. |
| `verify` | signedRecord: SignedRecord | boolean | Verify that a signed record's signature is valid and the content has not been tampered with. |

Single-organization deployments MAY use a no-op implementation. Cross-organization deployments (Federation Profile) MUST use a signing implementation.

### 12.7 ReportRenderer

Renders provenance and case state into human-readable formats.

| Operation | Input | Output | Description |
|-----------|-------|--------|-------------|
| `renderExplanation` | explanation: ExplanationStructure, template: string | rendered output | Render an assembled explanation (S9) into a human-readable format. |
| `renderAudit` | provenanceLog: array, format: string | rendered output | Render an audit trail into the specified format. |

The output format is implementation-defined. Common formats: PDF, HTML, plain text, accessible HTML with ARIA annotations.

### 12.8 EventQueue

Manages the per-instance event queue for serial processing (S4.1).

| Operation | Input | Output | Description |
|-----------|-------|--------|-------------|
| `enqueue` | instanceId: string, event: Event | (none) | Add an event to the instance's processing queue. |
| `dequeue` | instanceId: string | Event or empty | Remove and return the next event for processing. Returns empty if the queue is drained. |
| `peek` | instanceId: string | Event or empty | Return the next event without removing it. |

The EventQueue is a logical abstraction -- implementations MAY use an in-process queue, a message broker, or a database-backed queue. The only requirement is FIFO ordering per instance (S4.1). The queue MUST be durable: events enqueued but not yet processed MUST survive processor restarts.

Error conditions: `queueUnavailable`.

---

## 13. Security Model

This section is normative.

### 13.1 Engine Isolation

The evaluation engine MUST NOT have direct network access. All external communication flows through the ExternalService interface (S12.4). This constraint ensures the engine is a pure computational component: given the same inputs (documents, events, host interface responses), it produces the same outputs. Network access would introduce non-determinism.

### 13.2 Expression Sandboxing

FEL expressions are inherently sandboxed -- FEL has no I/O operations, no network access, no filesystem access, and no ability to invoke external services (Formspec Core S3). This sandboxing is a property of FEL itself, not an implementation requirement on the processor.

### 13.3 Data Protection

Case state containing personally identifiable information (PII) SHOULD be encrypted at rest by the host (via the InstanceStore implementation), not by the engine. The engine processes case state in memory; the host is responsible for storage-level encryption.

### 13.4 Provenance Immutability

Provenance records SHOULD be immutable at the storage level. The host SHOULD implement provenance storage as write-once (append-only), preventing modification or deletion of existing records. This is a SHOULD, not a MUST, because some regulatory frameworks require provenance expungement under specific legal orders.

---

## 14. Relationship-Triggered Events

This section is normative.

### 14.1 Overview

Case relationships (Kernel S5.5) are metadata declarations. This section defines standard kernel-generated events that the processor emits when related cases change state, enabling cross-case coordination without violating the deterministic evaluation constraint.

### 14.2 Standard Events

| Event | Trigger | Payload |
|-------|---------|---------|
| `$related.stateChanged` | A related case transitions to a new state. | `{ relatedInstanceId, fromState, toState, event }` |
| `$related.resolved` | A related case reaches a top-level final state. | `{ relatedInstanceId, finalState, resolution }` |
| `$related.holdReleased` | A related case exits a state tagged `hold`. | `{ relatedInstanceId, holdState, releaseEvent }` |

These events follow the `$` prefix convention for kernel-generated events (Kernel S4.10). They are processed by the lifecycle like any other event: if a matching transition exists, it fires; otherwise, the event is recorded in provenance.

### 14.3 Generation

The processor generates relationship-triggered events based on the declaring case's `caseRelationships` (Kernel S5.5). When a case declares a relationship to another case, the processor monitors the related case for state changes and generates the appropriate events.

The monitoring mechanism is implementation-defined. Options include polling, event bus subscription, or direct callback. The processor MUST guarantee that relationship-triggered events are delivered at-least-once and are subject to the same deduplication rules as external events (S4.3).

### 14.4 Cross-Instance Isolation

Relationship-triggered events carry data about the related case (instance ID, state names) but MUST NOT carry the related case's case state data. FEL guard expressions in the receiving case MUST NOT reference the related case's case state (Kernel S5.5). The receiving case can only observe that a state change occurred, not the data that caused it. Cross-case data sharing, when needed, flows through the ExternalService interface.

### 14.5 Cascade Prevention

Relationship-triggered events can cause cascading chains: case A's state change triggers `$related.stateChanged` in case B, whose resulting transition triggers `$related.stateChanged` in case C, and so on. Unbounded cascading could cause infinite loops in cyclically related cases.

The processor MUST track the cascade depth for each chain of relationship-triggered events. The maximum depth is governed by the `maxRelationshipEventDepth` property on the Kernel Document (Kernel S4.10), which defaults to **3**. When a `$related.*` event would exceed the depth cap, the processor MUST:

1. NOT generate the event.
2. Record a `relationshipDepthCapReached` provenance record on the case that would have received the event, including the cascade chain (list of instance IDs and events that led to this point).

The depth counter resets for each externally-originated event. Only `$related.*` events increment the depth counter -- other kernel-generated events (`$timeout.*`, `$join`, `$error`, `$compensation.complete`) do not participate in cascade depth tracking.

---

## References

### Normative References

- [WOS Kernel] Formspec Working Group, "WOS Kernel Specification v1.0".
- [WOS Lifecycle Detail] Formspec Working Group, "WOS Lifecycle Detail Companion v1.0".
- [WOS Governance] Formspec Working Group, "WOS Workflow Governance Specification v1.0".
- [WOS AI Integration] Formspec Working Group, "WOS AI Integration Specification v1.0".
- [Formspec Core] Formspec Working Group, "Formspec Core Specification v1.0".
- [RFC 2119] Bradner, S., "Key words for use in RFCs to Indicate Requirement Levels", BCP 14, RFC 2119, March 1997.
- [RFC 8174] Leiba, B., "Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words", BCP 14, RFC 8174, May 2017.
- [RFC 8259] Bray, T., "The JavaScript Object Notation (JSON) Data Interchange Format", STD 90, RFC 8259, December 2017.
- [RFC 3986] Berners-Lee, T., Fielding, R., and L. Masinter, "Uniform Resource Identifier (URI): Generic Syntax", STD 66, RFC 3986, January 2005.

### Informative References

- [ADR-0057] "WOS Core vs. Implementation Boundary", Architecture Decision Record, 2026.
- [Temporal.io] Temporal Technologies, "Temporal Workflow Engine".
- [PROV-DM] W3C, "PROV-DM: The PROV Data Model", W3C Recommendation, 2013.
