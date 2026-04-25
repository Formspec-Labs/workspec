# Standards Absorption Gap Analysis

**Date:** 2026-04-24
**Scope:** WOS relationship to BPMN, SCXML, DMN, CMMN, WS-HumanTask, and related workflow standards.
**Status:** Architectural note. Not normative text.

## Answer

WOS has a good best-of-architectures center, but not yet a complete best-of-operational-surfaces model.

The kernel choice is sound: WOS replaces BPMN flowchart topology with an SCXML/Harel-derived JSON statechart, then layers governed case data, provenance, due process, agent constraints, and Trellis custody around it. That center should not be weakened.

The remaining work is not "add more standards." It is to absorb the operational surfaces those standards got right while preserving WOS's stricter semantics.

## What WOS Already Absorbed

| Source | Absorbed | WOS posture |
|---|---|---|
| SCXML / Harel statecharts | Nested states, history states, parallel regions, transitions, guards, deterministic transition resolution | Keep as lifecycle center. WOS is SCXML-derived, not a JSON serialization of SCXML. |
| BPMN | Event taxonomy, timers, errors, service invocation, compensation, task/SLA/escalation ideas, fork/join parallelism (Kernel §4.8) | Keep as concepts. Do not adopt gateways or visual-diagram topology as core. |
| CMMN | Case file orientation, milestones, sentry-like activation posture, discretionary/adaptive case management pressure | Partially absorbed. Needs a more explicit governed discretionary work surface. |
| DMN | External decision services and decision-trace pressure | Do not embed DMN or FEEL. Capture decision trace and authority instead. |
| WS-HumanTask | Human task lifecycle (7 states), role model (5 roles incl. `excludedOwner`), SLA/escalation chain — Governance §10 | Substantially absorbed. Narrow gap: missing `withdrawn`/`expired` terminal states and explicit claim-window/visibility rules. |
| DCR Graphs | Condition/response relations, milestones, flexible case constraints | Absorbed through Advanced Governance constraint zones. |
| Temporal / Durable Task | Deterministic replay, durability, idempotency, crash recovery | Absorbed as abstract durable execution guarantees behind `DurableRuntime`. |

## Gaps Worth Integrating Early

### 1. Decision Table Trace, Not DMN Execution

WOS should not embed a DMN engine or adopt FEEL. FEL remains the only expression language.

The architectural hook already exists: Kernel §10.3 (`provenanceLayer` seam) explicitly names "decision table trace" as L1 Reasoning-tier content. What is missing is the concrete normative shape. Governance §6.2 defines `RuleReference` and `EvidenceReference` but no `DecisionServiceEvidence`.

WOS should add a native shape for deterministic decision-service evidence:

- decision service invoked
- decision table or policy identifier
- version
- hit policy, if applicable
- matched rule identifiers
- inputs and normalized outputs
- authority basis
- confidence or verification posture when derived from an external service

Delivered as a new `$defs/DecisionServiceEvidence` entry in the governance schema plus normative prose in Governance §6.2. No new seam. High value for benefits adjudication, procurement, licensing, and SBA-style workflows.

### 2. Governed Discretionary Work

CMMN's strongest lesson is that real caseworkers need to add work that was not fully pre-modeled. WOS should support this without letting runtime users mutate the authoritative statechart.

Add a governed discretionary work model:

- catalog of permitted discretionary task types
- activation criteria
- who may create or authorize the task
- allowed case-file read/write scope
- required evidence or rationale
- closure effects
- provenance on creation, assignment, completion, cancellation, and override

This captures the useful CMMN planning-table pressure without importing unconstrained planning tables.

### 3. Shared Activation Criteria

CMMN sentries are useful because they combine event and condition into one reusable authoring concept. WOS already has events, guards, milestones, tags, task availability, escalation entry conditions, and wait-state behavior, but those surfaces are not yet one reusable shape.

Introduce a shared activation shape, likely `activationCriteria` or `entryCriteria`, usable by:

- human tasks
- discretionary tasks
- escalation levels
- wait states
- milestones
- callback/event waits
- capability invocation gates

The shape should express:

- triggering event or event class
- FEL condition
- required case-data availability
- actor or authority constraint where relevant
- timeout or expiry behavior where relevant

This is the CMMN sentry lesson adapted to WOS.

### 4. Boundary-Event Ergonomics

BPMN's boundary events are author-friendly. "Attach a timeout, error, or message handler to this activity" is easier to author than spelling out every structural statechart edge.

WOS should not add BPMN boundary events as core topology. It should add an authoring or companion surface that compiles to explicit statechart transitions:

- interrupting timeout handler
- non-interrupting timeout notification
- error handler
- message/callback handler
- compensation trigger

The compiled statechart remains authoritative.

### 5. Human Task Runtime Seam — Completion, Not Introduction

Governance §10 already defines a substantial human task runtime: seven lifecycle states (`created`, `assigned`, `claimed`, `completed`, `failed`, `delegated`, `escalated`, `skipped`), five assignment roles (`owner`, `nominee`, `potentialOwner`, `businessAdministrator`, `excludedOwner`), SLA/escalation chains, and provenance on lifecycle transitions.

The gap is narrow. What is missing:

- `withdrawn` and `expired` terminal states
- explicit claim-window rules (when a nominee may claim, how long a reservation holds)
- explicit `taskVisibility` rules (who may read an unclaimed task)
- separation-of-duties enforcement at claim time (currently lives in review §7.2, not at the task surface)

This is a completion of Governance §10, not a new seam. It absorbs the remaining useful parts of WS-HumanTask and CMMN human tasks in a WOS-native way.

## Do Not Integrate

The following remain rejected or adapter-only:

- FEEL as a second expression language
- embedded DMN engine semantics
- BPMN gateways as core topology
- BPMN diagram parity as an authoring goal
- CMMN planning tables as unconstrained runtime mutation
- SCXML executable script/datamodel semantics
- SCXML as the authoritative wire format

## Refactor Target

The early refactor should center on three shared abstractions, each attaching to an existing Kernel §10 seam — no new seams required:

1. **Governed work activation** — `activationCriteria` shape usable across tasks, milestones, escalation levels, and wait states. Attaches via `lifecycleHook`.

2. **Governed output commit** — one processor pipeline for validating external work output, applying bindings, writing case mutations, and emitting provenance. Mutation records carry `mutationSource` (agent / human / service / event) and `verificationLevel` as part of the commit, not as a separate capability. Attaches via `contractHook` (validation) and `provenanceLayer` (reasoning/facts emission); the mutation-history schema extension lands on Kernel §5.4.

3. **Decision trace** — `DecisionServiceEvidence` shape in Governance §6.2 for deterministic decisions, including decision tables and external policy engines. Attaches via `provenanceLayer` (hook already named in Kernel §10.3).

These refactors absorb the useful standards lessons without moving WOS toward standards pluralism and without adding extension surface.

## Priority

Highest leverage:

1. Governed output commit pipeline, including `mutationSource` and `verificationLevel` on the mutation record — single deliverable; the provenance tag is the pipeline's output, not a parallel workstream.
2. Shared output binding shape across agent, service, event, and human task surfaces.
3. Shared activation criteria — promoted ahead of the task-seam completion because claim rules, milestone activation, and escalation entry all depend on the shared shape.
4. Human task runtime seam completion (`withdrawn`/`expired` terminals, claim-window, `taskVisibility`, separation-of-duties at claim).
5. Decision trace (`DecisionServiceEvidence`) in Governance §6.2.
6. Boundary-event authoring sugar — compiles to explicit statechart transitions.
7. SCXML/BPMN export or import tooling.

Interoperability tooling is lower priority than the authoring/runtime abstractions. Translators can wait; hardening the internal shapes should happen while WOS is still pre-release.
