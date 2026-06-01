use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::app_data::{
    self, Companion, CompanionDraft, CompanionRelationshipState, PetDrawerConfig,
};

const MEMORY_DB_FILE_NAME: &str = "pet-memory.db";
const LEGACY_MEMORY_JSON_FILE_NAME: &str = "pet-memory.json";
pub const DEFAULT_COMPANION_ID: &str = "default";
pub const SHORT_TERM_SUMMARY_MEMORY_TYPE: &str = "short_term_summary";
pub const MAX_SHORT_TERM_SUMMARY_LEN: usize = 2400;
const MAX_MESSAGES: usize = 200;
const MAX_MESSAGE_CONTENT_LEN: usize = 1200;
const MAX_MEMORY_CONTENT_LEN: usize = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetMemoryMessage {
    pub id: u64,
    #[serde(default = "default_companion_id")]
    pub companion_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetMemory {
    pub id: u64,
    #[serde(default = "default_companion_id")]
    pub companion_id: String,
    #[serde(default = "default_memory_type")]
    pub memory_type: String,
    pub content: String,
    #[serde(default = "default_importance")]
    pub importance: u8,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source_message: String,
    #[serde(default = "default_confidence")]
    pub confidence: f32,
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
    #[serde(default)]
    pub source_message: String,
    #[serde(default = "default_confidence")]
    pub confidence: f32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PetMemoryStore {
    #[serde(default)]
    messages: Vec<PetMemoryMessage>,
    #[serde(default)]
    memories: Vec<PetMemory>,
}

fn default_companion(skin_id: &str) -> Companion {
    let now = app_data::now_seconds();
    Companion {
        id: DEFAULT_COMPANION_ID.to_string(),
        name: "凯蒂".to_string(),
        avatar: None,
        persona_prompt: "你是温柔、活泼的桌面伴侣凯蒂，陪用户聊天并提供恰当帮助。".to_string(),
        system_prompt: String::new(),
        model: String::new(),
        voice_id: String::new(),
        memory_scope: DEFAULT_COMPANION_ID.to_string(),
        skin_id: normalize_skin_id(skin_id),
        relationship_state: CompanionRelationshipState {
            favorability: 0,
            intimacy: 0,
            mood: String::new(),
        },
        created_at: now.clone(),
        updated_at: now,
    }
}

fn read_companion_config(app: &AppHandle) -> Result<PetDrawerConfig, String> {
    let mut config = app_data::read_config(app)?;
    let mut changed = false;
    let inherited_skin_id = normalize_skin_id(&config.pet.current_skin);

    if !config
        .companions
        .iter()
        .any(|companion| companion.id == DEFAULT_COMPANION_ID)
    {
        config
            .companions
            .insert(0, default_companion(&inherited_skin_id));
        changed = true;
    }

    if !config.companions_initialized {
        if let Some(companion) = config
            .companions
            .iter_mut()
            .find(|companion| companion.id == DEFAULT_COMPANION_ID)
        {
            companion.skin_id = inherited_skin_id.clone();
            companion.updated_at = app_data::now_seconds();
        }
        config.companions_initialized = true;
        changed = true;
    }

    if !config
        .companions
        .iter()
        .any(|companion| companion.id == config.current_companion_id)
    {
        config.current_companion_id = DEFAULT_COMPANION_ID.to_string();
        changed = true;
    }

    if changed {
        app_data::write_config(app, &config)?;
    }
    Ok(config)
}

pub fn list_companions(app: &AppHandle) -> Result<Vec<Companion>, String> {
    let mut config = read_companion_config(app)?;
    config.companions.sort_by(|left, right| {
        let left_rank = if left.id == DEFAULT_COMPANION_ID {
            0
        } else {
            1
        };
        let right_rank = if right.id == DEFAULT_COMPANION_ID {
            0
        } else {
            1
        };
        left_rank
            .cmp(&right_rank)
            .then_with(|| left.created_at.cmp(&right.created_at))
    });
    Ok(config.companions)
}

pub fn current_companion(app: &AppHandle) -> Result<Companion, String> {
    let config = read_companion_config(app)?;
    config
        .companions
        .iter()
        .find(|companion| companion.id == config.current_companion_id)
        .or_else(|| {
            config
                .companions
                .iter()
                .find(|companion| companion.id == DEFAULT_COMPANION_ID)
        })
        .cloned()
        .ok_or_else(|| "未找到可用伴侣档案".to_string())
}

pub fn current_companion_id(app: &AppHandle) -> Result<String, String> {
    Ok(current_companion(app)?.id)
}

pub fn upsert_companion(app: &AppHandle, draft: CompanionDraft) -> Result<Companion, String> {
    let mut config = read_companion_config(app)?;
    let name = draft.name.trim();
    let persona_prompt = draft.persona_prompt.trim();
    if name.is_empty() {
        return Err("请填写伴侣名称".to_string());
    }
    if persona_prompt.is_empty() {
        return Err("请填写伴侣角色设定".to_string());
    }

    let id = match draft
        .id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        Some(id) => {
            safe_companion_id(id)?;
            if !config.companions.iter().any(|companion| companion.id == id) {
                return Err("未找到要编辑的伴侣".to_string());
            }
            id.to_string()
        }
        None => format!("companion_{}", now_millis()),
    };
    let existing = config
        .companions
        .iter()
        .find(|companion| companion.id == id)
        .cloned();
    let now = app_data::now_seconds();
    let state = draft
        .relationship_state
        .unwrap_or(CompanionRelationshipState {
            favorability: 0,
            intimacy: 0,
            mood: String::new(),
        });
    let companion = Companion {
        id: id.clone(),
        name: truncate_text(name, 40),
        avatar: draft
            .avatar
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        persona_prompt: truncate_text(persona_prompt, 2000),
        system_prompt: truncate_text(draft.system_prompt.trim(), 2000),
        model: truncate_text(draft.model.trim(), 120),
        voice_id: truncate_text(draft.voice_id.trim(), 120),
        memory_scope: id.clone(),
        skin_id: normalize_skin_id(&draft.skin_id),
        relationship_state: CompanionRelationshipState {
            favorability: state.favorability.clamp(-9999, 9999),
            intimacy: state.intimacy.clamp(-100, 100),
            mood: truncate_text(state.mood.trim(), 40),
        },
        created_at: existing
            .as_ref()
            .map(|item| item.created_at.clone())
            .unwrap_or_else(|| now.clone()),
        updated_at: now,
    };
    if let Some(index) = config
        .companions
        .iter()
        .position(|existing| existing.id == companion.id)
    {
        config.companions[index] = companion.clone();
    } else {
        config.companions.push(companion.clone());
    }
    app_data::write_config(app, &config)?;
    Ok(companion)
}

pub fn switch_companion(app: &AppHandle, companion_id: &str) -> Result<Companion, String> {
    let mut config = read_companion_config(app)?;
    let companion_id = safe_companion_id(companion_id)?;
    let companion = config
        .companions
        .iter()
        .find(|companion| companion.id == companion_id)
        .cloned()
        .ok_or_else(|| "未找到要切换的伴侣".to_string())?;
    config.current_companion_id = companion_id.to_string();
    app_data::write_config(app, &config)?;
    Ok(companion)
}

pub fn delete_companion(app: &AppHandle, companion_id: &str) -> Result<Companion, String> {
    let companion_id = safe_companion_id(companion_id)?;
    if companion_id == DEFAULT_COMPANION_ID {
        return Err("默认伴侣不能删除".to_string());
    }

    let mut config = read_companion_config(app)?;
    if !config
        .companions
        .iter()
        .any(|companion| companion.id == companion_id)
    {
        return Err("未找到要删除的伴侣".to_string());
    }
    let mut conn = open_connection(app)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("删除伴侣失败：{err}"))?;
    tx.execute(
        "DELETE FROM pet_memory_messages WHERE companion_id = ?1",
        params![companion_id],
    )
    .map_err(|err| format!("删除伴侣聊天记录失败：{err}"))?;
    tx.execute(
        "DELETE FROM pet_memories WHERE companion_id = ?1",
        params![companion_id],
    )
    .map_err(|err| format!("删除伴侣长期记忆失败：{err}"))?;
    tx.commit().map_err(|err| format!("删除伴侣失败：{err}"))?;
    config
        .companions
        .retain(|companion| companion.id != companion_id);
    if config.current_companion_id == companion_id {
        config.current_companion_id = DEFAULT_COMPANION_ID.to_string();
    }
    app_data::write_config(app, &config)?;
    current_companion(app)
}

