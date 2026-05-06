# ADR 0092: WOS API — TypeID as URN Namespace-Specific String

**Status:** Proposed  
**Date:** 2026-05-06  
**Scope:** WOS — public REST API identity format (`work-spec/schemas/api/`, `workspec-server`)  
**Amends:** ADR 0082 D-4 (URN segment structure) and D-14 (URN ref discipline) per [stack ADR 0082](../../../thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md).  
**Related:** ADR 0061 (custody-hook TypeID adoption); `wos-core/src/typeid.rs` (minting); kernel `custody-hook-encoding.md` §1.4 (TypeID rules); ADR 0068 D-1.1 (tenant grammar); [`thoughts/specs/2026-05-06-api-typeid-identity.md`](../specs/2026-05-06-api-typeid-identity.md) (implementation spec).

---

## 1. Context

Two identity formats coexist in the stack:

| Layer | Format | Defined by |
|---|---|---|
| Public REST API | `urn:wos:<entity-type>:<scope>:<date>:<hash>` (5-segment URN) | ADR 0082 D-4 |
| DB, durable execution, Trellis custody hook | `{tenant}_{type}_{uuidv7_base32}` (TypeID) | ADR 0061 |

ADR 0061 states TypeID should be "one identity, zero mapping" — the same identifier across WOS case id, durable-execution workflow id, and provenance-store primary key. ADR 0082 D-4 explicitly rejects opaque UUIDs at the API surface and requires a URN wire format with human-readable `<scope>`, `<date>`, and `<hash>` segments.

The server bridges them with a heuristic: `to_instance_urn()` (`instance_service.rs:113-128`) tries to parse the stored `instance_id` as a URN, and on failure extracts the TypeID tenant, synthesizes a scope and date, and stuffs the raw TypeID into the `<hash>` segment. This is not a design — it is a fallback that leaks the TypeID through the URN shape by accident.

The greenfield posture matters: no production API consumers exist. The current `workspec-server` HTTP surface is pre-stable development scaffolding. This is the moment to resolve the tension before it becomes a migration problem.

### Load-bearing questions resolved before acceptance

1. **Does the URN envelope carry its weight?** Yes. The `urn:wos:` namespace disambiguates WOS resources in multi-system environments (vs. Formspec responses, Trellis bundles). The cost is 8 bytes and a trivial prefix wrap/strip.
2. **Why not drop the URN entirely and go bare TypeID?** Bare TypeIDs are ambiguous across systems. A `case` TypeID vs. a Formspec response TypeID vs. a Trellis artifact TypeID are indistinguishable without a namespace signal. The `urn:wos:` prefix is lightweight ceremony that pulls its weight in audit and log correlation.
3. **Why not keep the 5-segment URN and just put a TypeID in `<hash>`?** The `<scope>` and `<date>` segments are lossy re-encodings of TypeID data (tenant → scope, UUIDv7 timestamp → date). Maintaining them creates synthesis code that can diverge from the actual TypeID. The 3-segment form eliminates the synthesis — the TypeID IS the NSS.
4. **What about sub-resources (tasks, delegations, holds)?** Sub-resources are identified by URL path context (`/instances/{case_id}/tasks/{index}`), not independent TypeIDs. The TypeID prefix registry stays at 5 reserved entries instead of exploding to match the 14-member URN entity-type enum.
5. **Are we preserving backwards compat?** No. Greenfield. The old 5-segment URN is deleted, not deprecated. No dual-format acceptance, no migration window, no alias table.

---

## 2. Decision

### D-1. Narrow `WosResourceUrn` to 3-segment `urn:wos:<typeid>`

The current 5-segment pattern:

```
^urn:wos:(instance|task|bundle|...|timer):[A-Za-z0-9._:-]+:[0-9]{4}-[0-9]{2}-[0-9]{2}:[A-Za-z0-9._-]+$
```

is replaced with:

```
^urn:wos:[a-z][a-z0-9-]*_(case|prov|gov|ai|assurance|x-[a-z]+-[a-z]+)_[0-9a-hjkmnp-tv-z]{26}$
```

The namespace-specific string (NSS) IS the TypeID. Strip `urn:wos:` to extract the canonical TypeID. One identity from DB through API through durable execution through Trellis.

Examples:

```
urn:wos:sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsc
urn:wos:default_case_01hw7rm71vfay8vvw14d2pf2db
urn:wos:agency-gov_prov_01jqrxabcd3f8xtx9qxkkv3raaa
urn:wos:acme-corp_ai_01jqs1234abcd5f8xtx9qxkkabcde
```

### D-2. TypeID prefix = record family, not REST resource type

