# WOS Public API Case View

**Status:** Schema authored — Refactor 3A.2 / pre-release execution.
**Schema:** [`api/case-view.schema.json`](../../schemas/api/case-view.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/case-view/v1`
**OpenAPI:** [`wos-public-api.openapi.json`](../../api/wos-public-api.openapi.json) (route rewrite follows in 3A.4).
**ADR anchors:** [ADR 0082](../../../thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md) D-4 (URN), D-12 (closed taxonomies). [ADR 0093](../../thoughts/adr/0093-case-is-its-trellis-ledger.md) §2.3 (closed event-type enum), §5 (route surface).
**Gating analysis:** [Case Boundary Decision Report](../../thoughts/analysis/case-boundary-decision-report.md) §4.4 (HTTP surface), §4.5 (schema updates), §4.7 (replay-vs-projection conformance).

## Purpose

The `CaseView` is the staff-audience read-side projection of a durable case ledger. It is keyed by the case ledger URN (`urn:wos:<tenant>_case_<ulid>`) and aggregates every workflow process bound to that ledger plus the merged provenance event stream. The case IS its Trellis ledger per CBR §1 and ADR-0093 D-1 — there is no second source of truth and no parallel `Case` aggregate.

A single architectural commitment serves two audience-appropriate routes:

- `GET /api/v1/cases/{case_id}` returns this `CaseView` shape for staff (caseworker, supervisor, adjudicator).
- `GET /api/v1/applicant/cases/{id}` returns the distinct `ApplicantCaseDetail` shape from `applicant.schema.json` for the applicant audience.

The `audienceProjection` block is the **only** allowed asymmetry between replay-read and projection-read paths. The Phase 5 N:1 conformance fixture (CBR §4.7) requires byte-identical JSON between deterministic replay and materialized projection for the same `case_id`, modulo the `audienceProjection.redactedFields` block alone. Every other field — `caseLedgerId`, `processes`, `events`, `status`, `lastUpdated` — MUST byte-match across read paths.

## Resource Shape

`CaseView` carries five required fields plus one optional audience-projection block. Required: `caseLedgerId`, `processes`, `events`, `status`, `lastUpdated`. Optional: `audienceProjection`.

`caseLedgerId` is the durable case-ledger URN. `processes` is an array of workflow process URNs (not full process projections — those live at the sibling subresource route). `events` is the merged time-ordered provenance stream across direct-append rows (no `processId`) and process-emitted rows (with `processId`). `status` is the closed `CaseStatus` rollup. `lastUpdated` is the RFC 3339 UTC timestamp of the most recent event in the merged stream.

N:1 (N workflow processes per one case ledger) is supported from day one per CBR §3. The `processes` array MAY hold zero entries when the case was created via the direct-append surface (`POST /api/v1/cases/{case_id}/events` with `wos.kernel.case_created`) and no workflow has been started yet — `status` is `genesis` in this case.

## Identifiers

`CaseView.caseLedgerId` is a `urn:wos:<typeid>` URN whose TypeID prefix is `case`. Workflow process URNs in `processes` use the `process` prefix per the dual-identity model (CBR §3, ADR-0093 D-1). Event identifiers in `events[].eventId` are storage-local opaque strings — they are NOT public URNs and are not stable across deploys.

## Endpoints

| Method | Path | Body in / out | Idempotency-Key |
|---|---|---|---|
| `GET` | `/api/v1/cases/{case_id}` | -> `CaseView` | n/a |

`GET /api/v1/cases/{case_id}` returns the staff `CaseView` projection. Returns `404` (`WOS-1404`) when no workflow process is bound to the case ledger AND no direct-append provenance row exists at that ledger URN. Returns `400` when `case_id` is not a valid case TypeID or `urn:wos:` case URN.

The applicant-audience route at `GET /api/v1/applicant/cases/{id}` returns a different shape (`ApplicantCaseDetail`) and is documented in [`applicant.md`](./applicant.md).

The case-scoped process subresource routes — `GET /api/v1/cases/{case_id}/processes/{process_id}` and its lifecycle/provenance siblings — return the full `WorkflowProcess` projection from [`instance.md`](./instance.md). The case view intentionally does NOT inline full process bodies (ADR-0082 D-3 subresource discipline).

## Pagination

`GET /api/v1/cases/{case_id}` is not paginated. The merged event stream is returned in full because case views are bounded by case-ledger size, not directory-scale enumeration. A case with sufficiently large event history to require pagination is a deployment-scaling concern that surfaces at the provenance subresource (`/cases/{case_id}/processes/{process_id}/provenance`), not the case view.

## Idempotency

`GET` is idempotent by construction; no `Idempotency-Key` header.

## Replay-vs-projection byte identity

The Phase 5 N:1 conformance fixture (CBR §4.7) verifies that two read paths return byte-identical JSON for the same `case_id`:

1. **Replay path** — start from the case-ledger genesis row, replay every provenance event in `case_ledger_id`-partitioned order, materialize `CaseView` from the replay output.
2. **Projection path** — read from the materialized projection table directly.

The two paths MUST produce identical bytes for `caseLedgerId`, `processes`, `events`, `status`, and `lastUpdated`. The `audienceProjection` block is the only allowed asymmetry: a deployment MAY decorate the projection-path response with `audienceProjection.redactedFields` (recording class-bag suppression) without breaking the conformance gate. Implementations MUST canonicalize JSON output (sorted keys, no insignificant whitespace) to make byte-identity tractable.

## Closed Taxonomies

Per ADR-0082 D-12, the case-level rollup `status` is a closed enum with no vendor extension:

| Taxonomy | Values | Extension |
|---|---|---|
| `CaseStatus` | `active`, `closed`, `genesis` | None |
| `AudienceProjection.audience` | `staff`, `applicant`, `support` | None |

`CaseStatus` rolls up from the bound processes' lifecycle states:

- `active` — at least one bound workflow process is non-terminal (`active`, `suspended`, `migrating` per `LifecycleState`).
- `closed` — every bound workflow process is in a terminal lifecycle state (`completed`, `terminated`, `stalled`).
- `genesis` — no workflow process is bound; only direct-append events exist on the case ledger.

The `AudienceProjection` block accepts `x-`-prefixed vendor extensions for deployment-specific audit metadata, but the top-level `CaseView` fields are closed.

## Errors

All non-2xx responses use `application/problem+json` per ADR-0082 D-8 and `api/error.schema.json`. Domain-relevant codes:

- `WOS-1400`: `case_id` path parameter is not a valid case TypeID or `urn:wos:` case URN.
- `WOS-1404`: no workflow process is bound to the case ledger AND no direct-append provenance row exists at the ledger URN.

## Schema Cross-References

| Schema | `$ref` | Used for |
|---|---|---|
| `_common.schema.json` | `WosResourceUrn` | Case ledger URN, process URNs, event `processId` |

## Non-Goals

- **Applicant-audience view.** The applicant route returns `ApplicantCaseDetail` from `applicant.schema.json`; the staff `CaseView` is a separate shape.
- **Full process inlining.** `processes` carries URNs only; full `WorkflowProcess` bodies live at the subresource route per ADR-0082 D-3.
- **Direct-append surface.** `POST /api/v1/cases/{case_id}/events` (direct ledger append) is documented separately; the `CaseView` shape is read-only.
- **Pagination.** Bounded by case-ledger size, not directory-scale; no `CursorToken` / `PageLimit` machinery.
- **Per-class encryption decisions.** Whether `events[].payload` is plaintext or class-bagged is a deployment posture; the schema captures the result via `audienceProjection.redactedFields` after server-side projection.
- **Status semantics beyond rollup.** `CaseStatus` is a closed taxonomy rolling up bound processes' `LifecycleState`. It does NOT carry workflow-specific business state — that lives on the individual `WorkflowProcess` subresource.

## Exported Models

The schema exports four interface models through its top-level `oneOf`: `CaseView`, `CaseStatus`, `CaseViewEvent`, `AudienceProjection`. `CaseView` is the response body for `GET /api/v1/cases/{case_id}`; the other three are nested resource models reused by the view.
