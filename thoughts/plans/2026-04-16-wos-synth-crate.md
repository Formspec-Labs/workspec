# WOS Synth Crate — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `wos-synth` — the reference LLM-authoring harness — as a fourth crate alongside `wos-core`, `wos-lint`, `wos-conformance`. It takes natural language (+ optional context), generates a workflow document, lints it, applies fixes, runs conformance, iterates until stable, and emits the final document plus a trace of the loop.

**Architecture:** Thin orchestrator. It does not embed an LLM; it calls a provider via a pluggable trait. Default provider uses the Anthropic SDK (Opus 4.7 with prompt caching). The loop is deterministic modulo the LLM: generate → diagnostics → targeted repair prompt → diagnostics → stop when diagnostics are empty or iteration cap reached. Prompt assembly consumes the schemas, the BLUF spec digests, and the diagnostic stream from `wos-lint` and `wos-conformance`.

**Tech Stack:** Rust (new `wos-synth` crate), `anthropic-sdk` or equivalent via HTTP, reqwest, tokio. Optional: llama-cpp fallback provider for offline runs.

**Spec anchor:** [architecture-review-handoff.md §5.4](../reviews/2026-04-16-architecture-review-handoff.md) — the reference impl for [Claim A](../../POSITIONING.md).

---

## Prerequisites

- [§5.1 schema description audit](./2026-04-16-wos-schema-description-audit.md) — the schemas must be rich enough for LLM generation to be feasible.
- [§5.2 structured lint diagnostics](./2026-04-16-wos-structured-lint-diagnostics.md) — the repair prompt consumes `LintDiagnostic` JSON.
- [§5.3 trace-emitting conformance](./2026-04-16-wos-trace-emitting-conformance.md) — the repair prompt consumes `ConformanceTrace` on behavioral failures.
- An LLM provider the crate can call (Anthropic API key via env var; offline fallback is nice-to-have).

## Completion criteria

1. `wos-synth generate --problem <path-to-nl.md> [--layer kernel|governance|ai]` emits a valid WOS document, or reports why the loop did not converge.
2. Every iteration is logged to a synth trace artifact.
3. Provider abstraction: at minimum `AnthropicProvider`, with a `MockProvider` for tests (no network).
4. Prompt caching is enabled on the Anthropic provider (schemas + BLUF specs are cache-anchors).
5. At least 10 NL problem statements from the [benchmark plan](./2026-04-16-wos-synthesis-benchmark.md) produce converging workflows.

## File structure

- **Create:** `crates/wos-synth/Cargo.toml`
- **Create:** `crates/wos-synth/src/lib.rs` — library entry.
- **Create:** `crates/wos-synth/src/loop.rs` — orchestrator.
- **Create:** `crates/wos-synth/src/provider/{mod,anthropic,mock}.rs` — LLM provider abstraction.
- **Create:** `crates/wos-synth/src/prompts/{generate,repair}.rs` — prompt templates by layer.
- **Create:** `crates/wos-synth/src/main.rs` — `wos-synth generate|repair|explain` CLI.
- **Create:** `crates/wos-synth/tests/mock_loop.rs` — deterministic loop tests using mock provider.
- **Modify:** `Cargo.toml` workspace members.

---

## Task 1: Scaffold the crate

**Files:**
- Create: `crates/wos-synth/Cargo.toml`, `src/lib.rs`, `src/main.rs`.
- Modify: root `Cargo.toml`.

- [ ] **Step 1.1:** Add to workspace `members`.
- [ ] **Step 1.2:** Deps: `serde`, `serde_json`, `clap`, `reqwest`, `tokio`, `thiserror`, `tracing`, `wos-lint`, `wos-conformance`, `wos-core`.
- [ ] **Step 1.3:** `cargo build -p wos-synth` — green, even though lib is empty.
- [ ] **Step 1.4:** Commit. `build: scaffold wos-synth crate`.

## Task 2: Provider trait and mock implementation

**Files:**
- Create: `crates/wos-synth/src/provider/mod.rs`
- Create: `crates/wos-synth/src/provider/mock.rs`

- [ ] **Step 2.1:** Provider trait:

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(
        &self,
        system: &str,
        user: &str,
        cache_anchors: &[CacheAnchor],
    ) -> Result<Completion, ProviderError>;
}

