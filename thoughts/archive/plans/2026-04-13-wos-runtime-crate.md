# Implementation Plan: Build `wos-runtime`

**Date:** 2026-04-13
**Status:** Completed
**Author:** Formspec project

---

## Goal

Build a new `wos-runtime` crate that turns `wos-core` from a pure evaluator into a reusable runtime surface for real processors.

`wos-runtime` should own the generic orchestration layer:

- instance creation and loading
- per-instance event enqueue and drain
- timer wake-up handling
- provenance append coordination
- host-managed action dispatch
- Formspec task presentation and submission flow from Runtime S15

It should **not** own engine-specific durability or workflow infrastructure. Temporal, Camunda, and other bindings should sit **above** this crate, not inside it.

---

## Why This Crate Exists

The repo now has a clean lower layer but no reusable runtime layer:

- `wos-core` owns typed models, evaluator logic, timers, provenance types, and host traits.
- `wos-lint` owns static verification.
- `wos-conformance` owns fixture-driven behavioral verification.

What is still missing is the crate that a real processor would embed to do the work described in the Runtime Companion:

- load a `CaseInstance`
- dequeue one event
- evaluate it through `wos-core`
- append provenance durably
- execute host-managed actions
- persist the new instance atomically
- drive Formspec-backed task flows through `TaskPresenter` and `submitTaskResponse`

Without that crate, every future binding will reassemble the same orchestration logic differently. That is the wrong seam.

---

## Architectural Decision

`wos-runtime` is a **generic runtime crate**, not the first engine binding.

### `wos-runtime` should do

- provide the processor-facing API for WOS runtime operations
- coordinate `wos-core` with storage, queues, provenance, idempotency, and task presentation
- implement the Runtime Companion S12 and S15 orchestration rules in one place
- provide an in-memory reference runtime for tests, demos, and single-process use

### `wos-runtime` should not do

- embed Temporal workflow code
- own Postgres schemas specific to one deployment
- render forms or reviewer UI
- implement Formspec semantics inline
- duplicate `wos-core` evaluation logic

### Follow-on crates

- `wos-temporal`: Temporal workflow + activities that delegate behavioral decisions to `wos-runtime`
- later engine adapters: Camunda, Flowable, KIE, Step Functions, or SaaS-local workers

This keeps the behavioral contract portable and keeps engine adapters thin.

---

## Current State

### What already exists

- `wos-core` exposes typed documents, `Evaluator`, `CaseInstance`, timers, provenance, and host traits.
- `wos-core` already models `activeTasks`, `FormspecTaskContext`, and `ValidationOutcome`.
- Runtime Companion S12 defines the host interface contract.
- Runtime Companion S15 defines the Formspec coprocessor algorithm.
- `wos-conformance` already has in-memory stubs for `InstanceStore`, `ContractValidator`, and `ExternalService`.
- The Temporal reference architecture already describes one likely first binding.

### What is missing

- no crate owns runtime command flow such as `create_case`, `submit_event`, `drain_until_idle`, or `submit_task_response`
- no shared provenance append/store abstraction
- no shared idempotency/replay abstraction for S15
- no `TaskPresenter` trait in Rust yet
- no shared runtime-side seam for executing Formspec Mapping documents
- no generic runtime loop that ties `InstanceStore`, `EventQueue`, `Evaluator`, and host actions together

### Trait parity gaps in `wos-core`

The current Rust trait surface is close to Runtime S12, but not complete.

The plan should start by reconciling these mismatches:

- `InstanceStore` is missing optional `listByState` and `listByDefinition`
- `DocumentResolver` only resolves kernels today; the spec also names governance and sidecar resolution
- `AccessControl` is missing `canDelegate`
- `ReportRenderer` is missing `renderAudit`
- `EventQueue` is missing `peek`
- `TaskPresenter` is specified in Runtime S12.9 but does not yet exist in `wos-core`
- `ActionExecutor` is too generic for some runtime flows and may need typed action envelopes

Do this first. `wos-runtime` should not be built on a knowingly incomplete host seam.

---

## Target State

```text
SaaS API / worker / engine adapter
                |
                v
           wos-runtime
        /      |       \
       v       v        v
  wos-core  host deps  Formspec processor
   (pure)   (stores,   (validation + mapping)
            queue,
            tasks,
            provenance)
```

`wos-runtime` should expose a small, stable command surface:

- `create_instance`
- `enqueue_event`
- `drain_once`
- `drain_until_idle`
- `persist_task_draft`
- `submit_task_response`
- `dismiss_task`
- `load_instance`
- `load_provenance_window`

The runtime should make one event-processing transaction the unit of correctness.

One event in, one atomic runtime step out.

---

## Proposed Crate Shape

