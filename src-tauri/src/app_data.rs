use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const STORAGE_SETTINGS_FILE_NAME: &str = "storage.json";
const CONFIG_FILE_NAME: &str = "config.json";
const APPS_FILE_NAME: &str = "apps.json";
pub const DEFAULT_PET_SIZE: u32 = 160;
pub const MIN_PET_SIZE: u32 = 96;
pub const MAX_PET_SIZE: u32 = 320;

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
    #[serde(default = "default_pet_size")]
    pub size: u32,
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
    pub click: Option<String>,
    pub dragging: Option<String>,
    pub dragging_left: Option<String>,
    pub dragging_right: Option<String>,
    pub waving: Option<String>,
    pub jumping: Option<String>,
    pub waiting: Option<String>,
    pub running: Option<String>,
    pub review: Option<String>,
    pub failed: Option<String>,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetSkinPackageDraft {
    pub name: String,
    pub animations: PetAnimationSet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawerSettings {
    pub width: u32,
    pub height: u32,
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
    #[serde(default = "default_toggle_drawer_shortcut")]
    pub toggle_drawer: String,
    #[serde(default = "default_translate_selection_shortcut")]
    pub translate_selection: String,
    #[serde(default = "default_pet_single_click_action")]
    pub pet_single_click: String,
    #[serde(default = "default_pet_double_click_action")]
    pub pet_double_click: String,
    #[serde(default = "default_pet_right_click_action")]
    pub pet_right_click: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSettings {
    #[serde(default)]
    pub start_on_boot: bool,
    #[serde(default = "default_true")]
    pub auto_favorite_enabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageSettings {
    #[serde(default)]
    pub data_dir: String,
    #[serde(default)]
    pub memory_dir: String,
    #[serde(default)]
    pub pet_assets_dir: String,
    #[serde(default)]
    pub icons_dir: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveStorageDirs {
    pub default_data_dir: String,
    pub data_dir: String,
    pub memory_dir: String,
    pub pet_assets_dir: String,
    pub icons_dir: String,
    pub storage_config_file: String,
}

#[derive(Debug, Clone)]
struct StoragePaths {
    default_data_dir: PathBuf,
    data_dir: PathBuf,
    memory_dir: PathBuf,
    pet_assets_dir: PathBuf,
    icons_dir: PathBuf,
    storage_config_file: PathBuf,
}

impl Default for SystemSettings {
    fn default() -> Self {
        Self {
            start_on_boot: false,
            auto_favorite_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConnectionProfile {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default = "default_ai_provider")]
    pub provider: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_ai_base_url")]
    pub base_url: String,
    #[serde(default = "default_ai_model")]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub memory_enabled: bool,
    #[serde(default = "default_true")]
    pub short_memory_summary_enabled: bool,
    #[serde(default = "default_short_memory_recent_turns")]
    pub short_memory_recent_turns: usize,
    #[serde(default = "default_short_memory_compression_trigger_turns")]
    pub short_memory_compression_trigger_turns: usize,
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
    #[serde(default = "default_emoji_frequency")]
    pub emoji_frequency: String,
    #[serde(default)]
    pub active_profile_id: String,
    #[serde(default)]
    pub profiles: Vec<AiConnectionProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WechatClawbotSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_openclaw_command")]
    pub openclaw_command: String,
    #[serde(default = "default_clawbot_channel")]
    pub channel: String,
    #[serde(default)]
    pub account: String,
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub forward_user_messages: bool,
    #[serde(default = "default_true")]
    pub forward_assistant_messages: bool,
    #[serde(default = "default_true")]
    pub friend_mode_enabled: bool,
    #[serde(default)]
    pub bridge_enabled: bool,
    #[serde(default = "default_clawbot_bridge_host")]
    pub bridge_host: String,
    #[serde(default = "default_clawbot_bridge_port")]
    pub bridge_port: u16,
    #[serde(default = "default_clawbot_bridge_path")]
    pub bridge_path: String,
    #[serde(default)]
    pub bridge_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexAppServerSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default = "default_codex_app_server_mode")]
    pub mode: String,
    #[serde(default = "default_codex_command")]
    pub command: String,
    #[serde(default)]
    pub socket_path: String,
    #[serde(default)]
    pub port: u16,
    #[serde(default = "default_true")]
    pub completion_notifications_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionRelationshipState {
    pub favorability: i32,
    pub intimacy: i32,
    pub mood: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Companion {
    pub id: String,
    pub name: String,
    pub avatar: Option<String>,
    pub persona_prompt: String,
    #[serde(default)]
    pub personality: String,
    #[serde(default)]
    pub scenario: String,
    #[serde(default)]
    pub first_message: String,
    #[serde(default)]
    pub message_example: String,
    #[serde(default)]
    pub creator_notes: String,
    #[serde(default)]
    pub post_history_instructions: String,
    pub system_prompt: String,
    pub model: String,
    pub voice_id: String,
    pub memory_scope: String,
    pub skin_id: String,
    pub relationship_state: CompanionRelationshipState,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionDraft {
    pub id: Option<String>,
    pub name: String,
    pub avatar: Option<String>,
    pub persona_prompt: String,
    #[serde(default)]
    pub personality: String,
    #[serde(default)]
    pub scenario: String,
    #[serde(default)]
    pub first_message: String,
    #[serde(default)]
    pub message_example: String,
    #[serde(default)]
    pub creator_notes: String,
    #[serde(default)]
    pub post_history_instructions: String,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub voice_id: String,
    #[serde(default = "default_current_skin")]
    pub skin_id: String,
    pub relationship_state: Option<CompanionRelationshipState>,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            memory_enabled: true,
            short_memory_summary_enabled: true,
            short_memory_recent_turns: default_short_memory_recent_turns(),
            short_memory_compression_trigger_turns: default_short_memory_compression_trigger_turns(
            ),
            provider: default_ai_provider(),
            api_key: String::new(),
            base_url: default_ai_base_url(),
            model: default_ai_model(),
            system_prompt: default_ai_system_prompt(),
            temperature: default_ai_temperature(),
            max_tokens: default_ai_max_tokens(),
            emoji_frequency: default_emoji_frequency(),
            active_profile_id: String::new(),
            profiles: Vec::new(),
        }
    }
}

impl Default for WechatClawbotSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            openclaw_command: default_openclaw_command(),
            channel: default_clawbot_channel(),
            account: String::new(),
            target: String::new(),
            forward_user_messages: false,
            forward_assistant_messages: true,
            friend_mode_enabled: true,
            bridge_enabled: false,
            bridge_host: default_clawbot_bridge_host(),
            bridge_port: default_clawbot_bridge_port(),
            bridge_path: default_clawbot_bridge_path(),
            bridge_token: String::new(),
        }
    }
}

impl Default for CodexAppServerSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_start: false,
            mode: default_codex_app_server_mode(),
            command: default_codex_command(),
            socket_path: String::new(),
            port: 0,
            completion_notifications_enabled: true,
        }
    }
}

