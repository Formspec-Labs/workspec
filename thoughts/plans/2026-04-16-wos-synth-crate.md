# WOS Synth Crate — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the WOS LLM-authoring harness as a family of crates that takes natural language (+ optional context), generates a workflow document, lints it, applies fixes, runs conformance, iterates until stable, and emits the final document plus a trace of the loop. Split across four crates to honor dependency inversion: core owns the loop + abstractions; each provider is a separate crate; CLI composes them. `wos-synth-core` is the authoring library (single spec); the separate [`wos-bench` crate](./2026-04-16-wos-synthesis-benchmark.md) is the measurement harness that imports `wos-synth-core` as a library dependency.

**Architecture:** Thin orchestrator in `wos-synth-core`. It does not embed an LLM; it calls a provider via the `Prompter` trait. Tools (lint, conformance) are called via the `ToolContext` trait, not by direct crate coupling. No network client, no Anthropic SDK, no `reqwest`, no `tokio` appear anywhere in `wos-synth-core`'s dependency graph. Default provider uses `AnthropicPrompter` in `wos-synth-anthropic` (Opus 4.7 with prompt caching). The loop is deterministic modulo the LLM: generate → diagnostics → targeted repair prompt → diagnostics → stop when diagnostics are empty or iteration cap reached. Prompt assembly consumes the schemas, the BLUF spec digests, and the diagnostic stream from `wos-lint` and `wos-conformance` (delivered through `ToolContext`).

**Dependency inversion:** The original design proposed a single `wos-synth` crate with LLM-provider deps gated behind `--features synth`. Architectural review (2026-04-17) rejected this: a feature flag is a compilation-time mechanism that keeps deps out of a build, not out of the crate's `Cargo.toml`. Crate separation is a distribution mechanism — it keeps deps out of the dependency graph entirely for consumers that do not need providers. The loop (business logic) must not compile-time-depend on any specific LLM provider (infrastructure detail). The split mirrors the parent Formspec project: `formspec-chat` owns the loop + `AIAdapter` + `ToolContext` interfaces, concrete providers are separate packages, and tool handlers live in `formspec-mcp` which `formspec-chat` consumes via its `ToolContext`. See [open questions Q2](../archive/reviews/2026-04-16-architecture-review-open-questions.md#q2-should-wos-synth-live-in-wos-spec-or-a-sibling-repo) for the full architectural discussion that preceded this decision.

**Tech Stack:** Rust (four crates). `wos-synth-core`: pure Rust, no network deps, no feature flags. `wos-synth-anthropic`: unconditionally depends on `anthropic-sdk`, `reqwest`, `tokio`. `wos-synth-mock`: `wos-synth-core` only. `wos-synth-cli`: `clap` + the two above. Because providers are separate crates, the old `--features synth` approach is obsoleted: crate-level separation gives cleaner distribution boundaries than feature flags.

