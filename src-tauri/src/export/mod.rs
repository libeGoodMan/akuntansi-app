//! Ekspor laporan ke PDF (template Typst) dan Excel (rust_xlsxwriter).
//!
//! Alur: `ReportRequest` -> data laporan dari services -> `pdf_data()` (teks sudah diformat) ->
//! template Typst -> bytes PDF; atau `xlsx::write()` -> bytes XLSX.

mod format;
pub mod pdf;
pub mod xlsx;

use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::services::company::{self, Company};
use crate::services::journal::{self, JournalDetail};
use crate::services::reports::{
    self, LedgerPage, LedgerQuery, TrialBalance, TrialBalanceQuery,
};
use crate::services::statements::{
    self, BalanceSheet, BalanceSheetQuery, IncomeStatement, IncomeStatementQuery, LineKind, StatementLine,
};
use pdf::Template;

/// Permintaan ekspor dari frontend: `{ "kind": "trial-balance", ...parameter }`.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ReportRequest {
    TrialBalance(TrialBalanceQuery),
    IncomeStatement(IncomeStatementQuery),
    BalanceSheet(BalanceSheetQuery),
    Ledger(LedgerQuery),
    Voucher { id: i64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Pdf,
    Xlsx,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Format::Pdf => "pdf",
            Format::Xlsx => "xlsx",
        }
    }
}

pub enum ReportData {
    TrialBalance(TrialBalance),
    IncomeStatement(IncomeStatement),
    BalanceSheet(BalanceSheet),
    Ledger(LedgerPage),
    Voucher(JournalDetail),
}

/// Data laporan beserta profil perusahaan untuk kop.
pub struct Report {
    pub company: Company,
    pub data: ReportData,
}

pub async fn load(pool: &SqlitePool, req: ReportRequest) -> AppResult<Report> {
    let company = company::get(pool).await?.ok_or_else(|| AppError::Validation("Perusahaan belum disetup".into()))?;
    let data = match req {
        ReportRequest::TrialBalance(q) => ReportData::TrialBalance(reports::trial_balance(pool, q).await?),
        ReportRequest::IncomeStatement(q) => ReportData::IncomeStatement(statements::income_statement(pool, q).await?),
        ReportRequest::BalanceSheet(q) => ReportData::BalanceSheet(statements::balance_sheet(pool, q).await?),
        ReportRequest::Ledger(q) => ReportData::Ledger(reports::ledger_all(pool, q).await?),
        ReportRequest::Voucher { id } => ReportData::Voucher(journal::get(pool, id).await?),
    };
    Ok(Report { company, data })
}

impl Report {
    /// Nama berkas yang disarankan (tanpa ekstensi).
    pub fn file_stem(&self) -> String {
        let opt = |s: &Option<String>| s.clone().unwrap_or_else(|| "semua".into());
        match &self.data {
            ReportData::TrialBalance(t) => format!("neraca-saldo-{}", opt(&t.as_of)),
            ReportData::IncomeStatement(i) => format!("laba-rugi-{}_{}", opt(&i.from), opt(&i.to)),
            ReportData::BalanceSheet(b) => format!("neraca-{}", opt(&b.as_of)),
            ReportData::Ledger(l) => format!("buku-besar-{}", l.account.code),
            ReportData::Voucher(v) => format!("bukti-{}", v.entry.number),
        }
    }
}

// ---------------------------------------------------------------------------
// Bentuk data untuk template PDF
// ---------------------------------------------------------------------------

fn company_json(c: &Company) -> Value {
    json!({ "name": c.name, "address": c.address })
}

fn type_label(t: &str) -> &'static str {
    match t {
        "ASSET" => "Aset",
        "LIABILITY" => "Kewajiban",
        "EQUITY" => "Ekuitas",
        "REVENUE" => "Pendapatan",
        _ => "Beban",
    }
}

fn kind_str(k: LineKind) -> &'static str {
    match k {
        LineKind::Heading => "heading",
        LineKind::Account => "account",
        LineKind::Subtotal => "subtotal",
        LineKind::Total => "total",
        LineKind::Net => "net",
    }
}

fn lines_json(lines: &[StatementLine], has_compare: bool) -> Value {
    lines
        .iter()
        .map(|l| {
            let amount = l.amount.map(format::money).unwrap_or_default();
            let compare = l.compare.map(format::money).unwrap_or_default();
            let change = match (l.amount, l.compare) {
                (Some(a), Some(c)) if has_compare => format::money(a - c),
                _ => String::new(),
            };
            json!({
                "kind": kind_str(l.kind), "level": l.level, "label": l.label, "code": l.code,
                "amount": amount, "compare": compare, "change": change,
            })
        })
        .collect()
}

fn balance_note(balanced: bool, difference: i64, what: &str) -> String {
    if balanced {
        format!("Seimbang: {what}.")
    } else {
        format!("Tidak seimbang, selisih {}. Periksa jurnal.", format::money(difference.abs()))
    }
}

