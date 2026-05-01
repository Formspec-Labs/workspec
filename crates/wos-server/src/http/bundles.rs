use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::routing::{get, post};

use crate::AppState;
use crate::auth::{RequireRole, Supervisor};
use crate::domain::{BundleView, KernelSummaryView, ValidationResultView};
use crate::error::{ApiError, ApiResult};
use crate::services::bundle_service::validate_kernel;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/bundles", get(list))
        .route("/bundles/{url}", get(get_full))
        .route("/bundles/{url}/kernel", get(get_kernel).put(put_kernel))
        .route("/kernel/validate", post(validate))
}

async fn list(State(s): State<AppState>) -> Json<Vec<KernelSummaryView>> {
    Json(s.services.bundle.list().await)
}

async fn get_full(
    State(s): State<AppState>,
    Path(url): Path<String>,
) -> ApiResult<Json<BundleView>> {
    s.services
        .bundle
        .full_bundle(&url)
        .await
        .map(Json)
        .ok_or(ApiError::NotFound)
}

async fn get_kernel(
    State(s): State<AppState>,
    Path(url): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    s.services
        .bundle
        .get(&url)
        .await
        .map(|r| Json(r.document))
        .ok_or(ApiError::NotFound)
}

async fn put_kernel(
    State(s): State<AppState>,
    Path(url): Path<String>,
    _: RequireRole<Supervisor>,
    Json(doc): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let result = validate_kernel(&doc);
    if !result.is_valid {
        return Err(ApiError::Validation {
            issues: serde_json::to_value(&result.issues)?,
        });
    }
    s.services
        .bundle
        .replace(&url, doc)
        .await
        .map_err(ApiError::Storage)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn validate(Json(doc): Json<serde_json::Value>) -> Json<ValidationResultView> {
    Json(validate_kernel(&doc))
}
