# WOS Public API Applicant Views

**Status:** Schema authored — pending implementation pair (server + portal).
**Schema:** [`api/applicant.schema.json`](../../schemas/api/applicant.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/applicant/v1`
**OpenAPI:** [`wos-public-api.openapi.json`](../../api/wos-public-api.openapi.json) (snapshot follow-on per ADR 0082 D-13 today; auto-emit per PLN-0401).
**ADR anchor:** [ADR 0082](../../../thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md) D-3 (subresource decomposition, applicant-owed task subresource), D-4 (URN identifiers), D-7 (cursor pagination), D-9 (`ActorRef`), D-12 (closed-with-vendor-extension taxonomies), D-14 (no inline redefinition; cross-`$ref` `LifecycleState` from `instance.schema.json`), D-15 step 6 (dashboard / applicant / auth / bundle / audit close the user-facing entry-point surface).
**Gating ADR:** [ADR 0068 — Stack Tenant and Scope Composition](../../../thoughts/adr/0068-stack-tenant-and-scope-composition.md) (Proposed) for the applicant-scoped tenant filter at the API boundary; this spec authors against the proposed shape per ADR 0082 D-15 greenfield discipline.

## Purpose

The applicant API surface is the **case-subject view** — narrower projections of staff `instance.schema.json` and `task.schema.json` shapes, scoped to applicant authority. Applicants are the case subjects (or their authorized agents) — distinct from staff caseworkers, reviewers, and adjudicators. The contract makes the authority boundary load-bearing: governance internals (delegation rationale, review-flagged details, hold legal-review reasoning) are NOT exposed; applicant-owed tasks are NOT mixed with staff-owed tasks; the status timeline shows applicant-visible events only.

Per ADR 0082 D-3 the surface is subresource-shaped: `cases` (list), `cases/{id}` (detail), `cases/{id}/tasks` (applicant-owed tasks subresource), `notifications` (applicant inbox feed). Per ADR 0082 D-14 the contract `$ref`s `LifecycleState` from `instance.schema.json` and `WosResourceUrn` from `_common.schema.json` — never redefining.

## Resource Shape

### `ApplicantCaseSummary`

Lightweight projection for list views. Carries the case URN, governing workflow, lifecycle posture, last-update timestamp, and an `actionNeeded` flag indicating whether at least one applicant-owed task is in `pending` or `drafted` status.

Required: `id`, `workflowUrl`, `lifecycleState`, `actionNeeded`, `createdAt`, `updatedAt`.
Optional: `title` (workflow-defined case label, open prose), `continuationOfServicesActive` (governance §3.6), `continuationOfServicesEndsAt` (date-time-or-`never` sentinel; present only when `continuationOfServicesActive == true`).

### `ApplicantCaseDetail`

Full applicant-visible projection. Wraps `summary` (the same `ApplicantCaseSummary` shape returned by the list endpoint), `openTasks` (applicant-owed `pending`/`drafted` tasks), `recentNotifications` (most recent applicant notifications scoped to this case), `statusTimeline` (bounded applicant-visible event timeline), and `aiInvolvement` (EU AI Act Art. 13 / OMB M-24-10 disclosure summary).

Required: `summary`, `openTasks`, `recentNotifications`, `statusTimeline`.
Optional: `aiInvolvement` (`ApplicantAiInvolvementSummary` — present iff the governing workflow's `aiOversight.disclosure.discloseThatAgentAssisted` is true).

This shape is **distinct** from staff `CaseInstanceWithIncludes`: holds, delegations, review-state details, and other staff-only subresources are NOT exposed. The applicant view is intentionally narrower.

### `ApplicantAiInvolvementSummary`

Authority-bounded disclosure of AI involvement on the case (EU AI Act Art. 13 transparency obligation to the affected person; OMB M-24-10; ai-integration.md §12.1-§12.2 line 592-604: "For `rights-impacting` workflows, `discloseThatAgentAssisted` MUST be `true`"). Surfaces the FACT of AI involvement and the operational shape that bears on appeal rights — never internal model identifiers, autonomy enums, or proprietary configuration.

Required: `agentsInvolved: ApplicantAgentSummary[]`, `narrativeRecordCount: integer`, `humanReviewedAllAgentDecisions: boolean`.

`ApplicantAgentSummary` carries `displayName` (plain-language label, NOT model identifier), `roleInDecision` (closed-no-extension `advisory | primary | fallback`), and optional `confidence` (calibrated value in [0,1]). The disclosure is per-case-actual-involvement: an empty `agentsInvolved` is meaningful and means no agent participated despite the workflow declaring AI capability.

### `ApplicantTaskSummary`

Subset of staff `task.schema.json#/$defs/TaskListItem`. Only applicant-owed tasks appear here; the contract defends that scope by carrying a closed `kind` subset (`intake | correspondence-response | signature | verification`) covering applicant-actionable families. Staff-owned families (`review`, `determination`, `escalation`, `hold-resolution`) are filtered server-side and MUST NOT appear.

`status` mirrors `task.schema.json#/$defs/TaskStatus` (`pending | drafted | submitted | dismissed | expired`). The `pattern` branch is a forward-compatibility seam only — the staff `TaskStatus` itself is closed-no-extension.

### `ApplicantNotificationListItem`

Thin applicant-scoped projection of a notification feed item. The full staff `notification.schema.json#/$defs/Notification` shape carries fields (rich actions, governance source classifiers) that exceed applicant authority. This projection keeps identity, kind, status, headline, body, and case linkage. Authoring lives here (rather than `$ref`ing staff notification) because the applicant view is intentionally narrower with a smaller closed `kind` taxonomy.

`status` is `unread | read | archived` — closed-no-extension. Note this is a strict subset of `notification.schema.json#/$defs/NotificationStatus`: the staff `dismissed` status is not exposed because applicant notifications are never administratively dismissed, only read or archived by the applicant.

### `ApplicantStatusTimelineEntry`

Single timeline entry for `ApplicantCaseDetail.statusTimeline`. Each entry projects an applicant-visible governed event (case-created, lifecycle-changed, applicant-task assigned/submitted, decision-reached, correspondence sent/received). The timeline is NOT a cursor-paginated list — entry count is bounded per case and ships as a small array on the detail response.

## Closed Taxonomies

| Taxonomy | Source | Extension |
|---|---|---|
| `LifecycleState` | `$ref` to `instance.schema.json#/$defs/LifecycleState` (no inline) | closed-with-vendor-extension `^x-[a-z]+-` (mirrors kernel `wos-workflow.schema.json#/$defs/InstanceStatus`) |
| `ApplicantAgentSummary.roleInDecision` | new applicant-disclosure axis | closed-no-extension `advisory \| primary \| fallback` (Art. 13 disclosure cannot drift into vendor-specific labels) |
| `ApplicantTaskSummary.kind` | new applicant-actionable subset of `task.schema.json#/$defs/TaskKind` | closed-with-vendor-extension `^x-[a-z]+-` |
| `ApplicantTaskSummary.status` | mirror of `task.schema.json#/$defs/TaskStatus` | closed-with-vendor-extension forward-compat seam (staff side closed-no-extension) |
| `ApplicantNotificationListItem.kind` | new applicant-facing subset of staff notification families | closed-with-vendor-extension `^x-[a-z]+-` |
| `ApplicantNotificationListItem.status` | new — `unread \| read \| archived` (strict subset of staff `NotificationStatus` minus `dismissed`) | closed-no-extension |
| `ApplicantStatusTimelineEntry.event` | new applicant-visible event-class subset | closed-with-vendor-extension `^x-[a-z]+-` |

`ApplicantTaskSummary.kind` reserved literals: `intake | correspondence-response | signature | verification`. Staff-only families (`review`, `determination`, `escalation`, `hold-resolution`) are filtered server-side.

`ApplicantNotificationListItem.kind` reserved literals: `task-assigned | task-deadline-approaching | case-update | correspondence-received | decision-reached`. Staff-internal categories (`escalation`, `review-activated`, `hold-expired`) are filtered server-side.

`ApplicantStatusTimelineEntry.event` reserved literals: `case-created | lifecycle-changed | applicant-task-assigned | applicant-task-submitted | decision-reached | correspondence-sent | correspondence-received`. Staff-internal events (delegation, review-flagged, hold-engaged rationale) are filtered server-side.

## Endpoints

| Method | Path | Body in / out | Idempotency-Key |
|---|---|---|---|
| `GET` | `/api/v1/applicant/cases` | `ApplicantCaseListOptions` (query) -> `ApplicantCaseSummaryPage` | n/a |
| `GET` | `/api/v1/applicant/cases/{id}` | -> `ApplicantCaseDetail` | n/a |
| `GET` | `/api/v1/applicant/cases/{id}/tasks` | `PaginationQuery` (query) -> `ApplicantTaskPage` | n/a |
| `GET` | `/api/v1/applicant/notifications` | `ApplicantNotificationListOptions` (query) -> `ApplicantNotificationPage` | n/a |

Default scope is the **calling applicant's** cases and tasks — the server filters by the bearer principal's actor URN. Admins MAY NOT widen the applicant-scoped scope at these endpoints; staff visibility goes through the `instance.schema.json` and `task.schema.json` endpoints. This is a structural defense against accidental privilege-leak: a staff token presented at `/api/v1/applicant/...` returns the staff member's own applicant view (typically empty), not a tenant-wide projection.

`GET /api/v1/applicant/cases/{id}/tasks` is the **applicant-owed task subresource** per ADR 0082 D-3. Same `ApplicantTaskSummary` projection as the embedded `openTasks` array on `ApplicantCaseDetail`, but cursor-paginated for cases with many historical tasks.

`GET` endpoints are idempotent by construction — no `Idempotency-Key` header required (ADR 0082 D-16).

## Identifiers

- `ApplicantCaseSummary.id`, `ApplicantCaseDetail.summary.id`: `urn:wos:<typeid>` URN — the same case URN every other surface uses per ADR 0092 D-1. The applicant view does not mint applicant-specific URNs; it projects the canonical case identity.

- `ApplicantTaskSummary.id`: `urn:wos:<typeid>` URN. Tasks are sub-resources identified by URL path context.

- `ApplicantNotificationListItem.id`: `urn:wos:<typeid>` URN.

## Pagination

`GET /api/v1/applicant/cases`, `GET /api/v1/applicant/cases/{id}/tasks`, and `GET /api/v1/applicant/notifications` use cursor pagination per `api/pagination.schema.json`. Cursors are opaque, single-use within the issuing deploy. Cursor expiry returns `410 Gone` with `WOS-1410` (ADR 0082 D-7). No `total`, `page`, or `pageSize` echo.

The `ApplicantCaseDetail.openTasks`, `recentNotifications`, and `statusTimeline` arrays are NOT paginated — they are bounded by server-side cap (typically 20 items each). Clients fetch the full list via the dedicated subresource endpoints when the case has more.

## Errors

All non-2xx responses use `application/problem+json` per ADR 0082 D-8 and `api/error.schema.json`. Domain-relevant codes from the registry:

- `WOS-1403`: applicant scope filter rejected — for example a staff principal calling `/api/v1/applicant/cases` for another applicant's URN.
- `WOS-1404`: case URN does not exist or is not visible to the calling applicant.
- `WOS-1410`: cursor expired.
- `WOS-1422`: request failed schema validation.
- `WOS-1503`: durable runtime backend unavailable.

## Greenfield Discipline

Per ADR 0082 Context section and the owner's greenfield-contracts memory: prior `case-portal` `ApplicantView` projections are NOT preserved. The schema makes the worst shapes prior art tends to accumulate structurally inexpressible:

1. **No mixed staff/applicant task lists.** `ApplicantTaskSummary.kind` is a closed subset that excludes `review | determination | escalation | hold-resolution`; the wire shape forbids serializing a staff-owned task into an applicant feed.
2. **No governance-internals leak.** `ApplicantCaseDetail` does not carry `delegations`, `reviewState`, or hold-rationale fields; the applicant view simply lacks the seams those fields would need.
3. **No nested `applicantCase: { instance: { ... }, tasks: [...] }` re-flattening.** The detail shape is `summary | openTasks | recentNotifications | statusTimeline` — a flat envelope that names the four subprojections explicitly. No "everything-in-one-blob" denormalization that scales poorly per ADR 0082 D-3.
4. **Closed `status` taxonomies on the applicant inbox.** Applicant `NotificationStatus` is `unread | read | archived` (closed-no-extension) — the staff `dismissed` literal does not leak into applicant authority.

## Non-Goals

- **Applicant-side write endpoints.** Tasks are submitted via the canonical `task.schema.json` endpoints (`POST /api/v1/tasks/{id}/draft`, `/response`, `/dismiss`); notifications are marked-read via the staff `notification.schema.json` endpoint when the applicant has a session token. The applicant API surface here is read-only — there is no separate applicant-side write API.
- **Applicant-to-applicant case sharing.** Cases belong to one applicant principal (or their authorized agents per VISION §V); cross-applicant case discovery is not part of the public API.
- **Applicant document upload.** File upload is a future ADR slot per ADR 0082 D-16 (likely a presigned-URL pattern). The applicant API shows attachment metadata via `correspondence.schema.json` projections, not via this surface.
- **Cross-tenant applicant views.** ADR 0068 D-1 makes cross-tenant reads impossible by construction; the applicant surface scopes by tenant only.
- **Streaming applicant feeds.** Out of scope per ADR 0082 D-16. Clients refresh by re-fetching.

## ADR Amendments

None required. The applicant surface closes ADR 0082 D-15 step 6 alongside the parallel dashboard / auth / bundle / audit agents. The "narrower-projection-with-defended-authority-boundary" pattern (closed `kind` subset + closed `status` subset) is a precedent for any future role-narrowed projection.
