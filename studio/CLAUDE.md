# CLAUDE.md — WOS Studio (Authoring)

The Authoring layer for WOS — a non-technical, source-backed workflow
intelligence platform. Lives in this `studio/` directory as a self-contained
tree designed for future extraction to its own repo. Parent
[`../CLAUDE.md`](../CLAUDE.md) carries WOS-spec conventions; this file
carries only Studio-specific deltas.

## Read first

| For | Read |
|---|---|
| Product vision (PRD) | [`VISION.md`](VISION.md) |
| Entity catalog + lifecycles + composition strategy | [`CONCEPT-MODEL.md`](CONCEPT-MODEL.md) |
| Spec index (16 internal specs across 7 families) | [`specs/README.md`](specs/README.md) |
| Vertical slice (SNAP redetermination from sources) | [`examples/snap-redetermination-from-sources/README.md`](examples/snap-redetermination-from-sources/README.md) |
| Stage-3 schemas (15 files) | [`schemas/`](schemas/) |
| Studio-tier Python tests | [`tests/schemas/`](tests/schemas/) |

## Identity

WOS Studio (Authoring) transforms institutional policy and operational
knowledge — federal regulations, state manuals, agency memos, accumulated
experience — into validated, explainable, reviewable, WOS-aligned workflows.
Most users work with sources, requirements, notices, deadlines, appeals,
decisions, evidence, roles, assumptions, and scenarios — not WOS JSON. The
compiler emits the WOS JSON; lint + scenarios + readiness gates ensure the
emitted workflow holds.

**Sibling product:** [`../case-portal/`](../case-portal/) is the runtime
case-management UI (renamed 2026-05-02 from `studio/` to free the path
for this Authoring tree). The two products share a name family but have
no runtime dependency on each other.

## Boundary discipline (load-bearing)

The owner anticipates moving Studio out to its own repo. The current layout
is engineered so that future move is a `git filter-repo --subdirectory
studio/` plus a submodule swap — no source changes. The boundary is
enforced structurally:

1. **One-way dependencies.** `studio/` consumes `wos-core`, `wos-lint`,
   `wos-runtime` only through their published `studio_api` modules. The
   parent never imports from `studio/`. The boundary is enforced by
   one workspace-wide guard test at
   `studio/crates/wos-studio-types/tests/api_surface.rs` — it walks
   every `.rs` file under `studio/crates/` (including each crate's
   `tests/` directory) and rejects `use` / `pub use` /
   `pub(...) use` of `wos_(core|lint|runtime)::` outside `studio_api`.
   Three known bypass surfaces (re-export laundering inside
   `studio_api`, `build.rs` macro-generated imports, type-alias
   inheritance via parent-type return values) are documented at the
   top of the guard test file.
2. **Separate Cargo workspace.** `studio/Cargo.toml` is a sibling
   workspace to the parent. Future repo extraction swaps
   `path = "../crates/wos-core"` to a submodule path with no semantic
   change. Wave 0.2.
3. **Separate test runner.** `studio/tests/schemas/` runs under a
   Studio-local `conftest.py`; the parent ratchet stops at the WOS-spec
   boundary. Wave 0.1 + Wave 0.4.
4. **Separate build target.** `make case-portal-*` (parent) and the
   Studio-tier `make` targets (Wave 0.6) are independent. A Studio-only
   PR doesn't trigger parent rebuild and vice-versa.

**Owner directive 2026-05-02:** the wos-spec-level schema-doc ratchet
DOES NOT apply at the Studio tier. Studio quality lives under
Studio-team discipline; whether Studio adopts its own ratchet is a
Studio-team decision.

## Stage roadmap

- **Stage 0** — Vision (DONE) — [`VISION.md`](VISION.md)
- **Stage 1–2** — Concept Model + Specs (DONE) — [`CONCEPT-MODEL.md`](CONCEPT-MODEL.md), [`specs/`](specs/)
- **Stage 3** — Schemas (DONE; 15 schemas including the F1 kernel
  extensions ApplicabilityScope / EffectivePeriod / WosVersionPin
  $defs, expanded DeonticConstraint, and DPV/canonicalTermRef on
  FieldDeclaration) — [`schemas/`](schemas/), vertical-slice examples in [`examples/`](examples/)
