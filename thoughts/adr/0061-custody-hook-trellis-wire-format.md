# ADR-0061: `custodyHook` wire format for the Trellis binding

**Status:** Proposed  
**Date:** 2026-04-21  
**Deciders:** WOS + Trellis Working Group  
**Author:** WOS-T1 / Trellis Stream 5  
**Supersedes:** None  
**Related:**

- [TODO.md](../../TODO.md) — Do next #1 (`custodyHook` Trellis joint ADR)
- [Trellis TODO](../../../trellis/TODO.md) — Stream 5, WOS `custodyHook` joint ADR
- [Kernel §10.5 `custodyHook`](../../specs/kernel/spec.md)
- [Trellis Core §23, Composition with WOS `custodyHook`](../../../trellis/specs/trellis-core.md)
- [Trellis Operational Companion §24.9](../../../trellis/specs/trellis-operational-companion.md)
- [WOS provenance-record schema](../../schemas/kernel/wos-provenance-record.schema.json)
- [Mirrored Trellis-side note](../../../trellis/thoughts/specs/2026-04-21-trellis-wos-custody-hook-wire-format.md)

---

## 1. Context

The seam already exists on both sides:

- WOS Kernel §10.5 says `custodyHook` is where a deployment delegates custody semantics downstream.
- Trellis Core §23 says Trellis is that downstream layer for WOS-Trellis deployments and pins the Trellis-owned obligations: `wos.*` event-type namespace, `ledger_scope`, canonical append, idempotency, and posture transitions.

What is still missing is the **WOS-owned authored-record surface**. Trellis Core §23.5 is explicit that Trellis does not pin WOS field names for idempotency construction. Trellis Core §23.2 item 3 is explicit that Trellis wraps the bytes WOS produces, but WOS has not yet said what the stable authored bytes are, which identifiers travel with them, or which fields are WOS-owned versus Trellis-owned.

Without that joint decision, each runtime will invent its own append input and quietly calcify the seam.

---

## 2. Decision

### 2.1 Unit of admission

`custodyHook` admits **one authored WOS record per Trellis append**.

The authored WOS record is the thing WOS semantics define:

- a Kernel Facts-tier provenance record,
- a governance-sidecar record,
- an AI/governance/assurance record defined by a WOS companion.

Trellis wraps that authored record as one canonical event. It does NOT batch multiple WOS records into one append, and it does NOT split one authored WOS record across multiple canonical events.

### 2.2 Authored-byte authority

The WOS-authored payload routed through `custodyHook` is:

- the WOS-native record object,
- serialized as **UTF-8 JSON canonicalized with JCS (RFC 8785)**,
- with those bytes treated as the authored payload Trellis wraps.

This is a WOS content encoding decision, not a Trellis hash decision. Trellis still computes `canonical_event_hash` over its own dCBOR envelope per Trellis Core §9.2. The JCS bytes are the stable WOS-authored bytes inside that envelope.

Rationale:

- WOS is JSON-native.
- WOS already uses JCS for deterministic case-file snapshots in Kernel §8.2.1.
- A WOS-side canonical JSON payload gives one stable byte surface for export, diffing, and pre-append validation, without making WOS speak Trellis-native dCBOR internally.

### 2.3 WOS-owned append input

A WOS runtime routing a record through `custodyHook` MUST supply the following logical fields to the Trellis binding:

| Field | Owner | Meaning |
|---|---|---|
| `recordId` | WOS | Stable identifier for the authored WOS record. For Kernel Facts-tier records this is Kernel §8 `id`; for companion-layer records the owning WOS spec MUST define an equivalent stable id. |
| `eventType` | WOS registry / operator binding | Registered, outcome-neutral `wos.*` identifier for the record family admitted into Trellis. |
| `wosRecordKind` | WOS | WOS-native kind discriminator carried by the authored record family (for example Kernel `recordKind`). |
| `wosSpecVersion` | WOS | The WOS version whose semantics define the record bytes. |
| `recordSchemaRef` | WOS | URI identifying the schema or normative surface that validates the authored record. |
| `workflowRef` | WOS | URI of the governing WOS workflow or targeted kernel document. |
| `caseRef` | WOS | Stable case identifier / URI within the deployment. |
| `instanceRef` | WOS | Stable workflow-instance identifier / URI for retry and audit correlation. |
| `governanceEnvelopeRef` | WOS, optional | URI of the higher-level governance envelope / sidecar document this record came from when applicable. |
| `lifecycleRef` | WOS, optional | Structured pointer to the runtime moment that produced the record: `transitionId`, `stateId`, `eventName`, `taskPattern`, `taskId` as applicable. |
| `recordCanonicalJson` | WOS | The JCS-canonical UTF-8 JSON bytes of the authored record. |
| `recordDigestSha256` | WOS | Lowercase SHA-256 hex digest of `recordCanonicalJson`. This is a content digest for the WOS payload, not the Trellis `canonical_event_hash`. |

The Trellis binding MAY compute `recordDigestSha256` locally if the caller omits it, but when both are present they MUST match byte-for-byte. The authored WOS bytes are authoritative; the digest is an integrity convenience, not a substitute for the bytes.

### 2.4 Idempotency source tuple

For WOS-Trellis deployments, the WOS-owned stable source tuple for Trellis idempotency is:

`(caseRef, eventType, recordId)`

Normative consequences:

1. Retries of the **same** authored WOS record MUST preserve all three values.
2. A genuinely new authored WOS record MUST mint a new `recordId`.
3. WOS runtime internals MAY retry, replay, or compensate however they want; they MUST NOT mint a fresh tuple for the same authored fact.

Trellis remains free to encode or hash this tuple into its concrete `idempotency_key` bytes per Trellis Core §17 / §23.5. The tuple above is the WOS-owned semantic input.

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

### 2.6 Posture transitions are dual-record events

When a WOS governance decision changes custody posture, the deployment emits **two** distinct records in order:

1. the authored `wos.*` governance record routed through `custodyHook`, then
2. the Trellis posture-transition canonical event that records the resulting Trellis-layer posture change.

The second MAY carry the first record's `canonical_event_hash` as its authorizing reference, per Trellis Operational Companion §24.10. The two facts MUST NOT be collapsed into one hybrid record.

---

## 3. Consequences

### Positive

- The center/adaptor line becomes explicit: WOS owns authored record semantics; Trellis owns canonical append semantics.
- Trellis Core §23.5 now has a concrete WOS-side idempotency source tuple instead of "deployment-specific magic."
- The Workflow Governance Sidecar's `admitted_event_types[].wos_record_kind` field (Operational Companion Appendix B.2) has a pinned meaning.
- The seam stays narrow: no per-record anchor-target creep, no duplicate WOS-side hash chain, no fake hybrid object.

### Negative

- WOS runtimes that do not currently surface a stable `recordId` for every custody-routed record have follow-on work to do. This ADR intentionally chooses the stable architecture rather than preserving under-specified runtime behavior.
- Companion-layer records now need to define their `recordId` and canonical JSON surface explicitly if they want to route through `custodyHook`.

---

## 4. Follow-on work

1. Publish the WOS-side schema / type for this append input in runtime-facing surfaces once the `DurableRuntime` extraction begins.
2. Add at least one Trellis fixture exercising a `wos.*` append whose idempotency key is derived from `(caseRef, eventType, recordId)`.
3. Close the runtime drift between Kernel §8's required `id` field and any WOS runtime code paths that still stamp provenance without an explicit stable record id.
