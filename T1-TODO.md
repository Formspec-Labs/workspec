# WOS-T1 TODO — `custodyHook` ADR-0061 Execution Cascade

Working plan for WOS-T1: execute the accepted `custodyHook` wire-format ADR across WOS and the Trellis binding seam.

**Status:** complete (2026-04-21)  
**Owner:** WOS-T1 / Trellis Stream 5  
**Stack boundary:** WOS owns authored record semantics, TypeID minting, schema enforcement, JSON→dCBOR conversion, and the runtime append surface; Trellis owns canonical append, idempotency consumption, anchoring, and verification of the emitted bytes.

## Closeout Summary

WOS-T1 landed on 2026-04-21 with the accepted ADR-0061 shape in code, schemas, and spec prose:

- Registry and schema surfaces publish reserved WOS TypeID families and `wos.*` ownership.
- `wos-core` now mints TypeID-backed provenance-record IDs and case-instance IDs at authoring time.
- `wos-runtime::custody` now emits the narrow four-field append input (`caseId`, `recordId`, `eventType`, `record`) with dCBOR-authored bytes and the 2-tuple idempotency source `(caseId, recordId)`.
- The superseded JCS / wide-shape custody path is gone from the live runtime surface.
- `CustodyAppendReceipt { canonical_event_hash }` is defined, `#[non_exhaustive]`, and wired through a persisted provenance-stamping path.
- Trellis Operational Companion §24.9 now explicitly names the four-field WOS wire, 2-tuple idempotency input, and minimum receipt.

Verification run on closeout:

- `cargo test -p wos-core --lib`
- `cargo test -p wos-runtime --lib`
- `cargo test -p wos-export --lib`
- `cargo test -p wos-conformance --lib`
- `pytest tests/schemas/test_custody_hook_encoding.py tests/schemas/test_extension_registry.py tests/schemas/test_facts_tier_snapshot.py tests/schemas/test_facts_tier_outcome.py tests/schemas/test_capability_invocation_record.py tests/schemas/test_override_record_shape.py tests/schemas/test_case_instance_typeid.py tests/schemas/test_meta_validity.py`
- `npm run docs:check`

---

## Completion Contract

WOS-T1 is complete only when the accepted ADR is true in code, schemas, spec prose, and Trellis verification:

1. WOS publishes the reserved TypeID family prefixes and `wos.*` event-type ownership in the extension-registry surface.
2. WOS case instances and authored records mint TypeIDs at authoring time rather than deriving them from log position or storage order.
3. Record-family schemas reject malformed `caseId` / `id` values at authoring time.
4. The binding mechanically converts schema-valid WOS JSON records into dCBOR using the ADR-0061 encoding table and rejection list.
5. The WOS runtime emits the narrow four-field append input: `caseId`, `recordId`, `eventType`, `record`.
6. The WOS-side idempotency source tuple is exactly `(caseId, recordId)` with domain tag `trellis-wos-idempotency-v1`.
7. The runtime receipt surface is narrowed to `CustodyAppendReceipt { canonical_event_hash }`, and WOS stamps that hash into the first downstream consumer path.
8. Trellis fixture `append/010-wos-custody-hook-state-transition` and Trellis Operational Companion §24.9 match the final emitted shape.
9. Round-trip fixture corpora byte-match in Rust and Python for every WOS record family crossing `custodyHook`.

---

## Ownership

| Layer | Owns |
|---|---|
| WOS spec | TypeID format rules, four-field append surface, idempotency tuple, return contract, conversion rules, rejection list, out-of-scope declarations |
| WOS schemas | TypeID patterns on `caseId` / `id`, family-specific prefix enforcement, schema-test coverage |
| WOS runtime | TypeID minting, JSON→dCBOR conversion, four-field append input, receipt propagation, runtime tests |
| WOS core | Record constructors mint `id`; `CaseInstance::create` mints `caseId`; downstream provenance stamping consumes `canonical_event_hash` |
| Trellis | Canonical append, idempotency-key realization, append/010 verification corpus, operational-companion alignment |
| Python cross-check | Byte-match validation against Rust-authored `record.dcbor` / `record.sha256` fixtures |

WOS MUST NOT own `ledger_scope`, `sequence`, `prev_hash`, `canonical_event_hash` computation, checkpoint metadata, anchor target selection, or posture-transition canonical-event semantics. Those remain Trellis-owned.

---

## T1-0 — Freeze The Acceptance Surface

- [x] Confirm the implementation target remains the accepted ADR in [ADR-0061](thoughts/adr/0061-custody-hook-trellis-wire-format.md), not the superseded JCS draft.
- [x] Pin the execution surface in `TODO.md` as:
  - [x] four-field wire: `caseId`, `recordId`, `eventType`, `record`
  - [x] idempotency tuple: `(caseId, recordId)`
  - [x] return contract: `CustodyAppendReceipt { canonical_event_hash }`
  - [x] authored bytes: dCBOR-via-hybrid from schema-valid JSON records
