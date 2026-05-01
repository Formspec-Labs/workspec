# WOS runtime parity and vocabulary closure — chore + decision plan

**Status:** Chore (executable) + tracked decisions. **Not** an ADR; no architectural forks opened. Decision items here are sequenced for future ADR slots when ratification is needed; chore items run independently as gating allows.
**Date:** 2026-05-01.
**Scope:** Drive open-string-leaf count from current 394 toward the **semantic floor** (~50–80 under today’s counter, optionally **0** if the leaf-inventory definition later excludes C7-annotated honest opens) by closing schema↔spec↔crate parity gaps surfaced in the 2026-05-01 wos-scout parity inventory; **land** the ADR 0083 migration **primitive** (runtime + HTTP + provenance kind) and **finish** the remaining migration checklist items (conformance, cross-version HTTP, idempotency keys, durable store semantics — see **C3**/**C5** and **WS-042**); back the delivery sidecar's already-closed enums with normative prose; clean up the residual D-runtime drift where Rust still parses already-closed schemas as `String`.

---

## Open-string-leaf parity vs `open_string_leaf_ratchet.rs`

**Yes — the goal is full parity with the ratchet table, in the sense of “no mystery debt,” not “keep nudging the same number forever.”**

`crates/wos-lint/tests/open_string_leaf_ratchet.rs` fixes **expected open-string-leaf counts per schema** (`EXPECTED_OPEN_STRING_LEAVES`). Today any leaf without `enum`/`const`/`pattern` counts as “open.” The ratchet **forces every change to be explicit**: tightening closes rows and **lowers** the baseline; regressions that reopen leaves fail CI.

**Terminal state (what “all the way” means):**

1. **C0** inventory classifies every open leaf (stable row IDs): close it with a schema constraint where appropriate, or mark it **HONESTLY-OPEN** with rationale.
2. **C6** executes closures cluster-by-cluster; each PR lowers `EXPECTED_OPEN_STRING_LEAVES` to match the leaves actually removed from the open set.
3. **C7** (`SCHEMA-OPEN-001` + `x-wos.openStringKind`) makes every remaining honest-open leaf **machine-auditable** so “open” is a declared choice, not an accident.
4. **Parity achieved** when the ratchet baselines **stop at the semantic floor** encoded in C0+C7: the numbers in `EXPECTED_OPEN_STRING_LEAVES` match “only leaves we intend to stay open under the current counter definition.” Optionally, a follow-up ADR can redefine the walker so C7-tagged leaves **do not** count as open—then baselines can be driven toward **0** for true “no unconstrained string leaves.” Until then, ~50–80 is the honest floor if honest opens still count as open.

**Wrong framing:** treating the current baseline (e.g. 193 on `wos-workflow.schema.json`) as a permanent equilibrium. **Right framing:** the baseline is a **debt thermometer** that should **monotone down** with real closures until it rests only on classified, honest opens (and eventually zero if the counter definition is tightened).

---

## Why this is a chore (not a phased program)

Per project discipline (see parent CLAUDE.md, "Sequential ADRs, not phased deliverables"), this plan does not number phases. Each item is independently executable and gated only by data dependencies, not calendar. Items mature through ratchets (`schema_doc_zero_regression`, `open_string_leaf_ratchet`, conformance suite, `cargo nextest run --workspace`), not through a Gantt position.

Repo-hygiene items (CI parity, `wos-server*` → `flowspec-server`, `formspec-internal` rename, `focusconsulting` submodule deletion) are owned by parent [`thoughts/plans/2026-05-01-platform-repository-architecture.md`](../../../thoughts/plans/2026-05-01-platform-repository-architecture.md); this plan does not duplicate them.

**Pre-flight:** this plan assumes the 2026-05-01 hardening pass has been committed before any C-item lands. While the pass remains uncommitted, treat baselines and file:line citations as living. Reference state by `git log --oneline -- wos-spec/` after commit, not by working-tree folklore. The durable inventory artifact captured in **C0** below is the load-bearing record; everything else is a pointer into git.

**Landed (runtime ⟷ conformance, same plan date):** Conformance **SIG-013** (`crates/wos-conformance/tests/fixtures/SIG-013-policy-assurance-below-floor.json`) now has a direct `wos-runtime` mirror: `submit_task_response_sig013_policy_assurance_below_floor_blocks_affirmation` in `crates/wos-runtime/src/runtime.rs` (`#[cfg(test)]`). It calls `WosRuntime::submit_task_response` with the same kernel/profile fixtures as the conformance case. The harness uses **`Sig013HarnessFormspecAdapter`** (same `"formspec"` binding key as the default test `TestAdapter`) because binding `validate_submission` runs *before* `signature_affirmation_for_submission`; `TestAdapter` only treats `data.approved` as valid and would return `TaskSubmissionResult::Failed { code: "validationFailed", … }` instead of reaching `identity_binding_meets_policy` and the policy-floor `RuntimeError::Signature` string asserted by SIG-013. The fixture `description` field cross-links the test and adapter by name.

**Landed (migration slice, ADR 0083 — 2026-05-01):** [`thoughts/adr/0083-wos-instance-migration-runtime-and-http.md`](../adr/0083-wos-instance-migration-runtime-and-http.md) is **Accepted** for D1/D2/D3/D3b; **C3**/**C4**/**C5** primitives are in-tree. Summary: `WosRuntime::migrate` (+ `MigrationMap` / `MigrationOutcome` / `validate_migration_configuration`), `ProvenanceKind::InstanceMigrated` + `ProvenanceRecord::instance_migrated`, `recordKind: instanceMigrated` on `FactsTierRecord`, `POST /api/instances/:id/migrate` (Supervisor), `RuntimeOps::migrate_instance` (local + explicit unsupported in Restate), unit tests (version bump + `stateNotFound`), HTTP integration for **same-version** idempotent migrate. Post–semi-formal-review hardening: supervisor `operator_actor_id` threaded into provenance; `RuntimeKernelResolver` maps resolver/version errors to **400/404** (not blanket **503**); empty/whitespace `target_definition_version` rejected at HTTP; `fieldCoercions` `"number"` rejects non-finite floats; `RuntimeError::FeatureDisabled` when `runtime-local` is off (**400**); Restate tests assert `migrate_instance` unsupported. **Still open vs this plan:** `MIG-*` conformance fixtures (C3 checklist), HTTP **Idempotency-Key** (ADR D5), cross-version HTTP proof (bundle keyed by URL only — see `crates/wos-server/TODO.md` **WS-042**), durable same-transaction co-write with embedded event store (parent PLN-0387 / WS-090).

**First pickups (no gates closed against them):**

1. **C0** — commit the parity inventory. Required by C6; everything else benefits.
2. **C1** — D-runtime typed-enum cleanup. Pure schema-runtime parity, zero spec churn, six concrete sites.
3. **C6/C7** tail — only after C0 row IDs exist; can proceed in parallel with C1 if merge conflicts are managed per cluster.

**No longer blocked:** **C3**/**C4**/**C5** shipped their **minimal** slices under ADR 0083; remaining migration work is **incremental** (fixtures, resolver indexing, idempotency, durable atomicity) and is tracked in ADR 0083 §Open decisions (D4–D7), **WS-042**, and the C3/C5 checklists below — not a blanket “do not land.”

---

## Decisions (each opens its own ADR slot when ratification matures)

### D1 — Rename delivery `actorType` → `correspondenceRole`

**Why:** Naming collision. Kernel `actorType` ∈ {`human`, `system`, `agent`} (governance role); delivery sidecar `actorType` ∈ {`applicant`, `representative`, `third-party`, `system`, `agency`} (correspondence party role). Same property name, orthogonal vocab. Scout inventory called this out as the seam-hiding hazard. Renaming **before** any Rust-side closure prevents conflating two different enums in one Rust module.

**Settled name: `correspondenceRole`.** Rejected alternatives: `correspondenceActorType` (keeps the colliding "actor" word); `partyType` (legal-person framing leaks into `system`/`agency` cases that aren't natural persons). `role` already appears as a top-level concept in delivery; this stays consistent.

**Evidence:**

- `schemas/sidecars/wos-delivery.schema.json:604-650` — delivery `actorType` enum.
- `schemas/wos-workflow.schema.json/$defs/Actor/properties/type` — kernel actor type oneOf.
- `crates/wos-core/src/model/kernel.rs:400` — `ActorKind` enum.

**Done when:**

- [ ] Sidecar schema property renamed; description updated; examples updated.
- [ ] Any fixture / Python test / Rust binding referencing old name updated.
- [ ] One sentence in `specs/companions/notification-template.md` (or wherever the delivery sidecar prose ends up landing per **D2** below) noting the convention.
- [ ] Migration note in `COMPLETED.md` flagging the rename as a breaking sidecar property name (no users in production yet, so cost is minimal — see "Nothing is released" memory).

---

### D2 — Delivery sidecar prose home

**Why:** Scout inventory flagged D-down: delivery sidecar schema closes vocabularies (`channel`, `direction`, `actorType`, `correspondenceCategory`, etc.) but no normative prose backs them. `specs/companions/business-calendar.md` and `specs/companions/notification-template.md` cover sub-domains; **no** `specs/companions/delivery.md` (or sidecar-spec equivalent) covers correspondence shape end-to-end.

Decision shape:

- **Option A:** consolidate the two existing sub-domain docs plus a new `correspondence` section into one `specs/sidecars/delivery.md` (matches the merged `wos-delivery.schema.json` envelope per ADR 0076 D-2).
- **Option B:** add a new `specs/companions/correspondence.md` as a peer of the other two.

**Recommend A** (mirror the schema consolidation; one prose home per sidecar schema). Decide and commit.

**Done when:**

- [ ] Single normative prose document covers calendar + notification template + correspondence vocabularies.
- [ ] Each closed-enum schema property cites a §-anchor in the prose document.
- [ ] D-down count drops to 0 in the next scout inventory run.

---

### D3 — `HoldPolicy.holdType` closed-vocab decision

**Why:** D-up: `specs/governance/spec.md` §12 enumerates seven hold reasons inline (`pending-applicant-response`, `pending-third-party`, `pending-legal-review`, `pending-legislation`, `pending-related-case`, `voluntary-hold`, `legal-hold`); schema admits free `string`. Runtime `crates/wos-core/src/instance.rs:428` is `hold_type: String`.

**Already partially recorded** in `schemas/wos-workflow.schema.json:7038-7049` description prose ("Reason for the hold. MUST be one of the seven standard values OR an `x-` prefixed vendor extension. Standard values: …"). The closure shape is `oneOf: [enum: [seven values], pattern: ^x-[a-z][a-z0-9-]*$]` — same shape used for `assuranceLevel` and `Actor.type`.

**Done when:**

- [ ] Schema property promoted from prose-only to `oneOf: [enum, x-pattern]`.
- [ ] Spec §12 cites the schema location.
- [ ] Rust runtime: `HoldType` enum at `crates/wos-core/src/instance.rs` mirroring the seven-value closed set with `Vendor(String)` newtype for `x-*` fallback (validated in constructor).
- [ ] Lint catches free-string `holdType` that is neither standard nor `x-*` shaped.
- [ ] Open-string-leaf ratchet decreases by 1 on workflow schema.

---

### D4 — ADR 0083 open decisions (do NOT renumber here)

The migration ADR's **remaining** open decisions are recorded once, in [`thoughts/adr/0083-wos-instance-migration-runtime-and-http.md`](../adr/0083-wos-instance-migration-runtime-and-http.md). This plan does **not** fork the decision register by re-labeling them. Refer to ADR 0083 §"Open decisions" for the canonical list (**D4** preconditions / posture, **D5** idempotency model, **D6** error-model extensions, **D7** provenance ordering vs facts-tier).

**Gating consequence (revised 2026-05-01):** D1/D2/D3/D3b are **ratified**; **C3**/**C4**/**C5** minimal implementations **have landed** without waiting on D4–D7. Half-landing *would* have been the failure mode if the ADR had stayed at "Proposed" with no implementation snapshot — the ADR now records an **implementation snapshot** and explicitly **trigger-gates** D4–D7 for posture, full idempotency semantics, richer error taxonomy, ordering refinements, and reference-server cross-version prove-out.

**Done when (D4 block):** D4–D7 each either closes in ADR prose or stays explicitly trigger-gated; **WS-042** row closes when version-aware bundle resolution + one HTTP fixture proves a real `definitionVersion` bump (see ADR §Implementation snapshot).

---

### D5 — Vendor `x-*` assurance floor enforcement (fail-closed default)

Already recorded in `T4-TODO.md` ("Vendor `x-*` assurance floor enforcement (deferred-strict-mode)"). No duplicate scope here.

**Gating:** parent PLN-0384 (`wos-event-types.md` ratification) closes the namespace seam.

---

## Chores (independently executable)

### C0 — Commit a durable parity-inventory artifact

**Why:** The 2026-05-01 wos-scout parity inventory exists today only as a transcript and as the working-tree `EXPECTED_OPEN_STRING_LEAVES` ratchet table. Neither carries stable per-leaf row IDs. Without committed row IDs, **C6** ("schema-by-schema by `$defs` cluster") is dispatchable in principle but not in practice — a future agent regenerating the CSV picks up rows in walker order, which can shift between runs. Stable IDs make picked-up-cold execution cheap.

**Artifact:** `wos-spec/thoughts/research/2026-05-01-schema-spec-crate-parity-inventory.md` containing:

- Top-line counts table (per-schema string-leaves / constrained / open) — current snapshot.
- Per-schema CSV (or markdown table) of every open leaf with stable row IDs `WS-001..WS-N` for workflow, `WT-001..` for tooling, `WC-001..` for case-instance, `WP-..` provenance-log, `WD-..` delivery, `WO-..` ontology-alignment, `WL-..` lint-diagnostic, `WM-..` mcp-tools, `WCT-..` conformance-trace.
- Each row: stable ID, JSON pointer, `$defs` context, classification (PARITY / D-down / D-up / D-runtime / HONESTLY-OPEN), proposed closure (or rationale class for honestly-open).
- Per-cluster target: how many leaves close in each `$defs` cluster of workflow when the proposed closures land.
- A regenerate-script-or-recipe block: `cargo run -q --example schema_string_leaf_report -p wos-lint -- <schema> [--csv]` per file, sorted by JSON pointer (already the walker's behavior). Future agents diff against the committed snapshot to detect drift.

**Done when:**

- [ ] Inventory document committed under `wos-spec/thoughts/research/`.
- [ ] Stable row IDs assigned for the current 394 open leaves.
- [ ] C6 references row-ID ranges as work-unit boundaries.
- [ ] Future schema-leaf-report runs can be diffed against the committed snapshot to find new debt.

**Gates:** none. Independent of all other chores; required by C6.

---

### C1 — D-runtime cleanup: typed enums for already-closed schemas

**Why:** Scout's highest-leverage finding. Schema is already closed; runtime spends `String`. Zero spec churn. Template is `CompletionRequirementKind` (`crates/wos-runtime/src/runtime/signature.rs:200-217`).

**Sites (file:line):**

| Schema field | Schema constraint | Crate field | Proposed Rust type |
|---|---|---|---|
| `WorkflowDocument.status` | `enum [draft, active, deprecated]` | `crates/wos-core/src/model/kernel.rs:76` (`status: Option<String>`) | new `PublicationStatus` enum |
| `ActiveTask.impactLevel` | `enum [rights-impacting, safety-impacting, operational, informational]` | `crates/wos-core/src/instance.rs:266` | reuse existing `ImpactLevel` (`kernel.rs:357`) |
| `FactsTierRecord.actorType` | `enum [human, system, agent]` | `crates/wos-core/src/provenance/record.rs:450` | reuse existing `ActorKind` (`kernel.rs:400`) |
| `FactsTierRecord.auditLayer` | `enum [facts, reasoning, counterfactual, narrative]` | `crates/wos-core/src/provenance/record.rs:444` | new `AuditLayer` enum |
| `CaseStateMutation.mutationSource` | `$ref MutationSource` (closed in `$defs`) | `crates/wos-core/src/provenance/record.rs:592` (`Option<&str>` constructor param) | type-narrow constructor to `MutationSource` enum |
| `CaseStateMutation.verificationLevel` | `$ref VerificationLevel` (closed in `$defs`) | `crates/wos-core/src/provenance/record.rs:593` | type-narrow constructor to `VerificationLevel` enum |

**Done when:**

- [ ] Each Rust field is the typed enum / newtype, not `String`.
- [ ] `record.rs:603-608` no longer emits `Value::String(src.to_string())` against a typed source — serde handles the JSON serialization.
- [ ] Existing tests pass; `cargo nextest run --workspace` green.
- [ ] No D-runtime sites for these six fields in next scout pass.

**Commit shape:** one PR per data home (`record.rs` + dependent payload constructors; then `kernel.rs`/`instance.rs`).

---

### C2 — D-up schema closure: signature vocabularies

**Why:** Scout flagged D-up. Spec is normative; schema is open. Aligning schema to spec costs nothing in spec churn.

**Sites:**

| Schema field | Spec citation | Closure shape |
|---|---|---|
| `Signature.documents.documentHashAlgorithm` | `specs/profiles/signature.md:157` (`sha-256` REQUIRED for Core; others MAY appear only via a future profile revision or an `x-*` extension policy) | `oneOf: [{const: "sha-256"}, {pattern: "^x-[a-z][a-z0-9-]*$"}]`. **Do not** preemptively add `sha-384`/`sha-512` to the core enum — that closes the schema tighter than the spec opens (D-down). If those algorithms become normative, ratify a Signature Profile revision first, then extend. |
| `Signature.auditCertificate.signingMode` | `signature.md` `signingFlow.type` mirrors `["sequential", "parallel", "routed", "free-for-all", "witness", "notary"]` | enum mirroring `SigningFlowType` (Rust enum at `signature.rs:170-176`) |
| `Signature.auditCertificate.format` | spec silent | **defer until spec ratification** — open D-up question, NOT a chore |
| `IdentityBindingRequirement.method` / `AuthenticationPolicy.method` | `signature.md:202` requires `in-person` / `notary` / `x-*` for notary roles | `oneOf: [enum: [closed canonical methods], pattern: ^x-…]`; new `IdentityMethod { Login, Credential, InPerson, Notary, X(VendorTag) }` Rust enum |

**Done when:**

- [ ] Schema enums land alongside spec citations.
- [ ] Existing fixtures continue to validate (any using `sha-256` work; any using non-canonical methods will need either ratification or `x-*` migration).
- [ ] Open-string-leaf ratchet decreases by 3 on workflow schema.

**Regression tripwire (already green):** Keep `submit_task_response_sig013_policy_assurance_below_floor_blocks_affirmation` + `signature.rs::assurance_binding_tests` passing when editing affirmation / `identity_binding_meets_policy` or the formspec binding seam ahead of a production `ContractBindingAdapter` for signature completions.

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
- [ ] Conformance fixture(s) under `crates/wos-conformance/tests/fixtures/MIG-*.json` exercise at least one passing migration and one rejected migration (state-not-found).

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

**Why:** ADR 0083 D2 settled the route home (wos-server). **Route + handler landed 2026-05-01** (`crates/wos-server/src/http/instances.rs`); **WS-042** stays **partial** in `crates/wos-server/TODO.md` until cross-version HTTP proof and any checklist items below close.

**Done when:**

- [x] Route handler parses migration body, calls `RuntimeOps::migrate_instance` / runtime migrate path, maps `RuntimeError` → `ApiError` in `crates/wos-server/src/error.rs` (including `MigrationRejected` → **400**, resolver/kernel mismatch → **400**/**404**, `FeatureDisabled` when runtime-local off → **400**).
- [ ] **Idempotency-Key** (or equivalent) honored for duplicate POST semantics — **ADR 0083 D5**; not implemented yet.
- [x] Error mapping covers migration + resolver + feature-off paths; auth via existing `RequireRole<Supervisor>`. **Note:** same-version migrate is a **runtime no-op** (no store write); HTTP returns success with consistent body — not necessarily **409** unless D5 standardizes conflict semantics for duplicates.
- [x] Integration test: `migrate_instance_via_http_same_version_is_idempotent` in `crates/wos-server/tests/integration/runtime_lifecycle.rs` (extend with negative paths + cross-version when bundle resolver supports `(url, version)`).
- [ ] WS-042 marked **complete** in `crates/wos-server/TODO.md` (depends on version-indexed bundle resolution + real bump fixture — see ADR snapshot).

