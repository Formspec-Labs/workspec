//! AI chat proxy. Matches `server.ts:602–625` — a 64 KiB, token-gated
//! endpoint that forwards to Google's Gemini API when enabled. All other
//! configurations return 503 so the studio falls back gracefully.

use axum::Json;
use axum::extract::{DefaultBodyLimit, State};
use axum::routing::post;
use axum::Router;

use crate::AppState;
use crate::auth::AuthCtx;
use crate::config::AiChatKind;
use crate::error::{ApiError, ApiResult};

const BODY_LIMIT: usize = 64 * 1024; // 64 KiB

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/ai/chat", post(chat))
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
}

async fn chat(
    State(s): State<AppState>,
    _auth: AuthCtx,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    match s.cfg.ai_chat {
        AiChatKind::Disabled => Err(ApiError::ServiceUnavailable(
            "AI chat is not enabled on this server".into(),
        )),
        AiChatKind::Gemini => {
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key={}",
                s.cfg.gemini_api_key,
            );
            let res = reqwest::Client::new()
                .post(&url)
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
