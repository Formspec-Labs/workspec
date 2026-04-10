---
title: WOS Lifecycle Detail Companion
version: 1.0.0-draft.1
date: 2026-04-09
status: draft
---

# WOS Lifecycle Detail Companion v1.0

**Version:** 1.0.0-draft.1
**Date:** 2026-04-09
**Editors:** Formspec Working Group
**Companion to:** WOS Kernel Specification v1.0

---

## Abstract

The WOS Lifecycle Detail Companion elaborates the lifecycle semantics defined in the WOS Kernel Specification. The kernel defines the deterministic evaluation algorithm, state types, transitions, fork/join, and the compensation seam. This companion provides the detailed execution algorithms that Kernel Complete processors need: the full compensation execution algorithm (reverse ordering, pivot steps, forward/backward recovery), advanced parallel execution semantics (region synchronization, nested parallelism, event routing, history states), the transition evaluation algorithm as pseudocode, timer semantics (creation, cancellation, reset, parallel interaction), and SCXML interoperability mapping.

This is a companion specification, not a layer. It elaborates kernel semantics without adding new concepts, seams, or document types. A Kernel Structural processor does not need this document. A Kernel Complete processor SHOULD implement the algorithms defined here.

WOS MUST NOT alter core Formspec processing semantics. WOS processors MUST delegate Formspec Definition evaluation to a Formspec-conformant processor (Core S1.4).

---

## Status of This Document

This document is a **draft specification**. It is a companion to the WOS Kernel Specification v1.0 that elaborates lifecycle execution semantics. It does not define new document types, kernel seams, or governance structures. Implementors building Kernel Complete processors are encouraged to use this document as the normative reference for execution algorithms.

---

## 1. Introduction

### 1.1 Purpose

The WOS Kernel Specification (Kernel S4) defines the lifecycle topology and its deterministic evaluation algorithm at the level needed for document authoring and structural validation. This companion provides the implementation-level detail that runtime processors need:

- **Compensation:** The kernel defines the seam (Kernel S9.5). This companion defines the execution algorithm.
- **Parallel execution:** The kernel defines fork/join basics (Kernel S4.8). This companion defines region synchronization, nested parallelism, and event routing.
- **Transition evaluation:** The kernel defines the algorithm in prose (Kernel S4.6, S4.7). This companion provides pseudocode.
- **History states:** The kernel mentions compound states. This companion defines shallow and deep history semantics.
- **Timers:** The kernel defines timeout categories (Kernel S9.7). This companion defines timer lifecycle interaction.
- **SCXML mapping:** This companion defines how WOS kernel documents map to and from W3C SCXML.

### 1.2 Scope

**Within scope:** compensation execution algorithm; advanced parallel semantics; transition evaluation pseudocode; history state semantics; timer lifecycle; SCXML interoperability mapping.

**Out of scope:** new document types; new kernel seams; governance structures (Layers 1-3); constraint zone semantics (Layer 3: Advanced Governance S4).

### 1.3 Notational Conventions

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [BCP 14][RFC 2119] [RFC 8174] when, and only when, they appear in ALL CAPITALS, as shown here.

Pseudocode in this document uses Python-like syntax for readability. It is not executable code; it is a precise specification of algorithmic behavior.

---

## 2. Transition Evaluation Algorithm

This section is normative.

### 2.1 Overview

The kernel defines the lifecycle as a deterministic pure function (Kernel S4.2). This section provides the complete algorithm as pseudocode. Two conformant Kernel Complete processors given the same document and the same event sequence MUST produce the same state transitions.

### 2.2 Configuration

A **configuration** is the set of currently active states. In a workflow without parallel states, the configuration contains exactly one state. In a workflow with parallel states, the configuration contains one state per active region.

### 2.3 Algorithm: Process Event

```
function processEvent(configuration, event, document):
    # Step 1: Collect candidate transitions
    candidates = []
    for state in configuration:
        for transition in state.transitions:
            if transition.event == event.type:
                candidates.append((state, transition))

    # Step 2: Evaluate guards in document order
    # Document order = order of transitions array within each state,
    # states evaluated in configuration order (innermost first for nested)
    for (state, transition) in candidates:
        if transition.guard is None:
            fire(configuration, state, transition, event, document)
            return
        result = evaluateFEL(transition.guard, buildContext(configuration, event))
        if result == true:
            fire(configuration, state, transition, event, document)
            return

    # Step 3: No matching transition — record in provenance, no state change
    recordUnmatchedEvent(event, configuration)
```