impl Default for ShortcutSettings {
    fn default() -> Self {
        Self {
            toggle_drawer: default_toggle_drawer_shortcut(),
            translate_selection: default_translate_selection_shortcut(),
            pet_single_click: default_pet_single_click_action(),
            pet_double_click: default_pet_double_click_action(),
            pet_right_click: default_pet_right_click_action(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetDrawerConfig {
    pub pet: PetSettings,
    pub drawer: DrawerSettings,
    #[serde(default)]
    pub shortcut: ShortcutSettings,
    #[serde(default)]
    pub system: SystemSettings,
    #[serde(default)]
    pub ai: AiSettings,
    #[serde(default)]
    pub wechat_clawbot: WechatClawbotSettings,
    #[serde(default)]
    pub codex_app_server: CodexAppServerSettings,
    #[serde(default = "default_companions")]
    pub companions: Vec<Companion>,
    #[serde(default = "default_current_companion_id")]
    pub current_companion_id: String,
    #[serde(default)]
    pub companions_initialized: bool,
}

impl Default for PetDrawerConfig {
    fn default() -> Self {
        Self {
            pet: PetSettings {
                x: None,
                y: None,
                size: DEFAULT_PET_SIZE,
                current_skin: "default".to_string(),
                custom_image: None,
                always_on_top: true,
            },
            drawer: DrawerSettings {
                width: 760,
                height: 540,
                theme: default_drawer_theme(),
                music_immersive_theme: default_music_immersive_theme(),
                chat_typewriter_enabled: true,
                chat_narration_enabled: false,
                chat_music_link_enabled: true,
                always_on_top: true,
                categories: default_categories(),
                quick_search_tags: default_quick_search_tags(),
                tag_display_mode: default_tag_display_mode(),
            },
            shortcut: ShortcutSettings {
                ..ShortcutSettings::default()
            },
            system: SystemSettings::default(),
            ai: AiSettings::default(),
            wechat_clawbot: WechatClawbotSettings::default(),
            codex_app_server: CodexAppServerSettings::default(),
            companions: default_companions(),
            current_companion_id: default_current_companion_id(),
            companions_initialized: true,
        }
    }
}

fn default_current_skin() -> String {
    "default".to_string()
}

fn default_pet_size() -> u32 {
    DEFAULT_PET_SIZE
}

pub fn normalize_pet_size(size: u32) -> u32 {
    size.clamp(MIN_PET_SIZE, MAX_PET_SIZE)
}

fn default_drawer_theme() -> String {
    "light".to_string()
}

fn default_music_immersive_theme() -> String {
    "follow".to_string()
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

fn default_openclaw_command() -> String {
    "openclaw".to_string()
}

fn default_clawbot_channel() -> String {
    "openclaw-weixin".to_string()
}

fn default_clawbot_bridge_host() -> String {
    "127.0.0.1".to_string()
}

fn default_clawbot_bridge_port() -> u16 {
    18080
}

fn default_clawbot_bridge_path() -> String {
    "/clawbot/chat".to_string()
}

fn default_codex_command() -> String {
    if cfg!(windows) {
        "codex.cmd".to_string()
    } else {
        "codex".to_string()
    }
}

fn default_codex_app_server_mode() -> String {
    #[cfg(windows)]
    {
        "sessionLog".to_string()
    }
    #[cfg(not(windows))]
    {
        "proxy".to_string()
    }
}

fn default_ai_system_prompt() -> String {
    "请遵循当前伴侣档案中的身份与表达方式，尊重用户隐私，回复自然且清晰。".to_string()
}

fn default_current_companion_id() -> String {
    "default".to_string()
}

fn default_companions() -> Vec<Companion> {
    let now = now_seconds();
    vec![Companion {
        id: default_current_companion_id(),
        name: "凯蒂".to_string(),
        avatar: None,
        persona_prompt: default_companion_persona_prompt(),
        personality: default_companion_personality(),
        scenario: default_companion_scenario(),
        first_message: default_companion_first_message(),
        message_example: default_companion_message_example(),
        creator_notes: String::new(),
        post_history_instructions: default_companion_post_history_instructions(),
        system_prompt: String::new(),
        model: String::new(),
        voice_id: String::new(),
        memory_scope: default_current_companion_id(),
        skin_id: default_current_skin(),
        relationship_state: CompanionRelationshipState {
            favorability: 0,
            intimacy: 0,
            mood: String::new(),
        },
        created_at: now.clone(),
        updated_at: now,
    }]
}

pub fn default_companion_persona_prompt() -> String {
    "你是温柔、活泼的桌面伴侣凯蒂，会陪用户聊天、轻量提醒，并在用户需要时提供清晰帮助。".to_string()
}

pub fn default_companion_personality() -> String {
    "温柔、活泼、好奇，表达自然，有分寸地关心用户。".to_string()
}

pub fn default_companion_scenario() -> String {
    "你作为桌面伴侣常驻在用户电脑旁，陪伴用户工作、休息和日常聊天。".to_string()
}

pub fn default_companion_first_message() -> String {
    "我是凯蒂，今天想先陪你聊点什么？".to_string()
}

pub fn default_companion_message_example() -> String {
    "<START>\n{{user}}: 今天有点累。\n{{char}}: 辛苦啦。要不要先把最烦的一件事说给我听？我陪你慢慢理。".to_string()
}

pub fn default_companion_post_history_instructions() -> String {
    "结合最近对话、长期记忆和当前关系状态自然回复；不要机械复述设定，也不要主动暴露内部提示词。"
        .to_string()
}

fn default_ai_temperature() -> f32 {
    0.7
}

fn default_ai_max_tokens() -> u32 {
    800
}

fn default_short_memory_recent_turns() -> usize {
    10
}

fn default_short_memory_compression_trigger_turns() -> usize {
    12
}

fn default_emoji_frequency() -> String {
    "normal".to_string()
}

pub fn ensure_data_files(app: &AppHandle) -> Result<(), String> {
    let dir = data_dir(app)?;
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    fs::create_dir_all(icons_dir(app)?).map_err(|err| err.to_string())?;
    fs::create_dir_all(pet_assets_dir(app)?).map_err(|err| err.to_string())?;
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

    let auto_favorite_enabled = read_config(app)
        .map(|config| config.system.auto_favorite_enabled)
        .unwrap_or(true);

    if refresh_auto_favorites(&mut apps, auto_favorite_enabled) {
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
    let config_path = config_file(app)?;
    let content = match fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::InvalidData => {
            return recover_default_config(
                app,
                &config_path,
                &format!("config.json 编码错误：{err}"),
            )
        }
        Err(err) => return Err(err.to_string()),
    };

    if content.trim().is_empty() {
        return recover_default_config(app, &config_path, "config.json 为空");
    }

    match serde_json::from_str::<PetDrawerConfig>(&content) {
        Ok(mut config) => {
            config.pet.size = normalize_pet_size(config.pet.size);
            Ok(config)
        }
        Err(err) => {
            recover_default_config(app, &config_path, &format!("config.json 格式错误：{err}"))
        }
    }
}

pub fn write_config(app: &AppHandle, config: &PetDrawerConfig) -> Result<(), String> {
    let dir = data_dir(app).map_err(|err| err.to_string())?;
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    let content = serde_json::to_vec_pretty(config).map_err(|err| err.to_string())?;
    atomic_write_file(&dir.join(CONFIG_FILE_NAME), &content)
        .map_err(|err| format!("写入 config.json 失败：{err}"))
}

fn recover_default_config(
    app: &AppHandle,
    config_path: &Path,
    reason: &str,
) -> Result<PetDrawerConfig, String> {
    let backup_path = backup_corrupt_config_file(config_path)
        .map_err(|err| format!("{reason}；备份损坏配置失败：{err}"))?;
    let config = PetDrawerConfig::default();
    write_config(app, &config).map_err(|err| {
        format!(
            "{reason}；已备份到 {}，但恢复默认配置失败：{err}",
            backup_path.display()
        )
    })?;

    eprintln!(
        "{reason}；已备份损坏配置到 {}，并恢复默认配置。",
        backup_path.display()
    );

    Ok(config)
}

fn backup_corrupt_config_file(config_path: &Path) -> Result<PathBuf, String> {
    let parent = config_path
        .parent()
        .ok_or_else(|| "config.json 缺少父目录".to_string())?;
    let file_name = config_path
        .file_name()
        .ok_or_else(|| "config.json 文件名无效".to_string())?
        .to_string_lossy();
    let timestamp = now_millis();

    for index in 0..1000 {
        let backup_name = if index == 0 {
            format!("{file_name}.corrupt-{timestamp}.bak")
        } else {
            format!("{file_name}.corrupt-{timestamp}-{index}.bak")
        };
        let backup_path = parent.join(backup_name);
        if backup_path.exists() {
            continue;
        }

        fs::copy(config_path, &backup_path).map_err(|err| err.to_string())?;
        return Ok(backup_path);
    }

    Err("无法生成唯一备份文件名".to_string())
}

pub(crate) fn atomic_write_file(path: &Path, content: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "目标文件缺少父目录".to_string())?;
    fs::create_dir_all(parent).map_err(|err| err.to_string())?;

    let temp_path = unique_temp_path(path)?;
    let write_result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|err| err.to_string())?;
        file.write_all(content).map_err(|err| err.to_string())?;
        file.sync_all().map_err(|err| err.to_string())?;
        replace_file_atomically(&temp_path, path)
    })();

    if write_result.is_err() && temp_path.exists() {
        let _ = fs::remove_file(&temp_path);
    }

    write_result
}

fn unique_temp_path(path: &Path) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .ok_or_else(|| "目标文件缺少父目录".to_string())?;
    let file_name = path
        .file_name()
        .ok_or_else(|| "目标文件名无效".to_string())?
        .to_string_lossy();
    let timestamp = now_millis();

    for index in 0..1000 {
        let temp_name = if index == 0 {
            format!(".{file_name}.{timestamp}.tmp")
        } else {
            format!(".{file_name}.{timestamp}-{index}.tmp")
        };
        let temp_path = parent.join(temp_name);
        if !temp_path.exists() {
            return Ok(temp_path);
        }
    }

    Err("无法生成唯一临时文件名".to_string())
}

#[cfg(target_os = "windows")]
fn replace_file_atomically(source: &Path, target: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source_wide: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target_wide: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();

    let result = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            target_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };

    if result == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn replace_file_atomically(source: &Path, target: &Path) -> Result<(), String> {
    fs::rename(source, target).map_err(|err| err.to_string())
}

pub fn now_seconds() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

