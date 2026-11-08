use sqlx::SqlitePool;

use super::accounts;
use super::company::{self, SetupInput};
use super::journal::{self, JournalFilter, JournalInput, LineInput};
use super::opening::{self, OpeningInput};
use crate::db;
use crate::error::AppError;

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

fn input(date: &str, lines: Vec<LineInput>) -> JournalInput {
    JournalInput { date: date.into(), description: "Transaksi uji".into(), reference: None, lines }
}

/// Jurnal seimbang Kas (debit) - Penjualan (kredit).
async fn sale(pool: &SqlitePool, date: &str, amount: i64) -> JournalInput {
    input(date, vec![line(acc(pool, "1-1100").await, amount, 0), line(acc(pool, "4-1000").await, 0, amount)])
}

fn msg(e: AppError) -> String {
    match e {
        AppError::Validation(m) => m,
        other => panic!("bukan error validasi: {other:?}"),
    }
}

async fn close_period(pool: &SqlitePool, year: i32, month: i32) {
    sqlx::query(
        "INSERT INTO fiscal_periods (year, month, is_closed) VALUES (?, ?, 1)
         ON CONFLICT(year, month) DO UPDATE SET is_closed = 1",
    )
    .bind(year)
    .bind(month)
    .execute(pool)
    .await
    .unwrap();
}

async fn open_period(pool: &SqlitePool, year: i32, month: i32) {
    sqlx::query("UPDATE fiscal_periods SET is_closed = 0 WHERE year = ? AND month = ?")
        .bind(year)
        .bind(month)
        .execute(pool)
        .await
        .unwrap();
}

async fn count_lines(pool: &SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM journal_lines").fetch_one(pool).await.unwrap()
}

// ---------- Draft & posting ----------

#[tokio::test]
async fn draft_boleh_belum_seimbang_tapi_posting_tidak() {
    let pool = setup().await;
    let kas = acc(&pool, "1-1100").await;

    let draft = journal::create(&pool, input("2026-11-05", vec![line(kas, 100_000, 0)]), false).await.unwrap();
    assert_eq!(draft.entry.status, "DRAFT");
    assert!(draft.entry.posted_at.is_none());

    let e = journal::post(&pool, draft.entry.id).await.unwrap_err();
    assert!(msg(e).contains("minimal terdiri dari 2 baris"));

    // Lengkapi lalu posting.
    let full = sale(&pool, "2026-11-05", 100_000).await;
    journal::update(&pool, draft.entry.id, full, false).await.unwrap();
    let posted = journal::post(&pool, draft.entry.id).await.unwrap();
    assert_eq!(posted.entry.status, "POSTED");
    assert!(posted.entry.posted_at.is_some());
    assert!(msg(journal::post(&pool, draft.entry.id).await.unwrap_err()).contains("sudah diposting"));
}

#[tokio::test]
async fn posting_menolak_tidak_seimbang() {
    let pool = setup().await;
    let (kas, jual) = (acc(&pool, "1-1100").await, acc(&pool, "4-1000").await);
    let e = journal::create(&pool, input("2026-11-05", vec![line(kas, 100_000, 0), line(jual, 0, 99_999)]), true)
        .await
        .unwrap_err();
    let m = msg(e);
    assert!(m.contains("tidak seimbang") && m.contains("1.000,00") && m.contains("0,01"), "{m}");
    // Tidak ada yang tersimpan.
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM journal_entries").fetch_one(&pool).await.unwrap();
    assert_eq!(n, 0);
}

#[tokio::test]
async fn create_langsung_posting() {
    let pool = setup().await;
    let j = journal::create(&pool, sale(&pool, "2026-11-05", 250_000).await, true).await.unwrap();
    assert_eq!(j.entry.status, "POSTED");
    assert_eq!(j.lines.len(), 2);
    assert_eq!(j.lines[0].account_code, "1-1100");
}

