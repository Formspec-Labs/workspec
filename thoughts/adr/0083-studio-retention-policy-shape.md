# ADR-0083: Studio `RetentionPolicy` shape and placement

**Status:** Accepted 2026-05-03 (Revision 2 — incorporates 2026-05-03 review feedback; G7 added §2.2 worked-example clarification)
**Date:** 2026-05-03 (revised same day)
**Deciders:** WOS Working Group (Studio sub-team review pending)
**Author:** Studio authoring layer
**Supersedes:** None
**Amends:** [`studio/specs/policy-object-model.md`](../../studio/specs/policy-object-model.md) §"EvidenceRequirement" — replaces singular `retentionPeriod?` with `retentionPolicy?: RetentionPolicy`; adds `RetentionPolicy` to § Data model.

**Related:**

- [`studio/DEFERRED.md`](../../studio/DEFERRED.md) `STUDIO-DEFER-005` (typed-promotion target)
- [`studio/crates/wos-studio-model/src/docs.rs`](../../studio/crates/wos-studio-model/src/docs.rs) `retention_policy()` (~L291)
- [`studio/crates/wos-studio-lint/src/workspace_rules.rs`](../../studio/crates/wos-studio-lint/src/workspace_rules.rs) `WF-LINT-006` (~L1486)
- [`studio/schemas/wos-studio-policy-object.schema.json`](../../studio/schemas/wos-studio-policy-object.schema.json), [`wos-studio-workspace.schema.json`](../../studio/schemas/wos-studio-workspace.schema.json) `retentionPolicies` map
- [`specs/governance/workflow-governance.md`](../../specs/governance/workflow-governance.md) §7.15 (Legal Hold), §12 (Typed Hold Policies)
- `SA-MUST-pom-037` (sharpened by this ADR)

---

## 1. Context

`studio/specs/policy-object-model.md` lists `retentionPeriod?` as a singular optional body field on `EvidenceRequirement` (line 170). However, `studio/crates/wos-studio-model/src/docs.rs::retention_policy()` (L291) and `WF-LINT-006` already key on `retentionPolicy` (a richer shape). The spec and code are *already inconsistent*; this ADR closes the drift rather than migrating a live field. Aspirational comments in the spec reference `mode`, `duration`, `legalHoldOverride`, but those fields are not normative anywhere — `retention_policy()` returns `Option<&serde_json::Value>` and `WF-LINT-006` does presence-only.

Three candidate placement points exist: per-EvidenceRequirement, per-DataElement, or workspace-level mapping (`workspace.policy.retentionPolicies`, currently `additionalProperties: true`). Without an ADR, Studio cannot promote the accessor, cannot tighten `WF-LINT-006`, and cannot close the workspace mapping. The shape MUST be pinned first.

This ADR's substrate is **government-benefits / due-process workflows** (SNAP, TANF, fair hearings) where statutory retention rules (7 CFR §272.1(f) — SNAP records 3 years; HIPAA §164.530(j) — 6 years; NIST SP 800-53 SI-12 — archival lean) frequently REQUIRE multi-year retention post-case-closure for audit, OIG review, and appeal-window reconstruction. Defaults must reflect this safety posture.

---

## 2. Decision

### 2.1 `RetentionPolicy` is a typed object with a fixed field set

Closed-shape object. Required: `duration`, `disposalAction` (no default — author MUST pick). Optional: `mode`, `triggerEvent`, `respectsLegalHold`, `regulatoryBasis[]`. `additionalProperties: false` plus `^(\$|x-)` patternProperties for vendor extensions.

