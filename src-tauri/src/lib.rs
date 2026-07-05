mod ai_chat;
mod ai_memory;
mod app_data;
mod clawbot_bridge;
mod codex_app_server;
mod commands;
mod favorability;
mod feature_flags;
mod kugou_music;
mod launcher;
mod netease_music;
mod startup;
mod story_mode;
mod tray;
mod updater;
mod wechat_clawbot;
mod windowing;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(codex_app_server::CodexAppServerState::default())
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
            commands::save_music_immersive_theme,
            commands::save_chat_display_preferences,
            commands::list_pet_skins,
            commands::get_current_pet_skin,
            commands::set_pet_skin,
            commands::read_pet_skin_package,
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
            commands::show_pet_bubble,
            commands::hide_pet_bubble,
            commands::reposition_pet_bubble,
            commands::show_pet_chat,
            commands::hide_pet_chat,
            commands::show_story,
            commands::hide_story,
            commands::show_translator,
            commands::hide_translator,
            commands::show_music_player,
            commands::hide_music_player,
            commands::list_music_files_in_directory,
            commands::import_music_files,
            commands::read_music_metadata,
            commands::read_music_lyrics,
            commands::create_netease_qr_login,
            commands::check_netease_qr_login,
            commands::get_netease_login_status,
            commands::clear_netease_login,
            commands::list_netease_playlists,
            commands::get_netease_playlist_detail,
            commands::search_netease_songs,
            commands::read_netease_lyrics,
            commands::get_netease_song_playback_url,
            commands::create_kugou_qr_login,
            commands::check_kugou_qr_login,
            commands::get_kugou_login_status,
            commands::clear_kugou_login,
            commands::list_kugou_playlists,
            commands::get_kugou_playlist_detail,
            commands::list_kugou_recommended_playlists,
            commands::get_kugou_recommended_playlist_detail,
            commands::get_kugou_daily_recommended_songs,
            commands::search_kugou_songs,
            commands::read_kugou_lyrics,
            commands::get_kugou_song_playback_url,
            commands::get_kugou_song_quality_availability,
            commands::get_kugou_playback_proxy_status,
            commands::send_pet_chat_message,
            commands::classify_music_intent,
            commands::send_pet_music_chat_message,
            commands::list_story_saves,
            commands::get_story_save,
            commands::create_story,
            commands::advance_story,
            commands::delete_story_save,
            commands::rename_story_save,
            commands::translate_text,
            commands::translate_selected_text,
            commands::test_ai_connection,
            commands::test_wechat_clawbot,
            commands::simulate_wechat_clawbot_message,
            commands::send_wechat_clawbot_message,
            commands::get_codex_app_server_status,
            commands::start_codex_app_server,
            commands::stop_codex_app_server,
            commands::ack_codex_notifications,
            commands::start_codex_app_server_turn,
            commands::open_codex_window,
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
            if feature_flags::WECHAT_INTEGRATION_ENABLED {
                if let Err(err) = clawbot_bridge::restart_bridge_server(app.handle()) {
                    eprintln!("ClawBot HTTP Bridge 启动失败：{err}");
                }
            }
            start_codex_app_server_on_boot(app);
            tray::create_tray(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run PetDrawer");
}

fn start_codex_app_server_on_boot(app: &tauri::App) {
    let app_handle = app.handle().clone();
    let settings = match app_data::read_config(&app_handle) {
        Ok(config) => config.codex_app_server,
        Err(err) => {
            eprintln!("读取 Codex 自动连接配置失败：{err}");
            return;
        }
    };

    if !settings.enabled || !settings.auto_start {
        return;
    }

    let state = app.state::<codex_app_server::CodexAppServerState>();
    if let Err(err) = codex_app_server::start(app_handle, &state, settings) {
        eprintln!("Codex 启动时自动连接失败：{err}");
    }
}
