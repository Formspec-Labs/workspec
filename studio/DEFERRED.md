# DEFERRED.md — Studio (Authoring)

Single source of truth for Studio-tier deferred work. Each entry has a
stable ID so other documents (`CLAUDE.md`, spec front-matter, test
comments, ratchet messages) can reference deferred items by ID rather
than carrying prose locally.

## Format

Each entry carries:

- **ID** — stable, never reused (`STUDIO-DEFER-NNN`).
- **Title** — one-line summary.
- **Origin** — commit hash + date the entry was opened.
- **Rationale** — why deferred, what unblocks it.
- **Inventory anchor** — file/line counts to re-discover the surface
  later (the actual numbers age, but the *shape* stays useful).
- **Trigger** — the event that re-opens the work.

When closing an entry, move it to `## Closed` and note the closing
commit; do NOT delete (the entry is the audit trail).

---

## Open


### STUDIO-DEFER-004-LINT — Lint-pending invariants (31 markers residual)

- **Origin:** D-wave 2026-05-03 split. D8-A burndown moved 17 mis-classified `schema-pending` markers here (cross-document refs, uniqueness on derived keys, state-machine transitions, etc.).
- **Rationale:** `(lint-pending)` markers track MUSTs that should
  graduate into a Studio-tier lint rule (`STUDIO-LINT-MATRIX.md`).
  The lint engine is closed (Stage 4) but the rule catalog is
  promoted incrementally; not every spec MUST yet has a fixture-backed
  rule.
- **Inventory anchor:** baseline 31 in
  `studio/tests/pending_ratchet.py::BASELINES["lint"]`.
  I-wave 2026-05-03 closed 36 markers across 8 clusters (I-A1..A8),
  taking lint 67 → 31:
  * I-A1 SV-LINT-007..014 (-9 source-vault).
  * I-A2 BIND-LINT-001..006 (-6 ServiceBinding).
  * I-A3+A4+A5 BIND-LINT-010..072 (-8 EventBinding/Policy/Coverage).
  * I-A6+A7+A8 WF-LINT-009..013 / MAP-LINT-009..011 / RA-LINT-001..002
    / PROV-LINT-005..007 (-13 cross-ref).
  E11.1 had earlier closed pom-020/033/040/051 (-4).
  31 residual: spec-side blocked (workspace-policy crystallization
  for ra-004/005/034/wfi-006/014/017) + compiler-completion-blocked
  (map-010/011/012, wfi-006-runtime-side) + needs-fixture-migration
  (assorted markers awaiting fixture authoring per the H-wave
  pattern).
  D-wave evolution: D7 split set baseline at 93; D8-A moved 17
  mis-classified schema-pending markers here (93 → 110); D8-B
  retired 47 of those (110 → 63, baseline set here) after audit
  confirmed an existing rule in the registry already covered the
  predicate end-to-end. Notable retirements: `SV-LINT-001/2/3/4`
  (source-vault `020/021/033/004`), `EFF-LINT-001/2/5` (effectiveness
  `002/003/021`), `COMP-LINT-001/2` (workspace `060/061`),
  `AI-LINT-002` (prov-074), `POM-LINT-001/7/8` (pom-004/039/041),
  `WF-LINT-001/4/5/6/8` (pom-030/031/036/037 + wfi-002/015),
  `MAP-LINT-001/4/8` (map-001/041/014), `SC-LINT-001/2` (scn-003/004),
  `EQ-LINT-003` / `ACC-LINT-001` / `JUR-LINT-001` (scn-040/041/042),
  `TERM-LINT-001/2` (term-003/010), `ID-LINT-001` (id-011),
  `CMP-LINT-010` (cmp-054), `PROV-LINT-002/4` (prov-011/013/014/020),
  `AI-LINT-003` (scn-005), `PUB-LINT-001/2` (rv-040/041/042 +
  wfi-021/022/023 + ra-040/050).
  E-wave evolution: E2 dropped 1 (README skip; 63 → 62);
  E3.2 reclassified 6 schema-pending → lint-pending (62 → 68);
  E6.3 added 1 (Akoma Ntoso SA-MUST-source-080; 68 → 69);
  E10 reclassified 4 runtime-pending → lint-pending (69 → 73);
  Akoma Ntoso net adjustment + WORKFLOW reclass nuances brought
  the final post-F3 baseline to 71.
