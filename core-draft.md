# Workflow Orchestration Standard (WOS) Core Specification

## Final Draft

**Latest published version:**
: https://wos-spec.org/TR/wos-core/

**Editors:**
: Mike (TealWolf Consulting LLC)

**Version:**
: 6.0.0

**Date:**
: 9 April 2026

**Status:**
: Final Draft for Community Review

---

## Abstract

This specification defines the Workflow Orchestration Standard (WOS), a declarative, machine-readable language for describing high-stakes, long-running workflows in which humans and AI agents collaborate on consequential decisions. WOS provides a governance layer above transport protocols, a constraint language alongside execution engines, and an accountability framework embedded in the workflow itself.

WOS is organized as a **Constraint-Enhanced Layered Kernel**: four required kernel layers (Lifecycle, Case State, Provenance, Durable Execution) surrounded by four independently adoptable profile layers (Decision, Human Task, Agent Governance, Integration). Every structured data exchange — human task rms, agent input/output contracts, decision service interfaces, integration schemas — uses Formspec Definitions as the universal interface contract. The Formspec Expression Language (FEL) is the single expression language for guard conditions, data transformations, deontic constraints, contract validation, and data flow mappings.

Six foundational innovations define this specification:

**A tripartite object model** separating ActivityDefinitions (reusable work templates), WorkflowDefinitions (process topologies), and Tasks (runtime instances), enabling cross-program sharing and compositional AI-assisted authoring.

**A deontic governance framework** classifying agent constraints as Permissions, Prohibitions, Obligations, or Rights, adopted from OASIS LegalRuleML, with a formally verifiable constraint subset amenable to SMT-based proof before deployment.

**Declarative constraint zones** within the lifecycle topology, enabling adaptive case management phases where valid actions are governed by DCR-style rations (condition, response, include, exclude, milestone) rather than explicit transitions — proven at government scale by Danish central government adoption.

**An Assist Governance Interface** ensuring that WOS's deontic constraint framework governs AI agent behavior during form-level interactions with the same rigor as workflow-level interactions.

**Agent behavioral attestations** providing independently verifiable evidence of an agent's evaluated behavioral characteristics, bridging the gap between self-declared autonomy levels and the independent evaluation requirements of the EU AI Act, Canada's Directive on Automated Decision-Making, and OMB M-24-10.

**Dual-readability provenance** requiring that adverse decision records in rights-impacting workflows include both structured provenance data and a deterministically generated human-readable narrative suitable for due process notices.

WOS documents are natively serialized as JSON-LD. Guardrails may be expressed as SHACL shapes. Provenance records aln with W3C PROV-O and PROV-AGENT. Case data links to domain vocabularies through JSON-LD context extension.

The standard treats human authority as supreme, AI participation as governed, and audit as foundational. It is informed by empirical research demonstrating that naive human-in-the-loop designs degrade decision quality (Vaccaro et al., 2024), that model-generated explanations are unreliable audit evidence (Turpin et al., 2023; Lanham et al., 2023), that behavioral drift between model versions can be catastrophic (Chen et al., 2023), and that no single defense prevents prompt injection (Nasr et al., 2025). These findings are encoded as structural requirements.

---

## Status of This Document

