// The firm's own identity on a document it produces.
//
// WHY THIS EXISTS
//
// The Legal Notice template declares its letterhead as `computed` fields —
// PARTNER_ONE_NAME, FIRM_OFFICE_LINE_TWO, NOTICE_DATE, SIGNATORY_BLOCK and the
// rest. `computed` means Keel assembles it. Nothing did. Every declared field
// that nobody supplies is bound to an empty string by the form compiler, so a
// notice rendered today came out with a blank header, an empty footer, no date
// and no signature: a document the firm could not send. This module is the
// missing half of that contract.
//
// WHY IT IS A SERVICE AND NOT PART OF THE DRAFTING COMMAND
//
// The letterhead is not drafting. It is the same on every instrument the firm
// issues, it changes when the firm moves office rather than when an attorney
// types, and it has rules of its own — the ordinal in an Indian date, which
// partner signs, what happens when a partner has no enrolment number yet. Those
// rules are worth testing on their own, against a database, without compiling a
// document.
//
// ESCAPING
//
// Every value here comes out of SQLite, which means it is text somebody typed
// into a settings form, which means it is untrusted. It reaches LaTeX as
// `Field::text` — escaped — without exception. The firm's own name contains an
// ampersand, and that is exactly the character that broke every invoice the
// last time escaping was left to the caller.
//
// The three assembled blocks are `Field::raw`, because this module builds their
// markup: a line break between address lines, a superscript on an ordinal. Every
// value interpolated into them goes through `latex::escape` first, and the
// markup around them is a fixed string written here.

use crate::db::queries::billing::{self as bq, FirmPartnerRow};
use crate::services::latex::{self, Field};
use crate::services::templates::{FieldKind, TemplateManifest};
use crate::SessionData;
use anyhow::{bail, Result};
use chrono::{Datelike, NaiveDate};
use sqlx::SqlitePool;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Letterhead
// ---------------------------------------------------------------------------

/// The firm's identity as the letterhead prints it.
///
/// Partners fill the numbered slots in the order the roster gives them, so the
/// template decides how many partners it has room for and this decides who they
/// are. A template with a third slot needs no change here.
pub async fn letterhead_fields(pool: &SqlitePool) -> Result<HashMap<String, Field>> {
    let settings = bq::get_firm_settings(pool).await?;
    let partners = bq::list_firm_partners(pool).await?;

    let mut fields = HashMap::new();

    // A missing setting is an empty line on the letterhead rather than a failed
    // render. The office moving and nobody having typed the new address yet is
    // not a reason an attorney cannot serve a notice today.
    let mut put = |key: &str, value: Option<&str>| {
        fields.insert(key.to_owned(), Field::text(value.unwrap_or("")));
    };
    put("FIRM_WEBSITE", settings.firm_website.as_deref());
    put("FIRM_CONTACT", settings.firm_contact_email.as_deref());
    put("FIRM_OFFICE_LINE_ONE", settings.firm_office_line_one.as_deref());
    put("FIRM_OFFICE_LINE_TWO", settings.firm_office_line_two.as_deref());

    for (slot, partner) in PARTNER_SLOTS.iter().zip(partners.iter()) {
        fields.insert(format!("PARTNER_{slot}_NAME"), Field::text(&partner.name));
        fields.insert(format!("PARTNER_{slot}_ROLE"), Field::text(&partner.role));
        fields.insert(
            format!("PARTNER_{slot}_PHONE"),
            Field::text(partner.phone.as_deref().unwrap_or("")),
        );
        fields.insert(
            format!("PARTNER_{slot}_EMAIL"),
            Field::text(partner.email.as_deref().unwrap_or("")),
        );
    }

    Ok(fields)
}

/// The slot names the templates use, in letterhead order.
const PARTNER_SLOTS: [&str; 4] = ["ONE", "TWO", "THREE", "FOUR"];

// ---------------------------------------------------------------------------
// The date on a letter
// ---------------------------------------------------------------------------

