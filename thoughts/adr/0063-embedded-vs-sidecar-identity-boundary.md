# ADR-0063: Embedded-vs-sidecar identity boundary

**Status:** Accepted
**Date:** 2026-04-30
**Deciders:** WOS Working Group
**Author:** WOS conformance hardening
**Supersedes:** None
**Related:**

- [CLAUDE.md schema-structure section](../../CLAUDE.md) -- merged-envelope direction (one author-time core schema, embedded blocks, sidecars)
- [`schemas/wos-workflow.schema.json`](../../schemas/wos-workflow.schema.json) -- merged author-time envelope
- [`schemas/sidecars/`](../../schemas/sidecars/) -- sidecar schemas
- [ADR-0060](0060-cross-reference-naming-ref-key-id.md) -- cross-reference naming (`*Ref`, `*Key`, `*Id`)
- [ADR-0062](0062-signature-profile-workflow-semantics.md) -- profiles attach without widening kernel enums

---

## 1. Context

The merged-envelope direction collapsed WOS author-time documents into a single `$wosWorkflow` envelope. That direction was right: it gives workflows one canonical identity, makes risk-based requirements (e.g., governance required when `impactLevel` is `rights-impacting` or `safety-impacting`) enforceable at the root, and dramatically simplifies authoring, validation, and downstream tooling.

The migration consolidated wrappers but did not consolidate identity. The current repo carries three different concepts under the same `targetWorkflow` keyword:

1. **Embedded blocks** -- `governance`, `agents`, `aiOversight`, `signature`, `custody`, `advanced`, `assurance` -- which are part of the enclosing `$wosWorkflow` and govern it.
2. **Sidecars** -- `$wosDelivery`, `$wosOntologyAlignment` -- which are deployment/environment overlays that legitimately point at a workflow URI.
3. **Legacy standalone documents** -- `$wosKernel`, `$wosWorkflowGovernance`, `$wosAIIntegration`, `$wosAdvancedGovernance`, etc. -- which used to target a kernel document and which the merged direction retired.

Conformance fixtures expose the muddle. One fixture wraps a sidecar-shaped payload inside a placeholder `$wosWorkflow` envelope whose `url` is a dummy migration URI, while the embedded block carries `targetWorkflow` pointing to the real workflow. That breaks deterministic project loading, cross-reference validation, and any claim that the merged document is the single source of truth.

The missing principle: **embedded blocks do not target workflows; they govern the enclosing workflow.** Only sidecars target workflows.

---

## 2. Decision

The merged `$wosWorkflow` envelope is the **sole author-time identity boundary** for a workflow. Identity is defined by the envelope's `url` and `version`, never by an embedded block.

Three categories, three identity rules:

### 2.1 Embedded blocks

`governance`, `agents`, `aiOversight`, `signature`, `custody`, `advanced`, `assurance` are part of the enclosing workflow.

- They MUST NOT declare `targetWorkflow`.
- They MUST NOT declare independent `url` or `version`.
- They have no identity of their own; they govern the envelope they are embedded in.
- If an embedded block requires per-block versioning for compatibility tracking, that is recorded in `$wosWorkflow` envelope-version metadata, not in the block itself.

### 2.2 Sidecars

`$wosDelivery`, `$wosOntologyAlignment` are deployment/environment overlays that bind to a workflow at deploy time, not author time.

- They MUST declare `targetWorkflow` as a non-empty workflow URI matching some workflow's `url`.
- They MUST carry their own `$wos*` marker.
- They MAY declare their own `version` independently of the workflow they target.
- Multiple sidecars MAY target the same workflow.

### 2.3 Runtime artifacts

`$wosProcess` and `$wosProvenanceLog` are produced by processors at runtime.

- They reference a workflow URI (via `workflowRef` or equivalent), but they are not author-time documents.
- They MUST carry their `$wos*` marker so the parser detects them uniformly with author-time documents.
- They are written, not authored; lint rules apply only to author-time documents.

### 2.4 Legacy standalone documents

Legacy standalone author-time documents -- `$wosKernel`, `$wosWorkflowGovernance`, `$wosAIIntegration`, `$wosAdvancedGovernance`, `$wosSignatureProfile`, `$wosCustody`, `$wosAssurance` -- are retired. The merged-envelope direction supersedes them.

This is a **greenfield** retirement: no migration aliases, no compatibility shims, no "view" or "fragment" preservation of legacy markers. Authors who previously held standalone documents migrate by inlining them as embedded blocks under a single `$wosWorkflow` envelope.

