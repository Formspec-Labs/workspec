# Unified Ledger: Concrete Proposal

**Date:** 2026-04-10
**Status:** Proposal
**Extends:** ADR-0059 (Unified Ledger as Canonical Event Store)
**Incorporates:** Expert panel corrections, technology survey, TPIF privacy model, architectural + cryptographic review findings, technical audit (2026-04-11), concrete audit resolutions (2026-04-11), technical approach audit (2026-04-11: 8 findings — custodial trust boundary, frontier stall breaker, nonce disambiguation, layer fence fix, registry extensibility, WebAuthn PRF spike), threshold custodial key design (2026-04-11: FROST 2-of-3 replacing single-party custody, user-story-driven iteration), second technical approach audit (2026-04-11: 8 findings — dCBOR spike, ProvenanceKind mapping coverage, HPKE crate selection, disclosure crate extraction, cross-range field overlap clarification, custodial recovery ordering, byte-wise tiebreak clarification, sequencer sort normative precision)
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
  -> Recovery re-grant (see authorization model below)
  -> Old events now readable via new DID key
  -> New events use new DID key
  -> VC re-issued binding new DID to same IAL2 identity
  -> Old credential's key_id revoked from future grants

  Recovery re-grant authorization model:
    Requires TWO of:
      a. Respondent's new WebAuthn credential (proves current identity)
      b. Platform admin approval (logged as a separate event)
      c. Automated policy check (OIDC identity continuity verified)

    Default: (a) + (c) for automated recovery.
    If OIDC continuity check fails: (a) + (b) required.

    Execution:
      1. Append key.recovery_regranted event (0x0407) FIRST:
           old_did, new_did, oidc_continuity_verified: bool,
           admin_approver: Option<ActorId>,
           events_regranted: u64 (count), regranted_at: u64
      2. For each historical event:
           decrypt DEK via ledger access key,
           wrap with new DID public key,
           append immutable `access.granted` event carrying new wrapping
           (each references key.recovery_regranted as causal dep)
      3. The admin/system never holds plaintext DEKs in memory longer
         than one re-wrap cycle. DEKs are zeroized after each re-wrap.
         KMS audit log captures every key-release call.
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

### Signing key registry

The server MUST maintain a registry mapping `signing_key_id` to verification metadata. Without this registry, the server cannot verify event signatures and cannot enforce key revocation after device recovery.

