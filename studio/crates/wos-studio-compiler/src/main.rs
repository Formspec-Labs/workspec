// Rust guideline compliant 2026-05-02

//! `wos-studio-compile` — CLI front-end for the Studio compiler.
//!
//! Usage:
//!
//! ```text
//! wos-studio-compile <workspace-dir> [--out <dir>]
//!                                    [--no-gates]
//!                                    [--dry-run]
//! ```
//!
//! Stdout: JSON compile result (success: emitted file paths; failure:
//! structured diagnostic stream).

use std::path::PathBuf;
use std::process::ExitCode;

use wos_studio_compiler::{CompileError, CompileOptions, compile_workspace};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("compile failed: {err}");
            if let CompileError::Halt { details, .. } = &err {
                for d in details {
                    eprintln!("  - {d}");
                }
            }
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), CompileError> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        eprintln!(
            "usage: wos-studio-compile <workspace-dir> [--out <dir>] [--no-gates] [--dry-run]"
        );
        std::process::exit(2);
    }

    let mut workspace_dir: Option<PathBuf> = None;
    let mut out_dir: Option<PathBuf> = None;
    let mut options = CompileOptions::default();
    let mut dry_run = false;

    let mut i = 0;
    while i < args.len() {
        let arg = std::mem::take(&mut args[i]);
        match arg.as_str() {
            "--out" => {
                i += 1;
                out_dir = args.get(i).map(PathBuf::from);
            }
            "--no-gates" => {
                options.run_external_gates = false;
            }
            "--dry-run" => {
                dry_run = true;
                options.halt_on_readiness_error = false;
                options.run_external_gates = false;
            }
            "--help" | "-h" => {
                println!(
                    "wos-studio-compile <workspace-dir> [--out <dir>] [--no-gates] [--dry-run]"
                );
                std::process::exit(0);
            }
            other if workspace_dir.is_none() => {
                workspace_dir = Some(PathBuf::from(other));
            }
            other => {
                eprintln!("unknown arg: {other}");
                std::process::exit(2);
            }
        }
        i += 1;
    }

    let dir = workspace_dir.expect("workspace-dir required");
    let artifact = compile_workspace(&dir, options)?;

    let summary = serde_json::json!({
        "outcome": if dry_run { "dry-run" } else { "compiled" },
        "manifest": &artifact.manifest,
        "embeddedBlocks": &artifact.manifest.embedded_blocks_emitted,
        "scenarios": artifact.scenarios.len(),
        "readinessFindings": artifact.readiness_findings.len(),
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&summary).unwrap_or_else(|_| String::new())
    );

    if let Some(out) = out_dir {
        std::fs::create_dir_all(&out)?;
        std::fs::write(
            out.join("wos-workflow.json"),
            serde_json::to_string_pretty(&artifact.wos_workflow)
                .map_err(CompileError::Json)?,
        )?;
        std::fs::write(
            out.join("compile-manifest.json"),
            serde_json::to_string_pretty(&artifact.manifest)
                .map_err(CompileError::Json)?,
        )?;
        std::fs::write(
            out.join("scenarios.json"),
            serde_json::to_string_pretty(&artifact.scenarios)
                .map_err(CompileError::Json)?,
        )?;
        if let Some(notes) = &artifact.release_notes {
            std::fs::write(out.join("release-notes.md"), notes)?;
        }
        // SA-MUST-cmp-070..073: write the compiler event stream as
        // JSON-Lines beside the artifacts so observability tooling
        // can pick it up without re-parsing the manifest.
        std::fs::write(
            out.join("compile-events.jsonl"),
            artifact.events.to_jsonl(),
        )?;
        // SA-MUST-cmp-060..063: workspace export bundle —
        // self-contained, deterministic, reproducible from manifest
        // alone.
        std::fs::write(
            out.join("workspace-export.bundle.json"),
            serde_json::to_string_pretty(&artifact.export_bundle)
                .map_err(CompileError::Json)?,
        )?;
    }
    Ok(())
}
