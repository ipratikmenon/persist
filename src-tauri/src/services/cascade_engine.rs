// Cascade engine — generates statutory deadline chains from templates.
// specs/module-02-docketing.md §2.9
//
// The rules live in the `cascade_templates` table as JSON, never in Rust. When
// the Trade Marks Rules change a period, that is a DB update and a new
// last_verified date — not a code change and a release. This matters because a
// wrong statutory period here abandons a client's application.
//
// ANCHORS, NOT GUESSES
//
// A chain is generated from an event that has actually happened, and each
// registry-triggered event is its own anchor. The firm cannot know at filing
// when an examination report will issue, so `TMApplication` generates only what
// is computable from the filing date (renewal, grace). When the report arrives,
// `TMExaminationReport` is run as its own anchor with the report's date. Nothing
// is ever dated from an event that has not occurred.

use anyhow::{anyhow, Context, Result};
use chrono::{Datelike, NaiveDate};
use sqlx::SqlitePool;

// ---------------------------------------------------------------------------
// Template shape
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DeadlineRule {
    pub event_name: String,
    /// Statutory | Procedural
    pub event_type: String,
    pub offset: i64,
    /// days | months | years
    pub offset_unit: String,
    /// If > 0, also generate a Procedural deadline this many days earlier so the
    /// firm works to its own date rather than the registry's.
    #[serde(default)]
    pub internal_buffer_days: i64,
    #[serde(default)]
    pub client_visible: i64,
    /// Patent annuities: generate one deadline per year from `offset` through
    /// this bound inclusive.
    #[serde(default)]
    pub repeat_years: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CascadeAnchor {
    pub matter_id:   String,
    pub ip_asset_id: String,
    /// e.g. 'TMApplication', 'PatentFER'
    pub event_type:  String,
    /// The date the anchor event actually occurred (YYYY-MM-DD).
    pub anchor_date: String,
}

/// A deadline the engine proposes. Nothing is written until the attorney
/// confirms — preview and generate share this type.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposedDeadline {
    pub docketing_event:  String,
    pub event_type:       String,
    pub due_date:         String,
    pub is_client_visible: bool,
    /// True when this row is the firm's internal buffer ahead of a statutory date.
    pub is_internal_buffer: bool,
    pub notes:            Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CascadePreview {
    pub template_id:    String,
    pub anchor_event:   String,
    pub anchor_date:    String,
    /// When the statutory periods in this template were last confirmed.
    pub last_verified:  String,
    pub template_notes: Option<String>,
    pub deadlines:      Vec<ProposedDeadline>,
}

// ---------------------------------------------------------------------------
// Date arithmetic
// ---------------------------------------------------------------------------

/// Add an offset to a date. Month and year arithmetic clamps to the last valid
/// day (31 Jan + 1 month = 28/29 Feb), which is the conventional treatment for
/// statutory periods and avoids silently rolling into the following month.
pub fn add_offset(date: NaiveDate, offset: i64, unit: &str) -> Result<NaiveDate> {
    match unit {
        "days" => date
            .checked_add_signed(chrono::Duration::days(offset))
            .ok_or_else(|| anyhow!("date overflow adding {offset} days")),
        "months" => add_months(date, offset),
        "years" => add_months(date, offset.checked_mul(12).ok_or_else(|| anyhow!("year overflow"))?),
        other => Err(anyhow!("unknown offset_unit '{other}' (expected days|months|years)")),
    }
}

fn add_months(date: NaiveDate, months: i64) -> Result<NaiveDate> {
    let total = date.year() as i64 * 12 + (date.month() as i64 - 1) + months;
    let year = (total.div_euclid(12)) as i32;
    let month = (total.rem_euclid(12) + 1) as u32;

    // Clamp the day to the target month's length.
    let last_day = days_in_month(year, month);
    let day = date.day().min(last_day);

    NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| anyhow!("invalid date after adding {months} months"))
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

// ---------------------------------------------------------------------------
// Rule expansion
// ---------------------------------------------------------------------------

/// Expand one rule into the deadlines it implies, relative to the anchor date.
fn expand_rule(rule: &DeadlineRule, anchor: NaiveDate) -> Result<Vec<ProposedDeadline>> {
    let mut out = Vec::new();

    // repeat_years turns a single rule into an annual series (patent annuities).
    let occurrences: Vec<i64> = match rule.repeat_years {
        Some(until) if rule.offset_unit == "years" => {
            if until < rule.offset {
                return Err(anyhow!(
                    "repeat_years ({until}) is before the first occurrence ({})",
                    rule.offset
                ));
            }
            (rule.offset..=until).collect()
        }
        Some(_) => return Err(anyhow!("repeat_years requires offset_unit 'years'")),
        None => vec![rule.offset],
    };

    let multi = occurrences.len() > 1;

    for n in occurrences {
        let due = add_offset(anchor, n, &rule.offset_unit)?;

        let name = if multi {
            format!("{} — year {}", rule.event_name, n)
        } else {
            rule.event_name.clone()
        };

        out.push(ProposedDeadline {
            docketing_event:   name.clone(),
            event_type:        rule.event_type.clone(),
            due_date:          due.format("%Y-%m-%d").to_string(),
            is_client_visible: rule.client_visible != 0,
            is_internal_buffer: false,
            notes:             None,
        });

        // The firm's own working date, ahead of the registry's.
        if rule.internal_buffer_days > 0 {
            let buffer_due = due
                .checked_sub_signed(chrono::Duration::days(rule.internal_buffer_days))
                .ok_or_else(|| anyhow!("date underflow computing internal buffer"))?;

            out.push(ProposedDeadline {
                docketing_event:   format!("Internal: prepare {name}"),
                // Always Procedural — the buffer is the firm's discipline, not law.
                event_type:        "Procedural".to_string(),
                due_date:          buffer_due.format("%Y-%m-%d").to_string(),
                // Internal working dates are never shown to the client.
                is_client_visible: false,
                is_internal_buffer: true,
                notes: Some(format!(
                    "{} days before the statutory date of {}",
                    rule.internal_buffer_days,
                    due.format("%Y-%m-%d")
                )),
            });
        }
    }

    Ok(out)
}

/// Build the full proposal for a template's rules.
pub fn expand_template(template_json: &str, anchor_date: &str) -> Result<Vec<ProposedDeadline>> {
    let anchor = NaiveDate::parse_from_str(anchor_date, "%Y-%m-%d")
        .with_context(|| format!("anchor_date '{anchor_date}' is not YYYY-MM-DD"))?;

    let rules: Vec<DeadlineRule> =
        serde_json::from_str(template_json).context("cascade template JSON is malformed")?;

    if rules.is_empty() {
        return Err(anyhow!("cascade template contains no rules"));
    }

    let mut out = Vec::new();
    for rule in &rules {
        out.extend(expand_rule(rule, anchor)?);
    }

    // Chronological order — this is the order an attorney reads a docket in.
    out.sort_by(|a, b| a.due_date.cmp(&b.due_date));
    Ok(out)
}

// ---------------------------------------------------------------------------
// Database-facing
// ---------------------------------------------------------------------------

#[derive(Debug, sqlx::FromRow)]
struct TemplateRow {
    id:            String,
    template_json: String,
    last_verified: String,
    notes:         Option<String>,
}

/// Look up the template for an anchor and produce the proposal. Nothing is
/// written — the attorney sees the chain before it exists.
pub async fn preview(
    pool: &SqlitePool,
    anchor: &CascadeAnchor,
    ip_type: &str,
    jurisdiction: &str,
) -> Result<CascadePreview> {
    let row = sqlx::query_as::<_, TemplateRow>(
        "SELECT id, template_json, last_verified, notes
         FROM cascade_templates
         WHERE anchor_event_type = ? AND ip_type = ? AND jurisdiction = ?",
    )
    .bind(&anchor.event_type)
    .bind(ip_type)
    .bind(jurisdiction)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        anyhow!(
            "No cascade template for '{}' ({ip_type}, {jurisdiction}). \
             Add one to cascade_templates rather than entering the chain by hand.",
            anchor.event_type
        )
    })?;

    let deadlines = expand_template(&row.template_json, &anchor.anchor_date)?;

    Ok(CascadePreview {
        template_id:    row.id,
        anchor_event:   anchor.event_type.clone(),
        anchor_date:    anchor.anchor_date.clone(),
        last_verified:  row.last_verified,
        template_notes: row.notes,
        deadlines,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    // --- date arithmetic -------------------------------------------------

    #[test]
    fn adds_days_months_years() {
        assert_eq!(add_offset(d("2026-01-01"), 30, "days").unwrap(), d("2026-01-31"));
        assert_eq!(add_offset(d("2026-01-15"), 4, "months").unwrap(), d("2026-05-15"));
        assert_eq!(add_offset(d("2026-03-10"), 10, "years").unwrap(), d("2036-03-10"));
    }

    #[test]
    fn month_arithmetic_clamps_to_month_end() {
        // 31 Jan + 1 month must not roll into March.
        assert_eq!(add_offset(d("2026-01-31"), 1, "months").unwrap(), d("2026-02-28"));
        // Leap year.
        assert_eq!(add_offset(d("2028-01-31"), 1, "months").unwrap(), d("2028-02-29"));
        assert_eq!(add_offset(d("2026-03-31"), 1, "months").unwrap(), d("2026-04-30"));
    }

    #[test]
    fn year_arithmetic_handles_leap_day() {
        // 29 Feb + 1 year has no exact counterpart; clamp to 28 Feb.
        assert_eq!(add_offset(d("2028-02-29"), 1, "years").unwrap(), d("2029-02-28"));
    }

    #[test]
    fn month_arithmetic_crosses_year_boundary() {
        assert_eq!(add_offset(d("2026-11-15"), 4, "months").unwrap(), d("2027-03-15"));
        assert_eq!(add_offset(d("2026-01-15"), 126, "months").unwrap(), d("2036-07-15"));
    }

    #[test]
    fn rejects_unknown_unit() {
        assert!(add_offset(d("2026-01-01"), 1, "fortnights").is_err());
    }

    // --- rule expansion ---------------------------------------------------

    #[test]
    fn single_rule_produces_one_deadline() {
        let json = r#"[{"event_name":"Response to Examination Report",
                        "event_type":"Statutory","offset":30,"offset_unit":"days",
                        "client_visible":1}]"#;
        let out = expand_template(json, "2026-03-01").unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].docketing_event, "Response to Examination Report");
        assert_eq!(out[0].due_date, "2026-03-31");
        assert!(out[0].is_client_visible);
    }

    #[test]
    fn internal_buffer_adds_an_earlier_procedural_deadline() {
        let json = r#"[{"event_name":"Response to Examination Report",
                        "event_type":"Statutory","offset":30,"offset_unit":"days",
                        "internal_buffer_days":7,"client_visible":1}]"#;
        let out = expand_template(json, "2026-03-01").unwrap();

        assert_eq!(out.len(), 2, "expected statutory + buffer");
        // Sorted chronologically, so the buffer comes first.
        assert!(out[0].is_internal_buffer);
        assert_eq!(out[0].due_date, "2026-03-24");
        assert_eq!(out[0].event_type, "Procedural");
        // The firm's internal date must never reach the client.
        assert!(!out[0].is_client_visible, "internal buffer must stay private");

        assert!(!out[1].is_internal_buffer);
        assert_eq!(out[1].due_date, "2026-03-31");
    }

    #[test]
    fn repeat_years_generates_an_annual_series() {
        // Patent annuities: years 2 through 20 inclusive = 19 deadlines.
        let json = r#"[{"event_name":"Annuity due","event_type":"Statutory",
                        "offset":2,"offset_unit":"years","repeat_years":20,
                        "client_visible":1}]"#;
        let out = expand_template(json, "2026-06-15").unwrap();

        assert_eq!(out.len(), 19);
        assert_eq!(out[0].due_date, "2028-06-15");
        assert_eq!(out[0].docketing_event, "Annuity due — year 2");
        assert_eq!(out[18].due_date, "2046-06-15");
        assert_eq!(out[18].docketing_event, "Annuity due — year 20");
    }

    #[test]
    fn repeat_years_with_buffer_pairs_every_occurrence() {
        let json = r#"[{"event_name":"Annuity due","event_type":"Statutory",
                        "offset":2,"offset_unit":"years","repeat_years":4,
                        "internal_buffer_days":30,"client_visible":1}]"#;
        let out = expand_template(json, "2026-06-15").unwrap();
        // 3 annuities, each with a buffer.
        assert_eq!(out.len(), 6);
        assert_eq!(out.iter().filter(|p| p.is_internal_buffer).count(), 3);
    }

    #[test]
    fn repeat_years_requires_year_units() {
        let json = r#"[{"event_name":"X","event_type":"Statutory",
                        "offset":30,"offset_unit":"days","repeat_years":5}]"#;
        let err = expand_template(json, "2026-01-01").unwrap_err().to_string();
        assert!(err.contains("requires offset_unit 'years'"), "got: {err}");
    }

    #[test]
    fn repeat_years_before_first_occurrence_is_rejected() {
        let json = r#"[{"event_name":"X","event_type":"Statutory",
                        "offset":10,"offset_unit":"years","repeat_years":5}]"#;
        assert!(expand_template(json, "2026-01-01").is_err());
    }

    #[test]
    fn output_is_chronological() {
        let json = r#"[
            {"event_name":"Late","event_type":"Statutory","offset":10,"offset_unit":"years"},
            {"event_name":"Early","event_type":"Statutory","offset":30,"offset_unit":"days"},
            {"event_name":"Middle","event_type":"Statutory","offset":4,"offset_unit":"months"}
        ]"#;
        let out = expand_template(json, "2026-01-01").unwrap();
        let names: Vec<&str> = out.iter().map(|p| p.docketing_event.as_str()).collect();
        assert_eq!(names, vec!["Early", "Middle", "Late"]);
    }

    #[test]
    fn malformed_json_is_rejected() {
        assert!(expand_template("not json", "2026-01-01").is_err());
        assert!(expand_template("[]", "2026-01-01").is_err(), "empty template is an error");
    }

    #[test]
    fn bad_anchor_date_is_rejected() {
        let json = r#"[{"event_name":"X","event_type":"Statutory","offset":1,"offset_unit":"days"}]"#;
        let err = expand_template(json, "01-03-2026").unwrap_err().to_string();
        assert!(err.contains("YYYY-MM-DD"), "got: {err}");
    }

    // --- the seeded templates actually work ------------------------------

    #[test]
    fn seeded_tm_application_chain_is_correct() {
        // Mirrors tpl-tm-application-in from migration 0010.
        let json = r#"[
          {"event_name":"Expect examination report","event_type":"Procedural",
           "offset":12,"offset_unit":"months","internal_buffer_days":0,"client_visible":0},
          {"event_name":"Trademark renewal due (10-year term)","event_type":"Statutory",
           "offset":10,"offset_unit":"years","internal_buffer_days":90,"client_visible":1},
          {"event_name":"Renewal grace period expires (with surcharge)","event_type":"Statutory",
           "offset":126,"offset_unit":"months","internal_buffer_days":30,"client_visible":1}
        ]"#;
        let out = expand_template(json, "2026-04-15").unwrap();

        // 3 rules, 2 with buffers = 5 deadlines.
        assert_eq!(out.len(), 5);

        let renewal = out.iter()
            .find(|p| p.docketing_event.starts_with("Trademark renewal"))
            .expect("renewal missing");
        assert_eq!(renewal.due_date, "2036-04-15", "10 years from filing");

        let grace = out.iter()
            .find(|p| p.docketing_event.starts_with("Renewal grace"))
            .expect("grace missing");
        assert_eq!(grace.due_date, "2036-10-15", "6 months after the 10-year date");
    }
}