```
SigningKeyEntry {
  signing_key_id:   [u8; 16],        // truncated hash of public key
  public_key:       Ed25519PublicKey, // for signature verification
  did:              String,           // owning DID (may change on recovery)
  device_id:        [u8; 8],         // originating device
  hkdf_version:     u8,              // HKDF info string version used
  status:           Active | Revoked { reason, revoked_at },
  created_at:       u64,
  last_seen_at:     u64,             // updated on each verified event
}

Lifecycle:
  Active:   signature verification succeeds; events accepted.
  Revoked:  events signed with this key are rejected. Set when:
            - respondent completes device recovery (old key revoked)
            - admin revokes a compromised credential
            - WebAuthn credential is deregistered

Recovery linking:
  When a respondent recovers via OIDC and registers a new WebAuthn
  credential (§1 Step 6), the platform creates a new SigningKeyEntry
  for the new DID and marks the old entry Revoked. The DID linkage
  (old DID → new DID) is recorded in the recovery event, not in
  the signing key registry itself.
```

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
|                     SHA-256(plaintext CBOR || commitment_nonce)|
|  payload_ciphertext_id: [u8; 32]  SHA-256 of ciphertext       |
|                                                               |
|  -- Identity & governance (immutable plaintext metadata) --   |
|  actor_type:         u8     enum (respondent, caseworker...)  |
|  actor_correlation_id: [u8; 16]  per-invocation correlation ID|
|                     Unique per AI invocation, system job, or  |
|                     human session. Enables structural AI audit|
|                     without payload decryption (OMB M-24-10). |
|                     Zero-filled for actors without correlation.|
|  event_type:         u16    index into event type registry    |
|  privacy_tier:       u8     (anonymous, pseudo, id'd, full)   |
|  signing_key_id:     [u8; 16]  identifies signer's key        |
|  signature:          [u8; 64]  Ed25519 over canonical envelope|
|  governance_result:  u8     (pass/fail/na for deontic checks) |
|  key_custody:        u8     (0=sovereign, 1=threshold_custodial,|
|                              2=delegated)                     |
|                                                               |
|  -- Privacy-preserving (immutable commitments only) --        |
|  tag_commitment:     [u8; 32]  SHA-256(dCBOR([tags, nonce]))  |
|    Nonce stored INSIDE encrypted payload, not in envelope.    |
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
|  Hash 1: payload_plaintext_commitment (IN the envelope)       |
|    = SHA-256(plaintext_cbor || commitment_nonce)               |
|    Purpose: bind the event to specific plaintext content.     |
|    The 16-byte commitment_nonce (stored inside the encrypted  |
|    payload) prevents brute-force recovery of low-entropy      |
|    payloads from the commitment alone. Without the nonce, a   |
|    structured payload with few possible values (e.g., boolean |
|    eligibility) is trivially recoverable by hashing candidates.|
|    Computed BEFORE encryption.                                |
|                                                               |
|  Hash 2: payload_ciphertext_id (IN the envelope)              |
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
|  Domain separation via versioned prefix strings and length    |
|  prefixing (encode-then-hash). Length prefixing prevents      |
|  component boundary ambiguity. This is a protocol-specific    |
|  convention; NIST SP 800-185's TupleHash provides a           |
|  standardized alternative but requires SHA-3.                 |
+--------------------------------------------------------------+

+--------------------------------------------------------------+
|  PAYLOAD (encrypted blob, content-addressed)                  |
|                                                               |
|  Deterministic CBOR encoding of:                              |
|    actor_id, field values, rationale, evidence,               |
|    confidence details, document content,                      |
|    delegation chain, equity dimensions,                       |
|    commitment_nonce: [u8; 16],   -- for plaintext commitment  |
|    tag_nonce: [u8; 16],          -- for tag_commitment verify |
|    tag_bitfield: u16,            -- actual tags               |
|    commitment_blinding_factors   -- for Pedersen opening      |
|                                                               |
|  Encrypted: AES-256-GCM(plaintext_cbor, DEK, encryption_nonce)|
|  Nonce rule: fresh random 96-bit encryption_nonce per payload |
|  Never reuse an encryption_nonce under the same DEK.          |
|  NOTE: encryption_nonce (96-bit, AES-GCM IV) is DISTINCT from|
|  commitment_nonce (128-bit, anti-brute-force salt for the     |
|  plaintext commitment hash). Different purposes, different    |
|  sizes, different security properties.                        |
|  Content-addressed: content_id = SHA-256(ciphertext)          |
|  Stored in: any content-addressed blob store                  |
+--------------------------------------------------------------+

+--------------------------------------------------------------+
|  KEY BAG (per-event, extensible, stored with author event)    |
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

Normative rules:

  Clock skew bound:
    The server MUST reject events whose hlc.wall_ms exceeds
    server_wall_ms + MAX_CLOCK_SKEW (default: 60,000 ms = 1 minute).
    Clients whose clocks are far ahead must sync time before submitting.

  Wall clock coarsening:
    wall_ms MUST be truncated to 1-second granularity (floor to nearest
    1000 ms). Sub-second precision enables device fingerprinting via
    clock drift patterns. 1-second granularity is sufficient for causal
    ordering when combined with the logical counter.

  Persistence:
    Clients MUST persist their current HLC to durable storage
    (IndexedDB/OPFS) on every event creation. On restart, clients MUST
    load the persisted HLC before creating new events. Failure to persist
    allows the logical counter to reset, breaking the monotonicity
    invariant and producing events that appear to precede their
    actual causal predecessors.

  Ordering ambiguity (informational):

    Events from different devices with the same coarsened wall_ms
    and logical = 0 are ordered by byte-wise lexicographic comparison
    of the 32-byte author_event_hash (raw bytes, not hex strings).
    This is deterministic but semantically arbitrary -- it does not
    reflect actual temporal ordering.

    For audit interpretation: events with identical (wall_ms, logical)
    from different devices are concurrent by definition. Their
    canonical order is a deterministic tiebreak, not a causal claim.
    Audit reports SHOULD present such events as "concurrent,
    arbitrarily ordered" rather than implying temporal sequence.
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
     then byte-wise lexicographic comparison of the 32-byte author_event_hash
  3. Detect concurrent conflicts on overlapping fields
  4. For non-conflicting events: issue canonical receipts in sorted order
  5. For conflict-sensitive overlaps: require explicit `ledger.merge`
     resolution before sequencing can continue past that frontier

FEL-calculated fields are always conflict-sensitive because their
causal chain matters. A stale auto-calculate overwriting a manual
entry is a due process issue in Medicaid adjudication.
```

Using the full 32-byte `author_event_hash` in each dependency is intentional. This removes collision ambiguity from the normative merge path; bandwidth optimization, if needed later, belongs in transport compression rather than the on-disk/on-wire dependency identifier.

Overflow behavior (max 8 causal_deps exceeded):
  If a client would exceed 8 causal dependencies, it MUST use one
  of the following strategies (in order of preference):

  Strategy A: Server sync (when online)
    1. Sync with the server to obtain canonical receipts for all
       pending local events
    2. The canonical receipt chain collapses the DAG: the client's
       latest receipt_hash represents the entire prior causal history
    3. Create the new event with a single causal dep pointing to
       the latest canonical receipt's author_event_hash

  Strategy B: Local consolidation (when offline)
    1. Create a local `draft.consolidated` event whose causal_deps
       are the 8 oldest pending deps
    2. This event carries no payload change (empty change_log,
       field_snapshot reflects current state)
    3. The event is signed and hashed normally
    4. Subsequent events reference draft.consolidated instead of
       the individual deps it absorbed
    5. On next sync, draft.consolidated is submitted like any
       other event; the server issues a canonical receipt for it

    This bounds local DAG width without requiring connectivity.
    The consolidated event is an explicit causal merge point,
    not a silent dep drop.

    Maximum local consolidation depth: 4 (configurable).
    If a client reaches 4 nested consolidations without syncing,
    it MUST block new event creation and require sync.
    This prevents unbounded local DAG growth.

  The client MUST NOT silently drop causal dependencies. Dropping
  deps can mask conflicts that the merge protocol is designed to detect.

### Tag commitment (privacy fix)

Tags like `determination` and `adverse-decision` are NO LONGER plaintext. Knowing "this person received an adverse Medicaid determination on March 15" from headers alone is a HIPAA-relevant disclosure.

```
Envelope contains:  tag_commitment =
                      SHA-256(dCBOR([tag_bitfield: u16, tag_nonce: bstr(16)]))
Payload contains:   tag_nonce: [u8; 16], tag_bitfield: u16

Verification (by authorized party who decrypted the payload):
  expected = SHA-256(dCBOR([decrypted.tag_bitfield, decrypted.tag_nonce]))
  assert constant_time_eq(envelope.tag_commitment, expected)

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

Blinding factors stored in encrypted payload (not in envelope).
Commitment schemas are versioned -- adding a field to an event type
requires a new schema version (append-only; old events keep old count).

Scalar encoding (normative):
  Each committed field declares its encoding in the schema:
    { field_path: "income.monthly",
      generator_index: 0,
      scalar_encoding: "fixed_point",
      precision: 2,          -- decimal places (cents for USD)
      signed: false,         -- unsigned: values >= 0 only
      range_max: 2^53 - 1 }  -- max representable value
  Supported encodings:
    fixed_point: value * 10^precision, stored as unsigned integer
    integer:     raw integer value, stored directly
    boolean:     0 or 1
  The encoding MUST be identical across all implementations for
  homomorphic sums to be meaningful. Two implementations committing
  the same income value MUST produce the same Ristretto scalar.

Example: event type 0x0042 (income determination) has 8 fields.
  Every income determination event: 8 × 32 = 256 bytes of commitments.
  Observer sees 8 commitments, learns nothing about which fields filled.

Generator derivation (normative):

  For event_type t and generator_index i:
    G_i = ristretto255_hash_to_group(
      "formspec-ledger-pedersen-generator-v1" || u16_le(t) || u16_le(i)
    )

  The blinding generator H is:
    H = ristretto255_hash_to_group(
      "formspec-ledger-pedersen-blinding-v1"
    )

  hash_to_group uses the Elligator2 map from RFC 9380 §6.8.2
  (ristretto255). All implementations MUST produce identical
  generators for the same (t, i) inputs.

  Generator precomputation:
    Generators for all registered event types are precomputed at
    startup and cached. The CI golden test suite MUST verify
    generator identity across native and WASM targets.

Cross-version aggregation:

  When aggregating commitments across events with different
  commitment schema versions for the same event type:

  1. Identify the union of all field positions across all
     schema versions (append-only: v2 is a superset of v1)
  2. For events under an older schema version, treat slots
     beyond that version's count as the group identity element
     (the zero point on Ristretto)
  3. Sum commitments at each slot position independently

  Example:
    v1 schema: 4 slots (income fields)
    v2 schema: 6 slots (income + new deduction fields)
    Aggregating 100 v1 events + 50 v2 events:
      Slots 0-3: sum all 150 commitments
      Slots 4-5: sum only the 50 v2 commitments
                 (v1 events contribute identity, which is
                  the additive identity for Ristretto points)

  The identity element is NOT commitment-to-zero-with-random-blinding.
  It is the actual group identity (the zero point on Ristretto).
  This is distinct from the privacy-preserving zero used in
  individual events. For aggregation, we need the additive identity
  so missing slots contribute nothing to the sum.

  Projection pipeline MUST track schema_version per event and
  apply the correct alignment before aggregating.
```

### What stays plaintext (receipt/envelope) vs. what gets encrypted (payload)

| Data | Location | Reason |
|------|----------|--------|
| Event type (u16 registry index) | Envelope | Structural verification and commitment schema lookup |
| HLC timestamp | Envelope | Causal ordering must be verifiable |
| Actor type (`human`, `agent`, `system`) | Envelope | AI disclosure is structural (OMB M-24-10) |
| Schema version | Envelope | Forward compatibility |
| Canonical sequence + receipt chain references | Receipt | Verification requires these |
| Governance check result (pass/fail) | Envelope | Deontic compliance is structural |
| Key custody mode (sovereign/threshold_custodial/delegated) | Envelope | Sovereignty posture is auditable without decryption |
| Tag commitment `SHA-256(dCBOR([tags, nonce]))` | Envelope | Hash-committed: verifiers with payload access can check; observers learn nothing |
| Tag bitfield + nonce | Payload | Actual tags readable only after decryption |
| Pedersen commitments (fixed-position vector) | Envelope | Aggregation without decryption; count fixed per event type |
| Pedersen blinding factors | Payload | Required for opening commitments; only visible to key holders |
| Actor identity (`angela.martinez`) | Payload | PII |
| Case file field values | Payload | PII/PHI |
| Determination rationale | Payload | Case-specific reasoning |
| Document content and attachments | Payload | Uploaded pay stubs, medical records |
| Confidence details (per-field scores) | Payload | May reveal case complexity |
| Equity monitoring dimensions | Payload | Demographic data is sensitive |
| Delegation chain details | Payload | Who delegated to whom may be sensitive |
| Actor correlation ID | Envelope | Per-invocation ID for AI/system audit without decryption |

**Event type granularity and privacy:** The event type registry is published (verifiers need it). An observer with registry access can map u16 indices to semantic names. Event types MUST be defined at a granularity that does not reveal case outcomes. Use `determination` — not `determination.adverse` vs `determination.favorable`. Outcome-specific information belongs in the encrypted payload. Types within the same commitment schema SHOULD be indistinguishable by commitment count or slot layout, so that the fixed-position Pedersen vector leaks no outcome signal.

**Security note on `payload_plaintext_commitment`:** The `commitment_nonce` (16 bytes, stored inside the encrypted payload) prevents brute-force recovery of low-entropy payloads by external observers. Without the nonce, a boolean eligibility field has 2 possible hashes — trivially recoverable. The nonce does NOT protect against a key-holding attacker who can decrypt the payload and read the plaintext directly. The commitment IS useful for key holders as an integrity check: "this envelope commits to exactly this plaintext." It detects payload substitution (swapping one encrypted blob for another under the same DEK) because the substituted payload would have a different commitment. In summary: anti-brute-force for external observers, integrity-check for key holders, not a confidentiality boundary against key compromise.

### Canonical serialization (expert panel critical fix)

All structured data serialized via **deterministic CBOR** (RFC 8949 Core Deterministic Encoding):
- Map keys sorted lexicographically
- Minimal-length integer encoding
- No indefinite-length items

Crate selection requires care. `ciborium` implements CBOR but does not enforce Core Deterministic Encoding by default — map key ordering and minimal integer encoding are not guaranteed without a post-serialization canonical sort pass or a wrapper that enforces RFC 8949 §4.2 rules. Options:
  - `ciborium` with a post-serialization canonicalization layer (sort + re-encode)
  - `coset`'s internal CBOR handling (already produces canonical CBOR for COSE)
  - `cbor4ii` with explicit deterministic mode
  - Custom serializer built on `minicbor` (no-std/WASM, but manual enforcement)
Whichever crate is chosen, the CI suite MUST include a cross-target golden test: serialize the same structured payload on native and WASM, assert byte-identical output. Non-canonical CBOR breaks the integrity model — two semantically identical payloads would produce different hashes.

Hash inputs are always over the CBOR-encoded bytes, never over concatenated strings. This eliminates the domain separation / parsing ambiguity the cryptographer flagged.

---

## 3. Cryptographic Stack

Every crate compiles to both native and `wasm32-unknown-unknown`.

| Component | Native crate | WASM crate | Purpose |
|-----------|-------------|------------|---------|
| SHA-256 hashing | `ring` | `sha2` | Hash chain, content addressing, Merkle leaves |
| AES-256-GCM | `ring` | `aes-gcm` | Payload encryption (per-event DEK) |
| Ed25519 signing | `ed25519-dalek` | `ed25519-dalek` | Per-event signatures (respondent + platform) |
| FROST threshold Ed25519 | `frost-ed25519` | `frost-ed25519` | Threshold signing for custodial mode (RFC 9591, 2-of-3) |
| X25519 ECDH | `x25519-dalek` | `x25519-dalek` | HPKE key wrapping (DHKEM) |
| HKDF key derivation | `hkdf` + `sha2` | `hkdf` + `sha2` | Derive signing/encryption keys from WebAuthn PRF |
| HPKE key wrapping (RFC 9180) | `hpke` (rust-hpke) | `hpke` (rust-hpke) | DEK wrapping: DHKEM(X25519, HKDF-SHA256), HKDF-SHA256, AES-256-GCM |
| Pedersen commitments | `curve25519-dalek` (Ristretto) | `curve25519-dalek` (Ristretto) | Homomorphic aggregation over encrypted numerics |
| COSE checkpoint signing | `coset` (Google) | `coset` (Google) | Signed tree heads at checkpoints |
| Merkle tree | `ct_merkle` | `ct_merkle` | RFC 6962 history tree, inclusion/consistency proofs |
| BBS+ signatures | `bbs_plus` | `bbs_plus` | Issuer-backed disclosure attestations + selective proofs |
| SD-JWT | `sd-jwt` or custom | `sd-jwt` or custom | Disclosure-attestation fallback for IETF-only environments |
| Deterministic CBOR | See §Canonical Serialization | See §Canonical Serialization | Canonical serialization for all structured data |
| OpenTimestamps | HTTP client (REST API) | N/A | Bitcoin-anchored timestamps for checkpoints (server only) |

`ring` is used on native for SHA-256 and AES-256-GCM because it delegates to platform-optimized assembly (AES-NI, SHA extensions). On WASM, `ring`'s target support has been unreliable (see issue #918); pure-Rust crates (`sha2`, `aes-gcm`) are the primary WASM path, not a fallback. Both backends are abstracted behind a `CryptoPrimitives` trait in `ledger-engine`.

`hpke` (rust-hpke) is used for HPKE key wrapping rather than hand-composing from `x25519-dalek` + `hkdf` + `aes-gcm`. RFC 9180 defines specific KEM encapsulation steps, key schedule derivation, and AEAD sealing with precise domain separators and label strings. Composing these from primitive crates risks subtle non-compliance (wrong label ordering, missing serialized context, incorrect KDF extraction steps). The `hpke` crate implements RFC 9180 directly, compiles to WASM, and is tested against the RFC's published test vectors. Spike 1 MUST include `hpke` in the WASM bundle size measurement.

### WASM CI gate

CI MUST build and test `ledger-engine` on `wasm32-unknown-unknown` with the exact primitives required for browser mode: SHA-256 (`sha2`), AES-256-GCM (`aes-gcm`), Ed25519, X25519, HPKE key wrapping (`hpke`), deterministic CBOR (crate selected by Spike 0), and Merkle verification. The `CryptoPrimitives` trait in `ledger-engine` abstracts `hash`, `encrypt`, `decrypt`, `sign`, `verify` with two implementations: `NativePrimitives` (`ring`-backed, server) and `WasmPrimitives` (pure-Rust-backed, browser). Both backends MUST pass the same golden test suite: serialize, encrypt, sign, and hash identical inputs on native and WASM, assert byte-identical outputs. The WASM build uses `sha2` + `aes-gcm` as the primary path; `ring` is a native-only performance optimization, not a correctness dependency. The `hpke` crate is used on both targets (it is pure Rust and WASM-compatible).

### Browser disclosure feasibility gate

BBS+ is not locked into the browser path until a spike demonstrates all of the following on a mid-tier phone:

- Added compressed bundle size `<= 350 KB`
- Cold start overhead `<= 300 ms`
- Proof generation for `<= 64` messages `<= 1.5 s`
- Peak memory `<= 64 MB`

If the spike misses any gate, browser mode falls back to server-issued proofs only or to `SD-JWT`-only deployments, while the core ledger format remains unchanged.

### Total WASM bundle size gate

The combined `ledger-wasm` output (crypto engine + event pipeline + Merkle verification + CBOR + identity, excluding BBS+ which has its own gate above) MUST NOT exceed **1 MB compressed** (brotli). This is the total for the ledger WASM module; the Formspec engine WASM module is separate. If the combined bundle exceeds this gate, BBS+/disclosure modules MUST be split into a lazy-loaded secondary WASM module that is fetched only when disclosure proof derivation is needed on the client.

### HPKE key wrapping invariant

All DEK wrapping operations use **HPKE** (RFC 9180) with suite `DHKEM(X25519, HKDF-SHA256), HKDF-SHA256, AES-256-GCM` in **Base mode** (no PSK, no Auth). This applies to initial event creation (wrapping for respondent + ledger-service) AND post-hoc access grants.

Every wrapping operation MUST use a fresh ephemeral X25519 keypair. The ephemeral keypair is generated, used once to wrap a single DEK for a single recipient, then destroyed.

The type system enforces this: the `EphemeralX25519Keypair` type is `!Clone + !Copy` and is consumed (moved) by the single `wrap_dek()` call. Reusing an ephemeral key is a compile error, not a runtime bug. Each key bag entry stores the 32-byte ephemeral public key so the recipient can perform the ECDH derivation.

Authentication note: HPKE Base mode provides no sender authentication at the key-wrapping layer. A party who knows a recipient's public key could construct a valid HPKE encapsulation wrapping a chosen DEK. This is acceptable because: (1) the `author_event_hash` covers the `key_bag_cbor`, (2) the author event envelope is Ed25519-signed, (3) therefore any modification to the key bag invalidates the event signature. An attacker would need to forge the Ed25519 signature to substitute a key bag entry. HPKE AuthMode is unnecessary given the envelope signature. Using Base mode simplifies the protocol (no sender key management for HPKE) without reducing security, because authentication is provided at the envelope layer.

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

Anchoring fallback:
  OpenTimestamps depends on volunteer calendar servers and Bitcoin
  confirmation (~10 min average, potentially hours under congestion).
  If the OTS calendar is unreachable or confirmation exceeds 6 hours:
    a. The COSE-signed tree head is authoritative by itself (platform
       signature). OTS adds external witness, not primary integrity.
    b. Retry OTS submission on the next checkpoint cycle.
    c. Maximum pending (unanchored) checkpoints: 5. If exceeded,
       log a warning and continue without OTS anchoring until the
       calendar is reachable. Do NOT block checkpoint creation.
    d. Deployments requiring external anchoring beyond OTS MAY
       configure additional witness services (Ethereum attestation,
       RFC 3161 TSA, or tenant-operated anchoring).
  OTS is a witness service, not an authoritative time source. The
  signed tree head from the platform sequencer is the trust anchor.
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

### Blob integrity scan (periodic)

```
Schedule: daily (configurable per deployment)

For each ledger with events in the scan window:
  1. Query all payload_cid values from ledger_author_events
  2. For each payload_cid, call primary_store.exists(cid)
  3. If missing from primary:
     a. Attempt recovery from any replica that has it
     b. If recovered: backfill to primary
     c. If unrecoverable: log as DATA_LOSS, alert,
        mark event as blob_missing in a separate tracking table
  4. For a configurable sample (default 1%):
     a. Fetch the blob, compute SHA-256(ciphertext)
     b. Verify it matches the payload_cid
     c. If mismatch: bit-rot detected, attempt replica recovery

This catches silent data loss (storage corruption, incomplete
writes, cloud provider failures) before it becomes permanent.
```

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

-- Prevent TRUNCATE (triggers above do not cover TRUNCATE)
CREATE TRIGGER no_author_event_truncate BEFORE TRUNCATE ON ledger_author_events
    EXECUTE FUNCTION prevent_ledger_mutation();
CREATE TRIGGER no_receipt_truncate BEFORE TRUNCATE ON ledger_receipts
    EXECUTE FUNCTION prevent_ledger_mutation();
```

Defense-in-depth note: The `prevent_ledger_mutation()` trigger blocks application-level UPDATE, DELETE, and TRUNCATE. It does NOT prevent a superuser or DDL-privileged role from dropping the trigger, nor does it prevent direct WAL manipulation. For FedRAMP deployments, append-only semantics additionally require:

- WAL-level audit logging (`pgaudit`)
- Restricted DDL privileges (no `ALTER TABLE` for application roles)
- Logical replication to an independently-administered audit replica (different admin credentials, separate infrastructure)
- Row-level security policies preventing application roles from inspecting trigger definitions

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

| Tier | Payload backends | Key plane | Canonical receipts |
|------|-----------------|-----------|--------------------|
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
|  |  sha2 + aes-gcm: SHA-256 hashing, AES-256-GCM        | |
|  |    (pure Rust; ring is native-only, see §3)           | |
|  |  curve25519-dalek: Pedersen commitments               | |
|  |  bbs_plus: proof derivation + verification            | |
|  |  ct_merkle: local Merkle tree + proof verification    | |
|  |  deterministic CBOR (see §Canonical Serialization)    | |
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
     b. Generate random commitment_nonce (16 bytes), include in payload
        Compute payload_plaintext_commitment = SHA-256(plaintext_cbor || commitment_nonce)
     c. Compute fixed-position Pedersen commitments                ~50us
     d. Generate random DEK (AES-256 key)
     e. Encrypt CBOR payload with DEK (AES-256-GCM, fresh random 96-bit nonce)
     f. Compute payload_ciphertext_id = SHA-256(ciphertext)
     g. Wrap DEK with fresh ephemeral per recipient:
        - Respondent's X25519 public key (HPKE)                   ~50us
        - Ledger public key (HPKE, version tracked)               ~50us
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
|  |  Activity results carry author_event_hash (determin-  | |
|  |    istic), NOT sequence/checkpoint (server-assigned)  | |
|  |  Append is idempotent (deduplicate by activity ID     | |
|  |    and author_event_hash)                             | |
|  |  On retry: same author_event_hash → same event,      | |
|  |    canonical receipt looked up after the fact          | |
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
  7. Return author_event_hash to Temporal activity result
     (author_event_hash is deterministic; sequence and checkpoint
     are server-assigned and may differ on retry. The activity
     result MUST carry the deterministic identifier, not the
     server-assigned one. Canonical receipt is looked up after.)
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

### Deferred verification (`ciphertext_only` → `verified_payload`)

Events ingested in `Relaxed` mode carry `verification_state = ciphertext_only` in their canonical receipt. To upgrade:

```
The server performs strict verification at a later time:
  1. Unwrap DEK via LAK
  2. Decrypt payload
  3. Verify payload_plaintext_commitment (including commitment_nonce)
  4. Verify tag_commitment
  5. Verify Pedersen openings against commitment schema
  6. Append a `verification.completed` event to the ledger:
     {
       target_author_event_hash: [u8; 32],
       target_sequence: u64,
       verification_result: pass | fail,
       failure_reason: Option<String>,  // if fail
     }
  7. Projection pipeline processes the verification event:
     - On pass: the target event becomes eligible for commitment-driven
       analytics, disclosure issuance, and adverse-action workflows.
       Projections that skipped it are incrementally updated.
     - On fail: the target event is permanently quarantined.
       An alert is raised for investigation.

The canonical receipt for the original event is NOT rewritten.
The verification.completed event is the upgrade record.
```

### Client -> Server (intake events)

The client creates key bag entries for BOTH respondent and ledger-service at event creation time, using each recipient's public key via HPKE/X25519. The server never sees the plaintext DEK on the wire.

```
Client-side key bag construction at event creation:

  key_bag = [
    { recipient: "respondent",
      ephemeral_pubkey: ...,   // fresh per wrapping
      wrapped_dek: HPKE(DEK, respondent_x25519_pub) },
    { recipient: "ledger-service",
      ledger_key_version: 3,   // tracked for rotation
      ephemeral_pubkey: ...,   // fresh per wrapping
      wrapped_dek: HPKE(DEK, ledger_public_key_v3) },
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
- The sequencer MUST topologically sort by `(hlc.wall_ms, hlc.logical, byte-wise lexicographic author_event_hash)`. "Byte-wise lexicographic" means comparing the raw 32-byte hash arrays element-by-element from index 0; this is NOT string comparison of hex-encoded representations.
- Receipt time, arrival order, worker count, and server build differences MUST NOT affect canonical order.
- When two concurrent events with no causal relationship touch overlapping fields:
  - If all overlaps are marked `last_writer_wins`, the sequencer MAY linearize them with the deterministic rule above.
  - If any overlap is `conflict-sensitive`, the server MUST stop sequencing past that conflicting frontier until an explicit `ledger.merge` event is appended.
- `ledger.merge` is itself an author event plus canonical receipt. It names the conflicting `author_event_hash` values and commits the chosen resolved value or merge rationale.
- Events with `verification_state = ciphertext_only` (relaxed ingest) MUST NOT participate in conflict-sensitive field overlap detection. The server cannot determine which fields a `ciphertext_only` event touches without decrypting it. The sequencer uses **event type range scoping** to narrow the potential overlap: a `ciphertext_only` event blocks the conflict-sensitive frontier ONLY for event types within its own range (intake 0x0001-0x00FF, governance 0x0100-0x01FF, etc.), because event types from different ranges operate on distinct field namespaces by schema construction. Specifically: intake events operate on the **response field namespace** (respondent-authored field values); governance events operate on the **case field namespace** (platform-authored case state). The `case.created` event (0x0100) contains a `case_file_mapping` that copies values from the response namespace into the case namespace, but subsequent governance events mutate case fields, not response fields. A concurrent intake `setData` and governance `wos.case.mutated` therefore touch different namespaces and cannot conflict, even though the case field's original value was derived from a response field. The event type is plaintext in the envelope and sufficient for this narrowing. If the event type is in the tenant-defined range (0x1000+), the sequencer MUST treat it as potentially touching ALL conflict-sensitive fields (conservative default for unknown schemas). Tenants MAY further narrow the scope by registering per-event-type field overlap declarations in the event type registry. Within the scoped range: if any concurrent event in the same frontier touches conflict-sensitive fields AND a `ciphertext_only` event is in that frontier, the sequencer MUST stop and require either (a) strict verification of the `ciphertext_only` event, upgrading it to `verified_payload`, or (b) an explicit `ledger.merge` event that acknowledges the unverified event. This prevents silent data loss from unverified payloads overwriting conflict-sensitive values while avoiding cross-range false positives.
- **Frontier stall circuit breaker:** A `ciphertext_only` event blocking the conflict-sensitive frontier creates a liveness hazard — a slow or misbehaving client can stall conflict resolution for all concurrent activity on that ledger. To bound this: if a `ciphertext_only` event blocks the frontier for longer than `MAX_FRONTIER_BLOCK_DURATION` (default: 15 minutes, tenant-configurable up to 4 hours), the server MUST force strict verification by unwrapping the DEK via LAK and decrypting the payload. If strict verification succeeds, the event is upgraded to `verified_payload` and sequencing resumes. If strict verification fails (commitment mismatch, schema violation), the event is permanently quarantined and the frontier advances past it. The server MUST append a `verification.completed` event recording the forced verification and its outcome. This mirrors the `PendingDependency` timeout model (24-hour buffer above) applied to the orthogonal problem of unverified payloads blocking conflict resolution.

### Ingestion rate limits

```
Per-ledger:
  Max events per sync request: 500
  Max sync requests per minute per ledger: 60
  Max pending (unsequenced) events per ledger: 10,000

Per-respondent (across all ledgers):
  Max sync requests per minute: 120
  Max total unsequenced events: 50,000

Per-deployment:
  Configurable burst and sustained rate limits at the
  HTTP gateway layer.

Cost model (informational):
  Each strict-ingest event costs:
    1 Ed25519 verify (~10us)
    2 SHA-256 hashes (~1us each)
    1 HPKE unwrap (~50us)
    1 AES-256-GCM decrypt (payload-size dependent)
    1-N Pedersen verification (~50us each)
    1 Postgres insert
  Total: ~200us compute + I/O

  At 500 events/sec sustained, one core handles ingestion.
  Rate limits exist to bound queue depth, not compute cost.
```

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

## 8b. WOS Provenance Integration

The existing `wos-core` crate has a `ProvenanceLog` containing `ProvenanceRecord` values (flat structs with `record_kind`, `actor_id`, `from_state`, `to_state`, `event`, `data`). These are unencrypted, unsigned, unchained records. The unified ledger replaces this model entirely for production deployments.

### Migration path

```
Phase 1: ProvenanceRecord becomes a ledger event payload type

  The existing ProvenanceRecord fields map to the encrypted payload
  of a ledger event:
    ProvenanceRecord.record_kind  -> event_type (u16 in envelope)
    ProvenanceRecord.actor_id     -> actor identity (encrypted payload)
    ProvenanceRecord.from_state   -> payload field
    ProvenanceRecord.to_state     -> payload field
    ProvenanceRecord.event        -> payload field
    ProvenanceRecord.data         -> payload field

  The mapping:
    ProvenanceKind::StateTransition  -> event_type: wos.transition.fired
    ProvenanceKind::CaseStateMutation -> event_type: wos.case.mutated
    ProvenanceKind::TimerCreated     -> event_type: wos.timer.created
    ProvenanceKind::TimerFired       -> event_type: wos.timer.fired
    ProvenanceKind::TimerCancelled   -> event_type: wos.timer.cancelled
    ProvenanceKind::OnEntry          -> event_type: wos.lifecycle.onEntry
    ProvenanceKind::OnExit           -> event_type: wos.lifecycle.onExit
    ProvenanceKind::ActionExecuted   -> event_type: wos.action.executed
    ProvenanceKind::UnmatchedEvent   -> event_type: wos.event.unmatched
    ProvenanceKind::InvalidDuration  -> event_type: wos.timer.invalidDuration

  The 10 mappings above cover the core lifecycle variants. The remaining
  137 ProvenanceKind variants (15 categories) collapse into parent event
  types with the variant kind as a payload-level discriminator:

    Deontic enforcement (6 variants: DeonticViolation, DeonticEvaluation,
      DeonticResolution, DeonticBypass, RightsViolation, ConsistencyViolation)
      -> event_type: wos.deontic.evaluated
         Payload: { kind: "violation"|"evaluation"|"resolution"|"bypass"|
                          "rights_violation"|"consistency_violation", ... }

    Autonomy tracking (7 variants: AutonomyViolation, AutonomyCapped,
      AutonomyComputed, HumanTaskCreated, ToolViolation, EscalationPending,
      AutonomyDemotion)
      -> event_type: wos.autonomy.evaluated
         Payload: { kind: "violation"|"capped"|"computed"|"human_task"|
                          "tool_violation"|"escalation"|"demotion", ... }

    Confidence (5 variants: ConfidenceViolation, ConfidenceDecay,
      CumulativeConfidenceViolation, SessionPaused, GroundTruthLabel)
      -> event_type: wos.confidence.evaluated
         Payload: { kind: "violation"|"decay"|"cumulative"|"paused"|
                          "ground_truth", ... }

    Agent lifecycle (10 variants: AgentOutput, ActorTypeViolation,
      AgentProvenanceAnnotation, AgentVersionChange, NarrativeTierRecorded,
      ConstraintTamperBlocked, DriftReclassification, AgentStateTransition,
      ProxyInvocation, DispositiveViolation)
      -> event_type: wos.agent.lifecycle
         Payload: { kind: <variant_name_snake_case>, ... }

    Fallback (3 variants)
      -> event_type: wos.fallback.evaluated
         Payload: { kind: "triggered"|"attempt"|"terminal", ... }

    Due process (8 variants: NoticeSent, SeparationViolation, AppealFiled,
      ProtocolViolation, IndependentFirstEnforced, SamplingDecision,
      OverrideViolation, OverrideRecorded)
      -> event_type: wos.due_process.evaluated
         Payload: { kind: <variant_name_snake_case>, ... }

    Pipeline (5 variants)
      -> event_type: wos.pipeline.evaluated
         Payload: { kind: <variant_name_snake_case>, ... }

    Compensation (3 variants)
      -> event_type: wos.compensation.evaluated
         Payload: { kind: <variant_name_snake_case>, ... }

    Delegation (1 variant)
      -> event_type: wos.delegation.evaluated

    Durability (6 variants)
      -> event_type: wos.durability.evaluated
         Payload: { kind: <variant_name_snake_case>, ... }

    DCR (5 variants)
      -> event_type: wos.dcr.evaluated
         Payload: { kind: <variant_name_snake_case>, ... }

    Verification (3 variants)
      -> event_type: wos.verification.evaluated
         Payload: { kind: <variant_name_snake_case>, ... }

    Sidecar (2 variants)
      -> event_type: wos.sidecar.evaluated
         Payload: { kind: <variant_name_snake_case>, ... }

    Relationship (1 variant)
      -> event_type: wos.relationship.changed

    Tolerance (1 variant: ToleranceViolation)
      -> event_type: wos.timer.tolerance_violated

  This collapse strategy groups related variants under parent event types
  where they share the same commitment schema layout and privacy tier.
  The payload-level `kind` discriminator preserves the full 147-variant
  taxonomy for conformance assertion and audit trail without inflating
  the u16 event type registry. The conformance adapter (Phase 3) maps
  back from (event_type, payload.kind) -> ProvenanceKind for assertion
  matching.

  Total event types from WOS provenance: ~25 parent types (the 10 direct
  mappings above + ~15 collapsed category types). Combined with the ~15
  intake/access/system types, this reaches the ~50 total target.

Phase 2: wos-core Evaluator emits ledger events

  The Evaluator (wos-core/src/eval.rs) currently pushes ProvenanceRecords
  to ProvenanceLog (an in-memory Vec). In production, the Evaluator
  instead calls into ledger-engine's EventBuilder to construct signed,
  encrypted, chained events.

  For conformance testing, ProvenanceLog remains as a lightweight
  adapter that extracts the equivalent fields from ledger events
  (or runs in a test-only mode that skips encryption/signing).

Phase 3: Conformance fixture migration

  The 100+ conformance test fixtures assert on ProvenanceRecord values
  (record_kind, from_state, to_state, event). These assertions remain
  valid — they verify the same fields, now extracted from decrypted
  ledger event payloads instead of raw ProvenanceRecord structs.

  The conformance harness (wos-conformance) wraps the Evaluator with
  a test adapter that decrypts events and maps them back to the
  assertion format. No fixture rewrite needed.
```

### Scope boundary

The unified ledger is a Formspec + WOS artifact. It lives in the Formspec codebase (`crates/ledger-*`), not in `wos-spec/crates/`. WOS specs reference the ledger through the `provenanceLayer` seam (Kernel S10.3). The kernel does not depend on or specify the ledger's cryptographic format — it specifies what provenance records must contain (S8.2). The ledger is an implementation of the provenance storage layer, not a modification of the kernel's processing model.

---

## 9. Materialized Views & Projections

### View definitions

| View | Source events | Decryption needed? | Purpose |
|------|-------------|-------------------|---------|
| Case index | `case.created` + latest `wos.transition.fired` | No (receipt/envelope fields) | Dashboard: "Angela's 47 pending cases" |
| Task queue | `wos.task.*` events | Partial (task metadata in receipt/envelope, details encrypted) | Reviewer work queue |
| Current case file | `setData` mutations within events | Yes (field values are encrypted) | "What is the current income value?" |
| SLA status | `wos.task.created` + `wos.timer.*` + `wos.task.completed` | No (receipt/envelope timestamps) | SLA breach warnings |
| Equity metrics | `wos.transition.fired` on determination-tagged events | Strict-verified Pedersen commitments (no decryption at query time) | Disparity monitoring via homomorphic aggregation |
| Analytics | All events | Strict-verified commitments + receipt/envelope fields | Completion rates, time-to-determination |
| Audit trail | All receipts + envelopes | No | Complete structural history |

Only events with `verification_state = verified_payload` may contribute commitments to analytics, selective-disclosure issuance, or adverse-action workflows. `ciphertext_only` events remain visible in the audit trail but are operationally quarantined until strict verification completes.

### Projection pipeline

```
Postgres LISTEN/NOTIFY (as latency hint) + durable polling (as authority)

LISTEN/NOTIFY is best-effort: notifications are lost if no listener
is connected, if the connection drops, or if the connection pool
recycles. Use NOTIFY as a hint to reduce latency, but poll the
receipt table on a regular interval (e.g., every 5s) as the
authoritative source. Each worker tracks its high-water mark
(last processed sequence) and catches up from there on restart
or missed notifications.

Worker:
  0. On startup or reconnect: query ledger_receipts WHERE sequence >
     last_processed_sequence. Process any missed events before
     switching to NOTIFY-driven mode.
  1. Receive event notification (or poll tick)
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

### Projection integrity verification

```
On demand or scheduled (e.g., daily):
  1. Select a random sample of N events (default: 1000)
     or a specific sequence range
  2. For each sampled event:
     a. Rebuild the expected projection state from the
        immutable author event + receipt + access events
     b. Compare against the current projection table rows
     c. Record any divergence
  3. If divergence > 0:
     a. Log the divergent event sequences
     b. Trigger a targeted rebuild from the nearest epoch
        snapshot for the affected ledger(s)
     c. Alert: projection corruption detected

Full rebuild verification:
  On epoch snapshot creation, hash the snapshot contents.
  Store snapshot_hash in the checkpoint event.
  On rebuild, verify the rebuilt snapshot matches the
  stored hash before serving from it.

The access_grant_entries table specifically:
  Rebuild from ledger by replaying all access.granted and
  access.revoked events. Compare row-by-row against current
  projection. This is the most sensitive projection because
  incorrect grants = unauthorized access. Verification of
  access_grant_entries SHOULD run at higher frequency
  (hourly) than general projection verification.
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
    For threshold custodial keys: destroy the platform's FROST share
    (this alone renders the custodial key irrecoverable; escrow shares
    become useless without the platform share, but escrow services
    SHOULD also destroy their shares for defense in depth).
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

Erasure modes (deployment-configured):

  Standard (default):
    Retains envelopes + final checkpoint/tombstone.
    Envelopes become metadata skeletons: event types, HLC timestamps,
    actor types, governance results, Pedersen commitments remain.
    Sufficient for: proof of prior existence, chain integrity
    verification, aggregate commitment analytics (if commitments
    were verified before erasure).
    Limitation: the retained skeleton reveals case complexity
    (event count), temporal patterns, actor type distribution, and
    outcome-adjacent signals (event types, governance results).
    Under strict CJEU interpretation of "personal data" (any
    information relating to an identified person), this metadata
    may not satisfy a right-to-be-forgotten request.

  Full structural erasure:
    Deletes envelopes, receipts, key bags, AND blobs.
    Retains ONLY the final Merkle root hash, the signed tree head,
    the OTS anchor proof, and the `ledger.erased` tombstone.
    This breaks chain reverification (individual events can no
    longer be verified) but satisfies strict erasure requirements.
    The final Merkle root proves the chain existed and was intact
    at erasure time, without retaining any per-event metadata.
    Deployments in EU jurisdictions SHOULD evaluate whether
    Standard mode is sufficient for their DPA's interpretation.
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

### Disclosure attestation issuance timing

```
Attestation issuance modes:

  Eager (default for strict-ingest events):
    Disclosure attestation is minted as part of the ingest
    pipeline, immediately after strict verification succeeds.
    The disclosure.attested event is appended in the same
    Postgres transaction as the canonical receipt.
    Added latency: ~5ms per event (BBS+ sign over message vector).

  Lazy (for relaxed-ingest or on-demand):
    Attestation is minted when first requested for a specific
    event. Requires strict verification first (upgrading
    ciphertext_only to verified_payload if needed).
    First-request latency: strict verification + BBS+ sign.
    Subsequent requests use cached attestation.

  Configuration:
    Default: eager for all strict-ingest events.
    Deployments where 5ms ingest overhead is unacceptable
    MAY switch to lazy mode, accepting that the first
    disclosure request for an event will be slower.
```

### BBS+ disclosure attestation and proof flow

```
At event creation:
  Core ledger event is signed only with Ed25519 and appended normally.

After ingest (server or designated issuer):
  Build disclosure message vector from the decrypted payload:
    messages = [target_author_event_hash, actor_id, field_1, field_2, ..., field_N]
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
    /// Whether proofs from this backend are unlinkable
    /// (two proofs from the same attestation cannot be correlated).
    fn is_unlinkable(&self) -> bool;

    fn sign(&self, fields: &[Field], key: &SigningKey) -> Result<Attestation>;
    fn derive_proof(
        &self,
        attestation: &Attestation,
        fields: &[Field],
        disclose: &[usize],
    ) -> Result<Proof>;
    fn verify_proof(
        &self,
        proof: &Proof,
        disclosed: &[Field],
        pubkey: &PublicKey,
    ) -> Result<VerificationResult>;
}

