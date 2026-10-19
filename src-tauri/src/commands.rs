use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use lofty::{
    picture::PictureType,
    prelude::{Accessor, AudioFile, ItemKey, TaggedFileExt},
    tag::Tag,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Emitter, Listener, Manager, WebviewWindow};

use crate::{
    ai_chat::{
        self, AiConnectionTestResult, MusicChatActionContext, MusicIntentContext, MusicIntentReply,
        PetChatMessageDraft, PetChatReply, TranslationReply, TranslationRequest,
    },
    ai_memory::{self, PetMemory, PetMemoryDraft, PetMemoryMessage},
    app_data::{
        self, AiConnectionProfile, AiSettings, AppDraft, CodexAppServerSettings, Companion,
        CompanionDraft, PetAnimationSet, PetApp, PetDrawerConfig, PetPosition, PetSkinPackageDraft,
        PetSkinSummary, ShortcutSettings, StorageSettings, WechatClawbotSettings,
    },
    clawbot_bridge, codex_app_server,
    favorability::{self, CompanionStatus, FavorabilityLog},
    kugou_music, launcher, netease_music, pet_atlas_converter, startup,
    story_mode::{self, StoryCreateDraft, StorySave, StoryTurnReply},
    translation_api::{self, TranslationSettingsDraft, TranslationSettingsView},
    updater, wechat_clawbot, windowing,
};
use tauri::State;

#[tauri::command]
pub fn window_frontend_ready(app: AppHandle, window: WebviewWindow) -> Result<(), String> {
    require_window(
        &window,
        &[
            "pet",
            "drawer",
            "pet-menu",
            "pet-bubble",
            "pet-chat",
            "story",
            "translator",
            "music",
        ],
    )?;
    crate::window_lifecycle::mark_ready(&app, window.label())
}

