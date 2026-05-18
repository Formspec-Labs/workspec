# WOS Public API Case Export Fence

**Status:** Stable — shipped behavior pinned to live struct.
**Schema:** [`api/case-export-fence.schema.json`](../../schemas/api/case-export-fence.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/case-export-fence/v1`
**Rust authority:** [`CaseExportFence` in `workspec-server/crates/wos-server/src/http/cases.rs:146-162`](../../../workspec-server/crates/wos-server/src/http/cases.rs).
**OpenAPI:** [`wos-public-api.openapi.json`](../../api/wos-public-api.openapi.json) (`GET /cases/{case_id}/export-fence`); registry component `CaseExportFence` in [`wos-public-api.registry.openapi.json`](../../api/wos-public-api.registry.openapi.json).
**ADR anchors:** [ADR 0082](../../../thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md) D-4 (URN), D-12 (closed taxonomies). [ADR 0093](../../thoughts/adr/0093-case-is-its-trellis-ledger.md) D-1 (case-is-its-ledger), §5 (route surface). Trellis-side counterpart: [`trellis/specs/trellis-core.md`](../../../trellis/specs/trellis-core.md) §18.3e (`trellis-export-seal-fence-v1`).
**Sibling spec:** [`case-view.md`](./case-view.md) — staff read-side projection over the same case ledger.

## Purpose

`CaseExportFence` is the WOS-side source fence for a case-export attempt. The handler captures the current case-ledger and policy/config high-water marks at fence-acquisition time, before downstream bundle assembly runs. Downstream Trellis publication binds its seal fence (`trellis-export-seal-fence-v1`, Trellis Core §18.3e) to these values; a verifier reading the bundle sees a coherent snapshot of the case's append history and policy-snapshot identity at fence time.

The case IS its Trellis ledger per ADR-0093 D-1. `CaseExportFence` is therefore not an independent aggregate — it is a structured, server-clock-stamped read of the ledger's append vector plus the active policy snapshot, captured under a per-case mutation lease.

## Resource shape (camelCase per `#[serde(rename_all = "camelCase")]`)

| Field | Type | Description |
|---|---|---|
| `caseLedgerId` | `string` | Stable ID of the case ledger this fence captures. |
| `trellisScope` | `string` | Trellis scope identifier (the case ledger's bundle-scope bytes in their string-encoded form). |
| `eventCount` | `integer` | Total event count at fence acquisition. Any consistent bundle reading this fence MUST observe the same count in `010-events.cbor`. |
| `eventHighWater` | object \| null | When non-null: the last event observed at fence acquisition, as a `CaseFenceEvent`. Absent when the case has no events. |
| `directHighWater` | object \| null | When non-null: the last direct-append event observed (subset of all events). Absent when the case has no direct-append rows. |
| `processHighWater` | array of `ProcessFenceHighWater` | Per-process high-water cursor for every workflow process bound to the case. Empty array when no processes exist. |
| `policySnapshotDigest` | `string` | SHA-256-prefixed digest of the active policy snapshot at fence time. |
| `policySources` | array of `PolicySnapshotSource` | Provenance for the policy snapshot — the policy documents that contributed to the digest, each with `sourceKind`, `sourceRef`, `versionBinding`, `digest`. |
| `capturedAt` | `string` (RFC 3339) | Server clock at fence acquisition. |

Nested `CaseFenceEvent`, `ProcessFenceHighWater`, `PolicySnapshotSource` schemas live in the same Rust module (`workspec-server/crates/wos-server/src/http/cases.rs`) and are exported via `utoipa::ToSchema` into the OpenAPI registry alongside `CaseExportFence`.

## Locking obligation

Fence acquisition MUST be serialized against case-source mutation. The handler MUST hold `Storage::case_source_lock(case_id)` — which wraps `stack_common_postgres::ScopedAdvisoryLease` under `LeaseScope::WosCaseSource` — for the entire fence-construction transaction. The scope's namespace pin is documented in `stack-common/CLAUDE.md` lock taxonomy.

The lease is load-bearing for correctness: without it, the fence could observe a torn append-in-flight and bind a `policySnapshotDigest` + `eventCount` pair that does not match any consistent post-fence read of the ledger. Concurrent case-event append, process submit/drain/migrate/lifecycle/holds, timer fire, and integration sync are all WOS operations that mutate case source and therefore contend on the same lease.

Lease-busy timeouts MUST surface as `423 Locked` with the closed code `WOS-1423` (resource lock contention; idempotently retriable after a brief backoff). Distinct from `WOS-1409` (duplicate-ledger / state-transition conflicts; not retriable). Clients dispatch on the closed code per the public error registry.

## Verifier obligations

A downstream verifier consuming a Trellis bundle whose seal fence (`trellis-export-seal-fence-v1`) names this WOS source fence:

1. Reads `CaseExportFence` from the bundle alongside the Trellis seal fence (delivery path is profile-specific — see Trellis Core §18.3e).
2. Re-validates `caseLedgerId` matches the bundle's stored case identity.
3. Re-validates `trellisScope` matches the bundle manifest's scope bytes.
4. Re-validates `eventCount` matches the count in `010-events.cbor`. This is also cross-bound by Trellis Core §18.3e via `event_count` and `high_water_sequence`.
5. Re-validates `policySnapshotDigest` matches the policy snapshot the case referenced at fence time, looked up via `policySources`.
6. Treats `eventHighWater`, `directHighWater`, and `processHighWater` as cross-checks — values MUST be consistent with `eventCount` and with the bundle's append vector. Any disagreement is a structural verification failure.

The fence is verdict-bound: there is no partial-credit admission path past a mismatch. This composes with Trellis Core §18.3e's "reject when any seal-fence field disagrees with the manifest, event stream, derived" rule on the substrate side.

## Endpoints

| Method | Path | Body in / out | Idempotency-Key |
|---|---|---|---|
| `GET` | (per OpenAPI `get_case_export_fence` operation) | → `CaseExportFence` | n/a |

`GET` is idempotent by construction; no `Idempotency-Key` header. Errors use `application/problem+json` per ADR-0082 D-8, with `WOS-1423` (HTTP 423 Locked) for lease-busy and the standard `WOS-1400` / `WOS-1404` shapes for malformed or missing case identifiers.

## Closed taxonomies

`policySources[*].sourceKind` and `versionBinding` use closed enums defined by the `PolicySnapshotSource` schema (sibling struct in the same Rust module). The `CaseExportFence` envelope itself carries no open-extension surface — vendor extensions belong on the bundle manifest, not the source fence.

## Non-goals

- **Bundle assembly.** This fence captures pre-bundle state. Bundle layout, event encoding, manifest construction, and substrate-side seal-fence emission belong to Trellis (Trellis Core §18.3, §18.3e).
- **Per-class encryption decisions.** Whether event payloads are class-bagged is a deployment posture; the fence binds the append vector, not the plaintext shape.
- **Trellis seal fence.** `trellis-export-seal-fence-v1` is a Trellis-side manifest extension — distinct artifact, distinct authority, cross-bound by the verifier obligations above.
- **Policy snapshot construction.** How `policySnapshotDigest` is computed from `policySources` belongs to the policy-snapshot spec, not this surface.

## References

- `workspec-server/crates/wos-server/src/http/cases.rs:146-162` — Rust authority for the wire shape.
- `work-spec/api/wos-public-api.registry.openapi.json:3680` (schema), `:814` (operation), `:832` (response binding).
- `stack-common/CLAUDE.md` — lock taxonomy, `LeaseScope::WosCaseSource` row.
- `trellis/specs/trellis-core.md` §18.3e — substrate-side seal-fence counterpart, identity rule `trellis-export-seal-fence-v1`.
- ADR-0093 — case-is-its-Trellis-ledger; sibling `case-view.md` follows the same authority.
- ADR-0082 — public-API contract discipline.
