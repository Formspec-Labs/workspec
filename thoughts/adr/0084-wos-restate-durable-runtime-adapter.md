# ADR 0084 — WOS durable runtime adapter (Restate, `restate-sdk`)

**Status:** Accepted — authoring stack (ADR → spec → plan) complete 2026-05-01; implementation proceeds under WS-094 in phases.

**Date:** 2026-05-01
**Production state authority refined:** 2026-10-08. This decision does not qualify the implementation or convert historical fixture state.

## Context

- [`work-spec/crates/wos-server/VISION.md`](../crates/wos-server/VISION.md) §IV Story 2 positions **`runtime-restate`** as the production durable execution adapter and **`runtime-local`** as the test / conformance oracle; conformance SHOULD eventually agree across both.
- **WS-094** in [`work-spec/crates/wos-server/TODO.md`](../crates/wos-server/TODO.md) tracks the `wos-server-runtime-restate` crate: **in-memory** [`RuntimeOps`](../../crates/wos-server-ports/src/runtime.rs) plus **ingress** to the `WosInstance` VO (durable handlers / Restate journal for that path). CI **`wos-restate-ingress-smoke`** exercises create/load/probe against a pinned Restate Server (**2026-05-01**). **Still open on the row:** `SeamAccess` / task APIs / `migrate_instance`, and PLN-0333 three-way conformance vs `runtime-local` + retry/stall acceptance.
- The **WOS-T3 spike** ([`thoughts/reviews/2026-04-21-wos-t3-durable-runtime-temporal-restate-spike.md`](../reviews/2026-04-21-wos-t3-durable-runtime-temporal-restate-spike.md)) chose **Restate first** for production Rust durability because the official **[`restate-sdk`](https://docs.rs/restate-sdk/latest/restate_sdk/)** exposes durable handlers, virtual objects, workflows, journaling, timers, and awakeables aligned with WOS command semantics.
- **Failure / compensation vocabulary** for cross-stack alignment remains repo-root [**ADR 0070**](../../../thoughts/adr/0070-stack-failure-and-compensation.md); this ADR does not fork those definitions — it binds how the Restate adapter surfaces **retry vs terminal** behavior at the WOS boundary.
- **Normative adapter obligations** (processor-visible “MUST” rules) live in the working spec: [`thoughts/specs/2026-05-01-wos-restate-durable-runtime-adapter-spec.md`](../specs/2026-05-01-wos-restate-durable-runtime-adapter-spec.md) (promotion into canonical `work-spec/specs/` is optional and gated on ratification workflow, not required for WS-094 execution).

## Decisions

### D1 — Restate service shape: Virtual Object per WOS instance

**Decision:** Model each **governed workflow instance** as a **Restate Virtual Object** keyed by a stable **`process_id`** string (the same identifier used in `RuntimeOps` and `Storage`).

**Rationale:** Exclusive handler semantics per key match WOS single-writer drain expectations. Restate journals and schedules execution; PostgreSQL owns the committed operational process record under D3. Restate K/V MAY retain execution references but MUST NOT become a second writable production process snapshot. [`#[restate_sdk::object]`](https://docs.rs/restate-sdk/latest/restate_sdk/#virtual-objects) with optional `#[shared]` handlers covers read/query vs write paths (see spec).

**Rejected for v1 default:** A single global **Service** without keyed serialization — would require external locking to match WOS ordering. **Workflow** as the *sole* representation of an instance remains a **future ADR slot** if a product line needs `run`-once lifecycle with different fault domains; the spike noted workflows as an alternative mapping.

### D2 — Process topology: Axum `wos-server` and Restate `HttpServer` are distinct

**Decision:** The **`restate_sdk`** worker exposes services through **`HttpServer`** / `Endpoint` on a **configurable bind address** (sidecar or co-process). **`wos-server`** Axum remains the operator HTTP surface; it **invokes** Restate via the **ingress URL** (HTTP client) or in-process client where the SDK allows — exact wiring is an implementation detail fixed in the [execution plan](../plans/2026-05-01-wos-restate-ws094-execution.md).

**Rationale:** Matches Restate’s documented serving model ([*Serving*](https://docs.rs/restate-sdk/latest/restate_sdk/#restate-rust-sdk)) and avoids merging two HTTP stacks into one listener without a ratified reverse-proxy story.

### D3 — One committed operational authority and separate execution/custody stores

**Decision:** For the selected production Restate adapter, PostgreSQL through the maintained WOS storage/runtime ports is the sole writable authority for committed operational process state, including task state, pending native events/callbacks, operational provenance and retained command outcomes. Restate owns execution journals, invocation ordering and durable scheduling; it MUST NOT maintain an independently writable production process snapshot. Trellis owns accepted canonical custody records and export; SQL commit and Restate journal completion do not constitute Trellis custody commitment.

Each retryable mutating command MUST bind a closed operation, stable command identity, selected deployment/scope and process/case/task targets, immutable request digest and authored artifact identity. The owning transaction MUST commit the command's native operational mutation, required provenance/custody-state updates, every immutable per-recipient delivery intent and its lossless typed outcome together, or commit none of them. Matching replay returns the retained outcome; changed immutable input conflicts. Unknown transaction completion MUST be settled by exact retained outcome lookup and ownership/fencing before another effective execution. Absence alone is not proof that an unresolved attempt failed.

All production mutations MUST use one maintained admitted native executor. Current authority MUST be checked at each actual effect attempt and protected output MUST be admitted again before disclosure, including journal-cached results. Credentials MUST NOT be serialized into journals; worker-local ownership, credential-reference resolution and service authority MUST be explicit and fail closed. Delivery transport MUST occur after domain commit, outside its transaction; transport attempts and qualifying producer-admitted receipt/case-clock events remain governed by [ADR 0144](../../../thoughts/adr/0144-delivery-channel-adapter-and-notification-semantics.md) and ADR 0067.

**Rationale:** One operational record supports atomic task/case changes, delivery intents and command recovery. Execution journals provide replay and scheduling; canonical custody remains a separately admitted Trellis boundary. The former SQL query-view decision is superseded for this selected production adapter. Historical K/V fixtures remain test evidence, not an alternative production authority or an authorization to convert existing objects.

### D4 — `RuntimeOps` ↔ `DurableRuntime` / `WosRuntime` bridge

**Decision:** Production path **embeds** [`WosRuntime`](../../crates/wos-runtime/src/runtime.rs) (or calls through **`DurableRuntime`** where the type bound is already satisfied) **inside** exclusive Virtual Object handlers that perform **journal-safe** steps only. The admitted executor reads and commits the D3 PostgreSQL operational record; **non-deterministic** I/O uses the SDK’s journaled APIs ([*Journaling Results*](https://docs.rs/restate-sdk/latest/restate_sdk/#restate-rust-sdk), state APIs). The existing **`RestateRuntimeAdapter`** may remain the **`RuntimeOps`** façade that forwards to ingress **until** in-process embedding is proven stable.

**Rationale:** Reuses kernel evaluation, bindings, and provenance stamping already centralized in `wos-runtime`; duplicating semantics in raw handlers would rot.

### D5 — Errors: terminal vs retryable

**Decision:** Governance violations and client contract failures that today surface as non-retryable WOS errors MUST map to **`HandlerError` / terminal** paths in `restate-sdk` so Restate does not retry forever ([*Error Handling*](https://docs.rs/restate-sdk/latest/restate_sdk/#restate-rust-sdk)). Transient infrastructure errors remain retryable per SDK defaults unless ADR 0070 narrows a specific class.

## Consequences

- Executable work is sequenced in [`thoughts/plans/2026-05-01-wos-restate-ws094-execution.md`](../plans/2026-05-01-wos-restate-ws094-execution.md) and tracked in **WS-094**; parent [**PLN-0333**](../../../PLANNING.md) continues to index this ADR + WS-094.
- **`restate-sdk`** becomes a normal dependency of `wos-server-runtime-restate` once Phase 0 of the plan lands; Restate Server version pairing is pinned in the execution plan.
- Promotion of [`thoughts/specs/2026-05-01-wos-restate-durable-runtime-adapter-spec.md`](../specs/2026-05-01-wos-restate-durable-runtime-adapter-spec.md) into `work-spec/specs/` is **out of scope** for this ADR unless a separate doc-ratification pass is opened.

## References

- [`restate-sdk` on docs.rs](https://docs.rs/restate-sdk/latest/restate_sdk/)
- [`work-spec/crates/wos-server/VISION.md`](../crates/wos-server/VISION.md) §IV Story 2
- [`work-spec/crates/wos-server/TODO.md`](../crates/wos-server/TODO.md) — **WS-094**
- [`thoughts/reviews/2026-04-21-wos-t3-durable-runtime-temporal-restate-spike.md`](../reviews/2026-04-21-wos-t3-durable-runtime-temporal-restate-spike.md)
- [`wos-runtime::DurableRuntime`](../../crates/wos-runtime/src/durable.rs)
- Repo-root [`thoughts/adr/0070-stack-failure-and-compensation.md`](../../../thoughts/adr/0070-stack-failure-and-compensation.md)
