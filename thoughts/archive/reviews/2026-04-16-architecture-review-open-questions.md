# Architecture Review — Open Questions for Maintainer

**Source:** [architecture-review-handoff.md §8](../archive/reviews/2026-04-16-architecture-review-handoff.md)
**Date raised:** 2026-04-16
**Reviewed:** 2026-04-17 — three independent agents (`wos-expert`, `spec-expert`, neutral solutions architect) answered each question. Synthesized recommendations below supersede the initial single-author recommendations, which are preserved for traceability.
**Revised:** 2026-04-17 — synthesis sharpened after parent-repo cross-check. Q3 scope narrowed (tags + Changesets, not Changesets alone — see ADR 0063 step 4); operational guardrails added to Q1/Q2/Q4; Q5 "verbatim" language corrected; Q6 added to resolve `wos-synth` ↔ benchmark ambiguity the plans leave open.
**Decisions recorded:** 2026-04-17 — all six questions resolved per closure protocol below. Chosen options marked ✅ in each Options list; rejected options struck through.
**Plan updates landed:** 2026-04-17 — §4.2, §4.4, §5.4, §5.5 plans all reflect the resolved decisions (promotion test + seeded load-bearing set; Changesets + per-stream tags mirroring ADR 0063; feature gate + extraction trigger + benchmark-causality guardrail; two-crate split with `wos-bench` depending on `wos-synth` as a library).
**Status:** Decisions made. Plan updates landed. **Ready to archive** — move this file to `thoughts/archive/reviews/` at the maintainer's convenience.

Each question below is presented with options, the original recommendation, and the synthesized multi-reviewer recommendation. Resolve by editing this document inline (strike through rejected options, mark the chosen one, link to the ADR if the decision warrants one).

---

## Q1. Is Claim A (LLM authoring) an accepted first-class goal?

The positioning work ([POSITIONING.md](../../POSITIONING.md), [README.md](../../README.md)) and [ADR 0064](../../../thoughts/adr/0064-wos-granularity-and-ai-native-positioning.md) already treat Claim A as first-class. But this question affects how aggressively we invest in it.

**Options:**

1. ✅ **First-class goal.** Ship [`wos-synth`](../plans/2026-04-16-wos-synth-crate.md) and the [benchmark](../plans/2026-04-16-wos-synthesis-benchmark.md) as load-bearing reference impls. Schema-description audit becomes mandatory. Trace-emitting conformance becomes mandatory. Plans §5.1 through §5.5 all graduate from "optional" to "on the roadmap." **— CHOSEN.**
2. ~~**Emergent property.**~~ Leave the framing in positioning docs but do not invest in a reference LLM-authoring harness. §5.4 and §5.5 become research sketches. §5.1 through §5.3 still happen but with lower urgency.
3. ~~**Defer.**~~ Keep the framing; revisit after a first external adopter of the kernel ships. §5.x plans all get held.

**Recommendation:** Option 1. The positioning is already written down; under-investing now makes it a broken promise. The cost is bounded (~1 engineer-quarter for a v0 that demonstrates the loop with one provider). The upside is a falsifiable differentiator — once the benchmark exists, "designed for LLMs" is no longer a claim, it is a metric.

**Synthesized (2026-04-17):** ✅ **Option 1.** Unanimous across all three reviewers. Reasoning across the three:

- *wos-expert:* 18 schemas + 3-tier verification are justified by Claim B alone only if you enjoy bloat; Claim A is what makes the granularity load-bearing.
- *spec-expert:* POSITIONING.md already declares schemas "load-bearing for this claim." A load-bearing claim without a reference impl is unfalsifiable.
- *solutions architect:* Positioning claims are either load-bearing or decorative. Keep the harness minimal (hundreds of LOC, not thousands). Risk: harness-driven design can bake today's model limitations into tomorrow's standard — keep the harness downstream of spec decisions.

**Action item:** To operationalize the solutions architect's caution (harness-driven spec drift), the [§5.4 `wos-synth` crate README](../plans/2026-04-16-wos-synth-crate.md) AND the [§5.5 benchmark `BENCHMARK.md`](../plans/2026-04-16-wos-synthesis-benchmark.md) MUST state: *benchmark regressions do not motivate normative-spec changes unless the benchmark is exercising a claim the spec actually makes.* The economic pressure flows one way by default (edit spec → benchmark passes); the policy is the counterweight. Review any spec PR whose motivation cites a benchmark failure against this rule.

