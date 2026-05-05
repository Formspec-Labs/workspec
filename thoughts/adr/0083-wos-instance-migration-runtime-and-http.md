# ADR 0083 — WOS instance migration (runtime, provenance, HTTP)

**Status:** Accepted — D1/D2/D3 (provenance kind) ratified 2026-05-01; runtime + HTTP primitive shipped same date; **D5 minimal HTTP idempotency** (successful `Idempotency-Key` replay) + **version-indexed bundle resolution** landed 2026-05-01; D4/D6/D7 remain open where noted below.

**Date:** 2026-05-01

## Context

Kernel §11.2 defines instance lifecycle against workflow definition versions, including author-time `execution.instanceVersioning` literals `pinned` and `migrateable` (see kernel spec §9.6). Moving a live case instance to a new definition version requires:

- State validation against the target definition (no silent retargeting of incompatible topology).
- A migration map or equivalent provenance so auditors can explain what changed and why.
- Atomicity boundaries where processors commit case state and provenance together.
- HTTP surface for operator-initiated migration — tracked as **WS-042** in [`../../crates/wos-server/TODO.md`](../../crates/wos-server/TODO.md) (search `WS-042`).

`WosRuntime::migrate` is intentionally **not** bundled with signature conformance (SIG-013) or schema inner-block hardening; those programs close independently.

## Decisions

### D1 — Migrate API shape (settled 2026-05-01)

`WosRuntime::migrate(&mut self, instance_id: &str, target_definition_version: &str, migration_map: MigrationMap, operator_actor_id: Option<&str>) -> Result<MigrationOutcome, RuntimeError>`

The optional `operator_actor_id` is the authenticated operator identity (HTTP: Supervisor JWT user id) carried into `instanceMigrated` provenance as `actor_id` when present; `None` is allowed for non-HTTP callers.

Lands as a method on the existing `WosRuntime` (NOT a separate trait/crate). `MigrationMap` is the JSON-shape from kernel §11.2 step 2 (`fieldRenames`, `fieldRemovals`, `fieldDefaults`, `fieldCoercions`). The governing `definitionUrl` is taken from the loaded instance (same URL migration only). `RuntimeOps::migrate_instance` on `wos-server-ports` mirrors the same `operator_actor_id` parameter.

**Why:** `WosRuntime` already owns the mutable instance store and the kernel resolver; adding `migrate` as a method is the minimum surface that satisfies the atomicity and audit invariants without a new abstraction layer that has only one consumer.

### D2 — HTTP route home (settled 2026-05-01)

`POST /api/instances/:id/migrate` lives in `wos-server` per WS-042 (currently in `crates/wos-server/`). Once `thoughts/plans/2026-05-01-platform-repository-architecture.md` §3.2 (`wos-server*` → `workspec-server`) lands, the route relocates with the crate; the API shape is stable.

**Why:** Consistent with all other instance-lifecycle routes (`/events`, `/drain`, `/holds`) which live in `wos-server/src/http/instances.rs`. Route stability across repository rename is the established convention.

### D3 — Provenance kind name (settled 2026-05-01)

`instanceMigrated` (camelCase). This is the variant on `ProvenanceKind` and the `recordKind` enum value in `wos-workflow.schema.json#/$defs/FactsTierRecord/properties/recordKind`.

**Why:** Follows the `{verb}{Object}` camelCase convention of adjacent kinds (`stateTransitioned`, `taskCompleted`, `milestoneReached`). Past-tense verb signals a completed, irreversible operation consistent with audit semantics.

### D3b — `TransitionEventError.code` vocabulary (trigger-gated; distinct from D3)

The merged workflow schema keeps `TransitionEventError.code` as a **pattern-shaped string** with examples (`schemas/wos-workflow.schema.json` → `$defs/TransitionEventError`). A **finite reserved-code table** plus optional JSON Schema `enum` tightening for `code` remains **trigger-gated** until the WG publishes the closed set (so processors, linters, and conformance harnesses agree on one authoritative list without churn).

**Why:** Error-kind dispatch is security- and ops-sensitive; prematurely freezing codes in schema would fork implementations before normative text exists.

## Implementation snapshot (2026-05-01)

Landed in-tree:

