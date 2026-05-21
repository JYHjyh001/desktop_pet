use std::{
    fs,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetApp {
    pub id: String,
    pub name: String,
    #[serde(default = "default_item_kind")]
    pub item_kind: String,
    pub path: String,
    pub icon: Option<String>,
    pub category: String,
    #[serde(default)]
    pub run_as_admin: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub auto_favorite: bool,
    #[serde(default)]
    pub launch_count: u32,
    #[serde(default)]
    pub launch_history: Vec<String>,
    pub last_launch_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppDraft {
    pub id: Option<String>,
    pub name: String,
    #[serde(default = "default_item_kind")]
    pub item_kind: String,
    pub path: String,
    pub icon: Option<String>,
    pub category: String,
    #[serde(default)]
    pub run_as_admin: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetSettings {
    pub x: Option<i32>,
    pub y: Option<i32>,
    #[serde(default = "default_current_skin")]
    pub current_skin: String,
    #[serde(default)]
    pub custom_image: Option<String>,
    #[serde(default = "default_true")]
    pub always_on_top: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetAnimationSet {
    pub idle: Option<String>,
    pub hover: Option<String>,
    pub dragging: Option<String>,
    pub click: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetSkinManifest {
    pub id: String,
    pub name: String,
    pub animations: PetAnimationSet,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetSkinSummary {
    pub id: String,
    pub name: String,
    pub builtin: bool,
    pub preview: Option<String>,
    pub animations: PetAnimationSet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawerSettings {
    pub width: u32,
    pub height: u32,
    pub theme: String,
    #[serde(default = "default_true")]
    pub always_on_top: bool,
    #[serde(default = "default_categories")]
    pub categories: Vec<String>,
    #[serde(default = "default_quick_search_tags")]
    pub quick_search_tags: Vec<String>,
    #[serde(default = "default_tag_display_mode")]
    pub tag_display_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutSettings {
    pub toggle_drawer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_ai_provider")]
    pub provider: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_ai_base_url")]
    pub base_url: String,
    #[serde(default = "default_ai_model")]
    pub model: String,
    #[serde(default = "default_ai_system_prompt")]
    pub system_prompt: String,
    #[serde(default = "default_ai_temperature")]
    pub temperature: f32,
    #[serde(default = "default_ai_max_tokens")]
    pub max_tokens: u32,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: default_ai_provider(),
            api_key: String::new(),
            base_url: default_ai_base_url(),
            model: default_ai_model(),
            system_prompt: default_ai_system_prompt(),
            temperature: default_ai_temperature(),
            max_tokens: default_ai_max_tokens(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetDrawerConfig {
    pub pet: PetSettings,
    pub drawer: DrawerSettings,
    pub shortcut: ShortcutSettings,
    #[serde(default)]
    pub ai: AiSettings,
}

impl Default for PetDrawerConfig {
    fn default() -> Self {
        Self {
            pet: PetSettings {
                x: None,
                y: None,
                current_skin: "default".to_string(),
                custom_image: None,
                always_on_top: true,
            },
            drawer: DrawerSettings {
                width: 760,
                height: 540,
                theme: "light".to_string(),
                always_on_top: true,
                categories: default_categories(),
                quick_search_tags: default_quick_search_tags(),
                tag_display_mode: default_tag_display_mode(),
            },
            shortcut: ShortcutSettings {
                toggle_drawer: "Ctrl+Space".to_string(),
            },
            ai: AiSettings::default(),
        }
    }
}

fn default_current_skin() -> String {
    "default".to_string()
}

fn default_true() -> bool {
    true
}

fn default_item_kind() -> String {
    "app".to_string()
}

fn default_categories() -> Vec<String> {
    vec![
        "全部".to_string(),
        "常用".to_string(),
        "开发工具".to_string(),
        "游戏".to_string(),
        "办公".to_string(),
        "系统工具".to_string(),
        "其他".to_string(),
    ]
}

fn default_quick_search_tags() -> Vec<String> {
    vec![
        "微信".to_string(),
        "浏览器".to_string(),
        "代码".to_string(),
        "办公".to_string(),
    ]
}

fn default_tag_display_mode() -> String {
    "compact".to_string()
}

fn default_ai_provider() -> String {
    "openai".to_string()
}

fn default_ai_base_url() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_ai_model() -> String {
    "gpt-4o-mini".to_string()
}

fn default_ai_system_prompt() -> String {
    "你是一个友好、简洁的桌面宠物助手。".to_string()
}

fn default_ai_temperature() -> f32 {
    0.7
}

fn default_ai_max_tokens() -> u32 {
    800
}

pub fn ensure_data_files(app: &AppHandle) -> Result<(), String> {
    let dir = data_dir(app)?;
    fs::create_dir_all(dir.join("icons")).map_err(|err| err.to_string())?;
    fs::create_dir_all(dir.join("pets")).map_err(|err| err.to_string())?;
    fs::create_dir_all(skins_dir(app)?).map_err(|err| err.to_string())?;

    let apps_path = apps_file(app)?;
    if !apps_path.exists() {
        fs::write(apps_path, "[]").map_err(|err| err.to_string())?;
    }

    let config_path = config_file(app)?;
    if !config_path.exists() {
        write_config(app, &PetDrawerConfig::default())?;
    }

    Ok(())
}

pub fn read_apps(app: &AppHandle) -> Result<Vec<PetApp>, String> {
    ensure_data_files(app)?;
    let content = fs::read_to_string(apps_file(app)?).map_err(|err| err.to_string())?;

    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut apps: Vec<PetApp> =
        serde_json::from_str(&content).map_err(|err| format!("apps.json 格式错误：{err}"))?;

    if refresh_auto_favorites(&mut apps) {
        write_apps(app, &apps)?;
    }

    Ok(apps)
}

pub fn write_apps(app: &AppHandle, apps: &[PetApp]) -> Result<(), String> {
    ensure_data_files(app)?;
    let content = serde_json::to_string_pretty(apps).map_err(|err| err.to_string())?;
    fs::write(apps_file(app)?, content).map_err(|err| err.to_string())
}

pub fn read_config(app: &AppHandle) -> Result<PetDrawerConfig, String> {
    ensure_data_files(app)?;
    let content = fs::read_to_string(config_file(app)?).map_err(|err| err.to_string())?;

    if content.trim().is_empty() {
        return Ok(PetDrawerConfig::default());
    }

    serde_json::from_str(&content).map_err(|err| format!("config.json 格式错误：{err}"))
}

pub fn write_config(app: &AppHandle, config: &PetDrawerConfig) -> Result<(), String> {
    let dir = data_dir(app).map_err(|err| err.to_string())?;
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    let content = serde_json::to_string_pretty(config).map_err(|err| err.to_string())?;
    fs::write(dir.join("config.json"), content).map_err(|err| err.to_string())
}

pub fn now_seconds() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

pub fn record_launch(app: &mut PetApp, launched_at: String) {
    app.launch_history.push(launched_at);
    prune_launch_history(app, current_seconds());

    if has_frequent_recent_launches(app) {
        app.favorite = true;
        app.auto_favorite = true;
    }
}

pub fn new_app_id() -> String {
    format!("app_{}", now_millis())
}

pub fn list_pet_skins(app: &AppHandle) -> Result<Vec<PetSkinSummary>, String> {
    ensure_data_files(app)?;

    let mut skins = builtin_pet_skins();

    for entry in fs::read_dir(skins_dir(app)?).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        if !entry.file_type().map_err(|err| err.to_string())?.is_dir() {
            continue;
        }

        let manifest_path = entry.path().join("pet.json");
        if !manifest_path.is_file() {
            continue;
        }

        let Ok(content) = fs::read_to_string(&manifest_path) else {
            continue;
        };
        let Ok(manifest) = serde_json::from_str::<PetSkinManifest>(&content) else {
            continue;
        };
        if let Ok(summary) = manifest_to_summary(app, manifest) {
            skins.push(summary);
        }
    }

    skins.sort_by(|a, b| match (a.builtin, b.builtin) {
        (true, true) => builtin_pet_skin_order(&a.id)
            .cmp(&builtin_pet_skin_order(&b.id))
            .then_with(|| a.name.cmp(&b.name)),
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        (false, false) => a.name.cmp(&b.name),
    });

    Ok(skins)
}

pub fn get_current_pet_skin(app: &AppHandle) -> Result<PetSkinSummary, String> {
    let config = read_config(app)?;

    if config.pet.current_skin == "default" {
        if let Some(custom_image) = config.pet.custom_image.as_deref() {
            if let Ok(preview) = read_image_data_url(app, custom_image) {
                return Ok(PetSkinSummary {
                    id: "legacy_custom".to_string(),
                    name: "旧版自定义宠物".to_string(),
                    builtin: false,
                    preview: Some(preview.clone()),
                    animations: PetAnimationSet {
                        idle: Some(preview),
                        ..PetAnimationSet::default()
                    },
                });
            }
        }

        return Ok(default_pet_skin());
    }

    if let Some(summary) = builtin_pet_skin(&config.pet.current_skin) {
        return Ok(summary);
    }

    read_pet_skin(app, &config.pet.current_skin).or_else(|_| Ok(default_pet_skin()))
}

pub fn set_current_pet_skin(app: &AppHandle, skin_id: &str) -> Result<PetSkinSummary, String> {
    let summary = if let Some(summary) = builtin_pet_skin(skin_id) {
        summary
    } else {
        read_pet_skin(app, skin_id)?
    };

    let mut config = read_config(app)?;
    config.pet.current_skin = skin_id.to_string();
    config.pet.custom_image = None;
    write_config(app, &config)?;

    Ok(summary)
}

pub fn import_pet_skin(
    app: &AppHandle,
    name: &str,
    animations: PetAnimationSet,
) -> Result<PetSkinSummary, String> {
    ensure_data_files(app)?;

    let idle_source = animations
        .idle
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "导入宠物至少需要选择待机动画".to_string())?;

    let skin_id = format!("skin_{}", now_millis());
    let skin_dir = skins_dir(app)?.join(&skin_id);
    fs::create_dir_all(&skin_dir).map_err(|err| err.to_string())?;

    let imported = PetAnimationSet {
        idle: Some(copy_pet_animation(app, &skin_id, "idle", idle_source)?),
        hover: copy_optional_pet_animation(app, &skin_id, "hover", animations.hover.as_deref())?,
        dragging: copy_optional_pet_animation(
            app,
            &skin_id,
            "dragging",
            animations.dragging.as_deref(),
        )?,
        click: copy_optional_pet_animation(app, &skin_id, "click", animations.click.as_deref())?,
    };

    let manifest = PetSkinManifest {
        id: skin_id.clone(),
        name: if name.trim().is_empty() {
            "自定义宠物".to_string()
        } else {
            name.trim().to_string()
        },
        animations: imported,
        created_at: now_seconds(),
    };

    let content = serde_json::to_string_pretty(&manifest).map_err(|err| err.to_string())?;
    fs::write(skin_dir.join("pet.json"), content).map_err(|err| err.to_string())?;

    set_current_pet_skin(app, &skin_id)
}

pub fn delete_pet_skin(app: &AppHandle, skin_id: &str) -> Result<PetSkinSummary, String> {
    ensure_data_files(app)?;

    let skin_id = safe_skin_id(skin_id)?;
    if builtin_pet_skin(skin_id).is_some() {
        return Err("内置宠物形象不能删除".to_string());
    }

    let skin_dir = skins_dir(app)?.join(skin_id);
    if !skin_dir.is_dir() {
        return Err("未找到要删除的宠物形象".to_string());
    }

    fs::remove_dir_all(&skin_dir).map_err(|err| format!("删除宠物形象失败：{err}"))?;

    let mut config = read_config(app)?;
    if config.pet.current_skin == skin_id {
        config.pet.current_skin = "default".to_string();
        config.pet.custom_image = None;
        write_config(app, &config)?;
        return Ok(default_pet_skin());
    }

    get_current_pet_skin(app)
}

fn default_pet_skin() -> PetSkinSummary {
    PetSkinSummary {
        id: "default".to_string(),
        name: "凯蒂".to_string(),
        builtin: true,
        preview: None,
        animations: PetAnimationSet::default(),
    }
}

fn builtin_pet_skins() -> Vec<PetSkinSummary> {
    vec![default_pet_skin()]
}

fn builtin_pet_skin(skin_id: &str) -> Option<PetSkinSummary> {
    match skin_id {
        "default" => Some(default_pet_skin()),
        _ => None,
    }
}

fn builtin_pet_skin_order(skin_id: &str) -> u8 {
    match skin_id {
        "default" => 0,
        _ => 1,
    }
}

fn safe_skin_id(skin_id: &str) -> Result<&str, String> {
    let skin_id = skin_id.trim();
    if skin_id.is_empty() {
        return Err("宠物形象 ID 不能为空".to_string());
    }

    let is_safe = skin_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-');
    if !is_safe || skin_id == "." || skin_id == ".." {
        return Err("宠物形象 ID 不合法".to_string());
    }

    Ok(skin_id)
}

fn read_pet_skin(app: &AppHandle, skin_id: &str) -> Result<PetSkinSummary, String> {
    let manifest_path = skins_dir(app)?.join(skin_id).join("pet.json");
    if !manifest_path.is_file() {
        return Err("未找到该宠物形象".to_string());
    }

    let content = fs::read_to_string(manifest_path).map_err(|err| err.to_string())?;
    let manifest: PetSkinManifest =
        serde_json::from_str(&content).map_err(|err| format!("宠物配置格式错误：{err}"))?;

    manifest_to_summary(app, manifest)
}

fn manifest_to_summary(
    app: &AppHandle,
    manifest: PetSkinManifest,
) -> Result<PetSkinSummary, String> {
    let animations = PetAnimationSet {
        idle: read_optional_image_data_url(app, manifest.animations.idle.as_deref())?,
        hover: read_optional_image_data_url(app, manifest.animations.hover.as_deref())?,
        dragging: read_optional_image_data_url(app, manifest.animations.dragging.as_deref())?,
        click: read_optional_image_data_url(app, manifest.animations.click.as_deref())?,
    };

    let preview = animations
        .idle
        .clone()
        .or_else(|| animations.hover.clone())
        .or_else(|| animations.click.clone())
        .or_else(|| animations.dragging.clone());

    Ok(PetSkinSummary {
        id: manifest.id,
        name: manifest.name,
        builtin: false,
        preview,
        animations,
    })
}

fn read_optional_image_data_url(
    app: &AppHandle,
    relative_path: Option<&str>,
) -> Result<Option<String>, String> {
    let Some(relative_path) = relative_path.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };

    read_image_data_url(app, relative_path).map(Some)
}

fn copy_optional_pet_animation(
    app: &AppHandle,
    skin_id: &str,
    state: &str,
    source_path: Option<&str>,
) -> Result<Option<String>, String> {
    let Some(source_path) = source_path.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };

    copy_pet_animation(app, skin_id, state, source_path).map(Some)
}

fn copy_pet_animation(
    app: &AppHandle,
    skin_id: &str,
    state: &str,
    source_path: &str,
) -> Result<String, String> {
    let source = Path::new(source_path);
    if !source.is_file() {
        return Err(format!("{state} 动画文件不存在"));
    }

    let extension = image_extension(source)?;
    let relative = PathBuf::from("pets")
        .join("skins")
        .join(skin_id)
        .join(format!("{state}.{extension}"));
    let target = data_dir(app)?.join(&relative);

    fs::copy(source, target).map_err(|err| format!("复制 {state} 动画失败：{err}"))?;

    Ok(relative.to_string_lossy().replace('\\', "/"))
}

pub fn import_image(
    app: &AppHandle,
    source_path: &str,
    subdir: &str,
    prefix: &str,
) -> Result<String, String> {
    ensure_data_files(app)?;

    let source = Path::new(source_path);
    if !source.is_file() {
        return Err("请选择有效的图片文件".to_string());
    }

    let extension = image_extension(source)?;

    let file_name = format!("{prefix}_{}.{}", now_millis(), extension);
    let relative = PathBuf::from(subdir).join(file_name);
    let target = data_dir(app)?.join(&relative);

    fs::copy(source, target).map_err(|err| format!("复制图片失败：{err}"))?;

    Ok(relative.to_string_lossy().replace('\\', "/"))
}

pub fn import_executable_icon(app: &AppHandle, source_path: &str) -> Result<String, String> {
    ensure_data_files(app)?;

    let source = Path::new(source_path);
    if !source.is_file() {
        return Err("请选择有效的软件路径".to_string());
    }

    let relative = PathBuf::from("icons").join(format!("auto_icon_{}.ico", now_millis()));
    let target = data_dir(app)?.join(&relative);

    extract_associated_icon(source, &target)?;

    Ok(relative.to_string_lossy().replace('\\', "/"))
}

pub fn read_image_data_url(app: &AppHandle, relative_path: &str) -> Result<String, String> {
    let relative = safe_relative_path(relative_path)?;
    let full_path = data_dir(app)?.join(&relative);

    if !full_path.is_file() {
        return Err("图片文件不存在".to_string());
    }

    let extension = full_path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())
        .ok_or_else(|| "图片文件缺少扩展名".to_string())?;

    let mime = mime_for_extension(&extension)?;
    let bytes = fs::read(full_path).map_err(|err| format!("读取图片失败：{err}"))?;
    let encoded = general_purpose::STANDARD.encode(bytes);

    Ok(format!("data:{mime};base64,{encoded}"))
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn current_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn refresh_auto_favorites(apps: &mut [PetApp]) -> bool {
    let now = current_seconds();
    let mut changed = false;

    for app in apps {
        let previous_favorite = app.favorite;
        let previous_auto_favorite = app.auto_favorite;
        let previous_history = app.launch_history.clone();

        prune_launch_history(app, now);

        if app.auto_favorite && app.launch_history.is_empty() {
            app.favorite = false;
            app.auto_favorite = false;
        }

        changed |= previous_favorite != app.favorite
            || previous_auto_favorite != app.auto_favorite
            || previous_history != app.launch_history;
    }

    changed
}

fn prune_launch_history(app: &mut PetApp, now: u64) {
    const WEEK_SECONDS: u64 = 7 * 24 * 60 * 60;
    let cutoff = now.saturating_sub(WEEK_SECONDS);

    app.launch_history.retain(|value| {
        parse_seconds(value)
            .map(|seconds| seconds >= cutoff && seconds <= now.saturating_add(60))
            .unwrap_or(false)
    });
}

fn has_frequent_recent_launches(app: &PetApp) -> bool {
    app.launch_history.len() >= 2
}

fn parse_seconds(value: &str) -> Option<u64> {
    value.parse::<u64>().ok()
}

#[cfg(target_os = "windows")]
fn extract_associated_icon(source: &Path, target: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let script = r#"
Add-Type -AssemblyName System.Drawing
$source = $args[0]
$target = $args[1]
$icon = [System.Drawing.Icon]::ExtractAssociatedIcon($source)
if ($null -eq $icon) { exit 2 }
$stream = [System.IO.File]::Open($target, [System.IO.FileMode]::Create, [System.IO.FileAccess]::Write)
try {
  $icon.Save($stream)
} finally {
  $stream.Dispose()
  $icon.Dispose()
}
"#;

    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
        .arg(source)
        .arg(target)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|err| format!("提取软件图标失败：{err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            return Err("未能从该软件提取图标".to_string());
        }

        return Err(format!("未能从该软件提取图标：{stderr}"));
    }

    if !target.is_file() {
        return Err("未生成图标文件".to_string());
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn extract_associated_icon(_source: &Path, _target: &Path) -> Result<(), String> {
    Err("当前仅支持在 Windows 上自动获取 exe 图标".to_string())
}

fn image_extension(source: &Path) -> Result<String, String> {
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())
        .ok_or_else(|| "图片文件缺少扩展名".to_string())?;

    if !matches!(
        extension.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "ico"
    ) {
        return Err("仅支持 png、jpg、jpeg、webp、gif、ico 图片".to_string());
    }

    Ok(extension)
}

fn safe_relative_path(relative_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative_path);

    if path.is_absolute() {
        return Err("图片路径必须是应用数据目录内的相对路径".to_string());
    }

    let mut safe = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => safe.push(value),
            _ => return Err("图片路径不合法".to_string()),
        }
    }

    if safe.as_os_str().is_empty() {
        return Err("图片路径不能为空".to_string());
    }

    Ok(safe)
}

fn mime_for_extension(extension: &str) -> Result<&'static str, String> {
    match extension {
        "png" => Ok("image/png"),
        "jpg" | "jpeg" => Ok("image/jpeg"),
        "webp" => Ok("image/webp"),
        "gif" => Ok("image/gif"),
        "ico" => Ok("image/x-icon"),
        _ => Err("不支持的图片类型".to_string()),
    }
}

fn apps_file(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("apps.json"))
}

fn config_file(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("config.json"))
}

fn skins_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("pets").join("skins"))
}

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|err| format!("无法获取应用数据目录：{err}"))
}
