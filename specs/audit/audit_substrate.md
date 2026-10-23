# WOS Audit Substrate Specification: WosAuditRecord, EvidenceRecord, and AuditSubstratePort

**Status:** Ratified  
**Date:** 2026-10-23  
**Domain:** Governance / Audit / Cross-Stack  
**Ticket:** fs-n2xr (T4-AUDIT-SUBSTRATE-001)  
**Parent Ticket:** fs-w0li (WOS-T4)  
**Reference Substrate:** Trellis (prod-MVP)  

---

## 1. Executive Summary and Scope

Workflow Orchestration Standard (WOS) instances emit high-assurance governance and state transition events that require durable, tamper-evident, and verifiable audit logging. While Trellis provides the production MVP substrate with Merkle-tree event chaining, CBOR export packaging, and cryptographic receipts, WOS must decouple its core audit requirements so that alternative enterprise audit backends (e.g., AWS CloudTrail Lake, QLDB, Splunk Enterprise Security, Google Cloud Immutable Storage, Hyperledger Fabric) can implement valid audit logging with narrower claims.

This specification formalizes:
1. **`WosAuditRecord`**: The canonical envelope for governance and workflow lifecycle events.
2. **`EvidenceRecord`**: Portable cryptographic commitment and evidence attachments linked to audit records.
3. **`AuditSubstratePort`**: The trait defining the dependency-injection boundary for append, recovery, export, and verification operations.
4. **Lifecycle States, Stream Ordering, and Idempotency**: Strict ordering and replay guarantees across case instances.
5. **Substrate Capability Matrix**: Standardized feature declaration allowing backends to assert their trust tier.
6. **Privacy and Redaction Rules**: Field-level masking, cryptographic salting, and erasure compliance (GDPR/CCPA) without invalidating Merkle stream continuity.

---

## 2. Canonical Record Shapes

### 2.1 `WosAuditRecord`
The primary atomic unit of audited activity within WOS.

```json
{
  "recordId": "urn:uuid:f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "streamId": "case_12345",
  "sequenceNumber": 42,
  "eventType": "wos.kernel.transition_fired",
  "occurredAt": "2026-10-23T06:00:00.000Z",
  "tenantId": "tenant_alpha",
  "actor": {
    "actorId": "usr_9981",
    "actorKind": "human_authorized",
    "role": "claims_adjudicator"
  },
  "payloadDigest": "sha256:7f83b1657ff1fc53b92dc18148a1d65dfc2d4b1fa3d677284addd200126d9069",
  "previousRecordHash": "sha256:4b227777d4dd1fc61c6f884f48641d02b4d121d3fd328cb08b5531fcacdabf8a",
  "evidenceRefs": [
    "urn:formspec:evidence:sig_act_001",
    "urn:wos:receipt:rec_8834"
  ],
  "lifecycleState": "committed",
  "redactionState": "unredacted"
}
```

### 2.2 `EvidenceRecord`
Encapsulates cryptographic proofs, signatures, or third-party attestations supporting an audit record.

```json
{
  "evidenceId": "urn:formspec:evidence:sig_act_001",
  "recordId": "urn:uuid:f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "evidenceKind": "detached_cose_signature",
  "contentType": "application/cose; cose-type=cose-sign1",
  "digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "rawBytesRef": "s3://wos-evidence-vault/tenant_alpha/case_12345/sig_001.cose",
  "capturedAt": "2026-10-23T05:59:58.120Z",
  "verificationStatus": "verified_valid"
}
```

---

## 3. `AuditSubstratePort` Contract

The port defines five core lifecycle methods:

