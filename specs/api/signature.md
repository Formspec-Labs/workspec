# WOS Public API Signature Ceremony

**Status:** Draft
**Schema:** [`api/signature.schema.json`](../../schemas/api/signature.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/signature/v1`
**ADR anchor:** [ADR 0082](../../../thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md) D-4 (URN), D-7 (cursor pagination), D-10 (`never` sentinel), D-12 (closed taxonomies), D-15 step 6 (signature/task pair closes the signature surface).
**Gating ADR:** Signature profile §2.2 (signer-role taxonomy), §2.3 (flow patterns), §2.11 (signing-intent registry).

## Purpose

A `SignatureCeremony` is the aggregate projection of a WOS signing ceremony — the FSM state that tracks which signers have signed, declined, or expired across a document or transition. Distinct from:
- **`signature` tasks** (`task.schema.json` `TaskKind.signature`) — the per-signer task lifecycle
- **`signatureAffirmation` provenance records** (`provenance.schema.json` `FactsRecordKind.signatureAffirmation`) — the per-signature audit evidence
- The **signature profile** author-time declaration (`wos-workflow.schema.json` `signature` embedded block) — the ceremony configuration

The ceremony aggregate lets the case portal answer "who hasn't signed yet?" and lets an operator inspect the full ceremony FSM without reconstructing from per-task and per-provenance records.

## Resource Shape

`SignatureCeremony` carries identity, case binding, optional task binding, status, ordered signers, flow pattern, timing metadata, and signing intent. Required fields: `id`, `processId`, `status`, `signers`, `flowPattern`, `createdAt`. Optional fields, omitted when absent: `taskId`, `documentRef`, `intentUri`, `completedAt`, `expiresAt`.

`status` is a closed lifecycle: `awaiting-signatures` (ceremony active, signers pending), `complete` (all required signers have signed), `declined` (a signer declined and the ceremony reached a terminal state), `expired` (signing window elapsed), `voided` (ceremony was administratively voided). Closed enum with no extension seam — ceremony lifecycle is normative.

`signers` is an ordered array of `SignerState` objects. Order reflects the flow pattern: `sequential` = order matters (each signer gates the next), `parallel` = order is informational only.

`SignerState` tracks the per-signer lifecycle: `actorRef` (URN-form signer reference), `role` (closed `SignerRole` taxonomy), `status` (closed `SignerStatus` per-signer lifecycle: `pending`, `notified`, `viewed`, `signed`, `declined`, `expired`). Optional timestamp fields `signedAt` and `declinedAt` capture when the signer reached those terminal states; `declineReason` carries free-text explanation.

`flowPattern` is a closed taxonomy: `sequential`, `parallel`, `routed`, `free-for-all`. No vendor extension — signing flow is normative.

`expiresAt` uses the ADR 0082 D-10 `never` sentinel for ceremonies without an expiry window.

## Identifier Scheme

`SignatureCeremony.id` is a `urn:wos:<typeid>` URN per ADR 0092 D-1. `SignatureCeremony.processId` is a `urn:wos:<typeid>` URN of the owning case.

## Endpoints

| Method | Path | Body | Idempotency-Key |
|--------|------|------|-----------------|
| GET | /api/v1/instances/{id}/signatures | → SignatureCeremonyPage | n/a |
| GET | /api/v1/instances/{id}/signatures/{ceremonyId} | → SignatureCeremony | n/a |

`GET /api/v1/instances/{id}/signatures` lists ceremonies for a workflow process. Response is `SignatureCeremonyPage`: `items: SignatureCeremony[]`, optional `cursor`, required `hasMore`. Cursor-paginated per ADR 0082 D-7. No `total`, no `page`. Cursors are deploy-lifetime stable; `WOS-1410` on expiry triggers client restart from the top.

`GET /api/v1/instances/{id}/signatures/{ceremonyId}` returns a single `SignatureCeremony`. The `ceremonyId` path parameter is a signature-ceremony URN; `404` (`WOS-1404`) when the ceremony is not visible to the caller's scope.

## Pagination

`GET /api/v1/instances/{id}/signatures` uses cursor pagination per `api/pagination.schema.json`. Cursors are opaque, single-use within the issuing deploy. Cursor expiry returns `410 Gone` with `WOS-1410`.

## Errors

All non-2xx responses use `application/problem+json` per ADR 0082 D-8 and `api/error.schema.json`. Domain-relevant codes:

- `WOS-1404`: signature ceremony URN does not exist or is not in the caller's scope.
- `WOS-1410`: cursor expired.

## Closed Taxonomies

This domain introduces the following closed taxonomies (ADR 0082 D-12). No vendor extension seams.

| Taxonomy | Reserved literals |
|---|---|
| `SignerRole` | `signer \| witness \| notary \| counter-signer \| approver \| reviewer \| observer \| custodian` |
| `SignerStatus` | `pending \| notified \| viewed \| signed \| declined \| expired` |
| `SignatureFlowPattern` | `sequential \| parallel \| routed \| free-for-all` |
| `SignatureCeremonyStatus` | `awaiting-signatures \| complete \| declined \| expired \| voided` |

## Non-Goals

- **Signature ceremony creation.** Ceremonies are created by the runtime when a signature-gated transition fires. There is no `POST` endpoint for ceremony creation.
- **Signing action submission.** Signing is performed through signature tasks (`task.schema.json`), not through the ceremony resource. The ceremony is a read-only aggregate projection.
- **Signature profile authoring.** The signature profile is declared at author-time in the workflow document (`wos-workflow.schema.json` `signature` embedded block). This API surface only projects the runtime ceremony state.
