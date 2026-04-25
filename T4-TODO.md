# WOS-T4 TODO — Signature Profile End-to-End

Active closeout plan for the remaining cross-repo work on WOS-T4. The WOS-side Signature Profile spec/schema/lint/runtime/conformance slice landed on 2026-04-22 and is archived in [COMPLETED.md](COMPLETED.md).

**Status:** active — cross-repo closeout (Trellis machine-verifiable slice landed 2026-04-22)
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

**Status:** landed 2026-04-22 in the parent Formspec repo.

- [x] Identify existing Signature component response shape.
- [x] Define canonical signature evidence fields consumed by WOS via response-level `authoredSignatures`.
- [x] Include signing-evidence fields needed for WOS/Trellis proof:
  - [x] `documentId`.
  - [x] `signatureValue` or attachment reference.
  - [x] `signatureMethod`.
  - [x] `signerId`.
  - [x] `signerName`.
  - [x] `signedAt`.
  - [x] `consentAccepted`.
  - [x] `consentTextRef`.
  - [x] `consentVersion`.
  - [x] `affirmationText`.
  - [x] `documentHash`.
  - [x] `documentHashAlgorithm`.
  - [x] `responseId`.
  - [x] `identityProofRef`.
  - [x] `identityBinding`.
  - [x] `signatureProvider`.
  - [x] `ceremonyId`.
- [x] Ensure server-side revalidation preserves these fields.
- [x] Add Formspec fixture for a signed response.
- [x] Add WOS-facing mapping example from canonical Formspec response to `SignatureAffirmation`.
- [x] State explicitly: WOS MUST NOT infer legal intent from a drawn signature image alone.

---

## T4-10 — Trellis Alignment

Work in Trellis after WOS `SignatureAffirmation` shape is stable.

**Status:** machine-verifiable export path **landed 2026-04-22** in the Trellis submodule; human-facing certificate-of-completion composition remains.

- [x] Confirm Trellis accepts WOS `SignatureAffirmation` through `custodyHook` (vector `append/019-wos-signature-affirmation`; `trellis-conformance` replays append).
- [x] Define idempotency tuple for signing records (ADR-0061 `(caseId, recordId)` tuple pinned in `019`; domain-separated key in append vector).
- [x] Add Trellis vector for WOS signature affirmation append (`append/019`; generator `fixtures/vectors/_generator/gen_append_019.py`).
- [x] Ensure export bundle includes:
  - [x] WOS `SignatureAffirmation` (as the signed event’s readable payload in `010-events.cbor`; catalog summarizes the same bytes).
  - [x] Document digest (`document_hash` / algorithm in `062-signature-affirmations.cbor` row).
  - [x] Consent reference (catalog row `consent_reference`).
  - [x] Identity-binding reference (catalog row `identity_binding`).
  - [x] Formspec response reference (catalog row `formspec_response_ref`).
  - [x] Anchor proof (export spine: checkpoints, inclusion proofs, manifest digests; optional external anchor unchanged).
- [ ] Define certificate-of-completion composition as Trellis-owned (renderer-facing bundle / UX; export catalog is the verifier-facing substrate).
- [x] Cross-link Trellis spec to WOS Signature Profile (`trellis/specs/trellis-core.md` §6.7 / §18 / §19 — `trellis.export.signature-affirmations.v1`, `062-signature-affirmations.cbor`, verifier obligations).

---

## T4-11 — Studio Support

- [x] Keep `npm run types:gen` output TypeScript-clean: post-process strips json-schema-to-typescript’s `patternProperties: { "^x-": ... }` vendor index signatures (without breaking nested maps such as `VerifiableConstraint.finiteDomainDeclarations`); generated `types/wos/index.ts` namespaces every module except `kernel` so inlined `ExtensionsMap` / `JsonSchemaUri` do not TS2308 across star exports; `types:check` compares post-processed output.
- [x] Add generated type consumption for Signature Profile (`WOSSignatureProfileDocument` types generated via `npm run types:gen`; consumed by `ISignatureProfilePort`, `FixtureSignatureProfilePort`, `HttpSignatureProfilePort`, wired into `WosContext` via `useSignatureProfile()` hook).
- [x] Add port/adapter layer for Signature Profile (`ISignatureProfilePort` interface in `WosPorts.ts`; `FixtureSignatureProfilePort` with structural validation + cross-reference checks; `HttpSignatureProfilePort` delegating to `/api/lint/document`; `GET /instances/{id}/signature-affirmations` endpoint in `wos-server`).
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
- [x] Add fixture-backed Studio tests for a sequential signing profile (12 tests in `SignatureProfileAdapter.test.ts`; load/validate/policy-section checks/step-resolution/save round-trip).
- [x] Add fixture-backed Studio tests for a parallel signing profile (covered in same suite; distinct-role validation, structural soundness).

---

## T4-12 — Verification Gate

Cross-repo verification:

- [x] Formspec signed-response fixture passes server revalidation.
- [x] Trellis custody vector accepts WOS `SignatureAffirmation` (`append/019`; conformance harness).
- [x] Trellis export fixture includes WOS signing evidence (`export/006-signature-affirmations-inline` + `verify/014` + `tamper/014`; `trellis-verify` checks catalog digest and row↔payload agreement). **Open:** same bytes/URLs as the parent Formspec fixture in one committed cross-repo bundle (see stack [`TODO.md`](../TODO.md)).
- [ ] Studio can author and validate at least one sequential and one parallel signature profile.

Bookkeeping:

- [ ] Update Trellis `COMPLETED.md` / WOS `COMPLETED.md` when the remaining cross-repo gates land (optional: short entry for the 2026-04-22 Trellis slice).
- [ ] Mark WOS-T4 as `-COMPLETE-` only after all WOS-side and cross-repo gates pass.

---

## Proposed Execution Order

1. ~~T4-10 Trellis alignment~~ — machine-verifiable slice done; finish COC presentation + optional fixture re-seeding.
2. T4-11 Studio support.
3. T4-12 verification and closeout (shared bundle + Studio).