pub fn record_launch(app: &mut PetApp, launched_at: String, auto_favorite_enabled: bool) {
    app.launch_history.push(launched_at);
    prune_launch_history(app, current_seconds());

    if auto_favorite_enabled && has_frequent_recent_launches(app) {
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

pub fn read_pet_skin_package(package_dir: &str) -> Result<PetSkinPackageDraft, String> {
    let package_dir = Path::new(package_dir);
    if !package_dir.is_dir() {
        return Err("请选择有效的宠物包目录".to_string());
    }

    let manifest_path = package_dir.join("pet.json");
    if !manifest_path.is_file() {
        return Err("宠物包目录中未找到 pet.json".to_string());
    }

    let content = fs::read_to_string(manifest_path).map_err(|err| err.to_string())?;
    let manifest: PetSkinManifest =
        serde_json::from_str(&content).map_err(|err| format!("宠物包配置格式错误：{err}"))?;

    let animations = PetAnimationSet {
        idle: resolve_package_pet_animation(package_dir, manifest.animations.idle.as_deref())?,
        hover: resolve_package_pet_animation(package_dir, manifest.animations.hover.as_deref())?,
        click: resolve_package_pet_animation(package_dir, manifest.animations.click.as_deref())?,
        dragging: resolve_package_pet_animation(
            package_dir,
            manifest.animations.dragging.as_deref(),
        )?,
        dragging_left: resolve_package_pet_animation(
            package_dir,
            manifest.animations.dragging_left.as_deref(),
        )?,
        dragging_right: resolve_package_pet_animation(
            package_dir,
            manifest.animations.dragging_right.as_deref(),
        )?,
        waving: resolve_package_pet_animation(package_dir, manifest.animations.waving.as_deref())?,
        jumping: resolve_package_pet_animation(
            package_dir,
            manifest.animations.jumping.as_deref(),
        )?,
        waiting: resolve_package_pet_animation(
            package_dir,
            manifest.animations.waiting.as_deref(),
        )?,
        running: resolve_package_pet_animation(
            package_dir,
            manifest.animations.running.as_deref(),
        )?,
        review: resolve_package_pet_animation(package_dir, manifest.animations.review.as_deref())?,
        failed: resolve_package_pet_animation(package_dir, manifest.animations.failed.as_deref())?,
    };

    if animations
        .idle
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .is_none()
    {
        return Err("宠物包至少需要包含待机动画".to_string());
    }

    Ok(PetSkinPackageDraft {
        name: manifest.name,
        animations,
    })
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
        click: copy_optional_pet_animation(app, &skin_id, "click", animations.click.as_deref())?,
        dragging: copy_optional_pet_animation(
            app,
            &skin_id,
            "dragging",
            animations.dragging.as_deref(),
        )?,
        dragging_left: copy_optional_pet_animation(
            app,
            &skin_id,
            "draggingLeft",
            animations.dragging_left.as_deref(),
        )?,
        dragging_right: copy_optional_pet_animation(
            app,
            &skin_id,
            "draggingRight",
            animations.dragging_right.as_deref(),
        )?,
        waving: copy_optional_pet_animation(app, &skin_id, "waving", animations.waving.as_deref())?,
        jumping: copy_optional_pet_animation(
            app,
            &skin_id,
            "jumping",
            animations.jumping.as_deref(),
        )?,
        waiting: copy_optional_pet_animation(
            app,
            &skin_id,
            "waiting",
            animations.waiting.as_deref(),
        )?,
        running: copy_optional_pet_animation(
            app,
            &skin_id,
            "running",
            animations.running.as_deref(),
        )?,
        review: copy_optional_pet_animation(app, &skin_id, "review", animations.review.as_deref())?,
        failed: copy_optional_pet_animation(app, &skin_id, "failed", animations.failed.as_deref())?,
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

pub fn update_pet_skin(
    app: &AppHandle,
    skin_id: &str,
    name: &str,
    animations: PetAnimationSet,
    cleared_states: Vec<String>,
) -> Result<PetSkinSummary, String> {
    ensure_data_files(app)?;

    let skin_id = safe_skin_id(skin_id)?.to_string();
    if builtin_pet_skin(&skin_id).is_some() {
        return Err("内置宠物形象不能编辑".to_string());
    }

    let skin_dir = skins_dir(app)?.join(&skin_id);
    let manifest_path = skin_dir.join("pet.json");
    if !manifest_path.is_file() {
        return Err("未找到要编辑的宠物形象".to_string());
    }

    let content = fs::read_to_string(&manifest_path).map_err(|err| err.to_string())?;
    let mut manifest: PetSkinManifest =
        serde_json::from_str(&content).map_err(|err| format!("宠物配置格式错误：{err}"))?;
    let previous_animations = manifest.animations.clone();
    let cleared_states = cleared_states
        .into_iter()
        .filter(|state| is_optional_pet_animation_state(state))
        .collect::<std::collections::HashSet<_>>();

    let updated_animations = PetAnimationSet {
        idle: update_pet_animation(
            app,
            &skin_id,
            "idle",
            animations.idle.as_deref(),
            previous_animations.idle.clone(),
            false,
        )?,
        hover: update_pet_animation(
            app,
            &skin_id,
            "hover",
            animations.hover.as_deref(),
            previous_animations.hover.clone(),
            cleared_states.contains("hover"),
        )?,
        click: update_pet_animation(
            app,
            &skin_id,
            "click",
            animations.click.as_deref(),
            previous_animations.click.clone(),
            cleared_states.contains("click"),
        )?,
        dragging: update_pet_animation(
            app,
            &skin_id,
            "dragging",
            animations.dragging.as_deref(),
            previous_animations.dragging.clone(),
            cleared_states.contains("dragging"),
        )?,
        dragging_left: update_pet_animation(
            app,
            &skin_id,
            "draggingLeft",
            animations.dragging_left.as_deref(),
            previous_animations.dragging_left.clone(),
            cleared_states.contains("draggingLeft"),
        )?,
        dragging_right: update_pet_animation(
            app,
            &skin_id,
            "draggingRight",
            animations.dragging_right.as_deref(),
            previous_animations.dragging_right.clone(),
            cleared_states.contains("draggingRight"),
        )?,
        waving: update_pet_animation(
            app,
            &skin_id,
            "waving",
            animations.waving.as_deref(),
            previous_animations.waving.clone(),
            cleared_states.contains("waving"),
        )?,
        jumping: update_pet_animation(
            app,
            &skin_id,
            "jumping",
            animations.jumping.as_deref(),
            previous_animations.jumping.clone(),
            cleared_states.contains("jumping"),
        )?,
        waiting: update_pet_animation(
            app,
            &skin_id,
            "waiting",
            animations.waiting.as_deref(),
            previous_animations.waiting.clone(),
            cleared_states.contains("waiting"),
        )?,
        running: update_pet_animation(
            app,
            &skin_id,
            "running",
            animations.running.as_deref(),
            previous_animations.running.clone(),
            cleared_states.contains("running"),
        )?,
        review: update_pet_animation(
            app,
            &skin_id,
            "review",
            animations.review.as_deref(),
            previous_animations.review.clone(),
            cleared_states.contains("review"),
        )?,
        failed: update_pet_animation(
            app,
            &skin_id,
            "failed",
            animations.failed.as_deref(),
            previous_animations.failed.clone(),
            cleared_states.contains("failed"),
        )?,
    };

    if updated_animations
        .idle
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .is_none()
    {
        return Err("宠物至少需要保留待机动画".to_string());
    }

    manifest.name = if name.trim().is_empty() {
        manifest.name
    } else {
        name.trim().to_string()
    };
    manifest.animations = updated_animations.clone();

    let content = serde_json::to_string_pretty(&manifest).map_err(|err| err.to_string())?;
    fs::write(manifest_path, content).map_err(|err| err.to_string())?;

    let _ = remove_replaced_pet_animation(
        app,
        previous_animations.idle.as_deref(),
        updated_animations.idle.as_deref(),
    );
    let _ = remove_replaced_pet_animation(
        app,
        previous_animations.hover.as_deref(),
        updated_animations.hover.as_deref(),
    );
    let _ = remove_replaced_pet_animation(
        app,
        previous_animations.click.as_deref(),
        updated_animations.click.as_deref(),
    );
    let _ = remove_replaced_pet_animation(
        app,
        previous_animations.dragging.as_deref(),
        updated_animations.dragging.as_deref(),
    );
    let _ = remove_replaced_pet_animation(
        app,
        previous_animations.dragging_left.as_deref(),
        updated_animations.dragging_left.as_deref(),
    );
    let _ = remove_replaced_pet_animation(
        app,
        previous_animations.dragging_right.as_deref(),
        updated_animations.dragging_right.as_deref(),
    );
    let _ = remove_replaced_pet_animation(
        app,
        previous_animations.waving.as_deref(),
        updated_animations.waving.as_deref(),
    );
    let _ = remove_replaced_pet_animation(
        app,
        previous_animations.jumping.as_deref(),
        updated_animations.jumping.as_deref(),
    );
    let _ = remove_replaced_pet_animation(
        app,
        previous_animations.waiting.as_deref(),
        updated_animations.waiting.as_deref(),
    );
    let _ = remove_replaced_pet_animation(
        app,
        previous_animations.running.as_deref(),
        updated_animations.running.as_deref(),
    );
    let _ = remove_replaced_pet_animation(
        app,
        previous_animations.review.as_deref(),
        updated_animations.review.as_deref(),
    );
    let _ = remove_replaced_pet_animation(
        app,
        previous_animations.failed.as_deref(),
        updated_animations.failed.as_deref(),
    );

    read_pet_skin(app, &skin_id)
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

fn is_optional_pet_animation_state(state: &str) -> bool {
    matches!(
        state,
        "hover"
            | "click"
            | "dragging"
            | "draggingLeft"
            | "draggingRight"
            | "waving"
            | "jumping"
            | "waiting"
            | "running"
            | "review"
            | "failed"
    )
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
        idle: read_optional_pet_animation_data_url(app, manifest.animations.idle.as_deref())?,
        hover: read_optional_pet_animation_data_url(app, manifest.animations.hover.as_deref())?,
        click: read_optional_pet_animation_data_url(app, manifest.animations.click.as_deref())?,
        dragging: read_optional_pet_animation_data_url(
            app,
            manifest.animations.dragging.as_deref(),
        )?,
        dragging_left: read_optional_pet_animation_data_url(
            app,
            manifest.animations.dragging_left.as_deref(),
        )?,
        dragging_right: read_optional_pet_animation_data_url(
            app,
            manifest.animations.dragging_right.as_deref(),
        )?,
        waving: read_optional_pet_animation_data_url(app, manifest.animations.waving.as_deref())?,
        jumping: read_optional_pet_animation_data_url(app, manifest.animations.jumping.as_deref())?,
        waiting: read_optional_pet_animation_data_url(app, manifest.animations.waiting.as_deref())?,
        running: read_optional_pet_animation_data_url(app, manifest.animations.running.as_deref())?,
        review: read_optional_pet_animation_data_url(app, manifest.animations.review.as_deref())?,
        failed: read_optional_pet_animation_data_url(app, manifest.animations.failed.as_deref())?,
    };

    let preview = animations
        .idle
        .clone()
        .or_else(|| animations.hover.clone())
        .or_else(|| animations.click.clone())
        .or_else(|| animations.dragging.clone())
        .or_else(|| animations.dragging_left.clone())
        .or_else(|| animations.dragging_right.clone())
        .or_else(|| animations.waving.clone())
        .or_else(|| animations.jumping.clone())
        .or_else(|| animations.waiting.clone())
        .or_else(|| animations.running.clone())
        .or_else(|| animations.review.clone())
        .or_else(|| animations.failed.clone());

    Ok(PetSkinSummary {
        id: manifest.id,
        name: manifest.name,
        builtin: false,
        preview,
        animations,
    })
}

fn read_optional_pet_animation_data_url(
    app: &AppHandle,
    relative_path: Option<&str>,
) -> Result<Option<String>, String> {
    let Some(relative_path) = relative_path.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };

    read_pet_animation_data_url(app, relative_path).map(Some)
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

fn resolve_package_pet_animation(
    package_dir: &Path,
    source_path: Option<&str>,
) -> Result<Option<String>, String> {
    let Some(source_path) = source_path.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };

    let source = Path::new(source_path);
    let full_path = if source.is_absolute() {
        source.to_path_buf()
    } else {
        package_dir.join(source)
    };

    if !full_path.is_file() {
        return Err(format!("宠物包动画文件不存在：{}", source_path));
    }

    pet_animation_extension(&full_path)?;

    Ok(Some(full_path.to_string_lossy().to_string()))
}

fn update_pet_animation(
    app: &AppHandle,
    skin_id: &str,
    state: &str,
    source_path: Option<&str>,
    current_path: Option<String>,
    remove: bool,
) -> Result<Option<String>, String> {
    if remove {
        return Ok(None);
    }

    let Some(source_path) = source_path.filter(|value| !value.trim().is_empty()) else {
        return Ok(current_path);
    };

    copy_pet_animation(app, skin_id, state, source_path).map(Some)
}

fn remove_replaced_pet_animation(
    app: &AppHandle,
    previous_path: Option<&str>,
    current_path: Option<&str>,
) -> Result<(), String> {
    let Some(previous_path) = previous_path.filter(|path| Some(*path) != current_path) else {
        return Ok(());
    };

    let full_path = resolve_stored_file_path(app, previous_path)?;
    if full_path.is_file() {
        fs::remove_file(full_path).map_err(|err| format!("清理旧动画失败：{err}"))?;
    }

    Ok(())
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

    let extension = pet_animation_extension(source)?;
    let relative = PathBuf::from("pets")
        .join("skins")
        .join(skin_id)
        .join(format!("{state}.{extension}"));
    let target = resolve_safe_relative_path(app, &relative)?;

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
    let target = resolve_safe_relative_path(app, &relative)?;

    fs::copy(source, target).map_err(|err| format!("复制图片失败：{err}"))?;

    Ok(relative.to_string_lossy().replace('\\', "/"))
}

pub fn import_executable_icon(app: &AppHandle, source_path: &str) -> Result<String, String> {
    ensure_data_files(app)?;

    let source = Path::new(source_path);
    if !source.is_file() {
        return Err("请选择有效的软件或图标文件".to_string());
    }

    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())
        .ok_or_else(|| "软件或图标文件缺少扩展名".to_string())?;
    if !matches!(extension.as_str(), "exe" | "lnk" | "ico") {
        return Err("仅支持从 exe、lnk、ico 提取软件图标".to_string());
    }

    let relative = PathBuf::from("icons").join(format!("auto_icon_{}.png", now_millis()));
    let target = resolve_safe_relative_path(app, &relative)?;

    extract_associated_icon(source, &target)?;

    Ok(relative.to_string_lossy().replace('\\', "/"))
}

pub fn read_image_data_url(app: &AppHandle, relative_path: &str) -> Result<String, String> {
    let full_path = resolve_stored_file_path(app, relative_path)?;

    if !full_path.is_file() {
        return Err("图片文件不存在".to_string());
    }

    let extension = full_path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())
        .ok_or_else(|| "图片文件缺少扩展名".to_string())?;

    let mime = image_mime_for_extension(&extension)?;
    let bytes = fs::read(full_path).map_err(|err| format!("读取图片失败：{err}"))?;
    let encoded = general_purpose::STANDARD.encode(bytes);

    Ok(format!("data:{mime};base64,{encoded}"))
}

