# Architecture Review — Open Questions for Maintainer

**Source:** [architecture-review-handoff.md §8](../archive/reviews/2026-04-16-architecture-review-handoff.md)
**Date raised:** 2026-04-16
**Reviewed:** 2026-04-17 — three independent agents (`wos-expert`, `spec-expert`, neutral solutions architect) answered each question. Synthesized recommendations below supersede the initial single-author recommendations, which are preserved for traceability.
**Status:** Awaiting maintainer decision. Each question blocks or colors one or more plans in `thoughts/plans/2026-04-16-*`.

Each question below is presented with options, the original recommendation, and the synthesized multi-reviewer recommendation. Resolve by editing this document inline (strike through rejected options, mark the chosen one, link to the ADR if the decision warrants one).

---

## Q1. Is Claim A (LLM authoring) an accepted first-class goal?

The positioning work ([POSITIONING.md](../../POSITIONING.md), [README.md](../../README.md)) and [ADR 0064](../../../thoughts/adr/0064-wos-granularity-and-ai-native-positioning.md) already treat Claim A as first-class. But this question affects how aggressively we invest in it.

**Options:**

1. **First-class goal.** Ship [`wos-synth`](../plans/2026-04-16-wos-synth-crate.md) and the [benchmark](../plans/2026-04-16-wos-synthesis-benchmark.md) as load-bearing reference impls. Schema-description audit becomes mandatory. Trace-emitting conformance becomes mandatory. Plans §5.1 through §5.5 all graduate from "optional" to "on the roadmap."
2. **Emergent property.** Leave the framing in positioning docs but do not invest in a reference LLM-authoring harness. §5.4 and §5.5 become research sketches. §5.1 through §5.3 still happen but with lower urgency.
3. **Defer.** Keep the framing; revisit after a first external adopter of the kernel ships. §5.x plans all get held.

**Recommendation:** Option 1. The positioning is already written down; under-investing now makes it a broken promise. The cost is bounded (~1 engineer-quarter for a v0 that demonstrates the loop with one provider). The upside is a falsifiable differentiator — once the benchmark exists, "designed for LLMs" is no longer a claim, it is a metric.

**Synthesized (2026-04-17):** ✅ **Option 1.** Unanimous across all three reviewers. Reasoning across the three:
- *wos-expert:* 18 schemas + 3-tier verification are justified by Claim B alone only if you enjoy bloat; Claim A is what makes the granularity load-bearing.
- *spec-expert:* POSITIONING.md already declares schemas "load-bearing for this claim." A load-bearing claim without a reference impl is unfalsifiable.
- *solutions architect:* Positioning claims are either load-bearing or decorative. Keep the harness minimal (hundreds of LOC, not thousands). Risk: harness-driven design can bake today's model limitations into tomorrow's standard — keep the harness downstream of spec decisions.

**Blocks:** §5.4, §5.5 plans — those assume Option 1.

---

## Q2. Should `wos-synth` live in `wos-spec/` or a sibling repo?

**Options:**

1. **Inside `wos-spec/crates/wos-synth/`.** Tight coupling to the other crates. Share workspace, share CI, share versioning. The LLM-authoring harness ships with the spec; anyone who has the spec can try it.
2. **Sibling repo (`formspec/wos-synth/` or `formspec/wos-authoring/`).** Clean separation. The spec stays free of an LLM-provider dependency in its Cargo.lock. Consumers who want the harness pull it separately.
3. **Start inside, graduate outside.** Develop in-tree while the seams are still fluid; extract when the API stabilizes.

**Recommendation:** Option 3. The first year of `wos-synth` development will iterate on the provider trait, prompt template shape, and trace format concurrently with changes to schemas, lint diagnostics, and conformance traces. Co-located development lets all four move together without cross-repo PRs. Once the shapes stabilize (schema-description audit done, diagnostic schema published, trace schema published), `wos-synth` is a thin orchestrator that extracts cleanly. At that point the provider-API surface no longer benefits from co-location.

**Synthesized (2026-04-17):** ✅ **Option 3 with an explicit extraction trigger.** Unanimous across all three reviewers. Reasoning:
- *wos-expert:* Existing crate layout already establishes that tooling-adjacent crates ship in-tree until their surface area hardens. Gate `wos-synth` behind a non-default feature (`--features synth`) so vendors pulling only `wos-core` / `wos-lint` never compile an LLM client.
- *spec-expert:* Parent formspec ADR 0063 solves cadence divergence with velocity tiers, not repo splits. Co-locating guarantees `wos-synth` breaks loudly when schemas change — which is the whole point of a reference harness.
- *solutions architect:* **"Extract later" commonly becomes "extract never."** Write down the extraction trigger now. Candidates: "when a second consumer wants the harness" or "at 1.0."