- **Stage 4 — Readiness lint engine** (DONE) — 70 rules across S1–S6
  tiers; catalog in [`specs/readiness-validation.md`](specs/readiness-validation.md);
  impl in [`crates/wos-studio-lint/`](crates/wos-studio-lint/) plus the
  graduation matrix at [`STUDIO-LINT-MATRIX.md`](STUDIO-LINT-MATRIX.md).
  S6 publication-blocker rules carry `LintSeverity::Block` per F0
  (2026-05-02).
- **Stage 5 — Compiler** (DONE) — Studio→WOS compiler at
  [`crates/wos-studio-compiler/`](crates/wos-studio-compiler/); 9 phases
  (phase 9 added 2026-05-02 for the SA-MUST-cmp-060..063 workspace
  export bundle), three external gates (`schema-pass`, `lint-pass`,
  `conformance-pass`) per
  [`specs/compiler-contract.md`](specs/compiler-contract.md).
  Phase 4 emits real lifecycle transitions (F2, 2026-05-02);
  phase 7 lint-pass runs T1 + T2 cross-document rules (F4.2);
  phase 9 emits a deterministic, self-contained export bundle
  carrying sources + PolicyObjects + mappings + scenarios +
  provenance log + custody receipts. Compiler lifecycle events
  emit per SA-MUST-cmp-070..073 to a JSON-Lines stream.
- **Stage 6 — Scenario simulator** (DONE) — runner +
  conformance-trace projection at
  [`crates/wos-studio-scenario/`](crates/wos-studio-scenario/);
  16 canonical scenario types per
  [`specs/scenario-authoring.md`](specs/scenario-authoring.md).
- **Stage 7 — Reference architecture spec** (DONE 2026-05-04) —
  [`specs/reference-architecture.md`](specs/reference-architecture.md)
  ratifies the layer model, component model, Studio-side port
  catalog, external-OSS-adapter seams, projection-target model
  (`ProjectionTarget` / `ExportSink` — WOS workflow + Formspec form
  co-equal first-class), canonical flows, and trust/governance
  invariants. Six sibling ADRs decompose the load-bearing decisions:
  [`thoughts/adr/0086`](../thoughts/adr/0086-studio-knowledge-platform-reference-architecture.md)
  (parent), `0087` (persistence + replay), `0088` (AI extraction +
  ConfidenceRecord), `0089` (projection target), `0090`
  (publish/export boundary), `0091` (port/adapter architecture +
  external-tool seams). Stage 7 contract code (port + adapter trait
  stubs, ConfidenceRecord, AILineageExt, ProjectionRef aliases) lives
  in [`crates/wos-studio-types/src/arch.rs`](crates/wos-studio-types/src/arch.rs);
  shape-only conformance skeleton at
  [`crates/wos-studio-types/tests/arch_contract.rs`](crates/wos-studio-types/tests/arch_contract.rs).
- **Stage 8 — Production vertical slice** — pending; planned at
  [`specs/stage-8-vertical-slice.md`](specs/stage-8-vertical-slice.md).
  Width-one path through every layer: source ingest → schema-guided
  AI extraction → human review → PolicyKnowledgeMap traceability →
  WOS workflow + Formspec form projections → scenario over both →
  signed ApprovalPackage + ExportBundle → filesystem ExportSink.
  Reuses the existing
  [`examples/snap-redetermination-from-sources/`](examples/snap-redetermination-from-sources/)
  fixture as the corpus and reproducibility target.

**Deferred work tracking:** [`DEFERRED.md`](DEFERRED.md) is the
single source of truth (one entry per ID, ratchets enforce baselines).
Open at HEAD:
STUDIO-DEFER-004-{LINT 31, SCHEMA 7} residual at HEAD (lint
markers needing spec-side decisions or fixture migration; schema
markers needing fixture vetting). RUNTIME / FIXTURE / COORDINATION
sub-IDs all Closed in I-wave 2026-05-03 (RUNTIME via reclassify
to DEFER-007; FIXTURE via cmp-051 sharpening; COORDINATION via
ADR-0084 + ADR-0085).
STUDIO-DEFER-006 (kernel-spec amendment for legal-hold clock-resume;
forward-looking, non-blocking).
STUDIO-DEFER-007 (substrate-pending invariants — 191 markers
reclassified from runtime + 1 from fixture; tracks Stage-7/8
substrate-dependency taxonomy: write-barriers, change-detection
engines, simulator emission, runtime-observation adapter, Trellis
identity seam, kernel clock-resume, etc.).
Closed in D-wave (2026-05-03):
- STUDIO-DEFER-001 (`.raw` access sweep, ratchet at residual 8 in
  `crates/wos-studio-lint/tests/raw_access_ratchet.rs`).
