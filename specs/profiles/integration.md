---
title: WOS Integration Profile
version: 1.0.0-draft.1
date: 2026-04-09
status: draft
---

# WOS Integration Profile v1.0

**Version:** 1.0.0-draft.1
**Date:** 2026-04-09
**Editors:** Formspec Working Group
**Companion to:** WOS Kernel Specification v1.0

---

## Abstract

The WOS Integration Profile is a parallel seam specification for the Workflow Orchestration Standard (WOS). An Integration Profile Document -- itself a JSON document -- declares named integration bindings that WOS workflows invoke through the kernel's `invokeService` action: Arazzo sequences for multi-step API orchestration, CWL-informed tool descriptors for non-HTTP invocations, CloudEvents extension attributes for event interoperability, and an external policy engine bridge for delegating governance decisions to XACML, OPA, or Cedar engines. Each integration binding declares interface references, input/output mappings, retry policies, and optional Formspec Definition contracts for request and response validation.

The Integration Profile is a parallel seam -- it attaches at any layer and does not introduce new kernel seams. A WOS workflow functions without an Integration Profile Document. The profile provides patterns for the existing `invokeService` action, `emitEvent` action, and `extensions` mechanism.

WOS MUST NOT alter core Formspec processing semantics. WOS processors MUST delegate Formspec Definition evaluation to a Formspec-conformant processor (Core S1.4).

---

## Status of This Document

This document is a **draft specification**. It is a parallel seam profile within the Workflow Orchestration Standard, a companion framework to Formspec v1.0 that does not modify Formspec's processing model. Implementors are encouraged to experiment with this specification and provide feedback, but MUST NOT treat it as stable for production use until a 1.0.0 release is published.

---

## 1. Introduction

### 1.1 Background

WOS workflows interact with external systems: invoking APIs, executing command-line tools, consuming and producing events, and delegating policy decisions. The kernel provides the `invokeService` action (Kernel S9.2), the `emitEvent` action, and the `extensions` mechanism. This profile standardizes the integration patterns that use those kernel primitives.

Without this profile, a WOS workflow still invokes services -- the `serviceRef` in an `invokeService` action is opaque to the kernel. The Integration Profile adds structure to that reference: typed integration bindings with interface contracts, input/output mappings, retry policies, resource constraints, and validation via Formspec Definition contracts.

### 1.2 Design Goals

1. **Patterns, not primitives.** This profile does not add new kernel seams. It provides structured patterns for the existing `invokeService`, `emitEvent`, and `extensions` mechanisms.
2. **Standard envelope, open content.** Event interoperability uses CloudEvents 1.0 with WOS-specific extension attributes. API orchestration references Arazzo documents. Tool descriptors follow CWL patterns. The profile standardizes the envelope; the external system's content is its own.
3. **Contract validation.** Every integration binding MAY declare Formspec Definition contracts for request and response validation. When declared, the WOS Processor MUST delegate validation to a Formspec-conformant processor (Core S1.4) and ingest the resulting ValidationReport as a provenance record.
4. **Policy delegation.** External policy engines (XACML, OPA, Cedar) are invocable as integration bindings. The bridge serializes the WOS evaluation context, calls the engine, and maps the result to a governance decision.

### 1.3 Scope

**Within scope:** integration binding types (Arazzo sequence, CWL-informed tool, request-response, event-emit, event-consume, callback, policy-engine); CloudEvents extension attributes for WOS events; input/output mappings; retry and timeout policies; Formspec Definition contract references for request and response validation; external policy engine bridge; correlation; idempotency; conformance profiles.

**Out of scope:** lifecycle topology, case state, actor model (Kernel). Due process, review protocols (Workflow Governance). Agent registration, deontic constraints (AI Integration). The transport mechanism for event delivery (implementation-defined). The internal semantics of referenced Arazzo documents, CWL tools, or policy engine rule sets.

### 1.4 Relationship to the Kernel

The Integration Profile targets a WOS Kernel Document via `targetWorkflow`. Every integration binding declared in this profile is invoked through the kernel's `invokeService` action (Kernel S9.2) by referencing the binding's key as the `serviceRef` value. Event-producing bindings are invoked through the kernel's `emitEvent` action.

The profile does not require any governance layer. A kernel-only workflow with an Integration Profile Document is valid.

### 1.5 Notational Conventions

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [BCP 14][RFC 2119] [RFC 8174] when, and only when, they appear in ALL CAPITALS, as shown here.

JSON syntax and data types are as defined in [RFC 8259]. URI syntax is as defined in [RFC 3986]. Duration and date-time values use ISO 8601 syntax [ISO 8601].

