# WOS Public API Appeal

**Status:** Schema authored — pending implementation pair (server + studio).
**Schema:** [`api/appeal.schema.json`](../../schemas/api/appeal.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/appeal/v1`
**OpenAPI:** [`wos-public-api.openapi.json`](../../api/wos-public-api.openapi.json) (snapshot follow-on per ADR 0082 D-13 today; auto-emit per PLN-0401).
**ADR anchor:** [ADR 0082](../../../thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md) D-4 (URN, `appeal` entity-type already admitted in `_common.schema.json` Phase 1), D-7 (cursor pagination), D-12 (closed taxonomies), D-16 (Idempotency-Key on POST).
**Gating ADR:** [Governance S3.5 Appeal Review](../../specs/workflow-governance.md) (appeal filing, independence verification, disposition lifecycle); [ADR 0064 Agent Actor Kind and Invoker Port](../../thoughts/adr/0064-agent-actor-kind-and-invoker-port.md) (ActorRef carry-through).

## Purpose

An `Appeal` is a cross-case governance artifact: an appellant challenges an adverse determination on a **closed** workflow process, triggering a governed review process against a declared ground for appeal. An adjudicator reviews the appeal and records a closed disposition (`upheld`, `overturned`, `remanded`, `modified`, `dismissed`). The appeal lifecycle is a closed taxonomy (`pending`, `accepted`, `denied`, `withdrawn`) — no vendor extension on lifecycle state or disposition.

Appeals are filed against a specific instance (must be in a completed lifecycle state — `WOS-1409` on non-completed instances). The appeal record links back to the instance via `processId` (instance URN) and to the appellant via `appellantRef` (ActorRef URN per ADR 0082 D-9). When routed through a governed review process, the adjudicator is recorded via `adjudicatorRef`.

Appeals do NOT reopen the case — they create a separate cross-case governance track. The case's `WorkflowProcessGovernance` subresource (instance.schema.json) may carry a per-case projection summarizing the appeal state, but the `Appeal` shape here is the authoritative cross-case record.

## Resource Shape

`Appeal` carries identity, instance linkage, appellant, status, and disposition. Required fields: `id`, `processId`, `appellantRef`, `filedAt`, `status`. Optional fields, omitted when absent: `groundForAppeal`, `disposition`, `dispositionAt`, `adjudicatorRef`, `createdAt`.

`status` is a closed lifecycle: `pending` (filed, awaiting adjudicator acceptance), `accepted` (under review), `denied` (adjudicator denied the appeal), `withdrawn` (appellant withdrew before disposition). Closed enum with no extension seam.

`disposition` is a closed taxonomy presented only when `status` is `accepted` or `denied`: `upheld` (original determination stands), `overturned` (original determination reversed), `remanded` (sent back for re-determination), `modified` (determination adjusted), `dismissed` (appeal dismissed without reaching merits). When `disposition` is present, `dispositionAt` MUST also be present (RFC 3339 UTC timestamp of the disposition recording).

`adjudicatorRef` is the ActorRef URN of the adjudicator reviewing the appeal. Present when an adjudicator has been assigned; absent while the appeal is unassigned.

`createdAt` is the RFC 3339 UTC timestamp when the appeal resource was created in the governance subsystem. Distinct from `filedAt` (the appellant's filing timestamp) — `createdAt` is the server-recorded persistence timestamp.

## Identifiers

`Appeal.id` is a `urn:wos:<typeid>` URN per ADR 0092 D-1. `Appeal.processId` is a `urn:wos:<typeid>` URN of the owning case.

## Endpoints

| Method | Path | Body in / out | Idempotency-Key |
|---|---|---|---|
| `POST` | `/api/v1/instances/{id}/appeals` | `AppealCreateRequest` -> `Appeal` | **REQUIRED** |
| `GET` | `/api/v1/appeals/{urn}` | -> `Appeal` | n/a |
| `GET` | `/api/v1/instances/{id}/appeals` | `AppealListOptions` (query) -> `AppealPage` | n/a |

`POST /api/v1/instances/{id}/appeals` files an appeal against a completed workflow process. **`Idempotency-Key` is REQUIRED** per ADR 0082 D-16. The server validates: the instance exists and is in a completed lifecycle state (`WOS-1409` if not), the appellant has standing to appeal, and the `groundForAppeal` is non-empty. Body: `AppealCreateRequest { processId, appellantRef, groundForAppeal }`. Response: an `Appeal` resource with `status: "pending"`. Server assigns `id`, `filedAt`, and `createdAt`; clients MUST NOT supply them.

`GET /api/v1/appeals/{urn}` returns a single appeal by its URN. Returns `404` (`WOS-1404`) when the URN is not visible to the caller's scope. This endpoint is the canonical lookup for an appeal resource; use it to poll `status` after filing.

`GET /api/v1/instances/{id}/appeals` is cursor-paginated per ADR 0082 D-7. Filters: `status` (most common: `pending` for unadjudicated appeals). Cursors are deploy-lifetime stable; `WOS-1410` on expiry triggers client restart from the top.

## Pagination

`GET /api/v1/instances/{id}/appeals` uses cursor pagination per `api/pagination.schema.json`. Cursors are opaque, single-use within the issuing deploy. Default ordering is `createdAt` ascending so paginating clients observe a stable monotone walk. Cursor expiry returns `410 Gone` with `WOS-1410`.

## Idempotency

`POST /api/v1/instances/{id}/appeals` requires `Idempotency-Key` per ADR 0082 D-16 — filing an appeal is an externally-visible side effect (governance records emit, provenance events fire). A repeat request within the retention window returns the original `Appeal` resource unchanged.

`GET /api/v1/appeals/{urn}` and `GET /api/v1/instances/{id}/appeals` are idempotent by construction.

## Errors

All non-2xx responses use `application/problem+json` per ADR 0082 D-8 and `api/error.schema.json`. Domain-relevant codes:

- `WOS-1404`: appeal URN does not exist or is not in the caller's scope.
- `WOS-1409`: appeal filed against a non-completed instance, or appeal state transition attempted from an invalid lifecycle posture.
- `WOS-1422`: `AppealCreateRequest.processId` is not a valid `urn:wos:<typeid>` URN, or `groundForAppeal` is empty, or `appellantRef` is not a valid ActorRef.

## Closed Taxonomies

Per ADR 0082 D-12, appeal lifecycle status and disposition are closed enums with no vendor extension:

| Taxonomy | Values | Extension |
|---|---|---|
| `AppealStatus` | `pending`, `accepted`, `denied`, `withdrawn` | None |
| `AppealDisposition` | `upheld`, `overturned`, `remanded`, `modified`, `dismissed` | None |

New statuses or dispositions require a schema major bump. The closed posture reflects that appeal lifecycle and disposition are normative governance primitives — an organization's appeal process either fits within these or does not.

## Schema Cross-References

| Schema | `$ref` | Used for |
|---|---|---|
| `_common.schema.json` | `ActorRef` | Appellant and adjudicator references |
| `_common.schema.json` | `WosResourceUrn` | Appeal and instance URNs |
| `pagination.schema.json` | `CursorToken` | AppealListOptions cursor, AppealPage cursor |
| `pagination.schema.json` | `PageLimit` | AppealListOptions limit |

## Non-Goals

- **Appeal authoring config.** Configuring appeal windows, grounds taxonomy, routing rules, and independence requirements are author-time governance concerns (workflow governance S3.5). This API surface is the runtime filing and lookup seam only.
- **Appeal routing rules.** Which adjudicator receives which appeal is a governance runtime concern driven by the workflow's governance block — NOT encoded in this API schema.
- **Appeal window configuration.** Time limits for filing after determination, expiration policies, and late-filing grace periods are governance author-time parameters, not API-level concerns.
- **Disposition enforcement.** Recording a `disposition` against an appeal does not automatically reopen or alter the original instance — the disposition is documentary. Any automated re-opening or re-determination is a governance runtime behavior outside the appeal resource shape.
