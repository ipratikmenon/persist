use sqlx::SqlitePool;
use crate::commands::matters::{
    Matter, MatterSummary, MatterParty, CreateMatterInput, UpdateMatterInput, MatterFilter,
};

// ---------------------------------------------------------------------------
// Row types — map directly to SQLite columns (snake_case)
// ---------------------------------------------------------------------------

#[derive(Debug, sqlx::FromRow)]
pub struct MatterRow {
    pub id:                     String,
    pub client_id:              String,
    pub title:                  String,
    pub matter_type:            String,
    pub sub_type:               Option<String>,
    pub status:                 String,
    pub priority:               String,
    pub responsible_partner_id: Option<String>,
    pub forum:                  Option<String>,
    pub jurisdiction:           String,
    pub opened_date:            String,
    pub target_close_date:      Option<String>,
    pub internal_notes:         Option<String>,
    pub client_notes:           Option<String>,
    pub tags:                   String,             // JSON array text
    pub linked_matter_ids:      String,             // JSON array text
    pub created_at:             String,
    pub updated_at:             String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct MatterSummaryRow {
    pub id:                   String,
    pub title:                String,
    pub client_name:          String,
    pub matter_type:          String,
    pub status:               String,
    pub priority:             String,
    pub responsible_attorney: Option<String>,
    pub next_deadline_date:   Option<String>,
    pub next_deadline_event:  Option<String>,
    pub updated_at:           String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct MatterPartyRow {
    pub id:         String,
    pub matter_id:  String,
    pub user_id:    String,
    pub role:       String,
    pub is_primary: i64,
    pub added_at:   String,
}

// ---------------------------------------------------------------------------
// Row → IPC struct conversions
// ---------------------------------------------------------------------------

fn parse_json_array(s: &str) -> Vec<String> {
    serde_json::from_str(s).unwrap_or_default()
}

pub fn matter_row_to_ipc(row: MatterRow, parties: Vec<MatterParty>) -> Matter {
    Matter {
        id:                     row.id,
        client_id:              row.client_id,
        title:                  row.title,
        matter_type:            row.matter_type,
        sub_type:               row.sub_type,
        status:                 row.status,
        priority:               row.priority,
        responsible_partner_id: row.responsible_partner_id,
        forum:                  row.forum,
        jurisdiction:           row.jurisdiction,
        opened_date:            row.opened_date,
        target_close_date:      row.target_close_date,
        internal_notes:         row.internal_notes,
        client_notes:           row.client_notes,
        tags:                   parse_json_array(&row.tags),
        linked_matter_ids:      parse_json_array(&row.linked_matter_ids),
        parties,
        created_at:             row.created_at,
        updated_at:             row.updated_at,
    }
}

fn summary_row_to_ipc(row: MatterSummaryRow) -> MatterSummary {
    MatterSummary {
        id:                   row.id,
        title:                row.title,
        client_name:          row.client_name,
        matter_type:          row.matter_type,
        status:               row.status,
        priority:             row.priority,
        responsible_attorney: row.responsible_attorney,
        next_deadline_date:   row.next_deadline_date,
        next_deadline_event:  row.next_deadline_event,
        updated_at:           row.updated_at,
    }
}

// ---------------------------------------------------------------------------
// Sequences — P&P-YYYY-TYPE-NNNN generation
// ---------------------------------------------------------------------------

pub async fn next_seq(pool: &SqlitePool, type_code: &str, year: i32) -> anyhow::Result<u32> {
    let key = format!("{type_code}-{year}");

    // Upsert: create row if missing, then increment
    sqlx::query(
        "INSERT INTO sequences (key, next_val) VALUES (?, 1)
         ON CONFLICT(key) DO UPDATE SET next_val = next_val + 1"
    )
    .bind(&key)
    .execute(pool)
    .await?;

    let val: i64 = sqlx::query_scalar("SELECT next_val FROM sequences WHERE key = ?")
        .bind(&key)
        .fetch_one(pool)
        .await?;

    Ok(val as u32)
}

// ---------------------------------------------------------------------------
// Matter queries
// ---------------------------------------------------------------------------

pub async fn get_by_id(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Matter>> {
    let Some(row) = sqlx::query_as::<_, MatterRow>(
        "SELECT id, client_id, title, matter_type, sub_type, status, priority,
                responsible_partner_id, forum, jurisdiction, opened_date,
                target_close_date, internal_notes, client_notes, tags,
                linked_matter_ids, created_at, updated_at
         FROM matters WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };

    let parties = get_parties(pool, id).await?;
    Ok(Some(matter_row_to_ipc(row, parties)))
}

pub async fn list(pool: &SqlitePool, filter: &MatterFilter) -> anyhow::Result<Vec<MatterSummary>> {
    // Build query dynamically — sqlx QueryBuilder
    let mut qb = sqlx::QueryBuilder::new(
        "SELECT m.id, m.title, c.name AS client_name, m.matter_type,
                m.status, m.priority, m.updated_at,
                NULL AS responsible_attorney,
                NULL AS next_deadline_date,
                NULL AS next_deadline_event
         FROM matters m
         JOIN clients c ON m.client_id = c.id
         WHERE 1=1"
    );

    if let Some(statuses) = &filter.status {
        if !statuses.is_empty() {
            qb.push(" AND m.status IN (");
            let mut sep = qb.separated(", ");
            for s in statuses { sep.push_bind(s); }
            qb.push(")");
        }
    }

    if let Some(types) = &filter.matter_type {
        if !types.is_empty() {
            qb.push(" AND m.matter_type IN (");
            let mut sep = qb.separated(", ");
            for t in types { sep.push_bind(t); }
            qb.push(")");
        }
    }

    if let Some(user_id) = &filter.responsible_user_id {
        qb.push(" AND m.responsible_partner_id = ");
        qb.push_bind(user_id);
    }

    if let Some(client_id) = &filter.client_id {
        qb.push(" AND m.client_id = ");
        qb.push_bind(client_id);
    }

    if let Some(priorities) = &filter.priority {
        if !priorities.is_empty() {
            qb.push(" AND m.priority IN (");
            let mut sep = qb.separated(", ");
            for p in priorities { sep.push_bind(p); }
            qb.push(")");
        }
    }

    qb.push(" ORDER BY m.updated_at DESC");

    let rows = qb
        .build_query_as::<MatterSummaryRow>()
        .fetch_all(pool)
        .await?;

    Ok(rows.into_iter().map(summary_row_to_ipc).collect())
}

pub async fn create(
    pool: &SqlitePool,
    id: &str,
    input: &CreateMatterInput,
) -> anyhow::Result<Matter> {
    let tags = serde_json::to_string(&input.tags.as_deref().unwrap_or(&[]))?;
    let linked = "[]";

    sqlx::query(
        "INSERT INTO matters (
             id, client_id, title, matter_type, sub_type, status, priority,
             forum, jurisdiction, opened_date, target_close_date,
             internal_notes, client_notes, tags, linked_matter_ids
         ) VALUES (?, ?, ?, ?, ?, 'Active', ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(&input.client_id)
    .bind(&input.title)
    .bind(&input.matter_type)
    .bind(&input.sub_type)
    .bind(input.priority.as_deref().unwrap_or("Normal"))
    .bind(&input.forum)
    .bind(input.jurisdiction.as_deref().unwrap_or("India"))
    .bind(&input.opened_date)
    .bind(&input.target_close_date)
    .bind(&input.internal_notes)
    .bind(&input.client_notes)
    .bind(&tags)
    .bind(linked)
    .execute(pool)
    .await?;

    get_by_id(pool, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Matter not found after insert"))
}

pub async fn update(
    pool: &SqlitePool,
    id: &str,
    input: &UpdateMatterInput,
) -> anyhow::Result<Matter> {
    let tags_json = input.tags.as_ref().map(|t| serde_json::to_string(t)).transpose()?;
    let linked_json = input.linked_matter_ids.as_ref()
        .map(|t| serde_json::to_string(t))
        .transpose()?;

    sqlx::query(
        "UPDATE matters SET
             title             = COALESCE(?, title),
             sub_type          = COALESCE(?, sub_type),
             priority          = COALESCE(?, priority),
             forum             = COALESCE(?, forum),
             target_close_date = COALESCE(?, target_close_date),
             internal_notes    = COALESCE(?, internal_notes),
             client_notes      = COALESCE(?, client_notes),
             tags              = COALESCE(?, tags),
             linked_matter_ids = COALESCE(?, linked_matter_ids),
             updated_at        = datetime('now')
         WHERE id = ?"
    )
    .bind(&input.title)
    .bind(&input.sub_type)
    .bind(&input.priority)
    .bind(&input.forum)
    .bind(&input.target_close_date)
    .bind(&input.internal_notes)
    .bind(&input.client_notes)
    .bind(&tags_json)
    .bind(&linked_json)
    .bind(id)
    .execute(pool)
    .await?;

    get_by_id(pool, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Matter not found"))
}

pub async fn update_status(
    pool: &SqlitePool,
    id: &str,
    status: &str,
) -> anyhow::Result<Matter> {
    sqlx::query(
        "UPDATE matters SET status = ?, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(status)
    .bind(id)
    .execute(pool)
    .await?;

    get_by_id(pool, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Matter not found"))
}

pub async fn search(pool: &SqlitePool, query: &str) -> anyhow::Result<Vec<MatterSummary>> {
    let pattern = format!("%{query}%");
    let rows = sqlx::query_as::<_, MatterSummaryRow>(
        "SELECT m.id, m.title, c.name AS client_name, m.matter_type,
                m.status, m.priority, m.updated_at,
                NULL AS responsible_attorney,
                NULL AS next_deadline_date,
                NULL AS next_deadline_event
         FROM matters m
         JOIN clients c ON m.client_id = c.id
         WHERE m.title LIKE ? OR c.name LIKE ? OR m.id LIKE ?
         ORDER BY m.updated_at DESC
         LIMIT 50"
    )
    .bind(&pattern)
    .bind(&pattern)
    .bind(&pattern)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(summary_row_to_ipc).collect())
}

// ---------------------------------------------------------------------------
// Party queries
// ---------------------------------------------------------------------------

pub async fn get_parties(pool: &SqlitePool, matter_id: &str) -> anyhow::Result<Vec<MatterParty>> {
    let rows = sqlx::query_as::<_, MatterPartyRow>(
        "SELECT id, matter_id, user_id, role, is_primary, added_at
         FROM matter_parties WHERE matter_id = ? ORDER BY is_primary DESC"
    )
    .bind(matter_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| MatterParty {
        user_id:    r.user_id,
        role:       r.role,
        is_primary: r.is_primary != 0,
        name:       String::new(), // populated by command layer once users table exists
    }).collect())
}

pub async fn add_party(
    pool: &SqlitePool,
    party_id: &str,
    matter_id: &str,
    user_id: &str,
    role: &str,
    is_primary: bool,
) -> anyhow::Result<MatterParty> {
    // If is_primary, demote all current primaries first
    if is_primary {
        sqlx::query(
            "UPDATE matter_parties SET is_primary = 0 WHERE matter_id = ?"
        )
        .bind(matter_id)
        .execute(pool)
        .await?;
    }

    sqlx::query(
        "INSERT INTO matter_parties (id, matter_id, user_id, role, is_primary)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(matter_id, user_id) DO UPDATE
         SET role = excluded.role, is_primary = excluded.is_primary"
    )
    .bind(party_id)
    .bind(matter_id)
    .bind(user_id)
    .bind(role)
    .bind(if is_primary { 1i64 } else { 0i64 })
    .execute(pool)
    .await?;

    Ok(MatterParty {
        user_id:    user_id.to_string(),
        role:       role.to_string(),
        is_primary,
        name:       String::new(),
    })
}

pub async fn remove_party(
    pool: &SqlitePool,
    matter_id: &str,
    user_id: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "DELETE FROM matter_parties WHERE matter_id = ? AND user_id = ?"
    )
    .bind(matter_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn has_primary_party(pool: &SqlitePool, matter_id: &str) -> anyhow::Result<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM matter_parties WHERE matter_id = ? AND is_primary = 1"
    )
    .bind(matter_id)
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::matters::CreateMatterInput;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE sequences (key TEXT PRIMARY KEY, next_val INTEGER NOT NULL DEFAULT 1)"
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE clients (
                id TEXT PRIMARY KEY, name TEXT NOT NULL,
                type TEXT NOT NULL DEFAULT 'Individual', email TEXT, phone TEXT,
                address TEXT, gstin TEXT, pan TEXT, notes TEXT,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at DATETIME NOT NULL DEFAULT (datetime('now')),
                updated_at DATETIME NOT NULL DEFAULT (datetime('now'))
            )"
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE matters (
                id TEXT PRIMARY KEY, client_id TEXT NOT NULL, title TEXT NOT NULL,
                matter_type TEXT NOT NULL, sub_type TEXT,
                status TEXT NOT NULL DEFAULT 'Active',
                priority TEXT NOT NULL DEFAULT 'Normal',
                responsible_partner_id TEXT, forum TEXT,
                jurisdiction TEXT NOT NULL DEFAULT 'India',
                opened_date DATE NOT NULL, target_close_date DATE,
                internal_notes TEXT, client_notes TEXT,
                tags TEXT NOT NULL DEFAULT '[]',
                linked_matter_ids TEXT NOT NULL DEFAULT '[]',
                created_at DATETIME NOT NULL DEFAULT (datetime('now')),
                updated_at DATETIME NOT NULL DEFAULT (datetime('now'))
            )"
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE matter_parties (
                id TEXT PRIMARY KEY, matter_id TEXT NOT NULL,
                user_id TEXT NOT NULL, role TEXT NOT NULL,
                is_primary INTEGER NOT NULL DEFAULT 0,
                added_at DATETIME NOT NULL DEFAULT (datetime('now')),
                UNIQUE(matter_id, user_id)
            )"
        ).execute(&pool).await.unwrap();

        // Seed a client
        sqlx::query(
            "INSERT INTO clients (id, name, type) VALUES ('client-1', 'Test Client', 'Company')"
        ).execute(&pool).await.unwrap();

        pool
    }

    #[tokio::test]
    async fn test_sequence_increments() {
        let pool = test_pool().await;
        let first  = next_seq(&pool, "TM", 2026).await.unwrap();
        let second = next_seq(&pool, "TM", 2026).await.unwrap();
        let other  = next_seq(&pool, "PAT", 2026).await.unwrap();
        assert_eq!(first,  1);
        assert_eq!(second, 2);
        assert_eq!(other,  1); // separate sequence per type
    }

    #[tokio::test]
    async fn test_create_and_get_matter() {
        let pool = test_pool().await;
        let input = CreateMatterInput {
            client_id:        "client-1".into(),
            title:            "Petalveda TM Application".into(),
            matter_type:      "Trademark".into(),
            sub_type:         Some("TM Application".into()),
            priority:         Some("High".into()),
            forum:            Some("Trade Marks Registry".into()),
            jurisdiction:     None,
            opened_date:      "2026-04-11".into(),
            target_close_date: None,
            internal_notes:   Some("Confidential attorney notes".into()),
            client_notes:     None,
            tags:             Some(vec!["priority".into()]),
        };

        let matter = create(&pool, "P&P-2026-TM-0001", &input).await.unwrap();
        assert_eq!(matter.id, "P&P-2026-TM-0001");
        assert_eq!(matter.title, "Petalveda TM Application");
        assert_eq!(matter.status, "Active");
        assert_eq!(matter.tags, vec!["priority"]);

        let fetched = get_by_id(&pool, "P&P-2026-TM-0001").await.unwrap().unwrap();
        assert_eq!(fetched.matter_type, "Trademark");
        assert_eq!(fetched.internal_notes, Some("Confidential attorney notes".into()));
    }

    #[tokio::test]
    async fn test_status_update() {
        let pool = test_pool().await;
        let input = CreateMatterInput {
            client_id: "client-1".into(), title: "Test".into(),
            matter_type: "Patent".into(), sub_type: None,
            priority: None, forum: None, jurisdiction: None,
            opened_date: "2026-04-11".into(), target_close_date: None,
            internal_notes: None, client_notes: None, tags: None,
        };
        create(&pool, "M-001", &input).await.unwrap();
        let updated = update_status(&pool, "M-001", "OnHold").await.unwrap();
        assert_eq!(updated.status, "OnHold");
    }

    #[tokio::test]
    async fn test_party_management() {
        let pool = test_pool().await;
        let input = CreateMatterInput {
            client_id: "client-1".into(), title: "Party Test".into(),
            matter_type: "Corporate".into(), sub_type: None,
            priority: None, forum: None, jurisdiction: None,
            opened_date: "2026-04-11".into(), target_close_date: None,
            internal_notes: None, client_notes: None, tags: None,
        };
        create(&pool, "M-002", &input).await.unwrap();

        add_party(&pool, "p1", "M-002", "user-sree", "Partner", true).await.unwrap();
        add_party(&pool, "p2", "M-002", "user-kajal", "Associate", false).await.unwrap();

        let parties = get_parties(&pool, "M-002").await.unwrap();
        assert_eq!(parties.len(), 2);
        assert!(parties.iter().any(|p| p.user_id == "user-sree" && p.is_primary));
        assert!(parties.iter().any(|p| p.user_id == "user-kajal" && !p.is_primary));

        remove_party(&pool, "M-002", "user-kajal").await.unwrap();
        let parties = get_parties(&pool, "M-002").await.unwrap();
        assert_eq!(parties.len(), 1);
    }
}
