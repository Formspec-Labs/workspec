---
title: WOS Semantic Profile
version: 1.0.0-draft.1
date: 2026-04-09
status: draft
---

> **Renamed (ADR 0076 D-3).** Sidecar schema renamed `wos-semantic-profile` → `wos-ontology-alignment` at `schemas/sidecars/wos-ontology-alignment.schema.json`. Transition-tag vocabulary moves back into `specs/kernel/spec.md` (deferred — currently still in this document). This spec doc retained for historical reference; canonical surface is the ontology-alignment schema. See `specs/sidecars/README.md` for the active sidecar index.


# WOS Semantic Profile v1.0

**Version:** 1.0.0-draft.1
**Date:** 2026-04-09
**Editors:** Formspec Working Group
**Companion to:** WOS Kernel Specification v1.0

---

## Abstract

The WOS Semantic Profile is a parallel seam specification for the Workflow Orchestration Standard (WOS). A Semantic Profile Document -- itself a JSON document -- declares a JSON-LD `@context` that maps WOS properties to RDF terms, SHACL shape references for semantic validation, PROV-O vocabulary mappings for provenance export, and XES/OCEL export configuration for process mining interoperability. The `@context` makes WOS documents interpretable as linked data without transformation. SHACL shapes validate cross-cutting constraints that JSON Schema cannot express. PROV-O mappings produce W3C-conformant provenance graphs from the kernel's Facts tier (Kernel S8). XES/OCEL mappings enable export of provenance records for process mining tools.

The Semantic Profile is a parallel seam -- it attaches at any layer and does not introduce new kernel seams. A WOS workflow functions without a Semantic Profile Document. The profile provides linked data interpretation for existing WOS structures and provenance records.

WOS MUST NOT alter core Formspec processing semantics. WOS processors MUST delegate Formspec Definition evaluation to a Formspec-conformant processor (Core S1.4).

---

## Status of This Document

This document is a **draft specification**. It is a parallel seam profile within the Workflow Orchestration Standard, a companion framework to Formspec v1.0 that does not modify Formspec's processing model. Implementors are encouraged to experiment with this specification and provide feedback, but MUST NOT treat it as stable for production use until a 1.0.0 release is published.

---

## Conventions and Terminology

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [BCP 14][rfc2119] [RFC 2119] [RFC 8174] when, and only when, they appear in ALL CAPITALS, as shown here.

JSON syntax and data types are as defined in [RFC 8259]. URI syntax is as defined in [RFC 3986]. IRI syntax is as defined in [RFC 3987].

Terms defined in the WOS Kernel Specification -- including *Kernel Document*, *lifecycle*, *case state*, *Facts tier*, *provenance record*, *actor*, and *evaluation context* -- retain their kernel-specification meanings throughout this document unless explicitly redefined.

Additional terms:

- **Semantic Profile Document** -- A JSON document conforming to this specification that declares linked data configuration for a WOS workflow.
- **Context Document** -- A JSON-LD `@context` artifact that maps WOS property names to IRIs.
- **SHACL Shape** -- A W3C SHACL constraint that validates the RDF graph produced by interpreting a WOS document as JSON-LD.
- **PROV-O Graph** -- An RDF graph conforming to the W3C PROV Ontology, produced by mapping WOS provenance records through the vocabulary mapping declared in this profile.
- **XES Event** -- An event record conforming to the IEEE XES standard (IEEE Std 1849-2016), produced by mapping WOS provenance records through the export configuration declared in this profile.
- **OCEL Event** -- An event record conforming to the Object-Centric Event Log (OCEL 2.0) format.

[rfc2119]: https://www.rfc-editor.org/rfc/rfc2119
[RFC 3986]: https://www.rfc-editor.org/rfc/rfc3986
[RFC 3987]: https://www.rfc-editor.org/rfc/rfc3987
[RFC 8174]: https://www.rfc-editor.org/rfc/rfc8174
[RFC 8259]: https://www.rfc-editor.org/rfc/rfc8259

---

## 1. Introduction

### 1.1 Background

WOS documents are JSON. They describe lifecycle topology, case state, actors, and provenance. These same documents can be interpreted as linked data -- nodes in an RDF graph that are queryable, linkable, and formally validatable -- without transformation, middleware, or translation layers. The Semantic Profile declares the configuration that enables this interpretation.

The kernel specifies that provenance records are compatible with PROV-DM (Kernel S8.4) but defers vocabulary mapping to this profile. This profile fulfills that deferral.

### 1.2 Design Goals

1. **Interpretation, not transformation.** The Semantic Profile does not change how WOS documents are processed. It declares how they are interpreted in RDF/linked data contexts.
2. **JSON-LD by overlay.** A WOS document becomes JSON-LD by applying the `@context` declared in this profile. The document's JSON structure is unchanged.
3. **SHACL for policy, JSON Schema for structure.** JSON Schema validates structural correctness. SHACL shapes validate policy-level semantic constraints that span properties and cross document boundaries.
4. **Standards alignment.** PROV-O for provenance, XES/OCEL for process mining, JSON-LD for linked data. This profile maps to established standards rather than inventing new ones.
5. **Incremental adoption.** A processor that ignores this profile loses no functionality. A processor that adopts it gains linked data interoperability, semantic validation, and standards-conformant provenance export.

### 1.3 Scope