Terms defined in the Formspec v1.0 core specification -- including *Definition*, *Item*, *Bind*, *FEL*, and *conformant processor* -- retain their core-specification meanings throughout this document unless explicitly redefined.

Additional terms:

- **Integration Profile Document** -- A JSON document conforming to this specification. Declares named integration bindings for a WOS workflow.
- **Integration Binding** -- A named declaration of an external system interaction, including its type, interface reference, input/output mappings, and optional contracts.
- **Arazzo Sequence** -- A multi-step API orchestration workflow defined by an Arazzo document (OpenAPI Initiative). Referenced by URI.
- **CWL-Informed Tool** -- A non-HTTP invocation descriptor following the structural pattern of CWL's `CommandLineTool`, adapted for WOS. WOS does not require a CWL-conformant processor.
- **Policy Engine Bridge** -- An integration binding that delegates a governance decision to an external policy engine (XACML, OPA, Cedar) and maps the result to a WOS governance outcome.
- **WOS CloudEvents Extension** -- A CloudEvents extension attribute prefixed with `wos` that carries WOS-specific context in event envelopes.

[rfc2119]: https://www.rfc-editor.org/rfc/rfc2119
[RFC 3986]: https://www.rfc-editor.org/rfc/rfc3986
[RFC 8174]: https://www.rfc-editor.org/rfc/rfc8174
[RFC 8259]: https://www.rfc-editor.org/rfc/rfc8259

---

## 2. Conformance

### 2.1 Conformance Classes

**Integration Profile Document.** A serialized integration profile conforming to the structural and semantic requirements of this specification.

**Integration Profile Processor.** A software system that consumes Integration Profile Documents and executes integration bindings in the context of a WOS workflow.

### 2.2 Conformance Profiles

This specification defines two conformance profiles:

| Profile | Requirements |
|---------|-------------|
| **Core** | MUST support `request-response`, `event-emit`, `event-consume`, and `callback` integration binding types. MUST produce CloudEvents 1.0 envelopes with the required WOS extension attributes (S5). MUST support Formspec Definition contract validation when declared (S4). |
| **Complete** | Core, plus: MUST support `arazzo-sequence` and `tool` binding types. MUST support `policy-engine` binding type. MUST support correlation (S6) and idempotency (S7). |

Complete is a strict superset of Core.

---

## 3. Integration Bindings

### 3.1 Overview

Integration bindings are declared under the `bindings` property of the Integration Profile Document. Each binding has a unique key and declares a typed external system interaction.

```json
{
  "$wosIntegrationProfile": "1.0",
  "targetWorkflow": {
    "url": "https://agency.gov/workflows/benefits-adjudication",
    "compatibleVersions": ">=1.0.0 <2.0.0"
  },
  "bindings": {
    "eligibilityCheck": {
      "type": "arazzo-sequence",
      "arazzoRef": "urn:agency.gov:arazzo:eligibility-check:1.0.0",
      "inputMapping": {
        "applicantSSN": "caseFile.application.ssn",
        "householdSize": "caseFile.application.householdSize"
      },
      "outputBinding": {
        "caseFile.eligibility.result": "$.steps.eligibility.output"
      }
    }
  }
}
```

### 3.2 Integration Binding Types

| Type | Description |
|------|-------------|
| `request-response` | Synchronous invocation of an external service. Interface defined by an OpenAPI reference. |
| `event-emit` | Production of an outbound CloudEvents 1.0 event. |
| `event-consume` | Subscription to inbound events from external sources, with correlation. |
| `callback` | Long-running external interaction: the workflow sends a request and later receives a callback event with the result. |
| `arazzo-sequence` | Multi-step API orchestration sequence. References an Arazzo document (OpenAPI Initiative). |
| `tool` | Non-HTTP invocation informed by CWL's `CommandLineTool` descriptor pattern. |
| `policy-engine` | External policy engine invocation (XACML, OPA, or Cedar). |

### 3.3 Common Binding Properties

All integration binding types share the following properties:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `type` | string | REQUIRED | The integration binding type (one of the values in S3.2). |
| `description` | string | OPTIONAL | Human-readable description of this integration binding. |
| `requestContract` | object | OPTIONAL | Formspec Definition contract for request validation (S4). |
| `responseContract` | object | OPTIONAL | Formspec Definition contract for response validation (S4). |
| `retry` | object | OPTIONAL | Retry policy (S3.8). |
| `timeout` | string (ISO 8601 duration) | OPTIONAL | Maximum time to wait for a response. |
| `idempotencyKeyExpression` | string (FEL) | OPTIONAL | FEL expression evaluated against the case state to produce an idempotency key. Maps to the kernel's `idempotencyKey` (Kernel S9.3). |
| `extensions` | object | OPTIONAL | Extension data. Property names MUST begin with `x-`. |

