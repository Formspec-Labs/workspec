# WOS runtime parity and vocabulary closure — chore + decision plan

**Status:** Chore (executable) + tracked decisions. **Not** an ADR; no architectural forks opened. Decision items here are sequenced for future ADR slots when ratification is needed; chore items run independently as gating allows.
**Date:** 2026-05-01. **Doc revision:** 2026-05-01 (final) — **C6/C7 ratchet-zero** on all `EXPECTED_OPEN_STRING_LEAVES` schemas; **C5 / WS-042** minimal checklist closed (version-indexed kernels + HTTP `Idempotency-Key` + `1.0.0 → 1.1.0` integration proof); **C7** `SCHEMA-OPEN-001` now runs inside `lint_schema`; **C8** implementation tracked in [`2026-05-01-wos-c8-graph-lint-k033-k034.md`](./2026-05-01-wos-c8-graph-lint-k033-k034.md).
**Scope (achieved for vocab parity):** Open-string-leaf debt is **0** under the current ratchet counter for every production schema in `open_string_leaf_ratchet.rs` (workflow full inner-block closure + honest-open annotation batch on tooling, case-instance, provenance-log, delivery, ontology-alignment via `wos-spec/scripts/annotate_open_string_kinds.py`). **Remaining open tails** are explicit: **C8** graph lint (child plan), **C1** optional `actor_type` serde storage, **C2** `auditCertificate.format` until signature profile ratifies, **PLN-0387** / **WS-090** same-transaction co-write, **D4–D7** items still called out in ADR 0083 where not yet ratified.

---

## Open-string-leaf parity vs `open_string_leaf_ratchet.rs`

**Yes — the goal is full parity with the ratchet table, in the sense of “no mystery debt,” not “keep nudging the same number forever.”**

`crates/wos-lint/tests/open_string_leaf_ratchet.rs` fixes **expected open-string-leaf counts per schema** (`EXPECTED_OPEN_STRING_LEAVES`). Today any leaf without `enum`/`const`/`pattern` counts as “open.” The ratchet **forces every change to be explicit**: tightening closes rows and **lowers** the baseline; regressions that reopen leaves fail CI.

**Terminal state (what “all the way” means):**

1. **C0** inventory classifies every open leaf (stable row IDs): close it with a schema constraint where appropriate, or mark it **HONESTLY-OPEN** with rationale.
2. **C6** executes closures cluster-by-cluster; each PR lowers `EXPECTED_OPEN_STRING_LEAVES` to match the leaves actually removed from the open set.
3. **C7** (`SCHEMA-OPEN-001` + `x-wos.openStringKind`) makes every remaining honest-open leaf **machine-auditable** so “open” is a declared choice, not an accident.
4. **Parity achieved** when the ratchet baselines **stop at the semantic floor** encoded in C0+C7: the numbers in `EXPECTED_OPEN_STRING_LEAVES` match “only leaves we intend to stay open under the current counter definition.” Optionally, a follow-up ADR can redefine the walker so C7-tagged leaves **do not** count as open—then baselines can be driven toward **0** for true “no unconstrained string leaves.” Until then, ~50–80 is the honest floor if honest opens still count as open.

**Wrong framing:** treating any intermediate ratchet baseline (for example a mid-pass **116** open leaves on `wos-workflow.schema.json`) as a permanent equilibrium. **Right framing:** the baseline is a **debt thermometer** that should **monotone down** with real closures until it rests only on classified, honest opens (and eventually zero under the current counter definition — **achieved 2026-05-01** for all schemas in `EXPECTED_OPEN_STRING_LEAVES`).

---

## Why this is a chore (not a phased program)

Per project discipline (see parent CLAUDE.md, "Sequential ADRs, not phased deliverables"), this plan does not number phases. Each item is independently executable and gated only by data dependencies, not calendar. Items mature through ratchets (`schema_doc_zero_regression`, `open_string_leaf_ratchet`, conformance suite, `cargo nextest run --workspace`), not through a Gantt position.

Repo-hygiene items (CI parity, `wos-server*` → `workspec-server`, `formspec-internal` rename, `focusconsulting` submodule deletion) are owned by parent [`thoughts/plans/2026-05-01-platform-repository-architecture.md`](../../../thoughts/plans/2026-05-01-platform-repository-architecture.md); this plan does not duplicate them.

**Pre-flight:** treat baselines and file:line citations as authoritative when committed. Reference state by `git log --oneline -- wos-spec/` for history; the durable inventory artifact (**C0**) and `open_string_leaf_ratchet.rs` are the load-bearing records for open-leaf work.

**Landed (runtime ⟷ conformance, same plan date):** Conformance **SIG-013** (`crates/wos-conformance/tests/fixtures/SIG-013-policy-assurance-below-floor.json`) now has a direct `wos-runtime` mirror: `submit_task_response_sig013_policy_assurance_below_floor_blocks_affirmation` in `crates/wos-runtime/src/runtime.rs` (`#[cfg(test)]`). It calls `WosRuntime::submit_task_response` with the same kernel/profile fixtures as the conformance case. The harness uses **`Sig013HarnessFormspecAdapter`** (same `"formspec"` binding key as the default test `TestAdapter`) because binding `validate_submission` runs *before* `signature_affirmation_for_submission`; `TestAdapter` only treats `data.approved` as valid and would return `TaskSubmissionResult::Failed { code: "validationFailed", … }` instead of reaching `identity_binding_meets_policy` and the policy-floor `RuntimeError::Signature` string asserted by SIG-013. The fixture `description` field cross-links the test and adapter by name.