#[tauri::command]
pub fn cancel_ai_requests(window: WebviewWindow) -> Result<(), String> {
    require_window(
        &window,
        &["drawer", "pet", "pet-chat", "story", "translator"],
    )?;
    crate::ai_transport::cancel_owner(window.label());
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawerPreferencesDraft {
    pub categories: Vec<String>,
    pub quick_search_tags: Vec<String>,
    pub tag_display_mode: String,
    #[serde(default = "default_drawer_theme")]
    pub theme: String,
    #[serde(default = "default_music_immersive_theme")]
    pub music_immersive_theme: String,
    #[serde(default = "default_true")]
    pub chat_typewriter_enabled: bool,
    #[serde(default)]
    pub chat_narration_enabled: bool,
    #[serde(default = "default_true")]
    pub chat_music_link_enabled: bool,
    #[serde(default = "default_pet_size")]
    pub pet_size: u32,
    pub pet_always_on_top: bool,
    pub drawer_always_on_top: bool,
    #[serde(default)]
    pub start_on_boot: bool,
    #[serde(default = "default_true")]
    pub auto_favorite_enabled: bool,
    #[serde(default)]
    pub shortcut: ShortcutSettingsDraft,
    #[serde(default)]
    pub ai: AiSettingsDraft,
    #[serde(default)]
    pub wechat_clawbot: WechatClawbotSettingsDraft,
    #[serde(default)]
    pub codex_app_server: CodexAppServerSettingsDraft,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutSettingsDraft {
    #[serde(default = "default_toggle_drawer_shortcut")]
    pub toggle_drawer: String,
    #[serde(default = "default_translate_selection_shortcut")]
    pub translate_selection: String,
    #[serde(default)]
    pub translate_selection_consent: bool,
    #[serde(default = "default_pet_single_click_action")]
    pub pet_single_click: String,
    #[serde(default = "default_pet_double_click_action")]
    pub pet_double_click: String,
    #[serde(default = "default_pet_right_click_action")]
    pub pet_right_click: String,
}

impl Default for ShortcutSettingsDraft {
    fn default() -> Self {
        Self {
            toggle_drawer: default_toggle_drawer_shortcut(),
            translate_selection: default_translate_selection_shortcut(),
            translate_selection_consent: false,
            pet_single_click: default_pet_single_click_action(),
            pet_double_click: default_pet_double_click_action(),
            pet_right_click: default_pet_right_click_action(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettingsDraft {
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub memory_enabled: bool,
    #[serde(default = "default_true")]
    pub short_memory_summary_enabled: bool,
    #[serde(default = "default_short_memory_recent_turns")]
    pub short_memory_recent_turns: usize,
    #[serde(default = "default_short_memory_compression_trigger_turns")]
    pub short_memory_compression_trigger_turns: usize,
    pub provider: String,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub system_prompt: String,
    pub temperature: f32,
    pub max_tokens: u32,
    #[serde(default = "default_emoji_frequency")]
    pub emoji_frequency: String,
    #[serde(default)]
    pub active_profile_id: String,
    #[serde(default)]
    pub profiles: Vec<AiConnectionProfileDraft>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConnectionProfileDraft {
    pub id: String,
    pub label: String,
    pub provider: String,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WechatClawbotSettingsDraft {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub openclaw_command: String,
    #[serde(default)]
    pub channel: String,
    #[serde(default)]
    pub account: String,
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub forward_user_messages: bool,
    #[serde(default)]
    pub forward_assistant_messages: bool,
    #[serde(default = "default_true")]
    pub friend_mode_enabled: bool,
    #[serde(default)]
    pub bridge_enabled: bool,
    #[serde(default)]
    pub bridge_host: String,
    #[serde(default)]
    pub bridge_port: u16,
    #[serde(default)]
    pub bridge_path: String,
    #[serde(default)]
    pub bridge_token: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexAppServerSettingsDraft {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub socket_path: String,
    #[serde(default)]
    pub port: u16,
    #[serde(default = "default_true")]
    pub completion_notifications_enabled: bool,
}

impl Default for AiSettingsDraft {
    fn default() -> Self {
        let settings = AiSettings::default();
        Self {
            enabled: settings.enabled,
            memory_enabled: settings.memory_enabled,
            short_memory_summary_enabled: settings.short_memory_summary_enabled,
            short_memory_recent_turns: settings.short_memory_recent_turns,
            short_memory_compression_trigger_turns: settings.short_memory_compression_trigger_turns,
            provider: settings.provider,
            api_key: settings.api_key,
            base_url: settings.base_url,
            model: settings.model,
            system_prompt: settings.system_prompt,
            temperature: settings.temperature,
            max_tokens: settings.max_tokens,
            emoji_frequency: settings.emoji_frequency,
            active_profile_id: settings.active_profile_id,
            profiles: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInfo {
    version: String,
    executable_path: String,
    default_data_dir: String,
    data_dir: String,
    memory_dir: String,
    pet_assets_dir: String,
    icons_dir: String,
    storage_config_file: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicImportItem {
    source_path: String,
    path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicLibraryStorageInfo {
    data_dir: String,
    library_file: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicMetadataResult {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    cover_img_url: Option<String>,
    duration: Option<u64>,
    source: String,
    confidence: f32,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicLyricsResult {
    content: String,
    source: String,
    warnings: Vec<String>,
}

#[tauri::command]
pub fn get_apps(app: AppHandle, window: WebviewWindow) -> Result<Vec<PetApp>, String> {
    require_command_window(&window, "get_apps")?;
    app_data::read_apps(&app)
}

fn reorder_app_list(apps: Vec<PetApp>, app_ids: &[String]) -> Result<Vec<PetApp>, String> {
    if app_ids.len() != apps.len() {
        return Err("快捷入口排序数量与当前列表不一致".to_string());
    }

    let mut apps_by_id = apps
        .into_iter()
        .map(|item| (item.id.clone(), item))
        .collect::<std::collections::HashMap<_, _>>();
    let mut reordered = Vec::with_capacity(app_ids.len());

    for app_id in app_ids {
        let item = apps_by_id
            .remove(app_id)
            .ok_or_else(|| format!("快捷入口排序包含重复或未知 ID：{app_id}"))?;
        reordered.push(item);
    }

    if !apps_by_id.is_empty() {
        return Err("快捷入口排序缺少当前列表中的项目".to_string());
    }

    Ok(reordered)
}

#[tauri::command]
pub fn reorder_apps(
    app: AppHandle,
    window: WebviewWindow,
    app_ids: Vec<String>,
) -> Result<(), String> {
    require_command_window(&window, "reorder_apps")?;
    let apps = app_data::read_apps(&app)?;
    let reordered = reorder_app_list(apps, &app_ids)?;
    app_data::write_apps(&app, &reordered)
}

#[tauri::command]
pub fn upsert_app(
    app: AppHandle,
    window: WebviewWindow,
    draft: AppDraft,
) -> Result<PetApp, String> {
    require_command_window(&window, "upsert_app")?;
    let mut apps = app_data::read_apps(&app)?;
    let now = app_data::now_seconds();
    let item_kind = normalize_item_kind(&draft.item_kind);
    let run_as_admin = item_kind == "app" && draft.run_as_admin;

    if let Some(id) = draft.id.as_deref() {
        let index = apps
            .iter()
            .position(|item| item.id == id)
            .ok_or_else(|| "未找到要编辑的快捷入口".to_string())?;

        apps[index].name = draft.name;
        apps[index].item_kind = item_kind;
        apps[index].path = draft.path;
        if let Some(icon) = draft.icon {
            apps[index].icon = Some(icon);
        }
        apps[index].category = draft.category;
        apps[index].run_as_admin = run_as_admin;
        apps[index].tags = draft.tags;
        apps[index].favorite = draft.favorite;
        apps[index].auto_favorite = false;

        let updated = apps[index].clone();
        app_data::write_apps(&app, &apps)?;
        return Ok(updated);
    }

    let created = PetApp {
        id: app_data::new_app_id(),
        name: draft.name,
        item_kind,
        path: draft.path,
        icon: draft.icon,
        category: draft.category,
        run_as_admin,
        tags: draft.tags,
        favorite: draft.favorite,
        auto_favorite: false,
        launch_count: 0,
        launch_history: Vec::new(),
        last_launch_at: None,
        created_at: now,
    };

    apps.insert(0, created.clone());
    app_data::write_apps(&app, &apps)?;

    Ok(created)
}

fn normalize_item_kind(value: &str) -> String {
    match value {
        "folder" | "website" | "file" => value.to_string(),
        _ => "app".to_string(),
    }
}

#[tauri::command]
pub fn delete_app(app: AppHandle, window: WebviewWindow, app_id: String) -> Result<(), String> {
    require_command_window(&window, "delete_app")?;
    let mut apps = app_data::read_apps(&app)?;
    let before_len = apps.len();
    apps.retain(|item| item.id != app_id);

    if apps.len() == before_len {
        return Err("未找到要删除的快捷入口".to_string());
    }

    app_data::write_apps(&app, &apps)
}

#[tauri::command]
pub fn set_app_run_as_admin(
    app: AppHandle,
    window: WebviewWindow,
    app_id: String,
    run_as_admin: bool,
) -> Result<PetApp, String> {
    require_command_window(&window, "set_app_run_as_admin")?;
    let mut apps = app_data::read_apps(&app)?;
    let index = apps
        .iter()
        .position(|item| item.id == app_id)
        .ok_or_else(|| "未找到要更新的快捷入口".to_string())?;

    apps[index].run_as_admin = apps[index].item_kind == "app" && run_as_admin;
    let updated = apps[index].clone();
    app_data::write_apps(&app, &apps)?;

    Ok(updated)
}

#[tauri::command]
pub fn launch_app(app: AppHandle, window: WebviewWindow, app_id: String) -> Result<PetApp, String> {
    require_command_window(&window, "launch_app")?;
    launcher::launch_app(&app, &app_id)
}

#[tauri::command]
pub fn open_app_dir(app: AppHandle, window: WebviewWindow, app_id: String) -> Result<(), String> {
    require_command_window(&window, "open_app_dir")?;
    launcher::open_app_dir(&app, &app_id)
}

#[tauri::command]
pub fn get_config(app: AppHandle, window: WebviewWindow) -> Result<PetDrawerConfig, String> {
    require_command_window(&window, "get_config")?;
    let mut config = read_runtime_config(&app)?;
    config.translation.api_key.clear();
    config.translation.base_url = public_ai_origin(&config.translation.base_url);
    Ok(config)
}

#[tauri::command]
pub fn get_public_config(app: AppHandle) -> Result<PetDrawerConfig, String> {
    read_runtime_config(&app).map(redact_public_config)
}

fn read_runtime_config(app: &AppHandle) -> Result<PetDrawerConfig, String> {
    let mut config = app_data::read_config(app)?;
    if let Ok(start_on_boot) = startup::is_start_on_boot_enabled() {
        if start_on_boot {
            config.system.start_on_boot = true;
        }
    }

    Ok(config)
}

fn require_window(window: &WebviewWindow, allowed_labels: &[&str]) -> Result<(), String> {
    if is_window_label_allowed(window.label(), allowed_labels) {
        Ok(())
    } else {
        Err(format!("窗口 {} 无权执行此操作", window.label()))
    }
}

fn is_window_label_allowed(label: &str, allowed_labels: &[&str]) -> bool {
    allowed_labels.contains(&label)
}

fn allowed_windows_for_command(command: &str) -> Option<&'static [&'static str]> {
    match command {
        "get_apps"
        | "upsert_app"
        | "reorder_apps"
        | "delete_app"
        | "set_app_run_as_admin"
        | "launch_app"
        | "open_app_dir"
        | "get_config"
        | "get_runtime_info"
        | "get_storage_settings"
        | "save_storage_settings"
        | "set_quick_search_tags"
        | "save_drawer_preferences"
        | "upsert_companion"
        | "import_companion_card"
        | "export_companion_card"
        | "switch_companion"
        | "delete_companion"
        | "list_pet_skins"
        | "set_pet_skin"
        | "read_pet_skin_package"
        | "convert_pet_atlas"
        | "cleanup_pet_atlas_conversion"
        | "import_pet_skin"
        | "update_pet_skin"
        | "delete_pet_skin"
        | "import_pet_image"
        | "reset_pet_image"
        | "import_app_icon"
        | "import_executable_icon"
        | "get_image_data_url"
        | "test_ai_connection"
        | "test_wechat_clawbot"
        | "simulate_wechat_clawbot_message"
        | "start_codex_app_server"
        | "stop_codex_app_server"
        | "start_codex_app_server_turn"
        | "list_pet_memories"
        | "add_pet_memory"
        | "update_pet_memory"
        | "delete_pet_memory"
        | "clear_pet_memories"
        | "clear_pet_memory_messages"
        | "import_pet_memory"
        | "export_pet_memory"
        | "open_pet_memory_dir"
        | "check_for_update"
        | "open_update_page" => Some(&["drawer"]),
        "list_companions" => Some(&["drawer", "story"]),
        "get_current_companion" => Some(&["drawer", "pet-chat"]),
        "get_current_pet_skin" => Some(&["drawer", "pet-chat", "pet"]),
        "save_chat_display_preferences"
        | "get_current_companion_status"
        | "set_current_companion_favorability_enabled"
        | "set_current_companion_favorability"
        | "reset_current_companion_favorability"
        | "list_current_companion_favorability_logs"
        | "get_companion_messages"
        | "delete_companion_messages"
        | "get_music_library_summary"
        | "send_pet_chat_message"
        | "classify_music_intent"
        | "send_pet_music_chat_message"
        | "send_wechat_clawbot_message" => Some(&["pet-chat"]),
        "list_story_saves" | "get_story_save" | "create_story" | "advance_story"
        | "delete_story_save" | "rename_story_save" => Some(&["story"]),
        "translate_text"
        | "get_translation_settings"
        | "save_translation_settings"
        | "test_translation_connection" => Some(&["translator"]),
        "translate_selected_text"
        | "save_pet_position"
        | "is_primary_mouse_button_pressed"
        | "open_codex_window" => Some(&["pet"]),
        "save_music_immersive_theme"
        | "get_music_library_state"
        | "save_music_library_state"
        | "get_music_library_storage_info"
        | "list_music_files_in_directory"
        | "import_music_files"
        | "read_music_metadata"
        | "read_music_lyrics"
        | "create_netease_qr_login"
        | "check_netease_qr_login"
        | "get_netease_login_status"
        | "clear_netease_login"
        | "list_netease_playlists"
        | "get_netease_playlist_detail"
        | "search_netease_songs"
        | "read_netease_lyrics"
        | "get_netease_song_playback_url"
        | "create_kugou_qr_login"
        | "check_kugou_qr_login"
        | "get_kugou_login_status"
        | "clear_kugou_login"
        | "list_kugou_playlists"
        | "get_kugou_playlist_detail"
        | "list_kugou_recommended_playlists"
        | "get_kugou_recommended_playlist_detail"
        | "get_kugou_daily_recommended_songs"
        | "search_kugou_songs"
        | "read_kugou_lyrics"
        | "get_kugou_song_playback_url"
        | "get_kugou_song_quality_availability"
        | "get_kugou_playback_proxy_status" => Some(&["music"]),
        "get_codex_app_server_status" | "ack_codex_notifications" => {
            Some(&["drawer", "pet", "pet-bubble"])
        }
        _ => None,
    }
}

#[cfg(test)]
fn is_command_allowed_for_window(command: &str, label: &str) -> bool {
    allowed_windows_for_command(command)
        .is_some_and(|allowed_labels| is_window_label_allowed(label, allowed_labels))
}

fn require_command_window(window: &WebviewWindow, command: &str) -> Result<(), String> {
    let allowed_labels = allowed_windows_for_command(command)
        .ok_or_else(|| format!("命令 {command} 未配置窗口授权"))?;
    require_window(window, allowed_labels)
}

fn redact_public_config(mut config: PetDrawerConfig) -> PetDrawerConfig {
    config.pet.x = None;
    config.pet.y = None;
    config.pet.custom_image = None;
    config.drawer.categories.clear();
    config.drawer.quick_search_tags.clear();
    config.system.start_on_boot = false;
    config.system.auto_favorite_enabled = false;

    config.ai.api_key.clear();
    config.ai.base_url = public_ai_origin(&config.ai.base_url);
    config.ai.system_prompt.clear();
    config.ai.active_profile_id.clear();
    config.ai.profiles.clear();
    config.translation.api_key.clear();
    config.translation.base_url = public_ai_origin(&config.translation.base_url);

    config.wechat_clawbot.openclaw_command.clear();
    config.wechat_clawbot.channel.clear();
    config.wechat_clawbot.account.clear();
    config.wechat_clawbot.target.clear();
    config.wechat_clawbot.bridge_host.clear();
    config.wechat_clawbot.bridge_port = 0;
    config.wechat_clawbot.bridge_path.clear();
    config.wechat_clawbot.bridge_token.clear();

    config.codex_app_server.command.clear();
    config.codex_app_server.socket_path.clear();
    config.codex_app_server.port = 0;
    config.companions.clear();
    config.current_companion_id.clear();
    config.companions_initialized = false;
    config
}

fn public_ai_origin(base_url: &str) -> String {
    reqwest::Url::parse(base_url.trim())
        .ok()
        .filter(|url| matches!(url.scheme(), "http" | "https"))
        .map(|url| url.origin().ascii_serialization())
        .unwrap_or_default()
}

#[tauri::command]
pub fn list_companions(app: AppHandle, window: WebviewWindow) -> Result<Vec<Companion>, String> {
    require_command_window(&window, "list_companions")?;
    ai_memory::list_companions(&app)
}

#[tauri::command]
pub fn get_current_companion(app: AppHandle, window: WebviewWindow) -> Result<Companion, String> {
    require_command_window(&window, "get_current_companion")?;
    ai_memory::current_companion(&app)
}

#[tauri::command]
pub fn upsert_companion(
    app: AppHandle,
    window: WebviewWindow,
    draft: CompanionDraft,
) -> Result<Companion, String> {
    require_command_window(&window, "upsert_companion")?;
    ai_memory::upsert_companion(&app, draft)
}

#[tauri::command]
pub fn import_companion_card(
    app: AppHandle,
    window: WebviewWindow,
    path: String,
) -> Result<Companion, String> {
    require_command_window(&window, "import_companion_card")?;
    ai_memory::import_companion_card(&app, &path)
}

#[tauri::command]
pub fn export_companion_card(
    app: AppHandle,
    window: WebviewWindow,
    companion_id: String,
    path: String,
) -> Result<(), String> {
    require_command_window(&window, "export_companion_card")?;
    ai_memory::export_companion_card(&app, &companion_id, &path)
}

#[tauri::command]
pub fn switch_companion(
    app: AppHandle,
    window: WebviewWindow,
    companion_id: String,
) -> Result<Companion, String> {
    require_command_window(&window, "switch_companion")?;
    let mut companion = ai_memory::switch_companion(&app, &companion_id)?;
    if app_data::set_current_pet_skin(&app, &companion.skin_id).is_err() {
        app_data::set_current_pet_skin(&app, "default")?;
        ai_memory::set_current_companion_skin(&app, "default")?;
        companion.skin_id = "default".to_string();
    }
    Ok(companion)
}

#[tauri::command]
pub fn delete_companion(
    app: AppHandle,
    window: WebviewWindow,
    companion_id: String,
) -> Result<Companion, String> {
    require_command_window(&window, "delete_companion")?;
    let mut companion = ai_memory::delete_companion(&app, &companion_id)?;
    let _ = favorability::delete_companion_data(&app, &companion_id);
    if app_data::set_current_pet_skin(&app, &companion.skin_id).is_err() {
        app_data::set_current_pet_skin(&app, "default")?;
        ai_memory::set_current_companion_skin(&app, "default")?;
        companion.skin_id = "default".to_string();
    }
    Ok(companion)
}

#[tauri::command]
pub fn get_current_companion_status(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<CompanionStatus, String> {
    require_command_window(&window, "get_current_companion_status")?;
    favorability::get_current_companion_status(&app)
}

#[tauri::command]
pub fn set_current_companion_favorability_enabled(
    app: AppHandle,
    window: WebviewWindow,
    enabled: bool,
) -> Result<CompanionStatus, String> {
    require_command_window(&window, "set_current_companion_favorability_enabled")?;
    favorability::set_current_enabled(&app, enabled)
}

#[tauri::command]
pub fn set_current_companion_favorability(
    app: AppHandle,
    window: WebviewWindow,
    value: i32,
) -> Result<CompanionStatus, String> {
    require_command_window(&window, "set_current_companion_favorability")?;
    favorability::set_current_favorability(&app, value)
}

#[tauri::command]
pub fn reset_current_companion_favorability(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<CompanionStatus, String> {
    require_command_window(&window, "reset_current_companion_favorability")?;
    favorability::reset_current_favorability(&app)
}

#[tauri::command]
pub fn list_current_companion_favorability_logs(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<Vec<FavorabilityLog>, String> {
    require_command_window(&window, "list_current_companion_favorability_logs")?;
    favorability::list_current_logs(&app, 100)
}

#[tauri::command]
pub fn get_companion_messages(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<Vec<PetMemoryMessage>, String> {
    require_command_window(&window, "get_companion_messages")?;
    let companion_id = ai_memory::current_companion_id(&app)?;
    ai_memory::recent_messages(&app, &companion_id, 20)
}

#[tauri::command]
pub fn get_companion_message_page(
    app: AppHandle,
    window: WebviewWindow,
    before: Option<u64>,
    limit: Option<usize>,
) -> Result<crate::message_history::MessagePage, String> {
    require_command_window(&window, "get_companion_messages")?;
    ai_memory::message_page(&app, before, limit.unwrap_or(20))
}

#[tauri::command]
pub fn get_companion_message(
    app: AppHandle,
    window: WebviewWindow,
    id: u64,
) -> Result<PetMemoryMessage, String> {
    require_command_window(&window, "get_companion_messages")?;
    ai_memory::message_by_id(&app, id)
}

#[tauri::command]
pub async fn load_chat_image(
    app: AppHandle,
    window: WebviewWindow,
    source: String,
) -> Result<String, String> {
    require_window(&window, &["pet-chat"])?;
    let owner = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        crate::ai_transport::with_scope(&owner, || crate::chat_media::load_image(&app, &source))
    })
    .await
    .map_err(|_| "图片加载任务失败")?
}

#[tauri::command]
pub fn delete_companion_messages(
    app: AppHandle,
    window: WebviewWindow,
    message_ids: Vec<u64>,
) -> Result<usize, String> {
    require_command_window(&window, "delete_companion_messages")?;
    ai_memory::delete_messages(&app, message_ids)
}

#[tauri::command]
pub fn get_runtime_info(app: AppHandle, window: WebviewWindow) -> Result<RuntimeInfo, String> {
    require_command_window(&window, "get_runtime_info")?;
    let executable_path = std::env::current_exe()
        .map_err(|err| format!("无法获取当前程序路径：{err}"))?
        .to_string_lossy()
        .to_string();
    let storage = app_data::effective_storage_dirs(&app)?;

    Ok(RuntimeInfo {
        version: app.package_info().version.to_string(),
        executable_path,
        default_data_dir: storage.default_data_dir,
        data_dir: storage.data_dir,
        memory_dir: storage.memory_dir,
        pet_assets_dir: storage.pet_assets_dir,
        icons_dir: storage.icons_dir,
        storage_config_file: storage.storage_config_file,
    })
}

#[tauri::command]
pub fn get_storage_settings(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<StorageSettings, String> {
    require_command_window(&window, "get_storage_settings")?;
    app_data::read_storage_settings(&app)
}

#[tauri::command]
pub fn save_storage_settings(
    app: AppHandle,
    window: WebviewWindow,
    settings: StorageSettings,
) -> Result<StorageSettings, String> {
    require_command_window(&window, "save_storage_settings")?;
    app_data::save_storage_settings(&app, &settings)
}

#[tauri::command]
pub fn set_quick_search_tags(
    app: AppHandle,
    window: WebviewWindow,
    tags: Vec<String>,
) -> Result<Vec<String>, String> {
    require_command_window(&window, "set_quick_search_tags")?;
    let mut seen = std::collections::HashSet::new();
    let normalized: Vec<String> = tags
        .into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .filter(|tag| seen.insert(tag.to_lowercase()))
        .take(20)
        .collect();

    let mut config = app_data::read_config(&app)?;
    config.drawer.quick_search_tags = normalized.clone();
    app_data::write_config(&app, &config)?;

    Ok(normalized)
}

#[tauri::command]
pub fn save_drawer_preferences(
    app: AppHandle,
    window: WebviewWindow,
    preferences: DrawerPreferencesDraft,
) -> Result<PetDrawerConfig, String> {
    require_command_window(&window, "save_drawer_preferences")?;
    let categories = normalize_unique_list(preferences.categories, 30);
    let quick_search_tags = normalize_unique_list(preferences.quick_search_tags, 20);
    let tag_display_mode = if preferences.tag_display_mode == "detailed" {
        "detailed".to_string()
    } else {
        "compact".to_string()
    };
    let theme = normalize_drawer_theme(preferences.theme);
    let music_immersive_theme = normalize_music_immersive_theme(preferences.music_immersive_theme);
    let pet_size = app_data::normalize_pet_size(preferences.pet_size);

    let mut config = app_data::read_config(&app)?;
    startup::set_start_on_boot(preferences.start_on_boot)?;

    config.drawer.categories = ensure_core_categories(categories);
    config.drawer.quick_search_tags = quick_search_tags;
    config.drawer.tag_display_mode = tag_display_mode;
    config.drawer.theme = theme;
    config.drawer.music_immersive_theme = music_immersive_theme;
    config.drawer.chat_typewriter_enabled = preferences.chat_typewriter_enabled;
    config.drawer.chat_narration_enabled = preferences.chat_narration_enabled;
    config.drawer.chat_music_link_enabled = preferences.chat_music_link_enabled;
    config.drawer.always_on_top = preferences.drawer_always_on_top;
    config.pet.size = pet_size;
    config.pet.always_on_top = preferences.pet_always_on_top;
    config.system.start_on_boot = preferences.start_on_boot;
    config.system.auto_favorite_enabled = preferences.auto_favorite_enabled;
    config.shortcut = normalize_shortcut_settings(preferences.shortcut);
    config.ai = normalize_ai_settings(preferences.ai);
    config.wechat_clawbot = normalize_wechat_clawbot_settings(preferences.wechat_clawbot);
    config.codex_app_server = normalize_codex_app_server_settings(preferences.codex_app_server);
    app_data::write_config_from_settings(&app, &config)?;
    let _ = app.emit("translation-config-changed", ());
    clawbot_bridge::restart_bridge_server(&app)?;

    windowing::set_pet_size(&app, pet_size)?;
    windowing::set_pet_always_on_top(&app, preferences.pet_always_on_top)?;
    windowing::set_drawer_always_on_top(&app, preferences.drawer_always_on_top)?;

    config.translation.api_key.clear();
    config.translation.base_url = public_ai_origin(&config.translation.base_url);
    Ok(config)
}

#[tauri::command]
pub fn save_music_immersive_theme(
    app: AppHandle,
    window: WebviewWindow,
    theme: String,
) -> Result<PetDrawerConfig, String> {
    require_command_window(&window, "save_music_immersive_theme")?;
    let music_immersive_theme = normalize_music_immersive_theme(theme);
    let mut config = app_data::read_config(&app)?;
    config.drawer.music_immersive_theme = music_immersive_theme;
    app_data::write_config(&app, &config)?;

    Ok(redact_public_config(config))
}

#[tauri::command]
pub fn save_chat_display_preferences(
    app: AppHandle,
    window: WebviewWindow,
    chat_typewriter_enabled: bool,
    chat_narration_enabled: bool,
    chat_music_link_enabled: bool,
) -> Result<PetDrawerConfig, String> {
    require_command_window(&window, "save_chat_display_preferences")?;
    let mut config = app_data::read_config(&app)?;
    config.drawer.chat_typewriter_enabled = chat_typewriter_enabled;
    config.drawer.chat_narration_enabled = chat_narration_enabled;
    config.drawer.chat_music_link_enabled = chat_music_link_enabled;
    app_data::write_config(&app, &config)?;
    Ok(redact_public_config(config))
}

fn normalize_unique_list(items: Vec<String>, max_items: usize) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    items
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .filter(|item| seen.insert(item.to_lowercase()))
        .take(max_items)
        .collect()
}

fn default_true() -> bool {
    true
}

fn default_toggle_drawer_shortcut() -> String {
    "Ctrl+Space".to_string()
}

fn default_translate_selection_shortcut() -> String {
    "Ctrl+Alt+T".to_string()
}

fn default_pet_single_click_action() -> String {
    "smartCodexOrDrawer".to_string()
}

fn default_pet_double_click_action() -> String {
    "toggleDrawer".to_string()
}

fn default_pet_right_click_action() -> String {
    "petMenu".to_string()
}

fn default_pet_size() -> u32 {
    app_data::DEFAULT_PET_SIZE
}

fn default_short_memory_recent_turns() -> usize {
    10
}

fn default_short_memory_compression_trigger_turns() -> usize {
    12
}

fn default_drawer_theme() -> String {
    "light".to_string()
}

fn default_music_immersive_theme() -> String {
    "follow".to_string()
}

fn normalize_wechat_clawbot_settings(
    settings: WechatClawbotSettingsDraft,
) -> WechatClawbotSettings {
    let defaults = WechatClawbotSettings::default();
    let openclaw_command = settings.openclaw_command.trim().to_string();
    let channel = settings.channel.trim().to_string();
    let bridge_host = settings.bridge_host.trim().to_string();
    let bridge_path = settings.bridge_path.trim().to_string();

    WechatClawbotSettings {
        enabled: settings.enabled,
        openclaw_command: if openclaw_command.is_empty() {
            defaults.openclaw_command
        } else {
            openclaw_command
        },
        channel: if channel.is_empty() {
            defaults.channel
        } else {
            channel
        },
        account: settings.account.trim().to_string(),
        target: settings.target.trim().to_string(),
        forward_user_messages: settings.forward_user_messages,
        forward_assistant_messages: settings.forward_assistant_messages,
        friend_mode_enabled: settings.friend_mode_enabled,
        bridge_enabled: settings.bridge_enabled,
        bridge_host: if bridge_host.is_empty() {
            defaults.bridge_host
        } else {
            bridge_host
        },
        bridge_port: if settings.bridge_port == 0 {
            defaults.bridge_port
        } else {
            settings.bridge_port
        },
        bridge_path: if bridge_path.is_empty() {
            defaults.bridge_path
        } else if bridge_path.starts_with('/') {
            bridge_path
        } else {
            format!("/{bridge_path}")
        },
        bridge_token: settings.bridge_token.trim().to_string(),
    }
}

fn normalize_shortcut_settings(settings: ShortcutSettingsDraft) -> ShortcutSettings {
    let translate_selection = settings.translate_selection.trim();
    let translate_selection = if translate_selection.is_empty() {
        String::new()
    } else {
        translate_selection.chars().take(64).collect()
    };
    let translate_selection_consent =
        !translate_selection.is_empty() && settings.translate_selection_consent;

    ShortcutSettings {
        toggle_drawer: {
            let value = settings.toggle_drawer.trim();
            if value.is_empty() {
                default_toggle_drawer_shortcut()
            } else {
                value.to_string()
            }
        },
        translate_selection,
        translate_selection_consent,
        pet_single_click: normalize_pet_action(
            &settings.pet_single_click,
            &default_pet_single_click_action(),
        ),
        pet_double_click: normalize_pet_action(
            &settings.pet_double_click,
            &default_pet_double_click_action(),
        ),
        pet_right_click: normalize_pet_action(
            &settings.pet_right_click,
            &default_pet_right_click_action(),
        ),
    }
}

fn normalize_pet_action(value: &str, fallback: &str) -> String {
    match value.trim() {
        "smartCodexOrDrawer" | "toggleDrawer" | "showDrawer" | "petMenu" | "petChat" | "story"
        | "music" | "none" => value.trim().to_string(),
        _ => fallback.to_string(),
    }
}

fn normalize_codex_app_server_settings(
    settings: CodexAppServerSettingsDraft,
) -> CodexAppServerSettings {
    let defaults = CodexAppServerSettings::default();
    let command = settings.command.trim().to_string();

    CodexAppServerSettings {
        enabled: settings.enabled,
        auto_start: settings.enabled && settings.auto_start,
        mode: normalize_codex_app_server_mode(&settings.mode),
        command: if command.is_empty() {
            defaults.command
        } else {
            command
        },
        socket_path: settings.socket_path.trim().to_string(),
        port: settings.port,
        completion_notifications_enabled: settings.completion_notifications_enabled,
    }
}

fn normalize_codex_app_server_mode(value: &str) -> String {
    match value.trim() {
        "managed" => "managed".to_string(),
        "sessionLog" => "sessionLog".to_string(),
        "proxy" => "proxy".to_string(),
        _ => CodexAppServerSettings::default().mode,
    }
}

fn normalize_drawer_theme(theme: String) -> String {
    match theme.trim().to_lowercase().as_str() {
        "animal-island" => "animal-island".to_string(),
        _ => default_drawer_theme(),
    }
}

fn normalize_music_immersive_theme(theme: String) -> String {
    match theme.trim().to_lowercase().as_str() {
        "light" => "light".to_string(),
        "animal-island" => "animal-island".to_string(),
        "cinema" => "cinema".to_string(),
        "galaxy" => "galaxy".to_string(),
        "neon" => "neon".to_string(),
        "sunset" => "sunset".to_string(),
        "midnight" => "midnight".to_string(),
        _ => default_music_immersive_theme(),
    }
}

fn ensure_core_categories(categories: Vec<String>) -> Vec<String> {
    let mut output = Vec::new();
    for category in ["全部", "常用"] {
        output.push(category.to_string());
    }

    for category in categories {
        if !output
            .iter()
            .any(|item| item.to_lowercase() == category.to_lowercase())
        {
            output.push(category);
        }
    }

    if !output.iter().any(|item| item == "其他") {
        output.push("其他".to_string());
    }

    output
}

fn normalize_ai_settings(settings: AiSettingsDraft) -> AiSettings {
    let profiles = normalize_ai_profiles(settings.profiles);
    let requested_profile_id = settings.active_profile_id.trim();
    let active_profile_id = profiles
        .iter()
        .find(|profile| profile.id == requested_profile_id)
        .map(|profile| profile.id.clone())
        .unwrap_or_default();

    AiSettings {
        enabled: settings.enabled,
        memory_enabled: settings.memory_enabled,
        short_memory_summary_enabled: settings.short_memory_summary_enabled,
        short_memory_recent_turns: settings.short_memory_recent_turns.clamp(2, 40),
        short_memory_compression_trigger_turns: settings
            .short_memory_compression_trigger_turns
            .clamp(4, 80),
        provider: normalize_ai_provider(settings.provider),
        api_key: settings.api_key.trim().to_string(),
        base_url: normalize_ai_base_url(settings.base_url),
        model: settings.model.trim().to_string(),
        system_prompt: settings.system_prompt.trim().to_string(),
        temperature: settings.temperature.clamp(0.0, 2.0),
        max_tokens: settings.max_tokens.clamp(64, 32768),
        emoji_frequency: normalize_emoji_frequency(settings.emoji_frequency),
        active_profile_id,
        profiles,
    }
}

fn normalize_ai_profiles(profiles: Vec<AiConnectionProfileDraft>) -> Vec<AiConnectionProfile> {
    let mut ids = std::collections::HashSet::new();
    let mut labels = std::collections::HashSet::new();

    profiles
        .into_iter()
        .filter_map(|profile| {
            let id = profile.id.trim().chars().take(80).collect::<String>();
            let label = profile.label.trim().chars().take(40).collect::<String>();
            if id.is_empty()
                || label.is_empty()
                || !ids.insert(id.to_lowercase())
                || !labels.insert(label.to_lowercase())
            {
                return None;
            }

            Some(AiConnectionProfile {
                id,
                label,
                provider: normalize_ai_provider(profile.provider),
                api_key: profile.api_key.trim().to_string(),
                base_url: normalize_ai_base_url(profile.base_url),
                model: profile.model.trim().to_string(),
            })
        })
        .take(20)
        .collect()
}

fn normalize_ai_provider(provider: String) -> String {
    let provider = provider.trim().to_lowercase();
    match provider.as_str() {
        "openai" | "deepseek" | "anthropic" | "gemini" | "ollama" | "custom" => provider,
        _ => "custom".to_string(),
    }
}

fn normalize_ai_base_url(base_url: String) -> String {
    let trimmed = base_url.trim();
    if trimmed.ends_with('/') {
        trimmed.trim_end_matches('/').to_string()
    } else {
        trimmed.to_string()
    }
}

fn default_emoji_frequency() -> String {
    "normal".to_string()
}

fn normalize_emoji_frequency(value: String) -> String {
    match value.trim().to_lowercase().as_str() {
        "none" | "low" | "normal" | "high" => value.trim().to_lowercase(),
        _ => default_emoji_frequency(),
    }
}

#[tauri::command]
pub fn list_pet_skins(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<Vec<PetSkinSummary>, String> {
    require_command_window(&window, "list_pet_skins")?;
    app_data::list_pet_skins(&app)
}

#[tauri::command]
pub fn get_current_pet_skin(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<PetSkinSummary, String> {
    require_command_window(&window, "get_current_pet_skin")?;
    app_data::get_current_pet_skin(&app)
}

#[tauri::command]
pub fn set_pet_skin(
    app: AppHandle,
    window: WebviewWindow,
    skin_id: String,
) -> Result<PetSkinSummary, String> {
    require_command_window(&window, "set_pet_skin")?;
    let skin = app_data::set_current_pet_skin(&app, &skin_id)?;
    ai_memory::set_current_companion_skin(&app, &skin.id)?;
    Ok(skin)
}

#[tauri::command]
pub fn read_pet_skin_package(
    window: WebviewWindow,
    path: String,
) -> Result<PetSkinPackageDraft, String> {
    require_command_window(&window, "read_pet_skin_package")?;
    app_data::read_pet_skin_package(&path)
}

#[tauri::command]
pub fn convert_pet_atlas(
    window: WebviewWindow,
    path: String,
) -> Result<pet_atlas_converter::PetAtlasConversionDraft, String> {
    require_command_window(&window, "convert_pet_atlas")?;
    pet_atlas_converter::convert(&path)
}

#[tauri::command]
pub fn cleanup_pet_atlas_conversion(window: WebviewWindow, path: String) -> Result<(), String> {
    require_command_window(&window, "cleanup_pet_atlas_conversion")?;
    pet_atlas_converter::cleanup(&path)
}

#[tauri::command]
pub fn import_pet_skin(
    app: AppHandle,
    window: WebviewWindow,
    name: String,
    animations: PetAnimationSet,
) -> Result<PetSkinSummary, String> {
    require_command_window(&window, "import_pet_skin")?;
    let skin = app_data::import_pet_skin(&app, &name, animations)?;
    ai_memory::set_current_companion_skin(&app, &skin.id)?;
    Ok(skin)
}

#[tauri::command]
pub fn update_pet_skin(
    app: AppHandle,
    window: WebviewWindow,
    skin_id: String,
    name: String,
    animations: PetAnimationSet,
    cleared_states: Vec<String>,
) -> Result<PetSkinSummary, String> {
    require_command_window(&window, "update_pet_skin")?;
    app_data::update_pet_skin(&app, &skin_id, &name, animations, cleared_states)
}

#[tauri::command]
pub fn delete_pet_skin(
    app: AppHandle,
    window: WebviewWindow,
    skin_id: String,
) -> Result<PetSkinSummary, String> {
    require_command_window(&window, "delete_pet_skin")?;
    let skin = app_data::delete_pet_skin(&app, &skin_id)?;
    ai_memory::set_current_companion_skin(&app, &skin.id)?;
    Ok(skin)
}

#[tauri::command]
pub fn import_pet_image(
    app: AppHandle,
    window: WebviewWindow,
    path: String,
) -> Result<String, String> {
    require_command_window(&window, "import_pet_image")?;
    let relative_path = app_data::import_image(&app, &path, "pets", "pet")?;
    let mut config = app_data::read_config(&app)?;
    config.pet.current_skin = "default".to_string();
    config.pet.custom_image = Some(relative_path.clone());
    app_data::write_config(&app, &config)?;
    Ok(relative_path)
}

#[tauri::command]
pub fn reset_pet_image(app: AppHandle, window: WebviewWindow) -> Result<(), String> {
    require_command_window(&window, "reset_pet_image")?;
    app_data::set_current_pet_skin(&app, "default")?;
    ai_memory::set_current_companion_skin(&app, "default")
}

#[tauri::command]
pub fn import_app_icon(
    app: AppHandle,
    window: WebviewWindow,
    path: String,
) -> Result<String, String> {
    require_command_window(&window, "import_app_icon")?;
    app_data::import_image(&app, &path, "icons", "icon")
}

#[tauri::command]
pub fn import_executable_icon(
    app: AppHandle,
    window: WebviewWindow,
    path: String,
) -> Result<String, String> {
    require_command_window(&window, "import_executable_icon")?;
    app_data::import_executable_icon(&app, &path)
}

#[tauri::command]
pub fn get_image_data_url(
    app: AppHandle,
    window: WebviewWindow,
    relative_path: String,
) -> Result<String, String> {
    require_command_window(&window, "get_image_data_url")?;
    app_data::read_image_data_url(&app, &relative_path)
}

#[tauri::command]
pub fn save_pet_position(
    app: AppHandle,
    window: WebviewWindow,
    position: PetPosition,
) -> Result<(), String> {
    require_command_window(&window, "save_pet_position")?;
    windowing::save_pet_position(&app, position.x, position.y)
}

#[tauri::command]
pub fn is_primary_mouse_button_pressed(window: WebviewWindow) -> Result<bool, String> {
    require_command_window(&window, "is_primary_mouse_button_pressed")?;
    Ok(primary_mouse_button_pressed())
}

#[cfg(windows)]
fn primary_mouse_button_pressed() -> bool {
    use windows_sys::Win32::UI::{
        Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON},
        WindowsAndMessaging::{GetSystemMetrics, SM_SWAPBUTTON},
    };

    let button = unsafe {
        if GetSystemMetrics(SM_SWAPBUTTON) != 0 {
            VK_RBUTTON
        } else {
            VK_LBUTTON
        }
    };

    unsafe { GetAsyncKeyState(i32::from(button)) < 0 }
}

#[cfg(not(windows))]
fn primary_mouse_button_pressed() -> bool {
    false
}

#[tauri::command]
pub async fn toggle_drawer(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || windowing::toggle_drawer(&app))
        .await
        .map_err(|_| "窗口任务失败")?
}

#[tauri::command]
pub async fn show_drawer(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || windowing::show_drawer(&app))
        .await
        .map_err(|_| "窗口任务失败")?
}

#[tauri::command]
pub fn hide_drawer(app: AppHandle) -> Result<(), String> {
    windowing::hide_drawer(&app)
}

#[tauri::command]
pub async fn show_pet_menu(app: AppHandle, x: i32, y: i32) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || windowing::show_pet_menu(&app, x, y))
        .await
        .map_err(|_| "窗口任务失败")?
}

#[tauri::command]
pub fn hide_pet_menu(app: AppHandle) -> Result<(), String> {
    windowing::hide_pet_menu(&app)
}

#[tauri::command]
pub async fn show_pet_bubble(
    app: AppHandle,
    payload: windowing::PetBubblePayload,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || windowing::show_pet_bubble(&app, payload))
        .await
        .map_err(|_| "窗口任务失败")?
}

#[tauri::command]
pub fn hide_pet_bubble(app: AppHandle) -> Result<(), String> {
    windowing::hide_pet_bubble(&app)
}

#[tauri::command]
pub fn reposition_pet_bubble(app: AppHandle) -> Result<(), String> {
    windowing::reposition_pet_bubble(&app)
}

#[tauri::command]
pub async fn show_pet_chat(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || windowing::show_pet_chat(&app))
        .await
        .map_err(|_| "窗口任务失败")?
}

#[tauri::command]
pub fn hide_pet_chat(app: AppHandle) -> Result<(), String> {
    windowing::hide_pet_chat(&app)
}

#[tauri::command]
pub async fn show_story(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || windowing::show_story(&app))
        .await
        .map_err(|_| "窗口任务失败")?
}

#[tauri::command]
pub fn hide_story(app: AppHandle) -> Result<(), String> {
    windowing::hide_story(&app)
}

#[tauri::command]
pub async fn show_translator(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || windowing::show_translator(&app))
        .await
        .map_err(|_| "窗口任务失败")?
}

#[tauri::command]
pub fn hide_translator(app: AppHandle) -> Result<(), String> {
    windowing::hide_translator(&app)
}

#[tauri::command]
pub async fn show_music_player(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || windowing::show_music_player(&app))
        .await
        .map_err(|_| "窗口任务失败")?
}

#[tauri::command]
pub fn hide_music_player(app: AppHandle) -> Result<(), String> {
    windowing::hide_music_player(&app)
}

#[tauri::command]
pub fn get_music_library_state(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<serde_json::Value, String> {
    require_command_window(&window, "get_music_library_state")?;
    let state = app_data::read_music_library_state(&app)?;
    allow_music_library_assets(&app, &state)?;
    Ok(state)
}

#[tauri::command]
pub fn save_music_library_state(
    app: AppHandle,
    window: WebviewWindow,
    state: serde_json::Value,
) -> Result<(), String> {
    require_command_window(&window, "save_music_library_state")?;
    app_data::write_music_library_state(&app, &state)?;
    allow_music_library_assets(&app, &state)
}

#[tauri::command]
pub fn get_music_library_summary(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<app_data::MusicLibrarySummary, String> {
    require_command_window(&window, "get_music_library_summary")?;
    app_data::read_music_library_summary(&app)
}

#[tauri::command]
pub fn get_music_library_storage_info(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<MusicLibraryStorageInfo, String> {
    require_command_window(&window, "get_music_library_storage_info")?;
    Ok(MusicLibraryStorageInfo {
        data_dir: app_data::data_dir(&app)?.to_string_lossy().to_string(),
        library_file: app_data::music_library_file(&app)?
            .to_string_lossy()
            .to_string(),
    })
}

#[tauri::command]
pub fn list_music_files_in_directory(
    window: WebviewWindow,
    directory: String,
) -> Result<Vec<String>, String> {
    require_command_window(&window, "list_music_files_in_directory")?;
    let root = PathBuf::from(directory.trim());
    if !root.is_dir() {
        return Err("请选择有效的音乐文件夹".to_string());
    }

    let mut paths = Vec::new();
    collect_music_files(&root, &mut paths)?;
    paths.sort_by_key(|path| path.to_string_lossy().to_lowercase());

    Ok(paths
        .into_iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect())
}

#[tauri::command]
pub fn import_music_files(
    app: AppHandle,
    window: WebviewWindow,
    paths: Vec<String>,
    storage_dir: String,
) -> Result<Vec<MusicImportItem>, String> {
    require_command_window(&window, "import_music_files")?;
    let storage_dir = storage_dir.trim();
    let storage = if storage_dir.is_empty() {
        None
    } else {
        let path = PathBuf::from(storage_dir);
        fs::create_dir_all(&path).map_err(|err| format!("无法创建音乐存储目录：{err}"))?;
        if !path.is_dir() {
            return Err("音乐存储目录无效".to_string());
        }
        Some(path)
    };

    let mut output = Vec::new();
    for source in paths {
        let source_path = PathBuf::from(source.trim());
        if !source_path.is_file() || !is_supported_music_file(&source_path) {
            continue;
        }

        let target_path = if let Some(storage) = storage.as_ref() {
            copy_music_file_to_storage(&source_path, storage)?
        } else {
            source_path.clone()
        };

        app.asset_protocol_scope()
            .allow_file(&target_path)
            .map_err(|err| format!("授权音乐播放文件失败：{err}"))?;
        output.push(MusicImportItem {
            source_path: source_path.to_string_lossy().to_string(),
            path: target_path.to_string_lossy().to_string(),
        });
    }

    if output.is_empty() {
        return Err("没有找到可导入的音频文件".to_string());
    }

    Ok(output)
}

#[tauri::command]
pub fn read_music_metadata(
    window: WebviewWindow,
    path: String,
) -> Result<MusicMetadataResult, String> {
    require_command_window(&window, "read_music_metadata")?;
    let music_path = PathBuf::from(path.trim());
    if !music_path.is_file() || !is_supported_music_file(&music_path) {
        return Err("请选择有效的音频文件".to_string());
    }

    let tagged_file = lofty::read_from_path(&music_path)
        .map_err(|err| format!("无法读取音频 metadata：{err}"))?;
    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());
    let title = tag.and_then(read_music_title);
    let artist = tag.and_then(read_music_artist);
    let album = tag.and_then(read_music_album);
    let duration = {
        let seconds = tagged_file.properties().duration().as_secs();
        if seconds > 0 {
            Some(seconds)
        } else {
            None
        }
    };

    let mut warnings = Vec::new();
    let cover_img_url = read_music_cover_img_url(tag, &mut warnings);
    if title.is_none() {
        warnings.push("metadata 中没有读取到歌名".to_string());
    }
    if artist.is_none() {
        warnings.push("metadata 中没有读取到歌手".to_string());
    }

    let confidence = match (title.is_some(), artist.is_some()) {
        (true, true) => 0.95,
        (true, false) | (false, true) => 0.62,
        (false, false) => 0.0,
    };

    Ok(MusicMetadataResult {
        title,
        artist,
        album,
        cover_img_url,
        duration,
        source: "metadata".to_string(),
        confidence,
        warnings,
    })
}

#[tauri::command]
pub fn read_music_lyrics(
    app: AppHandle,
    window: WebviewWindow,
    path: String,
    source_path: Option<String>,
) -> Result<Option<MusicLyricsResult>, String> {
    require_command_window(&window, "read_music_lyrics")?;
    let music_path = PathBuf::from(path.trim());
    if !music_path.is_file() || !is_supported_music_file(&music_path) {
        return Err("请选择有效的音频文件".to_string());
    }

    let library_state = app_data::read_music_library_state(&app)?;
    let candidates =
        registered_lyrics_candidates(&library_state, &music_path, source_path.as_deref())?;
    if !app.asset_protocol_scope().is_allowed(&music_path) {
        return Err("音频文件未经用户选择或音乐库授权，已拒绝读取歌词".to_string());
    }

    let mut warnings = Vec::new();

    for candidate in candidates {
        let Some(stem) = candidate.file_stem().and_then(|item| item.to_str()) else {
            continue;
        };
        let Some(parent) = candidate.parent() else {
            continue;
        };

        if let Some(result) = read_music_lyrics_next_to_file(parent, stem, &mut warnings)? {
            return Ok(Some(result));
        }
    }

    Ok(None)
}

#[tauri::command]
pub async fn create_netease_qr_login(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<netease_music::NeteaseQrLogin, String> {
    require_command_window(&window, "create_netease_qr_login")?;
    tauri::async_runtime::spawn_blocking(move || netease_music::create_qr_login(&app))
        .await
        .map_err(|err| format!("网易云二维码创建任务失败：{err}"))?
}

#[tauri::command]
pub async fn check_netease_qr_login(
    app: AppHandle,
    window: WebviewWindow,
    key: String,
) -> Result<netease_music::NeteaseQrCheckResult, String> {
    require_command_window(&window, "check_netease_qr_login")?;
    tauri::async_runtime::spawn_blocking(move || netease_music::check_qr_login(&app, key))
        .await
        .map_err(|err| format!("网易云登录检查任务失败：{err}"))?
}

#[tauri::command]
pub async fn get_netease_login_status(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<netease_music::NeteaseLoginStatus, String> {
    require_command_window(&window, "get_netease_login_status")?;
    tauri::async_runtime::spawn_blocking(move || netease_music::login_status(&app))
        .await
        .map_err(|err| format!("网易云登录状态读取任务失败：{err}"))?
}

#[tauri::command]
pub async fn clear_netease_login(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<netease_music::NeteaseLoginStatus, String> {
    require_command_window(&window, "clear_netease_login")?;
    tauri::async_runtime::spawn_blocking(move || netease_music::clear_login(&app))
        .await
        .map_err(|err| format!("网易云登录清除任务失败：{err}"))?
}

#[tauri::command]
pub async fn list_netease_playlists(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<Vec<netease_music::NeteasePlaylistSummary>, String> {
    require_command_window(&window, "list_netease_playlists")?;
    tauri::async_runtime::spawn_blocking(move || netease_music::list_playlists(&app))
        .await
        .map_err(|err| format!("网易云歌单读取任务失败：{err}"))?
}

#[tauri::command]
pub async fn get_netease_playlist_detail(
    app: AppHandle,
    window: WebviewWindow,
    playlist_id: u64,
    page: Option<u64>,
    limit: Option<u64>,
) -> Result<netease_music::NeteasePlaylistDetail, String> {
    require_command_window(&window, "get_netease_playlist_detail")?;
    tauri::async_runtime::spawn_blocking(move || {
        netease_music::playlist_detail(&app, playlist_id, page, limit)
    })
    .await
    .map_err(|err| format!("网易云歌单详情读取任务失败：{err}"))?
}

#[tauri::command]
pub async fn search_netease_songs(
    window: WebviewWindow,
    keyword: String,
    page: u64,
    limit: u64,
) -> Result<netease_music::NeteaseSearchResult, String> {
    require_command_window(&window, "search_netease_songs")?;
    tauri::async_runtime::spawn_blocking(move || netease_music::search_songs(keyword, page, limit))
        .await
        .map_err(|err| format!("网易云搜索任务失败：{err}"))?
}

#[tauri::command]
pub async fn read_netease_lyrics(
    app: AppHandle,
    window: WebviewWindow,
    song_id: u64,
) -> Result<netease_music::NeteaseLyricsResult, String> {
    require_command_window(&window, "read_netease_lyrics")?;
    tauri::async_runtime::spawn_blocking(move || netease_music::song_lyrics(&app, song_id))
        .await
        .map_err(|err| format!("网易云歌词读取任务失败：{err}"))?
}

#[tauri::command]
pub async fn get_netease_song_playback_url(
    app: AppHandle,
    window: WebviewWindow,
    song_id: u64,
    level: Option<String>,
) -> Result<netease_music::NeteasePlaybackUrl, String> {
    require_command_window(&window, "get_netease_song_playback_url")?;
    tauri::async_runtime::spawn_blocking(move || {
        netease_music::song_playback_url(&app, song_id, level)
    })
    .await
    .map_err(|err| format!("网易云播放链接获取任务失败：{err}"))?
}

#[tauri::command]
pub async fn create_kugou_qr_login(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<kugou_music::KugouQrLogin, String> {
    require_command_window(&window, "create_kugou_qr_login")?;
    tauri::async_runtime::spawn_blocking(move || kugou_music::create_qr_login(&app))
        .await
        .map_err(|err| format!("酷狗二维码登录创建任务失败：{err}"))?
}

#[tauri::command]
pub async fn check_kugou_qr_login(
    app: AppHandle,
    window: WebviewWindow,
    key: String,
) -> Result<kugou_music::KugouQrCheckResult, String> {
    require_command_window(&window, "check_kugou_qr_login")?;
    tauri::async_runtime::spawn_blocking(move || kugou_music::check_qr_login(&app, key))
        .await
        .map_err(|err| format!("酷狗登录检查任务失败：{err}"))?
}

#[tauri::command]
pub async fn get_kugou_login_status(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<kugou_music::KugouLoginStatus, String> {
    require_command_window(&window, "get_kugou_login_status")?;
    tauri::async_runtime::spawn_blocking(move || kugou_music::login_status(&app))
        .await
        .map_err(|err| format!("酷狗登录状态读取任务失败：{err}"))?
}

#[tauri::command]
pub async fn clear_kugou_login(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<kugou_music::KugouLoginStatus, String> {
    require_command_window(&window, "clear_kugou_login")?;
    tauri::async_runtime::spawn_blocking(move || kugou_music::clear_login(&app))
        .await
        .map_err(|err| format!("酷狗登录清除任务失败：{err}"))?
}

#[tauri::command]
pub async fn list_kugou_playlists(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<Vec<kugou_music::KugouPlaylistSummary>, String> {
    require_command_window(&window, "list_kugou_playlists")?;
    tauri::async_runtime::spawn_blocking(move || kugou_music::list_playlists(&app))
        .await
        .map_err(|err| format!("酷狗歌单读取任务失败：{err}"))?
}

#[tauri::command]
pub async fn get_kugou_playlist_detail(
    app: AppHandle,
    window: WebviewWindow,
    list_id: String,
    page: Option<u64>,
    limit: Option<u64>,
) -> Result<kugou_music::KugouPlaylistDetail, String> {
    require_command_window(&window, "get_kugou_playlist_detail")?;
    tauri::async_runtime::spawn_blocking(move || {
        kugou_music::playlist_detail(&app, list_id, page, limit)
    })
    .await
    .map_err(|err| format!("酷狗歌单歌曲读取任务失败：{err}"))?
}

#[tauri::command]
pub async fn list_kugou_recommended_playlists(
    app: AppHandle,
    window: WebviewWindow,
    page: Option<u64>,
    limit: Option<u64>,
) -> Result<kugou_music::KugouRecommendedPlaylists, String> {
    require_command_window(&window, "list_kugou_recommended_playlists")?;
    tauri::async_runtime::spawn_blocking(move || {
        kugou_music::recommended_playlists(&app, page, limit)
    })
    .await
    .map_err(|err| format!("酷狗推荐歌单读取任务失败：{err}"))?
}

#[tauri::command]
pub async fn get_kugou_recommended_playlist_detail(
    app: AppHandle,
    window: WebviewWindow,
    playlist_id: String,
    page: Option<u64>,
    limit: Option<u64>,
) -> Result<kugou_music::KugouPlaylistDetail, String> {
    require_command_window(&window, "get_kugou_recommended_playlist_detail")?;
    tauri::async_runtime::spawn_blocking(move || {
        kugou_music::recommended_playlist_detail(&app, playlist_id, page, limit)
    })
    .await
    .map_err(|err| format!("酷狗推荐歌单歌曲读取任务失败：{err}"))?
}

#[tauri::command]
pub async fn get_kugou_daily_recommended_songs(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<kugou_music::KugouSearchResult, String> {
    require_command_window(&window, "get_kugou_daily_recommended_songs")?;
    tauri::async_runtime::spawn_blocking(move || kugou_music::daily_recommended_songs(&app))
        .await
        .map_err(|err| format!("酷狗每日推荐读取任务失败：{err}"))?
}

#[tauri::command]
pub async fn search_kugou_songs(
    window: WebviewWindow,
    keyword: String,
    page: u64,
    limit: u64,
) -> Result<kugou_music::KugouSearchResult, String> {
    require_command_window(&window, "search_kugou_songs")?;
    tauri::async_runtime::spawn_blocking(move || kugou_music::search_songs(keyword, page, limit))
        .await
        .map_err(|err| format!("酷狗搜索任务失败：{err}"))?
}

#[tauri::command]
pub async fn read_kugou_lyrics(
    window: WebviewWindow,
    hash: String,
    name: String,
    artist: String,
    duration_ms: Option<u64>,
) -> Result<kugou_music::KugouLyricsResult, String> {
    require_command_window(&window, "read_kugou_lyrics")?;
    tauri::async_runtime::spawn_blocking(move || {
        kugou_music::read_lyrics(hash, name, artist, duration_ms)
    })
    .await
    .map_err(|err| format!("酷狗歌词读取任务失败：{err}"))?
}

#[tauri::command]
pub async fn get_kugou_song_playback_url(
    app: AppHandle,
    window: WebviewWindow,
    hash: String,
    hash_candidates: Option<Vec<String>>,
    album_audio_id: Option<u64>,
    audio_id: Option<u64>,
    playback_quality: Option<String>,
) -> Result<kugou_music::KugouPlaybackUrl, String> {
    require_command_window(&window, "get_kugou_song_playback_url")?;
    tauri::async_runtime::spawn_blocking(move || {
        kugou_music::get_song_playback_url(
            &app,
            hash,
            hash_candidates,
            album_audio_id,
            audio_id,
            playback_quality,
        )
    })
    .await
    .map_err(|err| format!("酷狗播放链接获取任务失败：{err}"))?
}

#[tauri::command]
pub async fn get_kugou_song_quality_availability(
    app: AppHandle,
    window: WebviewWindow,
    hash: String,
    hash_candidates: Option<Vec<String>>,
    album_audio_id: Option<u64>,
) -> Result<kugou_music::KugouQualityAvailability, String> {
    require_command_window(&window, "get_kugou_song_quality_availability")?;
    tauri::async_runtime::spawn_blocking(move || {
        kugou_music::get_song_quality_availability(&app, hash, hash_candidates, album_audio_id)
    })
    .await
    .map_err(|err| format!("酷狗音质可用性预检任务失败：{err}"))?
}

#[tauri::command]
pub async fn get_kugou_playback_proxy_status(
    window: WebviewWindow,
    proxy_url: String,
) -> Result<kugou_music::KugouPlaybackProxyStatus, String> {
    require_command_window(&window, "get_kugou_playback_proxy_status")?;
    tauri::async_runtime::spawn_blocking(move || kugou_music::playback_proxy_status(proxy_url))
        .await
        .map_err(|err| format!("酷狗播放代理状态读取任务失败：{err}"))?
}

fn read_music_lyrics_next_to_file(
    parent: &Path,
    stem: &str,
    warnings: &mut Vec<String>,
) -> Result<Option<MusicLyricsResult>, String> {
    for extension in ["lrc", "txt"] {
        let lyrics_path = parent.join(format!("{stem}.{extension}"));
        if !lyrics_path.is_file() {
            continue;
        }

        let metadata =
            fs::metadata(&lyrics_path).map_err(|err| format!("无法读取歌词文件信息：{err}"))?;
        if metadata.len() > 512 * 1024 {
            warnings.push("歌词文件过大，已跳过读取".to_string());
            continue;
        }

        let bytes = fs::read(&lyrics_path).map_err(|err| format!("无法读取歌词文件：{err}"))?;
        let content = String::from_utf8_lossy(&bytes).to_string();
        if content.trim().is_empty() {
            warnings.push("歌词文件为空".to_string());
            continue;
        }

        return Ok(Some(MusicLyricsResult {
            content,
            source: extension.to_string(),
            warnings: warnings.clone(),
        }));
    }

    Ok(None)
}

fn collect_music_files(directory: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|err| format!("无法读取文件夹 {}：{err}", directory.to_string_lossy()))?;

    for entry in entries {
        let entry = entry.map_err(|err| format!("读取文件夹项目失败：{err}"))?;
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|err| format!("无法读取文件信息 {}：{err}", path.to_string_lossy()))?;

        if metadata.is_dir() {
            collect_music_files(&path, output)?;
        } else if metadata.is_file() && is_supported_music_file(&path) {
            output.push(path);
        }
    }

    Ok(())
}

fn is_supported_music_file(path: &Path) -> bool {
    const MUSIC_EXTENSIONS: &[&str] = &["mp3", "wav", "ogg", "flac", "m4a", "aac", "webm"];

    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            MUSIC_EXTENSIONS
                .iter()
                .any(|item| item.eq_ignore_ascii_case(extension))
        })
        .unwrap_or(false)
}

fn allow_music_library_assets(app: &AppHandle, state: &serde_json::Value) -> Result<(), String> {
    for path in app_data::music_library_track_paths(state) {
        if !path.is_file() || !is_supported_music_file(&path) {
            continue;
        }
        app.asset_protocol_scope()
            .allow_file(&path)
            .map_err(|err| format!("授权音乐播放文件失败：{err}"))?;
    }
    if let Some(path) = registered_music_wallpaper_path(state) {
        app.asset_protocol_scope()
            .allow_file(&path)
            .map_err(|err| format!("授权沉浸壁纸文件失败：{err}"))?;
    }
    Ok(())
}

fn registered_music_wallpaper_path(state: &serde_json::Value) -> Option<PathBuf> {
    let path = PathBuf::from(
        state
            .get("settings")?
            .get("immersiveWallpaperPath")?
            .as_str()?
            .trim(),
    );
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    let supported = matches!(
        extension.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "mp4" | "m4v" | "webm" | "mov" | "ogv" | "ogg"
    );
    (supported && path.is_file()).then_some(path)
}

fn registered_lyrics_candidates(
    library_state: &serde_json::Value,
    music_path: &Path,
    source_path: Option<&str>,
) -> Result<Vec<PathBuf>, String> {
    let tracks = library_state
        .get("tracks")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "音乐库中没有可校验的本机歌曲".to_string())?;
    let registered_track = tracks.iter().find(|track| {
        track
            .get("path")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|registered_path| same_path(Path::new(registered_path), music_path))
    });
    let Some(registered_track) = registered_track else {
        return Err("该音频文件未在后端音乐库登记，已拒绝读取歌词".to_string());
    };

    let mut candidates = vec![music_path.to_path_buf()];
    let Some(source_path) = source_path.map(str::trim).filter(|path| !path.is_empty()) else {
        return Ok(candidates);
    };
    let source_path = PathBuf::from(source_path);
    if same_path(&source_path, music_path) {
        return Ok(candidates);
    }
    if !source_path.is_file() || !is_supported_music_file(&source_path) {
        return Err("sourcePath 必须是已导入的受支持音频文件".to_string());
    }

    let registered_source_path = registered_track
        .get("sourcePath")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .ok_or_else(|| "音乐库未登记该歌曲的原始音频路径".to_string())?;
    if !same_path(Path::new(registered_source_path), &source_path) {
        return Err("sourcePath 与已登记歌曲不关联，已拒绝读取歌词".to_string());
    }

    candidates.push(source_path);
    Ok(candidates)
}

fn copy_music_file_to_storage(source: &Path, storage: &Path) -> Result<PathBuf, String> {
    let file_name = source
        .file_name()
        .and_then(|item| item.to_str())
        .ok_or_else(|| "无法识别音乐文件名".to_string())?;
    let file_stem = source
        .file_stem()
        .and_then(|item| item.to_str())
        .unwrap_or("music");
    let extension = source
        .extension()
        .and_then(|item| item.to_str())
        .unwrap_or("");

    for index in 0..10_000 {
        let candidate_name = if index == 0 {
            file_name.to_string()
        } else if extension.is_empty() {
            format!("{file_stem} ({index})")
        } else {
            format!("{file_stem} ({index}).{extension}")
        };
        let candidate = storage.join(candidate_name);

        if candidate.exists() {
            if same_path(source, &candidate) {
                return Ok(candidate);
            }
            continue;
        }

        fs::copy(source, &candidate).map_err(|err| {
            format!(
                "复制音乐文件 {} 到 {} 失败：{err}",
                source.to_string_lossy(),
                candidate.to_string_lossy()
            )
        })?;
        return Ok(candidate);
    }

    Err("音乐文件重名过多，无法生成唯一文件名".to_string())
}

fn same_path(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn clean_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| item.chars().take(240).collect())
}

fn read_music_title(tag: &Tag) -> Option<String> {
    clean_optional_text(tag.get_string(ItemKey::TrackTitle))
        .or_else(|| clean_optional_text(tag.title().as_deref()))
}

fn read_music_artist(tag: &Tag) -> Option<String> {
    clean_joined_texts(tag.get_strings(ItemKey::TrackArtist))
        .or_else(|| clean_joined_texts(tag.get_strings(ItemKey::TrackArtists)))
        .or_else(|| clean_optional_text(tag.artist().as_deref()))
        .or_else(|| clean_joined_texts(tag.get_strings(ItemKey::AlbumArtist)))
        .or_else(|| clean_joined_texts(tag.get_strings(ItemKey::AlbumArtists)))
}

fn read_music_album(tag: &Tag) -> Option<String> {
    clean_optional_text(tag.get_string(ItemKey::AlbumTitle))
        .or_else(|| clean_optional_text(tag.album().as_deref()))
}

fn read_music_cover_img_url(tag: Option<&Tag>, warnings: &mut Vec<String>) -> Option<String> {
    const MAX_METADATA_COVER_BYTES: usize = 1_000_000;

    let picture = tag.and_then(|tag| {
        tag.get_picture_type(PictureType::CoverFront)
            .or_else(|| tag.pictures().first())
    })?;
    let data = picture.data();
    if data.is_empty() {
        return None;
    }

    if data.len() > MAX_METADATA_COVER_BYTES {
        warnings.push("metadata 中的封面图片过大，已跳过封面读取".to_string());
        return None;
    }

    let mime = picture
        .mime_type()
        .map(|mime| mime.as_str())
        .or_else(|| infer_image_mime(data))?;

    Some(format!(
        "data:{mime};base64,{}",
        BASE64_STANDARD.encode(data)
    ))
}

fn infer_image_mime(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }
    if data.starts_with(b"\x89PNG\r\n\x1A\n") {
        return Some("image/png");
    }
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if data.starts_with(b"BM") {
        return Some("image/bmp");
    }

    None
}

fn clean_joined_texts<'a>(values: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let mut cleaned = Vec::new();
    for value in values {
        if let Some(item) = clean_optional_text(Some(value)) {
            if !cleaned.iter().any(|existing| existing == &item) {
                cleaned.push(item);
            }
        }
    }

    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.join(" / ").chars().take(240).collect())
    }
}

#[tauri::command]
pub async fn send_pet_chat_message(
    app: AppHandle,
    window: WebviewWindow,
    messages: Vec<PetChatMessageDraft>,
) -> Result<PetChatReply, String> {
    require_command_window(&window, "send_pet_chat_message")?;
    let request_owner = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        crate::ai_transport::with_scope(&request_owner, || {
            ai_chat::send_pet_chat_message(&app, messages)
        })
    })
    .await
    .map_err(|err| format!("宠物对话任务失败：{err}"))?
}

#[tauri::command]
pub async fn classify_music_intent(
    app: AppHandle,
    window: WebviewWindow,
    user_input: String,
    context: MusicIntentContext,
) -> Result<MusicIntentReply, String> {
    require_command_window(&window, "classify_music_intent")?;
    let request_owner = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        crate::ai_transport::with_scope(&request_owner, || {
            ai_chat::classify_music_intent(&app, user_input, context)
        })
    })
    .await
    .map_err(|err| format!("音乐意图识别任务失败：{err}"))?
}

#[tauri::command]
pub async fn send_pet_music_chat_message(
    app: AppHandle,
    window: WebviewWindow,
    messages: Vec<PetChatMessageDraft>,
    action_context: MusicChatActionContext,
) -> Result<PetChatReply, String> {
    require_command_window(&window, "send_pet_music_chat_message")?;
    let request_owner = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        crate::ai_transport::with_scope(&request_owner, || {
            ai_chat::send_pet_music_chat_message(&app, messages, action_context)
        })
    })
    .await
    .map_err(|err| format!("音乐对话回复任务失败：{err}"))?
}

#[tauri::command]
pub fn list_story_saves(app: AppHandle, window: WebviewWindow) -> Result<Vec<StorySave>, String> {
    require_command_window(&window, "list_story_saves")?;
    story_mode::list_story_saves(&app)
}

#[tauri::command]
pub fn get_story_save(
    app: AppHandle,
    window: WebviewWindow,
    story_id: String,
) -> Result<StorySave, String> {
    require_command_window(&window, "get_story_save")?;
    story_mode::get_story_save(&app, &story_id)
}

#[tauri::command]
pub async fn create_story(
    app: AppHandle,
    window: WebviewWindow,
    draft: StoryCreateDraft,
) -> Result<StoryTurnReply, String> {
    require_command_window(&window, "create_story")?;
    let request_owner = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        crate::ai_transport::with_scope(&request_owner, || story_mode::create_story(&app, draft))
    })
    .await
    .map_err(|err| format!("故事创建任务失败：{err}"))?
}

#[tauri::command]
pub async fn advance_story(
    app: AppHandle,
    window: WebviewWindow,
    story_id: String,
    user_input: String,
) -> Result<StoryTurnReply, String> {
    require_command_window(&window, "advance_story")?;
    let request_owner = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        crate::ai_transport::with_scope(&request_owner, || {
            story_mode::advance_story(&app, &story_id, &user_input)
        })
    })
    .await
    .map_err(|err| format!("故事推进任务失败：{err}"))?
}

