pub mod queries;

// migrations/ directory is consumed by sqlx at runtime — not a Rust module.
// Run migrations with: cargo sqlx migrate run   (from src-tauri/)