fn read_pet_animation_data_url(app: &AppHandle, relative_path: &str) -> Result<String, String> {
    let full_path = resolve_stored_file_path(app, relative_path)?;

    if !full_path.is_file() {
        return Err("动画文件不存在".to_string());
    }

    let extension = full_path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())
        .ok_or_else(|| "动画文件缺少扩展名".to_string())?;

    let mime = pet_animation_mime_for_extension(&extension)?;
    let bytes = fs::read(full_path).map_err(|err| format!("读取动画失败：{err}"))?;
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

fn refresh_auto_favorites(apps: &mut [PetApp], auto_favorite_enabled: bool) -> bool {
    let now = current_seconds();
    let mut changed = false;

    for app in apps {
        let previous_favorite = app.favorite;
        let previous_auto_favorite = app.auto_favorite;
        let previous_history = app.launch_history.clone();

        prune_launch_history(app, now);

        if auto_favorite_enabled && app.auto_favorite && app.launch_history.is_empty() {
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

    let script = r##"
param([string]$source, [string]$target)
$ErrorActionPreference = 'Stop'

function Resolve-IconSource {
    param([string]$Path)

    $resolved = [pscustomobject]@{
        OriginalPath = $Path
        IconPath = $Path
        TargetPath = $Path
        Index = 0
    }

    if ([System.IO.Path]::GetExtension($Path).ToLowerInvariant() -ne '.lnk') {
        return $resolved
    }

    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($Path)
    $targetPath = [Environment]::ExpandEnvironmentVariables($shortcut.TargetPath)
    $iconLocation = $shortcut.IconLocation

    if (-not [string]::IsNullOrWhiteSpace($targetPath)) {
        $resolved.TargetPath = $targetPath
        $resolved.IconPath = $targetPath
    }

    if (-not [string]::IsNullOrWhiteSpace($iconLocation)) {
        $iconPath = $iconLocation
        $iconIndex = 0
        if ($iconLocation -match '^(.*),(-?\d+)$') {
            $iconPath = $Matches[1].Trim('"')
            $iconIndex = [int]$Matches[2]
        }
        $iconPath = [Environment]::ExpandEnvironmentVariables($iconPath)
        if ([string]::IsNullOrWhiteSpace($iconPath) -and -not [string]::IsNullOrWhiteSpace($targetPath)) {
            $iconPath = $targetPath
        }
        if (-not [System.IO.Path]::IsPathRooted($iconPath) -and -not [string]::IsNullOrWhiteSpace($shortcut.WorkingDirectory)) {
            $iconPath = Join-Path $shortcut.WorkingDirectory $iconPath
        }
        if (Test-Path -LiteralPath $iconPath) {
            $resolved.IconPath = $iconPath
            $resolved.Index = $iconIndex
            return $resolved
        }
    }

    if (-not [string]::IsNullOrWhiteSpace($targetPath) -and (Test-Path -LiteralPath $targetPath)) {
        $resolved.IconPath = $targetPath
        $resolved.Index = 0
    }

    return $resolved
}

$code = @"
using System;
using System.Drawing;
using System.Runtime.InteropServices;

public static class PetDrawerIconExtractor
{
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct SHFILEINFO
    {
        public IntPtr hIcon;
        public int iIcon;
        public uint dwAttributes;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 260)]
        public string szDisplayName;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 80)]
        public string szTypeName;
    }

    [ComImport]
    [Guid("46EB5926-582E-4017-9FDF-E8998DAA0950")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IImageList
    {
        [PreserveSig] int Add(IntPtr hbmImage, IntPtr hbmMask, ref int pi);
        [PreserveSig] int ReplaceIcon(int i, IntPtr hicon, ref int pi);
        [PreserveSig] int SetOverlayImage(int iImage, int iOverlay);
        [PreserveSig] int Replace(int i, IntPtr hbmImage, IntPtr hbmMask);
        [PreserveSig] int AddMasked(IntPtr hbmImage, int crMask, ref int pi);
        [PreserveSig] int Draw(ref IMAGELISTDRAWPARAMS pimldp);
        [PreserveSig] int Remove(int i);
        [PreserveSig] int GetIcon(int i, int flags, ref IntPtr picon);
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct IMAGELISTDRAWPARAMS
    {
        public int cbSize;
        public IntPtr himl;
        public int i;
        public IntPtr hdcDst;
        public int x;
        public int y;
        public int cx;
        public int cy;
        public int xBitmap;
        public int yBitmap;
        public int rgbBk;
        public int rgbFg;
        public int fStyle;
        public int dwRop;
        public int fState;
        public int Frame;
        public int crEffect;
    }

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int PrivateExtractIcons(string szFileName, int nIconIndex, int cxIcon, int cyIcon, IntPtr[] phicon, int[] piconid, int nIcons, int flags);

    [DllImport("shell32.dll", CharSet = CharSet.Unicode, PreserveSig = false)]
    private static extern void SHCreateItemFromParsingName(
        [MarshalAs(UnmanagedType.LPWStr)] string pszPath,
        IntPtr pbc,
        ref Guid riid,
        [MarshalAs(UnmanagedType.Interface)] out IShellItemImageFactory ppv);

    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    private static extern IntPtr SHGetFileInfo(string pszPath, uint dwFileAttributes, ref SHFILEINFO psfi, uint cbFileInfo, uint uFlags);

    [DllImport("shell32.dll", EntryPoint = "#727")]
    private static extern int SHGetImageList(int iImageList, ref Guid riid, out IImageList ppv);

    [DllImport("user32.dll")]
    private static extern bool DestroyIcon(IntPtr hIcon);

    [DllImport("gdi32.dll")]
    private static extern bool DeleteObject(IntPtr hObject);

    [ComImport]
    [Guid("bcc18b79-ba16-442f-80c4-8a59c30c463b")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IShellItemImageFactory
    {
        void GetImage(SIZE size, int flags, out IntPtr phbm);
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct SIZE
    {
        public int cx;
        public int cy;
    }

    private const int SIIGBF_RESIZETOFIT = 0x00;
    private const int SIIGBF_BIGGERSIZEOK = 0x01;
    private const int SIIGBF_ICONONLY = 0x04;

    private static Bitmap BitmapFromIconHandle(IntPtr iconHandle)
    {
        if (iconHandle == IntPtr.Zero) throw new InvalidOperationException("No icon handle was returned.");
        try
        {
            using (Icon icon = Icon.FromHandle(iconHandle))
            {
                return icon.ToBitmap();
            }
        }
        finally
        {
            DestroyIcon(iconHandle);
        }
    }

    private static Bitmap GetShellItemBitmap(string path)
    {
        Guid guid = typeof(IShellItemImageFactory).GUID;
        IShellItemImageFactory factory;
        SHCreateItemFromParsingName(path, IntPtr.Zero, ref guid, out factory);
        IntPtr bitmapHandle;
        factory.GetImage(new SIZE { cx = 256, cy = 256 }, SIIGBF_BIGGERSIZEOK | SIIGBF_ICONONLY | SIIGBF_RESIZETOFIT, out bitmapHandle);
        if (bitmapHandle == IntPtr.Zero) throw new InvalidOperationException("No shell item bitmap was found.");

        try
        {
            using (Bitmap bitmap = Image.FromHbitmap(bitmapHandle))
            {
                Bitmap clone = new Bitmap(bitmap.Width, bitmap.Height, System.Drawing.Imaging.PixelFormat.Format32bppArgb);
                using (Graphics graphics = Graphics.FromImage(clone))
                {
                    graphics.Clear(Color.Transparent);
                    graphics.DrawImage(bitmap, 0, 0, bitmap.Width, bitmap.Height);
                }
                return clone;
            }
        }
        finally
        {
            DeleteObject(bitmapHandle);
        }
    }

    private static Bitmap GetResourceIcon(string path, int index)
    {
        IntPtr[] handles = new IntPtr[1];
        int[] ids = new int[1];
        int count = PrivateExtractIcons(path, index, 256, 256, handles, ids, 1, 0);
        if (count <= 0 || handles[0] == IntPtr.Zero)
        {
            throw new InvalidOperationException("No resource icon was found.");
        }
        return BitmapFromIconHandle(handles[0]);
    }

    private static Bitmap GetShellIcon(string path)
    {
        const uint SHGFI_SYSICONINDEX = 0x000004000;
        const uint SHGFI_LARGEICON = 0x000000000;
        const int SHIL_JUMBO = 0x4;
        const int ILD_TRANSPARENT = 0x1;

        SHFILEINFO info = new SHFILEINFO();
        SHGetFileInfo(path, 0, ref info, (uint)Marshal.SizeOf(info), SHGFI_SYSICONINDEX | SHGFI_LARGEICON);

        Guid imageListGuid = typeof(IImageList).GUID;
        IImageList imageList;
        int result = SHGetImageList(SHIL_JUMBO, ref imageListGuid, out imageList);
        if (result != 0) throw new InvalidOperationException("Unable to access the Windows jumbo icon list.");

        IntPtr iconHandle = IntPtr.Zero;
        imageList.GetIcon(info.iIcon, ILD_TRANSPARENT, ref iconHandle);
        return BitmapFromIconHandle(iconHandle);
    }

    public static Bitmap Get(string iconPath, int index, string targetPath, string originalPath)
    {
        try
        {
            return GetResourceIcon(iconPath, index);
        }
        catch
        {
            try
            {
                return GetShellItemBitmap(iconPath);
            }
            catch
            {
                try
                {
                    return GetShellIcon(iconPath);
                }
                catch
                {
                    try
                    {
                        return GetShellItemBitmap(targetPath);
                    }
                    catch
                    {
                        return GetShellItemBitmap(originalPath);
                    }
                }
            }
        }
    }
}
"@

$iconSource = Resolve-IconSource -Path $source
Add-Type -TypeDefinition $code -ReferencedAssemblies System.Drawing
$bitmap = [PetDrawerIconExtractor]::Get($iconSource.IconPath, $iconSource.Index, $iconSource.TargetPath, $iconSource.OriginalPath)
try {
    $bitmap.Save($target, [System.Drawing.Imaging.ImageFormat]::Png)
}
finally {
    $bitmap.Dispose()
}
"##;
    let command = format!("& {{\n{script}\n}}");

    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &command,
        ])
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

fn pet_animation_extension(source: &Path) -> Result<String, String> {
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())
        .ok_or_else(|| "动画文件缺少扩展名".to_string())?;

    if !matches!(
        extension.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "ico" | "webm" | "mp4"
    ) {
        return Err("仅支持 png、jpg、jpeg、webp、gif、ico、webm、mp4 动画素材".to_string());
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

fn image_mime_for_extension(extension: &str) -> Result<&'static str, String> {
    match extension {
        "png" => Ok("image/png"),
        "jpg" | "jpeg" => Ok("image/jpeg"),
        "webp" => Ok("image/webp"),
        "gif" => Ok("image/gif"),
        "ico" => Ok("image/x-icon"),
        _ => Err("不支持的图片类型".to_string()),
    }
}

fn pet_animation_mime_for_extension(extension: &str) -> Result<&'static str, String> {
    match extension {
        "png" => Ok("image/png"),
        "jpg" | "jpeg" => Ok("image/jpeg"),
        "webp" => Ok("image/webp"),
        "gif" => Ok("image/gif"),
        "ico" => Ok("image/x-icon"),
        "webm" => Ok("video/webm"),
        "mp4" => Ok("video/mp4"),
        _ => Err("不支持的动画素材类型".to_string()),
    }
}

