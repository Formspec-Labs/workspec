use clap::Parser;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use wos_server::ServerConfig;

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

    let cfg = ServerConfig::parse();
    cfg.validate()?;

    wos_server::run(cfg).await
}
