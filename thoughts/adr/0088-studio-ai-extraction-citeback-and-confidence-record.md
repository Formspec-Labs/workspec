# ADR-0088: Studio AI extraction — cite-back, ConfidenceRecord, structured-output stance

**Status:** Proposed 2026-05-04 · Amended 2026-05-04 (validation)
**Date:** 2026-05-04
**Deciders:** WOS Working Group (Studio sub-team)
**Author:** Studio authoring layer (Stage 7)
**Supersedes:** None
**Amends:** [`studio/specs/authoring-provenance.md`](../../studio/specs/authoring-provenance.md) §"AI extraction subtype" — extends the existing AI-lineage record with `ConfidenceRecord`.

**Related:**
- ADR 0086 (parent — reference architecture)
- ADR 0087 (sibling — persistence; defines AI-output recording for replay)
- ADR 0091 (sibling — adapter seams; defines `KnowledgeMemoryAdapter`)
- Parent [`specs/ai/ai-integration.md`](../../specs/ai/ai-integration.md) §3.4 (AI extraction provenance)

---

## Amendment 2026-05-04 — verifier identity, policy stance, ConfidenceRecord file

Three §2 decisions tightened after parent-spec validation
(`wos-expert` 2026-05-04). All amendments make Studio's stance
**stricter and more enforceable** than the original Stage 7
draft.

### §2.1 — ConfidenceRecord serialization location

**Original (now superseded):** "serialized into a new
`wos-studio-confidence.schema.json` `$def` (Stage 8 deliverable)" —
the `$def` framing implied a sub-schema inside another file.

**Amended:** ConfidenceRecord lands in its **own schema file**,
`studio/schemas/wos-studio-confidence.schema.json`. Other
schemas (`wos-studio-provenance.schema.json` AI subtype;
review-queue UI; future scenario-confidence; future
mapping-confidence) `$ref` into it.

**Rationale.** ConfidenceRecord is multi-consumer (AI extraction
today; review UIs and scenario reports tomorrow). A `$def`
inside provenance schema couples its evolution to AILineage's
release cadence and produces confusing diffs ("provenance schema
changed for an AI-only reason"). Own file = clean change set.
Studio schema count goes from 15 to 16 — below the operational-
pain threshold.

### §2.4 — Verifier model identity (was open question)

**Original (now superseded):** "Stage 8 default: same provider,
different temperature/seed; Stage 9+ revisits for independence."

**Amended:** **Always-different model family.** Stage 8 ships two
`ModelAdapter` instances configured as extractor + verifier with
different model families (e.g., Claude Opus extracts, Claude
Sonnet or GPT-5 verifies). Provider may be the same or different;
the load-bearing axis is *model family*, not *provider*.

**Rationale.** Same model family makes the same systematic
mistakes; same-family verification rubber-stamps the extractor.
Different temperature/seed catches transients but misses
provider-correlated failure modes. Independence is the audit
story; same-provider-different-model-family meets that story at
half the integration cost of always-different-provider.

**Enforcement.** WOS spec is silent on this axis (validated
2026-05-04 against `specs/ai/ai-integration.md`,
`specs/ai/agent-config.md`, `specs/ai/drift-monitor.md` — no
hits for verifier / cross-model). The rule therefore lives at
the **Studio tier**:
- Reference-architecture spec encodes it as `SA-MUST-arch-085`
  "Verifier model independence."
- New Studio lint rule `RA-LINT-085` (S2 tier) MUST fire on
  workspace AI configuration if extractor and verifier share
  a model family.

Without lint enforcement, the rule erodes silently across
refactors. With it, the rule is durable.

### §2.6 — Source / model-use policy enforcement (was open question)

**Original (now superseded):** "Stage 8 default: imperative checks
(small, audit-able); Stage 9+ revisits if policy proliferation
justifies a declarative language."

**Amended:** **Typed declarative Rust structs at the Studio API
boundary.** Define `SourceUsePolicy` and `ModelUsePolicy` as
typed enums + structs with compile-time exhaustive matching.
Per-workspace policies serialize as JSON; load at workspace boot;
cache in memory; sign in the ApprovalPackage.

**Rationale.** Imperative checks scattered across handlers are
unmaintainable past ~10 rules. OPA/Cedar industrial-grade policy
infrastructure is overkill for "which AI can read which sources."
Typed structs split the difference: compile-time exhaustiveness,
zero runtime infra, audit-friendly.

Source/model-use policy is **authoring-tier**, not workflow-
runtime; it does NOT violate the parent "FEL is the only
expression language" commitment (validated 2026-05-04 — FEL
binds workflow guards, not Studio admission control). It does
NOT displace `PolicyEngineBinding` (which composes at workflow
runtime).

