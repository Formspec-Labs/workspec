# ADR-0061: `custodyHook` wire format for the Trellis binding

**Status:** Accepted
**Date:** 2026-04-21
**Deciders:** WOS + Trellis Working Group
**Author:** WOS-T1 / Trellis Stream 5
**Supersedes:** None (rewrote the 2026-04-21 JCS-based draft of this same ADR in-place before acceptance).
**Related:**

- [TODO.md](../../TODO.md) — Do next #1 (`custodyHook` Trellis joint ADR)
- [Trellis TODO](../../../trellis/TODO.md) — Stream 5, WOS `custodyHook` joint ADR
- [Kernel §10.5 `custodyHook`](../../specs/kernel/spec.md)
- [Trellis Core §5 Encoding rules (dCBOR)](../../../trellis/specs/trellis-core.md)
- [Trellis Core §9.2 `canonical_event_hash`](../../../trellis/specs/trellis-core.md)
- [Trellis Core §23, Composition with WOS `custodyHook`](../../../trellis/specs/trellis-core.md)
- [Trellis Operational Companion §24.9](../../../trellis/specs/trellis-operational-companion.md)
- [Trellis ADR 0004 — Rust is byte authority](../../../trellis/thoughts/specs/2026-04-20-trellis-phase-1-mvp-principles-and-format-adrs.md)
- [WOS provenance-record schema](../../schemas/kernel/wos-provenance-record.schema.json)
- [Mirrored Trellis-side note](../../../trellis/thoughts/specs/2026-04-21-trellis-wos-custody-hook-wire-format.md)

---

## 1. Context

The seam already exists on both sides. Trellis Core §23 declares the Trellis-owned obligations (`wos.*` event-type namespace, `ledger_scope`, canonical append, idempotency, posture transitions) and defers to WOS for the authored-fact byte form. WOS Kernel §10.5 names `custodyHook` as the seam without pinning what crosses it.

The owner resolved the following load-bearing questions before acceptance:

1. **Authored-byte authority.** *dCBOR-via-hybrid.* WOS authors records in JSON (JSON Schema remains structural truth); the binding crate mechanically converts to dCBOR at the seam so that Trellis receives dCBOR-native authored bytes. This aligns with Trellis ADR 0004 (Rust is byte authority; one byte oracle at the chain layer) without disturbing WOS's JSON-native authoring surface.
2. **Append-input surface.** Narrow to four load-bearing wire fields. Everything else is either derivable from the record bytes or held as in-process runtime state — not wire.
3. **Idempotency tuple.** Tight: `(caseId, recordId)`. Uniqueness is a structural guarantee of the TypeID id-scheme (see §2.4.1), not an invariant that needs per-family fixture assertion.
4. **Return contract.** Minimum `{ canonical_event_hash }`. Extension seam for `sequence`, `ledger_scope`, and anchor metadata when a concrete consumer forces them; refused until then.
5. **Identifier scheme.** `caseId` and `recordId` are both TypeID-structured (`{tenant}_{type}_{uuidv7_base32}`). Time-ordered (UUIDv7 core), self-describing (type-prefix), tenant-routable. Adopted from the Temporal reference implementation as the stack-wide convention. TypeID is the identity primitive; a shared formspec-stack TypeID utility crate is a potential separate ADR, but not a blocker here.
6. **Record-field encoding rules.** Minimum semantic tagging: dates and URIs get CBOR tags; binaries become byte strings; other formats stay plain (see §2.2 table). Closed list — extensions require ADR amendment.
7. **Inter-record references.** Cross- and intra-case citations between WOS records use Trellis `canonical_event_hash` (content-addressed), not a replicated WOS-side id chain.
8. **`wos.*` event-type registry.** WOS Extension Registry (§21) is the owning registry; Trellis §14 bound-registry entries reference WOS entries at a declared spec version.

These resolutions are recorded normatively below. The cheap-revision window is still open — no WOS-Trellis records have been issued. G-5 (Trellis Phase-1 stranger test) corpus terminates at `append/009` and does not lock in any WOS-authored payload bytes.

---

