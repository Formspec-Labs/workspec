# Studio Spec: Source Vault

**Status:** draft (Stage 2 of [Implementation Roadmap](../VISION.md#17-implementation-roadmap))
**Date:** 2026-04-30
**Concept-model anchor:** [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.2 SourceDocument, §1.3 SourceVersion, §1.4 SourceSection, §1.5 SourceCitation, §2.1 SourceDocument lifecycle.
**PRD anchor:** [`../VISION.md`](../VISION.md) §9.1 (Source Vault), §12 (User stories — Source Vault).

## Scope

The Source Vault is the system of record for **primary input artifacts**: the policy manuals, SOPs, regulations, memos, forms, decision guides, service blueprints, case documentation, diagrams, screenshots, and integration documents that workflow authoring is grounded in. It is the foundation on which every later spec depends — extracted claims, policy objects, mappings, scenarios, validation findings, and approvals all ultimately resolve back through Source Vault citations.

This spec defines:

- the SourceDocument / SourceVersion / SourceSection / SourceCitation data model;
- the lifecycle a SourceDocument passes through from upload to current/superseded;
- the normative contract for parsing, indexing, classification, citation integrity, and source-version comparison;
- how the Source Vault composes with downstream Studio specs;
- conformance expectations (what Stage-3+ schemas/lint/fixtures will check).

## Out of scope

- Permissions/access-control implementation (a deployment concern; this spec defines the surface, not the policy).
- Full-text search ranking (a UX/infra choice).
- OCR or PDF-extraction algorithm details (the spec requires the *result*: text + page map, not how it's produced).
- Cross-workspace source reuse (deferred — see [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §6 Open Issues).

## Terminology

- **SourceDocument** — the logical artifact of record (e.g., "FAFSA ISIR Guide"). Stable identity across versions.
- **SourceVersion** — a specific version of a SourceDocument, with effective-date metadata.
- **SourceSection** — a semantically meaningful chunk of a SourceVersion (section, paragraph, table cell, form field, diagram region).
- **SourceCitation** — a typed reference from a Studio object back to one or more SourceSections.
- **Anchor** — the addressable identifier of a SourceSection within its SourceVersion (e.g., `§3.2.1`, `page=4,para=2`).
- **Pageable** — a SourceVersion is *pageable* if its SourceSections have stable page-number anchors that survive re-parsing.

## Data model

The four entities below are defined in [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.2–§1.5. This spec adds Source-Vault-specific normative behavior on top of those definitions.

### SourceDocument (CM §1.2)

Adds, over the concept-model fields:

- `documentFingerprint` — a stable, content-derived identifier (e.g., SHA-256 of normalized text) used to detect re-uploads of the same document.
- `tags[]` — workspace-defined tags (program, policy area, jurisdiction, sensitivity).

### SourceVersion (CM §1.3)

Adds:

- `parsingResult` — a structured outcome `{status, errors[], warnings[], parserVersion, parsedAt}`. `status` is `ok | partial | failed`.
- `indexingResult` — `{status, sectionCount, fullTextIndexId, indexedAt}`.
- `classificationResult` — `{status, type, program, jurisdiction, language, effectiveStart, effectiveEnd, confidence, classifiedBy}`. `classifiedBy` is `human | ai | mixed`.
- `pageable` (boolean) — true iff sections have stable page anchors.

### SourceSection (CM §1.4)

Adds:

- `parentSectionId` (nullable) — for hierarchical documents.
- `siblingOrder` — integer; preserves document order.
- `extractedFrom` — `{startOffset, endOffset, page?, bbox?}` linking back to the raw payload.

### SourceCitation (CM §1.5)

Adds:

- `excerpt` — a verbatim quote, REQUIRED, used for human-readable display and for source-change detection.
- `relation` — already in CM; constrained here to `supports | derivedFrom | conflictsWith | supersedes | contradicts`.

## Lifecycle (normative)

The SourceDocument lifecycle from CM §2.1:

```text
uploaded → parsed → indexed → classified → { current | superseded | preliminary | disputed }
```

Allowed transitions (any other transition MUST be rejected by the implementation):

| From | To | Trigger |
|---|---|---|
| `uploaded` | `parsed` | parser produced text + section structure |
| `parsed` | `indexed` | full-text index built |
| `indexed` | `classified` | type/program/jurisdiction/effective-date metadata set |
| `classified` | `current` | reviewer confirms this is the active version |
| `classified` | `preliminary` | reviewer marks as not-yet-effective |
| `classified` | `disputed` | reviewer marks as contradicted by another source |
| `current` | `superseded` | a later SourceVersion is promoted to `current` |
| `preliminary` | `current` | the version reaches its `effectiveStart` |
| `preliminary` | `superseded` | a different version is promoted before this one becomes effective |
| `disputed` | `current` | the dispute is resolved in this version's favor |
| `disputed` | `superseded` | the dispute is resolved against this version |

Terminal states: `superseded`. (A superseded version remains queryable but cannot be cited as the basis for new approved PolicyObjects.)

## Normative Contract

Each MUST below carries a tracking ID. IDs of the form `SA-MUST-source-NNN` are Source-Vault-specific. Every MUST is annotated with how it is enforced today; "schema-pending" / "lint-pending" indicates a Stage-3 / Stage-4 gap.

### Upload, parse, index, classify

- **`SA-MUST-source-001`** — A SourceDocument MUST have at least one SourceVersion before any extraction or citation operation may target it. *(lint-pending: tier S1 readiness rule will surface if ExtractedClaims target a versionless SourceDocument.)*
- **`SA-MUST-source-002`** — Every SourceVersion MUST progress through `uploaded → parsed → indexed → classified` before it can be promoted to `current`, `preliminary`, or `disputed`. *(schema-pending: lifecycle state machine; lint-pending: enforce the order.)*
- **`SA-MUST-source-003`** — `parsingResult.status = ok` MUST be required for a SourceVersion to leave `uploaded`. A SourceVersion with `parsingResult.status = partial` MAY proceed to `indexed` only if a reviewer explicitly waives the parsing warnings; the waiver MUST be recorded as a ReviewerResolution. *(lint-pending; reviewer-waiver is a workspace-state edit.)*
- **`SA-MUST-source-004`** — Classification MUST set `effectiveStart` for any SourceVersion that will become `current` or `preliminary`. A SourceVersion without `effectiveStart` MUST NOT be promoted to `current`. *(lint-pending: tier S2 readiness rule.)*
- **`SA-MUST-source-005`** — Within a single SourceDocument, at most one SourceVersion MAY hold the `current` lifecycle state at any given moment. Promoting a new version to `current` MUST atomically transition the prior `current` version to `superseded`. *(schema-pending: cardinality constraint; runtime-pending: transition semantics.)*

### Sections + pageability

- **`SA-MUST-source-010`** — Every SourceSection MUST carry an anchor that is unique within its SourceVersion. *(schema-pending: uniqueness constraint.)*
- **`SA-MUST-source-011`** — When a SourceVersion's `pageable` flag is `true`, every SourceSection in that version MUST carry a `pageRange`. *(lint-pending.)*
- **`SA-MUST-source-012`** — A SourceSection's `siblingOrder` MUST be stable across re-parsings of the same SourceVersion's payload — i.e., re-running the parser on identical bytes MUST produce the same section sequence. *(fixture-pending: round-trip parsing test.)*
- **`SA-SHOULD-source-013`** — A SourceSection's `text` SHOULD be Unicode-normalized (NFC) before indexing. Citations rely on text identity for excerpt verification.

### Citations

- **`SA-MUST-source-020`** — Every SourceCitation MUST resolve to a real SourceSection inside an existing SourceVersion. Dangling citations (the section was deleted or the version was rebuilt with different anchors) MUST be flagged as tier-S1 ValidationFindings on the citing object. *(lint-pending.)*
- **`SA-MUST-source-021`** — Every SourceCitation MUST carry an `excerpt`. The excerpt MUST be a substring of the cited SourceSection's `text` (Unicode-normalized NFC). If the section's text changes (e.g., a new SourceVersion is uploaded and the section's text drifts), the citation's excerpt MUST be re-verified before any object that depends on it can advance its lifecycle. *(lint-pending: excerpt-mismatch finding; runtime-pending: re-verification on supersession.)*
- **`SA-MUST-source-022`** — A SourceCitation's `relation` MUST be one of `supports | derivedFrom | conflictsWith | supersedes | contradicts`. Other relation strings MUST be rejected at citation creation. *(schema-pending: enum constraint.)*
- **`SA-SHOULD-source-023`** — When a SourceCitation's `relation` is `supports`, the citing object's claim SHOULD be defensibly inferable from the cited excerpt; reviewers verify this at approval time.

### Supersession + change detection

- **`SA-MUST-source-030`** — Promoting a new SourceVersion to `current` MUST trigger a supersession event recording: `{priorVersionId, newVersionId, supersededAt, supersededBy}`. *(schema-pending: event record.)*
- **`SA-MUST-source-031`** — Supersession MUST trigger a ChangeImpactReport (see [`change-impact.md`](change-impact.md)) enumerating every PolicyObject, Mapping, Scenario, and PublishedWorkflowPackage that cites the prior `current` version. *(runtime-pending; cross-spec coupling with change-impact.)*
- **`SA-SHOULD-source-032`** — Source-version comparison SHOULD highlight changed sections at the section level, not at the byte level. Reviewers expect to see *which §-numbered subsection moved*, not raw character diffs.
- **`SA-MUST-source-033`** — A SourceVersion in `disputed` lifecycle state MUST NOT be the sole basis for any approved PolicyObject. An approved PolicyObject citing only disputed sources MUST surface a tier-S1 ValidationFinding until the dispute is resolved. *(lint-pending.)*

### Cross-document supersession

`SA-MUST-source-005` constrains supersession to **within** a single SourceDocument (a new SourceVersion supersedes a prior SourceVersion of the same document). Real workflows also have **cross-document** supersession: a federal corrective-action letter overrides an office procedure memo; a state directive overrides an agency guidance document; a court ruling overrides prior interpretation. These are *different* SourceDocuments with different authors and identifiers — Source Vault supersession does not handle them.

The mechanism for cross-document supersession is the **`Supersession` PolicyObject kind** in [`policy-object-model.md`](policy-object-model.md) §"Source-and-authority objects". A reviewer authors a `Supersession` PolicyObject identifying both `superseder` and `superseded` (which may be PolicySource or PolicyObject refs across SourceDocuments), records the rationale via a ReviewerResolution, and the policy-object layer takes precedence over the unrelated SourceVersion lifecycles.

- **`SA-MUST-source-006`** — When two PolicyObjects citing different SourceDocuments are both `approved` and a workspace-state `Supersession` PolicyObject identifies one as superseding the other, the implementation MUST treat the superseded PolicyObject as having reduced evidentiary weight: cross-document Conflicts (per [`policy-object-model.md`](policy-object-model.md) `SA-MUST-pom-040`) detected against the superseded object MUST resolve in favor of the superseder by default, with a reviewer override available. The `Supersession` PolicyObject's `effectiveAt` field bounds the supersession's effective range. *(runtime-pending; cross-spec coupling with `policy-object-model.md` and `change-impact.md`.)*
- **`SA-MUST-source-007`** — A `Supersession` PolicyObject MUST be authored, reviewed, and approved through the standard PolicyObject lifecycle; it is not a Source-Vault auto-detection. The policy-level supersession is reviewer-driven by design — auto-supersession across documents would risk silently overriding policy authority. *(runtime-pending.)*

### Permissions + audit

- **`SA-MUST-source-040`** — Every SourceDocument MUST carry a `permissionsRef` (even if it points to a permissive default) so access control can be uniformly evaluated across the workspace. *(schema-pending: required field.)*
- **`SA-MUST-source-041`** — Upload, supersession, classification changes, and lifecycle transitions on SourceDocument and SourceVersion MUST be appended to the workspace audit log with `{actorId, actorRole, action, before, after, at}`. *(runtime-pending: audit log emission.)*

## Composition

### Attachment point

The Source Vault attaches at the **workspace** layer — every Workspace contains one Source Vault; sources do not sit at the workspace-cluster level (cross-workspace reuse is deferred). The Studio→WOS compiler (Stage 5) reads from the Source Vault but does not write to it; sources are workspace-state only and never appear directly in the published `$wosWorkflow` artifact (citations do, via authoring provenance).

### Precedence

When two SourceVersions of the same SourceDocument disagree on a fact (e.g., two effective dates for the same regulation), precedence is:

1. The version with the later `effectiveStart` that is `current`.
2. If both are `current` (a transient state during supersession), the supersession event's `newVersionId` wins.
3. If both are `preliminary`, the workspace owner's explicit promotion decision wins; absent a decision, the conflict is recorded as a tier-S2 Conflict.
4. `disputed` versions never establish precedence on their own.

When two SourceDocuments (different documents, not different versions of the same document) disagree on a fact, the conflict is detected and recorded as a Conflict (CM §1.9); precedence is established by `AuthorityRank` PolicyObjects authored over those sources, not by Source Vault rules. The Source Vault provides the substrate; authority resolution is a [`policy-object-model.md`](policy-object-model.md) concern.

### Conflict handling

The Source Vault **records** disagreements; it does not silently merge or discard. A conflict between SourceVersions is a Conflict entity (CM §1.9). A conflict between SourceVersion + Assumption is similarly a Conflict. The reviewer's resolution (ReviewerResolution, CM §1.10) is the only path to clearing a Conflict.

### Versioning / migration

- Adding a new SourceVersion to an existing SourceDocument is a non-breaking workspace operation.
- Changing the parser version (`parsingResult.parserVersion`) on a SourceVersion produces a new `parsingResult` snapshot; if section anchors change, dependent citations MUST be re-verified (see `SA-MUST-source-021`). A parser-version change is a workspace-state migration, not a SourceVersion supersession.
- Re-classification (changing `effectiveStart`, `program`, `jurisdiction`) on a SourceVersion is a workspace audit-logged edit, not a supersession; downstream PolicyObjects do not re-validate unless the change crosses a `current → preliminary` or `current → disputed` boundary.

## Conformance

### Schema validation (Stage 3)

- SourceDocument, SourceVersion, SourceSection, SourceCitation each have JSON Schemas under [`../schemas/`](../schemas/) once Stage 3 lands. Until then, every MUST in this spec carrying "schema-pending" is **not** structurally enforced.
- Specific schema gates planned: lifecycle-state enum, mapping-state precedence, citation-relation enum, required-fields-by-state.

### Lint rules (Stage 4)

Tier-S1 ("Source and extraction readiness") rules planned:

- `SV-LINT-001` — every SourceCitation must resolve (SA-MUST-source-020).
- `SV-LINT-002` — every citation excerpt must match its section text (SA-MUST-source-021).
- `SV-LINT-003` — no PolicyObject may cite a `disputed`-only or `superseded`-only set of versions (SA-MUST-source-033).
- `SV-LINT-004` — every `current` version must have `effectiveStart` set (SA-MUST-source-004).
- `SV-LINT-005` — section anchors must be unique within a SourceVersion (SA-MUST-source-010).

### Runtime conformance fixtures (Stage 4–5)

Fixture-pending behaviors:

- Re-parsing identical bytes produces identical section sequence (SA-MUST-source-012).
- Promoting a new version to `current` atomically supersedes the prior `current` (SA-MUST-source-005).
- Supersession triggers a ChangeImpactReport (SA-MUST-source-031).
- Excerpt re-verification fires on supersession (SA-MUST-source-021).

### Current limitations

This spec is normatively prose-complete but **not enforceable** until Stage 3 (schemas) and Stage 4 (lint engine) land. Until then, the contract is reviewer-discipline and explicit ValidationFindings authored by hand.

## WOS mappings

Source Vault entities are **`authoringOnly`** by default — sources, versions, sections, and citations are workspace-state metadata, not WOS content. They never appear directly in `$wosWorkflow`.

The exception: SourceCitations are **read** by the Studio→WOS compiler and emitted into the published artifact's authoring-provenance records (see [`authoring-provenance.md`](authoring-provenance.md)). The compiler embeds compact citation references (`{sourceDocumentId, sourceVersionId, sectionAnchor, excerptHash}`) into provenance, not full SourceSection bodies.

| Studio object | Mapping state | Target WOS path | Notes |
|---|---|---|---|
| SourceDocument | `authoringOnly` | — | Workspace metadata only |
| SourceVersion | `authoringOnly` | — | Workspace metadata only |
| SourceSection | `authoringOnly` | — | Workspace metadata only |
| SourceCitation | `authoringOnly` (with provenance projection) | provenance records (compact form) | Excerpt body stays in workspace; only `{ids, anchor, excerptHash}` projects |

## Examples

### Example 1: New regulation, classification, supersession

A program manager uploads `WIOA-Final-Rule-2026.pdf` to a workforce-services workspace.

1. Document arrives: state `uploaded`. `documentFingerprint` computed.
2. Parser runs: `parsingResult.status = ok`, sections identified at §-level. State `parsed`.
3. Indexing builds full-text index. State `indexed`.
4. Reviewer classifies: `type = regulation`, `program = WIOA`, `jurisdiction = US-Federal`, `effectiveStart = 2026-07-01`. State `classified`.
5. Reviewer promotes to `current` on 2026-06-15 (during the rule's pre-effective publication window). The prior version (`WIOA-Interim-Rule-2024.pdf`, currently `current`) atomically transitions to `superseded`. A supersession event fires.
6. The supersession triggers a ChangeImpactReport that enumerates 47 PolicyObjects citing the prior version, plus 12 active workflow drafts and 3 published packages.

### Example 2: Citation excerpt re-verification

A reviewer-approved Requirement PolicyObject cites `34 CFR §668.34(b)(2)` from `Title-IV-Handbook-2025.pdf`. The handbook is re-uploaded as a new SourceVersion with editorial corrections; the §668.34 paragraph's text changes by two words.

1. New SourceVersion supersedes the prior one.
2. Citation re-verification fires (`SA-MUST-source-021`).
3. The Requirement's citation excerpt no longer matches the new section text → tier-S1 ValidationFinding raised on the Requirement.
4. The Requirement's lifecycle is **not** automatically demoted, but it cannot advance further (e.g., to `validated`) until a reviewer either updates the excerpt to match the new text or marks the change as immaterial via ReviewerResolution.

### Example 3: Disputed version

Two regulatory authorities publish overlapping guidance on the same SNAP eligibility question. Both are uploaded; both reach `classified`. A reviewer recognizes the contradiction and marks one version `disputed`.

1. Any PolicyObject citing only the disputed version surfaces a tier-S1 ValidationFinding (`SA-MUST-source-033`).
2. The reviewer authors a Conflict entity recording the disagreement; later, an AuthorityRank PolicyObject (in [`policy-object-model.md`](policy-object-model.md)) resolves precedence.
3. Once the AuthorityRank is approved, the dispute is recorded as resolved; the disputed version moves to `current` or `superseded` based on the resolution.

## Open issues

- **Cross-workspace source reuse.** A regulation cited by two workspaces is uploaded twice today. Whether to deduplicate at a tenant or organization scope is deferred (see [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §6).
- **Diagram-as-source.** PRD §9.1 lists "diagrams or screenshots" as legitimate sources. Anchor semantics for diagram regions (bbox-based vs. labeled-region) are unspecified; pageability is a degraded form. Stage 3 schema work will need to decide whether SourceSection's anchor format is open or constrained.
- **Live source feeds.** Some institutional sources (CFR e-CFR, state statutes) have machine-readable live feeds. Whether Source Vault supports subscription-style ingestion or only file uploads is deferred.
- **Citation granularity.** The `span` field in SourceCitation (CM §1.5) is optional; whether sub-section spans are normatively REQUIRED for any object kind is decided per [`policy-object-model.md`](policy-object-model.md).

## Cross-references

- Concept model: [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.2–§1.5, §2.1.
- PRD: [`../VISION.md`](../VISION.md) §9.1, §12 Source-Vault user stories.
- Downstream specs that depend on this: [`policy-object-model.md`](policy-object-model.md), [`authoring-provenance.md`](authoring-provenance.md), [`change-impact.md`](change-impact.md), [`readiness-validation.md`](readiness-validation.md).
- Repo conventions: [`../../CONVENTIONS.md`](../../CONVENTIONS.md).
