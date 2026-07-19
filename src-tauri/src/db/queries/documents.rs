// Document query layer — Phase 1 Module 3.
// Uses runtime sqlx queries (no DATABASE_URL required at compile time).
// vault_path is NEVER exposed beyond this module — strip it in commands layer.

use sqlx::SqlitePool;

// ---------------------------------------------------------------------------
// Row type (internal — vault_path must NOT reach Deck)
// ---------------------------------------------------------------------------

#[derive(Debug, sqlx::FromRow)]
pub struct DocumentRow {
    pub id: String,
    pub matter_id: String,
    pub filename: String,
    pub category: String,
    pub mime_type: String,
    pub file_size_bytes: i64,
    pub version: i64,
    pub vault_path: String,
    pub uploaded_by: String,
    pub is_shared_with_client: i64,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct CreateDocumentInput<'a> {
    pub id: &'a str,
    pub matter_id: &'a str,
    pub filename: &'a str,
    pub category: &'a str,
    pub mime_type: &'a str,
    pub file_size_bytes: i64,
    pub vault_path: &'a str,
    pub uploaded_by: &'a str,
    pub description: Option<&'a str>,
}

const SELECT_COLS: &str =
    "id, matter_id, filename, category, mime_type, \
     file_size_bytes, version, vault_path, uploaded_by, \
     is_shared_with_client, description, created_at, updated_at";

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

pub async fn list_for_matter(
    pool: &SqlitePool,
    matter_id: &str,
) -> anyhow::Result<Vec<DocumentRow>> {
    let sql = format!(
        "SELECT {} FROM documents WHERE matter_id = ? ORDER BY created_at DESC",
        SELECT_COLS
    );
    let rows = sqlx::query_as::<_, DocumentRow>(&sql)
        .bind(matter_id)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn get_by_id(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<DocumentRow>> {
    let sql = format!("SELECT {} FROM documents WHERE id = ?", SELECT_COLS);
    let row = sqlx::query_as::<_, DocumentRow>(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

pub async fn create(pool: &SqlitePool, input: CreateDocumentInput<'_>) -> anyhow::Result<DocumentRow> {
    sqlx::query(
        "INSERT INTO documents \
            (id, matter_id, filename, category, mime_type, \
             file_size_bytes, vault_path, uploaded_by, description) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(input.id)
    .bind(input.matter_id)
    .bind(input.filename)
    .bind(input.category)
    .bind(input.mime_type)
    .bind(input.file_size_bytes)
    .bind(input.vault_path)
    .bind(input.uploaded_by)
    .bind(input.description)
    .execute(pool)
    .await?;

    let row = get_by_id(pool, input.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("document not found after insert"))?;
    Ok(row)
}

/// Delete a document row. Returns the vault_path so the caller can also
/// remove the encrypted file from disk.
pub async fn delete(pool: &SqlitePool, id: &str) -> anyhow::Result<String> {
    let vault_path: Option<String> =
        sqlx::query_scalar("SELECT vault_path FROM documents WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;

    let path = vault_path.ok_or_else(|| anyhow::anyhow!("document not found: {id}"))?;

    sqlx::query("DELETE FROM documents WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(path)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite");

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS matters (
                id TEXT PRIMARY KEY,
                client_id TEXT NOT NULL,
                title TEXT NOT NULL,
                matter_type TEXT NOT NULL,
                sub_type TEXT,
                status TEXT NOT NULL DEFAULT 'Active',
                priority TEXT NOT NULL DEFAULT 'Normal',
                responsible_partner_id TEXT,
                forum TEXT,
                jurisdiction TEXT NOT NULL DEFAULT 'India',
                opened_date DATE NOT NULL,
                target_close_date DATE,
                internal_notes TEXT,
                client_notes TEXT,
                tags TEXT NOT NULL DEFAULT '[]',
                linked_matter_ids TEXT NOT NULL DEFAULT '[]',
                created_at DATETIME NOT NULL DEFAULT (datetime('now')),
                updated_at DATETIME NOT NULL DEFAULT (datetime('now'))
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS documents (
                id TEXT PRIMARY KEY,
                matter_id TEXT NOT NULL REFERENCES matters(id) ON DELETE CASCADE,
                filename TEXT NOT NULL,
                category TEXT NOT NULL DEFAULT 'Other',
                mime_type TEXT NOT NULL DEFAULT 'application/octet-stream',
                file_size_bytes INTEGER NOT NULL DEFAULT 0,
                version INTEGER NOT NULL DEFAULT 1,
                vault_path TEXT NOT NULL,
                uploaded_by TEXT NOT NULL DEFAULT 'system',
                is_shared_with_client INTEGER NOT NULL DEFAULT 0,
                description TEXT,
                created_at DATETIME NOT NULL DEFAULT (datetime('now')),
                updated_at DATETIME NOT NULL DEFAULT (datetime('now'))
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Seed a matter so FK passes
        sqlx::query(
            "INSERT INTO matters (id, client_id, title, matter_type, opened_date)
             VALUES ('M-001', 'C-001', 'Test Matter', 'Trademark', '2026-01-01')",
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn create_and_list() {
        let pool = test_pool().await;

        let created = create(
            &pool,
            CreateDocumentInput {
                id: "DOC-001",
                matter_id: "M-001",
                filename: "application.pdf",
                category: "Filing",
                mime_type: "application/pdf",
                file_size_bytes: 102_400,
                vault_path: "M-001/DOC-001.enc",
                uploaded_by: "user-1",
                description: Some("TM application form"),
            },
        )
        .await
        .expect("create should succeed");

        assert_eq!(created.id, "DOC-001");
        assert_eq!(created.filename, "application.pdf");

        let docs = list_for_matter(&pool, "M-001").await.unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].vault_path, "M-001/DOC-001.enc");
    }

    #[tokio::test]
    async fn get_by_id_returns_row() {
        let pool = test_pool().await;

        create(
            &pool,
            CreateDocumentInput {
                id: "DOC-002",
                matter_id: "M-001",
                filename: "power_of_attorney.pdf",
                category: "Correspondence",
                mime_type: "application/pdf",
                file_size_bytes: 50_000,
                vault_path: "M-001/DOC-002.enc",
                uploaded_by: "user-1",
                description: None,
            },
        )
        .await
        .unwrap();

        let row = get_by_id(&pool, "DOC-002").await.unwrap();
        assert!(row.is_some());
        assert_eq!(row.unwrap().category, "Correspondence");
    }

    #[tokio::test]
    async fn delete_returns_vault_path() {
        let pool = test_pool().await;

        create(
            &pool,
            CreateDocumentInput {
                id: "DOC-003",
                matter_id: "M-001",
                filename: "cert.pdf",
                category: "Certificate",
                mime_type: "application/pdf",
                file_size_bytes: 10_000,
                vault_path: "M-001/DOC-003.enc",
                uploaded_by: "user-1",
                description: None,
            },
        )
        .await
        .unwrap();

        let vault_path = delete(&pool, "DOC-003").await.unwrap();
        assert_eq!(vault_path, "M-001/DOC-003.enc");

        let docs = list_for_matter(&pool, "M-001").await.unwrap();
        assert!(docs.is_empty());
    }

    #[tokio::test]
    async fn delete_unknown_id_errors() {
        let pool = test_pool().await;
        let result = delete(&pool, "NONEXISTENT").await;
        assert!(result.is_err());
    }
}
