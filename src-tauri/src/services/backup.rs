//! Backup dan restore database. Backup memakai `VACUUM INTO` (salinan konsisten walau database
//! sedang dipakai/WAL). Tiga jenis backup otomatis dirotasi per jenis; backup manual tidak dirotasi.

use std::path::{Path, PathBuf};
use std::str::FromStr;

use chrono::Local;
use serde::Serialize;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::db::{self, AppState, MIGRATOR};
use crate::error::{AppError, AppResult};

const PREFIX: &str = "akuntansi-";
const EXT: &str = ".db";

fn invalid<T>(msg: impl Into<String>) -> AppResult<T> {
    Err(AppError::Validation(msg.into()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// Otomatis saat aplikasi ditutup.
    Exit,
    /// Otomatis sebelum migrasi skema (update aplikasi).
    PreMigration,
    /// Otomatis sebelum restore menimpa database.
    PreRestore,
    /// Otomatis sebelum memasang pembaruan aplikasi.
    PreUpdate,
    /// Diminta pengguna ("Backup sekarang").
    Manual,
}

const ALL_REASONS: [Reason; 5] = [Reason::Exit, Reason::PreMigration, Reason::PreRestore, Reason::PreUpdate, Reason::Manual];

impl Reason {
    pub fn slug(self) -> &'static str {
        match self {
            Reason::Exit => "keluar",
            Reason::PreMigration => "pra-migrasi",
            Reason::PreRestore => "pra-restore",
            Reason::PreUpdate => "pra-update",
            Reason::Manual => "manual",
        }
    }

    /// Jumlah backup terbaru yang disimpan per jenis (None = tidak dirotasi).
    pub fn keep(self) -> Option<usize> {
        match self {
            Reason::Exit => Some(10),
            Reason::PreMigration | Reason::PreRestore | Reason::PreUpdate => Some(5),
            Reason::Manual => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub file_name: String,
    pub path: String,
    pub size: u64,
    /// Waktu pembuatan (lokal), "YYYY-MM-DD HH:MM:SS".
    pub created_at: String,
    /// keluar | pra-migrasi | pra-restore | pra-update | manual
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupCheck {
    pub schema_version: i64,
    pub company_name: Option<String>,
    pub accounts: i64,
    pub journals: i64,
}

/// "akuntansi-20261108-014530-keluar.db" -> ("2026-11-08 01:45:30", "keluar")
fn parse_name(name: &str) -> Option<(String, String)> {
    let stem = name.strip_prefix(PREFIX)?.strip_suffix(EXT)?;
    let mut parts = stem.splitn(3, '-');
    let (date, time, rest) = (parts.next()?, parts.next()?, parts.next()?);
    if date.len() != 8 || time.len() != 6 || !date.chars().chain(time.chars()).all(|c| c.is_ascii_digit()) {
        return None;
    }
    // Jenis dikenali dari daftar tetap; akhiran "-2", "-3" (nama bentrok) diabaikan.
    let reason = ALL_REASONS
        .iter()
        .map(|r| r.slug())
        .find(|slug| rest == *slug || rest.strip_prefix(slug).is_some_and(|t| t.starts_with('-')))?;
    let created = format!(
        "{}-{}-{} {}:{}:{}",
        &date[..4], &date[4..6], &date[6..], &time[..2], &time[2..4], &time[4..]
    );
    Some((created, reason.to_string()))
}

fn unique_path(dir: &Path, reason: Reason) -> PathBuf {
    let stamp = Local::now().format("%Y%m%d-%H%M%S");
    let base = format!("{PREFIX}{stamp}-{}", reason.slug());
    let mut path = dir.join(format!("{base}{EXT}"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{base}-{n}{EXT}"));
        n += 1;
    }
    path
}

/// Salin seluruh database ke `target` (harus belum ada) dengan `VACUUM INTO`.
async fn vacuum_into(pool: &SqlitePool, target: &Path) -> AppResult<()> {
    sqlx::query("VACUUM INTO ?")
        .bind(target.to_string_lossy().to_string())
        .execute(pool)
        .await?;
    Ok(())
}

/// Backup ke folder backup otomatis, lalu rotasi jenis itu.
pub async fn create(pool: &SqlitePool, dir: &Path, reason: Reason) -> AppResult<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = unique_path(dir, reason);
    vacuum_into(pool, &path).await?;
    if let Some(keep) = reason.keep() {
        rotate(dir, reason, keep);
    }
    Ok(path)
}

/// Backup ke lokasi pilihan pengguna; berkas lama di lokasi itu ditimpa.
pub async fn backup_to(pool: &SqlitePool, target: &Path) -> AppResult<()> {
    if target.extension().and_then(|e| e.to_str()).is_none_or(|e| !e.eq_ignore_ascii_case("db")) {
        return invalid("Berkas backup harus berekstensi .db");
    }
    if !target.parent().is_some_and(Path::is_dir) {
        return invalid("Folder tujuan tidak ditemukan");
    }
    if target.exists() {
        std::fs::remove_file(target)?;
    }
    vacuum_into(pool, target).await
}

/// Daftar backup di folder otomatis, terbaru lebih dulu.
pub fn list(dir: &Path) -> Vec<BackupInfo> {
    let Ok(entries) = std::fs::read_dir(dir) else { return vec![] };
    let mut out: Vec<BackupInfo> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let (created_at, reason) = parse_name(&name)?;
            let size = e.metadata().ok()?.len();
            Some(BackupInfo { path: e.path().to_string_lossy().to_string(), file_name: name, size, created_at, reason })
        })
        .collect();
    out.sort_by(|a, b| b.file_name.cmp(&a.file_name));
    out
}

/// Hapus backup terlama dari satu jenis; simpan `keep` yang terbaru.
pub fn rotate(dir: &Path, reason: Reason, keep: usize) {
    let mut mine: Vec<BackupInfo> = list(dir).into_iter().filter(|b| b.reason == reason.slug()).collect();
    mine.sort_by(|a, b| b.file_name.cmp(&a.file_name)); // terbaru dulu
    for old in mine.into_iter().skip(keep) {
        if let Err(e) = std::fs::remove_file(&old.path) {
            tracing::warn!("Gagal menghapus backup lama {}: {e}", old.path);
        }
    }
}

/// Hapus satu backup dari folder otomatis. Nama harus berupa berkas backup yang sah (tanpa jalur).
pub fn delete(dir: &Path, file_name: &str) -> AppResult<()> {
    if file_name.contains(['/', '\\']) || parse_name(file_name).is_none() {
        return invalid("Nama berkas backup tidak valid");
    }
    let path = dir.join(file_name);
    if !path.is_file() {
        return invalid("Berkas backup tidak ditemukan");
    }
    std::fs::remove_file(path)?;
    Ok(())
}

async fn open_readonly(path: &Path) -> AppResult<SqlitePool> {
    // immutable: tidak membuat berkas -wal/-shm di samping backup dan aman karena berkasnya tidak berubah.
    let options = SqliteConnectOptions::from_str("sqlite://")?.filename(path).read_only(true).immutable(true);
    Ok(SqlitePoolOptions::new().max_connections(1).connect_with(options).await?)
}

/// Periksa berkas backup tanpa mengubahnya: format SQLite, integritas, tabel inti, dan versi skema.
pub async fn check_file(path: &Path) -> AppResult<BackupCheck> {
    let meta = std::fs::metadata(path).map_err(|_| AppError::Validation("Berkas backup tidak ditemukan".into()))?;
    if !meta.is_file() || meta.len() < 4096 {
        return invalid("Berkas ini bukan database backup yang sah");
    }
    let mut header = [0u8; 16];
    {
        use std::io::Read;
        std::fs::File::open(path)?.read_exact(&mut header)?;
    }
    if &header != b"SQLite format 3\0" {
        return invalid("Berkas ini bukan database SQLite");
    }

    let pool = open_readonly(path)
        .await
        .map_err(|_| AppError::Validation("Database backup tidak bisa dibuka (rusak?)".into()))?;
    let result = check_pool(&pool).await;
    pool.close().await;
    result
}

async fn check_pool(pool: &SqlitePool) -> AppResult<BackupCheck> {
    let corrupt = |_| AppError::Validation("Database backup rusak dan tidak bisa dipakai".into());
    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check").fetch_one(pool).await.map_err(corrupt)?;
    if integrity != "ok" {
        return invalid("Database backup rusak (pemeriksaan integritas gagal)");
    }
    for table in ["company", "accounts", "journal_entries", "journal_lines", "audit_log", "_sqlx_migrations"] {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(table)
            .fetch_one(pool)
            .await
            .map_err(corrupt)?;
        if n == 0 {
            return invalid("Berkas ini bukan backup aplikasi Akuntansi (tabel inti tidak ada)");
        }
    }
    let schema_version: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations")
        .fetch_one(pool)
        .await
        .map_err(corrupt)?;
    let known = MIGRATOR.iter().map(|m| m.version).max().unwrap_or(0);
    if schema_version > known {
        return invalid("Backup ini dibuat oleh versi aplikasi yang lebih baru. Perbarui aplikasi dulu");
    }
    let company_name: Option<String> = sqlx::query_scalar("SELECT name FROM company WHERE id = 1")
        .fetch_optional(pool)
        .await
        .map_err(corrupt)?;
    let accounts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM accounts").fetch_one(pool).await.map_err(corrupt)?;
    let journals: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM journal_entries").fetch_one(pool).await.map_err(corrupt)?;
    Ok(BackupCheck { schema_version, company_name, accounts, journals })
}

/// Backup sebelum migrasi: hanya bila database sudah berisi migrasi lama DAN ada migrasi baru yang akan dijalankan.
pub async fn before_migration(pool: &SqlitePool, dir: &Path) -> AppResult<Option<PathBuf>> {
    let applied: Vec<i64> = match sqlx::query_scalar("SELECT version FROM _sqlx_migrations").fetch_all(pool).await {
        Ok(v) => v,
        Err(_) => return Ok(None), // database baru: tabel migrasi belum ada
    };
    let pending = MIGRATOR.iter().filter(|m| !applied.contains(&m.version)).count();
    if applied.is_empty() || pending == 0 {
        return Ok(None);
    }
    Ok(Some(create(pool, dir, Reason::PreMigration).await?))
}

async fn last_audit_id(pool: &SqlitePool) -> AppResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(MAX(id), 0) FROM audit_log").fetch_one(pool).await?)
}

/// Backup saat aplikasi ditutup. Dilewati bila tidak ada perubahan sejak backup "keluar" terakhir
/// (dibandingkan lewat id audit log terakhir, yang mencatat semua perubahan data).
pub async fn backup_on_exit(pool: &SqlitePool, dir: &Path) -> AppResult<Option<PathBuf>> {
    let current = last_audit_id(pool).await?;
    if current == 0 {
        return Ok(None); // belum ada data apa pun
    }
    if let Some(latest) = list(dir).into_iter().find(|b| b.reason == Reason::Exit.slug())
        && let Ok(old) = open_readonly(Path::new(&latest.path)).await
    {
        let previous = last_audit_id(&old).await.unwrap_or(-1);
        old.close().await;
        if previous == current {
            return Ok(None);
        }
    }
    Ok(Some(create(pool, dir, Reason::Exit).await?))
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

/// Pulihkan database dari `from`. Langkah: periksa berkas -> backup pengaman database sekarang ->
/// salin ke berkas sementara -> tutup pool, tukar berkas -> buka ulang (migrasi bila backup lebih lama).
/// Bila langkah akhir gagal, database sebelumnya dikembalikan dari backup pengaman.
pub async fn restore(state: &AppState, from: &Path) -> AppResult<BackupCheck> {
    if same_file(from, &state.db_path) {
        return invalid("Tidak bisa memulihkan dari database yang sedang dipakai");
    }
    let check = check_file(from).await?;

    let dir = state.backups_dir();
    let safety = create(&state.pool(), &dir, Reason::PreRestore).await?;

    let db_path = &state.db_path;
    let staged = db_path.with_extension("db.restore");
    std::fs::copy(from, &staged)?;

    let old = state.pool();
    old.close().await;

    let swap = || -> AppResult<()> {
        for ext in ["db-wal", "db-shm"] {
            let _ = std::fs::remove_file(db_path.with_extension(ext));
        }
        std::fs::rename(&staged, db_path)?;
        Ok(())
    };

    let outcome = match swap() {
        Ok(()) => db::init(db_path).await,
        Err(e) => Err(e),
    };
    match outcome {
        Ok(pool) => {
            state.swap_pool(pool);
            Ok(check)
        }
        Err(e) => {
            // Kembalikan database lama dari backup pengaman supaya aplikasi tetap bisa dipakai.
            let _ = std::fs::remove_file(&staged);
            for ext in ["db-wal", "db-shm"] {
                let _ = std::fs::remove_file(db_path.with_extension(ext));
            }
            let _ = std::fs::copy(&safety, db_path);
            match db::init(db_path).await {
                Ok(pool) => {
                    state.swap_pool(pool);
                }
                Err(e2) => tracing::error!("Gagal membuka ulang database setelah restore gagal: {e2}"),
            }
            Err(AppError::Other(format!("Restore gagal, database sebelumnya dipulihkan: {e}")))
        }
    }
}
