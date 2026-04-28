---
title: WOS Signature Profile
version: 1.0.0-draft.2
date: 2026-04-28
status: draft
---

> **Partial absorption (ADR 0076 D-2).** Signature workflow semantics absorbed into the merged `schemas/wos-workflow.schema.json` `signature` embedded block — signature is no longer a separate sidecar but a load-bearing block on any DocuSign-tier workflow. Prose normative content remains here (signing flow patterns, role binding, authentication policies) until the absorption pass lands the within-block descriptions. SIG-001..SIG-012 lint rules continue to cite this document as canonical until the schema descriptions become full normative.


# WOS Signature Profile v1.0

**Version:** 1.0.0-draft.2
**Date:** 2026-04-28
**Editors:** Formspec Working Group
**Companion to:** WOS Kernel Specification v1.0

---

## Abstract

The WOS Signature Profile defines workflow semantics for signature ceremonies in WOS. It covers signer roles, signing order, routed signing, free-for-all signing, witness and notary participation, reminders, expiry, decline, void, reassignment, intent capture, identity binding, document binding, and the `SignatureAffirmation` provenance record emitted when a signing act is accepted.

The profile is a parallel seam. It does not add kernel actor types and does not define cryptographic certificate-of-completion bundles. Formspec captures signing and consent evidence. WOS governs the workflow semantics and emits semantic evidence. Trellis anchors that evidence and owns export-bundle composition.

---

## Status of This Document

This document is a **draft specification**. Implementors MUST NOT treat it as stable until WOS v1.0 is ratified.

### Revision history

| Version | Date | Change |
|---|---|---|
| 1.0.0-draft.1 | 2026-04-22 | Initial draft. |
| 1.0.0-draft.2 | 2026-04-28 | **§1.3 scope reopen (PLN-0380).** ESIGN / UETA / eIDAS posture mapping moved from out-of-scope to in-scope. Added §2.11 signing-intent URI registry, §2.12 signer-authority claim, §2.13 jurisdictional posture mapping. §2.8 binds to Trellis ADR 0010 `UserContentAttestationPayload` as the byte-level proof. §3.3 names the layered-verifier composition contract with Trellis. Counsel-pinned legal-sufficiency claims remain gated on PLN-0355. |

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

**Within scope:** signer roles; signing-flow patterns; lifecycle tags; reminder, expiry, decline, void, and reassignment semantics; signer-authentication policies; intent capture; identity binding; document binding; `SignatureAffirmation` provenance; profile conformance; **the registered set of signing-intent URIs (§2.11) and their semantic meaning**; **signer-authority claim shape (§2.12), distinct from authentication-method strength**; **jurisdictional posture mapping for ESIGN, UETA, and eIDAS (§2.13)** — that is, which combinations of registered intent URI, authentication-method floor, and signer-authority claim a deployment under each posture MUST present for the profile to admit a `SignatureAffirmation`.

**Scope reopen note (1.0.0-draft.2, PLN-0380).** Earlier drafts carved out "jurisdiction-specific legal sufficiency claims" wholesale. This revision reopens the carve-out: WOS Signature Profile DOES make jurisdictional posture claims, scoped to the registered intent URIs in §2.11 and the floor matrix in §2.13. The carve-out remaining out of scope is narrower — see "Out of scope" below.

**Out of scope:** DocuSign administrative UX; legal advice; **counsel-pinned legal-sufficiency assertions** (whether a specific `SignatureAffirmation` is admissible in a specific tribunal under a specific statute) — those remain gated on counsel review per PLN-0355 (parent `PLANNING.md`); key management; rendered-document storage; cryptographic certificate-of-completion composition (Trellis ADR 0007); Trellis export-bundle layout; jurisdictional postures beyond ESIGN, UETA, and eIDAS — registered as deployment-local extensions per §2.11 until a future profile revision admits them.

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

Identity binding answers "who authenticated, and how strongly." It does NOT answer "in what capacity did the signer act." The latter is the signer-authority claim (§2.12). A profile MAY require both an authentication-method floor (§2.13) AND a signer-authority claim for the same affirmation; the two are independent gates and a runtime MUST evaluate both.

### 2.7 Document Binding

Each signed document declaration MUST include a document digest and digest algorithm. `sha-256` is REQUIRED for Core conformance. Other algorithms MAY be used only when declared by a future profile revision or an `x-*` extension policy.

