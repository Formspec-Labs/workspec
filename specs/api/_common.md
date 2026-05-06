# WOS Public API Common Definitions

**Status:** Implemented (ADR 0082 D-15 cross-cutting)
**Schema:** [`api/_common.schema.json`](../../schemas/api/_common.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/_common/v1`
**Authority:** [ADR 0082 — Stack Public REST API Contract and Schema Discipline](../../../thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md) (D-4 URN identifiers; D-9 `ActorRef` URN replacing nested `actor: {id,type,name}`; D-12 closed taxonomies; D-14 no redefining)

## Purpose

Canonical definitions hoisted out of every per-resource api schema so adding a new entity-type literal (D-4) or principal class (D-9) touches one file, not the entire `api/*.schema.json` family. Closes PLN-0404 and prevents the systemic-drift class that hit provenance's `RuleReference` mirror in the 2026-05-05 review.

This schema is a definitions-only module: it has no resource shape, no top-level `oneOf`, and no endpoints. The leading underscore (`_common.schema.json`) is a vendor-convention marker that distinguishes module-scoped helpers from resource schemas.

## Hoisted Definitions

### `ActorRef`

URN-form principal reference: `actor:<principalClass>:<id-suffix>`. Principal-class segment is the closed VISION §V taxonomy `human | service-account | workload | support`. Per ADR 0082 D-9, identity details live once in the identity/governance subsystem and every other resource references actors by URN. Replaces the legacy nested `actor: {id, type, name}` shape from prior portal art.

### `WosResourceUrn`

Public WOS API resource URN: `urn:wos:<entity-type>:<workflow-or-scope-id>:<date>:<short-hash>`. Entity-type segment is the closed taxonomy from ADR 0082 D-4: `instance | task | bundle | delegation | agent | hold | timer | profile | notification | correspondence-message | report-run | provenance-record | actor`. Vendor extensions are NOT permitted in the entity-type segment — public WOS resource families are normative.

## Cross-Reference Discipline

Every other `api/*.schema.json` referencing these definitions MUST `$ref` the absolute `$id` form per ADR 0082 D-14:

- `https://schemas.formspec.io/wos-api/_common/v1#/$defs/ActorRef`
- `https://schemas.formspec.io/wos-api/_common/v1#/$defs/WosResourceUrn`

Inline redefinition of either shape is a contract bug — the fix is a single edit in this file plus an ADR amendment when the underlying taxonomy changes.

## Adding a New Entity-Type Literal

1. Open an ADR amendment to ADR 0082 D-4 declaring the new entity-type literal and its semantics.
2. Add the literal to the alternation in `_common.schema.json#/$defs/WosResourceUrn`.
3. Add an example URN under `examples` for the new family.
4. Land the per-domain `api/<resource>.schema.json` and `specs/api/<resource>.md`.

No other api schema file requires editing for the URN change — the `$ref` chain resolves automatically.

## Adding a New Principal Class

1. Open an ADR amendment to ADR 0082 D-9 (and VISION §V) declaring the new class and its semantics.
2. Add the literal to the alternation in `_common.schema.json#/$defs/ActorRef` AND to `actor.schema.json#/$defs/PrincipalClass`.
3. Add an example URN under `examples`.
4. Update [`specs/api/actor.md`](./actor.md) closed-taxonomy table.

The two-edit cost (`ActorRef` regex + `PrincipalClass` enum) is structural — the regex carries the wire shape; the enum carries the closed taxonomy on the `Actor` resource. Both are normative.

## Non-Goals

- Resource shapes — resource schemas live one-per-file under `api/*.schema.json`.
- Cross-schema `$ref` resolution tooling (typify, json-schema-to-typescript) — handled by the existing pipelines per ADR 0082 D-13.
- Principal-class semantics, scope hierarchy, RBAC ladder — VISION §V and the actor/governance domain specs.
