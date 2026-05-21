mod app_data;
mod commands;
mod launcher;
mod tray;
mod updater;
mod windowing;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_apps,
            commands::upsert_app,
            commands::delete_app,
            commands::set_app_run_as_admin,
            commands::launch_app,
            commands::open_app_dir,
            commands::get_config,
            commands::get_runtime_info,
            commands::set_quick_search_tags,
            commands::save_drawer_preferences,
            commands::list_pet_skins,
            commands::get_current_pet_skin,
            commands::set_pet_skin,
            commands::import_pet_skin,
            commands::delete_pet_skin,
            commands::import_pet_image,
            commands::reset_pet_image,
            commands::import_app_icon,
            commands::import_executable_icon,
            commands::get_image_data_url,
            commands::save_pet_position,
            commands::toggle_drawer,
            commands::show_drawer,
            commands::hide_drawer,
            commands::show_pet_menu,
            commands::hide_pet_menu,
            commands::show_pet,
            commands::hide_pet,
            commands::check_for_update,
            commands::open_update_page,
            commands::quit_app
        ])
        .setup(|app| {
            app_data::ensure_data_files(app.handle()).map_err(|err| {
                Box::<dyn std::error::Error>::from(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    err,
                ))
            })?;
            windowing::restore_pet_position(app.handle());
            windowing::apply_window_preferences(app.handle());
            tray::create_tray(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run PetDrawer");
}
