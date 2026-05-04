// Rust guideline compliant 2026-05-02

//! `wos-studio-simulate` — CLI for the Studio scenario simulator.
//!
//! Usage:
//!
//! ```text
//! wos-studio-simulate <workspace-dir> [<scenario-id>]
//! ```
//!
//! Compiles the workspace, runs every Scenario (or just one if a
//! scenario id is given), and prints per-scenario pass/fail + delta to
//! stdout. Exit code:
//! - 0 — all scenarios passed (or filter matched a passing scenario).
//! - 1 — at least one scenario failed.
//! - 2 — usage / argument error.
//! - 64 — workspace load / compile error (mirrors `EX_USAGE`-style
//!   semantics; distinguishes from scenario failures).

use std::path::PathBuf;
use std::process::ExitCode;

use wos_studio_compiler::load_workspace_from_dir;
use wos_studio_scenario::{ScenarioOutcome, run_workspace};

const USAGE: &str = "usage: wos-studio-simulate <workspace-dir> [<scenario-id>]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    if args.len() > 2 {
        eprintln!("{USAGE}\nerror: at most 2 positional arguments accepted; got {}", args.len());
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[0]);
    let scenario_filter = args.get(1).cloned();

    let ws = match load_workspace_from_dir(&dir) {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("failed to load workspace: {e}");
            return ExitCode::from(64);
        }
    };

    let results = match run_workspace(&ws) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("compile failed before simulation: {e}");
            return ExitCode::from(64);
        }
    };

    let total_pre_filter = results.len();
    let mut filtered: Vec<_> = results
        .into_iter()
        .filter(|r| match &scenario_filter {
            Some(id) => &r.scenario_id == id,
            None => true,
        })
        .collect();

    // Filter-not-found feedback: when the user asked for a specific
    // scenario id and the post-filter list is empty, exit non-zero
    // with a clear "unknown id" message (R5.11).
    if let Some(id) = &scenario_filter {
        if filtered.is_empty() && total_pre_filter > 0 {
            eprintln!("unknown scenario id: {id}");
            return ExitCode::from(2);
        }
    }

    let summary = serde_json::json!({
        "total": filtered.len(),
        "pass": filtered.iter().filter(|r| matches!(r.outcome, ScenarioOutcome::Pass)).count(),
        "fail": filtered.iter().filter(|r| matches!(r.outcome, ScenarioOutcome::Fail)).count(),
        "inconclusive": filtered.iter().filter(|r| matches!(r.outcome, ScenarioOutcome::Inconclusive)).count(),
        "results": &filtered,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&summary).unwrap_or_else(|_| String::new())
    );

    let any_fail = filtered
        .iter_mut()
        .any(|r| matches!(r.outcome, ScenarioOutcome::Fail));
    if any_fail {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
