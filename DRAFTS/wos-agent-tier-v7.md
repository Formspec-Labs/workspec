---
title: WOS-Agent Tier Specification
version: 1.0.0-draft.1
date: 2026-04-09
status: draft
---

# WOS-Agent Tier Specification v1.0

**Version:** 1.0.0-draft.1
**Date:** 2026-04-09
**Editors:** Formspec Working Group
**Tier Specification of:** WOS Core v7.0

---

## Abstract

The WOS-Agent Tier Specification defines the complete governance framework for AI agent participation in WOS workflows. It elaborates the foundational agent concepts established in WOS Core -- actor types, autonomy levels, deontic constraints, and the four-layer audit architecture -- into a buildable system covering agent configuration, autonomy governance, confidence assessment, guardrail composition, multi-step sessions, tool use governance, human-agent collaboration patterns, agent lifecycle management, evaluation and drift monitoring, provenance records, graceful degradation, and Assist protocol governance. Every mechanism defined here enforces a single architectural invariant: the agent is outside the trust boundary, and the WOS Processor governs its participation.

This specification is additive to the WOS Core. It MUST NOT be interpreted in a way that contradicts the WOS Core Specification.

---

## Status of This Document

This document is a **draft tier specification** of WOS Core v7.0. It depends on and extends the WOS Core Specification. Where this specification provides more detailed semantics than the Core, this specification governs for agent-related behavior.

This specification addresses a rapidly evolving area. The editors expect iteration based on implementation experience, changes in AI model capabilities, and emerging governance requirements.

---

## 1. Introduction

### 1.1 Purpose

WOS Core establishes that agents are actors outside the trust boundary (WOS Core S3.2), subject to deontic constraints (WOS Core S6), autonomy levels (WOS Core S5), and structured oversight (WOS Core S7). It defers all agent-specific mechanics to this tier specification, including the enforcement ordering items 4-6 referenced in WOS Core S6.5: confidence floors, volume constraints, and human review sampling.

This specification defines those mechanics.

### 1.2 Scope

This specification defines:

- Agent configuration schema and model version policy.
- Autonomy governance: escalation, demotion, dynamic selection, per-action overrides.
- Confidence framework: reporting, calibration, decay, cumulative computation.
- Guardrail system: three-level composition, semantic guardrails, volume constraints, review sampling, bypass.
- Multi-step agent sessions with checkpoints and intervention points.
- Tool use governance: permitted/prohibited registries, side-effect policy, provenance.
- Human-agent collaboration patterns for the task lifecycle.
- Agent lifecycle states and transitions.
- Evaluation, calibration measurement, and drift detection.
- Agent provenance record schemas.
- Graceful degradation modes and fallback chains.
- Assist Governance Proxy for form-level agent interactions.

Out of scope: model training, prompt engineering, AI safety research, and bias detection methodologies.

### 1.3 Relationship to WOS Core

This specification extends the following WOS Core sections:

| Core Section | Extension |
|---|---|
| S3 Actor Model | S4 elaborates agent architecture and identity. |
| S5 Autonomy Levels | S6 provides escalation, demotion, dynamic selection. |
| S6 Deontic Constraints | S8 provides guardrail composition and enforcement detail. |
| S6.5 Enforcement Ordering | S8 defines items 4-6: confidence floor, volume, sampling. |
| S7 Oversight Protocols | S11 defines human-agent collaboration patterns. |
| S8 Audit Architecture | S14 provides agent-specific provenance records. |
| S10 Interface Contracts | S5.3 uses Formspec Definitions for capability contracts. |

### 1.4 Relationship to Formspec

This specification follows the four Formspec integration rules:

1. **Additive only.** This specification MUST NOT alter core Formspec processing semantics. A Formspec processor that does not implement WOS-Agent remains fully conformant to Formspec.
2. **Cite, never restate.** Formspec behavior is referenced by section number, never restated.
3. **Delegate processing.** WOS-Agent processors MUST delegate Formspec Definition evaluation to a Formspec-conformant processor (Core S1.4).
4. **Canonical terminology.** Formspec terms use their normative definitions: "Assist Provider (Assist S2.1)" not "Formspec Assist Provider."

### 1.5 Notational Conventions

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [BCP 14][RFC 2119] [RFC 8174] when, and only when, they appear in ALL CAPITALS, as shown here.

JSON syntax and data types are as defined in [RFC 8259]. URI syntax is as defined in [RFC 3986].

Terms defined in the WOS Core Specification -- including *actor*, *agent*, *autonomy level*, *deontic constraint*, *guardrail*, *impact level*, and *WOS Processor* -- retain their WOS Core meanings throughout this document.

---

## 2. Conformance

### 2.1 Conformance Classes

**WOS-Agent Processor.** A WOS Processor that satisfies the requirements of this specification. A WOS-Agent Processor MUST implement the Governance conformance profile defined in WOS Core S2.2.

### 2.2 Conformance Levels

**WOS-Agent Basic.** MUST satisfy requirements in S5-S8, S11, S14, and S15. Supports agents as decision service evaluators and task assistants with guardrails, confidence reporting, and fallback.

**WOS-Agent Full.** MUST satisfy all requirements in this specification, including S9 (multi-step sessions), S10 (tool use governance), S12 (lifecycle management), and S13 (evaluation and drift monitoring).

### 2.3 Conformance Requirements

A conformant WOS-Agent Processor:

1. MUST implement the autonomy governance framework (S6), including escalation and demotion.
2. MUST implement the confidence framework (S7), including ConfidenceReport validation.
3. MUST implement the guardrail system (S8), including all guardrail types and the enforcement ordering from WOS Core S6.5.
4. MUST implement graceful degradation (S15) for all agent invocation points.
5. MUST produce all agent provenance record types defined in S14.
6. MUST support confidence-based routing in guard expressions.
7. SHOULD implement tool use governance (S10).
8. SHOULD implement evaluation and drift monitoring (S13).

---

## 3. Terminology

This section is normative. Terms defined in WOS Core retain their definitions. The following additional terms are defined.

- **Agent Configuration** -- A named, versioned declaration of an agent's identity, capabilities, autonomy policy, guardrails, fallback behavior, and operational constraints within a workflow.
- **Agent Session** -- A bounded multi-step interaction between a workflow instance and an agent, with checkpoints and a terminal event.
- **Calibration** -- The process of aligning an agent's reported confidence with the empirical frequency of correct outputs.
- **Checkpoint** -- A recorded intermediate state within an agent session that enables recovery, inspection, and intervention.
- **Confidence Decay** -- The reduction of effective confidence as underlying case data changes after an agent output is produced.
- **Drift** -- A statistically significant change in an agent's output distribution, accuracy, or confidence calibration over time.
- **Fallback Chain** -- An ordered degradation sequence executed when an agent is unavailable or produces unacceptable output, terminating in a human task.
- **Intervention Point** -- A defined moment in an agent session where a human may inspect intermediate results and redirect, modify, or terminate the session.
- **Tool** -- An external capability (API call, database query, calculation) that an agent may invoke during reasoning, subject to governance constraints.

---

## 4. Agent Architecture

This section is normative.

### 4.1 Architectural Position

