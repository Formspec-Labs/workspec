# Provider-Neutral Signature Port Catalog Specification

**Status:** Ratified  
**Date:** 2026-10-23  
**Domain:** Signature / Governance / Cross-Stack  
**Ticket:** fs-tdzo (T4-SIG-PROVIDER-001)  
**Parent Ticket:** fs-w0li (WOS-T4)  
**Related ADRs:** ADR 0083, ADR 0085, ADR 0086, ADR 0110, ADR 0111  

---

## 1. Overview and Rationale

The Workflow Orchestration Standard (WOS) and Formspec ecosystem require a robust, decoupled, and provider-neutral signature architecture. While WOS and Trellis serve as production reference implementations (prod-MVP), the core architecture MUST NOT hardwire WOS or Trellis assumptions. Third-party signing providers, custom corporate PKI infrastructures, HSM appliances, and lightweight air-gapped verifiers must be able to integrate with narrower claims without breaking invariants.

This specification formalizes the **Nine Provider-Neutral Signature Ports** along with their **Capability Manifests**.

---

## 2. Boundary Clarification and Architectural Constraints

### 2.1 ADR 0110 (Trellis Is Not a Verifier Port Provider)
Per **ADR 0110** (*formspec-trellis-signature-substrate-bridge*), Trellis is **NOT** a `VerifierPort` provider. Trellis consumes detached signatures through substrate primitives (e.g. `integrity-cose::verify_detached_signature`) during admission and export verification. There is no `trellis-formspec-signature` Verifier crate and none will be introduced.

The cryptographic verification providers on the `VerifierPort` axis are:
- **`ring`** (Rust, server-side admission and revalidation)
- **`webcrypto`** (TypeScript/JavaScript, browser-side signing and preview)
- **`python-cryptography`** (future Python tooling and data-science audit verification)

Trellis serves as a reference provider on other port axes: `EvidenceLedgerPort`, `ProofExportPort`, `OfflineVerifierPort`, and `AuditSinkPort`.

### 2.2 ADR 0111 (Receipt Signer Key and Domain Separation)
Receipt generation on `ReceiptIssuerPort` MUST conform to **ADR 0111**:
- Deployment-class signing keys (separate from personal user keys and node signing keys).
- Domain-separated canonical JSON Canonicalization Scheme (JCS / RFC 8785) preimages.
- Detached `COSE_Sign1` container with protected headers declaring algorithm and key identifier.

---

## 3. The Nine Signature Ports

| # | Port Name | Responsibility | Reference Implementations |
|---|-----------|----------------|---------------------------|
| 1 | `SignatureMethodPort` | Cryptographic signing execution and algorithm abstraction | WebCrypto (Browser), PKCS#11 / AWS KMS / GCP Cloud KMS (Server) |
| 2 | `VerifierPort` | Cryptographic signature validation over message digests | Ring (Rust), WebCrypto (TS), Python `cryptography` |
| 3 | `ReceiptIssuerPort` | Issues signed `VerificationReceipt` records upon verified assent | WOS Runtime Companion, Cloudflare Worker / Edge Signer |
| 4 | `AdmissionPort` | Evaluates admission policy, roles, and schema rules against signature acts | WOS Kernel, `trellis-admission-formspec` |
| 5 | `KeyRegistryResolverPort` | Resolves public keys, certificate chains, trust postures, and revocation | DID Resolver, JWKS / UCA Endpoint, X.509 CRL / OCSP |
| 6 | `AuditSinkPort` | Records structured signature lifecycle events into audit streams | WOS Provenance Stream, Syslog, OpenTelemetry |
| 7 | `EvidenceLedgerPort` | Append-only immutable custody of signature evidence and commitments | Trellis Ledger, QLDB / Immutable Log Substrates |
| 8 | `ProofExportPort` | Packages proof bundles, attestations, and certificates for transport | Trellis Export Generator, WOS Export Archive |
| 9 | `OfflineVerifierPort` | Verifies exported proof packages in air-gapped / offline environments | `trellis-verify-wos`, Standalone CLI / Mobile Offline Reader |

