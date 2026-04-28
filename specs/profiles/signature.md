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

If consent evidence is missing, identity binding is below the role's required policy, or the Formspec response is invalid, the runtime MUST NOT emit `SignatureAffirmation`.

### 2.9 Decline, Void, Expiry, and Reassignment

A decline MUST record signer, reason, timestamp, and document. It MUST follow the configured decline transition.

A void MUST cancel pending signing steps and record the authorizing actor and reason.

An expiry MUST be driven by a typed kernel timer event and MUST record pending signers and expired documents.

A reassignment MUST record original signer, new signer, authorizing actor, timestamp, and reason. Reassignment MUST NOT erase accountability for the original assignment.

### 2.10 Witness, Counter-Signature, Notary, and In-Person Signing

A witness or counter-signature step MUST depend on the primary signer affirmation unless the profile explicitly declares another dependency.

A notary or in-person signer role MUST require an authentication policy whose method is `in-person`, `notary`, or an `x-*` method that declares equivalent in-person evidence.

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
