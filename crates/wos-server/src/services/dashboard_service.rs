//! Deterministic dashboard aggregations over stored instances + provenance.

use chrono::{Duration, Utc};

use crate::domain::{
    AlertView, DashboardMetricsView, DriftDataPointView, PipelineDataPointView, StageMetricView,
};
use crate::storage::{self, InstanceQuery, StorageHandle, LIST_INSTANCES_PAGE_SIZE_MAX};

pub struct DashboardService {
    storage: StorageHandle,
}

impl DashboardService {
    pub fn new(storage: StorageHandle) -> Self {
        Self { storage }
    }

    pub async fn metrics(&self) -> DashboardMetricsView {
        let items: Vec<_> = storage::list_instances_all_pages(
            &self.storage,
            InstanceQuery::default(),
            LIST_INSTANCES_PAGE_SIZE_MAX,
        )
        .await
        .unwrap_or_default();
        let items = items.as_slice();
        let active = items.iter().filter(|r| r.status == "active").count() as u64;
        let completed_7d = items
            .iter()
            .filter(|r| r.status == "completed" && r.updated_at > Utc::now() - Duration::days(7))
            .count() as u64;

        DashboardMetricsView {
            active_instances: active,
            completed_7d,
            sla_compliance: 0.94,
            avg_processing_time_days: 3.2,
            ai_acceptance_rate: 0.82,
            active_instances_trend: 0.0,
            completed_7d_trend: 0.0,
            sla_compliance_trend: 0.0,
            avg_processing_time_trend: 0.0,
            ai_acceptance_rate_trend: 0.0,
            synthetic_fields: vec![
                "slaCompliance".into(),
                "avgProcessingTimeDays".into(),
                "aiAcceptanceRate".into(),
                "activeInstancesTrend".into(),
                "completed7dTrend".into(),
                "slaComplianceTrend".into(),
                "avgProcessingTimeTrend".into(),
                "aiAcceptanceRateTrend".into(),
            ],
        }
    }

    pub async fn stage_metrics(&self) -> Vec<StageMetricView> {
        let Ok(rows) = storage::list_instances_all_pages(
            &self.storage,
            InstanceQuery::default(),
            LIST_INSTANCES_PAGE_SIZE_MAX,
        )
        .await
        else {
            return Vec::new();
        };
        use std::collections::BTreeMap;
        let mut by_state: BTreeMap<String, u64> = BTreeMap::new();
        for row in &rows {
            for name in row.configuration() {
                *by_state.entry(name).or_default() += 1;
            }
        }
        by_state
            .into_iter()
            .map(|(name, count)| StageMetricView {
                name,
                count,
                avg_wait: "—".into(),
                status: "normal".into(),
            })
            .collect()
    }

    pub async fn alerts(&self) -> Vec<AlertView> {
        Vec::new()
    }

    /// Returns a synthetic weekly drift series. Values are stub fixtures
    /// (`override_rate` and `time_on_task`) — not measured observations.
    /// The reference server carries no drift telemetry; an analytics-backed
    /// implementation will replace this.
    pub async fn drift_data(&self) -> Vec<DriftDataPointView> {
        let now = Utc::now();
        (0..6)
            .rev()
            .map(|weeks_ago| {
                let day = now - Duration::weeks(weeks_ago);
                DriftDataPointView {
                    week: day.format("%Y-W%V").to_string(),
                    override_rate: 0.08,
                    time_on_task: 25.0,
                }
            })
            .collect()
    }

    pub async fn pipeline_data(&self) -> Vec<PipelineDataPointView> {
        self.stage_metrics()
            .await
            .into_iter()
            .map(|s| PipelineDataPointView {
                name: s.name,
                volume: s.count,
                capacity: s.count.max(10),
            })
            .collect()
    }
}
