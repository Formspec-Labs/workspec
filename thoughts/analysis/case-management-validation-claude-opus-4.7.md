# Validation of `case-management.md`

**Reviewer:** Claude Opus 4.7 (Cursor IDE), with delegated investigations to one `spec-expert` subagent and four `code-scout` subagents.
**Date:** 2026-05-10
**Subject:** [`case-management.md`](./case-management.md) — consultant memo proposing a Case / CaseProcess boundary refactor for WOS.

Five subagents (1× `spec-expert`, 4× `code-scout`) cross-checked the consultant memo against WOS spec text, runtime/API code, ADRs, and migration blast radius.

## Headline verdict

**Conceptually directionally sound, but with one architectural blind spot that needs to be resolved before the ADR is written.** The technical claims about the existing code are mostly accurate but have several precise misstatements, ~3 broken file references, and one significant ADR-misreading that the ADR text would propagate if not fixed.

The memo's central move — *durable Case aggregate ≠ workflow runtime instance* — matches what WOS already says internally (`CaseInstance` is normatively a "running workflow instance"; Kernel §5.1 says lifecycle ⊥ caseState). But the memo writes as if WOS has *no* concept of governed case today, when in fact **ADR 0073 D-1 places "governed case" identity inside WOS** and Kernel §11.4 already distinguishes `caseCreated` from `instanceCreated`. The unresolved question — does product `Case` collapse onto WOS `governed case`, or coexist as a second identity above it? — is the load-bearing decision the ADR must make, and the memo doesn't address it.

## 1. Conceptual validation (spec-expert)

| Claim | Verdict | Why |
|---|---|---|
| `CaseInstance` "conflates" Case + workflow instance | **PARTIALLY VALID** | Spec text is explicit it's a workflow runtime artifact (Kernel §11.1, schema description, `specs/api/instance.md` L10). The conflation is **product-naming and API root choice**, not spec-text. |
| `caseState` = workflow process business data | **VALID** | Kernel §5.1: per-instance, mutated by setData/output bindings, scoped to one workflow run. |
| Lifecycle states ≠ case status | **VALID** | Kernel §5 explicitly separates lifecycle from case data. |
| DCR constraint zones are overlays on compound states | **VALID** | Advanced §4.7: "governance overlay on existing `compound` states." |
| Intake supports `accepted \| attachToExistingCase \| deferred` | **PARTIALLY VALID** | Memo conflates two layers: ADR 0073 D-4 modes are `workflowInitiated \| publicIntake`; Kernel §11.4.1 acceptance outcomes are `attachToExistingCase \| createGovernedCase` + `intakeDeferred`. Both are real; the memo merges them confusingly. |
| ADR 0066 = amendment/correction/rescission/supersession/reinstatement | **VALID** | Five canonical revisit modes confirmed verbatim. |
| Governed output bindings need new `CaseStateMutation`/`CaseArtifact`/`CaseDecision`/`CaseTimelineEvent` write paths | **PARTIALLY VALID** | Kernel §9.2.36 + ADR 0080 already govern case-field writes via `outputBinding`. The artifact/decision/timeline split is **net-new product machinery**, not a constraint already in WOS. |
| Migration must preserve `caseId` | **INVALID as stated** | Kernel §11.2 migration semantics have no `caseId` today; this would be **new** normative content. Stack ADR 0071 has `caseId: TypeID` in `MigrationPinChanged` but at the stack record layer, not Kernel migration steps. |
| Tenant invariant Case ↔ CaseProcess | **PARTIALLY VALID** | ADR 0068 + instance schema have `tenant`; Case ↔ CaseProcess match is a plausible *new* invariant, not an existing MUST. |
| Provenance is process-scoped, aggregable | **VALID** | `provenancePosition` is per-instance cursor; cross-instance aggregation is a projection, not normative. |
| `$wosCaseInstance` marker should remain | **VALID** | Renaming would break schema/lint/conformance contract; aliasing is the right call. |