---

## 4. Capability Manifest Specification

Each provider publishes a machine-readable `CapabilityManifest` specifying its exact supported features, algorithms, formats, and trust tiers. Consumers check capabilities before dispatching operations.

### 4.1 Schema Fields

```json
{
  "providerId": "string (urn or domain-scoped)",
  "providerVersion": "semver string",
  "portKind": "signature_method | verifier | receipt_issuer | admission | key_registry | audit_sink | evidence_ledger | proof_export | offline_verifier",
  "supportedAlgorithms": ["Ed25519", "ES256", "ES384", "PS256"],
  "supportedFormats": ["COSE_Sign1", "JWS_Compact", "JWS_JSON", "RAW_DER"],
  "features": {
    "offlineCapable": true,
    "hardwareBacked": true,
    "revocationCheckSupported": true,
    "redactionSupported": true,
    "tamperEvidenceTier": "cryptographic_merkle"
  },
  "assuranceFloor": "low | medium | high | eidas_qes | fips_140_3"
}
```

---

## 5. Port Contracts (Normative Trait Definitions)

### 5.1 `SignatureMethodPort`
- `sign(payload: &[u8], key_id: &str) -> Result<SignatureOutput, SignatureError>`
- Emits detached or encapsulated signature bytes conforming to the specified algorithm and envelope.

### 5.2 `VerifierPort`
- `verify(digest: &[u8], signature: &[u8], public_key: &[u8], algorithm: Algorithm) -> Result<VerificationOutcome, VerificationError>`
- Validates mathematical correctness of the signature against the digest without assuming business or workflow context.

### 5.3 `ReceiptIssuerPort`
- `issue_receipt(request: &ReceiptRequest) -> Result<VerificationReceipt, ReceiptError>`
- Generates a cryptographically signed `VerificationReceipt` with timestamp, verifier identity, digest commitment, and outcome.

### 5.4 `AdmissionPort`
- `evaluate_admission(act: &SigningActEvidence, policy: &AdmissionPolicy) -> Result<AdmissionVerdict, AdmissionError>`
- Determines if the signature act meets the state-machine and deontic criteria required for transition firing.

### 5.5 `KeyRegistryResolverPort`
- `resolve_key(key_ref: &str, valid_at: DateTime<Utc>) -> Result<ResolvedKeyRecord, ResolutionError>`
- Returns active public key bytes, key posture (software, HSM, secure enclave), and revocation confirmation.

### 5.6 `AuditSinkPort`
- `record_signature_event(event: &SignatureAuditEvent) -> Result<AuditReceipt, AuditError>`
- Guarantees durable append of signature events before transaction completion.

### 5.7 `EvidenceLedgerPort`
- `append_evidence(evidence: &EvidenceRecord) -> Result<LedgerCommitment, LedgerError>`
- Appends evidence into a verifiable hash-chained or Merkle-tree substrate.

### 5.8 `ProofExportPort`
- `export_proof_package(case_id: &str, opts: &ExportOptions) -> Result<ProofPackage, ExportError>`
- Assembles standalone verifiable proof artifacts including certificates, ledger receipts, and source hashes.

### 5.9 `OfflineVerifierPort`
- `verify_package_offline(package: &[u8], trust_anchors: &[TrustAnchor]) -> Result<ProofVerificationReport, OfflineVerificationError>`
- Performs complete cryptographic and chain-of-custody verification without outbound network connectivity.

---

## 6. Conformance and Compliance

Conforming providers MUST:
1. Export a valid `CapabilityManifest` matching their real behavioral capabilities.
2. Return standard error types without panicking on malformed inputs.
3. Reject algorithm substitution attacks and downgrade attempts.
4. Uphold domain separation for all generated signatures and receipts.
