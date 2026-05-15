---
title: WOS Signature Profile
version: 1.0.0-draft.3
date: 2026-05-15
status: draft
---

> **Partial absorption (ADR 0076 D-2).** Signature workflow semantics absorbed into the merged `schemas/wos-workflow.schema.json` `signature` embedded block — signature is no longer a separate sidecar but a load-bearing block on any DocuSign-tier workflow. Prose normative content remains here (signing flow patterns, role binding, authentication policies) until the absorption pass lands the within-block descriptions. SIG-001..SIG-012 lint rules continue to cite this document as canonical until the schema descriptions become full normative.


# WOS Signature Profile v1.0

**Version:** 1.0.0-draft.3
**Date:** 2026-05-15
**Editors:** Formspec Working Group
**Companion to:** WOS Kernel Specification v1.0

---

## Abstract

The WOS Signature Profile defines workflow semantics for signature ceremonies in WOS. It covers signer roles, signing order, routed signing, free-for-all signing, envelope and recipient state, witness and notary participation, reminders, expiry, decline, void, reassignment, intent capture, identity binding, document binding, and the `SignatureAffirmation` provenance record emitted when a signing act is accepted.

The profile is a parallel seam. It does not add kernel actor types and does not define cryptographic certificate-of-completion bundles. Formspec captures signing and consent evidence. WOS governs the workflow semantics and emits semantic evidence. Trellis anchors that evidence and owns export-bundle composition.

---

## Status of This Document

This document is a **draft specification**. Implementors MUST NOT treat it as stable until WOS v1.0 is ratified.

### Revision history

| Version | Date | Change |
|---|---|---|
| 1.0.0-draft.1 | 2026-04-22 | Initial draft. |
| 1.0.0-draft.2 | 2026-04-28 | **§1.3 scope reopen (PLN-0380).** ESIGN / UETA / eIDAS posture mapping moved from out-of-scope to in-scope. Added §2.13 signing-intent URI registry, §2.14 signer-authority claim, §2.15 jurisdictional posture mapping. §2.8 binds to Trellis ADR 0010 `UserContentAttestationPayload` as the byte-level proof. §3.3 names the layered-verifier composition contract with Trellis. Counsel-pinned legal-sufficiency claims remain gated on PLN-0355. |
| 1.0.0-draft.3 | 2026-05-15 | Adds the K-3 envelope/recipient state model, aligns §2.8 with the K-2 `SignatureAffirmation` fields (`signingActId`, `documentRef`, `presentationHash`, `witnessedSignatureRef`), and clarifies §2.13.1 as the baseline WOS signing-intent registry. |

---

## 1. Introduction

### 1.1 Background

Rights-impacting workflows frequently require signatures: benefit attestations, consent forms, releases, certifications, delegated authorizations, witness statements, and notarial acts. A WOS processor can already model lifecycle states and actor tasks, but common signature behavior needs a portable semantic contract so workflows do not depend on a single signing provider.

### 1.2 Design Goals

1. **Workflow semantics, not ceremony vendor lock-in.** A signature provider is an adapter. The profile defines the evidence and state transitions WOS accepts.
2. **Intent is explicit.** A drawn signature image alone is not intent. The profile requires consent and affirmation evidence.
3. **Identity binding is provider-neutral.** The profile records authentication method, provider reference, assurance strength, and optional external attestation reference without baking in one identity provider.
4. **Trellis boundary stays clean.** WOS emits `SignatureAffirmation`; Trellis anchors it and composes certificate/export artifacts.
5. **Kernel remains stable.** Signature roles attach to `human` actors. The kernel actor enum is not widened.

### 1.3 Scope

**Within scope:** signer roles; signing-flow patterns; lifecycle tags; envelope and recipient state semantics; reminder, expiry, decline, void, and reassignment semantics; signer-authentication policies; intent capture; identity binding; document binding; `SignatureAffirmation` provenance; profile conformance; **the registered set of signing-intent URIs (§2.13) and their semantic meaning**; **signer-authority claim shape (§2.14), distinct from authentication-method strength**; **jurisdictional posture mapping for ESIGN, UETA, and eIDAS (§2.15)** — that is, which combinations of registered intent URI, authentication-method floor, and signer-authority claim a deployment under each posture MUST present for the profile to admit a `SignatureAffirmation`.

**Scope reopen note (1.0.0-draft.2, PLN-0380).** Earlier drafts carved out "jurisdiction-specific legal sufficiency claims" wholesale. This revision reopens the carve-out: WOS Signature Profile DOES make jurisdictional posture claims, scoped to the registered intent URIs in §2.13 and the floor matrix in §2.15. The carve-out remaining out of scope is narrower — see "Out of scope" below.

**Out of scope:** DocuSign administrative UX; legal advice; **counsel-pinned legal-sufficiency assertions** (whether a specific `SignatureAffirmation` is admissible in a specific tribunal under a specific statute) — those remain gated on counsel review per PLN-0355 (parent `PLANNING.md`); key management; rendered-document storage; cryptographic certificate-of-completion composition (Trellis ADR 0007); Trellis export-bundle layout; jurisdictional postures beyond ESIGN, UETA, and eIDAS — registered as deployment-local extensions per §2.15 until a future profile revision admits them.

**Authority discipline.** The Signature Profile authors the *spec text* of jurisdictional posture mapping. It does not author the *legal claim* a deployment makes to a regulator or counterparty. A deployment claiming "ESIGN-conformant" against this profile bears the legal burden; the profile bears the structural one. Counsel-pinned legal claim closure is tracked by PLN-0355.

### 1.4 Relationship to the Kernel

The Signature Profile targets a WOS Kernel Document through `targetWorkflow`. It binds signature roles to kernel actors by `actorId`. Those kernel actors MUST be human actors. Signing workflow states use ordinary kernel states and MAY carry profile lifecycle tags such as `awaiting-signature`.

The profile does not introduce a new kernel action type. Runtime processors implement signature behavior by composing existing lifecycle transitions, task assignment, timer events, provenance emission, and `custodyHook` admission.

### 1.5 Notational Conventions

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in BCP 14 when, and only when, they appear in ALL CAPITALS.

