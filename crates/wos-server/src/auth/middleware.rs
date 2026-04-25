use std::marker::PhantomData;

use axum::extract::{FromRequestParts, Request};
use axum::http::header;
use axum::http::request::Parts;
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

/// Free-function role check. Kept for backward compatibility while call sites
/// migrate to [`RequireRole<R>`]. Prefer the extractor for new handlers — it
/// moves the role string from a runtime literal (typo-prone) to a compile-time
/// type that does not exist if mistyped.
#[deprecated(
    since = "0.1.0",
    note = "Use the `RequireRole<R: Role>` extractor (e.g. `RequireRole<Supervisor>`); typos in the role string become compile errors."
)]
pub fn require_role(ctx: &AuthContext, expected: &str) -> Result<(), ApiError> {
    if ctx.user.role.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(ApiError::Forbidden)
    }
}

// ---------------------------------------------------------------------------
// WS-083: Role markers + `RequireRole<R>` extractor
// ---------------------------------------------------------------------------
//
// The `&'static str` const-generic form (`RequireRole<const ROLE: &'static
// str>`) needs `#![feature(adt_const_params)]` (nightly). The standard
// stable-Rust workaround is a marker trait with an associated `NAME`
// constant; the extractor reads `R::NAME` at runtime, but the role identity
// is carried entirely in the type, so a typo cannot compile.
//
// Adding a new role = declare a unit struct + `impl Role`. No string
// duplication anywhere in the call sites.

pub trait Role: Send + Sync + 'static {
    const NAME: &'static str;
}

#[derive(Debug, Clone, Copy)]
pub struct Supervisor;
impl Role for Supervisor {
    const NAME: &'static str = "Supervisor";
}

#[derive(Debug, Clone, Copy)]
pub struct Adjudicator;
impl Role for Adjudicator {
    const NAME: &'static str = "Adjudicator";
}

#[derive(Debug, Clone, Copy)]
pub struct Applicant;
impl Role for Applicant {
    const NAME: &'static str = "Applicant";
}

/// Auth-required extractor that ALSO checks the caller's role against `R`.
/// `RequireRole<Supervisor>` is the most common form. Anonymous → 401;
/// authenticated-but-wrong-role → 403.
pub struct RequireRole<R: Role>(pub AuthContext, PhantomData<R>);

impl<R: Role> RequireRole<R> {
    pub fn into_inner(self) -> AuthContext {
        self.0
    }
}

impl<S: Send + Sync, R: Role> FromRequestParts<S> for RequireRole<R> {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let ctx = parts
            .extensions
            .get::<AuthContext>()
            .cloned()
            .ok_or(ApiError::Unauthorized)?;
        if ctx.user.role.eq_ignore_ascii_case(R::NAME) {
            Ok(RequireRole(ctx, PhantomData))
        } else {
            Err(ApiError::Forbidden)
        }
    }
}