## 2. Decision

### 2.1 Unit of admission

`custodyHook` admits **one authored WOS record per Trellis append**.

The authored WOS record is the thing WOS semantics define: a Kernel Facts-tier provenance record, a governance-sidecar record, or an AI / governance / assurance record defined by a WOS companion. Trellis wraps that authored record as one canonical event. It does NOT batch multiple WOS records into one append, and it does NOT split one authored WOS record across multiple canonical events.

Batched append is a Phase-1 simplification, not an architectural rejection. The idempotency tuple is per-record and extends cleanly if a later phase opens batch appends.

### 2.2 Authored-byte authority — dCBOR-via-hybrid

The WOS-authored payload crossing `custodyHook` is:

- authored as a WOS-native record object conforming to the WOS JSON Schema for that record kind,
- converted mechanically to **dCBOR** per Trellis Core §5 encoding rules at the binding seam,
- and handed to Trellis as the authored-byte material referenced by Trellis Core §23.2 item 3.

The dCBOR bytes — not the JSON bytes — are the authored-fact material Trellis wraps. Trellis computes `canonical_event_hash` (§9.2) over its own envelope as usual; the WOS record's dCBOR bytes are what sits inside the encrypted payload (`PayloadRef`, §6.4).

Rationale:

- **One byte oracle at chain layer.** Trellis ADR 0004 pins Rust as byte authority for CBOR / COSE / hash ambiguities. Routing dCBOR authored bytes through the same discipline eliminates a second byte-determinism surface at the seam.
- **JSON authoring preserved.** JSON Schema remains structural truth for WOS records. FEL, authoring tooling, fixtures, and JSON-view exports (PROV-O JSON-LD, XES, OCEL, audit diffing) are unchanged.
- **WOS §8.2.1 case-file snapshots stay JCS.** Those snapshots are WOS-internal (determination provenance, snapshot digests) and do not cross `custodyHook`. The dCBOR decision is scoped to the chain-binding seam.
- **Export remains JSON.** Human-readable chain-byte inspection is via a dCBOR→JSON renderer in the binding crate; dCBOR preserves typing so the render is deterministic.

**Record-field encoding rules (closed).** The JSON→dCBOR converter applies exactly these mappings; everything else rejects at conversion:

| JSON Schema signal | CBOR encoding |
|---|---|
| `"type": "integer"` | CBOR major type 0 or 1 (negative integer); reject values outside ±2^63−1 |
| `"type": "number"` (without `"integer"`) | CBOR float64 (major type 7); reject `NaN`, `+Infinity`, `-Infinity` |
| `"format": "date-time"` | CBOR tag 0 (RFC 3339 date/time string) |
| `"format": "uri"` | CBOR tag 32 (URI string) |
| `"contentEncoding": "base64"` or binary media type | CBOR byte string (major type 2), untagged |
| `"type": "string"` otherwise | CBOR text string (major type 3), untagged |
| `"type": "boolean"` | CBOR true / false (major type 7, values 20/21) |
| `"type": "null"` / JSON `null` | CBOR null (major type 7, value 22) |
| `"type": "array"` | CBOR array (major type 4); element-wise recursion |
| `"type": "object"` | CBOR map (major type 5); dCBOR sort-by-key; recursion |

The list is closed. Adding a new format-to-tag mapping requires an ADR amendment, not a converter patch. Vendors extending with `x-*` patternProperties supply their own encoding rule alongside the extension registration per WOS Extension Registry (§21).

### 2.3 WOS-owned append input — four fields

A WOS runtime routing a record through `custodyHook` MUST supply the following wire fields to the Trellis binding:

| Field | Owner | Meaning |
|---|---|---|
| `caseId` | WOS | TypeID-structured case identifier, pattern `^[a-z][a-z0-9-]*_case_[0-9a-hjkmnp-tv-z]{26}$`. Input to `ledger_scope` selection (Trellis §23.3) and to the idempotency tuple. |
| `recordId` | WOS | TypeID-structured record identifier, pattern `^[a-z][a-z0-9-]*_{wos-type-prefix}_[0-9a-hjkmnp-tv-z]{26}$`, where `{wos-type-prefix}` is registered per-record-family in the WOS Extension Registry (§21). Minted at authoring time by the WOS runtime — MUST NOT be synthesized from log position or derived after the fact. |
| `eventType` | WOS Extension Registry | Registered, outcome-neutral `wos.*` identifier for the record family admitted into Trellis. Canonical pattern: `wos.<layer>.<recordKind>` where `<layer>` ∈ {`kernel`, `governance`, `ai`, `assurance`} and `<recordKind>` matches the record's in-bytes `recordKind` field (outcome-neutral naming per Trellis §23.4). Registered under the `wos.*` family in the WOS Extension Registry (§21); Trellis §14 bound-registry entries reference the WOS entry at the declared WOS spec version. Registry entries MAY pin deviations from the canonical pattern for legacy or spec-version-specific bindings; deviations MUST be documented in the registry entry's `notes` field. |
| `record` | WOS | The authored WOS record rendered as dCBOR per §2.2. |

Fields deliberately NOT on the wire (available as in-process API on the binding, not pinned at the seam):

- `wosRecordKind`, `wosSpecVersion`, and any schema-identity pointer — carried inside the record itself or derivable from it.
- `workflowRef`, `instanceRef`, `lifecycleRef`, `governanceEnvelopeRef` — runtime correlation state; callers requiring them use the binding crate's richer Rust API, which composes the wire input from the authored record plus runtime context.
- `recordDigestSha256` — a derived integrity convenience. The binding MAY expose it as an API field; it is not a wire obligation. When callers supply one, the binding MUST recompute and reject mismatches.

The narrow wire surface evolves additively: new optional wire fields may be introduced later without breaking existing callers; pruning a wire field is always a breaking change. Start narrow.

### 2.4 Idempotency source tuple

For WOS-Trellis deployments, the WOS-owned stable source tuple for Trellis idempotency is:

`(caseId, recordId)`

Normative consequences:

1. Retries of the **same** authored WOS record MUST preserve both values.
2. A genuinely new authored WOS record MUST mint a new `recordId` (a new TypeID; see §2.4.1).
3. WOS runtime internals MAY retry, replay, or compensate however they want; they MUST NOT mint a fresh tuple for the same authored fact.

Trellis encodes or hashes this tuple into its concrete `idempotency_key` bytes per Trellis Core §17 / §23.5. The tuple above is the WOS-owned semantic input.

**Idempotency-key construction.** The bound construction is:

`idempotency_key = SHA-256( len_prefix(domain_separation_tag) || dCBOR(input_map) )`

where:

- `domain_separation_tag = "trellis-wos-idempotency-v1"` (ASCII bytes), length-prefixed per Trellis Core §9.1.
- `input_map` is the CBOR map `{"caseId": caseId, "recordId": recordId}`.
- `dCBOR(input_map)` encodes the map with dCBOR canonical rules: CBOR major type 5 (map), lexicographic key ordering by encoded-bytes order (dCBOR §5.1), both values encoded as CBOR text strings (major type 3, untagged) per the §2.2 encoding table for `"type": "string"`.

The 32-byte SHA-256 output fits the `.size (1..64)` bound on `idempotency_key` in Trellis §6.1. Trellis §17.5 `IdempotencyKeyPayloadMismatch` is the designated fail mode when a WOS-layer bug produces a different canonical payload under the same key.

**Uniqueness is structural, not asserted.** `recordId`'s UUIDv7 core (see §2.4.1) makes collisions within a case astronomically unlikely by construction. Per-family uniqueness fixtures are not required; a format-regression test per family suffices. Runtime violation surfaces loudly as `IdempotencyKeyPayloadMismatch`.

**Rationale for dropping `eventType` from the tuple** relative to the earliest draft: `recordId` is TypeID-unique; `eventType` is deterministic from the record kind. Including it in the key either adds nothing (if the mapping is stable) or creates drift surface (if a registry change re-points the identifier). The tight tuple fails loudly on invariant violation; a looser tuple fails silently by splitting retries of the same fact into two chain entries. Loud failure is cheaper to diagnose.

