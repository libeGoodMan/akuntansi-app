//! Data contoh: perusahaan dagang, satu bulan (November 2026). Semua angka dalam rupiah (x100 = sen)
//! dan hasil laporannya dihitung manual:
//!
//! Saldo awal 1 Nov: Kas 50jt, Bank 100jt, Persediaan 30jt, Peralatan 20jt, Utang Usaha 40jt -> Modal 160jt.
//! Pendapatan: Penjualan 100jt, Retur -2jt                      = 98jt
//! Beban: HPP 20jt + Pembelian 25jt + Angkut 1jt = 46jt;
//!        Gaji 8jt + Sewa 3jt + Penyusutan 0,5jt = 11,5jt       = 57,5jt
//! Laba bersih = 98 - 57,5 = 40,5jt
//! Neraca 30 Nov: Aset 265,5jt = Kewajiban 65jt + Ekuitas 200,5jt (Modal 160jt + Laba berjalan 40,5jt)

use sqlx::SqlitePool;

use super::accounts;
use super::company::{self, SetupInput};
use super::journal::{self, JournalInput, LineInput};
use super::opening::{self, OpeningInput};
use super::reports::{self, TrialBalanceQuery};
use super::statements::{
    self, BalanceSheetQuery, IncomeStatementQuery, LineKind, StatementLine,
};
use crate::db;

const JT: i64 = 100_000_000; // 1 juta rupiah dalam sen

async fn acc(pool: &SqlitePool, code: &str) -> i64 {
    accounts::list(pool).await.unwrap().into_iter().find(|a| a.code == code).unwrap().id
}

fn line(account_id: i64, debit: i64, credit: i64) -> LineInput {
    LineInput { account_id, debit, credit, memo: None }
}

async fn post(pool: &SqlitePool, date: &str, dr: &str, cr: &str, amount: i64) {
    let input = JournalInput {
        date: date.into(),
        description: format!("{dr} / {cr}"),
        reference: None,
        lines: vec![line(acc(pool, dr).await, amount, 0), line(acc(pool, cr).await, 0, amount)],
    };
    journal::create(pool, input, true).await.unwrap();
}

pub(crate) async fn sample() -> SqlitePool {
    let pool = db::test_pool().await;
    company::setup(
        &pool,
        SetupInput { name: "PT Dagang Maju".into(), fiscal_year_start_month: 1, template: "standard".into() },
    )
    .await
    .unwrap();
    opening::save(
        &pool,
        OpeningInput {
            date: "2026-11-01".into(),
            lines: vec![
                line(acc(&pool, "1-1100").await, 50 * JT, 0),
                line(acc(&pool, "1-1200").await, 100 * JT, 0),
                line(acc(&pool, "1-1400").await, 30 * JT, 0),
                line(acc(&pool, "1-2400").await, 20 * JT, 0),
                line(acc(&pool, "2-1100").await, 0, 40 * JT),
            ],
        },
    )
    .await
    .unwrap();
    post(&pool, "2026-11-03", "5-1100", "2-1100", 25 * JT).await; // pembelian kredit
    post(&pool, "2026-11-05", "5-1200", "1-1100", JT).await; // angkut
    post(&pool, "2026-11-10", "1-1100", "4-1000", 60 * JT).await; // penjualan tunai
    post(&pool, "2026-11-12", "1-1300", "4-1000", 40 * JT).await; // penjualan kredit
    post(&pool, "2026-11-15", "4-1100", "1-1300", 2 * JT).await; // retur
    post(&pool, "2026-11-20", "5-1000", "1-1400", 20 * JT).await; // HPP
    post(&pool, "2026-11-25", "6-1000", "1-1200", 8 * JT).await; // gaji
    post(&pool, "2026-11-26", "6-1100", "1-1100", 3 * JT).await; // sewa
    post(&pool, "2026-11-28", "6-1400", "1-2410", JT / 2).await; // penyusutan
    post(&pool, "2026-11-30", "1-1200", "1-1300", 20 * JT).await; // terima piutang
    pool
}

fn find<'a>(lines: &'a [StatementLine], label: &str) -> &'a StatementLine {
    lines.iter().find(|l| l.label == label).unwrap_or_else(|| panic!("baris '{label}' tidak ada"))
}

fn amount(lines: &[StatementLine], label: &str) -> i64 {
    find(lines, label).amount.unwrap()
}

fn iq(from: &str, to: &str) -> IncomeStatementQuery {
    IncomeStatementQuery { from: Some(from.into()), to: Some(to.into()), ..Default::default() }
}

// ---------- Laba Rugi ----------

