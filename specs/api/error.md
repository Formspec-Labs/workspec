# WOS Public API Error

**Status:** Implemented
**Schema:** [`api/error.schema.json`](../../schemas/api/error.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/error/v1`
**Registry:** [`error-registry.md`](./error-registry.md)

## Purpose

Every non-2xx WOS public REST API response uses RFC 7807 problem details with content type `application/problem+json`. The wire body is the `Problem` definition in `api/error.schema.json`.

## Problem Shape

`Problem` requires:

- `type`: dereferenceable URI for the documented error type.
- `title`: short human-readable summary.
- `status`: HTTP status code, 400 through 599.
- `wosErrorCode`: stable machine-readable code from `error-registry.md`.

Optional fields:

- `detail`: occurrence-specific human-readable detail.
- `instance`: affected WOS resource URN when one resource caused the problem.
- `context`: problem-specific diagnostic object. This is the only anonymous object extension point in the public error contract.

## Codes

`wosErrorCode` values are stable once published. New codes are added to `error-registry.md`; existing codes are not repurposed. Server implementations map internal errors to the closest registered public code and must not expose legacy `{ "error": ... }` envelopes on the public API surface.

## OpenAPI Binding

The OpenAPI snapshot at [`wos-public-api.openapi.json`](../../api/wos-public-api.openapi.json) references this schema through `components.responses.Problem`. Endpoint-specific default error responses use that component unless a narrower response is specified.
