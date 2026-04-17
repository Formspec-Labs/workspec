# wos-synth

Reference LLM-authoring harness for WOS (Workflow Orchestration Standard)
documents. Given a natural-language problem statement and a target layer
(kernel / governance / ai), it drives a loop:

1. generate a candidate WOS document via an LLM provider,
2. run `wos-lint` for structural diagnostics,
3. run `wos-conformance` for behavioural diagnostics,
4. assemble a targeted repair prompt and iterate,
5. stop when diagnostics are empty or the iteration cap is reached.

At Task 1 of the implementation plan, this crate is a scaffold only. The
provider trait, prompt templates, loop orchestrator, and CLI land in
subsequent tasks. What is already load-bearing, and will stay so, is the
policy surface below.

## Feature gate: `synth`

Provider dependencies (`reqwest`, `tokio`, `anthropic-sdk`) are gated behind
the non-default `synth` Cargo feature:

- `cargo build --workspace` (default) compiles `wos-synth` as a library of
  public types with **no** LLM client in the dependency graph. Verify with
  `cargo tree --prefix none | grep -E '^reqwest|^tokio|^anthropic-sdk'`,
  which must return nothing.
- `cargo build -p wos-synth --features synth` pulls the provider stack and
  is exercised by a dedicated CI job.

The gate is enforced rather than theatre: a default build that silently
pulled an LLM client would defeat the purpose. If you find yourself wanting
to add an unconditional provider dep, stop and add it under
`[dependencies]` behind `optional = true` plus the `synth` feature list.

## Boundary with `wos-bench`

This crate owns the provider abstraction and prompt primitives. The
benchmark crate (`wos-bench`) imports these as a library dependency; do
not fold them together, do not split the provider abstraction across two
crates.

In practice: the `LlmProvider` trait, `CacheAnchor`, prompt templates, and
`SynthTrace` live here. Scoring, fixture selection, and statistical
aggregation live in `wos-bench`. Crossing that line (e.g. teaching
`wos-synth` about scoring, or teaching `wos-bench` about prompt assembly)
is the architecture smell that this boundary exists to prevent.

## Extraction trigger

`wos-synth` graduates to a sibling repository when **both** of these are
true:

1. the provider trait has survived one full release train without a
   breaking change, **and**
2. a second production-quality provider implementation exists beyond the
   default.

Both parts are observable; neither is calendar-based. Until both
conditions hold, the crate stays here alongside `wos-core`, `wos-lint`,
and `wos-conformance`.

## Benchmark regressions do not motivate spec changes

Benchmark regressions do not motivate normative-spec changes unless the
benchmark is exercising a claim the spec actually makes. Spec PRs whose
motivation cites a benchmark failure must be reviewed against this rule.

Concretely: if `wos-bench` shows convergence dropping because the LLM
generates a field the spec does not require, the fix is in the prompt or
the harness, not in the schema. The spec is the contract; the benchmark
measures how well today's models produce documents that honour that
contract. Those are different axes.

## Status

| Task | Scope | State |
|------|-------|-------|
| 1 | Scaffold + `synth` feature gate + README policies + CI matrix | in progress |
| 2 | `LlmProvider` trait + `MockProvider` | pending |
| 3 | Generate / repair prompt templates | pending |
| 4 | Loop orchestrator + trace | pending |
| 5 | `AnthropicProvider` with prompt caching | pending |
| 6 | CLI (`generate`, `explain`, `dry-run`) | pending |
| 7 | `synth-trace.schema.json` | pending |

See `thoughts/plans/2026-04-16-wos-synth-crate.md` for the full plan.