/// VerificationResult includes:
///   valid: bool,
///   backend_id: &str,        // "bbs_plus" or "sd_jwt"
///   unlinkable: bool,        // from is_unlinkable()
///   key_version: u32,

impl SelectiveDisclosure for BbsPlusBackend {
    fn is_unlinkable(&self) -> bool { true }
    // ...
}
impl SelectiveDisclosure for SdJwtBackend {
    fn is_unlinkable(&self) -> bool { false }
    // ...
}

// Callers who need unlinkability MUST check is_unlinkable()
// before issuing proofs for privacy-sensitive disclosures.
// A deployment configured with SD-JWT that attempts an
// unlinkable disclosure MUST fail loudly, not silently degrade.
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

The architecture must not require a personal device. Medicaid applicants use library computers. The custodial mode uses **threshold cryptography** so that no single party ever holds the full custodial private key.

#### Threshold custodial keys (2-of-3 FROST)

```
Custodial mode: threshold key generation (no single party holds full key)

  1. Respondent authenticates via OIDC (Login.gov/ID.me) -- no WebAuthn
  2. Platform, primary escrow, and backup escrow run Distributed Key
     Generation (FROST DKG, RFC 9591) for Ed25519 signing key:
       Share 1: Platform (KMS, non-extractable)
       Share 2: Primary escrow (independent FedRAMP service, separate KMS)
       Share 3: Backup escrow (deploying government agency, or second
                independent service)
       Quorum: any 2-of-3 can produce a valid Ed25519 signature
       No party ever holds the full private key at any point in the protocol
  3. DKG produces a custodial Ed25519 public key -> did:key:z6Mk...
     This is the custodial DID. Deterministic from the DKG output.
  4. For X25519 encryption: derive X25519 keypair shares via threshold
     ECDH (parallel DKG over Curve25519). Same 2-of-3 quorum.
  5. Events are signed via FROST threshold signing protocol:
       a. Coordinator (platform) initiates signing round
       b. Any 2 of 3 share holders contribute partial signatures
       c. Combined signature is a standard Ed25519 signature,
          indistinguishable from single-signer (verifiers cannot tell)
       d. Round-trip latency: ~100-200ms per signature
  6. DEKs are wrapped to the custodial X25519 public key (HPKE, no
     threshold operation needed -- public key encryption is single-party)
  7. Respondent can later upgrade to sovereign mode (see below)

Share holders in government context:
  Platform:       the SaaS operator (holds Share 1)
  Primary escrow: independent FedRAMP-authorized service (holds Share 2)
  Backup escrow:  the government agency deploying the platform,
                  a regulatory body (e.g., inspector general's office),
                  or a second FedRAMP service (holds Share 3)

The escrow MUST be genuinely independent of the platform operator.
If the platform operator also operates the escrow, the trust model
collapses to single-party custody.
```

