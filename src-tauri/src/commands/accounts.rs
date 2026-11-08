use tauri::State;

use crate::db::AppState;
use crate::error::AppResult;
use crate::services::accounts::{self, Account, AccountInput};

#[tauri::command]
pub async fn list_accounts(state: State<'_, AppState>) -> AppResult<Vec<Account>> {
    accounts::list(&state.pool()).await
}

#[tauri::command]
pub async fn create_account(state: State<'_, AppState>, input: AccountInput) -> AppResult<Account> {
    accounts::create(&state.pool(), input).await
}

#[tauri::command]
pub async fn update_account(
    state: State<'_, AppState>,
    id: i64,
    input: AccountInput,
) -> AppResult<Account> {
    accounts::update(&state.pool(), id, input).await
}

#[tauri::command]
pub async fn set_account_active(
    state: State<'_, AppState>,
    id: i64,
    active: bool,
) -> AppResult<Account> {
    accounts::set_active(&state.pool(), id, active).await
}

#[tauri::command]
pub async fn delete_account(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    accounts::delete(&state.pool(), id).await
}
