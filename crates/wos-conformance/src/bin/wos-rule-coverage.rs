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
//!   --json                 Emit machine-readable JSON instead of plain text
//!   --verbose              Include per-rule detail in text mode
//!   --fixtures-dir <PATH>  Override the fixture tree root (default: auto-detect)
//!   --strict               Exit 1 if orphaned fixtures or promotion candidates exist
//!   --help, -h             Print this message and exit
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
    coverage::{compute_coverage, render_json, render_text},
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

    let registries: &[&'static [wos_lint::RuleMetadata]] = &[all_lint_rules(), all_rules()];
    let fixtures_dir = options
        .fixtures_dir
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(default_fixtures_dir);

    let report = compute_coverage(registries, Some(&fixtures_dir));

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
                "--fixtures-dir" => {
                    opts.fixtures_dir =
                        Some(read_value(&mut args, "--fixtures-dir")?);
                }
                "--help" | "-h" => return Err(usage_message()),
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
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root is two levels above CARGO_MANIFEST_DIR")
        .join("fixtures")
}

fn usage_message() -> String {
    format!(
        "usage: wos-rule-coverage [OPTIONS]\n\
         \n\
         OPTIONS:\n\
           --json                 Emit machine-readable JSON\n\
           --verbose, -v          Include per-rule detail in text mode\n\
           --fixtures-dir <PATH>  Override the fixture tree root\n\
                                  (default: {default})\n\
           --strict               Exit 1 if orphaned fixtures or promotion\n\
                                  candidates exist\n\
           --help, -h             Print this message and exit\n\
         \n\
         EXIT CODES:\n\
           0  Clean\n\
           1  Orphaned fixtures or promotion candidates found (--strict only)\n\
           2  Usage error",
        default = default_fixtures_dir().display()
    )
}
