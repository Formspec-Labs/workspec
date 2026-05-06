# ADR 0082: WOS Kernel Semantic Projection and Import

**Status:** Proposed
**Date:** 2026-04-30
**Scope:** WOS Kernel, WOS Ontology Alignment sidecar
**Related:** [ADR 0075 (rejection register)](../../../thoughts/adr/0075-rejection-register.md); [ADR 0076 (product-tier consolidation)](../../../thoughts/adr/0076-product-tier-consolidation.md); [`work-spec/counter-proposal-disposition.md`](../../counter-proposal-disposition.md); [`work-spec/specs/sidecars/README.md`](../../specs/sidecars/README.md)

## Context

WOS Kernel §4 defines lifecycle semantics as a deterministic hierarchical statechart. The statechart is the authoring, processing, and conformance surface. FlowSpec-style flat `nodes[]` / `edges[]` topology and conditional-node-vs-conditional-edge heuristics remain rejected because they would create a second topology inside the Kernel.

The useful instinct behind those proposals is still valid: workflow structure can be queried and validated as a graph. WOS already has an Ontology Alignment sidecar for JSON-LD `@context`, SHACL shapes, PROV-O export, and XES/OCEL mapping. That sidecar is the correct home for graph interpretation because it adds linked-data and export capability without changing runtime processing.

## Decision

WOS Kernel remains statechart-native. Ontology Alignment defines deterministic semantic projection and import for Kernel documents and provenance records in RDF/JSON-LD tooling.

The projection is not a substrate, wire form, or alternate authoring model. Semantic import is an authoring/interchange path that must lower into ordinary `$wosWorkflow` before Kernel processing. A processor that ignores Ontology Alignment and a processor that supports it MUST produce identical lifecycle behavior, case state, and provenance records for the same workflow and event sequence.

## Rules

### D-1. Kernel Truth

The hierarchical `$wosWorkflow` lifecycle remains the canonical authoring and processing form. Kernel conformance is measured against statechart evaluation, guard ordering, transition execution, case state mutation, provenance emission, durable execution guarantees, and named extension seams.

### D-2. Projection Ownership

JSON-LD context, RDF graph interpretation, semantic import, SHACL validation, SPARQL queryability, PROV-O mapping, XES export, and OCEL export belong to Ontology Alignment. Kernel may state the additive invariant, but projection/import mechanics live in the sidecar/profile surface.

### D-3. Deterministic Identifiers

Semantic projection SHOULD mint WOS-defined IRIs from stable workflow URLs, workflow versions, authored identifiers, instance identifiers, and lifecycle paths. Processors SHOULD avoid blank nodes for WOS-defined terms unless no stable identifier exists.

### D-4. Reification Discipline

Projection processors MAY reify transitions or other relationships as graph nodes when RDF tooling needs relationship properties. Reification is projection-local. It MUST NOT create a second Kernel topology or change how authors decide between states, transitions, guards, and tags.

### D-5. Import Discipline

Semantic import MAY accept JSON-LD/RDF graphs as authoring or interchange input. Import MUST resolve to a single hierarchical `$wosWorkflow` document before Kernel validation or runtime processing. If a graph has multiple possible parents for a state, ambiguous transition ordering, unstable identifiers, or unresolved case-file shape, the importer MUST reject it with a stable diagnostic rather than choosing silently.

### D-6. SHACL Scope

SHACL shapes validate semantic/profile claims. They do not replace Rust lint, JSON Schema, or Kernel conformance. A SHACL shape may mirror a Kernel or governance invariant, but Kernel processors do not become SHACL processors by default.

### D-7. Evidence Bar

Any future claim that semantic projection/import is verifier-facing conformance requires the same four artifacts as other verifier-facing WOS claims: written semantics, machine grammar or schema, drift-failing vectors, and a runnable verifier. Until then, examples are examples, not conformance evidence.

## Consequences

### Positive

- Preserves the statechart execution model while giving RDF/SHACL/SPARQL consumers a deterministic graph surface and import path.
- Absorbs the graph-query value behind flat-node proposals without reopening flat topology as a Kernel authoring model.
- Keeps Ontology Alignment independent and honest: it adds interpretation and export capability, not runtime semantics.

### Negative

- Graph-oriented implementers must accept WOS statechart semantics as the source shape rather than authoring native RDF workflows.
- Projection/import conformance remains limited until a real verifier and fixture suite exist.

### Neutral

- Existing Kernel processors do not change.
- Existing `wos-export` PROV-O/XES/OCEL surfaces remain the implementation home for export behavior.
- Existing rejection rows for flat graph topology and X33 stay rejected, with ADR 0082 as clarification rather than supersession.