pub fn set_current_companion_skin(app: &AppHandle, skin_id: &str) -> Result<(), String> {
    let mut config = read_companion_config(app)?;
    let current_id = config.current_companion_id.clone();
    let companion = config
        .companions
        .iter_mut()
        .find(|companion| companion.id == current_id)
        .ok_or_else(|| "未找到当前伴侣档案".to_string())?;
    companion.skin_id = normalize_skin_id(skin_id);
    companion.updated_at = app_data::now_seconds();
    app_data::write_config(app, &config)?;
    Ok(())
}

pub fn save_message(
    app: &AppHandle,
    companion_id: &str,
    role: &str,
    content: &str,
) -> Result<(), String> {
    let companion_id = safe_companion_id(companion_id)?;
    let role = normalize_role(role).ok_or_else(|| "记忆消息角色不合法".to_string())?;
    let content = content.trim();
    if content.is_empty() {
        return Ok(());
    }
    let content = truncate_text(content, MAX_MESSAGE_CONTENT_LEN);

    let conn = open_connection(app)?;
    let id = next_unique_id(&conn, "pet_memory_messages")?;
    conn.execute(
        "INSERT INTO pet_memory_messages (id, companion_id, role, content, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            to_sql_id(id)?,
            companion_id,
            role,
            content,
            app_data::now_seconds()
        ],
    )
    .map_err(|err| format!("保存宠物聊天记忆失败：{err}"))?;
    trim_messages(&conn, companion_id)
}

pub fn recent_messages(
    app: &AppHandle,
    companion_id: &str,
    limit: usize,
) -> Result<Vec<PetMemoryMessage>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    let companion_id = safe_companion_id(companion_id)?;
    let conn = open_connection(app)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, companion_id, role, content, created_at
             FROM (
                SELECT id, companion_id, role, content, created_at
                FROM pet_memory_messages
                WHERE companion_id = ?1
                ORDER BY id DESC
                LIMIT ?2
             )
             ORDER BY id ASC",
        )
        .map_err(|err| format!("读取最近聊天记忆失败：{err}"))?;

    let rows = stmt
        .query_map(params![companion_id, limit as i64], row_to_message)
        .map_err(|err| format!("读取最近聊天记忆失败：{err}"))?;

    collect_rows(rows, "读取最近聊天记忆失败")
}

pub fn unsummarized_messages_for_short_summary(
    app: &AppHandle,
    companion_id: &str,
    keep_recent_messages: usize,
    last_summarized_message_id: u64,
) -> Result<Vec<PetMemoryMessage>, String> {
    let companion_id = safe_companion_id(companion_id)?;
    let conn = open_connection(app)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, companion_id, role, content, created_at
             FROM pet_memory_messages
             WHERE companion_id = ?1
               AND id > ?2
               AND id NOT IN (
                    SELECT id FROM pet_memory_messages
                    WHERE companion_id = ?1 ORDER BY id DESC LIMIT ?3
               )
             ORDER BY id ASC",
        )
        .map_err(|err| format!("读取待压缩短期记忆失败：{err}"))?;
    let rows = stmt
        .query_map(
            params![
                companion_id,
                last_summarized_message_id.min(i64::MAX as u64) as i64,
                keep_recent_messages as i64
            ],
            row_to_message,
        )
        .map_err(|err| format!("读取待压缩短期记忆失败：{err}"))?;

    collect_rows(rows, "读取待压缩短期记忆失败")
}

pub fn list_memories(app: &AppHandle) -> Result<Vec<PetMemory>, String> {
    let companion_id = current_companion_id(app)?;
    let conn = open_connection(app)?;
    list_memories_from_conn(&conn, &companion_id)
}

pub fn short_term_summary(
    app: &AppHandle,
    companion_id: &str,
) -> Result<Option<PetMemory>, String> {
    let companion_id = safe_companion_id(companion_id)?;
    let conn = open_connection(app)?;
    short_term_summary_from_conn(&conn, companion_id)
}

pub fn upsert_short_term_summary(
    app: &AppHandle,
    companion_id: &str,
    content: &str,
    last_summarized_message_id: u64,
) -> Result<Option<PetMemory>, String> {
    let companion_id = safe_companion_id(companion_id)?;
    let content = truncate_text(content.trim(), MAX_SHORT_TERM_SUMMARY_LEN);
    if content.is_empty() {
        return short_term_summary(app, companion_id);
    }

    let conn = open_connection(app)?;
    let now = app_data::now_seconds();
    let source_message = short_term_summary_source(last_summarized_message_id);
    let tags = vec!["短期摘要".to_string(), "上下文".to_string()];
    let memory = if let Some(mut memory) = short_term_summary_from_conn(&conn, companion_id)? {
        memory.content = content;
        memory.importance = 10;
        memory.tags = tags;
        memory.source_message = source_message;
        memory.confidence = 1.0;
        memory.deleted = false;
        memory.updated_at = now;
        update_memory(&conn, &memory)?;
        memory
    } else {
        let memory = PetMemory {
            id: next_unique_id(&conn, "pet_memories")?,
            companion_id: companion_id.to_string(),
            memory_type: SHORT_TERM_SUMMARY_MEMORY_TYPE.to_string(),
            content,
            importance: 10,
            tags,
            source_message,
            confidence: 1.0,
            deleted: false,
            created_at: now.clone(),
            updated_at: now,
        };
        insert_memory(&conn, &memory)?;
        memory
    };

    remove_extra_short_term_summaries(&conn, companion_id, memory.id)?;
    Ok(Some(memory))
}