#[tauri::command]
pub fn delete_story_save(
    app: AppHandle,
    window: WebviewWindow,
    story_id: String,
) -> Result<(), String> {
    require_command_window(&window, "delete_story_save")?;
    story_mode::delete_story_save(&app, &story_id)
}

#[tauri::command]
pub fn rename_story_save(
    app: AppHandle,
    window: WebviewWindow,
    story_id: String,
    title: String,
) -> Result<StorySave, String> {
    require_command_window(&window, "rename_story_save")?;
    story_mode::rename_story_save(&app, &story_id, &title)
}

#[tauri::command]
pub async fn test_ai_connection(
    window: WebviewWindow,
    settings: AiSettingsDraft,
) -> Result<AiConnectionTestResult, String> {
    require_command_window(&window, "test_ai_connection")?;
    let request_owner = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        crate::ai_transport::with_scope(&request_owner, || {
            ai_chat::test_ai_connection(normalize_ai_settings(settings))
        })
    })
    .await
    .map_err(|err| format!("AI 连接测试任务失败：{err}"))?
}

#[tauri::command]
pub fn get_translation_settings(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<TranslationSettingsView, String> {
    require_command_window(&window, "get_translation_settings")?;
    app_data::read_config(&app).map(|config| translation_api::view(&config))
}

#[tauri::command]
pub fn save_translation_settings(
    app: AppHandle,
    window: WebviewWindow,
    settings: TranslationSettingsDraft,
) -> Result<TranslationSettingsView, String> {
    require_command_window(&window, "save_translation_settings")?;
    let config = app_data::update_translation_config(&app, |config| {
        translation_api::apply_draft(config, settings)
    })?;
    let _ = app.emit("translation-config-changed", ());
    Ok(translation_api::view(&config))
}

#[tauri::command]
pub async fn test_translation_connection(
    app: AppHandle,
    window: WebviewWindow,
    settings: TranslationSettingsDraft,
) -> Result<AiConnectionTestResult, String> {
    require_command_window(&window, "test_translation_connection")?;
    let config = translation_api::draft_config(&app_data::read_config(&app)?, settings)?;
    let connection = translation_api::effective_settings(&config);
    translation_api::validate(&connection)?;
    let service = translation_api::summary(&config);
    let request_owner = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        crate::ai_transport::with_scope(&request_owner, || {
            ai_chat::test_ai_connection(connection)
                .map(|_| AiConnectionTestResult {
                    ok: true,
                    provider: service.provider.clone(),
                    model: service.model.clone(),
                    message: "连接成功，当前草稿未保存。".into(),
                })
                .map_err(|_| translation_api::safe_request_error(&service))
        })
    })
    .await
    .map_err(|_| "翻译连接测试执行失败。".to_string())?
}

