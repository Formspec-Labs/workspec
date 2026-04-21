# ADR-0060: Cross-reference naming — `Ref`, `Key`, and `Id`

**Status:** Proposed  
**Date:** 2026-04-21  
**Deciders:** Formspec Working Group  
**Author:** WOS backlog (TODO.md Do next #2)  
**Supersedes:** None  
**Related:**

- [TODO.md](../../TODO.md) — Do next #2 (Cross-reference shape ADR)
- `schemas/governance/wos-workflow-governance.schema.json` — `calendarRef` vs `templateRef` / `escalationChainRef` (mixed conventions today)
- Vision model stack-wide naming hygiene (URI vs in-document key vs map-local id)

---

## 1. Context

Several WOS JSON properties carry *references* to other objects, but the property names use `Ref` indiscriminately for three different semantics:

1. **URI / URN references** — Values are absolute or resolvable URIs into another artifact, sidecar, or registry entry (`format: uri` or documented URI grammar). Example: `BusinessCalendar` sidecar address.
2. **In-document lookup keys** — Values are plain strings naming an entry in a sibling map or catalog **within the same document** (not necessarily URIs). Example: a notification template identifier used to resolve into a `NotificationTemplate` bundle loaded by policy.
3. **Sibling identifiers** — Values are stable ids matched against `id` fields on ordered array elements (escalation ladder steps), where reordering must not silently retarget policy without an explicit id match.

Using `*Ref` for all three forces readers and tooling to infer semantics from schema descriptions (`description` prose) instead of from the field name. That increases integration bugs (passing a URI where a key was expected, or vice versa) and makes automated validation weaker than it could be.

---

## 2. Decision

Adopt three **suffix conventions** for new and revised normative properties:

| Suffix | Meaning | Value shape | Typical JSON Schema |
|--------|---------|-------------|---------------------|
| `*Ref` | **External or cross-artifact reference** | URI / URN (or documented URI template) | `type: string`, `format: uri` where applicable |
| `*Key` | **In-document catalog key** | Non-empty string matching a map key or registry slot in the same package | `type: string`, `minLength: 1`, pattern per document grammar |
| `*Id` | **Identifier of a sibling object** in an ordered collection | String matching `^[a-zA-Z][a-zA-Z0-9_-]*$` (kernel identifier grammar unless superseded) | `type: string`, `pattern: …` |

**Normative rules**

1. **Do not use `Ref` for in-document keys.** If the value is not a URI, the field MUST NOT end in `Ref` once the owning schema is revised under this ADR.
2. **Do not use `Key` for URIs.** If the value is primarily a URI into another artifact, the field MUST use `*Ref` (and SHOULD declare `format: uri` when the value is a URI).
3. **Use `Id` only for intra-array / intra-map stable matching**, where the author intends ordinal-independent targeting (e.g. breach policy pointing at a named escalation step).

**Illustrative renames (governance SLA surface — non-normative until schemas land)**

- `WarningThreshold.templateRef` → `templateKey` (resolves within notification template catalog / sidecar binding, not a bare HTTP URL in typical profiles).
- `BreachPolicy.escalationChainRef` → `escalationStepId` (selects `EscalationStep.id`).

Exact rename set is owned by schema PRs; this ADR records the **taxonomy**, not every field in one shot.

---

## 3. Consequences

### Positive

- Schema and generated docs become self-explanatory at the field name layer.
- Lints can classify resolution rules: “resolve `*Ref` via URI loader”, “resolve `*Key` via in-doc map”, “resolve `*Id` via sibling scan”.

### Negative / cost

- **Breaking JSON renames** unless we ship a compatibility window with `oneOf` / `deprecated` dual properties (recommended for one release in Formspec’s greenfield posture — optional; WOS may choose hard rename pre-1.0 per repo policy).
- **Documentation sweep** — companion specs and Runtime Companion prose must use the same vocabulary once schemas switch.

---

## 4. Migration (recommended)

1. Land schema changes with **dual properties** for one release if any external consumer exists; otherwise rename in place pre-1.0.
2. Update fixtures and conformance harness in the same merge as schema.
3. Add or extend a Tier 2 lint rule that flags deprecated old names if dual-write is chosen.

---

## 5. Status

**Proposed** — awaits working-group acceptance and a scoped schema PR implementing renames for the first high-churn surface (Workflow Governance SLA / notification template pointers).