#[tokio::test]
async fn laba_rugi_sampel_cocok_dengan_hitungan_manual() {
    let pool = sample().await;
    let is = statements::income_statement(&pool, iq("2026-11-01", "2026-11-30")).await.unwrap();

    assert_eq!(is.total_revenue, 98 * JT);
    assert_eq!(is.total_expense, 57 * JT + JT / 2);
    assert_eq!(is.net_income, 40 * JT + JT / 2);
    assert!(!is.has_compare);

    assert_eq!(amount(&is.lines, "Penjualan"), 100 * JT);
    assert_eq!(amount(&is.lines, "Retur dan Potongan Penjualan"), -2 * JT, "akun kontra pendapatan bernilai negatif");
    assert_eq!(amount(&is.lines, "Total Pendapatan"), 98 * JT);
    assert_eq!(amount(&is.lines, "Jumlah HARGA POKOK PENJUALAN"), 46 * JT);
    assert_eq!(amount(&is.lines, "Jumlah BEBAN OPERASIONAL"), 11 * JT + JT / 2);
    assert_eq!(amount(&is.lines, "Total Beban"), 57 * JT + JT / 2);
    assert_eq!(amount(&is.lines, "Laba (Rugi) Bersih"), 40 * JT + JT / 2);
    assert_eq!(find(&is.lines, "Laba (Rugi) Bersih").kind, LineKind::Net);

    // Akun bersaldo nol tidak tampil (mis. Beban Bunga); akun bernilai memuat kode dan id.
    assert!(is.lines.iter().all(|l| l.label != "Beban Bunga"));
    let gaji = find(&is.lines, "Beban Gaji");
    assert_eq!((gaji.code.as_deref(), gaji.kind), (Some("6-1000"), LineKind::Account));
    assert!(gaji.account_id.is_some());
}

#[tokio::test]
async fn laba_rugi_rentang_tanggal_dan_tanpa_draft() {
    let pool = sample().await;
    // Hanya penjualan dan retur (10-15 Nov); belum ada beban.
    let is = statements::income_statement(&pool, iq("2026-11-10", "2026-11-15")).await.unwrap();
    assert_eq!((is.total_revenue, is.total_expense, is.net_income), (98 * JT, 0, 98 * JT));

    // Draft tidak dihitung.
    let draft = JournalInput {
        date: "2026-11-12".into(),
        description: "draft".into(),
        reference: None,
        lines: vec![line(acc(&pool, "1-1100").await, 777 * JT, 0), line(acc(&pool, "4-1000").await, 0, 777 * JT)],
    };
    journal::create(&pool, draft, false).await.unwrap();
    let again = statements::income_statement(&pool, iq("2026-11-10", "2026-11-15")).await.unwrap();
    assert_eq!(again.net_income, 98 * JT);

    // Periode kosong.
    let empty = statements::income_statement(&pool, iq("2026-01-01", "2026-01-31")).await.unwrap();
    assert_eq!((empty.total_revenue, empty.total_expense, empty.net_income), (0, 0, 0));
}

#[tokio::test]
async fn laba_rugi_dengan_pembanding() {
    let pool = sample().await;
    let q = IncomeStatementQuery {
        from: Some("2026-11-16".into()),
        to: Some("2026-11-30".into()),
        compare_from: Some("2026-11-01".into()),
        compare_to: Some("2026-11-15".into()),
    };
    let is = statements::income_statement(&pool, q).await.unwrap();
    assert!(is.has_compare);
    // Periode ini (16-30 Nov): tanpa pendapatan; beban HPP 20 + gaji 8 + sewa 3 + penyusutan 0,5.
    assert_eq!(is.net_income, -(31 * JT + JT / 2));
    // Pembanding (1-15 Nov): pendapatan 98; beban pembelian 25 + angkut 1.
    assert_eq!(is.compare_net_income, Some(72 * JT));

    let penjualan = find(&is.lines, "Penjualan");
    assert_eq!((penjualan.amount, penjualan.compare), (Some(0), Some(100 * JT)));
    let total = find(&is.lines, "Total Pendapatan");
    assert_eq!((total.amount, total.compare), (Some(0), Some(98 * JT)));
    // Akun yang hanya ada di salah satu periode tetap tampil.
    assert_eq!(find(&is.lines, "Pembelian").amount, Some(0));
    assert_eq!(find(&is.lines, "Pembelian").compare, Some(25 * JT));
    assert_eq!(find(&is.lines, "Beban Gaji").compare, Some(0));
}

