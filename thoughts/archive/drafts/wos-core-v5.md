# Workflow Orchestration Standard (WOS) Core Specification

## W3C First Public Working Draft

**Latest published version:**
: https://wos-spec.org/TR/wos-core/

**Editor's Draft:**
: https://wos-spec.org/ed/wos-core/

**Editors:**
: Mike (TealWolf Consulting LLC)

**Version:**
: 5.0.0

**Date:**
: 9 April 2026

**Status:**
: First Public Working Draft

---

## Abstract

This specification defines the Workflow Orchestration Standard (WOS), a declarative, machine-readable language for describing high-stakes, long-running workflows in which humans and AI agents collaborate on consequential decisions.

WOS is organized as a **Constraint-Enhanced Layered Kernel**: a minimal, stable core focused on durable state transitions, object-centric case modeling, and cryptographic provenance, surrounded by independently adoptable profiles for decision logic, human task management, agent governance, and integration.

WOS v5 introduces a foundational architectural decision: **Formspec is the universal interface contract specification for WOS.** Everywhere WOS needs a typed, validated, version-pinned data contract — human task forms, agent input/output contracts, decision service interfaces, integration request/response schemas — Formspec Definitions replace inline JSON Schema fragments. This works because Formspec's presentation layer is entirely optional by design. A Formspec Definition with only Items, Binds, and Shapes is a pure data contract: typed fields, reactive cross-field constraints, structured validation results at three severity levels, version pinning, and extension points. No labels, no widgets, no rendering required. The result is a single contract specification, a single validation mechanism, a single provenance recording pattern, and a single expression language across all interface contexts.

The standard introduces five structural innovations:

**A tripartite object model** separating ActivityDefinitions (reusable work templates), WorkflowDefinitions (process topologies), and Tasks (runtime instances).

**A deontic governance framework** classifying agent constraints as Permissions, Prohibitions, Obligations, or Rights — adopted from OASIS LegalRuleML.

**A typed patch operation vocabulary** for AI-assisted authoring, enabling LLMs to propose structural edits as statically analyzable operations.

**A unified expression language** — the Formspec Expression Language (FEL) with Core, Decision, and Extended conformance profiles — providing a single, PEG-parseable, deterministic language for guard conditions, data transformations, deontic constraints, contract validation, and data flow mappings.

**An Assist Governance Interface** ensuring that WOS's deontic constraint framework governs AI agent behavior during form-level interactions with the same rigor as workflow-level interactions.

WOS documents are natively serialized as JSON-LD. Guardrails may be expressed as SHACL shapes. Provenance records align with W3C PROV-O and PROV-AGENT. Case data links to domain vocabularies through JSON-LD context extension.

The standard treats human authority as supreme, AI participation as governed, and audit as foundational. It is informed by empirical research demonstrating that naive human-in-the-loop designs degrade decision quality, that model-generated explanations are unreliable audit evidence, that behavioral drift between model versions can be catastrophic, and that no single defense prevents prompt injection.

---

## Status of This Document

