# WOS API — TypeID-in-URN Identity

**Status:** Draft.  
**Date:** 2026-05-06.  
**Amends:** ADR 0082 D-4 (URN segment structure — narrows 5-segment to 3-segment with TypeID NSS) and ADR 0082 D-14 (URN ref discipline).  
**Related:** ADR 0061 (custody wire TypeID adoption); `wos-core/src/typeid.rs` (minting); kernel `custody-hook-encoding.md` §1.4 (TypeID rules); ADR 0068 D-1.1 (tenant grammar).

This spec narrows `WosResourceUrn` from the current 5-segment `urn:wos:<entity-type>:<scope>:<date>:<hash>` to a 3-segment `urn:wos:<typeid>`. The URN envelope survives; the namespace-specific string is the TypeID. Strip the `urn:wos:` prefix and you have the canonical TypeID — one identity from DB through API through durable execution through Trellis.

Greenfield: no legacy format acceptance, no migration, no deprecation window. Replace everywhere in one pass.

---

## 1. Normative Contract

### R-1. Identity shape

**R-1.1** Every top-level resource `id` MUST be:

```
urn:wos:<typeid>
```

where `<typeid>` is `{tenant}_{type}_{uuidv7_base32}` per kernel `custody-hook-encoding.md` §1.4.

```
urn:wos:sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsc
```

**R-1.2** The TypeID type-prefix carries the *record family* (`case|prov|gov|ai|assurance` or vendor `x-{vendor}-{kind}`), not the REST resource type. Sub-resources (tasks, delegations, holds, notifications) are identified by URL path context (`/instances/{case_id}/tasks/{index}`) and do NOT get independent TypeIDs.

**R-1.3** The `id` field of every API resource schema `$ref`s `WosResourceUrn` from `_common.schema.json`. The definition changes; the `$ref` URIs do not.

**R-1.4** The old 5-segment URN (`urn:wos:<entity-type>:<scope>:<date>:<hash>`) is deleted — no acceptance, no aliasing, no mapping. Bare TypeIDs (without `urn:wos:` prefix) are NOT valid API `id` values.

### R-2. Idempotency and uniqueness

**R-2.1** The `id` is stable for the resource lifecycle. The TypeID mints once at creation via `wos_core::typeid::mint_*()`.

**R-2.2** The UUIDv7 core provides global uniqueness by construction. No per-resource-uniqueness enforcement beyond format validation.

### R-3. Schema pattern

**R-3.1** `_common.schema.json` `WosResourceUrn` becomes:

```json
{
  "WosResourceUrn": {
    "type": "string",
    "pattern": "^urn:wos:[a-z][a-z0-9-]*_(case|prov|gov|ai|assurance|x-[a-z]+-[a-z]+)_[0-9a-hjkmnp-tv-z]{26}$",
    "description": "WOS public API resource URN. Shape urn:wos:<typeid> where <typeid> is the stack canonical TypeID per custody-hook-encoding.md §1.4. Strip the urn:wos: prefix to extract the TypeID. One identity, zero translation.",
    "examples": [
      "urn:wos:sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsc",
      "urn:wos:default_case_01hw7rm71vfay8vvw14d2pf2db",
      "urn:wos:agency-gov_prov_01jqrxabcd3f8xtx9qxkkv3raaa",
      "urn:wos:acme-corp_ai_01jqs1234abcd5f8xtx9qxkkabcde"
    ]
  }
}
```

**R-3.2** All existing `id` `$ref`s across `schemas/api/*.schema.json` remain. Only the pattern in `_common.schema.json` changes. No file moves, no `$ref` URI changes.

### R-4. Server behavior

**R-4.1** At resource creation:

```rust
let typeid = typeid::mint_case_id();
let id = format!("urn:wos:{typeid}");
```

**R-4.2** `GET /api/v1/instances/{id}` resolves by direct string match on the stored `process_id`.

**R-4.3** The `to_instance_urn()` synthesis function (`workspec-server/crates/wos-server/src/services/instance_service.rs:113-128`) is deleted. Its replacement is `format!("urn:wos:{}", row.process_id)` — no parsing, no fallback, no heuristic.

**R-4.4** `urn_scope_and_date()` (`instance_service.rs:131-139`, `task_service.rs:95-104`) is deleted. Scope and date extraction from the URN is replaced by `typeid::extract_tenant(typeid)` and the UUIDv7 timestamp embedded in the TypeID suffix.

**R-4.5** The HTTP create handler default ID generation (`instances.rs:233-239`) stops generating free-form five-segment workflow-process URNs. When no `process_id` is supplied by the client, it mints via `typeid::mint_process_id()` and wraps.

**R-4.6** `ActorRef` (`actor:<principalClass>:<id>`) is unchanged by this spec.

### R-5. Versioning

