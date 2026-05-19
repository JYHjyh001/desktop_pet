use serde::Deserialize;
use tauri::AppHandle;

use crate::{
    app_data::{
        self, AppDraft, PetAnimationSet, PetApp, PetDrawerConfig, PetPosition, PetSkinSummary,
    },
    launcher, windowing,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawerPreferencesDraft {
    pub categories: Vec<String>,
    pub quick_search_tags: Vec<String>,
    pub tag_display_mode: String,
    pub pet_always_on_top: bool,
    pub drawer_always_on_top: bool,
}

#[tauri::command]
pub fn get_apps(app: AppHandle) -> Result<Vec<PetApp>, String> {
    app_data::read_apps(&app)
}

#[tauri::command]
pub fn upsert_app(app: AppHandle, draft: AppDraft) -> Result<PetApp, String> {
    let mut apps = app_data::read_apps(&app)?;
    let now = app_data::now_seconds();

    if let Some(id) = draft.id.as_deref() {
        let index = apps
            .iter()
            .position(|item| item.id == id)
            .ok_or_else(|| "未找到要编辑的软件".to_string())?;

        apps[index].name = draft.name;
        apps[index].path = draft.path;
        if let Some(icon) = draft.icon {
            apps[index].icon = Some(icon);
        }
        apps[index].category = draft.category;
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
        path: draft.path,
        icon: draft.icon,
        category: draft.category,
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

#[tauri::command]
pub fn delete_app(app: AppHandle, app_id: String) -> Result<(), String> {
    let mut apps = app_data::read_apps(&app)?;
    let before_len = apps.len();
    apps.retain(|item| item.id != app_id);

    if apps.len() == before_len {
        return Err("未找到要删除的软件".to_string());
    }

    app_data::write_apps(&app, &apps)
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
pub fn show_pet(app: AppHandle) -> Result<(), String> {
    windowing::show_pet(&app)
}

#[tauri::command]
pub fn hide_pet(app: AppHandle) -> Result<(), String> {
    windowing::hide_pet(&app)
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}
