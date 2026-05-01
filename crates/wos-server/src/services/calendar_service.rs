//! Business-calendar operations: `computeBusinessDaysDeadline`
//! (Semantic Profile §sidecars.business-calendar).
//!
//! Wraps `wos_core::business_calendar::next_business_moment` and routes
//! the active calendar document through `BundleService`.

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use wos_core::business_calendar::{BusinessCalendarDocument, next_business_moment};
use wos_core::parse_iso_duration_to_ms;

use crate::error::{ApiError, ApiResult};
use crate::services::bundle_service::BundleService;

/// `POST /api/calendar/:url/compute-deadline` request body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeDeadlineRequest {
    /// ISO 8601 duration (e.g. `PT72H`, `P10D`).
    pub duration: String,
    /// RFC 3339 start instant. Defaults to `now`.
    #[serde(default)]
    pub from: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeDeadlineResponse {
    pub deadline: DateTime<Utc>,
    pub from: DateTime<Utc>,
    pub duration: String,
    pub calendar_url: String,
    pub calendar_timezone: String,
}

pub struct CalendarService {
    bundle: Arc<BundleService>,
}

impl CalendarService {
    pub fn new(bundle: Arc<BundleService>) -> Self {
        Self { bundle }
    }

    pub async fn compute_deadline(
        &self,
        calendar_url: &str,
        req: &ComputeDeadlineRequest,
    ) -> ApiResult<ComputeDeadlineResponse> {
        let bundle = self
            .bundle
            .full_bundle(calendar_url)
            .await
            .ok_or(ApiError::NotFound)?;
        let calendar_json = bundle.business_calendar.ok_or_else(|| {
            ApiError::BadRequest("no business calendar sidecar attached to this workflow".into())
        })?;
        let calendar: BusinessCalendarDocument = serde_json::from_value(calendar_json)
            .map_err(|e| ApiError::ServiceUnavailable(format!("parse calendar: {e}")))?;
        let duration_ms = parse_iso_duration_to_ms(&req.duration)
            .map_err(|e| ApiError::BadRequest(format!("invalid duration: {e}")))?;
        let duration = Duration::milliseconds(duration_ms as i64);
        let from = req.from.unwrap_or_else(Utc::now);
        let deadline = next_business_moment(from, duration, &calendar)
            .map_err(|e| ApiError::ServiceUnavailable(format!("calendar compute: {e}")))?;
        let tz = calendar.timezone.clone();
        Ok(ComputeDeadlineResponse {
            deadline,
            from,
            duration: req.duration.clone(),
            calendar_url: calendar_url.to_string(),
            calendar_timezone: tz,
        })
    }
}
