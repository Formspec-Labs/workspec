

# Concrete Solutions for All 13 Findings

---

## FINDING 1: Multi-device causal ordering

**Problem:** Receipt-order sequencing loses causal relationships. Device A's FEL calculation affecting field B may be canonically ordered after Device B's direct edit of field B.

**Specific change:** Replace bare Lamport timestamps with a hybrid logical clock (HLC) plus explicit causal dependency references in the event header. Each event carries a vector of `(device_id, hlc)` pairs for events it causally depends on. The server uses these to topologically sort within a case before assigning canonical sequence numbers.

```rust
/// Hybrid Logical Clock — Kulkarni et al. 2014
/// Combines wall-clock with logical counter to give
/// causal ordering without full vector clocks.
#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
struct HLC {
    /// Wall-clock milliseconds (coarsened to prevent fingerprinting)
    wall_ms: u64,
    /// Logical counter, incremented when wall clock hasn't advanced
    logical: u32,
    /// Originating device identifier (truncated hash of device pubkey)
    device_id: [u8; 8],
}

impl HLC {
    fn new(device_id: [u8; 8]) -> Self {
        Self { wall_ms: now_ms(), logical: 0, device_id }
    }

    /// Called before creating a new event. Merges local clock
    /// with the latest HLC seen from any source (sync, local).
    fn tick(&mut self, other: Option<HLC>) -> HLC {
        let now = now_ms();
        let max_wall = match other {
            Some(o) => now.max(self.wall_ms).max(o.wall_ms),
            None => now.max(self.wall_ms),
        };
        self.logical = if max_wall == self.wall_ms && Some(max_wall) == other.map(|o| o.wall_ms) {
            self.logical.max(other.unwrap().logical) + 1
        } else if max_wall == self.wall_ms {
            self.logical + 1
        } else if other.map_or(false, |o| max_wall == o.wall_ms) {
            other.unwrap().logical + 1
        } else {
            0
        };
        self.wall_ms = max_wall;
        *self
    }
}

/// Causal dependency reference — which prior events this event
/// observed before being created. Compact: just the event hash
/// and the HLC of the dependency.
struct CausalDep {
    /// SHA-256 of the depended-upon event (first 16 bytes for compactness)
    event_hash_prefix: [u8; 16],
    /// HLC of the depended-upon event
    hlc: HLC,
}

/// Updated event header — replaces the bare u64 timestamp field
struct EventHeaderV2 {
    version: u8,                          // bumped to 2
    sequence: u64,                        // assigned by server
    hlc: HLC,                             // replaces timestamp u64
    prev_hash: [u8; 32],
    payload_content_id: [u8; 32],
    actor_type: u8,
    event_type: u16,
    privacy_tier: u8,
    signing_key_id: [u8; 16],
    signature: [u8; 64],
    governance_result: u8,
    tag_commitment: [u8; 32],             // changed per Finding 6
    commitment_count: u8,
    commitments: Vec<PedersenCommitment>,
    // NEW: causal dependencies
    causal_dep_count: u8,                 // max 8 deps per event
    causal_deps: Vec<CausalDep>,          // typically 1-3 entries
}

/// Server-side canonical ordering within a case.
/// Runs on each sync batch before assigning sequence numbers.
fn assign_canonical_order(
    existing_chain: &[EventHeaderV2],
    incoming_batch: &mut [EventHeaderV2],
) -> Result<(), ConflictSet> {
    // Build DAG from causal_deps
    let mut dag = CausalDAG::new();
    for event in existing_chain.iter().chain(incoming_batch.iter()) {
        dag.add_node(event.content_hash(), event.hlc);
        for dep in &event.causal_deps {
            dag.add_edge(dep.event_hash_prefix, event.content_hash());
        }
    }

    // Topological sort; ties broken by HLC (wall_ms, then logical)
    let ordered = dag.topological_sort_by_hlc()?;

    // Detect true conflicts: two events with no causal relationship
    // that modify overlapping field sets
    let conflicts = dag.find_concurrent_field_conflicts(&ordered);
    if !conflicts.is_empty() {
        // Return conflicts for application-layer resolution
        // (last-writer-wins by HLC, or manual merge)
        return Err(ConflictSet(conflicts));
    }

    // Assign canonical sequence numbers
    let next_seq = existing_chain.last().map_or(0, |e| e.sequence + 1);
    for (i, hash) in ordered.iter().enumerate() {
        if let Some(event) = incoming_batch.iter_mut().find(|e| e.content_hash() == *hash) {
            event.sequence = next_seq + i as u64;
        }
    }
    Ok(())
}
```

**Conflict resolution at the application layer:**

```rust
/// When two concurrent events touch the same field, the server
/// must pick a winner or request manual resolution.
enum ConflictResolution {
    /// HLC-ordered last-writer-wins (automatic, default for non-critical fields)
    LastWriterWins,
    /// Both events preserved, a synthetic merge event is appended
    /// that records the resolution. Used for fields marked conflict-sensitive.
    MergeEvent { resolved_value: Value, resolver_actor: ActorId },
}

/// Field-level conflict sensitivity is declared in the form definition.
/// FEL-calculated fields are always conflict-sensitive because their
/// causal chain matters.
struct FieldConflictPolicy {
    field_path: String,
    policy: ConflictResolution,
}
```

**Tradeoffs:**
- **Bytes:** +28 bytes per causal dep (16-byte hash prefix + 12-byte HLC). Typical event adds 1-3 deps = 28-84 bytes. On a 500-byte average event, this is 5-17% overhead.
- **Latency:** Server-side topological sort is O(V + E) where V = batch size, E = dep edges. For typical batches of 10-50 events, sub-millisecond.
- **Complexity:** HLC is well-understood (CockroachDB, TiDB use variants). The causal DAG is the main new complexity — roughly 200 lines of code.
- **Compatibility:** Header version bump to 2. Old clients can't produce v2 headers, but server can accept v1 headers and treat them as having zero causal deps (receipt-order fallback).

---

## FINDING 2: Key rotation (complete protocol)

**Problem:** No protocol for PRK rotation (thousands of re-wraps), BBS+ key versioning, TMK rotation, tenant pubkey rotation, or active session handling.

### 2a: PRK Rotation

**Specific change:** Lazy re-wrapping. New PRK is generated; old PRK is marked `rotating`, not destroyed. On read, if a DEK is wrapped under old PRK, transparently re-wrap under new PRK. Background worker sweeps remaining old-PRK wrappings. Old PRK destroyed only after sweep completes.

```rust
/// PRK lifecycle states
#[derive(Debug, Clone, PartialEq)]
enum PRKState {
    Active,
    /// Old PRK still usable for decrypt, new PRK used for all new wraps
    Rotating { new_prk_id: KmsKeyId, initiated_at: u64, sweep_progress: SweepProgress },
    /// All DEKs re-wrapped, old PRK pending destruction
    PendingDestruction { superseded_by: KmsKeyId },
    Destroyed { destroyed_at: u64 },
}

struct SweepProgress {
    total_events: u64,
    rewrapped_events: u64,
    last_processed_sequence: u64,
}

/// PRK rotation protocol
struct PRKRotation;

impl PRKRotation {
    /// Step 1: Generate new PRK in KMS, mark old as Rotating
    async fn initiate(
        kms: &dyn KmsClient,
        respondent_id: &RespondentId,
        old_prk_id: &KmsKeyId,
    ) -> Result<PRKRotationHandle> {
        let new_prk_id = kms.generate_key(KeySpec::Aes256).await?;

        // Atomic state transition in metadata store
        let handle = PRKRotationHandle {
            respondent_id: respondent_id.clone(),
            old_prk_id: old_prk_id.clone(),
            new_prk_id: new_prk_id.clone(),
            state: PRKState::Rotating {
                new_prk_id: new_prk_id.clone(),
                initiated_at: now_ms(),
                sweep_progress: SweepProgress {
                    total_events: 0, rewrapped_events: 0, last_processed_sequence: 0,
                },
            },
        };

        // Store rotation record — this is the commit point
        store_rotation_record(&handle).await?;
        Ok(handle)
    }

    /// Step 2: Background sweep — processes events in batches
    /// Rate-limited to avoid KMS throttling (AWS KMS: 5500 req/s shared)
    async fn sweep_batch(
        kms: &dyn KmsClient,
        store: &dyn EventStore,
        handle: &mut PRKRotationHandle,
        batch_size: usize,       // typically 100
        rate_limit: &RateLimiter, // e.g., 500 req/s per rotation
    ) -> Result<bool> {
        let events = store.get_events_with_prk(
            &handle.respondent_id,
            &handle.old_prk_id,
            handle.state.sweep_progress().last_processed_sequence,
            batch_size,
        ).await?;

        if events.is_empty() {
            return Ok(true); // sweep complete
        }

        for event in &events {
            rate_limit.acquire().await;

            // Decrypt DEK with old PRK
            let dek = kms.decrypt(
                &handle.old_prk_id,
                &event.wrapped_dek_prk,
            ).await?;

            // Re-encrypt DEK with new PRK
            let new_wrapped_dek = kms.encrypt(
                &handle.new_prk_id,
                &dek,
            ).await?;

            // Update key bag entry — atomic swap
            store.update_key_bag_prk_entry(
                event.event_id(),
                &handle.new_prk_id,
                &new_wrapped_dek,
            ).await?;

            // Clear DEK from memory
            dek.zeroize();
        }

        // Update sweep progress
        handle.update_progress(events.last().unwrap().sequence).await?;
        Ok(false)
    }

    /// Step 3: Finalize — schedule old PRK for destruction
    async fn finalize(
        kms: &dyn KmsClient,
        handle: &mut PRKRotationHandle,
    ) -> Result<()> {
        // Verify sweep is truly complete
        assert!(handle.verify_complete().await?);

        // KMS scheduled deletion (AWS: 7-30 day waiting period)
        kms.schedule_key_deletion(
            &handle.old_prk_id,
            Duration::days(30), // configurable, minimum 7 for AWS
        ).await?;

        handle.transition_to(PRKState::PendingDestruction {
            superseded_by: handle.new_prk_id.clone(),
        }).await?;

        Ok(())
    }
}

/// On-read transparent re-wrapping for events not yet swept
async fn read_event_with_lazy_rewrap(
    kms: &dyn KmsClient,
    store: &dyn EventStore,
    event: &StoredEvent,
    rotation: Option<&PRKRotationHandle>,
) -> Result<DecryptedPayload> {
    let (prk_id, wrapped_dek) = if let Some(rot) = rotation {
        if event.prk_id == rot.old_prk_id {
            // Opportunistic re-wrap on read
            let dek = kms.decrypt(&rot.old_prk_id, &event.wrapped_dek_prk).await?;
            let new_wrap = kms.encrypt(&rot.new_prk_id, &dek).await?;
            store.update_key_bag_prk_entry(
                event.event_id(), &rot.new_prk_id, &new_wrap,
            ).await?;
            // Use decrypted DEK directly (already have it)
            return decrypt_payload(&dek, &event.ciphertext, &event.nonce);
        }
        (&rot.new_prk_id, &event.wrapped_dek_prk)
    } else {
        (&event.prk_id, &event.wrapped_dek_prk)
    };

    let dek = kms.decrypt(prk_id, wrapped_dek).await?;
    decrypt_payload(&dek, &event.ciphertext, &event.nonce)
}
```

