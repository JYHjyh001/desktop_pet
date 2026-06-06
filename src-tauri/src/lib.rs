mod ai_chat;
mod ai_memory;
mod app_data;
mod clawbot_bridge;
mod commands;
mod favorability;
mod launcher;
mod startup;
mod story_mode;
mod tray;
mod updater;
mod wechat_clawbot;
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
            commands::list_companions,
            commands::get_current_companion,
            commands::upsert_companion,
            commands::import_companion_card,
            commands::export_companion_card,
            commands::switch_companion,
            commands::delete_companion,
            commands::get_current_companion_status,
            commands::set_current_companion_favorability_enabled,
            commands::set_current_companion_favorability,
            commands::reset_current_companion_favorability,
            commands::list_current_companion_favorability_logs,
            commands::get_companion_messages,
            commands::delete_companion_messages,
            commands::get_runtime_info,
            commands::get_storage_settings,
            commands::save_storage_settings,
            commands::set_quick_search_tags,
            commands::save_drawer_preferences,
            commands::save_chat_display_preferences,
            commands::list_pet_skins,
            commands::get_current_pet_skin,
            commands::set_pet_skin,
            commands::import_pet_skin,
            commands::update_pet_skin,
            commands::delete_pet_skin,
            commands::import_pet_image,
            commands::reset_pet_image,
            commands::import_app_icon,
            commands::import_executable_icon,
            commands::get_image_data_url,
            commands::save_pet_position,
            commands::is_primary_mouse_button_pressed,
            commands::toggle_drawer,
            commands::show_drawer,
            commands::hide_drawer,
            commands::show_pet_menu,
            commands::hide_pet_menu,
            commands::show_pet_chat,
            commands::hide_pet_chat,
            commands::show_story,
            commands::hide_story,
            commands::send_pet_chat_message,
            commands::list_story_saves,
            commands::get_story_save,
            commands::create_story,
            commands::advance_story,
            commands::delete_story_save,
            commands::rename_story_save,
            commands::test_ai_connection,
            commands::test_wechat_clawbot,
            commands::send_wechat_clawbot_message,
            commands::list_pet_memories,
            commands::add_pet_memory,
            commands::update_pet_memory,
            commands::delete_pet_memory,
            commands::clear_pet_memories,
            commands::clear_pet_memory_messages,
            commands::import_pet_memory,
            commands::export_pet_memory,
            commands::open_pet_memory_dir,
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
            if let Err(err) = clawbot_bridge::restart_bridge_server(app.handle()) {
                eprintln!("ClawBot HTTP Bridge 启动失败：{err}");
            }
            tray::create_tray(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run PetDrawer");
}