### 2.4.1 TypeID format

Both `caseId` and `recordId` use the TypeID format adopted stack-wide from the reference implementation at [`work-spec/thoughts/examples/temporal-reference-implementation.md`](../examples/temporal-reference-implementation.md).

`{tenant}_{type}_{uuidv7_base32}`

- **`{tenant}`** — operator-assigned deployment prefix, `[a-z][a-z0-9-]*`. Single-tenant deployments pick a fixed tenant (recommended: a short deployment identifier; `default` is acceptable). Multi-tenant deployments route by the prefix.
- **`{type}`** — record-family type prefix. **One prefix per top-level record family**; sub-kinds within a family are discriminated by the record's in-bytes `recordKind` field, not by a finer-grained TypeID prefix. Reserved WOS prefixes:

  | Prefix | Family | Covers (sub-kinds discriminated by `recordKind`) |
  |---|---|---|
  | `case` | WOS workflow process | (the case itself; not a record) |
  | `prov` | Kernel Facts-tier provenance records | `stateTransition`, `caseStateMutation`, `milestoneFired`, `convergenceCapReached`, etc. |
  | `gov` | Governance records | `overrideRecord`, `delegationGrant`, `holdApplied`, `authorityVerification`, `appealInitiated`, etc. |
  | `ai` | AI Integration records | `capabilityInvocation`, `driftAlert`, `autonomyDemotion`, `agentEscalation`, etc. |
  | `assurance` | Assurance records | `attestation`, `identityBinding`, `subjectContinuity`, etc. |

  Vendor extensions use `x-{vendor}-{kind}` form consistent with WOS Extension Registry (§21) and ADR-0060 vendor-prefix discipline.

  Keeping type-prefixes coarse (one per family) limits registry surface and matches the authoring-layer grouping; fine-grained discrimination lives in the authored bytes where the schema already enforces it.
- **`{uuidv7_base32}`** — lowercase Crockford base32 encoding of a UUIDv7 (RFC 9562), 26 characters, pattern `[0-9a-hjkmnp-tv-z]{26}`.

Type-prefix registration lives in the WOS Extension Registry (§21) alongside `wos.*` event-type registrations.

UUIDv7 construction requires a monotonic clock; deployment implications are noted in §3.

**Schema-level enforcement.** Each WOS record family's JSON Schema MUST pin a TypeID pattern on its `id` field (and any sibling `caseId` field), parameterized by the family's registered type-prefix. For example, `wos-provenance-record.schema.json` enforces `"pattern": "^[a-z][a-z0-9-]*_prov_[0-9a-hjkmnp-tv-z]{26}$"` on `id`. This catches TypeID violations at JSON Schema validation time — authoring tools, fixtures, and lint all fail early. The binding's JSON→dCBOR converter remains the final gate.

**Scope note.** A shared stack-wide TypeID utility (usable by Formspec Response IDs, WOS records, and Trellis bundle artifacts alike) is a follow-on ADR, not a blocker for this one. Current scope: WOS minting of TypeIDs at authoring time, in-spec format rules above.

### 2.5 Fields explicitly NOT in the WOS-authored surface

The following are **Trellis-owned** and MUST NOT be smuggled into the WOS-authored record as if they were WOS semantics:

- `ledger_scope`
- `sequence`
- `prev_hash`
- `canonical_event_hash`
- checkpoint references / sealing metadata
- anchor target / `anchor_refs`
- posture declaration digests

This ADR rejects a per-record WOS `anchorTarget` field. Anchor selection is a Trellis checkpoint / operator concern, not part of the authored WOS record. WOS says "this governance fact exists"; Trellis says "this canonical event was chained and later anchored under this operator posture."

**Inter-record references.** When one WOS record cites another — within the same case or across cases — the durable reference is the Trellis `canonical_event_hash` of the cited record, not a replicated `(caseId, recordId)` pair. `canonical_event_hash` is content-addressed (Trellis §9.2), survives WOS-side renames, and matches the hash-of-record contract (Trellis §23.2 item 4). WOS MAY additionally carry human-readable `(caseId, recordId)` for operator navigation but MUST treat `canonical_event_hash` as the authoritative identity of a cited record.

