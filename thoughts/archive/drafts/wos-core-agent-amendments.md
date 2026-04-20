# WOS Core Specification — AI Agent Foundation Amendments

## Amendment Instructions

This document specifies additions and modifications to the WOS Core Specification v0.1.0 to establish foundational support for AI agent participation in workflows. These amendments add the minimum necessary vocabulary, semantics, and provenance structures to the core so that AI agents are first-class participants rather than ad-hoc implementation details.

Each amendment identifies the target section, the nature of the change, and the full normative text to be incorporated.

---

## Amendment 1: Terminology Additions (§3)

**Nature:** Add new defined terms to the Terminology section.

**Insert the following terms in alphabetical order:**

---

**Actor.** An entity that performs actions within a workflow. Actors are classified by type: `human` (a person), `system` (a deterministic software component), or `agent` (an AI system capable of non-deterministic reasoning). All actors are identified, authenticated, and recorded in provenance records.

**Agent.** An AI system that participates in a workflow by performing tasks, evaluating decisions, or producing recommendations. Agents operate under declared autonomy levels and are subject to guardrail constraints, confidence reporting, and human oversight requirements. An agent is a type of Actor.

**Autonomy Level.** A declared classification governing how much independent authority an actor has over a workflow action. Autonomy levels control whether an action is performed without human involvement, performed with human review before commitment, or performed entirely by a human with optional AI assistance.

**Confidence.** A structured assessment of certainty associated with an agent's output. Confidence values are reported in provenance records and may be used in guard conditions to route work between autonomous processing and human review.

**Guardrail.** A declarative constraint on an agent's behavior that restricts its outputs, actions, or authority boundaries. Guardrails are enforced by the WOS Processor and violations are recorded in the provenance stream.

---

## Amendment 2: Actor Model (new §4.4)

**Nature:** Add a new subsection to §4 Architecture Overview, after §4.3 Cross-Cutting Concerns.

**Insert:**

---

### 4.4 Actor Model

This section is normative.

WOS recognizes three types of actors that participate in workflows:

| Actor Type | Description | Provenance Requirements |
|------------|-------------|------------------------|
| `human` | A person who performs tasks, makes decisions, and exercises judgment. Identified by user identity. | Identity, role, timestamp. |
| `system` | A deterministic software component that executes automated actions, integrations, and rule evaluations. | Component identifier, version, timestamp. |
| `agent` | An AI system that performs reasoning, classification, recommendation, or decision-making. Outputs are non-deterministic and carry confidence metadata. | Model identifier, model version, confidence, input summary, timestamp. |

The actor type distinction is not merely descriptive. It carries normative consequences:

1. **Provenance records** MUST include the actor type. Records for `agent` actors MUST include additional fields defined in §11.4.1.
2. **Autonomy levels** (§6.9) govern the relationship between agent outputs and workflow progression.
3. **Guardrails** (§7.7) constrain agent behavior. Guardrails do not apply to `human` or `system` actors.
4. **Override authority** (§8.8) allows `human` actors to supersede `agent` outputs. `agent` actors MUST NOT override `human` decisions.

An actor's type is immutable for a given action. A human using an AI tool remains a `human` actor if the human reviews and commits the output. An AI system operating autonomously is an `agent` actor even if a human configured it. The distinction is based on who bears decision authority for the specific action, not on the presence or absence of AI tooling.

---

## Amendment 3: Autonomy Levels (new §6.9)

**Nature:** Add a new subsection to §6 Layer 1: Lifecycle and Topology, after §6.8 Soundness Verification.

**Insert:**

---

### 6.9 Autonomy Levels

This section is normative.

Every action in a WOS workflow operates at a declared autonomy level that governs the relationship between AI agent outputs and workflow progression. Autonomy levels are a property of the action site — the point in the workflow where an agent is invoked — not a property of the agent itself. The same agent may operate at different autonomy levels in different workflow contexts.

The following autonomy levels are defined:

| Level | Name | Semantics |
|-------|------|-----------|
| `autonomous` | Full autonomy | The agent's output is committed directly to the case file and drives workflow progression without human review. Appropriate only when the action is low-risk, the agent's reliability is established, and policy permits autonomous operation. |
| `assistive` | Human-confirmed | The agent produces a recommendation. A human reviews, may modify, and explicitly confirms before the output is committed. The confirmed output is attributed to the human actor; the agent's recommendation is recorded in provenance. |
| `supervisory` | Human-supervised | The agent executes the action, and the output is provisionally committed. A human supervisor reviews within a defined window. If the supervisor does not intervene, the output becomes final. If the supervisor modifies or rejects, the modification is recorded as an override. |
| `manual` | Human-performed | The action is performed entirely by a human. An agent MAY provide contextual assistance (information retrieval, draft generation) but the output is solely the human's. Agent assistance, if any, is recorded in provenance but does not affect attribution. |