**Within scope:** JSON-LD `@context` document for WOS properties; SHACL shape references for semantic validation; PROV-O vocabulary mapping for provenance records; XES/OCEL export configuration for process mining; namespace declarations and domain vocabulary extension; conformance profiles.

**Out of scope:** lifecycle topology, case state, actor model (Kernel). Due process, review protocols (Workflow Governance). Agent registration, deontic constraints (AI Integration). The processing semantics of WOS documents (unchanged by this profile). SPARQL endpoint configuration (implementation-defined). RDF storage mechanisms (implementation-defined). SHACL shape authoring methodology (use the informative shapes in Appendix A as a starting point).

### 1.4 Relationship to the Kernel

The Semantic Profile targets a WOS Kernel Document via `targetWorkflow`. The profile does not use any kernel seam. It operates as a pure interpretation overlay: a processor reads the profile, applies the `@context` to the kernel document and its provenance records, and produces linked data artifacts.

The kernel's PROV-DM compatibility statement (Kernel S8.4) is the architectural seam. This profile fulfills that statement by providing the concrete PROV-O vocabulary mapping.

The profile does not require any governance layer. A kernel-only workflow with a Semantic Profile Document is valid.

### 1.5 Notational Conventions

JSON examples use standard JSON syntax. Turtle examples use W3C Turtle syntax for RDF. All examples are informative unless stated otherwise.

Namespace prefixes used in this specification:

| Prefix | Namespace IRI | Source |
|--------|---------------|--------|
| `wos:` | `https://wos-spec.org/ns/` | WOS (this specification) |
| `prov:` | `http://www.w3.org/ns/prov#` | W3C PROV-O |
| `sh:` | `http://www.w3.org/ns/shacl#` | W3C SHACL |
| `schema:` | `https://schema.org/` | Schema.org |
| `xsd:` | `http://www.w3.org/2001/XMLSchema#` | XML Schema Datatypes |
| `dcterms:` | `http://purl.org/dc/terms/` | Dublin Core Terms |
| `lrml:` | `http://docs.oasis-open.org/legalruleml/ns/v1.0/` | OASIS LegalRuleML |

---

## 2. Document Structure

This section is normative.

### 2.1 Top-Level Properties

A Semantic Profile Document is a JSON object with the following top-level properties:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `$wosSemanticProfile` | string | REQUIRED | Document type marker. MUST be `"1.0"`. |
| `targetWorkflow` | object | REQUIRED | The WOS Kernel Document this profile targets. |
| `version` | string | OPTIONAL | Version of this Semantic Profile Document. SemVer is RECOMMENDED. |
| `title` | string | OPTIONAL | Human-readable name for this semantic profile. |
| `description` | string | OPTIONAL | Human-readable description of purpose and scope. |
| `context` | object | REQUIRED | JSON-LD `@context` configuration. |
| `shapes` | object | OPTIONAL | SHACL shape references for semantic validation. |
| `provMapping` | object | OPTIONAL | PROV-O vocabulary mapping for provenance export. |
| `processMining` | object | OPTIONAL | XES/OCEL export configuration. |
| `extensions` | object | OPTIONAL | Extension data. All keys MUST be prefixed with `x-`. |

### 2.2 Target Workflow

The `targetWorkflow` property binds this profile to a specific Kernel Document:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `url` | string (URI) | REQUIRED | Canonical URL of the target Kernel Document. |
| `compatibleVersions` | string | OPTIONAL | SemVer range expression for compatible kernel versions. |

---

## 3. JSON-LD Context

This section is normative.

### 3.1 Overview

The `context` property declares the JSON-LD `@context` that a semantic processor applies to WOS documents. When applied, every property name in the WOS document maps to an IRI, and the document becomes interpretable as an RDF graph without structural change.

A processor that applies the `@context` MUST NOT alter the JSON structure of the WOS document. The `@context` is an interpretation overlay, not a transformation.

The Semantic Profile's graph surface is a projection of the Kernel statechart, not a replacement substrate. State, transition, actor, case-file, and provenance identifiers SHOULD be minted from stable workflow URLs, versions, authored identifiers, and lifecycle paths. Processors SHOULD avoid blank nodes for WOS-defined terms unless no stable source identifier exists. If a processor claims deterministic semantic projection support, it MUST produce the same expanded RDF graph for the same WOS document and same ontology-alignment configuration.

Processors MAY also support semantic import: accepting a JSON-LD/RDF graph and lowering it into a hierarchical `$wosWorkflow` document. Import MUST be deterministic. If the graph cannot resolve to one lifecycle hierarchy, transition order, identifier set, and case-file shape, the processor MUST reject it with a stable diagnostic rather than choosing a hierarchy silently.

### 3.2 Context Configuration

The `context` object declares:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `contextUrl` | string (URI) | REQUIRED | URL of the JSON-LD `@context` document. |
| `contextVersion` | string | REQUIRED | Version of the context document, synchronized with the WOS specification version. |
| `namespaces` | object | OPTIONAL | Additional namespace prefix-to-IRI mappings beyond the base WOS context. |
| `domainVocabularies` | array | OPTIONAL | External domain vocabularies to incorporate (e.g., NIEM, FHIR, Schema.org extensions). |

### 3.3 The WOS Namespace

The WOS namespace is `https://wos-spec.org/ns/`. All WOS-specific terms (lifecycle, case state, provenance record types, impact levels, actor types) are minted under this namespace.

Terms from external vocabularies are mapped to their canonical IRIs:

| WOS Property | Maps To | Source |
|--------------|---------|--------|
| `id` | `@id` | JSON-LD keyword |
| `type` | `@type` | JSON-LD keyword |
| `name` | `schema:name` | Schema.org |
| `description` | `schema:description` | Schema.org |
| `version` | `schema:version` | Schema.org |
| `created` | `schema:dateCreated` (typed `xsd:dateTime`) | Schema.org |
| `modified` | `schema:dateModified` (typed `xsd:dateTime`) | Schema.org |
| `authority` | `dcterms:authority` | Dublin Core |

WOS-specific terms under the `wos:` namespace:

| WOS Property | IRI | Domain |
|--------------|-----|--------|
| `lifecycle` | `wos:lifecycle` | Kernel |
| `initialState` | `wos:initialState` | Kernel |
| `states` | `wos:states` | Kernel |
| `transitions` | `wos:transitions` | Kernel |
| `guard` | `wos:guard` | Kernel |
| `event` | `wos:triggerEvent` | Kernel |
| `target` | `wos:targetState` (typed `@id`) | Kernel |
| `onEntry` | `wos:onEntry` | Kernel |
| `onExit` | `wos:onExit` | Kernel |
| `caseFile` | `wos:caseFile` | Kernel |
| `items` | `wos:items` | Kernel |
| `impactLevel` | `wos:impactLevel` | Kernel |
| `status` | `wos:status` | Kernel |
| `actors` | `wos:actors` | Kernel |
| `actorType` | `wos:actorType` | Kernel |
| `milestones` | `wos:milestones` | Kernel |
| `tags` | `wos:transitionTags` | Kernel |

### 3.4 Governance Property Mappings

When a governance layer is present, the context extends with governance-specific mappings:

| WOS Property | IRI | Layer |
|--------------|-----|-------|
| `dueProcess` | `wos:dueProcess` | Governance (L1) |
| `reviewProtocols` | `wos:reviewProtocols` | Governance (L1) |
| `pipelines` | `wos:validationPipelines` | Governance (L1) |
| `assertionGates` | `wos:assertionGates` | Governance (L1) |
| `deonticConstraints` | `wos:deonticConstraints` | AI Integration (L2) |
| `permissions` | `wos:permissions` | AI Integration (L2) |
| `prohibitions` | `wos:prohibitions` | AI Integration (L2) |
| `obligations` | `wos:obligations` | AI Integration (L2) |
| `rights` | `wos:rights` | AI Integration (L2) |
| `Permission` | `lrml:Permission` | LegalRuleML |
| `Prohibition` | `lrml:Prohibition` | LegalRuleML |
| `Obligation` | `lrml:Obligation` | LegalRuleML |
| `Right` | `lrml:Right` | LegalRuleML |
| `agents` | `wos:agents` | AI Integration (L2) |
| `autonomy` | `wos:autonomyLevel` | AI Integration (L2) |
| `confidence` | `wos:confidence` | AI Integration (L2) |
| `guardrails` | `wos:guardrails` | AI Integration (L2) |
| `fallback` | `wos:fallback` | AI Integration (L2) |

### 3.5 Domain Vocabulary Extension

The `domainVocabularies` array declares external vocabularies that extend the base WOS context for domain-specific case data:

```json
{
  "domainVocabularies": [
    {
      "prefix": "niem-hs",
      "namespace": "https://release.niem.gov/niem/domains/humanServices/5.2/",
      "description": "NIEM Human Services domain for benefits case data"
    },
    {
      "prefix": "fhir",
      "namespace": "http://hl7.org/fhir/",
      "description": "HL7 FHIR for health-related case data"
    }
  ]
}
```

When a Case File Item's schema references an external vocabulary, the item's properties map to that vocabulary's terms in the JSON-LD serialization. This enables cross-agency interoperability by extending the `@context` rather than building translation middleware.

### 3.6 Context Versioning

The context document is versioned alongside the WOS specification. Breaking changes to the context (renaming or removing mappings) MUST increment the major specification version. Adding new mappings is a non-breaking change.

A Semantic Profile processor MUST reject a context document whose major version it does not support.

---

## 4. SHACL Shapes

This section is normative.

### 4.1 Overview

SHACL shapes validate policy-level constraints that JSON Schema cannot express. They operate on the RDF graph produced by interpreting a WOS document through the `@context` declared in S3. JSON Schema validates structure; SHACL validates semantic policy.

A Semantic Profile processor SHOULD validate WOS documents against the declared SHACL shapes. Validation failures SHOULD be reported as provenance records at the Facts tier (Kernel S8).

### 4.2 Shape References

The `shapes` property declares SHACL shape sets:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `shapeGraphs` | array | REQUIRED | URIs of SHACL shape graphs to apply. |
| `severity` | enum | OPTIONAL | Minimum severity for reported violations: `"violation"`, `"warning"`, `"info"`. Default: `"violation"`. |
| `scope` | enum | OPTIONAL | What to validate: `"definition"` (the workflow definition), `"provenance"` (provenance records), `"all"`. Default: `"all"`. |

### 4.3 Standard Shape Categories

This specification defines eight categories of SHACL shapes for WOS documents. Informative Turtle examples are provided in Appendix A. Implementations MAY use these shapes directly or author equivalent constraints.

**SP-01: Lifecycle Soundness.** Every non-final state MUST have at least one outgoing transition. This validates that the lifecycle has no dead-end states that would trap a workflow instance.

