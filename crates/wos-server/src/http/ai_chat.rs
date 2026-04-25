//! AI chat proxy. Matches `server.ts:602–625` — a 64 KiB, token-gated
//! endpoint that forwards to Google's Gemini API when enabled. All other
//! configurations return 503 so the studio falls back gracefully.
//!
//! The API key is sent with the `x-goog-api-key` header (not in the query
//! string) and requests reuse a single [`reqwest::Client`].

use std::sync::LazyLock;

use axum::Json;
use axum::extract::{DefaultBodyLimit, State};
use axum::routing::post;
use axum::Router;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::AppState;
use crate::auth::{RequireRole, Supervisor};
use crate::config::AiChatKind;
use crate::error::{ApiError, ApiResult};

const BODY_LIMIT: usize = 64 * 1024; // 64 KiB

static GEMINI_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .build()
        .expect("reqwest client for Gemini")
});

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/ai/chat", post(chat))
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
}

async fn chat(
    State(s): State<AppState>,
    _: RequireRole<Supervisor>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    match s.cfg.ai_chat {
        AiChatKind::Disabled => Err(ApiError::ServiceUnavailable(
            "AI chat is not enabled on this server".into(),
        )),
        AiChatKind::Gemini => {
            let url = "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent";
            let mut headers = HeaderMap::new();
            headers.insert(
                HeaderName::from_static("x-goog-api-key"),
                HeaderValue::from_str(s.cfg.gemini_api_key.as_str()).map_err(|_| {
                    ApiError::ServiceUnavailable("invalid Gemini API key for HTTP header".into())
                })?,
            );
            let res = GEMINI_CLIENT
                .post(url)
                .headers(headers)
                .json(&body)
                .send()
                .await
                .map_err(|e| ApiError::ServiceUnavailable(format!("gemini call failed: {e}")))?;
            let status = res.status();
            let json: serde_json::Value = res.json().await.map_err(|e| {
                ApiError::ServiceUnavailable(format!("gemini response parse failed: {e}"))
            })?;
            if !status.is_success() {
                return Err(ApiError::ServiceUnavailable(format!(
                    "gemini returned {status}: {json}"
                )));
            }
            Ok(Json(json))
        }
    }
}
