# Expert Panel Review: Unified Ledger Architecture

**Date:** 2026-04-10
**Method:** Four independent Opus expert agents reviewed ADR-0059 and the technology survey, then provided critiques, ideal end-state visions, and concrete technical architectures.

**Experts:**
1. Distributed ledger / transparency log specialist
2. Applied cryptographer (BBS+, ZKP, MPC, FHE, PHE)
3. Distributed systems / performance engineer
4. FedRAMP SaaS solutions architect

---

## Round 1: Critical Review

### Unanimous endorsements

All four experts endorsed:
- Unified ledger as source of truth (one ledger per case, intake through resolution)
- Encrypt-then-hash (verify chain on ciphertext, decrypt separately)
- Separation from Temporal (evidentiary integrity vs execution durability)
- Rejection of blockchain (single writer, no consensus needed)
- Privacy tier model

### Unanimous recommendations

All four independently recommended:
- **Drop immudb. Use Postgres from day one.** Append-only table + hash chain column + `ct_merkle` for Merkle proofs. Own the proof format. Universal FedRAMP familiarity.
- **Drop Rekor.** OpenTimestamps alone is sufficient for external anchoring.
- **BBS+ is correct but not for Phase 1.** Design as pluggable layer with SD-JWT fallback.
- **Phase 1 must be radically simpler.** Postgres + Cloud KMS + OIDC + hash chain.

### Critical issues found

| Issue | Expert | Severity | Fix |
|-------|--------|----------|-----|
| PRK must be independent random keys, not HKDF-derived from TMK | Crypto | Critical | If derived, anyone with TMK + respondent ID rederives the key. Crypto-shredding breaks. |
| Hash concatenation needs canonical serialization | Crypto | High | Variable-length fields without domain separation create parsing ambiguity. Use deterministic CBOR or COSE. |
| Crypto-shredding must cascade to materialized views | Crypto | High | Postgres views contain decrypted plaintext. Purge all projections on key destruction. |
| Ledger append activity must be idempotent | Systems | High | Temporal retries on worker crash. Deduplicate by activity ID. |
| View rebuild needs checkpoint snapshots from day one | Systems | High | Full replay of millions of encrypted events is impractical. Snapshot views at epoch boundaries. |
| Key rotation protocol missing for all three levels | Crypto + Ledger | High | TMK rotation requires PRK re-wrapping. PRK rotation retains old for historical. BBS+ keys need version binding. |
| Respondent key loss unsolvable for non-technical populations | Ledger + SaaS | Medium | Medicaid applicants cannot manage DID key pairs. Need practical fallback. |

### Opportunities identified

| Opportunity | Expert | Value |
|-------------|--------|-------|
| PHE for equity monitoring | Crypto | Compute disparity rates on ciphertexts. Monitor never sees individual cases. |
| MPC for cross-agency analytics | Crypto | Joint metrics without sharing case data. |
| Most views rebuild from plaintext envelopes without decryption | Systems | Only "current case file" needs decrypted content. |
| Projection pipeline as Temporal worker with LISTEN/NOTIFY | Systems | Exactly-once projection with lag visibility. |

---

## Round 2: Ideal End-State Vision

All four converged on the same fundamental principle:

> **Trust is replaced by proof at every layer.**

- Respondents hold records the government cannot repudiate
- Government holds records it cannot silently alter
- Auditors verify rather than trust
- Cross-agency sharing requires no broker -- just math

---

## Round 3: Concrete Technical Architectures

### Ledger Expert: Data Structures

**Event format:** Fixed 181-byte binary header (version, sequence, timestamp, prev_hash, payload_hash, actor_type, event_type, privacy_tier, signing_key_id, Ed25519 signature). Variable CBOR-encoded payload (ciphertext + key bag + BBS+ signature). Chain links headers only -- verify integrity by streaming headers without touching payloads.

