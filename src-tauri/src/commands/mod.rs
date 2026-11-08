pub mod accounts;
pub mod backup;
pub mod company;
pub mod dashboard;
pub mod journal;
pub mod periods;
pub mod reports;
pub mod update;

use serde::Serialize;
use tauri::State;

use crate::db::AppState;
use crate::error::AppResult;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub db_path: String,
    pub schema_version: i64,
    pub company_configured: bool,
}

/// Info dasar aplikasi; juga dipakai frontend untuk memastikan database & migrasi berjalan.
#[tauri::command]
pub async fn app_info(state: State<'_, AppState>) -> AppResult<AppInfo> {
    let schema_version: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations")
            .fetch_one(&state.pool())
            .await?;
    let company_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM company")
        .fetch_one(&state.pool())
        .await?;

    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        db_path: state.db_path.display().to_string(),
        schema_version,
        company_configured: company_count > 0,
    })
}
