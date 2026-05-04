# ADR-0088: Studio AI extraction — cite-back, ConfidenceRecord, structured-output stance

**Status:** Proposed 2026-05-04
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
