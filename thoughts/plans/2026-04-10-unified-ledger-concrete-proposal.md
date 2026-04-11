# Unified Ledger: Concrete Proposal

**Date:** 2026-04-10
**Status:** Proposal
**Extends:** ADR-0059 (Unified Ledger as Canonical Event Store)
**Incorporates:** Expert panel corrections, technology survey, TPIF privacy model, architectural + cryptographic review findings
**Cost model:** Time is cheap. Development is cheaper. Processing is free. Tech debt is expensive. Build it right once.

---

## Principles

1. **The ledger starts in the browser.** The respondent had the data before the server did. Sovereignty is literal, not architectural.

2. **One implementation, two targets.** All crypto and ledger logic lives in Rust crates that compile to native (server) and WASM (browser). No TS reimplementation. No "simplified client version."

3. **Content-addressed encrypted blobs, stored anywhere.** The ciphertext hash IS the content address. The encrypted blob can live on the respondent's device, in Postgres, on IPFS, in S3 -- simultaneously. Where the ciphertext sits is a deployment knob, not an architectural decision.

4. **Permissioned sharing is immutable access events.** Granting access = wrapping existing DEKs for a new recipient and appending an `access.granted` event that commits the grant bundle. No payload re-encryption. No data movement. The platform sequences the grant; it does not rewrite history.

5. **No stubs, no phases, no "upgrade later."** Every "simple version now, real version later" is a migration. Migrations are tech debt. Build the real thing.

6. **The server is a processing node with permissioned access.** It caches encrypted blobs, decrypts the events wrapped to the ledger access key, runs materialized views, appends governance events, and maintains the canonical merged chain. It does not own the data.

---

## 1. Identity & Key Management

### The respondent's identity flow: OIDC + WebAuthn -> VC

The respondent never manages a key pair. They use their fingerprint.

```
Step 1: OIDC authentication
  Login.gov or ID.me -> IAL2 identity proof
  "This person is James Rodriguez"

Step 2: WebAuthn registration
  Browser creates a passkey in Secure Enclave / TPM
  Non-extractable by construction
  Biometric-gated (Face ID, fingerprint, PIN)
  Cross-device sync via iCloud Keychain / Google Password Manager

Step 3: Key derivation via WebAuthn PRF extension
  PRF salt is stored SERVER-SIDE (not secret -- security comes from
  the authenticator's internal HMAC key, not the salt).
  Server provides salt during authentication ceremony setup.

  On each authentication:
    WebAuthn PRF (hmac-secret, server-provided salt) -> deterministic 32-byte secret
    HKDF(secret, "formspec-ledger-ed25519-signing-v1")   -> Ed25519 signing key pair
    HKDF(secret, "formspec-ledger-x25519-encryption-v1") -> X25519 encryption key pair
  Same credential + same PRF salt = same derived keys, every time
  Keys exist only in memory during the session

  HKDF info strings are VERSIONED (v1 suffix). Adding future key types
  (e.g., ML-KEM for post-quantum) means adding new info strings under v2,
  without breaking v1 credentials.

  Server stores per credential:
    credential_id, public_key, sign_count, transports,
    prf_salt: [u8; 32],     -- NOT secret, stored cleartext
    hkdf_version: u8,       -- version of info strings used at registration

Step 4: DID derivation
  Ed25519 public key -> did:key:z6Mk...
  Deterministic: same passkey = same DID

Step 5: Verifiable Credential issuance
  VC {
    subject:  did:key:z6Mk... (from WebAuthn)
    issuer:   platform DID (or OIDC provider via adapter)
    claims:   { ial: 2, provider: "login.gov", name_hash: SHA-256(name) }
    proof:    Ed25519 signature by issuer
  }
  Stored in browser extension / PWA

Step 6: Recovery
  Respondent loses all devices
  -> Re-authenticate via OIDC (Login.gov proves they're still James Rodriguez)
  -> Create new WebAuthn credential with NEW PRF salt -> new DID
  -> Platform links old DID and new DID via OIDC identity proof
  -> Admin-initiated access re-grant: for each historical event,
     decrypt DEK via ledger access key, wrap with new DID public key,
     append immutable `access.granted` events carrying the new wrappings
  -> Old events now readable via new DID key
  -> New events use new DID key
  -> VC re-issued binding new DID to same IAL2 identity
  -> Old credential's key_id revoked from future grants
```

### Key hierarchy (corrected per expert panel)

```
Tenant Master Key (TMK)
  Cloud KMS (GovCloud for FedRAMP)
  Never exported
  Used ONLY for administrative operations (not for key derivation)
      |
      v
Ledger Access Key (LAK)                     Respondent's DID Key Pair
  Asymmetric X25519 keypair, one per        Derived from WebAuthn PRF
  ledger/case                               Lives only in browser memory
  Public key published to the client        Private key never leaves device
  Private key held in HSM/KMS or wrapped    Destroying this = respondent revocation
  under TMK/KMS KEK
  Destroying all private versions =
  platform-side crypto-shredding for
  this ledger
      |                                          |
      v                                          v
Per-Event Data Encryption Key (DEK)
  Random AES-256 key, generated per event
  Encrypts the event payload
  Wrapped (encrypted) by BOTH:
    LAK public key  ->  ledger_service_wrapped_dek  (stored in key bag)
    Respondent DID pubkey  ->  respondent_wrapped_dek  (stored in key bag)
  Additional wrappings for permissioned sharing:
    Any recipient's pubkey  ->  recipient_wrapped_dek
      (committed by later `access.granted` events)
  Plaintext DEK discarded immediately after wrapping
```

### Platform keys

```
Platform Signing Key (per deployment)
  Ed25519 key pair
  Signs governance events (wos.* events appended by the server)
  Signs checkpoints (COSE via coset)
  Public key published at well-known DID document endpoint

Platform Checkpoint Key (per deployment)
  COSE key for signed tree heads
  Used for Merkle tree checkpoints
  Public key in export artifact for offline verification
```

### Actor identity by type

| Actor | Identity mechanism | Signs events with |
|-------|-------------------|-------------------|
| Respondent | OIDC + WebAuthn -> DID | PRF-derived Ed25519 key |
| Respondent's delegate | OIDC + WebAuthn -> DID + delegation VC | Own PRF-derived Ed25519 key |
| Caseworker | Org SSO/SAML -> platform-issued DID | Platform-managed Ed25519 key |
| Supervisor | Org SSO/SAML -> platform-issued DID + authority VC | Platform-managed Ed25519 key |
| AI agent | Model ID + version + invocation ID | Platform signing key (system actor) |
| System | Component ID + version | Platform signing key |
| Support agent | Org SSO/SAML + JIT approval chain | Platform-managed Ed25519 key |
| External service | Service identifier + idempotency key | Service signing key (verified by platform) |

---

## 2. Event Data Model

### Event structure: author event + canonical receipt

