# WOS Synth Crate — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `wos-synth-*` family of crates — the reference LLM-authoring harness — split across four crates: `wos-synth-core` (loop + traits, zero provider deps), `wos-synth-anthropic` (one production provider), `wos-synth-mock` (deterministic test/bench provider), and `wos-synth-cli` (binary). Together they replace the earlier monolithic `wos-synth` design. `wos-synth-core` is a library the [`wos-bench` crate](./2026-04-16-wos-synthesis-benchmark.md) also depends on; do not fold loop logic into the CLI or provider crates.

**Architecture:** The loop (`SynthLoop`) lives in `wos-synth-core` and depends only on two injected traits: `Prompter` (LLM call abstraction) and `ToolContext` (lint + conformance dispatch abstraction). No network client, no Anthropic SDK, no `reqwest`, no `tokio` appear anywhere in `wos-synth-core`'s dependency graph — verify with `cargo tree`. This pattern mirrors `packages/formspec-chat` in the parent project: `AIAdapter` → `Prompter`, `ChatSession` → `SynthLoop`, `ToolContext` → same name. Tool handlers (document creation, lint, conformance invocation) are being moved to a new `wos-mcp` crate (see [2026-04-17-wos-mcp-crate.md](./2026-04-17-wos-mcp-crate.md)). Until `wos-mcp` is stable, a stopgap `ToolContext` impl lives in `wos-synth-core/src/stopgap.rs` and calls `wos-lint`/`wos-conformance` directly. The stopgap is removed once `wos-synth-cli` wires `ToolContext` to `wos-mcp`'s in-process dispatch.

Intent-driven authoring helpers (mirroring `packages/formspec-studio-core/src/project.ts`) live in the `wos-authoring` crate (see [2026-04-17-wos-authoring-crate.md](./2026-04-17-wos-authoring-crate.md)). `wos-synth-core` consumes `wos-authoring` for document mutation but does not own authoring logic directly.

**Tech Stack:** Rust (`wos-synth-core`, `wos-synth-anthropic`, `wos-synth-mock`, `wos-synth-cli` crates). `anthropic-sdk` v0.1.5 in `wos-synth-anthropic` only. `reqwest` and `tokio` in `wos-synth-anthropic` only. Because providers are separate crates, the old `--features synth` feature gate is obsoleted: crate-level separation gives cleaner distribution boundaries than feature flags. Default workspace builds compile cleanly with zero provider deps by simply not depending on `wos-synth-anthropic`.