**Spec anchor:** [architecture-review-handoff.md §5.4](../archive/reviews/2026-04-16-architecture-review-handoff.md) — the reference impl for [Claim A](../../POSITIONING.md). Location (in-tree with extraction trigger) resolved in [open questions Q2](../archive/reviews/2026-04-16-architecture-review-open-questions.md#q2-should-wos-synth-live-in-wos-spec-or-a-sibling-repo). Claim A's first-class status resolved in [open questions Q1](../archive/reviews/2026-04-16-architecture-review-open-questions.md#q1-is-claim-a-llm-authoring-an-accepted-first-class-goal). Two-crate split (synth + bench) resolved in [open questions Q6](../archive/reviews/2026-04-16-architecture-review-open-questions.md#q6-is-wos-synth-54-and-the-authoring-benchmark-55-one-project-or-two).

---

## Crate layout

| Crate | Role | Depends on |
|---|---|---|
| `wos-synth-core` | Loop + `Prompter` trait + `ToolContext` trait + prompt templates | `wos-core`, `wos-lint`, `wos-conformance` (via `ToolContext`) |
| `wos-synth-anthropic` | Concrete `Prompter` via Anthropic SDK | `wos-synth-core`, `anthropic-sdk`, `reqwest`, `tokio` |
| `wos-synth-mock` | Deterministic mock `Prompter` | `wos-synth-core` |
| `wos-synth-cli` | Binary wiring one `Prompter` + one `ToolContext` | `wos-synth-core`, `wos-synth-anthropic`, `clap` |

---

## Why each crate boundary earns its cost

The "Why this matters" section below frames the four-crate split by analogy to `formspec-chat` + `formspec-mcp`. This section names the concrete downstream consumers that justify each boundary in WOS terms.

1. **`wos-synth-core` — the loop + abstractions.** The loop living in a crate with no provider or transport deps benefits three concrete consumer groups: (a) `wos-bench` running mock-provider benchmarks in CI without pulling `anthropic-sdk`, `tokio`, or `reqwest` into its dependency graph — CI must not require API keys; (b) future vendors embedding WOS authoring in their own product who want to ship their own LLM client (OpenAI, self-hosted llama.cpp, Cohere, etc.) without inheriting Anthropic SDK deps from `wos-synth-core`; (c) offline-only deployments (e.g., air-gapped government environments, which are a realistic audience for a tool about algorithmic due-process compliance) that want the loop available locally without any HTTP client in their dep graph.

2. **`wos-synth-anthropic` — concrete Anthropic provider.** Direct consumers: `wos-synth-cli` (default binding for production use) and `wos-bench` for production-quality reporting runs where real token costs and latency need to be measured. A second provider (e.g., `wos-synth-openai`) would be a ~100 LOC sibling crate implementing the `Prompter` trait. The crate boundary makes that addition trivially additive — a new file, a new Cargo.toml, no feature flags, no conditional compilation in `wos-synth-core`.

3. **`wos-synth-mock` — deterministic test provider.** Consumers: `wos-bench` for CI runs (no API keys in CI); `wos-synth-core`'s own integration tests; future `wos-synth-*` provider conformance tests that need to verify loop behavior without incurring API cost. Separating `wos-synth-mock` from `wos-synth-core` means `wos-synth-core`'s test suite does not need to distinguish "real" from "mock" tests — tests that need deterministic responses import the mock crate; tests that exercise the real Anthropic path are gated on `ANTHROPIC_API_KEY`. Clean seam.

4. **`wos-synth-cli` — binary.** Consumers: humans running `wos-synth generate` and CI pipelines. Separating the binary from the library lets `wos-bench` import `wos-synth-core` without inheriting `clap` or any binary-only deps. More importantly, the CLI is the one place where provider selection is configured from the environment — keeping that decision out of library code ensures the loop is provider-agnostic by construction rather than by discipline.

### Why `ToolContext` earns the trait cost

`ToolContext` is an abstraction with one production implementation (forward to `wos-mcp::dispatch`) and one stopgap (`DirectToolContext`, wrapping `wos-lint`/`wos-conformance` directly until `wos-mcp` lands). A trait with one current production impl looks like YAGNI. Four concrete second-consumer candidates justify it:

1. **Instrumented variant for `wos-bench`.** Wraps the production `ToolContext` to count tool invocations, measure per-call latency, and record diagnostic-repair cycles per problem statement. Without the trait, `wos-bench` has to fork or monkey-patch the loop to collect these metrics.

2. **Caching variant.** During a repair iteration, `lint_document` and `run_conformance` may be called repeatedly on near-identical documents. A caching `ToolContext` that memoizes results keyed by document hash is opt-in via the trait and has zero impact on the production path. This is especially relevant for the benchmark, where many iterations may converge on the same document hash.

3. **Remote variant.** Some deployments will want the tool surface on a different machine from the LLM orchestrator (e.g., conformance checks run on a large-fixture server, LLM calls run on an inference server). A remote `ToolContext` over JSON-RPC satisfies this without touching the loop code.

4. **Dry-run variant for `wos-synth-cli --dry-run`.** Returns synthetic diagnostics to exercise the loop without hitting real lint or conformance. Useful for local development and CI smoke tests where the full tool surface is unavailable.

**The trait earns its abstraction cost once ANY ONE of these second consumers is written.** In the absence of all four, revisit the abstraction during the v0 spike (see `2026-04-17-wos-synth-v0-spike.md` if created). Do not keep the trait purely for hypothetical extensibility.

---

## Prerequisites

- [§5.1 schema description audit](./2026-04-16-wos-schema-description-audit.md) — the schemas must be rich enough for LLM generation to be feasible.
- [§5.2 structured lint diagnostics](./2026-04-16-wos-structured-lint-diagnostics.md) — the repair prompt consumes `LintDiagnostic` JSON.
- [§5.3 trace-emitting conformance](./2026-04-16-wos-trace-emitting-conformance.md) — the repair prompt consumes `ConformanceTrace` on behavioral failures.
- An LLM provider the harness can call (Anthropic API key via env var; offline fallback via `wos-synth-mock`).

## Prerequisites landed separately

- `wos-authoring` crate (see `./2026-04-17-wos-authoring-crate.md`) — intent-driven authoring helpers, analogous to `formspec-studio-core`. Lands before `wos-synth-core`'s `ToolContext` trait switches to its production implementation.
- `wos-mcp` crate (see `./2026-04-17-wos-mcp-crate.md`) — tool handlers + dual-entry (MCP stdio + in-process dispatch), analogous to `formspec-mcp`. Once available, `wos-synth-core`'s `ToolContext` trait uses `wos-mcp::dispatch` as its default production implementation.
- These land first; the in-crate `DirectToolContext` (Task 4b) is a stopgap until both are ready.

## Completion criteria

1. `wos-synth generate --problem <path-to-nl.md> [--layer kernel|governance|ai]` emits a valid WOS document, or reports why the loop did not converge.
2. Every iteration is logged to a synth trace artifact.
3. Provider abstraction: at minimum `AnthropicPrompter` in `wos-synth-anthropic`, with a `MockPrompter` in `wos-synth-mock` (no network).
4. Prompt caching is enabled on the Anthropic provider (schemas + BLUF specs are cache-anchors).
5. `wos-synth-core` has zero provider deps: `cargo tree -p wos-synth-core | grep -E 'reqwest|tokio|anthropic'` returns nothing.
6. At least 10 NL problem statements from the [benchmark plan](./2026-04-16-wos-synthesis-benchmark.md) produce converging workflows.

## File structure

- **Create:** `crates/wos-synth-core/Cargo.toml`
- **Create:** `crates/wos-synth-core/src/lib.rs` — library entry.
- **Create:** `crates/wos-synth-core/src/loop.rs` — orchestrator.
- **Create:** `crates/wos-synth-core/src/prompter.rs` — `Prompter` trait + `CacheAnchor` + `Completion` types.
- **Create:** `crates/wos-synth-core/src/tool_context/mod.rs` — `ToolContext` trait.
- **Create:** `crates/wos-synth-core/src/tool_context/direct.rs` — `DirectToolContext` stopgap.
- **Create:** `crates/wos-synth-core/src/prompts/{mod,generate,repair}.rs` — prompt templates by layer.
- **Create:** `crates/wos-synth-anthropic/Cargo.toml`
- **Create:** `crates/wos-synth-anthropic/src/lib.rs` — `AnthropicPrompter`.
- **Create:** `crates/wos-synth-mock/Cargo.toml`
- **Create:** `crates/wos-synth-mock/src/lib.rs` — `MockPrompter`.
- **Create:** `crates/wos-synth-cli/Cargo.toml`
- **Create:** `crates/wos-synth-cli/src/main.rs` — `wos-synth generate|repair|explain` CLI binary.
- **Create:** `crates/wos-synth-core/tests/mock_loop.rs` — deterministic loop tests using `MockPrompter`.
- **Modify:** `Cargo.toml` workspace members.

---

## Task 1: Reshape the scaffold to the split layout

**Context:** A monolithic `wos-synth` scaffold landed at commit `2815e4d`. That scaffold used a `--features synth` gate to keep Anthropic SDK deps out of default builds — a feature flag, not dependency inversion. This task realigns the scaffold with the DIP split.

**What needs to change:**

- The directory `crates/wos-synth/` becomes `crates/wos-synth-core/` — either via rename or by leaving the loop/trait code in place while extracting the provider code to new sibling crates during this task.
- The `--features synth` gate is removed from `wos-synth-core` entirely. Extract `wos-synth-anthropic` as a separate crate that unconditionally depends on Anthropic SDK; remove the feature flag from `wos-synth-core` entirely. Crate-level separation is the gate, not a feature flag.
- The three policy sections from the scaffold's README are preserved but restated in terms of the split:
  1. **Boundary statement:** *`wos-synth-core` owns the loop, the `Prompter` trait, the `ToolContext` trait, and prompt primitives. Provider crates (`wos-synth-anthropic`, `wos-synth-mock`) depend on `wos-synth-core`, never the reverse. The benchmark crate (`wos-bench`) imports `wos-synth-core` as a library dependency; do not fold them together, do not split the provider abstraction across two crates.*
  2. **Extraction trigger:** *The four `wos-synth-*` crates graduate to a sibling repository when BOTH of these are true: the `Prompter` trait has survived one full release train without a breaking change AND a second production-quality provider implementation exists beyond `wos-synth-anthropic`. Both conditions are observable; neither is calendar-based.*
  3. **Benchmark-regressions-do-not-motivate-spec-changes policy:** *Benchmark regressions do not motivate normative-spec changes unless the benchmark is exercising a claim the spec actually makes. Spec PRs whose motivation cites a benchmark failure must be reviewed against this rule.*

**Files:**

- Rename/restructure: `crates/wos-synth/` → `crates/wos-synth-core/` (and extract provider code to new crates).
- Create: `crates/wos-synth-anthropic/Cargo.toml`, `crates/wos-synth-mock/Cargo.toml`, `crates/wos-synth-cli/Cargo.toml`.
- Modify: root `Cargo.toml` workspace members.
- Modify or create: CI workflow.

- [ ] **Step 1.1:** Add all four crates to workspace `members` in root `Cargo.toml`. Remove the old `crates/wos-synth/` entry.

- [ ] **Step 1.2:** `wos-synth-core` Cargo manifest — no LLM deps, no feature flags:

```toml
[package]
name = "wos-synth-core"
# ...

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = "1"
tracing = "0.1"
async-trait = "0.1"
wos-core = { path = "../wos-core" }
wos-lint = { path = "../wos-lint" }
wos-conformance = { path = "../wos-conformance" }
```

- [ ] **Step 1.3:** `wos-synth-anthropic` Cargo manifest — unconditional LLM deps, no feature flag:

```toml
[package]
name = "wos-synth-anthropic"
# ...

[dependencies]
wos-synth-core = { path = "../wos-synth-core" }
anthropic-sdk = "0.1"
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
```

- [ ] **Step 1.4:** `wos-synth-mock` and `wos-synth-cli` Cargo manifests (mock depends only on `wos-synth-core`; CLI depends on `wos-synth-core` + `wos-synth-anthropic` + `clap`).

- [ ] **Step 1.5:** Verify zero provider deps in core:

```bash
cargo build --workspace
cargo tree -p wos-synth-core | grep -E 'reqwest|tokio|anthropic'
# Expected: no output
cargo build -p wos-synth-anthropic  # also green
```

- [ ] **Step 1.6:** Update CI — default job: `cargo build --workspace && cargo nextest run --workspace`. No special feature matrix needed; providers compile as separate crates in a default workspace build.

- [ ] **Step 1.7:** Author `crates/wos-synth-core/README.md` with the three restated policy sections above.

- [ ] **Step 1.8:** Commit. `build: reshape wos-synth scaffold into core + anthropic + mock + cli crates (DIP split)`.

## Task 2: `Prompter` trait + `MockPrompter`

**Files:**

- Create: `crates/wos-synth-core/src/prompter.rs`
- Create: `crates/wos-synth-mock/src/lib.rs`

The `Prompter` trait lives in `wos-synth-core`. `MockPrompter` is extracted to a separate `crates/wos-synth-mock/` crate — not an internal module.

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

- [ ] **Step 2.2:** In `crates/wos-synth-mock/src/lib.rs`, implement `MockPrompter` as a separate crate (not an internal module of `wos-synth-core`). `MockPrompter` returns canned responses keyed by a hash of the user prompt string. Callers register responses with `mock.expect(prompt_fragment, response_text)` before use. No network.

- [ ] **Step 2.3:** Three unit tests in `wos-synth-mock/tests/mock_prompter.rs`:
  - Single-turn: registered prompt hash returns expected text.
  - Unknown prompt hash returns `PrompterError::UnexpectedPrompt`.
  - Multiple registered prompts return independently.

- [ ] **Step 2.4:** Commit. `feat: wos-synth-core Prompter trait + wos-synth-mock MockPrompter`.

## Task 3: Prompt assembly

**Files:**

- Create: `crates/wos-synth-core/src/prompts/generate.rs`
- Create: `crates/wos-synth-core/src/prompts/repair.rs`

Prompt templates live in `wos-synth-core/src/prompts/`. Pure functions, no IO, no async.

- [ ] **Step 3.1:** `generate` prompt includes, as cache anchors:
  - The target schema for the requested layer.
  - BLUF digest of the relevant spec section(s).
  - 2–3 in-context fixture examples from the [benchmark](./2026-04-16-wos-synthesis-benchmark.md).

- [ ] **Step 3.2:** `repair` prompt includes:
  - The previous attempt.
  - The `LintDiagnostic` array (JSON).
  - The `ConformanceTrace` diff (when a T3 rule failed).
  - A directive: "apply the minimum diff that resolves each diagnostic."

- [ ] **Step 3.3:** Unit tests: generate prompt contains the problem string and the schema; repair prompt contains each diagnostic code.

- [ ] **Step 3.4:** Commit. `feat: wos-synth-core prompt templates for generate and repair`.

## Task 4: The loop

**Files:**

- Create: `crates/wos-synth-core/src/loop.rs`

The orchestrator lives in `wos-synth-core/src/loop.rs` and calls tools via `ToolContext`, NOT directly via `wos_lint::lint_document` / `wos_conformance::run_fixture`.

- [ ] **Step 4.1:** `ToolContext` trait in `wos-synth-core/src/tool_context/mod.rs`:

```rust
#[async_trait::async_trait]
pub trait ToolContext: Send + Sync {
    async fn lint_document(&self, doc: &WosDocument) -> Result<Vec<LintDiagnostic>, ToolError>;
    async fn run_conformance(&self, doc: &WosDocument) -> Result<ConformanceTrace, ToolError>;
}
```

- [ ] **Step 4.2:** Orchestrator in `wos-synth-core/src/loop.rs`:

```rust
pub async fn synthesize(
    provider: &dyn Prompter,
    tools: &dyn ToolContext,
    problem: &str,
    layer: Layer,
    max_iterations: u32,
) -> Result<SynthOutcome, SynthError> {
    let mut attempt = generate(provider, problem, layer).await?;
    let mut trace = SynthTrace::new();

    for i in 0..max_iterations {
        let lint_diags = tools.lint_document(&attempt).await?;
        let t3_trace = tools.run_conformance(&attempt).await?;
        trace.push(i, &attempt, &lint_diags, &t3_trace);

        if lint_diags.is_empty() && t3_trace.outcome == Outcome::Pass {
            return Ok(SynthOutcome::Converged { document: attempt, trace });
        }

        attempt = repair(provider, &attempt, &lint_diags, &t3_trace).await?;
    }

    Ok(SynthOutcome::Unconverged { last_attempt: attempt, trace })
}
```

- [ ] **Step 4.3:** Unit test using `MockPrompter` (from `wos-synth-mock`) + a test-double `ToolContext`. Pre-bake two rounds: first call returns a document with one lint error; second call returns a clean document. Assert `SynthOutcome::Converged` and `trace.iterations.len() == 2`.

- [ ] **Step 4.4:** Commit. `feat: wos-synth-core loop orchestrator with ToolContext abstraction`.

## Task 4b: `DirectToolContext` stopgap

**Files:**

- Create: `crates/wos-synth-core/src/tool_context/direct.rs`

Implement `DirectToolContext` in `wos-synth-core/src/tool_context/direct.rs` — an in-crate `ToolContext` that wraps `wos-lint` + `wos-conformance` directly. This is a stopgap until `wos-mcp` (see `./2026-04-17-wos-mcp-crate.md`) lands and provides in-process dispatch.

- [ ] **Step 4b.1:** Implement `DirectToolContext`:

```rust
// STOPGAP: wraps wos-lint + wos-conformance directly.
// TODO: replace with wos-mcp::dispatch once wos-mcp lands.
pub struct DirectToolContext;

#[async_trait::async_trait]
impl ToolContext for DirectToolContext {
    async fn lint_document(&self, doc: &WosDocument) -> Result<Vec<LintDiagnostic>, ToolError> {
        wos_lint::lint_document(doc).map_err(ToolError::Lint)
    }

    async fn run_conformance(&self, doc: &WosDocument) -> Result<ConformanceTrace, ToolError> {
        wos_conformance::run_fixture(doc).map_err(ToolError::Conformance)
    }
}
```

- [ ] **Step 4b.2:** Use `DirectToolContext` as the default in `wos-synth-cli` until `wos-mcp` is available.

- [ ] **Step 4b.3:** Two unit tests: lint returns diagnostics for an invalid document; conformance returns a failing trace for a document that violates a T3 rule.

- [ ] **Step 4b.4:** Commit. `feat: wos-synth-core DirectToolContext stopgap wrapping wos-lint + wos-conformance`.

## Task 5: Anthropic provider

**Files:**

- Create: `crates/wos-synth-anthropic/src/lib.rs`

This crate is entirely separate from `wos-synth-core`. It unconditionally depends on `anthropic-sdk`, `reqwest`, and `tokio`. No feature flag — crate-level separation is the gate.

- [ ] **Step 5.1:** Implement `AnthropicPrompter` in `crates/wos-synth-anthropic/src/lib.rs`. HTTP client via `anthropic-sdk`. Accept model id via config (default: `claude-opus-4-7`). Respect `ANTHROPIC_API_KEY`. No keys ever committed.

- [ ] **Step 5.2:** Enable prompt caching. Cache anchors ordered: schema (largest, most stable), BLUF spec, fixtures, then dynamic prompt.

- [ ] **Step 5.3:** Integration test gated on the env var being present; skip otherwise:

```rust
#[tokio::test]
async fn anthropic_integration_smoke() {
    let Ok(_key) = std::env::var("ANTHROPIC_API_KEY") else { return; };
    // minimal completion call; assert non-empty text
}
```

- [ ] **Step 5.4:** Commit. `feat: wos-synth-anthropic AnthropicPrompter with prompt caching`.

## Task 6: CLI

**Files:**

- Create: `crates/wos-synth-cli/src/main.rs`

`wos-synth-cli` is a separate crate whose binary is named `wos-synth`. It wires one `Prompter` (`AnthropicPrompter` or `MockPrompter`) + one `ToolContext` (`DirectToolContext` until `wos-mcp` lands) and passes them to `wos_synth_core::synthesize`.

- [ ] **Step 6.1:** `wos-synth generate --problem <path.md> --layer kernel --max-iterations 5 --output workflow.json`. On converge, writes the workflow; on unconverged, writes a `*.trace.json` and non-zero exit.

- [ ] **Step 6.2:** `wos-synth explain <trace.json>` prints a human-readable loop transcript: iteration number, lint diagnostics count, conformance outcome, and token cost per iteration.

- [ ] **Step 6.3:** `wos-synth dry-run` — runs the loop with `MockPrompter` (from `wos-synth-mock`) for smoke-testing in CI without network.

- [ ] **Step 6.4:** Commit. `feat: wos-synth-cli binary wiring AnthropicPrompter + DirectToolContext`.

## Task 7: Publish synth-trace schema

**Files:**

- Create: `work-spec/schemas/synth/synth-trace.schema.json`

Trace types (`SynthTrace`, `SynthOutcome`) are defined in `wos-synth-core`. The schema is derived from those types and published to the shared schemas directory.

- [ ] **Step 7.1:** JSON Schema for `SynthTrace` and `SynthOutcome`. Fields: `outcome` (enum: `converged|unconverged`), `iterations` (array of per-iteration snapshots), `total_tokens`.
- [ ] **Step 7.2:** `schemars`-derived verification test in `wos-synth-core` that serializes a `SynthTrace` instance and validates it against the schema. Prevents drift between the Rust type and the published schema.
- [ ] **Step 7.3:** Commit. `feat: publish wos-synth-core trace schema`.

---

## Self-review checklist

- All four crates integrated into the workspace (Task 1).
- `cargo tree -p wos-synth-core | grep -E 'reqwest|tokio|anthropic'` returns nothing (Task 1).
- `Prompter` trait in `wos-synth-core`; `MockPrompter` in `wos-synth-mock` as a separate crate (Task 2).
- `ToolContext` trait in `wos-synth-core`; loop calls only `tools.lint_document` / `tools.run_conformance` (Tasks 4).
- `DirectToolContext` stopgap present with `// STOPGAP:` comment and TODO (Task 4b).
- Prompt templates are pure functions in `wos-synth-core/src/prompts/` (Task 3).
- Loop converges deterministically against `MockPrompter` + test-double `ToolContext` (Task 4).
- `AnthropicPrompter` uses prompt caching; integration test skips without `ANTHROPIC_API_KEY` (Task 5).
- CLI covers generate, explain, dry-run (Task 6).
- `SynthTrace` schema published and drift-checked against `wos-synth-core` types (Task 7).

## Why this matters

This is the flagship reference impl for Claim A. Until `wos-synth-core` exists, "WOS is designed for LLMs" is an assertion; after it exists, it is a demo. Pair with the [synthesis benchmark](./2026-04-16-wos-synthesis-benchmark.md) to turn the assertion into a falsifiable metric.

The four-crate split mirrors the `packages/formspec-chat` + `packages/formspec-mcp` separation in the parent Formspec project: the loop orchestrator (`ChatSession` → `SynthLoop`) lives in a library with no provider or transport deps; providers (`AIAdapter` impls → `Prompter` impls) are separate crates; tool dispatch (`ToolContext`) is delegated to `wos-mcp` once it lands. The same pattern that keeps Formspec's chat layer testable without a live LLM keeps `wos-synth-core` testable without any network.

**Estimated effort:** ~5–6 engineer-weeks for a working v0 across all four crates; ongoing thereafter.

*Architectural note: this plan initially proposed a monolithic `wos-synth` crate with feature-gated provider deps. Architectural review (2026-04-17) flagged the design as violating dependency inversion — the loop was compile-time-coupled to a specific provider. The revised plan splits along DIP boundaries, mirroring parent Formspec's `formspec-chat` / `formspec-mcp` / `formspec-studio-core` layering. The scaffold that landed at `2815e4d` will be reshaped (or its position preserved as `wos-synth-core`) during Task 1 of the revised plan.*

---

## Addendum — v0-spike findings (2026-04-20)

Findings from the v0 spike retrospective
([`thoughts/research/2026-04-20-wos-synth-v0-spike-findings.md`](../research/2026-04-20-wos-synth-v0-spike-findings.md))
that affect this plan:

- **`ToolContext` is provisional, not empirically justified.** The spike at
  <800 LOC does not use the trait; it calls `wos_lint::lint_document` and
  `wos_conformance::run_fixture` directly. The trait shipped in `wos-synth-core`
  Task 2 without a spike counter-example. Keep the trait but **do not extend
  it with speculative methods** (remote dispatch, caching, benchmarking
  hooks) until a second concrete implementation materializes to inform the
  shape. Treat the existing `DirectToolContext` as the one valid
  implementation.
- **Structured repair-prompt improvement recommended.** Both this crate and
  `wos-synth-spike` flatten `LintDiagnostic` to its `Display` form in the
  repair prompt, losing `rule_id`, `suggested_fix`, and `spec_ref`. The
  single cheapest prompt-engineering gain available is emitting these as a
  structured block (JSON or labelled sections) so the LLM gets a rule
  identifier + a remediation hint alongside the narrative message. Track
  this as a follow-up to §5.4 Task 6 or as a §5.5 prerequisite — whichever
  ships first.
- **Conformance gate needs a fixture wrapper.** `wos_conformance::run_fixture`
  requires a full `ConformanceFixture`; there is no `run(&doc)` entry point.
  `wos-synth-spike` wraps the kernel inline (`documents: { "kernel": "inline"
  }`, empty event sequence). If `wos-synth-core`'s `DirectToolContext` gains
  a conformance method, it should either (a) mirror this wrapper pattern or
  (b) depend on a new upstream `wos_conformance::smoke_test_document(doc:
  &Value) -> Result<(), Vec<String>>` helper. Option (b) is preferred; track
  as a `wos-conformance` follow-up.
