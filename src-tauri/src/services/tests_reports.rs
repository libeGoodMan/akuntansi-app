use sqlx::SqlitePool;

use super::accounts;
use super::company::{self, SetupInput};
use super::journal::{self, JournalInput, LineInput};
use super::opening::{self, OpeningInput};
use super::reports::{self, LedgerQuery, TrialBalanceQuery};
use crate::db;

async fn setup() -> SqlitePool {
    let pool = db::test_pool().await;
    company::setup(
        &pool,
        SetupInput { name: "PT Contoh".into(), fiscal_year_start_month: 1, template: "standard".into() },
    )
    .await
    .unwrap();
    pool
}

async fn acc(pool: &SqlitePool, code: &str) -> i64 {
    accounts::list(pool).await.unwrap().into_iter().find(|a| a.code == code).unwrap().id
}

fn line(account_id: i64, debit: i64, credit: i64) -> LineInput {
    LineInput { account_id, debit, credit, memo: None }
}

/// Jurnal dua baris: `dr` didebit, `cr` dikredit, sebesar `amount` sen.
async fn post(pool: &SqlitePool, date: &str, dr: &str, cr: &str, amount: i64) -> i64 {
    let input = JournalInput {
        date: date.into(),
        description: format!("{dr} / {cr}"),
        reference: None,
        lines: vec![line(acc(pool, dr).await, amount, 0), line(acc(pool, cr).await, 0, amount)],
    };
    journal::create(pool, input, true).await.unwrap().entry.id
}

fn ledger_q(account_id: i64, from: Option<&str>, to: Option<&str>) -> LedgerQuery {
    LedgerQuery {
        account_id,
        from: from.map(Into::into),
        to: to.map(Into::into),
        page: 1,
        page_size: 100,
    }
}

fn tb_q(as_of: Option<&str>, include_zero: bool) -> TrialBalanceQuery {
    TrialBalanceQuery { as_of: as_of.map(Into::into), include_zero }
}


/// (saldo debit Kas, apakah seimbang) pada tanggal tertentu.
async fn kas_debit_at(pool: &SqlitePool, d: Option<&str>) -> (i64, bool) {
    let tb = reports::trial_balance(pool, tb_q(d, false)).await.unwrap();
    (tb.rows.iter().find(|r| r.code == "1-1100").map_or(0, |r| r.balance_debit), tb.balanced)
}

// ---------- Buku Besar ----------

#[tokio::test]
async fn saldo_berjalan_akun_debit_dan_kredit() {
    let pool = setup().await;
    let (kas, jual) = (acc(&pool, "1-1100").await, acc(&pool, "4-1000").await);
    post(&pool, "2026-11-01", "1-1100", "4-1000", 100_000).await; // kas +100rb
    post(&pool, "2026-11-02", "1-1100", "4-1000", 50_000).await; // kas +50rb
    post(&pool, "2026-11-03", "5-1100", "1-1100", 30_000).await; // kas -30rb

    let l = reports::ledger(&pool, ledger_q(kas, None, None)).await.unwrap();
    assert_eq!(l.opening_balance, 0);
    assert_eq!(l.rows.iter().map(|r| r.balance).collect::<Vec<_>>(), [100_000, 150_000, 120_000]);
    assert_eq!((l.total_debit, l.total_credit, l.closing_balance), (150_000, 30_000, 120_000));
    assert_eq!(l.total, 3);

    // Akun saldo normal kredit: penjualan bertambah di kredit.
    let l = reports::ledger(&pool, ledger_q(jual, None, None)).await.unwrap();
    assert_eq!(l.rows.iter().map(|r| r.balance).collect::<Vec<_>>(), [100_000, 150_000]);
    assert_eq!(l.closing_balance, 150_000);
}

#[tokio::test]
async fn saldo_awal_periode_dari_mutasi_sebelum_tanggal_awal() {
    let pool = setup().await;
    let kas = acc(&pool, "1-1100").await;
    post(&pool, "2026-10-15", "1-1100", "4-1000", 70_000).await;
    post(&pool, "2026-11-05", "1-1100", "4-1000", 20_000).await;
    post(&pool, "2026-11-20", "5-1100", "1-1100", 5_000).await;

    let l = reports::ledger(&pool, ledger_q(kas, Some("2026-11-01"), Some("2026-11-30"))).await.unwrap();
    assert_eq!(l.opening_balance, 70_000);
    assert_eq!(l.rows.iter().map(|r| r.balance).collect::<Vec<_>>(), [90_000, 85_000]);
    assert_eq!(l.closing_balance, 85_000);
    assert_eq!((l.total_debit, l.total_credit), (20_000, 5_000));
}