This document is a First Public Working Draft. It has not been endorsed by any standards body. Comments may be submitted as issues at the specification's repository.

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Conformance](#2-conformance)
3. [Terminology](#3-terminology)
4. [Architecture Overview](#4-architecture-overview)
5. [Document Model, Serialization, and Interface Contracts](#5-document-model-serialization-and-interface-contracts)
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
- [C. FEL Conformance Profiles and Grammar Additions](#appendix-c-fel-conformance-profiles-and-grammar-additions)
- [D. FEEL-to-FEL Translation Table](#appendix-d-feel-to-fel-translation-table)
- [E. Assist Governance Interface](#appendix-e-assist-governance-interface)
- [F. Patch Operation Reference](#appendix-f-patch-operation-reference)
- [G. Relationship to Existing Standards](#appendix-g-relationship-to-existing-standards)
- [H. Changelog](#appendix-h-changelog)

---

## 1. Introduction

### 1.1 Background

High-stakes workflows — grants processing, benefits adjudication, licensing, inspections, investigations, compliance review — share requirements that no existing standard adequately integrates. They are long-running, human-centric, evidence-driven, heavily regulated, and increasingly involve AI agents.

This specification is informed by three bodies of empirical evidence. First, a meta-analysis of 106 experiments demonstrates that naive human-AI combinations degrade decision quality compared to either humans or AI alone (Vaccaro et al., Nature Human Behaviour, 2024), necessitating structured oversight protocols. Second, research on chain-of-thought faithfulness demonstrates that model-generated explanations are systematically post-hoc rationalizations (Turpin et al., NeurIPS 2023; Lanham et al., Anthropic, 2023), necessitating a four-layer audit architecture separating facts from narrative. Third, documented government AI failures — Michigan's MiDAS, Arkansas's RUGs, the Dutch childcare benefits scandal, Australia's Robodebt — establish that due process protections are non-negotiable for rights-impacting workflows.

### 1.2 Design Goals

1. **Human authority is supreme.** Agent recommendations MUST NOT be the sole factor in adverse decisions affecting individual rights.
2. **Structured oversight, not checkbox review.** Human oversight MUST produce genuine cognitive engagement.
3. **Accountability requires specificity.** Every action traceable to a specific actor, authority, inputs, outputs, and rule version.
4. **Constraints are external to the agent.** The agent is outside the trust boundary. Guardrails are enforced by the WOS Processor using a deontic framework.
5. **Graceful degradation is mandatory.** Every workflow MUST function without any agent participation.
6. **Correctness is verifiable.** SHACL shapes enforce policy-level governance on top of JSON Schema syntax validation.
7. **Linked by construction.** Documents are natively JSON-LD, queryable as RDF graphs.
8. **Composable by design.** Reusable ActivityDefinitions are independently versionable.
9. **Safely editable by AI.** Typed patch operations enable statically analyzable structural edits.
10. **One contract specification.** Formspec Definitions serve as the universal interface contract across all structured data exchanges — human tasks, agent capabilities, decision services, and integrations.
11. **One expression language.** FEL is the single expression language for guard conditions, data transformations, deontic constraints, contract validation, and data flow mappings.
12. **Incrementally adoptable.** A minimal kernel is surrounded by progressively adoptable profiles.

### 1.3 Scope

**Within scope:** the kernel-and-profiles architecture; the tripartite object model; lifecycle semantics; decision services with defeasible rules; human task lifecycle with structured oversight; agent governance with deontic constraints; object-centric case state; event envelope format; four-layer provenance model; durable execution guarantees; typed patch operations; FEL expression language with three conformance profiles; Formspec integration as universal interface contract; Mapping DSL for data flow between case files and Formspec instances; Assist Governance Interface for form-level agent governance; Arazzo integration sequences; CWL-informed non-HTTP tool descriptors; Formspec provenance ingestion; JSON-LD serialization with normative `@context`; SHACL shapes; due process requirements; conformance profiles.

**Out of scope:** UI rendering; persistence mechanisms; transport protocols beyond contracts; process mining algorithms; ML training or inference; document management; notification delivery; general-purpose computation; form rendering, layout, or widget selection (governed by Formspec's Presentation layer, which WOS does not consume).

### 1.4 Relationship to Formspec

This specification is a **companion framework** to Formspec v1.0. The two specifications share a clean architectural boundary: Formspec governs the data-collection instrument (what data is collected, how it behaves reactively, how it is validated), and WOS governs the orchestration envelope (who does the work, when, under what authority, with what agent assistance, with what oversight, and what gets recorded in the audit trail).

The integration rests on Formspec's strict three-layer separation of Structure, Behavior, and Presentation. Because the Presentation layer is entirely optional, a Formspec Definition with only Items, Binds, and Shapes — no labels, no widgets, no layout — is a valid, conformant definition that functions as a pure data contract. WOS uses this "headless contract" pattern for agent capability contracts, decision service interfaces, and integration response validation, while using the full Formspec stack (including Presentation, References, Ontology, and Assist layers) for human task forms.

Both specifications share the Formspec Expression Language (FEL) as their single expression language, the `x-` extension model with identical semantics, the three-severity validation model (error/warning/info), the version pinning and immutability rules, and the status lifecycle (draft → active → retired).

### 1.5 Relationship to Tier Specifications

This is the **Core Specification**. Tier Specifications elaborate individual layers. Planned: WOS-Lifecycle, WOS-Decision, WOS-Task (defines Formspec integration with `formRef`/`inputMapping`/`outputBinding`), WOS-Agent, WOS-CaseState, WOS-Integration (Arazzo and CWL tool integration), WOS-Provenance (Formspec ValidationReport and Ledger ingestion), WOS-Execution, WOS-SHACL, WOS-Authoring, WOS-Conformance.

### 1.6 Notational Conventions

Key words per BCP 14 [RFC2119] [RFC8174].

---

## 2. Conformance

### 2.1 Conformance Classes

**WOS Document.** A serialized workflow definition, activity definition, decision service definition, or interface contract conforming to this specification.

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
4. MUST produce provenance records for every state transition, task operation, decision evaluation, agent invocation, and contract validation.
5. MUST enforce agent governance including deontic constraints, autonomy limits, and fallback chains.
6. MUST use recorded outputs during replay rather than re-invoking non-deterministic services (§13.1 G3).
7. MUST implement FEL Core (§15). SHOULD implement FEL Decision when the Decision profile is claimed.
8. MUST support Formspec Definition references (`definitionRef`) as interface contracts. MAY additionally support inline JSON Schema contracts for backward compatibility.

---

## 3. Terminology

This section is normative.

**ActivityDefinition.** A standalone, independently versionable, reusable template for a unit of work. Published as its own JSON-LD document with its own URI.

**Actor.** An entity performing actions: `human`, `system`, or `agent`.

**Agent.** An AI system participating in a workflow. Outside the trust boundary.

**Assist Governance Interface.** The normative contract specifying how a WOS Processor wraps Formspec Assist tool invocations with deontic constraint enforcement, ensuring that agent behavior within a form is governed by the same Permissions, Prohibitions, and Obligations as agent behavior within the workflow.

**Autonomy Level.** `autonomous`, `supervisory`, `assistive`, or `manual`.

**Case File.** The typed data container associated with a Case, modeled as objects with temporal Event-to-Object relationships following OCEL 2.0.

**Consequential Decision.** A determination with legal, material, or binding effects on individual rights.

**Deontic Constraint.** A governance rule classified as a Permission, Prohibition, Obligation, or Right, adopted from OASIS LegalRuleML.

**Formspec Definition.** A JSON document conforming to Formspec v1.0 that specifies a data-collection instrument or data contract via Items (structure), Binds (behavior), and Shapes (validation). Used in WOS as the universal interface contract specification.

**Headless Contract.** A Formspec Definition used purely as a data contract with no presentation layer. Omits labels, widgets, and all visual properties. Used for agent capability contracts, decision service interfaces, and integration response validation.

**Guard.** A boolean FEL expression controlling transition eligibility.

**Guardrail.** A deontic constraint on agent behavior classified as Permission, Prohibition, Obligation, or Right.

**Kernel Layer.** Required by every conformant implementation: Lifecycle (1), Case State (5), Provenance (7), Execution (8).

**Mapping DSL Document.** A Formspec companion document defining bidirectional data transformations between WOS case file data and Formspec instances, using FEL for computed transforms.

**Obligation.** A deontic constraint requiring the agent to perform a specified action or include specified content.

**Override.** A human action superseding an automated decision with mandatory structured rationale.

**Patch Operation.** A typed edit against a WOS document's abstract syntax tree.

**Permission.** A deontic constraint bounding what the agent is allowed to produce.

**Profile Layer.** Adopted as needed: Decision (2), Task (3), Agent (4), Integration (6).

**Prohibition.** A deontic constraint forbidding specified agent outputs or actions.

**Provenance Record.** An immutable audit entry structured in a four-layer architecture.

**Right.** A deontic constraint specifying what the agent is entitled to receive as input.

**Structured Oversight.** Human review protocols producing genuine cognitive engagement: `independentFirst`, `considerOpposite`, `calibratedConfidence`, `dualBlind`, `unassisted`.

**Task.** A runtime instance of an ActivityDefinition, carrying the template plus workflow-specific overrides plus case context.

**WorkflowDefinition.** A process topology referencing ActivityDefinitions, Decision Services, and Agent Configurations by URI.

---

## 4. Architecture Overview

This section is normative.

### 4.1 Constraint-Enhanced Layered Kernel

WOS is organized as a minimal, stable kernel surrounded by independently adoptable profile layers.

```
            ┌──────────────────────────────────────────┐
            │        Profile Layers (adoptable)        │
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
            │  Formspec Universal Interface Contracts   │
            │  FEL Expression Language                  │
            ├──────────────────────────────────────────┤
            │      JSON-LD / RDF Graph Foundation      │
            │      @context · SHACL · PROV-O           │
            └──────────────────────────────────────────┘
```

**Kernel layers** (1, 5, 7, 8) define how state progresses, what data accumulates, what gets recorded, and what guarantees hold. Changes require major version bumps.

**Profile layers** (2, 3, 4, 6) are adopted as needed. May evolve via minor versions.

**Formspec and FEL** are the interface contract and expression language substrates shared by all layers.

**JSON-LD/RDF** is the serialization and linked-data foundation.

### 4.2 Tripartite Object Model

```
┌─────────────────────────────────────────────────────┐
│ ActivityDefinition                                   │
│ Standalone, reusable, independently versionable.     │
│ References Formspec Definitions for data contracts.  │
│ Defines: what the work entails.                      │
├─────────────────────────────────────────────────────┤
│ WorkflowDefinition                                   │
│ Process topology composing ActivityDefinitions.      │
│ Defines: when and how work is orchestrated.          │
├─────────────────────────────────────────────────────┤
│ Task (runtime)                                       │
│ Instance of an ActivityDefinition.                   │
│ Carries template + workflow overrides + case context.│
└─────────────────────────────────────────────────────┘
```

### 4.3 Universal Interface Contract Model

WOS uses Formspec Definitions as interface contracts in four contexts. Each context uses the same Definition structure but enables different optional Formspec layers.

**Human task contracts** — the full Formspec stack. The Definition specifies the form structure, reactive behavior, and validation. Presentation hints, Theme, Component, References, Ontology, Assist, and Respondent Ledger are optionally available. Mapping DSL documents formalize data flow between WOS case files and Formspec instances.

**Agent capability contracts** — headless Formspec Definitions specifying what the agent receives as input and what it must produce as output. The Behavior layer (Binds and Shapes) specifies constraints the WOS Processor enforces on agent output before deontic constraint evaluation. References sidecars optionally provide agent-consumable grounding material. Ontology sidecars optionally provide semantic field identity. No Presentation, Theme, Component, or Assist layers.

**Decision service contracts** — headless Formspec Definitions specifying inputs and outputs for decision services. The WOS Processor can independently verify that service outputs satisfy declared constraints, including cross-field Shapes.

**Integration contracts** — headless Formspec Definitions specifying request and response schemas. Integration responses are validated against their contract before results are committed to the case file.

The headless contract pattern works because Formspec's `label` and all Presentation layer properties are advisory. A Definition with only Items, Binds, and Shapes is valid and provides typed fields, computed values via `calculate` Binds, conditional requiredness via `required` Binds, cross-field validation via Shapes, and structured ValidationResults with severity, path, message, code, and constraintKind — capabilities that inline JSON Schema cannot offer.

### 4.4 Separation Principles

**Process topology MUST be separated from decision logic.**
**Decision logic MUST be separated from task management.**
**Agent governance MUST be separated from agent implementation.**
**Case data MUST be separated from process state.**
**Audit MUST be separated from execution.**
**Execution guarantees MUST be separated from execution mechanisms.**
**Syntax validation MUST be separated from semantic governance.**
**Reusable templates MUST be separated from their instantiation context.**
**Interface contracts MUST be separated from the systems that produce and consume them.** A Formspec Definition used as a contract is independent of the agent, decision service, or integration that satisfies it.

### 4.5 Cross-Cutting Concerns

**Actor Model (§14).** Every action attributed to a typed actor. **Due Process (§16).** Consequential decisions subject to notice, explanation, appeal. **Expressions (§15).** FEL throughout. **Identity.** Every object identified by URI; `id` maps to `@id`. **Versioning (§18).** Independent versions for all publishable artifacts.

---

## 5. Document Model, Serialization, and Interface Contracts

This section is normative.

### 5.1 Document Types

WOS defines four publishable document types. All share the same JSON-LD serialization, `@context`, and validation requirements.

**WorkflowDefinition** — process topology referencing ActivityDefinitions, Decision Services, and Agent Configurations.

**ActivityDefinition** — standalone work template. References Formspec Definitions for data contracts.

**DecisionServiceDefinition** — standalone decision service. References Formspec Definitions for input/output contracts.

**AgentConfiguration** — standalone agent configuration. References Formspec Definitions for capability input/output contracts.

### 5.2 WorkflowDefinition Structure

```yaml
"@context": "https://wos-spec.org/context/5.0.0"
"@type": "WorkflowDefinition"
wos: "5.0.0"
id: "urn:wos:example.gov:grant-review"
name: "Grant Application Review"
version: "1.0.0"
status: "active"

metadata:
  description: "..."
  jurisdiction: "US-Federal"
  authority: "24 CFR Part 570"
  impactLevel: "rights-impacting"

lifecycle: { ... }                   # REQUIRED — Kernel Layer 1

decisions:
  eligibility:
    $ref: "urn:wos:example.gov:decisions:eligibility:2.1.0"

parameters: { ... }

activities:
  completenessCheck:
    $ref: "urn:wos:example.gov:activities:completeness-check:1.0.0"
    overrides:
      sla:
        dueIn: "P2BD"

agents:
  eligibilityScreener:
    $ref: "urn:wos:example.gov:agents:eligibility-screener:3.0.0"

caseFile: { ... }                    # Kernel Layer 5
integrations: { ... }                # Profile Layer 6
provenance: { ... }                  # Kernel Layer 7 config
execution: { ... }                   # Kernel Layer 8 config
dueProcess: { ... }                  # §16
extensions: { ... }                  # §20
```

### 5.3 ActivityDefinition Structure

```yaml
"@context": "https://wos-spec.org/context/5.0.0"
"@type": "ActivityDefinition"
id: "urn:wos:example.gov:activities:eligibility-review:1.0.0"
wos: "5.0.0"
name: "Eligibility Review"
version: "1.0.0"
status: "active"

form:
  # Formspec Definition reference (RECOMMENDED for human tasks)
  formRef: "https://example.gov/forms/eligibility-review|2.1.0"

  inputMapping:
    # Mapping DSL rules connecting WOS case file to Formspec instances
    rules:
      - sourcePath: "caseFile.application"
        targetPath: "$primary"
        transform: "preserve"
      - sourcePath: "caseFile.priorDetermination"
        targetPath: "@instance('priorDetermination')"
        transform: "preserve"

  outputBinding:
    rules:
      - sourcePath: "$primary.decision"
        targetPath: "caseFile.reviews.eligibility.decision"
        transform: "preserve"
      - sourcePath: "$primary.rationale"
        targetPath: "caseFile.reviews.eligibility.rationale"
        transform: "preserve"

  # Inline fallback for simple cases (backward compatibility)
  # formInline:
  #   inputSchema: { ... }
  #   outputSchema: { ... }

assignment:
  potentialOwners:
    roles: ["eligibilitySpecialist"]

sla:
  dueIn: "P5BD"
  businessCalendar: "federalWorkdays"

separation:
  excludeFrom: ["finalApproval"]

oversight:
  agentAssistance:
    capability: "eligibilityPreScreen"
    protocol: "independentFirst"
```

When `formRef` is used, the WOS Processor resolves the Formspec Definition, executes the `inputMapping` via the Mapping DSL, presents the form, and on completion executes the `outputBinding` to write Response data back to the case file. Every mapping operation produces a provenance record. The Formspec ValidationReport at completion is ingested per §12.6.

### 5.4 JSON-LD Serialization

The canonical machine-interchange format is JSON-LD 1.1. Every WOS Document is simultaneously valid JSON, valid JSON-LD, and an RDF graph. Implementations ignoring `@context` lose no structural functionality.

### 5.5 YAML Authoring Format

YAML 1.2 RECOMMENDED for human authoring. `@context` MUST be present or injected during conversion.

### 5.6 The `@context` Document

Published at `https://wos-spec.org/context/5.0.0`. Maps WOS terms to IRIs from PROV-O, Schema.org, LegalRuleML, and the WOS namespace. The `@context` composes with Formspec's context document via JSON-LD context arrays, enabling the combined WOS + Formspec graph to be natively queryable.

### 5.7 Domain Vocabulary Extension

WOS Documents MAY extend the `@context` with domain vocabularies (NIEM, FHIR, Schema.org) for cross-agency interoperability. Formspec Ontology Documents provide the semantic bridge between WOS case file vocabularies and Formspec field concepts.

### 5.8 JSON Schema Validation

Structural conformance via JSON Schema. Separate schemas for each document type.

### 5.9 SHACL Governance Validation

Policy-level validation via SHACL shapes (Appendix B). SHACL validates cross-cutting constraints: impact-level-to-autonomy relationships, guardrail completeness, oversight protocol requirements, contract reference resolution, and due process configuration presence.

### 5.10 Property: `impactLevel`

| Value | Definition | Requirements |
|-------|-----------|-------------|
| `rights-impacting` | Decisions affect individual rights, benefits, services, obligations. | Full due process. Agent autonomy capped at `assistive` unless elevated. |
| `safety-impacting` | Decisions affect individual or public safety. | Full due process. Agent autonomy capped at `assistive`. |
| `operational` | Organizational operations without direct individual impact. | Due process RECOMMENDED. |
| `informational` | Informational outputs, no binding decisions. | Due process OPTIONAL. |

Default: `operational`.

---

## 6. Kernel Layer 1: Lifecycle and Topology

This section is normative. This is a kernel layer.

### 6.1 Overview

The Lifecycle layer defines the statechart governing workflow progression, based on Harel statecharts formalized in SCXML. Guard conditions, milestone conditions, and computed case file values use FEL expressions evaluated via the processing model (§6.9).

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
| `guard` | FEL expression | OPTIONAL | Must evaluate `true` to fire. |
| `actions` | array of Action | OPTIONAL | Transition actions. |
| `priority` | integer | OPTIONAL | Lower = higher priority. Default: 0. |

Resolution: collect matching, evaluate guards via FEL, fire unique lowest-priority survivor. Execution: onExit (innermost first), transition actions, onEntry (outermost first), provenance record.

### 6.4 Actions

| Action Type | Description | Key Properties |
|-------------|-------------|----------------|
| `createTask` | Creates a Task from an ActivityDefinition. | `activityRef`, `overrides` |
| `invokeDecision` | Invokes a decision service. | `decisionRef`, `outputBinding`, `autonomy` |
| `invokeAgent` | Invokes agent within governance envelope. | `agentRef`, `capability`, `autonomy` |
| `invokeIntegration` | Invokes an integration (including Arazzo sequences and tools). | `integrationRef`, `outputBinding` |
| `setData` | Sets a case file value. | `path`, `value` |
| `emitEvent` | Emits event to integration layer. | `eventType`, `data` |
| `startTimer` | Starts durable timer. | `timerId`, `duration`/`deadline`, `event` |
| `cancelTimer` | Cancels timer. | `timerId` |
| `compensate` | Triggers compensation. | `scope` |
| `log` | Informational audit entry. | `message`, `level` |
| `notify` | Sends notification. | `recipientRoles`, `template` |

### 6.5 Events

Internal: `task.completed`, `task.failed`, `task.escalated`, `timer.expired`, `regions.allFinal`, `error`, `milestone.achieved`, `decision.complete`, `agent.complete`, `agent.failed`, `guardrail.violated`, `contract.validated`, `integration.complete`. External: matched via correlation (§11).

### 6.6 Milestones

Declarative checkpoints. Re-evaluated via the processing model (§6.9) when case data changes. Achievement raises `milestone.achieved`.

### 6.7 Soundness Verification

**Deadlock-freedom**, **livelock-freedom**, **proper termination**, **no dead elements**, **fallback completeness** (workflow remains sound when all agent invocations fail).

### 6.8 Processing Model for Guard and Milestone Evaluation

WOS v5 defines a normative processing model for how case file data changes propagate through guard conditions, milestone expressions, and computed case file values. This model is adapted from Formspec's four-phase Rebuild → Recalculate → Revalidate → Notify cycle, applied to workflow topology rather than form behavior. The same algorithmic foundation — DAG dependency tracking with topological evaluation and minimal recalculation — governs both.

**Phase 1: Rebuild.** When the case file's structural shape changes (a repeatable section is added or removed, or the workflow definition is migrated), the Processor reconstructs the dependency graph for all guard expressions, milestone conditions, and computed case file values. The dependency graph is a directed acyclic graph (DAG) where each node is a FEL expression and each edge represents a field reference. Cycles are definition errors.

**Phase 2: Recalculate.** When case file data changes (via task completion, agent output, external event, or contract validation result), the Processor identifies the affected subgraph — all expressions transitively dependent on the changed fields — topologically sorts it, and re-evaluates only affected expressions. Expressions outside the affected set MUST NOT be re-evaluated. This is the minimal recalculation guarantee.

**Phase 3: Re-evaluate.** Transition guards whose dependencies were in the affected subgraph are re-assessed. If a previously unsatisfied guard is now satisfied and its triggering event is pending, the transition fires. Milestones whose conditions were affected are re-assessed. Achievement raises `milestone.achieved`.

**Phase 4: Notify.** The Processor signals state changes to observers: which transitions became eligible, which milestones were achieved, which guards changed state.

---

## 7. Profile Layer 2: Decision and Policy

This section is normative. This is a profile layer.

### 7.1 Decision Services

Encapsulated logic for routing, eligibility, classification, policy evaluation. Independently versioned. May be published as standalone documents or defined inline.

Decision service input and output contracts are specified as Formspec Definition references:

```yaml
decisions:
  eligibilityDetermination:
    version: "2.1.0"
    inputContract:
      definitionRef: "urn:wos:example.gov:contracts:eligibility-input:1.0.0"
    outputContract:
      definitionRef: "urn:wos:example.gov:contracts:eligibility-decision-output:1.0.0"
    logic:
      type: "decisionTable"
      hitPolicy: "first"
      rules: [...]
    effectiveDate: "2026-01-01"
    sunsetDate: "2026-12-31"
```

When a decision service produces output, the WOS Processor validates the output against the `outputContract` Formspec Definition, running the headless contract validation cycle (populate once, validate once). The resulting Formspec ValidationReport is ingested as a provenance record (§12.6). If the output has error-severity validation results, the decision output is rejected and the fallback chain is activated.

### 7.2 Decision Logic Types

**Decision Tables** with hit policies (`unique`, `first`, `priority`, `collect`, `collectSum/Min/Max/Count`). **Expression Logic** using FEL. **Decision Requirement Graphs.** **Defeasible Rules** with override relationships. **External Decision Reference.**

### 7.3 Temporal Parameters

Values changing on specific dates. Effective value on reference date; no value raises error.

---

## 8. Profile Layer 3: Human Task Management

This section is normative. This is a profile layer.

### 8.1 Task Lifecycle

Tasks are runtime instances of ActivityDefinitions. Terminal: `Completed`, `Failed`, `Cancelled`.

### 8.2 Task Operations

`create`, `claim`, `release`, `start`, `complete`, `fail`, `delegate`, `forward`, `returnForRework`, `escalate`, `suspend`, `resume`, `cancel`. Each produces a provenance record.

### 8.3 Formspec-Driven Task Creation

When a Task is created from an ActivityDefinition with `formRef`:

1. The Processor resolves the Formspec Definition at the specified `url|version`.
2. The Processor executes the `inputMapping` via the Mapping DSL, reading from the WOS case file and producing Formspec instances.
3. The Processor loads associated Formspec sidecars: References (for contextual help), Ontology (for semantic field identity), Registry (for extension compatibility). Theme and Component sidecars are NOT loaded by the WOS Processor.
4. The form is presented to the assigned worker. The Formspec processing model governs reactive behavior during data collection.
5. If the ActivityDefinition specifies agent assistance, the Assist Governance Interface (§9.11) governs agent tool invocations.
6. When the worker completes the form (zero error-severity ValidationResults), the Processor executes the `outputBinding`, writing Response data to the case file.
7. The Formspec ValidationReport is ingested as a provenance record (§12.6). If a Respondent Ledger exists, material events are ingested at lifecycle boundaries.

### 8.4 Assignment, SLA, Separation of Duties

Five role categories. Business-calendar-aware SLA with escalation. Separation of duties enforced at claim time.

### 8.5 Structured Oversight Protocols

REQUIRED when a task involves agent assistance.

| Protocol | Description |
|----------|-------------|
| `independentFirst` | Reviewer forms independent assessment before agent output is revealed. Interface enforces ordering. |
| `considerOpposite` | Reviewer articulates reasons the recommendation might be wrong before confirming. |
| `calibratedConfidence` | Calibrated confidence displayed. Per-field scores shown. Low-confidence fields highlighted. |
| `dualBlind` | Two independent reviewers without seeing each other's or agent's assessment. |
| `unassisted` | No agent assistance. |

When `independentFirst` is specified and the ActivityDefinition uses `formRef` with agent assistance, the Assist Governance Interface suppresses agent-generated `FieldHelp.summary` content in `formspec.field.help` responses until the reviewer's independent assessment is recorded.

### 8.6 Override Authority

Overrides require structured rationale, authority verification, and supporting evidence.

---

## 9. Profile Layer 4: Agent Governance

This section is normative. This is a profile layer.

### 9.1 Overview

The governance envelope — deontic constraints, autonomy levels, confidence requirements, and fallback policies — is enforced by the WOS Processor. The agent is outside the trust boundary.

### 9.2 Agent Configuration with Formspec Capability Contracts

Agent Configurations specify capabilities with Formspec Definition references for input and output contracts:

```yaml
"@type": "AgentConfiguration"
id: "urn:wos:example.gov:agents:eligibility-screener:3.0.0"
version: "3.0.0"

model:
  provider: "anthropic"
  identifier: "claude-sonnet-4-20250514"
  versionPolicy: "pinned"

capabilities:
  - id: "eligibilityPreScreen"
    decisionRef: "urn:wos:example.gov:decisions:eligibility:2.1.0"

    inputContract:
      definitionRef: "urn:wos:example.gov:contracts:eligibility-input:1.0.0"
      mapping:
        rules:
          - sourcePath: "caseFile.application.income"
            targetPath: "applicantIncome"
            transform: "preserve"
          - sourcePath: "caseFile.application.householdSize"
            targetPath: "householdSize"
            transform: "preserve"

    outputContract:
      definitionRef: "urn:wos:example.gov:contracts:eligibility-output:1.0.0"
      # Formspec Shapes in this Definition enforce:
      # - "eligible" must be true or false
      # - "reason" must be at least 20 characters
      # - if eligible=true, reason must cite regulatory provision
      # - confidence must be between 0.0 and 1.0

    preconditions:
      - "caseFile.application != null"
      - "caseFile.intake.isComplete = true"

defaultAutonomy: "assistive"
```

When the WOS Processor invokes an agent capability, it first validates that the preconditions are met (FEL expressions evaluated against case file data), then prepares the agent input by executing the `inputContract.mapping` via the Mapping DSL. When the agent produces output, the Processor validates it against the `outputContract` Formspec Definition (headless contract validation), then evaluates deontic constraints. Formspec validation runs first (structural and behavioral correctness). Deontic constraints run second (governance correctness). Both produce provenance records.

### 9.3 Autonomy Levels

| Level | Semantics |
|-------|-----------|
| `autonomous` | Output committed without human review. REQUIRES deontic constraints. PROHIBITED for `rights-impacting`/`safety-impacting` unless elevated. |
| `supervisory` | Provisionally committed. Human reviews within `reviewWindow`. |
| `assistive` | Recommendation only. Human reviews, modifies, confirms. |
| `manual` | Human performs. Agent assists on demand only. |

### 9.4 Deontic Constraint Framework

All agent constraints classified using LegalRuleML's deontic vocabulary:

**Permissions** — what the agent is allowed to do, within bounds.

**Prohibitions** — what the agent must not do, regardless of confidence.

**Obligations** — what the agent must do. Checked after output and before commit.

**Rights** — what the agent is entitled to receive as input. The WOS Processor has an Obligation to provide the data specified in the agent's Rights.

Deontic constraints are evaluated after Formspec contract validation and before commit. Evaluation order: Permissions, Prohibitions, Obligations, Confidence floor, Volume constraints, Human review sampling. SHACL equivalence: every deontic constraint has an equivalent SHACL shape defined in the WOS-SHACL Tier Specification.

### 9.5 Confidence Framework

ConfidenceReport: `overall` (0.0–1.0), `method`, `calibrationStatus`, optional `fieldLevel`. Expired calibration caps autonomy at `assistive`.

### 9.6 Fallback Chains

Ordered degradation. MUST terminate in `escalateToHuman` or `fail`. MUST NOT cycle.

### 9.7 Input Preparation and Isolation

`sanitize`, `maxInputTokens`, `redactFields`/`includeFields`, `isolateUntrustedData` (CaMeL dual-LLM architecture).

### 9.8 Monitoring and Drift Detection

Agent states: `active`, `degraded`, `suspended`, `retired`. Drift detection methods. Shadow deployment RECOMMENDED for model changes in `rights-impacting` workflows.

### 9.9 Multi-Step Sessions

Sessions with checkpoints, intervention points, cumulative confidence tracking.

### 9.10 Tool Use Governance

Permitted/prohibited tool registries. No direct case file writes. Cascading autonomy declared with bounded depth.

### 9.11 Assist Governance Interface

When an AI agent assists a human reviewer during a Formspec-driven task, the agent interacts with the form via the Formspec Assist tool catalog. The Assist Governance Interface sits between the agent consumer and the Formspec Assist Provider, intercepting every tool invocation and applying the WOS governance envelope.

**Introspection tools** (`formspec.form.describe`, `formspec.field.list`, `formspec.field.describe`, `formspec.field.help`, `formspec.form.progress`) — passed through without governance interception. The `FieldHelp` response is recorded in provenance as part of the agent's input context (Layer 1). Exception: when the oversight protocol is `independentFirst`, the interface suppresses agent-generated `FieldHelp.summary` content until the reviewer's independent assessment is recorded.

**Mutation tools** (`formspec.field.set`, `formspec.field.bulkSet`) — intercepted by the deontic constraint evaluator:

1. **Permission check.** If the mutation's `path` is outside the agent's Permission scope, the mutation is rejected with a `guardrailViolation` provenance record. The Assist Provider is not invoked.
2. **Prohibition check.** If the mutation's `path` and `value` match a Prohibition pattern, the mutation is rejected.
3. **Obligation check.** After the Assist Provider processes the mutation and returns a `SetValueResult`, Obligations are evaluated against the cumulative form state.
4. **Confirmation enforcement.** If the ActivityDefinition specifies that agent-initiated mutations require human confirmation, the interface sets `confirm: true` on all mutation invocations.

**Profile tools** (`formspec.profile.apply`) — intercepted. The `matches` array is filtered to remove entries outside the agent's Permission scope or matching Prohibitions. Filtered entries appear in the `ProfileApplyResult.skipped` array with reason `PROHIBITED`.

**Validation and navigation tools** — passed through.

Every Assist tool invocation through the Governance Interface produces a provenance record: tool name, input, output, timestamp, and agent ID at Layer 1; deontic constraints evaluated at Layer 2; agent explanation at Layer 3 (non-authoritative).

The Assist Governance Interface is REQUIRED when a WOS Processor implements both the Agent Governance and Task Management profiles and an ActivityDefinition uses `formRef` with agent assistance. The normative interface contract is defined in Appendix E.

---

## 10. Kernel Layer 5: Case State and Evidence

This section is normative. This is a kernel layer.

### 10.1 Object-Centric Case Model

The Case File is modeled as typed objects with temporal Event-to-Object (E2O) relationships, following OCEL 2.0. Events mutating multiple objects produce a single event record with multiple E2O links.

### 10.2 Data Mutation Semantics

Every mutation recorded as an immutable E2O provenance event. A WOS Processor MUST reconstruct any object's state at any prior point.

### 10.3 Evidence Management

Claim check pattern: content hash (SHA-256+), content type, claim check URI. Trust labeling: verified evidence, untrusted evidence, agent-generated content.

### 10.4 Selective Visibility

Role-based field-level access: `read`, `readWrite`, `none`.

---

## 11. Profile Layer 6: Integration and Eventing

This section is normative. This is a profile layer.

### 11.1 Integration Types

WOS v5 defines six integration types:

| Type | Description |
|------|-------------|
| `request-response` | Synchronous HTTP call. OpenAPI reference. |
| `event-emit` | Outbound event. |
| `event-consume` | Inbound event with correlation. |
| `callback` | Long-running asynchronous call with callback. |
| `arazzo-sequence` | Multi-step API orchestration sequence. Arazzo document reference. |
| `tool` | Non-HTTP invocation (command-line, batch, database procedure, graph query). |

All integration types support Formspec Definition references for request and response contracts via `requestContract.definitionRef` and `responseContract.definitionRef`. When a response contract is specified, the integration response is validated against the headless Formspec Definition before results are committed to the case file. The resulting ValidationReport is ingested as a provenance record.

### 11.2 Arazzo Integration Sequences

The `arazzo-sequence` type references an Arazzo document (OpenAPI Initiative) for multi-step API orchestration sequences with dependencies, conditional logic, and data passing between steps.

```yaml
integrations:
  eligibilityCheck:
    type: "arazzo-sequence"
    ararzoRef: "urn:wos:example.gov:arazzo:eligibility-check:1.0.0"
    responseContract:
      definitionRef: "urn:wos:example.gov:contracts:eligibility-response:1.0.0"
    inputMapping:
      applicantSSN: "caseFile.application.ssn"
      householdSize: "caseFile.application.householdSize"
    outputBinding:
      caseFile.eligibility.result: "$.steps.eligibility.output"
    idempotencyKey: "caseFile.application.id"
```

Each step in the sequence produces a provenance record. When the sequence invokes an AI agent, the invocation is subject to WOS's agent governance.

### 11.3 Non-HTTP Tool Invocations

The `tool` type defines non-HTTP invocations informed by CWL's `CommandLineTool` descriptor pattern:

```yaml
integrations:
  legacyCheck:
    type: "tool"
    invocation:
      method: "command-line"        # or batch-file, database-procedure, graph-query, x-custom
      command: "/opt/legacy/check"
      arguments:
        - "--ssn"
        - "{{ caseFile.application.ssn }}"
      environment:
        image: "legacy-tools:2024.1"
    inputContract:
      definitionRef: "urn:wos:example.gov:contracts:legacy-input:1.0.0"
    outputContract:
      definitionRef: "urn:wos:example.gov:contracts:legacy-output:1.0.0"
    resourceRequirements:
      maxExecutionTime: "PT30S"
```

### 11.4 Event Envelope

CloudEvents 1.0 with WOS extensions: `wosinstanceid`, `wosdefid`, `wosdefversion`, `wosstate`, `wostaskid`, `woscorrelationkey`, `woscausationeventid`.

### 11.5 Idempotency and Correlation

Event consumption idempotent. Correlation via attribute-to-case-file-path mapping.

### 11.6 Capability Advertisement

Schema.org `potentialAction` for workflow capability discovery.

### 11.7 Protocol Alignment

**MCP** for agent-tool integration within governance envelope. **A2A** for inter-agent communication. **SOM/AWP** acknowledged as emerging; extension points defined.

---

## 12. Kernel Layer 7: Provenance and Audit

This section is normative. This is a kernel layer.

### 12.1 Four-Layer Audit Architecture

| Layer | Name | Content | Authority |
|-------|------|---------|-----------|
| 1 | Immutable Facts | Timestamp, actor, model version, inputs, outputs, policy version, confidence, reviewer ID. | **Authoritative.** |
| 2 | Structured Reasoning | Rules applied, evidence consulted, criteria checked, decision table trace. Formspec ValidationResult entries with severity, path, code, constraintKind. | **Authoritative** for deterministic logic. Descriptive for agent reasoning. |
| 3 | Generated Narrative | Model's natural language explanation. | **Informational only.** Labeled non-authoritative. |
| 4 | Counterfactual | What would change the outcome. | Informational. **Required** for adverse decisions in `rights-impacting` workflows. |

### 12.2 PROV-AGENT Alignment

Provenance records are JSON-LD aligned with PROV-O and PROV-AGENT.

### 12.3 Object-Centric Event Logging

OCEL 2.0 Event-to-Object and Object-to-Object mapping.

### 12.4 Record Types

`transition`, `decision`, `agentInvocation`, `agentCheckpoint`, `agentToolUse`, `taskOperation`, `dataMutation`, `override`, `guardrailViolation`, `guardrailBypass`, `autonomyChange`, `modelVersionChange`, `driftAlert`, `dueProcessNotice`, `appealFiled`, `patchApplied`, `contractValidation`, `formEvent`, `assistInvocation`.

### 12.5 Tamper Evidence

Merkle tree hash-chaining with SHA-256, signed tree heads, inclusion and consistency proofs.

### 12.6 Formspec Provenance Integration

Every time the WOS Processor validates data against a Formspec Definition — whether for a human task submission, an agent output, a decision service response, or an integration response — the resulting Formspec ValidationReport is ingested as a WOS provenance record.

**ValidationReport → Layer 1 + Layer 2.** The `valid` flag, `counts` (error/warning/info), timestamp, actor, and context identifier are Layer 1 facts. The individual ValidationResult entries — each with `severity`, `path`, `message`, `code`, `constraintKind`, and `source` — are Layer 2 structured reasoning, providing a trace of what validation rules were applied and what results were produced. The `constraintKind` taxonomy (`required`, `type`, `cardinality`, `constraint`, `shape`, `external`) maps directly to WOS's need to understand why data was accepted or rejected.

**Respondent Ledger events → Layer 1.** For human task contexts with a Formspec Respondent Ledger, material events are ingested as WOS provenance records at lifecycle boundaries: `session.started`, `draft.saved` (when validation state changes), `response.submit-attempted`, `response.completed`, `response.amendment-opened`, `response.amended`, `response.stopped`, `attachment.*`, `identity-verified`, `response.migrated`.

**Agent Assist events → Layer 1 + Layer 3.** Assist tool invocations through the Governance Interface are captured with tool name, input, and output at Layer 1. Agent explanations are Layer 3 (non-authoritative).

**Tamper evidence continuity.** When integrity chaining is enabled on both the Respondent Ledger and the WOS provenance stream, the WOS Processor cross-references the Ledger's checkpoint hashes in WOS provenance records, creating a tamper-evidence chain spanning from form to workflow.

### 12.7 Process Mining Interoperability

Primary: OCEL 2.0. Secondary: IEEE XES.

### 12.8 Provenance Export Packaging

RO-Crate with Workflow Run Crate profile.

---

## 13. Kernel Layer 8: Durable Execution Contract

This section is normative. This is a kernel layer.

### 13.1 Durability Guarantees

**G1: Crash Recovery.** Non-terminal instances resume from last durable state.

**G2: Persistent State.** Lifecycle state, case file objects, task states, timer registrations durably persisted.

**G3: Deterministic Replay.** Every action invoking a non-deterministic external service (including AI agent invocations) MUST persist the output as an immutable step result before advancing workflow state. During crash recovery, workflow resumption, or audit replay, the WOS Processor MUST use the persisted output from the first successful execution rather than re-invoking the external service. Re-invocation during replay is a conformance violation.

**G4: Durable Timers.** Survive restarts, fire within tolerance, consume no resources while waiting.

**G5: External Signal Delivery.** Signals to inactive instances durably enqueued.

### 13.2 Retry Policy

`maxAttempts`, `backoff`, `initialInterval`, `maxInterval`, `multiplier`, `nonRetryableErrors`. All external invocations carry idempotency keys.

### 13.3 Compensation

Activities with side effects SHOULD register compensation handlers. Reverse completion order.

---

## 14. Actor Model

This section is normative.

| Type | Description | Provenance |
|------|-------------|-----------|
| `human` | Person performing tasks. | Identity, role, timestamp. |
| `system` | Deterministic component. | Component ID, version, timestamp. |
| `agent` | AI system, non-deterministic, carries confidence. Outside trust boundary. | Model ID, version, confidence, input summary, all PROV-AGENT fields. |

Agents MUST NOT override human decisions. Cascading autonomous agents require declaration with bounded depth.

---

## 15. Expression Language

This section is normative.

### 15.1 Overview

WOS v5 uses the Formspec Expression Language (FEL) as its single expression language for guard conditions, data transformations, computed case file values, milestone conditions, deontic constraint expressions, Mapping DSL transforms, and contract validation constraints. FEL is a small, deterministic, side-effect-free language with a normative PEG grammar, an explicit type system (no truthy/falsy coercion), defined null propagation rules, and a comprehensive built-in function library.

The FEL normative grammar, type system, operators, built-in functions, null propagation rules, and dependency tracking algorithm are defined in the Formspec Expression Language Normative Grammar v1.0 (companion document to Formspec v1.0) and incorporated by reference. This section specifies the WOS-specific conformance profiles and extensions.

### 15.2 Conformance Profiles

**FEL Core** (Formspec baseline) — the current FEL v1.0 specification. All WOS Processors MUST implement FEL Core. Covers guard conditions, data transformations, milestone conditions, deontic constraint expressions, and simple computed values.

**FEL Decision** — FEL Core plus quantified expressions, range literals with range membership, and duration arithmetic. WOS Processors implementing the Decision conformance profile MUST implement FEL Decision. Covers decision table conditions, temporal parameter evaluation, SLA computation, and guard expressions involving universal or existential quantification.

**FEL Extended** — FEL Decision plus filter expressions. WOS Processors implementing the Agent Governance conformance profile SHOULD implement FEL Extended. Covers complex case file queries in capability preconditions and deontic constraint expressions.

The grammar additions for each profile are defined in Appendix C.

### 15.3 DMN Compatibility

WOS does not implement FEEL. A normative FEEL-to-FEL translation table (Appendix D) enables mechanical, bidirectional translation between the two languages. Conformant Processors MAY accept FEEL expressions and translate internally; this is OPTIONAL.

### 15.4 Null Propagation in WOS Contexts

| Context | `null` treated as | Rationale |
|---------|-------------------|-----------|
| Guard condition | `false` | Cannot-evaluate guard does not fire. |
| Milestone condition | `false` | Cannot-evaluate milestone is not achieved. |
| Decision table input | `null` propagates | Per FEL §3.8.1. |
| Deontic constraint | `true` (passes) | Cannot-evaluate constraint is not violated. |
| Contract validation | Per FEL §3.8 | Same rules as Formspec form validation. |

### 15.5 Expression Context

FEL expressions in WOS have access to the following context variables: `caseFile`, `event`, `task`, `instance`, `parameters`, `agent`, `env`, `output` (in deontic constraint and contract validation expressions).

---

## 16. Due Process Requirements

This section is normative for `rights-impacting` and `safety-impacting` workflows.

### 16.1 Notice

Adverse decisions require notice before effect: specific determination, factual basis with individualized reason codes, appeal rights and deadline, agent disclosure.

### 16.2 Explanation Levels

`individualized` (REQUIRED for `rights-impacting`), `categorical`, `aggregate`. Counterfactuals required: positive and negative.

### 16.3 Appeal Mechanisms

Human adjudicator independent of original determination. Agents MUST NOT decide appeals. Continuation of services during appeal.

### 16.4 Agent Disclosure

`discloseThatAgentAssisted: true` REQUIRED for `rights-impacting`.

### 16.5 Continuation-of-Service States

When `continuationOfServices` is true, the workflow MUST include topology that freezes adverse impacts and maintains current service levels during appeal.

---

## 17. AI-Native Authoring and Patch Operations

This section is normative.

### 17.1 Overview

WOS is designed for AI-assisted authoring. LLMs propose structural modifications as typed patch operations against the document's abstract syntax tree, validated through a four-stage pipeline before commit.

### 17.2 Patch Operation Structure

A PatchSet is a JSON-LD document with typed operations (`insertState`, `addTransition`, `addDeonticConstraint`, `addActivityRef`, `addContractRef`, etc.). Each operation has defined preconditions and postconditions.

### 17.3 Validation Pipeline

1. **Structural validation.** JSON Schema validation of resulting document.
2. **SHACL governance validation.** Policy-level cross-cutting constraints including contract reference resolution.
3. **Soundness verification.** Lifecycle soundness including fallback completeness.
4. **Provenance recording.** `patchApplied` record capturing author, operations, validation results.

### 17.4 Compositional Authoring

AI-assisted authoring is compositional: an LLM composing a new workflow can query an ActivityDefinition registry for existing activities (with their Formspec form contracts), compose a WorkflowDefinition referencing them by URI, and propose only the topology and overrides as a PatchSet. Each ActivityDefinition's Formspec Definition contract can be discovered and browsed via SPARQL if Formspec publishes its `@context`.

---

## 18. Versioning and Evolution

This section is normative.

All publishable documents use Semantic Versioning independently. Default: pinned execution. Optional: forward migration with safety verification. The `@context` is versioned with the spec; breaking `@context` changes = spec major version increment.

ActivityDefinition version changes do not require WorkflowDefinition version changes unless overrides are affected. Formspec Definition version changes referenced by ActivityDefinitions are assessed via Formspec Changelog Documents — `patch` impact requires no workflow change, `minor` requires mapping review, `major` requires mapping update and migration.

---

## 19. Security and Access Control

This section is normative.

Roles: `workflowAdministrator`, `instanceInitiator`, `caseParticipant`, `taskWorker`, `taskAdministrator`, `auditor`. Authorization enforced at all access points. Failures logged.

---

## 20. Extensibility

This section is normative.

Namespaced extension properties (`x-agency:classification`). MUST NOT alter core semantics. MUST be preserved during round-trips. MUST NOT use `wos:` or `formspec:` prefixes. MAY add `@context` entries. WOS and Formspec share identical extension model semantics — implementations can handle extensions from both specifications using the same machinery.

---

## 21. Conformance Profiles

This section is normative.

### 21.1 Profile: Structural

Parse, validate, round-trip, preserve `@context` and extensions. Resolve Formspec Definition references in `definitionRef` properties. Enables editors, validators, linters.

### 21.2 Profile: Kernel

Structural + execute kernel layer semantics. Produce transition and data mutation records. Satisfy durability guarantees including deterministic replay (G3). Implement FEL Core. Implement the DAG-based processing model for guard and milestone evaluation (§6.8).

### 21.3 Profile: Task Management

Kernel + full task lifecycle, all operations, separation of duties, SLA timers, structured oversight protocols. MUST support `formRef` mode on ActivityDefinitions. MUST implement Mapping DSL Core for `inputMapping` and `outputBinding` execution. MUST validate Formspec Responses against their Definitions and ingest ValidationReports as provenance records.

### 21.4 Profile: Decision

Kernel + decision tables with all hit policies, FEL Decision profile, temporal parameters, decision records. MUST validate decision service outputs against `outputContract` Formspec Definitions.

### 21.5 Profile: Agent Governance

Kernel + Decision + enforce deontic constraints, produce all agent provenance types (PROV-AGENT), enforce autonomy levels, enforce fallback chains, validate capability contracts (Formspec Definition pre/post validation), confidence-based routing, multi-step sessions. When combined with Task Management profile and ActivityDefinition uses `formRef` with agent assistance, MUST implement the Assist Governance Interface (§9.11, Appendix E).

### 21.6 Profile: Full

All of Kernel + Task + Decision + Agent + integration semantics with correlation (including `arazzo-sequence` and `tool` types), all provenance types including Formspec provenance ingestion, tamper evidence, access control, due process for `rights-impacting`, OCEL 2.0 E2O logging.

### 21.7 Profile: Verification

Structural + static soundness analysis, PatchSet validation pipeline, Formspec Definition reference resolution verification, diagnostic reports.

### 21.8 Profile: Semantic

Structural + valid JSON-LD with `@context` (composing WOS and Formspec contexts), SHACL governance validation, PROV-O/PROV-AGENT provenance graphs, SPARQL querying, OCEL 2.0 event emission.

### 21.9 Profile: Authoring

Structural + Verification + accept and validate PatchSets, execute four-stage validation pipeline (including contract reference resolution), produce `patchApplied` provenance records, support compositional authoring via ActivityDefinition and Formspec Definition registry queries.

---

## 22. Privacy Considerations

This section is informative.

`visibility` restricts access. `redactFields` limits agent exposure. Trust labeling separates verified from untrusted. Claim check pattern. Formspec Respondent Ledger events ingested into WOS provenance may contain personal data; implementations SHOULD support configurable retention and anonymization consistent with the Ledger's privacy tier model.

---

## 23. Security Considerations

This section is informative.

1. **Expression sandboxing.** FEL expressions evaluated in isolated context with no system access or side effects.
2. **Event authentication.** Signature verification.
3. **Provenance integrity.** Independent signed tree heads.
4. **Encryption.** At rest and in transit.
5. **Separation of duties.** Application-layer enforcement.
6. **Agent impersonation prevention.** Same authentication rigor as humans.
7. **Prompt injection defense.** `isolateUntrustedData` for CaMeL pattern. Deontic constraints as structural defense. Defense in depth.
8. **Model version drift.** Shadow deployment before production.
9. **Cascading autonomy.** Declared and bounded.
10. **Tool use.** Least-privilege. Recorded. Side effects require policy.
11. **`@context` integrity.** HTTPS, caching, hash verification for both WOS and Formspec contexts.
12. **Patch security.** PatchSets from untrusted sources MUST pass full validation pipeline. Patches weakening deontic constraints or removing due process configurations require elevated authorization.
13. **Assist Governance.** All Assist tool invocations from agents pass through the Governance Interface. Agents MUST NOT bypass the interface to interact with forms directly.
14. **Contract integrity.** Formspec Definitions used as interface contracts MUST be served from trusted sources. A compromised contract Definition could weaken validation constraints on agent outputs or integration responses.
15. **Formspec sidecar provenance.** References and Ontology documents loaded for Formspec-driven tasks should be verified for provenance. Loading sidecars from untrusted sources could inject misleading context into agent pipelines.

---

## 24. References

### 24.1 Normative References

**[RFC2119]**, **[RFC3339]**, **[RFC3986]**, **[RFC8174]**, **[RFC8259]**, **[YAML]**, **[JSON-LD11]**, **[RDF11]**, **[SHACL]**, **[PROV-O]**, **[PROV-DM]**, **[SemVer]**, **[ISO8601]**, **[CloudEvents]**, **[TraceContext]**, **[DMN]**, **[OpenAPI]**, **[AsyncAPI]**, **[JSONSchema]**, **[LegalRuleML]** — as in prior versions.

**[Formspec]** Formspec Working Group, "Formspec v1.0 — A JSON-Native Declarative Form Standard", 2025.

**[FEL]** Formspec Working Group, "Formspec Expression Language (FEL) — Normative Grammar", Version 1.0.

**[FormspecMappingDSL]** Formspec Working Group, "Formspec Mapping DSL v1.0 — Bidirectional Data Transformation", 2025.

**[FormspecAssist]** Formspec Working Group, "Formspec Assist Specification v1.0", 2026.

**[FormspecOntology]** Formspec Working Group, "Formspec Ontology Specification v1.0", 2026.

**[FormspecReferences]** Formspec Working Group, "Formspec References Specification v1.0", 2026.

**[FormspecLedger]** Formspec Working Group, "Respondent Ledger Add-On Specification v0.1", 2026.

**[FormspecRegistry]** Formspec Working Group, "Formspec Extension Registry v1.0", 2025.

**[FormspecChangelog]** Formspec Working Group, "Formspec Changelog Format v1.0", 2025.

**[Arazzo]** OpenAPI Initiative, "Arazzo Specification", 2024.

### 24.2 Informative References

**[Harel1987]**, **[SCXML]**, **[BPMN]**, **[CMMN]**, **[WS-HumanTask]**, **[XACML]**, **[Sagas]**, **[WorkflowPatterns]**, **[OCEL2]**, **[XES]**, **[RO-Crate]**, **[WfRunCrate]**, **[PROV-AGENT]**, **[RFC9162]**, **[NIST-SP-800-53]**, **[OMB-M-24-10]**, **[EU-AI-Act]**, **[NIST-AI-RMF]**, **[Vaccaro2024]**, **[Turpin2023]**, **[Lanham2023]**, **[Chen2023]**, **[Buçinca2021]**, **[Nasr2025]**, **[CaMeL]**, **[FormalLLM]**, **[ABC]**, **[Feng2025]**, **[Wachter2018]**, **[MCP]**, **[A2A]**, **[BBO]**, **[FHIR-Workflow]**, **[OASF]**, **[CanadaDirective]**, **[SchemaActions]** — as in prior versions.

**[CWL]** Amstutz, P., et al., "Common Workflow Language Specification v1.2", 2023.

---

## Appendix A. JSON-LD Context Document

Normative. Published at `https://wos-spec.org/context/5.0.0`. Extends v4 context with Formspec context composition and contract-related terms.

```json
{
  "@context": [
    "https://formspec.org/context/1.0.0",
    {
      "@version": 1.1,
      "wos": "https://wos-spec.org/ns/",
      "schema": "https://schema.org/",
      "prov": "http://www.w3.org/ns/prov#",
      "sh": "http://www.w3.org/ns/shacl#",
      "lrml": "http://docs.oasis-open.org/legalruleml/ns/v1.0/",
      "xsd": "http://www.w3.org/2001/XMLSchema#",
      "dcterms": "http://purl.org/dc/terms/",

      "id": "@id",
      "type": "@type",

      "WorkflowDefinition": "wos:WorkflowDefinition",
      "ActivityDefinition": "wos:ActivityDefinition",
      "DecisionServiceDefinition": "wos:DecisionServiceDefinition",
      "AgentConfiguration": "wos:AgentConfiguration",
      "PatchSet": "wos:PatchSet",

      "formRef": { "@id": "wos:formRef", "@type": "@id" },
      "definitionRef": { "@id": "wos:definitionRef", "@type": "@id" },
      "inputMapping": "wos:inputMapping",
      "outputBinding": "wos:outputBinding",
      "inputContract": "wos:inputContract",
      "outputContract": "wos:outputContract",
      "requestContract": "wos:requestContract",
      "responseContract": "wos:responseContract",

      "deonticConstraints": "wos:deonticConstraints",
      "permissions": { "@id": "wos:permissions", "@container": "@list" },
      "prohibitions": { "@id": "wos:prohibitions", "@container": "@list" },
      "obligations": { "@id": "wos:obligations", "@container": "@list" },
      "rights": { "@id": "wos:rights", "@container": "@list" },
      "Permission": "lrml:Permission",
      "Prohibition": "lrml:Prohibition",
      "Obligation": "lrml:Obligation",
      "Right": "lrml:Right",

      "lifecycle": "wos:lifecycle",
      "states": { "@id": "wos:states", "@container": "@index" },
      "transitions": { "@id": "wos:transitions", "@container": "@list" },
      "guard": "wos:guard",
      "target": { "@id": "wos:targetState", "@type": "@id" },
      "autonomy": "wos:autonomyLevel",
      "confidence": "wos:confidence",
      "impactLevel": "wos:impactLevel",

      "timestamp": { "@id": "prov:atTime", "@type": "xsd:dateTime" },
      "actor": { "@id": "prov:wasAssociatedWith", "@type": "@id" },
      "recordType": "wos:recordType",
      "auditLayer": "wos:auditLayer",

      "ararzoRef": { "@id": "wos:ararzoRef", "@type": "@id" },
      "invocation": "wos:invocation",

      "operations": { "@id": "wos:operations", "@container": "@list" },
      "op": "wos:patchOp",
      "path": "wos:patchPath",
      "value": "wos:patchValue"
    }
  ]
}
```

The first element of the `@context` array is the Formspec context document. This means every WOS document that includes this context also includes Formspec's term mappings, enabling the combined graph to be queried seamlessly.

---

## Appendix B. SHACL Shapes for Structural Governance

Normative. Extends v4 shapes with Formspec contract validation and Assist Governance requirements.

```turtle
@prefix wos: <https://wos-spec.org/ns/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .

# Rights-impacting workflows must have due process with appeals
# and continuation-of-service topology
wos:RightsImpactingDueProcessShape
  a sh:NodeShape ;
  sh:targetClass wos:WorkflowDefinition ;
  sh:sparql [
    sh:message "Rights-impacting workflows MUST include enabled appeal with continuation-of-service." ;
    sh:select """
      SELECT $this WHERE {
        $this wos:impactLevel "rights-impacting" .
        FILTER NOT EXISTS {
          $this wos:dueProcess/wos:appealMechanism/wos:enabled true .
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

# Agent capability contracts must reference valid Formspec Definitions
wos:CapabilityContractShape
  a sh:NodeShape ;
  sh:targetClass wos:AgentConfiguration ;
  sh:sparql [
    sh:message "Agent capability contracts MUST reference resolvable Formspec Definitions." ;
    sh:select """
      SELECT $this ?cap WHERE {
        $this wos:capabilities ?cap .
        ?cap wos:outputContract/wos:definitionRef ?ref .
        FILTER (!isIRI(?ref))
      }
    """ ;
  ] .

# ActivityDefinitions with formRef and agent assistance must
# use the Assist Governance Interface
wos:AssistGovernanceShape
  a sh:NodeShape ;
  sh:targetClass wos:ActivityDefinition ;
  sh:sparql [
    sh:message "Activities with formRef and agent assistance MUST specify oversight protocol." ;
    sh:select """
      SELECT $this WHERE {
        $this wos:formRef ?form .
        $this wos:oversight/wos:agentAssistance ?assist .
        FILTER NOT EXISTS {
          $this wos:oversight/wos:oversightProtocol ?protocol
        }
      }
    """ ;
  ] .

# Agent fallback must terminate in human task
wos:AgentFallbackShape
  a sh:NodeShape ;
  sh:targetSubjectsOf wos:agentRef ;
  sh:sparql [
    sh:message "Every agent invocation MUST have fallback terminating in human task." ;
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
```

---

## Appendix C. FEL Conformance Profiles and Grammar Additions

Normative. Defines the three FEL conformance profiles and their grammar additions to the FEL v1.0 PEG grammar.

### C.1 FEL Core

The current FEL v1.0 specification, unmodified. All WOS Processors MUST implement.

### C.2 FEL Decision

FEL Core plus three additions:

**Quantified expressions.** Insert between `LetExpr` and `IfExpr`:

```peg
Quantified     ← ('some' / 'every') _ Identifier _ 'in' _ Expression
                  _ 'satisfies' _ IfExpr
               / IfExpr
```

New reserved words: `some`, `every`, `satisfies`. Return type: `boolean`. The binding variable is scoped to the `satisfies` expression.

**Range literals.** Extend `Membership` production:

```peg
Membership     ← NullCoalesce (_ 'not' _ 'in' _ (RangeLiteral / NullCoalesce)
                              / _ 'in' !IdContinue _ (RangeLiteral / NullCoalesce))?
RangeLiteral   ← ('[' / '(') _ Expression _ '..' _ Expression _ (']' / ')')
```

`[` inclusive, `(` exclusive. `..` reserved token.

**Duration type.** `duration('P30D')` as built-in function. Operators: `date + duration → date`, `date - date → duration`, `duration + duration → duration`, comparison on durations. Business-day/business-hour: `duration('P5BD')`, `duration('P8BH')` with calendar context from host environment.

### C.3 FEL Extended

FEL Decision plus:

**Filter expressions.** Extend `PathTail`:

```peg
PathTail       ← '.' Identifier
               / '[' _ ( Integer / '*' ) _ ']'
               / '[' _ Expression _ ']'
```

Third alternative is a filter predicate on array-typed values. `$` binds to each element. Result is filtered array.

---

## Appendix D. FEEL-to-FEL Translation Table

Non-normative. Enables mechanical bidirectional translation between FEEL and FEL.

| FEEL | FEL | Notes |
|------|-----|-------|
| `x in [1..10]` | `$x in [1..10]` | Add `$` sigil. |
| `x in (1..10]` | `$x in (1..10]` | Add `$` sigil. |
| `some x in list satisfies f(x)` | `some x in $list satisfies f(x)` | Add `$` to list reference. |
| `every x in list satisfies f(x)` | `every x in $list satisfies f(x)` | Add `$` to list reference. |
| `if c then a else b` | `if c then a else b` | Identical. |
| `list[item > 5]` | `$list[$ > 5]` | FEL uses `$` self-reference in filter. |
| `for x in list return f(x)` | `$list[*].field` or deferred | Element-wise covers most cases. |
| `date("2026-01-01")` | `@2026-01-01` | FEL `@` date literal. |
| `duration("P30D")` | `duration('P30D')` | Function call, identical semantics. |
| `string length(s)` | `length($s)` | Different convention. |
| `contains(s, sub)` | `contains($s, sub)` | Add `$` sigil. |
| `not(x)` | `not $x` | Operator vs. function. |
| `null` | `null` | Identical. |

---

## Appendix E. Assist Governance Interface

Normative. Defines the contract for wrapping Formspec Assist tool invocations with WOS deontic constraint enforcement.

### E.1 Scope

The Assist Governance Interface is REQUIRED when a WOS Processor implements both Agent Governance and Task Management profiles and an ActivityDefinition uses `formRef` with agent assistance.

### E.2 Tool-Level Governance Rules

| Tool Category | Governance | Details |
|---------------|-----------|---------|
| Introspection (`formspec.form.describe`, `field.list`, `field.describe`, `field.help`, `form.progress`) | Pass-through | `FieldHelp` recorded in provenance. `independentFirst` suppresses `summary` until independent assessment recorded. |
| Mutation (`formspec.field.set`, `field.bulkSet`) | Intercepted | Permission → Prohibition → Obligation checks before forwarding. `confirm: true` enforced when ActivityDefinition requires. |
| Validation (`formspec.form.validate`, `field.validate`) | Pass-through | Results observed by provenance recorder. |
| Profile (`formspec.profile.apply`) | Intercepted | `matches` filtered against Permission scope and Prohibition patterns. Filtered entries: reason `PROHIBITED`. |
| Navigation (`formspec.form.pages`, `form.nextIncomplete`) | Pass-through | No governance implications. |

### E.3 Provenance Recording

Every tool invocation produces: Layer 1 (tool name, input, output, timestamp, agent ID), Layer 2 (deontic constraints evaluated, pass/violate result), Layer 3 (agent explanation if provided, labeled non-authoritative).

### E.4 Oversight Protocol Enforcement

When `oversight.protocol` is `independentFirst`, the interface tracks reviewer assessment state. Agent-generated content in `FieldHelp.summary` and suggestion-type responses are suppressed until the reviewer records their independent assessment. The mechanism is: the interface intercepts `formspec.field.help` responses and strips the `summary` field; the interface intercepts `formspec.field.describe` responses and strips `help.summary`. After the independent assessment is recorded (signaled via a WOS-internal state change), the interface releases full responses.

---

## Appendix F. Patch Operation Reference

Normative. Complete catalog of typed patch operations, as defined in WOS v4 Appendix C, extended with contract-related operations.

| Operation | Description | Preconditions |
|-----------|-------------|--------------|
| `insertState` | Add state to lifecycle. | ID unique. Parent exists. |
| `removeState` | Remove state. | No incoming transitions. |
| `modifyState` | Change state properties. | State exists. |
| `addTransition` | Add transition. | Source and target exist. |
| `removeTransition` | Remove transition. | Exists. |
| `modifyTransition` | Change transition properties. | Exists. |
| `addActivityRef` | Add ActivityDefinition reference. | URI resolvable. |
| `removeActivityRef` | Remove activity reference. | No actions reference it. |
| `addDecisionRef` | Add Decision Service reference. | URI resolvable. |
| `addAgentRef` | Add Agent Configuration reference. | URI resolvable. |
| `addContractRef` | Add Formspec Definition reference to any contract slot. | URI resolvable. Definition validates. |
| `removeContractRef` | Remove contract reference. | No active bindings reference it. |
| `addDeonticConstraint` | Add Permission/Prohibition/Obligation/Right. | Target agent exists. |
| `removeDeonticConstraint` | Remove constraint. | Exists. Elevated auth required. |
| `modifyDeonticConstraint` | Change constraint. | Exists. |
| `setParameter` | Set/modify temporal parameter. | Name valid. |
| `addCaseFileItem` | Add case file item. | ID unique. |
| `modifyOverride` | Change workflow override on activity. | Activity ref exists. |
| `setImpactLevel` | Change impact level. | If escalating, due process must exist. |
| `addMilestone` | Add milestone. | ID unique. Condition valid FEL. |
| `addIntegration` | Add integration definition. | Contract refs resolvable. |

---

## Appendix G. Relationship to Existing Standards

| Standard | Relationship |
|----------|-------------|
| **Formspec v1.0** | Universal interface contract specification. Definitions used for human task forms, agent contracts, decision service contracts, and integration contracts. |
| **Formspec Mapping DSL** | Data flow language for `inputMapping` and `outputBinding` between WOS case files and Formspec instances. |
| **Formspec Assist** | Form-filling agent protocol. Governed by the Assist Governance Interface. |
| **Formspec Ontology** | Semantic bridge between WOS case file vocabularies and Formspec field concepts. |
| **Formspec References** | Contextual knowledge sources for human and agent consumers during tasks. |
| **Formspec Respondent Ledger** | Fine-grained audit trail during data collection, ingested into WOS provenance. |
| **Formspec Registry** | Extension discovery and compatibility verification. |
| **Formspec Changelog** | Version migration assessment for referenced Definitions. |
| **FEL** | Unified expression language across both specifications. |
| **JSON-LD 1.1 / RDF 1.1** | Native serialization. Combined WOS + Formspec context for unified graph. |
| **SHACL** | Governance validation for definitions and deontic constraint enforcement. |
| **PROV-O / PROV-AGENT** | Provenance alignment. Four-layer audit as PROV extension. |
| **LegalRuleML** | Deontic vocabulary for agent governance. |
| **OCEL 2.0** | Object-centric case model and process mining. |
| **RO-Crate** | Audit archive packaging. |
| **Arazzo** | Multi-step API orchestration sequences for integration layer. |
| **CWL** | Inspiration for non-HTTP tool descriptor pattern. |
| **HL7 FHIR Workflow** | Inspiration for tripartite object model. |
| **BPMN / CMMN / DMN / SCXML** | Conceptual heritage. |
| **CloudEvents** | Event envelope. |
| **MCP / A2A** | Agent-tool and agent-agent interoperability. |
| **OMB M-24-10 / EU AI Act / Canada Directive** | Due process requirements operationalized as workflow constraints. |

---

## Appendix H. Changelog

| Date | Version | Description |
|------|---------|-------------|
| 2026-04-09 | 5.0.0 | Formspec as universal interface contract (human tasks, agent capabilities, decision services, integrations). FEL as unified expression language with Core/Decision/Extended profiles. DAG-based processing model for guard and milestone evaluation. Assist Governance Interface for form-level agent governance. Arazzo integration sequences. CWL-informed non-HTTP tool descriptors. Formspec provenance integration (ValidationReport, Respondent Ledger, Assist events). Combined WOS + Formspec JSON-LD context. |
| 2026-04-09 | 4.0.0 | Tripartite object model. Deontic constraints. Typed patch operations. Kernel-versus-profiles. Deterministic replay. Capability contracts. |
| 2026-04-09 | 3.0.0 | JSON-LD native. SHACL governance. PROV-AGENT. OCEL 2.0. RO-Crate. Semantic Profile. |
| 2026-04-08 | 2.0.0 | Eight-layer architecture. Four-layer audit. Structured oversight. Due process. |