#### Custodial key lifecycle events

```
custody.key_generated (0x0409):
  custody_mode:               "threshold_2_of_3"
  custodial_public_key:       Ed25519PublicKey (DKG output)
  custodial_encryption_pubkey: X25519PublicKey (DKG output)
  platform_share_attestation: [u8; 32]  (HSM attestation hash, non-extractable)
  escrow_1_identity:          DID of primary escrow service
  escrow_1_share_attestation: [u8; 32]  (HSM attestation hash)
  escrow_2_identity:          DID of backup escrow
  escrow_2_share_attestation: [u8; 32]  (HSM attestation hash)
  dkg_transcript_hash:        [u8; 32]  (hash of full DKG transcript for audit)

custody.share_destroyed (0x040A):
  party:                      "platform" | "escrow_1" | "escrow_2"
  party_identity:             DID of the party that destroyed
  kms_deletion_receipt:       Vec<u8>  (signed confirmation from that party's KMS)
  kms_deletion_scheduled_at:  u64
  kms_deletion_confirmed_at:  Option<u64>  (None if still in waiting period)

custody.upgraded (0x0408, existing):
  old_custodial_did:          DID (threshold-generated)
  new_sovereign_did:          DID (WebAuthn PRF-derived)
  shares_destroyed:           Vec<custody.share_destroyed event refs>

custody.escrow_reshared (0x040B):
  reason:                     "escrow_decommissioned" | "escrow_rotated"
  old_escrow_identity:        DID of departing escrow
  new_escrow_identity:        DID of replacement escrow
  new_escrow_share_attestation: [u8; 32]
  resharing_transcript_hash:  [u8; 32]
```

