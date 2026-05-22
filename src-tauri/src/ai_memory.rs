use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::app_data;

const MEMORY_FILE_NAME: &str = "pet-memory.json";
const MAX_MESSAGES: usize = 200;
const MAX_MESSAGE_CONTENT_LEN: usize = 1200;
const MAX_MEMORY_CONTENT_LEN: usize = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetMemoryMessage {
    pub id: u64,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetMemory {
    pub id: u64,
    #[serde(default = "default_memory_type")]
    pub memory_type: String,
    pub content: String,
    #[serde(default = "default_importance")]
    pub importance: u8,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub deleted: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetMemoryDraft {
    #[serde(default = "default_memory_type")]
    pub memory_type: String,
    pub content: String,
    #[serde(default = "default_importance")]
    pub importance: u8,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PetMemoryStore {
    #[serde(default)]
    messages: Vec<PetMemoryMessage>,
    #[serde(default)]
    memories: Vec<PetMemory>,
}

pub fn save_message(app: &AppHandle, role: &str, content: &str) -> Result<(), String> {
    let role = normalize_role(role).ok_or_else(|| "记忆消息角色不合法".to_string())?;
    let content = content.trim();
    if content.is_empty() {
        return Ok(());
    }
    let content = truncate_text(content, MAX_MESSAGE_CONTENT_LEN);

    let mut store = read_store(app)?;
    store.messages.push(PetMemoryMessage {
        id: now_millis(),
        role,
        content,
        created_at: app_data::now_seconds(),
    });

    if store.messages.len() > MAX_MESSAGES {
        let remove_count = store.messages.len() - MAX_MESSAGES;
        store.messages.drain(0..remove_count);
    }

    write_store(app, &store)
}

pub fn recent_messages(app: &AppHandle, limit: usize) -> Result<Vec<PetMemoryMessage>, String> {
    let store = read_store(app)?;
    let start = store.messages.len().saturating_sub(limit);
    Ok(store.messages[start..].to_vec())
}

pub fn list_memories(app: &AppHandle) -> Result<Vec<PetMemory>, String> {
    let mut memories = read_store(app)?
        .memories
        .into_iter()
        .filter(|memory| !memory.deleted)
        .collect::<Vec<_>>();

    memories.sort_by(|a, b| {
        b.importance
            .cmp(&a.importance)
            .then_with(|| b.updated_at.cmp(&a.updated_at))
    });

    Ok(memories)
}

pub fn save_memories(
    app: &AppHandle,
    drafts: Vec<PetMemoryDraft>,
) -> Result<Vec<PetMemory>, String> {
    let mut store = read_store(app)?;
    let mut saved = Vec::new();

    for draft in drafts {
        let Some(draft) = normalize_memory_draft(draft) else {
            continue;
        };

        let now = app_data::now_seconds();
        let duplicate_index = store.memories.iter().position(|memory| {
            !memory.deleted
                && memory.memory_type == draft.memory_type
                && same_memory_content(&memory.content, &draft.content)
        });

        if let Some(index) = duplicate_index {
            store.memories[index].content = draft.content;
            store.memories[index].importance =
                store.memories[index].importance.max(draft.importance);
            store.memories[index].tags = merge_tags(&store.memories[index].tags, &draft.tags);
            store.memories[index].updated_at = now;
            saved.push(store.memories[index].clone());
        } else {
            let memory = PetMemory {
                id: now_millis(),
                memory_type: draft.memory_type,
                content: draft.content,
                importance: draft.importance,
                tags: draft.tags,
                deleted: false,
                created_at: now.clone(),
                updated_at: now,
            };
            saved.push(memory.clone());
            store.memories.push(memory);
        }
    }

    if !saved.is_empty() {
        write_store(app, &store)?;
    }

    Ok(saved)
}

pub fn search_memories(
    app: &AppHandle,
    query: &str,
    limit: usize,
) -> Result<Vec<PetMemory>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let tokens = query_tokens(query);
    let mut scored = read_store(app)?
        .memories
        .into_iter()
        .filter(|memory| !memory.deleted)
        .filter_map(|memory| {
            let score = memory_score(&memory, &tokens, query);
            (score > 0).then_some((score, memory))
        })
        .collect::<Vec<_>>();

    scored.sort_by(|(left_score, left), (right_score, right)| {
        right_score
            .cmp(left_score)
            .then_with(|| right.importance.cmp(&left.importance))
            .then_with(|| right.updated_at.cmp(&left.updated_at))
    });

    Ok(scored
        .into_iter()
        .take(limit)
        .map(|(_, memory)| memory)
        .collect())
}

pub fn delete_memory(app: &AppHandle, memory_id: u64) -> Result<(), String> {
    let mut store = read_store(app)?;
    let memory = store
        .memories
        .iter_mut()
        .find(|memory| memory.id == memory_id && !memory.deleted)
        .ok_or_else(|| "未找到要删除的记忆".to_string())?;

    memory.deleted = true;
    memory.updated_at = app_data::now_seconds();
    write_store(app, &store)
}

pub fn clear_memories(app: &AppHandle) -> Result<(), String> {
    let mut store = read_store(app)?;
    let now = app_data::now_seconds();
    for memory in &mut store.memories {
        if !memory.deleted {
            memory.deleted = true;
            memory.updated_at = now.clone();
        }
    }

    write_store(app, &store)
}

pub fn import_memory_file(app: &AppHandle, source_path: &str) -> Result<Vec<PetMemory>, String> {
    let source_path = source_path.trim();
    if source_path.is_empty() {
        return Err("请选择要导入的记忆文件".to_string());
    }

    let source = Path::new(source_path);
    if !source.is_file() {
        return Err("记忆导入文件不存在".to_string());
    }

    let content =
        fs::read_to_string(source).map_err(|err| format!("读取记忆导入文件失败：{err}"))?;
    let mut store: PetMemoryStore =
        serde_json::from_str(&content).map_err(|err| format!("记忆导入文件格式错误：{err}"))?;
    normalize_imported_store(&mut store);
    write_store(app, &store)?;
    list_memories(app)
}

pub fn export_memory_file(app: &AppHandle, target_path: &str) -> Result<(), String> {
    let target_path = target_path.trim();
    if target_path.is_empty() {
        return Err("请选择记忆导出位置".to_string());
    }

    let target = Path::new(target_path);
    if target.is_dir() {
        return Err("请选择具体的 JSON 文件路径，而不是文件夹".to_string());
    }

    if let Some(parent) = target.parent() {
        if !parent.exists() {
            return Err("记忆导出目录不存在".to_string());
        }
    }

    let store = read_store(app)?;
    let content = serde_json::to_string_pretty(&store).map_err(|err| err.to_string())?;
    fs::write(target, content).map_err(|err| format!("导出宠物记忆失败：{err}"))
}

pub fn open_memory_dir(app: &AppHandle) -> Result<(), String> {
    app_data::ensure_data_files(app)?;
    let dir = memory_dir(app)?;
    fs::create_dir_all(&dir).map_err(|err| format!("创建记忆目录失败：{err}"))?;
    open_path(&dir)
}

fn read_store(app: &AppHandle) -> Result<PetMemoryStore, String> {
    app_data::ensure_data_files(app)?;
    let path = memory_file(app)?;
    if !path.exists() {
        return Ok(PetMemoryStore::default());
    }

    let content = fs::read_to_string(path).map_err(|err| format!("读取宠物记忆失败：{err}"))?;
    if content.trim().is_empty() {
        return Ok(PetMemoryStore::default());
    }

    serde_json::from_str(&content).map_err(|err| format!("宠物记忆文件格式错误：{err}"))
}

fn write_store(app: &AppHandle, store: &PetMemoryStore) -> Result<(), String> {
    app_data::ensure_data_files(app)?;
    let content = serde_json::to_string_pretty(store).map_err(|err| err.to_string())?;
    fs::write(memory_file(app)?, content).map_err(|err| format!("保存宠物记忆失败：{err}"))
}

fn memory_file(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(memory_dir(app)?.join(MEMORY_FILE_NAME))
}

fn memory_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|err| format!("无法获取应用数据目录：{err}"))?)
}