**SP-02: Actor Completeness.** Every actor referenced in lifecycle actions (onEntry, onExit, transition actions) MUST be declared in the kernel's actor registry. This validates that no action references an undefined actor.

**SP-03: Due Process Completeness.** Workflows classified as `rights-impacting` MUST have due process configured (notice, explanation, appeal, continuation of service). This validates that impact level and due process configuration are consistent.

**SP-04: Contract Coverage.** Every data exchange point (invokeService, createTask with input/output schemas) MUST have a contract reference. This validates that data flows through validated channels.

**SP-05: Attestation Requirement.** Provenance records for agent actions MUST include the required attestation fields: model identifier, model version, confidence value, and input summary. This validates that agent provenance meets audit requirements.

**SP-06: Dual-Readability Narrative.** Narrative tier provenance records MUST be marked `authoritative: false`. This validates that model-generated explanations are never treated as authoritative audit evidence.

**SP-07: Verifiable Constraint Annotation.** Constraints that claim formal verification status MUST include a reference to the verification report (Advanced Governance S8). This validates that verification claims are substantiated.

**SP-08: Constraint Zone Satisfiability.** In a DCR constraint zone (Advanced Governance), all pending activities MUST be reachable through the zone's include/exclude relations. This validates that the constraint zone is not trivially unsatisfiable.

### 4.4 Custom Shapes

Implementations MAY declare additional SHACL shapes beyond the standard categories. Custom shape graph URIs MUST be included in the `shapeGraphs` array. Custom shapes MUST NOT contradict the standard shapes defined in S4.3.

---

## 5. PROV-O Vocabulary Mapping

This section is normative.

### 5.1 Overview

The kernel's Facts tier (Kernel S8) records provenance as structured JSON. This section defines how those records map to the W3C PROV Ontology (PROV-O), producing standards-conformant provenance graphs that are interoperable with any PROV-O-aware system.

The kernel states that Facts tier records are compatible with PROV-DM (Kernel S8.4). This section fulfills that architectural commitment by providing the concrete vocabulary mapping.

### 5.2 The provMapping Configuration

The `provMapping` property declares PROV-O export configuration:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `provenanceNamespace` | string (URI) | REQUIRED | Base namespace for provenance IRIs. |
| `actorMapping` | object | OPTIONAL | Customizations for how WOS actors map to `prov:Agent`. |
| `tierMapping` | object | OPTIONAL | Configuration for how provenance tiers beyond Facts map to PROV-O constructs. |

### 5.3 Facts Tier to PROV-O

Each Facts tier record (Kernel S8.2) maps to the PROV-O Entity-Activity-Agent triad:

| WOS Facts Tier Field | PROV-O Term | Mapping |
|----------------------|-------------|---------|
| The provenance record itself | `prov:Activity` | Each Facts tier record is a `prov:Activity`. |
| `id` | `@id` on the Activity | The record identifier becomes the Activity IRI. |
| `timestamp` | `prov:atTime` (typed `xsd:dateTime`) | When the activity occurred. |
| `actorId` | `prov:wasAssociatedWith` (typed `@id`) | Links the Activity to its Agent. |
| Actor (from `actorId`) | `prov:Agent` | The actor is a `prov:Agent` node. |
| `actorType` | `rdf:type` on the Agent | `human` asserts both `prov:Person` and `wos:HumanAgent`; `system` asserts both `prov:SoftwareAgent` and `wos:SystemAgent`; `agent` (Layer 2) asserts both `prov:SoftwareAgent` and `wos:AIAgent`. Both PROV-O types (for tool interoperability) and WOS types (for domain querying) are asserted. |
| `inputs` | `prov:used` | Each input is a `prov:Entity` used by the Activity. |
| `outputs` | `prov:wasGeneratedBy` (inverse) | Each output is a `prov:Entity` generated by the Activity. |
| `action` | `wos:actionType` on the Activity | The type of action performed. |
| `lifecycleState` | `wos:atLifecycleState` on the Activity | The lifecycle state at action time. |
| `definitionVersion` | `wos:definitionVersion` on the Activity | The governing document version. |
| `inputDigest` | `wos:inputDigest` on used Entities | Tamper detection hash. |
| `outputDigest` | `wos:outputDigest` on generated Entities | Tamper detection hash. |

### 5.4 Higher Provenance Tiers

Higher-layer provenance tiers (Reasoning, Counterfactual, Narrative) are injected through the `provenanceLayer` seam (Kernel S10.3). In PROV-O, each tier maps to a `prov:Bundle` with a type annotation:

| Provenance Tier | PROV-O Construct | Type Annotation |
|-----------------|------------------|-----------------|
| **Facts** (Kernel) | `prov:Activity` + `prov:Entity` + `prov:Agent` | Direct PROV-O triad. No bundle wrapping. |
| **Reasoning** (Layer 1) | `prov:Bundle` | `wos:ReasoningBundle` -- contains `prov:Activity` nodes for each rule applied, evidence consulted, and authority cited. |
| **Counterfactual** (Layer 1) | `prov:Bundle` | `wos:CounterfactualBundle` -- contains `prov:Activity` nodes describing what inputs would change the outcome. |
| **Narrative** (Layer 2) | `prov:Bundle` | `wos:NarrativeBundle` -- contains model-generated explanation text. MUST include `wos:authoritative false` assertion. |

A `prov:Bundle` is itself a `prov:Entity`, so higher tiers are linked to the Facts tier record via `prov:wasDerivedFrom`. This preserves the kernel's invariant that provenance grows upward and lower layers are never modified.

