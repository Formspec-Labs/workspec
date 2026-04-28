# wos-synth v0 Spike — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a minimal-viable end-to-end LLM-authoring loop for WOS in 2–3 days. Single crate `wos-synth-spike`. Produces a valid WOS kernel document from one NL problem statement using one provider (Anthropic). Disposable — deleted after it validates the architecture.

**Architecture:** ONE crate. Straightforward imperative code. No trait abstractions. No dual-entry dispatch. No feature flags. No provider trait. Directly imports `wos-core`, `wos-lint`, `wos-conformance`. Hardcoded Anthropic client. Hardcoded prompt template. Target: `cargo run -p wos-synth-spike -- --problem benchmarks/problems/purchase-order-approval.md` produces a valid WOS kernel document OR a clear error.

**Tech Stack:** Rust (new throwaway crate `crates/wos-synth-spike/`), `anthropic-sdk`, `reqwest`, `tokio`, `serde_json`, `clap`, existing `wos-core` + `wos-lint` + `wos-conformance`.

**Spec anchor:** [2026-04-17 conceptual review Finding 9](../archive/reviews/2026-04-16-architecture-review-open-questions.md) — validate architecture before scaling. Supersedes nothing; validates [ADR 0065](../../../thoughts/adr/0065-wos-authoring-stack-mirrors-formspec.md) assumptions.

---

## Why this spike exists

The full architecture committed in ADR 0065 assumes six pressures justify six crates. Those pressures are plausible but untested. A 2–3 day spike:

1. Proves the loop works end-to-end with current `wos-lint` + `wos-conformance`.
2. Surfaces what `wos-mcp` tool handlers actually need (informs Task 3+ of the wos-mcp plan).
3. Surfaces what `wos-authoring` helpers are actually called in practice (trims the 28-helper count to real usage).
4. Validates whether `ToolContext` is genuinely useful or a YAGNI trait.
5. Validates whether the dual-entry MCP pattern adds Rust value or is TS-specific.

The spike's output is NOT production code — it's an experiment whose findings revise the larger plans.

---

## Success criteria

Must be achieved for the spike to count as complete:

1. `cargo run -p wos-synth-spike -- --problem benchmarks/problems/purchase-order-approval.md` emits a valid `$wosWorkflow` JSON document to stdout that passes `wos-lint::lint_document` with zero errors.
2. The code fits in <800 LOC across at most 4 files (including `main.rs`, `loop.rs`, `prompts.rs`, `errors.rs`).
3. Time-to-implement: 2–3 calendar days OR less. If it takes longer, STOP and surface what made it hard — that is a finding.
4. A written retrospective (`thoughts/research/2026-04-2X-wos-synth-v0-spike-findings.md`) answers:
   - How many iterations did the loop need to converge on the PO fixture? (Expected: 1–3.)
   - What was the shape of the repair prompt — text-formatted diagnostics, structured JSON, other?
   - What `wos-authoring`-style helpers would actually have made the LLM's generation easier? (Expected: a handful, not 28.)
   - Was the `ToolContext` abstraction needed? Answer: no, because single crate. But note: where would it have helped? (e.g., benchmarking, caching, remote.)
   - What surprised us?

---

## Completion criteria

1. `crates/wos-synth-spike/` exists with <800 LOC of Rust.
2. `benchmarks/problems/purchase-order-approval.md` exists (NL problem statement, ~200 words).
3. The CLI converges on the PO fixture in ≤5 iterations with `anthropic-sdk`.
4. Retrospective document committed.
5. Decisions from the retrospective propagate into §4.2 / §5.1 / §5.2 / §5.3 / §5.4 / §5.5 / wos-authoring / wos-mcp plans (as inline edits or addendum notes).

---

## File structure

```
crates/wos-synth-spike/
├── Cargo.toml              # deps: wos-core, wos-lint, wos-conformance, anthropic-sdk,
│                           #       reqwest, tokio (full), serde_json, clap, thiserror
└── src/
    ├── main.rs             # CLI: parse --problem arg, drive the loop, write output to stdout
    ├── loop.rs             # synthesize(problem, &client) -> Result<Value, SpikeError>
    ├── prompts.rs          # build_generate_prompt() + build_repair_prompt(); inline strings
    └── errors.rs           # simple SpikeError enum

benchmarks/
└── problems/
    └── purchase-order-approval.md   # ~200 word NL problem statement

thoughts/research/
└── 2026-04-2X-wos-synth-v0-spike-findings.md   # retrospective (date set when spike runs)
```

**`Cargo.toml` dependency block:**

```toml
[dependencies]
wos-core        = { path = "../wos-core" }
wos-lint        = { path = "../wos-lint" }
wos-conformance = { path = "../wos-conformance" }
anthropic-sdk   = "0.1"      # or whichever version is current; pin exactly
reqwest         = { version = "0.12", features = ["json"] }
tokio           = { version = "1", features = ["full"] }
serde_json      = "1"
clap            = { version = "4", features = ["derive"] }
thiserror       = "2"
```

---

## Architecture

The spike is intentionally flat. No traits, no registries, no multi-crate boundaries. The entire logic fits in four files:

```
main.rs
  └─ reads --problem file
  └─ creates reqwest::Client
  └─ calls loop::synthesize(problem_text, &client)
  └─ on Ok(doc): serde_json::to_string_pretty → stdout
  └─ on Err(e): eprintln error → exit 1

loop.rs  (synthesize function)
  1. call prompts::build_generate_prompt(problem, schema_str, bluf_str)
  2. POST to Anthropic /v1/messages — extract text from response
  3. parse response as serde_json::Value
  4. call wos_lint::lint_document(&doc) → Vec<LintDiagnostic>
  5. if errors: call prompts::build_repair_prompt(&doc, &diagnostics)
               repeat from step 2 with repair prompt
               cap iterations at N=5
  6. call wos_conformance::run(&doc)
  7. if conformance errors: one more repair pass using conformance trace
  8. return Ok(doc) or Err(SpikeError::DidNotConverge { iterations })

prompts.rs
  - build_generate_prompt(problem, schema, bluf) -> String
      include_str! for schema JSON and BLUF markdown; inline the problem
  - build_repair_prompt(attempt, diagnostics) -> String
      format diagnostics as plain numbered list; include the prior attempt JSON

errors.rs
  - SpikeError::AnthropicApi(String)
  - SpikeError::JsonParse { attempt: String, source: serde_json::Error }
  - SpikeError::DidNotConverge { iterations: usize, last_diagnostics: Vec<String> }
  - SpikeError::Io(std::io::Error)
```

No `WosProject`. No `ToolContext`. No `ProjectRegistry`. Those are what the spike is testing — they may or may not be needed. The spike finds out by doing without them.

---

## Tasks

Break into 5 small tasks. Each task is a single commit.

### Task 1: Problem statement + spike crate scaffold

**Files:**

- Create: `benchmarks/problems/purchase-order-approval.md`
- Create: `crates/wos-synth-spike/Cargo.toml`
- Create: `crates/wos-synth-spike/src/main.rs` (stub that prints "not implemented")
- Create: `crates/wos-synth-spike/src/errors.rs`
- Modify: root `Cargo.toml` workspace members

- [ ] **Step 1.1:** Write `benchmarks/problems/purchase-order-approval.md`. Content: a ~200 word natural language problem statement for a purchase-order approval workflow. Include: the two actors (requester and approver), the decision fork (direct approval under $50k vs. director review for larger amounts), the four terminal outcomes (approved, rejected, cancelled, returned for revision), and the compliance constraint (all determinations must be logged with actor and timestamp).

- [ ] **Step 1.2:** Create `Cargo.toml` with the dependency block listed above. Set `edition = "2021"`, `name = "wos-synth-spike"`.

- [ ] **Step 1.3:** Create `src/errors.rs` with the `SpikeError` enum (four variants as listed above). Derive `thiserror::Error`.

- [ ] **Step 1.4:** Create `src/main.rs` as a stub: parse `--problem <path>` with `clap`, read the file, print `"not implemented"`.

- [ ] **Step 1.5:** Add `"crates/wos-synth-spike"` to workspace `members` in root `Cargo.toml`.

- [ ] **Step 1.6:** Verify: `cargo build -p wos-synth-spike` passes.

- [ ] **Step 1.7:** Commit:

  ```
  feat(synth-spike): scaffold + problem statement for purchase-order-approval
  ```

---

### Task 2: Hardcoded prompt templates + first LLM call

**Files:**

- Create: `crates/wos-synth-spike/src/prompts.rs`
- Modify: `crates/wos-synth-spike/src/main.rs`

- [ ] **Step 2.1:** Create `src/prompts.rs` with two functions:
  - `pub fn build_generate_prompt(problem: &str) -> String` — embeds the problem text inline, uses `include_str!` to embed the WOS workflow schema JSON and the core spec BLUF markdown. Instructs the LLM to produce a single valid `$wosWorkflow` JSON object and nothing else.
  - `pub fn build_repair_prompt(prior_attempt: &str, diagnostics: &[String]) -> String` — includes the prior JSON attempt and a numbered list of diagnostic messages. Instructs the LLM to correct the errors and return only the corrected JSON.

- [ ] **Step 2.2:** Update `main.rs`: read the problem file, call `build_generate_prompt`, POST to the Anthropic `/v1/messages` endpoint using `reqwest` with the `ANTHROPIC_API_KEY` environment variable, extract the text content from the response, print it to stdout.

- [ ] **Step 2.3:** Verify: `cargo run -p wos-synth-spike -- --problem benchmarks/problems/purchase-order-approval.md` hits the API once and prints LLM output. The output may not be valid JSON yet — that is expected at this task.

- [ ] **Step 2.4:** Commit:

  ```
  feat(synth-spike): first LLM generation pass (no lint/repair yet)
  ```

---

### Task 3: Loop with lint + repair

**Files:**

- Create: `crates/wos-synth-spike/src/loop.rs`
- Modify: `crates/wos-synth-spike/src/main.rs`