pub fn short_term_summary_last_message_id(memory: Option<&PetMemory>) -> u64 {
    memory
        .and_then(|memory| memory.source_message.strip_prefix("last_message_id:"))
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0)
}

pub fn save_memories(
    app: &AppHandle,
    companion_id: &str,
    drafts: Vec<PetMemoryDraft>,
) -> Result<Vec<PetMemory>, String> {
    let companion_id = safe_companion_id(companion_id)?;
    let mut conn = open_connection(app)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("保存宠物长期记忆失败：{err}"))?;
    let mut saved = Vec::new();

    for draft in drafts {
        let Some(draft) = normalize_memory_draft(draft) else {
            continue;
        };

        let duplicate = find_duplicate_memory(&tx, companion_id, &draft)?;
        let now = app_data::now_seconds();
        if let Some(mut memory) = duplicate {
            memory.content = draft.content;
            memory.importance = memory.importance.max(draft.importance);
            memory.tags = merge_tags(&memory.tags, &draft.tags);
            if !draft.source_message.trim().is_empty() {
                memory.source_message = draft.source_message;
            }
            memory.confidence = memory.confidence.max(draft.confidence);
            memory.updated_at = now;
            update_memory(&tx, &memory)?;
            saved.push(memory);
        } else {
            let memory = PetMemory {
                id: next_unique_id(&tx, "pet_memories")?,
                companion_id: companion_id.to_string(),
                memory_type: draft.memory_type,
                content: draft.content,
                importance: draft.importance,
                tags: draft.tags,
                source_message: draft.source_message,
                confidence: draft.confidence,
                deleted: false,
                created_at: now.clone(),
                updated_at: now,
            };
            insert_memory(&tx, &memory)?;
            saved.push(memory);
        }
    }

    tx.commit()
        .map_err(|err| format!("保存宠物长期记忆失败：{err}"))?;
    Ok(saved)
}

pub fn add_memory(app: &AppHandle, draft: PetMemoryDraft) -> Result<PetMemory, String> {
    let companion_id = current_companion_id(app)?;
    save_memories(app, &companion_id, vec![draft])?
        .into_iter()
        .next()
        .ok_or_else(|| "记忆内容为空或不适合保存".to_string())
}

pub fn update_memory_by_id(
    app: &AppHandle,
    memory_id: u64,
    draft: PetMemoryDraft,
) -> Result<PetMemory, String> {
    let Some(draft) = normalize_memory_draft(draft) else {
        return Err("记忆内容为空或不适合保存".to_string());
    };

    let companion_id = current_companion_id(app)?;
    let conn = open_connection(app)?;
    let mut memory = load_memory_by_id(&conn, &companion_id, memory_id)?
        .ok_or_else(|| "未找到要修改的记忆".to_string())?;
    memory.memory_type = draft.memory_type;
    memory.content = draft.content;
    memory.importance = draft.importance;
    memory.tags = draft.tags;
    memory.source_message = draft.source_message;
    memory.confidence = draft.confidence;
    memory.deleted = false;
    memory.updated_at = app_data::now_seconds();
    update_memory(&conn, &memory)?;
    Ok(memory)
}

pub fn search_memories(
    app: &AppHandle,
    companion_id: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<PetMemory>, String> {
    let query = query.trim();
    if query.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    let companion_id = safe_companion_id(companion_id)?;
    let conn = open_connection(app)?;
    let fts_result = search_memories_fts(&conn, companion_id, query, limit);
    match fts_result {
        Ok(memories) if !memories.is_empty() => Ok(memories),
        _ => search_memories_scored(&conn, companion_id, query, limit),
    }
}

pub fn delete_memory(app: &AppHandle, memory_id: u64) -> Result<(), String> {
    let companion_id = current_companion_id(app)?;
    let conn = open_connection(app)?;
    let deleted = conn
        .execute(
            "DELETE FROM pet_memories WHERE id = ?1 AND companion_id = ?2",
            params![to_sql_id(memory_id)?, companion_id],
        )
        .map_err(|err| format!("删除宠物记忆失败：{err}"))?;

    if deleted == 0 {
        return Err("未找到要删除的记忆".to_string());
    }

    Ok(())
}

pub fn clear_memories(app: &AppHandle) -> Result<(), String> {
    let companion_id = current_companion_id(app)?;
    let conn = open_connection(app)?;
    clear_memories_from_conn(&conn, &companion_id)
}

fn clear_memories_from_conn(conn: &Connection, companion_id: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM pet_memories WHERE companion_id = ?1",
        params![companion_id],
    )
    .map_err(|err| format!("清空宠物记忆失败：{err}"))?;
    Ok(())
}

pub fn clear_messages(app: &AppHandle) -> Result<(), String> {
    let companion_id = current_companion_id(app)?;
    let conn = open_connection(app)?;
    conn.execute(
        "DELETE FROM pet_memory_messages WHERE companion_id = ?1",
        params![companion_id],
    )
    .map_err(|err| format!("清空宠物短期聊天记录失败：{err}"))?;
    conn.execute(
        "DELETE FROM pet_memories WHERE companion_id = ?1 AND memory_type = ?2",
        params![companion_id, SHORT_TERM_SUMMARY_MEMORY_TYPE],
    )
    .map_err(|err| format!("清空短期摘要记忆失败：{err}"))?;
    Ok(())
}

pub fn delete_messages(app: &AppHandle, message_ids: Vec<u64>) -> Result<usize, String> {
    let mut message_ids = message_ids
        .into_iter()
        .filter(|message_id| *message_id > 0)
        .collect::<Vec<_>>();
    message_ids.sort_unstable();
    message_ids.dedup();
    if message_ids.is_empty() {
        return Ok(0);
    }

    let companion_id = current_companion_id(app)?;
    let mut conn = open_connection(app)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("开始删除聊天记录事务失败：{err}"))?;
    let mut deleted = 0;
    for message_id in message_ids {
        deleted += tx
            .execute(
                "DELETE FROM pet_memory_messages WHERE id = ?1 AND companion_id = ?2",
                params![to_sql_id(message_id)?, &companion_id],
            )
            .map_err(|err| format!("删除聊天记录失败：{err}"))?;
    }
    tx.commit()
        .map_err(|err| format!("提交删除聊天记录事务失败：{err}"))?;
    Ok(deleted)
}

pub fn delete_related_memories(
    app: &AppHandle,
    companion_id: &str,
    query: &str,
    limit: usize,
) -> Result<usize, String> {
    let memories = search_memories(app, companion_id, query, limit)?;
    if memories.is_empty() {
        return Ok(0);
    }

    let conn = open_connection(app)?;
    let mut deleted = 0;
    for memory in memories {
        deleted += conn
            .execute(
                "DELETE FROM pet_memories WHERE id = ?1 AND companion_id = ?2",
                params![to_sql_id(memory.id)?, companion_id],
            )
            .map_err(|err| format!("删除相关宠物记忆失败：{err}"))?;
    }
    Ok(deleted)
}