pub struct CacheAnchor {
    pub name: &'static str,
    pub content: String,
}
```

- [ ] **Step 2.2:** `MockProvider` returns canned responses indexed by prompt hash. Used in tests to keep the loop deterministic.

- [ ] **Step 2.3:** Commit. `feat: wos-synth provider trait + mock`.

## Task 3: Prompt assembly

**Files:**
- Create: `crates/wos-synth/src/prompts/generate.rs`
- Create: `crates/wos-synth/src/prompts/repair.rs`

- [ ] **Step 3.1:** `generate` prompt includes, as cache anchors:
  - The target schema for the requested layer.
  - BLUF digest of the relevant spec section(s).
  - 2–3 in-context fixture examples from the [benchmark](./2026-04-16-wos-synthesis-benchmark.md).

- [ ] **Step 3.2:** `repair` prompt includes:
  - The previous attempt.
  - The `LintDiagnostic` array (JSON).
  - The `ConformanceTrace` diff (when a T3 rule failed).
  - A directive: "apply the minimum diff that resolves each diagnostic."

- [ ] **Step 3.3:** Commit. `feat: wos-synth prompt templates for generate and repair`.

## Task 4: The loop

**Files:**
- Create: `crates/wos-synth/src/loop.rs`

- [ ] **Step 4.1:** Pseudocode:

```rust
pub async fn synthesize(
    provider: &dyn LlmProvider,
    problem: &str,
    layer: Layer,
    max_iterations: u32,
) -> Result<SynthOutcome, SynthError> {
    let mut attempt = generate(provider, problem, layer).await?;
    let mut trace = SynthTrace::new();

    for i in 0..max_iterations {
        let lint_diags = wos_lint::lint_document(&attempt)?;
        let t3_trace = wos_conformance::run_fixture(&attempt)?;
        trace.push(i, &attempt, &lint_diags, &t3_trace);

        if lint_diags.is_empty() && t3_trace.outcome == Outcome::Pass {
            return Ok(SynthOutcome::Converged { document: attempt, trace });
        }

        attempt = repair(provider, &attempt, &lint_diags, &t3_trace).await?;
    }

    Ok(SynthOutcome::Unconverged { last_attempt: attempt, trace })
}
```

- [ ] **Step 4.2:** Unit test with mock provider returning pre-baked fix sequence; assert the loop converges.

- [ ] **Step 4.3:** Commit. `feat: wos-synth loop orchestrator with mock-provider test`.

## Task 5: Anthropic provider

**Files:**
- Create: `crates/wos-synth/src/provider/anthropic.rs`

- [ ] **Step 5.1:** HTTP client against the Messages API. Use `claude-opus-4-7` (latest capable) per `CLAUDE.md` guidance; accept model id via config.

- [ ] **Step 5.2:** Enable prompt caching. Cache anchors ordered: schema (largest, most stable), BLUF spec, fixtures, then dynamic prompt.

- [ ] **Step 5.3:** Respect `ANTHROPIC_API_KEY`. No keys ever committed.

- [ ] **Step 5.4:** Integration test gated on the env var being present; skip otherwise.

- [ ] **Step 5.5:** Commit. `feat: AnthropicProvider with prompt caching`.

## Task 6: CLI

**Files:**
- Modify: `crates/wos-synth/src/main.rs`

- [ ] **Step 6.1:** `wos-synth generate --problem <path.md> --layer kernel --max-iterations 5 --output workflow.json`. On converge, writes the workflow; on unconverged, writes a `*.trace.json` and non-zero exit.

- [ ] **Step 6.2:** `wos-synth explain <trace.json>` prints a human-readable loop transcript.

- [ ] **Step 6.3:** `wos-synth dry-run` — runs the loop with the mock provider for smoke-testing in CI without network.

- [ ] **Step 6.4:** Commit. `feat: wos-synth CLI`.

## Task 7: Publish synth-trace schema

**Files:**
- Create: `schemas/synth/synth-trace.schema.json`

- [ ] **Step 7.1:** Schema for `SynthTrace` and `SynthOutcome`.
- [ ] **Step 7.2:** schemars-derived verification.
- [ ] **Step 7.3:** Commit. `feat: publish wos-synth trace schema`.

---

## Self-review checklist

- New crate integrated into the workspace (Task 1).
- Provider trait permits mock + real implementations (Task 2).
- Prompt templates use stable cache anchors (Tasks 3, 5).
- Loop converges deterministically against the mock (Task 4).
- Real Anthropic provider works with `ANTHROPIC_API_KEY` set (Task 5).
- CLI covers generate, explain, dry-run (Task 6).
- Trace schema is published and stable (Task 7).

## Why this matters

This is the flagship reference impl for Claim A. Until `wos-synth` exists, "WOS is designed for LLMs" is an assertion; after it exists, it is a demo. Pair with the [synthesis benchmark](./2026-04-16-wos-synthesis-benchmark.md) to turn the assertion into a falsifiable metric.

**Estimated effort:** ~4 engineer-weeks for a working v0; ongoing thereafter.
