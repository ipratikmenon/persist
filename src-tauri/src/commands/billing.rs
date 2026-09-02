// Phase 2 Module 4 — Time Tracking & Billing
// Tauri commands for time entries, invoices, payments, and firm settings.
// All monetary values in INR. GST calculated here — never in Deck.

use crate::AppState;
use crate::services::money;
use crate::services::sync_engine::{self, EntityType, Op};
use crate::db::queries::billing as bq;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// IPC types — sent to Deck (snake_case → camelCase via serde)
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FirmSettings {
    pub firm_name:           String,
    pub firm_gstin:          Option<String>,
    pub firm_address:        Option<String>,
    pub firm_pan:            Option<String>,
    pub bank_name:           Option<String>,
    pub bank_account:        Option<String>,
    pub bank_ifsc:           Option<String>,
    pub default_hourly_rate: f64,
    pub partner_rate:        f64,
    pub associate_rate:      f64,
    pub paralegal_rate:      f64,
    pub gst_rate:            f64,
    /// Letterhead — printed on correspondence, not on invoices, but the same
    /// single record of who the firm is.
    pub firm_website:         Option<String>,
    pub firm_contact_email:   Option<String>,
    pub firm_office_line_one: Option<String>,
    pub firm_office_line_two: Option<String>,
    /// The partners as they appear on the letterhead, senior first.
    pub partners:             Vec<FirmPartner>,
    pub updated_at:          String,
}

