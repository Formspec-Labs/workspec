# Unified Ledger Technology Survey

**Date:** 2026-04-10
**Purpose:** Evaluate existing tools, frameworks, and ledger systems that could accelerate development of the unified ledger described in ADR-0059.
**Method:** Web research across 18 search queries covering immutable storage, signing, selective disclosure, transparency logs, identity, key management, and government compliance.

---

## Executive Summary

The unified ledger's ideal end state requires seven capabilities: immutable storage with Merkle proofs, event signing, external anchoring, selective disclosure, encryption-based deletion, decentralized identity, and portable export. **Every capability has at least one mature open-source solution.** The novel work is composing them and defining the event taxonomy.

| Capability | Recommended technology | Maturity | Rust support |
|-----------|----------------------|----------|-------------|
| Immutable storage + Merkle proofs | **immudb** | Production (v2.0) | No native SDK; gRPC API available |
| Event/checkpoint signing | **COSE (RFC 9052)** via `coset` crate | Stable | **Yes** -- Google-maintained Rust crate |
| Selective disclosure | **BBS+ signatures** via `bbs_plus` crate | W3C Candidate Recommendation | **Yes** -- multiple Rust crates |
| External anchoring | **OpenTimestamps** + **Sigstore Rekor** | Production | OTS: Python/JS; Rekor: Go; both have REST APIs |
| Key management + crypto-shredding | **HashiCorp Vault** Transit engine | Production | REST/gRPC API; no native Rust SDK needed |
| Decentralized identity | **W3C DIDs + Verifiable Credentials 2.0** | **W3C Recommendation** (May 2025) | Multiple Rust crates |
| Merkle tree (if building custom) | **ct_merkle** or **rs_merkle** Rust crates | Stable | **Yes** -- purpose-built |

**The biggest finding:** W3C Verifiable Credentials 2.0 became a full W3C Recommendation in May 2025. BBS+ Data Integrity Cryptosuites are in Candidate Recommendation. The EU eIDAS 2.0 regulation (effective June 2026) gives DID-based identities full legal standing. The identity and selective disclosure standards we need are no longer experimental -- they're standardized or weeks away from it.

---

## 1. Immutable Storage with Cryptographic Verification

### immudb (Codenotary)

**What it is:** Open-source immutable database with built-in cryptographic verification. Written in Go. Apache 2.0 license (BSL for some enterprise features).

**Key capabilities:**
- Append-only storage -- data cannot be changed or deleted
- Built-in Merkle tree with inclusion proofs and consistency proofs
- Client-side verification -- SDK verifies server responses against Merkle root
- SQL and key-value interfaces
- Time travel queries -- reconstruct state at any historical point
- Multi-database support (one DB per tenant or per case)
- Embedded mode (can run in-process)
- v2.0 released with rebuilt indexing layer and performance improvements

**Fit for unified ledger:**
- Provides immutability, Merkle proofs, and client verification out of the box
- SQL interface means standard tooling works for materialized view projections
- Multi-database maps to per-tenant or per-case isolation
- Time travel supports point-in-time case state reconstruction

**Limitations:**
- No native Rust SDK. Go SDK is primary. gRPC API available for Rust integration via tonic.
- Designed for key-value and SQL workloads, not event streaming. Event append patterns need adaptation.
- License: Apache 2.0 for core, BSL (Business Source License) for some enterprise features. Check which features fall under BSL for AGPL compatibility.

**Production use cases:** Financial services (tamper-evident transaction logs), CI/CD pipeline protection, sensor data verification, election verification prototype.

### Alternatives to immudb

