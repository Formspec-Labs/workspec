# ADR 0092 — TypeID-in-URN identity landing

**Status:** Not started  
**Anchor:** [`thoughts/adr/0092-api-typeid-urn-identity.md`](../adr/0092-api-typeid-urn-identity.md)  
**Closes:** ADR 0092 acceptance; PLN-0410

---

## Context

ADR 0092 narrows `WosResourceUrn` from the old 5-segment URN (`urn:wos:<entity-type>:<scope>:<date>:<hash>`) to a 3-segment URN whose namespace-specific string IS the TypeID (`urn:wos:<typeid>`). Strip `urn:wos:` → canonical TypeID. One identity from DB through API through durable execution through Trellis.

Greenfield posture: no backwards compat, no dual-format acceptance, no deprecation window. Delete the old synthesis functions; replace with mint-and-wrap. The old 5-segment URN is gone from every surface.

**Scope:** ~42 files across `work-spec/schemas/`, `work-spec/crates/`, `workspec-server/`, `case-portal/`, and `work-spec/specs/api/`.

---

## End-state assertions (definition of done)

- [ ] `WosResourceUrn` regex in `_common.schema.json` matches `^urn:wos:<typeid>$`
- [ ] `grep -rn "urn:wos:instance:" workspec-server/` returns zero matches
- [ ] `grep -rn "to_instance_urn" workspec-server/` returns zero matches
- [ ] `grep -rn "urn_scope_and_date" workspec-server/` returns zero matches
- [ ] `grep -rn "task_urn" workspec-server/` returns zero matches
- [ ] `grep -rn "instance_urn" workspec-server/` returns zero matches
- [ ] `grep -rn "parse_instance_urn_segments\|is_instance_urn\|extract_urn_parts" work-spec/crates/wos-core/` returns zero matches (or functions rewritten for new format)
- [ ] HTTP create handler mints via `typeid::mint_case_id()` + wraps with `urn:wos:`
- [ ] All ~55 test fixtures use 3-segment `urn:wos:{typeid}` — no old 5-segment strings remain
- [ ] `cargo build -p wos-server` regenerates typify types from new pattern
- [ ] `cargo nextest run --workspace` green
- [ ] `python3 -m pytest tests/schemas -q` green (schema examples validate against new pattern)
- [ ] `case-portal/src/types/wos/` regenerated from new schemas
- [ ] `case-portal` `npx tsc --noEmit` clean
- [ ] `case-portal` `npx vitest run` green
- [ ] 10 API spec docs updated from 5-segment to 3-segment URN

---

## Work streams

### WS-1 — Schema update (1 file)

**`work-spec/schemas/api/_common.schema.json`** — replace `WosResourceUrn` pattern and examples.

Old pattern (line 20):
```
^urn:wos:(actor|agent|appeal|bundle|correspondence-message|delegation|hold|instance|notification|profile|provenance-record|report-run|signature-ceremony|task|timer):[A-Za-z0-9._:-]+:[0-9]{4}-[0-9]{2}-[0-9]{2}:[A-Za-z0-9._-]+$
```

New:
```
^urn:wos:[a-z][a-z0-9-]*_(case|prov|gov|ai|assurance|x-[a-z]+-[a-z]+)_[0-9a-hjkmnp-tv-z]{26}$
```

Description and examples updated to reflect 3-segment `urn:wos:{typeid}` shape.

After this edit, `cargo build -p wos-server` regenerates all `typify`-derived types in `workspec-server/crates/wos-server/src/api/types/` with the new pattern. All 120 `$ref` sites across 15 schema files resolve the new pattern automatically — no schema file changes beyond `_common.schema.json`.

---

### WS-2 — `workspec-server` Rust code (3 files, 8 function deletions/replacements)

#### 2a. `workspec-server/crates/wos-server/src/services/instance_service.rs`
- **Delete** `to_instance_urn` (lines 113-129)
- **Delete** `urn_scope_and_date` (lines 131-140)
- **Replace** 3 call sites of `to_instance_urn` (lines 63, 146, 190) with `format!("urn:wos:{}", row.instance_id)`
- **Replace** `urn_scope_and_date` call site (lines 57-58). Task URN construction: since tasks are sub-resources identified by path context per ADR 0092 D-2, synthesize as `format!("urn:wos:{}#tasks/{}", row.instance_id, task_index)` or a format TBD — or drop task URN synthesis entirely if tasks are always returned with their parent instance context.