Agents are actors (WOS Core S3), not a separate architectural layer. An agent may evaluate Decision Services, perform Tasks, or invoke Integrations. What distinguishes agent actors is the governance envelope: autonomy constraints, guardrails, confidence requirements, and fallback policies that surround every invocation.

The governance envelope is defined in the workflow specification, enforced by the WOS Processor, and recorded in provenance. The agent is outside the trust boundary (WOS Core S3.2).

### 4.2 Trust Model

1. The **WOS Processor** is trusted to enforce governance: it controls invocation, validates outputs, enforces guardrails, and records provenance.
2. The **agent** is untrusted. Its outputs are claims validated against guardrail constraints before acceptance.
3. **Human actors** are the ultimate authority. When a human and an agent disagree, the human's judgment governs (WOS Core S1.2).
4. **Guardrails** are trusted to the extent that their constraint expressions are correct. Guardrails are part of the workflow definition, authored by humans, and subject to versioning.

### 4.3 Agent Identity

Every agent has a stable configuration identity and a traceable execution identity:

| Component | Description |
|---|---|
| `configurationId` | The agent configuration name from the workflow definition. |
| `provider` | The model provider. |
| `modelId` | The specific model identifier. |
| `modelVersion` | The exact version or checkpoint. |
| `sessionId` | A unique identifier for this invocation session. |

All five components MUST be recorded in every agent provenance record (WOS Core S3.3).

---

## 5. Agent Configuration

This section is normative.

### 5.1 Configuration Structure

Agent configurations are declared in the `agents` property of a WOS Document. Each configuration declares the agent's identity, capabilities, policies, and constraints.

```yaml
agents:
  eligibilityScreener:
    description: >
      Pre-screens grant applications for basic eligibility criteria.
      Produces a preliminary determination reviewed by a human specialist.
    version: "2.0.0"

    model:
      provider: "anthropic"
      identifier: "claude-sonnet-4-20250514"
      versionPolicy: "approved"
      approvedVersions:
        - "20250514"
        - "20250601"

    capabilities:
      - id: "eligibilityScreening"
        decisionRef: "eligibilityDetermination"
        description: "Evaluate application against eligibility criteria."

    defaultAutonomy: "assistive"

    fallback:
      primary:
        onFailure: "retry"
        maxRetries: 2
        backoff: "exponential"
        initialInterval: "PT5S"
      secondary:
        onFailure: "alternateAgent"
        alternateAgentRef: "eligibilityScreenerFallback"
      terminal:
        onFailure: "escalateToHuman"
        taskRef: "manualEligibilityScreening"

    operationalWindow:
      maxLatency: "PT30S"
      maxConcurrent: 20
      availabilityRequirement: 0.99
      maintenanceWindows:
        - dayOfWeek: ["SU"]
          startTime: "02:00"
          endTime: "06:00"
          timezone: "America/New_York"

    inputPreparation:
      sanitize: true
      maxInputTokens: 50000
      redactFields:
        - "caseFile.application.socialSecurityNumber"
        - "caseFile.application.bankAccountNumber"
```

### 5.2 Model Configuration

The `model` property specifies which AI model the agent uses and how version changes are managed.

| Property | Type | Required | Description |
|---|---|---|---|
| `provider` | string | REQUIRED | Model provider identifier. |
| `identifier` | string | REQUIRED | Model identifier within the provider. |
| `versionPolicy` | enum | REQUIRED | Version selection strategy: `pinned`, `approved`, or `latest`. |
| `approvedVersions` | array of string | CONDITIONAL | Required when `versionPolicy` is `approved`. |
| `minimumVersion` | string | OPTIONAL | Earliest acceptable version. |
| `versionChangeNotification` | boolean | OPTIONAL | Whether to emit an event on version change. Default: `true`. |

**Version Policies:**

| Policy | Semantics | Risk Profile |
|---|---|---|
| `pinned` | The exact version in `identifier` is always used. If unavailable, fallback triggers. | Lowest. Version never changes unexpectedly. |
| `approved` | The latest version from `approvedVersions` is used. New versions require explicit addition. | Moderate. Changes are deliberate. |
| `latest` | The provider's current production version is used. May change without notice. | Highest. Suitable only for low-stakes or heavily guardrailed uses. |

When a version change is detected (for `approved` and `latest` policies), the WOS Processor MUST emit a provenance record of type `agentVersionChange` and SHOULD trigger a recalibration evaluation (S13).

### 5.3 Capabilities

Each capability links the agent to a Decision Service or Task it can evaluate or assist with. A capability MAY reference a Formspec Definition (Core S10) for its input/output contract.

| Property | Type | Required | Description |
|---|---|---|---|
| `id` | string | REQUIRED | Unique identifier for this capability. |
| `decisionRef` | string | OPTIONAL | Reference to a Decision Service the agent evaluates. |
| `taskRef` | string | OPTIONAL | Reference to a Task the agent assists with. |
| `description` | string | OPTIONAL | Human-readable description. |

An agent MUST NOT be invoked for a capability it does not declare. A WOS-Agent Processor MUST reject an `invokeAgent` action referencing an undeclared capability.

### 5.4 Operational Window

| Property | Type | Required | Description |
|---|---|---|---|
| `maxLatency` | string (duration) | OPTIONAL | Maximum acceptable invocation latency. Exceeded = fallback. |
| `maxConcurrent` | integer | OPTIONAL | Maximum concurrent invocations. |
| `availabilityRequirement` | number (0.0-1.0) | OPTIONAL | Minimum uptime fraction. |
| `maintenanceWindows` | array of MaintenanceWindow | OPTIONAL | Scheduled unavailability periods. |

### 5.5 Input Preparation

| Property | Type | Required | Description |
|---|---|---|---|
| `sanitize` | boolean | OPTIONAL | Apply input sanitization for prompt injection patterns. Default: `false`. |
| `maxInputTokens` | integer | OPTIONAL | Maximum input size in tokens. Exceeded = fallback. |
| `redactFields` | array of string | OPTIONAL | Case file paths replaced with `[REDACTED]`. |
| `includeFields` | array of string | OPTIONAL | Only these paths are included. Mutually exclusive with `redactFields`. |

A WOS-Agent Processor MUST reject a configuration that specifies both `redactFields` and `includeFields`.

---

## 6. Autonomy Governance

This section is normative.

WOS Core S5 defines four autonomy levels (`autonomous`, `supervisory`, `assistive`, `manual`) and their constraints. This section specifies how autonomy levels are selected, adjusted, escalated, and demoted.

### 6.1 Default Autonomy

A WOS Document SHOULD declare a `defaultAutonomy` at the workflow level. When no autonomy level is declared on a specific action, the `defaultAutonomy` applies. When no `defaultAutonomy` is declared, the effective default is `manual`.

### 6.2 Autonomy Policy

An autonomy policy declares rules governing autonomy level selection for an agent or capability.

```yaml
autonomyPolicy:
  default: "assistive"

  escalation:
    toAutonomous:
      conditions:
        - "agent.calibration.accuracy >= 0.97"
        - "agent.recentViolations(P30D) = 0"
        - "caseFile.application.requestedAmount <= 10000"
      approval:
        roles: ["programDirector"]
        expires: "P90D"

  demotion:
    toManual:
      triggers:
        - condition: "agent.calibration.accuracy < 0.85"
          immediate: true
        - condition: "agent.guardrailViolations(P7D) >= 3"
          immediate: true
        - condition: "agent.modelVersionChanged"
          immediate: false
          pendingRecalibration: true

  perCapability:
    eligibilityScreening:
      maxAutonomy: "assistive"
      reason: "Eligibility determinations require human confirmation per program policy."
    documentCompleteness:
      maxAutonomy: "autonomous"
      reason: "Document completeness checks are low-risk and well-calibrated."
```

