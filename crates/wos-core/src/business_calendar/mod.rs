// Rust guideline compliant 2026-04-14

//! Business calendar types and SLA deadline evaluation.
//!
//! This module owns the typed model for `BusinessCalendarDocument` sidecars
//! and the `next_business_moment` evaluator that advances a duration past
//! non-business time (weekends, holidays, outside operating hours).

pub mod evaluator;

pub use evaluator::next_business_moment;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A WOS Business Calendar Config sidecar document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessCalendarDocument {
    /// Document type marker. Must be `"1.0"`.
    #[serde(rename = "$wosBusinessCalendar")]
    pub wos_business_calendar: String,

    /// Optional JSON Schema URI.
    #[serde(rename = "$schema", default)]
    pub schema: Option<String>,

    /// Kernel document this calendar targets.
    pub target_workflow: String,

    /// Document version.
    #[serde(default)]
    pub version: Option<String>,

    /// Human-readable title.
    #[serde(default)]
    pub title: Option<String>,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// IANA timezone identifier.
    pub timezone: String,

    /// Working days of the week.
    pub work_week: Vec<Weekday>,

    /// Holiday schedule.
    #[serde(default)]
    pub holidays: Vec<Holiday>,

    /// Operating hours within a business day.
    #[serde(default)]
    pub operating_hours: Option<OperatingHours>,

    /// Date this calendar becomes effective (ISO 8601 date).
    #[serde(default)]
    pub effective_date: Option<String>,

    /// Date this calendar expires (ISO 8601 date).
    #[serde(default)]
    pub expiration_date: Option<String>,

    /// Extension data. Keys MUST start with `x-`.
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Day of the week (lowercase).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

/// A holiday entry — either fixed-date or floating (rule-based).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Holiday {
    /// Human-readable holiday name.
    pub name: String,

    /// Fixed date (ISO 8601 date). One of `date` or `rule` MUST be present.
    #[serde(default)]
    pub date: Option<String>,

    /// Recurrence rule for floating holidays.
    #[serde(default)]
    pub rule: Option<String>,

    /// Whether this is an observed date.
    #[serde(default)]
    pub observed: bool,
}

/// Operating hours within a business day.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatingHours {
    /// Start of operating period (HH:MM).
    pub start: String,

    /// End of operating period (HH:MM).
    pub end: String,
}
