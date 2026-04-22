# WOS-T3 Durable Runtime Spike

**Status:** Complete  
**Date:** 2026-04-21  
**Task:** WOS-T3 — `DurableRuntime` trait extraction + Temporal/Restate spike + tenant-scope notes

## Decision

Use **Restate as the first production durable backend** for WOS runtime adapters.

Keep Temporal as a later engine binding, not the first production Rust backend. The blocker is not Temporal's execution model; it fits WOS Kernel S9 well. The blocker is the current Rust SDK maturity: the official `temporalio-sdk` crate describes itself as alpha-stage, with activity-only workers as the most stable path and workflow APIs still very unstable.

Restate is the better first backend because its Rust SDK exposes the primitives WOS needs directly: durable handlers, workflow `run` handlers, virtual-object style keyed isolation, K/V state, durable timers, journaled results, and durable promises for external events. Those line up with WOS's command surface without forcing WOS into a code-first workflow model.

## Completion Criteria Reviewed

WOS-T3 is complete when all of the following are true:

1. `wos-runtime` has a backend-neutral durable command seam.
2. The reference in-memory runtime implements that seam.
3. `runtime.rs` no longer hides adapter-heavy behavior in one monolithic module.
4. Temporal has been spiked against the seam with an explicit keep/reject/defer conclusion.
5. Restate has been spiked against the seam with an explicit keep/reject/defer conclusion.
6. The multi-tenant execution contract is stated for the selected backend.
7. The active TODO can mark WOS-T3 `-COMPLETE-` without hand-waving remaining adapter questions.

## Runtime Boundary State

The public seam is `wos_runtime::DurableRuntime`.

The reference in-memory adapter is `WosRuntime`. Its `DurableRuntime` impl forwards to inherent methods, so conformance tests can exercise the backend-neutral trait without committing to a production backend.

The runtime module split now isolates adapter-relevant behavior:

| Module | Responsibility |
|---|---|
| `runtime.rs` | Public runtime types, dependency traits, `WosRuntime` construction/configuration |
| `runtime/durable_impl.rs` | Mechanical `DurableRuntime for WosRuntime` forwarding impl |
| `runtime/instance.rs` | Instance create/load/enqueue and custody append-window commands |
| `runtime/drain.rs` | Event drain orchestration |
| `runtime/tasks.rs` | Task draft/dismiss/submit commands |
| `runtime/actions.rs` | Observed action realization and binding dispatch |
| `runtime/timers.rs` | Timer materialization and business-calendar annotations |
| `runtime/provenance.rs` | Provenance stamping, population, compensation helpers |
| `runtime/support.rs` | Shared timestamp, task-id, case-state, and semver helpers |

This is enough separation for production adapters to map each durable command to a backend primitive without first unpicking a single 4k-line file.

## Temporal Spike

**Finding:** Defer as first production backend; keep as future engine binding.

Official evidence:

- `temporalio-sdk` 0.2.0 says it is an "alpha-stage Temporal Rust SDK."
- It states that defining activities and running activity-only workers is currently the most stable code.
- It also states that workflow definitions and workflow workers exist, but the API is still very unstable.
- `temporalio-sdk-core` says it is a basis for creating SDKs and that its APIs are not stable and may break at any time.

Fit against WOS:

| WOS S9 guarantee | Temporal fit |
|---|---|
| G1 crash recovery | Strong conceptual fit |
| G2 persistent state | Strong conceptual fit through event history |
| G3 deterministic replay | Strong conceptual fit, but imposes replay discipline on workflow code |
| G4 durable timers | Strong conceptual fit |
| G5 external signal delivery | Strong conceptual fit |

Adapter shape if revisited:

- One WOS case instance maps to one Temporal workflow execution.
- `enqueue_event` maps to a Temporal signal.
- `load_instance` and provenance windows map to workflow queries plus an external provenance store.
- `invokeService`, task presentation, contract validation, and custody append should be Temporal activities, not inline workflow code.
- Tenant isolation should use namespace-per-tenant only for tenants requiring hard operational isolation; otherwise use workflow-id prefixes and search attributes.

Reason for deferral:

Temporal is semantically strong but Rust-workflow API instability makes it a poor first production Rust backend. WOS can still target Temporal later once the Rust workflow API is stable enough for a conformance-backed adapter.

## Restate Spike

**Finding:** Select as first production backend.

Official evidence:

- `restate-sdk` 0.9.0 describes Restate as a system for building resilient applications with a Rust SDK.
- The SDK lists durable RPC/messaging, journaled results, K/V state, durable timers, awakeables, serialization, and serving as Rust SDK capabilities.
- Its SDK overview says handlers can be services, virtual objects, or workflows.
- Its workflow macro docs state that a workflow `run` handler executes exactly once per workflow instance and runs durable steps/activities.
- Workflow docs also state that workflow K/V state is isolated to the workflow execution and can only be mutated by the `run` handler.
- Shared workflow handlers support querying state and signaling through durable promises.

Fit against WOS:

| WOS S9 guarantee | Restate fit |
|---|---|
| G1 crash recovery | Restate durable handlers and journaled execution fit directly |
| G2 persistent state | Workflow/virtual-object K/V state fits directly |
| G3 deterministic replay | Journaled results avoid re-executing completed nondeterministic steps |
| G4 durable timers | SDK exposes durable scheduling/timers |
| G5 external signal delivery | Durable promises/shared handlers fit external event delivery |

Adapter shape:

- One WOS case instance maps to one Restate workflow object key.
- `create_instance` starts the workflow or initializes a virtual-object-backed case record.
- `enqueue_event` resolves a durable promise or invokes a shared handler that appends to the per-instance event queue.
- `drain_once` and `drain_until_idle` execute inside the workflow `run` handler or a serialized virtual-object command path.
- `load_instance`, provenance-window reads, and custody-window reads are shared read handlers over per-instance state/logs.
- External service calls, task presentation, contract validation, and custody append calls must be journaled as durable steps before WOS advances state.

Production choice:

Restate should be the first production adapter because it gives WOS durable execution without requiring WOS authors to write deterministic workflow code. WOS remains the governance layer; Restate supplies the durable runtime substrate.

## Tenant Contract

The selected backend contract is logical tenant isolation first, physical isolation where required.

### Required identifiers

Every production adapter MUST carry:

- `tenant_id`: opaque deployment tenant, agency, or customer identifier.
- `instance_id`: WOS instance identifier.
- `case_ref`: custody/provenance case identifier.
- `workflow_ref`: governing workflow/kernel identifier.

### Key rules

1. Backend object keys MUST be tenant-qualified.
   - Restate key shape: `{tenant_id}/{instance_id}`.
   - Temporal fallback shape: workflow id `{tenant_id}/{instance_id}` unless using namespace-per-tenant.
2. `instance_id` SHOULD be globally unique. If it is not, the adapter MUST qualify it with `tenant_id` before handing it to the backend.
3. `case_ref` MUST be globally unique across tenants or tenant-qualified before custody append metadata is generated.
4. Provenance cursors are scoped to one tenant-qualified instance.
5. A custody append window MUST NOT combine records across tenants, workflows, or instances.
6. `DurableRuntime::load_custody_append_window` remains per-instance. Multi-instance export is an adapter/orchestrator concern, not a trait method.
7. Tenant-level operational isolation is an adapter deployment choice:
   - shared Restate deployment with tenant-qualified object keys is the default;
   - dedicated Restate deployment/cluster is required for tenants needing hard infrastructure isolation;
   - Temporal namespace-per-tenant is reserved for a future Temporal adapter or strict isolation deployment.

### Provenance log scoping

Per-tenant provenance scoping is by tenant-qualified instance plus append position:

```text
tenant_id / instance_id / provenance_position
```

ADR-0061 custody idempotency remains:

```text
(caseRef, eventType, recordId)
```

The adapter must ensure `caseRef` and `recordId` are tenant-safe before forwarding custody append inputs. The reference runtime currently derives persisted provenance record IDs as:

```text
{caseRef}#provenance-{position}
```

That shape is acceptable only when `caseRef` is already tenant-safe.

## Production Adapter Backlog After WOS-T3

These are follow-on implementation tasks, not WOS-T3 blockers:

1. Create a `wos-restate` adapter crate.
2. Add Restate service/workflow definitions over the existing `DurableRuntime` command surface.
3. Add conformance tests that run the same trait-level scenario against `WosRuntime` and `wos-restate`.
4. Add a deployment note for shared vs. dedicated tenant isolation.
5. Revisit Temporal when the official Rust workflow API is no longer described as unstable.

## Sources

- Temporal Rust SDK docs: <https://docs.rs/temporalio-sdk/latest/temporalio_sdk/>
- Temporal SDK Core docs: <https://docs.rs/temporalio-sdk-core/latest/temporalio_sdk_core/>
- Restate Rust SDK docs: <https://docs.rs/restate-sdk/latest/restate_sdk/>
- Restate workflow macro docs: <https://docs.rs/restate-sdk/latest/restate_sdk/attr.workflow.html>