| System | Status | Fit |
|--------|--------|-----|
| **Google Trillian** | Maintenance mode. Replaced by **Trillian Tessera** (tile-based, alpha). | Good for Merkle proofs. Not a database -- needs a storage backend. More infrastructure to run. |
| **Trillian Tessera** | Alpha (2025). Tile-based architecture reduces operational cost. | Next-gen transparency log. Too early for production dependency. Watch for GA. |
| **Azure Confidential Ledger** | GA. Runs on hardware enclaves (SGX). REST API. | FedRAMP-relevant (Azure Government). Vendor lock-in (Azure only). No Rust SDK. |
| **SQL Server Ledger Tables** | GA (SQL Server 2022). Merkle tree built into SQL Server. | Append-only tables with cryptographic verification. Digests stored externally (Azure Blob, ACL). SQL Server dependency. |
| **Kurrent (formerly EventStoreDB)** | Production. Event sourcing database with Rust client. | No cryptographic verification. Append-only streams. Good for event sourcing, not for tamper evidence. |
| **Amazon QLDB** | **Deprecated.** End of support July 2025. | Dead product. Do not adopt. AWS recommends Aurora PostgreSQL as replacement (no crypto verification). |
| **Postgres with append-only tables** | DIY. Trivial to implement. | No built-in Merkle proofs or client verification. Would need custom hash chain and proof implementation. Simplest MVP path. |

**Recommendation:** immudb for the production ledger. Postgres with append-only tables for MVP/Phase 1 (with hash chain column). Migrate to immudb when Merkle proof verification is needed. Monitor Trillian Tessera for future consideration.

---

## 2. Event and Checkpoint Signing

### COSE (RFC 9052) via `coset` Rust crate

**What it is:** CBOR Object Signing and Encryption. IETF standard for signing and encrypting structured data. Binary format (compact).

**Rust support:** `coset` crate maintained by Google. Used in Android platform. Supports `CoseSign1` (single signer), `CoseSign` (multi-signer), `CoseEncrypt`, `CoseMac`.

**Why COSE over JWS:**
- More compact binary encoding (CBOR vs JSON)
- Better support for deterministic serialization (important for hash chain)
- Native support in WebAuthn, FIDO2, and emerging standards
- `coset` crate is actively maintained and production-quality

**Alternative:** JWS (RFC 7515) -- JSON-based, more widely understood, slightly larger. Use JWS if JSON-native format is preferred over compactness.

### Additional signing crates

| Crate | Purpose |
|-------|---------|
| `coset` (Google) | COSE structures and signing |
| `cose_minicbor` | No-std COSE implementation for embedded/WASM |
| `ring` | Cryptographic primitives (Ed25519, P-256) used by signing crates |
| `ed25519-dalek` | Ed25519 signatures (fast, compact, widely used for audit signing) |

**Recommendation:** `coset` for checkpoint signing (compact, standard). Ed25519 via `ring` or `ed25519-dalek` for per-event signatures (fast, 64-byte signatures).

---

## 3. Selective Disclosure via BBS+ Signatures

### W3C Status (as of April 2026)

- **W3C Verifiable Credentials 2.0:** Full W3C Recommendation since May 2025.
- **Data Integrity BBS Cryptosuites v1.0:** Candidate Recommendation. W3C actively seeking implementation feedback. Requires two independent implementations per feature to advance.
- **The BBS Signature Scheme (IRTF):** IRTF draft at CFRG. Defines the core math.
- **Hyperledger Aries:** Archived (April 2025), but BBS credential exchange RFC (0646) lives on in the decentralized identity ecosystem.

### Rust implementations

| Crate | Description | Status |
|-------|-------------|--------|
| `bbs_plus` | BBS and BBS+ per academic spec. Signing, proof generation, selective disclosure. | Active on crates.io |
| `bbs` | BBS+ with zero-knowledge proofs. | Older, some dependencies deprecated |
| MATTR FFI wrapper | Rust BBS implementation exposed via FFI for Node.js/WASM | MATTR deprecated this in favor of their Pairing Crypto library |

### What BBS+ gives us

1. **Sign all fields of an event** at creation time (BBS signs a vector of messages, one per field).
2. **Derive a proof** revealing only selected fields (e.g., "income was verified" without the amount).
3. **Proof is verifiable** against the BBS public key without revealing hidden fields.
4. **Unlinkable proofs** -- two proofs from the same signature cannot be correlated (privacy).