**Action item:** When implementing [§5.4 wos-synth plan](../plans/2026-04-16-wos-synth-crate.md) Task 1, add (a) a `--features synth` gate on the provider dependencies so the default workspace build does not pull an LLM client, and (b) an explicit extraction-trigger line in the crate README.

**Blocks:** [§5.4 wos-synth plan](../plans/2026-04-16-wos-synth-crate.md) — Task 1 currently assumes Option 1/3 (in-tree). Change to sibling-repo would require revising that task.

---

## Q3. Release-versioning tool of choice — independent tags per layer, or a monorepo release manifest?

Context: the parent formspec repo already has [ADR 0063](../../../thoughts/adr/0063-release-trains-by-tier.md) for a similar split across npm packages using Changesets. The WOS question is whether to align on that tool or pick something else for the spec/schema/crate stack.

**Options:**

1. **Independent git tags** (`wos-kernel-v1.0.0`, `wos-governance-v1.0.0`, …). Matches Go module conventions. Plain git, no external tool.
2. **Changesets** (align with parent repo). Adds a config overhead but matches the rest of Formspec's release tooling. Supports changelogs, version bumps, cross-package dep pinning.
3. **release-please** (Google's). Generates CHANGELOGs + tags from Conventional Commits. Works well for monorepos.
4. **Custom script** keyed to the stream → path mapping. Minimal dependency but another thing to maintain.

**Recommendation:** Option 1 for the spec/schema stream (simple, predictable, vendors understand tags); Option 2 for the crate artifacts when we eventually publish them to crates.io (reuses Changesets investment from parent). The [release-trains plan](../plans/2026-04-16-wos-release-trains.md) is written generically and supports either.

**Synthesized (2026-04-17):** ✅ **Option 2 — Changesets everywhere, with `fixed` groups per stream, mirroring ADR 0063.** Split 2-1 among reviewers; the Changesets-everywhere position wins on consistency grounds. Reasoning:
- *spec-expert (chose this):* WOS's four streams (kernel / governance / ai / advanced) are structurally identical to Formspec's four tiers (kernel / foundation / integration / AI). Same problem, same shape of solution. Reuse ADR 0063's reasoning, tooling, and contributor mental model. Copy ADR 0063's mitigation for `updateInternalDependencies` cascade.
- *solutions architect (chose this):* Introducing a second tool creates two mental models, two failure modes, two places to learn. Changesets handles independent-versioning-within-a-monorepo explicitly. Budget ~1 day of glue to drive Rust crate releases (invoking `cargo publish`, updating `Cargo.toml` versions).
- *wos-expert (dissented):* Advocated a split — tags for spec streams, Changesets for crates — on grounds that tags are vendor-compliance artifacts and Changesets is crate-focused. **Overridden** by the consistency argument from the other two.

**Action item:** When implementing [§4.4 release-trains plan](../plans/2026-04-16-wos-release-trains.md), use Changesets with `fixed` groups per stream. Budget ~1 day for Rust-publication glue. Document the mapping so maintainers know which stream a given change targets.

**Blocks:** [§4.4 release-trains plan](../plans/2026-04-16-wos-release-trains.md) — Task 4 details depend on the choice.

---

## Q4. Which rules today are `load-bearing` on the graduation ladder?

The [rule-coverage plan](../plans/2026-04-16-wos-rule-coverage-conformance.md) introduces a four-state ladder (`draft` / `tested` / `stable` / `load-bearing`). Task 2 of that plan backfills `fixtures` for rules that already have them. But "load-bearing" is a maintainer judgment — which rules, if removed, would break a reference impl or a fixture that ships as part of the spec?

**Options:** Tag the following as load-bearing on first pass. The list is a proposal — strike or add as appropriate:

- **K-023** (terminal-without-transition) — structural invariant; every kernel fixture exercises it.
- **K-030** (extension-prefix) — protects the seam that §4.1 work depends on.
- **G-037** / G-042 / G-043 (governance structural invariants relied on by due-process fixtures).
- **AI-024** (agent-reference condition) — required for AI fixtures to parse.
- **K-012** / K-017 (guard-path and cross-case-reference) — exercised by every T3 scenario.

**Recommendation:** Start with the five families above. Everything else starts at `tested` (if it has fixtures) or `draft` (if it does not). Promote to `stable` after three release trains without a test failure. Promote to `load-bearing` only by explicit ADR.

**Synthesized (2026-04-17):** ✅ **Adopt a promotion *test* rather than a hand-picked list.** The list writes itself once the test is in place. Reasoning:
- *spec-expert:* Copy the mechanical criterion from Formspec's `specs/lint-codes.json`: a rule is load-bearing only if it has (i) a normative `specRef`, (ii) an imperative `suggestedFix`, (iii) ≥1 conformance fixture, and (iv) removing it would permit a conformance-suite regression. By that test: K-023, K-012/017, G-037, G-042/043, AI-024, K-030 all qualify — **and G-044/045** (delegation date ordering, analogous to Formspec's E201 "duplicate path" family).
- *wos-expert:* Also add **K-016** (mutation history append-only), **K-020** (every mutation produces Facts provenance), **K-047** (case relationships MUST NOT affect lifecycle evaluation). These are the provenance floor (Kernel §8.2) — load-bearing for both Claim A's authoring loop and Claim B's agent runtime.
- *solutions architect:* Approximately right in count (~3% of 197 rules). Audit each candidate with the question "**name the fixture that breaks.**" Gently challenges whether *all* of K-012/017 qualify — some FEL guards are ergonomic rather than structural. Load-bearing creates a ratchet; pre-1.0 is the moment to be ruthless.

