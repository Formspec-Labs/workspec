---
title: WOS Advanced Governance Specification
version: 1.0.0-draft.1
date: 2026-04-09
status: draft
---

# WOS Advanced Governance Specification v1.0

**Version:** 1.0.0-draft.1
**Date:** 2026-04-09
**Editors:** Formspec Working Group
**Companion to:** WOS Kernel Specification v1.0

---

## Abstract

The WOS Advanced Governance Specification defines Layer 3 of the Workflow Orchestration Standard: capabilities for formally verifiable constraints, statistical fairness monitoring, adaptive case management, structured multi-step agent interactions, tool use governance, agent lifecycle management, and operational resilience patterns. An Advanced Governance Document -- itself a JSON document -- targets a WOS Kernel Document and declares verifiable constraint subsets (SMT), equity guardrails, constraint zones (DCR-style), multi-step sessions, tool use governance, agent lifecycle state machines, calibration methods, drift detection methods, shadow mode, and circuit breaker patterns.

These capabilities serve any complex workflow, not just AI-assisted ones. DCR constraint zones model compliance rules for human case management. Equity monitoring of human decisions is a civil rights concern. The "Advanced Governance" name reflects that these are governance capabilities applicable across all workflow types.

WOS MUST NOT alter core Formspec processing semantics. WOS processors MUST delegate Formspec Definition evaluation to a Formspec-conformant processor (Core S1.4).

---

## Status of This Document

This document is a **draft specification**. It is a companion to the WOS Kernel Specification v1.0 and does not modify the kernel's processing model. Implementors are encouraged to experiment with this specification and provide feedback, but MUST NOT treat it as stable for production use until a 1.0.0 release is published.

---

## 1. Introduction

### 1.1 Background

Some workflows require governance mechanisms beyond the foundational kernel orchestration (Layer 0), human governance (Layer 1), and AI integration (Layer 2):

- **Formal verification:** Can we prove that a deontic constraint holds for all possible inputs, rather than relying solely on runtime enforcement?
- **Statistical fairness:** Are certain demographics disproportionately denied -- whether by human caseworkers or AI agents?
- **Adaptive case management:** An investigator may interview witnesses, request documents, and consult experts in any order, subject to constraints -- but the valid next actions are not predetermined by explicit transitions.
- **Structured multi-step reasoning:** Complex agent tasks require multiple steps with checkpoints and human intervention points, where errors compound.
- **Operational resilience:** Shadow deployments, circuit breakers, and agent lifecycle management ensure production stability.

### 1.2 Design Goals

1. **These capabilities serve human workflows too.** Constraint zones, equity guardrails, and calibration methods are not AI-specific.
2. **Adoptable independently.** Runtime capabilities (constraint zones, multi-step sessions, equity guardrails) are useful without verification capabilities (SMT, calibration, drift detection).
3. **Defense in depth.** Verification complements runtime enforcement -- it does not replace it.

### 1.3 Scope

**Within scope:** verifiable constraint subset (SMT); equity guardrails; constraint zones (DCR); multi-step sessions; tool use governance; agent lifecycle state machine; calibration methods; drift detection methods; shadow mode; circuit breaker patterns; conformance profiles.

**Out of scope:** lifecycle topology, case state (Layer 0: Kernel). Due process, review protocols, data validation (Layer 1: Workflow Governance). Agent registration, deontic constraints, autonomy levels, confidence framework, fallback chains (Layer 2: AI Integration).

### 1.4 Notational Conventions

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [BCP 14][RFC 2119] [RFC 8174] when, and only when, they appear in ALL CAPITALS, as shown here.

---

## 2. Conformance

### 2.1 Sub-Conformance Levels

Layer 3 packages conceptually distinct capabilities. To prevent all-or-nothing adoption, conformance is split:

**Advanced Governance: Runtime.** Constraint zones (S4), multi-step sessions (S5), equity guardrails (S3), agent lifecycle (S7), tool use governance (S6). These are runtime governance patterns.