**Merkle tree:** RFC 6962 history tree (Certificate Transparency). Binary tree, leaves are event hashes in append order. Supports: append O(log n), inclusion proof O(log n), consistency proof O(log n). For 50,000 events: depth 16, inclusion proof is 16 hashes (512 bytes).

**Cross-ledger proofs:** Referencing event contains: target case's tree root at reference point, signed tree head, signing key ID. Verify: fetch signed tree head, confirm root matches, verify signature. Consistency: request proof between reference size and current size (pair of hash lists, no data transfer). Cross-agency: signed tree heads chain to keys published at well-known DID document endpoints.

**Export artifact:** Deterministic ZIP containing: `events/` (header + payload per event), `tree.bin` (full Merkle tree node array), `checkpoints/` (signed tree heads with OpenTimestamps .ots files), `keys/public.json` (all public keys), `schema/` (event schemas at each version), `verify.sh` (self-contained verification script). Verifiable on air-gapped machine.

**Respondent wallet:** W3C VC wallet holding identity credentials (IAL2 VC from ID.me/Login.gov) and case receipts (VC from platform with case ID, latest signed tree head, respondent_wrapped_dek entries, BBS+ public keys). Wallet stores DID private key (Ed25519). Periodic consistency verification against platform. Key backup: Shamir 3-of-5 secret sharing across respondent's chosen recovery contacts.

**Archival:** Do not compact. Build epoch summaries every 10,000 events (tree root, cumulative statistics, key rotation record). View rebuild starts from nearest epoch snapshot, not genesis.

### Cryptographer: FHE for Verifiable Eligibility

**Scheme: TFHE, not CKKS.** Eligibility determinations need exact Boolean logic (threshold comparisons, lookups, conditionals), not approximate arithmetic. CKKS's approximate comparison is a due process violation. TFHE operates on exact Boolean and small-integer arithmetic with programmable bootstrapping.

**Performance (achievable now):**
- TFHE-rs on CPU: 15-40ms per programmable bootstrapping
- Eligibility determination: 50-200 operations = 1-8 seconds CPU
- With GPU acceleration: 100ms-2.5s per determination
- Eligibility rules are shallow circuits (5-10 levels of dependent logic) -- well within TFHE's range
- Real bottleneck: TFHE public key is ~40MB (distribute once per case, cache aggressively)

**Proof-carrying adverse decision data structure:**
```
AdverseDetermination {
  case_id:                CaseRef,
  encrypted_inputs:       Vec<TFHECiphertext>,     // hashes reference ledger events
  circuit_id:             Hash,                     // hash of compiled eligibility program
  circuit_version:        SemanticVersion,          // auditable, published rule set
  encrypted_result:       TFHECiphertext,           // encrypted determination
  encrypted_reason:       Vec<TFHECiphertext>,      // which threshold failed, encrypted
  computation_transcript: TranscriptHash,           // deterministic replay proof
  bbs_signature:          BBS+Signature,            // signs [circuit_id, input_hashes, output_hash]
  ledger_event_hash:      SHA256,                   // chain inclusion
}
```

Respondent decrypts result and reason with TFHE secret key. Verifies computation by checking transcript against published circuit. BBS+ enables selective disclosure of denial reason without revealing exact figures.

**Crypto stack:** TFHE-rs (Zama, Rust-native, BSD-3) for encrypted computation. BBS+ via `bbs_plus` crate for selective disclosure. AES-256-GCM for symmetric payload encryption. SHA-256 for hash chain. TFHE parameters: 128-bit security, `PARAM_MESSAGE_2_CARRY_2`.

**Additional recommendations:**
- PHE (Paillier) for equity monitoring aggregation over encrypted demographic data
- SD-JWT as BBS+ fallback for procurement environments requiring IETF/NIST-only primitives
- Pluggable selective disclosure layer (SD-JWT Phase 1, BBS+ Phase 2)

### Systems Engineer: Dual-Mode Projection Pipeline

**Key architectural decision:** Add Pedersen commitment fields to the event envelope from day one. Cost: ~50 microseconds on write path (negligible). Pedersen commitments are additively homomorphic -- sums, counts, averages work TODAY using elliptic curve addition, no bootstrapping, no GPU.