**KMS call budget analysis:**
- 1,000 events per respondent (typical multi-year case)
- Sweep: 1,000 decrypt + 1,000 encrypt = 2,000 KMS calls
- At 500 req/s rate limit: 4 seconds per respondent
- Batch of 100 respondents rotating simultaneously: ~400 seconds
- AWS KMS cost: $0.06 per 2,000 calls = negligible

### 2b: BBS+ Key Rotation

```rust
/// BBS+ key is tied to the issuer (system-wide or per-tenant).
/// Key rotation means new events use new key; old signatures
/// remain verifiable against old key.

/// Key version registry — append-only, stored in a system-level
/// events table (not per-respondent).
struct BBSKeyVersion {
    version: u32,
    public_key: BBSPublicKey,
    /// KMS-wrapped private key material
    wrapped_private_key: Vec<u8>,
    kms_key_id: KmsKeyId,
    valid_from: u64,       // timestamp
    valid_until: Option<u64>, // None = current
    revoked: bool,
}

/// Event header carries the BBS+ key version used for signing
struct EventHeaderV2 {
    // ... existing fields ...
    bbs_key_version: u32,  // NEW: which BBS+ key version signed this event
    bbs_signature: Vec<u8>, // BBS+ signature (variable length)
}

/// Verification looks up the correct key version
fn verify_bbs_signature(
    event: &EventHeaderV2,
    plaintext_fields: &[Vec<u8>],
    key_registry: &BBSKeyRegistry,
) -> Result<bool> {
    let key_version = key_registry.get(event.bbs_key_version)
        .ok_or(Error::UnknownBBSKeyVersion(event.bbs_key_version))?;

    if key_version.revoked {
        return Err(Error::RevokedBBSKey(event.bbs_key_version));
    }

    // Verify the BBS+ signature was created during the key's valid period
    if event.hlc.wall_ms < key_version.valid_from {
        return Err(Error::BBSKeyNotYetValid);
    }
    if let Some(until) = key_version.valid_until {
        if event.hlc.wall_ms > until {
            return Err(Error::BBSKeyExpiredAtEventTime);
        }
    }

    bbs_verify(&key_version.public_key, plaintext_fields, &event.bbs_signature)
}

/// Selective disclosure proof includes the key version so verifiers
/// can look up the correct public key
struct SelectiveDisclosureProof {
    bbs_key_version: u32,
    disclosed_indices: Vec<usize>,
    proof: BBSProof,
}
```

### 2c: TMK Rotation

```rust
/// TMK wraps PRKs. TMK rotation = re-wrap all PRKs.
/// Similar lazy pattern to PRK rotation but one level up.

struct TMKRotation;

impl TMKRotation {
    /// TMK rotation is simpler: only PRK metadata is re-wrapped,
    /// not individual DEKs. Number of operations = number of respondents.
    async fn rotate(
        kms: &dyn KmsClient,
        prk_registry: &dyn PRKRegistry,
        old_tmk_id: &KmsKeyId,
        new_tmk_id: &KmsKeyId,
    ) -> Result<()> {
        // TMK doesn't directly wrap PRKs in the current design —
        // PRKs are independent KMS keys. TMK wraps the PRK *metadata*
        // (which PRK belongs to which respondent).
        //
        // If TMK is used as a key-encryption-key over PRK material:
        let prk_ids = prk_registry.list_all_prks().await?;

        for prk_id in prk_ids {
            // PRKs are KMS-native keys, not wrapped by TMK.
            // TMK's role is administrative: it gates who can
            // create/destroy PRKs, not wrap them.
            //
            // TMK rotation = rotate the KMS key policy that
            // authorizes PRK operations.
            kms.update_key_policy(
                &prk_id,
                KeyPolicy::require_tmk_authorization(new_tmk_id),
            ).await?;
        }

        kms.schedule_key_deletion(old_tmk_id, Duration::days(90)).await?;
        Ok(())
    }
}
```

### 2d: Active Session Handling During Key Rotation

```rust
/// Sessions in flight during rotation must not lose work.
/// Protocol: session tokens carry the PRK version they were issued against.
/// Server accepts events wrapped under any non-revoked PRK version.

struct SessionKeyContext {
    session_id: SessionId,
    respondent_id: RespondentId,
    /// PRK version this session was established with
    prk_version: u32,
    /// Respondent's X25519 pubkey at session start
    recipient_pubkey: X25519PublicKey,
    /// Tenant pubkey at session start
    tenant_pubkey_version: u32,
}

/// Sync response includes rotation advisory
struct SyncResponse {
    accepted_events: Vec<EventId>,
    assigned_sequences: Vec<(EventId, u64)>,
    /// If set, client should re-wrap future DEKs for this PRK version
    prk_rotation_advisory: Option<PRKRotationAdvisory>,
    /// Current tenant pubkey version
    current_tenant_pubkey_version: u32,
}

struct PRKRotationAdvisory {
    /// New PRK version to use for future events
    new_prk_version: u32,
    /// Old version still accepted until this time
    old_version_accepted_until: u64,
    /// Grace period: typically 30 days
    grace_period_days: u32,
}
```

**Tradeoffs:**
- **PRK rotation:** 2 KMS calls per event × number of events. At 1,000 events, ~$0.06 and 4 seconds. Acceptable.
- **BBS+ key versioning:** 4 bytes per event header for version field. Key registry is small (tens of entries over system lifetime).
- **TMK rotation:** Minutes for thousands of respondents, but infrequent (annual or on compromise).
- **Complexity:** Rotation state machine adds ~500 lines. But it's mandatory for FedRAMP.
- **NIST 800-57 compliance:** This covers crypto-period management (Section 5.3), key compromise recovery (Section 8.3), and key destruction scheduling (Section 8.3.4).

---

## FINDING 3: Crate layout consolidation

**Problem:** Sign-then-encrypt-then-hash split across 4+ crates increases misuse risk.

**Specific change:** Merge `ledger-crypto`, `ledger-core`, parts of `ledger-disclosure`, and `ledger-merkle` into a single `ledger-engine` crate. The `EventBuilder` is the only public entry point for event construction. Individual crypto primitives are `pub(crate)` only.

