# WOS Public API Actor

**Status:** Draft (ADR 0082 D-15 step 5 prerequisite — actor resource shape settles before governance domain)
**Schema:** [`api/actor.schema.json`](../../schemas/api/actor.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/actor/v1`
**Common definitions:** [`api/_common.schema.json`](../../schemas/api/_common.schema.json) (`https://schemas.formspec.io/wos-api/_common/v1`) — canonical `ActorRef` and `WosResourceUrn` home (ADR 0082 D-4, D-9, D-14).
**Authority:** [ADR 0082 — Stack Public REST API Contract and Schema Discipline](../../../thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md) (D-4 URN identifiers; D-9 `ActorRef` URN replacing nested `actor: {id,type,name}`; D-12 closed taxonomies; D-14 no redefining)
**Gating ADRs / plans:** [ADR 0068 — Stack Tenant and Scope Composition](../../../thoughts/adr/0068-stack-tenant-and-scope-composition.md) (Proposed — scope shape); PLN-0381 (stack identity-attestation ADR, TBD number — actor-attestation is out of scope here); PLN-0404 (closes when this lands).

## Purpose

An Actor is the addressable representation of a principal in the WOS identity / governance subsystem. The four principal classes are normative and closed (VISION §V): `human | service-account | workload | support`. Identity details live here once and every other public API resource (notifications, correspondence, provenance, governance, reports) references actors via the `ActorRef` URN string from [`api/_common.schema.json`](../../schemas/api/_common.schema.json) — never by inlining nested `{id, type, name}` shapes (ADR 0082 D-9, D-14).

This spec authors the actor *resource* (greenfield), not just the reference. Consumers that hold an `ActorRef` URN dereference it through this resource; consumers that already hold an `Actor` resource use the `actorRef` field as the canonical reference shape. The two forms agree by construction: the principal-class segment of the URN equals `principalClass`.

Authentication credentials, key material, attestation chain, and session lifecycle are NOT exposed here — actor-attestation is tracked separately under PLN-0381 (stack identity-attestation ADR, TBD number) and remains out of scope for this domain.

## Resource Shape

`Actor` requires:

- `id` — actor URN (entity-type segment `actor`; ADR 0082 D-4).
- `principalClass` — closed VISION §V taxonomy `human | service-account | workload | support`. Equals the URN's principal-class segment.
- `displayName` — open prose suitable for UI, audit timelines, and correspondence summaries (`x-wos.openStringKind: prose`).
- `status` — closed `ActorStatus` lifecycle: `active | suspended | retired`.
- `createdAt` — UTC RFC 3339 (ADR 0082 D-10).

Optional fields:

- `scope` — closed `ActorScope` object carrying `tenant`, `organization`, `workspace?`, `environment` per VISION §V scope hierarchy. Shape gates on ADR 0068 D-2 (Proposed); consumers tolerate the field's absence on early-deployment servers.
- `actorRef` — URN-form alias derived from this resource. Present so consumers that round-trip the resource have the canonical reference shape on hand without re-deriving it.
- `retiredAt` — UTC RFC 3339; present iff `status` is `retired`.

## Identifier Scheme

URNs follow ADR 0082 D-4: `urn:wos:<entity-type>:<workflow-or-scope-id>:<date>:<short-hash>`. This domain uses the `actor` entity-type literal which is already part of the closed taxonomy ratified at ADR 0082 D-4 / `_common.schema.json#/$defs/WosResourceUrn`.

`ActorRef` (the URN-form principal reference at `_common.schema.json#/$defs/ActorRef`) is a distinct shape: `actor:<principalClass>:<id-suffix>`. The two forms agree on the principal-class segment but live at different URLs because they are semantically distinct — `WosResourceUrn` addresses a public WOS resource record; `ActorRef` is the structured reference shape every other resource uses to point at an actor.

## Endpoints

### Read one actor

`GET /api/v1/actors/{urn}`

Returns the `Actor`. The `{urn}` path segment is a URL-encoded actor URN. Responds `WOS-1404` when the URN is not visible in the caller's scope.

### List actors

`GET /api/v1/actors`

Query fields per `ActorListOptions`: `principalClass`, `status`, `cursor`, `limit`. Returns `ActorPage` (`items`, optional `cursor`, `hasMore`). Cursor pagination per ADR 0082 D-7. Default ordering is `createdAt` ascending so paginating clients observe a stable monotone walk.

## Request and Response Discipline

All endpoints use `application/json` request and response bodies; errors use `application/problem+json` per ADR 0082 D-8. Tenant and scope headers per ADR 0082 D-9 (`X-WOS-Tenant`, `X-WOS-Organization`, `X-WOS-Workspace`, `X-WOS-Environment`) are required on every endpoint in this domain.

The actor surface is read-only at the public API layer; actor creation, suspension, and retirement happen through the identity/governance subsystem and are out of public-API scope. Mutating operations on actors will land under that subsystem's spec, not under `specs/api/actor.md`.

## Closed Taxonomies

This domain introduces or projects the following closed taxonomies (ADR 0082 D-12):

| Taxonomy | Source | Extension |
|---|---|---|
| `PrincipalClass` | new — `human \| service-account \| workload \| support` (VISION §V) | none — closed; new classes require a major schema bump |
| `ActorStatus` | new — `active \| suspended \| retired` | none — closed |
| `ActorScope.environment` | new — `sandbox \| staging \| prod` (VISION §V) | none — closed |

`ActorScope` itself is a closed object shape gating on ADR 0068 D-2 (Proposed). When ADR 0068 promotes to Accepted, this object becomes the wire form scope is reported in; until then, server implementations MAY return only the fields they have settled, and consumers MUST tolerate the field's absence.

## Schema Cross-References

Per ADR 0082 D-14, the schema `$ref`s the canonical definitions in `_common.schema.json` instead of redefining:

- `https://schemas.formspec.io/wos-api/_common/v1#/$defs/ActorRef`
- `https://schemas.formspec.io/wos-api/_common/v1#/$defs/WosResourceUrn`

The API layer is a projection, not an alternative reality (ADR 0082 D-14).

## Non-Goals

- Actor-attestation (identity proofing, attestation chain, claim graph) — covered by PLN-0381 (stack identity-attestation ADR, TBD number). The `Actor` resource here carries display-state only; the attestation surface lives elsewhere.
- Authentication credentials, key material, session tokens — not exposed at the public API layer.
- Actor creation, suspension, retirement (mutating operations) — owned by the identity/governance subsystem, out of public-API scope until that subsystem's spec lands.
- Cross-tenant actor discovery — actors are scope-bound per VISION §V; cross-tenant linking is not part of the public API contract.
- Role assignment / permission grants — RBAC ladder (`Owner / Admin / Author / Reviewer / Analyst / Submitter`) and OpenFGA tuples are governance-domain concerns (ADR 0082 D-15 step 5 governance schema), not actor-domain.
