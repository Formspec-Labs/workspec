//! HTTP router composition.

use axum::Router;
use axum::http::{HeaderValue, Method, header};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::AppState;

pub mod advanced;
pub mod agents;
pub mod ai_chat;
pub mod applicant;
pub mod assurance;
pub mod auth;
pub mod bundles;
pub mod calendar;
pub mod conformance;
pub mod dashboard;
pub mod deontic;
pub mod governance;
pub mod health;
pub mod instances;
pub mod integration;
pub mod lint;
pub mod notifications;
pub mod tasks;

pub fn router(state: AppState) -> Router {
    let cors = build_cors(&state.cfg.cors_origin);

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
        .merge(lint::routes())
        .merge(conformance::routes())
        .merge(calendar::routes())
        .merge(notifications::routes())
        .merge(deontic::routes())
        .merge(assurance::routes())
        .merge(integration::routes())
        .merge(agents::routes())
        .merge(advanced::routes())
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::auth::middleware::attach_auth,
        ));

    Router::new()
        .nest("/api", api)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}

fn health_router() -> Router<AppState> {
    // Infra liveness at `/healthz`. Governance owns `/health` and returns a
    // `ServiceHealthView[]` per the studio `IGovernanceReader.getHealthStatus`
    // contract; `/healthz` is the lightweight probe for load balancers.
    Router::new().route("/healthz", axum::routing::get(health::get))
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