#[tokio::test]
async fn aturan_baris_dan_akun() {
    let pool = setup().await;
    let (kas, jual) = (acc(&pool, "1-1100").await, acc(&pool, "4-1000").await);
    let header = acc(&pool, "1-1000").await;

    // Debit dan kredit sekaligus / dua-duanya nol / negatif.
    for bad in [line(kas, 10, 10), line(kas, 0, 0), line(kas, -5, 0)] {
        assert!(journal::create(&pool, input("2026-11-05", vec![bad, line(jual, 0, 10)]), false).await.is_err());
    }
    // Akun header.
    let e = journal::create(&pool, input("2026-11-05", vec![line(header, 10, 0), line(jual, 0, 10)]), false)
        .await
        .unwrap_err();
    assert!(msg(e).contains("header"));
    // Akun tidak ada.
    assert!(journal::create(&pool, input("2026-11-05", vec![line(9999, 10, 0)]), false).await.is_err());
    // Akun nonaktif.
    accounts::set_active(&pool, kas, false).await.unwrap();
    let e = journal::create(&pool, input("2026-11-05", vec![line(kas, 10, 0)]), false).await.unwrap_err();
    assert!(msg(e).contains("nonaktif"));
    assert_eq!(count_lines(&pool).await, 0);
}

#[tokio::test]
async fn header_jurnal_divalidasi() {
    let pool = setup().await;
    let ok = sale(&pool, "2026-11-05", 10).await;

    let mut bad = ok.clone();
    bad.date = "05/11/2026".into();
    assert!(msg(journal::create(&pool, bad, false).await.unwrap_err()).contains("Tanggal"));

    let mut bad = ok.clone();
    bad.date = "2026-02-30".into();
    assert!(journal::create(&pool, bad, false).await.is_err());

    let mut bad = ok.clone();
    bad.description = "   ".into();
    assert!(msg(journal::create(&pool, bad, false).await.unwrap_err()).contains("Keterangan"));
}

// ---------- Edit & hapus jurnal POSTED ----------

#[tokio::test]
async fn edit_jurnal_posted_divalidasi_ulang_dan_atomik() {
    let pool = setup().await;
    let (kas, jual) = (acc(&pool, "1-1100").await, acc(&pool, "4-1000").await);
    let j = journal::create(&pool, sale(&pool, "2026-11-05", 100_000).await, true).await.unwrap();

    // Edit jadi tidak seimbang: ditolak, isi lama utuh.
    let bad = input("2026-11-05", vec![line(kas, 200_000, 0), line(jual, 0, 100_000)]);
    assert!(msg(journal::update(&pool, j.entry.id, bad, false).await.unwrap_err()).contains("tidak seimbang"));
    let still = journal::get(&pool, j.entry.id).await.unwrap();
    assert_eq!(still.lines.iter().map(|l| l.debit).sum::<i64>(), 100_000);
    assert_eq!(still.entry.status, "POSTED");

    // Edit yang seimbang: berhasil, tetap POSTED, nomor sama, updated_at terisi.
    let good = input("2026-11-06", vec![line(kas, 300_000, 0), line(jual, 0, 300_000)]);
    let after = journal::update(&pool, j.entry.id, good, false).await.unwrap();
    assert_eq!(after.entry.status, "POSTED");
    assert_eq!(after.entry.number, j.entry.number);
    assert_eq!(after.entry.date, "2026-11-06");
    assert!(after.entry.updated_at.is_some());
    assert_eq!(after.lines.iter().map(|l| l.debit).sum::<i64>(), 300_000);
    assert_eq!(count_lines(&pool).await, 2);
}

#[tokio::test]
async fn edit_posted_tidak_boleh_turun_ke_baris_kurang() {
    let pool = setup().await;
    let kas = acc(&pool, "1-1100").await;
    let j = journal::create(&pool, sale(&pool, "2026-11-05", 100).await, true).await.unwrap();
    let e = journal::update(&pool, j.entry.id, input("2026-11-05", vec![line(kas, 100, 0)]), false).await.unwrap_err();
    assert!(msg(e).contains("2 baris"));
}

