---
title: WOS Custody Hook Encoding
version: 1.0.0
date: 2026-04-29
status: accepted
---

# WOS Custody Hook Encoding

**Version:** 1.0.0  
**Date:** 2026-04-29  
**Status:** Accepted  
**Companion to:** [WOS Kernel Specification](spec.md) §10.5  
**Related ADR:** [ADR-0061](../../thoughts/adr/0061-custody-hook-trellis-wire-format.md) (rationale only; this document is normative)

This document defines the WOS-owned authored-record surface that crosses the kernel `custodyHook` seam. It does not define Trellis canonical append semantics, anchor selection, checkpoint sealing, or receipt expansion beyond the first pinned WOS consumer.

The **JSON serialization** of the four-field append input (fixtures, tests, and
debug export) is described here and checked in-repo by
`wos-spec/tests/schemas/test_custody_hook_encoding.py` (inline Draft 2020-12
schema). Author-time anchoring posture for workflows lives under
[`../../schemas/wos-workflow.schema.json`](../../schemas/wos-workflow.schema.json)
`custody` (ADR 0076; the former standalone kernel custody JSON Schema was
removed). The live `custodyHook` seam itself carries raw dCBOR bytes in
`record`, not a base64 text wrapper.

---

## 1. Normative Contract

This section is normative.

### 1.1 Scope

This document governs one thing: how a WOS-authored record is prepared before it crosses the kernel `custodyHook` seam.

WOS owns:

- the authored record shape
- the four-field append input
- the `caseId` / `recordId` TypeID rules
- the `eventType` ownership rule
- the JSON→dCBOR conversion rules
- the WOS-owned idempotency input
- the minimum receipt field WOS consumes

WOS does NOT own:

- `ledger_scope`
- `sequence`
- `prev_hash`
- `canonical_event_hash` computation
- checkpoint references or sealing metadata
- anchor selection or `anchor_refs`
- Trellis posture-transition canonical events

Those remain Trellis concerns.

### 1.2 Unit Of Admission

`custodyHook` admits exactly one authored WOS record per append.

A processor MUST NOT batch multiple authored WOS records into one append input. A processor MUST NOT split one authored WOS record across multiple append inputs.

### 1.3 Four-Field Append Input

A WOS runtime routing a record through `custodyHook` MUST supply exactly these
four wire fields:

| Field | Meaning |
|---|---|
| `caseId` | TypeID-structured case identifier |
| `recordId` | TypeID-structured authored-record identifier |
| `eventType` | Outcome-neutral registered `wos.*` identifier for the authored record family |
| `record` | The authored WOS record rendered as dCBOR |

The wire surface is intentionally narrow. WOS runtimes MAY keep richer in-process correlation state, but they MUST NOT widen the seam by treating that state as required wire data.

When that four-field append input is represented as JSON for fixtures, tests,
or debug export, the `record` field is serialized as base64-encoded dCBOR
bytes. That JSON serialization is not a second wire format; it is a host
representation of the same seam artifact.

The following fields MUST NOT appear as WOS-owned wire obligations:

- `wosSpecVersion`
- `wosRecordKind`
- `workflowRef`
- `instanceRef`
- `lifecycleRef`
- `governanceEnvelopeRef`
- `recordDigestSha256`

If a binding surface exposes any of these for local convenience, it MUST treat them as in-process API fields, not seam fields.

### 1.4 TypeID Rules

`caseId` and `recordId` MUST use this format:

`{tenant}_{type}_{uuidv7_base32}`

Constraints:

- `{tenant}` MUST match `[a-z][a-z0-9-]*`.
- `{uuidv7_base32}` MUST be lowercase Crockford base32 and MUST match `[0-9a-hjkmnp-tv-z]{26}`.
- `{uuidv7_base32}` MUST decode to an RFC 9562 UUIDv7 value, not merely to a
  shape-conforming 26-character lowercase Crockford string.
- `caseId` MUST use type prefix `case`.
- `recordId` MUST use the family prefix registered for the authored record family.

Reserved WOS family prefixes:

| Prefix | Family |
|---|---|
| `case` | case instance |
| `prov` | kernel facts-tier provenance records |
| `gov` | governance runtime records |
| `ai` | AI runtime records |
| `assurance` | assurance runtime records |

One prefix maps to one top-level record family. Sub-kind discrimination lives in the authored record bytes, typically in `recordKind`, not in a finer-grained TypeID prefix.

Vendors extending the family set MUST use `x-{vendor}-{kind}` naming consistent with the Extension Registry's vendor-prefix discipline.

### 1.5 Event-Type Ownership