- STUDIO-DEFER-002 (lint-engine fixture suite externalization;
  37 of 43 fixtures externalized to
  `crates/wos-studio-lint/fixtures/{s1..s6,cross_cutting}/`;
  inventory ratchet at
  `crates/wos-studio-lint/tests/fixture_inventory_ratchet.rs`;
  6 date-arithmetic / sentinel tests intentionally inline).
- STUDIO-DEFER-003 Tranches A + B + C (boon format-assertion +
  lint K-016 for `initialState ∈ states` + lint-covered actor-id
  uniqueness via K-009 with no schema reshape).

Closed in E8 (2026-05-03):
- STUDIO-DEFER-005 (typed `RetentionPolicy` promotion; spec +
  schema $def + Rust struct at `crates/wos-studio-model/src/policy.rs`
  + WF-LINT-006 shape-aware migration + new
  `SA-WARN-pom-MIGRATE-RETENTION` advisory; ADR-0083 r2 Accepted
  2026-05-03).

F4.1 (Draft 2020-12 schema-pass via `boon`) and the F5.4 graduation
flip have closed; F4.3 (real conformance-pass replay) remains open
under the Stage roadmap above.

## Key rules

- **`wos-spec` is the canonical substrate.** Every approved Studio object
  either projects to a WOS construct, is explicitly authoring-only,
  identifies a controlled extension need, or is unmapped-but-approved with
  a documented rationale (CONCEPT-MODEL §5; `specs/policy-object-model.md`
  PRD §6).
- **Source-backed by construction.** Every PolicyObject carries
  `citations[]` pointing to a SourceCitation that resolves to a real
  SourceSection in a SourceVersion. Lint rule SV-LINT-001 enforces this
  at readiness time.
- **No Studio-side lint engine reaches into WOS-spec internals.** Studio
  rules consume `wos_lint::studio_api::LintDiagnostic` (and other
  `studio_api` types); they do NOT import from `wos_lint::rules::*`.
  The forbidden-import guard test enforces this structurally.
- **Determinism is load-bearing in the compiler.** Identical Studio input
  + identical compile date → bit-identical `wos-workflow.json` output.
  IndexMap / BTreeMap throughout; never HashMap with iteration leakage.
- **Compiler refuses to emit if any of the three external gates fail:**
  schema-pass (compiled output validates against
  `wos-workflow.schema.json`), lint-pass (`wos_lint::lint_workflow`
  returns zero diagnostics), conformance-pass (compiled output replays
  cleanly through `wos-runtime` against ≥1 conformance fixture).

## Build & test

```bash
# Studio-tier Python schema regression suite (39 tests)
python3 -m pytest studio/tests/schemas -q

# Studio-tier Rust workspace (once Wave 0.2 lands)
(cd studio && cargo check --workspace)
(cd studio && cargo test --workspace)

# Compile the SNAP slice (once Wave 2 lands)
(cd studio && cargo run -p wos-studio-compiler -- examples/snap-redetermination-from-sources)

# Run scenarios against the slice (once Wave 3 lands)
(cd studio && cargo run -p wos-studio-scenario -- examples/snap-redetermination-from-sources)
```

The parent repo's build (`make check-core` / `cargo check --workspace` from
the repo root) does NOT include Studio crates. Studio runs its own checks.

## Spec authoring contract

Same three-section rubric as WOS-spec: Normative Contract / Composition /
Conformance (per parent [`../CONVENTIONS.md`](../CONVENTIONS.md)). Every
spec under `specs/` follows it.

## Submodule awareness

Lives inside the WOS-spec repo as `studio/`. Until extraction, commits land
on the same branch as parent WOS work; bump per-feature, not per-stage.
Once extracted: this directory becomes a standalone repo; the parent will
consume it via submodule (path `wos-spec/studio/`). Plan for symmetry —
new code lands as if the boundary already exists.

AI-authored commits end with:

```
Co-Authored-By: Claude <noreply@anthropic.com>
```
