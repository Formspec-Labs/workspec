//! Authentication trait and shared types.
//!
//! Handlers talk to [`AuthProvider`] only — never a concrete impl. Swapping in
//! OIDC / Keycloak / managed identity is a new impl crate plus a config variant.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type AuthHandle = Arc<dyn AuthProvider>;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("invalid or expired token")]
    InvalidToken,

    #[error("revoked")]
    Revoked,

    #[error("{0}")]
    Other(String),
}

pub type AuthResult<T> = Result<T, AuthError>;

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
    pub access_token: Option<String>,
}

#[async_trait]
pub trait AuthProvider: Send + Sync + 'static {
    async fn login(&self, email: &str, password: &str) -> AuthResult<TokenPair>;
    async fn refresh(&self, refresh_token: &str) -> AuthResult<TokenPair>;
    async fn logout(&self, access_token: &str) -> AuthResult<()>;
    async fn verify(&self, access_token: &str) -> AuthResult<AuthContext>;
}
