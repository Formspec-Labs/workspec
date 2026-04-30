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

The WOS Kernel Specification defines the minimal orchestration substrate for the Workflow Orchestration Standard (WOS). A Kernel Document -- itself a JSON document -- declares a workflow's lifecycle topology (states, transitions, events, milestones), case state model (typed data with append-only mutation history), actor model (human and system), impact level classification, contract validation interface, provenance Facts tier, durable execution guarantees, and six named extension seams (per ADR 0077: `actorExtension`, `contractHook`, `provenanceLayer`, `lifecycleHook`, `custodyHook`, `extensions` / `x-` keys; enumerated normatively in §10). The kernel is self-sufficient: a kernel-only deployment orchestrates workflows without requiring any governance layer.

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
6. **Every normative claim is testable.** A behavior described in normative prose MUST be reducible to a conformance test, a lint rule, or a schema constraint that a processor can pass or fail. Prose that no test can falsify is design intent, not specification — and is moved to non-normative commentary or deleted. This principle applies recursively to higher-layer companions: see Governance §6.1 and AI Integration §1.2 for layer-specific obligations.

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
| `type` | enum | REQUIRED | `atomic`, `compound`, `parallel`, `foreach`, or `final`. |
| `onEntry` | array of Action | OPTIONAL | Actions executed on state entry. |
| `onExit` | array of Action | OPTIONAL | Actions executed on state exit. |
| `transitions` | array of Transition | OPTIONAL | Outgoing transitions from this state. |
| `tags` | array of string | OPTIONAL | Semantic tags for governance attachment via `lifecycleHook` (S10.4). |
| `initialState` | string | CONDITIONAL | REQUIRED when `type` = `compound`. MUST NOT appear on `atomic`, `parallel`, or `final` states. |
| `states` | map of State | CONDITIONAL | REQUIRED (non-empty) when `type` = `compound`. MUST NOT appear on `atomic`, `parallel`, or `final` states. |
| `regions` | map of Region | CONDITIONAL | REQUIRED (non-empty) when `type` = `parallel`. MUST NOT appear on `atomic`, `compound`, or `final` states. |
| `cancellationPolicy` | enum | CONDITIONAL | Permitted only when `type` = `parallel`: `cancel-siblings`, `wait-all`, or `fail-fast`. Default: `wait-all`. MUST NOT appear on `atomic`, `compound`, or `final` states. |
| `historyState` | enum | CONDITIONAL | Permitted only when `type` = `compound`: `shallow` or `deep` (S4.14). MUST NOT appear on `atomic`, `parallel`, or `final` states. |

**Atomic states** have no substates. They MUST NOT declare `initialState`, `states`, `regions`, `cancellationPolicy`, or `historyState`.

**Compound states** contain substates with a designated `initialState`. When entered, execution proceeds to the initial substate. A compound state MUST declare `initialState` and a non-empty `states` map.

**Parallel states** contain named regions executing concurrently. A parallel state is not exited until all regions reach a final state, unless the `cancellationPolicy` overrides this behavior. A parallel state MUST declare a non-empty `regions` map and MUST NOT declare `initialState` or `states`.

**Final states** indicate completion of the enclosing scope. A top-level final state indicates workflow completion. Final states MUST NOT have outgoing transitions and MUST NOT declare `initialState`, `states`, `regions`, `cancellationPolicy`, or `historyState`. A final state MAY carry an `outcomeCode` — a machine-readable string that allows downstream systems to branch on terminal outcome without parsing state names or tags. `outcomeCode` MUST NOT duplicate any entry in `tags`.

The structural constraints above are enforced by the Kernel JSON Schema (`schemas/wos-workflow.schema.json`) via conditional `allOf` blocks on the `State` definition. A Kernel Structural processor MUST reject any document that violates them. The "final states MUST NOT have outgoing transitions" rule remains a semantic constraint enforced by a Kernel Complete processor (see S13.3).

#### 4.3.1 ForEach States

A **ForEach state** runs an inline `body` State once per element of a bounded collection. ForEach is a structural primitive — distinct from compound (single nested machine) and parallel (fixed concurrent regions) — because the branch count is *runtime-derived* from a FEL expression.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `collection` | string (FEL) | REQUIRED | FEL expression evaluated against case state at entry. MUST evaluate to a bounded array; unbounded streams are rejected. Each element drives one iteration. |
| `body` | State | REQUIRED | Inline State executed once per iteration. MAY be `atomic`, `compound`, or `parallel` (nested ForEach is permitted but discouraged). |
| `itemVariable` | string | OPTIONAL | Case-state binding name for the current iteration's item. Defaults to `$item`. |
| `indexVariable` | string | OPTIONAL | Case-state binding name for the current iteration's zero-based index. Defaults to `$index`. |
| `concurrency` | integer or null | OPTIONAL | Maximum number of items processed concurrently. Positive integer for bounded concurrency; `null` for unbounded (processor decides). MUST be at least 1 when present. Sequential canonical semantics treat `concurrency` as advisory; processors that implement parallel iteration honor the bound. Processors MAY refuse unbounded concurrency on rights-impacting workflows. |
| `breakCondition` | string (FEL) | OPTIONAL | FEL expression evaluated after each iteration; when true, terminates the foreach early. Useful for early-exit on first match or threshold. |
| `outputPath` | string | OPTIONAL | Case-file path where iteration results are written per `mergeStrategy`. The write goes through the governed output-commit pipeline. |
| `mergeStrategy` | enum | OPTIONAL | `shallow` (per-iteration output replaces top-level keys), `deep` (deep-merges into existing structure), or `collect` (accumulates as an array). Required when `outputPath` is set. |
| `transitions` | array of Transition | OPTIONAL | Fired after the foreach completes (all iterations done OR `breakCondition` triggered OR collection empty). |

**Iteration semantics.** Sequential is the canonical semantics:

1. On entry, the processor evaluates `collection` against case state. If the result is not an array, the processor MUST reject with a kernel-level error.
2. If the array is **empty**, no iterations run; the foreach state's outgoing transitions become eligible immediately. This is the empty-collection fast path.
3. For each element, the processor binds the element under `itemVariable` (default `$item`) and the zero-based index under `indexVariable` (default `$index`) in case state, then enters `body`. The body executes to completion (a final state within `body`, or an outgoing transition from `body`).
4. After body completion, if `breakCondition` is set and evaluates to true, iteration terminates early.
5. After all iterations (or early termination), the foreach state's outgoing transitions become eligible.

**Per-iteration bindings** are scoped to the body's execution; they do NOT persist into case state after the foreach completes. Authors that need per-iteration outputs to survive use `outputPath` + `mergeStrategy`.

**Cancellation and timers.** `cancellationPolicy` is reserved for parallel iteration semantics in a future revision; sequential foreach has a single in-flight branch and no cancellation surface. Timers created inside `body` are scoped to the iteration that created them (cancelled when the iteration completes or is broken).

**Implementation status.** Sub-PR D shipped authoring + schema validity + lint coverage (`K-FOREACH-001`/`002`/`003`/`004`). Sub-PR D-2 wired sequential runtime iteration: `collection` FEL evaluation (rejects non-array with `EvalError::ForEach`), per-iteration `itemVariable` / `indexVariable` bindings (defaults `$item` / `$index`; restored to their prior values after the foreach completes), `breakCondition` FEL predicate evaluated after each iteration for early exit, the empty-collection fast path, and auto-firing of the foreach state's first eligible anonymous outgoing transition with synthetic event `$foreachComplete`. Provenance is emitted as `foreachIterationStarted` (per item, carrying `{foreachState, index, item}`) plus `foreachIterationCompleted` (carrying `{foreachState, index}` and `breakTriggered: true` when iteration broke early) plus exactly one `foreachCompleted` summary (`{foreachState, iterations, broke}`) before the outgoing transition fires. Sub-PR D-3 added per-iteration body action execution: `body.onEntry` actions run with the iteration's bindings visible, then `body.onExit` runs after the body but before `breakCondition` evaluation; mutations are attributed to the synthetic lifecycle-state label `<foreach-state>:body` so audit tooling can distinguish state-level onEntry mutations from body-iteration mutations. Sub-PR D-4 added `outputPath` + `mergeStrategy` writes: after each iteration's body executes, the post-body value of `case_state[itemVariable]` is captured per `mergeStrategy` (`collect` appends to the array at `outputPath`; `shallow` merges top-level object keys; `deep` recursively merges nested objects, replacing arrays wholesale and overwriting non-object collisions). Each merge emits a `caseStateMutation` record attributed to `<foreach-state>:output`. Type errors surface as `EvalError::ForEach` (e.g., `mergeStrategy=shallow` requires per-iteration items to be objects; `collect` rejects a non-array existing value at `outputPath`). Body kind is read but only atomic bodies are exercised; nested-state body transitions and parallel iteration honoring `concurrency` are tracked as Sub-PR D-5.

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
| `event` | TransitionEvent | OPTIONAL | Typed trigger for explicit **event delivery** (see below). When omitted, the transition does not match any external event **name**; in `evaluationMode: continuous` it still participates in the post-mutation guard re-scan when `event` is omitted or `event.kind` is `condition`, per Runtime Companion §10.3. |
| `target` | string | REQUIRED | Target state identifier. |
| `guard` | string (FEL) | OPTIONAL | FEL expression that must evaluate to `true` for the transition to fire. |
| `actions` | array of Action | OPTIONAL | Actions executed during the transition. |
| `tags` | array of string | OPTIONAL | Semantic tags for governance attachment via `lifecycleHook` (S10.4). |
| `description` | string | OPTIONAL | Human-readable explanation of this transition. |

**`TransitionEvent`.** Normative authored shape is a JSON object with required discriminant `kind`: `timer` \| `message` \| `signal` \| `condition` \| `error`, plus kind-specific fields. The Kernel JSON Schema defines the full shape (`schemas/wos-workflow.schema.json`, `$defs/TransitionEvent` and branch definitions). At the **runtime boundary**, events are still identified by a string **name** (for example the value passed to `process_event`). A transition matches when that string equals the transition's typed `event` resolved to the same name (for `message` and `signal`, the `name` field; for `timer`, the synthesized expiry name from `timerId`, `source`, and optional `firesAs` per §9.2 and §4.10; for `error`, the runtime dispatch name is the literal `$error` while the typed `code` (and optional `actionPath`) carry the error discriminant; for `condition`, continuous-mode rescan rules in the Runtime Companion). Authoring and integration surfaces SHOULD preserve the full typed `event` JSON where they expose transitions (for example search and graph tools), and SHOULD use a separate derived dispatch string only for matching against `process_event` names—not as a substitute for the authored object. A reference deserializer MAY accept a legacy bare string for `event` and coerce it to the equivalent `TransitionEvent` for migration; new documents SHOULD use the object form. Unknown legacy reserved strings beginning with `$` that are not recognized by that coercion MUST NOT be silently rewritten to a different message `name` (they remain invalid and are rejected by static rules such as K-007).

### 4.6 Transition Resolution

When an event occurs (identified by its runtime **event name** string):

1. Collect all transitions from current active states whose `event` is present **and** whose typed `event` resolves to the same name as the triggering event (per §4.5).
2. Evaluate guards in **document order**. The first transition whose guard evaluates to `true` (or has no guard) wins.
3. If no transition matches, the event is recorded in provenance but does not change lifecycle state.

This is deterministic: document order is the tiebreaker. Two conformant processors given the same document and the same event from the same state MUST select the same transition.

### 4.7 Transition Execution Sequence

When a transition fires:

1. Execute `onExit` actions of the source state, innermost first.
2. Execute transition `actions`.
3. Execute `onEntry` actions of the target state, outermost first.
4. Emit a provenance record for the transition.

<!-- absorbed-from: companions/lifecycle-detail.md §2 Transition Evaluation Algorithm per ADR 0076 D-8 — full content migrated; source section may be removed once cross-references are updated. -->

This section is normative.

### 4.7.1 Overview