### 5.5 Actor Type Mapping

The `actorMapping` property allows customization of how WOS actor types map to PROV-O Agent subclasses:

```json
{
  "actorMapping": {
    "human": ["prov:Person", "wos:HumanAgent"],
    "system": ["prov:SoftwareAgent", "wos:SystemAgent"],
    "agent": ["prov:SoftwareAgent", "wos:AIAgent"]
  }
}
```

The default mapping (used when `actorMapping` is absent) is:

| WOS Actor Type | PROV-O Type |
|----------------|-------------|
| `human` | `prov:Person`, `wos:HumanAgent` |
| `system` | `prov:SoftwareAgent`, `wos:SystemAgent` |
| `agent` (Layer 2) | `prov:SoftwareAgent`, `wos:AIAgent` |

### 5.6 PROV-O Export

A semantic processor that supports PROV-O export MUST produce valid PROV-O RDF graphs. The processor MUST:

1. Map every Facts tier record to the PROV-O triad as defined in S5.3.
2. Map higher-tier records to `prov:Bundle` as defined in S5.4.
3. Link bundles to their source Facts record via `prov:wasDerivedFrom`.
4. Apply actor type mapping as defined in S5.5.
5. Mint IRIs using the `provenanceNamespace` as the base.

The processor SHOULD serialize PROV-O output in one of: JSON-LD, Turtle, N-Quads, or TriG.

---

## 6. Process Mining Export

This section is normative.

### 6.1 Overview

WOS provenance records contain the event data that process mining tools analyze. This section defines how provenance records map to XES (IEEE Std 1849-2016) and OCEL 2.0 formats, enabling export to process mining tools without WOS-specific adapters.

### 6.2 The processMining Configuration

The `processMining` property declares export configuration:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `formats` | array of enum | REQUIRED | Export formats to support: `"xes"`, `"ocel"`, or both. |
| `caseIdentifier` | string | OPTIONAL | The case state field used as the XES case identifier. Default: `"instanceId"`. |
| `eventClassifier` | string | OPTIONAL | The provenance field used as the XES activity classifier. Default: `"action"`. |
| `objectTypes` | array | OPTIONAL | For OCEL: the WOS object types to include in the object-centric event log. |

### 6.3 XES Mapping

WOS provenance records map to XES events as follows:

| WOS Provenance Field | XES Element | XES Standard Extension |
|----------------------|-------------|------------------------|
| Workflow instance | `<trace>` | Each workflow instance is an XES trace. |
| Provenance record | `<event>` | Each Facts tier record is an XES event. |
| `action` | `concept:name` | The activity name (Concept extension). |
| `timestamp` | `time:timestamp` | Event timestamp (Time extension). |
| `actorId` | `org:resource` | The actor performing the action (Organizational extension). |
| `actorType` | `org:group` | Actor type classification. |
| `lifecycleState` | `wos:lifecycleState` (custom WOS attribute) | The workflow state name at event time. Note: WOS does NOT use the standard XES lifecycle model (`lifecycle:transition`). The standard lifecycle extension expects values like "start", "complete", "suspend" — WOS lifecycle states are workflow-specific names. |
| `id` | `identity:id` | Unique event identifier (ID extension). |
| `inputs`/`outputs` | `<string>` attributes | Serialized as string attributes on the event. |
| `inputDigest`/`outputDigest` | `<string>` attributes | Preserved as custom attributes. |
| `definitionVersion` | `<string>` attribute | Custom global attribute on the trace. |

A semantic processor that supports XES export MUST produce valid XES documents conforming to IEEE Std 1849-2016. The processor MUST include the Concept, Time, and Lifecycle standard extensions. The processor SHOULD include the Organizational and ID extensions.

### 6.4 OCEL Mapping

OCEL 2.0's object-centric event model maps more naturally to WOS's case-centric workflows than XES's flat trace model. WOS provenance maps to OCEL as follows:

| WOS Concept | OCEL 2.0 Concept | Mapping |
|-------------|-------------------|---------|
| Provenance record | Event | Each Facts tier record is an OCEL event. |
| `action` | Event type | The event's activity type. |
| `timestamp` | Event timestamp | When the event occurred. |
| Case File Item | Object | Each case file item is an OCEL object. |
| Case File Item type | Object type | The item's schema type. |
| Record-to-item relationship | Event-to-Object (E2O) | Events that read or modify a case file item produce E2O relationships. |
| Item-to-item relationship | Object-to-Object (O2O) | Cross-references between case file items produce O2O relationships. |
| Item state changes | Object attribute changes | Mutations to case file items are tracked as attribute changes over time. |

A semantic processor that supports OCEL export MUST produce valid OCEL 2.0 event logs. Events that mutate multiple case file items MUST produce a single OCEL event with multiple E2O links, not duplicated event records.

### 6.5 Export Scope

Process mining export applies to Facts tier provenance records. Higher-tier records (Reasoning, Counterfactual, Narrative) are excluded from process mining export by default because they represent interpretive annotations, not process execution events. A processor MAY include higher-tier records as additional event attributes when explicitly configured.

---

## 7. Conformance

This section is normative.

### 7.1 Conformance Levels

**Semantic Structural.** Parse and validate Semantic Profile Documents against the JSON Schema. The processor MUST reject documents that fail schema validation.