**Promotion path.** `PolicyEngineBinding` (OPA/Cedar/XACML) is
admitted at scale (>50 rules per workspace, or
attribute-based-access requirements). Until then, typed structs.

Reference-architecture spec encodes this as `SA-MUST-arch-086`.

---

## 1. Context

[`studio/specs/reference-architecture.md`](../../studio/specs/reference-architecture.md)
§"AI invariants" defines four obligations on the AI extraction
pipeline:

- Schema-guided structured outputs.
- Validator loop on every output.
- ConfidenceRecord that combines multiple signals.
- AI lineage captured per call.

The existing [`studio/specs/authoring-provenance.md`](../../studio/specs/authoring-provenance.md)
already pins the AI-extraction subtype with model lineage
(`modelId`, `modelVersion`, `promptTemplateRef`, `temperature`,
`seed`, `humanApprover`). This ADR extends that contract with:

1. The **ConfidenceRecord shape** (multi-signal; not
   model-self-reported).
2. The **cite-back enforcement** rule.
3. The **structured-output stance** (JSON Schema first; LinkML
   deferred).
4. The **verifier-pass requirement** (separate model invocation;
   independence preferred).
5. The **recorded-AI-output replay primitive** (referenced from
   ADR 0087).

A common alternative — "the model's self-reported confidence
gates approval" — is **rejected** because LLM self-confidence is
not calibrated and is degenerate as a governance signal. The
ConfidenceRecord combines six signals; no single signal gates
approval.

## 2. Decision

### 2.1 ConfidenceRecord shape

Each AI-extracted candidate carries a `ConfidenceRecord` of:

```text
schemaValidationResult     enum { passed | failed | recovered }
citationSupportScore       float [0.0, 1.0]   // citation-to-text support
retrievalScore             float [0.0, 1.0]   // top-k retrieval relevance
verifierResult             enum { agreed | disagreed | abstained | error }
riskTier                   enum { low | medium | high | block }  // from RiskTier registry
humanReviewState           enum { pending | approved | rejected | revisedThenApproved }
```

Stored as part of the `AuthoringProvenanceRecord`'s AI subtype
extension; serialized into a new
`wos-studio-confidence.schema.json` `$def` (Stage 8 deliverable).

`SA-MUST-arch-082` is satisfied iff:
- ALL six fields are present on every AI-extracted candidate,
- `humanReviewState ∈ { approved, revisedThenApproved }` before
  the candidate becomes a durable PolicyObject (no exceptions for
  high `citationSupportScore` alone),
- `riskTier == block` MUST NOT be approved by any single reviewer.

### 2.2 Cite-back enforcement

`SA-MUST-arch-070` — every AI-emitted artifact carries cite-back
to a source span. Stage 7 commits to the following enforcement:

- The structured-output schema MUST require a `citations[]` field
  with at least one `SourceCitation` reference per top-level
  candidate.
- The validator pass MUST verify that each cited
  `(SourceVersionId, sectionPath, byteRange)` resolves to a real
  Source Vault section.
- The `citationSupportScore` MUST be computed by the verifier
  (LLM-judge or embedding similarity, per Stage 8 implementation
  choice) and recorded.

Citation reuse from the AI's training data ("cite-from-memory")
is rejected — citations MUST resolve to Source Vault content.

### 2.3 Structured-output stance: JSON Schema first

Studio's structured-output stance is **JSON Schema + OpenAPI
first**. This matches:
- The existing 15 [`studio/schemas/`](../../studio/schemas/) JSON
  Schemas (Stage 3).