**Gates:** ~~required by C3~~ — **cleared** for the minimal route; remaining items are **WS-042** / ADR D5–D7.

---

### C6 — Long-tail open-leaf hardening (workflow inner-block schema-by-schema)

**Why:** 193 of the 394 open leaves live in `wos-workflow.schema.json`. Most are inner-block (governance / agents / aiOversight / signature / custody / advanced / assurance) leaves where the spec absorption pass (PLN-0176..0207) is still in flight.

**Approach:** schema-by-schema pass, ordered by file size:

1. `wos-workflow.schema.json` — by `$defs` cluster (Actor cluster, Transition cluster, Governance cluster, Agents cluster, Signature cluster, Custody cluster, Advanced cluster).
2. `wos-tooling.schema.json` (54 open).
3. `wos-case-instance.schema.json` (49 open).
4. `wos-delivery.schema.json` (29 open) — coordinate with **D2** (delivery prose home).
5. `wos-conformance/conformance-trace.schema.json` (22 open).
6. `wos-ontology-alignment.schema.json` (20 open).
7. `wos-provenance-log.schema.json` (20 open) — most leaves are `$ref`-shared with workflow $defs; closes naturally as workflow closes.
8. `wos-lint-diagnostic.schema.json` (6 open) — already partially hardened this session.
9. `wos-mcp-tools.schema.json` (1 open) — already partially hardened this session.

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

