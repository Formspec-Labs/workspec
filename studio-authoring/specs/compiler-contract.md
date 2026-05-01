# Studio Spec: Compiler Contract

**Status:** draft (Stage 2 of [Implementation Roadmap](../VISION.md#17-implementation-roadmap))
**Date:** 2026-04-30
**Concept-model anchor:** none directly; this spec defines the *transformation* between the data described in CM §1 and the published WOS artifacts.
**PRD anchor:** [`../VISION.md`](../VISION.md) §11 (Compiler / Scenario Runtime / Reference Architecture). Companion PRD §3 (capabilities-first framing).
**Depends on:** every other Studio spec (consumer of all of them).

## Scope

This spec defines the **transformation contract** for the Studio→WOS compiler:

```
PolicyObject* + StudioToWosMapping* + WorkflowIntent + Bindings*
   → $wosWorkflow (conformant to wos-workflow.schema.json)
   → wos-tooling.scenarios[*] (conformant to wos-tooling.schema.json)
   → ApprovalPackage (per review-and-approval.md)
```

It is the **center-of-gravity spec**. Every other Studio spec describes *what gets created*; this one describes *what gets emitted, how, deterministically, with what gates*.

This spec defines:

- the **input shape**: what workspace state is required to compile.
- the **output shape**: what the compiler produces.
- the **determinism guarantees**: identical inputs ⇒ identical output, byte-for-byte (modulo JSON key ordering).
- the **external gates**: the schema-pass / lint-pass / conformance-pass thresholds that gate publication.
- the **failure modes**: what causes compilation to halt vs. emit with warnings.
- the **projection rules** that govern what workspace metadata projects compactly into the artifact vs. stays in the workspace.
- the **versioning contract**: how compiler version bumps, schema version bumps, and workflow version bumps interact.

## Out of scope

- The compiler implementation language (Rust per `wos-core` philosophy is the working assumption, but the contract is implementation-language-agnostic).
- The scenario simulator (Stage 6; spec is in [`scenario-authoring.md`](scenario-authoring.md)).
- The readiness engine implementation (Stage 4; rules are in [`readiness-validation.md`](readiness-validation.md)).
- The Validation Center UX.

## Terminology

- **Compile** — the transformation from workspace state to a published artifact.
- **Compile-input** — the workspace state required as input.
- **Compile-output** — the produced artifacts.
- **Determinism boundary** — the inputs that, fixed together, fully determine the output.
- **External gate** — a pass/fail check produced by something outside Studio's control: `wos-workflow.schema.json` validation, `wos-lint`, `wos-conformance`.
- **Projection** — the subset of workspace state that is emitted into the artifact (see [`authoring-provenance.md`](authoring-provenance.md) `SA-MUST-prov-030`).
- **Failure mode** — `halt` (compilation stops; no artifact produced), `emit-with-warnings` (artifact produced; tier-S6 ValidationFindings recorded), `emit-with-blockers` (artifact produced for inspection but cannot be published).

## Compile-input

The compiler reads, from a single Workspace at a single point in time:

1. **`WorkflowIntent`** — exactly one, the subject of compilation, in lifecycle state ≥ `mapped`.
2. **PolicyObjects** — every `policyObjectRefs[]` entry of the WorkflowIntent's elements, in lifecycle state `approved` or later.
3. **StudioToWosMappings** — exactly one mapping per referenced PolicyObject, in lifecycle state `approved` or later.
4. **Bindings** — every binding (`ServiceBinding`, `EventBinding`, `PolicyEngineBinding`, `DecisionTable`) referenced by a WorkflowElement, in lifecycle state `approved` or later.
5. **Scenarios** — every Scenario whose `workflowIntentRef` matches the subject WorkflowIntent, in lifecycle state ≥ `reviewed`.
6. **AuthoringProvenanceRecords** — every record on the above objects (for the projection rule).
7. **ApprovalDecisions** — every `active` decision on the above objects (for the ApprovalPackage).
8. **ValidationFindings** — every open / acknowledged / waived finding (for the gate evaluation).
9. **WorkspacePolicy** — workspace-administrator-configured behavior (per [`workspace.md`](workspace.md)).
10. **The compiler version** — pinned at compile time; recorded in the output.

## Compile-output

The compiler produces, on a successful compile:

1. **`$wosWorkflow` document** — conformant to `../../schemas/wos-workflow.schema.json`. Carries the workflow envelope, lifecycle, governance/agents/aiOversight/signature/custody/advanced/assurance embedded blocks (where applicable), actors, integration bindings, and provenance configuration.
2. **`wos-tooling.scenarios[*]`** — conformant to `../../schemas/wos-tooling.schema.json` `scenarios[*]`. One entry per emitted Scenario.
3. **`ApprovalPackage`** — per [`review-and-approval.md`](review-and-approval.md) §"Data model".
4. **Release notes** — derived from SemanticDiff vs. the prior published version (when applicable; per [`change-impact.md`](change-impact.md)).
5. **Compile manifest** — `{compilerVersion, schemaVersion, compiledAt, compiledBy, workspaceId, workflowIntentId, workflowIntentVersion, sourceVersionsConsumed[], policyObjectsConsumed[]}` for reproducibility audit.

On a failed compile, the compiler produces:

1. **A structured failure record** — `{failureKind, failedAt: phase, missingInputs[], structuralErrors[], findings[]}`. No partial `$wosWorkflow` document is produced; either the artifact is complete and conformant, or it does not exist.

## Compilation pipeline

The compiler executes in **eight ordered phases**, matching PRD §11 §"Compiler passes":

```text
1. Select approved policy objects   → assemble compile-input
2. Resolve mapping records          → bind every PolicyObject to a target
3. Generate workflow intent         → walk the WorkflowIntent's elements
4. Emit lifecycle / governance / AI / advanced / assurance blocks
5. Emit scenario artifacts          → project Scenarios into wos-tooling.scenarios[*]
6. Run Studio readiness checks      → tier S1–S6 evaluation; halt-or-warn per failure-mode rules
7. Run WOS schema/lint/conformance  → external gates
8. Produce review package           → ApprovalPackage + manifest + release notes
```

Phases are strictly ordered. A phase MUST NOT begin until the prior phase completes successfully. Phase failure halts the pipeline at that phase; partial outputs from prior phases are discarded.

## Normative Contract

### Determinism

- **`SA-MUST-cmp-001`** — Compilation MUST be **deterministic**: given identical compile-inputs (frozen at a specific moment, including identical compiler version, identical schema version, identical workspace state, identical referenced objects), the output `$wosWorkflow` MUST be identical byte-for-byte modulo JSON key ordering. *(fixture-pending: round-trip determinism tests.)*
- **`SA-MUST-cmp-002`** — JSON key ordering in the output MAY vary; structural identity (semantic equality) is the determinism boundary. The compile manifest's `sourceVersionsConsumed[]` and `policyObjectsConsumed[]` MUST be sorted deterministically (e.g., by id) so audit comparisons are stable. *(runtime-pending.)*
- **`SA-MUST-cmp-003`** — The compiler MUST NOT introduce non-deterministic content (timestamps, UUIDs, random ordering) into the artifact. Timestamps in the manifest are compile-time; timestamps in the artifact body (e.g., effective dates) come from the input PolicyObjects. *(fixture-pending.)*
- **`SA-MUST-cmp-004`** — When two PolicyObjects produce equivalent artifact content (e.g., two NoticeRequirements compiling to overlapping `governance.notices[*]` entries), the compiler MUST detect the collision at phase 4 and halt with `failureKind = artifact-collision`. AI / reviewer judgment MUST resolve the collision; the compiler does NOT auto-merge. *(runtime-pending; cross-cutting [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md) `mapping-target-collision`.)*

### Input completeness

- **`SA-MUST-cmp-010`** — Phase 1 (Select approved policy objects) MUST verify that every `policyObjectRefs[]` entry of the WorkflowIntent's elements is in lifecycle state `approved` or later. Missing approvals halt the pipeline with `failureKind = unapproved-input`. *(runtime-pending.)*
- **`SA-MUST-cmp-011`** — Phase 2 (Resolve mapping records) MUST verify that every PolicyObject has exactly one mapping. PolicyObjects without a mapping halt the pipeline with `failureKind = unmapped-input`. *(runtime-pending; cross-cutting `SA-MUST-map-001`.)*
- **`SA-MUST-cmp-012`** — Phase 3 (Generate workflow intent) MUST verify that every WorkflowElement's `bridge` is well-formed for its `kind`. Malformed bridges halt with `failureKind = malformed-bridge`. *(runtime-pending; cross-cutting [`workflow-intent.md`](workflow-intent.md) `SA-MUST-wfi-040`.)*
- **`SA-MUST-cmp-013`** — When the WorkflowIntent references events for which no `EventBinding` exists, phase 4 MUST halt with `failureKind = unresolved-event-reference`. (Cross-cutting [`binding-and-integration.md`](binding-and-integration.md) `SA-MUST-bind-023`.) *(runtime-pending.)*
- **`SA-MUST-cmp-014`** — When the WorkflowIntent references API operations for which no `ServiceBinding` covers required inputs, phase 4 MUST halt with `failureKind = incomplete-service-binding`. *(runtime-pending.)*

### Emission rules

- **`SA-MUST-cmp-020`** — Phase 4 emits `$wosWorkflow` content; only `mapsToWos` PolicyObjects (and their bindings) project into the artifact body. `authoringOnly`, `requiresSpecExtension` (without an `x-` target), and `unmappedButApproved` PolicyObjects MUST NOT produce artifact body content. *(runtime-pending; cross-cutting `SA-MUST-map-030`, `SA-MUST-map-040`.)*
- **`SA-MUST-cmp-021`** — `requiresSpecExtension` PolicyObjects whose ExtensionRecord is `lifecycleState = shipped` AND has an `x-` target MUST emit content under that `x-` key. ExtensionRecords still `open` MUST halt the pipeline if their host PolicyObject is workflow-bearing. (Cross-cutting `SA-MUST-map-005`.) *(runtime-pending.)*
- **`SA-MUST-cmp-022`** — Phase 5 emits `wos-tooling.scenarios[*]` entries. Only Scenarios in lifecycle state ≥ `reviewed` are emitted. Scenarios in `failing` or `acceptedAsKnownGap` are emitted with a status flag (per `SA-MUST-scn-023`) so WOS conformance does not treat them as expected-passing. *(runtime-pending.)*
- **`SA-MUST-cmp-023`** — `unmappedButApproved` mappings MUST be enumerated in the published artifact's release notes per `SA-MUST-map-042`. *(runtime-pending.)*
- **`SA-MUST-cmp-024`** — Authoring provenance MUST be projected per [`authoring-provenance.md`](authoring-provenance.md) `SA-MUST-prov-030` / `SA-MUST-prov-031` / `SA-MUST-prov-032`. The compiler is the sole emitter of compact projections. *(runtime-pending.)*

### External gates (the named gates Priya cares about)

- **`SA-MUST-cmp-030`** — Phase 7 MUST run three external gates **in order**, in this exact sequence, each producing a named pass/fail result. Subsequent gates MUST NOT run if a prior gate fails:
  1. **`schema-pass`** — the produced `$wosWorkflow` document validates against `../../schemas/wos-workflow.schema.json`. Pass means: schema validation returns no errors.
  2. **`lint-pass`** — the produced document passes `../../crates/wos-lint`. Pass means: every WOS lint constraint either passes or is explicitly waived (per `wos-lint`'s waiver mechanism, not the Studio waiver mechanism — these are distinct).
  3. **`conformance-pass`** — emitted scenarios pass `../../crates/wos-conformance` against the compiled artifact. Pass means: every emitted scenario produces an actual trace matching its expected trace (per [`scenario-authoring.md`](scenario-authoring.md) §"Conformance trace correspondence").
  
  Each gate's result is recorded in the compile manifest. *(runtime-pending; cross-cutting [`readiness-validation.md`](readiness-validation.md) tier S6.)*
- **`SA-MUST-cmp-031`** — Failure of any external gate produces a tier-S6 ValidationFinding lifted into the workspace per [`readiness-validation.md`](readiness-validation.md) §"WOS mappings". The artifact is NOT published; the workflow's `lifecycleState` does NOT advance. *(runtime-pending.)*
- **`SA-MUST-cmp-032`** — The three gate names (`schema-pass`, `lint-pass`, `conformance-pass`) MUST be preserved verbatim across spec versions and compiler versions. They are the contract by which technical implementers ([`../VISION.md`](../VISION.md) §3 user role) triage publication failures. *(spec-stability constraint.)*

### Failure modes

- **`SA-MUST-cmp-040`** — Compilation has exactly three failure dispositions:
  - `halt` — pipeline stops; no artifact produced; failure record returned.
  - `emit-with-warnings` — artifact produced; tier-S6 ValidationFindings recorded at severity `warn`; publication may proceed at workspace-administrator discretion (subject to publication-gate rules).
  - `emit-with-blockers` — artifact produced for inspection (e.g., expert-mode review); cannot be published; tier-S6 findings at severity `block`.
  
  Implementations MUST NOT introduce intermediate dispositions. *(schema-pending: enum.)*
- **`SA-MUST-cmp-041`** — `halt` disposition is the default for: missing inputs (input-completeness rules), malformed bridges, artifact collisions, blocked external gates (`schema-pass = fail`).
- **`SA-MUST-cmp-042`** — `emit-with-warnings` applies for: `unmappedButApproved` mappings (warn-perpetual per `SA-MUST-map-041`), failing scenarios marked `acceptedAsKnownGap`, soft tier-S5 coverage gaps.
- **`SA-MUST-cmp-043`** — `emit-with-blockers` applies for: failing scenarios not marked `acceptedAsKnownGap`, tier-S6 findings unresolved at severity `block`, lint failures.

### Versioning contract (composition with parent RELEASE-STREAMS.md + COMPATIBILITY-MATRIX.md)

Per CM §1.33 MigrationPath: every WorkflowIntent declares a `wosVersionPin` (a claim string per parent [`RELEASE-STREAMS.md`](../../RELEASE-STREAMS.md), e.g., `kernel@1.0, governance@1.0, ai@0.5, signature@1.0, custody@1.0, advanced@0.3, assurance@1.0`). The compile manifest carries this pin as load-bearing reproducibility metadata.

- **`SA-MUST-cmp-050`** — Every compile manifest MUST record `compilerVersion`, `schemaVersion` (the version of `wos-workflow.schema.json`), `wosVersionPin` (claim string per parent RELEASE-STREAMS.md), and the `workflowIntentVersion` consumed. *(schema-pending.)*
- **`SA-MUST-cmp-051`** — A compiler version bump MAY change the artifact byte-for-byte (e.g., new optional fields populated, deprecated fields cleaned up); semantic equality with the prior compiler version's output for the same inputs MUST be preserved. *(fixture-pending.)*
- **`SA-MUST-cmp-052`** — A WorkflowIntent's `wosVersionPin` constrains compilation: the compiler MUST refuse to compile against schema/stream versions outside the pin. Updating the pin is an explicit reviewer action per `change-impact.md` `triggerKind = wos-version-deprecation`. *(runtime-pending.)*
- **`SA-MUST-cmp-053`** — Two consecutive published versions of a WorkflowIntent MUST produce a SemanticDiff (per [`change-impact.md`](change-impact.md)) used to derive release notes. The diff is computed from the WorkflowIntent versions, not from the compiled artifacts; this avoids cosmetic-only diffs cluttering the release notes. *(runtime-pending.)*
- **`SA-MUST-cmp-054`** — When a parent stream version reaches deprecation per parent `COMPATIBILITY-MATRIX.md`, the compiler MUST surface a tier-S6 ValidationFinding `CMP-LINT-010` "wos-version-deprecation-pending" 90 days before the deprecation effective date, prompting reviewer-driven migration. *(lint-pending.)*

### Reproducibility / disaster-recovery

- **`SA-MUST-cmp-060`** — Given the compile manifest, the workspace state at the recorded time, and the recorded compiler/schema/wosVersionPin versions, the same compile MUST be reproducible. *(fixture-pending.)*
- **`SA-MUST-cmp-061`** — The compile manifest MUST be small enough to be embedded in the published artifact. (Manifest details that exceed embedding capacity are stored in the workspace export per [`authoring-provenance.md`](authoring-provenance.md) `SA-SHOULD-prov-034`.) *(schema-pending.)*
- **`SA-MUST-cmp-062`** — Workspace-export-driven disaster recovery: given a workspace export bundle (sources + PolicyObjects + mappings + scenarios + provenance log + compile manifest + custody-anchored receipts per [`authoring-provenance.md`](authoring-provenance.md) `SA-MUST-prov-081`) and the recorded compiler version, an external party MUST be able to reproduce the published artifact byte-for-byte (modulo JSON key order). The workspace export is the disaster-recovery primitive. *(runtime-pending.)*
- **`SA-MUST-cmp-063`** — The workspace export bundle MUST be self-contained: every external reference (parent schemas, parent custody receipts) MUST either (a) be inlined, or (b) carry an externally-resolvable URI + content hash. *(schema-pending.)*

### Composition with parent wos-event-types.md (PLN-0384)

The compiler emits **provenance events** during compile (start, phase transitions, gate results, halt-or-success). These events MUST conform to parent **PLN-0384** `wos-event-types.md` taxonomy in the `wos.compiler.*` namespace:

- **`SA-MUST-cmp-070`** — Compiler-phase transitions MUST emit `wos.compiler.phase-started` / `wos.compiler.phase-completed` / `wos.compiler.phase-halted` events with phase identifier and outcome. *(runtime-pending; coordination-pending parent PLN-0384.)*
- **`SA-MUST-cmp-071`** — External gate results (schema-pass / lint-pass / conformance-pass) MUST emit `wos.compiler.gate-passed` / `wos.compiler.gate-failed` with gate name and finding refs. *(runtime-pending.)*
- **`SA-MUST-cmp-072`** — Compile success / failure MUST emit `wos.compiler.compile-succeeded` (with manifest ref) / `wos.compiler.compile-failed` (with failureKind). *(runtime-pending.)*
- **`SA-MUST-cmp-073`** — Compiler events MUST custody-anchor per [`authoring-provenance.md`](authoring-provenance.md) `SA-MUST-prov-081`. The compile manifest's `manifestHash` and the ApprovalPackage's binding to the artifact derive their authenticity from this anchor chain. *(runtime-pending.)*

## Composition

### Attachment point

The compiler attaches at the publication boundary of a Workspace. It reads the Workspace's state at a point in time and produces external artifacts. It is the **only** Studio component that emits content outside the Workspace.

### Precedence

When multiple PolicyObjects of different mapping states compete for the same artifact path:
1. `mapsToWos` (with concrete target) wins over `requiresSpecExtension` (with `x-` target).
2. Both forms are emitted; the published artifact MAY contain both a structural entry AND an `x-` extension entry (when both apply, e.g., a NoticeRequirement that maps to `governance.notices[*]` with an additional `x-translation-parity` extension).
3. `unmappedButApproved` produces no artifact entry; only release-note enumeration.

When two `mapsToWos` PolicyObjects target the **same** JSON path (collision), the compiler halts per `SA-MUST-cmp-004`; the resolution is reviewer-driven, never compiler-driven.

### Conflict handling

The compiler does NOT resolve conflicts. It detects them and halts. Conflicts are resolved upstream:
- PolicyObject conflicts: by ReviewerResolution per [`policy-object-model.md`](policy-object-model.md) §"Conflict surface".
- Mapping collisions: by re-mapping per [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md) §"Conflict handling".
- ApprovalDecision conflicts: by aggregation per [`review-and-approval.md`](review-and-approval.md) §"Precedence".

This separation keeps the compiler deterministic.

### Versioning / migration

- Adding new compile phases: schema-breaking; coordinated with the readiness engine and Validation Center.
- Adding new failure-mode dispositions: schema-breaking.
- Renaming an external gate name: **forbidden** (per `SA-MUST-cmp-032`).
- Compiler-implementation refactors that preserve the contract: non-breaking.

## Conformance

### Schema validation (Stage 3)

- Compile manifest required fields.
- Failure-record shape.
- ApprovalPackage shape (cross-spec with [`review-and-approval.md`](review-and-approval.md)).
- External-gate result shape (`{gate, status, errors[]}`).

### Lint rules (Stage 4)

Tier-S6 (Publication readiness) rules planned:

- `CMP-LINT-001` — every workflow advancing to `published` has a successful compile-manifest (`SA-MUST-cmp-030`).
- `CMP-LINT-002` — `schema-pass` succeeded (`SA-MUST-cmp-030`).
- `CMP-LINT-003` — `lint-pass` succeeded (`SA-MUST-cmp-030`).
- `CMP-LINT-004` — `conformance-pass` succeeded (`SA-MUST-cmp-030`).
- `CMP-LINT-005` — every emitted artifact has a release-note enumeration of `unmappedButApproved` mappings (`SA-MUST-cmp-023`).

### Runtime conformance fixtures (Stage 4–5)

- Determinism: identical inputs produce byte-for-byte (modulo key order) identical output.
- Halt-on-collision: two PolicyObjects targeting the same `$.governance.notices[*]` entry halt the pipeline.
- External-gate ordering: `schema-pass` precedes `lint-pass` precedes `conformance-pass`; failure of `schema-pass` does not run subsequent gates.
- Reproducibility: given a compile manifest, the same compile reproduces the same artifact.
- Compiler-version compatibility: a new compiler version produces semantically equal artifacts to the prior version on the same inputs.

### Current limitations

- The compiler implementation is Stage 5 work; this contract is what it must satisfy.
- The exact JSON-emission algorithm (key-ordering policy, formatting, line-endings) is implementation detail; only structural identity is normative.
- Schema-version migration is sketched; the migration tool is deferred.

## WOS mappings

The compiler is the **sole emitter** of WOS-side artifacts. The mapping table:

| Compile phase | WOS output | Schema |
|---|---|---|
| Phase 4: lifecycle/governance/AI/advanced/assurance emission | `$wosWorkflow` document body | `wos-workflow.schema.json` |
| Phase 5: scenario emission | `wos-tooling.scenarios[*]` | `wos-tooling.schema.json` |
| Phase 7: external gate runs | (no artifact; gate results recorded in manifest) | `wos-workflow.schema.json` (gate 1), `wos-lint` (gate 2), `wos-conformance` (gate 3) |
| Phase 8: ApprovalPackage emission | bundled artifact | per [`review-and-approval.md`](review-and-approval.md) |

The compiler does NOT emit anything to `wos-delivery` (a deployment-environment sidecar) or `wos-ontology-alignment` (a separate-purpose sidecar). Those are workspace-tooling outputs deferred to later phases or operator workflows.

## Examples

### Example 1: Successful compile of a SNAP redetermination workflow

Compile-input: WorkflowIntent v1.2 (16 elements), 47 PolicyObjects (all `approved`, all mapped), 4 ServiceBindings, 6 EventBindings, 1 PolicyEngineBinding, 1 DecisionTable, 14 Scenarios (12 `passing`, 2 `acceptedAsKnownGap`), 89 active ApprovalDecisions, no open `error`/`block` findings.

Pipeline:
1. Phase 1 succeeds: all PolicyObjects approved.
2. Phase 2 succeeds: 47 mappings resolved; 0 `requiresSpecExtension`, 0 `unmappedButApproved`, 47 `mapsToWos`.
3. Phase 3 succeeds: 16 elements with valid bridges.
4. Phase 4 emits `$wosWorkflow` with `governance` (3 notices, 1 appeal, 2 explanation requirements), `agents` (1 triage agent + AI Use config), `lifecycle` (compound + atomic states for phases/steps, transitions for decisions/exceptions, timers for deadlines, tasks for reviews), `actors` (caseworker, supervisor, applicant, triage-agent), `integration.bindings` (4 OpenAPI calls + 6 events + 1 policy-engine).
5. Phase 5 emits 14 `wos-tooling.scenarios[*]` entries (2 with status `accepted-gap`).
6. Phase 6: tier S1–S6 readiness all green.
7. Phase 7:
   - `schema-pass`: ✓ (`wos-workflow.schema.json` validates).
   - `lint-pass`: ✓ (197 wos-lint constraints pass).
   - `conformance-pass`: ✓ (12 of 12 expected-passing scenarios match; 2 `accepted-gap` scenarios do not run as pass-fail).
8. Phase 8 emits ApprovalPackage with 89 ApprovalDecisions, citation manifest (89 SourceCitations), scenario suite (14), validation report snapshot, release notes (semantic diff vs. v1.1).

Manifest recorded; artifact published. Disposition: success.

### Example 2: Halt at phase 4 due to artifact collision

Compile-input: a WorkflowIntent referencing two NoticeRequirement PolicyObjects, both mapping to `$.governance.notices[*]` with overlapping `field bindings` (one specifies a 90-day notice content; the other specifies a 60-day notice content for the same trigger Outcome).

Pipeline:
1. Phase 1: ✓.
2. Phase 2: ✓ (both `mapsToWos`).
3. Phase 3: ✓.
4. Phase 4: halt with `failureKind = artifact-collision`; record points to the two NoticeRequirement IDs and the conflicting JSON path.

No artifact produced. Tier-S3 finding recorded. Reviewer must resolve by either: deleting one Notice, merging them via Conflict resolution, or re-mapping one to a different target.

### Example 3: Emit-with-warnings disposition

Compile-input: identical to Example 1 except 1 `unmappedButApproved` mapping is present (a state-specific local-policy field that is not modeled in WOS today).

Pipeline runs through phase 8 normally. The `unmappedButApproved` mapping does not produce artifact body content; it appears in release notes per `SA-MUST-cmp-023`. Tier-S3 `MAP-LINT-004` fires at severity `warn`; disposition is `emit-with-warnings`. Workspace administrator may proceed to publish (subject to publication-gate per `review-and-approval.md` `SA-MUST-ra-040` — the warning is informational, not blocking).

## Open issues

- **Compile-time vs. publish-time gates.** The current spec runs external gates in phase 7 of compilation. An alternative is to compile first, then have a separate publication step that runs gates. The current model is tighter but conflates two responsibilities.
- **Incremental compilation.** Today the contract assumes whole-workspace compile per WorkflowIntent. Incremental compilation (only re-emitting changed sections) is a performance optimization, deferred.
- **Compiler-version deprecation.** When does an old compiler version stop being supported? Workspace-administrator policy or ecosystem policy is unsettled.
- **Cross-workflow compilation.** A workspace with three workflows compiles each independently; cross-workflow interactions (e.g., a SNAP workflow producing case-state consumed by a TANF workflow) are not addressed.
- **Schema-version migration tool.** Sketched in `SA-MUST-cmp-052`; the actual tool is deferred to Stage 5.

## Cross-references

- Concept model: this spec consumes every entity in `../CONCEPT-MODEL.md`.
- PRD: [`../VISION.md`](../VISION.md) §11 (Compiler / Scenario Runtime / Reference Architecture), §17 Stage 5.
- Upstream (read state from): every other Studio spec.
- Downstream (produces output for): WOS runtime adapters; technical implementers.
- WOS: `../../schemas/wos-workflow.schema.json`, `../../schemas/wos-tooling.schema.json`, `../../crates/wos-lint`, `../../crates/wos-conformance`.
- Repo conventions: [`../../CONVENTIONS.md`](../../CONVENTIONS.md).
