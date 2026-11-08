//! Memakai data contoh November 2026 dari tests_statements (hasil dihitung manual):
//! kas 106jt + bank 112jt = 218jt; pendapatan bulan ini 98jt; beban 57,5jt; laba 40,5jt.

use super::accounts::{self, AccountInput};
use super::dashboard::{self, DashboardQuery};
use super::journal::{self, JournalInput, LineInput};
use super::tests_statements::sample;
use crate::db;

const JT: i64 = 100_000_000;

fn q(today: &str) -> DashboardQuery {
    DashboardQuery { today: Some(today.into()) }
}

async fn acc(pool: &sqlx::SqlitePool, code: &str) -> i64 {
    accounts::list(pool).await.unwrap().into_iter().find(|a| a.code == code).unwrap().id
}

async fn post(pool: &sqlx::SqlitePool, date: &str, dr: &str, cr: &str, amount: i64) {
    let input = JournalInput {
        date: date.into(),
        description: format!("{dr} / {cr}"),
        reference: None,
        lines: vec![
            LineInput { account_id: acc(pool, dr).await, debit: amount, credit: 0, memo: None },
            LineInput { account_id: acc(pool, cr).await, debit: 0, credit: amount, memo: None },
        ],
    };
    journal::create(pool, input, true).await.unwrap();
}

#[tokio::test]
async fn kartu_ringkasan_cocok_dengan_hitungan_manual() {
    let pool = sample().await;
    let d = dashboard::dashboard(&pool, q("2026-11-30")).await.unwrap();

    assert!(d.has_data);
    assert_eq!(d.month, "2026-11");
    assert_eq!(d.cash_balance, 218 * JT);
    let codes: Vec<(&str, i64)> = d.cash_accounts.iter().map(|a| (a.code.as_str(), a.balance)).collect();
    assert_eq!(codes, [("1-1100", 106 * JT), ("1-1200", 112 * JT)]);
    assert_eq!((d.revenue, d.expense, d.net), (98 * JT, 57 * JT + JT / 2, 40 * JT + JT / 2));
    // Oktober kosong.
    assert_eq!((d.prev_revenue, d.prev_expense, d.prev_net), (0, 0, 0));
}

#[tokio::test]
async fn tren_12_bulan_dan_saldo_kas_akhir_bulan() {
    let pool = sample().await;
    let d = dashboard::dashboard(&pool, q("2026-11-30")).await.unwrap();

    assert_eq!(d.monthly.len(), 12);
    assert_eq!(d.monthly.first().unwrap().month, "2025-12");
    assert_eq!(d.monthly.last().unwrap().month, "2026-11");
    // Bulan-bulan sebelum November kosong; saldo kas baru ada setelah saldo awal 1 November.
    assert!(d.monthly[..11].iter().all(|m| m.revenue == 0 && m.expense == 0 && m.cash == 0));
    let nov = d.monthly.last().unwrap();
    assert_eq!((nov.revenue, nov.expense, nov.net, nov.cash), (98 * JT, 57 * JT + JT / 2, 40 * JT + JT / 2, 218 * JT));
}

#[tokio::test]
async fn kas_kumulatif_antar_bulan_dan_lintas_tahun() {
    let pool = sample().await;
    // Desember (tahun berikutnya): penjualan tunai 10jt.
    post(&pool, "2026-12-10", "1-1100", "4-1000", 10 * JT).await;
    let d = dashboard::dashboard(&pool, q("2027-01-15")).await.unwrap();

    assert_eq!(d.month, "2027-01");
    assert_eq!(d.monthly.first().unwrap().month, "2026-02");
    let by = |m: &str| d.monthly.iter().find(|p| p.month == m).unwrap().clone();
    assert_eq!(by("2026-11").cash, 218 * JT);
    assert_eq!(by("2026-12").cash, 228 * JT);
    assert_eq!(by("2026-12").revenue, 10 * JT);
    assert_eq!(by("2027-01").cash, 228 * JT, "bulan tanpa transaksi membawa saldo bulan lalu");
    // Kartu: Januari kosong, bulan lalu (Desember) berisi.
    assert_eq!(d.revenue, 0);
    assert_eq!(d.prev_revenue, 10 * JT);
    assert_eq!(d.cash_balance, 228 * JT);
}

#[tokio::test]
async fn saldo_sebelum_jendela_12_bulan_ikut_terhitung() {
    let pool = sample().await;
    // Hari ini 2028-03: November 2026 sudah di luar jendela, tetapi saldonya harus tetap terbawa.
    let d = dashboard::dashboard(&pool, q("2028-03-05")).await.unwrap();
    assert_eq!(d.monthly.first().unwrap().month, "2027-04");
    assert!(d.monthly.iter().all(|m| m.cash == 218 * JT));
    assert_eq!(d.cash_balance, 218 * JT);
    assert!(d.monthly.iter().all(|m| m.revenue == 0));
}

