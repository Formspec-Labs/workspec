# WOS Restate durable runtime adapter — working specification

**Status:** Draft (thoughts lane; ratified by [ADR 0084](../adr/0084-wos-restate-durable-runtime-adapter.md)).  
**Date:** 2026-05-01  
**Normative companions (reference only):** [`specs/companions/runtime.md`](../../specs/companions/runtime.md), kernel §9 / §11 as cited in ADR 0083 and WS-094.

This document states **processor-observable obligations** for the `wos-server-runtime-restate` adapter. Each requirement cites an **ADR 0084 decision id** (`D1`–`D5`).

---

## 1. Identity and keying (D1)

**R-1.1 [D1]** Every Restate Virtual Object key used for WOS instance execution MUST equal the WOS `instance_id` string exposed on `CaseInstance.instance_id` and accepted by `RuntimeOps::load_instance`.

**R-1.2 [D1]** At most one exclusive (`ObjectContext`) handler for a given `instance_id` MUST run at a time; concurrent reads that do not mutate durable WOS state MUST use `#[shared]` handlers and `SharedObjectContext` as defined by `restate-sdk`.

**R-1.3 [D1]** Handlers that mutate WOS operational projection or enqueue drain work MUST NOT be marked `#[shared]`.

---

## 2. Topology and HTTP surfaces (D2)

**R-2.1 [D2]** The Restate worker process MUST expose `restate_sdk::HttpServer` on a bind address that is **distinct** from the Axum `wos-server` listener unless a future ADR specifies a single-listener reverse-proxy composition.

**R-2.2 [D2]** `wos-server` configuration MUST record the Restate **ingress base URL** (or equivalent client target) used by `RuntimeOps` forwarding paths when `runtime-restate` is selected.

---

## 3. Store boundaries (D3)

**R-3.1 [D3]** Restate journal entries MUST be treated as **execution-replay** state; they MUST NOT be advertised as Trellis-canonical governance artifacts.

**R-3.2 [D3]** WOS provenance records emitted during drain MUST still flow through the existing WOS provenance pipeline (`wos-runtime` + `AuditSink` / storage as today) unchanged in semantic meaning from `runtime-local`.

**R-3.3 [D3]** SQL `InstanceRow.instance_json` MAY lag authoritative execution state during migration phases; any such lag MUST be documented in the execution plan and bounded by explicit recovery or resync steps.

---

## 4. Runtime bridge (D4)

**R-4.1 [D4]** Kernel resolution, transition evaluation, binding dispatch, and provenance stamping for a drained event MUST reuse `wos-runtime` code paths (via `WosRuntime` or `DurableRuntime` impls); adapters MUST NOT reimplement kernel transition tables in handler strings.

**R-4.2 [D4]** External I/O (integration bindings, remote validators, signer calls) invoked from a durable handler MUST use `restate-sdk` journaled side-effect APIs so replay does not duplicate side effects.

**R-4.3 [D4]** Wall-clock timestamps and random identifiers needed for WOS records MUST come from Restate/SDK replay-stable sources inside journaled code paths, not from non-deterministic OS calls that bypass journaling.

---

## 5. Errors and retries (D5, ADR 0070 alignment)

**R-5.1 [D5]** Errors that represent invalid operator input, policy denial, or contract violation MUST surface as **terminal** errors in the Restate sense so the runtime does not retry indefinitely.

**R-5.2 [D5]** Errors that represent transient dependency outage (remote HTTP 503, DB timeout) SHOULD remain retryable unless ADR 0070 or product policy marks them terminal for a specific integration.

**R-5.3 [D5]** `RuntimeOps` errors returned to Axum MUST preserve existing `wos-server` HTTP mapping conventions for the same logical failure classes as `runtime-local` where applicable.

---

## 6. Conformance and testing

**R-6.1** After handler logic ships, CI MUST run at least one **strict replay** (or SDK-documented equivalent) test path for every merged change that alters handler ordering, journaling layout, or branch structure in Restate handlers.

**R-6.2** Conformance targets MUST remain [`wos-server-runtime-local`](../../crates/wos-server-runtime-local) until VISION three-way agreement explicitly adds Restate-backed fixtures.

**Implementation snapshot (2026-05-01, non-normative):** Root CI **`wos-restate-ingress-smoke`** + [`scripts/restate_ingress_smoke.sh`](../../scripts/restate_ingress_smoke.sh) satisfy **R-6.1** for the current ingress + VO wiring. [`wos-conformance/tests/r6_restate_conformance_slice.rs`](../../crates/wos-conformance/tests/r6_restate_conformance_slice.rs) adds a **supplementary** parity + terminal-failure slice (reference `WosRuntime` vs `RestateRuntimeAdapter::new`, plus ingress smoke for terminal paths). It does **not** replace the **R-6.2** canonical target above. **PLN-0333 / WS-094 lifecycle acceptance** ([checklist](../plans/2026-05-01-pln0333-ws094-acceptance-checklist.md)) closed on **A/B/C + D.1a**; **D.1b** retryable proof and oracle extension remain **WS-105** / explicit VISION agreement, not implied by this snapshot.

---

## Traceability matrix

| Req | ADR § |
|-----|--------|
| R-1.* | D1 |
| R-2.* | D2 |
| R-3.* | D3 |
| R-4.* | D4 |
| R-5.* | D5 |
