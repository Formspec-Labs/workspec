use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::routing::get;

use crate::AppState;
use crate::domain::{
    AlertView, DashboardMetricsView, DriftDataPointView, PipelineDataPointView, StageMetricView,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/dashboard/metrics", get(metrics))
        .route("/dashboard/stage-metrics", get(stage_metrics))
        .route("/dashboard/alerts", get(alerts))
        .route("/dashboard/drift-data", get(drift_data))
        .route("/dashboard/pipeline-data", get(pipeline_data))
}

async fn metrics(State(s): State<AppState>) -> Json<DashboardMetricsView> {
    Json(s.services.dashboard.metrics().await)
}

async fn stage_metrics(State(s): State<AppState>) -> Json<Vec<StageMetricView>> {
    Json(s.services.dashboard.stage_metrics().await)
}

async fn alerts(State(s): State<AppState>) -> Json<Vec<AlertView>> {
    Json(s.services.dashboard.alerts().await)
}

async fn drift_data(State(s): State<AppState>) -> Json<Vec<DriftDataPointView>> {
    Json(s.services.dashboard.drift_data().await)
}

async fn pipeline_data(State(s): State<AppState>) -> Json<Vec<PipelineDataPointView>> {
    Json(s.services.dashboard.pipeline_data().await)
}