```
+--------------------------------------------------------------+
|  AUTHOR EVENT ENVELOPE v2 (plaintext, actor-signed)          |
|                                                               |
|  -- Actor-authored, immutable, causally ordered --           |
|  version:            u8     event format version (= 2)        |
|  hlc:                HLC    Hybrid Logical Clock               |
|    wall_ms:          u64    wall-clock ms (coarsened)          |
|    logical:          u32    monotonic logical counter          |
|    device_id:        [u8; 8]  truncated hash of device pubkey  |
|  payload_plaintext_commitment: [u8; 32]                       |
|                     SHA-256 of PLAINTEXT CBOR                 |
|  payload_ciphertext_id: [u8; 32]  SHA-256 of ciphertext       |
|                                                               |
|  -- Identity & governance (immutable plaintext metadata) --   |
|  actor_type:         u8     enum (respondent, caseworker...)  |
|  event_type:         u16    index into event type registry    |
|  privacy_tier:       u8     (anonymous, pseudo, id'd, full)   |
|  signing_key_id:     [u8; 16]  identifies signer's key        |
|  signature:          [u8; 64]  Ed25519 over canonical header  |
|  governance_result:  u8     (pass/fail/na for deontic checks) |
|                                                               |
|  -- Privacy-preserving (immutable commitments only) --        |
|  tag_commitment:     [u8; 32]  SHA-256(tag_bitfield || nonce) |
|    Nonce stored INSIDE encrypted payload, not in header.      |
|    Verifiers who need tags must decrypt to obtain nonce.       |
|  commitment_count:   u8     FIXED per event_type (schema)     |
|  commitments:        [PedersenCommitment; commitment_count]   |
|    each: 32 bytes (compressed Ristretto point)                |
|    Fixed-position vector: unused fields get commitment-to-zero|
|    with random blinding, indistinguishable from real values.  |
|                                                               |
|  -- Causal ordering (immutable merge metadata) --             |
|  causal_dep_count:   u8     max 8 dependencies per event      |
|  causal_deps:        [CausalDep; causal_dep_count]            |
|    each: author_event_hash [u8; 32] + HLC (20 bytes) = 52    |
|                                                               |
|  This envelope is signed by the author and NEVER rewritten.   |
+--------------------------------------------------------------+

+--------------------------------------------------------------+
|  CANONICAL RECEIPT v1 (plaintext, sequencer-signed)           |
|                                                               |
|  sequence:           u64    server-assigned total order       |
|  canonical_prev_receipt_hash: [u8; 32]                        |
|                     links receipts into one total-order chain |
|  author_event_hash:  [u8; 32]  binds receipt to author event  |
|  ingest_mode:        u8     strict | relaxed                  |
|  verification_state: u8     verified_payload | ciphertext_only|
|  merge_result:       u8     linearized | explicit_merge       |
|  sequencer_key_id:   [u8; 16]                                 |
|  sequencer_signature:[u8; 64]  signature over receipt bytes   |
+--------------------------------------------------------------+

+--------------------------------------------------------------+
|  FOUR HASHES (different purposes, all mandatory)              |
|                                                               |
|  Hash 1: payload_plaintext_commitment (IN the header)         |
|    = SHA-256(plaintext_cbor)                                  |
|    Purpose: bind the event to specific plaintext content.     |
|    Computed BEFORE encryption.                                |
|                                                               |
|  Hash 2: payload_ciphertext_id (IN the header)                |
|    = SHA-256(ciphertext)                                      |
|    Purpose: content address of the encrypted blob.            |
|    Used for blob lookup, export packaging, and sync checks.   |
|                                                               |
|  Hash 3: author_event_hash                                    |
|    = SHA-256(                                                 |
|        "formspec-ledger-author-event-hash-v1" ||              |
|        len(envelope_bytes) || envelope_bytes ||               |
|        len(ciphertext)   || ciphertext   ||                   |
|        len(key_bag_cbor) || key_bag_cbor                      |
|      )                                                        |
|    Purpose: integrity of the exact author-authored event.     |
|    Computed AFTER all construction steps.                      |
|                                                               |
|  Hash 4: receipt_hash                                         |
|    = SHA-256(                                                 |
|        "formspec-ledger-receipt-hash-v1" ||                   |
|        len(receipt_bytes) || receipt_bytes                    |
|      )                                                        |
|    Purpose: integrity of canonical sequencing and server-side |
|    verification state. canonical_prev_receipt_hash references |
|    receipt_hash of the predecessor receipt. Merkle tree       |
|    leaves are receipt_hashes.                                 |
|                                                               |
|  Domain separation per NIST SP 800-185.                       |
|  Length prefixing prevents component boundary ambiguity.       |
+--------------------------------------------------------------+

+--------------------------------------------------------------+
|  PAYLOAD (encrypted blob, content-addressed)                  |
|                                                               |
|  Deterministic CBOR encoding of:                              |
|    actor_id, field values, rationale, evidence,               |
|    confidence details, document content,                      |
|    delegation chain, equity dimensions,                       |
|    tag_nonce: [u8; 16],          -- for tag_commitment verify |
|    tag_bitfield: u16,            -- actual tags               |
|    commitment_blinding_factors   -- for Pedersen opening      |
|                                                               |
|  Encrypted: AES-256-GCM(plaintext_cbor, DEK, nonce)          |
|  Nonce rule: fresh random 96-bit nonce per payload encryption |
|  Never reuse a nonce under the same DEK.                      |
|  Content-addressed: content_id = SHA-256(ciphertext)          |
|  Stored in: any content-addressed blob store                  |
+--------------------------------------------------------------+

+--------------------------------------------------------------+
|  KEY BAG (per-event, extensible, stored with header)          |
|                                                               |
|  entries: [                                                   |
|    { recipient: "ledger-service", ledger_key_version: 1,      |
|      ephemeral_pubkey: [...],  wrapped_dek: [...] },          |
|    { recipient: "respondent",                                 |
|      ephemeral_pubkey: [...],  wrapped_dek: [...] },          |
|  ]                                                            |
|                                                               |
|  Each HPKE/X25519 wrapping uses a FRESH ephemeral keypair.    |
|  Ephemeral keypair is consumed (moved) by the wrap operation. |
|  Historical event key bags never mutate after append.         |
|  Additional recipients are handled by later access events.    |
|  Ledger-service entries include ledger_key_version.           |
+--------------------------------------------------------------+

+--------------------------------------------------------------+
|  DISCLOSURE ATTESTATION (optional, separate artifact)         |
|                                                               |
|  Core ledger events are signed only with Ed25519.             |
|  Selective disclosure uses a separate issuer-backed artifact  |
|  minted AFTER strict ingest, over the target author_event_hash|
|  and the                                                     |
|  plaintext field vector.                                      |
|  This keeps offline intake independent of issuer private keys.|
|  BBS+/SD-JWT proofs derive from the disclosure attestation,   |
|  not from the core event itself.                              |
+--------------------------------------------------------------+
```

The canonical ledger is therefore one author-event DAG plus one canonical receipt chain. Actors sign author event envelopes. The server sequences those envelopes by issuing immutable canonical receipts. Total order is a receipt property, not an actor-authored field.

### Hybrid Logical Clock (HLC)

Replaces the bare `timestamp: u64` from v1 headers. Combines wall-clock with logical counter to give causal ordering across devices without full vector clocks (Kulkarni et al. 2014, used by CockroachDB and TiDB).

```
HLC {
  wall_ms: u64    -- wall-clock milliseconds (coarsened to prevent fingerprinting)
  logical: u32    -- incremented when wall clock hasn't advanced
  device_id: [u8; 8]  -- truncated hash of device's WebAuthn credential public key
}

Tick protocol:
  On event creation:
    new_wall = max(now_ms(), local_hlc.wall_ms, latest_received_hlc.wall_ms)
    if new_wall == local and new_wall == received:
      logical = max(local.logical, received.logical) + 1
    elif new_wall == local:
      logical = local.logical + 1
    elif new_wall == received:
      logical = received.logical + 1
    else:
      logical = 0

  On sync (receiving canonical chain):
    Merge local HLC with server's latest HLC before creating new events.
```

### Causal dependencies

Each author event carries up to 8 explicit causal dependency references. The server uses these plus HLC to build a closed DAG before issuing canonical receipts.

```
CausalDep {
  author_event_hash: [u8; 32]  -- full hash of depended-upon author event
  hlc: HLC                     -- HLC of the depended-upon event
}

Server sequencing:
  1. Build a closed DAG from satisfiable causal_deps
  2. Topological sort; ties broken by HLC (wall_ms, then logical),
     then lexicographic author_event_hash
  3. Detect concurrent conflicts on overlapping fields
  4. For non-conflicting events: issue canonical receipts in sorted order
  5. For conflict-sensitive overlaps: require explicit `ledger.merge`
     resolution before sequencing can continue past that frontier

FEL-calculated fields are always conflict-sensitive because their
causal chain matters. A stale auto-calculate overwriting a manual
entry is a due process issue in Medicaid adjudication.
```

Using the full 32-byte `author_event_hash` in each dependency is intentional. This removes collision ambiguity from the normative merge path; bandwidth optimization, if needed later, belongs in transport compression rather than the on-disk/on-wire dependency identifier.

### Tag commitment (privacy fix)

Tags like `determination` and `adverse-decision` are NO LONGER plaintext. Knowing "this person received an adverse Medicaid determination on March 15" from headers alone is a HIPAA-relevant disclosure.

```
Header contains:    tag_commitment =
                      SHA-256(dCBOR([tag_bitfield: u16, tag_nonce: bstr(16)]))
Payload contains:   tag_nonce: [u8; 16], tag_bitfield: u16

Verification (by authorized party who decrypted the payload):
  expected = SHA-256(dCBOR([decrypted.tag_bitfield, decrypted.tag_nonce]))
  assert constant_time_eq(header.tag_commitment, expected)

Projection queries needing tag-based filtering:
  Option 1: Decrypt payload, filter in application code
  Option 2: Maintain encrypted tag index per projection
            (AES-256-GCM encrypted tag bitfield, keyed by
            a projection-specific key managed alongside the LAK)
```

### Fixed-position Pedersen commitments (privacy fix)

Commitment count and positions are FIXED per event_type. Observers still learn the event type and the fixed slot count from the envelope. They do NOT learn which slots correspond to populated business values, nor the committed values themselves, without valid openings or additional side information.

```
EventTypeCommitmentSchema {
  event_type: u16,
  field_positions: [                     -- ordered, published
    { field_path: "income.monthly",    generator_index: 0 },
    { field_path: "income.assets",     generator_index: 1 },
    { field_path: "income.deductions", generator_index: 2 },
    ...
  ]
}

Every event of a given type produces exactly len(field_positions) commitments.
Unused fields get commitment-to-zero with random blinding factor:
  C = 0 * G_i + r * H   (indistinguishable from real commitments)

Blinding factors stored in encrypted payload (not in header).
Commitment schemas are versioned -- adding a field to an event type
requires a new schema version (append-only; old events keep old count).

Example: event type 0x0042 (income determination) has 8 fields.
  Every income determination event: 8 × 32 = 256 bytes of commitments.
  Observer sees 8 commitments, learns nothing about which fields filled.
```

### What stays plaintext (header) vs. what gets encrypted (payload)

