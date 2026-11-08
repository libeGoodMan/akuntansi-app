//! Laporan keuangan: Laba Rugi (rentang tanggal, dengan pembanding) dan Neraca (per tanggal).
//!
//! Keduanya dikembalikan sebagai daftar baris datar (`StatementLine`) sehingga layar, PDF, dan
//! Excel menampilkan susunan yang persis sama.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::journal::parse_date;
use crate::error::{AppError, AppResult};

const MIN_DATE: &str = "0000-01-01";
const MAX_DATE: &str = "9999-12-31";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LineKind {
    /// Judul bagian (level 1) atau kelompok (level 2).
    Heading,
    /// Satu akun.
    Account,
    /// Jumlah satu kelompok (level 2).
    Subtotal,
    /// Jumlah satu bagian (level 1).
    Total,
    /// Hasil akhir (Laba Bersih, Total Kewajiban dan Ekuitas).
    Net,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatementLine {
    pub kind: LineKind,
    pub level: u8,
    pub label: String,
    pub code: Option<String>,
    pub account_id: Option<i64>,
    /// Sen. Kosong pada baris judul.
    pub amount: Option<i64>,
    /// Sen periode pembanding (hanya bila ada pembanding).
    pub compare: Option<i64>,
}

fn line(kind: LineKind, level: u8, label: impl Into<String>) -> StatementLine {
    StatementLine { kind, level, label: label.into(), code: None, account_id: None, amount: None, compare: None }
}

fn sum_line(kind: LineKind, level: u8, label: &str, amount: i64, compare: Option<i64>) -> StatementLine {
    StatementLine { amount: Some(amount), compare, ..line(kind, level, label) }
}

#[derive(sqlx::FromRow)]
struct Acc {
    id: i64,
    code: String,
    name: String,
    #[sqlx(rename = "type")]
    account_type: String,
    parent_id: Option<i64>,
}

type Balances = HashMap<i64, (i64, i64)>;

async fn load_accounts(pool: &SqlitePool) -> AppResult<Vec<Acc>> {
    Ok(sqlx::query_as(
        "SELECT id, code, name, type, parent_id FROM accounts WHERE is_header = 0 ORDER BY code",
    )
    .fetch_all(pool)
    .await?)
}

async fn load_headers(pool: &SqlitePool) -> AppResult<HashMap<i64, (String, String)>> {
    let rows: Vec<(i64, String, String)> = sqlx::query_as("SELECT id, code, name FROM accounts WHERE is_header = 1")
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|(id, code, name)| (id, (code, name))).collect())
}

/// Total debit/kredit per akun untuk jurnal POSTED dalam rentang. `exclude_closing` membuang jurnal
/// penutup (dipakai Laba Rugi supaya laba tahun yang sudah ditutup tetap terlihat).
async fn balances(
    pool: &SqlitePool,
    from: Option<&str>,
    to: Option<&str>,
    exclude_closing: bool,
) -> AppResult<Balances> {
    let rows: Vec<(i64, i64, i64)> = sqlx::query_as(
        "SELECT l.account_id, COALESCE(SUM(l.debit), 0), COALESCE(SUM(l.credit), 0)
         FROM journal_lines l JOIN journal_entries e ON e.id = l.entry_id
         WHERE e.status = 'POSTED' AND e.date >= ? AND e.date <= ?
           AND (? = 0 OR e.type <> 'CLOSING')
         GROUP BY l.account_id",
    )
    .bind(from.unwrap_or(MIN_DATE))
    .bind(to.unwrap_or(MAX_DATE))
    .bind(exclude_closing)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|(id, d, c)| (id, (d, c))).collect())
}

/// Saldo satu akun dalam arah normal jenisnya: Aset/Beban = debit - kredit; lainnya = kredit - debit.
fn natural(account_type: &str, b: Option<&(i64, i64)>) -> i64 {
    let (d, c) = b.copied().unwrap_or((0, 0));
    match account_type {
        "ASSET" | "EXPENSE" => d - c,
        _ => c - d,
    }
}

struct Group {
    name: String,
    rows: Vec<(i64, String, String, i64, Option<i64>)>, // id, code, name, amount, compare
}