**Advanced Governance: Verification.** Verifiable constraint subset (S8), calibration methods (S9), drift detection methods (S10), verification reports. These are static and offline analysis tools.

**Advanced Governance: Complete.** Both Runtime and Verification.

An implementer can adopt constraint zones for human case management without building an SMT solver integration.

---

## 3. Equity Guardrails

This section is normative.

### 3.1 Overview

Equity guardrails monitor for statistical disparities in workflow outcomes across demographic or categorical groupings. They apply to **human AND AI decisions** -- equity monitoring of human caseworker decisions is a civil rights concern, not an AI concern.

### 3.2 Configuration

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Unique guardrail identifier. |
| `metric` | string | REQUIRED | The outcome metric to monitor (e.g., `approvalRate`, `processingTime`, `denialRate`). |
| `groupBy` | string | REQUIRED | Case file path defining the grouping dimension (e.g., `caseFile.application.geographicRegion`). |
| `maxDisparity` | number (0-1) | REQUIRED | Maximum acceptable difference between the highest and lowest group rates. |
| `evaluationWindow` | string (duration) | REQUIRED | ISO 8601 duration of the evaluation window. |
| `minimumSampleSize` | integer | REQUIRED | Minimum observations per group for a valid evaluation. |
| `onViolation` | enum | REQUIRED | Action when disparity exceeds threshold: `flag`, `alert`, or `suspend`. |
| `reason` | string | OPTIONAL | Human-readable explanation of why this guardrail exists. |

### 3.3 Asynchronous Evaluation

Equity guardrails are evaluated **asynchronously** -- not per-invocation. They produce alerts rather than blocking individual actions. When an equity guardrail is violated, a provenance record is produced and the configured notification is sent.

Equity guardrails SHOULD NOT block individual actions because statistical disparity at the aggregate level does not imply error at the individual case level. The `suspend` action suspends further autonomous processing at the **aggregate level** (pausing a category of agent actions across the workflow) -- it does not block a specific individual action.

### 3.4 Governance Attachment

Equity guardrails attach via the `lifecycleHook` seam. They monitor outcomes across all transitions tagged with specific semantic tags (e.g., all `determination`-tagged transitions).

---

## 4. Constraint Zones (DCR)

This section is normative.

### 4.1 Overview

A **constraint zone** is a governance overlay on a kernel `compound` state, providing declarative internal behavior governed by relations between activities rather than explicit transitions. Constraint zones enable adaptive case management phases where the valid next actions are not predetermined -- an investigator may interview witnesses, request documents, consult experts, or issue subpoenas in any order, subject to constraints.

The constraint zone model is adapted from DCR Graphs (Dynamic Condition Response), proven at government scale by Danish central government deployment (65-70% institutional adoption via KMD WorkZone).

### 4.2 Zone Activities

Each activity within a constraint zone tracks three-state markings:

| Marking | Description |
|---------|-------------|
| `included` | The activity is currently available for execution. |
| `executed` | The activity has been executed at least once in the current zone activation. |
| `pending` | The activity must eventually be executed before the zone can complete. |

Initial markings are declared in the zone definition. Activities may start as included or excluded, and as pending or not pending.

### 4.3 Zone Relations

Five relation types govern activity dependencies:

| Relation | Semantics |
|----------|-----------|
| `condition` | Activity B cannot execute until activity A has been executed. |
| `response` | When activity A executes, activity B becomes pending. |
| `include` | When activity A executes, activity B becomes included (available). |
| `exclude` | When activity A executes, activity B becomes excluded (unavailable). |
| `milestone` | Activity B can only execute while the milestone condition on A holds (A is `executed` and `included`). |

### 4.4 Relation Evaluation

When activity A executes:

1. Set A.`executed` = true.
2. For each `response` relation from A to B: set B.`pending` = true.
3. For each `include` relation from A to B: set B.`included` = true.
4. For each `exclude` relation from A to B: set B.`included` = false. If B was `pending`, a resolution error is raised.
5. Re-evaluate availability for all activities.

