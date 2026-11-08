//! Tutup periode bulanan dan tutup buku tahunan.
//!
//! - Periode yang ditutup memblokir buat/edit/hapus/posting jurnal di bulan itu (lihat `journal::ensure_period_open`).
//! - Tutup buku tahunan membuat satu jurnal penutup (type CLOSING) yang menolkan semua akun pendapatan dan
//!   beban pada tahun fiskal itu dan memindahkan labanya ke Laba Ditahan. Jurnal penutup tidak ikut dihitung
//!   di Laba Rugi, tetapi ikut di Neraca.

use chrono::{Datelike, Days, Months, NaiveDate};
use serde::Serialize;
use serde_json::json;
use sqlx::{SqliteConnection, SqlitePool};

use super::journal::{self, LineInput};
use super::audit;
use crate::error::{AppError, AppResult};

fn invalid<T>(msg: impl Into<String>) -> AppResult<T> {
    Err(AppError::Validation(msg.into()))
}

/// Tahun fiskal (tahun kalender saat dimulai) yang memuat bulan `month` tahun `year`.
pub fn fiscal_year_of(year: i32, month: u32, start_month: u32) -> i32 {
    if month >= start_month { year } else { year - 1 }
}

/// Rentang tanggal tahun fiskal: dari tanggal 1 bulan awal sampai hari terakhir bulan ke-12.
pub fn fiscal_year_range(fiscal_year: i32, start_month: u32) -> AppResult<(NaiveDate, NaiveDate)> {
    let start = NaiveDate::from_ymd_opt(fiscal_year, start_month, 1)
        .ok_or_else(|| AppError::Validation("Tahun fiskal tidak valid".into()))?;
    let end = start + Months::new(12) - Days::new(1);
    Ok((start, end))
}