### The blind spot

The memo says "Case management lives one layer above WOS" (line 205). But:

- **ADR 0073 D-1**: *"WOS is the only layer that emits the governed case boundary event."* WOS owns governed case identity.
- **ADR 0068 D-2**: tenant/scope four-tuple is the case identity.
- **Kernel §11.3–11.4**: explicit `caseCreated` vs `instanceCreated` distinction.

If the memo's `Case` is meant to *be* WOS's governed case (renamed/projected), **ADR 0073 stays intact**. If it's meant to be a *second* aggregate above WOS's governed case, **ADR 0073, ADR 0068, ADR 0071 all need re-homing** and the stack story duplicates "case" identity — that's a much larger ADR than the memo describes.

The ADR template in the memo does not list this as a "decision required" — it should be the **first** decision.

## 2. Technical validation (4× code-scout)

### Runtime model (`instance.rs`, schemas) — mostly accurate

All 11 enumerated fields confirmed: `definition_url`, `definition_version`, `configuration`, `case_state`, `active_tasks`, `timers`, `pending_events`, `status`, `governance_state`, `volume_counters`, `provenance_position`. Specifically:

- `instance.rs:36-109` carries every field as named.
- Lifecycle status enum: doc lists six values (`active | suspended | migrating | completed | terminated | stalled`); **kernel schema actually has nine** — adds `declined`, `voided`, `expired` (`wos-case-instance.schema.json:273-285`). Doc's six matches the *public API* `LifecycleState`, not the kernel runtime artifact.
- **No `caseId` field exists on `CaseInstance`** — identity is `instance_id`. `correlationKey` (public API) is the closest existing case-linkage analog but is not parent-pointer.
- TypeID prefix `case` already exists (`typeid.rs:15-16`); URN format is `urn:wos:{tenant}_case_{base32}`. **A new `Case` aggregate would either need a new TypeID family or collide with the current `case` prefix that today identifies instances.** This is unstated in the memo and is a real design constraint.
- Memo missed several CaseInstance fields a refactor would need to relocate: `next_task_sequence`, `history_store`, `compensation_logs`, terminal-status satellites (`stalled_since`, `decline_reason`, `voided_by/at`, `expired_at`), `fired_milestones`, `pending_callbacks`, `extensions`.

### Public API — proposals are additive, but reference server is already drifting

- `/api/v1/cases` and `/api/v1/case-processes` **don't exist today** — proposal is purely additive, no naming conflict.
- Public `CaseInstance` schema has 9 required + 8 optional fields including `dcrZones`, `milestonesFired`, `continuationOfServicesActive`, `correlationKey`, `tenant`. No `caseId`.
- Subresources confirmed: `/governance`, `/tasks`, `/timers`, `/holds`, `/related`, `/provenance`, `/custody`, `/compensation`.
- **Reference server is OUT OF SYNC with OpenAPI**: `suspend`/`resume`/`terminate` are in the contract but **not implemented** in `workspec-server/.../instances.rs`; `/explanation` (OpenAPI) ≠ `/explain` (server); tasks live at `/api/v1/tasks` (server) but OpenAPI also has `/instances/{id}/tasks`. The Case/Process refactor will inherit this drift if not noticed.
- Generated SDK consumers: `case-portal/src/types/wos/api-instance.ts` (json-schema-to-typescript, 62 hits in one file) and `wos-server` typify-generated Rust types from `work-spec/schemas/api/*.schema.json`. Both bind to the schema `$defs` shape.

### Cross-spec / ADRs — biggest factual issue

