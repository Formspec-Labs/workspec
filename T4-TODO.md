# WOS-T4 TODO — Signature Profile End-to-End

Active closeout plan for the remaining cross-repo work on WOS-T4. The WOS-side Signature Profile spec/schema/lint/runtime/conformance slice landed on 2026-04-22 and is archived in [COMPLETED.md](COMPLETED.md).

**Status:** active — cross-repo closeout (Trellis machine-verifiable slice landed 2026-04-22; unchanged through Trellis Wave 16 store-postgres production hardening, Wave 17 HPKE/`KeyEntry` corpus, Wave 18 HPKE crate hardening + reason-code parity lint + store-postgres review follow-ups + ADR 0005 Stage 1 spec deltas, Wave 19 AEAD nonce determinism Core §9.4 + §17 amendment per parent **PLN-0383** [signature-stack-relevant — silent retry-determinism class on signed events], Wave 20 ADR 0008 interop sidecar reservation Phase-1 lock-off, and **Wave 21 pending close** [ADR 0005 Stages 2-5 working tree complete — Rust verifier + Python parity + positive vectors `append/023..027` committed; tamper `017..019` + export bundle + CLI + §27 in working tree; closes parent **PLN-0312** entirely once committed]). Trellis cert-of-completion at [`trellis/TODO.md`](../trellis/TODO.md) item #4 remains the integrity-artifact gate for **T4-10** rendering + parent **PLN-0355** ESIGN/UETA Trigger. Open gates as of 2026-04-28: Studio authoring/validation UI (T4-11); Trellis human-facing certificate-of-completion **rendering** (T4-10) — note: spec composition shape is closed by [Trellis ADR 0007](../trellis/thoughts/adr/0007-certificate-of-completion-composition.md) (Accepted 2026-04-24); remaining work is renderer/template/UX, not new byte format; shared cross-repo fixture bundle (T4-12 + parent design at [`../thoughts/specs/2026-04-24-shared-cross-seam-fixture-bundle-design.md`](../thoughts/specs/2026-04-24-shared-cross-seam-fixture-bundle-design.md)).

**Scope reframe 2026-04-27 (VISION §X DocuSign 100% parity bar).** T4 remains the workflow-tier slice (signing semantics + `SignatureAffirmation` provenance + machine-verifiable export). Parent stack closure cluster pulls administrative surface back into 1.0 scope under separate handles — T4 does not expand. Path back to original DocuSign-100% framing: **PLN-0380** (`signature.md` §1.3 scope reopen + signing-intent URI registry + signer-authority claim shape), **PLN-0398** (Trigger — administrative surface: templates, bulk-send, dashboards, signer status views, reminder cadence configuration, audit history view), **PLN-0379** (Trellis ADR 0010 user-content Attestation primitive — composes for byte-level intent URI carriage). T4 closes when the workflow slice is verified end-to-end; admin-surface parity is its own follow-on.
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

**T4 scope** is the workflow-tier slice of the parity bar: ESIGN/UETA/eIDAS-aligned intent capture, identity binding, signing order, reminders, expiry, decline, reassignment, witness/counter-signature, and notary/in-person signing semantics. The full parity bar per VISION §X (post-synthesis-merge 2026-04-27) also pulls administrative surface into 1.0 scope (templates, bulk-send, dashboards, signer status views, reminder cadence, audit history view) — that work lands as parent **PLN-0398** (Trigger) plus **PLN-0380** spec extension (signing-intent URI registry, signer-authority claim shape, §1.3 scope reopen for ESIGN/UETA/eIDAS posture mapping). T4 itself does not expand to admin-surface scope; T4 closes when the workflow-tier slice is verified end-to-end.

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

**Status:** machine-verifiable export path **landed 2026-04-22** in the Trellis submodule; certificate-of-completion **byte composition shape** closed by [Trellis ADR 0007](../trellis/thoughts/adr/0007-certificate-of-completion-composition.md) (Accepted 2026-04-24 — `trellis.certificate-of-completion.v1`, export catalog at `065-certificates-of-completion.cbor`, verifier obligations in Trellis Core §19); remaining T4-10 work is renderer/template/UX (HTML-to-PDF reference, signing-bundle visualization), not new spec authoring.

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
- [x] Define certificate-of-completion **byte composition shape** as Trellis-owned (closed by Trellis ADR 0007, Accepted 2026-04-24 — `trellis.certificate-of-completion.v1` + export catalog row + verifier obligations).
- [ ] **Renderer / template / UX:** reference HTML-to-PDF rendering of the manifest, signing-bundle visualization (`trellis-interop-c2pa` adapter at `trellis/TODO.md` item #21 layers C2PA assertion onto the presentation PDF — co-lands with this work).
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

## Vendor `x-*` assurance floor enforcement (deferred-strict-mode)

`crates/wos-runtime/src/runtime/signature.rs::identity_binding_meets_policy` currently fail-opens when either `identityBinding.assuranceLevel` or `policy.assuranceLevel` is an `x-*` token: the runtime cannot ordinal-compare vendor tokens against IAL/AAL/ranked tiers, so it admits the binding. Spec [`profiles/signature.md` §2.13](specs/profiles/signature.md) declares posture × intent-URI floors as normative; admission of `x-*` assurance MUST be gated on a Posture Declaration registry that names the vendor token and its floor mapping. **Cross-link:** ADR 0083 D4 (migrate preconditions) shares the same Posture Declaration / PLN-0384 seam — posture floors are one registry-shaped dependency for both vendor assurance and migration gates.

**Why:** zero-trust runtime posture (root [`VISION.md`](../VISION.md)) — fail-open on unknown vendor tokens lets unverified deployments admit `SignatureAffirmation` without a declared floor. Today there are no users, so the gap is cheap to fix; it gets expensive the moment a deployment ships an `x-acme-tier-2` policy with no floor table behind it.

**Gating:** parent **PLN-0384** (`wos-event-types.md` ratification) closes the namespace seam; the Posture Declaration registry shape lands alongside §2.13 floor tables. New ADR slot: "Vendor `x-*` assurance floor: fail-closed default + Posture Declaration registry binding."

**Done when:**

- [ ] ADR authored proposing fail-closed default with explicit Posture Declaration opt-in.
- [ ] Spec §2.13 amends with the vendor-floor declaration shape.
- [ ] Runtime flips `identity_binding_meets_policy` to reject `x-*` tokens absent a Posture Declaration entry; existing pass-through tests (`identity_binding_meets_policy_vendor_*_skips_ordinal_*`) retire or invert.
- [ ] SIG-* conformance fixture exercises a declared `x-*` floor and a missing-declaration rejection.

---

## Proposed Execution Order

1. ~~T4-10 Trellis alignment~~ — machine-verifiable slice done; finish COC presentation + optional fixture re-seeding.
2. T4-11 Studio support.
3. T4-12 verification and closeout (shared bundle + Studio).
