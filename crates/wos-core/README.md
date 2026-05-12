# wos-core

`wos-core` is the typed domain model and evaluation kernel for WOS processors.

It sits below linting, conformance, and any future runtime adapter. The crate owns:

- typed Rust models for kernel, governance, AI integration, and sidecar documents
- deterministic lifecycle evaluation over typed state, not raw `serde_json::Value`
- shared provenance, timer, deontic, autonomy, confidence, and explanation logic
- host interface traits that bind the pure evaluator to storage and external systems

## Architecture

The crate boundary is intentionally narrow:

```text
                 wos-lint
                    |
                    v
                wos-core
               /   |    \
              /    |     \
             v     v      v
      wos-conformance  future runtime adapters
```

- `wos-lint` deserializes documents into `wos-core` models and runs static checks against typed fields.
- `wos-conformance` executes fixtures through the shared evaluator and asserts on transitions and provenance.
- Runtime adapters are expected to provide storage, queueing, task execution, signing, and external-service integrations through traits defined in this crate.

## Typed Model

The crate exports the main WOS document types from [`src/lib.rs`](./src/lib.rs):

- `KernelDocument`: lifecycle structure, actors, case file, contracts, execution config, and version-pinned runtime fields
- `GovernanceDocument`: due process, pipelines, delegation, hold policies, and other Layer 2 governance controls
- `AIIntegrationDocument`: agent declarations, deontic constraints, autonomy controls, confidence policy, fallback configuration, and proxy governance
- `BusinessCalendarDocument` and `NotificationTemplateDocument`: sidecars used by governance and runtime behavior
- `Project`: a typed multi-document container used for cross-document resolution

The goal is to keep deserialization at the boundary. Once a consumer has loaded documents, the evaluation hot path works on typed structs and enums instead of re-walking ad hoc JSON.

## Evaluation Algorithm

The evaluator entry points live in [`src/eval.rs`](./src/eval.rs) and are exposed through:

- `Evaluator`
- `Configuration`
- `ObservedTransition`
- `IndexedState`
- `EvalContext`

At a high level, the runtime flow is:

1. Load the typed kernel and workflow process state.
2. Build the indexed state view used for deterministic transition lookup.
3. Process one event at a time against the active configuration.
4. Evaluate guards through FEL using `EvalContext`.
5. Fire matching transitions, execute actions, and update timers and provenance.
6. Re-evaluate when `evaluationMode` requires continuous convergence handling.

Supporting modules handle the rule families that are shared across processors:

- [`src/deontic.rs`](./src/deontic.rs): permission, prohibition, obligation, and bypass enforcement
- [`src/autonomy.rs`](./src/autonomy.rs): effective autonomy caps, escalation, demotion, calibration expiry
- [`src/confidence.rs`](./src/confidence.rs): confidence report validation, decay, cumulative thresholds
- [`src/event_handler.rs`](./src/event_handler.rs): governance, fallback, delegation, DCR, and sidecar-driven runtime handling
- [`src/eval_mode.rs`](./src/eval_mode.rs): event-driven versus continuous re-evaluation
- [`src/explain.rs`](./src/explain.rs): explanation assembly from provenance
- [`src/timer.rs`](./src/timer.rs): pending timer tracking
- [`src/provenance.rs`](./src/provenance.rs): normalized provenance record types

## Host Trait Interfaces

`wos-core` keeps the evaluator pure by pushing deployment concerns behind traits in [`src/traits/mod.rs`](./src/traits/mod.rs). These traits correspond to Runtime Companion S12:

- `InstanceStore`: durable load/save of `WorkflowProcess`
- `DocumentResolver`: version-aware document loading
- `ContractValidator`: Formspec or schema validation for contracts
- `ExternalService`: controlled service invocation for `invokeService`
- `AccessControl`: actor transition and read permissions
- `ProvenanceSigner`: signing and verification of provenance records
- `ReportRenderer`: rendering human-readable explanations
- `EventQueue`: per-instance queue operations
- `ActionExecutor`: host-managed actions such as task creation

`DefaultRuntime` provides an in-memory implementation for the small subset needed by tests and single-process experiments. Real deployments are expected to supply their own implementations around durable storage and controlled service boundaries.

## Conformance Profile Guidance

This crate implements processor behavior. It does not, by itself, prove that a deployment satisfies every runtime or AI profile claim.

### Runtime S13 Security Model

- `S13.1 Engine Isolation`: MUST be demonstrated by architecture. The evaluator must not have direct network access. In practice, a conformant host wires all outbound effects through `ExternalService` and keeps the engine process or library free of ambient network clients.
- `S13.2 Expression Sandboxing`: satisfied by FEL design, not host configuration. A host demonstrates compliance by using the shipped FEL evaluator rather than adding out-of-band side effects to expression execution.
- `S13.3 Data Protection`: SHOULD be demonstrated by the `InstanceStore` implementation and deployment controls, such as encrypted database storage or volume encryption.
- `S13.4 Provenance Immutability`: SHOULD be demonstrated by the provenance store design, such as append-only tables, object-lock storage, or signed append logs. Because the spec permits legally required expungement, this remains a host policy and storage concern rather than an engine-enforced invariant.

Evidence for these claims should come from:

- code review of the runtime adapter layer
- deployment configuration and storage architecture
- conformance fixture results for engine behavior
- operator documentation describing which host interfaces back each requirement

### AI-004 and AI-050 Verification Strategy

Two AI-related claims remain architectural rather than fixture-local:

- `AI-004`: a processor that delegates Formspec evaluation must delegate to a conformant Formspec processor
- `AI-050`: the Assist Governance Proxy must not weaken or rewrite Assist conformance requirements

The current verification strategy is:

1. Prove local engine behavior with fixture-based conformance tests.
2. Exercise the delegated validator seam and proxy differential harness in `wos-conformance` so `AI-004` and `AI-050` are backed by observed behavior rather than prose-only self-declaration.
3. Record which external Formspec processor or Assist implementation the host delegates to.
4. Review the adapter layer to confirm it forwards validation or proxy operations without semantic modification.

These rules become fully automatable only when there is either:

- a second processor implementation to run differential tests against, or
- proxy instrumentation that can compare pre-proxy and post-proxy behavior at the protocol boundary

Until then, the expected evidence is a mix of fixture results, behavior-level adapter/proxy checks, architectural review, and deployment self-declaration.

## Testing

The crate is exercised through both direct unit tests and downstream integration tests:

- [`tests/`](./tests): typed deserialization and evaluator behavior
- `wos-conformance`: end-to-end fixture execution over the shared evaluator
- `wos-lint`: typed-model consumers that validate structural and cross-document rules

For targeted work, run:

```bash
cargo nextest run -p wos-core
cargo nextest run -p wos-conformance
cargo nextest run -p wos-lint
```