fn iso(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

async fn start_month(conn: &mut SqliteConnection) -> AppResult<u32> {
    let m: Option<i64> = sqlx::query_scalar("SELECT fiscal_year_start_month FROM company WHERE id = 1")
        .fetch_optional(conn)
        .await?;
    match m {
        Some(m) if (1..=12).contains(&m) => Ok(m as u32),
        Some(_) => invalid("Bulan awal tahun fiskal tidak valid"),
        None => invalid("Perusahaan belum disetup"),
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthStatus {
    pub year: i32,
    pub month: u32,
    pub closed: bool,
    /// Jumlah jurnal POSTED dan DRAFT pada bulan itu.
    pub posted: i64,
    pub drafts: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YearClosing {
    pub closed_at: String,
    pub entry_id: Option<i64>,
    pub number: Option<String>,
    /// Laba (+) atau rugi (-) yang dipindahkan ke Laba Ditahan.
    pub net_income: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeriodOverview {
    pub fiscal_year: i32,
    pub start: String,
    pub end: String,
    pub months: Vec<MonthStatus>,
    /// Terisi bila tahun fiskal ini sudah ditutup.
    pub closing: Option<YearClosing>,
    /// Alasan tutup buku belum bisa dilakukan (kosong bila siap).
    pub blockers: Vec<String>,
    pub can_close_year: bool,
    /// Perkiraan laba/rugi yang akan dipindahkan, dan jumlah akun yang akan ditolkan.
    pub preview_net_income: i64,
    pub preview_accounts: i64,
}

/// Saldo bersih satu akun pendapatan/beban dalam rentang tanggal, tanpa jurnal penutup.
struct PlBalance {
    account_id: i64,
    account_type: String,
    debit: i64,
    credit: i64,
}

async fn pl_balances(conn: &mut SqliteConnection, from: &str, to: &str) -> AppResult<Vec<PlBalance>> {
    let rows: Vec<(i64, String, i64, i64)> = sqlx::query_as(
        "SELECT a.id, a.type, COALESCE(SUM(l.debit), 0), COALESCE(SUM(l.credit), 0)
         FROM journal_lines l
         JOIN journal_entries e ON e.id = l.entry_id
         JOIN accounts a ON a.id = l.account_id
         WHERE e.status = 'POSTED' AND e.type <> 'CLOSING' AND a.type IN ('REVENUE', 'EXPENSE')
           AND e.date >= ? AND e.date <= ?
         GROUP BY a.id ORDER BY a.code",
    )
    .bind(from)
    .bind(to)
    .fetch_all(conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(account_id, account_type, debit, credit)| PlBalance { account_id, account_type, debit, credit })
        .filter(|b| b.debit != b.credit)
        .collect())
}

/// Susun baris jurnal penutup dan laba bersih dari saldo pendapatan/beban.
fn closing_lines(balances: &[PlBalance], retained_earnings: i64) -> (Vec<LineInput>, i64) {
    let mut lines = vec![];
    let mut net = 0i64; // laba (+) / rugi (-)
    for b in balances {
        let nat = if b.account_type == "REVENUE" { b.credit - b.debit } else { b.debit - b.credit };
        net += if b.account_type == "REVENUE" { nat } else { -nat };
        // Saldo normal pendapatan ada di kredit (ditutup dengan debit); beban sebaliknya.
        let close_with_debit = (b.account_type == "REVENUE") == (nat > 0);
        let amount = nat.abs();
        lines.push(LineInput {
            account_id: b.account_id,
            debit: if close_with_debit { amount } else { 0 },
            credit: if close_with_debit { 0 } else { amount },
            memo: None,
        });
    }
    if net != 0 {
        lines.push(LineInput {
            account_id: retained_earnings,
            debit: if net < 0 { -net } else { 0 },
            credit: if net > 0 { net } else { 0 },
            memo: Some("Pemindahan laba/rugi tahun berjalan".into()),
        });
    }
    (lines, net)
}

pub async fn overview(pool: &SqlitePool, fiscal_year: i32) -> AppResult<PeriodOverview> {
    let mut conn = pool.acquire().await?;
    overview_on(&mut conn, fiscal_year).await
}

async fn overview_on(conn: &mut SqliteConnection, fiscal_year: i32) -> AppResult<PeriodOverview> {
    let sm = start_month(conn).await?;
    let (start, end) = fiscal_year_range(fiscal_year, sm)?;

    let counts: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT strftime('%Y-%m', date), status, COUNT(*) FROM journal_entries
         WHERE date >= ? AND date <= ? GROUP BY 1, 2",
    )
    .bind(iso(start))
    .bind(iso(end))
    .fetch_all(&mut *conn)
    .await?;
    let closed: Vec<(i32, u32)> = sqlx::query_as("SELECT year, month FROM fiscal_periods WHERE is_closed = 1")
        .fetch_all(&mut *conn)
        .await?;

    let mut months = Vec::with_capacity(12);
    for i in 0..12 {
        let d = start + Months::new(i);
        let key = format!("{:04}-{:02}", d.year(), d.month());
        let n = |status: &str| counts.iter().find(|(k, s, _)| *k == key && s == status).map_or(0, |(_, _, c)| *c);
        months.push(MonthStatus {
            year: d.year(),
            month: d.month(),
            closed: closed.contains(&(d.year(), d.month())),
            posted: n("POSTED"),
            drafts: n("DRAFT"),
        });
    }

    let closing: Option<YearClosing> = {
        let row: Option<(String, Option<i64>, Option<String>, i64)> = sqlx::query_as(
            "SELECT f.closed_at, f.entry_id, e.number, f.net_income
             FROM fiscal_years f LEFT JOIN journal_entries e ON e.id = f.entry_id WHERE f.year = ?",
        )
        .bind(fiscal_year)
        .fetch_optional(&mut *conn)
        .await?;
        row.map(|(closed_at, entry_id, number, net_income)| YearClosing { closed_at, entry_id, number, net_income })
    };

    // Syarat tutup buku.
    let mut blockers = vec![];
    if closing.is_none() {
        let open = months.iter().filter(|m| !m.closed).count();
        if open > 0 {
            blockers.push(format!("{open} dari 12 periode bulanan belum ditutup"));
        }
        let drafts: i64 = months.iter().map(|m| m.drafts).sum();
        if drafts > 0 {
            blockers.push(format!("Ada {drafts} jurnal draft pada tahun fiskal ini; posting atau hapus dulu"));
        }
        // Tahun fiskal sebelumnya harus sudah ditutup: saldo pendapatan/beban sebelum `start` harus nol.
        let leftover: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM (
                SELECT a.id FROM journal_lines l
                JOIN journal_entries e ON e.id = l.entry_id
                JOIN accounts a ON a.id = l.account_id
                WHERE e.status = 'POSTED' AND a.type IN ('REVENUE', 'EXPENSE') AND e.date < ?
                GROUP BY a.id HAVING SUM(l.debit) <> SUM(l.credit))",
        )
        .bind(iso(start))
        .fetch_one(&mut *conn)
        .await?;
        if leftover > 0 {
            blockers.push("Tahun fiskal sebelumnya belum ditutup".to_string());
        }
    }

    let preview = pl_balances(conn, &iso(start), &iso(end)).await?;
    let (_, preview_net) = closing_lines(&preview, 0);

    Ok(PeriodOverview {
        fiscal_year,
        start: iso(start),
        end: iso(end),
        months,
        can_close_year: closing.is_none() && blockers.is_empty(),
        closing,
        blockers,
        preview_net_income: preview_net,
        preview_accounts: preview.len() as i64,
    })
}

/// Tahun fiskal (tahun mulai) yang memuat tanggal tertentu.
pub async fn fiscal_year_for(pool: &SqlitePool, date: NaiveDate) -> AppResult<i32> {
    let mut conn = pool.acquire().await?;
    let sm = start_month(&mut conn).await?;
    Ok(fiscal_year_of(date.year(), date.month(), sm))
}

async fn year_is_closed(conn: &mut SqliteConnection, fiscal_year: i32) -> AppResult<bool> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM fiscal_years WHERE year = ?")
        .bind(fiscal_year)
        .fetch_one(conn)
        .await?;
    Ok(n > 0)
}

fn check_month(year: i32, month: u32) -> AppResult<NaiveDate> {
    NaiveDate::from_ymd_opt(year, month, 1).ok_or_else(|| AppError::Validation("Periode tidak valid".into()))
}

pub async fn close_period(pool: &SqlitePool, year: i32, month: u32) -> AppResult<()> {
    check_month(year, month)?;
    let mut tx = pool.begin().await?;

    let already: Option<bool> = sqlx::query_scalar("SELECT is_closed FROM fiscal_periods WHERE year = ? AND month = ?")
        .bind(year)
        .bind(month)
        .fetch_optional(&mut *tx)
        .await?;
    if already == Some(true) {
        return invalid("Periode ini sudah ditutup");
    }
    let key = format!("{year:04}-{month:02}");
    let drafts: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM journal_entries WHERE status = 'DRAFT' AND strftime('%Y-%m', date) = ?",
    )
    .bind(&key)
    .fetch_one(&mut *tx)
    .await?;
    if drafts > 0 {
        return invalid(format!("Ada {drafts} jurnal draft pada periode ini; posting atau hapus dulu"));
    }

    sqlx::query(
        "INSERT INTO fiscal_periods (year, month, is_closed) VALUES (?, ?, 1)
         ON CONFLICT(year, month) DO UPDATE SET is_closed = 1",
    )
    .bind(year)
    .bind(month)
    .execute(&mut *tx)
    .await?;
    audit::log(&mut tx, "CLOSE", "period", None, json!({ "period": key })).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn reopen_period(pool: &SqlitePool, year: i32, month: u32) -> AppResult<()> {
    check_month(year, month)?;
    let mut tx = pool.begin().await?;
    let sm = start_month(&mut tx).await?;
    if year_is_closed(&mut tx, fiscal_year_of(year, month, sm)).await? {
        return invalid("Tahun fiskal periode ini sudah ditutup buku. Batalkan tutup buku tahunan dulu");
    }
    let closed: Option<bool> = sqlx::query_scalar("SELECT is_closed FROM fiscal_periods WHERE year = ? AND month = ?")
        .bind(year)
        .bind(month)
        .fetch_optional(&mut *tx)
        .await?;
    if closed != Some(true) {
        return invalid("Periode ini belum ditutup");
    }
    sqlx::query("UPDATE fiscal_periods SET is_closed = 0 WHERE year = ? AND month = ?")
        .bind(year)
        .bind(month)
        .execute(&mut *tx)
        .await?;
    audit::log(&mut tx, "REOPEN", "period", None, json!({ "period": format!("{year:04}-{month:02}") })).await?;
    tx.commit().await?;
    Ok(())
}

/// Tutup buku satu tahun fiskal: buat jurnal penutup dan tandai tahunnya ditutup.
pub async fn close_year(pool: &SqlitePool, fiscal_year: i32) -> AppResult<PeriodOverview> {
    let mut tx = pool.begin().await?;
    let ov = overview_on(&mut tx, fiscal_year).await?;
    if ov.closing.is_some() {
        return invalid("Tahun fiskal ini sudah ditutup buku");
    }
    if let Some(first) = ov.blockers.first() {
        return invalid(format!("Belum bisa tutup buku: {first}"));
    }

    let retained: i64 = sqlx::query_scalar("SELECT id FROM accounts WHERE system_role = 'RETAINED_EARNINGS'")
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::Other("Akun Laba Ditahan tidak ditemukan".into()))?;
    let balances = pl_balances(&mut tx, &ov.start, &ov.end).await?;
    let (lines, net) = closing_lines(&balances, retained);

    let end = NaiveDate::parse_from_str(&ov.end, "%Y-%m-%d").expect("tanggal dari sistem");
    let entry_id = if lines.is_empty() {
        None // tahun tanpa pendapatan/beban: cukup ditandai ditutup
    } else {
        let number = journal::next_number(&mut tx, "TB", end).await?;
        let id = sqlx::query(
            "INSERT INTO journal_entries (number, date, description, status, type, posted_at)
             VALUES (?, ?, ?, 'POSTED', 'CLOSING', datetime('now'))",
        )
        .bind(&number)
        .bind(&ov.end)
        .bind(format!("Jurnal penutup tahun fiskal {fiscal_year}"))
        .execute(&mut *tx)
        .await?
        .last_insert_rowid();
        journal::insert_lines(&mut tx, id, &lines).await?;
        Some(id)
    };

    sqlx::query("INSERT INTO fiscal_years (year, entry_id, net_income) VALUES (?, ?, ?)")
        .bind(fiscal_year)
        .bind(entry_id)
        .bind(net)
        .execute(&mut *tx)
        .await?;
    audit::log(
        &mut tx,
        "CLOSE_YEAR",
        "fiscal_year",
        Some(fiscal_year as i64),
        json!({ "entryId": entry_id, "netIncome": net, "accounts": balances.len() }),
    )
    .await?;

    let result = overview_on(&mut tx, fiscal_year).await?;
    tx.commit().await?;
    Ok(result)
}

