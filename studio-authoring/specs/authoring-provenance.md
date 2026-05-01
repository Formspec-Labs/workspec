# Studio Spec: Authoring Provenance

**Status:** draft (Stage 2 of [Implementation Roadmap](../VISION.md#17-implementation-roadmap))
**Date:** 2026-04-30
**Concept-model anchor:** [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.5 SourceCitation, §1.7 PolicyObject (`provenance` field), §1.10 ReviewerResolution, §1.15 ApprovalDecision.
**PRD anchor:** [`../VISION.md`](../VISION.md) §12 ("Authoring Provenance" user stories), §16 Phase-2 Epic 2.6.
**Depends on:** [`source-vault.md`](source-vault.md), [`policy-object-model.md`](policy-object-model.md).

## Scope

Authoring Provenance is the **append-only audit trail** that records *where each workflow element came from* — the chain from a source passage through extraction, review, approval, mapping, scenario validation, and publication. It is the answer to "Why does this workflow step exist?" (PRD §9.3 graph user story) and "Who approved this interpretation and when?" (PRD §12 auditor user story).

This spec defines:

- the AuthoringProvenance record shape and what events it captures;
- how provenance edges connect SourceCitation, ExtractedClaim, PolicyObject, StudioToWosMapping, WorkflowIntent, Scenario, ApprovalDecision, and PublishedWorkflowPackage;
- the **origin classification** that distinguishes workflow elements derived from source, approved interpretation, local practice, or assumption;
- the **projection rule** that determines what subset of authoring provenance is emitted into the published `$wosWorkflow` artifact's WOS provenance records vs. what stays in Studio metadata;
- composition with [`source-vault.md`](source-vault.md), [`policy-object-model.md`](policy-object-model.md), and [`change-impact.md`](change-impact.md).

This spec is the **glue** between Studio's authoring lifecycle and WOS's runtime provenance. Studio emits authoring records; Trellis (downstream of WOS) anchors runtime records (per parent [`../../CLAUDE.md`](../../CLAUDE.md): "Trellis anchors; WOS emits"). Authoring Provenance projects compactly into the WOS provenance config so that runtime audit can resolve back through to source citations.

## Out of scope

- Trellis custody anchoring (parent CLAUDE.md and [`../../specs/kernel/custody-hook-encoding.md`](../../specs/kernel/custody-hook-encoding.md)).
- WOS runtime provenance record format (lives in `wos-workflow.schema.json` and the kernel/governance specs).
- The audit log UI.

## Terminology

- **Authoring event** — a recorded act in the workspace: upload, extract, normalize, review, approve, reject, merge, split, map, validate, scenario-test, publish.
- **Provenance edge** — a typed link from one Studio object to its source.
- **Origin class** — one of `source` | `approved-interpretation` | `local-practice` | `assumption` | `runtime-observed` (Phase 4). Every workflow element carries exactly one.
- **Projection** — the subset of authoring provenance that is emitted into the published WOS artifact.
- **Anchor** — a stable reference into a SourceVersion; anchors survive supersession-with-dispute via re-verification.

## Data model

### AuthoringProvenanceRecord

Every Studio object that participates in authoring (PolicyObject, StudioToWosMapping, WorkflowIntent element, Scenario, ApprovalDecision, PublishedWorkflowPackage) carries a `provenance` field that is a **list** of AuthoringProvenanceRecord entries. The list is **append-only**.

```text
AuthoringProvenanceRecord {
  id, recordedAt, recordedBy (subjectId per identity-and-attestation.md), role,
  eventKind, eventSubtype?,
  payload,
  parentRecordIds[], originClass,
  hashChain {                       // cryptographic integrity (composes parent custodyHook)
    prevRecordHash,
    selfHash,
    anchoredAt?,                     // when this record was last anchored to Trellis via custodyHook
    custodyAppendReceiptRef?         // CustodyAppendReceipt.canonical_event_hash per parent ADR-0061
  }
}
```

Where:

- `eventKind` is one of: `extracted` | `normalized` | `reviewed` | `approved` | `rejected` | `merged` | `split` | `mapped` | `mappingStateAssigned` | `validated` | `findingRaised` | `findingResolved` | `findingWaived` | `scenarioTested` | `published` | `superseded` | `demoted` | `editedBody`.
- `payload` is event-kind-specific; e.g., for `mapped`: `{mappingState, targets[], extensionRecordRef?, unmappedRationale?}`.
- `parentRecordIds[]` chains to the prior provenance records that this event depends on (multi-parent for merges; single-parent for normal advances; none for genesis events like `extracted`).
- `originClass` is the origin classification (see below). Recorded redundantly on every event so a downstream consumer doesn't have to walk the chain.

### Origin classes (normative)

Every PolicyObject and every WorkflowIntent element MUST carry exactly one `originClass`:

- **`source`** — derived directly from approved SourceCitations. The default and most common.
- **`approved-interpretation`** — the citation supports a more general claim; the specific workflow shape was inferred by reviewer judgment and approved. Distinguished from `source` because the source does not literally say what the workflow does, only what the workflow must accomplish.
- **`local-practice`** — established by the operating organization, not by external policy. E.g., "this office reviews all denials twice." MUST NOT be promoted out of authoring-only mappings without explicit reviewer attestation that the local practice is policy-permissible.
- **`assumption`** — backed by an approved Assumption (CM §1.8), not by source citation.
- **`runtime-observed`** (Phase 4 only) — promoted from a RuntimeObservation that was reviewed and accepted as a workflow improvement. Distinguished from the above three because the basis is observed practice, not policy.

### AI-assisted extraction subtype

When `eventKind ∈ {extracted, normalized}` AND the action was performed by AI (not a human reviewer), the AuthoringProvenanceRecord MUST carry an `eventSubtype = "ai-assisted"` and an additional `aiLineage` block:

```text
aiLineage {
  modelId,                          // e.g., 'claude-opus-4-7'
  modelVersion,                     // e.g., '20260301'
  modelVersionPolicy,               // 'pinned' | 'approved' | 'latest' (per parent ai-integration.md §3.4)
  promptTemplateRef,                // pointer to the prompt template used
  promptTemplateVersion,
  temperature?,                     // sampling parameter (when applicable)
  seed?,                            // when reproducibility was sought
  toolUse[]?,                       // capabilities the model invoked (per parent agent-config.md)
  confidence?,                      // model-reported confidence (when available)
  inputContextHash,                 // hash of the context the model saw (privacy-redacted)
  humanApprover?,                   // when this AI action was reviewed and approved
  humanApprovedAt?,
  humanRationale?
}
```

The `aiLineage` block is the **audit-boundary closure** Marco identified: AI proposes ⇒ humans approve ⇒ both are recorded. Without this, AI authorship leaves no audit trail. With it, every AI-extracted claim can be traced to the model + prompt + reviewer + rationale.

- **`SA-MUST-prov-070`** — Every AuthoringProvenanceRecord with `recordedBy` resolving to an agent-typed actor (per parent `ai-integration.md`) MUST carry `aiLineage`. AI-authored events without lineage MUST be rejected. *(schema-pending; runtime-pending.)*
- **`SA-MUST-prov-071`** — `aiLineage.modelId`, `modelVersion`, `promptTemplateRef`, `promptTemplateVersion`, and `inputContextHash` are REQUIRED. Other fields optional. *(schema-pending.)*
- **`SA-MUST-prov-072`** — `inputContextHash` MUST be computed over a privacy-redacted view of the model's context — fields with `dpv:` sensitivity classes MUST be redacted before hashing per workspace policy. The hash provides reproducibility AND privacy. *(runtime-pending.)*
- **`SA-MUST-prov-073`** — When the model version changes (per `modelVersionPolicy`), an `agentVersionChange` provenance record MUST be emitted referencing the prior model version (composition with parent `ai-integration.md` §3.4 `agentVersionChange`). *(runtime-pending.)*
- **`SA-MUST-prov-074`** — Promotion of an AI-extracted claim past `extracted` lifecycle state MUST require a human approver (`humanApprover` populated). AI-only promotion is disallowed. *(lint-pending: tier-S2.)*

### Cryptographic anchoring (composes parent custodyHook)

The Studio authoring audit log itself anchors to Trellis via the parent `custodyHook` four-field append wire surface (per [`../../specs/kernel/custody-hook-encoding.md`](../../specs/kernel/custody-hook-encoding.md), parent ADR-0061, parent PLN-0385). This closes Marcus's persona-round-2 concern: a workspace operator could be the litigation defendant; "MUST NOT be edited" by policy is not enough; cryptographic chain + external anchoring is.

- **`SA-MUST-prov-080`** — Every AuthoringProvenanceRecord MUST carry `hashChain.prevRecordHash` and `hashChain.selfHash`, computing a Merkle-chain over the workspace audit log. Records that fail to chain (where `prevRecordHash != predecessor.selfHash`) MUST surface as a tier-S6 ValidationFinding. *(schema-pending; runtime-pending.)*
- **`SA-MUST-prov-081`** — At workspace-policy-configurable intervals, Studio MUST emit a custody-hook append (per parent ADR-0061 four-field input: `caseId, recordId, eventType, record`) anchoring the workspace audit log's current head. The receipt's `canonical_event_hash` MUST be stored on the AuthoringProvenanceRecord at the head. Default cadence: every 1000 records OR every 24 hours, whichever first; configurable per WorkspacePolicy. *(runtime-pending.)*
- **`SA-MUST-prov-082`** — Studio's custody-hook event types MUST use the `wos.authoring.*` namespace per parent **PLN-0384** (`wos-event-types.md` taxonomy). The "Audit event catalog" subsection below enumerates the specific event types. *(coordination-pending: parent PLN-0384 ratification.)*
- **`SA-MUST-prov-083`** — Workspace audit log retention MUST satisfy parent custody-tier requirements (per `crates/wos-server/VISION.md` zero-trust posture: encrypted-at-rest, key-bagged per access class). *(deployment-environment configuration.)*

### Audit event catalog

Studio emits the following event types into the `wos.authoring.*` namespace via parent `wos-event-types.md` (composition; Studio adds; parent PLN-0384 ratifies the broader taxonomy):

| Event type | Description | Custody-anchored |
|---|---|---|
| `wos.authoring.source-uploaded` | New SourceDocument or SourceVersion uploaded | YES |
| `wos.authoring.source-superseded` | SourceVersion lifecycle transitioned to superseded | YES |
| `wos.authoring.claim-extracted` | ExtractedClaim created (often AI-assisted; see aiLineage) | YES |
| `wos.authoring.claim-approved` | ExtractedClaim promoted to PolicyObject | YES |
| `wos.authoring.policy-object-edited` | PolicyObject body edited | YES |
| `wos.authoring.policy-object-demoted` | PolicyObject demoted to draft (e.g., source superseded) | YES |
| `wos.authoring.mapping-assigned` | StudioToWosMapping record created | YES |
| `wos.authoring.mapping-state-changed` | mappingState transitioned | YES |
| `wos.authoring.scenario-authored` | Scenario created or edited | YES |
| `wos.authoring.scenario-tested` | Scenario simulated; pass/fail recorded | YES |
| `wos.authoring.finding-raised` | ValidationFinding produced | YES |
| `wos.authoring.finding-waived` | Tier-S6 finding waived (consults AuthorityGrant) | YES |
| `wos.authoring.approval-decided` | ApprovalDecision recorded | YES |
| `wos.authoring.workflow-published` | PublishedWorkflowPackage created | YES |
| `wos.authoring.change-impact-acknowledged` | ChangeImpactReport `acknowledged` lifecycle transition | YES |
| `wos.authoring.change-impact-closed` | ChangeImpactReport `closed` with closureRationale | YES |
| `wos.authoring.local-practice-attested` | A reviewer attests `originClass = local-practice` (high-assurance attestation level required) | YES |
| `wos.authoring.compliance-attested` | ComplianceAttestation recorded against ApprovalPackage | YES |

Custody-anchored events are subject to `SA-MUST-prov-080/081/082`. Anchoring frequency is workspace-policy-configurable; high-stakes events (compliance attestations, local-practice attestations) anchor immediately rather than batched.

### Compaction (immutable log + projection)

Compaction is allowed for the **projection** (what reviewers see in the UI / what auditors see in compact reports), NEVER for the underlying log. The Plan agent's review identified this as a litigation hazard if not separated.

- **`SA-MUST-prov-090`** — The underlying AuthoringProvenanceRecord log is **immutable**. No record may be deleted, modified, or compacted in the underlying log. Compaction operates on a derived projection; the projection is rebuildable from the log at any time. *(architectural commitment; runtime-pending.)*
- **`SA-MUST-prov-091`** — Compacted projections MAY summarize "redundant" runs (e.g., 47 successive `editedBody` events on the same draft compress to "47 edits between T1 and T2 by reviewer R") in reviewer-facing UI. The compacted form MUST display a "show full log" affordance that recomputes from the underlying log. *(runtime-pending.)*
- **`SA-MUST-prov-092`** — Workspace administrators MUST NOT have authority to compact the underlying log, regardless of WorkspacePolicy retention settings. The closest a workspace administrator can do is *delete the workspace entirely* (terminal state per [`workspace.md`](workspace.md) `SA-MUST-ws-004`); they cannot selectively prune. *(architectural commitment.)*
- **`SA-MUST-prov-093`** — Custody-anchored events (per audit event catalog above) MUST be retained for at least the parent ADR-0061 retention window (7 years default) regardless of workspace retention policy. The custody anchor's external anchor in Trellis means deletion locally MUST NOT erase the global audit chain. *(deployment-environment + parent custody-tier policy.)*

### PROV-O export

The **W3C PROV-O** vocabulary is the canonical interop format for provenance graphs. Auditors and regulators understand PROV-O directly. Studio adopts PROV-O as a **first-class export format** for AuthoringProvenanceRecord chains.

- **`SA-MUST-prov-100`** — Studio MUST be able to export the workspace audit log (or any sub-graph of it; e.g., "the provenance chain leading to NoticeRequirement N") in W3C PROV-O JSON-LD. The export composes the existing parent [`schemas/sidecars/wos-ontology-alignment.schema.json`](../../schemas/sidecars/wos-ontology-alignment.schema.json) PROV-O sidecar. *(runtime-pending.)*
- **`SA-MUST-prov-101`** — PROV-O export MUST be deterministic: identical workspace state produces byte-identical (modulo JSON key order) PROV-O graphs. *(fixture-pending.)*
- **`SA-MUST-prov-102`** — PROV-O exports MUST redact privacy-classified content (per DPV class membership and viewing-reviewer's authority grants per `identity-and-attestation.md`). The export carries the structural graph; redacted leaves are noted as such. *(runtime-pending.)*
- **`SA-MUST-prov-103`** — PROV-O export MUST include the cryptographic chain heads (custody-hook receipts) so that an external verifier can confirm the export covers the actual audit log without redirection. *(runtime-pending.)*

### Provenance edges

The chain from raw source to published artifact composes the following edges:

```text
SourceSection
  --(citedBy via SourceCitation)-->        ExtractedClaim
  --(promotedTo)-->                        PolicyObject
  --(mappedTo via StudioToWosMapping)-->   WorkflowIntent element
  --(exercisedBy)-->                       Scenario
  --(approvedBy via ApprovalDecision)-->   PublishedWorkflowPackage
```

Plus orthogonal edges from Assumption → PolicyObject (when origin is `assumption`), ReviewerResolution → Conflict (when a conflict was resolved en route), and ChangeImpactReport → any of the above (when supersession triggered re-review).

## The provenance chain (normative)

For any approved workflow element, walking the chain backwards MUST resolve to one of the following terminals:

1. A SourceCitation (origin `source` or `approved-interpretation`).
2. An approved Assumption (origin `assumption`).
3. A documented LocalPractice attestation (origin `local-practice`).
4. A reviewed RuntimeObservation (origin `runtime-observed`, Phase 4).

A workflow element whose chain does not resolve to one of these terminals MUST be flagged as a tier-S2 ValidationFinding (`unsupported-element`).

The chain is **deterministic**: given a workflow element, the implementation MUST produce the same chain regardless of when the query runs (modulo new events that extend the chain forward). Walking is read-only; it never mutates provenance.

## Lifecycle

Authoring provenance is **append-only**. There is no record-edit lifecycle. Records are added monotonically as events occur. The only transformations are:

- **Append** — a new record is added; existing records are immutable.
- **Compaction** — for storage efficiency, contiguous redundant records (e.g., five `editedBody` events within a minute by the same reviewer) MAY be summarized into a single record at workspace administrator discretion. Compaction MUST preserve the first and last events of any compacted run and MUST NOT lose any record carrying an `approved`, `rejected`, `mapped`, `validated`, `published`, or `superseded` event kind.
- **Replication** — records project (compactly) into the published artifact (see "Projection" below). Replication is read-only on the source; it produces a new record in the published artifact's provenance config.

There is no "delete." If a reviewer wishes to retract an action, they author a *compensating* record (e.g., `demoted`) — the original record stands.

## Normative Contract

### Append-only integrity

- **`SA-MUST-prov-001`** — Every workspace state change to a PolicyObject, StudioToWosMapping, WorkflowIntent element, Scenario, ApprovalDecision, or PublishedWorkflowPackage MUST emit one or more AuthoringProvenanceRecords. *(runtime-pending: state-change interceptor.)*
- **`SA-MUST-prov-002`** — AuthoringProvenanceRecords MUST NOT be edited or deleted after creation. The implementation MUST reject any update operation. *(schema-pending: immutable record; runtime-pending: write-barrier.)*
- **`SA-MUST-prov-003`** — Every record MUST carry `id`, `recordedAt`, `recordedBy`, `role`, `eventKind`, and `originClass`. *(schema-pending: required fields.)*
- **`SA-MUST-prov-004`** — `recordedAt` MUST be a server-side timestamp; client-side timestamps are advisory and stored separately if at all. *(runtime-pending.)*
- **`SA-MUST-prov-005`** — `parentRecordIds[]` MUST resolve to existing records within the same workspace. Dangling parent references MUST be rejected at append time. *(runtime-pending: foreign-key-style check.)*

### Origin classification

- **`SA-MUST-prov-010`** — Every PolicyObject and every WorkflowIntent element MUST have exactly one `originClass` set when its `lifecycleState` reaches `approved`. *(schema-pending: required-when-state.)*
- **`SA-MUST-prov-011`** — `originClass = source` MUST be supported by at least one SourceCitation in the citation chain. *(lint-pending: tier S2 readiness rule, cross-cutting with [`policy-object-model.md`](policy-object-model.md) `SA-MUST-pom-004`.)*
- **`SA-MUST-prov-012`** — `originClass = approved-interpretation` MUST be supported by at least one SourceCitation **and** at least one ReviewerResolution explaining how the interpretation was derived from the citation. *(lint-pending.)*
- **`SA-MUST-prov-013`** — `originClass = local-practice` MUST carry an attestation: a ReviewerResolution authored by a reviewer with `role` claiming attestation authority. *(lint-pending; role policy is workspace-scoped.)*
- **`SA-MUST-prov-014`** — `originClass = assumption` MUST be backed by at least one approved Assumption listed in the chain. *(lint-pending; cross-cutting with `SA-MUST-pom-004`.)*
- **`SA-MUST-prov-015`** — Origin class transitions are restricted: an object MAY move from `assumption` to `source` (when a citation is later established), or from `local-practice` to `source` (when an external policy is later identified). It MUST NOT move from `source` to `assumption` without an explicit demotion event recorded in provenance. *(runtime-pending: state-machine for originClass.)*

### Chain integrity

- **`SA-MUST-prov-020`** — Walking the chain from any approved workflow element MUST resolve to a SourceCitation, an approved Assumption, a LocalPractice attestation, or a reviewed RuntimeObservation. Chains that terminate elsewhere MUST be flagged as tier-S2 `unsupported-element` ValidationFindings. *(lint-pending.)*
- **`SA-MUST-prov-021`** — Every `approved` event record MUST link to the ApprovalDecision that approved the underlying object. *(runtime-pending.)*
- **`SA-MUST-prov-022`** — Every `mapped` event record MUST carry the assigned `mappingState` and (when `mapsToWos`) the target list. *(schema-pending.)*
- **`SA-MUST-prov-023`** — Every `validated` event record MUST list the readiness rules that were evaluated and their outcomes (pass / waive / fail). *(runtime-pending: validation-engine emission.)*
- **`SA-MUST-prov-024`** — On supersession of a SourceVersion, every PolicyObject citing the prior version MUST receive a `superseded` or `demoted` record (depending on the materiality of the change). *(runtime-pending; cross-spec with [`source-vault.md`](source-vault.md) and [`change-impact.md`](change-impact.md).)*

### Projection to WOS

The Studio→WOS compiler (Stage 5) emits authoring provenance into the published `$wosWorkflow` artifact's authoring-provenance configuration. **Not all provenance projects** — the projection rule is normative.

- **`SA-MUST-prov-030`** — The compiler MUST project the following records into the published artifact's provenance config: every `approved` ApprovalDecision; every `mapped` event with `mappingState ∈ {mapsToWos, requiresSpecExtension}`; every `validated` event for a published workflow element; every `published` event. *(runtime-pending: compiler emission.)*
- **`SA-MUST-prov-031`** — The compiler MUST NOT project the following: ExtractedClaim normalization events, intermediate `editedBody` events, rejected/withdrawn events, raw chat-context-derived candidate records, or session-state ephemera. These remain in the workspace audit log only. *(runtime-pending; cross-cutting with PRD §15 Risk #8 — chat as source of truth.)*
- **`SA-MUST-prov-032`** — Projected records MUST be **compactly encoded**: SourceCitation excerpts are replaced with `{sourceDocumentId, sourceVersionId, sectionAnchor, excerptHash}`; the verbatim excerpt body stays in the workspace. The hash is computed over the Unicode-NFC-normalized excerpt string. *(schema-pending: projection schema.)*
- **`SA-MUST-prov-033`** — A consumer of the published artifact MUST be able to verify, given the projection alone, that any cited claim resolves to a real SourceVersion + section anchor — even without access to the originating workspace — by checking `excerptHash` against an independently obtained section text. *(runtime-pending: verification harness.)*
- **`SA-SHOULD-prov-034`** — When a workspace is exported (e.g., for archive or transfer), the export SHOULD include both the workspace-state authoring provenance (full) and the projected published-state provenance (compact); the published artifact alone is not a complete authoring record.

### Audit

- **`SA-MUST-prov-040`** — Every workspace MUST carry a single, append-only authoring audit log that aggregates AuthoringProvenanceRecords across all objects. The log is queryable by `recordedBy`, `eventKind`, `subjectKind`, time range, and `originClass`. *(runtime-pending: indexing.)*
- **`SA-MUST-prov-041`** — Audit log entries MUST NOT be alterable; corrections are appended as compensating events. *(runtime-pending.)*
- **`SA-SHOULD-prov-042`** — Audit log queries SHOULD be answerable in plain language ("Why does this workflow step exist?", PRD §9.3) by walking the chain backward and rendering each step's `eventKind` + `recordedBy` + `recordedAt` + relevant `payload` fields.

## Composition

### Attachment point

Authoring Provenance attaches at every Studio object that participates in authoring. It is the cross-cutting spine that ties [`source-vault.md`](source-vault.md), [`policy-object-model.md`](policy-object-model.md), [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md), [`scenario-authoring.md`](scenario-authoring.md), [`review-and-approval.md`](review-and-approval.md), and [`change-impact.md`](change-impact.md) together. (Phase-4 forward reference: the `runtime-observed` origin class is reserved for when RuntimeObservation gains its own spec.)

The Studio→WOS compiler (Stage 5) is the **only** consumer that emits provenance outside the workspace. Compiler-emitted provenance lands in the WOS artifact's authoring-provenance configuration (a non-`x-` extension; whether it lives directly in `wos-workflow.schema.json` or in a sidecar/seam is a Stage-3 schema decision).

### Precedence

When two events arrive concurrently for the same subject (a rare but possible race), append order at the server timestamp wins; ties are broken by `id` lexical order. There is **no precedence rule for content** — append-only means events do not contradict, they only accumulate.

When `originClass` could plausibly be more than one value (e.g., a workflow element backed by both a citation and an assumption), `source` > `approved-interpretation` > `local-practice` > `assumption` > `runtime-observed` (later phase). The most-citation-grounded class wins. Reviewers MAY override, but the override is recorded as a ReviewerResolution in the chain.

### Conflict handling

Authoring Provenance does not surface "conflicts" in the [`policy-object-model.md`](policy-object-model.md) sense — it records *what happened*, not *what should happen*. If the chain shows two reviewers approving contradictory interpretations, that contradiction surfaces as a Conflict in the policy-object layer; the provenance log records both approvals faithfully.

### Versioning / migration

- Adding a new `eventKind` is **non-breaking** if the schema's `oneOf`/`enum` for `eventKind` is open or extended with backwards-compatible defaults; otherwise it is **schema-breaking**. The closure decision sits in Stage 3.
- Renaming or removing an `eventKind` is **schema-breaking**.
- Changing the projection rule (`SA-MUST-prov-030` / `031`) is **breaking** for any downstream consumer of published artifacts; it requires a published-artifact-format version bump.
- Changing `originClass` enum values is **schema-breaking**.

## Conformance

### Schema validation (Stage 3)

Planned schema gates:

- AuthoringProvenanceRecord required fields (`SA-MUST-prov-003`).
- `eventKind` enum closure (Stage 3 decision).
- `originClass` enum closure.
- Append-only enforcement at the schema level (records are immutable; updates rejected).
- Projection schema: compact citation form (`SA-MUST-prov-032`).

### Lint rules (Stage 4)

Tier-S2 readiness rules planned:

- `PROV-LINT-001` — every approved object has at least one `approved` provenance record (SA-MUST-prov-001).
- `PROV-LINT-002` — every approved object's chain resolves to a citation, assumption, or attestation (SA-MUST-prov-020).
- `PROV-LINT-003` — `originClass = approved-interpretation` carries a ReviewerResolution (SA-MUST-prov-012).
- `PROV-LINT-004` — `originClass = local-practice` carries an attestation (SA-MUST-prov-013).
- `PROV-LINT-005` — `originClass = source` does not carry conflicting `assumption`-only support (SA-MUST-prov-011 + SA-MUST-prov-015).

### Runtime conformance fixtures (Stage 4–5)

- Append-only enforcement: a record cannot be edited or deleted.
- Compaction preserves first/last events of a run and never loses load-bearing event kinds (`approved`, `mapped`, `validated`, `published`, `superseded`).
- Projection emits compact citations and excludes session-state ephemera.
- Chain walking is deterministic and resolves to a terminal.

### Current limitations

- The full `eventKind` enum is provisional; refinement in Stage 3 schema work is expected.
- The compiler-emitted projection schema is sketched but not yet bound to a concrete WOS path. Stage 3 schema work will determine whether projection lives in `wos-workflow.schema.json` directly, in a sidecar, or at the kernel `provenanceLayer` seam (named in [`../../CLAUDE.md`](../../CLAUDE.md) §"Six canonical kernel seams").

## WOS mappings

Authoring Provenance is **`authoringOnly`** as a whole, with a **compact projection** into the published artifact:

| Studio object | Mapping state | Target | Notes |
|---|---|---|---|
| AuthoringProvenanceRecord (full) | `authoringOnly` | — | Workspace state only |
| Projected provenance (per `SA-MUST-prov-030`) | `mapsToWos` | `$wosWorkflow` authoring-provenance config / kernel `provenanceLayer` seam | Compact form per `SA-MUST-prov-032` |
| `originClass` | `mapsToWos` (compact) | provenance record field | One of source / approved-interpretation / local-practice / assumption / runtime-observed |
| Reviewer identity (`recordedBy`, `role`) | `mapsToWos` (compact) | provenance record field | Exposed for downstream audit |
| Source citation excerpt body | `authoringOnly` | — | Kept in workspace; only `excerptHash` projects |

The projection is the **only** way Studio's authoring history reaches a runtime audit consumer. Trellis (per parent CLAUDE.md) anchors runtime evidence; Studio's contribution is the *authoring* evidence that makes the runtime record traceable to source.

Concrete WOS path: pending Stage-3 schema decision (one of: a new top-level `authoringProvenance` block in `wos-workflow.schema.json`, a sidecar, or a `provenanceLayer` seam plug-in named in [`../../CLAUDE.md`](../../CLAUDE.md)). This spec is **target-agnostic** with respect to the path; the projection rule is what is normative.

## Examples

### Example 1: Walking the chain backward — "Why does this 60-day appeal deadline exist?"

A reviewer asks the question against an approved AppealRight in a SNAP-eligibility workflow:

1. Look up the AppealRight's provenance log: `[…, mapped, approved, reviewed, extracted]`.
2. The `extracted` record's `parentRecordIds` resolves to the originating ExtractedClaim's provenance.
3. The ExtractedClaim's provenance terminates at a SourceCitation: SourceVersion = `7-CFR-273.10-2025`, SectionAnchor = `§273.15(c)(2)`, excerpt = "Claimants must request a fair hearing within 90 days of the notice of adverse action."
4. The ReviewerResolution at the `approved` record records: "Adopted 90-day deadline per AuthorityRank — 7 CFR §273.15 governs over the older state-level guidance citing 60 days."
5. Plain-language answer to the user: "This 90-day deadline comes from §273.15(c)(2) of the SNAP federal regulation. A reviewer chose it over a conflicting 60-day state guidance because federal regulation outranks state guidance in this workspace's authority order. Approved by R. Garcia (Compliance) on 2026-03-12."

### Example 2: Origin class transition — assumption to source

The workflow author drafts an Assumption: "If the applicant does not respond to a request for information within 14 days, the workflow auto-denies." This is reviewed and approved as `originClass = assumption` because no source establishes the 14-day rule.

Three months later, a new SourceVersion of the agency manual is uploaded; it now contains a paragraph establishing exactly this 14-day rule.

1. The reviewer authors a SourceCitation against the new section, attaching it to the existing PolicyObject backed by the Assumption.
2. The reviewer triggers an originClass transition: `assumption → source`. A `transitionedOriginClass` provenance record is appended.
3. The Assumption is marked `superseded` in its own lifecycle (it is no longer the basis; the citation is).
4. Tier-S2 readiness re-evaluates: the workflow no longer carries the high-severity unresolved-assumption finding for this element.

### Example 3: Compact projection in the published artifact

A published `$wosWorkflow` artifact for a permit-review workflow includes (in compact form) a provenance record for its denial-notice template:

```text
{
  "eventKind": "approved",
  "subjectKind": "NoticeRequirement",
  "originClass": "source",
  "approvedBy": "j.kim@agency.gov",
  "approvedAt": "2026-04-21T14:32:00Z",
  "citations": [
    {
      "sourceDocumentId": "permit-handbook-2026",
      "sourceVersionId": "v3.1",
      "sectionAnchor": "§4.7.2",
      "excerptHash": "sha256:9f8a..."
    }
  ]
}
```

A downstream auditor with access to `permit-handbook-2026` v3.1 can verify the claim by re-reading §4.7.2 and confirming the excerpt's hash matches; they do not need access to the originating Studio workspace.

## Open issues

- **WOS path for projection.** Whether projected provenance lives directly in `wos-workflow.schema.json`, in a sidecar (e.g., `wos-authoring-provenance.schema.json`), or at the `provenanceLayer` kernel seam is unsettled. Pending Stage-3 schema work and consultation with the kernel/governance specs.
- **Compaction policy.** The compaction allowance (`SA-MUST-prov-001` notes) is permissive — workspaces choose. Whether a per-workspace policy (compaction-window, retention duration) is configurable schema or hard-coded is unsettled.
- **Cross-workspace provenance.** When a PolicyObject is reused across workspaces (deferred per [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §6), its provenance history is bound to the original workspace. Whether the second workspace can see the first workspace's provenance, and how citation hashes carry across, is deferred.
- **Phase-4 origin transitions.** RuntimeObservation → workflow-element promotion (Phase 4) introduces the `runtime-observed` origin class. The transition rules (Phase 4 → published, Phase 4 → workspace-only improvement proposal) will be defined when the RuntimeObservation spec is written; until then, no PolicyObject should carry `originClass = runtime-observed`.

## Cross-references

- Concept model: [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.5, §1.7 (provenance field), §1.10, §1.15.
- PRD: [`../VISION.md`](../VISION.md) §9.3 (knowledge map), §16 Phase-2 Epic 2.6, §12 user stories.
- Upstream: [`source-vault.md`](source-vault.md), [`policy-object-model.md`](policy-object-model.md).
- Downstream: [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md), [`readiness-validation.md`](readiness-validation.md), [`change-impact.md`](change-impact.md), [`review-and-approval.md`](review-and-approval.md).
- WOS: [`../../specs/kernel/spec.md`](../../specs/kernel/spec.md) (`provenanceLayer` seam), [`../../specs/kernel/custody-hook-encoding.md`](../../specs/kernel/custody-hook-encoding.md).
- Repo conventions: [`../../CONVENTIONS.md`](../../CONVENTIONS.md).
