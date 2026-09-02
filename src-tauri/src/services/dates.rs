// Dates, written the way an Indian legal document writes them.
//
// One implementation, for the same reason money has one: a notice dated "14th
// July 2026" over a schedule of payments dated "2026-02-11" is one document in
// two registers, and the schedule reads like a spreadsheet somebody pasted in.

use chrono::{Datelike, NaiveDate};

/// The English ordinal suffix for a day of the month.
///
/// The teens are the whole difficulty: 11, 12 and 13 take "th" even though they
/// end in 1, 2 and 3. A rule written only on the last digit prints "11st".
pub fn ordinal_suffix(day: u32) -> &'static str {
    match (day % 100, day % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    }
}

/// Month names in full, as correspondence sets them.
///
/// Spelled out here rather than taken from chrono's `%B` formatting, which is
/// locale-independent today but is a formatting concern rather than a promise.
pub fn month_name(month: u32) -> &'static str {
    const MONTHS: [&str; 12] = [
        "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December",
    ];
    MONTHS.get((month as usize).saturating_sub(1)).copied().unwrap_or("")
}


/// "11 February 2026" — a date set in running text or in a table cell.
///
/// No superscripted ordinal: it is right in a dateline, where the eye rests on
/// it, and fussy in a narrow column of a schedule. `notice_date` in
/// services/firm.rs is the superscripted form for the dateline.
pub fn long_date(date: NaiveDate) -> String {
    format!("{} {} {}", date.day(), month_name(date.month()), date.year())
}

/// The same, from the ISO string a `date` field holds, or `None` if it is not
/// one. A cell the attorney left half-typed prints as typed rather than
/// failing — validation is where a malformed date is refused.
pub fn long_date_from_iso(raw: &str) -> Option<String> {
    NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d").ok().map(long_date)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn a_date_in_a_table_reads_as_a_date_and_not_as_a_field() {
        assert_eq!(long_date(day(2026, 2, 11)), "11 February 2026");
        assert_eq!(long_date(day(2026, 12, 1)), "1 December 2026");
    }

    /// The teens are the whole difficulty — a rule written on the last digit
    /// prints "11st".
    #[test]
    fn the_ordinal_is_right_on_the_days_that_are_not_th() {
        let expected = [
            (1, "st"), (2, "nd"), (3, "rd"), (4, "th"),
            (11, "th"), (12, "th"), (13, "th"),
            (21, "st"), (22, "nd"), (23, "rd"), (31, "st"),
        ];
        for (d, suffix) in expected {
            assert_eq!(ordinal_suffix(d), suffix, "day {d}");
        }
    }

    #[test]
    fn every_month_has_a_name() {
        for m in 1..=12 {
            assert!(!month_name(m).is_empty(), "month {m}");
        }
    }

    #[test]
    fn something_that_is_not_a_date_is_left_as_it_was() {
        assert_eq!(long_date_from_iso("2026-02-11").as_deref(), Some("11 February 2026"));
        assert_eq!(long_date_from_iso("2026-02"), None);
        assert_eq!(long_date_from_iso(""), None);
        assert_eq!(long_date_from_iso("not a date"), None);
    }
}
