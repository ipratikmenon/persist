// IP Asset query layer — Phase 1 Module 2 extended (B02).
// Runtime sqlx queries (no DATABASE_URL at compile time).
//
// `classes` is stored as a JSON array of Nice/Locarno class numbers ('[9,42]')
// and surfaced to Deck as Vec<i64>. Parsing is tolerant: a malformed value
// degrades to an empty list rather than failing the whole read, so one bad row
// can never take out an attorney's asset list.

use crate::commands::ip_assets::{CreateIpAssetInput, IpAsset, UpdateIpAssetInput};
use sqlx::SqlitePool;

const SELECT_COLUMNS: &str = "id, matter_id, asset_type, title, application_number,
     registration_number, filing_date, priority_date, grant_date, registration_date,
     expiry_date, applicant_entity_type, jurisdiction, classes, status, notes,
     created_at, updated_at";

/// Denormalised renewal view for the firm-wide dashboard.
#[derive(Debug, sqlx::FromRow)]
pub struct RenewalRow {
    pub id:                  String,
    pub matter_id:           String,
    pub asset_type:          String,
    pub title:               String,
    pub registration_number: Option<String>,
    pub expiry_date:         Option<String>,
    pub status:              String,
    pub jurisdiction:        String,
    pub matter_title:        String,
    pub client_name:         String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct IpAssetRow {
    pub id:                    String,
    pub matter_id:             String,
    pub asset_type:            String,
    pub title:                 String,
    pub application_number:    Option<String>,
    pub registration_number:   Option<String>,
    pub filing_date:           Option<String>,
    pub priority_date:         Option<String>,
    pub grant_date:            Option<String>,
    pub registration_date:     Option<String>,
    pub expiry_date:           Option<String>,
    pub applicant_entity_type: String,
    pub jurisdiction:          String,
    pub classes:               String,
    pub status:                String,
    pub notes:                 Option<String>,
    pub created_at:            String,
    pub updated_at:            String,
}

impl From<IpAssetRow> for IpAsset {
    fn from(r: IpAssetRow) -> Self {
        IpAsset {
            id:                    r.id,
            matter_id:             r.matter_id,
            asset_type:            r.asset_type,
            title:                 r.title,
            application_number:    r.application_number,
            registration_number:   r.registration_number,
            filing_date:           r.filing_date,
            priority_date:         r.priority_date,
            grant_date:            r.grant_date,
            registration_date:     r.registration_date,
            expiry_date:           r.expiry_date,
            applicant_entity_type: r.applicant_entity_type,
            jurisdiction:          r.jurisdiction,
            classes:               parse_classes(&r.classes),
            status:                r.status,
            notes:                 r.notes,
            created_at:            r.created_at,
            updated_at:            r.updated_at,
        }
    }
}

/// Parse the stored JSON class array. Unparseable input yields an empty list.
fn parse_classes(raw: &str) -> Vec<i64> {
    serde_json::from_str::<Vec<i64>>(raw).unwrap_or_else(|_| {
        if !raw.trim().is_empty() && raw.trim() != "[]" {
            log::warn!("ip_assets.classes held unparseable JSON: {raw}");
        }
        Vec::new()
    })
}

/// Serialise class numbers for storage.
fn serialise_classes(classes: &[i64]) -> String {
    serde_json::to_string(classes).unwrap_or_else(|_| "[]".to_string())
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Assets for a matter. Most recently filed first; undated assets last.
pub async fn list_by_matter(pool: &SqlitePool, matter_id: &str) -> anyhow::Result<Vec<IpAsset>> {
    let sql = format!(
        "SELECT {SELECT_COLUMNS} FROM ip_assets
         WHERE matter_id = ?
         ORDER BY filing_date DESC NULLS LAST, created_at DESC"
    );
    let rows = sqlx::query_as::<_, IpAssetRow>(&sql)
        .bind(matter_id)
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn get(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<IpAsset>> {
    let sql = format!("SELECT {SELECT_COLUMNS} FROM ip_assets WHERE id = ?");
    let row = sqlx::query_as::<_, IpAssetRow>(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(Into::into))
}

pub async fn create(
    pool: &SqlitePool,
    id: &str,
    input: CreateIpAssetInput,
) -> anyhow::Result<IpAsset> {
    let classes = serialise_classes(&input.classes.unwrap_or_default());

    sqlx::query(
        "INSERT INTO ip_assets (
             id, matter_id, asset_type, title, application_number, registration_number,
             filing_date, priority_date, grant_date, registration_date, expiry_date,
             applicant_entity_type, jurisdiction, classes, status, notes
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                   COALESCE(?, 'Company'), COALESCE(?, 'India'), ?, COALESCE(?, 'Pending'), ?)",
    )
    .bind(id)
    .bind(&input.matter_id)
    .bind(&input.asset_type)
    .bind(input.title.trim())
    .bind(&input.application_number)
    .bind(&input.registration_number)
    .bind(&input.filing_date)
    .bind(&input.priority_date)
    .bind(&input.grant_date)
    .bind(&input.registration_date)
    .bind(&input.expiry_date)
    .bind(&input.applicant_entity_type)
    .bind(&input.jurisdiction)
    .bind(&classes)
    .bind(&input.status)
    .bind(&input.notes)
    .execute(pool)
    .await?;

    get(pool, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("ip_asset not found after insert"))
}

/// Partial update — NULL inputs leave the existing value untouched (COALESCE).
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    input: UpdateIpAssetInput,
) -> anyhow::Result<Option<IpAsset>> {
    let classes = input.classes.as_ref().map(|c| serialise_classes(c));
    let title = input.title.as_ref().map(|t| t.trim().to_string());

    sqlx::query(
        "UPDATE ip_assets SET
             asset_type            = COALESCE(?, asset_type),
             title                 = COALESCE(?, title),
             application_number    = COALESCE(?, application_number),
             registration_number   = COALESCE(?, registration_number),
             filing_date           = COALESCE(?, filing_date),
             priority_date         = COALESCE(?, priority_date),
             grant_date            = COALESCE(?, grant_date),
             registration_date     = COALESCE(?, registration_date),
             expiry_date           = COALESCE(?, expiry_date),
             applicant_entity_type = COALESCE(?, applicant_entity_type),
             jurisdiction          = COALESCE(?, jurisdiction),
             classes               = COALESCE(?, classes),
             status                = COALESCE(?, status),
             notes                 = COALESCE(?, notes),
             updated_at            = datetime('now')
         WHERE id = ?",
    )
    .bind(&input.asset_type)
    .bind(&title)
    .bind(&input.application_number)
    .bind(&input.registration_number)
    .bind(&input.filing_date)
    .bind(&input.priority_date)
    .bind(&input.grant_date)
    .bind(&input.registration_date)
    .bind(&input.expiry_date)
    .bind(&input.applicant_entity_type)
    .bind(&input.jurisdiction)
    .bind(&classes)
    .bind(&input.status)
    .bind(&input.notes)
    .bind(id)
    .execute(pool)
    .await?;

    get(pool, id).await
}

pub async fn delete(pool: &SqlitePool, id: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM ip_assets WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Assets with a renewal date inside `within_days`, plus already-lapsed ones.
/// Drives the Renewal Dashboard, which is firm-wide rather than per-matter —
/// a renewal missed because nobody opened that matter is still a lost right.
pub async fn list_upcoming_renewals(
    pool: &SqlitePool,
    within_days: i64,
) -> anyhow::Result<Vec<RenewalRow>> {
    let horizon = format!("+{within_days} days");

    let rows = sqlx::query_as::<_, RenewalRow>(
        "SELECT a.id, a.matter_id, a.asset_type, a.title, a.registration_number,
                a.expiry_date, a.status, a.jurisdiction,
                m.title AS matter_title, c.name AS client_name
         FROM ip_assets a
         JOIN matters m ON m.id = a.matter_id
         JOIN clients c ON c.id = m.client_id
         WHERE a.expiry_date IS NOT NULL
           AND a.expiry_date <= date('now', ?)
           AND a.status NOT IN ('Abandoned','Cancelled')
         ORDER BY a.expiry_date ASC",
    )
    .bind(&horizon)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// The raw row, for the sync projection.
pub async fn get_row(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<IpAssetRow>> {
    let sql = format!("SELECT {SELECT_COLUMNS} FROM ip_assets WHERE id = ?");
    let row = sqlx::query_as::<_, IpAssetRow>(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

/// How many deadlines point at this asset — guards deletion.
pub async fn count_linked_deadlines(pool: &SqlitePool, id: &str) -> anyhow::Result<i64> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM deadlines WHERE ip_asset_id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(n)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        sqlx::query(
            "CREATE TABLE ip_assets (
                id TEXT PRIMARY KEY,
                matter_id TEXT NOT NULL,
                asset_type TEXT NOT NULL,
                title TEXT NOT NULL,
                application_number TEXT,
                registration_number TEXT,
                filing_date DATE,
                priority_date DATE,
                grant_date DATE,
                registration_date DATE,
                expiry_date DATE,
                applicant_entity_type TEXT NOT NULL DEFAULT 'Company',
                jurisdiction TEXT NOT NULL DEFAULT 'India',
                classes TEXT NOT NULL DEFAULT '[]',
                status TEXT NOT NULL DEFAULT 'Pending',
                notes TEXT,
                created_at DATETIME NOT NULL DEFAULT (datetime('now')),
                updated_at DATETIME NOT NULL DEFAULT (datetime('now'))
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE deadlines (
                id TEXT PRIMARY KEY,
                matter_id TEXT NOT NULL,
                docketing_event TEXT NOT NULL,
                due_date DATE NOT NULL,
                ip_asset_id TEXT
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    fn base_input(matter_id: &str, title: &str) -> CreateIpAssetInput {
        CreateIpAssetInput {
            matter_id:             matter_id.to_string(),
            asset_type:            "Trademark".to_string(),
            title:                 title.to_string(),
            application_number:    None,
            registration_number:   None,
            filing_date:           None,
            priority_date:         None,
            grant_date:            None,
            registration_date:     None,
            expiry_date:           None,
            applicant_entity_type: None,
            jurisdiction:          None,
            classes:               None,
            status:                None,
            notes:                 None,
        }
    }

    #[tokio::test]
    async fn create_applies_defaults() {
        let pool = test_pool().await;
        let asset = create(&pool, "ip-1", base_input("m-1", "PETALVEDA"))
            .await
            .unwrap();

        assert_eq!(asset.title, "PETALVEDA");
        assert_eq!(asset.status, "Pending");
        assert_eq!(asset.jurisdiction, "India");
        assert_eq!(asset.applicant_entity_type, "Company");
        assert!(asset.classes.is_empty());
    }

    #[tokio::test]
    async fn classes_round_trip_as_numbers() {
        let pool = test_pool().await;
        let mut input = base_input("m-1", "PETALVEDA");
        input.classes = Some(vec![3, 5, 44]);

        let asset = create(&pool, "ip-2", input).await.unwrap();
        assert_eq!(asset.classes, vec![3, 5, 44]);

        // ...and survives a re-read from the DB.
        let fetched = get(&pool, "ip-2").await.unwrap().unwrap();
        assert_eq!(fetched.classes, vec![3, 5, 44]);
    }

    #[tokio::test]
    async fn malformed_classes_degrade_to_empty() {
        let pool = test_pool().await;
        create(&pool, "ip-3", base_input("m-1", "BROKEN")).await.unwrap();
        sqlx::query("UPDATE ip_assets SET classes = 'not json' WHERE id = 'ip-3'")
            .execute(&pool)
            .await
            .unwrap();

        // The row still reads — it does not poison the list.
        let asset = get(&pool, "ip-3").await.unwrap().unwrap();
        assert!(asset.classes.is_empty());
    }

    #[tokio::test]
    async fn update_only_touches_supplied_fields() {
        let pool = test_pool().await;
        let mut input = base_input("m-1", "PETALVEDA");
        input.application_number = Some("TM-4455".to_string());
        create(&pool, "ip-4", input).await.unwrap();

        let patch = UpdateIpAssetInput {
            asset_type:            None,
            title:                 None,
            application_number:    None,
            registration_number:   None,
            filing_date:           None,
            priority_date:         None,
            grant_date:            None,
            registration_date:     None,
            expiry_date:           None,
            applicant_entity_type: None,
            jurisdiction:          None,
            classes:               None,
            status:                Some("Registered".to_string()),
            notes:                 None,
        };
        let updated = update(&pool, "ip-4", patch).await.unwrap().unwrap();

        assert_eq!(updated.status, "Registered");
        // Untouched fields survive.
        assert_eq!(updated.title, "PETALVEDA");
        assert_eq!(updated.application_number.as_deref(), Some("TM-4455"));
    }

    #[tokio::test]
    async fn update_unknown_id_returns_none() {
        let pool = test_pool().await;
        let patch = UpdateIpAssetInput {
            asset_type: None, title: None, application_number: None,
            registration_number: None, filing_date: None, priority_date: None,
            grant_date: None, registration_date: None, expiry_date: None,
            applicant_entity_type: None, jurisdiction: None, classes: None,
            status: Some("Lapsed".to_string()), notes: None,
        };
        assert!(update(&pool, "ghost", patch).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_is_scoped_to_the_matter() {
        let pool = test_pool().await;
        create(&pool, "a", base_input("m-1", "ALPHA")).await.unwrap();
        create(&pool, "b", base_input("m-1", "BETA")).await.unwrap();
        create(&pool, "c", base_input("m-2", "GAMMA")).await.unwrap();

        let assets = list_by_matter(&pool, "m-1").await.unwrap();
        assert_eq!(assets.len(), 2);
        assert!(assets.iter().all(|a| a.matter_id == "m-1"));
    }

    #[tokio::test]
    async fn linked_deadlines_are_counted() {
        let pool = test_pool().await;
        create(&pool, "ip-5", base_input("m-1", "LINKED")).await.unwrap();
        assert_eq!(count_linked_deadlines(&pool, "ip-5").await.unwrap(), 0);

        sqlx::query(
            "INSERT INTO deadlines (id, matter_id, docketing_event, due_date, ip_asset_id)
             VALUES ('d-1', 'm-1', 'Renewal', '2027-01-01', 'ip-5')",
        )
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(count_linked_deadlines(&pool, "ip-5").await.unwrap(), 1);
    }

    #[tokio::test]
    async fn delete_removes_the_asset() {
        let pool = test_pool().await;
        create(&pool, "ip-6", base_input("m-1", "GONE")).await.unwrap();
        delete(&pool, "ip-6").await.unwrap();
        assert!(get(&pool, "ip-6").await.unwrap().is_none());
    }
}