### 6.3 Autonomy Escalation

Escalation increases an agent's effective autonomy level. Escalation is a governance action requiring explicit policy authorization and human approval.

1. The autonomy policy MUST define conditions under which escalation is permissible.
2. A human with the required role MUST review and approve the escalation.
3. The approval MUST have a defined expiration period, after which the agent reverts to its prior level unless re-approved.
4. A provenance record of type `autonomyEscalation` MUST be produced.
5. Escalation MUST NOT bypass guardrails. An agent elevated to `autonomous` still has its outputs validated by guardrail constraints.

### 6.4 Autonomy Demotion

Demotion decreases an agent's effective autonomy level. Demotion MAY be automatic (triggered by policy conditions) or manual (initiated by an administrator).

Automatic demotion triggers include:

- Calibration accuracy falling below a threshold.
- Guardrail violation rate exceeding a threshold.
- Model version change (pending recalibration).
- Drift detection alert (S13).

When automatic demotion fires with `immediate: true`, the demotion takes effect for the next invocation. In-flight sessions at the prior level are not retroactively affected, but a provenance annotation records that the output was produced under a subsequently demoted level.

When automatic demotion fires with `pendingRecalibration: true`, the agent operates at the demoted level until recalibration meets escalation conditions.

### 6.5 Per-Action Autonomy Overrides

An action MAY override the agent's default autonomy level. The override is subject to directional constraints declared in the autonomy policy:

| Property | Type | Description |
|---|---|---|
| `allowEscalation` | boolean | Whether the action may specify a higher autonomy level than the agent's default. |
| `allowDemotion` | boolean | Whether the action may specify a lower autonomy level. Default: `true`. |

The effective autonomy level MUST NOT exceed the `maxAutonomy` defined in the per-capability policy. When the override produces a level higher than `maxAutonomy`, the effective level is capped at `maxAutonomy`.

### 6.6 Dynamic Autonomy Selection

A workflow MAY compute the effective autonomy level from case data and agent state using a FEL expression:

```yaml
actions:
  - action: "invokeAgent"
    agentRef: "eligibilityScreener"
    capability: "eligibilityScreening"
    autonomy:
      dynamic: true
      expression: |
        if caseFile.application.requestedAmount > 100000
          then 'manual'
        else if caseFile.application.expedited = true
          then 'assistive'
        else if agent.calibration.accuracy >= 0.97
          then 'supervisory'
        else 'assistive'
```

Dynamic autonomy expressions are evaluated by the WOS Processor before invocation. The expression context includes `agent` (operational state, calibration metrics) and `caseFile` (current case data). The effective level MUST NOT exceed `maxAutonomy`.

---

## 7. Confidence Framework

This section is normative.

### 7.1 Confidence Report

Every agent output MUST be accompanied by a ConfidenceReport.

| Property | Type | Required | Description |
|---|---|---|---|
| `overall` | number (0.0-1.0) | REQUIRED | Estimated probability that the output is correct. |
| `method` | enum | REQUIRED | How confidence was derived. |
| `explanation` | string | OPTIONAL | Human-readable explanation of confidence factors. |
| `fieldLevel` | object | OPTIONAL | Per-output-field confidence values, keyed by field name. |

The `overall` value represents the agent's estimated probability that its output would be accepted without modification by a competent human reviewer performing the same task.

**Confidence Methods:**

| Method | Definition | Calibration Requirement |
|---|---|---|
| `modelNative` | Derived from the model's own probability estimates (token log-probabilities, internal confidence). | MUST be calibrated per S13. |
| `calibrated` | Post-hoc calibration applied to model-native scores using historical accuracy data. | Calibration is inherent. |
| `heuristic` | Derived from structural properties of the output (consistency checks, cross-validation, output stability). | SHOULD be calibrated. |
| `conformal` | Conformal prediction sets with guaranteed coverage. | Calibration is inherent. |
| `declared` | Manually assigned by the agent developer based on testing and domain knowledge. | MAY be calibrated. |

### 7.2 Per-Field Confidence

When per-field confidence is available, guard expressions MAY reference individual fields:

```yaml
confidence:
  overall: 0.88
  method: "calibrated"
  fieldLevel:
    documentType: 0.97
    extractedAmount: 0.62
    fiscalYear: 0.95
```

Per-field confidence enables targeted routing: a classification output with high confidence on document type but low confidence on an extracted amount routes only the low-confidence field to human review.

### 7.3 Confidence Decay

Agent outputs become less reliable as underlying case data changes. Confidence decay models this degradation.

```yaml
agents:
  riskAssessor:
    confidenceDecay:
      enabled: true
      halfLife: "P14D"
      triggers:
        - event: "caseFile.financials.modified"
          decayFactor: 0.5
        - event: "caseFile.documents.added"
          decayFactor: 0.8
```

| Property | Type | Required | Description |
|---|---|---|---|
| `enabled` | boolean | REQUIRED | Whether confidence decay is active. |
| `halfLife` | string (duration) | OPTIONAL | Duration after which effective confidence halves, absent triggering events. |
| `triggers` | array of DecayTrigger | OPTIONAL | Events causing immediate confidence reduction. |

When a decay trigger fires, the effective confidence of all outputs produced by that agent for the current case is multiplied by the `decayFactor`. When the resulting confidence falls below the confidence floor guardrail (S8), the output is invalidated and the agent is re-invoked or the action is escalated to a human.

### 7.4 Temporal Confidence Thresholds

Confidence thresholds used in autonomy decisions and routing SHOULD be modeled as temporal parameters so they can be adjusted based on operational experience:

```yaml
parameters:
  autonomousConfidenceThreshold:
    description: "Minimum confidence for autonomous agent processing."
    type: "number"
    values:
      - effectiveDate: "2026-04-01"
        value: 0.95
      - effectiveDate: "2026-07-01"
        value: 0.93
```

### 7.5 Cumulative Confidence

In multi-step sessions (S9), errors compound. The cumulative confidence after step *n* is the product of individual step confidences by default. A four-step session where each step has 0.9 confidence yields a cumulative confidence of approximately 0.66.

A WOS-Agent Processor MUST track cumulative confidence across session steps. The default computation is multiplicative unless the agent configuration declares that steps are independent. When cumulative confidence falls below the session's confidence floor, the session MUST pause at the next checkpoint for human review.

### 7.6 Expired Calibration

When an agent's calibration has expired (the `calibrationFrequency` period has elapsed without recalibration), the agent's effective autonomy MUST be capped at `assistive` regardless of its configured level. An agent with expired calibration MUST NOT operate at `autonomous` or `supervisory` levels.

---

## 8. Guardrail System

This section is normative.

WOS Core S6.5 defines the enforcement ordering for deontic constraints and references items 4-6 as "defined in WOS-Agent." This section defines those items and the complete guardrail composition model.

### 8.1 Guardrail Composition

