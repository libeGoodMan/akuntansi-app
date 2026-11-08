//! Data contoh November 2026 (lihat tests_statements): pendapatan 98jt, beban 57,5jt, laba 40,5jt.

use chrono::NaiveDate;
use sqlx::SqlitePool;

use super::accounts;
use super::journal::{self, JournalInput, LineInput};
use super::periods;
use super::reports::{self, TrialBalanceQuery};
use super::statements::{self, BalanceSheetQuery, IncomeStatementQuery};
use super::tests_statements::sample;
use crate::db;
use crate::error::AppError;

const JT: i64 = 100_000_000;

fn msg(e: AppError) -> String {
    match e {
        AppError::Validation(m) => m,
        other => panic!("bukan error validasi: {other:?}"),
    }
}

async fn acc(pool: &SqlitePool, code: &str) -> i64 {
    accounts::list(pool).await.unwrap().into_iter().find(|a| a.code == code).unwrap().id
}

fn input(date: &str, lines: Vec<LineInput>) -> JournalInput {
    JournalInput { date: date.into(), description: "uji".into(), reference: None, lines }
}

async fn post(pool: &SqlitePool, date: &str, dr: &str, cr: &str, amount: i64) -> i64 {
    let lines = vec![
        LineInput { account_id: acc(pool, dr).await, debit: amount, credit: 0, memo: None },
        LineInput { account_id: acc(pool, cr).await, debit: 0, credit: amount, memo: None },
    ];
    journal::create(pool, input(date, lines), true).await.unwrap().entry.id
}

async fn close_months(pool: &SqlitePool, year: i32) {
    for m in 1..=12 {
        periods::close_period(pool, year, m).await.unwrap();
    }
}

async fn audit_actions(pool: &SqlitePool, entity: &str) -> Vec<String> {
    sqlx::query_scalar("SELECT action FROM audit_log WHERE entity = ? ORDER BY id")
        .bind(entity)
        .fetch_all(pool)
        .await
        .unwrap()
}

async fn closing_entry(pool: &SqlitePool) -> Option<journal::JournalDetail> {
    let id: Option<i64> = sqlx::query_scalar("SELECT id FROM journal_entries WHERE type = 'CLOSING'").fetch_optional(pool).await.unwrap();
    match id {
        Some(id) => Some(journal::get(pool, id).await.unwrap()),
        None => None,
    }
}

// ---------- Periode bulanan ----------

#[tokio::test]
async fn tutup_periode_memblokir_jurnal_dan_bisa_dibuka_kembali() {
    let pool = sample().await;
    periods::close_period(&pool, 2026, 11).await.unwrap();

    let lines = vec![
        LineInput { account_id: acc(&pool, "1-1100").await, debit: 100, credit: 0, memo: None },
        LineInput { account_id: acc(&pool, "4-1000").await, debit: 0, credit: 100, memo: None },
    ];
    let e = journal::create(&pool, input("2026-11-20", lines.clone()), true).await.unwrap_err();
    assert!(msg(e).contains("sudah ditutup"));
    // Bulan lain tetap terbuka.
    journal::create(&pool, input("2026-12-05", lines.clone()), true).await.unwrap();

    assert!(msg(periods::close_period(&pool, 2026, 11).await.unwrap_err()).contains("sudah ditutup"));
    periods::reopen_period(&pool, 2026, 11).await.unwrap();
    journal::create(&pool, input("2026-11-20", lines), true).await.unwrap();
    assert!(msg(periods::reopen_period(&pool, 2026, 11).await.unwrap_err()).contains("belum ditutup"));
    assert_eq!(audit_actions(&pool, "period").await, ["CLOSE", "REOPEN"]);
}

#[tokio::test]
async fn tutup_periode_ditolak_bila_ada_draft() {
    let pool = sample().await;
    let lines = vec![
        LineInput { account_id: acc(&pool, "1-1100").await, debit: 1, credit: 0, memo: None },
        LineInput { account_id: acc(&pool, "4-1000").await, debit: 0, credit: 1, memo: None },
    ];
    let draft = journal::create(&pool, input("2026-10-05", lines), false).await.unwrap();
    let e = periods::close_period(&pool, 2026, 10).await.unwrap_err();
    assert!(msg(e).contains("1 jurnal draft"));
    journal::delete(&pool, draft.entry.id).await.unwrap();
    periods::close_period(&pool, 2026, 10).await.unwrap();
    assert!(periods::close_period(&pool, 2026, 13).await.is_err());
}

