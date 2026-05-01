# Studio Spec: Workspace

**Status:** draft (Stage 2 of [Implementation Roadmap](../VISION.md#17-implementation-roadmap))
**Date:** 2026-04-30
**Concept-model anchor:** [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.1 Workspace, §4 State boundaries.
**PRD anchor:** [`../VISION.md`](../VISION.md) §3 (Primary users), §13 (Security and permissions).
**Depends on:** none (this spec is the attachment-point for everything else).

## Scope

Workspace is the **bounded authoring environment** that owns one or more workflows. Every other Studio spec attaches at a Workspace; nothing in Studio exists outside one. This spec defines the Workspace data model, the ReviewerRole registry, the permissions surface, the identity model (at the surface level — implementation deferred), and the audit-log boundary.

This spec consolidates definitions that were previously scattered:

- ReviewerRole registry was buried in [`review-and-approval.md`](review-and-approval.md) §"Data model" — moves here.
- Workspace permissions were referenced by [`source-vault.md`](source-vault.md) `SA-MUST-source-040` — anchored here.
- Audit-log scope was mentioned across half the specs without a single home — defined here.

## Out of scope

- Identity provider integration (a deployment concern; the spec defines the role surface, not the IdP).
- Tenant model / multi-organization (each Workspace is single-tenant in this spec; cross-tenant federation is deferred).
- UI for workspace administration.
- Storage backend choice (workspace state is durable; how it's stored is implementation).

## Terminology

- **Workspace** — the top-level container; one Workspace = one bounded authoring environment.
- **Workspace owner** — the role with administrative authority over the Workspace itself (creating workflows, defining roles, configuring policy).
- **ReviewerRole** — a workspace-defined hat (workflow-owner, compliance-reviewer, legal-reviewer, technical-reviewer, operations-reviewer, …) that participants wear when authoring or reviewing.
- **Override authority** — a flag on a ReviewerRole granting the power to waive `block`-severity findings or approve `unmappedButApproved` mappings.
- **Audit log** — the workspace-scoped, append-only record of authoring events.
- **Workspace policy** — workspace-administrator-configured behavior (e.g., self-approval prohibition strictness, default waiver scopes, multi-role gating requirements).

## Data model

### `Workspace` (CM §1.1, extended)

```text
Workspace {
  id, title, description?,
  owners[],                 // list of user/identity refs with workspace-admin authority
  programs[],               // jurisdictions / programs the workspace covers
  reviewerRoles[],          // ReviewerRole registry
  policies,                 // WorkspacePolicy
  permissionsRef,           // pointer to permissions configuration
  createdAt, createdBy,
  archivedAt?, archivedBy?
}
```

A Workspace is **either active or archived**. Archived workspaces are read-only; their published artifacts remain queryable; no new authoring operations succeed.

### `ReviewerRole`

```text
ReviewerRole {
  id,                       // workspace-scoped
  name,                     // machine-readable (e.g., "compliance-reviewer")
  displayName,              // human-readable (e.g., "Compliance Reviewer")
  responsibilities[],       // which review levels this role is competent for (source | extracted-object | mapping | workflow | scenario | conflict)
  hasOverrideAuthority,     // boolean
  requiredForPublication,   // boolean — gates the publication contract in review-and-approval.md
  description?,
  workspaceId
}
```

The role registry is **append-only-by-id**: once a role id is used in an approval decision, retiring the role is allowed but reusing the id is not.

### `WorkspacePolicy`

Workspace-administrator-configured behavior:

```text
WorkspacePolicy {
  selfApprovalProhibition,        // strict | trivial-exempt | disabled — default: strict
  multiRoleGating[],              // [{subjectKind, requiredRoles[]}] — e.g., rights-impacting workflows require legal+compliance+technical
  defaultWaiverScope,             // this-instance-only | this-rule-on-this-subject-until-condition — default: this-instance-only
  unmappedButApprovedAuthority,   // role with authority to approve unmappedButApproved mappings
  conflictWaiverAuthority,        // role with authority to waive Conflict resolutions
  retentionPolicies,              // per-DataElement-sensitivity retention durations (referenced by EvidenceRequirement)
  workspaceId
}
```

### `WorkspaceAuditLogEntry`

Per [`authoring-provenance.md`](authoring-provenance.md) `SA-MUST-prov-040`, every Workspace MUST carry a single append-only authoring audit log. This spec defines the entry shape:

```text
WorkspaceAuditLogEntry {
  id, recordedAt, actorId, actorRole,
  action,                         // workspace-policy-edit | role-defined | role-retired | workflow-published | finding-waived | …
  subjectKind, subjectRef,
  before?, after?,                // structured snapshots when applicable
  workspaceId
}
```

The audit log aggregates AuthoringProvenanceRecords across all objects in the workspace; this entity is the queryable view, not a separate store.

## Lifecycle

A Workspace lifecycle:

```text
created → active → { archived | suspended }
suspended → active                              (workspace-admin reactivates)
archived (terminal — read-only)
```

A ReviewerRole lifecycle:

```text
defined → active → { retired | renamed }
retired (terminal for the id; the role's prior decisions remain queryable)
```

A WorkspacePolicy is **edit-in-place** with an audit-log entry per change; policies do not have lifecycle states beyond the audit trail.

## Normative Contract

### Workspace integrity

- **`SA-MUST-ws-001`** — Every Workspace MUST have at least one active workspace owner. A Workspace whose last owner leaves MUST either be transferred (new owner assigned by a tenant administrator) or archived. *(runtime-pending: ownership-transfer mechanism.)*
- **`SA-MUST-ws-002`** — Every Workspace MUST have a non-empty `reviewerRoles[]` set with at least one role having `hasOverrideAuthority = true` and at least one role having `requiredForPublication = true`. (Same as [`review-and-approval.md`](review-and-approval.md) `SA-MUST-ra-001`; that rule cross-references this one as the structural source of truth.) *(schema-pending.)*
- **`SA-MUST-ws-003`** — Workspace-policy edits MUST be appended to the audit log with `before/after` snapshots. *(runtime-pending.)*
- **`SA-MUST-ws-004`** — Archived Workspaces MUST reject every authoring operation with a structured `workspace-archived` error. Read operations remain available. *(runtime-pending.)*

### ReviewerRole integrity

- **`SA-MUST-ws-010`** — ReviewerRole `id` is workspace-scoped and MUST NOT be reused after retirement. *(schema-pending.)*
- **`SA-MUST-ws-011`** — A ReviewerRole's `requiredForPublication` flag MAY be flipped from `false → true`, but doing so applies only to subsequent workflow versions; workflows currently in `approved` lifecycle state are NOT retroactively required to obtain that role's approval. (Same as [`review-and-approval.md`](review-and-approval.md) §"Versioning / migration"; this is the structural source.) *(runtime-pending.)*
- **`SA-MUST-ws-012`** — Retiring a ReviewerRole MUST NOT invalidate prior ApprovalDecisions made under that role. The decisions remain `active` or `superseded` per their normal lifecycle. *(runtime-pending.)*
- **`SA-MUST-ws-013`** — Default-suggested role set for a new workspace SHOULD include: `workflow-owner` (override + required), `compliance-reviewer` (required), `legal-reviewer` (required for rights-impacting workflows; conditional via WorkspacePolicy.multiRoleGating), `technical-reviewer` (required), `operations-reviewer` (required). Workspace administrators MAY add, remove, or rename roles.

### Permissions

- **`SA-MUST-ws-020`** — Every workspace-state mutation (object creation, edit, lifecycle transition, mapping change, ApprovalDecision recording, finding waiver, scenario simulation) MUST be authorized against the actor's identity + role + the relevant subject's permissions. The default-deny posture applies when no explicit permission grant exists. *(runtime-pending: authorization layer.)*
- **`SA-MUST-ws-021`** — Sensitive data (DataElements with `sensitivity` ∈ {`pii`, `phi`, `restricted`}) MUST have access logged at every read, not only at write. *(runtime-pending.)*
- **`SA-MUST-ws-022`** — Workspace policy edits MUST be authorized to workspace owners only. *(runtime-pending.)*
- **`SA-SHOULD-ws-023`** — Permissions SHOULD be configurable at three granularities: workspace-wide, per-workflow, per-object. The default permission model applies the most-permissive resolved grant.

### Audit log

- **`SA-MUST-ws-030`** — Every Workspace MUST carry a single audit log queryable by `actorId`, `actorRole`, `action`, `subjectKind`, time range, and `originClass`. *(runtime-pending: indexing.)*
- **`SA-MUST-ws-031`** — Audit-log entries MUST NOT be alterable; corrections are appended as compensating entries. (Same shape as [`authoring-provenance.md`](authoring-provenance.md) `SA-MUST-prov-002` for AuthoringProvenanceRecords; this rule applies to non-provenance entries like workspace-policy edits.) *(schema-pending.)*
- **`SA-MUST-ws-032`** — Audit-log retention MUST be at least the maximum retention required by any DataElement's `sensitivity` plus one year, OR per workspace policy, whichever is longer. *(runtime-pending.)*
- **`SA-SHOULD-ws-033`** — Audit-log queries SHOULD be answerable in plain language ("what changed in this workspace last week?") via reviewer-friendly rendering.

## Composition

### Attachment point

Workspace IS the attachment point for Studio. Every other spec attaches here:

- [`source-vault.md`](source-vault.md) — sources live in a Workspace.
- [`policy-object-model.md`](policy-object-model.md) — PolicyObjects live in a Workspace.
- [`authoring-provenance.md`](authoring-provenance.md) — AuthoringProvenanceRecords accumulate per Workspace.
- [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md) — mappings live in a Workspace.
- [`scenario-authoring.md`](scenario-authoring.md) — Scenarios live in a Workspace.
- [`readiness-validation.md`](readiness-validation.md) — ValidationFindings are workspace-scoped.
- [`review-and-approval.md`](review-and-approval.md) — ApprovalDecisions and ReviewerComments are workspace-scoped; the role registry it consumes lives here.
- [`change-impact.md`](change-impact.md) — ChangeImpactReports are workspace-scoped.
- [`binding-and-integration.md`](binding-and-integration.md) — bindings live in a Workspace.
- [`workflow-intent.md`](workflow-intent.md) — WorkflowIntents live in a Workspace.
- [`compiler-contract.md`](compiler-contract.md) — compilation runs against a Workspace's state.

Cross-workspace federation (e.g., shared SourceDocuments across two Workspaces) is deferred per [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §6.

### Precedence

WorkspacePolicy rules are **strict** — they cannot be overridden per-object. If a workspace policy prohibits self-approval (default), individual reviewers cannot approve their own subjects regardless of override authority.

When two ReviewerRoles claim authority over the same review level (e.g., both `compliance-reviewer` and `legal-reviewer` are responsibilities for `mapping`), both can author ApprovalDecisions; the publication gate aggregates by required-role per [`review-and-approval.md`](review-and-approval.md) `SA-MUST-ra-040`.

### Conflict handling

Two reviewers in different roles disagreeing is not a conflict at this layer — see [`review-and-approval.md`](review-and-approval.md) for approval-aggregation rules. Workspace-policy contradictions (e.g., two configured retention durations for the same sensitivity) MUST be resolved by workspace-administrator action; the implementation MUST refuse to apply contradictory policy.

### Versioning / migration

- Adding a ReviewerRole: non-breaking.
- Marking a role `requiredForPublication = true`: applies prospectively only (per `SA-MUST-ws-011`).
- Retiring a role: prior decisions stand; the role's id is permanently consumed.
- Editing WorkspacePolicy: per-edit audit-log entry; semantics apply prospectively.

## Conformance

### Schema validation (Stage 3)

- Workspace required fields and lifecycle enum.
- ReviewerRole shape and id-uniqueness constraint.
- WorkspacePolicy shape.
- Audit-log entry shape.

### Lint rules (Stage 4)

Tier-S6 readiness rules cross-cutting from [`review-and-approval.md`](review-and-approval.md):

- `WS-LINT-001` — Workspace has at least one override-authority role and one required-for-publication role (`SA-MUST-ws-002`).
- `WS-LINT-002` — Sensitive DataElements have retention policy in WorkspacePolicy.retentionPolicies (cross-cutting `SA-MUST-pom-037`).
- `WS-LINT-003` — Workspace audit-log retention satisfies the maximum sensitivity requirement (`SA-MUST-ws-032`).

### Runtime conformance fixtures (Stage 4–5)

- Workspace with no required-publication role refuses to publish.
- Archived Workspace refuses authoring operations.
- ReviewerRole id reuse after retirement is rejected.
- Workspace-policy edit appends the expected audit-log entry.
- Sensitive-data read access is logged.

### Current limitations

- Identity model is sketched at the role-and-permission surface; identity-provider integration is a deployment concern.
- Cross-workspace federation is deferred.
- Permission-resolution algorithm is sketched (most-permissive resolved grant); precise predicate is Stage-4 detail.

## WOS mappings

Workspace, ReviewerRole, WorkspacePolicy, and WorkspaceAuditLogEntry are **`authoringOnly`** as a whole — they are Studio-internal concerns and never appear directly in `$wosWorkflow`.

| Studio object | Mapping state | WOS path |
|---|---|---|
| Workspace | `authoringOnly` | — |
| ReviewerRole | `authoringOnly` | — (referenced by ApprovalDecision compact projection per [`authoring-provenance.md`](authoring-provenance.md)) |
| WorkspacePolicy | `authoringOnly` | — |
| WorkspaceAuditLogEntry | `authoringOnly` | — (provenance entries project compactly per `SA-MUST-prov-030`) |

The exception: `reviewerRole` strings (e.g., "compliance-reviewer") project as part of ApprovalDecision compact projections into the published artifact's authoring-provenance config, so a downstream auditor sees role names without needing access to the originating Workspace.

## Examples

### Example 1: New Workspace setup for a state SNAP program

A state Department of Human Services creates a Workspace for SNAP redetermination workflow authoring.

1. Workspace `snap-redetermination-2026` created. Owner: `snap-program-director@dhs.state.gov`.
2. WorkspacePolicy: `selfApprovalProhibition = strict`, `multiRoleGating = [{subjectKind: WorkflowIntent, requiredRoles: [workflow-owner, legal-reviewer, compliance-reviewer, technical-reviewer, operations-reviewer]}]`, `unmappedButApprovedAuthority = workflow-owner`, retention policy: `phi: 7y; pii: 7y; restricted: 7y`.
3. ReviewerRoles defined: `workflow-owner` (override, required), `compliance-reviewer` (required), `legal-reviewer` (required), `technical-reviewer` (required), `operations-reviewer` (required), `governance-reviewer` (not required, advisory).
4. Programs: `["SNAP", "SNAP-E&T"]`.
5. Audit log entry recorded for workspace creation; entry recorded for each role definition.

### Example 2: Retiring a role mid-flight

The state agency reorganizes; the `governance-reviewer` role is retired. Three approved workflows have `governance-reviewer` ApprovalDecisions in their approval packages.

1. Role retired in workspace registry; audit log entry recorded.
2. Existing ApprovalDecisions remain `active` per `SA-MUST-ws-012`.
3. Future workflows cannot record `governance-reviewer` decisions; the role's id is permanently consumed.
4. The role retirement is visible in any future export of the workspace's role registry.

### Example 3: Sensitive-data access logging

A reviewer with `legal-reviewer` role views a NoticeRequirement that depends on a DataElement (`monthlyIncome`, `sensitivity = pii`) and a citation excerpt that quotes a tax form. The view is a read, not an edit.

1. The viewer's identity, role, and the subject refs are logged in the audit log per `SA-MUST-ws-021`.
2. The audit-log entry shows the viewer accessed PII context; downstream audit reports can enumerate who saw what.
3. No state was changed; no AuthoringProvenanceRecord (those are for state changes).

## Open issues

- **Cross-workspace federation.** Two workspaces sharing a SourceDocument (or a PolicyObject) is deferred. The federation model is the largest open question for cross-organization authoring.
- **Tenant model.** This spec assumes a Workspace is single-tenant. Whether multiple Workspaces can share an organizational parent (with inherited roles, policies, retention) is unspecified.
- **Identity-provider integration.** The role and permission surface is defined; how identity is established (OIDC, SAML, custom) is implementation.
- **Permission-resolution algorithm.** "Most-permissive resolved grant" is a default; precise rules at the per-object granularity are Stage-4 detail.
- **WorkspacePolicy versioning.** Edit-in-place with audit log is acceptable for v1; whether policy itself becomes a versioned artifact (with rollback) is unsettled.

## Cross-references

- Concept model: [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.1, §4.
- PRD: [`../VISION.md`](../VISION.md) §3 (users), §13 (security/permissions).
- Consumes: nothing (this is the attachment-point spec).
- Consumed by: every other Studio spec (workspaces own everything).
- Repo conventions: [`../../CONVENTIONS.md`](../../CONVENTIONS.md).