### 4.5 Zone Completion

A constraint zone is **satisfied** when: for every activity where `pending = true`, `executed = true`; and no included activity has `pending = true` and `executed = false`. Zone satisfaction raises a `constraintZone.satisfied` event, triggering outgoing transitions.

### 4.6 Zone Provenance

Every activity execution produces standard provenance records. Additionally, each relation evaluation produces a provenance record capturing the relation type, source and target activities, and resulting marking changes.

### 4.7 Integration with Kernel Lifecycle

A constraint zone is activated when the kernel transitions to a state that references the zone. The zone is bound to a kernel state via the `x-constraintZoneRef` property on the kernel state's `extensions` (Kernel S10.5). When entering the state, the zone's activities are initialized to their declared markings. When the zone is satisfied, it raises a `constraintZone.satisfied` event that matches outgoing transitions from the kernel state.

Constraint zones do not introduce a new kernel state type. They are a governance overlay on existing `compound` states: the compound state provides the lifecycle container, and the zone provides declarative internal behavior in place of explicit substates and transitions.

### 4.8 Governance Attachment

Constraint zones attach via the `extensions` seam (Kernel S10.5). The zone itself is declared in the Advanced Governance Document; the kernel state references it by id.

---

## 5. Multi-Step Sessions

This section is normative.

### 5.1 Overview

Some tasks require multiple reasoning steps with checkpoints and intervention points. A multi-step session is a bounded sequence of steps forming a DAG, with defined checkpoints where human review may occur.

### 5.2 Session Structure

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Session identifier. |
| `maxSteps` | integer | OPTIONAL | Maximum number of steps. |
| `maxDuration` | string (duration) | OPTIONAL | Maximum session duration. |
| `checkpointPolicy` | enum | OPTIONAL | `afterEachStep`, `atInterventionPoints`, or `onDemand`. |
| `steps` | array of SessionStep | REQUIRED | Ordered steps with dependencies. |

### 5.3 Session Steps

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Step identifier. |
| `description` | string | OPTIONAL | What this step does. |
| `dependsOn` | array of string | OPTIONAL | Step identifiers that must complete before this step. |
| `interventionPoint` | boolean | OPTIONAL | When true, the session pauses for human review at this step. |
| `interventionPrompt` | string | OPTIONAL | Human-readable prompt displayed at the intervention point. |

### 5.4 Cumulative Confidence

In multi-step sessions, errors compound. The cumulative confidence after step *n* is the product of individual step confidences by default. When cumulative confidence falls below the confidence floor, the session MUST pause at the next checkpoint for human review.

### 5.5 Governance Attachment

Multi-step sessions attach via the `lifecycleHook` seam (Kernel S10.4). Session management is triggered when an agent action requires multi-step execution.

---

## 6. Tool Use Governance

This section is normative.

### 6.1 Normative Constraints

1. An agent MUST NOT invoke a tool that is not in its permitted list. Unauthorized tool invocations MUST be blocked and recorded.
2. An agent MUST NOT write to the case file directly. All case file modifications flow through the WOS Processor's normal data mutation path.
3. Tool invocations accessing external systems MUST respect declared rate limits.
4. Every tool invocation MUST be recorded in provenance.
5. Tools with `sideEffects: true` MUST NOT be invoked by agents at `autonomous` level unless explicitly permitted by a `sideEffectPolicy`.

### 6.2 Tool Registry

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Tool identifier. |
| `category` | string | REQUIRED | Tool category (e.g., `dataRetrieval`, `computation`, `externalApi`). |
| `sideEffects` | boolean | REQUIRED | Whether this tool modifies external state. |
| `rateLimit` | RateLimit | OPTIONAL | Rate limit configuration. |
| `outputSchema` | object | OPTIONAL | Expected output schema for validation. |

### 6.3 Governance Attachment

Tool use governance attaches via the `contractHook` seam (Kernel S10.2). Tool invocations are validated against the tool registry as part of contract validation.

---