Guardrails are defined at three levels. Narrower scopes compose with broader ones by union:

1. **Workflow-level** guardrails apply to all agent invocations.
2. **Agent-level** guardrails apply to all invocations of a specific agent.
3. **Action-level** guardrails apply to a specific invocation point.

All applicable guardrails at all levels are evaluated. When enforcement actions conflict, the most restrictive action applies. The restriction ordering (WOS Core S6.5) is: `reject` > `escalateToHuman` > `switchToAssistive` > `flag`.

```yaml
# Workflow-level
lifecycle:
  agentGuardrails:
    confidenceFloor:
      threshold: 0.5
      onViolation: "escalateToHuman"

# Agent-level
agents:
  riskAssessor:
    guardrails:
      confidenceFloor:
        threshold: 0.7
        onViolation: "escalateToHuman"

# Action-level
lifecycle:
  states:
    riskAssessment:
      onEntry:
        - action: "invokeAgent"
          agentRef: "riskAssessor"
          guardrails:
            confidenceFloor:
              threshold: 0.85
              onViolation: "reject"
```

In this example, the effective confidence floor is 0.85 with `reject` enforcement (most specific and most restrictive).

### 8.2 Enforcement Ordering

Guardrail evaluation occurs after the agent produces output and before the output is committed. The evaluation order extends WOS Core S6.5:

1. **Permissions** -- structural bounds on allowed outputs (WOS Core S6.1).
2. **Prohibitions** -- forbidden output patterns (WOS Core S6.2).
3. **Obligations** -- required output elements (WOS Core S6.3).
4. **Confidence floor** -- minimum certainty threshold (S8.3).
5. **Volume constraints** -- rate limits on autonomous actions (S8.6).
6. **Human review sampling** -- quality assurance selection (S8.7).

A guardrail violation MUST produce a provenance record of type `guardrailViolation` containing the guardrail identifier, violation type, agent output that triggered it, and enforcement action taken.

### 8.3 Confidence Floor

The confidence floor requires a minimum confidence level for the declared autonomy level.

| Property | Type | Required | Description |
|---|---|---|---|
| `threshold` | number (0.0-1.0) | REQUIRED | Minimum acceptable confidence. |
| `onViolation` | enum | REQUIRED | `escalateToHuman`, `reject`, or `retry`. |

When an agent's `overall` confidence falls below the threshold, the configured enforcement action is taken.

### 8.4 Semantic Guardrails

Beyond the deontic constraint types in WOS Core S6, this specification defines semantic guardrails operating on meaning and consistency.

#### 8.4.1 Consistency Guardrails

Consistency guardrails detect contradictions between an agent's output and case data or prior agent outputs.

```yaml
guardrails:
  consistency:
    - name: "crossFieldConsistency"
      check: >
        not (output.eligible = true and output.riskScore > 90)
      onViolation: "escalateToHuman"
      reason: "Eligible determination with very high risk score is inconsistent."
```

#### 8.4.2 Scope Guardrails

Scope guardrails ensure agent output stays within declared capability boundaries.

```yaml
guardrails:
  scope:
    - name: "outputFieldRestriction"
      allowedOutputFields:
        - "classification"
        - "confidence"
        - "extractedFields"
      onViolation: "reject"
      reason: "Agent produced output fields outside its declared scope."
```

#### 8.4.3 Equity Guardrails

Equity guardrails monitor for statistical disparities in agent outputs across categories.

```yaml
guardrails:
  equity:
    - name: "approvalRateMonitor"
      metric: "approvalRate"
      groupBy: "caseFile.application.geographicRegion"
      maxDisparity: 0.15
      evaluationWindow: "P30D"
      minimumSampleSize: 50
      onViolation: "flag"
      reason: "Significant regional disparity in approval rates detected."
```

| Property | Type | Required | Description |
|---|---|---|---|
| `metric` | string | REQUIRED | The output metric to monitor. |
| `groupBy` | string | REQUIRED | Case file path defining the grouping dimension. |
| `maxDisparity` | number (0.0-1.0) | REQUIRED | Maximum acceptable difference between group rates. |
| `evaluationWindow` | string (duration) | REQUIRED | Rolling window for aggregate evaluation. |
| `minimumSampleSize` | integer | REQUIRED | Minimum observations per group before evaluation. |

Equity guardrails are evaluated asynchronously and SHOULD NOT block individual actions. Statistical disparity at the aggregate level does not imply error at the individual case level. When violated, a provenance record is produced and configured notifications are sent.

### 8.5 Guardrail Bypass

In extraordinary circumstances, an authorized human MAY bypass a guardrail.

1. The guardrail MUST declare `bypassable: true`. Guardrails are non-bypassable by default.
2. The bypassing actor MUST have a role at or above the `bypassAuthority` level.
3. A structured rationale is REQUIRED.
4. A provenance record of type `guardrailBypass` MUST be produced.
5. The bypass applies to a single invocation only. It MUST NOT disable the guardrail for future invocations.

```yaml
guardrails:
  confidenceFloor:
    threshold: 0.85
    onViolation: "escalateToHuman"
    bypassable: true
    bypassAuthority:
      roles: ["programDirector"]
```

### 8.6 Volume Constraints

Volume constraints limit the rate of autonomous agent actions to prevent runaway automation.

| Property | Type | Required | Description |
|---|---|---|---|
| `maxAutonomousPerHour` | integer | OPTIONAL | Maximum autonomous actions per hour. |
| `maxAutonomousPerDay` | integer | OPTIONAL | Maximum autonomous actions per day. |
| `onExceeded` | enum | REQUIRED | `switchToAssistive`, `pause`, or `flag`. |

When the volume threshold is reached, the configured enforcement action applies to subsequent invocations for the remainder of the counting window.

```yaml
guardrails:
  volumeConstraints:
    maxAutonomousPerHour: 50
    maxAutonomousPerDay: 500
    onExceeded: "switchToAssistive"
```

### 8.7 Human Review Sampling

Human review sampling selects a percentage of agent-processed actions for human review, even when all other guardrails pass. This provides ongoing quality assurance.

| Property | Type | Required | Description |
|---|---|---|---|
| `rate` | number (0.0-1.0) | REQUIRED | Proportion of actions to sample. |
| `method` | enum | REQUIRED | Sampling strategy. |

**Sampling Methods:**

| Method | Description |
|---|---|
| `random` | Uniform random sampling at the configured rate. |
| `stratified` | Sample proportionally across output categories, ensuring coverage of each category. |
| `adversarial` | Preferentially sample low-confidence outputs and outputs near decision boundaries. |

Actions selected for sampling are routed to human review regardless of the autonomy level. The review outcome feeds the calibration pipeline (S13).

---

## 9. Multi-Step Agent Sessions

This section is normative.

### 9.1 Session Structure

A multi-step agent interaction is modeled as an Agent Session: a bounded sequence of steps with checkpoints and intervention points.

