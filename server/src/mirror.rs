// Mirror writes — applying a desktop push to PostgreSQL.
//
// This module contains NO decisions about what may be shared. That was settled
// on the desktop by projection.rs. Here we only persist what arrived, which is
// why every function takes an already-projected type: there is no path in this
// crate that can read a desktop row and decide for itself.

use crate::types::*;
use anyhow::{Context, Result};
use rust_decimal::Decimal;
use sqlx::PgPool;

/// f64 (SQLite REAL, as sent) -> NUMERIC(14,2).
/// The desktop already rounded; this is the type bridge, not a second rounding.
fn money(v: f64) -> Decimal {
    Decimal::from_f64_retain(v)
        .map(|d| d.round_dp(2))
        .unwrap_or_default()
}

pub async fn upsert_client(pool: &PgPool, c: &ClientPublic) -> Result<()> {
    sqlx::query(
        "INSERT INTO mirror.clients (id, name, updated_at)
         VALUES ($1, $2, now())
         ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, updated_at = now()",
    )
    .bind(&c.id)
    .bind(&c.name)
    .execute(pool)
    .await
    .context("upsert client")?;
    Ok(())
}

pub async fn upsert_portal_user(pool: &PgPool, u: &PortalUserPublic) -> Result<()> {
    sqlx::query(
        "INSERT INTO mirror.portal_users
             (id, client_id, full_name, email, phone, status, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, now())
         ON CONFLICT (id) DO UPDATE SET
             client_id = EXCLUDED.client_id,
             full_name = EXCLUDED.full_name,
             email     = EXCLUDED.email,
             phone     = EXCLUDED.phone,
             status    = EXCLUDED.status,
             updated_at = now()",
    )
    .bind(&u.id).bind(&u.client_id).bind(&u.full_name)
    .bind(&u.email).bind(&u.phone).bind(&u.status)
    .execute(pool).await.context("upsert portal user")?;
    Ok(())
}

pub async fn upsert_matter(pool: &PgPool, m: &MatterPublic) -> Result<()> {
    sqlx::query(
        "INSERT INTO mirror.matters_public
             (id, client_id, title, matter_type, status, opened_date, forum,
              jurisdiction, client_notes, responsible_attorney,
              next_deadline_date, next_deadline_event, updated_at)
         VALUES ($1,$2,$3,$4,$5,$6::date,$7,$8,$9,$10,$11::date,$12, now())
         ON CONFLICT (id) DO UPDATE SET
             client_id            = EXCLUDED.client_id,
             title                = EXCLUDED.title,
             matter_type          = EXCLUDED.matter_type,
             status               = EXCLUDED.status,
             opened_date          = EXCLUDED.opened_date,
             forum                = EXCLUDED.forum,
             jurisdiction         = EXCLUDED.jurisdiction,
             client_notes         = EXCLUDED.client_notes,
             responsible_attorney = EXCLUDED.responsible_attorney,
             next_deadline_date   = EXCLUDED.next_deadline_date,
             next_deadline_event  = EXCLUDED.next_deadline_event,
             updated_at           = now()",
    )
    .bind(&m.id).bind(&m.client_id).bind(&m.title).bind(&m.matter_type)
    .bind(&m.status).bind(&m.opened_date).bind(&m.forum).bind(&m.jurisdiction)
    .bind(&m.client_notes).bind(&m.responsible_attorney)
    .bind(&m.next_deadline_date).bind(&m.next_deadline_event)
    .execute(pool).await.context("upsert matter")?;
    Ok(())
}

pub async fn upsert_deadline(pool: &PgPool, d: &DeadlinePublic) -> Result<()> {
    sqlx::query(
        "INSERT INTO mirror.deadlines_public
             (id, client_id, matter_id, docketing_event, due_date, status, updated_at)
         VALUES ($1,$2,$3,$4,$5::date,$6, now())
         ON CONFLICT (id) DO UPDATE SET
             client_id       = EXCLUDED.client_id,
             matter_id       = EXCLUDED.matter_id,
             docketing_event = EXCLUDED.docketing_event,
             due_date        = EXCLUDED.due_date,
             status          = EXCLUDED.status,
             updated_at      = now()",
    )
    .bind(&d.id).bind(&d.client_id).bind(&d.matter_id)
    .bind(&d.docketing_event).bind(&d.due_date).bind(&d.status)
    .execute(pool).await.context("upsert deadline")?;
    Ok(())
}

