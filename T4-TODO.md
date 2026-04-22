# WOS-T4 TODO — Signature Profile End-to-End

Active closeout plan for the remaining cross-repo work on WOS-T4. The WOS-side Signature Profile spec/schema/lint/runtime/conformance slice landed on 2026-04-22 and is archived in [COMPLETED.md](COMPLETED.md).

**Status:** active — cross-repo closeout
**Owner:** WOS leads  
**Stack boundary:** Formspec captures signing evidence, WOS governs the signing workflow and emits `SignatureAffirmation`, Trellis anchors and exports the evidence bundle.

---

## Completion Contract

WOS-T4 is complete only when a signature workflow runs end to end:

1. Formspec captures a signing/consent response.
2. WOS routes the signer ceremony through lifecycle and governance semantics.
3. WOS emits a `SignatureAffirmation` provenance record.
4. Trellis accepts the record through `custodyHook` and anchors it.
5. Conformance proves the behavior across the common signing patterns.

The parity bar is **DocuSign common-case workflow semantics**, not DocuSign administrative UX. T4 targets ESIGN/UETA/eIDAS-aligned intent capture, identity binding, signing order, reminders, expiry, decline, reassignment, witness/counter-signature, and notary/in-person signing semantics.

---

## Ownership

| Layer | Owns |
|---|---|
| WOS | Signature workflow semantics, signer roles, flow patterns, signer-authentication policy, `SignatureAffirmation` provenance, lint, runtime, conformance |
| Formspec | Signature/consent capture controls and canonical response fields consumed by WOS |
| Trellis | Custody anchoring, certificate-of-completion/export bundle, offline verification |
| Studio | Authoring and validation UX across the WOS/Formspec/Trellis surfaces |

WOS MUST NOT own cryptographic certificate assembly, key custody, rendered-document storage, or final export-bundle composition. Those are Trellis or adapter responsibilities. WOS owns the semantic evidence record that says what signing act occurred.

---

## WOS-Side Closeout

Archived in [COMPLETED.md](COMPLETED.md):

- ADR-0062 design freeze and center-vs-adapter split.
- Signature Profile spec and schema.
- Schema fixtures/tests and generated Studio type bindings.
- SIG-001..SIG-012 lint rules and matrix entries.
- `ProvenanceKind::SignatureAffirmation` plus schema-constrained payload.
- Runtime signing semantics for sequential, parallel, routed, free-for-all, decline, void, expiry, reassignment, witness/counter-signature, and notary/in-person flows.
- 13 SIG-* conformance fixtures and WOS-side verification gates.

WOS-side verification is green. Remaining work is cross-repo proof and authoring support.

---

## T4-9 — Formspec Alignment

Work in the parent Formspec project after the WOS center is stable.

- [ ] Identify existing Signature component response shape.
- [ ] Define canonical signature evidence fields consumed by WOS:
  - [ ] `signatureValue` or attachment reference.
  - [ ] `signatureMethod`.
  - [ ] `signerName`.
  - [ ] `signedAt`.
  - [ ] `consentAccepted`.
  - [ ] `consentTextRef`.
  - [ ] `consentVersion`.
  - [ ] `affirmationText`.
  - [ ] `documentHash`.
  - [ ] `documentHashAlgorithm`.
  - [ ] `responseId`.
  - [ ] `identityProofRef`.
  - [ ] `signatureProvider`.
  - [ ] `ceremonyId`.
- [ ] Ensure server-side revalidation preserves these fields.
- [ ] Add Formspec fixture for a signed response.
- [ ] Add WOS-facing mapping example from canonical Formspec response to `SignatureAffirmation`.
- [ ] State explicitly: WOS MUST NOT infer legal intent from a drawn signature image alone.

---

## T4-10 — Trellis Alignment

Work in Trellis after WOS `SignatureAffirmation` shape is stable.

- [ ] Confirm Trellis accepts WOS `SignatureAffirmation` through `custodyHook`.
- [ ] Define idempotency tuple for signing records.
- [ ] Add Trellis vector for WOS signature affirmation append.
- [ ] Ensure export bundle includes:
  - [ ] WOS `SignatureAffirmation`.
  - [ ] Document digest.
  - [ ] Consent reference.
  - [ ] Identity-binding reference.
  - [ ] Formspec response reference.
  - [ ] Anchor proof.
- [ ] Define certificate-of-completion composition as Trellis-owned.
- [ ] Cross-link Trellis ADR/spec to WOS Signature Profile.

---

## T4-11 — Studio Support

- [ ] Add generated type consumption for Signature Profile.
- [ ] Add authoring UI for:
  - [ ] Signer roles.
  - [ ] Signing order.
  - [ ] Routed guards.
  - [ ] Witness/notary requirements.
  - [ ] Authentication policies.
  - [ ] Reminders.
  - [ ] Expiry.
  - [ ] Decline.
  - [ ] Void.
  - [ ] Reassignment.
  - [ ] Linked Formspec signature/consent fields.
- [ ] Add validation UI for lint results.
- [ ] Add fixture-backed Studio tests for a sequential signing profile.
- [ ] Add fixture-backed Studio tests for a parallel signing profile.

---

## T4-12 — Verification Gate

Cross-repo verification:

- [ ] Formspec signed-response fixture passes server revalidation.
- [ ] Trellis custody vector accepts WOS `SignatureAffirmation`.
- [ ] Trellis export/certificate fixture includes WOS signing evidence.
- [ ] Studio can author and validate at least one sequential and one parallel signature profile.

Bookkeeping:

- [ ] Update `COMPLETED.md` when the remaining cross-repo gates land.
- [ ] Mark WOS-T4 as `-COMPLETE-` only after all WOS-side and cross-repo gates pass.

---

## Proposed Execution Order

1. T4-9 Formspec alignment.
2. T4-10 Trellis alignment.
3. T4-11 Studio support.
4. T4-12 verification and closeout.