**Work-unit dispatch:** committed inventory from **C0** assigns stable row IDs (`WS-001..` for workflow, etc.) and per-cluster targets. A PR for C6 names its row-ID range explicitly: e.g., *"close `WS-014..WS-039` (Governance.dueProcessPaths cluster); ratchet `wos-workflow.schema.json: 193 → 167`"*. Without C0's stable IDs, C6 is too wide to dispatch.

**Done when (per PR):**

- [ ] PR names a row-ID range from C0 inventory.
- [ ] `EXPECTED_OPEN_STRING_LEAVES` ratchet baseline decremented by the closed-row count.
- [ ] C0 inventory updated: closed rows marked, classifications stable, no orphan row IDs.
- [ ] No SCHEMA-DOC-001 regressions.
- [ ] No conformance regressions.

**Done when (overall):** Total open-leaf count reaches the **semantic floor** from C0: every surviving “open” count in `EXPECTED_OPEN_STRING_LEAVES` maps to a **HONESTLY-OPEN** row with **`x-wos.openStringKind`** (per **C7**). Under the **current** ratchet definition, that floor is ~50–80; if the project later adopts “C7-tagged leaves are not counted as open,” rerun the inventory and **lower baselines toward 0** in a dedicated ratchet-reset PR.

**Cold-read test:** a future agent reads C0's committed inventory, picks an unclosed row range, executes the closure per the proposed shape in that row, decrements the ratchet, updates C0. No prior conversation context needed.