**Landed (migration slice, ADR 0083 — 2026-05-01):** [`thoughts/adr/0083-wos-instance-migration-runtime-and-http.md`](../adr/0083-wos-instance-migration-runtime-and-http.md) is **Accepted** for D1/D2/D3/D3b; **C3**/**C4**/**C5** primitives are in-tree. Summary: `WosRuntime::migrate` (+ `MigrationMap` / `MigrationOutcome` / `validate_migration_configuration`), `ProvenanceKind::InstanceMigrated` + `ProvenanceRecord::instance_migrated`, `recordKind: instanceMigrated` on `FactsTierRecord`, `POST /api/instances/:id/migrate` (Supervisor), `RuntimeOps::migrate_instance` (local + explicit unsupported in Restate), unit tests (version bump + `stateNotFound`), HTTP integration for same-version idempotent migrate **and** cross-version `1.0.0 → 1.1.0` with **`Idempotency-Key`** replay (`runtime_lifecycle.rs`). **Storage:** `kernels` primary key `(url, version)`; bundle resolution keyed by `(workflow_url, definition_version)`. **Conformance:** `MIG-001` / `MIG-002` in `migration_conformance.rs`. **Explicitly still open (not WS-042):** durable same-transaction co-write with embedded event store (PLN-0387 / WS-090); full D5 posture (restart-safe dedupe, conflicting-body semantics) per ADR 0083 §Open decisions.

**Landed (WS-094 Phase 4 slice, Restate — 2026-05-01):** Execution plan [`thoughts/plans/2026-05-01-wos-restate-ws094-execution.md`](./2026-05-01-wos-restate-ws094-execution.md) Phase 4 checklist updated. **R-6.1:** parent `.github/workflows/ci.yml` job **`wos-restate-ingress-smoke`**, `wos-spec/Makefile` **`restate-ingress-smoke`**, `wos-spec/scripts/restate_ingress_smoke.sh`, binary **`wos-restate-worker`**, pinned server **`docker.restate.dev/restatedev/restate:1.6.2`**, ignored **`ingress_create_load_probe_smoke`**. **R-6.2 supplementary:** `crates/wos-conformance/tests/r6_restate_conformance_slice.rs` (SIG-013 + C.0/C.1 parity + terminal-failure tests vs `restate_signature_fixture_runtime` / ingress). **PLN-0333** lifecycle row **`Done`**; **WS-094** **`[✓]`**; remainder **WS-101..WS-105** in `wos-server/TODO.md` (tasks, provenance read, adapter migrate, Axum composition, retryable/stall).

**Landed (D3 Rust tail — 2026-05-01):** `HoldType` enum in `crates/wos-core/src/model/governance.rs` (seven standard literals + `Vendor(String)` matching `^x-[a-z][a-z0-9-]*$`); `HoldPolicy.hold_type` and `ActiveHold.hold_type` use it; `pub use` as `wos_core::HoldType`; HTTP `CreateHoldRequest` typed; integration tests use schema-valid `holdType` tokens (`crates/wos-server/tests/integration/ws_spec_gaps_2.rs`); `governance_deser` asserts + vendor / rejection tests.

**Landed (C7 + ratchet on small schemas — 2026-05-01):** `schemas/lint/wos-lint-diagnostic.schema.json` and `schemas/mcp/wos-mcp-tools.schema.json` carry `x-wos.openStringKind` on every formerly-open honest string leaf (prose / identifier / pathExpression as appropriate); `wos-lint` `leaf_string_has_value_constraint` now treats a **listed** `openStringKind` like `enum`/`const`/`pattern` for `inventory_string_leaves` / open-string ratchet. Ratchet: **lint 6→0**, **mcp 1→0**. See `CONVENTIONS.md` and `schema_doc.rs` tests `open_string_kind_is_inventory_constraint_when_value_is_allowed`.

**Landed (C6/C7 slice — Custody + Delegation + DelegationScope, 2026-05-01):** `/$defs/Custody` (`trustProfileRef`, `exportBundleRef`, `anchorRequirements[].on`, `anchorRequirements[].trellisLedger`), `/$defs/Delegation` (`id`, `delegator`, `delegate`, `legalInstrument`, `effectiveDate`, `expirationDate`, `revokedDate`, `quorumPool[]`), and `/$defs/DelegationScope` (`caseTypes[]`, `conditions`) carry `minLength: 1` where SCHEMA-DOC-001 requires it and `x-wos.openStringKind` (`uri`, `tagLabel`, `identifier`, `prose`, `timestamp`, `fel`). Ratchet `wos-workflow.schema.json`: **152 → 138** (14 leaves). C0 inventory **WS-041**–**WS-054** marked **CLOSED** in the parity inventory doc.

**Landed (C6/C7 slice — `wos-tooling` conformance-trace deltas, 2026-05-01):** Embedded conformance-trace guard shapes (`conformanceTrace__DeltaGuardFalse`, `conformanceTrace__DeltaPolicyOverride`, `conformanceTrace__DeltaStateMismatch`) — eight string leaves annotated with `x-wos.openStringKind` (`fel`, `identifier`, `tagLabel`, `prose`) plus `minLength: 1` where needed. Ratchet `wos-tooling.schema.json`: **54 → 46**. C0 **WT-001**–**WT-008** marked **CLOSED**.

**Landed (C6/C7 slice — DueProcess, EscalationStep, EvidenceReference, FactsTierRecord, FieldDeclaration, ReasoningTierConfig, HoldPolicy, JsonSchemaUri, 2026-05-01):** `DueProcess.scope`, `EscalationStep.assignTo`, `EvidenceReference` (`caseFieldPath`, `uri`, `summary`), `FactsTierRecord` scalars and `inputs`/`outputs`/`transitionTags` items, `FieldDeclaration.description`, `ReasoningTierConfig.requiredForTags` items, `HoldPolicy` (`description`, `notificationTemplateKey`, `resumeTrigger`, `scope`), and `$defs/JsonSchemaUri` carry `minLength: 1` / `x-wos.openStringKind` as appropriate (`fel`, `uri`, `pathExpression`, `prose`, `identifier`, `tagLabel`, `timestamp`). Ratchet `wos-workflow.schema.json`: **138 → 116** (22 leaves). C0 **WS-057**–**WS-076**, **WS-089**, **WS-110** marked **CLOSED**.