```yaml
agents:
  investigationAnalyst:
    sessions:
      evidenceAnalysis:
        description: "Analyze evidence package and produce findings."
        maxSteps: 5
        maxDuration: "PT10M"
        checkpointPolicy: "afterEachStep"
        interventionPolicy: "onCheckpoint"

        steps:
          - id: "documentInventory"
            description: "Catalog all documents."
            outputSchema:
              type: "object"
              properties:
                documents:
                  type: "array"
            guardrails:
              outputConstraints:
                - field: "documents"
                  constraint: "count(value) > 0"
                  onViolation: "reject"

          - id: "contentExtraction"
            dependsOn: ["documentInventory"]
            description: "Extract key facts from each document."

          - id: "crossReference"
            dependsOn: ["contentExtraction"]
            interventionPoint: true
            interventionPrompt: >
              Review intermediate findings before the agent
              produces a recommendation.

          - id: "findingsReport"
            dependsOn: ["crossReference"]
            description: "Produce structured findings report."

        termination:
          onCompletion: "commitSession"
          onFailure: "rollbackSession"
          onTimeout: "escalateToHuman"
          onIntervention: "pauseSession"
```

| Property | Type | Required | Description |
|---|---|---|---|
| `maxSteps` | integer | REQUIRED | Maximum steps permitted. |
| `maxDuration` | string (duration) | REQUIRED | Maximum total session time. |
| `checkpointPolicy` | enum | REQUIRED | `afterEachStep` or `onIntervention`. |
| `interventionPolicy` | enum | REQUIRED | `onCheckpoint`, `onCompletion`, or `never`. |

### 9.2 Step Definitions

Each step declares its outputs, guardrails, and dependencies.

| Property | Type | Required | Description |
|---|---|---|---|
| `id` | string | REQUIRED | Unique step identifier. |
| `description` | string | OPTIONAL | Human-readable description. |
| `outputSchema` | object | OPTIONAL | JSON Schema for step output. |
| `dependsOn` | array of string | OPTIONAL | Step IDs that must complete first (DAG ordering). |
| `guardrails` | GuardrailDef | OPTIONAL | Per-step guardrail constraints. |
| `interventionPoint` | boolean | OPTIONAL | Whether to pause for human review after this step. |
| `interventionPrompt` | string | OPTIONAL | Guidance for the human reviewer at this intervention point. |

### 9.3 Checkpoints

A checkpoint records intermediate session state.

| Property | Type | Required | Description |
|---|---|---|---|
| `sessionId` | string | REQUIRED | Agent session identifier. |
| `stepId` | string | REQUIRED | The step that produced this checkpoint. |
| `stepIndex` | integer | REQUIRED | Ordinal position of this step. |
| `output` | object | REQUIRED | The step's output data. |
| `confidence` | ConfidenceReport | REQUIRED | Confidence for this step. |
| `cumulativeConfidence` | number | OPTIONAL | Compounded confidence across all steps. |
| `timestamp` | string (datetime) | REQUIRED | When the checkpoint was created. |

### 9.4 Intervention Actions

At an intervention point, a human reviewer MAY take one of five actions:

| Action | Effect |
|---|---|
| `approve` | Session continues to the next step. |
| `modify` | Reviewer modifies intermediate output. Modified output is used in subsequent steps. |
| `redirect` | Reviewer changes the remaining session plan (skip or add steps). |
| `terminate` | Session ends. Completed steps retained; remaining steps cancelled. |
| `restart` | Session restarts from the beginning or a specified checkpoint. |

When a human modifies intermediate output, provenance captures the original agent output and the modification.

### 9.5 Session Termination

| Action | Semantics |
|---|---|
| `commitSession` | Session outputs committed to case file. |
| `rollbackSession` | Session outputs discarded. Case file unchanged. |
| `escalateToHuman` | A human task is created to complete the work. |
| `pauseSession` | Session suspended at current checkpoint. Resumable. |

---

## 10. Tool Use Governance

This section is normative.

### 10.1 Tool Registry

Each agent configuration MAY declare permitted and prohibited tools.

```yaml
agents:
  investigationAnalyst:
    tools:
      permitted:
        - id: "caseFileRead"
          type: "dataAccess"
          scope: "caseFile"
          access: "read"
          fields: ["application", "eligibility", "reviews"]

        - id: "externalDatabaseLookup"
          type: "integration"
          integrationRef: "samLookup"
          operations: ["search"]
          rateLimit: 10
          rateLimitWindow: "PT1M"

        - id: "calculator"
          type: "computation"
          sideEffects: false

      prohibited:
        - type: "dataAccess"
          scope: "caseFile"
          access: "write"
          reason: "Agents may not directly modify the case file."

        - type: "integration"
          integrationRef: "paymentService"
          reason: "Agents may not initiate financial transactions."
```

| Property | Type | Required | Description |
|---|---|---|---|
| `id` | string | REQUIRED | Unique tool identifier. |
| `type` | enum | REQUIRED | `dataAccess`, `integration`, or `computation`. |
| `scope` | string | OPTIONAL | Data scope the tool accesses. |
| `access` | enum | OPTIONAL | `read` or `write`. |
| `operations` | array of string | OPTIONAL | Permitted operations. |
| `rateLimit` | integer | OPTIONAL | Maximum invocations per `rateLimitWindow`. |
| `rateLimitWindow` | string (duration) | OPTIONAL | Window for rate limit counting. |
| `sideEffects` | boolean | OPTIONAL | Whether the tool modifies external state. |

### 10.2 Tool Use Constraints

The following constraints are normative:

1. An agent MUST NOT invoke a tool not in its `permitted` list. The WOS Processor MUST block unpermitted invocations and record a guardrail violation.
2. An agent MUST NOT write to the case file directly. All case file modifications flow through the WOS Processor's data mutation path, ensuring provenance.
3. Tool invocations MUST respect declared rate limits.
4. Every tool invocation MUST be recorded in provenance, including tool identifier, input, output, and duration.
5. Tools with `sideEffects: true` MUST NOT be invoked at `autonomous` autonomy unless explicitly permitted by a `sideEffectPolicy` declaration.

### 10.3 Tool Output Validation

A WOS-Agent Processor SHOULD validate tool outputs against expected schemas before providing them to the agent. Malformed outputs SHOULD be intercepted, enabling graceful handling rather than corrupted reasoning.

---

## 11. Human-Agent Collaboration Patterns

This section is normative.

### 11.1 Agent-Assisted Task

The most common pattern: an agent prepares a recommendation and a human completes the task by reviewing, modifying, and confirming.

```yaml
tasks:
  eligibilityReview:
    agentAssistance:
      agentRef: "eligibilityScreener"
      capability: "eligibilityScreening"
      timing: "beforeClaim"
      presentation:
        showAgentOutput: true
        showConfidence: true
        showAlternatives: true
        highlightLowConfidenceFields: true
```

| Timing | Description |
|---|---|
| `beforeClaim` | Agent runs when the task is created. Output available when human claims the task. |
| `onClaim` | Agent runs when a human claims the task. |
| `onDemand` | Agent runs only when the human explicitly requests assistance. |
| `parallel` | Agent runs concurrently with human work. |

When `timing` is `beforeClaim`, the agent's output is stored in the task's input data under a reserved `_agentAssistance` field.

### 11.2 Agent-Performed Task

For tasks at `autonomous` autonomy, the agent performs the task without human involvement. The task lifecycle is compressed: Created -> InProgress -> Completed (or Failed), bypassing Available and Claimed states.

Constraints on agent-performed tasks:

1. The task MUST have applicable guardrails.
2. The task MUST have a fallback to human performance.
3. The task MUST produce the same output schema as the human-performed version.
4. Task completion by an agent MUST be distinguishable from human completion in provenance.