```text
wos-runtime/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── runtime.rs          # WosRuntime, builder, command entry points
│   ├── commands.rs         # create_instance, enqueue_event, drain, task ops
│   ├── transaction.rs      # atomic step boundary and error mapping
│   ├── coprocessor.rs      # Runtime S15 orchestration
│   ├── actions.rs          # createTask, invokeService, follow-up event handling
│   ├── timers.rs           # timer wake-up scheduling and dispatch
│   ├── task.rs             # active task lifecycle helpers
│   ├── provenance.rs       # append coordination and cursor updates
│   ├── idempotency.rs      # replay keys for S15 and host retries
│   ├── host/
│   │   ├── mod.rs
│   │   ├── in_memory.rs    # reference runtime services
│   │   └── adapters.rs     # service bundle wrappers
│   └── testsupport.rs
└── tests/
    ├── happy_path.rs
    ├── task_submission.rs
    ├── draft_abandonment.rs
    ├── timer_recovery.rs
    └── idempotency.rs
```

The exact file split can move. The ownership boundary should not.

---

## Runtime-Owned Interfaces

Some seams belong in `wos-core` because they are spec-level host interfaces. Others belong in `wos-runtime` because they are runtime plumbing, not normative WOS surface.

### Stay in `wos-core`

- `InstanceStore`
- `DocumentResolver`
- `ContractValidator`
- `ExternalService`
- `AccessControl`
- `ProvenanceSigner`
- `ReportRenderer`
- `EventQueue`
- `ActionExecutor`
- `TaskPresenter` once added

### Live in `wos-runtime`

- `ProvenanceStore`
  Stores append-only records and advances `provenancePosition`.
- `IdempotencyStore`
  Persists replay outcomes for `submitTaskResponse` and other retryable entry points.
- `MappingExecutor`
  Executes Formspec Mapping documents for prefill and response projection. This should not be pushed into `wos-core`; it is a Formspec integration seam.
- `Clock`
  Supplies wall-clock time and deadline comparisons without hard-coding `Utc::now()` into the runtime.
- `TaskStateStore` only if needed
  Prefer `CaseInstance.activeTasks` as the canonical workflow state. Add a separate projection store only for dashboard/query efficiency, not semantics.

---

## Phases

### Phase 0: Reconcile `wos-core` Host Trait Parity

Before adding `wos-runtime`, make `wos-core::traits` match the Runtime Companion.

Deliverables:

- add `TaskPresenter`
- add `peek` to `EventQueue`
- add optional query methods to `InstanceStore`
- add governance and sidecar resolution to `DocumentResolver`
- add `canDelegate` to `AccessControl`
- add `renderAudit` to `ReportRenderer`
- review `ActionExecutor` shape against `createTask`, `invokeService`, and host-managed side effects

Success condition:

- the trait surface matches Runtime S12 closely enough that `wos-runtime` does not need workaround traits for normative behavior

### Phase 1: Scaffold `wos-runtime` with an In-Memory Reference Runtime

Create the new crate and land the generic runtime shell.

Deliverables:

- `WosRuntime` builder with injected services
- in-memory implementations for runtime-only stores
- `create_instance`, `load_instance`, `enqueue_event`
- `drain_once` using `EventQueue` + `Evaluator`
- one atomic runtime-step boundary, even if the first implementation is in-memory only

Success condition:

- a single-process test can create a case, enqueue an event, drain it, and observe updated instance state plus appended provenance

### Phase 2: Event Processing, Actions, and Timers

Move from “can process one event” to “behaves like a real runtime.”

Deliverables:

- action dispatch pipeline for host-managed effects
- durable timer wake-up scheduling and timeout event emission
- follow-up event enqueue after action execution
- crash-safe ordering for save, provenance append, and queue advancement
- runtime error taxonomy that distinguishes host failures, validation failures, and deterministic engine failures

Important rule:

`wos-runtime` must keep `wos-core` pure. If action execution needs retries, backoff, or external handles, that belongs here or in the engine adapter.

### Phase 3: Formspec Coprocessor and Task Lifecycle

Implement Runtime S15 in the runtime crate.

Deliverables:

- `present_task` path for Formspec-backed `createTask`
- `persist_task_draft`
- `submit_task_response`
- `dismiss_task`
- pin checks against `definitionUrl` and `definitionVersion`
- delegation to a Formspec-conformant `ContractValidator`
- mapping execution through `MappingExecutor`
- task failure, completion, rejection, and abandonment handling
- idempotency replay store for submission retries

Success condition:

- one end-to-end test can create a Formspec-backed task, present it, submit a completed Response, project the mapped data into case state, emit the configured completion event, and append the expected provenance

### Phase 4: Queries, Projections, and Host-Friendly APIs

Make the crate usable from an API layer and future adapters.

Deliverables:

- query helpers for active state, task list, and recent provenance
- optional projection hooks for dashboard/task queue read models
- clean error mapping for HTTP APIs and worker loops
- typed command results instead of raw `serde_json::Value` where the result shape is known