**Semantic Context.** Structural conformance plus: apply the declared `@context` to WOS documents, producing valid JSON-LD. The processor MUST preserve the `@context` during serialization round-trips. The processor MUST NOT alter the JSON structure of the source document when applying the context.

**Semantic Projection.** Context conformance plus: produce a deterministic graph projection from WOS lifecycle, actor, case-file, and provenance structures. The projection MAY reify transitions or other relationships when RDF tooling requires relationship properties, but that reification is projection-local. It MUST NOT introduce a second authoring topology or alter Kernel statechart semantics.

**Semantic Import.** Projection conformance plus: accept a JSON-LD/RDF graph and lower it into a hierarchical `$wosWorkflow` document. The imported document then follows ordinary Kernel validation and processing. Ambiguous graphs MUST be rejected with stable diagnostics.

**Semantic Validation.** Context conformance plus: validate WOS documents against declared SHACL shapes. The processor MUST report SHACL validation results. The processor SHOULD record validation results as provenance records.

**Semantic Export.** Validation conformance plus: produce PROV-O graphs from provenance records (S5) and/or XES/OCEL event logs from provenance records (S6). Exported artifacts MUST conform to their respective standards.

### 7.2 Additive Invariant

The Semantic Profile MUST NOT alter how WOS documents are processed. Lifecycle semantics, case state mutation, provenance recording, and all kernel guarantees (Kernel S9.1) are unchanged. The profile adds interpretation (how documents are understood as linked data) and export (how provenance is represented in external formats) without affecting processing.

A WOS processor that does not implement the Semantic Profile MUST produce identical lifecycle behavior, case state, and provenance records as one that does.

---

## 8. Extension Points

This section is normative.

### 8.1 Document-Level Extensions

Semantic Profile Documents support extension properties at the top level via the `extensions` property. All extension keys MUST be prefixed with `x-`.

### 8.2 Shape Extensions

Custom SHACL shapes beyond the standard categories (S4.3) MAY be declared by adding shape graph URIs to the `shapeGraphs` array. Custom shapes MUST NOT contradict standard shapes.

### 8.3 Vocabulary Extensions

Additional namespace prefixes and domain vocabularies MAY be declared via the `namespaces` and `domainVocabularies` properties in the `context` configuration (S3.2, S3.5).

---

## 9. Security Considerations

This section is informative.

### 9.1 Context Integrity

The JSON-LD `@context` document controls how WOS property names map to IRIs. A compromised `@context` could cause misinterpretation of WOS terms when processed as RDF. Implementations SHOULD serve the context document over HTTPS from the specification's canonical domain. Implementations SHOULD cache the context document and verify its integrity (e.g., via Subresource Integrity or content-addressed hashes).

### 9.2 SHACL Execution

SHACL shapes that include SPARQL-based constraints execute queries against the RDF graph. Malicious shapes could construct expensive queries. Implementations SHOULD set resource limits (query timeout, result set size) on SHACL processors.

### 9.3 Provenance Export

PROV-O and XES/OCEL exports may contain personally identifiable information (PII) from provenance records. Implementations MUST apply the same access controls and data protection requirements to exported artifacts as to the source provenance records.

---

## References

### Normative References

- [RFC 2119] Bradner, S., "Key words for use in RFCs to Indicate Requirement Levels", BCP 14, RFC 2119, March 1997.
- [RFC 8174] Leiba, B., "Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words", BCP 14, RFC 8174, May 2017.
- [RFC 8259] Bray, T., "The JavaScript Object Notation (JSON) Data Interchange Format", STD 90, RFC 8259, December 2017.
- [RFC 3986] Berners-Lee, T., Fielding, R., and L. Masinter, "Uniform Resource Identifier (URI): Generic Syntax", STD 66, RFC 3986, January 2005.
- [RFC 3987] Duerst, M. and M. Suignard, "Internationalized Resource Identifiers (IRIs)", RFC 3987, January 2005.
- [WOS Kernel] Formspec Working Group, "WOS Kernel Specification v1.0".
- [Formspec Core] Formspec Working Group, "Formspec Core Specification v1.0".
- [JSON-LD11] Sporny, M., Longley, D., Kellogg, G., Lanthaler, M., and N. Lindstrom, "JSON-LD 1.1", W3C Recommendation, July 2020.
- [SHACL] Knublauch, H. and D. Kontokostas, "Shapes Constraint Language (SHACL)", W3C Recommendation, July 2017.
- [PROV-O] Lebo, T., Sahoo, S., and D. McGuinness, "PROV-O: The PROV Ontology", W3C Recommendation, April 2013.
- [PROV-DM] Moreau, L. and P. Missier, "PROV-DM: The PROV Data Model", W3C Recommendation, April 2013.

### Informative References

- [RDF11] Cyganiak, R., Wood, D., and M. Lanthaler, "RDF 1.1 Concepts and Abstract Syntax", W3C Recommendation, February 2014.
- [XES] IEEE, "IEEE Standard for eXtensible Event Stream (XES)", IEEE Std 1849-2016.
- [OCEL2] van der Aalst, W.M.P., et al., "Object-Centric Event Log (OCEL) 2.0", 2023.
- [LegalRuleML] OASIS, "LegalRuleML Core Specification Version 1.0", OASIS Standard, 2021.
- [NIEM] NIEM Technical Architecture Committee, "National Information Exchange Model".
- [FHIR] HL7 International, "Fast Healthcare Interoperability Resources (FHIR)".

---

