# WOS Public API Kernel Summary

**Status:** Stable — what `GET /api/v1/bundles` returns.
**Schema:** [`api/kernel-summary.schema.json`](../../schemas/api/kernel-summary.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/kernel-summary/v1`
**Rust binding:** generated from this schema into `wos_api::types::kernel_summary` (ADR 0165 §1.1).
**OpenAPI:** [`wos-public-api.openapi.json`](../../api/wos-public-api.openapi.json) (`GET /api/v1/bundles`).

## Purpose

`GET /api/v1/bundles` returns a `KernelSummaryList`: the Workflow Document versions the server has loaded, one `KernelSummary` each, by URL then version. Each carries the document's `url`, `title`, `version`, publication `status` and `impactLevel`, copied from the document itself. A client picks a workflow from the list and reads it, with its sidecars, at `GET /api/v1/bundles/{url}`.

The route's name predates this schema; it does not return Trellis export bundles. Case exports are completion exports (`completion-exports.schema.json`, ADR 0164).

The source Workflow MUST pass its canonical schema before catalog ingestion. `url`, `title`, `version` and top-level `impactLevel` are required; aliases and nested governance fields do not replace them. Omitted publication `status` is reported as `draft`; explicit null, a wrong type, or an unknown literal is refused. A valid version `0.0.0` is retained exactly. Updating a loaded workflow MUST preserve its canonical URL; a wrapper URL that differs from the document URL is refused before storage or cache changes.