/// "14\textsuperscript{th} July 2026".
///
/// Indian correspondence carries the ordinal. A bare 14/07/2026 reads as a form
/// rather than a letter, and on a notice — a document whose date decides when a
/// limitation period starts running — the difference is not cosmetic.
///
/// `raw` because the superscript is markup. Nothing untrusted reaches it: the
/// day and year are integers and the month name comes from chrono's own table.
pub fn notice_date(date: NaiveDate) -> Field {
    Field::raw(format!(
        "{}\\textsuperscript{{{}}} {} {}",
        date.day(),
        ordinal_suffix(date.day()),
        month_name(date.month()),
        date.year()
    ))
}

/// The English ordinal suffix for a day of the month.
///
/// The teens are the whole difficulty: 11, 12 and 13 take "th" even though they
/// end in 1, 2 and 3. A rule written only on the last digit prints "11st".
fn ordinal_suffix(day: u32) -> &'static str {
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
fn month_name(month: u32) -> &'static str {
    const MONTHS: [&str; 12] = [
        "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December",
    ];
    MONTHS.get((month as usize).saturating_sub(1)).copied().unwrap_or("")
}

// ---------------------------------------------------------------------------
// Blocks
// ---------------------------------------------------------------------------

/// Who signed, with the standing to sign it.
///
/// ```text
/// Sree Lakshmi Menon\\
/// D/6361/2020\\
/// Advocates
/// ```
///
/// The enrolment number is on the block because a notice served without one
/// invites a challenge to the signatory's standing. A partner who has not had
/// theirs entered yet signs without it rather than not at all — the line is
/// simply absent.
pub async fn signatory_block(
    pool: &SqlitePool,
    signed_in: Option<&SessionData>,
) -> Result<Field> {
    let partners = bq::list_firm_partners(pool).await?;
    let partner = choose_signatory(&partners, signed_in)?;

    let mut lines = vec![latex::escape(&partner.name)];
    if let Some(enrolment) = partner.enrolment_number.as_deref().filter(|e| !e.trim().is_empty()) {
        lines.push(latex::escape(enrolment));
    }
    lines.push("Advocates".to_owned());

    Ok(Field::raw(lines.join("\\\\\n")))
}

/// The partner whose name goes under the signature.
///
/// The document is signed by whoever generated it, where that person is a
/// partner. Matching is by `user_id` first — the link the settings screen
/// maintains — and then by name compared with case and spacing ignored, because
/// the letterhead spells it "Sreelakshmi Menon" and the login spells it "Sree
/// Lakshmi Menon" and they are the same person.
///
/// Falling back to the senior partner is deliberate. An associate may draft a
/// notice; the firm still serves it over a partner's signature, and printing an
/// associate's name under "Advocates" would misstate who is answerable for it.
fn choose_signatory<'a>(
    partners: &'a [FirmPartnerRow],
    signed_in: Option<&SessionData>,
) -> Result<&'a FirmPartnerRow> {
    if let Some(session) = signed_in {
        let matched = partners
            .iter()
            .find(|p| p.user_id.as_deref() == Some(session.user_id.as_str()))
            .or_else(|| {
                partners.iter().find(|p| same_person(&p.name, &session.name))
            });
        if let Some(partner) = matched {
            return Ok(partner);
        }
    }

    partners.first().ok_or_else(|| {
        anyhow::anyhow!(
            "The firm's letterhead has no partners. Add them under Billing → Firm \
             Settings before generating this document."
        )
    })
}