**Landed (C6 slice — provenance adjunct `recordKind`, 2026-05-01):** Fifteen single-kind `$defs` on `schemas/wos-workflow.schema.json` now pin `properties.recordKind` with `const` (plus `type: "string"`) matching existing `allOf`/`if` arms: `AmendmentAuthorizedRecord`, `AuthorizationAttestationRecord`, `AuthorizationRejectedRecord`, `CapabilityInvocationRecord`, `ClockResolvedRecord`, `ClockSkewObservedRecord`, `ClockStartedRecord`, `CommitAttemptFailureRecord`, `CorrectionAuthorizedRecord`, `DeterminationAmendedRecord`, `DeterminationRescindedRecord`, `IdentityAttestationRecord`, `MigrationPinChangedRecord`, `ReinstatedRecord`, `RescissionAuthorizedRecord`. Ratchet `wos-workflow.schema.json`: **189 → 174**. C0 inventory rows **WS-008**, **WS-027**, **WS-028**, **WS-031**, **WS-035**, **WS-036**, **WS-037**, **WS-038**, **WS-039**, **WS-055**, **WS-056**, **WS-077**, **WS-090**, **WS-111**, **WS-112** marked **CLOSED**. `FactsTierRecord.recordKind` stays the multi-kind **enum** (unchanged `$comment`).

**Landed (C6/C7 ratchet-zero — 2026-05-01):** Remaining open leaves on `wos-workflow.schema.json` closed to **0**; parallel schemas (`wos-tooling`, `wos-case-instance`, `wos-provenance-log`, `wos-delivery`, `wos-ontology-alignment`) annotated via `wos-spec/scripts/annotate_open_string_kinds.py` (heuristic `x-wos.openStringKind` + `minLength: 1`); `EXPECTED_OPEN_STRING_LEAVES` all **0**; `lint_schema` invokes `check_open_string_kinds` (CI-wide **SCHEMA-OPEN-001**).

**Open tails (post-parity-plan):**

1. **C8** — graph-membership lint **K-033** / **K-034** — child plan [`2026-05-01-wos-c8-graph-lint-k033-k034.md`](./2026-05-01-wos-c8-graph-lint-k033-k034.md).
2. **C1** — optional serde storage for `FactsTierRecord.actor_type` as `ActorKind` (wire-compat tradeoff).
3. **C2** — `Signature.auditCertificate.format` remains spec-gated.
4. **ADR 0083** — D4 posture, D5 full semantics, D6/D7 as recorded in ADR prose; **WS-090** / **PLN-0387** durable atomicity.

**No longer blocked:** **C3**/**C4**/**C5** minimal checklist and **C6/C7** ratchet-zero are **landed**; follow-on work is the explicit tails above or ADR-gated items — not a blanket “do not land.”

---

## Decisions (each opens its own ADR slot when ratification matures)

### D1 — Rename delivery `actorType` → `correspondenceRole`

**Why:** Naming collision. Kernel `actorType` ∈ {`human`, `system`, `agent`} (governance role); delivery sidecar `actorType` ∈ {`applicant`, `representative`, `third-party`, `system`, `agency`} (correspondence party role). Same property name, orthogonal vocab. Scout inventory called this out as the seam-hiding hazard. Renaming **before** any Rust-side closure prevents conflating two different enums in one Rust module.

