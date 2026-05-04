# ADR-0084: Identity attestation primitive (PLN-0381) — Studio anchor

**Status:** Proposed 2026-05-03
**Date:** 2026-05-03
**Deciders:** WOS Working Group (parent-team ratification pending)
**Author:** Studio authoring layer (I-wave)
**Supersedes:** None
**Amends:** [`studio/specs/identity-and-attestation.md`](../../studio/specs/identity-and-attestation.md) §"AttestationEnvelope" — pins the Studio-side placeholder shape against parent PLN-0381 ratification.

**Related:**

- [`studio/DEFERRED.md`](../../studio/DEFERRED.md) `STUDIO-DEFER-004-COORDINATION` (`SA-MUST-id-004` blocker)
- Parent `PLANNING.md::PLN-0381` (identity attestation primitive — not yet ratified as ADR)
- `SA-MUST-id-004` (closed by this ADR's Studio-side anchor)

---

## 1. Context

Parent `PLN-0381` proposes a unified identity-attestation primitive across the WOS stack: a single `AttestationEnvelope` shape carrying signed attestations about an `IdentitySubject` (human, agent, system) at decision time. Studio currently carries a placeholder `IdentitySubject` + inline attestation fields, but has no canonical envelope shape — it is waiting for parent ratification.

`SA-MUST-id-004` asks Studio's identity model to "compose with PLN-0381's AttestationEnvelope once ratified." Without a Studio-side anchor, `SA-MUST-id-004` stays in `STUDIO-DEFER-004-COORDINATION` (no rule, no schema constraint, no test). The parent PLN has no ADR in `thoughts/adr/`; ratification timing is unknown.

This ADR pins the **Studio-side placeholder shape** so:

1. Studio code (`wos-studio-lint`, `wos-studio-compiler`, `wos-studio-model`) can consume a stable shape.
2. The placeholder is structurally aligned with parent PLN-0381's expected envelope, so when the parent ratifies, the Studio side becomes a `$ref` to the ratified primitive with no breaking change.
3. `SA-MUST-id-004` closes against this Studio anchor, not against parent ratification.

---

## 2. Decision

### 2.1 Studio-side `AttestationEnvelope` shape

```text
AttestationEnvelope {
  id,                           // attestation id (URN)
  subjectRef,                   // IdentitySubject id this envelope attests
  subjectKind,                  // human | agent | system | service-account
  attestationLevel,             // session | persistent | high-assurance
  attestedAt,                   // ISO-8601 date-time of attestation
  attestedBy,                   // attestor id (typically an identity provider)
  attestationMethod,            // webauthn | oidc | mtls | x509 | hardware-token
  validFrom,
  validUntil?,                  // optional; "indefinite" sentinel | date-time | null
  signature: {
    algorithm,                  // ES256 | EdDSA | RS256
    keyId,                      // signer's public key id
    value,                      // base64-encoded signature bytes
    canonicalizationMethod      // jcs | rfc8785
  },
  attestationChain?[],          // recursive AttestationEnvelope refs (delegation chain)
  extensions: { ^x-* }          // vendor-extension bag
}
```

This shape attaches at:

- `IdentitySubject.activeAttestations[]` — the subject's currently-valid envelopes.
- `ApprovalDecision.attestationRef` — the envelope active at decision time (snapshot).
- `AuthoringProvenanceRecord.attestationRef` — the envelope active when the record was emitted.

### 2.2 Studio-MUST: every approved-and-beyond IdentitySubject MUST carry ≥ 1 active AttestationEnvelope

`SA-MUST-id-004` (revised by this ADR): an `IdentitySubject` at `lifecycleState=approved` (or downstream) MUST carry ≥ 1 entry in `activeAttestations[]` whose `validUntil` is null/indefinite OR in the future. Lint rule `ID-LINT-004` enforces the cardinality + temporal validity (J3 close, 2026-05-03; ID-LINT-003 was already taken by an attestationLevel-sufficiency check). Schema enforces shape (planned: `$defs/AttestationEnvelope` in `wos-studio-identity-subject.schema.json`).

### 2.3 PLN-0381 alignment commitment

When parent PLN-0381 ratifies, Studio's `AttestationEnvelope` becomes a `$ref` to the ratified parent primitive. The Studio-side placeholder fields above are an opinionated subset chosen for Studio's operational needs (Stage-3 schema + Stage-4 lint), and are designed to be a strict subset of parent PLN-0381's expected fields:

| Studio-side field | PLN-0381 expected mapping |
|---|---|
| `attestationLevel` | identical (session / persistent / high-assurance) |
| `attestationMethod` | identical (webauthn / oidc / mtls / etc.) |
| `signature` | maps to PLN-0381's signature block (COSE_Sign1 or detached-JWS expected) |
| `attestationChain` | maps to PLN-0381's delegation-graph primitive (when ratified) |

If PLN-0381 ratifies a structurally-different envelope, Studio amends this ADR with a migration path (analogous to ADR-0083's hard-deprecation pattern).

---

## 3. Rejected Alternatives

- **Wait for parent PLN-0381 ratification.** Rejected because `STUDIO-DEFER-004-COORDINATION` has been open with no parent-ADR motion; Studio implementation work cannot proceed without a stable shape anchor.
- **Invent a Studio-only attestation primitive divergent from PLN-0381.** Rejected because the cross-stack interop guarantee in WOS depends on stack-wide identity primitives.
- **Defer all attestation work to `STUDIO-DEFER-007`.** Rejected because basic IdentitySubject readiness (lint rule existence, schema shape, test fixtures) is achievable today with the placeholder; only the parent-ratified primitive is blocked.

---

## 4. Consequences

**Positive.** `SA-MUST-id-004` closes against the Studio-side anchor (no longer in `STUDIO-DEFER-004-COORDINATION`). Studio can author lint rules, schema constraints, test fixtures referring to `AttestationEnvelope` immediately. Parent ratification becomes a one-line `$ref` swap, not a cross-cutting refactor.

**Negative.** If parent PLN-0381 ratifies a structurally-different shape, Studio carries technical debt: the placeholder is referenced in ~12 spec locations (`identity-and-attestation.md`, `authoring-provenance.md`, `review-and-approval.md`, `compiler-contract.md`), in `wos-studio-identity-subject.schema.json` `$defs/AttestationEnvelope` (planned), in lint rule `ID-LINT-004` (J3, cardinality + temporal validity), and in fixtures yet to be authored. The migration is *not analogous* to ADR-0083's hard-deprecation pattern (which had only 3 spec mentions and zero schema/lint/fixture coverage); it is `O(N specs + M fixtures + lint refactor)`.

**Migration playbook (pre-emptive contingencies):**

- **Field rename** (e.g., parent uses `signingKey` instead of `signature{}`): rename across all spec mentions + schema $def + fixture migration; lint predicate untouched if field-access is keyed on the canonical name post-swap.
- **Enum value divergence** (e.g., parent's `attestationLevel` adds `provisional` or drops `session`): treat as breaking change to the Studio enum; emit a one-rev advisory `SA-WARN-id-MIGRATE-ATTESTATION-LEVEL`; bump the schema enum to the parent's set; migrate fixtures with explicit reviewer-attestation that the new enum's mapping is intentional.
- **Required-field addition** (parent mandates `attestationVersion` not in Studio's placeholder): add to schema as required; migrate fixtures; ID-LINT-004 already validates cardinality/temporal — the new required field's check lands as a sibling lint rule (e.g., `ID-LINT-005`).
- **Structural divergence** (parent uses a wrapper envelope or attaches at a different seam): this is the high-cost path — Studio amends ADR-0084 to r2 with full migration narrative analogous to ADR-0083 r2. Cost: ~2 days for spec edits + schema rework + lint refactor + fixture migration; tracked under a new STUDIO-DEFER-008 if parent ratifies before Stage-7.

**Out of scope.**
- Cross-tenant identity federation (parent decision).
- Hardware-attestation key-rotation semantics (parent decision; PLN-0381 §TBD).
- AttestationEnvelope revocation / replay-window semantics (parent decision).

---

## 5. Open questions

1. **Signature canonicalization** (`jcs` vs `rfc8785`): Studio currently exercises `jcs`; PLN-0381 may pin one or the other. Until PLN-0381 ratifies, Studio accepts either.
2. **AttestationChain depth limit**: Studio enforces no maximum today; PLN-0381 may pin a depth bound. Open until parent decides.
3. **`subjectKind = "service-account"`**: Studio includes this for Stage-7 service-mesh integration; PLN-0381 may unify under `system`. Open until parent decides.

---

## 6. Implementation Notes

1. **Spec amendment** — `studio/specs/identity-and-attestation.md` adds § "AttestationEnvelope" with the table above (this ADR's §2.1).
2. **Schema $def** — `wos-studio-identity-subject.schema.json` adds `$defs/AttestationEnvelope` referenced from `IdentitySubject.activeAttestations[]`.
3. **Lint rule** — `ID-LINT-004` (cardinality + temporal-validity check) registered + implemented in `studio/crates/wos-studio-lint/src/rules/publication_readiness.rs::id_lint_004` (J3, 2026-05-03). Note: `ID-LINT-003` already exists with a different semantic (attestationLevel sufficiency for high-stakes actions); a new rule id was minted to avoid the collision.
4. **Migration** — when PLN-0381 ratifies, swap `$ref` to parent primitive; revise this ADR with migration narrative (analogous to ADR-0083 r2).