| Data | Location | Reason |
|------|----------|--------|
| Event type (`wos.task.completed`) | Header | Structural verification without decryption |
| Timestamp | Header | Temporal ordering must be verifiable |
| Actor type (`human`, `agent`, `system`) | Header | AI disclosure is structural (OMB M-24-10) |
| Schema version | Header | Forward compatibility |
| Hash chain references | Header | Verification requires these |
| Governance check result (pass/fail) | Header | Deontic compliance is structural |
| Tag commitment `SHA-256(tags \|\| nonce)` | Header | Hash-committed: verifiers with payload access can check; observers learn nothing |
| Tag bitfield + nonce | Payload | Actual tags readable only after decryption |
| Pedersen commitments (fixed-position vector) | Header | Aggregation without decryption; count fixed per event type |
| Pedersen blinding factors | Payload | Required for opening commitments; only visible to key holders |
| Actor identity (`angela.martinez`) | Payload | PII |
| Case file field values | Payload | PII/PHI |
| Determination rationale | Payload | Case-specific reasoning |
| Document content and attachments | Payload | Uploaded pay stubs, medical records |
| Confidence details (per-field scores) | Payload | May reveal case complexity |
| Equity monitoring dimensions | Payload | Demographic data is sensitive |
| Delegation chain details | Payload | Who delegated to whom may be sensitive |

### Canonical serialization (expert panel critical fix)

All structured data serialized via **deterministic CBOR** (RFC 8949 Core Deterministic Encoding):
- Map keys sorted lexicographically
- Minimal-length integer encoding
- No indefinite-length items

Crate: `ciborium` with deterministic mode, or `minicbor` for no-std/WASM.

Hash inputs are always over the CBOR-encoded bytes, never over concatenated strings. This eliminates the domain separation / parsing ambiguity the cryptographer flagged.

---

## 3. Cryptographic Stack

Every crate compiles to both native and `wasm32-unknown-unknown`.

| Component | Crate | Purpose | WASM? |
|-----------|-------|---------|-------|
| SHA-256 hashing | `ring` | Hash chain, content addressing, Merkle leaves | Yes |
| Ed25519 signing | `ed25519-dalek` | Per-event signatures (respondent + platform) | Yes |
| AES-256-GCM | `ring` | Payload encryption (per-event DEK) | Yes |
| COSE checkpoint signing | `coset` (Google) | Signed tree heads at checkpoints | Yes |
| Pedersen commitments | `curve25519-dalek` (Ristretto) | Homomorphic aggregation over encrypted numerics | Yes |
| Merkle tree | `ct_merkle` | RFC 6962 history tree, inclusion/consistency proofs | Yes |
| BBS+ signatures | `bbs_plus` | Issuer-backed disclosure attestations + selective proofs | Yes (pairing-friendly curves compile to WASM) |
| SD-JWT | `sd-jwt` or custom | Disclosure-attestation fallback for IETF-only environments | Yes |
| HKDF key derivation | `hkdf` + `sha2` | Derive signing/encryption keys from WebAuthn PRF | Yes |
| X25519 key agreement + ECIES | `x25519-dalek` | Respondent encryption key pair + DEK wrapping | Yes |
| Deterministic CBOR | `ciborium` or `minicbor` | Canonical serialization for all structured data | Yes |
| OpenTimestamps | HTTP client (REST API) | Bitcoin-anchored timestamps for checkpoints | Server only |

### WASM CI gate

CI MUST build and test `ledger-engine` on `wasm32-unknown-unknown` with the exact primitives required for browser mode: SHA-256, AES-256-GCM, Ed25519, X25519, deterministic CBOR, and Merkle verification. If `ring` does not meet that gate on `wasm32-unknown-unknown`, the approved fallback is `sha2` plus `aes-gcm` for WASM only, hidden behind the same Rust API surface in `ledger-engine`.

### Browser disclosure feasibility gate

BBS+ is not locked into the browser path until a spike demonstrates all of the following on a mid-tier phone:

- Added compressed bundle size `<= 350 KB`
- Cold start overhead `<= 300 ms`
- Proof generation for `<= 64` messages `<= 1.5 s`
- Peak memory `<= 64 MB`

If the spike misses any gate, browser mode falls back to server-issued proofs only or to `SD-JWT`-only deployments, while the core ledger format remains unchanged.

### ECIES key wrapping invariant

Every DEK wrapping operation MUST use a fresh ephemeral X25519 keypair. This applies to initial event creation (wrapping for respondent + ledger-service) AND post-hoc access grants. The ephemeral keypair is generated, used once to wrap a single DEK for a single recipient, then destroyed.

The type system enforces this: the `EphemeralX25519Keypair` type is `!Clone + !Copy` and is consumed (moved) by the single `wrap_dek()` call. Reusing an ephemeral key is a compile error, not a runtime bug. Each key bag entry stores the 32-byte ephemeral public key so the recipient can perform the ECDH derivation.

### Checkpoint cycle

```
Every N events (configurable, default 100):
  1. Compute Merkle root of all receipt_hash leaves since last checkpoint
     (ct_merkle append-only tree, RFC 6962)
  2. Build signed tree head:
     COSE_Sign1(coset) {
       payload: { tree_size, root_hash, timestamp }
       key: platform checkpoint key
     }
  3. Submit root_hash to OpenTimestamps calendar server
     -> returns pending OTS proof
     -> OTS proof completes when Bitcoin block confirms (~10 min)
  4. Append ledger.checkpoint event with:
     signed_tree_head, ots_proof (pending or confirmed)
  5. Store checkpoint as epoch snapshot for view rebuild
```

---

## 3b. Key Rotation Protocol

Key rotation is a mandatory operational requirement for NIST 800-57 compliance (crypto-period management, Section 5.3) and a FedRAMP blocker if absent.

### Ledger Access Key (LAK) rotation (lazy re-wrap)

New LAK version is generated for the ledger. The old private key is marked `Rotating`, not destroyed. New events wrap DEKs to the new public key immediately. Historical events are lazily re-wrapped in the background. Old private key material is destroyed only after the sweep completes.

```
LAK lifecycle states:
  Active -> Rotating { new_lak_version, sweep_progress }
         -> PendingDestruction { superseded_by }
         -> Destroyed { destroyed_at }

Sweep protocol:
  Process events in batches (typically 100).
  Rate-limited to avoid KMS throttling (e.g., 500 req/s per rotation).
  For each event:
    1. Unwrap DEK with old LAK private key
    2. Re-wrap DEK with new LAK public key
    3. Rewrite the active grant projection entry or append a new
       immutable grant bundle, depending on access mode
    4. Zeroize plaintext DEK immediately
  Resume from last_processed_sequence on restart.

On-read lazy re-wrap:
  If a read encounters an old-LAK wrapping during rotation,
  opportunistically re-wrap (piggyback on the read's KMS call).
  This reduces sweep time for frequently-accessed events.

KMS call budget (typical case):
  1,000 events per ledger × 2 unwrap/re-wrap operations = 2,000 ops
  At 500 req/s rate limit: 4 seconds per ledger
  AWS KMS cost: ~$0.06 per 2,000 calls = negligible
  Old LAK private key scheduled for deletion (7-30 day waiting period) after sweep.
```

### Disclosure issuer key rotation

Disclosure keys are tied to the issuer (system-wide or per-tenant). Rotation means new disclosure attestations use the new key; old attestations remain verifiable against their original key version.

```
BBSKeyVersion {
  version: u32,
  public_key: BBSPublicKey,
  wrapped_private_key: Vec<u8>,   // KMS-wrapped
  kms_key_id: KmsKeyId,
  valid_from: u64,                // timestamp
  valid_until: Option<u64>,       // None = current
  revoked: bool,
}

Disclosure artifacts carry `bbs_key_version` so verifiers look up the correct key.
Export artifact's `bbs_public_keys.cbor` is a versioned key list.

Verification:
  1. Look up key version from the disclosure attestation
  2. Check key wasn't revoked
  3. Verify attestation timestamp falls within key's valid period
  4. Verify BBS+ attestation against that key version's public key

Selective disclosure proofs include bbs_key_version so verifiers
can look up the correct public key independently.
```

### TMK rotation

TMK's role is administrative. It protects ledger private key material, mapping keys, and other tenant-held secrets. TMK rotation = rotate the KEK or policy layer protecting those secrets.

```
For each LAK or mapping key protected by the TMK:
  Re-wrap or re-authorize under the new TMK policy.
Schedule old TMK deletion (90-day waiting period).
Infrequent operation (annual or on compromise).
```

### LAK public key rollout

```
LedgerKeyRegistry (append-only versioned key list per ledger):

  LedgerKeyEntry {
    version: u32,
    public_key: X25519PublicKey,
    private_key_ref: KeyHandle,
    status: Active | Deprecated { grace_deadline } |
            RetiredForDecryptionOnly | Revoked { reason },
  }

Lifecycle:
  Active: preferred key, returned in sync responses.
  Deprecated: still accepted, but clients should transition.
              Grace period typically 30 days.
  RetiredForDecryptionOnly: no longer accepted for new events,
              still usable for decryption of historical events.
              Server re-wraps or projects historical grants forward.
  Revoked (compromise): hard-reject events, disable decryption.

Client behavior:
  Fetches the current ledger public key during session setup.
  Sync response always includes current key version + pubkey.
  Client updates cached key on next sync cycle.
  Offline clients that sync after weeks still work (grace period).
```

### Active session handling during rotation

```
Sessions in flight during rotation must not lose work.
Session tokens carry the LAK version
they were established with.

SyncResponse includes:
  lak_rotation_advisory: { new_lak_version, old_accepted_until }
  current_ledger_pubkey_version: u32

Server accepts events wrapped under any non-revoked key version.
Only compromised (Revoked) keys are hard-rejected.
```