#[tokio::test]
async fn ringkasan_periode_menghitung_jurnal_per_bulan() {
    let pool = sample().await;
    let lines = vec![
        LineInput { account_id: acc(&pool, "1-1100").await, debit: 5, credit: 0, memo: None },
        LineInput { account_id: acc(&pool, "4-1000").await, debit: 0, credit: 5, memo: None },
    ];
    journal::create(&pool, input("2026-11-07", lines), false).await.unwrap();
    periods::close_period(&pool, 2026, 3).await.unwrap();

    let ov = periods::overview(&pool, 2026).await.unwrap();
    assert_eq!((ov.start.as_str(), ov.end.as_str()), ("2026-01-01", "2026-12-31"));
    assert_eq!(ov.months.len(), 12);
    let nov = &ov.months[10];
    assert_eq!((nov.month, nov.posted, nov.drafts, nov.closed), (11, 11, 1, false)); // 1 saldo awal + 10 jurnal
    assert!(ov.months[2].closed);
    assert_eq!(ov.preview_net_income, 40 * JT + JT / 2);
    assert!(!ov.can_close_year);
    assert!(ov.blockers.iter().any(|b| b.contains("11 dari 12 periode")));
    assert!(ov.blockers.iter().any(|b| b.contains("1 jurnal draft")));
}

// ---------- Tutup buku tahunan ----------

#[tokio::test]
async fn tutup_buku_butuh_semua_periode_ditutup() {
    let pool = sample().await;
    let e = periods::close_year(&pool, 2026).await.unwrap_err();
    assert!(msg(e).contains("12 periode bulanan belum ditutup"));
    for m in 1..=11 {
        periods::close_period(&pool, 2026, m).await.unwrap();
    }
    assert!(periods::close_year(&pool, 2026).await.is_err(), "masih ada satu periode terbuka");
    periods::close_period(&pool, 2026, 12).await.unwrap();
    assert!(periods::overview(&pool, 2026).await.unwrap().can_close_year);
}

#[tokio::test]
async fn tutup_buku_membuat_jurnal_penutup_yang_benar() {
    let pool = sample().await;
    close_months(&pool, 2026).await;
    let ov = periods::close_year(&pool, 2026).await.unwrap();

    let closing = ov.closing.expect("tahun ditutup");
    assert_eq!(closing.net_income, 40 * JT + JT / 2);
    assert_eq!(closing.number.as_deref(), Some("TB-2026-12-0001"));

    let j = closing_entry(&pool).await.unwrap();
    assert_eq!((j.entry.date.as_str(), j.entry.status.as_str(), j.entry.entry_type.as_str()), ("2026-12-31", "POSTED", "CLOSING"));
    let line = |code: &str| j.lines.iter().find(|l| l.account_code == code).map(|l| (l.debit, l.credit)).unwrap();
    assert_eq!(line("4-1000"), (100 * JT, 0), "pendapatan ditutup di debit");
    assert_eq!(line("4-1100"), (0, 2 * JT), "akun kontra pendapatan ditutup di kredit");
    assert_eq!(line("5-1000"), (0, 20 * JT));
    assert_eq!(line("6-1400"), (0, JT / 2));
    assert_eq!(line("3-3000"), (0, 40 * JT + JT / 2), "laba ke Laba Ditahan");
    assert_eq!(j.lines.iter().map(|l| l.debit).sum::<i64>(), j.lines.iter().map(|l| l.credit).sum::<i64>());
    assert_eq!(j.lines.len(), 9);

    assert!(msg(periods::close_year(&pool, 2026).await.unwrap_err()).contains("sudah ditutup"));
    assert_eq!(audit_actions(&pool, "fiscal_year").await, ["CLOSE_YEAR"]);
}