## 7. Agent Lifecycle

This section is normative.

### 7.1 Agent States

| State | Description |
|-------|-------------|
| `active` | Available for invocation. |
| `degraded` | Available but operating at a reduced autonomy level. |
| `suspended` | Temporarily unavailable. All invocations route to fallback. |
| `retired` | Permanently unavailable. Configuration preserved for audit. |

### 7.2 State Transitions

| Transition | From | To | Trigger |
|-----------|------|-----|---------|
| `demote` | `active` | `degraded` | Autonomy demotion trigger. |
| `restore` | `degraded` | `active` | Recalibration meets escalation conditions. |
| `suspend` | `active`, `degraded` | `suspended` | Manual action or operational failure. |
| `resume` | `suspended` | `active` or `degraded` | Manual action. |
| `retire` | any | `retired` | Manual action. Irreversible. |

All state transitions MUST produce a provenance record. When an agent transitions to `suspended`, all in-flight sessions are paused at their current checkpoint.

### 7.3 Governance Attachment

Agent lifecycle management attaches via the `actorExtension` seam (Kernel S10.1), extending the Layer 2 agent registration with lifecycle state tracking.

---

## 8. Verifiable Constraint Subset (SMT)

This section is normative.

### 8.1 Overview

The verifiable constraint subset defines a decidable fragment of FEL for deontic constraints amenable to SMT-based formal verification. Verification is complementary to runtime enforcement -- defense in depth.

### 8.2 Restrictions

Constraints within the verifiable subset MUST satisfy:

1. **No unbounded recursion.** `let` bindings are permitted but MUST NOT create recursive definitions.
2. **Bounded quantification.** `some` and `every` expressions MUST quantify over finite, enumerable domains.
3. **Linear arithmetic.** Arithmetic expressions MUST be linear: no multiplication of two variables, no division by a variable, no exponentiation.
4. **Finite domain enumerations.** Equality comparisons MUST reference types with finite domains.
5. **No external function calls.** Only FEL built-in functions (Core S3.5). No extension functions (Core S3.12).
6. **No filter expressions.** The `[Expression]` filter syntax is excluded.

### 8.3 Verification Interface

A processor or external tool that verifies constraints MUST:

1. Accept constraints annotated as `verifiable: true`.
2. Translate each constraint to an SMT formula.
3. Produce a verification report: `proven-safe` (holds for all possible inputs), `proven-unsafe` (counterexample exists and is included), or `inconclusive` (solver timeout or construct outside the verifiable subset).
4. Record the report as a provenance record.

### 8.4 Relationship to Runtime Enforcement

A constraint proven safe still executes at runtime (defense in depth). A constraint proven unsafe is a definition error that MUST be corrected before the workflow reaches `active` status. An inconclusive result is a warning that SHOULD trigger additional runtime monitoring.

---

## 9. Calibration Methods

This section is normative.

### 9.1 Overview

Calibration methods extend Layer 2's confidence framework (AI Integration S7) with specific algorithms for aligning reported confidence with actual accuracy.

### 9.2 Methods

| Method | Description |
|--------|-------------|
| `plattScaling` | Platt scaling -- fits a logistic regression model to map model scores to calibrated probabilities. Suitable for binary classification. |
| `isotonic` | Isotonic regression -- non-parametric calibration using a monotonically increasing step function. More flexible than Platt scaling. |
| `binning` | Equal-width or equal-frequency binning of confidence scores with per-bin accuracy. Simple and interpretable. |
| `custom` | Implementation-defined calibration method. |

### 9.3 Calibration Data

Calibration data is derived from review outcomes. Every time a human reviews an agent's output, the review outcome provides a ground-truth label: `accepted` (correct), `modified` (partially correct), or `rejected` (incorrect).

### 9.4 Governance Attachment

Calibration methods extend the Layer 2 confidence framework and attach via the `actorExtension` seam (Kernel S10.1) as part of the agent's operational profile.

---

## 10. Drift Detection Methods

This section is normative.

### 10.1 Overview