### NIST perspective

NIST presented on BBS+ in October 2023 at their Crypto Reading Club, indicating active federal interest in the scheme for government identity applications.

**Recommendation:** Adopt BBS+ via the `bbs_plus` crate. The W3C VC 2.0 ecosystem is the right frame. BBS is in Candidate Recommendation -- not yet a full Recommendation, but actively progressing with multiple implementations. For government procurement, note that BBS+ is being evaluated by NIST and is part of the EU eIDAS 2.0 ecosystem.

---

## 4. External Anchoring

### OpenTimestamps

**What it is:** Open-source protocol for Bitcoin-anchored timestamps. Proves a document existed at a specific time.

**How it works:** Aggregates multiple document hashes into a Merkle tree. Anchors the Merkle root to the Bitcoin blockchain via an OP_RETURN transaction. Provides inclusion proofs. Verification is offline -- the proof is self-contained.

**Production status:** Stable, production software used for years. Timestamp format is stable and backward-compatible.

**Fit for unified ledger:** Anchor ledger checkpoint hashes to Bitcoin. Provides the strongest possible "this checkpoint existed at this time" proof -- Bitcoin's PoW makes retroactive forgery computationally infeasible. Free to use (public calendar servers).

**Limitation:** Bitcoin block time is ~10 minutes. A timestamp is pending until confirmed. For audit purposes this is fine (checkpoints are periodic, not real-time). Not suitable for per-event anchoring.

### Sigstore Rekor

**What it is:** Open-source transparency log for signed metadata. CNCF project. Built on Google Trillian (v1) and Trillian Tessera (v2).

**How it works:** Append-only log of signed entries. Merkle tree provides inclusion and consistency proofs. Public instance available. Anyone can verify entries.

**Recent developments:**
- Rekor v2 GA (2025) -- cheaper to run, simpler to maintain, tile-based architecture
- Research dataset available on BigQuery for audit analysis
- Being extended beyond software signing to ML model signing

**Fit for unified ledger:** Submit checkpoint signatures to Rekor. Public log provides continuous transparency -- anyone can audit the log for consistency (append-only, no mutations). Stronger than OpenTimestamps for ongoing monitoring (not just point-in-time proof).

**Limitation:** Rekor is designed for software supply chain. Using it for case audit checkpoints is a novel application. The Rekor team may not prioritize non-software use cases. Self-hosting Rekor is an option.

**Recommendation:** Use both. OpenTimestamps for offline-verifiable, Bitcoin-anchored point-in-time proofs. Rekor for continuous public transparency and consistency monitoring. They serve different verification scenarios.

---

## 5. Key Management and Crypto-Shredding

### HashiCorp Vault

**What it is:** Open-source secrets management and encryption service. Transit engine provides encryption-as-a-service.

**Crypto-shredding pattern with Vault:**
1. Create a named encryption key per respondent in Vault's Transit engine.
2. Encrypt event payloads using Vault's `encrypt` endpoint (Vault manages the DEK).
3. To delete a respondent's data: `DELETE /transit/keys/:respondent-key-name` -- Vault destroys the key material.
4. Ciphertext remains in the ledger. Content is irrecoverable.

**Production examples:** Multiple published implementations of GDPR-compliant crypto-shredding using Vault with event-sourced systems (Marten documents, Kafka streams).

**EDPB endorsement:** The European Data Protection Board released Guidelines 02/2025 specifically addressing crypto-shredding for GDPR compliance on immutable ledger systems. This approach is now explicitly endorsed by the EU data protection authority.

**Alternative: Cloud KMS**
- AWS KMS, GCP KMS, Azure Key Vault all support per-key creation/destruction.
- Simpler to operate than self-hosted Vault. Vendor dependency.
- For FedRAMP: AWS KMS (GovCloud) and Azure Key Vault (Government) are authorized.

**Recommendation:** Vault for self-hosted / Dedicated tier. Cloud KMS for Shared and Regulated Cloud tiers. The crypto-shredding pattern is identical regardless of KMS provider.