#### Sovereignty upgrade path

```
Respondent registers a WebAuthn credential on their own device:

  1. WebAuthn PRF -> HKDF -> sovereign Ed25519 + X25519 key pairs
  2. Platform re-wraps historical DEKs using the LAK (not the custodial key):
     a. Unwrap DEK via LAK private key (server-side, no threshold needed)
     b. Re-wrap DEK with respondent's new sovereign X25519 public key (HPKE)
     c. Append immutable `access.granted` events carrying new wrappings
  3. Respondent now has independent decryption access
  4. Destroy custodial key: each share holder destroys its share
     a. Platform schedules KMS deletion of Share 1
     b. Primary escrow schedules KMS deletion of Share 2
     c. Backup escrow schedules KMS deletion of Share 3
     d. Each destruction appends a custody.share_destroyed event
  5. Append custody.upgraded event referencing the destruction events
  6. New events are signed with respondent's sovereign PRF-derived key

The sovereignty upgrade does NOT require the custodial private key.
The LAK provides a parallel decryption path for re-wrapping. The
custodial key's role in the upgrade is simply: stop signing.
```

#### Trust model comparison

```
Single-party custodial (rejected):
  Trust assumption: "the platform is honest"
  Key destruction: trust the platform's claim
  Adversarial platform: full key compromise (sign, decrypt anything)
  Erasure: trust the platform to destroy
  Audit: trust-based claims only

Threshold custodial (adopted):
  Trust assumption: "at least one of {platform, escrow_1, escrow_2} is honest"
  Key destruction: destroying ANY one share renders the key irrecoverable
  Adversarial platform: platform alone cannot sign or decrypt (needs an escrow)
  Erasure: respondent can request ANY share holder to destroy, bypassing platform
  Audit: custody lifecycle is verifiable on the ledger (DKG attestations,
         share destruction receipts from independent KMS instances)
```

#### Performance characteristics

```
Threshold operations occur ONLY during event signing:
  - ~3-10 threshold signatures per form session (~100-200ms each)
  - Total session overhead: 300ms - 2s (invisible in form-fill context)

Threshold operations do NOT occur during:
  - Projection pipeline decryption (uses LAK, unaffected)
  - Analytics / Pedersen aggregation (no decryption needed)
  - Selective disclosure (uses BBS+ issuer key, unaffected)
  - Sovereignty upgrade re-wrapping (uses LAK, unaffected)

The hot path (projections, analytics, disclosure) is entirely unaffected
by threshold custody.
```

#### Session-scoped signing budget (offline resilience)