---

## 4. Storage Topology

### Content-addressed blob store interface

```rust
#[async_trait]
pub trait BlobStore: Send + Sync {
    /// Store an encrypted payload by its content hash.
    async fn put(&self, content_id: ContentId, ciphertext: &[u8]) -> Result<()>;

    /// Retrieve an encrypted payload by its content hash.
    async fn get(&self, content_id: ContentId) -> Result<Vec<u8>>;

    /// Check if a blob exists without retrieving it.
    async fn exists(&self, content_id: ContentId) -> Result<bool>;
}

/// ContentId = SHA-256 of the ciphertext. This IS the content address.
pub struct ContentId([u8; 32]);
```

### Backends

| Backend | Implementation | Best for |
|---------|---------------|----------|
| `PostgresBlobStore` | `BYTEA` column keyed by content hash | Server-side cache, simple deployments |
| `S3BlobStore` | S3 object keyed by hex(content_hash) | GovCloud, FedRAMP, durable government storage |
| `IpfsBlobStore` | HTTP API to IPFS node or pinning service (Pinata, web3.storage) | Decentralized durability, platform-independent |
| `OpfsBlobStore` | Origin Private File System (browser) | Client-side, offline, fast |
| `IndexedDbBlobStore` | IndexedDB (browser) | Client-side, universal browser support |

Multiple backends may be active simultaneously. One durable backend is designated the primary store for acceptance semantics; any others are replicas.

### Replication semantics

- Accept an event only after the primary blob write, the author-event row, and the canonical-receipt row all succeed.
- Secondary blob writes are best-effort replicas. On partial failure, mark the event `blob_replication_state = pending` and retry asynchronously.
- Reads use first-success order: local cache, primary durable store, then replicas.
- If a read succeeds from a replica while the primary is missing the blob, schedule backfill to the missing store.
- A replica outage MUST NOT cause canonical receipt disagreement. Sequencing depends on the author event and receipt stores, not on replica completion.

### Author event store + canonical receipt store

Separate from blobs. The server persists immutable author events and immutable canonical receipts as distinct append-only records. PostgreSQL 11 is the minimum version for the trigger syntax below; PostgreSQL 14+ is the recommended operational floor.

| Location | Technology | Purpose |
|----------|-----------|---------|
| Server | Postgres append-only tables with UPDATE/DELETE trigger | Author-event persistence + canonical receipt chain |
| Client | IndexedDB (structured) or OPFS (binary) | Local author-event store + cached receipts |

The Postgres tables:

```sql
CREATE TABLE ledger_author_events (
    ledger_id          UUID NOT NULL,
    author_event_hash  BYTEA NOT NULL,      -- SHA-256 over envelope+ciphertext+key_bag
    envelope_bytes     BYTEA NOT NULL,      -- immutable actor-signed envelope
    payload_cid        BYTEA NOT NULL,      -- content address of encrypted payload
    key_bag_cbor       BYTEA NOT NULL,      -- CBOR-encoded key bag
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (ledger_id, author_event_hash)
);

CREATE TABLE ledger_receipts (
    ledger_id                     UUID NOT NULL,
    sequence                      BIGINT NOT NULL,
    author_event_hash             BYTEA NOT NULL,
    receipt_bytes                 BYTEA NOT NULL,
    receipt_hash                  BYTEA NOT NULL,
    canonical_prev_receipt_hash   BYTEA NOT NULL,
    ingest_mode                   TEXT NOT NULL CHECK (ingest_mode IN ('strict', 'relaxed')),
    verification_state            TEXT NOT NULL CHECK (verification_state IN ('verified_payload', 'ciphertext_only')),
    created_at                    TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (ledger_id, sequence),
    UNIQUE (ledger_id, author_event_hash),
    UNIQUE (ledger_id, receipt_hash),
    FOREIGN KEY (ledger_id, author_event_hash)
      REFERENCES ledger_author_events(ledger_id, author_event_hash),
    CONSTRAINT receipt_chain_integrity CHECK (
      (sequence = 0 AND canonical_prev_receipt_hash = decode(repeat('00', 32), 'hex')) OR
      (sequence > 0 AND canonical_prev_receipt_hash <> decode(repeat('00', 32), 'hex'))
    )
);

-- Prevent mutation
CREATE OR REPLACE FUNCTION prevent_ledger_mutation()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'ledger_events is append-only: % not permitted', TG_OP;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER no_author_event_update BEFORE UPDATE ON ledger_author_events
    FOR EACH ROW EXECUTE FUNCTION prevent_ledger_mutation();
CREATE TRIGGER no_author_event_delete BEFORE DELETE ON ledger_author_events
    FOR EACH ROW EXECUTE FUNCTION prevent_ledger_mutation();
CREATE TRIGGER no_receipt_update BEFORE UPDATE ON ledger_receipts
    FOR EACH ROW EXECUTE FUNCTION prevent_ledger_mutation();
CREATE TRIGGER no_receipt_delete BEFORE DELETE ON ledger_receipts
    FOR EACH ROW EXECUTE FUNCTION prevent_ledger_mutation();
```

### Key bag store and access-grant projections

Base key bags are stored with the immutable event record (in `key_bag_cbor`). Post-hoc sharing is NOT a mutable update to historical events. It is source-of-truth in append-only `access.granted` and `access.revoked` events.

For query speed, the server maintains a derived projection table that expands grant bundles into recipient/event rows:

```sql
CREATE TABLE access_grant_entries (
    ledger_id       UUID NOT NULL,
    grant_event_sequence BIGINT NOT NULL,    -- source-of-truth event
    target_sequence BIGINT NOT NULL,         -- event this grant unlocks
    recipient_id    TEXT NOT NULL,           -- DID or role identifier
    ephemeral_pubkey BYTEA NOT NULL,         -- required for HPKE/X25519 unwrap
    wrapped_dek     BYTEA NOT NULL,          -- DEK encrypted with recipient's public key
    expires_at      TIMESTAMPTZ,
    revoked_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (ledger_id, grant_event_sequence, target_sequence, recipient_id),
    FOREIGN KEY (ledger_id, grant_event_sequence) REFERENCES ledger_receipts(ledger_id, sequence),
    FOREIGN KEY (ledger_id, target_sequence) REFERENCES ledger_receipts(ledger_id, sequence)
);
```

Granting access to a new party for a range of events = append an `access.granted` event whose payload contains an immutable grant bundle. The projection table above is rebuildable from the ledger. No payload re-encryption. No payload movement.

### Deployment tiers

| Tier | Payload backends | Key plane | Header chain |
|------|-----------------|-----------|-------------|
| **Shared Cloud** | Postgres + respondent device | Cloud KMS/HSM-protected LAK + respondent WebAuthn | Postgres + OpenTimestamps |
| **Shared + IPFS** | IPFS + Postgres cache + respondent device | Cloud KMS/HSM-protected LAK + respondent WebAuthn | Postgres + OpenTimestamps |
| **Regulated Cloud** | S3 GovCloud + Postgres cache + respondent device | GovCloud KMS/HSM-protected LAK + respondent WebAuthn | Postgres + OpenTimestamps |
| **Dedicated** | Tenant infrastructure + respondent device | Tenant Vault/HSM-protected LAK + respondent WebAuthn | Tenant Postgres + tenant-controlled anchoring |

---

## 5. Client Architecture

### The browser extension / PWA

The tool the respondent uses to fill out the form IS the wallet. One artifact, five capabilities:

```
+----------------------------------------------------------+
|  Formspec Client (Browser Extension or PWA)               |
|                                                           |
|  +------------------------------------------------------+ |
|  |  Formspec Engine (Rust -> WASM, already exists)       | |
|  |  FEL evaluation, validation, branching, repeats       | |
|  +------------------------------------------------------+ |
|                                                           |
|  +------------------------------------------------------+ |
|  |  Local Ledger                                         | |
|  |  Append-only event store (IndexedDB or OPFS)          | |
|  |  Hash chain computed locally                          | |
|  |  Events signed with PRF-derived Ed25519 key           | |
|  |  Payloads encrypted locally before sync               | |
|  +------------------------------------------------------+ |
|                                                           |
|  +------------------------------------------------------+ |
|  |  Identity Wallet                                      | |
|  |  WebAuthn credential (passkey, hardware-backed)       | |
|  |  PRF-derived signing + encryption keys (in-memory)    | |
|  |  Verifiable Credentials (OIDC-sourced)                | |
|  |  DID document                                         | |
|  +------------------------------------------------------+ |
|                                                           |
|  +------------------------------------------------------+ |
|  |  Crypto Engine (Rust -> WASM)                         | |
|  |  ed25519-dalek: event signing                         | |
|  |  ring: AES-256-GCM encryption, SHA-256 hashing        | |
|  |  curve25519-dalek: Pedersen commitments               | |
|  |  bbs_plus: proof derivation + verification            | |
|  |  ct_merkle: local Merkle tree + proof verification    | |
|  |  ciborium: deterministic CBOR serialization           | |
|  +------------------------------------------------------+ |
|                                                           |
|  +------------------------------------------------------+ |
|  |  Sync Client                                          | |
|  |  POST encrypted events to server                     | |
|  |  Receive governance events from server                | |
|  |  Bidirectional consistency verification               | |
|  |  Pin blobs to IPFS (if enabled)                       | |
|  +------------------------------------------------------+ |
+----------------------------------------------------------+
```