```rust
// crates/ledger-engine/src/lib.rs
//
// Consolidated crate structure:
//   ledger-engine/
//     src/
//       lib.rs              — public API: EventBuilder, EventVerifier
//       builder.rs          — EventBuilder (typestate pattern)
//       verifier.rs         — EventVerifier
//       crypto/
//         mod.rs            — pub(crate) re-exports
//         aead.rs           — AES-256-GCM encrypt/decrypt
//         signing.rs        — Ed25519 sign/verify
//         bbs.rs            — BBS+ sign/verify/disclose
//         ecies.rs          — X25519 ECIES key wrapping
//         pedersen.rs       — Pedersen commitments
//         kdf.rs            — HKDF for key derivation
//       merkle/
//         mod.rs            — pub(crate) Merkle tree ops
//         rfc6962.rs        — ct_merkle integration
//       hash.rs             — event hashing (single canonical construction)
//       types.rs            — EventHeader, EncryptedPayload, KeyBag, etc.

/// Typestate builder — compile-time enforcement of correct ordering.
/// Each state transition is a method that consumes self and returns
/// the next state. You literally cannot call .sign() before .encrypt().

// Typestate markers (zero-sized types)
struct NeedsPayload;
struct NeedsEncryption;
struct NeedsSigning;
struct NeedsCommitments;
struct NeedsBBS;
struct NeedsHash;
struct Complete;

struct EventBuilder<State> {
    header: PartialHeader,
    payload: Option<Vec<u8>>,
    ciphertext: Option<Vec<u8>>,
    key_bag: Option<KeyBag>,
    signature: Option<[u8; 64]>,
    commitments: Option<Vec<PedersenCommitment>>,
    bbs_signature: Option<Vec<u8>>,
    event_hash: Option<[u8; 32]>,
    _state: PhantomData<State>,
}

impl EventBuilder<NeedsPayload> {
    /// Start building an event. Only entry point.
    pub fn new(
        event_type: u16,
        actor_type: u8,
        privacy_tier: u8,
        prev_hash: [u8; 32],
        hlc: HLC,
        causal_deps: Vec<CausalDep>,
    ) -> Self {
        EventBuilder {
            header: PartialHeader::new(event_type, actor_type, privacy_tier, prev_hash, hlc, causal_deps),
            payload: None,
            ciphertext: None,
            key_bag: None,
            signature: None,
            commitments: None,
            bbs_signature: None,
            event_hash: None,
            _state: PhantomData,
        }
    }

    /// Set the CBOR payload. Deterministic encoding enforced here.
    pub fn with_payload(mut self, payload: &impl Serialize) -> Result<EventBuilder<NeedsCommitments>> {
        let cbor = deterministic_cbor_encode(payload)?;
        self.payload = Some(cbor);
        Ok(EventBuilder {
            header: self.header,
            payload: self.payload,
            ciphertext: None,
            key_bag: None,
            signature: None,
            commitments: None,
            bbs_signature: None,
            event_hash: None,
            _state: PhantomData,
        })
    }
}

impl EventBuilder<NeedsCommitments> {
    /// Generate Pedersen commitments for numeric fields.
    /// Fixed-position vector per event type (Finding 5 fix).
    pub fn with_commitments(
        mut self,
        numeric_fields: &CommitmentFieldMap,
        commitment_schema: &EventTypeCommitmentSchema,
    ) -> Result<EventBuilder<NeedsEncryption>> {
        let commitments = generate_fixed_position_commitments(
            numeric_fields,
            commitment_schema,
        )?;
        self.commitments = Some(commitments);
        Ok(EventBuilder {
            header: self.header,
            payload: self.payload,
            ciphertext: None,
            key_bag: None,
            signature: None,
            commitments: self.commitments,
            bbs_signature: None,
            event_hash: None,
            _state: PhantomData,
        })
    }
}

impl EventBuilder<NeedsEncryption> {
    /// Encrypt payload, generate key bag for all recipients.
    /// DEK is random, single-use. ECIES with fresh ephemeral per recipient.
    pub fn encrypt(
        mut self,
        recipients: &[Recipient],
        prk_wrap: &dyn PRKWrapper,
    ) -> Result<EventBuilder<NeedsSigning>> {
        let dek = generate_dek();
        let (ciphertext, nonce) = aes_256_gcm_encrypt(&dek, self.payload.as_ref().unwrap())?;
        let key_bag = build_key_bag(&dek, recipients, prk_wrap)?;
        dek.zeroize();

        self.ciphertext = Some(ciphertext);
        self.key_bag = Some(key_bag);
        self.header.set_payload_content_id(&self.payload.as_ref().unwrap());
        self.header.set_nonce(nonce);

        Ok(EventBuilder {
            header: self.header,
            payload: self.payload,
            ciphertext: self.ciphertext,
            key_bag: self.key_bag,
            signature: None,
            commitments: self.commitments,
            bbs_signature: None,
            event_hash: None,
            _state: PhantomData,
        })
    }
}

impl EventBuilder<NeedsSigning> {
    /// Ed25519 sign the header (which includes payload_content_id,
    /// covering the encrypted payload by reference).
    pub fn sign(
        mut self,
        signing_key: &Ed25519SigningKey,
    ) -> Result<EventBuilder<NeedsBBS>> {
        let header_bytes = self.header.canonical_bytes();
        let sig = signing_key.sign(&header_bytes);
        self.signature = Some(sig);
        self.header.set_signature(sig);
        self.header.set_signing_key_id(signing_key.key_id());

        Ok(EventBuilder {
            header: self.header,
            payload: self.payload,
            ciphertext: self.ciphertext,
            key_bag: self.key_bag,
            signature: self.signature,
            commitments: self.commitments,
            bbs_signature: None,
            event_hash: None,
            _state: PhantomData,
        })
    }
}

impl EventBuilder<NeedsBBS> {
    /// BBS+ signature over the plaintext field vector.
    /// This is the LAST crypto operation before hashing.
    pub fn bbs_sign(
        mut self,
        bbs_key: &BBSSigningKey,
        plaintext_field_vector: &[Vec<u8>],
    ) -> Result<EventBuilder<NeedsHash>> {
        let bbs_sig = bbs_key.sign(plaintext_field_vector)?;
        self.bbs_signature = Some(bbs_sig);
        self.header.set_bbs_key_version(bbs_key.version());

        Ok(EventBuilder {
            header: self.header,
            payload: self.payload,
            ciphertext: self.ciphertext,
            key_bag: self.key_bag,
            signature: self.signature,
            commitments: self.commitments,
            bbs_signature: self.bbs_signature,
            event_hash: None,
            _state: PhantomData,
        })
    }
}

impl EventBuilder<NeedsHash> {
    /// Canonical hash over the complete event (Finding MISSED-4 fix).
    /// This is the ONLY hash construction in the system.
    pub fn finalize(mut self) -> Result<SealedEvent> {
        let event_hash = canonical_event_hash(
            &self.header.canonical_bytes(),
            self.ciphertext.as_ref().unwrap(),
            self.key_bag.as_ref().unwrap(),
            self.bbs_signature.as_ref().unwrap(),
        );

        Ok(SealedEvent {
            header: self.header.finalize(event_hash),
            ciphertext: self.ciphertext.unwrap(),
            key_bag: self.key_bag.unwrap(),
            bbs_signature: self.bbs_signature.unwrap(),
            event_hash,
            // payload is NOT stored — only ciphertext is
            _plaintext_dropped: (),
        })
    }
}

/// SealedEvent is the only type that can be persisted or transmitted.
/// It cannot be modified after construction.
pub struct SealedEvent {
    header: FinalizedHeader,
    ciphertext: Vec<u8>,
    key_bag: KeyBag,
    bbs_signature: Vec<u8>,
    event_hash: [u8; 32],
    _plaintext_dropped: (),
}
```

**Revised crate layout (10 → 6):**

| Old crate | New location |
|-----------|-------------|
| `ledger-core` | `ledger-engine` |
| `ledger-crypto` | `ledger-engine::crypto` (pub(crate)) |
| `ledger-disclosure` | `ledger-engine::disclosure` (BBS+ proofs) |
| `ledger-merkle` | `ledger-engine::merkle` (pub(crate)) |
| `ledger-checkpoint` | `ledger-engine::checkpoint` |
| `ledger-anchor` | `ledger-anchor` (kept separate — external service integration) |
| `ledger-identity` | `ledger-identity` (kept separate — OIDC/WebAuthn/DID) |
| `ledger-store` | `ledger-store` (kept separate — Postgres, blob stores) |
| `ledger-sync` | `ledger-sync` (kept separate — network protocol) |
| `ledger-projection` | `ledger-projection` (kept separate — read models) |
| `ledger-wasm` | `ledger-wasm` (kept separate — thin WASM bindings) |
| `ledger-server` | `ledger-server` (kept separate — HTTP/gRPC) |

**Result:** 10 crates → 7 crates. The critical crypto pipeline is entirely within `ledger-engine`, with a single public entry point.

**Tradeoffs:**
- **Compile time:** Slightly longer for `ledger-engine` (more code in one crate). Mitigated by cargo's per-file incremental compilation.
- **Complexity:** Typestate pattern adds type-level complexity but eliminates an entire class of runtime errors (wrong ordering, missing steps). Net reduction in total system complexity.
- **Testing:** Crypto internals testable via `#[cfg(test)]` within the crate. No need for public API exposure just for testing.

---

## FINDING 4: Tenant public key rotation grace period

**Problem:** Hard rejection of events wrapped under old tenant pubkey creates availability problems for offline/intermittent users.

**Specific change:** Server accepts events wrapped under any non-revoked tenant pubkey version. Sync response includes current version. Client transitions at its own pace.

```rust
/// Tenant key registry — append-only versioned key list
struct TenantKeyRegistry {
    tenant_id: TenantId,
    keys: Vec<TenantKeyEntry>,
}

struct TenantKeyEntry {
    version: u32,
    public_key: X25519PublicKey,
    /// KMS-managed private key reference
    kms_key_id: KmsKeyId,
    status: TenantKeyStatus,
    created_at: u64,
    /// When this key stops being accepted for NEW events
    /// (still usable for decryption of old events indefinitely)
    accept_new_events_until: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
enum TenantKeyStatus {
    /// Current preferred key — returned in sync responses
    Active,
    /// Still accepted for incoming events, but clients should transition
    Deprecated { successor_version: u32, grace_deadline: u64 },
    /// No longer accepted for new events, still usable for decryption
    RetiredForDecryptionOnly,
    /// Revoked due to compromise — reject events, disable decryption
    Revoked { reason: String, revoked_at: u64 },
}

/// Server-side sync event acceptance
fn accept_sync_event(
    event: &IncomingEvent,
    tenant_keys: &TenantKeyRegistry,
) -> SyncAcceptResult {
    let wrapped_for_version = event.key_bag.tenant_key_version();

    match tenant_keys.get(wrapped_for_version) {
        None => SyncAcceptResult::Reject {
            reason: "unknown tenant key version",
            current_version: tenant_keys.active_version(),
        },
        Some(entry) if entry.status == TenantKeyStatus::Revoked { .. } => {
            SyncAcceptResult::Reject {
                reason: "tenant key revoked — re-wrap required",
                current_version: tenant_keys.active_version(),
            }
        }
        Some(entry) if matches!(entry.status, TenantKeyStatus::RetiredForDecryptionOnly) => {
            // Past grace period but not revoked — accept with strong warning
            SyncAcceptResult::AcceptWithWarning {
                warning: "tenant key version retired; re-wrap future events",
                current_version: tenant_keys.active_version(),
                // Server-side: re-wrap the DEK for current tenant key
                server_rewrap: true,
            }
        }
        Some(entry) => {
            // Active or Deprecated — fully accepted
            let advisory = if entry.version != tenant_keys.active_version() {
                Some(TenantKeyAdvisory {
                    current_version: tenant_keys.active_version(),
                    current_pubkey: tenant_keys.active_pubkey(),
                    your_version_deprecated_at: match &entry.status {
                        TenantKeyStatus::Deprecated { grace_deadline, .. } => Some(*grace_deadline),
                        _ => None,
                    },
                })
            } else {
                None
            };
            SyncAcceptResult::Accept { advisory }
        }
    }
}

/// Sync response — always includes current tenant key info
struct SyncResponse {
    // ... existing fields ...
    /// Current active tenant public key — client should use for future events
    tenant_key_info: TenantKeyInfo,
}

struct TenantKeyInfo {
    active_version: u32,
    active_pubkey: X25519PublicKey,
    /// If client was using a deprecated version, deadline to transition
    deprecation_deadline: Option<u64>,
}

/// Server-side transparent re-wrapping for deprecated/retired keys.
/// When a sync event arrives wrapped for a non-active tenant key,
/// the server decrypts the DEK with the old tenant key and adds
/// a new key_bag entry for the current tenant key.
async fn server_rewrap_for_current_tenant_key(
    kms: &dyn KmsClient,
    event: &mut StoredEvent,
    old_key: &TenantKeyEntry,
    current_key: &TenantKeyEntry,
) -> Result<()> {
    // Decrypt DEK using old tenant private key (in KMS)
    let dek = kms.decrypt(&old_key.kms_key_id, &event.key_bag.tenant_wrapped_dek())?;

    // Re-wrap for current tenant key
    let new_wrapped = ecies_wrap(&current_key.public_key, &dek)?;
    dek.zeroize();

    // Add new entry to key bag (don't remove old — it's append-only)
    event.key_bag.add_tenant_entry(current_key.version, new_wrapped);

    Ok(())
}
```

