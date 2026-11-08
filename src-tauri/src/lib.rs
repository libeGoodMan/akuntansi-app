mod commands;
mod db;
mod error;
mod export;
mod services;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn".into()),
        )
        .init();

    tauri::Builder::default()
        // Satu proses saja: dua jendela pada database yang sama berisiko merusak data.
        // Harus didaftarkan pertama; membuka aplikasi lagi cukup memunculkan jendela yang sudah ada.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // Database: %APPDATA%\<identifier>\data.db
            // AKUNTANSI_DATA_DIR: lokasi data alternatif untuk pengujian (tidak menyentuh data asli).
            let data_dir = match std::env::var_os("AKUNTANSI_DATA_DIR") {
                Some(dir) => std::path::PathBuf::from(dir),
                None => app.path().app_data_dir()?,
            };
            let db_path = data_dir.join("data.db");
            let pool = tauri::async_runtime::block_on(db::init(&db_path))?;
            tracing::info!("Database siap di {}", db_path.display());
            app.manage(db::AppState::new(pool, db_path));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::company::get_company,
            commands::company::setup_company,
            commands::company::update_company,
            commands::accounts::list_accounts,
            commands::accounts::create_account,
            commands::accounts::update_account,
            commands::accounts::set_account_active,
            commands::accounts::delete_account,
            commands::journal::list_journals,
            commands::journal::get_journal,
            commands::journal::create_journal,
            commands::journal::update_journal,
            commands::journal::post_journal,
            commands::journal::delete_journal,
            commands::journal::journal_history,
            commands::journal::get_opening,
            commands::journal::save_opening,
            commands::journal::delete_opening,
            commands::dashboard::get_dashboard,
            commands::update::check_update,
            commands::update::install_update,
            commands::periods::get_period_overview,
            commands::periods::fiscal_year_for,
            commands::periods::close_period,
            commands::periods::reopen_period,
            commands::periods::close_year,
            commands::periods::reopen_year,
            commands::backup::list_backups,
            commands::backup::create_backup,
            commands::backup::backup_to_path,
            commands::backup::check_backup,
            commands::backup::restore_backup,
            commands::backup::delete_backup,
            commands::reports::get_ledger,
            commands::reports::get_trial_balance,
            commands::reports::get_income_statement,
            commands::reports::get_balance_sheet,
            commands::reports::export_report,
            commands::reports::print_report,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Backup otomatis saat aplikasi ditutup (dilewati bila tidak ada perubahan sejak backup terakhir).
            if let tauri::RunEvent::Exit = event
                && let Some(state) = app.try_state::<db::AppState>()
            {
                let (pool, dir) = (state.pool(), state.backups_dir());
                match tauri::async_runtime::block_on(services::backup::backup_on_exit(&pool, &dir)) {
                    Ok(Some(path)) => tracing::info!("Backup saat keluar: {}", path.display()),
                    Ok(None) => tracing::info!("Backup saat keluar dilewati (tidak ada perubahan)"),
                    Err(e) => tracing::error!("Backup saat keluar gagal: {e}"),
                }
            }
        });
}
