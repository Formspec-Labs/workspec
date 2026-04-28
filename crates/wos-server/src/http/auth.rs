use argon2::{PasswordHasher, PasswordVerifier};
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::AppState;
use crate::auth::{AuthCtx, TokenPair};
use crate::domain::{HasRoleResponse, LoginRequest, RefreshRequest};
use crate::error::{ApiError, ApiResult};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/me", get(me))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/refresh", post(refresh))
        .route("/auth/change-password", post(change_password))
        .route("/auth/has-role/{role}", get(has_role))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// WS-002. Bearer-authenticated. Verifies the caller's current password
/// against the stored Argon2 hash, then rotates via
/// [`Storage::set_user_password_hash`] which updates the hash, bumps
/// `auth_epoch`, and revokes existing sessions in one transaction. The
/// "atomic txn" guarantee from PARITY ▎ Auth contract is upheld in the
/// storage method, not in this handler.
async fn change_password(
    State(state): State<AppState>,
    AuthCtx(ctx): AuthCtx,
    Json(body): Json<ChangePasswordRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let user = state
        .storage
        .get_user(&ctx.user.id)
        .await?
        .ok_or(ApiError::Unauthorized)?;

    let parsed = argon2::password_hash::PasswordHash::new(&user.password_hash)
        .map_err(|_| ApiError::ServiceUnavailable("stored password hash malformed".into()))?;
    argon2::Argon2::default()
        .verify_password(body.current_password.as_bytes(), &parsed)
        .map_err(|_| ApiError::BadRequest("current password is incorrect".into()))?;

    if body.new_password.len() < 8 {
        return Err(ApiError::BadRequest(
            "new password must be at least 8 characters".into(),
        ));
    }

    let salt = argon2::password_hash::SaltString::generate(&mut rand::rngs::OsRng);
    let new_hash = argon2::Argon2::default()
        .hash_password(body.new_password.as_bytes(), &salt)
        .map_err(|e| ApiError::ServiceUnavailable(e.to_string()))?
        .to_string();

    state
        .storage
        .set_user_password_hash(&user.id, &new_hash)
        .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn me(AuthCtx(ctx): AuthCtx) -> ApiResult<Json<crate::auth::AuthUser>> {
    Ok(Json(ctx.user))
}

async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> ApiResult<Json<TokenPair>> {
    let pair = state.auth.login(&body.email, &body.password).await?;
    Ok(Json(pair))
}

async fn logout(
    State(state): State<AppState>,
    AuthCtx(ctx): AuthCtx,
) -> ApiResult<Json<serde_json::Value>> {
    let access = ctx
        .access_token
        .as_deref()
        .ok_or(ApiError::Unauthorized)?;
    state.auth.logout(access).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn refresh(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> ApiResult<Json<TokenPair>> {
    let pair = state.auth.refresh(&body.refresh_token).await?;
    Ok(Json(pair))
}

async fn has_role(
    Path(role): Path<String>,
    ctx: Result<AuthCtx, ApiError>,
) -> ApiResult<Json<HasRoleResponse>> {
    let has = match ctx {
        Ok(AuthCtx(c)) => c.user.role.eq_ignore_ascii_case(&role),
        Err(_) => false,
    };
    Ok(Json(HasRoleResponse { has_role: has }))
}
