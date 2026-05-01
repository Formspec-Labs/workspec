# Studio Spec: Review and Approval

**Status:** draft (Stage 2 of [Implementation Roadmap](../VISION.md#17-implementation-roadmap))
**Date:** 2026-04-30
**Concept-model anchor:** [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.10 ReviewerResolution, §1.15 ApprovalDecision, §1.16 PublishedWorkflowPackage.
**PRD anchor:** [`../VISION.md`](../VISION.md) §9.9 (Review, Governance, and Approval), §16 Phase-2 Epic 2.4.
**Depends on:** [`policy-object-model.md`](policy-object-model.md), [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md), [`readiness-validation.md`](readiness-validation.md), [`scenario-authoring.md`](scenario-authoring.md), [`authoring-provenance.md`](authoring-provenance.md).

## Scope

Review and Approval is the **trust and accountability** layer of Studio. It supports organizational sign-off across legal, compliance, operations, and technical reviewers; it makes reviewer decisions and the rationale behind them durable; and it gates publication on findings + reviewer presence (PRD §9.9).

This spec defines:

- the six review levels (source / extracted object / mapping / workflow / scenario / conflict) and their valid review states;
- the ApprovalDecision and ReviewerComment data shapes;
- the ReviewerRole model and assignment rules;
- the **approval package** that aggregates everything reviewers signed off on and projects into the published artifact;
- the **publication gate**: what reviewer presence + finding state is required before a workflow can ship;
- the audit-trail integration with [`authoring-provenance.md`](authoring-provenance.md);
- composition and conformance.

## Out of scope

- Identity / authentication (a deployment concern; the spec defines the *role* surface, not how identity is established).
- The comment-thread UX.
- Cross-organization review delegation (deferred — see Open Issues).

## Terminology

- **Review level** — one of source / extracted-object / mapping / workflow / scenario / conflict (PRD §9.9 table).
- **Reviewer role** — the workspace-defined hat a reviewer wears: workflow-owner, legal, compliance, operations, technical, governance, executive, etc. The set of roles is workspace-configurable; some roles carry override authority.
- **ApprovalDecision** — the durable signed-off record (CM §1.15).
- **ReviewerResolution** — the durable conflict/assumption/open-question resolution (CM §1.10). Used during review; sometimes accompanies an ApprovalDecision.
- **ReviewerComment** — a non-normative thread message on a subject; not a decision.
- **Approval package** — the aggregated record at publication time: ApprovalDecisions, observed findings, citation manifest, scenario suite, release notes.
- **Override authority** — a workspace-policy attribute granting a role the ability to waive `block`-severity findings or approve `unmappedButApproved` mappings.

## Data model

### `ApprovalDecision` (CM §1.15, extended)

```text
ApprovalDecision {
  id, subjectKind, subjectRef,
  reviewerId, reviewerRole, decision (approved|rejected|approved-with-conditions),
  conditions[], observedFindings[],
  rationale,
  decidedAt, signatureRef?,
  workspaceId, version
}
```

`observedFindings[]` is **load-bearing**: it lists the ValidationFinding ids the reviewer reviewed (whether or not they were waived). This is what an auditor looks at to ask "what did this reviewer actually see at the time of approval?"

### `ReviewerComment`

```text
ReviewerComment {
  id, subjectKind, subjectRef, parentCommentId?,
  authorId, authorRole, body,
  createdAt, editedAt?,
  resolved (boolean), resolvedBy?, resolvedAt?,
  workspaceId
}
```

Comments are conversational; they do not gate advance. They feed into authoring provenance only when explicitly converted to a ReviewerResolution.

### `ReviewerRole`

The ReviewerRole entity is **defined in [`workspace.md`](workspace.md)** §"Data model" — the role registry is workspace-level state, not approval-level state. Repeated here for reference:

```text
ReviewerRole {
  id (workspace-scoped), name, displayName,
  responsibilities[],         // which review levels this role is competent for
  hasOverrideAuthority (boolean),
  requiredForPublication (boolean),  // if true, the publication gate requires at least one ApprovalDecision by a reviewer in this role
  workspaceId
}
```

The default role set, integrity rules (`SA-MUST-ws-010`–`013`), and the no-id-reuse-after-retirement constraint live in [`workspace.md`](workspace.md). This spec consumes them; it does not define them.

### `ApprovalPackage`

```text
ApprovalPackage {
  id, workflowIntentRef, workflowVersion,
  wosVersionPin,              // claim string per parent RELEASE-STREAMS.md (per CM §1.33 MigrationPath); gates re-compilation if streams deprecate
  approvals[],                // every ApprovalDecision contributing to publication
  citationManifest,           // every SourceVersion cited by any approved object (compact form)
  scenarioSuiteRef[],
  validationReportRef,        // snapshot of ValidationFindings at publication time
  releaseNotes,
  unmappedListings[],         // every unmappedButApproved mapping per `SA-MUST-map-042`
  waivedFindings[],           // every waived finding per readiness-validation `SA-MUST-rv-023`
  complianceAttestations[],   // ComplianceAttestation per CM §1.28; one per declared regime
  identitySigningKeyRefs[],   // public-key references per identity-and-attestation.md `SA-MUST-id-021`; for downstream verifiers to validate signatures
  custodyAnchorReceipt,       // CustodyAppendReceipt covering this package per parent ADR-0061; cryptographically anchors the package
  publishedBy, publishedAt,
  workspaceId
}
```

The ApprovalPackage IS the audit record at publication. It MUST be reproducible from the workspace state at the publication timestamp AND independently verifiable via the custody-anchor receipt.

### `ComplianceAttestation` (per CM §1.28)

Attached to the ApprovalPackage; one entry per regime the workspace claims:

```text
ComplianceAttestation {
  regime ('SOC2-Type-II' | 'FedRAMP-Moderate' | 'FedRAMP-High' | 'StateRAMP-Moderate' | 'StateRAMP-High' | 'NIST-800-53-Rev5' | 'HIPAA' | 'GDPR-DPIA' | 'CCPA' | ...),
  regimeVersion,
  controls[] {                // controls satisfied by THIS workflow (subset of workspace baseline)
    controlId, controlName,
    status ('met' | 'partially-met' | 'not-applicable' | 'compensating-control'),
    evidenceRef,              // pointer to readiness findings, scenario passes, signature events, etc.
    attestor                  // SubjectId per identity-and-attestation.md
  },
  attestedAt, expiresAt?,
  auditorRef?                 // when an external auditor signs off
}
```

When a workspace declares a compliance baseline (per `workspace.md` `WorkspaceComplianceBaseline`), publication MUST emit a ComplianceAttestation per regime. The compiler derives this from workspace baseline + workflow's actually-met controls.

## Review levels

The six review levels (PRD §9.9 table) and their valid review states:

| Level | Valid review states | Notes |
|---|---|---|
| **source** | `imported` → `current` → `superseded` | Lifecycle of the SourceDocument itself; not an approval decision per se but a curation state |
| **extracted-object** | `draft` → `approved` / `rejected` | Promotion to PolicyObject is the "approved" path; rejected is terminal |
| **mapping** | `unmapped` → `mapped` → `approved` | Mapping declaration → reviewed → approved |
| **workflow** | `draft` → `reviewed` → `approved` → `published` | The host WorkflowIntent; matches CM §2.4 |
| **scenario** | `generated` → `reviewed` → `passing` / `failing` / `acceptedAsKnownGap` | Matches CM §2.5 |
| **conflict** | `unresolved` → `resolved` / `waived` | Per CM §1.9 |

Each review level carries its own ApprovalDecisions; the publication gate aggregates across all six.

## Lifecycle

ApprovalDecisions are **durable** — once recorded, they cannot be deleted. The set of valid lifecycle transitions on a single ApprovalDecision:

```text
recorded → { active | superseded | revoked }
```

- `recorded`: the decision is fresh.
- `active`: the decision is the currently-applicable approval for the subject (workspace state has not invalidated it).
- `superseded`: a later ApprovalDecision (often after subject edits requiring re-approval) replaces this one.
- `revoked`: the reviewer or another with override authority has explicitly retracted the decision (rare; emits its own AuthoringProvenanceRecord).

Allowed transitions:

| From | To | Trigger |
|---|---|---|
| `recorded` | `active` | the subject has not been edited; this is the current approval |
| `active` | `superseded` | a new ApprovalDecision on the same subject + workspace + reviewer-role |
| `active` | `revoked` | explicit retraction by reviewer or override |
| `recorded` | `revoked` | a fresh decision is retracted before becoming active (rare) |

Subject-edit semantics: when the subject of an ApprovalDecision is edited (e.g., a PolicyObject body is modified after approval), the existing decision moves to `superseded` and the subject lifecycle is demoted (per [`policy-object-model.md`](policy-object-model.md) `SA-MUST-pom-022`). A fresh ApprovalDecision is required.

ReviewerComments have no lifecycle beyond `created → resolved (boolean) → archived (workspace policy)`.

## Normative Contract

### Reviewer assignment and roles

- **`SA-MUST-ra-001`** — Every workspace MUST define a non-empty set of ReviewerRoles. The set MUST include at least one role with `hasOverrideAuthority = true` and at least one role with `requiredForPublication = true`. *(schema-pending: required-cardinality.)*
- **`SA-MUST-ra-002`** — Every ApprovalDecision MUST carry a `reviewerRole` from the workspace's defined set. Decisions with unknown roles MUST be rejected at creation. *(schema-pending: cross-reference.)*
- **`SA-MUST-ra-003`** — A reviewer MAY hold multiple roles in a single workspace; ApprovalDecisions identify the role under which the decision was made (a reviewer cannot accidentally use their compliance authority to approve a technical decision unless they explicitly select that role). *(runtime-pending: role-selection at decision time.)*
- **`SA-MUST-ra-004`** — Self-approval prohibition: a reviewer MUST NOT approve a subject they themselves authored, except for trivial subjects (workspace policy decides; default = no exception). *(lint-pending: tier-S6 readiness rule.)*
- **`SA-MUST-ra-005`** — Cross-role gating: when a workspace policy designates that a particular subject kind requires multiple roles' approvals (e.g., rights-impacting workflows require both legal and compliance), the publication gate MUST enforce all required roles. *(lint-pending; workspace policy schema.)*

### Comments and resolution

- **`SA-MUST-ra-010`** — ReviewerComments MUST NOT be deleted; they MAY be marked resolved or archived. *(schema-pending: immutable; runtime-pending.)*
- **`SA-MUST-ra-011`** — Edited comments MUST preserve the edit history (the body field carries an edit-trail or the implementation maintains a separate edit log). *(runtime-pending.)*
- **`SA-MUST-ra-012`** — A comment may reference a finding, mapping, scenario, or any subject; subjects MUST be valid workspace references. Dangling references MUST be rejected. *(runtime-pending: foreign-key check.)*
- **`SA-SHOULD-ra-013`** — When a comment thread leads to a ReviewerResolution (CM §1.10), the resolution SHOULD reference the originating thread; this preserves the conversational context behind a structured decision.

### ApprovalDecision integrity

- **`SA-MUST-ra-020`** — ApprovalDecisions MUST NOT be deleted. Retraction is a separate `revoked` lifecycle state. *(schema-pending: immutable.)*
- **`SA-MUST-ra-021`** — Every ApprovalDecision MUST carry `observedFindings[]` listing the ValidationFinding ids the reviewer reviewed at decision time, including waived ones. Empty `observedFindings[]` is valid only when the workspace state had no findings on the subject. *(schema-pending; runtime-pending: snapshot at decision time.)*
- **`SA-MUST-ra-022`** — `decision = approved-with-conditions` MUST carry a non-empty `conditions[]`. Each condition is a structured statement (rule + criterion + deadline) that must be satisfied for the approval to remain valid. Failure to satisfy a condition transitions the decision from `active` to `superseded` and triggers a tier-S6 finding. *(schema-pending: required-when; runtime-pending: condition tracking.)*
- **`SA-MUST-ra-023`** — Every ApprovalDecision MUST emit an AuthoringProvenanceRecord with `eventKind = approved` (or `rejected` / `revoked`) per [`authoring-provenance.md`](authoring-provenance.md) `SA-MUST-prov-001`. *(runtime-pending.)*
- **`SA-SHOULD-ra-024`** — High-stakes decisions (approval of `requiresSpecExtension` mappings, waiver of `block`-severity findings, publication of rights-impacting workflows) SHOULD carry a `signatureRef` — a binding signature emission per [`../../specs/profiles/signature.md`](../../specs/profiles/signature.md) and the WOS `signature` embedded block. The spec is permissive about *which* decisions require a signature; workspace policy decides.

### Approval package

- **`SA-MUST-ra-030`** — On publication, the implementation MUST construct an ApprovalPackage that includes: every `active` ApprovalDecision contributing to the workflow (workflow-level + every PolicyObject + every Mapping + every Scenario reviewed for this version); the citation manifest aggregating all SourceCitations; the scenario suite reference; the validation report snapshot; release notes; and the lists of unmapped mappings and waived findings. *(runtime-pending: compiler emission.)*
- **`SA-MUST-ra-031`** — The ApprovalPackage MUST be reproducible from the workspace state at the publication timestamp — given the same {workspace state, registry version, compiler version}, the package's content MUST be identical. *(fixture-pending.)*
- **`SA-MUST-ra-032`** — The ApprovalPackage MUST be stored alongside the published `$wosWorkflow` artifact and referenced from the [`authoring-provenance.md`](authoring-provenance.md) `published` provenance record. *(runtime-pending.)*
- **`SA-SHOULD-ra-033`** — Workspaces SHOULD render the ApprovalPackage as a human-readable summary at publication time, suitable for inclusion in a release announcement.
- **`SA-MUST-ra-034`** — When a Workspace declares a compliance baseline (per `workspace.md` `WorkspaceComplianceBaseline`), the ApprovalPackage MUST include a ComplianceAttestation per declared regime. Workflows that fail to satisfy any required-control for any declared regime MUST NOT publish (tier-S6 finding `COMP-LINT-001` per `workspace.md` `SA-MUST-ws-060`). *(lint-pending; runtime-pending.)*
- **`SA-MUST-ra-035`** — The ApprovalPackage MUST be cryptographically anchored: `custodyAnchorReceipt` MUST be a valid `CustodyAppendReceipt` per parent ADR-0061 covering the package's content hash. External verifiers without workspace access can validate the package's authenticity via the receipt + parent Trellis verifier. (Cross-cutting [`authoring-provenance.md`](authoring-provenance.md) `SA-MUST-prov-081`.) *(runtime-pending.)*
- **`SA-MUST-ra-036`** — `wosVersionPin` MUST be present in the ApprovalPackage (cross-cutting `compiler-contract.md` `SA-MUST-cmp-050`). On parent stream deprecation per `change-impact.md` `triggerKind = wos-version-deprecation`, the pin gates which package versions need migration. *(schema-pending.)*

### Key rotation handling

- **`SA-MUST-ra-040-keys`** — When an identity issuer rotates a signing key (per [`identity-and-attestation.md`](identity-and-attestation.md) `SA-MUST-id-022`), in-flight ApprovalDecisions referencing the rotated key MUST be re-verifiable against the new key via standard rotation chain semantics (issuer-published verification keys with rotation timestamps). The implementation MUST NOT invalidate prior decisions on rotation — they were valid at the time of decision; rotation is forward-going. *(runtime-pending.)*
- **`SA-MUST-ra-041-keys`** — The ApprovalPackage's `identitySigningKeyRefs[]` MUST include the public-key reference active at decision-time, NOT the key currently in force. Downstream verifiers resolve the historical key via the issuer's rotation chain. *(schema-pending; runtime-pending.)*
- **`SA-MUST-ra-042-keys`** — When a key is REVOKED (not rotated; revocation indicates compromise), the implementation MUST surface a tier-S6 finding (`ID-LINT-002` per `identity-and-attestation.md`) for every PublishedWorkflowPackage whose ApprovalDecisions reference the revoked key. Re-attestation by uncompromised subjects MUST be required to keep the workflow `published`. *(runtime-pending.)*

### Publication gate

The publication gate is the **most consequential check** in Studio. It is the boundary between workspace state and published state ([`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §4 cross-boundary rules).

- **`SA-MUST-ra-040`** — A WorkflowIntent MUST NOT advance from `approved → published` unless: (a) every ReviewerRole with `requiredForPublication = true` has at least one `active` ApprovalDecision on the workflow; (b) no tier-S6 finding has severity `error` or `block` and lifecycle `open` or `acknowledged` (per [`readiness-validation.md`](readiness-validation.md) `SA-MUST-rv-042`); (c) the compiled `$wosWorkflow` passes WOS schema/lint/conformance checks. *(lint-pending: tier-S6 rule; runtime-pending: gate enforcement.)*
- **`SA-MUST-ra-041`** — When the publication gate denies advance, the implementation MUST emit a structured denial record listing exactly which conditions failed and which reviewer roles or findings remain. *(runtime-pending.)*
- **`SA-MUST-ra-042`** — Override of the publication gate is **not allowed** — even override-authority roles cannot publish a workflow with unresolved `block` findings or missing required reviewer roles. The only path through is to resolve, waive, or assign the missing pieces. *(runtime-pending: hard gate.)*
- **`SA-SHOULD-ra-043`** — Workspaces SHOULD provide a "publication readiness" preview that shows the gate status without attempting publication, so reviewers can see what's missing without producing a denial record.

### Block-or-warn semantics

- **`SA-MUST-ra-050`** — At review levels that gate downstream advance (mapping, workflow, scenario), an open critical (`error` or `block`) finding MUST be either resolved or waived before approval can be recorded. The reviewer MUST NOT approve through a critical open finding. *(lint-pending: tier-S6 cross-cutting.)*
- **`SA-MUST-ra-051`** — `warn`-severity findings do NOT block approval but MUST be visible to the reviewer at decision time, listed in `observedFindings[]` per `SA-MUST-ra-021`. *(runtime-pending.)*
- **`SA-MUST-ra-052`** — Waiver of a finding (per [`readiness-validation.md`](readiness-validation.md) `SA-MUST-rv-030`) does NOT itself approve the subject; a separate ApprovalDecision is still required. The waiver clears the gate; the approval makes the affirmative decision. *(runtime-pending.)*

## Composition

### Attachment point

Review and Approval attaches at every reviewable subject in the workspace: SourceDocument, ExtractedClaim, PolicyObject, StudioToWosMapping, WorkflowIntent, Scenario, Conflict, ChangeImpactReport. The role registry attaches at the workspace.

The publication gate attaches at the WorkflowIntent → PublishedWorkflowPackage transition. It is the **single point** at which all upstream specs converge:

- [`source-vault.md`](source-vault.md) supplies citations and supersession state;
- [`policy-object-model.md`](policy-object-model.md) supplies approved PolicyObjects;
- [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md) supplies mappings and ExtensionRecords;
- [`readiness-validation.md`](readiness-validation.md) supplies findings;
- [`scenario-authoring.md`](scenario-authoring.md) supplies scenario passing/known-gap state;
- [`authoring-provenance.md`](authoring-provenance.md) supplies the chain.

### Precedence

When two ApprovalDecisions on the same subject from the same reviewer-role disagree (e.g., one approved, one rejected), the **later** decision (by `decidedAt`) wins; the earlier decision moves to `superseded`. There is no "voting" across reviewers — each role's decision stands on its own; the publication gate aggregates by requiring at least one approval per required role.

When a workspace's role policy changes (e.g., adding a new required role mid-flight), workflows in `approved` lifecycle state do **not** retroactively need the new role's approval; the new requirement applies to subsequent workflow versions. This avoids accidentally invalidating a published workflow.

### Conflict handling

Two reviewers in the same role disagreeing is **not a conflict** for this spec — both decisions are recorded; the later one's role-aggregation effect wins. If the workspace policy requires all-of-N approval (rare, defaults to any-of-N), that is a workspace-policy choice not a spec-default.

A reviewer attempting to approve a subject they authored (`SA-MUST-ra-004`) is rejected at decision-creation time, not as a finding.

### Versioning / migration

- Adding a new ReviewerRole to the workspace policy: **non-breaking** for existing approvals; affects only future required-role gating.
- Marking an existing role `requiredForPublication = true` (when previously false): **breaking** for any workflow not yet published — those workflows now need that role's approval.
- Removing a role: existing ApprovalDecisions under that role remain valid in their `superseded` or `active` state; new decisions cannot be authored under it.
- Changing the `hasOverrideAuthority` flag: governance-significant; should require careful migration with workspace administrator awareness.

## Conformance

### Schema validation (Stage 3)

Planned schema gates:

- ApprovalDecision required fields (`SA-MUST-ra-021` `observedFindings[]`).
- `decision` enum.
- `approved-with-conditions` requires non-empty `conditions[]` (`SA-MUST-ra-022`).
- Immutability of comments and decisions.
- ApprovalPackage shape (`SA-MUST-ra-030`).

### Lint rules (Stage 4)

Tier-S6 readiness rules planned:

- `RA-LINT-001` — every required role has an active ApprovalDecision (cross-cutting `SA-MUST-ra-040`).
- `RA-LINT-002` — no self-approval (`SA-MUST-ra-004`).
- `RA-LINT-003` — `approved-with-conditions` carries unresolved conditions blocks publication.
- `RA-LINT-004` — `observedFindings[]` matches the actual findings on the subject at decision time (catches stale snapshots).
- `RA-LINT-005` — every approved workflow has a corresponding ApprovalPackage at publication.

### Runtime conformance fixtures (Stage 4–5)

- Subject edit demotes prior approval (cross-cutting with [`policy-object-model.md`](policy-object-model.md)).
- Publication gate denial emits a structured denial record listing exact gaps.
- ApprovalPackage is reproducible from workspace state.
- Override authority cannot bypass `block`-severity findings (`SA-MUST-ra-042`).

### Current limitations

- The role registry is workspace-scoped; tenant-level role inheritance is not specified.
- The `approved-with-conditions` condition-tracking subsystem is not yet specified beyond the schema requirement; runtime semantics for condition expiry are deferred.
- Multi-organization review (where reviewers from different organizations participate in the same workspace) is not specified.

## WOS mappings

ApprovalDecisions and the ApprovalPackage have **mixed** mapping states:

| Studio object | Mapping state | WOS path |
|---|---|---|
| ReviewerComment | `authoringOnly` | — (workspace state) |
| ApprovalDecision (full body) | `authoringOnly` | — (workspace state) |
| ApprovalDecision (compact projection) | `mapsToWos` | authoring-provenance projection per [`authoring-provenance.md`](authoring-provenance.md) `SA-MUST-prov-030` |
| ApprovalPackage | `authoringOnly` (storage)+ projected fields | release notes + waivers + unmapped listings appear in published artifact |
| `signatureRef` (when present) | `mapsToWos` | [`../../specs/profiles/signature.md`](../../specs/profiles/signature.md) and `wos-workflow.schema.json` `signature` block |
| ReviewerRole registry | `authoringOnly` | — (workspace state) |
| Override-authority decisions | `mapsToWos` | projected per provenance + listed in release notes |

The publication gate produces no WOS-side construct itself — it's a Studio-side enforcement. But its **outputs** (the ApprovalPackage, the projected ApprovalDecisions, the signature emissions) are the load-bearing audit record that the published artifact carries.

When a workflow uses [`../../specs/profiles/signature.md`](../../specs/profiles/signature.md) (signed approvals binding to the WOS signature embedded block), the signed ApprovalDecision is the bridge between Studio's review system and WOS's `SignatureAffirmation` semantics described in the parent CLAUDE.md.

## Examples

### Example 1: Multi-role gated approval of a rights-impacting workflow

A SNAP-redetermination workflow in a state agency is rights-impacting (multiple `triggersDueProcess = true` Outcomes). The workspace policy requires:

- workflow-owner (override-authority) — required
- compliance-reviewer — required
- legal-reviewer — required (because rights-impacting)
- technical-reviewer — required
- operations-reviewer — required

Reviewer flow:

1. Workflow draft passes tier S1–S5 readiness; reaches `approved` lifecycle state.
2. Workflow owner authors ApprovalDecision (role: workflow-owner; decision: approved). `observedFindings[]` snapshots 0 errors, 3 warnings (waived).
3. Compliance reviewer authors ApprovalDecision (role: compliance-reviewer; decision: approved-with-conditions). Conditions: "Re-evaluate Spanish translation parity at Q4 2026."
4. Legal reviewer authors approval (role: legal-reviewer).
5. Technical reviewer authors approval.
6. Operations reviewer authors approval.
7. All required roles satisfied; tier-S6 readiness passes; the workflow advances to `published`.
8. ApprovalPackage is constructed: 5 ApprovalDecisions, citation manifest, scenario suite, validation report snapshot, release notes, 0 unmapped listings, 3 waived findings.
9. Compliance reviewer's `approved-with-conditions` is tracked: at Q4 2026, a re-evaluation reminder fires; if the condition is unmet by the deadline, the active approval transitions to `superseded` and a tier-S6 finding fires.

### Example 2: Failed publication gate

A workflow is marked `approved` but the technical-reviewer role has no decision yet.

1. Reviewer attempts to publish.
2. Publication gate denies advance with structured denial: `{missingRoles: [technical-reviewer], openCriticalFindings: [], wosValidation: passed}`.
3. The denial is recorded; no PublishedWorkflowPackage is created.
4. Reviewer assigns the technical-reviewer; technical reviewer authors approval.
5. Re-attempt; publication gate passes; workflow advances.

### Example 3: Subject edit invalidates approval

A NoticeRequirement PolicyObject is approved by the compliance reviewer. Two days later, the workflow author edits the NoticeRequirement's `content[]` array to add a new required content element.

1. The PolicyObject is demoted from `approved → draft` per [`policy-object-model.md`](policy-object-model.md) `SA-MUST-pom-022`.
2. The compliance reviewer's prior ApprovalDecision transitions from `active → superseded`.
3. The workflow's lifecycle is also demoted (since one of its referenced PolicyObjects is no longer approved).
4. The compliance reviewer is notified; they re-review the edited NoticeRequirement and author a fresh ApprovalDecision.
5. Workflow advance resumes.

This cascade is what prevents stale approvals from carrying through edits — every approval is tied to the subject as it existed at decision time.

## Open issues

- **Quorum / multi-of-N approval.** The spec assumes any-of-N approval per role; quorum (e.g., "at least 2 of 3 legal reviewers must approve") is not specified.
- **Cross-organization delegation.** A state agency might want to delegate review of certain decisions to a federal counterpart. The role-and-identity model needed to support this cleanly is unsettled.
- **Time-bound approvals.** An approval-with-conditions that has a deadline is sketched; the runtime semantics for deadline-driven re-validation are not yet pinned.
- **Approval revocation policy.** Who may revoke an active approval (only the original reviewer? Override roles? Workspace administrators?) is workspace-policy and not specified normatively here.
- **Comment-thread lifecycle.** The spec sketches `created → resolved → archived`; whether archival is reversible and whether archived comments project into the audit record is unsettled.

## Cross-references

- Concept model: [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.10, §1.15, §1.16, §4 (cross-boundary rules).
- PRD: [`../VISION.md`](../VISION.md) §9.9, §16 Phase-2 Epic 2.4, §12 user stories.
- Upstream: [`policy-object-model.md`](policy-object-model.md), [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md), [`readiness-validation.md`](readiness-validation.md), [`scenario-authoring.md`](scenario-authoring.md), [`authoring-provenance.md`](authoring-provenance.md).
- Downstream: [`change-impact.md`](change-impact.md) (post-publication change → re-approval).
- WOS: [`../../specs/profiles/signature.md`](../../specs/profiles/signature.md) (binding signatures), [`../../specs/governance/workflow-governance.md`](../../specs/governance/workflow-governance.md) (rationale records), `wos-workflow.schema.json` `signature` block.
- Repo conventions: [`../../CONVENTIONS.md`](../../CONVENTIONS.md).
