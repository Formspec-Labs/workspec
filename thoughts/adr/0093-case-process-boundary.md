# ADR 0093 — Case / Process Boundary and Case Projection Introduction

## 1. Status

Proposed

## 2. Context

The foundational architecture of the Formspec/WOS/Trellis stack has correctly separated Formspec (data/evidence collection) from WOS (process governance/routing). However, a critical conflation exists at the product level: the durable domain `Case` is currently modeled directly as a `CaseInstance` (a running WOS workflow execution).

Treating `CaseInstance` as the root product abstraction is a structural flaw. Real-world matters (cases) have a lifespan that often exceeds any single workflow. A case may be created manually without an active workflow, it may spawn multiple workflows over time (e.g., initial intake, then an appeal later), and its status does not inherently equal the lifecycle state of a currently active workflow.

By conflating the two, the system risks bloating WOS `caseState` into a junk-drawer for case-domain data (notes, participants, related cases) and forcing product-level case management features into the WOS kernel layer, violating the stack's defined boundaries.

Furthermore, introducing a `Case` aggregate must not violate the zero-trust architecture defined in `VISION.md` by creating a new, parallel authoritative database store. Trellis remains the sole authoritative integrity spine.

## 3. Decision

We will introduce the `Case` as a distinct, first-class domain aggregate that sits *above* WOS and is populated as a projection of the Trellis Case Ledger.

### 3.1. The Architecture Triad

The system will strictly adhere to the following composition across layers:
1. **Trellis (The Ledger):** Authoritative linear order for committed canonical events and integrity artifacts (`trellis-core.md` §23.2.5). The Case Ledger anchors WOS governance events and Formspec intake heads.
2. **WOS (The Instrument):** Governs case identity, emits the foundational `case.created` provenance event (ADR-0073 D-1), and executes workflow processes (now explicitly tracked as `CaseProcess`) within the shell of a governed case.
3. **Case (The View/Projection):** A domain-level metadata projection. It is not an authoritative store. It is materialized in the operator's `projections` schema by replaying the case-scoped Trellis event stream.

### 3.2. Case Projection and Privacy (ADR-0074 Alignment)

To uphold the SBA strict per-class encryption mandate (ADR-0074), the `Case` projection schema will contain **metadata only** (e.g., ID, case type, top-level status, timestamps, and TypeID links to processes).
* Domain content—such as `notes`, `artifacts` (Formspec responses), and `decisions`—are treated as subresources.
* In the routine read path (and required for the end-state posture), these subresources are fetched as ciphertext plus wrapped key-bag fragments and decrypted client-side.
* For the prod-MVP (per `GOAL.md` and `wos-server/VISION.md`), audited server-side decryption for bounded processing is permissible, provided the API never flattens classified bodies directly into the root `Case` projection document.

### 3.3. Identity and Naming (TypeID Prefixing)

We will not reuse the `case_` TypeID prefix for the new `Case` aggregate, as reusing it would break parse-time safety (`schemas/api/_common.schema.json` `WosResourceUrn`) and cause silent drift in existing Trellis export bundles and provenance records.
* The workflow instance will retain the `case_` prefix (internally conceptualized as `CaseProcess` or `WorkflowInstance`).
* The new domain `Case` aggregate will be minted under a net-new family prefix: **`casefile_`** (or `matter_` / `cf_`).

*Note on `$wosCaseInstance`:* For Phase 1, we will retain the `$wosCaseInstance` JSON marker as a legacy identifier to prevent massive schema version-bump churn across fixtures, `wos-lint`, and conformance traces. A hard rename to `$wosProcessInstance` is deferred until a coordinated breaking-change window.

### 3.4. Governed Output Commit Pipeline (ADR-0080 Extension)

Workflow processes must be able to mutate the Case domain, but they must do so securely. We will not invent a new, seventh kernel extension seam.
Instead, we will evolve the existing declarative pipeline shape defined in **ADR-0080** and `$defs/OutputBinding` (Kernel §9.2.22).
* The `OutputBinding` schema will be extended to support a `target` discriminator (or equivalent `writeScope` partition).
* Targets will include `processCaseState` (the current behavior, scoped to the workflow) alongside new domain targets like `caseArtifact`, `caseDecision`, or `caseTimeline`.
* This requires a corresponding bifurcation in Kernel §5 to explicitly distinguish between process-scoped and case-scoped `caseState`.