- [ ] **Step 3.1:** Implement `pub async fn synthesize(problem: &str, client: &reqwest::Client) -> Result<serde_json::Value, SpikeError>` in `loop.rs`:
  1. Build the generate prompt.
  2. Call the Anthropic API; extract the text response.
  3. Attempt `serde_json::from_str` on the response. On parse failure, treat the raw text as a single diagnostic and go to step 5.
  4. Call `wos_lint::lint_document(&doc)` — collect error-severity diagnostics as strings.
  5. If any errors remain and `iteration < 5`: build the repair prompt from the current attempt + diagnostics, call the API again with the repair prompt, increment iteration, go to step 3.
  6. If zero lint errors: return `Ok(doc)`.
  7. If `iteration == 5` and errors remain: return `Err(SpikeError::DidNotConverge { ... })`.

- [ ] **Step 3.2:** Update `main.rs` to call `synthesize`, serialize the result, and write it to stdout.

- [ ] **Step 3.3:** Verify: the spike converges on the PO problem in ≤3 iterations, or surface what's going wrong — that is a finding worth documenting now. Do NOT keep iterating on prompt engineering past the time budget.

- [ ] **Step 3.4:** Commit:

  ```
  feat(synth-spike): lint-driven repair loop with iteration cap
  ```

---

### Task 4: Add conformance gate

**Files:**

- Modify: `crates/wos-synth-spike/src/loop.rs`

- [ ] **Step 4.1:** After lint passes (zero errors), call `wos_conformance::run(&doc)`. Collect any conformance failures as diagnostic strings.

- [ ] **Step 4.2:** If conformance fails and `iteration < 5`: build one more repair prompt from the conformance diagnostics, call the API, re-run lint + conformance on the new attempt.

- [ ] **Step 4.3:** Accept convergence if BOTH lint and conformance pass. `SpikeError::DidNotConverge` is returned if the cap is hit with either still failing — include which gate failed in the error message.

- [ ] **Step 4.4:** Verify: `cargo run -p wos-synth-spike -- --problem benchmarks/problems/purchase-order-approval.md` produces a JSON document that passes lint + conformance.

- [ ] **Step 4.5:** Commit:

  ```
  feat(synth-spike): conformance gate after lint-pass
  ```

---

### Task 5: Retrospective + plan propagation

**Files:**

- Create: `thoughts/research/2026-04-2X-wos-synth-v0-spike-findings.md` (replace `2X` with actual date)

- [ ] **Step 5.1:** Run the spike end-to-end at least twice with the PO problem. Record: number of iterations to convergence, which diagnostics appeared most frequently, which prompts worked, which did not.

- [ ] **Step 5.2:** Write the retrospective document answering the five questions from the Success Criteria section. Be direct — "ToolContext added no value at this scale" is a valid finding; so is "we needed it immediately for benchmarking." The document's value is in honest observation, not in confirming the plan.

- [ ] **Step 5.3:** For each material finding, propagate a decision into the affected larger plans:
  - If the helper count that actually mattered was well below 28: add an "Addendum (post-spike)" section to `thoughts/plans/2026-04-17-wos-authoring-crate.md` trimming the helper list to the observed set.
  - If `ToolContext` was genuinely not needed in the spike: add a note to `thoughts/plans/2026-04-16-wos-synth-crate.md` flagging it as provisional — implement only when a second consumer arrives.
  - If the dual-entry pattern showed no benefit: note it in `thoughts/plans/2026-04-17-wos-mcp-crate.md` as an architectural assumption to revisit before Task 2 of that plan executes.
  - If the repair prompt shape was different from what the plans assumed: update `thoughts/plans/2026-04-16-wos-synthesis-benchmark.md` to reflect the actual diagnostic format.

- [ ] **Step 5.4:** Commit:

  ```
  docs(research): wos-synth v0 spike findings + plan propagation
  ```

---

## Post-spike: keep or delete?

After findings propagate into the larger plans:

- **Keep the spike crate** as `crates/wos-synth-spike/` if the retrospective surfaces that its integration-test value is high (e.g., it exercises a code path in `wos-lint` or `wos-conformance` that no other test covers). Label it `[spike — do not extend]` in a crate-level comment; treat it as a read-only reference artifact.
- **Delete it entirely** if the larger `wos-synth-core` plan (§5.4) can absorb any test coverage the spike provides. CLAUDE.md prefers deletion.

Default choice: **delete**. The spike was never meant to live past v0; its output is the retrospective and plan revisions, not the code.

---

## Why this matters

A v0 spike is the single cheapest way to validate an architecture before scaling it. ADR 0065 commits WOS to a quarter of engineering work on the strength of structural analogy to Formspec. The spike replaces that structural argument with direct observation. If the architecture is right, the spike confirms it in 2–3 days and we execute the larger plans with high confidence. If the architecture is wrong somewhere — if `ToolContext` is YAGNI, if 28 helpers collapses to 7 in practice, if the dual-entry pattern adds nothing in Rust — we find out now rather than 5 engineer-weeks into `wos-synth-core` Task 5.

**Estimated effort:** 2–3 calendar days. Expected LOC: <800. Expected files: 4 Rust source files + 1 problem statement + 1 retrospective. Result: a set of retrospective findings that revise the larger plans with observed behavior rather than architectural analogy.