---

### C7 — `SCHEMA-OPEN-001` lint rule (structured `x-wos.openStringKind` annotation)

**Why:** The remaining floor (~50–80 leaves) is honestly open — free prose, FEL expressions, opaque IDs. They lack `enum`/`const`/`pattern` and that's correct. But "open without justification" is debt; "open with a structured rationale" is honest.

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

- [ ] `SCHEMA-OPEN-001` registered in `crates/wos-lint/src/rules/registry.rs`.
- [ ] Inline tests cover annotation-present (clean), annotation-missing (fail), and annotation-with-unknown-value (fail).
- [ ] Existing schemas pass after C6 lands (the C6 hardening pass adds annotations as it triages).
- [ ] LINT-MATRIX entry registered.
- [ ] `x-wos.openStringKind` documented in `wos-spec/CONVENTIONS.md` alongside the existing `x-lm` annotation conventions.

**Gates:** runs alongside C6; converges as C6 progresses.

---

### C8 — Lint-rule coverage for graph membership beyond initialState

**Why:** K-031 (Transition.actor membership) and K-032 (initialState resolution) are landed. Remaining graph-membership rules JSON Schema cannot express:

- Transition `target` resolves to a sibling state (or to a substate via dotted path).
- Compound state's outbound transitions terminate inside the same parent or escape to a sibling explicitly.
- Parallel-region join targets resolve.

