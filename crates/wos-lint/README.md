# wos-lint

Static and selected-project validation for WOS (Workflow Orchestration Standard) documents, using canonical JSON Schema and maintained lint rules.

## Shared static-validation findings

The public `wos_lint::studio_api::validate` operation also emits
`WOS-SCHEMA-001` for canonical schema failures and `WOS-REF-001` for conflicting
or missing explicitly selected project references. These operation findings
retain the existing diagnostic type and use the empty JSON Pointer for a whole
document. They do not add replay rules or change the individual rule registry.
Required check groups report `pass`, `fail` or `not-run`; static success does not
establish runtime readiness. Native Formspec findings retain their owner codes,
JSONPath locations and repair metadata in the dependency report. Behavioral
controls live in `crates/wos-lint/tests/shared_validation.rs` and the actual MCP
adapter tests in `crates/wos-mcp/tests/shared_validation.rs`.

## What it checks

WOS documents are validated against JSON Schema for structural correctness, but many normative constraints require logic beyond what schema validation can express. This crate fills that gap.

**Tier 1 — single-document checks (32 rules).** Examines one WOS document in isolation. No cross-document resolution, no expression parsing.

- Final states with outgoing transitions (K-001)
- Compound states missing `initialState` or `states` (K-002)
- Parallel states missing `regions` (K-003)
- Properties on wrong state types — `cancellationPolicy` on non-parallel, `historyState` on non-compound (K-004, K-005)
- Transition targets referencing nonexistent states (K-006)
- Reserved `$` prefix on user-defined event names (K-007)
- Parallel state transitions not using `$join` (K-008)
- `startTimer` with both `duration` and `deadline`, or neither (K-029)
- Extension keys missing `x-` prefix (K-030)
- Delegation date ordering — expiration before effective, revocation before effective (G-044, G-045)
- Parameter and binding values not in ascending date order (G-047, G-057)
- Binding `id` not matching its map key (G-048)
- Assertion `id` uniqueness (G-037)
- Fallback chain cycle detection and missing terminal action (AI-003, AI-041)
- Disclosure requirement for rights-impacting workflows (AI-046)
- Hold `expectedDuration` format validation (G-055)

**Tier 2 — cross-document + FEL AST checks (51 rules).** Loads a project directory containing kernel, governance, AI integration, and sidecar documents. Resolves cross-references and parses FEL expressions.

Cross-document resolution:

- `targetWorkflow` matches kernel `url` (G-034)
- Governance review protocol tags exist in the kernel (G-011)
- Delegation actors exist in kernel actor declarations (G-046)
- Hold `resumeTrigger` corresponds to a kernel event (G-029)
- Policy parameter `resolutionDateRef` points to a kernel case file field (G-031, G-056)
- Rights-impacting kernel requires `discloseThatAgentAssisted: true` (AI-046)

FEL AST analysis (via `fel-core`):

- Guard expressions parse as valid FEL (K-012, K-013)
- Delegation and assertion conditions parse as valid FEL (G-042, G-043)
- No cross-case variable references in guards (K-017)
- Only built-in and extension functions used (K-019)
- SMT verifiable subset restrictions — linear arithmetic, no recursion, no extension functions, finite quantification (AG-010 through AG-014)

See [`LINT-MATRIX.md`](../../LINT-MATRIX.md) for the complete constraint catalog with rule IDs, spec section references, and tier assignments.

## Usage

```rust
use wos_lint::{lint_document, lint_project};
use std::path::Path;

// Tier 1: lint a single document
let json = std::fs::read_to_string("kernel.json").unwrap();
let diagnostics = lint_document(&json).unwrap();
for d in &diagnostics {
    eprintln!("{d}");
}

// Tier 1 + 2: lint a project directory
let diagnostics = lint_project(Path::new("my-workflow/")).unwrap();
for d in &diagnostics {
    eprintln!("{d}");
}
```

Each diagnostic includes a rule ID (e.g., `K-001`), a JSON Pointer to the offending location, a human-readable message, severity (`LintSeverity`), tier, and optional structured fields (`suggestedFix`, `relatedDocs`, `source`).

## Dependencies

- **`fel-core`** — FEL lexer, parser, and AST for Tier 2 expression analysis. This is the same FEL implementation used by the Formspec engine, ensuring WOS validates against identical FEL semantics.
- **`serde_json`** — JSON parsing and document traversal.
- **`url`** — URI validation for `targetWorkflow` and document references.

