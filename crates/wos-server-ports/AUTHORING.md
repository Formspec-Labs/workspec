# wos-server adapter authoring

This guide defines the minimum contract for third-party adapters against `wos-server-ports`.

## 1) Trait surface

- Storage adapters implement `wos_server_ports::Storage`.
- Auth adapters implement `wos_server_ports::AuthProvider`.
- Runtime adapters implement layered traits:
  - `RuntimeOps`
  - `SeamAccess`
  - `TimerCoord`

Adapters should depend on:

- `wos-server-ports`
- `wos-core` only when required by trait signatures
- backend crates required by the adapter itself

Adapters should **not** depend on `wos-server`.

## 2) Behavioral requirements

- **Idempotency**
  - `insert_inbound_cloud_event` must deduplicate by cloud-event id.
  - replay-key and task-submit paths must not create duplicate effects on retry.
- **Atomicity**
  - `update_instance_atomic` must apply instance row update and appended provenance in one transaction boundary.
  - auth epoch + session revocation flows must be transaction-safe.
- **Pagination**
  - `list_instances` must clamp `page_size` to `[1, LIST_INSTANCES_PAGE_SIZE_MAX]`.
  - ordered pagination should be deterministic for stable UI behavior.

## 3) Parity-test expectations

Every adapter crate should carry parity tests against fixture behavior used by `wos-server`:

- instance create/load/update
- provenance append and chain ordering
- agent and delegation CRUD
- auth session validity + revocation semantics
- intake and inbound event idempotency

If behavior intentionally diverges from SQLite reference behavior, document it in the adapter README and add explicit tests.

## 4) Cargo shape

Recommended pattern:

- adapter crate is standalone (`crates/wos-server-<adapter>/`)
- `wos-server` uses `optional = true` dependency + feature gate
- composition root fails fast when config requests an adapter whose feature is disabled

This keeps adapter selection explicit and Cargo-enforced.