**Done when:**

- [ ] Lint rule(s) registered (likely `K-033`, `K-034`).
- [ ] Spec citations for each rule.
- [ ] Inline tests + LINT-MATRIX entries.

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
- **Long-tail leaf hardening fatigue (C6).** 193 leaves in workflow alone. Mitigation: ratchet enforces forward progress; each PR has a concrete decrement target. Don't try to do all 193 in one pass.

---

## Verification gates (apply per PR)

- `cargo nextest run --workspace` — green.
- `python3 -m pytest tests/ -q` — green.
- `cargo nextest run -p wos-lint --test schema_doc_zero_regression --test open_string_leaf_ratchet` — green.
- `cargo nextest run -p wos-conformance` — green (signature_profile + SIG-013 + any new MIG-* fixtures).
- `cargo test -p wos-runtime submit_task_response_sig013` — green (direct SIG-013 policy-floor path; complements conformance harness).
- `cargo test -p wos-runtime migrate_ --lib` — green after any `migrate` / `MigrationMap` change.
- `cargo test -p wos-server --test integration migrate_instance` — green after HTTP migrate / `RuntimeOps` signature changes.
- `cargo test -p wos-server-runtime-restate --lib` — green when `migrate_instance` contract changes (unsupported path must stay explicit).
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
- 2026-05-01 wos-scout parity inventory (run history; regenerable via `cargo run -q --example schema_string_leaf_report` per schema).