- **Trigger:** every promoted rule (Draft → Tested → Stable) that
  retires a `lint-pending` marker decrements the count. Closes at 0.

### STUDIO-DEFER-004-SCHEMA — Schema-pending invariants (7 markers residual)

- **Origin:** D-wave 2026-05-03 split. D8-A burndown (2026-05-03)
  reduced this from 75 to 20 by per-marker verification:
  - 5 markers removed as DEAD (schema fully enforced today): `wfi-001`,
    `pom-002`, `pom-003`, `map-002`, plus the schema-part of `rv-032`.
  - 11 markers removed via ENCODABLE-NOW schema diffs (added `required`
    fields, `enum` constraints, `minLength`, conditional `if/then`):
    `source-022`, `source-040`, `eff-001`, `bind-020`, `map-020`,
    `map-022`, `prov-003`, `prov-022`, `prov-071`, `prov-080` (schema part),
    `rv-002`, `ra-022` (schema part), `ra-036`, `term-001`, `wfi-016`.
  - 39 markers reclassified as NOT-SCHEMA: 17 → `lint-pending` (cross-document
    refs, uniqueness, state-conditional cross-doc invariants), 22 →
    `runtime-pending` (append-only / write-barrier / temporal /
    no-schema-exists-yet for compile manifest + RuntimeObservation).
- **Rationale:** `(schema-pending)` markers track shape constraints
  the Studio schemas have a skeleton field for but don't fully
  enforce — required-field gaps, missing `uniqueItems`, missing
  enum tightenings, missing minLength constraints. The remaining 11
  cluster around: state-dependent required fields requiring
  reshape (`pom-001`, `pom-021`, `pom-032`, `map-003`, `id-010`,
  `rv-030`, `ra-021`); cardinality + contains constraints across
  array items (`ws-002` ≡ `ra-001`); compiler / runtime gates with
  no schema yet (`rtos-001`, `bind-031`, `bind-040`, `wfi-010`,
  `wfi-040`, `source-051`); cross-document Scenario refs
  (`scn-001`, `scn-002`, `scn-006`, `scn-043`); and the README
  documentary mention (1 ratchet false-positive).
- **Inventory anchor:** baseline 11 in
  `studio/tests/pending_ratchet.py::BASELINES["schema"]`.
- **Trigger:** schema rev that fully encodes a marker's MUST
  (e.g., conditional `if/then` on lifecycleState for state-dependent
  required fields, `contains` constraint with subschema for
  cardinality, RuntimeObservation schema landing in Phase 4)
  decrements the count. Closes at 0.

### STUDIO-DEFER-006 — Kernel-spec amendment for legal-hold clock-resume

- **Origin:** G-wave 2026-05-03 (G8); surfaced by ADR-0083 r2 review
  (A5 F-MAJOR-5).
- **Rationale:** ADR-0083 §2.4 commits Studio to delegating legal-hold
  semantics to kernel `holdType: legal-hold` per
  `specs/governance/workflow-governance.md` §7.15. §7.15 backs
  (1) "suspend disposal clock" and (2) "reject disposalAction execution
  with hold reference in rejection provenance". It does NOT pin
  clock-on-release behavior. Until pinned, runtime adapters MAY
  implement either policy.
- **Inventory anchor:** `specs/governance/workflow-governance.md`
  §7.15 (Legal Hold) — needs one-paragraph addition specifying
  clock-on-release.
- **Trigger:** kernel-spec PR amending §7.15 with chosen semantic.
- **Cross-repo dependency.** This is the first DEFER pointing at the
  kernel surface; any kernel-spec amendment is a parent-repo PR,
  not a Studio-only change.

