//! Ekspor Excel. Angka ditulis sebagai sel numerik (rupiah, 2 desimal) supaya bisa dihitung ulang
//! di Excel; konversi sen -> rupiah hanya terjadi di sini, untuk tampilan sel.

use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook, Worksheet, XlsxError};

use super::format as fmt;
use super::{Report, ReportData};
use crate::error::{AppError, AppResult};
use crate::services::statements::{LineKind, StatementLine};

const MONEY: &str = "#,##0.00;[Red]-#,##0.00";

fn err(e: XlsxError) -> AppError {
    AppError::Other(format!("Gagal membuat berkas Excel: {e}"))
}

fn rupiah(cents: i64) -> f64 {
    cents as f64 / 100.0
}

struct Styles {
    title: Format,
    sub: Format,
    head: Format,
    head_right: Format,
    bold: Format,
    money: Format,
    money_bold: Format,
    total: Format,
    total_money: Format,
    section: Format,
}

impl Styles {
    fn new() -> Self {
        let grey = Color::RGB(0xE8E8E8);
        let dark = Color::RGB(0xD8D8D8);
        Self {
            title: Format::new().set_bold().set_font_size(14),
            sub: Format::new().set_font_color(Color::RGB(0x555555)),
            head: Format::new().set_bold().set_border_bottom(FormatBorder::Medium),
            head_right: Format::new().set_bold().set_border_bottom(FormatBorder::Medium).set_align(FormatAlign::Right),
            bold: Format::new().set_bold(),
            money: Format::new().set_num_format(MONEY),
            money_bold: Format::new().set_num_format(MONEY).set_bold(),
            total: Format::new().set_bold().set_background_color(dark).set_border_top(FormatBorder::Thin),
            total_money: Format::new()
                .set_bold()
                .set_num_format(MONEY)
                .set_background_color(dark)
                .set_border_top(FormatBorder::Thin),
            section: Format::new().set_bold().set_background_color(grey),
        }
    }
}

/// Kop: nama perusahaan, alamat, judul, periode. Mengembalikan baris berikutnya yang kosong.
fn kop(ws: &mut Worksheet, s: &Styles, report: &Report, title: &str, period: &str) -> Result<u32, XlsxError> {
    ws.write_string_with_format(0, 0, &report.company.name, &s.title)?;
    let mut row = 1;
    if let Some(a) = &report.company.address {
        ws.write_string_with_format(row, 0, a, &s.sub)?;
        row += 1;
    }
    ws.write_string_with_format(row, 0, title, &s.bold)?;
    ws.write_string_with_format(row + 1, 0, period, &s.sub)?;
    Ok(row + 3)
}

fn statement_rows(
    ws: &mut Worksheet,
    s: &Styles,
    mut row: u32,
    lines: &[StatementLine],
    has_compare: bool,
    value_label: &str,
) -> Result<u32, XlsxError> {
    ws.write_string_with_format(row, 0, "Kode", &s.head)?;
    ws.write_string_with_format(row, 1, "Keterangan", &s.head)?;
    ws.write_string_with_format(row, 2, value_label, &s.head_right)?;
    if has_compare {
        ws.write_string_with_format(row, 3, "Pembanding", &s.head_right)?;
        ws.write_string_with_format(row, 4, "Selisih", &s.head_right)?;
    }
    row += 1;

    let last_col = if has_compare { 4 } else { 2 };
    for l in lines {
        let indent = Format::new().set_indent(l.level.saturating_sub(1));
        match l.kind {
            LineKind::Heading => {
                let f = if l.level == 1 { s.section.clone() } else { s.bold.clone() };
                if l.level == 1 {
                    for c in 0..=last_col {
                        ws.write_blank(row, c, &f)?;
                    }
                }
                ws.write_string_with_format(row, 1, &l.label, &f)?;
            }
            LineKind::Account => {
                if let Some(code) = &l.code {
                    ws.write_string(row, 0, code)?;
                }
                ws.write_string_with_format(row, 1, &l.label, &indent)?;
                write_amounts(ws, row, l, has_compare, &s.money)?;
            }
            LineKind::Subtotal | LineKind::Total | LineKind::Net => {
                let net = l.kind == LineKind::Net;
                let (label_f, money_f) = if net || l.kind == LineKind::Total {
                    (&s.total, &s.total_money)
                } else {
                    (&s.bold, &s.money_bold)
                };
                for c in 0..=last_col {
                    ws.write_blank(row, c, label_f)?;
                }
                ws.write_string_with_format(row, 1, &l.label, label_f)?;
                write_amounts(ws, row, l, has_compare, money_f)?;
            }
        }
        row += 1;
    }
    Ok(row)
}

fn write_amounts(ws: &mut Worksheet, row: u32, l: &StatementLine, has_compare: bool, f: &Format) -> Result<(), XlsxError> {
    if let Some(a) = l.amount {
        ws.write_number_with_format(row, 2, rupiah(a), f)?;
    }
    if has_compare {
        if let Some(c) = l.compare {
            ws.write_number_with_format(row, 3, rupiah(c), f)?;
        }
        if let (Some(a), Some(c)) = (l.amount, l.compare) {
            ws.write_number_with_format(row, 4, rupiah(a - c), f)?;
        }
    }
    Ok(())
}