#[tokio::test]
async fn hapus_jurnal_posted_boleh_dan_nomor_tidak_dipakai_ulang() {
    let pool = setup().await;
    let a = journal::create(&pool, sale(&pool, "2026-11-05", 100).await, true).await.unwrap();
    let b = journal::create(&pool, sale(&pool, "2026-11-06", 100).await, true).await.unwrap();
    assert_eq!(a.entry.number, "JU-2026-11-0001");
    assert_eq!(b.entry.number, "JU-2026-11-0002");

    journal::delete(&pool, b.entry.id).await.unwrap();
    assert!(journal::get(&pool, b.entry.id).await.is_err());
    assert_eq!(count_lines(&pool).await, 2); // baris jurnal b ikut terhapus

    let c = journal::create(&pool, sale(&pool, "2026-11-07", 100).await, true).await.unwrap();
    assert_eq!(c.entry.number, "JU-2026-11-0003", "nomor 0002 tidak boleh dipakai ulang");
}

#[tokio::test]
async fn nomor_berurutan_per_bulan() {
    let pool = setup().await;
    let n1 = journal::create(&pool, sale(&pool, "2026-11-05", 1).await, false).await.unwrap().entry.number;
    let n2 = journal::create(&pool, sale(&pool, "2026-12-01", 1).await, false).await.unwrap().entry.number;
    let n3 = journal::create(&pool, sale(&pool, "2026-11-30", 1).await, false).await.unwrap().entry.number;
    assert_eq!(n1, "JU-2026-11-0001");
    assert_eq!(n2, "JU-2026-12-0001");
    assert_eq!(n3, "JU-2026-11-0002");
}

// ---------- Periode tertutup ----------

#[tokio::test]
async fn periode_tertutup_memblokir_semua_perubahan() {
    let pool = setup().await;
    let j = journal::create(&pool, sale(&pool, "2026-10-15", 100).await, true).await.unwrap();
    let draft = journal::create(&pool, sale(&pool, "2026-10-20", 100).await, false).await.unwrap();
    close_period(&pool, 2026, 10).await;

    // Buat baru di periode tertutup.
    assert!(msg(journal::create(&pool, sale(&pool, "2026-10-25", 1).await, false).await.unwrap_err()).contains("ditutup"));
    // Edit, hapus, posting jurnal yang berada di periode tertutup.
    assert!(msg(journal::update(&pool, j.entry.id, sale(&pool, "2026-10-15", 5).await, false).await.unwrap_err()).contains("ditutup"));
    assert!(msg(journal::delete(&pool, j.entry.id).await.unwrap_err()).contains("ditutup"));
    assert!(msg(journal::post(&pool, draft.entry.id).await.unwrap_err()).contains("ditutup"));
    // Memindahkan jurnal dari periode terbuka ke periode tertutup.
    let open = journal::create(&pool, sale(&pool, "2026-11-02", 1).await, true).await.unwrap();
    assert!(msg(journal::update(&pool, open.entry.id, sale(&pool, "2026-10-31", 1).await, false).await.unwrap_err()).contains("ditutup"));

    // Setelah dibuka kembali, semuanya boleh lagi.
    open_period(&pool, 2026, 10).await;
    journal::update(&pool, j.entry.id, sale(&pool, "2026-10-15", 5).await, false).await.unwrap();
    journal::delete(&pool, j.entry.id).await.unwrap();
}

// ---------- Audit log ----------

#[tokio::test]
async fn riwayat_mencatat_isi_sebelum_dan_sesudah() {
    let pool = setup().await;
    let j = journal::create(&pool, sale(&pool, "2026-11-05", 100).await, true).await.unwrap();
    journal::update(&pool, j.entry.id, sale(&pool, "2026-11-05", 700).await, false).await.unwrap();
    journal::delete(&pool, j.entry.id).await.unwrap();

    let h = journal::history(&pool, j.entry.id).await.unwrap();
    let actions: Vec<&str> = h.iter().map(|x| x.action.as_str()).collect();
    assert_eq!(actions, ["CREATE", "POST", "UPDATE", "DELETE"]);

    let update = &h[2].detail;
    assert_eq!(update["before"]["lines"][0]["debit"], 100);
    assert_eq!(update["after"]["lines"][0]["debit"], 700);
    assert_eq!(h[3].detail["before"]["number"], "JU-2026-11-0001");
}