### STUDIO-DEFER-007 — Substrate-pending invariants (191 markers)

- **Origin:** I-wave 2026-05-03 (Phase D5). Reclassification of the
  former STUDIO-DEFER-004-RUNTIME (190 reclassified markers) plus
  cmp-051 sharpened from fixture-pending → substrate-pending
  (cross-version comparison harness lands when v2 compiler exists).
- **Rationale:** `(substrate-pending)` markers track MUSTs whose
  enforcement requires Stage-7/8 substrate work that Studio cannot
  unblock alone:
  * **Audit log + write-barriers** (~24 markers, authoring-provenance):
    append-only invariants needing event-sourcing seam.
  * **Change-detection + SemanticDiff engines** (~18 markers,
    change-impact / source-vault): engines do not exist in
    `studio/crates/`.
  * **Scenario simulator emission** (~12 markers, scenario-authoring /
    runtime-observation-seam): Stage-6 simulator exists but does not
    yet emit ActualTrace / compute divergences / render reports.
  * **Runtime-observation adapter** (~8 markers, runtime-observation-
    seam): Phase-4 case-execution context + redaction.
  * **Trellis identity seam** (~9 markers, identity-and-attestation):
    parent Stage-8 integration for key rotation + revocation.
  * **Kernel clock-resume amendment** (1 marker; tracked separately
    as STUDIO-DEFER-006).
  * **Compiler-event-emission gates** (~25 markers, compiler-contract):
    determinism + manifest-completeness + lifecycle-event emission.
  * **Approval-state write-barriers** (~19 markers, review-and-approval):
    append-only / non-deletable comments + decisions.
  * **Workspace temporal/state-machine semantics** (~16 markers,
    workspace): ownership transfer + role-id non-reuse + retroactive-
    exemption + archived-workspace rejection.
  * **Other Stage-7/8 dependencies** (~59 markers, scattered).
- **Inventory anchor:** baseline 191 in
  `studio/tests/pending_ratchet.py::BASELINES["substrate"]`.
  D-wave evolution: original DEFER-004-RUNTIME peaked at 185
  (runtime-pending kind) before reclassification; I-wave Phase A
  added +5 from cluster sharpening (source-002, source-005,
  pom-040, wfi-004, source-051) before the bulk reclassification
  ran.
- **Trigger:** Stage-7 reference architecture spec landing +
  Stage-8 SNAP vertical slice through Restate adapter + Trellis
  export; each substrate component closes a marker cluster.

---

## Closed

### STUDIO-DEFER-004-RUNTIME — Runtime-pending invariants (CLOSED)

- **Closed:** 2026-05-03 in I-wave Phase D5.
- **Resolution:** every `(runtime-pending)` marker reclassified to
  `(substrate-pending)` under STUDIO-DEFER-007 (Stage-7/8 substrate
  dependency taxonomy). The reclassification reflects that these
  markers are not "deferred runtime authoring" but "deferred
  substrate work" — they need the actual substrate (audit log,
  change-detection engine, simulator emission, runtime-observation
  adapter, Trellis seam) to land before they can close.

### STUDIO-DEFER-004-FIXTURE — Fixture-pending invariants (CLOSED)

- **Closed:** 2026-05-03 in I-wave Phase D4.
- **Resolution:** the residual marker `cmp-051` (compiler version-
  bump semantic equality) was sharpened from `(fixture-pending)`
  to `(substrate-pending)` — the cross-version comparison harness
  lands when a v2 compiler exists to compare against. The marker
  rolls into STUDIO-DEFER-007.

### STUDIO-DEFER-004-COORDINATION — Cross-repo coordination markers (CLOSED)