- [x] Explicitly preserve the ADR's non-goals:
  - [x] no WOS-side `ledger_scope`
  - [x] no WOS-side `sequence`
  - [x] no WOS-side `prev_hash`
  - [x] no WOS-side anchor-target field
  - [x] no widened receipt before a concrete WOS consumer requires it
- [x] Confirm placement choice for the initial converter implementation: `wos-runtime::custody` landed in T1; no new `wos-trellis-binding` crate was introduced.
- [x] Record the choice in this file and in the landing PR description.

---

## T1-1 — Registry Entries And Naming Discipline

- [x] Extend [specs/registry/extension-registry.md](specs/registry/extension-registry.md) with the reserved TypeID family prefixes from ADR §2.4.1:
  - [x] `case`
  - [x] `prov`
  - [x] `gov`
  - [x] `ai`
  - [x] `assurance`
- [x] State that one TypeID prefix maps to one top-level record family; sub-kind discrimination lives in the record's in-bytes `recordKind`.
- [x] State that vendor-owned families use `x-{vendor}-{kind}` naming consistent with the registry's vendor-prefix discipline.
- [x] Add or update machine-readable registry examples showing:
  - [x] `wos.kernel.*`
  - [x] `wos.governance.*`
  - [x] `wos.ai.*`
  - [x] `wos.assurance.*`
- [x] Ensure the registry prose says the `wos.*` event-type namespace is owned by WOS and referenced by Trellis at a declared WOS spec version.
- [x] Update any registry schema/examples needed so doc and schema stay aligned.
- [x] Add a bounded regression check:
  - [x] docs render cleanly
  - [x] no schema-doc drift

---

## T1-2 — TypeID Utility And Authoring-Time Minting

- [x] Add a stack-local TypeID utility in WOS code, inline for now.
- [x] Encode the adopted TypeID format:
  - [x] `{tenant}_{type}_{uuidv7_base32}`
  - [x] tenant pattern `[a-z][a-z0-9-]*`
  - [x] lowercase Crockford base32 UUIDv7 payload, 26 chars
- [x] Provide mint helpers for:
  - [x] `caseId`
  - [x] `prov`
  - [x] `gov`
  - [x] `ai`
  - [x] `assurance`
- [x] Ensure every `ProvenanceRecord` constructor mints `id` at construction time.
- [x] Ensure `CaseInstance::create` mints `caseId` at creation time.
- [x] Remove any fallback path that synthesizes record identity from log position, append order, or persistence-local sequence.
- [x] Add tests covering:
  - [x] format conformance for each reserved prefix
  - [x] lowercase/base32 discipline
  - [x] stable tuple reuse for retries of the same authored fact
  - [x] fresh `recordId` for genuinely new authored facts

---

## T1-3 — Schema Tightening For TypeIDs

- [x] Close the schema-surface gap before calling T1.3 complete:
  - [x] tighten TypeID patterns on record-family schemas that already exist
  - [x] narrow the claimed scope explicitly where no custody-emitted runtime-record family exists yet
- [x] Tighten record-family schemas to require TypeID patterns on `id`.
- [x] Tighten any sibling `caseId` fields to require the `case` prefix pattern.
- [x] Cover the active emitted surfaces:
  - [x] Kernel Facts-tier provenance schema
  - [x] Governance record schemas
  - [x] case-instance / custody-adjacent runtime schemas
  - [x] AI and Assurance remain top-level document schemas, not current custody-emitted runtime-record families
- [x] Parameterize the family-specific prefix patterns per ADR §2.4.1 rather than using one generic catch-all.
- [x] Add or update pytest schema contracts for:
  - [x] valid `caseId`
  - [x] invalid tenant segment
  - [x] invalid family prefix
  - [x] invalid UUIDv7/base32 tail
- [x] Confirm authoring tools and fixture validation fail early on malformed IDs before the converter runs.

---

## T1-4 — JSON→dCBOR Converter And Fixture Corpus

- [x] Decide implementation location: `wos-runtime::custody` landed in T1; no new `wos-trellis-binding` crate was introduced.
- [x] Implement a single mechanical converter for any schema-valid WOS record.
- [x] Implement the closed encoding table from ADR §2.2:
  - [x] integers → CBOR major type 0/1, reject outside ±2^63−1
  - [x] numbers → float64, reject `NaN` / `+Infinity` / `-Infinity`
  - [x] `date-time` → CBOR tag 0
  - [x] `uri` → CBOR tag 32 remains part of the normative contract; no live provenance-record field in the current custody emitter set exercises it yet
  - [x] base64/binary → CBOR byte string remains part of the normative contract; no live provenance-record field in the current custody emitter set exercises it yet
  - [x] strings → CBOR text
  - [x] booleans / null / arrays / objects → canonical dCBOR equivalents
- [x] Implement the rejection list from ADR §2.7:
  - [x] integer overflow
  - [x] non-finite floats
  - [x] ill-formed UTF-8 is rejected before converter entry because authored JSON strings are UTF-8 and schema/serde parsing fails earlier
  - [x] undeclared special formats without an `x-` vendor encoding rule
