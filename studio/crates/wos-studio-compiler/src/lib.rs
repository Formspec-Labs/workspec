// Rust guideline compliant 2026-05-02

//! Stage-5 Studio → WOS compiler.
//!
//! Eight ordered phases (per `studio/specs/compiler-contract.md`):
//!
//! 1. Select approved policy objects → assemble compile-input
//! 2. Resolve mapping records → bind every PolicyObject to a target
//! 3. Generate workflow intent → walk WorkflowIntent's elements
//! 4. Emit lifecycle / governance / AI / advanced / assurance blocks
//! 5. Emit scenario artifacts → project into `wos-tooling.scenarios[*]`
//! 6. Run Studio readiness checks → tier S1–S6 evaluation
//! 7. Run WOS schema/lint/conformance → three external gates
//! 8. Produce review package → ApprovalPackage + manifest + release notes
//!
//! Determinism is **load-bearing** (`SA-MUST-cmp-001`): identical inputs
//! produce byte-identical output (modulo JSON key ordering). Phases use
//! [`indexmap::IndexMap`] for stable iteration order; sorted vectors
//! everywhere user content lands.

pub mod artifact;
pub mod error;
pub mod events;
pub mod gates;
pub mod manifest;
pub mod phase1_load;
pub mod phase2_mapping;
pub mod phase3_workflow;
pub mod phase4_emit;
pub mod phase5_scenarios;
pub mod phase6_readiness;
pub mod phase7_gates;
pub mod phase8_package;
pub mod phase9_export;
pub mod pipeline;
pub mod schema_validator;

pub use artifact::{ApprovalPackage, CompileArtifact, EmittedScenario};
pub use error::{CompileError, Disposition, FailureKind};
pub use events::{CompilerEvent, EventBuffer, EventSink};
pub use gates::{ExternalGate, GateOutcome, GateResult};
pub use manifest::CompileManifest;
pub use pipeline::{CompileOptions, compile, compile_workspace};

use wos_studio_lint::Workspace;

/// Compiler version string. Embedded into every `CompileManifest` so
/// downstream verifiers can confirm reproduction by recorded compiler.
pub const COMPILER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Schema version identifier sourced at build time from
/// `schemas/wos-workflow.schema.json` via the build.rs FNV-1a content
/// hash. Closes `SA-MUST-cmp-050` ("the actual version of
/// wos-workflow.schema.json consumed"). Falls back to
/// `"schema-unknown"` if the schema file isn't reachable from the
/// build environment.
pub const SCHEMA_VERSION: &str = env!("WOS_SCHEMA_VERSION");

/// Construct a [`Workspace`] from a directory of Studio JSON files. The
/// loader walks the directory tree, parses each `.json` file, and adds
/// it to the workspace. Files that fail to parse or carry no
/// `$wosStudio*` marker are silently skipped (auxiliary artifacts).
pub fn load_workspace_from_dir(
    dir: &std::path::Path,
) -> std::io::Result<Workspace> {
    let mut entries: Vec<(String, String)> = Vec::new();
    walk_json(dir, &mut entries)?;
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(Workspace::from_iter(entries))
}

fn walk_json(
    dir: &std::path::Path,
    out: &mut Vec<(String, String)>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_json(&path, out)?;
        } else if path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s == "json")
        {
            let content = std::fs::read_to_string(&path)?;
            out.push((path.to_string_lossy().to_string(), content));
        }
    }
    Ok(())
}