#### 2b. `workspec-server/crates/wos-server/src/services/task_service.rs`
- **Delete** `urn_scope_and_date` duplicate (lines 95-104)
- **Delete** `task_urn` (lines 77-83)
- **Delete** `instance_urn` (lines 85-93)
- **Replace** all 4 call sites. Sub-resources identified by parent instance URN + local index.

#### 2c. `workspec-server/crates/wos-server/src/http/instances.rs`
- **Replace** default ID generation (lines 233-240):
  ```rust
  // OLD
  let instance_id = body.instance_id.unwrap_or_else(|| {
      format!("urn:wos:instance:{}:{}:{}", "default", chrono::Utc::now().format("%Y-%m-%d"), uuid::Uuid::now_v7())
  });
  // NEW
  let instance_id = body.instance_id.unwrap_or_else(|| {
      format!("urn:wos:{}", wos_core::typeid::mint_case_id())
  });
  ```

#### 2d. `workspec-server/crates/wos-server/src/services/applicant_service.rs`
- **Fix** fallback URN (line 178-179): `"urn:wos:instance:reference:fallback:0"` → minted TypeID or a conformance-valid placeholder like `"urn:wos:default_case_00000000000000000000000000"` (all-zeros base32 is not a valid UUIDv7, so use a real mint for test paths).

---

### WS-3 — `wos-core` Rust code (1 file, 3 function rewrites)

#### `work-spec/crates/wos-core/src/instance.rs`

Three functions parse/validate the old 5-segment URN. These need to either be deleted or rewritten for the new format:

- **`parse_instance_urn_segments`** (lines 119-148) — private, splits on `:` and extracts scope/date/id. Rewrite to: strip `urn:wos:` prefix, validate the remainder is a valid TypeID via `typeid::is_valid_type_id`. Return `Option<&str>` (the TypeID string). Callers that need scope/date extract them from the TypeID tenant and UUIDv7 timestamp.

- **`is_instance_urn`** (lines 181-187) — checks `urn:wos:instance:` prefix. Rewrite to: strip `urn:wos:`, check `typeid::is_valid_type_id`. Keeps the name since the calling code in `wos-runtime/src/runtime/instance.rs:162` gates on this. The semantics change from "is this a 5-segment instance URN?" to "is this a valid TypeID-in-URN?"

- **`extract_urn_parts`** (lines 193-196) — returns `Option<(&str, &str, &str)>` for (scope, date, id). Rewrite or delete depending on whether the 2 consumers (`urn_scope_and_date` in server services) still need scope/date. Since those consumers are being deleted (WS-2), this function's callers vanish. Delete it, or if kept for `wos-runtime` consumer (line 134-136) that extracts tenant, replace with `typeid::extract_tenant`.

**Consumer in `wos-runtime`:** `work-spec/crates/wos-runtime/src/runtime/instance.rs:134-136`

```rust
wos_core::instance::CaseInstance::extract_urn_parts(&instance_id)
    .map(|(ns, _, _)| ns.to_string())
```

Replace with `typeid::extract_tenant(&instance_id).map(String::from)`.

---

### WS-4 — `workspec-server` test fixture updates (~33 fixtures)

All integration test files under `workspec-server/crates/wos-server/tests/integration/` that reference old 5-segment URNs need their `instance_id` strings updated. The typical pattern:

```rust
// OLD
"urn:wos:instance:default:2026-04-15:abc123"
// NEW
let id = format!("urn:wos:{}", wos_core::typeid::mint_case_id());
```

Or use a helper that produces a valid 3-segment URN. Key files:

| File | Est. fixtures | Pattern to update |
|---|---|---|
| `http_coverage_backfill.rs` | 8 | `urn:wos:instance:...` → typeid-based |
| `ws_spec_gaps_2.rs` | 8 | `urn:wos:instance:...` → typeid-based |
| `runtime_lifecycle.rs` | 2 | both instance + task URNs |
| `http_tasks_lifecycle.rs` | 1 | task URN |
| `http_tenant_passthrough.rs` | 2 | already uses TypeID — verify they still pass |
| `audit_sink_consistency.rs` | 1 | instance URN |
| `http_coverage_slice_b.rs` | 2 | instance URN |
| `signature_affirmations.rs` | 2 | instance URN |
| `adr_0082_response_conformance.rs` | 1 | instance URN |
| `http_coverage_slice_c.rs` | 2 | instance URN |
| `timer_poll_e2e.rs` | 1 | instance URN |

