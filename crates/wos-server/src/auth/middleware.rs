use axum::extract::{FromRequestParts, Request};
use axum::http::request::Parts;
use axum::middleware::Next;
use axum::response::Response;

use super::{AuthContext, AuthError};
use crate::AppState;
use crate::error::ApiError;

/// Attach an [`AuthContext`] to the request extensions if a valid bearer is
/// present. Does NOT reject anonymous requests — handlers that require auth
/// use the [`RequireAuth`] extractor instead.
pub async fn attach_auth(
    axum::extract::State(state): axum::extract::State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let token = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string());

    if let Some(tok) = token {
        if let Ok(ctx) = state.auth.verify(&tok).await {
            req.extensions_mut().insert(ctx);
        }
    }
    next.run(req).await
}

pub struct AuthCtx(pub AuthContext);
pub struct RequireAuth(pub AuthContext);

impl<S: Send + Sync> FromRequestParts<S> for AuthCtx {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthContext>()
            .cloned()
            .map(AuthCtx)
            .ok_or(ApiError::Unauthorized)
    }
}

impl<S: Send + Sync> FromRequestParts<S> for RequireAuth {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthContext>()
            .cloned()
            .map(RequireAuth)
            .ok_or(ApiError::Unauthorized)
    }
}

impl From<AuthError> for ApiError {
    fn from(e: AuthError) -> Self {
        match e {
            AuthError::InvalidCredentials | AuthError::InvalidToken | AuthError::Revoked => {
                ApiError::Unauthorized
            }
            AuthError::Storage(s) => ApiError::Storage(s),
            AuthError::Other(m) => ApiError::Other(anyhow::anyhow!(m)),
        }
    }
}

pub fn require_role(ctx: &AuthContext, expected: &str) -> Result<(), ApiError> {
    if ctx.user.role.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(ApiError::Forbidden)
    }
}