**Blocks:** §5.4, §5.5 plans — those assume Option 1.

---

## Q2. Should `wos-synth` live in `work-spec/` or a sibling repo?

**Options:**

1. ~~**Inside `work-spec/crates/wos-synth/`.**~~ Tight coupling to the other crates. Share workspace, share CI, share versioning. The LLM-authoring harness ships with the spec; anyone who has the spec can try it. *(Rejected: no explicit extraction path means "extract later" becomes "extract never.")*
2. ~~**Sibling repo (`formspec/wos-synth/` or `formspec/wos-authoring/`).**~~ Clean separation. The spec stays free of an LLM-provider dependency in its Cargo.lock. Consumers who want the harness pull it separately. *(Rejected: cross-repo PRs slow iteration while provider trait, prompts, and trace format are still shifting.)*
3. ✅ **Start inside, graduate outside.** Develop in-tree while the seams are still fluid; extract when the API stabilizes. **— CHOSEN.** In-tree via `crates/wos-synth/` gated behind non-default `--features synth`; extraction trigger and CI enforcement per the Action item below.

**Recommendation:** Option 3. The first year of `wos-synth` development will iterate on the provider trait, prompt template shape, and trace format concurrently with changes to schemas, lint diagnostics, and conformance traces. Co-located development lets all four move together without cross-repo PRs. Once the shapes stabilize (schema-description audit done, diagnostic schema published, trace schema published), `wos-synth` is a thin orchestrator that extracts cleanly. At that point the provider-API surface no longer benefits from co-location.

**Synthesized (2026-04-17):** ✅ **Option 3 with an explicit extraction trigger.** Unanimous across all three reviewers. Reasoning:

- *wos-expert:* Existing crate layout already establishes that tooling-adjacent crates ship in-tree until their surface area hardens. Gate `wos-synth` behind a non-default feature (`--features synth`) so vendors pulling only `wos-core` / `wos-lint` never compile an LLM client.
- *spec-expert:* Parent formspec ADR 0063 solves cadence divergence with velocity tiers, not repo splits. Co-locating guarantees `wos-synth` breaks loudly when schemas change — which is the whole point of a reference harness.
- *solutions architect:* **"Extract later" commonly becomes "extract never."** Write down the extraction trigger now. Candidates: "when a second consumer wants the harness" or "at 1.0."

**Action item:** When implementing [§5.4 wos-synth plan](../plans/2026-04-16-wos-synth-crate.md) Task 1:

1. Gate provider dependencies behind `--features synth` in `crates/wos-synth/Cargo.toml`.
2. Ensure the default workspace CI job (`cargo build --workspace`) builds *without* `--features synth`, with a single additional CI job that runs *with* it. Without this, the feature gate is unenforced theatre — a default build that silently pulls an LLM client defeats the isolation purpose the gate exists for.
3. Write the extraction trigger into the crate README as a **single observable condition, two parts AND'd**: *the provider trait has survived one full release train without a breaking change AND a second production-quality provider implementation exists beyond the default.* Both parts are observable; neither is calendar-based. This replaces the ambiguous "when a second consumer wants it OR at 1.0" — a calendar trigger would ignore the actual signal (trait stability).

**2026-04-17 addendum:** The extraction-trigger language applies to `wos-synth-core` specifically (the loop crate), not to the monolithic `wos-synth` originally scoped. Under the DIP split recorded in [ADR 0065](../../../../thoughts/adr/0065-wos-authoring-stack-mirrors-formspec.md), `wos-synth-core` is the sibling-repo-extraction candidate; `wos-synth-anthropic`, `wos-synth-mock`, and `wos-synth-cli` are already independent crates and need no further extraction. The feature-gate enforcement step (step 2 above) is superseded by crate-boundary separation: provider deps live in separate crates, not behind a `--features synth` flag. The CI-guard job that was to verify feature-gate effectiveness is reinterpreted as a `cargo tree` check that `wos-synth-core`'s dep graph contains no LLM-client crates regardless of features.

**Blocks:** [§5.4 wos-synth plan](../plans/2026-04-16-wos-synth-crate.md) — Task 1 currently assumes Option 1/3 (in-tree). Change to sibling-repo would require revising that task.

---

## Q3. Release-versioning tool of choice — independent tags per layer, or a monorepo release manifest?

Context: the parent formspec repo already has [ADR 0063](../../../thoughts/adr/0063-release-trains-by-tier.md) for a similar split across npm packages using Changesets. The WOS question is whether to align on that tool or pick something else for the spec/schema/crate stack.