**Tradeoffs:**
- **Bytes:** Key bag grows by ~80 bytes per additional tenant key wrapping. Negligible.
- **Latency:** Server-side re-wrap adds one KMS call on deprecated-key events. Only during transition period.
- **Security:** Deprecated keys remain in KMS for decryption but are not used for new wrapping. Revoked keys (compromise) are hard-rejected.
- **Availability:** Offline clients that sync after weeks/months still work as long as their key version isn't revoked. This is the critical win.

---

## FINDING 5: Pedersen commitment fixed-position vector

**Problem:** Variable-position commitments leak which numeric fields are populated.

**Specific change:** Define a commitment schema per event type. Every event of that type produces a fixed-length commitment vector with commitments-to-zero for unused fields.

```rust
/// Per-event-type schema defining which positions map to which fields.
/// Published as part of the system's public verification parameters.
struct EventTypeCommitmentSchema {
    event_type: u16,
    /// Ordered list of field paths that get commitments.
    /// Position in this list = position in the commitment vector.
    field_positions: Vec<CommitmentFieldDef>,
}

struct CommitmentFieldDef {
    /// JSON path within the decrypted payload (e.g., "income.monthly")
    field_path: String,
    /// Pedersen generator index — each field uses a unique generator
    /// to prevent cross-field correlation
    generator_index: usize,
}

/// Pedersen commitment with explicit blinding factor management
struct PedersenCommitment {
    /// Compressed Ristretto point (32 bytes)
    point: [u8; 32],
}

/// Generate fixed-position commitment vector.
/// ALL positions are filled — unused fields get commitment-to-zero
/// with a random blinding factor, making them indistinguishable
/// from commitments to actual values.
fn generate_fixed_position_commitments(
    actual_values: &HashMap<String, u64>,
    schema: &EventTypeCommitmentSchema,
    generators: &PedersenGenerators,
) -> Vec<PedersenCommitment> {
    let mut commitments = Vec::with_capacity(schema.field_positions.len());
    let mut blinding_factors = Vec::with_capacity(schema.field_positions.len());

    for field_def in &schema.field_positions {
        let value = actual_values.get(&field_def.field_path).copied().unwrap_or(0);
        let blinding = Scalar::random(&mut OsRng);

        // C = v * G_i + r * H
        // where G_i is the field-specific generator, H is the blinding generator
        let commitment = generators.commit(
            field_def.generator_index,
            value,
            &blinding,
        );

        commitments.push(PedersenCommitment { point: commitment.compress().to_bytes() });
        blinding_factors.push(blinding);
    }

    // Blinding factors must be stored in the encrypted payload
    // so the respondent can later prove values if needed.
    // They are NOT in the plaintext header.

    commitments
}

/// Updated header: commitment_count is now fixed per event_type,
/// enforced by the builder.
impl EventBuilder<NeedsCommitments> {
    pub fn with_commitments(
        mut self,
        numeric_fields: &HashMap<String, u64>,
        schema: &EventTypeCommitmentSchema,
        generators: &PedersenGenerators,
    ) -> Result<EventBuilder<NeedsEncryption>> {
        let commitments = generate_fixed_position_commitments(
            numeric_fields, schema, generators,
        );

        // Enforce: commitment count must match schema
        assert_eq!(commitments.len(), schema.field_positions.len());

        self.commitments = Some(commitments);
        self.header.commitment_count = schema.field_positions.len() as u8;
        Ok(self.transition())
    }
}

/// Verification: auditor with access to blinding factors can
/// open specific commitments without learning other values.
fn verify_commitment(
    commitment: &PedersenCommitment,
    claimed_value: u64,
    blinding_factor: &Scalar,
    generator_index: usize,
    generators: &PedersenGenerators,
) -> bool {
    let expected = generators.commit(generator_index, claimed_value, blinding_factor);
    expected.compress().to_bytes() == commitment.point
}

/// Range proof that a committed value is in [0, 2^64)
/// Uses Bulletproofs over Ristretto for compactness.
fn generate_range_proof(
    value: u64,
    blinding: &Scalar,
    generator_index: usize,
    generators: &PedersenGenerators,
) -> BulletproofRangeProof {
    // Standard Bulletproofs range proof
    bulletproofs::prove_range(value, blinding, generators, 64)
}
```

**Example:** Event type `0x0042` (income determination) has schema with 8 fields: `[monthly_income, asset_value, deductions, household_size, threshold, net_income, copay, premium]`. Every income determination event produces exactly 8 commitments, whether or not all fields are populated. An observer sees 8 commitments and learns nothing about which fields were filled.

**Tradeoffs:**
- **Bytes:** Fixed overhead per event type. Income determination: 8 × 32 = 256 bytes regardless of fields used. Previously, a 3-field event would have been 96 bytes. Overhead: 160 bytes worst case.
- **Computation:** One Ristretto scalar multiplication per unused field (~2μs each). Negligible.
- **Schema management:** Commitment schemas must be versioned and published. Adding a new field to an event type requires a new schema version (append-only — old events keep old position count).

---

## FINDING 6: Header tags — hash commitment

**Problem:** Plaintext tags reveal determination outcomes, which is HIPAA-relevant.

**Specific change:** Replace `tags: u16` in the header with `tag_commitment: [u8; 32]` containing `SHA-256(tag_bitfield || nonce)`. The nonce is stored in the encrypted payload. Verifiers who need to check tags must decrypt the payload to obtain the nonce, then verify the commitment.

```rust
/// Tag commitment — replaces plaintext tags in header
struct TagCommitment {
    /// SHA-256(tag_bitfield_le_bytes || nonce)
    commitment: [u8; 32],
}

/// Tag nonce — stored INSIDE the encrypted payload, not in the header
struct TagNonce {
    /// 16 bytes of randomness, generated fresh per event
    nonce: [u8; 16],
}

/// Constructing the commitment (inside EventBuilder, before encryption)
fn create_tag_commitment(tags: u16, nonce: &TagNonce) -> TagCommitment {
    let mut hasher = Sha256::new();
    hasher.update(&tags.to_le_bytes());
    hasher.update(&nonce.nonce);
    TagCommitment {
        commitment: hasher.finalize().into(),
    }
}

/// The encrypted payload now includes tag-related fields
struct EncryptedPayloadContent {
    /// The actual form data
    form_data: CborValue,
    /// Tag nonce for commitment verification
    tag_nonce: [u8; 16],
    /// Actual tag bitfield (only readable after decryption)
    tag_bitfield: u16,
    /// Pedersen blinding factors (from Finding 5)
    commitment_blinding_factors: Vec<Scalar>,
}

/// Updated header — tag_commitment replaces tags
struct EventHeaderV2 {
    version: u8,
    sequence: u64,
    hlc: HLC,
    prev_hash: [u8; 32],
    payload_content_id: [u8; 32],
    actor_type: u8,
    event_type: u16,
    privacy_tier: u8,
    signing_key_id: [u8; 16],
    signature: [u8; 64],
    governance_result: u8,
    tag_commitment: [u8; 32],  // WAS: tags: u16 — now a commitment
    commitment_count: u8,
    commitments: Vec<PedersenCommitment>,
    causal_dep_count: u8,
    causal_deps: Vec<CausalDep>,
    bbs_key_version: u32,
}

/// Verification by authorized party (who has decrypted the payload)
fn verify_tag_commitment(
    header: &EventHeaderV2,
    decrypted: &EncryptedPayloadContent,
) -> bool {
    let expected = create_tag_commitment(
        decrypted.tag_bitfield,
        &TagNonce { nonce: decrypted.tag_nonce },
    );
    constant_time_eq(&header.tag_commitment, &expected.commitment)
}

/// Server-side projection queries: authorized projections that need
/// to filter by tags must decrypt the payload. This is intentional —
/// tag-based queries on encrypted data require the projection to have
/// key access. For system-level queries that need tag filtering
/// without decryption, use a separate encrypted tag index:

struct EncryptedTagIndex {
    event_id: EventId,
    /// AES-256-GCM encrypted tag bitfield, keyed by a
    /// projection-specific key derived from PRK.
    /// Different projections can have different tag index keys.
    encrypted_tags: Vec<u8>,
    /// Nonce for the tag index encryption
    index_nonce: [u8; 12],
}

/// Derive a tag-index key from PRK for a specific projection
fn derive_tag_index_key(
    prk: &[u8; 32],
    projection_id: &str,
) -> [u8; 32] {
    hkdf_sha256(
        prk,
        b"tag-index",
        projection_id.as_bytes(),
        32,
    )
}
```

**What this costs for projections:** Projections that currently filter by tags in SQL (`WHERE tags & 0x04 != 0`) must change to one of:
1. Decrypt payloads and filter in application code (simplest, works for small result sets)
2. Use the encrypted tag index (adds one AES-GCM decrypt per event in the index scan, but avoids full payload decryption)
3. Pre-compute and cache tag values in a projection-specific encrypted index on event ingest

**Tradeoffs:**
- **Bytes:** +30 bytes net per event (32-byte commitment replaces 2-byte bitfield = +30). Tag nonce is inside the already-encrypted payload, so no additional wire overhead.
- **Query performance:** Tag-based queries go from O(1) SQL WHERE clause to O(n) decrypt-and-filter or O(n) encrypted-index lookup. For typical case sizes (hundreds of events), this is sub-millisecond.
- **HIPAA compliance:** Adverse determination dates are no longer inferrable from plaintext headers. This is a hard requirement.

---

## FINDING 7: Self-correction (editorial)

**Specific change:** Remove any self-referential correction text from the document. No code change needed. This is purely editorial.

```
// No code changes. Remove the self-correction paragraph from the ADR.
// Search for any text that reads like "actually, I was wrong about..."
// or "correction: the above should say..." and rewrite the section
// to state the correct information directly.
```

**Tradeoffs:** None. Pure documentation cleanup.

---

## FINDING 8: Event granularity — batch at draft boundaries

**Problem:** Per-field-change events would create massive chains over multi-year cases. Need batching policy.

**Specific change:** Define three event granularity levels. The default is `DraftSession` which batches all changes between explicit save points.

