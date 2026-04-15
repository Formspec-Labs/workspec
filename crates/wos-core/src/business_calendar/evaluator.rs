// Rust guideline compliant 2026-04-14

//! Business calendar SLA deadline evaluator.
//!
//! The public entry point is [`next_business_moment`], which computes the
//! deadline for `start + duration` while respecting a calendar's work week,
//! holidays, and operating hours.
//!
//! ## Algorithm (snap-forward)
//!
//! 1. Compute `naive = start + duration` in wall-clock time.
//! 2. Convert `naive` to the calendar's configured timezone.
//! 3. Snap the naive result forward to the next valid business moment:
//!    - If the day is a non-work-week day or a fixed holiday, advance to the
//!      next work day's operating-hours start.
//!    - If the time is before the day's `operating_hours.start`, snap to
//!      `operating_hours.start` on the same day.
//!    - If the time is at or after `operating_hours.end`, carry the excess
//!      time (`naive.time - op_end`) over to the next work day's
//!      `operating_hours.start + excess`, then snap again if needed.
//! 4. Return the result in UTC.

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;

use super::{BusinessCalendarDocument, Holiday, OperatingHours, Weekday};

// ── Error type ────────────────────────────────────────────────────

/// Errors returned by the business calendar evaluator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BusinessCalendarError {
    /// Snap-forward did not converge within the iteration limit.
    ///
    /// Indicates a degenerate calendar (e.g. empty `work_week` or a holiday
    /// cluster longer than one year).  Callers MUST fall back to the naive
    /// (non-calendar) deadline rather than hanging or panicking.
    DidNotConverge {
        /// The cursor position at exhaustion (UTC).
        cursor: DateTime<Utc>,
        /// Number of iterations attempted before giving up.
        iterations: u32,
    },
}

impl std::fmt::Display for BusinessCalendarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BusinessCalendarError::DidNotConverge { cursor, iterations } => write!(
                f,
                "business calendar snap-forward did not converge after {iterations} iterations \
                 (cursor at {cursor})"
            ),
        }
    }
}

impl std::error::Error for BusinessCalendarError {}

// ── Public API ────────────────────────────────────────────────────

/// Compute the SLA deadline for `start + duration` adjusted for business time.
///
/// Uses the snap-forward model described in the module documentation.
/// The result is always in UTC.
///
/// # Errors
///
/// Returns [`BusinessCalendarError::DidNotConverge`] when the snap-forward
/// algorithm exhausts its iteration budget (366 days).  This only occurs with
/// degenerate calendars — e.g. an empty `work_week` or a holiday cluster that
/// spans more than one year.  Callers should fall back to the naive deadline
/// and surface the failure to operators.
pub fn next_business_moment(
    start: DateTime<Utc>,
    duration: Duration,
    calendar: &BusinessCalendarDocument,
) -> Result<DateTime<Utc>, BusinessCalendarError> {
    if duration <= Duration::zero() {
        return Ok(start);
    }

    let tz: Tz = calendar.timezone.parse().unwrap_or(chrono_tz::UTC);
    let (op_start, op_end) = parse_operating_hours(calendar.operating_hours.as_ref());

    // Step 1: compute naive deadline by plain addition.
    let naive_utc = start + duration;

    // Step 2: convert to calendar timezone.
    let naive_local = naive_utc.with_timezone(&tz);

    // Step 3: snap forward to a valid business moment.
    let result = snap_forward(naive_local, &calendar.work_week, &calendar.holidays, op_start, op_end, &tz)?;

    // Step 4: return in UTC.
    Ok(result.with_timezone(&Utc))
}

// ── Implementation ────────────────────────────────────────────────

/// Snap `candidate` forward to the nearest valid business moment.
///
/// A valid business moment is:
/// - On a work-week day that is not a fixed holiday.
/// - At or after `op_start` and strictly before `op_end`.
///
/// When the candidate overshoots `op_end`, the excess time carries over to
/// the next business day.  The loop is bounded by 366 iterations — one full
/// year — which covers any realistic statutory holiday cluster or emergency
/// closure.  A degenerate calendar (e.g. empty `work_week`) exhausts the
/// budget and returns [`BusinessCalendarError::DidNotConverge`].
const MAX_SNAP_ITERATIONS: u32 = 366;