/// Tulis laporan ke XLSX; mengembalikan isi berkas.
pub fn write(report: &Report) -> AppResult<Vec<u8>> {
    let s = Styles::new();
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();

    let result: Result<(), XlsxError> = (|| {
        match &report.data {
            ReportData::TrialBalance(t) => {
                ws.set_name("Neraca Saldo")?;
                let period = t.as_of.as_deref().map_or("Seluruh periode".into(), |d| format!("Per {}", fmt::date_long(d)));
                let mut row = kop(ws, &s, report, "Neraca Saldo", &period)?;
                for (c, h) in ["Kode", "Nama akun", "Debit", "Kredit"].iter().enumerate() {
                    let f = if c >= 2 { &s.head_right } else { &s.head };
                    ws.write_string_with_format(row, c as u16, *h, f)?;
                }
                row += 1;
                for r in &t.rows {
                    ws.write_string(row, 0, &r.code)?;
                    ws.write_string(row, 1, &r.name)?;
                    if r.balance_debit != 0 {
                        ws.write_number_with_format(row, 2, rupiah(r.balance_debit), &s.money)?;
                    }
                    if r.balance_credit != 0 {
                        ws.write_number_with_format(row, 3, rupiah(r.balance_credit), &s.money)?;
                    }
                    row += 1;
                }
                for c in 0..4 {
                    ws.write_blank(row, c, &s.total)?;
                }
                ws.write_string_with_format(row, 1, "Total", &s.total)?;
                ws.write_number_with_format(row, 2, rupiah(t.total_debit), &s.total_money)?;
                ws.write_number_with_format(row, 3, rupiah(t.total_credit), &s.total_money)?;
                ws.set_column_width(0, 12)?;
                ws.set_column_width(1, 44)?;
                ws.set_column_width(2, 20)?;
                ws.set_column_width(3, 20)?;
            }
            ReportData::IncomeStatement(i) => {
                ws.set_name("Laba Rugi")?;
                let mut period = fmt::range(i.from.as_deref(), i.to.as_deref());
                if i.has_compare {
                    period = format!("{period}  |  Pembanding: {}", fmt::range(i.compare_from.as_deref(), i.compare_to.as_deref()));
                }
                let row = kop(ws, &s, report, "Laporan Laba Rugi", &period)?;
                statement_rows(ws, &s, row, &i.lines, i.has_compare, "Periode ini")?;
                set_statement_widths(ws, i.has_compare)?;
            }
            ReportData::BalanceSheet(b) => {
                ws.set_name("Neraca")?;
                let period = b.as_of.as_deref().map_or("Seluruh periode".into(), |d| format!("Per {}", fmt::date_long(d)));
                let row = kop(ws, &s, report, "Neraca", &period)?;
                statement_rows(ws, &s, row, &b.lines, false, "Jumlah")?;
                set_statement_widths(ws, false)?;
            }
            ReportData::Ledger(l) => {
                ws.set_name("Buku Besar")?;
                let period = fmt::range(l.from.as_deref(), l.to.as_deref());
                let mut row = kop(ws, &s, report, &format!("Buku Besar: {} {}", l.account.code, l.account.name), &period)?;
                for (c, h) in ["Tanggal", "Nomor", "Keterangan", "Debit", "Kredit", "Saldo"].iter().enumerate() {
                    let f = if c >= 3 { &s.head_right } else { &s.head };
                    ws.write_string_with_format(row, c as u16, *h, f)?;
                }
                row += 1;
                ws.write_string(row, 2, "Saldo awal")?;
                ws.write_number_with_format(row, 5, rupiah(l.opening_balance), &s.money)?;
                row += 1;
                for r in &l.rows {
                    ws.write_string(row, 0, fmt::date_short(&r.date))?;
                    ws.write_string(row, 1, &r.number)?;
                    let text = match &r.memo {
                        Some(m) => format!("{} ({m})", r.description),
                        None => r.description.clone(),
                    };
                    let text = if l.account.is_header { format!("[{}] {text}", r.account_code) } else { text };
                    ws.write_string(row, 2, text)?;
                    if r.debit != 0 {
                        ws.write_number_with_format(row, 3, rupiah(r.debit), &s.money)?;
                    }
                    if r.credit != 0 {
                        ws.write_number_with_format(row, 4, rupiah(r.credit), &s.money)?;
                    }
                    ws.write_number_with_format(row, 5, rupiah(r.balance), &s.money)?;
                    row += 1;
                }
                for c in 0..6 {
                    ws.write_blank(row, c, &s.total)?;
                }
                ws.write_string_with_format(row, 2, "Total mutasi / saldo akhir", &s.total)?;
                ws.write_number_with_format(row, 3, rupiah(l.total_debit), &s.total_money)?;
                ws.write_number_with_format(row, 4, rupiah(l.total_credit), &s.total_money)?;
                ws.write_number_with_format(row, 5, rupiah(l.closing_balance), &s.total_money)?;
                ws.set_column_width(0, 13)?;
                ws.set_column_width(1, 18)?;
                ws.set_column_width(2, 50)?;
                for c in 3..6 {
                    ws.set_column_width(c, 18)?;
                }
            }
            ReportData::Voucher(_) => {
                // Bukti jurnal hanya tersedia sebagai PDF; dicegah di lapisan perintah.
            }
        }
        Ok(())
    })();
    result.map_err(err)?;

    if matches!(report.data, ReportData::Voucher(_)) {
        return Err(AppError::Validation("Bukti jurnal hanya tersedia dalam format PDF".into()));
    }
    wb.save_to_buffer().map_err(err)
}

fn set_statement_widths(ws: &mut Worksheet, has_compare: bool) -> Result<(), XlsxError> {
    ws.set_column_width(0, 12)?;
    ws.set_column_width(1, 46)?;
    ws.set_column_width(2, 20)?;
    if has_compare {
        ws.set_column_width(3, 20)?;
        ws.set_column_width(4, 20)?;
    }
    Ok(())
}
