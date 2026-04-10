# wos-lint

Static linter for WOS (Workflow Orchestration Standard) documents. Checks 83 normative constraints that JSON Schema cannot enforce.

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

Each diagnostic includes a rule ID (e.g., `K-001`), a JSON pointer path to the offending location, a human-readable message, and a severity (error, warning, info).

## Dependencies

- **`fel-core`** — FEL lexer, parser, and AST for Tier 2 expression analysis. This is the same FEL implementation used by the Formspec engine, ensuring WOS validates against identical FEL semantics.
- **`serde_json`** — JSON parsing and document traversal.
- **`url`** — URI validation for `targetWorkflow` and document references.

## Document detection

Documents are identified by their `$wos*` marker property:

| Marker | Document kind |
| ------ | ------------- |
| `$wosKernel` | Kernel |
| `$wosWorkflowGovernance` | Workflow Governance |
| `$wosDueProcess` | Due Process Config |
| `$wosAssertionLibrary` | Assertion Gate Library |
| `$wosPolicyParameters` | Policy Parameter Config |
| `$wosAIIntegration` | AI Integration |
| `$wosAgentConfig` | Agent Config |
| `$wosDriftMonitor` | Drift Monitor |
| `$wosAdvancedGovernance` | Advanced Governance |
| `$wosEquityConfig` | Equity Config |
| `$wosVerificationReport` | Verification Report |
| `$wosIntegrationProfile` | Integration Profile |
| `$wosSemanticProfile` | Semantic Profile |
| `$wosLifecycleDetail` | Lifecycle Detail |
| `$wosCorrespondenceMetadata` | Correspondence Metadata |