- **Closed:** 2026-05-03 in I-wave Phase D3.
- **Resolution:** ADR-0084 (PLN-0381 identity attestation) and
  ADR-0085 (PLN-0384 event-types taxonomy) drafted at Status:
  Proposed. Each ADR pins a Studio-side placeholder shape that is
  a strict subset of the parent's expected primitive — so parent-
  team ratification swaps to a `$ref` with no breaking change.
  Markers `id-004` (per ADR-0084) and `prov-082` (per ADR-0085)
  closed against the Studio-side anchors, not against parent
  ratification. ADR ratification at the parent-team level is
  tracked via the ADRs themselves (Status: Proposed → Accepted)
  and does not block Studio-side closure.

### STUDIO-DEFER-005 — `retention_policy()` accessor returns `&Value`

- **Closed:** 2026-05-03 in E8 (post-G-wave).
- **Origin:** R-wave commit 5 of 5, 2026-05-02 (R7.4).
- **Resolution:** ADR-0083 r2 accepted (E8.0); spec amendment to
  `studio/specs/policy-object-model.md` adds the typed
  `RetentionPolicy` block under § Data Model and replaces the
  singular `retentionPeriod?` mention on EvidenceRequirement (E8.1);
  schema $def added in `wos-studio-policy-object.schema.json` and
  workspace bag tightened to `additionalProperties: $ref` in
  `wos-studio-workspace.schema.json` (E8.2); Rust struct landed at
  `studio/crates/wos-studio-model/src/policy.rs` with three enums
  + `shape_violations()` validator + 6 unit tests (E8.3); accessor
  promoted to `Option<Result<RetentionPolicy, serde_json::Error>>`
  with companion `legacy_retention_period()` for migration advisory
  (E8.3); `WF-LINT-006` migrated from presence-only to shape-aware
  validation with workspace-default resolution + 3 fixtures (E8.4);
  new `SA-WARN-pom-MIGRATE-RETENTION` advisory rule registered for
  the legacy `retentionPeriod` field (E8.4).
- **What's now enforced (chain):** spec → schema $def + allOf
  guards → typed Rust struct → shape_violations() → WF-LINT-006
  shape-aware error + SA-WARN-pom-MIGRATE-RETENTION advisory.

---

### STUDIO-DEFER-004-WORKFLOW — Workflow-pending policy decisions

- **Closed:** E-wave commit E7, 2026-05-03.
- **Resolution:** The sole workflow-pending marker
  (`policy-object-model.md::SA-MUST-pom-010`) tracked the unpinned
  AI confidence-threshold policy for ExtractedClaim auto-advance.
  Pinned `0.5` as default-with-override per parent
  `specs/ai/ai-integration.md` §S7 confidence-framework (which
  already specifies per-workflow `confidenceFloor` policies as
  configurable). WorkflowIntent authors MAY override via
  per-workflow `confidenceFloor`; default is `0.5` for the
  ExtractedClaim promotion gate.
- **Reclassification:** The marker is now
  `*(schema-pending: confidenceFloor field on ExtractedClaim.body.)* *(runtime-pending: promotion-gate enforcement.)*`
  — the policy decision is closed; the implementation now belongs
  to schema (add the field) + runtime (enforce the gate). Net:
  workflow 1 → 0 (sub-ID closed); schema 10 → 11; runtime 182 → 183.
- **Trigger:** none — the Studio team policy decision is documented.
  Future re-opens require a Studio team policy revision (e.g.,
  changing the default threshold, requiring per-tenant overrides).

### STUDIO-DEFER-002 — Lint-engine fixture suite externalization

- **Closed:** D-wave commits 7 + 8 + 9 of 9, 2026-05-03 (D3.1 +
  D3.2 + D3.3 + D3.4).
- **Resolution:** Added `load_workspace()` helper and a
  `fixtures/{s1_source_vault, s2_policy_object, s3_mapping,
  s4_workflow, s5_scenario, s6_publication, cross_cutting}/`
  directory tree to `studio/crates/wos-studio-lint/`. Externalized
  37 of 43 lint test fixtures across all six tiers. Each fixture is
  a JSON array of `[filename, doc]` pairs matching the existing
  `ws_from()` shape exactly; the loader feeds them through
  `Workspace::from_iter`. Test bodies shrink from ~30 lines of
  inline `json!({...})` to a single `load_workspace("path")` call
  + the existing assertions. Inventory ratchet at
  `tests/fixture_inventory_ratchet.rs` asserts every
  `load_workspace("...")` call site resolves to an existing
  fixture file.
