//! Authentication trait and implementations.
//!
//! All handlers talk to [`AuthProvider`] only — never a concrete impl — so
//! swapping in OIDC / SAML / managed identity later is a single new impl
//! plus a [`AuthKind`](crate::config::AuthKind) variant.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::{AuthKind, ServerConfig};
use crate::storage::StorageHandle;

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

pub type AuthHandle = Arc<dyn AuthProvider>;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("invalid or expired token")]
    InvalidToken,

    #[error("revoked")]
    Revoked,

    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),

    #[error("{0}")]
    Other(String),
}

pub type AuthResult<T> = Result<T, AuthError>;

/// Subset of [`UserRow`](crate::storage::UserRow) safe to expose to clients.
/// Matches the studio's `AuthUser` shape (`WosPorts.ts:270–276`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthUser {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: chrono::DateTime<chrono::Utc>,
    pub refresh_expires_at: chrono::DateTime<chrono::Utc>,
    pub user: AuthUser,
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user: AuthUser,
    pub jti: String,
    /// Raw access JWT from the `Authorization: Bearer` header, set only by
    /// [`crate::auth::middleware::attach_auth`] so [`AuthProvider::logout`] can
    /// decode and revoke the correct session.
    pub access_token: Option<String>,
}

#[async_trait]
pub trait AuthProvider: Send + Sync + 'static {
    async fn login(&self, email: &str, password: &str) -> AuthResult<TokenPair>;
    async fn refresh(&self, refresh_token: &str) -> AuthResult<TokenPair>;
    /// End the caller's session. For JWT, implementations should invalidate
    /// refresh as well as access (for example by revoking all session rows
    /// for the user) so a stolen refresh token cannot continue after logout.
    async fn logout(&self, access_token: &str) -> AuthResult<()>;
    async fn verify(&self, access_token: &str) -> AuthResult<AuthContext>;
}

pub fn build(cfg: &ServerConfig, storage: StorageHandle) -> AuthHandle {
    match cfg.auth {
        AuthKind::Jwt => Arc::new(JwtAuth::new(
            cfg.jwt_secret.as_bytes(),
            cfg.jwt_access_ttl_secs,
            cfg.jwt_refresh_ttl_secs,
            storage,
        )),
        AuthKind::Mock => Arc::new(MockAuth::default()),
    }
}
