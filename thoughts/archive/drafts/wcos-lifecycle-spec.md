# Workflow Case Orchestration Standard (WCOS)
# Layer 1: Lifecycle and Topology

**W3C Editor's Draft, 08 April 2026**

---

**This version:**
: https://w3c.github.io/wcos/lifecycle/ED-wcos-lifecycle-20260408

**Latest published version:**
: https://w3c.github.io/wcos/lifecycle/

**Editors:**
: _[Editor names TBD]_

**Abstract:**
: This specification defines the Lifecycle and Topology layer of the Workflow Case Orchestration Standard (WCOS). It provides a formal model for defining the lifecycle structure of long-running, human-in-the-loop workflows using hierarchical state machines derived from Harel statecharts. The model supports nested states, parallel regions, guarded transitions, history states, milestone-gated progression, deferred choice, compensation boundaries, and structured exception handling. The specification defines both an abstract data model and a canonical YAML/JSON serialization optimized for machine validation, AI-assisted authoring, and human readability.

**Status of This Document:**
: This is an Editor's Draft. It has no official standing. Feedback is welcome via the public mailing list or GitHub issues.

**Copyright:**
: This document is licensed under the [W3C Software and Document License](https://www.w3.org/copyright/software-license-2023/).

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Conformance](#2-conformance)
3. [Terminology](#3-terminology)
4. [Design Principles](#4-design-principles)
5. [Data Model](#5-data-model)
6. [State Semantics](#6-state-semantics)
7. [Transition Semantics](#7-transition-semantics)
8. [Parallel Regions](#8-parallel-regions)
9. [History States](#9-history-states)
10. [Milestones](#10-milestones)
11. [Timers and Temporal Behavior](#11-timers-and-temporal-behavior)
12. [Exception Boundaries and Compensation](#12-exception-boundaries-and-compensation)
13. [Deferred Choice](#13-deferred-choice)
14. [Cancellation](#14-cancellation)
15. [Invocations and Integration Points](#15-invocations-and-integration-points)
16. [Data Model Binding](#16-data-model-binding)
17. [Execution Semantics](#17-execution-semantics)
18. [Static Analysis Requirements](#18-static-analysis-requirements)
19. [Serialization](#19-serialization)
20. [JSON Schema Vocabulary](#20-json-schema-vocabulary)
21. [Conformance Test Suite Structure](#21-conformance-test-suite-structure)
22. [Security and Privacy Considerations](#22-security-and-privacy-considerations)
23. [Relationship to Other WCOS Layers](#23-relationship-to-other-wcos-layers)

**Appendices**

- [A. Complete YAML Examples](#appendix-a-complete-yaml-examples)
- [B. JSON Schema (Normative)](#appendix-b-json-schema-normative)
- [C. Mapping to SCXML Semantics](#appendix-c-mapping-to-scxml-semantics)
- [D. Mapping to BPMN Concepts](#appendix-d-mapping-to-bpmn-concepts)
- [E. Workflow Pattern Coverage](#appendix-e-workflow-pattern-coverage)
- [F. References](#appendix-f-references)

---

## 1. Introduction

### 1.1 Purpose

This specification defines Layer 1 of the Workflow Case Orchestration Standard (WCOS): the **Lifecycle and Topology** layer. This layer provides the structural backbone of a workflow definition — the states a case or process instance can occupy, the transitions between them, and the rules governing progression.

Layer 1 is deliberately scoped to **lifecycle structure**. It defines *where* a workflow can be and *how* it moves between positions. It does not define *what decisions are made* (Layer 2: Decision and Policy), *who performs work* (Layer 3: Human Task and Work Management), or *what data accumulates* (Layer 4: Case State and Evidence). Those concerns are addressed by companion specifications and connected to this layer through well-defined integration points.

### 1.2 Motivation

Existing process standards fall into two camps. Flowchart-based standards (BPMN, BPEL) model control flow as directed graphs connecting activities. They excel at sequential and parallel routing but struggle with long-running case lifecycles, suspension and resumption, discretionary branching, and the kind of ad-hoc, evidence-driven progression that characterizes high-stakes government workflows. State-machine standards (SCXML, UML State Machines) model lifecycle phases and transitions with formal precision but lack first-class concepts for milestones, compensation, integration points, and human-task coordination.

This specification combines the formal rigor of Harel statecharts with the pragmatic workflow concepts proven in CMMN, BPMN, and the workflow patterns literature. It produces a lifecycle model that is:

- **Formally verifiable** — soundness properties (deadlock-freedom, livelock-freedom, proper termination) are decidable for the core language.
- **AI-generatable** — the YAML serialization and constrained object model are designed for reliable LLM generation and round-trip editing.
- **Human-comprehensible** — hierarchical states map naturally to how domain experts describe case lifecycles ("the application is in Review, specifically in the Financial Assessment sub-phase").
- **Runtime-portable** — the specification defines abstract execution semantics without mandating implementation mechanisms.

### 1.3 Scope

This specification defines:

- The abstract data model for lifecycle definitions (states, transitions, regions, milestones, timers, exception boundaries, invocations).
- The execution semantics governing state entry, exit, transition selection, and event processing.
- The canonical YAML/JSON serialization format.
- The JSON Schema vocabulary for structural validation.
- Static analysis requirements for soundness checking.
- Conformance levels and test suite structure.

This specification does NOT define:

- Decision logic or routing rules (see WCOS Layer 2).
- Human task lifecycle, assignment, or work queues (see WCOS Layer 3).
- Case data schemas or evidence management (see WCOS Layer 4).
- Event envelope formats or integration protocols (see WCOS Layer 5).
- Audit record structures or provenance (see WCOS Layer 6).
- Durable execution mechanisms or runtime infrastructure (see WCOS Layer 7).
- Visual notation or diagram interchange format.
- Form rendering or UI specification.

### 1.4 Relationship to Harel Statecharts and SCXML

This specification adopts the semantic foundation of Harel statecharts [HAREL87] as formalized by the W3C State Chart XML (SCXML) specification [SCXML]. The core state machine semantics — hierarchical states, parallel regions, transitions with events and guards, entry/exit actions, history states, and event processing — are drawn from SCXML with the following adaptations:

1. **Serialization**: YAML/JSON replaces XML as the canonical format.
2. **Expression language**: FEEL [DMN] replaces ECMAScript/XPath as the guard and expression language.
3. **Extensions**: Milestones, compensation boundaries, durable timers, invocation contracts, and cancellation regions are added as first-class constructs not present in SCXML.
4. **Restrictions**: The `<script>` element and arbitrary executable content blocks from SCXML are excluded. Computation is delegated to invocations and the Decision layer.
5. **Event model**: The internal event queue and external event queue semantics of SCXML are preserved, with CloudEvents [CE] as the external event envelope format.

Where this specification is silent on a semantic question, SCXML's Algorithm for SCXML Interpretation [SCXML §D] is the normative reference.

### 1.5 Conventions

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [RFC2119] and [RFC8174].

The term "implementation" refers to any software system that processes WCOS Layer 1 lifecycle definitions, whether for validation, execution, simulation, or analysis.

---

## 2. Conformance

### 2.1 Conformance Classes

This specification defines four conformance classes. An implementation MAY conform to one or more classes.

**Class 1: Document Conformance.** A WCOS Lifecycle document conforms to this specification if it satisfies all structural constraints defined in [§19 Serialization](#19-serialization) and validates against the JSON Schema defined in [Appendix B](#appendix-b-json-schema-normative).

**Class 2: Validator Conformance.** A WCOS Lifecycle Validator conforms to this specification if it correctly accepts all