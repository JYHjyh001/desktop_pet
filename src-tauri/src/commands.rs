use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::{
    ai_chat::{self, AiConnectionTestResult, PetChatMessageDraft, PetChatReply},
    ai_memory::{self, PetMemory, PetMemoryDraft, PetMemoryMessage},
    app_data::{
        self, AiConnectionProfile, AiSettings, AppDraft, Companion, CompanionDraft,
        PetAnimationSet, PetApp, PetDrawerConfig, PetPosition, PetSkinSummary, StorageSettings,
        WechatClawbotSettings,
    },
    clawbot_bridge,
    favorability::{self, CompanionStatus, FavorabilityLog},
    launcher, startup,
    story_mode::{self, StoryCreateDraft, StorySave, StoryTurnReply},
    updater, wechat_clawbot, windowing,
};

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
    #[serde(default = "default_pet_size")]
    pub pet_size: u32,
    pub pet_always_on_top: bool,
    pub drawer_always_on_top: bool,
    #[serde(default)]
    pub start_on_boot: bool,
    #[serde(default = "default_true")]
    pub auto_favorite_enabled: bool,
    #[serde(default)]
    pub ai: AiSettingsDraft,
    #[serde(default)]
    pub wechat_clawbot: WechatClawbotSettingsDraft,
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
    config.drawer.always_on_top = preferences.drawer_always_on_top;
    config.pet.size = pet_size;
    config.pet.always_on_top = preferences.pet_always_on_top;
    config.system.start_on_boot = preferences.start_on_boot;
    config.system.auto_favorite_enabled = preferences.auto_favorite_enabled;
    config.ai = normalize_ai_settings(preferences.ai);
    config.wechat_clawbot = normalize_wechat_clawbot_settings(preferences.wechat_clawbot);
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
) -> Result<PetDrawerConfig, String> {
    let mut config = app_data::read_config(&app)?;
    config.drawer.chat_typewriter_enabled = chat_typewriter_enabled;
    config.drawer.chat_narration_enabled = chat_narration_enabled;
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
pub async fn send_pet_chat_message(
    app: AppHandle,
    messages: Vec<PetChatMessageDraft>,
) -> Result<PetChatReply, String> {
    tauri::async_runtime::spawn_blocking(move || ai_chat::send_pet_chat_message(&app, messages))
        .await
        .map_err(|err| format!("宠物对话任务失败：{err}"))?
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
    tauri::async_runtime::spawn_blocking(move || {
        wechat_clawbot::send_message(&settings, &message)
    })
    .await
    .map_err(|err| format!("微信 ClawBot 测试任务失败：{err}"))?
}

#[tauri::command]
pub async fn send_wechat_clawbot_message(
    app: AppHandle,
    message: String,
) -> Result<wechat_clawbot::WechatClawbotSendResult, String> {
    let settings = app_data::read_config(&app)?.wechat_clawbot;
    tauri::async_runtime::spawn_blocking(move || {
        wechat_clawbot::send_message(&settings, &message)
    })
    .await
    .map_err(|err| format!("微信 ClawBot 发送任务失败：{err}"))?
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
