# WS-094 execution plan — Restate durable `wos-server-runtime-restate`

**Date:** 2026-05-01  
**Authoring chain:** [ADR 0084](../adr/0084-wos-restate-durable-runtime-adapter.md) → [spec `2026-05-01-wos-restate-durable-runtime-adapter-spec.md`](../specs/2026-05-01-wos-restate-durable-runtime-adapter-spec.md) → this plan.

Phases **0–2** below keep **ADR §** / **spec req** on each line. **Phases 3–4** are *not* duplicated here: sequencing and actionable todos live in the Cursor plan **continue_wos_parity_and_restate** (parity land first, then `restate-phase3` / `restate-phase4`). Normative obligations for 3–4 stay in ADR 0084 and the working spec (R-3–R-6.2). Parity / vocab / verification / commits / submodule: [`2026-05-01-wos-runtime-parity-and-vocab-closure.md`](./2026-05-01-wos-runtime-parity-and-vocab-closure.md).

---

## Version matrix (pin before production)

| Component | Pin policy |
|-----------|------------|
| `restate-sdk` (crates.io) | Match Restate Server LTS used in CI; bump in lockstep with server image tag. **Current pin: 0.8.0** (MSRV 1.85 per crates.io — compatible with workspace rustc 1.89). Upgrade to **0.10** when the workspace adopts **rustc 1.90+** ([0.10 docs.rs](https://docs.rs/restate-sdk/0.10.0/restate_sdk/)). |
| Restate Server | CI `docker run` (or `testcontainers`) image tag recorded in `wos-spec/crates/wos-server-runtime-restate` README or module doc. |

---

## Phase 0 — Dependency and compile proof (ADR D1, spec R-1.x prep)

- [x] Add `restate-sdk` (**0.8.0** — 0.10+ needs rustc 1.90+) and related deps to [`wos-spec/crates/wos-server-runtime-restate/Cargo.toml`](../../crates/wos-server-runtime-restate/Cargo.toml). **ADR D1**; **spec** preamble.
- [x] Add `src/restate_virtual.rs`: `#[restate_sdk::object]` `WosInstance` + handlers (`probe`, lifecycle). **ADR D1**; **spec R-1.2** groundwork.
- [x] `cargo check -p wos-server-runtime-restate` and `cargo test -p wos-server-runtime-restate --lib` green. **spec R-6.2** (local unit scope).

---

## Phase 1 — `HttpServer` endpoint and discovery (ADR D2, spec R-2.x)

- [x] Export [`restate_virtual::wos_instance_endpoint`](../../crates/wos-server-runtime-restate/src/restate_virtual.rs) (`Endpoint::builder().bind(...)`). **ADR D2**; **spec R-2.1**.
- [x] Document bind / env in [`wos-spec/crates/wos-server-runtime-restate/src/lib.rs`](../../crates/wos-server-runtime-restate/src/lib.rs) (`WOS_RESTATE_INGRESS_URL`, `WOS_RESTATE_IT_URL` for ignored test). **spec R-2.2**.

---

## Phase 2 — Ingress client + `RuntimeOps` forward (ADR D4, spec R-4.1)

- [x] Add Restate **ingress** client (`reqwest` + [`ingress_http::RestateIngressClient`](../../crates/wos-server-runtime-restate/src/ingress_http.rs)); `create_instance` / `load_instance` / `enqueue_event` / `drain_once` on [`RestateRuntimeAdapter`](../../crates/wos-server-runtime-restate/src/lib.rs) delegate to virtual-object handlers when `with_restate_ingress` / `from_env` is used. **ADR D4**; **spec R-4.1**, **R-5.3**.
- [x] Retain in-memory path as default (`RestateRuntimeAdapter::new`). **spec R-6.2**.

---

## Phases 3–4 (execution elsewhere)

**Phase 3** — `RuntimeStore` + `WosRuntime` in exclusive handlers; provenance / storage / `AuditSink` mapping (**ADR D3–D4**; **spec R-3.2–R-4.2**). **Phase 4** — CI strict replay (Testcontainers or SDK harness); conformance / integration vs `runtime-restate`; refresh **WS-094** + **PLN-0333** evidence (**spec R-6.1–R-6.2**; **VISION** §IV). Track and gate in **continue_wos_parity_and_restate** after parity closure (same doc family: [`2026-05-01-wos-runtime-parity-and-vocab-closure.md`](./2026-05-01-wos-runtime-parity-and-vocab-closure.md)).

---

## File touch list (expected)

| Path | Phases |
|------|--------|
| [`crates/wos-server-runtime-restate/Cargo.toml`](../../crates/wos-server-runtime-restate/Cargo.toml) | 0–1 |
| [`crates/wos-server-runtime-restate/src/lib.rs`](../../crates/wos-server-runtime-restate/src/lib.rs) | 0–2 |
| `src/restate_virtual.rs` (and siblings) | 0–2 landed; 3–4 extend per **continue_wos_parity_and_restate** |
| [`crates/wos-server/TODO.md`](../../crates/wos-server/TODO.md) WS-094 row | ongoing |
| [`.github/workflows/ci.yml`](../../../.github/workflows/ci.yml) | Phase 4 job when scheduled in **continue_wos_parity_and_restate** |

---

## Done when

WS-094 checkbox in `wos-server/TODO.md` flips to **done** only after Phase 4 criteria and VISION three-way agreement are met; until then row stays **partial** with links to ADR 0084, this plan, and the working spec.