### 11.3 Supervisory Review

For tasks at `supervisory` autonomy, the agent performs the task and the result is provisionally committed. A review task is created for a human supervisor.

```yaml
tasks:
  documentClassification:
    autonomy: "supervisory"
    supervisoryReview:
      taskRef: "classificationReview"
      reviewWindow: "PT4H"
      onExpiry: "acceptAsIs"
```

| On Expiry | Semantics |
|---|---|
| `acceptAsIs` | Agent output becomes final. Provenance notes implicit confirmation. |
| `escalate` | A more senior reviewer is notified. |
| `reject` | Output discarded. Task falls back to human performance. |

### 11.4 Triage Pattern

An agent evaluates incoming work and routes it to the appropriate human queue.

```yaml
tasks:
  applicationTriage:
    agentTriage:
      agentRef: "triageClassifier"
      capability: "applicationRouting"
      routingOutput:
        field: "assignedQueue"
        mapping:
          "simpleRenewal": { roles: ["juniorReviewer"] }
          "standardApplication": { roles: ["seniorReviewer"] }
          "complexCase": { roles: ["specializedReviewer"] }
          "highRisk": { roles: ["supervisor"], priority: 1 }
      fallback:
        queue: { roles: ["generalPool"] }
```

Triage agents are well-suited for higher autonomy levels because routing errors are recoverable and the agent does not make the substantive decision.

---

## 12. Agent Lifecycle Management

This section is normative.

### 12.1 Agent States

| State | Description |
|---|---|
| `active` | Available for invocation at its configured autonomy level. |
| `degraded` | Available but operating at a reduced autonomy level due to demotion. |
| `suspended` | Temporarily unavailable. All invocations route to fallback. |
| `retired` | Permanently unavailable. Configuration preserved for audit. Not invocable. |

### 12.2 State Transitions

| Transition | From | To | Trigger |
|---|---|---|---|
| `demote` | `active` | `degraded` | Autonomy demotion trigger (S6.4). |
| `restore` | `degraded` | `active` | Recalibration meets escalation conditions. |
| `suspend` | `active`, `degraded` | `suspended` | Manual action or operational failure. |
| `resume` | `suspended` | `active` or `degraded` | Manual action. |
| `retire` | any | `retired` | Manual action. Irreversible. |

All state transitions MUST produce a provenance record. When an agent transitions to `suspended`, all in-flight sessions MUST pause at their current checkpoint.

### 12.3 Model Version Transitions

When an agent's effective model version changes:

1. The WOS Processor MUST emit a provenance record of type `agentVersionChange` with the prior version, new version, and the version policy that permitted the change.
2. When the autonomy policy includes a `modelVersionChanged` demotion trigger, the agent MUST be demoted pending recalibration.
3. In-flight sessions complete using the version that started them. New invocations use the new version.

---

## 13. Evaluation and Drift Monitoring

This section is normative for WOS-Agent Full and informative for WOS-Agent Basic.

### 13.1 Calibration

Calibration measures alignment between reported confidence and actual accuracy.

| Property | Type | Required | Description |
|---|---|---|---|
| `calibrationRequired` | boolean | REQUIRED | Whether calibration is required. |
| `calibrationFrequency` | string (duration) | CONDITIONAL | Re-evaluation frequency. Required when `calibrationRequired` is `true`. |
| `minimumEvaluationSamples` | integer | CONDITIONAL | Minimum reviewed outputs for valid calibration. |
| `calibrationMethod` | enum | OPTIONAL | `plattScaling`, `isotonic`, `binning`, or `custom`. Default: `binning`. |

Calibration data is derived from ReviewOutcome records (S14). Every human review under `assistive` or `supervisory` autonomy provides a ground-truth label: `accepted` (correct), `modified` (partially correct), or `rejected` (incorrect).

### 13.2 Drift Detection

Drift detection identifies statistically significant behavioral changes over time.

```yaml
evaluation:
  driftDetection:
    enabled: true
    method: "psi"
    threshold: 0.2
    window: "P7D"
    dimensions:
      - field: "riskScore"
        type: "continuous"
      - field: "classification"
        type: "categorical"
    onDetection: "alert"
    alertRoles: ["programDirector", "systemAdministrator"]
```

**Detection Methods:**

| Method | Description |
|---|---|
| `psi` | Population Stability Index. Compares output distributions against a reference period. |
| `ks` | Kolmogorov-Smirnov test. Detects distribution shifts in continuous fields. |
| `chi2` | Chi-squared test. Detects distribution shifts in categorical fields. |
| `accuracy` | Monitors accuracy trend from human review data. |

When drift is detected:

1. A provenance record of type `driftAlert` MUST be produced.
2. Configured alert roles MUST be notified.
3. When the autonomy policy includes a drift-triggered demotion, the demotion MUST be applied.
4. The alert remains active until acknowledged or recalibration is performed.

### 13.3 Operational Metrics

A WOS-Agent Processor SHOULD track the following metrics per agent:

| Metric | Description |
|---|---|
| `invocationCount` | Total invocations in the current period. |
| `successRate` | Proportion of invocations completing without error. |
| `averageLatency` | Mean invocation-to-output time. |
| `guardrailViolationRate` | Proportion triggering guardrail violations. |
| `humanModificationRate` | Proportion of reviewed outputs modified by humans. |
| `humanRejectionRate` | Proportion of reviewed outputs rejected by humans. |
| `confidenceAccuracy` | Calibration accuracy (correlation between confidence and correctness). |

These metrics are available in the expression context as `agent.<metricName>` for use in autonomy policies, guardrail conditions, and dynamic autonomy selection.

### 13.4 Detection-to-Demotion Pipeline

When drift is detected and the autonomy policy includes a drift-triggered demotion:

1. The drift detection method fires and produces a `driftAlert` provenance record.
2. The WOS Processor evaluates the demotion trigger condition.
3. When the condition is met, the agent transitions to `degraded` (S12.2).
4. An `autonomyDemotion` provenance record is produced.
5. The agent operates at the demoted level until recalibration meets escalation conditions (S6.3).

---

## 14. Agent Provenance

This section is normative.

### 14.1 Record Types

This specification defines the following provenance record types, extending WOS Core S8:

| Record Type | Produced When |
|---|---|
| `agentInvocation` | An agent is invoked. |
| `agentSessionStart` | A multi-step session begins. |
| `agentCheckpoint` | A checkpoint is recorded during a session. |
| `agentSessionEnd` | A session completes, fails, or is terminated. |
| `agentToolUse` | An agent invokes a tool. |
| `guardrailViolation` | An agent's output violates a guardrail. |
| `guardrailBypass` | A human bypasses a guardrail. |
| `autonomyEscalation` | An agent's autonomy level is increased. |
| `autonomyDemotion` | An agent's autonomy level is decreased. |
| `agentVersionChange` | The effective model version changes. |
| `driftAlert` | Drift detection identifies a significant change. |
| `confidenceDecayEvent` | Effective confidence decays below threshold. |

### 14.2 Invocation Record

Every agent invocation MUST produce an invocation record.

