use sqlx::SqlitePool;

use super::accounts::{self, Account, AccountInput};
use super::company::{self, SetupInput};
use crate::db;
use crate::error::AppError;

async fn setup_pool(template: &str) -> SqlitePool {
    let pool = db::test_pool().await;
    company::setup(
        &pool,
        SetupInput { name: "PT Contoh".into(), fiscal_year_start_month: 1, template: template.into() },
    )
    .await
    .unwrap();
    pool
}

async fn by_code(pool: &SqlitePool, code: &str) -> Account {
    accounts::list(pool).await.unwrap().into_iter().find(|a| a.code == code).unwrap()
}

fn input(code: &str, name: &str, t: &str, parent: Option<i64>, header: bool) -> AccountInput {
    AccountInput {
        code: code.into(),
        name: name.into(),
        account_type: t.into(),
        normal_balance: None,
        parent_id: parent,
        is_header: header,
        is_cash: false,
    }
}

fn msg(e: AppError) -> String {
    match e {
        AppError::Validation(m) => m,
        other => panic!("bukan error validasi: {other:?}"),
    }
}

/// Buat satu jurnal dengan satu baris untuk akun tertentu (menandai akun "sudah dipakai").
async fn use_account(pool: &SqlitePool, account_id: i64) {
    sqlx::query("INSERT INTO journal_entries (number, date, description) VALUES ('JU-T', '2026-01-01', 't')")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO journal_lines (entry_id, account_id, debit, credit) VALUES (1, ?, 100, 0)")
        .bind(account_id)
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn setup_standard_mengisi_coa_dan_akun_sistem() {
    let pool = setup_pool("standard").await;
    let all = accounts::list(&pool).await.unwrap();
    assert!(all.len() > 40);

    let modal = by_code(&pool, "3-1000").await;
    assert_eq!(modal.system_role.as_deref(), Some("OPENING_EQUITY"));
    assert!(modal.is_system);
    let laba = by_code(&pool, "3-3000").await;
    assert_eq!(laba.system_role.as_deref(), Some("RETAINED_EARNINGS"));

    // Akun kontra punya saldo normal berlawanan.
    assert_eq!(by_code(&pool, "1-2210").await.normal_balance, "CREDIT");
    assert_eq!(by_code(&pool, "3-2000").await.normal_balance, "DEBIT");
    assert_eq!(by_code(&pool, "1-1100").await.normal_balance, "DEBIT");
    assert_eq!(by_code(&pool, "4-1000").await.normal_balance, "CREDIT");

    // Struktur: induk harus header dan berjenis sama.
    for a in &all {
        if let Some(pid) = a.parent_id {
            let p = all.iter().find(|x| x.id == pid).unwrap();
            assert!(p.is_header, "{} punya induk bukan header", a.code);
            assert_eq!(p.account_type, a.account_type, "{} beda jenis dengan induk", a.code);
        }
    }
}

#[tokio::test]
async fn setup_empty_hanya_akun_sistem_dan_tidak_bisa_dua_kali() {
    let pool = setup_pool("empty").await;
    assert_eq!(accounts::list(&pool).await.unwrap().len(), 3);

    let again = company::setup(
        &pool,
        SetupInput { name: "X".into(), fiscal_year_start_month: 1, template: "standard".into() },
    )
    .await;
    assert!(msg(again.unwrap_err()).contains("sudah disetup"));
}

#[tokio::test]
async fn setup_menolak_input_tidak_valid() {
    let pool = db::test_pool().await;
    for (name, month, template) in [("", 1, "standard"), ("A", 13, "standard"), ("A", 1, "ngawur")] {
        let r = company::setup(
            &pool,
            SetupInput { name: name.into(), fiscal_year_start_month: month, template: template.into() },
        )
        .await;
        assert!(matches!(r, Err(AppError::Validation(_))));
    }
    assert!(company::get(&pool).await.unwrap().is_none());
}

#[tokio::test]
async fn kode_akun_harus_unik() {
    let pool = setup_pool("standard").await;
    let err = accounts::create(&pool, input("1-1100", "Kas Kecil", "ASSET", None, false)).await.unwrap_err();
    assert!(msg(err).contains("sudah dipakai"));
}

#[tokio::test]
async fn induk_harus_header_dan_sejenis() {
    let pool = setup_pool("standard").await;
    let kas = by_code(&pool, "1-1100").await;
    let aset_lancar = by_code(&pool, "1-1000").await;

    let e = accounts::create(&pool, input("1-1101", "Kas Toko", "ASSET", Some(kas.id), false)).await.unwrap_err();
    assert!(msg(e).contains("header"));

    let e = accounts::create(&pool, input("1-1101", "Kas Toko", "EXPENSE", Some(aset_lancar.id), false)).await.unwrap_err();
    assert!(msg(e).contains("sama"));

    let ok = accounts::create(&pool, input("1-1101", "Kas Toko", "ASSET", Some(aset_lancar.id), false)).await.unwrap();
    assert_eq!(ok.normal_balance, "DEBIT");
}

#[tokio::test]
async fn input_dasar_divalidasi() {
    let pool = setup_pool("empty").await;
    assert!(accounts::create(&pool, input("  ", "X", "ASSET", None, false)).await.is_err());
    assert!(accounts::create(&pool, input("9-1", "  ", "ASSET", None, false)).await.is_err());
    assert!(accounts::create(&pool, input("9-1", "X", "BUKAN_JENIS", None, false)).await.is_err());
    let mut bad = input("9-1", "X", "ASSET", None, false);
    bad.normal_balance = Some("MIRING".into());
    assert!(accounts::create(&pool, bad).await.is_err());
}

#[tokio::test]
async fn siklus_induk_dicegah() {
    let pool = setup_pool("empty").await;
    let a = accounts::create(&pool, input("8-1", "A", "ASSET", None, true)).await.unwrap();
    let b = accounts::create(&pool, input("8-2", "B", "ASSET", Some(a.id), true)).await.unwrap();

    let e = accounts::update(&pool, a.id, input("8-1", "A", "ASSET", Some(b.id), true)).await.unwrap_err();
    assert!(msg(e).contains("turunan"));
    let e = accounts::update(&pool, a.id, input("8-1", "A", "ASSET", Some(a.id), true)).await.unwrap_err();
    assert!(msg(e).contains("dirinya sendiri"));
}

#[tokio::test]
async fn akun_terpakai_tidak_bisa_dihapus_hanya_dinonaktifkan() {
    let pool = setup_pool("standard").await;
    let kas = by_code(&pool, "1-1100").await;
    use_account(&pool, kas.id).await;

    let e = accounts::delete(&pool, kas.id).await.unwrap_err();
    assert!(msg(e).contains("Nonaktifkan"));

    let off = accounts::set_active(&pool, kas.id, false).await.unwrap();
    assert!(!off.is_active);
    assert!(off.in_use);
    let on = accounts::set_active(&pool, kas.id, true).await.unwrap();
    assert!(on.is_active);
}

#[tokio::test]
async fn akun_tidak_terpakai_bisa_dihapus() {
    let pool = setup_pool("standard").await;
    let piutang = by_code(&pool, "1-1300").await;
    accounts::delete(&pool, piutang.id).await.unwrap();
    assert!(accounts::list(&pool).await.unwrap().iter().all(|a| a.code != "1-1300"));
}

#[tokio::test]
async fn header_dengan_anak_tidak_bisa_dihapus_atau_dinonaktifkan_sebelum_anaknya() {
    let pool = setup_pool("standard").await;
    let lancar = by_code(&pool, "1-1000").await;

    assert!(msg(accounts::delete(&pool, lancar.id).await.unwrap_err()).contains("anak"));
    assert!(msg(accounts::set_active(&pool, lancar.id, false).await.unwrap_err()).contains("anaknya"));
}

#[tokio::test]
async fn akun_sistem_dilindungi() {
    let pool = setup_pool("standard").await;
    let modal = by_code(&pool, "3-1000").await;

    assert!(msg(accounts::delete(&pool, modal.id).await.unwrap_err()).contains("sistem"));
    assert!(msg(accounts::set_active(&pool, modal.id, false).await.unwrap_err()).contains("sistem"));
    // Jenis tidak boleh diubah.
    assert!(accounts::update(&pool, modal.id, input("3-1000", "Modal Pemilik", "LIABILITY", modal.parent_id, false)).await.is_err());
    // Ganti nama boleh.
    let renamed = accounts::update(&pool, modal.id, input("3-1000", "Modal Disetor", "EQUITY", modal.parent_id, false)).await.unwrap();
    assert_eq!(renamed.name, "Modal Disetor");
    assert_eq!(renamed.system_role.as_deref(), Some("OPENING_EQUITY"));
}

#[tokio::test]
async fn akun_terpakai_tidak_bisa_ganti_jenis_atau_jadi_header() {
    let pool = setup_pool("standard").await;
    let kas = by_code(&pool, "1-1100").await;
    use_account(&pool, kas.id).await;

    let e = accounts::update(&pool, kas.id, input("1-1100", "Kas", "ASSET", kas.parent_id, true)).await.unwrap_err();
    assert!(msg(e).contains("dipakai"));

    // Ganti nama tetap boleh.
    let ok = accounts::update(&pool, kas.id, input("1-1100", "Kas Besar", "ASSET", kas.parent_id, false)).await.unwrap();
    assert_eq!(ok.name, "Kas Besar");
}

#[tokio::test]
async fn aktifkan_anak_butuh_induk_aktif() {
    let pool = setup_pool("empty").await;
    let h = accounts::create(&pool, input("8-1", "H", "ASSET", None, true)).await.unwrap();
    let c = accounts::create(&pool, input("8-2", "C", "ASSET", Some(h.id), false)).await.unwrap();
    accounts::set_active(&pool, c.id, false).await.unwrap();
    accounts::set_active(&pool, h.id, false).await.unwrap();
    assert!(msg(accounts::set_active(&pool, c.id, true).await.unwrap_err()).contains("induk"));
    accounts::set_active(&pool, h.id, true).await.unwrap();
    accounts::set_active(&pool, c.id, true).await.unwrap();
}

#[tokio::test]
async fn setiap_perubahan_dicatat_di_audit_log() {
    let pool = setup_pool("empty").await;
    let a = accounts::create(&pool, input("8-1", "A", "ASSET", None, false)).await.unwrap();
    accounts::update(&pool, a.id, input("8-1", "A2", "ASSET", None, false)).await.unwrap();
    accounts::set_active(&pool, a.id, false).await.unwrap();
    accounts::delete(&pool, a.id).await.unwrap();

    let actions: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM audit_log WHERE entity = 'account' ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(actions, ["CREATE", "UPDATE", "DEACTIVATE", "DELETE"]);

    let detail: String = sqlx::query_scalar("SELECT detail FROM audit_log WHERE action = 'UPDATE'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(detail.contains("\"before\"") && detail.contains("A2"));
}

#[tokio::test]
async fn gagal_validasi_tidak_meninggalkan_jejak() {
    let pool = setup_pool("empty").await;
    let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_log").fetch_one(&pool).await.unwrap();
    let _ = accounts::create(&pool, input("3-1000", "Dobel", "EQUITY", None, false)).await;
    let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_log").fetch_one(&pool).await.unwrap();
    assert_eq!(before, after);
}
