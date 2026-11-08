use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{QueryBuilder, Sqlite, SqliteConnection, SqlitePool};

use super::audit;
use crate::error::{AppError, AppResult};

/// Batas wajar per baris (dalam sen) dan jumlah baris, agar penjumlahan tidak pernah overflow.
const MAX_AMOUNT: i64 = 1_000_000_000_000_000;
const MAX_LINES: usize = 500;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineInput {
    pub account_id: i64,
    /// Sen. Tepat satu dari debit/kredit yang boleh > 0.
    pub debit: i64,
    pub credit: i64,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalInput {
    /// YYYY-MM-DD
    pub date: String,
    pub description: String,
    pub reference: Option<String>,
    pub lines: Vec<LineInput>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct JournalEntry {
    pub id: i64,
    pub number: String,
    pub date: String,
    pub description: String,
    pub reference: Option<String>,
    pub status: String,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub entry_type: String,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub posted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct JournalLine {
    pub id: i64,
    pub account_id: i64,
    pub account_code: String,
    pub account_name: String,
    pub debit: i64,
    pub credit: i64,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalDetail {
    #[serde(flatten)]
    pub entry: JournalEntry,
    pub lines: Vec<JournalLine>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct JournalSummary {
    pub id: i64,
    pub number: String,
    pub date: String,
    pub description: String,
    pub reference: Option<String>,
    pub status: String,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub entry_type: String,
    pub total: i64,
    pub line_count: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct JournalFilter {
    pub from: Option<String>,
    pub to: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "type")]
    pub entry_type: Option<String>,
    pub search: Option<String>,
    /// Mulai dari 1.
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalPage {
    pub items: Vec<JournalSummary>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: i64,
    pub at: String,
    pub action: String,
    pub detail: Value,
}

fn invalid<T>(msg: impl Into<String>) -> AppResult<T> {
    Err(AppError::Validation(msg.into()))
}

/// 125000050 -> "1.250.000,50"
pub fn format_rupiah(cents: i64) -> String {
    let whole = (cents / 100).to_string();
    let frac = cents % 100;
    let mut grouped = String::new();
    for (i, ch) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) {
            grouped.push('.');
        }
        grouped.push(ch);
    }
    format!("{grouped},{frac:02}")
}

pub(super) fn parse_date(s: &str) -> AppResult<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d")
        .map_err(|_| AppError::Validation("Tanggal tidak valid (format YYYY-MM-DD)".into()))
}

/// Tolak bila periode (bulan) dari tanggal itu sudah ditutup. Periode tanpa catatan dianggap terbuka.
pub(super) async fn ensure_period_open(conn: &mut SqliteConnection, date: NaiveDate) -> AppResult<()> {
    let closed: Option<bool> =
        sqlx::query_scalar("SELECT is_closed FROM fiscal_periods WHERE year = ? AND month = ?")
            .bind(date.year())
            .bind(date.month() as i64)
            .fetch_optional(conn)
            .await?;
    if closed == Some(true) {
        return invalid(format!(
            "Periode {:02}-{} sudah ditutup. Buka kembali periodenya bila perlu mengubah jurnal di dalamnya",
            date.month(),
            date.year()
        ));
    }
    Ok(())
}

/// Validasi baris jurnal. `balanced` = wajib minimal 2 baris dan debit = kredit (posting / edit jurnal POSTED).
pub(super) async fn validate_lines(
    conn: &mut SqliteConnection,
    lines: &[LineInput],
    balanced: bool,
) -> AppResult<()> {
    if lines.len() > MAX_LINES {
        return invalid(format!("Maksimal {MAX_LINES} baris per jurnal"));
    }
    let (mut debit, mut credit) = (0i64, 0i64);
    for (i, l) in lines.iter().enumerate() {
        let n = i + 1;
        if l.debit < 0 || l.credit < 0 || l.debit > MAX_AMOUNT || l.credit > MAX_AMOUNT {
            return invalid(format!("Baris {n}: nilai tidak valid"));
        }
        if (l.debit == 0) == (l.credit == 0) {
            return invalid(format!("Baris {n}: isi tepat salah satu, debit atau kredit"));
        }
        let acc: Option<(String, bool, bool)> =
            sqlx::query_as("SELECT code, is_header, is_active FROM accounts WHERE id = ?")
                .bind(l.account_id)
                .fetch_optional(&mut *conn)
                .await?;
        match acc {
            None => return invalid(format!("Baris {n}: akun tidak ditemukan")),
            Some((code, true, _)) => {
                return invalid(format!("Baris {n}: akun {code} adalah akun header dan tidak bisa dijurnal"));
            }
            Some((code, _, false)) => {
                return invalid(format!("Baris {n}: akun {code} nonaktif"));
            }
            Some(_) => {}
        }
        debit += l.debit;
        credit += l.credit;
    }
    if balanced {
        if lines.len() < 2 {
            return invalid("Jurnal minimal terdiri dari 2 baris");
        }
        if debit != credit {
            return invalid(format!(
                "Jurnal tidak seimbang: debit {}, kredit {} (selisih {})",
                format_rupiah(debit),
                format_rupiah(credit),
                format_rupiah((debit - credit).abs())
            ));
        }
    }
    Ok(())
}

/// Ambil nomor berikutnya dari penghitung bulanan (tidak pernah dipakai ulang).
pub(super) async fn next_number(
    conn: &mut SqliteConnection,
    prefix: &str,
    date: NaiveDate,
) -> AppResult<String> {
    let (year, month) = (date.year(), date.month() as i64);
    sqlx::query(
        "INSERT INTO journal_counters (year, month, last_seq) VALUES (?, ?, 1)
         ON CONFLICT(year, month) DO UPDATE SET last_seq = last_seq + 1",
    )
    .bind(year)
    .bind(month)
    .execute(&mut *conn)
    .await?;
    let seq: i64 = sqlx::query_scalar("SELECT last_seq FROM journal_counters WHERE year = ? AND month = ?")
        .bind(year)
        .bind(month)
        .fetch_one(&mut *conn)
        .await?;
    Ok(format!("{prefix}-{year:04}-{month:02}-{seq:04}"))
}

pub(super) async fn insert_lines(
    conn: &mut SqliteConnection,
    entry_id: i64,
    lines: &[LineInput],
) -> AppResult<()> {
    for l in lines {
        let memo = l.memo.as_deref().map(str::trim).filter(|m| !m.is_empty());
        sqlx::query("INSERT INTO journal_lines (entry_id, account_id, debit, credit, memo) VALUES (?, ?, ?, ?, ?)")
            .bind(entry_id)
            .bind(l.account_id)
            .bind(l.debit)
            .bind(l.credit)
            .bind(memo)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

pub(super) async fn replace_lines(
    conn: &mut SqliteConnection,
    entry_id: i64,
    lines: &[LineInput],
) -> AppResult<()> {
    sqlx::query("DELETE FROM journal_lines WHERE entry_id = ?")
        .bind(entry_id)
        .execute(&mut *conn)
        .await?;
    insert_lines(conn, entry_id, lines).await
}

pub(super) async fn load(conn: &mut SqliteConnection, id: i64) -> AppResult<JournalDetail> {
    let entry: JournalEntry = sqlx::query_as(
        "SELECT id, number, date, description, reference, status, type, created_at, updated_at, posted_at
         FROM journal_entries WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| AppError::Validation("Jurnal tidak ditemukan".into()))?;
    let lines: Vec<JournalLine> = sqlx::query_as(
        "SELECT l.id, l.account_id, a.code AS account_code, a.name AS account_name, l.debit, l.credit, l.memo
         FROM journal_lines l JOIN accounts a ON a.id = l.account_id
         WHERE l.entry_id = ? ORDER BY l.id",
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;
    Ok(JournalDetail { entry, lines })
}

struct Header {
    date: NaiveDate,
    description: String,
    reference: Option<String>,
}

fn validate_header(input: &JournalInput) -> AppResult<Header> {
    let date = parse_date(&input.date)?;
    let description = input.description.trim().to_string();
    if description.is_empty() || description.chars().count() > 300 {
        return invalid("Keterangan wajib diisi (maksimal 300 karakter)");
    }
    let reference = input.reference.as_deref().map(str::trim).filter(|r| !r.is_empty()).map(String::from);
    if reference.as_ref().is_some_and(|r| r.chars().count() > 60) {
        return invalid("Referensi maksimal 60 karakter");
    }
    Ok(Header { date, description, reference })
}

fn require_general(d: &JournalDetail) -> AppResult<()> {
    match d.entry.entry_type.as_str() {
        "GENERAL" => Ok(()),
        "OPENING" => invalid("Jurnal saldo awal diubah lewat layar Saldo Awal"),
        _ => invalid("Jurnal penutup diubah lewat menu Tutup Buku"),
    }
}

/// Sama isinya (tanggal, keterangan, referensi, dan semua baris), mengabaikan id baris dan status.
fn same_content(a: &JournalDetail, b: &JournalDetail) -> bool {
    let key = |d: &JournalDetail| {
        d.lines
            .iter()
            .map(|l| (l.account_id, l.debit, l.credit, l.memo.clone()))
            .collect::<Vec<_>>()
    };
    a.entry.date == b.entry.date
        && a.entry.description == b.entry.description
        && a.entry.reference == b.entry.reference
        && key(a) == key(b)
}

fn to_inputs(d: &JournalDetail) -> Vec<LineInput> {
    d.lines
        .iter()
        .map(|l| LineInput { account_id: l.account_id, debit: l.debit, credit: l.credit, memo: l.memo.clone() })
        .collect()
}

/// Buat jurnal baru. `post = false` menyimpan sebagai DRAFT (boleh belum seimbang).
pub async fn create(pool: &SqlitePool, input: JournalInput, post: bool) -> AppResult<JournalDetail> {
    let mut tx = pool.begin().await?;
    let h = validate_header(&input)?;
    ensure_period_open(&mut tx, h.date).await?;
    validate_lines(&mut tx, &input.lines, post).await?;

    let number = next_number(&mut tx, "JU", h.date).await?;
    let id = sqlx::query(
        "INSERT INTO journal_entries (number, date, description, reference, status, type, posted_at)
         VALUES (?, ?, ?, ?, ?, 'GENERAL', CASE WHEN ? THEN datetime('now') END)",
    )
    .bind(&number)
    .bind(h.date.format("%Y-%m-%d").to_string())
    .bind(&h.description)
    .bind(&h.reference)
    .bind(if post { "POSTED" } else { "DRAFT" })
    .bind(post)
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();
    insert_lines(&mut tx, id, &input.lines).await?;

    let after = load(&mut tx, id).await?;
    audit::log(&mut tx, "CREATE", "journal", Some(id), json!({ "after": after })).await?;
    if post {
        audit::log(&mut tx, "POST", "journal", Some(id), json!({ "number": number })).await?;
    }
    tx.commit().await?;
    Ok(after)
}

/// Ubah jurnal. Jurnal POSTED tetap boleh diubah, tetapi divalidasi ulang seperti posting.
/// Untuk DRAFT, `post = true` sekaligus memposting.
pub async fn update(pool: &SqlitePool, id: i64, input: JournalInput, post: bool) -> AppResult<JournalDetail> {
    let mut tx = pool.begin().await?;
    let before = load(&mut tx, id).await?;
    require_general(&before)?;
    let h = validate_header(&input)?;

    // Periode lama dan baru harus terbuka (tidak boleh memindahkan jurnal dari/ke periode tertutup).
    ensure_period_open(&mut tx, parse_date(&before.entry.date)?).await?;
    ensure_period_open(&mut tx, h.date).await?;

    let was_posted = before.entry.status == "POSTED";
    validate_lines(&mut tx, &input.lines, was_posted || post).await?;

    let posting_now = !was_posted && post;
    sqlx::query(
        "UPDATE journal_entries
         SET date = ?, description = ?, reference = ?, updated_at = datetime('now'),
             status = CASE WHEN ? THEN 'POSTED' ELSE status END,
             posted_at = CASE WHEN ? THEN datetime('now') ELSE posted_at END
         WHERE id = ?",
    )
    .bind(h.date.format("%Y-%m-%d").to_string())
    .bind(&h.description)
    .bind(&h.reference)
    .bind(posting_now)
    .bind(posting_now)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    replace_lines(&mut tx, id, &input.lines).await?;

    let after = load(&mut tx, id).await?;
    // Catat UPDATE hanya bila isinya benar-benar berubah (posting draft tanpa perubahan = hanya POST).
    if !same_content(&before, &after) {
        audit::log(&mut tx, "UPDATE", "journal", Some(id), json!({ "before": before, "after": after })).await?;
    }
    if posting_now {
        audit::log(&mut tx, "POST", "journal", Some(id), json!({ "number": after.entry.number })).await?;
    }
    tx.commit().await?;
    Ok(after)
}

/// Posting DRAFT yang sudah tersimpan.
pub async fn post(pool: &SqlitePool, id: i64) -> AppResult<JournalDetail> {
    let mut tx = pool.begin().await?;
    let before = load(&mut tx, id).await?;
    require_general(&before)?;
    if before.entry.status == "POSTED" {
        return invalid("Jurnal sudah diposting");
    }
    ensure_period_open(&mut tx, parse_date(&before.entry.date)?).await?;
    validate_lines(&mut tx, &to_inputs(&before), true).await?;

    sqlx::query("UPDATE journal_entries SET status = 'POSTED', posted_at = datetime('now') WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let after = load(&mut tx, id).await?;
    audit::log(&mut tx, "POST", "journal", Some(id), json!({ "number": after.entry.number })).await?;
    tx.commit().await?;
    Ok(after)
}

/// Hapus jurnal (DRAFT maupun POSTED). Nomornya tidak dipakai ulang; isinya tersimpan di audit log.
pub async fn delete(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    let before = load(&mut tx, id).await?;
    require_general(&before)?;
    ensure_period_open(&mut tx, parse_date(&before.entry.date)?).await?;

    sqlx::query("DELETE FROM journal_entries WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    audit::log(&mut tx, "DELETE", "journal", Some(id), json!({ "before": before })).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn get(pool: &SqlitePool, id: i64) -> AppResult<JournalDetail> {
    let mut conn = pool.acquire().await?;
    load(&mut conn, id).await
}

/// Riwayat perubahan satu jurnal (terlama dulu).
pub async fn history(pool: &SqlitePool, id: i64) -> AppResult<Vec<HistoryItem>> {
    let rows: Vec<(i64, String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, at, action, detail FROM audit_log WHERE entity = 'journal' AND entity_id = ? ORDER BY id",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, at, action, detail)| HistoryItem {
            id,
            at,
            action,
            detail: detail.and_then(|d| serde_json::from_str(&d).ok()).unwrap_or(Value::Null),
        })
        .collect())
}

fn apply_filter(qb: &mut QueryBuilder<Sqlite>, f: &JournalFilter) {
    qb.push(" WHERE 1 = 1");
    if let Some(from) = f.from.as_deref().filter(|s| !s.is_empty()) {
        qb.push(" AND e.date >= ").push_bind(from.to_string());
    }
    if let Some(to) = f.to.as_deref().filter(|s| !s.is_empty()) {
        qb.push(" AND e.date <= ").push_bind(to.to_string());
    }
    if let Some(status) = f.status.as_deref().filter(|s| !s.is_empty()) {
        qb.push(" AND e.status = ").push_bind(status.to_string());
    }
    if let Some(t) = f.entry_type.as_deref().filter(|s| !s.is_empty()) {
        qb.push(" AND e.type = ").push_bind(t.to_string());
    }
    if let Some(q) = f.search.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let like = format!("%{q}%");
        qb.push(" AND (e.number LIKE ").push_bind(like.clone());
        qb.push(" OR e.description LIKE ").push_bind(like.clone());
        qb.push(" OR e.reference LIKE ").push_bind(like);
        qb.push(")");
    }
}

pub async fn list(pool: &SqlitePool, filter: JournalFilter) -> AppResult<JournalPage> {
    let page = filter.page.max(1);
    let page_size = if filter.page_size <= 0 { 25 } else { filter.page_size.min(200) };

    let mut count = QueryBuilder::new("SELECT COUNT(*) FROM journal_entries e");
    apply_filter(&mut count, &filter);
    let total: i64 = count.build_query_scalar().fetch_one(pool).await?;

    let mut qb = QueryBuilder::new(
        "SELECT e.id, e.number, e.date, e.description, e.reference, e.status, e.type,
                COALESCE(SUM(l.debit), 0) AS total, COUNT(l.id) AS line_count
         FROM journal_entries e LEFT JOIN journal_lines l ON l.entry_id = e.id",
    );
    apply_filter(&mut qb, &filter);
    qb.push(" GROUP BY e.id ORDER BY e.date DESC, e.id DESC LIMIT ")
        .push_bind(page_size)
        .push(" OFFSET ")
        .push_bind((page - 1) * page_size);
    let items: Vec<JournalSummary> = qb.build_query_as().fetch_all(pool).await?;

    Ok(JournalPage { items, total, page, page_size })
}