The lint document parser MUST recognize only six canonical author-time markers (`$wosWorkflow`, `$wosDelivery`, `$wosOntologyAlignment`, `$wosTooling`) plus two runtime-artifact markers (`$wosProcess`, `$wosProvenanceLog`).

---

## 3. Rejected Alternatives

### Keep `targetWorkflow` on embedded blocks as a migration alias

Rejected. An alias creates two competing sources of identity (envelope `url` vs. block `targetWorkflow`). LLM authoring will produce both shapes; validators will need to reconcile them; export bundles and provenance records will diverge. The whole point of the merged envelope is one identity.

### Treat embedded blocks as standalone documents that happen to be inlined

Rejected. If an embedded block is "really" a standalone document, the envelope is a transport wrapper rather than a semantic unit. That re-creates the pre-merge problem under a different name.

### Allow sidecars to be inlined as embedded blocks

Rejected. Sidecars carry deployment/environment configuration (calendars, notification templates, JSON-LD context, SHACL shapes) that legitimately differs across deployments of the same workflow. Inlining would force authors to maintain a separate workflow per environment. The sidecar/envelope split is the right abstraction for that distinction.

### Preserve legacy standalone documents as "views" or "fragments"

Rejected (greenfield). A "view" preserves the authoring ergonomics of standalone documents at the cost of two object models the spec must describe and validate. The repo is small enough to migrate now; the documentation cost of maintaining two models forever is larger than the migration cost today.

---

## 4. Consequences

### Positive

- One canonical identity per workflow; no envelope-vs-block ambiguity.
- LLM authoring is simpler: a single `$wosWorkflow` envelope with optional embedded blocks, no second-place identity to coordinate.
- Cross-reference validation becomes deterministic. `targetWorkflow` always points to a real workflow URI, never to a placeholder.
- Generated TypeScript and Rust runtime types collapse around `WorkflowDocument`; legacy `WOSAdvancedGovernanceDocument`, `WOSAIIntegrationDocument`, `WOSWorkflowGovernanceDocument` shapes go away.
- Export bundles and provenance records carry one workflow identity, not two competing ones.

### Negative

- Existing standalone documents that pre-date the merge must migrate to merged envelopes. Greenfield posture means no compatibility path; a migration utility is the operator's responsibility, not WOS's.
- Some authoring tools that displayed sidecar-shaped editors for governance/AI/advanced now display embedded-block editors instead. UX work, not spec work.
- Spec prose under `specs/governance/spec.md`, `specs/ai/ai-integration.md`, `specs/advanced/spec.md` that referenced "the Workflow Governance Document targets a WOS Kernel Document" must be rewritten as "the `governance` embedded block governs the enclosing `$wosWorkflow`."

### Neutral

- The six-seam invariant (kernel `actorExtension`, `contractHook`, `provenanceLayer`, `lifecycleHook`, `custodyHook`, `extensions` / `x-` keys) is unchanged. Embedded blocks are not seams; they are part of the envelope.
- Sidecars remain independent release artifacts. The independence three-question rubric in CONVENTIONS.md applies only to candidate sidecars, never to embedded blocks.

---

## 5. Implementation Notes

This ADR is the spec layer of a larger conformance hardening pass. The implementation lands in five sub-PRs:

1. **Sub-PR A (this ADR + schema + fixtures + lint)** -- ratify the boundary rule, add schema conditionals, lint rules, and rewrite the one fixture (`AI-AUTO-002-drift-alert-demotion.json`) that violates it.
2. **Sub-PR B** -- merged-envelope Rust runtime model (`WorkflowDocument` + projections), `Agent` first-class `ActorKind`.
3. **Sub-PR C** -- `AgentInvoker` port and adapter crates.
4. **Sub-PR D** -- `foreach` state end-to-end.
5. **Sub-PR E** -- generated TypeScript regen, studio call-site updates, spec prose rewrites.

Schema-level enforcement of the boundary rule lives in `schemas/wos-workflow.schema.json` as `allOf` conditionals (one per embedded block) forbidding `targetWorkflow`. Lint-level enforcement lives in `WOS-EMBED-TARGET-001` (Tier 1) and `WOS-EMBED-IDENTITY-001` (Tier 1). Sidecar enforcement lives in `WOS-SIDECAR-TARGET-001` (Tier 1).

Runtime-artifact marker requirements (`$wosProcess`, `$wosProvenanceLog`) land in this sub-PR as schema additions and parser-test alignment.
