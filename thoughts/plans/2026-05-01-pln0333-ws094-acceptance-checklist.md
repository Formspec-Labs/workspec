# PLN-0333 / WS-094 acceptance checklist (lifecycle slice — **ratified 2026-05-01**)

**Purpose:** Record observable, CI-checkable criteria used to close **PLN-0333** and **`WS-094 [✓]`** in [`wos-server/TODO.md`](../../crates/wos-server/TODO.md). Successor scope is **WS-101..WS-105**; keep this doc aligned with [WS-094 execution plan](./2026-05-01-wos-restate-ws094-execution.md) when **D.1b** or successors move.

**Normative note:** [Working spec R-6.2](../specs/2026-05-01-wos-restate-durable-runtime-adapter-spec.md) keeps **`wos-server-runtime-local`** as the canonical conformance oracle until explicit VISION three-way agreement extends it; PLN-0333 closure ratifies **lifecycle** evidence + supplementary Restate fixtures, not a replacement oracle.

**Tracker boundary (2026-05-01, applied):** **Restate lifecycle parity** ratified for **PLN-0333** (`Done` in [`PLANNING.md`](../../../PLANNING.md)). **Task APIs**, **provenance window**, **adapter migration**, **`WOS_RUNTIME=restate` composition**, and **D.1b retryable/stall** are **WS-101..WS-105** — not silent deferrals inside WS-094.

## Ratified decisions

- **Oracle:** keep **`runtime-local`** authoritative until A-D close. Restate is production target, not conformance oracle yet.
- **Ingress:** implement **B.1**; do not defer. The VO already supports `enqueueEvent` + `drainOnce`, so smoke-only ingress is under-evidence.
- **Parity style:** extend **C.0** in `wos-conformance`; do not add `wos-server` local-vs-Restate API parity until `WOS_RUNTIME=restate` stops failing fast.
- **Failure boundary:** **D.1a** is CI-checkable (terminal). **D.1b** must be CI-checkable unless Restate SDK failure injection proves impossible; a manual runbook is a fallback evidence artifact, not the preferred closure path (**WS-105**).

---

## A — Local oracle (`runtime-local`)

- [x] **A.1** Named integration paths in `wos-server` (default **`runtime-local`**) remain green under `cargo nextest run -p wos-server` (or a documented subset). Minimum subset: lifecycle create/load, event submit+drain, task lifecycle, timer poll, migration same-version idempotency.
  - Verified: 164/164 tests pass, 0 skipped (`cargo nextest run -p wos-server`).
- [x] **A.2** Migration / provenance HTTP slices that define "reference" behavior (e.g. [`runtime_lifecycle.rs`](../../crates/wos-server/tests/integration/runtime_lifecycle.rs), [`http_event_submit_drain.rs`](../../crates/wos-server/tests/integration/http_event_submit_drain.rs), [`http_tasks_lifecycle.rs`](../../crates/wos-server/tests/integration/http_tasks_lifecycle.rs), [`timer_poll_e2e.rs`](../../crates/wos-server/tests/integration/timer_poll_e2e.rs)) stay authoritative until Restate parity claims them.
  - All four reference modules green: runtime_lifecycle (3 tests), http_event_submit_drain (2), http_tasks_lifecycle (3), timer_poll_e2e (1).

## B — Restate ingress (same logical scenarios as supported `RuntimeOps`)

- [x] **B.0** Baseline: root CI **`wos-restate-ingress-smoke`** + [`scripts/restate_ingress_smoke.sh`](../../scripts/restate_ingress_smoke.sh) + ignored **`ingress_create_load_probe_smoke`** (create + load + probe).
- [x] **B.1** Extend ingress coverage to **multi-step drain** against a running Restate cluster. Done means: create → enqueue `start` → repeated `drain_once` until idle (or adapter `drain_until_idle`) → load; assert the same observable fields used by **C.1**. **No deferral accepted** unless the VO drain path is removed or Restate cluster orchestration becomes impossible in CI.
  - Implemented: [`ingress_drain_lifecycle_smoke`](../../crates/wos-server-runtime-restate/src/lib.rs) — create → enqueue start → `drain_until_idle` → load; asserts `DrainOnceResult` shape (transitions, tasks, provenance, idle sentinel) + `WorkflowProcess` fields (configuration, active_tasks, pending_events empty). CI-wired via [`restate_ingress_smoke.sh`](../../scripts/restate_ingress_smoke.sh).

## C — Parity: `runtime-restate` vs reference / local

