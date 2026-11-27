# WOS Public API Pagination

**Status:** Implemented
**Schema:** [`api/pagination.schema.json`](../../schemas/api/pagination.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/pagination/v1`

## Purpose

Public WOS list endpoints use cursor pagination for append-only or concurrently changing resources. Page numbers and totals are not part of the public list contract.

## Request Shape

List endpoints use the `PaginationQuery` fields where pagination is needed:

- `cursor`: opaque, single-use, server-sealed resume token returned by a prior page.
- `limit`: requested page size. The public maximum is 200 unless a resource-specific spec sets a lower limit.

Clients must treat cursors as opaque. Cursors are single-use and bound to the issuing deployment, admitted scope, authenticated principal, query filter set, and sort order. Servers MUST NOT emit decimal offsets or any value derived from unauthorized rows. Clients must not parse, replay, persist across sessions, or share cursors across principals or scopes.

## Response Shape

Paginated responses use the `PaginatedResult` envelope or a resource-specific equivalent with the same contract:

- `items`: current page of authorized resources.
- `cursor`: opaque single-use sealed next-page token, omitted when `hasMore` is `false` or `searchIncomplete` is `true`.
- `hasMore`: `true` when another authorized page is confirmed available; `false` on exhaustion or when `searchIncomplete` is `true`.
- `searchIncomplete`: optional boolean; `true` when the server's per-page authorization scan budget was spent before establishing whether another authorized page exists. When `searchIncomplete` is `true`, `hasMore` MUST be `false` and `cursor` MUST be omitted so nothing returned is derived from rows the caller may not see. Callers retry rather than treating the list as exhausted.

There is no `total`, `page`, or `pageSize` echo. Unauthorized rows MUST NOT influence visible items, cursor payload, or continuation state; two datasets differing only in unauthorized rows MUST produce indistinguishable responses within the scan budget. Counts are separate resources, stamped with `asOf`, when a domain has a real authorized count use case.

## Cursor Expiry and Invalidity

If a cursor is expired, already consumed (replayed), tampered with, malformed (including plain decimal offsets), or bound to a different principal, scope, query filter set, sort order, or deployment, the server fails closed with `410 Gone` and `wosErrorCode: WOS-1410`. Clients restart pagination from the beginning.
