# `wos-server-runtime-restate`

Restate-backed [`RuntimeOps`](../../wos-server-ports/src/runtime.rs) adapter (WS-094 / ADR 0084).

## Cargo.lock / transitive dependencies

`restate-sdk` pulls **pre-release** versions of some crypto stack crates (for example `block-buffer` via `restate-sdk-shared-core` → `sha2` / `digest`). That is expected for the current SDK line; verify with `cargo tree -i block-buffer` before attempting to “clean” the lockfile.

## CI and integration tests

- **Default:** `cargo test -p wos-server-runtime-restate --lib` exercises the in-memory backend with a real [`WosRuntime`](../../wos-runtime/src/runtime.rs) `create_instance` / `enqueue_event` / `drain_once` path (`signature-runtime.json` + sequential signature profile).
- **Against a running Restate cluster:** set `WOS_RESTATE_IT_URL` to the ingress base and run the ignored `ingress_create_load_probe_smoke` test (Phase 4 manual / optional Testcontainers job).
