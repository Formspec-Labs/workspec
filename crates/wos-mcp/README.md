# wos-mcp

MCP adapter exposing WOS authoring operations as JSON-RPC-2.0 tools.

## Two audiences, one set of handlers

`wos-mcp` exposes the same tool handlers to two audiences:

1. **External MCP clients** (Claude Desktop, Cline) via the `wos-mcp` stdio binary wrapping JSON-RPC-2.0.
2. **In-workspace Rust callers** (`wos-synth-core`, `wos-bench`, tests) via direct import of `wos_mcp::dispatch::dispatch`.

No protocol round-trip for in-process callers; they call the async function directly.

```rust
// External: JSON-RPC-2.0 over stdio (launched by Claude Desktop / Cline)
// $ echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"wos_ping","arguments":{}}}' | wos-mcp

// In-process: direct library call (zero overhead)
let result = wos_mcp::dispatch::dispatch("wos_ping", serde_json::json!({})).await?;
assert_eq!(result, serde_json::json!({"pong": true}));
```

## MCP transport decision

**Hand-rolled** — `rust-mcp-sdk` v0.9.0 exists on crates.io but pulls in hyper, axum, reqwest, SSL, and tokio-stream as default features. That is far too heavy for a pure stdio adapter where the protocol is 3 methods (`initialize`, `tools/list`, `tools/call`) over newline-delimited JSON. The hand-rolled transport in `src/server.rs` is ~130 lines, has no ecosystem risk, and can be upgraded to an SDK if the ecosystem stabilises.

## Running

```bash
# Build
cargo build -p wos-mcp

# Smoke test
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | cargo run -q -p wos-mcp

# Full test suite (unit + subprocess integration)
cargo nextest run -p wos-mcp
```

## Architecture

```
src/
  lib.rs          — public module declarations
  server.rs       — [[bin]] stdio JSON-RPC-2.0 loop
  dispatch.rs     — in-process dispatch entry point (library API)
  registry.rs     — ProjectRegistry (stub; expanded in Task 3)
  errors.rs       — ToolError, DispatchError
  tools/
    mod.rs        — re-exports all tool handlers
    ping.rs       — wos_ping health-check tool
tests/
  stdio_transport.rs — subprocess integration tests
```

## Adding a new tool

1. Create `src/tools/<name>.rs` with `pub async fn <name>(args: Value) -> Result<Value, ToolError>`.
2. `pub use self::<name>::<name>;` in `src/tools/mod.rs`.
3. Add a match arm in `src/dispatch.rs`.
4. Add the tool descriptor to `tools/list` in `src/server.rs`.
