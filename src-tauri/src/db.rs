use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::RwLock;

use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::error::AppResult;
use crate::services::backup;

/// Semua migrasi skema yang dikenal aplikasi ini (di-embed saat compile).
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// State yang dibagikan ke semua command. Pool bisa diganti saat restore backup, jadi disimpan
/// di dalam RwLock; ambil lewat `pool()` (murah: SqlitePool hanya pegangan ber-Arc).
pub struct AppState {
    pool: RwLock<SqlitePool>,
    pub db_path: PathBuf,
}

impl AppState {
    pub fn new(pool: SqlitePool, db_path: PathBuf) -> Self {
        Self { pool: RwLock::new(pool), db_path }
    }

    pub fn pool(&self) -> SqlitePool {
        self.pool.read().expect("lock pool").clone()
    }

    /// Ganti pool (setelah restore) dan kembalikan pool lama agar bisa ditutup.
    pub fn swap_pool(&self, new: SqlitePool) -> SqlitePool {
        std::mem::replace(&mut *self.pool.write().expect("lock pool"), new)
    }

    /// Folder backup otomatis: `<folder data>/backups`.
    pub fn backups_dir(&self) -> PathBuf {
        backups_dir_of(&self.db_path)
    }
}

pub fn backups_dir_of(db_path: &Path) -> PathBuf {
    db_path.parent().map_or_else(|| PathBuf::from("backups"), |p| p.join("backups"))
}

/// Buka (atau buat) database di `db_path`, aktifkan foreign key + WAL, backup bila ada migrasi
/// yang akan dijalankan pada database yang sudah berisi, lalu jalankan migrasi.
pub async fn init(db_path: &Path) -> AppResult<SqlitePool> {
    if let Some(dir) = db_path.parent() {
        std::fs::create_dir_all(dir)?;
    }

    let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    // Update aplikasi yang mengubah skema: simpan salinan database dulu. Bila backup gagal,
    // migrasi dibatalkan supaya data lama tidak berisiko.
    if let Some(path) = backup::before_migration(&pool, &backups_dir_of(db_path)).await? {
        tracing::info!("Backup sebelum migrasi: {}", path.display());
    }

    MIGRATOR.run(&pool).await?;
    Ok(pool)
}

/// Database in-memory berisi skema lengkap, untuk unit test.
#[cfg(test)]
pub async fn test_pool() -> SqlitePool {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")
        .unwrap()
        .foreign_keys(true);
    // Satu koneksi saja: setiap koneksi in-memory adalah database terpisah.
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    MIGRATOR.run(&pool).await.unwrap();
    pool
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migrasi_membuat_semua_tabel() {
        let dir = std::env::temp_dir().join(format!("akuntansi-test-{}", std::process::id()));
        let pool = init(&dir.join("test.db")).await.unwrap();

        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '\\_sqlx%' ESCAPE '\\' ORDER BY name",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        let names: Vec<&str> = tables.iter().map(|t| t.0.as_str()).collect();

        for expected in [
            "accounts", "audit_log", "company", "fiscal_periods", "fiscal_years",
            "journal_counters", "journal_entries", "journal_lines",
        ] {
            assert!(names.contains(&expected), "tabel {expected} tidak ada: {names:?}");
        }

        // Constraint: satu baris tidak boleh debit dan kredit sekaligus.
        sqlx::query("INSERT INTO accounts (code,name,type,normal_balance) VALUES ('1','Kas','ASSET','DEBIT')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO journal_entries (number,date,description) VALUES ('JU-1','2026-01-01','x')")
            .execute(&pool).await.unwrap();
        let bad = sqlx::query("INSERT INTO journal_lines (entry_id,account_id,debit,credit) VALUES (1,1,100,100)")
            .execute(&pool).await;
        assert!(bad.is_err(), "baris dengan debit dan kredit sekaligus harus ditolak");

        pool.close().await;
        let _ = std::fs::remove_dir_all(&dir);
    }
}