### 2.4 Algorithm: Fire Transition

```
function fire(configuration, sourceState, transition, event, document):
    # Step 1: Compute exit path (innermost to outermost)
    exitPath = computeExitPath(sourceState, transition.target, document)

    # Step 2: Execute onExit actions, innermost first
    for state in exitPath:
        for action in state.onExit:
            executeAction(action, buildContext(configuration, event))

    # Step 3: Execute transition actions
    for action in transition.actions:
        executeAction(action, buildContext(configuration, event))

    # Step 4: Compute entry path (outermost to innermost)
    entryPath = computeEntryPath(sourceState, transition.target, document)

    # Step 5: Execute onEntry actions, outermost first
    for state in entryPath:
        for action in state.onEntry:
            executeAction(action, buildContext(configuration, event))
        if state.type == "parallel":
            activateAllRegions(state, configuration)
        if state.type == "compound":
            # If this compound state is the transition target, enter its initialState.
            # If a descendant of this compound state is the target, do NOT enter
            # initialState -- the entry path already includes the specific descendant.
            if state == transition.target or not isAncestor(state, transition.target, document):
                enterInitialSubstate(state, configuration)

    # Step 6: Update configuration
    configuration.remove(sourceState)
    configuration.add(transition.target)

    # Step 7: Emit provenance
    emitTransitionProvenance(sourceState, transition, event)
```

### 2.5 Exit and Entry Path Computation

The **exit path** is the sequence of states from the source state up to (but not including) the Least Common Ancestor (LCA) of the source and target. States are ordered innermost first.

The **entry path** is the sequence of states from the LCA down to the target state. States are ordered outermost first.

```
function computeExitPath(source, target, document):
    lca = leastCommonAncestor(source, target, document)
    path = []
    current = source
    while current != lca:
        path.append(current)
        current = parent(current, document)
    return path  # innermost first

function computeEntryPath(source, target, document):
    lca = leastCommonAncestor(source, target, document)
    path = []
    current = target
    while current != lca:
        path.insert(0, current)  # build outermost first
        current = parent(current, document)
    return path
```

### 2.6 Nested State Transitions

When a transition crosses nesting boundaries (e.g., from a substate of one compound state to a substate of another), the exit path includes all states being exited up to the LCA, and the entry path includes all states being entered down from the LCA. This ensures all `onExit` and `onEntry` actions fire in the correct order.

---

## 3. History States

This section is normative.

### 3.1 Overview

History states record the last active configuration within a compound state, enabling resumption after suspension. The kernel defines compound states with an `initialState` property. This companion defines how history overrides the initial state on reentry.

### 3.2 Shallow History

When a compound state declares `historyState: "shallow"`, the processor records the last active **direct substate** when the compound state is exited. On subsequent entry to the compound state, execution resumes in the recorded substate rather than the `initialState`.

```
function enterCompoundState(compoundState, configuration):
    if compoundState.historyState == "shallow" and hasHistory(compoundState):
        target = getShallowHistory(compoundState)
    else:
        target = compoundState.initialState
    enterState(target, configuration)
```

### 3.3 Deep History

When a compound state declares `historyState: "deep"`, the processor records the **full active state configuration** within the compound state at all nesting levels. On subsequent entry, the entire nested configuration is restored.

```
function enterCompoundStateDeep(compoundState, configuration):
    if compoundState.historyState == "deep" and hasHistory(compoundState):
        targets = getDeepHistory(compoundState)
        for target in targets:
            configuration.add(target)
            executeOnEntry(target)
    else:
        enterState(compoundState.initialState, configuration)
```

### 3.4 History Clearing

History is cleared when the compound state's parent is exited. If a compound state is within a parallel region and that region is cancelled, the history for the compound state is cleared.

---

## 4. Advanced Parallel Execution

This section is normative.

### 4.1 Region Activation

When a parallel state is entered (Kernel S4.8), all regions are activated simultaneously. Each region begins in its `initialState`. The processor MUST add the initial state of every region to the configuration atomically -- no region may begin processing events before all regions are initialized.

