use std::path::PathBuf;

use clap::builder::BoolishValueParser;
use clap::{Args, Parser, Subcommand, ValueEnum};
use http::HeaderValue;

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

    /// When set, refuse to start if `WOS_CORS_ORIGIN` is not `*` and is not a
    /// valid HTTP header value for `Access-Control-Allow-Origin`. When unset,
    /// an invalid origin logs a warning and the server falls back to
    /// permissive origins without credentials.
    #[arg(
        long,
        env = "WOS_CORS_STRICT",
        default_value_t = false,
        value_parser = BoolishValueParser::new()
    )]
    pub cors_strict: bool,

    /// When set, a present `Authorization` header must be `Bearer <token>` with a
    /// non-empty token that passes verification; otherwise the response is 401
    /// instead of treating the caller as anonymous.
    #[arg(
        long,
        env = "WOS_BEARER_STRICT",
        default_value_t = false,
        value_parser = BoolishValueParser::new()
    )]
    pub bearer_strict: bool,

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

    /// Runtime adapter selector.
    #[arg(long, env = "WOS_RUNTIME", value_enum, default_value_t = RuntimeKind::Local)]
    pub runtime: RuntimeKind,

    /// Audit sink selector.
    #[arg(long, env = "WOS_AUDIT_SINK", value_enum, default_value_t = AuditSinkKind::None)]
    pub audit_sink: AuditSinkKind,

    /// Audit sink database URL.
    #[arg(long, env = "WOS_AUDIT_DATABASE_URL", default_value = "")]
    pub audit_database_url: String,

    /// Run the daily session-table sweep inside `timer_task` (deletes rows
    /// where `expires_at < now - 7d`, plus revoked rows older than 30d).
    /// Set `WOS_SESSION_SWEEP=off|false|0` to opt out for ops parity with
    /// external schedulers; default `true`.
    #[arg(
        long = "session-sweep",
        env = "WOS_SESSION_SWEEP",
        default_value_t = true,
        value_parser = parse_session_sweep
    )]
    pub session_sweep_enabled: bool,

    /// Provenance signer backend. **Today only `noop` is wired.** `noop`
    /// ships spec-correct empty-signature attestation blocks. `Ed25519File`
    /// (WS-043) and `External` will be added with their impls; until then,
    /// `WOS_SIGNER=ed25519-file` (or any other non-`noop` value) currently
    /// produces a clap error — there is no fallback.
    #[arg(long, env = "WOS_SIGNER", value_enum, default_value_t = SignerKind::Noop)]
    pub signer_kind: SignerKind,
}

/// Parse `WOS_SESSION_SWEEP` / `--session-sweep`. Recognises `off`, `false`,
/// and `0` as `false`; everything else (including the empty string) is
/// `true` so the daily sweep is opt-out, matching the env-var contract
/// documented on [`ServerConfig::session_sweep_enabled`].
fn parse_session_sweep(s: &str) -> Result<bool, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "off" | "false" | "0" => Ok(false),
        _ => Ok(true),
    }
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageKind {
    Sqlite,
    Postgres,
    Embedded,
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

/// Provenance signer selection. **Today only `Noop` is wired** — there
/// is no fallback. `Ed25519File` (WS-043) and `External` will be added
/// as variants when their impls land; until then, any non-`noop` value
/// passed via `WOS_SIGNER=…` produces a clap parse error rather than
/// silently degrading.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignerKind {
    Noop,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeKind {
    Local,
    Restate,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditSinkKind {
    None,
    Postgres,
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
        if self.cors_strict && self.cors_origin != "*" && HeaderValue::from_str(&self.cors_origin).is_err()
        {
            anyhow::bail!(
                "WOS_CORS_STRICT is enabled but WOS_CORS_ORIGIN is not a valid HTTP header value ({:?}); set a valid origin URL, use \"*\", or unset WOS_CORS_STRICT",
                self.cors_origin
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_jwt_cfg() -> ServerConfig {
        ServerConfig {
            port: 0,
            fixtures_dir: PathBuf::from("."),
            storage: StorageKind::Sqlite,
            database_url: "sqlite::memory:".into(),
            auth: AuthKind::Jwt,
            jwt_secret: "x".into(),
            jwt_access_ttl_secs: 900,
            jwt_refresh_ttl_secs: 3600,
            cors_origin: "http://localhost:3000".into(),
            cors_strict: false,
            bearer_strict: false,
            seed: false,
            ai_chat: AiChatKind::Disabled,
            gemini_api_key: String::new(),
            cursor_throttle_ms: 50,
            timer_poll_ms: 1000,
            runtime: RuntimeKind::Local,
            audit_sink: AuditSinkKind::None,
            audit_database_url: String::new(),
            session_sweep_enabled: true,
            signer_kind: SignerKind::Noop,
        }
    }

    #[test]
    fn cors_strict_rejects_invalid_origin() {
        let mut cfg = minimal_jwt_cfg();
        cfg.cors_strict = true;
        cfg.cors_origin = "http://bad\nhost".into();
        let err = cfg.validate().unwrap_err();
        assert!(
            err.to_string().contains("WOS_CORS_STRICT"),
            "{err}"
        );
    }

    #[test]
    fn cors_strict_allows_wildcard_without_header_parse() {
        let mut cfg = minimal_jwt_cfg();
        cfg.cors_strict = true;
        cfg.cors_origin = "*".into();
        cfg.validate().unwrap();
    }
}