#[tokio::test]
async fn batas_tanggal_inklusif() {
    let pool = setup().await;
    let kas = acc(&pool, "1-1100").await;
    for d in ["2026-11-01", "2026-11-02", "2026-11-03"] {
        post(&pool, d, "1-1100", "4-1000", 1_000).await;
    }
    let l = reports::ledger(&pool, ledger_q(kas, Some("2026-11-02"), Some("2026-11-02"))).await.unwrap();
    assert_eq!(l.rows.len(), 1);
    assert_eq!(l.rows[0].date, "2026-11-02");
    assert_eq!(l.opening_balance, 1_000);
    assert_eq!(l.closing_balance, 2_000);
}

#[tokio::test]
async fn draft_tidak_masuk_buku_besar_dan_perubahan_langsung_terlihat() {
    let pool = setup().await;
    let (kas, jual) = (acc(&pool, "1-1100").await, acc(&pool, "4-1000").await);
    let draft = JournalInput {
        date: "2026-11-01".into(),
        description: "draft".into(),
        reference: None,
        lines: vec![line(kas, 999, 0), line(jual, 0, 999)],
    };
    journal::create(&pool, draft, false).await.unwrap();
    assert_eq!(reports::ledger(&pool, ledger_q(kas, None, None)).await.unwrap().total, 0);

    let id = post(&pool, "2026-11-02", "1-1100", "4-1000", 500).await;
    assert_eq!(reports::ledger(&pool, ledger_q(kas, None, None)).await.unwrap().closing_balance, 500);

    // Edit jurnal POSTED -> buku besar ikut berubah; hapus -> hilang.
    let edited = JournalInput {
        date: "2026-11-02".into(),
        description: "edit".into(),
        reference: None,
        lines: vec![line(kas, 800, 0), line(jual, 0, 800)],
    };
    journal::update(&pool, id, edited, false).await.unwrap();
    assert_eq!(reports::ledger(&pool, ledger_q(kas, None, None)).await.unwrap().closing_balance, 800);
    journal::delete(&pool, id).await.unwrap();
    assert_eq!(reports::ledger(&pool, ledger_q(kas, None, None)).await.unwrap().total, 0);
}

#[tokio::test]
async fn saldo_berjalan_benar_di_setiap_halaman() {
    let pool = setup().await;
    let kas = acc(&pool, "1-1100").await;
    for day in 1..=7 {
        post(&pool, &format!("2026-11-{day:02}"), "1-1100", "4-1000", day * 100).await;
    }
    let page = |p| LedgerQuery { account_id: kas, page: p, page_size: 3, ..Default::default() };
    let (p1, p2, p3) = (
        reports::ledger(&pool, page(1)).await.unwrap(),
        reports::ledger(&pool, page(2)).await.unwrap(),
        reports::ledger(&pool, page(3)).await.unwrap(),
    );
    let all: Vec<i64> = [&p1, &p2, &p3].iter().flat_map(|p| p.rows.iter().map(|r| r.balance)).collect();
    assert_eq!(all, [100, 300, 600, 1000, 1500, 2100, 2800]);
    assert_eq!((p1.rows.len(), p2.rows.len(), p3.rows.len()), (3, 3, 1));
    assert_eq!(p1.total, 7);
    // Total dan saldo akhir sama di setiap halaman.
    assert!([&p1, &p2, &p3].iter().all(|p| p.closing_balance == 2800 && p.total_debit == 2800));
}

#[tokio::test]
async fn akun_header_menggabungkan_turunannya() {
    let pool = setup().await;
    let lancar = acc(&pool, "1-1000").await; // header: Kas, Bank, Piutang, ...
    post(&pool, "2026-11-01", "1-1100", "4-1000", 1_000).await; // kas
    post(&pool, "2026-11-02", "1-1200", "4-1000", 2_000).await; // bank
    post(&pool, "2026-11-03", "1-1300", "4-1000", 4_000).await; // piutang
    post(&pool, "2026-11-04", "5-1100", "1-1100", 500).await; // kas keluar

    let l = reports::ledger(&pool, ledger_q(lancar, None, None)).await.unwrap();
    assert!(l.account.is_header);
    assert_eq!(l.rows.iter().map(|r| r.account_code.as_str()).collect::<Vec<_>>(), ["1-1100", "1-1200", "1-1300", "1-1100"]);
    assert_eq!(l.closing_balance, 1_000 + 2_000 + 4_000 - 500);
}

#[tokio::test]
async fn buku_besar_menolak_input_tidak_valid() {
    let pool = setup().await;
    let kas = acc(&pool, "1-1100").await;
    assert!(reports::ledger(&pool, ledger_q(9999, None, None)).await.is_err());
    assert!(reports::ledger(&pool, ledger_q(kas, Some("xx"), None)).await.is_err());
    assert!(reports::ledger(&pool, ledger_q(kas, Some("2026-12-01"), Some("2026-11-01"))).await.is_err());
}

