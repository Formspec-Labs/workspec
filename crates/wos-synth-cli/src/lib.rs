//! Synthesis host configuration and execution using complete native tool evidence.
//!
//! Providers and concrete tools meet here. The core loop depends on neither.

use clap::{Args, Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use wos_synth_core::{
    CheckDisposition, Layer, Prompter, SynthOutcome, SynthTrace, ToolContext, ToolResult,
    synthesize,
};

pub mod tool_context;
#[doc(inline)]
pub use tool_context::{CheckConfig, McpToolContext, ReplayScenario};

/// Application errors at the host boundary.
pub type HostResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Parsed command line shared by the executable and host callers.
#[derive(Parser, Debug)]
#[command(name = "wos-synth", version, about = "WOS LLM synthesis loop")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Supported synthesis and evidence-inspection commands.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Generate a document with independent requested checks.
    Generate(GenerateArgs),
    /// Run the same host with a deterministic provider, without network access.
    DryRun(GenerateArgs),
    /// Check a saved candidate using the same native tools as generation and MCP.
    CheckCandidate {
        #[arg(long)]
        candidate: PathBuf,
        #[command(flatten)]
        tools: CheckArgs,
    },
    /// Explain a saved complete synthesis trace.
    Explain { trace: PathBuf },
}

/// Frozen tool selections and explicit static-only draft intent.
#[derive(Args, Debug)]
pub struct CheckArgs {
    /// JSON file containing separate validation and identified replay selections.
    #[arg(long)]
    pub checks: Option<PathBuf>,
    /// Complete a static-only draft; behavior remains unchecked.
    #[arg(long)]
    pub draft_only: bool,
}

impl CheckArgs {
    /// Load selections once and validate them through the shared catalog.
    ///
    /// # Errors
    /// Returns file/JSON errors or the owning catalog argument error.
    pub fn load(&self) -> HostResult<McpToolContext> {
        let config = match &self.checks {
            Some(path) => serde_json::from_str(&read(path)?)?,
            None => CheckConfig::default(),
        };
        Ok(McpToolContext::new(config, self.draft_only)?)
    }
}

/// Normal generation input, also used by the deterministic dry-run host.
#[derive(Args, Debug)]
pub struct GenerateArgs {
    #[arg(long)]
    pub problem: PathBuf,
    #[arg(long, value_enum, default_value_t = LayerArg::Kernel)]
    pub layer: LayerArg,
    #[arg(long)]
    pub output: PathBuf,
    #[arg(long)]
    /// Evidence output; defaults to the document path with `.synth-trace.json` extension.
    pub trace: Option<PathBuf>,
    #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u32).range(1..))]
    pub max_iterations: u32,
    #[command(flatten)]
    pub tools: CheckArgs,
}

/// WOS authoring layer selected for the provider prompt.
#[derive(clap::ValueEnum, Clone, Debug)]
pub enum LayerArg {
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

/// Execute a parsed command, constructing providers only at this host boundary.
///
/// # Errors
/// Returns input, provider, or host failures. Candidate-check failures use exit 2.
pub async fn run(cli: Cli) -> HostResult<ExitCode> {
    match cli.command {
        Command::Generate(args) => {
            // Reject incomplete checks before requesting credentials or model work.
            let (args, tools) = prepare_generation(args)?;
            let key = std::env::var("ANTHROPIC_API_KEY")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .ok_or("ANTHROPIC_API_KEY is not set or is empty")?;
            let provider = wos_synth_anthropic::AnthropicPrompter::new(key);
            generate_checked(&provider, args, tools).await
        }
        Command::DryRun(args) => {
            let provider = wos_synth_mock::MockPrompter::new();
            // The workflow is canonical; scenarios remain independently supplied.
            provider.expect(
                "",
                include_str!("../../wos-lint/tests/fixtures/shared-validation/positive.json"),
            );
            generate_with(&provider, args).await
        }
        Command::CheckCandidate { candidate, tools } => {
            let assessment = tools.load()?.check_candidate(&read(&candidate)?).await?;
            println!("{}", serde_json::to_string_pretty(&assessment)?);
            Ok(if assessment.disposition == CheckDisposition::Blocked {
                ExitCode::from(2)
            } else {
                ExitCode::SUCCESS
            })
        }
        Command::Explain { trace } => {
            let trace: SynthTrace = serde_json::from_str(&read(&trace)?)?;
            print!("{}", render_trace(&trace));
            Ok(ExitCode::SUCCESS)
        }
    }
}

/// Generate through actual parsed CLI options with an injected provider and real tools.
///
/// # Errors
/// Returns invalid configuration, input/output, or provider failures.
pub async fn generate_with(provider: &dyn Prompter, args: GenerateArgs) -> HostResult<ExitCode> {
    let (args, tools) = prepare_generation(args)?;
    generate_checked(provider, args, tools).await
}

fn prepare_generation(mut args: GenerateArgs) -> HostResult<(GenerateArgs, McpToolContext)> {
    let output = output_destination(&args.output)?;
    let trace = output_destination(
        &args
            .trace
            .clone()
            .unwrap_or_else(|| args.output.with_extension("synth-trace.json")),
    )?;
    require_distinct_outputs(&output, &trace)?;
    // Freeze the effective destinations before asynchronous provider work.
    args.output = output;
    args.trace = Some(trace);
    let tools = args.tools.load()?;
    tools.require_generation_checks()?;
    Ok((args, tools))
}

fn require_distinct_outputs(output: &Path, trace: &Path) -> HostResult<()> {
    if output == trace
        || (output.try_exists()? && trace.try_exists()? && same_file::is_same_file(output, trace)?)
    {
        return Err("document and trace outputs must use different paths".into());
    }
    Ok(())
}

fn output_destination(path: &Path) -> std::io::Result<PathBuf> {
    match std::fs::canonicalize(path) {
        Ok(destination) => return Ok(destination),
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error),
        Err(_) => {}
    }
    // canonicalize cannot resolve a dangling final symlink. Preserve its real
    // destination rather than treating the link's own filename as the output.
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            let target = std::fs::read_link(path)?;
            let target = if target.is_absolute() {
                target
            } else {
                path.parent().unwrap_or_else(|| Path::new(".")).join(target)
            };
            return output_destination(&target);
        }
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error),
        _ => {}
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let filename = path.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "output requires a file name",
        )
    })?;
    Ok(std::fs::canonicalize(parent)?.join(filename))
}

