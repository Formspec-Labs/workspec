# WOS Assurance Layer

## Abstract

The WOS Assurance Layer specifies identity and attestation semantics for workflows that handle rights-impacting, safety-impacting, or otherwise consequential decisions. It defines an assurance-level taxonomy independent of disclosure posture, a subject-continuity primitive for linking related activity across time without requiring full legal-identity disclosure, and normative rules for representing attestations provider-neutrally.

This layer attaches to the WOS Kernel via the `provenanceLayer` seam (S10.3) and the `custodyHook` seam (S10.5). It is opt-in: kernel-only deployments conform to WOS without implementing any assurance layer. Deployments that record identity facts, issue attestations, or make claims about the evidentiary weight of provenance MUST conform to this layer.

## Status of This Document

This document is a normative companion to the WOS Kernel. Statements using BCP 14 keywords are normative. All other statements are informative.

## 1. Introduction

### 1.1 Scope

Within scope: assurance-level taxonomy; subject continuity; provider-neutral attestation representation; disclosure posture independence from assurance level (Invariant 6); legal-sufficiency disclosure obligations; assurance-upgrade facts.

Out of scope: cryptographic signing algorithms; key lifecycle mechanics; custody posture declarations (see `custodyHook` seam, Kernel S10.5); concrete identity-provider bindings.

### 1.2 Notational Conventions

The key words MUST, MUST NOT, REQUIRED, SHALL, SHALL NOT, SHOULD, SHOULD NOT, RECOMMENDED, NOT RECOMMENDED, MAY, and OPTIONAL in this document are to be interpreted as described in BCP 14 [RFC2119] [RFC8174].