pub async fn upsert_ip_asset(pool: &PgPool, a: &IpAssetPublic) -> Result<()> {
    let classes = serde_json::to_value(&a.classes).unwrap_or(serde_json::json!([]));

    sqlx::query(
        "INSERT INTO mirror.ip_assets_public
             (id, client_id, matter_id, asset_type, title, application_number,
              registration_number, filing_date, registration_date, expiry_date,
              status, classes, jurisdiction, updated_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8::date,$9::date,$10::date,$11,$12,$13, now())
         ON CONFLICT (id) DO UPDATE SET
             client_id           = EXCLUDED.client_id,
             matter_id           = EXCLUDED.matter_id,
             asset_type          = EXCLUDED.asset_type,
             title               = EXCLUDED.title,
             application_number  = EXCLUDED.application_number,
             registration_number = EXCLUDED.registration_number,
             filing_date         = EXCLUDED.filing_date,
             registration_date   = EXCLUDED.registration_date,
             expiry_date         = EXCLUDED.expiry_date,
             status              = EXCLUDED.status,
             classes             = EXCLUDED.classes,
             jurisdiction        = EXCLUDED.jurisdiction,
             updated_at          = now()",
    )
    .bind(&a.id).bind(&a.client_id).bind(&a.matter_id).bind(&a.asset_type)
    .bind(&a.title).bind(&a.application_number).bind(&a.registration_number)
    .bind(&a.filing_date).bind(&a.registration_date).bind(&a.expiry_date)
    .bind(&a.status).bind(&classes).bind(&a.jurisdiction)
    .execute(pool).await.context("upsert ip asset")?;
    Ok(())
}

pub async fn upsert_invoice(pool: &PgPool, i: &InvoicePublic) -> Result<()> {
    // Belt and braces: projection.rs already withholds drafts. If one somehow
    // arrives, refuse it here rather than writing it and relying on the API to
    // filter later.
    if i.status == "Draft" {
        anyhow::bail!("draft invoices must never reach the mirror");
    }

    sqlx::query(
        "INSERT INTO mirror.invoices_public
             (id, client_id, status, invoice_date, due_date, subtotal,
              cgst_amount, sgst_amount, igst_amount, total_with_tax,
              amount_paid, notes, updated_at)
         VALUES ($1,$2,$3,$4::date,$5::date,$6,$7,$8,$9,$10,$11,$12, now())
         ON CONFLICT (id) DO UPDATE SET
             client_id      = EXCLUDED.client_id,
             status         = EXCLUDED.status,
             invoice_date   = EXCLUDED.invoice_date,
             due_date       = EXCLUDED.due_date,
             subtotal       = EXCLUDED.subtotal,
             cgst_amount    = EXCLUDED.cgst_amount,
             sgst_amount    = EXCLUDED.sgst_amount,
             igst_amount    = EXCLUDED.igst_amount,
             total_with_tax = EXCLUDED.total_with_tax,
             amount_paid    = EXCLUDED.amount_paid,
             notes          = EXCLUDED.notes,
             updated_at     = now()",
    )
    .bind(&i.id).bind(&i.client_id).bind(&i.status)
    .bind(&i.invoice_date).bind(&i.due_date)
    .bind(money(i.subtotal)).bind(money(i.cgst_amount))
    .bind(money(i.sgst_amount)).bind(money(i.igst_amount))
    .bind(money(i.total_with_tax)).bind(money(i.amount_paid))
    .bind(&i.notes)
    .execute(pool).await.context("upsert invoice")?;
    Ok(())
}

/// Tombstone. Un-sharing must actually remove the row, not merely stop
/// refreshing it — otherwise a withdrawn document stays visible for ever.
pub async fn delete_entity(pool: &PgPool, entity_type: &str, id: &str) -> Result<u64> {
    let table = match entity_type {
        "Matter"     => "mirror.matters_public",
        "Deadline"   => "mirror.deadlines_public",
        "IpAsset"    => "mirror.ip_assets_public",
        "Document"   => "mirror.documents_shared",
        "Invoice"    => "mirror.invoices_public",
        "Payment"    => "mirror.payments_public",
        "PortalUser" => "mirror.portal_users",
        other        => anyhow::bail!("unknown entity type for delete: {other}"),
    };

    // Table name comes from the match above, never from the request body.
    let sql = format!("DELETE FROM {table} WHERE id = $1");
    let r = sqlx::query(&sql).bind(id).execute(pool).await
        .with_context(|| format!("delete from {table}"))?;
    Ok(r.rows_affected())
}