## Appendix A: SHACL Shape Library (informative)

This appendix is informative. The following SHACL shapes provide reference implementations for the eight standard shape categories defined in S4.3. Implementations MAY use these shapes directly or author equivalent constraints.

### A.1 SP-01: Lifecycle Soundness

```turtle
@prefix wos: <https://wos-spec.org/ns/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .

wos:LifecycleSoundnessShape
  a sh:NodeShape ;
  sh:targetClass wos:State ;
  sh:sparql [
    sh:message "Non-final states MUST have at least one outgoing transition." ;
    sh:severity sh:Violation ;
    sh:select """
      SELECT $this WHERE {
        $this a wos:State .
        $this wos:stateType ?type .
        FILTER (?type != "final")
        FILTER NOT EXISTS {
          ?t wos:sourceState $this .
          ?t a wos:Transition .
        }
      }
    """ ;
  ] .
```

### A.2 SP-02: Actor Completeness

```turtle
@prefix wos: <https://wos-spec.org/ns/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .

wos:ActorCompletenessShape
  a sh:NodeShape ;
  sh:targetSubjectsOf wos:actorId ;
  sh:sparql [
    sh:message "Every actor referenced in actions MUST be declared in the actor registry." ;
    sh:severity sh:Violation ;
    sh:select """
      SELECT $this ?actor WHERE {
        $this wos:actorId ?actor .
        FILTER NOT EXISTS {
          ?workflow wos:actors ?registry .
          ?registry wos:actorId ?actor .
        }
      }
    """ ;
  ] .
```

### A.3 SP-03: Due Process Completeness

```turtle
@prefix wos: <https://wos-spec.org/ns/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .

wos:DueProcessCompletenessShape
  a sh:NodeShape ;
  sh:targetClass wos:WorkflowDefinition ;
  sh:sparql [
    sh:message "Rights-impacting workflows MUST have due process configured with notice, explanation, appeal, and continuation of service. This shape validates appeal as a representative check -- production deployments SHOULD extend this to validate all four components." ;
    sh:severity sh:Violation ;
    sh:select """
      SELECT $this WHERE {
        $this wos:impactLevel "rights-impacting" .
        FILTER NOT EXISTS {
          $this wos:dueProcess ?dp .
          ?dp wos:appealMechanism ?am .
          ?am wos:enabled true .
        }
      }
    """ ;
  ] .
```

### A.4 SP-04: Contract Coverage

```turtle
@prefix wos: <https://wos-spec.org/ns/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .

wos:ContractCoverageShape
  a sh:NodeShape ;
  sh:targetSubjectsOf wos:serviceRef ;
  sh:sparql [
    sh:message "Every invokeService action MUST have a contract reference for validation." ;
    sh:severity sh:Violation ;
    sh:select """
      SELECT $this WHERE {
        $this wos:serviceRef ?ref .
        FILTER NOT EXISTS {
          $this wos:inputContract ?ic .
        }
        FILTER NOT EXISTS {
          $this wos:outputContract ?oc .
        }
      }
    """ ;
  ] .
```

### A.5 SP-05: Attestation Requirement

```turtle
@prefix wos: <https://wos-spec.org/ns/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .

wos:AttestationRequirementShape
  a sh:NodeShape ;
  sh:targetClass wos:ProvenanceRecord ;
  sh:sparql [
    sh:message "Agent provenance records MUST include model identifier, model version, confidence, and input summary." ;
    sh:severity sh:Violation ;
    sh:select """
      SELECT $this WHERE {
        $this wos:actorType "agent" .
        FILTER (
          NOT EXISTS { $this wos:modelIdentifier ?mi } ||
          NOT EXISTS { $this wos:modelVersion ?mv } ||
          NOT EXISTS { $this wos:confidence ?c } ||
          NOT EXISTS { $this wos:inputSummary ?is }
        )
      }
    """ ;
  ] .
```

### A.6 SP-06: Dual-Readability Narrative

```turtle
@prefix wos: <https://wos-spec.org/ns/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .

wos:NarrativeNonAuthoritativeShape
  a sh:NodeShape ;
  sh:targetClass wos:ProvenanceRecord ;
  sh:sparql [
    sh:message "Narrative tier records MUST be marked non-authoritative." ;
    sh:severity sh:Violation ;
    sh:select """
      SELECT $this WHERE {
        $this wos:auditLayer "narrative" .
        FILTER NOT EXISTS {
          $this wos:authoritative false .
        }
      }
    """ ;
  ] .
```

### A.7 SP-07: Verifiable Constraint Annotation

```turtle
@prefix wos: <https://wos-spec.org/ns/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .

wos:VerifiableConstraintShape
  a sh:NodeShape ;
  sh:targetSubjectsOf wos:verificationStatus ;
  sh:sparql [
    sh:message "Constraints claiming verification MUST reference a verification report." ;
    sh:severity sh:Violation ;
    sh:select """
      SELECT $this WHERE {
        $this wos:verificationStatus "verified" .
        FILTER NOT EXISTS {
          $this wos:verificationReport ?vr .
        }
      }
    """ ;
  ] .
```

### A.8 SP-08: Constraint Zone Satisfiability