```
The escrow must be reachable for each threshold signature. For kiosk
or clinic environments with intermittent connectivity:

  1. At session start (connectivity exists), the platform requests a
     signing budget from the escrow:
       a. Escrow generates N FROST round-1 nonce commitments (typically N=20)
       b. Escrow returns the nonce commitments to the platform
       c. Platform caches them for the session duration
  2. During the session, the platform completes FROST round-2 locally
     using the pre-authorized nonces — no per-event escrow round-trip
  3. When the budget is exhausted, the client syncs and obtains more
  4. The escrow chose to authorize those N signatures; the platform
     cannot forge additional signatures beyond the budget
  5. Budget is bound to a specific session, custodial DID, and time window

This bounds offline signing without breaking the threshold guarantee.
The escrow retains control: it decides how many signatures to authorize,
for which DID, and for how long.
```

#### Escrow resilience

```
2-of-3 quorum survives the loss of any one share holder:

  If the primary escrow is decommissioned:
    1. Platform + backup escrow cooperate to reshare:
       Proactive secret sharing generates a new share for the
       replacement escrow WITHOUT reconstructing the full key
    2. Append custody.escrow_reshared event recording the transition
    3. New escrow receives its share via secure DKG resharing protocol
    4. Old escrow destroys its share (custody.share_destroyed event)
    5. System continues with platform + new escrow + backup escrow

  This handles: escrow service shutdown, FedRAMP deauthorization,
  contractual transitions, disaster recovery.
```

#### GDPR erasure with threshold custody

```
Threshold custody strengthens the respondent's erasure rights:

  Standard erasure flow:
    Respondent requests erasure -> platform initiates §9 GDPR protocol
    (same 6-step flow: request, checkpoint, tombstone, key destruction,
    mapping destruction, state purge)

  Custodial key destruction:
    Destroying ANY one share renders the custodial key irrecoverable.
    The respondent can request ANY share holder to destroy:
      - Ask the platform -> platform destroys Share 1
      - Ask the primary escrow directly -> escrow destroys Share 2
      - Ask the backup escrow directly -> backup destroys Share 3
    Any one destruction is sufficient. The respondent has an independent
    erasure path that bypasses the platform entirely.

  The LAK destruction (for server-side decryption) still follows the
  standard §9 protocol. Threshold custody adds an ADDITIONAL layer:
  even if the LAK is somehow not destroyed, the custodial key is
  independently irrecoverable after any share destruction.
```

#### Custodial-mode device recovery

When a threshold-custodial respondent loses all devices and re-authenticates via OIDC (§1 Step 6), the recovery flow differs from sovereign mode because the "old signing key" is threshold-generated, not PRF-derived.

```
Custodial recovery ordering (strict):

  1. Respondent re-authenticates via OIDC (Login.gov proves identity)
  2. Platform + escrows run NEW FROST DKG for the recovered respondent:
     New custodial Ed25519 + X25519 key pairs (new custodial DID)
  3. ONLY AFTER new DKG completes and the new custodial key signs its
     first event (custody.key_generated for the new key):
     Revoke the old custodial signing_key_id in the signing key registry
  4. Re-grant historical DEKs via LAK (§1 Step 6, same as sovereign):
     a. Append key.recovery_regranted event
     b. Re-wrap DEKs with new custodial X25519 public key
  5. The old custodial key's FROST shares remain in escrow until
     revocation is confirmed, then each share holder destroys its share
     (custody.share_destroyed events)

The window between step 2 (new key active) and step 3 (old key revoked)
MUST be bounded: the server MUST revoke the old signing_key_id within
60 seconds of the new DKG completing. During this window, events signed
with the old key are rejected by the signing key registry (which marks
the old key Revoked at step 3). Events signed with EITHER key during
the window are buffered until revocation completes, then only new-key
events are accepted.

This prevents the race condition where the old custodial key (still
active) could sign fraudulent events during recovery.
```

Custodial-mode events MUST be distinguishable from sovereign-mode events in the envelope via the `key_custody` field (see author event envelope). `key_custody` values: `0 = sovereign`, `1 = threshold_custodial`, `2 = delegated`. When a respondent upgrades from custodial to sovereign, a `custody.upgraded` system event (0x0408) is appended recording the transition. Historical custodial events remain with `key_custody = 1` (immutable envelopes are never rewritten).

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
  8. If the user will not manage a recovery secret, fall back to
     threshold custodial mode (2-of-3 FROST, see above)
```

### Offline-only (no connectivity during form fill)

```
Already handled:

  1. WASM engine evaluates locally (existing capability)
  2. Local ledger records all events to IndexedDB/OPFS
  3. Encrypted blobs stored in local blob store
  4. When connectivity returns: sync protocol sends everything
  5. Server verifies author events and issues canonical receipts
  6. Respondent at a rural clinic on spotty cellular fills the entire
     Medicaid application offline. Every interaction recorded.
     Syncs at the library when they get wifi.
```

---

## 13. Rust Crate Layout

Consolidated from 10 to 8 crates. The serialize-commit-encrypt-wrap-sign-hash pipeline is one atomic operation; splitting it across crate boundaries increased the risk of wrong ordering or missing steps. The `EventBuilder` typestate pattern now enforces the correct pipeline at compile time. Disclosure (BBS+ and SD-JWT) is extracted into its own crate (`ledger-disclosure`) because wasm-pack produces one WASM module per crate, and the BBS+ browser feasibility gate (§3) may require disclosure to be a lazy-loaded secondary WASM module separate from the core `ledger-wasm` artifact.

```
crates/
  ledger-engine/         # THE CORE CRATE. Event types, author envelope v2,
                         # canonical receipt v1, CBOR serialization,
                         # hash computation, event type registry, HLC, causal DAG.
                         #
                         # crypto/ (pub(crate)):
                         #   CryptoPrimitives trait with two backends:
                         #     NativePrimitives: ring (SHA-256, AES-256-GCM)
                         #     WasmPrimitives: sha2, aes-gcm (pure Rust)
                         #   ed25519-dalek signing, curve25519-dalek Pedersen
                         #   (incl. generator derivation via hash-to-group),
                         #   x25519-dalek wrapping, HKDF key derivation
                         #
                         # merkle/ (pub(crate)):
                         #   ct_merkle integration, inclusion/consistency proofs
                         #
                         # checkpoint/:
                         #   COSE signing (coset), signed tree heads
                         #
                         # disclosure/:
                         #   SelectiveDisclosure trait definition,
                         #   DisclosurePolicy type (trait impls are in
                         #   ledger-disclosure, not here)
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
                         # OIDC-to-VC adapter, versioned HKDF info strings,
                         # FROST DKG + threshold signing (custodial mode),
                         # session-scoped signing budget management
                         # Compiles to: native + wasm32

  ledger-disclosure/     # BBS+ proof derivation (bbs_plus),
                         # SD-JWT backend (sd-jwt or custom),
                         # SelectiveDisclosure trait implementations
                         #   (BbsPlusBackend, SdJwtBackend),
                         # disclosure attestation issuance + verification,
                         # DisclosurePolicy enforcement logic
                         #
                         # Separated from ledger-engine because wasm-pack
                         # produces one WASM module per crate. If BBS+
                         # fails the browser feasibility gate (§3),
                         # this crate compiles to a lazy-loaded secondary
                         # WASM module (ledger-disclosure-wasm) separate
                         # from the core ledger-wasm artifact.
                         # Compiles to: native + wasm32

  ledger-anchor/         # OpenTimestamps client (HTTP)
                         # Server-only (not WASM)

  ledger-store/          # BlobStore trait + implementations:
                         #   PostgresBlobStore (server)
                         #   S3BlobStore (server)
                         #   IpfsBlobStore (server + WASM via HTTP)
                         #   OpfsBlobStore (WASM only)
                         #   IndexedDbBlobStore (WASM only)
                         # Author-event store + canonical receipt store
                         # Key bag store + access-grant projections
                         # Key rotation state machine (LAK, disclosure, mapping)
                         # Blob integrity scan (periodic verification)

  ledger-sync/           # Sync protocol: client side + server side
                         # Chain verification, consistency proofs
                         # Ledger key version negotiation
                         # Causal ordering, frontier merge, conflict resolution
                         # Compiles to: native + wasm32

  ledger-projection/     # Materialized view definitions,
                         # projection pipeline,
                         # epoch snapshot management,
                         # projection integrity verification (periodic),
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

All crates share the same cryptographic primitives via `ledger-engine`. One implementation. Two compilation targets. No TS reimplementation. The critical crypto pipeline is entirely within `ledger-engine` with a single public entry point (`EventBuilder`). Disclosure backends (`ledger-disclosure`) depend on `ledger-engine` for the `SelectiveDisclosure` trait and `DisclosurePolicy` type, but implement BBS+/SD-JWT independently so they can be compiled to a separate WASM module if the browser feasibility gate requires lazy-loading.

### Crate dependency fence

Dependencies flow strictly downward. Same-layer dependencies are forbidden.

```
Layer 0: ledger-engine        (core types, crypto, EventBuilder,
                               runtime-loaded event type registry)
                               Targets: native + wasm32

Layer 1: ledger-identity       (DID, VC, WebAuthn PRF, signing key registry)
                               Targets: native + wasm32
                               Depends on: ledger-engine (L0)

         ledger-disclosure     (BBS+, SD-JWT, SelectiveDisclosure impls,
                                disclosure attestation, policy enforcement)
                               Targets: native + wasm32
                               Depends on: ledger-engine (L0)
                               NOTE: same layer as ledger-identity; no
                               lateral dependency between them.

Layer 2: ledger-store          (BlobStore trait + per-target impls,
                                author-event store, receipt store,
                                key bag store, key rotation state machine)
                               Targets: native + wasm32 (trait + browser impls)
                               Depends on: ledger-engine (L0)

         ledger-anchor         (OpenTimestamps client)
                               Targets: native only
                               Depends on: ledger-engine (L0)

Layer 3: ledger-sync           (sync protocol, chain verification,
                                causal ordering, frontier merge)
                               Targets: native + wasm32
                               Depends on: ledger-engine (L0), ledger-store (L2)
                               NOTE: ledger-sync depends on ledger-store (L2),
                               so it MUST be L3, not L2. The sync protocol reads
                               and writes through the store trait boundary.

         ledger-projection     (materialized views, projection pipeline,
                                epoch snapshots, GDPR erasure, tag index)
                               Targets: native only
                               Depends on: ledger-engine (L0),
                                           ledger-store (L2)

Layer 4: ledger-wasm           (WASM entry point, wasm-bindgen,
                                DraftAccumulator)
                               Targets: wasm32 only
                               Depends on: ledger-engine (L0),
                                           ledger-identity (L1),
                                           ledger-store (L2),
                                           ledger-sync (L3)

         ledger-server         (HTTP endpoints, Postgres integration,
                                Temporal activities, KMS, key rotation)
                               Targets: native only
                               Depends on: ledger-engine (L0),
                                           ledger-identity (L1),
                                           ledger-disclosure (L1),
                                           ledger-store (L2),
                                           ledger-sync (L3),
                                           ledger-anchor (L2),
                                           ledger-projection (L3)
```