| Field | Type | Required | Values |
|---|---|---|---|
| `duration` | ISO 8601 duration string | yes (when `mode` ≠ `indefinite`) | e.g., `"P7Y"`, `"P30D"`, `"PT0S"` (immediate disposal). When `mode = "indefinite"`, `duration` MUST be omitted (the `"indefinite"` literal sentinel is dropped from r2 — use the explicit `mode` instead). |
| `mode` | enum | no (default `time-bound`) | `time-bound` (runs `duration` from `triggerEvent`), `event-bound` (retains until `triggerEvent` fires, no clock), `indefinite` (no scheduled disposal; `duration` MUST be absent). |
| `triggerEvent` | string | no (default `caseClosure`) | Canonical: `caseClosure`, `lastSubstantiation`, `policyExpiry`, `subjectRequestErasure`, `dataElementSuperseded`. `x-` extensions admitted. See §5 on `subjectRequestErasure` × legal-hold tension. |
| `disposalAction` | enum | **yes (no default)** | `archive`, `cryptoErase`, `redact`, `purge`. Authors MUST explicitly pick — there is no safe-by-default disposal in a regulated-records substrate. `transfer` is reserved for a future ADR once the destination shape is pinned (see §3 + §5). |
| `respectsLegalHold` | boolean | no (default `true`) | `true` → kernel `holdType: legal-hold` suspends the disposal clock and blocks `disposalAction` execution; resume semantics on release follow whatever `workflow-governance.md` §7.15 specifies (today: only "blocks" is normative; clock-resume not yet pinned — see §5). `false` → disposal runs to completion regardless of any hold; MUST cite `regulatoryBasis`. (Renamed from `legalHoldOverride` in r2 — the prior name read backwards: `legalHoldOverride: true` actually meant "hold overrides me, I respect it.") |
| `regulatoryBasis[]` | SourceCitation refs | no | Required when `respectsLegalHold = false`. Override semantics on inheritance: workspace-default `regulatoryBasis[]` and EvidenceRequirement-level `regulatoryBasis[]` **merge** (union, deduplicated by SourceCitation id) rather than replace, since each citation independently grounds the disposal decision. (Pinned in r2 per §2.2 review.) |

### 2.2 Placement: per-EvidenceRequirement, with optional workspace defaults

`RetentionPolicy` attaches at **`EvidenceRequirement.body.retentionPolicy`**. This matches the existing `retention_policy()` accessor location, the `WF-LINT-006` enforcement site, and the spec's `retentionPeriod?` slot. EvidenceRequirement is where collection happens; the policy attaches where the collection does.

`workspace.policy.retentionPolicies` is preserved as a defaults map keyed by DPV sensitivity IRI (e.g., `dpv:HealthData → {duration: "P7Y", ...}`). EvidenceRequirements that collect a DataElement of that sensitivity inherit the workspace default unless they declare their own `retentionPolicy`, which overrides **field-by-field, NOT whole-object**. Override semantics, pinned in r2:

- **Scalar fields** (`duration`, `mode`, `triggerEvent`, `disposalAction`, `respectsLegalHold`): EvidenceRequirement value replaces workspace value if present; otherwise workspace value applies.
- **List field `regulatoryBasis[]`**: workspace and EvidenceRequirement values **merge** (union, deduplicated by SourceCitation id). A workspace default that cites HIPAA + an EvidenceRequirement that cites a CMS sub-rule produces the union.
- **"Required when `respectsLegalHold = false`"** is interpreted on the **resolved** policy (post-merge), not the literal EvidenceRequirement. If the workspace default sets `respectsLegalHold: false` with `regulatoryBasis: [HIPAA]` and the EvidenceRequirement upgrades to `respectsLegalHold: true`, the workspace's `[HIPAA]` basis **remains in the resolved `regulatoryBasis[]`** (per the merge rule above); only the `regulatoryBasis-required-when-false` lint check is non-firing. The resolved policy never strips workspace bases as a side effect of the upgrade. If workspace `true`/EvidenceRequirement `false`, the EvidenceRequirement MUST contribute `regulatoryBasis`.

The map's value-type tightens to a `RetentionPolicy` `$ref` and `additionalProperties: false`, **with the exception of `^(\$|x-)` patternProperties preserved** so vendor extensions and `$comment` survive (otherwise this is a workspace-bag-breaking change for any deployment relying on extensibility — see §4 Negative).