- The WOS workflow schema at
  [`schemas/wos-workflow.schema.json`](../../schemas/wos-workflow.schema.json).
- Formspec's form schemas.

LinkML, RDF/Turtle, and SHACL-as-output-schema are **deferred** —
Studio uses SHACL only as a sidecar to JSON-LD ingest (per
[`schemas/sidecars/wos-ontology-alignment.schema.json`](../../schemas/sidecars/wos-ontology-alignment.schema.json)),
not as an extraction-output schema.

Re-evaluation trigger: drift across JSON Schema, JSON-LD, SHACL,
docs, and generated Rust types becomes operationally painful
(measured: more than two manual reconciliations per quarter).

### 2.4 Verifier-pass requirement

Each AI extraction MUST run through a **verifier pass** distinct
from the primary extraction. The verifier:
- Re-checks structured-output validity.
- Re-checks citation-text support.
- Computes `citationSupportScore`.
- Records `verifierResult`.

**Open: verifier model identity.** Same provider as primary
extractor or always different? Stage 8 default: same provider,
different temperature/seed; Stage 9+ revisits for independence.

### 2.5 Recorded AI output (replay primitive)

Per ADR 0087, replay requires recorded AI outputs. Each
`ModelAdapter` invocation records:
- Input prompt (rendered) — by hash.
- Retrieval set — by hash of ordered chunk IDs.
- Output bytes — verbatim, in `studio_canonical.ai_outputs` table.
- Versioned metadata — model identity + version, prompt template
  + version, parser version, projection version.

`AuthoringProvenanceRecord` references the recorded output by
`(invocation_id, output_hash)`. The replay test (ADR 0087 §2.5)
re-uses recorded outputs verbatim — it does NOT re-invoke the
model.

### 2.6 Source / model-use policy enforcement

Per `SA-MUST-arch-065`, Studio enforces source/model-use policy
at the API boundary. **Open: declarative policy language vs
imperative checks.** Stage 8 default: imperative checks (small,
audit-able); Stage 9+ revisits if policy proliferation justifies a
declarative language.

## 3. Rejected Alternatives

- **Model self-reported confidence gates approval.** Rejected;
  LLM self-confidence is not calibrated.
- **Single-signal confidence (only `citationSupportScore`).**
  Rejected; misses risk tier, schema validity, human review state.
- **No verifier pass.** Rejected; single-pass extraction has no
  independent check.
- **LinkML now.** Rejected; deferred until JSON Schema drift
  becomes operationally painful.
- **Optional citations.** Rejected; cite-back is load-bearing per
  `SA-MUST-arch-070`.
- **Replay by re-invoking the model.** Rejected; non-determinism
  defeats replay. Use recorded outputs.
- **Verifier in same call as extractor (chain-of-thought
  self-check).** Rejected; the verifier MUST be a separate
  invocation so its result is independently auditable.

## 4. Consequences

### Positive

- ConfidenceRecord makes governance signals explicit and auditable.
- Cite-back enforcement is testable.
- Replay invariant holds without re-invoking models.
- JSON-Schema-first matches the existing schema-first ecosystem;
  no toolchain churn.

### Negative

- Two model invocations per extraction (extractor + verifier) —
  cost roughly 2×.
- Recorded AI outputs grow the `studio_canonical` schema.
- Open: verifier-model-identity question deferred to Stage 8 wire-up.

### Neutral

- ConfidenceRecord is internal to Studio's typed model; not
  surfaced in the published WOS workflow artifact (it is an
  authoring concern, not a runtime concern).

## 5. Conformance

- `SA-MUST-arch-070..084` in
  [`studio/specs/reference-architecture.md`](../../studio/specs/reference-architecture.md).
- Stage 8 deliverables 6, 7 in
  [`studio/specs/stage-8-vertical-slice.md`](../../studio/specs/stage-8-vertical-slice.md).
- Schema: new `wos-studio-confidence.schema.json` `$def` extending
  `wos-studio-provenance.schema.json` AI subtype (Stage 8 deliverable).