#[tokio::test]
async fn laba_rugi_akun_tanpa_induk_tetap_masuk() {
    let pool = sample().await;
    let lonely = accounts::create(
        &pool,
        accounts::AccountInput {
            code: "4-9999".into(),
            name: "Pendapatan Yatim".into(),
            account_type: "REVENUE".into(),
            normal_balance: None,
            parent_id: None,
            is_header: false,
            is_cash: false,
        },
    )
    .await
    .unwrap();
    post(&pool, "2026-11-29", "1-1100", "4-9999", 5 * JT).await;
    let is = statements::income_statement(&pool, iq("2026-11-01", "2026-11-30")).await.unwrap();
    assert_eq!(find(&is.lines, "Pendapatan Yatim").account_id, Some(lonely.id));
    assert_eq!(is.total_revenue, 103 * JT);
}

#[tokio::test]
async fn laporan_menolak_tanggal_tidak_valid() {
    let pool = sample().await;
    assert!(statements::income_statement(&pool, iq("2026-12-01", "2026-11-01")).await.is_err());
    assert!(statements::income_statement(&pool, iq("xx", "2026-11-01")).await.is_err());
    let bad = BalanceSheetQuery { as_of: Some("31-12-2026".into()) };
    assert!(statements::balance_sheet(&pool, bad).await.is_err());
}

// ---------- Neraca ----------

fn bq(as_of: &str) -> BalanceSheetQuery {
    BalanceSheetQuery { as_of: Some(as_of.into()) }
}

#[tokio::test]
async fn neraca_sampel_cocok_dengan_hitungan_manual() {
    let pool = sample().await;
    let bs = statements::balance_sheet(&pool, bq("2026-11-30")).await.unwrap();

    assert_eq!(bs.total_assets, 265 * JT + JT / 2);
    assert_eq!(bs.total_liabilities, 65 * JT);
    assert_eq!(bs.current_earnings, 40 * JT + JT / 2);
    assert_eq!(bs.total_equity, 200 * JT + JT / 2);
    assert_eq!(bs.total_liabilities_equity, 265 * JT + JT / 2);
    assert!(bs.balanced);
    assert_eq!(bs.difference, 0);

    assert_eq!(amount(&bs.lines, "Kas"), 106 * JT);
    assert_eq!(amount(&bs.lines, "Bank"), 112 * JT);
    assert_eq!(amount(&bs.lines, "Piutang Usaha"), 18 * JT);
    assert_eq!(amount(&bs.lines, "Persediaan Barang Dagang"), 10 * JT);
    assert_eq!(amount(&bs.lines, "Akumulasi Penyusutan Peralatan"), -JT / 2, "akun kontra aset bernilai negatif");
    assert_eq!(amount(&bs.lines, "Jumlah Aset Lancar"), 246 * JT);
    assert_eq!(amount(&bs.lines, "Jumlah Aset Tetap"), 19 * JT + JT / 2);
    assert_eq!(amount(&bs.lines, "Total Aset"), 265 * JT + JT / 2);
    assert_eq!(amount(&bs.lines, "Utang Usaha"), 65 * JT);
    assert_eq!(amount(&bs.lines, "Modal Pemilik"), 160 * JT);
    let laba = find(&bs.lines, "Laba (Rugi) Berjalan");
    assert_eq!((laba.amount, laba.account_id, laba.code.clone()), (Some(40 * JT + JT / 2), None, None));
    assert_eq!(amount(&bs.lines, "Total Ekuitas"), 200 * JT + JT / 2);
    assert_eq!(amount(&bs.lines, "Total Kewajiban dan Ekuitas"), 265 * JT + JT / 2);
}

#[tokio::test]
async fn neraca_per_tanggal_selalu_seimbang() {
    let pool = sample().await;
    for d in ["2026-10-31", "2026-11-01", "2026-11-03", "2026-11-12", "2026-11-20", "2026-11-30", "2027-06-30"] {
        let bs = statements::balance_sheet(&pool, bq(d)).await.unwrap();
        assert!(bs.balanced, "neraca per {d} tidak seimbang: selisih {}", bs.difference);
    }
    // Sebelum saldo awal: kosong tapi seimbang.
    let bs = statements::balance_sheet(&pool, bq("2026-10-31")).await.unwrap();
    assert_eq!((bs.total_assets, bs.total_liabilities_equity), (0, 0));
    // Per 3 Nov (pembelian kredit 25jt): aset 200jt, utang 65jt, modal 160jt, rugi berjalan -25jt.
    let bs = statements::balance_sheet(&pool, bq("2026-11-03")).await.unwrap();
    assert_eq!((bs.total_assets, bs.total_liabilities, bs.current_earnings), (200 * JT, 65 * JT, -25 * JT));
    assert_eq!(bs.total_equity, 135 * JT);
}