### 3.3.1 outputBinding JSONPath Profile

`outputBinding` values are JSON Path expressions into the service response. This specification pins an explicit **RFC 9535 subset** for all `outputBinding` path expressions.

**Supported constructs:**

- **Member access** — `.key`, `['key']`, `["key"]` (including quoted keys with backslash-escape)
- **Index** — `[n]` (zero-based non-negative integer)
- **Wildcard** — `[*]` (fans out over all array elements or object values; subsequent segments apply to each element and results are collected into an array)
- **Slice** — `[start:end]` and `[start:end:step]` (Python-style; negative indices count from the end; open bounds are allowed via `[start:]`, `[:end]`, `[::step]`)

**Excluded constructs:**

- **Recursive descent** — `..` (RFC 9535 §2.5) is NOT supported. Rationale: recursive descent can match nodes at unpredictable depths, making provenance records non-deterministic and complicating replay verification.
- **Filter expressions** — `[?(...)]` (RFC 9535 §2.6) are NOT supported. Rationale: filter expressions introduce a second expression language (distinct from FEL) inside binding documents, making static analysis and lint-time validation significantly harder.

**Enforcement:** A WOS processor MUST reject any Integration Profile Document whose `outputBinding` values use unsupported constructs at definition load time. This is a lint-time error (rule I-001 in the verification matrix), not a runtime surprise. If a future binding genuinely requires filter expressions or recursive descent, the outputBinding profile MUST be extended via a dedicated ADR rather than silently tolerating the feature.

**Forward compatibility note:** The profile is designed to grow backwards-compatibly. Adding new supported constructs does not require existing profiles to change. Removing a supported construct is a breaking change and requires a new major version of this specification.

**Iteration order for wildcard over objects:** For `[*]` applied to a JSON object, iteration order equals `serde_json::Map` insertion order (preserved as-of-parse). Fixtures SHOULD NOT rely on alphabetical key order unless they sort explicitly.

**All-or-nothing binding:** If any `outputBinding` JSONPath resolves to no value, the binding invocation fails with a binding error. For optional event payload fields, use a default-providing input mapping in the downstream consumer rather than relying on partial output bindings.

### 3.4 Request-Response Bindings

A `request-response` binding defines a synchronous HTTP service invocation.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `interface` | object | REQUIRED | OpenAPI reference. Contains `$ref` (URI to an OpenAPI document). |
| `operation` | string | REQUIRED | The operation ID within the referenced OpenAPI document. |
| `inputMapping` | object | OPTIONAL | Maps case state paths to service request parameters. Keys are parameter names; values are FEL expressions or case state paths. |
| `outputBinding` | object | OPTIONAL | Maps service response fields to case state paths. Keys are case state paths; values are JSON Path expressions into the response. |

```json
{
  "type": "request-response",
  "interface": {
    "$ref": "https://api.example.gov/background-checks/openapi.yaml"
  },
  "operation": "submitCheck",
  "timeout": "PT30M",
  "retry": {
    "maxAttempts": 3,
    "backoff": "exponential",
    "initialInterval": "PT10S"
  },
  "inputMapping": {
    "applicantId": "caseFile.application.applicantId"
  },
  "outputBinding": {
    "caseFile.backgroundCheck.result": "$.result",
    "caseFile.backgroundCheck.completedAt": "$.completedAt"
  }
}
```

### 3.5 Arazzo Sequence Bindings

An `arazzo-sequence` binding references an Arazzo document (OpenAPI Initiative) for multi-step API orchestration. Arazzo defines sequences of API calls with dependencies, conditional logic, and data passing between steps.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `arazzoRef` | string (URI) | REQUIRED | URI reference to the Arazzo document. |
| `inputMapping` | object | OPTIONAL | Maps case state paths to Arazzo workflow input parameters. Keys are Arazzo input parameter names; values are FEL expressions or case state paths. |
| `outputBinding` | object | OPTIONAL | Maps Arazzo workflow output to case state paths. Keys are case state paths; values are JSON Path expressions into the Arazzo workflow output. |

Each step in the Arazzo sequence produces a separate provenance record in the workflow's Facts tier (Kernel S8). When a step in the Arazzo sequence invokes an AI agent registered in a Layer 2 AI Integration Document, that invocation is subject to the agent's deontic constraints and autonomy level.

**WOS v1.0 limitation:** In WOS v1.0, step inputs cannot reference prior step outputs via FEL (`$.steps[...]`). Cross-step data flow is through the sequence-level output binding only. Inter-step references are reserved for Arazzo Engine Binding (§2 of TODO).