`DataElement.body.retentionPolicy` is **rejected** (see §3).

### 2.3 Migration from singular `retentionPeriod?`

Hard deprecation, single-rev. The `retentionPeriod?` field is removed from the spec; authors lift its value into `retentionPolicy.duration`. Verification confirmed (repo grep): `retentionPeriod` appears only in (a) the spec text at `policy-object-model.md:170`, (b) two archived/research drafts, (c) this ADR, (d) `studio/DEFERRED.md`. Zero schema enforcement, zero fixture, zero lint reads it. The accessor (`retention_policy()`) and lint rule (`WF-LINT-006`) already key on `retentionPolicy` — the spec/code are already drifted and this ADR closes the drift.

The Studio compiler emits a one-rev advisory diagnostic (`SA-WARN-pom-MIGRATE-RETENTION`) for any document still carrying `retentionPeriod`; removed in the rev after.

### 2.4 Legal hold delegates to kernel `holdType: legal-hold` (block-only)

`respectsLegalHold: true` (the default) is a delegation flag, not a parallel mechanism. When the kernel records a `legal-hold` per `specs/governance/workflow-governance.md` §7.15, the case-instance processor MUST:

1. **Suspend the disposal clock** for the affected EvidenceRequirements (§7.15: "blocks data destruction, retention expiry, and scheduled lifecycle operations").
2. **Reject `disposalAction` execution** with the hold reference recorded in rejection provenance (§7.15: "blocks data destruction").

**Clock-resume semantics on release are NOT specified by this ADR.** `workflow-governance.md` §7.15 names hold release as a fact-recording event but does not pin clock-on-release behavior (preserve elapsed time? reset? become immediately due?). Pinning that semantic is a kernel-spec amendment, tracked as a follow-up:

> **Open kernel-spec amendment (blocking E8.4 lint promotion):** `workflow-governance.md` §7.15 SHOULD specify: "On legal-hold release, the disposal clock resumes from the suspension point preserving elapsed pre-hold time." (Or alternative: clock resets, with explicit rationale.) Until pinned, runtime adapters MAY implement either policy; Studio's lint rule (WF-LINT-006) does NOT enforce a particular clock-resume semantic.

Studio does not invent its own legal-hold mechanism; it declares the policy's behavior under one. `WF-LINT-006` validates `RetentionPolicy` shape only; runtime hold enforcement is the kernel's concern.

---

## 3. Rejected Alternatives

### Workspace-only mapping is the sole source

Rejected. Forces every EvidenceRequirement to inherit; no per-collection override. Real policy distinguishes one-time-intake retention from ongoing-monitoring retention even when sensitivity is identical.

### `DataElement.body.retentionPolicy` (per-element)

Rejected. DataElements describe shape; EvidenceRequirements describe collection. Same SSN can have different retention depending on why it was collected.

### Keep singular `retentionPeriod?`

Rejected. Cannot encode `mode`, cannot record `regulatoryBasis`, cannot disable legal-hold respect. The accessor stays untyped and `WF-LINT-006` stays presence-only forever.

### Soft deprecation (allow both fields for one rev)

Rejected. Two competing fields force every consumer to reconcile them. Pre-1.0 Studio has no compatibility surface to protect; greenfield migration is cheaper.

### New top-level PolicyObject kind `RetentionPolicy`

Rejected. Retention is a property of collection, not an independent legal object. A free-standing kind would force every EvidenceRequirement to carry a `retentionPolicyRef` and invent identity (id, lifecycle, citations) where none is needed. The workspace defaults map already covers sharing.

### `disposalAction` default of `purge`

Rejected (revised in r2). In a government-benefits / due-process substrate where 7 CFR §272.1(f), HIPAA §164.530(j), and NIST SP 800-53 SI-12 require multi-year retention post-closure, defaulting to `purge` would silently destroy records statute requires retained. Making `disposalAction` required-no-default forces authors to explicitly pick the disposal mode for each EvidenceRequirement; the schema rejects under-specified policies before runtime can act on a dangerous default.