The kernel defines the lifecycle as a deterministic pure function (Kernel S4.2). This section provides the complete algorithm as pseudocode. Two conformant Kernel Complete processors given the same document and the same event sequence MUST produce the same state transitions.

### 4.7.2 Configuration

A **configuration** is the set of currently active states. In a workflow without parallel states, the configuration contains exactly one state. In a workflow with parallel states, the configuration contains one state per active region.

### 4.7.3 Algorithm: Process Event

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

### 4.7.4 Algorithm: Fire Transition

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

### 4.7.5 Exit and Entry Path Computation

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

### 4.7.6 Nested State Transitions

When a transition crosses nesting boundaries (e.g., from a substate of one compound state to a substate of another), the exit path includes all states being exited up to the LCA, and the entry path includes all states being entered down from the LCA. This ensures all `onExit` and `onEntry` actions fire in the correct order.

---

### 4.8 Fork and Join

**Fork (entering a parallel state):** All regions are activated simultaneously. Each region begins in its `initialState`.

**Join (exiting a parallel state):** Governed by the `cancellationPolicy`. Under `wait-all` (default), when all regions reach a final state, the processor generates a synthetic `$join` event (runtime name). Outgoing transitions from the parallel state MUST declare `event` as a `signal` with `name` `$join` and `scope` `instance` — that is, `{ "kind": "signal", "name": "$join", "scope": "instance" }` in JSON — so they match that synthetic delivery. Under `cancel-siblings`, the synthetic event fires when any region reaches a final state; remaining regions are cancelled. Under `fail-fast`, the synthetic event fires when any region reaches a state tagged `error`; remaining regions are cancelled.

The `$join` name is kernel-defined for join completion. Workflow authors MUST NOT use `$join` as a **message** event `name`; it is reserved for the join **signal** shape above.

<!-- absorbed-from: companions/lifecycle-detail.md §4 Advanced Parallel Execution per ADR 0076 D-8 — full content migrated; source section may be removed once cross-references are updated. -->

This section is normative.

### 4.8.1 Region Activation

When a parallel state is entered (Kernel S4.8), all regions are activated simultaneously. Each region begins in its `initialState`. The processor MUST add the initial state of every region to the configuration atomically -- no region may begin processing events before all regions are initialized.

```
function activateAllRegions(parallelState, configuration):
    for regionName, region in parallelState.regions:
        initialState = region.states[region.initialState]
        configuration.add(initialState)
        executeOnEntry(initialState)
```

### 4.8.2 Event Routing to Regions

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

### 4.8.3 Join Semantics

The join condition for a parallel state depends on the `cancellationPolicy` (Kernel S4.4):

- **`wait-all`:** The `$join` event (Kernel S4.10) fires when every region has an active state of type `final`. The parallel state then evaluates its own outgoing transitions.
- **`cancel-siblings`:** When any region reaches a final state, the `$join` event fires. All other regions are cancelled: their active states receive `onExit` actions (innermost first), and their states are removed from the configuration.
- **`fail-fast`:** When any region reaches an error final state (a final state entered via an error transition), all other regions are cancelled immediately.

### 4.8.4 Region Cancellation

When a region is cancelled (by `cancel-siblings`, `fail-fast`, or an explicit transition out of the parallel state):

1. For each active state in the region, execute `onExit` actions innermost first.
2. Cancel any active timers scoped to the region.
3. If any active state has `historyState`, clear its history (the cancellation invalidates the recorded configuration).
4. Remove all region states from the configuration.
5. Emit a provenance record for the cancellation.

### 4.8.5 Nested Parallelism

Parallel states MAY be nested within compound states, and compound states MAY be nested within parallel regions. The algorithms in S2 and S4 apply recursively. The configuration may contain states at arbitrary nesting depth.

### 4.8.6 Transitions Exiting a Parallel State

A transition on the parallel state itself (not on a region substate) exits the entire parallel state. All regions are cancelled as described in S4.4, and the transition fires normally.

---

### 4.9 Event Handling

Events that match no transition from any current active state are recorded in provenance but do not change lifecycle state. This is not an error condition.

<!-- absorbed-from: companions/runtime.md §4 Event Delivery Contract per ADR 0076 D-8 — full content migrated; source section may be removed once cross-references are updated. -->

This section is normative.

### 4.9.1 Serial Processing

The deterministic evaluation algorithm (Kernel S4.2) requires that events are processed one at a time per instance. The processor MUST serialize concurrent event delivery. Two events arriving simultaneously for the same instance MUST be queued and processed sequentially. The queue order is implementation-defined (FIFO is RECOMMENDED).

Multiple instances MAY process events concurrently -- the serialization requirement is per-instance, not global.

### 4.9.2 Event Structure

Events carry:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `event` | string | REQUIRED | Event name matching a transition's `event` property. |
| `actorId` | string | OPTIONAL | Identifier of the actor submitting the event. |
| `data` | object | OPTIONAL | Event payload, available in the evaluation context as `event` (Kernel S7.2). |
| `timestamp` | string (datetime) | REQUIRED | ISO 8601 timestamp of event submission. |
| `idempotencyToken` | string | OPTIONAL | Token for exactly-once delivery (S4.3). |

### 4.9.3 Exactly-Once Semantics

The processor MUST provide at-least-once delivery and MUST deduplicate events. Deduplication uses the event's `idempotencyToken` when provided. When no token is provided, the processor MUST use its own delivery tracking mechanism to prevent duplicate processing.

The deduplication window is implementation-defined but MUST span at least the duration of a single event processing cycle (from event receipt through durability checkpoint). A deduplication window shorter than this provides no protection against the crash-and-replay scenario.

### 4.9.4 Unmatched Events

Events that match no transition from any current active state are recorded in provenance but do not change lifecycle state (Kernel S4.9). This is not an error. The provenance record for an unmatched event includes the event name, the configuration at the time of receipt, and the actor (if provided).

---

### 4.9.10 Kernel-Generated Events

The kernel generates synthetic events in response to internal conditions. Kernel-generated event **names** are prefixed with `$` to distinguish them from ordinary author **message** names. In the typed `TransitionEvent` model, authors match these deliveries using the appropriate kind: for example `$timeout.*` names map to `timer` events with the matching `source` (and `timerId` / `firesAs` as in the schema); `$join` and `$compensation.complete` map to `signal` events whose `name` is exactly that string (including the `$` prefix) and whose `scope` is typically `instance`; relationship events `$related.*` map to `signal` with `scope` `related` and a `name` that resolves to the same identifier the processor delivers on the event boundary (Runtime Companion). **`message` event `name` MUST NOT begin with `$`** (schema-enforced); reserved kernel prefixes are expressed as `timer`, `signal`, or `error` as applicable — not as `message`.

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

### 4.9.11 Reentry

Entering a state fires `onEntry` behavior regardless of prior visits. Context from prior visits is preserved in the case state history (S5), not in the lifecycle model.

### 4.9.12 Semantic Transition Tags

Transitions and states carry semantic `tags` that declare their nature (e.g., `["determination", "review"]`). Governance documents from higher layers match on tags to attach governance rules via the `lifecycleHook` seam (S10.4).

Tags are free-form strings. The following tags are conventionally recognized by Layer 1 (Workflow Governance):

| Tag | Conventional Meaning |
|-----|---------------------|
| `determination` | A step that produces a consequential decision. |
| `review` | A step where work is reviewed by another actor. |
| `adverse-decision` | A step that may produce an unfavorable outcome for an individual. |
| `quality-check` | A step subject to quality assurance sampling. |
| `intake` | An intake-handling or intake-acknowledgment step, not the raw submission itself. |
| `appeal` | An appeal or reconsideration step. |
| `notification` | A step that produces notices to affected individuals. |
| `hold` | A state where the case is suspended pending an external condition. |

<!-- absorbed-from: companions/runtime.md §8.2 Governance Scoping per ADR 0076 D-8 — full content migrated. -->

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

### 4.13 Milestones

Milestones are named conditions on case state that, when satisfied, indicate meaningful progress. Milestones do not affect lifecycle state directly -- they are observable conditions. The milestone identifier is the map key in `lifecycle.milestones`.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `condition` | string (FEL) | REQUIRED | FEL expression evaluated against case state. |
| `description` | string | OPTIONAL | Human-readable description. |
| `triggerMode` | enum | OPTIONAL | When the processor evaluates the condition. Defaults to `writeSettled`. |

**Trigger semantics (`triggerMode: writeSettled`).** A processor MUST evaluate every un-fired milestone's `condition` after each durable case-state write — once the write has been persisted and is observable to subsequent reads. A milestone fires at most once per case instance: once `condition` evaluates true and a `MilestoneFired` provenance record has been appended (carrying `{"milestoneId": <id>}`), the milestone id is recorded on the case instance and never re-evaluated. Multiple milestones firing from a single write MUST be appended to provenance in lexicographic milestone-id order so the stream is deterministic. Future trigger modes (e.g., reactive event-based firing per §4.13 Future Work) will extend the enum without altering `writeSettled` semantics.

### 4.14 History States

Compound states MAY declare a `historyState` property (`shallow` or `deep`). When present, reentry to the compound state resumes the last active substate instead of the `initialState`, overriding the default reentry behavior (S4.11).

- **`shallow`**: Resumes the last active direct substate of this compound state.
- **`deep`**: Restores the full nested configuration at all nesting levels.

History state semantics (algorithms, clearing rules, interactions with parallel states) are defined in the Lifecycle Detail Companion (S3). A Kernel Structural processor MAY ignore `historyState`; a Kernel Complete processor MUST implement it per the companion's algorithms.

---

<!-- absorbed-from: companions/lifecycle-detail.md §3 History States per ADR 0076 D-8 — full content migrated; source section may be removed once cross-references are updated. -->

This section is normative.

### 4.14.1 Overview

History states record the last active configuration within a compound state, enabling resumption after suspension. The kernel defines compound states with an `initialState` property. This companion defines how history overrides the initial state on reentry.

### 4.14.2 Shallow History

When a compound state declares `historyState: "shallow"`, the processor records the last active **direct substate** when the compound state is exited. On subsequent entry to the compound state, execution resumes in the recorded substate rather than the `initialState`.

```
function enterCompoundState(compoundState, configuration):
    if compoundState.historyState == "shallow" and hasHistory(compoundState):
        target = getShallowHistory(compoundState)
    else:
        target = compoundState.initialState
    enterState(target, configuration)
```

### 4.14.3 Deep History

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

### 4.14.4 History Clearing

History is cleared when the compound state's parent is exited. If a compound state is within a parallel region and that region is cancelled, the history for the compound state is cleared.

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

Each mutation record MAY carry:

1. `mutationSource` — the origin of the value change. Reserved literals: `human-entered`, `human-corrected`, `agent-extracted`, `system-fetched`, `computed`, `self-attested`. Vendor extensions MUST use an `x-` prefix.
2. `verificationLevel` — the degree of independent confirmation behind the value. Reserved literals: `independent`, `attested`, `corroborated`, `authoritative`. Vendor extensions MUST use an `x-` prefix. OPTIONAL; tying it to `determination`-tagged transitions is policy-shaped (governance profile), not a blanket kernel MUST.

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

<!-- absorbed-from: companions/runtime.md §14 Relationship-Triggered Events per ADR 0076 D-8 — full content migrated; source section may be removed once cross-references are updated. -->

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

Persisted or exported provenance logs use one shared record envelope across all tiers. Facts-tier fields are normative in the kernel; higher tiers reuse the same envelope with `auditLayer` set accordingly. Runtime constructors MAY leave append-stamped fields empty in memory, but any provenance document validated against `wos-provenance-log.schema.json` is the post-append export shape.

### 8.2 Facts Tier Record