#[cfg(target_os = "windows")]
fn open_path(path: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    Command::new("explorer.exe")
        .arg(path)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("打开记忆目录失败：{err}"))
}

#[cfg(not(target_os = "windows"))]
fn open_path(path: &Path) -> Result<(), String> {
    Command::new("open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("打开记忆目录失败：{err}"))
}

fn normalize_role(role: &str) -> Option<String> {
    match role.trim().to_lowercase().as_str() {
        "user" => Some("user".to_string()),
        "assistant" => Some("assistant".to_string()),
        _ => None,
    }
}

fn normalize_memory_draft(draft: PetMemoryDraft) -> Option<PetMemoryDraft> {
    let content = draft.content.trim();
    if content.is_empty() {
        return None;
    }

    let content = truncate_text(content, MAX_MEMORY_CONTENT_LEN);

    Some(PetMemoryDraft {
        memory_type: normalize_memory_type(&draft.memory_type),
        content,
        importance: draft.importance.clamp(1, 10),
        tags: normalize_tags(draft.tags),
    })
}

fn normalize_memory_type(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "preference" | "project" | "event" | "profile" => value.trim().to_lowercase(),
        _ => default_memory_type(),
    }
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    tags.into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .filter(|tag| seen.insert(tag.to_lowercase()))
        .take(8)
        .collect()
}

