// Allow-list projections — the ONLY place desktop rows become portal-visible
// payloads. specs/module-05-portal.md §5.1.
//
// REVIEWERS: treat any change in this file as security-relevant.
//
// THE RULE
//
// Nothing here serialises a desktop row wholesale. Every projection names each
// field explicitly, so a column added to `matters` next year is invisible to the
// portal until someone edits this file deliberately.
//
// This is the inverse of a deny-list. A deny-list fails *open* as the schema
// grows — you forget to add the new column and it leaks. An allow-list fails
// closed: you forget, and the field simply never appears.
//
// The `From<Row>` trait is deliberately NOT used. `From` invites `..Default` and
// struct-update syntax, both of which can carry fields you did not name. Plain
// functions that list every field are harder to write and much harder to get
// wrong.
//
// MONEY
//
// Desktop stores money as SQLite REAL. The mirror is NUMERIC(14,2). Everything
// monetary goes through `round_money` on the way out — an invoice total shown to
// a client must never drift by floating-point noise.

use crate::db::queries::billing::InvoiceRow;
use crate::db::queries::deadlines::DeadlineRow;
use crate::db::queries::ip_assets::IpAssetRow;
use crate::db::queries::matters::{ClientRow, MatterRow};

// ---------------------------------------------------------------------------
// Wire types — exactly what leaves the machine
// ---------------------------------------------------------------------------

