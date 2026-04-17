use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

/// Top-level CLI: the default invocation is `wos-server` (no subcommand),
/// which boots the server. Add `wos-server export <id>` to dump provenance
/// to PROV-O / XES / OCEL.
#[derive(Parser, Debug, Clone)]
#[command(
    name = "wos-server",
    about = "Reference WOS HTTP + Socket.IO server",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[command(flatten)]
    pub serve: ServerConfig,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Export provenance for an instance to PROV-O / XES / OCEL.
    Export(ExportArgs),
}

#[derive(Args, Debug, Clone)]
pub struct ExportArgs {
    /// Instance identifier to export.
    pub instance_id: String,

    /// Target format.
    #[arg(long, value_enum, default_value_t = ExportFormat::ProvO)]
    pub format: ExportFormat,

    /// Namespace for minted IRIs. Must end with `:` or `/`.
    #[arg(
        long,
        default_value = "urn:wos:prov:wos-server:"
    )]
    pub namespace: String,

    /// Output path. `-` (default) writes to stdout.
    #[arg(long, default_value = "-")]
    pub out: String,

    #[command(flatten)]
    pub server: ServerConfig,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    #[value(name = "prov-o")]
    ProvO,
    Xes,
    Ocel,
}

#[derive(Args, Debug, Clone)]
#[command(
    about = "Boot the HTTP + Socket.IO server"
)]
pub struct ServerConfig {
    /// TCP port to listen on.
    #[arg(long, env = "PORT", default_value_t = 4000)]
    pub port: u16,

    /// Directory containing fixture kernels + sidecars to seed from.
    #[arg(long, env = "WOS_FIXTURES_DIR", default_value = "fixtures")]
    pub fixtures_dir: PathBuf,

    /// Storage backend selector.
    #[arg(long, env = "WOS_STORAGE", value_enum, default_value_t = StorageKind::Sqlite)]
    pub storage: StorageKind,

    /// Database connection URL (SQLite file path, or `sqlite::memory:`).
    #[arg(
        long,
        env = "WOS_DATABASE_URL",
        default_value = "sqlite://./wos-server.db?mode=rwc"
    )]
    pub database_url: String,

    /// Auth provider selector.
    #[arg(long, env = "WOS_AUTH", value_enum, default_value_t = AuthKind::Jwt)]
    pub auth: AuthKind,

    /// HMAC secret for JWT HS256. Required when `--auth jwt`. May be raw or hex.
    #[arg(long, env = "WOS_JWT_SECRET", default_value = "")]
    pub jwt_secret: String,

    /// Access-token lifetime (seconds).
    #[arg(long, env = "WOS_JWT_ACCESS_TTL_SECS", default_value_t = 900)]
    pub jwt_access_ttl_secs: i64,

    /// Refresh-token lifetime (seconds).
    #[arg(
        long,
        env = "WOS_JWT_REFRESH_TTL_SECS",
        default_value_t = 7 * 24 * 3600
    )]
    pub jwt_refresh_ttl_secs: i64,

    /// CORS allow-origin value. `*` disables credentials; set a specific origin
    /// to enable cookie/authorization-header sharing.
    #[arg(
        long,
        env = "WOS_CORS_ORIGIN",
        default_value = "http://localhost:3000"
    )]
    pub cors_origin: String,

    /// Seed the database from `--fixtures-dir` if empty.
    #[arg(long, env = "WOS_SEED", default_value_t = false)]
    pub seed: bool,

    /// AI chat backend. `disabled` returns 503; `gemini` forwards to Google's API.
    #[arg(long, env = "WOS_AI_CHAT", value_enum, default_value_t = AiChatKind::Disabled)]
    pub ai_chat: AiChatKind,

    /// Gemini API key (required when `--ai-chat gemini`).
    #[arg(long, env = "GEMINI_API_KEY", default_value = "")]
    pub gemini_api_key: String,

    /// Socket.IO cursor-update throttle (ms per socket).
    #[arg(long, env = "WOS_CURSOR_THROTTLE_MS", default_value_t = 50)]
    pub cursor_throttle_ms: u64,

    /// Timer poll interval (ms).
    #[arg(long, env = "WOS_TIMER_POLL_MS", default_value_t = 1000)]
    pub timer_poll_ms: u64,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageKind {
    Sqlite,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthKind {
    Jwt,
    Mock,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiChatKind {
    Disabled,
    Gemini,
}

impl ServerConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        if matches!(self.auth, AuthKind::Jwt) && self.jwt_secret.trim().is_empty() {
            anyhow::bail!(
                "WOS_JWT_SECRET must be set when WOS_AUTH=jwt (generate with `openssl rand -hex 32`)"
            );
        }
        if matches!(self.ai_chat, AiChatKind::Gemini) && self.gemini_api_key.trim().is_empty() {
            anyhow::bail!("GEMINI_API_KEY must be set when WOS_AI_CHAT=gemini");
        }
        if self.cors_origin == "*" && matches!(self.auth, AuthKind::Mock) {
            anyhow::bail!("Refusing to start with WOS_CORS_ORIGIN=* and WOS_AUTH=mock (unsafe)");
        }
        Ok(())
    }
}
