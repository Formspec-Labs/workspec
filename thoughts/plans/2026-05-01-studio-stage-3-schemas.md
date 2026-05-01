# Stage 3 — Studio JSON Schemas

## Context

Stage 2 (specs) is complete on `claude/wos-studio-setup-zFFDC` at HEAD `ded8f31`: 16 specs across 7 families, 34 concept-model entities, vertical-slice example, plus three follow-on commits (substrate-choice clarification, decisionTable kernel extension, bridge.kernelKind inference). The implementation roadmap calls for **Stage 3 = the JSON schemas that validate Studio author-time artifacts**.

CM §6.1 ("Schema composition strategy") prescribes ~13 schemas via composition with parent WOS schemas, reducing from a naive ~33 entity-per-schema explosion. The pattern: each Studio object is `{studioMetadataEnvelope + body}` where the body either co-locates a Studio-defined shape OR composes a parent WOS shape via $ref.

Plan-agent design review surfaced **three architectural findings** that require small concept-model corrections **before** schema authoring begins:

1. **The "$ref to WOS $def" claim in CM §6.1 is mostly aspirational.** Of the 8 WOS-projecting PolicyObject kinds, only 2 have clean $defs in `wos-workflow.schema.json` (AppealRight → AppealMechanism, DecisionRule → DecisionTable). The other 6 (NoticeRequirement, ExplanationRequirement, Deadline, ActorMapping, EvidenceRequirement, Outcome) have no clean $def — their shape is either flat properties inside larger blocks, runtime-reference shapes at the wrong abstraction, or simply not promoted to $defs. Forcing $ref into shapes that don't exist would produce broken schemas. **Honest fix:** the studio body inlines the shape; envelope carries `wosTarget` (a JSONPath naming the projection) and `wosShapeRef` (citation of the closest WOS $def or block path) for traceability; "WOS scales down dynamically" stops claiming structural inheritance and instead claims projection-target-discipline.

2. **`wos-tooling.schema.json` has no `scenarios` $def.** It only carries `lintDiagnostic`, `conformanceTrace`, `synthTrace`, `mcpToolCatalog`, `extensionRegistry`. The CM §6.1 row promising scenarios `$ref` into wos-tooling is a false reference. **Honest fix:** Studio Scenario is standalone; document the downstream `conformanceTrace.fixtureRef` correlation as the runtime tie-back.

3. **Bridge kinds (6) belong in workflow-intent, not policy-object.** They're products of bridge-inference compilation, not author-written PolicyObjects. Co-locating them with WorkflowIntent makes the kernel-kind / bridge-kind referential closure local to one file.

These three corrections land first as a small CM §6.1 patch (~30 lines), then schema authoring proceeds in four waves.

## Recommended approach

### Pre-stage correction (1 commit, ~30 lines)

Edit `studio-authoring/CONCEPT-MODEL.md` §6.1:

- Composition table: replace "envelope + $ref to wos-workflow.schema.json $defs/<TargetType>" → "envelope (with `wosTarget` JSONPath + `wosShapeRef` citation) + body co-located in studio schema". Keep the table's per-entity bucketing.
- Move 6 bridge kinds (WorkflowStepMapping, LifecycleTagMapping, TransitionMapping, TimerMapping, TaskMapping, CaseFileMapping) row from `wos-studio-policy-object.schema.json` bucket → `wos-studio-workflow-intent.schema.json` bucket.
- Scenario row: drop the "$ref into wos-tooling scenarios[*]" claim; replace with "standalone schema; conformanceTrace.fixtureRef cites scenario.id at runtime correlation time".
- Add a paragraph above the table acknowledging the design clarification (preserves CM history; doesn't pretend the previous claim was always right).

### Schema set (14 files in `/home/user/wos-spec/schemas/studio/`)

| # | File | Bucket | Marker | Notes |
|---|---|---|---|---|
| 1 | `wos-studio-common.schema.json` | $defs library | (none) | StudioMetadataEnvelope, OriginClass, MappingState, LifecycleState enums, AuthorityGrantApplied. Cross-referenced by every other schema. No marker — registered by `$id`. |
| 2 | `wos-studio-effectiveness.schema.json` | foundation | `$wosStudioEffectiveness` | jurisdictions[], temporalScope, appellateState, supersession refs |
| 3 | `wos-studio-identity-subject.schema.json` | foundation | `$wosStudioIdentitySubject` | composes parent PLN-0381 (placeholder until ratified) |
| 4 | `wos-studio-terminology-map.schema.json` | foundation | `$wosStudioTerminologyMap` | CanonicalTerm + workspace TerminologyMap; DPV IRI-as-string-with-pattern |
| 5 | `wos-studio-migration-path.schema.json` | foundation | `$wosStudioMigrationPath` | wosVersionPin, deprecation tracking |
| 6 | `wos-studio-source.schema.json` | foundation | `$wosStudioSource` | SourceDocument, SourceVersion, SourceSection, SourceCitation, ExtractedClaim, CanonicalSourceRef; JSON-LD @context accepted |
| 7 | `wos-studio-policy-object.schema.json` | core | `$wosStudioPolicyObject` | Polymorphic (oneOf by `kind`); 22 PolicyObject kinds (8 WOS-projecting + 14 studio-only); deontic kinds carry `defeasible: boolean` for LegalRuleML; bridges moved out per finding #3 |
| 8 | `wos-studio-binding.schema.json` | composer | `$wosStudioBinding` | ServiceBinding, EventBinding, PolicyEngineBinding, DMNImport (carries dmnXml string + feelToFelMapping issues) |
| 9 | `wos-studio-scenario.schema.json` | composer | `$wosStudioScenario` | Standalone (per finding #2); 12 scenarioType values; row-coverage for DecisionTable scenarios |
| 10 | `wos-studio-workflow-intent.schema.json` | orchestrator | `$wosStudioWorkflowIntent` | 16 element kinds + 6 bridge kinds (folded in per finding #3); bridge.kernelKind inference rules |
| 11 | `wos-studio-provenance.schema.json` | orchestrator | `$wosStudioProvenance` | AuthoringProvenanceRecord with hashChain (custodyHook 4-field); AI-extraction subtype with aiLineage |
| 12 | `wos-studio-workspace.schema.json` | orchestrator | `$wosStudioWorkspace` | Workspace, ReviewerRole, WorkspacePolicy, AuthorityGrant, ComplianceAttestation |
| 13 | `wos-studio-approval.schema.json` | orchestrator | `$wosStudioApproval` | ApprovalDecision, ApprovalPackage, ChangeImpactReport |
| 14 | `wos-studio-readiness.schema.json` | orchestrator | `$wosStudioReadiness` | ValidationFinding + readiness rule registry (~80 rules across S1-S6 tiers) |

(14 files total counting `wos-studio-common`. Plan-agent confirmed common as separate is correct dependency direction — common is leaf-of-imports, not folded into the largest schema.)

### Wave plan (4 commits)

**Rationale:** ~50+ `x-lm.critical=true` nodes across 13 schemas means `schema_doc_zero_regression` ratchet enforcement is high-volume. Single-commit failure mode is "spiral debugging which of 50 nodes is missing examples." Waves give 4 ratchet checkpoints with clean rollback.

- **Wave 1 (foundation, no inter-schema deps except common):** common + effectiveness + identity-subject + terminology-map + migration-path + source. 6 files. Lands the studio/ directory + conftest.py registration plumbing.
- **Wave 2 (the polymorphic core):** policy-object. 1 file but the largest (~22 kinds × ~50-150 lines = 1500-2500 lines). Depends on common + effectiveness + terminology-map.
- **Wave 3 (composers):** binding + scenario. 2 files. Depends on policy-object.
- **Wave 4 (orchestrators):** workflow-intent + provenance + workspace + approval + readiness. 5 files. Depends on everything prior; workflow-intent absorbs the 6 bridge kinds from finding #3.

### Conventions (compose with parent infrastructure)

- JSON Schema Draft 2020-12.
- `$id` pattern: `https://wos-spec.org/schemas/studio-<concept>/1.0` (short semantic URL; test harness aliases both filename + short forms in `conftest.py:71-85`).
- Document marker: `$wosStudio<Name>` const `"1.0"`, marked `x-lm.critical=true` with description + ≥1 examples (parent CLAUDE.md ratchet).
- `additionalProperties: false` at top level; `patternProperties: {"^x-": {}}` for vendor extensions.
- $ref to parent: `https://wos-spec.org/schemas/wos-workflow.schema.json#/$defs/<TargetType>` for the 2 kinds where it actually works (AppealRight, DecisionRule); inline elsewhere with `wosTarget` JSONPath citation.
- **`x-lm.critical=true` discipline:** apply sparingly to 5-10 load-bearing fields per schema (`mappingState`, `kind` discriminator, `kernelKind`, `originClass`, deontic kind, hash-chain links) — not every field. The ratchet is to keep that small set healthy, not to enforce description+examples on every property.

### Fixture strategy

- **Positive fixtures:** point at the existing `studio-authoring/examples/snap-redetermination-from-sources/**/*.json` artifacts via test parametrize. Don't copy — duplication immediately drifts. Add a `tests/schemas/test_studio_examples.py` (or extend `test_fixture_validity.py`) that walks the examples directory, classifies each by its embedded marker, and validates against the registered schema.
- **Negative fixtures:** ~3 per schema that has load-bearing rules. Author under `tests/schemas/fixtures/studio/`. Rules to cover negatively: mapping-state precedence violation, missing kernelKind on `step`/`system-check` ambiguous kinds, decisionTable hit-policy violation, AuthorityGrant subject mismatch, body-without-envelope, envelope-without-body, missing originClass on approved object.
- **conftest.py MARKER_TO_SCHEMA registration** updated per wave so each commit's added schemas are discoverable by the harness.

### External-standards bindings (from explored CM)

- **JSON-LD** (SourceVersion `@context`): accept arbitrary object shape under `canonicalSourceRef.jsonLdContext`; no schema validation of inner @context.
- **DPV IRIs** (DataElement.sensitivity): string with IRI-format pattern (`^https?://`); not enum (DPV is open vocabulary).
- **OASIS LegalRuleML** (deontic kinds): `body.defeasible: boolean` + JSON-LD lrml:* terms accepted as `additionalProperties: true` under `body.legalRuleML`.
- **DMN one-way import** (DMNImport.dmnXml): plain string field accepting XML content; transpilation happens compiler-side.
- **CloudEvents** (EventBinding.cloudEventsExtensions): typed fields `woscausationeventid`, `woscorrelationkey`.
- **PROV-O**: not a schema field — export-time transformation only (per CM §6.1 footnote).

## Critical files

- `studio-authoring/CONCEPT-MODEL.md` (§6.1 corrections; pre-stage)
- `schemas/studio/*.schema.json` — 14 new schema files (wave commits)
- `tests/schemas/conftest.py` — MARKER_TO_SCHEMA dict additions per wave
- `tests/schemas/test_fixture_validity.py` (or new `test_studio_examples.py`) — parametrize over `studio-authoring/examples/**/*.json`
- `tests/schemas/fixtures/studio/*.json` — negative fixtures for load-bearing rules

Reference shapes (read-only):
- `schemas/wos-workflow.schema.json` (parent shapes; $ref targets)
- `schemas/wos-tooling.schema.json` (no scenarios $def; conformanceTrace is the runtime correlate)
- `studio-authoring/examples/snap-redetermination-from-sources/policy-objects/*.json` (truth artifacts to validate against)

## Verification

End-to-end test plan:

1. **Pre-stage:** CM §6.1 patch parses; cross-references in spec set still resolve. Run `grep -rn "§6.1\|wos-tooling.schema.json scenarios" studio-authoring/` and verify no stale references survive.
2. **Per wave:** `python3 -c "import json; [json.load(open(f)) for f in glob.glob('schemas/studio/*.schema.json')]"` — every schema parses.
3. **Per wave:** `python3 -m pytest tests/schemas -q` — baseline (288 passed, 8 pre-existing failures) MUST hold; new tests MUST NOT add to failure count.
4. **Per wave:** `schema_doc_zero_regression` ratchet — every `x-lm.critical=true` node carries `description` AND ≥1 `examples`. Verify via a small Python walker (one-time script) reading each new schema and asserting the rule.
5. **Wave 4 (final):** validate every artifact under `studio-authoring/examples/snap-redetermination-from-sources/` against its corresponding registered schema. Drift between v4 specs and the example artifacts surfaces here; fix the example, not the schema (specs are spec-authoritative).
6. **Cargo:** `cargo check --workspace` — schema additions don't ripple into Rust code (parent FEL crate path absent in sandbox is unrelated; verify locally with parent fel-core present).

Per-wave commit messages follow the convention used in this branch (subject ≤72 chars, body explains rationale + cross-cutting impact, ends with `Co-Authored-By: Claude <noreply@anthropic.com>`).

## Out of scope of this plan

- Stage 4 (Rust lint engine impl in `crates/wos-lint`; K-051/K-052/K-053 implementation; conformance fixtures requiring runtime).
- Stage 5 (Studio→WOS compiler implementing `compiler-contract.md`).
- Stage 6 (scenario simulator runtime).
- Runtime evaluator for DecisionTableGuard in `wos-runtime` (kernel impl work, not Studio Stage 3).
- Adding the 5 still-queued WOS extensions (ApplicabilityScope, EffectivePeriod, top-level wosVersionPin, governance.deonticConstraints LegalRuleML, DPV/canonicalTermRef) — explicitly Q1-deferred.
- Federated/multi-workspace concerns (`x-federation` slot stays reserved).

## Risks

1. **`schema_doc_zero_regression` is mechanically expensive.** ~50+ critical nodes need description+examples. Plan agent's recommendation: use `critical=true` sparingly. Hold the line on 5-10 critical fields per schema; let other fields be standard description-only.
2. **The vertical-slice examples may not validate cleanly.** Authored under v3/v4/bridge-inference churn; some shapes may have drifted. Wave 4 surfaces this. Resolution: fix the example to match the schema, not the schema to match the example. The schema is spec-authoritative.
3. **Bridge-kind relocation may surface compositional gaps.** Bridges referenced from PolicyObject mappings may not have a clean reference path now that bridges live in workflow-intent. Resolution: cross-schema $ref via short URL form is supported by the harness.
4. **The `wosTarget` JSONPath field is a soft contract** — not validated against parent WOS schema structure. A Stage-4 lint rule (`SA-MUST-map-007` perhaps) would assert the JSONPath actually resolves; for Stage 3 it stays a documentation field.

## Landed state (preserved for context)

Branch `claude/wos-studio-setup-zFFDC` HEAD `ded8f31`. Commits this session:

- `b3d00a1` — substrate-choice clarification (CM §5 "WOS as canonical substrate"; SA-MUST-cmp-005/006 thin-projection rules)
- `1892a47` — decisionTable kernel extension (Kernel §4.5.1; first parent-side WOS extension to land; K-051/K-052/K-053 lint catalog)
- `ded8f31` — bridge.kernelKind inference (SA-MUST-wfi-005/006; example dropped redundant field on 13 of 15 elements; cross-cutting audit cleanup)

Stage 2 (16 specs, 34 entities, ~600 SA-MUST IDs across families) complete; ready for Stage 3 schemas per this plan.
