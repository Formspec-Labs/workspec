# WOS-T4 TODO — Signature Profile End-to-End

Working plan for WOS-T4: Signature Profile workflow semantics. This is an end-to-end stack feature, not a WOS-only schema patch.

**Status:** active
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

## T4-0 — Freeze The Acceptance Criteria

- [x] Write exact `WOS-T4 -COMPLETE-` criteria in `TODO.md`.
- [x] Confirm the DocuSign parity bar:
  - [x] ESIGN/UETA consent and intent capture.
  - [x] eIDAS-compatible identity-binding hooks.
  - [x] Sequential signing.
  - [x] Parallel signing.
  - [x] Routed signing via FEL guards.
  - [x] Free-for-all signing.
  - [x] Witness/counter-signature.
  - [x] Notary / in-person signer role.
  - [x] Certified recipient.
  - [x] Approver and viewer roles.
  - [x] Form-filler role.
  - [x] Reminders.
  - [x] Expiry.
  - [x] Decline.
  - [x] Void.
  - [x] Reassign / delegate.
- [x] Explicitly mark out of scope:
  - [x] DocuSign administrative UX.
  - [x] Cryptographic certificate-of-completion composition.
  - [x] Key management.
  - [x] Rendered-document storage.
  - [x] Legal advice or jurisdiction-specific legal sufficiency claims.

---

## T4-1 — Design ADR

- [x] Add `thoughts/adr/0062-signature-profile-workflow-semantics.md`.
- [x] Lock the center/adapter split:
  - [x] WOS center defines signer workflow semantics and evidence record shape.
  - [x] Formspec provides canonical response/signature/consent inputs.
  - [x] Trellis anchors WOS-emitted evidence and builds export bundles.
  - [x] Signature ceremony providers are adapters.
- [x] Decide profile attachment:
  - [x] Signature Profile is a parallel profile document, not a kernel enum widening.
  - [x] Signer roles bind to kernel `human` actors through the `actorExtension` seam.
  - [x] Kernel `ActorDeclaration.type` remains `human | system`.
- [x] Define ADR-0060 naming:
  - [x] `*Ref` only for URI/cross-artifact references.
  - [x] `*Key` only for map/catalog keys.
  - [x] `*Id` only for sibling/local id-bearing objects.
- [x] Record rejected alternatives:
  - [x] Making signer roles new kernel actor types.
  - [x] Treating a drawn signature image as legal intent by itself.
  - [x] Letting Trellis define WOS workflow semantics.
  - [x] Encoding the whole signature ceremony as an opaque vendor extension.

---

## T4-2 — WOS Signature Profile Spec

- [x] Add `specs/profiles/signature.md`.
- [x] Add required sections from `CONVENTIONS.md`:
  - [x] Normative Contract.
  - [x] Composition.
  - [x] Conformance.
- [x] Define conformance classes:
  - [x] Signature Profile Document.
  - [x] Signature Profile Processor.
  - [x] Signature Profile Runtime.
- [x] Define conformance profiles:
  - [x] Core: single/sequential/parallel signing, consent, identity binding, affirmation provenance.
  - [x] Complete: routed/free-for-all flows, reminders, expiry, decline, void, reassignment, witness/notary/certified-recipient.
- [x] Define signer roles:
  - [x] `signer`
  - [x] `in-person-signer`
  - [x] `certified-recipient`
  - [x] `witness`
  - [x] `notary`
  - [x] `approver`
  - [x] `form-filler`
  - [x] `viewer`
- [x] Define signing flow patterns:
  - [x] Sequential.
  - [x] Parallel.
  - [x] Routed via FEL guards.
  - [x] Free-for-all with completion requirements.
- [x] Define lifecycle tags:
  - [x] `awaiting-signature`
  - [x] `signature-complete`
  - [x] `signature-declined`
  - [x] `signature-expired`
  - [x] `signature-voided`
- [x] Define workflow semantics:
  - [x] Sequential flow blocks later signers until prior role completion.
  - [x] Parallel flow allows all pending signers.
  - [x] Routed flow evaluates FEL against case state and profile context.
  - [x] Free-for-all accepts any required signer order but does not complete until requirements are met.
  - [x] Decline emits provenance and follows the configured transition.
  - [x] Void cancels pending signing tasks.
  - [x] Expiry fires a typed timer event.
  - [x] Reassign records original signer, new signer, authorizing actor, and reason.