#[tauri::command]
pub async fn translate_text(
    app: AppHandle,
    window: WebviewWindow,
    request: TranslationRequest,
) -> Result<TranslationReply, String> {
    require_command_window(&window, "translate_text")?;
    let config = app_data::read_config(&app)?;
    let request_owner = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        crate::ai_transport::with_scope(&request_owner, || ai_chat::translate_text(config, request))
    })
    .await
    .map_err(|err| format!("翻译任务失败：{err}"))?
}

#[tauri::command]
pub async fn translate_selected_text(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<TranslationReply, String> {
    require_command_window(&window, "translate_selected_text")?;
    let config = app_data::read_config(&app)?;
    let request_owner = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        crate::ai_transport::with_scope(&request_owner, || ai_chat::translate_selected_text(config))
    })
    .await
    .map_err(|err| format!("划选翻译任务失败：{err}"))?
}

#[tauri::command]
pub async fn test_wechat_clawbot(
    window: WebviewWindow,
    settings: WechatClawbotSettingsDraft,
    message: String,
) -> Result<wechat_clawbot::WechatClawbotSendResult, String> {
    require_command_window(&window, "test_wechat_clawbot")?;
    let settings = normalize_wechat_clawbot_settings(settings);
    tauri::async_runtime::spawn_blocking(move || wechat_clawbot::send_message(&settings, &message))
        .await
        .map_err(|err| format!("微信 ClawBot 测试任务失败：{err}"))?
}