---

## 6. Decentralized Identity

### W3C Verifiable Credentials 2.0

**Status:** Full W3C Recommendation since May 2025. Seven specifications published simultaneously. This is no longer experimental -- it is a ratified standard.

**What it provides:**
- Standard data model for cryptographically verifiable credentials
- Issuer, Holder, Verifier roles
- Multiple proof mechanisms (Data Integrity, JWT, COSE)
- BBS+ cryptosuite for selective disclosure (Candidate Recommendation)

### W3C Decentralized Identifiers (DIDs)

**Status:** W3C Recommendation since 2022. Multiple DID methods in production.

**Relevant methods:**
- `did:web` -- DNS-based, simple, works with existing web infrastructure
- `did:key` -- Self-contained, no resolution needed, good for offline/embedded
- `did:ion` -- Bitcoin-anchored, decentralized resolution

### Government adoption signals

- **EU eIDAS 2.0:** Effective June 2026. All 27 EU member states must issue digital wallets meeting technical specifications. DID-based identities get full legal standing.
- **US mDL:** Mobile Driver's License rollout across US states using ISO 18013-5 (interoperable with VC ecosystem).
- **NIST:** Evaluating BBS+ for government identity applications (October 2023 Crypto Reading Club presentation).

**ID.me and Login.gov:** Neither currently issues W3C Verifiable Credentials directly. Both use OIDC/SAML. However, the VC model can wrap OIDC assertions -- an adapter layer maps OIDC identity proofing results into VCs. The EU wallet mandate may accelerate VC adoption by government identity providers.

**Recommendation:** Design the ledger's identity model around W3C VCs and DIDs. Use an adapter layer for current OIDC-based providers (ID.me, Login.gov). As government providers adopt VCs natively (driven by eIDAS 2.0 and mDL), the adapter becomes a passthrough.

### Rust ecosystem for VCs/DIDs

| Crate/Project | Description |
|---------------|-------------|
| `ssi` (Spruce Systems) | Rust library for DIDs, VCs, JSON-LD, linked data proofs |
| `didkit` (Spruce Systems) | CLI and library for DID and VC operations |
| `identity.rs` (IOTA Foundation) | Rust library for decentralized identity |
| `bbs_plus` | BBS+ signatures for selective disclosure |

---

## 7. Merkle Tree Implementations (Rust)

If building custom verification rather than using immudb's built-in Merkle tree:

| Crate | Features | Best for |
|-------|----------|----------|
| `ct_merkle` | Append-only, inclusion proofs, consistency proofs. Certificate Transparency compatible. | Closest to our transparency log use case. Designed for CT. |
| `rs_merkle` | Multi-proofs, transactional changes, rollback. "Most advanced Merkle tree library for Rust." | General purpose. Richer API than ct_merkle. |
| `merkle-rs` | Inclusion proofs, consistency proofs. Flexible. | Simple, clean API. |
| `merkletree` | Full arity tree, CT encoding scheme. | If CT compatibility matters. |

**Recommendation:** `ct_merkle` if building a custom transparency log layer. `rs_merkle` for general-purpose Merkle operations. If using immudb, its built-in Merkle tree is sufficient -- these crates are only needed for custom verification or offline proof checking in the export artifact.

---

## 8. Government Compliance Landscape

### FedRAMP requirements for audit logs

- Audit logs must be append-only and tamper-evident
- Minimum 90 days online retention, 1 year archived
- Centralized log management
- Real-time alerting on security events
- Time synchronization (NTP)
- Cross-organizational sharing for interconnected systems

The unified ledger's hash-chained, externally-anchored design exceeds these requirements. FedRAMP auditors are familiar with immutable audit logs but may not have encountered Merkle-proof-based verification. The self-verification capability (anyone can verify the chain without platform access) is stronger than what FedRAMP requires.

### EDPB Guidelines 02/2025 (GDPR + immutable ledgers)

