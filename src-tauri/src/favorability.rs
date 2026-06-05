use std::{
    collections::HashSet,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::{
    ai_memory::{self, PetMemoryMessage},
    app_data,
};

const MEMORY_DB_FILE_NAME: &str = "pet-memory.db";
const LOCAL_USER_ID: &str = "local";
const FAVORABILITY_MIN: i32 = -9999;
const FAVORABILITY_MAX: i32 = 9999;
const MOOD_MIN: i32 = 0;
const MOOD_MAX: i32 = 100;
const TRUST_MIN: i32 = -100;
const TRUST_MAX: i32 = 100;
const INTIMACY_MIN: i32 = -100;
const INTIMACY_MAX: i32 = 100;
const DAILY_POSITIVE_LIMIT: i32 = 30;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionStatus {
    pub character_id: String,
    pub favorability_enabled: bool,
    pub favorability: i32,
    pub relationship_stage: String,
    pub relationship_stage_name: String,
    pub mood: i32,
    pub trust: i32,
    pub intimacy: i32,
    pub daily_gain: i32,
    pub last_interaction_time: Option<String>,
    pub last_change_reason: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FavorabilityLog {
    pub id: u64,
    pub character_id: String,
    pub message_id: Option<u64>,
    pub old_favorability: i32,
    pub change_value: i32,
    pub new_favorability: i32,
    pub old_stage: String,
    pub new_stage: String,
    pub old_mood: i32,
    pub mood_change: i32,
    pub new_mood: i32,
    pub old_trust: i32,
    pub trust_change: i32,
    pub new_trust: i32,
    pub old_intimacy: i32,
    pub intimacy_change: i32,
    pub new_intimacy: i32,
    pub reason: String,
    pub source: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FavorabilityScore {
    #[serde(default, alias = "favorability_change")]
    pub favorability_change: i32,
    #[serde(default, alias = "mood_change")]
    pub mood_change: i32,
    #[serde(default, alias = "trust_change")]
    pub trust_change: i32,
    #[serde(default, alias = "intimacy_change")]
    pub intimacy_change: i32,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FavorabilityChangeResult {
    pub changed: bool,
    pub status: CompanionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log: Option<FavorabilityLog>,
}

#[derive(Debug, Clone)]
struct RuleScore {
    favorability_change: i32,
    mood_change: i32,
    trust_change: i32,
    intimacy_change: i32,
    reason: String,
    positive_cap: Option<i32>,
}

#[derive(Debug, Clone)]
struct StatusRow {
    character_id: String,
    favorability_enabled: bool,
    favorability: i32,
    relationship_stage: String,
    mood: i32,
    trust: i32,
    intimacy: i32,
    daily_gain: i32,
    last_interaction_time: Option<String>,
    updated_at: String,
}

pub fn get_current_companion_status(app: &AppHandle) -> Result<CompanionStatus, String> {
    let companion_id = ai_memory::current_companion_id(app)?;
    get_companion_status(app, &companion_id)
}

pub fn get_companion_status(
    app: &AppHandle,
    character_id: &str,
) -> Result<CompanionStatus, String> {
    let character_id = safe_character_id(character_id)?;
    let conn = open_connection(app)?;
    ensure_status(&conn, character_id)
}

pub fn set_current_enabled(app: &AppHandle, enabled: bool) -> Result<CompanionStatus, String> {
    let companion_id = ai_memory::current_companion_id(app)?;
    set_enabled(app, &companion_id, enabled)
}

pub fn set_enabled(
    app: &AppHandle,
    character_id: &str,
    enabled: bool,
) -> Result<CompanionStatus, String> {
    let character_id = safe_character_id(character_id)?;
    let conn = open_connection(app)?;
    let mut row = ensure_status_row(&conn, character_id)?;
    let old = row.clone();
    row.favorability_enabled = enabled;
    row.updated_at = app_data::now_seconds();
    update_status_row(&conn, &row)?;
    insert_log(
        &conn,
        &old,
        &row,
        0,
        0,
        0,
        0,
        if enabled {
            "好感度系统已开启"
        } else {
            "好感度系统已关闭"
        },
        "system",
        None,
    )?;
    status_from_row(&conn, row)
}

pub fn set_current_favorability(app: &AppHandle, value: i32) -> Result<CompanionStatus, String> {
    let companion_id = ai_memory::current_companion_id(app)?;
    set_favorability(app, &companion_id, value)
}

pub fn set_favorability(
    app: &AppHandle,
    character_id: &str,
    value: i32,
) -> Result<CompanionStatus, String> {
    let character_id = safe_character_id(character_id)?;
    let conn = open_connection(app)?;
    let mut row = ensure_status_row(&conn, character_id)?;
    let old = row.clone();
    row.favorability = value.clamp(FAVORABILITY_MIN, FAVORABILITY_MAX);
    row.relationship_stage = relationship_stage(row.favorability).to_string();
    row.updated_at = app_data::now_seconds();
    update_status_row(&conn, &row)?;
    insert_log(
        &conn,
        &old,
        &row,
        row.favorability - old.favorability,
        0,
        0,
        0,
        "用户手动设置好感度",
        "manual",
        None,
    )?;
    status_from_row(&conn, row)
}

pub fn reset_current_favorability(app: &AppHandle) -> Result<CompanionStatus, String> {
    let companion_id = ai_memory::current_companion_id(app)?;
    reset_favorability(app, &companion_id)
}

pub fn reset_favorability(app: &AppHandle, character_id: &str) -> Result<CompanionStatus, String> {
    let character_id = safe_character_id(character_id)?;
    let conn = open_connection(app)?;
    let mut row = ensure_status_row(&conn, character_id)?;
    let old = row.clone();
    row.favorability = 0;
    row.relationship_stage = relationship_stage(0).to_string();
    row.daily_gain = 0;
    row.updated_at = app_data::now_seconds();
    update_status_row(&conn, &row)?;
    insert_log(
        &conn,
        &old,
        &row,
        -old.favorability,
        0,
        0,
        0,
        "用户重置好感度为 0",
        "reset",
        None,
    )?;
    status_from_row(&conn, row)
}

pub fn list_current_logs(app: &AppHandle, limit: usize) -> Result<Vec<FavorabilityLog>, String> {
    let companion_id = ai_memory::current_companion_id(app)?;
    list_logs(app, &companion_id, limit)
}

pub fn list_logs(
    app: &AppHandle,
    character_id: &str,
    limit: usize,
) -> Result<Vec<FavorabilityLog>, String> {
    let character_id = safe_character_id(character_id)?;
    let conn = open_connection(app)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, character_id, message_id, old_favorability, change_value, new_favorability,
                    old_stage, new_stage, old_mood, mood_change, new_mood, old_trust,
                    trust_change, new_trust, old_intimacy, intimacy_change, new_intimacy,
                    reason, source, created_at
             FROM favorability_logs
             WHERE user_id = ?1 AND character_id = ?2
             ORDER BY id DESC
             LIMIT ?3",
        )
        .map_err(|err| format!("读取好感度日志失败：{err}"))?;
    let rows = stmt
        .query_map(
            params![LOCAL_USER_ID, character_id, limit.max(1).min(200) as i64],
            row_to_log,
        )
        .map_err(|err| format!("读取好感度日志失败：{err}"))?;
    collect_logs(rows)
}

pub fn delete_companion_data(app: &AppHandle, character_id: &str) -> Result<(), String> {
    let character_id = safe_character_id(character_id)?;
    let conn = open_connection(app)?;
    conn.execute(
        "DELETE FROM favorability_logs WHERE user_id = ?1 AND character_id = ?2",
        params![LOCAL_USER_ID, character_id],
    )
    .map_err(|err| format!("删除伴侣好感度日志失败：{err}"))?;
    conn.execute(
        "DELETE FROM companion_status WHERE user_id = ?1 AND character_id = ?2",
        params![LOCAL_USER_ID, character_id],
    )
    .map_err(|err| format!("删除伴侣好感度状态失败：{err}"))?;
    Ok(())
}

pub fn apply_dialogue_change(
    app: &AppHandle,
    character_id: &str,
    user_message: &str,
    assistant_message: &str,
    ai_score: FavorabilityScore,
    recent_messages: &[PetMemoryMessage],
) -> Result<FavorabilityChangeResult, String> {
    let character_id = safe_character_id(character_id)?;
    let conn = open_connection(app)?;
    let mut row = ensure_status_row(&conn, character_id)?;
    if !row.favorability_enabled {
        return Ok(FavorabilityChangeResult {
            changed: false,
            status: status_from_row(&conn, row)?,
            log: None,
        });
    }

    let old = row.clone();
    let now = app_data::now_seconds();
    if day_number(row.last_interaction_time.as_deref()) != day_number(Some(&now)) {
        row.daily_gain = 0;
    }

    let rule_score = rule_score(user_message, assistant_message, recent_messages);
    let mut favorability_change =
        ai_score.favorability_change.clamp(-10, 10) + rule_score.favorability_change;
    if let Some(cap) = rule_score.positive_cap {
        favorability_change = favorability_change.min(cap);
    }
    favorability_change = favorability_change.clamp(-20, 10);
    if favorability_change > 0 {
        let remaining = (DAILY_POSITIVE_LIMIT - row.daily_gain).max(0);
        favorability_change = favorability_change.min(remaining);
    }

    let mood_change = (ai_score.mood_change.clamp(-10, 10) + rule_score.mood_change).clamp(-15, 15);
    let trust_change = (ai_score.trust_change.clamp(-5, 5) + rule_score.trust_change).clamp(-8, 8);
    let intimacy_change =
        (ai_score.intimacy_change.clamp(-5, 5) + rule_score.intimacy_change).clamp(-8, 8);

    row.favorability =
        (row.favorability + favorability_change).clamp(FAVORABILITY_MIN, FAVORABILITY_MAX);
    row.relationship_stage = relationship_stage(row.favorability).to_string();
    row.mood = (row.mood + mood_change).clamp(MOOD_MIN, MOOD_MAX);
    row.trust = (row.trust + trust_change).clamp(TRUST_MIN, TRUST_MAX);
    row.intimacy = (row.intimacy + intimacy_change).clamp(INTIMACY_MIN, INTIMACY_MAX);
    if favorability_change > 0 {
        row.daily_gain = (row.daily_gain + favorability_change).clamp(0, DAILY_POSITIVE_LIMIT);
    }
    row.last_interaction_time = Some(now.clone());
    row.updated_at = now;
    update_status_row(&conn, &row)?;

    let changed =
        favorability_change != 0 || mood_change != 0 || trust_change != 0 || intimacy_change != 0;
    let log = if changed {
        let reason = merged_reason(&ai_score.reason, &rule_score.reason);
        Some(insert_log(
            &conn,
            &old,
            &row,
            favorability_change,
            mood_change,
            trust_change,
            intimacy_change,
            &reason,
            "dialogue",
            None,
        )?)
    } else {
        None
    };

    Ok(FavorabilityChangeResult {
        changed,
        status: status_from_row(&conn, row)?,
        log,
    })
}

pub fn relationship_stage(score: i32) -> &'static str {
    if score <= -1000 {
        "hostile"
    } else if score <= -300 {
        "dislike"
    } else if score <= -1 {
        "guarded"
    } else if score == 0 {
        "neutral"
    } else if score < 100 {
        "acquaintance"
    } else if score < 300 {
        "familiar"
    } else if score < 700 {
        "friend"
    } else if score < 1300 {
        "close"
    } else if score < 2200 {
        "dependent"
    } else {
        "bond"
    }
}

pub fn relationship_stage_name(stage: &str) -> &'static str {
    match stage {
        "hostile" => "敌对",
        "dislike" => "讨厌",
        "guarded" => "戒备",
        "neutral" => "初始",
        "acquaintance" => "初识",
        "familiar" => "熟悉",
        "friend" => "朋友",
        "close" => "亲近",
        "dependent" => "依赖",
        "bond" => "羁绊",
        _ => "未知",
    }
}

fn open_connection(app: &AppHandle) -> Result<Connection, String> {
    app_data::ensure_data_files(app)?;
    fs::create_dir_all(memory_dir(app)?).map_err(|err| format!("创建记忆目录失败：{err}"))?;
    let conn = Connection::open(memory_db_file(app)?)
        .map_err(|err| format!("打开好感度数据库失败：{err}"))?;
    initialize_schema(&conn)?;
    Ok(conn)
}

fn initialize_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS companion_status (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL DEFAULT 'local',
            character_id TEXT NOT NULL,
            favorability_enabled INTEGER NOT NULL DEFAULT 0,
            favorability INTEGER NOT NULL DEFAULT 0,
            mood INTEGER NOT NULL DEFAULT 50,
            trust INTEGER NOT NULL DEFAULT 0,
            intimacy INTEGER NOT NULL DEFAULT 0,
            relationship_stage TEXT NOT NULL DEFAULT 'neutral',
            daily_gain INTEGER NOT NULL DEFAULT 0,
            last_interaction_time TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(user_id, character_id)
        );

        CREATE TABLE IF NOT EXISTS favorability_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL DEFAULT 'local',
            character_id TEXT NOT NULL,
            message_id INTEGER,
            old_favorability INTEGER NOT NULL,
            change_value INTEGER NOT NULL,
            new_favorability INTEGER NOT NULL,
            old_mood INTEGER,
            mood_change INTEGER,
            new_mood INTEGER,
            old_trust INTEGER,
            trust_change INTEGER,
            new_trust INTEGER,
            old_intimacy INTEGER,
            intimacy_change INTEGER,
            new_intimacy INTEGER,
            old_stage TEXT,
            new_stage TEXT,
            reason TEXT,
            source TEXT NOT NULL DEFAULT 'dialogue',
            created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_companion_status_character
        ON companion_status(user_id, character_id);

        CREATE INDEX IF NOT EXISTS idx_favorability_logs_character
        ON favorability_logs(user_id, character_id, id DESC);
        "#,
    )
    .map_err(|err| format!("初始化好感度数据库失败：{err}"))?;
    ensure_status_columns(conn)
}

