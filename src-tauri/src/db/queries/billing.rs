use sqlx::SqlitePool;

// ---------------------------------------------------------------------------
// Row types — map directly to SQLite columns
// ---------------------------------------------------------------------------

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct FirmSettingsRow {
    pub id:                  i64,
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
    // Letterhead (0013). These print on correspondence rather than on an
    // invoice, but they are the same single row of firm identity.
    pub firm_website:          Option<String>,
    pub firm_contact_email:    Option<String>,
    pub firm_office_line_one:  Option<String>,
    pub firm_office_line_two:  Option<String>,
    pub updated_at:          String,
}

/// One partner as the letterhead and the signature block need them.
#[derive(Debug, sqlx::FromRow, Clone)]
pub struct FirmPartnerRow {
    pub id:               String,
    pub user_id:          Option<String>,
    pub name:             String,
    pub role:             String,
    pub phone:            Option<String>,
    pub email:            Option<String>,
    pub enrolment_number: Option<String>,
    pub sort_order:       i64,
}

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct TimeEntryRow {
    pub id:            String,
    pub matter_id:     String,
    pub user_id:       String,
    pub date:          String,
    pub hours:         f64,
    pub description:   String,
    pub activity_code: String,
    pub rate_per_hour: f64,
    pub is_billable:   i64,
    pub is_invoiced:   i64,
    pub invoice_id:    Option<String>,
    pub created_at:    String,
    pub updated_at:    String,
}

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct InvoiceRow {
    pub id:             String,
    pub client_id:      String,
    pub matter_ids:     String,   // JSON array text
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
    pub gst_type:       String,
    pub pdf_doc_id:     Option<String>,
    pub created_by:     String,
    pub created_at:     String,
    pub updated_at:     String,
}

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct InvoiceSummaryRow {
    pub id:             String,
    pub client_name:    String,
    pub status:         String,
    pub invoice_date:   String,
    pub due_date:       Option<String>,
    pub total_with_tax: f64,
    pub amount_paid:    f64,
}

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct LineItemRow {
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

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct PaymentRow {
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

// ---------------------------------------------------------------------------
// firm_settings queries
// ---------------------------------------------------------------------------

pub async fn get_firm_settings(pool: &SqlitePool) -> anyhow::Result<FirmSettingsRow> {
    let row = sqlx::query_as::<_, FirmSettingsRow>(
        "SELECT id, firm_name, firm_gstin, firm_address, firm_pan, bank_name, bank_account,
                bank_ifsc, default_hourly_rate, partner_rate, associate_rate, paralegal_rate,
                gst_rate, firm_website, firm_contact_email, firm_office_line_one,
                firm_office_line_two, updated_at
         FROM firm_settings WHERE id = 1"
    )
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn update_firm_settings(
    pool: &SqlitePool,
    firm_name: Option<&str>,
    firm_gstin: Option<&str>,
    firm_address: Option<&str>,
    firm_pan: Option<&str>,
    bank_name: Option<&str>,
    bank_account: Option<&str>,
    bank_ifsc: Option<&str>,
    partner_rate: Option<f64>,
    associate_rate: Option<f64>,
    paralegal_rate: Option<f64>,
    // Letterhead (0013), in the same statement as the rest: saving the firm's
    // identity must not be able to half-succeed.
    firm_website: Option<&str>,
    firm_contact_email: Option<&str>,
    firm_office_line_one: Option<&str>,
    firm_office_line_two: Option<&str>,
) -> anyhow::Result<FirmSettingsRow> {
    sqlx::query(
        "UPDATE firm_settings SET
            firm_name       = COALESCE(?, firm_name),
            firm_gstin      = COALESCE(?, firm_gstin),
            firm_address    = COALESCE(?, firm_address),
            firm_pan        = COALESCE(?, firm_pan),
            bank_name       = COALESCE(?, bank_name),
            bank_account    = COALESCE(?, bank_account),
            bank_ifsc       = COALESCE(?, bank_ifsc),
            partner_rate    = COALESCE(?, partner_rate),
            associate_rate  = COALESCE(?, associate_rate),
            paralegal_rate  = COALESCE(?, paralegal_rate),
            firm_website         = COALESCE(?, firm_website),
            firm_contact_email   = COALESCE(?, firm_contact_email),
            firm_office_line_one = COALESCE(?, firm_office_line_one),
            firm_office_line_two = COALESCE(?, firm_office_line_two),
            updated_at      = datetime('now')
         WHERE id = 1"
    )
    .bind(firm_name)
    .bind(firm_gstin)
    .bind(firm_address)
    .bind(firm_pan)
    .bind(bank_name)
    .bind(bank_account)
    .bind(bank_ifsc)
    .bind(partner_rate)
    .bind(associate_rate)
    .bind(paralegal_rate)
    .bind(firm_website)
    .bind(firm_contact_email)
    .bind(firm_office_line_one)
    .bind(firm_office_line_two)
    .execute(pool)
    .await?;
    get_firm_settings(pool).await
}

// ---------------------------------------------------------------------------
// firm_partners queries — the letterhead roster (0013)
// ---------------------------------------------------------------------------

/// The partners as they print, senior first.
///
/// Inactive rows are left out: a partner who has left the firm must stop
/// appearing on documents the moment they are deactivated, without anyone
/// having to remember to delete the row.
pub async fn list_firm_partners(pool: &SqlitePool) -> anyhow::Result<Vec<FirmPartnerRow>> {
    let rows = sqlx::query_as::<_, FirmPartnerRow>(
        "SELECT id, user_id, name, role, phone, email, enrolment_number, sort_order
         FROM firm_partners WHERE is_active = 1 ORDER BY sort_order, name"
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// What the settings form sends back for one row of the roster.
#[derive(Debug, Clone)]
pub struct FirmPartnerInput {
    /// Empty for a partner being added — the caller allocates the id.
    pub id:               String,
    pub name:             String,
    pub role:             String,
    pub phone:            Option<String>,
    pub email:            Option<String>,
    pub enrolment_number: Option<String>,
}

/// Replace the letterhead roster with exactly this list, in this order.
///
/// Upsert-then-prune rather than delete-then-insert, so that `user_id` — the
/// link between a partner and their login, which the form never sees — survives
/// an edit. In one transaction: a half-written letterhead is a document with one
/// partner on it.
pub async fn replace_firm_partners(
    pool: &SqlitePool,
    partners: &[FirmPartnerInput],
) -> anyhow::Result<Vec<FirmPartnerRow>> {
    let mut tx = pool.begin().await?;

    // Rows are matched by id, so a list arriving with unknown ids simply
    // becomes new partners and the old ones are pruned below.
    let mut keep: Vec<String> = Vec::with_capacity(partners.len());
    for (index, partner) in partners.iter().enumerate() {
        sqlx::query(
            "INSERT INTO firm_partners
                 (id, name, role, phone, email, enrolment_number, sort_order)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                 name             = excluded.name,
                 role             = excluded.role,
                 phone            = excluded.phone,
                 email            = excluded.email,
                 enrolment_number = excluded.enrolment_number,
                 sort_order       = excluded.sort_order,
                 is_active        = 1,
                 updated_at       = datetime('now')"
        )
        .bind(&partner.id)
        .bind(&partner.name)
        .bind(&partner.role)
        .bind(&partner.phone)
        .bind(&partner.email)
        .bind(&partner.enrolment_number)
        .bind(index as i64 + 1)
        .execute(&mut *tx)
        .await?;
        keep.push(partner.id.clone());
    }

    // Anything not in the submitted list is deactivated rather than deleted:
    // documents already generated name a partner, and a row that a foreign key
    // may point at should not disappear because someone edited a form.
    let placeholders = vec!["?"; keep.len()].join(",");
    let sql = format!(
        "UPDATE firm_partners SET is_active = 0, updated_at = datetime('now')
         WHERE is_active = 1 AND id NOT IN ({placeholders})"
    );
    let mut prune = sqlx::query(&sql);
    for id in &keep {
        prune = prune.bind(id);
    }
    prune.execute(&mut *tx).await?;

    tx.commit().await?;
    list_firm_partners(pool).await
}

// ---------------------------------------------------------------------------
// time_entries queries
// ---------------------------------------------------------------------------

pub async fn create_time_entry(
    pool: &SqlitePool,
    id: &str,
    matter_id: &str,
    user_id: &str,
    date: &str,
    hours: f64,
    description: &str,
    activity_code: &str,
    rate_per_hour: f64,
    is_billable: bool,
) -> anyhow::Result<TimeEntryRow> {
    sqlx::query(
        "INSERT INTO time_entries
         (id, matter_id, user_id, date, hours, description, activity_code, rate_per_hour, is_billable)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(matter_id)
    .bind(user_id)
    .bind(date)
    .bind(hours)
    .bind(description)
    .bind(activity_code)
    .bind(rate_per_hour)
    .bind(if is_billable { 1i64 } else { 0i64 })
    .execute(pool)
    .await?;
    get_time_entry(pool, id).await
}

pub async fn get_time_entry(pool: &SqlitePool, id: &str) -> anyhow::Result<TimeEntryRow> {
    let row = sqlx::query_as::<_, TimeEntryRow>(
        "SELECT id, matter_id, user_id, date, hours, description, activity_code, rate_per_hour,
                is_billable, is_invoiced, invoice_id, created_at, updated_at
         FROM time_entries WHERE id = ?"
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn update_time_entry(
    pool: &SqlitePool,
    id: &str,
    date: Option<&str>,
    hours: Option<f64>,
    description: Option<&str>,
    activity_code: Option<&str>,
    is_billable: Option<bool>,
) -> anyhow::Result<TimeEntryRow> {
    // Verify not invoiced first
    let row = get_time_entry(pool, id).await?;
    if row.is_invoiced != 0 {
        anyhow::bail!("Cannot edit a time entry that has been invoiced");
    }
    sqlx::query(
        "UPDATE time_entries SET
            date          = COALESCE(?, date),
            hours         = COALESCE(?, hours),
            description   = COALESCE(?, description),
            activity_code = COALESCE(?, activity_code),
            is_billable   = COALESCE(?, is_billable),
            updated_at    = datetime('now')
         WHERE id = ?"
    )
    .bind(date)
    .bind(hours)
    .bind(description)
    .bind(activity_code)
    .bind(is_billable.map(|b| if b { 1i64 } else { 0i64 }))
    .bind(id)
    .execute(pool)
    .await?;
    get_time_entry(pool, id).await
}

pub async fn delete_time_entry(pool: &SqlitePool, id: &str) -> anyhow::Result<()> {
    let row = get_time_entry(pool, id).await?;
    if row.is_invoiced != 0 {
        anyhow::bail!("Cannot delete a time entry that has been invoiced");
    }
    sqlx::query("DELETE FROM time_entries WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_time_entries(
    pool: &SqlitePool,
    matter_id: Option<&str>,
    user_id: Option<&str>,
    date_from: Option<&str>,
    date_to: Option<&str>,
) -> anyhow::Result<Vec<TimeEntryRow>> {
    let sql = "SELECT id, matter_id, user_id, date, hours, description, activity_code,
                      rate_per_hour, is_billable, is_invoiced, invoice_id, created_at, updated_at
               FROM time_entries
               WHERE (? IS NULL OR matter_id = ?)
                 AND (? IS NULL OR user_id   = ?)
                 AND (? IS NULL OR date     >= ?)
                 AND (? IS NULL OR date     <= ?)
               ORDER BY date DESC, created_at DESC";
    let rows = sqlx::query_as::<_, TimeEntryRow>(sql)
        .bind(matter_id).bind(matter_id)
        .bind(user_id).bind(user_id)
        .bind(date_from).bind(date_from)
        .bind(date_to).bind(date_to)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn unbilled_total(pool: &SqlitePool, matter_id: &str) -> anyhow::Result<(f64, f64, i64)> {
    // Returns (total_hours, total_amount, entry_count) for unbilled billable entries
    #[derive(sqlx::FromRow)]
    struct Totals { total_hours: f64, total_amount: f64, entry_count: i64 }
    let t = sqlx::query_as::<_, Totals>(
        "SELECT COALESCE(SUM(hours), 0.0)          AS total_hours,
                COALESCE(SUM(hours * rate_per_hour), 0.0) AS total_amount,
                COUNT(*)                           AS entry_count
         FROM time_entries
         WHERE matter_id = ? AND is_billable = 1 AND is_invoiced = 0"
    )
    .bind(matter_id)
    .fetch_one(pool)
    .await?;
    Ok((t.total_hours, t.total_amount, t.entry_count))
}

/// Mark a batch of time entries as invoiced under a given invoice.
pub async fn mark_entries_invoiced(
    pool: &SqlitePool,
    entry_ids: &[String],
    invoice_id: &str,
) -> anyhow::Result<()> {
    for eid in entry_ids {
        sqlx::query(
            "UPDATE time_entries SET is_invoiced = 1, invoice_id = ?, updated_at = datetime('now')
             WHERE id = ?"
        )
        .bind(invoice_id)
        .bind(eid)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Unmark time entries when an invoice is cancelled.
pub async fn unmark_entries_invoiced(
    pool: &SqlitePool,
    invoice_id: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE time_entries SET is_invoiced = 0, invoice_id = NULL, updated_at = datetime('now')
         WHERE invoice_id = ?"
    )
    .bind(invoice_id)
    .execute(pool)
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// invoices queries
// ---------------------------------------------------------------------------

pub async fn create_invoice(
    pool: &SqlitePool,
    id: &str,
    client_id: &str,
    matter_ids_json: &str,
    invoice_date: &str,
    due_date: Option<&str>,
    subtotal: f64,
    cgst_amount: f64,
    sgst_amount: f64,
    igst_amount: f64,
    total_with_tax: f64,
    notes: Option<&str>,
    gst_type: &str,
    created_by: &str,
) -> anyhow::Result<InvoiceRow> {
    sqlx::query(
        "INSERT INTO invoices
         (id, client_id, matter_ids, invoice_date, due_date, subtotal,
          cgst_amount, sgst_amount, igst_amount, total_with_tax, notes, gst_type, created_by)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(client_id)
    .bind(matter_ids_json)
    .bind(invoice_date)
    .bind(due_date)
    .bind(subtotal)
    .bind(cgst_amount)
    .bind(sgst_amount)
    .bind(igst_amount)
    .bind(total_with_tax)
    .bind(notes)
    .bind(gst_type)
    .bind(created_by)
    .execute(pool)
    .await?;
    get_invoice(pool, id).await
}

/// The raw row for the sync projection. Returns None rather than erroring:
/// an invoice deleted since it was queued is a normal outcome, not a failure.
pub async fn get_invoice_row(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<InvoiceRow>> {
    let row = sqlx::query_as::<_, InvoiceRow>(
        "SELECT id, client_id, matter_ids, status, invoice_date, due_date, subtotal,
                cgst_amount, sgst_amount, igst_amount, total_with_tax, amount_paid,
                notes, gst_type, pdf_doc_id, created_by, created_at, updated_at
         FROM invoices WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn get_invoice(pool: &SqlitePool, id: &str) -> anyhow::Result<InvoiceRow> {
    let row = sqlx::query_as::<_, InvoiceRow>(
        "SELECT id, client_id, matter_ids, status, invoice_date, due_date, subtotal,
                cgst_amount, sgst_amount, igst_amount, total_with_tax, amount_paid,
                notes, gst_type, pdf_doc_id, created_by, created_at, updated_at
         FROM invoices WHERE id = ?"
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn list_invoices(
    pool: &SqlitePool,
    client_id: Option<&str>,
    status: Option<&str>,
    date_from: Option<&str>,
) -> anyhow::Result<Vec<InvoiceSummaryRow>> {
    let sql = "SELECT i.id, c.name AS client_name, i.status, i.invoice_date,
                      i.due_date, i.total_with_tax, i.amount_paid
               FROM invoices i
               JOIN clients c ON c.id = i.client_id
               WHERE (? IS NULL OR i.client_id = ?)
                 AND (? IS NULL OR i.status    = ?)
                 AND (? IS NULL OR i.invoice_date >= ?)
               ORDER BY i.invoice_date DESC";
    let rows = sqlx::query_as::<_, InvoiceSummaryRow>(sql)
        .bind(client_id).bind(client_id)
        .bind(status).bind(status)
        .bind(date_from).bind(date_from)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn update_invoice_status(
    pool: &SqlitePool,
    id: &str,
    status: &str,
) -> anyhow::Result<InvoiceRow> {
    sqlx::query(
        "UPDATE invoices SET status = ?, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(status)
    .bind(id)
    .execute(pool)
    .await?;
    get_invoice(pool, id).await
}

pub async fn set_invoice_pdf(
    pool: &SqlitePool,
    id: &str,
    pdf_doc_id: &str,
) -> anyhow::Result<InvoiceRow> {
    sqlx::query(
        "UPDATE invoices SET pdf_doc_id = ?, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(pdf_doc_id)
    .bind(id)
    .execute(pool)
    .await?;
    get_invoice(pool, id).await
}

pub async fn next_invoice_number(pool: &SqlitePool, year: i32) -> anyhow::Result<u64> {
    let key = format!("INV-{year}");
    sqlx::query(
        "INSERT INTO sequences (key, next_val) VALUES (?, 1)
         ON CONFLICT(key) DO UPDATE SET next_val = next_val + 1"
    )
    .bind(&key)
    .execute(pool)
    .await?;
    #[derive(sqlx::FromRow)]
    struct Seq { next_val: i64 }
    let s = sqlx::query_as::<_, Seq>("SELECT next_val FROM sequences WHERE key = ?")
        .bind(&key)
        .fetch_one(pool)
        .await?;
    Ok(s.next_val as u64)
}

// ---------------------------------------------------------------------------
// invoice_line_items queries
// ---------------------------------------------------------------------------

pub async fn insert_line_item(
    pool: &SqlitePool,
    id: &str,
    invoice_id: &str,
    time_entry_id: Option<&str>,
    description: &str,
    activity_code: Option<&str>,
    hours: Option<f64>,
    rate: f64,
    amount: f64,
    sort_order: i64,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO invoice_line_items
         (id, invoice_id, time_entry_id, description, activity_code, hours, rate, amount, sort_order)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(invoice_id)
    .bind(time_entry_id)
    .bind(description)
    .bind(activity_code)
    .bind(hours)
    .bind(rate)
    .bind(amount)
    .bind(sort_order)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_line_items(
    pool: &SqlitePool,
    invoice_id: &str,
) -> anyhow::Result<Vec<LineItemRow>> {
    let rows = sqlx::query_as::<_, LineItemRow>(
        "SELECT id, invoice_id, time_entry_id, description, activity_code, hours, rate, amount, sort_order
         FROM invoice_line_items WHERE invoice_id = ? ORDER BY sort_order ASC"
    )
    .bind(invoice_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ---------------------------------------------------------------------------
// payments queries
// ---------------------------------------------------------------------------

pub async fn record_payment(
    pool: &SqlitePool,
    id: &str,
    invoice_id: &str,
    amount: f64,
    payment_date: &str,
    method: &str,
    reference: Option<&str>,
    notes: Option<&str>,
    recorded_by: &str,
) -> anyhow::Result<PaymentRow> {
    sqlx::query(
        "INSERT INTO payments
         (id, invoice_id, amount, payment_date, method, reference, notes, recorded_by)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(invoice_id)
    .bind(amount)
    .bind(payment_date)
    .bind(method)
    .bind(reference)
    .bind(notes)
    .bind(recorded_by)
    .execute(pool)
    .await?;

    // Recompute amount_paid and status on invoice
    #[derive(sqlx::FromRow)]
    struct PaidTotal { total: f64 }
    let pt = sqlx::query_as::<_, PaidTotal>(
        "SELECT COALESCE(SUM(amount), 0.0) AS total FROM payments WHERE invoice_id = ?"
    )
    .bind(invoice_id)
    .fetch_one(pool)
    .await?;

    #[derive(sqlx::FromRow)]
    struct InvTotal { total_with_tax: f64, status: String }
    let inv = sqlx::query_as::<_, InvTotal>(
        "SELECT total_with_tax, status FROM invoices WHERE id = ?"
    )
    .bind(invoice_id)
    .fetch_one(pool)
    .await?;

    let new_status = if pt.total >= inv.total_with_tax {
        "Paid"
    } else if pt.total > 0.0 {
        "PartiallyPaid"
    } else {
        &inv.status
    };

    sqlx::query(
        "UPDATE invoices SET amount_paid = ?, status = ?, updated_at = datetime('now')
         WHERE id = ?"
    )
    .bind(pt.total)
    .bind(new_status)
    .bind(invoice_id)
    .execute(pool)
    .await?;

    // Return the payment row
    let row = sqlx::query_as::<_, PaymentRow>(
        "SELECT id, invoice_id, amount, payment_date, method, reference, notes, recorded_by, created_at
         FROM payments WHERE id = ?"
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn list_payments(
    pool: &SqlitePool,
    invoice_id: &str,
) -> anyhow::Result<Vec<PaymentRow>> {
    let rows = sqlx::query_as::<_, PaymentRow>(
        "SELECT id, invoice_id, amount, payment_date, method, reference, notes, recorded_by, created_at
         FROM payments WHERE invoice_id = ? ORDER BY payment_date DESC"
    )
    .bind(invoice_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("src/db/migrations").run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn firm_settings_default_row_exists() {
        let pool = test_pool().await;
        let s = get_firm_settings(&pool).await.unwrap();
        assert_eq!(s.firm_name, "Persistas & Partners");
        assert!((s.gst_rate - 0.18).abs() < f64::EPSILON);
        assert!((s.partner_rate - 8000.0).abs() < f64::EPSILON);
    }

    /// The update is fifteen positional arguments, which is exactly the shape of
    /// call where the office line quietly ends up in the website column. Each
    /// value is distinct so a transposition cannot pass.
    #[tokio::test]
    async fn every_letterhead_value_lands_in_its_own_column() {
        let pool = test_pool().await;

        update_firm_settings(
            &pool,
            None, None, None, None, None, None, None, None, None, None,
            Some("website.example"),
            Some("contact@example.in"),
            Some("office line one"),
            Some("office line two"),
        )
        .await
        .unwrap();

        let s = get_firm_settings(&pool).await.unwrap();
        assert_eq!(s.firm_website.as_deref(), Some("website.example"));
        assert_eq!(s.firm_contact_email.as_deref(), Some("contact@example.in"));
        assert_eq!(s.firm_office_line_one.as_deref(), Some("office line one"));
        assert_eq!(s.firm_office_line_two.as_deref(), Some("office line two"));
        // The billing half of the same row is untouched.
        assert_eq!(s.firm_name, "Persistas & Partners");
    }

    #[tokio::test]
    async fn the_firm_starts_with_its_two_partners_in_order() {
        let pool = test_pool().await;
        let partners = list_firm_partners(&pool).await.unwrap();

        assert_eq!(partners.len(), 2);
        assert_eq!(partners[0].name, "Sreelakshmi Menon");
        assert_eq!(partners[0].enrolment_number.as_deref(), Some("D/6361/2020"));
        assert_eq!(partners[1].name, "Kajal Thakur");
    }

    /// The settings form never sees `user_id` — it is the link between a partner
    /// and their login. Saving the roster must not drop it, or the next document
    /// that partner generates would be signed by someone else.
    #[tokio::test]
    async fn saving_the_roster_keeps_a_partner_linked_to_their_login() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO users (id, name, email, role, password_hash)
             VALUES ('u-1', 'Kajal Thakur', 'kt@persist.in', 'Partner', 'x')"
        ).execute(&pool).await.unwrap();
        sqlx::query("UPDATE firm_partners SET user_id = 'u-1' WHERE id = 'partner-kt'")
            .execute(&pool).await.unwrap();

        let submitted = vec![FirmPartnerInput {
            id: "partner-kt".into(),
            name: "Kajal Thakur".into(),
            role: "Advocate & Partner".into(),
            phone: Some("+91 93153 67642".into()),
            email: None,
            enrolment_number: Some("D/1234/2021".into()),
        }];

        let after = replace_firm_partners(&pool, &submitted).await.unwrap();

        assert_eq!(after.len(), 1, "the partner left off the list stops printing");
        assert_eq!(after[0].user_id.as_deref(), Some("u-1"), "the login link was lost");
        assert_eq!(after[0].enrolment_number.as_deref(), Some("D/1234/2021"));
    }

    /// Removed from the letterhead, not removed from history: documents already
    /// generated name this partner.
    #[tokio::test]
    async fn a_partner_taken_off_the_letterhead_is_deactivated_not_deleted() {
        let pool = test_pool().await;

        let keep = vec![FirmPartnerInput {
            id: "partner-slm".into(),
            name: "Sreelakshmi Menon".into(),
            role: "Advocate & Partner".into(),
            phone: None,
            email: None,
            enrolment_number: Some("D/6361/2020".into()),
        }];
        replace_firm_partners(&pool, &keep).await.unwrap();

        let still_there: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM firm_partners WHERE id = 'partner-kt'")
                .fetch_one(&pool).await.unwrap();
        assert_eq!(still_there, 1, "the row was deleted rather than deactivated");
        assert_eq!(list_firm_partners(&pool).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn a_new_partner_joins_the_end_of_the_letterhead() {
        let pool = test_pool().await;
        let mut submitted: Vec<FirmPartnerInput> = list_firm_partners(&pool)
            .await
            .unwrap()
            .into_iter()
            .map(|p| FirmPartnerInput {
                id: p.id, name: p.name, role: p.role, phone: p.phone, email: p.email,
                enrolment_number: p.enrolment_number,
            })
            .collect();
        submitted.push(FirmPartnerInput {
            id: "partner-new".into(),
            name: "A Third Partner".into(),
            role: "Advocate".into(),
            phone: None,
            email: None,
            enrolment_number: None,
        });

        let after = replace_firm_partners(&pool, &submitted).await.unwrap();
        assert_eq!(after.len(), 3);
        assert_eq!(after[2].name, "A Third Partner");
    }

    #[tokio::test]
    async fn time_entry_create_and_retrieve() {
        let pool = test_pool().await;
        // Seed required FK rows
        sqlx::query(
            "INSERT INTO clients (id, name, type) VALUES ('c1', 'ACME Corp', 'Company')"
        ).execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO matters (id, client_id, title, matter_type, jurisdiction, opened_date)
             VALUES ('m1', 'c1', 'TM Filing', 'Trademark', 'India', '2026-01-01')"
        ).execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO users (id, name, email, role, password_hash)
             VALUES ('u1', 'Test User', 'test@persist.in', 'Partner', 'hash')"
        ).execute(&pool).await.unwrap();

        let entry = create_time_entry(
            &pool, "te1", "m1", "u1", "2026-04-15",
            2.5, "Draft trademark application", "L300", 8000.0, true,
        ).await.unwrap();

        assert_eq!(entry.id, "te1");
        assert!((entry.hours - 2.5).abs() < f64::EPSILON);
        assert_eq!(entry.activity_code, "L300");
        assert_eq!(entry.is_invoiced, 0);
    }

    #[tokio::test]
    async fn time_entry_cannot_be_deleted_when_invoiced() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO clients (id, name, type) VALUES ('c2', 'XYZ Ltd', 'Company')"
        ).execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO matters (id, client_id, title, matter_type, jurisdiction, opened_date)
             VALUES ('m2', 'c2', 'Patent', 'Patent', 'India', '2026-01-01')"
        ).execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO users (id, name, email, role, password_hash)
             VALUES ('u2', 'Another User', 'u2@persist.in', 'Partner', 'hash')"
        ).execute(&pool).await.unwrap();

        // Create entry then mark it invoiced directly
        create_time_entry(
            &pool, "te2", "m2", "u2", "2026-04-15",
            1.0, "Research", "L200", 8000.0, true,
        ).await.unwrap();
        sqlx::query("UPDATE time_entries SET is_invoiced = 1 WHERE id = 'te2'")
            .execute(&pool).await.unwrap();

        let err = delete_time_entry(&pool, "te2").await;
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("invoiced"));
    }

    #[tokio::test]
    async fn invoice_sequence_increments() {
        let pool = test_pool().await;
        let n1 = next_invoice_number(&pool, 2026).await.unwrap();
        let n2 = next_invoice_number(&pool, 2026).await.unwrap();
        assert_eq!(n1, 1);
        assert_eq!(n2, 2);
    }

    #[tokio::test]
    async fn payment_updates_invoice_status_to_paid() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO clients (id, name, type) VALUES ('c3', 'Biz Corp', 'Company')"
        ).execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO users (id, name, email, role, password_hash)
             VALUES ('u3', 'Partner One', 'p@persist.in', 'Partner', 'hash')"
        ).execute(&pool).await.unwrap();

        // Create invoice
        sqlx::query(
            "INSERT INTO invoices
             (id, client_id, matter_ids, invoice_date, subtotal, cgst_amount, sgst_amount,
              igst_amount, total_with_tax, gst_type, created_by)
             VALUES ('INV-2026-0001','c3','[]','2026-04-15',10000.0,900.0,900.0,0.0,11800.0,'Intra','u3')"
        ).execute(&pool).await.unwrap();

        record_payment(
            &pool, "p1", "INV-2026-0001", 11800.0,
            "2026-04-20", "BankTransfer", Some("UTR123"), None, "u3",
        ).await.unwrap();

        let inv = get_invoice(&pool, "INV-2026-0001").await.unwrap();
        assert_eq!(inv.status, "Paid");
        assert!((inv.amount_paid - 11800.0).abs() < f64::EPSILON);
    }
}
