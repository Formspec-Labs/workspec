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

## 2. Assurance Levels

### 2.1 Taxonomy

An **assurance level** is an ordered declaration of the binding strength between a recorded fact and the subject or actor it identifies. Assurance levels are declared per fact; they are not properties of the subject or actor.

Implementations MUST support at minimum the following four-level taxonomy:

| Level | Label | Meaning |
|---|---|---|
| `L1` | Self-asserted | Subject or actor asserted the identity binding; no external corroboration. |
| `L2` | Corroborated | Binding corroborated by at least one external source (e.g., emailed magic link, phone verification). |
| `L3` | Verified | Binding verified against an authoritative source (e.g., government ID match, credential issuer). |
| `L4` | In-person or equivalent | Binding verified under conditions equivalent to in-person government-issued identity check. |

Implementations MAY define additional levels; additional levels MUST be declared against the base four.

### 2.2 Assurance Level Is Not Authorization

Assurance level describes how strongly a fact is bound to its subject or actor. It does NOT describe what that subject or actor is authorized to do. Authorization decisions MAY use assurance level as an input but MUST NOT collapse authorization into assurance.

### 2.3 Assurance-Upgrade Facts

A subject's assurance level MAY be upgraded (but not silently downgraded) by recording an assurance-upgrade fact. Assurance-upgrade facts:

- MUST reference the existing subject continuity reference (§3).
- MUST declare the prior assurance level and the new assurance level.
- MUST declare the basis for the upgrade (document inspection, biometric match, etc.).
- MUST be canonical facts admitted through the normal Kernel Facts tier.
- MUST NOT rewrite prior facts. Upgrades apply forward only.
- MUST preserve disclosure posture independently from assurance level (see §4).
