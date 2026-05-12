# Workflow Orchestration Standard (WOS) Core Specification

## W3C First Public Working Draft

**Latest published version:**
: https://wos-spec.org/TR/wos-core/

**Editor's Draft:**
: https://wos-spec.org/ed/wos-core/

**Editors:**
: Mike (TealWolf Consulting LLC)

**Version:**
: 4.0.0

**Date:**
: 9 April 2026

**Status:**
: First Public Working Draft

---

## Abstract

This specification defines the Workflow Orchestration Standard (WOS), a declarative, machine-readable language for describing high-stakes, long-running workflows in which humans and AI agents collaborate on consequential decisions.

WOS is organized as a **Constraint-Enhanced Layered Kernel**: a minimal, highly stable core focused on durable state transitions, object-centric case modeling, and cryptographic provenance, surrounded by independently adoptable profiles for decision logic, human task management, agent governance, and integration. This architecture ensures that the core remains microscopic and mathematically verifiable, while domain-specific complexity is delegated to interchangeable, progressively adoptable profile layers.

The standard introduces three foundational innovations over prior workflow specifications:

**A tripartite object model** separating ActivityDefinitions (reusable, independently versionable work templates), WorkflowDefinitions (process topologies that compose activities), and Tasks (runtime instances). This enables cross-program sharing, compositional AI-assisted authoring, and granular version management.

**A deontic governance framework** classifying all agent constraints as Permissions, Prohibitions, Obligations, or Rights — adopted from OASIS LegalRuleML — providing the formal vocabulary necessary to govern autonomous actors operating with broad operational mandates in regulated environments.

**A typed patch operation vocabulary** for AI-assisted authoring, enabling LLMs to propose structural edits to workflow definitions as statically analyzable operations rather than brittle text diffs.

WOS documents are natively serialized as JSON-LD. Every document is simultaneously valid JSON, valid JSON-LD, and a serialization of an RDF graph. Guardrails may be expressed as SHACL shapes. Provenance records align with W3C PROV-O and PROV-AGENT. Case data links to domain vocabularies through JSON-LD context extension.

The standard treats human authority as supreme, AI participation as governed, and audit as foundational. It is informed by empirical research demonstrating that naive human-in-the-loop designs degrade decision quality, that model-generated explanations are unreliable audit evidence, that behavioral drift between model versions can be catastrophic, and that no single defense prevents prompt injection. These findings are encoded as structural requirements.

---

## Status of This Document

