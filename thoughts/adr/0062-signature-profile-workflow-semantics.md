# ADR-0062: Signature Profile workflow semantics

**Status:** Accepted
**Date:** 2026-04-22
**Deciders:** WOS Working Group
**Author:** WOS-T4
**Supersedes:** None
**Related:**

- [TODO.md](../../TODO.md) -- WOS-T4 Signature Profile workflow semantics
- [T4-TODO.md](../../T4-TODO.md) -- Signature Profile execution plan
- [Signature Profile](../../specs/profiles/signature.md)
- [Signature Profile schema](../../schemas/profiles/wos-signature-profile.schema.json)
- [ADR-0060](0060-cross-reference-naming-ref-key-id.md) -- cross-reference naming
- [ADR-0061](0061-custody-hook-trellis-wire-format.md) -- WOS `custodyHook` wire format
- [STACK.md Open Contracts](../../../STACK.md#open-contracts) -- signature attestation shape and identity attestation shape

---

## 1. Context

The stack has a DocuSign replacement use case: a Formspec response captures signing and consent evidence, WOS governs the signing workflow, and Trellis anchors the resulting evidence bundle. The prior WOS kernel can model lifecycle states, tasks, actors, timers, and provenance, but it does not name the common signing semantics that a processor must implement for signature workflows.

The owner accepted the default parity bar for WOS-T4:

- ESIGN/UETA consent and intent capture.
- eIDAS-compatible identity-binding hooks.
- Sequential, parallel, routed, and free-for-all signing.
- Witness, counter-signature, certified-recipient, notary, in-person signer, approver, form-filler, and viewer roles.
- Reminders, expiry, decline, void, and reassignment/delegation.
- DocuSign administrative UX, legal advice, key custody, rendered-document storage, and cryptographic certificate-of-completion composition are out of scope.

The question is where these semantics live. They are too specific to become kernel actor types or lifecycle primitives. They are too semantic to be delegated to Trellis, which owns anchoring and export-bundle integrity. They are also too important to be hidden in an opaque vendor extension, because conformance must prove signing behavior.

---

## 2. Decision

Define a **Signature Profile** as a parallel WOS profile document.

The WOS center owns:

1. Signer roles and signing-flow semantics.
2. Signer-authentication policy declarations.
3. Intent capture and identity-binding requirements.
4. The `SignatureAffirmation` provenance record shape emitted when a signing act is accepted.
5. Lint and conformance obligations for profile-to-kernel consistency.

Formspec owns the response controls and canonical response fields that capture signature and consent evidence. WOS consumes those fields; WOS MUST NOT reinterpret an invalid Formspec response as valid signing evidence.

Trellis owns `custodyHook` admission, anchoring, offline verification, and certificate-of-completion/export-bundle composition. WOS emits semantic evidence; Trellis preserves it.

Signature ceremony providers are adapters. A deployment may use an embedded form control, a commercial e-sign provider, an in-person notary flow, or another ceremony system, provided the adapter can produce the evidence inputs required by the Signature Profile.

The profile attaches without widening kernel enums:

- Signature Profile is a parallel profile document, not a new kernel document kind.
- Signer roles bind to kernel `human` actors through the existing actor-extension seam.
- Kernel `ActorDeclaration.type` remains `human | system`.
- Lifecycle tags such as `awaiting-signature` are profile semantics over existing state/tag fields, not new state kinds.

ADR-0060 naming applies:

- `*Ref` is used only for URI or cross-artifact references.
- `*Key` is used only for catalog keys, including authentication-policy keys.
- `*Id` is used for sibling id-bearing objects, including role, document, step, and signer identifiers.

---

## 3. Rejected Alternatives

### New kernel actor types

Rejected. Making `signer`, `notary`, `witness`, or `viewer` kernel actor types would move workflow-specific roles into the stable center. Kernel actors remain `human` or `system`; signature roles attach through `actorExtension`.

### Drawn signature image as legal intent

Rejected. A signature image can be evidence, but it is not sufficient by itself. The profile requires explicit consent and affirmation evidence. Legal sufficiency is jurisdiction-specific and outside WOS.

### Trellis defines signing workflow semantics

Rejected. Trellis preserves records and composes export bundles; it does not decide whether a witness dependency, reassignment, or consent step satisfied workflow semantics.

### Opaque vendor extension for signing ceremonies

Rejected. Ceremony providers are adapters, but the evidence WOS accepts is center work. Hiding the whole ceremony behind vendor JSON would make conformance and portable audit impossible.

---

## 4. Consequences

### Positive

- The DocuSign-replacement path has a WOS-owned semantic center.
- Formspec, WOS, and Trellis responsibilities stay separated.
- `SignatureAffirmation.identityBinding` becomes the first concrete identity-attestation shape that can later be generalized for non-signature evidence.
- Signing behavior becomes conformance-testable instead of provider-specific prose.

### Negative

- The profile introduces a new schema, fixtures, lint rules, and runtime conformance surface.
- Runtime must model signing state in addition to ordinary lifecycle state.
- Trellis export-bundle work remains required before the full certificate-of-completion story is complete.

### Neutral

- A deployment can still use a commercial signing provider. The provider is an adapter that supplies evidence inputs.
- The profile does not claim jurisdiction-specific legal sufficiency.

---

## 5. Implementation Notes

WOS-T4 proceeds in center-first order:

1. Signature Profile spec and schema.
2. Profile fixtures and schema tests.
3. Lint rules for profile-to-kernel consistency.
4. `SignatureAffirmation` provenance shape and runtime emission.
5. Runtime signing semantics and conformance fixtures.
6. Formspec and Trellis alignment after the WOS evidence shape is stable.