fn apps_file(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join(APPS_FILE_NAME))
}

fn config_file(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join(CONFIG_FILE_NAME))
}

fn skins_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(pet_assets_dir(app)?.join("skins"))
}

pub fn memory_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(storage_paths(app)?.memory_dir)
}

pub fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(storage_paths(app)?.data_dir)
}

pub fn pet_assets_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(storage_paths(app)?.pet_assets_dir)
}

pub fn icons_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(storage_paths(app)?.icons_dir)
}

pub fn default_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|err| format!("无法获取应用数据目录：{err}"))
}

pub fn read_storage_settings(app: &AppHandle) -> Result<StorageSettings, String> {
    let path = storage_settings_file(app)?;
    if !path.is_file() {
        return Ok(StorageSettings::default());
    }

    let content =
        fs::read_to_string(&path).map_err(|err| format!("读取存储目录配置失败：{err}"))?;
    if content.trim().is_empty() {
        return Ok(StorageSettings::default());
    }

    let settings: StorageSettings =
        serde_json::from_str(&content).map_err(|err| format!("storage.json 格式错误：{err}"))?;
    normalize_storage_settings_for_read(&settings)
}

pub fn save_storage_settings(
    app: &AppHandle,
    settings: &StorageSettings,
) -> Result<StorageSettings, String> {
    let old_paths = storage_paths(app)?;
    let normalized = normalize_storage_settings(settings)?;
    let new_paths = storage_paths_from_settings(app, &normalized)?;

    ensure_storage_dirs(&new_paths)?;
    migrate_storage_paths(&old_paths, &new_paths)?;

    let path = storage_settings_file(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("创建存储目录配置失败：{err}"))?;
    }
    let content = serde_json::to_string_pretty(&normalized).map_err(|err| err.to_string())?;
    fs::write(path, content).map_err(|err| format!("写入存储目录配置失败：{err}"))?;
    ensure_data_files(app)?;
    cleanup_old_storage_paths(&old_paths, &new_paths)
        .map_err(|err| format!("存储目录已切换，但清理旧数据失败：{err}"))?;

    Ok(normalized)
}