### Event granularity policy

Not every field change is an event. The default batching policy groups changes between explicit save points.

```
Three granularity levels (configured per form definition):

  DraftSession (DEFAULT):
    All field changes between save/submit actions are batched
    into a single event. Auto-save triggers after 30s of inactivity
    or when pending changes exceed a threshold.

    50-field form = 1 event per save, not 50 events.
    Multi-year Medicaid case: ~200 events vs ~5,000 per-field.
    25x chain size reduction.

  PerField:
    One event per field change. Only for audit-critical fields
    where individual change tracking is legally required
    (e.g., income amount, eligibility determination).
    Configurable per field in the form definition.

  PerSection:
    One event per logical form section submission.
    Middle ground for multi-page forms.

DraftAccumulator (client-side):
  Collects field changes into pending_changes map.
  Each change records: field_path, old_value, new_value, HLC,
    triggered_by (if FEL calculation caused this change).
  Flushes on: explicit save, submit, page navigation, auto-save
    timeout, or max pending changes threshold.

DraftSession event payload includes:
  field_snapshot:          full current state (for reconstruction
                          without replaying entire chain)
  change_log:             individual changes within this batch
                          (for audit trail; encrypted in payload)
  calculations_triggered: FEL calculations that fired
```

### Client event lifecycle

```
DraftAccumulator flushes (save/submit/auto-save/threshold):
  1. Formspec engine has processed all changes (FEL, validation, branching)
  2. EventBuilder creates event (typestate pipeline, compile-time ordering):
     a. Serialize payload fields to deterministic CBOR
     b. Compute payload_plaintext_commitment = SHA-256(plaintext_cbor)
     c. Compute fixed-position Pedersen commitments                ~50us
     d. Generate random DEK (AES-256 key)
     e. Encrypt CBOR payload with DEK (AES-256-GCM, fresh random 96-bit nonce)
     f. Compute payload_ciphertext_id = SHA-256(ciphertext)
     g. Wrap DEK with fresh ephemeral per recipient:
        - Respondent's X25519 public key (ECIES)                   ~50us
        - Ledger public key (ECIES, version tracked)               ~50us
     h. Build author envelope (event_type, HLC,
        payload_plaintext_commitment, payload_ciphertext_id,
        tag_commitment, commitments, causal_deps)
     i. Sign author envelope with respondent's Ed25519 key         ~1us
     j. Compute author_event_hash = SHA-256(domain_sep || len-prefixed
        envelope || ciphertext || key_bag)
  3. Append author event to local ledger (IndexedDB/OPFS)
  4. Store encrypted blob in local blob store
  5. Queue for sync (when online); canonical receipt arrives from server

Total client-side cost per event: sub-ms to low-ms on modern hardware.
No issuer private key is required on the client path.
```

### Multi-device handling

Respondent uses phone and laptop with the same synced passkey:

- Each device maintains its own local event stream with its own HLC
- Each event carries causal dependencies (up to 8 CausalDep entries)
- On sync, server receives events from multiple devices
- Server verifies each author event's signature (same DID, valid Ed25519)
- Server buffers events with unresolved dependencies instead of sequencing them prematurely
- Server builds a closed causal DAG, topologically sorts by HLC, then lexicographic `author_event_hash`
- Server detects true conflicts: concurrent events modifying overlapping fields with no causal relationship
- Conflict-sensitive overlaps REQUIRE an explicit `ledger.merge` event; receipt time never resolves them
- Server issues canonical receipts in deterministic order; it does not rewrite author envelopes
- Each client periodically syncs back the canonical receipt chain and merges its HLC
- Client can verify: all its locally-created author events appear in the canonical receipt chain, unmodified, in correct causal relative order

The server's receipt chain is authoritative for total order across devices. The author-event DAG preserves causality. Together they ensure that auto-calculated values from device A cannot silently overwrite manual entries from device B — the server detects this as a conflict and requires explicit resolution when the fields are conflict-sensitive.

---

## 6. Server Architecture

```
+----------------------------------------------------------+
|  Server                                                    |
|                                                           |
|  +------------------------------------------------------+ |
|  |  Sync Endpoint                                        | |
|  |  Receive encrypted events from clients                | |
|  |  Verify author-event integrity + signatures           | |
|  |  Deterministically issue canonical receipts           | |
|  |  Verify ledger-key wrapping and author_event_hash     | |
|  |  Append author events + receipts to Postgres          | |
|  |  Store blobs in configured backends                   | |
|  |  Deduplicate by author_event_hash (idempotent)        | |
|  +------------------------------------------------------+ |
|                                                           |
|  +------------------------------------------------------+ |
|  |  Postgres                                             | |
|  |  ledger_author_events: immutable actor-signed events  | |
|  |  ledger_receipts: append-only canonical order chain   | |
|  |  access_grant_entries: projected recipient wrappings  | |
|  |  merkle_tree: ct_merkle state per ledger              | |
|  |  Materialized views: case index, task queue, etc.     | |
|  +------------------------------------------------------+ |
|                                                           |
|  +------------------------------------------------------+ |
|  |  Temporal                                             | |
|  |  Workflow execution durability                        | |
|  |  Activities append governance events to ledger        | |
|  |  Activity results include checkpoint hashes           | |
|  |  Append is idempotent (deduplicate by activity ID)    | |
|  +------------------------------------------------------+ |
|                                                           |
|  +------------------------------------------------------+ |
|  |  Cloud KMS                                            | |
|  |  TMK (administrative, never used for derivation)      | |
|  |  LAKs (protected private keys, one per ledger)        | |
|  |  Mapping keys + disclosure issuer keys                | |
|  |  Decryption and key-release operations logged         | |
|  +------------------------------------------------------+ |
|                                                           |
|  +------------------------------------------------------+ |
|  |  Projection Pipeline                                  | |
|  |  Temporal worker tailing Postgres via LISTEN/NOTIFY   | |
|  |  Decrypts payloads (via LAK) for authorized views     | |
|  |  Updates materialized views                           | |
|  |  Epoch snapshots every N events for fast rebuild       | |
|  +------------------------------------------------------+ |
|                                                           |
|  +------------------------------------------------------+ |
|  |  Checkpoint Service                                   | |
|  |  Periodic Merkle root computation (ct_merkle)         | |
|  |  COSE-signed tree heads (coset)                       | |
|  |  OpenTimestamps anchoring (HTTP)                      | |
|  +------------------------------------------------------+ |
+----------------------------------------------------------+
```

### Governance event append path (server-side)

```
Temporal activity fires (e.g., caseworker completes determination):
  1. Activity constructs governance event:
     a. Serialize payload to deterministic CBOR
     b. Compute Pedersen commitments over numeric fields
     c. Generate DEK, encrypt payload
     d. Wrap DEK with ledger LAK public key
     e. Wrap DEK with respondent's DID public key
     f. Build author envelope, sign with platform Ed25519 key
     g. Compute author_event_hash
  2. Persist author event (deduplicate by activity ID / author_event_hash)
  3. Issue canonical receipt and append to ledger_receipts
  4. Store blob in configured backends
  5. NOTIFY on Postgres channel
  6. Optionally mint a disclosure attestation for that event
     only after strict payload verification
  7. Return checkpoint hash to Temporal activity result
  8. Projection worker picks up NOTIFY, updates materialized views

Total server-side cost per event: low-ms, dominated by key release and I/O.
```

---

## 7. Sync Protocol

### Ingest integrity modes

| Mode | Server verifies | Use when |
|------|-----------------|----------|
| `Strict` | Signature, `payload_ciphertext_id`, `author_event_hash`, LAK decrypt, `payload_plaintext_commitment`, `tag_commitment`, Pedersen openings/schema | Default. Required for PHI, regulated workflows, analytics, disclosure issuance, adverse actions |
| `Relaxed` | Signature, `payload_ciphertext_id`, `author_event_hash`, key-bag structure only | Only when tenant explicitly accepts payload content remaining unverified until later strict verification |

`Strict` is the default and SHOULD be mandatory for PHI/PII-heavy deployments. `Relaxed` mode MUST be tenant-configured, recorded in the canonical receipt, exposed in audit output, and excluded from commitment-driven analytics, disclosure issuance, and adverse-action workflows until strict verification succeeds.

### Client -> Server (intake events)

The client creates key bag entries for BOTH respondent and ledger-service at event creation time, using each recipient's public key via HPKE/X25519. The server never sees the plaintext DEK on the wire.

```
Client-side key bag construction at event creation:

  key_bag = [
    { recipient: "respondent",
      ephemeral_pubkey: ...,   // fresh per wrapping
      wrapped_dek: ECIES(DEK, respondent_x25519_pub) },
    { recipient: "ledger-service",
      ledger_key_version: 3,   // tracked for rotation
      ephemeral_pubkey: ...,   // fresh per wrapping
      wrapped_dek: ECIES(DEK, ledger_public_key_v3) },
  ]

  The ledger public key is fetched during session setup from the
  ledger metadata endpoint. Includes key version for rotation tracking.
```