#[tauri::command]
pub async fn simulate_wechat_clawbot_message(
    app: AppHandle,
    window: WebviewWindow,
    settings: WechatClawbotSettingsDraft,
    message: String,
    sender: String,
    session_id: String,
) -> Result<clawbot_bridge::ClawbotChatResponse, String> {
    require_command_window(&window, "simulate_wechat_clawbot_message")?;
    let settings = normalize_wechat_clawbot_settings(settings);
    tauri::async_runtime::spawn_blocking(move || {
        clawbot_bridge::simulate_chat(&app, &settings, message, sender, session_id)
    })
    .await
    .map_err(|err| format!("微信入站模拟任务失败：{err}"))?
}

#[tauri::command]
pub async fn send_wechat_clawbot_message(
    app: AppHandle,
    window: WebviewWindow,
    message: String,
) -> Result<wechat_clawbot::WechatClawbotSendResult, String> {
    require_command_window(&window, "send_wechat_clawbot_message")?;
    let settings = app_data::read_config(&app)?.wechat_clawbot;
    tauri::async_runtime::spawn_blocking(move || wechat_clawbot::send_message(&settings, &message))
        .await
        .map_err(|err| format!("微信 ClawBot 发送任务失败：{err}"))?
}

#[tauri::command]
pub fn get_codex_app_server_status(
    window: WebviewWindow,
    state: State<'_, codex_app_server::CodexAppServerState>,
) -> Result<codex_app_server::CodexStatusPayload, String> {
    require_command_window(&window, "get_codex_app_server_status")?;
    Ok(codex_app_server::get_status(&state))
}