**Settled name: `correspondenceRole`.** Rejected alternatives: `correspondenceActorType` (keeps the colliding "actor" word); `partyType` (legal-person framing leaks into `system`/`agency` cases that aren't natural persons). `role` already appears as a top-level concept in delivery; this stays consistent.

**Evidence:**

- `schemas/sidecars/wos-delivery.schema.json` — `correspondenceRole` enum (templates + entries; kernel collision avoided — see **D2** `specs/sidecars/delivery.md` §4.1).
- `schemas/wos-workflow.schema.json/$defs/Actor/properties/type` — kernel actor type oneOf.
- `crates/wos-core/src/model/kernel.rs` — `ActorKind` enum.

**Done when:**

- [x] Sidecar schema property renamed to `correspondenceRole`; descriptions and examples updated.
- [x] Fixtures / tests updated (search `correspondenceRole` in sidecar + integration paths).
- [x] Normative sentence in **`specs/sidecars/delivery.md` §4** (correspondenceRole vs kernel `actorType`).
- [x] Migration note in `COMPLETED.md` flagging the rename as a breaking sidecar property name (session 2026-05-01 — see `wos-spec/COMPLETED.md`).

---

### D2 — Delivery sidecar prose home

**Why:** Scout inventory flagged D-down: delivery sidecar schema closed vocabularies before a single normative envelope existed.

**Resolved:** **Option A** — [`specs/sidecars/delivery.md`](../../specs/sidecars/delivery.md) is the normative home; it incorporates calendar + notification references and **§4 Correspondence** (including `correspondenceRole` vs kernel `actorType` and §4.1 vocabulary table for `channel`, `direction`, `correspondenceRole`).

**Done when:**

- [x] Single normative prose document covers calendar + notification template + correspondence (`delivery.md`).
- [x] Closed correspondence vocabularies called out in prose (**§4.1**); deeper semantics remain in absorbed `business-calendar.md` / `notification-template.md` as linked.
- [x] Residual open-string leaves on `wos-delivery.schema.json` — **0** per ratchet (`annotate_open_string_kinds.py` + SCHEMA-DOC-001 gate, 2026-05-01). C0 row-level regeneration optional.

---

### D3 — `HoldPolicy.holdType` closed-vocab decision

**Why:** D-up: governance prose enumerates hold reasons; schema and runtime should agree. Runtime now uses `HoldType` in `crates/wos-core/src/model/governance.rs` on `HoldPolicy` and `ActiveHold` (schema `oneOf` + vendor pattern).

**Schema:** `HoldPolicy.holdType` in `schemas/wos-workflow.schema.json` is **`oneOf`**: closed enum (seven standard values, including `pending-external-verification` per schema/governance alignment) **or** `pattern: ^x-[a-z][a-z0-9-]*$` for vendor tokens.

**Governance:** [`specs/governance/workflow-governance.md`](../../specs/governance/workflow-governance.md) documents typed hold policies (S12); hold reason table includes `holdType` standard values + `x-*` extensibility.

**Done when:**

- [x] Schema property uses `oneOf: [enum, x-pattern]` (not free string).
- [x] Governance prose documents `holdType` vocabulary (S12 / hold policy table).
- [x] Rust runtime: `HoldType` enum in `crates/wos-core/src/model/governance.rs`; `HoldPolicy` + `instance::ActiveHold` + `wos_core::HoldType` re-export; HTTP create-hold request typed.
- [x] Schema rejects invalid `holdType` strings at validation time (JSON Schema).
- [x] Ratchet: `holdType` closure contributed to open-leaf reductions; current workflow baseline is in `EXPECTED_OPEN_STRING_LEAVES` (`open_string_leaf_ratchet.rs`).

---

### D4 — ADR 0083 open decisions (do NOT renumber here)

The migration ADR's **remaining** open decisions are recorded once, in [`thoughts/adr/0083-wos-instance-migration-runtime-and-http.md`](../adr/0083-wos-instance-migration-runtime-and-http.md). This plan does **not** fork the decision register by re-labeling them. Refer to ADR 0083 §"Open decisions" for the canonical list (**D4** preconditions / posture, **D5** idempotency model, **D6** error-model extensions, **D7** provenance ordering vs facts-tier).

**Gating consequence (revised 2026-05-01):** D1/D2/D3/D3b are **ratified**; **C3**/**C4**/**C5** minimal implementations **have landed** without waiting on D4–D7. Half-landing *would* have been the failure mode if the ADR had stayed at "Proposed" with no implementation snapshot — the ADR now records an **implementation snapshot** and explicitly **trigger-gates** D4–D7 for posture, **full** D5 idempotency semantics (beyond the reference-server replay cache), richer error taxonomy, and ordering refinements.

**Done when (D4 block):** D4–D7 each either closes in ADR prose or stays explicitly trigger-gated; **WS-042** reference-server row **closed 2026-05-01** (version-aware bundle + HTTP bump + `Idempotency-Key` — see ADR §Implementation snapshot).

---

### D5 — Vendor `x-*` assurance floor enforcement (fail-closed default)

Already recorded in `T4-TODO.md` ("Vendor `x-*` assurance floor enforcement (deferred-strict-mode)"). No duplicate scope here.

**Gating:** parent PLN-0384 (`wos-event-types.md` ratification) closes the namespace seam.

---

## Chores (independently executable)

### C0 — Commit a durable parity-inventory artifact

**Why:** Stable per-leaf row IDs let **C6** dispatch cold without walker-order drift.

**Artifact (committed):** [`thoughts/research/2026-05-01-schema-spec-crate-parity-inventory.md`](../research/2026-05-01-schema-spec-crate-parity-inventory.md) — top-line counts, per-schema tables with stable IDs (`WS-*`, `WT-*`, …), regeneration recipe. **Current ratchet (2026-05-01):** **0** open string leaves on every schema row in `open_string_leaf_ratchet.rs`; C0 per-row tables may lag until the next full regeneration pass (see inventory header note).

**Done when:**

- [x] Inventory document committed under `wos-spec/thoughts/research/`.
- [x] Stable row IDs assigned for the inventory snapshot (workflow through `WS-*`, other prefixes per schema).
- [x] C6 can reference row-ID ranges as work-unit boundaries.
- [x] Regeneration recipe in-file; diff new `schema_string_leaf_report` CSV output against the snapshot to detect drift.

**Gates:** none. Independent of all other chores; required by C6.

---

### C1 — D-runtime cleanup: typed enums for already-closed schemas

**Why:** Scout's highest-leverage finding. Schema is already closed; runtime spends `String`. Zero spec churn. Template is `CompletionRequirementKind` (`crates/wos-runtime/src/runtime/signature.rs:200-217`).

**Sites (status):**

| Schema field | Schema constraint | Crate field | Status |
|---|---|---|---|
| `WorkflowDocument.status` | `enum [draft, active, deprecated]` | `PublicationStatus` in `kernel.rs` | **Done** |
| `ActiveTask.impactLevel` | closed enum | `ImpactLevel` in `instance.rs` | **Done** |
| `FactsTierRecord.actorType` | closed enum | `actor_type: Option<String>` + `actor_kind()` / `set_actor_kind()` in `record.rs` | **Partial** — wire format still string; accessors enforce `ActorKind` |
| `FactsTierRecord.auditLayer` | closed enum | `AuditLayer` + getters/setters in `record.rs` | **Done** |
| `CaseStateMutation` mutation / verification | closed `$defs` | `MutationSource` / `VerificationLevel` in constructors (`record.rs`) | **Done** |

**Done when:**

- [x] Typed enums / accessors for the kernel/provenance sites above (remaining gap: serde field storage for `FactsTierRecord.actor_type` — optional hardening).
- [x] Serialization paths use canonical enum tokens (see `provenance/tests.rs` coverage).
- [x] Existing tests pass; `cargo nextest run --workspace` green.
- [ ] Scout pass confirms no stray `String` debt for these fields beyond the documented `actor_type` storage pattern — **optional follow-up** (**explicitly deferred** past this parity plan unless wire-compat review reopens it).

**Commit shape:** one PR per data home (`record.rs` + dependent payload constructors; then `kernel.rs`/`instance.rs`).

---

### C2 — D-up schema closure: signature vocabularies

**Why:** Scout flagged D-up. Spec is normative; schema is open. Aligning schema to spec costs nothing in spec churn.

**Sites:**

| Schema field | Spec citation | Closure shape |
|---|---|---|
| `Signature.documents.documentHashAlgorithm` | `specs/profiles/signature.md:157` (`sha-256` REQUIRED for Core; others MAY appear only via a future profile revision or an `x-*` extension policy) | `oneOf: [{const: "sha-256"}, {pattern: "^x-[a-z][a-z0-9-]*$"}]`. **Do not** preemptively add `sha-384`/`sha-512` to the core enum — that closes the schema tighter than the spec opens (D-down). If those algorithms become normative, ratify a Signature Profile revision first, then extend. |
| `Signature.auditCertificate.signingMode` | `signature.md` `signingFlow.type` mirrors `["sequential", "parallel", "routed", "free-for-all", "witness", "notary"]` | enum mirroring `SigningFlowType` (Rust enum at `signature.rs:251-256`) |
| `Signature.auditCertificate.format` | spec silent | **defer until spec ratification** — open D-up question, NOT a chore |
| `IdentityBindingRequirement.method` / `AuthenticationPolicy.method` | `signature.md:202` requires `in-person` / `notary` / `x-*` for notary roles | `oneOf: [enum: [closed canonical methods], pattern: ^x-…]`; new `IdentityMethod { Login, Credential, InPerson, Notary, X(VendorTag) }` Rust enum |

**Done when:**

- [x] Schema closures landed for `documentHashAlgorithm`, `identityBinding.method`, related authentication-policy shapes, and signing vocabularies per spec citations (`wos-workflow.schema.json`).
- [x] Existing fixtures validate; SIG-013 + runtime harness remain green.
- [x] Inventory/ratchet reflect closure (ratchet **0** open on `wos-workflow.schema.json` per `open_string_leaf_ratchet.rs`, 2026-05-01).
- [ ] `Signature.auditCertificate.format` — **still deferred** until signature profile prose ratifies allowed formats.

**Regression tripwire (already green):** Keep `submit_task_response_sig013_policy_assurance_below_floor_blocks_affirmation` + `signature.rs::assurance_binding_tests` passing when editing affirmation / `identity_binding_meets_policy` or the formspec binding seam ahead of a production `ContractBindingAdapter` for signature completions.

**Note:** `auditCertificate.signingMode` uses a focused enum in-schema; confirm parity with full `SigningFlowType` if profile adds modes (`witness`, `notary`, etc.).

---

### C3 — `WosRuntime::migrate` Rust implementation (per ADR 0083 D1)

**Why:** ADR 0083 D1 settled the API shape: method on `WosRuntime` with `migrate(&mut self, instance_id, target_definition_version, migration_map, operator_actor_id) -> Result<MigrationOutcome, RuntimeError>` (fourth parameter threads Supervisor identity into `instanceMigrated` provenance). **Minimal slice landed 2026-05-01** — this section tracks **residual** closure.

**Spec anchor:** kernel §11.2 (`specs/kernel/spec.md:1494-1535`) — state validation, case-state transformation, provenance, version update, atomicity.

**Done when:**

- [x] `MigrationMap` type defined per kernel §11.2 step 2 shape (`fieldRenames`, `fieldRemovals`, `fieldDefaults`, `fieldCoercions`).
- [x] `MigrationOutcome` carries instance id, prior/new `definitionVersion`, and applied `migration_map` (echo for audit correlation). **Still open (ADR D6):** richer “applied summary” / explicit mutated-field list if spec requires it beyond the map echo.
- [x] State-validation step rejects when target definition lacks any state currently in `instance.configuration` (kernel §11.2 step 1) — `stateNotFound` / `MigrationRejected` path.
- [x] Successful migration: exactly one `instanceMigrated` record appended, then `save_record` once (in-memory / local store: single persist call = atomic unit). **Still open:** durable embedded event-store co-commit with parent WS-090 / PLN-0387.
- [x] Failed migration: instance state untouched; no `instanceMigrated` record emitted.
- [x] Inline unit tests: happy path (version bump + provenance kind), `stateNotFound` rejection, migration-map rename/remove/default/coerce (including non-finite float rejection for `"number"` coercion).
- [x] Conformance fixtures `MIG-001-migrate-version-bump.json` (pass) and `MIG-002-migrate-state-not-found.json` (reject); see `migration_conformance.rs`.

**Gates:** ~~C4 before this lands~~ — **cleared**; C4 landed in the same integration pass.

---

### C4 — `instanceMigrated` ProvenanceKind variant

**Why:** ADR 0083 D3 settled the name. Registration **landed 2026-05-01** in:

- `crates/wos-core/src/provenance/kind.rs` / `record.rs` — `ProvenanceKind::InstanceMigrated`, `InstanceMigratedInput`, `ProvenanceRecord::instance_migrated`.
- `schemas/wos-workflow.schema.json#/$defs/FactsTierRecord/properties/recordKind` — `"instanceMigrated"` enum member.
- `schemas/wos-provenance-log.schema.json` — log `items` already `allOf` + `$ref` … `#/$defs/FactsTierRecord`; new `recordKind` validates through that merge (ADR 0076). **Optional follow-up:** add a dedicated `if/then` branch under `wos-provenance-log.schema.json` `$defs` if export validation needs payload fields stricter than the merged workflow `$defs` alone.
- Audit-tier mapping — facts-tier in `crates/wos-core/src/provenance/audit_tier.rs`.

**Done when:**

- [x] `recordKind` enum includes `instanceMigrated`.
- [x] Rust enum variant exists with payload shape (from/to version, migration map JSON, optional `actor_id`).
- [x] Constructor + coverage in `record` / runtime tests.
- [x] `schema_doc_zero_regression` green for touched workflow schema (and any new examples added with the enum extension).
- [x] No ad-hoc string `recordKind` for this event in the emit path.

**Gates:** satisfied for C3 — **complete** unless the optional provenance-log `if/then` tightening is scheduled.

---

### C5 — `POST /api/instances/:id/migrate` HTTP route (WS-042)

**Why:** ADR 0083 D2 settled the route home (wos-server). **Route + handler + reference-server checklist closed 2026-05-01** (`crates/wos-server/src/http/instances.rs`, `AppState::migrate_idempotency`, storage `(url, version)` kernels).

**Done when:**

- [x] Route handler parses migration body, calls `RuntimeOps::migrate_instance` / runtime migrate path, maps `RuntimeError` → `ApiError` in `crates/wos-server/src/error.rs` (including `MigrationRejected` → **400**, resolver/kernel mismatch → **400**/**404**, `FeatureDisabled` when runtime-local off → **400**).
- [x] **Idempotency-Key** honored for successful duplicate POST semantics (minimal D5 slice — in-memory replay cache).
- [x] Error mapping covers migration + resolver + feature-off paths; auth via existing `RequireRole<Supervisor>`. **Note:** same-version migrate is a **runtime no-op** (no store write); HTTP returns success with consistent body — not necessarily **409** unless full D5 standardizes conflict semantics for duplicates.
- [x] Integration tests in `runtime_lifecycle.rs`: same-version idempotent migrate; duplicate same-version with idempotency key; **cross-version** `1.0.0 → 1.1.0` with idempotency replay.
- [x] WS-042 marked **complete** in `crates/wos-server/TODO.md`.

**Gates:** **cleared** for the WS-042 reference-server checklist; ADR **D4–D7** full posture and **WS-090** / **PLN-0387** atomicity remain separately tracked.

---

### C6 — Long-tail open-leaf hardening (workflow inner-block schema-by-schema)

**Status 2026-05-01:** **Complete** for ratchet-registered production schemas — `EXPECTED_OPEN_STRING_LEAVES` is **all zeros**; `schema_doc_zero_regression` + `open_string_leaf_ratchet` + `schema_open_string_kind` (via `lint_schema`) are the enforcement surface.

**How it landed:** workflow `$defs` clusters were closed in prior slices (see bullets above through DueProcess / Signature / …); the final tail went **116 → 0** on `wos-workflow.schema.json`; parallel files used `wos-spec/scripts/annotate_open_string_kinds.py` (CSV-driven pointers + heuristic `x-wos.openStringKind`) plus ratchet decrements. **Follow-up:** re-run the C0 regeneration recipe when someone wants per-row `CLOSED` cells to match the walker again.

**Per-leaf classification:** every open leaf MUST land in one of:

- closed-vocab → `enum`/`const`
- extensible-vocab → `oneOf: [enum, pattern: "^x-…"]`
- document-relative id → `pattern` per ADR-0060 / kernel convention
- URI → `format: uri` (or stricter pattern for URN family)
- hash digest → `pattern` keyed to algorithm
- ISO timestamp → `format: date-time`
- TypeID → `format: wos-record-typeid`
- FEL expression → `minLength: 1` + description marker `FEL`
- free prose → `minLength: 1` + description marker `prose` / `narrative` / `title`

**Work-unit dispatch:** committed inventory from **C0** assigns stable row IDs (`WS-001..` for workflow, etc.) and per-cluster targets. A PR for C6 names its row-ID range explicitly: e.g., *"close `WS-008`, `WS-027`–`WS-039`, … (provenance adjunct `recordKind` consts); ratchet `wos-workflow.schema.json`: **189 → 174**"* (landed 2026-05-01). Without C0's stable IDs, C6 is too wide to dispatch.

**Done when (landed 2026-05-01 — provenance `recordKind` const batch):**

- [x] PR named C0 row IDs **WS-008**, **WS-027**, **WS-028**, **WS-031**, **WS-035**, **WS-036**, **WS-037**, **WS-038**, **WS-039**, **WS-055**, **WS-056**, **WS-077**, **WS-090**, **WS-111**, **WS-112** (fifteen single-kind `$defs`).
- [x] Ratchet `wos-workflow.schema.json` **189 → 174**; C0 rows marked **CLOSED**; no SCHEMA-DOC-001 / conformance regressions on that slice.

**Done when (landed 2026-05-01 — lint/MCP honest-open + ratchet):**

- [x] **WL-001..WL-006** + **WM-001** annotated with `x-wos.openStringKind`; `leaf_string_has_value_constraint` counts listed kinds for inventory; ratchet **lint 6→0**, **mcp 1→0**; C0 + `CONVENTIONS.md` updated.

**Done when (landed 2026-05-01 — Assertion cluster honest-open):**

- [x] `$defs/Assertion`, `AssertionDefinition`, `AssertionInlineUse`, `AssertionReference`: `minLength` where needed for SCHEMA-DOC-001; `x-wos.openStringKind` on prose / FEL / path / identifier / URI leaves; ratchet `wos-workflow.schema.json` **174 → 161** (13 leaves).

**Done when (landed 2026-05-01 — AuthorityBasis + BreachPolicy + CaseFile + CaseFileSnapshot + CounterfactualTierConfig):**

- [x] C0 rows **WS-024**–**WS-026**, **WS-029**–**WS-030**, **WS-032**–**WS-034**, **WS-040**: `minLength` / `x-wos.openStringKind` as appropriate; ratchet `wos-workflow.schema.json` **161 → 152** (9 leaves).

**Done when (landed 2026-05-01 — DueProcess / EscalationStep / EvidenceReference / FactsTierRecord / FieldDeclaration / ReasoningTierConfig / HoldPolicy / JsonSchemaUri):**

- [x] C0 rows **WS-057**–**WS-076**, **WS-089**, **WS-110**: `minLength` / `x-wos.openStringKind` as appropriate; ratchet `wos-workflow.schema.json` **138 → 116** (22 leaves).

**Done when (ratchet-zero batch — 2026-05-01):**

- [x] `wos-workflow.schema.json` open leaves **0**; parallel schemas **0**; `EXPECTED_OPEN_STRING_LEAVES` updated.
- [x] No SCHEMA-DOC-001 regressions (`lint_schema`).
- [x] Conformance gates unchanged on this pass (`cargo test -p wos-conformance` as CI runs).

**Done when (overall):** Satisfied — every schema in the ratchet table is at **0** open leaves under the current counter; honest opens carry **`x-wos.openStringKind`**. Optional future: redefine the walker to exclude C7-tagged leaves from the open count (ADR + ratchet-reset PR).

**Cold-read test:** a future agent reads C0's committed inventory, picks an unclosed row range, executes the closure per the proposed shape in that row, decrements the ratchet, updates C0. No prior conversation context needed.

---

### C7 — `SCHEMA-OPEN-001` lint rule (structured `x-wos.openStringKind` annotation)

**Why:** Honest-open leaves (free prose, FEL, opaque IDs) lack `enum`/`const`/`pattern` by design. Structured **`x-wos.openStringKind`** makes that choice auditable instead of accidental.

**Shape: structured vendor extension, not description text-grep.** Description-text matching is too gameable (a description with the word "prose" in a parenthetical false-positives; a legitimately-prose leaf that doesn't say the word false-negatives). Mirror the existing `x-lm.critical` precedent: at any string leaf without `enum`/`const`/`pattern`, require an `x-wos.openStringKind` annotation drawn from a closed enum:

```jsonc
"someProseField": {
  "type": "string",
  "minLength": 1,
  "x-wos": { "openStringKind": "prose" },
  "description": "...",
  "examples": ["..."]
}
```

Allowed `x-wos.openStringKind` values (initial closed set; extend via ADR):

- `prose` — free narrative / titles / descriptions.
- `fel` — FEL expression source.
- `uri` — URIs constrained by `format: uri` already.
- `identifier` — opaque external ID where shape varies (provider IDs, correlation keys).
- `pathExpression` — JSONPath / dotted path / pointer.
- `hash` — hash digest (paired with `pattern` for the family; this marker covers the "we know what algorithm but the value is opaque" case).
- `timestamp` — ISO 8601 (paired with `format: date-time`).
- `tagLabel` — author-allocated semantic tag (ADR 0077 lifecycleHook seam).

Description still required (SCHEMA-DOC-001 unchanged); the annotation says **why open**, not **what value**.

**Lint behavior:** at a string leaf with no `enum`/`const`/`pattern`, fail unless `x-wos.openStringKind` is present and matches the closed enum.

**Done when:**

- [x] `SCHEMA-OPEN-001` registered in `crates/wos-lint/src/rules/registry.rs`.
- [x] Inline tests in `crates/wos-lint/tests/schema_open_string_kind.rs` (present / missing / unknown value).
- [x] `wos-lint-diagnostic` + `wos-mcp-tools`: every formerly open honest string leaf carries `x-wos.openStringKind`; ratchet baselines **0** each; inventory rows **WL-001..WL-006** and **WM-001** marked **CLOSED** (2026-05-01).
- [x] All production schemas in the ratchet carry listed `openStringKind` where needed; **`lint_schema`** calls `check_open_string_kinds` (CI-wide **SCHEMA-OPEN-001**, 2026-05-01).
- [x] `LINT-MATRIX.md` entry for `SCHEMA-OPEN-001`.
- [x] `x-wos.openStringKind` documented in `wos-spec/CONVENTIONS.md` (includes ratchet alignment: listed `openStringKind` counts as constrained in `inventory_string_leaves`).

**Gates:** satisfied with ratchet-zero + `lint_schema` wiring (2026-05-01).

---

### C8 — Lint-rule coverage for graph membership beyond initialState

**Why:** K-031 (Transition.actor membership) and K-032 (initialState resolution) are landed. Remaining graph-membership rules JSON Schema cannot express:

- Transition `target` resolves to a sibling state (or to a substate via dotted path).
- Compound state's outbound transitions terminate inside the same parent or escape to a sibling explicitly.
- Parallel-region join targets resolve.

**Done when:**

- [ ] Lint rule(s) for transition targets / compound-region joins / parallel join targets (plan placeholder **K-033** / **K-034** — distinct from **K-032**, which already covers compound `initialState` resolution with the same rule id per `tier1.rs` comments).
- [ ] Spec citations for each rule.
- [ ] Inline tests + LINT-MATRIX entries.

**Scope split (2026-05-01):** This parity plan treats **C8** as **deferred** to [`2026-05-01-wos-c8-graph-lint-k033-k034.md`](./2026-05-01-wos-c8-graph-lint-k033-k034.md) so vocab/runtime parity can close independently.

**Gates:** none. Independent of C6/C7.

---

## Out of scope (named so they don't drift in)

- **Repo hygiene:** owned by parent `thoughts/plans/2026-05-01-platform-repository-architecture.md` §3.1–3.4.
- **Posture Declaration registry:** parent PLN-0384.
- **Studio authoring/validation UX (T4-11):** owned by `T4-TODO.md`.
- **Trellis cert-of-completion rendering (T4-10):** owned by `T4-TODO.md`.
- **`fel-core` independent publication:** parent VISION rejection-list item; reopen only with a forcing function.
- **Polyrepo end state:** parent VISION rejection-list item.

---

## Risks

- **Schema-edit conflicts with concurrent agents.** Multiple chores (C1, C2, C6) edit `wos-workflow.schema.json`. Sequence them per `$defs` cluster, not per chore — one cluster per PR, never two clusters simultaneously.
- **Breaking serde changes (C1).** Rust struct field type changes from `String` to typed enum break any consumer feeding non-canonical values. Mitigation: schema is the source of truth; consumers were already supposed to validate against it. Document each break in `COMPLETED.md` per session.
- **Migration atomicity (C3).** The **landed** primitive uses one `save_record` on the runtime store (sufficient for in-memory / local SQLite path today). A **single transaction** spanning case state + separate audit/event-store append is still gated on parent stack closure (PLN-0387, WS-090). Mitigation: treat current behavior as the reference-server slice; tighten when embedded event store lands.
- **Scripted schema annotation drift (C6).** `annotate_open_string_kinds.py` uses heuristics; if a leaf's semantics change, re-verify `openStringKind` against `CONVENTIONS.md`. Mitigation: ratchet + `lint_schema` (SCHEMA-OPEN-001) catch missing/invalid kinds on edit.

---

## Verification gates (apply per PR)

- `cargo nextest run --workspace` — green.
- `python3 -m pytest tests/ -q` — green.
- `cargo nextest run -p wos-lint --test schema_doc_zero_regression --test open_string_leaf_ratchet --test schema_open_string_kind` — green.
- `cargo nextest run -p wos-conformance` — green (signature_profile + SIG-013 + any new MIG-* fixtures).
- `cargo test -p wos-runtime submit_task_response_sig013` — green (direct SIG-013 policy-floor path; complements conformance harness).
- `cargo test -p wos-runtime migrate_ --lib` — green after any `migrate` / `MigrationMap` change.
- `cargo test -p wos-server --test integration migrate_instance` — green after HTTP migrate / `RuntimeOps` signature changes.
- `cargo test -p wos-server-runtime-restate --lib` — green when `migrate_instance` contract changes (unsupported path must stay explicit).
- `make -C wos-spec restate-ingress-smoke` — green when changing Restate VO / ingress client / worker (**Docker**; same pins as [`wos-server-runtime-restate/README.md`](../../crates/wos-server-runtime-restate/README.md)).
- `cd studio && npm run types:gen && git diff` — types regenerate cleanly.
- `cargo run -q --example count_schema_violations -p wos-lint -- <touched-schema>` — open count matches new ratchet baseline.

---

## References

- Parent CLAUDE.md "Sequential ADRs, not phased deliverables" memory.
- Parent [`VISION.md`](../../../VISION.md) — stack-wide commitments.
- Parent [`thoughts/plans/2026-05-01-platform-repository-architecture.md`](../../../thoughts/plans/2026-05-01-platform-repository-architecture.md) — repo hygiene.
- `wos-spec/CLAUDE.md` — submodule conventions, schema structure, ratchet gates.
- `wos-spec/T4-TODO.md` — Signature Profile track + vendor-floor enforcement entry.
- `wos-spec/COMPLETED.md` — historical record (read backward for context).
- `wos-spec/thoughts/adr/0083-wos-instance-migration-runtime-and-http.md` — migration ADR.
- `wos-spec/specs/kernel/spec.md` §9.6, §11.1, §11.2 — instance versioning + migration normative prose.
- `wos-spec/specs/profiles/signature.md` §2.7, §2.10, §2.13 — signature normative prose.
- `wos-spec/crates/wos-lint/tests/open_string_leaf_ratchet.rs` — per-schema open-string-leaf baseline table (guardrail during C6; terminal parity when baselines match C0+C7 semantic floor, optionally 0 after counter-definition ADR).
- `wos-spec/crates/wos-conformance/tests/fixtures/SIG-013-policy-assurance-below-floor.json` — conformance SIG-013 (policy assurance below floor); description links runtime harness.
- `wos-spec/crates/wos-runtime/src/runtime.rs` — `Sig013HarnessFormspecAdapter`, `runtime_with_kernel_sig013_harness`, `submit_task_response_sig013_policy_assurance_below_floor_blocks_affirmation`; `MigrationMap` / `migrate_*` unit tests.
- `wos-spec/crates/wos-runtime/src/runtime/instance.rs` — `WosRuntime::migrate` (kernel §11.2, idempotency doc).
- `wos-spec/crates/wos-server/src/http/instances.rs` — `POST …/migrate`, request validation.
- `wos-spec/crates/wos-server-runtime-local/src/resolver.rs` — `RuntimeKernelResolver` (typed resolver errors for HTTP mapping).
- `wos-spec/crates/wos-server/tests/integration/runtime_lifecycle.rs` — `migrate_instance_via_http_same_version_is_idempotent`.
- `wos-spec/crates/wos-server/TODO.md` — **WS-042** partial vs complete criteria.
- [`thoughts/research/2026-05-01-schema-spec-crate-parity-inventory.md`](../research/2026-05-01-schema-spec-crate-parity-inventory.md) — stable row-ID inventory (C0).
- `wos-spec/crates/wos-lint/tests/schema_open_string_kind.rs` — **SCHEMA-OPEN-001** coverage.
- `wos-spec/crates/wos-conformance/tests/fixtures/MIG-001-migrate-version-bump.json`, `MIG-002-migrate-state-not-found.json` — migration conformance.
- `wos-spec/thoughts/plans/2026-05-01-wos-restate-ws094-execution.md` — WS-094 Phase 3–4 checklist (Restate).
- `wos-spec/scripts/restate_ingress_smoke.sh`, `wos-spec/crates/wos-server-runtime-restate/src/bin/wos-restate-worker.rs` — Phase 4 CI replay harness.
- `wos-spec/crates/wos-conformance/tests/r6_restate_conformance_slice.rs` — R-6.2 supplementary parity slice.
- `wos-spec/specs/sidecars/delivery.md` — delivery / correspondence prose (**D2**).
- 2026-05-01 wos-scout parity inventory (run history; regenerable via `cargo run -q --example schema_string_leaf_report` per schema).