The profile MAY reference a rendered document and a Formspec canonical response by URI. WOS consumes those references as evidence inputs; it does not own rendered-document storage.

### 2.8 SignatureAffirmation Provenance

When a signing task completes and all profile requirements for that signing act are satisfied, the runtime MUST emit exactly one `SignatureAffirmation` provenance record for each signer/document pair.

The record MUST include:

- `signerId`
- `roleId`
- `role`
- `documentId`
- `documentHash`
- `documentHashAlgorithm`
- `signedAt`
- `identityBinding`
- `consentReference`
- `signatureProvider`
- `ceremonyId`
- `profileRef` or `profileKey` according to ADR-0060 semantics
- `formspecResponseRef`
- `custodyHookEligible`
- `signingIntent` — a URI from the registered set in §2.11 naming the legal-effect class of this affirmation. The URI MUST also populate the corresponding Formspec `authoredSignatures[*]` entry per Formspec Core §2.1.6 ("authoredSignatures") on the bound Response; the URI does NOT replace the Formspec field, it co-occupies its semantic slot.
- `signerAuthority` — a signer-authority claim (§2.12). REQUIRED for any registered intent URI whose §2.11 row sets a non-`self` signer-authority floor. OPTIONAL for `self` floors; when present it MUST validate against §2.12.

If consent evidence is missing, identity binding is below the role's required policy, the signing-intent URI is unregistered for the deployment, the signer-authority floor for the URI is not met, or the Formspec response is invalid, the runtime MUST NOT emit `SignatureAffirmation`.

**Binding to Trellis ADR 0010.** The byte-level proof inside a `SignatureAffirmation` is one `UserContentAttestationPayload` per signer/document pair, encoded under the Trellis `trellis.user-content-attestation.v1` event extension (Trellis ADR 0010 §"Wire shape"). The `SignatureAffirmation.signingIntent` URI MUST equal the `signing_intent` field of the corresponding `UserContentAttestationPayload`. The `documentHash` MUST equal the host event's `canonical_event_hash` carried by the payload's `attested_event_hash`. The `signedAt` MUST equal the payload's `attested_at`, which per ADR 0010 MUST equal the envelope's `authored_at`. Field-level binding is normative: a `SignatureAffirmation` whose payload disagrees with these equalities MUST NOT be admitted by `custodyHook`.

### 2.9 Decline, Void, Expiry, and Reassignment

A decline MUST record signer, reason, timestamp, and document. It MUST follow the configured decline transition.

A void MUST cancel pending signing steps and record the authorizing actor and reason.

An expiry MUST be driven by a typed kernel timer event and MUST record pending signers and expired documents.

A reassignment MUST record original signer, new signer, authorizing actor, timestamp, and reason. Reassignment MUST NOT erase accountability for the original assignment.

### 2.10 Witness, Counter-Signature, Notary, and In-Person Signing

A witness or counter-signature step MUST depend on the primary signer affirmation unless the profile explicitly declares another dependency.

A notary or in-person signer role MUST require an authentication policy whose method is `in-person`, `notary`, or an `x-*` method that declares equivalent in-person evidence.

### 2.11 Signing-Intent URI Registry

Every `SignatureAffirmation` carries a `signingIntent` URI naming its legal-effect class. Trellis ADR 0010 owns the byte-level URI shape (`signing_intent: tstr`, RFC 3986 syntactic check at the byte verifier). WOS Signature Profile owns the URI's *meaning*: which intent URIs the profile recognizes, what each one claims, and what authentication-method floor and signer-authority floor (§2.12) each one requires under each jurisdictional posture (§2.13).

#### 2.11.1 Registered URIs (initial set)

The profile registers the following intent URIs. The set is **append-only**: removing a URI is a breaking profile change; adding a URI is additive (§3.5).

| Intent URI | Meaning | Authentication-method floor (general) | Signer-authority floor (§2.12) |
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

The "general" floor is the baseline floor when no jurisdictional posture is declared. Jurisdiction-specific floors per §2.13 MAY raise (never lower) the floor for a given intent URI.

#### 2.11.2 Deployment-local URIs

Deployments MAY register additional intent URIs in their Posture Declaration (Trellis Operational Companion §"Posture Declaration"). A deployment-local URI MUST:

- use a deployment-scoped URI namespace distinct from `urn:wos:signing-intent:*` (which is reserved for this profile);
- declare its meaning, authentication-method floor, and signer-authority floor in the Posture Declaration;
- declare its mapping under each jurisdictional posture the deployment claims (§2.13);
- pass the same byte-level RFC 3986 check at the Trellis verifier per ADR 0010 step 6d.2.

