# WOS Public API Correspondence

**Status:** Draft (ADR 0082 D-15 step 2)
**Schema:** [`api/correspondence.schema.json`](../../schemas/api/correspondence.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/correspondence/v1`
**Authority:** [ADR 0082 — Stack Public REST API Contract and Schema Discipline](../../../thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md)
**Projects from:** [`schemas/sidecars/wos-delivery.schema.json`](../../schemas/sidecars/wos-delivery.schema.json) (`https://wos-spec.org/schemas/delivery/1.0`); prose at [`specs/sidecars/delivery.md`](../sidecars/delivery.md) §4 (correspondence) and §3 (notifications).

## Purpose

Correspondence messages are addressable inbound and outbound communications associated with a WOS workflow process: physical mail, phone calls, emails, portal submissions, fax, in-person interactions, and the agency notices the workflow renders and sends back. The API surface projects the WOS delivery sidecar's correspondence and notification-template vocabularies into a public REST shape; it does not redefine those vocabularies. Payload bytes are never embedded — `contentRef` is a claim-check pointer.

This spec also covers the rendering subresource that turns a delivery-sidecar notification template into a draft correspondence body. Rendering is delivery preparation: the legacy `POST /api/v1/notifications/{workflowUrl}/render` route folds into this domain because it produces a correspondence artifact, not a notification-feed entry.

## Resource Shape

`CorrespondenceMessage` is the addressable resource. Required fields:

- `id` — correspondence-message URN (ADR 0082 D-4).
- `processId` — workflow process URN this message belongs to.
- `workflowUrl` — workflow URI used to resolve the delivery sidecar.
- `templateRef` — delivery-sidecar entry-template id (inbound) or notification-template key (outbound).
- `channel` — communication channel (closed projection of delivery `EntryTemplate.channel`, vendor-extensible).
- `direction` — `inbound` or `outbound` (`$ref` to delivery `EntryTemplate.direction`).
- `partyRole` — sender or recipient role (`$ref` to delivery `EntryTemplate.correspondenceRole`).
- `status` — closed `MessageStatus` lifecycle: `draft | queued | sent | delivered | failed | received | acknowledged | cancelled`. No vendor extension; new states require a major schema bump.
- `producer` — closed `MessageProducer` family with vendor extensions.
- `summary` — required one-line case-timeline summary.
- `occurredAt`, `recordedAt` — UTC RFC 3339 (ADR 0082 D-10).

Optional fields are omitted when absent: `subject`, `contentRef`, `renderingId`, `deliveryChannels`, `relatedTaskId`, `relatedNotificationId`, `actorRef`, `expiresAt` (RFC 3339 or the literal `never`).

`CorrespondenceRendering` is the auxiliary resource produced by the rendering subresource. It carries `id`, `workflowUrl`, `templateRef`, `category` (`$ref` to delivery `NotificationTemplate.category`), ordered `sections[]` with `contentType` (`$ref` to delivery `TemplateSection.contentType`), `deliveryChannels[]`, the concatenated `rendered` body, and the sorted `resolvedVariables` and `missingVariables` name lists. Only variable names cross the wire — never values — to prevent rendering responses from leaking case-state shapes back to clients.

## Identifier Scheme

URNs follow ADR 0082 D-4: `urn:wos:<entity-type>:<workflow-or-scope-id>:<date>:<short-hash>`. This domain uses the existing `correspondence-message` entity-type literal. The rendering subresource also addresses URNs of the same `correspondence-message` family (renderings are draft messages); a separate entity-type is not introduced.

## Endpoints

### List correspondence

`GET /api/v1/correspondence`

Query fields per `CorrespondenceListOptions`: `processId`, `direction`, `status`, `channel`, `partyRole`, `occurredFrom`, `occurredUntil`, `cursor`, `limit`. Returns `CorrespondenceMessagePage` (`items`, optional `cursor`, `hasMore`). Cursor pagination per ADR 0082 D-7. Default ordering is `occurredAt` descending.

### Read one correspondence message

`GET /api/v1/correspondence/{id}`

Returns the `CorrespondenceMessage`. Responds `WOS-1404` when the URN is not visible in the caller's scope.

### Instance-scoped correspondence subresource

`GET /api/v1/instances/{processId}/correspondence`

Per ADR 0082 D-3 (composition over flattening). Same `CorrespondenceListOptions` query fields except `processId` is implicit from the path. Same `CorrespondenceMessagePage` envelope. The instance aggregation endpoint at `GET /api/v1/instances/{id}?include=correspondence` extends the closed `?include=` enum (D-3) with the literal `correspondence`; consumers that want a count alongside the embedded slice request `?include=correspondence(limit=10)` per the existing aggregation grammar.

### Case/process-scoped correspondence subresource

`GET /api/v1/cases/{case_id}/processes/{process_id}/correspondence`

Case/process bridge for ADR 0093 dual identity. Same `CorrespondenceListOptions` query fields except `processId` is implicit from the bound process path. The server validates that `{process_id}` belongs to `{case_id}` before listing the same `CorrespondenceMessagePage`; mismatch returns `WOS-1404`/404 and does not fall back to the process alone.

### Log a correspondence message

`POST /api/v1/correspondence`

Body: `LogCorrespondenceMessageRequest`. The server assigns the URN, stamps `recordedAt`, and resolves the message status from `direction` and `producer` defaults (`received` for inbound, `sent` for outbound when the request supplies a `contentRef` and `renderingId`, otherwise `draft`). `Idempotency-Key` is REQUIRED per ADR 0082 D-16 because logging produces externally-visible audit state. Returns `201 Created` with the full `CorrespondenceMessage` and a `Location` header pointing at `GET /api/v1/correspondence/{id}`.

### Render a delivery template

`POST /api/v1/correspondence/renderings`

Body: `RenderCorrespondenceRequest`. Returns `CorrespondenceRendering`. This route subsumes the legacy `POST /api/v1/notifications/{workflowUrl}/render` route. Rendering does not by itself produce a correspondence message; callers commit a rendering to an outbound message by submitting a follow-up `POST /api/v1/correspondence` with `renderingId` set. When `missingVariables` is non-empty the rendering is advisory only — the commit endpoint MUST refuse it with `WOS-1422`.

The `workflowUrl` and `templateRef` move from URL path to request body so the rendering URI is stable as templates evolve and so the route does not need to URL-encode workflow URIs.

### Read a rendering

`GET /api/v1/correspondence/renderings/{id}`

Returns the previously-rendered `CorrespondenceRendering`. Renderings are immutable.

### Read a business-calendar date range

`GET /api/v1/correspondence/calendar/{calendarId}/dates?from={YYYY-MM-DD}&to={YYYY-MM-DD}`

Returns `BusinessCalendarDateList` (cursor envelope) projecting the delivery sidecar's calendar block (specs/sidecars/business-calendar.md §business-calendar) into a typed read-only date series. Each `BusinessCalendarDate` carries `date`, closed-no-extension `dayKind: business | holiday | weekend`, optional `holidayName: prose`, optional `observed` boolean for §4.3 observed-date entries. Page envelope carries `calendarId` and `timezone` so consumers do not need to re-resolve the calendar block to interpret day boundaries.

Lookup-side projection only — calendar authoring lives on the workflow's delivery sidecar, not the public API. Lets a portal display "is today a business day?", "what's the next business day?", or "show me all federal holidays in Q3" without re-implementing the workWeek + holidays composition logic. Multi-calendar selection (specs/sidecars/business-calendar.md §7.1) is performed server-side; this endpoint returns the resolved series for a single calendar identifier.

`from` and `to` are required ISO 8601 dates; `to` MUST be ≥ `from`. Server enforces a maximum range (typical: one year per request; longer ranges page via `cursor`). Cursor pagination per ADR 0082 D-7.

## Request and Response Discipline

All endpoints use `application/json` request and response bodies; errors use `application/problem+json` per ADR 0082 D-8.

Idempotency: `POST /api/v1/correspondence` REQUIRES `Idempotency-Key`. `POST /api/v1/correspondence/renderings` accepts `Idempotency-Key` and a repeat request returns the original rendering URN within the 24-hour retention window.

Tenant and scope headers per ADR 0082 D-9 (`X-WOS-Tenant`, `X-WOS-Organization`, `X-WOS-Workspace`, `X-WOS-Environment`) are required on every endpoint in this domain.

## Error Codes

Public WOS error codes used by this domain (registry: [`error-registry.md`](./error-registry.md)):

- `WOS-1400` — request body fails JSON Schema validation against `LogCorrespondenceMessageRequest` or `RenderCorrespondenceRequest`.
- `WOS-1404` — correspondence URN, rendering URN, instance URN, or workflow URI is not visible in the caller's scope.
- `WOS-1409` — `Idempotency-Key` conflict (a different request body was previously seen for the same key, route, and scope).
- `WOS-1410` — list cursor expired (ADR 0082 D-7); restart pagination.
- `WOS-1422` — semantic validation failure: `templateRef` does not resolve in the workflow's delivery sidecar; `direction` mismatches the resolved entry template; `LogCorrespondenceMessageRequest.renderingId` references a rendering with non-empty `missingVariables`; an `adverse-decision` rendering omits a due-process section per delivery sidecar §3.2.
- `WOS-1503` — bundle service is unavailable and the delivery sidecar cannot be resolved.

`Problem.context` carries domain-specific diagnostic fields: `templateRef`, `workflowUrl`, `renderingId`, `missingVariables`, `unsupportedDirection`. `context` is the only anonymous-object extension surface in this domain (per ADR 0082 D-12).

## Pagination Posture

Cursor-based per ADR 0082 D-7 with the `CorrespondenceMessagePage` envelope. No `total`, `page`, or `pageSize` echoes. Counts, when needed by an admin dashboard, are exposed at a separate cached subresource (out of scope for this spec; declared by the dashboard domain when authored).

## Aggregation Include

The workflow-process aggregation endpoint extends its closed `?include=` enum (ADR 0082 D-3) with the literal `correspondence`. The embedded slice carries up to the negotiated `correspondence(limit=N)` cap (server enforces a maximum); clients that need the full feed page through `GET /api/v1/instances/{processId}/correspondence`.

## Closed Taxonomies

This domain introduces or projects the following closed taxonomies (ADR 0082 D-12):

| Taxonomy | Source | Extension |
|---|---|---|
| `MessageDirection` | `$ref` delivery sidecar `EntryTemplate.direction` | none — closed |
| `MessageChannel` | `$ref` delivery sidecar `EntryTemplate.channel` | `^x-[a-z]+-` |
| `CorrespondencePartyRole` | `$ref` delivery sidecar `EntryTemplate.correspondenceRole` | none — closed |
| `MessageStatus` | new — `draft | queued | sent | delivered | failed | received | acknowledged | cancelled` | none — major bump for new states |
| `MessageProducer` | new — `agency | system | workflow | applicant-portal | third-party-intake` | `^x-[a-z]+-` |
| `RenderingCategory` | `$ref` delivery sidecar `NotificationTemplate.category` | none — closed |
| `DeliveryChannelKind` | new — `postal | email | portal | sms | in-app` (mirrors delivery `deliveryChannels`) | `^x-[a-z]+-` |
| `BusinessCalendarDate.dayKind` | new — `business | holiday | weekend` (specs/sidecars/business-calendar.md §3-§4) | none — major bump |

`RenderingVariableValue` is a closed `oneOf` over `string | number | integer | boolean | null | array<string>` — nested objects are intentionally not expressible. This eliminates the `Record<string, unknown>` escape hatch the prior `RenderRequest.context` shape carried in the workspec-server prior art.

## Schema Cross-References

Per ADR 0082 D-14, the schema `$ref`s the delivery sidecar instead of redefining the closed taxonomies it shares:

- `https://wos-spec.org/schemas/delivery/1.0#/$defs/EntryTemplate/properties/channel`
- `https://wos-spec.org/schemas/delivery/1.0#/$defs/EntryTemplate/properties/direction`
- `https://wos-spec.org/schemas/delivery/1.0#/$defs/EntryTemplate/properties/correspondenceRole`
- `https://wos-spec.org/schemas/delivery/1.0#/$defs/NotificationTemplate/properties/category`
- `https://wos-spec.org/schemas/delivery/1.0#/$defs/TemplateSection/properties/contentType`

The API layer is a projection, not an alternative reality (ADR 0082 D-14).

## Non-Goals

- Outbound delivery transport (postal vendor handoff, SMTP, SMS gateways) — implementation concern; the API contract ends at `status: queued`.
- Notice rendering for human-readable PDF/HTML output — Trellis renderer concern (TODO-STACK item).
- Provenance records for correspondence events — covered by the `api/provenance.schema.json` schema (ADR 0082 D-15 step 3).
- Notification feed projection of correspondence events — covered by `api/notification.schema.json`.
- Authoring delivery sidecars or notification templates — Studio (Authoring) concern; out of public-API scope.