#[tauri::command]
pub fn start_codex_app_server(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, codex_app_server::CodexAppServerState>,
) -> Result<codex_app_server::CodexStatusPayload, String> {
    require_command_window(&window, "start_codex_app_server")?;
    let settings = app_data::read_config(&app)?.codex_app_server;
    codex_app_server::start(app, &state, settings)
}

#[tauri::command]
pub fn stop_codex_app_server(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, codex_app_server::CodexAppServerState>,
) -> Result<codex_app_server::CodexStatusPayload, String> {
    require_command_window(&window, "stop_codex_app_server")?;
    codex_app_server::stop(app, &state)
}

#[tauri::command]
pub fn ack_codex_notifications(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, codex_app_server::CodexAppServerState>,
) -> Result<codex_app_server::CodexStatusPayload, String> {
    require_command_window(&window, "ack_codex_notifications")?;
    codex_app_server::ack_notifications(app, &state)
}

#[tauri::command]
pub fn start_codex_app_server_turn(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, codex_app_server::CodexAppServerState>,
    prompt: String,
    cwd: Option<String>,
) -> Result<codex_app_server::CodexStatusPayload, String> {
    require_command_window(&window, "start_codex_app_server_turn")?;
    let settings = app_data::read_config(&app)?.codex_app_server;
    codex_app_server::start_turn(app, &state, settings, prompt, cwd)
}

