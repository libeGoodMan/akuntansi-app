//! Jurnal saldo awal (type = OPENING): satu per perusahaan, dibuat lewat layar Saldo Awal.
//! Selisih debit/kredit otomatis diarahkan ke akun Modal / Saldo Awal Ekuitas.

use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::json;
use sqlx::{SqliteConnection, SqlitePool};

use super::audit;
use super::journal::{
    ensure_period_open, insert_lines, load, next_number, parse_date, replace_lines, validate_lines,
    JournalDetail, LineInput,
};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpeningInput {
    /// Tanggal mulai pembukuan, YYYY-MM-DD.
    pub date: String,
    /// Saldo per akun neraca. Baris Modal/Saldo Awal Ekuitas diisi otomatis, jangan dikirim.
    pub lines: Vec<LineInput>,
}

fn invalid<T>(msg: impl Into<String>) -> AppResult<T> {
    Err(AppError::Validation(msg.into()))
}

async fn existing_id(conn: &mut SqliteConnection) -> AppResult<Option<i64>> {
    Ok(sqlx::query_scalar("SELECT id FROM journal_entries WHERE type = 'OPENING'")
        .fetch_optional(conn)
        .await?)
}

pub async fn get(pool: &SqlitePool) -> AppResult<Option<JournalDetail>> {
    let mut conn = pool.acquire().await?;
    match existing_id(&mut conn).await? {
        Some(id) => Ok(Some(load(&mut conn, id).await?)),
        None => Ok(None),
    }
}

pub async fn save(pool: &SqlitePool, input: OpeningInput) -> AppResult<JournalDetail> {
    let mut tx = pool.begin().await?;
    let date: NaiveDate = parse_date(&input.date)?;
    ensure_period_open(&mut tx, date).await?;
    if input.lines.is_empty() {
        return invalid("Isi saldo awal minimal satu akun");
    }

    let equity_id: i64 = sqlx::query_scalar("SELECT id FROM accounts WHERE system_role = 'OPENING_EQUITY'")
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::Other("Akun Modal / Saldo Awal Ekuitas tidak ditemukan".into()))?;

    // Hanya akun neraca; akun penyeimbang diisi otomatis.
    for l in &input.lines {
        if l.account_id == equity_id {
            return invalid("Akun Modal / Saldo Awal Ekuitas diisi otomatis sebagai penyeimbang");
        }
        let t: Option<String> = sqlx::query_scalar("SELECT type FROM accounts WHERE id = ?")
            .bind(l.account_id)
            .fetch_optional(&mut *tx)
            .await?;
        match t.as_deref() {
            Some("ASSET" | "LIABILITY" | "EQUITY") => {}
            Some(_) => return invalid("Saldo awal hanya untuk akun neraca (aset, kewajiban, ekuitas)"),
            None => return invalid("Akun tidak ditemukan"),
        }
    }
    validate_lines(&mut tx, &input.lines, false).await?;

    let diff: i64 = input.lines.iter().map(|l| l.debit - l.credit).sum();
    let mut lines = input.lines.clone();
    if diff != 0 {
        lines.push(LineInput {
            account_id: equity_id,
            debit: if diff < 0 { -diff } else { 0 },
            credit: if diff > 0 { diff } else { 0 },
            memo: Some("Penyeimbang saldo awal".into()),
        });
    }
    validate_lines(&mut tx, &lines, true).await?;

    let date_s = date.format("%Y-%m-%d").to_string();
    let id = match existing_id(&mut tx).await? {
        Some(id) => {
            let before = load(&mut tx, id).await?;
            ensure_period_open(&mut tx, parse_date(&before.entry.date)?).await?;
            sqlx::query("UPDATE journal_entries SET date = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(&date_s)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            replace_lines(&mut tx, id, &lines).await?;
            let after = load(&mut tx, id).await?;
            audit::log(&mut tx, "OPENING_SAVE", "journal", Some(id), json!({ "before": before, "after": after }))
                .await?;
            id
        }
        None => {
            let number = next_number(&mut tx, "SA", date).await?;
            let id = sqlx::query(
                "INSERT INTO journal_entries (number, date, description, status, type, posted_at)
                 VALUES (?, ?, 'Saldo awal', 'POSTED', 'OPENING', datetime('now'))",
            )
            .bind(&number)
            .bind(&date_s)
            .execute(&mut *tx)
            .await?
            .last_insert_rowid();
            insert_lines(&mut tx, id, &lines).await?;
            let after = load(&mut tx, id).await?;
            audit::log(&mut tx, "OPENING_SAVE", "journal", Some(id), json!({ "after": after })).await?;
            id
        }
    };

    let detail = load(&mut tx, id).await?;
    tx.commit().await?;
    Ok(detail)
}

/// Hapus jurnal saldo awal (mulai dari nol lagi).
pub async fn delete(pool: &SqlitePool) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    let Some(id) = existing_id(&mut tx).await? else {
        return invalid("Saldo awal belum diisi");
    };
    let before = load(&mut tx, id).await?;
    ensure_period_open(&mut tx, parse_date(&before.entry.date)?).await?;
    sqlx::query("DELETE FROM journal_entries WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    audit::log(&mut tx, "DELETE", "journal", Some(id), json!({ "before": before })).await?;
    tx.commit().await?;
    Ok(())
}
