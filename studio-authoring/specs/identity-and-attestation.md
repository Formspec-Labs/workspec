# Studio Spec: Identity and Attestation

**Status:** draft (Stage 2 of [Implementation Roadmap](../VISION.md#17-implementation-roadmap)) — **seam contract**; semantics inherit from parent **PLN-0381** as it lands.
**Date:** 2026-04-30
**Concept-model anchor:** [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.27 IdentitySubject; §1.29 AuthorityGrant.
**PRD anchor:** [`../VISION.md`](../VISION.md) §13 (Security and permissions).
**Depends on:** [`workspace.md`](workspace.md), [`authoring-provenance.md`](authoring-provenance.md), [`review-and-approval.md`](review-and-approval.md).

## Why this spec exists

Studio's `workspace.md` says explicitly: "Identity provider integration is a deployment concern; the spec defines the role surface, not the IdP." That's correct for implementation. But it leaves a structural hole at the seam: what crosses the identity boundary — subject claims, role mappings, attestation signing keys, revocation semantics — has no spec home.

Without this seam, multi-tenancy + RBAC + signed-approval will collide later when implementation begins. The Plan-agent review identified this gap directly.

**The composition story:** parent **PLN-0381** ("identity attestation stack ADR — supersedes PLN-0310") was promoted to a P0 WOS-side commitment 2026-04-27 per parent [TODO.md](../TODO.md) synthesis-merge. Parent stack will land the attestation primitive (likely a `$def` in `wos-workflow.schema.json` or a sibling sidecar). Studio's job is **NOT** to define identity attestation primitives. Studio's job is to **bind them at the authoring seam** — every AuthoringProvenanceRecord references an IdentitySubject; every ApprovalDecision references a signed attestation; every authority grant resolves through the seam.

This spec defines the **Studio-side seam contract** and explicitly references parent PLN-0381 for the underlying primitive shape.

## Scope

This spec defines:

- The **IdentitySubject** Studio-side shape (a thin wrapper over the parent attestation primitive).
- The **subject-claims contract** (what claims a subject carries; what Studio reads from them).
- The **role-mapping seam** (how parent-attested roles map to Studio's ReviewerRole registry).
- The **signing-key boundary** (what Studio holds vs. what the IdP / KMS holds).
- The **attestation envelope** (the wire format Studio receives at the seam).
- The **revocation semantics** (how revoked identities affect in-flight authoring state).
- **AuthorityGrant** resolution (how `attestOrigin` / `waive` / `override` / `approve` actions consult the seam).

## Out of scope

- The IdP itself (OIDC, SAML, WebAuthn provider — deployment concern).
- KMS / HSM / WebAuthn-PRF specifics (parent `crates/wos-server/VISION.md` zero-trust posture handles this).
- The attestation primitive's wire format (parent PLN-0381; Studio composes, does not invent).
- Identity provisioning / lifecycle (deployment-environment concern; users are created in the IdP, mirrored to Studio via subject claims).
- Cross-tenant identity federation (deferred to §1.34 federation reservation).

## Terminology

- **IdentitySubject** — Studio's thin wrapper over the parent attestation primitive: a stable identifier for a person, role-bot, or system actor authorized to perform authoring actions in a Workspace.
- **Subject claims** — the set of assertions about a subject (id, display name, email, roles, attestation key references, validity windows, revocation status). Carried in a signed envelope from the IdP / attestation issuer.
- **Attestation envelope** — the cryptographically signed structure carrying subject claims + a reference to the signing key. Format inherited from parent PLN-0381.
- **AuthorityGrant** — a workspace-level grant authorizing a specific role/subject for a specific authoring action (per §1.29 in CONCEPT-MODEL.md).
- **Resolution** — the act of consulting the seam to determine whether a given subject + claims may perform a given authoring action.

## Data model

### `IdentitySubject` (Studio side)

```text
IdentitySubject {
  subjectId,                       // stable across renames; opaque
  displayName,                     // human-readable; from claims, not authoritative
  email?,                          // from claims; informational
  attestationRef,                  // pointer to parent PLN-0381 attestation primitive
  attestationIssuer,               // identifier of the attestation issuer
  validFrom, validUntil?,          // attestation validity window
  revokedAt?, revokedBy?, revocationReason?,
  workspaceMemberships[] {         // workspaces this subject can act in
    workspaceId,
    roleRefs[],                    // resolved ReviewerRole ids per workspace.md
    grantedAt
  },
  bindingScope ('workspace' | 'workflow' | 'object'),
  createdAt
}
```

Studio does NOT generate `subjectId`; it is provided by the parent attestation primitive. The mapping from subject to ReviewerRole(s) is **per-workspace** — the same subject may be `compliance-reviewer` in workspace A and `governance-reviewer` in workspace B.

### `AttestationEnvelope` (received at seam)

The wire format Studio receives at the seam. Inherited from parent PLN-0381 (when ratified); Studio reads but does NOT mutate.

```text
AttestationEnvelope {
  subjectClaims {
    subjectId, displayName, email?,
    issuedAt, validUntil?,
    issuerRef,                     // who issued the attestation
    rolesAttested[],               // raw roles per IdP (not yet mapped to Studio ReviewerRoles)
    attestationLevel               // e.g., 'session' | 'persistent' | 'high-assurance'
  },
  signatureEnvelope {              // per parent PLN-0381 (placeholder until ratified)
    signingKeyRef,
    algorithm,
    signedAt,
    signature                      // bytes
  }
}
```

When parent PLN-0381 ratifies the exact structure, this entry is **non-substantively replaced** with the canonical reference (Studio reads the parent's shape and binds; no Studio-side semantic adjustment).

### `AuthorityGrant` (defined in `workspace.md`; resolution semantics here)

`AuthorityGrant` per §1.29 resolves through the seam at action-time:

```text
canPerform(subject, action, target) :=
  exists Grant g such that:
    g.grantedTo matches subject's attested roles (after role-mapping)
    g.action == action
    g.scope subsumes target
    g.revokedAt is null
    subject's attestation validUntil > now
    subject's revokedAt is null
```

The resolution is reviewer-action-time only; Studio does NOT cache resolved authority across attestation refreshes (see §"Revocation semantics" below).

## Lifecycle

An IdentitySubject lifecycle:

```text
attested → active → { renewed | revoked | expired }
active → renewed                  (parent issues new attestation; subject continues)
active → revoked                  (parent revokes; Studio marks revoked)
active → expired                  (validUntil passes without renewal)
revoked, expired (terminal for the attestation; subjectId may re-attest later)
```

A revoked or expired IdentitySubject's PRIOR ApprovalDecisions and AuthoringProvenanceRecords stand — they were valid at the time of action. Forward authority is denied.

## Normative Contract

### Seam binding

- **`SA-MUST-id-001`** — Every AuthoringProvenanceRecord MUST carry `recordedBy = subjectId`. The subjectId MUST resolve to a known IdentitySubject in the workspace at the time of recording. *(schema-pending: cross-cutting `authoring-provenance.md` `SA-MUST-prov-001`.)*
- **`SA-MUST-id-002`** — Every ApprovalDecision MUST carry `signatureRef` resolving to an AttestationEnvelope at the time of decision. The envelope's `subjectClaims.subjectId` MUST equal the decision's `decidedBy`. *(schema-pending; cross-cutting `review-and-approval.md` `SA-MUST-ra-021`.)*
- **`SA-MUST-id-003`** — Studio MUST NOT generate `subjectId`. `subjectId` is provided by the parent attestation primitive. Forging or fabricating a subjectId Studio-side MUST be impossible by construction (the seam reads, does not write). *(runtime-pending.)*
- **`SA-MUST-id-004`** — When parent PLN-0381 ratifies, Studio MUST update its IdentitySubject and AttestationEnvelope shapes to reference the ratified primitive directly (likely via JSON Schema `$ref`). Studio MUST NOT carry a divergent or wrapping shape long-term. *(coordination-pending.)*

### Subject claims

- **`SA-MUST-id-010`** — Subject claims MUST include `subjectId`, `validUntil` (or sentinel "indefinite"), `attestationLevel`. Claims missing required fields MUST be rejected at seam ingest. *(schema-pending.)*
- **`SA-MUST-id-011`** — Roles attested by the IdP (`rolesAttested[]`) MUST be mapped to workspace ReviewerRole ids via a workspace-policy-configured mapping table. Unmappable roles surface as a tier-S6 ValidationFinding (`ID-LINT-001`); the subject can still act but only via direct AuthorityGrants until a mapping is configured. *(lint-pending.)*
- **`SA-MUST-id-012`** — `attestationLevel` MUST gate sensitive actions: `session`-level attestations MAY perform low-risk authoring (commenting, draft creation); `persistent`-level attestations MAY perform mid-risk authoring (object approval); `high-assurance` attestations are required for publication and `originClass = local-practice` attestation. *(runtime-pending; tier-S6 cross-cutting.)*

### Signing-key boundary

- **`SA-MUST-id-020`** — Studio MUST NOT hold private signing keys. Signing operations MUST cross the seam to the IdP / KMS / HSM. *(architectural commitment; cross-cutting parent `crates/wos-server/VISION.md` zero-trust posture.)*
- **`SA-MUST-id-021`** — Public verification keys MAY be cached workspace-side for offline verification; cache TTL MUST respect parent PLN-0381's key-rotation cadence. *(runtime-pending.)*
- **`SA-MUST-id-022`** — Key-rotation events from the IdP MUST trigger workspace-cache invalidation; in-flight ApprovalDecisions referencing rotated keys MUST be re-verified against the new key (or fail closed). *(runtime-pending.)*

### Revocation semantics

- **`SA-MUST-id-030`** — When an IdentitySubject is revoked: (a) all in-flight authoring actions by that subject after the revocation timestamp MUST be rejected; (b) prior actions stand (they were valid at the time); (c) if the subject's prior approval was a `requiredForPublication` ApprovalDecision and the workflow has not yet published, the implementation MUST surface a tier-S6 finding (`ID-LINT-002`, "required-publication approver revoked") requiring re-approval by another authorized subject. *(runtime-pending.)*
- **`SA-MUST-id-031`** — Revocation MUST be append-only: revoking a subject creates a revocation record; it does NOT alter prior AuthoringProvenanceRecords or ApprovalDecisions. *(schema-pending.)*
- **`SA-MUST-id-032`** — Workspaces MUST publish revocation visibility — every reviewer can see which subjects were revoked when, and which prior actions are affected. *(runtime-pending.)*

### AuthorityGrant resolution

- **`SA-MUST-id-040`** — AuthorityGrant resolution at action-time MUST consult the seam (subject's current attestation + roles + workspace mapping). Cached resolution MUST NOT be used; every action re-resolves. *(runtime-pending.)*
- **`SA-MUST-id-041`** — Workspace owners MUST be authorized to grant / revoke AuthorityGrants. The owner role itself is granted via subject claims at the IdP level (workspace-policy-configurable role-mapping). *(runtime-pending.)*
- **`SA-MUST-id-042`** — Self-grants are disallowed: a subject MUST NOT grant authority to itself. The implementation MUST reject. *(lint-pending.)*

## Composition

### Attachment point

The seam attaches at the Workspace level. Each Workspace configures: (a) which IdP issuer(s) it accepts, (b) the role-mapping table from IdP-attested roles to ReviewerRole ids, (c) the AuthorityGrant catalog.

Cross-workspace identity reuse is **federation** (§1.34, deferred). Today, a subject attested in two workspaces has two IdentitySubject records (one per workspace) sharing the same parent `subjectId`.

### Precedence

Where IdP-attested roles AND direct AuthorityGrants both apply: both are honored; whichever permits the action allows it (most-permissive resolution per `workspace.md` `SA-SHOULD-ws-023`). However: where a direct AuthorityGrant prohibits an action that role-attested authority would permit, the prohibition wins (deny-overrides for explicit grants).

Where a subject's `attestationLevel` does NOT meet the action's required level: the action MUST be rejected, regardless of role/grant.

### Composition with parent attestation primitive

This spec composes (does NOT replace) parent PLN-0381. Coordination posture:

- Until PLN-0381 ratifies: Studio uses the placeholder shape above for AttestationEnvelope; readers should expect a non-substantive shape change post-ratification.
- After PLN-0381 ratifies: Studio's `attestationRef` resolves to a parent attestation primitive instance. The seam this spec defines is the contract; the primitive is the implementation.
- Cross-stack ADR ratification gates the substantive resolution; until then, Studio's lint rules use placeholder shapes that mirror PLN-0381's intent.

### Versioning / migration

- Adding a new `attestationLevel` enum value: schema-breaking; coordinated with parent PLN-0381.
- Changing the role-mapping table format: workspace-policy-configurable; not schema-breaking.
- Changing seam binding rules (`SA-MUST-id-001/002/003`): would force re-keying of every AuthoringProvenanceRecord and ApprovalDecision; do not change post-1.0.

## Conformance

### Schema validation (Stage 3)

- IdentitySubject required fields and lifecycle enum.
- AttestationEnvelope shape (placeholder until parent PLN-0381 ratifies; then `$ref`).
- AuthorityGrant action / scope / grantedTo enums.
- Revocation record shape.

### Lint rules (Stage 4)

Tier-S6 (Publication readiness) and tier-S2 (Policy readiness) rules:

- `ID-LINT-001` — IdP role unmapped to workspace ReviewerRole (subject can act only via direct grants).
- `ID-LINT-002` — required-publication approver revoked before publication; re-approval required.
- `ID-LINT-003` — `attestationLevel` insufficient for action attempted.
- `ID-LINT-004` — self-grant attempted (and rejected at construction; lint reports the attempt for audit visibility).
- `ID-LINT-005` — public verification key cache stale beyond TTL.

### Runtime conformance fixtures (Stage 4–5; substantive testing depends on parent PLN-0381 landing)

- Subject with `session`-level attestation cannot perform publication action.
- Revoked subject's in-flight actions are rejected; prior actions stand.
- Required-publication approver revocation surfaces ID-LINT-002.
- Key rotation invalidates workspace cache; in-flight verifications re-run against new key.
- Self-grant attempt is rejected.

### Current limitations

- AttestationEnvelope shape is placeholder until parent PLN-0381 ratifies.
- Cross-workspace identity reuse (federation, §1.34) is deferred.
- Multi-tenant key isolation is parent's concern (`crates/wos-server/VISION.md` zero-trust); Studio's seam abstracts.

## WOS mappings

IdentitySubject and AuthorityGrant are **`authoringOnly`** as a whole — they are workspace-scoped concerns and never appear directly in `$wosWorkflow`.

| Studio object | Mapping state | WOS path |
|---|---|---|
| IdentitySubject | `authoringOnly` | — (referenced by ApprovalDecision compact projection per `authoring-provenance.md` `SA-MUST-prov-031` — only role + display name, never subjectId or attestation key, project to the artifact) |
| AttestationEnvelope | `authoringOnly` | — (same; only attestationLevel projects to artifact via release-note attestation) |
| AuthorityGrant | `authoringOnly` | — (consulted at action-time only) |

The exception: ApprovalDecisions in the published ApprovalPackage carry compact attestation references — sufficient for downstream verifiers to validate the published workflow's approval chain without workspace access. The minimal projection includes attestation issuer, attested role at decision-time, attestation level, and a verifiable signature reference.

## Examples

### Example 1: Subject acting in two workspaces

A compliance attorney `marcus.williams@dhs.state.gov` is attested by the state IdP. They are mapped to `compliance-reviewer` in workspace `snap-redetermination-2026` and `legal-reviewer` in workspace `tanf-eligibility-2026`.

1. Marcus signs into Studio. The IdP issues an AttestationEnvelope: `{subjectId: "spki:abc...", rolesAttested: ["doh-state-attorney", "doh-state-policy-reviewer"], attestationLevel: "persistent"}`.
2. Workspace `snap-redetermination-2026` maps `doh-state-attorney → compliance-reviewer`. Marcus acts as compliance-reviewer there.
3. Workspace `tanf-eligibility-2026` maps `doh-state-policy-reviewer → legal-reviewer`. Marcus acts as legal-reviewer there.
4. Same subjectId; different ReviewerRole memberships. Both workspaces' audit logs are correct.

### Example 2: Revocation invalidating in-flight publication

Marcus approves a SNAP redetermination workflow as `compliance-reviewer` (one of three required-publication roles). Three days later, Marcus's attestation is revoked (he leaves the agency). The workflow has not yet published.

1. `ID-LINT-002` fires: required-publication approver revoked.
2. The workflow's lifecycleState remains `approved` but cannot advance to `published` until another `compliance-reviewer` re-attests on the workflow.
3. Marcus's prior ApprovalDecision stands (it was valid at the time); the audit log shows revocation does not retroactively invalidate the decision.
4. Re-approval by another compliance-reviewer creates a new ApprovalDecision; the workflow advances to `published`.

### Example 3: Action-time resolution with role mapping

A workspace administrator grants `attestOrigin:local-practice` to the `compliance-reviewer` role. Marcus (compliance-reviewer in this workspace) wants to attest a local-practice claim ("we don't enforce 30-day rule between Christmas and New Year").

1. Marcus invokes the attest action; Studio resolves: subjectId → IdentitySubject → roles in this workspace = [compliance-reviewer]; AuthorityGrant `attestOrigin:local-practice` → granted to `compliance-reviewer` role; attestationLevel = persistent (sufficient).
2. Action permitted. AuthoringProvenanceRecord emitted with `originClass: local-practice`, `recordedBy: subjectId`, attestation reference.
3. The local-practice attestation now has Marcus's identity bound to it; if Marcus's attestation is later revoked, the local-practice claim does NOT auto-invalidate (it was valid at attestation), but post-revocation Marcus cannot make further attestations.

## Open issues

- **Parent PLN-0381 ratification.** Until parent ADR ratifies the exact attestation primitive shape, this spec uses placeholder shapes. Coordination via cross-stack tracking; spec updates when ratification lands.
- **Cross-tenant identity sharing.** Federation (§1.34) deferred; this spec assumes workspace-scoped identity.
- **Step-up authentication.** When a low-attestation-level subject attempts a high-attestation-level action, the user experience SHOULD step them up to high-assurance auth. The seam contract permits this; the UX is product-side.
- **Delegation chains.** A reviewer delegating to a deputy ("Marcus delegates compliance-review on workflow W to Maria for the next 2 weeks") is a valuable capability. Sketched as `bindingScope = workflow` + AuthorityGrant time-boxed; full delegation semantics may want their own spec.
- **Audit visibility of revocation reasons.** Workspace-policy-configurable: how much detail about WHY someone was revoked do other reviewers see?

## Cross-references

- Concept model: [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.27 IdentitySubject; §1.29 AuthorityGrant.
- Cross-cutting: [`workspace.md`](workspace.md) (RBAC + role registry + AuthorityGrant catalog), [`authoring-provenance.md`](authoring-provenance.md) (`recordedBy` resolution), [`review-and-approval.md`](review-and-approval.md) (`signatureRef` resolution).
- Composes with: parent **PLN-0381** (identity attestation stack ADR; supersedes PLN-0310; promoted to P0 2026-04-27 per [TODO.md](../TODO.md)). When ratified, Studio's IdentitySubject and AttestationEnvelope shapes will reference the parent primitive directly via `$ref`.
- References parent **PLN-0384** `wos-event-types.md` for the `wos.identity.*` namespace event tags emitted on revocation, key rotation, and attestation events.
- Repo conventions: [`../../CONVENTIONS.md`](../../CONVENTIONS.md).