This document is a First Public Working Draft. It has not been endorsed by any standards body. It is published to solicit feedback from implementers, domain experts, standards practitioners, and the broader workflow, case management, AI governance, and semantic web communities. Comments may be submitted as issues at the specification's repository.

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Conformance](#2-conformance)
3. [Terminology](#3-terminology)
4. [Architecture Overview](#4-architecture-overview)
5. [Document Model and Serialization](#5-document-model-and-serialization)
6. [Kernel Layer 1: Lifecycle and Topology](#6-kernel-layer-1-lifecycle-and-topology)
7. [Profile Layer 2: Decision and Policy](#7-profile-layer-2-decision-and-policy)
8. [Profile Layer 3: Human Task Management](#8-profile-layer-3-human-task-management)
9. [Profile Layer 4: Agent Governance](#9-profile-layer-4-agent-governance)
10. [Kernel Layer 5: Case State and Evidence](#10-kernel-layer-5-case-state-and-evidence)
11. [Profile Layer 6: Integration and Eventing](#11-profile-layer-6-integration-and-eventing)
12. [Kernel Layer 7: Provenance and Audit](#12-kernel-layer-7-provenance-and-audit)
13. [Kernel Layer 8: Durable Execution Contract](#13-kernel-layer-8-durable-execution-contract)
14. [Actor Model](#14-actor-model)
15. [Expression Language](#15-expression-language)
16. [Due Process Requirements](#16-due-process-requirements)
17. [AI-Native Authoring and Patch Operations](#17-ai-native-authoring-and-patch-operations)
18. [Versioning and Evolution](#18-versioning-and-evolution)
19. [Security and Access Control](#19-security-and-access-control)
20. [Extensibility](#20-extensibility)
21. [Conformance Profiles](#21-conformance-profiles)
22. [Privacy Considerations](#22-privacy-considerations)
23. [Security Considerations](#23-security-considerations)
24. [References](#24-references)

**Appendices**

- [A. JSON-LD Context Document](#appendix-a-json-ld-context-document)
- [B. SHACL Shapes for Structural Governance](#appendix-b-shacl-shapes-for-structural-governance)
- [C. Patch Operation Reference](#appendix-c-patch-operation-reference)
- [D. Relationship to Existing Standards](#appendix-d-relationship-to-existing-standards)
- [E. Changelog](#appendix-e-changelog)

---

## 1. Introduction

### 1.1 Background

High-stakes workflows — grants processing, benefits adjudication, licensing, inspections, investigations, compliance review — share requirements that no existing standard adequately integrates. They are long-running, human-centric, evidence-driven, heavily regulated, and increasingly involve AI agents. This specification addresses these requirements by treating agent governance, structured human oversight, formal constraint enforcement, verifiable provenance, and due process protections as foundational architectural concerns.

The specification is informed by three bodies of empirical evidence that constrain its design. First, a meta-analysis of 106 experiments demonstrates that naive human-AI combinations degrade decision quality compared to either humans or AI alone (Vaccaro et al., Nature Human Behaviour, 2024), necessitating structured oversight protocols rather than checkbox review. Second, research on chain-of-thought faithfulness demonstrates that model-generated explanations are systematically post-hoc rationalizations that do not faithfully represent actual reasoning (Turpin et al., NeurIPS 2023; Lanham et al., Anthropic, 2023), necessitating a four-layer audit architecture that distinguishes immutable facts from generated narrative. Third, documented government AI failures — Michigan's MiDAS (93% false positive rate), Arkansas's RUGs algorithm, the Dutch childcare benefits scandal, Australia's Robodebt — establish that due process protections, appeal mechanisms, and continuation-of-service guarantees are non-negotiable architectural requirements for rights-impacting workflows.

### 1.2 Design Goals

1. **Human authority is supreme.** Agent recommendations MUST NOT be the sole or determinative factor in adverse decisions affecting individual rights.
2. **Structured oversight, not checkbox review.** Human oversight MUST produce genuine cognitive engagement via specified protocols.
3. **Accountability requires specificity.** Every action traceable to a specific actor, authority, inputs, outputs, and rule version.
4. **Constraints are external to the agent.** The agent is outside the trust boundary. Guardrails are enforced by the WOS Processor using a deontic framework of Permissions, Prohibitions, Obligations, and Rights.
5. **Graceful degradation is mandatory.** Every workflow MUST function without any agent participation.
6. **Correctness is verifiable.** Workflow definitions are verifiable for soundness. SHACL shapes enforce policy-level governance.
7. **Linked by construction.** Documents are natively JSON-LD, queryable as RDF graphs without transformation.
8. **Composable by design.** Reusable ActivityDefinitions are independently versionable, shareable across programs and agencies.
9. **Safely editable by AI.** Typed patch operations enable LLMs to propose structural edits that are statically analyzable before commit.
10. **Incrementally adoptable.** A minimal kernel is surrounded by progressively adoptable profiles.

### 1.3 Scope

**Within scope:** the kernel-and-profiles architecture; the tripartite object model (ActivityDefinition, WorkflowDefinition, Task); lifecycle semantics based on hierarchical state machines; decision services with defeasible rules and temporal parameters; human task lifecycle with structured oversight; agent governance with deontic constraints, autonomy levels, confidence, fallback, and monitoring; object-centric case state and evidence; event envelope format and correlation; four-layer provenance model with PROV-AGENT alignment and tamper evidence; abstract durable execution guarantees with deterministic replay; typed patch operations for AI-assisted authoring; JSON-LD serialization with normative `@context`; SHACL shapes for guardrail and definition validation; due process requirements; conformance profiles and testing.

**Out of scope:** UI rendering; persistence mechanisms; transport protocols beyond contracts; process mining algorithms; ML training or inference; document management; notification delivery; general-purpose computation.

### 1.4 Relationship to Tier Specifications

This is the **Core Specification**. Tier Specifications elaborate individual layers and MUST NOT contradict this document.

Planned Tier Specifications: WOS-Lifecycle, WOS-Decision, WOS-Task, WOS-Agent, WOS-CaseState, WOS-Integration, WOS-Provenance, WOS-Execution, WOS-SHACL, WOS-Authoring, WOS-Conformance.

### 1.5 Notational Conventions

Key words per BCP 14 [RFC2119] [RFC8174].

---

## 2. Conformance

### 2.1 Conformance Classes

**WOS Document.** A serialized workflow definition, activity definition, or decision service definition conforming to this specification. MUST validate against the appropriate JSON Schema and include a valid `@context`.

**WOS Processor.** A software system consuming WOS Documents and producing behavior consistent with this specification. MUST support at least one Conformance Profile (§21).

### 2.2 Document Conformance

1. MUST be serialized in JSON-LD [JSON-LD11] or YAML resolving to valid JSON-LD.
2. MUST include `@context` referencing the WOS context document.
3. MUST validate against the applicable JSON Schema.
4. MUST satisfy static semantic constraints.
5. MUST define a fallback to human performance for every agent invocation.
6. SHOULD pass soundness verification (§6.8).
7. SHOULD pass SHACL structural governance validation (Appendix B).

### 2.3 Processor Conformance

1. MUST accept conformant documents and reject structurally invalid documents with diagnostics.
2. MUST preserve `@context` during round-trips.
3. MUST execute kernel layer semantics consistent with this specification.
4. MUST produce provenance records for every state transition, task operation, decision evaluation, and agent invocation.
5. MUST enforce agent governance including deontic constraints, autonomy limits, and fallback chains.
6. MUST use recorded outputs during replay rather than re-invoking non-deterministic services (§13.1 G3).

---

## 3. Terminology

This section is normative.

**ActivityDefinition.** A standalone, independently versionable, reusable template for a unit of work. Specifies what work entails (input/output schemas, assignment model, SLA, oversight protocol, separation of duties) without binding to any specific workflow. Published as its own JSON-LD document with its own URI.

**Actor.** An entity performing actions: `human`, `system`, or `agent`. See §14.

**Agent.** An AI system participating in a workflow. Outside the trust boundary. Subject to deontic constraints.

**Agent Session.** A bounded interaction between a workflow instance and an agent with checkpoints and intervention points.

**Autonomy Level.** `autonomous`, `supervisory`, `assistive`, or `manual`.

**Case.** A workflow instance applied to a specific subject.

**Case File.** The typed data container associated with a Case, modeled as a set of objects with temporal Event-to-Object relationships following the OCEL 2.0 paradigm.

**Compensation.** A semantically meaningful reversal of a completed activity.

**Confidence.** A structured certainty assessment: scalar value, derivation method, calibration status, optional per-field scores.

**Consequential Decision.** A determination with legal, material, or binding effects on individual rights, benefits, services, or obligations.

**Deontic Constraint.** A governance rule classified as a Permission, Prohibition, Obligation, or Right, adopted from OASIS LegalRuleML's formal vocabulary.

**Guard.** A boolean expression controlling transition eligibility.

**Guardrail.** A deontic constraint on agent behavior enforced by the WOS Processor. Classified as Permission (what the agent may do within bounds), Prohibition (what the agent must not do), Obligation (what the agent must do), or Right (what the agent is entitled to receive as input).

**History State.** Records last active substate for resumption after suspension.

**Kernel Layer.** A layer that every conformant implementation must understand: Lifecycle (1), Case State (5), Provenance (7), Execution (8).

**Milestone.** A declarative checkpoint achieved when a case data condition becomes true.

**Obligation.** A deontic constraint requiring the agent to perform a specified action or include specified content. Checked after output and before commit. Violation triggers enforcement action.

**Override.** A human action superseding an automated decision with mandatory structured rationale.

**Patch Operation.** A typed edit operation against a WOS document's abstract syntax tree, statically analyzable for validity before commit.

**Permission.** A deontic constraint bounding what the agent is allowed to produce. Agent outputs within permission bounds are accepted; outputs outside are violations.

**Profile Layer.** A layer adopted as needed: Decision (2), Task (3), Agent (4), Integration (6).

**Prohibition.** A deontic constraint forbidding specified agent outputs or actions regardless of confidence.

**Provenance Record.** An immutable audit entry structured in a four-layer architecture: Immutable Facts, Structured Reasoning, Generated Narrative, and Counterfactual.

**Right.** A deontic constraint specifying what the agent is entitled to receive from the WOS Processor as input context.

**Soundness.** Deadlock-freedom, livelock-freedom, proper termination, reachability, and fallback completeness.

**Structured Oversight.** Human review protocols producing genuine cognitive engagement: `independentFirst`, `considerOpposite`, `calibratedConfidence`, `dualBlind`, `unassisted`.

**Task.** A runtime instance of an ActivityDefinition, carrying the template plus workflow-specific overrides plus case context.

**WorkflowDefinition.** A process topology referencing ActivityDefinitions, Decision Services, and Agent Configurations by URI.

---

## 4. Architecture Overview

This section is normative.

### 4.1 Constraint-Enhanced Layered Kernel

WOS is organized as a minimal, stable kernel surrounded by independently adoptable profile layers. The kernel defines the minimum for interoperability and durable execution. Profiles add domain-specific capabilities progressively.

```
            ┌──────────────────────────────────────────┐
            │        Profile Layers (adoptable)        │
            │                                          │
            │  ┌──────────┐ ┌──────────┐ ┌──────────┐ │
            │  │ Decision │ │  Human   │ │  Agent   │ │
            │  │ & Policy │ │  Task    │ │Governance│ │
            │  │ Layer 2  │ │ Layer 3  │ │ Layer 4  │ │
            │  └──────────┘ └──────────┘ └──────────┘ │
            │                ┌──────────┐              │
            │                │Integrate │              │
            │                │ Layer 6  │              │
            │                └──────────┘              │
            ├──────────────────────────────────────────┤
            │          Kernel Layers (required)         │
            │                                          │
            │  ┌──────────┐ ┌──────────┐              │
            │  │Lifecycle │ │Case State│              │
            │  │ Layer 1  │ │ Layer 5  │              │
            │  └──────────┘ └──────────┘              │
            │  ┌──────────┐ ┌──────────┐              │
            │  │Provenance│ │ Durable  │              │
            │  │ Layer 7  │ │Execution │              │
            │  └──────────┘ │ Layer 8  │              │
            │               └──────────┘              │
            ├──────────────────────────────────────────┤
            │      JSON-LD / RDF Graph Foundation      │
            │      @context · SHACL · PROV-O           │
            └──────────────────────────────────────────┘
```

**Kernel layers** (1, 5, 7, 8) define how state progresses, what data accumulates, what gets recorded, and what guarantees hold. A workflow using only kernel layers is a statechart with typed case data, immutable audit, and crash recovery. Changes to kernel layers require major version bumps.

**Profile layers** (2, 3, 4, 6) are adopted as needed. An organization not using AI agents skips Layer 4. An organization handling all routing internally skips Layer 6. Profile layers may evolve more rapidly via minor version additions. Each profile is independently testable.

The JSON-LD/RDF foundation is not a layer but the substrate on which all layers are expressed.

### 4.2 Tripartite Object Model

WOS v4 separates three tiers of abstraction:

```
┌─────────────────────────────────────────────────────┐
│ ActivityDefinition                                   │
│ Standalone, reusable, independently versionable.     │
│ Published as its own JSON-LD document.               │
│ Shared across programs and agencies.                 │
│ Defines: what the work entails.                      │
├─────────────────────────────────────────────────────┤
│ WorkflowDefinition                                   │
│ Process topology composing ActivityDefinitions.      │
│ References activities, decisions, agents by URI.     │
│ Defines: when and how work is orchestrated.          │
├─────────────────────────────────────────────────────┤
│ Task (runtime)                                       │
│ Instance of an ActivityDefinition.                   │
│ Carries template + workflow overrides + case context.│
│ Defines: this specific unit of work, right now.      │
└─────────────────────────────────────────────────────┘
```

This separation enables an agency to publish a registry of standard ActivityDefinitions (eligibility review, environmental assessment, financial analysis) that multiple WorkflowDefinitions reference. Version updates to an ActivityDefinition propagate to all referencing workflows without touching any workflow topology. AI-assisted workflow authoring becomes compositional: an LLM composing a new grant workflow references existing activities from a registry rather than reinventing each task.

The same separation applies to Decision Services and Agent Configurations. All three are publishable as independent JSON-LD documents with stable URIs.

### 4.3 Separation Principles

**Process topology MUST be separated from decision logic.** The lifecycle defines structure; Decision Services evaluate conditions.

**Decision logic MUST be separated from task management.** Decisions determine what; tasks determine who and how.

**Agent governance MUST be separated from agent implementation.** Deontic constraints are properties of the workflow definition, enforced by the WOS Processor.

**Case data MUST be separated from process state.** Case File holds business data; lifecycle tracks progress.

**Audit MUST be separated from execution.** Provenance observes; it does not control.

**Execution guarantees MUST be separated from execution mechanisms.** The kernel defines what holds, not how.

**Syntax validation MUST be separated from semantic governance.** JSON Schema validates structure. SHACL validates policy.

**Reusable templates MUST be separated from their instantiation context.** ActivityDefinitions are independent of the WorkflowDefinitions that reference them.

### 4.4 Cross-Cutting Concerns

**Actor Model (§14).** Every action attributed to a typed actor. **Due Process (§16).** Consequential decisions subject to notice, explanation, appeal. **Expressions (§15).** WOS Expression Language throughout. **Identity.** Every object identified by URI; `id` maps to `@id`. **Versioning (§18).** Independent versions for all publishable artifacts.

---

## 5. Document Model and Serialization

This section is normative.

### 5.1 Document Types

WOS defines three publishable document types:

**WorkflowDefinition** — the primary document type. Contains the lifecycle topology and references to ActivityDefinitions, Decision Services, and Agent Configurations.

**ActivityDefinition** — a standalone work template. Contains form contracts, assignment models, SLA, oversight protocols, separation of duties. Referenced from WorkflowDefinitions by URI.

**DecisionServiceDefinition** — a standalone decision service. Contains inputs, outputs, logic, temporal parameters. Referenced from WorkflowDefinitions by URI. (Agent Configurations follow the same pattern.)

All three document types share the same serialization model, `@context`, and validation requirements.

### 5.2 WorkflowDefinition Structure

```yaml
"@context": "https://wos-spec.org/context/4.0.0"
"@type": "WorkflowDefinition"
wos: "4.0.0"
id: "urn:wos:example.gov:grant-review"
name: "Grant Application Review"
version: "1.0.0"
status: "active"

metadata:
  description: "..."
  authors: [...]
  created: "2026-04-09T00:00:00Z"
  modified: "2026-04-09T00:00:00Z"
  jurisdiction: "US-Federal"
  authority: "24 CFR Part 570"
  effectiveDate: "2026-04-15"
  impactLevel: "rights-impacting"

lifecycle: { ... }                   # REQUIRED — Kernel Layer 1

# Profile references: inline definitions or URIs to standalone documents
decisions:
  eligibility:
    $ref: "urn:wos:example.gov:decisions:eligibility:2.1.0"
  # or inline: { version: "2.1.0", inputs: [...], ... }

parameters: { ... }                  # Temporal parameters

activities:                          # References to ActivityDefinitions
  completenessCheck:
    $ref: "urn:wos:example.gov:activities:completeness-check:1.0.0"
    overrides:                       # Workflow-specific overrides
      sla:
        dueIn: "P2BD"               # Tighter SLA for this program

  eligibilityReview:
    $ref: "urn:wos:example.gov:activities:eligibility-review:1.0.0"
    overrides:
      assignment:
        potentialOwners:
          roles: ["cdbgEligibilitySpecialist"]

agents:
  eligibilityScreener:
    $ref: "urn:wos:example.gov:agents:eligibility-screener:2.0.0"
    overrides:
      autonomyPolicy:
        maxAutonomy: "assistive"

caseFile: { ... }                    # Kernel Layer 5
integrations: { ... }                # Profile Layer 6
provenance: { ... }                  # Kernel Layer 7 config
execution: { ... }                   # Kernel Layer 8 config
dueProcess: { ... }                  # §16
extensions: { ... }                  # §20
```

### 5.3 ActivityDefinition Structure

An ActivityDefinition is a standalone JSON-LD document:

```yaml
"@context": "https://wos-spec.org/context/4.0.0"
"@type": "ActivityDefinition"
id: "urn:wos:example.gov:activities:eligibility-review:1.0.0"
wos: "4.0.0"
name: "Eligibility Review"
version: "1.0.0"
status: "active"

description: "Review eligibility determination against program criteria."

form:
  inputSchema:
    type: "object"
    properties:
      applicationData: { $ref: "urn:wos:example.gov:schemas:application" }
      eligibilityResult: { $ref: "urn:wos:example.gov:schemas:eligibility-output" }
  outputSchema:
    type: "object"
    properties:
      decision: { type: "string", enum: ["eligible", "ineligible", "needsInfo"] }
      rationale: { type: "string", minLength: 50 }
      citedRegulation: { type: "string" }
    required: ["decision", "rationale", "citedRegulation"]

assignment:
  potentialOwners:
    roles: ["eligibilitySpecialist"]
    skills: ["programEligibility"]
  businessAdministrators:
    roles: ["programSupervisor"]

priority:
  expression: "if caseFile.application.expedited then 1 else 5"

sla:
  dueIn: "P5BD"
  warningAt: "P3BD"
  businessCalendar: "federalWorkdays"
  escalateOnBreach:
    to: { roles: ["programSupervisor"] }
    action: "reassign"

separation:
  excludeFrom: ["finalApproval"]
  constraint: "sameInstance"

oversight:
  agentAssistance:
    capability: "eligibilityPreScreen"
    timing: "beforeClaim"
    protocol: "independentFirst"
    presentation:
      showConfidence: true
      showAlternatives: true
      highlightLowConfidenceFields: true
      showDiffFromIndependent: true
```

When a WorkflowDefinition references this ActivityDefinition, it inherits all properties and may apply overrides for program-specific customization. The runtime Task instance merges the ActivityDefinition, the overrides, and the case-specific context.

### 5.4 JSON-LD Serialization

The canonical machine-interchange format is JSON-LD 1.1 [JSON-LD11]. A WOS Document is simultaneously valid JSON, valid JSON-LD, and an RDF graph. Implementations that do not use RDF tooling MAY ignore the `@context` without loss of structural functionality.

### 5.5 YAML Authoring Format

YAML 1.2 [YAML] is RECOMMENDED for human authoring. The `@context` MUST be present in the YAML source or injected during conversion. YAML features (anchors, aliases, comments) are not part of the semantic model.

### 5.6 The `@context` Document

Published at `https://wos-spec.org/context/4.0.0`. Maps WOS terms to IRIs from PROV-O, Schema.org, LegalRuleML (for deontic terms), BBO (for process ontology terms where aligned), and the WOS namespace for novel concepts. See Appendix A.

The `@context` maps `@type` values for the three document types:

| WOS `@type` | IRI |
|------------|-----|
| `WorkflowDefinition` | `wos:WorkflowDefinition` |
| `ActivityDefinition` | `wos:ActivityDefinition` |
| `DecisionServiceDefinition` | `wos:DecisionServiceDefinition` |
| `AgentConfiguration` | `wos:AgentConfiguration` |

### 5.7 Extending `@context` for Domain Vocabularies

WOS Documents MAY extend the `@context` with domain vocabularies (NIEM, FHIR, Schema.org) for cross-agency interoperability without middleware.

### 5.8 JSON Schema Validation

Structural conformance via JSON Schema at `https://wos-spec.org/schema/core/4.0.0`. Separate schemas for WorkflowDefinition, ActivityDefinition, and DecisionServiceDefinition.

### 5.9 SHACL Governance Validation

Policy-level validation via SHACL [SHACL] shapes (Appendix B). SHACL validates cross-cutting constraints that JSON Schema cannot express: impact-level-to-autonomy relationships, guardrail completeness, oversight protocol requirements, due process configuration presence. RECOMMENDED for all documents. REQUIRED for the Semantic Conformance Profile.

### 5.10 Property: `impactLevel`

| Value | Definition | Requirements |
|-------|-----------|-------------|
| `rights-impacting` | Decisions affect individual rights, benefits, services, obligations. | Full due process. Agent autonomy capped at `assistive` unless elevated. |
| `safety-impacting` | Decisions affect individual or public safety. | Full due process. Agent autonomy capped at `assistive`. |
| `operational` | Organizational operations without direct individual impact. | Due process RECOMMENDED. Autonomy up to `autonomous` with guardrails. |
| `informational` | Informational outputs, no binding decisions. | Due process OPTIONAL. No restrictions. |

Default: `operational`.

---

## 6. Kernel Layer 1: Lifecycle and Topology

This section is normative. This is a kernel layer.

### 6.1 Overview

The Lifecycle layer defines the statechart governing workflow progression, based on Harel statecharts [Harel1987] formalized in SCXML [SCXML]. Each state is a named RDF node; each transition is a directed relationship.

### 6.2 States

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `type` | enum | REQUIRED | `atomic`, `compound`, `parallel`, `final`. |
| `onEntry` | array of Action | OPTIONAL | Actions on entry. |
| `onExit` | array of Action | OPTIONAL | Actions on exit. |
| `transitions` | array of Transition | OPTIONAL | Outgoing transitions. |
| `historyState` | enum | OPTIONAL | `shallow`, `deep`. Compound only. |
| `initialState` | string | CONDITIONAL | Required for `compound`. |
| `regions` | map of Region | CONDITIONAL | Required for `parallel`. |

### 6.3 Transitions

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `event` | string | REQUIRED | Triggering event. |
| `target` | string | REQUIRED | Target state. |
| `guard` | expression | OPTIONAL | Must evaluate `true`. |
| `actions` | array of Action | OPTIONAL | Transition actions. |
| `priority` | integer | OPTIONAL | Lower = higher priority. Default: 0. |
| `description` | string | OPTIONAL | Human-readable. |

Resolution: collect matching, evaluate guards, fire unique lowest-priority survivor. Execution: onExit (innermost first), transition actions, onEntry (outermost first), provenance record.

### 6.4 Actions

| Action Type | Description | Key Properties |
|-------------|-------------|----------------|
| `createTask` | Creates a Task from an ActivityDefinition. | `activityRef` (URI to ActivityDefinition or inline key), `overrides` |
| `invokeDecision` | Invokes a decision service. | `decisionRef`, `outputBinding`, `autonomy` |
| `invokeAgent` | Invokes agent within governance envelope. | `agentRef`, `capability`, `autonomy` |
| `setData` | Sets a case file value. | `path`, `value` |
| `emitEvent` | Emits event to integration layer. | `eventType`, `data` |
| `startTimer` | Starts durable timer. | `timerId`, `duration`/`deadline`, `event` |
| `cancelTimer` | Cancels timer. | `timerId` |
| `compensate` | Triggers compensation. | `scope` |
| `log` | Informational audit entry. | `message`, `level` |
| `notify` | Sends notification. | `recipientRoles`, `template` |

Note: `createTask` uses `activityRef` (referencing an ActivityDefinition by URI) rather than `taskRef`. The runtime Task is instantiated from the ActivityDefinition plus any `overrides` declared in the action or the workflow's `activities` section.

### 6.5 Events

Internal: `task.completed`, `task.failed`, `task.escalated`, `timer.expired`, `regions.allFinal`, `error`, `milestone.achieved`, `decision.complete`, `agent.complete`, `agent.failed`, `guardrail.violated`. External: matched via correlation (§11).

### 6.6 Milestones

Declarative checkpoints. Re-evaluated on data change. Achievement raises `milestone.achieved`.

### 6.7 Soundness Verification

**Deadlock-freedom**, **livelock-freedom**, **proper termination**, **no dead elements**, **fallback completeness** (workflow remains sound when all agent invocations fail). Verifiers MUST support a decidable fragment. Inconclusive results are warnings.

---

## 7. Profile Layer 2: Decision and Policy

This section is normative. This is a profile layer.

### 7.1 Decision Services

Encapsulated logic for routing, eligibility, classification, policy evaluation. Independently versioned. May be published as standalone DecisionServiceDefinition documents or defined inline.

### 7.2 Decision Logic Types

**Decision Tables** with hit policies (`unique`, `first`, `priority`, `collect`, `collectSum/Min/Max/Count`). **Expression Logic**. **Decision Requirement Graphs** (DAG of sub-decisions). **Defeasible Rules** with override relationships forming a DAG. **External Decision Reference**.

### 7.3 Temporal Parameters

Values changing on specific dates. The value effective on the reference date is used. No effective value raises an error.

### 7.4 Decision Provenance

Every invocation produces a Decision Record as a PROV-AGENT-compatible graph node.

---

## 8. Profile Layer 3: Human Task Management

This section is normative. This is a profile layer.

### 8.1 Task Lifecycle

Tasks are runtime instances of ActivityDefinitions. The lifecycle:

```
Available → Claimed → InProgress → Completed
                                  → Failed
                         → Returned → Available
Any non-terminal → Escalated
Any non-terminal → Suspended → (prior state)
Any non-terminal → Cancelled
```

Terminal: `Completed`, `Failed`, `Cancelled`.

### 8.2 Task Operations

`create`, `claim`, `release`, `start`, `complete`, `fail`, `delegate`, `forward`, `returnForRework`, `escalate`, `suspend`, `resume`, `cancel`. Each produces a provenance record.

### 8.3 Assignment, SLA, Separation of Duties

Five role categories. Business-calendar-aware SLA with escalation. Separation of duties enforced at claim time. All as specified in §8.3–8.5 of prior versions.

### 8.4 Structured Oversight Protocols

REQUIRED when a task involves agent assistance.

| Protocol | Description |
|----------|-------------|
| `independentFirst` | Reviewer forms independent assessment before agent output is revealed. Interface enforces ordering. |
| `considerOpposite` | Reviewer articulates reasons the recommendation might be wrong before confirming. |
| `calibratedConfidence` | Calibrated confidence displayed. Per-field scores shown. Low-confidence fields highlighted. |
| `dualBlind` | Two independent reviewers without seeing each other's or agent's assessment. |
| `unassisted` | No agent assistance. |

Multiple protocols MAY be combined. `independentFirst` enforcement is REQUIRED by the WOS Processor.

### 8.5 Override Authority

Overrides require structured rationale, authority verification, and supporting evidence. Override Records capture original result, override values, and rationale.

---

## 9. Profile Layer 4: Agent Governance

This section is normative. This is a profile layer.

### 9.1 Overview

The governance envelope — deontic constraints, autonomy levels, confidence requirements, and fallback policies — is enforced by the WOS Processor. The agent is outside the trust boundary. The WOS Processor is the Policy Enforcement Point (PEP); the deontic constraint definitions are the Policy Decision Point (PDP).

### 9.2 Agent Configuration

Agent Configurations may be published as standalone documents or defined inline. They specify model identity, capabilities with formal contracts, default autonomy, deontic constraints, fallback chain, input preparation, and monitoring.

```yaml
"@type": "AgentConfiguration"
id: "urn:wos:example.gov:agents:eligibility-screener:2.0.0"
version: "2.0.0"
model:
  provider: "anthropic"
  identifier: "claude-sonnet-4-20250514"
  versionPolicy: "pinned"

capabilities:
  - id: "eligibilityPreScreen"
    decisionRef: "urn:wos:example.gov:decisions:eligibility:2.1.0"
    inputContract:
      required: ["applicantIncome", "householdSize", "applicationDate"]
      maxTokens: 8000
    outputContract:
      schema: { $ref: "urn:wos:example.gov:schemas:eligibility-output" }
      confidenceRequired: true
      obligationsRequired: ["citesRegulation"]
    preconditions:
      - "caseFile.application != null"
      - "caseFile.intake.isComplete = true"

defaultAutonomy: "assistive"
autonomyPolicy:
  maxAutonomy: "assistive"
  reason: "Eligibility determinations affect individual rights."
```

### 9.3 Capability Contracts

Each capability declares formal contracts verified before and after invocation:

**Input contract** — `required` fields, `maxTokens`, and `preconditions` (case data conditions that must hold). The WOS Processor verifies preconditions before invocation and rejects if unsatisfied.

**Output contract** — `schema` for structural validation, `confidenceRequired` flag, and `obligationsRequired` listing which Obligation guardrails must be satisfiable. The WOS Processor validates output against the contract before guardrail evaluation.

### 9.4 Autonomy Levels

| Level | Semantics |
|-------|-----------|
| `autonomous` | Output committed without human review. REQUIRES deontic constraints. PROHIBITED for `rights-impacting`/`safety-impacting` unless elevated. |
| `supervisory` | Provisionally committed. Human reviews within `reviewWindow`. |
| `assistive` | Recommendation only. Human reviews, may modify, explicitly confirms. |
| `manual` | Human performs. Agent assists on demand only. |

### 9.5 Deontic Constraint Framework

All agent constraints are classified using LegalRuleML's deontic vocabulary:

#### 9.5.1 Permissions

What the agent is allowed to do, within bounds. Subsumes output constraints (permitted value ranges) and scope constraints (permitted output fields).

```yaml
deonticConstraints:
  permissions:
    - id: "outputRange"
      field: "eligible"
      bounds: "value = true or value = false"
      onViolation: "reject"

    - id: "outputScope"
      allowedFields: ["eligible", "reason", "applicableThreshold", "confidence"]
      onViolation: "reject"
```

#### 9.5.2 Prohibitions

What the agent must not do, regardless of confidence.

```yaml
  prohibitions:
    - id: "noInconsistentApproval"
      condition: "output.eligible = true and caseFile.application.areaMedianIncome > parameters.amiThreshold(caseFile.application.submittedDate) * 1.5"
      reason: "Eligible with income far above threshold requires human review."
      onViolation: "escalateToHuman"

    - id: "noFinalDenial"
      condition: "output.eligible = false and instance.impactLevel = 'rights-impacting'"
      reason: "Agent may not render final denial in rights-impacting workflow."
      onViolation: "escalateToHuman"
```

#### 9.5.3 Obligations

What the agent must do. Checked after output. Violation triggers enforcement.

```yaml
  obligations:
    - id: "citesRegulation"
      requirement: "contains(output.reason, 'CFR') or contains(output.reason, 'USC')"
      reason: "Determination must cite regulatory authority."
      onViolation: "reject"

    - id: "flagsOldDocuments"
      requirement: "all doc in caseFile.documents satisfies (doc.receivedDate >= today() - duration('P12M')) or contains(output.notes, 'stale document')"
      reason: "Agent must flag documents older than 12 months."
      onViolation: "flag"

    - id: "perFieldConfidence"
      requirement: "output.confidence.fieldLevel != null"
      reason: "Per-field confidence required for this capability."
      onViolation: "reject"
```

#### 9.5.4 Rights

What the agent is entitled to receive as input.

```yaml
  rights:
    - id: "receivesApplicationData"
      entitlement: "caseFile.application"
      description: "Agent has a right to receive application data as specified in inputPreparation.includeFields."

    - id: "receivesParameterValues"
      entitlement: "parameters.amiThreshold"
      description: "Agent has a right to receive current parameter values for referenced temporal parameters."
```

Rights formalize the input-side contract: the WOS Processor has an Obligation to provide the data specified in the agent's Rights. A Rights violation (the processor fails to provide entitled data) MUST NOT be attributed to the agent and MUST trigger a system-level error rather than an agent fallback.

#### 9.5.5 Enforcement

Deontic constraints are evaluated after the agent produces output and before commit. Evaluation order: (1) Permissions (structural bounds). (2) Prohibitions (forbidden patterns). (3) Obligations (required elements). (4) Confidence floor (minimum certainty). (5) Volume constraints. (6) Human review sampling.

Violation actions: `reject` > `escalateToHuman` > `switchToAssistive` > `flag`. Most restrictive applies.

Composition: constraints at workflow, agent, and action levels compose by union.

SHACL equivalence: every deontic constraint has an equivalent SHACL shape. The WOS-SHACL Tier Specification defines the bidirectional mapping. SHACL processors can validate agent outputs as RDF graphs against the constraint shapes directly.

### 9.6 Confidence Framework

ConfidenceReport: `overall` (0.0–1.0), `method` (`modelNative`, `calibrated`, `heuristic`, `conformal`, `declared`), `calibrationStatus` (`calibrated`, `uncalibrated`, `expired`), optional `explanation`, optional `fieldLevel`. Calibration required and validated empirically. Expired calibration caps autonomy at `assistive`. Cumulative confidence tracked in multi-step sessions.

### 9.7 Fallback Chains

Ordered degradation. MUST terminate in `escalateToHuman` or `fail`. MUST NOT cycle. Validated at load. Each attempt produces provenance.

### 9.8 Input Preparation and Isolation

`sanitize`, `maxInputTokens`, `redactFields`/`includeFields`, `isolateUntrustedData` (CaMeL dual-LLM architecture).

### 9.9 Monitoring and Drift Detection

Agent states: `active`, `degraded`, `suspended`, `retired`. Drift detection via `psi`, `ks`, `chi2`, `accuracy`. Shadow deployment RECOMMENDED for model changes in `rights-impacting` workflows.

### 9.10 Multi-Step Sessions

Sessions with checkpoints, intervention points, cumulative confidence tracking. Intervention options: approve, modify, redirect, terminate, restart.

### 9.11 Tool Use Governance

Permitted/prohibited tool registries. No direct case file writes. All invocations recorded. Side-effecting tools at autonomous level require explicit policy. Cascading autonomy requires declaration with bounded depth.

---

## 10. Kernel Layer 5: Case State and Evidence

This section is normative. This is a kernel layer.

### 10.1 Object-Centric Case Model

The Case File is modeled as a set of typed objects with temporal Event-to-Object (E2O) relationships, following the OCEL 2.0 paradigm. Rather than a flat case payload, the kernel tracks discrete objects and records timestamped mutations. This natively handles complex scenarios where a single event (a multi-party hearing, a joint determination) mutates the state of multiple objects simultaneously.

Each Case File Item declares a `vocabulary` property for domain alignment via `@context` extension.

### 10.2 Data Mutation Semantics

Every mutation recorded as an immutable E2O provenance event: object ID, attribute path, prior value, new value, actor, triggering context, timestamp. A WOS Processor MUST reconstruct any object's state at any prior point.

### 10.3 Evidence Management

Claim check pattern: content hash (SHA-256+), content type, claim check URI. Evidence MUST NOT be stored inline. Trust labeling distinguishes between verified evidence (authenticated, integrity-checked), untrusted evidence (citizen submissions, external documents), and agent-generated content (outputs, drafts, summaries). Trust labels are immutable metadata on evidence references.

### 10.4 Selective Visibility

Role-based field-level access: `read`, `readWrite`, `none`.

---

## 11. Profile Layer 6: Integration and Eventing

This section is normative. This is a profile layer.

### 11.1 Integration Types

`request-response` (OpenAPI), `event-emit`, `event-consume` (with correlation), `callback`. All external invocations wrapped in idempotent envelopes with idempotency keys and bounded retries.

### 11.2 Event Envelope

CloudEvents 1.0 with WOS extensions: `wosinstanceid`, `wosdefid`, `wosdefversion`, `wosstate`, `wostaskid`, `woscorrelationkey`, `woscausationeventid`.

### 11.3 Idempotency and Correlation

Event consumption idempotent. Correlation via attribute-to-case-file-path mapping.

### 11.4 Capability Advertisement

Schema.org `potentialAction` for workflow capability discovery. The `actionStatus` four-state model is a derived view, not a replacement for WOS lifecycle semantics.

### 11.5 Protocol Alignment

**MCP** for agent-tool integration within governance envelope. **A2A** for inter-agent communication with `input-required` state for human-in-the-loop. **SOM/AWP** acknowledged as emerging; extension points defined. No protocol mandated.

---

## 12. Kernel Layer 7: Provenance and Audit

This section is normative. This is a kernel layer.

### 12.1 Four-Layer Audit Architecture

| Layer | Name | Content | Authority |
|-------|------|---------|-----------|
| 1 | Immutable Facts | Timestamp, actor, model version, inputs, outputs, policy version, confidence, reviewer ID. | **Authoritative.** |
| 2 | Structured Reasoning | Rules applied, evidence consulted, criteria checked, decision table trace. | **Authoritative** for deterministic logic. Descriptive for agent reasoning. |
| 3 | Generated Narrative | Model's natural language explanation. | **Informational only.** Labeled non-authoritative. |
| 4 | Counterfactual | What would change the outcome. Positive and negative evidence. | Informational. **Required** for adverse decisions in `rights-impacting` workflows. |

Layer 3 MUST NOT be treated as dispositive evidence.

### 12.2 PROV-AGENT Alignment

Provenance records are JSON-LD aligned with PROV-O and PROV-AGENT. Agent invocations, tool use, and MCP interactions are first-class provenance graph nodes. WOS extends PROV-AGENT on structured oversight outcomes, due process records, and the four-layer separation.

### 12.3 Object-Centric Event Logging

Provenance records use OCEL 2.0's Event-to-Object (E2O) and Object-to-Object (O2O) mapping. Events that mutate multiple objects produce a single event record with multiple E2O links, not duplicated event records. Attribute mutations tracked per-object over time.

### 12.4 Record Types

`transition`, `decision`, `agentInvocation`, `agentCheckpoint`, `agentToolUse`, `taskOperation`, `dataMutation`, `override`, `guardrailViolation`, `guardrailBypass`, `autonomyChange`, `modelVersionChange`, `driftAlert`, `dueProcessNotice`, `appealFiled`, `patchApplied`.

### 12.5 Tamper Evidence

Merkle tree hash-chaining with SHA-256, signed tree heads, inclusion and consistency proofs. REQUIRED for Full conformance.

### 12.6 Process Mining Interoperability

Primary: OCEL 2.0 event emission. Secondary: IEEE XES for legacy tooling.

### 12.7 Provenance Export Packaging

RO-Crate [RO-Crate] with Workflow Run Crate profile. Self-describing JSON-LD archive mapping steps to Schema.org types.

---

## 13. Kernel Layer 8: Durable Execution Contract

This section is normative. This is a kernel layer.

### 13.1 Durability Guarantees

**G1: Crash Recovery.** Non-terminal instances resume from last durable state.

**G2: Persistent State.** Lifecycle state, case file objects, task states, timer registrations durably persisted.

**G3: Deterministic Replay.** Every action invoking a non-deterministic external service (including AI agent invocations) MUST persist the output as an immutable step result before advancing workflow state. During crash recovery, workflow resumption, or audit replay, the WOS Processor MUST use the persisted output from the first successful execution rather than re-invoking the external service. Re-invocation of a non-deterministic service during replay is a conformance violation.

**G4: Durable Timers.** Survive restarts, fire within tolerance, consume no resources while waiting.

**G5: External Signal Delivery.** Signals to inactive instances durably enqueued.

### 13.2 Retry Policy

`maxAttempts`, `backoff` (`fixed`, `linear`, `exponential`), `initialInterval`, `maxInterval`, `multiplier`, `nonRetryableErrors`. All external invocations carry idempotency keys.

### 13.3 Timeout Categories

`stepTimeout`, `taskTimeout`, `instanceTimeout`, `heartbeatTimeout`, `queueTimeout`.

### 13.4 Compensation

Activities with side effects SHOULD register compensation handlers. Reverse completion order.

---

## 14. Actor Model

This section is normative.

| Type | Description | Provenance |
|------|-------------|-----------|
| `human` | Person performing tasks. | Identity, role, timestamp. |
| `system` | Deterministic component. | Component ID, version, timestamp. |
| `agent` | AI system, non-deterministic, carries confidence. Outside trust boundary. | Model ID, version, confidence, input summary, all PROV-AGENT fields. |

Agents MUST NOT override human decisions. Actor type based on decision authority. Cascading autonomous agents require declaration with bounded depth.

---

## 15. Expression Language

This section is normative.

Profile of FEEL (DMN). Literals, paths, arithmetic, comparison, boolean, conditionals, membership, string/list/date operations, context construction, null-safe evaluation. Pure functions, bounded termination. Context: `caseFile`, `event`, `task`, `instance`, `parameters`, `agent`, `env`, `output` (in guardrail expressions).

---

## 16. Due Process Requirements

This section is normative for `rights-impacting` and `safety-impacting` workflows.

### 16.1 Notice

Adverse decisions require notice before effect: specific determination, factual basis with individualized reason codes, appeal rights and deadline, agent disclosure.

### 16.2 Explanation Levels

`individualized` (REQUIRED for `rights-impacting`), `categorical`, `aggregate`. Counterfactuals required: positive (what would change the outcome) and negative (what did NOT affect it, including protected characteristics).

### 16.3 Appeal Mechanisms

Human adjudicator independent of original determination. Agents assist preparation, MUST NOT decide appeals. Continuation of services during appeal.

### 16.4 Agent Disclosure

`discloseThatAgentAssisted: true` REQUIRED for `rights-impacting`.

### 16.5 Continuation-of-Service States

When `continuationOfServices` is true, the workflow MUST include a topological pattern that freezes adverse impacts and maintains current service levels during the appeal window. This is a structural workflow requirement, not merely a policy preference. The SHACL governance shapes (Appendix B) validate that rights-impacting workflows with appeal mechanisms include continuation-of-service topology.

---

## 17. AI-Native Authoring and Patch Operations

This section is normative.

### 17.1 Overview

WOS is designed for AI-assisted authoring. LLMs should propose structural modifications to workflow definitions as typed patch operations against the document's abstract syntax tree, not as complete regenerations or text diffs. Each operation is statically analyzable for syntax validity (JSON Schema), semantic governance (SHACL), and soundness (§6.7) before commit.

### 17.2 Patch Operation Structure

```yaml
"@context": "https://wos-spec.org/context/4.0.0"
"@type": "PatchSet"
id: "urn:wos:example.gov:patches:add-env-review:1"
targetDocument: "urn:wos:example.gov:grant-review"
targetVersion: "1.0.0"
author:
  type: "agent"
  id: "urn:agent:workflow-designer-v2"
description: "Add environmental review stage to grant workflow."

operations:
  - op: "insertState"
    path: "lifecycle.states"
    id: "environmentalReview"
    value:
      type: "atomic"
      onEntry:
        - action: "createTask"
          activityRef: "urn:wos:example.gov:activities:environmental-review:1.0.0"

  - op: "addTransition"
    path: "lifecycle.states.technicalReview.transitions"
    value:
      event: "task.completed"
      target: "environmentalReview"
      guard: "caseFile.application.requiresNepa = true"

  - op: "addTransition"
    path: "lifecycle.states.environmentalReview.transitions"
    value:
      event: "task.completed"
      target: "adjudication"

  - op: "addActivityRef"
    path: "activities"
    id: "environmentalReview"
    value:
      $ref: "urn:wos:example.gov:activities:environmental-review:1.0.0"

  - op: "addDeonticConstraint"
    path: "agents.eligibilityScreener.deonticConstraints.obligations"
    value:
      id: "flagsNepaRequirement"
      requirement: "if caseFile.application.proposedActivities contains 'construction' then contains(output.notes, 'NEPA')"
      reason: "Agent must flag NEPA requirement for construction activities."
      onViolation: "flag"
```

### 17.3 Operation Types

| Operation | Description | Preconditions |
|-----------|-------------|--------------|
| `insertState` | Add a state to the lifecycle. | ID must not collide. Parent must exist. |
| `removeState` | Remove a state. | State must have no incoming transitions from other states. |
| `modifyState` | Change state properties. | State must exist. |
| `addTransition` | Add a transition to a state. | Source state must exist. Target must exist or be created in the same PatchSet. |
| `removeTransition` | Remove a transition. | Transition must exist. |
| `modifyTransition` | Change transition properties. | Transition must exist. |
| `addActivityRef` | Add an ActivityDefinition reference. | URI must resolve. |
| `removeActivityRef` | Remove an activity reference. | No actions in the lifecycle may reference it. |
| `addDecisionRef` | Add a Decision Service reference. | URI must resolve. |
| `addAgentRef` | Add an Agent Configuration reference. | URI must resolve. |
| `addDeonticConstraint` | Add a Permission, Prohibition, Obligation, or Right. | Target agent must exist. |
| `removeDeonticConstraint` | Remove a constraint by ID. | Constraint must exist. |
| `modifyDeonticConstraint` | Change a constraint. | Constraint must exist. |
| `setParameter` | Set or modify a temporal parameter. | Parameter name must be valid. |
| `addCaseFileItem` | Add a case file item. | ID must not collide. |
| `modifyOverride` | Change a workflow-specific override on an activity reference. | Activity reference must exist. |

### 17.4 Validation Pipeline

A PatchSet MUST be validated before application:

1. **Structural validation.** Each operation's preconditions are checked. The resulting document (after applying all operations) MUST validate against the JSON Schema.
2. **SHACL governance validation.** The resulting document MUST pass all applicable SHACL governance shapes.
3. **Soundness verification.** The resulting lifecycle MUST pass soundness verification (§6.7), including fallback completeness.
4. **Provenance recording.** If the PatchSet passes validation and is committed, a `patchApplied` provenance record is produced capturing the author, the operations, the validation results, and the resulting document version.

A PatchSet that fails any validation step MUST be rejected with diagnostics identifying each failure. The original document MUST NOT be modified.

### 17.5 Compositional Authoring

Because ActivityDefinitions, Decision Services, and Agent Configurations are independently publishable, AI-assisted authoring becomes compositional. An LLM generating a new workflow can:

1. Query an ActivityDefinition registry (SPARQL over the linked data graph) for existing reusable activities.
2. Compose a WorkflowDefinition that references existing activities by URI.
3. Propose only the topology (states, transitions, guards) and workflow-specific overrides as a PatchSet against a template or blank workflow.
4. Have each patch operation validated individually against the resulting graph.

This dramatically reduces the surface area for hallucination compared to generating a complete workflow definition from scratch.

---

## 18. Versioning and Evolution

This section is normative.

### 18.1 Versioning Model

All publishable documents (WorkflowDefinitions, ActivityDefinitions, DecisionServiceDefinitions, AgentConfigurations) use Semantic Versioning independently. An ActivityDefinition version change does not require a WorkflowDefinition version change unless the workflow's overrides are affected.

### 18.2 Instance Migration

Default: pinned execution (instances continue under creation version). Optional: forward migration with safety verification and provenance.

### 18.3 Schema Evolution

Backward-compatible: add optional properties, widen constraints, add enum values, add items. Breaking: remove properties, add required without defaults, narrow constraints, rename, type change. Breaking = major version increment.

### 18.4 `@context` Versioning

The `@context` is versioned with the spec. Breaking `@context` changes = spec major version increment.

### 18.5 ActivityDefinition Reference Resolution

When a WorkflowDefinition references an ActivityDefinition by URI, the reference MAY include a version constraint:

```yaml
activities:
  environmentalReview:
    $ref: "urn:wos:example.gov:activities:environmental-review:1.0.0"  # pinned
    # or: $ref: "urn:wos:example.gov:activities:environmental-review"  # latest active
    # or: $ref: "urn:wos:example.gov:activities:environmental-review:^1.0.0"  # semver range
```

Pinned references use the exact version. Unpinned references resolve to the latest `active` version at instance creation time, recorded immutably in the instance metadata.

---

## 19. Security and Access Control

This section is normative.

Roles: `workflowAdministrator`, `instanceInitiator`, `caseParticipant`, `taskWorker`, `taskAdministrator`, `auditor`. Authorization enforced at: instance creation, task operations, case file access, decision invocation, provenance access, override execution, agent configuration changes, guardrail bypass, patch commit. Failures logged.

---

## 20. Extensibility

This section is normative.

Namespaced extension properties (`x-agency:classification`). MUST NOT alter core semantics. MUST be preserved during round-trips. MUST NOT use `wos:` prefix. MAY add `@context` entries for domain vocabulary integration and protocol bindings.

---

## 21. Conformance Profiles

This section is normative.

### 21.1 Profile: Structural

Parse, validate (JSON Schema), round-trip, preserve `@context` and extensions. Enables editors, validators, linters, migration tools.

### 21.2 Profile: Kernel

Structural + execute kernel layer semantics (Lifecycle, Case State, Provenance, Execution). Produce transition records and data mutation records. Support all state types, milestones. Satisfy durability guarantees including deterministic replay (G3).

### 21.3 Profile: Task Management

Kernel + full task lifecycle, all operations, separation of duties, SLA timers, structured oversight protocols, ActivityDefinition resolution and override merging.

### 21.4 Profile: Decision

Kernel + decision tables with all hit policies, WOS Expression Language, temporal parameters, decision records.

### 21.5 Profile: Agent Governance

Kernel + Decision + enforce deontic constraints (Permissions, Prohibitions, Obligations, Rights), produce all agent provenance types (PROV-AGENT-aligned), enforce autonomy levels, enforce fallback chains, validate capability contracts (pre/post), confidence-based routing, multi-step sessions with checkpoints.

### 21.6 Profile: Full

All of Kernel + Task + Decision + Agent + integration semantics with correlation, all provenance types, tamper evidence, access control, due process for `rights-impacting`, OCEL 2.0 E2O event logging.

### 21.7 Profile: Verification

Structural + static soundness analysis (deadlock-freedom, livelock-freedom, proper termination, dead elements, fallback completeness), PatchSet validation pipeline, diagnostic reports. SHOULD support simulation.

### 21.8 Profile: Semantic

Structural + valid JSON-LD with `@context`, SHACL governance validation (Appendix B), SHACL-based deontic constraint enforcement, PROV-O/PROV-AGENT provenance graphs, SPARQL querying, OCEL 2.0 event emission. Enables portfolio-wide governance queries, cross-agency interoperability, and formal constraint validation.

### 21.9 Profile: Authoring

Structural + Verification + accept and validate PatchSets (§17), execute the four-stage validation pipeline, produce `patchApplied` provenance records, support compositional authoring via ActivityDefinition registry queries.

---

## 22. Privacy Considerations

This section is informative.

`visibility` restricts access. `redactFields` limits agent exposure. Trust labeling separates verified evidence from untrusted inputs. Provenance retention and anonymization. Claim check pattern. Right-to-erasure versus audit integrity via redaction with structural preservation.

---

## 23. Security Considerations

This section is informative.

1. **Expression sandboxing.** No side effects.
2. **Event authentication.** Signature verification.
3. **Provenance integrity.** Independent signed tree heads.
4. **Encryption.** At rest and in transit.
5. **Separation of duties.** Application-layer enforcement.
6. **Agent impersonation prevention.** Same authentication rigor as humans.
7. **Prompt injection defense.** `isolateUntrustedData` for CaMeL pattern. Deontic constraints as structural defense. Defense in depth.
8. **Model version drift.** Shadow deployment before production.
9. **Cascading autonomy.** Declared and bounded.
10. **Tool use.** Least-privilege. Recorded. Side effects require policy.
11. **`@context` integrity.** HTTPS, caching, hash verification.
12. **Patch security.** PatchSets from untrusted sources (including AI agents) MUST pass the full validation pipeline before commit. Patch authorship recorded in provenance. Patches that would weaken deontic constraints or remove due process configurations require elevated authorization.

---

## 24. References

### 24.1 Normative References

**[RFC2119]**, **[RFC3339]**, **[RFC3986]**, **[RFC8174]**, **[RFC8259]** — as in prior versions.

**[YAML]** YAML 1.2.

**[JSON-LD11]** W3C Recommendation, July 2020.

**[RDF11]** W3C Recommendation, February 2014.

**[SHACL]** W3C Recommendation, July 2017.

**[PROV-O]** W3C Recommendation, April 2013.

**[PROV-DM]** W3C Recommendation, April 2013.

**[SemVer]** Semantic Versioning 2.0.0.

**[ISO8601]** ISO 8601:2019.

**[CloudEvents]** CNCF, Version 1.0.2, 2022.

**[TraceContext]** W3C Recommendation, February 2020.

**[DMN]** OMG, Version 1.4, 2021.

**[OpenAPI]** Version 3.1.0, 2021.

**[AsyncAPI]** Version 3.0, 2023.

**[JSONSchema]** IETF, draft-bhutton-json-schema-01, 2022.

**[LegalRuleML]** OASIS, "LegalRuleML Core Specification Version 1.0", 2021.

### 24.2 Informative References

**[Harel1987]** Harel, D., "Statecharts", SCP 8(3), 1987.

**[SCXML]** W3C Recommendation, 2015.

**[BPMN]** OMG, ISO/IEC 19510:2013.

**[CMMN]** OMG, Version 1.1, 2016.

**[WS-HumanTask]** OASIS, Version 1.1, 2012.

**[XACML]** OASIS, Version 3.0, 2013.

**[Sagas]** Garcia-Molina and Salem, ACM SIGMOD, 1987.

**[WorkflowPatterns]** van der Aalst et al., DPD 14(1), 2003.

**[OCEL2]** van der Aalst et al., OCEL 2.0, 2023.

**[XES]** IEEE Std 1849-2016.

**[RO-Crate]** Soiland-Reyes et al., Data Science 5(2), 2022.

**[WfRunCrate]** Leo et al., PLOS ONE, 2024.

**[PROV-AGENT]** PROV-O extension for agentic workflows.

**[RFC9162]** Certificate Transparency 2.0.

**[NIST-SP-800-53]** SP 800-53 Rev 5.

**[OMB-M-24-10]** M-24-10, March 2024.

**[EU-AI-Act]** Regulation 2024/1689.

**[NIST-AI-RMF]** AI 100-1, 2023.

**[Vaccaro2024]** Nature Human Behaviour 8, 2287–2297.

**[Turpin2023]** NeurIPS 2023.

**[Lanham2023]** Anthropic, 2023.

**[Chen2023]** ChatGPT behavioral drift study.

**[Buçinca2021]** CSCW 2021.

**[Nasr2025]** "The Attacker Moves Second", 2025.

**[CaMeL]** Google DeepMind, 2025.

**[FormalLLM]** EMNLP 2024.

**[ABC]** Agent Behavioral Contracts, 2026.

**[Feng2025]** Knight First Amendment Institute, 2025.

**[Wachter2018]** HJLT 31(2), 2018.

**[MCP]** Model Context Protocol, 2024.

**[A2A]** Agent-to-Agent Protocol, Linux Foundation, 2025.

**[BBO]** BPMN Based Ontology.

**[FHIR-Workflow]** HL7, "FHIR Workflow Module", R5.

**[OASF]** Open Agent Schema Framework.

**[CanadaDirective]** Treasury Board of Canada, "Directive on Automated Decision-Making", 2019 (amended 2024).

---

## Appendix A. JSON-LD Context Document

Normative. Published at `https://wos-spec.org/context/4.0.0`. Extends v3 context with:

```json
{
  "@context": {
    "@version": 1.1,
    "wos": "https://wos-spec.org/ns/",
    "schema": "https://schema.org/",
    "prov": "http://www.w3.org/ns/prov#",
    "sh": "http://www.w3.org/ns/shacl#",
    "xsd": "http://www.w3.org/2001/XMLSchema#",
    "dcterms": "http://purl.org/dc/terms/",
    "lrml": "http://docs.oasis-open.org/legalruleml/ns/v1.0/",

    "id": "@id",
    "type": "@type",

    "WorkflowDefinition": "wos:WorkflowDefinition",
    "ActivityDefinition": "wos:ActivityDefinition",
    "DecisionServiceDefinition": "wos:DecisionServiceDefinition",
    "AgentConfiguration": "wos:AgentConfiguration",
    "PatchSet": "wos:PatchSet",

    "name": "schema:name",
    "description": "schema:description",
    "version": "schema:version",
    "created": { "@id": "schema:dateCreated", "@type": "xsd:dateTime" },
    "modified": { "@id": "schema:dateModified", "@type": "xsd:dateTime" },
    "status": "wos:status",
    "impactLevel": "wos:impactLevel",

    "lifecycle": "wos:lifecycle",
    "initialState": "wos:initialState",
    "states": { "@id": "wos:states", "@container": "@index" },
    "transitions": { "@id": "wos:transitions", "@container": "@list" },
    "regions": { "@id": "wos:regions", "@container": "@index" },
    "guard": "wos:guard",
    "event": "wos:triggerEvent",
    "target": { "@id": "wos:targetState", "@type": "@id" },
    "onEntry": { "@id": "wos:onEntry", "@container": "@list" },
    "onExit": { "@id": "wos:onExit", "@container": "@list" },
    "historyState": "wos:historyState",
    "milestones": { "@id": "wos:milestones", "@container": "@index" },

    "activities": { "@id": "wos:activities", "@container": "@index" },
    "activityRef": { "@id": "wos:activityRef", "@type": "@id" },
    "decisions": { "@id": "wos:decisions", "@container": "@index" },
    "agents": { "@id": "wos:agents", "@container": "@index" },
    "parameters": { "@id": "wos:parameters", "@container": "@index" },

    "form": "wos:form",
    "inputSchema": "wos:inputSchema",
    "outputSchema": "wos:outputSchema",
    "assignment": "wos:assignment",
    "potentialOwners": "wos:potentialOwners",
    "sla": "wos:sla",
    "separation": "wos:separation",
    "oversight": "wos:oversight",
    "protocol": "wos:oversightProtocol",

    "capabilities": { "@id": "wos:capabilities", "@container": "@list" },
    "inputContract": "wos:inputContract",
    "outputContract": "wos:outputContract",
    "preconditions": { "@id": "wos:preconditions", "@container": "@list" },

    "deonticConstraints": "wos:deonticConstraints",
    "permissions": { "@id": "wos:permissions", "@container": "@list" },
    "prohibitions": { "@id": "wos:prohibitions", "@container": "@list" },
    "obligations": { "@id": "wos:obligations", "@container": "@list" },
    "rights": { "@id": "wos:rights", "@container": "@list" },
    "Permission": "lrml:Permission",
    "Prohibition": "lrml:Prohibition",
    "Obligation": "lrml:Obligation",
    "Right": "lrml:Right",

    "autonomy": "wos:autonomyLevel",
    "confidence": "wos:confidence",
    "guardrails": "wos:guardrails",
    "fallback": "wos:fallback",
    "monitoring": "wos:monitoring",

    "caseFile": "wos:caseFile",
    "items": { "@id": "wos:items", "@container": "@index" },
    "vocabulary": { "@id": "wos:vocabulary", "@type": "@id" },

    "timestamp": { "@id": "prov:atTime", "@type": "xsd:dateTime" },
    "actor": { "@id": "prov:wasAssociatedWith", "@type": "@id" },
    "wasGeneratedBy": { "@id": "prov:wasGeneratedBy", "@type": "@id" },
    "wasDerivedFrom": { "@id": "prov:wasDerivedFrom", "@type": "@id" },
    "recordType": "wos:recordType",
    "auditLayer": "wos:auditLayer",
    "processId": { "@id": "wos:processId", "@type": "@id" },

    "operations": { "@id": "wos:operations", "@container": "@list" },
    "op": "wos:patchOp",
    "path": "wos:patchPath",
    "value": "wos:patchValue",
    "targetDocument": { "@id": "wos:targetDocument", "@type": "@id" },
    "targetVersion": "wos:targetVersion"
  }
}
```

---

## Appendix B. SHACL Shapes for Structural Governance

Normative. Extends v3 shapes with tripartite model and deontic constraints.

```turtle
@prefix wos: <https://wos-spec.org/ns/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix lrml: <http://docs.oasis-open.org/legalruleml/ns/v1.0/> .

# Rights-impacting workflows must have due process with appeals
wos:RightsImpactingDueProcessShape
  a sh:NodeShape ;
  sh:targetClass wos:WorkflowDefinition ;
  sh:sparql [
    sh:message "Rights-impacting workflows MUST include an enabled appeal mechanism." ;
    sh:select """
      SELECT $this WHERE {
        $this wos:impactLevel "rights-impacting" .
        FILTER NOT EXISTS {
          $this wos:dueProcess/wos:appealMechanism/wos:enabled true
        }
      }
    """ ;
  ] .

# Rights-impacting workflows with appeals must have continuation-of-service
wos:ContinuationOfServiceShape
  a sh:NodeShape ;
  sh:targetClass wos:WorkflowDefinition ;
  sh:sparql [
    sh:message "Rights-impacting workflows with appeals MUST include continuation-of-service topology." ;
    sh:select """
      SELECT $this WHERE {
        $this wos:impactLevel "rights-impacting" .
        $this wos:dueProcess/wos:appealMechanism/wos:enabled true .
        FILTER NOT EXISTS {
          $this wos:dueProcess/wos:appealMechanism/wos:continuationOfServices true
        }
      }
    """ ;
  ] .

# Autonomous agent actions must have deontic constraints
wos:AutonomousDeonticShape
  a sh:NodeShape ;
  sh:sparql [
    sh:message "Autonomous agent actions MUST have deontic constraints." ;
    sh:select """
      SELECT $this WHERE {
        $this wos:autonomyLevel "autonomous" .
        $this wos:agentRef ?agent .
        FILTER NOT EXISTS { ?agent wos:deonticConstraints ?dc }
      }
    """ ;
  ] .

# Agent invocations must have fallback to human
wos:AgentFallbackShape
  a sh:NodeShape ;
  sh:targetSubjectsOf wos:agentRef ;
  sh:sparql [
    sh:message "Every agent invocation MUST have a fallback terminating in human task." ;
    sh:select """
      SELECT $this WHERE {
        $this wos:agentRef ?agent .
        ?agent wos:fallback ?fb .
        FILTER NOT EXISTS {
          ?fb wos:terminal/wos:onFailure "escalateToHuman"
        }
      }
    """ ;
  ] .

# Tasks with agent assistance must specify oversight protocol
wos:OversightProtocolShape
  a sh:NodeShape ;
  sh:targetSubjectsOf wos:agentAssistance ;
  sh:property [
    sh:path ( wos:oversight wos:oversightProtocol ) ;
    sh:minCount 1 ;
    sh:message "Tasks with agent assistance MUST specify an oversight protocol." ;
  ] .

# Obligations on agent must match capability contract
wos:ObligationCapabilityAlignmentShape
  a sh:NodeShape ;
  sh:targetClass wos:AgentConfiguration ;
  sh:sparql [
    sh:message "Capabilities with obligationsRequired must have matching Obligation constraints." ;
    sh:select """
      SELECT $this ?cap ?req WHERE {
        $this wos:capabilities ?cap .
        ?cap wos:outputContract/wos:obligationsRequired ?req .
        FILTER NOT EXISTS {
          $this wos:deonticConstraints/wos:obligations ?ob .
          ?ob wos:id ?req .
        }
      }
    """ ;
  ] .

# ActivityDefinitions must be resolvable
wos:ActivityRefResolvableShape
  a sh:PropertyShape ;
  sh:path wos:activityRef ;
  sh:nodeKind sh:IRI ;
  sh:message "ActivityDefinition references must be valid IRIs." ;
  sh:severity sh:Violation .

# Patches must not weaken deontic constraints without elevated auth
wos:PatchDeonticWeakeningShape
  a sh:NodeShape ;
  sh:targetClass wos:PatchSet ;
  sh:sparql [
    sh:message "Patches removing deontic constraints require elevated authorization." ;
    sh:select """
      SELECT $this WHERE {
        $this wos:operations ?op .
        ?op wos:patchOp "removeDeonticConstraint" .
        FILTER NOT EXISTS {
          $this wos:author/wos:role "workflowAdministrator"
        }
      }
    """ ;
  ] .
```

---

## Appendix C. Patch Operation Reference

Normative. Complete catalog of typed patch operations.

| Operation | Description | Preconditions | Postconditions |
|-----------|-------------|--------------|----------------|
| `insertState` | Add state to lifecycle. | ID unique. Parent exists. | Document passes JSON Schema. |
| `removeState` | Remove state. | No incoming transitions from other states. No active instances in this state. | Lifecycle remains sound. |
| `modifyState` | Change state properties. | State exists. | Document passes JSON Schema. |
| `addTransition` | Add transition. | Source exists. Target exists or created in same PatchSet. | No ambiguous transition resolution. |
| `removeTransition` | Remove transition. | Transition exists. | Lifecycle remains sound. |
| `modifyTransition` | Change transition properties. | Transition exists. | Guard expression valid. |
| `addActivityRef` | Add ActivityDefinition reference. | URI resolvable. ID unique in `activities`. | ActivityDefinition compatible with workflow. |
| `removeActivityRef` | Remove activity reference. | No actions reference it. | — |
| `addDecisionRef` | Add Decision Service reference. | URI resolvable. | — |
| `addAgentRef` | Add Agent Configuration reference. | URI resolvable. | Fallback chain valid. |
| `addDeonticConstraint` | Add Permission/Prohibition/Obligation/Right. | Target agent exists. Constraint ID unique. | Constraint expression valid. |
| `removeDeonticConstraint` | Remove constraint by ID. | Constraint exists. Author has elevated auth. | — |
| `modifyDeonticConstraint` | Change constraint. | Constraint exists. | Expression valid. |
| `setParameter` | Set/modify temporal parameter. | Parameter name valid. | Value type matches. |
| `addCaseFileItem` | Add case file item. | ID unique. | Schema valid. |
| `modifyOverride` | Change workflow override on activity. | Activity ref exists. | Override compatible with ActivityDefinition. |
| `setImpactLevel` | Change workflow impact level. | — | If escalating to rights-impacting, due process config must exist. |
| `addMilestone` | Add milestone. | ID unique. | Condition expression valid. |

---

## Appendix D. Relationship to Existing Standards

| Standard | Relationship |
|----------|-------------|
| **JSON-LD 1.1 / RDF 1.1** | Native serialization. Every document is an RDF graph. |
| **SHACL** | Governance validation for definitions and deontic constraint enforcement. |
| **PROV-O / PROV-AGENT** | Provenance alignment. Four-layer audit as PROV extension. |
| **LegalRuleML** | Deontic vocabulary (Permission, Prohibition, Obligation, Right) for agent governance. `@context` maps to LegalRuleML IRIs. |
| **OCEL 2.0** | Object-centric case model and process mining interoperability. E2O event logging. |
| **RO-Crate** | Audit archive packaging. Workflow Run Crate profile. |
| **HL7 FHIR Workflow** | Inspiration for tripartite object model (PlanDefinition → ActivityDefinition → Task). |
| **Schema.org** | Case data mapping. Capability advertisement via `potentialAction`. |
| **BBO / sBPMN** | Process ontology terms in `@context` for lifecycle concepts. |
| **BPMN / CMMN / DMN / SCXML** | Conceptual heritage. Statechart semantics, decision tables, case milestones. |
| **WS-HumanTask** | Task lifecycle and role model, with structured oversight additions. |
| **CloudEvents** | Event envelope with WOS extensions. |
| **MCP / A2A** | Extension points for agent-tool and agent-agent interoperability. |
| **OMB M-24-10 / EU AI Act / Canada Directive** | Due process, transparency, oversight requirements operationalized as workflow constraints. |
| **NIST AI RMF** | Layers map to GOVERN, MAP, MEASURE, MANAGE. |
| **Workflow Patterns** | Completeness benchmark for lifecycle and task constructs. |

---

## Appendix E. Changelog

| Date | Version | Description |
|------|---------|-------------|
| 2026-04-09 | 4.0.0 | Tripartite object model (ActivityDefinition / WorkflowDefinition / Task). Deontic constraint framework (Permission / Prohibition / Obligation / Right) from LegalRuleML. Typed patch operations for AI authoring. Kernel-versus-profiles architectural narrative. Deterministic replay guarantee (G3). Capability contracts with pre/postconditions. Object-centric case model with E2O relationships. Continuation-of-service topology requirement. Trust labeling for evidence. Authoring Conformance Profile. |
| 2026-04-09 | 3.0.0 | JSON-LD native serialization. SHACL governance. PROV-AGENT provenance. OCEL 2.0. RO-Crate. Semantic Profile. |
| 2026-04-08 | 2.0.0 | Eight-layer architecture. Four-layer audit. Structured oversight. Due process. |