async fn generate_checked(
    provider: &dyn Prompter,
    args: GenerateArgs,
    tools: McpToolContext,
) -> HostResult<ExitCode> {
    let outcome = synthesize(
        provider,
        &tools,
        &read(&args.problem)?,
        args.layer.into(),
        args.max_iterations,
    )
    .await?;
    let (document, trace, label, code) = match &outcome {
        SynthOutcome::Converged { document, trace } => (
            document,
            trace,
            "requested synthesis checks passed",
            ExitCode::SUCCESS,
        ),
        SynthOutcome::Draft { document, trace } => (
            document,
            trace,
            "DRAFT ONLY: requested static checks passed; behavior unchecked",
            ExitCode::SUCCESS,
        ),
        SynthOutcome::Interrupted {
            last_attempt,
            trace,
        } => (
            last_attempt,
            trace,
            "synthesis interrupted; completed check evidence retained",
            ExitCode::from(1),
        ),
        SynthOutcome::Unconverged {
            last_attempt,
            trace,
            ..
        } => (
            last_attempt,
            trace,
            "required synthesis checks incomplete or unsuccessful",
            ExitCode::from(2),
        ),
    };
    std::fs::write(&args.output, document)?;
    let trace_path = args
        .trace
        .unwrap_or_else(|| args.output.with_extension("synth-trace.json"));
    // Creating the candidate makes previously absent aliases observable, such
    // as differently cased names on a case-insensitive filesystem. Never let
    // the trace truncate that candidate; report the output error instead.
    require_distinct_outputs(&args.output, &trace_path)?;
    std::fs::write(&trace_path, serde_json::to_string_pretty(trace)?)?;
    if let Some(interruption) = &trace.interruption {
        eprintln!("{}: {}", interruption.operation, interruption.error);
    }
    eprintln!(
        "{label}; {} iteration(s); wrote {}; evidence {}",
        trace.iterations.len(),
        args.output.display(),
        trace_path.display()
    );
    Ok(code)
}

fn read(path: &Path) -> HostResult<String> {
    std::fs::read_to_string(path)
        .map_err(|error| format!("reading {}: {error}", path.display()).into())
}

/// Render owner status, identities, absence and errors from a complete saved trace.
pub fn render_trace(trace: &SynthTrace) -> String {
    use std::fmt::Write as _;
    let mut out = format!(
        "synth-trace: {} iteration(s); tokens in/out/cache = {}\n",
        trace.iterations.len(),
        format_tokens(
            trace.total_input_tokens,
            trace.total_output_tokens,
            trace.total_cache_read_tokens
        )
    );
    if let Some(interruption) = &trace.interruption {
        writeln!(
            out,
            "  interrupted during {}: {}",
            interruption.operation, interruption.error
        )
        .expect("writing to string");
    }
    for iteration in &trace.iterations {
        writeln!(
            out,
            "  iter {}: {:?}; {} finding(s); tokens in/out/cache = {}",
            iteration.index,
            iteration.assessment.disposition,
            iteration.assessment.findings.len(),
            format_tokens(
                iteration.input_tokens,
                iteration.output_tokens,
                iteration.cache_read_tokens
            )
        )
        .expect("writing to string");
        if let Some(error) = &iteration.assessment.intake_error {
            writeln!(out, "    intake error: {error}").expect("writing to string");
        }
        for record in
            std::iter::once(&iteration.assessment.validation).chain(&iteration.assessment.replays)
        {
            let summary = match &record.result {
                ToolResult::Completed { report } => format!(
                    "status={}; identity={}",
                    report
                        .pointer("/validation/status")
                        .or_else(|| report.get("status"))
                        .unwrap_or(&serde_json::Value::Null),
                    report.get("identity").unwrap_or(&serde_json::Value::Null)
                ),
                ToolResult::Error { error } => format!("error: {error}"),
                ToolResult::NotRun { reason } => format!("not run: {reason}"),
            };
            writeln!(out, "    {}: {summary}", record.id).expect("writing to string");
        }
        for finding in &iteration.assessment.findings {
            writeln!(
                out,
                "    [{}] {:?}: {}",
                finding.rule_id, finding.severity, finding.message
            )
            .expect("writing to string");
        }
    }
    out
}

fn format_tokens(input: u64, output: u64, cache: u64) -> String {
    if input == 0 && output == 0 && cache == 0 {
        "unknown (provider does not report)".into()
    } else {
        format!("{input}/{output}/{cache}")
    }
}