#[tokio::test]
async fn setelah_tutup_buku_laporan_tetap_benar() {
    let pool = sample().await;
    let before = statements::balance_sheet(&pool, BalanceSheetQuery { as_of: Some("2026-12-31".into()) }).await.unwrap();
    close_months(&pool, 2026).await;
    periods::close_year(&pool, 2026).await.unwrap();

    // Neraca: total sama, laba pindah ke Laba Ditahan.
    let after = statements::balance_sheet(&pool, BalanceSheetQuery { as_of: Some("2026-12-31".into()) }).await.unwrap();
    assert!(after.balanced);
    assert_eq!((after.total_assets, after.total_equity), (before.total_assets, before.total_equity));
    assert_eq!(after.current_earnings, 0);

    // Laba Rugi tahun itu tidak berubah.
    let is = statements::income_statement(
        &pool,
        IncomeStatementQuery { from: Some("2026-01-01".into()), to: Some("2026-12-31".into()), ..Default::default() },
    )
    .await
    .unwrap();
    assert_eq!(is.net_income, 40 * JT + JT / 2);

    // Neraca saldo: akun pendapatan/beban nol; tetap seimbang.
    let tb = reports::trial_balance(&pool, TrialBalanceQuery { as_of: Some("2026-12-31".into()), include_zero: false }).await.unwrap();
    assert!(tb.balanced);
    assert!(tb.rows.iter().all(|r| !r.code.starts_with(['4', '5', '6'])));

    // Tahun berikutnya mulai dari nol, laba lama tetap di Laba Ditahan.
    post(&pool, "2027-01-15", "1-1100", "4-1000", 10 * JT).await;
    let bs = statements::balance_sheet(&pool, BalanceSheetQuery { as_of: Some("2027-01-31".into()) }).await.unwrap();
    assert!(bs.balanced);
    assert_eq!(bs.current_earnings, 10 * JT);
    let is27 = statements::income_statement(
        &pool,
        IncomeStatementQuery { from: Some("2027-01-01".into()), to: Some("2027-12-31".into()), ..Default::default() },
    )
    .await
    .unwrap();
    assert_eq!(is27.net_income, 10 * JT);
}

#[tokio::test]
async fn jurnal_penutup_dilindungi_dari_edit_dan_hapus_biasa() {
    let pool = sample().await;
    close_months(&pool, 2026).await;
    periods::close_year(&pool, 2026).await.unwrap();
    let j = closing_entry(&pool).await.unwrap();

    assert!(msg(journal::delete(&pool, j.entry.id).await.unwrap_err()).contains("Tutup Buku"));
    let attempt = input("2026-12-31", vec![]);
    assert!(msg(journal::update(&pool, j.entry.id, attempt, false).await.unwrap_err()).contains("Tutup Buku"));
}

#[tokio::test]
async fn batalkan_tutup_buku_menghapus_jurnal_penutup() {
    let pool = sample().await;
    close_months(&pool, 2026).await;
    periods::close_year(&pool, 2026).await.unwrap();

    // Periode di tahun yang sudah ditutup tidak bisa dibuka sebelum tutup buku dibatalkan.
    assert!(msg(periods::reopen_period(&pool, 2026, 11).await.unwrap_err()).contains("Batalkan tutup buku"));

    let ov = periods::reopen_year(&pool, 2026).await.unwrap();
    assert!(ov.closing.is_none());
    assert!(closing_entry(&pool).await.is_none());
    assert!(ov.months.iter().all(|m| m.closed), "periode bulanan tetap tertutup");
    assert_eq!(ov.preview_net_income, 40 * JT + JT / 2);

    // Akun pendapatan/beban kembali berisi saldo.
    let tb = reports::trial_balance(&pool, TrialBalanceQuery::default()).await.unwrap();
    assert!(tb.rows.iter().any(|r| r.code == "4-1000"));
    assert!(tb.balanced);

    periods::reopen_period(&pool, 2026, 11).await.unwrap();
    assert!(msg(periods::reopen_year(&pool, 2026).await.unwrap_err()).contains("belum ditutup"));

    // Riwayat menyimpan isi jurnal penutup yang dihapus.
    let detail: String = sqlx::query_scalar("SELECT detail FROM audit_log WHERE action = 'REOPEN_YEAR'").fetch_one(&pool).await.unwrap();
    assert!(detail.contains("TB-2026-12-0001") && detail.contains("Laba Ditahan"));
    assert_eq!(audit_actions(&pool, "fiscal_year").await, ["CLOSE_YEAR", "REOPEN_YEAR"]);

    // Bisa ditutup lagi setelah periode ditutup kembali.
    periods::close_period(&pool, 2026, 11).await.unwrap();
    periods::close_year(&pool, 2026).await.unwrap();
    assert!(closing_entry(&pool).await.is_some());
}

