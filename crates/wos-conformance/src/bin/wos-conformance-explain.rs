// @filedesc Run a conformance fixture and render its trace as human-readable prose or JSON.
//
// Usage:
//   wos-conformance-explain <fixture.json> [--base-dir <dir>] [--json]
//
// Runs <fixture.json> through the conformance runner, builds a ConformanceTrace,
// and either prints human-readable prose (default) or the raw trace JSON (--json).
//
// Exit codes:
//   0  fixture ran and trace rendered (whether the fixture passed or failed)
//   1  fixture produced a failing outcome (prose/JSON is still printed)
//   2  usage error or I/O / parse failure

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use wos_conformance::{render_trace, run_fixture_with_trace, Outcome};

const EXIT_OK: u8 = 0;
const EXIT_FIXTURE_FAILED: u8 = 1;
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

    let (_result, trace) = run_fixture_with_trace(&fixture_json, base_dir).map_err(|error| {
        format!(
            "conformance runner error for '{}': {error}",
            options.fixture_path.display()
        )
    })?;

    if options.emit_json {
        let json = serde_json::to_string_pretty(&trace)
            .map_err(|error| format!("failed to serialize trace: {error}"))?;
        println!("{json}");
    } else {
        print!("{}", render_trace(&trace));
    }

    Ok(if trace.outcome == Outcome::Pass {
        EXIT_OK
    } else {
        EXIT_FIXTURE_FAILED
    })
}

#[derive(Debug)]
struct CliOptions {
    fixture_path: PathBuf,
    base_dir: Option<PathBuf>,
    emit_json: bool,
}

impl CliOptions {
    fn parse(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut args = args.peekable();
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
                    if fixture_path.is_none() {
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

        let Some(fixture_path) = fixture_path else {
            return Err(usage_message());
        };

        Ok(Self {
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
        "usage: wos-conformance-explain <fixture.json> [--base-dir <dir>] [--json]\n\n\
         Options:\n\
           --base-dir <dir>  Resolve relative document paths from <dir>.\n\
                             Default: directory containing <fixture.json>.\n\
           --json            Emit raw ConformanceTrace JSON instead of prose.\n\n\
         Exit codes:\n\
           0  fixture ran; trace rendered (fixture passed)\n\
           1  fixture ran; trace rendered (fixture FAILED)\n\
           2  usage error or I/O failure\n\n\
         Default base dir: {}\n",
        default_base.display()
    )
}