/// The client itself. Everything else in the mirror hangs off this row, so it
/// must be projected first — a matter for a client the mirror has never seen is
/// refused by the foreign key, not silently accepted.
///
/// Only id and name. gstin, pan, address, phone, email and notes stay on the
/// desktop: the portal has no use for them and they are commercially sensitive.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientPublic {
    pub id:   String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatterPublic {
    pub id:                   String,
    pub client_id:            String,
    pub title:                String,
    pub matter_type:          String,
    pub status:               String,
    pub opened_date:          String,
    pub forum:                Option<String>,
    pub jurisdiction:         String,
    /// The client-facing note. `internal_notes` is NEVER projected.
    pub client_notes:         Option<String>,
    /// Display name only — never a user id, which would leak firm structure.
    pub responsible_attorney: Option<String>,
    pub next_deadline_date:   Option<String>,
    pub next_deadline_event:  Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeadlinePublic {
    pub id:              String,
    pub client_id:       String,
    pub matter_id:       String,
    pub docketing_event: String,
    pub due_date:        String,
    pub status:          String,
    // `notes` is NEVER projected — it routinely holds strategy
    // ("weak prior art, consider opposing").
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpAssetPublic {
    pub id:                  String,
    pub client_id:           String,
    pub matter_id:           String,
    pub asset_type:          String,
    pub title:               String,
    pub application_number:  Option<String>,
    pub registration_number: Option<String>,
    pub filing_date:         Option<String>,
    pub registration_date:   Option<String>,
    pub expiry_date:         Option<String>,
    pub status:              String,
    pub classes:             Vec<i64>,
    pub jurisdiction:        String,
    // `notes`, `priority_date` and `applicant_entity_type` are not projected:
    // internal, or commercially sensitive (entity type drives fee strategy).
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoicePublic {
    pub id:             String,
    pub client_id:      String,
    pub status:         String,
    pub invoice_date:   String,
    pub due_date:       Option<String>,
    pub subtotal:       f64,
    pub cgst_amount:    f64,
    pub sgst_amount:    f64,
    pub igst_amount:    f64,
    pub total_with_tax: f64,
    pub amount_paid:    f64,
    pub notes:          Option<String>,
    // `created_by`, `gst_type`, `matter_ids` and `pdf_doc_id` are not projected.
    // The PDF reaches the client as an object key set by the document pipeline,
    // never as a desktop vault id.
}

// ---------------------------------------------------------------------------
// Rejection — rows that must not be projected at all
// ---------------------------------------------------------------------------

/// Why a row was withheld. Callers log this; nothing silently disappears.
#[allow(dead_code)]  // DocumentNotShared is used by the document projection (Step 3b)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Withheld {
    /// Draft invoices never leave the firm (spec §15.7).
    DraftInvoice,
    /// The attorney has not marked this deadline client-visible.
    DeadlineNotClientVisible,
    /// The document is not shared.
    DocumentNotShared,
}

/// Money to 2dp, matching the mirror's NUMERIC(14,2).
fn round_money(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

// ---------------------------------------------------------------------------
// Projections
// ---------------------------------------------------------------------------

/// Project a matter. `responsible_attorney` is resolved to a display name by the
/// caller — this function will not accept a user id, so a caller cannot pass one
/// through by accident.
pub fn project_matter(
    row: &MatterRow,
    responsible_attorney_name: Option<&str>,
    next_deadline: Option<(&str, &str)>,
) -> MatterPublic {
    MatterPublic {
        id:                   row.id.clone(),
        client_id:            row.client_id.clone(),
        title:                row.title.clone(),
        matter_type:          row.matter_type.clone(),
        status:               row.status.clone(),
        opened_date:          row.opened_date.clone(),
        forum:                row.forum.clone(),
        jurisdiction:         row.jurisdiction.clone(),
        client_notes:         row.client_notes.clone(),
        responsible_attorney: responsible_attorney_name.map(str::to_string),
        next_deadline_date:   next_deadline.map(|(d, _)| d.to_string()),
        next_deadline_event:  next_deadline.map(|(_, e)| e.to_string()),
    }
    // NOT projected: sub_type, priority, responsible_partner_id,
    // target_close_date, internal_notes, tags, linked_matter_ids,
    // created_at, updated_at.
}

/// Project a deadline, or withhold it when the attorney has not made it
/// client-visible. Visibility is decided on the desktop, never in the mirror.
pub fn project_deadline(
    row: &DeadlineRow,
    client_id: &str,
) -> Result<DeadlinePublic, Withheld> {
    if row.is_client_visible == 0 {
        return Err(Withheld::DeadlineNotClientVisible);
    }

    Ok(DeadlinePublic {
        id:              row.id.clone(),
        client_id:       client_id.to_string(),
        matter_id:       row.matter_id.clone(),
        docketing_event: row.docketing_event.clone(),
        due_date:        row.due_date.clone(),
        status:          row.status.clone(),
    })
    // NOT projected: notes (strategy), event_type, urgency, reference_number,
    // completed_by, created_by, is_verified, verified_by/at, ip_asset_id,
    // cascade linkage.
}

pub fn project_ip_asset(row: &IpAssetRow, client_id: &str) -> IpAssetPublic {
    IpAssetPublic {
        id:                  row.id.clone(),
        client_id:           client_id.to_string(),
        matter_id:           row.matter_id.clone(),
        asset_type:          row.asset_type.clone(),
        title:               row.title.clone(),
        application_number:  row.application_number.clone(),
        registration_number: row.registration_number.clone(),
        filing_date:         row.filing_date.clone(),
        registration_date:   row.registration_date.clone(),
        expiry_date:         row.expiry_date.clone(),
        status:              row.status.clone(),
        classes:             parse_classes(&row.classes),
        jurisdiction:        row.jurisdiction.clone(),
    }
    // NOT projected: priority_date, grant_date, applicant_entity_type, notes.
}

/// Tolerant class parsing, matching db::queries::ip_assets — one malformed row
/// must not stop a sync batch.
fn parse_classes(raw: &str) -> Vec<i64> {
    serde_json::from_str::<Vec<i64>>(raw).unwrap_or_default()
}

/// Project a client. Nothing is withheld: a client the firm has invited to the
/// portal needs a row, and the only two fields carried are the two the mirror
/// declares.
pub fn project_client(row: &ClientRow) -> ClientPublic {
    ClientPublic { id: row.id.clone(), name: row.name.clone() }
}

/// Project an invoice, or withhold it while it is a draft.
///
/// The exclusion happens *here*, at projection time, not as a filter in the
/// portal API — a draft must never reach the mirror at all (spec §15.7).
pub fn project_invoice(row: &InvoiceRow) -> Result<InvoicePublic, Withheld> {
    if row.status == "Draft" {
        return Err(Withheld::DraftInvoice);
    }

    Ok(InvoicePublic {
        id:             row.id.clone(),
        client_id:      row.client_id.clone(),
        status:         row.status.clone(),
        invoice_date:   row.invoice_date.clone(),
        due_date:       row.due_date.clone(),
        subtotal:       round_money(row.subtotal),
        cgst_amount:    round_money(row.cgst_amount),
        sgst_amount:    round_money(row.sgst_amount),
        igst_amount:    round_money(row.igst_amount),
        total_with_tax: round_money(row.total_with_tax),
        amount_paid:    round_money(row.amount_paid),
        notes:          row.notes.clone(),
    })
    // NOT projected: matter_ids, gst_type, pdf_doc_id, created_by,
    // created_at, updated_at.
}

// ---------------------------------------------------------------------------
// Tests
//
// The important tests here are not "does the projection copy the title" — they
// are "does a field we never named stay out of the wire payload". Each builds a
// row with sensitive values populated, serialises the projection to JSON, and
// asserts the sensitive strings are absent from the bytes.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Serialise and search the actual wire bytes. Checking struct fields would
    /// only prove what we already know; checking the JSON proves what leaves.
    fn wire(value: &impl serde::Serialize) -> String {
        serde_json::to_string(value).expect("projection must serialise")
    }

    fn matter_row() -> MatterRow {
        MatterRow {
            id:                     "P&P-2026-TM-0042".into(),
            client_id:              "c-1".into(),
            title:                  "PETALVEDA — word mark".into(),
            matter_type:            "Trademark".into(),
            sub_type:               Some("SUBTYPE_LEAK".into()),
            status:                 "Active".into(),
            priority:               "PRIORITY_LEAK".into(),
            responsible_partner_id: Some("USERID_LEAK".into()),
            forum:                  Some("Trade Marks Registry, Delhi".into()),
            jurisdiction:           "India".into(),
            opened_date:            "2026-01-10".into(),
            target_close_date:      Some("TARGETDATE_LEAK".into()),
            internal_notes:         Some("INTERNAL_LEAK weak prior art, push back".into()),
            client_notes:           Some("We have responded to the report.".into()),
            tags:                   r#"["TAGS_LEAK"]"#.into(),
            linked_matter_ids:      r#"["LINKED_LEAK"]"#.into(),
            created_at:             "2026-01-10 09:00:00".into(),
            updated_at:             "2026-08-01 09:00:00".into(),
        }
    }

    #[test]
    fn matter_projection_omits_every_unnamed_field() {
        let out = wire(&project_matter(&matter_row(), Some("Sree Lakshmi Menon"), None));

        for leak in [
            "INTERNAL_LEAK",   // internal_notes — the big one
            "SUBTYPE_LEAK",
            "PRIORITY_LEAK",
            "USERID_LEAK",     // responsible_partner_id leaks firm structure
            "TARGETDATE_LEAK",
            "TAGS_LEAK",
            "LINKED_LEAK",
        ] {
            assert!(!out.contains(leak), "{leak} reached the wire payload: {out}");
        }

        // ...and the fields we *do* want are present.
        assert!(out.contains("PETALVEDA"));
        assert!(out.contains("We have responded"));
        assert!(out.contains("Sree Lakshmi Menon"));
    }

    #[test]
    fn matter_projection_carries_a_name_not_a_user_id() {
        // The signature only accepts a name, so this is a compile-time property
        // as much as a runtime one. Assert the id never appears regardless.
        let out = wire(&project_matter(&matter_row(), Some("Kajal Thakur"), None));
        assert!(out.contains("Kajal Thakur"));
        assert!(!out.contains("user-kt"));
        assert!(!out.contains("USERID_LEAK"));
    }

    fn deadline_row(visible: i64) -> DeadlineRow {
        DeadlineRow {
            id:               "d-1".into(),
            matter_id:        "P&P-2026-TM-0042".into(),
            ip_asset_id:      Some("IPASSET_LEAK".into()),
            reference_number: Some("REFERENCE_LEAK".into()),
            docketing_event:  "Response to Examination Report".into(),
            event_type:       "Statutory".into(),
            due_date:         "2026-09-01".into(),
            status:           "Pending".into(),
            urgency:          "URGENCY_LEAK".into(),
            notes:            Some("NOTES_LEAK consider opposing, weak prior art".into()),
            completed_at:     None,
            completed_by:     Some("COMPLETEDBY_LEAK".into()),
            created_by:       Some("CREATEDBY_LEAK".into()),
            is_verified:      1,
            verified_by:      Some("VERIFIEDBY_LEAK".into()),
            verified_at:      Some("VERIFIEDAT_LEAK".into()),
            is_client_visible: visible,
            created_at:       "2026-07-01 09:00:00".into(),
            updated_at:       "2026-07-01 09:00:00".into(),
        }
    }

    #[test]
    fn deadline_projection_omits_notes_and_internal_fields() {
        let out = wire(&project_deadline(&deadline_row(1), "c-1").expect("visible"));

        for leak in [
            "NOTES_LEAK",        // strategy — the reason notes never sync
            "REFERENCE_LEAK",
            "URGENCY_LEAK",
            "CREATEDBY_LEAK",
            "VERIFIEDBY_LEAK",
            "VERIFIEDAT_LEAK",
            "COMPLETEDBY_LEAK",
            "IPASSET_LEAK",
        ] {
            assert!(!out.contains(leak), "{leak} reached the wire payload: {out}");
        }

        assert!(out.contains("Response to Examination Report"));
        assert!(out.contains("2026-09-01"));
    }

    #[test]
    fn deadline_not_marked_visible_is_withheld() {
        assert_eq!(
            project_deadline(&deadline_row(0), "c-1"),
            Err(Withheld::DeadlineNotClientVisible),
        );
    }

    fn ip_asset_row() -> IpAssetRow {
        IpAssetRow {
            id:                    "ip-1".into(),
            matter_id:             "P&P-2026-TM-0042".into(),
            asset_type:            "Trademark".into(),
            title:                 "PETALVEDA".into(),
            application_number:    Some("5544121".into()),
            registration_number:   None,
            filing_date:           Some("2026-01-10".into()),
            priority_date:         Some("PRIORITYDATE_LEAK".into()),
            grant_date:            Some("GRANTDATE_LEAK".into()),
            registration_date:     None,
            expiry_date:           Some("2036-01-10".into()),
            applicant_entity_type: "ENTITYTYPE_LEAK".into(),
            jurisdiction:          "India".into(),
            classes:               "[3,44]".into(),
            status:                "Examination".into(),
            notes:                 Some("ASSETNOTES_LEAK".into()),
            created_at:            "2026-01-10 09:00:00".into(),
            updated_at:            "2026-01-10 09:00:00".into(),
        }
    }

    #[test]
    fn ip_asset_projection_omits_internal_fields() {
        let out = wire(&project_ip_asset(&ip_asset_row(), "c-1"));

        for leak in [
            "ASSETNOTES_LEAK",
            "ENTITYTYPE_LEAK",   // drives fee strategy — commercially sensitive
            "PRIORITYDATE_LEAK",
            "GRANTDATE_LEAK",
        ] {
            assert!(!out.contains(leak), "{leak} reached the wire payload: {out}");
        }

        assert!(out.contains("PETALVEDA"));
        assert!(out.contains("5544121"));
    }

    #[test]
    fn ip_asset_classes_survive_as_numbers() {
        let p = project_ip_asset(&ip_asset_row(), "c-1");
        assert_eq!(p.classes, vec![3, 44]);
    }

    #[test]
    fn malformed_classes_do_not_break_the_batch() {
        let mut row = ip_asset_row();
        row.classes = "not json".into();
        assert!(project_ip_asset(&row, "c-1").classes.is_empty());
    }

    /// The client projection is two fields wide by design. This test exists so
    /// that widening `ClientRow` later — to answer some other query — cannot
    /// quietly widen what the portal is told about the firm's clients.
    #[test]
    fn client_projection_is_only_id_and_name() {
        let out = wire(&project_client(&ClientRow {
            id:   "c-petal".into(),
            name: "Petalveda Botanicals Pvt Ltd".into(),
        }));

        assert!(out.contains("Petalveda Botanicals Pvt Ltd"));

        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        let fields: Vec<String> = parsed.as_object().unwrap().keys().cloned().collect();
        assert_eq!(fields, ["id", "name"], "the client wire type grew: {out}");
    }

    fn invoice_row(status: &str) -> InvoiceRow {
        InvoiceRow {
            id:             "INV-2026-0014".into(),
            client_id:      "c-1".into(),
            matter_ids:     r#"["MATTERIDS_LEAK"]"#.into(),
            status:         status.into(),
            invoice_date:   "2026-07-15".into(),
            due_date:       Some("2026-08-14".into()),
            subtotal:       70_000.005,
            cgst_amount:    6_300.004,
            sgst_amount:    6_300.004,
            igst_amount:    0.0,
            total_with_tax: 82_600.013,
            amount_paid:    0.0,
            notes:          Some("Professional fees for TM prosecution".into()),
            gst_type:       "GSTTYPE_LEAK".into(),
            pdf_doc_id:     Some("PDFDOCID_LEAK".into()),
            created_by:     "CREATEDBY_LEAK".into(),
            created_at:     "2026-07-15 09:00:00".into(),
            updated_at:     "2026-07-15 09:00:00".into(),
        }
    }

    #[test]
    fn draft_invoices_are_withheld_at_projection_time() {
        // Not filtered later in the API — a draft must never reach the mirror.
        assert_eq!(project_invoice(&invoice_row("Draft")), Err(Withheld::DraftInvoice));
    }

    #[test]
    fn sent_invoice_projection_omits_internal_fields() {
        let out = wire(&project_invoice(&invoice_row("Sent")).expect("not a draft"));

        for leak in ["MATTERIDS_LEAK", "GSTTYPE_LEAK", "PDFDOCID_LEAK", "CREATEDBY_LEAK"] {
            assert!(!out.contains(leak), "{leak} reached the wire payload: {out}");
        }
        assert!(out.contains("INV-2026-0014"));
    }

    #[test]
    fn money_is_rounded_to_two_decimal_places() {
        // The mirror is NUMERIC(14,2); a total shown to a client must not drift.
        let p = project_invoice(&invoice_row("Sent")).unwrap();
        assert_eq!(p.subtotal,       70_000.01);
        assert_eq!(p.cgst_amount,    6_300.00);
        assert_eq!(p.total_with_tax, 82_600.01);
    }

    /// A time entry has no projection at all — `time_entries` never syncs. This
    /// test exists so that "someone adds one later" is a deliberate act with a
    /// failing test in front of it, not an oversight.
    #[test]
    fn there_is_no_time_entry_projection() {
        let source = include_str!("projection.rs");
        // Split so the needle does not appear literally in this file, which
        // would make the test fail against its own source.
        let needle = concat!("fn project_", "time_entry");
        assert!(
            !source.contains(needle),
            "time_entries must never sync — it is the firm's most commercially \
             sensitive data (spec §5.3). If this is being added deliberately, \
             the spec has to change first."
        );
    }
}