**Spec anchor:** [architecture-review-handoff.md §5.4](../archive/reviews/2026-04-16-architecture-review-handoff.md) — extraction trigger for `wos-synth-core` specifically (not the whole family) resolved in [open questions Q2](../archive/reviews/2026-04-16-architecture-review-open-questions.md#q2-should-wos-synth-live-in-wos-spec-or-a-sibling-repo). Multi-crate split (synth-core + wos-mcp + wos-bench) resolved as an evolution of [open questions Q6](../archive/reviews/2026-04-16-architecture-review-open-questions.md#q6-is-wos-synth-54-and-the-authoring-benchmark-55-one-project-or-two). Claim A first-class status: [open questions Q1](../archive/reviews/2026-04-16-architecture-review-open-questions.md#q1-is-claim-a-llm-authoring-an-accepted-first-class-goal).

**Related plans:**
- [2026-04-17-wos-mcp-crate.md](./2026-04-17-wos-mcp-crate.md) — `wos-mcp` owns MCP stdio + in-process tool dispatch; `wos-synth-core`'s `ToolContext` will delegate to it once stable.
- [2026-04-17-wos-authoring-crate.md](./2026-04-17-wos-authoring-crate.md) — `wos-authoring` owns intent-driven authoring helpers; `wos-synth-core` consumes it.
- [2026-04-16-wos-synthesis-benchmark.md](./2026-04-16-wos-synthesis-benchmark.md) — `wos-bench` depends on `wos-synth-core` + `wos-synth-mock` (and optionally `wos-synth-anthropic`) as library dependencies.

---

## Prerequisites

- `wos-authoring` crate exists (see [2026-04-17-wos-authoring-crate.md](./2026-04-17-wos-authoring-crate.md)).
- `wos-mcp` crate exists (see [2026-04-17-wos-mcp-crate.md](./2026-04-17-wos-mcp-crate.md)) — OR the stopgap `ToolContext` impl in `wos-synth-core/src/stopgap.rs` is acceptable pre-`wos-mcp`.
- [§5.1 schema description audit](./2026-04-16-wos-schema-description-audit.md) — schemas must be rich enough for LLM generation to be feasible.
- [§5.2 structured lint diagnostics](./2026-04-16-wos-structured-lint-diagnostics.md) — repair prompt consumes `LintDiagnostic` JSON.
- [§5.3 trace-emitting conformance](./2026-04-16-wos-trace-emitting-conformance.md) — repair prompt consumes `ConformanceTrace` on behavioral failures.
- An LLM provider (Anthropic API key via env var; `wos-synth-mock` covers offline CI runs).

## Completion criteria

1. `wos-synth-cli generate --problem <path-to-nl.md> [--layer kernel|governance|ai]` emits a valid WOS document, or reports why the loop did not converge.
2. Every iteration is logged to a `SynthTrace` artifact.
3. `Prompter` trait is implemented by both `AnthropicPrompter` (real) and `MockPrompter` (deterministic, no network).
4. Prompt caching is enabled on `AnthropicPrompter` (schemas + BLUF specs are cache anchors).
5. `wos-synth-core` has zero provider deps: `cargo tree -p wos-synth-core | grep -E 'reqwest|tokio|anthropic'` returns nothing.
6. At least 10 NL problem statements from the [benchmark plan](./2026-04-16-wos-synthesis-benchmark.md) produce converging workflows.

## File structure

```
crates/wos-synth-core/
  Cargo.toml              — depends on wos-core, wos-lint (stopgap only), wos-conformance (stopgap only), wos-authoring, serde, thiserror, tracing
  src/lib.rs              — public API: Prompter, ToolContext, SynthLoop, SynthOutcome, SynthTrace
  src/prompter.rs         — Prompter trait (async, model-agnostic)
  src/tool_context.rs     — ToolContext trait (lint/conformance dispatch abstraction)
  src/loop.rs             — SynthLoop orchestrator
  src/prompts/
    mod.rs
    generate.rs           — generate prompt template (pure function)
    repair.rs             — repair prompt template (pure function, consumes LintDiagnostic + ConformanceTrace)
  src/types.rs            — SynthOutcome, CacheAnchor, SynthTrace
  src/stopgap.rs          — in-crate ToolContext impl calling wos-lint/wos-conformance directly; removed once wos-mcp lands

crates/wos-synth-anthropic/
  Cargo.toml              — depends on wos-synth-core, anthropic-sdk, reqwest, tokio, serde, thiserror
  src/lib.rs              — AnthropicPrompter impl of Prompter; no public types beyond that

crates/wos-synth-mock/
  Cargo.toml              — depends on wos-synth-core only; no network deps
  src/lib.rs              — MockPrompter (deterministic, keyed by prompt hash); used in tests and wos-bench offline runs

crates/wos-synth-cli/
  Cargo.toml              — depends on wos-synth-core, wos-synth-anthropic, wos-synth-mock, clap, tokio
  src/main.rs             — binary: wires Prompter + ToolContext + SynthLoop; --dry-run uses MockPrompter
```

---

## Task 1: Scaffold four crates

**Files:**
- Create: `crates/wos-synth-core/Cargo.toml`, `src/lib.rs`
- Create: `crates/wos-synth-anthropic/Cargo.toml`, `src/lib.rs`
- Create: `crates/wos-synth-mock/Cargo.toml`, `src/lib.rs`
- Create: `crates/wos-synth-cli/Cargo.toml`, `src/main.rs`
- Modify: root `Cargo.toml` workspace members.
- Modify or create: CI workflow matrix.

- [ ] **Step 1.1:** Add all four crates to workspace `members` in root `Cargo.toml`.

- [ ] **Step 1.2:** Each crate compiles to an empty library or binary — just enough to pass `cargo build --workspace`. No feature gates needed; providers are separate crates, not feature flags.

- [ ] **Step 1.3:** Verify zero provider deps in core:

```bash
cargo tree -p wos-synth-core | grep -E 'reqwest|tokio|anthropic'
# Expected: no output
```

- [ ] **Step 1.4:** CI matrix — add a single job that runs `cargo test --workspace`. No special feature matrix needed (provider crates compile cleanly in a default workspace build because they are separate crates, not features).

- [ ] **Step 1.5:** Commit. `build: scaffold wos-synth-core/anthropic/mock/cli crate family`.

## Task 2: `Prompter` trait + `CacheAnchor` + `Completion` types

**Files:**
- Create: `crates/wos-synth-core/src/prompter.rs`
- Create: `crates/wos-synth-core/src/types.rs`
- Create: `crates/wos-synth-mock/src/lib.rs` (MockPrompter)

- [ ] **Step 2.1:** In `wos-synth-core/src/prompter.rs`:

```rust
#[async_trait::async_trait]
pub trait Prompter: Send + Sync {
    async fn complete(
        &self,
        system: &str,
        user: &str,
        cache_anchors: &[CacheAnchor],
    ) -> Result<Completion, PrompterError>;
}

pub struct CacheAnchor {
    pub name: &'static str,
    pub content: String,
}

pub struct Completion {
    pub text: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
}
```

- [ ] **Step 2.2:** In `wos-synth-mock/src/lib.rs`, implement `MockPrompter`. It returns canned responses keyed by a hash of the user prompt string. Callers register responses with `mock.expect(prompt_fragment, response_text)` before use. No network. No async runtime required beyond what `async_trait` demands.

- [ ] **Step 2.3:** Three unit tests in `wos-synth-mock/tests/mock_prompter.rs`:
  - Single-turn: registered prompt hash returns expected text.
  - Unknown prompt hash returns a `PrompterError::UnexpectedPrompt`.
  - Multiple registered prompts return independently.

- [ ] **Step 2.4:** Commit. `feat: Prompter trait + CacheAnchor + MockPrompter`.

## Task 3: `ToolContext` trait + in-crate stopgap

**Files:**
- Create: `crates/wos-synth-core/src/tool_context.rs`
- Create: `crates/wos-synth-core/src/stopgap.rs`

- [ ] **Step 3.1:** In `wos-synth-core/src/tool_context.rs`:

```rust
pub trait ToolContext: Send + Sync {
    fn lint(&self, doc: &Document) -> Result<Vec<LintDiagnostic>, ToolError>;
    fn run_conformance(&self, doc: &Document) -> Result<ConformanceTrace, ToolError>;
    fn document_from_json(&self, s: &str) -> Result<Document, ToolError>;
}
```

- [ ] **Step 3.2:** In `wos-synth-core/src/stopgap.rs`, implement `StopgapToolContext` that calls `wos_lint::lint_document` and `wos_conformance::run` directly. Include a `#[doc]` comment:

```rust
/// Stopgap ToolContext that dispatches directly to wos-lint and wos-conformance.
/// Removed in a future plan once `wos-mcp` is stable and `wos-synth-cli`
/// wires ToolContext to wos-mcp's in-process dispatch entry point.
pub struct StopgapToolContext;
```

- [ ] **Step 3.3:** Two unit tests: lint returns diagnostics for an invalid document; conformance returns a failing trace for a document that violates a T3 rule.

- [ ] **Step 3.4:** Commit. `feat: ToolContext trait + StopgapToolContext`.

## Task 4: Prompt templates (pure functions)

**Files:**
- Create: `crates/wos-synth-core/src/prompts/generate.rs`
- Create: `crates/wos-synth-core/src/prompts/repair.rs`

- [ ] **Step 4.1:** `generate_prompt(problem: &str, layer: Layer, schema: &str, bluf: &str, fixtures: &[&str]) -> (String, String, Vec<CacheAnchor>)` — returns `(system, user, cache_anchors)`. Pure function, no IO, no async. Cache anchors ordered: schema (largest/most stable), BLUF digest, fixtures, then dynamic user prompt.

- [ ] **Step 4.2:** `repair_prompt(previous_attempt: &str, lint_diags: &[LintDiagnostic], conformance_trace: &ConformanceTrace) -> (String, String)` — returns `(system, user)`. Directive: "apply the minimum diff that resolves each diagnostic." Pure function, no IO.

- [ ] **Step 4.3:** Unit tests: generate prompt contains the problem string and the schema string; repair prompt contains each diagnostic code.

- [ ] **Step 4.4:** Commit. `feat: wos-synth-core prompt templates (pure functions)`.

## Task 5: `SynthLoop` orchestrator

**Files:**
- Create: `crates/wos-synth-core/src/loop.rs`

- [ ] **Step 5.1:** Public signature:

```rust
pub async fn synthesize(
    problem: &str,
    layer: Layer,
    prompter: &dyn Prompter,
    tools: &dyn ToolContext,
    max_iterations: u32,
) -> Result<SynthOutcome, SynthError> {
    let (system, user, anchors) = prompts::generate_prompt(problem, layer, ...);
    let mut attempt_text = prompter.complete(&system, &user, &anchors).await?.text;
    let mut trace = SynthTrace::new();

    for i in 0..max_iterations {
        let doc = tools.document_from_json(&attempt_text)?;
        let lint_diags = tools.lint(&doc)?;
        let conformance = tools.run_conformance(&doc)?;
        trace.push(i, &attempt_text, &lint_diags, &conformance);

        if lint_diags.is_empty() && conformance.outcome == Outcome::Pass {
            return Ok(SynthOutcome::Converged { document: attempt_text, trace });
        }

        let (sys2, usr2) = prompts::repair_prompt(&attempt_text, &lint_diags, &conformance);
        attempt_text = prompter.complete(&sys2, &usr2, &[]).await?.text;
    }

    Ok(SynthOutcome::Unconverged { last_attempt: attempt_text, trace })
}
```

- [ ] **Step 5.2:** Unit test using `MockPrompter` + `StopgapToolContext` (or a test-double `ToolContext`). Pre-bake two rounds: first call returns a document with one lint error; second call returns a clean document. Assert `SynthOutcome::Converged` and that `trace.iterations.len() == 2`.

- [ ] **Step 5.3:** Commit. `feat: SynthLoop orchestrator with mock-provider convergence test`.

## Task 6: `AnthropicPrompter` in `wos-synth-anthropic`

**Files:**
- Create: `crates/wos-synth-anthropic/src/lib.rs`

- [ ] **Step 6.1:** HTTP client via `anthropic-sdk` v0.1.5. Accept model id via config (default: `claude-opus-4-7`). Respect `ANTHROPIC_API_KEY`. No keys ever committed.

- [ ] **Step 6.2:** Prompt caching enabled. Cache anchors passed by the caller are marshalled into the Anthropic Messages API's `cache_control` blocks. Ordering: schema → BLUF → fixtures → dynamic prompt.

- [ ] **Step 6.3:** Integration test in `tests/integration.rs` gated on env var presence:

```rust
#[tokio::test]
async fn anthropic_integration_smoke() {
    let Ok(key) = std::env::var("ANTHROPIC_API_KEY") else { return; };
    // minimal completion call; assert non-empty text
}
```

- [ ] **Step 6.4:** Commit. `feat: AnthropicPrompter with prompt caching`.

## Task 7: CLI in `wos-synth-cli`

**Files:**
- Create: `crates/wos-synth-cli/src/main.rs`

- [ ] **Step 7.1:** Subcommand `generate`:

```
wos-synth-cli generate
  --problem <path.md>
  --layer <kernel|governance|ai>
  --max-iterations <n>       # default 5
  --output <workflow.json>   # default stdout
  --dry-run                  # use MockPrompter, no network
```

On converge: writes the document. On unconverged: writes a `*.trace.json` alongside the output path and exits non-zero.

- [ ] **Step 7.2:** `--dry-run` wires `MockPrompter` instead of `AnthropicPrompter`. Uses `StopgapToolContext` until `wos-mcp` in-process dispatch is available.

- [ ] **Step 7.3:** Subcommand `explain <trace.json>` prints a human-readable loop transcript: iteration number, lint diagnostics count, conformance outcome, and token cost per iteration.

- [ ] **Step 7.4:** Commit. `feat: wos-synth-cli with generate + explain subcommands`.

## Task 8: Publish `SynthTrace` JSON Schema

**Files:**
- Create: `schemas/synth/synth-trace.schema.json`

- [ ] **Step 8.1:** JSON Schema for `SynthTrace` and `SynthOutcome`. Fields: `outcome` (enum: `converged|unconverged`), `iterations` (array of per-iteration snapshots), `total_tokens`.

- [ ] **Step 8.2:** `schemars`-derived verification test in `wos-synth-core` that serializes a `SynthTrace` instance and validates it against the schema. Prevents drift between the Rust type and the published schema.

- [ ] **Step 8.3:** Commit. `feat: publish SynthTrace JSON Schema`.

---

## Self-review checklist

- All four crates added to workspace members (Task 1).
- `cargo tree -p wos-synth-core | grep -E 'reqwest|tokio|anthropic'` returns nothing (Task 1).
- `Prompter` trait + `MockPrompter` cover unit tests without network (Tasks 2, 3).
- `ToolContext` trait + stopgap allow loop tests without `wos-mcp` (Task 3).
- Prompt templates are pure functions, no IO (Task 4).
- Loop converges deterministically against `MockPrompter` in tests (Task 5).
- `AnthropicPrompter` uses prompt caching; integration test skips without `ANTHROPIC_API_KEY` (Task 6).
- CLI covers generate, dry-run, explain (Task 7).
- `SynthTrace` schema is published and drift-checked (Task 8).

## Why this matters

This is the flagship reference implementation for Claim A ("WOS is designed for LLMs"). Until this family of crates exists, that claim is an assertion. After it exists, it is a runnable demo paired with the [synthesis benchmark](./2026-04-16-wos-synthesis-benchmark.md) — turning the assertion into a falsifiable metric.

The four-crate split mirrors the `packages/formspec-chat` + `packages/formspec-mcp` separation in the parent Formspec project: the loop orchestrator (`ChatSession` → `SynthLoop`) lives in a library with no provider or transport deps; providers (`AIAdapter` impls → `Prompter` impls) are separate packages; tool dispatch (`ToolContext`) is delegated to `wos-mcp`. The same pattern that keeps Formspec's chat layer testable without a live LLM keeps `wos-synth-core` testable without any network.

**Estimated effort:** ~4 engineer-weeks for a working v0; ongoing thereafter.

---

## Previous iteration footnote

The 2026-04-16 version of this plan described a single monolithic `wos-synth` crate with a `--features synth` Cargo feature gate to keep LLM client deps out of default builds. That approach was superseded for the following reason: feature flags are a compilation-time mechanism — they keep deps out of a build but not out of a crate's `Cargo.toml`. Crate separation is a distribution mechanism — it keeps deps out of the dependency graph entirely for consumers that do not need providers. The `packages/formspec-chat` precedent (loop in chat, tools in mcp, adapters separate) demonstrated that the separation is worth the extra crate overhead. Feature flags remain useful within a single crate for optional capabilities; they are not the right tool for separating a library's core loop from its heavyweight provider implementations.