The European Data Protection Board explicitly addressed crypto-shredding for GDPR compliance on blockchain and immutable ledger systems in 2025. Key findings:
- Crypto-shredding is accepted as "effective deletion" under GDPR
- The encrypted data remaining in the chain does not constitute personal data if the key is provably destroyed
- Per-data-subject keys are the recommended approach

This is direct regulatory endorsement of our crypto-shredding architecture.

### Court admissibility

FRE 803(6) business records exception requires: regular practice, contemporaneous recording, person with knowledge or automated process, trustworthy. The unified ledger satisfies all five. SQL Server Ledger Tables documentation explicitly references this pattern for regulatory audit trails.

---

## 9. Technology Risk Assessment

| Technology | Risk | Mitigation |
|-----------|------|------------|
| **immudb** | Small company (Codenotary). BSL license for some features. No Rust SDK. | Core is Apache 2.0. gRPC API for Rust. Can migrate to Postgres+custom Merkle if Codenotary fails. |
| **BBS+** | W3C Candidate Recommendation, not full Recommendation. Government procurement may require finalized standard. | VC 2.0 is a Recommendation. BBS+ is a proof suite within it. EU eIDAS 2.0 and NIST interest signal government acceptance. BBS+ is a progressive enhancement -- ledger works without it. |
| **Sigstore Rekor** | Designed for software supply chain. Non-software use is novel. | Self-host Rekor for our use case. Or use only OpenTimestamps for external anchoring (simpler). |
| **Vault** | HashiCorp licensing changes (BSL for Vault Enterprise). | Vault Community Edition remains open source. Cloud KMS is the alternative. OpenBao is the open-source Vault fork. |
| **W3C VCs** | Government identity providers (ID.me, Login.gov) don't issue VCs yet. | Adapter layer wraps OIDC assertions. EU eIDAS 2.0 mandate (June 2026) will drive adoption. |
| **OpenTimestamps** | Depends on Bitcoin blockchain. If Bitcoin fails (unlikely), timestamps lose their anchor. | OpenTimestamps proofs are self-contained -- they remain verifiable as long as the Bitcoin blockchain history exists. Even a hypothetical Bitcoin shutdown preserves existing proofs. |

---

## 10. Recommended Architecture

```
+-------------------------------------------------------+
|            Unified Ledger Spec (WE BUILD)              |
|  Event taxonomy - Privacy tiers - Compliance semantics |
+-------------------------+-----------------------------+
                          |
+-------------------------v-----------------------------+
|            Cryptographic Layer                          |
|                                                        |
|  Signing:     coset (COSE RFC 9052) + ed25519-dalek   |
|  Disclosure:  bbs_plus (BBS+ selective disclosure)     |
|  Encryption:  AES-256-GCM via ring                     |
|  Hashing:     SHA-256 via ring                         |
|  Merkle:      ct_merkle or rs_merkle (for export       |
|               verification; immudb for live system)     |
+-------------------------+-----------------------------+
                          |
+-------------------------v-----------------------------+
|            Storage + Verification                      |
|                                                        |
|  Phase 1:  Postgres append-only + hash chain column    |
|  Phase 2+: immudb (Merkle proofs, client verification) |
|                                                        |
|  External anchoring:                                   |
|    OpenTimestamps (Bitcoin, offline-verifiable)         |
|    Sigstore Rekor (continuous transparency, optional)   |
+-------------------------+-----------------------------+
                          |
+-------------------------v-----------------------------+
|            Key Management                              |
|                                                        |
|  Shared Cloud:    Cloud KMS (AWS/GCP/Azure)            |
|  Regulated Cloud: Cloud KMS (GovCloud) + ext anchoring |
|  Dedicated:       HashiCorp Vault (self-hosted)        |
|                                                        |
|  Pattern: per-respondent KEK + per-event DEK           |
|  Crypto-shredding: KEK destruction = GDPR erasure      |
|  Endorsed by EDPB Guidelines 02/2025                   |
+-------------------------+-----------------------------+
                          |
+-------------------------v-----------------------------+
|            Identity Layer                              |
|                                                        |
|  Standard: W3C VCs 2.0 (Recommendation) + DIDs         |
|  Rust:     ssi/didkit crates                           |
|  Adapters: OIDC wrapper for ID.me/Login.gov            |
|  Future:   Native VC issuance (eIDAS 2.0, June 2026)  |
|  Disclosure: BBS+ cryptosuite for VC selective reveal   |
+-------------------------------------------------------+
```