- [x] Define intent capture:
  - [x] ESIGN/UETA consent step.
  - [x] Consent text/version reference.
  - [x] Affirmation action reference.
  - [x] Explicit statement that image capture alone is not intent.
- [x] Define identity binding:
  - [x] Authentication method.
  - [x] Identity provider reference.
  - [x] Assurance level / strength.
  - [x] Optional external attestation reference.
- [x] Define document binding:
  - [x] Document digest.
  - [x] Digest algorithm.
  - [x] Rendered document reference.
  - [x] Canonical response reference.
- [x] Define `SignatureAffirmation` provenance record shape.
- [x] Define Trellis composition:
  - [x] WOS emits evidence.
  - [x] Trellis anchors evidence.
  - [x] Trellis owns certificate-of-completion/export bundle.
- [x] Define Formspec composition:
  - [x] Formspec captures signature/consent response fields.
  - [x] WOS consumes those fields as evidence inputs.
  - [x] WOS MUST NOT reinterpret invalid Formspec responses as valid signature evidence.

---

## T4-3 — Signature Profile Schema

- [x] Add `schemas/profiles/wos-signature-profile.schema.json`.
- [x] Root marker: `$wosSignatureProfile`.
- [x] Required root fields:
  - [x] `$wosSignatureProfile`
  - [x] `targetWorkflow`
  - [x] `roles`
  - [x] `documents`
  - [x] `signingFlow`
  - [x] `evidence`
- [x] Root optional fields:
  - [x] `$schema`
  - [x] `version`
  - [x] `title`
  - [x] `description`
  - [x] `authenticationPolicies`
  - [x] `reminders`
  - [x] `expiryPolicy`
  - [x] `declinePolicy`
  - [x] `voidPolicy`
  - [x] `reassignmentPolicy`
  - [x] `extensions`
- [x] `$defs.TargetWorkflow`
  - [x] `url`
  - [x] `compatibleVersions`
- [x] `$defs.SignatureRole`
  - [x] `id`
  - [x] `role`
  - [x] `actorId`
  - [x] `required`
  - [x] `authenticationPolicyKey`
  - [x] `description`
- [x] `$defs.SignatureDocument`
  - [x] `id`
  - [x] `documentRef`
  - [x] `documentHash`
  - [x] `documentHashAlgorithm`
  - [x] `renderingRef`
  - [x] `formspecResponseRef`
- [x] `$defs.SigningFlow`
  - [x] `type`
  - [x] `steps`
  - [x] `completion`
- [x] `$defs.SigningStep`
  - [x] `id`
  - [x] `roleId`
  - [x] `documentId`
  - [x] `guard`
  - [x] `dependsOn`
  - [x] `required`
- [x] `$defs.AuthenticationPolicy`
  - [x] `key`
  - [x] `method`
  - [x] `assuranceLevel`
  - [x] `providerRef`
  - [x] `requiresInPerson`
  - [x] `requiresCredentialEvidence`
- [x] `$defs.ConsentReference`
  - [x] `consentTextRef`
  - [x] `consentVersion`
  - [x] `acceptedAtPath`
  - [x] `affirmationPath`
- [x] `$defs.SignatureEvidence`
  - [x] `recordKind`
  - [x] `requiredFields`
  - [x] `consentReference`
  - [x] `identityBinding`
  - [x] `custodyHookEligible`
- [x] `$defs.ReminderPolicy`
- [x] `$defs.ExpiryPolicy`
- [x] `$defs.DeclinePolicy`
- [x] `$defs.VoidPolicy`
- [x] `$defs.ReassignmentPolicy`
- [x] Use closed enums for core roles, flow types, auth methods, and policy actions.
- [x] Permit vendor extensions only via `extensions` or `x-` keys.

---

## T4-4 — Fixtures, Schema Tests, And Generated Types

- [x] Add fixture `fixtures/profiles/signature-benefits-attestation.json`.
- [x] Add fixture `fixtures/profiles/signature-parallel-countersignature.json`.
- [x] Add fixture `fixtures/profiles/signature-routed-notary.json`.
- [x] Add `"$wosSignatureProfile": "profiles/wos-signature-profile.schema.json"` mapping in `tests/schemas/conftest.py`.
- [x] Add `tests/schemas/test_signature_profile_shape.py`.
- [x] Positive schema tests:
  - [x] Minimal valid single signer.
  - [x] Sequential signing.
  - [x] Parallel signing.
  - [x] Routed signing.
  - [x] Witness/counter-signature.
  - [x] Notary/in-person signer.
