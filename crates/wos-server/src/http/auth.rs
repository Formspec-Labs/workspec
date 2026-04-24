use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};

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
        .route("/auth/has-role/{role}", get(has_role))
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
