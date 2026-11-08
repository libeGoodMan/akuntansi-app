use std::path::PathBuf;

use tauri::State;

use crate::db::AppState;
use crate::error::AppResult;
use crate::services::backup::{self, BackupCheck, BackupInfo, Reason};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupOverview {
    /// Folder backup otomatis.
    pub dir: String,
    pub backups: Vec<BackupInfo>,
}

#[tauri::command]
pub async fn list_backups(state: State<'_, AppState>) -> AppResult<BackupOverview> {
    let dir = state.backups_dir();
    Ok(BackupOverview { dir: dir.to_string_lossy().to_string(), backups: backup::list(&dir) })
}

/// "Backup sekarang": simpan ke folder backup otomatis (jenis manual, tidak dirotasi).
#[tauri::command]
pub async fn create_backup(state: State<'_, AppState>) -> AppResult<String> {
    let path = backup::create(&state.pool(), &state.backups_dir(), Reason::Manual).await?;
    Ok(path.to_string_lossy().to_string())
}

/// Backup ke lokasi pilihan pengguna (hasil dialog simpan).
#[tauri::command]
pub async fn backup_to_path(state: State<'_, AppState>, path: String) -> AppResult<String> {
    backup::backup_to(&state.pool(), &PathBuf::from(&path)).await?;
    Ok(path)
}

/// Periksa berkas backup sebelum dipulihkan (untuk ditampilkan di dialog konfirmasi).
#[tauri::command]
pub async fn check_backup(path: String) -> AppResult<BackupCheck> {
    backup::check_file(&PathBuf::from(path)).await
}

#[tauri::command]
pub async fn restore_backup(state: State<'_, AppState>, path: String) -> AppResult<BackupCheck> {
    backup::restore(&state, &PathBuf::from(path)).await
}

#[tauri::command]
pub async fn delete_backup(state: State<'_, AppState>, file_name: String) -> AppResult<()> {
    backup::delete(&state.backups_dir(), &file_name)
}