**Write path with commitments:**
```
1. Generate DEK, encrypt payload (AES-256-GCM)          ~1μs
2. Wrap DEK with PRK and respondent key                  ~2-5ms (KMS)
3. Compute Pedersen commitments over numeric fields       ~50μs (CPU)
4. Hash envelope + ciphertext + commitments + key bag     ~1μs
5. Append to ledger                                       ~1-2ms (Postgres fsync)
Total: ~3-8ms per event
```

**Dual-mode projection:** Every view definition compiles to two artifacts: a SQL projection (for Postgres, runs today) and an FHE circuit (for GPU computation, runs when hardware available). Same declarative source, two backends. View consumer never knows which backend produced the result.

**GPU infrastructure:** Dedicated GPU nodes for FHE projection as a separate Kubernetes job queue. A100 nodes autoscale based on projection backlog. Views needing sub-second freshness get dedicated GPU slices. Analytics views tolerate minutes of lag and run in batch.

**TFHE GPU performance for view projection:**
- Single Boolean gate bootstrapping: ~10μs on A100 (2173x CPU speedup)
- Filter predicate over one event (20 Boolean ops): ~200μs
- View projection over 10,000 events for one case: ~2 seconds on single A100
- Batch rebuild over 1M cases with 8 A100s: hours, not weeks

**Temporal integration unchanged.** Workflow engine does NOT operate on encrypted state. Activities call FHE projection service when they need computed results over encrypted data. Activity receives committed result or proven cleartext with ZK proof. Temporal replay unaffected.

**Verifiable materialized views as byproduct:** Every FHE computation produces a result mathematically bound to input ciphertexts. Pedersen commitment over an aggregate carries a proof the aggregate was correctly computed. Auditors verify the view without replaying events.

### SaaS Architect: Three-Plane Deployment

**Storage plane:** Unified ledger, unchanged. Append-only, hash-chained, encrypted payloads. Postgres. The platform operates it but reads nothing.

**Compute plane:** GPU nodes running FHE. Tenant submits encrypted queries. Platform evaluates homomorphically. Result returns encrypted under tenant's key. Platform never sees plaintext. Eligibility: `income > 138% * FPL` over encrypted integers -- fast today with TFHE-rs.

**Key plane:** Tenant-controlled KMS, completely outside platform boundary. Platform never holds, derives, or transits any decryption key. Tenant encrypts before data enters, decrypts after results leave.

**FedRAMP impact:** Counterintuitively harder short-term (no 3PAO has assessed FHE systems; will be first), radically easier long-term. Half of NIST 800-53 Moderate controls address data protection, access control, personnel security for data access, and audit logging of access. When the platform mathematically cannot access data, those controls become "N/A -- platform cannot access data, see cryptographic proof." Continuous monitoring scope shrinks. Incident response scope shrinks (platform compromise cannot produce data breach). Insider threat POA&M items evaporate.

**CISO pitch:** "You do not need to trust us. A compromised platform operator, a nation-state attacker with root access, or a subpoena served on us -- none produce your data. Your data is in your KMS, encrypted under your keys, and our compute operates on ciphertext. We can prove this mathematically. Your ISSO can verify it independently." Not a trust argument. A risk elimination argument.

**Pricing:** GPU-accelerated FHE costs ~10-100x cleartext computation today (~$2-4/GPU-hour for A100). Price per-case-per-month. Premium tier at 3-5x cleartext tier. As FHE hardware accelerates (Intel, DARPA DPRIVE, dedicated ASICs), multiplier drops toward 2x and eventually approaches parity.

**Dream deployment:** Every agency runs the key plane. Shared or regional compute plane runs FHE on GPU clusters in GovCloud. Storage per-agency or shared with per-tenant encryption. No agency trusts any other. Cross-agency queries run as multi-party FHE. Platform orchestrates and learns nothing. Maximally useful, maximally ignorant.

---