Step outputs are accessible in the binding-level `outputBinding` via `$.steps.<stepId>.output`. The runtime structures the accumulated step context as `{ "steps": { "<stepId>": { "output": <stepResponse> } } }`.

```json
{
  "type": "arazzo-sequence",
  "arazzoRef": "urn:agency.gov:arazzo:eligibility-check:1.0.0",
  "responseContract": {
    "definitionRef": "urn:agency.gov:contracts:eligibility-response:1.0.0"
  },
  "inputMapping": {
    "applicantSSN": "caseFile.application.ssn",
    "householdSize": "caseFile.application.householdSize"
  },
  "outputBinding": {
    "caseFile.eligibility.result": "$.steps.eligibility.output"
  },
  "idempotencyKeyExpression": "caseFile.application.id"
}
```

### 3.6 Tool Bindings

A `tool` binding defines a non-HTTP invocation informed by CWL's `CommandLineTool` descriptor pattern. WOS does not require a CWL-conformant processor. The descriptor structure is adapted for WOS: it describes the invocation method, command, arguments, and resource requirements.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `invocation` | object | REQUIRED | Invocation descriptor (S3.6.1). |
| `inputMapping` | object | OPTIONAL | Maps case state paths to tool input parameters. |
| `outputBinding` | object | OPTIONAL | Maps tool output to case state paths. |
| `resourceRequirements` | object | OPTIONAL | Resource constraints (S3.6.2). |

#### 3.6.1 Invocation Descriptor

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `method` | string | REQUIRED | Invocation method: `command-line`, `batch-file`, `database-procedure`, `graph-query`, or an `x-` prefixed custom method. |
| `command` | string | REQUIRED | The command, procedure name, or query to execute. |
| `arguments` | array of strings | OPTIONAL | Ordered command arguments. FEL expressions embedded in `{{ }}` delimiters are evaluated against the evaluation context (Kernel S7.2) before invocation. The `{{ }}` delimiter syntax follows the Formspec Locale specification's template convention. |
| `environment` | object | OPTIONAL | Execution environment metadata (e.g., container image, runtime version). Implementation-defined. |

#### 3.6.2 Resource Requirements

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `maxExecutionTime` | string (ISO 8601 duration) | OPTIONAL | Maximum execution time. The processor MUST terminate the invocation if this duration is exceeded. |
| `maxMemory` | string | OPTIONAL | Maximum memory allocation (e.g., `"512MB"`, `"2GB"`). Implementation-defined enforcement. |
| `maxCores` | integer | OPTIONAL | Maximum CPU cores. Implementation-defined enforcement. |

```json
{
  "type": "tool",
  "invocation": {
    "method": "command-line",
    "command": "/opt/legacy/eligibility-check",
    "arguments": [
      "--ssn",
      "{{ caseFile.application.ssn }}",
      "--household-size",
      "{{ caseFile.application.householdSize }}"
    ],
    "environment": {
      "image": "legacy-tools:2024.1"
    }
  },
  "requestContract": {
    "definitionRef": "urn:agency.gov:contracts:legacy-input:1.0.0"
  },
  "responseContract": {
    "definitionRef": "urn:agency.gov:contracts:legacy-output:1.0.0"
  },
  "resourceRequirements": {
    "maxExecutionTime": "PT30S"
  }
}
```

### 3.7 Event Bindings

#### 3.7.1 Event-Emit Bindings

An `event-emit` binding produces an outbound CloudEvents 1.0 event.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `eventType` | string | REQUIRED | The CloudEvents `type` attribute value (e.g., `"org.example.grants.notification"`). |
| `dataMapping` | object | OPTIONAL | Maps case state paths to event data fields. Keys are event data field names; values are FEL expressions or case state paths. |
| `channel` | string | OPTIONAL | Delivery channel hint (e.g., `"email"`, `"webhook"`, `"queue"`). Implementation-defined. |

All outbound events MUST include the WOS CloudEvents extension attributes defined in S5.

#### 3.7.2 Event-Consume Bindings

An `event-consume` binding subscribes to inbound events from external sources.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `eventType` | string | REQUIRED | The CloudEvents `type` attribute value to subscribe to. |
| `correlation` | array of objects | REQUIRED | Correlation rules for matching inbound events to workflow instances (S6). |
| `outputBinding` | object | OPTIONAL | Maps event data fields to case state paths. |

#### 3.7.3 Callback Bindings

A `callback` binding models a long-running external interaction: the workflow sends a request and later receives a callback event.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `interface` | object | REQUIRED | OpenAPI reference for the initial request. |
| `operation` | string | REQUIRED | Operation ID for the initial request. |
| `callbackEventType` | string | REQUIRED | The CloudEvents `type` attribute value of the expected callback event. |
| `correlation` | array of objects | REQUIRED | Correlation rules for matching the callback to the originating instance (S6). |
| `inputMapping` | object | OPTIONAL | Input mapping for the initial request. |
| `outputBinding` | object | OPTIONAL | Maps callback event data to case state paths. |