- **Residual (6 inline tests, intentionally retained):**
  - `comp_lint_002_fires` / `comp_lint_002_silent` /
    `chain_lint_002_silent` / `eff_lint_005_fires` /
    `eff_lint_005_silent` — each computes its date string at
    runtime via `iso_date_offset_from(FROZEN_TODAY, ±N)` because
    the production rule uses `SystemTime::now()`. Externalizing
    would require either pinning to a static date (brittle: the
    test starts failing once `now()` drifts past the rule's
    window) or templating the loader (overengineered for 5
    sites). Kept inline; production rule logic is unchanged.
  - `fixture_pollution_sentinel_known_clean_fixtures_fire_exactly_one_rule`
    — the meta-test that asserts each fixture in its own
    list fires exactly one rule. The list IS its data; cannot
    be moved to a separate file without losing the readability
    that makes new entries safe to add.
- **Re-open trigger:** templating the fixture loader to handle
  runtime-relative dates (`{{today_plus:60}}` substitution at load
  time), enabling the 5 date-arithmetic tests to also externalize.
  Tracked as a follow-up but not part of DEFER-002.

### STUDIO-DEFER-001 — Typed-accessor sweep on residual `.raw` access

- **Closed:** D-wave commits 4 + 5 of 9, 2026-05-03 (D2.1 + D2.2).
- **Resolution:** Added `StudioDocument::body()` dispatch helper in
  `wos-studio-model/src/docs.rs`, plus seven `WorkspaceDocument`
  convenience accessors (`id()`, `kind()`, `lifecycle_state_str()`,
  `idp_role()`, `entries()`, `source_versions()`, `elements()`) in
  `wos-studio-lint/src/workspace.rs`. Migrated all 40 lint-rule sites
  in `workspace_rules.rs` to either typed accessors or
  `doc.document.body().get(...)`. Migrated 4 of 7 collection
  iterators in `workspace.rs` to the same shape.
- **Residual (8 sites, baseline lowered 47→8):** the three remaining
  `out.push((doc, &doc.raw))` fallbacks in `policy_object_records()` /
  `mapping_records()` / `scenario_records()` need the WHOLE document
  shape (`&Value` with the marker key) for downstream consumers; the
  typed `body()` map omits the marker. Annotated inline as
  legitimate infrastructure-tier `.raw` use. The remaining 5 byte
  matches are doc comments referencing `.raw` for documentation
  purposes (the byte-pattern walker can't distinguish code from
  prose). The ratchet at
  `studio/crates/wos-studio-lint/tests/raw_access_ratchet.rs` is now
  pinned at 8.
- **Re-open trigger:** the three `&doc.raw` fallbacks could be
  retired by changing `policy_object_records()` /
  `mapping_records()` / `scenario_records()` to return a Cow-wrapped
  Value (or by reshaping the schema so collection wrappers are
  required, eliminating the single-record fallback). Both options
  cost more than the residual is worth today.

### STUDIO-DEFER-003 Tranche A — Schema format-assertion mode

- **Closed:** D-wave commit 1 of 9, 2026-05-03 (D0).
- **Resolution:** Enabled `boon::Compiler::enable_format_assertions()`
  in `studio/crates/wos-studio-compiler/src/schema_validator.rs::compile()`.
  boon now treats `format` as a hard assertion (not annotation-only),
  so malformed `format: "uri"`, `format: "date-time"`, `format: "date"`,
  `format: "duration"`, and `format: "iri-reference"` values fail
  schema-pass. Sentinel test
  `schema_validator::tests::schema_pass_catches_malformed_url_format`
  asserts the new behavior. Lint-side URI/date/duration shape checks
  remain in place per the three-way agreement posture (overlapping
  coverage by design).