```rust
#[async_trait]
pub trait AuditSubstratePort: Send + Sync {
    /// Declare provider capabilities and supported assurance tier.
    fn capability_matrix(&self) -> &SubstrateCapabilityMatrix;

    /// Durably append an audit record and linked evidence to the case stream.
    /// MUST be idempotent given the same (stream_id, sequence_number, record_id).
    async fn append_record(
        &self,
        record: &WosAuditRecord,
        evidence: &[EvidenceRecord],
    ) -> Result<SubstrateAppendReceipt, AuditSubstrateError>;

    /// Read records from a stream with sequence bounds.
    async fn read_stream(
        &self,
        stream_id: &str,
        from_seq: u64,
        max_records: usize,
    ) -> Result<Vec<WosAuditRecord>, AuditSubstrateError>;

    /// Retrieve an individual evidence record by evidence ID.
    async fn get_evidence(
        &self,
        evidence_id: &str,
    ) -> Result<Option<EvidenceRecord>, AuditSubstrateError>;

    /// Apply a compliant redaction or erasure to an audit record while preserving
    /// the cryptographic stream digest and parent link continuity.
    async fn redact_record(
        &self,
        stream_id: &str,
        record_id: &str,
        redaction_request: &RedactionRequest,
    ) -> Result<RedactionReceipt, AuditSubstrateError>;
}
```

---

## 4. Lifecycle States, Per-Stream Ordering, and Idempotency

### 4.1 Lifecycle States
1. `Pending`: Formulated by WOS runtime, awaiting durable write confirmation.
2. `Committed`: Durably written to substrate storage, hash linked to previous stream record.
3. `Attested`: Incorporated into a Merkle root or external custody receipt (e.g. Trellis block or notary anchor).
4. `Redacted`: Modified under formal privacy rules with tombstone and zero-knowledge or salted hash preservation.

### 4.2 Per-Stream Ordering
- Every `streamId` (corresponding to a WOS Case File or Case Instance) maintains a strictly increasing, contiguous `sequenceNumber` (0, 1, 2, ...).
- Substrates MUST refuse out-of-order sequence insertion. If `seq = 5` is submitted when `head = 3`, the append MUST return `AuditSubstrateError::SequenceGap`.
- Each record binds the `previousRecordHash` of sequence `N-1`. Sequence 0 binds the canonical Genesis Hash `sha256:0000...0000`.

### 4.3 Idempotency and Replay Semantics
- Network retries and activity replays submit the exact same `(stream_id, sequence_number, record_id, payload_digest)`.
- If an append matches an existing committed record in all identifiers and digests, the substrate MUST return `Ok(receipt)` with `already_existed: true` without creating duplicate records or modifying stream state.
- If sequence numbers match but payload digests disagree, the substrate MUST return `AuditSubstrateError::ConflictingRecordId`.

---

## 5. Substrate Capability Matrix

Backends register a `SubstrateCapabilityMatrix`:

| Capability | Description | Trellis (Reference) | Basic Cloud SQL / Log Sink |
|------------|-------------|---------------------|----------------------------|
| `tamper_evidence` | Cryptographic Merkle tree or hash-chain integrity verification | `MerkleTreeTreeHash` | `HashChainOnly` / `AppendOnlyStorage` |
| `offline_verifiable` | Can be verified disconnected using proof exports and trust anchors | `true` | `false` |
| `granular_redaction` | Can redact specific fields without invalidating cryptographic verification | `true` | `false` (record tombstone only) |
| `hardware_anchoring` | External notary anchoring (KMS, Bitcoin, TSA, transparency log) | `true` | `optional` |
| `max_throughput_iops` | Upper bounded batch ingestion rate | `10_000/s` | `1_000/s` |

---

## 6. Privacy and Redaction Rules

1. **Hash Invariance**: Redaction MUST NOT alter `previousRecordHash` or break the stream chain. The original record hash is replaced by a redaction commitment: `H_redacted = SHA256(original_digest || redaction_salt || tombstone_reason)`.
2. **Access Control**: Redacted fields are purged from plain text storage. Only authorized forensic auditors with the redaction salt can verify pre-image correspondence.
3. **Evidence Disposal**: For "right to be forgotten" requests, evidence raw bytes are securely shredded while retaining the cryptographic hash commitment in the ledger.