**Rules:**
- Layer N depends only on layers < N. No lateral (same-layer) dependencies.
- `ledger-engine` is the sole owner of cryptographic primitives. No other crate imports `ed25519-dalek`, `ring`, `curve25519-dalek`, etc. directly.
- Server-only crates (`ledger-anchor`, `ledger-projection`, `ledger-server`) MUST NOT be transitive dependencies of `ledger-wasm`. If `ledger-wasm` compiles, server-only code is excluded.
- `ledger-disclosure` MAY be a dependency of `ledger-wasm` (for client-side BBS+ proof verification) OR compiled as a separate `ledger-disclosure-wasm` module (for lazy-loading if BBS+ exceeds the browser feasibility gate). The choice is made after Spike 1 results.
- CI enforces: `cargo build --target wasm32-unknown-unknown -p ledger-wasm` must succeed without pulling in any server-only crate.

---

## 14. What This Does NOT Include

| Excluded | Why |
|----------|-----|
| FHE eligibility computation | Blocked on GPU infrastructure + case volume. Pedersen commitments are the bridge. |
| PHE equity monitoring | Blocked on actual equity metrics to monitor. Commitments are the bridge. |
| Respondent-facing wallet app | Blocked on identity ecosystem maturity. The browser extension IS the wallet. |
| Cross-ledger causal references | Single-ledger scope. Family/household linking is a projection-layer concern (join on encrypted respondent identity), not a ledger-chain concern. Cross-ledger causal deps would require a consensus protocol this design explicitly avoids. |
| Consortium blockchain | Single sequencer per ledger. No multi-party consensus needed. |
| Mix network / onion routing | Consumer internet identity problem (TPIF), not case management. |
| Verification oracles | Same. TPIF scope, not WOS scope. |
| Custom storage engine | Postgres + ct_merkle + blob stores. No custom DB. |
| Custom signing scheme | COSE (coset) + Ed25519 (ed25519-dalek) + FROST (frost-ed25519, RFC 9591). Standards. |
| Proof of Personhood framework | IAL2 identity proofing subsumes PoP. |

---

## 15. What We Build

The novel work. Everything else is composition.

1. **Unified event taxonomy** -- intake + governance + lifecycle event types, field schemas, privacy classifications, provenance tier mappings. ~50 event types. Per-event-type commitment schemas and disclosure message schemas.

2. **Three-tier access model** -- No Access / Full Decryption / Selective Proof. Audience-scoped views backed by base key bags + immutable access events + disclosure-attestation governance.

3. **Coprocessor protocol** -- `response.completed` -> `case.created`. Sync protocol with causal ordering (HLC + causal deps). Chain handoff from respondent-signed to platform-signed. Subsequent intake events (RFI, appeal) re-entering the chain.

4. **Regulatory compliance semantics** -- retention, legal hold, tombstoned GDPR erasure (6-step protocol with final checkpoint + key destruction + mapping destruction), expungement as ledger operations with mandatory cascades.

5. **Key rotation protocol** -- LAK lazy re-wrap, disclosure key version registry, ledger public key grace periods, active session handling. NIST 800-57 compliant.

6. **Materialized view projection definitions** -- which events project to which views, decryption requirements, encrypted tag index, epoch snapshot strategy, rebuild procedures.

7. **Export artifact format** -- self-verifiable ZIP with author events, receipts, blobs, Merkle tree, checkpoints, OTS proofs, access bundles, disclosure artifacts, and versioned public keys.

8. **Threshold custodial mode** -- 2-of-3 FROST (RFC 9591) threshold custody so no single party holds the full custodial key. Session-scoped signing budgets for offline resilience. Proactive secret sharing for escrow rotation. Sovereignty upgrade via LAK re-wrapping (no custodial key needed). GDPR erasure strengthened: respondent can request any share holder to destroy, bypassing the platform.

9. **Event granularity framework** -- DraftAccumulator batching (DraftSession / PerField / PerSection), conflict-sensitive field policies, auto-save with causal tracking.

10. **The Rust crates** -- 8 crates with typestate EventBuilder enforcing correct event construction ordering, disclosure extracted into its own crate for WASM lazy-loading. One implementation, two compilation targets, no TS reimplementation. Dependency fences enforced by CI.

11. **WOS provenance migration** -- ProvenanceRecord → ledger event mapping. Conformance test adapter. Scope boundary via `provenanceLayer` seam.

12. **Error model** -- sync errors, consistency errors, client recovery procedures.

13. **Signing key registry** -- server-side registry mapping signing_key_id to public keys, lifecycle state, and device attribution.

14. **Recovery re-grant protocol** -- authorization model (two-of-three: WebAuthn + admin + OIDC continuity), `key.recovery_regranted` event type, DEK re-wrap sweep with KMS audit.

15. **Projection integrity verification** -- periodic random-sample verification of materialized views against immutable ledger events, access-grant-specific high-frequency checks, snapshot hash verification.

16. **CryptoPrimitives trait** -- `NativePrimitives` (ring-backed) and `WasmPrimitives` (pure-Rust-backed) with golden test suite asserting byte-identical outputs across both backends.

Everything else -- the cryptographic primitives, the identity standards, the storage systems, the workflow engine -- already exists. We compose them. Once.

---

## 16. Event Type Registry (Draft)

The event type registry maps u16 indices to event type definitions. Each definition includes a commitment schema, privacy classification, and provenance tier mapping. This is a draft taxonomy; the full registry is novel work item #1.

**Registry extensibility and crate independence:** The event type registry is **runtime-loaded configuration**, not compiled into `ledger-engine`. `ledger-engine` defines the `EventTypeSchema` trait and the `EventTypeRegistry` container, but the concrete event type definitions (intake, governance, AI, access, system) are loaded from CBOR or JSON configuration at startup. This ensures `ledger-engine` has zero compile-time knowledge of WOS event semantics — it knows how to process events with u16 type codes, fixed-position commitment schemas, and privacy tiers, but the mapping from type code to semantic name and commitment layout is injected. WOS-specific event types (0x0100-0x02FF) are defined in a separate configuration artifact maintained alongside the WOS specs. Tenant-defined event types (0x1000+) use the same registration mechanism. The `ledger-engine` crate validates registry entries at load time (unique type codes, valid commitment slot counts, consistent schema versions) but does not hardcode any specific event type.

```
Event type ranges (reserved):

  0x0000          Reserved (invalid)
  0x0001 - 0x00FF Intake events (respondent-authored)
  0x0100 - 0x01FF Governance events (platform-authored, WOS lifecycle)
  0x0200 - 0x02FF AI/agent events (platform-authored, AI-specific)
  0x0300 - 0x03FF Access and sharing events
  0x0400 - 0x04FF System/administrative events
  0x0500 - 0x0FFF Reserved for future standard event types
  0x1000 - 0xFFFF Tenant-defined extension event types

Draft event types:

  Intake (0x0001 - 0x00FF):
    0x0001  session.started
    0x0002  draft.saved
    0x0003  setData               (field mutation batch)
    0x0004  attachment.uploaded
    0x0005  response.completed    (submission)
    0x0006  session.resumed       (RFI, appeal re-entry)
    0x0007  draft.consolidated    (offline causal dep overflow merge point)

  Governance (0x0100 - 0x01FF):
    0x0100  case.created          (coprocessor transition)
    0x0101  wos.transition.fired
    0x0102  wos.task.created
    0x0103  wos.task.claimed
    0x0104  wos.task.completed
    0x0105  wos.case.mutated      (case state field change)
    0x0106  wos.timer.created
    0x0107  wos.timer.fired
    0x0108  wos.timer.cancelled
    0x0109  wos.review.submitted
    0x010A  wos.review.protocol   (review protocol enforcement)
    0x010B  wos.determination     (NOT .adverse or .favorable)
    0x010C  wos.explanation.assembled
    0x010D  wos.hold.entered
    0x010E  wos.hold.exited

  AI/Agent (0x0200 - 0x02FF):
    0x0200  wos.agent.invoked
    0x0201  wos.agent.completed
    0x0202  wos.agent.fallback
    0x0203  wos.confidence.reported
    0x0204  wos.drift.detected

  Access (0x0300 - 0x03FF):
    0x0300  access.granted
    0x0301  access.revoked
    0x0302  disclosure.attested
    0x0303  disclosure.proof.issued

  System (0x0400 - 0x04FF):
    0x0400  ledger.checkpoint
    0x0401  ledger.merge
    0x0402  verification.completed
    0x0403  erasure.requested
    0x0404  ledger.erased
    0x0405  key.rotated
    0x0406  key.revoked
    0x0407  key.recovery_regranted
    0x0408  custody.upgraded
    0x0409  custody.key_generated     (threshold DKG completed)
    0x040A  custody.share_destroyed   (one share holder destroyed its share)
    0x040B  custody.escrow_reshared   (escrow replaced via proactive resharing)

Privacy note: event types within the Governance range (0x0100-0x01FF)
are deliberately outcome-neutral. "wos.determination" does not reveal
whether the determination was favorable or adverse. Outcome-specific
information is in the encrypted payload. See §Event type granularity.
```

---

## 17. Error Model

### Sync errors (client → server)

| Error | Condition | Client recovery |
|-------|-----------|-----------------|
| `signature_invalid` | Ed25519 signature verification failed | Re-derive keys from WebAuthn PRF; re-sign and retry |
| `author_event_hash_mismatch` | Recomputed hash doesn't match | Client has a serialization bug; log and alert |
| `ciphertext_id_mismatch` | `payload_ciphertext_id` doesn't match blob hash | Re-encrypt and retry (nonce reuse bug) |
| `ledger_key_revoked` | Key bag references a revoked LAK version | Fetch current ledger public key; re-wrap DEK; retry |
| `ledger_key_unknown` | Key bag references an unknown LAK version | Fetch current ledger public key; re-wrap DEK; retry |
| `clock_skew_exceeded` | `hlc.wall_ms` exceeds server time + MAX_CLOCK_SKEW | Sync device clock; merge HLC with server; retry |
| `dependency_unresolved` | `causal_deps` reference unknown events after buffer window | Sync to obtain missing events; rebuild causal deps; retry |
| `dependency_expired` | Buffered event exceeded max pending duration | Re-create event with updated causal deps against current frontier |
| `conflict_detected` | Concurrent conflict-sensitive field overlap | Client presents conflict to user for `ledger.merge` resolution |
| `verification_failed` | Strict mode: plaintext commitment, tag, or Pedersen check failed | Client has a commitment bug; log, alert, do not retry |

### Consistency errors (client verification)

| Error | Condition | Client action |
|-------|-----------|---------------|
| `receipt_chain_break` | `canonical_prev_receipt_hash` doesn't match prior receipt | Cryptographic evidence of server tampering; alert user; log signed tree head and failed proof as evidence |
| `consistency_proof_invalid` | Merkle consistency proof verification failed | Same as above |
| `sequencer_signature_invalid` | Receipt signature verification failed | Reject receipt batch; re-request from server |
| `event_missing_from_chain` | Client's locally-created event not found in canonical chain | Re-submit event; if repeatedly absent, escalate |

