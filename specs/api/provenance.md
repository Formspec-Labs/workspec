# WOS Public API Provenance

**Status:** Schema authored — pending implementation pair (server + portal).
**Schema:** [`api/provenance.schema.json`](../../schemas/api/provenance.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/provenance/v1`
**OpenAPI:** [`wos-public-api.openapi.json`](../../api/wos-public-api.openapi.json) (server-emitted; not yet declared)
**ADR anchor:** [ADR 0082](../../../thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md) D-5 (tier-discriminated union), D-7 (cursor pagination), D-9 (`actorRef` URN), D-12 (closed taxonomies).

## Purpose

Provenance records are the public read-shape for the WOS audit trail of a single workflow process. The four tiers — Facts, Reasoning, Counterfactual, Narrative — surface the kernel and governance audit layers (`auditLayer` in the persisted log) as a typed discriminated union at the API boundary. The wire shape is greenfield per ADR 0082 D-15: the legacy `case-portal` `ProvenanceRecord` (single envelope with four optional tier blocks) is the explicit anti-design and is structurally inexpressible here.

## Resource Shape

`ProvenanceRecord` is a `oneOf` over four tier-typed variants discriminated by the literal `tier` field. Each variant is a complete object with no shared `allOf` envelope — the schema is greenfield-flat by design so codegen on both sides produces a typed discriminated union without hand-edits (ADR 0082 Notes section).

| Variant | `tier` literal | Spec source | Authoritative |
|---|---|---|---|
| `FactsTierRecord` | `facts` | Kernel §8 | yes |
| `ReasoningTierRecord` | `reasoning` | Governance §6.2 | yes |
| `CounterfactualTierRecord` | `counterfactual` | Governance §6.4 | yes |
| `NarrativeTierRecord` | `narrative` | AI Integration §13 | NO (`authoritative: false`) |

Identity uses URNs throughout (ADR 0082 D-4, D-9):

- `id` and `processId` are WOS resource URNs (`urn:wos:<typeid>`) per ADR 0092 D-1.
- `actorRef` is an `actor:<class>:<id>` URN. The legacy nested `actor: { id, type, name }` shape is not part of this contract; identity details live once in the identity / governance subsystem and every record references them by URN.
- `factsRecordRef` (on Reasoning, Counterfactual, Narrative variants) points to the Facts-tier record being interpreted, narrated, or analyzed.

`recordKind` exposes the kernel `FactsTierRecord.recordKind` enum (kernel S8.2.3 + ADRs 0066/0070/0071) as a closed taxonomy with vendor-prefix extension (`x-…`). Reserved literals MUST appear here when their kernel definitions ratify; vendor extensions never break compatibility.

`outcome`, `integrity.algorithm`, and `recordKind` use the closed-taxonomy + `^x-[a-z]+-` extension pattern from ADR 0082 D-12. No anonymous string fields.

### Facts typed payload obligations

`FactsTierRecord` stays one shared envelope. JSON Schema does not encode brittle `if recordKind then required payload` rules at the public boundary; the obligations are normative here and validators enforce the object shapes when the typed field is present.

| `recordKind` | REQUIRED typed field | Semantics |
|---|---|---|
| `overrideRecorded` | `overrideRecord` | Governance §7.3 structured rationale, authority verification, and supporting evidence. |
| `noticeSent` | `noticeSent` | Governance §3.2 byte-deterministic notice metadata. |
| `authorizationRejected` | `authorizationRejectedRecord` | ADR 0070 authorization denial: attempted actor, action, target, and rejection reason. |
| `legalHoldPlaced`, `legalHoldReleased`, `legalHoldDestructionRejected` | `legalHoldRecord` | Legal-hold placement/release/destruction-blocking event. A destruction attempt under active legal hold MUST emit `legalHoldDestructionRejected`, not silently skip deletion. |
| `circuitBreakerTripped`, `circuitBreakerReset` | `circuitBreakerEvent` | Advanced-governance circuit-breaker state transition for an agent. Trip records explain the threshold breach; reset records explain the governed recovery. |
| `shadowModeDivergence` | `shadowModeDivergence` | Shadow-mode output materially diverged from the configured baseline. The record does not make the shadow output authoritative; it is an audit signal and report input. |
| `dcrZoneViolation` | `dcrZoneViolation` | DCR constraint-zone condition/response/include/exclude/milestone violation. |
| `driftAlert` | `driftAlert` | Drift-monitor threshold crossing and resulting drift posture. |
| `reportTimedOut` | `reportTimedOut` | Report run exceeded `maxDurationSeconds`, failed with `WOS-1408`, and was terminated. |
| `equityAlert` | (lightweight inline payload — see schema `data.description`) | Equity guardrail threshold breach for a protected category. The `data` bag carries `guardrailId`, `protectedCategoryId`, `disparityScore`, `threshold`, and `occurredAt`. Consumed by the `equity-disparity` report generator (`reports.schema.json`). |
| `autonomyEscalation`, `autonomyDemotion` | (lightweight inline payload — see schema `data.description`) | Agent autonomy-level transition. The `data` bag carries `previousAutonomyLevel`, `newAutonomyLevel`, `trigger`, and optional `rationale`. The `AutonomyEvent` aggregate on `governance.schema.json` provides the governance-query view; this is the per-event audit record. |

Quorum reductions are not a new reserved `recordKind`. Governance §S4.9 says a quorum MUST NOT be silently reduced; when an implementation reduces quorum, it MUST emit an explicit policy-transition Facts record carrying `quorumReduction`. The parent kind remains the policy action that authorized the change, and the typed field carries `priorRequiredCount`, `newRequiredCount`, `authorityBasis`, and `actorRef`.

Lifecycle-control public literals are `instanceSuspended`, `instanceResumed`, and `instanceTerminated`. Each operation MUST emit the corresponding Facts record when the lifecycle-control mutation commits. `continuationOfServicesActivated` records the due-process fact that current service levels were preserved during an appeal window; the case projection exposes the current state via `continuationOfServicesActive` / `continuationOfServicesEndsAt`.

### Tier-specific required fields

The variants are flattened, not composed: each variant's `required` array enumerates exactly the fields it needs. Cold-read summary:

- **Facts.** `tier`, `id`, `processId`, `recordKind`, `timestamp`, `definitionVersion`, `event`. `actorRef` is optional (kernel S8 leaves the actor optional when the processor cannot name one). `caseFileSnapshot` is REQUIRED when `transitionTags` includes `determination` (kernel S8.2.1) — enforced at the runtime append path; the API schema records this as a `description`-level obligation rather than a conditional `if/then` because the kernel envelope already enforces it on the persisted log shape.
- **Reasoning.** Adds `rulesApplied` (`min 1`), `evidenceConsulted`, `criteriaChecked`. Required for `determination`-tagged transitions. Optional `assertionGates: AssertionGateResult[]` (governance §5.4 — closed `gateKind: source-grounded | arithmetic | range | consistency | format | cross-document | temporal` + `gatePassed: boolean` + `evidenceRefs: EvidenceReference[]`) and optional `confidenceReport: ConfidenceReport` (ai-integration.md §7.1 — `outputId` + `confidence` + `decayApplied` + `cumulativeConfidenceWindow`).
- **Counterfactual.** Adds `positiveCounterfactuals`, `negativeCounterfactuals`. Required for `adverse-decision`-tagged transitions in `rights-impacting` workflows.
- **Narrative.** Adds `narrative`, `authoritative` (MUST be the literal `false`), `modelIdentifier`, `modelVersion`. Per AI Integration §13.2, every implementation MUST treat narrative content as non-authoritative; the schema makes this structurally enforced via the `const false` discriminant. Optional `confidenceReport: ConfidenceReport` (ai-integration.md §7.1 applies to narrative output too — every agent output carries one).

### Resource shape — `AssembledExplanation`

`AssembledExplanation` is the server-assembled adverse-decision explanation projected as a typed read-only resource (governance §3.8.1; workflow-governance.md:166-238). The §3.8.1 algorithm runs deterministically server-side — two conformant processors MUST produce byte-identical output (workflow-governance.md:170) — and clients consume the result via `GET /api/v1/cases/{case_id}/processes/{process_id}/explanation`. Eliminates the prior client-side re-implementation of §3.8.1 ordering.

Required fields: `processId`, `assembledAt`, `assembledBy: ActorRef`, `narrative: prose`, `factsTrace: WosResourceUrn[]` (ordered Facts-tier record URNs walked by §3.8.1 step 1), `rulesApplied: RuleReference[]` (ordered by authority rank descending per §3.8.2). Optional: `positiveCounterfactuals`, `negativeCounterfactuals`, `transitionId`. Cross-`$ref`s `RuleReference` and `Counterfactual` from this same schema rather than redefining (ADR 0082 D-14).

### Resource shape — `AssertionGateResult` and `ConfidenceReport`

Both shapes embed on tier records and so do not have their own endpoints. `AssertionGateResult` lives on `ReasoningTierRecord.assertionGates`; `ConfidenceReport` lives on `ReasoningTierRecord.confidenceReport` AND on `NarrativeTierRecord.confidenceReport`. The closed `AssertionGateKind` enum carries the seven §5.4 literals verbatim. The `ConfidenceReport.cumulativeConfidenceWindow.below` boolean is the load-bearing observable that triggers the §7.7 multi-step pause-for-human-review MUST.

### Reused kernel/governance shapes

The API layer is a projection. Per ADR 0082 D-14 fourth bullet, kernel and governance record types are referenced rather than reinvented. Because typify and other codegens resolve cross-schema `$ref`s only when given an explicit schema-resolution map, this schema mirrors (rather than cross-`$ref`s) the structurally-stable shapes from `wos-workflow.schema.json`:

| API definition | Kernel / governance source |
|---|---|
| `RuleReference` | `wos-workflow.schema.json#/$defs/RuleReference` (governance S6.2) |
| `EvidenceReference` | `wos-workflow.schema.json#/$defs/EvidenceReference` (governance S6.2) |
| `CaseFileSnapshot` | `wos-workflow.schema.json#/$defs/CaseFileSnapshot` (kernel S8.2.1) |
| `FactsRecordKind` | `wos-workflow.schema.json#/$defs/FactsTierRecord.recordKind` (kernel S8) |
| `outcome` literals | `wos-workflow.schema.json#/$defs/ProvenanceOutcome` (kernel S8.2.2) |

Each mirrored definition links back to its kernel source in its `description`. CI (ADR 0082 D-13 gate 4) catches divergence: when the kernel definition changes, the API mirror must follow in the same PR or `oasdiff` flags a breaking change.

## Endpoints

| Method | Path | Body in / out |
|---|---|---|
| `GET` | `/api/v1/cases/{case_id}/processes/{process_id}/provenance` | `ProvenanceListOptions` (query) → `ProvenanceRecordPage` |
| `GET` | `/api/v1/cases/{case_id}/processes/{process_id}/provenance` | → `ProvenanceRecord` |
| `GET` | `/api/v1/cases/{case_id}/processes/{process_id}/explanation` | → `AssembledExplanation` |

`GET /api/v1/cases/{case_id}/processes/{process_id}/provenance` is cursor-paginated per ADR 0082 D-7 and accepts the `ProvenanceListOptions` query envelope. Filters compose with **AND semantics** — a record is returned only when it satisfies every supplied filter independently.

| Field | Type | Semantics |
|---|---|---|
| `tier` | `facts \| reasoning \| counterfactual \| narrative` | Optional tier filter. When omitted, all four tiers are returned. |
| `recordKindFilter` | `FactsRecordKind[]` (`minItems: 1`) | Optional record-kind filter against the closed-with-vendor-extension `FactsRecordKind` taxonomy declared in this schema (e.g. `["stateTransition"]`, `["caseStateMutation", "signatureAffirmation"]`). Empty arrays are rejected; omit the field to span every kind. |
| `timeRange` | `ProvenanceTimeRange { since, until }` | Optional inclusive UTC RFC 3339 window against `record.timestamp` (ADR 0069 wire format). Both bounds are REQUIRED when `timeRange` is supplied; omit the field entirely to span the full retention window. |
| `actorRefFilter` | `ActorRef[]` (`minItems: 1`) | Optional actor-URN filter (ADR 0082 D-9). A record matches when its `actorRef` equals one of the supplied principals. Empty arrays are rejected. |
| `cursor` | opaque token | Resume token returned by the previous page. A cursor is only valid against the filter set that issued it; the filter set MUST stay byte-identical across pages of one query. |
| `limit` | integer, max 200 | Page size hint. Server may return fewer records and still set `hasMore`. |

Response is `ProvenanceRecordPage`: `items: ProvenanceRecord[]`, optional `cursor`, required `hasMore`. No `total` (D-7). Cursors are deploy-lifetime stable; clients receiving `410 Gone` with `wosErrorCode: WOS-1410` MUST restart pagination from the top.

These filters share intent — but not wire shape — with `AuditQueryRequest` in [`audit.schema.json`](../../schemas/api/audit.schema.json): per-case provenance uses `tier` scalar / `actorRefFilter` array / `since,until`, while cross-case audit uses `tierFilter` array / `actorRef` scalar / `start,end`. The divergence predates this row; clients writing a generic query layer should treat the two domains as parallel surfaces rather than byte-equivalent shapes.

`GET /api/v1/cases/{case_id}/processes/{process_id}/provenance` returns a single `ProvenanceRecord` of any tier. The `recordId` path parameter is a provenance-record URN; `404` (with `wosErrorCode: WOS-1404`) when the record is not visible to the caller's scope (ADR 0082 D-8).

The aggregate `GET /api/v1/cases/{case_id}/processes/{process_id}?include=...` (ADR 0082 D-3) does not include `provenance` in its `include` enum. Provenance is volume-unbounded and always paginated — the aggregator never embeds it. Clients render "show recent activity" via a separate paginated call.

## Codegen

Verified against ADR 0082 Notes section (sharp edges around `oneOf` with shared base fields):

- **Rust (typify).** Local dry-run produces `pub enum ProvenanceRecord { FactsTierRecord(FactsTierRecord), ReasoningTierRecord(ReasoningTierRecord), CounterfactualTierRecord(CounterfactualTierRecord), NarrativeTierRecord(NarrativeTierRecord) }` with `#[serde(untagged)]`. The `tier` `const` discriminant in each variant guards deserialization deterministically. Rust output compiles with only `serde`, `serde_json`, `regress`, and `chrono` deps. No hand-edits required. The variant-naming difference from the ADR sketch (`FactsTierRecord(...)` vs `Facts(...)`) is a typify naming convention — the named field `tier` on each struct preserves `match record { ProvenanceRecord::FactsTierRecord(f) => f.tier, ... }` semantics.
- **TypeScript (`json-schema-to-typescript`).** Dry-run produces `export type ProvenanceRecord = FactsTierRecord | ReasoningTierRecord | CounterfactualTierRecord | NarrativeTierRecord` with `tier: "facts"` / `"reasoning"` / `"counterfactual"` / `"narrative"` literal-typed in each interface. `tsc --strict` confirms `if (record.tier === 'reasoning')` narrows to `ReasoningTierRecord` with `rulesApplied` non-optional, and `record.authoritative` narrows to the literal type `false` on the narrative variant.

Both codegens deliver the contract D-5 promised. The "flattened variants, no shared `allOf` envelope" choice is what makes this work; nesting tier blocks under a shared envelope (the legacy portal shape) breaks both codegens' narrowing.

## Greenfield discipline

Per ADR 0082 Context section and the owner's greenfield-contracts memory: the existing `case-portal` `ProvenanceRecord` (single envelope, four optional tier blocks, nested `actor: { id, type, name }`) is the explicit anti-design. This schema makes that shape impossible:

1. The `oneOf` over four flat variants forbids "all four tier blocks at once on one record."
2. The `tier` `const` literals forbid an envelope without a tier.
3. Tier-specific fields are non-optional within their variant — you cannot send a Reasoning record without `rulesApplied`.
4. `actorRef` is a URN string; the nested `actor` object is unrepresentable.

## Non-Goals

- **Cross-case provenance.** A separate `api/audit.schema.json` will own cross-case audit queries (ADR 0082 D-1 schema list).
- **Append / write paths.** This contract is read-only on the public API; provenance records are emitted by the runtime through internal seams (`provenanceLayer`), not by external clients.
- **Bundle export.** Trellis-side export bundles use `api/bundle.schema.json`; this domain only surfaces the live record stream.
- **Cursor durability across deploys.** Per ADR 0082 D-7 cursors are deploy-lifetime; clients restart pagination from the top after `WOS-1410`.

## ADR amendments

None required. The schema and codegen verification confirm D-5, D-7, D-9, D-12, D-13, and D-14 as ratified. The Notes-section sharp-edge insurance (flatten tier blocks rather than `allOf`-compose them) is the design choice this schema realizes.