/// Two spellings of one attorney's name.
fn same_person(one: &str, other: &str) -> bool {
    let squash = |s: &str| {
        s.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    squash(one) == squash(other)
}

/// A typed address as LaTeX lines.
///
/// The attorney types an address into a box, one line per line, and the notice
/// has to print it that way — an addressee block run together into a paragraph
/// is not an address. Each line is escaped before the line break is put between
/// them, so a recipient at "Flat 3, Ram & Co." is addressed rather than fatal.
fn lines_block(text: &str) -> Field {
    let lines: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(latex::escape)
        .collect();
    Field::raw(lines.join("\\\\\n"))
}

// ---------------------------------------------------------------------------
// The entry point the drafting command uses
// ---------------------------------------------------------------------------

/// Everything on a document that does not come from the form: the letterhead,
/// the date it carries, who signs it, and the addressee block.
///
/// Only keys the manifest declares as `computed` are returned. A template that
/// does not ask for a letterhead does not get one, and this can never introduce
/// a key the template never declared.
///
/// `today` is a parameter rather than read from the clock here so that the date
/// on a document is a fact the caller states and a test can state too.
pub async fn document_fields(
    pool: &SqlitePool,
    manifest: &TemplateManifest,
    values: &HashMap<String, String>,
    signed_in: Option<&SessionData>,
    today: NaiveDate,
) -> Result<HashMap<String, Field>> {
    let computed: std::collections::HashSet<&str> = manifest
        .fields
        .iter()
        .filter(|spec| matches!(spec.kind, FieldKind::Computed))
        .map(|spec| spec.key.as_str())
        .collect();

    let mut assembled = HashMap::new();

    for (key, value) in letterhead_fields(pool).await? {
        if computed.contains(key.as_str()) {
            assembled.insert(key, value);
        }
    }

    if computed.contains("NOTICE_DATE") {
        assembled.insert("NOTICE_DATE".to_owned(), notice_date(today));
    }

    if computed.contains("SIGNATORY_BLOCK") {
        assembled.insert("SIGNATORY_BLOCK".to_owned(), signatory_block(pool, signed_in).await?);
    }

    if computed.contains("RECIPIENT_ADDRESS_BLOCK") {
        let typed = values.get("RECIPIENT_ADDRESS").map(String::as_str).unwrap_or("");
        assembled.insert("RECIPIENT_ADDRESS_BLOCK".to_owned(), lines_block(typed));
    }

    // A letterhead with nobody on it is not a letterhead. Refused here rather
    // than at the engine, which would happily compile a blank header.
    if computed.contains("PARTNER_ONE_NAME")
        && assembled.get("PARTNER_ONE_NAME").map(|f| f.as_str().is_empty()).unwrap_or(true)
    {
        bail!(
            "The firm's letterhead has no partners. Add them under Billing → Firm \
             Settings before generating this document."
        );
    }

    Ok(assembled)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::templates::FieldSpec;

    /// The real migration set against an empty database — the same schema, and
    /// the same seeded partners, an attorney's machine has at first launch.
    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("src/db/migrations").run(&pool).await.unwrap();
        pool
    }

    fn session(user_id: &str, name: &str) -> SessionData {
        SessionData {
            session_id: "s-1".into(),
            user_id: user_id.into(),
            name: name.into(),
            role: "Partner".into(),
        }
    }

    fn manifest(keys: &[&str]) -> TemplateManifest {
        TemplateManifest {
            id: "t".into(),
            name: "T".into(),
            category: "C".into(),
            version: 1,
            revised: "2026-08-11".into(),
            approved_by: None,
            authority: None,
            description: None,
            fields: keys
                .iter()
                .map(|key| FieldSpec {
                    key: (*key).to_owned(),
                    label: (*key).to_owned(),
                    kind: FieldKind::Computed,
                    required: false,
                    help: None,
                    autofill: None,
                    shown_when: None,
                    input_only: false,
                })
                .collect(),
        }
    }

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    // -- the date ----------------------------------------------------------

    #[test]
    fn the_ordinal_is_right_on_every_day_that_is_not_th() {
        for (d, want) in [
            (1, "1st"), (2, "2nd"), (3, "3rd"), (4, "4th"),
            (11, "11th"), (12, "12th"), (13, "13th"),
            (21, "21st"), (22, "22nd"), (23, "23rd"),
            (30, "30th"), (31, "31st"),
        ] {
            let field = notice_date(day(2026, 7, d));
            assert!(
                field.as_str().starts_with(&format!(
                    "{d}\\textsuperscript{{{}}}",
                    &want[want.len() - 2..]
                )),
                "day {d} should read {want}, got {}",
                field.as_str()
            );
        }
    }

    #[test]
    fn the_date_reads_as_correspondence_not_as_a_form() {
        assert_eq!(
            notice_date(day(2026, 7, 14)).as_str(),
            "14\\textsuperscript{th} July 2026"
        );
    }

    #[test]
    fn every_month_has_a_name() {
        for month in 1..=12 {
            let field = notice_date(day(2026, month, 1));
            assert!(
                field.as_str().split(' ').nth(1).map(|m| !m.is_empty()).unwrap_or(false),
                "month {month} printed nothing: {}",
                field.as_str()
            );
        }
    }

    // -- the letterhead ----------------------------------------------------

    #[tokio::test]
    async fn a_fresh_install_has_a_letterhead_with_both_partners_on_it() {
        let pool = test_pool().await;
        let fields = letterhead_fields(&pool).await.unwrap();

        assert_eq!(fields["PARTNER_ONE_NAME"].as_str(), "Sreelakshmi Menon");
        assert_eq!(fields["PARTNER_ONE_PHONE"].as_str(), "+91 99535 31789");
        assert_eq!(fields["PARTNER_TWO_NAME"].as_str(), "Kajal Thakur");
        assert_eq!(fields["PARTNER_TWO_PHONE"].as_str(), "+91 93153 67642");
        assert_eq!(fields["FIRM_WEBSITE"].as_str(), "www.persistas.com");
        assert_eq!(fields["FIRM_CONTACT"].as_str(), "persistas.pnp@outlook.com");
        assert_eq!(
            fields["FIRM_OFFICE_LINE_TWO"].as_str(),
            "Mayur Vihar Phase-III, Delhi - 110096"
        );
    }

    /// The role on the letterhead is "Advocate & Partner". An unescaped
    /// ampersand is a column separator in LaTeX and it is what broke every
    /// invoice the last time a field was passed through unescaped.
    #[tokio::test]
    async fn an_ampersand_from_the_database_reaches_latex_escaped() {
        let pool = test_pool().await;
        let fields = letterhead_fields(&pool).await.unwrap();

        assert_eq!(fields["PARTNER_ONE_ROLE"].as_str(), r"Advocate \& Partner");
        assert!(!fields["PARTNER_ONE_ROLE"].as_str().contains(" & "));
    }

    #[tokio::test]
    async fn a_firm_name_typed_with_latex_in_it_cannot_become_markup() {
        let pool = test_pool().await;
        sqlx::query("UPDATE firm_settings SET firm_website = ? WHERE id = 1")
            .bind(r"\input{/etc/passwd} & 100%")
            .execute(&pool)
            .await
            .unwrap();

        let fields = letterhead_fields(&pool).await.unwrap();
        let website = fields["FIRM_WEBSITE"].as_str();
        assert!(!website.contains(r"\input{"), "letterhead is injectable: {website}");
        assert!(website.contains(r"\textbackslash{}"), "{website}");
        assert!(website.contains(r"\&") && website.contains(r"\%"), "{website}");
    }

    #[tokio::test]
    async fn a_third_partner_takes_the_third_slot_without_a_schema_change() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO firm_partners (id, name, role, phone, email, sort_order)
             VALUES ('p-3', 'A New Partner', 'Advocate', '+91 90000 00000', 'new@x.in', 3)"
        )
        .execute(&pool)
        .await
        .unwrap();

        let fields = letterhead_fields(&pool).await.unwrap();
        assert_eq!(fields["PARTNER_THREE_NAME"].as_str(), "A New Partner");
    }

    #[tokio::test]
    async fn a_partner_who_has_left_stops_appearing_on_documents() {
        let pool = test_pool().await;
        sqlx::query("UPDATE firm_partners SET is_active = 0 WHERE id = 'partner-kt'")
            .execute(&pool)
            .await
            .unwrap();

        let fields = letterhead_fields(&pool).await.unwrap();
        assert!(!fields.contains_key("PARTNER_TWO_NAME"), "a departed partner still prints");
        assert_eq!(fields["PARTNER_ONE_NAME"].as_str(), "Sreelakshmi Menon");
    }

    // -- the signature -----------------------------------------------------

    #[tokio::test]
    async fn the_signature_carries_the_enrolment_number() {
        let pool = test_pool().await;
        let block = signatory_block(&pool, None).await.unwrap();
        assert_eq!(block.as_str(), "Sreelakshmi Menon\\\\\nD/6361/2020\\\\\nAdvocates");
    }

    /// The letterhead and the login spell the same attorney's name differently.
    /// Nothing links the rows on a fresh install — users are seeded after the
    /// migrations run — so the name is all there is to go on.
    #[tokio::test]
    async fn the_signatory_is_the_attorney_who_generated_the_document() {
        let pool = test_pool().await;
        let block = signatory_block(&pool, Some(&session("user-kt", "Kajal Thakur")))
            .await
            .unwrap();
        assert!(block.as_str().starts_with("Kajal Thakur"), "{}", block.as_str());
        // No enrolment number on record: she signs without the line rather than
        // not at all.
        assert_eq!(block.as_str(), "Kajal Thakur\\\\\nAdvocates");
    }

    #[tokio::test]
    async fn a_login_linked_to_a_partner_row_is_matched_on_the_link_not_the_name() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO users (id, name, email, role, password_hash)
             VALUES ('u-1', 'Nothing Like The Letterhead', 'u1@persist.in', 'Partner', 'x')"
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("UPDATE firm_partners SET user_id = 'u-1' WHERE id = 'partner-kt'")
            .execute(&pool)
            .await
            .unwrap();

        let block = signatory_block(
            &pool,
            Some(&session("u-1", "Nothing Like The Letterhead")),
        )
        .await
        .unwrap();
        assert!(block.as_str().starts_with("Kajal Thakur"), "{}", block.as_str());
    }

    /// An associate may draft the notice; the firm serves it over a partner's
    /// signature.
    #[tokio::test]
    async fn a_drafter_who_is_not_a_partner_signs_over_the_senior_partner() {
        let pool = test_pool().await;
        let block = signatory_block(&pool, Some(&session("user-x", "An Associate")))
            .await
            .unwrap();
        assert!(block.as_str().starts_with("Sreelakshmi Menon"), "{}", block.as_str());
    }

    #[tokio::test]
    async fn a_name_in_a_signature_is_escaped_like_any_other_text() {
        let pool = test_pool().await;
        sqlx::query("UPDATE firm_partners SET name = ? WHERE id = 'partner-slm'")
            .bind("Menon & Co_")
            .execute(&pool)
            .await
            .unwrap();

        let block = signatory_block(&pool, None).await.unwrap();
        assert!(block.as_str().starts_with(r"Menon \& Co\_"), "{}", block.as_str());
    }

    // -- the addressee -----------------------------------------------------

    #[test]
    fn an_address_keeps_its_lines_and_loses_its_special_characters() {
        let block = lines_block("Flat 3, Ram & Co.\nLucknow - 226003\n");
        assert_eq!(block.as_str(), "Flat 3, Ram \\& Co.\\\\\nLucknow - 226003");
    }

    #[test]
    fn a_blank_line_in_a_typed_address_does_not_become_a_blank_line_on_the_page() {
        // A stray newline in a textarea would otherwise open a new paragraph
        // mid-address.
        let block = lines_block("House No. 460/21,\n\n\nLucknow -- 226003");
        assert_eq!(block.as_str(), "House No. 460/21,\\\\\nLucknow -- 226003");
    }

    // -- what the drafting command gets ------------------------------------

    #[tokio::test]
    async fn only_the_computed_keys_a_template_declares_come_back() {
        let pool = test_pool().await;
        let m = manifest(&["PARTNER_ONE_NAME", "NOTICE_DATE"]);

        let fields =
            document_fields(&pool, &m, &HashMap::new(), None, day(2026, 7, 14)).await.unwrap();

        assert_eq!(fields.len(), 2, "{fields:?}");
        assert!(fields.contains_key("PARTNER_ONE_NAME"));
        assert!(fields.contains_key("NOTICE_DATE"));
        assert!(!fields.contains_key("FIRM_WEBSITE"), "a key the template never declared");
    }

    #[tokio::test]
    async fn a_template_with_no_letterhead_is_given_none() {
        let pool = test_pool().await;
        let m = manifest(&["LINE_ITEMS_TABLE"]);

        let fields =
            document_fields(&pool, &m, &HashMap::new(), None, day(2026, 7, 14)).await.unwrap();
        assert!(fields.is_empty(), "{fields:?}");
    }

    #[tokio::test]
    async fn the_addressee_block_is_built_from_what_the_attorney_typed() {
        let pool = test_pool().await;
        let m = manifest(&["RECIPIENT_ADDRESS_BLOCK"]);
        let values: HashMap<String, String> = [(
            "RECIPIENT_ADDRESS".to_owned(),
            "House No. 460/21,\nLucknow -- 226003".to_owned(),
        )]
        .into_iter()
        .collect();

        let fields = document_fields(&pool, &m, &values, None, day(2026, 7, 14)).await.unwrap();
        assert_eq!(
            fields["RECIPIENT_ADDRESS_BLOCK"].as_str(),
            "House No. 460/21,\\\\\nLucknow -- 226003"
        );
    }

    /// A blank letterhead is worse than a refusal: the notice looks finished and
    /// is unusable, and nobody notices until it has been served.
    #[tokio::test]
    async fn a_firm_with_no_partners_refuses_to_produce_a_letterhead() {
        let pool = test_pool().await;
        sqlx::query("UPDATE firm_partners SET is_active = 0").execute(&pool).await.unwrap();

        let m = manifest(&["PARTNER_ONE_NAME", "SIGNATORY_BLOCK"]);
        let error = document_fields(&pool, &m, &HashMap::new(), None, day(2026, 7, 14))
            .await
            .expect_err("a letterhead with nobody on it must not render");

        assert!(error.to_string().contains("no partners"), "{error}");
    }
}