Every persisted or exported provenance record MUST include:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | REQUIRED | Unique record identifier. |
| `recordKind` | string | REQUIRED | Provenance record discriminator. |
| `timestamp` | datetime | REQUIRED | When the action occurred (ISO 8601). |
| `auditLayer` | string | REQUIRED | Provenance tier for the exported record (`facts` today; higher layers MAY emit `reasoning`, `counterfactual`, or `narrative` through the same envelope). |
| `definitionVersion` | string | REQUIRED | Version of the Kernel Document governing this action. |
| `actorId` | string | OPTIONAL | Identifier of the actor who performed the action, when the processor can name a concrete actor. |
| `actorType` | enum | OPTIONAL | `human`, `system`, or `agent` when the processor can classify the actor. |
| `event` | string | OPTIONAL | Triggering event or reserved event literal when the record kind defines one. |
| `data` | object | OPTIONAL | Additional context payload for the record kind. |
| `inputs` | array[string] | OPTIONAL | Input entity references used by this activity. |
| `outputs` | array[string] | OPTIONAL | Output entity references generated by this activity. |
| `transitionTags` | array | REQUIRED for state-transition records with tagged transitions, OPTIONAL otherwise | Semantic tags copied from the firing transition. |
| `caseFileSnapshot` | object | REQUIRED for `determination` transitions, OPTIONAL otherwise | Canonical snapshot of the case-file state used by the determination. |
| `outcome` | string | OPTIONAL | Open-enum outcome literal recorded by the processor. See §8.2.2 for reserved values. |
| `inputDigest` | string | OPTIONAL | Cryptographic digest of inputs for tamper detection. |
| `outputDigest` | string | OPTIONAL | Cryptographic digest of outputs for tamper detection. |
| `lifecycleState` | string | OPTIONAL | Lifecycle state at the time of the action when the processor can determine it canonically. |
| `extensions` | object | OPTIONAL | Extension data. All keys MUST be prefixed with `x-`. |

`wos-provenance-log.schema.json` validates the persisted/exported shape above, not unstamped in-memory constructors. Runtime append paths MUST populate the required export fields before persistence; helper constructors in `wos-core` MAY leave those fields empty until the runtime stamps them.

#### 8.2.1 Snapshot Semantics

When a transition tagged `determination` fires, the processor MUST copy the transition's semantic tags into `transitionTags` on the Facts-tier state-transition record and capture the current case-file state in `caseFileSnapshot` immediately before any transition action or post-transition action mutates the case file. The snapshot records the facts the determination was made from, not the values produced by the determination.

`caseFileSnapshot` MUST contain:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `value` | JSON value | REQUIRED | The case-file state observed at transition fire time. |
| `jcsCanonical` | string | REQUIRED | Canonical JSON representation of `value` using JCS (RFC 8785) canonicalization semantics. |
| `sha256` | string | REQUIRED | Lowercase SHA-256 hex digest of `jcsCanonical`. |

Identical case-file state at determination fire time MUST produce byte-identical `jcsCanonical` values and identical `sha256` values. Governance adverse-decision notices (Governance §3.2) and override records (Governance §7.3) use this snapshot as the deterministic factual basis for later explanation, appeal, and audit.

#### 8.2.2 Outcome Field

The optional `outcome` field records why the processor completed (or declined to complete) the action captured by the record. It is an **open enum**: the values listed below are reserved across all WOS tiers, and vendor extensions MUST use an `x-` prefix to avoid future collisions. The reserved values are:

| Value | Mandated by | Meaning |
|-------|-------------|---------|
| `preconditionNotSatisfied` | AI Integration §3.3.1 | A capability precondition FEL expression evaluated to `false` or a non-boolean, so the processor skipped the agent invocation and fell through to the fallback chain. |
| `convergenceCapReached` | Runtime §10.3 | Continuous-mode re-evaluation reached the 100-cycle convergence cap for a single triggering mutation; the processor halted further re-evaluation for that mutation. |

The canonical schema definition is `$defs/ProvenanceOutcome` in `wos-provenance-log.schema.json`. Processors that emit outcome values outside this reserved set MUST prefix them with `x-` (see Kernel §10.6 extensions).

The `preconditionNotSatisfied` outcome pairs with the `capabilityInvocation` record-kind discriminator (AI Integration §3.3.1): when a record carries `recordKind: "capabilityInvocation"` with `data.invocationBlocked: true`, the `outcome` field MUST be `preconditionNotSatisfied`. This pairing is enforced at schema-validation time via `$defs/CapabilityInvocationRecord` in `wos-provenance-log.schema.json`, which `FactsTierRecord` composes via `allOf` so that every conformant provenance log participates in the MUST regardless of whether an AI Integration document is also attached to the workflow.

#### 8.2.3 Intake and Governed-Case Boundary Record Kinds

The following Facts-tier record kinds are reserved for the intake-acceptance boundary named by Runtime Companion §3.4:

| `recordKind` | Emitted by | Meaning |
|--------------|------------|---------|
| `intakeAccepted` | Runtime `acceptIntakeHandoff` | The host accepted an intake handoff into WOS-managed workflow handling. |
| `intakeRejected` | Runtime `acceptIntakeHandoff` | The host rejected an intake handoff. No governed case was created or updated. |
| `intakeDeferred` | Runtime `acceptIntakeHandoff` | The host received an intake handoff but deferred governed-case mutation pending later action. |
| `caseCreated` | Runtime / binding finalization during accepted governed-case birth | A governed case boundary was established from accepted intake or an equivalent governance-owned creation path. |

`caseCreated` is distinct from Runtime Companion `instanceCreated`. `instanceCreated` records runtime allocation of instance state. `caseCreated` records the governance boundary at which a governed case exists. A public-intake flow MAY emit both records; they remain semantically distinct.

The canonical schema definitions are `$defs/IntakeAcceptedRecord`, `$defs/IntakeRejectedRecord`, `$defs/IntakeDeferredRecord`, and `$defs/CaseCreatedRecord` in `wos-provenance-log.schema.json`. Those schema branches intentionally constrain only the binding-agnostic minimum:

- the `recordKind` literal,
- the canonical event name when one is reserved,
- the minimum relationship between intake inputs and governed-case outputs.

Binding-specific evidence payloads remain owned by the binding seam. For example, a Formspec-driven `caseCreated` record MAY carry `intakeHandoffRef`, `formspecResponseRef`, and `validationReportRef` inside `data`, but the kernel does not require those exact keys for every future binding.

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

<!-- absorbed-from: companions/runtime.md §6 Durability Guarantees per ADR 0076 D-8 — full content migrated; source section may be removed once cross-references are updated. -->

This section is normative.

### 9.1.1 Kernel Guarantees as Runtime Requirements

The Kernel Specification (Kernel S9.1) defines five durable execution guarantees. This section restates them as concrete runtime requirements.

**G1: Crash Recovery.** A non-terminal workflow instance MUST resume from the last durability checkpoint after a processor crash. The processor MUST NOT lose state that was durably persisted before the crash.

**G2: No Duplicate Action Execution.** On replay after a crash, actions with idempotency keys (Kernel S9.3) MUST NOT be re-executed if the previous execution's output was already persisted. Actions without idempotency keys MAY be re-executed -- the processor MUST document which action types are safe for re-execution.

**G3: Non-Deterministic Output Persisted Before Advancing.** Every `invokeService` action MUST persist its output as an immutable step result before the processor advances lifecycle state. During recovery, the processor MUST use the persisted output rather than re-invoking the service.

**G4: Timer Durability.** Timers MUST survive processor restarts, fire within their declared tolerance (S7.2), and consume no runtime resources while waiting. Timer state is part of the CaseInstance (S3.1) and is persisted at every durability checkpoint.

**G5: Signal Delivery.** External signals addressed to suspended or temporarily unreachable instances MUST be durably enqueued. The processor MUST process enqueued signals when the instance becomes available.

### 9.1.2 Checkpoint Semantics

The unit of durability is the **event**. After each event is fully processed -- all transitions fired, all actions executed, all provenance recorded -- the processor MUST durably persist the CaseInstance. The checkpoint includes the updated configuration, case state, provenance position, timer state, active task state, and history store.

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

### 9.1.3 Provenance Durability

The provenance log MUST survive any single-point failure (Kernel S9.1, G5). Provenance records are part of the durability checkpoint -- they are persisted atomically with the instance state. A conformant processor MUST NOT acknowledge an event as processed until both the instance state and the provenance records are durably persisted.

---

### 9.2.12 Actions

The kernel defines the following action types:

| Action Type | Description | Key Properties |
|-------------|-------------|----------------|
| `createTask` | Creates a human or Formspec-backed task instance. | `taskRef`, `assignTo`, `contractRef` |
| `invokeService` | Invokes an external service. | `serviceRef`, `idempotencyKey` |
| `setData` | Sets a case file value. | `path`, `value` |
| `emitEvent` | Emits an event. | `eventType`, `data` |
| `startTimer` | Starts a durable timer. | `timerId`, `duration` or `deadline`, `event` (`TransitionEvent`; in practice the `timer` kind) |
| `cancelTimer` | Cancels a running timer. | `timerId` |
| `log` | Writes an entry to provenance. | `message`, `data` |

**Execution ordering.** Actions within a single state's `onEntry` or `onExit` execute sequentially in document order. The processor MUST NOT reorder actions within a state or transition. Actions across parallel regions MAY execute concurrently; provenance MUST record the actual execution order regardless of whether execution was concurrent or sequential.

**Formspec-backed tasks.** A `createTask` action MAY include `contractRef` when the task is backed by a ContractReference. If that ContractReference has `binding: "formspec"`, Runtime Companion S15 defines the presentation, draft, submit, validation, mapping, and provenance behavior. `prefillMappingRef` and `responseMappingRef` MAY appear on either the ContractReference or the action. The action-level value overrides the ContractReference value for that task. `completionEvent` and `failureEvent` MAY name lifecycle events emitted after the task reaches `completed` or `failed`.

<!-- absorbed-from: companions/runtime.md §5 Action Execution Model + profiles/integration.md §3 Integration Bindings + §4 Contract Validation + §5 CloudEvents Extensions + §6 Correlation + §7 Idempotency + §9 Processing Model per ADR 0076 D-8 — full content migrated; source section may be removed once cross-references are updated. -->

This section is normative.

### 9.2.13.1 Sequential Execution Within a State

Actions within a single state's `onEntry` or `onExit` execute sequentially in document order (Kernel S9.2). Transition actions execute sequentially between exit and entry (Kernel S4.7). The processor MUST NOT reorder actions within a state or transition.

### 9.2.14.2 Transition Execution Sequence

The full sequence for a fired transition (Kernel S4.7, Lifecycle Detail S2.4):

1. Execute `onExit` actions of the source state, innermost first.
2. Execute transition `actions` in document order.
3. Execute `onEntry` actions of the target state, outermost first.
4. Emit provenance records.

Each action produces a provenance record of type `actionExecuted`. The record includes the action type, inputs, outputs, executing actor, and timestamp.

### 9.2.15.3 Parallel Region Actions

Actions across parallel regions MAY execute concurrently. The processor is not required to parallelize -- sequential execution of region actions is conformant. However, provenance MUST record the actual execution order regardless of whether execution was concurrent or sequential. Two conformant processors given the same document and events MUST agree on which actions executed, even if the execution ordering differs.

### 9.2.16.4 Service Invocation

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

### 9.2.17.5 Contract Validation

Contract validation flows through the `contractHook` seam (Kernel S10.2). The processor delegates to the host's ContractValidator (S12.3). Results flow back as a ValidationResult (valid or errors). Validation failures trigger the rejection policy declared in the Governance Document (Governance S8).

Formspec-backed task completion uses the coprocessor protocol in S15. That protocol validates a full Formspec Response envelope before case mutation and MAY then run `contractHook` / Governance S5 checks on the proposed completion bundle. A processor MUST NOT use `contractHook` alone as the per-task completion gate for a Formspec-bound task.

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

---

#### 9.2.6 Integration Bindings

### 9.2.18 Overview

Integration bindings are declared under the `bindings` property of a `$wosWorkflow` document. Each binding is an `OutputBinding` entry that governs how a source surface (state, transition, capability, or signal) commits mutations to case state through the validated output-commit pipeline (ADR 0080).

