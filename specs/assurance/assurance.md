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

## 3. Subject Continuity

### 3.1 Definition

A **subject continuity reference** is a stable identifier linking related activity, records, or attestations across time without, by itself, requiring full legal-identity disclosure.

Subject continuity is a provenance primitive, not an identity claim. Two facts sharing a subject continuity reference assert that they concern the same subject; they do NOT assert what that subject's legal identity is.

### 3.2 Requirements

Implementations that record identity facts MUST:

- Support at least one subject continuity reference mechanism.
- Declare the scope within which a continuity reference is stable (instance, case, tenant, deployment).
- Preserve continuity references across workflow instance migration (Kernel S9.6).
- Allow distinct continuity references to be held by the same legal subject (pseudonymous separation).

Implementations MUST NOT:

- Assume that a continuity reference implies any particular assurance level.
- Assume that distinct continuity references imply distinct legal subjects.
- Merge continuity references implicitly. Explicit merge MUST be recorded as a canonical fact.

## 4. Invariant 6: Disclosure Posture Is Not Assurance Level

### 4.1 Statement

**Invariant 6 (normative, constitutional):** Disclosure posture and assurance level are independent properties of a fact. Implementations MUST NOT conflate them, MUST NOT derive one from the other, and MUST NOT couple their transitions.

### 4.2 Background

**Disclosure posture** declares how much of a subject's identity is revealed in a given context (anonymous, pseudonymous, identified, public). **Assurance level** declares how strongly a fact is bound to its subject (§2).

A fact MAY be highly assured and pseudonymously disclosed (a verified-L3 claim disclosed under a pseudonym). A fact MAY be weakly assured and fully identified (a self-asserted L1 claim tied to a legal name). All four combinations are valid. Implementations that force these to co-vary violate this invariant.

### 4.3 Behavioral Consequences

- Profiles MAY constrain disclosure posture or assurance level independently. A profile that constrains both MUST constrain them as independent predicates, not a joint predicate.
- Assurance-upgrade facts (§2.3) MUST NOT implicitly change disclosure posture.
- Disclosure re-scoping (e.g., a pseudonymous record being identified later) MUST NOT imply an assurance upgrade.
- Verifiers MUST be able to check assurance claims independently of disclosure claims.

### 4.4 Normative Home

This invariant is stated normatively here. Other specifications in the WOS family, and bindings such as Trellis, MUST reference this section rather than restating the invariant.