### 2.6 Posture transitions are dual-record events

When a WOS governance decision changes custody posture, the deployment emits **two** distinct records in order:

1. the authored `wos.*` governance record routed through `custodyHook`, then
2. the Trellis posture-transition canonical event that records the resulting Trellis-layer posture change.

The second MAY carry the first record's `canonical_event_hash` as its authorizing reference, per Trellis Operational Companion §24.10. The two facts MUST NOT be collapsed into one hybrid record.

**Atomicity posture.** The pair is not transactional. If step 1 succeeds and step 2 fails, the deployment holds a WOS governance fact that records the intent to change posture alongside a Trellis layer that has not yet transitioned. WOS retry machinery replays step 1 safely via the idempotency tuple; step 2 retries via Trellis's own idempotency construction. A WOS-Trellis deployment MUST surface a detectable condition — either a runtime alert or a deferred reconciliation job — when step 1 has committed and step 2 has not, because the two-record pair is the unit of semantic correctness even though it is not the unit of atomicity. Concrete detection mechanisms are deployment-specific; the MUST is that the gap does not go silent.

**Return direction.** The custodyHook return for the governance record (step 1) carries only step 1's `canonical_event_hash` per §2.8. WOS does not receive step 2's hash at the seam because the Trellis posture-transition event references step 1 — not vice versa — per Trellis Operational Companion §24.10. Audit queries that need the step-2 hash resolve it through Trellis's posture-transition-lookup API, outside the custodyHook contract. A future concrete consumer needing step-2 at the return site extends the receipt additively; no speculation now.

### 2.7 JSON→dCBOR conversion discipline

The JSON→dCBOR conversion crossing `custodyHook` is:

- **Mechanical.** No per-record-family code paths. One conversion function applied to any JSON-Schema-validated WOS record.
- **Deterministic.** The same logical record produces byte-identical dCBOR output across Rust and Python reference implementations. Byte-level ambiguity resolves per Trellis ADR 0004 (Rust wins; Python cross-check updates to match).
- **Schema-guided where necessary.** JSON has no integer / float distinction; the WOS JSON Schema's `"type": "integer"` vs. `"type": "number"` disambiguates the dCBOR major-type choice. Similar schema-driven disambiguation applies to binary fields (base64 strings → CBOR byte strings when the schema declares `contentEncoding: base64`) and date-time fields (RFC 3339 strings → tagged CBOR strings when the schema declares `format: date-time`).

The conversion algorithm is published in a WOS normative section (target: `specs/kernel/custody-hook-encoding.md` or an insertion into an existing kernel section — placement decided during acceptance). The reference implementation ships in a Trellis binding surface — either a new `wos-trellis-binding` sibling crate (mirroring the existing `wos-formspec-binding` pattern) or the current `wos-runtime::custody` module if the cross-tier split does not yet justify a new crate. Placement is an implementation decision at acceptance, not an ADR-pinned surface. A round-trip fixture corpus (`{record.json, record.dcbor, record.sha256}`) lands next to the implementation, one fixture per WOS record family that crosses `custodyHook`. Byte-match on both Rust and Python is the acceptance condition.

Export / inspection works the other direction: dCBOR → JSON via a reverse mapping that preserves typing. Audit tooling, export bundles, and human-readable chain-byte rendering all use this path; the chain remains dCBOR, the view stays JSON.

**Conversion-failure rejection list.** The converter rejects — does not silently encode — the following:

- JSON integers outside ±2^63−1.
- `NaN`, `+Infinity`, `-Infinity` in JSON numbers.
- Ill-formed UTF-8 in JSON strings (dCBOR already rejects).
- Values whose declared `format` is not in §2.2's encoding table and whose schema does not declare an `x-` vendor encoding rule.

Schemas tighten over time to catch most of these at authoring (defense-in-depth); the converter is the final gate. A rejection is a loud failure — no partial writes, no fallback to plain encoding.