pub fn delete_latest_memory(app: &AppHandle, companion_id: &str) -> Result<usize, String> {
    let conn = open_connection(app)?;
    let Some(memory) = latest_memory(&conn, companion_id)? else {
        return Ok(0);
    };

    let deleted = conn
        .execute(
            "DELETE FROM pet_memories WHERE id = ?1 AND companion_id = ?2",
            params![to_sql_id(memory.id)?, companion_id],
        )
        .map_err(|err| format!("删除最近宠物记忆失败：{err}"))?;
    Ok(deleted)
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

    let companion_id = current_companion_id(app)?;
    for message in &mut store.messages {
        message.companion_id = companion_id.clone();
    }
    for memory in &mut store.memories {
        memory.companion_id = companion_id.clone();
    }
    let mut conn = open_connection(app)?;
    replace_store(&mut conn, &companion_id, &store)?;
    list_memories_from_conn(&conn, &companion_id)
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
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|err| format!("创建导出目录失败：{err}"))?;
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
    let companion_id = current_companion_id(app)?;
    let conn = open_connection(app)?;
    read_store_from_conn(&conn, &companion_id)
}

fn open_connection(app: &AppHandle) -> Result<Connection, String> {
    app_data::ensure_data_files(app)?;
    fs::create_dir_all(memory_dir(app)?).map_err(|err| format!("创建记忆目录失败：{err}"))?;

    let conn = Connection::open(memory_db_file(app)?)
        .map_err(|err| format!("打开记忆数据库失败：{err}"))?;
    initialize_schema(&conn)?;
    migrate_legacy_json(app, &conn)?;
    purge_soft_deleted_memories(&conn)?;
    Ok(conn)
}

fn initialize_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS pet_memory_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS pet_memory_messages (
            id INTEGER PRIMARY KEY,
            companion_id TEXT NOT NULL DEFAULT 'default',
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS pet_memories (
            id INTEGER PRIMARY KEY,
            companion_id TEXT NOT NULL DEFAULT 'default',
            memory_type TEXT NOT NULL,
            content TEXT NOT NULL,
            importance INTEGER NOT NULL,
            tags_json TEXT NOT NULL,
            tags TEXT NOT NULL,
            source_message TEXT NOT NULL DEFAULT '',
            confidence REAL NOT NULL DEFAULT 0.8,
            deleted INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS pet_memories_fts
        USING fts5(
            content,
            memory_type,
            tags,
            content='pet_memories',
            content_rowid='id',
            tokenize='unicode61'
        );

        CREATE TRIGGER IF NOT EXISTS pet_memories_ai
        AFTER INSERT ON pet_memories
        WHEN new.deleted = 0
        BEGIN
            INSERT INTO pet_memories_fts(rowid, content, memory_type, tags)
            VALUES (new.id, new.content, new.memory_type, new.tags);
        END;

        CREATE TRIGGER IF NOT EXISTS pet_memories_ad
        AFTER DELETE ON pet_memories
        WHEN old.deleted = 0
        BEGIN
            INSERT INTO pet_memories_fts(pet_memories_fts, rowid, content, memory_type, tags)
            VALUES ('delete', old.id, old.content, old.memory_type, old.tags);
        END;

        CREATE TRIGGER IF NOT EXISTS pet_memories_au_delete
        AFTER UPDATE ON pet_memories
        WHEN old.deleted = 0
        BEGIN
            INSERT INTO pet_memories_fts(pet_memories_fts, rowid, content, memory_type, tags)
            VALUES ('delete', old.id, old.content, old.memory_type, old.tags);
        END;

        CREATE TRIGGER IF NOT EXISTS pet_memories_au_insert
        AFTER UPDATE ON pet_memories
        WHEN new.deleted = 0
        BEGIN
            INSERT INTO pet_memories_fts(rowid, content, memory_type, tags)
            VALUES (new.id, new.content, new.memory_type, new.tags);
        END;

        PRAGMA user_version = 2;
        "#,
    )
    .map_err(|err| format!("初始化记忆数据库失败：{err}"))?;

    ensure_memory_columns(conn)?;
    conn.execute_batch("DROP TABLE IF EXISTS companions; DROP TABLE IF EXISTS user_settings;")
        .map_err(|err| format!("清理旧版伴侣配置表失败：{err}"))?;
    Ok(())
}

fn ensure_memory_columns(conn: &Connection) -> Result<(), String> {
    let columns = table_columns(conn, "pet_memories")?;
    if !columns.contains("companion_id") {
        conn.execute(
            "ALTER TABLE pet_memories ADD COLUMN companion_id TEXT NOT NULL DEFAULT 'default'",
            [],
        )
        .map_err(|err| format!("升级记忆数据库失败：{err}"))?;
    }
    if !columns.contains("source_message") {
        conn.execute(
            "ALTER TABLE pet_memories ADD COLUMN source_message TEXT NOT NULL DEFAULT ''",
            [],
        )
        .map_err(|err| format!("升级记忆数据库失败：{err}"))?;
    }
    if !columns.contains("confidence") {
        conn.execute(
            "ALTER TABLE pet_memories ADD COLUMN confidence REAL NOT NULL DEFAULT 0.8",
            [],
        )
        .map_err(|err| format!("升级记忆数据库失败：{err}"))?;
    }
    let message_columns = table_columns(conn, "pet_memory_messages")?;
    if !message_columns.contains("companion_id") {
        conn.execute(
            "ALTER TABLE pet_memory_messages ADD COLUMN companion_id TEXT NOT NULL DEFAULT 'default'",
            [],
        )
        .map_err(|err| format!("升级聊天数据库失败：{err}"))?;
    }
    Ok(())
}

