# Provider-Neutral ProofVerificationReport Specification

**Status:** Ratified  
**Date:** 2026-10-23  
**Domain:** Signature / Governance / Cross-Stack  
**Ticket:** fs-4ath (T4-PROOF-REPORT-001)  
**Parent Ticket:** fs-w0li (WOS-T4)  
**Reference Producer:** Trellis Offline Verifier (`trellis-verify-wos`)  

---

## 1. Context and Architectural Purpose

When governed transactions and multi-party signing workflows conclude, stakeholders require an authoritative, verifiable artifact summarizing the cryptographic validity of all signing acts, custodial handoffs, ledger integrity, and policy compliance. This output contract is the **`ProofVerificationReport`**.

It serves two distinct architectural consumers:
1. **Certificate of Completion (COC) Renderers**: Human-facing document renderers (e.g. Trellis HTML-to-PDF certificate engine) that turn cryptographic assertions into audited, printable closing certificates.
2. **Offline and Air-Gapped Verifiers**: Automated compliance engines, forensic auditing tools, and legal discovery systems that verify self-contained proof packages (`.cbor` / `.zip` bundles) without network dependencies.

Trellis (`trellis-verify-wos`) is the production MVP producer of this report, but the schema and typed Rust contract are strictly provider-neutral to allow alternative verifiers (e.g., enterprise HSM bridges, external qualified trust service providers, or standalone command-line tools) to emit compatible reports.

---

## 2. Canonical JSON Schema Representation

```json
{
  "reportId": "urn:uuid:7b3a9c40-1a6f-4028-98e3-f09bce8a8e31",
  "caseId": "case_fl_2026_0981",
  "generatedAt": "2026-10-23T06:30:00.000Z",
  "producer": {
    "providerId": "urn:trellis:verifier:offline",
    "providerVersion": "2.1.0",
    "engine": "trellis-verify-wos",
    "assuranceTier": "cryptographic_merkle"
  },
  "overallVerdict": "valid",
  "summary": {
    "totalSigningActs": 2,
    "validSigningActs": 2,
    "totalReceipts": 2,
    "validReceipts": 2,
    "ledgerEventsChecked": 48,
    "ledgerContinuityValid": true,
    "hasFatalFindings": false
  },
  "signingActs": [
    {
      "actId": "act_applicant_001",
      "signerIdentity": "did:wos:usr_jane_doe_481",
      "assertedRole": "applicant",
      "algorithm": "Ed25519",
      "keyId": "key_ed_982",
      "signedAt": "2026-10-23T05:45:00.000Z",
      "signatureValid": true,
      "digestMatchesPayload": true,
      "payloadDigest": "sha256:4a3b8c...",
      "intent": "sign_and_attest"
    }
  ],
  "ledgerVerification": {
    "streamId": "case_fl_2026_0981",
    "eventsChecked": 48,
    "merkleRoot": "sha256:9f2c8d...",
    "rootAnchored": true,
    "continuityValid": true
  },
  "custodyReceipts": [
    {
      "receiptId": "rec_001",
      "signerClass": "deployment_receipt_key",
      "coseValid": true,
      "issuedAt": "2026-10-23T05:45:01.000Z"
    }
  ],
  "findings": [
    {
      "code": "advisory_clock_drift_within_tolerance",
      "severity": "advisory",
      "message": "Signer timestamp skewed by 320ms relative to reference clock; within 5s grace.",
      "targetRef": "act_applicant_001"
    }
  ]
}
```

---

## 3. Verdict and Finding Severities

### 3.1 Overall Verdict
- `valid`: All signatures, receipts, ledger continuity, and mandatory policies passed without defect.
- `invalid`: At least one cryptographic signature failed, payload hash mismatched, or ledger continuity was broken.
- `incomplete`: Required evidence records or certificate chain elements are missing from the export bundle.
- `advisory_findings`: Cryptographically valid, but non-blocking warnings or informational notices were logged (e.g., render drift, clock drift).

### 3.2 Finding Severities
- `fatal`: Immediately invalidates the entire case verification (e.g., signature forgery, broken hash chain).
- `error`: Invalidates a specific signing act or receipt, failing verification unless an approved fallback rule applies.
- `warning`: Suspicious metadata or partial verification degraded gracefully.
- `advisory`: Informational note for audit record transparency (e.g., render drift advisory per ADR 0086).

---

## 4. Consumer Obligations

1. **Certificate of Completion (COC) Renderer**:
   - MUST refuse to render a passing certificate if `overallVerdict != valid` and `overallVerdict != advisory_findings`.
   - MUST display all fatal and error findings conspicuously on the audit certificate if configured to generate failure reports.
   - MUST include the `producer.providerId`, `merkleRoot`, and individual signing acts with their exact algorithm and timestamp.

2. **Offline Verifiers**:
   - MUST populate the `ProofVerificationReport` deterministically given identical package bytes and trust anchors.
   - MUST check both `responseHash` and `signedPayload.digest` independently per ADR 0083/0085.
