use tauri::State;

use crate::db::AppState;
use crate::error::AppResult;
use crate::services::dashboard::{self, Dashboard, DashboardQuery};

#[tauri::command]
pub async fn get_dashboard(state: State<'_, AppState>, query: DashboardQuery) -> AppResult<Dashboard> {
    dashboard::dashboard(&state.pool(), query).await
}
