//! Authentication adapter dispatch.
//!
//! Trait + types live in [`wos_server_ports::auth`]; this module re-exports them
//! and adds concrete JWT + mock adapters + build helper.

pub mod jwt;
pub mod middleware;
pub mod mock;

pub use jwt::JwtAuth;
#[allow(deprecated)]
pub use middleware::require_role;
pub use middleware::{
    Adjudicator, Applicant, AuthCtx, RequireAuth, RequireRole, Role, Supervisor,
};
pub use mock::MockAuth;

pub use wos_server_ports::auth::{AuthContext, AuthError, AuthHandle, AuthProvider, AuthResult, AuthUser, TokenPair};

use crate::config::{AuthKind, ServerConfig};
use crate::storage::StorageHandle;

pub fn build(cfg: &ServerConfig, storage: StorageHandle) -> AuthHandle {
    match cfg.auth {
        AuthKind::Jwt => std::sync::Arc::new(JwtAuth::new(
            cfg.jwt_secret.as_bytes(),
            cfg.jwt_access_ttl_secs,
            cfg.jwt_refresh_ttl_secs,
            storage,
        )),
        AuthKind::Mock => std::sync::Arc::new(MockAuth::default()),
    }
}