fn snap_forward(
    candidate: DateTime<Tz>,
    work_week: &[Weekday],
    holidays: &[Holiday],
    op_start: NaiveTime,
    op_end: NaiveTime,
    tz: &Tz,
) -> Result<DateTime<Tz>, BusinessCalendarError> {
    let mut cursor = candidate;

    for _ in 0..MAX_SNAP_ITERATIONS {
        let date = cursor.date_naive();
        let weekday = chrono_to_wos_weekday(cursor.weekday());

        // Non-work day or holiday → advance by one calendar day, preserving time.
        // This carries any excess time forward until we land on a work day.
        if !work_week.contains(&weekday) || is_fixed_holiday(date, holidays) {
            let next = date.succ_opt().unwrap_or(date);
            cursor = build_local_datetime(tz, next, cursor.time());
            continue;
        }

        let current_time = cursor.time();

        // Before operating hours → snap to op_start of the same day.
        if current_time < op_start {
            cursor = build_local_datetime(tz, date, op_start);
            return Ok(cursor);
        }

        // After operating hours → carry excess to next business day.
        if current_time >= op_end {
            let excess = current_time.signed_duration_since(op_end);
            let next = date.succ_opt().unwrap_or(date);
            let next_open = build_local_datetime(tz, next, op_start);
            // Add excess to next day's open, then snap again.
            cursor = next_open + excess;
            continue;
        }

        // Within operating hours on a work day — valid.
        return Ok(cursor);
    }

    Err(BusinessCalendarError::DidNotConverge {
        cursor: cursor.with_timezone(&Utc),
        iterations: MAX_SNAP_ITERATIONS,
    })
}

// ── Helpers ───────────────────────────────────────────────────────

