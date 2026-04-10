# WOS-Agent: AI Agent Integration Tier Specification

## W3C First Public Working Draft

**Latest published version:**
: https://wos-spec.org/TR/wos-agent/

**Editor's Draft:**
: https://wos-spec.org/ed/wos-agent/

**Editors:**
: Mike (TealWolf Consulting LLC)

**Version:**
: 0.1.0

**Date:**
: 8 April 2026

**Status:**
: First Public Working Draft

**Depends on:**
: WOS Core Specification 0.1.0

---

## Abstract

This specification defines the AI Agent Integration layer for the Workflow Orchestration Standard (WOS). It elaborates the foundational agent concepts established in the WOS Core — autonomy levels, confidence reporting, guardrails, and agent provenance — into a complete framework for incorporating AI agents into high-stakes, human-in-the-loop workflows. The specification addresses agent lifecycle management, multi-step agentic reasoning with checkpoints, tool use governance, evaluation and drift monitoring, graceful degradation, and conformance testing for agent-integrated workflows. It is designed to ensure that AI participation in consequential workflows remains auditable, explainable, constrained, and subordinate to human authority.

---

## Status of This Document

This section describes the status of this document at the time of its publication.

This document is a First Public Working Draft of a WOS Tier Specification. It depends on and extends the WOS Core Specification v0.1.0. Where this specification provides more detailed semantics than the Core Specification, this specification governs. This specification MUST NOT be interpreted in a way that contradicts the Core Specification.

This specification addresses a rapidly evolving area. The editors expect substantial iteration based on implementation experience, changes in AI model capabilities, and emerging best practices for AI governance in regulated environments.