#[tokio::test]
async fn buku_besar_menyertakan_saldo_awal_dan_urutan_stabil() {
    let pool = setup().await;
    let (kas, utang) = (acc(&pool, "1-1100").await, acc(&pool, "2-1100").await);
    opening::save(
        &pool,
        OpeningInput { date: "2026-01-01".into(), lines: vec![line(kas, 1_000_000, 0), line(utang, 0, 400_000)] },
    )
    .await
    .unwrap();
    post(&pool, "2026-01-01", "1-1100", "4-1000", 5_000).await; // tanggal sama dengan saldo awal

    let l = reports::ledger(&pool, ledger_q(kas, None, None)).await.unwrap();
    assert_eq!(l.rows.len(), 2);
    assert!(l.rows[0].number.starts_with("SA-"), "saldo awal dulu (id lebih kecil)");
    assert_eq!(l.rows[0].balance, 1_000_000);
    assert_eq!(l.closing_balance, 1_005_000);
}

// ---------- Neraca Saldo ----------

#[tokio::test]
async fn neraca_saldo_menaruh_saldo_di_kolom_yang_benar() {
    let pool = setup().await;
    post(&pool, "2026-11-01", "1-1100", "4-1000", 100_000).await; // kas D, penjualan K
    post(&pool, "2026-11-02", "6-1100", "1-1100", 30_000).await; // beban sewa D, kas K
    post(&pool, "2026-11-03", "6-1400", "1-2210", 8_000).await; // beban penyusutan D, akumulasi K (akun kontra)

    let tb = reports::trial_balance(&pool, tb_q(None, false)).await.unwrap();
    let get = |code: &str| tb.rows.iter().find(|r| r.code == code).unwrap();
    assert_eq!((get("1-1100").balance_debit, get("1-1100").balance_credit), (70_000, 0));
    assert_eq!((get("4-1000").balance_debit, get("4-1000").balance_credit), (0, 100_000));
    assert_eq!((get("6-1100").balance_debit, get("6-1100").balance_credit), (30_000, 0));
    assert_eq!((get("1-2210").balance_debit, get("1-2210").balance_credit), (0, 8_000));
    assert!(tb.balanced);
    assert_eq!(tb.difference, 0);
    assert_eq!(tb.total_debit, 70_000 + 30_000 + 8_000);

    // Aset bersaldo kredit (kas minus) pindah ke kolom kredit.
    post(&pool, "2026-11-04", "6-1100", "1-1100", 100_000).await;
    let tb = reports::trial_balance(&pool, tb_q(None, false)).await.unwrap();
    let kas = tb.rows.iter().find(|r| r.code == "1-1100").unwrap();
    assert_eq!((kas.balance_debit, kas.balance_credit), (0, 30_000));
    assert!(tb.balanced);
}

#[tokio::test]
async fn neraca_saldo_per_tanggal_dan_tanpa_draft() {
    let pool = setup().await;
    post(&pool, "2026-10-31", "1-1100", "4-1000", 1_000).await;
    post(&pool, "2026-11-15", "1-1100", "4-1000", 2_000).await;
    let (kas, jual) = (acc(&pool, "1-1100").await, acc(&pool, "4-1000").await);
    let draft = JournalInput {
        date: "2026-10-01".into(),
        description: "draft".into(),
        reference: None,
        lines: vec![line(kas, 777, 0), line(jual, 0, 777)],
    };
    journal::create(&pool, draft, false).await.unwrap();

    assert_eq!(kas_debit_at(&pool, Some("2026-10-30")).await, (0, true)); // sebelum transaksi apa pun (draft diabaikan)
    assert_eq!(kas_debit_at(&pool, Some("2026-10-31")).await, (1_000, true)); // batas inklusif
    assert_eq!(kas_debit_at(&pool, Some("2026-11-14")).await, (1_000, true));
    assert_eq!(kas_debit_at(&pool, Some("2026-11-15")).await, (3_000, true));
    assert_eq!(kas_debit_at(&pool, None).await, (3_000, true));
}