```rust
/// Event granularity policy — configured per form definition,
/// overridable per field for audit-critical fields.
#[derive(Debug, Clone)]
enum EventGranularity {
    /// One event per user-initiated save/submit action.
    /// All field changes since last save are bundled into a single event.
    /// This is the DEFAULT. Produces 10-50x fewer events than per-field.
    DraftSession,

    /// One event per field change. Only for audit-critical fields
    /// where individual change tracking is legally required
    /// (e.g., income amount, eligibility determination).
    PerField,

    /// One event per logical form section submission.
    /// Middle ground for multi-page forms.
    PerSection,
}

/// Draft accumulator — collects field changes until a save boundary.
/// This is the client-side component that implements DraftSession batching.
struct DraftAccumulator {
    /// Changes since last save, keyed by field path
    pending_changes: IndexMap<String, FieldChange>,
    /// When accumulation started (for timeout-based auto-save)
    started_at: Option<u64>,
    /// Auto-save timeout (e.g., 30 seconds of inactivity)
    auto_save_timeout_ms: u64,
    /// Maximum pending changes before forced save
    max_pending_changes: usize,
}

struct FieldChange {
    field_path: String,
    old_value: Option<CborValue>,
    new_value: CborValue,
    changed_at: HLC,
    /// If this change was triggered by FEL calculation, record the source
    triggered_by: Option<String>, // FEL expression path
}

impl DraftAccumulator {
    fn record_change(&mut self, change: FieldChange) -> Option<DraftBatch> {
        self.pending_changes.insert(change.field_path.clone(), change);

        if self.started_at.is_none() {
            self.started_at = Some(now_ms());
        }

        // Force save if too many pending changes
        if self.pending_changes.len() >= self.max_pending_changes {
            return Some(self.flush());
        }

        None
    }

    /// Called on explicit save, submit, navigation, or auto-save timeout
    fn flush(&mut self) -> DraftBatch {
        let batch = DraftBatch {
            changes: std::mem::take(&mut self.pending_changes),
            accumulated_from: self.started_at.take().unwrap_or(now_ms()),
            accumulated_until: now_ms(),
        };
        batch
    }
}

/// The DraftBatch becomes the payload of a single event.
/// This replaces N individual field-change events with 1 batch event.
struct DraftBatch {
    changes: IndexMap<String, FieldChange>,
    accumulated_from: u64,
    accumulated_until: u64,
}

/// Event payload structure for a draft-session event
struct DraftSessionPayload {
    /// Snapshot of all current field values (not just changes)
    /// This allows reconstruction without replaying the entire chain.
    field_snapshot: HashMap<String, CborValue>,
    /// Individual changes for audit trail within this batch
    change_log: Vec<FieldChange>,
    /// FEL calculations that fired during this session
    calculations_triggered: Vec<CalculationRecord>,
}

struct CalculationRecord {
    expression: String,
    target_field: String,
    result: CborValue,
    triggered_by_fields: Vec<String>,
}

/// Chain size estimate comparison:
/// 
/// Multi-year Medicaid case, ~200 interactions:
///   Per-field:     ~5,000 events (25 fields avg per interaction)
///   DraftSession:  ~200 events (1 per interaction)  — 25x reduction
///   PerSection:    ~600 events (3 sections avg per interaction) — 8x reduction
///
/// At ~500 bytes per event header + overhead:
///   Per-field:     2.5 MB of headers alone
///   DraftSession:  100 KB of headers
```

**Tradeoffs:**
- **Audit granularity:** Individual field-change timestamps are preserved in the change_log within the encrypted payload, so nothing is lost for audit purposes. But the change_log is only visible to parties who can decrypt the event.
- **Conflict resolution:** Coarser events mean more potential for conflicts between devices. The causal ordering from Finding 1 helps, but same-session conflicts on the same field are resolved by last-writer-wins within the batch.
- **Payload size:** Batch events are larger (~2-10KB vs ~200 bytes per field event). But total chain size is 10-25x smaller, which dominates.

---

## MISSED 1: AES-256-GCM nonce management — ECIES ephemeral keys

**Problem:** Single-use DEKs make payload nonce reuse impossible, but key bag ECIES wrapping needs fresh ephemeral keys per wrapping operation.

**Specific change:** Enforce fresh ephemeral X25519 keypair per ECIES wrapping operation. Make this a type-level guarantee, not a runtime check.

```rust
/// ECIES wrapping — one-shot type that cannot be reused.
/// The ephemeral keypair is generated in the constructor and
/// consumed (moved) by the single wrap() call.

/// Ephemeral keypair — generated fresh, used exactly once, then dropped.
/// Cannot be cloned, copied, or serialized.
struct EphemeralX25519Keypair {
    secret: X25519StaticSecret, // from x25519-dalek
    public: X25519PublicKey,
}

impl EphemeralX25519Keypair {
    fn generate() -> Self {
        let secret = X25519StaticSecret::random_from_rng(OsRng);
        let public = X25519PublicKey::from(&secret);
        Self { secret, public }
    }

    /// Consume the keypair to perform one ECIES wrap.
    /// After this call, the ephemeral secret is dropped and zeroized.
    fn wrap_dek(self, recipient_pubkey: &X25519PublicKey, dek: &[u8; 32]) -> ECIESWrappedDEK {
        // ECDH: shared_secret = ephemeral_secret * recipient_pubkey
        let shared_secret = self.secret.diffie_hellman(recipient_pubkey);

        // KDF: derive wrapping key from shared secret
        let wrapping_key = hkdf_sha256(
            shared_secret.as_bytes(),
            b"ledger-ecies-dek-wrap-v1",  // context string
            &self.public.as_bytes(),       // info includes ephemeral pubkey
            32,
        );

        // AES-256-GCM wrap with ZERO nonce — safe because wrapping_key
        // is unique per (ephemeral_keypair, recipient) pair, and the
        // ephemeral keypair is used exactly once then destroyed.
        let nonce = Nonce::from([0u8; 12]); // safe: key is one-time
        let ciphertext = aes_256_gcm_encrypt_raw(&wrapping_key, &nonce, dek);

        // self.secret is dropped here — zeroize on Drop
        ECIESWrappedDEK {
            ephemeral_pubkey: self.public,
            wrapped_dek: ciphertext,
        }
    }
}

// Prevent accidental reuse at type level
impl !Clone for EphemeralX25519Keypair {}
impl !Copy for EphemeralX25519Keypair {}

/// Ensure zeroization on drop
impl Drop for EphemeralX25519Keypair {
    fn drop(&mut self) {
        self.secret.zeroize();
    }
}

/// ECIES-wrapped DEK as stored in the key bag
struct ECIESWrappedDEK {
    /// The ephemeral public key (sent to recipient so they can derive shared secret)
    ephemeral_pubkey: X25519PublicKey,  // 32 bytes
    /// AES-256-GCM(KDF(ECDH(ephemeral, recipient)), DEK)
    wrapped_dek: Vec<u8>,              // 32 + 16 = 48 bytes (DEK + GCM tag)
}

/// Key bag construction — MUST use fresh ephemeral per recipient
fn build_key_bag(
    dek: &[u8; 32],
    recipients: &[Recipient],
    prk_wrap: &dyn PRKWrapper,
) -> Result<KeyBag> {
    let mut entries = Vec::with_capacity(recipients.len() + 1);

    // One fresh ephemeral keypair PER recipient
    for recipient in recipients {
        let ephemeral = EphemeralX25519Keypair::generate(); // fresh each time
        let wrapped = ephemeral.wrap_dek(&recipient.pubkey, dek); // consumes ephemeral
        entries.push(KeyBagEntry::ECIES {
            recipient_key_id: recipient.key_id,
            wrapped: wrapped,
        });
    }

    // PRK wrapping (symmetric, via KMS) — separate mechanism, not ECIES
    let prk_wrapped = prk_wrap.wrap(dek)?;
    entries.push(KeyBagEntry::PRK {
        prk_id: prk_wrap.prk_id(),
        wrapped_dek: prk_wrapped,
    });

    Ok(KeyBag { entries })
}

/// Post-hoc key bag grant — CRITICAL: must also use fresh ephemeral
/// This is called when granting access to an event after initial creation.
fn grant_key_bag_access(
    event: &StoredEvent,
    new_recipient: &Recipient,
    prk_wrap: &dyn PRKWrapper,
    kms: &dyn KmsClient,
) -> Result<KeyBagEntry> {
    // Decrypt DEK via PRK
    let dek = kms.decrypt(
        &prk_wrap.prk_id(),
        &event.key_bag.prk_entry().wrapped_dek,
    ).await?;

    // Fresh ephemeral for the new grant
    let ephemeral = EphemeralX25519Keypair::generate();
    let wrapped = ephemeral.wrap_dek(&new_recipient.pubkey, &dek);

    dek.zeroize();

    Ok(KeyBagEntry::ECIES {
        recipient_key_id: new_recipient.key_id,
        wrapped: wrapped,
    })
}

/// Decryption by recipient
fn decrypt_dek_ecies(
    wrapped: &ECIESWrappedDEK,
    recipient_secret: &X25519StaticSecret,
) -> Result<[u8; 32]> {
    // ECDH: shared_secret = recipient_secret * ephemeral_pubkey
    let shared_secret = recipient_secret.diffie_hellman(&wrapped.ephemeral_pubkey);

    // Same KDF as wrapping
    let wrapping_key = hkdf_sha256(
        shared_secret.as_bytes(),
        b"ledger-ecies-dek-wrap-v1",
        &wrapped.ephemeral_pubkey.as_bytes(),
        32,
    );

    let nonce = Nonce::from([0u8; 12]);
    let dek = aes_256_gcm_decrypt_raw(&wrapping_key, &nonce, &wrapped.wrapped_dek)?;

    Ok(dek.try_into().map_err(|_| Error::InvalidDEKLength)?)
}
```

**Tradeoffs:**
- **Bytes:** Each ECIES entry is 80 bytes (32 ephemeral pubkey + 48 wrapped DEK). Same as before — no change.
- **Performance:** One X25519 key generation + one ECDH per recipient. ~50μs per recipient on modern hardware. For typical 2-3 recipients, negligible.
- **Safety:** The `!Clone + !Copy` trait bounds plus ownership semantics make ephemeral key reuse a compile error, not a runtime bug. This is the critical improvement.

---

## MISSED 2: BBS+ / key bag grant asymmetry documentation

**Problem:** Selective disclosure (BBS+) operates on individual fields, but key bag grants give all-or-nothing decryption. A party who decrypts sees ALL fields but can prove SELECTED fields. This asymmetry needs to be explicitly designed, not accidental.

**Specific change:** Formalize the disclosure model with three tiers: no access (no key bag entry), full decryption (key bag entry), and selective proof (BBS+ proof derived by a decrypting party). Add a `DisclosurePolicy` that governs which fields a grantee is allowed to derive BBS+ proofs for.