/// One partner on the letterhead.
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FirmPartner {
    /// Empty when Deck is adding a row — Keel allocates the id on save.
    #[serde(default)]
    pub id:               String,
    pub name:             String,
    /// As it prints under the name — "Advocate & Partner".
    pub role:             String,
    pub phone:            Option<String>,
    pub email:            Option<String>,
    /// Bar Council enrolment, e.g. D/6361/2020. Printed on the signature block.
    pub enrolment_number: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimeEntry {
    pub id:            String,
    pub matter_id:     String,
    pub user_id:       String,
    pub date:          String,
    pub hours:         f64,
    pub description:   String,
    pub activity_code: String,
    pub rate_per_hour: f64,
    pub amount:        f64,   // hours × rate_per_hour — computed
    pub is_billable:   bool,
    pub is_invoiced:   bool,
    pub invoice_id:    Option<String>,
    pub created_at:    String,
    pub updated_at:    String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Invoice {
    pub id:             String,
    pub client_id:      String,
    pub matter_ids:     Vec<String>,
    pub status:         String,
    pub invoice_date:   String,
    pub due_date:       Option<String>,
    pub subtotal:       f64,
    pub cgst_amount:    f64,
    pub sgst_amount:    f64,
    pub igst_amount:    f64,
    pub total_with_tax: f64,
    pub amount_paid:    f64,
    pub balance_due:    f64,   // computed: total_with_tax - amount_paid
    pub notes:          Option<String>,
    pub gst_type:       String,
    pub pdf_doc_id:     Option<String>,
    pub created_by:     String,
    pub created_at:     String,
    pub updated_at:     String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceSummary {
    pub id:             String,
    pub client_name:    String,
    pub status:         String,
    pub invoice_date:   String,
    pub due_date:       Option<String>,
    pub total_with_tax: f64,
    pub amount_paid:    f64,
    pub balance_due:    f64,
    pub is_overdue:     bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LineItem {
    pub id:             String,
    pub invoice_id:     String,
    pub time_entry_id:  Option<String>,
    pub description:    String,
    pub activity_code:  Option<String>,
    pub hours:          Option<f64>,
    pub rate:           f64,
    pub amount:         f64,
    pub sort_order:     i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Payment {
    pub id:           String,
    pub invoice_id:   String,
    pub amount:       f64,
    pub payment_date: String,
    pub method:       String,
    pub reference:    Option<String>,
    pub notes:        Option<String>,
    pub recorded_by:  String,
    pub created_at:   String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UnbilledSummary {
    pub matter_id:   String,
    pub total_hours:  f64,
    pub total_amount: f64,
    pub entry_count:  i64,
}

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTimeEntryInput {
    pub matter_id:     String,
    pub date:          String,
    pub hours:         f64,
    pub description:   String,
    pub activity_code: Option<String>,
    pub is_billable:   Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTimeEntryInput {
    pub date:          Option<String>,
    pub hours:         Option<f64>,
    pub description:   Option<String>,
    pub activity_code: Option<String>,
    pub is_billable:   Option<bool>,
}

/// A single line on an invoice that the composer sends.
/// Either `time_entry_id` OR (`description` + `rate` + `amount`) for fixed-fee lines.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineItemInput {
    pub time_entry_id: Option<String>,
    pub description:   String,
    pub activity_code: Option<String>,
    pub hours:         Option<f64>,
    pub rate:          f64,
    pub amount:        f64,
    #[allow(dead_code)]
    pub sort_order:    i64,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInvoiceInput {
    pub client_id:    String,
    pub matter_ids:   Vec<String>,
    pub invoice_date: String,
    pub due_date:     Option<String>,
    pub gst_type:     String,   // "Intra" | "Inter"
    pub line_items:   Vec<LineItemInput>,
    pub notes:        Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordPaymentInput {
    pub invoice_id:   String,
    pub amount:       f64,
    pub payment_date: String,
    pub method:       String,
    pub reference:    Option<String>,
    pub notes:        Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFirmSettingsInput {
    pub firm_name:      Option<String>,
    pub firm_gstin:     Option<String>,
    pub firm_address:   Option<String>,
    pub firm_pan:       Option<String>,
    pub bank_name:      Option<String>,
    pub bank_account:   Option<String>,
    pub bank_ifsc:      Option<String>,
    pub partner_rate:   Option<f64>,
    pub associate_rate: Option<f64>,
    pub paralegal_rate: Option<f64>,
    pub firm_website:         Option<String>,
    pub firm_contact_email:   Option<String>,
    pub firm_office_line_one: Option<String>,
    pub firm_office_line_two: Option<String>,
    /// The whole letterhead roster, in the order it prints. Absent leaves it
    /// alone; present replaces it, so removing a partner is removing a row.
    pub partners:             Option<Vec<FirmPartner>>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn row_to_time_entry(r: bq::TimeEntryRow) -> TimeEntry {
    let amount = r.hours * r.rate_per_hour;
    TimeEntry {
        id:            r.id,
        matter_id:     r.matter_id,
        user_id:       r.user_id,
        date:          r.date,
        hours:         r.hours,
        description:   r.description,
        activity_code: r.activity_code,
        rate_per_hour: r.rate_per_hour,
        amount,
        is_billable:   r.is_billable != 0,
        is_invoiced:   r.is_invoiced != 0,
        invoice_id:    r.invoice_id,
        created_at:    r.created_at,
        updated_at:    r.updated_at,
    }
}

fn row_to_invoice(r: bq::InvoiceRow) -> Invoice {
    let balance_due = (r.total_with_tax - r.amount_paid).max(0.0);
    let matter_ids: Vec<String> = serde_json::from_str(&r.matter_ids).unwrap_or_default();
    Invoice {
        id: r.id, client_id: r.client_id, matter_ids,
        status: r.status, invoice_date: r.invoice_date, due_date: r.due_date,
        subtotal: r.subtotal, cgst_amount: r.cgst_amount, sgst_amount: r.sgst_amount,
        igst_amount: r.igst_amount, total_with_tax: r.total_with_tax,
        amount_paid: r.amount_paid, balance_due, notes: r.notes,
        gst_type: r.gst_type, pdf_doc_id: r.pdf_doc_id, created_by: r.created_by,
        created_at: r.created_at, updated_at: r.updated_at,
    }
}

fn row_to_summary(r: bq::InvoiceSummaryRow) -> InvoiceSummary {
    let balance_due = (r.total_with_tax - r.amount_paid).max(0.0);
    let is_overdue = r.status == "Sent" && r.due_date.as_deref()
        .map(|d| d < chrono::Utc::now().format("%Y-%m-%d").to_string().as_str())
        .unwrap_or(false);
    InvoiceSummary {
        id: r.id, client_name: r.client_name, status: r.status,
        invoice_date: r.invoice_date, due_date: r.due_date,
        total_with_tax: r.total_with_tax, amount_paid: r.amount_paid,
        balance_due, is_overdue,
    }
}

fn row_to_payment(r: bq::PaymentRow) -> Payment {
    Payment {
        id: r.id, invoice_id: r.invoice_id, amount: r.amount,
        payment_date: r.payment_date, method: r.method, reference: r.reference,
        notes: r.notes, recorded_by: r.recorded_by, created_at: r.created_at,
    }
}

fn row_to_line_item(r: bq::LineItemRow) -> LineItem {
    LineItem {
        id: r.id, invoice_id: r.invoice_id, time_entry_id: r.time_entry_id,
        description: r.description, activity_code: r.activity_code,
        hours: r.hours, rate: r.rate, amount: r.amount, sort_order: r.sort_order,
    }
}

/// Calculate GST amounts from subtotal.
fn calc_gst(subtotal: f64, gst_type: &str, gst_rate: f64) -> (f64, f64, f64) {
    if gst_type == "Inter" {
        (0.0, 0.0, subtotal * gst_rate)              // igst
    } else {
        let half = subtotal * gst_rate / 2.0;
        (half, half, 0.0)                            // cgst, sgst
    }
}

/// Look up the hourly rate for a user's role from firm_settings.
fn rate_for_role(role: &str, settings: &bq::FirmSettingsRow) -> f64 {
    match role {
        "Partner"   => settings.partner_rate,
        "Associate" => settings.associate_rate,
        "Paralegal" => settings.paralegal_rate,
        _           => settings.default_hourly_rate,
    }
}

// ---------------------------------------------------------------------------
// Commands — Firm Settings
// ---------------------------------------------------------------------------

fn row_to_firm_settings(r: bq::FirmSettingsRow, partners: Vec<bq::FirmPartnerRow>) -> FirmSettings {
    FirmSettings {
        firm_name: r.firm_name, firm_gstin: r.firm_gstin, firm_address: r.firm_address,
        firm_pan: r.firm_pan, bank_name: r.bank_name, bank_account: r.bank_account,
        bank_ifsc: r.bank_ifsc, default_hourly_rate: r.default_hourly_rate,
        partner_rate: r.partner_rate, associate_rate: r.associate_rate,
        paralegal_rate: r.paralegal_rate, gst_rate: r.gst_rate,
        firm_website: r.firm_website, firm_contact_email: r.firm_contact_email,
        firm_office_line_one: r.firm_office_line_one,
        firm_office_line_two: r.firm_office_line_two,
        partners: partners.into_iter().map(|p| FirmPartner {
            id: p.id, name: p.name, role: p.role, phone: p.phone, email: p.email,
            enrolment_number: p.enrolment_number,
        }).collect(),
        updated_at: r.updated_at,
    }
}

#[tauri::command]
pub async fn get_firm_settings(
    state: tauri::State<'_, AppState>,
) -> Result<FirmSettings, String> {
    let pool = state.db.lock().await;
    let r = bq::get_firm_settings(&pool).await.map_err(|e| e.to_string())?;
    let partners = bq::list_firm_partners(&pool).await.map_err(|e| e.to_string())?;
    Ok(row_to_firm_settings(r, partners))
}

#[tauri::command]
pub async fn update_firm_settings(
    input: UpdateFirmSettingsInput,
    state: tauri::State<'_, AppState>,
) -> Result<FirmSettings, String> {
    crate::rbac::require(&state, crate::rbac::Permission::EditFirmSettings).await?;

    let pool = state.db.lock().await;
    let r = bq::update_firm_settings(
        &pool,
        input.firm_name.as_deref(), input.firm_gstin.as_deref(),
        input.firm_address.as_deref(), input.firm_pan.as_deref(),
        input.bank_name.as_deref(), input.bank_account.as_deref(),
        input.bank_ifsc.as_deref(),
        input.partner_rate, input.associate_rate, input.paralegal_rate,
        input.firm_website.as_deref(), input.firm_contact_email.as_deref(),
        input.firm_office_line_one.as_deref(), input.firm_office_line_two.as_deref(),
    ).await.map_err(|e| e.to_string())?;

    // A partner with no name would print as a blank line on every document the
    // firm issues afterwards, so an unnamed row is dropped rather than saved.
    let partners = match input.partners {
        Some(submitted) => {
            let rows: Vec<bq::FirmPartnerInput> = submitted
                .into_iter()
                .filter(|p| !p.name.trim().is_empty())
                .map(|p| bq::FirmPartnerInput {
                    id: if p.id.trim().is_empty() { Uuid::new_v4().to_string() } else { p.id },
                    name: p.name.trim().to_owned(),
                    role: p.role,
                    phone: p.phone,
                    email: p.email,
                    enrolment_number: p.enrolment_number,
                })
                .collect();
            bq::replace_firm_partners(&pool, &rows).await.map_err(|e| e.to_string())?
        }
        None => bq::list_firm_partners(&pool).await.map_err(|e| e.to_string())?,
    };

    Ok(row_to_firm_settings(r, partners))
}

// ---------------------------------------------------------------------------
// Commands — Time Entries
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn create_time_entry(
    input: CreateTimeEntryInput,
    state: tauri::State<'_, AppState>,
) -> Result<TimeEntry, String> {
    if input.hours < 0.25 {
        return Err("Minimum time entry is 0.25 hours (15 minutes)".into());
    }

    let session = state.session.lock().await;
    let (user_id, role) = match session.as_ref() {
        Some(s) => (s.user_id.clone(), s.role.clone()),
        None    => return Err("Not logged in".into()),
    };
    drop(session);

    let pool = state.db.lock().await;
    let settings = bq::get_firm_settings(&pool).await.map_err(|e| e.to_string())?;
    let rate = rate_for_role(&role, &settings);

    let id = Uuid::new_v4().to_string();
    let activity = input.activity_code.as_deref().unwrap_or("L300");
    let billable = input.is_billable.unwrap_or(true);

    let row = bq::create_time_entry(
        &pool, &id, &input.matter_id, &user_id, &input.date,
        input.hours, &input.description, activity, rate, billable,
    ).await.map_err(|e| e.to_string())?;
    Ok(row_to_time_entry(row))
}

#[tauri::command]
pub async fn update_time_entry(
    id: String,
    input: UpdateTimeEntryInput,
    state: tauri::State<'_, AppState>,
) -> Result<TimeEntry, String> {
    let pool = state.db.lock().await;
    let row = bq::update_time_entry(
        &pool, &id,
        input.date.as_deref(), input.hours,
        input.description.as_deref(), input.activity_code.as_deref(),
        input.is_billable,
    ).await.map_err(|e| e.to_string())?;
    Ok(row_to_time_entry(row))
}

#[tauri::command]
pub async fn delete_time_entry(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let pool = state.db.lock().await;
    bq::delete_time_entry(&pool, &id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_time_entries(
    matter_id:  Option<String>,
    user_id:    Option<String>,
    date_from:  Option<String>,
    date_to:    Option<String>,
    state:      tauri::State<'_, AppState>,
) -> Result<Vec<TimeEntry>, String> {
    let pool = state.db.lock().await;
    let rows = bq::list_time_entries(
        &pool,
        matter_id.as_deref(), user_id.as_deref(),
        date_from.as_deref(), date_to.as_deref(),
    ).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(row_to_time_entry).collect())
}

#[tauri::command]
pub async fn get_unbilled_summary(
    matter_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<UnbilledSummary, String> {
    let pool = state.db.lock().await;
    let (total_hours, total_amount, entry_count) =
        bq::unbilled_total(&pool, &matter_id).await.map_err(|e| e.to_string())?;
    Ok(UnbilledSummary { matter_id, total_hours, total_amount, entry_count })
}

// ---------------------------------------------------------------------------
// Commands — Invoices
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn create_invoice(
    input: CreateInvoiceInput,
    state: tauri::State<'_, AppState>,
) -> Result<Invoice, String> {
    crate::rbac::require(&state, crate::rbac::Permission::CreateInvoice).await?;

    let session = state.session.lock().await;
    let created_by = match session.as_ref() {
        Some(s) => s.user_id.clone(),
        None    => return Err("Not logged in".into()),
    };
    drop(session);

    let pool = state.db.lock().await;
    let settings = bq::get_firm_settings(&pool).await.map_err(|e| e.to_string())?;

    // Calculate totals from line items
    let subtotal: f64 = input.line_items.iter().map(|li| li.amount).sum();
    let (cgst, sgst, igst) = calc_gst(subtotal, &input.gst_type, settings.gst_rate);
    let total_with_tax = subtotal + cgst + sgst + igst;

    // Generate invoice ID
    let year = chrono::Utc::now().format("%Y").to_string().parse::<i32>().unwrap_or(2026);
    let seq = bq::next_invoice_number(&pool, year).await.map_err(|e| e.to_string())?;
    let invoice_id = format!("INV-{year}-{seq:04}");

    let matter_ids_json = serde_json::to_string(&input.matter_ids)
        .map_err(|e| e.to_string())?;

    let row = bq::create_invoice(
        &pool, &invoice_id, &input.client_id, &matter_ids_json,
        &input.invoice_date, input.due_date.as_deref(),
        subtotal, cgst, sgst, igst, total_with_tax,
        input.notes.as_deref(), &input.gst_type, &created_by,
    ).await.map_err(|e| e.to_string())?;

    // Insert line items
    for (i, li) in input.line_items.iter().enumerate() {
        let li_id = Uuid::new_v4().to_string();
        bq::insert_line_item(
            &pool, &li_id, &invoice_id,
            li.time_entry_id.as_deref(), &li.description,
            li.activity_code.as_deref(), li.hours, li.rate, li.amount,
            i as i64,
        ).await.map_err(|e| e.to_string())?;
    }

    // Mark time entries as invoiced
    let te_ids: Vec<String> = input.line_items.iter()
        .filter_map(|li| li.time_entry_id.clone())
        .collect();
    if !te_ids.is_empty() {
        bq::mark_entries_invoiced(&pool, &te_ids, &invoice_id)
            .await.map_err(|e| e.to_string())?;
    }

    Ok(row_to_invoice(row))
}

#[tauri::command]
pub async fn get_invoice(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Invoice, String> {
    let pool = state.db.lock().await;
    let row = bq::get_invoice(&pool, &id).await.map_err(|e| e.to_string())?;
    Ok(row_to_invoice(row))
}

#[tauri::command]
pub async fn list_invoices(
    client_id:  Option<String>,
    status:     Option<String>,
    date_from:  Option<String>,
    state:      tauri::State<'_, AppState>,
) -> Result<Vec<InvoiceSummary>, String> {
    let pool = state.db.lock().await;
    let rows = bq::list_invoices(
        &pool,
        client_id.as_deref(), status.as_deref(), date_from.as_deref(),
    ).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(row_to_summary).collect())
}

#[tauri::command]
pub async fn list_invoice_line_items(
    invoice_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<LineItem>, String> {
    let pool = state.db.lock().await;
    let rows = bq::list_line_items(&pool, &invoice_id).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(row_to_line_item).collect())
}

#[tauri::command]
pub async fn list_payments(
    invoice_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Payment>, String> {
    let pool = state.db.lock().await;
    let rows = bq::list_payments(&pool, &invoice_id).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(row_to_payment).collect())
}

#[tauri::command]
pub async fn update_invoice_status(
    id:     String,
    status: String,
    state:  tauri::State<'_, AppState>,
) -> Result<Invoice, String> {
    // Validate transition
    let pool = state.db.lock().await;
    let current = bq::get_invoice(&pool, &id).await.map_err(|e| e.to_string())?;

    let valid = match (current.status.as_str(), status.as_str()) {
        ("Draft",   "Sent")      => true,
        ("Draft",   "Cancelled") => true,
        ("Sent",    "Cancelled") => true,
        _ => false,
    };
    if !valid {
        return Err(format!(
            "Invalid status transition: {} → {}",
            current.status, status
        ));
    }

    if status == "Cancelled" {
        bq::unmark_entries_invoiced(&pool, &id).await.map_err(|e| e.to_string())?;
    }

    let row = bq::update_invoice_status(&pool, &id, &status)
        .await.map_err(|e| e.to_string())?;

    // Draft invoices are withheld at projection, so only the transitions that
    // change what the client can see are queued: issuing one publishes it,
    // cancelling an already-issued one withdraws it. Draft → Cancelled never
    // reached the portal and needs no tombstone.
    match (current.status.as_str(), status.as_str()) {
        (_,       "Sent")      => {
            sync_engine::note_change(&pool, EntityType::Invoice, &id, Op::Upsert).await;
        }
        ("Sent",  "Cancelled") => {
            sync_engine::note_change(&pool, EntityType::Invoice, &id, Op::Delete).await;
        }
        _ => {}
    }

    Ok(row_to_invoice(row))
}

#[tauri::command]
pub async fn record_payment(
    input: RecordPaymentInput,
    state: tauri::State<'_, AppState>,
) -> Result<Payment, String> {
    if input.amount <= 0.0 {
        return Err("Payment amount must be greater than zero".into());
    }
    let session = state.session.lock().await;
    let recorded_by = match session.as_ref() {
        Some(s) => s.user_id.clone(),
        None    => return Err("Not logged in".into()),
    };
    drop(session);

    let pool = state.db.lock().await;
    let id = Uuid::new_v4().to_string();
    let row = bq::record_payment(
        &pool, &id, &input.invoice_id, input.amount, &input.payment_date,
        &input.method, input.reference.as_deref(), input.notes.as_deref(),
        &recorded_by,
    ).await.map_err(|e| e.to_string())?;

    // The payment itself has no projection yet; what the client sees change is
    // the invoice's amount_paid, so that is what gets queued.
    sync_engine::note_change(&pool, EntityType::Invoice, &input.invoice_id, Op::Upsert).await;

    Ok(row_to_payment(row))
}

/// Generate invoice PDF via LaTeX. Returns the vault document ID.
#[tauri::command]
pub async fn generate_invoice_pdf(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    use crate::services::latex;

    let pool = state.db.lock().await;
    let inv = bq::get_invoice(&pool, &id).await.map_err(|e| e.to_string())?;
    let line_items = bq::list_line_items(&pool, &id).await.map_err(|e| e.to_string())?;
    let settings = bq::get_firm_settings(&pool).await.map_err(|e| e.to_string())?;

    // Load client details for invoice header
    let client_row = sqlx::query_as::<_, (String, Option<String>, Option<String>)>(
        "SELECT name, gstin, address FROM clients WHERE id = ?",
    )
    .bind(&inv.client_id)
    .fetch_optional(&*pool)
    .await
    .map_err(|e| e.to_string())?;
    drop(pool);

    let (client_name, client_gstin, client_address) = client_row
        .unwrap_or_else(|| (String::from("—"), None, None));

    // Build template variables.
    //
    // Every value is a `Field`: `text` is escaped on the way in, `raw` is LaTeX
    // this function assembled. There is no third option, which is why the
    // "escaped one field out of twenty" bug cannot recur here.
    use latex::Field;
    let mut fields = std::collections::HashMap::new();
    fields.insert("INVOICE_ID".into(),     Field::text(&inv.id));
    fields.insert("INVOICE_DATE".into(),   Field::text(&inv.invoice_date));
    fields.insert("DUE_DATE".into(),       Field::text(inv.due_date.clone().unwrap_or_default()));
    fields.insert("CLIENT_NAME".into(),    Field::text(&client_name));
    fields.insert("CLIENT_GSTIN".into(),   Field::text(client_gstin.unwrap_or_default()));
    fields.insert("CLIENT_ADDRESS".into(), Field::text(client_address.unwrap_or_default()));
    fields.insert("FIRM_NAME".into(),      Field::text(&settings.firm_name));
    fields.insert("FIRM_GSTIN".into(),     Field::text(settings.firm_gstin.unwrap_or_default()));
    fields.insert("FIRM_ADDRESS".into(),   Field::text(settings.firm_address.unwrap_or_default()));
    fields.insert("FIRM_PAN".into(),       Field::text(settings.firm_pan.unwrap_or_default()));
    fields.insert("FIRM_BANK".into(),      Field::text(format!("{} — A/C: {} IFSC: {}",
        settings.bank_name.unwrap_or_default(),
        settings.bank_account.unwrap_or_default(),
        settings.bank_ifsc.unwrap_or_default())));

    // Line items are the one place this function writes LaTeX itself. The
    // structural `&` and `\\` are markup; everything interpolated between them
    // goes through `latex::escape` first.
    let table_rows: String = line_items.iter().map(|li| {
        let hrs  = li.hours.map(|h| format!("{h:.2}")).unwrap_or_default();
        let code = latex::escape(li.activity_code.as_deref().unwrap_or(""));
        format!("{} & {} & {} & ₹{} & ₹{} \\\\",
            code, latex::escape(&li.description), hrs,
            money::format_inr(li.rate), money::format_inr(li.amount))
    }).collect::<Vec<_>>().join("\n");
    fields.insert("LINE_ITEMS_TABLE".into(), Field::raw(table_rows));

    fields.insert("SUBTOTAL".into(),        Field::text(money::format_inr(inv.subtotal)));
    fields.insert("CGST_AMOUNT".into(),     Field::text(money::format_inr(inv.cgst_amount)));
    fields.insert("SGST_AMOUNT".into(),     Field::text(money::format_inr(inv.sgst_amount)));
    fields.insert("IGST_AMOUNT".into(),     Field::text(money::format_inr(inv.igst_amount)));
    fields.insert("TOTAL".into(),           Field::text(money::format_inr(inv.total_with_tax)));
    fields.insert("AMOUNT_IN_WORDS".into(), Field::text(amount_in_words(inv.total_with_tax)));
    fields.insert("NOTES".into(),           Field::text(inv.notes.clone().unwrap_or_default()));
    fields.insert("SAC_CODE".into(),        Field::text("998212"));
    fields.insert("GST_TYPE".into(),        Field::text(&inv.gst_type));

    // Run LaTeX
    let pdf_bytes = latex::compile_latex("invoice", &fields).await
        .map_err(|e| format!("LaTeX compile failed: {e}"))?;

    // Store in vault
    let pool = state.db.lock().await;
    let doc_id = Uuid::new_v4().to_string();

    // `documents.matter_id` is NOT NULL REFERENCES matters(id). This previously
    // bound the client id — which the foreign key rejects, so the PDF was
    // encrypted into the vault and then the row failed, leaving an orphan.
    //
    // An invoice may cover several matters. It is filed against the first, which
    // is a real matter belonging to this client; an invoice covering none has
    // nowhere to be filed and says so rather than writing a broken row.
    let filing_matter_id = serde_json::from_str::<Vec<String>>(&inv.matter_ids)
        .unwrap_or_default()
        .into_iter()
        .next()
        .ok_or_else(|| format!(
            "Invoice {} is not linked to any matter, so its PDF has nowhere to be filed. \
             Add a matter to the invoice and try again.",
            inv.id
        ))?;

    let vault_path = {
        let vault_dir = &state.vault_dir;
        let matter_dir = vault_dir.join(&inv.client_id);
        std::fs::create_dir_all(&matter_dir)
            .map_err(|e| format!("Failed to create vault dir: {e}"))?;
        let enc_path = matter_dir.join(format!("{doc_id}.enc"));
        crate::storage::vault::encrypt_to_vault(
            vault_dir, &state.vault_key, &pdf_bytes, &inv.client_id, &doc_id,
        ).map_err(|e| format!("Vault encrypt failed: {e}"))?;
        enc_path.to_string_lossy().to_string()
    };

    let filename = format!("{}-invoice.pdf", inv.id);
    let file_size = pdf_bytes.len() as i64;
    sqlx::query(
        "INSERT INTO documents
         (id, matter_id, filename, category, mime_type, file_size_bytes, vault_path, uploaded_by)
         VALUES (?, ?, ?, 'Invoice', 'application/pdf', ?, ?, ?)"
    )
    .bind(&doc_id)
    .bind(&filing_matter_id)
    .bind(&filename)
    .bind(file_size)
    .bind(&vault_path)
    .bind(&inv.created_by)
    .execute(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    bq::set_invoice_pdf(&pool, &id, &doc_id).await.map_err(|e| e.to_string())?;
    Ok(doc_id)
}

// ---------------------------------------------------------------------------
// Helpers for generate_invoice_pdf
// ---------------------------------------------------------------------------


/// Convert a monetary amount to Indian-style words (INR).
/// e.g. 11800.50 → "Rupees Eleven Thousand Eight Hundred and Fifty Paise Only"
fn amount_in_words(amount: f64) -> String {
    let rupees = amount.floor() as u64;
    let paise  = ((amount - rupees as f64) * 100.0).round() as u8;

    let rupee_words = num_in_words(rupees);
    if paise == 0 {
        format!("Rupees {rupee_words} Only")
    } else {
        let paise_words = num_in_words(paise as u64);
        format!("Rupees {rupee_words} and {paise_words} Paise Only")
    }
}

fn num_in_words(n: u64) -> String {
    const ONES: &[&str] = &[
        "", "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine",
        "Ten", "Eleven", "Twelve", "Thirteen", "Fourteen", "Fifteen", "Sixteen",
        "Seventeen", "Eighteen", "Nineteen",
    ];
    const TENS: &[&str] = &[
        "", "", "Twenty", "Thirty", "Forty", "Fifty", "Sixty", "Seventy", "Eighty", "Ninety",
    ];

    if n == 0 { return "Zero".into(); }

    let mut parts = Vec::new();

    let crore    = n / 10_000_000;
    let lakh     = (n % 10_000_000) / 100_000;
    let thousand = (n % 100_000) / 1_000;
    let hundred  = (n % 1_000) / 100;
    let below    = n % 100;

    if crore    > 0 { parts.push(format!("{} Crore",    num_in_words(crore)));    }
    if lakh     > 0 { parts.push(format!("{} Lakh",     num_in_words(lakh)));     }
    if thousand > 0 { parts.push(format!("{} Thousand", num_in_words(thousand))); }
    if hundred  > 0 { parts.push(format!("{} Hundred",  ONES[hundred as usize])); }
    if below > 0 {
        if below < 20 {
            parts.push(ONES[below as usize].into());
        } else {
            let t = TENS[(below / 10) as usize];
            let o = ONES[(below % 10) as usize];
            if o.is_empty() { parts.push(t.into()); }
            else            { parts.push(format!("{t} {o}")); }
        }
    }
    parts.join(" ")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;




}
