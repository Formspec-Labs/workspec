# ADR-0085: Event-types taxonomy (PLN-0384) — Studio anchor

**Status:** Proposed 2026-05-03
**Date:** 2026-05-03
**Deciders:** WOS Working Group (parent-team ratification pending)
**Author:** Studio authoring layer (I-wave)
**Supersedes:** None
**Amends:** [`studio/specs/authoring-provenance.md`](../../studio/specs/authoring-provenance.md) §"Event-type taxonomy" — pins the Studio-side placeholder enum against parent PLN-0384 ratification.

**Related:**

- [`studio/DEFERRED.md`](../../studio/DEFERRED.md) `STUDIO-DEFER-004-COORDINATION` (`SA-MUST-prov-082` blocker)
- Parent `PLANNING.md::PLN-0384` (event-types taxonomy — not yet ratified as ADR)
- `SA-MUST-prov-082` (closed by this ADR's Studio-side anchor)
- ADR-0084 (sibling — identity attestation primitive)

---

## 1. Context

Parent `PLN-0384` proposes a unified event-types taxonomy across the WOS stack: a single closed enum naming every event kind that flows through `custodyHook` / `provenanceLayer` / external observation seams. Studio currently uses an open-ended `eventKind` string field on `AuthoringProvenanceRecord` (extensible per `wos.authoring.*` convention), but has no canonical taxonomy — it is waiting for parent ratification.

`SA-MUST-prov-082` asks Studio's provenance log to "compose with PLN-0384's canonical event-types taxonomy once ratified." Without a Studio-side anchor, `SA-MUST-prov-082` stays in `STUDIO-DEFER-004-COORDINATION`. The parent PLN has no ADR in `thoughts/adr/`; ratification timing is unknown.

This ADR pins the **Studio-side placeholder taxonomy** so:

1. Studio code consumes a stable enum.
2. Spec text and schema can name event kinds without ambiguity.
3. `SA-MUST-prov-082` closes against this Studio anchor, not against parent ratification.

---

## 2. Decision

### 2.1 Studio-side `eventKind` taxonomy

The closed enum that `AuthoringProvenanceRecord.eventKind` accepts (per the existing schema enum in `wos-studio-provenance.schema.json`):

| Cluster | Event kinds |
|---|---|
| **Extraction / promotion** | `extracted`, `normalized`, `promoted`, `demoted` |
| **Author actions** | `edited`, `merged`, `split`, `superseded` |
| **Review / approval** | `approved`, `rejected`, `manualOverrideRecorded`, `waived` |
| **Mapping** | `mapped`, `unmapped`, `bridge-override` |
| **Scenario** | `scenario-generated`, `scenario-passed`, `scenario-failed` |
| **Compile / publish** | `compiled`, `published`, `republished`, `deprecated` |

The enum is already in `wos-studio-provenance.schema.json`. The cluster grouping above is **advisory-only** in this revision — the schema does NOT enforce cluster membership. Cross-spec references to "the author-action cluster" or "the mapping cluster" are reader-facing prose anchors; if parent PLN-0384 ratifies a divergent grouping, only the prose changes (no normative schema impact). This downgrade addresses J9 (review feedback): an ADR pinning a normative grouping while flagging the demoted-vs-deprecated ambiguity as an open question would commit Studio to a bet without a schema gate to detect parent divergence. Marking the grouping advisory keeps the cross-spec naming useful while leaving room for parent reconciliation.

**`demoted` vs `deprecated` resolution.** `demoted` and `deprecated` are NOT synonyms in the Studio enum:

- `demoted` (Extraction/promotion cluster): an *author action* moving an approved object back to a workshop state (e.g., source supersession invalidates a citation; the dependent PolicyObject is demoted from `approved` to `draft`).
- `deprecated` (Compile/publish cluster): a *compile/publish action* marking a workflow as no longer recommended for new cases while preserving its historical execution semantics.

The two events occur at different lifecycle phases (authoring vs publication) and target different subjects (PolicyObject vs WorkflowIntent). Consolidating them would lose this distinction. If parent PLN-0384 ratifies a consolidated taxonomy, Studio amends this ADR to map both Studio events onto the parent's chosen value.

### 2.2 Studio-side `eventSubtype` conventions

`eventSubtype` is a free-text refinement of `eventKind`. Studio pins three conventions:

- `ai-assisted` — used on `extracted` / `normalized` to mark AI-authored events; requires `aiLineage` per `prov-071` (already enforced by F2 outer if/then).
- `human-only` — explicit non-AI authoring; the `aiLineage` requirement does not fire.
- `fast-track` — workspace-policy-driven shortcut (e.g., emergency action); requires `fastTrackJustificationRef`.

Other `eventSubtype` values are workspace-extensible.

### 2.3 Cross-stack alignment commitment

When parent PLN-0384 ratifies, the Studio-side enum becomes a strict subset of (or `$ref` into) the parent canonical taxonomy. The Studio enum above is designed to be a strict subset of parent PLN-0384's expected event vocabulary. Cluster grouping is informative-only (no schema enforcement); the `eventKind` enum itself is the load-bearing surface.

If PLN-0384 ratifies a divergent enum, Studio amends this ADR with a migration path; today's enum stays valid as a Studio-prefixed subset.

---

## 3. Rejected Alternatives

- **Wait for parent PLN-0384 ratification.** Rejected for the same reason as ADR-0084 (parent motion is unknown; Studio cannot block).
- **Open-ended event-kind field (no enum).** Rejected because schema enforcement is load-bearing for audit replay; an open-ended field defeats the audit guarantee.
- **Defer to `STUDIO-DEFER-007`.** Rejected because the enum is already in the schema; this ADR pins the cluster grouping that makes it cross-spec composable.

---

## 4. Consequences

**Positive.** `SA-MUST-prov-082` closes against the Studio-side anchor. Cross-spec references to "the author-action cluster" or "the mapping cluster" become unambiguous. Parent ratification becomes a one-token enum extension if needed.

**Negative.** If parent PLN-0384 ratifies a structurally-different taxonomy (e.g., uses `event_type` instead of `eventKind`, or pins a different cluster grouping), Studio carries debt. Mitigation: r2 documents migration path.

**Out of scope.**
- Cross-tenant event-stream federation (parent decision).
- CloudEvents extension attribute mapping (handled by parent profile spec, not this Studio ADR).
- Event ordering / replay semantics (parent decision; `runtime-observation-seam.md` references).

---

## 5. Open questions

1. **`eventKind = "demoted"` vs `"deprecated"`** — RESOLVED in §2.1: distinct events at distinct lifecycle phases (author-action vs compile/publish). Both names stay in the schema. If parent PLN-0384 consolidates, Studio will map both Studio events onto the parent's chosen value via an ADR amendment.
2. **`bridge-override` placement** — currently in the Mapping cluster but it's authored at compile time (`compiler-contract.md`). Open: should it move to Compile/publish?
3. **Hierarchical event names** — `wos.authoring.policy-object.approved` vs flat `approved`. Studio uses flat; parent may demand hierarchical for cross-stack disambiguation.

---

## 6. Implementation Notes

1. **Spec amendment** — `studio/specs/authoring-provenance.md` adds § "Event-type taxonomy" with the table above.
2. **Schema** — `wos-studio-provenance.schema.json` already enumerates the eventKind enum; this ADR pins the cluster commentary in spec text only.
3. **Migration** — when PLN-0384 ratifies, revise this ADR with migration narrative; if the parent enum diverges, file a J-wave fixup.
