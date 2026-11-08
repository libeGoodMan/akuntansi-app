use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::export::{self, Format, ReportRequest};
use crate::services::reports::{self, LedgerPage, LedgerQuery, TrialBalance, TrialBalanceQuery};
use crate::services::statements::{
    self, BalanceSheet, BalanceSheetQuery, IncomeStatement, IncomeStatementQuery,
};

#[tauri::command]
pub async fn get_ledger(state: State<'_, AppState>, query: LedgerQuery) -> AppResult<LedgerPage> {
    reports::ledger(&state.pool(), query).await
}

#[tauri::command]
pub async fn get_trial_balance(
    state: State<'_, AppState>,
    query: TrialBalanceQuery,
) -> AppResult<TrialBalance> {
    reports::trial_balance(&state.pool(), query).await
}

#[tauri::command]
pub async fn get_income_statement(
    state: State<'_, AppState>,
    query: IncomeStatementQuery,
) -> AppResult<IncomeStatement> {
    statements::income_statement(&state.pool(), query).await
}

#[tauri::command]
pub async fn get_balance_sheet(
    state: State<'_, AppState>,
    query: BalanceSheetQuery,
) -> AppResult<BalanceSheet> {
    statements::balance_sheet(&state.pool(), query).await
}

/// Simpan laporan ke `path` (hasil dialog simpan). Ekstensi harus sesuai format.
#[tauri::command]
pub async fn export_report(
    state: State<'_, AppState>,
    request: ReportRequest,
    format: Format,
    path: String,
) -> AppResult<String> {
    let target = PathBuf::from(&path);
    let ext_ok = target
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(format.extension()));
    if !ext_ok {
        return Err(AppError::Validation(format!("Berkas harus berekstensi .{}", format.extension())));
    }
    if !target.parent().is_some_and(Path::is_dir) {
        return Err(AppError::Validation("Folder tujuan tidak ditemukan".into()));
    }

    let (bytes, _) = export::build(&state.pool(), request, format).await?;
    std::fs::write(&target, bytes)?;
    tracing::info!("Laporan diekspor ke {}", target.display());
    Ok(path)
}

/// Buat PDF di folder sementara lalu buka di aplikasi PDF bawaan Windows untuk dicetak.
#[tauri::command]
pub async fn print_report(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ReportRequest,
) -> AppResult<()> {
    let (bytes, name) = export::build(&state.pool(), request, Format::Pdf).await?;

    let dir = std::env::temp_dir().join("akuntansi-cetak");
    std::fs::create_dir_all(&dir)?;
    remove_old_prints(&dir);
    let file = dir.join(name);
    std::fs::write(&file, bytes)?;

    app.opener()
        .open_path(file.to_string_lossy(), None::<&str>)
        .map_err(|e| AppError::Other(format!("Gagal membuka PDF: {e}")))?;
    Ok(())
}

/// Hapus berkas cetak sementara yang lebih tua dari sehari.
fn remove_old_prints(dir: &Path) {
    let limit = Duration::from_secs(24 * 3600);
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let old = e
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| SystemTime::now().duration_since(t).ok())
            .is_some_and(|age| age > limit);
        if old {
            let _ = std::fs::remove_file(e.path());
        }
    }
}
