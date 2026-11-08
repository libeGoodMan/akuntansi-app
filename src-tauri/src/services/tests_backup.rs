use std::path::Path;

use sqlx::SqlitePool;
use tempfile::TempDir;

use super::accounts;
use super::backup::{self, Reason};
use super::company::{self, SetupInput};
use super::journal::{self, JournalInput, LineInput};
use crate::db::{self, AppState};
use crate::error::AppError;

fn msg(e: AppError) -> String {
    match e {
        AppError::Validation(m) | AppError::Other(m) => m,
        other => panic!("error tak terduga: {other:?}"),
    }
}

/// Database berkas sungguhan (bukan in-memory) dengan perusahaan terdaftar.
async fn file_state() -> (TempDir, AppState) {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("data.db");
    let pool = db::init(&db_path).await.unwrap();
    company::setup(
        &pool,
        SetupInput { name: "PT Backup".into(), fiscal_year_start_month: 1, template: "standard".into() },
    )
    .await
    .unwrap();
    (dir, AppState::new(pool, db_path))
}

async fn add_journal(pool: &SqlitePool, desc: &str) {
    let acc = |code: &'static str| async move {
        accounts::list(pool).await.unwrap().into_iter().find(|a| a.code == code).unwrap().id
    };
    let lines = vec![
        LineInput { account_id: acc("1-1100").await, debit: 1000, credit: 0, memo: None },
        LineInput { account_id: acc("4-1000").await, debit: 0, credit: 1000, memo: None },
    ];
    journal::create(
        pool,
        JournalInput { date: "2026-11-05".into(), description: desc.into(), reference: None, lines },
        true,
    )
    .await
    .unwrap();
}

async fn journal_count(pool: &SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM journal_entries").fetch_one(pool).await.unwrap()
}

fn touch(dir: &Path, name: &str) {
    std::fs::write(dir.join(name), b"x").unwrap();
}

// ---------- Backup ----------

#[tokio::test]
async fn backup_menghasilkan_salinan_valid_dengan_data_yang_sama() {
    let (dir, state) = file_state().await;
    add_journal(&state.pool(), "satu").await;
    let target = dir.path().join("salinan.db");

    backup::backup_to(&state.pool(), &target).await.unwrap();
    let check = backup::check_file(&target).await.unwrap();
    assert_eq!(check.company_name.as_deref(), Some("PT Backup"));
    assert_eq!(check.journals, 1);
    assert_eq!(check.accounts, 50);
    assert_eq!(check.schema_version, db::MIGRATOR.iter().map(|m| m.version).max().unwrap());

    // Menimpa berkas lama di lokasi yang sama.
    add_journal(&state.pool(), "dua").await;
    backup::backup_to(&state.pool(), &target).await.unwrap();
    assert_eq!(backup::check_file(&target).await.unwrap().journals, 2);

    assert!(msg(backup::backup_to(&state.pool(), &dir.path().join("x.txt")).await.unwrap_err()).contains(".db"));
    assert!(backup::backup_to(&state.pool(), &dir.path().join("tidak-ada/x.db")).await.is_err());
}

#[tokio::test]
async fn backup_otomatis_dinamai_dan_dirotasi_per_jenis() {
    let (_dir, state) = file_state().await;
    let backups = state.backups_dir();

    let a = backup::create(&state.pool(), &backups, Reason::Manual).await.unwrap();
    let b = backup::create(&state.pool(), &backups, Reason::Manual).await.unwrap();
    assert_ne!(a, b, "dua backup pada detik yang sama tidak boleh bentrok");
    let list = backup::list(&backups);
    assert_eq!(list.len(), 2);
    assert!(list.iter().all(|x| x.reason == "manual" && x.size > 4096));

    // Rotasi: simpan 3 terbaru dari jenis "keluar"; jenis lain dan berkas asing tidak disentuh.
    let dir = TempDir::new().unwrap();
    for i in 1..=6 {
        touch(dir.path(), &format!("akuntansi-20260101-00000{i}-keluar.db"));
    }
    touch(dir.path(), "akuntansi-20260101-000001-pra-migrasi.db");
    touch(dir.path(), "akuntansi-20260101-000001-manual.db");
    touch(dir.path(), "catatan-saya.txt");
    backup::rotate(dir.path(), Reason::Exit, 3);
    let names: Vec<String> = backup::list(dir.path()).into_iter().map(|b| b.file_name).collect();
    assert_eq!(names.iter().filter(|n| n.ends_with("keluar.db")).count(), 3);
    assert!(names.contains(&"akuntansi-20260101-000006-keluar.db".to_string()));
    assert!(!names.contains(&"akuntansi-20260101-000001-keluar.db".to_string()));
    assert!(names.contains(&"akuntansi-20260101-000001-pra-migrasi.db".to_string()));
    assert!(names.contains(&"akuntansi-20260101-000001-manual.db".to_string()));
    assert!(dir.path().join("catatan-saya.txt").exists());

    // Backup manual tidak pernah dirotasi.
    assert_eq!(Reason::Manual.keep(), None);
}

