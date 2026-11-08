//! Laporan dasar: Buku Besar (mutasi per akun + saldo berjalan) dan Neraca Saldo.
//! Semua agregasi dilakukan di SQLite; hanya jurnal berstatus POSTED yang dihitung.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::journal::parse_date;
use crate::error::{AppError, AppResult};

const MIN_DATE: &str = "0000-01-01";
const MAX_DATE: &str = "9999-12-31";

fn invalid<T>(msg: impl Into<String>) -> AppResult<T> {
    Err(AppError::Validation(msg.into()))
}

/// Kosong -> None; selain itu harus tanggal YYYY-MM-DD yang valid.
fn opt_date(s: &Option<String>) -> AppResult<Option<String>> {
    match s.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(v) => Ok(Some(parse_date(v)?.format("%Y-%m-%d").to_string())),
    }
}

// ---------------------------------------------------------------------------
// Buku Besar
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LedgerQuery {
    pub account_id: i64,
    pub from: Option<String>,
    pub to: Option<String>,
    /// Mulai dari 1.
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct LedgerAccount {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub is_header: bool,
    pub normal_balance: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerRow {
    pub date: String,
    pub entry_id: i64,
    pub number: String,
    pub description: String,
    pub memo: Option<String>,
    pub account_code: String,
    pub account_name: String,
    pub debit: i64,
    pub credit: i64,
    /// Saldo berjalan dalam arah saldo normal akun (negatif = berlawanan arah).
    pub balance: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerPage {
    pub account: LedgerAccount,
    pub from: Option<String>,
    pub to: Option<String>,
    /// Saldo sebelum tanggal `from` (0 bila `from` kosong).
    pub opening_balance: i64,
    pub total_debit: i64,
    pub total_credit: i64,
    pub closing_balance: i64,
    pub rows: Vec<LedgerRow>,
    /// Jumlah seluruh baris dalam rentang (bukan hanya halaman ini).
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(sqlx::FromRow)]
struct RawLedgerRow {
    date: String,
    entry_id: i64,
    number: String,
    description: String,
    memo: Option<String>,
    account_code: String,
    account_name: String,
    debit: i64,
    credit: i64,
    cum: i64,
}

/// Buku besar satu akun. Untuk akun header, mutasi seluruh akun turunannya digabung.
pub async fn ledger(pool: &SqlitePool, q: LedgerQuery) -> AppResult<LedgerPage> {
    let account: LedgerAccount = sqlx::query_as(
        "SELECT id, code, name, is_header, normal_balance FROM accounts WHERE id = ?",
    )
    .bind(q.account_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Validation("Akun tidak ditemukan".into()))?;

    let from = opt_date(&q.from)?;
    let to = opt_date(&q.to)?;
    if let (Some(f), Some(t)) = (&from, &to)
        && f > t
    {
        return invalid("Tanggal awal tidak boleh setelah tanggal akhir");
    }
    let from_b = from.clone().unwrap_or_else(|| MIN_DATE.into());
    let to_b = to.clone().unwrap_or_else(|| MAX_DATE.into());
    let page = q.page.max(1);
    let page_size = if q.page_size <= 0 { 100 } else { q.page_size.min(500) };
    // +1 bila saldo normal debit, -1 bila kredit: saldo = sign * (debit - kredit).
    let sign: i64 = if account.normal_balance == "DEBIT" { 1 } else { -1 };

    // Saldo awal: seluruh mutasi sebelum `from`.
    let opening_net: i64 = sqlx::query_scalar(
        "WITH RECURSIVE tree(id) AS (
            SELECT id FROM accounts WHERE id = ?
            UNION ALL SELECT a.id FROM accounts a JOIN tree t ON a.parent_id = t.id
         )
         SELECT COALESCE(SUM(l.debit - l.credit), 0)
         FROM journal_lines l JOIN journal_entries e ON e.id = l.entry_id
         WHERE e.status = 'POSTED' AND l.account_id IN (SELECT id FROM tree) AND e.date < ?",
    )
    .bind(q.account_id)
    .bind(&from_b)
    .fetch_one(pool)
    .await?;

    let (total_debit, total_credit, total): (i64, i64, i64) = sqlx::query_as(
        "WITH RECURSIVE tree(id) AS (
            SELECT id FROM accounts WHERE id = ?
            UNION ALL SELECT a.id FROM accounts a JOIN tree t ON a.parent_id = t.id
         )
         SELECT COALESCE(SUM(l.debit), 0), COALESCE(SUM(l.credit), 0), COUNT(*)
         FROM journal_lines l JOIN journal_entries e ON e.id = l.entry_id
         WHERE e.status = 'POSTED' AND l.account_id IN (SELECT id FROM tree)
           AND e.date >= ? AND e.date <= ?",
    )
    .bind(q.account_id)
    .bind(&from_b)
    .bind(&to_b)
    .fetch_one(pool)
    .await?;

    // Jendela dihitung atas seluruh rentang, baru dipotong LIMIT/OFFSET: saldo berjalan tetap
    // benar di halaman mana pun.
    let raw: Vec<RawLedgerRow> = sqlx::query_as(
        "WITH RECURSIVE tree(id) AS (
            SELECT id FROM accounts WHERE id = ?
            UNION ALL SELECT a.id FROM accounts a JOIN tree t ON a.parent_id = t.id
         )
         SELECT e.date AS date, e.id AS entry_id, e.number AS number, e.description AS description,
                l.memo AS memo, a.code AS account_code, a.name AS account_name,
                l.debit AS debit, l.credit AS credit,
                SUM(l.debit - l.credit) OVER (
                    ORDER BY e.date, e.id, l.id ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
                ) AS cum
         FROM journal_lines l
         JOIN journal_entries e ON e.id = l.entry_id
         JOIN accounts a ON a.id = l.account_id
         WHERE e.status = 'POSTED' AND l.account_id IN (SELECT id FROM tree)
           AND e.date >= ? AND e.date <= ?
         ORDER BY e.date, e.id, l.id
         LIMIT ? OFFSET ?",
    )
    .bind(q.account_id)
    .bind(&from_b)
    .bind(&to_b)
    .bind(page_size)
    .bind((page - 1) * page_size)
    .fetch_all(pool)
    .await?;

    let rows = raw
        .into_iter()
        .map(|r| LedgerRow {
            date: r.date,
            entry_id: r.entry_id,
            number: r.number,
            description: r.description,
            memo: r.memo,
            account_code: r.account_code,
            account_name: r.account_name,
            debit: r.debit,
            credit: r.credit,
            balance: sign * (opening_net + r.cum),
        })
        .collect();

    Ok(LedgerPage {
        account,
        from,
        to,
        opening_balance: sign * opening_net,
        total_debit,
        total_credit,
        closing_balance: sign * (opening_net + total_debit - total_credit),
        rows,
        total,
        page,
        page_size,
    })
}

