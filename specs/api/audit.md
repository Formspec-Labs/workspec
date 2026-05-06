# WOS Public API Audit

**Status:** Schema authored — pending implementation pair (server + portal).
**Schema:** [`api/audit.schema.json`](../../schemas/api/audit.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/audit/v1`
**OpenAPI:** [`wos-public-api.openapi.json`](../../api/wos-public-api.openapi.json) (snapshot follow-on per ADR 0082 D-13 today; auto-emit per PLN-0401).
**ADR anchor:** [ADR 0082](../../../thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md) D-4 (URN), D-7 (cursor pagination), D-9 (`actorRef` URN), D-12 (closed taxonomies), D-15 step 6 (bundle + audit close the export surface), D-16 (Idempotency-Key on materialize).
**Gating ADRs:** [ADR 0068](../../../thoughts/adr/0068-stack-tenant-and-scope-composition.md) D-2 (Proposed) — auth scope composition; [PLN-0381](../../../PLANNING.md) — stack identity-attestation ADR (Proposed) — `AuditAttestationView` shape gates on PLN-0381 promotion.
**Related:** [`api/provenance.schema.json`](../../schemas/api/provenance.schema.json) — audit cross-`$ref`s `ProvenanceRecord` and tier sub-types; the per-case shape is the audit shape.

## Purpose

Audit is the **cross-case retrospective query** surface for compliance investigations, statistical evidence reviews, and identity-attestation searches. It is **distinct from `provenance.schema.json`**:

| Surface | Scope | Authority | Use case |
|---|---|---|---|
| `provenance` (per-case) | One `instance` URN | Authoritative live record stream | "Show me the decision trail for case X." |
| `audit` (cross-case) | Tenant / Org / Workspace / Environment scope (ADR 0068 D-2) | Projected; same record shape | "Across all cases this quarter, find every `identityAttestation` from provider Y." |

Audit reuses the per-case provenance shape. `AuditQueryResult.items` cross-`$ref`s `provenance.schema.json#/$defs/ProvenanceRecord` directly — there is no second meaning of `record`. Investigators cross-reference cases without learning a second projection.

Per ADR 0082 D-15 step 6, this domain pairs with `bundle.schema.json` to close the export / cross-case API surface.

## Resource Shapes

### `AuditQueryRequest`

Closed envelope: `scope` (REQUIRED), optional `timeRange`, `actorRef`, `tierFilter`, `recordKindFilter`, `instanceScope`, `sort`, `maxResults`, `materialize`. There is no open `criteria: Record<string, unknown>` parameter bag — every filter is named and scoped.

`scope` is `AuditScopeFilter`, a closed object on Tenant / Organization / Workspace / Environment per VISION §V scope hierarchy. **Tenant is REQUIRED**; cross-tenant audit is structurally inexpressible by construction (ADR 0068 D-1: the runtime refuses cross-tenant reads).

`tierFilter` is a non-empty array drawn from `{facts, reasoning, counterfactual, narrative}` — the four provenance tiers. Mirrors the `tier` discriminator on `ProvenanceRecord`. Omit to span all four.

`recordKindFilter` cross-`$ref`s the closed-with-vendor-extension `FactsRecordKind` taxonomy from `provenance.schema.json` — the same enum the Facts-tier records use. Vendor-extension literals are reserved at the URN / wire layer per ADR 0082 D-12.

`materialize: true` requests durable result storage so a long-running query can be polled across multiple GETs. **`Idempotency-Key` is REQUIRED when `materialize: true`** (ADR 0082 D-16) because materialization is an externally-visible side effect.

### `AuditQueryResult`

Cursor-paginated envelope per ADR 0082 D-7. Required fields: `queryId`, `status`, `items`, `hasMore`. Optional: `cursor`, `submittedAt`, `completedAt`, `failure`. `items` is the typed `ProvenanceRecord[]` cross-`$ref`ed from `provenance.schema.json`.

`status` is a closed lifecycle: `pending`, `running`, `succeeded`, `failed`, `cancelled`. Synchronous queries (`materialize: false`) return `succeeded` directly with the first page inline.

### `AuditScopeFilter`

Closed object — Tenant / Organization / Workspace / Environment scope per ADR 0068 D-2 (**Proposed**). Tenant is REQUIRED; organization, workspace, environment are optional refinements. The shape is authored against the proposed ADR 0068 D-2 settlement so consumers have a stable contract; when ADR 0068 promotes to Accepted this object becomes the wire form audit scope is reported in. Until then, server implementations MAY accept only the fields they have settled.

### `AuditAttestationView`

Projected identity-attestation chain for a single subject across cases. Aggregates `IdentityAttestation` Facts-tier records (Kernel S8 / ADR 0068 D-3.1) so an investigator can answer "what attestations did this subject hold, by which provider, with what assurance, when?" without enumerating cases by hand.

**This shape gates on [PLN-0381](../../../PLANNING.md)** — the stack identity-attestation ADR (Proposed; supersedes PLN-0310). The projection is authored against ADR 0068 D-3.1's `IdentityAttestation` record shape; the wire form ratifies when PLN-0381 lands. Until then, servers MAY return only the fields they have settled and clients tolerate field absence.

`subjectGlobalId` is the tenant-independent URI subject identifier (`did:web:...`, `urn:idp:subject:...`) per ADR 0068 D-3.1. `attestations` is an array of Facts-tier `ProvenanceRecord` instances cross-`$ref`ed from `provenance.schema.json` (the audit projection is the per-case shape). `highestAssuranceLevel` mirrors the closed-with-vendor-extension `assuranceLevel` taxonomy from `wos-workflow.schema.json` `IdentityAttestationRecord.assuranceLevel`.

### `AuditQuerySort`

Closed: `timestamp-asc | timestamp-desc`. No vendor-extension seam — sort orders are normative and a verifier MUST be able to reproduce a result set deterministically.

## Identifiers

`AuditQueryResult.queryId` is a server-issued opaque string token (not a `urn:wos:...` URN — query handles are server-internal, not domain entities). `actorRef` filters use the standard `ActorRef` URN from `_common.schema.json` (`actor:(human|service-account|workload|support):...`). `instanceScope` filters use `WosResourceUrn` (`urn:wos:instance:...`). `recordKindFilter` literals use the `FactsRecordKind` cross-`$ref` so adding a new kernel record kind propagates automatically.

## Endpoints

| Method | Path | Body in / out | Idempotency-Key |
|---|---|---|---|
| `POST` | `/api/v1/audit/queries` | `AuditQueryRequest` -> `AuditQueryResult` (cursor envelope) | **REQUIRED when `materialize: true`** |
| `GET` | `/api/v1/audit/queries/{queryId}` | -> `AuditQueryResult` (status + first page) | n/a |
| `GET` | `/api/v1/audit/queries/{queryId}/results?cursor=...&limit=...` | -> `AuditQueryResult` | n/a |
| `GET` | `/api/v1/audit/records/{provenanceUrn}` | -> `ProvenanceRecord` (cross-`$ref`) | n/a |

`POST /api/v1/audit/queries` submits a cross-case query. When `materialize: false` (default) the server executes synchronously and returns `AuditQueryResult` with `status: succeeded` and the first page inline. When `materialize: true`, the server queues the query and returns immediately with `status` typically `pending` or `running` and a `queryId` the client uses for polling. **`Idempotency-Key` is REQUIRED when `materialize: true`** per ADR 0082 D-16.

`GET /api/v1/audit/queries/{queryId}` fetches the current status of a materialized query plus the first page of results. Supports deferred-execution polling for long queries: clients poll until `status in {succeeded, failed, cancelled}`.

`GET /api/v1/audit/queries/{queryId}/results?cursor=...` paginates the materialized result set per ADR 0082 D-7. Available only when `status == succeeded`; returns `409 Conflict` (`WOS-1409`) otherwise.

`GET /api/v1/audit/records/{provenanceUrn}` fetches a single provenance record by URN — the same shape `provenance.schema.json` returns for the per-case `GET /api/v1/instances/{id}/provenance/{recordId}` endpoint, but accessible cross-case under audit scope. Useful for "I have a record URN from a federated investigation, give me the record" workflows. Returns `404` (`WOS-1404`) when the URN is not visible to the caller's scope.

## Pagination

`GET /api/v1/audit/queries/{queryId}/results` uses cursor pagination per `api/pagination.schema.json`. Cursors are opaque, single-use within the issuing deploy. Cursor expiry returns `410 Gone` with `WOS-1410`. Audit queries can return very large result sets — `maxResults` on the `AuditQueryRequest` caps the total result corpus; `limit` on the result endpoints caps the page size.

## Errors

All non-2xx responses use `application/problem+json` per ADR 0082 D-8 and `api/error.schema.json`. Domain-relevant codes:

- `WOS-1404`: query ID does not exist, or `provenanceUrn` is not in the caller's scope.
- `WOS-1409`: result page requested before `status == succeeded`, or cancel against a terminal query.
- `WOS-1410`: cursor expired.
- `WOS-1422`: query rejected — invalid `scope.tenant`, empty filter array, vendor-extension literal in a position not allowed at v1, inverted `timeRange`, `materialize: true` without `Idempotency-Key`.
- `WOS-1503`: audit backend unavailable.

## Greenfield Discipline

Per ADR 0082 Context section and the owner's greenfield-contracts memory: prior `case-portal` and `workspec-server` audit DTOs are NOT preserved. The schema makes the worst shapes that prior art accumulated structurally inexpressible:

1. **No second meaning of `record`.** `AuditQueryResult.items` cross-`$ref`s `provenance.schema.json#/$defs/ProvenanceRecord` — the per-case shape is the audit shape. There is no `AuditRecord` type.
2. **No open `criteria: Record<string, unknown>` bag.** Every filter on `AuditQueryRequest` is named and scoped. Vendor extensions live only on `recordKindFilter` literals (via the cross-`$ref`ed `FactsRecordKind`).
3. **Tenant filter is REQUIRED.** Cross-tenant audit is structurally inexpressible (ADR 0068 D-1: substrate boundary). The runtime refuses queries whose `tenant` does not match the caller's authenticated scope.
4. **`AuditAttestationView` is a projection of Facts-tier records, not a parallel shape.** The chain is `FactsTierRecord[]` cross-`$ref`ed; `IdentityAttestation` is a `recordKind` literal under that shape (PLN-0381 ratifies the kernel literal).

## Relationship to Provenance

`provenance.schema.json` is the per-case live record stream — bound to one `instance` URN, append-only, paginated forward. `audit.schema.json` is the cross-case retrospective query — bound to a Tenant / Org / Workspace / Environment scope, queryable by tier / record-kind / actor / time, optionally materialized for deferred polling. **Audit reuses provenance's `ProvenanceRecord` shape** so the same record carries through both surfaces; investigators do not learn a second projection.

| Concern | Provenance | Audit |
|---|---|---|
| Scope | One `instance` | Tenant + sub-scope |
| Read direction | Forward append | Backward query |
| Pagination | Cursor (D-7) | Cursor (D-7) |
| Authority | Authoritative live stream | Projected cross-case view |
| Identity-attestation | One record per case | `AuditAttestationView` chain across cases |

## Non-Goals

- **Cross-tenant queries.** ADR 0068 D-1 substrate-boundary refuses cross-tenant reads. Federation across tenants is a future ADR slot (ADR 0068 D-5 supersession is the current pattern).
- **Append / write paths.** Audit is read-only on the public API. Provenance records are emitted by the runtime through internal seams (`provenanceLayer`), not by external clients.
- **Real-time streaming.** When pushed-update support arrives, it gets its own ADR (ADR 0082 Out-of-Scope: streaming). Today audit is request/response with deferred-execution support for long queries.
- **Bundle export.** Trellis-side export bundles use `api/bundle.schema.json`; audit returns the JSON record stream, not the CBOR byte stream.
- **Cursor durability across deploys.** Per ADR 0082 D-7 cursors are deploy-lifetime; clients restart pagination from the top after `WOS-1410`.

## ADR Amendments

None required. The bundle/audit pair closes ADR 0082 D-15 step 6 alongside the parallel dashboard / applicant / auth agents. `AuditAttestationView` is the first audit-domain shape gating on PLN-0381 (identity attestation); the cross-`$ref` to `FactsTierRecord` ensures the wire shape ratifies automatically when PLN-0381 lands its `recordKind: identityAttestation` literal.