#[tokio::test]
async fn simpan_tanpa_perubahan_tidak_menambah_noise_di_riwayat() {
    let pool = setup().await;
    let same = sale(&pool, "2026-11-05", 100).await;
    let draft = journal::create(&pool, same.clone(), false).await.unwrap();

    // Posting draft tanpa mengubah isi: hanya CREATE lalu POST.
    journal::update(&pool, draft.entry.id, same.clone(), true).await.unwrap();
    // Simpan ulang jurnal POSTED tanpa perubahan: tidak ada entri baru.
    journal::update(&pool, draft.entry.id, same.clone(), false).await.unwrap();
    let actions: Vec<String> = journal::history(&pool, draft.entry.id).await.unwrap().into_iter().map(|h| h.action).collect();
    assert_eq!(actions, ["CREATE", "POST"]);

    // Perubahan sekecil apa pun tetap tercatat (mis. hanya catatan baris).
    let mut changed = same.clone();
    changed.lines[0].memo = Some("catatan".into());
    journal::update(&pool, draft.entry.id, changed, false).await.unwrap();
    let h = journal::history(&pool, draft.entry.id).await.unwrap();
    assert_eq!(h.last().unwrap().action, "UPDATE");
}

#[tokio::test]
async fn gagal_validasi_tidak_meninggalkan_jejak_atau_menaikkan_nomor() {
    let pool = setup().await;
    let (kas, jual) = (acc(&pool, "1-1100").await, acc(&pool, "4-1000").await);
    let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_log").fetch_one(&pool).await.unwrap();
    let _ = journal::create(&pool, input("2026-11-05", vec![line(kas, 5, 0), line(jual, 0, 4)]), true).await;
    let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_log").fetch_one(&pool).await.unwrap();
    assert_eq!(before, after);
    // Transaksi gagal di-rollback, jadi nomor pertama tetap 0001.
    let ok = journal::create(&pool, sale(&pool, "2026-11-05", 1).await, true).await.unwrap();
    assert_eq!(ok.entry.number, "JU-2026-11-0001");
}

// ---------- Daftar ----------

#[tokio::test]
async fn daftar_filter_cari_dan_pagination() {
    let pool = setup().await;
    for day in 1..=7 {
        let mut i = sale(&pool, &format!("2026-11-{day:02}"), day * 1000).await;
        i.description = format!("Penjualan hari ke-{day}");
        if day == 3 {
            i.reference = Some("INV-777".into());
        }
        journal::create(&pool, i, day % 2 == 0).await.unwrap();
    }
    let f = |f: JournalFilter| JournalFilter { page: 1, page_size: 25, ..f };

    let all = journal::list(&pool, f(JournalFilter::default())).await.unwrap();
    assert_eq!(all.total, 7);
    assert_eq!(all.items[0].date, "2026-11-07"); // terbaru dulu
    assert_eq!(all.items[0].total, 7000);
    assert_eq!(all.items[0].line_count, 2);

    let posted = journal::list(&pool, f(JournalFilter { status: Some("POSTED".into()), ..Default::default() })).await.unwrap();
    assert_eq!(posted.total, 3);

    let range = journal::list(&pool, f(JournalFilter { from: Some("2026-11-02".into()), to: Some("2026-11-04".into()), ..Default::default() })).await.unwrap();
    assert_eq!(range.total, 3);

    let by_ref = journal::list(&pool, f(JournalFilter { search: Some("inv-777".into()), ..Default::default() })).await.unwrap();
    assert_eq!(by_ref.total, 1);
    let by_no = journal::list(&pool, f(JournalFilter { search: Some("0005".into()), ..Default::default() })).await.unwrap();
    assert_eq!(by_no.total, 1);

    let p2 = journal::list(&pool, JournalFilter { page: 2, page_size: 3, ..Default::default() }).await.unwrap();
    assert_eq!((p2.total, p2.items.len(), p2.page), (7, 3, 2));
    assert_eq!(p2.items[0].date, "2026-11-04");
    let p3 = journal::list(&pool, JournalFilter { page: 3, page_size: 3, ..Default::default() }).await.unwrap();
    assert_eq!(p3.items.len(), 1);
}

// ---------- Saldo awal ----------

