use sqlx::SqlitePool;
use crate::commands::matters::{Client, CreateClientInput, UpdateClientInput};

// ---------------------------------------------------------------------------
// Row type — maps directly to the `clients` SQLite table
// ---------------------------------------------------------------------------

#[derive(Debug, sqlx::FromRow)]
pub struct ClientRow {
    pub id:         String,
    pub name:       String,
    pub r#type:     String,
    pub email:      Option<String>,
    pub phone:      Option<String>,
    pub address:    Option<String>,
    pub gstin:      Option<String>,
    pub pan:        Option<String>,
    pub notes:      Option<String>,
    pub is_active:  i64,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ClientRow> for Client {
    fn from(r: ClientRow) -> Self {
        Client {
            id:         r.id,
            name:       r.name,
            client_type: r.r#type,
            email:      r.email,
            phone:      r.phone,
            address:    r.address,
            gstin:      r.gstin,
            pan:        r.pan,
            is_active:  r.is_active != 0,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

pub async fn get_by_id(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Client>> {
    let row = sqlx::query_as::<_, ClientRow>(
        "SELECT id, name, type, email, phone, address, gstin, pan, notes,
                is_active, created_at, updated_at
         FROM clients WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(Client::from))
}

pub async fn list_active(pool: &SqlitePool) -> anyhow::Result<Vec<Client>> {
    let rows = sqlx::query_as::<_, ClientRow>(
        "SELECT id, name, type, email, phone, address, gstin, pan, notes,
                is_active, created_at, updated_at
         FROM clients
         WHERE is_active = 1
         ORDER BY name ASC"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Client::from).collect())
}

pub async fn list_all(pool: &SqlitePool) -> anyhow::Result<Vec<Client>> {
    let rows = sqlx::query_as::<_, ClientRow>(
        "SELECT id, name, type, email, phone, address, gstin, pan, notes,
                is_active, created_at, updated_at
         FROM clients
         ORDER BY name ASC"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Client::from).collect())
}

pub async fn create(
    pool: &SqlitePool,
    id: &str,
    input: &CreateClientInput,
) -> anyhow::Result<Client> {
    sqlx::query(
        "INSERT INTO clients (id, name, type, email, phone, address, gstin, pan, is_active)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1)"
    )
    .bind(id)
    .bind(&input.name)
    .bind(&input.client_type)
    .bind(&input.email)
    .bind(&input.phone)
    .bind(&input.address)
    .bind(&input.gstin)
    .bind(&input.pan)
    .execute(pool)
    .await?;

    get_by_id(pool, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Client not found after insert"))
}

pub async fn update(
    pool: &SqlitePool,
    id: &str,
    input: &UpdateClientInput,
) -> anyhow::Result<Client> {
    sqlx::query(
        "UPDATE clients
         SET name       = COALESCE(?, name),
             type       = COALESCE(?, type),
             email      = COALESCE(?, email),
             phone      = COALESCE(?, phone),
             address    = COALESCE(?, address),
             gstin      = COALESCE(?, gstin),
             pan        = COALESCE(?, pan),
             is_active  = COALESCE(?, is_active),
             updated_at = datetime('now')
         WHERE id = ?"
    )
    .bind(&input.name)
    .bind(&input.client_type)
    .bind(&input.email)
    .bind(&input.phone)
    .bind(&input.address)
    .bind(&input.gstin)
    .bind(&input.pan)
    .bind(input.is_active.map(|v| if v { 1i64 } else { 0i64 }))
    .bind(id)
    .execute(pool)
    .await?;

    get_by_id(pool, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Client not found"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::matters::CreateClientInput;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE clients (
                id TEXT PRIMARY KEY, name TEXT NOT NULL,
                type TEXT NOT NULL DEFAULT 'Individual', email TEXT, phone TEXT,
                address TEXT, gstin TEXT, pan TEXT, notes TEXT,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at DATETIME NOT NULL DEFAULT (datetime('now')),
                updated_at DATETIME NOT NULL DEFAULT (datetime('now'))
            )"
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    #[tokio::test]
    async fn test_create_and_get_client() {
        let pool = test_pool().await;
        let input = CreateClientInput {
            name: "Petalveda Scents Pvt Ltd".into(),
            client_type: "Company".into(),
            email: Some("legal@petalveda.com".into()),
            phone: None, address: None, gstin: None, pan: None,
        };
        let client = create(&pool, "test-id-1", &input).await.unwrap();
        assert_eq!(client.name, "Petalveda Scents Pvt Ltd");
        assert_eq!(client.client_type, "Company");
        assert!(client.is_active);

        let fetched = get_by_id(&pool, "test-id-1").await.unwrap().unwrap();
        assert_eq!(fetched.id, "test-id-1");
    }

    #[tokio::test]
    async fn test_list_active_clients() {
        let pool = test_pool().await;
        let input = CreateClientInput {
            name: "Active Co".into(), client_type: "Company".into(),
            email: None, phone: None, address: None, gstin: None, pan: None,
        };
        create(&pool, "c1", &input).await.unwrap();
        // Deactivate
        sqlx::query("UPDATE clients SET is_active = 0 WHERE id = 'c1'")
            .execute(&pool).await.unwrap();

        let input2 = CreateClientInput {
            name: "Active Two".into(), client_type: "Individual".into(),
            email: None, phone: None, address: None, gstin: None, pan: None,
        };
        create(&pool, "c2", &input2).await.unwrap();

        let active = list_active(&pool).await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "c2");
    }
}