### 3.5. Case Origination

WOS remains the sole layer authorized to emit `case.created`. To support manual case creation (zero workflows attached) without bypassing WOS:
* We will draft a follow-up architecture decision (**ADR-0073-bis**) to formalize the "manual case creation" path.
* This path will define the actor surface, authority chain, and payload shape necessary to emit a valid `case.created` event into the canonical ledger when no `IntakeHandoff` or `WorkflowDocument` is bound.
* *Prerequisite:* Any new WOS events (`wos.case-closed`, `wos.artifact-attached`) must be registered in the bound `event_type` registry per Trellis Core §23.2 and §14 before emission.

## 4. Consequences

### Positive
* **Boundary Integrity:** Prevents workflow state machines from becoming bloated CRM databases.
* **Architectural Consistency:** Reaffirming Case as a *projection* of the Trellis ledger maintains the zero-trust, event-sourced vision of the stack.
* **Security Compliance:** By explicitly mapping Case metadata vs. Case subresources, we prevent accidental plaintext leakage of classified domain content (ADR-0074).
* **Flexibility:** Allows the product to support cases that outlive workflows, cases with multiple parallel workflows, and manually created cases.

### Negative / Complexity
* **Dual-State Crash Recovery:** Introduces a new failure mode. A process crash mid-mutation could leave the Trellis chain committed but the Case projection stale. Recovery mechanics (ADR-0070) must eventually account for syncing the projection.
* **Implementation Overhead:** Requires updates to schemas (`OutputBinding`), Kernel §5, routing, and the introduction of a new projection materialization engine.
* **Testing Burden:** The projection mechanism requires rigorous empirical validation (fixture tests asserting the projection accurately and idempotently rebuilds state from a mocked Trellis event stream).

## 5. Alternatives Considered

### Alternative 1: Case as a WOS-Centered Domain Model
Instead of treating `Case` strictly as a projection, we could model `Case` as a primary WOS domain entity (similar to how `CaseInstance` currently operates, carrying a `serde_json::Value` for state) whose mutations produce Trellis events via `custodyHook`.
* *Why Rejected:* While this pattern is used for workflow instances, elevating it to the root product matter risks duplicating the source of truth. If WOS maintains an authoritative database representation of the Case *and* Trellis maintains the event chain, we lose the strict "Trellis is the only canonical store" zero-trust guarantee. The projection pattern forces the system to treat the Trellis ledger as the singular truth.

### Alternative 2: Reassign `case_` TypeID to the New Aggregate
We could reassign the `case_` prefix to the new `Case` aggregate and migrate workflow instances to `process_`.
* *Why Rejected:* Silent drift. Existing Trellis exports, cryptographic signatures, and provenance records have etched the `case_` prefix into immutable history as pointing to a workflow instance. Reusing the prefix for a different conceptual entity is a critical data corruption risk in an event-sourced system.

## 6. Execution Plan (Phase 1: MVP Realignment)

To validate the boundary isolation while minimizing risk to the seed deployment, Phase 1 will target the following:

1. **New Prefix:** Define the `casefile_` (or similar) TypeID prefix for the Case aggregate.
2. **Schema & Kernel Updates:** Ship the `$defs/OutputBinding` `target` discriminator extension alongside the Kernel §5 bifurcation.
3. **Projection MVP:** Define the basic Case projection schema (metadata only) and implement the materialization logic with fixture-backed idempotency tests. Explicitly map Formspec versioning (`definitionUrl` + `definitionVersion` + ADR-0074 profile) in `CaseArtifact` payloads to ensure projections don't crash on replay if form definitions evolve.
4. **Boundary Validation (Asymmetric State):** To prove the decoupling works, MVP testing MUST include at least one asymmetric state: either a manually created Case with zero active processes (pending ADR-0073-bis), or a Case that remains "open" while its bound `CaseProcess` completes. A strict 1:1 constraint provides false confidence and will not be enforced as an ontology rule (though it may be the primary deployment profile).
5. **API Compatibility:** Ensure `/instances` routes remain functional (aliased or mapped to `CaseProcess`) and resolve any existing drift between OpenAPI and `workspec-server` regarding lifecycle enums and routing paths.