#[tokio::test]
async fn daftar_backup_mengurai_nama_dan_menghapus_dengan_aman() {
    let dir = TempDir::new().unwrap();
    touch(dir.path(), "akuntansi-20261108-014530-keluar.db");
    touch(dir.path(), "akuntansi-20261108-014531-pra-restore-2.db"); // akhiran nama bentrok
    touch(dir.path(), "akuntansi-20261108-014532-pra-update.db");
    touch(dir.path(), "akuntansi-bukan-tanggal-keluar.db");
    touch(dir.path(), "random.db");

    let list = backup::list(dir.path());
    assert_eq!(list.len(), 3);
    assert_eq!((list[0].reason.as_str(), list[0].created_at.as_str()), ("pra-update", "2026-11-08 01:45:32"));
    assert_eq!((list[1].reason.as_str(), list[1].created_at.as_str()), ("pra-restore", "2026-11-08 01:45:31"));
    assert_eq!((list[2].reason.as_str(), list[2].created_at.as_str()), ("keluar", "2026-11-08 01:45:30"));

    assert!(backup::delete(dir.path(), "../data.db").is_err());
    assert!(backup::delete(dir.path(), "random.db").is_err(), "hanya berkas backup yang sah");
    assert!(backup::delete(dir.path(), "akuntansi-20260101-000001-manual.db").is_err(), "tidak ada");
    backup::delete(dir.path(), "akuntansi-20261108-014530-keluar.db").unwrap();
    assert_eq!(backup::list(dir.path()).len(), 2);
    assert!(dir.path().join("random.db").exists());
}

#[tokio::test]
async fn backup_saat_keluar_dilewati_bila_tidak_ada_perubahan() {
    let dir = TempDir::new().unwrap();
    let pool = db::init(&dir.path().join("data.db")).await.unwrap();
    let backups = dir.path().join("backups");

    // Database kosong (belum ada aktivitas): tidak perlu backup.
    assert!(backup::backup_on_exit(&pool, &backups).await.unwrap().is_none());

    company::setup(&pool, SetupInput { name: "X".into(), fiscal_year_start_month: 1, template: "empty".into() })
        .await
        .unwrap();
    let first = backup::backup_on_exit(&pool, &backups).await.unwrap();
    assert!(first.is_some());
    // Tidak ada perubahan sejak backup tadi.
    assert!(backup::backup_on_exit(&pool, &backups).await.unwrap().is_none());
    assert_eq!(backup::list(&backups).len(), 1);

    // Setelah ada perubahan data, backup dibuat lagi.
    company::update(&pool, company::UpdateInput { name: "Y".into(), address: None }).await.unwrap();
    assert!(backup::backup_on_exit(&pool, &backups).await.unwrap().is_some());
    assert_eq!(backup::list(&backups).iter().filter(|b| b.reason == "keluar").count(), 2);
}

// ---------- Backup sebelum migrasi ----------

#[tokio::test]
async fn backup_pra_migrasi_hanya_bila_ada_migrasi_tertunda_pada_database_berisi() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("data.db");
    let backups = db::backups_dir_of(&db_path);

    // Database baru: tidak ada yang perlu dibackup.
    let pool = db::init(&db_path).await.unwrap();
    assert!(backup::list(&backups).is_empty());

    // Database mutakhir: init berikutnya tidak membuat backup.
    pool.close().await;
    let pool = db::init(&db_path).await.unwrap();
    assert!(backup::list(&backups).is_empty());

    // Simulasi database dari versi lama: migrasi terakhir belum tercatat.
    let last = db::MIGRATOR.iter().map(|m| m.version).max().unwrap();
    sqlx::query("DROP TABLE fiscal_years").execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version = ?").bind(last).execute(&pool).await.unwrap();
    company::setup(&pool, SetupInput { name: "Lama".into(), fiscal_year_start_month: 1, template: "empty".into() })
        .await
        .unwrap();
    pool.close().await;

    let pool = db::init(&db_path).await.unwrap();
    let list = backup::list(&backups);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].reason, "pra-migrasi");
    // Isi backup = kondisi sebelum migrasi (tanpa tabel baru), tetapi datanya lengkap.
    let check = backup::check_file(Path::new(&list[0].path)).await.unwrap();
    assert_eq!(check.company_name.as_deref(), Some("Lama"));
    assert_eq!(check.schema_version, last - 1);
    // Migrasi berhasil setelahnya.
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM fiscal_years").fetch_one(&pool).await.unwrap();
    assert_eq!(n, 0);
}

// ---------- Restore ----------