---

## 18. Envelope and Receipt Versioning

### Version fields

The author event envelope carries `version: u8` (currently 2). The canonical receipt carries an implicit version (currently 1, encoded in the receipt format).

### Versioning rules

```
Forward compatibility:
  New fields are APPENDED to the envelope or receipt format.
  Old fields are NEVER removed or reordered.
  A v3 envelope is a strict superset of v2.

Backward compatibility:
  A v3 server MUST accept v2 author events. It processes them
  using v2 rules and issues v3 receipts.
  A v2 client receiving a v3 receipt it doesn't fully understand
  MUST still verify the sequencer signature and receipt chain
  (these fields are positionally stable). It MAY ignore new
  fields it doesn't recognize.

Version negotiation:
  Sync response includes: { min_supported_version, current_version }
  Client includes its envelope version in the sync request.
  Server rejects events below min_supported_version.

Hash domain separation by version:
  Hash prefix strings are versioned ("formspec-ledger-author-event-hash-v2").
  A version bump to v3 means v3 events use "-v3" prefix.
  A verifier must use the prefix matching the event's version field.
  This prevents cross-version hash collisions.

Deprecation:
  Old versions are supported for at least 12 months after the
  successor version ships. Deprecation is announced via sync
  response metadata. Removal requires a major version bump
  to the ledger specification itself.

Mixed-version ledger behavior:

  A single ledger MAY contain events with different envelope
  versions (e.g., v2 and v3 events in the same chain).

  Merkle tree leaves are receipt_hashes. Receipt hashes use
  their own versioned domain separator, independent of the
  author event version. A v1 receipt for a v2 author event
  and a v1 receipt for a v3 author event both use
  "formspec-ledger-receipt-hash-v1" as the domain separator.

  Therefore: the Merkle tree is homogeneous in receipt hash
  format even when author event versions are heterogeneous.
  Version heterogeneity does NOT affect tree construction
  or proof verification.

  Author event verification requires matching the hash domain
  separator to the event's version field. Verifiers MUST
  support all non-deprecated envelope versions.
```

---

## 19. Pre-Implementation Spikes

These spikes MUST be completed before writing `ledger-engine`. They validate assumptions the design depends on. If a spike fails its gate, the design must be revised before proceeding.

### Spike 0: Deterministic CBOR cross-target identity (GATES ALL OTHER SPIKES)

```
The entire integrity model — plaintext commitments, author_event_hash,
receipt_hash, Merkle leaves, Pedersen opening verification, tag
commitment verification — depends on byte-identical deterministic CBOR
(RFC 8949 §4.2) serialization across native and wasm32-unknown-unknown.

Test protocol:
  1. Select a dCBOR crate. Candidates (evaluated in order):
     a. ciborium with post-serialization canonical sort pass
     b. cbor4ii with explicit deterministic mode
     c. minicbor with manual RFC 8949 §4.2 enforcement (no-std/WASM)
     d. coset's internal CBOR (already canonical for COSE structures)
  2. Build a test harness with representative payloads:
     a. Simple: { "field": "value", "count": 42 }
     b. Nested: event payload with actor_id, field_snapshot, change_log,
        commitment_nonce, tag_nonce, tag_bitfield, blinding_factors
     c. Edge cases: empty maps, maps with 24+ keys (triggers 2-byte
        length encoding), negative integers, byte strings, nested arrays,
        maps with keys requiring lexicographic sort that differs from
        insertion order
     d. Key bag: CBOR array of structs with byte string fields
  3. Serialize each payload on native and wasm32-unknown-unknown
  4. Assert byte-identical output for all payloads
  5. Hash each serialized output with SHA-256
  6. Assert hash identity across targets
  7. Deserialize on each target, re-serialize, assert round-trip
     byte identity

Gates:
  - Byte-identical CBOR output on native and WASM for ALL test payloads
  - Round-trip (serialize -> deserialize -> serialize) produces
    identical bytes on both targets
  - Serialization of a 50-field event payload < 1ms on WASM

If the gate fails:
  - Try the next candidate crate
  - If ALL candidates fail: implement a minimal RFC 8949 §4.2
    serializer (map key sort + minimal integer encoding + no
    indefinite-length items) as a ~300-line module in ledger-engine.
    The spec is simple enough that a custom implementation is
    auditable and preferable to non-deterministic output.
  - The hash scheme, commitment scheme, and Merkle tree construction
    CANNOT proceed until this spike passes.

This spike MUST pass before Spike 1 (bundle size) because the crate
choice affects WASM bundle size. Run Spike 0 first, then include the
selected dCBOR crate in Spike 1's bundle measurement.
```

### Spike 1: WASM bundle size validation

```
Build a minimal Rust crate that imports:
  sha2, aes-gcm, ed25519-dalek, x25519-dalek, hkdf,
  curve25519-dalek (ristretto), ct_merkle, <dCBOR crate from Spike 0>,
  hpke (rust-hpke)

Compile to wasm32-unknown-unknown with:
  opt-level = 'z', lto = true, codegen-units = 1

Measure: wasm-opt -Oz output, then brotli-compress.

Gate: <= 700 KB compressed (leaving 300 KB headroom for
event pipeline + HLC + DAG + sync protocol).

If exceeded:
  - Identify which crate is the size contributor
  - Evaluate wasm-opt --strip-dwarf --strip-producers
  - Evaluate whether ct_merkle can be lazy-loaded
    (verification-only use case on client)
  - Consider splitting into core WASM module (crypto + events)
    and optional WASM module (Merkle verification + proofs)
  - Revise the 1 MB total gate if necessary, with justification

Do this BEFORE writing ledger-engine. The crate selection
table may need to change based on results.
```

### Spike 2: ct_merkle lifecycle validation

```
Build a test harness that exercises the full ct_merkle proof
lifecycle on both native and wasm32-unknown-unknown:

  1. Append 10,000 receipt_hash leaves
  2. Build inclusion proof for leaf at index N
  3. Build consistency proof between tree sizes S1 and S2
  4. Verify both proofs on native and WASM
  5. Roundtrip proofs through CBOR serialization/deserialization
  6. Verify serialized proof size (expect < 1 KB for 1M-event tree)
  7. Measure append + proof-generation latency

Gates:
  - All proofs verify identically on native and WASM
  - Serialized proof size < 1 KB for 1M leaves
  - Append latency < 1ms per leaf
  - Proof generation < 5ms

If ct_merkle fails any gate: evaluate merkle_light or a
hand-rolled RFC 6962 implementation (the spec is simple
enough that a ~200-line implementation is viable and auditable).
```

### Spike 3: Pedersen generator cross-target identity

```
Implement the generator derivation from §2:
  G_i = ristretto255_hash_to_group(
    "formspec-ledger-pedersen-generator-v1" || u16_le(t) || u16_le(i)
  )

Generate generators for 10 event types × 16 slots = 160 generators.

Gate: byte-identical compressed Ristretto points on native
and wasm32-unknown-unknown for all 160 generators.

This validates that the hash-to-group implementation in
curve25519-dalek produces identical results across targets.
If it does not, the Pedersen commitment scheme is broken
across targets and an alternative derivation is needed.
```

### Spike 4: WebAuthn PRF cross-platform determinism

```
The entire identity model (§1) depends on the WebAuthn PRF
extension producing deterministic 32-byte secrets from the
same credential + salt pair across devices and browsers.

Test protocol:
  1. Register a WebAuthn credential on Chrome desktop (macOS)
  2. Derive keys via PRF with a fixed test salt
  3. Record the derived Ed25519 public key and X25519 public key
  4. Sync the credential via iCloud Keychain to Safari on iOS
  5. Derive keys via PRF with the same test salt on Safari iOS
  6. Compare: byte-identical public keys?
  7. Repeat for: Chrome Android (Google Password Manager sync),
     Chrome Windows (Windows Hello + cross-device sync)

Gates:
  - Same credential + same PRF salt = same derived keys across
    all tested platform/browser combinations
  - Key derivation latency < 200ms on mid-tier phone

If the gate fails on ANY platform combination:
  - The "same passkey = same DID" assumption (§1 Step 4) breaks
  - Options: (a) treat each device as a separate DID and link
    them via platform identity (OIDC), similar to recovery flow;
    (b) use the PRF fallback (§12 "No WebAuthn PRF extension")
    as the default for cross-device scenarios; (c) store an
    encrypted key blob server-side, unlocked by any valid
    WebAuthn assertion (not PRF-derived)
  - The fallback path (§12) already handles the PRF-absent case;
    this spike determines whether PRF-absent is the exception
    or the norm for cross-platform use

This spike is critical because the fallback (§12) exists but
shifts the security model (user-managed recovery secret or
custodial keys). If PRF cross-platform determinism holds,
the identity model is strictly stronger.
```

### Spike 5: FROST threshold signing on native + WASM

```
The threshold custodial mode (§12) depends on FROST (RFC 9591)
producing valid Ed25519 signatures from a 2-of-3 DKG across
native and WASM targets.

Test protocol:
  1. Run FROST DKG for 3 participants on native, producing
     share_1, share_2, share_3, and group_public_key
  2. Run FROST signing round with participants 1+2 (native)
     Sign a test message. Verify signature against group_public_key.
  3. Run FROST signing round with participants 1+3 (native)
     Sign the same message. Verify. Different signature bytes
     (randomized nonces), same valid verification.
  4. Repeat steps 2-3 with one participant running on
     wasm32-unknown-unknown (simulating browser-side participation
     in the signing protocol, if ever needed for advanced modes)
  5. Verify: group_public_key from DKG is a standard Ed25519 public
     key. Threshold signatures verify with standard ed25519-dalek
     verify() — no special verifier needed.
  6. Measure: DKG latency, signing round latency (round-1 + round-2),
     WASM bundle size contribution of frost-ed25519

Gates:
  - All threshold signatures verify with standard Ed25519 verify()
  - DKG completes in < 500ms (3 participants, single machine)
  - Signing round completes in < 100ms (2 participants, single machine)
  - frost-ed25519 adds < 100 KB to compressed WASM bundle
  - Session-scoped signing budget: pre-generate 20 round-1 nonce
    commitments, verify all 20 can be used to complete round-2
    independently (no additional escrow round-trip)

If frost-ed25519 fails any gate:
  - Evaluate frost-core with custom Ed25519 backend
  - If WASM bundle size is the blocker: threshold signing runs
    server-side only (both share holders are server-side services),
    so WASM compilation of frost-ed25519 is a nice-to-have, not
    a hard requirement. The custodial mode operates entirely
    server-side by design.
  - If signing latency exceeds the gate: increase the session-scoped
    budget size to amortize the DKG/setup cost over more signatures
```
