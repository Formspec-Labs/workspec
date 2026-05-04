# ADR-0086: Studio Knowledge Platform — reference architecture

**Status:** Proposed 2026-05-04
**Date:** 2026-05-04
**Deciders:** WOS Working Group (Studio sub-team)
**Author:** Studio authoring layer (Stage 7)
**Supersedes:** None
**Amends:** [`studio/VISION.md`](../../studio/VISION.md) §17 Stage 7 — replaces the Stage 7 placeholder with the landed reference-architecture spec.

**Related:**
- [`studio/specs/reference-architecture.md`](../../studio/specs/reference-architecture.md) (the normative spec this ADR ratifies)
- [`studio/specs/stage-8-vertical-slice.md`](../../studio/specs/stage-8-vertical-slice.md)
- ADRs 0087, 0088, 0089, 0090, 0091 (Stage 7 sibling decisions)

---

## 1. Context

`studio/VISION.md` §17 reserves Stage 7 for a reference-architecture
spec. Stages 1–6 produced 16 normative specs, 15 schemas, and 5
crates; Stage 7's job is to compose them into a system-level
architecture so Stage 8 can build a vertical slice with a clear
contract.

A pre-Stage-7 framing treated the work as a 15-week backend
build-out. That framing was rejected: it would have produced
production adapters before the architecture was nailed, and would
have framed Studio as "a WOS backend" — a positioning that
forecloses the multi-projection identity Studio actually carries.

The corrected framing: Studio is a **governed
knowledge-to-operational-artifact platform**. WOS workflow is the
first-class workflow projection; Formspec form is the first-class
form projection; decision artifacts, integration bindings,
scenario suites, approval packages, and signed export bundles are
co-equal projections in v1. Future projections (data contracts,
reports, public knowledge bases) attach the same way.

External tools (Cognee, dlt/Airbyte, OpenMetadata, OpenLineage,
ODCS, Great Expectations) attach behind named adapter seams — they
do not become normative dependencies.

## 2. Decision

### 2.1 Ratify the reference architecture

Adopt [`studio/specs/reference-architecture.md`](../../studio/specs/reference-architecture.md) as the
Stage 7 normative anchor. It defines:

- **Layer model** (Knowledge / Authoring / Design / Validation /
  Publication).
- **Component model** (composes the 16 existing specs by
  reference; introduces only the new abstractions justified
  inline).
- **Studio-side port catalog** (16 ports — distinct from
  `crates/wos-server`'s port set).
- **External-OSS-adapter seam catalog** (7 seams — see ADR 0091).
- **Projection-target model** (uniform `ProjectionTarget` /
  `ExportSink` port — see ADR 0089).
- **Canonical flows** (10 named flows).
- **Trust / governance invariants** (`SA-MUST-arch-060..084`).
- **Replay / rebuild contract** (`SA-MUST-arch-011` — see ADR 0087).

### 2.2 Stage 7 vs Stage 8 split

Stage 7 ships:
- `studio/specs/reference-architecture.md`.
- ADRs 0086–0091.
- Minimal contract code (trait stubs, boundary guard, type
  aliases, conformance skeletons).
- `studio/specs/stage-8-vertical-slice.md`.
- VISION + CLAUDE updates.

Stage 7 does **not** ship production adapters, full pipelines,
HTTP/WS/gRPC, worker queues, prompt registries, real-time
collaboration, KMS/WebAuthn impl. Those are Stage 8+.

### 2.3 Composition stance

The architecture spec **composes existing artifacts by
reference**. It does not restate behavioral semantics that the 16
existing specs already own; it points at them. New abstractions
land only when the architecture genuinely lacks the concept (see
"New abstractions (justified)" in the spec).

## 3. Rejected Alternatives

- **Stage 7 = full backend build-out.** Rejected because it
  conflates contract ratification with implementation; produces
  premature adapters; and risks WOS-only framing.
- **Stage 7 = single-page architecture sketch.** Rejected because
  the 16 existing specs are not self-composing — readers need a
  named composition surface, port catalog, and projection-target
  model to navigate the system.
- **Stage 7 ratifies Cognee (or any other knowledge-memory tool)
  as canonical SoT.** Rejected. Studio's typed model + Source
  Vault + Policy Knowledge Map remain canonical; external tools
  attach behind `KnowledgeMemoryAdapter` (see ADR 0091 + spec
  `SA-MUST-arch-032`).
- **Stage 7 introduces a new "FormIntent" object inside Studio.**
  Rejected. Formspec owns form definition; Studio supplies the
  reviewed knowledge model that the Formspec projection consumes
  (see ADR 0089).
- **Stage 7 mandates LinkML.** Rejected. JSON Schema + OpenAPI
  structured outputs first; LinkML deferred until drift across
  schema languages becomes operationally painful (see ADR 0088).

## 4. Consequences

### Positive

- Clear Stage 7 / Stage 8 boundary: ratification vs vertical slice.
- 16 existing specs gain an explicit composition surface.
- Multi-projection identity is encoded in the architecture, not
  bolted on.
- External-tool adapter seams keep Studio free to swap reference
  adapters.
- `crates/wos-server` boundary stays clean (separate port set;
  Studio publishes, server consumes).

### Negative

- Six new ADRs to maintain.
- One new spec to keep aligned with the 16 existing specs as they
  evolve.
- Open questions remain (DecisionModel unification,
  DataRequirement first-class, Authoring Ledger persistence
  shape, Form-projection adapter shape, port-crate location) —
  Stage 8 deliverables resolve these as they're touched.

### Neutral

- Tracking-ID prefix `SA-MUST-arch-*` joins the existing
  per-spec prefix family.

## 5. Conformance

- Spec-level: every `SA-MUST-arch-*` in
  [`studio/specs/reference-architecture.md`](../../studio/specs/reference-architecture.md).
- Adapter-side: Stage 8 deliverables (per
  [`studio/specs/stage-8-vertical-slice.md`](../../studio/specs/stage-8-vertical-slice.md))
  fire the boundary guard test and the replay test.
- ADR cross-refs: 0087 (persistence + replay), 0088 (AI extraction),
  0089 (projection target), 0090 (publish/export boundary), 0091
  (port/adapter architecture + external-tool seams).