**Options:**

1. ✅ **Independent git tags** (`wos-kernel-v1.0.0`, `wos-governance-v1.0.0`, …). Matches Go module conventions. Plain git, no external tool. **— CHOSEN as the vendor-pinning artifact** (what `COMPAT.md` cells reference, what compliance docs cite). Used jointly with Option 2.
2. ✅ **Changesets** (align with parent repo). Adds a config overhead but matches the rest of Formspec's release tooling. Supports changelogs, version bumps, cross-package dep pinning. **— CHOSEN as the version-management tool** (`fixed` groups per stream, per-tier CHANGELOGs, cross-stream dep updates). Used jointly with Option 1, mirroring parent ADR 0063 steps 1–4 verbatim.
3. ~~**release-please**~~ (Google's). Generates CHANGELOGs + tags from Conventional Commits. Works well for monorepos. *(Rejected: second tool not aligned with parent-repo stack.)*
4. ~~**Custom script**~~ keyed to the stream → path mapping. Minimal dependency but another thing to maintain. *(Rejected: bespoke tool when Changesets already handles the shape.)*

**Recommendation:** Option 1 for the spec/schema stream (simple, predictable, vendors understand tags); Option 2 for the crate artifacts when we eventually publish them to crates.io (reuses Changesets investment from parent). The [release-trains plan](../plans/2026-04-16-wos-release-trains.md) is written generically and supports either.

**Synthesized (2026-04-17, revised):** ✅ **Adopted with narrower scope: Changesets for version management (with `fixed` groups per stream) PLUS per-stream git tags (`kernel-v…`, `governance-v…`, `ai-v…`, `advanced-v…`) for vendor pinning — mirroring ADR 0063 in full, including its step 4.** Split 2-1 among reviewers on framing, but parent-repo cross-check (see `/COMPAT.md` and [ADR 0063](../../../thoughts/adr/0063-release-trains-by-tier.md) step 4) shows the parent does NOT replace tags with Changesets — it uses both. The initial synthesis flattened "Changesets-everywhere" into a tool choice when the parent pattern is actually a two-artifact approach. Reasoning:

- *spec-expert (chose Changesets):* WOS's four streams are structurally identical to Formspec's four tiers. Reuse ADR 0063's reasoning, tooling, and contributor mental model. Copy its mitigation for `updateInternalDependencies` cascade.
- *solutions architect (chose Changesets):* Introducing a second tool creates two mental models, two failure modes, two places to learn. Changesets handles independent-versioning-within-a-monorepo explicitly. Budget ~1 day of glue to drive Rust crate releases (invoking `cargo publish`, updating `Cargo.toml` versions).
- *wos-expert (dissented, partially vindicated):* Advocated a split — tags for spec streams, Changesets for crates — on grounds that tags are vendor-compliance artifacts and Changesets is crate-focused. **Parent-repo cross-check shows ADR 0063 step 4 explicitly adopts per-stream tags (`kernel-v…` etc.) alongside Changesets.** The dissent tracked parent practice more closely than the initial synthesis credited. Final answer incorporates both: Changesets for versioning + per-stream tags for the vendor-pinning story `COMPAT.md` sells.

**Action item:** When implementing [§4.4 release-trains plan](../plans/2026-04-16-wos-release-trains.md):

1. Use Changesets with `fixed` groups per stream (kernel / governance / ai / advanced).
2. Budget ~1 day for Rust-publication glue (invoking `cargo publish`, updating `Cargo.toml` versions from Changesets-computed bumps).
3. Adopt per-stream git tags (`kernel-v1.0.0`, `governance-v1.0.0`, `ai-v0.1.0`, `advanced-v0.1.0`) as vendor-pinning artifacts. These are what `COMPAT.md` cells reference and what vendors cite in compliance docs. Matches ADR 0063 step 4 exactly.
4. Document the stream-mapping so maintainers know which stream a given change targets (spec markdown edits in `specs/kernel/` → kernel stream, etc.).

**Blocks:** [§4.4 release-trains plan](../plans/2026-04-16-wos-release-trains.md) — Task 4 details depend on the choice.

---

## Q4. Which rules today are `load-bearing` on the graduation ladder?

The [rule-coverage plan](../plans/2026-04-16-wos-rule-coverage-conformance.md) introduces a four-state ladder (`draft` / `tested` / `stable` / `load-bearing`). Task 2 of that plan backfills `fixtures` for rules that already have them. But "load-bearing" is a maintainer judgment — which rules, if removed, would break a reference impl or a fixture that ships as part of the spec?

**Decision — initial `load-bearing` set (2026-04-17):** Promotion test adopted as the mechanism (see Synthesized block below). Initial seeded set is the expanded union of all three reviewers' proposals, with K-012/K-017 explicitly held at `stable` pending the audit called out in the Action item. Strikethrough = rejected; ✅ = promoted on first pass; ⏸ = held pending audit.

- ✅ **K-023** (terminal-without-transition) — structural invariant; every kernel fixture exercises it.
- ✅ **K-030** (extension-prefix) — protects the seam that §4.1 work depends on.
- ✅ **G-037** / **G-042** / **G-043** (governance structural invariants relied on by due-process fixtures).
- ✅ **G-044** / **G-045** (delegation date ordering) — added per spec-expert; analogous to Formspec's E201 "duplicate path" family; fixture set under `governance/delegation-dates/` exercises both.
- ✅ **AI-024** (agent-reference condition) — required for AI fixtures to parse.
- ✅ **K-016** (mutation history append-only) / **K-020** (every mutation produces Facts provenance) / **K-047** (case relationships MUST NOT affect lifecycle evaluation) — added per wos-expert; provenance floor per Kernel §8.2; load-bearing for both Claim A authoring loop and Claim B agent runtime.
- ⏸ **K-012** / **K-017** (guard-path and cross-case-reference) — **held at `stable`** pending the K-012/K-017 audit called out in the Action item. Solutions architect's challenge ("some FEL guards are ergonomic rather than structural") is unresolved; promote only if the audit produces a named fixture that breaks for each. Ruthless-pre-1.0 principle applies.

**Recommendation:** Start with the five families above. Everything else starts at `tested` (if it has fixtures) or `draft` (if it does not). Promote to `stable` after three release trains without a test failure. Promote to `load-bearing` only by explicit ADR.

**Synthesized (2026-04-17):** ✅ **Adopt a promotion *test* rather than a hand-picked list.** The list writes itself once the test is in place. Reasoning:

- *spec-expert:* Copy the mechanical criterion from Formspec's `specs/lint-codes.json`: a rule is load-bearing only if it has (i) a normative `specRef`, (ii) an imperative `suggestedFix`, (iii) ≥1 conformance fixture, and (iv) removing it would permit a conformance-suite regression. By that test: K-023, K-012/017, G-037, G-042/043, AI-024, K-030 all qualify — **and G-044/045** (delegation date ordering, analogous to Formspec's E201 "duplicate path" family).
- *wos-expert:* Also add **K-016** (mutation history append-only), **K-020** (every mutation produces Facts provenance), **K-047** (case relationships MUST NOT affect lifecycle evaluation). These are the provenance floor (Kernel §8.2) — load-bearing for both Claim A's authoring loop and Claim B's agent runtime.
- *solutions architect:* Approximately right in count (~3% of 197 rules). Audit each candidate with the question "**name the fixture that breaks.**" Gently challenges whether *all* of K-012/017 qualify — some FEL guards are ergonomic rather than structural. Load-bearing creates a ratchet; pre-1.0 is the moment to be ruthless.