Autonomy levels are declared on actions, task definitions, and decision service invocations:

```yaml
# On an action within a lifecycle transition
actions:
  - action: "invokeDecision"
    decisionRef: "riskClassification"
    outputBinding: "caseFile.risk"
    autonomy: "assistive"

# On a task definition
tasks:
  documentReview:
    autonomy:
      default: "assistive"
      overridePolicy:
        allowEscalation: true    # Can escalate to manual
        allowDemotion: false     # Cannot demote to autonomous
```

#### 6.9.1 Autonomy Level Routing

A workflow MAY use confidence-based routing to dynamically select the appropriate autonomy level for an action. This is modeled as a guard condition on transitions:

```yaml
transitions:
  - event: "decision.complete"
    target: "autoApproved"
    guard: >
      caseFile.risk.confidence >= 0.95
      and caseFile.risk.classification = 'low'
    description: "High-confidence low-risk: proceed autonomously."

  - event: "decision.complete"
    target: "humanReview"
    guard: "caseFile.risk.confidence < 0.95"
    description: "Insufficient confidence: route to human reviewer."
```

This pattern — invoking an agent, capturing its confidence, and routing based on confidence thresholds — is the RECOMMENDED approach for graduated autonomy. The confidence thresholds themselves SHOULD be modeled as temporal parameters (§7.5) so they can be adjusted based on operational experience without modifying the workflow definition.

#### 6.9.2 Autonomy Level Constraints

The following constraints are normative:

1. An action with `autonomy: "autonomous"` MUST have an associated guardrail definition (§7.7) unless the workflow definition explicitly declares `guardrails: "none"` for that action. This is a safety mechanism: autonomous AI actions without declared constraints are a design error, and the specification treats them as such.
2. An action with `autonomy: "assistive"` MUST create a human task for confirmation. The task's input schema MUST include the agent's recommendation and confidence.
3. An action with `autonomy: "supervisory"` MUST define a `reviewWindow` (an ISO 8601 duration) within which the supervisor may intervene. If the review window expires without intervention, the agent's output is finalized and a provenance record noting the implicit confirmation is produced.
4. A WOS Document SHOULD declare a `defaultAutonomy` at the workflow level. If no autonomy level is declared on a specific action, the `defaultAutonomy` applies. If no `defaultAutonomy` is declared, the effective default is `manual`.

```yaml
lifecycle:
  defaultAutonomy: "assistive"
  # ...
```

---

## Amendment 4: Decision Provenance Extension (amend §11.4)

**Nature:** Add §11.4.1 as a new subsection under the existing §11.4 Decision Records.

**Insert after §11.4:**

---

#### 11.4.1 Agent Decision Records

When a Decision Service is evaluated by an `agent` actor, the Decision Record MUST include the following additional fields beyond those required by §11.4:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `agentModel` | string | REQUIRED | Identifier of the AI model (e.g., `claude-sonnet-4-20250514`, `gpt-4o-2024-08-06`). |
| `agentModelVersion` | string | REQUIRED | Specific version or checkpoint of the model. |
| `confidence` | ConfidenceReport | REQUIRED | Structured confidence assessment. |
| `inputRepresentation` | string | OPTIONAL | A summary or hash of the input provided to the agent, sufficient for reproducibility assessment. Full inputs MAY be stored by reference. |
| `alternativesConsidered` | array of AlternativeOutput | OPTIONAL | Other outputs the agent considered, if available. |
| `reviewOutcome` | ReviewOutcome | OPTIONAL | If the autonomy level required human review, the outcome of that review. |

