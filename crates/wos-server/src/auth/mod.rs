//! Authentication adapter dispatch.
//!
//! Trait + types live in [`wos_server_ports::auth`]; this module re-exports them
//! and adds concrete JWT + mock adapters + build helper.

pub mod middleware;
#[cfg(feature = "auth-jwt")]
pub use wos_server_auth_jwt::JwtAuth;
pub use middleware::{
    Adjudicator, Applicant, AuthCtx, RequireAuth, RequireRole, Role, Supervisor,
};
#[cfg(feature = "auth-mock")]
pub use wos_server_auth_mock::MockAuth;

pub use wos_server_ports::auth::{AuthContext, AuthError, AuthHandle, AuthProvider, AuthResult, AuthUser, TokenPair};

use crate::config::{AuthKind, ServerConfig};
use crate::storage::StorageHandle;

pub fn build(cfg: &ServerConfig, storage: StorageHandle) -> anyhow::Result<AuthHandle> {
    match cfg.auth {
        AuthKind::Jwt => {
            #[cfg(feature = "auth-jwt")]
            {
                return Ok(std::sync::Arc::new(JwtAuth::new(
                    cfg.jwt_secret.as_bytes(),
                    cfg.jwt_access_ttl_secs,
                    cfg.jwt_refresh_ttl_secs,
                    storage,
                )));
            }
            #[cfg(not(feature = "auth-jwt"))]
            anyhow::bail!("WOS_AUTH=jwt requested but crate built without feature `auth-jwt`");
        }
        AuthKind::Mock => {
            #[cfg(feature = "auth-mock")]
            {
                return Ok(std::sync::Arc::new(MockAuth::default()));
            }
            #[cfg(not(feature = "auth-mock"))]
            anyhow::bail!("WOS_AUTH=mock requested but crate built without feature `auth-mock`");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AiChatKind, AuditSinkKind, RuntimeKind, SignerKind, StorageKind};
    use crate::storage;
    use std::path::PathBuf;

    fn stub_cfg(auth: AuthKind) -> ServerConfig {
        ServerConfig {
            port: 0,
            fixtures_dir: PathBuf::from("."),
            storage: StorageKind::Sqlite,
            database_url: "sqlite::memory:".into(),
            auth,
            jwt_secret: "test-secret".into(),
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
            session_sweep_enabled: false,
            signer_kind: SignerKind::Noop,
        }
    }

    #[tokio::test]
    async fn auth_build_succeeds_for_enabled_backends() {
        let storage = storage::build(&stub_cfg(AuthKind::Mock))
            .await
            .expect("storage build");
        let cfg = stub_cfg(AuthKind::Mock);
        let _ = build(&cfg, storage.clone()).expect("mock auth build");

        #[cfg(feature = "auth-jwt")]
        {
            let cfg = stub_cfg(AuthKind::Jwt);
            let _ = build(&cfg, storage).expect("jwt auth build");
        }
    }

    #[cfg(not(feature = "auth-jwt"))]
    #[tokio::test]
    async fn auth_build_rejects_jwt_when_feature_disabled() {
        let storage = storage::build(&stub_cfg(AuthKind::Mock))
            .await
            .expect("storage build");
        let err = match build(&stub_cfg(AuthKind::Jwt), storage) {
            Ok(_) => panic!("jwt should be rejected"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("without feature `auth-jwt`"));
    }

    #[cfg(not(feature = "auth-mock"))]
    #[tokio::test]
    async fn auth_build_rejects_mock_when_feature_disabled() {
        let storage = storage::build(&stub_cfg(AuthKind::Jwt))
            .await
            .expect("storage build");
        let err = match build(&stub_cfg(AuthKind::Mock), storage) {
            Ok(_) => panic!("mock should be rejected"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("without feature `auth-mock`"));
    }
}