**Action item:** When implementing [§4.2 plan](../plans/2026-04-16-wos-rule-coverage-conformance.md) Task 2:

1. Encode the promotion test (specRef + suggestedFix + fixture + removal-breaks-conformance) as a gate before any rule can be marked `load-bearing`.
2. **Automate criterion (iv) in CI**: add a job that, for each `load-bearing` candidate, disables the rule, runs the conformance suite, and asserts at least one failure. Without automation, "removing it would permit a conformance-suite regression" collapses to judgment, and the ratchet is aspirational. The CI job is expensive (O(n × conformance-suite)) but runs on promotion only, not per-PR.
3. Run the promotion test against the union of all three reviewers' proposed lists (K-023, K-030, G-037, G-042, G-043, AI-024, K-012, K-017, G-044, G-045, K-016, K-020, K-047).
4. **Explicitly audit K-012 and K-017** against the solutions architect's challenge that "some FEL guards are ergonomic rather than structural." Name the fixture that breaks for each, or downgrade to `stable`. Do not leave this unresolved — load-bearing is a ratchet, and pre-1.0 is the last moment to be ruthless about what earns the tag.

**Blocks:** [§4.2 plan](../plans/2026-04-16-wos-rule-coverage-conformance.md) Task 2 — needs the initial load-bearing set to seed the ladder.