/// Kelompokkan akun jenis tertentu berdasarkan akun header induknya; buang baris bernilai nol.
fn build_groups(
    types: &[&str],
    accounts: &[Acc],
    headers: &HashMap<i64, (String, String)>,
    cur: &Balances,
    cmp: Option<&Balances>,
    fallback_name: &str,
) -> Vec<Group> {
    // (kode header untuk urutan, nama) -> baris
    let mut groups: Vec<(String, Group)> = vec![];
    for a in accounts.iter().filter(|a| types.contains(&a.account_type.as_str())) {
        let amount = natural(&a.account_type, cur.get(&a.id));
        let compare = cmp.map(|c| natural(&a.account_type, c.get(&a.id)));
        if amount == 0 && compare.unwrap_or(0) == 0 {
            continue;
        }
        let (order, name) = match a.parent_id.and_then(|p| headers.get(&p)) {
            Some((code, name)) => (code.clone(), name.clone()),
            None => ("~".to_string(), fallback_name.to_string()),
        };
        let row = (a.id, a.code.clone(), a.name.clone(), amount, compare);
        match groups.iter_mut().find(|(o, g)| *o == order && g.name == name) {
            Some((_, g)) => g.rows.push(row),
            None => groups.push((order, Group { name, rows: vec![row] })),
        }
    }
    groups.sort_by(|a, b| a.0.cmp(&b.0));
    groups.into_iter().map(|(_, g)| g).collect()
}

/// Tambahkan satu bagian (judul, kelompok, akun, total) ke daftar baris. Mengembalikan (total, total pembanding).
fn push_section(out: &mut Vec<StatementLine>, title: &str, total_label: &str, groups: &[Group], compare: bool) -> (i64, i64) {
    out.push(line(LineKind::Heading, 1, title));
    let show_groups = groups.len() > 1;
    let (mut total, mut total_cmp) = (0i64, 0i64);
    for g in groups {
        if show_groups {
            out.push(line(LineKind::Heading, 2, &g.name));
        }
        let (mut gt, mut gc) = (0i64, 0i64);
        for (id, code, name, amount, cmp) in &g.rows {
            out.push(StatementLine {
                code: Some(code.clone()),
                account_id: Some(*id),
                amount: Some(*amount),
                compare: *cmp,
                ..line(LineKind::Account, 3, name)
            });
            gt += amount;
            gc += cmp.unwrap_or(0);
        }
        if show_groups {
            out.push(sum_line(LineKind::Subtotal, 2, &format!("Jumlah {}", g.name), gt, compare.then_some(gc)));
        }
        total += gt;
        total_cmp += gc;
    }
    out.push(sum_line(LineKind::Total, 1, total_label, total, compare.then_some(total_cmp)));
    (total, total_cmp)
}

fn opt_date(s: &Option<String>) -> AppResult<Option<String>> {
    match s.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(v) => Ok(Some(parse_date(v)?.format("%Y-%m-%d").to_string())),
    }
}