A Signature Profile Runtime MUST reject a `SignatureAffirmation` whose `signingIntent` is neither in §2.11.1 nor in the deployment's Posture Declaration registry. Trellis admits any well-formed URI at the byte layer; semantic gating happens here.

#### 2.11.3 URI propagation contract

The same URI string traverses three layers. Each layer's verifier MUST observe byte equality with its source:

1. **Formspec Response.** `authoredSignatures[*].signingIntent` (Formspec Core §2.1.6) carries the URI authored by the signer.
2. **WOS `SignatureAffirmation`.** `signingIntent` MUST equal the corresponding Formspec `authoredSignatures[*].signingIntent`.
3. **Trellis `UserContentAttestationPayload`.** `signing_intent` MUST equal the WOS `signingIntent`.

A divergence at any boundary fails the affirmation. WOS lint enforces (1)↔(2); WOS runtime enforces (2)↔(3) at `custodyHook` admission; Trellis verifier enforces (3) is syntactically a URI per RFC 3986.

#### 2.11.4 Open question — notarial commission credential format

The `notarial-attestation` URI requires a notary commission as the authority-source (§2.12). Commission credential format varies by jurisdiction (state-issued certificate, registry lookup URI, x.509 cert with notarial extension, deployment-issued bearer credential). The §2.12 `authoritySource` field admits any URI; deployment-side registration of which commission registries are accepted in which jurisdictional posture is the deployment's responsibility, not this profile's. A future profile revision MAY add a `notaryCommissionFormat` enum once jurisdiction-specific patterns settle.

### 2.12 Signer-Authority Claim

The signer-authority claim declares **capacity to bind** — in what role and on whose behalf the signer acted. It is distinct from §2.6 identity binding (which is identity-strength). Both are independent gates: a strong identity claim (`webauthn`) does NOT establish authority to bind a third party; a weak identity claim (`email-otp`) is not rescued by an authority claim.

#### 2.12.1 Claim shape

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

#### 2.12.2 Class semantics

| `class` | Meaning | `principal` | `authoritySource` content |
|---|---|---|---|
| `self` | Signer acts on their own behalf only. | MUST be omitted or equal `signerId`. | OPTIONAL; if present, identifies the signer's identity record. |
| `as-officer-of` | Signer acts as an officer or authorized agent of an organization. | REQUIRED — URI of the organization. | URI of the appointment / authorization instrument (board resolution, employment record, agency registration). |
| `as-attorney-in-fact` | Signer acts under a power of attorney for a natural or legal person. | REQUIRED — URI of the principal granting the power. | URI of the executed power-of-attorney instrument. |
| `notary-commissioned` | Signer acts as a commissioned notary or jurisdictional equivalent. | OPTIONAL — URI of the affiant whose act the notary attests. | REQUIRED — URI of the notarial commission. |
| `witness` | Signer acts as a witness to another party's signing act. | OPTIONAL — URI of the principal whose act is witnessed. | OPTIONAL; if present, identifies the witness's identity record. |
| `x-*` | Vendor or deployment-defined authority class. | Per Posture Declaration. | Per Posture Declaration. |

The class enum is closed at the WOS center. Deployments add `x-*` classes through the Posture Declaration; admission rules and floor mapping (§2.13) for `x-*` classes MUST be declared per deployment.

#### 2.12.3 Evidence binding

`evidenceBinding.evidenceHash` MUST be the digest of the canonical supporting evidence (commission certificate bytes, executed power of attorney bytes, board resolution bytes). `sha-256` is REQUIRED for Core conformance. The hash is what binds the claim to its proof; the optional `evidenceLocation` URI is for retrieval, not for trust.

When a deployment carries the supporting evidence as a chained event (e.g., a notary commission registered as its own ledger event), `evidenceLocation` MAY be the `canonical_event_hash` URI of that event. The verifier resolves the URI; trust derives from the chain's integrity, not from the URI's resolvability at verification time.

#### 2.12.4 Runtime obligations

A Signature Profile Runtime, when admitting a `SignatureAffirmation`, MUST:

1. Resolve the `signingIntent` URI against §2.11 (or the Posture Declaration registry).
2. If the resolved URI's signer-authority floor is anything other than `self`, REQUIRE a non-omitted `signerAuthority` claim and confirm `signerAuthority.class` matches the floor (or is a stricter class as ranked in §2.12.5).
3. Confirm `authoritySource` is present when REQUIRED for the class.
4. Confirm `principal` is present when REQUIRED for the class and that it is not equal to `signerId` (a signer cannot be their own principal in a delegating class).
5. Confirm `evidenceBinding.evidenceHash` is present and uses an algorithm permitted by §2.7.
6. If `validFrom` / `validUntil` are present, confirm `signedAt` falls within the window.
7. If any check fails, MUST NOT emit `SignatureAffirmation` and MUST record the failure reason in the runtime's diagnostic stream.

#### 2.12.5 Authority class strength ordering

For floor-matching purposes, the classes are ordered weakest → strongest:

`self` → `witness` → `as-officer-of` ≈ `as-attorney-in-fact` → `notary-commissioned`

A claim of strength ≥ floor satisfies the floor. `as-officer-of` and `as-attorney-in-fact` are unordered relative to each other; either satisfies a floor expressed as either. `x-*` classes are unordered relative to the closed set; the Posture Declaration MUST place each `x-*` class explicitly relative to a registered class for floor-matching purposes.

### 2.13 Jurisdictional Posture Mapping

A deployment declares zero or more jurisdictional postures in its Posture Declaration. Postures the profile recognizes:

- **`general`** — no jurisdictional claim. The default. Floors are the §2.11.1 "Authentication-method floor (general)" column.
- **`esign`** — U.S. Electronic Signatures in Global and National Commerce Act (15 U.S.C. ch. 96).
- **`ueta`** — U.S. Uniform Electronic Transactions Act (state-enacted; floors apply where the deployment claims a UETA-adopting jurisdiction).
- **`eidas`** — EU Regulation 910/2014 (electronic IDentification, Authentication and trust Services).

A deployment MAY claim multiple postures simultaneously (e.g., a U.S. federal-agency deployment claiming both ESIGN and UETA). When multiple postures apply, the **strictest** floor for each (intent URI × authentication-method) and (intent URI × signer-authority) cell wins.

#### 2.13.1 Posture floor matrix

The matrix below SHOULD be read as: "to admit a `SignatureAffirmation` carrying intent URI X under posture Y, the runtime MUST require at least authentication-method floor A and signer-authority floor B." Floors may be raised per deployment; this is the profile's normative minimum.

| Intent URI | `general` auth floor | `esign` auth floor | `ueta` auth floor | `eidas` auth floor | Authority floor (all postures) |
|---|---|---|---|---|---|
| `applicant-signature` | `email-otp` | `email-otp` + ESIGN consent (§2.13.2) | `email-otp` | `oidc` (advanced electronic signature) | `self` |
| `counter-signature` | `email-otp` | `email-otp` + ESIGN consent | `email-otp` | `oidc` | `self` |
| `witness-attestation` | `email-otp` | `email-otp` | `email-otp` | `oidc` | `witness` |
| `notarial-attestation` | `notary` or `in-person` | `notary` or `in-person` | `notary` or `in-person` (RON-permitted states only; deployment declares) | `notary` or `in-person` (qualified electronic signature where required) | `notary-commissioned` |
| `consent` | `email-otp` | `email-otp` + ESIGN consumer consent (§2.13.2) | `email-otp` | `email-otp` (where lawful basis distinct from signature) | `self` |
| `attestation-of-fact` | `email-otp` | `email-otp` + ESIGN consent | `email-otp` | `oidc` | `self` or `as-officer-of` |
| `agent-as-attorney-in-fact` | `oidc` / `webauthn` / `credential` | `oidc` / `webauthn` / `credential` + ESIGN consent | `oidc` / `webauthn` / `credential` | `webauthn` / `credential` (advanced electronic signature) | `as-attorney-in-fact` |
| `agent-as-officer` | `oidc` / `webauthn` / `credential` | `oidc` / `webauthn` / `credential` + ESIGN consent | `oidc` / `webauthn` / `credential` | `webauthn` / `credential` | `as-officer-of` |
| `approval` | `email-otp` | `email-otp` | `email-otp` | `oidc` | `self` or `as-officer-of` |
| `certified-receipt` | `email-otp` | `email-otp` | `email-otp` | `email-otp` | `self` |

The matrix is normative for the registered URIs in §2.11.1. Deployment-local URIs (§2.11.2) MUST declare their own row in the deployment's Posture Declaration.