- [x] Negative schema tests:
  - [x] Missing consent reference rejected.
  - [x] Missing document hash rejected.
  - [x] Invalid signer role rejected.
  - [x] Invalid authentication method rejected.
  - [x] Invalid flow type rejected.
  - [x] Unknown root property rejected.
  - [x] Non-`x-` vendor extension rejected.
- [x] Update `studio/scripts/generate-wos-types.ts` to include `signature-profile`.
- [x] Run `npm run types:gen`.
- [x] Confirm generated `studio/src/types/wos/signature-profile.ts`.
- [x] Confirm `studio/src/types/wos/index.ts` exports it.

---

## T4-5 — Lint Rules

Add T2 lint rules for profile-to-kernel/governance consistency.

- [x] Add registry entries in `crates/wos-lint/src/rules/registry.rs`.
- [x] Implement checks in `crates/wos-lint/src/rules/tier2.rs` or a new profile-specific module.
- [x] Add tests in `crates/wos-lint/tests/tier2_rules.rs` or a new `signature_profile_rules.rs`.
- [x] Proposed rules:
  - [x] `SIG-001`: Signature Profile `targetWorkflow.url` MUST match a loaded kernel URL.
  - [x] `SIG-002`: Every `roles[*].actorId` MUST resolve to a kernel actor.
  - [x] `SIG-003`: Signature roles MUST bind to kernel `human` actors.
  - [x] `SIG-004`: Every `authenticationPolicyKey` MUST resolve to an authentication policy key.
  - [x] `SIG-005`: Every signing step `roleId` MUST resolve to a declared signature role.
  - [x] `SIG-006`: Every signing step `documentId` MUST resolve to a declared signature document.
  - [x] `SIG-007`: Signing step dependencies MUST resolve and MUST NOT cycle.
  - [x] `SIG-008`: Routed signing guards MUST parse as valid FEL.
  - [x] `SIG-009`: Referenced lifecycle tags SHOULD appear in the target kernel.
  - [x] `SIG-010`: Reminder and expiry timers MUST map to typed kernel timer events.
  - [x] `SIG-011`: Required `SignatureAffirmation` evidence inputs MUST be satisfiable from Formspec response, case file, or profile config.
  - [x] `SIG-012`: ADR-0060 naming conventions MUST hold for Signature Profile fields.
- [x] Update `LINT-MATRIX.md`.

---

## T4-6 — WOS Runtime Provenance