// ---------------------------------------------------------------------------
// Compile tests
//
// A string assertion cannot tell you whether the firm's name reached the page.
// These render the real Legal Notice against the real template with the fields
// this module assembles, and read the result back out of the PDF. The failure
// they exist to catch is the one this module was written for: a notice that
// compiles cleanly and comes out with a blank letterhead.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod compile_tests {
    use super::*;
    use crate::services::latex::CompileMode;

    /// The schema and the seeded letterhead a new install starts with.
    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("src/db/migrations").run(&pool).await.unwrap();
        pool
    }

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    /// What the attorney fills in on the form, from the firm's own notice.
    fn typed_values() -> HashMap<String, String> {
        [
            ("RECIPIENT_NAME", "Mr. Mohammed Danish"),
            ("RECIPIENT_ADDRESS", "House No. 460/21,\nLucknow -- 226003"),
            ("MODE_OF_SERVICE", "THROUGH SPEED POST/ WHATSAPP"),
            ("SUBJECT", "DEMAND FOR REFUND OF ₹1,04,000/- WITH INTEREST"),
            ("SALUTATION", "Sir"),
            ("CLIENT_NAME", "Mr. Nikhil Prabhakar"),
            ("CLIENT_DESCRIPTION", "son of P. Prabhakaran"),
            ("CLIENT_ADDRESS", "A-004, Mangal Apartment, New Delhi-110096"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
    }

    /// The whole render path a notice takes, minus the Tauri command around it:
    /// declared fields bound from the form, computed fields from this module,
    /// and the section and annexure blocks their own assemblers build.
    async fn render_notice(pool: &SqlitePool, signed_in: Option<&SessionData>) -> Vec<u8> {
        let dir = latex::templates_dir().expect("the shipped template library");
        let manifest = crate::services::templates::get(&dir, "legal-notice").unwrap();
        let values = typed_values();

        let mut fields: HashMap<String, Field> = manifest
            .fields
            .iter()
            .filter(|spec| !spec.input_only)
            .map(|spec| {
                let raw = values.get(&spec.key).map(String::as_str).unwrap_or("");
                (spec.key.clone(), Field::text(raw))
            })
            .collect();

        let firm = document_fields(pool, &manifest, &values, signed_in, day(2026, 7, 14))
            .await
            .expect("the firm's identity must assemble");
        fields.extend(firm);

        // Not this module's to build; supplied so the document is whole.
        fields.insert(
            "SECTIONS_BLOCK".into(),
            Field::raw(
                "\\noticesection{1}{Background}\n\\begin{noticebody}\n\
                 That you received a sum from my client and have not repaid it.\n\
                 \\end{noticebody}\n",
            ),
        );
        fields.insert("ANNEXURES_BLOCK".into(), Field::raw(""));
        fields.insert("ANNEXURE_PAGES".into(), Field::raw(""));

        latex::compile_with("legal-notice", &fields, CompileMode::Final, &[])
            .await
            .expect("the notice must compile")
    }

    /// The page as one line of words. Line breaks in the letterhead are a
    /// layout decision, and an assertion should not depend on where the
    /// partner block happened to wrap.
    fn pdf_text(pdf: &[u8]) -> String {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.pdf");
        std::fs::write(&path, pdf).unwrap();
        let out = std::process::Command::new("pdftotext")
            .arg(&path)
            .arg("-")
            .output()
            .expect("pdftotext is needed to read the document back");
        String::from_utf8_lossy(&out.stdout).split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// The defect, stated as a test: a notice generated today must carry the
    /// firm's identity. Every one of these was blank before this module existed.
    #[tokio::test]
    async fn a_generated_notice_carries_the_firms_letterhead() {
        if !latex::engine_available() {
            return;
        }

        let pool = test_pool().await;
        let text = pdf_text(&render_notice(&pool, None).await);

        for expected in [
            // Both partners, with the numbers a recipient would call.
            "Sreelakshmi Menon",
            "Advocate & Partner",
            "+91 99535 31789",
            "Kajal Thakur",
            "+91 93153 67642",
            "kajalthakur.pnp@outlook.com",
            // The footer: website, contact, registered office.
            "www.persistas.com",
            "persistas.pnp@outlook.com",
            "80-A, Pocket-A, Mayuri Enclave,",
            "Mayur Vihar Phase-III, Delhi - 110096",
            // The date, with its ordinal, and the signature under it.
            "14th July 2026",
            "D/6361/2020",
            // The addressee's own address, built from what was typed.
            "House No. 460/21,",
            "Lucknow",
        ] {
            assert!(
                text.contains(expected),
                "the notice does not carry {expected:?}. Page text:\n{text}"
            );
        }
    }

    /// The ampersand in "Advocate & Partner" is the character that broke every
    /// invoice the firm issued the last time a value went to LaTeX unescaped.
    /// It reaches the page as an ampersand, and the page still compiles.
    #[tokio::test]
    async fn the_firms_ampersand_survives_the_round_trip_to_the_page() {
        if !latex::engine_available() {
            return;
        }

        let pool = test_pool().await;
        sqlx::query("UPDATE firm_settings SET firm_website = ? WHERE id = 1")
            .bind("Sharma & Co. — 100% of the time")
            .execute(&pool)
            .await
            .unwrap();

        let text = pdf_text(&render_notice(&pool, None).await);
        assert!(text.contains("Sharma & Co."), "page text:\n{text}");
        assert!(text.contains("100% of the time"), "page text:\n{text}");
    }

    /// Who signed is who generated it. The letterhead spells her name one way
    /// and her login another, and the notice still goes out over her name.
    #[tokio::test]
    async fn the_notice_is_signed_by_the_attorney_who_produced_it() {
        if !latex::engine_available() {
            return;
        }

        let pool = test_pool().await;
        let signed_in = SessionData {
            session_id: "s-1".into(),
            user_id: "user-kt".into(),
            name: "Kajal Thakur".into(),
            role: "Partner".into(),
        };

        let text = pdf_text(&render_notice(&pool, Some(&signed_in)).await);

        // "Yours Sincerely" and then the signatory — the name appears in the
        // letterhead too, so the position is what makes this an assertion.
        let closing = text.find("Yours Sincerely").expect("the notice must close");
        assert!(
            text[closing..].contains("Kajal Thakur Advocates"),
            "the signature block does not name the signatory. Page text:\n{}",
            &text[closing..]
        );
    }
}
