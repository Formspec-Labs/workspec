# ADR-0064: Agent as first-class ActorKind; AgentInvoker port

**Status:** Accepted
**Date:** 2026-04-30
**Deciders:** WOS Working Group
**Author:** WOS conformance hardening
**Supersedes:** None
**Amends:** [ADR-0062](0062-signature-profile-workflow-semantics.md) §2 (signer-roles statement that "kernel `ActorDeclaration.type` remains `human | system`" — extended to include `agent`).
**Related:**

- [ADR-0063](0063-embedded-vs-sidecar-identity-boundary.md) -- embedded-vs-sidecar identity boundary
- [`schemas/wos-workflow.schema.json`](../../schemas/wos-workflow.schema.json) -- Actor.type closed enum, agents[] embedded block
- [CLAUDE.md](../../CLAUDE.md) Identity Claim B -- "Agents as first-class runtime actors"

---

## 1. Context

WOS makes two identity claims (per CLAUDE.md):

- **Claim A** -- Workflows are LLM-authorable structured data.
- **Claim B** -- Agents are first-class runtime actors, alongside humans and services, with autonomy levels, confidence gates, deontic constraints, drift monitoring.

The schema (`schemas/wos-workflow.schema.json`) implements Claim B: `Actor.type` accepts `human`, `system`, or `agent`; the `agents[]` embedded block carries per-agent declarations (capabilities, autonomy, deontic constraints, fallback chain, drift monitoring); a top-level `aiOversight` block carries cross-cutting AI policy.

The Rust runtime model in `wos-core` does not yet match. `ActorKind` admits only `Human` and `System`; agent-typed actors deserialize into a non-existent variant or are silently treated as system actors. The generated TypeScript narrows further to `'human' | 'system' | 'retired'`. ADR 0062 §2 even explicitly states "kernel `ActorDeclaration.type` remains `human | system`" -- but ADR 0062 was scoped to the Signature Profile (where signers ARE humans) and never settled the cross-cutting agent-actor question.

Two questions are open:

1. **Modeling.** Is `agent` a first-class `ActorKind` variant alongside `Human` and `System`, or does the spec govern agents purely as a runtime overlay (`agents[]` declarations attached to `system`-typed actors)?
2. **Invocation.** When a workflow assigns work to an agent, how does the runtime invoke it? WOS has multiple production substrates to integrate with: direct LLM call, Claude Agent SDK, MCP servers, A2A multi-agent orchestrators, deployed agent services. The spec must not couple to any single substrate.

---

## 2. Decision

### 2.1 `Agent` is a first-class `ActorKind` variant

`ActorKind` becomes `Human | System | Agent` (closed). Agent-typed actors live in the `actors[]` registry alongside humans and services. The `agents[]` embedded block remains the agent overlay carrying capabilities, autonomy level, deontic constraints, confidence floor, fallback chain, drift monitoring, and an invoker discriminator. Actor and agent are joined by `id`.

This makes accountability symmetric: the same dispatch model that assigns work to a human or service assigns work to an agent. Permissions, oversight checks, deontic-constraint evaluation, and provenance recording all attach uniformly. The whole point of WOS — governing agents as parties with rights and obligations — only coheres when agents are first-class actors. The OASIS LegalRuleML deontic model treats agents as parties, not as configurations.

ADR 0062 §2 is amended: signer roles still bind to `human` actors via the `actorExtension` seam, but `ActorKind` itself widens.

### 2.2 `AgentInvoker` is a substrate-neutral port in `wos-core`

Agent invocation goes through a port:

```rust
pub trait AgentInvoker: Send + Sync {
    fn invoke(
        &self,
        decl: &AgentDeclaration,
        task: AgentTask,
        ctx: AgentContext,
    ) -> impl Future<Output = AgentResult> + Send;
}
```

The `agents[]` block carries an `invoker` discriminator declaring which substrate each agent uses (e.g., `{kind: "anthropic", model: "claude-opus-4-7"}`, `{kind: "claudeAgentSdk", configRef: "..."}`, `{kind: "mcp", server: "...", tool: "..."}`, `{kind: "a2a", orchestrator: "..."}`, `{kind: "http", endpoint: "..."}`, `{kind: "stub", responses: [...]}`). Deployment binds the discriminator to a concrete adapter.

Adapters live in separate crates so the dependency graph reflects the substrate boundary:

- `wos-agent-stub` -- deterministic in-memory adapter for conformance fixtures.
- `wos-agent-anthropic` -- direct Anthropic SDK call.
- `wos-agent-sdk-claude` -- Claude Agent SDK.
- `wos-agent-mcp` -- MCP server invocation.
- `wos-agent-a2a` -- A2A multi-agent orchestrator.
- `wos-agent-http` -- generic HTTP/OpenAPI service.

