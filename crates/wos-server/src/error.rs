use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use thiserror::Error;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("not found")]
    NotFound,

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden")]
    Forbidden,

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("validation failed")]
    Validation { issues: serde_json::Value },

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("payload too large")]
    PayloadTooLarge,

    #[error("service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),

    #[error(transparent)]
    Serde(#[from] serde_json::Error),

    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<crate::services::hold_service::HoldServiceError> for ApiError {
    fn from(e: crate::services::hold_service::HoldServiceError) -> Self {
        use crate::services::hold_service::HoldServiceError as H;
        match e {
            H::NotFound { .. } => ApiError::NotFound,
            H::Storage(s) => ApiError::Storage(s),
        }
    }
}

impl From<wos_runtime::RuntimeError> for ApiError {
    fn from(e: wos_runtime::RuntimeError) -> Self {
        use wos_runtime::RuntimeError as R;
        use wos_runtime::store::StoreError;
        match e {
            R::Store(StoreError::NotFound(_)) => ApiError::NotFound,
            R::TaskNotFound(_) | R::ContractNotFound(_) => ApiError::NotFound,
            R::Store(StoreError::AlreadyExists(m)) => ApiError::Conflict(m),
            R::Unauthorized(_) => ApiError::Forbidden,
            R::InvalidResponseStatus(_)
            | R::UnsupportedAction(_)
            | R::UnsupportedBinding(_)
            | R::UnsupportedBindingKind(_)
            | R::MissingMetadata(_)
            | R::ContractValidation(_) => ApiError::BadRequest(e.to_string()),
            other => ApiError::ServiceUnavailable(other.to_string()),
        }
    }
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    error: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    issues: Option<serde_json::Value>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message, issues) = match &self {
            ApiError::NotFound => (StatusCode::NOT_FOUND, "not_found", None, None),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized", None, None),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "forbidden", None, None),
            ApiError::BadRequest(m) => (
                StatusCode::BAD_REQUEST,
                "bad_request",
                Some(m.clone()),
                None,
            ),
            ApiError::Validation { issues } => (
                StatusCode::BAD_REQUEST,
                "validation_failed",
                None,
                Some(issues.clone()),
            ),
            ApiError::Conflict(m) => (StatusCode::CONFLICT, "conflict", Some(m.clone()), None),
            ApiError::PayloadTooLarge => {
                (StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large", None, None)
            }
            ApiError::ServiceUnavailable(m) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "service_unavailable",
                Some(m.clone()),
                None,
            ),
            other => {
                tracing::error!(error = ?other, "internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    Some(other.to_string()),
                    None,
                )
            }
        };

        let body = ErrorBody {
            error: code,
            message,
            issues,
        };
        (status, Json(body)).into_response()
    }
}