#[tauri::command]
pub fn open_codex_window(window: WebviewWindow) -> Result<(), String> {
    require_command_window(&window, "open_codex_window")?;
    focus_codex_window()
}

#[cfg(windows)]
fn focus_codex_window() -> Result<(), String> {
    use std::collections::HashSet;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HWND, INVALID_HANDLE_VALUE, LPARAM},
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindow, GetWindowTextLengthW, GetWindowThreadProcessId,
            IsWindowVisible, SetForegroundWindow, ShowWindowAsync, GW_OWNER, SW_RESTORE,
        },
    };

    struct CodexWindowSearch {
        hwnd: HWND,
        codex_process_ids: HashSet<u32>,
        last_error: Option<String>,
    }

    unsafe extern "system" fn enum_codex_windows(hwnd: HWND, lparam: LPARAM) -> i32 {
        let search = &mut *(lparam as *mut CodexWindowSearch);
        if !is_candidate_top_level_window(hwnd) {
            return 1;
        }

        let mut process_id = 0_u32;
        GetWindowThreadProcessId(hwnd, &mut process_id);
        if process_id == 0 {
            return 1;
        }

        if search.codex_process_ids.contains(&process_id) {
            search.hwnd = hwnd;
            return 0;
        }

        1
    }

    unsafe fn is_candidate_top_level_window(hwnd: HWND) -> bool {
        IsWindowVisible(hwnd) != 0
            && GetWindow(hwnd, GW_OWNER).is_null()
            && GetWindowTextLengthW(hwnd) > 0
    }

    unsafe fn codex_process_ids() -> Result<HashSet<u32>, String> {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Err("无法读取 Windows 进程列表。".to_string());
        }

        let mut ids = HashSet::new();
        let mut entry = std::mem::zeroed::<PROCESSENTRY32W>();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                if process_entry_name(&entry).eq_ignore_ascii_case("Codex.exe") {
                    ids.insert(entry.th32ProcessID);
                }

                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
        Ok(ids)
    }

    fn process_entry_name(entry: &PROCESSENTRY32W) -> String {
        let len = entry
            .szExeFile
            .iter()
            .position(|item| *item == 0)
            .unwrap_or(entry.szExeFile.len());
        String::from_utf16_lossy(&entry.szExeFile[..len])
    }

    let codex_process_ids = unsafe { codex_process_ids()? };
    if codex_process_ids.is_empty() {
        return Err("未找到已启动的 Codex 进程。".to_string());
    }

    let mut search = CodexWindowSearch {
        hwnd: std::ptr::null_mut(),
        codex_process_ids,
        last_error: None,
    };

    unsafe {
        EnumWindows(Some(enum_codex_windows), &mut search as *mut _ as LPARAM);
        if search.hwnd.is_null() {
            return Err(search
                .last_error
                .unwrap_or_else(|| "未找到已打开的 Codex 窗口。".to_string()));
        }

        ShowWindowAsync(search.hwnd, SW_RESTORE);
        if SetForegroundWindow(search.hwnd) == 0 {
            return Err("找到 Codex 窗口，但无法将它切到前台。".to_string());
        }
    }

    Ok(())
}

#[cfg(not(windows))]
fn focus_codex_window() -> Result<(), String> {
    Err("当前仅支持在 Windows 下聚焦已打开的 Codex 窗口。".to_string())
}

#[tauri::command]
pub fn list_pet_memories(app: AppHandle, window: WebviewWindow) -> Result<Vec<PetMemory>, String> {
    require_command_window(&window, "list_pet_memories")?;
    ai_memory::list_memories(&app)
}

#[tauri::command]
pub fn add_pet_memory(
    app: AppHandle,
    window: WebviewWindow,
    draft: PetMemoryDraft,
) -> Result<PetMemory, String> {
    require_command_window(&window, "add_pet_memory")?;
    ai_memory::add_memory(&app, draft)
}

#[tauri::command]
pub fn update_pet_memory(
    app: AppHandle,
    window: WebviewWindow,
    memory_id: u64,
    draft: PetMemoryDraft,
) -> Result<PetMemory, String> {
    require_command_window(&window, "update_pet_memory")?;
    ai_memory::update_memory_by_id(&app, memory_id, draft)
}

#[tauri::command]
pub fn delete_pet_memory(
    app: AppHandle,
    window: WebviewWindow,
    memory_id: u64,
) -> Result<(), String> {
    require_command_window(&window, "delete_pet_memory")?;
    ai_memory::delete_memory(&app, memory_id)
}

#[tauri::command]
pub fn clear_pet_memories(app: AppHandle, window: WebviewWindow) -> Result<(), String> {
    require_command_window(&window, "clear_pet_memories")?;
    ai_memory::clear_memories(&app)
}

#[tauri::command]
pub fn clear_pet_memory_messages(app: AppHandle, window: WebviewWindow) -> Result<(), String> {
    require_command_window(&window, "clear_pet_memory_messages")?;
    ai_memory::clear_messages(&app)
}

#[tauri::command]
pub fn import_pet_memory(
    app: AppHandle,
    window: WebviewWindow,
    path: String,
) -> Result<Vec<PetMemory>, String> {
    require_command_window(&window, "import_pet_memory")?;
    ai_memory::import_memory_file(&app, &path)
}

