# ADR-0087: Studio persistence + projection model

**Status:** Proposed 2026-05-04
**Date:** 2026-05-04
**Deciders:** WOS Working Group (Studio sub-team)
**Author:** Studio authoring layer (Stage 7)
**Supersedes:** None
**Amends:** [`studio/specs/authoring-provenance.md`](../../studio/specs/authoring-provenance.md) (adds the persistence-port shape; record shape unchanged); [`studio/specs/workspace.md`](../../studio/specs/workspace.md) (adds the WorkingStore port shape).

**Related:**
- ADR 0086 (parent — reference architecture)
- ADR 0088 (sibling — AI extraction; defines AI-output recording for replay)
- ADR 0090 (sibling — publish/export boundary)

---

## 1. Context

[`studio/specs/reference-architecture.md`](../../studio/specs/reference-architecture.md)
§"Data ownership model" names six data stores with their owning
components. Three are **canonical immutable inputs** (Source
Vault, Authoring Ledger, recorded AI outputs); three are
**rebuildable projections** (Working Store, Policy Knowledge Map,
Retrieval Index); two are the **published boundary**
(ApprovalPackage, ExportBundle).

The replay/rebuild contract (`SA-MUST-arch-011`) specifies that
projections MUST be reconstructible from the ledger + Source Vault
+ recorded AI outputs + versioned parser/prompt/model/projection
metadata. This ADR pins:

1. The **port shapes** for the canonical and projection stores.
2. The **persistence model** (single Postgres database, content-
   addressed blobs, per-projection schemas).
3. The **AuthoringLedger signing primitive** (Ed25519 per actor).
4. The **replay-test contract** (Stage 8 deliverable).

A common alternative — "the audit ledger alone reconstructs
everything" — is **rejected** because AI outputs are
non-deterministic and parser/prompt/model versions are external
state; the ledger references but does not contain them. The replay
contract therefore names six inputs, not one.

## 2. Decision

### 2.1 Port shapes

| Port | Backing Stage 8 default | Notes |
|---|---|---|
| `SourceVault` | Filesystem (Stage 8); S3-compatible object store (Stage 9+) | Content-addressed; new versions = new blobs; never overwritten |
| `AuthoringLedger` | Postgres `studio_canonical.ledger` | Hash-chained, Ed25519-signed entries; append-only |
| `WorkingStore` | Postgres `studio_projections.workspace` | Mutable; rebuildable via replay |
| `PolicyKnowledgeMap` | Kuzu (candidate); Postgres recursive-CTE fallback | Mutable; rebuildable |
| `RetrievalIndex` | pgvector (Stage 8); dedicated vector DB (Stage 9+) | Mutable; rebuildable |

### 2.2 Persistence model

Stage 8 uses a **single Postgres database** with two named
schemas:

- `studio_canonical` — append-only, hash-chained tables for the
  Authoring Ledger; references to immutable Source Vault blobs.
- `studio_projections` — mutable tables for Working Store, Policy
  Knowledge Map (when Postgres-backed), and Retrieval Index
  metadata.

This **mirrors the parent `crates/wos-server`'s canonical /
projections schema discipline** (per
[`../../crates/wos-server/VISION.md`](../../crates/wos-server/VISION.md)
lines 96–101) without sharing tables. Studio and server each own
their own database; neither reads the other's tables.

Multi-tenant deployment (Stage 9+) gets one database per tenant,
matching the parent's per-tenant scaling commitment.

### 2.3 Authoring Ledger entry shape

`AuthoringProvenanceRecord` (existing; per
[`studio/specs/authoring-provenance.md`](../../studio/specs/authoring-provenance.md))
is the entry payload. The ledger storage row adds:

```text
ledger_seq           BIGSERIAL    PRIMARY KEY
prev_hash            BYTEA        NOT NULL  -- SHA-256 of previous row's stored bytes
record_hash          BYTEA        NOT NULL  -- SHA-256 of canonical-JSON(record)
actor_signature      BYTEA        NOT NULL  -- Ed25519 over (prev_hash || record_hash)
actor_key_id         TEXT         NOT NULL  -- public-key fingerprint; resolves via IdentityProvider
record               JSONB        NOT NULL  -- canonical-JSON serialization of AuthoringProvenanceRecord
recorded_at          TIMESTAMPTZ  NOT NULL
```

Hash-chain semantics:
- `prev_hash` of row N = SHA-256 of row N-1's `(prev_hash ||
  record_hash || actor_signature)` byte concatenation.
- Row 0's `prev_hash` is the workspace genesis hash (32 zero
  bytes for the slice; per-workspace-derived in production).
- Verifier recomputes the chain on read; mismatch fails the
  replay test.

### 2.4 Signing primitive: Ed25519 per actor

Each authoring actor (human reviewer, AI orchestrator, system
process) holds an Ed25519 keypair issued through the
`IdentityProvider`. Signing is per-row, not per-batch.

HMAC was rejected because it provides no nonrepudiation — a stolen
HMAC key produces signatures indistinguishable from legitimate
ones. Ed25519 keeps audit verifiability when keys eventually leak.

Key custody for Stage 8: file-backed keystore (dev-mode). Stage
9+: HSM/KMS-backed via the `KeyManager` port.

### 2.5 Replay / rebuild contract (Stage 8 test)

The Stage 8 replay test asserts:

```text
given:
  - the AuthoringLedger contents,
  - the Source Vault blobs referenced therein,
  - the recorded AI outputs (per ADR 0088),
  - the versioned parser/prompt/model/projection metadata
    (stored alongside each invocation in the ledger),
when:
  the Working Store, Policy Knowledge Map, and Retrieval Index
  are torn down and rebuilt by replaying the ledger,
then:
  the published ExportBundle is byte-identical to the
  pre-teardown bundle.
```

This is the load-bearing conformance test for `SA-MUST-arch-011`.

## 3. Rejected Alternatives

- **HMAC signing.** No nonrepudiation; rejected.
- **Per-batch signing.** Per-row catches single-entry tampering;
  per-batch admits a class of attacks that swap entries within a
  batch. Rejected.
- **Ledger-only replay (no recorded AI outputs).** Non-
  deterministic AI outputs cannot be reproduced; the ledger would
  reference outputs that no longer exist. Rejected.
- **Single mutable store (no projections / canonical split).**
  Loses the audit guarantee at the database layer; conflates
  "what was authored" with "what's currently shown." Rejected.
- **Separate Postgres clusters for canonical vs projections.**
  Operational overhead too high for Stage 8; one database with two
  schemas suffices and the database itself is the rebuild unit.
- **Trellis as Studio's ledger.** Studio's authoring events are
  not the same population as Trellis's case events; conflating
  them muddies the publication boundary. Studio publishes signed
  ExportBundles to Trellis; the Studio ledger remains internal.

## 4. Consequences

### Positive

- Replay invariant is testable from day one (Stage 8 deliverable).
- Audit verifiability holds even if signing keys are eventually
  compromised.
- Mirrors parent server's canonical/projections discipline without
  coupling.
- Migration path to per-tenant DB in Stage 9+ is straightforward.

### Negative

- Ed25519 per-actor key issuance requires an `IdentityProvider`
  with key-issuance capability — Stage 8 ships file-backed
  dev-mode; production hardening deferred.
- Rebuilding Policy Knowledge Map by replay can be expensive at
  scale; Stage 9+ may add incremental rebuild.

### Neutral

- The ledger row schema is Studio-internal; cross-spec consumers
  read `AuthoringProvenanceRecord` shape, not the Postgres row.

## 5. Conformance

- `SA-MUST-arch-010..011` in
  [`studio/specs/reference-architecture.md`](../../studio/specs/reference-architecture.md).
- Stage 8 replay test (per
  [`studio/specs/stage-8-vertical-slice.md`](../../studio/specs/stage-8-vertical-slice.md)
  deliverable 16).
- Boundary guard test (per ADR 0091).