## Synthesis: What to Build Now vs. What to Design For

### Build now (Phase 1)

| Component | Decision |
|-----------|----------|
| Storage | Postgres append-only table + hash chain column + UPDATE/DELETE trigger |
| Event format | Fixed binary header + CBOR payload. **Include Pedersen commitment fields from day one** (50μs, negligible) |
| Signing | HMAC-SHA256 checkpoints initially |
| Key management | Cloud KMS (AWS GovCloud for FedRAMP). Per-respondent keys. Independent random PRKs (NOT derived from TMK). |
| Identity | OIDC (Login.gov, ID.me) with VC adapter stub |
| Projection | Temporal worker tailing Postgres via LISTEN/NOTIFY. View snapshots at epoch boundaries. |
| Append idempotency | Deduplicate by Temporal activity ID |
| Crypto-shredding | Key destruction + explicit view/cache purge cascade |

### Design for now, build later

| Component | When | Why wait |
|-----------|------|----------|
| Merkle proofs via `ct_merkle` | When auditors demand inclusion proofs | Proof format is independent of storage |
| COSE signing via `coset` | When checkpoint signing needs formal standard | HMAC-SHA256 is sufficient initially |
| OpenTimestamps anchoring | When government trust story demands third-party verifiability | Add without changing event format |
| SD-JWT selective disclosure | When cross-agency sharing or FOIA requires field-level proofs | Pluggable behind disclosure interface |
| BBS+ selective disclosure | When W3C BBS Cryptosuites reach Recommendation | Replaces SD-JWT behind same interface |
| TFHE eligibility computation | When GPU infrastructure is justified by case volume | Pedersen commitments in events are the bridge |
| PHE equity monitoring | When disparity monitoring over encrypted demographics is required | Commitments already in events |

### The bridge: Pedersen commitments

The systems engineer's insight is the most actionable finding of the entire review. Pedersen commitments cost 50μs per event, add ~64 bytes per numeric field, and enable:
- Linear aggregation (sums, counts, averages) over encrypted data TODAY, no GPU
- FHE view projections when GPU infrastructure arrives (the commitments are the input format)
- Verifiable materialized views as a byproduct of homomorphic computation

Adding them to the event envelope from day one costs nothing and buys everything. Every future capability in the north star architecture -- FHE eligibility, PHE equity monitoring, verifiable views, zero-knowledge cross-agency queries -- depends on having commitments in the event format. Retrofitting them later means re-processing every historical event.

---

## Sources

- [Cheddar: GPU-accelerated FHE (ASPLOS 2026)](https://arxiv.org/abs/2407.13055)
- [Zama TFHE-rs GPU benchmarks](https://docs.zama.org/tfhe-rs/get-started/benchmarks/gpu)
- [Zama Concrete FHE compiler](https://github.com/zama-ai/concrete)
- [W3C Verifiable Credentials 2.0 (Recommendation, May 2025)](https://www.w3.org/TR/vc-data-model-2.0/)
- [W3C Data Integrity BBS Cryptosuites v1.0 (CR)](https://www.w3.org/TR/vc-di-bbs/)
- [EDPB Guidelines 02/2025 (crypto-shredding for GDPR)](https://dev.to/veritaschain/how-i-solved-the-unsolvable-gdpr-mifid-ii-paradox-with-crypto-shredding-2k61)
- [RFC 6962 Certificate Transparency](https://datatracker.ietf.org/doc/rfc6962/)
- [ct_merkle Rust crate](https://docs.rs/ct-merkle/latest/ct_merkle/)
- [bbs_plus Rust crate](https://crates.io/crates/bbs_plus)
- [coset COSE Rust crate (Google)](https://crates.io/crates/coset)
- [OpenTimestamps](https://opentimestamps.org/)
- [immudb](https://immudb.io/)
- [HashiCorp Vault crypto-shredding](https://mysticmind.dev/encrypting-and-crypto-shredding-of-pii-data-in-marten-documents-using-hashicorp-vault)
