# Case Management Architecture: Aggregate Analysis Report

> **Historical synthesis.** This report is retained as analysis context. The controlling source of truth for case-management decisions is [`case-boundary-decision-report.md`](case-boundary-decision-report.md), not this aggregate report.

**Context:** This report synthesizes the architectural exploration, multi-agent validation, and structural discoveries documented across the `case-management` lifecycle. It extracts the pure architectural analysis from the procedural decisions, providing a comprehensive assessment of the system's boundaries, constraints, and realized structures.

**Source Corpus:**

- `case-management.md` (Original Consultant Proposal)
- `case-management-validation-claude-opus-4-7-1m.md`
- `case-management-validation-claude-opus-4.7.md`
- `case-management-validation-gemini.md`
- `case-management-validation-glm-5.1.md`
- `case-management-validation-gpt-5-codex.md`
- `case-management-aggregate-synthesis.md` (v2 Synthesis)
- `case-boundary-decision-report.md` (Final Assessment)

---

## 1. The Core Architectural Tension

The analysis phase began by observing a fundamental conflation within the WOS (Workflow Orchestration Standard) architecture: **the `WorkflowProcess` object was bearing the weight of two distinct domain concepts.**

1. **The Execution Artifact:** The ephemeral, runtime state of a specific workflow process (timers, active tasks, current lifecycle state).
2. **The Real-World Matter:** The durable, long-lived domain object representing the overarching issue (e.g., a fraud investigation, a benefit application) which may span months and encompass multiple distinct workflows.

The initial proposal (`case-management.md`) accurately diagnosed this tension and recommended separating the two concepts by introducing a new, first-class `Case` aggregate to sit above the WOS workflow engine, relegating the workflow engine execution to a mere `CaseProcess`.

## 2. Validation Findings: The Constraints of Reality

Extensive cross-stack validation by multiple analytical models revealed that while the *symptom* (conflation) was correctly identified, the *proposed cure* (a separate `Case` aggregate) severely violated existing structural and security invariants of the broader system.

### 2.1 The Single Source of Truth Constraint (Trellis)

*Ref: `case-management-validation-claude-opus-4-7-1m.md`, `case-management-validation-glm-5.1.md`, `case-management-validation-gemini.md`*

The system's zero-trust architecture relies on the **Trellis Case Ledger** as the singular, cryptographically anchored, append-only source of truth. Attempting to introduce a `Case` aggregate as a *second* durable store created immediate contradictions. As identified by the validation passes:

- The Trellis chain anchors at the case level.
- Any "Case" object exposed to the product layer must strictly be a **read-side projection** (a derived view materialized from event replay), not a distinct, authoritative database table or aggregate root.
- Projection lag or dual-state crash recovery are eliminated as failure modes when the ledger is recognized as the sole authority.

### 2.2 Security and Content Visibility (ADR-0074)

*Ref: `case-management-validation-claude-opus-4-7-1m.md`, `case-management-validation-glm-5.1.md`*

The original proposal suggested an API returning a `Case` object heavily populated with `notes`, `communications`, and `artifacts` in plaintext. The validation models identified this as a critical violation of per-class encryption protocols. In this architecture:

- Servers are brokers, clients decrypt.
- A Case projection cannot store or serve plaintext content; it can only serve opaque references and key-bag fragments.
- This constraint further reinforces that the "Case" is a thin operational view, not a data-heavy aggregate.

### 2.3 The Governance of Origination (ADR-0073)

*Ref: `case-management-validation-claude-opus-4.7.md`, `case-management-validation-claude-opus-4-7-1m.md`*

The proposal theorized that a Case could be created manually, entirely outside the scope of a workflow. However, system invariants dictate that WOS exclusively owns the emission of the `case.created` boundary event.

- Bypassing WOS to create cases would violate established governance boundaries.
- The act of creating a case—even an ad-hoc, zero-process one—must flow through governed event emission paths to land on the Trellis ledger.

### 2.4 Uncovering Fake Primitives

*Ref: `case-management-validation-gpt-5-codex.md`*

Adversarial validation passes revealed that endpoints which appeared to be direct ledger-append surfaces (like `POST /instances/{id}/events`) were, in reality, secretly driven by workflow queues and state diffs. This proved that file-level validation is insufficient; the architectural behavior of the endpoint is what dictates its capabilities. Ad-hoc ledger mutations require genuinely distinct, workflow-agnostic write paths.

## 3. The Architectural Collapse: Case = Ledger

*Ref: `case-management-aggregate-synthesis.md`*

The tension between the need to separate "Matter" from "Workflow" and the strict invariants of Trellis/WOS was resolved through a conceptual collapse: **A case is exactly and only its Trellis ledger.**

This realization dissolved the need for a complex, newly engineered `Case` aggregate:

- **One Entity:** The case ledger (durable, outlives any workflow).
- **One Write Path Pattern:** Operations append typed events within a closed `wos.*` family (`case.created`, `process.started`, `note.added`) directly to the ledger via `$defs/OutputBinding` or direct-append APIs.
- **One Read Path Pattern:** Consumers query a derived, denormalized view materialized from the event stream.

This analytical shift proved that the existing primitives were sufficient. Workflows are simply runtime processes (Restate / Temporal) that bind to a ledger, emit events onto it, and eventually terminate. The ledger, and therefore the Case, remains immutable and authoritative.

## 4. Operational Pluralism: The Dual Identity Model

*Ref: `case-boundary-decision-report.md`*

While the data layer conceptually collapsed to a single source of truth, the Codex adversarial review exposed a critical operational reality: **Runtime identity cannot be collapsed.**

Real-world scenarios (e.g., an applicant filing an appeal while a primary investigation is still ongoing) demand **N:1 workflows** (multiple concurrent processes operating on a single case).

If the system utilized a single identity where the Case ID and the Workflow Instance ID were identical (`case_<ulid>`), the runtime substrate could not reliably route events, fire timers, or manage queues for multiple concurrent workflows acting on that case. Therefore, the analysis defined a strict dual-identity infrastructure:

1. **Ledger Identity (`case_<ulid>`):**
   - The durable identifier for the real-world matter.
   - 1 per case.
   - Survives all workflows.
2. **Process Identity (`process_<ulid>`):**
   - The ephemeral identifier for a specific workflow runtime execution.
   - 0..N per case.
   - Used exclusively for internal event routing, task timers, and callback state within the runtime execution substrate.

## 5. Conclusion & Structural Realities

The extensive analytical arc surrounding Case Management in WOS yielded a robust architectural framework defined by distinct, non-overlapping operational layers.

- **The Conceptual Illusion:** The UI and API may present a rich `Case` to the user, but structurally, this object does not exist as an independent, writeable aggregate database entity.
- **The Execution Layer:** WOS operates strictly as an ephemeral execution engine (`CaseProcess`). It manipulates the real world only by emitting events.
- **The Durability Layer:** Trellis serves as the unyielding bedrock. Every note, decision, transition, and artifact is simply a cryptographically anchored event on a ledger.

Ultimately, the aggregate analysis demonstrates that true case management within a highly governed, zero-trust system is achieved not by building a massive, data-heavy product abstraction on top of the workflow engine, but by strictly limiting the workflow engine to emitting verifiable events onto an immutable cryptographic ledger.
