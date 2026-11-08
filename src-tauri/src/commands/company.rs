use tauri::State;

use crate::db::AppState;
use crate::error::AppResult;
use crate::services::company::{self, Company, SetupInput, UpdateInput};

#[tauri::command]
pub async fn get_company(state: State<'_, AppState>) -> AppResult<Option<Company>> {
    company::get(&state.pool()).await
}

#[tauri::command]
pub async fn setup_company(state: State<'_, AppState>, input: SetupInput) -> AppResult<Company> {
    company::setup(&state.pool(), input).await
}

#[tauri::command]
pub async fn update_company(state: State<'_, AppState>, input: UpdateInput) -> AppResult<Company> {
    company::update(&state.pool(), input).await
}
