//! Test-only date helpers (frozen-date arithmetic).
//!
//! Production rules use `chrono::Utc::now()` directly. These helpers exist
//! so test fixtures can compute "today + N days" against a **frozen**
//! `today` constant, producing byte-identical date strings regardless of
//! when CI runs. This eliminates timezone-boundary flakes in tests for
//! COMP-LINT-002, EFF-LINT-005, CHAIN-LINT-002, etc.
//!
//! Visibility: `pub(crate)` — re-exported under `#[cfg(test)]` only.

/// Convert days-since-epoch (1970-01-01 = 0) to `(year, month, day)` in
/// the proleptic Gregorian calendar.
///
/// Algorithm: Howard Hinnant's "civil_from_days" — handles BC dates,
/// 4/100/400-year leap rules correctly, valid for the entire range of
/// `i64` days (well past year 9999).
pub(crate) fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z / 146097 } else { (z - 146096) / 146097 };
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y_adj = if m <= 2 { y + 1 } else { y };
    (y_adj, m, d)
}

/// Inverse of [`days_to_ymd`]: convert `(year, month, day)` to
/// days-since-epoch. Used by [`iso_date_offset_from`] to walk a frozen
/// today by a signed day offset.
pub(crate) fn ymd_to_days(y: i64, m: u32, d: u32) -> i64 {
    let m = m as i64;
    let d = d as i64;
    let y_adj = if m <= 2 { y - 1 } else { y };
    let era = if y_adj >= 0 { y_adj / 400 } else { (y_adj - 399) / 400 };
    let yoe = (y_adj - era * 400) as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Take a frozen `today` (e.g., `"2026-05-02"`) and return the ISO date
/// string for `today + days` (negative offsets allowed). Tests use this
/// to express "60 days from now" without calling `SystemTime::now()`,
/// guaranteeing byte-identical output across CI runs and timezones.
///
/// Panics on malformed input (test-only helper; the panics document
/// fixture authoring bugs, not runtime behavior).
pub(crate) fn iso_date_offset_from(today: &str, days: i64) -> String {
    let parts: Vec<&str> = today.split('-').collect();
    assert_eq!(parts.len(), 3, "frozen today MUST be yyyy-MM-dd: {today:?}");
    let y: i64 = parts[0].parse().expect("year parse");
    let m: u32 = parts[1].parse().expect("month parse");
    let d: u32 = parts[2].parse().expect("day parse");
    let base = ymd_to_days(y, m, d);
    let (yy, mm, dd) = days_to_ymd(base + days);
    format!("{yy:04}-{mm:02}-{dd:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_boundary() {
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
    }

    #[test]
    fn leap_year_2024_feb_29() {
        // 2024-02-29 is valid (leap year).
        let days = ymd_to_days(2024, 2, 29);
        assert_eq!(days_to_ymd(days), (2024, 2, 29));
        // Day after 2024-02-29 is 2024-03-01.
        assert_eq!(days_to_ymd(days + 1), (2024, 3, 1));
    }

    #[test]
    fn non_leap_year_2025_february_boundary() {
        // 2025-02-28 → 2025-03-01 (no Feb 29 in non-leap year).
        let feb_28 = ymd_to_days(2025, 2, 28);
        assert_eq!(days_to_ymd(feb_28 + 1), (2025, 3, 1));
    }

    #[test]
    fn century_2100_is_not_leap() {
        // 2100 is divisible by 4 but NOT by 400, so it's NOT a leap year.
        let feb_28 = ymd_to_days(2100, 2, 28);
        assert_eq!(days_to_ymd(feb_28 + 1), (2100, 3, 1));
    }

    #[test]
    fn quad_century_2000_is_leap() {
        // 2000 is divisible by 400, so it IS a leap year.
        let feb_29 = ymd_to_days(2000, 2, 29);
        assert_eq!(days_to_ymd(feb_29), (2000, 2, 29));
    }

    #[test]
    fn far_future_2100_dec_31_to_2101_jan_1() {
        let dec_31 = ymd_to_days(2100, 12, 31);
        assert_eq!(days_to_ymd(dec_31), (2100, 12, 31));
        assert_eq!(days_to_ymd(dec_31 + 1), (2101, 1, 1));
    }

    #[test]
    fn far_future_2200_jan_1() {
        let jan_1 = ymd_to_days(2200, 1, 1);
        assert_eq!(days_to_ymd(jan_1), (2200, 1, 1));
    }

    #[test]
    fn iso_offset_walks_forward_into_next_month() {
        // 2026-05-02 + 30 days → 2026-06-01.
        assert_eq!(iso_date_offset_from("2026-05-02", 30), "2026-06-01");
    }

    #[test]
    fn iso_offset_walks_backward_into_previous_month() {
        // 2026-05-02 - 5 days → 2026-04-27.
        assert_eq!(iso_date_offset_from("2026-05-02", -5), "2026-04-27");
    }

    #[test]
    fn iso_offset_crosses_year_boundary() {
        // 2026-12-31 + 1 day → 2027-01-01.
        assert_eq!(iso_date_offset_from("2026-12-31", 1), "2027-01-01");
        // 2027-01-01 - 1 day → 2026-12-31.
        assert_eq!(iso_date_offset_from("2027-01-01", -1), "2026-12-31");
    }

    #[test]
    fn iso_offset_zero_returns_today() {
        assert_eq!(iso_date_offset_from("2026-05-02", 0), "2026-05-02");
    }
}