```rust
/// The three-tier access model, explicitly documented and enforced.
///
/// Tier 0: No Access
///   - Can see plaintext header (version, sequence, event_type, etc.)
///   - Can see tag_commitment (but not open it without the nonce)
///   - Can see Pedersen commitments (but not open them)
///   - CANNOT see any payload fields
///
/// Tier 1: Full Decryption (key bag grantee)
///   - Has a key bag entry → can decrypt DEK → can decrypt payload
///   - Sees ALL fields in the decrypted payload
///   - Subject to DisclosurePolicy for what they may further disclose
///
/// Tier 2: Selective Proof Recipient (BBS+ verifier)
///   - Receives a BBS+ selective disclosure proof from a Tier 1 party
///   - Sees ONLY the disclosed fields
///   - Can verify the disclosed fields are authentic (BBS+ verification)
///   - CANNOT decrypt the full event or see undisclosed fields

/// Disclosure policy — which fields a Tier 1 party may include
/// in BBS+ selective disclosure proofs for third parties.
struct DisclosurePolicy {
    /// Grantee identity (who received the key bag entry)
    grantee: ActorId,
    /// Event types this policy applies to
    event_types: Vec<u16>,
    /// Fields the grantee is ALLOWED to disclose via BBS+ proofs
    disclosable_fields: Vec<DisclosableField>,
    /// Fields the grantee MUST NOT disclose (even if they can decrypt them)
    redacted_fields: Vec<String>,
    /// Policy expiration
    valid_until: u64,
    /// Who set this policy (respondent, admin, regulation)
    authority: PolicyAuthority,
}

struct DisclosableField {
    /// JSON path within the decrypted payload
    field_path: String,
    /// Index in the BBS+ message vector (must match the BBS+ signing schema)
    bbs_message_index: usize,
}

/// BBS+ message vector construction — defines the canonical field ordering
/// for BBS+ signatures. This is separate from the Pedersen commitment
/// schema (Finding 5) because BBS+ operates on arbitrary byte strings,
/// not just numeric values.
struct BBSMessageSchema {
    event_type: u16,
    /// Ordered list of fields included in the BBS+ message vector.
    /// Position in this list = message index for BBS+ operations.
    fields: Vec<BBSFieldDef>,
}

struct BBSFieldDef {
    field_path: String,
    /// How to serialize this field for BBS+ signing
    encoding: BBSFieldEncoding,
}

enum BBSFieldEncoding {
    /// Deterministic CBOR of the field value
    CborValue,
    /// UTF-8 string representation
    Utf8String,
    /// Raw bytes (for binary fields)
    RawBytes,
    /// Fixed-length big-endian integer
    BigEndianU64,
}

/// Creating a selective disclosure proof (done by a Tier 1 party)
fn create_selective_disclosure(
    event: &DecryptedEvent,
    bbs_signature: &[u8],
    bbs_public_key: &BBSPublicKey,
    message_schema: &BBSMessageSchema,
    disclosure_policy: &DisclosurePolicy,
    fields_to_disclose: &[String], // subset of disclosable_fields
) -> Result<SelectiveDisclosureProof> {
    // Validate: requested fields must be in the disclosure policy
    for field in fields_to_disclose {
        if !disclosure_policy.disclosable_fields.iter().any(|f| f.field_path == *field) {
            return Err(Error::FieldNotDisclosable(field.clone()));
        }
    }

    // Validate: no redacted fields included
    for field in fields_to_disclose {
        if disclosure_policy.redacted_fields.contains(field) {
            return Err(Error::FieldRedacted(field.clone()));
        }
    }

    // Build the full message vector from decrypted payload
    let full_messages: Vec<Vec<u8>> = message_schema.fields.iter().map(|f| {
        let value = event.payload.get_field(&f.field_path);
        f.encoding.encode(value)
    }).collect();

    // Determine which indices to disclose
    let disclosed_indices: Vec<usize> = fields_to_disclose.iter().map(|field| {
        message_schema.fields.iter().position(|f| f.field_path == *field).unwrap()
    }).collect();

    // Generate BBS+ proof of knowledge
    let proof = bbs_create_proof(
        bbs_public_key,
        bbs_signature,
        &full_messages,
        &disclosed_indices,
    )?;

    Ok(SelectiveDisclosureProof {
        bbs_key_version: event.header.bbs_key_version,
        event_id: event.event_id(),
        disclosed_indices,
        disclosed_values: disclosed_indices.iter().map(|&i| {
            full_messages[i].clone()
        }).collect(),
        proof,
        policy_reference: disclosure_policy.policy_id(),
    })
}

/// Verification by a Tier 2 party (who only sees disclosed fields)
fn verify_selective_disclosure(
    proof: &SelectiveDisclosureProof,
    bbs_public_key: &BBSPublicKey,
    expected_message_count: usize, // from the BBS message schema
) -> Result<VerifiedDisclosure> {
    let valid = bbs_verify_proof(
        bbs_public_key,
        &proof.proof,
        &proof.disclosed_values,
        &proof.disclosed_indices,
        expected_message_count,
    )?;

    if !valid {
        return Err(Error::InvalidBBSProof);
    }

    Ok(VerifiedDisclosure {
        event_id: proof.event_id,
        verified_fields: proof.disclosed_indices.iter().zip(&proof.disclosed_values)
            .map(|(&idx, val)| (idx, val.clone()))
            .collect(),
    })
}
```

**Key insight documented:** The asymmetry is a feature, not a bug. It enables a critical workflow: a caseworker decrypts the full event (Tier 1), then generates a BBS+ proof disclosing only income and eligibility status to an auditor (Tier 2), without revealing medical information or SSN that was also in the event. The `DisclosurePolicy` is the governance layer that prevents the caseworker from disclosing fields they shouldn't.

**Tradeoffs:**
- **Complexity:** `DisclosurePolicy` is a new governance primitive that needs storage, versioning, and enforcement. ~300 lines of code.
- **BBS+ message schema:** Must be defined per event type and kept in sync with the commitment schema (Finding 5). These could share a definition.
- **Performance:** BBS+ proof generation is ~5ms per proof. Verification is ~3ms. Acceptable for non-real-time disclosure workflows.
- **No new cryptographic primitives:** Uses standard BBS+ proof of knowledge, already in the design.

---

## MISSED 3: WebAuthn PRF salt management

**Problem:** PRF salt must be presented on every authentication. If stored only in the browser and browser data is cleared, keys are unrecoverable. Salt storage location and HKDF info string versioning are unspecified.