#[tokio::test]
async fn saldo_awal_menyeimbangkan_ke_modal_dan_bisa_disimpan_ulang() {
    let pool = setup().await;
    let (kas, bank, utang) = (acc(&pool, "1-1100").await, acc(&pool, "1-1200").await, acc(&pool, "2-1100").await);
    let modal = acc(&pool, "3-1000").await;

    let o = opening::save(
        &pool,
        OpeningInput {
            date: "2026-01-01".into(),
            lines: vec![line(kas, 500_000_000, 0), line(bank, 1_500_000_000, 0), line(utang, 0, 800_000_000)],
        },
    )
    .await
    .unwrap();
    assert_eq!(o.entry.entry_type, "OPENING");
    assert_eq!(o.entry.status, "POSTED");
    assert_eq!(o.entry.number, "SA-2026-01-0001");
    let eq = o.lines.iter().find(|l| l.account_id == modal).unwrap();
    assert_eq!((eq.debit, eq.credit), (0, 1_200_000_000)); // 20jt aset - 8jt utang
    assert_eq!(o.lines.iter().map(|l| l.debit).sum::<i64>(), o.lines.iter().map(|l| l.credit).sum::<i64>());

    // Simpan ulang: menggantikan, tetap satu jurnal, nomor sama.
    let again = opening::save(&pool, OpeningInput { date: "2026-01-01".into(), lines: vec![line(kas, 100_000, 0)] })
        .await
        .unwrap();
    assert_eq!(again.entry.id, o.entry.id);
    assert_eq!(again.entry.number, o.entry.number);
    assert_eq!(again.lines.len(), 2);
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM journal_entries WHERE type = 'OPENING'").fetch_one(&pool).await.unwrap();
    assert_eq!(n, 1);
    assert_eq!(opening::get(&pool).await.unwrap().unwrap().entry.id, o.entry.id);

    opening::delete(&pool).await.unwrap();
    assert!(opening::get(&pool).await.unwrap().is_none());
}

#[tokio::test]
async fn saldo_awal_menolak_akun_non_neraca_dan_akun_penyeimbang() {
    let pool = setup().await;
    let (kas, jual, modal) = (acc(&pool, "1-1100").await, acc(&pool, "4-1000").await, acc(&pool, "3-1000").await);

    let e = opening::save(&pool, OpeningInput { date: "2026-01-01".into(), lines: vec![line(jual, 0, 10)] }).await.unwrap_err();
    assert!(msg(e).contains("akun neraca"));
    let e = opening::save(&pool, OpeningInput { date: "2026-01-01".into(), lines: vec![line(modal, 0, 10)] }).await.unwrap_err();
    assert!(msg(e).contains("otomatis"));
    let e = opening::save(&pool, OpeningInput { date: "2026-01-01".into(), lines: vec![] }).await.unwrap_err();
    assert!(msg(e).contains("minimal satu"));
    assert!(opening::save(&pool, OpeningInput { date: "x".into(), lines: vec![line(kas, 1, 0)] }).await.is_err());
}

#[tokio::test]
async fn jurnal_saldo_awal_tidak_bisa_diubah_lewat_jurnal_umum() {
    let pool = setup().await;
    let kas = acc(&pool, "1-1100").await;
    let o = opening::save(&pool, OpeningInput { date: "2026-01-01".into(), lines: vec![line(kas, 100, 0)] }).await.unwrap();

    let e = journal::delete(&pool, o.entry.id).await.unwrap_err();
    assert!(msg(e).contains("Saldo Awal"));
    let e = journal::update(&pool, o.entry.id, sale(&pool, "2026-01-01", 1).await, false).await.unwrap_err();
    assert!(msg(e).contains("Saldo Awal"));
}

#[tokio::test]
async fn akun_di_jurnal_menjadi_in_use_dan_tidak_bisa_dihapus() {
    let pool = setup().await;
    journal::create(&pool, sale(&pool, "2026-11-05", 100).await, true).await.unwrap();
    let kas = acc(&pool, "1-1100").await;
    assert!(accounts::list(&pool).await.unwrap().iter().find(|a| a.id == kas).unwrap().in_use);
    assert!(accounts::delete(&pool, kas).await.is_err());
}

#[test]
fn format_rupiah_benar() {
    assert_eq!(journal::format_rupiah(0), "0,00");
    assert_eq!(journal::format_rupiah(5), "0,05");
    assert_eq!(journal::format_rupiah(125_000_050), "1.250.000,50");
    assert_eq!(journal::format_rupiah(100_000), "1.000,00");
}