```
Client POST /ledger/{ledger_id}/sync
  Body: [
    {
      envelope_bytes,
      encrypted_payload,
      key_bag,          // respondent + ledger-service wrapped DEKs
    },
    ...
  ]

Server processing:
  1. Verify each author event:
     a. Deserialize envelope
     b. Verify Ed25519 signature against respondent's known DID public key
     c. Verify payload_ciphertext_id matches the ciphertext hash
     d. Verify author_event_hash covers envelope + ciphertext + key_bag
     e. Apply ingest mode:
        - Strict: unwrap via LAK and verify payload_plaintext_commitment,
          tag_commitment, and Pedersen openings/schema
        - Relaxed: skip decrypt, mark verification_state = ciphertext_only
  3. Verify ledger_key_version in key bag:
     a. If version is Active or Deprecated: accept
     b. If version is RetiredForDecryptionOnly: accept with warning,
        server schedules re-wrap for current key version
     c. If version is Revoked (compromise): reject, return current version
     d. If version is unknown: reject, return current version
  4. Persist encrypted payload to the primary blob store
  5. Persist immutable author event row (deduplicate by author_event_hash)
  6. Resolve dependencies and sequencing (see Merge And Fork Handling)
  7. Issue canonical receipts for all sequenceable events
  8. Schedule replica writes / backfill for any secondary blob failures
 10. Return:
     {
       accepted: count,
       canonical_receipts: [(author_event_hash, sequence, receipt_hash), ...],
       signed_tree_head: COSE_Sign1(...),
       ledger_key_info: {
         active_version: 4,
         active_pubkey: ...,
         deprecation_deadline: ...,  // if client used deprecated version
       },
       lak_rotation_advisory: ...,  // if LAK is rotating (see §Key Rotation)
     }

Ledger key rotation grace period:
  Server ACCEPTS events wrapped under any non-revoked key version.
  Only compromised (Revoked) keys are hard-rejected.
  Deprecated keys get a grace period (typically 30 days).
  Offline clients that sync after weeks still work.
  Server transparently re-wraps DEKs for retired keys.
```

### Merge And Fork Handling (normative)

- An event whose `causal_deps` reference an unknown `author_event_hash` MUST enter `PendingDependency`. The server MUST NOT assign it a canonical receipt, project it, or include it in a checkpoint while pending.
- `PendingDependency` events MAY be buffered for up to 24 hours by default. Tenants MAY increase this window up to 7 days. On expiry, the server MUST reject the event with `dependency_unresolved`.
- The sequencer MUST build a closed DAG from already-accepted author events plus newly satisfiable pending/incoming events before ordering.
- The sequencer MUST topologically sort by `(hlc.wall_ms, hlc.logical, lexicographic author_event_hash)`.
- Receipt time, arrival order, worker count, and server build differences MUST NOT affect canonical order.
- When two concurrent events with no causal relationship touch overlapping fields:
  - If all overlaps are marked `last_writer_wins`, the sequencer MAY linearize them with the deterministic rule above.
  - If any overlap is `conflict-sensitive`, the server MUST stop sequencing past that conflicting frontier until an explicit `ledger.merge` event is appended.
- `ledger.merge` is itself an author event plus canonical receipt. It names the conflicting `author_event_hash` values and commits the chosen resolved value or merge rationale.

### Server -> Client (governance events)

```
Client GET /ledger/{ledger_id}/events?since={sequence}

Server returns:
  [
    {
      envelope_bytes,        // author event envelope
      receipt_bytes,         // canonical receipt
      encrypted_payload_cid, // content address (client can fetch blob if authorized)
      key_bag_for_respondent, // respondent_wrapped_dek if respondent has access
    },
    ...
  ]

Client processing:
  1. Verify canonical receipt continuity from last known receipt
  2. Verify author signature on each envelope
  3. Verify sequencer signature on each receipt
  4. Store envelope + receipt in local ledger cache
  5. If respondent has key bag entry: can decrypt and read governance events
  6. Even without decryption: can verify structural compliance
     (deontic checks passed, AI was disclosed, review protocol was followed)
```

### Bidirectional consistency verification

```
Periodically (or on demand):

Client requests:
  GET /ledger/{ledger_id}/consistency-proof
    ?client_tree_size={N}
    &server_tree_size={latest}

Server returns:
  {
    signed_tree_head: COSE_Sign1({ tree_size, root_hash, timestamp }),
    consistency_proof: [hash_1, hash_2, ...],  // ct_merkle consistency proof
  }

Client verifies:
  1. Verify COSE signature on signed tree head
  2. Verify consistency proof: server's receipt tree is a valid extension of client's tree
     (ct_merkle consistency_verify)
  3. If verification fails: client has cryptographic evidence of server tampering
     (the signed tree head and the failed proof are the evidence)
```

---

## 8. Coprocessor Transition

```
Client ledger (respondent's browser):
  [0] session.started
  [1] draft.saved
  [2] setData (field mutation)
  ...
  [N] response.completed        <- signed by respondent

      | sync (POST encrypted events)
      v

Server verifies chain, merges any concurrent frontiers, assigns
canonical receipt sequences [0..N]

      | Coprocessor
      v

Server appends:
  [N+1] case.created {           <- signed by PLATFORM
    intake_author_event_hash: hash of author event [N],
    intake_receipt_hash: canonical receipt hash for [N],
    case_id: "MED-2026-0847",
    case_file_mapping: { response_field -> case_field },
    contract_validation_result: pass/fail,
    kernel_document_ref: "...",
    workflow_id: "medicaid-redetermination-v3",
  }

Server ledger continues (all signed by platform):
  [N+2] wos.transition.fired
  [N+3] wos.task.created
  ...
  [N+M] wos.explanation.assembled

If respondent files RFI response or appeal:
  Client creates new intake events
  Syncs to server
  Server appends to SAME ledger, continuing the chain
  [N+M+1] session.started (RFI)   <- signed by respondent again
  [N+M+2] ...
```

The `case.created` event is the phase boundary. Before it, author events are respondent-signed. After it, author events may be platform-signed (governance) or respondent-signed (subsequent intake). The author-event DAG stays immutable. The canonical receipt chain stays continuous. One ledger, one receipt Merkle tree, no rewriting of actor-signed bytes.

---

## 9. Materialized Views & Projections

### View definitions

| View | Source events | Decryption needed? | Purpose |
|------|-------------|-------------------|---------|
| Case index | `case.created` + latest `wos.transition.fired` | No (header fields) | Dashboard: "Angela's 47 pending cases" |
| Task queue | `wos.task.*` events | Partial (task metadata in header, details encrypted) | Reviewer work queue |
| Current case file | `setData` mutations within events | Yes (field values are encrypted) | "What is the current income value?" |
| SLA status | `wos.task.created` + `wos.timer.*` + `wos.task.completed` | No (timestamps in headers) | SLA breach warnings |
| Equity metrics | `wos.transition.fired` on determination-tagged events | Strict-verified Pedersen commitments (no decryption at query time) | Disparity monitoring via homomorphic aggregation |
| Analytics | All events | Strict-verified commitments + receipt/envelope fields | Completion rates, time-to-determination |
| Audit trail | All headers | No | Complete structural history |

Only events with `verification_state = verified_payload` may contribute commitments to analytics, selective-disclosure issuance, or adverse-action workflows. `ciphertext_only` events remain visible in the audit trail but are operationally quarantined until strict verification completes.

### Projection pipeline

```
Postgres LISTEN/NOTIFY -> Temporal projection worker

Worker:
  1. Receive event notification
  2. Read envelope + receipt from Postgres
  3. For views needing decrypted content:
     a. Fetch encrypted blob from blob store
     b. Unwrap DEK via LAK key release/decryption path (logged)
     c. Decrypt payload
     d. Update materialized view
  4. For views using only receipts/envelopes/commitments:
     a. Update directly from receipt and envelope fields
     b. Aggregate Pedersen commitments only for `verified_payload` events
  5. At epoch boundaries (every N events):
     a. Snapshot all views for this ledger
     b. Store snapshot reference in checkpoint

View rebuild:
  Start from nearest epoch snapshot, replay events forward.
  NOT from genesis. Expert panel: "full replay of millions of
  encrypted events is impractical."
```

### Pseudonymous ledger identity

The `ledger_id` is a random UUID assigned at case creation. The mapping from respondent identity to `ledger_id` is stored separately, encrypted with a dedicated mapping key, and independently deletable. This ensures that after GDPR erasure, the link between respondent and ledger is cryptographically severed.

```
LedgerIdMapping {
  encrypted_respondent_id: Vec<u8>,  // encrypted with mapping-specific key
  ledger_id: UUID,                   // the pseudonymous identifier
  mapping_key_id: KmsKeyId,          // independently destroyable
}

Destroying the mapping key makes the respondent-to-ledger link
unrecoverable, even though the ledger events still exist.
```

### GDPR erasure protocol (6-step, no in-place mutation)

This design does NOT rewrite committed bytes after append. Erasure terminates the live ledger at a final anchored tombstone, destroys platform-held decryption capability, severs identity mapping, and purges all platform-controlled ciphertext and projections. After erasure, the platform retains only the canonical receipts, the `ledger.erased` tombstone, and the final anchored checkpoint needed to prove prior existence and ordering; decryptable payloads, mappings, grants, caches, and platform-controlled replicas are deleted. Copies already exported or stored outside platform control are out of scope.