**Specific change:** Store PRF salt server-side (it is not secret — it's an input to the PRF, not the output). Serve it during authentication ceremony setup. Version the HKDF info strings for future key type extensibility.

```rust
/// PRF salt is NOT secret. It's a public parameter that selects
/// which PRF output the authenticator produces. The security
/// property comes from the authenticator's internal secret, not
/// the salt.

/// Server-side credential record — extends standard WebAuthn credential storage
struct StoredCredential {
    /// Standard WebAuthn fields
    credential_id: Vec<u8>,
    public_key: CoseKey,
    sign_count: u32,
    transports: Vec<AuthenticatorTransport>,
    /// PRF salt for this credential — NOT secret, stored in cleartext
    prf_salt: [u8; 32],
    /// HKDF info string version used when this credential was registered
    hkdf_version: u8,
    /// Backup eligibility and state (WebAuthn Level 3)
    backup_eligible: bool,
    backup_state: bool,
}

/// HKDF derivation with versioned info strings
struct KeyDerivation;

/// Version 1 info strings — current
const HKDF_V1_SIGNING: &[u8] = b"formspec-ledger-ed25519-signing-v1";
const HKDF_V1_ENCRYPTION: &[u8] = b"formspec-ledger-x25519-encryption-v1";
const HKDF_V1_DID: &[u8] = b"formspec-ledger-did-key-v1";

impl KeyDerivation {
    /// Derive Ed25519 signing key from PRF output
    fn derive_signing_key(
        prf_output: &[u8; 32],
        hkdf_version: u8,
    ) -> Result<Ed25519SigningKey> {
        let info = match hkdf_version {
            1 => HKDF_V1_SIGNING,
            // Future versions add new info strings here
            // without breaking existing credentials
            v => return Err(Error::UnsupportedHKDFVersion(v)),
        };

        let ikm = prf_output;
        let salt = None; // HKDF salt is optional; PRF output has sufficient entropy
        let derived = hkdf_sha256(ikm, salt, info, 32)?;

        Ok(Ed25519SigningKey::from_bytes(&derived))
    }

    /// Derive X25519 encryption key from PRF output
    fn derive_encryption_key(
        prf_output: &[u8; 32],
        hkdf_version: u8,
    ) -> Result<X25519StaticSecret> {
        let info = match hkdf_version {
            1 => HKDF_V1_ENCRYPTION,
            v => return Err(Error::UnsupportedHKDFVersion(v)),
        };

        let derived = hkdf_sha256(prf_output, None, info, 32)?;
        Ok(X25519StaticSecret::from(derived))
    }

    /// Derive DID from PRF output
    fn derive_did(
        prf_output: &[u8; 32],
        hkdf_version: u8,
    ) -> Result<DIDKey> {
        let signing_key = Self::derive_signing_key(prf_output, hkdf_version)?;
        let verifying_key = signing_key.verifying_key();
        Ok(DIDKey::from_ed25519_public(&verifying_key))
    }
}

/// Registration flow — server generates and stores the PRF salt
async fn register_credential(
    rp: &RelyingParty,
    user: &User,
) -> Result<RegistrationResponse> {
    // Generate random PRF salt
    let prf_salt = generate_random_bytes::<32>();

    // Include PRF extension in creation options
    let options = PublicKeyCredentialCreationOptions {
        rp: rp.into(),
        user: user.into(),
        challenge: generate_challenge(),
        extensions: Extensions {
            prf: Some(PRFExtension {
                eval: Some(PRFEvaluation {
                    first: prf_salt.to_vec(),
                }),
            }),
            ..Default::default()
        },
        ..Default::default()
    };

    // After successful registration, store credential WITH salt
    // The salt is returned to the server as part of the registration response
    Ok(RegistrationResponse {
        creation_options: options,
        prf_salt,
        hkdf_version: 1, // current version
    })
}

/// Authentication flow — server provides the salt in the auth ceremony
async fn begin_authentication(
    stored_credential: &StoredCredential,
) -> Result<AuthenticationOptions> {
    Ok(PublicKeyCredentialRequestOptions {
        challenge: generate_challenge(),
        allow_credentials: vec![stored_credential.credential_id.clone().into()],
        extensions: Extensions {
            prf: Some(PRFExtension {
                eval: Some(PRFEvaluation {
                    // Server provides the salt — this is why server-side
                    // storage is critical. If the salt is lost, the user's
                    // derived keys are unrecoverable.
                    first: stored_credential.prf_salt.to_vec(),
                }),
            }),
            ..Default::default()
        },
        ..Default::default()
    })
}

/// Recovery flow — when a user loses their authenticator.
/// The PRF salt alone is useless without the authenticator.
/// A new credential must be registered, producing new keys.
/// Old events remain accessible via PRK (KMS) but the user's
/// personal key bag entries are unrecoverable without the
/// original authenticator.

struct KeyRecoveryProtocol;

impl KeyRecoveryProtocol {
    /// Step 1: Admin verifies identity out-of-band
    /// Step 2: Register new credential with new PRF salt
    /// Step 3: Issue new key bag entries for historical events
    ///         using PRK (server-side, does not need old user keys)
    async fn recover_access(
        admin: &AdminSession,
        user: &User,
        new_credential: &StoredCredential,
        kms: &dyn KmsClient,
        store: &dyn EventStore,
    ) -> Result<RecoveryResult> {
        // Derive new user pubkey from new credential
        let new_pubkey = user.derive_pubkey_from_credential(new_credential)?;

        // For each historical event, add a new key bag entry
        // wrapped for the new user pubkey (using PRK to decrypt DEK)
        let events = store.get_events_for_respondent(&user.respondent_id).await?;
        let mut granted = 0;

        for event in &events {
            let entry = grant_key_bag_access(event, &Recipient {
                key_id: new_pubkey.key_id(),
                pubkey: new_pubkey.clone(),
            }, &user.prk_wrapper, kms).await?;

            store.append_key_bag_entry(event.event_id(), entry).await?;
            granted += 1;
        }

        // Revoke old credential's key_id from future grants
        store.revoke_user_key(&user.old_key_id).await?;

        Ok(RecoveryResult { events_regranted: granted })
    }
}
```

**Tradeoffs:**
- **Storage:** 32 bytes per credential for the PRF salt. Negligible.
- **Security:** PRF salt is not secret — it's equivalent to a username in the derivation. The authenticator's internal HMAC key is the secret. Storing it server-side does not weaken security.
- **Recovery:** The recovery flow requires admin intervention and PRK access (KMS). This is intentional — fully automated recovery would defeat the purpose of hardware-bound keys.
- **Versioning:** The `hkdf_version` field costs 1 byte per credential and future-proofs the derivation. Adding a new key type (e.g., ML-KEM for post-quantum) means adding a new info string under version 2, without breaking version 1 credentials.

---

## MISSED 4: Canonical hash construction

**Problem:** ADR-0059 says `event_hash = SHA-256(envelope + ciphertext + key_bag)`. The concrete proposal has `header_hash = SHA-256(header_bytes)` with `payload_content_id` as a field. Different constructions, different integrity properties.

**Specific change:** Define ONE canonical hash construction in the `EventBuilder`. The hash covers ALL components. The `payload_content_id` in the header is a binding commitment to the plaintext (for content-addressing before encryption), while the `event_hash` covers the entire serialized event (for integrity of the whole package).

```rust
/// There are TWO hashes, serving different purposes.
/// Both are mandatory. Both are computed by the EventBuilder.
/// No other code path may compute event hashes.

/// Hash 1: payload_content_id — computed BEFORE encryption
/// Purpose: content-address the plaintext payload for deduplication,
///          caching, and binding the header to a specific plaintext.
/// Input: deterministic CBOR of the plaintext payload
/// This goes INTO the header as a field.
fn compute_payload_content_id(plaintext_cbor: &[u8]) -> [u8; 32] {
    sha256(plaintext_cbor)
}

/// Hash 2: event_hash — computed AFTER all other construction steps
/// Purpose: integrity of the entire serialized event as transmitted/stored.
/// Input: canonical concatenation of all components.
/// This is the hash used in the Merkle tree and prev_hash chain.
fn compute_event_hash(
    header_bytes: &[u8],     // canonical serialization of the finalized header
    ciphertext: &[u8],       // AES-256-GCM ciphertext (includes GCM tag)
    key_bag_bytes: &[u8],    // canonical serialization of the key bag
    bbs_signature: &[u8],    // BBS+ signature bytes
) -> [u8; 32] {
    // Domain separation to prevent cross-protocol attacks
    let mut hasher = Sha256::new();
    hasher.update(b"formspec-ledger-event-hash-v1"); // domain separator

    // Length-prefixed components to prevent ambiguity
    hasher.update(&(header_bytes.len() as u64).to_le_bytes());
    hasher.update(header_bytes);

    hasher.update(&(ciphertext.len() as u64).to_le_bytes());
    hasher.update(ciphertext);

    hasher.update(&(key_bag_bytes.len() as u64).to_le_bytes());
    hasher.update(key_bag_bytes);

    hasher.update(&(bbs_signature.len() as u64).to_le_bytes());
    hasher.update(bbs_signature);

    hasher.finalize().into()
}

/// Canonical serialization order for the header.
/// MUST be deterministic — no floating point, no map key ordering ambiguity.
/// Using a fixed struct layout, not CBOR/JSON, for the header.
fn serialize_header_canonical(header: &EventHeaderV2) -> Vec<u8> {
    let mut buf = Vec::with_capacity(256);

    // Fixed-order fields, fixed-width encoding
    buf.push(header.version);
    buf.extend_from_slice(&header.sequence.to_le_bytes());
    // HLC: wall_ms (8) + logical (4) + device_id (8) = 20 bytes
    buf.extend_from_slice(&header.hlc.wall_ms.to_le_bytes());
    buf.extend_from_slice(&header.hlc.logical.to_le_bytes());
    buf.extend_from_slice(&header.hlc.device_id);
    buf.extend_from_slice(&header.prev_hash);
    buf.extend_from_slice(&header.payload_content_id);
    buf.push(header.actor_type);
    buf.extend_from_slice(&header.event_type.to_le_bytes());
    buf.push(header.privacy_tier);
    buf.extend_from_slice(&header.signing_key_id);
    buf.extend_from_slice(&header.signature);
    buf.push(header.governance_result);
    buf.extend_from_slice(&header.tag_commitment);
    buf.push(header.commitment_count);
    for commitment in &header.commitments {
        buf.extend_from_slice(&commitment.point);
    }
    buf.extend_from_slice(&header.bbs_key_version.to_le_bytes());
    buf.push(header.causal_dep_count);
    for dep in &header.causal_deps {
        buf.extend_from_slice(&dep.event_hash_prefix);
        buf.extend_from_slice(&dep.hlc.wall_ms.to_le_bytes());
        buf.extend_from_slice(&dep.hlc.logical.to_le_bytes());
        buf.extend_from_slice(&dep.hlc.device_id);
    }

    buf
}

/// Canonical serialization for the key bag
fn serialize_key_bag_canonical(key_bag: &KeyBag) -> Vec<u8> {
    let mut buf = Vec::new();

    // Entry count
    buf.extend_from_slice(&(key_bag.entries.len() as u16).to_le_bytes());

    // Entries sorted by recipient_key_id for determinism
    let mut sorted_entries = key_bag.entries.clone();
    sorted_entries.sort_by_key(|e| e.recipient_key_id());

    for entry in &sorted_entries {
        match entry {
            KeyBagEntry::ECIES { recipient_key_id, wrapped } => {
                buf.push(0x01); // type tag
                buf.extend_from_slice(recipient_key_id);
                buf.extend_from_slice(&wrapped.ephemeral_pubkey.as_bytes());
                buf.extend_from_slice(&wrapped.wrapped_dek);
            }
            KeyBagEntry::PRK { prk_id, wrapped_dek } => {
                buf.push(0x02); // type tag
                buf.extend_from_slice(prk_id.as_bytes());
                buf.extend_from_slice(&(wrapped_dek.len() as u16).to_le_bytes());
                buf.extend_from_slice(wrapped_dek);
            }
        }
    }

    buf
}

/// Verification: given a stored event, recompute and check the hash
fn verify_event_integrity(event: &StoredEvent) -> Result<bool> {
    let header_bytes = serialize_header_canonical(&event.header);
    let key_bag_bytes = serialize_key_bag_canonical(&event.key_bag);

    let expected = compute_event_hash(
        &header_bytes,
        &event.ciphertext,
        &key_bag_bytes,
        &event.bbs_signature,
    );

    Ok(constant_time_eq(&expected, &event.event_hash))
}

/// The Merkle tree uses event_hash, which covers everything.
/// prev_hash in the header also uses event_hash of the predecessor.
/// This means:
///   - Merkle tree proves: this exact event (header + ciphertext + key_bag + BBS+)
///     was included at this position
///   - prev_hash chain proves: this event was created after that event
///   - payload_content_id proves: this ciphertext, when decrypted, produces
///     this specific plaintext (binding header to content)
```

**Tradeoffs:**
- **Bytes:** No additional storage — the two hashes already existed conceptually, they just weren't precisely defined.
- **Computation:** Two SHA-256 passes per event instead of one. SHA-256 is ~500 MB/s on modern CPUs. For a 5KB event, both hashes combined take ~20μs.
- **Compatibility:** This is a breaking change if any existing prototype computes hashes differently. But since this is pre-release, no compatibility concern.
- **Domain separation:** The `formspec-ledger-event-hash-v1` prefix prevents a different protocol from accidentally producing the same hash for a different message. Standard practice per NIST SP 800-185.
- **Length prefixing:** Prevents ambiguity attacks where component boundaries could be shifted. Standard technique from Merkle-Damgard constructions.

---

## MISSED 5: Crypto-shredding incomplete for GDPR

**Problem:** After PRK destruction, plaintext headers still reveal event count, timeline, event types, and governance results. If `ledger_id` is linkable to a respondent, this metadata constitutes personal data under GDPR Article 4(1).

**Specific change:** Three-part solution: (a) `ledger_id` is a pseudonymous identifier with the mapping stored encrypted and deletable, (b) post-shredding metadata redaction of headers, (c) a formal GDPR erasure protocol that goes beyond PRK destruction.

```rust
/// Part A: Pseudonymous ledger_id
///
/// The ledger_id MUST NOT be directly derivable from PII.
/// It is a random UUID assigned at case creation. The mapping
/// from respondent identity to ledger_id is stored separately,
/// encrypted, and deletable.

struct LedgerIdMapping {
    /// The respondent's real-world identifier (e.g., Medicaid ID)
    /// Encrypted with a mapping-specific key
    encrypted_respondent_id: Vec<u8>,
    /// The pseudonymous ledger identifier
    ledger_id: Uuid,
    /// Key used to encrypt the respondent_id
    /// This key is stored in KMS and can be independently destroyed
    mapping_key_id: KmsKeyId,
}

/// Destroying the mapping key makes the link between
/// respondent and ledger unrecoverable, even though the
/// ledger events still exist with their pseudonymous ID.

/// Part B: Header redaction after crypto-shredding
///
/// After PRK destruction, the encrypted payloads are unrecoverable.
/// But headers remain. We redact non-structural header fields.

struct HeaderRedactionPolicy {
    /// Fields that are ZEROED after crypto-shredding
    /// (they served no purpose once payloads are unrecoverable)
    redacted_fields: Vec<&'static str>,
    /// Fields that REMAIN for structural integrity
    /// (Merkle tree verification, chain integrity)
    retained_fields: Vec<&'static str>,
}

const POST_SHRED_POLICY: HeaderRedactionPolicy = HeaderRedactionPolicy {
    redacted_fields: &[
        "actor_type",         // who acted — personal data
        "event_type",         // what happened — reveals case trajectory
        "privacy_tier",       // sensitivity level — reveals nature of data
        "governance_result",  // outcome — HIPAA-relevant
        "tag_commitment",     // committed tags — no value without nonce
        "signing_key_id",     // links to actor identity
        "signature",          // links to actor key — Ed25519 sig
        "bbs_key_version",    // BBS+ key version
        "bbs_signature",      // BBS+ signature
        "commitments",        // Pedersen commitments — no value without blinding factors
        "causal_deps",        // reveals interaction patterns
    ],
    retained_fields: &[
        "version",            // structural
        "sequence",           // structural — Merkle tree position
        "hlc",                // structural — but device_id is zeroed
        "prev_hash",          // structural — chain integrity
        "payload_content_id", // structural — but payload is gone
        "commitment_count",   // structural — fixed per event type
    ],
};

/// Apply header redaction. This is a DESTRUCTIVE operation
/// that modifies stored events in-place. It is performed AFTER
/// PRK destruction and AFTER the Merkle tree checkpoint covering
/// these events has been anchored externally.
async fn redact_headers_post_shred(
    store: &dyn EventStore,
    ledger_id: &Uuid,
    anchor_verification: &AnchorProof, // proof that Merkle checkpoint is anchored
) -> Result<RedactionReport> {
    // Verify: the Merkle checkpoint covering these events has been
    // anchored (OpenTimestamps or similar). This preserves the ability
    // to prove "these events existed at this time" without revealing content.
    verify_anchor_proof(anchor_verification)?;

    let events = store.get_all_events(ledger_id).await?;
    let mut redacted_count = 0;

    for event in &events {
        let mut header = event.header.clone();

        // Zero out redactable fields
        header.actor_type = 0;
        header.event_type = 0;
        header.privacy_tier = 0;
        header.governance_result = 0;
        header.tag_commitment = [0u8; 32];
        header.signing_key_id = [0u8; 16];
        header.signature = [0u8; 64];
        header.bbs_key_version = 0;
        header.commitments = vec![]; // clear but keep commitment_count for structure
        header.causal_deps = vec![]; // clear causal deps
        header.causal_dep_count = 0;

        // Zero device_id in HLC (but keep wall_ms and logical for structure)
        header.hlc.device_id = [0u8; 8];

        // Delete ciphertext, key_bag, and BBS+ signature
        store.delete_event_payload(event.event_id()).await?;
        store.delete_key_bag(event.event_id()).await?;
        store.delete_bbs_signature(event.event_id()).await?;

        // Replace header with redacted version
        store.replace_header(event.event_id(), &header).await?;
        redacted_count += 1;
    }

    Ok(RedactionReport {
        ledger_id: *ledger_id,
        events_redacted: redacted_count,
        anchor_proof: anchor_verification.clone(),
        redacted_at: now_ms(),
    })
}

/// Part C: Complete GDPR erasure protocol
///
/// This is the full sequence. PRK destruction is step 3 of 6.
struct GDPRErasureProtocol;

impl GDPRErasureProtocol {
    async fn execute(
        kms: &dyn KmsClient,
        store: &dyn EventStore,
        merkle: &dyn MerkleService,
        anchor: &dyn AnchorService,
        ledger_id: &Uuid,
        respondent_id: &RespondentId,
    ) -> Result<ErasureReceipt> {
        // Step 1: Verify and record the erasure request
        let request = ErasureRequest::new(ledger_id, respondent_id, now_ms());
        store.record_erasure_request(&request).await?;

        // Step 2: Generate final Merkle checkpoint and anchor it
        // This preserves structural proof without content
        let checkpoint = merkle.generate_checkpoint(ledger_id).await?;
        let anchor_proof = anchor.anchor_checkpoint(&checkpoint).await?;

        // Step 3: Destroy PRK — makes all encrypted payloads unrecoverable
        let prk_id = store.get_prk_id(ledger_id).await?;
        kms.schedule_key_deletion(&prk_id, Duration::days(7)).await?;

        // Step 4: Destroy mapping key — breaks link between respondent and ledger
        let mapping = store.get_ledger_mapping(respondent_id).await?;
        kms.schedule_key_deletion(&mapping.mapping_key_id, Duration::days(7)).await?;

        // Step 5: Delete the encrypted mapping record
        store.delete_ledger_mapping(respondent_id).await?;

        // Step 6: Redact headers — remove metadata that constitutes personal data
        let redaction = redact_headers_post_shred(
            store, ledger_id, &anchor_proof,
        ).await?;

        // Generate unforgeable receipt
        Ok(ErasureReceipt {
            request_id: request.id,
            ledger_id: *ledger_id,
            // Note: respondent_id is NOT stored in receipt — link is severed
            prk_destruction_scheduled: true,
            mapping_key_destruction_scheduled: true,
            mapping_record_deleted: true,
            headers_redacted: redaction.events_redacted,
            anchor_proof,
            completed_at: now_ms(),
        })
    }
}

/// What remains after full GDPR erasure:
///
/// 1. Redacted headers with: version, sequence, prev_hash, payload_content_id,
///    commitment_count — all structural, not personal data.
/// 2. An anchored Merkle checkpoint proving these events existed at a point in time.
/// 3. An erasure receipt proving the erasure was performed.
///
/// What is GONE:
/// - All encrypted payloads (PRK destroyed)
/// - All key bag entries (PRK destroyed + entries deleted)
/// - All BBS+ signatures (deleted)
/// - The link between respondent identity and ledger_id (mapping destroyed)
/// - All metadata that could reveal case trajectory (headers redacted)
///
/// An observer who finds the ledger sees: N events of unknown type,
/// by unknown actors, with unknown outcomes, belonging to an
/// unidentifiable respondent. The structural chain (sequence + prev_hash)
/// and anchored checkpoint prove the events existed, satisfying any
/// retention-of-proof requirements without retaining personal data.
```

**Tradeoffs:**
- **Merkle tree integrity:** After header redaction, the event_hash no longer matches the stored (redacted) header. This is intentional — the anchored checkpoint from before redaction is the proof. Post-redaction integrity verification uses the checkpoint, not individual event hashes. This must be documented clearly.
- **Auditability:** After erasure, it's provable that N events existed at known times (via anchor), but not what they contained or who they involved. This satisfies GDPR Article 17 while preserving structural audit proof.
- **Latency:** The full erasure protocol involves KMS deletion scheduling (7-30 day waiting period per AWS), Merkle checkpoint generation, and external anchoring. Total: minutes to hours, not real-time.
- **Irrecoverability:** This is truly irreversible. There is no "undo" for GDPR erasure. The 7-day KMS deletion waiting period is the last chance to abort.
- **Regulatory compatibility:** The mapping-key destruction pattern is stronger than most GDPR erasure implementations. Even with a court order, the link between respondent and ledger cannot be reconstructed after the mapping key is destroyed.

---

## Summary of Header V2 Format

Consolidating all changes into the final header layout:

```rust
/// Final EventHeaderV2 incorporating all findings
struct EventHeaderV2 {
    // Structural (retained after GDPR erasure)
    version: u8,                           // = 2
    sequence: u64,                         // server-assigned canonical
    hlc: HLC,                              // Finding 1: replaces bare timestamp
    prev_hash: [u8; 32],                   // event_hash of predecessor
    payload_content_id: [u8; 32],          // MISSED 4: SHA-256 of plaintext CBOR

    // Identity & governance (redacted on GDPR erasure)
    actor_type: u8,
    event_type: u16,
    privacy_tier: u8,
    signing_key_id: [u8; 16],
    signature: [u8; 64],                   // Ed25519 over canonical header bytes
    governance_result: u8,

    // Privacy-preserving (redacted on GDPR erasure)
    tag_commitment: [u8; 32],              // Finding 6: replaces tags u16
    commitment_count: u8,                  // Finding 5: fixed per event_type
    commitments: Vec<PedersenCommitment>,   // Finding 5: fixed-position vector

    // BBS+ (redacted on GDPR erasure)
    bbs_key_version: u32,                  // Finding 2b: key version for verification

    // Causal ordering (redacted on GDPR erasure)
    causal_dep_count: u8,                  // Finding 1: max 8
    causal_deps: Vec<CausalDep>,           // Finding 1: explicit causal references
}

// Total header size estimate (no commitments, no causal deps):
//   1 + 8 + 20 + 32 + 32 + 1 + 2 + 1 + 16 + 64 + 1 + 32 + 1 + 4 + 1
//   = 216 bytes base
//
// With 8 commitments + 3 causal deps:
//   216 + (8 × 32) + (3 × 28) = 216 + 256 + 84 = 556 bytes
//
// V1 header was ~155 bytes. V2 is ~2-3.5x larger.
// For 500 events in a case: 278 KB vs 77.5 KB = 200 KB additional.
// Acceptable for the privacy and integrity gains.
```