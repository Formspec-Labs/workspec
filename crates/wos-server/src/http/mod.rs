//! HTTP router composition.

use axum::Router;
use axum::http::{HeaderValue, Method, header};
use axum::routing::get;
use socketioxide::layer::SocketIoLayer;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::AppState;

pub mod auth;
pub mod health;

/// Build the axum router and Socket.IO layer. Attach the returned layer as
/// a regular axum service via `.layer(layer)`.
pub fn router(state: AppState) -> (Router, SocketIoLayer) {
    let cors_origin = state.cfg.cors_origin.clone();
    let cors = build_cors(&cors_origin);

    let (io_layer, _io) = crate::realtime::build(state.clone());

    let api = Router::new()
        .route("/health", get(health::get))
        .merge(auth::routes())
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
