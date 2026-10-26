mod commands;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            app.manage(commands::Ctx::new(data_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::overview,
            commands::detect_games,
            commands::set_game_dir,
            commands::save_game_settings,
            commands::list_profiles,
            commands::save_profile,
            commands::list_asio_drivers,
            commands::apply_audio_config,
            commands::component_releases,
            commands::install_component,
            commands::uninstall_component,
            commands::restore_snapshot,
            commands::restore_original,
            commands::monitor_sizes,
            commands::set_dpi_override,
            commands::set_keep_visible,
            commands::cdlc_overview,
            commands::cdlc_import,
            commands::cdlc_set_enabled,
            commands::cdlc_delete,
            commands::install_cdlc_enabler,
            commands::open_customsforge,
            commands::customsforge_bounds,
            commands::hide_customsforge,
            commands::customsforge_navigate,
            commands::set_hide_ads,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
