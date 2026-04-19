// Rust guideline compliant 2026-02-21

//! Emit a rule-coverage report for the unified WOS rule registry.
//!
//! Merges the `wos-lint` (T1/T2) and `wos-conformance` (T3) rule registries,
//! computes graduation-tier and per-category statistics, identifies orphaned
//! fixture files, and surfaces Draft rules ready for promotion.
//!
//! # Usage
//!
//! ```text
//! wos-rule-coverage [OPTIONS]
//!
//! OPTIONS:
//!   --json                   Emit machine-readable JSON instead of plain text
//!   --verbose                Include per-rule detail in text mode
//!   --fixtures-dir <PATH>    Override the fixture tree root (default: auto-detect)
//!   --strict                 Exit 1 if orphaned fixtures or promotion candidates exist
//!   --generate-matrix        Write LINT-MATRIX.md to the workspace root and exit
//!   --matrix-out <PATH>      Override the output path for --generate-matrix
//!   --help, -h               Print this message and exit
//! ```
//!
//! # Exit codes
//!
//! | Code | Meaning |
//! |------|---------|
//! |  0   | Clean (no orphans, no candidates — or `--strict` not set) |
//! |  1   | Orphaned fixtures or promotion candidates exist (`--strict` only) |
//! |  2   | Usage error |

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use wos_conformance::{
    coverage::{compute_coverage, render_json, render_matrix, render_text},
    rules::all_rules,
};
use wos_lint::all_lint_rules;

const EXIT_OK: u8 = 0;
const EXIT_STRICT_VIOLATION: u8 = 1;
const EXIT_USAGE_ERROR: u8 = 2;

fn main() -> ExitCode {
    match run() {
        Ok(exit_code) => ExitCode::from(exit_code),
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(EXIT_USAGE_ERROR)
        }
    }
}

fn run() -> Result<u8, String> {
    let options = CliOptions::parse(std::env::args().skip(1))?;

    // --help prints to stdout and exits 0 (user-requested, not an error).
    if options.help_requested {
        return Ok(EXIT_OK);
    }

    let registries: &[&'static [wos_lint::RuleMetadata]] = &[all_lint_rules(), all_rules()];
    let fixtures_dir = options
        .fixtures_dir
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(default_fixtures_dir);

    let report = compute_coverage(registries, Some(&fixtures_dir));

    // --generate-matrix: write LINT-MATRIX.md and exit.
    if options.generate_matrix {
        let commit_sha = resolve_commit_sha();
        let matrix_content = render_matrix(&report, &commit_sha);
        let out_path = options
            .matrix_out
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(default_matrix_path);
        std::fs::write(&out_path, &matrix_content).map_err(|e| {
            format!("failed to write {}: {e}", out_path.display())
        })?;
        eprintln!(
            "LINT-MATRIX.md written to {} ({} rules)",
            out_path.display(),
            report.summary.total,
        );
        return Ok(EXIT_OK);
    }

    if options.json {
        let json = render_json(&report)
            .map_err(|e| format!("failed to serialize coverage report: {e}"))?;
        println!("{json}");
    } else {
        print!("{}", render_text(&report, options.verbose));
    }

    if options.strict {
        let violations = !report.orphaned_fixtures.is_empty()
            || !report.promotion_candidates.is_empty();
        if violations {
            return Ok(EXIT_STRICT_VIOLATION);
        }
    }

    Ok(EXIT_OK)
}

/// Resolve a commit SHA for the matrix header.
///
/// Priority:
/// 1. `WOS_REGEN_COMMIT` environment variable (for CI).
/// 2. `git rev-parse HEAD` subprocess.
/// 3. Fallback literal `"unknown"`.
fn resolve_commit_sha() -> String {
    if let Ok(sha) = std::env::var("WOS_REGEN_COMMIT") {
        if !sha.trim().is_empty() {
            return sha.trim().to_string();
        }
    }
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output();
    match output {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
        _ => "unknown".to_string(),
    }
}

// ── CLI option parsing ────────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct CliOptions {
    /// Emit JSON output.
    json: bool,
    /// Include per-rule detail in text output.
    verbose: bool,
    /// Override the fixture tree root.
    fixtures_dir: Option<String>,
    /// Exit 1 if orphaned fixtures or promotion candidates exist.
    strict: bool,
    /// Write LINT-MATRIX.md to disk and exit.
    generate_matrix: bool,
    /// Override output path when --generate-matrix is set.
    matrix_out: Option<String>,
    /// Set when --help/-h is seen; causes run() to return Ok(0) immediately.
    help_requested: bool,
}

impl CliOptions {
    fn parse(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut opts = CliOptions::default();
        let mut args = args.peekable();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--json" => opts.json = true,
                "--verbose" | "-v" => opts.verbose = true,
                "--strict" => opts.strict = true,
                "--generate-matrix" => opts.generate_matrix = true,
                "--fixtures-dir" => {
                    opts.fixtures_dir =
                        Some(read_value(&mut args, "--fixtures-dir")?);
                }
                "--matrix-out" => {
                    opts.matrix_out =
                        Some(read_value(&mut args, "--matrix-out")?);
                }
                "--help" | "-h" => {
                    // Print to stdout (not stderr) — help is not an error.
                    println!("{}", usage_message());
                    opts.help_requested = true;
                    return Ok(opts);
                }
                _ if arg.starts_with('-') => {
                    return Err(format!(
                        "unknown option '{}'\n\n{}",
                        arg,
                        usage_message()
                    ));
                }
                _ => {
                    return Err(format!(
                        "unexpected positional argument '{}'\n\n{}",
                        arg,
                        usage_message()
                    ));
                }
            }
        }

        Ok(opts)
    }
}

fn read_value(
    args: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("missing value for {flag}\n\n{}", usage_message()))
}

/// Default fixture tree root: two levels above CARGO_MANIFEST_DIR is the
/// workspace root; the `fixtures/` directory lives there.
fn default_fixtures_dir() -> PathBuf {
    workspace_root().join("fixtures")
}

/// Default output path for `--generate-matrix`: `LINT-MATRIX.md` at the
/// workspace root.
fn default_matrix_path() -> PathBuf {
    workspace_root().join("LINT-MATRIX.md")
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root is two levels above CARGO_MANIFEST_DIR")
        .to_path_buf()
}

fn usage_message() -> String {
    format!(
        "usage: wos-rule-coverage [OPTIONS]\n\
         \n\
         OPTIONS:\n\
           --json                   Emit machine-readable JSON\n\
           --verbose, -v            Include per-rule detail in text mode\n\
           --fixtures-dir <PATH>    Override the fixture tree root\n\
                                    (default: {fixtures_default})\n\
           --strict                 Exit 1 if orphaned fixtures or promotion\n\
                                    candidates exist\n\
           --generate-matrix        Write LINT-MATRIX.md from code registries\n\
           --matrix-out <PATH>      Override LINT-MATRIX.md output path\n\
                                    (default: {matrix_default})\n\
           --help, -h               Print this message and exit\n\
         \n\
         EXIT CODES:\n\
           0  Clean\n\
           1  Orphaned fixtures or promotion candidates found (--strict only)\n\
           2  Usage error",
        fixtures_default = default_fixtures_dir().display(),
        matrix_default = default_matrix_path().display(),
    )
}