```yaml
recordType: "agentInvocation"
data:
  agentRef: "eligibilityScreener"
  sessionId: "urn:session:abc-123"
  capability: "eligibilityScreening"
  model:
    provider: "anthropic"
    identifier: "claude-sonnet-4-20250514"
    version: "20250514"
  autonomyLevel: "assistive"
  inputSummary:
    fieldCount: 12
    tokenEstimate: 3200
    redactedFields: ["socialSecurityNumber"]
  guardrailsApplied:
    - "confidenceFloor"
    - "outputConstraints"
    - "prohibitedOutputs"
  latencyMs: 4200
  outcome: "success"
  confidence:
    overall: 0.91
    method: "calibrated"
```

### 14.3 Tool Use Record

```yaml
recordType: "agentToolUse"
data:
  sessionId: "urn:session:abc-123"
  stepId: "crossReference"
  toolId: "externalDatabaseLookup"
  toolType: "integration"
  input:
    query: "SAM registration lookup"
    entityId: "..."
  output:
    registrationStatus: "active"
    expirationDate: "2027-01-15"
  latencyMs: 850
  cached: false
```

### 14.4 AlternativeOutput Tracking

Agent provenance records MAY include alternatives the agent considered:

| Property | Type | Description |
|---|---|---|
| `output` | object | The alternative output values. |
| `confidence` | number | Confidence for this alternative. |
| `reason` | string | Why this alternative was not selected. |

```yaml
alternativesConsidered:
  - output:
      classification: "unauditedFinancial"
    confidence: 0.08
    reason: "Auditor signature block detected."
```

### 14.5 ReviewOutcome Structure

When a human reviews an agent's output, the review MUST be recorded:

| Property | Type | Description |
|---|---|---|
| `reviewer` | ActorRef | The human reviewer. |
| `action` | enum | `accepted`, `modified`, or `rejected`. |
| `modifications` | object | Field-level before/after values when `modified`. |
| `rationale` | string | REQUIRED when `modified` or `rejected`. |
| `reviewDuration` | string (duration) | Time from agent output to review completion. |

```yaml
reviewOutcome:
  reviewer:
    id: "urn:user:jchen"
    role: "financialAnalyst"
  action: "modified"
  modifications:
    extractedEntities.totalRevenue:
      before: 1250000
      after: 1245000
  rationale: "Corrected revenue figure per footnote 3 adjustment."
  reviewDuration: "PT4M30S"
```

### 14.6 Provenance Completeness

A WOS-Agent Processor MUST produce provenance sufficient to reconstruct, for any agent-involved action:

1. Which agent was invoked, with what model version.
2. What data the agent received (by reference or summary).
3. What the agent produced, with confidence.
4. What guardrails were evaluated and their results.
5. What the effective autonomy level was and how it was determined.
6. Whether a human reviewed the output, and what changes were made.
7. What tools the agent used and what results it received.
8. Whether the agent's output was ultimately used in a case decision.

---

## 15. Graceful Degradation

This section is normative.

### 15.1 Principle

Every workflow that uses agents MUST function correctly when agents are unavailable (WOS Core S1.2, S5.1). Agent unavailability is a regular operating condition, not an edge case.

### 15.2 Fallback Chain Execution

When an agent invocation fails (error, timeout, guardrail rejection, or unavailability), the fallback chain is executed:

1. Each level is attempted in order.
2. A successful level stops the chain.
3. A failed level advances to the next.
4. Every attempt MUST produce a provenance record.
5. The terminal level MUST produce a result or transition to a human task.

A fallback chain MUST terminate in either `escalateToHuman` or `fail`. A chain MUST NOT cycle. A WOS-Agent Processor MUST validate chains at document load time and reject chains that cycle or lack a terminal action.

**Fallback Options:**

| Option | Description |
|---|---|
| `escalateToHuman` | Create a human task via `taskRef`. |
| `retry` | Retry with backoff (`maxRetries`, `backoff`, `initialInterval`). |
| `alternateAgent` | Try a different agent via `alternateAgentRef`. |
| `fail` | Fail the action. |

### 15.3 Degradation Modes

A workflow MAY define explicit degradation modes:

```yaml
lifecycle:
  degradationModes:
    noAgents:
      description: "All agents unavailable. Fully manual mode."
      stateOverrides:
        riskAssessment:
          onEntry:
            - action: "createTask"
              taskRef: "manualRiskAssessment"
      autonomyOverride: "manual"
      notification:
        roles: ["systemAdministrator", "programDirector"]

    partialAgents:
      description: "Some agents unavailable. Available agents at reduced autonomy."
      autonomyOverride: "assistive"
```

### 15.4 Degradation Testing

A WOS-Agent Full Processor SHOULD support degradation testing: simulating agent unavailability for specific or all agents, verifying that fallback chains execute correctly and the workflow completes without agent participation.

### 15.5 Deployment Sequence

For `rights-impacting` and `safety-impacting` workflows, model version changes SHOULD follow a three-phase deployment sequence:

1. **Shadow.** The new version runs in parallel with production but its outputs are not used. Outputs are compared to the production version for drift analysis.
2. **Canary.** The new version handles a small percentage of invocations, with strict guardrails and human review sampling at elevated rates.
3. **Production.** The new version replaces the prior version for all invocations after shadow and canary phases demonstrate acceptable performance.

---

## 16. Assist Governance Proxy

This section is normative.

### 16.1 Scope

The WOS Assist Governance Proxy is a WOS-defined governance construct that consumes the Formspec Assist protocol (Assist S2, S7). It sits between the Assist Consumer and the Assist Provider (Assist S2.1), intercepting tool invocations and applying the WOS deontic constraint framework. The proxy does not modify either role's Assist conformance requirements.

The proxy is REQUIRED when a WOS-Agent Processor implements both agent governance and task management, and an activity uses agent assistance with a Formspec Definition.

### 16.2 Per-Tool-Category Governance

| Tool Category | Governance | Detail |
|---|---|---|
| Introspection (`formspec.form.describe`, `field.list`, `field.describe`, `field.help`, `form.progress`) | Pass-through | `FieldHelp` responses recorded in provenance (Layer 1). See S16.3 for `independentFirst` suppression. |
| Mutation (`formspec.field.set`, `field.bulkSet`) | Intercepted | Permission -> Prohibition -> Obligation checks before forwarding. `confirm: true` enforced when the activity requires human confirmation. |
| Validation (`formspec.form.validate`, `field.validate`) | Pass-through | Results observed by provenance recorder. |
| Profile (`formspec.profile.apply`) | Intercepted | `matches` array filtered against Permission scope and Prohibition patterns. Filtered entries appear as `PROHIBITED` in skipped results. |
| Navigation (`formspec.form.pages`, `form.nextIncomplete`) | Pass-through | No governance implications. |

### 16.3 independentFirst Suppression

When the structured oversight protocol is `independentFirst` (WOS Core S7), the proxy MUST suppress agent-generated `FieldHelp.summary` content in `formspec.field.help` responses (Assist S2) until the reviewer's independent assessment is recorded. This ensures the reviewer forms an independent judgment before the agent's recommendation is accessible.

### 16.4 Provenance Recording

Every tool invocation through the proxy produces a provenance record containing: Layer 1 (tool name, input, output, timestamp, agent identity), Layer 2 (deontic constraints evaluated, pass/violate result), and Layer 3 (agent explanation if provided, labeled non-authoritative per WOS Core S8.1).

