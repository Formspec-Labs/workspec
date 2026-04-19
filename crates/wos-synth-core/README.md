# wos-synth-core

Loop, abstractions, and prompt templates for WOS LLM authoring. The reference
implementation of [Claim A](../../POSITIONING.md) — that WOS is designed to be
authored, not just executed, by an LLM driven through schemas, BLUF specs, and
structured lint diagnostics.

## Boundary

`wos-synth-core` owns the loop, the `Prompter` trait, the `ToolContext` trait,
and the prompt templates. Provider crates (`wos-synth-anthropic`,
`wos-synth-mock`) depend on this crate, never the reverse. The benchmark crate
(`wos-bench`, planned) imports `wos-synth-core` as a library dependency; do not
fold them together, do not split the provider abstraction across two crates.

The dependency-inversion invariant is enforceable by:

```bash
cargo tree -p wos-synth-core --edges normal | grep -E 'reqwest|tokio|anthropic'
# expected: empty
```

## Extraction trigger

The four `wos-synth-*` crates graduate to a sibling repository when BOTH of
these are true:

1. The `Prompter` trait has survived one full release train without a breaking
   change.
2. A second production-quality provider implementation exists beyond
   `wos-synth-anthropic`.

Both conditions are observable; neither is calendar-based.

## Benchmark-regressions-do-not-motivate-spec-changes

Benchmark regressions do not motivate normative-spec changes unless the
benchmark is exercising a claim the spec actually makes. Spec PRs whose
motivation cites a benchmark failure must be reviewed against this rule.

## Layout

| File | Purpose |
| --- | --- |
| `src/prompter.rs` | `Prompter` trait + `CacheAnchor` + `Completion` types |
| `src/tool_context.rs` | `ToolContext` trait + `LintFinding` / `Severity` types |
| `src/tool_context/direct.rs` | `DirectToolContext` stopgap wrapping `wos-lint` |
| `src/prompts.rs` | `Layer` enum + generate/repair prompt builders |
| `src/synth_loop.rs` | Loop orchestrator |
| `src/trace.rs` | `SynthTrace` + `IterationRecord` |
| `src/errors.rs` | `SynthError` |

## See also

- `crates/wos-synth-anthropic` — Anthropic-API provider.
- `crates/wos-synth-mock` — deterministic mock provider for tests.
- `crates/wos-synth-cli` — `wos-synth` binary wiring providers + tools.
- [`thoughts/plans/2026-04-16-wos-synth-crate.md`](../../thoughts/plans/2026-04-16-wos-synth-crate.md) — full plan.