### Phase 1 (MVP) stack

| Component | Technology | Why |
|-----------|-----------|-----|
| Storage | Postgres with append-only table + hash chain column | Simplest. No new infrastructure. Hash chain provides basic integrity. |
| Signing | `coset` (COSE) for checkpoints | Standard, Rust-native. |
| Key management | Cloud KMS (whichever cloud) | Managed. Per-respondent keys from day one. |
| Identity | OIDC (ID.me, Login.gov) with VC adapter stub | Practical. VC model is the abstraction; OIDC is the current provider. |
| External anchoring | None (or OpenTimestamps if easy to integrate) | Add when needed for government trust story. |
| Selective disclosure | None | Add when BBS+ reaches W3C Recommendation. |

### Phase 2+ enhancements

| Enhancement | Technology | Trigger |
|-------------|-----------|---------|
| Merkle proofs | immudb or custom `ct_merkle` | When ledger size makes full-chain replay expensive, or when auditors demand inclusion proofs |
| External anchoring | OpenTimestamps + optional Rekor | When government trust narrative needs third-party verifiability |
| Selective disclosure | `bbs_plus` + VC BBS cryptosuite | When FOIA or cross-agency sharing requires field-level disclosure |
| Native VCs | Direct VC issuance/verification | When government providers adopt VC (eIDAS 2.0, June 2026) |
| Client-side verification | immudb client SDK or custom | When respondents need to verify their own ledger independently |

---

## Sources