```
Complete GDPR erasure protocol:

  Step 1: Record erasure request
    Append `erasure.requested` event to the ledger.

  Step 2: Generate final Merkle checkpoint and anchor it
    Compute Merkle root, sign tree head (COSE), anchor via OpenTimestamps.
    This preserves structural proof of event existence without
    mutating prior committed bytes.
    The anchored checkpoint is the last verifiable snapshot.

  Step 3: Append `ledger.erased` tombstone event
    Tombstone payload commits:
      final_tree_size, final_root_hash, ots_reference,
      erasure_authority, effective_at.
    No further events are accepted after this tombstone.

  Step 4: Destroy platform-held decryption keys
    Destroy all historical LAK private key versions for this ledger.
    Destroy any custodial respondent keys held by the platform.
    Schedule with 7-day waiting period (last chance to abort).

  Step 5: Destroy mapping key and delete mapping records
    Break the respondent <-> ledger_id link in KMS and storage.
    After this, the platform cannot re-identify the ledger.

  Step 6: Purge platform-controlled mutable state
    Delete ciphertext replicas from blob stores the platform controls.
    Delete access-grant projections, disclosure attestations,
    materialized views, caches, and exports.
    Retain only the immutable committed envelope plus the final
    anchored checkpoint/tombstone required for proof of prior existence.

  Post-erasure state:
    The platform retains no live decryption path and no identity mapping.
    Proofs terminate at the `ledger.erased` tombstone.
    Historical existence remains provable up to that final checkpoint.

  This protocol is IRREVERSIBLE. The 7-day KMS deletion waiting period
  is the last chance to abort. After mapping key destruction and LAK
  destruction, the platform cannot reconstruct or read the ledger content.
```

---

## 10. Selective Disclosure & Permissioned Sharing

### Three-tier access model

Access to event data operates at three visibility tiers, but grants also have enforcement modes. This distinction matters: once a recipient receives a full-event DEK, `DisclosurePolicy` is governance and audit policy, not cryptographic containment.

```
GrantMode:
  proof_only      -- DEFAULT for external sharing. No DEK is shared.
                     Authorized proofs are minted by the disclosure issuer service.
  full_decrypt    -- Exceptional. Recipient receives a DEK wrapping and can read all fields.
  projection_only -- No DEK shared. Recipient sees only approved server-derived views.
```

```
Tier 0: No Access (observer, auditor without keys)
  - Can see plaintext receipt/envelope fields needed for structural verification
  - Can see tag_commitment (but cannot open it without the nonce)
  - Can see Pedersen commitments (but cannot open without blinding factors)
  - Can verify chain integrity, Merkle proofs, Ed25519 signatures
  - CANNOT see any payload fields or determine tag values

Tier 1: Full Decryption (key bag grantee, exceptional)
  - Has a key bag entry -> can decrypt DEK -> can decrypt payload
  - Sees ALL fields in the decrypted payload
  - May be governed by DisclosurePolicy, but that policy is NOT a
    cryptographic confidentiality boundary once plaintext is revealed

Tier 2: Selective Proof Recipient (BBS+ / SD-JWT verifier)
  - Receives an authorized selective disclosure proof
  - Sees ONLY the disclosed fields
  - Can verify the disclosed fields are authentic (BBS+ verification)
  - CANNOT decrypt the full event or see undisclosed fields
  - Proof is unlinkable (two proofs from same signature can't be correlated)
```

**Default workflow:** A caseworker with access to a case requests a disclosure proof for an auditor. The disclosure issuer service validates authorization plus `DisclosurePolicy`, loads the disclosure attestation for the target event, and returns a proof revealing only the approved fields. External parties SHOULD receive `proof_only`, not `full_decrypt`.

### DisclosurePolicy (governance primitive)

```
DisclosurePolicy {
  grantee: ActorId,           // who is requesting proof issuance
  event_types: [u16],         // which event types this policy applies to
  disclosable_fields: [       // fields the issuer MAY disclose on grantee's behalf
    { field_path: "income.monthly", bbs_message_index: 2 },
    { field_path: "eligibility.status", bbs_message_index: 7 },
  ],
  redacted_fields: [          // fields the grantee MUST NOT disclose
    "ssn", "medical_records",
  ],
  valid_until: u64,           // policy expiration
  authority: PolicyAuthority, // respondent, admin, or regulation
}

Creating a selective disclosure proof:
  1. Validate: requested fields are in disclosable_fields
  2. Validate: no redacted fields included
  3. Build full BBS+ message vector from decrypted payload
  4. Load the disclosure attestation for the target event
  5. Generate BBS+ proof of knowledge for disclosed indices
     inside the disclosure issuer service by default
  6. Proof includes bbs_key_version so verifier can look up correct public key
```

Direct client-side proof derivation by a `full_decrypt` grantee is optional. If enabled, it MUST be documented as a convenience feature, not as an enforceable confidentiality boundary.

### BBS+ disclosure attestation and proof flow

```
At event creation:
  Core ledger event is signed only with Ed25519 and appended normally.

After ingest (server or designated issuer):
  Build disclosure message vector from the decrypted payload:
    messages = [target_event_hash, actor_id, field_1, field_2, ..., field_N]
  Issue disclosure attestation:
    attestation = BBS+.sign(messages, disclosure_issuer_key)
  Store attestation as a separate artifact or reference it from a
  `disclosure.attested` ledger event.

At disclosure time (FOIA, cross-agency, audit):
  Default path: authorized caller requests proof from disclosure issuer:
    disclosed_indices = [2, 5]  // reveal only field_1 and field_4
    proof = BBS+.derive_proof(attestation, messages, disclosed_indices)
  Tier 2 verifier checks:
    BBS+.verify_proof(proof, disclosed_messages, public_key[bbs_key_version])
    -> confirms these fields were part of a signed event
    -> learns nothing about undisclosed fields
    -> proof is unlinkable
```

### SD-JWT as parallel backend

```
Same pluggable interface:

pub trait SelectiveDisclosure {
    fn sign(&self, fields: &[Field], key: &SigningKey) -> Result<Signature>;
    fn derive_proof(&self, sig: &Signature, fields: &[Field], disclose: &[usize]) -> Result<Proof>;
    fn verify_proof(&self, proof: &Proof, disclosed: &[Field], pubkey: &PublicKey) -> Result<bool>;
}

impl SelectiveDisclosure for BbsPlusBackend { ... }
impl SelectiveDisclosure for SdJwtBackend { ... }
```

Both backends may be built. Deployment config chooses which is active. Government deployments requiring IETF-only primitives use SD-JWT. Everyone else uses BBS+, subject to the browser feasibility gate in §3.

### Permissioned sharing

```
Permissioned sharing has two different paths:

  A. Proof-only grant (DEFAULT for external recipients)
     1. Append `access.granted` with:
        grantor, recipient, scope, expiry, grant_mode = proof_only,
        disclosure_policy_version
     2. No DEK is re-wrapped to the recipient
     3. Disclosure issuer service may mint selective proofs for that recipient
        while the grant is active

  B. Full-decrypt grant (EXCEPTIONAL)
     1. Respondent authenticates (WebAuthn -> PRF -> keys in memory)
     2. For each event in scope:
        a. Decrypt respondent_wrapped_dek using respondent's X25519 private key
        b. Wrap DEK with recipient public key (fresh ephemeral per wrapping)
        c. Add { target_sequence, recipient_id, ephemeral_pubkey, wrapped_dek }
           to an immutable GrantBundle
     3. Append `access.granted` carrying:
        grantor, recipient, scope, expiry, grant_mode = full_decrypt,
        disclosure_policy_version, and GrantBundle content hash
     4. Projection worker expands the bundle into `access_grant_entries`
     5. Recipient can now decrypt the scoped events

Revocation is append-only in both modes:
  - `access.revoked` references the original grant event
  - Projection state marks rows revoked
  - Historical audit remains intact
```

---

## 11. Export Artifact

```
Self-verifying deterministic ZIP:

  ledger-export-MED-2026-0847/
    manifest.cbor              # export metadata, ledger_id, export timestamp
    author-events/
      000000.cbor              # actor-signed envelopes
      000001.cbor
      ...
    receipts/
      000000.cbor              # canonical receipts in sequence order
      000001.cbor
      ...
    payloads/
      <content_id_hex>.enc     # encrypted payload blobs, named by content hash
      ...
    tree/
      tree.bin                 # full ct_merkle tree over receipt_hash leaves
      checkpoints/
        checkpoint_100.cbor    # signed tree head at sequence 100
        checkpoint_100.ots     # OpenTimestamps proof
        checkpoint_200.cbor
        checkpoint_200.ots
        ...
    bitcoin/
      headers.cbor             # optional but REQUIRED for fully air-gapped OTS verification
    keys/
      public_keys.cbor         # all signing public keys (respondent DID, platform, etc.)
      bbs_public_keys.cbor     # BBS+ public keys for selective disclosure verification
    key_bags/
      key_bags.cbor            # base per-event key bags
    access/
      grants.cbor              # immutable grant bundles + access events
    disclosures/
      *.cbor                   # disclosure attestations / SD-JWT artifacts
    schemas/
      event_v1.cbor            # event schema at each version
      event_v2.cbor
    verify.sh                  # self-contained verification script
    README.md                  # human-readable explanation of the artifact

  Verification:
    1. For each author event: verify Ed25519 signature
    2. For each author event: verify payload_ciphertext_id matches SHA-256(payload blob)
    3. Recompute author_event_hash from envelope + ciphertext + key bag
    4. For each receipt: verify sequencer signature
    5. Verify canonical receipt chain via canonical_prev_receipt_hash
    6. Recompute receipt_hash from receipt bytes
    7. Rebuild Merkle tree from receipt_hash leaves, compare against tree.bin
    8. Verify each checkpoint's COSE signature
    9. Verify any disclosure attestations against bbs_public_keys.cbor
   10. Verify OpenTimestamps proofs against Bitcoin block headers if
       `bitcoin/headers.cbor` is included; otherwise anchoring verification
       requires an external Bitcoin header source
   11. Result: "this ledger is intact, unmodified, and was anchored to
       Bitcoin at these timestamps"

  No platform access needed. No trust in the platform needed.
  No network needed only when the export includes the Bitcoin header bundle.
```

