use tauri::State;

use crate::db::AppState;
use crate::error::AppResult;
use crate::services::journal::{self, HistoryItem, JournalDetail, JournalFilter, JournalInput, JournalPage};
use crate::services::opening::{self, OpeningInput};

#[tauri::command]
pub async fn list_journals(state: State<'_, AppState>, filter: JournalFilter) -> AppResult<JournalPage> {
    journal::list(&state.pool(), filter).await
}

#[tauri::command]
pub async fn get_journal(state: State<'_, AppState>, id: i64) -> AppResult<JournalDetail> {
    journal::get(&state.pool(), id).await
}

#[tauri::command]
pub async fn create_journal(
    state: State<'_, AppState>,
    input: JournalInput,
    post: bool,
) -> AppResult<JournalDetail> {
    journal::create(&state.pool(), input, post).await
}

#[tauri::command]
pub async fn update_journal(
    state: State<'_, AppState>,
    id: i64,
    input: JournalInput,
    post: bool,
) -> AppResult<JournalDetail> {
    journal::update(&state.pool(), id, input, post).await
}

#[tauri::command]
pub async fn post_journal(state: State<'_, AppState>, id: i64) -> AppResult<JournalDetail> {
    journal::post(&state.pool(), id).await
}

#[tauri::command]
pub async fn delete_journal(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    journal::delete(&state.pool(), id).await
}

#[tauri::command]
pub async fn journal_history(state: State<'_, AppState>, id: i64) -> AppResult<Vec<HistoryItem>> {
    journal::history(&state.pool(), id).await
}

#[tauri::command]
pub async fn get_opening(state: State<'_, AppState>) -> AppResult<Option<JournalDetail>> {
    opening::get(&state.pool()).await
}

#[tauri::command]
pub async fn save_opening(state: State<'_, AppState>, input: OpeningInput) -> AppResult<JournalDetail> {
    opening::save(&state.pool(), input).await
}

#[tauri::command]
pub async fn delete_opening(state: State<'_, AppState>) -> AppResult<()> {
    opening::delete(&state.pool()).await
}