#[tokio::test]
async fn restore_mengembalikan_data_dan_membuat_backup_pengaman() {
    let (dir, state) = file_state().await;
    add_journal(&state.pool(), "sebelum").await;
    let snapshot = dir.path().join("snapshot.db");
    backup::backup_to(&state.pool(), &snapshot).await.unwrap();

    add_journal(&state.pool(), "sesudah-1").await;
    add_journal(&state.pool(), "sesudah-2").await;
    assert_eq!(journal_count(&state.pool()).await, 3);

    let check = backup::restore(&state, &snapshot).await.unwrap();
    assert_eq!(check.journals, 1);
    assert_eq!(journal_count(&state.pool()).await, 1, "data kembali ke kondisi snapshot");

    // Backup pengaman berisi kondisi SEBELUM restore (3 jurnal) sehingga restore bisa dibatalkan.
    let safety = backup::list(&state.backups_dir()).into_iter().find(|b| b.reason == "pra-restore").unwrap();
    assert_eq!(backup::check_file(Path::new(&safety.path)).await.unwrap().journals, 3);

    // Aplikasi tetap berfungsi dan bisa menulis setelah pool diganti.
    add_journal(&state.pool(), "setelah restore").await;
    assert_eq!(journal_count(&state.pool()).await, 2);

    // Dan restore dari backup pengaman memulihkan 3 jurnal tadi.
    backup::restore(&state, Path::new(&safety.path)).await.unwrap();
    assert_eq!(journal_count(&state.pool()).await, 3);
}

#[tokio::test]
async fn restore_menolak_berkas_tidak_sah_dan_data_tetap_utuh() {
    let (dir, state) = file_state().await;
    add_journal(&state.pool(), "aman").await;
    let before = journal_count(&state.pool()).await;

    // Bukan SQLite.
    let junk = dir.path().join("sampah.db");
    std::fs::write(&junk, vec![b'a'; 10_000]).unwrap();
    assert!(msg(backup::restore(&state, &junk).await.unwrap_err()).contains("bukan database SQLite"));

    // Terlalu kecil / kosong / tidak ada.
    let empty = dir.path().join("kosong.db");
    std::fs::write(&empty, b"").unwrap();
    assert!(backup::restore(&state, &empty).await.is_err());
    assert!(backup::restore(&state, &dir.path().join("tidak-ada.db")).await.is_err());

    // SQLite sah, tetapi bukan backup aplikasi ini.
    let other = dir.path().join("lain.db");
    {
        let opts = sqlx::sqlite::SqliteConnectOptions::new().filename(&other).create_if_missing(true);
        let p = sqlx::SqlitePool::connect_with(opts).await.unwrap();
        sqlx::query("CREATE TABLE barang (id INTEGER PRIMARY KEY, nama TEXT)").execute(&p).await.unwrap();
        for i in 0..2000 {
            sqlx::query("INSERT INTO barang (nama) VALUES (?)").bind(format!("barang {i}")).execute(&p).await.unwrap();
        }
        p.close().await;
    }
    assert!(msg(backup::restore(&state, &other).await.unwrap_err()).contains("bukan backup aplikasi"));

    // Dibuat oleh versi aplikasi yang lebih baru.
    let newer = dir.path().join("baru.db");
    backup::backup_to(&state.pool(), &newer).await.unwrap();
    {
        let opts = sqlx::sqlite::SqliteConnectOptions::new().filename(&newer);
        let p = sqlx::SqlitePool::connect_with(opts).await.unwrap();
        sqlx::query("INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) VALUES (9999, 'masa depan', 1, x'00', 0)")
            .execute(&p)
            .await
            .unwrap();
        p.close().await;
    }
    assert!(msg(backup::restore(&state, &newer).await.unwrap_err()).contains("versi aplikasi yang lebih baru"));

    // Berkas yang sedang dipakai aplikasi.
    assert!(msg(backup::restore(&state, &state.db_path).await.unwrap_err()).contains("sedang dipakai"));

    // Tidak ada yang berubah dan tidak ada backup pengaman yang dibuat untuk percobaan gagal.
    assert_eq!(journal_count(&state.pool()).await, before);
    assert!(backup::list(&state.backups_dir()).iter().all(|b| b.reason != "pra-restore"));
}

#[tokio::test]
async fn restore_dari_backup_versi_lama_memigrasikannya() {
    let (dir, state) = file_state().await;
    add_journal(&state.pool(), "data lama").await;
    let old = dir.path().join("lama.db");
    backup::backup_to(&state.pool(), &old).await.unwrap();

    // Jadikan salinan itu "versi lama": tanpa migrasi terakhir.
    let last = db::MIGRATOR.iter().map(|m| m.version).max().unwrap();
    {
        let opts = sqlx::sqlite::SqliteConnectOptions::new().filename(&old);
        let p = sqlx::SqlitePool::connect_with(opts).await.unwrap();
        sqlx::query("DROP TABLE fiscal_years").execute(&p).await.unwrap();
        sqlx::query("DELETE FROM _sqlx_migrations WHERE version = ?").bind(last).execute(&p).await.unwrap();
        p.close().await;
    }
    assert_eq!(backup::check_file(&old).await.unwrap().schema_version, last - 1);

    backup::restore(&state, &old).await.unwrap();
    assert_eq!(journal_count(&state.pool()).await, 1);
    let v: i64 = sqlx::query_scalar("SELECT MAX(version) FROM _sqlx_migrations").fetch_one(&state.pool()).await.unwrap();
    assert_eq!(v, last, "skema dinaikkan ke versi terbaru setelah restore");
    // Backup pra-migrasi ikut dibuat karena restore membuka database lama.
    assert!(backup::list(&state.backups_dir()).iter().any(|b| b.reason == "pra-migrasi"));
}