**Size bound.** The authored-record dCBOR bytes MUST fit within Trellis's `PayloadInline` bounds (§6.4). WOS records are semantic governance facts — small by construction. Large evidence (attachments, ID photos, pay stubs, source documents) is not part of the authored fact and routes through the Evidence Integrity seam (open cross-layer contract per [STACK.md](../../../STACK.md)). Oversize records reject at the binding layer; there is no auto-switch to `PayloadExternal` for WOS authored bytes in 1.0.

### 2.8 Return contract

The `custodyHook` binding returns to the WOS runtime, at minimum:

| Field | Owner | Meaning |
|---|---|---|
| `canonical_event_hash` | Trellis | The Trellis §9.2 hash of the admitting event. This is the value WOS Kernel §8 requires for any hash-of-record field stamped into downstream provenance. |

Additional fields (`sequence`, `ledger_scope`, `anchor_refs` snapshot, admission timestamp) are refused at the return surface until a concrete WOS consumer site requires one and the shape is pinned in this ADR. The principle is symmetric to §2.3: narrow return surfaces are additively extensible; wide surfaces calcify on first use.

The return contract is part of the ADR because every WOS emission site that stamps `canonical_event_hash` into downstream provenance fossilizes the receipt's shape. Changing a returned field post-emission means rewriting stored provenance.

---

## 3. Consequences

### Positive

- The center/adaptor line becomes explicit: WOS owns authored record semantics; Trellis owns canonical append semantics.
- One byte oracle at the chain layer: Rust is the byte authority across both submodules (Trellis ADR 0004 extends cleanly).
- Trellis Core §23.5 now has a concrete WOS-side idempotency source tuple — two fields, structurally unique via TypeID / UUIDv7.
- The Workflow Governance Sidecar's `admitted_event_types[].wos_record_kind` field (Operational Companion Appendix B.2) has a pinned meaning (record-native, derivable from bytes).
- Narrow append-input + narrow return contract: the seam stays additively extensible. Every field on the wire survives because a caller forced it, not because a draft speculated.
- WOS authoring surface is unchanged. Tools, fixtures, exports, and §8.2.1 snapshots all keep JSON / JCS.
- No authorization surface at the seam. Operator-level auth composes with Kernel §10 `actorExtension` plus the STACK.md Actor-authorization open contract; this ADR does not duplicate or widen either.
- TypeID gives cross-tenant routing, time-ordered records, and self-describing identifiers for free. Same identifier works as WOS case id, durable-execution workflow id (Temporal workflow ID, Restate workflow ID), and provenance-store primary key — one identity, zero mapping.

### Negative

- Mechanical JSON→dCBOR conversion is new code surface — one converter with round-trip fixtures. Scope is bounded; complexity is concentrated.
- WOS publishes a normative encoding section (new prose). Small, one-time.
- `wos-runtime/src/custody.rs` — which landed against the 2026-04-21 JCS-based draft — needs to be rewritten to match the new wire shape, TypeID identifiers, and conversion discipline. Existing struct fields, tests, and the `serde_json_canonicalizer` dependency are all superseded.
- `trellis/fixtures/vectors/append/010-wos-custody-hook-state-transition` — the demonstration fixture wired against JCS bytes and the earlier 3-tuple — must be regenerated with dCBOR authored bytes, TypeID identifiers, and the 2-tuple. The fixture is outside the G-5 allowed-readset, so regeneration does not disturb the Phase-1 ratification corpus.
- Every WOS record family that crosses `custodyHook` needs a registered TypeID type-prefix. Small registry addition; bounded discipline.
- UUIDv7 construction requires a monotonic clock on the authoring host. Deployments without one (unusual) break TypeID guarantees.
- Large WOS authored records that exceed Trellis inline-payload bounds are rejected at the binding, not silently wrapped. Evidence-heavy workflows route evidence through the Evidence Integrity open contract, keeping WOS authored facts small.

### Neutral

- Spec-version bumps that change a record family's canonical dCBOR encoding are a migration event requiring a new provenance record marking the transition; they are not retry-safe replays. Trellis §17.5 `IdempotencyKeyPayloadMismatch` is the designated failure mode if a runtime accidentally attempts retry-across-version.

