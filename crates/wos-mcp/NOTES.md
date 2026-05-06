# wos-mcp — implementation notes

Decision record for the wos-mcp crate. Captures the load-bearing
choices that aren't obvious from `Cargo.toml` or `src/server.rs` alone.

## Transport: hand-rolled JSON-RPC-2.0 over stdio

The crate ships ~100 LOC of hand-rolled JSON-RPC-2.0 framing in
`src/server.rs` instead of depending on the `rust-mcp-sdk` crate.

**Original rationale (2026-04-17, MCP plan Task 1.2):** `rust-mcp-sdk`
v0.9.0 with default features pulls `hyper` / `axum` / `reqwest` / `ssl` /
`jsonwebtoken` for transports we don't need (SSE, streamable-HTTP,
hyper-server, OAuth). The hand-rolled stdio loop avoids the dependency
weight entirely.

**Rationale retraction (2026-04-18, recorded in `Cargo.toml:13-21`):** the
"too heavy" framing is no longer accurate. A minimal
`rust-mcp-sdk = { version = "0.9", default-features = false, features = ["stdio", "server"] }`
gates the heavy transports behind their own features and pulls only the
stdio transport — verified against the upstream `Cargo.toml`. The hand-
rolled loop remains the current implementation, but the dependency-weight
argument no longer justifies it.

**Current shape:**

- `src/server.rs:30-70` defines `Request` / `Response` / `Error` types
  matching the JSON-RPC-2.0 wire format (`jsonrpc: "2.0"`, `id`, `method`,
  `params`, `result`/`error`).
- The stdio loop reads newline-delimited requests from stdin, routes each
  to `wos_mcp::dispatch::handle(...)`, and writes responses to stdout.
- A single `ProjectRegistry` lives for the lifetime of the process; the
  stdio loop is single-threaded so no synchronization is needed
  (`src/server.rs:470` and the surrounding loop).
- Shutdown is graceful on EOF.

**Why hand-rolled is still in place:** transport-swap is a separate
work item, larger than a dependency-cleanup commit. Migration to
`rust-mcp-sdk` would also need to be exercised against a real MCP host
(e.g. Claude Desktop) before retirement — see TODO.md ADR 0065
**#65f** (Real MCP client validation; v0 spike never touched MCP, so
silence is not proof of dual-entry correctness).

**When to revisit the decision:**

1. The hand-rolled loop diverges meaningfully from JSON-RPC-2.0 — at
   that point ecosystem parity wins.
2. A consumer requires a transport other than stdio (SSE, HTTP) — at
   that point the SDK's transport abstraction earns its weight.
3. A real MCP host (Claude Desktop, MCP Inspector) surfaces a wire-
   compatibility bug the hand-rolled loop doesn't reproduce — that's
   evidence the loop's framing is incomplete and the SDK's reference
   implementation should take over.

If/when that revisit happens: swap to `rust-mcp-sdk` with
`default-features = false, features = ["stdio", "server"]`, retire the
local `Request` / `Response` / `Error` types, and exercise the binary
under a real MCP host before merging. The dependency-weight blocker is
already gone.

## References

- MCP plan: `thoughts/plans/2026-04-17-wos-mcp-crate.md` (Open Questions
  Q-A — transport choice).
- TODO follow-ups: `work-spec/TODO.md` ADR 0065 cluster items
  **#65d** (this note), **#65e** (SDK migration follow-up — `Cargo.toml`
  TODO), **#65f** (real MCP client validation).
- Cargo.toml feature analysis: `work-spec/crates/wos-mcp/Cargo.toml:13-21`.
- Header comment retraction: `work-spec/crates/wos-mcp/src/server.rs:1-5`.

## Production seam (ADR 0065 D-3)

Separate from transport: the dispatch handler at
`crates/wos-mcp/src/dispatch.rs` is the production seam between
LLM-driven authoring (via `wos-synth-core`) and the lint/conformance
tools. The transport choice (hand-rolled vs SDK) is orthogonal to the
seam — both produce the same `dispatch::handle(...)` calls. ADR 0065
D-3 captures the seam direction; transport choice is bounded by this
note plus `Cargo.toml:13-21`.
