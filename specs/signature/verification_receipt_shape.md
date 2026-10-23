# VerificationReceipt Production, Embedding Shape, and Failure Taxonomy Specification

**Status:** Ratified  
**Date:** 2026-10-23  
**Domain:** Signature / Governance / Cross-Stack  
**Ticket:** fs-i5j7 (T4-SIG-RECEIPT-SHAPE-001)  
**Parent Ticket:** fs-w0li (WOS-T4)  
**Related ADRs:** ADR 0083, ADR 0085, ADR 0110, ADR 0111  
**Unblocks:** Bundle 007, receipt-001, cert-001  

---

## 1. Executive Summary and Baseline Re-use

This specification settles the production and embedding shape of the **`VerificationReceipt`**, establishing the remaining architecture required to close the Bundle 007 verification gate.

In accordance with **ADR 0111** (*receipt-signer-key-and-wire-shape*), this specification directly inherits and preserves the following accepted architectural decisions:
1. **Deployment-Class Signing Key**: Receipts are signed by an authoritative deployment-class signing key (`deployment_receipt_key`), strictly isolated from personal user signing keys and server node identity keys.
2. **Canonical JCS Preimage**: The receipt claims payload is serialized via the JSON Canonicalization Scheme (JCS, RFC 8785) before hashing, ensuring byte-level determinism across platforms.
3. **Detached COSE_Sign1 Container**: The wire envelope is a detached `COSE_Sign1` object with protected headers declaring algorithm and key identifier.
4. **Signer-Failure Behavior**: If receipt generation fails or times out, the signing ceremony is not failed if the primary signature act verified; instead, an incomplete receipt warning is logged and re-queued.

This specification completes the decisions left open by ADR 0111:
- The exact canonical receipt payload schema.
- Certificate / UCA placement and embedding locations.
- The complete normative Verifier Failure Taxonomy.

---

## 2. Canonical VerificationReceipt Schema

### 2.1 Receipt Wire Format (Detached COSE_Sign1)
A `COSE_Sign1` structure (RFC 9052):
- **Protected Header**:
  - `alg` (label 1): Algorithm ID (e.g. `EdDSA` = -8, `ES256` = -7)
  - `kid` (label 4): Deployment key identifier
  - `typ` (label 16): `"application/vnd.wos.receipt+cbor"`
- **Payload**: Canonical JCS-encoded JSON bytes of `VerificationReceiptPayload`.
- **Signature**: Detached raw signature bytes computed over `Sig_structure`.

### 2.2 Canonical JSON Payload (`VerificationReceiptPayload`)

```json
{
  "$schema": "https://specs.formspec.org/schemas/v1/verification-receipt.schema.json",
  "receiptVersion": "1.0.0",
  "receiptId": "urn:uuid:8e03e41b-417e-4074-9f44-8cbefd0e2e50",
  "issuedAt": "2026-10-23T06:40:00.000Z",
  "issuer": {
    "issuerId": "urn:wos:deployment:us-east-prod",
    "keyClass": "deployment_receipt_key",
    "keyId": "key_deploy_2026_q4"
  },
  "verifiedAct": {
    "actId": "act_applicant_001",
    "signerIdentity": "did:wos:usr_jane_doe_481",
    "assertedRole": "applicant",
    "signatureAlgorithm": "Ed25519",
    "signerKeyId": "key_ed_982",
    "signedAt": "2026-10-23T05:45:00.000Z",
    "payloadDigest": "sha256:4a3b8c7d6e5f...",
    "claimsDigest": "sha256:8b7a6c5d4e3f...",
    "verificationOutcome": "valid"
  },
  "ledgerContext": {
    "streamId": "case_fl_2026_0981",
    "sequenceNumber": 42,
    "entryHash": "sha256:1a2b3c4d5e6f..."
  }
}
```

---

## 3. Certificate and UCA Embedding Location

### 3.1 Certificate of Completion (COC) Embedding
1. **JSON / CBOR Proof Export Bundles**:
   - Receipts are stored in the export root under `064-verification-receipts.cbor`.
   - Each receipt record embeds the detached `COSE_Sign1` container along with its parsed canonical payload.
2. **Human-Facing Rendered Certificates (PDF / HTML)**:
   - Rendered Certificates of Completion include a dedicated normative section titled **"Custodial Verification & Receipt Proofs"**.
   - Each entry displays:
     - Receipt UUID and Issuance Timestamp.
     - Issuer Key Identifier and Deployment Class.
     - SHA-256 Digest of the receipt payload.
     - Direct Universal Certificate Authority (UCA) verification URL and Scannable QR Code.

### 3.2 UCA (Universal Certificate Authority) Locator Seam
- UCA endpoints resolve receipt validity:
  `https://{tenant_domain}/.well-known/uca/receipts/{receipt_id}`
- Response returns the raw detached `COSE_Sign1` bytes and public certificate chain of the deployment receipt key.

---

## 4. Verifier Failure Taxonomy

The table below enumerates all standardized verifier failure codes:

| Failure Code | Severity | Description | Remediable? |
|--------------|----------|-------------|-------------|
| `SIGNATURE_MALFORMED` | Fatal | Signature byte length or ASN.1 / COSE encoding corrupted | No |
| `SIGNATURE_CRYPTOGRAPHICALLY_INVALID` | Fatal | Signature does not mathematically verify against public key and digest | No |
| `DIGEST_PAYLOAD_MISMATCH` | Fatal | Signed payload digest does not equal SHA-256 of actual payload bytes | No |
| `KEY_NOT_FOUND` | Error | Signer key identifier is missing from key registry | Yes (Registry sync) |
| `KEY_EXPIRED` | Error | Signer key was beyond validity period at `signedAt` instant | No |
| `KEY_REVOKED` | Fatal | Signer key has an active revocation record prior to signing instant | No |
| `ALGORITHM_UNSUPPORTED` | Error | Algorithm is disallowed by current tenant security policy | Yes (Policy update) |
| `ASSURANCE_FLOOR_NOT_MET` | Error | Signing act failed required authentication / hardware posture floor | Yes (Step-up auth) |
| `ROLE_MISMATCH` | Error | Signer identity lacks the required workflow role for this transition | Yes (Re-assign) |
| `INTENT_REJECTED` | Error | Declared intent does not match transition requirement | Yes (Re-author) |
| `CLOCK_SKEW_EXCEEDED` | Warning | Signer clock drifted beyond max allowable tolerance (default 5s) | Advisory/Policy |
| `SIGNER_FAILURE` | Warning | Transient fault while signing the receipt itself; act valid, receipt deferred | Yes (Retry) |

---

## 5. Bundle 007 Conformance Gate

To satisfy the Bundle 007 acceptance gate:
1. `verification-receipt.cose` MUST be byte-populated using the canonical JCS payload signed by an Ed25519 or ES256 deployment key.
2. The receipt MUST be referenced from the exported certificate document.
3. Offline verifier test harnesses MUST verify both the primary signature and the verification receipt detached envelope.