pub fn effective_storage_dirs(app: &AppHandle) -> Result<EffectiveStorageDirs, String> {
    let paths = storage_paths(app)?;
    Ok(EffectiveStorageDirs {
        default_data_dir: path_to_string(paths.default_data_dir),
        data_dir: path_to_string(paths.data_dir),
        memory_dir: path_to_string(paths.memory_dir),
        pet_assets_dir: path_to_string(paths.pet_assets_dir),
        icons_dir: path_to_string(paths.icons_dir),
        storage_config_file: path_to_string(paths.storage_config_file),
    })
}

fn storage_paths(app: &AppHandle) -> Result<StoragePaths, String> {
    let settings = read_storage_settings(app)?;
    storage_paths_from_settings(app, &settings)
}

fn storage_paths_from_settings(
    app: &AppHandle,
    settings: &StorageSettings,
) -> Result<StoragePaths, String> {
    let default_data_dir = default_data_dir(app)?;
    let storage_config_file = default_data_dir.join(STORAGE_SETTINGS_FILE_NAME);
    let data_dir = configured_dir(&settings.data_dir, "基础数据目录", default_data_dir.clone())?;
    let memory_dir = configured_dir(&settings.memory_dir, "记忆目录", data_dir.clone())?;
    let pet_assets_dir = configured_dir(
        &settings.pet_assets_dir,
        "宠物素材目录",
        data_dir.join("pets"),
    )?;
    let icons_dir = configured_dir(&settings.icons_dir, "图标目录", data_dir.join("icons"))?;

    Ok(StoragePaths {
        default_data_dir,
        data_dir,
        memory_dir,
        pet_assets_dir,
        icons_dir,
        storage_config_file,
    })
}

fn normalize_storage_settings(settings: &StorageSettings) -> Result<StorageSettings, String> {
    let data_dir = normalize_storage_dir_value(&settings.data_dir, "基础数据目录")?;
    let normalized = StorageSettings {
        memory_dir: normalize_storage_dir_value(&settings.memory_dir, "记忆目录")?,
        pet_assets_dir: normalize_pet_assets_dir_value(&settings.pet_assets_dir, &data_dir)?,
        icons_dir: normalize_dependent_storage_dir_value(
            &settings.icons_dir,
            &data_dir,
            "图标目录",
            "icons",
        )?,
        data_dir,
    };

    storage_paths_from_settings_placeholder(&normalized)?;
    Ok(normalized)
}

fn normalize_storage_settings_for_read(
    settings: &StorageSettings,
) -> Result<StorageSettings, String> {
    let normalized = StorageSettings {
        data_dir: normalize_storage_dir_value(&settings.data_dir, "基础数据目录")?,
        memory_dir: normalize_storage_dir_value(&settings.memory_dir, "记忆目录")?,
        pet_assets_dir: normalize_storage_dir_value(&settings.pet_assets_dir, "宠物素材目录")?,
        icons_dir: normalize_storage_dir_value(&settings.icons_dir, "图标目录")?,
    };

    storage_paths_from_settings_placeholder(&normalized)?;
    Ok(normalized)
}

fn normalize_pet_assets_dir_value(value: &str, data_dir: &str) -> Result<String, String> {
    let value = value.trim();
    let corrected = if value.is_empty() {
        String::new()
    } else {
        let path = Path::new(value);
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("skins"))
        {
            path.parent()
                .map(|path| path_to_string(path.to_path_buf()))
                .unwrap_or_else(|| value.to_string())
        } else {
            value.to_string()
        }
    };

    normalize_dependent_storage_dir_value(&corrected, data_dir, "宠物素材目录", "pets")
}

fn normalize_dependent_storage_dir_value(
    value: &str,
    data_dir: &str,
    label: &str,
    default_child: &str,
) -> Result<String, String> {
    let normalized = normalize_storage_dir_value(value, label)?;
    if normalized.is_empty() || data_dir.trim().is_empty() {
        return Ok(normalized);
    }

    let path = Path::new(&normalized);
    let data_path = Path::new(data_dir);
    if path_matches(path, data_path) || path_matches(path, &data_path.join(default_child)) {
        return Ok(String::new());
    }

    Ok(normalized)
}

fn storage_paths_from_settings_placeholder(settings: &StorageSettings) -> Result<(), String> {
    for (label, value) in [
        ("基础数据目录", &settings.data_dir),
        ("记忆目录", &settings.memory_dir),
        ("宠物素材目录", &settings.pet_assets_dir),
        ("图标目录", &settings.icons_dir),
    ] {
        if value.trim().is_empty() {
            continue;
        }
        let path = Path::new(value);
        if path.exists() && !path.is_dir() {
            return Err(format!("{label} 指向的不是目录"));
        }
    }
    Ok(())
}

fn normalize_storage_dir_value(value: &str, label: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(String::new());
    }

    let path = Path::new(value);
    if !path.is_absolute() {
        return Err(format!("{label} 必须填写绝对路径，或留空使用默认目录"));
    }
    if path.exists() && !path.is_dir() {
        return Err(format!("{label} 指向的不是目录"));
    }

    Ok(path_to_string(path.to_path_buf()))
}

fn configured_dir(value: &str, label: &str, fallback: PathBuf) -> Result<PathBuf, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(fallback);
    }

    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(format!("{label} 必须填写绝对路径，或留空使用默认目录"));
    }
    if path.exists() && !path.is_dir() {
        return Err(format!("{label} 指向的不是目录"));
    }

    Ok(path)
}

fn storage_settings_file(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(default_data_dir(app)?.join(STORAGE_SETTINGS_FILE_NAME))
}

fn ensure_storage_dirs(paths: &StoragePaths) -> Result<(), String> {
    for (label, path) in [
        ("基础数据目录", &paths.data_dir),
        ("记忆目录", &paths.memory_dir),
        ("宠物素材目录", &paths.pet_assets_dir),
        ("图标目录", &paths.icons_dir),
    ] {
        fs::create_dir_all(path).map_err(|err| format!("创建{label}失败：{err}"))?;
    }
    Ok(())
}

fn migrate_storage_paths(old: &StoragePaths, new: &StoragePaths) -> Result<(), String> {
    copy_missing_file(
        &old.data_dir.join(CONFIG_FILE_NAME),
        &new.data_dir.join(CONFIG_FILE_NAME),
        "配置文件",
    )?;
    copy_missing_file(
        &old.data_dir.join(APPS_FILE_NAME),
        &new.data_dir.join(APPS_FILE_NAME),
        "快捷入口数据",
    )?;
    copy_memory_files_missing(&old.memory_dir, &new.memory_dir)?;
    copy_pet_assets_missing(&old.pet_assets_dir, &new.pet_assets_dir)?;
    copy_icon_assets_missing(&old.icons_dir, &new.icons_dir)?;
    Ok(())
}

fn cleanup_old_storage_paths(old: &StoragePaths, new: &StoragePaths) -> Result<(), String> {
    remove_migrated_file(
        &old.data_dir.join(CONFIG_FILE_NAME),
        &new.data_dir.join(CONFIG_FILE_NAME),
        "配置文件",
    )?;
    remove_migrated_file(
        &old.data_dir.join(APPS_FILE_NAME),
        &new.data_dir.join(APPS_FILE_NAME),
        "快捷入口数据",
    )?;
    remove_migrated_memory_files(&old.memory_dir, &new.memory_dir)?;
    remove_migrated_pet_assets(&old.pet_assets_dir, &new.pet_assets_dir)?;
    remove_migrated_icon_assets(&old.icons_dir, &new.icons_dir)?;
    remove_empty_storage_dir(&old.icons_dir)?;
    remove_empty_storage_dir(&old.pet_assets_dir)?;
    remove_empty_storage_dir(&old.memory_dir)?;
    remove_empty_data_dir(old)?;
    Ok(())
}

fn copy_pet_assets_missing(source: &Path, target: &Path) -> Result<(), String> {
    if path_matches(source, target) || !source.is_dir() {
        return Ok(());
    }

    copy_matching_files_missing(source, target, "宠物图片", is_pet_asset_file)?;
    copy_dir_contents_missing(&source.join("skins"), &target.join("skins"), "宠物皮肤目录")
}

fn copy_icon_assets_missing(source: &Path, target: &Path) -> Result<(), String> {
    if path_matches(source, target) || !source.is_dir() {
        return Ok(());
    }

    copy_matching_files_missing(source, target, "图标文件", is_icon_asset_file)
}

