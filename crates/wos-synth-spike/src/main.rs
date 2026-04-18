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
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| SpikeError::MissingApiKey)?;

    let problem_text = std::fs::read_to_string(&cli.problem)?;

    eprintln!(
        "synthesizing WOS kernel from: {}",
        cli.problem.display()
    );

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
