//! Format teks untuk laporan cetak/ekspor: rupiah, tanggal Indonesia.

use chrono::NaiveDate;

use crate::services::journal::format_rupiah;

/// 125000050 -> "1.250.000,50"; negatif -> "-1.250.000,50".
pub fn money(cents: i64) -> String {
    if cents < 0 {
        format!("-{}", format_rupiah(-cents))
    } else {
        format_rupiah(cents)
    }
}

/// Seperti `money`, tetapi 0 menjadi string kosong (sel debit/kredit).
pub fn money_blank(cents: i64) -> String {
    if cents == 0 { String::new() } else { money(cents) }
}

const MONTHS: [&str; 12] = [
    "Januari", "Februari", "Maret", "April", "Mei", "Juni", "Juli", "Agustus", "September", "Oktober",
    "November", "Desember",
];
const MONTHS_SHORT: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "Mei", "Jun", "Jul", "Agu", "Sep", "Okt", "Nov", "Des"];

fn parse(iso: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(iso, "%Y-%m-%d").ok()
}

/// "2026-11-05" -> "5 November 2026"
pub fn date_long(iso: &str) -> String {
    use chrono::Datelike;
    match parse(iso) {
        Some(d) => format!("{} {} {}", d.day(), MONTHS[d.month0() as usize], d.year()),
        None => iso.to_string(),
    }
}

/// "2026-11-05" -> "05 Nov 2026"
pub fn date_short(iso: &str) -> String {
    use chrono::Datelike;
    match parse(iso) {
        Some(d) => format!("{:02} {} {}", d.day(), MONTHS_SHORT[d.month0() as usize], d.year()),
        None => iso.to_string(),
    }
}

/// Rentang tanggal untuk judul: "1 November 2026 s.d. 30 November 2026" (sisi kosong = tak terbatas).
pub fn range(from: Option<&str>, to: Option<&str>) -> String {
    match (from, to) {
        (Some(f), Some(t)) => format!("{} s.d. {}", date_long(f), date_long(t)),
        (Some(f), None) => format!("Sejak {}", date_long(f)),
        (None, Some(t)) => format!("Sampai {}", date_long(t)),
        (None, None) => "Seluruh periode".into(),
    }
}

/// Waktu cetak lokal: "8 November 2026 00:54".
pub fn now_long() -> String {
    use chrono::{Datelike, Local, Timelike};
    let n = Local::now();
    format!("{} {} {} {:02}:{:02}", n.day(), MONTHS[n.month0() as usize], n.year(), n.hour(), n.minute())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uang_dan_tanggal() {
        assert_eq!(money(125_000_050), "1.250.000,50");
        assert_eq!(money(-125_000_050), "-1.250.000,50");
        assert_eq!(money(-5), "-0,05");
        assert_eq!(money_blank(0), "");
        assert_eq!(date_long("2026-11-05"), "5 November 2026");
        assert_eq!(date_short("2026-01-09"), "09 Jan 2026");
        assert_eq!(range(Some("2026-11-01"), Some("2026-11-30")), "1 November 2026 s.d. 30 November 2026");
        assert_eq!(range(None, None), "Seluruh periode");
        assert_eq!(date_long("bukan-tanggal"), "bukan-tanggal");
    }
}
