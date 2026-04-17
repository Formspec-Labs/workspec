use serde::{Deserialize, Serialize};

/// `DashboardMetrics` in `WosPorts.ts:187`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardMetricsView {
    pub active_instances: u64,
    pub completed_7d: u64,
    pub sla_compliance: f64,
    pub avg_processing_time_days: f64,
    pub ai_acceptance_rate: f64,
    pub active_instances_trend: f64,
    pub completed_7d_trend: f64,
    pub sla_compliance_trend: f64,
    pub avg_processing_time_trend: f64,
    pub ai_acceptance_rate_trend: f64,
}

/// `StageMetricView` in `WosPorts.ts:200`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageMetricView {
    pub name: String,
    pub count: u64,
    pub avg_wait: String,
    pub status: String,
}

/// `AlertView` in `WosPorts.ts:207`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertView {
    pub id: String,
    #[serde(rename = "type")]
    pub alert_type: String,
    pub title: String,
    pub description: String,
    pub time_ago: String,
    pub severity: String,
}

/// `DriftDataPoint` in `WosPorts.ts:216`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriftDataPointView {
    pub week: String,
    pub override_rate: f64,
    pub time_on_task: f64,
}

/// `PipelineDataPoint` in `WosPorts.ts:222`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineDataPointView {
    pub name: String,
    pub volume: u64,
    pub capacity: u64,
}
