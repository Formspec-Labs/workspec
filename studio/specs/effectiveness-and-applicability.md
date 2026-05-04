# Studio Spec: Effectiveness and Applicability

**Status:** draft (Stage 2 of [Implementation Roadmap](../VISION.md#17-implementation-roadmap))
**Date:** 2026-04-30
**Concept-model anchor:** [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.25 Effectiveness; §1.31 CanonicalSourceRef.
**PRD anchor:** [`../VISION.md`](../VISION.md) §6 (Mapping Contract), §9.5 (Validation Center).
**Depends on:** [`source-vault.md`](source-vault.md), [`policy-object-model.md`](policy-object-model.md), [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md).

## Why this spec exists

Real policy is **temporally and jurisdictionally patchwork**. The simple `effectiveStart / effectiveEnd` fields on a SourceVersion (per current `source-vault.md`) cannot represent:

- A federal regulation enjoined by a court order in one circuit only, effective immediately, on appeal.
- A state directive that supersedes paragraphs 4–7 of a prior version while leaving paragraphs 1–3 and 8–12 unchanged.
- An errata memo issued at 4:30pm Friday that supersedes a single sentence retroactively from the original effective date.
- A sub-regulatory letter that supersedes guidance only for specific applicant populations (e.g., elderly applicants in tribal jurisdictions).
- A workflow whose authority basis is partly federal (CFR), partly state (administrative code), partly local (county ordinance) — each with its own temporal validity.

Marco's persona-round-2 review surfaced this: "Every workflow-tooling vendor I have watched die in govtech died on jurisdictional and temporal patchworks." The simple `effectiveStart`/`effectiveEnd` fields cannot represent these realities. The Plan agent's review surfaced the **god-object risk**: if every entity (SourceVersion, PolicyObject, Mapping) carries its own copy of effectiveness fields, three places drift.

This spec defines **a single composable Effectiveness object** that SourceVersion / PolicyObject (where applicable) / Mapping reference by `effectivenessRef`. One canonical home; never copied. Updating jurisdictional or temporal scope is one edit, not three.

## Scope

This spec defines:

- The **Effectiveness object** data model.
- The **referencing rules** (who carries `effectivenessRef`, who doesn't, and why).
- The **composition rules** with parent WOS machinery (especially `wos-delivery.schema.json#appliesWhen` FEL).
- The **conflict resolution rules** when multiple Effectiveness objects intersect.
- The **runtime semantics** the published `$wosWorkflow` artifact obtains from compiling Effectiveness-bearing PolicyObjects.

## Out of scope

- **Effectiveness inheritance across federation.** When two workspaces share a SourceDocument (federation, §1.34, deferred), inheritance of effectiveness across them is Phase-4-and-later.
- **Time-travel queries** ("show me the workflow as it was effective on 2025-08-15"). Reservable as a future capability; this spec stores enough to enable it but doesn't specify the query surface.
- **Automated supersession detection.** Whether one Effectiveness supersedes another is reviewer-driven (per `source-vault.md` `SA-MUST-source-007`); this spec does NOT auto-supersede.
- **Court-decision feed integration.** A real product ingests court rulings (PACER, state appellate sites) to update appellate state. The seam is `appellateState`'s `on-appeal | enjoined | final` enum; the feed integration is product-specific.

## Terminology

- **Effectiveness** — the single canonical object.
- **Jurisdiction** — a (kind, code) pair: `(federal, US)`, `(circuit, EDTX)`, `(state, CA)`, `(local, Cook County IL)`, `(tribal, Navajo Nation)`.
- **Temporal scope** — set of intervals where an Effectiveness is in force.
- **Appellate state** — the appeal posture: `final` (no pending appeal), `on-appeal` (under appeal but not stayed), `enjoined` (stayed/blocked in part), `provisional` (not yet effective; e.g., proposed reg).
- **Enjoined scope** — when `appellateState = enjoined`: WHICH jurisdictions are stayed (federal court orders may stay nationally, in one circuit, or in one district).
- **Effectiveness narrowing** — a Mapping or PolicyObject's Effectiveness narrower than its source's Effectiveness. (E.g., a state-specific PolicyObject deriving from a federal regulation has narrower jurisdiction.)
- **Effectiveness widening** — a Mapping or PolicyObject's Effectiveness wider than its source's Effectiveness. **Always disallowed** — you can't widen authority you don't have.

## Data model

### `Effectiveness`

```text
Effectiveness {
  id, workspaceId,
  jurisdictions[] {                // closed set of authoritative scope
    kind ('federal' | 'state' | 'circuit' | 'district' | 'local' | 'tribal' | 'territorial' | 'agency-specific'),
    code,                          // controlled vocab (ISO 3166-2:US-CA, EDTX for E.D. Texas, etc.)
    displayName,
    notes?
  },
  temporalScope {                  // when in force
    intervals[] { start, end? },   // open-ended end means "until superseded"
    sunsetAt?,                     // end of effectiveness (e.g., a sunsetting regulation)
    retroactiveFrom?               // when an errata/correction applies retroactively to a prior date
  },
  appellateState ('final' | 'on-appeal' | 'enjoined' | 'provisional'),
  enjoinedScope? {                 // when appellateState = enjoined
    enjoinedJurisdictions[],       // subset of jurisdictions[]
    enjoinedAt,
    enjoinedBy,                    // citation to court order or stay
    expectedResolutionAt?
  },
  applicabilityScope? {            // optional narrowing predicate (orthogonal to jurisdiction)
    appliesTo,                     // e.g., 'elderly-applicants', 'household-with-disability', 'tribal-members'
    appliesToCriteriaRef           // citation backing the narrowing
  },
  supersedingRef?,                 // Effectiveness id this supersedes
  supersededByRef?,                // Effectiveness id that supersedes this
  supersessionReason?,             // free-text rationale (review-pipeline gates this)
  createdBy, createdAt, lastEditedBy, lastEditedAt,
  provenance                       // AuthoringProvenanceRecord refs
}
```

### Carriers (who references Effectiveness)

- **SourceVersion** — every SourceVersion carries `effectivenessRef`. (Replaces the simpler `effectiveStart`/`effectiveEnd` fields.)
- **PolicyObject** (where applicable) — PolicyObjects whose applicability is narrower than their source's MAY carry `effectivenessRef`. PolicyObjects that simply inherit the source's effectiveness do NOT carry one — they read through to the source's. This is the **inheritance-by-default** rule that prevents drift.
- **Mapping** (`StudioToWosMapping`) — Mappings MAY carry `effectivenessRef` when the mapping itself is jurisdictionally scoped (e.g., a NoticeRequirement maps to `governance.notices[*]` only when the case is in Texas, because the Spanish-translation rule is state-mandated).
- **WorkflowIntent** — a WorkflowIntent MAY carry a workspace-level `effectivenessRef` declaring the workflow's overall scope (the union or intersection of its constituent parts).
- **Conflict** — Conflict resolutions MAY cite Effectiveness ranges (e.g., "this Conflict applies only when both PolicyObjects' effectiveness includes 2026-Q1").

### Carriers (who does NOT reference Effectiveness)

- **SourceDocument** — documents are containers; their VERSIONS have effectiveness.
- **SourceSection** — sections inherit from their parent SourceVersion.
- **SourceCitation** — citations are pointers; they don't have independent effectiveness.
- **AuthoringProvenanceRecord** — records of authoring events are timestamped but don't have an effectiveness scope (the events happened, period).
- **ValidationFinding, ApprovalDecision, ChangeImpactReport, ApprovalPackage** — workspace-state artifacts; no effectiveness.

## Lifecycle

An Effectiveness object lifecycle:

```text
draft → reviewed → approved → { active | superseded | retracted }
active → superseded                    (a later Effectiveness supersedes this)
active → retracted                     (court order vacates; reviewer retracts)
superseded (terminal)
retracted (terminal)
```

A `provisional` appellate state is orthogonal: an Effectiveness in `appellateState = provisional` is `active` lifecycle but not yet enforceable; `lifecycleState = active + appellateState = provisional` means "approved as a forward-dated effectiveness."

## Normative Contract

### Composition rules

- **`SA-MUST-eff-001`** — Every SourceVersion MUST carry `effectivenessRef` (replaces the simpler `effectiveStart` / `effectiveEnd` on SourceVersion). Migration: existing SourceVersions with simple fields are migrated by creating a default Effectiveness object with `jurisdictions = [{kind: 'federal', code: 'US'}]`, `temporalScope = {intervals: [{start: effectiveStart, end: effectiveEnd}]}`, `appellateState = 'final'`. Pre-migration data MAY remain readable; post-migration data MUST use Effectiveness. *(substrate-pending: data-migration of legacy fields.)*
- **`SA-MUST-eff-002`** — A PolicyObject MAY carry `effectivenessRef` only if its applicability is **narrower** than its source(s). PolicyObjects that match their source's effectiveness MUST NOT carry an Effectiveness ref (use inheritance). Implementations MUST flag PolicyObjects that wrap their source's effectiveness verbatim in a separate Effectiveness object as a tier-S2 ValidationFinding (`EFF-LINT-001`, "redundant effectiveness duplicate"). (`EFF-LINT-001` enforces.)
- **`SA-MUST-eff-003`** — Effectiveness widening is **disallowed**. A PolicyObject's or Mapping's Effectiveness MUST be a subset of its source(s)' Effectiveness on every dimension (jurisdictions, temporal scope, appellate state). Widening MUST be rejected at creation with `effectiveness-widening-disallowed`. (`EFF-LINT-002` enforces.)
- **`SA-MUST-eff-004`** — A Mapping carrying `effectivenessRef` MUST narrow its source PolicyObject's Effectiveness; the narrower scope MUST be reflected in the compiled `$wosWorkflow` artifact via a `wos-delivery.schema.json#appliesWhen` FEL expression. (See §"WOS mappings" below; this is the **slight composition with parent machinery** that v3's plan-agent review surfaced as the right composition.) *(substrate-pending.)*

### Conflict resolution

- **`SA-MUST-eff-010`** — When two PolicyObjects with overlapping jurisdictions and overlapping temporal scope produce conflicting requirements, the implementation MUST surface a Conflict per [`policy-object-model.md`](policy-object-model.md) §"Conflict surface". Effectiveness intersection is the *predicate* for conflict detection; resolution is reviewer-driven. *(substrate-pending.)*
- **`SA-MUST-eff-011`** — When `appellateState = enjoined` covers the case's jurisdiction at runtime, the enjoined PolicyObject MUST NOT be applied. Implementations MUST emit a runtime FactsTier provenance record `effectiveness-injunction-respected` for the case. *(substrate-pending; deferred to compiler + runtime.)*
- **`SA-MUST-eff-012`** — Cross-document supersession (per `source-vault.md` `SA-MUST-source-006/007`) MUST update the Effectiveness `supersededByRef` on the superseded document's Effectiveness. The Supersession PolicyObject is the *trigger*; this rule is the *consequence*. *(substrate-pending.)*

### Migration / errata

- **`SA-MUST-eff-020`** — Errata that retroactively change effectiveness (e.g., "this regulation was effective 2025-01-01, not 2025-04-01 as previously published") MUST set `temporalScope.retroactiveFrom` and emit a ChangeImpactReport per [`change-impact.md`](change-impact.md). The retroactive change requires reviewer attestation; it is NOT auto-applied. *(substrate-pending.)*
- **`SA-MUST-eff-021`** — Sunsetting effectiveness (regulation expires on 2027-12-31) MUST be modeled by setting `temporalScope.sunsetAt`; the readiness engine MUST surface a tier-S6 finding 90 days before sunset that workflows depending on this Effectiveness will need migration. (`EFF-LINT-005` enforces.)

## Composition

### Attachment point

Effectiveness attaches at the Workspace level (workspace-scoped objects). The Effectiveness registry is per-workspace; cross-workspace Effectiveness sharing is federation (§1.34, deferred).

### Precedence

Where a SourceVersion's Effectiveness conflicts with a derived PolicyObject's Effectiveness (cannot widen): the SourceVersion's wins; the PolicyObject's narrowing applies only where it intersects. Implementations MUST detect widening attempts and reject.

Where two Mappings of the same PolicyObject carry different `effectivenessRef`s pointing to overlapping scopes: the implementation MUST treat this as a `mapping-effectiveness-collision` tier-S3 ValidationFinding requiring reviewer resolution.

### Composition with parent WOS machinery

This spec composes (does not replace) the parent `wos-delivery.schema.json#appliesWhen` mechanism:

- A workflow's case-level routing — "Is this case in Texas? Use the Texas notice template" — is expressed in `wos-delivery.schema.json#appliesWhen` as an FEL expression.
- A PolicyObject's effectiveness — "This NoticeRequirement is in force only when the case's jurisdiction matches Texas AND the date is between 2025-01-01 and 2026-12-31" — is expressed as an Effectiveness object.
- The compiler emits an `appliesWhen` FEL expression DERIVED FROM the Effectiveness object, narrowing the WOS notice's runtime applicability accordingly.

The slight WOS-side extension — making `wos-delivery.schema.json#appliesWhen` carry an explicit jurisdictional expression rather than an arbitrary FEL — is queued in [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md) ExtensionRecord candidates. Until ratified, the Studio compiler emits a generated FEL expression that does the same job.

### Versioning / migration

- Adding a new `jurisdictions[].kind` enum value: schema-breaking; coordinated with parent jurisdictional taxonomy.
- Adding fields to `Effectiveness` body: non-breaking if optional.
- Changing the inheritance-by-default rule (`SA-MUST-eff-002`): would force massive re-authoring; do not change post-1.0.

## Conformance

### Schema validation (Stage 3)

- Effectiveness required-fields and lifecycle enum.
- `jurisdictions[].kind` enum constraints.
- `temporalScope.intervals` shape.
- `appellateState` enum + cross-field constraint (when `enjoined`, `enjoinedScope` is required).

### Lint rules (Stage 4)

Tier-S2 (Policy readiness) rules planned:

- `EFF-LINT-001` — redundant effectiveness duplicate (PolicyObject wraps source effectiveness verbatim).
- `EFF-LINT-002` — effectiveness widening disallowed (PolicyObject/Mapping widens its source).
- `EFF-LINT-003` — enjoined-but-no-enjoinedScope (when `appellateState = enjoined` but `enjoinedScope` is missing).
- `EFF-LINT-004` — mapping effectiveness collision (two mappings of the same PolicyObject with overlapping but conflicting scopes).
- `EFF-LINT-005` — sunset window (tier-S6: workflow depends on Effectiveness sunsetting in <90 days).

### Runtime conformance fixtures (Stage 4–5)

- Effectiveness with simple federal-final scope compiles to no `appliesWhen` narrowing.
- Effectiveness with Texas-only scope compiles to an `appliesWhen` expression narrowing to Texas cases.
- Effectiveness with `appellateState = enjoined` in EDTX compiles to an `appliesWhen` that excludes EDTX cases.
- Effectiveness widening attempt is rejected.
- Cross-document supersession updates `supersededByRef`.

### Current limitations

- The closed `jurisdictions[].kind` enum may be insufficient for rare cases (consortium jurisdictions, treaty obligations); extension via `x-` is reserved.
- `enjoinedScope` is currently a list of jurisdictions; partial-paragraph injunctions (one paragraph of a regulation enjoined while others stand) are not yet modeled — would require finer-grained Effectiveness on PolicyObject portions, deferred.
- Time-travel querying ("workflow as effective on 2025-08-15") is enabled by the data model but the query surface is not specified.

## WOS mappings

Effectiveness is **`authoringOnly`** as an object — the Effectiveness record itself never appears in `$wosWorkflow`. But its SEMANTICS project to WOS via two paths:

| Studio path | Mapping state | WOS path |
|---|---|---|
| Effectiveness applied to a NoticeRequirement | Compose with `mapsToWos` | Compiler emits derived FEL `appliesWhen` on `governance.notices[*]` (via `wos-delivery.schema.json` integration) |
| Effectiveness applied to a Deadline | Compose with `mapsToWos` | Compiler emits derived FEL on the timer's `appliesWhen` |
| Effectiveness applied to a workflow-level WorkflowIntent | Compose with `mapsToWos` | Compiler emits an artifact-level `applicabilityScope` (proposed slight WOS extension; until ratified, compiled into the artifact's release notes + `x-wos-studio` envelope) |
| Effectiveness with `appellateState = enjoined` | Compose with `mapsToWos` | Compiler emits an `appliesWhen` excluding enjoined jurisdictions; runtime FactsTier emits `effectiveness-injunction-respected` per case |

### Slight WOS-side extension queued

A new `wos-workflow.schema.json` field `applicabilityScope` (workflow-level jurisdictional scope) is the cleanest landing for workflow-level Effectiveness. **ExtensionRecord candidate** in [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md). Until ratified, the compiler uses the `x-wos-studio.applicability` extension envelope.

## Examples

### Example 1: Federal SNAP regulation effective in all states

```text
Effectiveness {
  id: "eff-7cfr273-current",
  jurisdictions: [{kind: "federal", code: "US", displayName: "United States"}],
  temporalScope: { intervals: [{start: "2025-01-01"}] },  // open-ended; until superseded
  appellateState: "final"
}
```

A SourceVersion of 7 CFR §273 carries `effectivenessRef = eff-7cfr273-current`. PolicyObjects derived from it inherit unless they narrow.

### Example 2: State directive narrowing federal regulation

```text
Effectiveness {
  id: "eff-tx-snap-spanish-notices",
  jurisdictions: [{kind: "state", code: "US-TX", displayName: "Texas"}],
  temporalScope: { intervals: [{start: "2025-04-01"}] },
  appellateState: "final"
}
```

A NoticeRequirement requiring Spanish-translation in Texas SNAP cases carries `effectivenessRef = eff-tx-snap-spanish-notices`. Compiler emits `appliesWhen: jurisdiction == "US-TX"` on the corresponding `governance.notices[*]` entry.

### Example 3: Federal regulation enjoined in one circuit

```text
Effectiveness {
  id: "eff-7cfr273-postcurrent-edtx-enjoined",
  jurisdictions: [{kind: "federal", code: "US"}],
  temporalScope: { intervals: [{start: "2026-03-15"}] },  // post-injunction
  appellateState: "enjoined",
  enjoinedScope: {
    enjoinedJurisdictions: [{kind: "circuit", code: "US-EDTX", displayName: "E.D. Texas"}],
    enjoinedAt: "2026-03-15",
    enjoinedBy: "Smith v. USDA, 2026 WL 12345 (E.D. Tex. 2026), preliminary injunction"
  }
}
```

The PolicyObject derived from this regulation, when applied at runtime, MUST not be enforced for cases in EDTX. Compiler emits `appliesWhen: !(jurisdiction MATCHES 'US-EDTX')` on the relevant `governance.notices[*]` / `lifecycle.transitions[*]`.

### Example 4: Errata applying retroactively

```text
Effectiveness {
  id: "eff-state-manual-ch8-corrected",
  jurisdictions: [{kind: "state", code: "US-CA"}],
  temporalScope: {
    intervals: [{start: "2026-04-15"}],
    retroactiveFrom: "2026-01-01"
  },
  appellateState: "final",
  supersedingRef: "eff-state-manual-ch8-original"
}
```

The errata supersedes the original effective from 2026-01-01 (retroactive). All cases adjudicated between 2026-01-01 and 2026-04-15 under the original Effectiveness need ChangeImpactReport review. The retroactive flag triggers `triggerKind = jurisdictional-supersession` per `change-impact.md` (post-tighten).

## Open issues

- **Partial-paragraph injunctions.** When a court enjoins one paragraph of a regulation while leaving others, the Effectiveness object can scope by jurisdiction but not by source-section. Workaround: split the SourceVersion into multiple PolicyObjects each with its own Effectiveness. Deeper modeling deferred.
- **Court decisions as Effectiveness sources.** A court decision that interprets a regulation IS an authority and shifts effectiveness, but it's not the regulation itself. Currently modeled as a separate SourceDocument with citing Supersession. May warrant a richer model in v5.
- **Federation of Effectiveness.** Two workspaces sharing a SourceDocument should agree on its Effectiveness. Federation deferred (§1.34).
- **Effectiveness inference from source text.** AI extraction of effective dates from regulation prose is error-prone; reviewer attestation is the contract for `validity` of an Effectiveness, not auto-extraction.
- **Time-travel queries.** The data model supports them; a Stage-4 query surface is reserved.

## Cross-references

- Concept model: [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.25 Effectiveness; §1.31 CanonicalSourceRef; §3 mapping states.
- Companion PRD: [`../VISION.md`](../VISION.md) §6 (Mapping Contract).
- Upstream: [`source-vault.md`](source-vault.md) (every SourceVersion carries `effectivenessRef`).
- Downstream: [`policy-object-model.md`](policy-object-model.md) (Effectiveness narrowing rules), [`change-impact.md`](change-impact.md) (jurisdictional-supersession triggerKind), [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md) (`appliesWhen` projection + ExtensionRecord candidates), [`compiler-contract.md`](compiler-contract.md) (compilation derives FEL `appliesWhen` from Effectiveness).
- Composes with: parent [`schemas/sidecars/wos-delivery.schema.json#appliesWhen`](../../schemas/sidecars/wos-delivery.schema.json), parent [`specs/governance/due-process-config.md`](../../specs/governance/due-process-config.md) `continuationOfServices` / `escalationPath` for appellate-state semantics.
- Repo conventions: [`../../CONVENTIONS.md`](../../CONVENTIONS.md).