- [x] Enforce dCBOR canonical map ordering.
- [x] Enforce the 1.0 size posture:
  - [x] authored WOS records must fit Trellis `PayloadInline`
  - [x] no automatic `PayloadExternal` escape hatch
- [x] Provide a reverse dCBOR→JSON inspection path sufficient for fixture verification and human-readable debugging.
- [x] Add a round-trip corpus for the live record family crossing `custodyHook` today:
  - [x] `record.json`
  - [x] `record.dcbor`
  - [x] `record.sha256`
- [x] Byte-match Rust and Python for every corpus entry.
- [x] Make Rust the byte authority when a cross-check exposes ambiguity, then update Python to match.

---

## T1-5 — Normative Encoding Spec Section

- [x] Land the missing normative artifact: [specs/kernel/custody-hook-encoding.md](specs/kernel/custody-hook-encoding.md).
- [x] Publish the WOS normative encoding prose at one of:
  - [x] `specs/kernel/custody-hook-encoding.md`
  - [x] kernel §8 / §10 insertion
- [x] Include the normative contract for:
  - [x] four-field append input
  - [x] TypeID rules and reserved prefixes
  - [x] `wos.*` event-type ownership
  - [x] dCBOR encoding table
  - [x] conversion rejection list
  - [x] domain-separated idempotency-key construction inputs
  - [x] return contract `canonical_event_hash`
  - [x] oversized-record rejection
  - [x] Trellis-owned fields explicitly excluded from the authored surface
- [x] Make the section point back to ADR-0061 for rationale only; normative truth must stand on its own.
- [x] Run docs checks after placement is chosen.

---

## T1-6 — Runtime Rewrite To The Narrow Wire Surface

- [x] Rewrite [crates/wos-runtime/src/custody.rs](crates/wos-runtime/src/custody.rs) to match ADR-0061 instead of the superseded JCS draft.
- [x] Replace the current wide/JCS append input with the narrow four-field wire shape.
- [x] Preserve a richer in-process `CustodyAppendContext` Rust API for runtime callers that need correlation state off-wire.
- [x] Drop `serde_json_canonicalizer` from the `custodyHook` path.
- [x] Ensure `eventType` is outcome-neutral and registry-backed.
- [x] Ensure `recordId` is minted, not synthesized from append position.
- [x] Update `DurableRuntime::load_custody_append_window` and any adjacent runtime APIs that currently assume the old shape.
- [x] Update runtime tests for:
  - [x] wire-shape construction
  - [x] retry reuse of `(caseId, recordId)`
  - [x] converter failures are loud and non-partial
  - [x] posture-transition callers still model the two-record sequence rather than a hybrid event

---

## T1-7 — Receipt Wire-Through And Downstream Stamping

- [x] Add `CustodyAppendReceipt { canonical_event_hash }` as the first pinned WOS consumer surface.
- [x] Mark the receipt `#[non_exhaustive]` for additive evolution.
- [x] Thread the receipt through the first downstream WOS consumer path in Kernel §8's hash-of-record stamping flow.
- [x] Ensure downstream provenance uses Trellis's `canonical_event_hash`, not a recomputed WOS-side digest, as the durable citation.
- [x] Add tests covering:
  - [x] receipt propagation from runtime append to provenance stamping
  - [x] stamped hash equals Trellis-returned `canonical_event_hash`
  - [x] no speculative receipt fields are required by WOS consumers

---

## T1-8 — Trellis Verification Pass

- [x] Verify `../trellis/fixtures/vectors/append/010-wos-custody-hook-state-transition/` matches the accepted ADR:
  - [x] dCBOR-authored payload
  - [x] TypeID `caseId`
  - [x] TypeID `recordId`
  - [x] idempotency tuple input `(caseId, recordId)`
  - [x] domain tag `trellis-wos-idempotency-v1`
- [x] Verify [../trellis/specs/trellis-operational-companion.md](../trellis/specs/trellis-operational-companion.md) §24.9 matches the final emitted shape.
- [x] If Trellis still reflects the superseded JCS draft, patch Trellis before calling T1 complete.
- [x] Record the verification result in:
  - [x] `TODO.md`
  - [x] `COMPLETED.md` when closed

---

## Parallelization Plan

- [x] `T1.1` lands first.
- [x] `T1.2` and `T1.3` may run in parallel after `T1.1`.
- [x] `T1.4` and `T1.5` may run in parallel after `T1.1` and enough of `T1.2` exists to define the family patterns.
- [x] `T1.6`, `T1.7`, and `T1.8` may run in parallel once `T1.4` and `T1.5` are stable enough to pin the emitted shape.

---

## Explicitly Out Of Scope

- [x] Trellis `ledger_scope` modeling
- [x] Trellis `sequence` or `prev_hash` exposure on the WOS wire
- [x] WOS-side anchor-target selection
- [x] automatic promotion of oversized authored records to `PayloadExternal`
- [x] widening the receipt before a real WOS consumer requires it
- [x] changing the accepted TypeID scheme without a new ADR
