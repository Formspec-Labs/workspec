# WOS Public API Pagination

**Status:** Implemented
**Schema:** [`api/pagination.schema.json`](../../schemas/api/pagination.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/pagination/v1`

## Purpose

Public WOS list endpoints use cursor pagination for append-only or concurrently changing resources. Page numbers and totals are not part of the public list contract.

## Request Shape

List endpoints use the `PaginationQuery` fields where pagination is needed:

- `cursor`: opaque resume token returned by a prior page.
- `limit`: requested page size. The public maximum is 200 unless a resource-specific spec sets a lower limit.

Clients must treat cursors as opaque. They must not persist cursors across sessions or share them across users.

## Response Shape

Paginated responses use the `PaginatedResult` envelope or a resource-specific equivalent with the same contract:

- `items`: current page of resources.
- `cursor`: next-page token, omitted when `hasMore` is false.
- `hasMore`: true when another page is available.

There is no `total`, `page`, or `pageSize` echo. Counts are separate resources, stamped with `asOf`, when a domain has a real count use case.

## Cursor Expiry

If a cursor is no longer valid, the server responds with `410 Gone` and `wosErrorCode: WOS-1410`. Clients restart pagination from the beginning.