Multi-agent orchestrators are *one kind* of `AgentInvoker`, not a special concept in WOS. The runtime holds either a single `Box<dyn AgentInvoker>` (for single-substrate deployments) or a discriminator-keyed map (for multi-substrate deployments) and dispatches per agent declaration.

This mirrors the established WOS port/adapter posture:

- `DurableRuntime` trait (in-memory, Restate, Temporal adapters).
- `EventStore` trait per `wos-server` end-state vision (Postgres/Trellis adapters).
- `ContractValidator` (Formspec, JSON Schema, stub adapters).

`AgentInvoker` joins the same pattern as the third spec-authoritative port.

### 2.3 Cross-reference enforcement

When `actors[].type == "agent"`, a matching `agents[].id` MUST exist. This is already enforced by lint rule `WOS-AGENT-XREF-001`; ADR 0064 promotes the rule from SHOULD to MUST given `Agent` is now a closed `ActorKind` variant rather than an open-string extension.

---

## 3. Rejected Alternatives

### Keep `Agent` out of `ActorKind`; declare agents purely as `agents[]` overlay

Rejected. The agent-as-overlay model treats agents as configurations of system actors, but WOS governance treats agents as accountable parties. Mixing those two stances forces every WOS rule that talks about "actors" to disambiguate between human/service actors and agent overlays, fragmenting the dispatch model and the deontic constraint surface.

### Make `ActorKind` an open string with `actorExtension` validation

Rejected. The schema already closed the enum to `["human", "system", "agent"]`. An open string would re-create the validator/runtime drift this ADR resolves and would let third parties invent actor types that bypass governance.

### Bake one substrate into `wos-runtime`

Rejected. WOS deploys across SBA (small benefits agency), federal-tier (FedRAMP-bounded), and sovereign-tier (in-house) postures. Each posture may use a different agent invocation substrate (direct LLM, MCP-mediated, hardware-token-protected service). A baked-in choice would force every deployment to take dependencies it doesn't want and prevent the substrate-portability that's a Q1 design goal.

### Define each substrate as a separate `Actor` variant

Rejected. `ActorKind` is a permissioning/dispatch concept, not an integration concept. Substrate selection lives at the `agents[].invoker` discriminator and `AgentInvoker` adapter level.

---

## 4. Consequences

### Positive

- Schema, Rust, generated TypeScript, and conformance fixtures align around the same closed `Human | System | Agent` enum.
- Cross-actor handoff (human↔agent, agent↔agent) uses the same dispatch model; provenance emits uniformly.
- `AgentInvoker` port lets WOS stay portable across agent-orchestration architectures. Multi-agent platforms (A2A, AutoGen, CrewAI, OpenAI Swarm) integrate as adapters, not as special spec features.
- Deontic constraint evaluation, autonomy capping, and drift monitoring attach to a single accountable identity rather than an actor-config split.

### Negative

- Existing Rust code that pattern-matches `ActorKind` exhaustively must add an `Agent` arm. Lint rule `WOS-AGENT-XREF-001` tightens from SHOULD to MUST.
- Adapter crates (`wos-agent-anthropic`, etc.) need workspace skeletons. This ADR scopes only `wos-agent-stub` for the initial PR; production adapters land as follow-up crates.
- Generated TypeScript regen (Sub-PR E) must update studio call sites that previously narrowed to `'human' | 'system'`.

### Neutral

- ADR 0062 stands; signer roles still attach to `human` actors. Adding `Agent` to the enum does not introduce agent-as-signer; signer-actor binding is governed by the Signature Profile's role declarations.
- Existing `actorExtension` seam (one of the six canonical seams per ADR 0077) is unchanged. Domain-specific actor sub-roles (`signer`, `notary`, `witness`, `caseworker`, `caseManager`) continue to attach via that seam.

---

## 5. Implementation Notes

This ADR ships in three sub-PRs:

- **Sub-PR B (this PR's lighter half)** -- adds `Agent` variant to `ActorKind`, tightens lint, and lands the unit tests. The `AgentInvoker` port and adapter crates land in Sub-PR C.
- **Sub-PR C** -- `wos-core::agent` module with `AgentInvoker` trait, `AgentDeclaration`, `InvokerSpec` discriminator, and the `wos-agent-stub` adapter. Production-adapter crates (`wos-agent-anthropic`, etc.) ship as workspace skeletons with `unimplemented!()` bodies; their first real impls are tracked in `TODO.md` follow-ups.
- **Sub-PR E** -- generated TypeScript regen picks up the closed `Human | System | Agent` enum. Studio call-site updates remove any narrowing to `'human' | 'system'`.

The schema-level `Actor.type` constraint already accepts `agent`. The Rust enum widening is the load-bearing change.