/// Batalkan tutup buku: hapus jurnal penutup. Periode bulanan tetap tertutup (buka sendiri bila perlu).
pub async fn reopen_year(pool: &SqlitePool, fiscal_year: i32) -> AppResult<PeriodOverview> {
    let mut tx = pool.begin().await?;
    let row: Option<(Option<i64>, i64)> = sqlx::query_as("SELECT entry_id, net_income FROM fiscal_years WHERE year = ?")
        .bind(fiscal_year)
        .fetch_optional(&mut *tx)
        .await?;
    let Some((entry_id, net)) = row else {
        return invalid("Tahun fiskal ini belum ditutup buku");
    };
    if year_is_closed(&mut tx, fiscal_year + 1).await? {
        return invalid("Tahun fiskal berikutnya sudah ditutup. Batalkan tutup buku tahun itu lebih dulu");
    }

    let before = match entry_id {
        Some(id) => Some(journal::load(&mut tx, id).await?),
        None => None,
    };
    sqlx::query("DELETE FROM fiscal_years WHERE year = ?").bind(fiscal_year).execute(&mut *tx).await?;
    if let Some(id) = entry_id {
        sqlx::query("DELETE FROM journal_entries WHERE id = ?").bind(id).execute(&mut *tx).await?;
    }
    audit::log(
        &mut tx,
        "REOPEN_YEAR",
        "fiscal_year",
        Some(fiscal_year as i64),
        json!({ "netIncome": net, "before": before }),
    )
    .await?;

    let result = overview_on(&mut tx, fiscal_year).await?;
    tx.commit().await?;
    Ok(result)
}