fn table_columns(conn: &Connection, table_name: &str) -> Result<HashSet<String>, String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table_name})"))
        .map_err(|err| format!("读取记忆数据库结构失败：{err}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|err| format!("读取记忆数据库结构失败：{err}"))?;

    rows.collect::<Result<HashSet<_>, _>>()
        .map_err(|err| format!("读取记忆数据库结构失败：{err}"))
}

fn purge_soft_deleted_memories(conn: &Connection) -> Result<(), String> {
    conn.execute("DELETE FROM pet_memories WHERE deleted <> 0", [])
        .map_err(|err| format!("清理旧版已删除长期记忆失败：{err}"))?;
    Ok(())
}

fn migrate_legacy_json(app: &AppHandle, conn: &Connection) -> Result<(), String> {
    if get_meta(conn, "legacy_json_migrated")?.as_deref() == Some("1") {
        return Ok(());
    }

    let legacy_path = legacy_memory_json_file(app)?;
    if !legacy_path.exists() {
        set_meta(conn, "legacy_json_migrated", "1")?;
        return Ok(());
    }

    if has_any_memory_data(conn)? {
        set_meta(conn, "legacy_json_migrated", "1")?;
        return Ok(());
    }

    let content =
        fs::read_to_string(&legacy_path).map_err(|err| format!("读取旧版 JSON 记忆失败：{err}"))?;
    if content.trim().is_empty() {
        set_meta(conn, "legacy_json_migrated", "1")?;
        return Ok(());
    }

    let mut store: PetMemoryStore =
        serde_json::from_str(&content).map_err(|err| format!("旧版 JSON 记忆格式错误：{err}"))?;
    normalize_imported_store(&mut store);
    replace_store_in_conn(conn, DEFAULT_COMPANION_ID, &store)?;
    set_meta(conn, "legacy_json_migrated", "1")
}

fn read_store_from_conn(conn: &Connection, companion_id: &str) -> Result<PetMemoryStore, String> {
    Ok(PetMemoryStore {
        messages: read_all_messages(conn, companion_id)?,
        memories: read_all_memories(conn, companion_id)?,
    })
}

fn read_all_messages(
    conn: &Connection,
    companion_id: &str,
) -> Result<Vec<PetMemoryMessage>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, companion_id, role, content, created_at
             FROM pet_memory_messages WHERE companion_id = ?1 ORDER BY id ASC",
        )
        .map_err(|err| format!("读取宠物聊天记忆失败：{err}"))?;
    let rows = stmt
        .query_map(params![companion_id], row_to_message)
        .map_err(|err| format!("读取宠物聊天记忆失败：{err}"))?;
    collect_rows(rows, "读取宠物聊天记忆失败")
}

fn read_all_memories(conn: &Connection, companion_id: &str) -> Result<Vec<PetMemory>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, companion_id, memory_type, content, importance, tags_json, source_message,
                    confidence, deleted, created_at, updated_at
             FROM pet_memories
             WHERE companion_id = ?1
             ORDER BY id ASC",
        )
        .map_err(|err| format!("读取宠物长期记忆失败：{err}"))?;
    let rows = stmt
        .query_map(params![companion_id], row_to_memory)
        .map_err(|err| format!("读取宠物长期记忆失败：{err}"))?;
    collect_rows(rows, "读取宠物长期记忆失败")
}

fn list_memories_from_conn(
    conn: &Connection,
    companion_id: &str,
) -> Result<Vec<PetMemory>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, companion_id, memory_type, content, importance, tags_json, source_message,
                    confidence, deleted, created_at, updated_at
             FROM pet_memories
             WHERE companion_id = ?1 AND deleted = 0
             ORDER BY importance DESC, updated_at DESC",
        )
        .map_err(|err| format!("读取宠物长期记忆失败：{err}"))?;
    let rows = stmt
        .query_map(params![companion_id], row_to_memory)
        .map_err(|err| format!("读取宠物长期记忆失败：{err}"))?;
    collect_rows(rows, "读取宠物长期记忆失败")
}

fn load_memory_by_id(
    conn: &Connection,
    companion_id: &str,
    memory_id: u64,
) -> Result<Option<PetMemory>, String> {
    conn.query_row(
        "SELECT id, companion_id, memory_type, content, importance, tags_json, source_message,
                confidence, deleted, created_at, updated_at
         FROM pet_memories
         WHERE id = ?1 AND companion_id = ?2 AND deleted = 0",
        params![to_sql_id(memory_id)?, companion_id],
        row_to_memory,
    )
    .optional()
    .map_err(|err| format!("读取宠物长期记忆失败：{err}"))
}

fn latest_memory(conn: &Connection, companion_id: &str) -> Result<Option<PetMemory>, String> {
    conn.query_row(
        "SELECT id, companion_id, memory_type, content, importance, tags_json, source_message,
                confidence, deleted, created_at, updated_at
         FROM pet_memories
         WHERE companion_id = ?1 AND deleted = 0
         ORDER BY updated_at DESC, id DESC
         LIMIT 1",
        params![companion_id],
        row_to_memory,
    )
    .optional()
    .map_err(|err| format!("读取最近宠物记忆失败：{err}"))
}

fn short_term_summary_from_conn(
    conn: &Connection,
    companion_id: &str,
) -> Result<Option<PetMemory>, String> {
    conn.query_row(
        "SELECT id, companion_id, memory_type, content, importance, tags_json, source_message,
                confidence, deleted, created_at, updated_at
         FROM pet_memories
         WHERE companion_id = ?1 AND deleted = 0 AND memory_type = ?2
         ORDER BY updated_at DESC, id DESC
         LIMIT 1",
        params![companion_id, SHORT_TERM_SUMMARY_MEMORY_TYPE],
        row_to_memory,
    )
    .optional()
    .map_err(|err| format!("读取短期摘要记忆失败：{err}"))
}

fn short_term_summary_source(last_summarized_message_id: u64) -> String {
    format!("last_message_id:{last_summarized_message_id}")
}

fn remove_extra_short_term_summaries(
    conn: &Connection,
    companion_id: &str,
    keep_id: u64,
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM pet_memories
         WHERE companion_id = ?1 AND memory_type = ?2 AND id <> ?3",
        params![
            companion_id,
            SHORT_TERM_SUMMARY_MEMORY_TYPE,
            to_sql_id(keep_id)?
        ],
    )
    .map_err(|err| format!("清理重复短期摘要记忆失败：{err}"))?;
    Ok(())
}

