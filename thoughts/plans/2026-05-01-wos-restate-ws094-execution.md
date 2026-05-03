# WS-094 execution plan — Restate durable `wos-server-runtime-restate`

**Date:** 2026-05-01  
**Authoring chain:** [ADR 0084](../adr/0084-wos-restate-durable-runtime-adapter.md) → [spec `2026-05-01-wos-restate-durable-runtime-adapter-spec.md`](../specs/2026-05-01-wos-restate-durable-runtime-adapter-spec.md) → this plan.

Phases **0–2** below keep **ADR §** / **spec req** on each line. **Phases 3–4** checklist lives here; cross-links to parity/vocab work stay in [`2026-05-01-wos-runtime-parity-and-vocab-closure.md`](./2026-05-01-wos-runtime-parity-and-vocab-closure.md). Normative obligations for 3–4 stay in ADR 0084 and the working spec (R-3–R-6.2).

---

## Version matrix (pin before production)

| Component | Pin policy |
|-----------|------------|
| `restate-sdk` (crates.io) | Match Restate Server LTS used in CI; bump in lockstep with server image tag. **Current pin: 0.8.0** (MSRV 1.85 per crates.io — compatible with workspace rustc 1.89). Upgrade to **0.10** when the workspace adopts **rustc 1.90+** ([0.10 docs.rs](https://docs.rs/restate-sdk/0.10.0/restate_sdk/)). |
| Restate Server | CI `docker run` image **`docker.restate.dev/restatedev/restate:1.6.2`** (Admin API 1.6.x); tag recorded in [`wos-server-runtime-restate/README.md`](../../crates/wos-server-runtime-restate/README.md) and overridable via `WOS_RESTATE_SERVER_IMAGE`. |

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

## Phase 3 — Durable drain in VO + in-memory adapter (**ADR D3–D4**; **spec R-3.2–R-4.2**)

- [x] `WosRuntime::create_instance` / `drain_once` on `signature-runtime` + sequential signature profile inside `WosInstance` exclusive handlers; split K/V (`STATE_INSTANCE`, `STATE_PROVENANCE_V1`, `STATE_AUX_V1`, legacy queue merge). **In-memory** [`RestateRuntimeAdapter::new`](../../crates/wos-server-runtime-restate/src/lib.rs) matches the same path.
- [x] Evidence on [`wos-server/TODO.md`](../../crates/wos-server/TODO.md) WS-094 row (Phase 3 bullet).

## Phase 4 — CI replay + conformance slice (**spec R-6.1–R-6.2**; **VISION** §IV)

**Landed (2026-05-01):**

- [x] **R-6.1 — CI replay:** Root [`.github/workflows/ci.yml`](../../../.github/workflows/ci.yml) job **`wos-restate-ingress-smoke`**; [`wos-spec/Makefile`](../../Makefile) target **`restate-ingress-smoke`**; [`scripts/restate_ingress_smoke.sh`](../../scripts/restate_ingress_smoke.sh) (Docker **`docker.restate.dev/restatedev/restate:1.6.2`**, `POST /deployments` → `http://host.docker.internal:9080`, worker binary **`wos-restate-worker`**); ignored test **`ingress_create_load_probe_smoke`** with `WOS_RESTATE_IT_URL` (default `http://127.0.0.1:8080` in script).
- [x] **Ingress wire fix:** No-input handlers (`loadInstance`, `drainOnce`) use **empty POST** bodies (Restate 1.6 rejects spurious `application/json` on those handlers) — [`ingress_http.rs`](../../crates/wos-server-runtime-restate/src/ingress_http.rs).
- [x] **R-6.2 — selected conformance:** [`crates/wos-conformance/tests/r6_restate_conformance_slice.rs`](../../crates/wos-conformance/tests/r6_restate_conformance_slice.rs) — SIG-013 Tier-3 negative + in-memory parity (reference `restate_signature_fixture_runtime` vs `RestateRuntimeAdapter::new` for create + start + drain).
- [x] **Trackers:** WS-094 row + PLN-0333 evidence strings updated (row stays **partial** until three-way + seam closure).

**PLN-0333 acceptance checklist (A–D):** [2026-05-01-pln0333-ws094-acceptance-checklist.md](./2026-05-01-pln0333-ws094-acceptance-checklist.md) — ratify items there before flipping WS-094 / PLN-0333 to done.

**Acceptance status (2026-05-01):**

- [x] **A.1/A.2** — local oracle verified (164/164 tests, 4 reference modules authoritative).
- [x] **B.0/B.1** — baseline smoke + multi-step drain ingress against Restate cluster (`ingress_drain_lifecycle_smoke`).
- [x] **C.0/C.1** — in-memory parity slice + full `DrainOnceResult` field-by-field + `CaseInstance` parity.
- [ ] **D.1** — terminal failures proven (8 tests: 4 memory + 3 ingress + 1 drain_parse). Retryable/stall blocked pending PLN-0039 (`AppendFailure`) + `RuntimeError → HandlerError` classification.

**Tracker boundary decision (2026-05-01):** WS-094 lifecycle-parity scope (`create_instance` / `load_instance` / `enqueue_event` / `drain_once` / `drain_until_idle` + ingress + conformance + terminal failures) is **ratified for PLN-0333**. Remaining work splits into successor rows:

| Row | Scope | Gate |
|-----|-------|------|
| WS-101 | Task APIs on adapter (`persist_task_draft` / `submit_task_response` / `dismiss_task`) | ADR 0084 D5, durable VO task handlers |
| WS-102 | Provenance read on adapter (`load_provenance_window`) | VO provenance-log pagination |
| WS-103 | `migrate_instance` on adapter | ADR 0083 D5 + WS-042 + VO migration design |
| WS-104 | `WOS_RUNTIME=restate` Axum composition root | WS-101 + WS-102 + gate lift in `wos-server/src/lib.rs` |
| WS-105 | Retryable vs terminal classification + stall recovery | PLN-0039 (`AppendFailure`) + `RuntimeError → HandlerError` mapping |

No silent deferral inside WS-094. Once successor rows are filed in [`crates/wos-server/TODO.md`](../../crates/wos-server/TODO.md), WS-094 flips to done for its lifecycle-parity scope.

- [ ] **Tracker update:** file WS-100..WS-104 in TODO.md, flip WS-094 to done, update PLN-0333 in PLANNING.md.
- [ ] **Optional:** Testcontainers-based job if we outgrow the shell + Docker script (same pins).

---

## File touch list (expected)

| Path | Phases |
|------|--------|
| [`crates/wos-server-runtime-restate/Cargo.toml`](../../crates/wos-server-runtime-restate/Cargo.toml) | 0–1, 4 (`wos-restate-worker` bin + `tokio`) |
| [`crates/wos-server-runtime-restate/src/lib.rs`](../../crates/wos-server-runtime-restate/src/lib.rs) | 0–4 |
| [`crates/wos-server-runtime-restate/src/ingress_http.rs`](../../crates/wos-server-runtime-restate/src/ingress_http.rs) | 2, 4 (empty-body ingress) |
| [`crates/wos-server-runtime-restate/src/bin/wos-restate-worker.rs`](../../crates/wos-server-runtime-restate/src/bin/wos-restate-worker.rs) | 4 |
| `src/restate_virtual.rs` (and siblings) | 0–3 landed; seam/migrate extensions still open |
| [`scripts/restate_ingress_smoke.sh`](../../scripts/restate_ingress_smoke.sh) | 4 |
| [`Makefile`](../../Makefile) | 4 (`restate-ingress-smoke`) |
| [`crates/wos-conformance/tests/r6_restate_conformance_slice.rs`](../../crates/wos-conformance/tests/r6_restate_conformance_slice.rs) | 4 |
| [`crates/wos-server/TODO.md`](../../crates/wos-server/TODO.md) WS-094 row | ongoing |
| [`.github/workflows/ci.yml`](../../../.github/workflows/ci.yml) | 4 (`wos-restate-ingress-smoke`) |

---

## Done when

WS-094 checkbox in `wos-server/TODO.md` flips to **done** only after Phase 4 criteria and VISION three-way agreement are met; until then row stays **partial** with links to ADR 0084, this plan, and the working spec.
