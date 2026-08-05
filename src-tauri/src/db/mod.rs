pub mod queries;

// migrations/ directory is consumed by sqlx at runtime — not a Rust module.
// Run migrations with: cargo sqlx migrate run   (from src-tauri/)

// ---------------------------------------------------------------------------
// Migration tests
//
// The query-layer tests build their tables by hand, so they cannot catch a
// migration that is malformed or ordered wrongly. These apply the real
// migration set to an empty database — the same thing that happens on an
// attorney's machine at launch.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    async fn migrated_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("src/db/migrations")
            .run(&pool)
            .await
            .expect("migrations must apply cleanly to an empty database");
        pool
    }

    #[tokio::test]
    async fn all_migrations_apply_in_order() {
        let pool = migrated_pool().await;

        // Every table the app expects should exist afterwards.
        for table in [
            "sequences", "clients", "matters", "matter_parties", "deadlines",
            "documents", "users", "firm_settings", "time_entries", "invoices",
            "invoice_line_items", "payments", "sessions", "ip_assets",
        ] {
            let found: Option<String> = sqlx::query_scalar(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?",
            )
            .bind(table)
            .fetch_optional(&pool)
            .await
            .unwrap();

            assert_eq!(found.as_deref(), Some(table), "missing table: {table}");
        }
    }

    /// 0008 adds ip_asset_id to a table created back in 0003 — verify the
    /// ALTER actually landed, since a silent failure there would only surface
    /// as a runtime query error much later.
    #[tokio::test]
    async fn deadlines_gains_ip_asset_id_column() {
        let pool = migrated_pool().await;

        let columns: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info('deadlines')")
            .fetch_all(&pool)
            .await
            .unwrap();

        assert!(
            columns.iter().any(|c| c == "ip_asset_id"),
            "deadlines.ip_asset_id missing; columns were: {columns:?}"
        );
    }

    /// A deadline must be insertable both with and without an asset link.
    #[tokio::test]
    async fn migrated_schema_accepts_linked_and_unlinked_deadlines() {
        let pool = migrated_pool().await;

        sqlx::query("INSERT INTO clients (id, name) VALUES ('c1', 'Acme Corp')")
            .execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO matters (id, client_id, title, matter_type, opened_date)
             VALUES ('M-1', 'c1', 'Test', 'Trademark', date('now'))",
        )
        .execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO ip_assets (id, matter_id, asset_type, title)
             VALUES ('ip-1', 'M-1', 'Trademark', 'PETALVEDA')",
        )
        .execute(&pool).await.unwrap();

        sqlx::query(
            "INSERT INTO deadlines (id, matter_id, ip_asset_id, docketing_event, due_date)
             VALUES ('d-1', 'M-1', 'ip-1', 'Renewal', '2036-01-01')",
        )
        .execute(&pool).await.expect("asset-linked deadline should insert");

        sqlx::query(
            "INSERT INTO deadlines (id, matter_id, docketing_event, due_date)
             VALUES ('d-2', 'M-1', 'Client call', '2026-09-01')",
        )
        .execute(&pool).await.expect("matter-level deadline should insert");

        let linked: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM deadlines WHERE ip_asset_id = 'ip-1'",
        )
        .fetch_one(&pool).await.unwrap();
        assert_eq!(linked, 1);
    }

    /// The ip_assets CHECK constraints must actually reject bad enum values.
    #[tokio::test]
    async fn ip_asset_check_constraints_hold() {
        let pool = migrated_pool().await;

        sqlx::query("INSERT INTO clients (id, name) VALUES ('c1', 'Acme Corp')")
            .execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO matters (id, client_id, title, matter_type, opened_date)
             VALUES ('M-1', 'c1', 'Test', 'Trademark', date('now'))",
        )
        .execute(&pool).await.unwrap();

        let bad_type = sqlx::query(
            "INSERT INTO ip_assets (id, matter_id, asset_type, title)
             VALUES ('bad-1', 'M-1', 'Hologram', 'X')",
        )
        .execute(&pool).await;
        assert!(bad_type.is_err(), "invalid asset_type must be rejected");

        let bad_status = sqlx::query(
            "INSERT INTO ip_assets (id, matter_id, asset_type, title, status)
             VALUES ('bad-2', 'M-1', 'Trademark', 'X', 'Vibing')",
        )
        .execute(&pool).await;
        assert!(bad_status.is_err(), "invalid status must be rejected");
    }
}