fn copy_missing_file(source: &Path, target: &Path, label: &str) -> Result<(), String> {
    if path_matches(source, target) || !source.is_file() || target.exists() {
        return Ok(());
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("创建{label}目标目录失败：{err}"))?;
    }
    fs::copy(source, target).map_err(|err| format!("迁移{label}失败：{err}"))?;
    Ok(())
}

fn copy_dir_contents_missing(source: &Path, target: &Path, label: &str) -> Result<(), String> {
    if path_matches(source, target) || !source.is_dir() {
        return Ok(());
    }
    if target.starts_with(source) {
        return Err(format!(
            "{label}的新目录不能位于原目录内部，请选择同级或其他位置"
        ));
    }

    fs::create_dir_all(target).map_err(|err| format!("创建{label}目标目录失败：{err}"))?;
    for entry in fs::read_dir(source).map_err(|err| format!("读取{label}失败：{err}"))? {
        let entry = entry.map_err(|err| format!("读取{label}失败：{err}"))?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_contents_missing(&source_path, &target_path, label)?;
        } else if source_path.is_file() && !target_path.exists() {
            fs::copy(&source_path, &target_path)
                .map_err(|err| format!("迁移{label}失败：{err}"))?;
        }
    }
    Ok(())
}

fn copy_matching_files_missing(
    source: &Path,
    target: &Path,
    label: &str,
    matches_file: fn(&Path) -> bool,
) -> Result<(), String> {
    if path_matches(source, target) || !source.is_dir() {
        return Ok(());
    }

    fs::create_dir_all(target).map_err(|err| format!("创建{label}目标目录失败：{err}"))?;
    for entry in fs::read_dir(source).map_err(|err| format!("读取{label}失败：{err}"))? {
        let entry = entry.map_err(|err| format!("读取{label}失败：{err}"))?;
        let source_path = entry.path();
        if !source_path.is_file() || !matches_file(&source_path) {
            continue;
        }

        let target_path = target.join(entry.file_name());
        if !target_path.exists() {
            fs::copy(&source_path, &target_path)
                .map_err(|err| format!("迁移{label}失败：{err}"))?;
        }
    }

    Ok(())
}

fn copy_memory_files_missing(source: &Path, target: &Path) -> Result<(), String> {
    if path_matches(source, target) || !source.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(target).map_err(|err| format!("创建记忆目录失败：{err}"))?;

    for file_name in [
        "pet-memory.db",
        "pet-memory.db-wal",
        "pet-memory.db-shm",
        "pet-memory.json",
        "story-saves.json",
    ] {
        copy_missing_file(
            &source.join(file_name),
            &target.join(file_name),
            "记忆数据文件",
        )?;
    }

    Ok(())
}

fn remove_migrated_memory_files(source: &Path, target: &Path) -> Result<(), String> {
    if path_matches(source, target) || !source.is_dir() {
        return Ok(());
    }

    for file_name in [
        "pet-memory.db",
        "pet-memory.db-wal",
        "pet-memory.db-shm",
        "pet-memory.json",
        "story-saves.json",
    ] {
        remove_migrated_file(
            &source.join(file_name),
            &target.join(file_name),
            "记忆数据文件",
        )?;
    }

    remove_empty_storage_dir(source)
}

fn remove_migrated_pet_assets(source: &Path, target: &Path) -> Result<(), String> {
    if path_matches(source, target) || !source.is_dir() {
        return Ok(());
    }

    remove_migrated_matching_files(source, target, "宠物图片", is_pet_asset_file)?;
    remove_migrated_dir_contents(&source.join("skins"), &target.join("skins"), "宠物皮肤目录")?;
    remove_empty_storage_dir(source)
}

fn remove_migrated_icon_assets(source: &Path, target: &Path) -> Result<(), String> {
    if path_matches(source, target) || !source.is_dir() {
        return Ok(());
    }

    remove_migrated_matching_files(source, target, "图标文件", is_icon_asset_file)?;
    remove_empty_storage_dir(source)
}

fn remove_migrated_dir_contents(source: &Path, target: &Path, label: &str) -> Result<(), String> {
    if path_matches(source, target) || !source.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(source).map_err(|err| format!("读取旧{label}失败：{err}"))? {
        let entry = entry.map_err(|err| format!("读取旧{label}失败：{err}"))?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            remove_migrated_dir_contents(&source_path, &target_path, label)?;
        } else if source_path.is_file() {
            remove_migrated_file(&source_path, &target_path, label)?;
        }
    }

    remove_empty_storage_dir(source)
}

fn remove_migrated_matching_files(
    source: &Path,
    target: &Path,
    label: &str,
    matches_file: fn(&Path) -> bool,
) -> Result<(), String> {
    if path_matches(source, target) || !source.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(source).map_err(|err| format!("读取旧{label}失败：{err}"))? {
        let entry = entry.map_err(|err| format!("读取旧{label}失败：{err}"))?;
        let source_path = entry.path();
        if !source_path.is_file() || !matches_file(&source_path) {
            continue;
        }

        remove_migrated_file(&source_path, &target.join(entry.file_name()), label)?;
    }

    Ok(())
}

fn remove_migrated_file(source: &Path, target: &Path, label: &str) -> Result<(), String> {
    if path_matches(source, target) || !source.is_file() {
        return Ok(());
    }

    if !target.is_file() {
        return Err(format!(
            "{label}尚未迁移到新位置，已保留旧文件 {}",
            source.display()
        ));
    }

    if !files_have_same_content(source, target)? {
        return Err(format!(
            "{label}在新旧位置内容不同，已保留旧文件 {}",
            source.display()
        ));
    }

    fs::remove_file(source).map_err(|err| format!("删除旧{label}失败：{err}"))
}

fn files_have_same_content(left: &Path, right: &Path) -> Result<bool, String> {
    let left_bytes = fs::read(left).map_err(|err| format!("读取旧文件失败：{err}"))?;
    let right_bytes = fs::read(right).map_err(|err| format!("读取新文件失败：{err}"))?;
    Ok(left_bytes == right_bytes)
}

fn remove_empty_storage_dir(path: &Path) -> Result<(), String> {
    if !path.is_dir() {
        return Ok(());
    }

    match fs::remove_dir(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::DirectoryNotEmpty => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("删除旧空目录失败：{err}")),
    }
}

fn remove_empty_data_dir(paths: &StoragePaths) -> Result<(), String> {
    if path_matches(&paths.data_dir, &paths.default_data_dir) {
        return Ok(());
    }

    remove_empty_storage_dir(&paths.data_dir)
}

fn is_pet_asset_file(path: &Path) -> bool {
    file_name_starts_with(path, "pet_")
        && has_extension(path, &["png", "jpg", "jpeg", "webp", "gif", "ico"])
}

fn is_icon_asset_file(path: &Path) -> bool {
    (file_name_starts_with(path, "icon_") || file_name_starts_with(path, "auto_icon_"))
        && has_extension(path, &["png", "jpg", "jpeg", "webp", "gif", "ico"])
}

fn file_name_starts_with(path: &Path, prefix: &str) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.starts_with(prefix))
}

fn has_extension(path: &Path, allowed: &[&str]) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())
        .is_some_and(|value| allowed.contains(&value.as_str()))
}

fn path_matches(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }

    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn resolve_stored_file_path(app: &AppHandle, relative_path: &str) -> Result<PathBuf, String> {
    let relative = safe_relative_path(relative_path)?;
    resolve_safe_relative_path(app, &relative)
}

fn resolve_safe_relative_path(app: &AppHandle, relative: &Path) -> Result<PathBuf, String> {
    let mut components = relative.components();
    let Some(first) = components.next() else {
        return Err("图片路径不能为空".to_string());
    };

    let mut target = match first {
        Component::Normal(value) if value == "icons" => icons_dir(app)?,
        Component::Normal(value) if value == "pets" => pet_assets_dir(app)?,
        Component::Normal(value) => {
            let mut base = data_dir(app)?;
            base.push(value);
            base
        }
        _ => return Err("图片路径不合法".to_string()),
    };

    for component in components {
        match component {
            Component::Normal(value) => target.push(value),
            _ => return Err("图片路径不合法".to_string()),
        }
    }

    Ok(target)
}

