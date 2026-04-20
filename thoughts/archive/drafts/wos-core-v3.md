# Workflow Orchestration Standard (WOS) Core Specification

## W3C First Public Working Draft

**Latest published version:**
: https://wos-spec.org/TR/wos-core/

**Editor's Draft:**
: https://wos-spec.org/ed/wos-core/

**Editors:**
: Mike (TealWolf Consulting LLC)

**Version:**
: 3.0.0

**Date:**
: 9 April 2026

**Status:**
: First Public Working Draft

---

## Abstract

This specification defines the Workflow Orchestration Standard (WOS), a declarative, machine-readable language for describing high-stakes, long-running workflows in which humans and AI agents collaborate on consequential decisions. WOS provides an eight-layer architecture separating lifecycle topology, decision logic, human task management, agent governance, case state, integration, provenance, and durable execution into independently evolvable concerns.

WOS documents are natively serialized as JSON-LD. Every WOS document is simultaneously valid JSON, valid JSON-LD, and a serialization of an RDF graph. This design yields a single artifact that basic implementations consume as plain JSON while semantic-web-aware systems query, validate, link, and reason over the same artifact without transformation. Guardrails may be expressed as SHACL shapes. Provenance records align with W3C PROV-O and PROV-AGENT. Case data links to domain vocabularies (NIEM, FHIR, Schema.org) through standard JSON-LD context extension.

The standard treats human authority as supreme, AI participation as governed, and audit as foundational. It is designed for workflows where errors carry consequences for individuals and the public interest: grants processing, benefits adjudication, licensing, inspections, investigations, compliance review, and similar regulated processes.

WOS is informed by empirical research demonstrating that naive human-in-the-loop designs degrade decision quality, that model-generated explanations are unreliable audit evidence, that behavioral drift between model versions can be catastrophic, and that no single defense prevents prompt injection. The standard encodes these findings as structural requirements.

---

## Status of This Document

This document is a First Public Working Draft. It has not been endorsed by any standards body and has no formal standing. It is published to solicit feedback from implementers, domain experts, standards practitioners, and the broader workflow, case management, AI governance, and semantic web communities.