### Immutable Storage
- [immudb - immutable database](https://immudb.io/)
- [immudb GitHub](https://github.com/codenotary/immudb)
- [immudb proof of untampered records](https://immudb.io/blog/proof-of-untampered-records-in-immudb)
- [immudb and Gnark ZKP integration](https://immudb.io/blog/immudb-and-gnark-creating-immutable-and-verifiable-databases-with-zero-knowledge-proof)
- [Implementing Verifiable Data Pipelines Using immudb](https://medium.com/@firmanbrilian/implementing-verifiable-data-pipelines-using-immudb-78c613daad5d)

### Transparency Logs
- [Google Trillian GitHub](https://github.com/google/trillian)
- [Trillian Tessera Alpha Announcement](https://blog.transparency.dev/announcing-the-alpha-release-of-trillian-tessera)
- [Transparency.dev](https://transparency.dev/)
- [Verifiable Data Structures (Trillian)](https://transparency.dev/verifiable-data-structures/)

### BBS+ Signatures
- [W3C Data Integrity BBS Cryptosuites v1.0](https://www.w3.org/TR/vc-di-bbs/)
- [The BBS Signature Scheme (IRTF draft)](https://identity.foundation/bbs-signature/draft-irtf-cfrg-bbs-signatures.html)
- [W3C Invites BBS Implementations (2024)](https://www.w3.org/news/2024/w3c-invites-implementations-of-data-integrity-bbs-cryptosuites-v1-0/)
- [BBS+ at NIST (2023 presentation)](https://csrc.nist.gov/Presentations/2023/crclub-2023-10-18)
- [bbs_plus Rust crate](https://crates.io/crates/bbs_plus)
- [Privacy-preserving BBS+ for Digital Identity (Worldline)](https://blog.worldline.tech/2024/05/14/bbs-plus-credentials.html)

### External Anchoring
- [Sigstore Rekor GitHub](https://github.com/sigstore/rekor)
- [Rekor v2 GA announcement](https://blog.sigstore.dev/rekor-v2-ga/)
- [Sigstore Rekor Tiles](https://github.com/sigstore/rekor-tiles)
- [OpenTimestamps](https://opentimestamps.org/)
- [OpenTimestamps Guide](https://dgi.io/ots/)
- [Standard-Compliant Blockchain Anchoring for Timestamp Tokens](https://www.mdpi.com/2076-3417/15/23/12722)

### Crypto-Shredding
- [How to Set Up Crypto-Shredding for GDPR (2026)](https://oneuptime.com/blog/post/2026-02-17-how-to-set-up-crypto-shredding-for-gdpr-right-to-erasure-compliance-in-google-cloud/view)
- [GDPR-MiFID II Paradox with Crypto-Shredding](https://dev.to/veritaschain/how-i-solved-the-unsolvable-gdpr-mifid-ii-paradox-with-crypto-shredding-2k61)
- [Crypto Shredding for Kafka (Conduktor)](https://www.conduktor.io/glossary/crypto-shredding-for-kafka)
- [Privacy and GDPR in Event-Driven Systems](https://event-driven.io/en/gdpr_in_event_driven_architecture/)
- [Crypto-shredding with Vault and Marten](https://mysticmind.dev/encrypting-and-crypto-shredding-of-pii-data-in-marten-documents-using-hashicorp-vault)
- [GDPR Compliant Event Sourcing with Vault](https://medium.com/sydseter/gdpr-compliant-event-sourcing-with-hashicorp-vault-f27011cac318)

### Verifiable Credentials and Identity
- [W3C publishes VC 2.0 as W3C Standard (May 2025)](https://www.w3.org/press-releases/2025/verifiable-credentials-2-0/)
- [Verifiable Credentials Data Model v2.0](https://www.w3.org/TR/vc-data-model-2.0/)
- [2025 State of Verifiable Credential Report](https://everycred.com/blog/2025-state-of-verifiable-credential-report/)
- [VC Working Group Charter 2026](https://w3c.github.io/vc-charter-2026/)
- [Hyperledger Aries BBS Credentials RFC](https://identity.foundation/aries-rfcs/latest/features/0646-bbs-credentials/)

### COSE and Signing
- [coset Rust crate (Google)](https://crates.io/crates/coset)
- [coset GitHub](https://github.com/google/coset)
- [RFC 9052 - COSE Structures and Process](https://datatracker.ietf.org/doc/rfc9052/)

### Merkle Trees (Rust)
- [ct_merkle - Certificate Transparency Merkle trees](https://docs.rs/ct-merkle/latest/ct_merkle/)
- [rs_merkle - Advanced Merkle tree library](https://github.com/antouhou/rs-merkle)
- [merkle-rs - Inclusion and consistency proofs](https://github.com/shahn/merkle-rs)

### Government Compliance
- [Azure Confidential Ledger Overview](https://learn.microsoft.com/en-us/azure/confidential-ledger/overview)
- [SQL Server 2022 Ledger: Immutable Audit Trails](https://dzone.com/articles/sql-server-ledger-tamper-evident-audit-trails)
- [FedRAMP Ledger - Why Compliance Needs a Security Ledger](https://knoxsystems.com/blog/fedramp-ledger)
- [QLDB Deprecated - Alternatives (DoltHub)](https://www.dolthub.com/blog/2024-08-12-qldb-deprecated-alternatives/)
- [AWS QLDB to Aurora PostgreSQL Migration](https://aws.amazon.com/blogs/database/replace-amazon-qldb-with-amazon-aurora-postgresql-for-audit-use-cases/)

### Event Sourcing (Rust)
- [Event Sourcing CQRS in Rust (2026)](https://oneuptime.com/blog/post/2026-01-25-event-sourcing-cqrs-rust/view)
- [Kurrent (formerly EventStoreDB)](https://www.kurrent.io/)
- [eventually-rs - Event Sourcing for Rust](https://github.com/get-eventually/eventually-rs)
- [Eventus - Rust Event Store](https://tqwewe.com/blog/building-a-rust-powered-event-store/)