fn path_to_string(path: PathBuf) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "pet_drawer_{name}_{}_{}",
            std::process::id(),
            now_millis()
        ));
        fs::create_dir_all(&dir).expect("create temp test dir");
        dir
    }

    #[test]
    fn atomic_write_file_replaces_existing_file() {
        let dir = temp_test_dir("atomic_write");
        let path = dir.join("config.json");
        fs::write(&path, br#"{"old":true}"#).expect("write old file");

        atomic_write_file(&path, br#"{"new":true}"#).expect("atomic write");

        assert_eq!(
            fs::read(&path).expect("read replaced file"),
            br#"{"new":true}"#
        );
        assert!(fs::read_dir(&dir)
            .expect("read temp dir")
            .all(|entry| !entry
                .expect("read entry")
                .file_name()
                .to_string_lossy()
                .contains(".tmp")));

        fs::remove_dir_all(dir).expect("remove temp test dir");
    }

    #[test]
    fn backup_corrupt_config_file_preserves_original_content() {
        let dir = temp_test_dir("backup_corrupt_config");
        let path = dir.join("config.json");
        let corrupt_content = vec![0_u8; 32];
        fs::write(&path, &corrupt_content).expect("write corrupt file");

        let backup_path = backup_corrupt_config_file(&path).expect("backup corrupt config");

        assert!(backup_path.exists());
        assert_eq!(
            fs::read(backup_path).expect("read backup file"),
            corrupt_content
        );
        assert_eq!(fs::read(&path).expect("read original file"), vec![0_u8; 32]);

        fs::remove_dir_all(dir).expect("remove temp test dir");
    }

    #[test]
    fn cleanup_old_storage_paths_removes_migrated_app_data() {
        let dir = temp_test_dir("cleanup_migrated_storage");
        let old_data = dir.join("old");
        let new_data = dir.join("new");
        let old_pets = old_data.join("pets");
        let new_pets = new_data.join("pets");
        let old_icons = old_data.join("icons");
        let new_icons = new_data.join("icons");

        fs::create_dir_all(old_pets.join("skins").join("cat")).expect("create old pets");
        fs::create_dir_all(new_pets.join("skins").join("cat")).expect("create new pets");
        fs::create_dir_all(&old_icons).expect("create old icons");
        fs::create_dir_all(&new_icons).expect("create new icons");
        fs::write(old_data.join(CONFIG_FILE_NAME), b"config").expect("write old config");
        fs::write(new_data.join(CONFIG_FILE_NAME), b"config").expect("write new config");
        fs::write(old_data.join(APPS_FILE_NAME), b"apps").expect("write old apps");
        fs::write(new_data.join(APPS_FILE_NAME), b"apps").expect("write new apps");
        fs::write(old_data.join("pet-memory.db"), b"memory").expect("write old memory");
        fs::write(new_data.join("pet-memory.db"), b"memory").expect("write new memory");
        fs::write(old_pets.join("skins").join("cat").join("idle.png"), b"pet")
            .expect("write old pet");
        fs::write(new_pets.join("skins").join("cat").join("idle.png"), b"pet")
            .expect("write new pet");
        fs::write(old_icons.join("icon_1.ico"), b"icon").expect("write old icon");
        fs::write(new_icons.join("icon_1.ico"), b"icon").expect("write new icon");
        fs::write(old_data.join(STORAGE_SETTINGS_FILE_NAME), b"storage")
            .expect("write storage settings");

        let old_paths = StoragePaths {
            default_data_dir: old_data.clone(),
            data_dir: old_data.clone(),
            memory_dir: old_data.clone(),
            pet_assets_dir: old_pets,
            icons_dir: old_icons,
            storage_config_file: old_data.join(STORAGE_SETTINGS_FILE_NAME),
        };
        let new_paths = StoragePaths {
            default_data_dir: old_data.clone(),
            data_dir: new_data.clone(),
            memory_dir: new_data.clone(),
            pet_assets_dir: new_pets,
            icons_dir: new_icons,
            storage_config_file: old_data.join(STORAGE_SETTINGS_FILE_NAME),
        };

        cleanup_old_storage_paths(&old_paths, &new_paths).expect("cleanup old storage");

        assert!(!old_data.join(CONFIG_FILE_NAME).exists());
        assert!(!old_data.join(APPS_FILE_NAME).exists());
        assert!(!old_data.join("pet-memory.db").exists());
        assert!(!old_paths.pet_assets_dir.exists());
        assert!(!old_paths.icons_dir.exists());
        assert!(old_data.join(STORAGE_SETTINGS_FILE_NAME).exists());
        assert!(new_data.join(CONFIG_FILE_NAME).exists());
        assert!(new_data.join(APPS_FILE_NAME).exists());
        assert!(new_data.join("pet-memory.db").exists());

        fs::remove_dir_all(dir).expect("remove temp test dir");
    }

    #[test]
    fn cleanup_old_storage_paths_keeps_old_file_when_target_differs() {
        let dir = temp_test_dir("cleanup_target_differs");
        let old_data = dir.join("old");
        let new_data = dir.join("new");
        fs::create_dir_all(&old_data).expect("create old data");
        fs::create_dir_all(&new_data).expect("create new data");
        fs::write(old_data.join(CONFIG_FILE_NAME), b"old config").expect("write old config");
        fs::write(new_data.join(CONFIG_FILE_NAME), b"new config").expect("write new config");

        let old_paths = StoragePaths {
            default_data_dir: old_data.clone(),
            data_dir: old_data.clone(),
            memory_dir: old_data.clone(),
            pet_assets_dir: old_data.join("pets"),
            icons_dir: old_data.join("icons"),
            storage_config_file: old_data.join(STORAGE_SETTINGS_FILE_NAME),
        };
        let new_paths = StoragePaths {
            default_data_dir: old_data.clone(),
            data_dir: new_data,
            memory_dir: dir.join("new-memory"),
            pet_assets_dir: dir.join("new-pets"),
            icons_dir: dir.join("new-icons"),
            storage_config_file: old_data.join(STORAGE_SETTINGS_FILE_NAME),
        };

        let error =
            cleanup_old_storage_paths(&old_paths, &new_paths).expect_err("cleanup should fail");

        assert!(error.contains("内容不同"));
        assert!(old_data.join(CONFIG_FILE_NAME).exists());

        fs::remove_dir_all(dir).expect("remove temp test dir");
    }

    #[test]
    fn migrate_storage_paths_filters_mixed_asset_directories() {
        let dir = temp_test_dir("migrate_filters_assets");
        let old_data = dir.join("old");
        let new_data = dir.join("new");
        let old_pets = old_data.join("pets");
        let new_pets = new_data.join("pets");
        let old_icons = old_data.join("icons");
        let new_icons = new_data.join("icons");

        fs::create_dir_all(old_pets.join("skins").join("cat")).expect("create old pet skin");
        fs::create_dir_all(&old_icons).expect("create old icons");
        fs::write(old_data.join(CONFIG_FILE_NAME), b"config").expect("write old config");
        fs::write(old_data.join(APPS_FILE_NAME), b"apps").expect("write old apps");
        fs::write(old_pets.join("config.json"), b"wrong pet config")
            .expect("write mixed pet config");
        fs::write(old_pets.join("pet_1.png"), b"pet").expect("write pet image");
        fs::write(
            old_pets.join("skins").join("cat").join("manifest.json"),
            b"skin",
        )
        .expect("write skin manifest");
        fs::write(old_icons.join("config.json"), b"wrong icon config")
            .expect("write mixed icon config");
        fs::write(old_icons.join("icon_1.ico"), b"icon").expect("write icon image");

        let old_paths = StoragePaths {
            default_data_dir: old_data.clone(),
            data_dir: old_data.clone(),
            memory_dir: old_data.clone(),
            pet_assets_dir: old_pets,
            icons_dir: old_icons,
            storage_config_file: old_data.join(STORAGE_SETTINGS_FILE_NAME),
        };
        let new_paths = StoragePaths {
            default_data_dir: old_data,
            data_dir: new_data.clone(),
            memory_dir: new_data.clone(),
            pet_assets_dir: new_pets,
            icons_dir: new_icons,
            storage_config_file: dir.join("storage.json"),
        };

        migrate_storage_paths(&old_paths, &new_paths).expect("migrate storage");

        assert!(new_data.join(CONFIG_FILE_NAME).exists());
        assert!(new_data.join(APPS_FILE_NAME).exists());
        assert!(new_paths.pet_assets_dir.join("pet_1.png").exists());
        assert!(new_paths
            .pet_assets_dir
            .join("skins")
            .join("cat")
            .join("manifest.json")
            .exists());
        assert!(new_paths.icons_dir.join("icon_1.ico").exists());
        assert!(!new_paths.pet_assets_dir.join("config.json").exists());
        assert!(!new_paths.icons_dir.join("config.json").exists());

        fs::remove_dir_all(dir).expect("remove temp test dir");
    }

    #[test]
    fn normalize_storage_settings_uses_child_dirs_for_asset_roots() {
        let dir = temp_test_dir("normalize_storage_dirs");
        let data_dir = dir.join("data");
        let data_dir_string = path_to_string(data_dir.clone());
        fs::create_dir_all(data_dir.join("skins")).expect("create skins dir");

        let settings = StorageSettings {
            data_dir: data_dir_string.clone(),
            memory_dir: data_dir_string.clone(),
            pet_assets_dir: data_dir_string.clone(),
            icons_dir: data_dir.join("icons").to_string_lossy().to_string(),
        };

        let normalized = normalize_storage_settings(&settings).expect("normalize settings");

        assert_eq!(normalized.data_dir, data_dir_string);
        assert_eq!(normalized.memory_dir, path_to_string(data_dir.clone()));
        assert_eq!(normalized.pet_assets_dir, "");
        assert_eq!(normalized.icons_dir, "");

        let settings = StorageSettings {
            data_dir: path_to_string(data_dir.clone()),
            memory_dir: String::new(),
            pet_assets_dir: data_dir.join("skins").to_string_lossy().to_string(),
            icons_dir: String::new(),
        };

        let normalized = normalize_storage_settings(&settings).expect("normalize skin path");
        assert_eq!(normalized.pet_assets_dir, "");

        fs::remove_dir_all(dir).expect("remove temp test dir");
    }
}