```json
{
  "$wosWorkflow": "1.0",
  "url": "https://example.gov/workflows/benefits-adjudication",
  "version": "1.0.0",
  "title": "Benefits Adjudication Integration Bindings",
  "impactLevel": "operational",
  "actors": [
    { "id": "system", "type": "system" }
  ],
  "lifecycle": {
    "initialState": "start",
    "states": {
      "start": { "type": "atomic" },
      "done": { "type": "final" }
    }
  },
  "bindings": [
    {
      "on": "capability:eligibilityCheck",
      "projection": {
        "$.result": "caseFile.eligibility.result"
      }
    }
  ]
}
```

### 9.2.19 Integration Binding Types

| Type | Description |
|------|-------------|
| `request-response` | Synchronous invocation of an external service. Interface defined by an OpenAPI reference. |
| `event-emit` | Production of an outbound CloudEvents 1.0 event. |
| `event-consume` | Subscription to inbound events from external sources, with correlation. |
| `callback` | Long-running external interaction: the workflow sends a request and later receives a callback event with the result. |
| `arazzo-sequence` | Multi-step API orchestration sequence. References an Arazzo document (OpenAPI Initiative). |
| `tool` | Non-HTTP invocation informed by CWL's `CommandLineTool` descriptor pattern. |
| `policy-engine` | External policy engine invocation (XACML, OPA, or Cedar). |

### 9.2.20 Common Binding Properties

All integration binding types share the following properties:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `type` | string | REQUIRED | The integration binding type (one of the values in S3.2). |
| `description` | string | OPTIONAL | Human-readable description of this integration binding. |
| `requestContract` | object | OPTIONAL | Formspec Definition contract for request validation (S4). |
| `responseContract` | object | OPTIONAL | Formspec Definition contract for response validation (S4). |
| `retry` | object | OPTIONAL | Retry policy (S3.8). |
| `timeout` | string (ISO 8601 duration) | OPTIONAL | Maximum time to wait for a response. |
| `idempotencyKeyExpression` | string (FEL) | OPTIONAL | FEL expression evaluated against the case state to produce an idempotency key. Maps to the kernel's `idempotencyKey` (Kernel S9.3). |
| `extensions` | object | OPTIONAL | Extension data. Property names MUST begin with `x-`. |

### 9.2.21.1 outputBinding JSONPath Profile

`outputBinding` values are JSON Path expressions into the service response. This specification pins an explicit **RFC 9535 subset** for all `outputBinding` path expressions.

**Supported constructs:**

- **Member access** — `.key`, `['key']`, `["key"]` (including quoted keys with backslash-escape)
- **Index** — `[n]` (zero-based non-negative integer)
- **Wildcard** — `[*]` (fans out over all array elements or object values; subsequent segments apply to each element and results are collected into an array)
- **Slice** — `[start:end]` and `[start:end:step]` (Python-style; negative indices count from the end; open bounds are allowed via `[start:]`, `[:end]`, `[::step]`)

**Excluded constructs:**

- **Recursive descent** — `..` (RFC 9535 §2.5) is NOT supported. Rationale: recursive descent can match nodes at unpredictable depths, making provenance records non-deterministic and complicating replay verification.
- **Filter expressions** — `[?(...)]` (RFC 9535 §2.6) are NOT supported. Rationale: filter expressions introduce a second expression language (distinct from FEL) inside binding documents, making static analysis and lint-time validation significantly harder.

**Enforcement:** A WOS processor MUST reject any Integration Profile Document whose `outputBinding` values use unsupported constructs at definition load time. This is a lint-time error (rule I-001 in the verification matrix), not a runtime surprise. If a future binding genuinely requires filter expressions or recursive descent, the outputBinding profile MUST be extended via a dedicated ADR rather than silently tolerating the feature.

**Forward compatibility note:** The profile is designed to grow backwards-compatibly. Adding new supported constructs does not require existing profiles to change. Removing a supported construct is a breaking change and requires a new major version of this specification.

**Iteration order for wildcard over objects:** For `[*]` applied to a JSON object, iteration order equals `serde_json::Map` insertion order (preserved as-of-parse). Fixtures SHOULD NOT rely on alphabetical key order unless they sort explicitly.

**All-or-nothing binding:** If any `outputBinding` JSONPath resolves to no value, the binding invocation fails with a binding error. For optional event payload fields, use a default-providing input mapping in the downstream consumer rather than relying on partial output bindings.

### 9.2.22 Request-Response Bindings

A `request-response` binding defines a synchronous HTTP service invocation.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `interface` | object | REQUIRED | OpenAPI reference. Contains `$ref` (URI to an OpenAPI document). |
| `operation` | string | REQUIRED | The operation ID within the referenced OpenAPI document. |
| `inputMapping` | object | OPTIONAL | Maps case state paths to service request parameters. Keys are parameter names; values are FEL expressions or case state paths. |
| `outputBinding` | object | OPTIONAL | Maps service response fields to case state paths. Keys are case state paths; values are JSON Path expressions into the response. |

```json
{
  "type": "request-response",
  "interface": {
    "$ref": "https://api.example.gov/background-checks/openapi.yaml"
  },
  "operation": "submitCheck",
  "timeout": "PT30M",
  "retry": {
    "maxAttempts": 3,
    "backoff": "exponential",
    "initialInterval": "PT10S"
  },
  "inputMapping": {
    "applicantId": "caseFile.application.applicantId"
  },
  "outputBinding": {
    "caseFile.backgroundCheck.result": "$.result",
    "caseFile.backgroundCheck.completedAt": "$.completedAt"
  }
}
```

### 9.2.23 Arazzo Sequence Bindings

An `arazzo-sequence` binding references an Arazzo document (OpenAPI Initiative) for multi-step API orchestration. Arazzo defines sequences of API calls with dependencies, conditional logic, and data passing between steps.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `arazzoRef` | string (URI) | REQUIRED | URI reference to the Arazzo document. |
| `inputMapping` | object | OPTIONAL | Maps case state paths to Arazzo workflow input parameters. Keys are Arazzo input parameter names; values are FEL expressions or case state paths. |
| `outputBinding` | object | OPTIONAL | Maps Arazzo workflow output to case state paths. Keys are case state paths; values are JSON Path expressions into the Arazzo workflow output. |

Each step in the Arazzo sequence produces a separate provenance record in the workflow's Facts tier (Kernel S8). When a step in the Arazzo sequence invokes an AI agent registered in a Layer 2 AI Integration Document, that invocation is subject to the agent's deontic constraints and autonomy level.

**WOS v1.0 limitation:** In WOS v1.0, step inputs cannot reference prior step outputs via FEL (`$.steps[...]`). Cross-step data flow is through the sequence-level output binding only. Inter-step references are reserved for Arazzo Engine Binding (§2 of TODO).

Step outputs are accessible in the binding-level `outputBinding` via `$.steps.<stepId>.output`. The runtime structures the accumulated step context as `{ "steps": { "<stepId>": { "output": <stepResponse> } } }`.

```json
{
  "type": "arazzo-sequence",
  "arazzoRef": "urn:agency.gov:arazzo:eligibility-check:1.0.0",
  "responseContract": {
    "definitionRef": "urn:agency.gov:contracts:eligibility-response:1.0.0"
  },
  "inputMapping": {
    "applicantSSN": "caseFile.application.ssn",
    "householdSize": "caseFile.application.householdSize"
  },
  "outputBinding": {
    "caseFile.eligibility.result": "$.steps.eligibility.output"
  },
  "idempotencyKeyExpression": "caseFile.application.id"
}
```

### 9.2.24 Tool Bindings

A `tool` binding defines a non-HTTP invocation informed by CWL's `CommandLineTool` descriptor pattern. WOS does not require a CWL-conformant processor. The descriptor structure is adapted for WOS: it describes the invocation method, command, arguments, and resource requirements.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `invocation` | object | REQUIRED | Invocation descriptor (S3.6.1). |
| `inputMapping` | object | OPTIONAL | Maps case state paths to tool input parameters. |
| `outputBinding` | object | OPTIONAL | Maps tool output to case state paths. |
| `resourceRequirements` | object | OPTIONAL | Resource constraints (S3.6.2). |

#### 3.6.1 Invocation Descriptor

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `method` | string | REQUIRED | Invocation method: `command-line`, `batch-file`, `database-procedure`, `graph-query`, or an `x-` prefixed custom method. |
| `command` | string | REQUIRED | The command, procedure name, or query to execute. |
| `arguments` | array of strings | OPTIONAL | Ordered command arguments. FEL expressions embedded in `{{ }}` delimiters are evaluated against the evaluation context (Kernel S7.2) before invocation. The `{{ }}` delimiter syntax follows the Formspec Locale specification's template convention. |
| `environment` | object | OPTIONAL | Execution environment metadata (e.g., container image, runtime version). Implementation-defined. |

#### 3.6.2 Resource Requirements

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `maxExecutionTime` | string (ISO 8601 duration) | OPTIONAL | Maximum execution time. The processor MUST terminate the invocation if this duration is exceeded. |
| `maxMemory` | string | OPTIONAL | Maximum memory allocation (e.g., `"512MB"`, `"2GB"`). Implementation-defined enforcement. |
| `maxCores` | integer | OPTIONAL | Maximum CPU cores. Implementation-defined enforcement. |

```json
{
  "type": "tool",
  "invocation": {
    "method": "command-line",
    "command": "/opt/legacy/eligibility-check",
    "arguments": [
      "--ssn",
      "{{ caseFile.application.ssn }}",
      "--household-size",
      "{{ caseFile.application.householdSize }}"
    ],
    "environment": {
      "image": "legacy-tools:2024.1"
    }
  },
  "requestContract": {
    "definitionRef": "urn:agency.gov:contracts:legacy-input:1.0.0"
  },
  "responseContract": {
    "definitionRef": "urn:agency.gov:contracts:legacy-output:1.0.0"
  },
  "resourceRequirements": {
    "maxExecutionTime": "PT30S"
  }
}
```

### 9.2.25 Event Bindings

#### 3.7.1 Event-Emit Bindings

An `event-emit` binding produces an outbound CloudEvents 1.0 event.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `eventType` | string | REQUIRED | The CloudEvents `type` attribute value (e.g., `"org.example.grants.notification"`). |
| `dataMapping` | object | OPTIONAL | Maps case state paths to event data fields. Keys are event data field names; values are FEL expressions or case state paths. |
| `channel` | string | OPTIONAL | Delivery channel hint (e.g., `"email"`, `"webhook"`, `"queue"`). Implementation-defined. |

All outbound events MUST include the WOS CloudEvents extension attributes defined in S5.

#### 3.7.2 Event-Consume Bindings

An `event-consume` binding subscribes to inbound events from external sources.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `eventType` | string | REQUIRED | The CloudEvents `type` attribute value to subscribe to. |
| `correlation` | array of objects | REQUIRED | Correlation rules for matching inbound events to workflow instances (S6). |
| `outputBinding` | object | OPTIONAL | Maps event data fields to case state paths. |

#### 3.7.3 Callback Bindings

A `callback` binding models a long-running external interaction: the workflow sends a request and later receives a callback event.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `interface` | object | REQUIRED | OpenAPI reference for the initial request. |
| `operation` | string | REQUIRED | Operation ID for the initial request. |
| `callbackEventType` | string | REQUIRED | The CloudEvents `type` attribute value of the expected callback event. |
| `correlation` | array of objects | REQUIRED | Correlation rules for matching the callback to the originating instance (S6). |
| `inputMapping` | object | OPTIONAL | Input mapping for the initial request. |
| `outputBinding` | object | OPTIONAL | Maps callback event data to case state paths. |

### 9.2.26 Retry Policy

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `maxAttempts` | integer | REQUIRED | Maximum number of invocation attempts (including the initial attempt). |
| `backoff` | string | OPTIONAL | Backoff strategy: `"fixed"`, `"linear"`, or `"exponential"`. Default: `"fixed"`. |
| `initialInterval` | string (ISO 8601 duration) | OPTIONAL | Initial interval between retries. Default: `"PT1S"`. |
| `maxInterval` | string (ISO 8601 duration) | OPTIONAL | Maximum interval between retries (for exponential/linear backoff). |

