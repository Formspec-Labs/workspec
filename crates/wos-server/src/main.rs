use clap::Parser;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use wos_server::config::{Cli, Command};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("wos_server=info,tower_http=info")),
        )
        .with(fmt::layer().with_target(false))
        .init();

    let cli = Cli::parse();
    match cli.command {
        None => {
            cli.serve.validate()?;
            wos_server::run(cli.serve).await
        }
        Some(Command::Export(args)) => {
            args.server.validate()?;
            wos_server::export::run(args).await
        }
    }
}