- [x] **C.0** In-memory parity slice: [`r6_restate_conformance_slice.rs`](../../crates/wos-conformance/tests/r6_restate_conformance_slice.rs) (SIG-013 Tier-3 + **`configuration`** + **`active_tasks.len()`** parity after start+drain; **`SeamAccess`** noop signer/renderer smoke).
- [x] **C.1** Extend **C.0** with deterministic adapter parity slices; cross-link the new tests here. Minimum expansion: compare reference `WosRuntime` vs `RestateRuntimeAdapter::new()` for full `DrainOnceResult` shape on start+idle (`processed_event`, transition tuple, created task IDs, emitted events, provenance kinds, guard evaluation count) plus final `WorkflowProcess` fields (`configuration`, `active_tasks` IDs, pending queue empty). Do **not** add `wos-server` public-API local-vs-Restate parity until [`wos-server/src/lib.rs`](../../crates/wos-server/src/lib.rs) no longer gates `RuntimeKind::Restate`.
  - Implemented: [`r6_c1_full_drain_result_shape_parity`](../../crates/wos-conformance/tests/r6_restate_conformance_slice.rs) — full `DrainOnceResult` field-by-field parity (processed_event, transition tuples, created_task_ids sorted, emitted_events sorted, provenance kinds in order, guard evaluation count) + `WorkflowProcess` parity (configuration, active_tasks IDs sorted, pending_events empty).

## D — Retry / stall / recovery (ADR 0070 boundary)

- [x] **D.1a (terminal)** — CI + unit evidence for **terminal** failures at the Restate adapter boundary, aligned to [ADR 0070](../../../thoughts/adr/0070-stack-failure-and-compensation.md). Invalid contract input / duplicate create / malformed event / nonexistent load map to `TerminalError`; Restate does not retry them.
  - **Evidence:** In-memory tests — `duplicate_create_returns_terminal_error`, `malformed_event_returns_terminal_error`, `invalid_definition_url_returns_terminal_error`, `load_nonexistent_returns_terminal_error` (all in [`lib.rs`](../../crates/wos-server-runtime-restate/src/lib.rs)). Ingress tests — `ingress_duplicate_create_is_terminal`, `ingress_malformed_event_is_terminal`, `ingress_load_nonexistent_is_terminal` (CI-wired via [`restate_ingress_smoke.sh`](../../scripts/restate_ingress_smoke.sh)). VO handlers return `TerminalError` for these paths.
- [ ] **D.1b (retryable / stall)** — CI evidence for **retryable** classification and stall recovery at the VO boundary. Requires SDK failure injection or a test-only failing handler; blocked until typed `AppendFailure` / stalled-state work lands. Manual runbook only counts if SDK-level automation is unavailable and the runbook records exact commands, injected failure, observed Restate state, and WOS boundary error.
  - **Retryable/stall blocker:** VO handlers map *all* errors through `TerminalError` today — no path produces Restate-retryable `HandlerError`. Proving retryable classification requires: (1) `RuntimeError → HandlerError` classification mapping in [`restate_virtual.rs`](../../crates/wos-server-runtime-restate/src/restate_virtual.rs) (governance/contract → terminal, store/transient → retryable), (2) `AppendFailure` typed outcomes per ADR 0070 D-4.3 (tracked under **PLN-0039**), (3) failure injection via `FailingStore`-style wrapper or worker restart. **Tracker:** **WS-105** (not WS-094).

---

## `migrate_instance` on Restate (explicit deferral)

**Decision (2026-05-01):** Keep **`migrate_instance`** on [`RestateRuntimeAdapter`](../../crates/wos-server-runtime-restate/src/lib.rs) as **structured unsupported** until [ADR 0083](../adr/0083-wos-instance-migration-runtime-and-http.md) **D5** (idempotency), **WS-042** (version-indexed bundle resolution / HTTP cross-version proof), and VO migration design land. HTTP **`POST …/migrate`** remains **`runtime-local`**-backed per existing server gates.

**Tracker effect (2026-05-01):** Restate **`migrate_instance`** is owned by **WS-103**; WS-094 lifecycle closure does not require adapter migration. HTTP **`POST …/migrate`** stays **`runtime-local`**-backed until WS-103 + ADR 0083 D5 land.

---

## `WOS_RUNTIME=restate` composition root

**Today:** [`wos-server/src/lib.rs`](../../crates/wos-server/src/lib.rs) **`RuntimeKind::Restate`** fails fast with a clear message — **`AppRuntime`** is not yet parameterized with [`RestateRuntimeAdapter`](../../crates/wos-server-runtime-restate/src/lib.rs). Closing **B/C** above does **not** require lifting that gate until product wants Axum-on-Restate; when lifting, implement **`persist_task_draft` / `submit_task_response` / `load_provenance_window` / `dismiss_task`** on the adapter (or keep gate for routes that call unsupported ops).

**Tracker effect (2026-05-01):** Axum-on-Restate composition is **WS-104** (gated WS-101 + WS-102). WS-094 closed without lifting `RuntimeKind::Restate` in [`wos-server/src/lib.rs`](../../crates/wos-server/src/lib.rs).