### 3.8 Retry Policy

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `maxAttempts` | integer | REQUIRED | Maximum number of invocation attempts (including the initial attempt). |
| `backoff` | string | OPTIONAL | Backoff strategy: `"fixed"`, `"linear"`, or `"exponential"`. Default: `"fixed"`. |
| `initialInterval` | string (ISO 8601 duration) | OPTIONAL | Initial interval between retries. Default: `"PT1S"`. |
| `maxInterval` | string (ISO 8601 duration) | OPTIONAL | Maximum interval between retries (for exponential/linear backoff). |

---

## 4. Contract Validation

### 4.1 Formspec Definition Contracts

Integration bindings MAY declare Formspec Definition contracts for request and response validation. A contract reference points to a headless Formspec Definition (a Definition used purely for validation, with no rendering or user-facing semantics).

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `definitionRef` | string (URI) | REQUIRED | URI reference to the Formspec Definition used as the validation contract. |

### 4.2 Validation Semantics

When a `requestContract` is declared, the WOS Processor MUST validate the constructed request against the referenced Formspec Definition before sending the request. The processor MUST delegate this validation to a Formspec-conformant processor (Core S1.4).

When a `responseContract` is declared, the WOS Processor MUST validate the external system's response against the referenced Formspec Definition before committing results to the case state. The processor MUST delegate this validation to a Formspec-conformant processor (Core S1.4).

In both cases:

1. The Formspec-conformant processor evaluates the Definition against the data and produces a ValidationReport (Core S5).
2. The WOS Processor MUST ingest the ValidationReport as a provenance record in the workflow's Facts tier (Kernel S8).
3. If the ValidationReport contains errors (severity `"error"`), the WOS Processor MUST NOT commit the results to the case state. The invocation is treated as failed and is subject to the retry policy (S3.8), if configured.

This is the same Formspec-as-validator pattern used in Layer 2 for agent output validation (AI Integration S6). The Formspec Definition is the contract. The external system's output is untrusted input validated against that contract.

---

## 5. CloudEvents Extensions

### 5.1 WOS Extension Attributes

All events produced by a WOS workflow MUST conform to the CloudEvents 1.0 specification [CloudEvents]. WOS defines the following extension attributes:

| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `wosinstanceid` | string (URI) | REQUIRED | The workflow instance identifier. |
| `wosdefid` | string (URI) | REQUIRED | The workflow definition identifier (the kernel document's `url`). |
| `wosdefversion` | string | REQUIRED | The workflow definition version (the kernel document's `version`). |
| `wosstate` | string | OPTIONAL | The current lifecycle state at the time of event emission. |
| `wostaskid` | string | OPTIONAL | The task identifier, if the event relates to a task. |
| `woscorrelationkey` | string | OPTIONAL | The primary business correlation key for the workflow instance. |
| `woscausationeventid` | string | OPTIONAL | The `id` of the CloudEvents event that triggered this event. Enables causal event chains. |

### 5.2 Attribute Semantics

**`wosinstanceid`.** The unique identifier of the running workflow instance. This is the primary key for routing events back to the correct instance. A WOS Processor MUST populate this attribute on every outbound event.

**`wosdefid` and `wosdefversion`.** Together these identify which workflow definition (and version) the emitting instance was created from. External systems can use this to determine the expected schema of event data.

**`wosstate`.** The lifecycle state of the workflow instance at the moment the event was emitted. This is OPTIONAL because some events (e.g., timer expiry events generated by infrastructure) may not have access to the workflow's current state.

**`woscausationeventid`.** When an inbound event triggers a workflow transition that produces one or more outbound events, each outbound event SHOULD carry the `id` of the inbound event in this attribute. This creates an auditable causal chain.

### 5.3 Inbound Event Processing

When an external event arrives at a WOS Processor:

1. The processor MUST extract the correlation attribute values from the event (S6).
2. The processor MUST find all running workflow instances whose mapped case state values match the correlation attributes.
3. The processor MUST deliver the event to the matched instance(s).
4. If no match is found, the event MUST be logged and MAY be queued for retry.

### 5.4 Idempotent Event Consumption

All event consumption MUST be idempotent. A WOS Processor MUST handle duplicate delivery of the same event (identified by the CloudEvents `id` attribute) without producing duplicate effects. The RECOMMENDED mechanism is to record processed event identifiers and reject events whose `id` has already been processed.

---

## 6. Correlation

### 6.1 Correlation Rules

Correlation is the mechanism by which an inbound external event is matched to the correct running workflow instance. Each `event-consume` or `callback` binding declares one or more correlation rules.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `attribute` | string | REQUIRED | The CloudEvents attribute or extension attribute containing the correlation value. |
| `caseStateMapping` | string (path) | REQUIRED | The case state path whose value MUST match the correlation attribute. |

When multiple correlation rules are declared, all MUST match (logical AND).

```json
{
  "correlation": [
    {
      "attribute": "subject",
      "caseStateMapping": "caseFile.application.applicationId"
    },
    {
      "attribute": "wosinstanceid",
      "caseStateMapping": "instance.id"
    }
  ]
}
```

---

## 7. Idempotency

### 7.1 Idempotency Keys

Integration bindings that invoke external services MAY declare an `idempotencyKeyExpression` (S3.3). This is a FEL expression (Core S3) evaluated against the case state to produce a deterministic key.

When an `idempotencyKeyExpression` is declared, the WOS Processor MUST:

1. Evaluate the FEL expression against the current case state to produce the key value.
2. Pass the key to the external service as an idempotency token (the mechanism is service-specific).
3. Record the key in the invocation's provenance record.

This maps to the kernel's `idempotencyKey` property on the `invokeService` action (Kernel S9.3). The Integration Profile adds the FEL expression; the kernel handles the deduplication guarantee.

---

## 8. External Policy Engine Bridge

### 8.1 Overview

The `policy-engine` binding type invokes an external authorization or policy engine and maps its decision to a WOS governance outcome. This enables WOS workflows to delegate specific governance decisions to purpose-built policy engines without embedding their rule languages in WOS.

The bridge pattern:

1. The WOS Processor serializes the evaluation context (case state fields, actor identity, requested action, resource identifiers) into the policy engine's request format.
2. The processor invokes the policy engine.
3. The processor maps the engine's response to a WOS governance decision (`permit`, `deny`, or `indeterminate`).
4. The decision is recorded as a provenance record and is available for use in guard expressions or deontic constraint evaluation (AI Integration S4).

### 8.2 Policy Engine Binding Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `engineType` | string | REQUIRED | The policy engine type: `"xacml"`, `"opa"`, `"cedar"`, or an `x-` prefixed custom engine type. |
| `endpoint` | string (URI) | REQUIRED | The policy engine's decision endpoint. |
| `contextMapping` | object | REQUIRED | Maps WOS evaluation context fields to policy engine request parameters. Keys are engine-specific parameter names; values are FEL expressions or case state paths. |
| `decisionMapping` | object | REQUIRED | Maps the engine's response to a WOS governance decision. See S8.3. |

### 8.3 Decision Mapping

The `decisionMapping` object specifies how the policy engine's response translates to a WOS governance decision.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `permitPath` | string (JSON Path) | REQUIRED | JSON Path expression into the engine's response that evaluates to a boolean indicating `permit`. |
| `denyPath` | string (JSON Path) | OPTIONAL | JSON Path expression for `deny`. If omitted, `deny` is inferred as the negation of `permit`. |
| `reasonPath` | string (JSON Path) | OPTIONAL | JSON Path expression for the engine's human-readable reason. Recorded in provenance. |
| `obligationsPath` | string (JSON Path) | OPTIONAL | JSON Path expression for any obligations the engine attaches to the decision (XACML obligation pattern). |

### 8.4 Governance Integration

The result of a policy engine invocation is a governance decision object with the following structure:

| Property | Type | Description |
|----------|------|-------------|
| `decision` | string | `"permit"`, `"deny"`, or `"indeterminate"`. |
| `reason` | string | Human-readable reason from the engine (if available). |
| `obligations` | array | Engine-specific obligations (if available). |
| `engineType` | string | The engine type that produced this decision. |
| `timestamp` | string (ISO 8601) | When the decision was made. |

This decision object is:

1. Recorded as a provenance record in the Facts tier (Kernel S8).
2. Made available in the case state under the binding's `outputBinding` path, so guard expressions on transitions (Kernel S4.5) can reference it.
3. When used in conjunction with Layer 2 deontic constraints (AI Integration S4), the decision participates in the constraint evaluation pipeline. A `deny` decision from a policy engine overrides a `permit` from a deontic constraint -- external policy engines are more restrictive, never more permissive.

### 8.5 Engine-Specific Notes

**XACML.** The context mapping SHOULD populate Subject, Resource, Action, and Environment categories. The `obligationsPath` maps to XACML Obligations.

**OPA.** The context mapping populates the OPA input document. The `permitPath` typically maps to `$.result.allow`. OPA does not have a native obligations concept -- use `extensions` for OPA-specific advice.

**Cedar.** The context mapping populates the Cedar authorization request (principal, action, resource, context). The `permitPath` maps to the authorization decision.

```json
{
  "type": "policy-engine",
  "engineType": "opa",
  "endpoint": "https://policy.agency.gov/v1/data/wos/benefits/eligibility",
  "contextMapping": {
    "input.applicant.id": "caseFile.application.applicantId",
    "input.applicant.income": "caseFile.application.annualIncome",
    "input.action": "'determineEligibility'",
    "input.actor": "event.actorId"
  },
  "decisionMapping": {
    "permitPath": "$.result.allow",
    "reasonPath": "$.result.reason",
    "obligationsPath": "$.result.obligations"
  },
  "timeout": "PT5S",
  "retry": {
    "maxAttempts": 2,
    "backoff": "fixed",
    "initialInterval": "PT1S"
  }
}
```

---

## 9. Processing Model

### 9.1 Binding Resolution

When a WOS Processor encounters an `invokeService` action (Kernel S9.2) whose `serviceRef` matches a binding key in the Integration Profile Document, the processor MUST resolve the binding and execute the integration according to the binding's type and properties.

If no Integration Profile Document is present, or the `serviceRef` does not match any binding key, the `serviceRef` is treated as an opaque reference and execution is implementation-defined (the kernel's default behavior).

### 9.2 Execution Order

For each integration invocation, the processor MUST follow this order:

1. **Input construction.** Evaluate `inputMapping` expressions against the current case state.
2. **Request validation.** If `requestContract` is declared, validate the constructed input against the Formspec Definition (S4).
3. **Invocation.** Execute the integration binding (send request, run tool, emit event, or invoke policy engine).
4. **Response validation.** If `responseContract` is declared, validate the response against the Formspec Definition (S4).
5. **Output binding.** Apply `outputBinding` to commit results to the case state.
6. **Provenance.** Record the invocation as a provenance record in the Facts tier (Kernel S8), including input/output digests if configured (Kernel S8.3).

If any step fails, the processor MUST NOT proceed to subsequent steps. Failed invocations are subject to the retry policy (S3.8).

### 9.3 FEL Expression Evaluation

FEL expressions in `inputMapping`, `idempotencyKeyExpression`, and `contextMapping` are evaluated using the kernel's evaluation context (Kernel S7). The processor MUST delegate FEL evaluation to a Formspec-conformant processor (Core S1.4). FEL expressions use only built-in functions (Core S3.5) and extension functions (Core S3.12).

---

## 10. Extension Points

### 10.1 Custom Binding Types

The `type` property accepts values prefixed with `x-` for custom integration binding types. Custom bindings MUST follow the common binding property structure (S3.3) but MAY define additional type-specific properties.

### 10.2 Custom Policy Engine Types

The `engineType` property in policy engine bindings accepts values prefixed with `x-` for custom policy engines.

### 10.3 Binding-Level Extensions

Every integration binding supports an `extensions` object for custom metadata. Extension property names MUST begin with `x-`.

---

## 11. Security Considerations

### 11.1 External System Trust

Integration bindings invoke external systems that are outside the WOS Processor's trust boundary. The WOS Processor SHOULD:

- Validate TLS certificates for HTTPS endpoints.
- Authenticate to external services using credentials managed outside the Integration Profile Document. Credentials MUST NOT appear in Integration Profile Documents.
- Log all external invocations for audit.

### 11.2 Policy Engine Trust

Policy engine decisions are authoritative only within their declared scope. The WOS Processor MUST NOT allow a policy engine decision to weaken governance constraints declared in WOS governance documents (Workflow Governance, AI Integration, Advanced Governance). A policy engine can restrict -- never relax.

### 11.3 Input Injection

FEL expressions in `inputMapping` and tool `arguments` evaluate against case state data. When case state contains untrusted user input, the evaluated values are passed to external systems. The WOS Processor SHOULD sanitize values before constructing command-line arguments (S3.6.1) to prevent injection attacks. Formspec Definition contract validation (S4) provides structural sanitization for HTTP-based integrations.

---

## 12. Examples

### 12.1 Benefits Adjudication Integration Profile

This example demonstrates a complete Integration Profile for a benefits adjudication workflow.

```json
{
  "$wosIntegrationProfile": "1.0",
  "targetWorkflow": {
    "url": "https://agency.gov/workflows/benefits-adjudication",
    "compatibleVersions": ">=1.0.0 <2.0.0"
  },
  "bindings": {
    "eligibilityCheck": {
      "type": "arazzo-sequence",
      "description": "Multi-step eligibility verification via federal eligibility APIs",
      "arazzoRef": "urn:agency.gov:arazzo:eligibility-check:1.0.0",
      "responseContract": {
        "definitionRef": "urn:agency.gov:contracts:eligibility-response:1.0.0"
      },
      "inputMapping": {
        "applicantSSN": "caseFile.application.ssn",
        "householdSize": "caseFile.application.householdSize",
        "annualIncome": "caseFile.application.annualIncome"
      },
      "outputBinding": {
        "caseFile.eligibility.result": "$.steps.eligibility.output",
        "caseFile.eligibility.verifiedAt": "$.steps.verification.completedAt"
      },
      "idempotencyKeyExpression": "caseFile.application.id",
      "timeout": "PT5M",
      "retry": {
        "maxAttempts": 3,
        "backoff": "exponential",
        "initialInterval": "PT5S"
      }
    },
    "legacySystemCheck": {
      "type": "tool",
      "description": "Legacy mainframe eligibility cross-reference",
      "invocation": {
        "method": "command-line",
        "command": "/opt/legacy/eligibility-check",
        "arguments": [
          "--ssn", "{{ caseFile.application.ssn }}",
          "--household-size", "{{ caseFile.application.householdSize }}"
        ],
        "environment": {
          "image": "legacy-tools:2024.1"
        }
      },
      "responseContract": {
        "definitionRef": "urn:agency.gov:contracts:legacy-output:1.0.0"
      },
      "resourceRequirements": {
        "maxExecutionTime": "PT30S"
      }
    },
    "applicantNotification": {
      "type": "event-emit",
      "description": "Send notification event to applicant communication service",
      "eventType": "gov.agency.benefits.notification",
      "dataMapping": {
        "applicantId": "caseFile.application.applicantId",
        "noticeType": "caseFile.determination.noticeType",
        "determinationDate": "caseFile.determination.date"
      },
      "channel": "email"
    },
    "documentReceived": {
      "type": "event-consume",
      "description": "Receive uploaded supporting documents from document management system",
      "eventType": "gov.agency.documents.received",
      "correlation": [
        {
          "attribute": "subject",
          "caseStateMapping": "caseFile.application.applicationId"
        }
      ],
      "outputBinding": {
        "caseFile.documents.latest": "$.data.documentRef",
        "caseFile.documents.receivedAt": "$.time"
      }
    },
    "eligibilityPolicy": {
      "type": "policy-engine",
      "description": "OPA-based eligibility policy evaluation",
      "engineType": "opa",
      "endpoint": "https://policy.agency.gov/v1/data/benefits/eligibility",
      "contextMapping": {
        "input.applicant.income": "caseFile.application.annualIncome",
        "input.applicant.householdSize": "caseFile.application.householdSize",
        "input.applicant.state": "caseFile.application.stateOfResidence",
        "input.action": "'determineEligibility'",
        "input.actor": "event.actorId"
      },
      "decisionMapping": {
        "permitPath": "$.result.eligible",
        "reasonPath": "$.result.reason",
        "obligationsPath": "$.result.requiredDocuments"
      },
      "outputBinding": {
        "caseFile.policy.eligibilityDecision": "$.result"
      },
      "timeout": "PT5S"
    }
  }
}
```

---

## Normative References

- [RFC 2119] Bradner, S., "Key words for use in RFCs to Indicate Requirement Levels", BCP 14, RFC 2119, March 1997.
- [RFC 8174] Leiba, B., "Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words", BCP 14, RFC 8174, May 2017.
- [RFC 8259] Bray, T., "The JavaScript Object Notation (JSON) Data Interchange Format", STD 90, RFC 8259, December 2017.
- [RFC 3986] Berners-Lee, T., Fielding, R., and L. Masinter, "Uniform Resource Identifier (URI): Generic Syntax", STD 66, RFC 3986, January 2005.
- [ISO 8601] ISO, "ISO 8601:2019 Date and time -- Representations for information interchange".
- [Formspec Core] Formspec Working Group, "Formspec Core Specification v1.0".
- [WOS Kernel] Formspec Working Group, "WOS Kernel Specification v1.0".

## Informative References

- [CloudEvents] CNCF, "CloudEvents Specification Version 1.0.2", 2022.
- [Arazzo] OpenAPI Initiative, "Arazzo Specification", 2024.
- [CWL] Amstutz, P., et al., "Common Workflow Language Specification v1.2", 2023.
- [OpenAPI] OpenAPI Initiative, "OpenAPI Specification v3.1".
- [AsyncAPI] AsyncAPI Initiative, "AsyncAPI Specification v3.0".
- [XACML] OASIS, "eXtensible Access Control Markup Language Version 3.0", January 2013.
- [OPA] Styra Inc., "Open Policy Agent Documentation".
- [Cedar] Amazon Web Services, "Cedar Policy Language".
