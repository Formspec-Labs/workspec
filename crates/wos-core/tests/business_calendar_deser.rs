// Rust guideline compliant 2026-04-10

//! Round-trip deserialization tests for WOS Business Calendar Config documents.

use std::fs;
use wos_core::BusinessCalendarDocument;
use wos_core::model::business_calendar::Weekday;

fn load_fixture(name: &str) -> BusinessCalendarDocument {
    let path = format!(
        "{}/fixtures/sidecars/{name}",
        env!("CARGO_MANIFEST_DIR").replace("/crates/wos-core", "")
    );
    let json =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read fixture {path}: {e}"));
    serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("failed to deserialize fixture {name}: {e}"))
}

#[test]
fn benefits_business_calendar_round_trips() {
    let doc = load_fixture("benefits-business-calendar.json");
    assert_eq!(doc.wos_business_calendar, "1.0");
    assert!(doc.target_workflow.contains("benefits-adjudication"));
    assert_eq!(doc.timezone, "America/New_York");

    // Work week: standard Monday-Friday
    assert_eq!(doc.work_week.len(), 5);
    assert_eq!(doc.work_week[0], Weekday::Monday);
    assert_eq!(doc.work_week[4], Weekday::Friday);

    // Holidays
    assert!(!doc.holidays.is_empty());
    let new_years = &doc.holidays[0];
    assert_eq!(new_years.name, "New Year's Day");
    assert_eq!(new_years.date.as_deref(), Some("2026-01-01"));
    assert!(!new_years.observed);

    // Floating holiday (rule-based)
    let mlk = &doc.holidays[1];
    assert_eq!(mlk.name, "Martin Luther King Jr. Day");
    assert!(mlk.date.is_none());
    assert_eq!(mlk.rule.as_deref(), Some("nthWeekday(3, monday, january)"));

    // Observed holiday
    let july3 = doc
        .holidays
        .iter()
        .find(|h| h.observed)
        .expect("observed holiday");
    assert_eq!(july3.date.as_deref(), Some("2026-07-03"));
    assert!(july3.observed);

    // Operating hours
    let hours = doc
        .operating_hours
        .as_ref()
        .expect("operating hours present");
    assert_eq!(hours.start, "08:00");
    assert_eq!(hours.end, "17:00");

    // Effective/expiration dates
    assert_eq!(doc.effective_date.as_deref(), Some("2025-10-01"));
    assert_eq!(doc.expiration_date.as_deref(), Some("2026-09-30"));
}

#[test]
fn business_calendar_minimal_document() {
    let json = r#"{
        "$wosBusinessCalendar": "1.0",
        "targetWorkflow": "https://example.gov/test",
        "timezone": "UTC",
        "workWeek": ["monday", "tuesday", "wednesday", "thursday", "friday"]
    }"#;
    let doc: BusinessCalendarDocument = serde_json::from_str(json).unwrap();
    assert_eq!(doc.wos_business_calendar, "1.0");
    assert_eq!(doc.timezone, "UTC");
    assert_eq!(doc.work_week.len(), 5);
    assert!(doc.holidays.is_empty());
    assert!(doc.operating_hours.is_none());
    assert!(doc.effective_date.is_none());
    assert!(doc.expiration_date.is_none());
}

#[test]
fn business_calendar_serialization_round_trip() {
    let doc = load_fixture("benefits-business-calendar.json");
    let serialized = serde_json::to_string(&doc).unwrap();
    let deserialized: BusinessCalendarDocument = serde_json::from_str(&serialized).unwrap();
    assert_eq!(
        doc.wos_business_calendar,
        deserialized.wos_business_calendar
    );
    assert_eq!(doc.timezone, deserialized.timezone);
    assert_eq!(doc.work_week.len(), deserialized.work_week.len());
    assert_eq!(doc.holidays.len(), deserialized.holidays.len());
}