**Action item:** When implementing [§4.2 plan](../plans/2026-04-16-wos-rule-coverage-conformance.md) Task 2, (a) encode the promotion test (specRef + suggestedFix + fixture + removal-breaks-conformance) as a gate before any rule can be marked `load-bearing`, (b) run it against the union of all three reviewers' proposed lists, (c) audit each survivor against "name the fixture that breaks" before promoting.

**Blocks:** [§4.2 plan](../plans/2026-04-16-wos-rule-coverage-conformance.md) Task 2 — needs the initial load-bearing set to seed the ladder.

---

## Q5. Existing compat-matrix convention to adopt, or design one?

**Options:**

1. **Adopt SemVer + caret ranges** (`wos-ai@^0.5 requires wos-kernel@^1.0`). Familiar to package-ecosystem consumers. Expressive enough for the four-stream case.
2. **Adopt the Go module pattern** — explicit version-suffixed imports (`wos-ai/v1`, `wos-kernel/v1`). Heavier but unambiguous across a decade-long kernel.
3. **Design a custom matrix table** (rows = kernel versions, columns = governance/ai/advanced ranges). Most explicit, but requires tooling to consume.

**Recommendation:** Option 1 for human-readable compatibility statements (`wos-ai@^0.5 requires wos-kernel@^1.0`) combined with Option 3 as a formal table in `COMPATIBILITY-MATRIX.md` — the matrix is explicit, the caret ranges are a shorthand for vendors. Option 2 overkill for a spec document. A simple JSON file (`compat.json`) can be the machine-readable source that both the matrix and CI consume.

**Synthesized (2026-04-17):** ✅ **Adopt the parent repo's `COMPAT.md` pattern (hand-authored matrix with caret ranges in cells) PLUS a CI staleness check.** All three reviewers use both a matrix and caret ranges; they split on which is normative. Parent-repo consistency settles it. Reasoning:
- *spec-expert (chose this):* Parent formspec already has [`/Users/mikewolfd/Work/formspec/COMPAT.md`](../../../../COMPAT.md) as a hand-authored table with caret/x-range notation. It's load-bearing for the "vendors can pin here" story ADR 0063 sells. Zero learning cost — copy the pattern verbatim.
- *wos-expert:* Proposed `compat.json` as machine-readable source with a rendered matrix + caret prose. Compatible with the chosen pattern — `compat.json` can exist as the generator input for `COMPAT.md` without contradicting the hand-authored posture.
- *solutions architect:* Argued for caret ranges in manifests as source-of-truth, matrix as derived artifact. **Overridden** by parent-repo consistency, with the risk (matrix drift) addressed by the staleness-check action item below.

**Action item:** When implementing [§4.4 release-trains plan](../plans/2026-04-16-wos-release-trains.md) Task 3, (a) create `wos-spec/COMPAT.md` following the parent `/COMPAT.md` structure verbatim; (b) add a CI check analogous to `npm run docs:check` that fails if a stream version in `COMPAT.md` doesn't match the current package/stream manifest; (c) leave `compat.json` as an optional future optimization if hand-authoring becomes unsustainable.

**Blocks:** [§4.4 release-trains plan](../plans/2026-04-16-wos-release-trains.md) Task 3 — the exact syntax of the matrix.

---

## How to close this document

When a question is resolved:

1. Edit the **Options** list to mark the chosen option (bold, prefix with ✅).
2. If the decision warrants a persistent record, link to an ADR under `thoughts/adr/` or `../../../thoughts/adr/`.
3. Update the affected plan in `thoughts/plans/2026-04-16-*` if the decision changes a task.
4. When all five are resolved, archive this file to `thoughts/archive/reviews/`.