---

## 12. Degraded Modes

### No personal device (library computer, kiosk, shared phone)

The architecture must not require a personal device. Medicaid applicants use library computers.

```
Degraded mode: server-custodial keys

  1. Respondent authenticates via OIDC (Login.gov/ID.me) -- no WebAuthn
  2. Server generates a custodial key pair on respondent's behalf
  3. Server holds the private key in KMS (tagged as custodial)
  4. Events are signed with the custodial key
  5. Encryption uses the custodial key (server can decrypt -- less sovereign)
  6. Respondent can later register a WebAuthn credential on their own device:
     a. Server wraps all existing DEKs with the respondent's new DID public key
     b. Appends immutable `access.granted` events carrying the new wrappings
     c. Respondent now has independent access
     d. Optionally: destroy custodial key (full sovereignty transfer)
```

The ledger still works. The chain still has integrity. The crypto-shredding still works. The sovereignty is weaker (server holds keys) but the respondent can upgrade to full sovereignty at any time by registering a WebAuthn credential.

### No WebAuthn PRF extension (older browser)

```
Fallback: explicit local recovery secret + encrypted key blob

  1. WebAuthn registration succeeds (passkey created)
  2. PRF extension not available
  3. Client generates Ed25519 + X25519 key pairs
  4. User chooses a recovery passphrase or stores a recovery code
  5. Argon2id derives a KEK from that recovery secret
  6. Client encrypts the private keys with the KEK and stores the blob
     in IndexedDB/OPFS
  7. Optional: require a fresh WebAuthn assertion before unlock, but the
     assertion bytes are NOT used as cryptographic key material
  8. If the user will not manage a recovery secret, fall back to custodial mode
```

### Offline-only (no connectivity during form fill)

```
Already handled:

  1. WASM engine evaluates locally (existing capability)
  2. Local ledger records all events to IndexedDB/OPFS
  3. Encrypted blobs stored in local blob store
  4. When connectivity returns: sync protocol sends everything
  5. Server verifies chain, assigns canonical sequence
  6. Respondent at a rural clinic on spotty cellular fills the entire
     Medicaid application offline. Every interaction recorded.
     Syncs at the library when they get wifi.
```

---

## 13. Rust Crate Layout

Consolidated from 10 to 7 crates. The serialize-commit-encrypt-wrap-sign-hash pipeline is one atomic operation; splitting it across crate boundaries increased the risk of wrong ordering or missing steps. The `EventBuilder` typestate pattern now enforces the correct pipeline at compile time.

```
crates/
  ledger-engine/         # THE CORE CRATE. Event types, header format v2,
                         # CBOR serialization, hash chain computation,
                         # event type registry, HLC, causal DAG.
                         #
                         # crypto/ (pub(crate)):
                         #   ed25519-dalek signing, ring AES-256-GCM,
                         #   curve25519-dalek Pedersen, x25519-dalek wrapping,
                         #   HKDF key derivation
                         #
                         # merkle/ (pub(crate)):
                         #   ct_merkle integration, inclusion/consistency proofs
                         #
                         # checkpoint/:
                         #   COSE signing (coset), signed tree heads
                         #
                         # disclosure/:
                         #   BBS+ proof derivation, SD-JWT backend,
                         #   SelectiveDisclosure trait, DisclosurePolicy
                         #
                         # Public API: EventBuilder (typestate),
                         #   EventVerifier, SealedEvent
                         # EventBuilder enforces: payload -> commitments ->
                         #   encrypt -> wrap -> sign -> finalize
                         # Each step consumes self, returns next state.
                         # Calling .sign() before .encrypt() is a compile error.
                         #
                         # Compiles to: native + wasm32

  ledger-identity/       # DID derivation from Ed25519 pubkey,
                         # VC data model, WebAuthn PRF key derivation
                         # (with server-side salt management),
                         # OIDC-to-VC adapter, versioned HKDF info strings
                         # Compiles to: native + wasm32

  ledger-anchor/         # OpenTimestamps client (HTTP)
                         # Server-only (not WASM)

  ledger-store/          # BlobStore trait + implementations:
                         #   PostgresBlobStore (server)
                         #   S3BlobStore (server)
                         #   IpfsBlobStore (server + WASM via HTTP)
                         #   OpfsBlobStore (WASM only)
                         #   IndexedDbBlobStore (WASM only)
                         # Header chain store (Postgres + IndexedDB)
                         # Key bag store + access-grant projections
                         # Key rotation state machine (LAK, disclosure, mapping)

  ledger-sync/           # Sync protocol: client side + server side
                         # Chain verification, consistency proofs
                         # Ledger key version negotiation
                         # Causal ordering, frontier merge, conflict resolution
                         # Compiles to: native + wasm32

  ledger-projection/     # Materialized view definitions,
                         # projection pipeline,
                         # epoch snapshot management,
                         # GDPR erasure protocol (6-step tombstone flow),
                         # encrypted tag index management
                         # Server-only

  ledger-wasm/           # WASM entry point, binds all crates for browser
                         # wasm-bindgen exports
                         # DraftAccumulator (client-side event batching)

  ledger-server/         # Server entry point, Postgres integration,
                         # Temporal activities, KMS integration,
                         # HTTP endpoints (sync, consistency, export)
                         # Key rotation sweep workers
```

All crates share the same cryptographic primitives via `ledger-engine`. One implementation. Two compilation targets. No TS reimplementation. The critical crypto pipeline is entirely within `ledger-engine` with a single public entry point (`EventBuilder`).

---

## 14. What This Does NOT Include

| Excluded | Why |
|----------|-----|
| FHE eligibility computation | Blocked on GPU infrastructure + case volume. Pedersen commitments are the bridge. |
| PHE equity monitoring | Blocked on actual equity metrics to monitor. Commitments are the bridge. |
| Respondent-facing wallet app | Blocked on identity ecosystem maturity. The browser extension IS the wallet. |
| Consortium blockchain | Single sequencer per ledger. No multi-party consensus needed. |
| Mix network / onion routing | Consumer internet identity problem (TPIF), not case management. |
| Verification oracles | Same. TPIF scope, not WOS scope. |
| Custom storage engine | Postgres + ct_merkle + blob stores. No custom DB. |
| Custom signing scheme | COSE (coset) + Ed25519 (ed25519-dalek). Standards. |
| Proof of Personhood framework | IAL2 identity proofing subsumes PoP. |

---

## What We Build

The novel work. Everything else is composition.

1. **Unified event taxonomy** -- intake + governance + lifecycle event types, field schemas, privacy classifications, provenance tier mappings. ~50 event types. Per-event-type commitment schemas and disclosure message schemas.

2. **Three-tier access model** -- No Access / Full Decryption / Selective Proof. Audience-scoped views backed by base key bags + immutable access events + disclosure-attestation governance.

3. **Coprocessor protocol** -- `response.completed` -> `case.created`. Sync protocol with causal ordering (HLC + causal deps). Chain handoff from respondent-signed to platform-signed. Subsequent intake events (RFI, appeal) re-entering the chain.

4. **Regulatory compliance semantics** -- retention, legal hold, tombstoned GDPR erasure (6-step protocol with final checkpoint + key destruction + mapping destruction), expungement as ledger operations with mandatory cascades.

5. **Key rotation protocol** -- LAK lazy re-wrap, disclosure key version registry, ledger public key grace periods, active session handling. NIST 800-57 compliant.

6. **Materialized view projection definitions** -- which events project to which views, decryption requirements, encrypted tag index, epoch snapshot strategy, rebuild procedures.

7. **Export artifact format** -- self-verifiable ZIP with headers, blobs, Merkle tree, checkpoints, OTS proofs, access bundles, disclosure artifacts, and versioned public keys.

8. **Degraded mode protocols** -- custodial keys, PRF fallback, sovereignty upgrade path.

9. **Event granularity framework** -- DraftAccumulator batching (DraftSession / PerField / PerSection), conflict-sensitive field policies, auto-save with causal tracking.

10. **The Rust crates** -- 7 crates with typestate EventBuilder enforcing correct event construction ordering, plus separate disclosure issuance. One implementation, two compilation targets, no TS reimplementation.

Everything else -- the cryptographic primitives, the identity standards, the storage systems, the workflow engine -- already exists. We compose them. Once.