JSON syntax and data types are as defined in RFC 8259. URI syntax is as defined in RFC 3986. Date-time values use RFC 3339.

---

## 2. Normative Contract

### 2.1 Signature Profile Document

A Signature Profile Document MUST declare:

- `$wosSignatureProfile`
- `targetWorkflow`
- `roles`
- `documents`
- `signingFlow`
- `evidence`

The document MAY declare authentication policies, reminders, expiry, decline, void, and reassignment policy. Vendor extensions MUST use `x-` properties or the `extensions` object.

### 2.2 Signer Roles

The standard signer roles are:

| Role | Meaning |
|---|---|
| `signer` | Primary party who signs a document. |
| `in-person-signer` | Primary signer whose act occurs in the presence of an authorized actor. |
| `certified-recipient` | Actor who receives and acknowledges delivery without necessarily signing. |
| `witness` | Actor who attests to another signing act. |
| `notary` | Actor who performs a notarial or equivalent in-person verification role. |
| `approver` | Actor who approves a document without being the primary signer. |
| `form-filler` | Actor who completes fields for another signer before signature. |
| `viewer` | Actor who can inspect the document but does not complete the ceremony. |

Each role declaration MUST bind to a kernel actor by `actorId`. A Signature Profile Processor MUST reject a role whose `actorId` does not resolve to a kernel actor. A Signature Profile Processor MUST reject a role bound to a non-human kernel actor.

### 2.3 Signing Flow Patterns

The profile defines four flow types:

| Flow | Processor obligation |
|---|---|
| `sequential` | Later required steps MUST remain blocked until prior required dependencies complete. |
| `parallel` | Required steps without dependencies MAY complete in any order. Completion waits for all selected required steps. |
| `routed` | The processor MUST evaluate each step guard against case state and profile context. Non-selected steps MUST NOT block completion. |
| `free-for-all` | Required signers MAY complete in any order. Completion waits for the configured completion requirement. |

Each signing step MUST reference a declared role by `roleId` and a declared document by `documentId`. Step dependencies MUST reference sibling signing-step identifiers and MUST NOT form a cycle.

### 2.4 Lifecycle Tags

The standard lifecycle tags are:

- `awaiting-signature`
- `signature-complete`
- `signature-declined`
- `signature-expired`
- `signature-voided`

These are profile semantics layered over kernel state/tag data. They are not new kernel state kinds.

#### 2.4.1 Envelope and Recipient State Model

The signature envelope state model is profile-owned runtime state derived from signer/recipient progress and final seal observation. It MUST NOT be encoded as new kernel state kinds.

Envelope state transitions are:

```
draft -> sent -> in-progress -> completed | voided | expired
```

Recipient state transitions are:

```
pending -> accessed -> signed | declined | reassigned | expired
```

State obligations:

- `draft` means the signature ceremony is assembled but no signer has been notified or authorized to act.
- `sent` means at least one recipient has an active `pending` assignment and no recipient has completed access or signature.
- `in-progress` means at least one recipient has reached `accessed` or `signed`, but the ceremony has not reached a terminal envelope state.
- `completed` means every required signer/document pair has an admitted `SignatureAffirmation` and the runtime has observed the final seal or equivalent closure event for the ceremony. Last-signer success alone is not `completed` until the seal/closure observation lands.
- `voided` means an authorized actor canceled the ceremony. Pending recipients are canceled, and signatures attempted after the void MUST NOT be admitted.
- `expired` means the configured expiry fired before completion. Already-admitted `SignatureAffirmation` records remain evidence; pending recipients become `expired` and cannot sign without a new ceremony or reassignment path.
- `declined` is recipient-terminal, not envelope-terminal by itself. Envelope handling follows the configured decline transition (§2.11).
- `reassigned` is recipient-terminal for the original assignment and opens a replacement `pending` recipient assignment. Reassignment MUST preserve the original assignment edge for audit.

WOS runtimes MAY expose provider-native status strings to operators, but conformance and provenance MUST normalize those strings into the envelope/recipient states above before lifecycle transitions or `SignatureAffirmation` admission depend on them.

### 2.5 Intent Capture

A signature affirmation MUST have an explicit consent reference. The consent reference MUST identify the consent text, consent version, acceptance evidence path, and affirmation evidence path.

A drawn signature image, typed name, checkbox, or provider callback MAY be evidence, but it MUST NOT be treated as legal intent without the consent and affirmation evidence required by the profile.

### 2.6 Identity Binding

A signature affirmation MUST include identity-binding evidence with:

- authentication method;
- assurance level;
- identity-provider reference when a provider is used;
- optional external attestation reference.

Authentication methods are closed at the WOS center: `none`, `email-otp`, `sms-otp`, `knowledge-based`, `oidc`, `webauthn`, `credential`, `in-person`, `notary`, and `x-*` vendor methods.

Identity binding answers "who authenticated, and how strongly." It does NOT answer "in what capacity did the signer act." The latter is the signer-authority claim (§2.14). A profile MAY require both an authentication-method floor (§2.15) AND a signer-authority claim for the same affirmation; the two are independent gates and a runtime MUST evaluate both.

### 2.7 Document Binding

Each signed document declaration MUST include a document digest and digest algorithm. `sha-256` is REQUIRED for Core conformance. Other algorithms MAY be used only when declared by a future profile revision or an `x-*` extension policy.

The profile MAY reference a rendered document and a source response or evidence artifact by URI. WOS consumes those references as evidence inputs; it does not own rendered-document or source-response storage.

### 2.8 SignatureAffirmation Provenance

When a signing task completes and all profile requirements for that signing act are satisfied, the runtime MUST emit exactly one `SignatureAffirmation` provenance record for each signer/document pair.

The custody/export event type for an admitted `SignatureAffirmation` is `wos.kernel.signature_affirmation`; the inner record kind remains `signatureAffirmation` until D26 removes inner `recordKind` dispatch.

The record MUST include:

- `caseLedgerId` — the durable case-ledger identifier receiving the emitted record.
- `signerId`
- `roleId`
- `role`
- `documentId` — the flat Signature Profile document id retained for existing profile/document lookups.
- `signingActId` — an opaque identifier shared by all `SignatureAffirmation` records produced by one human signing gesture, including localized document variants.
- `documentRef` — a structured reference to the rendered document presentation the signer saw. It MUST include `documentId` and `locale`; `locale` is a BCP 47 token or `und` when no locale was declared.
- `documentHash`
- `presentationHash` — the digest of the rendered presentation bytes under the `trellis-presentation-artifact-v1` domain tag. It binds the signer-visible presentation separately from the source document digest.
- `documentHashAlgorithm`
- `sourceSignatureSystem` — the binding or provider family that supplied the verified signature evidence.
- `sourceSignatureId` — the source-system signature id accepted by WOS.
- `signedPayloadDigest` — the consumed source signature evidence digest for the signer-assented payload. This digest is distinct from any transferred response-envelope hash.
- `signedPayloadDigestAlgorithm`
- `signedAt`
- `identityBinding`
- `consentReference`
- `signatureProvider`
- `ceremonyId`
- `profileRef` or `profileKey` according to ADR-0060 semantics
- `sourceResponseRef`
- `custodyHookEligible`
- `signingIntent` — a URI from the registered set in §2.13 naming the legal-effect class of this affirmation. The URI MUST equal the consumed source signature evidence intent; it records WOS governance acceptance of that legal-effect class.
- `signerAuthority` — a signer-authority claim (§2.14). REQUIRED for any registered intent URI whose §2.15 row sets a non-`self` signer-authority floor. OPTIONAL for `self` floors; when present it MUST validate against §2.14.
- `primitiveVerification` — a `{ status, reason? }` object reporting the cryptographic-primitive verification outcome from the binding adapter. See §2.8.1 for the contract.
- `witnessedSignatureRef` — a reference to the principal `SignatureAffirmation` when this record witnesses, notarizes, or counter-signs another signing act; otherwise `null`.

If both `documentId` and `documentRef.documentId` are present, they MUST be byte-equal. A mismatch is a document-binding failure and the runtime MUST NOT admit the record.

#### 2.8.1 `primitiveVerification`

Each `SignatureAffirmation` MUST carry a `primitiveVerification` object reporting the cryptographic-primitive verification status of the underlying authored-signature evidence at admission time:

| Status | Meaning |
| ------ | ------- |
| `verified` | The binding adapter executed and passed the cryptographic signature primitive (canonical-digest + signature-suite check over the binding's signature value/method, e.g. Formspec `signatureValue` / `signatureMethod`). Required for legal-tier deployments that gate on a `verified` posture. |
| `deferredPendingHelper` | The binding adapter parsed and pre-checked the signature (pins, consent, digest) but did not execute the cryptographic primitive — for example because the canonicalization/signing helper for that binding has not shipped. Admissible at the WOS layer; downstream verifiers and operators decide whether this status meets their proof posture. |

**Amended 2026-05-08 (ADR-0089):** The `failed` outcome is no longer part of `SignatureAffirmation`. Failed evaluations emit a separate `signatureAdmissionFailed` record (§2.9). See ADR-0089.

When `status` is `deferredPendingHelper`, a non-empty `reason` MUST be present and SHOULD identify the cause (e.g. `formspec-signing-helper-pending` for the deferred case while `FORMSPEC-SIGN-HELPER-001` is unshipped). When `status` is `verified`, `reason` MAY be omitted.

A future deployment posture MAY require `verified` for any signing-intent class; until that posture lands, admission accepts `verified` or `deferredPendingHelper`. The reference Formspec binding currently emits `deferredPendingHelper` with reason `formspec-signing-helper-pending` because the cryptographic primitive over `signatureValue` / `signatureMethod` has not yet shipped at the binding; downstream verifiers MUST treat the resulting `SignatureAffirmation` as "pin/consent/digest pre-checked, primitive deferred" rather than "cryptographically verified."

**Binding to Formspec and Trellis.** For Formspec-backed signing, WOS consumes a Formspec `authoredSignatures[*]` record and admits `SignatureAffirmation` only after `signatureId`, `signingActId`, `documentRef`, `signedPayload.digest`, signed-payload response pins, `documentHash`, `presentationHash`, `consentAccepted`, `signingIntent`, signer role, signer authority, identity, and posture checks pass. A WOS/Trellis profile MAY additionally require one `UserContentAttestationPayload` per signer/document pair under the Trellis `trellis.user-content-attestation.v1` event extension (Trellis ADR 0010 §"Wire shape") as byte-level proof. That Trellis payload corroborates bytes and chain position; it does not define Formspec signature semantics or WOS signing-intent meaning.

If consent evidence is missing, identity binding is below the role's required policy, the signing-intent URI is unregistered for the deployment, the signer-authority floor for the URI is not met, or required source signature evidence is invalid, the runtime MUST NOT emit `SignatureAffirmation` and MUST instead emit a `signatureAdmissionFailed` record (§2.9).

### 2.9 SignatureAdmissionFailed Record

When a signature is evaluated but not admitted, the runtime MUST emit exactly one `signatureAdmissionFailed` provenance record for each evaluated signer/document pair that failed admission. The custody/export event type is `wos.kernel.signature_admission_failed`; the inner record kind remains `signatureAdmissionFailed` until D26 removes inner `recordKind` dispatch. This record is a sibling artifact to `SignatureAffirmation` (§2.8), used exclusively for non-admission — it is not a status inside `SignatureAffirmation`.

A `signatureAdmissionFailed` record MUST carry:

- `reason` — a closed-enum admission-failure code (§2.9.1)
- `evidenceBindings` — a `{responseId, signedPayloadDigest, signatureId, signingIntent}` object that ties the failed admission to the source response, verified signature evidence, and signing-intent URI
- `emittedAt` — RFC 3339 timestamp of evaluation (named `emittedAt` in the schema; previously referred to as `evaluatedAt` in draft spec text)

#### 2.9.1 `reason` values

| `reason` | Meaning | Required fields |
|---|---|---|
| `primitive_verification_failed` | The cryptographic primitive was attempted and rejected by the binding adapter. | `evidenceBindings` |
| `method_unsupported` | The identity method or signing intent is not supported by the runtime or deployment posture. | `evidenceBindings` |
| `method_unregistered` | The identity method is not registered in the WOS method registry. | `evidenceBindings` |
| `evidence_divergence` | Evidence fields in the response diverge from the verified binding evidence. | `evidenceBindings` |
| `posture_floor_unmet` | The signing intent is registered but the affirmation does not meet the authentication-method floor or signer-authority floor required by the declared posture (§2.15). | `evidenceBindings` |
| `registry_unrecognized_method` | The signing method is not recognized in the method registry. | `evidenceBindings` |
| `adapter_unavailable` | The adapter required to verify this method is unavailable. | `evidenceBindings` |

Additional reason values (`signing_intent_unregistered`, `signer_authority_invalid`, `evidence_binding_invalid`, `consent_missing_or_invalid`, `identity_binding_below_policy`, `document_hash_mismatch`, `uri_propagation_mismatch`, `esign_consent_unresolved`, `trellis_byte_verification_failed`, `x-*`) are deferred to Phase 3.3+.

#### 2.9.2 Cross-references

This record is referenced by ADR-0089 (which removed the `failed` status from `primitiveVerification` and directed non-admission emission here) and by the decision matrix D3 in `fel-core` (which defines the receipt-processing decision table). See also §2.10 for how the Posture Declaration drives admission decisions.

### 2.10 Posture Declaration Consumption

Signature admission is a function of three inputs: the incoming receipt (signature evidence from the binding adapter), the deployment's Posture Declaration, and the profile's normative rules. This can be summarized as:

```
admission = receipt × posture → decision
```

Where `decision ∈ { emit SignatureAffirmation, emit signatureAdmissionFailed }`.

#### 2.10.1 Posture Declaration object shape

The Posture Declaration is defined by the Trellis Operational Companion. Its shape relevant to signature admission is (reference ADR-0090):

```json
{
  "posture": {
    "jurisdictions": ["general" | "esign" | "ueta" | "eidas"],
    "signing": {
      "intents": {
        "<uri>": {
          "allow": true | false,
          "authMethodFloor": "...",
          "signerAuthorityFloor": "...",
          "esignConsumerConsentRequired": true | false
        }
      },
      "deploymentLocalIntents": {
        "<uri>": { "..." }
      }
    }
  }
}
```

The WOS Signature Profile Runtime reads the `signing` block from the active Posture Declaration at admission time. Fields not declared in the Posture Declaration default to the profile's normative floors (§2.15).

#### 2.10.2 Admission comparison

For each incoming receipt, the runtime:

1. Resolves the `signingIntent` URI. If the URI is not in the registered set (§2.13.1) and not in `posture.signing.deploymentLocalIntents`, admission fails with `reason: signing_intent_unregistered`.
2. For the resolved URI, retrieves the effective floor by computing `max(profile_floor, deployment_floor)` for both authentication-method and signer-authority, where the profile floor is from the §2.15 matrix and the deployment floor is from the Posture Declaration.
3. Checks the receipt's authentication method against the effective floor. If the receipt's method is weaker, admission fails with `reason: posture_floor_unmet`.
4. Checks the receipt's `signerAuthority` claim against the effective floor per §2.14.4–§2.14.5. If the claim is absent when required or weaker than the floor, admission fails with `reason: posture_floor_unmet`.
5. If the posture includes `esign` jurisdiction and the URI row has `esignConsumerConsentRequired: true`, verifies the ESIGN §7001(c) consent reference (§2.15.2).
6. If all checks pass, emits `SignatureAffirmation`.

#### 2.10.3 Posture unmet

When any posture check fails, the runtime MUST emit `signatureAdmissionFailed` with `reason: posture_floor_unmet` (or the more specific reason code from §2.9.1 that applies to the first failing check). The `reasonDetail` MUST identify: the posture that failed, the expected floor, and the received value.

#### 2.10.4 Sunset: `deploymentLocalSigningIntents`

The `deploymentLocalSigningIntents` field was the original mechanism in the Posture Declaration for registering deployment-local intent URIs. ADR-0090 restructures the Posture Declaration shape; the `signing.deploymentLocalIntents` block (§2.10.1) replaces `deploymentLocalSigningIntents` at the top level. Deployments MUST migrate from `deploymentLocalSigningIntents` to `posture.signing.deploymentLocalIntents` when adopting the ADR-0090 posture shape. The `deploymentLocalSigningIntents` top-level field is **deprecated** and will be removed in a future profile revision.

### 2.11 Decline, Void, Expiry, and Reassignment

A decline MUST record signer, reason, timestamp, and document. It MUST follow the configured decline transition.

A void MUST cancel pending signing steps and record the authorizing actor and reason.

An expiry MUST be driven by a typed kernel timer event and MUST record pending signers and expired documents.

A reassignment MUST record original signer, new signer, authorizing actor, timestamp, and reason. Reassignment MUST NOT erase accountability for the original assignment.

### 2.12 Witness, Counter-Signature, Notary, and In-Person Signing

A witness or counter-signature step MUST depend on the primary signer affirmation unless the profile explicitly declares another dependency.

A notary or in-person signer role MUST require an authentication policy whose method is `in-person`, `notary`, or an `x-*` method that declares equivalent in-person evidence.

When a witness, notary, or counter-signature step attests to another signing act, the emitted `SignatureAffirmation.witnessedSignatureRef` MUST reference the principal `SignatureAffirmation`. Non-attesting signing acts MUST set `witnessedSignatureRef` to `null`.

### 2.13 Signing-Intent URI Registry

Every `SignatureAffirmation` carries a `signingIntent` URI naming its legal-effect class. Trellis ADR 0010 owns the byte-level URI shape (`signing_intent: tstr`, RFC 3986 syntactic check at the byte verifier). WOS Signature Profile owns the URI's *meaning*: which intent URIs the profile recognizes, what each one claims, and what authentication-method floor and signer-authority floor (§2.14) each one requires under each jurisdictional posture (§2.15).

#### 2.13.1 Registered URIs (baseline set)

The profile registers the following intent URIs. The set is **append-only**: removing a URI is a breaking profile change; adding a URI is additive (§3.5).

This table is the baseline WOS signing-intent registry, not an example catalog. A Signature Profile Runtime MUST recognize every URI in this table. Deployment-local URI registration happens only through §2.13.2, and the `urn:wos:signing-intent:*` namespace is reserved to this profile.

| Intent URI | Meaning | Authentication-method floor (general) | Signer-authority floor (§2.14) |
|---|---|---|---|
| `urn:wos:signing-intent:applicant-signature` | The principal party signing on their own behalf — the primary applicant, party, signer of record. | `email-otp` or stronger | `self` |
| `urn:wos:signing-intent:counter-signature` | A second party signing the same document on their own behalf to indicate concurrence (e.g., co-applicant, co-purchaser). | `email-otp` or stronger | `self` |
| `urn:wos:signing-intent:witness-attestation` | A witness attesting to having observed another party's signing act. Signer is the witness, not the principal. | `email-otp` or stronger | `witness` |
| `urn:wos:signing-intent:notarial-attestation` | A commissioned notary or jurisdictional equivalent attesting to identity and signing act of another party under a notarial commission. | `notary` or `in-person` | `notary-commissioned` |
| `urn:wos:signing-intent:consent` | Affirmative consent to a defined disclosure, policy, or processing activity (e.g., ESIGN consumer consent, GDPR processing consent). Distinct from a substantive signature on a contract. | `email-otp` or stronger | `self` |
| `urn:wos:signing-intent:attestation-of-fact` | A non-notarial attestation that named facts are true to the signer's knowledge — e.g., a benefits applicant attesting under penalty of perjury, an officer attesting to corporate records. | `email-otp` or stronger | `self` or `as-officer-of` |
| `urn:wos:signing-intent:agent-as-attorney-in-fact` | A signer acting under a power of attorney for another principal. | `oidc` / `webauthn` / `credential` | `as-attorney-in-fact` |
| `urn:wos:signing-intent:agent-as-officer` | A signer acting in their capacity as an officer or authorized agent of an organization. | `oidc` / `webauthn` / `credential` | `as-officer-of` |
| `urn:wos:signing-intent:approval` | A reviewer approving a document without being its primary signer (e.g., a manager approving a subordinate's submission). Distinct from `applicant-signature`: approval does not assert authorship. | `email-otp` or stronger | `self` or `as-officer-of` |
| `urn:wos:signing-intent:certified-receipt` | An acknowledgement of receipt and inspection, without signing the document's substantive content. | `email-otp` or stronger | `self` |

The "general" floor is the baseline floor when no jurisdictional posture is declared. Jurisdiction-specific floors per §2.15 MAY raise (never lower) the floor for a given intent URI.

#### 2.13.2 Deployment-local URIs

Deployments MAY register additional intent URIs in their Posture Declaration (Trellis Operational Companion §"Posture Declaration"). A deployment-local URI MUST:

- use a deployment-scoped URI namespace distinct from `urn:wos:signing-intent:*` (which is reserved for this profile);
- declare its meaning, authentication-method floor, and signer-authority floor in the Posture Declaration;
- declare its mapping under each jurisdictional posture the deployment claims (§2.15);
- pass the same byte-level RFC 3986 check at the Trellis verifier per ADR 0010 step 6d.2.

A Signature Profile Runtime MUST reject a `SignatureAffirmation` whose `signingIntent` is neither in §2.13.1 nor in the deployment's Posture Declaration registry. Trellis admits any well-formed URI at the byte layer; semantic gating happens here.

#### 2.13.3 URI propagation contract

The same URI string traverses three layers. Each layer's verifier MUST observe byte equality with its source:

1. **Formspec Response.** `authoredSignatures[*].signingIntent` (Formspec Core §2.1.6) carries the URI authored by the signer.
2. **WOS `SignatureAffirmation`.** `signingIntent` MUST equal the corresponding Formspec `authoredSignatures[*].signingIntent`.
3. **Trellis `UserContentAttestationPayload`, when required by profile.** `signing_intent` MUST equal the WOS `signingIntent`.

A divergence at any required boundary fails the affirmation. WOS lint and runtime enforce (1)↔(2). When a WOS/Trellis profile requires UCA, WOS runtime and the domain verifier enforce (2)↔(3) at `custodyHook` or export-verification admission; Trellis Core verifier enforces only byte-level invariants and URI syntax per RFC 3986.

#### 2.13.4 Open question — notarial commission credential format

The `notarial-attestation` URI requires a notary commission as the authority-source (§2.14). Commission credential format varies by jurisdiction (state-issued certificate, registry lookup URI, x.509 cert with notarial extension, deployment-issued bearer credential). The §2.14 `authoritySource` field admits any URI; deployment-side registration of which commission registries are accepted in which jurisdictional posture is the deployment's responsibility, not this profile's. A future profile revision MAY add a `notaryCommissionFormat` enum once jurisdiction-specific patterns settle.

### 2.14 Signer-Authority Claim

The signer-authority claim declares **capacity to bind** — in what role and on whose behalf the signer acted. It is distinct from §2.6 identity binding (which is identity-strength). Both are independent gates: a strong identity claim (`webauthn`) does NOT establish authority to bind a third party; a weak identity claim (`email-otp`) is not rescued by an authority claim.

#### 2.14.1 Claim shape

A `signerAuthority` claim has the following shape:

```json
{
  "class": "self" | "as-officer-of" | "as-attorney-in-fact" | "notary-commissioned" | "witness" | "x-*",
  "authoritySource": "<URI of the credential, commission, or appointment instrument>",
  "principal": "<URI of the principal whose interest is bound, when class != self/witness/notary-commissioned>",
  "evidenceBinding": {
    "evidenceHash": "<digest of supporting evidence>",
    "evidenceHashAlgorithm": "sha-256",
    "evidenceLocation": "<optional URI to the evidence in the case ledger or external store>"
  },
  "validFrom": "<RFC 3339 date-time, optional>",
  "validUntil": "<RFC 3339 date-time, optional>",
  "extensions": { "x-*": "..." }
}
```

#### 2.14.2 Class semantics

| `class` | Meaning | `principal` | `authoritySource` content |
|---|---|---|---|
| `self` | Signer acts on their own behalf only. | MUST be omitted or equal `signerId`. | OPTIONAL; if present, identifies the signer's identity record. |
| `as-officer-of` | Signer acts as an officer or authorized agent of an organization. | REQUIRED — URI of the organization. | URI of the appointment / authorization instrument (board resolution, employment record, agency registration). |
| `as-attorney-in-fact` | Signer acts under a power of attorney for a natural or legal person. | REQUIRED — URI of the principal granting the power. | URI of the executed power-of-attorney instrument. |
| `notary-commissioned` | Signer acts as a commissioned notary or jurisdictional equivalent. | OPTIONAL — URI of the affiant whose act the notary attests. | REQUIRED — URI of the notarial commission. |
| `witness` | Signer acts as a witness to another party's signing act. | OPTIONAL — URI of the principal whose act is witnessed. | OPTIONAL; if present, identifies the witness's identity record. |
| `x-*` | Vendor or deployment-defined authority class. | Per Posture Declaration. | Per Posture Declaration. |

The class enum is closed at the WOS center. Deployments add `x-*` classes through the Posture Declaration; admission rules and floor mapping (§2.15) for `x-*` classes MUST be declared per deployment.

#### 2.14.3 Evidence binding

`evidenceBinding.evidenceHash` MUST be the digest of the canonical supporting evidence (commission certificate bytes, executed power of attorney bytes, board resolution bytes). `sha-256` is REQUIRED for Core conformance. The hash is what binds the claim to its proof; the optional `evidenceLocation` URI is for retrieval, not for trust.

When a deployment carries the supporting evidence as a chained event (e.g., a notary commission registered as its own ledger event), `evidenceLocation` MAY be the `canonical_event_hash` URI of that event. The verifier resolves the URI; trust derives from the chain's integrity, not from the URI's resolvability at verification time.

#### 2.14.4 Runtime obligations

A Signature Profile Runtime, when admitting a `SignatureAffirmation`, MUST:

1. Resolve the `signingIntent` URI against §2.15 (or the Posture Declaration registry).
2. If the resolved URI's signer-authority floor is anything other than `self`, REQUIRE a non-omitted `signerAuthority` claim and confirm `signerAuthority.class` matches the floor (or is a stricter class as ranked in §2.14.5).
3. Confirm `authoritySource` is present when REQUIRED for the class.
4. Confirm `principal` is present when REQUIRED for the class and that it is not equal to `signerId` (a signer cannot be their own principal in a delegating class).
5. Confirm `evidenceBinding.evidenceHash` is present and uses an algorithm permitted by §2.7.
6. If `validFrom` / `validUntil` are present, confirm `signedAt` falls within the window.
7. If any check fails, MUST NOT emit `SignatureAffirmation` and MUST record the failure reason in the runtime's diagnostic stream.

#### 2.14.5 Authority class strength ordering

For floor-matching purposes, the classes are ordered weakest → strongest:

`self` → `witness` → `as-officer-of` ≈ `as-attorney-in-fact` → `notary-commissioned`

A claim of strength ≥ floor satisfies the floor. `as-officer-of` and `as-attorney-in-fact` are unordered relative to each other; either satisfies a floor expressed as either. `x-*` classes are unordered relative to the closed set; the Posture Declaration MUST place each `x-*` class explicitly relative to a registered class for floor-matching purposes.

### 2.15 Jurisdictional Posture Mapping

A deployment declares zero or more jurisdictional postures in its Posture Declaration. Postures the profile recognizes:

- **`general`** — no jurisdictional claim. The default. Floors are the §2.13.1 "Authentication-method floor (general)" column.
- **`esign`** — U.S. Electronic Signatures in Global and National Commerce Act (15 U.S.C. ch. 96).
- **`ueta`** — U.S. Uniform Electronic Transactions Act (state-enacted; floors apply where the deployment claims a UETA-adopting jurisdiction).
- **`eidas`** — EU Regulation 910/2014 (electronic IDentification, Authentication and trust Services).

A deployment MAY claim multiple postures simultaneously (e.g., a U.S. federal-agency deployment claiming both ESIGN and UETA). When multiple postures apply, the **strictest** floor for each (intent URI × authentication-method) and (intent URI × signer-authority) cell wins.

#### 2.15.1 Posture floor matrix

The matrix below SHOULD be read as: "to admit a `SignatureAffirmation` carrying intent URI X under posture Y, the runtime MUST require at least authentication-method floor A and signer-authority floor B." Floors may be raised per deployment; this is the profile's normative minimum.

| Intent URI | `general` auth floor | `esign` auth floor | `ueta` auth floor | `eidas` auth floor | Authority floor (all postures) |
|---|---|---|---|---|---|
| `applicant-signature` | `email-otp` | `email-otp` + ESIGN consent (§2.15.2) | `email-otp` | `oidc` (advanced electronic signature) | `self` |
| `counter-signature` | `email-otp` | `email-otp` + ESIGN consent | `email-otp` | `oidc` | `self` |
| `witness-attestation` | `email-otp` | `email-otp` | `email-otp` | `oidc` | `witness` |
| `notarial-attestation` | `notary` or `in-person` | `notary` or `in-person` | `notary` or `in-person` (RON-permitted states only; deployment declares) | `notary` or `in-person` (qualified electronic signature where required) | `notary-commissioned` |
| `consent` | `email-otp` | `email-otp` + ESIGN consumer consent (§2.15.2) | `email-otp` | `email-otp` (where lawful basis distinct from signature) | `self` |
| `attestation-of-fact` | `email-otp` | `email-otp` + ESIGN consent | `email-otp` | `oidc` | `self` or `as-officer-of` |
| `agent-as-attorney-in-fact` | `oidc` / `webauthn` / `credential` | `oidc` / `webauthn` / `credential` + ESIGN consent | `oidc` / `webauthn` / `credential` | `webauthn` / `credential` (advanced electronic signature) | `as-attorney-in-fact` |
| `agent-as-officer` | `oidc` / `webauthn` / `credential` | `oidc` / `webauthn` / `credential` + ESIGN consent | `oidc` / `webauthn` / `credential` | `webauthn` / `credential` | `as-officer-of` |
| `approval` | `email-otp` | `email-otp` | `email-otp` | `oidc` | `self` or `as-officer-of` |
| `certified-receipt` | `email-otp` | `email-otp` | `email-otp` | `email-otp` | `self` |

The matrix is normative for the registered URIs in §2.13.1. Deployment-local URIs (§2.13.2) MUST declare their own row in the deployment's Posture Declaration.

#### 2.15.2 ESIGN consumer-consent prerequisite

ESIGN posture additionally requires consumer-consent disclosures (15 U.S.C. § 7001(c)) for a `SignatureAffirmation` against a consumer. The profile records this through the existing `consentReference` field (§2.5): under `esign` posture, the runtime MUST verify that `consentReference` resolves to a Formspec consent record whose disclosed-content meets the ESIGN §7001(c)(1) categories (right to receive paper records, withdrawal procedure, hardware/software requirements, scope of consent). The profile does not author the consent text; the deployment authors it and the profile checks the structural reference.

UETA does not impose the §7001(c) consumer-consent prerequisite at the federal level, but state UETA enactments may. Deployments claiming UETA in jurisdictions with such state-level prerequisites MUST raise the floor accordingly in their Posture Declaration.

#### 2.15.3 eIDAS signature-tier mapping

eIDAS distinguishes three signature tiers: *simple*, *advanced* (AdES), *qualified* (QES). The matrix above maps `oidc` / `webauthn` / `credential` floors to advanced electronic signature; `notary` to qualified-equivalent under deployment-specific qualified trust service provider integration. A deployment claiming QES for any URI MUST register the qualified trust service provider in its Posture Declaration; the profile floor admits AdES baseline for the registered URIs and a deployment may raise to QES per use case.

#### 2.15.4 Posture-declaration responsibility

Per §1.3, the profile authors the structural mapping; the deployment authors the legal claim. A deployment claiming `esign` posture in its Posture Declaration commits to:

- meeting the §2.15 floors for every URI it admits;
- carrying ESIGN §7001(c) consumer-consent disclosures where the affirmation is against a consumer;
- counsel review per parent PLN-0355 before commercial-mode procurement claims.

The Trellis Operational Companion's Posture Declaration enforces the structural side; counsel-pinned legal sufficiency is gated downstream.

---

## 3. Composition

### 3.1 Attachment Point

The Signature Profile is a profile document. It attaches to a kernel workflow by `targetWorkflow` and composes with governance, AI, and advanced governance documents through the existing kernel seams.

### 3.2 Formspec Composition

Formspec captures signature controls, consent controls, identity-proofing references, and canonical response fields. WOS consumes those fields as evidence inputs. WOS MUST NOT infer a valid signing act from fields that failed Formspec validation or signed-payload verification.

The signing-intent URI authored into Formspec `authoredSignatures[*].signingIntent` (Formspec Core §2.1.6) MUST equal the WOS `SignatureAffirmation.signingIntent` per §2.13.3. In a Formspec-backed binding, WOS maps Formspec `signatureId` into `sourceSignatureId`, carries `signingActId`, `documentRef`, and `presentationHash` into the WOS record, sets `sourceSignatureSystem` to the binding/provider family, and records `signedPayload.digest` in `signedPayloadDigest`; these identify the authored signature that WOS accepted without making WOS the source of the Formspec signature primitive.

**Source-of-truth.** The Formspec Response is the canonical record of a signing act. WOS `SignatureAffirmation` and `signatureAdmissionFailed` records are derived/corroborating evidence — they attest that WOS governance admitted (or rejected) the act, but they do not replace the Formspec record as the primary source. See the source-of-truth table in `formspec/specs/core/spec.md` §2.1.N for the full cross-spec evidence hierarchy.

### 3.3 Trellis Composition

WOS emits `SignatureAffirmation` records through `custodyHook`. Trellis anchors the WOS evidence record and owns certificate-of-completion and export-bundle composition. WOS MUST NOT place Trellis-owned chain fields inside the authored signature record.

A WOS/Trellis profile MAY require the byte-level proof for a `SignatureAffirmation` to include one `UserContentAttestationPayload` per signer/document pair, encoded under Trellis `trellis.user-content-attestation.v1` (ADR 0010). The specs compose through a **layered verifier contract**:

| Layer | What it verifies | Failure mode |
|---|---|---|
| Formspec verifier | Response schema; `authoredSignatures[*].signatureId`; `signedPayload.responseId`, `definitionUrl`, and `definitionVersion` pins; `signedPayload.digest`; consent and signature-method/provider checks. | WOS MUST NOT admit `SignatureAffirmation`; runtime records the failure reason. |
| Trellis ADR 0010 verifier | URI is syntactically valid (RFC 3986); `attested_event_hash` resolves to chain position; `identity_attestation_ref` resolves; signature valid under domain tag `trellis-user-content-attestation-v1`; signing key Active. | `integrity_verified = false` per Core §19 step 6d. |
| WOS Signature Profile runtime | `signingIntent` is in the registered set (§2.13.1) or the deployment's Posture Declaration; equals the consumed Formspec `authoredSignatures[*].signingIntent`; meets the §2.15 floor for the deployment's declared posture; `signerAuthority` claim (§2.14) satisfies the URI's authority floor; document-hash and consent evidence valid; ESIGN §7001(c) consent reference resolves where `esign` posture applies. | `SignatureAffirmation` MUST NOT be admitted at `custodyHook`; runtime records the failure reason. |

The active verifier set composes. **Integrity failure at any required layer fails the artifact.** Formspec catches signed-response attacks; Trellis catches byte-level integrity attacks (wrong-position attestation, key-state evasion, cross-family signature confusion); WOS catches semantic-intent attacks (unregistered URI, floor underrun, missing authority claim, wrong posture). Neither Trellis nor WOS alone creates Formspec signature semantics.

A WOS Signature Profile Runtime MUST consume a successful Formspec signature-verification result or perform equivalent Formspec signed-payload checks before semantic WOS admission. If the active profile also requires Trellis UCA, byte-level verification MUST pass before export or custody claims assert Trellis-backed integrity. The Formspec, WOS, and Trellis results are reported separately in the runtime or verifier diagnostic stream.

### 3.4 Conflict Handling

Profile-to-kernel reference failures are load-time errors. Missing optional policy blocks use the profile defaults defined by the schema and runtime. Conflicting role or step declarations reject; processors MUST NOT merge duplicate ids by declaration order.

### 3.5 Versioning

Changing signer-role enums, flow semantics, authentication-method semantics, or `SignatureAffirmation` required fields is a breaking profile change. Adding a new optional policy block or `x-*` extension is additive.

Changes specific to §2.13–§2.15:

- **Adding** a registered intent URI to §2.13.1, an authority class to §2.14, a jurisdictional posture to §2.15, or raising a floor in the §2.15 matrix is **additive** (deployments that did not declare the new URI / class / posture see no behavioral change; deployments that opt in get the new floor).
- **Removing** a registered intent URI, removing an authority class, removing a posture, or **lowering** a floor in the §2.15 matrix is a **breaking** change. A deployment relying on the removed item or the prior floor must explicitly migrate.
- **Renaming** a URI string (even a typo fix) is **breaking** — URIs are byte-equal across three layers (§2.13.3) and a rename forces all three to redeploy in lockstep. Deprecate-and-add is the migration pattern, not rename.

---

## 4. Conformance

### 4.1 Conformance Classes

**Signature Profile Document.** A JSON document conforming to this specification and the Signature Profile JSON Schema.

**Signature Profile Processor.** A processor that loads Signature Profile Documents, resolves profile references, and rejects invalid documents.

**Signature Profile Runtime.** A runtime that executes signing flow semantics and emits `SignatureAffirmation` provenance records.

### 4.2 Conformance Profiles

| Profile | Requirements |
|---|---|
| Core | Single, sequential, and parallel signing; envelope and recipient state normalization; consent; identity binding; document binding; `SignatureAffirmation` provenance; signing-intent URI from the registered set in §2.13.1; `general` posture floor checks per §2.15. |
| Complete | Core, plus routed and free-for-all flows; reminders; expiry; decline; void; reassignment; witness; notary; in-person signer; certified recipient; deployment-local intent URIs (§2.13.2); signer-authority claims for non-`self` floors (§2.14); jurisdictional postures (§2.15) the deployment declares (`esign`, `ueta`, `eidas`); layered-verifier composition with Trellis ADR 0010 (§3.3). |

Complete is a strict superset of Core.

### 4.3 Verification

Schema validation checks the document shape, closed enums, URI/reference field shapes, and `x-*` extension discipline. Schema validation MUST also check that any `signingIntent` value declared in a Signature Profile Document is a syntactically valid URI per RFC 3986 (the byte-level check Trellis ADR 0010 also enforces).

Lint checks profile-to-kernel consistency: target workflow resolution, actor resolution, human actor binding, authentication-policy key resolution, role/document/step references, dependency cycles, FEL guard parsing, timer-event mapping, and ADR-0060 naming. Lint additionally checks: every authored `signingIntent` URI is either in §2.13.1 or in the deployment's Posture Declaration registry; every `signerAuthority.class` matches §2.14.1's closed enum (or a `x-*` class declared in the Posture Declaration); the §2.15 floor row for the declared posture is satisfied by the document's authentication policies; Formspec `authoredSignatures[*].signingIntent` equals the corresponding WOS `signingIntent` (§2.13.3 boundary 1↔2); signature steps that declare `signingIntent` use registered or deployment-local URIs.

Runtime conformance checks signing behavior: sequential blocking, parallel completion, routed guard selection, envelope/recipient state normalization, expiry timers, decline paths, reassignment accountability, witness dependencies, notary/in-person authentication, missing-consent rejection, and custody append inclusion. Runtime additionally checks: the consumed Formspec authored signature verifies its `signatureId`, `signingActId`, `documentRef`, signed-payload digest, response pins, consent, document hash, `presentationHash`, and `signingIntent`; `signingIntent` is registered for the deployment at the time of admission; §2.15 floor is satisfied for the declared posture; `signerAuthority` claim is present and valid where the URI's floor demands it; `evidenceBinding.evidenceHash` algorithm is permitted by §2.7; ESIGN §7001(c) consumer-consent reference resolves under `esign` posture; any profile-required Trellis UCA verifier passes before WOS/Trellis integrity claims are made.

### 4.4 Conformance fixture coverage

Conformance fixtures for §2.13–§2.15 land under `crates/wos-conformance/tests/fixtures/SIG-*` once the WOS event-type taxonomy ratification (parent PLN-0384) settles the F-13 signature-related event-type registrations. The fixture set MUST include at minimum:

- one positive vector per registered intent URI in §2.13.1, exercising the URI's `general` floor;
- one positive vector per declared jurisdictional posture (`esign`, `ueta`, `eidas`), exercising the strictest floor for `applicant-signature`;
- one positive vector exercising a non-`self` signer-authority claim for `as-officer-of`, `as-attorney-in-fact`, and `notary-commissioned`;
- one positive vector exercising the layered-verifier composition with a real Trellis ADR 0010 `UserContentAttestationPayload` byte-encoding;
- negative vectors per failure surface: unregistered URI; URI registered but floor underrun; missing signer-authority claim where the floor demands it; `signerAuthority.principal` equal to `signerId` for a delegating class; `evidenceBinding.evidenceHash` algorithm not permitted by §2.7; ESIGN posture without ESIGN §7001(c) consent reference; URI mismatch across the three-layer propagation contract (§2.13.3).

Until PLN-0384 closes, the fixtures live in the existing `SIG-*` series with placeholder ids; once the F-13 signature-related event names ratify, they renumber to align with the namespace registration. The fixture authoring is non-blocking on this profile revision — fixture coverage gates land with the namespace ratification, not with the spec text.
