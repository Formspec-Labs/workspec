# WOS Public API Dashboard

**Status:** Schema authored — pending implementation pair (server + portal).
**Schema:** [`api/dashboard.schema.json`](../../schemas/api/dashboard.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/dashboard/v1`
**OpenAPI:** [`wos-public-api.openapi.json`](../../api/wos-public-api.openapi.json) (snapshot follow-on per ADR 0082 D-13 today; auto-emit per PLN-0401).
**ADR anchor:** [ADR 0082](../../../thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md) D-3 (decomposition over flattening), D-7 (count-style reads use cached `asOf`-stamped envelopes — NOT cursor pagination), D-12 (closed taxonomies, named seams), D-14 (no inline redefinition; cross-`$ref` `LifecycleState` from `instance.schema.json`), D-15 step 6 (dashboard / applicant / auth / bundle / audit close the user-facing entry-point surface).
**Gating ADR:** [ADR 0068 — Stack Tenant and Scope Composition](../../../thoughts/adr/0068-stack-tenant-and-scope-composition.md) (Proposed) for the `tenantScope` filter shape; this spec authors against the Proposed shape per ADR 0082 D-15 greenfield discipline and tolerates server-side absence of the field on early-deployment servers.

## Purpose

`Dashboard*` resources are **read-only count-style aggregations** across cases. The dashboard surface answers questions an operator or admin asks: "how many open cases sit in each lifecycle posture?", "how many SLA breaches happened this week?", "what governed-event volume is the platform processing?". Per ADR 0082 D-7 these are NOT lists — they are cached `asOf`-stamped envelopes computed at server cadence (typically per-minute) and shipped without cursor pagination. Clients refresh by re-fetching, not by scrolling.

The dashboard surface is intentionally narrow. Per ADR 0082 D-3 ("composition over flattening") this is one of five user-facing entry-point surfaces: a single top-level summary endpoint composes three section rollups, each of which is also retrievable as its own endpoint. Section endpoints exist so a single-purpose UI (an alerts panel, a lifecycle widget) does not pay the full-summary cost.

## Resource Shape

### Top-level `DashboardSummary`

Composes three section rollups under a single envelope-level `asOf` timestamp so consumers can decide whether to refresh.

Required fields:

- `asOf` — UTC RFC 3339 envelope-level timestamp (ADR 0082 D-7).
- `lifecycle` — `LifecycleStateRollup` (closed map).
- `slaBreaches` — `SlaBreachSummary` (count + earliest/latest envelope).
- `recentActivity` — `RecentActivitySummary` (counts grouped by `RecentActivityEventClass`).

Optional fields:

- `tenantScope` — single DNS label per ADR 0068 D-1.1 (Proposed); echoed when supplied; resolved from `X-WOS-Tenant` otherwise.
- `since`, `until` — request range echoes; defaults to the server's look-back window when absent.

Section-level `asOf` fields MAY be earlier than the envelope-level `asOf` when the server caches sections at different cadences.

### `LifecycleStateRollup`

Closed map from `LifecycleState` to count. Carried as an array of `LifecycleStateCount` pairs (`state`, `count`) rather than `additionalProperties: true` keyed by lifecycle literal so the closed-with-vendor-extension `LifecycleState` taxonomy stays load-bearing on the wire — most consumers lose enum validation when JSON object keys are dynamic. `LifecycleState` is `$ref`d directly from `instance.schema.json#/$defs/LifecycleState` per ADR 0082 D-14: the dashboard projects, never redefines.

States with zero cases MAY be omitted at server discretion.

### `SlaBreachSummary`

Count of SLA breach occurrences in the requested range (governance S10.3, S10.4 SLA semantics) plus the timestamp envelope (`earliestBreachAt`, `latestBreachAt`). Per ADR 0082 D-7 the response is a cached `asOf`-stamped envelope, not a list.

### `RecentActivitySummary`

Counts of recent governed events grouped by `RecentActivityEventClass` — case-lifecycle, task-lifecycle, review-protocol, holds, delegation, correspondence. Same `asOf`-stamped envelope discipline.

### `DashboardRangeOptions`

Query fields. All optional. The contract carries no `cursor`/`limit` fields because the response is count-style cached envelopes (ADR 0082 D-7), not lists.

## Closed Taxonomies

| Taxonomy | Source | Extension |
|---|---|---|
| `LifecycleState` | `$ref` to `instance.schema.json#/$defs/LifecycleState` (no inline) | closed-with-vendor-extension `^x-[a-z]+-` (mirrors kernel `wos-workflow.schema.json#/$defs/InstanceStatus`) |
| `RecentActivityEventClass` | new — case-lifecycle, task-lifecycle, review-protocol, holds, delegation, correspondence | closed-with-vendor-extension `^x-[a-z]+-` |

`RecentActivityEventClass` reserved literals at v1: `case-created | case-completed | case-terminated | task-created | task-submitted | task-dismissed | task-expired | review-cleared | review-flagged | hold-engaged | hold-released | delegation-granted | delegation-revoked | correspondence-sent | correspondence-received`. Vendor extensions MUST use an `^x-[a-z]+-` prefix. Adding a reserved literal requires an ADR amendment (D-12).

## Endpoints

| Method | Path | Body in / out | Idempotency-Key |
|---|---|---|---|
| `GET` | `/api/v1/dashboard/summary` | `DashboardRangeOptions` (query) -> `DashboardSummary` | n/a |
| `GET` | `/api/v1/dashboard/lifecycle-rollup` | `DashboardRangeOptions` (query) -> `LifecycleStateRollup` | n/a |
| `GET` | `/api/v1/dashboard/sla-breaches` | `DashboardRangeOptions` (query) -> `SlaBreachSummary` | n/a |
| `GET` | `/api/v1/dashboard/recent-activity` | `DashboardRangeOptions` (query) -> `RecentActivitySummary` | n/a |

`GET /api/v1/dashboard/summary` is the composed view; the three section endpoints exist for clients that only need one slice. All four endpoints accept the same `DashboardRangeOptions` query (`since`, `until`, `tenantScope`).

`GET` endpoints are idempotent by construction — no `Idempotency-Key` header required (ADR 0082 D-16).

## Pagination

**No cursor pagination.** Per ADR 0082 D-7, count-style reads on append-only logs are expensive, so the dashboard surface ships cached `asOf`-stamped envelopes that the server may compute at any cadence. Clients refresh by re-fetching the endpoint; there is no `cursor` parameter, no `limit`, no `total`, no `page`. This is the single most opinionated decision in the dashboard contract — and it is normative.

The cached envelope's `asOf` field is the load-bearing freshness signal: clients display "as of <timestamp>" alongside the rendered counts so users know the data is not transactional.

## Errors

All non-2xx responses use `application/problem+json` per ADR 0082 D-8 and `api/error.schema.json`. Domain-relevant codes from the registry:

- `WOS-1404`: dashboard scope is empty for the caller's tenant (e.g., the tenant has no cases).
- `WOS-1422`: request failed schema validation, including `tenantScope` mismatch with `X-WOS-Tenant`.
- `WOS-1503`: dashboard cache backend unavailable.

## Greenfield Discipline

Per ADR 0082 Context section and the owner's greenfield-contracts memory: prior `case-portal` dashboard projections and `workspec-server` rollup DTOs are NOT preserved. The schema rules out the worst shapes prior art tends to accumulate:

1. **No `Record<LifecycleState, integer>` open-map.** The closed-map shape is an array of typed pairs so vendor-extension `LifecycleState` literals stay load-bearing on the wire. JSON consumers that key on dynamic object keys lose enum validation by default; the array-of-pairs form preserves it.
2. **No `total` count fields.** `DashboardSummary` and the section rollups carry typed counts only — no envelope-level total cards that promise transactional consistency the server cannot deliver on append-only logs.
3. **No mixed-tier `metrics: Record<string, number>` bag.** Every count is named (`slaBreaches.count`, `recentActivity.items[].count`); vendor extensions go through the closed-with-vendor-extension `RecentActivityEventClass` seam, not anonymous keys.
4. **`asOf` is mandatory on every section.** "When was this counted?" is a load-bearing freshness contract, not optional metadata.

## Non-Goals

- **Per-actor dashboards.** This domain is the operator/admin aggregate view. Applicant-facing case views live in [`applicant.md`](./applicant.md); per-actor task inboxes live on `task.md`.
- **Time-series rollups.** The dashboard ships single-point counts. Long-tailed time-series (charts, trend lines) are a future surface — likely the reports domain (`reports.md`) or a dedicated metrics/observability seam, not the dashboard surface.
- **Real-time streaming.** Out of scope per ADR 0082 D-16. Clients refresh by re-fetching, not by subscribing.
- **Cross-tenant aggregations.** ADR 0068 D-1 makes cross-tenant reads impossible by construction. The dashboard scopes by tenant only.
- **Drill-through navigation.** The rollup surfaces counts; clients drill into the underlying cases / tasks / events through the existing `instance`, `task`, `provenance` endpoints. The dashboard does not re-export those lists.

## ADR Amendments

None required. The dashboard surface closes ADR 0082 D-15 step 6 alongside the parallel applicant / auth / bundle / audit agents. The closed-map-as-array-of-pairs pattern (D-12 named seams + D-14 no-redefine) is a precedent for any future closed-keyed map shape.
