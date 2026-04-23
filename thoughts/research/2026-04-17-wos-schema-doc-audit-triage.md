# Schema Description Audit — Triage Report

**Date:** 2026-04-17
**Source data:** Python reimplementation of `SCHEMA-DOC-001` walker (logic mirrors `crates/wos-lint/src/rules/schema_doc.rs` exactly), run against all 19 schemas at commit `daec5b87d8c7e9cf44ba238f22fa28913b547406`.
**Total violations:** 901 (matches the Task 1 agent's report of 2026-04-17).
**Unique leaf pointers violated:** 603 (298 violations are dual — same property missing both description AND examples).
**Method:** Systematic sample of every 5th unique (schema, pointer) pair (121 of 603 sampled = 20% coverage). For each, checked: (1) fixture usage via grep across all 43 fixture files; (2) canonical spec prose citation (non-`.llm.md` files only); (3) lint rule references in `crates/wos-lint/`. Buckets extrapolated proportionally from sample.

---

## Summary by bucket

| Bucket   | Violations (est.) | % of 901 | Next step |
|----------|-------------------|-----------|-----------|
| Backfill | ~506              | ~56%      | Task 3: tier-by-tier backfill (~2 engineer-weeks); start with kernel/companions, highest adopter impact |
| Reshape  | ~394              | ~44%      | Consolidate `extensions`, `$schema`, `version`, `title`, `description`, `id`, and bare `items` into shared `$defs`; see §Reshape below |
| Delete   | <15               | <2%       | `chore(schemas): remove unused properties` — only after confirming zero spec intent in §Delete below |

**Note on dual-counting:** Each property can generate 0, 1, or 2 violations (one for description, one for examples). The bucket percentages are by violation count, not unique pointer count. The 56/44 split is consistent whether measured on violations or unique pointers.

---

## Per-schema breakdown

### `schemas/kernel/wos-kernel.schema.json` (86 violations, 63 unique pointers)

- **Dominant bucket:** Backfill (~70%), Reshape (~30%).
- The kernel is the most reference-heavy schema — 17 kernel fixtures, referenced by 10 lint rules (K-023 through K-031, K-EXT-001, K-EXT-002). Nearly every property has fixture or spec evidence. Most violations are "description too short" or "missing examples" on properties that already exist and are used.
- The bulk of reshape candidates here are `extensions` properties nested inside `$defs/ActorDeclaration`, `$defs/ExecutionConfig`, `$defs/ProvenanceConfig`, and `$defs/State` — all four follow the same `{ "type": "object", "propertyNames": { "pattern": "^x-" } }` pattern and could share a single `$defs/ExtensionsMap`.
- **Top 5 sampled offenders:**
  - `/properties/url` — desc 127 chars (need 140, critical); 2 examples. **Backfill** — 14 kernel fixtures reference `url`, K-023 rule checks it. Needs 13 more description chars and one more example.
  - `/properties/$wosKernel` — desc 134 chars (need 140, critical); 1 example (need 2). **Backfill** — every kernel fixture has this marker. Minor extension to existing text.
  - `/$defs/Action/properties/prefillMappingRef` — desc 140 chars (exact boundary, passes description); 0 examples. **Backfill** — referenced in spec prose (Kernel S11) and 2 spec files. Just needs concrete examples.
  - `/$defs/Action/properties/responseMappingRef` — same as above. **Backfill**.
  - `/$defs/State/properties/initialState` — desc 185 chars; 0 examples. **Backfill** — 19 fixtures use `initialState`, lint rule K-025 checks it. Just needs examples.
  - `/$defs/State/properties/tags/items` — desc 0, examples 0. **Reshape** — bare string items; merge into shared `$defs/TagString`.

### `schemas/kernel/wos-correspondence-metadata.schema.json` (23 violations, 17 unique pointers)

- **Dominant bucket:** Reshape (~53%), Backfill (~47%).
- Almost all violations are on cross-schema boilerplate: `$schema`, `version`, `title`, `description`, `targetWorkflow`, `extensions`. These 6 properties are repeated verbatim (or near-verbatim) across every companion/sidecar/governance schema. Consolidating into shared `$defs` would eliminate ~60% of this schema's violations instantly.
- **Top 5 sampled offenders:**
  - `/properties/$wosCorrespondenceMetadata` — desc 72 chars (need 140, critical); 1 example (need 2). **Backfill**.
  - `/properties/targetWorkflow` — desc 69 chars (need 140, critical); 1 example. **Backfill** — 15+ kernel fixtures reference `targetWorkflow` indirectly.
  - `/properties/title` — desc 58 chars (need 60); 1 example. **Reshape** — identical semantics across 16 schemas; candidate for `$defs/HumanReadableTitle`.
  - `/properties/version` — desc 56 chars (need 60). **Reshape** — identical SemVer semantics across 16 schemas; `$defs/SemVerString`.
  - `/$defs/EntryTemplate/properties/requiredFields/items` — desc 0, examples 0. **Reshape** — bare `string` items, no schema-specific semantics beyond string.

### `schemas/companions/wos-case-instance.schema.json` (106 violations, 68 unique pointers)

- **Dominant bucket:** Backfill (~75%), Reshape (~25%).
- The largest companion in terms of property count. Many properties are load-bearing runtime state: `instanceId`, `caseState`, `status`, `activeTasks`, `compensationLogs`, `historyStore`. All are referenced by spec prose in `specs/companions/runtime.md`. The reshape fraction comes from repeated `extensions`, `items` arrays on embedded role lists, and `additionalProperties` on mapping objects.
- **Top 5 sampled offenders:**
  - `/properties/instanceId` — desc 95 chars (need 140, critical); 2 examples (passes). **Backfill** — 1 fixture uses it; spec mandates it as the root identifier.
  - `/properties/caseState` — desc 179 chars; 1 example (need 2, critical). **Backfill** — foundational runtime property; needs a second concrete example.
  - `/properties/status` — desc 314 chars; 1 example (need 2, critical). **Backfill** — referenced by lint (status-related rules); needs a second example enum value illustrated.
  - `/properties/configuration/items` — desc 0, examples 0. **Reshape** — bare string array item; candidates for `$defs/ConfigurationRef`.
  - `/$defs/CompensationEntry/properties/compensatingAction` — desc 109 chars; 0 examples. **Backfill** — spec prose (Kernel S9.5) and 2 spec files reference `compensatingAction`.

### `schemas/companions/wos-lifecycle-detail.schema.json` (22 violations, 14 unique pointers)

- **Dominant bucket:** Reshape (~57%), Backfill (~43%).
- High boilerplate ratio: `$schema`, `version`, `title`, `targetWorkflow`, `extensions`, and the schema marker are all short/missing. The schema-specific properties (`CompensationConfig`, `ScxmlMappingConfig`) are partially documented and are Backfill.
- **Top 5 sampled offenders:**
  - `/properties/$wosLifecycleDetail` — desc 58 chars (need 140, critical); 1 example (need 2). **Backfill**.
  - `/properties/targetWorkflow` — desc 58 chars (need 140, critical); 1 example. **Backfill**.
  - `/$defs/CompensationScopeOverride/properties/onCompensationFailure` — desc 54 chars; 0 examples. **Backfill** — spec prose covers compensation failure modes.
  - `/$defs/ScxmlMappingConfig/properties/dropUnsupported` — desc 196 chars; 0 examples. **Backfill** — SCXML compatibility properties, 1 fixture, 0 spec prose (flag as spec gap).
  - `/properties/version` — desc 47 chars. **Reshape** — SemVer string, identical across schemas.

### `schemas/governance/wos-workflow-governance.schema.json` (97 violations, 65 unique pointers)

- **Dominant bucket:** Backfill (~62%), Reshape (~38%).
- Second-largest violation count. Many deep `$defs` properties (delegation, appeal, pipeline, assertion gate types) have brief descriptions and zero examples. `/$defs/Assertion/properties/type` is a critical property with only 38-char description. Several governance-specific terms (`quorumCount`, `separationOfDuties`, `maxDelegationDepth`) are referenced by lint rules and spec prose.
- **Top 5 sampled offenders:**
  - `/properties/$wosWorkflowGovernance` — desc 125 chars (need 140, critical); 1 example. **Backfill**.
  - `/$defs/Assertion/properties/type` — desc 38 chars (need 140, critical). **Backfill** — referenced by Governance S5.4 spec prose and lint rules.
  - `/$defs/Delegation/properties/quorumCount` — desc 113 chars; 0 examples. **Backfill** — spec prose (Governance S11.3) defines quorum semantics.
  - `/properties/schemaUpgrade/properties/priorVersion` — desc 0, examples 0. **Backfill** — parent `schemaUpgrade` is cited in spec (Governance S2.9); child properties are just undocumented.
  - `/$defs/SeparationOfDuties/properties/excludeRoles/items` — desc 0, examples 0. **Reshape** — role reference string; `$defs/RoleRef`.

### `schemas/governance/wos-assertion-gate.schema.json` (21 violations, 14 unique pointers)

- **Dominant bucket:** Reshape (~50%), Backfill (~50%).
- Smaller schema; half the violations are boilerplate (`$schema`, `title`, `description`, `version`, `extensions`, `url`). The schema-specific assertion definition properties need backfill.
- **Top 3 sampled offenders:**
  - `/properties/$wosAssertionLibrary` — desc 64 chars (need 140, critical). **Backfill**.
  - `/$defs/AssertionDefinition/properties/expression` — desc 49 chars; 2 examples. **Backfill** — FEL-anchored, spec prose (Governance S5).
  - `/$defs/AssertionDefinition/properties/fields/items` — desc 0, examples 0. **Reshape** — bare string field-name reference.

### `schemas/governance/wos-due-process.schema.json` (31 violations, 20 unique pointers)

- **Dominant bucket:** Reshape (~55%), Backfill (~45%).
- Notable boilerplate density: 7 of 20 unique violating pointers are cross-schema standard fields. Due-process-specific properties (`NoticeTemplate`, `ExplanationTemplate`, `AppealRouting`) need backfill.
- **Top 3 sampled offenders:**
  - `/properties/$wosDueProcess` — desc 60 chars (need 140, critical); 1 example. **Backfill** — every due-process fixture has this marker.
  - `/$defs/NoticeTemplate/properties/id` — desc 27 chars. **Backfill** — referenced by fixtures.
  - `/$defs/AppealRouting/properties/escalationPath/items` — desc 0, examples 0. **Reshape** — role/authority reference string.

### `schemas/governance/wos-policy-parameters.schema.json` (27 violations, 21 unique pointers)

- **Dominant bucket:** Backfill (~52%), Reshape (~48%).
- Both critical properties (`$wosPolicyParameters`, `targetWorkflow`) need description extension. The `ParameterDefinition` sub-schema properties (`type`, `authority`, `constraint`) are backfill — they carry semantic weight for typed policy values.
- **Top 3 sampled offenders:**
  - `/properties/$wosPolicyParameters` — desc 65 chars (need 140, critical); 1 example. **Backfill**.
  - `/$defs/ParameterDefinition/properties/type` — desc 21 chars; 2 examples. **Backfill** — distinguishes number/boolean/date/string policy values.
  - `/properties/description` — desc 49 chars. **Reshape** — near-identical short desc across many schemas.

### `schemas/ai/wos-ai-integration.schema.json` (93 violations, 65 unique pointers)

- **Dominant bucket:** Backfill (~58%), Reshape (~42%).
- Largest AI-tier schema. Deep property tree covering agent declarations, permissions, obligations, tool governance, drift detection. Most specific properties have some description text (30–60 chars) but fall short of the 60-char baseline. Reshape candidates are the repeated `id`, `description`, `enabled`, `method` leaves within different `$defs`.
- **Top 5 sampled offenders:**
  - `/properties/$wosAIIntegration` — desc 115 chars (need 140, critical); 1 example. **Backfill**.
  - `/$defs/Capability/properties/id` — desc 22 chars; 3 examples. **Backfill** — capability identifier is spec-defined (AI-Integration S4).
  - `/$defs/Permission/properties/bounds` — desc 37 chars; 1 example. **Backfill** — FEL-anchored bounds expression.
  - `/$defs/OversightPresentation/properties/showConfidence` — desc 37 chars; 0 examples. **Backfill** — referenced in spec prose.
  - `/$defs/VolumeConstraints/properties/maxAutonomousPerDay` — desc 41 chars; 2 examples. **Backfill** — 2 fixtures, 1 spec prose mention.

### `schemas/ai/wos-agent-config.schema.json` (39 violations, 24 unique pointers)

- **Dominant bucket:** Backfill (~58%), Reshape (~42%).
- Many critical properties need description extension: `$wosAgentConfig` (54 chars vs 140 needed), `targetAgent` (72 chars vs 140). Agent-config-specific properties (`DemotionRule`, `ActionOverride`, `CalibrationConfig`) are backfill.
- **Top 3 sampled offenders:**
  - `/properties/$wosAgentConfig` — desc 54 chars (need 140, critical). **Backfill**.
  - `/$defs/CalibrationConfig/properties/minimumSamples` — desc 47 chars; 2 examples. **Backfill** — calibration semantics defined in spec.
  - `/properties/approvedVersions/items` — desc 0, examples 0. **Reshape** — bare string version reference; `$defs/SemVerString`.

### `schemas/ai/wos-drift-monitor.schema.json` (29 violations, 21 unique pointers)

- **Dominant bucket:** Backfill (~57%), Reshape (~43%).
- Two interesting cases: `/$defs/MonitorMetric/properties/method` (desc 230 chars, 0 examples) and `/$defs/AlertThreshold/properties/action` (desc 222 chars, 0 examples). Both have rich descriptions but no examples — pure examples-only backfill. The spec defines their allowed values clearly.
- **Top 3 sampled offenders:**
  - `/$defs/MonitorMetric/properties/method` — desc 230 chars; 0 examples. **Backfill** — allowed values: `psi`, `ks`, `chi2`, `hellinger`.
  - `/$defs/AlertThreshold/properties/action` — desc 222 chars; 0 examples. **Backfill** — allowed values: `notify`, `escalate`, `suspend`.
  - `/$defs/DeploymentSequence/properties/canaryDuration` — desc 38 chars; 2 examples. **Backfill** — just a short description.

### `schemas/advanced/wos-advanced.schema.json` (126 violations, 78 unique pointers)

- **Dominant bucket:** Backfill (~55%), Reshape (~45%).
- Largest schema by unique violated pointers. Many advanced-tier properties (`MultiStepSession`, `SessionStep`, `ToolDefinition`, `CircuitBreaker`, `ShadowMode`, `RateLimit`, `VerifiableConstraint`) have zero description and zero examples — research-grade properties that were sketched but not documented. All have at least 1 fixture reference (the advanced fixture set is comprehensive), so these are backfill not delete.
- **Top 5 sampled offenders:**
  - `/$defs/MultiStepSession/properties/maxSteps` — desc 0, examples 0. **Backfill** — 1 fixture uses `maxSteps`; spec prose (Advanced S7.2).
  - `/$defs/ToolDefinition/properties/category` — desc 0; 3 examples. **Backfill** — just needs a description.
  - `/$defs/DriftMethod/properties/window` — desc 18 chars; 0 examples. **Backfill** — ISO 8601 duration, simple to document.
  - `/$defs/ShadowMode/properties/compareTo` — desc 0, examples 0. **Backfill** — 1 fixture; advanced S9 coverage.
  - `/$defs/SessionStep/properties/dependsOn/items` — desc 0, examples 0. **Reshape** — step identifier reference string.

### `schemas/advanced/wos-equity.schema.json` (36 violations, 20 unique pointers)

- **Dominant bucket:** Reshape (~55%), Backfill (~45%).
- Six of the top violating pointers are boilerplate (`$schema`, `version`, `title`, `extensions`, `targetWorkflow` marker, `id`). The equity-specific properties (`ProtectedCategory`, `DisparityMethod`, `RemediationTrigger`) need backfill.
- **Top 3 sampled offenders:**
  - `/properties/targetWorkflow` — desc 132 chars (need 140, critical); 1 example. **Backfill** — 3 chars short.
  - `/$defs/ProtectedCategory/properties/groupByPath` — desc 28 chars. **Backfill** — case-file path reference semantics.
  - `/$defs/DisparityMethod/properties/description` — desc 0, examples 0. **Reshape** — identical to all other `description` properties.

### `schemas/advanced/wos-verification-report.schema.json` (30 violations, 19 unique pointers)

- **Dominant bucket:** Backfill (~53%), Reshape (~47%).
- The `VerificationSummary` sub-schema has 5 integer properties (totalConstraints, provenSafe, provenUnsafe, inconclusive, totalSolverTimeMs) all with zero description and zero examples — these are the most actionable backfill targets (counting fields, easy to document).
- **Top 3 sampled offenders:**
  - `/properties/targetWorkflow` — desc 144 chars; 1 example (need 2, critical). **Backfill** — 1 more example needed.
  - `/$defs/VerificationSummary/properties/inconclusive` — desc 0, examples 0. **Backfill** — integer count, simple.
  - `/$defs/Counterexample/properties/inputs` — desc 41 chars; 1 example. **Backfill** — verification-specific, spec-defined.

### `schemas/assurance/wos-assurance.schema.json` (13 violations, 7 unique pointers)

- **Dominant bucket:** Backfill (~77%), Reshape (~23%).
- Smallest schema by violation count. All 7 unique violating pointers are in the `subjectContinuity` and `attestation` sub-objects — completely missing descriptions and examples. The spec (`specs/assurance/assurance.md`) covers these concepts. The only reshape candidate is `disclosurePosture` which has cross-schema semantic overlap with AI-tier disclosure properties.
- **Top 3 sampled offenders:**
  - `/properties/attestation/properties/subject` — desc 0, examples 0. **Backfill** — spec prose in `specs/assurance/assurance.md`.
  - `/properties/attestation/properties/predicate` — desc 0, examples 0. **Backfill** — W3C PROV concept, spec-defined.
  - `/properties/subjectContinuity/properties/reference` — desc 0, examples 0. **Backfill** — case reference semantics.

### `schemas/profiles/wos-integration-profile.schema.json` (51 violations, 39 unique pointers)

- **Dominant bucket:** Backfill (~62%), Reshape (~38%).
- Notable: `bindings` is critical (149-char description, 1 example — needs 2). `IntegrationBinding` sub-schema has multiple map-typed properties (`contextMapping`, `dataMapping`, `inputMapping`, `outputBinding`) with `additionalProperties` leaves at 0 description — these are reshape candidates (they all map string → string or string → path).
- **Top 5 sampled offenders:**
  - `/properties/bindings` — desc 149 chars; 1 example (need 2, critical). **Backfill** — 2 profile fixtures; spec anchor (Integration S3).
  - `/$defs/IntegrationBinding/properties/arazzoRef` — desc 76 chars (need 140, critical); 1 example. **Backfill** — integration spec (profiles/integration.md line 393).
  - `/$defs/IntegrationBinding/properties/dataMapping` — desc 155 chars; 0 examples. **Backfill** — spec defines the mapping direction semantics.
  - `/$defs/IntegrationBinding/properties/outputBinding/additionalProperties` — desc 0, examples 0. **Reshape** — mapping value pattern; `$defs/MappingValue`.
  - `/$defs/RetryPolicy/properties/backoff` — desc 140 chars; 0 examples. **Backfill** — backoff strategy string enum.

### `schemas/profiles/wos-semantic-profile.schema.json` (31 violations, 21 unique pointers)

- **Dominant bucket:** Reshape (~52%), Backfill (~48%).
- High boilerplate ratio for its size. Semantic-profile-specific violations: `contextUrl`/`contextVersion` (both have descriptions but zero examples), `actorMapping/additionalProperties` and `objectTypes/items` (both structural map/array leaves), `DomainVocabulary/description` (desc but no examples).
- **Top 3 sampled offenders:**
  - `/properties/$wosSemanticProfile` — desc 154 chars; 1 example (need 2, critical). **Backfill**.
  - `/$defs/TargetWorkflow/properties/url` — desc 48 chars (need 140, critical); 1 example. **Backfill** — critical url reference.
  - `/$defs/ProvMappingConfiguration/properties/actorMapping/additionalProperties` — desc 0, examples 0. **Reshape** — PROV actor mapping value; `$defs/MappingValue`.

### `schemas/sidecars/wos-business-calendar.schema.json` (18 violations, 12 unique pointers)

- **Dominant bucket:** Reshape (~58%), Backfill (~42%).
- 7 of 12 unique violating pointers are cross-schema boilerplate. Schema-specific properties (`Holiday`, `workWeek/items`) need attention: `Holiday.observed` has 189-char description but zero examples (pure examples backfill); `workWeek/items` is a bare string weekday name — reshape to `$defs/WeekdayName`.
- **Top 3 sampled offenders:**
  - `/$defs/Holiday/properties/observed` — desc 189 chars; 0 examples. **Backfill** — just needs `true`/`false` examples.
  - `/properties/workWeek/items` — desc 0, examples 0. **Reshape** — weekday string; could be `$defs/WeekdayName`.
  - `/properties/title` — desc 47 chars. **Reshape** — cross-schema boilerplate.

### `schemas/sidecars/wos-notification-template.schema.json` (22 violations, 15 unique pointers)

- **Dominant bucket:** Reshape (~53%), Backfill (~47%).
- Mix of boilerplate and notification-specific properties. `NotificationTemplate.description` (57 chars) and `TemplateSection.id` (55 chars) are both 3–5 chars short of the baseline — the lowest-effort backfills in the whole dataset.
- **Top 3 sampled offenders:**
  - `/properties/$wosNotificationTemplate` — desc 63 chars (need 140, critical); 1 example. **Backfill** — critical marker.
  - `/$defs/NotificationTemplate/properties/description` — desc 57 chars; 1 example. **Backfill** — 3 chars short.
  - `/$defs/TemplateSection/properties/id` — desc 55 chars; 5 examples. **Backfill** — 5 chars short.

---

## Reshape candidates

These properties appear across 3 or more schemas with near-identical semantics. Consolidating them into shared `$defs` (in a `schemas/shared/wos-common-defs.schema.json` or directly in each schema's `$defs` section as a cross-ref target) would eliminate a large fraction of boilerplate violations in a single pass.

### Candidate 1: `extensions` (30 schema occurrences, ~60 violations)

Every WOS schema declares an `extensions` property with the same semantics: `{ "type": "object", "propertyNames": { "pattern": "^x-" }, ... }`. The descriptions vary from 0 to 158 chars. The lint rule K-EXT-001/K-EXT-002 already validates extension keys — the property definition itself should be a single `$ref` target documented once.

**Proposed shape:**
```json
"$defs": {
  "ExtensionsMap": {
    "type": "object",
    "description": "Vendor extension data. All keys MUST start with 'x-'. The standard extension namespace is 'x-wos-*' (reserved for WOS Working Group use). Third-party extensions MUST use a unique prefix (e.g., 'x-acme-'). Processors MUST ignore unknown extension keys (forward-compatibility). Extension values are unconstrained — any JSON value is valid.",
    "examples": [
      { "x-acme-audit-ref": "AUD-2026-001" },
      { "x-vendor-custom-tier": "gold", "x-vendor-custom-region": "us-east" }
    ],
    "propertyNames": { "pattern": "^x-" },
    "additionalProperties": {}
  }
}
```

### Candidate 2: `$schema` (18 schema occurrences, ~36 violations)

Every schema declares `/properties/$schema` as an optional JSON Schema URI for editor tooling. All descriptions are either missing or the same 47-char text ("Optional JSON Schema URI for editor validation."). A shared ref would document it once with a concrete URI example.

**Proposed shape:**
```json
"$defs": {
  "JsonSchemaUri": {
    "type": "string",
    "format": "uri",
    "description": "Optional JSON Schema URI enabling editor validation and autocompletion. When present, editors (VS Code, IntelliJ, etc.) will validate the document against this schema. Omit in production; the schema URI is implicit from the document type marker (e.g., '$wosKernel': '1.0').",
    "examples": [
      "https://wos-spec.org/schemas/kernel/wos-kernel.schema.json",
      "https://wos-spec.org/schemas/governance/wos-workflow-governance.schema.json"
    ]
  }
}
```

### Candidate 3: `version` / `title` / `description` boilerplate (16 + 16 + 51 occurrences)

These three properties appear on almost every schema root with slightly varying short descriptions. The semantics are identical: SemVer string, display name, free-text summary. Rather than three separate `$defs` entries (which would add indirection), the practical fix is a template: copy the canonical long description from `wos-kernel.schema.json` (where it was most carefully written) and paste it into every sibling schema during Task 3. This is technically "backfill" for each schema, but the mechanical uniformity makes it reshape-class in terms of effort.

**Preferred approach:** During Task 3 kernel backfill, write authoritative descriptions for `version`, `title`, and `description` in the kernel schema. Copy those verbatim to every other schema (except the schema-specific `description` field of the `$wos*` marker, which varies). No shared `$defs` needed — the copy is the simplest solution.

### Candidate 4: bare `items` string arrays (41 occurrences, ~82 violations)

Forty-one array properties across all 19 schemas have bare `{ "type": "string" }` items with zero description and zero examples. These fall into named categories:

| Category | Examples | Proposed `$defs` name |
|----------|----------|----------------------|
| Role references | `notifyRoles/items`, `excludeRoles/items`, `suspensionNotifyRoles/items` | `$defs/RoleRef` |
| Tag/label strings | `tags/items`, `monitorTags/items` | `$defs/TagString` |
| Weekday names | `workWeek/items` | `$defs/WeekdayName` |
| Identifier strings | `dependsOn/items`, `requiredElements/items` | `$defs/IdentifierString` |
| Section strings | `sections/items` (governance notice templates) | `$defs/SectionString` |

**Recommended:** add `$defs/RoleRef` and `$defs/TagString` (highest frequency) in a kernel schema pass; reference them from all schemas. The remaining three are local to 1–2 schemas and can be documented inline.

### Candidate 5: map `additionalProperties` values (6 occurrences)

Six `additionalProperties` leaves (in `IntegrationBinding.contextMapping`, `IntegrationBinding.dataMapping`, `IntegrationBinding.inputMapping`, `IntegrationBinding.outputBinding`, `ProvMappingConfiguration.actorMapping`, `ProcessMiningConfiguration.objectTypes`) are bare string or object types with zero documentation. All represent key→value mappings from domain identifiers to case-file paths or external identifiers. A shared `$defs/MappingValue` (typed as `string`, documented as a case-file path or FEL expression) would cover the majority.

---

## Delete candidates

The audit instruction is to keep this list tight — "zero references today" is not always "unused." Based on the full evidence sweep:

**Confirmed candidate: none at this time.**

The only property class that could plausibly qualify is the `schemaUpgrade` sub-properties (`priorVersion`, `newVersion`, `migrationMechanism`, `scope`) in `wos-workflow-governance.schema.json`, which have zero fixture references and no spec prose citation at the property level. However:
- The parent property `schemaUpgrade` IS cited in spec prose (Governance S2.9).
- The four children are structural decomposition of the parent concept, not orphaned extensions.
- Removing them would make the parent object meaningless.

**Verdict: Backfill the `schemaUpgrade` children during the governance tier pass.** They are not delete candidates; they are undocumented-but-structural properties whose parent has spec coverage.

**Properties that should be watched but not deleted yet:**
- `wosdefversion` in `wos-integration-profile.schema.json` — 0 fixtures, but 1 spec prose mention (profiles/integration.md:393). Keep; add examples during profiles pass.
- All 26 "completely dark" properties in the advanced tier (e.g., `maxPerMinute`, `maxPerHour`, `probeCount`) — each has exactly 1 fixture reference. Research-grade but exercised. Backfill, not delete.

**If a delete candidate emerges during Task 3 backfill** — i.e., during the kernel or governance pass an author cannot find any fixture, spec prose, or lint rule that justifies a property — flag it in the commit message as a delete candidate and open a `chore:` follow-up. Do not delete speculatively from this audit alone.

---

## Recommended Task 3 sequencing

The violation counts and release-train assignments in the plan map cleanly to this priority order:

| Priority | Tier | Violations | Rationale |
|----------|------|-----------|-----------|
| 1 | Kernel (`wos-kernel`, `wos-correspondence-metadata`) | 109 | Most stable; highest adopter count; lint rules already reference these properties; fixes are unblocked by other tiers |
| 2 | Companions (`wos-case-instance`, `wos-lifecycle-detail`) | 128 | Runtime-facing; `wos-case-instance` is the companion most likely to be LLM-authored in live workflows; high-value Claim A ROI |
| 3 | Governance (`wos-workflow-governance`, `wos-assertion-gate`, `wos-due-process`, `wos-policy-parameters`) | 176 | Critical governance properties have high doc debt; fixing `$wosWorkflowGovernance` and `Assertion.type` unblocks LLM authoring of governance documents |
| 4 | AI (`wos-ai-integration`, `wos-agent-config`, `wos-drift-monitor`) | 161 | AI schemas are the most LLM-author-forward tier; fixing these has outsized Claim A impact |
| 5 | Profiles (`wos-integration-profile`, `wos-semantic-profile`) | 82 | Integration-tier; used by integrators, not end-users; important but lower urgency |
| 6 | Sidecars (`wos-business-calendar`, `wos-notification-template`) | 40 | Lowest violation count; sidecar documents are operator-configured, lower LLM-authoring frequency |
| 7 | Assurance (`wos-assurance`) | 13 | Smallest violation count; fix in a single short pass |
| 8 | Advanced (`wos-advanced`, `wos-equity`, `wos-verification-report`) | 192 | Research-grade; plan's explicit "lower bar acceptable"; do last but do not skip |

**Cross-cutting pre-work (do before Priority 1):** Before the tier-by-tier pass, add the shared `$defs/ExtensionsMap` and `$defs/JsonSchemaUri` shapes to the kernel schema and cross-reference them from all other schemas. This eliminates ~96 violations (30 `extensions` × ~2 + 18 `$schema` × ~2 = ~96) before any per-schema work begins, collapsing the backfill burden significantly.

**Suggested commit structure for Task 3:**
```
docs(schemas/shared): add $defs/ExtensionsMap and $defs/JsonSchemaUri, cross-ref across 19 schemas
docs(kernel): backfill SCHEMA-DOC-001 violations — wos-kernel and wos-correspondence-metadata
docs(companions): backfill SCHEMA-DOC-001 violations — wos-case-instance and wos-lifecycle-detail
docs(governance): backfill SCHEMA-DOC-001 violations — all four governance schemas
docs(ai): backfill SCHEMA-DOC-001 violations — ai-integration, agent-config, drift-monitor
docs(profiles): backfill SCHEMA-DOC-001 violations — integration-profile and semantic-profile
docs(sidecars): backfill SCHEMA-DOC-001 violations — business-calendar and notification-template
docs(assurance): backfill SCHEMA-DOC-001 violations — wos-assurance
docs(advanced): backfill SCHEMA-DOC-001 violations — advanced, equity, verification-report
```

---

## Notes and caveats

1. **Sample representativeness.** The 121-item sample (every 5th of 603 unique pointers) covers all 19 schemas and all 8 tiers. The smallest schemas (assurance: 7 unique pointers) have low sample density. For those, the per-schema analysis above augments the statistical sample with full-schema inspection.

2. **Dual-violation inflation.** The 901 figure counts violations, not properties. 298 properties triggered both a description violation and an examples violation. The bucket percentages are consistent regardless of whether you count violations or unique pointers; the 56%/44% split holds in both framings.

3. **Critical-property concentration.** 27 of the 603 unique violating pointers are on `x-lm.critical == true` properties. These have a higher remediation bar (140-char descriptions, 2 examples) but are the highest-value properties from a Claim A standpoint. They should be the first properties addressed within each tier during Task 3.

4. **Spec gaps found during audit.** Three spec-gap signals emerged:
   - `/$defs/ScxmlMappingConfig/properties/dropUnsupported` in lifecycle-detail has 196-char description but zero spec prose citation. During Task 3, verify whether `dropUnsupported` is fully specified in `specs/companions/lifecycle-detail.md` or whether the spec needs an update.
   - `schemaUpgrade` sub-properties (`priorVersion`, `newVersion`, `migrationMechanism`) have no spec prose at the property level despite the parent being cited. The governance spec may need a sub-section for schema upgrade mechanics.
   - `wosdefversion` in integration-profile has spec prose but zero fixtures. The integration spec section (line 393) should have a fixture exercising this field.

5. **Reshape vs. backfill boundary.** A property was classified as Reshape only if it appears in ≥3 schemas with near-identical semantics. Properties with schema-specific semantic weight (even if the name is generic, like `type` in `$defs/Assertion`) were classified as Backfill. Borderline cases were classified Backfill (conservative per the audit plan's "err on the side of not-listing" guidance for Delete, applied here to Reshape as well).

6. **Delete list is intentionally empty.** Every property examined either has fixture coverage (most common), spec prose coverage, or belongs to a documented object whose parent has coverage. In a greenfield project without legacy code to protect, the correct response is documentation, not deletion. If Task 3 authors find truly dead properties, they should flag them in commit messages rather than making a speculative delete based on this audit.