/// Parse `"HH:MM"` strings from operating hours into `NaiveTime` values.
///
/// When no operating hours are configured, returns midnight-to-midnight
/// so the entire day is treated as business time.
fn parse_operating_hours(hours: Option<&OperatingHours>) -> (NaiveTime, NaiveTime) {
    let Some(hours) = hours else {
        return (
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            // 23:59:59 so there is still time within the day.
            NaiveTime::from_hms_opt(23, 59, 59).unwrap(),
        );
    };
    let start = parse_hhmm(&hours.start)
        .unwrap_or_else(|| NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    let end = parse_hhmm(&hours.end)
        .unwrap_or_else(|| NaiveTime::from_hms_opt(17, 0, 0).unwrap());
    (start, end)
}

/// Parse an `"HH:MM"` string into a `NaiveTime`.  Returns `None` on failure.
fn parse_hhmm(s: &str) -> Option<NaiveTime> {
    let (h, m) = s.split_once(':')?;
    let hour: u32 = h.parse().ok()?;
    let minute: u32 = m.parse().ok()?;
    NaiveTime::from_hms_opt(hour, minute, 0)
}

/// Build a `DateTime<Tz>` for `date` at `time` in the given timezone.
///
/// Prefers the earliest representation for ambiguous local times (DST
/// fall-back), and the latest for invalid times (DST spring-forward gap).
/// Both arms together are exhaustive for all `chrono-tz` local results, so
/// the final `expect` should never trigger in practice.
fn build_local_datetime(tz: &Tz, date: NaiveDate, time: NaiveTime) -> DateTime<Tz> {
    let naive = date.and_time(time);
    tz.from_local_datetime(&naive)
        .earliest()
        .or_else(|| tz.from_local_datetime(&naive).latest())
        .expect("chrono-tz invariant: LocalResult should produce at least one valid time")
}

/// Convert a `chrono::Weekday` to the WOS `Weekday` enum.
fn chrono_to_wos_weekday(wd: chrono::Weekday) -> Weekday {
    match wd {
        chrono::Weekday::Mon => Weekday::Monday,
        chrono::Weekday::Tue => Weekday::Tuesday,
        chrono::Weekday::Wed => Weekday::Wednesday,
        chrono::Weekday::Thu => Weekday::Thursday,
        chrono::Weekday::Fri => Weekday::Friday,
        chrono::Weekday::Sat => Weekday::Saturday,
        chrono::Weekday::Sun => Weekday::Sunday,
    }
}

/// Return `true` if `date` matches any fixed-date holiday in the list.
///
/// Rule-based holidays (`rule` fields) are not evaluated — only holidays
/// with an ISO 8601 `date` string are checked.
fn is_fixed_holiday(date: NaiveDate, holidays: &[Holiday]) -> bool {
    let date_str = date.format("%Y-%m-%d").to_string();
    holidays
        .iter()
        .any(|h| h.date.as_deref() == Some(date_str.as_str()))
}

// ── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn utc_hms(year: i32, month: u32, day: u32, hour: u32, min: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, hour, min, 0)
            .unwrap()
    }

    fn standard_calendar(tz: &str) -> BusinessCalendarDocument {
        BusinessCalendarDocument {
            wos_business_calendar: "1.0".to_string(),
            schema: None,
            target_workflow: "urn:test:calendar".to_string(),
            version: Some("test-1.0".to_string()),
            title: None,
            description: None,
            timezone: tz.to_string(),
            work_week: vec![
                Weekday::Monday,
                Weekday::Tuesday,
                Weekday::Wednesday,
                Weekday::Thursday,
                Weekday::Friday,
            ],
            holidays: vec![],
            operating_hours: Some(OperatingHours {
                start: "09:00".to_string(),
                end: "17:00".to_string(),
            }),
            effective_date: None,
            expiration_date: None,
            extensions: Default::default(),
        }
    }

    /// Duration fits within a single business day — no snapping needed.
    ///
    /// Monday 2026-03-02 10:00 UTC + 2h = 12:00 UTC (within 09:00–17:00).
    #[test]
    fn within_single_business_day() {
        let cal = standard_calendar("UTC");
        let start = utc_hms(2026, 3, 2, 10, 0); // Monday
        let result = next_business_moment(start, Duration::hours(2), &cal).unwrap();
        assert_eq!(result, utc_hms(2026, 3, 2, 12, 0));
    }

    /// Naive deadline falls on a weekend (Saturday) — time is preserved and carried to Monday.
    ///
    /// Monday 2026-03-02 10:00 UTC + 5 days = Saturday 2026-03-07 10:00 UTC.
    /// Saturday not a work day: advance to Sunday 10:00, then Monday 10:00.
    /// Monday 10:00 is within 09:00–17:00 → return Monday 10:00 UTC.
    #[test]
    fn weekend_snaps_to_monday_preserving_time() {
        let cal = standard_calendar("UTC");
        let start = utc_hms(2026, 3, 2, 10, 0); // Monday
        let result = next_business_moment(start, Duration::days(5), &cal).unwrap();
        assert_eq!(result, utc_hms(2026, 3, 9, 10, 0)); // Next Monday 10:00
    }

    /// Naive deadline falls after operating hours — carry excess to next day.
    ///
    /// Start: Friday 2026-03-06 16:00 UTC. Duration: 4h.
    /// Naive: Friday 20:00 UTC. op_end = 17:00.
    /// Excess = 20:00 - 17:00 = 3h.
    /// Next work day: Monday 2026-03-09. Deadline: 09:00 + 3h = 12:00 UTC.
    #[test]
    fn operating_hours_cutoff_carries_excess() {
        let cal = standard_calendar("UTC");
        let start = utc_hms(2026, 3, 6, 16, 0); // Friday
        let result = next_business_moment(start, Duration::hours(4), &cal).unwrap();
        assert_eq!(result, utc_hms(2026, 3, 9, 12, 0)); // Monday 12:00
    }

    /// A fixed holiday on the naive-deadline day causes a skip to the next work day.
    ///
    /// Start: Friday 2026-03-06 09:00 UTC. Duration: 3 days.
    /// Naive: Monday 2026-03-09 09:00 UTC. Monday is a fixed holiday.
    /// Snap to Tuesday 2026-03-10 09:00 UTC.
    #[test]
    fn holiday_on_naive_day_skips_to_tuesday() {
        let mut cal = standard_calendar("UTC");
        cal.holidays = vec![Holiday {
            name: "Test Holiday".to_string(),
            date: Some("2026-03-09".to_string()),
            rule: None,
            observed: false,
        }];
        let start = utc_hms(2026, 3, 6, 9, 0); // Friday 09:00
        let result = next_business_moment(start, Duration::days(3), &cal).unwrap();
        assert_eq!(result, utc_hms(2026, 3, 10, 9, 0)); // Tuesday 09:00
    }

    /// Timezone-aware: New York calendar correctly uses EDT offset after DST.
    ///
    /// Start: Saturday 2026-04-04 14:00 UTC = Saturday 10:00 EDT (UTC-4).
    /// Duration: 1 day. Naive UTC: Sunday 2026-04-05 14:00 = Sunday 10:00 EDT.
    /// Sunday not a work day → Monday 10:00 EDT = Monday 14:00 UTC.
    /// Monday 10:00 EDT within 09:00–17:00 → result is Monday 14:00 UTC.
    #[test]
    fn timezone_new_york_edt() {
        let cal = standard_calendar("America/New_York");
        let start = utc_hms(2026, 4, 4, 14, 0); // Saturday 10:00 EDT
        let result = next_business_moment(start, Duration::days(1), &cal).unwrap();
        // Sunday 10:00 EDT → Monday 10:00 EDT = Monday 14:00 UTC.
        assert_eq!(result, utc_hms(2026, 4, 6, 14, 0));
    }

    /// When operating hours are absent, the full day (midnight-to-23:59:59) counts.
    ///
    /// Monday 2026-03-02 10:00 + 2h = 12:00 — fits within the day, no change.
    #[test]
    fn no_operating_hours_uses_full_day() {
        let mut cal = standard_calendar("UTC");
        cal.operating_hours = None;
        let start = utc_hms(2026, 3, 2, 10, 0); // Monday
        let result = next_business_moment(start, Duration::hours(2), &cal).unwrap();
        assert_eq!(result, utc_hms(2026, 3, 2, 12, 0));
    }

    /// A calendar with an empty work_week can never find a valid business day.
    ///
    /// Every day is a non-work day, so snap_forward exhausts the 366-iteration
    /// budget and returns `Err(DidNotConverge { .. })`.
    #[test]
    fn empty_work_week_returns_did_not_converge() {
        let mut cal = standard_calendar("UTC");
        cal.work_week = vec![]; // no valid work days
        let start = utc_hms(2026, 3, 2, 10, 0);
        let result = next_business_moment(start, Duration::hours(2), &cal);
        assert!(
            matches!(
                result,
                Err(BusinessCalendarError::DidNotConverge { iterations: 366, .. })
            ),
            "expected DidNotConverge, got {result:?}"
        );
    }
}