```
function activateAllRegions(parallelState, configuration):
    for regionName, region in parallelState.regions:
        initialState = region.states[region.initialState]
        configuration.add(initialState)
        executeOnEntry(initialState)
```

### 4.2 Event Routing to Regions

When an event occurs and the configuration includes states within a parallel state's regions, the event is offered to each region independently. Each region evaluates its own transitions. Multiple regions MAY fire transitions from the same event -- this is concurrent execution, not a conflict.

```
function processEventInParallel(parallelState, event, configuration):
    for regionName, region in parallelState.regions:
        activeInRegion = getActiveStatesInRegion(region, configuration)
        for state in activeInRegion:
            candidates = matchTransitions(state, event)
            # Evaluate guards in document order, first match wins (Kernel S4.6)
            for candidate in candidates:
                if evaluateGuard(candidate.guard, buildContext(configuration, event)):
                    fire(configuration, state, candidate, event)
                    break
```

### 4.3 Join Semantics

The join condition for a parallel state depends on the `cancellationPolicy` (Kernel S4.4):

- **`wait-all`:** The `$join` event (Kernel S4.10) fires when every region has an active state of type `final`. The parallel state then evaluates its own outgoing transitions.
- **`cancel-siblings`:** When any region reaches a final state, the `$join` event fires. All other regions are cancelled: their active states receive `onExit` actions (innermost first), and their states are removed from the configuration.
- **`fail-fast`:** When any region reaches an error final state (a final state entered via an error transition), all other regions are cancelled immediately.

### 4.4 Region Cancellation

When a region is cancelled (by `cancel-siblings`, `fail-fast`, or an explicit transition out of the parallel state):

1. For each active state in the region, execute `onExit` actions innermost first.
2. Cancel any active timers scoped to the region.
3. If any active state has `historyState`, clear its history (the cancellation invalidates the recorded configuration).
4. Remove all region states from the configuration.
5. Emit a provenance record for the cancellation.

### 4.5 Nested Parallelism

Parallel states MAY be nested within compound states, and compound states MAY be nested within parallel regions. The algorithms in S2 and S4 apply recursively. The configuration may contain states at arbitrary nesting depth.

### 4.6 Transitions Exiting a Parallel State

A transition on the parallel state itself (not on a region substate) exits the entire parallel state. All regions are cancelled as described in S4.4, and the transition fires normally.

---

## 5. Compensation Execution Algorithm

This section is normative.

### 5.1 Overview

The kernel defines the compensation seam: actions MAY declare a `compensatingAction`, and scopes MAY be marked `compensable: true` (Kernel S9.5). This section defines the algorithm for executing compensation when a compensable scope fails.

### 5.2 Compensation Log

A Kernel Complete processor MUST maintain a **compensation log** for each compensable scope. The log records, in order, every action that completed successfully within the scope and has a declared `compensatingAction`. The log is append-only during forward execution.

### 5.3 Algorithm: Execute Compensation

When a `compensate` action is executed for a scope (triggered by an error transition or explicit action), the processor executes the compensation sequence:

```
function executeCompensation(scope, compensationLog):
    # Step 1: Identify the pivot step
    # The pivot is the action that failed, triggering compensation.
    # It does NOT receive compensation (it did not complete).
    pivotStep = compensationLog.getFailedAction()

    # Step 2: Build the compensation sequence in reverse order
    completedActions = compensationLog.getCompletedActions()
    compensationSequence = reverse(completedActions)

    # Step 3: Execute compensation actions in reverse order
    for originalAction in compensationSequence:
        compensatingAction = originalAction.compensatingAction
        try:
            executeAction(compensatingAction)
            emitCompensationProvenance(originalAction, compensatingAction, "success")
        except CompensationError as e:
            # Compensation failure is itself recorded
            emitCompensationProvenance(originalAction, compensatingAction, "failed", e)
            # Compensation continues -- best effort for remaining actions
            continue

    # Step 4: Emit completion event
    emitKernelEvent("$compensation.complete", scope)
```

### 5.4 Reverse Ordering

Compensation actions execute in the **reverse of forward completion order**. If actions A, B, C completed in that order, compensation runs C's compensating action, then B's, then A's. This preserves semantic consistency -- later actions may depend on earlier ones, so they must be undone first.

