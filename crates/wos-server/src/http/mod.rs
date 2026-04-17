//! HTTP router composition.

use axum::Router;
use axum::http::{HeaderValue, Method, header};
use socketioxide::layer::SocketIoLayer;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::AppState;

pub mod ai_chat;
pub mod applicant;
pub mod auth;
pub mod bundles;
pub mod dashboard;
pub mod governance;
pub mod health;
pub mod instances;
pub mod tasks;

pub fn router(state: AppState) -> (Router, SocketIoLayer) {
    let cors = build_cors(&state.cfg.cors_origin);
    let (io_layer, _io) = crate::realtime::build(state.clone());

    let api = Router::new()
        .merge(health_router())
        .merge(auth::routes())
        .merge(bundles::routes())
        .merge(instances::routes())
        .merge(tasks::routes())
        .merge(governance::routes())
        .merge(dashboard::routes())
        .merge(applicant::routes())
        .merge(ai_chat::routes())
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::auth::middleware::attach_auth,
        ));

    let app = Router::new()
        .nest("/api", api)
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    (app, io_layer)
}

fn health_router() -> Router<AppState> {
    Router::new().route("/health", axum::routing::get(health::get))
}

fn build_cors(origin: &str) -> CorsLayer {
    let mut cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);
    if origin == "*" {
        cors = cors.allow_origin(tower_http::cors::Any);
    } else if let Ok(val) = HeaderValue::from_str(origin) {
        cors = cors.allow_origin(val).allow_credentials(true);
    }
    cors
}