- **Memo misstates ADR 0073 modes.** Memo says "accepted, attachToExistingCase, deferred" (line 678). ADR 0073 D-4 actually defines `workflowInitiated | publicIntake` as initiation modes; Kernel §11.4.1 outcomes are `attachToExistingCase | createGovernedCase` + `intakeDeferred`. The Rust binding (`wos-formspec-binding/src/lib.rs:32-51`) implements `IntakeHandoffInitiationMode = WorkflowInitiated | PublicIntake` and `IntakeHandoffCaseIntent = AttachToExistingCase | CreateCaseAfterAcceptance`. **The ADR will inherit this misstatement if copied directly.** Fix: cite both layers explicitly.
- ADR 0066 confirmed verbatim (5 modes). ADR 0080 confirmed (governed output-commit pipeline; case-field paths only — does *not* pre-define artifact/decision/timeline types).
- VISION.md / STACK.md confirm the Formspec → WOS → Trellis layering but **do not assert a fourth "Case" layer** — confirms this memo is proposing genuinely new layering, not codifying existing structure.
- `caseRelationships` already exists in Kernel §5.5 with vocabulary `parent | child | sibling | related | supersedes`. API `RelatedCaseLink.relationship` adds `predecessor | successor | appeals | appealed-by`. **There's already documented divergence** (`thoughts/specs/2026-05-05-api-coverage-kernel.md:118-122`). Memo's proposed `CaseRelationshipKind` taxonomy partially overlaps and partially extends — needs to acknowledge and reconcile, not just declare.

### Blast radius

- **~584 matches across ~106 files** for `CaseInstance` patterns (work-spec + workspec-server combined).
- Top-density files: `instance.schema.json` (49), `instance.md` (40), OpenAPI (33), `temporal-reference-implementation.md` (13), `instance.rs` (12), portal `api-instance.ts` (62).
- Lint touchpoints: `wos-lint/src/document.rs:84-90` (`DocumentKind::CaseInstance`), `fel_analysis.rs`, `tier1.rs`.
- **Alias-only saves Rust churn but not the schema/OpenAPI/portal/typify chain** — if wire names or `$defs` titles change at all, portal regeneration is mandatory.
- ADR 0082 confirmed at stack root with closed-taxonomy discipline (D-12, lines 329-331); memo's characterization is correct.
- Tenant pattern: `^[a-z][a-z0-9-]{0,62}$` (single DNS label). No central "assert tenant match" function — enforcement is distributed across schema validation, TypeID parsing (`typeid.rs::extract_tenant`), auth scope tests (`auth_conformance.rs`).

## 3. Doc cleanup needed

### Broken file references in memo's "Files to inspect first" (lines 821-838)

| Memo cites | Reality |
|---|---|
| `crates/wos-core/src/model/kernel.rs` | **Does not exist** — there is no `model/` tree under `wos-core/src/`. Code is flat (`instance.rs`, `provenance/`, etc.). |
| `WOS-IMPLEMENTATION-STATUS.md` | **Does not exist** at the cited path. |
| `specs/kernel/spec.md`, `specs/advanced/advanced-governance.md` | **Not present in the current checkout** (one scout found them missing; another scout grepped them as present at `specs/kernel/spec.md` with 15 hits — likely a partial-checkout artifact, not a real absence; verify before ADR work). |

### Markdown corruption / typos in the memo

- Line 92: `├── ├── notes` (double-tree-glyph)
- Line 128: `├âme` (mojibake)
- Line 161: `casease-processes` → `case-processes`
- Line 250: `e-state.md` → `end-state.md`
- Line 329: `instae.md` → `instance.md`
- Line 334-335: `casschema.json` → `case.schema.json`
- Line 347: `processoutes` → `process routes`
- Line 354: `CaseProcessLink\n    CaseArtifact` (missing list bullet)
- Line 377: `-pdate` → `Update`
- Line 384: `Add schema test` (missing trailing word; probably `tests`)
- Line 894: `cargo nexst` → `cargo nextest`

### ADR template fixes needed before adoption (line 175-244)