**ConfidenceReport** is an object with the following structure:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `overall` | number (0.0–1.0) | REQUIRED | A scalar confidence value. |
| `method` | enum | REQUIRED | How confidence was derived: `modelNative` (from the model's own calibration), `calibrated` (post-hoc calibration applied), `heuristic` (rule-based approximation), or `declared` (manually assigned by the agent developer). |
| `explanation` | string | OPTIONAL | Human-readable explanation of confidence factors. |
| `fieldLevel` | object | OPTIONAL | Per-output-field confidence values, keyed by field name. |

**AlternativeOutput** is an object with:

| Field | Type | Description |
|-------|------|-------------|
| `output` | object | The alternative output values. |
| `confidence` | number | Confidence for this alternative. |
| `reason` | string | Why this alternative was not selected. |

**ReviewOutcome** is an object with:

| Field | Type | Description |
|-------|------|-------------|
| `reviewer` | ActorRef | The human who reviewed the agent's output. |
| `action` | enum | `accepted` (output used as-is), `modified` (output changed), `rejected` (output discarded, human provided replacement). |
| `modifications` | object | If `modified`, the fields that were changed with before/after values. |
| `rationale` | string | Reviewer's explanation, REQUIRED if action is `modified` or `rejected`. |
| `reviewDuration` | string (duration) | Time between agent output and human review completion. |

Example:

```yaml
recordType: "decision"
data:
  decisionRef: "documentClassification"
  decisionVersion: "3.0.0"
  actor:
    type: "agent"
    id: "urn:agent:doc-classifier-v3"
  agentModel: "claude-sonnet-4-20250514"
  agentModelVersion: "20250514"
  inputs:
    documentRef: "urn:evidence:doc-2026-04-abc"
    documentType: "financial-statement"
  confidence:
    overall: 0.87
    method: "calibrated"
    explanation: "High structural match but unusual formatting reduced confidence."
    fieldLevel:
      classification: 0.92
      extractedEntities: 0.78
  outputs:
    classification: "auditedFinancial"
    extractedEntities:
      totalRevenue: 1250000
      fiscalYear: 2025
  alternativesConsidered:
    - output:
        classification: "unauditedFinancial"
      confidence: 0.08
      reason: "Auditor signature block detected."
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

---

## Amendment 5: Guardrail Definitions (new §7.7)

**Nature:** Add a new subsection to §7 Layer 2: Decision and Policy, after §7.6 Decision Invocation and Provenance.

**Insert:**

---

### 7.7 Guardrails

This section is normative.

Guardrails are declarative constraints on agent behavior that are enforced by the WOS Processor at the point of agent invocation. Guardrails operate independently of the agent itself — they constrain the agent's outputs after production, not its internal reasoning. This separation is essential: the workflow definition, not the agent, is the authority on what outputs are acceptable.

Guardrails are defined at the decision service level, the task level, or the workflow level:

```yaml
decisions:
  riskClassification:
    # ...
    guardrails:
      outputConstraints:
        - field: "riskScore"
          constraint: "value >= 0 and value <= 100"
          onViolation: "reject"

        - field: "classification"
          constraint: "value in ['low', 'medium', 'high', 'critical']"
          onViolation: "reject"

      confidenceFloor:
        threshold: 0.6
        onViolation: "escalateToHuman"

      prohibitedOutputs:
        - condition: "classification = 'low' and riskScore > 70"
          reason: "Inconsistent classification and score."
          onViolation: "escalateToHuman"

      volumeConstraints:
        maxAutonomousPerHour: 50
        onExceeded: "switchToAssistive"

      humanReviewSampling:
        rate: 0.05
        method: "random"
        description: "5% random sampling for quality assurance."
```

#### 7.7.1 Guardrail Types

The following guardrail types are defined:

**Output Constraints.** Validate that agent outputs conform to declared boundaries. These are structural and semantic checks applied to the output values.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `field` | string | REQUIRED | The output field to constrain. |
| `constraint` | string (expression) | REQUIRED | A WOS Expression that MUST evaluate to `true` for the output to be accepted. |
| `onViolation` | enum | REQUIRED | Action on violation: `reject` (discard output, raise error), `escalateToHuman` (route to human review), `flag` (accept but flag in provenance). |
| `reason` | string | OPTIONAL | Explanation of why this constraint exists. |

**Confidence Floor.** Require a minimum confidence level for autonomous processing.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `threshold` | number (0.0–1.0) | REQUIRED | Minimum acceptable confidence for the declared autonomy level. |
| `onViolation` | enum | REQUIRED | `escalateToHuman`, `reject`, or `retry`. |

**Prohibited Outputs.** Detect logically inconsistent or policy-violating output combinations.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `condition` | string (expression) | REQUIRED | A condition that, if `true`, indicates a prohibited output. |
| `reason` | string | REQUIRED | Explanation of the prohibition. |
| `onViolation` | enum | REQUIRED | `reject`, `escalateToHuman`, or `flag`. |

**Volume Constraints.** Limit the rate of autonomous agent actions to prevent runaway automation.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `maxAutonomousPerHour` | integer | OPTIONAL | Maximum autonomous actions per hour. |
| `maxAutonomousPerDay` | integer | OPTIONAL | Maximum autonomous actions per day. |
| `onExceeded` | enum | REQUIRED | `switchToAssistive` (demote autonomy level), `pause` (halt agent processing), or `flag`. |

**Human Review Sampling.** Require a percentage of agent-processed actions to be reviewed by humans for quality assurance, even when the agent's output meets all other constraints.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `rate` | number (0.0–1.0) | REQUIRED | Proportion of actions to sample. |
| `method` | enum | REQUIRED | `random`, `stratified` (sample proportionally across output categories), or `adversarial` (preferentially sample low-confidence outputs). |

#### 7.7.2 Guardrail Enforcement

Guardrail evaluation occurs after the agent produces its output and before the output is committed to the case file or used to drive transitions. The evaluation order is:

1. Output Constraints (structural validity).
2. Prohibited Outputs (semantic validity).
3. Confidence Floor (certainty threshold).
4. Volume Constraints (rate limiting).
5. Human Review Sampling (quality assurance selection).

A guardrail violation MUST produce a provenance record of type `guardrailViolation` containing: the guardrail identifier, the violation type, the agent's output that triggered the violation, and the enforcement action taken.

If multiple guardrails are violated simultaneously, the most restrictive enforcement action applies. The restriction ordering is: `reject` > `escalateToHuman` > `switchToAssistive` > `flag`.

---

## Amendment 6: Provenance Record Type Addition (amend §11.2)

**Nature:** Add `guardrailViolation` to the `recordType` enumeration in §11.2.

**Add to the recordType enum:**

| Value | Description |
|-------|-------------|
| `guardrailViolation` | A guardrail constraint was violated by an agent's output. |

**Add new §11.9:**

---

### 11.9 Guardrail Violation Records

Produced when an agent's output violates a guardrail constraint.

```yaml
recordType: "guardrailViolation"
data:
  decisionRef: "riskClassification"
  agentModel: "claude-sonnet-4-20250514"
  guardrailId: "outputConstraints[0]"
  guardrailType: "outputConstraint"
  violationDetail:
    field: "riskScore"
    constraint: "value >= 0 and value <= 100"
    actualValue: 142
  agentOutput:
    riskScore: 142
    classification: "high"
  enforcementAction: "reject"
  resolution: "Agent re-invoked after input correction."
```

---

## Amendment 7: Action Type Additions (amend §6.5)

**Nature:** Add new action types to the table in §6.5.

**Add to the action types table:**

| Action Type | Description | Key Properties |
|-------------|-------------|----------------|
| `invokeAgent` | Invokes an AI agent to perform reasoning, classification, or recommendation. | `agentRef`: reference to an agent configuration. `decisionRef`: optional reference to a Decision Service the agent evaluates. `autonomy`: autonomy level for this invocation. `guardrailRef`: reference to applicable guardrails. |
| `awaitReview` | Creates a human review task for an agent's output. | `agentOutputBinding`: case file path where the agent's output is stored. `taskRef`: reference to the review task definition. `reviewWindow`: duration within which review must occur. |

---

## Amendment 8: Extension to Document Structure (amend §5.1)

**Nature:** Add an `agents` top-level property to the document structure.

**Add to the top-level structure in §5.1:**

```yaml
agents:                                # OPTIONAL
  # Agent configurations and policies
  ...
```

**Add to §5, after §5.6:**

---

### 5.7 Property: `agents`

The `agents` property is OPTIONAL. When present, it defines the AI agent configurations available for use in this workflow. Each agent configuration declares the agent's identity, capabilities, default autonomy level, applicable guardrails, and operational constraints.

```yaml
agents:
  documentClassifier:
    description: "Classifies uploaded documents by type and extracts key fields."
    model:
      provider: "anthropic"
      identifier: "claude-sonnet-4-20250514"
      versionPolicy: "pinned"
    capabilities:
      - "documentClassification"
      - "fieldExtraction"
    defaultAutonomy: "assistive"
    guardrails:
      $ref: "#/decisions/documentClassification/guardrails"
    fallback:
      onFailure: "escalateToHuman"
      taskRef: "manualDocumentClassification"
    operationalWindow:
      maxLatency: "PT30S"
      maxConcurrent: 10
```

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `description` | string | OPTIONAL | Human-readable description of the agent's role. |
| `model` | ModelConfig | REQUIRED | Identifies the AI model and versioning policy. |
| `capabilities` | array of string | OPTIONAL | Named capabilities this agent provides. |
| `defaultAutonomy` | enum | REQUIRED | Default autonomy level when invoked without explicit override. |
| `guardrails` | GuardrailDef or reference | OPTIONAL | Guardrail constraints specific to this agent. |
| `fallback` | FallbackDef | REQUIRED | What happens when the agent fails or is unavailable. |
| `operationalWindow` | OperationalWindow | OPTIONAL | Performance and capacity constraints. |

**ModelConfig** specifies the AI model:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `provider` | string | REQUIRED | The model provider identifier. |
| `identifier` | string | REQUIRED | The model identifier. |
| `versionPolicy` | enum | REQUIRED | `pinned` (use the exact version specified), `latest` (use the provider's current version), or `approved` (use the latest version from an approved list). |
| `approvedVersions` | array of string | OPTIONAL | When `versionPolicy` is `approved`, the list of acceptable versions. |

**FallbackDef** specifies degradation behavior:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `onFailure` | enum | REQUIRED | `escalateToHuman` (create a human task), `retry` (retry with backoff), `alternateAgent` (try a different agent), or `fail` (fail the action). |
| `taskRef` | string | OPTIONAL | Reference to a human task definition for fallback. Required when `onFailure` is `escalateToHuman`. |
| `alternateAgentRef` | string | OPTIONAL | Reference to a fallback agent. Required when `onFailure` is `alternateAgent`. |

---

## Amendment 9: Conformance Profile Extension (amend §18)

**Nature:** Add a new conformance profile for agent support.

**Add after §18.6:**

---

### 18.7 Profile: Agent

An Agent conformant implementation satisfies Lifecycle and Decision conformance and additionally:

1. MUST recognize and enforce autonomy levels (§6.9) on actions, tasks, and decision invocations.
2. MUST produce Agent Decision Records (§11.4.1) for all agent-evaluated decisions, including confidence reporting.
3. MUST enforce guardrail constraints (§7.7) and produce Guardrail Violation Records (§11.9) on violations.
4. MUST support confidence-based routing in guard expressions.
5. MUST enforce fallback behavior when agents fail or are unavailable.
6. MUST prevent agents from overriding human decisions.
7. MUST record the model identifier and version in all agent provenance records.
8. SHOULD support human review sampling for quality assurance.
9. SHOULD support volume constraints on autonomous agent actions.

**Renumber existing §18.7 (Verification) to §18.8.**

---

## Amendment 10: Security Considerations Extension (amend §20)

**Nature:** Add agent-specific security considerations.

**Add to §20:**

---

6. **Agent impersonation.** Implementations MUST authenticate agent actors with the same rigor as human actors. An agent MUST NOT be able to claim a human actor identity, and a human MUST NOT be able to impersonate an agent to bypass guardrail enforcement.

7. **Prompt injection via case data.** When agents process case file data (documents, form inputs, correspondence), adversarial content in the data may attempt to manipulate agent behavior. Implementations SHOULD sanitize or isolate agent inputs and MUST treat guardrail constraints as the authoritative boundary regardless of agent output.

8. **Model version drift.** When an agent's model version changes, its behavior may change in ways that affect workflow outcomes. Implementations using `versionPolicy: "latest"` SHOULD monitor output distribution shifts and SHOULD support automated alerting when agent behavior diverges significantly from historical patterns.

9. **Cascading autonomy.** An agent operating at `autonomous` level MUST NOT invoke other agents at `autonomous` level without explicit declaration in the workflow definition. Unsupervised chains of autonomous agent actions compound risk. The workflow definition MUST make cascading autonomy visible and auditable.

---

## Summary of Amendments

| # | Target Section | Change | Rationale |
|---|---------------|--------|-----------|
| 1 | §3 Terminology | Add Actor, Agent, Autonomy Level, Confidence, Guardrail | Vocabulary foundation |
| 2 | §4.4 (new) | Actor Model with three types | Distinguish human/system/agent provenance requirements |
| 3 | §6.9 (new) | Autonomy Levels | Graduated human oversight for agent actions |
| 4 | §11.4.1 (new) | Agent Decision Records | Confidence, model version, review outcome provenance |
| 5 | §7.7 (new) | Guardrails | Declarative constraints on agent outputs |
| 6 | §11.2, §11.9 | Guardrail Violation Records | Audit trail for constraint enforcement |
| 7 | §6.5 | invokeAgent, awaitReview actions | Agent invocation and review action types |
| 8 | §5.1, §5.7 | agents top-level property | Agent configuration with model, fallback, operational window |
| 9 | §18.7 (new) | Agent Conformance Profile | Testable conformance for agent support |
| 10 | §20 | Security Considerations | Impersonation, prompt injection, version drift, cascading autonomy |