---

## Q5. Existing compat-matrix convention to adopt, or design one?

**Options:**

1. ✅ **Adopt SemVer + caret ranges** (`wos-ai@^0.5 requires wos-kernel@^1.0`). Familiar to package-ecosystem consumers. Expressive enough for the four-stream case. **— CHOSEN as the cell syntax** inside the hand-authored matrix. Used jointly with Option 3.
2. ~~**Adopt the Go module pattern**~~ — explicit version-suffixed imports (`wos-ai/v1`, `wos-kernel/v1`). Heavier but unambiguous across a decade-long kernel. *(Rejected: overkill for a spec document; not aligned with parent-repo `/COMPAT.md` convention.)*
3. ✅ **Design a custom matrix table** (rows = kernel versions, columns = governance/ai/advanced ranges). Most explicit, but requires tooling to consume. **— CHOSEN as the document structure** (hand-authored, separate tables per artifact type), matching parent `/COMPAT.md` format conventions. Used jointly with Option 1 for cell contents.

**Recommendation:** Option 1 for human-readable compatibility statements (`wos-ai@^0.5 requires wos-kernel@^1.0`) combined with Option 3 as a formal table in `COMPATIBILITY-MATRIX.md` — the matrix is explicit, the caret ranges are a shorthand for vendors. Option 2 overkill for a spec document. A simple JSON file (`compat.json`) can be the machine-readable source that both the matrix and CI consume.

**Synthesized (2026-04-17):** ✅ **Adopt the parent repo's `COMPAT.md` pattern (hand-authored matrix with caret ranges in cells) PLUS a CI staleness check.** All three reviewers use both a matrix and caret ranges; they split on which is normative. Parent-repo consistency settles it. Reasoning:

- *spec-expert (chose this):* Parent formspec already has [`/Users/mikewolfd/Work/formspec/COMPAT.md`](../../../../COMPAT.md) as a hand-authored table with caret/x-range notation. It's load-bearing for the "vendors can pin here" story ADR 0063 sells. Zero learning cost — copy the pattern verbatim.
- *wos-expert:* Proposed `compat.json` as machine-readable source with a rendered matrix + caret prose. Compatible with the chosen pattern — `compat.json` can exist as the generator input for `COMPAT.md` without contradicting the hand-authored posture.
- *solutions architect:* Argued for caret ranges in manifests as source-of-truth, matrix as derived artifact. **Overridden** by parent-repo consistency, with the risk (matrix drift) addressed by the staleness-check action item below.

**Action item:** When implementing [§4.4 release-trains plan](../plans/2026-04-16-wos-release-trains.md) Task 3:

1. Create `work-spec/COMPAT.md` adopting the parent's **format conventions** (caret ranges in cells, hand-authored, separate tables per artifact type, per-stream sections with cadence + semver discipline) but **adapting the matrix shape** to WOS's kernel-centric compat story. Parent `/COMPAT.md` is a package-dependency matrix across npm + Rust + Python (three tables, many-to-many pin relationships); WOS has one kernel + three streams pinning against it. Literal verbatim copy would contort the shape — adopt the conventions, adapt the shape.
2. Use **separate tables per artifact type**: one for spec-stream documents (kernel / governance / ai / advanced as markdown), one for crate artifacts (`wos-core`, `wos-lint`, `wos-conformance`, `wos-synth`), mirroring parent's three-table layout even though WOS only has two artifact types today.
3. Add a CI check analogous to `npm run docs:check` that fails if a stream version declared in `COMPAT.md` doesn't match the current manifest (Changesets output, stream CHANGELOGs, or the latest per-stream tag). **Parent `/COMPAT.md` doesn't have this check today** — adding it to WOS would put it ahead; consider back-porting to parent once proven.
4. Leave `compat.json` as an optional future optimization if hand-authoring becomes unsustainable.

**Blocks:** [§4.4 release-trains plan](../plans/2026-04-16-wos-release-trains.md) Task 3 — the exact syntax of the matrix.

---

## Q6. Is `wos-synth` (§5.4) and the authoring benchmark (§5.5) one project or two?