### `transfer` as a v1 enum value

Rejected (revised in r2). The `transfer` action implies a destination (`transferTo`) shape that this ADR does not pin. Admitting `transfer` in v1 lets authors declare a disposal mode that schema-validates, lints clean, and runtime-fails (or silently no-ops). Reserved for a follow-up ADR once the destination shape lands. Authors needing transfer semantics today MUST either (a) use `archive` + an out-of-band transfer process, or (b) propose the follow-up ADR.

### `legalHoldOverride` as the field name

Rejected (revised in r2). The name reads backwards: `legalHoldOverride: true` was intended to mean "I respect the hold (it overrides my disposal)" but a reader plausibly parses it as "I override the legal hold." Renamed to `respectsLegalHold` (default `true`); the negation (`false`) now reads naturally as "this policy does NOT respect legal hold," which is the rare case requiring explicit `regulatoryBasis`.

---

## 4. Consequences

### Positive

- `retention_policy()` promotes to `Option<&RetentionPolicy>` per `STUDIO-DEFER-005`.
- `WF-LINT-006` becomes shape-aware: enforces required fields, validates `disposalAction` enum (no default), requires `regulatoryBasis` when `respectsLegalHold = false` (on resolved policy).
- `wos-studio-policy-object.schema.json` gains a `RetentionPolicy` `$def` (`additionalProperties: false`); schema-pass catches typos.
- `wos-studio-workspace.schema.json::retentionPolicies` tightens to `additionalProperties: false` over a value-type `$ref`, while preserving `^(\$|x-)` patternProperties — closes the third `STUDIO-DEFER-005` unblock path without breaking vendor extensions.
- Kernel/Studio retention semantics align: legal-hold delegates to the kernel concept in `workflow-governance.md` §7.15 (block-only; clock-resume pinned via follow-up amendment).
- Spec/code drift on `retentionPeriod` vs `retentionPolicy` resolved.

### Negative

- Authors who held `retentionPeriod?` migrate to `retentionPolicy.duration`. Pre-1.0; cost is one find/replace plus a migration warning for one rev.
- Authors MUST explicitly pick `disposalAction` (no default) for every EvidenceRequirement carrying a `retentionPolicy`. This is intentional — the cost is one enum-value typed per requirement; the alternative is silent statute violation under a default.
- **Workspace-bag tightening from `additionalProperties: true` to `additionalProperties: false` is a breaking change** for any deployment that placed unprefixed extensions in the `retentionPolicies` map. Mitigation: `^(\$|x-)` patternProperties preserved (vendor extensions using the `x-` convention or `$comment`/`$id` keys still validate). One-rev advisory `SA-WARN-pom-WORKSPACE-RETENTION-EXT` flags any unprefixed unknown key before the rev that hard-rejects.
- Five enum decisions added to Studio's spec surface. Mitigated by reasonable defaults (`time-bound`, `caseClosure`, `respectsLegalHold: true`); the one decision left to the author (`disposalAction`) is the safety-critical choice.
- Kernel-spec amendment to `workflow-governance.md` §7.15 (clock-resume on release) is a follow-up dependency; until landed, runtime adapters retain implementation discretion. WF-LINT-006 does not enforce a particular semantic.

### Neutral

- Per Studio CLAUDE.md "owner directive 2026-05-02," Studio quality lives under Studio-team discipline. This ADR is proposed for WOS Working Group review on the cross-cutting kernel-delegation question (§2.4); Studio retains final discretion on field-set decisions.
- The six-seam invariant is unchanged. `RetentionPolicy` lives inside Studio's PolicyObject body; no new kernel seam.

---

## 5. Open questions

