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

use crate::app_data;

const MEMORY_DB_FILE_NAME: &str = "pet-memory.db";
const LEGACY_MEMORY_JSON_FILE_NAME: &str = "pet-memory.json";
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

    let mut conn = open_connection(app)?;
    let id = next_unique_id(&conn, "pet_memory_messages")?;
    conn.execute(
        "INSERT INTO pet_memory_messages (id, role, content, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![to_sql_id(id)?, role, content, app_data::now_seconds()],
    )
    .map_err(|err| format!("保存宠物聊天记忆失败：{err}"))?;
    trim_messages(&conn)
}

pub fn recent_messages(app: &AppHandle, limit: usize) -> Result<Vec<PetMemoryMessage>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    let conn = open_connection(app)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, role, content, created_at
             FROM (
                SELECT id, role, content, created_at
                FROM pet_memory_messages
                ORDER BY id DESC
                LIMIT ?1
             )
             ORDER BY id ASC",
        )
        .map_err(|err| format!("读取最近聊天记忆失败：{err}"))?;

    let rows = stmt
        .query_map(params![limit as i64], row_to_message)
        .map_err(|err| format!("读取最近聊天记忆失败：{err}"))?;

    collect_rows(rows, "读取最近聊天记忆失败")
}

pub fn list_memories(app: &AppHandle) -> Result<Vec<PetMemory>, String> {
    let conn = open_connection(app)?;
    list_memories_from_conn(&conn)
}

pub fn save_memories(
    app: &AppHandle,
    drafts: Vec<PetMemoryDraft>,
) -> Result<Vec<PetMemory>, String> {
    let mut conn = open_connection(app)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("保存宠物长期记忆失败：{err}"))?;
    let mut saved = Vec::new();

    for draft in drafts {
        let Some(draft) = normalize_memory_draft(draft) else {
            continue;
        };

        let duplicate = find_duplicate_memory(&tx, &draft)?;
        let now = app_data::now_seconds();
        if let Some(mut memory) = duplicate {
            memory.content = draft.content;
            memory.importance = memory.importance.max(draft.importance);
            memory.tags = merge_tags(&memory.tags, &draft.tags);
            memory.updated_at = now;
            update_memory(&tx, &memory)?;
            saved.push(memory);
        } else {
            let memory = PetMemory {
                id: next_unique_id(&tx, "pet_memories")?,
                memory_type: draft.memory_type,
                content: draft.content,
                importance: draft.importance,
                tags: draft.tags,
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

pub fn search_memories(
    app: &AppHandle,
    query: &str,
    limit: usize,
) -> Result<Vec<PetMemory>, String> {
    let query = query.trim();
    if query.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    let conn = open_connection(app)?;
    let fts_result = search_memories_fts(&conn, query, limit);
    match fts_result {
        Ok(memories) if !memories.is_empty() => Ok(memories),
        _ => search_memories_scored(&conn, query, limit),
    }
}

pub fn delete_memory(app: &AppHandle, memory_id: u64) -> Result<(), String> {
    let conn = open_connection(app)?;
    let updated = conn
        .execute(
            "UPDATE pet_memories
             SET deleted = 1, updated_at = ?1
             WHERE id = ?2 AND deleted = 0",
            params![app_data::now_seconds(), to_sql_id(memory_id)?],
        )
        .map_err(|err| format!("删除宠物记忆失败：{err}"))?;

    if updated == 0 {
        return Err("未找到要删除的记忆".to_string());
    }

    Ok(())
}

pub fn clear_memories(app: &AppHandle) -> Result<(), String> {
    let conn = open_connection(app)?;
    conn.execute(
        "UPDATE pet_memories SET deleted = 1, updated_at = ?1 WHERE deleted = 0",
        params![app_data::now_seconds()],
    )
    .map_err(|err| format!("清空宠物记忆失败：{err}"))?;
    Ok(())
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

    let mut conn = open_connection(app)?;
    replace_store(&mut conn, &store)?;
    list_memories_from_conn(&conn)
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
    let conn = open_connection(app)?;
    read_store_from_conn(&conn)
}

fn open_connection(app: &AppHandle) -> Result<Connection, String> {
    app_data::ensure_data_files(app)?;
    fs::create_dir_all(memory_dir(app)?).map_err(|err| format!("创建记忆目录失败：{err}"))?;

    let conn = Connection::open(memory_db_file(app)?)
        .map_err(|err| format!("打开记忆数据库失败：{err}"))?;
    initialize_schema(&conn)?;
    migrate_legacy_json(app, &conn)?;
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
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS pet_memories (
            id INTEGER PRIMARY KEY,
            memory_type TEXT NOT NULL,
            content TEXT NOT NULL,
            importance INTEGER NOT NULL,
            tags_json TEXT NOT NULL,
            tags TEXT NOT NULL,
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

        PRAGMA user_version = 1;
        "#,
    )
    .map_err(|err| format!("初始化记忆数据库失败：{err}"))
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
    replace_store_in_conn(conn, &store)?;
    set_meta(conn, "legacy_json_migrated", "1")
}

fn read_store_from_conn(conn: &Connection) -> Result<PetMemoryStore, String> {
    Ok(PetMemoryStore {
        messages: read_all_messages(conn)?,
        memories: read_all_memories(conn)?,
    })
}

fn read_all_messages(conn: &Connection) -> Result<Vec<PetMemoryMessage>, String> {
    let mut stmt = conn
        .prepare("SELECT id, role, content, created_at FROM pet_memory_messages ORDER BY id ASC")
        .map_err(|err| format!("读取宠物聊天记忆失败：{err}"))?;
    let rows = stmt
        .query_map([], row_to_message)
        .map_err(|err| format!("读取宠物聊天记忆失败：{err}"))?;
    collect_rows(rows, "读取宠物聊天记忆失败")
}

fn read_all_memories(conn: &Connection) -> Result<Vec<PetMemory>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, memory_type, content, importance, tags_json, deleted, created_at, updated_at
             FROM pet_memories
             ORDER BY id ASC",
        )
        .map_err(|err| format!("读取宠物长期记忆失败：{err}"))?;
    let rows = stmt
        .query_map([], row_to_memory)
        .map_err(|err| format!("读取宠物长期记忆失败：{err}"))?;
    collect_rows(rows, "读取宠物长期记忆失败")
}

fn list_memories_from_conn(conn: &Connection) -> Result<Vec<PetMemory>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, memory_type, content, importance, tags_json, deleted, created_at, updated_at
             FROM pet_memories
             WHERE deleted = 0
             ORDER BY importance DESC, updated_at DESC",
        )
        .map_err(|err| format!("读取宠物长期记忆失败：{err}"))?;
    let rows = stmt
        .query_map([], row_to_memory)
        .map_err(|err| format!("读取宠物长期记忆失败：{err}"))?;
    collect_rows(rows, "读取宠物长期记忆失败")
}