**Cross-layer note.** `SignatureAffirmation.identityBinding` is the first concrete shape of the [STACK.md identity attestation open contract](../STACK.md#open-contracts). When it lands, WOS-TODO "Identity attestation shape — generalize beyond signatures" lifts the per-field shape into a reusable `$def` consumed by reviewer-policy assurance refs, amendment-authority attestations (ADR 0066), and review-gate credentials.

- [x] Add `ProvenanceKind::SignatureAffirmation`.
- [x] Add `SignatureAffirmation` schema constraints to `schemas/kernel/wos-provenance-record.schema.json`.
- [x] Add Rust constructor/helper for signature affirmation records.
- [x] Add runtime emission path when a signing task completes and profile requirements are satisfied.
- [x] Required record data:
  - [x] `signerId`
  - [x] `roleId`
  - [x] `role`
  - [x] `documentId`
  - [x] `documentHash`
  - [x] `documentHashAlgorithm`
  - [x] `signedAt`
  - [x] `identityBinding`
  - [x] `consentReference`
  - [x] `signatureProvider`
  - [x] `ceremonyId`
  - [x] `profileRef` or `profileKey` per ADR-0060 semantics.
  - [x] `formspecResponseRef`
  - [x] `custodyHookEligible`
- [x] Add constructor and custody append tests.
- [x] Add runtime emission tests.
- [x] Confirm `SignatureAffirmation` is included in custody append windows.
- [x] Confirm `SignatureAffirmation` has stable serialized `recordKind: "signatureAffirmation"`.

---

## T4-7 — Runtime Signing Semantics

- [x] Sequential flow:
  - [x] Later signing steps remain blocked until prior required steps affirm.
  - [x] Completion emits exactly one `SignatureAffirmation` per signer/document pair.
- [x] Parallel flow:
  - [x] Independent signing steps may complete in any order.
  - [x] Flow completes only when required steps affirm.
- [x] Routed flow:
  - [x] FEL guards select applicable signing steps.
  - [x] Non-selected steps do not block completion.
- [x] Free-for-all flow:
  - [x] Required signer set may complete in any order.
  - [x] Optional viewers/recipients do not block completion unless configured.
- [x] Decline:
  - [x] Decline records signer, reason, timestamp, and document.
  - [x] Decline follows configured lifecycle transition.
- [x] Void:
  - [x] Void cancels pending signing steps.
  - [x] Void records authorizing actor and reason.
- [x] Expiry:
  - [x] Expiry timer fires typed timer event.
  - [x] Expiry records pending signers and expired documents.
- [x] Reassignment:
  - [x] Reassignment records original signer, new signer, authorizing actor, and reason.
  - [x] Reassignment does not erase accountability for the original assignment.
- [x] Witness/counter-signature:
  - [x] Witness step depends on primary signer affirmation.
  - [x] Counter-signature binds to the same document hash or a declared post-signature hash.
- [x] Notary / in-person signer:
  - [x] Requires stronger authentication policy.
  - [x] Emits identity-binding evidence with in-person method.

---

## T4-8 — Conformance Fixtures

Add executable WOS conformance fixtures.

- [x] `SIG-001-sequential-single-signer.json`
  - [x] Sequential single signer emits `signatureAffirmation`.
- [x] `SIG-002-parallel-signers-any-order.json`
  - [x] Parallel signers complete in either order.
- [x] `SIG-003-routed-signer-fel-guard.json`
  - [x] Routed signer path selected by FEL guard.
- [x] `SIG-004-expiry-timer.json`
  - [x] Expiry timer transitions to expired and records evidence.
- [x] `SIG-005-decline-path.json`
  - [x] Decline emits decline evidence and follows configured transition.
- [x] `SIG-006-reassignment-accountability.json`
  - [x] Reassignment keeps original/new signer accountability chain.
- [x] `SIG-007-witness-countersignature.json`
  - [x] Witness/counter-signature dependency enforced.
- [x] `SIG-008-notary-in-person-auth.json`
  - [x] Notary/in-person signer requires stronger auth policy.
- [x] `SIG-009-missing-consent-blocks-affirmation.json`
  - [x] Missing consent blocks `SignatureAffirmation`.
- [x] `SIG-010-custody-append-window.json`
  - [x] `SignatureAffirmation` appears as a custody append input.
- [x] `SIG-011-free-for-all-signers-any-order.json`
  - [x] Free-for-all signers complete in any order and wait for all required signatures.
- [x] `SIG-012-void-path.json`
  - [x] Authorized void cancels pending signature tasks and preserves reason evidence.
- [x] Add fixture loader support if Signature Profile documents need a new companion classification.
- [x] Add golden trace updates if required.

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

WOS-side verification:

- [x] `cargo fmt --all`
- [x] `cargo check --workspace`
- [x] `cargo test -p wos-lint`
- [x] `cargo test -p wos-runtime --lib`
- [x] `cargo test -p wos-conformance --test signature_profile -- --nocapture`
- [x] `../.venv/bin/pytest tests/schemas -q`
- [x] `npm run types:check` in `studio/`
- [x] `git diff --check`

Cross-repo verification:

- [ ] Formspec signed-response fixture passes server revalidation.
- [ ] Trellis custody vector accepts WOS `SignatureAffirmation`.
- [ ] Trellis export/certificate fixture includes WOS signing evidence.
- [ ] Studio can author and validate at least one sequential and one parallel signature profile.

Bookkeeping:

- [x] Update `TODO.md`.
- [ ] Update `COMPLETED.md`.
- [x] Update `WOS-IMPLEMENTATION-STATUS.md`.
- [x] Update `LINT-MATRIX.md`.
- [x] Update `MD-INVENTORY.md` if new docs are added.
- [ ] Mark WOS-T4 as `-COMPLETE-` only after all WOS-side and cross-repo gates pass.

---

## Proposed Execution Order

1. T4-0 acceptance criteria.
2. T4-1 design ADR.
3. T4-2 spec.
4. T4-3 schema.
5. T4-4 schema fixtures and generated types.
6. T4-5 lint.
7. T4-6 provenance record.
8. T4-7 runtime semantics.
9. T4-8 conformance fixtures.
10. T4-9 Formspec alignment.
11. T4-10 Trellis alignment.
12. T4-11 Studio support.
13. T4-12 verification and closeout.
