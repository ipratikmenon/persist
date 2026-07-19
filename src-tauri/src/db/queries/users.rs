// User query layer — Phase 1 Auth.
// Runtime sqlx queries (no DATABASE_URL at compile time).

use sqlx::SqlitePool;

#[derive(Debug, sqlx::FromRow)]
pub struct UserRow {
    pub id:            String,
    pub name:          String,
    pub email:         String,
    pub role:          String,
    pub password_hash: String,
    pub is_active:     i64,
    pub last_login_at: Option<String>,
    pub created_at:    String,
    pub updated_at:    String,
}

pub struct CreateUserInput<'a> {
    pub id:            &'a str,
    pub name:          &'a str,
    pub email:         &'a str,
    pub role:          &'a str,
    pub password_hash: &'a str,
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

pub async fn get_by_email(pool: &SqlitePool, email: &str) -> anyhow::Result<Option<UserRow>> {
    let row = sqlx::query_as::<_, UserRow>(
        "SELECT id, name, email, role, password_hash, is_active,
                last_login_at, created_at, updated_at
         FROM users WHERE email = ? AND is_active = 1",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn get_by_id(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<UserRow>> {
    let row = sqlx::query_as::<_, UserRow>(
        "SELECT id, name, email, role, password_hash, is_active,
                last_login_at, created_at, updated_at
         FROM users WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn count(pool: &SqlitePool) -> anyhow::Result<i64> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    Ok(n)
}

pub async fn create(pool: &SqlitePool, input: CreateUserInput<'_>) -> anyhow::Result<UserRow> {
    sqlx::query(
        "INSERT INTO users (id, name, email, role, password_hash)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(input.id)
    .bind(input.name)
    .bind(input.email)
    .bind(input.role)
    .bind(input.password_hash)
    .execute(pool)
    .await?;

    get_by_id(pool, input.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("user not found after insert"))
}

/// Stamp last_login_at to now.
pub async fn touch_login(pool: &SqlitePool, id: &str) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE users SET last_login_at = datetime('now'), updated_at = datetime('now')
         WHERE id = ?",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_active(pool: &SqlitePool) -> anyhow::Result<Vec<UserRow>> {
    let rows = sqlx::query_as::<_, UserRow>(
        "SELECT id, name, email, role, password_hash, is_active,
                last_login_at, created_at, updated_at
         FROM users WHERE is_active = 1 ORDER BY name",
    )
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

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE users (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT NOT NULL UNIQUE,
                role TEXT NOT NULL DEFAULT 'Associate',
                password_hash TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                last_login_at DATETIME,
                created_at DATETIME NOT NULL DEFAULT (datetime('now')),
                updated_at DATETIME NOT NULL DEFAULT (datetime('now'))
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    #[tokio::test]
    async fn create_and_get_by_email() {
        let pool = test_pool().await;
        create(
            &pool,
            CreateUserInput {
                id: "u-001",
                name: "Sree Lakshmi Menon",
                email: "slm@persist.in",
                role: "Partner",
                password_hash: "$2b$12$placeholder_hash",
            },
        )
        .await
        .unwrap();

        let user = get_by_email(&pool, "slm@persist.in").await.unwrap();
        assert!(user.is_some());
        assert_eq!(user.unwrap().role, "Partner");
    }

    #[tokio::test]
    async fn unknown_email_returns_none() {
        let pool = test_pool().await;
        let result = get_by_email(&pool, "ghost@persist.in").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn count_empty_and_populated() {
        let pool = test_pool().await;
        assert_eq!(count(&pool).await.unwrap(), 0);

        create(
            &pool,
            CreateUserInput {
                id: "u-002",
                name: "Kajal Thakur",
                email: "kt@persist.in",
                role: "Associate",
                password_hash: "$2b$12$placeholder_hash",
            },
        )
        .await
        .unwrap();

        assert_eq!(count(&pool).await.unwrap(), 1);
    }
}