Additional server fixtures:
| `src/api/types/mod.rs` | 2 | doc examples / constants |
| `src/seed.rs` | 1 | seed data |
| `crates/wos-server-runtime-restate/src/instance_seed.rs` | 2 | seed data |

---

### WS-5 — `wos-core` test fixture updates (~6 fixtures)

**`work-spec/crates/wos-core/tests/instance_deser.rs`** (6 occurrences). These test `parse_instance_urn_segments`, `is_instance_urn`, and `extract_urn_parts`. Rewrite to test the new 3-segment format — strip prefix, validate TypeID, extract tenant.

---

### WS-6 — `case-portal` updates (~15 files)

#### 6a. Type regeneration
- Regenerate `case-portal/src/types/wos/` from updated schemas. The `json-schema-to-typescript` pipeline reads `_common.schema.json` — the new pattern propagates automatically through `$ref` resolution.

#### 6b. Fixture and test updates (~12 fixture occurrences)
Key files:
| File | Count |
|---|---|
| `src/adapters/fixture/ports.ts` | 8 |
| `src/components/audit/AuditViewer.test.tsx` | 2 |
| `src/App.tsx` | 1 |
| `src/components/portal/ApplicantPortal.tsx` | 1 |
| `src/adapters/fixture/workspace.test.ts` | 1 |
| `tests/integration/adr-0082-response-conformance.test.ts` | 1 |
| `tests/integration/api.test.ts` | 2 |
| `server.ts` | 6+ |
| `e2e/journeys/mobile-journeys.spec.ts` | 1 |
| `e2e/journeys/extended-journeys.spec.ts` | 2 |

Pattern: replace hardcoded 5-segment URNs with TypeID-based 3-segment URNs.

#### 6c. Conformance test update
- `tests/integration/adr-0082-response-conformance.test.ts:11,13` — AJV validation against `WosResourceUrn` schema. Update the AJV schema registry with the new `_common.schema.json` to validate against the new pattern.

---

### WS-7 — Spec doc updates (~10 files)

All API spec docs under `work-spec/specs/api/` that reference the old 5-segment URN shape:

| File | Update |
|---|---|
| `_common.md` | Update `urn:wos:<entity-type>:<scope>:<date>:<hash>` description → `urn:wos:<typeid>` |
| `instance.md` | Replace `urn:wos:instance:<scope>:<date>:<short-hash>` with `urn:wos:{typeid}` |
| `task.md` | Replace `urn:wos:task:<scope>:<date>:<short-hash>` with sub-resource identification |
| `provenance.md` | Replace `urn:wos:provenance-record:...` and `urn:wos:instance:...` |
| `signature.md` | Replace `urn:wos:signature-ceremony:...` |
| `appeal.md` | Replace entity URNs |
| `bundle.md` | Replace entity URNs |
| `applicant.md` | Replace instance + task URNs |
| `audit.md` | Replace instance URN |
| `semantic.md` (profiles) | Replace `urn:wos:task:eligibility-review-001` if present |

The ADR 0082 stack-level doc (`thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md`) is amended per ADR 0092's "Amends" declaration — no separate edit needed if the amendment is noted; but at minimum a footnote or cross-reference should be added.

---

### WS-8 — CI verification

```sh
# Schema validation
python3 -m pytest work-spec/tests/schemas -q

# wos-core
cargo nextest run -p wos-core --lib
cargo nextest run -p wos-core --test instance_deser

# wos-runtime
cargo nextest run -p wos-runtime --lib

# workspec-server full test suite
cargo nextest run -p wos-server --tests

# Regenerate case-portal types and verify
cd case-portal && npm run build && npx tsc --noEmit && npx vitest run
```

---

## Execution order

Dependency chain: schema → typify regen → Rust code → tests → spec docs → case-portal.

1. **WS-1** — Update `_common.schema.json` pattern
2. **WS-3** — Rewrite `wos-core/src/instance.rs` URN parse functions
3. **WS-2** — Delete/replace synthesis functions in `workspec-server` services + handler
4. **WS-5** — Update `wos-core` tests
5. **WS-4** — Update `workspec-server` integration tests
6. **WS-8** — CI: `cargo nextest run --workspace` + `python3 -m pytest tests/schemas -q`
7. **WS-7** — Update API spec docs
8. **WS-6** — Regenerate case-portal types + update fixtures + CI

Steps 1-2 are independent and can run in parallel. Step 2 blocks step 3 (server services import from `wos_core::instance`). Step 6 gates everything.

---

## Execution log

*(Populated as each step lands.)*
