//! Data ringkasan untuk Dashboard. Semua agregasi dilakukan di SQLite; frontend hanya menggambar.

use std::collections::HashMap;

use chrono::{Datelike, Local, Months, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::journal::{self, JournalFilter, JournalSummary};
use crate::error::{AppError, AppResult};

/// Jumlah bulan pada grafik tren (bulan berjalan + 11 sebelumnya).
const MONTHS: u32 = 12;
/// Jumlah irisan terpisah pada komposisi beban; sisanya digabung menjadi "Lainnya".
const TOP_SLICES: usize = 5;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DashboardQuery {
    /// Tanggal "hari ini" (YYYY-MM-DD). Kosong = tanggal lokal komputer.
    pub today: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashAccount {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub balance: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthPoint {
    /// "2026-11"
    pub month: String,
    pub revenue: i64,
    pub expense: i64,
    /// Laba (positif) atau rugi (negatif) bulan itu.
    pub net: i64,
    /// Saldo seluruh akun kas/bank pada akhir bulan itu.
    pub cash: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseSlice {
    pub label: String,
    pub code: Option<String>,
    pub amount: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    pub today: String,
    /// Bulan berjalan, "2026-11".
    pub month: String,
    /// Ada jurnal yang sudah diposting (selain itu tampilkan keadaan kosong).
    pub has_data: bool,
    pub cash_balance: i64,
    pub cash_accounts: Vec<CashAccount>,
    pub revenue: i64,
    pub expense: i64,
    pub net: i64,
    pub prev_revenue: i64,
    pub prev_expense: i64,
    pub prev_net: i64,
    pub monthly: Vec<MonthPoint>,
    pub expense_breakdown: Vec<ExpenseSlice>,
    pub recent: Vec<JournalSummary>,
    pub draft_count: i64,
}

fn ym(d: NaiveDate) -> String {
    format!("{:04}-{:02}", d.year(), d.month())
}

fn parse_today(q: &DashboardQuery) -> AppResult<NaiveDate> {
    match q.today.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(Local::now().date_naive()),
        Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|_| AppError::Validation("Tanggal tidak valid (format YYYY-MM-DD)".into())),
    }
}

pub async fn dashboard(pool: &SqlitePool, q: DashboardQuery) -> AppResult<Dashboard> {
    let today = parse_today(&q)?;
    let this_month = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).expect("tanggal valid");
    let first_month = this_month - Months::new(MONTHS - 1);
    let window_end = this_month + Months::new(1) - chrono::Days::new(1);
    let (start_s, end_s) = (first_month.format("%Y-%m-%d").to_string(), window_end.format("%Y-%m-%d").to_string());
    let today_s = today.format("%Y-%m-%d").to_string();

    // Pendapatan & beban per bulan (jurnal penutup dikecualikan agar laba tahun yang sudah ditutup terlihat).
    let pl: Vec<(String, String, i64, i64)> = sqlx::query_as(
        "SELECT strftime('%Y-%m', e.date), a.type, COALESCE(SUM(l.debit), 0), COALESCE(SUM(l.credit), 0)
         FROM journal_lines l
         JOIN journal_entries e ON e.id = l.entry_id
         JOIN accounts a ON a.id = l.account_id
         WHERE e.status = 'POSTED' AND e.type <> 'CLOSING' AND a.type IN ('REVENUE', 'EXPENSE')
           AND e.date >= ? AND e.date <= ?
         GROUP BY 1, 2",
    )
    .bind(&start_s)
    .bind(&end_s)
    .fetch_all(pool)
    .await?;
    let mut revenue: HashMap<String, i64> = HashMap::new();
    let mut expense: HashMap<String, i64> = HashMap::new();
    for (month, ty, d, c) in pl {
        if ty == "REVENUE" {
            *revenue.entry(month).or_default() += c - d;
        } else {
            *expense.entry(month).or_default() += d - c;
        }
    }

    // Saldo kas/bank: saldo sebelum jendela + mutasi bulanan secara kumulatif.
    let cash_opening: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(l.debit - l.credit), 0)
         FROM journal_lines l JOIN journal_entries e ON e.id = l.entry_id JOIN accounts a ON a.id = l.account_id
         WHERE e.status = 'POSTED' AND a.is_cash = 1 AND e.date < ?",
    )
    .bind(&start_s)
    .fetch_one(pool)
    .await?;
    let cash_moves: Vec<(String, i64)> = sqlx::query_as(
        "SELECT strftime('%Y-%m', e.date), COALESCE(SUM(l.debit - l.credit), 0)
         FROM journal_lines l JOIN journal_entries e ON e.id = l.entry_id JOIN accounts a ON a.id = l.account_id
         WHERE e.status = 'POSTED' AND a.is_cash = 1 AND e.date >= ? AND e.date <= ?
         GROUP BY 1",
    )
    .bind(&start_s)
    .bind(&end_s)
    .fetch_all(pool)
    .await?;
    let cash_moves: HashMap<String, i64> = cash_moves.into_iter().collect();

    let mut monthly = Vec::with_capacity(MONTHS as usize);
    let mut running = cash_opening;
    for i in 0..MONTHS {
        let key = ym(first_month + Months::new(i));
        running += cash_moves.get(&key).copied().unwrap_or(0);
        let (r, e) = (revenue.get(&key).copied().unwrap_or(0), expense.get(&key).copied().unwrap_or(0));
        monthly.push(MonthPoint { month: key, revenue: r, expense: e, net: r - e, cash: running });
    }

    let cur = monthly.last().expect("12 bulan").clone();
    let prev = monthly[monthly.len() - 2].clone();

    // Kas & bank per akun sampai hari ini.
    let cash_rows: Vec<(i64, String, String, i64)> = sqlx::query_as(
        "SELECT a.id, a.code, a.name,
                COALESCE((SELECT SUM(l.debit - l.credit) FROM journal_lines l JOIN journal_entries e ON e.id = l.entry_id
                          WHERE l.account_id = a.id AND e.status = 'POSTED' AND e.date <= ?), 0)
         FROM accounts a WHERE a.is_cash = 1 AND a.is_header = 0
         ORDER BY a.code",
    )
    .bind(&today_s)
    .fetch_all(pool)
    .await?;
    // Akun kas selalu tampil, juga bila saldonya nol.
    let cash_accounts: Vec<CashAccount> = cash_rows
        .into_iter()
        .map(|(id, code, name, balance)| CashAccount { id, code, name, balance })
        .collect();
    let cash_balance = cash_accounts.iter().map(|a| a.balance).sum();

    // Komposisi beban bulan berjalan.
    let month_start = this_month.format("%Y-%m-%d").to_string();
    let slices: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT a.code, a.name, COALESCE(SUM(l.debit - l.credit), 0) AS amount
         FROM journal_lines l
         JOIN journal_entries e ON e.id = l.entry_id
         JOIN accounts a ON a.id = l.account_id
         WHERE e.status = 'POSTED' AND e.type <> 'CLOSING' AND a.type = 'EXPENSE'
           AND e.date >= ? AND e.date <= ?
         GROUP BY a.id HAVING amount > 0
         ORDER BY amount DESC, a.code",
    )
    .bind(&month_start)
    .bind(&end_s)
    .fetch_all(pool)
    .await?;
    let mut expense_breakdown: Vec<ExpenseSlice> = slices
        .iter()
        .take(TOP_SLICES)
        .map(|(code, name, amount)| ExpenseSlice { label: name.clone(), code: Some(code.clone()), amount: *amount })
        .collect();
    let rest: i64 = slices.iter().skip(TOP_SLICES).map(|(_, _, a)| a).sum();
    if rest > 0 {
        expense_breakdown.push(ExpenseSlice { label: "Lainnya".into(), code: None, amount: rest });
    }

    let recent = journal::list(pool, JournalFilter { page: 1, page_size: 5, ..Default::default() }).await?.items;
    let draft_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM journal_entries WHERE status = 'DRAFT'")
        .fetch_one(pool)
        .await?;
    let posted: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM journal_entries WHERE status = 'POSTED'")
        .fetch_one(pool)
        .await?;

    Ok(Dashboard {
        today: today_s,
        month: cur.month.clone(),
        has_data: posted > 0,
        cash_balance,
        cash_accounts,
        revenue: cur.revenue,
        expense: cur.expense,
        net: cur.net,
        prev_revenue: prev.revenue,
        prev_expense: prev.expense,
        prev_net: prev.net,
        monthly,
        expense_breakdown,
        recent,
        draft_count,
    })
}