- `wos-runtime`: `MigrationMap`, `MigrationOutcome`, `RuntimeError::MigrationRejected`, `WosRuntime::migrate` (including same-version **no-op**: no store write, no new provenance), `wos_core::eval::validate_migration_configuration` (Kernel S11.2 step 1), `ProvenanceRecord::instance_migrated`, unit tests for happy-path version bump + `stateNotFound` rejection. `fieldCoercions` with `"number"` rejects non-finite floats. `RuntimeError::{KernelWorkflowNotFound, KernelDefinitionVersionMismatch, FeatureDisabled}` support correct HTTP status mapping at the server boundary.
- `wos-server`: `POST /api/instances/:id/migrate` (Supervisor), `RuntimeOps::migrate_instance` on local + Restate adapters. Restate returns the same explicit **unsupported** error pattern as other unimplemented ops on that adapter (`migrate_instance` is test-covered; full Restate durability remains **WS-094**). Request body: empty or whitespace-only `target_definition_version` rejected with **400**. Stub path when `runtime-local` is off: `FeatureDisabled` → **400** (not generic **503**).
- `wos-server-runtime-local`: `RuntimeKernelResolver` wraps bundle resolution so kernel-not-found / definition-version mismatch surface as typed `RuntimeError` variants (**404** / **400**) instead of a blanket resolver **503**.
- Schema: `instanceMigrated` added to `FactsTierRecord.recordKind` enum; `TransitionEventError.code` carries a `$comment` documenting the trigger gate for the future finite set.
- **2026-05-01 — bundle + HTTP completion:** operational `kernels` rows are keyed by `(url, version)`; `BundleService` / `BundleResolverPort::resolve_kernel_bundle` resolve by `(workflow_url, definition_version)`; integration tests cover HTTP `1.0.0 → 1.1.0` and **`Idempotency-Key`** replay of a successful migrate (`runtime_lifecycle.rs`).

**Resolved vs earlier snapshot:** reference-server `BundleService` is no longer URL-only; cross-version HTTP migrate is proven in-tree. Same-version migrate remains a runtime no-op (`migrate_instance_via_http_same_version_is_idempotent`).

## Open decisions

- **D4: Preconditions for `migrate`** — requires posture-floor table from Posture Declaration registry; gated on PLN-0384. **Related (vendor assurance):** Signature Profile `x-*` assurance tokens remain **fail-open** in `identity_binding_meets_policy` until Posture Declaration + spec §2.13 land — tracked in [`../../T4-TODO.md`](../../T4-TODO.md) § *Vendor `x-*` assurance floor enforcement*.
- **D5: Idempotency model** — **Minimal reference-server slice closed 2026-05-01:** HTTP `Idempotency-Key` replays the cached successful [`MigrationOutcome`](../../crates/wos-runtime/src/runtime.rs) for the same `(instance_id, target_definition_version, key)` triple (in-memory cache on `AppState`). **Still open (full D5 posture):** dedupe across process restarts, failure semantics for conflicting bodies with the same key, and alignment with D4 preconditions when the posture floor lands.
- **D6: Error model** — `MigrationOutcome` extensions and additional `RuntimeError` variants for topology mismatch beyond `stateNotFound`, missing migration map fields, and rollback posture are gated on D4.
- **D7: Provenance ordering vs facts-tier** — whether `instanceMigrated` precedes or follows the target-version `stateEntered` record is gated on D4 and D5.

## Consequences

- Implementation may reference this ADR for the runtime primitive; HTTP cross-version prove-out and reference-server idempotency cache are **landed** (WS-042 ✓); richer D5/D6/D7 items remain advisory until closed.
- D1/D2/D3/D3b are stable for planning; D4–D7 remain advisory until closed.

## References

- WOS Kernel `spec.md` §9.6, §11.1–§11.2
- [`../../crates/wos-server/TODO.md`](../../crates/wos-server/TODO.md) — **WS-042**
- [`../../T4-TODO.md`](../../T4-TODO.md) — vendor `x-*` assurance / posture follow-on
- [`thoughts/plans/2026-05-01-platform-repository-architecture.md`](../plans/2026-05-01-platform-repository-architecture.md) — §3.2 `wos-server*` → `workspec-server` rename
