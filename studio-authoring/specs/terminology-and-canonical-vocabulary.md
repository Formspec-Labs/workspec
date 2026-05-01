# Studio Spec: Terminology and Canonical Vocabulary

**Status:** draft (Stage 2 of [Implementation Roadmap](../VISION.md#17-implementation-roadmap))
**Date:** 2026-04-30
**Concept-model anchor:** [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.30 TerminologyMap.
**PRD anchor:** [`../VISION.md`](../VISION.md) §3 (Primary users), §6 (Mapping Contract), §13 (Security).
**Depends on:** [`policy-object-model.md`](policy-object-model.md), [`source-vault.md`](source-vault.md).

## Why this spec exists

Two structural problems share a root cause:

1. **Sarah's complaint:** "23 entities is too many for my staff. I need plain-English names." This is a vocabulary gap between the spec set's internal terminology and the operational language reviewers use.
2. **Cross-workspace DataElement reuse:** When two workspaces both have a `DataElement` named "household income," is that the same canonical term? Or is workspace A's `householdIncome` distinct from workspace B's `monthlyHouseholdIncome`? Without a canonical-term registry, federation (§1.34) and citation deduplication are structurally broken.

Both problems are solved by **a canonical-term registry**: a workspace-scoped (eventually federated) registry that resolves Studio entities and DataElements to canonical IRIs, plus carries plain-English projections for non-technical users.

This spec also formalizes Studio's adoption of:
- **W3C DPV (Data Privacy Vocabulary)** for `DataElement.sensitivity` (replacing hand-rolled enum).
- **Schema.org / domain-specific vocabularies** for canonical references where they exist (e.g., USDA SNAP terminology, HHS HIPAA terminology, EU eIDAS for identity terms).
- **Plain-English projections** for every CONCEPT-MODEL entity.

## Scope

This spec defines:

- The **CanonicalTerm** data model.
- The **TerminologyMap** registry shape (workspace-scoped).
- The **DPV vocabulary adoption** for sensitivity classification.
- The **plain-English projection** for entities (Sarah's "operator language" layer).
- The **resolution rules** (how a Studio DataElement / entity name resolves to a CanonicalTerm IRI).
- The **federation precursor** (CanonicalTerm sharing across workspaces, deferred but framed).

## Out of scope

- Authoring the universal canonical vocabulary itself (that's a community / agency-led effort; Studio adopts).
- Auto-classification of free-text terms into canonical IRIs (AI-assisted; not specified here — falls under AI extraction in `authoring-provenance.md`).
- Multi-lingual canonical mapping (deferred; English baseline; locale-keyed projections in `source-vault.md` cover content multi-lingual support, not term-name multi-lingual).
- Federation-level term agreement protocols (deferred to §1.34 federation).

## Terminology

- **CanonicalTerm** — a stable identifier for a term that has authoritative meaning in a domain (e.g., `dpv:HealthData`, `usda-snap:HouseholdIncome`, `wos-snap:RedeterminationWindow`).
- **TerminologyMap** — a workspace-scoped registry mapping local Studio names to canonical IRIs.
- **Plain-English projection** — a non-technical operator-facing label and one-sentence operational description for every entity. Used in UI, audit log narratives, and reviewer-facing finding messages.
- **DPV** — W3C Data Privacy Vocabulary. The controlled vocabulary used for sensitivity classification.
- **Synonym** — a local name in this workspace pointing to a canonical IRI.
- **Resolution** — the act of looking up a local name and obtaining the canonical IRI (or marking it as un-resolved).

## Data model

### `CanonicalTerm`

```text
CanonicalTerm {
  iri,                              // stable IRI (e.g., 'dpv:HealthData', 'usda-snap:HouseholdIncome')
  displayName,                      // human-readable, locale-default English
  definition,                       // authoritative definition (one paragraph)
  domain,                           // 'privacy' | 'snap' | 'tanf' | 'medicaid' | 'ada' | 'gdpr' | 'studio-meta' | ...
  authoritativeSource?,             // SourceCitation; the document defining this term
  parentTerm?,                      // CanonicalTerm IRI; for hierarchical vocabularies (DPV is a graph)
  alsoSeeRefs[],                    // related terms (other vocab IRIs)
  dataType?,                        // when the term implies a data type (e.g., money, date, integer)
  unit?,                            // for measurable terms (e.g., 'USD/month' for income)
  versions[],                       // controlled-vocabulary versioning
  status ('canonical' | 'proposed' | 'deprecated' | 'superseded'),
  studioPlainEnglishProjection?     // user-friendly label + description (see below)
}
```

### `TerminologyMap` (workspace-scoped registry)

```text
TerminologyMap {
  workspaceId,
  entries[] {
    localName,                      // e.g., 'householdIncome' as used in this workspace
    canonicalIri,                   // resolves to a CanonicalTerm
    synonyms[],                     // other local names mapping to same canonical
    confidence,                     // 'reviewed' | 'auto-suggested' | 'manual-pending'
    mappedBy, mappedAt,
    rationale?
  }
}
```

### `PlainEnglishProjection`

```text
PlainEnglishProjection {
  conceptModelEntity,               // e.g., 'PolicyObject', 'StudioToWosMapping'
  operatorLabel,                    // e.g., 'Policy Item' for PolicyObject; 'Translation Table' for StudioToWosMapping
  operationalDescription,           // one sentence: 'A policy item is a thing the rule says you must do, may do, or must not do'
  exampleNarrative,                 // worked example in plain English
  audienceTier ('reviewer' | 'operator' | 'caseworker' | 'public')
}
```

The set of PlainEnglishProjections is finite — one per CONCEPT-MODEL entity. The set is shipped with Studio (not workspace-configured) so that all workspaces speak the same operator language. Workspace administrators MAY override projections via WorkspacePolicy (e.g., calling PolicyObjects "Policy Items" vs. "Rule Items" per local tradition).

## DPV adoption (W3C Data Privacy Vocabulary)

Studio's `DataElement.sensitivity` uses **W3C DPV** controlled-vocabulary IRIs as primary, with hand-rolled aliases preserved for backward continuity.

### Mapping table

| Hand-rolled (legacy alias) | DPV IRI (canonical) | Use case |
|---|---|---|
| `pii` | `dpv:PersonalData` | Generic PII |
| `phi` | `dpv:HealthData` | Health information (HIPAA-relevant) |
| `restricted` | `dpv:Identifier` | Strong identifiers (SSN, biometric) |
| (new) | `dpv:FinancialPreference` | Financial / income data |
| (new) | `dpv:Demographic` | Race, ethnicity, language (Title VI) |
| (new) | `dpv:Disability` | Disability data (ADA-relevant) |
| (new) | `dpv:HousingStatus` | Housing / homelessness |
| (new) | `dpv:LegalProceeding` | Court / litigation references |
| (new) | `dpv:GovernmentBenefit` | SNAP, TANF, Medicaid, etc. status |

### Why this matters

- **GDPR / CCPA / HIPAA mapping**: DPV is designed for legal-compliance interop. A DataElement marked `dpv:HealthData` automatically inherits HIPAA constraints; `dpv:PersonalData` inherits GDPR Article 4(1) treatment.
- **Automated retention/access policy derivation**: Workspace policy can attach default retention, access scopes, and audit requirements to DPV classes; new DataElements automatically inherit.
- **Privacy-engineering tool interop**: tools like OpenDPV, PrivacyChain, etc. consume DPV directly. Studio's DataElement annotations interop without translation.
- **Backward continuity**: existing PolicyObjects that use `pii | phi | restricted` enum values are accepted (legacy aliases); the implementation displays both legacy and canonical IRI; new PolicyObjects MUST use DPV IRI.

## Lifecycle

A CanonicalTerm lifecycle:

```text
proposed → reviewed → canonical → { deprecated | superseded }
```

Workspaces inherit canonical terms from a baseline registry (Studio ships a starter set covering DPV + common gov terminology) and MAY add workspace-specific terms via `proposed → reviewed → canonical`.

A TerminologyMap entry lifecycle:

```text
auto-suggested → manual-pending → reviewed → confirmed
```

Auto-suggested entries (AI-mapping local names to canonical IRIs) MUST progress to `reviewed` before they affect workflow shape; `confirmed` entries are stable.

## Normative Contract

### CanonicalTerm integrity

- **`SA-MUST-term-001`** — Every CanonicalTerm MUST carry a stable `iri`, `displayName`, `definition`, and `domain`. Terms missing required fields MUST be rejected at registration. *(schema-pending.)*
- **`SA-MUST-term-002`** — CanonicalTerm IRIs MUST be globally unique and dereferenceable in principle (i.e., resolvable via web request when their authority publishes them; e.g., DPV IRIs resolve at w3.org). Studio-internal IRIs MUST use the `wos-studio:` prefix. *(architectural commitment.)*
- **`SA-MUST-term-003`** — Deprecated CanonicalTerms MUST carry `supersededBy`; existing TerminologyMap entries pointing to deprecated terms MUST surface a tier-S2 ValidationFinding (`TERM-LINT-001`, "term-deprecated"). *(lint-pending.)*

### TerminologyMap resolution

- **`SA-MUST-term-010`** — Every Studio DataElement and entity name MAY carry an optional `canonicalTermRef` pointing to a CanonicalTerm IRI. PolicyObjects whose DataElement `canonicalTermRef` is `manual-pending` MUST surface a tier-S2 finding (`TERM-LINT-002`, "canonical-term-pending"). *(lint-pending.)*
- **`SA-MUST-term-011`** — DataElement `sensitivity` MUST be a DPV IRI for new PolicyObjects authored after this spec ratifies. Legacy values (`pii` | `phi` | `restricted`) MAY remain on existing PolicyObjects as aliases. The implementation MUST display both the legacy alias AND the canonical DPV IRI in reviewer UI. *(schema-pending; runtime-pending.)*
- **`SA-MUST-term-012`** — Cross-workspace DataElement identity MUST be determined by `canonicalTermRef` equivalence, not by `localName` string equality. Two workspaces' "household income" are the same term iff their canonicalTermRefs are equal (after IRI resolution). *(runtime-pending; cross-cutting federation §1.34.)*

### Plain-English projections

- **`SA-MUST-term-020`** — Every CONCEPT-MODEL entity MUST have a PlainEnglishProjection. Studio ships a baseline set; the set MUST be present at workspace creation. *(schema-pending.)*
- **`SA-MUST-term-021`** — Reviewer-facing UI surfaces (titles, finding messages, audit-log narratives) MUST render entities via `operatorLabel` by default. Power-user toggles MAY display canonical names. *(runtime-pending.)*
- **`SA-MUST-term-022`** — PlainEnglishProjection overrides via WorkspacePolicy MUST NOT alter canonical structure. The override is a display-time substitution. *(runtime-pending.)*

### Federation precursor

- **`SA-SHOULD-term-030`** — Workspaces SHOULD prefer existing CanonicalTerms over coining new ones. The implementation SHOULD surface a workflow-creation hint suggesting matching canonical terms when the local name is similar to an existing canonical (e.g., "Did you mean `usda-snap:HouseholdIncome`?").
- **`SA-MUST-term-031`** — Cross-workspace terminology agreement (federation, §1.34) MUST resolve through the canonical IRI; local name conflicts are local concerns, not canonical concerns. *(runtime-pending.)*

## Composition

### Attachment point

The TerminologyMap registry attaches at the Workspace level. The CanonicalTerm baseline ships with Studio. Workspace-specific CanonicalTerms are workspace-scoped.

### Precedence

When a DataElement carries both `canonicalTermRef` and `sensitivity` (DPV IRI):
- The `canonicalTermRef` is the term's identity (e.g., "this is the canonical 'household income' term").
- The `sensitivity` is the privacy classification (e.g., `dpv:FinancialPreference`).
- They are orthogonal; a DataElement can have one without the other.

When two workspaces' TerminologyMaps point local names to different CanonicalTerm IRIs, the workspaces are explicitly modeling different concepts. Federation (§1.34) provides the protocol for reconciling.

### Composition with parent WOS machinery

Studio's CanonicalTerms compose with parent **PROV-O / JSON-LD / DPV** vocabularies already cited in [`schemas/sidecars/wos-ontology-alignment.schema.json`](../../schemas/sidecars/wos-ontology-alignment.schema.json). Where the parent ontology-alignment sidecar declares semantic vocabularies, Studio's TerminologyMap aligns. Studio does NOT re-implement; Studio composes.

### Versioning / migration

- Adding new domains (`'snap'`, `'tanf'`, etc.) to `CanonicalTerm.domain`: non-breaking.
- Deprecating a CanonicalTerm: mark `status = deprecated`; existing references continue to resolve; new references rejected.
- Migrating from `pii | phi | restricted` to DPV IRIs: aliases coexist; UI displays both.

## Conformance

### Schema validation (Stage 3)

- CanonicalTerm shape, IRI uniqueness constraint.
- TerminologyMap entry shape per workspace.
- DPV IRI validation (subset of dereferenceable IRIs from W3C DPV registry).
- PlainEnglishProjection shape; one-per-entity coverage.

### Lint rules (Stage 4)

Tier-S2 (Policy readiness):
- `TERM-LINT-001` — term-deprecated (entry points to deprecated term).
- `TERM-LINT-002` — canonical-term-pending (DataElement awaits canonical mapping).
- `TERM-LINT-003` — sensitivity-not-DPV (DataElement uses legacy alias on a NEW PolicyObject; warn).
- `TERM-LINT-004` — no-PlainEnglishProjection (entity referenced in UI lacks projection; should never fire after Studio baseline ships).

### Runtime conformance fixtures (Stage 4–5)

- DataElement with `canonicalTermRef = usda-snap:HouseholdIncome` resolves correctly.
- DataElement with legacy `sensitivity: phi` still renders `dpv:HealthData` as canonical.
- Cross-workspace identity: workspace A's `householdIncome` and workspace B's `monthly_household_income` agree via canonical IRI.
- Reviewer UI renders `Policy Item` (operatorLabel) instead of `PolicyObject` (canonical name) by default.

### Current limitations

- The canonical baseline registry is bootstrapped Studio-side; community / agency-led canonical authoring is a future capability.
- DPV vocabulary is English-baseline; multi-lingual sensitivity is an open issue (DPV has multilingual extensions — Studio adopts as they mature).
- Auto-classification (AI suggesting canonical terms for free-text DataElements) is in `authoring-provenance.md` AI-extraction subtype, not specified here.

## WOS mappings

CanonicalTerm and TerminologyMap are **`authoringOnly`** — workspace-scoped registries; no `$wosWorkflow` body content.

The exception: DataElement `canonicalTermRef` and `sensitivity` (DPV IRI) project to `$wosWorkflow.caseFile.fields[*]` annotations:
- `caseFile.fields[*].x-canonical-term`: the CanonicalTerm IRI (when set).
- `caseFile.fields[*].x-dpv-sensitivity`: the DPV IRI (when set).

These are extension keys (`x-`) per ADR-0077. A **slight WOS-side extension** would promote them to first-class fields; queued in [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md) as an ExtensionRecord candidate.

The PlainEnglishProjection is workspace-internal; no WOS projection.

## Examples

### Example 1: DataElement with canonical term + DPV sensitivity

```text
DataElement {
  localName: "household_monthly_income",
  canonicalTermRef: "usda-snap:HouseholdIncome",
  sensitivity: "dpv:FinancialPreference",
  dataType: "money",
  unit: "USD/month",
  description: "The applicant household's gross monthly income from all sources, per 7 CFR §273.10(c)"
}
```

This DataElement: (a) has a stable canonical term identity (workspace can be federated with another workspace using same canonical), (b) carries DPV-classified sensitivity (workspace policy applies financial-data retention rules automatically), (c) preserves the local name for the workspace's caseworkers who say "income" not "household monthly income."

### Example 2: Plain-English projection for `PolicyObject`

```text
PlainEnglishProjection {
  conceptModelEntity: "PolicyObject",
  operatorLabel: "Policy Item",
  operationalDescription: "A specific thing the regulation says we must do, may do, or must not do — like requiring a 90-day appeal window or sending a Spanish-translated denial notice.",
  exampleNarrative: "A 'Policy Item' might be: '7 CFR §273.10(g)(1) — applicant must receive a written notice of denial within 30 days of the decision.' This becomes one Policy Item in your workspace, citing 7 CFR §273.10(g)(1) as the source.",
  audienceTier: "operator"
}
```

The reviewer UI shows "Policy Item" instead of "PolicyObject"; the audit log narrative says "Sarah created a new Policy Item for the 90-day appeal deadline" instead of "PolicyObject 'pol-deadline-appeal-90d' created by sarah.chen@dhs.state.gov."

### Example 3: Cross-workspace federation precursor

Workspace A (Texas DHS) has a DataElement `dispute_pending` with `canonicalTermRef: "wos-snap:DispositionPendingAppeal"`.

Workspace B (California DHS) has a DataElement `appeal_in_flight` with the same `canonicalTermRef: "wos-snap:DispositionPendingAppeal"`.

Both workspaces are explicitly modeling the same concept. When federation (§1.34) lands:

- The two workspaces can share Conflict resolutions about this term.
- The two workspaces can cite each other's PolicyObjects citing this term.
- The federation does NOT require local name agreement.

## Open issues

- **Community-led canonical vocabulary curation.** Bootstrapping a "USDA SNAP terminology" canonical registry requires either USDA participation or a conservatively-grown community baseline. Studio ships starter set; full curation is product-roadmap.
- **DPV multilingual extensions.** DPV has multilingual support but adoption varies; Studio's baseline is English; non-English projections are ad-hoc.
- **Term governance.** When two workspaces disagree on a canonical term's definition, who governs? Currently: workspace-scoped + reviewer-driven; federation will need a governance layer.
- **Slight WOS-side extension.** Promoting `x-canonical-term` and `x-dpv-sensitivity` from `x-` extensions to first-class `caseFile.fields[*].canonicalTermRef` / `sensitivity` would simplify; queued ExtensionRecord candidate.

## Cross-references

- Concept model: [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.30 TerminologyMap; §1.32 ProtectedCategory (uses DPV); §6.1 Schema composition strategy (this spec is a load-bearing input).
- Cross-cutting: [`policy-object-model.md`](policy-object-model.md) (DataElement.sensitivity uses DPV; canonicalTermRef on every DataElement), [`source-vault.md`](source-vault.md) (CanonicalSourceRef as a federation precursor).
- Composes with: [W3C DPV](https://www.w3.org/TR/dpv/) (canonical sensitivity vocabulary), parent [`schemas/sidecars/wos-ontology-alignment.schema.json`](../../schemas/sidecars/wos-ontology-alignment.schema.json) (parent semantic-vocab sidecar; Studio aligns).
- Repo conventions: [`../../CONVENTIONS.md`](../../CONVENTIONS.md).