Drift detection methods extend Layer 2's drift monitor (AI Integration S9) with specific statistical algorithms. These detect statistically significant changes in agent behavior over time.

### 10.2 Methods

| Method | Description | Output Type |
|--------|-------------|-------------|
| `psi` | Population Stability Index. Compares the distribution of outputs in the current window against a reference period. | Continuous |
| `ks` | Kolmogorov-Smirnov test. Detects distribution shifts in continuous output fields. | Continuous |
| `chi2` | Chi-squared test. Detects distribution shifts in categorical output fields. | Categorical |
| `accuracy` | Monitors accuracy trend against human-reviewed ground truth. | Labeled data required |

### 10.3 Detection Actions

When drift is detected:

1. A provenance record of type `driftAlert` is produced.
2. Configured alert roles are notified.
3. If the autonomy policy includes a drift-triggered demotion, the demotion is applied.
4. The alert remains active until acknowledged or recalibration is performed.

### 10.4 Governance Attachment

Drift detection extends the Layer 2 drift monitor and attaches via the `lifecycleHook` seam (Kernel S10.4). Drift alerts may trigger agent lifecycle transitions (S7).

---

## 11. Shadow Mode and Circuit Breaker

This section is normative.

### 11.1 Shadow Mode

Shadow mode runs an agent in parallel with human or production-agent performance without using the agent's output for decisions. Outputs are compared for drift analysis and calibration data collection.

Shadow mode is the RECOMMENDED starting point for agents in `rights-impacting` workflows. It enables calibration before granting operational authority.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `enabled` | boolean | REQUIRED | Whether shadow mode is active. |
| `compareTo` | enum | OPTIONAL | What to compare shadow output against: `human`, `productionAgent`. Default: `human`. |
| `duration` | string (duration) | OPTIONAL | How long to run in shadow mode. |
| `minimumSamples` | integer | OPTIONAL | Minimum shadow runs before evaluation. |

### 11.2 Circuit Breaker

The circuit breaker pattern automatically falls back when error rates exceed thresholds, preventing cascading failures.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `enabled` | boolean | REQUIRED | Whether the circuit breaker is active. |
| `errorRateThreshold` | number (0-1) | REQUIRED | Error rate that trips the breaker. |
| `evaluationWindow` | string (duration) | REQUIRED | Window over which error rate is computed. |
| `minimumInvocations` | integer | OPTIONAL | Minimum invocations before the breaker can trip. |
| `cooldownDuration` | string (duration) | REQUIRED | How long the breaker stays open before a probe is attempted. |
| `probeCount` | integer | OPTIONAL | Number of probe invocations to attempt before closing. Default: 1. |

**States:** `closed` (normal operation), `open` (all invocations route to fallback), `half-open` (probe invocations test recovery).

### 11.3 Governance Attachment

Shadow mode and circuit breaker attach via the `lifecycleHook` seam (Kernel S10.4). Shadow mode monitors agent output alongside production; circuit breaker monitors error rates and triggers fallback.

---

## 12. Inter-Document Referencing

An Advanced Governance Document targets a WOS Kernel Document via `targetWorkflow`, following the same pattern as Layer 1 and Layer 2. Equity Config and Verification Report sidecars also target the WOS Kernel Document via `targetWorkflow`, consistent with the sidecar binding pattern used by all WOS layers.

---

## References

### Normative References

- [WOS Kernel] Formspec Working Group, "WOS Kernel Specification v1.0".
- [WOS AI Integration] Formspec Working Group, "WOS AI Integration Specification v1.0".
- [Formspec Core] Formspec Working Group, "Formspec Core Specification v1.0".

### Informative References

- Hildebrandt, T. and Mukkamala, R., "Declarative Event-Based Workflow as Distributed Dynamic Condition Response Graphs", 2011.
- DCR Solutions, "Declarative Process Management for Danish Government", Exformatics.
- de Moura, L. and Bjorner, N., "Z3: An Efficient SMT Solver", TACAS, 2008.
