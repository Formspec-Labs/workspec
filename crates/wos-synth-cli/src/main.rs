//! `wos-synth` binary — generate WOS documents from plain-English problems.
//!
//! Three subcommands today:
//!
//! - `generate` — run the synthesis loop against the Anthropic API.
//! - `dry-run` — run the loop with a mock prompter (no network); useful for
//!   smoke-testing the wiring in CI without an API key.
//! - `explain` — render a saved synth-trace JSON as a human-readable transcript.
//!
//! `ANTHROPIC_API_KEY` must be set for `generate`; `dry-run` ignores it.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use wos_synth_core::{DirectToolContext, Layer, Prompter, SynthOutcome, SynthTrace, synthesize};

#[derive(Parser, Debug)]
#[command(name = "wos-synth", version, about = "WOS LLM synthesis loop")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Generate a WOS document from a plain-English problem statement.
    Generate {
        /// Path to the problem statement (markdown or plain text).
        #[arg(long)]
        problem: PathBuf,
        /// Which WOS layer to target.
        #[arg(long, value_enum, default_value_t = LayerArg::Kernel)]
        layer: LayerArg,
        /// Output path for the synthesized JSON document.
        #[arg(long)]
        output: PathBuf,
        /// Optional path to write the synth trace JSON.
        #[arg(long)]
        trace: Option<PathBuf>,
        /// Maximum loop iterations including the initial generation.
        #[arg(long, default_value_t = 5)]
        max_iterations: u32,
    },
    /// Run the loop with a deterministic mock prompter (no network).
    DryRun {
        #[arg(long)]
        problem: PathBuf,
        #[arg(long, value_enum, default_value_t = LayerArg::Kernel)]
        layer: LayerArg,
        #[arg(long)]
        output: PathBuf,
    },
    /// Render a saved trace as a human-readable transcript.
    Explain {
        /// Path to a previously written trace JSON.
        trace: PathBuf,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum LayerArg {
    Kernel,
    Governance,
    Ai,
    Advanced,
}

impl From<LayerArg> for Layer {
    fn from(arg: LayerArg) -> Self {
        match arg {
            LayerArg::Kernel => Layer::Kernel,
            LayerArg::Governance => Layer::Governance,
            LayerArg::Ai => Layer::Ai,
            LayerArg::Advanced => Layer::Advanced,
        }
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli).await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

async fn run(cli: Cli) -> anyhow_lite::Result<ExitCode> {
    match cli.command {
        Command::Generate {
            problem,
            layer,
            output,
            trace,
            max_iterations,
        } => {
            let api_key = require_env("ANTHROPIC_API_KEY")?;
            let provider = wos_synth_anthropic::AnthropicPrompter::new(api_key);
            run_loop(
                &provider,
                &problem,
                layer.into(),
                &output,
                trace.as_deref(),
                max_iterations,
            )
            .await
        }
        Command::DryRun {
            problem,
            layer,
            output,
        } => {
            let mock = wos_synth_mock::MockPrompter::new();
            // Canned: first call returns a minimal valid kernel doc.
            mock.expect(
                "Problem statement",
                r#"{"$wosKernel":"1.0","title":"dry-run","status":"draft","impactLevel":"informational","caseFile":{"fields":{}},"actors":[{"id":"a","type":"system"}],"lifecycle":{"initialState":"start","states":{"start":{"type":"final"}}}}"#,
            );
            run_loop(&mock, &problem, layer.into(), &output, None, 1).await
        }
        Command::Explain { trace } => {
            let raw = std::fs::read_to_string(&trace)
                .map_err(|e| anyhow_lite::err(format!("reading {}: {e}", trace.display())))?;
            let parsed: SynthTrace = serde_json::from_str(&raw)
                .map_err(|e| anyhow_lite::err(format!("parsing trace: {e}")))?;
            print_trace(&parsed);
            Ok(ExitCode::SUCCESS)
        }
    }
}

async fn run_loop(
    provider: &dyn Prompter,
    problem_path: &PathBuf,
    layer: Layer,
    output: &PathBuf,
    trace_path: Option<&std::path::Path>,
    max_iterations: u32,
) -> anyhow_lite::Result<ExitCode> {
    let problem = std::fs::read_to_string(problem_path)
        .map_err(|e| anyhow_lite::err(format!("reading {}: {e}", problem_path.display())))?;
    let tools = DirectToolContext::new();

    eprintln!(
        "synthesizing {layer:?} document from {}",
        problem_path.display()
    );

    let outcome = synthesize(provider, &tools, &problem, layer, max_iterations)
        .await
        .map_err(|e| anyhow_lite::err(format!("loop error: {e}")))?;

    match outcome {
        SynthOutcome::Converged { document, trace } => {
            std::fs::write(output, &document)
                .map_err(|e| anyhow_lite::err(format!("writing {}: {e}", output.display())))?;
            if let Some(path) = trace_path {
                let trace_json =
                    serde_json::to_string_pretty(&trace).expect("trace serialisation cannot fail");
                std::fs::write(path, trace_json).map_err(|e| {
                    anyhow_lite::err(format!("writing trace {}: {e}", path.display()))
                })?;
            }
            eprintln!(
                "converged in {} iteration(s); wrote {}",
                trace.iterations.len(),
                output.display()
            );
            Ok(ExitCode::SUCCESS)
        }
        SynthOutcome::Unconverged {
            last_attempt,
            last_findings,
            trace,
        } => {
            std::fs::write(output, &last_attempt)
                .map_err(|e| anyhow_lite::err(format!("writing {}: {e}", output.display())))?;
            if let Some(path) = trace_path {
                let trace_json =
                    serde_json::to_string_pretty(&trace).expect("trace serialisation cannot fail");
                std::fs::write(path, trace_json).map_err(|e| {
                    anyhow_lite::err(format!("writing trace {}: {e}", path.display()))
                })?;
            }
            eprintln!(
                "DID NOT CONVERGE after {} iteration(s); {} finding(s) remain",
                trace.iterations.len(),
                last_findings.len(),
            );
            for f in &last_findings {
                eprintln!(
                    "  [{rule}] {sev:?}: {msg}",
                    rule = f.rule_id,
                    sev = f.severity,
                    msg = f.message
                );
            }
            Ok(ExitCode::from(2))
        }
    }
}

fn print_trace(trace: &SynthTrace) {
    print!("{}", render_trace(trace));
}

fn render_trace(trace: &SynthTrace) -> String {
    let mut out = String::new();
    use std::fmt::Write as _;

    writeln!(
        out,
        "synth-trace: {} iteration(s); tokens in/out/cache = {}",
        trace.iterations.len(),
        format_token_totals(trace),
    )
    .expect("writing to string cannot fail");
    for iter in &trace.iterations {
        writeln!(
            out,
            "  iter {idx}: {n} finding(s); conformance = {conformance}; tokens in/out/cache = {tokens}",
            idx = iter.index,
            n = iter.lint_findings.len(),
            conformance = format_conformance(iter.conformance.as_ref()),
            tokens = format_iteration_tokens(iter.input_tokens, iter.output_tokens, iter.cache_read_tokens),
        )
        .expect("writing to string cannot fail");
        for f in &iter.lint_findings {
            writeln!(
                out,
                "    [{rule}] {sev:?}: {msg}",
                rule = f.rule_id,
                sev = f.severity,
                msg = f.message
            )
            .expect("writing to string cannot fail");
        }
    }
    out
}

fn format_conformance(conformance: Option<&wos_synth_core::ConformanceVerdict>) -> String {
    match conformance {
        Some(verdict) if verdict.passed => format!("pass: {}", verdict.summary),
        Some(verdict) => format!("fail: {}", verdict.summary),
        None => "not run".to_string(),
    }
}

/// Render token totals for the trace summary line.
///
/// When at least one iteration ran but every total is zero, the underlying
/// provider almost certainly does not surface usage metadata (today, the
/// Anthropic streaming-callback SDK does not). Print "unknown" so the user
/// does not assume the binary is broken.
fn format_token_totals(trace: &SynthTrace) -> String {
    let totals_zero = trace.total_input_tokens == 0
        && trace.total_output_tokens == 0
        && trace.total_cache_read_tokens == 0;
    if totals_zero && !trace.iterations.is_empty() {
        return "unknown (provider does not report)".into();
    }
    format!(
        "{}/{}/{}",
        trace.total_input_tokens, trace.total_output_tokens, trace.total_cache_read_tokens
    )
}

fn format_iteration_tokens(input: u64, output: u64, cache: u64) -> String {
    if input == 0 && output == 0 && cache == 0 {
        return "unknown".into();
    }
    format!("{input}/{output}/{cache}")
}

fn require_env(name: &str) -> anyhow_lite::Result<String> {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(anyhow_lite::err(format!(
            "{name} is not set or is empty; export it before running"
        ))),
    }
}

/// Tiny `anyhow`-shaped error used by `main`.
///
/// Intentionally not `anyhow` itself: the binary's error surface is six
/// `format!` calls; pulling a 12K-LOC dependency for that is not free in
/// build time, and the binary's reach (CLI distribution to government
/// users) makes dependency footprint visible. Six call sites do not earn a
/// dependency. If the surface ever crosses ~30 sites or starts needing
/// chained context, swap to `anyhow` — by then the cost-benefit flips.
mod anyhow_lite {
    use std::fmt;

    #[derive(Debug)]
    pub struct Error(String);

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&self.0)
        }
    }

    impl std::error::Error for Error {}

    pub type Result<T> = std::result::Result<T, Error>;

    pub fn err(message: impl Into<String>) -> Error {
        Error(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wos_synth_core::{ConformanceVerdict, IterationRecord, LintFinding, Severity, SynthTrace};

    #[test]
    fn render_trace_includes_iteration_conformance() {
        let mut trace = SynthTrace::new();
        trace.push(IterationRecord {
            index: 0,
            attempt: r#"{"$wosKernel":"1.0"}"#.to_string(),
            lint_findings: vec![LintFinding {
                rule_id: "K-001".to_string(),
                severity: Severity::Error,
                message: "missing terminal state".to_string(),
                path: None,
                suggested_fix: None,
                related_docs: vec![],
            }],
            conformance: Some(ConformanceVerdict {
                passed: false,
                summary: "step 1 diverged".to_string(),
            }),
            input_tokens: 1,
            output_tokens: 2,
            cache_read_tokens: 3,
        });
        trace.push(IterationRecord {
            index: 1,
            attempt: r#"{"$wosKernel":"1.0"}"#.to_string(),
            lint_findings: vec![],
            conformance: None,
            input_tokens: 0,
            output_tokens: 0,
            cache_read_tokens: 0,
        });

        let rendered = render_trace(&trace);

        assert!(rendered.contains("conformance = fail: step 1 diverged"));
        assert!(rendered.contains("conformance = not run"));
        assert!(rendered.contains("[K-001] Error: missing terminal state"));
    }
}