This phase is where ergonomics matter. Do not leak internal evaluator details into every caller.

### Phase 5: Adapter Readiness

Shape the crate so `wos-temporal` can stay thin.

Deliverables:

- no engine-specific code in `wos-runtime`
- clear worker-loop entry points
- documented transaction boundary and retry expectations
- reference notes for how a Temporal adapter maps workflow signals, activities, and queries onto the runtime API

Success condition:

- a future `wos-temporal` crate can mostly wire storage, queue, timers, and activities around `wos-runtime` rather than re-implementing behavior

---

## Testing Strategy

### Unit tests in `wos-runtime`

- instance creation
- queue drain semantics
- timer wake-up routing
- provenance cursor advancement
- idempotent replay for duplicate submission tokens
- draft persistence and dismissal behavior
- task completion without response mapping
- task completion with response mapping

### Integration tests in `wos-runtime`

- submission creates case and first event
- human review task completes and advances lifecycle
- validation failure triggers `failureEvent`
- abandonment path records `taskFailed` or `taskSkipped` correctly
- restart scenario resumes from persisted queue and instance state

### Reuse of existing WOS verification

- reuse `wos-conformance` fixtures where possible through the runtime entry points
- move conformance stubs toward the shared in-memory runtime instead of keeping two parallel stub stacks

Do not fork behavioral truth between `wos-conformance` and `wos-runtime`.

---

## Open Decisions

### 1. Where does Formspec validation execute?

Options:

- call an external Formspec service from the runtime
- embed a Rust-native Formspec validator when that exists
- bridge to existing Formspec tooling in-process where feasible

Recommendation:

Treat validation as an injected seam now. Do not block `wos-runtime` on a Rust-native Formspec engine.

### 2. Where does Mapping execution live?

The spec now depends on `responseMappingRef`, but WOS should not absorb Mapping DSL semantics.

Recommendation:

Add a runtime-local `MappingExecutor` seam. Keep Mapping behavior delegated to Formspec infrastructure.

### 3. What is the atomic transaction boundary?

At minimum the runtime must coordinate:

- instance save
- provenance append
- idempotency replay record
- task state update when applicable
- follow-up event enqueue

Recommendation:

Model one runtime step as a single commit unit and make that explicit in the runtime API before adding a database-backed implementation.

### 4. Does `activeTasks` remain canonical?

Recommendation:

Yes. `CaseInstance.activeTasks` remains the canonical workflow state. Add read-model projections only for query speed and UX.

### 5. Should the first adapter be Temporal?

Recommendation:

Yes, but after `wos-runtime` exists. The Temporal reference architecture is already the best fit for durable execution, timers, and replay. The generic runtime crate should come first so the Temporal crate stays thin.

---

## Risks

### Risk 1: Rebuilding infrastructure that Temporal already solves

If `wos-runtime` starts owning distributed scheduling, durable timer services, or workflow history, it has crossed the boundary and is rebuilding an engine.

Mitigation:

Keep `wos-runtime` generic and host-driven. Let engine adapters own infrastructure.

### Risk 2: Letting coprocessor logic split across TypeScript and Rust arbitrarily

If some S15 rules live in the SaaS API and others in `wos-runtime`, the semantics will drift.

Mitigation:

Make `wos-runtime` the home of WOS-side S15 orchestration. The host may render and transport, but it should not own WOS completion semantics.

### Risk 3: Duplicating stub and test infrastructure

`wos-conformance` already has its own in-memory stubs.

Mitigation:

Unify on the `wos-runtime` in-memory reference runtime as soon as the crate is stable enough.

### Risk 4: Forcing Mapping or Formspec semantics into `wos-core`

That would violate the clean boundary already established in ADR-0057 and the current WOS/Formspec split.

Mitigation:

Keep WOS orchestration in `wos-runtime`; keep Formspec semantics behind injected adapters.

---

## Success Criteria

- `wos-runtime` exists as a new crate in `work-spec/Cargo.toml`
- `wos-core` trait parity gaps for Runtime S12 are closed
- one in-memory runtime can create, persist, reload, and advance a case instance
- Runtime S15 task submission is implemented in one place, not spread across callers
- `wos-conformance` can reuse runtime-owned helpers instead of maintaining separate runtime semantics
- a future `wos-temporal` crate can be mostly storage and workflow wiring, not a second implementation of WOS behavior

---

## Recommended Build Order

1. Close `wos-core` trait parity gaps.
2. Add `wos-runtime` with in-memory services and event drain.
3. Land S15 coprocessor operations in `wos-runtime`.
4. Move shared runtime stubs out of `wos-conformance`.
5. Start `wos-temporal` as the first real durable adapter.

That order preserves the clean architecture already won in `wos-core` instead of smearing runtime semantics back across the repo.