---

## 4. Follow-on work

This section captures the concrete work the ADR's acceptance unblocks. The cascade tracking is in §5 and [`work-spec/TODO.md`](../../TODO.md) Do-next #1.

1. **Land the JSON→dCBOR converter** (in `wos-runtime::custody` initially, or in a new `wos-trellis-binding` sibling crate — decision during acceptance) with a round-trip fixture per WOS record family that crosses `custodyHook` (Kernel Facts-tier, Governance, AI Integration, Assurance). Byte-match Rust + Python.
2. **Publish the WOS normative encoding section** pinning the §2.2 encoding table and §2.7 conversion-failure rejection list, with Rust as byte authority.
3. **Register TypeID type-prefixes** for each WOS record family in the WOS Extension Registry (§21): `case`, `prov`, `override`, `aigov`, `assurance` at minimum. Vendor-extension pattern follows ADR-0060 + `x-{vendor}-{kind}` under §21 discipline.
4. **Mint TypeIDs at authoring time** in `wos-core` / `wos-runtime`: every authored WOS record carries a TypeID `recordId` assigned when the record is first constructed, not synthesized downstream. Case creation mints the `caseId` TypeID.
5. **Rewrite `wos-runtime/src/custody.rs`** to the four-field wire shape. Drop `serde_json_canonicalizer` dependency. Keep a richer in-process Rust API (`CustodyAppendContext`, lifecycle correlation helpers) — it just does not appear on the wire.
6. **Regenerate `trellis/fixtures/vectors/append/010-wos-custody-hook-state-transition`** with dCBOR authored bytes. Update manifest `description`, `derivation.md`, and the `(caseId, recordId)` idempotency tuple with TypeID-shaped inputs. Replace `input-wos-record.jcs.json` with `input-wos-record.dcbor`.
7. **Pin the return-contract shape** with the first WOS consumer site (the Kernel §8 hash-of-record stamping path). Add `CustodyAppendReceipt` with `canonical_event_hash` only; extend only when a caller requires more.
8. **Update Trellis Operational Companion §24.9** to reference the new wire shape (if §24.9 currently names field lists — verify during acceptance).
9. **Open a shared-stack TypeID ADR** (optional): WOS, Formspec Response IDs, and Trellis bundle artifacts could all share a single TypeID utility crate. Not a blocker for this ADR; decide after first-implementation landings reveal whether the shared utility is worth the additional coordination.

---

## 5. Implementation cascade

Design decisions above are Accepted. Implementation lands through the cascade tracked in [`work-spec/TODO.md`](../../TODO.md) Do-next **#1** (T1.1–T1.8) and [`trellis/TODO.md`](../../../trellis/TODO.md) Stream 5. No further design signoff required for any cascade item; open new ADRs only if implementation discovers a load-bearing semantic the ADR didn't anticipate.

Summary of the cascade (full detail in WOS TODO):

1. Register TypeID type-prefixes (`case`, `prov`, `gov`, `ai`, `assurance`) in the WOS Extension Registry (§21).
2. Mint TypeIDs at WOS authoring time (`ProvenanceRecord` constructors, `WorkflowProcess::create`).
3. Tighten record-family JSON Schemas with TypeID patterns on `id` / `caseId`.
4. Land JSON→dCBOR converter with round-trip fixture corpus (byte-match Rust + Python).
5. Publish WOS normative encoding spec section (codifies §2.2 table + §2.7 rejection list + domain tag).
6. Rewrite `wos-runtime/src/custody.rs` to the four-field wire shape + TypeID identifiers.
7. Pin `CustodyAppendReceipt { canonical_event_hash }` at first WOS consumer (`#[non_exhaustive]`).
8. Verify Trellis-side state (`append/010` fixture, Operational Companion §24.9).

Trellis-side regeneration of `append/010` was completed externally on 2026-04-21 (dCBOR payload + TypeID identifiers + 2-tuple). Rust conformance replay against the new fixture is part of item 8.