fn check_range(from: &Option<String>, to: &Option<String>) -> AppResult<()> {
    if let (Some(f), Some(t)) = (from, to)
        && f > t
    {
        return Err(AppError::Validation("Tanggal awal tidak boleh setelah tanggal akhir".into()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Laba Rugi
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct IncomeStatementQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    /// Isi keduanya untuk menampilkan kolom pembanding.
    pub compare_from: Option<String>,
    pub compare_to: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomeStatement {
    pub from: Option<String>,
    pub to: Option<String>,
    pub compare_from: Option<String>,
    pub compare_to: Option<String>,
    pub has_compare: bool,
    pub lines: Vec<StatementLine>,
    pub total_revenue: i64,
    pub total_expense: i64,
    /// Laba (positif) atau rugi (negatif).
    pub net_income: i64,
    pub compare_net_income: Option<i64>,
}

pub async fn income_statement(pool: &SqlitePool, q: IncomeStatementQuery) -> AppResult<IncomeStatement> {
    let (from, to) = (opt_date(&q.from)?, opt_date(&q.to)?);
    let (cfrom, cto) = (opt_date(&q.compare_from)?, opt_date(&q.compare_to)?);
    check_range(&from, &to)?;
    check_range(&cfrom, &cto)?;
    let has_compare = cfrom.is_some() || cto.is_some();

    let accounts = load_accounts(pool).await?;
    let headers = load_headers(pool).await?;
    let cur = balances(pool, from.as_deref(), to.as_deref(), true).await?;
    let cmp = if has_compare { Some(balances(pool, cfrom.as_deref(), cto.as_deref(), true).await?) } else { None };

    let revenue = build_groups(&["REVENUE"], &accounts, &headers, &cur, cmp.as_ref(), "Pendapatan");
    let expense = build_groups(&["EXPENSE"], &accounts, &headers, &cur, cmp.as_ref(), "Beban");

    let mut lines = vec![];
    let (rev, rev_c) = push_section(&mut lines, "Pendapatan", "Total Pendapatan", &revenue, has_compare);
    let (exp, exp_c) = push_section(&mut lines, "Beban", "Total Beban", &expense, has_compare);
    let net = rev - exp;
    let net_c = rev_c - exp_c;
    lines.push(sum_line(LineKind::Net, 1, "Laba (Rugi) Bersih", net, has_compare.then_some(net_c)));

    Ok(IncomeStatement {
        from,
        to,
        compare_from: cfrom,
        compare_to: cto,
        has_compare,
        lines,
        total_revenue: rev,
        total_expense: exp,
        net_income: net,
        compare_net_income: has_compare.then_some(net_c),
    })
}

// ---------------------------------------------------------------------------
// Neraca
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BalanceSheetQuery {
    /// Per tanggal (termasuk). Kosong = semua jurnal.
    pub as_of: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceSheet {
    pub as_of: Option<String>,
    pub lines: Vec<StatementLine>,
    pub total_assets: i64,
    pub total_liabilities: i64,
    /// Ekuitas termasuk laba berjalan.
    pub total_equity: i64,
    pub current_earnings: i64,
    pub total_liabilities_equity: i64,
    pub difference: i64,
    pub balanced: bool,
}

pub async fn balance_sheet(pool: &SqlitePool, q: BalanceSheetQuery) -> AppResult<BalanceSheet> {
    let as_of = opt_date(&q.as_of)?;
    let accounts = load_accounts(pool).await?;
    let headers = load_headers(pool).await?;
    // Jurnal penutup ikut dihitung: laba tahun yang sudah ditutup ada di Laba Ditahan, sedangkan
    // akun pendapatan/beban yang tersisa hanyalah laba berjalan.
    let bal = balances(pool, None, as_of.as_deref(), false).await?;

    let assets = build_groups(&["ASSET"], &accounts, &headers, &bal, None, "Aset");
    let liabilities = build_groups(&["LIABILITY"], &accounts, &headers, &bal, None, "Kewajiban");
    let mut equity = build_groups(&["EQUITY"], &accounts, &headers, &bal, None, "Ekuitas");

    let current_earnings: i64 = accounts
        .iter()
        .map(|a| match a.account_type.as_str() {
            "REVENUE" => natural("REVENUE", bal.get(&a.id)),
            "EXPENSE" => -natural("EXPENSE", bal.get(&a.id)),
            _ => 0,
        })
        .sum();

    let mut lines = vec![];
    let (total_assets, _) = push_section(&mut lines, "Aset", "Total Aset", &assets, false);
    let (total_liabilities, _) = push_section(&mut lines, "Kewajiban", "Total Kewajiban", &liabilities, false);

    // Laba berjalan menjadi baris tambahan di bagian Ekuitas.
    if current_earnings != 0 {
        let row = (0, String::new(), "Laba (Rugi) Berjalan".to_string(), current_earnings, None);
        match equity.first_mut() {
            Some(g) => g.rows.push(row),
            None => equity.push(Group { name: "Ekuitas".into(), rows: vec![row] }),
        }
    }
    let start = lines.len();
    let (total_equity, _) = push_section(&mut lines, "Ekuitas", "Total Ekuitas", &equity, false);
    // Baris laba berjalan bukan akun sungguhan: lepaskan id/kode-nya.
    for l in &mut lines[start..] {
        if l.account_id == Some(0) {
            l.account_id = None;
            l.code = None;
        }
    }

    let total_le = total_liabilities + total_equity;
    lines.push(sum_line(LineKind::Net, 1, "Total Kewajiban dan Ekuitas", total_le, None));

    Ok(BalanceSheet {
        as_of,
        lines,
        total_assets,
        total_liabilities,
        total_equity,
        current_earnings,
        total_liabilities_equity: total_le,
        difference: total_assets - total_le,
        balanced: total_assets == total_le,
    })
}
