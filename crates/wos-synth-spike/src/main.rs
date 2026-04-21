// Rust guideline compliant 2026-02-21

//! CLI entry point for the WOS synthesis spike.
//!
//! Reads a plain-English problem statement from `--problem`, drives the
//! lint-repair loop via [`loop_mod::synthesize`], and writes the resulting
//! WOS kernel JSON to stdout or `--output`.
//!
//! # Usage
//!
//! ```text
//! export ANTHROPIC_API_KEY=sk-ant-...
//! cargo run -p wos-synth-spike -- \
//!     --problem benchmarks/problems/purchase-order-approval.md \
//!     --output out/purchase-order.json
//! ```
//!
//! If `ANTHROPIC_API_KEY` is not set the binary exits with a clear error
//! message and a non-zero exit code.

mod errors;
mod loop_mod;
mod prompts;

use std::path::PathBuf;

use clap::Parser;
use errors::SpikeError;

/// WOS synthesis spike — generate a valid kernel document from a problem statement.
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the plain-English problem statement (Markdown or plain text).
    #[arg(long)]
    problem: PathBuf,

    /// Optional path to write the resulting JSON document.
    ///
    /// If omitted the document is written to stdout.
    #[arg(long)]
    output: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

/// Drive the full synthesis pipeline and write the result.
///
/// Separated from `main` so that `?` propagation works cleanly and the
/// exit-code logic stays in one place.
///
/// # Errors
///
/// Forwards any [`SpikeError`] produced by the synthesis loop or I/O layer.
async fn run(cli: Cli) -> Result<(), SpikeError> {
    // Validate the API key before doing any I/O.
    let api_key = require_api_key(std::env::var("ANTHROPIC_API_KEY").ok())?;

    let problem_text = std::fs::read_to_string(&cli.problem)?;

    eprintln!("synthesizing WOS kernel from: {}", cli.problem.display());

    let document = loop_mod::synthesize(&problem_text, &api_key).await?;

    let json_output = serde_json::to_string_pretty(&document)
        .expect("serialising a successfully parsed Value must not fail");

    match cli.output {
        Some(ref path) => {
            std::fs::write(path, &json_output)?;
            eprintln!("wrote {} bytes to {}", json_output.len(), path.display());
        }
        None => {
            println!("{json_output}");
        }
    }

    Ok(())
}

/// Validate the `ANTHROPIC_API_KEY` environment variable.
///
/// `std::env::var` returns `Ok("")` for a set-but-empty variable, which would
/// otherwise slip past a naive `ok()` / `map_err` guard and fail later inside
/// the Anthropic SDK with an opaque auth error.  This helper rejects both
/// `None` and any whitespace-only value so failures surface early with a
/// clear message.
///
/// # Errors
///
/// Returns [`SpikeError::MissingApiKey`] when the value is missing, empty,
/// or whitespace-only.
fn require_api_key(raw: Option<String>) -> Result<String, SpikeError> {
    match raw {
        Some(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(SpikeError::MissingApiKey),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_api_key_accepts_real_value() {
        let key =
            require_api_key(Some("sk-ant-abc123".to_string())).expect("non-empty key should pass");
        assert_eq!(key, "sk-ant-abc123");
    }

    #[test]
    fn require_api_key_rejects_none() {
        let err = require_api_key(None).expect_err("None must reject");
        assert!(matches!(err, SpikeError::MissingApiKey));
    }

    #[test]
    fn require_api_key_rejects_empty_string() {
        let err = require_api_key(Some(String::new()))
            .expect_err("empty string must reject — previously slipped through");
        assert!(matches!(err, SpikeError::MissingApiKey));
    }

    #[test]
    fn require_api_key_rejects_whitespace_only() {
        let err =
            require_api_key(Some("   \t\n".to_string())).expect_err("whitespace-only must reject");
        assert!(matches!(err, SpikeError::MissingApiKey));
    }
}