Added 2026-04-17 during review revision. The plans live side-by-side but leave the relationship implicit — the [benchmark plan](../plans/2026-04-16-wos-synthesis-benchmark.md) literally says "benchmark runner as a binary in `wos-synth` or a new `wos-bench` crate." That ambiguity will harden into whichever option the first implementer picks unless decided explicitly. It matters because the answer determines ownership of the provider trait, prompt-template shape, and trace schema.

**Options:**

1. ~~**One project.**~~ `wos-synth` IS the benchmark harness — the benchmark is a CI job that invokes `wos-synth` against a fixture set. One crate, one provider trait, one prompt-template shape. Benchmark additions are tests or fixtures inside the `wos-synth` crate. *(Rejected: folds measurement infrastructure (stats, scoring, regression tracking) into the authoring demo; the two loops have genuinely different shapes.)*
2. ✅ **Two projects sharing primitives.** `wos-synth` owns the authoring loop (single-spec generation, provider trait, prompt templates, trace types). A separate `wos-bench` crate (or `crates/wos-synth/benches/` Cargo bench) imports `wos-synth` as a library and adds multi-spec evaluation, scoring, regression tracking, and the `benchmark-runs/*.json` output. Clear separation; shared primitives prevent drift. **— CHOSEN.** Implementation is a new `crates/wos-bench/` crate depending on `wos-synth` as a library.
3. ~~**Two projects, independent.**~~ Each has its own provider abstraction. Maximum duplication. *(Rejected as obvious waste — flagged only for completeness.)*

**Decision (2026-04-17, added during revision): ✅ Option 2.** The authoring loop (generate one spec, check it lints, iterate to convergence) and the benchmark loop (run N fixtures, aggregate outcomes, score against reference) have genuinely different shapes — one is a demo, the other is a measurement. Sharing the provider trait + prompt templates + trace types keeps provider changes in one place; keeping the benchmark separate prevents the demo from growing a stats/scoring layer it doesn't need. The benchmark plan's "or a new `wos-bench` crate" framing already leans this direction — this decision resolves the "or" as "bench crate."

**Action item:** When implementing §5.4 and §5.5:

1. Establish the provider trait (`ProviderHandle`), prompt-template types, trace types, and outcome enum in `wos-synth` as the library API.
2. Implement the benchmark runner as `crates/wos-bench/` (new crate, Tier: tooling, non-default feature if Cargo workspace member), importing `wos-synth` as a library dependency. Binary target is `wos-bench`.
3. Document the boundary in both crate READMEs: `wos-synth` README states "owns the provider abstraction and prompt primitives — benchmark crate imports these"; `wos-bench` README states "consumes `wos-synth` as a library; owns fixture sets, scoring, regression tracking." Future contributors should not fold them together or split the provider abstraction across two crates.
4. Share the trace format: `wos-bench` records the same trace shape that `wos-synth generate --trace` emits — one format for both the demo and the measurement.

**2026-04-17 addendum:** The "two crates sharing primitives" framing holds, but the primitives now live in a different location than originally stated. Under [ADR 0065](../../../../thoughts/adr/0065-wos-authoring-stack-mirrors-formspec.md):

- `wos-synth-core` owns the **loop primitives**: `Prompter` trait, `ToolContext` trait, prompt templates, trace types, outcome enum.
- `wos-mcp` owns the **tool-handler primitives**: 20+ tools over `wos-authoring` with dual entry (MCP stdio + in-process dispatch).
- `wos-bench` depends on `wos-synth-core` (for the loop) + one `Prompter` provider + (transitively) `wos-mcp` via `ToolContext`'s production implementation.

The original Q6 decision said `wos-bench` depends on `wos-synth`; revised: `wos-bench` depends on `wos-synth-core` + `wos-synth-mock` (or `-anthropic`) + `wos-mcp`. Shared primitives still prevent drift — just across three crate boundaries instead of two.

**Blocks:** [§5.4 wos-synth plan](../plans/2026-04-16-wos-synth-crate.md) Task 1 (crate layout decision) and [§5.5 benchmark plan](../plans/2026-04-16-wos-synthesis-benchmark.md) Task 1 (crate placement decision) — both plans currently leave this as "or."

---

## How to close this document

When a question is resolved:

1. Edit the **Options** list to mark the chosen option (bold, prefix with ✅).
2. If the decision warrants a persistent record, link to an ADR under `thoughts/adr/` or `../../../thoughts/adr/`.
3. Update the affected plan in `thoughts/plans/2026-04-16-*` if the decision changes a task.
4. When all six are resolved, archive this file to `thoughts/archive/reviews/`.
