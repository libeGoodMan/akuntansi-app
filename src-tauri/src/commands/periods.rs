use tauri::State;

use crate::db::AppState;
use crate::error::AppResult;
use crate::services::periods::{self, PeriodOverview};

#[tauri::command]
pub async fn get_period_overview(state: State<'_, AppState>, fiscal_year: i32) -> AppResult<PeriodOverview> {
    periods::overview(&state.pool(), fiscal_year).await
}

/// Tahun fiskal yang memuat tanggal tertentu (YYYY-MM-DD); dipakai untuk memilih tahun awal di UI.
#[tauri::command]
pub async fn fiscal_year_for(state: State<'_, AppState>, date: String) -> AppResult<i32> {
    let d = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|_| crate::error::AppError::Validation("Tanggal tidak valid".into()))?;
    periods::fiscal_year_for(&state.pool(), d).await
}

#[tauri::command]
pub async fn close_period(state: State<'_, AppState>, year: i32, month: u32) -> AppResult<()> {
    periods::close_period(&state.pool(), year, month).await
}

#[tauri::command]
pub async fn reopen_period(state: State<'_, AppState>, year: i32, month: u32) -> AppResult<()> {
    periods::reopen_period(&state.pool(), year, month).await
}

#[tauri::command]
pub async fn close_year(state: State<'_, AppState>, fiscal_year: i32) -> AppResult<PeriodOverview> {
    periods::close_year(&state.pool(), fiscal_year).await
}

#[tauri::command]
pub async fn reopen_year(state: State<'_, AppState>, fiscal_year: i32) -> AppResult<PeriodOverview> {
    periods::reopen_year(&state.pool(), fiscal_year).await
}
