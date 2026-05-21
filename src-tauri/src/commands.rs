use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::{
    app_data::{
        self, AiSettings, AppDraft, PetAnimationSet, PetApp, PetDrawerConfig, PetPosition,
        PetSkinSummary,
    },
    ai_chat::{self, PetChatMessageDraft, PetChatReply},
    launcher, updater, windowing,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawerPreferencesDraft {
    pub categories: Vec<String>,
    pub quick_search_tags: Vec<String>,
    pub tag_display_mode: String,
    pub pet_always_on_top: bool,
    pub drawer_always_on_top: bool,
    #[serde(default)]
    pub ai: AiSettingsDraft,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettingsDraft {
    pub enabled: bool,
    pub provider: String,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub system_prompt: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

impl Default for AiSettingsDraft {
    fn default() -> Self {
        let settings = AiSettings::default();
        Self {
            enabled: settings.enabled,
            provider: settings.provider,
            api_key: settings.api_key,
            base_url: settings.base_url,
            model: settings.model,
            system_prompt: settings.system_prompt,
            temperature: settings.temperature,
            max_tokens: settings.max_tokens,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInfo {
    version: String,
    executable_path: String,
    data_dir: String,
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
        "folder" | "website" => value.to_string(),
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
    app_data::read_config(&app)
}

#[tauri::command]
pub fn get_runtime_info(app: AppHandle) -> Result<RuntimeInfo, String> {
    let executable_path = std::env::current_exe()
        .map_err(|err| format!("无法获取当前程序路径：{err}"))?
        .to_string_lossy()
        .to_string();
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|err| format!("无法获取应用数据目录：{err}"))?
        .to_string_lossy()
        .to_string();

    Ok(RuntimeInfo {
        version: app.package_info().version.to_string(),
        executable_path,
        data_dir,
    })
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

    let mut config = app_data::read_config(&app)?;
    config.drawer.categories = ensure_core_categories(categories);
    config.drawer.quick_search_tags = quick_search_tags;
    config.drawer.tag_display_mode = tag_display_mode;
    config.drawer.always_on_top = preferences.drawer_always_on_top;
    config.pet.always_on_top = preferences.pet_always_on_top;
    config.ai = normalize_ai_settings(preferences.ai);
    app_data::write_config(&app, &config)?;

    windowing::set_pet_always_on_top(&app, preferences.pet_always_on_top)?;
    windowing::set_drawer_always_on_top(&app, preferences.drawer_always_on_top)?;

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
    AiSettings {
        enabled: settings.enabled,
        provider: normalize_ai_provider(settings.provider),
        api_key: settings.api_key.trim().to_string(),
        base_url: normalize_ai_base_url(settings.base_url),
        model: settings.model.trim().to_string(),
        system_prompt: settings.system_prompt.trim().to_string(),
        temperature: settings.temperature.clamp(0.0, 2.0),
        max_tokens: settings.max_tokens.clamp(64, 32768),
    }
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
    app_data::set_current_pet_skin(&app, &skin_id)
}

#[tauri::command]
pub fn import_pet_skin(
    app: AppHandle,
    name: String,
    animations: PetAnimationSet,
) -> Result<PetSkinSummary, String> {
    app_data::import_pet_skin(&app, &name, animations)
}

#[tauri::command]
pub fn delete_pet_skin(app: AppHandle, skin_id: String) -> Result<PetSkinSummary, String> {
    app_data::delete_pet_skin(&app, &skin_id)
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
    app_data::set_current_pet_skin(&app, "default").map(|_| ())
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
pub async fn send_pet_chat_message(
    app: AppHandle,
    messages: Vec<PetChatMessageDraft>,
) -> Result<PetChatReply, String> {
    tauri::async_runtime::spawn_blocking(move || ai_chat::send_pet_chat_message(&app, messages))
        .await
        .map_err(|err| format!("宠物对话任务失败：{err}"))?
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
