use serde_json::Value;
use sqlx::SqliteConnection;

use crate::error::AppResult;

/// Catat satu baris ke audit_log. Panggil di dalam transaksi yang sama dengan perubahannya.
pub async fn log(
    conn: &mut SqliteConnection,
    action: &str,
    entity: &str,
    entity_id: Option<i64>,
    detail: Value,
) -> AppResult<()> {
    sqlx::query("INSERT INTO audit_log (action, entity, entity_id, detail) VALUES (?, ?, ?, ?)")
        .bind(action)
        .bind(entity)
        .bind(entity_id)
        .bind(detail.to_string())
        .execute(conn)
        .await?;
    Ok(())
}