#[tauri::command]
pub fn export_pet_memory(
    app: AppHandle,
    window: WebviewWindow,
    path: String,
) -> Result<(), String> {
    require_command_window(&window, "export_pet_memory")?;
    ai_memory::export_memory_file(&app, &path)
}

#[tauri::command]
pub fn open_pet_memory_dir(app: AppHandle, window: WebviewWindow) -> Result<(), String> {
    require_command_window(&window, "open_pet_memory_dir")?;
    ai_memory::open_memory_dir(&app)
}

#[tauri::command]
pub fn show_pet(app: AppHandle) -> Result<(), String> {
    windowing::show_pet(&app)
}

#[tauri::command]
pub fn hide_pet(app: AppHandle) -> Result<(), String> {
    windowing::hide_pet(&app)
}

#[tauri::command]
pub fn check_for_update(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<updater::UpdateCheckResult, String> {
    require_command_window(&window, "check_for_update")?;
    Ok(updater::check_for_update(&app))
}

#[tauri::command]
pub fn open_update_page(window: WebviewWindow, url: Option<String>) -> Result<(), String> {
    require_command_window(&window, "open_update_page")?;
    updater::open_update_page(url)
}

#[tauri::command]
pub async fn quit_app(app: AppHandle) {
    let (sender, receiver) = std::sync::mpsc::channel();
    let listener_id = app.once("music-library-flush-completed", move |_| {
        let _ = sender.send(());
    });
    let flush_requested = app.get_webview_window("music").is_some()
        && app.emit("music-library-flush-requested", ()).is_ok();
    if flush_requested {
        let _ = tauri::async_runtime::spawn_blocking(move || {
            receiver.recv_timeout(std::time::Duration::from_secs(5))
        })
        .await;
    }
    app.unlisten(listener_id);
    app.exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_settings_are_only_authorized_for_translator() {
        for command in [
            "get_translation_settings",
            "save_translation_settings",
            "test_translation_connection",
        ] {
            assert!(is_command_allowed_for_window(command, "translator"));
            for label in [
                "drawer",
                "pet",
                "pet-chat",
                "music",
                "story",
                "pet-bubble",
                "unknown",
            ] {
                assert!(!is_command_allowed_for_window(command, label));
            }
        }
        assert!(!is_command_allowed_for_window("get_config", "translator"));
        assert!(!is_command_allowed_for_window(
            "save_drawer_preferences",
            "translator"
        ));
    }

    #[test]
    fn shortcut_consent_requires_non_empty_shortcut_and_explicit_approval() {
        let pending = normalize_shortcut_settings(ShortcutSettingsDraft::default());
        assert!(!pending.translate_selection.is_empty());
        assert!(!pending.translate_selection_consent);

        let enabled = normalize_shortcut_settings(ShortcutSettingsDraft {
            translate_selection_consent: true,
            ..ShortcutSettingsDraft::default()
        });
        assert!(enabled.translate_selection_consent);

        let disabled = normalize_shortcut_settings(ShortcutSettingsDraft {
            translate_selection: String::new(),
            translate_selection_consent: true,
            ..ShortcutSettingsDraft::default()
        });
        assert!(disabled.translate_selection.is_empty());
        assert!(!disabled.translate_selection_consent);
    }

    #[test]
    fn command_window_matrix_allows_each_feature_owner() {
        let allowed_cases = [
            ("reorder_apps", "drawer"),
            ("save_drawer_preferences", "drawer"),
            ("list_companions", "story"),
            ("get_current_companion", "pet-chat"),
            ("get_current_pet_skin", "pet"),
            ("send_pet_chat_message", "pet-chat"),
            ("create_story", "story"),
            ("translate_text", "translator"),
            ("translate_selected_text", "pet"),
            ("save_music_library_state", "music"),
            ("get_netease_song_playback_url", "music"),
            ("get_kugou_playlist_detail", "music"),
            ("get_codex_app_server_status", "pet-bubble"),
        ];

        for (command, label) in allowed_cases {
            assert!(
                is_command_allowed_for_window(command, label),
                "{label} should be allowed to invoke {command}"
            );
        }
    }

    fn shortcut_for_order_test(id: &str) -> PetApp {
        PetApp {
            id: id.to_string(),
            name: id.to_uppercase(),
            item_kind: "app".to_string(),
            path: format!("{id}.exe"),
            icon: None,
            category: "其他".to_string(),
            run_as_admin: false,
            tags: Vec::new(),
            favorite: false,
            auto_favorite: false,
            launch_count: 0,
            launch_history: Vec::new(),
            last_launch_at: None,
            created_at: "2026-08-01".to_string(),
        }
    }

    #[test]
    fn reorder_app_list_accepts_an_exact_permutation() {
        let apps = vec![shortcut_for_order_test("a"), shortcut_for_order_test("b")];
        let ids = vec!["b".to_string(), "a".to_string()];

        let reordered = reorder_app_list(apps, &ids).expect("valid order");

        assert_eq!(
            reordered
                .iter()
                .map(|app| app.id.as_str())
                .collect::<Vec<_>>(),
            vec!["b", "a"]
        );
    }

    #[test]
    fn reorder_app_list_rejects_missing_duplicate_and_unknown_ids() {
        let apps = || vec![shortcut_for_order_test("a"), shortcut_for_order_test("b")];

        assert!(reorder_app_list(apps(), &["a".to_string()]).is_err());
        assert!(reorder_app_list(apps(), &["a".to_string(), "a".to_string()]).is_err());
        assert!(reorder_app_list(apps(), &["a".to_string(), "x".to_string()]).is_err());
    }

    #[test]
    fn command_window_matrix_rejects_cross_window_privilege_escalation() {
        let rejected_cases = [
            ("save_drawer_preferences", "music"),
            ("upsert_companion", "pet-chat"),
            ("delete_companion_messages", "drawer"),
            ("create_story", "translator"),
            ("translate_text", "pet"),
            ("translate_selected_text", "translator"),
            ("save_music_library_state", "pet-chat"),
            ("create_netease_qr_login", "drawer"),
            ("start_codex_app_server_turn", "pet"),
            ("unknown_command", "drawer"),
        ];

        for (command, label) in rejected_cases {
            assert!(
                !is_command_allowed_for_window(command, label),
                "{label} should be rejected from invoking {command}"
            );
        }
    }

    #[test]
    fn shared_read_only_commands_do_not_grant_write_permissions() {
        assert!(is_command_allowed_for_window("list_companions", "drawer"));
        assert!(is_command_allowed_for_window("list_companions", "story"));
        assert!(!is_command_allowed_for_window("upsert_companion", "story"));

        assert!(is_command_allowed_for_window(
            "get_current_pet_skin",
            "pet-chat"
        ));
        assert!(!is_command_allowed_for_window("set_pet_skin", "pet-chat"));

        assert!(is_command_allowed_for_window(
            "get_codex_app_server_status",
            "pet"
        ));
        assert!(!is_command_allowed_for_window(
            "start_codex_app_server",
            "pet"
        ));
    }

    fn temp_music_test_dir(name: &str) -> PathBuf {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "pet_drawer_commands_{name}_{}_{}",
            std::process::id(),
            timestamp
        ));
        fs::create_dir_all(&dir).expect("create temp test dir");
        dir
    }

    #[test]
    fn public_config_removes_secrets_paths_and_private_profiles() {
        let mut config = PetDrawerConfig::default();
        config.pet.x = Some(123);
        config.pet.custom_image = Some("private-image-path".to_string());
        config.drawer.categories = vec!["private-category".to_string()];
        config.ai.enabled = true;
        config.translation.api_key = "private-translation-key".into();
        config.translation.base_url =
            "https://translation.example.test/private/path?token=private".into();
        config.ai.api_key = "private-api-key".to_string();
        config.ai.base_url = "https://ai.example.test/private/path?token=private".to_string();
        config.ai.system_prompt = "private-system-prompt".to_string();
        config.ai.active_profile_id = "private-profile".to_string();
        config.ai.profiles.push(AiConnectionProfile {
            id: "private-profile".to_string(),
            label: "private-label".to_string(),
            provider: "custom".to_string(),
            api_key: "private-profile-key".to_string(),
            base_url: "https://profile.example.test/private".to_string(),
            model: "private-model".to_string(),
        });
        config.wechat_clawbot.target = "private-target".to_string();
        config.wechat_clawbot.bridge_token = "private-bridge-token".to_string();
        config.codex_app_server.socket_path = "private-socket-path".to_string();

        let public = redact_public_config(config);
        let serialized = serde_json::to_string(&public).expect("serialize public config");

        assert!(public.ai.enabled);
        assert!(public.translation.api_key.is_empty());
        assert_eq!(
            public.translation.base_url,
            "https://translation.example.test"
        );
        assert_eq!(public.ai.base_url, "https://ai.example.test");
        assert!(public.ai.api_key.is_empty());
        assert!(public.ai.profiles.is_empty());
        assert!(public.companions.is_empty());
        assert!(!serialized.contains("private-"));
    }

    #[test]
    fn public_ai_origin_rejects_non_http_and_invalid_urls() {
        assert_eq!(
            public_ai_origin("http://127.0.0.1:11434/private?key=value"),
            "http://127.0.0.1:11434"
        );
        assert!(public_ai_origin("file:///private/config.json").is_empty());
        assert!(public_ai_origin("not a url").is_empty());
    }

    #[test]
    fn lyrics_candidates_require_registered_audio_relationship() {
        let dir = temp_music_test_dir("lyrics_relationship");
        let playback_path = dir.join("stored.mp3");
        let source_path = dir.join("original.flac");
        let unrelated_path = dir.join("unrelated.wav");
        let text_path = dir.join("private.txt");
        fs::write(&playback_path, b"audio").expect("write playback audio");
        fs::write(&source_path, b"audio").expect("write source audio");
        fs::write(&unrelated_path, b"audio").expect("write unrelated audio");
        fs::write(&text_path, b"private").expect("write private text");
        let state = serde_json::json!({
            "version": 1,
            "settings": {},
            "tracks": [{
                "path": playback_path.to_string_lossy(),
                "sourcePath": source_path.to_string_lossy()
            }],
            "playlists": []
        });

        let candidates = registered_lyrics_candidates(
            &state,
            &playback_path,
            Some(&source_path.to_string_lossy()),
        )
        .expect("registered source should be accepted");
        assert_eq!(candidates, vec![playback_path.clone(), source_path]);

        assert!(registered_lyrics_candidates(
            &state,
            &playback_path,
            Some(&unrelated_path.to_string_lossy())
        )
        .expect_err("unregistered audio should fail")
        .contains("不关联"));
        assert!(registered_lyrics_candidates(
            &state,
            &playback_path,
            Some(&text_path.to_string_lossy())
        )
        .expect_err("text source should fail")
        .contains("音频文件"));

        fs::remove_dir_all(dir).expect("remove temp test dir");
    }

    #[test]
    fn persisted_wallpaper_asset_requires_supported_existing_media() {
        let dir = temp_music_test_dir("wallpaper_asset");
        let image_path = dir.join("wallpaper.png");
        let text_path = dir.join("private.txt");
        fs::write(&image_path, b"image").expect("write wallpaper image");
        fs::write(&text_path, b"private").expect("write private text");

        let image_state = serde_json::json!({
            "settings": { "immersiveWallpaperPath": image_path.to_string_lossy() }
        });
        let text_state = serde_json::json!({
            "settings": { "immersiveWallpaperPath": text_path.to_string_lossy() }
        });

        assert_eq!(
            registered_music_wallpaper_path(&image_state),
            Some(image_path)
        );
        assert_eq!(registered_music_wallpaper_path(&text_state), None);

        fs::remove_dir_all(dir).expect("remove temp test dir");
    }

    #[test]
    fn lyrics_candidates_reject_unregistered_main_track() {
        let dir = temp_music_test_dir("lyrics_unregistered_main");
        let registered_path = dir.join("registered.mp3");
        let requested_path = dir.join("requested.mp3");
        fs::write(&registered_path, b"audio").expect("write registered audio");
        fs::write(&requested_path, b"audio").expect("write requested audio");
        let state = serde_json::json!({
            "version": 1,
            "settings": {},
            "tracks": [{ "path": registered_path.to_string_lossy() }],
            "playlists": []
        });

        assert!(registered_lyrics_candidates(&state, &requested_path, None)
            .expect_err("unregistered main track should fail")
            .contains("未在后端音乐库登记"));

        fs::remove_dir_all(dir).expect("remove temp test dir");
    }
}