fn search_memories_fts(
    conn: &Connection,
    companion_id: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<PetMemory>, String> {
    let Some(fts_query) = fts_match_query(query) else {
        return Ok(Vec::new());
    };

    let mut stmt = conn
        .prepare(
            "SELECT m.id, m.companion_id, m.memory_type, m.content, m.importance, m.tags_json,
                    m.source_message, m.confidence, m.deleted, m.created_at, m.updated_at
             FROM pet_memories_fts
             JOIN pet_memories m ON m.id = pet_memories_fts.rowid
             WHERE pet_memories_fts MATCH ?1 AND m.companion_id = ?2 AND m.deleted = 0
             ORDER BY m.importance DESC, m.updated_at DESC, bm25(pet_memories_fts) ASC
             LIMIT ?3",
        )
        .map_err(|err| format!("检索宠物记忆失败：{err}"))?;
    let rows = stmt
        .query_map(
            params![fts_query, companion_id, limit as i64],
            row_to_memory,
        )
        .map_err(|err| format!("检索宠物记忆失败：{err}"))?;
    collect_rows(rows, "检索宠物记忆失败")
}

fn search_memories_scored(
    conn: &Connection,
    companion_id: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<PetMemory>, String> {
    let tokens = query_tokens(query);
    let mut scored = list_memories_from_conn(conn, companion_id)?
        .into_iter()
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

fn find_duplicate_memory(
    conn: &Connection,
    companion_id: &str,
    draft: &PetMemoryDraft,
) -> Result<Option<PetMemory>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, companion_id, memory_type, content, importance, tags_json, source_message,
                    confidence, deleted, created_at, updated_at
             FROM pet_memories
             WHERE companion_id = ?1 AND deleted = 0 AND memory_type = ?2",
        )
        .map_err(|err| format!("查找重复宠物记忆失败：{err}"))?;
    let rows = stmt
        .query_map(params![companion_id, &draft.memory_type], row_to_memory)
        .map_err(|err| format!("查找重复宠物记忆失败：{err}"))?;

    for row in rows {
        let memory = row.map_err(|err| format!("查找重复宠物记忆失败：{err}"))?;
        if same_memory_content(&memory.content, &draft.content) {
            return Ok(Some(memory));
        }
    }

    Ok(None)
}

fn insert_memory(conn: &Connection, memory: &PetMemory) -> Result<(), String> {
    let tags_json = serialize_tags(&memory.tags)?;
    let tags_text = memory.tags.join(" ");
    conn.execute(
        "INSERT INTO pet_memories
         (id, companion_id, memory_type, content, importance, tags_json, tags, source_message,
          confidence, deleted, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            to_sql_id(memory.id)?,
            &memory.companion_id,
            &memory.memory_type,
            &memory.content,
            i64::from(memory.importance),
            &tags_json,
            &tags_text,
            &memory.source_message,
            f64::from(memory.confidence),
            bool_to_i64(memory.deleted),
            &memory.created_at,
            &memory.updated_at,
        ],
    )
    .map_err(|err| format!("保存宠物长期记忆失败：{err}"))?;
    Ok(())
}

fn update_memory(conn: &Connection, memory: &PetMemory) -> Result<(), String> {
    let tags_json = serialize_tags(&memory.tags)?;
    let tags_text = memory.tags.join(" ");
    conn.execute(
        "UPDATE pet_memories
         SET memory_type = ?1, content = ?2, importance = ?3, tags_json = ?4, tags = ?5,
             source_message = ?6, confidence = ?7, deleted = ?8, updated_at = ?9
         WHERE id = ?10 AND companion_id = ?11",
        params![
            &memory.memory_type,
            &memory.content,
            i64::from(memory.importance),
            &tags_json,
            &tags_text,
            &memory.source_message,
            f64::from(memory.confidence),
            bool_to_i64(memory.deleted),
            &memory.updated_at,
            to_sql_id(memory.id)?,
            &memory.companion_id,
        ],
    )
    .map_err(|err| format!("更新宠物长期记忆失败：{err}"))?;
    Ok(())
}

fn replace_store(
    conn: &mut Connection,
    companion_id: &str,
    store: &PetMemoryStore,
) -> Result<(), String> {
    let tx = conn
        .transaction()
        .map_err(|err| format!("导入宠物记忆失败：{err}"))?;
    replace_store_in_conn(&tx, companion_id, store)?;
    tx.commit()
        .map_err(|err| format!("导入宠物记忆失败：{err}"))
}

fn replace_store_in_conn(
    conn: &Connection,
    companion_id: &str,
    store: &PetMemoryStore,
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM pet_memory_messages WHERE companion_id = ?1",
        params![companion_id],
    )
    .map_err(|err| format!("导入宠物聊天记忆失败：{err}"))?;
    conn.execute(
        "DELETE FROM pet_memories WHERE companion_id = ?1",
        params![companion_id],
    )
    .map_err(|err| format!("导入宠物长期记忆失败：{err}"))?;

    for message in &store.messages {
        conn.execute(
            "INSERT INTO pet_memory_messages (id, companion_id, role, content, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                to_sql_id(message.id)?,
                companion_id,
                &message.role,
                &message.content,
                &message.created_at
            ],
        )
        .map_err(|err| format!("导入宠物聊天记忆失败：{err}"))?;
    }

    for memory in &store.memories {
        insert_memory(conn, memory)?;
    }

    trim_messages(conn, companion_id)?;
    Ok(())
}

fn trim_messages(conn: &Connection, companion_id: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM pet_memory_messages
         WHERE companion_id = ?1 AND id NOT IN (
            SELECT id FROM pet_memory_messages
            WHERE companion_id = ?1 ORDER BY id DESC LIMIT ?2
         )",
        params![companion_id, MAX_MESSAGES as i64],
    )
    .map_err(|err| format!("裁剪宠物聊天记忆失败：{err}"))?;
    Ok(())
}

fn has_any_memory_data(conn: &Connection) -> Result<bool, String> {
    let messages: i64 = conn
        .query_row("SELECT COUNT(*) FROM pet_memory_messages", [], |row| {
            row.get(0)
        })
        .map_err(|err| format!("检查记忆数据库失败：{err}"))?;
    let memories: i64 = conn
        .query_row("SELECT COUNT(*) FROM pet_memories", [], |row| row.get(0))
        .map_err(|err| format!("检查记忆数据库失败：{err}"))?;
    Ok(messages > 0 || memories > 0)
}

fn get_meta(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT value FROM pet_memory_meta WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .optional()
    .map_err(|err| format!("读取记忆数据库元信息失败：{err}"))
}

fn set_meta(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO pet_memory_meta (key, value)
         VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .map_err(|err| format!("保存记忆数据库元信息失败：{err}"))?;
    Ok(())
}

fn row_to_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<PetMemoryMessage> {
    let id: i64 = row.get(0)?;
    Ok(PetMemoryMessage {
        id: id.max(0) as u64,
        companion_id: row.get(1)?,
        role: row.get(2)?,
        content: row.get(3)?,
        created_at: row.get(4)?,
    })
}