This document is a Final Draft for Community Review. It incorporates feedback from six prior working drafts and two landscape research passes covering 80+ standards, formal models, and governance frameworks. Comments may be submitted as issues at the specification's repository.

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
18. [Versioning, Evolution, and Instance Migration](#18-versioning-evolution-and-instance-migration)
19. [Security and Access Control](#19-security-and-access-control)
20. [Extensibility](#20-extensibility)
21. [Conformance Profiles](#21-conformance-profiles)
22. [Companion Specification Contracts](#22-companion-specification-contracts)
23. [Privacy Considerations](#23-privacy-considerations)
24. [Security Considerations](#24-security-considerations)
25. [References](#25-references)

**Appendices**

- [A. JSON-LD Context Document](#appendix-a-json-ld-context-document)
- [B. SHACL Shapes for Structural Governance](#appendix-b-shacl-shapes-for-structural-governance)
- [C. FEL Conformance Profiles and Grammar Additions](#appendix-c-fel-conformance-profiles-and-grammar-additions)
- [D. FEEL-to-FEL Translation Table](#appendix-d-feel-to-fel-translation-table)
- [E. Assist Governance Interface](#appendix-e-assist-governance-interface)
- [F. Verifiable Constraint Subset](#appendix-f-verifiable-constraint-subset)
- [G. Constraint Zone Semantics](#appendix-g-constraint-zone-semantics)
- [H. Patch Operation Reference](#appendix-h-patch-operation-reference)
- [I. Relationship to Existing Standards](#appendix-i-relationship-to-existing-standards)
- [J. Changelog](#appendix-j-changelog)

---

## 1. Introduction

### 1.1 Background

High-stakes workflows — grants processing, benefits adjudication, licensing, inspections, investigations, compliance review — share requirements that no existing standard adequately integrates. They long-running, human-centric, evidence-driven, heavily regulated, and increasingly involve AI agents. Existing process standards (BPMN, CMMN, Serverless Workflow) were engineered for deterministic execution and lack native concepts for governing probabilistic, non-deterministic AI participants. Agent transport protocols (MCP, A2A) solve the discovery and invocation problem but provide no governance — they specify how an agent calls a tool but not whether the agent should call it, under what constraints, with what review, or how to audit the interaction.

This specification provides the governance layer that sits above tnsport protocols and alongside execution engines. It defines what agents may do (Permissions), what they must not do (Prohibitions), what they must do (Obligations), what they are entitled to receive (Rights), what evidence must be recorded, what human oversight is required, and how all of this is auditable.

The specification is informed by three bodies of empirical evidence that constrain its design. First, a meta-analysis of 106 experiments demonstrates that naive human-AI combinations degrade decision quality compared to either humans or AI alone (Vaccaro et al., Nature Human Behaviour, 2024), necessitating structured oversight protocols rather than checkbox review. Second, research on chain-of-thought faithfulness demonstrates that model-generated explanations are systematically post-hoc rationalizations (Turpin et al., NeurIPS 2023; Lanham et al., Anthropic, 2023), necessitating a four-layer audit architecture separating facts from narrative. Third, documented government AI failures — Michigan's MiDAS (93% false positive rate), Arkansas's RUGs algithm, the Dutch childcare benefits scandal, Australia's Robodebt — establish that due process protections, appeal mechanisms, and continuation-of-service guarantees are non-negotiable.

# 1.2 Design Goals

1. **Human authority is supreme.** Agent recommendations MUST NOT be the sole factor in adverse decisions affecting individual rights.
2. **Structured oversight, not checkbox review.** Human oversight MUST produce genuine cognitive engagement via empirically grounded protocols.
3. **Accountability requires specificity.** Every action traceable to a specific actor, authority, inputs, outputs, and rule version.
4. **Constraints are external to the agent.** The agent is outside the trust boundary. Guardrails are enforced by the WOS Processor using a deontic framework. Safety-critical constraints are formally verifiable before deployment.
5. **Graceful degradation is mandatory.** Every workflow MUST function without any agent participation.
6. **Correctness is verifiable.** Workflow definitions are verifiable for soundness. SHACL shapes enforce policy-level governance. Critical deontic constraints are provable via SMT solving.
7. **Linked by construction.** Documents are natively JSON-LD, queryable as RDF graphs.
8. **Composable by design.** Reusable ActivityDefinitions are independently versionable. Adaptive case management phases use declarative constraint zones.
9. **Safely editable by AI.** Typed patch operations enable statically analyzable structural edits.
10. **One contract specification.** Formspec Definitions serve as the universal interface contract across all structured data exchanges.
11. **One expression language.** FEL is the single expression language across all contexts.
12. **Incrementally adoptable.** A minimal kernel is surrounded by progressively adoptable profiles.

### 1.3 Scope

**Within scope:** the kernel-and-profiles architecture; the tripartite object model; lifecycle semantics including declarative constraint zones; decision services with defeasible rules; human task lifecycle with structured oversight; agent governance with deontic constraints, verifiable constraint subset, and behavioral attestations; object-centric case state; event envelope format; four-layer provenance model with dual-readability narrative; durable execution guarantees with deterministic replay; typed patch operations; FEL expression language with three conformance profiles; Formspec integration as universal interface contract; Mapping DSL for data flow; Assist Governance Interface; Arazzo integration sequences; CWL-informed non-HTTP tool descriptors; Formspec provenance ingestion; instance migration contract; JSON-LD serialization with normative `@context`; SHACL shapes; due process requirements; conformance profiles; companion specification contracts.

**Out of scope:** UI rendering; persistence mechanisms; transport protocols beyond contracts; process mining algorithms; ML training or inference; document management; notification delivery mechanisms; general-purpose computation; form rendering, layout, or widget selection.

### 1.4 Relationship to Formspec

This specification is a **companion framework** to Formspec v1.0. Formspec governs the data-collection instrument (what data is collected, how it behaves reactively, how it is validated). WOS governs the orchestration envelope (who does the work, when, under what authority, with what agent assistance, with what oversight, and what gets recorded). The integration rests on Formspec's strict three-layer separation of Structure, Behavior, and Presentation. Because the Presentation layer is entirely optional, a Formspec Definition with only Items, Binds, and Shapes functions as a pure data contract — the "headless contract" pattern used for agent capability contracts, decision service interfaces, and integration response validation.

Both specifications share FEL as their expression language, the `x-` extension model, the three-severity validation model, version pinning and immutability rules, and thstatus lifecycle (draft → active → retired).

### 1.5 Relationship to Tier Specifications

This is the **Core Specification**. Planned Tier Specifications: WOS-Lifecycle (including consnt zone formal semantics and instance migration), WOS-Decision (including Rules-as-Code integration), WOS-Task (Formspec integration, `formRef`/`inputMapping`/`outputBinding`), WOS-Agent (attestation verification, drift monitoring), WOS-CaseState, WOS-Integration (Arazzo, CWL tools, federation profile), WOS-Provenance (Formspec ingestion, reporting metrics, RO-Crate packaging), WOS-Execution, WOS-SHACL (bidirectional FEL-SHACL equivalence), WOS-Authoring, WOS-Conformance.

### 1.6 Notational Conventions

Key words per BCP 14 [RFC2119] [RFC8174].

---

## 2. Conformance

### 2.1 Conformance Classes

**WOS Document.** A serialized workflow definition, activity definition, decision service definition, agent configuration, or interface contract conforming to this specification.

**WOS Processor.** A software system consuming WOS Documents and producing behavior consistent with this specification. MUST support at least one Conformance Profile (§21).

### 2.2 Document Conformance

1. MUST be serialized in JSON-LD [JSON-LD11] or YAML resolving to valid JSON-LD.
2. MUST include `@context` referencing the WOS context document.
3. MUST validate against the applicable JSON Schema.
4. UST satisfy static semantic constraints.
5. MUST define a fallback to human performance for every agent invocation.
6. SHOULD pass soundness verification (§6.9).
7. SHOULD pass SHACL strucural governance validation (Appendix B).

### 2.3 Processor Conformance

1. MUST accept conformant documents and reject structurally invalid documents with diagnostics.
2. MUST preserve `@context` during round-trips.
3. MUST execute kernel layer semantics.
4. MUST produce provenance records for every state transition, task operation, decision evaluation, agent invocation, and contract validation.
5. MUST enforce agent governance including deontic constraints, autonomy limits, and fallback chains.
6. MUST use recorded outputs during replay rather than re-invoking non-deterministic services (§13.1 G3).
7. MUST implement FEL Core (§15).
8. MUST support Formspec Definition references as interface contracts.

---

## 3. Terminology

This section is rmative.

**ActivityDefinition.** A standalone, independently versionable, reusable template for a unit of work. References Formspec Definitions for data contracts. Published as its own JSON-LD document.

**Actor.** An entity performing actions: `human`, `system`, or `agent`.

**Agent.** An AI system participating in a workflow. Outside the trust boundary.

**Assist Governance Interface.** The contract specifying how a WOS Processor wraps Formspec Assist tool invocations with deontic constraint enforcement.

**Attestation.** An independently issued, structured reference to an evaluation or certification verifying an agent's behavioral characteristics for a specific domain and autonomy level.

**Autonomy Level.** `autonomous`, `supervisory`, `assistive`, or `manual`.

**Case File.** The typed data container modeled as objects with temporal Event-to-Object relationships following OCEL 2.0.

**Constraint Zone.** A compound state type whose internal behavior is governed by declarative DCR-style relations between activities, rather than explicit transitions.

**Deontic Constraint.** A governance rule classified as a Permission, Prohibition, Obligation, or Right.

**Dual-Readability Narrative.** A deterministically generated human-readable summary of an adverse decision, derived from Layer 1 facts and Layer 2 structured reasoning, suitable for due process notices.

**Formspec Definition.** A JSON document conforming to Formspec v1.0 specifying a data-collection instrument or data contract via Items, Binds, and Shapes.

**Guard.** A boolean FEL expression controlling transition eligibility.

**Headless Contract.** A Formspec Definition used as a pure data contract with no presentation layer.

**Kernel Layer.** Required by every implementation: Lifecycle (1), Case State (5), Provenance (7), Execution (8).

**Mapping DSL Document.** A Formspec companion document defining bidirectional transforms between WOS case file data and Formspec instances.

**Obligation.** A deontic constraint requiring the agent to perform a specified action or include specified content.

**Patch Operation.** A typed edit against a WOS document's abstract syntax tree.

**Permission.** A deontic constraint bounding what the agent is allowed to produce.

**Profile Layer.** Adopted as needed: Decision (2), Task (3), Agent (4), Integration (6).

**Prohibition.** A deontic constraint forbidding specified agent outputs or actions.

**Provenance Record.** An immutable audit entry structured in a four-layer architecture.

**Right.** A deontic constraint specifying what the agent is entitled to receive as input.

**Structured Oversight.** Human review protocols producing genuine cognitive engagement: `independentFirst`, `considerOpposite`, `calibratedConfidence`, `dualBlind`, `unassisted`.

**Task.** A runtime instance of an ActivityDefinition.

**Verifiable Constraint Subset.** A decidable fragment of FEL for deontic constraints amenable to SMT-based formal verification before deployment.

**WorkflowDefinition.** A process topology referencing ActivityDefinitions, Decision Services, and Agent Configurations by URI.

---

## 4. Architecture Overview

This section is normative.

### 4.1 Constraint-Enhanced Layered Kernel

```
            ┌──────â────────────────────┐
            │        Profile Layers (adoptable)        │
            │  ┌──────────┐ ┌─â
            │  │ & Policy │ │  Task    │ │Governance│ │
            │  │ Layer 2  │ │ Layer 3  │ │ Layer 4  │ │
            │  └──────â          ┌──────────┐              │
            │                │Integrate │              │
            │                │ Layer 6  │            ────┘              │
            ├──────────────────────────────────────────┤
            │
            │  │Lifecycle │ │Case State│              │
            │  │ Layer 1  │ │ Layer 5  │              │
            │  └──────────┘ └──────────┘              │
            │  ┌──────────┐ ┌──────────┐              │
            │  │Provenance│ │ Durable  │              │
            │         │
            ├──────────────────────────────────────────┤
            │  Formspec Univer      ├──────────────────────────────────────────┤
            │      JSON-LD / RDF Graph Foundatio───────────────────────────────┘
```

**Kernel layers** (1, 5, 7, 8) define how state progresses, what data accumulates, what gets recorded, and what guarantees hold. Changes require majorneeded. May evolve via minor versions.

**Formspec and FEL** are the interface contract and expression language substrates shared by all layers.

### 4.2 Tripartite Object Model

**ActivityDefinition** — standalone, reusable, independently versionabl References Formspec Definitions for data contracts. Defines what the work entails.

**WorkflowDefinition** — process topology composing ActivityDefinitions. Defines when and how work is orchestrated.

**Task** (runtime) — instance of an ActivityDefinition carrying template plus workflow overrides plus case cot.

### 4.3 Universal Interface Contract Model

WOS uses Formspec Definitions as interface contracts in four contexts. Each uses the same Definition structure but enables different Formspec layers.

**Human task contracts** — the full Formspec stack. Mapping DSL documents formalize data flow. References, Ontology, Assist, and Respondent Ledger optionally available. Presentaon, Theme, and Component available to renderers but not consumed by the WOS Processor.

**Agent capability contracts** — heaess Formspec Definitions specifying agent input/output schemas. Binds and Shapes enforce constraints the WOS Processor validates before deontic constraint evaluation. References optionally provide agent-consumable grounding. Ontology provides semantic field identity. No Presentation, Theme, Component, or Assist.

**Decision service contracts** — headless Formspec Definitions for inputs and outputs. Enables independent verification of service output consistency.

**Integration contracts** — headlFormspec Definitions for request and response schemas. Responses validated before results committed to case file.

### 4.4 Separation Principles

Process topology MUST be separated from decision logic. Decision logic from task management. Agent governance from agent implementation. Case data from process state. Audit from execution. Execution guarantees from execution mechanisms. Syntax validation from semantic governance. Reusable templates from instantiation context. Interface contracts from the systems that produce and consume them.

---

## 5. Document Model, Serialization, and Interface Contracts

This section is normative.

### 5.1 Document Types

**WorkflowDefinition**, **ActivityDefinition**, **DecisionServiceDefinition**, **AgentConfiguration**. All share JSON-LD serialization, `@context`, and validation requirements.

### 5.2 WorkflowDefinition Structure

Contains `lifecycle` (REQUIRED), and references to `decisions`, `parameters`, `activities` (ActivityDefinition references with overrides), `agents` (AgentConfiguration references), `caseFile`, `integrations`, `provenance` config, `execution` config, `dueProcess`, and `extensions`.

### 5.3 ActivityDefinition Structure

Contains `form` (Formspec reference or inline schema), `assignment`, `sla`, `separation`, and `oversight`. The `form` property supports `formRef` (a Formspec Definition by `url|version` with `inputMapping` and `outputBinding` as Mapping DSL rules) and `formInline` (inline JSON Schema for backward compatibility).

### 5.4 JSON-LD Serialization

Canonical format is JSON-LD 1.1. Every document is simultaneously valid JSON, valid JSON-LD, and an RDF graph. Implementations ignoring `@context` lose no structural functionality.

### 5.5 The `@context` Document

Published at `https://wos-spec.org/context/6.0.0`. Composes with Formspec's context document via JSON-LD context arrays for unified graph querying. Maps WOS terms to PROV-O, Schema.org, LegalRuleML, and WOS namespace IRIs.

### 5.6 SHACL Governance Validation

Policy-level validation via SHACL shapes (Appendix B). Validates cross-cutting constraints: impact-level-to-autonomy relationships, guardrail completeness, oversight requirements, contract reference resolution, attestation presence for elevated autonomy, constraint zone relation validity, and due process configuration.

### 5.7 Property: `impactLevel`

| Value | Definition | Requirements |
|-------|-----------|-------------|
| `rights-impacting` | Decisions affect individual rights, benefits, services, obligations. | Full due process. Agent autonomy capped at `assistive` unless elevated. Dual-readability narrative required for adverse decisions. Verifiable constraint subset RECOMMENDED. |
| `safety-impacting` | Decisions affect individual or public safety. | Full due process. Autonomy capped at `assistive`. |
| `operational` | Organizational operations without direct individual impact. | Due process RECOMMENDED. |
| `informational` | Informational outputs, no binding decisions. | Due process OPTIONAL. |

---

## 6. Kernel Layer 1: Lifecycle and Topology

This section is normative. This is a kernel layer.

### 6.1 Overview

The Lifecycle layer defines the statechart governing workflow progression, based on Harel statecharts formalized in SCXML, extended with declarative constraint zones for adaptive case management.

### 6.2 States

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `type` | enum | REQUIRED | `atomic`, `compound`, `parallel`, `constraintZone`, or `final`. |
| `onEntry` | array of Action | OPTIONAL | Actions on entry. |
| `onExit` | array of Action | OPTIONAL | Actions on exit. |
| `transitions` | array of Transition | OPTIONAL | Outgoing transitions. |
| `historyState` | enum | OPTIONAL | `shallow`, `deep`. Compound only. |
| `initialState` | string | CONDITIONAL | Required for `compound`. |
| `regions` | map of Region | CONDITIONAL | Required for `parallel`. |
| `activities` | array of ZoneActivity | CONDITIONAL | Required for `constraintZone`. See §6.10. |
| `relations`| array of ZoneRelation | CONDITIONAL | Required for `constraintZone`. See §6.10. |

### 6.3 Transitions

| Property | Type | Required | Description |
|----------|------|----------|------------|
| `event` | string | REQUIRED | Triggering event. |
| `target` | string | REQUIRED | Target state. |
| `guard` | FEL expression | OPTIONAL | Must evaluate `true` to fire. |
| `actions` | array of Action | OPTIONAL | Transition actions. |
| `priority` | integer | OPTIONAL | Lower = higher priority. Default: 0. |

### 6.4 Actions

`createTask`, `invokeDecision`, `invokeAgent`, `invokeIntegration`, `setData`, `emitEvent`, `startTimer`, `cancelTimer`, `compensate`, `log`, `notify`. The `createTask` action uses `activityRef` referencing an ActivityDefinition by URI.

### 6.5 Events

Internal: `task.completed`, `task.failed`, `task.escalated`, `timer.expired`, `regions.allFinal`, `constraintZone.satisfied`, `error`, `milestone.achieved`, `decision.complete`, `agent.complete`, `agent.failed`, `guardrail.violated`, `contract.validated`, `integration.complete`. External: matched via correlation.

### 6.6 Milestones

Declarative checkpoints re-evaluated via the processing model (§6.8).

### 6.7 Soundness Verification

**Deadlock-freedom**, **livelock-freedom**, **proper termination**, **no dead elements**, **allback completeness**, **constraint zone satisfiability** (every constraint zone has at least one execution sequence that satisfies all response obligations).

### 6.8 Processing Model for Guard and Milestone Evaluation

WOS defines a normative processing model adapted from Formspec's four-phase Rebuild → Recalculate → Revalidate → Notify cycle, applied to workflow topology.

**Phase 1: Rebuild.** When case file structure changes,struct the dependency graph for all guard expressions, milestone conditions, constraint zone relations, and computed values. The graph is a DAG; cycles are definition errors.

**Phase 2: Recalculate.** When case file data changes, identify the affected subgraph, topologically sort, and re-evaluate only affected expressions. Minimal recalculation guarantee: expressions outside the affected set MUST NOT be re-evaluated.

**Phase 3: Re-evaluate.** Transition guards, milestones, and constraint zone activity availability are re-assessed.

**Phase 4: Notify.** The Processor signals state changes to observers.

### 6.9 Declarative Constraint Zones

A **constraint zone** is a compound state type whose internal behavior is governed by declarative relations between activities, rather than explicit transitions. Constraint zones enable adaptive case management phases where the valid next actions are not predetermined — an investigator may interview witnesses, request documents, consult experts, or issueubpoenas in any order, subject to constraints.

The constraint zone model is adapted from DCR Graphs (Dynamic Condition Response), proven at government scale by Danish central government deployment (65-70% institutional adoption via KMD WorkZone). WOS adapts the five DCR relation types for its existing architecture:

#### 6.9.1 Zone Activities

Each activity within a constraint zone references an ActivityDefinition and tracks three-state markings:

| Marking | Description |
|---------|-------------|
| `included` | The activity is currently available for execution. |
| `executed` | The activity has been executed at least once in the current zone activation. |
| `pending` | The activity must eventually be executed before the zone can complete (a response obligation is outstanding). |

Initial markings are declared in the zone definition. Activities may start as included or excluded, and as pending or not pending.

#### 6.9.2 Zone Relations

Five relation types govern activity dependencies within a constraint zone:

| Relation | Semantics |
|----------|-----------|
| `condition` | Activity B cannot execute until activity A has been executed. A must have `executed = true` for B to be available. |
| `response` | When activity A executes, activity B becomes pending. B must eventually execute before the zone can complete. |
| `include` | When activity A executes, activity B becomes included (available). |
| `exclude` | When activity A executes, activity B becomes excluded (unavailable). |
| `milestone` | Activity B can only execute while the milestone condition on A holds (A is `executed` and `included`). |

```yaml
constraintZoneExample:
  type: "constraintZone"

  activities:
    - activityRef: "urn:wos:example.gov:activities:witness-interview:1.0.0"
      id: "witnessInterview"
      initialIncluded: true
      initialPending: false

    - activityRef: "urn:wos:example.gov:activities:document-request:1.0.0"
      id: "documentRequest"
      initialIncluded: true
      initialPending: false

    - activityRef: "urn:wos:example.gov:activities:expert-consultation:1.0.0"
      id: "expertConsult"
      initialIncluded: false
      initialPending: false

    - activityRef: "urn:wos:example.gov:activities:final-report:1.0.0"
      id: "finalReport"
      initialIncluded: true
      initialPending: true    # Must execute before zone completes

  relations:
    - type: "condition"
      from: "witnessInterview"
      to: "finalReport"        # Cannot write report until at least one interview

    - type: "response"
      from: "witnessInterview"
      to: "documentRequest"    # Every interview triggers a document request obligation

    - type: "include"
      from: "documentRequest"
      to: "expertConsult"      # Document requests may reveal need for expert

    - type: "exclude"
      from: "finalReport"
      to: "witnessInterview"   # No more interviews after report is filed

  transitions:
    - event: "constraintZone.satisfied"
      target: "supervisorReview"
```

#### 6.9.3 Zone Completion

A constraint zone is satisfied (raises `constraintZone.satisfied`) when all of the following hold: every activity with `pending = true` has been executed, and no outstanding response obligations remain unsatisfied. The zone's completion triggers outgoing transitions.

#### 6.9.4 Zone Provenance

Every activity execution within a constraint zone produces standard task provenance records. Additionally, each relation evaluation (a condition being checked, a response obligation being created, an include/exclude being applied) produces a provenance record capturing the relation type, the source and target activities, and the resulting marking changes.

The formal semantics of constraint zones, including the marking evaluation algorithm and the interaction with the DAG-based processing model, are defined in Appendix G and elaborated in the WOS-Lifecycle Tier Specification.

---

## 7. Profile Layer 2: Decision and Policy

This section is normative. This is a profile layer.

### 7.1 Decision Services

Encapsulated logic with Formspec Definition references for input and output contracts. Decision service output is validated against the output contract Formspec Definition (headless contract validation). The resulting ValidationReport is ingested as a provenance record. If the output has error-severity results, the output is rejected and the fallback chain is activated.

### 7.2 Decision Logic Types

**Decision Tables** with hit policies. **Expression Logic** using FEL. **Decision Requirement Graphs.** **Defeasible Rules** with override relationships forming a DAG. **External Decision Reference** including Rules-as-Code engines (the WOS-Decision Tier Specification defines bindings for OpenFisca, Rune DSL, and similar platforms).

### 7.3 Temporal Parameters

Values changing on specific dates. Effective value on reference date; no value raises error. Business calendar resolution delegated to the companion Business Calendar specification (§22.2).

---

## 8. Profile Layer 3: Human Task Management

This section is normative. This is a profile layer.

### 8.1 Task Lifecycle

Tasks are runtime instances of ActivityDefinitions. Terminal: `Completed`, `Failed`,`Cancelled`. Operations: `create`, `claim`, `release`, `start`, `complete`, `fail`, `delegate`, `forward`, `returnForRework`, `escalate`, `suspend`, `resume`, `cancel`.

### 8.2 Formspec-Driven Task Creation

When a Task is created from an ActivityDefinition with `formRef`, the Processor resolves the Definition, executes `inputMapping` via the Mapping DSL, loads associated sidecars (References, Ontology, Registry — not Theme or Component), presents the form, enforces the Assist Governance Interface for agent-assisted fields, and on completion executes `outputBinding` to write Response data to the case file. The ValidationReport and Respondent Ledger events are ingested as provenanceecords.

### 8.3 Structured Oversight Protocols

REQUIRED when a task involves agent assistance. `independentFirst`, `considerOpposite`, `calibratedConfidence`, `dualBlind`, `unassisted`. When `independentFirst` is specified with `formRef` and agent assistance, the Assist Governance Interface suppresses agent-generated content until the reviewer's independent assessment is recorded.

### 8.4 Override Authority

Overrides require structured rationale, authority verification, and supporting evidence.

---

## 9. Profile Layer 4: Agent Governance

This section is normative. This is a profile layer.

### 9.1 Overview

The governance envelope is enforced by the WOS Processor. The agent is outside the trust boundary.

### 9.2 Agent Configuration with Formspec Capability Contracts

Agent Configurations specify capabilities with Formspec Definition references for input and output contracts, preconditions (FEL expressions evaluated against case file data), and optional behavioral attestations.

### 9.3 Autonomy Levels

| Level | Semantics |
|-------|-----------|
| `autonomous` | Output committed without human review. REQUIRES deontic constraints. PROHIBITED for `rights-impacting`/`safety-impacting` unless elevated with attestation. |
| `supervisory` | Provisionally committed. Human reviews within `reviewWindow`. |
| `assistive` | Recommendation only. Human reviews, modifies, confirms. |
| `manual` | Human performs. Agent assists on demand only. |

### 9.4 Deontic Constraint Framework

**Permissions** — what the agent is allowed to do, within bounds. **Prohibitions** — what the agent must not do, regardless of confid. **Obligations** — what the agent must do. **Rights** — what the agent is entitled to receive as input.

Deontic constraints are evaluated after Formspec contract validation and before commit. The WOS Processor is the Policy Enforcement Point (PEPhe deontic constraint definitions are the Policy Decision Point (PDP). SHACL equivalence: every deontic constraint has an equivalent SHACL shape.

### 9.5 Verifiable Constraint Subset

Deontic constraints written within the **Verifiable Constraint Subset** are amenable to formal analysis. A WOS Processor or external verification tool can prove properties about the constraint set before the workflow is deployed — for example, "no agent can approve a benefit above $X without human review" or "agents with confidence below threshold T always escalate" — across possible scenarios, not just tested ones.

The verifiable subset restricts FEL expressions to a decidable fragment: no unbounded recursion, bounded quantification (finite domain enumeration), finite domains for enumerated types, and arithmetic limited to linear inequalities. These restrictions preserve expressiveness for the most common governance patterns while enabling translation to SMT formulas for sound and complete analysis.

Constraints outside the verifiable subset continue to function as runtime-evaluated FEL expressions with the existing enforcement mechanism. The verification tier is optional — it does not change how constraints are enforced, only whether they can be prov correct in advance.

The verifiable constraint subset is defined in Appendix F. The Verified Governance conformance profile (§21.10) requires support for formal verification of constraints within the subset.

### 9.6 Behavioral Attestations

Agent Cofigurations MAY include `attestations` — structured references to independent evaluations or certifications verifying the agent's behavioral characteristics for a specific domain and automy level.

```yaml
attestations:
  - issuer:
      name: "Federal AI Evaluation Board"
      url: "https://ai-eval.gov"
    subject: "urn:wos:example.gov:agents:eligibility-screener:3.0.0"
    claims:
      evaluatedAutonomy: "assistive"
      domain: "benefits-eligibility"
      evaluationMethodology: "AIA-Level-III"
      accuracy: 0.97
      calibration: "verified"
    issued: "2026-03-15T00:00:00Z"
    expires: "2026-09-15T00:00:00Z"
    verificationMethod: "https://ai-eval.gov/certs/ES-2026-0412"
```

Attestations are not self-issued by the agent or deploying organization — they reference independent evaluations (coormity assessments, algorithmic impact assessments, third-party audits). The WOS Processor records attestation references in the provenance stream and validates that the claimed autonomy level does not exceed what the attestation supports.

For `rights-impacting` workflows, agents operating above `manual` autonomy SHOULD present at least one valid, non-expired attestation. The SHACL governance shapes (Appendix B) validate this requirement.

### 9.7 Confidence Framework

ConfidenceReport with `overall`, `method`, `calibrationStatus`, optional `fieldLevel`. Expired calibration caps autonomy at `assistive`.

### 9.8 Fallback Chains

MUST terminate in `escalateToHuman` or `fail`. MUST NOT cycle.

### 9.9 Input Preparation and Isolation

`sanitize`, `maxInputTokens`, `redactFields`/`includeFields`, `isolateUntrustedData` (CaMeL dual-LLM architecture).

### 9.10 Assist Governance Interface

When an AI agent assists a human reviewer during a Formspec-driven task, the Assist Governance Interface sits between the agent consumer and the Formspec Assist Provider, intercepting every tool invocation and applying the WOS governance envelope.

**Introspection tools** — passed through. `FieldHelp` recorded in provenance. `independentFirst` suppresses `summary` until independent assessment recorded.

**Mutation too** — intercepted. Permission check, Prohibition check, Obligation check, confirmation enforcement.

**Profile tools** — intercepted. `matches` filtered against Permission scope and Prohibition patterns.

**Validation and navigation tools** — passough.

The normative contract is defined in Appendix E.

---

## 10. Kernel Layer 5: Case State and Evidence

This section is normative. This is a kernel layer.

Object-centric case model following OCEL 2.0. Every mutation recorded as immutable E2O provenance event. Evidence via claim check pattern with trust labeling (verified, untrusted, agent-generated). Selective visibility via role-based field-level access.

---

## 11. Profile Layer 6: Integration and Eventing

This section is normative. This is a profile layer.

### 11.1 Integration Types

Six types: `request-response` (OpenAPI), `event-emit`, `event-consume`, `callback`, `arazzo-sequence` (multi-step API orchestration), and `tool` (non-HTTP invocations: command-line, batch, database procedure, graph query). All support Formspec contract references for request and response validation.

### 11.2 Event Envelope

CloudEvents 1.0 with WOS extensions.

### 11.3 Protocol Alignment

**MCP** for agent-tool integration within governance envelope. **A2A** for inter-agent communication. Extension points for emerging protocols.

---

## 12. Kernel Layer 7: Provenance and Audit

This section is normative. This is a kernel layer.

### 12.1 Four-Layer Audit Architecture

| Layer | Name | Content | Authority |
|-------|------|---------|-----------|
| 1 | Immutable Facts | Timestamp, actor, model version, inputs, outputs, policy version, confidence, reviewer ID, attestation references. | **Authoritative.** |
| 2 | Structured Reasoning | Rules applied, evidence consulted, criteria checked, decision table trace. Formspec ValidationResult entries. For adverse decisions in `rights-impacting` workflows: **dual-readability narrative** (see §12.7). | **Authoritative** for deterministic logic. |
| 3 | Generated Narrative | Model's natural language explanation. | **Informational only.** Non-authoritative. |
| 4 | Counterfactual | What would change the outcome. Positive and neative. | Informational. **Required** for adverse decisions in `rights-impacting` workflows. |

### 12.2 PROV-AGENT Alignment

Provenance records are JSON-LD aligned with PROV-O and PROV-AGENT.

### 12.3 Object-Centric Event Logging

OCEL 2.0 Event-to-Object and Object-to-Object mapping.

### 12.4 Record Types

`transition`, `decision`, `agentInvocation`, `agentCheckpoint`, `agentToolUse`, `taskOperation`, `dataMutation`, `override`, `guardrailViolation`, `guardrailBypass`, `autonomyChange`, `modelVersionChange`, `driftAlert`, `dueProcessNotice`, `appealFiled`, `patchApplied`, `contractValidation`, `formEvent`, `assistInvocation`, `constraintZoneRelation`, `attestationVerified`.

### 12.5 Tamper Evidence

Merkle tree hash-chaining with SHA-256, signed tree heads, inclusion and consistency proofs.

### 12.6 Formspec Provenance Integration

**ValidationReport → Layer 1 + Layer 2.** Every Formspec contract validation — human task, agent output, decision service, integration response — pr a provenance record with `valid` flag, `counts`, and individual ValidationResult entries.

**Respondent Ledger events → Lay 1.** Material events at lifecycle boundaries.

**Agent Assist events → Layer 1 + Layer 3.** Tool invocations via Governance Interface.

**Tamper evidence continuity.** Cross-reference Ledger checkpoint hashes in WOS provenance.

### 12.7 Dual-Readabity Narrative

For `rights-impacting` workflows, provenance records for adverse decisions MUST include a `narrative` property containing a human-readable summary. The narrative MUST be derived deterministically from Layer 1 facts and Layer 2 structured reasoning. It MUST NOT be generated by an AI agent. It MUST produce identical output given identical Layer 1 and Layer 2 inputs. The narrative MUST be suitable for inclusion in due process notices (§16.1).

The narrative generation algorithm is implementation-defined, but the following elements MUST be present: the specific determination, the factual basis with cited reguatory provisions, the evidence items considered, the agent involvement (if any) with confidence levels, and the appeal rights with deadline.

### 12.8 Process Mining Interoperability

Primary: OCEL 2.0. Secondary: IEEE XES.

### 12.9 Provenance Export Packaging

RO-Crate with Workflow Run Crate profile.

---

## 13. Kernel Layer 8: Durable Execution Contract

This section is normative. This is a kernel layer.

**G1: Crash Recovery.** **G2: Persistent State.** **G3: Deterministic Replay** — re-invocation of a non-deterministic service during replay is a conformance violation. **G4: Durable Timers.** **G5: External Signal Delivery.**

Retry policy with idempotency keys. Compensation wh reverse completion order.

---

## 14. Actor Model

This section is normative.

| Type | Provenance |
|------|-----------|
| `human` | Identity, role, timestamp. |
| `system` | Component ID, version, timestamp. |
| `agent` | Model ID, version, confidence, attestation references, all PROV-AGENT fields. |

Agents MUST NOT override human decisions. Cascading autonomous agents require declaration with bounded depth.

---

## 15. Expression Language

This section is normative.

### 15.1 Overview

WOS uses FEL as its single expression language. The normative PEG grammar, type system, operators, built-in functions, null propagation rules, and dependency tracking algorithm are defined in the FEL Normative Grammar and incorporated by reference.

### 15.2 Conformance Profiles

**FEL Core** — FEL v1.0. All ocessors MUST implement.

**FEL Decision** — Core plus quantified expressions (`some`/`every`/`satisfies`), range literals (`$x in [0..32760]`), duration arithmetic (`duration('P30D')`). quired for Decision profile.

**FEL Extended** — Decision plus filter expressions (`$list[$.field > 5]`). Recommended for Agent Governance profile.

Grammar additions in Appendix C. FEEL anslation table in Appendix D.

### 15.3 Null Propagation

Guard: `false`. Milestone: `false`. Decision input: propagates. Deontic constraint: `true` (passes). Contract validation: per FEL §3.8.

---

## 16. Due Process Requirements

This section is normative for `rights-impacting` and `safety-impacting` workflows.

### 16.1 Notice

Adverse decisions require notice before efect: the specific determination, the factual basis with individualized reason codes, appeal rights and deadline, agent disclosure. The notice SHOULD be derived from the provenance record's dual-readability narrative (§12.7) when available.

### 16.2 Eplanation Levels

`individualized` (REQUIRED for `rights-impacting`), `categorical`, `aggregate`. Counterfactuals required: positive and negative.

### 16.3 Appeal Mechanisms

Human adjudicator independent of original determination. Agents MUST NOT decide appeals. Continuation of services during appeal.

### 16.4 Agent Disclosure

REQUIRED for `rights-impacting`.

### 16.5 Continuation-of-Service States

Workflow MUST include topology freezing adverse impacts during appeal.

---

## 17. AI-Native Authoring and Patch Operations

This section is normative.

Typed patch operations against the document's AST. Four-stage validation pipeline: JSON Schema → SHACL → soundness (including constraint zone satisfiability) → provenance. Compositional authoring via ActivityDefinition and Formspec Definition registry queries. Operation types in Appendix H.

---

## 18. Versioninlution, and Instance Migration

This section is normative.

### 18.1 Versioning Model

All publishable documents use Semantic Versioning independently. The `@context` is versioned with the spec.

### 18.2 Instance Migration Contract

When a regulation changes and the workflow definition must be updated, in-flight instances face a migration decision. WOS defines the migration contract:

**Migration assessment.** The WOS Processor evaluates the impact of the new version on each in-flight instance using Formspec Changelog-style impact classification: `patch` (cosmetic, no migration needed), `minor` (compatible additions, mapping review needed), `major` (breaking changes, explicit migration required).

**State mapping.** A migration descriptor defines how states in the old version map to states in the new version. States that exist in both versions map directly. States that exist only in the old version require explicit mapping to the nearest equivalent state in the new version. States with no mapping produce a migration error.

**In-flight task handling.** A task in progress at the time of migration completes under the version-in-effect at task creation (the Formspec Pinning Rule applies to the task's form contract). After task completion, the workflow advances under the new version's topology.

**Case file transformation.** When the new version adds required case file fields, the migration descriptor specifies default values or Mapping DSL transform expressions. When the new version removes fields, data is preserved in `extensions` for auditability.

**Migration provenance.** Every instance migration produces a `response.migrated` provenance record capturing the source version, target version, state mapping applied, case file transformations performed, and the migration descriptor used.

**Policy-based routing.** The workflow definition MAY specify a `migrationPolicy` controlling which in-flight instances are migrated and which are grandfathered: `grandfather` (all in-flight instances continue under old version), `migrateAll` (all instances migrate), `migrateByState` (instances in specified states migrate, others continue), or `expression` (a FEL expression evaluated per-instance).

---

## 19. Security and Access Control

This section is normative.

Roles: `workflowAdministrator`, `instanceInitiator`, `caseParticipant`, `taskWorker`, `taskAdministrator`, `auditor`. Authorization enforced at all access points. Failures logged.

---

## 20. Extensibility

This section is normative.

Namespaced `x-` properties. MUST NOT alter core semantics. MUST be preserved during round-trips. WOS and Formspec share identical extension model semantics.

---

## 21. Conformance Profiles

This section is normative.

### 21.1 Profile: Structural

Parse, validate, round-trip, resolve Formspec Definition references.

### 21.2 Profile: Kernel

Structural + kernel layer semantics. DAG-based processing model. Deterministic replay. FEL Core.

### 21.3 Profile: Task Management

Kernel + full task lifecycle. `formRef` support. Mapping DSL Core. ValidationReport ingestion. Respondent Ledger ingestion RECOMMENDED.

### 21.4 Profile: Decision

Kernel + decision tables. FEL Decision. Temporal parameters with business calendar delegation. Output contract validation.

### 21.5 Profile: Agent Governance

Kernel + Decision + deontic constraints, autonomy enforcement, fallback chains, capability contract validation (Formspec pre/post), confidence routing, multi-step sessions. Assist Governance Interface REQUIRED when combined with Task Management and `formRef` with agent assistance. FEL Extended RECOMMENDED.

### 21.6 Profile: Full

All profiles + integration (including `arazzo-sequence` and `tool`), all provenance types, tamper evidence, access control, due process, OCEL 2.0, dual-readability narrative for rights-impacting adverse decisions.

### 21.7 Profile: Verification

Structural + soundness analysis (including constraint zone satisfiability), PatchSet validation, contract reference resolution.

### 21.8 Profile: Semantic

Structural + JSON-LD with composed WOS/Formspec contexts, SHACL validation, PROV-O/PROV-AGENT graphs, SPARQL querying, OCEL 2.0 emission.

### 21.9 Profile: Authoring

Structural + Verification + PatchSet acceptance, four-stage pipeline, compositional authoring.

### 21.10 Profile: Verified Governance

Agent Governance + formal verification of deontic constraints within the Verifiable Constraint Subset (Appendix F). Verification tool MUST accept constraints in the subset and produce a verification report: proven safe, proven unsafe (with counterexample), or inconclusive. Proven-safe constraints are annotated in the provenance stream. Inconclusive results are warnings.

---

## 22. Companion Specification Contracts

This section is normative. It defines the integration contracts for companion specifications that WOS references but does not itself define. Each contract specifies the interface that the companion specification MUST satisfy for WOS integration.

### 22.1 Formspec Integration Contract

**Already defined.** Formspec v1.0 and its companion specifications (Mapping DSL, Assist, Ontology, References, Respondent Ledger, Registry, Changelog) satisfy the universal interface contract requirements defined in §4.3 and §5.

### 22.2 Business Calendar Contract

WOS references `businessCalendar` in SLA configurations and FEL Decision's `duration()` function supports business-day/business-hour suffixes. The companion Business Calendar specificaon MUST define:

- A calendar document format specifying holidays, working hours, and regional variants as a JSON-LD document.
- A composition model where a jurisdiction-specific calendar inherits from a parent calendar and adds local holidays.
- A versioning model where calendars are version-pinned (in-flight cases use the calendar effective at the triggering event's timestamp).
- An integration interface that the FEL evaluator calls to resolve `BD`/`BH` duration suffixes against the applicable calendar.

Until the Business Calendar specification is published, WOS Processors MUST support at minimum a `federalWorkdays` calendar excluding US federal holidays and weekends. Additional calendars are implementation-defined.

### 22.3 Notification Contract

WOS's `notify` action and §16.1 (due process notices) require legally compliant notification delivery. The companion Notification specification MUST define:

- A template format for omposing notification content from case file data, provenance records, and the dual-readability narrative.
- Delivery channel selection based on recipient preferences and statutory requirements.
- Delivery confirmation tracking recorded in the provenance stream.
- Content hash linkage between the notification and the provenance record, creating a tamper-evident chain.

Until the Notification specification is published, the `notify` action's implementation is entirely implementation-defined. WOS Processors MUST record the notification intent as a provenance record regardless of delivery mechanism.

### 22.4 Document Generation Contract

Government workflows produce official documents (grant awards, denial notices, license certificates, inspection reports). The companion Document Generation specification MUST define:

- Template format for composing document content from case file data and provenance records.
- Content validation ensuring generated documents satisfy regulatory content requirements.
- Document versioning alongside the workflow definition.
- Storage as evidence in the case file with content hashes and retention metadata.

Until the Document Generation specification is published, document generation is implementation-defined.

### 22.5 Simulation and Testing Contract

WOS's soundness verification (§6.7) addresses static analysis; simulation addresses dynamic testing with realistic data. The companion Simulation secification MUST define:

- A test fixture format specifying case scenarios with expected outcomes at each workflow step.
- A simulation execution model where the Processor runs scenarios and compares actual to expected outcomes.
- A reporting format identifying passed, failed, and inconclusive scenarios with diagnostic detail.

Until the Simulation specification is published, testing is implementation-defined.

---

## 23. Privacy Considerations

This section is informative.

Selective visibility, agent data redaction, trust labeling, claim check pattern. Formspec Respondent Ledger events may contain personal data; implementations SHOULD support configurable retention and anonymization. Attestation references SHOULD NOT contain personal information about model developers or evaluators. The dual-readability narrative for adverse decisions may contain personal information about the affected individual; implementations MUST apply the same access controls to narratives as to case file data.

---

## 24. Security Considerations

This section is informative.

Expression sandboxing (FEL: no side effects). Event authentication. Provenance integrity (independent signed tree heads). Encryption. Separation of duties. Agent impersonation prevention. Prompt injection defense (CaMeL pattern, deontic constraints as structural defense, defense in depth). Model version drift (shadow deployment). Cascading autonomy (declared and bounded). Tool use (least-privilege). `@context` integrity (HTTPS, caching, hash verification for WOS and Formspec contexts). Patch security (full validation pipeline, elevated auth for weakening constraints). Assist Governance (agents MUST NOT bypass the interface). Contract integrity (trusted sources). Sidecar provenance (verify document provenance). Constraint zone relation integrity (relations MUST NOT be modifiable by agents). Attestation verification (implementations SHOULD verify attestation issuer identity and expiry).

---

## 25. References

### 25.1 Normative References

**[RFC2119]**, **[RFC3339]**, **[RFC3986]**, **[RFC8174]**, **[RFC8259]**, **[YAML]**, **[JSON-LD11]**, **[RDF11]**, **[SHACL]**, **[PROV-O]**, **[PROV-DM]**, **[SemVer]**, **[ISO8601]**, **[CloudEvents]**, **[TraceContext]**, **[DMN]**, **[OpenAPI]**, **[AsyncAPI]**, **[JSONSchema]**, **[LegalRuleML]** — as in prior versions.

**[Formspec]** "Formspec v1.0 — A JSON-Native Declarative Form Standard", 2025.

**[FEL]** "Formspec Expression Language — Normative Grammar", Version 1.0.

**pecMappingDSL]** "Formspec Mapping DSL v1.0", 2025.

**[FormspecAssist]** "Formspec Assist Specification v1.0", 2026.

**[FormspecOntology]** "Formspec Ontology Specification v1.0", 2026.

**[FormspecReferences]** "Formspec References Specification v1.0", 2026.

**[FormspecLedger]** "Respondent Ledger Add-On Specification v0.1", 2026.

**[FormspecRegistry]** "Formspec Extension Registry v1.0", 2025.

**[FormspecChangelog]** "Formspec Changelog Format v1.0", 2025.

**[Arazzo]** OpenAPI Initiative, "Arazzo Specification", 2024.

**[DCR]** Hildebrandt, T. and Mukkamala, R., "Declarative Event-Based Workflow as Distributed Dynamic Condition Response Graphs", 2011.

### 25.2 Informative References

**[Harel1987]**, **[SCXML]**, **[BPMN]**, **[CMMN]**, **[WS-HumanTask]**, **[XACML]**, **[Sagas]**, **[WorkflowPatterns]**, **[OCEL2]**, **[XES]**, **[RO-Crate]**, **[WfRunCrate]**, **[PROV-AGENT]**, **[RFC9162]**, **[NIST-SP-800-53]**, **[OMB-M-24-10]**, **[EU-AI-Act]**, **[NIST-AI-RMF]**, **[CanadaDirective]**, **[ISO42001]**, **[Vaccaro2024]**, **[Turpin2023]**, **[Lanham2023]**, **[Chen2023]**, **[Buçinca2021]**, **[Nasr2025]**, **[CaMeL]**, **[FormalLLM]**, **[ABC]**, **[Feng2025]**, **[Wachter2018]**, **[MCP]**, **[A2A]**, **[BBO]**, **[FHIR-Wokflow]**, **[OASF]**, **[SchemaActions]** — as in prior versions.

**[CWL]** "Common Workflow Language Specification v1.2", 2023.

**[Cedar]** Bak, J. et al., "Cedar: A New Language for Expressive, Fast, Safe, and Analyzable Authorization", OOPSLA 20.

**[OpenFisca]** "OpenFisca: Rules as Code Platform", 2011-present.

**[ISDA-CDM]** "ISDA Common Domain Model and Digital Regulatory Reporting".

**[XRoad]** "X-Road: Open-Source Data Exchange Layer", Estonian Information System Authority.

**[DCRSolutions]** "DCR Solutions: Declarative Process Management for Danish Government", Exformatics.

---

## Appendix A. JSON-LD Context Document

Normative. Published at `https://wos-spec.org/context/6.0.0`. Composes with Formspec's context. Extends v5 context with constraint zone terms, attestation terms, and verifiable constraint annotations.

---

## Appendix B. SHACL Shapes for Structural Governance

Normative. Includes all v5 shapes plus:

- **Constraint zone satisfiability** — every constraint zone must have at least one valid execution sequence.
- **Attestation requirement** — agents above `manual` autonomy in `rightpacting` workflows SHOULD have a valid attestation.
- **Dual-readability narrative** — adverse decision provenance records in `rights-impacting` workflows MUST include `narrative`.
- **Vefiable constraint annotation** — constraints within the verifiable subset SHOULD be annotated with their verification status.

---

## Appendix C. FEL Conformance Profiles and Grammar Addions

Normative. FEL Core (v1.0 baseline), FEL Decision (quantified expressions, range literals, duration type), FEL Extended (filter expressions). Grammar additions as defined in v5.

---

## Appendix D. FEEL-to-FEL Translation Table

Non-normative. Mechanical bidirectional translation as defined in v5.

---

## Appendix E. Assist Governance Interface

Normative. Tool-level governance rules as defined in v5, extended with attestation-awareness: the interface validates that the assisting agent's attestation covers the domain and autonomy level required by the ActivityDefinition before any tool invocations are forwarded.

---

## Appendix F. Verifiable Constraint Subset

Normative. Defines the decidable fragment of FEL for deontic constraints amenable to SMT-based formal verification.

### F.1 Restrictions

Constraints within the verifiable subset MUST satisfy:

1. **No unbounded recursion.** `let` bindings are permitted but MUST NOT create recursive definitions.
2. **Bounded quantification.** `some` and `every` expressions MUST quantify over finite, enumerable domains — case file arrays with declared `maxRepeat`, option sets with declared options, or literal arrays.
3. **Linear arithmetic.** Arithmetic expressions MUST be linear: no multiication of two variables, no division by a variable, no exponentiation. Constants and single-variable multiplication (`$x * 3`) are permitted.
4. **Finite domain enumerations.** Equality comparisons MUST reference types with finite domains (booleans, choice fields with declared options, enumerated strings).
5. **No external function calls.** Only built-in FEL functions from the Core and Decision profiles. No extension functions, no `@instance()` references.
6. **No filter expressions.** The `[Expression]` filter from FEL Extended is excluded.

### F.2 Verification Interface

A WOS Processor or external tool that verifies constraints within the subset MUST:

1. Accept a set of deontic constraints annotated as `verifiable: true`.
2. Translate each constraint to an SMT formula (or equivalent decidable formalism).
3. Produce a verification report for each constraint: `proven-safe` (the constraint holds for all possible inputs within the declared domains), `proven-unsafe` (a counterexample exists, included in the report), or `inconclusive` (the solver timed out or the constraint uses constructs outside the verifiable subset).
4. Record the verification report as a provenance record with `recordType: "constraintVerification"`.

### F.3 Relationship to Runtime Enforcement

Verification is complementary to runtime enforcement, not a replacement. A constraint proven safe still executes at runtime (defense in depth). A constraint proven unsafe is a definition error that MUST be corrected before the workflow reaches `active` status. An inconclusive result is a warning that SHOULD trigger additional runtime monitoring.

---

## Appendix G. Constraint Zone Semantics

Normative. Defines the formal evaluation algorithm for constraint zones.

### G.1 Marking State

Each activity in a constraint zone maintains a triple `(included, executed, pending)`:

- `included` (boolean) — the activity is currently available.
- `executed` (boolean) — the activity has been executed at least once.
- `peg` (boolean) — the activity has an outstanding obligation to execute.

### G.2 Activity Availability

An activity is **available** (can be executed) when: it is `included`, and all `condion` relations targeting it are satisfied (every source activity of a condition relation has `executed = true`), and all `milestone` relations targeting it are satisfied (every source activity of a milestone relation has `executed = true` and `included = true`).

### G.3 Relation Evaluation on Activity Execution

When activity A executes:

1. Set A.`executed` = true.
2. For each `response` relation from A to B: set B.`pending` = true.
3. For each `include` relation from A to B: set B.`included` = true.
4. For each `exclude` relation from A to B: set B.`included` = false. If B was `pending`, the obligation transfers to a resolution error (the zone cannot complete until the conflict is resolved — either re-include B and execute it, or remove the response obligation via another relation).
5. Re-evaluate availability for all activities.

### G.4 Zone Completion

A constrnt zone is **satisfied** when: for every activity where `pending = true`, `executed = true`; and no included activity has `pending = true` and `executed = false`.

### G.5 Integration with Processing Model

Constraint zone evaluation participates in the DAG-based processing model (§6.8). When an activity within azone executes, the marking changes trigger Phase 2 (Recalculate) for any guard conditions or milestone expressions that reference zone activity state. Phase 3 (Re-evaluate) updates activity availability and checks zone completion. Phase 4 (Notify) signals which activities became available or unavailable.

---

## Appendix H. Patch Operation Reference

Normative. All v5 operations plus: `insertConstraintZoneActivity`, `addConstraintZoneRelation`, `removeConstraintZoneRelation`, `addAttestation`, `setVerifiable` (annotate a deontic constraint as within the verifiable subset), `setMigrationPolicy`.

---

## Appendix I. Relationship to Existing Standards

All v5 relationships plus:

| Standard | Relationship |
|----------|-------------|
| **DCR Graphs** | Constraint zone semantics adapted from DCR's five relation types. Proven at Danish government scale. |
| **Cedar** | Inspiration for the Verifiable Constraint Subset. SMT-based formal verification of governance properties. |
| **W3C Verifiable Credentials** | Attestation model draws from VC structure (issuer, subject, claims, verification method). |
| **OpenFisca / ISDA CDM** | Rules-as-Code integration targets for the Decision Tier. |
| **Estonia X-Road** | Federation profile model for cross-agency coordination in the Integration Tier. |
| **ISO/IEC 42001:2023** | AI management system standard. Attestation model supports conformity assessment alignment. |

---

## Appendix J. Changelog

| Date | Version | Description |
|------|---------|-------------|
| 2026-04-09 | 6.0.0 | Declarative constraint zones (DCR-style adaptive case management). Verifiable Constraint Subset (SMT-provable governance). Agent behavioral attestations. Dual-readability narrative for adverse decisions. Instance migration contract. Companion specification contracts (Business Calendar, Notification, Document Generation, Simulation). Verified Governance conformance profile. |
| 2026-04-09 | 5.0.0 | Formspec as universal interface contract. FEL unified expression language. Assist Governance Interface. Arazzo and CWL tool integration. Formspec provenance integration. DAG-based processing model. |
| 2026-04-09 | 4.0.0 | Tripartite object model. Deontic constraints. Typed patch operations. Kernel-versus-profiles. Deterministic replay. |
| 2026-04-09 | 3.0.0 | JSON-LD native. SHACL governance. PROV-AGENT. OCEL 2.0. RO-Crate. |
| 2026-04-08 | 2.0.0 | Eight-layer architecture. Four-layer audit. Structured oversight. Due process. |