**R-5.1** The API ships at `/api/v1`. There is no existing production API surface to preserve; this is the first stable API. The current `workspec-server` HTTP surface is pre-stable development scaffolding.

**R-5.2** If a future identity change requires a breaking pattern, it bumps to `/api/v2` per ADR 0071. ADR 0071's deprecation-window rules apply at that point, when there are real consumers to protect.

---

## 2. Composition

| Composes with | How |
|---|---|
| ADR 0061 | ADR 0061 defined TypeID for custody hook + durable execution. This spec extends it to the API inside a `urn:wos:` namespace envelope. |
| ADR 0082 | Narrows D-4 from 5-segment to 3-segment. Preserves D-3 (resource decomposition), D-5 through D-13. |
| `wos-core/src/typeid.rs` | Byte authority for minting and validation. Server calls `mint_*()` and wraps. |
| `custody-hook-encoding.md` §1.4 | Authoritative for the TypeID format embedded in the URN. This spec references, does not duplicate. |
| `work-spec/schemas/api/` | One file edits (`_common.schema.json`). All other schemas unchanged. |
| `workspec-server` | Handlers mint + wrap. `to_instance_urn` deleted. `urn_scope_and_date` deleted. |
| `case-portal` | Generated types consume 3-segment URNs. |

### 2.1 Precedence

- **This spec amends ADR 0082 D-4 and D-14.** Remaining ADR 0082 decisions unchanged.
- **`custody-hook-encoding.md` §1.4 is authoritative** for the embedded TypeID format.
- **`typeid.rs` validators are authoritative** for runtime TypeID acceptance.

### 2.2 Resource identity vs record-family identity

TypeID prefixes map to record families, not REST resource types:

| Prefix | Record family | REST path |
|---|---|---|
| `case` | Workflow process | `/instances/{id}` |
| `prov` | Provenance records | `/instances/{case_id}/provenance/{id}` |
| `gov` | Governance records | `/instances/{case_id}/governance/...` |
| `ai` | AI records | `/instances/{case_id}/...` |
| `assurance` | Assurance records | `/instances/{case_id}/...` |

Sub-resources use path context + local identifier, not independent TypeIDs.

### 2.3 Why the `urn:wos:` prefix stays

1. **Namespace signal.** In multi-system environments, `urn:wos:acme-corp_case_...` disambiguates WOS resources from Formspec responses, Trellis bundles, etc.
2. **Consumer non-breakage.** Existing code that parses `urn:wos:` strings works unchanged. The 5→3 segment change is a pattern narrowing, not a format-category jump.
3. **The cost is trivial.** Append 8 bytes on output; strip 8 bytes on input. No allocation overhead beyond what `format!` already does.

---

## 3. Conformance

### 3.1 Schema validation

- `WosResourceUrn` pattern updated in `_common.schema.json`.
- Schema examples updated to 3-segment URNs.
- `test_examples_validate.py` enforces the new pattern on all examples.

### 3.2 Rust unit tests

- `typeid.rs` existing tests remain format authority.
- New test: `typeid_to_urn_roundtrip` — mint, wrap as `urn:wos:{typeid}`, strip prefix, assert equality.
- New test: `urn_pattern_matches_all_reserved_families` — each reserved prefix mints, wraps, validates against the regex.
- New test: `urn_pattern_matches_vendor_families` — vendor prefix variant.

### 3.3 HTTP integration tests

- All `workspec-server` HTTP tests switch to 3-segment URN assertions.
- New test: `create_instance_returns_typeid_urn` — response `id` matches `^urn:wos:.*_case_.*$`.
- New test: `get_instance_by_typeid_urn` — `GET /instances/{urn:wos:...}` resolves.
- New test: `urn_scope_and_date_deleted` — verify the deleted functions are not reachable from any handler.

### 3.4 DB normalization

- `process_id` in the DB stores the 3-segment URN string (`urn:wos:{typeid}`).
- Any pre-existing rows with bare TypeIDs or legacy 5-segment URNs are normalized in a one-time migration script. No ongoing dual-format acceptance.

---

## 4. Implementation sequence

| Step | What | Files touched |
|---|---|---|
| **1** | Update `WosResourceUrn` pattern and examples in `_common.schema.json` | 1 file |
| **2** | Regenerate typify types; update server DTOs to match new pattern | `workspec-server` generated types |
| **3** | Replace `to_instance_urn` with `format!("urn:wos:{}", typeid)`; delete `urn_scope_and_date` | `instance_service.rs`, `task_service.rs` |
| **4** | Replace HTTP create handler default ID generation with `typeid::mint_case_id()` + wrap | `instances.rs` |
| **5** | Update all HTTP tests to assert 3-segment URN | `workspec-server/tests/` |
| **6** | Run full CI: `cargo check --workspace`, `cargo nextest run --workspace`, `python3 -m pytest tests/schemas -q` | — |
| **7** | Normalize any dev DB rows | one-time script |