#[tokio::test]
async fn jurnal_penutup_tidak_mengubah_neraca_dan_tidak_menghapus_laba_rugi() {
    let pool = sample().await;
    let before = statements::balance_sheet(&pool, bq("2026-12-31")).await.unwrap();
    assert_eq!(before.current_earnings, 40 * JT + JT / 2);

    // Jurnal penutup tahunan: nolkan semua akun pendapatan/beban, laba ke Laba Ditahan.
    sqlx::query("INSERT INTO journal_entries (number, date, description, status, type) VALUES ('TB-2026-12-0001', '2026-12-31', 'Penutup', 'POSTED', 'CLOSING')")
        .execute(&pool)
        .await
        .unwrap();
    let entry_id: i64 = sqlx::query_scalar("SELECT id FROM journal_entries WHERE type = 'CLOSING'").fetch_one(&pool).await.unwrap();
    let mut lines = vec![(acc(&pool, "4-1000").await, 100 * JT, 0), (acc(&pool, "4-1100").await, 0, 2 * JT)];
    for (code, amt) in [("5-1000", 20), ("5-1100", 25), ("5-1200", 1), ("6-1000", 8), ("6-1100", 3)] {
        lines.push((acc(&pool, code).await, 0, amt * JT));
    }
    lines.push((acc(&pool, "6-1400").await, 0, JT / 2));
    lines.push((acc(&pool, "3-3000").await, 0, 40 * JT + JT / 2));
    for (a, d, c) in lines {
        sqlx::query("INSERT INTO journal_lines (entry_id, account_id, debit, credit) VALUES (?, ?, ?, ?)")
            .bind(entry_id)
            .bind(a)
            .bind(d)
            .bind(c)
            .execute(&pool)
            .await
            .unwrap();
    }

    // Neraca: sama totalnya; laba pindah dari "laba berjalan" ke Laba Ditahan.
    let after = statements::balance_sheet(&pool, bq("2026-12-31")).await.unwrap();
    assert!(after.balanced);
    assert_eq!(after.total_assets, before.total_assets);
    assert_eq!(after.total_equity, before.total_equity);
    assert_eq!(after.current_earnings, 0);
    assert_eq!(amount(&after.lines, "Laba Ditahan"), 40 * JT + JT / 2);
    assert!(after.lines.iter().all(|l| l.label != "Laba (Rugi) Berjalan"));

    // Laba Rugi untuk periode berjalan tetap menunjukkan laba (jurnal penutup dikecualikan).
    let is = statements::income_statement(&pool, iq("2026-11-01", "2026-12-31")).await.unwrap();
    assert_eq!(is.net_income, 40 * JT + JT / 2);
    assert_eq!(is.total_revenue, 98 * JT);

    // Neraca saldo setelah penutupan: akun pendapatan/beban nol, tetap seimbang.
    let tb = reports::trial_balance(&pool, TrialBalanceQuery { as_of: Some("2026-12-31".into()), include_zero: false })
        .await
        .unwrap();
    assert!(tb.balanced);
    assert!(tb.rows.iter().all(|r| !r.code.starts_with(['4', '5', '6'])));
}

#[tokio::test]
async fn neraca_seimbang_setelah_edit_dan_hapus() {
    let pool = sample().await;
    let page = journal::list(&pool, journal::JournalFilter { page: 1, page_size: 50, entry_type: Some("GENERAL".into()), ..Default::default() })
        .await
        .unwrap();
    // Hapus satu jurnal dan ubah nominal jurnal lain.
    journal::delete(&pool, page.items[0].id).await.unwrap();
    let j = journal::get(&pool, page.items[1].id).await.unwrap();
    let (a, b) = (j.lines[0].clone(), j.lines[1].clone());
    let input = JournalInput {
        date: j.entry.date.clone(),
        description: "diubah".into(),
        reference: None,
        lines: vec![
            line(a.account_id, if a.debit > 0 { 7 * JT } else { 0 }, if a.credit > 0 { 7 * JT } else { 0 }),
            line(b.account_id, if b.debit > 0 { 7 * JT } else { 0 }, if b.credit > 0 { 7 * JT } else { 0 }),
        ],
    };
    journal::update(&pool, j.entry.id, input, false).await.unwrap();

    let bs = statements::balance_sheet(&pool, BalanceSheetQuery::default()).await.unwrap();
    assert!(bs.balanced, "selisih {}", bs.difference);
}