1. **Add a "Decision Required #0"**: Does product `Case` *replace* WOS governed case (ADR 0073 D-1 is rewritten), *project from* it (ADR 0073 stays, Case is a read-side aggregate), or *coexist above* it (creates two case identities, requires re-homing 0073/0068/0071)? This is more load-bearing than the naming decision (`CaseProcess` vs `WorkflowInstance` vs `GovernedProcessInstance`).
2. **Fix intake-mode citation**: replace "accepted, attachToExistingCase, deferred" with the two-layer reality (ADR 0073 D-4 initiation modes + Kernel §11.4.1 acceptance outcomes).
3. **Add TypeID-prefix decision**: today `case_` prefix denotes an instance. Does new `Case` reuse or get its own family (e.g., `casefolder_`, `matter_`)?
4. **Add migration scope decision**: Kernel §11.2 has no `caseId`-preservation MUST today. Adding one is normative spec change, not just schema work — call it out.
5. **Acknowledge `caseRelationships` overlap** (Kernel §5.5 + API `RelatedCaseLink` divergence per `thoughts/specs/2026-05-05-api-coverage-kernel.md`) rather than proposing fresh taxonomy in vacuum.
6. **Server/OpenAPI drift**: the lifecycle ops (`suspend`/`resume`/`terminate`) the memo says to "reframe as process endpoints" don't exist in the reference server even though they're in OpenAPI. The Case/Process refactor either needs to land them or explicitly defer.

## 4. Strengths to preserve

- **Edge-case enumeration (lines 622-799) is excellent.** All 35 cases survive scrutiny; the spec-expert flagged none as already settled in WOS in a contradictory way. Use this as the test matrix.
- **DCR positioning is correct** (Advanced §4.7 confirms overlay-on-compound).
- **Recommendation against marker rename** (`$wosCaseInstance` legacy) is correct — it's load-bearing in lint/conformance.
- **Phase ordering** (ADR → end-state spec → landing plan → formal specs → schemas → code) matches repo conventions; no structural blockers found in any phase.

## Recommendation

**Don't ship the ADR as-described.** Ship a corrected v2 of the analysis first. Specifically:

1. Add the **governed-case-vs-product-Case identity question** as Decision #0 — answer it before listing other decisions.
2. **Fix the ADR 0073 intake-vocabulary citation.** Reading the memo, anyone will write the wrong words into the ADR and propagate them.
3. **Resolve the TypeID-prefix collision** (`case_` already in use).
4. **Reconcile with existing `caseRelationships`** rather than redefining.
5. **Repair the file/path references** (no `wos-core/src/model/`, no `WOS-IMPLEMENTATION-STATUS.md`).
6. Acknowledge that **alias-only does not avoid portal/OpenAPI/typify churn** — the migration has a hard floor of ~150 hits in schema/OpenAPI/portal even with maximally lazy Rust aliasing.

Once those are in, the memo's structure (boundary refactor, not rewrite; Case-above-WOS layering with WOS as instrument; phased landing) is **architecturally sound** given the WOS spec suite as it stands today.

---

## Appendix: methodology

This validation was produced by Claude Opus 4.7 (Cursor IDE) by reading the source memo, then dispatching five subagents in parallel/sequence:

| Subagent | Subject |
|---|---|
| `spec-expert` | Normative WOS spec validation (Kernel, Governance, AI, Advanced, sidecars, ADRs). |
| `code-scout` #1 | Runtime model: `crates/wos-core/src/instance.rs`, `wos-case-instance.schema.json`, public API `CaseInstance` shape. |
| `code-scout` #2 | Public API surface: OpenAPI, `specs/api/instance.md`, `workspec-server` route handlers, generated SDK consumers. |
| `code-scout` #3 | Cross-spec contracts: ADR 0066, 0073, 0080, custody seam, stack VISION/STACK alignment. |
| `code-scout` #4 | Migration blast radius (`CaseInstance` references), ADR 0082 closed-taxonomy, TypeID-in-URN, tenant discipline, phase-feasibility checks. |

All five agents ran in read-only mode. Findings cross-checked against each other; the few disagreements (e.g., presence of `specs/kernel/spec.md`) are flagged in §3 as checkout-state artifacts to verify before ADR work begins.