This is a living specification. Substantive changes will be tracked in a public changelog. Comments may be submitted as issues at the specification's repository.

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Conformance](#2-conformance)
3. [Terminology](#3-terminology)
4. [Architecture Overview](#4-architecture-overview)
5. [Document Structure and Serialization](#5-document-structure-and-serialization)
6. [Layer 1: Lifecycle and Topology](#6-layer-1-lifecycle-and-topology)
7. [Layer 2: Decision and Policy](#7-layer-2-decision-and-policy)
8. [Layer 3: Human Task Management](#8-layer-3-human-task-management)
9. [Layer 4: Agent Governance](#9-layer-4-agent-governance)
10. [Layer 5: Case State and Evidence](#10-layer-5-case-state-and-evidence)
11. [Layer 6: Integration and Eventing](#11-layer-6-integration-and-eventing)
12. [Layer 7: Provenance and Audit](#12-layer-7-provenance-and-audit)
13. [Layer 8: Durable Execution Contract](#13-layer-8-durable-execution-contract)
14. [Actor Model](#14-actor-model)
15. [Expression Language](#15-expression-language)
16. [Due Process Requirements](#16-due-process-requirements)
17. [Versioning and Evolution](#17-versioning-and-evolution)
18. [Security and Access Control](#18-security-and-access-control)
19. [Extensibility](#19-extensibility)
20. [Conformance Profiles](#20-conformance-profiles)
21. [Privacy Considerations](#21-privacy-considerations)
22. [Security Considerations](#22-security-considerations)
23. [References](#23-references)

**Appendices**

- [A. JSON-LD Context Document](#appendix-a-json-ld-context-document)
- [B. SHACL Shapes for Structural Governance](#appendix-b-shacl-shapes-for-structural-governance)
- [C. Complete Example: Grant Application Workflow](#appendix-c-complete-example)
- [D. Relationship to Existing Standards](#appendix-d-relationship-to-existing-standards)
- [E. Changelog](#appendix-e-changelog)

---

## 1. Introduction

### 1.1 Background

High-stakes workflows share requirements that no existing standard adequately integrates. They are long-running: a single case may span weeks, months, or years. They are human-centric: professional judgment, discretionary action, and override authority are core operating modes. They are evidence-driven: decisions depend on accumulated documents, data, and findings. They are heavily regulated: every action must be auditable, explainable, and traceable. And they increasingly involve AI agents: systems capable of classification, extraction, recommendation, and reasoning that participate alongside human workers.

Empirical research demonstrates that naive human-in-the-loop designs — presenting an AI recommendation and asking a human to confirm — degrade decision quality compared to either humans or AI operating independently (Vaccaro et al., Nature Human Behaviour, 2024; meta-analysis of 106 experiments). Model-generated explanations are systematically unfaithful to actual reasoning processes (Turpin et al., NeurIPS 2023; Lanham et al., Anthropic, 2023). Behavioral drift between model versions can cause near-total performance collapse (Chen, Zaharia, and Zou, 2023). No single defense against prompt injection has proven robust against adaptive adversaries (Nasr et al., 2025). These findings have produced documented harm in government deployments including Michigan's MiDAS system, Arkansas's RUGs algorithm, and the Dutch childcare benefits scandal.

This specification addresses these realities by treating agent governance, structured human oversight, formal constraint enforcement, and due process protections as foundational architectural concerns.

Additionally, this specification recognizes that workflow definitions, case data, and provenance records do not exist in isolation. They are nodes in a broader information ecosystem spanning agencies, jurisdictions, and domains. By adopting JSON-LD as the native serialization format, WOS documents are simultaneously plain JSON for basic tooling and RDF graphs for semantic querying, cross-system linking, and formal validation — without transformation, translation layers, or middleware.

### 1.2 Design Goals

WOS is designed to satisfy the following goals, listed in priority order:

1. **Human authority is supreme.** No agent configuration, autonomy level, or guardrail may override, circumvent, or diminish human decision-making authority. Agents assist; humans decide. Agent recommendations MUST NOT be the sole or determinative factor in adverse decisions affecting individual rights.

2. **Structured oversight, not checkbox review.** Human oversight mechanisms MUST produce genuine cognitive engagement. The standard specifies structural requirements — independent assessment, cognitive forcing, consider-the-opposite prompts — informed by empirical research on human-AI decision making.

3. **Accountability requires specificity.** Every action must be traceable to a specific actor, authority, inputs, outputs, and rule or policy version. The provenance model distinguishes immutable facts from model-generated narrative using a four-layer audit architecture.

4. **Constraints are external to the agent.** Guardrails are enforced by the WOS Processor, not by the agent. The agent is outside the trust boundary of the governance envelope.

5. **Graceful degradation is mandatory.** Every workflow MUST function correctly without any agent participation. Agent unavailability is a regular operating condition.

6. **Correctness is verifiable.** Workflow definitions MUST be formally verifiable for soundness. Agent behavior MUST be constrained within formally specified bounds. SHACL shapes provide policy-level structural governance on top of JSON Schema syntax validation.

7. **Linked by construction.** WOS documents are natively JSON-LD. Workflow definitions, case data, and provenance records are nodes in a queryable knowledge graph without requiring translation. Domain vocabularies are adopted by extending the `@context`, not by building middleware.

8. **Separation of concerns.** Process topology, decision logic, task management, agent governance, case state, integration, provenance, and execution guarantees are distinct concerns requiring distinct formalisms.

9. **Interoperability.** Workflow definitions MUST be portable across conformant implementations. The JSON-LD foundation enables cross-system querying and linked-data integration at the ecosystem level.

10. **AI-native authoring.** The serialization format, object model, and expression language MUST be designed for reliable generation, validation, transformation, and explanation by AI systems.

11. **Incremental adoption.** Implementations MAY conform to subsets of the specification via defined conformance profiles. Implementations that do not use RDF tooling MAY ignore the `@context` without loss of functionality.

### 1.3 Scope

**Within scope:** the eight-layer architecture and inter-layer interfaces; the metamodel of objects, relationships, and constraints; lifecycle semantics based on hierarchical state machines; decision services with defeasible rules and temporal parameters; human task lifecycle with structured oversight protocols; agent governance including autonomy levels, guardrails, confidence, fallback, and monitoring; case file structure and evidence model; event envelope format and correlation; four-layer provenance model with tamper evidence; abstract durable execution guarantees; the expression language profile; SHACL shapes for guardrail and definition validation; JSON-LD serialization with normative `@context`; due process requirements for adverse decisions; conformance profiles and testing.

**Out of scope:** user interface rendering and form specification; specific persistence mechanisms; specific transport protocols beyond interface contracts; process mining algorithms; ML model training or inference specification; document management systems; notification delivery mechanisms; general-purpose computation.

### 1.4 Relationship to Tier Specifications

This document is the **Core Specification**. Subsequent **Tier Specifications** elaborate individual layers. Each Tier Specification is normative for its layer but MUST NOT contradict the Core Specification.

Planned Tier Specifications:

- **WOS-Lifecycle** — Statechart semantics, transition resolution, verification algorithms.
- **WOS-Decision** — Decision table semantics, hit policies, defeasible rules, temporal parameters.
- **WOS-Task** — Human task lifecycle, structured oversight, SLA enforcement, separation of duties.
- **WOS-Agent** — Agent configuration, autonomy governance, guardrail system, multi-step sessions, drift monitoring.
- **WOS-CaseState** — Case file schema, evidence management, selective visibility.
- **WOS-Integration** — Event consumption/production, correlation, MCP/A2A alignment.
- **WOS-Provenance** — Four-layer audit records, PROV-AGENT alignment, tamper evidence, OCEL 2.0 interoperability, RO-Crate packaging.
- **WOS-Execution** — Durable execution guarantees, retry policies, compensation semantics.
- **WOS-SHACL** — SHACL shape library for guardrails and structural governance, bidirectional equivalence with WOS Expression Language constraints.
- **WOS-Conformance** — Test suite, canonical fixtures, certification procedures.

### 1.5 Notational Conventions

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in BCP 14 [RFC2119] [RFC8174] when, and only when, they appear in all capitals, as shown here.

---

## 2. Conformance

### 2.1 Conformance Classes

**WOS Document.** A serialized workflow definition conforming to the structural and semantic requirements of this specification. A conformant WOS Document MUST validate against the WOS Core JSON Schema and MUST include a valid `@context` reference.

**WOS Processor.** A software system that consumes WOS Documents and produces behavior consistent with the semantics defined herein. A WOS Processor MAY conform to one or more Conformance Profiles (§20).

### 2.2 Conformance Requirements for WOS Documents

A conformant WOS Document:

1. MUST be serialized in JSON-LD [JSON-LD11] (§5.3) or YAML 1.2 [YAML] that resolves to valid JSON-LD (§5.4).
2. MUST include the `@context` property referencing the WOS Core context document (§5.5).
3. MUST validate against the WOS Core JSON Schema without errors.
4. MUST satisfy all static semantic constraints including unique identifiers, valid state references, type-correct expressions, and resolvable cross-references.
5. MUST define a fallback to human performance for every agent invocation point.
6. SHOULD pass soundness verification as defined in §6.8.
7. SHOULD pass SHACL structural governance validation (Appendix B).

### 2.3 Conformance Requirements for WOS Processors

A conformant WOS Processor:

1. MUST accept any conformant WOS Document without error.
2. MUST reject documents that fail JSON Schema validation, producing diagnostics for each violation.
3. MUST preserve the `@context` property during serialization round-trips.
4. MUST execute lifecycle semantics (§6) consistent with this specification.
5. MUST produce provenance records (§12) for every state transition, task operation, decision evaluation, and agent invocation.
6. MUST enforce agent governance constraints (§9) including guardrails, autonomy limits, and fallback chains.
7. MUST support at least one Conformance Profile (§20).
8. SHOULD provide static soundness verification for workflow definitions.

---

## 3. Terminology

This section is normative.

**Activity.** A unit of work within a workflow, either automated (performed by a system or agent) or human (performed by a person via a Task).

**Actor.** An entity that performs actions within a workflow. Actors are classified by type: `human`, `system`, or `agent`. See §14.

**Agent.** An AI system participating in a workflow by performing tasks, evaluating decisions, or producing recommendations. Agents operate under declared autonomy levels and are subject to guardrail constraints, confidence reporting, and human oversight requirements. An agent is a type of Actor outside the trust boundary of the governance envelope.

**Agent Session.** A bounded interaction between a workflow instance and an agent, with a start event, zero or more checkpoints, and a terminal event.

**Autonomy Level.** A declared classification governing how much independent authority an actor has over a workflow action: `autonomous`, `supervisory`, `assistive`, or `manual`.

**Case.** An instance of a workflow applied to a specific subject, with a lifecycle, accumulated data and evidence, and audit records.

**Case File.** The structured data container associated with a Case, holding all typed data items, evidence references, and computed values.

**Compensation.** A semantically meaningful reversal of a completed activity.

**Confidence.** A structured assessment of certainty associated with an agent's output, comprising a scalar value, derivation method, optional per-field scores, and calibration status.

**Conformance Profile.** A named subset of this specification that an implementation may claim to support.

**Consequential Decision.** A determination that has legal, material, binding, or similarly significant effects on an individual's rights, benefits, services, or obligations. Subject to due process requirements (§16).

**Context Document.** The normative JSON-LD `@context` artifact published at `https://wos-spec.org/context/3.0.0` that maps WOS property names to IRIs, enabling every WOS document to function as a node in the linked data web. See §5.5.

**Decision Record.** An immutable audit entry recording the evaluation of a Decision Service, structured as a PROV-AGENT-compatible provenance graph node.

**Decision Service.** An encapsulated unit of decision logic with defined inputs and outputs, independently versionable and invocable.

**Defeasible Rule.** A rule that may be overridden by a more specific rule with higher priority, modeling the "general rule with exceptions" pattern common in regulatory contexts.

**Deferred Choice.** A pattern where the next step is determined by an external event or human action at runtime, not predetermined by the designer.

**Durable Timer.** A timer that persists across system restarts, may span arbitrary durations, and consumes no runtime resources while waiting.

**Evidence.** A document, dataset, image, or other artifact attached to a Case File Item with content integrity verification.

**Governance Envelope.** The set of autonomy constraints, guardrails, confidence requirements, and fallback policies surrounding every agent invocation, enforced by the WOS Processor, not by the agent.

**Guard.** A boolean expression controlling whether a transition may fire, expressed in the WOS Expression Language (§15).

**Guardrail.** A declarative constraint on an agent's behavior, enforced by the WOS Processor after the agent produces output and before the output is committed. Guardrails MAY be expressed as SHACL shapes or WOS Expression Language constraints.

**History State.** A mechanism recording the last active substate within a compound state, enabling resumption of prior configuration after suspension.

**Milestone.** A named condition on case data that, when satisfied, indicates meaningful progress.

**Override.** A human action superseding an automated decision, requiring structured rationale and recorded in the audit trail.

**Parallel Region.** An independently executing concurrent track within a compound state.

**Provenance Record.** An immutable audit entry aligned with W3C PROV-O and PROV-AGENT, recording what happened, when, by whom, under what authority, and why, structured in a four-layer audit architecture.

**SHACL Shape.** A constraint definition conforming to the W3C Shapes Constraint Language, used to validate RDF graph data against structural and semantic rules.

**Sentry.** A combination of event trigger and guard condition controlling stage activation or milestone achievement.

**Soundness.** A formal property guaranteeing deadlock-freedom, livelock-freedom, proper termination, reachability, and fallback completeness.

**Stage.** A compound lifecycle phase containing substates, activities, milestones, and sentries.

**State.** A named condition in the lifecycle of a Case, Task, or Stage.

**Structured Oversight.** Human review mechanisms designed to produce genuine cognitive engagement, including independent assessment, cognitive forcing functions, and consider-the-opposite prompts. Distinct from checkbox confirmation.

**Task.** A unit of human work with a defined lifecycle, assignment model, data contract, and oversight protocol.

**Transition.** A directed edge between states, triggered by an event and optionally gated by a guard condition.

**Workflow Definition.** A complete WOS Document specifying a workflow's lifecycle, decisions, tasks, agent governance, case structure, integrations, and audit requirements.

**Workflow Instance.** A running case governed by a Workflow Definition.

---

## 4. Architecture Overview

This section is normative.

### 4.1 Eight-Layer Architecture

WOS defines eight conceptual layers. Each layer addresses a distinct concern, has well-defined interfaces to adjacent layers, and is independently authorable and versionable.

```
┌──────────────────────────────────────────────────────────┐
│  Layer 8: Durable Execution Contract                     │
│  Abstract guarantees for runtime resilience               │
├──────────────────────────────────────────────────────────┤
│  Layer 7: Provenance and Audit                           │
│  PROV-AGENT records, four-layer model, tamper evidence    │
├──────────────────────────────────────────────────────────┤
│  Layer 6: Integration and Eventing                       │
│  CloudEvents, correlation, MCP/A2A/Schema.org alignment  │
├──────────────────────────────────────────────────────────┤
│  Layer 5: Case State and Evidence                        │
│  Typed case data, evidence, vocabulary-linked, selective  │
├──────────────────────────────────────────────────────────┤
│  Layer 4: Agent Governance                               │
│  Autonomy, guardrails (SHACL), confidence, fallback      │
├──────────────────────────────────────────────────────────┤
│  Layer 3: Human Task Management                          │
│  Task lifecycle, structured oversight, SLAs, separation   │
├──────────────────────────────────────────────────────────┤
│  Layer 2: Decision and Policy                            │
│  Decision tables, defeasible rules, temporal parameters   │
├──────────────────────────────────────────────────────────┤
│  Layer 1: Lifecycle and Topology                         │
│  Statechart, states, transitions, guards, milestones      │
└──────────────────────────────────────────────────────────┘

           ┌─────────────────────────────────┐
           │  JSON-LD / RDF Graph Foundation  │
           │  @context · SHACL · PROV-O      │
           └─────────────────────────────────┘
```

The JSON-LD/RDF foundation is not a layer. It is the substrate on which all layers are expressed. Every object in every layer is a node in an RDF graph by virtue of the JSON-LD serialization. This does not require implementations to use RDF tooling; it means that those capabilities are available by construction for implementations that choose to use them.

### 4.2 Separation Principles

The following separation principles are normative:

**Process topology MUST be separated from decision logic.** The lifecycle (Layer 1) defines structure. Decision Services (Layer 2) evaluate conditions. A guard MAY invoke a Decision Service, but the Decision Service MUST NOT contain process topology.

**Decision logic MUST be separated from task management.** Decisions determine what should happen. Tasks determine who does it and how.

**Agent governance MUST be separated from agent implementation.** Guardrails, autonomy constraints, and confidence requirements are properties of the workflow definition, enforced by the WOS Processor. They MUST NOT be delegable to the agent.

**Case data MUST be separated from process state.** The Case File (Layer 5) holds business data. The lifecycle state (Layer 1) tracks process progress.

**Audit MUST be separated from execution.** Provenance records (Layer 7) observe all other layers but do not participate in control flow.

**Execution guarantees MUST be separated from execution mechanisms.** Layer 8 defines what guarantees hold, not how they are achieved.

**Syntax validation MUST be separated from semantic governance.** JSON Schema validates structural syntax (required fields, types, enums). SHACL shapes validate policy-level constraints (the interplay between impact level, autonomy, guardrails, and due process). Both are conformance mechanisms; they operate at different levels.

### 4.3 Cross-Cutting Concerns

**Actor Model (§14).** Every action is attributed to a typed actor with distinct provenance requirements.

**Due Process (§16).** Consequential decisions are subject to notice, explanation, appeal, and continuation-of-services requirements spanning multiple layers.

**Expressions (§15).** Guard conditions, data transformations, and computed values use the WOS Expression Language.

**Identity.** Every object has a unique identifier expressed as a URI (IRI). When the document is interpreted as JSON-LD, `id` maps to `@id`, making every WOS object a named RDF node.

**Versioning (§17).** Workflow Definitions, Decision Services, Task Definitions, and Agent Configurations carry independent version identifiers.

---

## 5. Document Structure and Serialization

This section is normative.

### 5.1 Design Rationale

WOS documents serve three audiences simultaneously: human authors who write and review workflow definitions in YAML, software systems that consume and execute workflow definitions as JSON, and semantic tooling that queries, validates, links, and reasons over workflow definitions as RDF graphs. By choosing JSON-LD as the canonical JSON format, a single document serves all three audiences without transformation. The `@context` document does the mapping work. Authors write familiar YAML. Processors consume familiar JSON. Semantic systems interpret the same JSON as RDF. The cost of this design is near zero: the `@context` is published once at the specification level, and the only structural constraint it imposes is that property names must be unambiguous when interpreted as predicates — which is already good practice.

### 5.2 Top-Level Structure

```yaml
"@context": "https://wos-spec.org/context/3.0.0"  # REQUIRED
wos: "3.0.0"                                       # REQUIRED
id: "urn:wos:example.gov:grant-review"              # REQUIRED
name: "Grant Application Review"                    # REQUIRED
version: "1.0.0"                                    # REQUIRED
status: "active"                                    # REQUIRED

metadata:                                           # OPTIONAL
  description: "..."
  authors: [...]
  created: "2026-04-09T00:00:00Z"
  modified: "2026-04-09T00:00:00Z"
  tags: [...]
  jurisdiction: "US-Federal"
  authority: "24 CFR Part 570"
  effectiveDate: "2026-04-15"
  sunsetDate: null
  impactLevel: "rights-impacting"

lifecycle: { ... }                                  # REQUIRED — Layer 1
decisions: { ... }                                  # OPTIONAL — Layer 2
parameters: { ... }                                 # OPTIONAL — Layer 2
tasks: { ... }                                      # OPTIONAL — Layer 3
agents: { ... }                                     # OPTIONAL — Layer 4
caseFile: { ... }                                   # OPTIONAL — Layer 5
integrations: { ... }                               # OPTIONAL — Layer 6
provenance: { ... }                                 # OPTIONAL — Layer 7
execution: { ... }                                  # OPTIONAL — Layer 8
dueProcess: { ... }                                 # OPTIONAL — §16
extensions: { ... }                                 # OPTIONAL — §19
```

### 5.3 JSON-LD Serialization

The canonical machine-interchange format for WOS Documents is JSON-LD 1.1 [JSON-LD11]. A WOS Document serialized as JSON-LD is simultaneously valid JSON [RFC8259], valid JSON-LD, and a serialization of an RDF graph [RDF11].

This means that the same document, without any transformation, can be consumed by a JSON parser that ignores the `@context` (losing no structural information), processed by a JSON-LD processor that resolves property names to IRIs and interprets the document as an RDF dataset, loaded into a triplestore and queried with SPARQL, or validated by a SHACL processor against governance shapes.

Implementations that do not use RDF tooling MAY ignore the `@context` property. Ignoring the `@context` does not affect JSON Schema validation, lifecycle semantics, or any other conformance requirement. The semantic layer is available by construction for implementations that choose to use it and invisible to those that do not.

### 5.4 YAML Authoring Format

YAML 1.2 [YAML] is the RECOMMENDED format for human authoring. A WOS Document authored in YAML MUST resolve to valid JSON-LD upon conversion to JSON. The `@context` property MUST be present in the YAML source or injected during YAML-to-JSON conversion by the WOS Processor.

YAML-specific features (anchors, aliases, tags, comments) MAY be used for authoring convenience but are not part of the semantic model. A WOS Processor MUST interpret YAML documents by first resolving them to their JSON-LD equivalent.

### 5.5 The `@context` Document

The WOS specification publishes a normative JSON-LD `@context` document at:

```
https://wos-spec.org/context/3.0.0
```

This document maps every WOS property name to an IRI. The mappings draw from three sources, in order of preference:

1. **Existing W3C and community vocabularies** where WOS semantics align: `prov:` for provenance terms, `schema:` for Schema.org types, `sh:` for SHACL references, `xsd:` for data types.

2. **Process ontology terms** from the BBO (BPMN Based Ontology) and related research where WOS lifecycle concepts have sound existing formalizations: states, transitions, guards, parallel regions, milestones.

3. **The WOS namespace** (`https://wos-spec.org/ns/`) for concepts original to this specification: agent governance, structured oversight, confidence framework, due process, guardrails, autonomy levels.

At minimum, the `@context` establishes the following mappings:

| WOS Property | Maps To | Source |
|-------------|---------|--------|
| `id` | `@id` | JSON-LD keyword |
| `type` | `@type` | JSON-LD keyword |
| `name` | `schema:name` | Schema.org |
| `description` | `schema:description` | Schema.org |
| `created` | `schema:dateCreated` | Schema.org |
| `modified` | `schema:dateModified` | Schema.org |
| `actor` | `prov:wasAssociatedWith` | PROV-O |
| `timestamp` | `prov:atTime` | PROV-O |
| `wasGeneratedBy` | `prov:wasGeneratedBy` | PROV-O |
| `wasDerivedFrom` | `prov:wasDerivedFrom` | PROV-O |
| `lifecycle` | `wos:lifecycle` | WOS namespace |
| `states` | `wos:states` | WOS namespace |
| `transitions` | `wos:transitions` | WOS namespace |
| `guard` | `wos:guard` | WOS namespace |
| `autonomy` | `wos:autonomyLevel` | WOS namespace |
| `confidence` | `wos:confidence` | WOS namespace |
| `guardrails` | `wos:guardrails` | WOS namespace |
| `impactLevel` | `wos:impactLevel` | WOS namespace |

The complete `@context` document is provided in Appendix A. The `@context` is versioned alongside the specification. Breaking changes to the `@context` (renaming or removing mappings) MUST increment the major specification version.

### 5.6 Extending the `@context` for Domain Vocabularies

WOS Documents MAY extend the `@context` to reference additional domain vocabularies. This is the mechanism for cross-agency data interoperability:

```json
{
  "@context": [
    "https://wos-spec.org/context/3.0.0",
    {
      "niem-hs": "https://release.niem.gov/niem/domains/humanServices/6.0/",
      "fhir": "http://hl7.org/fhir/"
    }
  ],
  "id": "urn:wos:example.gov:benefits-review",
  "wos": "3.0.0"
}
```

When a Case File Item's schema references an external vocabulary, the item's properties are mapped to that vocabulary's terms in the JSON-LD serialization. A grant case file referencing NIEM's Human Services vocabulary becomes a linked data node that other agencies' systems can consume directly, without middleware or translation layers.

### 5.7 JSON Schema Validation

JSON Schema [JSONSchema] remains the structural conformance mechanism. Every WOS Document MUST validate against the WOS Core JSON Schema published at `https://wos-spec.org/schema/core/3.0.0`. JSON Schema validates syntactic structure: required fields, types, enumerations, and structural constraints. It does not validate policy-level semantics; that role belongs to SHACL (§5.8).

JSON-LD is valid JSON. JSON Schema validation operates on the JSON surface and is unaffected by the `@context`.

### 5.8 SHACL Governance Validation

SHACL [SHACL] provides policy-level validation of WOS Documents interpreted as RDF graphs. The specification defines a set of structural governance shapes (Appendix B) that enforce architectural constraints:

- Every workflow with `impactLevel: "rights-impacting"` MUST include a `dueProcess` configuration with an enabled appeal mechanism.
- No `invokeAgent` action MAY specify `autonomy: "autonomous"` without an associated guardrail definition.
- Every `invokeAgent` action MUST have a reachable fallback path terminating in a human task.
- Every task with `agentAssistance` MUST specify an `oversight.protocol`.

JSON Schema cannot express these constraints because they involve relationships between multiple objects across different sections of the document. SHACL shapes operate on the graph structure and can validate cross-cutting policies.

SHACL governance validation is RECOMMENDED for all WOS Documents and REQUIRED for the Semantic Conformance Profile (§20.8). A WOS Processor that performs SHACL validation MUST produce a `sh:ValidationReport` identifying each violation.

### 5.9 Properties Common to All Sections

**Property: `wos`.** REQUIRED. Semantic version string identifying the specification version. A WOS Processor MUST reject a document whose major version it does not support.

**Property: `id`.** REQUIRED. A URI [RFC3986] uniquely identifying this Workflow Definition. Maps to `@id` in the JSON-LD context. The `urn:wos:` scheme is RECOMMENDED.

**Property: `version`.** REQUIRED. Semantic version string [SemVer] identifying this definition version. Recorded immutably in instance metadata at creation time.

**Property: `status`.** REQUIRED. One of `draft` (not for production), `active` (approved for production), `deprecated` (superseded; existing instances continue), or `retired` (no new instances).

**Property: `metadata`.** OPTIONAL. Descriptive information including `description`, `authors`, `created`, `modified`, `tags`, `jurisdiction`, `authority`, `effectiveDate`, `sunsetDate`, and `impactLevel`.

**Property: `impactLevel`.** Classifies the consequence level of decisions made within this workflow:

| Value | Definition | Requirements |
|-------|-----------|-------------|
| `rights-impacting` | Decisions affect individual legal rights, benefits, services, or obligations. | Full due process (§16). Agent autonomy capped at `assistive` unless elevated. |
| `safety-impacting` | Decisions affect individual or public safety. | Full due process. Agent autonomy capped at `assistive`. |
| `operational` | Decisions affect organizational operations without direct individual impact. | Due process RECOMMENDED. Agent autonomy up to `autonomous` with guardrails. |
| `informational` | Outputs are informational and do not drive binding decisions. | Due process OPTIONAL. No autonomy restrictions. |

Default when unspecified: `operational`.

---

## 6. Layer 1: Lifecycle and Topology

This section is normative.

### 6.1 Overview

The Lifecycle layer defines the statechart governing a workflow instance's progression. The semantics are based on Harel statecharts [Harel1987] as formalized in W3C SCXML [SCXML], adapted for case-oriented workflows. When a lifecycle definition is interpreted as JSON-LD, each state is a named RDF node and each transition is a directed relationship, enabling SPARQL queries over process topology.

### 6.2 Property: `lifecycle`

REQUIRED. Defines the top-level statechart.

```yaml
lifecycle:
  initialState: "intake"
  defaultAutonomy: "assistive"

  states:
    intake:
      type: "atomic"
      onEntry:
        - action: "createTask"
          taskRef: "initialReview"
      transitions:
        - event: "task.completed"
          target: "evaluation"
          guard: "caseFile.application.isComplete = true"
        - event: "task.completed"
          target: "returnedToApplicant"
          guard: "caseFile.application.isComplete = false"

    evaluation:
      type: "parallel"
      regions:
        technicalReview:
          initialState: "pending"
          states:
            pending: { type: "atomic" }
            complete: { type: "final" }
        financialReview:
          initialState: "pending"
          states:
            pending: { type: "atomic" }
            complete: { type: "final" }
      transitions:
        - event: "regions.allFinal"
          target: "adjudication"

    adjudication:
      type: "compound"
      initialState: "recommendation"
      historyState: "shallow"
      states:
        recommendation: { type: "atomic" }
        supervisorDecision: { type: "atomic" }
        decided: { type: "final" }

    completed:
      type: "final"

  milestones:
    applicationReceived:
      condition: "caseFile.application != null"
    eligibilityConfirmed:
      condition: "caseFile.eligibility.decision = 'eligible'"
```

### 6.3 States

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `type` | enum | REQUIRED | `atomic`, `compound`, `parallel`, or `final`. |
| `onEntry` | array of Action | OPTIONAL | Actions on state entry. |
| `onExit` | array of Action | OPTIONAL | Actions on state exit. |
| `transitions` | array of Transition | OPTIONAL | Outgoing transitions. |
| `historyState` | enum | OPTIONAL | `shallow` or `deep`. Compound states only. |
| `initialState` | string | CONDITIONAL | Required for `compound` states. |
| `regions` | map of Region | CONDITIONAL | Required for `parallel` states. |
| `metadata` | object | OPTIONAL | Descriptive metadata. |

**Compound states** contain substates with a designated `initialState`. `historyState: "shallow"` records the last active direct substate; `"deep"` records the full nested configuration, enabling complete restoration after suspension.

**Parallel states** contain named regions executing concurrently. A parallel state is not exited until all regions reach a final state (raising `regions.allFinal`), unless an explicit transition overrides this.

**Final states** indicate completion. A top-level final state indicates workflow completion. Final states MUST NOT have outgoing transitions.

### 6.4 Transitions

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `event` | string | REQUIRED | Triggering event identifier. |
| `target` | string | REQUIRED | Target state identifier. |
| `guard` | string (expression) | OPTIONAL | WOS Expression evaluating to `true` to fire. |
| `actions` | array of Action | OPTIONAL | Actions executed during transition. |
| `priority` | integer | OPTIONAL | Resolution priority; lower is higher. Default: 0. |
| `description` | string | OPTIONAL | Human-readable explanation. |

**Transition resolution:** Collect matching transitions, evaluate guards, discard those evaluating to `false`. If one remains, fire it. If multiple share the same lowest priority, the document is ill-formed. If none remain, the event is discarded (not an error).

**Execution sequence:** (1) onExit of source, innermost first. (2) Transition actions. (3) onEntry of target, outermost first. (4) Provenance record emitted.

### 6.5 Actions

| Action Type | Description | Key Properties |
|-------------|-------------|----------------|
| `createTask` | Creates a human task instance. | `taskRef` |
| `invokeDecision` | Invokes a decision service. | `decisionRef`, `outputBinding`, `autonomy` |
| `invokeAgent` | Invokes an agent within its governance envelope. | `agentRef`, `capability`, `autonomy` |
| `setData` | Sets a case file value. | `path`, `value` |
| `emitEvent` | Emits an event to the integration layer. | `eventType`, `data` |
| `startTimer` | Starts a durable timer. | `timerId`, `duration` or `deadline`, `event` |
| `cancelTimer` | Cancels a running timer. | `timerId` |
| `compensate` | Triggers compensation for a scope. | `scope` |
| `log` | Writes an informational audit entry. | `message`, `level` |
| `notify` | Sends a notification. | `recipientRoles`, `template` |

### 6.6 Events

**Internal events:** `task.completed`, `task.failed`, `task.escalated`, `timer.expired`, `regions.allFinal`, `error`, `milestone.achieved`, `decision.complete`, `agent.complete`, `agent.failed`, `guardrail.violated`.

**External events** originate outside the workflow and are matched via correlation (§11.4).

### 6.7 Milestones

Declarative checkpoints achieved when a condition on case data becomes true. Re-evaluated when referenced data changes. Achievement raises `milestone.achieved`.

### 6.8 Soundness Verification

A WOS Document SHOULD be verifiable for:

**Deadlock-freedom.** From every reachable non-final state, a final state is reachable.

**Livelock-freedom.** No infinite cycle without progress toward a final state.

**Proper termination.** When the top-level final state is reached, no parallel regions have active states and no tasks remain non-terminal.

**No dead elements.** Every state, transition, and task definition is reachable.

**Fallback completeness.** Every `invokeAgent` action has a reachable path to workflow completion that does not require any agent to succeed. Verifiable by treating all agent invocations as failures and checking the workflow remains sound.

A conformant verifier MUST support a decidable fragment and MUST report inconclusive results as warnings, not passes.

---

## 7. Layer 2: Decision and Policy

This section is normative.

### 7.1 Decision Services

Encapsulated logic for routing, eligibility, classification, and policy evaluation. Independently versioned. Each Decision Service, when interpreted as JSON-LD, is a named RDF node that can be discovered and linked across workflow definitions.

```yaml
decisions:
  eligibilityDetermination:
    version: "2.1.0"
    description: "Determines applicant eligibility per 24 CFR 570.200."
    inputs:
      - name: "applicantIncome"
        schema: { type: "number" }
      - name: "householdSize"
        schema: { type: "integer" }
      - name: "applicationDate"
        schema: { type: "string", format: "date" }
    outputs:
      - name: "eligible"
        schema: { type: "boolean" }
      - name: "reason"
        schema: { type: "string" }
    logic:
      type: "decisionTable"
      hitPolicy: "first"
      rules: [...]
    effectiveDate: "2026-01-01"
    sunsetDate: "2026-12-31"
```

### 7.2 Decision Logic Types

**Decision Tables** with hit policies: `unique`, `first`, `priority`, `collect`, `collectSum`, `collectMin`, `collectMax`, `collectCount`. Consistent with DMN [DMN].

**Expression Logic** — a single WOS Expression computing the output.

**Decision Requirement Graphs** — a DAG of sub-decisions enabling composition from testable components.

**Defeasible Rules** — rules with structured exceptions for regulatory contexts. Override relationships MUST form a DAG. When multiple rules match, the overriding rule takes precedence.

```yaml
logic:
  type: "defeasibleRules"
  rules:
    - id: "generalEligibility"
      condition: "applicantIncome <= parameters.incomeThreshold(applicationDate)"
      conclusion: { eligible: true }
    - id: "veteranException"
      overrides: "generalEligibility"
      condition: "applicant.veteranStatus = true"
      conclusion: { eligible: true, applicableThreshold: "parameters.incomeThreshold(applicationDate) * 1.2" }
    - id: "disqualification"
      overrides: ["generalEligibility", "veteranException"]
      condition: "applicant.priorFraudFinding = true"
      conclusion: { eligible: false, reason: "Disqualified: prior fraud finding" }
```

**External Decision Reference** — delegates to an external service via the Integration layer.

### 7.3 Temporal Parameters

Values that change on specific dates, modeling periodically updated thresholds, rates, and criteria. When referenced, the value effective on the reference date is used. No effective value raises an error.

### 7.4 Decision Invocation and Provenance

Every invocation MUST produce a Decision Record (§12.4) as a PROV-AGENT-compatible provenance graph node capturing service identifier and version, complete inputs, matched rules, outputs, timestamp, and workflow context.

---

## 8. Layer 3: Human Task Management

This section is normative.

### 8.1 Task Lifecycle

```
                               ┌──────────┐
                         ┌────>│ Returned │──┐
                         │     └──────────┘  │
                         │                   v
┌───────────┐    ┌──────────┐    ┌────────────┐    ┌───────────┐
│ Available │───>│ Claimed  │───>│ InProgress │───>│ Completed │
└───────────┘    └──────────┘    └────────────┘    └───────────┘
      │               │               │
      │               │               ├──> ┌──────────┐
      │               │               │    │  Failed  │
      │               │               │    └──────────┘
      v               v               v
┌───────────┐   ┌───────────┐   ┌───────────┐
│ Cancelled │   │ Suspended │   │ Escalated │
└───────────┘   └───────────┘   └───────────┘
```

Terminal states: `Completed`, `Failed`, `Cancelled`.

### 8.2 Task Operations

`create`, `claim`, `release`, `start`, `complete`, `fail`, `delegate`, `forward`, `returnForRework`, `escalate`, `suspend`, `resume`, `cancel`. Every operation MUST produce a provenance record.

### 8.3 Assignment Model

Five role categories: `potentialOwners`, `excludedOwners`, `businessAdministrators`, `taskStakeholders`, `notificationRecipients`. Each specified by `roles`, `skills`, `individuals`, or `expression`.

### 8.4 Service Level Agreements

`dueIn` (duration or business duration with `BD`/`BH` suffix), `dueBy` (computed deadline), `warningAt`, `escalateOnBreach`, `businessCalendar`.

### 8.5 Separation of Duties

Constraints preventing the same individual from performing specified task combinations. Enforced at claim time. Violations rejected and logged.

### 8.6 Structured Oversight Protocols

This section is normative. It addresses the empirical finding that naive human-AI review degrades decision quality.

When a task involves agent assistance, the `oversight.protocol` field is REQUIRED.

| Protocol | Description | Empirical Basis |
|----------|-------------|----------------|
| `independentFirst` | Reviewer MUST form and record an independent assessment before the agent's recommendation is revealed. The interface MUST enforce this ordering. | Buçinca et al. (CSCW 2021): cognitive forcing functions reduce overreliance. |
| `considerOpposite` | After viewing the agent's recommendation, the reviewer is prompted to articulate reasons the recommendation might be wrong before confirming. | Anchoring bias research: consider-the-opposite debiases. |
| `calibratedConfidence` | The agent's calibrated confidence score is displayed. Per-field confidence shown when available. Low-confidence fields highlighted. | Li et al. (2024): miscalibrated confidence impairs reliance. |
| `dualBlind` | Two independent reviewers assess the case without seeing each other's or the agent's assessment. Results reconciled. | Standard for high-stakes adjudication. |
| `unassisted` | No agent assistance. Entirely human. | Baseline for tasks requiring unmediated judgment. |

Multiple protocols MAY be combined. When `independentFirst` is specified, the WOS Processor MUST enforce that the reviewer's independent assessment is recorded before the agent's output is accessible.

### 8.7 Override Authority

Authorized individuals may override automated decisions with mandatory structured accountability. Override Records (§12.5) capture the original result, override values, rationale, authority, and evidence.

---

## 9. Layer 4: Agent Governance

This section is normative.

### 9.1 Overview

The Agent Governance layer defines how AI agents participate as constrained, monitored, accountable actors. The governance envelope — autonomy constraints, guardrails, confidence requirements, and fallback policies — is enforced by the WOS Processor. The agent is outside the trust boundary.

### 9.2 Agent Configuration

```yaml
agents:
  eligibilityScreener:
    version: "2.0.0"
    description: "Pre-screens applications against eligibility criteria."
    model:
      provider: "anthropic"
      identifier: "claude-sonnet-4-20250514"
      versionPolicy: "pinned"
    capabilities:
      - id: "eligibilityPreScreen"
        decisionRef: "eligibilityDetermination"
    defaultAutonomy: "assistive"
    autonomyPolicy:
      maxAutonomy: "assistive"
      reason: "Eligibility determinations affect individual rights."
      escalation:
        toSupervisory:
          conditions:
            - "agent.calibration.accuracy >= 0.97"
            - "agent.guardrailViolations(P30D) = 0"
          approval: { roles: ["programDirector"], expires: "P90D" }
      demotion:
        triggers:
          - condition: "agent.calibration.accuracy < 0.85"
            immediate: true
          - condition: "agent.guardrailViolations(P7D) >= 3"
            immediate: true
          - condition: "agent.modelVersionChanged"
            pendingRecalibration: true
    guardrails:
      outputConstraints:
        - field: "eligible"
          constraint: "value = true or value = false"
          onViolation: "reject"
        - field: "reason"
          constraint: "string length(value) >= 20"
          onViolation: "reject"
      confidenceFloor:
        threshold: 0.7
        onViolation: "escalateToHuman"
      prohibited:
        - condition: "output.eligible = true and caseFile.application.areaMedianIncome > parameters.amiThreshold(caseFile.application.submittedDate) * 1.5"
          reason: "Eligible with income far above threshold requires review."
          onViolation: "escalateToHuman"
      humanReviewSampling:
        rate: 0.10
        method: "random"
      volumeConstraints:
        maxAutonomousPerHour: 50
        onExceeded: "switchToAssistive"
    fallback:
      primary: { onFailure: "retry", maxRetries: 2, backoff: "exponential", initialInterval: "PT5S" }
      terminal: { onFailure: "escalateToHuman", taskRef: "manualEligibilityScreening" }
    inputPreparation:
      sanitize: true
      maxInputTokens: 50000
      redactFields: ["caseFile.application.socialSecurityNumber"]
      isolateUntrustedData: true
    monitoring:
      calibrationRequired: true
      calibrationFrequency: "P30D"
      minimumEvaluationSamples: 100
      driftDetection:
        enabled: true
        method: "psi"
        threshold: 0.2
        window: "P7D"
```

### 9.3 Autonomy Levels

| Level | Semantics |
|-------|-----------|
| `autonomous` | Output committed without human review. REQUIRES guardrails. PROHIBITED for `rights-impacting`/`safety-impacting` unless explicitly elevated. |
| `supervisory` | Output provisionally committed. Human reviews within a defined window. No intervention = finalized. |
| `assistive` | Agent produces recommendation. Human reviews, may modify, explicitly confirms. Output attributed to human. |
| `manual` | Human performs action. Agent MAY assist on demand. Output solely human's. |

**Normative constraints:** Autonomous actions without guardrails are structural errors. Assistive actions MUST create a human task with a specified oversight protocol. Supervisory actions MUST define a `reviewWindow`. Effective autonomy MUST NOT exceed `maxAutonomy`. For `rights-impacting`/`safety-impacting` workflows, default autonomy is `assistive` regardless of agent configuration.

### 9.4 Confidence Framework

Every agent output MUST include a ConfidenceReport: `overall` (0.0–1.0), `method` (`modelNative`, `calibrated`, `heuristic`, `conformal`, `declared`), `calibrationStatus` (`calibrated`, `uncalibrated`, `expired`), optional `explanation`, optional `fieldLevel` per-output-field scores.

Agents with `calibrationRequired: true` MUST have confidence validated empirically. Agents with `calibrationStatus: "expired"` MUST NOT operate above `assistive`. Cumulative confidence in multi-step sessions is tracked conservatively (multiplicative default); below the floor triggers human review.

### 9.5 Guardrail System

Guardrails are declarative constraints on agent outputs, enforced by the WOS Processor.

**Guardrail types:** `outputConstraints`, `confidenceFloor`, `prohibited`, `consistency`, `volumeConstraints`, `humanReviewSampling`, `scope`.

**Dual representation.** Guardrail constraints have two equivalent forms:

1. **WOS Expression Language** — the practitioner authoring surface, specified inline in the workflow definition.
2. **SHACL shapes** — the formal validation representation. Because WOS documents and agent outputs are JSON-LD (and therefore RDF graphs), SHACL processors can validate them directly.

A WOS Processor MAY implement guardrail enforcement via either mechanism. The WOS-SHACL Tier Specification will define the bidirectional equivalence between expression-based guardrails and SHACL shapes, and will provide a library of SHACL shapes corresponding to each guardrail type.

**Enforcement order:** (1) Output constraints. (2) Prohibited outputs. (3) Consistency. (4) Confidence floor. (5) Volume constraints. (6) Human review sampling.

**Violation actions:** `reject` > `escalateToHuman` > `switchToAssistive` > `flag`. Most restrictive applies when multiple guardrails are violated.

**Composition:** Guardrails at workflow, agent, and action levels compose by union. All are evaluated. Narrower scopes take precedence for severity.

**Bypass:** Authorized human with role at or above `bypassAuthority`, providing structured rationale. Single invocation only. Produces `guardrailBypass` provenance record.

### 9.6 Fallback Chains

Ordered degradation sequences. MUST terminate in `escalateToHuman` or `fail`. MUST NOT cycle. Validated at document load. Every attempt produces a provenance record.

### 9.7 Input Preparation and Isolation

`sanitize` (prompt injection patterns), `maxInputTokens`, `redactFields`/`includeFields`, `isolateUntrustedData` (CaMeL dual-LLM pattern: separate trusted control flow from untrusted data processing).

### 9.8 Monitoring and Drift Detection

Agent states: `active`, `degraded`, `suspended`, `retired`. Drift detection via `psi`, `ks`, `chi2`, or `accuracy` methods. Shadow deployment RECOMMENDED for model version changes in `rights-impacting`/`safety-impacting` workflows.

### 9.9 Multi-Step Agent Sessions

Sessions with checkpoints and intervention points. Checkpoints enable recovery, inspection, and intervention. At intervention points, a human may approve, modify, redirect, terminate, or restart.

### 9.10 Agent Tool Use Governance

Agents MUST NOT invoke non-permitted tools. MUST NOT write to case file directly. Every tool invocation recorded in provenance. Side-effecting tools at autonomous level require explicit policy. Cascading autonomous agent invocations require `cascadingAutonomy: "permitted"` with bounded `maxCascadeDepth`.

---

## 10. Layer 5: Case State and Evidence

This section is normative.

### 10.1 Overview

The Case File is the central artifact: the process exists to serve the case. When serialized as JSON-LD, Case File Items are RDF nodes that can reference external vocabularies, enabling cross-agency interoperability by extending the `@context` (§5.6) rather than building translation middleware.

### 10.2 Case File Definition

Case File Items have `schema` (JSON Schema), `visibility` (role-based access), and `multiplicity` (`one` or `many`). Items MAY declare a `vocabulary` property referencing a namespace (NIEM, FHIR, Schema.org) whose terms the item's properties map to in the JSON-LD serialization.

```yaml
caseFile:
  items:
    application:
      schema: { ... }
      vocabulary: "https://release.niem.gov/niem/domains/humanServices/6.0/"
      visibility:
        default: "restricted"
        overrides:
          - roles: ["reviewer", "supervisor"]
            access: "readWrite"
      multiplicity: "one"
```

### 10.3 Data Mutation Semantics

Every mutation recorded as an immutable provenance event: path, prior value, new value, actor, context, timestamp. A WOS Processor MUST reconstruct Case File state at any prior point by replaying events.

### 10.4 Evidence Management

Evidence referenced via claim check pattern: content hash (SHA-256+), content type, claim check URI. Hash integrity failures logged. Evidence MUST NOT be stored inline.

### 10.5 Selective Visibility

Role-based field-level access control: `read`, `readWrite`, `none`. Enforced when presenting data to task participants and via query interfaces.

---

## 11. Layer 6: Integration and Eventing

This section is normative.

### 11.1 Integration Types

`request-response` (sync, OpenAPI), `event-emit` (outbound), `event-consume` (inbound with correlation), `callback` (long-running).

### 11.2 Event Envelope

All events MUST conform to CloudEvents 1.0 [CloudEvents] with WOS extension attributes: `wosinstanceid`, `wosdefid`, `wosdefversion`, `wosstate`, `wostaskid`, `woscorrelationkey`, `woscausationeventid`.

### 11.3 Idempotency

Event consumption MUST be idempotent. Duplicate events (same CloudEvents `id`) MUST NOT produce duplicate effects.

### 11.4 Correlation

Inbound events matched to running instances via attribute-to-case-file-path mapping. Multiple attributes = logical AND. No match = logged and MAY be queued.

### 11.5 Capability Advertisement via Schema.org Actions

A WOS workflow definition MAY expose its available actions as Schema.org `potentialAction` entries, enabling AI agents and discovery systems to find workflow capabilities using standard vocabulary. The `actionStatus` values (Active, Completed, Failed, Potential) provide a coarse external view for interoperability. Schema.org Actions' four-state model is a derived view and does not replace WOS's internal lifecycle semantics.

### 11.6 Interoperability Protocol Alignment

**Model Context Protocol (MCP)** — for agent-to-tool integration. WOS integration definitions that reference agent tool use SHOULD align with MCP's three-primitive model. The WOS Processor serves as the MCP host, managing agent access within the governance envelope.

**Agent-to-Agent Protocol (A2A)** — for inter-agent and cross-workflow communication. WOS instances coordinating with external agent systems SHOULD expose capabilities as A2A Agent Cards and use A2A's task lifecycle model (including `input-required` for human-in-the-loop).

**SOM and AWP** — the W3C Community Group incubations for Semantic Object Model and Agent Web Protocol are acknowledged as an emerging direction. Extension points are defined for future alignment. Neither is adopted as a normative dependency given incubation status.

WOS does not mandate any interoperability protocol but defines extension points (§19) for protocol-specific bindings.

---

## 12. Layer 7: Provenance and Audit

This section is normative.

### 12.1 Overview

The Provenance layer produces an immutable, tamper-evident record of everything that happens. It observes all other layers but does not participate in control flow.

WOS provenance aligns with W3C PROV-O [PROV-O] and PROV-AGENT [PROV-AGENT]. Provenance records are JSON-LD documents that, when interpreted as RDF, form PROV-compatible graphs. The `@context` maps WOS provenance terms to PROV-O predicates. Agent-specific provenance (model invocations, tool use, MCP interactions) uses PROV-AGENT vocabulary, treating these as first-class nodes in the provenance graph.

### 12.2 Four-Layer Audit Architecture

The provenance model uses a four-layer architecture informed by the empirical finding that model-generated explanations are systematically unfaithful to actual reasoning:

| Layer | Name | Content | Authority |
|-------|------|---------|-----------|
| 1 | Immutable Facts | Timestamp, actor ID/type, model version, exact inputs, exact outputs, policy version, confidence score, reviewer ID. | **Authoritative.** Machine-generated from runtime state. |
| 2 | Structured Reasoning | Policy rules applied, evidence sources consulted, eligibility criteria checked, decision table trace. | **Authoritative** for deterministic logic. Descriptive for agent reasoning. |
| 3 | Generated Narrative | Model's natural language explanation of its reasoning. | **Informational only.** Explicitly labeled as model-generated. NOT authoritative for audit or legal purposes. |
| 4 | Counterfactual | What would need to change for a different outcome. Positive evidence (controllable features) and negative evidence (irrelevant attributes). | Informational. **Required** for adverse decisions in `rights-impacting` workflows. |

Layer 3 records are preserved for informational value and labeled with `wos:explanationFaithfulnessDisclaimer`. They MUST NOT be treated as dispositive evidence of why a decision was made.

### 12.3 Base Provenance Record

Every provenance record MUST include:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | URI | REQUIRED | Globally unique. Maps to `@id`. |
| `timestamp` | datetime | REQUIRED | RFC 3339 with timezone. Maps to `prov:atTime`. |
| `instanceId` | URI | REQUIRED | Workflow instance. |
| `recordType` | enum | REQUIRED | See §12.4–12.9. |
| `actor` | ActorRef | REQUIRED | Maps to `prov:wasAssociatedWith`. |
| `authority` | string | OPTIONAL | Role, rule, or policy. |
| `traceContext` | TraceContext | OPTIONAL | W3C Trace Context. |
| `auditLayer` | integer | REQUIRED | Which layer (1–4). |
| `data` | object | REQUIRED | Record-type-specific payload. |

### 12.4 Record Types

`transition`, `decision`, `agentInvocation`, `agentCheckpoint`, `agentToolUse`, `taskOperation`, `dataMutation`, `override`, `guardrailViolation`, `guardrailBypass`, `autonomyChange`, `modelVersionChange`, `driftAlert`, `dueProcessNotice`, `appealFiled`.

For agent-evaluated decisions, the Decision Record MUST include Layer 1 facts (model ID, version, inputs, outputs, confidence, guardrails evaluated), Layer 2 structured trace (matched rules, parameters used), Layer 3 generated narrative (explicitly labeled informational), Layer 4 counterfactual (required for `rights-impacting`), and review outcome (populated after human review under `assistive`/`supervisory`).

### 12.5 PROV-AGENT Alignment

Agent invocation records use PROV-AGENT vocabulary for model invocations, prompts, tool responses, and MCP interactions as first-class provenance graph nodes. The `@context` maps WOS agent provenance terms to PROV-AGENT predicates. WOS extends PROV-AGENT on structured oversight records (protocol outcomes), due process records, and the four-layer audit architecture's explicit separation of fact from narrative.

Provenance records are JSON-LD. An auditor can load them into a triplestore and run SPARQL queries such as "show every decision where the agent's confidence was below 0.8 and the human reviewer modified the output" across an entire corpus of case histories, without WOS-specific query tooling.

### 12.6 Tamper Evidence

A Full conformant WOS Processor MUST provide tamper evidence. RECOMMENDED: Merkle tree hash-chaining with SHA-256, signed tree heads at configurable intervals (100 records or 60 seconds), inclusion proofs, and consistency proofs.

### 12.7 Process Mining Interoperability

WOS Processors SHOULD emit event data compatible with OCEL 2.0 [OCEL2]. OCEL 2.0's object-centric event model — natively capturing Event-to-Object and Object-to-Object relationships with attribute change tracking — maps more naturally to WOS's case-centric workflows than flat trace models. When provenance records are JSON-LD and OCEL events are mapped into the same RDF graph, process mining becomes graph traversal spanning both definition-level structure and instance-level execution.

IEEE XES [XES] is retained as a secondary compatibility target for legacy process mining tooling.

### 12.8 Provenance Export Packaging

For audit archive export, the RECOMMENDED packaging format is RO-Crate [RO-Crate] using the Workflow Run Crate profile [WfRunCrate]. An RO-Crate provenance export is a self-describing JSON-LD package that maps workflow steps to Schema.org types (`schema:SoftwareApplication` for tools, `schema:CreateAction` for executions), capturing both the prospective plan (the workflow definition) and the retrospective execution facts (the provenance records). Auditors can consume execution history with standard linked-data tooling without WOS-specific software.

---

## 13. Layer 8: Durable Execution Contract

This section is normative.

### 13.1 Durability Guarantees

**G1: Crash Recovery.** Non-terminal instances resume from last durable state after restart.

**G2: Persistent State.** Lifecycle state, case file, task states, timer registrations durably persisted.

**G3: At-Least-Once Execution.** Actions execute at least once. SHOULD be idempotent.

**G4: Durable Timers.** Survive restarts, fire within tolerance, consume no resources while waiting.

**G5: External Signal Delivery.** Signals to inactive instances durably enqueued and delivered when activated.

### 13.2 Retry Policy

`maxAttempts`, `backoff` (`fixed`, `linear`, `exponential`), `initialInterval`, `maxInterval`, `multiplier`, `nonRetryableErrors`.

### 13.3 Timeout Categories

`stepTimeout`, `taskTimeout`, `instanceTimeout`, `heartbeatTimeout`, `queueTimeout`.

### 13.4 Compensation

Activities with side effects SHOULD register compensation handlers. On `compensate` action, handlers execute in reverse completion order.

---

## 14. Actor Model

This section is normative.

### 14.1 Actor Types

| Type | Description | Provenance Requirements |
|------|-------------|------------------------|
| `human` | Person performing tasks, making decisions. | Identity, role, timestamp. |
| `system` | Deterministic software component. | Component identifier, version, timestamp. |
| `agent` | AI system performing reasoning. Outputs non-deterministic, carry confidence. | Model identifier, version, confidence report, input summary, timestamp. All PROV-AGENT fields. |

### 14.2 Normative Constraints

Every provenance record includes actor type and identifier. Agent actors include model version and confidence. Human actors have override authority over agent outputs; agents MUST NOT override human decisions. Actor type is based on decision authority for the specific action. Cascading autonomous agent invocations require explicit declaration with bounded `maxCascadeDepth`.

---

## 15. Expression Language

This section is normative.

The WOS Expression Language is a profile of FEEL (DMN) chosen for readability and executability. It supports literals, paths, arithmetic, comparison, boolean operators, conditionals, membership, string/list/date operations, context construction, and null-safe evaluation. Expressions are pure functions: no side effects, bounded termination. Expression context includes `caseFile`, `event`, `task`, `instance`, `parameters`, `agent`, `env`.

---

## 16. Due Process Requirements

This section is normative for `rights-impacting` and `safety-impacting` workflows.

### 16.1 Adverse Decision Policy

When a workflow produces an adverse decision in a `rights-impacting` workflow, the affected individual MUST receive notice before the decision takes effect, including: the specific determination, the factual basis using individualized reason codes, the right to appeal with deadline and process, and disclosure that an AI system assisted (if applicable).

### 16.2 Explanation Levels

`individualized` (specific factual reasons — REQUIRED for `rights-impacting`), `categorical` (category-level — RECOMMENDED for `operational`), `aggregate` (system-level — minimum for `informational`).

When `counterfactualRequired` is `true`, the explanation MUST include positive counterfactuals (controllable factors that would change the outcome) and negative counterfactuals (irrelevant factors, including protected characteristics, that did NOT affect the outcome).

### 16.3 Appeal Mechanisms

Appeals MUST be reviewed by a human adjudicator independent of the original determination. AI agents MAY assist preparation but MUST NOT serve as the appeal decision-maker. When `continuationOfServices` is `true`, current benefits continue during appeal.

### 16.4 Agent Disclosure

For `rights-impacting` workflows, `discloseThatAgentAssisted: true` MUST be set, consistent with OMB M-24-10 and EU AI Act Art. 13.

---

## 17. Versioning and Evolution

This section is normative.

Semantic Versioning for definitions, decisions, tasks, and agent configurations. Default instance migration: pinned execution. Optional forward migration with safety verification and provenance record. Backward-compatible schema changes (add optional properties, widen constraints, add enum values). Breaking changes increment major version. The `@context` is versioned alongside the specification; breaking `@context` changes increment the spec major version.

---

## 18. Security and Access Control

This section is normative.

Roles: `workflowAdministrator`, `instanceInitiator`, `caseParticipant`, `taskWorker`, `taskAdministrator`, `auditor`. Authorization enforced at: instance creation, task operations, case file access, decision invocation, provenance access, override execution, agent configuration changes, guardrail bypass. Failures logged.

---

## 19. Extensibility

This section is normative.

Extension properties use namespaced prefixes (`x-agency:classification`). Extensions MUST NOT alter core semantics, MUST be preserved during round-trips, MUST NOT cause document rejection, and MUST NOT use the reserved `wos:` prefix.

For JSON-LD, extensions MAY add additional `@context` entries to map extension properties to IRIs. This is the mechanism for domain-specific vocabulary integration (§5.6) and interoperability protocol bindings (§11.6).

---

## 20. Conformance Profiles

This section is normative.

### 20.1 Profile: Structural

Parse and validate WOS Documents against JSON Schema. Produce diagnostics. Round-trip between YAML and JSON-LD. Preserve `@context` and extension properties. Enables editors, validators, linters, migration tools.

### 20.2 Profile: Lifecycle

Structural + execute lifecycle semantics, produce transition records, support all state types, milestones, transition resolution.

### 20.3 Profile: Task Management

Lifecycle + full task lifecycle, all operations, separation of duties, SLA timers, structured oversight protocols.

### 20.4 Profile: Decision

Lifecycle + decision tables with all hit policies, WOS Expression Language, temporal parameters, decision records.

### 20.5 Profile: Agent Governance

Lifecycle + Decision + enforce autonomy levels, produce agent provenance (all types aligned with PROV-AGENT), enforce guardrails with violation records, enforce fallback chains, confidence-based routing, multi-step sessions with checkpoints, prevent agents from overriding human decisions.

### 20.6 Profile: Full

All of Lifecycle + Task Management + Decision + Agent Governance + case file with mutation tracking, integration semantics with correlation, all provenance record types, tamper evidence, durable execution guarantees, access control, due process requirements for `rights-impacting` workflows.

### 20.7 Profile: Verification

Structural + static soundness analysis (deadlock-freedom, livelock-freedom, proper termination, dead elements, fallback completeness), diagnostic reports. SHOULD support simulation.

### 20.8 Profile: Semantic

Structural + produce valid JSON-LD documents with correct `@context`, support SHACL-based validation of workflow definitions against structural governance shapes (Appendix B), support SHACL-based guardrail enforcement as an alternative to expression-based enforcement, emit provenance records that are valid PROV-O/PROV-AGENT graphs when interpreted as JSON-LD, support SPARQL querying over provenance records, and support OCEL 2.0 event emission.

This profile enables advanced governance (portfolio-wide structural queries such as "which workflows allow autonomous agents on rights-impacting decisions"), cross-system provenance querying, linked-data interoperability, and formal constraint validation.

---

## 21. Privacy Considerations

This section is informative.

Case data may be subject to privacy regulations. The `visibility` model restricts access. `redactFields` limits agent data exposure. Provenance records may contain personal data; implementations SHOULD support configurable retention and anonymization. The claim check pattern keeps documents in access-controlled storage. The right to erasure may conflict with audit requirements; implementations SHOULD support redaction while preserving audit integrity.

---

## 22. Security Considerations

This section is informative.

1. **Expression sandboxing.** WOS Expressions evaluated in isolated context with no system access or side effects.
2. **Event authentication.** External events SHOULD be signature-verified.
3. **Provenance integrity.** Signed tree heads stored independently from provenance records.
4. **Case file encryption.** Data at rest and in transit SHOULD be encrypted.
5. **Separation of duties enforcement.** Constraints enforced at application layer, not solely UI.
6. **Agent impersonation.** Agents authenticated with same rigor as humans. Agents MUST NOT claim human identity.
7. **Prompt injection defense.** `isolateUntrustedData` implements the CaMeL dual-LLM pattern. Guardrails provide structural defense regardless of agent compromise. No single defense is sufficient; defense in depth is required.
8. **Model version drift.** Implementations using non-pinned version policies SHOULD monitor output distributions. Shadow deployment RECOMMENDED before production changes.
9. **Cascading autonomy.** `cascadingAutonomy` declaration and `maxCascadeDepth` make chains visible and bounded.
10. **Tool use boundaries.** Agent tool permissions follow least-privilege. Invocations recorded. Side-effecting tools at autonomous level require explicit policy.
11. **JSON-LD context integrity.** The `@context` document MUST be served over HTTPS from the specification's canonical domain. Implementations SHOULD cache the context document and verify its integrity. A compromised `@context` could cause misinterpretation of WOS terms when processed as RDF.

---

## 23. References

### 23.1 Normative References

**[RFC2119]** Bradner, S., "Key words for use in RFCs to Indicate Requirement Levels", BCP 14, RFC 2119, March 1997.

**[RFC3339]** Klyne, G. and C. Newman, "Date and Time on the Internet: Timestamps", RFC 3339, July 2002.

**[RFC3986]** Berners-Lee, T., et al., "Uniform Resource Identifier (URI): Generic Syntax", RFC 3986, January 2005.

**[RFC8174]** Leiba, B., "Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words", BCP 14, RFC 8174, May 2017.

**[RFC8259]** Bray, T., "The JavaScript Object Notation (JSON) Data Interchange Format", RFC 8259, December 2017.

**[YAML]** Ben-Kiki, O., Evans, C., and I. döt Net, "YAML Ain't Markup Language Version 1.2", October 2021.

**[JSON-LD11]** Sporny, M., Longley, D., Kellogg, G., Lanthaler, M., and N. Lindström, "JSON-LD 1.1", W3C Recommendation, July 2020.

**[RDF11]** Cyganiak, R., Wood, D., and M. Lanthaler, "RDF 1.1 Concepts and Abstract Syntax", W3C Recommendation, February 2014.

**[SHACL]** Knublauch, H. and D. Kontokostas, "Shapes Constraint Language (SHACL)", W3C Recommendation, July 2017.

**[PROV-O]** Lebo, T., Sahoo, S., and D. McGuinness, "PROV-O: The PROV Ontology", W3C Recommendation, April 2013.

**[PROV-DM]** Moreau, L. and P. Missier, "PROV-DM: The PROV Data Model", W3C Recommendation, April 2013.

**[SemVer]** Preston-Werner, T., "Semantic Versioning 2.0.0".

**[ISO8601]** ISO, "ISO 8601:2019 Date and time — Representations for information interchange".

**[CloudEvents]** CNCF, "CloudEvents Specification Version 1.0.2", 2022.

**[TraceContext]** W3C, "Trace Context", W3C Recommendation, February 2020.

**[DMN]** OMG, "Decision Model and Notation Version 1.4", March 2021.

**[OpenAPI]** Linux Foundation, "OpenAPI Specification Version 3.1.0", February 2021.

**[AsyncAPI]** AsyncAPI Initiative, "AsyncAPI Specification Version 3.0", 2023.

**[JSONSchema]** IETF, "JSON Schema: A Media Type for Describing JSON Documents", draft-bhutton-json-schema-01, 2022.

### 23.2 Informative References

**[Harel1987]** Harel, D., "Statecharts: A Visual Formalism for Complex Systems", Science of Computer Programming, 8(3), pp. 231–274, 1987.

**[SCXML]** W3C, "State Chart XML (SCXML)", W3C Recommendation, September 2015.

**[BPMN]** OMG, "Business Process Model and Notation Version 2.0", ISO/IEC 19510:2013.

**[CMMN]** OMG, "Case Management Model and Notation Version 1.1", December 2016.

**[WS-HumanTask]** OASIS, "Web Services – Human Task Version 1.1", August 2012.

**[XACML]** OASIS, "eXtensible Access Control Markup Language Version 3.0", January 2013.

**[Sagas]** Garcia-Molina, H. and Salem, K., "Sagas", ACM SIGMOD, 1987.

**[WorkflowPatterns]** van der Aalst, W.M.P., et al., "Workflow Patterns", Distributed and Parallel Databases, 14(1), pp. 5–51, 2003.

**[XES]** IEEE, "IEEE Standard for eXtensible Event Stream (XES)", IEEE Std 1849-2016.

**[OCEL2]** van der Aalst, W.M.P. et al., "Object-Centric Event Log (OCEL) 2.0", 2023.

**[RO-Crate]** Soiland-Reyes, S., et al., "Packaging research artefacts with RO-Crate", Data Science, 5(2), 2022.

**[WfRunCrate]** Leo, S., et al., "Recording provenance of workflow runs with RO-Crate", PLOS ONE, 2024.

**[PROV-AGENT]** Extension of W3C PROV-O for agentic workflows.

**[RFC9162]** Laurie, B., et al., "Certificate Transparency Version 2.0", RFC 9162, December 2021.

**[NIST-SP-800-53]** NIST, "Security and Privacy Controls for Information Systems and Organizations", SP 800-53 Rev 5, September 2020.

**[OMB-M-24-10]** OMB, "Advancing Governance, Innovation, and Risk Management for Agency Use of Artificial Intelligence", M-24-10, March 2024.

**[EU-AI-Act]** European Parliament, "Regulation (EU) 2024/1689 on Artificial Intelligence", June 2024.

**[NIST-AI-RMF]** NIST, "AI Risk Management Framework (AI RMF 1.0)", AI 100-1, January 2023.

**[Vaccaro2024]** Vaccaro, M., Almaatouq, A., and Malone, T., "When combinations of humans and AI are useful", Nature Human Behaviour, 8, pp. 2287–2297, 2024.

**[Turpin2023]** Turpin, M., et al., "Language Models Don't Always Say What They Think", NeurIPS 2023.

**[Lanham2023]** Lanham, T., et al., "Measuring Faithfulness in Chain-of-Thought Reasoning", Anthropic, 2023.

**[Chen2023]** Chen, L., Zaharia, M., and Zou, J., "How Is ChatGPT's Behavior Changing over Time?", 2023.

**[Buçinca2021]** Buçinca, Z., Malaya, M.B., and Gajos, K.Z., "To Trust or to Think: Cognitive Forcing Functions Can Reduce Overreliance on AI", CSCW 2021.

**[Nasr2025]** Nasr, M., et al., "The Attacker Moves Second", 2025.

**[CaMeL]** Debenedetti, E., et al., "CaMeL: Causal Mediation for LLM Defense", Google DeepMind, 2025.

**[FormalLLM]** Li, Y., et al., "Formal-LLM: Integrating Formal Language and Natural Language for Controllable LLM-based Agents", EMNLP 2024.

**[ABC]** "Agent Behavioral Contracts: Formal Specification and Runtime Enforcement", February 2026.

**[Feng2025]** Feng, Y., McDonald, A., and Zhang, B., "Levels of Autonomy for AI Agents", Knight First Amendment Institute, Columbia University, 2025.

**[Wachter2018]** Wachter, S., Mittelstadt, B., and Russell, C., "Counterfactual Explanations Without Opening the Black Box", Harvard Journal of Law & Technology, 31(2), 2018.

**[MCP]** Anthropic, "Model Context Protocol Specification", 2024.

**[A2A]** Google, "Agent-to-Agent Protocol", Linux Foundation, 2025.

**[BBO]** BPMN Based Ontology. Process ontology research for semantic process models.

**[SchemaActions]** Schema.org, "Action" type and related vocabulary. https://schema.org/Action

**[SOM]** W3C Web Content Browser for AI Agents Community Group, "Semantic Object Model", incubation.

**[AWP]** W3C Web Content Browser for AI Agents Community Group, "Agent Web Protocol", incubation.

---

## Appendix A. JSON-LD Context Document

This appendix is normative. The following is the core structure of the WOS JSON-LD `@context` document published at `https://wos-spec.org/context/3.0.0`. The complete document is maintained at the specification repository.

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

    "id": "@id",
    "type": "@type",

    "name": "schema:name",
    "description": "schema:description",
    "version": "schema:version",
    "created": { "@id": "schema:dateCreated", "@type": "xsd:dateTime" },
    "modified": { "@id": "schema:dateModified", "@type": "xsd:dateTime" },
    "authors": "schema:author",
    "status": "wos:status",

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
    "condition": "wos:condition",

    "decisions": { "@id": "wos:decisions", "@container": "@index" },
    "inputs": { "@id": "wos:inputs", "@container": "@list" },
    "outputs": { "@id": "wos:outputs", "@container": "@list" },
    "logic": "wos:logic",
    "hitPolicy": "wos:hitPolicy",
    "rules": { "@id": "wos:rules", "@container": "@list" },
    "overrides": { "@id": "wos:overrides", "@type": "@id" },
    "effectiveDate": { "@id": "wos:effectiveDate", "@type": "xsd:date" },
    "sunsetDate": { "@id": "wos:sunsetDate", "@type": "xsd:date" },

    "parameters": { "@id": "wos:parameters", "@container": "@index" },

    "tasks": { "@id": "wos:tasks", "@container": "@index" },
    "form": "wos:form",
    "inputSchema": "wos:inputSchema",
    "outputSchema": "wos:outputSchema",
    "assignment": "wos:assignment",
    "potentialOwners": "wos:potentialOwners",
    "excludedOwners": "wos:excludedOwners",
    "businessAdministrators": "wos:businessAdministrators",
    "roles": "wos:roles",
    "skills": "wos:skills",
    "sla": "wos:sla",
    "dueIn": "wos:dueIn",
    "separation": "wos:separation",
    "oversight": "wos:oversight",
    "protocol": "wos:oversightProtocol",

    "agents": { "@id": "wos:agents", "@container": "@index" },
    "model": "wos:model",
    "provider": "wos:provider",
    "identifier": "wos:identifier",
    "versionPolicy": "wos:versionPolicy",
    "capabilities": { "@id": "wos:capabilities", "@container": "@list" },
    "defaultAutonomy": "wos:defaultAutonomy",
    "autonomy": "wos:autonomyLevel",
    "autonomyPolicy": "wos:autonomyPolicy",
    "maxAutonomy": "wos:maxAutonomy",
    "guardrails": "wos:guardrails",
    "confidenceFloor": "wos:confidenceFloor",
    "threshold": "wos:threshold",
    "onViolation": "wos:onViolation",
    "fallback": "wos:fallback",
    "inputPreparation": "wos:inputPreparation",
    "monitoring": "wos:monitoring",
    "confidence": "wos:confidence",
    "overall": "wos:overallConfidence",
    "method": "wos:confidenceMethod",
    "calibrationStatus": "wos:calibrationStatus",

    "caseFile": "wos:caseFile",
    "items": { "@id": "wos:items", "@container": "@index" },
    "multiplicity": "wos:multiplicity",
    "visibility": "wos:visibility",
    "vocabulary": { "@id": "wos:vocabulary", "@type": "@id" },

    "integrations": { "@id": "wos:integrations", "@container": "@index" },
    "correlation": "wos:correlation",

    "provenance": "wos:provenanceConfig",
    "execution": "wos:executionConfig",
    "dueProcess": "wos:dueProcess",

    "impactLevel": "wos:impactLevel",
    "jurisdiction": "wos:jurisdiction",
    "authority": "dcterms:authority",

    "timestamp": { "@id": "prov:atTime", "@type": "xsd:dateTime" },
    "actor": { "@id": "prov:wasAssociatedWith", "@type": "@id" },
    "wasGeneratedBy": { "@id": "prov:wasGeneratedBy", "@type": "@id" },
    "wasDerivedFrom": { "@id": "prov:wasDerivedFrom", "@type": "@id" },
    "recordType": "wos:recordType",
    "auditLayer": "wos:auditLayer",
    "instanceId": { "@id": "wos:instanceId", "@type": "@id" }
  }
}
```

---

## Appendix B. SHACL Shapes for Structural Governance

This appendix is normative. The following SHACL shapes enforce policy-level constraints that JSON Schema cannot express. The complete shape library is maintained at the specification repository.

```turtle
@prefix wos: <https://wos-spec.org/ns/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

# Shape: Rights-impacting workflows must have due process with appeals
wos:RightsImpactingDueProcessShape
  a sh:NodeShape ;
  sh:targetClass wos:WorkflowDefinition ;
  sh:property [
    sh:path wos:impactLevel ;
    sh:hasValue "rights-impacting" ;
  ] ;
  sh:property [
    sh:path ( wos:dueProcess wos:appealMechanism wos:enabled ) ;
    sh:hasValue true ;
    sh:message "Rights-impacting workflows MUST include an enabled appeal mechanism." ;
    sh:severity sh:Violation ;
  ] .

# Shape: Autonomous agent actions must have guardrails
wos:AutonomousGuardrailShape
  a sh:PropertyShape ;
  sh:path wos:autonomyLevel ;
  sh:sparql [
    sh:message "Autonomous agent actions MUST have associated guardrail definitions." ;
    sh:select """
      SELECT $this
      WHERE {
        $this wos:autonomyLevel "autonomous" .
        FILTER NOT EXISTS { $this wos:guardrails ?g }
      }
    """ ;
  ] .

# Shape: Agent invocations must have fallback to human
wos:AgentFallbackShape
  a sh:NodeShape ;
  sh:targetSubjectsOf wos:agentRef ;
  sh:sparql [
    sh:message "Every agent invocation MUST have a fallback chain terminating in a human task." ;
    sh:select """
      SELECT $this
      WHERE {
        $this wos:agentRef ?agent .
        ?agent wos:fallback ?fb .
        FILTER NOT EXISTS {
          ?fb wos:terminal/wos:onFailure "escalateToHuman"
        }
      }
    """ ;
  ] .

# Shape: Tasks with agent assistance must specify oversight protocol
wos:OversightProtocolShape
  a sh:NodeShape ;
  sh:targetSubjectsOf wos:agentAssistance ;
  sh:property [
    sh:path ( wos:oversight wos:oversightProtocol ) ;
    sh:minCount 1 ;
    sh:message "Tasks with agent assistance MUST specify an oversight protocol." ;
    sh:severity sh:Violation ;
  ] .

# Shape: Rights-impacting workflows must not have autonomous agents
# unless explicitly elevated
wos:RightsImpactingAutonomyShape
  a sh:NodeShape ;
  sh:targetClass wos:WorkflowDefinition ;
  sh:sparql [
    sh:message "Rights-impacting workflows MUST NOT use autonomous agents without explicit elevation." ;
    sh:select """
      SELECT $this
      WHERE {
        $this wos:impactLevel "rights-impacting" .
        $this wos:lifecycle/wos:states/wos:onEntry/wos:autonomyLevel "autonomous" .
        FILTER NOT EXISTS {
          $this wos:lifecycle/wos:states/wos:onEntry/wos:autonomyElevation ?elev
        }
      }
    """ ;
  ] .
```

---

## Appendix C. Complete Example

This appendix is informative. A complete WOS v3 Document for a Community Development Block Grant review workflow — demonstrating all eight layers, JSON-LD serialization with `@context`, NIEM vocabulary references in case data, agent governance with guardrails, structured oversight with `independentFirst`, defeasible eligibility rules with temporal parameters, parallel technical reviews with separation of duties, PROV-AGENT-aligned provenance, and due process protections — is maintained at the specification repository and published separately as the **WOS Reference Workflow**.

---

## Appendix D. Relationship to Existing Standards

| Standard | Relationship |
|----------|-------------|
| **JSON-LD 1.1** | WOS documents are natively JSON-LD. The `@context` maps WOS terms to standard vocabularies. No transformation required. |
| **RDF 1.1** | Every WOS document is an RDF graph by virtue of JSON-LD serialization. |
| **SHACL** | Shapes validate guardrails, workflow definitions, and structural governance. Dual representation with WOS Expressions. |
| **PROV-O / PROV-AGENT** | Provenance records align with PROV-O; agent provenance uses PROV-AGENT vocabulary for model invocations and tool use. |
| **OCEL 2.0** | Primary process mining interoperability target. Object-centric event model for case-centric workflows. |
| **RO-Crate** | Recommended audit archive packaging. Workflow Run Crate profile for self-describing provenance exports. |
| **Schema.org** | Case data and capability advertisement map to Schema.org types. `potentialAction` for capability discovery. |
| **BBO / sBPMN** | Process ontology terms evaluated as `@context` candidates. WOS arrives at sBPMN's destination natively. |
| **BPMN 2.0** | WOS adopts event taxonomy; replaces flowchart topology with statecharts. |
| **CMMN 1.1** | WOS adopts case file model, discretionary items, milestones. |
| **DMN 1.4** | WOS adopts decision tables, hit policies, FEEL profile; extends with defeasible rules and temporal parameters. |
| **SCXML 1.0** | WOS adopts statechart semantics; replaces XML with JSON-LD. |
| **WS-HumanTask 1.1** | WOS adopts task lifecycle and roles; adds structured oversight. |
| **CloudEvents 1.0** | Event envelope with WOS extension attributes. |
| **OMB M-24-10** | WOS operationalizes rights-impacting AI requirements as workflow constraints. |
| **EU AI Act** | WOS provenance, transparency, oversight, and monitoring satisfy high-risk AI obligations. |
| **NIST AI RMF** | WOS layers map to GOVERN, MAP, MEASURE, MANAGE functions. |
| **MCP** | Extension points for tool integration within governance envelope. |
| **A2A** | Extension points for inter-agent communication. |
| **NIEM** | Domain vocabulary integration via `@context` extension for government case data. |
| **FHIR** | Domain vocabulary integration via `@context` extension for health-related case data. |

---

## Appendix E. Changelog

| Date | Version | Description |
|------|---------|-------------|
| 2026-04-09 | 3.0.0 | JSON-LD native serialization with normative `@context`. SHACL for guardrails and structural governance. PROV-AGENT provenance alignment. OCEL 2.0 process mining. RO-Crate audit packaging. Schema.org capability advertisement. Semantic Conformance Profile. Domain vocabulary integration via `@context` extension. |
| 2026-04-08 | 2.0.0 | Eight-layer architecture with Agent Governance. Four-layer audit model. Structured oversight. Due process. Research-informed design. |
