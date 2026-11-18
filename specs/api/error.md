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
- `error_code`: stable machine-readable code from `error-registry.md`.

Optional fields:

- `detail`: occurrence-specific human-readable detail.
- `instance`: affected WOS resource URN when one resource caused the problem.
- `context`: problem-specific diagnostic object. This is the only anonymous object extension point in the public error contract.

## Codes

`error_code` values are stable once published. Add new codes to `error-registry.md`. Server implementations map internal errors to registered public codes and use the `Problem` body for public errors.

## OpenAPI Binding

The OpenAPI snapshot at [`wos-public-api.openapi.json`](../../api/wos-public-api.openapi.json) publishes the shared shape as `components.schemas.Problem`. Operations can declare narrower problem shapes for specific responses.
