# WOS Public API Reports

**Status:** Draft
**ADR:** [`thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md`](../../../thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md) (D-15 step 2)
**Schema:** [`api/reports.schema.json`](../../schemas/api/reports.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/reports/v1`

## Purpose

Report runs project governance, AI oversight, equity, and operational telemetry into a public read-model surface. Each run is a typed analytical query: a closed `reportType` selects a fixed input shape and a fixed row shape. Runs are derived; they are not authoritative audit records and do not replace provenance, the equity-config sidecar reports, or cross-case audit exports.

A report run owns a parameterisation (`input`), a queue lifecycle (`status`), and (on success) a typed result accessible as a paginated rows subresource. The endpoint family is greenfield; the prior `ReportTemplate` / `GeneratedReport` shapes in `case-portal/src/ports/types.ts` and `IReportsPort.ts` are advisory only.

## Resource Shape

`ReportRun` carries identity, lifecycle, parameterisation, and outcome. Required fields: `id`, `reportType`, `status`, `input`, `submittedBy`, `queuedAt`. Optional fields, omitted when absent: `startedAt`, `completedAt`, `rowCount`, `failure`.

`reportType` is a closed enum with vendor-prefixed extension literals per ADR 0082 D-12. Reserved literals at v1:

- `decision-drift`: per-bucket reviewer override and AI confidence trends.
- `sla-performance`: per-task-pattern SLA compliance and time-to-decision.
- `equity-disparity`: equity-config disparity projection per protected category.
- `caseload-summary`: open / closed instance counts grouped by outcome.
- `ai-override-rate`: per-agent reviewer override rate.
- `timer-breach`: kernel timer expirations and governance hold breaches.
- `reviewer-engagement`: per-reviewer agreement-rate and review-depth signals.
- `agent-shadow-mode-divergence`: per-agent divergence between shadow-mode output and the configured baseline (advanced/advanced-governance.md shadow mode; advanced-governance.md:370-385). Paired with `governance.schema.json#/$defs/AgentView.shadowMode` — the same agent declaration drives both. Row carries `comparisonsCount`, `divergenceCount`, `divergenceRate`, optional `averageDivergenceMagnitude` and `baselineModelIdentifier`.

Vendor-extension `reportType` literals are reserved at the URN / wire layer but are NOT accepted by the v1 input or row contract — the input and row shapes are exhaustively defined for the reserved literals only. A vendor-extension report family ships its own `schemas/api/<resource>.schema.json` rather than overloading this one.

`status` is a closed lifecycle: `pending`, `running`, `succeeded`, `failed`, `cancelled`. Time fields (`queuedAt`, `startedAt`, `completedAt`) are RFC 3339 UTC per ADR 0082 D-10.

`input` is a closed discriminated union (`ReportInput`) keyed on `reportType`. Each variant is a closed object with `additionalProperties: false`; there is no open `Record<string, unknown>` parameter bag. The seven variants (`DecisionDriftInput`, `SlaPerformanceInput`, `EquityDisparityInput`, `CaseloadSummaryInput`, `AiOverrideRateInput`, `TimerBreachInput`, `ReviewerEngagementInput`) each carry a fixed `timeRange` and a small set of typed scoping fields. The `WorkflowUrl` reuses the WOS author-time workflow document URL.

Result rows are NOT carried inline on `ReportRun`. The rows subresource (`GET /api/v1/reports/runs/{id}/rows`) returns `ReportRowPage`, where every page row matches the run's `reportType` via the same closed discriminated union (`ReportRow`). The run envelope's `rowCount` advertises the total row count once `status == succeeded`.

`failure` is set when `status == failed`, carrying a stable `wosErrorCode` plus human-readable `title` and optional `detail`. Failure codes draw from the public registry (`error-registry.md`).

## Identifiers

`ReportRun.id` is a `urn:wos:report-run:<scope>:<date>:<short-hash>` URN per ADR 0082 D-4. `report-run` is already in the URN entity-type enum (`api/notification.schema.json` `WosResourceUrn`); this spec consumes it without extension. Submitter actors use the standard `ActorRef` (`actor:(human|service-account|workload|support):...`).

## Endpoints

```
POST  /api/v1/reports/runs                 -> ReportRun                (queue a new run)
GET   /api/v1/reports/runs                 -> ReportRunPage             (cursor-paginated)
GET   /api/v1/reports/runs/{id}            -> ReportRun
GET   /api/v1/reports/runs/{id}/rows       -> ReportRowPage             (cursor-paginated)
POST  /api/v1/reports/runs/{id}/cancel     -> ReportRun                 (idempotent)
GET   /api/v1/reports/runs/{id}/export     -> binary stream             (CSV / JSON / parquet)
```

`POST /api/v1/reports/runs` accepts `CreateReportRunRequest { input, maxDurationSeconds? }`. The server rejects requests whose `input.reportType` is a vendor-extension literal at v1 with `WOS-1422`. `Idempotency-Key` is required (ADR 0082 D-16); a repeat request within the retention window returns the original run unchanged.

`maxDurationSeconds` is the caller-supplied wall-clock timeout ceiling in seconds (`minimum: 1`, `maximum: 86400`). Reports can scan up to `maxResults: 100000` rows; without a ceiling a stuck runner stays `running` indefinitely. **Server-enforced default is 3600 seconds** when the caller omits the field; the per-deployment ceiling applies regardless and clamps higher requests downward without rejection. The resolved value is server-stamped at queue time on `ReportRun.maxDurationSeconds` (REQUIRED) so callers can see what was applied without round-tripping the server config.

`GET /api/v1/reports/runs` accepts `ReportRunListOptions`: optional `reportType`, `status`, `submittedBy` filters, plus `cursor` and `limit` (max 200). Returns `ReportRunPage`. Runs accrete monotonically by `queuedAt`, so cursor pagination is the right posture (ADR 0082 D-7); there is no `total`, `page`, or `pageSize` echo.

`GET /api/v1/reports/runs/{id}` returns the run envelope. While `status in {pending, running}`, clients poll with their own backoff; the v1 contract does not include push notifications.

`GET /api/v1/reports/runs/{id}/rows` returns one cursor-paginated page of rows; the page carries `runId` and `reportType` so clients can verify alignment without consulting the run resource. Available only when `status == succeeded`; otherwise the server responds `409 Conflict` with `WOS-1409`.

`POST /api/v1/reports/runs/{id}/cancel` accepts an optional `CancelReportRunRequest { reason }`. Idempotent — a cancel request against a terminal run returns the existing terminal run without modification.

`GET /api/v1/reports/runs/{id}/export?format=csv|json|parquet` streams the result body in the requested closed `ReportOutputFormat`. Format defaults to `json` if omitted. Available only when `status == succeeded`.

## Pagination

Both `GET /api/v1/reports/runs` and `GET /api/v1/reports/runs/{id}/rows` use cursor pagination per `api/pagination.schema.json`. Cursors are opaque, single-use within the issuing deploy. Cursor expiry returns `410 Gone` with `WOS-1410`.

## Errors

All non-2xx responses use `application/problem+json` per ADR 0082 D-8 and `api/error.schema.json`. Domain-relevant codes from the registry:

- `WOS-1404`: report run does not exist or is not in the caller's scope.
- `WOS-1408`: timeout exceeded. The run's wall-clock execution exceeded `ReportRun.maxDurationSeconds`; the runner terminates the run, sets `status == failed`, populates `failure.wosErrorCode == "WOS-1408"`, and records a `reportTimedOut` Facts-tier provenance literal with `reportTimedOut` typed payload. Cross-cite [`error-registry.md`](./error-registry.md).
- `WOS-1409`: rows or export requested before `status == succeeded`, or cancel against a terminal run that returned a non-cancellation outcome.
- `WOS-1410`: cursor expired.
- `WOS-1422`: input failed schema validation, including vendor-extension `reportType` at v1, mismatched `equityDisparityInput.protectedCategoryId`, inverted `timeRange`, or `maxDurationSeconds` outside the `[1, 86400]` bound.
- `WOS-1503`: report execution backend unavailable.

## Non-Goals

This spec does not define: report scheduling (recurring runs), pinning to dashboards, cross-tenant aggregate reports, streaming partial-row delivery, or report-result anchoring into Trellis. Those concerns get separate ADRs and / or schemas. Authoritative audit evidence remains the provenance log; equity-config-driven recurring reporting (Equity Config S5) remains the equity sidecar's responsibility.

## Notes for ADR 0082 implementers

ADR 0082 D-15 step 2 grouped `correspondence-message` and `report-run` as "same `oneOf`-free shape". Reports cannot be authored that way without losing the named-seam discipline ADR 0082 D-12 mandates: a `Record<string, unknown>` `input` bag would re-introduce the open-taxonomy escape hatch the platform thesis exists to eliminate, and a single anonymous `rows: unknown[]` array would forfeit the typed projection. This spec keeps the `ReportRun` envelope `oneOf`-free at the top level — `id`, `status`, `submittedBy`, time fields, and `failure` are common — and pushes the discriminated unions inward to `ReportInput` and `ReportRow`. Both unions are closed (no vendor extensions accepted at v1) and discriminated on `reportType`, matching the pattern ADR 0082 D-5 sets for `ProvenanceRecord`. An ADR 0082 amendment relaxing D-15 step 2 to "report-run inputs and rows are closed discriminated unions on `reportType`; the run envelope is `oneOf`-free" would make the contract self-consistent.
