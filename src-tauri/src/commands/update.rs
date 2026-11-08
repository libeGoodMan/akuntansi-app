//! Pembaruan aplikasi. Tidak pernah otomatis: aplikasi hanya memberi tahu, pengguna yang memilih kapan memasang.
//! Paket pembaruan diverifikasi dengan kunci publik di tauri.conf.json sebelum dipasang.

use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::UpdaterExt;

use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::services::backup::{self, Reason};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    pub notes: Option<String>,
    /// Tanggal rilis (ISO 8601) bila ada di manifest.
    pub date: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct Progress {
    downloaded: u64,
    total: Option<u64>,
}

/// Batas waktu pengecekan: jaringan yang "menggantung" tidak boleh membuat UI menunggu tanpa akhir.
const CHECK_TIMEOUT: Duration = Duration::from_secs(20);
/// Batas total unduhan paket (diteruskan updater ke permintaan unduh); cukup longgar untuk koneksi lambat.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(15 * 60);

async fn find_update(app: &AppHandle, timeout: Duration) -> AppResult<Option<tauri_plugin_updater::Update>> {
    let updater = app
        .updater_builder()
        .timeout(timeout)
        .build()
        .map_err(|e| AppError::Other(format!("Pembaruan tidak tersedia: {e}")))?;
    updater
        .check()
        .await
        .map_err(|e| AppError::Other(format!("Tidak bisa memeriksa pembaruan: {e}")))
}

/// Periksa apakah ada versi yang lebih baru. `None` = sudah terbaru.
#[tauri::command]
pub async fn check_update(app: AppHandle) -> AppResult<Option<UpdateInfo>> {
    Ok(find_update(&app, CHECK_TIMEOUT).await?.map(|u| UpdateInfo {
        version: u.version.clone(),
        current_version: u.current_version.clone(),
        notes: u.body.clone(),
        date: u.date.map(|d| d.to_string()),
    }))
}

/// Backup data, unduh paket (diverifikasi tanda tangannya), lalu jalankan installer.
/// Kemajuan unduhan dikirim lewat event `update-progress`.
#[tauri::command]
pub async fn install_update(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    let update = find_update(&app, DOWNLOAD_TIMEOUT)
        .await?
        .ok_or_else(|| AppError::Validation("Tidak ada pembaruan yang bisa dipasang".into()))?;

    // Salinan data sebelum installer berjalan (installer menutup aplikasi, jadi tidak ada kesempatan lagi).
    let path = backup::create(&state.pool(), &state.backups_dir(), Reason::PreUpdate).await?;
    tracing::info!("Backup sebelum update: {}", path.display());

    let mut downloaded = 0u64;
    let handle = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let _ = handle.emit("update-progress", Progress { downloaded, total });
            },
            || {},
        )
        .await
        .map_err(|e| AppError::Other(format!("Pembaruan gagal dipasang: {e}")))?;
    Ok(())
}
