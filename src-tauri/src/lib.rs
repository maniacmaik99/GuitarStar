mod commands;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            app.manage(commands::Ctx::new(data_dir));
            app.manage(commands::CfDownloads::default());
            app.manage(commands::UpdateCheck::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::overview,
            commands::set_language,
            commands::cdlc_check_updates,
            commands::cdlc_update_status,
            commands::detect_games,
            commands::set_game_dir,
            commands::save_game_settings,
            commands::list_profiles,
            commands::save_profile,
            commands::list_asio_drivers,
            commands::linux_setup_status,
            commands::apply_audio_config,
            commands::component_releases,
            commands::install_component,
            commands::uninstall_component,
            commands::restore_snapshot,
            commands::restore_original,
            commands::monitor_sizes,
            commands::set_dpi_override,
            commands::set_keep_visible,
            commands::set_dll_log,
            commands::cdlc_overview,
            commands::cdlc_import,
            commands::cdlc_set_enabled,
            commands::cdlc_delete,
            commands::cdlc_bulk,
            commands::cdlc_sources,
            commands::customsforge_open_url,
            commands::install_cdlc_enabler,
            commands::open_customsforge,
            commands::customsforge_bounds,
            commands::hide_customsforge,
            commands::customsforge_navigate,
            commands::set_hide_ads,
            commands::default_output_device,
            commands::rsmods_status,
            commands::install_rsmods,
            commands::open_rsmods_gui,
            commands::uninstall_rsmods,
            commands::save_folders,
            commands::save_backups,
            commands::backup_saves,
            commands::auto_backup_saves,
            commands::restore_save_backup,
            commands::delete_save_backup,
            commands::open_save_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