#[tokio::test]
async fn neraca_saldo_selalu_seimbang_setelah_banyak_perubahan() {
    let pool = setup().await;
    let codes = ["1-1100", "1-1200", "1-1300", "2-1100", "3-1000", "4-1000", "5-1100", "6-1100", "6-1400", "1-2210"];
    // Pembuat angka semu yang deterministik.
    let mut x: i64 = 12345;
    let mut next = |m: i64| {
        x = (x * 1103515245 + 12345) % 2147483648;
        x % m
    };
    let mut ids = vec![];
    for i in 0..60 {
        let (a, mut b) = (next(codes.len() as i64) as usize, next(codes.len() as i64) as usize);
        if a == b {
            b = (b + 1) % codes.len();
        }
        let amount = 1 + next(5_000_000);
        let day = 1 + (i % 28);
        ids.push(post(&pool, &format!("2026-11-{day:02}"), codes[a], codes[b], amount).await);
    }
    // Hapus beberapa jurnal dan edit beberapa lainnya.
    for id in ids.iter().step_by(7) {
        journal::delete(&pool, *id).await.unwrap();
    }
    for id in ids.iter().skip(3).step_by(11) {
        let Ok(j) = journal::get(&pool, *id).await else { continue };
        let a = j.lines[0].account_id;
        let b = j.lines[1].account_id;
        let amount = 1 + next(900_000);
        let edit = JournalInput {
            date: j.entry.date.clone(),
            description: "edit".into(),
            reference: None,
            lines: vec![line(a, amount, 0), line(b, 0, amount)],
        };
        // Jurnal asli bisa saja kredit di baris pertama; susun ulang sesuai arah aslinya.
        let edit = if j.lines[0].credit > 0 {
            JournalInput { lines: vec![line(a, 0, amount), line(b, amount, 0)], ..edit }
        } else {
            edit
        };
        journal::update(&pool, *id, edit, false).await.unwrap();
    }

    for include_zero in [false, true] {
        let tb = reports::trial_balance(&pool, tb_q(None, include_zero)).await.unwrap();
        assert!(tb.balanced, "neraca saldo tidak seimbang: selisih {}", tb.difference);
        let (d, c): (i64, i64) = tb.rows.iter().fold((0, 0), |(d, c), r| (d + r.total_debit, c + r.total_credit));
        assert_eq!(d, c, "total mutasi debit harus sama dengan kredit");
    }
}

#[tokio::test]
async fn neraca_saldo_dengan_saldo_awal_tetap_seimbang_dan_cocok_dengan_buku_besar() {
    let pool = setup().await;
    let (kas, bank, utang) = (acc(&pool, "1-1100").await, acc(&pool, "1-1200").await, acc(&pool, "2-1100").await);
    opening::save(
        &pool,
        OpeningInput {
            date: "2026-01-01".into(),
            lines: vec![line(kas, 5_000_000, 0), line(bank, 15_000_000, 0), line(utang, 0, 8_000_000)],
        },
    )
    .await
    .unwrap();
    post(&pool, "2026-11-01", "1-1100", "4-1000", 250_000).await;
    post(&pool, "2026-11-02", "2-1100", "1-1200", 1_000_000).await;

    let tb = reports::trial_balance(&pool, tb_q(Some("2026-11-30"), false)).await.unwrap();
    assert!(tb.balanced);
    let modal = tb.rows.iter().find(|r| r.code == "3-1000").unwrap();
    assert_eq!(modal.balance_credit, 12_000_000);

    // Setiap akun: saldo neraca saldo = saldo akhir buku besar (dalam arah saldo normalnya).
    for r in &tb.rows {
        let l = reports::ledger(&pool, ledger_q(r.account_id, None, Some("2026-11-30"))).await.unwrap();
        let from_tb = if l.account.normal_balance == "DEBIT" {
            r.balance_debit - r.balance_credit
        } else {
            r.balance_credit - r.balance_debit
        };
        assert_eq!(l.closing_balance, from_tb, "akun {} tidak cocok", r.code);
    }
}

#[tokio::test]
async fn neraca_saldo_opsi_akun_nol_dan_input_tidak_valid() {
    let pool = setup().await;
    post(&pool, "2026-11-01", "1-1100", "4-1000", 100).await;
    post(&pool, "2026-11-02", "4-1000", "1-1100", 100).await; // saldo kembali nol

    let hide = reports::trial_balance(&pool, tb_q(None, false)).await.unwrap();
    assert!(hide.rows.is_empty());
    let show = reports::trial_balance(&pool, tb_q(None, true)).await.unwrap();
    assert_eq!(show.rows.len(), 40); // 50 akun template - 10 header
    assert!(show.rows.iter().all(|r| r.balance_debit == 0 && r.balance_credit == 0 || r.code == "1-1100" || r.code == "4-1000"));
    assert!(show.balanced);

    assert!(reports::trial_balance(&pool, tb_q(Some("31/12/2026"), false)).await.is_err());

    // Akun nonaktif tanpa mutasi tidak ditampilkan walau include_zero.
    let piutang = acc(&pool, "1-1300").await;
    accounts::set_active(&pool, piutang, false).await.unwrap();
    let show = reports::trial_balance(&pool, tb_q(None, true)).await.unwrap();
    assert!(show.rows.iter().all(|r| r.code != "1-1300"));
}