#[tokio::test]
async fn komposisi_beban_top_5_plus_lainnya() {
    let pool = sample().await;
    let d = dashboard::dashboard(&pool, q("2026-11-30")).await.unwrap();
    let s: Vec<(&str, i64)> = d.expense_breakdown.iter().map(|x| (x.label.as_str(), x.amount)).collect();
    assert_eq!(
        s,
        [
            ("Pembelian", 25 * JT),
            ("Harga Pokok Penjualan", 20 * JT),
            ("Beban Gaji", 8 * JT),
            ("Beban Sewa", 3 * JT),
            ("Beban Angkut Pembelian", JT),
            ("Lainnya", JT / 2),
        ]
    );
    assert_eq!(d.expense_breakdown.iter().map(|x| x.amount).sum::<i64>(), d.expense);
    assert!(d.expense_breakdown.last().unwrap().code.is_none());
    assert!(d.expense_breakdown[0].code.is_some());
}

#[tokio::test]
async fn draft_dan_jurnal_penutup_tidak_dihitung() {
    let pool = sample().await;
    let (kas, jual) = (acc(&pool, "1-1100").await, acc(&pool, "4-1000").await);
    let draft = JournalInput {
        date: "2026-11-15".into(),
        description: "draft".into(),
        reference: None,
        lines: vec![
            LineInput { account_id: kas, debit: 777 * JT, credit: 0, memo: None },
            LineInput { account_id: jual, debit: 0, credit: 777 * JT, memo: None },
        ],
    };
    journal::create(&pool, draft, false).await.unwrap();

    // Jurnal penutup: nolkan pendapatan ke Laba Ditahan.
    sqlx::query("INSERT INTO journal_entries (number, date, description, status, type) VALUES ('TB-1', '2026-11-30', 'Penutup', 'POSTED', 'CLOSING')")
        .execute(&pool)
        .await
        .unwrap();
    let id: i64 = sqlx::query_scalar("SELECT id FROM journal_entries WHERE type = 'CLOSING'").fetch_one(&pool).await.unwrap();
    for (a, d, c) in [(jual, 100 * JT, 0), (acc(&pool, "3-3000").await, 0, 100 * JT)] {
        sqlx::query("INSERT INTO journal_lines (entry_id, account_id, debit, credit) VALUES (?, ?, ?, ?)")
            .bind(id).bind(a).bind(d).bind(c).execute(&pool).await.unwrap();
    }

    let d = dashboard::dashboard(&pool, q("2026-11-30")).await.unwrap();
    assert_eq!(d.draft_count, 1);
    assert_eq!(d.revenue, 98 * JT, "draft tidak masuk dan jurnal penutup tidak menghapus pendapatan");
    assert_eq!(d.cash_balance, 218 * JT);
}

#[tokio::test]
async fn jurnal_terakhir_dan_urutannya() {
    let pool = sample().await;
    let d = dashboard::dashboard(&pool, q("2026-11-30")).await.unwrap();
    assert_eq!(d.recent.len(), 5);
    let dates: Vec<&str> = d.recent.iter().map(|j| j.date.as_str()).collect();
    assert_eq!(dates, ["2026-11-30", "2026-11-28", "2026-11-26", "2026-11-25", "2026-11-20"]);
    assert_eq!(d.recent[0].total, 20 * JT);
}

#[tokio::test]
async fn penanda_kas_bisa_diubah_pengguna() {
    let pool = sample().await;
    let piutang = accounts::list(&pool).await.unwrap().into_iter().find(|a| a.code == "1-1300").unwrap();
    // Tandai piutang sebagai kas (mis. akun e-wallet) -> ikut kartu Kas & Bank.
    accounts::update(
        &pool,
        piutang.id,
        AccountInput {
            code: piutang.code.clone(),
            name: "E-Wallet".into(),
            account_type: "ASSET".into(),
            normal_balance: None,
            parent_id: piutang.parent_id,
            is_header: false,
            is_cash: true,
        },
    )
    .await
    .unwrap();
    let d = dashboard::dashboard(&pool, q("2026-11-30")).await.unwrap();
    assert_eq!(d.cash_balance, (218 + 18) * JT);
    assert_eq!(d.cash_accounts.len(), 3);

    // Hanya akun aset non-header yang boleh ditandai kas.
    let jual = accounts::list(&pool).await.unwrap().into_iter().find(|a| a.code == "4-1000").unwrap();
    let bad = AccountInput {
        code: jual.code.clone(),
        name: jual.name.clone(),
        account_type: "REVENUE".into(),
        normal_balance: None,
        parent_id: jual.parent_id,
        is_header: false,
        is_cash: true,
    };
    assert!(accounts::update(&pool, jual.id, bad).await.is_err());
}

#[tokio::test]
async fn database_kosong_dan_tanggal_tidak_valid() {
    let pool = db::test_pool().await;
    let d = dashboard::dashboard(&pool, q("2026-11-08")).await.unwrap();
    assert!(!d.has_data);
    assert_eq!(d.monthly.len(), 12);
    assert!(d.monthly.iter().all(|m| m.revenue == 0 && m.expense == 0 && m.cash == 0));
    assert!(d.expense_breakdown.is_empty() && d.recent.is_empty() && d.cash_accounts.is_empty());
    assert_eq!((d.cash_balance, d.draft_count), (0, 0));

    assert!(dashboard::dashboard(&pool, q("08/11/2026")).await.is_err());
    // Tanpa parameter: memakai tanggal lokal (tidak error).
    assert!(dashboard::dashboard(&pool, DashboardQuery::default()).await.is_ok());
}