`eventType` MUST be a registered, outcome-neutral `wos.*` identifier owned by WOS.

Canonical form:

`wos.<layer>.<recordKind>`

Where `<layer>` is one of:

- `kernel`
- `governance`
- `ai`
- `assurance`

The WOS Extension Registry is the owning registry for `wos.*` event-type registrations and their associated family prefixes. Trellis-bound registries reference those WOS-owned identifiers at the declared WOS spec version.

### 1.6 JSON→dCBOR Conversion

The authored record crossing `custodyHook` MUST be authored as schema-valid JSON and converted mechanically to dCBOR at the seam.

The conversion MUST be deterministic. The same logical WOS record MUST produce byte-identical dCBOR output across conformant implementations. When a Rust/Python cross-check disagrees, Rust is the byte authority for the chain-binding surface and the cross-check MUST be updated to match.

The converter MUST apply exactly this encoding table:

| JSON Schema signal | dCBOR encoding |
|---|---|
| `type: integer` | CBOR major type 0 or 1; reject outside ±2^63−1 |
| `type: number` without `integer` | CBOR float64; reject `NaN`, `+Infinity`, `-Infinity` |
| `format: date-time` | CBOR tag 0 with RFC 3339 string payload |
| `format: uri` | CBOR tag 32 with URI string payload |
| `contentEncoding: base64` or binary media type | CBOR byte string |
| `type: string` otherwise | CBOR text string |
| `type: boolean` | CBOR true or false |
| `type: null` or JSON `null` | CBOR null |
| `type: array` | CBOR array with recursive element encoding |
| `type: object` | CBOR map with dCBOR canonical key ordering |

This table is closed. Adding a new format-to-tag mapping requires a normative spec change, not a converter-local shortcut.

### 1.7 Conversion Rejection List

The converter MUST reject, not silently coerce:

- integers outside ±2^63−1
- `NaN`
- `+Infinity`
- `-Infinity`
- ill-formed UTF-8
- values whose declared special format is not listed in §1.6 and does not declare an approved vendor encoding rule

A conversion failure MUST be loud. It MUST NOT produce a partial write. It MUST NOT fall back to a plain-text or JCS encoding path.

### 1.8 Size Posture

The authored `record` bytes MUST fit within Trellis's inline payload bound.

Oversized authored records MUST be rejected at the binding surface. A WOS runtime MUST NOT auto-promote oversized authored bytes to an external payload mechanism in 1.0.

Large evidence artifacts are outside this seam. They belong to the separate evidence-integrity contract.

### 1.9 WOS-Owned Idempotency Input

The WOS-owned semantic idempotency source tuple is:

`(caseId, recordId)`

Normative consequences:

- retries of the same authored fact MUST preserve both values
- a genuinely new authored fact MUST mint a new `recordId`
- WOS MUST NOT include `eventType` in the source tuple

For the Trellis binding, the domain-separation tag is:

`trellis-wos-idempotency-v1`

The bound input map is:

```json
{
  "caseId": "<caseId>",
  "recordId": "<recordId>"
}
```

That map is encoded as dCBOR and consumed by Trellis's concrete idempotency-key construction. WOS owns the semantic input. Trellis owns the concrete append-layer key bytes.

### 1.10 Receipt Contract

The `custodyHook` binding MUST return, at minimum:

| Field | Meaning |
|---|---|
| `canonical_event_hash` | Trellis hash of the admitting canonical event |

This is the first pinned WOS consumer field. WOS runtimes stamping a durable hash-of-record into downstream provenance MUST use this returned `canonical_event_hash`.

This return surface is narrow on purpose. WOS MUST NOT require `sequence`, `ledger_scope`, `anchor_refs`, or similar fields unless a later normative revision adds them.

### 1.11 Posture-Transition Pairing

When a governance decision changes custody posture, the deployment emits two distinct records:

1. the authored WOS governance record through `custodyHook`
2. the Trellis posture-transition canonical event

These facts MUST NOT be collapsed into one hybrid record.

The pair is semantically load-bearing even when it is not transactionally atomic. Deployments MUST surface a detectable reconciliation condition when the first record admits successfully and the second does not.

---

## 2. Composition

This section is normative.

### 2.1 Attachment Point

This document attaches at the kernel `custodyHook` seam defined in [spec.md](spec.md) §10.5.

It defines the WOS-owned authored-byte surface before Trellis canonical append semantics begin.

### 2.2 Precedence

When this document and a binding-specific implementation disagree about the authored-record surface, this document wins.

Binding implementations MAY add in-process helper fields or helper APIs. They MUST NOT change:

- the four required wire fields
- the TypeID rules
- the `wos.*` ownership rule
- the conversion table
- the rejection list
- the WOS-owned idempotency input
- the minimum receipt field

### 2.3 Conflict Handling

Conflicts resolve by rejection, not merge.

Examples:

- A record that fails TypeID validation MUST be rejected.
- A converter implementation that cannot encode a declared field under §1.6 MUST reject.
- A retry that changes `recordId` for the same authored fact is semantically invalid and MUST be rejected by the runtime before append.
- A binding that attempts to substitute JCS bytes for dCBOR at this seam is non-conformant.

### 2.4 Versioning And Migration

An additive helper field in a local runtime API is not a wire change.

Any change to these surfaces is a breaking seam change and requires a normative revision:

- removing or renaming a wire field
- changing the TypeID format
- changing a reserved family prefix
- changing the `eventType` ownership rule
- changing the conversion table
- changing the rejection list semantics
- widening the minimum receipt in a way WOS consumers depend on

When a spec revision changes the authored-byte outcome for a record family, retries across the old and new encoding are not safe replays. That migration MUST be treated as a new record event, not as an idempotent retry of the old one.

---

## 3. Conformance

This section is normative.

### 3.1 Schema Validation

Schema validation MUST check, at minimum:

- family-specific TypeID patterns on record-family `id` fields
- `case`-prefix TypeID patterns on sibling `caseId` fields where present
- any registry fixture or registry example carrying WOS-owned `wos.*` identifiers and reserved family prefixes

For the custody-hook append-input schema, a conformant validator MUST run the
schema with format-aware validation enabled so the `wos-case-typeid` and
`wos-record-typeid` formats enforce UUIDv7 semantics on the decoded TypeID
tail, not just regex shape.

Current gap:

- some emitted record families still lack split schema surfaces or TypeID enforcement. Until those schemas land, full family-wide closure is tracked by WOS-T1 T1.3.

### 3.2 Registry Validation

The extension-registry surface MUST publish:

- ownership of the `wos.*` event-type namespace
- reserved WOS family prefixes
- example entries covering `wos.kernel.*`, `wos.governance.*`, `wos.ai.*`, and `wos.assurance.*`

Current gap:

- the registry schema today catalogs seams, not a first-class event-family registration object. Until that shape is promoted, the minimum conformance bar is prose + example-fixture + regression-test coverage.

### 3.3 Runtime Conformance

Runtime conformance MUST check, at minimum:

- one authored record produces one append input
- append input shape is exactly the four required fields
- the converter emits dCBOR, not JCS JSON bytes
- conversion failures reject loudly
- retries preserve `(caseId, recordId)`
- downstream provenance stamps the returned `canonical_event_hash`

### 3.4 Byte-Authority Cross-Check

The reference implementation MUST ship a fixture corpus for each record family that crosses `custodyHook`:

- `record.json`
- `record.dcbor`
- `record.sha256`

Rust produces the authoritative bytes. A Python cross-check MUST match those bytes exactly.

Current runtime scope:

- the live `custodyHook` emitter set is the Kernel provenance-record family
- the shipped reference corpus therefore covers provenance-record fixtures today and MUST expand if additional WOS record families begin crossing `custodyHook`

The bound cross-stack ingestion fixture is
[`trellis/fixtures/vectors/append/010-wos-custody-hook-state-transition/`](../../../trellis/fixtures/vectors/append/010-wos-custody-hook-state-transition/),
which exercises one authored WOS record (`input-wos-record.dcbor`) → ADR-0061
`(caseId, recordId)` idempotency tuple (`input-wos-idempotency-tuple.cbor`) →
Trellis canonical envelope (`expected-event.cbor`), byte-exact, anchored to
TR-CORE-001/018/021/030/031/050/051/080. Sibling fixtures
`append/019-022` (`wos-signature-affirmation`, `wos-intake-accepted-*`,
`wos-case-created-*`) extend the cross-stack pattern across additional WOS
record families.

### 3.5 Trellis Verification

Trellis verification MUST confirm that the WOS `custodyHook` seam matches this document for the bound fixture set, including:

- dCBOR-authored payload
- TypeID `caseId`
- TypeID `recordId`
- `trellis-wos-idempotency-v1` domain separation
- the two-field WOS idempotency input

The verifier round-trip (envelope → dCBOR decode of `payload` → authored
record → byte-equal against `input-wos-record.dcbor`) is exercised by the
Trellis conformance replay against the bound fixture corpus. Running
`cargo nextest run -p trellis-conformance` validates byte-identity end-to-end for
every WOS record family in the corpus.