```turtle
@prefix wos: <https://wos-spec.org/ns/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .

wos:ConstraintZoneSatisfiabilityShape
  a sh:NodeShape ;
  sh:targetClass wos:ConstraintZone ;
  sh:sparql [
    sh:message "All pending activities in a constraint zone MUST be reachable." ;
    sh:severity sh:Violation ;
    sh:select """
      SELECT $this ?activity WHERE {
        $this wos:activities ?activity .
        ?activity wos:status "pending" .
        FILTER NOT EXISTS {
          $this wos:includeRelation ?inc .
          ?inc wos:target ?activity .
        }
      }
    """ ;
  ] .
```

---

## Appendix B: Complete JSON-LD Context Document (informative)

This appendix is informative. The following is the reference `@context` document for WOS v1.0. The normative published context is maintained at `https://wos-spec.org/context/1.0.0`.

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

    "name": "schema:name",
    "description": "schema:description",
    "version": "schema:version",
    "created": { "@id": "schema:dateCreated", "@type": "xsd:dateTime" },
    "modified": { "@id": "schema:dateModified", "@type": "xsd:dateTime" },
    "status": "wos:status",
    "impactLevel": "wos:impactLevel",
    "authority": "dcterms:authority",

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
    "tags": "wos:transitionTags",

    "actors": { "@id": "wos:actors", "@container": "@index" },
    "actorType": "wos:actorType",

    "caseFile": "wos:caseFile",
    "items": { "@id": "wos:items", "@container": "@index" },
    "multiplicity": "wos:multiplicity",
    "visibility": "wos:visibility",
    "vocabulary": { "@id": "wos:vocabulary", "@type": "@id" },

    "dueProcess": "wos:dueProcess",
    "reviewProtocols": "wos:reviewProtocols",
    "pipelines": "wos:validationPipelines",
    "assertionGates": "wos:assertionGates",

    "agents": { "@id": "wos:agents", "@container": "@index" },
    "capabilities": { "@id": "wos:capabilities", "@container": "@list" },
    "inputContract": "wos:inputContract",
    "outputContract": "wos:outputContract",
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

## Appendix C: PROV-O Export Example (informative)

This appendix is informative. The following shows a Facts tier provenance record and its PROV-O Turtle equivalent.

**WOS Facts Tier Record (JSON):**

```json
{
  "id": "urn:wos:prov:benefits-001:tr-001",
  "timestamp": "2026-03-15T10:30:00Z",
  "actorId": "urn:wos:actor:case-worker-42",
  "actorType": "human",
  "action": "stateTransition",
  "lifecycleState": "eligibilityReview",
  "definitionVersion": "1.0.0",
  "inputs": {
    "event": "applicationSubmitted",
    "sourceState": "intake"
  },
  "outputs": {
    "targetState": "eligibilityReview",
    "taskCreated": "urn:wos:task:eligibility-review-001"
  }
}
```

**PROV-O Turtle equivalent:**

```turtle
@prefix wos: <https://wos-spec.org/ns/> .
@prefix prov: <http://www.w3.org/ns/prov#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<urn:wos:prov:benefits-001:tr-001>
  a prov:Activity, wos:StateTransition ;
  prov:atTime "2026-03-15T10:30:00Z"^^xsd:dateTime ;
  prov:wasAssociatedWith <urn:wos:actor:case-worker-42> ;
  wos:actionType "stateTransition" ;
  wos:atLifecycleState "eligibilityReview" ;
  wos:definitionVersion "1.0.0" ;
  prov:used [
    a prov:Entity ;
    wos:eventType "applicationSubmitted" ;
    wos:sourceState "intake"
  ] ;
  prov:generated [
    a prov:Entity ;
    wos:targetState "eligibilityReview" ;
    wos:taskCreated <urn:wos:task:eligibility-review-001>
  ] .

<urn:wos:actor:case-worker-42>
  a prov:Person, wos:HumanAgent .
```

---

## Appendix D: XES Export Example (informative)

This appendix is informative. The following shows how the provenance record from Appendix C maps to XES.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<log xes.version="1849.2016" xes.features="">
  <extension name="Concept" prefix="concept"
    uri="http://www.xes-standard.org/concept.xesext"/>
  <extension name="Time" prefix="time"
    uri="http://www.xes-standard.org/time.xesext"/>
  <extension name="Lifecycle" prefix="lifecycle"
    uri="http://www.xes-standard.org/lifecycle.xesext"/>
  <extension name="Organizational" prefix="org"
    uri="http://www.xes-standard.org/org.xesext"/>
  <extension name="Identity" prefix="identity"
    uri="http://www.xes-standard.org/identity.xesext"/>

  <global scope="trace">
    <string key="concept:name" value="UNKNOWN"/>
    <string key="wos:definitionVersion" value="UNKNOWN"/>
  </global>
  <global scope="event">
    <string key="concept:name" value="UNKNOWN"/>
    <date key="time:timestamp" value="1970-01-01T00:00:00.000+00:00"/>
  </global>

  <trace>
    <string key="concept:name" value="benefits-001"/>
    <string key="wos:definitionVersion" value="1.0.0"/>

    <event>
      <string key="identity:id" value="urn:wos:prov:benefits-001:tr-001"/>
      <string key="concept:name" value="stateTransition"/>
      <date key="time:timestamp" value="2026-03-15T10:30:00.000+00:00"/>
      <string key="org:resource" value="urn:wos:actor:case-worker-42"/>
      <string key="org:group" value="human"/>
      <string key="wos:lifecycleState" value="eligibilityReview"/>
      <string key="wos:sourceState" value="intake"/>
      <string key="wos:targetState" value="eligibilityReview"/>
    </event>
  </trace>
</log>
```
