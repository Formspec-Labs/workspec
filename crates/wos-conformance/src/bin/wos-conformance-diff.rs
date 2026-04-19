// @filedesc Compare an expected ConformanceTrace against a freshly-run fixture and report divergence.
//
// Usage:
//   wos-conformance-diff <expected-trace.json> <fixture.json> [--base-dir <dir>] [--json]
//
// Loads <expected-trace.json> as the baseline ConformanceTrace, runs <fixture.json>
// through the conformance runner to produce a fresh trace, then diffs the two.
//
// Output:
//   "OK" (exit 0) when the traces match.
//   A structured divergence report (prose or JSON) plus exit 1 when they differ.
//
// Exit codes:
//   0  traces match
//   1  traces diverge (report printed to stdout)
//   2  usage error, I/O failure, or parse error

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use wos_conformance::{diff_traces, render_diff, run_fixture_with_trace, ConformanceTrace, TraceDiffResult};

const EXIT_MATCH: u8 = 0;
const EXIT_DIVERGE: u8 = 1;
const EXIT_USAGE_ERROR: u8 = 2;

fn main() -> ExitCode {
    match run() {
        Ok(exit_code) => ExitCode::from(exit_code),
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(EXIT_USAGE_ERROR)
        }
    }
}

fn run() -> Result<u8, String> {
    let options = CliOptions::parse(std::env::args().skip(1))?;

    // Load the committed expected trace.
    let expected_json =
        std::fs::read_to_string(&options.expected_trace_path).map_err(|error| {
            format!(
                "failed to read expected trace '{}': {error}",
                options.expected_trace_path.display()
            )
        })?;
    let expected: ConformanceTrace =
        serde_json::from_str(&expected_json).map_err(|error| {
            format!(
                "failed to parse expected trace '{}': {error}",
                options.expected_trace_path.display()
            )
        })?;

    // Run the fixture to produce a fresh trace.
    let fixture_json =
        std::fs::read_to_string(&options.fixture_path).map_err(|error| {
            format!(
                "failed to read fixture '{}': {error}",
                options.fixture_path.display()
            )
        })?;

    let base_dir = options
        .base_dir
        .as_deref()
        .map(|p| p.to_str().unwrap_or("."))
        .unwrap_or_else(|| {
            options
                .fixture_path
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or(".")
        });

    let (_result, actual) =
        run_fixture_with_trace(&fixture_json, base_dir).map_err(|error| {
            format!(
                "conformance runner error for '{}': {error}",
                options.fixture_path.display()
            )
        })?;

    // Compare the two traces.
    let diff_result = diff_traces(&expected, &actual);

    if options.emit_json {
        let json = serde_json::to_string_pretty(&diff_result)
            .map_err(|error| format!("failed to serialize diff result: {error}"))?;
        println!("{json}");
    } else {
        print!("{}", render_diff(&diff_result));
    }

    Ok(match diff_result {
        TraceDiffResult::Match => EXIT_MATCH,
        TraceDiffResult::Divergence(_) => EXIT_DIVERGE,
    })
}

#[derive(Debug)]
struct CliOptions {
    expected_trace_path: PathBuf,
    fixture_path: PathBuf,
    base_dir: Option<PathBuf>,
    emit_json: bool,
}

impl CliOptions {
    fn parse(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut args = args.peekable();
        let mut expected_trace_path: Option<PathBuf> = None;
        let mut fixture_path: Option<PathBuf> = None;
        let mut base_dir: Option<PathBuf> = None;
        let mut emit_json = false;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--base-dir" => {
                    base_dir = Some(PathBuf::from(next_value(&mut args, "--base-dir")?));
                }
                "--json" => {
                    emit_json = true;
                }
                "--help" | "-h" => return Err(usage_message()),
                _ if arg.starts_with('-') => {
                    return Err(format!("unknown argument '{arg}'\n\n{}", usage_message()));
                }
                _ => {
                    if expected_trace_path.is_none() {
                        expected_trace_path = Some(PathBuf::from(arg));
                    } else if fixture_path.is_none() {
                        fixture_path = Some(PathBuf::from(arg));
                    } else {
                        return Err(format!(
                            "unexpected positional argument '{arg}'\n\n{}",
                            usage_message()
                        ));
                    }
                }
            }
        }

        let (Some(expected_trace_path), Some(fixture_path)) =
            (expected_trace_path, fixture_path)
        else {
            return Err(usage_message());
        };

        Ok(Self {
            expected_trace_path,
            fixture_path,
            base_dir,
            emit_json,
        })
    }
}

fn next_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("missing value for {flag}\n\n{}", usage_message()))
}

fn usage_message() -> String {
    let default_base = Path::new("<fixture-directory>");
    format!(
        "usage: wos-conformance-diff <expected-trace.json> <fixture.json> [--base-dir <dir>] [--json]\n\n\
         Options:\n\
           --base-dir <dir>  Resolve relative document paths from <dir>.\n\
                             Default: directory containing <fixture.json>.\n\
           --json            Emit the diff result as JSON instead of prose.\n\n\
         Exit codes:\n\
           0  traces match\n\
           1  traces diverge (report printed)\n\
           2  usage error or I/O failure\n\n\
         Default base dir: {}\n",
        default_base.display()
    )
}
