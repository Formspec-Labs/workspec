# WOS Public API Notification

**Status:** Implemented
**Schema:** [`api/notification.schema.json`](../../schemas/api/notification.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/notification/v1`
**OpenAPI:** [`wos-public-api.openapi.json`](../../api/wos-public-api.openapi.json)

## Purpose

Notifications are user-scoped read-model entries derived from governed workflow events. They are not authoritative audit records and do not replace provenance, correspondence, or delivery-template records.

## Resource Shape

`Notification` uses stable WOS resource URNs and closed public vocabularies with vendor-extension seams.

Required fields:

- `id`: notification URN.
- `type`: notification event family.
- `severity`: `info`, `success`, `warning`, or `critical`.
- `status`: `unread`, `read`, `archived`, or `dismissed`.
- `source`: producer family.
- `title`: short headline.
- `body`: display body.
- `createdAt`: UTC RFC 3339 creation timestamp.

Optional fields are omitted when absent:

- `readAt`
- `expiresAt`, including the literal `never` for non-expiring notifications.
- `actorRef`
- `processId`
- `taskId`
- `bundleId` — REQUIRED on the retained `bundle-completed` arm; see its current emission status below. The closed discriminated union preserves this conditional requirement (ADR 0082 D-11).
- `action`

`action.kind` is a closed action family with `x-<vendor>-...` extension support. Resource-opening actions use `resourceId`; external-link actions use `href`.

## Bundle-completion seam

The schema retains the `bundle-completed` arm and its required `bundleId`, but the current server does not emit this arm. It names a retired case-export resource, not the loaded-workflow catalog returned by `GET /api/v1/bundles`. Current case exports use completion exports (`completion-exports.schema.json`, ADR 0164); this specification does not define a new completion notification producer or download route.

## NotificationType Taxonomy

The `type` field is a closed-with-vendor-extension family; vendor extensions use the `^x-[a-z]+-` pattern.

| Literal | Semantics | Recommended severity |
|---|---|---|
| `bundle-completed` | Retained arm for the retired case-export resource; not emitted by the current server. | Not currently emitted. |
| `adverse-decision` | A governed determination was reached that denies, reduces, or otherwise adversely affects a claimed benefit, right, or access — triggers procedural-review and appeal-path awareness. Emitted under governance §S10.3 escalation when an adverse outcome renders. | `critical` |
| `appeal-filed` | An interested party has initiated a formal appeal against a prior adverse determination. Emitted to the adjudication authority and any registered observers; carries `processId` of the appeal case and `actorRef` of the appellant. | `critical` |

`adverse-decision` enables case-portal UI to surface the "your application was denied / your benefit was reduced" feed item with deep-link to the determination and appeal instructions. `appeal-filed` enables the original adjudicator and oversight dashboard to surface pending-appeal state inline.

## Endpoints

The public notification feed endpoints are:

- `GET /api/v1/notifications`
- `GET /api/v1/notifications/unread-count`
- `POST /api/v1/notifications/{id}/read`

`GET /api/v1/notifications` accepts `NotificationFeedOptions`: `includeRead`, `cursor`, and `limit`. It returns `NotificationPage`, which uses `items`, optional `cursor`, and `hasMore`.

`GET /api/v1/notifications/unread-count` returns `UnreadCountResponse` with `count` and `asOf`.

`POST /api/v1/notifications/{id}/read` accepts optional `MarkNotificationReadRequest.readAt`. The server may replace client time with server receipt time. Mark-read is idempotent for the visible read model. `Idempotency-Key` REQUIRED per ADR 0082 D-16 (the status flip is an externally-visible side effect — it changes the unread count and the per-user feed state).

## Non-Goals

The notification feed does not define outbound correspondence, delivery-template rendering, durable audit evidence, or cross-case reporting. Those domains get separate `schemas/api/*.schema.json` files and `specs/api/*.md` specs.