The closed entity-type alternation in the old URN (14 members: `instance | task | bundle | ...`) is retired. TypeID prefixes carry record families (5 reserved: `case | prov | gov | ai | assurance`). Sub-resources (tasks, delegations, holds, notifications) are identified by URL path context, matching ADR 0082 D-3's resource decomposition.

A task does not need a `_task_` prefix — it lives at `/instances/{case_id}/tasks/{index}`. This keeps the TypeID registry surface bounded and prevents it from growing a parallel REST-resource taxonomy.

### D-3. Server synthesis functions deleted

`to_instance_urn()` (`instance_service.rs:113-128`) — deleted. Replaced by `format!("urn:wos:{}", stored_typeid)`. No parsing, no fallback, no synthetic date construction.

`urn_scope_and_date()` (`instance_service.rs:131-139`, `task_service.rs:95-104`) — deleted. Scope and date extraction from the old URN is replaced by `typeid::extract_tenant()` and the UUIDv7 timestamp embedded in the TypeID suffix.

The HTTP create handler default ID generation (`instances.rs:233-239`) stops generating free-form `urn:wos:instance:default:...:<uuid>`. When no `instance_id` is supplied, the handler calls `typeid::mint_case_id()` and wraps the result with `urn:wos:`.

### D-4. `ActorRef` unchanged

`ActorRef` (`actor:<principalClass>:<id>`) is a separate identity namespace for the governance/identity subsystem (ADR 0082 D-9). It is not affected by this ADR.

### D-5. Single schema-file edit

Only `work-spec/schemas/api/_common.schema.json` changes — the `WosResourceUrn` `$def` pattern is replaced. All 18 resource schemas that `$ref` `WosResourceUrn` are unchanged. No `$ref` URI changes, no file renames.

### D-6. No backwards compatibility

The old 5-segment URN is deleted, not deprecated. No dual-format acceptance in the server, no alias table, no `/v2` version bump to protect `/v1`. The API is greenfield; there are no production consumers to preserve.

If a future identity change requires a breaking pattern, it will bump to `/v2` per ADR 0071 at that point, when there are real consumers to protect.

---

## 3. Consequences

### Positive

- One identity from DB through API through durable execution through Trellis. `id` on the API wire IS the TypeID (with an 8-byte namespace prefix). Zero heuristic parsing, zero synthesis, zero fallback.
- `to_instance_urn` and `urn_scope_and_date` are deleted — two functions whose only purpose was papering over the identity mismatch.
- The `urn:wos:` prefix provides a clean extraction surface: `id[8..]` or `id.strip_prefix("urn:wos:")` gives the TypeID. No regex, no split-on-colon counting, no segment parsing.
- The TypeID prefix registry stays at 5 reserved entries. Sub-resources use path context instead of exploding the registry.
- A single `.schema.json` file edit; all other schema files, `$ref` URIs, and API contract surfaces are unchanged.

### Negative

- The `<scope>` and `<date>` segments that operators could read from old URNs at a glance are gone. The tenant is still visible in the TypeID prefix; the UUIDv7 timestamp is embedded but not human-readable without decoding.
- `cargo check/test` must pass across `wos-core`, `wos-runtime`, and `workspec-server` after the pattern change. The regex narrowing means existing test fixtures with old 5-segment URNs will break — each one needs a one-line ID update (mint a TypeID, wrap it, point the fixture at it).
- The schema-pattern change is breaking for any generated TypeScript types in `case-portal` that currently validate against the 5-segment regex.

### Neutral

- The `urn:wos:` envelope stays. Existing code that splits on `urn:wos:` continues to work. The 5→3 segment change is invisible to prefix-based URN parsing.
- TypeID minting infrastructure (`wos-core/src/typeid.rs`) is already shipped and tested. This ADR only changes where the minted TypeID is wrapped with a prefix — no new minting or validation surface.
- The `_common.schema.json` file continues to be the single edit point for the identity pattern, per ADR 0082 D-14's existing discipline.

---

## 4. Follow-on work

1. **Update `_common.schema.json`** — replace `WosResourceUrn` pattern and examples.
2. **Delete `to_instance_urn`** from `instance_service.rs`.
3. **Delete `urn_scope_and_date`** from `instance_service.rs` and `task_service.rs`.
4. **Replace HTTP create handler default ID** in `instances.rs` with `typeid::mint_case_id()` + `urn:wos:` wrap.
5. **Update all HTTP integration tests** — switch from 5-segment to 3-segment URN assertions.
6. **Regenerate typify types** in `workspec-server` to match the new pattern.
7. **Update `case-portal` generated types** to consume 3-segment URNs.
8. **Run full CI** — `cargo check --workspace`, `cargo nextest run --workspace`, `python3 -m pytest tests/schemas -q`.