/// Pilih template dan susun data JSON (teks sudah diformat) untuk satu laporan.
pub fn pdf_data(report: &Report) -> (Template, Value) {
    let company = company_json(&report.company);
    let printed_at = format::now_long();

    match &report.data {
        ReportData::TrialBalance(t) => {
            let order = ["ASSET", "LIABILITY", "EQUITY", "REVENUE", "EXPENSE"];
            let groups: Vec<Value> = order
                .iter()
                .filter_map(|ty| {
                    let rows: Vec<Value> = t
                        .rows
                        .iter()
                        .filter(|r| r.account_type == *ty)
                        .map(|r| json!({
                            "code": r.code, "name": r.name,
                            "debit": format::money_blank(r.balance_debit), "credit": format::money_blank(r.balance_credit),
                        }))
                        .collect();
                    (!rows.is_empty()).then(|| json!({ "name": type_label(ty), "rows": rows }))
                })
                .collect();
            let period = match &t.as_of {
                Some(d) => format!("Per {}", format::date_long(d)),
                None => "Seluruh periode".into(),
            };
            (
                Template::TrialBalance,
                json!({
                    "title": "Neraca Saldo", "period": period, "printed_at": printed_at, "company": company,
                    "groups": groups,
                    "total_debit": format::money(t.total_debit), "total_credit": format::money(t.total_credit),
                    "balanced": t.balanced,
                    "note": balance_note(t.balanced, t.difference, "total debit sama dengan total kredit"),
                }),
            )
        }
        ReportData::IncomeStatement(i) => {
            let period2 = i
                .has_compare
                .then(|| format!("Pembanding: {}", format::range(i.compare_from.as_deref(), i.compare_to.as_deref())));
            (
                Template::IncomeStatement,
                json!({
                    "title": "Laporan Laba Rugi",
                    "period": format::range(i.from.as_deref(), i.to.as_deref()),
                    "period2": period2, "printed_at": printed_at, "company": company,
                    "has_compare": i.has_compare, "current_label": "Periode ini", "compare_label": "Pembanding",
                    "lines": lines_json(&i.lines, i.has_compare),
                }),
            )
        }
        ReportData::BalanceSheet(b) => {
            let period = match &b.as_of {
                Some(d) => format!("Per {}", format::date_long(d)),
                None => "Seluruh periode".into(),
            };
            (
                Template::BalanceSheet,
                json!({
                    "title": "Neraca", "period": period, "printed_at": printed_at, "company": company,
                    "has_compare": false, "current_label": "Jumlah",
                    "lines": lines_json(&b.lines, false),
                    "balanced": b.balanced,
                    "note": balance_note(b.balanced, b.difference, "Aset sama dengan Kewajiban ditambah Ekuitas"),
                }),
            )
        }
        ReportData::Ledger(l) => {
            let rows: Vec<Value> = l
                .rows
                .iter()
                .map(|r| {
                    let description = if l.account.is_header {
                        format!("[{}] {}", r.account_code, r.description)
                    } else {
                        r.description.clone()
                    };
                    json!({
                        "date": format::date_short(&r.date), "number": r.number, "description": description,
                        "memo": r.memo, "debit": format::money_blank(r.debit), "credit": format::money_blank(r.credit),
                        "balance": format::money(r.balance),
                    })
                })
                .collect();
            (
                Template::Ledger,
                json!({
                    "title": "Buku Besar",
                    "period": format::range(l.from.as_deref(), l.to.as_deref()),
                    "printed_at": printed_at, "company": company,
                    "account": format!("{} {}", l.account.code, l.account.name),
                    "normal": if l.account.normal_balance == "DEBIT" { "Debit" } else { "Kredit" },
                    "opening": format::money(l.opening_balance), "closing": format::money(l.closing_balance),
                    "total_debit": format::money(l.total_debit), "total_credit": format::money(l.total_credit),
                    "rows": rows,
                }),
            )
        }
        ReportData::Voucher(v) => {
            let (td, tc) = v.lines.iter().fold((0, 0), |(d, c), l| (d + l.debit, c + l.credit));
            let lines: Vec<Value> = v
                .lines
                .iter()
                .map(|l| json!({
                    "code": l.account_code, "account": l.account_name, "memo": l.memo,
                    "debit": format::money_blank(l.debit), "credit": format::money_blank(l.credit),
                }))
                .collect();
            (
                Template::Voucher,
                json!({
                    "title": "Bukti Jurnal", "period": format!("Tanggal {}", format::date_long(&v.entry.date)),
                    "printed_at": printed_at, "company": company,
                    "number": v.entry.number, "date": format::date_long(&v.entry.date),
                    "description": v.entry.description, "reference": v.entry.reference,
                    "status": if v.entry.status == "POSTED" { "Diposting" } else { "Draft" },
                    "kind": match v.entry.entry_type.as_str() { "OPENING" => "Saldo awal", "CLOSING" => "Penutup", _ => "Umum" },
                    "lines": lines,
                    "total_debit": format::money(td), "total_credit": format::money(tc),
                }),
            )
        }
    }
}

/// Bangun berkas laporan (PDF atau XLSX) sebagai bytes. Pekerjaan berat berjalan di thread terpisah
/// supaya UI tidak macet.
pub async fn build(pool: &SqlitePool, req: ReportRequest, format: Format) -> AppResult<(Vec<u8>, String)> {
    let report = load(pool, req).await?;
    let name = format!("{}.{}", report.file_stem(), format.extension());
    let bytes = tauri::async_runtime::spawn_blocking(move || match format {
        Format::Pdf => {
            let (template, data) = pdf_data(&report);
            pdf::render(template, &data)
        }
        Format::Xlsx => xlsx::write(&report),
    })
    .await
    .map_err(|e| AppError::Other(format!("Proses ekspor terhenti: {e}")))??;
    Ok((bytes, name))
}

#[cfg(test)]
mod tests;
