use lofty::{
    prelude::{Accessor, AudioFile, ItemKey, TaggedFileExt},
    tag::Tag,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::AppHandle;

use crate::{
    ai_chat::{
        self, AiConnectionTestResult, MusicChatActionContext, MusicIntentContext, MusicIntentReply,
        PetChatMessageDraft, PetChatReply,
    },
    ai_memory::{self, PetMemory, PetMemoryDraft, PetMemoryMessage},
    app_data::{
        self, AiConnectionProfile, AiSettings, AppDraft, CodexAppServerSettings, Companion,
        CompanionDraft, PetAnimationSet, PetApp, PetDrawerConfig, PetPosition, PetSkinPackageDraft,
        PetSkinSummary, ShortcutSettings, StorageSettings, WechatClawbotSettings,
    },
    clawbot_bridge, codex_app_server,
    favorability::{self, CompanionStatus, FavorabilityLog},
    launcher, netease_music, startup,
    story_mode::{self, StoryCreateDraft, StorySave, StoryTurnReply},
    updater, wechat_clawbot, windowing,
};
use tauri::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawerPreferencesDraft {
    pub categories: Vec<String>,
    pub quick_search_tags: Vec<String>,
    pub tag_display_mode: String,
    #[serde(default = "default_drawer_theme")]
    pub theme: String,
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
pub struct MusicMetadataResult {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
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
pub fn get_apps(app: AppHandle) -> Result<Vec<PetApp>, String> {
    app_data::read_apps(&app)
}

#[tauri::command]
pub fn upsert_app(app: AppHandle, draft: AppDraft) -> Result<PetApp, String> {
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
pub fn delete_app(app: AppHandle, app_id: String) -> Result<(), String> {
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
    app_id: String,
    run_as_admin: bool,
) -> Result<PetApp, String> {
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
pub fn launch_app(app: AppHandle, app_id: String) -> Result<PetApp, String> {
    launcher::launch_app(&app, &app_id)
}

#[tauri::command]
pub fn open_app_dir(app: AppHandle, app_id: String) -> Result<(), String> {
    launcher::open_app_dir(&app, &app_id)
}

#[tauri::command]
pub fn get_config(app: AppHandle) -> Result<PetDrawerConfig, String> {
    let mut config = app_data::read_config(&app)?;
    if let Ok(start_on_boot) = startup::is_start_on_boot_enabled() {
        if start_on_boot {
            config.system.start_on_boot = true;
        }
    }

    Ok(config)
}

#[tauri::command]
pub fn list_companions(app: AppHandle) -> Result<Vec<Companion>, String> {
    ai_memory::list_companions(&app)
}

#[tauri::command]
pub fn get_current_companion(app: AppHandle) -> Result<Companion, String> {
    ai_memory::current_companion(&app)
}

#[tauri::command]
pub fn upsert_companion(app: AppHandle, draft: CompanionDraft) -> Result<Companion, String> {
    ai_memory::upsert_companion(&app, draft)
}

#[tauri::command]
pub fn import_companion_card(app: AppHandle, path: String) -> Result<Companion, String> {
    ai_memory::import_companion_card(&app, &path)
}

#[tauri::command]
pub fn export_companion_card(
    app: AppHandle,
    companion_id: String,
    path: String,
) -> Result<(), String> {
    ai_memory::export_companion_card(&app, &companion_id, &path)
}

#[tauri::command]
pub fn switch_companion(app: AppHandle, companion_id: String) -> Result<Companion, String> {
    let mut companion = ai_memory::switch_companion(&app, &companion_id)?;
    if app_data::set_current_pet_skin(&app, &companion.skin_id).is_err() {
        app_data::set_current_pet_skin(&app, "default")?;
        ai_memory::set_current_companion_skin(&app, "default")?;
        companion.skin_id = "default".to_string();
    }
    Ok(companion)
}

#[tauri::command]
pub fn delete_companion(app: AppHandle, companion_id: String) -> Result<Companion, String> {
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
pub fn get_current_companion_status(app: AppHandle) -> Result<CompanionStatus, String> {
    favorability::get_current_companion_status(&app)
}

#[tauri::command]
pub fn set_current_companion_favorability_enabled(
    app: AppHandle,
    enabled: bool,
) -> Result<CompanionStatus, String> {
    favorability::set_current_enabled(&app, enabled)
}

#[tauri::command]
pub fn set_current_companion_favorability(
    app: AppHandle,
    value: i32,
) -> Result<CompanionStatus, String> {
    favorability::set_current_favorability(&app, value)
}

#[tauri::command]
pub fn reset_current_companion_favorability(app: AppHandle) -> Result<CompanionStatus, String> {
    favorability::reset_current_favorability(&app)
}

#[tauri::command]
pub fn list_current_companion_favorability_logs(
    app: AppHandle,
) -> Result<Vec<FavorabilityLog>, String> {
    favorability::list_current_logs(&app, 100)
}

#[tauri::command]
pub fn get_companion_messages(app: AppHandle) -> Result<Vec<PetMemoryMessage>, String> {
    let companion_id = ai_memory::current_companion_id(&app)?;
    ai_memory::recent_messages(&app, &companion_id, 200)
}

#[tauri::command]
pub fn delete_companion_messages(app: AppHandle, message_ids: Vec<u64>) -> Result<usize, String> {
    ai_memory::delete_messages(&app, message_ids)
}

#[tauri::command]
pub fn get_runtime_info(app: AppHandle) -> Result<RuntimeInfo, String> {
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
pub fn get_storage_settings(app: AppHandle) -> Result<StorageSettings, String> {
    app_data::read_storage_settings(&app)
}

#[tauri::command]
pub fn save_storage_settings(
    app: AppHandle,
    settings: StorageSettings,
) -> Result<StorageSettings, String> {
    app_data::save_storage_settings(&app, &settings)
}

#[tauri::command]
pub fn set_quick_search_tags(app: AppHandle, tags: Vec<String>) -> Result<Vec<String>, String> {
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
    preferences: DrawerPreferencesDraft,
) -> Result<PetDrawerConfig, String> {
    let categories = normalize_unique_list(preferences.categories, 30);
    let quick_search_tags = normalize_unique_list(preferences.quick_search_tags, 20);
    let tag_display_mode = if preferences.tag_display_mode == "detailed" {
        "detailed".to_string()
    } else {
        "compact".to_string()
    };
    let theme = normalize_drawer_theme(preferences.theme);
    let pet_size = app_data::normalize_pet_size(preferences.pet_size);

    let mut config = app_data::read_config(&app)?;
    startup::set_start_on_boot(preferences.start_on_boot)?;

    config.drawer.categories = ensure_core_categories(categories);
    config.drawer.quick_search_tags = quick_search_tags;
    config.drawer.tag_display_mode = tag_display_mode;
    config.drawer.theme = theme;
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
    app_data::write_config(&app, &config)?;
    clawbot_bridge::restart_bridge_server(&app)?;

    windowing::set_pet_size(&app, pet_size)?;
    windowing::set_pet_always_on_top(&app, preferences.pet_always_on_top)?;
    windowing::set_drawer_always_on_top(&app, preferences.drawer_always_on_top)?;

    Ok(config)
}

#[tauri::command]
pub fn save_chat_display_preferences(
    app: AppHandle,
    chat_typewriter_enabled: bool,
    chat_narration_enabled: bool,
    chat_music_link_enabled: bool,
) -> Result<PetDrawerConfig, String> {
    let mut config = app_data::read_config(&app)?;
    config.drawer.chat_typewriter_enabled = chat_typewriter_enabled;
    config.drawer.chat_narration_enabled = chat_narration_enabled;
    config.drawer.chat_music_link_enabled = chat_music_link_enabled;
    app_data::write_config(&app, &config)?;
    Ok(config)
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
    ShortcutSettings {
        toggle_drawer: {
            let value = settings.toggle_drawer.trim();
            if value.is_empty() {
                default_toggle_drawer_shortcut()
            } else {
                value.to_string()
            }
        },
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
pub fn list_pet_skins(app: AppHandle) -> Result<Vec<PetSkinSummary>, String> {
    app_data::list_pet_skins(&app)
}

#[tauri::command]
pub fn get_current_pet_skin(app: AppHandle) -> Result<PetSkinSummary, String> {
    app_data::get_current_pet_skin(&app)
}

#[tauri::command]
pub fn set_pet_skin(app: AppHandle, skin_id: String) -> Result<PetSkinSummary, String> {
    let skin = app_data::set_current_pet_skin(&app, &skin_id)?;
    ai_memory::set_current_companion_skin(&app, &skin.id)?;
    Ok(skin)
}

#[tauri::command]
pub fn read_pet_skin_package(path: String) -> Result<PetSkinPackageDraft, String> {
    app_data::read_pet_skin_package(&path)
}

#[tauri::command]
pub fn import_pet_skin(
    app: AppHandle,
    name: String,
    animations: PetAnimationSet,
) -> Result<PetSkinSummary, String> {
    let skin = app_data::import_pet_skin(&app, &name, animations)?;
    ai_memory::set_current_companion_skin(&app, &skin.id)?;
    Ok(skin)
}

#[tauri::command]
pub fn update_pet_skin(
    app: AppHandle,
    skin_id: String,
    name: String,
    animations: PetAnimationSet,
    cleared_states: Vec<String>,
) -> Result<PetSkinSummary, String> {
    app_data::update_pet_skin(&app, &skin_id, &name, animations, cleared_states)
}

#[tauri::command]
pub fn delete_pet_skin(app: AppHandle, skin_id: String) -> Result<PetSkinSummary, String> {
    let skin = app_data::delete_pet_skin(&app, &skin_id)?;
    ai_memory::set_current_companion_skin(&app, &skin.id)?;
    Ok(skin)
}

#[tauri::command]
pub fn import_pet_image(app: AppHandle, path: String) -> Result<String, String> {
    let relative_path = app_data::import_image(&app, &path, "pets", "pet")?;
    let mut config = app_data::read_config(&app)?;
    config.pet.current_skin = "default".to_string();
    config.pet.custom_image = Some(relative_path.clone());
    app_data::write_config(&app, &config)?;
    Ok(relative_path)
}

#[tauri::command]
pub fn reset_pet_image(app: AppHandle) -> Result<(), String> {
    app_data::set_current_pet_skin(&app, "default")?;
    ai_memory::set_current_companion_skin(&app, "default")
}

#[tauri::command]
pub fn import_app_icon(app: AppHandle, path: String) -> Result<String, String> {
    app_data::import_image(&app, &path, "icons", "icon")
}

#[tauri::command]
pub fn import_executable_icon(app: AppHandle, path: String) -> Result<String, String> {
    app_data::import_executable_icon(&app, &path)
}

#[tauri::command]
pub fn get_image_data_url(app: AppHandle, relative_path: String) -> Result<String, String> {
    app_data::read_image_data_url(&app, &relative_path)
}

#[tauri::command]
pub fn save_pet_position(app: AppHandle, position: PetPosition) -> Result<(), String> {
    windowing::save_pet_position(&app, position.x, position.y)
}

#[tauri::command]
pub fn is_primary_mouse_button_pressed() -> bool {
    primary_mouse_button_pressed()
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
pub fn toggle_drawer(app: AppHandle) -> Result<(), String> {
    windowing::toggle_drawer(&app)
}

#[tauri::command]
pub fn show_drawer(app: AppHandle) -> Result<(), String> {
    windowing::show_drawer(&app)
}

#[tauri::command]
pub fn hide_drawer(app: AppHandle) -> Result<(), String> {
    windowing::hide_drawer(&app)
}

#[tauri::command]
pub fn show_pet_menu(app: AppHandle, x: i32, y: i32) -> Result<(), String> {
    windowing::show_pet_menu(&app, x, y)
}

#[tauri::command]
pub fn hide_pet_menu(app: AppHandle) -> Result<(), String> {
    windowing::hide_pet_menu(&app)
}

#[tauri::command]
pub fn show_pet_bubble(app: AppHandle, payload: windowing::PetBubblePayload) -> Result<(), String> {
    windowing::show_pet_bubble(&app, payload)
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
pub fn show_pet_chat(app: AppHandle) -> Result<(), String> {
    windowing::show_pet_chat(&app)
}

#[tauri::command]
pub fn hide_pet_chat(app: AppHandle) -> Result<(), String> {
    windowing::hide_pet_chat(&app)
}

#[tauri::command]
pub fn show_story(app: AppHandle) -> Result<(), String> {
    windowing::show_story(&app)
}

#[tauri::command]
pub fn hide_story(app: AppHandle) -> Result<(), String> {
    windowing::hide_story(&app)
}

#[tauri::command]
pub fn show_music_player(app: AppHandle) -> Result<(), String> {
    windowing::show_music_player(&app)
}

#[tauri::command]
pub fn hide_music_player(app: AppHandle) -> Result<(), String> {
    windowing::hide_music_player(&app)
}

#[tauri::command]
pub fn list_music_files_in_directory(directory: String) -> Result<Vec<String>, String> {
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
    paths: Vec<String>,
    storage_dir: String,
) -> Result<Vec<MusicImportItem>, String> {
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
pub fn read_music_metadata(path: String) -> Result<MusicMetadataResult, String> {
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
        duration,
        source: "metadata".to_string(),
        confidence,
        warnings,
    })
}

#[tauri::command]
pub fn read_music_lyrics(
    path: String,
    source_path: Option<String>,
) -> Result<Option<MusicLyricsResult>, String> {
    let music_path = PathBuf::from(path.trim());
    if !music_path.is_file() || !is_supported_music_file(&music_path) {
        return Err("请选择有效的音频文件".to_string());
    }

    let mut warnings = Vec::new();
    let mut candidates = vec![music_path.clone()];
    if let Some(source) = source_path {
        let source = PathBuf::from(source.trim());
        if source.is_file() && source != music_path {
            candidates.push(source);
        }
    }

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
) -> Result<netease_music::NeteaseQrLogin, String> {
    tauri::async_runtime::spawn_blocking(move || netease_music::create_qr_login(&app))
        .await
        .map_err(|err| format!("网易云二维码创建任务失败：{err}"))?
}

#[tauri::command]
pub async fn check_netease_qr_login(
    app: AppHandle,
    key: String,
) -> Result<netease_music::NeteaseQrCheckResult, String> {
    tauri::async_runtime::spawn_blocking(move || netease_music::check_qr_login(&app, key))
        .await
        .map_err(|err| format!("网易云登录检查任务失败：{err}"))?
}

#[tauri::command]
pub async fn get_netease_login_status(
    app: AppHandle,
) -> Result<netease_music::NeteaseLoginStatus, String> {
    tauri::async_runtime::spawn_blocking(move || netease_music::login_status(&app))
        .await
        .map_err(|err| format!("网易云登录状态读取任务失败：{err}"))?
}

#[tauri::command]
pub async fn clear_netease_login(
    app: AppHandle,
) -> Result<netease_music::NeteaseLoginStatus, String> {
    tauri::async_runtime::spawn_blocking(move || netease_music::clear_login(&app))
        .await
        .map_err(|err| format!("网易云登录清除任务失败：{err}"))?
}

#[tauri::command]
pub async fn list_netease_playlists(
    app: AppHandle,
) -> Result<Vec<netease_music::NeteasePlaylistSummary>, String> {
    tauri::async_runtime::spawn_blocking(move || netease_music::list_playlists(&app))
        .await
        .map_err(|err| format!("网易云歌单读取任务失败：{err}"))?
}

#[tauri::command]
pub async fn get_netease_playlist_detail(
    app: AppHandle,
    playlist_id: u64,
) -> Result<netease_music::NeteasePlaylistDetail, String> {
    tauri::async_runtime::spawn_blocking(move || netease_music::playlist_detail(&app, playlist_id))
        .await
        .map_err(|err| format!("网易云歌单详情读取任务失败：{err}"))?
}

#[tauri::command]
pub async fn read_netease_lyrics(
    app: AppHandle,
    song_id: u64,
) -> Result<netease_music::NeteaseLyricsResult, String> {
    tauri::async_runtime::spawn_blocking(move || netease_music::song_lyrics(&app, song_id))
        .await
        .map_err(|err| format!("网易云歌词读取任务失败：{err}"))?
}

#[tauri::command]
pub async fn get_netease_song_playback_url(
    app: AppHandle,
    song_id: u64,
    level: Option<String>,
) -> Result<netease_music::NeteasePlaybackUrl, String> {
    tauri::async_runtime::spawn_blocking(move || {
        netease_music::song_playback_url(&app, song_id, level)
    })
    .await
    .map_err(|err| format!("网易云播放链接获取任务失败：{err}"))?
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
    messages: Vec<PetChatMessageDraft>,
) -> Result<PetChatReply, String> {
    tauri::async_runtime::spawn_blocking(move || ai_chat::send_pet_chat_message(&app, messages))
        .await
        .map_err(|err| format!("宠物对话任务失败：{err}"))?
}

#[tauri::command]
pub async fn classify_music_intent(
    app: AppHandle,
    user_input: String,
    context: MusicIntentContext,
) -> Result<MusicIntentReply, String> {
    tauri::async_runtime::spawn_blocking(move || {
        ai_chat::classify_music_intent(&app, user_input, context)
    })
    .await
    .map_err(|err| format!("音乐意图识别任务失败：{err}"))?
}

#[tauri::command]
pub async fn send_pet_music_chat_message(
    app: AppHandle,
    messages: Vec<PetChatMessageDraft>,
    action_context: MusicChatActionContext,
) -> Result<PetChatReply, String> {
    tauri::async_runtime::spawn_blocking(move || {
        ai_chat::send_pet_music_chat_message(&app, messages, action_context)
    })
    .await
    .map_err(|err| format!("音乐对话回复任务失败：{err}"))?
}

#[tauri::command]
pub fn list_story_saves(app: AppHandle) -> Result<Vec<StorySave>, String> {
    story_mode::list_story_saves(&app)
}

#[tauri::command]
pub fn get_story_save(app: AppHandle, story_id: String) -> Result<StorySave, String> {
    story_mode::get_story_save(&app, &story_id)
}

#[tauri::command]
pub async fn create_story(
    app: AppHandle,
    draft: StoryCreateDraft,
) -> Result<StoryTurnReply, String> {
    tauri::async_runtime::spawn_blocking(move || story_mode::create_story(&app, draft))
        .await
        .map_err(|err| format!("故事创建任务失败：{err}"))?
}

#[tauri::command]
pub async fn advance_story(
    app: AppHandle,
    story_id: String,
    user_input: String,
) -> Result<StoryTurnReply, String> {
    tauri::async_runtime::spawn_blocking(move || {
        story_mode::advance_story(&app, &story_id, &user_input)
    })
    .await
    .map_err(|err| format!("故事推进任务失败：{err}"))?
}

#[tauri::command]
pub fn delete_story_save(app: AppHandle, story_id: String) -> Result<(), String> {
    story_mode::delete_story_save(&app, &story_id)
}

#[tauri::command]
pub fn rename_story_save(
    app: AppHandle,
    story_id: String,
    title: String,
) -> Result<StorySave, String> {
    story_mode::rename_story_save(&app, &story_id, &title)
}

#[tauri::command]
pub async fn test_ai_connection(
    settings: AiSettingsDraft,
) -> Result<AiConnectionTestResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        ai_chat::test_ai_connection(normalize_ai_settings(settings))
    })
    .await
    .map_err(|err| format!("AI 连接测试任务失败：{err}"))?
}

#[tauri::command]
pub async fn test_wechat_clawbot(
    settings: WechatClawbotSettingsDraft,
    message: String,
) -> Result<wechat_clawbot::WechatClawbotSendResult, String> {
    let settings = normalize_wechat_clawbot_settings(settings);
    tauri::async_runtime::spawn_blocking(move || wechat_clawbot::send_message(&settings, &message))
        .await
        .map_err(|err| format!("微信 ClawBot 测试任务失败：{err}"))?
}

#[tauri::command]
pub async fn simulate_wechat_clawbot_message(
    app: AppHandle,
    settings: WechatClawbotSettingsDraft,
    message: String,
    sender: String,
    session_id: String,
) -> Result<clawbot_bridge::ClawbotChatResponse, String> {
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
    message: String,
) -> Result<wechat_clawbot::WechatClawbotSendResult, String> {
    let settings = app_data::read_config(&app)?.wechat_clawbot;
    tauri::async_runtime::spawn_blocking(move || wechat_clawbot::send_message(&settings, &message))
        .await
        .map_err(|err| format!("微信 ClawBot 发送任务失败：{err}"))?
}

#[tauri::command]
pub fn get_codex_app_server_status(
    state: State<'_, codex_app_server::CodexAppServerState>,
) -> codex_app_server::CodexStatusPayload {
    codex_app_server::get_status(&state)
}

#[tauri::command]
pub fn start_codex_app_server(
    app: AppHandle,
    state: State<'_, codex_app_server::CodexAppServerState>,
) -> Result<codex_app_server::CodexStatusPayload, String> {
    let settings = app_data::read_config(&app)?.codex_app_server;
    codex_app_server::start(app, &state, settings)
}

#[tauri::command]
pub fn stop_codex_app_server(
    app: AppHandle,
    state: State<'_, codex_app_server::CodexAppServerState>,
) -> Result<codex_app_server::CodexStatusPayload, String> {
    codex_app_server::stop(app, &state)
}

#[tauri::command]
pub fn ack_codex_notifications(
    app: AppHandle,
    state: State<'_, codex_app_server::CodexAppServerState>,
) -> Result<codex_app_server::CodexStatusPayload, String> {
    codex_app_server::ack_notifications(app, &state)
}

#[tauri::command]
pub fn start_codex_app_server_turn(
    app: AppHandle,
    state: State<'_, codex_app_server::CodexAppServerState>,
    prompt: String,
    cwd: Option<String>,
) -> Result<codex_app_server::CodexStatusPayload, String> {
    let settings = app_data::read_config(&app)?.codex_app_server;
    codex_app_server::start_turn(app, &state, settings, prompt, cwd)
}

#[tauri::command]
pub fn open_codex_window() -> Result<(), String> {
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
pub fn list_pet_memories(app: AppHandle) -> Result<Vec<PetMemory>, String> {
    ai_memory::list_memories(&app)
}

#[tauri::command]
pub fn add_pet_memory(app: AppHandle, draft: PetMemoryDraft) -> Result<PetMemory, String> {
    ai_memory::add_memory(&app, draft)
}

#[tauri::command]
pub fn update_pet_memory(
    app: AppHandle,
    memory_id: u64,
    draft: PetMemoryDraft,
) -> Result<PetMemory, String> {
    ai_memory::update_memory_by_id(&app, memory_id, draft)
}

#[tauri::command]
pub fn delete_pet_memory(app: AppHandle, memory_id: u64) -> Result<(), String> {
    ai_memory::delete_memory(&app, memory_id)
}

#[tauri::command]
pub fn clear_pet_memories(app: AppHandle) -> Result<(), String> {
    ai_memory::clear_memories(&app)
}

#[tauri::command]
pub fn clear_pet_memory_messages(app: AppHandle) -> Result<(), String> {
    ai_memory::clear_messages(&app)
}

#[tauri::command]
pub fn import_pet_memory(app: AppHandle, path: String) -> Result<Vec<PetMemory>, String> {
    ai_memory::import_memory_file(&app, &path)
}

#[tauri::command]
pub fn export_pet_memory(app: AppHandle, path: String) -> Result<(), String> {
    ai_memory::export_memory_file(&app, &path)
}

#[tauri::command]
pub fn open_pet_memory_dir(app: AppHandle) -> Result<(), String> {
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
pub fn check_for_update(app: AppHandle) -> updater::UpdateCheckResult {
    updater::check_for_update(&app)
}

#[tauri::command]
pub fn open_update_page(url: Option<String>) -> Result<(), String> {
    updater::open_update_page(url)
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}