### STUDIO-DEFER-003 Tranche C — Actors uniqueness via array→object reshape

- **Closed:** D-wave commit 10 of 10, 2026-05-03 (lint-covered,
  no schema action).
- **Resolution:** JSON Schema Draft 2020-12 has no native
  "uniqueItems by property" keyword. The only schema-side encoding
  reshapes `actors: Array<Actor>` → `actors: Map<id, Actor>` so
  JSON object key uniqueness enforces the invariant. Audit
  measured ~225 consumer references across the workspace (typed
  `KernelDocument::actors: Vec<Actor>`, every `actors[i]` index,
  every `"actors": [...]` test fixture and sample workflow). The
  reshape would buy redundant coverage with parent-tier lint rule
  **K-009** at workspace-breaking cost; not worth the trade.
  Renamed the schema-pass sentinel to
  `phase7_gates.rs::schema_pass_silent_on_actor_id_collision_lint_catches_it`
  to match the layered-defense pattern from Tranche B (K-016).
  Studio cross-pass test
  `lint_pass_xref.rs::lint_catches_actor_id_collision` confirms
  K-009 fires end-to-end. With Tranches A, B, and C closed, all
  cross-property invariants tracked under DEFER-003 are resolved.
- **Re-open trigger:** if the workspace ships an actors-shape rev
  for unrelated reasons (e.g., a richer Actor envelope that
  benefits from object-keyed lookup), reshape the array→object at
  the same time and tighten the schema then. Until then, K-009 is
  the canonical catch.

### STUDIO-DEFER-003 Tranche B — `lifecycle.initialState ∈ lifecycle.states`

- **Closed:** D-wave commit 2 of 9, 2026-05-03 (D1).
- **Resolution:** JSON Schema Draft 2020-12 cannot natively express
  "value of property A must equal a key in object property B"; pursued
  the lint-fallback path. Added parent-tier rule **K-016** in
  `crates/wos-lint/src/rules/tier1.rs::check_initial_state_keys_into_states_typed()`
  and `crates/wos-lint/src/rules/registry.rs`. K-016 catches both
  top-level `lifecycle.initialState` and per-compound-state
  `state.initialState` against their respective scope of state keys.
  Sentinel tests at `crates/wos-lint/tests/tier1_rules.rs::k016_*`
  (4 cases: top-level / compound × known / unknown). Studio-side
  cross-check at
  `studio/crates/wos-studio-compiler/tests/lint_pass_xref.rs::
  lint_catches_unknown_initial_state` plus the schema-side silent-pass
  sentinel `phase7_gates.rs::
  schema_pass_silent_on_unknown_initial_state_lint_catches_it`.

### STUDIO-DEFER-VERIFIED-001 — `kernelVersion` envelope const not workflow `version`

- **Closed:** R-wave commit 5 of 5, 2026-05-02 (R7.1 cross-check).
- **Verified:** `studio/specs/scenario-authoring.md` and
  `studio/specs/compiler-contract.md` define the
  conformance-trace `kernelVersion` field as the WOS envelope marker
  (`$wosWorkflow` const, "1.0"), distinct from the workflow's own
  semver `version` field. Phase-4 emit
  (`studio/crates/wos-studio-compiler/src/phase4_emit.rs`) and the
  integration tests both reflect this. No spec amendment required.

---

## False positives (recorded so future review cycles don't re-flag)

### FP-R4-MAJ-3 — "Rule count grep showed 68 vs 70"

- **Reporter:** R4 cross-cutting review (2026-05-02).
- **Verdict:** False positive. The reviewer's regex
  (`'^\s*rule\s*\(' …`) missed `WFI-SHAPE-001` and similar
  non-LINT-suffixed registrations. Running the actual production
  count (`registry_carries_at_least_seventy_rules`) and the
  graduation distribution sentinel both confirm 70 rules. No code
  action.