---

#### 9.2.7 Integration Profile — Contract Validation

### 9.2.27 Formspec Definition Contracts

Integration bindings MAY declare Formspec Definition contracts for request and response validation. A contract reference points to a headless Formspec Definition (a Definition used purely for validation, with no rendering or user-facing semantics).

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `definitionRef` | string (URI) | REQUIRED | URI reference to the Formspec Definition used as the validation contract. |

### 9.2.28 Validation Semantics

When a `requestContract` is declared, the WOS Processor MUST validate the constructed request against the referenced Formspec Definition before sending the request. The processor MUST delegate this validation to a Formspec-conformant processor (Core S1.4).

When a `responseContract` is declared, the WOS Processor MUST validate the external system's response against the referenced Formspec Definition before committing results to the case state. The processor MUST delegate this validation to a Formspec-conformant processor (Core S1.4).

In both cases:

1. The Formspec-conformant processor evaluates the Definition against the data and produces a ValidationReport (Core S5).
2. The WOS Processor MUST ingest the ValidationReport as a provenance record in the workflow's Facts tier (Kernel S8).
3. If the ValidationReport contains errors (severity `"error"`), the WOS Processor MUST NOT commit the results to the case state. The invocation is treated as failed and is subject to the retry policy (S3.8), if configured.

This is the same Formspec-as-validator pattern used in Layer 2 for agent output validation (AI Integration S6). The Formspec Definition is the contract. The external system's output is untrusted input validated against that contract.

---

#### 9.2.8 Integration Profile — CloudEvents Extensions

### 9.2.29.1 WOS Extension Attributes

All events produced by a WOS workflow MUST conform to the CloudEvents 1.0 specification [CloudEvents]. WOS defines the following extension attributes:

| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `wosinstanceid` | string (URI) | REQUIRED | The workflow instance identifier. |
| `wosdefid` | string (URI) | REQUIRED | The workflow definition identifier (the kernel document's `url`). |
| `wosdefversion` | string | REQUIRED | The workflow definition version (the kernel document's `version`). |
| `wosstate` | string | OPTIONAL | The current lifecycle state at the time of event emission. |
| `wostaskid` | string | OPTIONAL | The task identifier, if the event relates to a task. |
| `woscorrelationkey` | string | OPTIONAL | The primary business correlation key for the workflow instance. |
| `woscausationeventid` | string | OPTIONAL | The `id` of the CloudEvents event that triggered this event. Enables causal event chains. |

### 9.2.30.2 Attribute Semantics

**`wosinstanceid`.** The unique identifier of the running workflow instance. This is the primary key for routing events back to the correct instance. A WOS Processor MUST populate this attribute on every outbound event.

**`wosdefid` and `wosdefversion`.** Together these identify which workflow definition (and version) the emitting instance was created from. External systems can use this to determine the expected schema of event data.

**`wosstate`.** The lifecycle state of the workflow instance at the moment the event was emitted. This is OPTIONAL because some events (e.g., timer expiry events generated by infrastructure) may not have access to the workflow's current state.

**`woscausationeventid`.** When an inbound event triggers a workflow transition that produces one or more outbound events, each outbound event SHOULD carry the `id` of the inbound event in this attribute. This creates an auditable causal chain.

### 9.2.31.3 Inbound Event Processing

When an external event arrives at a WOS Processor:

1. The processor MUST extract the correlation attribute values from the event (S6).
2. The processor MUST find all running workflow instances whose mapped case state values match the correlation attributes.
3. The processor MUST deliver the event to the matched instance(s).
4. If no match is found, the event MUST be logged and MAY be queued for retry.

### 9.2.32.4 Idempotent Event Consumption

All event consumption MUST be idempotent. A WOS Processor MUST handle duplicate delivery of the same event (identified by the CloudEvents `id` attribute) without producing duplicate effects. The RECOMMENDED mechanism is to record processed event identifiers and reject events whose `id` has already been processed.

---

#### 9.2.9 Integration Profile — Correlation

### 9.2.33 Correlation Rules

Correlation is the mechanism by which an inbound external event is matched to the correct running workflow instance. Each `event-consume` or `callback` binding declares one or more correlation rules.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `attribute` | string | REQUIRED | The CloudEvents attribute or extension attribute containing the correlation value. |
| `caseStateMapping` | string (path) | REQUIRED | The case state path whose value MUST match the correlation attribute. |

When multiple correlation rules are declared, all MUST match (logical AND).

```json
{
  "correlation": [
    {
      "attribute": "subject",
      "caseStateMapping": "caseFile.application.applicationId"
    },
    {
      "attribute": "wosinstanceid",
      "caseStateMapping": "instance.id"
    }
  ]
}
```

---

#### 9.2.10 Integration Profile — Idempotency

### 9.2.34 Idempotency Keys

Integration bindings that invoke external services MAY declare an `idempotencyKeyExpression` (S3.3). This is a FEL expression (Core S3) evaluated against the case state to produce a deterministic key.

When an `idempotencyKeyExpression` is declared, the WOS Processor MUST:

1. Evaluate the FEL expression against the current case state to produce the key value.
2. Pass the key to the external service as an idempotency token (the mechanism is service-specific).
3. Record the key in the invocation's provenance record.

This maps to the kernel's `idempotencyKey` property on the `invokeService` action (Kernel S9.3). The Integration Profile adds the FEL expression; the kernel handles the deduplication guarantee.

---

#### 9.2.11 Integration Profile — Processing Model

### 9.2.35 Binding Resolution

When a WOS Processor encounters an `invokeService` action (Kernel S9.2) whose `serviceRef` matches a binding key in the Integration Profile Document, the processor MUST resolve the binding and execute the integration according to the binding's type and properties.

If no Integration Profile Document is present, or the `serviceRef` does not match any binding key, the `serviceRef` is treated as an opaque reference and execution is implementation-defined (the kernel's default behavior).

### 9.2.36 Execution Order

For each integration invocation, the processor MUST follow this order:

1. **Input construction.** Evaluate `inputMapping` expressions against the current case state.
2. **Request validation.** If `requestContract` is declared, validate the constructed input against the Formspec Definition (S4).
3. **Invocation.** Execute the integration binding (send request, run tool, emit event, or invoke policy engine).
4. **Response validation.** If `responseContract` is declared, validate the response against the Formspec Definition (S4).
5. **Output binding.** Apply `outputBinding` to commit results to the case state.
6. **Provenance.** Record the invocation as a provenance record in the Facts tier (Kernel S8), including input/output digests if configured (Kernel S8.3).

If any step fails, the processor MUST NOT proceed to subsequent steps. Failed invocations are subject to the retry policy (S3.8).

### 9.3 FEL Expression Evaluation

FEL expressions in `inputMapping`, `idempotencyKeyExpression`, and `contextMapping` are evaluated using the kernel's evaluation context (Kernel S7). The processor MUST delegate FEL evaluation to a Formspec-conformant processor (Core S1.4). FEL expressions use only built-in functions (Core S3.5) and extension functions (Core S3.12).

---

### 9.3 Idempotency Keys

The `invokeService` action supports an optional `idempotencyKey` property. When present, the processor MUST use this key to deduplicate service invocations. This closes the crash-between-invoke-and-persist window: if the processor crashes after invoking the service but before persisting the result, the service can be safely re-invoked with the same idempotency key.

### 9.4 Correlation Keys

External signals and callbacks carry a `correlationKey` that the processor uses to route the response to the correct workflow instance. Correlation keys are fundamental for any workflow awaiting external callbacks.

### 9.5 Compensation Seam

Actions MAY declare a `compensatingAction` -- a semantically meaningful reversal. Scopes MAY be marked `compensable: true`. This defines the compensation seam only; detailed execution semantics (reverse ordering, pivot steps, forward/backward recovery) are deferred to the Lifecycle Detail companion.

<!-- absorbed-from: companions/lifecycle-detail.md §5 Compensation Execution Algorithm per ADR 0076 D-8 — full content migrated; source section may be removed once cross-references are updated. -->

This section is normative.

### 9.5.1 Overview

The kernel defines the compensation seam: actions MAY declare a `compensatingAction`, and scopes MAY be marked `compensable: true` (Kernel S9.5). This section defines the algorithm for executing compensation when a compensable scope fails.

### 9.5.2 Compensation Log

A Kernel Complete processor MUST maintain a **compensation log** for each compensable scope. The log records, in order, every action that completed successfully within the scope and has a declared `compensatingAction`. The log is append-only during forward execution.

### 9.5.3 Algorithm: Execute Compensation

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

### 9.5.4 Reverse Ordering

Compensation actions execute in the **reverse of forward completion order**. If actions A, B, C completed in that order, compensation runs C's compensating action, then B's, then A's. This preserves semantic consistency -- later actions may depend on earlier ones, so they must be undone first.

### 9.5.5 The Pivot Step

The **pivot step** is the action whose failure triggered compensation. It does NOT receive compensation because it did not complete successfully. The pivot step's failure is recorded in provenance separately from the compensation sequence.

### 9.5.6 Forward Recovery vs. Backward Recovery

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

### 9.5.7 Compensation and Parallel States

When compensating a parallel state scope, each region is compensated independently. The compensation order within each region is reverse completion order. Regions are compensated concurrently (the same parallelism semantics as forward execution).

### 9.5.8 Nested Compensation

If a compensable scope is nested within another compensable scope, compensation of the inner scope does NOT automatically trigger compensation of the outer scope. Each scope is compensated independently. The outer scope's compensation log includes the inner scope as a single entry; if the inner scope's compensation fails, the outer scope records that failure.

### 9.5.9 Compensation Triggering

Compensation is triggered when an action within a compensable scope fails and the scope's `recoveryMode` is `backward`. The triggering mechanism:

1. During forward execution of a compensable scope, the processor appends each completed action (that has a `compensatingAction`) to the scope's compensation log.
2. When an action fails, the processor checks the scope's `recoveryMode`.
3. If `backward`: invoke `executeCompensation(scope, compensationLog)` as defined in S5.3. After compensation completes, the `$compensation.complete` event fires and the lifecycle processes it like any other event (Kernel S4.10) — if a transition matches, it fires; otherwise the event is recorded in provenance.
4. If `forward`: the compensation log is discarded. The processor retries the failed action or evaluates alternative transitions from the current state. No compensation actions execute.

---

### 9.6 Instance Versioning

A workflow instance is bound to its creation-time definition version unless explicitly migrated. When a Kernel Document is updated, existing instances continue executing under the version that created them. Instance migration is an explicit operation that MUST be recorded in provenance.

Version pinning applies equally to Formspec Definitions referenced as contracts. When a Kernel Document references a Formspec Definition via `contractRef`, the version of that Definition is pinned at instance creation time (Formspec Changelog VP-01, VP-02). Instance migration SHOULD use the Formspec Changelog (Changelog S4) to generate migration maps for contract changes between versions.

<!-- absorbed-from: companions/runtime.md §11 Multi-Version Coexistence per ADR 0076 D-8 — full content migrated; source section may be removed once cross-references are updated. -->

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

<!-- absorbed-from: companions/lifecycle-detail.md §6 Timer Semantics per ADR 0076 D-8 — full content migrated; source section may be removed once cross-references are updated. -->

This section is normative.

<!-- absorbed-from: companions/runtime.md §7 Timer Management per ADR 0076 D-8 — full content migrated; source section may be removed once cross-references are updated. -->

This section is normative.

### 9.7.11 Overview

The Lifecycle Detail Companion (S6) defines timer creation, cancellation, reset-on-reentry, and region scoping. This section defines the precision, persistence, and testing requirements.

### 9.7.12 Precision

Timers MUST fire within a declared tolerance of their deadline. The tolerance depends on the timer's duration:

| Timer Duration | Maximum Tolerance |
|----------------|-------------------|
| Under 1 hour | 1 second |
| 1 hour to under 1 day | 1 minute |
| 1 day or longer | 5 minutes |

A tolerance greater than the timer's duration is a conformance violation. A 30-second timer that fires 45 seconds late has violated its tolerance. A 24-hour timer that fires 3 minutes late is conformant.

The processor MAY declare tighter tolerances than these maximums. Tighter tolerances SHOULD be documented.

### 9.7.13 Persistence

Timer state is part of the CaseInstance (S3.1). Timers are persisted at every durability checkpoint (S6.2). After a processor restart, all pending timers MUST be reconstituted from the persisted state and scheduled for firing at their original deadlines. Timers whose deadlines have passed during the outage MUST fire immediately on restart, in deadline order.

### 9.7.14 Simulated Time

Conformance test processors MAY implement simulated time via the `advanceTime` operation (S3.3). Under simulated time, the processor does not use wall-clock time for timer firing -- instead, `advanceTime` fires all timers whose deadline is at or before the specified timestamp.

Production processors MUST use wall-clock time. A production processor that implements `advanceTime` MUST restrict it to administrative or testing contexts and MUST NOT expose it as a normal operational API.

### 9.7.15 Timer Ordering

When multiple timers fire at the same logical instant (same deadline, or multiple deadlines passed during an outage), the processor MUST fire them in deadline order. Ties (identical deadlines) are broken by timer creation order. This ordering is deterministic and observable via provenance.

---

### 9.7.1 Overview

The kernel defines five timeout categories (Kernel S9.7) and the `startTimer`/`cancelTimer` actions (Kernel S9.2). This section defines the detailed timer lifecycle and its interaction with the state machine.

### 9.7.2 Timer Creation

A `startTimer` action creates a durable timer with:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `timerId` | string | REQUIRED | Unique timer identifier within the workflow instance. |
| `duration` | string (ISO 8601) | CONDITIONAL | Relative duration. One of `duration` or `deadline` MUST be specified. |
| `deadline` | string (ISO 8601) | CONDITIONAL | Absolute deadline. |
| `event` | string | REQUIRED | Event to emit when the timer fires. |

When the timer fires, it emits the declared event as a kernel-generated event with the `$timeout.` prefix (Kernel S4.10). The event is processed by the lifecycle like any other event.

### 9.7.3 Timer Cancellation

A `cancelTimer` action cancels a running timer. If the timer has already fired, the cancellation has no effect. If the timer does not exist, the cancellation is a no-op (not an error).

### 9.7.4 Timer Reset on Reentry

When a state with a `startTimer` action in its `onEntry` is exited and then re-entered (Kernel S4.11), the original timer is cancelled (if still active) and a new timer is created. This prevents stale timers from firing for states that have been re-entered.

### 9.7.5 Timers and Parallel States

Timers scoped to a parallel region are cancelled when the region is cancelled (S4.4). A timer created in region A does not affect region B. Timer events are routed to the region that created them.

### 9.7.6 Timer Durability

Per Kernel S9.1 (G4: Durable Timers), timers MUST survive process restarts, fire within tolerance, and consume no runtime resources while waiting. The tolerance for timer firing is implementation-defined but SHOULD be documented. A tolerance greater than the timer's duration is a conformance violation.

### 9.7.7 Timer Provenance

Timer creation, cancellation, and firing each produce provenance records:

| Record Type | When |
|-------------|------|
| `timer.created` | A `startTimer` action executes. |
| `timer.cancelled` | A `cancelTimer` action executes or a timer is cancelled by region cancellation or state reentry. |
| `timer.fired` | A timer fires and emits its event. |

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

Custody postures are declared, not inferred. Bindings that populate this seam MUST declare, at minimum: who may read content during ordinary operation, whether recovery can occur without the user, and whether delegated compute exposes content to ordinary service components. When Governance (Layer 1) is adopted, custody transitions (changes to any of those answers) are recorded as canonical lifecycle facts (Governance S2.9).

The kernel does NOT define the concrete Trust Profile object. Trellis (the distributed-trust binding) defines that object and binds it to this seam. A monolithic binding may populate this seam with a single declared posture (e.g., "provider-readable, no recovery without user, no delegated compute") and satisfy conformance.

The WOS-owned authored-record wire surface that crosses this seam is defined separately in [WOS Custody Hook Encoding](custody-hook-encoding.md). That companion pins the four-field append input, TypeID rules, JSON→dCBOR conversion discipline, WOS-owned idempotency input, and minimum receipt contract.

### 10.6 `extensions` and `x-` Keys

**Purpose:** Standard escape hatch for vendor and implementation-specific data.

WOS supports two parallel mechanisms for vendor extensions:

1. **`extensions` property.** An object whose keys MUST be prefixed with `x-`. Implementations that group vendor data under a single container SHOULD use this property.
2. **`x-`-prefixed keys on any object.** Any object in a WOS document MAY carry sibling keys prefixed with `x-` alongside its declared properties. This matches the extension convention used by OpenAPI and is the recommended mechanism for decorating individual structural elements (states, transitions, actors, actions) with vendor metadata.

Both mechanisms are equivalent in authority: a processor MUST NOT reject a document on the grounds that vendor data appears at one location rather than the other. Processors SHOULD preserve `x-` keys through read/write cycles (round-trip fidelity) but MAY strip them during normalization passes; any such behavior MUST be documented by the processor.

**Naming rules for `x-` keys:**

- Keys MUST be lowercase ASCII. `X-Vendor-Foo` is REJECTED by conformant validators.
- Keys MUST follow the shape `x-<namespace>-<name>` where `<namespace>` identifies the publisher (e.g., `x-acme-tenant-id`, `x-camunda-priority`).
- The prefix `x-wos-` is **RESERVED** for future normative use by this specification. Implementations and vendors MUST NOT author keys beginning with `x-wos-` until a future spec version publishes them under that namespace.

Rationale for the two-mechanism design: the `extensions` container is useful when vendor data is itself structured and opaque; sibling `x-` keys are useful when vendor data decorates specific structural elements (e.g., `x-acme-ui-label` on a single state). Forcing all vendor data into a single container loses the structural locality that makes element-level decoration valuable. Forcing all vendor data to sibling keys obscures the distinction between data intended for WOS processors and data intended for vendor tooling.

---

## 11. Runtime Serialization
<!-- absorbed-from: companions/runtime.md §3 Instance Lifecycle (CaseInstance serialization) per ADR 0076 D-8 — moved verbatim with renumbered headers; cross-references to runtime.md own subsections remain valid. -->

### 11.1 CaseInstance

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
| `activeTasks` | array of ActiveTask | REQUIRED | Durable nonterminal task state. Empty array when no tasks are active. Terminal task history lives in provenance. |
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

### 11.2 Configuration Ordering

The `configuration` array is ordered by document declaration order, depth-first. Given a kernel document where states are declared in the order `[intake, review, review.initial, review.detailed, determination, complete]`, a configuration of `[review.initial, determination]` (two parallel regions) preserves that order. Two conformant processors serializing the same runtime state MUST produce the same `configuration` array.

### 11.3 Instance Operations

A conformant processor MUST support the following operations on CaseInstance:

| Operation | Input | Effect | Provenance |
|-----------|-------|--------|------------|
| `create` | Kernel Document URL + version, initial case state | Creates a new runtime instance in the kernel's initial state. Runtime allocation, not governed-case birth. | `instanceCreated` |
| `acceptIntakeHandoff` | IntakeHandoff, policy decision, idempotency token | Acknowledges a Formspec intake handoff, records WOS-owned intake provenance, and either attaches it to an existing governed case or births a new governed case. | `intakeAccepted`, `intakeRejected`, or `intakeDeferred`; `caseCreated` when a new governed case is born |
| `processEvent` | Event (name, actor, data, idempotency token) | Evaluates the transition algorithm (Lifecycle Detail S2). | `stateTransition` or `unmatchedEvent` |
| `advanceTime` | Target timestamp | Fires all timers whose deadline is at or before the target timestamp, in deadline order. | `timer.fired` per timer |
| `migrate` | New definition URL + version, migration map | Changes the governing definition (S11). | `instanceMigrated` |
| `suspend` | Reason | Sets status to `suspended`. No events are processed while suspended. | `instanceSuspended` |
| `resume` | (none) | Sets status to `active`. Pending events (if any) are processed. | `instanceResumed` |
| `terminate` | Reason | Sets status to `terminated`. Irreversible. | `instanceTerminated` |

Every operation produces at least one provenance record. A `processEvent` that fires a transition produces the transition provenance defined in Kernel S4.7 and Kernel S8.2.

### 11.4 Intake Acceptance

The `create` operation allocates runtime state. It does not, by itself, establish a governed case. Intake acceptance is a separate host operation: the processor consumes a Formspec `IntakeHandoff`, records intake provenance, and only then decides whether the handoff attaches to an existing governed case or births a new one. In a public-intake flow, `instanceCreated` and `caseCreated` MAY both occur, but they remain distinct records with distinct meanings.

#### 11.4.1 Normative `acceptIntakeHandoff` Algorithm

A conformant processor implementing `acceptIntakeHandoff` MUST apply the following sequence:

1. Resolve the configured intake adapter for the requested binding. Unsupported bindings fail before any receipt, provenance, or case mutation is written.
2. Parse and validate the binding-native handoff through that adapter. For Formspec this means validating the `IntakeHandoff` document and deriving at least:
   - stable intake identity,
   - whether the handoff targets an existing governed case or requests governed-case creation,
   - the binding-owned evidence references carried by the handoff.
3. Resolve replay identity before host policy runs. At minimum, replay identity MUST distinguish:
   - binding discriminator,
   - stable intake identity,
   - the full host request fields that can affect acceptance outcome.
   A replay with the same binding and intake identity but different policy-relevant request metadata MUST fail as a conflict, not silently replay.
4. Persist a durable intake receipt before applying any accepted side effects. A crash between receipt creation and case mutation MUST replay to exactly one accepted/rejected/deferred outcome; it MUST NOT duplicate provenance on an existing case nor mint a second governed case for the same intake identity.
5. Evaluate host acceptance policy against the adapter interpretation. The policy outcome space is closed to:
   - `accepted` with either `attachToExistingCase` or `createGovernedCase`,
   - `rejected`,
   - `deferred`.
6. Finalize binding-owned provenance after the policy outcome is known. The runtime owns the intake decision records; bindings MAY add seam-specific provenance such as Formspec-facing `caseCreated` evidence for accepted public-intake creation.
7. Apply accepted side effects only after steps 1-6 have succeeded:
   - `attachToExistingCase` appends intake provenance to an already-governed case,
   - `createGovernedCase` creates runtime instance state if needed and then appends intake provenance.
   The processor MUST canonicalize any host alias or legacy case handle before persisting accepted outcome and case provenance, while still preserving the handoff-carried attach string for any adapter finalization step that compares against the source handoff.
8. Persist the applied intake receipt after case mutation or detached provenance completion. Subsequent replays MUST return the applied durable decision.

#### 11.4.2 Required Checks

The runtime layer owns the acceptance algorithm even when concrete storage and transport are host-specific. A conformant processor SHOULD verify, before accepted case mutation:

- the handoff's pinned definition still resolves to the exact referenced Definition version;
- the stored canonical Response envelope still matches `responseHash`;
- any stored ValidationReport referenced by the handoff still resolves;
- the requested attach target or create target remains valid for the current tenant/scope boundary;
- public-intake creation is authorized by host policy for the current route or product profile.

When the processor cannot perform one of these checks itself because the relevant artifact store or identity system is host-supplied, it MUST expose a host seam for that verification and MUST NOT redefine the check as adapter-private behavior.

#### 11.4.3 Outcome Semantics

The runtime owns the intake decision records:

- `intakeAccepted` means the handoff was accepted into WOS-managed workflow handling.
- `intakeRejected` means the handoff was declined and did not create or update governed case state.
- `intakeDeferred` means the handoff was received but withheld from governed case mutation pending a later host decision.

When acceptance births a governed case, `caseCreated` records the governed-case boundary. It is not interchangeable with `instanceCreated`, which only records runtime allocation of instance state.

### 11.5 Status Transitions

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

## 12. Evaluation Modes

<!-- absorbed-from: companions/runtime.md §10 Evaluation Modes per ADR 0076 D-8 — moved verbatim with renumbered headers; cross-references to runtime.md own subsections remain valid. -->

This section is normative.

### 12.1 Overview

The Kernel Document MAY declare an `evaluationMode` property on the top-level document. The evaluation mode determines when the processor evaluates transition guards.

### 12.2 Event-Driven Mode (Default)

In `event-driven` mode (the default), transition guards are evaluated only when an explicit event arrives. Case state mutations (`setData` actions, contract validation results) do not trigger guard re-evaluation. This is the standard statechart evaluation model.

### 12.3 Continuous Mode

In `continuous` mode, after any case state mutation -- whether from a `setData` action, a contract validation result, or an external signal -- the processor re-evaluates all guards in the current configuration. If any guard that was previously `false` now evaluates to `true`, the corresponding transition fires. The triggering mutation is recorded in provenance.

**Convergence cap.** To prevent infinite loops (a `setData` in `onEntry` triggers re-evaluation, which fires a transition whose `onEntry` does another `setData`), the processor imposes a convergence cap of **100 re-evaluation cycles** per triggering mutation. If the cap is reached, the processor MUST:

1. Halt re-evaluation for this mutation.
2. Record a `convergenceCapReached` provenance record with the mutation that triggered the cycle.
3. Continue processing subsequent events normally.

Transitions fired during a convergence cycle are committed -- they have already emitted provenance and mutated case state. The cap halts *further re-evaluation*, not the effects of transitions that already fired.

Timer-driven mutations (timer expiry firing a `$timeout.*` event whose actions include `setData`) trigger re-evaluation in continuous mode, subject to the same convergence cap. The `$timeout.*` event is the triggering mutation for the re-evaluation cycle.

The convergence cap value (100) matches Formspec's processing model convergence behavior for consistency across the ecosystem.

### 12.4 Mode Declaration

```json
{
  "$wosWorkflow": "1.0",
  "url": "https://example.gov/workflows/continuous-eval-demo",
  "version": "1.0.0",
  "title": "Continuous Evaluation Demo",
  "impactLevel": "operational",
  "evaluationMode": "continuous",
  "actors": [
    { "id": "system", "type": "system" }
  ],
  "lifecycle": {
    "initialState": "start",
    "states": {
      "start": { "type": "atomic" },
      "done": { "type": "final" }
    }
  }
}
```

When `evaluationMode` is absent, the default is `event-driven`. A conformant processor MUST support both modes.

---

## 13. Formspec Coprocessor

<!-- absorbed-from: companions/runtime.md §15 Formspec Coprocessor (15-step protocol) per ADR 0076 D-8 — moved verbatim with renumbered headers; cross-references to runtime.md own subsections remain valid. -->

This section is normative.

The Formspec coprocessor protocol defines how a WOS task bound to a Formspec Definition is presented, saved, submitted, validated, mapped to case state, and recorded in provenance. WOS delegates Formspec processing semantics to a Formspec-conformant processor. WOS defines only the orchestration envelope around that processor.

### 13.1 Applicability

This protocol applies to a kernel `createTask` action whose `contractRef` resolves to a ContractReference with `binding: "formspec"`.

A Formspec task has one completion bundle: one task, one pinned Formspec Definition, and one full Formspec Response. Multi-form packets MUST be modeled as multiple coordinated tasks or as one composite Formspec Definition. A processor MUST NOT treat one WOS task as a collection of independent Formspec contracts unless a later WOS version defines multi-contract task semantics.

### 13.2 Task Context

When a Formspec-backed `createTask` action executes, the processor resolves the ContractReference and creates an ActiveTask entry in CaseInstance `activeTasks`.

The processor then constructs a FormspecTaskContext:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `taskId` | string | REQUIRED | Processor task identifier. Stable for idempotency and provenance. |
| `instanceId` | string (URI) | REQUIRED | WOS CaseInstance identifier. |
| `contractRef` | string | REQUIRED | Kernel contract map key used by the task. |
| `definitionUrl` | string (URI) | REQUIRED | Formspec Definition `url`. MUST match `response.definitionUrl`. |
| `definitionVersion` | string | REQUIRED | Pinned Formspec Definition version. MUST match `response.definitionVersion`. |
| `binding` | string | REQUIRED | MUST be `formspec`. |
| `assignedActor` | string | REQUIRED | Actor assigned by the `createTask` action. |
| `prefillData` | object | OPTIONAL | Host-provided initial values for rendering. |
| `prefillMappingRef` | string (URI) | OPTIONAL | Mapping document used to prefill the Response. |
| `responseMappingRef` | string (URI) | OPTIONAL | Mapping document used to project a completed Response into case state. |
| `deadline` | string (datetime) | OPTIONAL | Task deadline. |
| `impactLevel` | string | OPTIONAL | Effective impact level for task-level governance. Defaults to the Kernel `impactLevel`. |
| `extensions` | object | OPTIONAL | Extension data. Keys MUST be prefixed with `x-`. |

The processor calls `TaskPresenter.presentTask(context)`. Presentation MUST produce a `taskPresented` Facts-tier provenance record and MUST NOT mutate case state.

### 13.3 Mapping Profiles

`responseMappingRef` controls Response-to-case mutation. A processor MUST NOT invent a host-defined default projection from Formspec Response data into case state. If `responseMappingRef` is absent, the processor MAY store the Response reference and emit the completion event, but MUST NOT automatically mutate case fields from the Response.

When prefill and response projection use the same transform, `prefillMappingRef` and `responseMappingRef` SHOULD reference the same Mapping document with `direction: "both"` (Mapping S3.1, S3.1.2), and the host MUST claim Mapping Bidirectional conformance. If the host cannot execute reverse or bidirectional Mapping, prefill is a host-local behavior and is not portable WOS semantics. Separate one-way Mapping documents MAY be used when prefill and response projection are intentionally different.

### 13.4 Drafts and Abandonment

`submitTaskResponse` is the completion operation and accepts only a full Formspec Response with `status: "completed"`.

Draft persistence is separate. A processor MAY expose `persistTaskDraft(taskId, response, actorId, timestamp, idempotencyToken)`. That operation accepts a full Formspec Response with `status: "in-progress"`, `status: "amended"`, or `status: "stopped"`. It MUST be idempotent, MUST record `taskDraftPersisted` provenance, and MUST NOT mutate case state, emit `completionEvent`, or advance task lifecycle to `completed`.

Host UI dismissal is not abandonment. `dismissTask` records `taskDismissed` provenance and leaves the task resumable in its current state. Deliberate abandonment uses `persistTaskDraft` with `status: "stopped"` or a deployment-specific `abandonTask` operation. Unless the workflow explicitly maps the rationale to `skipped`, deliberate abandonment transitions the task to `failed`, records `taskFailed` provenance, emits `failureEvent` when configured, and applies Governance S8 remediation when configured. If the workflow maps the abandonment rationale to `skipped`, the task transitions to `skipped`, records `taskSkipped` provenance with the structured rationale required by Governance S10.1, emits no `completionEvent` or `failureEvent`, and is removed from `activeTasks`.

### 13.5 Submission Algorithm

`submitTaskResponse(taskId, response, actorId, timestamp, idempotencyToken)` submits a completed Formspec Response to the processor.

The processor MUST execute the following algorithm:

1. If `idempotencyToken` is present, check a durable replay store before resolving `activeTasks`, using a replay key scoped to `taskId`, `actorId`, and `idempotencyToken`. A duplicate replay key MUST return the same outcome and MUST NOT re-run authorization, validation, mapping, provenance, task completion, or event emission. A token used by a different actor MUST NOT replay another actor's outcome. This replay key covers every later outcome, including `taskResponseRejected`, `taskFailed`, and `taskCompleted`, and MUST outlive removal from `activeTasks` for the host retry window.
2. Resolve the ActiveTask by `taskId`. If no active task exists, reject with `taskNotFound`.
3. Authorize `actorId` against the task's `assignedActor`. `actorId` MUST match `assignedActor` unless AccessControl or Governance delegation allows the substitution. If authorization fails, reject with `taskSubmitterUnauthorized`, record `taskResponseRejected` provenance when policy allows, and do not advance lifecycle, emit `completionEvent` or `failureEvent`, record `taskResponseSubmitted` or `taskFailed`, or mutate case state.
4. If the actor is an agent, the actor MUST be registered through `actorExtension` and provenance MUST record `actorType: "agent"` plus agent identity, model/version, confidence/source metadata when available, and any `principalActorId` or `delegationRef`. Rights-impacting and safety-impacting respondent submissions still require a human or legally delegated authority. If these agent requirements fail, reject with `agentSubmitterUnauthorized`, record `taskResponseRejected` provenance when policy allows, and do not advance lifecycle, emit `completionEvent` or `failureEvent`, record `taskResponseSubmitted` or `taskFailed`, or mutate case state.
5. If `response.status` is not `completed`, reject with `taskResponseStatusNotCompleted`, record `taskResponseRejected` provenance when policy allows, and do not advance lifecycle, emit `completionEvent` or `failureEvent`, record `taskResponseSubmitted` or `taskFailed`, or mutate case state.
6. Record `taskResponseSubmitted` provenance for this new completed submission attempt.
7. Validate the full Response envelope against Formspec `response.schema.json` and Core S2.1.6, including the schema's root additional-property and `data` rules.
8. Verify the pin: `response.definitionUrl` MUST equal the task `definitionUrl`; `response.definitionVersion` MUST equal the task `definitionVersion`.
9. Delegate Definition validation over `response.data` to a Formspec-conformant processor (Core S1.4, S2.4, S5-S5.4).
10. Record `contractValidation` provenance with the envelope, pin, and Definition validation outcome.
11. If envelope validation, pin validation, or Definition validation fails, record `taskFailed` provenance, transition the task to `failed`, emit `failureEvent` when configured, apply Governance S8 remediation, do not map data, and do not mutate case state.
12. If Respondent Ledger evidence is required by S15.7 and missing, reject with `ledgerEvidenceMissing`, record `taskFailed` provenance, transition the task to `failed`, emit `failureEvent` when configured, apply Governance S8 remediation, do not map data, and do not mutate case state.
13. Resolve `responseMappingRef`. If absent, record the accepted Response reference and skip automatic case mutation.
14. If `responseMappingRef` is present, execute the Formspec Mapping document in the forward direction (Mapping S3.4, S8) to compute a proposed case mutation bundle. The processor MUST NOT commit the mutation yet. The processor MUST record `dataMapping` provenance for the proposed mapping outcome.
15. Run optional `contractHook` / Governance S5 checks on the completion bundle and record `contractValidation` provenance for each post-pass outcome. These hooks SHOULD validate disjoint case-level concerns and MUST NOT repeat Formspec Definition validation on the same Response.
16. If an optional hook fails, record `taskFailed` provenance, leave case state unchanged, transition the task to `failed`, emit `failureEvent` when configured, and apply Governance S8 remediation.
17. Atomically commit case mutation, task completion, `completionEvent` emission when configured, and `taskCompleted` provenance. The case mutation MAY be empty when `responseMappingRef` is absent.
18. The task transitions to `completed`, is removed from `activeTasks`, and terminal task history remains in provenance.

If the Formspec processor is unavailable, the processor MUST reject with `processorUnavailable` or return a retryable failure without case mutation. Hosts SHOULD retry with the same idempotency token.

### 13.6 ValidationOutcome

The Formspec task validator returns a WOS `ValidationOutcome` wrapper:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `envelopeValid` | boolean | REQUIRED | Whether the Response envelope validated against `response.schema.json`. |
| `pinMatch` | boolean | REQUIRED | Whether the Response Definition pin matched the task pin. |
| `definitionValid` | boolean | REQUIRED | Whether Definition validation over `response.data` passed. |
| `errors` | array | REQUIRED | WOS-level validation errors, including envelope and pin errors. |
| `validationResults` | array | OPTIONAL | Formspec-shaped ValidationResult entries from Definition validation. |

`ValidationOutcome` is a WOS wrapper. It does not replace Formspec ValidationResult or ValidationReport semantics.

### 13.7 Ledger and Notice

For respondent-facing Formspec tasks in `rights-impacting` or `safety-impacting` workflows, the processor MUST require Respondent Ledger evidence for the submit boundary before accepting completion. If required evidence is missing, the processor MUST follow the `ledgerEvidenceMissing` failure path in S15.5.

For respondent-facing Formspec tasks in `operational` workflows, Respondent Ledger evidence SHOULD be present. For `informational` workflows, it MAY be present.

Respondent Ledger evidence and legal notice delivery are separate. The Respondent Ledger proves respondent-side Response history. WOS Notification Template, Correspondence Metadata, and Facts-tier provenance prove notice generation or delivery. A ledger reference to a notice record does not itself satisfy notice obligations.

### 13.8 Provenance Records

The processor SHOULD use these Facts-tier provenance action names for Formspec task flows:

| Action | When recorded |
|--------|---------------|
| `taskPresented` | `presentTask` is called. |
| `taskDismissed` | `dismissTask` records a UI close without lifecycle advancement. |
| `taskDraftPersisted` | A draft, amendment draft, or stopped Response is durably recorded. |
| `taskResponseRejected` | `submitTaskResponse` rejects a non-terminal submission attempt before accepting it for validation. |
| `taskResponseSubmitted` | `submitTaskResponse` receives a completed Response. |
| `contractValidation` | Envelope, pin, Definition, or post-pass validation executes. |
| `dataMapping` | A Mapping document computes proposed or committed case mutations. |
| `taskCompleted` | The task completes and is removed from `activeTasks`. |
| `taskFailed` | The task fails validation, ledger gating, abandonment, or a post-pass hook. |
| `taskSkipped` | The task is deliberately skipped as not applicable and is removed from `activeTasks`. |

Provenance records SHOULD include `taskId`, `responseId` when available, `definitionUrl`, `definitionVersion`, `mappingRef` when used, `respondentLedgerRef` when required or available, and the actor fields required by Kernel S8.2 plus any agent metadata required by AI Integration S3.1.

### 13.9 Amendments

**Five-mode revisit taxonomy.** A Workflow Document that embeds Workflow Governance MAY declare `governance.amendmentTaxonomy`: an array of distinct string literals naming which governed-revisit modes the workflow permits. When present, each array item MUST be exactly one of the following **five** closed literals (no vendor extensions at this key; structural enforcement is normative in `wos-workflow.schema.json` under `Governance.properties.amendmentTaxonomy`):

| Literal | Meaning (kernel-level summary) |
|---------|--------------------------------|
| `correction` | Factual correction alongside a preserved determination record (narrow field-set semantics per governance). |
| `amendment` | Substantive change to a determination on the **same** governed chain. |
| `supersession` | Replacement by a new governed case or chain. Declaring `supersession` here permits the mode in governance taxonomy; **cross-case linkage** is expressed through **`caseRelationships`** (Kernel S5.5), not through additional `amendmentTaxonomy` values. |
| `rescission` | Withdrawal of a determination to a non-operative state on the same chain. |
| `reinstatement` | Re-activation of a determination after rescission **without** rewriting the substantive determination value (distinct from `amendment`). |

When `governance.amendmentTaxonomy` is absent, author-time validation treats the workflow as asserting no amendment-taxonomy modes at this key (see `wos-workflow.schema.json` for the default and lint posture).

After a Formspec task completes, amendment flows MUST create a new task. A processor MUST NOT reopen a terminal completed task. The amended task or Response SHOULD reference the original through Respondent Ledger `amendmentRef` and WOS provenance fields such as `supersedesResponseId` or `relatedTaskId`.

This preserves immutable completion history while allowing corrected or updated Responses to supersede earlier submissions.

---

## 14. Separation Principles

<!-- renumbered-from: kernel §12 → kernel §14 per ADR 0076 D-8 amendment 2026-04-28 (append-at-end preserves existing §2 Conformance Classes + §6 Impact Level Classification anchors). -->

This section is normative.

1. **Lifecycle state MUST be separated from case state.** The lifecycle tracks where the workflow is. Case state tracks what data exists. These are independent.

2. **Process topology MUST be separated from decision logic.** The lifecycle defines structure. Decision services evaluate conditions. A guard MAY invoke a decision service, but the decision service MUST NOT contain process topology.

3. **Audit MUST be separated from execution.** Provenance records are produced as a consequence of execution but do not participate in control flow.

4. **Governance MUST be separated from orchestration.** The kernel orchestrates. Layers govern. The kernel provides seams; governance attaches through them.

---

## 15. Contract Validation

<!-- renumbered-from: kernel §11 → kernel §15 per ADR 0076 D-8 amendment 2026-04-28 (append-at-end preserves existing §2 Conformance Classes + §6 Impact Level Classification anchors). -->

This section is normative.

### 15.1 Contract Interface

WOS uses interface contracts to define typed data exchanges between workflow participants. A contract MUST provide:

1. **Typed fields** with declared data types.
2. **Validation rules** producing structured results.
3. **Structured validation results** with severity, field path, message, and constraint identifier.

### 15.2 Conformant Bindings

Two conformant bindings are defined:

| Binding | Capabilities | When to Use |
|---------|-------------|-------------|
| **Formspec** (recommended) | Full reactive behavior, cross-field validation (Shapes), Mapping DSL for data flow. | WOS-native implementations. Human task contracts. |
| **JSON Schema** (baseline) | Structural validation only. No reactive behavior or cross-field rules. | Minimum viable integration. |

### 15.3 Processing Delegation

WOS processors MUST delegate contract evaluation to a conformant contract processor. For Formspec bindings, this means a Formspec-conformant processor (Core S1.4). The WOS processor provides orchestration context; the contract processor provides Definition evaluation.

For Formspec-backed tasks, a ContractReference MAY also declare `prefillMappingRef` and `responseMappingRef`. `prefillMappingRef` identifies the Mapping document used to populate the task's initial Response. `responseMappingRef` identifies the Mapping document used by Runtime Companion S15 to project a completed Response into case state. When no `responseMappingRef` is supplied, Runtime Companion S15 forbids automatic host-defined Response-to-case projection.

---

## 16. Host Interfaces

<!-- absorbed-from: companions/runtime.md §12 Host Interfaces per ADR 0076 D-8 — full content migrated. The host interface contracts (ProvenanceSigner, ReportRenderer, ContractValidator, EventStore, AccessControl, etc.) are normative obligations on the host that runs a WOS processor; they live in the kernel because they are the boundary between processor responsibility and host responsibility. -->

This section is normative.

The processor expects its host to provide implementations of the following interfaces. Each interface is a named behavioral contract with required operations and error semantics. These are spec-level interface definitions -- implementations map them to their language's type system (Rust traits, Java interfaces, TypeScript abstract classes, Python protocols).

### 16.1 InstanceStore

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

### 16.2 DocumentResolver

Loads WOS documents (kernel, governance, sidecars) from storage.

| Operation | Input | Output | Description |
|-----------|-------|--------|-------------|
| `resolveKernel` | url: string, version: string | KernelDocument | Resolve a Kernel Document by URL and version. |
| `resolveGovernance` | url: string, version: string | GovernanceDocument | Resolve a Governance Document. |
| `resolveSidecar` | url: string, anchorDate: string (optional) | SidecarDocument | Resolve a sidecar document. When `anchorDate` is provided, used for temporal parameter resolution (Governance S13). |

Error conditions: `documentNotFound`, `versionNotFound`, `resolverUnavailable`.

### 16.3 ContractValidator

Validates data against a Formspec Definition or JSON Schema contract.

| Operation | Input | Output | Description |
|-----------|-------|--------|-------------|
| `validate` | contractRef: string, data: object | ValidationResult | Validate data against the referenced contract. Returns `{ valid: boolean, errors: array }`. |

For Formspec bindings, the ContractValidator MUST delegate to a Formspec-conformant processor (Core S1.4). The WOS processor provides orchestration context; the Formspec processor provides Definition evaluation.

Error conditions: `contractNotFound`, `processorUnavailable`.

### 16.4 ExternalService

Fulfills `invokeService` actions (Kernel S9.2).

| Operation | Input | Output | Description |
|-----------|-------|--------|-------------|
| `invoke` | serviceRef: string, input: object, idempotencyKey: string (optional), timeout: duration (optional) | object | Invoke the referenced service. Returns the service response. |

The processor MUST pass the `idempotencyKey` to the service when provided (Kernel S9.3). The service is responsible for deduplication at the service level; the processor is responsible for deduplication at the instance level.

Error conditions: `serviceNotFound`, `serviceTimeout`, `serviceError` (with error payload).

### 16.5 AccessControl

Controls which actors can perform which operations.

| Operation | Input | Output | Description |
|-----------|-------|--------|-------------|
| `canRead` | actorId: string, fieldPath: string | boolean | Whether the actor can read the specified case state field. |
| `canTransition` | actorId: string, transition: Transition | boolean | Whether the actor can trigger this transition. |
| `canDelegate` | delegatorId: string, delegateId: string, scope: DelegationScope | boolean | Whether the delegator can delegate authority to the delegate within the given scope. |

A processor MAY use a permissive default implementation (all operations return `true`) for single-user or testing deployments. Production deployments with multiple actors SHOULD use a restrictive implementation.

### 16.6 ProvenanceSigner

Signs and verifies provenance records for cross-organization trust.

| Operation | Input | Output | Description |
|-----------|-------|--------|-------------|
| `sign` | record: ProvenanceRecord | SignedRecord | Attach a cryptographic signature to a provenance record. |
| `verify` | signedRecord: SignedRecord | boolean | Verify that a signed record's signature is valid and the content has not been tampered with. |

Single-organization deployments MAY use a no-op implementation. Cross-organization deployments (Federation Profile) MUST use a signing implementation.

### 16.7 ReportRenderer

Renders provenance and case state into human-readable formats.

| Operation | Input | Output | Description |
|-----------|-------|--------|-------------|
| `renderExplanation` | explanation: ExplanationStructure, template: string | rendered output | Render an assembled explanation (S9) into a human-readable format. |
| `renderAudit` | provenanceLog: array, format: string | rendered output | Render an audit trail into the specified format. |

The output format is implementation-defined. Common formats: PDF, HTML, plain text, accessible HTML with ARIA annotations.

### 16.8 EventQueue

Manages the per-instance event queue for serial processing (S4.1).

| Operation | Input | Output | Description |
|-----------|-------|--------|-------------|
| `enqueue` | instanceId: string, event: Event | (none) | Add an event to the instance's processing queue. |
| `dequeue` | instanceId: string | Event or empty | Remove and return the next event for processing. Returns empty if the queue is drained. |
| `peek` | instanceId: string | Event or empty | Return the next event without removing it. |

The EventQueue is a logical abstraction -- implementations MAY use an in-process queue, a message broker, or a database-backed queue. The only requirement is FIFO ordering per instance (S4.1). The queue MUST be durable: events enqueued but not yet processed MUST survive processor restarts.

Error conditions: `queueUnavailable`.

### 16.9 TaskPresenter

Presents Formspec-backed tasks to the host user interface.

| Operation | Input | Output | Description |
|-----------|-------|--------|-------------|
| `presentTask` | context: FormspecTaskContext | (none) | Render the referenced Formspec Definition for the assigned actor. Presentation alone MUST NOT mutate case state. |
| `dismissTask` | taskId: string, reason: string | (none) | Record that the host UI was closed without completion. Dismissal MUST NOT complete, fail, or skip the task. |

The TaskPresenter is a host interface. The processor owns task lifecycle and case mutation semantics; the host owns rendering, local draft buffering, and user interaction. A host MAY call `dismissTask` when the actor closes a browser tab or modal. Deliberate abandonment is not dismissal; it uses the S15 draft/abandonment path.

Error conditions: `taskNotFound`, `presentationUnavailable`, `actorUnavailable`.

---

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