fn ensure_status_columns(conn: &Connection) -> Result<(), String> {
    let status_columns = table_columns(conn, "companion_status")?;
    for (column, ddl) in [
        (
            "trust",
            "ALTER TABLE companion_status ADD COLUMN trust INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "intimacy",
            "ALTER TABLE companion_status ADD COLUMN intimacy INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "daily_gain",
            "ALTER TABLE companion_status ADD COLUMN daily_gain INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "last_interaction_time",
            "ALTER TABLE companion_status ADD COLUMN last_interaction_time TEXT",
        ),
    ] {
        if !status_columns.contains(column) {
            conn.execute(ddl, [])
                .map_err(|err| format!("升级好感度状态表失败：{err}"))?;
        }
    }
    Ok(())
}

fn table_columns(conn: &Connection, table_name: &str) -> Result<HashSet<String>, String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table_name})"))
        .map_err(|err| format!("读取好感度数据库结构失败：{err}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|err| format!("读取好感度数据库结构失败：{err}"))?;
    rows.collect::<Result<HashSet<_>, _>>()
        .map_err(|err| format!("读取好感度数据库结构失败：{err}"))
}

fn ensure_status(conn: &Connection, character_id: &str) -> Result<CompanionStatus, String> {
    let row = ensure_status_row(conn, character_id)?;
    status_from_row(conn, row)
}

fn ensure_status_row(conn: &Connection, character_id: &str) -> Result<StatusRow, String> {
    if let Some(row) = load_status_row(conn, character_id)? {
        return Ok(row);
    }

    let now = app_data::now_seconds();
    conn.execute(
        "INSERT INTO companion_status
         (user_id, character_id, favorability_enabled, favorability, mood, trust, intimacy,
          relationship_stage, daily_gain, created_at, updated_at)
         VALUES (?1, ?2, 0, 0, 50, 0, 0, 'neutral', 0, ?3, ?3)",
        params![LOCAL_USER_ID, character_id, now],
    )
    .map_err(|err| format!("创建好感度状态失败：{err}"))?;
    load_status_row(conn, character_id)?.ok_or_else(|| "创建好感度状态后读取失败".to_string())
}

fn load_status_row(conn: &Connection, character_id: &str) -> Result<Option<StatusRow>, String> {
    conn.query_row(
        "SELECT character_id, favorability_enabled, favorability, relationship_stage, mood,
                trust, intimacy, daily_gain, last_interaction_time, updated_at
         FROM companion_status
         WHERE user_id = ?1 AND character_id = ?2",
        params![LOCAL_USER_ID, character_id],
        row_to_status,
    )
    .optional()
    .map_err(|err| format!("读取好感度状态失败：{err}"))
}

fn update_status_row(conn: &Connection, row: &StatusRow) -> Result<(), String> {
    conn.execute(
        "UPDATE companion_status
         SET favorability_enabled = ?3,
             favorability = ?4,
             relationship_stage = ?5,
             mood = ?6,
             trust = ?7,
             intimacy = ?8,
             daily_gain = ?9,
             last_interaction_time = ?10,
             updated_at = ?11
         WHERE user_id = ?1 AND character_id = ?2",
        params![
            LOCAL_USER_ID,
            row.character_id,
            bool_to_i64(row.favorability_enabled),
            row.favorability,
            row.relationship_stage,
            row.mood,
            row.trust,
            row.intimacy,
            row.daily_gain,
            row.last_interaction_time,
            row.updated_at,
        ],
    )
    .map_err(|err| format!("保存好感度状态失败：{err}"))?;
    Ok(())
}

fn status_from_row(conn: &Connection, row: StatusRow) -> Result<CompanionStatus, String> {
    let stage = relationship_stage(row.favorability).to_string();
    Ok(CompanionStatus {
        character_id: row.character_id.clone(),
        favorability_enabled: row.favorability_enabled,
        favorability: row.favorability,
        relationship_stage: stage.clone(),
        relationship_stage_name: relationship_stage_name(&stage).to_string(),
        mood: row.mood,
        trust: row.trust,
        intimacy: row.intimacy,
        daily_gain: row.daily_gain,
        last_interaction_time: row.last_interaction_time,
        last_change_reason: latest_reason(conn, &row.character_id)?,
        updated_at: row.updated_at,
    })
}

fn latest_reason(conn: &Connection, character_id: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT reason
         FROM favorability_logs
         WHERE user_id = ?1 AND character_id = ?2
         ORDER BY id DESC
         LIMIT 1",
        params![LOCAL_USER_ID, character_id],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(|err| format!("读取最近好感度变化失败：{err}"))
}

fn insert_log(
    conn: &Connection,
    old: &StatusRow,
    new: &StatusRow,
    favorability_change: i32,
    mood_change: i32,
    trust_change: i32,
    intimacy_change: i32,
    reason: &str,
    source: &str,
    message_id: Option<u64>,
) -> Result<FavorabilityLog, String> {
    let created_at = app_data::now_seconds();
    conn.execute(
        "INSERT INTO favorability_logs
         (user_id, character_id, message_id, old_favorability, change_value, new_favorability,
          old_mood, mood_change, new_mood, old_trust, trust_change, new_trust,
          old_intimacy, intimacy_change, new_intimacy, old_stage, new_stage, reason, source,
          created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                 ?18, ?19, ?20)",
        params![
            LOCAL_USER_ID,
            new.character_id,
            optional_sql_id(message_id)?,
            old.favorability,
            favorability_change,
            new.favorability,
            old.mood,
            mood_change,
            new.mood,
            old.trust,
            trust_change,
            new.trust,
            old.intimacy,
            intimacy_change,
            new.intimacy,
            old.relationship_stage,
            new.relationship_stage,
            reason.trim(),
            source,
            created_at,
        ],
    )
    .map_err(|err| format!("写入好感度日志失败：{err}"))?;
    let id = conn.last_insert_rowid();
    load_log(conn, id as u64)?.ok_or_else(|| "写入好感度日志后读取失败".to_string())
}

fn load_log(conn: &Connection, id: u64) -> Result<Option<FavorabilityLog>, String> {
    conn.query_row(
        "SELECT id, character_id, message_id, old_favorability, change_value, new_favorability,
                old_stage, new_stage, old_mood, mood_change, new_mood, old_trust,
                trust_change, new_trust, old_intimacy, intimacy_change, new_intimacy,
                reason, source, created_at
         FROM favorability_logs
         WHERE id = ?1",
        params![to_sql_id(id)?],
        row_to_log,
    )
    .optional()
    .map_err(|err| format!("读取好感度日志失败：{err}"))
}

fn row_to_status(row: &rusqlite::Row<'_>) -> rusqlite::Result<StatusRow> {
    let favorability = row
        .get::<_, i64>(2)?
        .clamp(FAVORABILITY_MIN as i64, FAVORABILITY_MAX as i64) as i32;
    let stored_stage: String = row.get(3)?;
    let stage = if stored_stage.trim().is_empty() {
        relationship_stage(favorability).to_string()
    } else {
        stored_stage
    };
    Ok(StatusRow {
        character_id: row.get(0)?,
        favorability_enabled: row.get::<_, i64>(1)? != 0,
        favorability,
        relationship_stage: stage,
        mood: row
            .get::<_, i64>(4)?
            .clamp(MOOD_MIN as i64, MOOD_MAX as i64) as i32,
        trust: row
            .get::<_, i64>(5)?
            .clamp(TRUST_MIN as i64, TRUST_MAX as i64) as i32,
        intimacy: row
            .get::<_, i64>(6)?
            .clamp(INTIMACY_MIN as i64, INTIMACY_MAX as i64) as i32,
        daily_gain: row.get::<_, i64>(7)?.clamp(0, DAILY_POSITIVE_LIMIT as i64) as i32,
        last_interaction_time: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn row_to_log(row: &rusqlite::Row<'_>) -> rusqlite::Result<FavorabilityLog> {
    let id: i64 = row.get(0)?;
    let message_id: Option<i64> = row.get(2)?;
    Ok(FavorabilityLog {
        id: id.max(0) as u64,
        character_id: row.get(1)?,
        message_id: message_id.map(|value| value.max(0) as u64),
        old_favorability: row.get::<_, i64>(3)? as i32,
        change_value: row.get::<_, i64>(4)? as i32,
        new_favorability: row.get::<_, i64>(5)? as i32,
        old_stage: row
            .get::<_, Option<String>>(6)?
            .unwrap_or_else(|| "neutral".to_string()),
        new_stage: row
            .get::<_, Option<String>>(7)?
            .unwrap_or_else(|| "neutral".to_string()),
        old_mood: row.get::<_, Option<i64>>(8)?.unwrap_or(50) as i32,
        mood_change: row.get::<_, Option<i64>>(9)?.unwrap_or(0) as i32,
        new_mood: row.get::<_, Option<i64>>(10)?.unwrap_or(50) as i32,
        old_trust: row.get::<_, Option<i64>>(11)?.unwrap_or(0) as i32,
        trust_change: row.get::<_, Option<i64>>(12)?.unwrap_or(0) as i32,
        new_trust: row.get::<_, Option<i64>>(13)?.unwrap_or(0) as i32,
        old_intimacy: row.get::<_, Option<i64>>(14)?.unwrap_or(0) as i32,
        intimacy_change: row.get::<_, Option<i64>>(15)?.unwrap_or(0) as i32,
        new_intimacy: row.get::<_, Option<i64>>(16)?.unwrap_or(0) as i32,
        reason: row.get::<_, Option<String>>(17)?.unwrap_or_default(),
        source: row.get(18)?,
        created_at: row.get(19)?,
    })
}

fn collect_logs<F>(rows: rusqlite::MappedRows<'_, F>) -> Result<Vec<FavorabilityLog>, String>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<FavorabilityLog>,
{
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("读取好感度日志失败：{err}"))
}

fn rule_score(
    user_message: &str,
    assistant_message: &str,
    recent_messages: &[PetMemoryMessage],
) -> RuleScore {
    let normalized = normalize_text(user_message);
    if normalized.is_empty() || is_trivial_message(&normalized) {
        return RuleScore {
            favorability_change: 0,
            mood_change: 1,
            trust_change: 0,
            intimacy_change: 0,
            reason: "短消息或无意义消息不提升长期好感度".to_string(),
            positive_cap: Some(0),
        };
    }

    let mut score = RuleScore {
        favorability_change: 0,
        mood_change: 0,
        trust_change: 0,
        intimacy_change: 0,
        reason: String::new(),
        positive_cap: None,
    };

    if contains_any(
        &normalized,
        &[
            "谢谢",
            "辛苦",
            "喜欢你",
            "想你",
            "陪你",
            "抱抱",
            "夸夸",
            "真好",
            "可爱",
        ],
    ) {
        score.favorability_change += 2;
        score.mood_change += 3;
        score.trust_change += 1;
        score.intimacy_change += 1;
        score.reason = "用户表达了关心、感谢或亲近".to_string();
    }
    if contains_any(
        &normalized,
        &["认真", "告诉我", "我记得", "一起", "慢慢说", "听你说"],
    ) {
        score.favorability_change += 1;
        score.trust_change += 1;
        if score.reason.is_empty() {
            score.reason = "用户进行了认真交流".to_string();
        }
    }
    if contains_any(
        &normalized,
        &[
            "随便",
            "无所谓",
            "别烦",
            "烦死",
            "讨厌",
            "闭嘴",
            "滚",
            "冷淡",
        ],
    ) {
        score.favorability_change -= 5;
        score.mood_change -= 6;
        score.trust_change -= 2;
        score.intimacy_change -= 2;
        score.reason = "用户表达了冷淡或负向态度".to_string();
    }
    if contains_any(
        &normalized,
        &["废物", "垃圾", "蠢", "笨蛋", "讨厌你", "去死"],
    ) {
        score.favorability_change -= 12;
        score.mood_change -= 10;
        score.trust_change -= 4;
        score.intimacy_change -= 4;
        score.reason = "用户使用了攻击性表达".to_string();
    }

    let repeat_count = repeated_user_message_count(&normalized, recent_messages);
    if score.favorability_change > 0 && repeat_count > 1 {
        let decayed = match repeat_count {
            2 => ((score.favorability_change as f32) * 0.5).round() as i32,
            3 => ((score.favorability_change as f32) * 0.2).round() as i32,
            _ => 0,
        };
        score.favorability_change = decayed;
        score.reason = if score.reason.is_empty() {
            "重复相似表达，好感度加成衰减".to_string()
        } else {
            format!("{}；重复表达已衰减", score.reason)
        };
    }

    if score.reason.is_empty() && !assistant_message.trim().is_empty() {
        score.reason = "普通对话，小幅更新短期状态".to_string();
    }

    score
}

fn merged_reason(ai_reason: &str, rule_reason: &str) -> String {
    let ai_reason = ai_reason.trim();
    let rule_reason = rule_reason.trim();
    match (ai_reason.is_empty(), rule_reason.is_empty()) {
        (true, true) => "对话后关系状态自然变化".to_string(),
        (false, true) => ai_reason.chars().take(120).collect(),
        (true, false) => rule_reason.chars().take(120).collect(),
        (false, false) => format!(
            "{}；{}",
            ai_reason.chars().take(80).collect::<String>(),
            rule_reason.chars().take(80).collect::<String>()
        ),
    }
}

fn repeated_user_message_count(normalized: &str, recent_messages: &[PetMemoryMessage]) -> usize {
    recent_messages
        .iter()
        .rev()
        .filter(|message| message.role == "user")
        .take(5)
        .filter(|message| similar_message(normalized, &normalize_text(&message.content)))
        .count()
}

fn similar_message(left: &str, right: &str) -> bool {
    if left.is_empty() || right.is_empty() {
        return false;
    }
    left == right
        || (left.len() >= 6 && right.len() >= 6 && (left.contains(right) || right.contains(left)))
}

fn is_trivial_message(value: &str) -> bool {
    if value.chars().count() <= 1 {
        return true;
    }
    matches!(
        value,
        "嗯" | "哦" | "额" | "啊" | "哈" | "哈哈" | "。" | "？" | "?" | "." | "随便" | "不知道"
    )
}

fn normalize_text(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect()
}

fn contains_any(input: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|keyword| input.contains(keyword))
}

fn day_number(timestamp: Option<&str>) -> Option<u64> {
    timestamp
        .and_then(|value| value.parse::<u64>().ok())
        .map(|seconds| seconds / 86_400)
}

fn memory_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app_data::memory_dir(app)
}

fn memory_db_file(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(memory_dir(app)?.join(MEMORY_DB_FILE_NAME))
}

fn safe_character_id(character_id: &str) -> Result<&str, String> {
    let character_id = character_id.trim();
    if character_id.is_empty() {
        return Err("伴侣 ID 不能为空".to_string());
    }
    let safe = character_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-');
    if !safe || character_id == "." || character_id == ".." {
        return Err("伴侣 ID 不合法".to_string());
    }
    Ok(character_id)
}

fn bool_to_i64(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn optional_sql_id(id: Option<u64>) -> Result<Option<i64>, String> {
    id.map(to_sql_id).transpose()
}

fn to_sql_id(id: u64) -> Result<i64, String> {
    if id == 0 || id > i64::MAX as u64 {
        return Err("好感度日志 ID 超出 SQLite 支持范围".to_string());
    }
    Ok(id as i64)
}

#[allow(dead_code)]
fn now_seconds_u64() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relationship_stage_supports_negative_values() {
        assert_eq!(relationship_stage(-1200), "hostile");
        assert_eq!(relationship_stage(-500), "dislike");
        assert_eq!(relationship_stage(-1), "guarded");
        assert_eq!(relationship_stage(0), "neutral");
        assert_eq!(relationship_stage(2200), "bond");
    }

    #[test]
    fn trivial_messages_cap_positive_gain() {
        let score = rule_score("嗯", "", &[]);
        assert_eq!(score.positive_cap, Some(0));
        assert_eq!(score.favorability_change, 0);
    }

    #[test]
    fn repeated_positive_messages_decay() {
        let recent = vec![
            PetMemoryMessage {
                id: 1,
                companion_id: "default".to_string(),
                role: "user".to_string(),
                content: "喜欢你".to_string(),
                created_at: "1".to_string(),
            },
            PetMemoryMessage {
                id: 2,
                companion_id: "default".to_string(),
                role: "user".to_string(),
                content: "喜欢你".to_string(),
                created_at: "2".to_string(),
            },
        ];
        let score = rule_score("喜欢你", "", &recent);
        assert!(score.favorability_change <= 1);
    }
}