fn row_to_memory(row: &rusqlite::Row<'_>) -> rusqlite::Result<PetMemory> {
    let id: i64 = row.get(0)?;
    let tags_json: String = row.get(5)?;
    let confidence = row.get::<_, f64>(7)?.clamp(0.0, 1.0) as f32;
    let deleted: i64 = row.get(8)?;

    Ok(PetMemory {
        id: id.max(0) as u64,
        companion_id: row.get(1)?,
        memory_type: row.get(2)?,
        content: row.get(3)?,
        importance: row.get::<_, i64>(4)?.clamp(1, 10) as u8,
        tags: parse_tags_json(&tags_json),
        source_message: row.get(6)?,
        confidence,
        deleted: deleted != 0,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn collect_rows<T, F>(rows: rusqlite::MappedRows<'_, F>, context: &str) -> Result<Vec<T>, String>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
{
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("{context}：{err}"))
}

fn memory_db_file(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(memory_dir(app)?.join(MEMORY_DB_FILE_NAME))
}

fn legacy_memory_json_file(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(memory_dir(app)?.join(LEGACY_MEMORY_JSON_FILE_NAME))
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

fn safe_companion_id(companion_id: &str) -> Result<&str, String> {
    let companion_id = companion_id.trim();
    if companion_id.is_empty()
        || !companion_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        return Err("伴侣 ID 不合法".to_string());
    }
    Ok(companion_id)
}

fn normalize_skin_id(skin_id: &str) -> String {
    let skin_id = skin_id.trim();
    if skin_id.is_empty()
        || !skin_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        default_skin_id()
    } else {
        skin_id.to_string()
    }
}

fn normalize_memory_draft(draft: PetMemoryDraft) -> Option<PetMemoryDraft> {
    let content = draft.content.trim();
    if content.is_empty() {
        return None;
    }
    if contains_sensitive_memory(content) || contains_sensitive_memory(&draft.source_message) {
        return None;
    }

    let memory_type = normalize_memory_type(&draft.memory_type);
    let max_content_len = if memory_type == SHORT_TERM_SUMMARY_MEMORY_TYPE {
        MAX_SHORT_TERM_SUMMARY_LEN
    } else {
        MAX_MEMORY_CONTENT_LEN
    };
    let content = truncate_text(content, max_content_len);
    let source_message = truncate_text(draft.source_message.trim(), MAX_MESSAGE_CONTENT_LEN);

    Some(PetMemoryDraft {
        memory_type,
        content,
        importance: draft.importance.clamp(1, 10),
        tags: normalize_tags(draft.tags),
        source_message,
        confidence: draft.confidence.clamp(0.0, 1.0),
    })
}

fn normalize_memory_type(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "nickname" | "preference" | "dislike" | "relationship" | "emotion" | "habit"
        | "life_event" | "important_person" | "interest" | "goal" | "boundary" | "instruction"
        | "short_term_summary" | "other" => value.trim().to_lowercase(),
        "project" => "goal".to_string(),
        "event" => "life_event".to_string(),
        "profile" => "other".to_string(),
        _ => default_memory_type(),
    }
}

fn contains_sensitive_memory(value: &str) -> bool {
    let lower = value.to_lowercase();
    let sensitive_keywords = [
        "密码",
        "口令",
        "银行卡",
        "身份证",
        "住址",
        "精确地址",
        "手机号",
        "电话",
        "联系方式",
        "api key",
        "apikey",
        "access token",
        "secret",
        "token",
        "验证码",
    ];
    if sensitive_keywords
        .iter()
        .any(|keyword| lower.contains(keyword))
    {
        return true;
    }

    let digit_count = value.chars().filter(|ch| ch.is_ascii_digit()).count();
    digit_count >= 11 && value.chars().filter(|ch| ch.is_whitespace()).count() <= 4
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut output = Vec::new();
    for tag in tags {
        let tag = truncate_text(tag.trim(), 32);
        if tag.is_empty() || !seen.insert(tag.to_lowercase()) {
            continue;
        }
        output.push(tag);
        if output.len() >= 8 {
            break;
        }
    }
    output
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
        message.companion_id = safe_companion_id(&message.companion_id)
            .unwrap_or(DEFAULT_COMPANION_ID)
            .to_string();
        message.role = normalize_role(&message.role).unwrap_or_else(|| "user".to_string());
        let content = message.content.trim().to_string();
        message.content = truncate_text(&content, MAX_MESSAGE_CONTENT_LEN);
        if message.created_at.trim().is_empty() {
            message.created_at = app_data::now_seconds();
        }
    }

    store.messages.retain(|message| !message.content.is_empty());
    ensure_unique_message_ids(&mut store.messages);

    for memory in &mut store.memories {
        memory.companion_id = safe_companion_id(&memory.companion_id)
            .unwrap_or(DEFAULT_COMPANION_ID)
            .to_string();
        memory.memory_type = normalize_memory_type(&memory.memory_type);
        let content = memory.content.trim().to_string();
        let max_content_len = if memory.memory_type == SHORT_TERM_SUMMARY_MEMORY_TYPE {
            MAX_SHORT_TERM_SUMMARY_LEN
        } else {
            MAX_MEMORY_CONTENT_LEN
        };
        memory.content = truncate_text(&content, max_content_len);
        memory.importance = memory.importance.clamp(1, 10);
        memory.tags = normalize_tags(memory.tags.clone());
        let source_message = memory.source_message.trim().to_string();
        memory.source_message = truncate_text(&source_message, MAX_MESSAGE_CONTENT_LEN);
        memory.confidence = memory.confidence.clamp(0.0, 1.0);
        if memory.created_at.trim().is_empty() {
            memory.created_at = app_data::now_seconds();
        }
        if memory.updated_at.trim().is_empty() {
            memory.updated_at = memory.created_at.clone();
        }
    }

    store
        .memories
        .retain(|memory| !memory.content.is_empty() && !contains_sensitive_memory(&memory.content));
    ensure_unique_memory_ids(&mut store.memories);
}

fn ensure_unique_message_ids(messages: &mut [PetMemoryMessage]) {
    let mut seen = HashSet::new();
    let mut next_id = now_millis();
    for message in messages {
        if is_sql_safe_id(message.id) && seen.insert(message.id) {
            continue;
        }

        while seen.contains(&next_id) || !is_sql_safe_id(next_id) {
            next_id = next_id.saturating_add(1);
        }
        message.id = next_id;
        seen.insert(next_id);
        next_id = next_id.saturating_add(1);
    }
}

fn ensure_unique_memory_ids(memories: &mut [PetMemory]) {
    let mut seen = HashSet::new();
    let mut next_id = now_millis();
    for memory in memories {
        if is_sql_safe_id(memory.id) && seen.insert(memory.id) {
            continue;
        }

        while seen.contains(&next_id) || !is_sql_safe_id(next_id) {
            next_id = next_id.saturating_add(1);
        }
        memory.id = next_id;
        seen.insert(next_id);
        next_id = next_id.saturating_add(1);
    }
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
        .flat_map(char::to_lowercase)
        .collect()
}