## Document detection

Documents are identified by their `$wos*` marker property. Per ADR 0076 the schema family collapsed to six canonical markers — author-time core + two sidecars + two runtime artifacts + one tooling envelope. Legacy per-block markers (`$wosWorkflowGovernance`, `$wosAIIntegration`, `$wosAdvancedGovernance`, `$wosSignatureProfile`, `$wosBusinessCalendar`, `$wosNotificationTemplate`, `$wosCorrespondenceMetadata`, `$wosSemanticProfile`, `$wosAssertionLibrary`, `$wosDueProcess`, `$wosPolicyParameters`, `$wosAgentConfig`, `$wosDriftMonitor`, `$wosEquityConfig`, `$wosVerificationReport`, `$wosExtensionRegistry`, `$wosIntegrationProfile`, `$wosLifecycleDetail`, `$wosKernel`) are retired; their content lives under embedded blocks or `$views` of the merged schemas.

| Marker | Document kind | Schema |
| ------ | ------------- | ------ |
| `$wosWorkflow` | Author-time workflow (kernel + governance + ai + advanced + signature + custody + assurance) | `schemas/wos-workflow.schema.json` |
| `$wosDelivery` | Deployment-environment delivery sidecar (calendar + notifications + correspondence) | `schemas/sidecars/wos-delivery.schema.json` |
| `$wosOntologyAlignment` | Per-deployment ontology alignment sidecar (JSON-LD `@context`, SHACL, PROV-O, XES/OCEL) | `schemas/sidecars/wos-ontology-alignment.schema.json` |
| `$wosProcess` | Runtime workflow-process state | `schemas/wos-process.schema.json` |
| `$wosProvenanceLog` | Runtime append-only audit log | `schemas/wos-provenance-log.schema.json` |
| `$wosTooling` | Tooling envelope wrapping conformance traces, lint diagnostics, MCP catalogs, synth traces, extension registries | `schemas/wos-tooling.schema.json` |

### Structural control flow

Tier 1 runs one typed lifecycle checker through `lint_document`, project lint,
and the shared `validate` operation used by authoring and exact-byte static
prepublication checks. `WOS-FLOW-001` refuses duplicate global state IDs;
`002` reports unreachable states; `003` reports reachable nonfinal configurations
without an exit; `004` reports closed configuration cycles without an outcome.
Locations use authored JSON Pointers; targets retain the runtime's global IDs.

The checker tracks concurrent active states, compound entry and ancestor exits,
and parallel `$join` eligibility for `wait-all`, `cancel-siblings` and `fail-fast`.
Fail-fast accepts normal all-region completion or an error final state, consistent
with Kernel §4.4/§4.8.3. A nested final completes its enclosing scope; it does not
by itself finish the process. Region transitions are conservatively interleaved:
guard correlations and simultaneous event effects may create extra possible
paths. A structural path is a possibility, never an execution or liveness proof.

Kernel's deferred callback rule permits regional business finality while an
independent receipt region remains active. `WOS-FLOW-007` identifies the narrow
supported shape: a top-level `wait-all` parallel, all business regions at direct
regional finals, and each receipt region containing one atomic state whose
transitions self-target and invoke typed inbound callbacks, with no entry/exit
actions or task creation. Names and tags cannot exempt a cycle. This business
outcome does not establish process termination, callback authentication, pending
work preservation, delivery or terminal-cleanup readiness. An authored process
final that cannot be reached still receives `002`, including behind a blocked
join in a persistent workflow. Other persistent shapes require separate
qualification and can receive structural defects until modeled explicitly.

`WOS-FLOW-006` always reports the analysis boundary: guards, ordered choice
shadowing, case data, action effects, external events, fairness and eventual
completion remain unproved. Administration and migration are outside normal
start-to-outcome analysis. History restoration, data-dependent `foreach`, advanced
constraints, and exhausted bounds emit `WOS-FLOW-005` and a required
`workflow-control-flow: not-run` stage in shared validation, suppressing graph
conclusions. The current author-time schema does not declare `historyState`;
typed callers still receive an explicit history limitation, and shared validation
also preserves any schema refusal. Limits are 4,096 states, depth 128, 8,192 active
configurations and 65,536 authored transitions/configuration edges. These are
analysis limits, not normative workflow size limits. A static pass proves neither
runtime completion nor suitability for publication or deployment.