1. **`disposalAction = transfer` follow-up**: when a real use-case lands, a follow-up ADR pins the destination shape (`transferTo`: actor / system / regulator) and re-admits `transfer` to the enum.
2. **`triggerEvent` extensibility**: the canonical-events enumeration is initial; vendor extensions via `x-` patternProperties are admitted. Whether to grow the canonical set or formalize a "retention-trigger registry" follows the first vendor extension.
3. **Cross-spec naming**: should the kernel governance block (`holdPolicies`) gain a back-reference to `RetentionPolicy` for documentation symmetry? Tracked separately; not load-bearing for this ADR.
4. **Versioning at workflow republish** (added r2): when a WorkflowIntent is republished with a tightened `RetentionPolicy`, do already-collected evidence records keep their original (collection-time) policy or adopt the new one? **Default-immutable** (collection-time policy persists for the record's lifetime) is the safer answer — matches "policy in force at collection" common in records-retention regimes — but MUST be pinned in a follow-up. The tooling implication: the case-instance projection MUST snapshot `retentionPolicy` at collection time, not resolve it at disposal time.
5. **Disposal audit sink** (added r2): when a `purge` / `cryptoErase` / `redact` action executes, where is the receipt recorded? Candidates: `$wosProvenanceLog` (FactsTier), a dedicated `disposalLog`, or a Trellis custody event. Without a designated audit sink, "we deleted it" is unprovable post-hoc, undermining the very compliance posture the policy enforces.
6. **Pseudonymization vs. deletion** (added r2): GDPR Art. 4(5) treats pseudonymization as distinct from deletion. The current enum collapses pseudonymization into `redact` (or arguably `cryptoErase`). A future spec rev should clarify which enum value implements GDPR-compliant pseudonymization.
7. **DSAR (`subjectRequestErasure`) × legal-hold tension** (added r2): when a Data Subject Access Request triggers erasure (GDPR Art. 17) on a record under legal hold, the default `respectsLegalHold: true` blocks the erasure. The DSAR response MUST acknowledge the conflict (legal-hold takes precedence per §7.15) without leaking the hold's existence (which may itself be confidential). Operational handling is workflow-level, not policy-level; flagging here so the conflict is documented.
8. **Cross-tenant retention**: in multi-tenant deployments, the workspace-default map is workspace-scoped. A hosting tenant with stricter retention than its workspace currently has no override path. Out of scope for this ADR; raise as a separate concern when SaaS multi-tenancy ratifies.

---

## 6. Implementation Notes

Once accepted, follow-up commits land in this order:

1. **Spec** — `studio/specs/policy-object-model.md`: add `RetentionPolicy` block under § Data model, replace `retentionPeriod?` with `retentionPolicy?` on `EvidenceRequirement`, add Composition note on workspace-default inheritance (field-by-field for scalars, merge for `regulatoryBasis[]`), sharpen `SA-MUST-pom-037` to "well-formed `RetentionPolicy`."
2. **Schema** — `wos-studio-policy-object.schema.json` `RetentionPolicy` `$def` referenced from `EvidenceRequirement.body`; `wos-studio-workspace.schema.json::retentionPolicies` tightens to `additionalProperties: false` + preserves `^(\$|x-)` patternProperties.
3. **Rust typed promotion** — `retention_policy()` → `Option<&RetentionPolicy>`; struct in `wos-studio-model/src/policy.rs` with `serde` derives. Field set per §2.1; `disposalAction` is required (no `Option` wrapper); `respectsLegalHold` defaults to `true` via serde.
4. **Lint promotion** — `WF-LINT-006` migrates from presence-only to shape-aware; emits `SA-WARN-pom-MIGRATE-RETENTION` for old `retentionPeriod` field, `SA-WARN-pom-WORKSPACE-RETENTION-EXT` for unknown unprefixed workspace-bag keys; resolves workspace-default inheritance (field-by-field merge) before validating "regulatoryBasis required when respectsLegalHold=false."
5. **Kernel-spec amendment** (parallel, blocking 4): `workflow-governance.md` §7.15 amendment pinning legal-hold clock-resume semantics. NOT a Studio commit; coordinate with WOS Working Group.

`STUDIO-DEFER-005` closes when 3 + 4 land. Step 5 is a kernel concern tracked separately.