/// Seluruh baris buku besar dalam rentang (tanpa paginasi), untuk ekspor PDF/Excel.
pub async fn ledger_all(pool: &SqlitePool, q: LedgerQuery) -> AppResult<LedgerPage> {
    const CHUNK: i64 = 500;
    let mut first = ledger(pool, LedgerQuery { page: 1, page_size: CHUNK, ..q.clone() }).await?;
    let pages = (first.total + CHUNK - 1) / CHUNK;
    for page in 2..=pages {
        let more = ledger(pool, LedgerQuery { page, page_size: CHUNK, ..q.clone() }).await?;
        first.rows.extend(more.rows);
    }
    first.page = 1;
    first.page_size = first.total.max(1);
    Ok(first)
}

// ---------------------------------------------------------------------------
// Neraca Saldo
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TrialBalanceQuery {
    /// Per tanggal (termasuk). Kosong = semua jurnal.
    pub as_of: Option<String>,
    /// Tampilkan juga akun yang saldonya nol.
    pub include_zero: bool,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct TrialBalanceRow {
    pub account_id: i64,
    pub code: String,
    pub name: String,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub account_type: String,
    pub is_active: bool,
    /// Jumlah seluruh mutasi debit/kredit sampai tanggal itu.
    pub total_debit: i64,
    pub total_credit: i64,
    /// Saldo ditaruh di kolom debit atau kredit sesuai arah saldonya (salah satunya selalu 0).
    #[sqlx(default)]
    pub balance_debit: i64,
    #[sqlx(default)]
    pub balance_credit: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialBalance {
    pub as_of: Option<String>,
    pub rows: Vec<TrialBalanceRow>,
    pub total_debit: i64,
    pub total_credit: i64,
    /// Selisih kolom debit dan kredit; selalu 0 bila pembukuan benar.
    pub difference: i64,
    pub balanced: bool,
}

pub async fn trial_balance(pool: &SqlitePool, q: TrialBalanceQuery) -> AppResult<TrialBalance> {
    let as_of = opt_date(&q.as_of)?;
    let as_of_b = as_of.clone().unwrap_or_else(|| MAX_DATE.into());

    let mut rows: Vec<TrialBalanceRow> = sqlx::query_as(
        "SELECT a.id AS account_id, a.code AS code, a.name AS name, a.type AS type,
                a.is_active AS is_active,
                COALESCE(SUM(l.debit), 0) AS total_debit,
                COALESCE(SUM(l.credit), 0) AS total_credit
         FROM accounts a
         LEFT JOIN journal_lines l ON l.account_id = a.id
              AND l.entry_id IN (SELECT id FROM journal_entries WHERE status = 'POSTED' AND date <= ?)
         WHERE a.is_header = 0
         GROUP BY a.id
         ORDER BY a.code",
    )
    .bind(&as_of_b)
    .fetch_all(pool)
    .await?;

    for r in &mut rows {
        let net = r.total_debit - r.total_credit;
        r.balance_debit = net.max(0);
        r.balance_credit = (-net).max(0);
    }
    if !q.include_zero {
        rows.retain(|r| r.total_debit != r.total_credit);
    } else {
        // Akun nonaktif tanpa mutasi tidak perlu muncul.
        rows.retain(|r| r.is_active || r.total_debit != 0 || r.total_credit != 0);
    }

    let total_debit: i64 = rows.iter().map(|r| r.balance_debit).sum();
    let total_credit: i64 = rows.iter().map(|r| r.balance_credit).sum();
    Ok(TrialBalance {
        as_of,
        rows,
        total_debit,
        total_credit,
        difference: total_debit - total_credit,
        balanced: total_debit == total_credit,
    })
}