fn merge_tags(existing: &[String], incoming: &[String]) -> Vec<String> {
    normalize_tags(
        existing
            .iter()
            .chain(incoming.iter())
            .cloned()
            .collect::<Vec<_>>(),
    )
}

fn normalize_imported_store(store: &mut PetMemoryStore) {
    if store.messages.len() > MAX_MESSAGES {
        let remove_count = store.messages.len() - MAX_MESSAGES;
        store.messages.drain(0..remove_count);
    }

    for message in &mut store.messages {
        message.role = normalize_role(&message.role).unwrap_or_else(|| "user".to_string());
        message.content = truncate_text(message.content.trim(), MAX_MESSAGE_CONTENT_LEN);
        if message.created_at.trim().is_empty() {
            message.created_at = app_data::now_seconds();
        }
    }

    store.messages.retain(|message| !message.content.is_empty());

    for memory in &mut store.memories {
        memory.memory_type = normalize_memory_type(&memory.memory_type);
        memory.content = truncate_text(memory.content.trim(), MAX_MEMORY_CONTENT_LEN);
        memory.importance = memory.importance.clamp(1, 10);
        memory.tags = normalize_tags(memory.tags.clone());
        if memory.created_at.trim().is_empty() {
            memory.created_at = app_data::now_seconds();
        }
        if memory.updated_at.trim().is_empty() {
            memory.updated_at = memory.created_at.clone();
        }
    }

    store.memories.retain(|memory| !memory.content.is_empty());
}

fn same_memory_content(left: &str, right: &str) -> bool {
    let left = compact_text(left);
    let right = compact_text(right);
    left == right || left.contains(&right) || right.contains(&left)
}

fn compact_text(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}

fn truncate_text(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    value.chars().take(max_chars).collect()
}

fn query_tokens(query: &str) -> Vec<String> {
    let mut tokens = HashSet::new();
    let mut current = String::new();

    for ch in query.chars() {
        if ch.is_alphanumeric() || is_cjk(ch) {
            current.push(ch);
        } else {
            push_query_token(&mut tokens, &current);
            current.clear();
        }
    }
    push_query_token(&mut tokens, &current);

    tokens.into_iter().collect()
}

fn push_query_token(tokens: &mut HashSet<String>, value: &str) {
    let value = value.trim().to_lowercase();
    if value.chars().count() >= 2 {
        tokens.insert(value.clone());
    }

    let chars = value.chars().collect::<Vec<_>>();
    if chars.iter().any(|ch| is_cjk(*ch)) {
        for size in 2..=4 {
            for window in chars.windows(size) {
                tokens.insert(window.iter().copied().collect());
            }
        }
    }
}

fn memory_score(memory: &PetMemory, tokens: &[String], query: &str) -> usize {
    let haystack = format!(
        "{} {} {}",
        memory.content,
        memory.memory_type,
        memory.tags.join(" ")
    )
    .to_lowercase();

    let mut score = 0;
    for token in tokens {
        if haystack.contains(token) {
            score += token.chars().count().max(2) * 2;
        }
    }

    let query_chars = query
        .chars()
        .filter(|ch| is_cjk(*ch))
        .collect::<HashSet<_>>();
    if !query_chars.is_empty() {
        score += haystack
            .chars()
            .filter(|ch| query_chars.contains(ch))
            .count()
            .min(12);
    }

    score
}

fn is_cjk(ch: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&ch)
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

fn default_memory_type() -> String {
    "event".to_string()
}

fn default_importance() -> u8 {
    5
}