### 5.5 The Pivot Step

The **pivot step** is the action whose failure triggered compensation. It does NOT receive compensation because it did not complete successfully. The pivot step's failure is recorded in provenance separately from the compensation sequence.

### 5.6 Forward Recovery vs. Backward Recovery

| Recovery Mode | Behavior | When to Use |
|---------------|----------|-------------|
| **Backward recovery** | Execute compensation for all completed actions in reverse order, then transition to an error or alternative state. | Default. Appropriate when the failed scope's effects must be fully reversed. |
| **Forward recovery** | Skip compensation. Retry the failed action or take an alternative path that does not require reversal. | When partial completion is acceptable and the failed action can be retried or bypassed. |

The recovery mode is declared on the compensable scope:

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `compensable` | boolean | `false` | Whether this scope supports compensation. |
| `recoveryMode` | enum | `backward` | `backward` or `forward`. |
| `onCompensationFailure` | enum | `continue` | `continue` (best effort) or `halt` (stop compensation on first failure). |

### 5.7 Compensation and Parallel States

When compensating a parallel state scope, each region is compensated independently. The compensation order within each region is reverse completion order. Regions are compensated concurrently (the same parallelism semantics as forward execution).

### 5.8 Nested Compensation

If a compensable scope is nested within another compensable scope, compensation of the inner scope does NOT automatically trigger compensation of the outer scope. Each scope is compensated independently. The outer scope's compensation log includes the inner scope as a single entry; if the inner scope's compensation fails, the outer scope records that failure.

### 5.9 Compensation Triggering

Compensation is triggered when an action within a compensable scope fails and the scope's `recoveryMode` is `backward`. The triggering mechanism:

1. During forward execution of a compensable scope, the processor appends each completed action (that has a `compensatingAction`) to the scope's compensation log.
2. When an action fails, the processor checks the scope's `recoveryMode`.
3. If `backward`: invoke `executeCompensation(scope, compensationLog)` as defined in S5.3. After compensation completes, the `$compensation.complete` event fires and the lifecycle processes it like any other event (Kernel S4.10) — if a transition matches, it fires; otherwise the event is recorded in provenance.
4. If `forward`: the compensation log is discarded. The processor retries the failed action or evaluates alternative transitions from the current state. No compensation actions execute.

---

## 6. Timer Semantics

This section is normative.

### 6.1 Overview

The kernel defines five timeout categories (Kernel S9.7) and the `startTimer`/`cancelTimer` actions (Kernel S9.2). This section defines the detailed timer lifecycle and its interaction with the state machine.

### 6.2 Timer Creation

A `startTimer` action creates a durable timer with:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `timerId` | string | REQUIRED | Unique timer identifier within the workflow instance. |
| `duration` | string (ISO 8601) | CONDITIONAL | Relative duration. One of `duration` or `deadline` MUST be specified. |
| `deadline` | string (ISO 8601) | CONDITIONAL | Absolute deadline. |
| `event` | string | REQUIRED | Event to emit when the timer fires. |

When the timer fires, it emits the declared event as a kernel-generated event with the `$timeout.` prefix (Kernel S4.10). The event is processed by the lifecycle like any other event.

### 6.3 Timer Cancellation

A `cancelTimer` action cancels a running timer. If the timer has already fired, the cancellation has no effect. If the timer does not exist, the cancellation is a no-op (not an error).

### 6.4 Timer Reset on Reentry

When a state with a `startTimer` action in its `onEntry` is exited and then re-entered (Kernel S4.11), the original timer is cancelled (if still active) and a new timer is created. This prevents stale timers from firing for states that have been re-entered.

### 6.5 Timers and Parallel States

Timers scoped to a parallel region are cancelled when the region is cancelled (S4.4). A timer created in region A does not affect region B. Timer events are routed to the region that created them.

### 6.6 Timer Durability

Per Kernel S9.1 (G4: Durable Timers), timers MUST survive process restarts, fire within tolerance, and consume no runtime resources while waiting. The tolerance for timer firing is implementation-defined but SHOULD be documented. A tolerance greater than the timer's duration is a conformance violation.

### 6.7 Timer Provenance

Timer creation, cancellation, and firing each produce provenance records:

| Record Type | When |
|-------------|------|
| `timer.created` | A `startTimer` action executes. |
| `timer.cancelled` | A `cancelTimer` action executes or a timer is cancelled by region cancellation or state reentry. |
| `timer.fired` | A timer fires and emits its event. |

---

## 7. SCXML Interoperability Mapping

This section is informative.

### 7.1 Purpose

This section defines how WOS Kernel Documents map to and from W3C SCXML documents, enabling interoperability with existing SCXML-based workflow engines. The mapping is bidirectional: a WOS document can be translated to SCXML for execution, and an SCXML document can be imported as a WOS kernel document (with some loss of WOS-specific metadata).

### 7.2 State Type Mapping

| WOS Kernel | SCXML | Notes |
|------------|-------|-------|
| `atomic` | `<state>` (no child states) | Direct mapping. |
| `compound` | `<state>` (with child `<state>` elements) | WOS `initialState` maps to SCXML `initial` attribute. |
| `parallel` | `<parallel>` | WOS regions map to child `<state>` elements within `<parallel>`. |
| `final` | `<final>` | Direct mapping. |

### 7.3 Transition Mapping

| WOS Kernel | SCXML | Notes |
|------------|-------|-------|
| `event` | `event` attribute | Direct mapping. |
| `target` | `target` attribute | Direct mapping. |
| `guard` | `cond` attribute | FEL expression must be translated to the SCXML datamodel's expression language. |
| `actions` | `<script>` or executable content | WOS actions map to SCXML executable content. `setData` maps to `<assign>`. `emitEvent` maps to `<send>`. `startTimer` maps to `<send>` with delay. |
| `tags` | No SCXML equivalent. | Tags are WOS-specific metadata. Dropped on export, ignored on import. |

### 7.4 Action Mapping

| WOS Action | SCXML Element | Notes |
|------------|---------------|-------|
| `setData` | `<assign>` | `path` maps to `location`, `value` maps to `expr`. |
| `emitEvent` | `<send>` | `eventType` maps to `event`, `data` maps to content. |
| `startTimer` | `<send>` with `delay`/`delayexpr` | Timer semantics differ: SCXML `<send>` with delay is less structured than WOS durable timers. |
| `cancelTimer` | `<cancel>` | `timerId` maps to `sendid`. |
| `log` | `<log>` | Direct mapping. |
| `createTask` | No SCXML equivalent. | Task creation is WOS-specific. |
| `invokeService` | `<invoke>` | `serviceRef` maps to `src` or `type`. |

### 7.5 History State Mapping

| WOS Kernel | SCXML | Notes |
|------------|-------|-------|
| `historyState: "shallow"` | `<history type="shallow">` | Direct mapping. |
| `historyState: "deep"` | `<history type="deep">` | Direct mapping. |

### 7.6 Cancellation Policy Mapping

WOS `cancellationPolicy` has no direct SCXML equivalent. SCXML `<parallel>` always uses `wait-all` semantics. The `cancel-siblings` and `fail-fast` policies require SCXML extensions or post-processing.

### 7.7 Limitations

The following WOS kernel concepts have no SCXML equivalent and are dropped on export:

- Semantic transition tags (`tags`)
- Cancellation policies other than `wait-all`
- Impact level classification
- Case file schema
- Provenance configuration
- Contract references
- Milestone conditions (SCXML has no milestone concept)
- Kernel-generated events with `$` prefix (must be renamed)

SCXML concepts not used by WOS:

- `<script>` executable content (WOS uses typed actions)
- ECMAScript/XPath expressions (WOS uses FEL)
- `<invoke>` platform-specific type identifiers
- `<donedata>` (WOS uses case state)

---

## References

### Normative References

- [WOS Kernel] Formspec Working Group, "WOS Kernel Specification v1.0".
- [Formspec Core] Formspec Working Group, "Formspec Core Specification v1.0".

### Informative References

- [SCXML] W3C, "State Chart XML (SCXML): State Machine Notation for Control Abstraction", W3C Recommendation, September 2015.
- [Harel1987] Harel, D., "Statecharts: a visual formalism for complex systems", Science of Computer Programming, 8(3), 1987.
- [PROV-DM] W3C, "PROV-DM: The PROV Data Model", W3C Recommendation, April 2013.
