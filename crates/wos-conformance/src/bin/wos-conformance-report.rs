// Rust guideline compliant 2026-02-21

//! Emit a processor-level conformance report.
//!
//! Reads a `ProcessorManifest`, verifies it against the bundled fixture
//! inventory or a caller-supplied fixtures directory, prints the resulting
//! `ProcessorConformanceReport`, and returns a failing exit code if any
//! claimed meta-rule does not verify.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use wos_conformance::{ClaimStatus, ProcessorManifest, verify_processor_manifest};

const DEFAULT_FORMAT: &str = "json";
const EXIT_OK: u8 = 0;
const EXIT_FAILED_CLAIM: u8 = 1;
const EXIT_USAGE_ERROR: u8 = 2;

fn main() -> ExitCode {
    match run() {
        Ok(exit_code) => ExitCode::from(exit_code),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(EXIT_USAGE_ERROR)
        }
    }
}

fn run() -> Result<u8, String> {
    let options = CliOptions::parse(std::env::args().skip(1))?;
    let manifest_json = std::fs::read_to_string(&options.manifest_path).map_err(|error| {
        format!(
            "failed to read manifest '{}': {error}",
            options.manifest_path.display()
        )
    })?;
    let manifest: ProcessorManifest = serde_json::from_str(&manifest_json).map_err(|error| {
        format!(
            "failed to parse manifest '{}': {error}",
            options.manifest_path.display()
        )
    })?;
    let report = verify_processor_manifest(&manifest, &options.fixtures_dir).map_err(|error| {
        format!(
            "failed to verify manifest against fixtures '{}': {error}",
            options.fixtures_dir.display()
        )
    })?;

    match options.format {
        ReportFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .map_err(|error| format!("failed to serialize report: {error}"))?
            );
        }
        ReportFormat::Text => {
            print!("{}", render_text_report(&report));
        }
    }

    Ok(
        if report
            .claims
            .iter()
            .any(|claim| claim.status == ClaimStatus::Failed)
        {
            EXIT_FAILED_CLAIM
        } else {
            EXIT_OK
        },
    )
}

#[derive(Debug)]
struct CliOptions {
    manifest_path: PathBuf,
    fixtures_dir: PathBuf,
    format: ReportFormat,
}

impl CliOptions {
    fn parse(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut args = args.peekable();
        let mut manifest_path: Option<PathBuf> = None;
        let mut fixtures_dir = default_fixtures_dir();
        let mut format = ReportFormat::Json;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--manifest" => {
                    manifest_path = Some(PathBuf::from(read_value(&mut args, "--manifest")?));
                }
                "--fixtures-dir" => {
                    fixtures_dir = PathBuf::from(read_value(&mut args, "--fixtures-dir")?);
                }
                "--format" => {
                    format = ReportFormat::parse(&read_value(&mut args, "--format")?)?;
                }
                "--help" | "-h" => return Err(usage_message()),
                _ if arg.starts_with('-') => {
                    return Err(format!("unknown argument '{arg}'\n\n{}", usage_message()));
                }
                _ => {
                    if manifest_path.is_none() {
                        manifest_path = Some(PathBuf::from(arg));
                    } else {
                        return Err(format!(
                            "unexpected positional argument '{arg}'\n\n{}",
                            usage_message()
                        ));
                    }
                }
            }
        }

        let Some(manifest_path) = manifest_path else {
            return Err(usage_message());
        };

        Ok(Self {
            manifest_path,
            fixtures_dir,
            format,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum ReportFormat {
    Json,
    Text,
}

impl ReportFormat {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "json" => Ok(Self::Json),
            "text" => Ok(Self::Text),
            _ => Err(format!(
                "unsupported format '{value}' (expected 'json' or 'text')"
            )),
        }
    }
}

fn render_text_report(report: &wos_conformance::ProcessorConformanceReport) -> String {
    let mut output = String::new();
    output.push_str(&format!("processor: {}\n", report.processor_name));

    let verified = report
        .claims
        .iter()
        .filter(|claim| claim.status == ClaimStatus::Verified)
        .count();
    let failed = report
        .claims
        .iter()
        .filter(|claim| claim.status == ClaimStatus::Failed)
        .count();
    let not_claimed = report
        .claims
        .iter()
        .filter(|claim| claim.status == ClaimStatus::NotClaimed)
        .count();

    output.push_str(&format!(
        "summary: {} verified, {} failed, {} not claimed\n",
        verified, failed, not_claimed
    ));

    for claim in &report.claims {
        output.push_str(&format!(
            "- {}: {} ({})\n",
            claim.rule_id,
            status_label(&claim.status),
            claim.message
        ));
    }

    output
}

fn status_label(status: &ClaimStatus) -> &'static str {
    match status {
        ClaimStatus::NotClaimed => "not-claimed",
        ClaimStatus::Verified => "verified",
        ClaimStatus::Failed => "failed",
    }
}

fn read_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("missing value for {flag}\n\n{}", usage_message()))
}

fn default_fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn usage_message() -> String {
    format!(
        "usage: wos-conformance-report --manifest <path> [--fixtures-dir <path>] [--format <json|text>]\n\
         default fixtures dir: {}\n\
         default format: {}",
        default_fixtures_dir().display(),
        DEFAULT_FORMAT
    )
}
