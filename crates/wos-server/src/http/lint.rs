use axum::Json;
use axum::extract::State;
use axum::routing::{get, post};
use axum::Router;

use crate::AppState;
use crate::error::ApiResult;
use crate::services::lint_service::{self, LintResult, RuleMetadataView};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/lint/document", post(lint_document))
        .route("/lint/schema", post(lint_schema))
        .route("/lint/rules", get(list_rules))
}

async fn lint_document(
    State(_s): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<LintResult>> {
    Ok(Json(lint_service::lint_document(&body)))
}

async fn lint_schema(
    State(_s): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<LintResult>> {
    Ok(Json(lint_service::lint_schema(&body)))
}

async fn list_rules(State(_s): State<AppState>) -> Json<Vec<RuleMetadataView>> {
    Json(lint_service::list_rules())
}