fn truncate_text(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn fts_match_query(query: &str) -> Option<String> {
    let terms = query_tokens(query)
        .into_iter()
        .filter(|term| term.chars().count() >= 2)
        .take(16)
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>();

    (!terms.is_empty()).then(|| terms.join(" OR "))
}

fn query_tokens(query: &str) -> Vec<String> {
    let mut tokens = HashSet::new();
    let mut current = String::new();

    for ch in query.chars() {
        if ch.is_alphanumeric() || is_cjk(ch) {
            current.push(ch.to_ascii_lowercase());
        } else {
            push_query_token(&mut tokens, &current);
            current.clear();
        }
    }
    push_query_token(&mut tokens, &current);

    tokens.into_iter().collect()
}

fn push_query_token(tokens: &mut HashSet<String>, value: &str) {
    let value = value.trim().to_string();
    if value.chars().count() >= 2 {
        tokens.insert(value.clone());
    }

    let chars = value.chars().collect::<Vec<_>>();
    if chars.iter().any(|ch| is_cjk(*ch)) {
        for window in chars.windows(2) {
            if window.iter().all(|ch| is_cjk(*ch)) {
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

    let compact_query = compact_text(query);
    if !compact_query.is_empty() {
        let compact_haystack = compact_text(&haystack);
        if compact_haystack.contains(&compact_query) {
            score += compact_query.chars().count().max(4) * 3;
        }
    }

    score
}

fn is_cjk(ch: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&ch)
}

fn parse_tags_json(value: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(value).unwrap_or_default()
}

fn serialize_tags(tags: &[String]) -> Result<String, String> {
    serde_json::to_string(tags).map_err(|err| format!("序列化记忆标签失败：{err}"))
}

fn bool_to_i64(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn next_unique_id(conn: &Connection, table: &str) -> Result<u64, String> {
    let table = match table {
        "pet_memory_messages" => "pet_memory_messages",
        "pet_memories" => "pet_memories",
        _ => return Err("记忆表名不合法".to_string()),
    };

    let mut id = now_millis();
    loop {
        let count: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE id = ?1"),
                params![to_sql_id(id)?],
                |row| row.get(0),
            )
            .map_err(|err| format!("生成记忆 ID 失败：{err}"))?;
        if count == 0 {
            return Ok(id);
        }
        id = id.saturating_add(1);
    }
}

fn to_sql_id(id: u64) -> Result<i64, String> {
    if !is_sql_safe_id(id) {
        return Err("记忆 ID 超出 SQLite 支持范围".to_string());
    }
    Ok(id as i64)
}

fn is_sql_safe_id(id: u64) -> bool {
    id > 0 && id <= i64::MAX as u64
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as u64
}

fn default_memory_type() -> String {
    "other".to_string()
}

fn default_companion_id() -> String {
    DEFAULT_COMPANION_ID.to_string()
}

fn default_skin_id() -> String {
    "default".to_string()
}

fn default_importance() -> u8 {
    5
}

fn default_confidence() -> f32 {
    0.8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_schema_includes_fts5() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        initialize_schema(&conn).expect("initialize schema with fts5");
        conn.execute(
            "INSERT INTO pet_memories
             (id, memory_type, content, importance, tags_json, tags, deleted, created_at, updated_at)
             VALUES (1, 'project', '桌面宠物', 8, '[\"项目\"]', '项目', 0, '1', '1')",
            [],
        )
        .expect("insert memory");
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pet_memories_fts WHERE pet_memories_fts MATCH '\"桌面宠物\"'",
                [],
                |row| row.get(0),
            )
            .expect("query fts5");
        assert_eq!(count, 1);
    }

    #[test]
    fn sqlite_schema_includes_companion_memory_columns() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        initialize_schema(&conn).expect("initialize schema");
        let columns = table_columns(&conn, "pet_memories").expect("read table columns");
        assert!(columns.contains("source_message"));
        assert!(columns.contains("confidence"));
        assert!(columns.contains("companion_id"));
        let message_columns =
            table_columns(&conn, "pet_memory_messages").expect("read message columns");
        assert!(message_columns.contains("companion_id"));
    }

    #[test]
    fn sensitive_memory_draft_is_skipped() {
        let draft = PetMemoryDraft {
            memory_type: "preference".to_string(),
            content: "用户的 API Key 是 abc123。".to_string(),
            importance: 10,
            tags: vec!["敏感".to_string()],
            source_message: String::new(),
            confidence: 1.0,
        };
        assert!(normalize_memory_draft(draft).is_none());
    }

    #[test]
    fn clear_memories_physically_removes_rows_and_search_index() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        initialize_schema(&conn).expect("initialize schema");
        conn.execute(
            "INSERT INTO pet_memories
             (id, memory_type, content, importance, tags_json, tags, deleted, created_at, updated_at)
             VALUES (1, 'preference', '喜欢安静', 8, '[]', '', 0, '1', '1')",
            [],
        )
        .expect("insert active memory");

        clear_memories_from_conn(&conn, DEFAULT_COMPANION_ID).expect("clear memories");

        let memory_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM pet_memories", [], |row| row.get(0))
            .expect("query memories");
        let index_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM pet_memories_fts", [], |row| {
                row.get(0)
            })
            .expect("query fts rows");
        assert_eq!(memory_count, 0);
        assert_eq!(index_count, 0);
    }

    #[test]
    fn companion_memory_reads_and_clears_are_scoped() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        initialize_schema(&conn).expect("initialize schema");
        conn.execute(
            "INSERT INTO pet_memories
             (id, companion_id, memory_type, content, importance, tags_json, tags, deleted, created_at, updated_at)
             VALUES (1, 'default', 'preference', '默认角色记忆', 8, '[]', '', 0, '1', '1')",
            [],
        )
        .expect("insert default memory");
        conn.execute(
            "INSERT INTO pet_memories
             (id, companion_id, memory_type, content, importance, tags_json, tags, deleted, created_at, updated_at)
             VALUES (2, 'companion_second', 'preference', '第二角色记忆', 8, '[]', '', 0, '1', '1')",
            [],
        )
        .expect("insert second companion memory");
        conn.execute(
            "INSERT INTO pet_memory_messages (id, companion_id, role, content, created_at)
             VALUES (3, 'default', 'user', '默认角色对话', '1')",
            [],
        )
        .expect("insert default message");
        conn.execute(
            "INSERT INTO pet_memory_messages (id, companion_id, role, content, created_at)
             VALUES (4, 'companion_second', 'user', '第二角色对话', '1')",
            [],
        )
        .expect("insert second companion message");

        let default_memories =
            list_memories_from_conn(&conn, DEFAULT_COMPANION_ID).expect("read default memories");
        let second_memories =
            list_memories_from_conn(&conn, "companion_second").expect("read second memories");
        assert_eq!(default_memories.len(), 1);
        assert_eq!(default_memories[0].content, "默认角色记忆");
        assert_eq!(second_memories.len(), 1);
        assert_eq!(second_memories[0].content, "第二角色记忆");
        assert_eq!(
            read_all_messages(&conn, DEFAULT_COMPANION_ID).expect("read default messages")[0]
                .content,
            "默认角色对话"
        );
        assert_eq!(
            read_all_messages(&conn, "companion_second").expect("read second messages")[0].content,
            "第二角色对话"
        );

        clear_memories_from_conn(&conn, DEFAULT_COMPANION_ID).expect("clear default memories");
        assert!(list_memories_from_conn(&conn, DEFAULT_COMPANION_ID)
            .expect("read cleared default memories")
            .is_empty());
        assert_eq!(
            list_memories_from_conn(&conn, "companion_second")
                .expect("read remaining second memories")
                .len(),
            1
        );
    }

    #[test]
    fn purge_soft_deleted_memories_removes_legacy_hidden_rows() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        initialize_schema(&conn).expect("initialize schema");
        conn.execute(
            "INSERT INTO pet_memories
             (id, memory_type, content, importance, tags_json, tags, deleted, created_at, updated_at)
             VALUES (1, 'preference', '旧的隐藏记录', 5, '[]', '', 1, '1', '1')",
            [],
        )
        .expect("insert soft deleted memory");

        purge_soft_deleted_memories(&conn).expect("purge hidden memories");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM pet_memories", [], |row| row.get(0))
            .expect("query memories");
        assert_eq!(count, 0);
    }
}