#### 2.13.2 ESIGN consumer-consent prerequisite

ESIGN posture additionally requires consumer-consent disclosures (15 U.S.C. § 7001(c)) for a `SignatureAffirmation` against a consumer. The profile records this through the existing `consentReference` field (§2.5): under `esign` posture, the runtime MUST verify that `consentReference` resolves to a Formspec consent record whose disclosed-content meets the ESIGN §7001(c)(1) categories (right to receive paper records, withdrawal procedure, hardware/software requirements, scope of consent). The profile does not author the consent text; the deployment authors it and the profile checks the structural reference.

UETA does not impose the §7001(c) consumer-consent prerequisite at the federal level, but state UETA enactments may. Deployments claiming UETA in jurisdictions with such state-level prerequisites MUST raise the floor accordingly in their Posture Declaration.

#### 2.13.3 eIDAS signature-tier mapping

eIDAS distinguishes three signature tiers: *simple*, *advanced* (AdES), *qualified* (QES). The matrix above maps `oidc` / `webauthn` / `credential` floors to advanced electronic signature; `notary` to qualified-equivalent under deployment-specific qualified trust service provider integration. A deployment claiming QES for any URI MUST register the qualified trust service provider in its Posture Declaration; the profile floor admits AdES baseline for the registered URIs and a deployment may raise to QES per use case.

#### 2.13.4 Posture-declaration responsibility

Per §1.3, the profile authors the structural mapping; the deployment authors the legal claim. A deployment claiming `esign` posture in its Posture Declaration commits to:

- meeting the §2.13 floors for every URI it admits;
- carrying ESIGN §7001(c) consumer-consent disclosures where the affirmation is against a consumer;
- counsel review per parent PLN-0355 before commercial-mode procurement claims.

The Trellis Operational Companion's Posture Declaration enforces the structural side; counsel-pinned legal sufficiency is gated downstream.

---

## 3. Composition

### 3.1 Attachment Point

The Signature Profile is a profile document. It attaches to a kernel workflow by `targetWorkflow` and composes with governance, AI, and advanced governance documents through the existing kernel seams.

### 3.2 Formspec Composition

Formspec captures signature controls, consent controls, identity-proofing references, and canonical response fields. WOS consumes those fields as evidence inputs. WOS MUST NOT infer a valid signing act from fields that failed Formspec validation.

### 3.3 Trellis Composition

WOS emits `SignatureAffirmation` records through `custodyHook`. Trellis anchors the WOS evidence record and owns certificate-of-completion and export-bundle composition. WOS MUST NOT place Trellis-owned chain fields inside the authored signature record.

### 3.4 Conflict Handling

Profile-to-kernel reference failures are load-time errors. Missing optional policy blocks use the profile defaults defined by the schema and runtime. Conflicting role or step declarations reject; processors MUST NOT merge duplicate ids by declaration order.

### 3.5 Versioning

Changing signer-role enums, flow semantics, authentication-method semantics, or `SignatureAffirmation` required fields is a breaking profile change. Adding a new optional policy block or `x-*` extension is additive.

---

## 4. Conformance

### 4.1 Conformance Classes

**Signature Profile Document.** A JSON document conforming to this specification and the Signature Profile JSON Schema.

**Signature Profile Processor.** A processor that loads Signature Profile Documents, resolves profile references, and rejects invalid documents.

**Signature Profile Runtime.** A runtime that executes signing flow semantics and emits `SignatureAffirmation` provenance records.

### 4.2 Conformance Profiles

| Profile | Requirements |
|---|---|
| Core | Single, sequential, and parallel signing; consent; identity binding; document binding; `SignatureAffirmation` provenance. |
| Complete | Core, plus routed and free-for-all flows; reminders; expiry; decline; void; reassignment; witness; notary; in-person signer; certified recipient. |

Complete is a strict superset of Core.

### 4.3 Verification

Schema validation checks the document shape, closed enums, URI/reference field shapes, and `x-*` extension discipline.

Lint checks profile-to-kernel consistency: target workflow resolution, actor resolution, human actor binding, authentication-policy key resolution, role/document/step references, dependency cycles, FEL guard parsing, timer-event mapping, and ADR-0060 naming.

Runtime conformance checks signing behavior: sequential blocking, parallel completion, routed guard selection, expiry timers, decline paths, reassignment accountability, witness dependencies, notary/in-person authentication, missing-consent rejection, and custody append inclusion.