#[tokio::test]
async fn tahun_rugi_mendebit_laba_ditahan() {
    let pool = db::test_pool().await;
    super::company::setup(
        &pool,
        super::company::SetupInput { name: "PT Rugi".into(), fiscal_year_start_month: 1, template: "standard".into() },
    )
    .await
    .unwrap();
    post(&pool, "2026-05-10", "1-1100", "4-1000", 10 * JT).await;
    post(&pool, "2026-06-10", "6-1000", "1-1100", 30 * JT).await;
    close_months(&pool, 2026).await;
    let ov = periods::close_year(&pool, 2026).await.unwrap();
    assert_eq!(ov.closing.unwrap().net_income, -20 * JT);

    let j = closing_entry(&pool).await.unwrap();
    let re = j.lines.iter().find(|l| l.account_code == "3-3000").unwrap();
    assert_eq!((re.debit, re.credit), (20 * JT, 0), "rugi mengurangi Laba Ditahan (debit)");
    assert_eq!(j.lines.iter().map(|l| l.debit).sum::<i64>(), j.lines.iter().map(|l| l.credit).sum::<i64>());
}

#[tokio::test]
async fn tahun_tanpa_aktivitas_bisa_ditutup_tanpa_jurnal() {
    let pool = sample().await;
    close_months(&pool, 2025).await;
    let ov = periods::close_year(&pool, 2025).await.unwrap();
    let closing = ov.closing.unwrap();
    assert_eq!((closing.entry_id, closing.net_income), (None, 0));
    assert!(closing_entry(&pool).await.is_none());
    periods::reopen_year(&pool, 2025).await.unwrap();
}

#[tokio::test]
async fn tahun_fiskal_harus_ditutup_berurutan() {
    let pool = sample().await;
    post(&pool, "2027-03-10", "1-1100", "4-1000", 5 * JT).await;
    close_months(&pool, 2027).await;

    let e = periods::close_year(&pool, 2027).await.unwrap_err();
    assert!(msg(e).contains("Tahun fiskal sebelumnya belum ditutup"));

    close_months(&pool, 2026).await;
    periods::close_year(&pool, 2026).await.unwrap();
    periods::close_year(&pool, 2027).await.unwrap();

    // Tidak bisa membatalkan tahun yang lebih awal selagi tahun berikutnya masih tertutup.
    assert!(msg(periods::reopen_year(&pool, 2026).await.unwrap_err()).contains("berikutnya sudah ditutup"));
    periods::reopen_year(&pool, 2027).await.unwrap();
    periods::reopen_year(&pool, 2026).await.unwrap();
}

#[tokio::test]
async fn tahun_fiskal_bukan_januari() {
    let pool = sample().await;
    sqlx::query("UPDATE company SET fiscal_year_start_month = 7").execute(&pool).await.unwrap();

    assert_eq!(periods::fiscal_year_of(2026, 11, 7), 2026);
    assert_eq!(periods::fiscal_year_of(2027, 3, 7), 2026);
    assert_eq!(periods::fiscal_year_of(2026, 6, 7), 2025);
    let (s, e) = periods::fiscal_year_range(2026, 7).unwrap();
    assert_eq!((s, e), (NaiveDate::from_ymd_opt(2026, 7, 1).unwrap(), NaiveDate::from_ymd_opt(2027, 6, 30).unwrap()));

    post(&pool, "2027-03-10", "1-1100", "4-1000", 5 * JT).await; // masih tahun fiskal 2026 (Jul 2026 - Jun 2027)
    let ov = periods::overview(&pool, 2026).await.unwrap();
    assert_eq!((ov.start.as_str(), ov.end.as_str()), ("2026-07-01", "2027-06-30"));
    assert_eq!(ov.months[0].month, 7);
    assert_eq!((ov.months[11].year, ov.months[11].month), (2027, 6));
    assert_eq!(ov.preview_net_income, 40 * JT + JT / 2 + 5 * JT);

    for (y, m) in (7..=12).map(|m| (2026, m)).chain((1..=6).map(|m| (2027, m))) {
        periods::close_period(&pool, y, m).await.unwrap();
    }
    // Reopen satu bulan di tahun fiskal 2026 lalu tutup lagi tidak masalah sebelum tutup buku.
    periods::reopen_period(&pool, 2027, 3).await.unwrap();
    periods::close_period(&pool, 2027, 3).await.unwrap();

    let ov = periods::close_year(&pool, 2026).await.unwrap();
    assert_eq!(ov.closing.unwrap().number.as_deref(), Some("TB-2027-06-0001"));
    let j = closing_entry(&pool).await.unwrap();
    assert_eq!(j.entry.date, "2027-06-30");
    assert_eq!(j.lines.iter().find(|l| l.account_code == "3-3000").unwrap().credit, 45 * JT + JT / 2);
}
