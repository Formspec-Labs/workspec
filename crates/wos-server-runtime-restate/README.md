# `wos-server-runtime-restate`

Restate-backed [`RuntimeOps`](../../wos-server-ports/src/runtime.rs) adapter (WS-094 / ADR 0084).

## Known gaps (documented 2026-05-03)

These are **not** oversights from the WS-094 lifecycle / ingress review; they stay open until the listed `WS-*` work lands in [`wos-server/TODO.md`](../wos-server/TODO.md).

| Gap | Notes |
|-----|--------|
| **Seam signer** | [`RestateSeamSigner`](src/lib.rs) is an intentional no-op (`sign` returns empty bytes, `verify` returns true) until a Trellis-bound production signer is wired. Do not treat this as integrity for real deployments. Tracks with **WS-043** and Restate signing follow-on for WS-094. |
| **Server boot** | `wos-server` still refuses `WOS_RUNTIME=restate` at startup; the adapter is used from tests, conformance (`r6_restate_conformance_slice`), and **ignored** ingress smoke, not as the Axum `AppState.runtime` root. **WS-104**. |
| **`migrate_instance`** | Adapter returns unsupported for migration until **WS-103**. |
| **`drain_until_idle`** | Bounded by a large step cap (see implementation); infinite loops surface as `RuntimeAdapterError` instead of hanging forever. |

## Cargo.lock / transitive dependencies

`restate-sdk` pulls **pre-release** versions of some crypto stack crates (for example `block-buffer` via `restate-sdk-shared-core` → `sha2` / `digest`). That is expected for the current SDK line; verify with `cargo tree -i block-buffer` before attempting to “clean” the lockfile.

## Version matrix (pin with Restate Server in CI)

| Component | Pin (2026-05-01) |
|-----------|------------------|
| `restate-sdk` (this crate) | **0.8.0** (MSRV 1.85; workspace rustc 1.89). Bump with server image when upgrading. |
| Restate Server (Docker) | **`docker.restate.dev/restatedev/restate:1.6.2`** — matches Admin API docset `1.6.2`; override with `WOS_RESTATE_SERVER_IMAGE`. |

## Lockstep policy (SDK and server image)

Bump **`restate-sdk`** and the **Docker image** together in one change set. Touch points: this crate’s [`Cargo.toml`](./Cargo.toml), the table above, [`scripts/restate_ingress_smoke.sh`](../../scripts/restate_ingress_smoke.sh) default `WOS_RESTATE_SERVER_IMAGE`, and the parent repo [`.github/workflows/ci.yml`](../../../.github/workflows/ci.yml) job **`wos-restate-ingress-smoke`** (must stay aligned with the script).

## CI and integration tests

- **Default:** `cargo test -p wos-server-runtime-restate --lib` exercises the in-memory backend with a real [`WosRuntime`](../../wos-runtime/src/runtime.rs) `create_instance` / `enqueue_event` / `drain_once` path (`signature-runtime.json` + sequential signature profile).
- **Worker binary:** `cargo build -p wos-server-runtime-restate --bin wos-restate-worker` — serves [`wos_instance_endpoint`](src/restate_virtual.rs) on `WOS_RESTATE_WORKER_ADDR` (default `0.0.0.0:9080`).
- **Phase 4 CI replay (R-6.1):** root GitHub Actions job **`wos-restate-ingress-smoke`** and `make -C wos-spec restate-ingress-smoke` run [`scripts/restate_ingress_smoke.sh`](../../scripts/restate_ingress_smoke.sh): Docker Restate (pinned image), worker on the host, `POST /deployments` to `http://host.docker.internal:9080`, then `cargo test … ingress_create_load_probe_smoke -- --ignored`.
- **Against a running Restate cluster (manual):** set `WOS_RESTATE_IT_URL` to the ingress base (for example `http://127.0.0.1:8080`) after registering the worker, then run the ignored `ingress_create_load_probe_smoke` test.