---

## 17. Security Considerations

This section is informative.

### 17.1 Prompt Injection

When agents process case data, adversarial content may manipulate behavior. Mitigations:

1. **Input isolation.** Input preparation (S5.5) SHOULD sanitize inputs.
2. **CaMeL architecture.** When `inputPreparation.isolateUntrustedData` is `true`, the WOS Processor SHOULD separate trusted control flow (workflow definition, system instructions) from untrusted data processing (case files, documents). This follows the CaMeL dual-LLM pattern where a planning component processes only trusted instructions and a quarantined component processes untrusted data.
3. **Output validation.** Guardrails provide structural defense regardless of agent compromise.
4. **Behavioral monitoring.** Drift detection (S13) may identify injection campaigns shifting output distributions.

Input sanitization is not a complete defense. The guardrail system is designed as defense-in-depth.

### 17.2 Cascading Autonomy

An agent operating autonomously MUST NOT invoke other agents autonomously without explicit declaration (WOS Core S3.3). This specification further requires:

1. The tool registry MUST NOT include `invokeAgent` as a permitted tool unless the workflow declares `cascadingAgentPolicy: "permitted"`.
2. When cascading is permitted, total chain depth MUST be bounded by `maxCascadeDepth` (default: 2).
3. Each cascade step MUST produce its own provenance record with the depth recorded.

### 17.3 Inference Disclosure

Agents may infer information not explicitly present in input data. Inferred information SHOULD be flagged as agent-derived in the case file. Visibility rules SHOULD restrict access to inferred data to the same degree as the source data from which it was inferred.

### 17.4 Model Supply Chain

1. Use `pinned` or `approved` version policies for consequential workflows.
2. Record model version in all provenance records (this is REQUIRED by S14).
3. Require recalibration after model version changes (S13).
4. Monitor output distribution shifts for non-pinned policies.

---

## 18. Privacy Considerations

This section is informative.

### 18.1 Data Minimization

The `inputPreparation` mechanism (S5.5) implements least-privilege data access. The `includeFields` approach is more conservative and is RECOMMENDED when agents interact with external model providers.

### 18.2 Agent Processing as Data Processing

In jurisdictions with data protection regulations, agent invocations sending case data to external providers constitute data processing. Implementations SHOULD record the data processor in provenance, support processing agreements, and ensure `redactFields` aligns with data classification policies.

---

## 19. Conformance Testing

This section is normative.

### 19.1 Test Categories

#### 19.1.1 Guardrail Enforcement

Verify that: (1) output constraint violations trigger correct enforcement, (2) confidence floor violations trigger correct response, (3) prohibited patterns are detected, (4) volume constraints are enforced, (5) human review sampling operates at the configured rate, (6) three-level composition produces correct effective guardrails, (7) bypass produces correct provenance and does not persist.

#### 19.1.2 Autonomy Governance

Verify that: (1) correct autonomy level is applied per policy, (2) dynamic selection evaluates correctly, (3) escalation requires correct role and produces provenance, (4) demotion triggers on all defined conditions, (5) `maxAutonomy` caps are enforced, (6) supervisory review windows trigger correct expiry action.

#### 19.1.3 Fallback

Verify that: (1) agent failure triggers fallback, (2) timeout triggers fallback, (3) guardrail rejection triggers fallback, (4) chain executes in order, (5) terminal action creates a functional human task, (6) workflow completes without any agent participation.

#### 19.1.4 Provenance

Verify that: (1) all required record types are produced, (2) invocation records include model identity and version, (3) confidence reports are present in all agent decisions, (4) review outcomes are recorded, (5) guardrail violations identify the constraint, (6) autonomy changes capture before/after, (7) tool use captures inputs and outputs.

#### 19.1.5 Multi-Step Sessions (WOS-Agent Full)

Verify that: (1) checkpoints are produced per policy, (2) intervention points create review tasks, (3) cumulative confidence triggers review when below threshold, (4) timeout triggers configured action, (5) failure triggers rollback, (6) human modifications propagate to subsequent steps.

### 19.2 Canonical Fixtures

**Fixture A: Basic Agent Decision.** Agent evaluates eligibility. Tests: invocation, confidence, guardrails (pass/fail), human review under assistive, provenance.

**Fixture B: Graduated Autonomy.** Same decision across varying cases. Tests: dynamic selection, confidence routing, escalation, demotion.

**Fixture C: Graceful Degradation.** Simulated agent unavailability. Tests: fallback chain, human task creation, completion without agents.

**Fixture D: Multi-Step Session (Full).** Evidence analysis with intervention. Tests: checkpoints, intervention task, human modification, cumulative confidence.

**Fixture E: Guardrail Bypass.** Authorized bypass with rationale. Tests: authorization, provenance, non-persistence.

---

## Appendix A. Shadow Mode Pattern

This appendix is informative.

Run the agent in parallel with human performance without using the agent's output. Compare agent outputs to human outputs to build calibration data before granting operational authority. Shadow mode is the RECOMMENDED starting point for agents in `rights-impacting` workflows.

Implementation: set `autonomy: "manual"` with `agentAssistance.timing: "parallel"`, suppress agent output in the task form, and feed both outputs to the calibration pipeline.

---

## Appendix B. Circuit Breaker Pattern

This appendix is informative.

If the guardrail violation rate exceeds a threshold within a window (e.g., 3 violations in 1 hour), automatically suspend the agent and route all work to human fallback. This prevents sustained misbehavior from accumulating harm.

Implementation: model as an equity-style asynchronous guardrail with `evaluationWindow` and violation count threshold, with `onViolation` triggering agent suspension (S12.2).

---

## References

### Normative References

- [WOS-Core] Workflow Orchestration Standard Core Specification, Version 7.0.0-draft.1, 2026-04-09.
- [RFC 2119] Bradner, S. "Key words for use in RFCs to Indicate Requirement Levels." BCP 14, RFC 2119, March 1997.
- [RFC 8174] Leiba, B. "Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words." BCP 14, RFC 8174, May 2017.
- [RFC 8259] Bray, T. "The JavaScript Object Notation (JSON) Data Interchange Format." RFC 8259, December 2017.
- [RFC 3986] Berners-Lee, T., Fielding, R., Masinter, L. "Uniform Resource Identifier (URI): Generic Syntax." RFC 3986, January 2005.
- [Formspec] Formspec Working Group. "Formspec v1.0 -- A JSON-Native Declarative Form Standard."
- [Assist] Formspec Working Group. "Formspec Assist Specification v1.0."

### Informative References

- [Vaccaro2024] Vaccaro, M. et al. "When combinations of humans and AI are useful." Nature Human Behaviour, 2024.
- [CaMeL] Debenedetti, E. et al. "CaMeL: Causal Mediation for LLM Defense." Google DeepMind, 2025.
- [NIST-AI-600-1] NIST. "Artificial Intelligence Risk Management Framework: Generative AI Profile." NIST AI 600-1, July 2024.
- [EU-AI-Act] European Parliament. "Regulation (EU) 2024/1689 (Artificial Intelligence Act)." 2024.
- [OMB-M-24-10] OMB. "Advancing Governance, Innovation, and Risk Management for Agency Use of Artificial Intelligence." M-24-10, March 2024.
