//! Executable entry point for the shared synthesis host.

use clap::Parser;
use std::process::ExitCode;
use wos_synth_cli::{Cli, run};

#[tokio::main]
async fn main() -> ExitCode {
    match run(Cli::parse()).await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}