Comments on this specification are welcome and may be submitted as issues at the specification's repository.

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Conformance](#2-conformance)
3. [Terminology](#3-terminology)
4. [Agent Architecture](#4-agent-architecture)
5. [Agent Configuration](#5-agent-configuration)
6. [Autonomy Governance](#6-autonomy-governance)
7. [Confidence Framework](#7-confidence-framework)
8. [Guardrail System](#8-guardrail-system)
9. [Multi-Step Agent Reasoning](#9-multi-step-agent-reasoning)
10. [Agent Tool Use](#10-agent-tool-use)
11. [Human-Agent Collaboration Patterns](#11-human-agent-collaboration-patterns)
12. [Agent Lifecycle Management](#12-agent-lifecycle-management)
13. [Evaluation and Drift Monitoring](#13-evaluation-and-drift-monitoring)
14. [Agent Provenance](#14-agent-provenance)
15. [Graceful Degradation](#15-graceful-degradation)
16. [Security Considerations](#16-security-considerations)
17. [Privacy Considerations](#17-privacy-considerations)
18. [Conformance Testing](#18-conformance-testing)
19. [References](#19-references)

**Appendices**

- [A. Complete Example: Agent-Augmented Grant Review](#appendix-a-complete-example)
- [B. Autonomy Level Decision Guide](#appendix-b-autonomy-level-decision-guide)
- [C. Guardrail Pattern Catalog](#appendix-c-guardrail-pattern-catalog)

---

## 1. Introduction

### 1.1 Purpose

AI agents are increasingly capable of performing tasks that were previously the exclusive domain of human workers: classifying documents, evaluating eligibility, extracting structured data from unstructured sources, drafting correspondence, detecting anomalies, and producing recommendations. In high-stakes workflows — grants, benefits, licensing, compliance, investigations — the question is not whether AI will participate, but how that participation is governed.

The WOS Core Specification establishes foundational concepts: actor types (human, system, agent), autonomy levels, confidence reporting, guardrails, and agent provenance records. This Tier Specification elaborates those concepts into a complete governance framework that ensures AI agents participate in consequential workflows as accountable, constrained, auditable actors subordinate to human authority.

### 1.2 Design Principles

This specification is guided by the following principles, listed in priority order:

1. **Human authority is supreme.** No agent configuration, guardrail, or autonomy level may be used to override, circumvent, or diminish human decision-making authority. Agents assist; humans decide. Where conflict exists between an agent's output and a human's judgment, the human's judgment governs.

2. **Accountability requires specificity.** Every agent action must be traceable to a specific model version, specific inputs, specific outputs, specific confidence assessment, and specific guardrail evaluation. Vague attribution ("the AI decided") is insufficient.

3. **Constraints are external to the agent.** Guardrails are enforced by the WOS Processor, not by the agent. The agent is not trusted to enforce its own constraints. This separation is not a commentary on agent reliability — it is an architectural principle that ensures governance survives agent changes.

4. **Degradation must be graceful.** Every point where an agent participates must have a defined fallback path that does not require the agent. Workflows MUST NOT become inoperable when an agent is unavailable.

5. **Transparency is non-negotiable.** Case participants, reviewers, and auditors must be able to determine whether an agent participated in any action, what the agent produced, how confident it was, whether a human reviewed its output, and what guardrails were applied.

### 1.3 Scope

This specification defines:

- Detailed agent configuration and lifecycle semantics.
- The autonomy governance framework, including escalation, demotion, and dynamic autonomy adjustment.
- The confidence framework, including calibration requirements, per-field confidence, and confidence decay.
- The guardrail system, including composition, ordering, conflict resolution, and runtime enforcement.
- Multi-step agent reasoning with intermediate checkpoints and human intervention points.
- Agent tool use governance, including permitted tools, tool output validation, and side-effect management.
- Human-agent collaboration patterns for the task lifecycle.
- Agent evaluation, calibration, and behavioral drift monitoring.
- Detailed agent provenance record specifications.
- Graceful degradation policies and fallback chains.
- Conformance testing requirements for agent-integrated workflows.

The following are out of scope:

- AI model training, fine-tuning, or evaluation methodologies.
- Specific model APIs or invocation protocols (these are implementation details behind the integration interface).
- Prompt engineering guidance (prompts are part of agent configuration, not workflow specification).
- AI safety research or alignment techniques beyond the governance framework defined here.
- Bias detection or fairness assessment methodologies (these are important but belong in separate standards).

### 1.4 Relationship to WOS Core

This specification extends the following WOS Core sections:

| Core Section | Extension in This Specification |
|-------------|-------------------------------|
| §3 Terminology | §3 adds agent-specific terms. |
| §4.4 Actor Model | §4 elaborates the agent actor architecture. |
| §5.7 agents property | §5 provides detailed agent configuration semantics. |
| §6.5 Action Types | §9, §10 elaborate invokeAgent and multi-step patterns. |
| §6.9 Autonomy Levels | §6 provides detailed autonomy governance. |
| §7.7 Guardrails | §8 provides the complete guardrail system. |
| §8 Human Task Management | §11 defines human-agent collaboration patterns. |
| §11.4.1 Agent Decision Records | §14 provides complete agent provenance. |
| §18.7 Agent Conformance Profile | §18 provides detailed conformance testing. |

---

## 2. Conformance

### 2.1 Conformance Classes

This specification defines one conformance class: **WOS-Agent Processor.** A WOS-Agent Processor is a WOS Processor that satisfies the Agent Conformance Profile defined in WOS Core §18.7 and additionally satisfies the requirements of this specification.

### 2.2 Conformance Requirements

A conformant WOS-Agent Processor:

1. MUST satisfy all requirements of the WOS Core Agent Conformance Profile.
2. MUST implement the full autonomy governance framework (§6), including autonomy escalation and demotion.
3. MUST implement the complete guardrail system (§8), including all guardrail types and enforcement semantics.
4. MUST support multi-step agent reasoning with checkpoints (§9).
5. MUST implement graceful degradation (§15) for all agent invocation points.
6. MUST produce all agent provenance record types defined in §14.
7. MUST support confidence-based routing in guard expressions.
8. SHOULD implement agent tool use governance (§10).
9. SHOULD implement evaluation and drift monitoring (§13).

### 2.3 Conformance Levels

This specification defines two conformance levels:

**WOS-Agent Basic.** Satisfies requirements 1–6 above. Supports agents as decision service evaluators and task assistants with guardrails, confidence reporting, and fallback.

**WOS-Agent Full.** Satisfies all requirements 1–9 above. Supports multi-step agentic reasoning, tool use governance, and operational monitoring.

---

## 3. Terminology

This section is normative. Terms defined in the WOS Core Specification retain their definitions. The following additional terms are defined for this specification.

**Agent Configuration.** A named, versioned declaration of an agent's identity, capabilities, autonomy level, guardrails, fallback behavior, and operational constraints within a specific workflow.

**Agent Session.** A bounded interaction between a workflow instance and an agent. A session has a start event, zero or more intermediate checkpoints, and a terminal event (completion, failure, or timeout). Sessions are the unit of agent provenance.

**Calibration.** The process of aligning an agent's reported confidence values with the empirical frequency of correct outputs. A well-calibrated agent that reports 0.9 confidence should be correct approximately 90% of the time.

**Checkpoint.** A recorded intermediate state within a multi-step agent reasoning session. Checkpoints enable human inspection of agent reasoning in progress and provide recovery points if the agent fails mid-session.

**Confidence Decay.** The reduction of effective confidence when the data or context underlying an agent's output changes after the output was produced. A risk classification produced 30 days ago under different case data is less reliable than one produced today.

**Drift.** A statistically significant change in an agent's output distribution, accuracy, or confidence calibration over time. Drift may result from model updates, data distribution changes, or environmental factors.

**Fallback Chain.** An ordered sequence of degradation steps taken when an agent is unavailable or produces unacceptable output. Fallback chains terminate in a human task.

**Graduated Autonomy.** The practice of adjusting an agent's effective autonomy level based on confidence, case complexity, operational history, and policy thresholds.

**Intervention Point.** A defined moment in a multi-step agent reasoning session where a human may inspect intermediate results and redirect, modify, or terminate the session.

**Tool.** An external capability (API call, database query, document retrieval, calculation) that an agent may invoke during reasoning. Tool use is subject to governance constraints.

---

## 4. Agent Architecture

This section is normative.

### 4.1 Architectural Position

Within the WOS layered architecture, agents are **actors** — not a separate layer. An agent may operate at any layer where human or system actors operate: evaluating Decision Services (Layer 2), performing Tasks (Layer 3), processing Case File data (Layer 4), or invoking Integrations (Layer 5). The agent's actions are observed by Provenance (Layer 6) and subject to Execution guarantees (Layer 7) in the same manner as any other actor.

What distinguishes agent actors architecturally is the **governance envelope** — the set of autonomy constraints, guardrails, confidence requirements, and fallback policies that surround every agent invocation. This governance envelope is defined in the workflow specification, enforced by the WOS Processor, and recorded in provenance. The agent itself is outside the trust boundary of the governance envelope.

```
┌─────────────────────────────────────────────────────────┐
│  WOS Processor (Trusted)                                │
│                                                         │
│  ┌──────────────────────────────────────────────────┐   │
│  │  Governance Envelope                              │   │
│  │                                                    │   │
│  │  1. Pre-invocation: Autonomy check, input prep    │   │
│  │  2. Invocation ──────────────┐                    │   │
│  │  3. Post-invocation:         │                    │   │
│  │     - Output validation      │    ┌────────────┐  │   │
│  │     - Guardrail enforcement  │◄───│   Agent     │  │   │
│  │     - Confidence evaluation  │    │ (Untrusted) │  │   │
│  │     - Provenance recording   │    └────────────┘  │   │
│  │  4. Routing decision         │                    │   │
│  │     (autonomous / review)    │                    │   │
│  └──────────────────────────────────────────────────┘   │
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │  Case File   │  │  Provenance  │  │  Task Queue  │   │
│  └──────────────┘  └──────────────┘  └──────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### 4.2 Trust Model

The following trust assumptions are normative:

1. The **WOS Processor** is trusted to enforce governance. It controls invocation, validates outputs, enforces guardrails, and records provenance.
2. The **Agent** is untrusted. Its outputs are treated as claims that must be validated against guardrail constraints before acceptance. The agent's self-reported confidence is useful but not dispositive — calibration and guardrails provide independent checks.
3. **Human actors** are the ultimate authority. When a human and an agent disagree, the human's judgment governs. The standard provides mechanisms for human oversight but does not provide mechanisms for agents to override human decisions.
4. **Guardrails** are trusted to the extent that their constraint expressions are correct. Guardrails are part of the workflow definition, authored and reviewed by humans, and subject to the same versioning and testing as any other workflow component.

### 4.3 Agent Identity

Every agent has a stable identity within the workflow definition (the agent configuration name) and a traceable execution identity composed of:

| Component | Description | Example |
|-----------|-------------|---------|
| `configurationId` | The agent configuration name from the workflow definition. | `documentClassifier` |
| `provider` | The model provider. | `anthropic` |
| `modelId` | The specific model identifier. | `claude-sonnet-4-20250514` |
| `modelVersion` | The exact version or checkpoint. | `20250514` |
| `sessionId` | A unique identifier for this invocation session. | `urn:session:abc-123` |

All five components MUST be recorded in every agent provenance record. This enables tracing an agent action to the exact model version, even when version policies allow model updates.

---

## 5. Agent Configuration

This section is normative.

### 5.1 Configuration Structure

Agent configurations are declared in the top-level `agents` property of a WOS Document, as established in WOS Core §5.7. This section specifies the complete configuration schema.

```yaml
agents:
  eligibilityScreener:
    description: >
      Pre-screens grant applications for basic eligibility criteria.
      Produces a preliminary determination that is always reviewed by
      a human eligibility specialist before becoming final.
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
      - id: "documentCompleteness"
        decisionRef: "completenessCheck"
        description: "Assess whether required documents are present."

    defaultAutonomy: "assistive"
    autonomyPolicy:
      $ref: "#/agents/eligibilityScreener/autonomyPolicy"

    guardrails:
      $ref: "#/agents/eligibilityScreener/guardrails"

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

    evaluation:
      calibrationRequired: true
      calibrationFrequency: "P30D"
      minimumEvaluationSamples: 100
      driftDetection:
        enabled: true
        method: "psi"
        threshold: 0.2
        window: "P7D"
```

### 5.2 Model Configuration

The `model` property specifies which AI model the agent uses and how version changes are managed.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `provider` | string | REQUIRED | Model provider identifier. |
| `identifier` | string | REQUIRED | Model identifier within the provider. |
| `versionPolicy` | enum | REQUIRED | Version selection strategy. |
| `approvedVersions` | array of string | CONDITIONAL | Required when `versionPolicy` is `approved`. |
| `minimumVersion` | string | OPTIONAL | Earliest acceptable version. |
| `versionChangeNotification` | boolean | OPTIONAL | Whether to emit an event when the effective model version changes. Default: `true`. |

**Version Policies:**

| Policy | Semantics | Risk Profile |
|--------|-----------|-------------|
| `pinned` | The exact version specified in `identifier` is always used. If that version becomes unavailable, the agent is treated as unavailable and fallback is triggered. | Lowest risk. Version never changes unexpectedly. Risk of unavailability if version is deprecated. |
| `approved` | The latest version from the `approvedVersions` list is used. New versions must be explicitly added to the list. | Moderate risk. Version changes are deliberate and controlled. |
| `latest` | The provider's current production version is used. Version may change without notice. | Highest risk. Suitable only for low-stakes or heavily guardrailed uses. |

When a version change is detected (for `approved` and `latest` policies), the WOS Processor MUST emit a provenance record of type `agentVersionChange` and SHOULD trigger a recalibration evaluation (§13).

### 5.3 Capabilities

The `capabilities` array declares what the agent can do within the workflow. Each capability links the agent to a specific Decision Service or Task that it can evaluate or assist with.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Unique identifier for this capability. |
| `decisionRef` | string | OPTIONAL | Reference to a Decision Service the agent evaluates. |
| `taskRef` | string | OPTIONAL | Reference to a Task the agent assists with. |
| `description` | string | OPTIONAL | Human-readable description. |

An agent MUST NOT be invoked for a capability it does not declare. A WOS-Agent Processor MUST reject an `invokeAgent` action that references an undeclared capability.

### 5.4 Fallback Chains

The `fallback` property defines an ordered degradation sequence. Each level of the chain specifies what happens if the prior level fails.

A fallback chain MUST terminate in either `escalateToHuman` or `fail`. A fallback chain MUST NOT create a cycle (an agent falling back to itself). A WOS-Agent Processor MUST validate fallback chains at document load time and reject chains that cycle or lack a terminal action.

The RECOMMENDED pattern for consequential workflows is a three-level chain: retry the primary agent, try a fallback agent, escalate to a human. For the most critical decisions, a two-level chain (retry, escalate to human) or a one-level chain (escalate to human immediately) is appropriate.

### 5.5 Input Preparation

The `inputPreparation` property governs how case data is prepared before being sent to the agent.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `sanitize` | boolean | OPTIONAL | Whether to apply input sanitization (removal of potential prompt injection patterns). Default: `false`. |
| `maxInputTokens` | integer | OPTIONAL | Maximum input size in tokens. If the prepared input exceeds this limit, the invocation fails and fallback is triggered. |
| `redactFields` | array of string | OPTIONAL | Case file paths whose values are replaced with `[REDACTED]` before being sent to the agent. |
| `includeFields` | array of string | OPTIONAL | If specified, only these case file paths are included in the agent's input. All other data is excluded. Mutually exclusive with `redactFields`. |

The `redactFields` and `includeFields` properties implement the principle of least privilege for agent data access. An agent evaluating document completeness does not need the applicant's financial details, and those details should not be provided.

---

## 6. Autonomy Governance

This section is normative.

### 6.1 Overview

Autonomy governance controls the degree of independent authority an agent has at each invocation point. The WOS Core defines four autonomy levels (`autonomous`, `assistive`, `supervisory`, `manual`). This section specifies how autonomy levels are selected, adjusted, escalated, and demoted during workflow execution.

### 6.2 Autonomy Policy

An autonomy policy declares the rules governing autonomy level selection for an agent. Policies are defined per-agent or per-capability and MAY be overridden at the action level.

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
      reason: "Eligibility determinations always require human confirmation per program policy."
    documentCompleteness:
      maxAutonomy: "autonomous"
      reason: "Document completeness checks are low-risk and well-calibrated."
```

### 6.3 Autonomy Escalation

Escalation increases an agent's effective autonomy level (e.g., from `assistive` to `autonomous`). Escalation is a governance action that requires explicit policy authorization and human approval.

The escalation process:

1. The autonomy policy defines conditions under which escalation is permissible.
2. A human with the required role reviews and approves the escalation.
3. The approval has a defined expiration period, after which the agent reverts to its prior autonomy level unless re-approved.
4. A provenance record of type `autonomyEscalation` is produced.
5. During the escalation period, the agent operates at the elevated autonomy level subject to all applicable guardrails.

Escalation MUST NOT bypass guardrails. An agent elevated to `autonomous` still has its outputs validated by guardrail constraints. Escalation removes the human review step, not the structural safety checks.

### 6.4 Autonomy Demotion

Demotion decreases an agent's effective autonomy level (e.g., from `autonomous` to `assistive`, or from `assistive` to `manual`). Demotion may be automatic (triggered by policy conditions) or manual (initiated by an administrator).

Automatic demotion triggers include:

- Calibration accuracy falling below a threshold.
- Guardrail violation rate exceeding a threshold.
- Model version change (pending recalibration).
- Drift detection alert.
- Operational anomaly (latency spike, error rate increase).

When automatic demotion is triggered with `immediate: true`, the demotion takes effect for the next agent invocation. In-flight agent sessions at the prior autonomy level are not retroactively affected, but their outputs receive additional scrutiny — a provenance annotation records that the output was produced under an autonomy level that was subsequently demoted.

When automatic demotion is triggered with `pendingRecalibration: true`, the agent operates at the demoted level until recalibration is completed and the results meet the escalation conditions.

### 6.5 Dynamic Autonomy Selection

A workflow MAY define dynamic autonomy selection, where the effective autonomy level for a specific invocation is computed from case data and agent state:

```yaml
actions:
  - action: "invokeAgent"
    agentRef: "eligibilityScreener"
    capability: "eligibilityScreening"
    autonomy:
      dynamic: true
      expression: |
        if caseFile.application.requestedAmount > 100000
          then "manual"
        else if caseFile.application.expedited = true
          then "assistive"
        else if agent.calibration.accuracy >= 0.97
          then "supervisory"
        else "assistive"
```

Dynamic autonomy expressions are evaluated by the WOS Processor before agent invocation. The expression context includes `agent` (the agent's operational state, including calibration metrics) and `caseFile` (the current case data).

The effective autonomy level MUST NOT exceed the `maxAutonomy` defined in the per-capability autonomy policy. If the dynamic expression produces a level higher than `maxAutonomy`, the effective level is capped at `maxAutonomy`.

---

## 7. Confidence Framework

This section is normative.

### 7.1 Confidence Model

Confidence is a structured assessment of certainty associated with an agent's output. Unlike a simple probability score, the WOS confidence model captures the nature, derivation, and limitations of the confidence assessment.

### 7.2 Confidence Report Structure

Every agent output MUST be accompanied by a ConfidenceReport, as defined in WOS Core §11.4.1. This section specifies additional requirements.

The `overall` confidence value MUST be a number between 0.0 and 1.0, inclusive. The value represents the agent's estimated probability that its output is correct, where "correct" is defined as: the output would be accepted without modification by a competent human reviewer performing the same task.

The `method` field MUST accurately reflect how the confidence value was derived:

| Method | Definition | Calibration Requirement |
|--------|-----------|------------------------|
| `modelNative` | Derived from the model's own probability estimates (e.g., token log-probabilities, internal confidence heads). | MUST be calibrated per §13. |
| `calibrated` | Post-hoc calibration has been applied to model-native scores using historical accuracy data. | Calibration is inherent. |
| `heuristic` | Derived from structural properties of the output (e.g., consistency checks, cross-validation, output stability across rephrased inputs). | SHOULD be calibrated. |
| `declared` | Manually assigned by the agent developer based on testing and domain knowledge. | MAY be calibrated. |

### 7.3 Per-Field Confidence

For outputs with multiple fields, the ConfidenceReport MAY include a `fieldLevel` object mapping field names to individual confidence values. Per-field confidence enables more precise routing — a document classification output might have high confidence on the document type but low confidence on an extracted monetary amount, and only the low-confidence field requires human review.

```yaml
confidence:
  overall: 0.88
  method: "calibrated"
  fieldLevel:
    documentType: 0.97
    extractedAmount: 0.62
    fiscalYear: 0.95
```

When per-field confidence is available, guard expressions MAY reference individual field confidence:

```yaml
guard: >
  caseFile.extraction.confidence.fieldLevel.extractedAmount >= 0.9
```

### 7.4 Confidence Decay

Agent outputs become less reliable as the underlying case data changes. Confidence decay models this degradation.

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
|----------|------|----------|-------------|
| `enabled` | boolean | REQUIRED | Whether confidence decay is active. |
| `halfLife` | string (duration) | OPTIONAL | Duration after which the effective confidence is halved, in the absence of triggering events. |
| `triggers` | array of DecayTrigger | OPTIONAL | Events that cause immediate confidence reduction. |

When a decay trigger fires, the effective confidence of all outputs produced by that agent for the current case is multiplied by the `decayFactor`. If the resulting effective confidence falls below the confidence floor guardrail (§8), the output is invalidated and the agent is re-invoked or the action is escalated to a human.

### 7.5 Confidence Thresholds

Confidence thresholds used in autonomy decisions and routing SHOULD be modeled as temporal parameters (WOS Core §7.5) so they can be adjusted based on operational experience:

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

---

## 8. Guardrail System

This section is normative.

### 8.1 Overview

The guardrail system provides declarative, externally enforced constraints on agent behavior. Guardrails are the primary safety mechanism for agent participation in high-stakes workflows. They operate on the principle that the workflow specification — not the agent — defines what outputs are acceptable.

WOS Core §7.7 defines guardrail types and enforcement order. This section specifies composition, conflict resolution, inheritance, and advanced guardrail patterns.

### 8.2 Guardrail Composition

Guardrails may be defined at three levels, with narrower scopes taking precedence:

1. **Workflow-level guardrails** apply to all agent invocations in the workflow.
2. **Agent-level guardrails** apply to all invocations of a specific agent.
3. **Action-level guardrails** apply to a specific invocation point.

When guardrails are defined at multiple levels, they are composed by union: all applicable guardrails are evaluated. A violation at any level triggers the corresponding enforcement action. When enforcement actions conflict (e.g., workflow-level says `flag` but action-level says `reject`), the most restrictive action applies.

```yaml
# Workflow-level guardrail
lifecycle:
  agentGuardrails:
    confidenceFloor:
      threshold: 0.5
      onViolation: "escalateToHuman"

agents:
  riskAssessor:
    guardrails:
      # Agent-level guardrail
      confidenceFloor:
        threshold: 0.7
        onViolation: "escalateToHuman"

lifecycle:
  states:
    riskAssessment:
      onEntry:
        - action: "invokeAgent"
          agentRef: "riskAssessor"
          guardrails:
            # Action-level guardrail
            confidenceFloor:
              threshold: 0.85
              onViolation: "reject"
```

In this example, the effective confidence floor for this specific invocation is 0.85 with `reject` enforcement (the most specific and most restrictive).

### 8.3 Semantic Guardrails

Beyond the structural guardrail types defined in WOS Core §7.7, this specification defines semantic guardrails that operate on the meaning and consistency of agent outputs in context.

#### 8.3.1 Consistency Guardrails

Consistency guardrails detect contradictions between an agent's output and other case data or prior agent outputs.

```yaml
guardrails:
  consistency:
    - name: "crossFieldConsistency"
      check: >
        not (output.eligible = true and output.riskScore > 90)
      onViolation: "escalateToHuman"
      reason: "Eligible determination with very high risk score is inconsistent."

    - name: "temporalConsistency"
      check: >
        output.assessedDate >= caseFile.application.submittedDate
      onViolation: "reject"
      reason: "Assessment date cannot precede application submission."

    - name: "priorOutputConsistency"
      check: >
        abs(output.riskScore - caseFile.priorAssessment.riskScore) <= 30
        or caseFile.materialChange = true
      onViolation: "flag"
      reason: "Significant score change without material case change."
```

#### 8.3.2 Scope Guardrails

Scope guardrails ensure the agent's output stays within the boundaries of its intended role and does not include content outside its declared capability.

```yaml
guardrails:
  scope:
    - name: "noLegalConclusions"
      prohibitedPatterns:
        - field: "recommendation"
          mustNotContain: ["legal liability", "statutory violation", "prosecute"]
      onViolation: "escalateToHuman"
      reason: "Agent must not draw legal conclusions."

    - name: "outputFieldRestriction"
      allowedOutputFields:
        - "classification"
        - "confidence"
        - "extractedFields"
      onViolation: "reject"
      reason: "Agent produced output fields outside its declared scope."
```

#### 8.3.3 Equity Guardrails

Equity guardrails monitor for potential bias in agent outputs by detecting statistical disparities across protected categories, without requiring access to individual protected characteristics.

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

Equity guardrails are evaluated asynchronously (not per-invocation) and produce alerts rather than blocking individual actions. When an equity guardrail is violated, a provenance record is produced and the configured notification is sent. Equity guardrails SHOULD NOT block individual actions because statistical disparity at the aggregate level does not imply error at the individual case level.

### 8.4 Guardrail Bypass

In extraordinary circumstances, an authorized human may bypass a guardrail. Guardrail bypass is modeled as a special case of override (WOS Core §8.8) with additional requirements:

1. The guardrail definition MUST declare whether bypass is permitted (`bypassable: true`). Guardrails are non-bypassable by default.
2. The bypassing actor MUST have a role at or above the guardrail's `bypassAuthority` level.
3. A structured rationale is REQUIRED.
4. A provenance record of type `guardrailBypass` is produced.
5. The bypass applies to a single invocation only. It does not disable the guardrail for future invocations.

```yaml
guardrails:
  confidenceFloor:
    threshold: 0.85
    onViolation: "escalateToHuman"
    bypassable: true
    bypassAuthority:
      roles: ["programDirector"]
```

---

## 9. Multi-Step Agent Reasoning

This section is normative.

### 9.1 Overview

Some agent tasks require multiple reasoning steps: analyzing a document, then cross-referencing findings with case data, then producing a recommendation. Multi-step agent reasoning creates additional governance requirements because intermediate outputs may compound errors, and the total reasoning time may be substantial.

### 9.2 Agent Sessions

A multi-step agent interaction is modeled as an **Agent Session**: a bounded sequence of steps with defined checkpoints and intervention points.

```yaml
agents:
  investigationAnalyst:
    sessions:
      evidenceAnalysis:
        description: "Analyze submitted evidence package and produce findings report."
        maxSteps: 5
        maxDuration: "PT10M"
        checkpointPolicy: "afterEachStep"
        interventionPolicy: "onCheckpoint"

        steps:
          - id: "documentInventory"
            description: "Catalog all documents in the evidence package."
            outputSchema:
              type: "object"
              properties:
                documents:
                  type: "array"
                  items:
                    type: "object"
                    properties:
                      id: { type: "string" }
                      type: { type: "string" }
                      pages: { type: "integer" }
                      relevanceAssessment: { type: "string" }
            guardrails:
              outputConstraints:
                - field: "documents"
                  constraint: "count(value) > 0"
                  onViolation: "reject"

          - id: "contentExtraction"
            description: "Extract key facts and figures from each document."
            dependsOn: ["documentInventory"]
            # ...

          - id: "crossReference"
            description: "Cross-reference extracted facts against case data."
            dependsOn: ["contentExtraction"]
            interventionPoint: true
            interventionPrompt: >
              The agent has inventoried documents and extracted key facts.
              Review the intermediate findings before the agent proceeds
              to produce a recommendation.
            # ...

          - id: "findingsReport"
            description: "Produce structured findings report."
            dependsOn: ["crossReference"]
            # ...

        termination:
          onCompletion: "commitSession"
          onFailure: "rollbackSession"
          onTimeout: "escalateToHuman"
          onIntervention: "pauseSession"
```

### 9.3 Checkpoints

A checkpoint records the intermediate state of an agent session. Checkpoints serve three purposes:

1. **Recovery.** If the agent fails mid-session, execution can resume from the last checkpoint rather than restarting.
2. **Inspection.** A human reviewer can examine intermediate outputs to verify the agent's reasoning trajectory.
3. **Intervention.** At designated intervention points, a human may redirect, modify, or terminate the session.

A checkpoint record contains:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `sessionId` | string | REQUIRED | The agent session identifier. |
| `stepId` | string | REQUIRED | The step that produced this checkpoint. |
| `stepIndex` | integer | REQUIRED | The ordinal position of this step. |
| `output` | object | REQUIRED | The step's output data. |
| `confidence` | ConfidenceReport | REQUIRED | Confidence assessment for this step. |
| `cumulativeConfidence` | number | OPTIONAL | The compounded confidence across all steps so far. |
| `timestamp` | string (datetime) | REQUIRED | When the checkpoint was created. |
| `agentState` | string | OPTIONAL | An opaque token the agent may use to resume from this checkpoint. |

### 9.4 Intervention Points

An intervention point is a step in a multi-step session where the WOS Processor pauses execution and creates a human task for review. The human reviewer may:

| Action | Effect |
|--------|--------|
| `approve` | Session continues to the next step. |
| `modify` | The reviewer modifies the intermediate output. The modified output is used as input for subsequent steps. |
| `redirect` | The reviewer changes the remaining session plan (e.g., skipping a step or adding an additional step). |
| `terminate` | The session is ended. Completed steps are retained; remaining steps are cancelled. |
| `restart` | The session restarts from the beginning or from a specified checkpoint. |

When a human modifies an intermediate output, the provenance record captures the original agent output and the human modification, following the same pattern as Override Records (WOS Core §11.5).

### 9.5 Cumulative Confidence

In multi-step sessions, errors compound. The cumulative confidence after step *n* is, in the worst case, the product of individual step confidences. A four-step session where each step has 0.9 confidence yields a cumulative confidence of approximately 0.66 — significantly lower than any individual step suggests.

A WOS-Agent Processor MUST track cumulative confidence across session steps. The cumulative confidence SHOULD be computed conservatively (multiplicative by default, unless the agent configuration specifies that steps are independent). If cumulative confidence falls below the session's confidence floor guardrail, the session is paused at the next checkpoint for human review, regardless of the intervention policy.

---

## 10. Agent Tool Use

This section is normative.

### 10.1 Overview

Agents may invoke tools — external capabilities such as API calls, database queries, calculations, and document retrievals — during reasoning. Tool use introduces side effects and data access beyond the agent's direct input, creating additional governance requirements.

### 10.2 Tool Registry

Each agent configuration MAY declare a set of permitted tools:

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
          description: "Mathematical calculations."
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

### 10.3 Tool Use Governance

The following constraints on agent tool use are normative:

1. An agent MUST NOT invoke a tool that is not in its `permitted` list. If the agent attempts to invoke a non-permitted tool, the WOS Processor MUST block the invocation, record a guardrail violation, and increment the agent's violation count.

2. An agent MUST NOT write to the case file directly. All case file modifications resulting from agent outputs flow through the WOS Processor's normal data mutation path (WOS Core §9.4), ensuring provenance records are produced.

3. Tool invocations that access external systems MUST respect the rate limits declared in the tool registry.

4. Every tool invocation MUST be recorded in the agent session's provenance record, including the tool identifier, input, output, and duration.

5. Tools with `sideEffects: true` (tools that modify external state) MUST NOT be invoked by agents operating at `autonomous` autonomy level unless explicitly permitted by a `sideEffectPolicy` declaration.

### 10.4 Tool Output Validation

Outputs from tool invocations are incorporated into the agent's reasoning context. A WOS-Agent Processor SHOULD validate tool outputs against expected schemas before providing them to the agent. Malformed tool outputs SHOULD be intercepted and the agent SHOULD be informed of the failure, enabling graceful handling rather than corrupted reasoning.

---

## 11. Human-Agent Collaboration Patterns

This section is normative.

### 11.1 Overview

This section defines how agents interact with the human task lifecycle (WOS Core §8). The patterns described here are not exhaustive — they represent the most common collaboration modes. Implementations MAY support additional patterns via the extension mechanism.

### 11.2 Agent-Assisted Task Pattern

The most common pattern: an agent prepares a draft or recommendation, and a human completes the task by reviewing, modifying, and confirming the agent's work.

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
|--------|-------------|
| `beforeClaim` | Agent runs when the task is created (in `Available` state). Agent output is available to the human when they claim the task. |
| `onClaim` | Agent runs when a human claims the task. |
| `onDemand` | Agent runs only when the human explicitly requests assistance. |
| `parallel` | Agent runs concurrently with human work. The human may incorporate agent output at any time. |

When `timing` is `beforeClaim`, the agent's output is stored in the task's input data under a reserved `_agentAssistance` field. The task's form SHOULD present the agent's output alongside the raw case data, with visual indicators of confidence levels and flagged fields.

### 11.3 Agent-Performed Task Pattern

For tasks with `autonomy: "autonomous"`, the agent performs the task without human involvement. The task lifecycle is compressed: Created → InProgress → Completed (or Failed), with no Available or Claimed states because no human queue is involved.

The following constraints apply to agent-performed tasks:

1. The task MUST have applicable guardrails.
2. The task MUST have a fallback to human performance.
3. The task MUST produce the same output schema as the human-performed version.
4. Task completion by an agent MUST be distinguishable from human completion in provenance records.

### 11.4 Supervisory Review Pattern

For tasks with `autonomy: "supervisory"`, the agent performs the task and the result is provisionally committed. A supervisory review task is automatically created for a human supervisor.

```yaml
tasks:
  documentClassification:
    autonomy: "supervisory"
    supervisoryReview:
      taskRef: "classificationReview"
      reviewWindow: "PT4H"
      presentation:
        showAgentOutput: true
        showConfidence: true
        diffView: false
      onExpiry: "acceptAsIs"
```

If the supervisor does not act within the `reviewWindow`, the `onExpiry` action is taken. The options are `acceptAsIs` (the agent's output becomes final), `escalate` (a more senior reviewer is notified), or `reject` (the output is discarded and the task falls back to human performance).

### 11.5 Triage Pattern

An agent evaluates incoming work and routes it to the appropriate human queue. The agent does not perform the substantive task; it determines who should.

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

Triage agents are well-suited for higher autonomy levels because routing errors are recoverable (the human reviewer can reassign) and the agent is not making the substantive decision.

---

## 12. Agent Lifecycle Management

This section is normative.

### 12.1 Agent States

An agent configuration has a lifecycle within the workflow:

| State | Description |
|-------|-------------|
| `active` | Available for invocation. |
| `degraded` | Available but operating at a reduced autonomy level due to demotion. |
| `suspended` | Temporarily unavailable. All invocations route to fallback. |
| `retired` | Permanently unavailable. The agent configuration is preserved for audit but not invocable. |

### 12.2 State Transitions

| Transition | From | To | Trigger |
|-----------|------|-----|---------|
| `demote` | `active` | `degraded` | Autonomy demotion trigger (§6.4). |
| `restore` | `degraded` | `active` | Recalibration meets escalation conditions. |
| `suspend` | `active`, `degraded` | `suspended` | Manual action or operational failure. |
| `resume` | `suspended` | `active` or `degraded` | Manual action. |
| `retire` | any | `retired` | Manual action. Irreversible. |

All state transitions MUST produce a provenance record. When an agent transitions to `suspended`, all in-flight agent sessions are paused at their current checkpoint, and a notification is sent to the appropriate administrator.

### 12.3 Model Version Transitions

When an agent's effective model version changes (relevant for `approved` and `latest` version policies):

1. The WOS Processor MUST emit a provenance record of type `agentVersionChange` containing the prior version, new version, and the version policy that permitted the change.
2. If the autonomy policy includes a `modelVersionChanged` demotion trigger, the agent is demoted pending recalibration.
3. If `versionChangeNotification` is `true` (default), the appropriate administrator is notified.
4. In-flight agent sessions are NOT interrupted by version changes. Sessions complete using the version that started them. New invocations use the new version.

---

## 13. Evaluation and Drift Monitoring

This section is normative for WOS-Agent Full conformance and informative for WOS-Agent Basic conformance.

### 13.1 Calibration

Calibration measures the alignment between an agent's reported confidence and its actual accuracy. A well-calibrated agent with 0.9 confidence should be correct approximately 90% of the time.

A WOS-Agent Full Processor MUST support calibration evaluation with the following properties:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `calibrationRequired` | boolean | REQUIRED | Whether calibration is required for this agent. |
| `calibrationFrequency` | string (duration) | CONDITIONAL | How often calibration is re-evaluated. Required when `calibrationRequired` is `true`. |
| `minimumEvaluationSamples` | integer | CONDITIONAL | Minimum number of reviewed outputs required for a valid calibration assessment. |
| `calibrationMethod` | enum | OPTIONAL | `plattScaling`, `isotonic`, `binning`, or `custom`. Default: `binning`. |

Calibration data is derived from the ReviewOutcome records in agent provenance (§14). Every time a human reviews an agent's output (under `assistive` or `supervisory` autonomy), the review outcome provides a ground-truth label: `accepted` (agent was correct), `modified` (agent was partially correct), or `rejected` (agent was incorrect).

### 13.2 Drift Detection

Drift detection identifies statistically significant changes in an agent's behavior over time. Drift may indicate model degradation, data distribution changes, or environmental factors that affect agent performance.

```yaml
agents:
  riskAssessor:
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

| Method | Description |
|--------|-------------|
| `psi` | Population Stability Index. Compares the distribution of agent outputs in the current window against a reference period. |
| `ks` | Kolmogorov-Smirnov test. Detects distribution shifts in continuous output fields. |
| `chi2` | Chi-squared test. Detects distribution shifts in categorical output fields. |
| `accuracy` | Monitors accuracy trend (requires labeled data from human reviews). |

When drift is detected:

1. A provenance record of type `driftAlert` is produced.
2. The configured alert roles are notified.
3. If the autonomy policy includes a drift-triggered demotion, the demotion is applied.
4. The drift alert remains active until a human acknowledges it or recalibration is performed.

### 13.3 Performance Monitoring

A WOS-Agent Processor SHOULD track the following operational metrics per agent:

| Metric | Description |
|--------|-------------|
| `invocationCount` | Total invocations in the current period. |
| `successRate` | Proportion of invocations that completed without error. |
| `averageLatency` | Mean time from invocation to output. |
| `guardrailViolationRate` | Proportion of invocations that triggered guardrail violations. |
| `humanModificationRate` | Proportion of reviewed outputs that humans modified. |
| `humanRejectionRate` | Proportion of reviewed outputs that humans rejected. |
| `confidenceAccuracy` | Calibration accuracy (correlation between reported confidence and actual correctness). |

These metrics are available in the expression context as `agent.<metricName>` for use in autonomy policies, guardrail conditions, and dynamic autonomy selection.

---

## 14. Agent Provenance

This section is normative.

### 14.1 Overview

Agent provenance records extend the WOS Core provenance model (WOS Core §11) with agent-specific record types. All agent provenance records share the base provenance record structure (WOS Core §11.2) and add agent-specific data.

### 14.2 Record Types

This specification defines the following agent-specific provenance record types in addition to the Agent Decision Records defined in WOS Core §11.4.1:

| Record Type | Produced When |
|-------------|--------------|
| `agentInvocation` | An agent is invoked for any purpose. |
| `agentSessionStart` | A multi-step agent session begins. |
| `agentCheckpoint` | A checkpoint is recorded during a multi-step session. |
| `agentSessionEnd` | A multi-step session completes, fails, or is terminated. |
| `agentToolUse` | An agent invokes a tool during reasoning. |
| `guardrailViolation` | An agent's output violates a guardrail (defined in WOS Core §11.9). |
| `guardrailBypass` | A human bypasses a guardrail. |
| `autonomyEscalation` | An agent's autonomy level is increased. |
| `autonomyDemotion` | An agent's autonomy level is decreased. |
| `agentVersionChange` | The effective model version changes. |
| `driftAlert` | Drift detection identifies a significant behavioral change. |
| `confidenceDecayEvent` | Effective confidence of an agent output decays below threshold. |

### 14.3 Invocation Record

Every agent invocation — whether for a Decision Service, Task, or session — MUST produce an invocation record.

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

### 14.4 Tool Use Record

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

### 14.5 Provenance Completeness

A WOS-Agent Processor MUST produce provenance records for every agent interaction. The provenance stream for an agent-involved workflow MUST be sufficient to reconstruct, for any case action involving an agent:

1. Which agent was invoked, with what model version.
2. What data the agent received (by reference or summary).
3. What the agent produced, with confidence.
4. What guardrails were evaluated and their results.
5. What the effective autonomy level was and how it was determined.
6. Whether a human reviewed the agent's output, and if so, what changes the human made.
7. What tools the agent used and what results it received.
8. Whether the agent's output was ultimately used in a decision affecting the case.

---

## 15. Graceful Degradation

This section is normative.

### 15.1 Principle

Every workflow that uses agents MUST function correctly when agents are unavailable. Agent unavailability is not an edge case — it is a regular operating condition that MUST be planned for.

### 15.2 Fallback Chain Execution

When an agent invocation fails (due to error, timeout, guardrail rejection, or unavailability), the fallback chain defined in the agent configuration (§5.4) is executed in order.

The fallback chain execution follows these semantics:

1. Each level in the chain is attempted in order.
2. If a level succeeds, the chain stops and the successful result is used.
3. If a level fails, the next level is attempted.
4. Every fallback attempt MUST produce a provenance record.
5. The terminal level MUST either produce a result or transition the workflow to a state where a human takes over.

### 15.3 Degradation Modes

A workflow MAY define explicit degradation modes that adjust overall behavior when agents are unavailable:

```yaml
lifecycle:
  degradationModes:
    noAgents:
      description: "All agents unavailable. Workflow operates in fully manual mode."
      stateOverrides:
        riskAssessment:
          onEntry:
            # Replace invokeAgent with createTask
            - action: "createTask"
              taskRef: "manualRiskAssessment"
      autonomyOverride: "manual"
      notification:
        roles: ["systemAdministrator", "programDirector"]
        message: "Workflow operating in degraded mode: all agents unavailable."

    partialAgents:
      description: "Some agents unavailable. Available agents operate at reduced autonomy."
      autonomyOverride: "assistive"
```

### 15.4 Degradation Testing

A WOS-Agent Full Processor SHOULD support degradation testing: the ability to simulate agent unavailability for specific agents or all agents, verifying that fallback chains execute correctly and the workflow reaches completion.

---

## 16. Security Considerations

This section is informative.

### 16.1 Prompt Injection

When agents process case file data — application narratives, uploaded documents, correspondence — adversarial content may attempt to manipulate agent behavior. The following mitigations are RECOMMENDED:

1. **Input isolation.** Agent input preparation (§5.5) SHOULD sanitize inputs to remove or neutralize common prompt injection patterns.
2. **Output validation.** Guardrails provide a structural defense: even if the agent is manipulated, its output must pass validation constraints.
3. **Behavioral monitoring.** Drift detection (§13.2) may identify prompt injection campaigns that shift agent output distributions.
4. **Audit trail.** Agent provenance records enable post-hoc detection of injected inputs and compromised outputs.

Input sanitization is not a complete defense against prompt injection. The guardrail system is designed as a defense-in-depth layer that operates regardless of whether the agent's reasoning was compromised.

### 16.2 Model Supply Chain

The AI model is a dependency whose integrity and provenance affect the trustworthiness of workflow outputs. The following practices are RECOMMENDED:

1. Use `pinned` or `approved` version policies for consequential workflows.
2. Record model version in all provenance records (this is REQUIRED by this specification).
3. Require recalibration after model version changes.
4. Maintain an approved model registry with hash-verified model identifiers.

### 16.3 Data Exfiltration

Agents receive case data as input and may invoke tools that send data to external services. The `inputPreparation` (§5.5) and tool use governance (§10) mechanisms limit what data the agent can access and where it can send data. These mechanisms SHOULD be reviewed as part of a workflow's security assessment.

### 16.4 Cascading Agent Autonomy

As noted in WOS Core §20, an agent operating autonomously MUST NOT invoke other agents autonomously without explicit declaration. This specification further requires:

1. The `tools` registry MUST NOT include `invokeAgent` as a permitted tool type unless the workflow definition explicitly declares `cascadingAgentPolicy: "permitted"`.
2. When cascading agent invocations are permitted, the total chain depth MUST be bounded by `maxCascadeDepth` (default: 2).
3. Each step in a cascade MUST produce its own provenance record with the cascade depth recorded.

---

## 17. Privacy Considerations

This section is informative.

### 17.1 Data Minimization

Agents should receive only the case data necessary for their task. The `inputPreparation` mechanism (§5.5) provides two approaches: `redactFields` (exclude specific sensitive fields) and `includeFields` (include only specified fields). The `includeFields` approach is more conservative and is RECOMMENDED for agents that interact with external model providers.

### 17.2 Agent Processing as Data Processing

In jurisdictions with data protection regulations (GDPR, CCPA, HIPAA), agent invocations that send case data to external model providers constitute data processing activities. Implementations SHOULD:

1. Record the data processor (model provider) in provenance records.
2. Support data processing agreements with model providers.
3. Provide mechanisms to use on-premise or privacy-preserving models for sensitive data categories.
4. Ensure that `redactFields` configuration aligns with data classification policies.

### 17.3 Inference Disclosure

Agents may infer information not explicitly present in the input data (for example, inferring health conditions from financial patterns). Inferred information SHOULD be flagged as agent-derived in the case file, and visibility rules SHOULD restrict access to inferred data to the same degree as the source data from which it was inferred.

---

## 18. Conformance Testing

This section is normative.

### 18.1 Overview

Conformance testing for WOS-Agent implementations verifies that the governance framework — not the agent itself — operates correctly. The agent is an external dependency whose behavior is not governed by this specification. The WOS Processor's handling of agent outputs, guardrail enforcement, provenance production, and fallback behavior are what conformance testing verifies.

### 18.2 Test Categories

#### 18.2.1 Guardrail Enforcement Tests

Verify that:

1. Output constraint violations are detected and the correct enforcement action is taken.
2. Confidence floor violations trigger the correct response.
3. Prohibited output patterns are detected.
4. Volume constraints are enforced.
5. Human review sampling operates at the configured rate (within statistical tolerance).
6. Guardrail composition (workflow + agent + action levels) produces the correct effective guardrails.
7. Guardrail bypass produces correct provenance records and does not persist.

#### 18.2.2 Autonomy Governance Tests

Verify that:

1. The correct autonomy level is applied based on policy configuration.
2. Dynamic autonomy selection evaluates correctly.
3. Autonomy escalation requires the correct role and produces provenance.
4. Autonomy demotion triggers correctly on all defined conditions.
5. `maxAutonomy` caps are enforced.
6. Agent-performed tasks (autonomous) compress the task lifecycle correctly.
7. Supervisory review windows trigger the correct expiry action.

#### 18.2.3 Fallback Tests

Verify that:

1. Agent failure triggers the fallback chain.
2. Agent timeout triggers the fallback chain.
3. Guardrail rejection triggers the fallback chain (when enforcement action is `reject`).
4. The fallback chain executes in order.
5. The terminal fallback action creates a functional human task.
6. The workflow completes successfully via fallback without any agent participation.

#### 18.2.4 Provenance Tests

Verify that:

1. All required provenance record types are produced.
2. Agent invocation records include model identifier and version.
3. Confidence reports are included in all agent decision records.
4. Review outcomes are recorded when humans review agent outputs.
5. Guardrail violation records identify the violated constraint.
6. Autonomy change records capture before/after states.
7. Tool use records capture inputs and outputs.

#### 18.2.5 Multi-Step Session Tests (WOS-Agent Full)

Verify that:

1. Checkpoints are produced at the configured policy intervals.
2. Intervention points pause execution and create review tasks.
3. Cumulative confidence is tracked and triggers review when below threshold.
4. Session timeout triggers the configured action.
5. Session failure triggers rollback to the last checkpoint.
6. Human modifications at intervention points are used in subsequent steps.

### 18.3 Canonical Test Fixtures

The conformance test suite MUST include the following canonical fixtures:

**Fixture A: Basic Agent Decision with Guardrails.** An agent evaluates an eligibility decision. Tests: invocation, confidence reporting, guardrail enforcement (pass and fail cases), human review under assistive autonomy, provenance production.

**Fixture B: Graduated Autonomy Routing.** The same decision is invoked across multiple cases with varying complexity and confidence levels. Tests: dynamic autonomy selection, confidence-based routing, escalation and demotion triggers.

**Fixture C: Graceful Degradation.** Agent is unavailable (simulated). Tests: fallback chain execution, human task creation, workflow completion without agents.

**Fixture D: Multi-Step Session with Intervention (WOS-Agent Full).** A multi-step evidence analysis session with a human intervention point. Tests: checkpoint production, intervention task creation, human modification of intermediate output, cumulative confidence tracking, session completion.

**Fixture E: Guardrail Bypass.** An authorized human bypasses a guardrail with structured rationale. Tests: bypass authorization verification, provenance recording, non-persistence of bypass.

---

## 19. References

### 19.1 Normative References

**[WOS-Core]** Workflow Orchestration Standard Core Specification, Version 0.1.0, 8 April 2026.

All normative references from WOS Core §21.1 are incorporated by reference.

### 19.2 Informative References

**[Guo2024]** Guo, Z. et al., "A Survey on Large Language Model based Autonomous Agents", Frontiers of Computer Science, 2024.

**[NIST-AI-600-1]** NIST, "Artificial Intelligence Risk Management Framework: Generative AI Profile", NIST AI 600-1, July 2024.

**[EU-AI-Act]** European Parliament and Council, "Regulation (EU) 2024/1689 laying down harmonised rules on artificial intelligence", June 2024.

**[Anthropic-RSP]** Anthropic, "Responsible Scaling Policy", 2023.

**[OMB-M-24-10]** Office of Management and Budget, "Advancing Governance, Innovation, and Risk Management for Agency Use of Artificial Intelligence", Memorandum M-24-10, March 2024.

---

## Appendix A. Complete Example

This appendix is informative. The following example extends the WOS Core grant review workflow (Core Appendix B) with agent configurations.

```yaml
# Agent configurations added to the grant review workflow
agents:
  completenessChecker:
    description: >
      Checks application packages for missing required documents
      and form completeness. Low-risk, high-volume task suitable
      for elevated autonomy after calibration.
    version: "1.0.0"
    model:
      provider: "anthropic"
      identifier: "claude-sonnet-4-20250514"
      versionPolicy: "approved"
      approvedVersions: ["20250514"]
    capabilities:
      - id: "completenessAssessment"
        taskRef: "completenessCheck"
    defaultAutonomy: "assistive"
    autonomyPolicy:
      default: "assistive"
      escalation:
        toAutonomous:
          conditions:
            - "agent.calibration.accuracy >= 0.98"
            - "agent.recentViolations(P30D) = 0"
          approval:
            roles: ["programDirector"]
            expires: "P90D"
      perCapability:
        completenessAssessment:
          maxAutonomy: "autonomous"
    guardrails:
      confidenceFloor:
        threshold: 0.85
        onViolation: "escalateToHuman"
      outputConstraints:
        - field: "isComplete"
          constraint: "value = true or (value = false and count(output.missingItems) > 0)"
          onViolation: "reject"
          reason: "If incomplete, missing items must be identified."
      humanReviewSampling:
        rate: 0.10
        method: "random"
    fallback:
      primary:
        onFailure: "retry"
        maxRetries: 2
        backoff: "exponential"
        initialInterval: "PT5S"
      terminal:
        onFailure: "escalateToHuman"
        taskRef: "completenessCheck"
    inputPreparation:
      sanitize: true
      redactFields:
        - "caseFile.application.socialSecurityNumber"
    evaluation:
      calibrationRequired: true
      calibrationFrequency: "P30D"
      minimumEvaluationSamples: 100

  eligibilityScreener:
    description: >
      Pre-screens applications against eligibility criteria.
      Always operates in assistive mode due to the consequential
      nature of eligibility determinations.
    version: "1.0.0"
    model:
      provider: "anthropic"
      identifier: "claude-sonnet-4-20250514"
      versionPolicy: "pinned"
    capabilities:
      - id: "eligibilityPreScreen"
        decisionRef: "eligibilityDetermination"
    defaultAutonomy: "assistive"
    autonomyPolicy:
      perCapability:
        eligibilityPreScreen:
          maxAutonomy: "assistive"
          reason: >
            Eligibility determinations directly affect applicant rights.
            Human confirmation is always required per program policy
            and 24 CFR 570.
    guardrails:
      confidenceFloor:
        threshold: 0.7
        onViolation: "escalateToHuman"
      outputConstraints:
        - field: "eligible"
          constraint: "value = true or value = false"
          onViolation: "reject"
        - field: "reason"
          constraint: "string length(value) >= 20"
          onViolation: "reject"
          reason: "Eligibility determination must include substantive reasoning."
      consistency:
        - name: "incomeConsistency"
          check: >
            not (output.eligible = true
                 and caseFile.application.areaMedianIncome
                     > parameters.amiThreshold(caseFile.application.submittedDate) * 1.5)
          onViolation: "escalateToHuman"
          reason: "Eligible finding with income significantly above threshold requires review."
    fallback:
      terminal:
        onFailure: "escalateToHuman"
        taskRef: "manualEligibilityScreening"
    inputPreparation:
      includeFields:
        - "caseFile.application.proposedActivities"
        - "caseFile.application.populationServed"
        - "caseFile.application.areaMedianIncome"
        - "caseFile.application.submittedDate"
    evaluation:
      calibrationRequired: true
      calibrationFrequency: "P14D"
      minimumEvaluationSamples: 50
      driftDetection:
        enabled: true
        method: "psi"
        threshold: 0.15
        window: "P7D"
        dimensions:
          - field: "eligible"
            type: "categorical"
        onDetection: "alert"
        alertRoles: ["programDirector"]

# Additional task for manual fallback
tasks:
  manualEligibilityScreening:
    version: "1.0.0"
    description: >
      Manual eligibility screening, used as fallback when the
      eligibility screening agent is unavailable or produces
      insufficient confidence.
    form:
      inputSchema:
        type: "object"
        properties:
          proposedActivities:
            type: "array"
          populationServed:
            type: "integer"
          areaMedianIncome:
            type: "number"
      outputSchema:
        $ref: "#/decisions/eligibilityDetermination/outputs"
    assignment:
      potentialOwners:
        roles: ["eligibilitySpecialist"]
      businessAdministrators:
        roles: ["programDirector"]
    sla:
      dueIn: "P3BD"
      businessCalendar: "federalWorkdays"
```

---

## Appendix B. Autonomy Level Decision Guide

This appendix is informative. The following guide helps workflow designers select the appropriate autonomy level for agent actions.

**Use `autonomous` when all of the following are true:**
The action is low-risk (errors are easily detectable and reversible). The agent is well-calibrated with sustained high accuracy. Guardrails can structurally validate the output. The action is high-volume, making human review for every instance impractical. Policy permits autonomous processing for this action type.

**Use `assistive` when any of the following are true:**
The action affects individual rights or entitlements. The action involves subjective judgment. The action has regulatory review requirements. The agent's calibration is not yet established. The output will be relied upon for a consequential downstream decision.

**Use `supervisory` when all of the following are true:**
The action benefits from agent speed (time-sensitive processing). The action has a natural review cycle where a supervisor would review anyway. Error detection is feasible within the review window. The cost of a brief delay (the review window) is acceptable.

**Use `manual` when any of the following are true:**
The action requires professional certification (legal opinion, medical diagnosis). The action has explicit regulatory requirements for human performance. The case involves unprecedented circumstances outside the agent's training distribution. The agent's calibration has degraded below acceptable thresholds.

---

## Appendix C. Guardrail Pattern Catalog

This appendix is informative. The following patterns represent common guardrail configurations for high-stakes workflows.

**Pattern: Belt and Suspenders.** Apply both output constraints (structural) and consistency checks (semantic) to the same output. Structural checks catch format errors; consistency checks catch logical errors.

**Pattern: Progressive Trust.** Start with `assistive` autonomy and strict guardrails. After calibration demonstrates sustained accuracy, escalate to `supervisory`. After extended `supervisory` operation with low modification rates, escalate to `autonomous`. Encode this progression in the autonomy policy with explicit thresholds.

**Pattern: High-Water Mark.** Track the highest-risk case an agent has processed autonomously. Alert when a new case exceeds the high-water mark (e.g., the requested amount is higher than any previously processed autonomously). This catches cases that are technically within guardrail bounds but represent new territory for the agent.

**Pattern: Shadow Mode.** Run the agent in parallel with human performance but do not use the agent's output. Compare agent outputs to human outputs to build calibration data before giving the agent any operational authority.

**Pattern: Circuit Breaker.** If the guardrail violation rate exceeds a threshold within a window (e.g., 3 violations in 1 hour), automatically suspend the agent and route all work to human fallback. This prevents sustained agent misbehavior from accumulating harm.