fn search_memories_fts(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<PetMemory>, String> {
    let Some(fts_query) = fts_match_query(query) else {
        return Ok(Vec::new());
    };

    let mut stmt = conn
        .prepare(
            "SELECT m.id, m.memory_type, m.content, m.importance, m.tags_json, m.deleted, m.created_at, m.updated_at
             FROM pet_memories_fts
             JOIN pet_memories m ON m.id = pet_memories_fts.rowid
             WHERE pet_memories_fts MATCH ?1 AND m.deleted = 0
             ORDER BY bm25(pet_memories_fts) ASC, m.importance DESC, m.updated_at DESC
             LIMIT ?2",
        )
        .map_err(|err| format!("检索宠物记忆失败：{err}"))?;
    let rows = stmt
        .query_map(params![fts_query, limit as i64], row_to_memory)
        .map_err(|err| format!("检索宠物记忆失败：{err}"))?;
    collect_rows(rows, "检索宠物记忆失败")
}

fn search_memories_scored(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<PetMemory>, String> {
    let tokens = query_tokens(query);
    let mut scored = list_memories_from_conn(conn)?
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
    draft: &PetMemoryDraft,
) -> Result<Option<PetMemory>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, memory_type, content, importance, tags_json, deleted, created_at, updated_at
             FROM pet_memories
             WHERE deleted = 0 AND memory_type = ?1",
        )
        .map_err(|err| format!("查找重复宠物记忆失败：{err}"))?;
    let rows = stmt
        .query_map(params![&draft.memory_type], row_to_memory)
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
         (id, memory_type, content, importance, tags_json, tags, deleted, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            to_sql_id(memory.id)?,
            &memory.memory_type,
            &memory.content,
            i64::from(memory.importance),
            &tags_json,
            &tags_text,
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
             deleted = ?6, updated_at = ?7
         WHERE id = ?8",
        params![
            &memory.memory_type,
            &memory.content,
            i64::from(memory.importance),
            &tags_json,
            &tags_text,
            bool_to_i64(memory.deleted),
            &memory.updated_at,
            to_sql_id(memory.id)?,
        ],
    )
    .map_err(|err| format!("更新宠物长期记忆失败：{err}"))?;
    Ok(())
}

fn replace_store(conn: &mut Connection, store: &PetMemoryStore) -> Result<(), String> {
    let tx = conn
        .transaction()
        .map_err(|err| format!("导入宠物记忆失败：{err}"))?;
    replace_store_in_conn(&tx, store)?;
    tx.commit()
        .map_err(|err| format!("导入宠物记忆失败：{err}"))
}

fn replace_store_in_conn(conn: &Connection, store: &PetMemoryStore) -> Result<(), String> {
    conn.execute("DELETE FROM pet_memory_messages", [])
        .map_err(|err| format!("导入宠物聊天记忆失败：{err}"))?;
    conn.execute("DELETE FROM pet_memories", [])
        .map_err(|err| format!("导入宠物长期记忆失败：{err}"))?;

    for message in &store.messages {
        conn.execute(
            "INSERT INTO pet_memory_messages (id, role, content, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![to_sql_id(message.id)?, &message.role, &message.content, &message.created_at],
        )
        .map_err(|err| format!("导入宠物聊天记忆失败：{err}"))?;
    }

    for memory in &store.memories {
        insert_memory(conn, memory)?;
    }

    trim_messages(conn)?;
    Ok(())
}

fn trim_messages(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "DELETE FROM pet_memory_messages
         WHERE id NOT IN (
            SELECT id FROM pet_memory_messages ORDER BY id DESC LIMIT ?1
         )",
        params![MAX_MESSAGES as i64],
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
        role: row.get(1)?,
        content: row.get(2)?,
        created_at: row.get(3)?,
    })
}

fn row_to_memory(row: &rusqlite::Row<'_>) -> rusqlite::Result<PetMemory> {
    let id: i64 = row.get(0)?;
    let tags_json: String = row.get(4)?;
    let deleted: i64 = row.get(5)?;

    Ok(PetMemory {
        id: id.max(0) as u64,
        memory_type: row.get(1)?,
        content: row.get(2)?,
        importance: row.get::<_, i64>(3)?.clamp(1, 10) as u8,
        tags: parse_tags_json(&tags_json),
        deleted: deleted != 0,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
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
        memory.memory_type = normalize_memory_type(&memory.memory_type);
        let content = memory.content.trim().to_string();
        memory.content = truncate_text(&content, MAX_MEMORY_CONTENT_LEN);
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
    "event".to_string()
}

fn default_importance() -> u8 {
    5
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
}
