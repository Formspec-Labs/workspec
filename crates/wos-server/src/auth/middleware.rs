use axum::extract::{FromRequestParts, Request};
use axum::http::request::Parts;
use axum::http::header;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use super::{AuthContext, AuthError};
use crate::AppState;
use crate::error::ApiError;

/// Attach an [`AuthContext`] to the request extensions if a valid bearer is
/// present. Does NOT reject anonymous requests — handlers that require auth
/// use the [`RequireAuth`] extractor instead.
///
/// A malformed or expired `Authorization: Bearer …` header is ignored and the
/// request continues without auth (optional-auth pattern), not 401 — unless
/// `WOS_BEARER_STRICT` is set on [`crate::config::ServerConfig`], in which case
/// any `Authorization` header must be a non-empty Bearer token that verifies.
pub async fn attach_auth(
    axum::extract::State(state): axum::extract::State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let strict = state.cfg.bearer_strict;
    let authz = req.headers().get(header::AUTHORIZATION);

    let bearer_token: Option<String> = match authz {
        None => None,
        Some(h) => match h.to_str() {
            Ok(s) => s
                .strip_prefix("Bearer ")
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(|t| t.to_string()),
            Err(_) if strict => return ApiError::Unauthorized.into_response(),
            Err(_) => None,
        },
    };

    if authz.is_some() && bearer_token.is_none() {
        if strict {
            return ApiError::Unauthorized.into_response();
        }
    } else if let Some(tok) = bearer_token {
        match state.auth.verify(&tok).await {
            Ok(mut ctx) => {
                ctx.access_token = Some(tok);
                req.extensions_mut().insert(ctx);
            }
            Err(_) if strict => return ApiError::Unauthorized.into_response(),
            Err(_) => {}
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
