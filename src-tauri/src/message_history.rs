//! Keyset pagination bounds IPC payloads without rewriting legacy records.
use crate::ai_memory::PetMemoryMessage;
use rusqlite::{params, Connection};
use serde::Serialize;
const PAGE_BYTES: usize = 8 * 1024 * 1024;

/// Bound the serialized JSON too: escaping and pretty-printing can expand text.
pub fn serialize_bounded(value: &impl Serialize, limit: usize) -> Result<Vec<u8>, String> {
    struct BoundedBuffer {
        bytes: Vec<u8>,
        limit: usize,
    }
    impl std::io::Write for BoundedBuffer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
                return Err(std::io::Error::other("export size limit"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut output = BoundedBuffer {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer_pretty(&mut output, value)
        .map_err(|_| "历史导出超过大小限制或序列化失败，未写入目标文件。")?;
    Ok(output.bytes)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageMessage {
    #[serde(flatten)]
    message: PetMemoryMessage,
    content_deferred: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagePage {
    pub messages: Vec<PageMessage>,
    pub next_before: Option<u64>,
    pub has_more: bool,
    pub companion_id: String,
}

fn row_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<PetMemoryMessage> {
    Ok(PetMemoryMessage {
        id: row.get::<_, i64>(0)?.max(0) as u64,
        companion_id: row.get(1)?,
        role: row.get(2)?,
        content: row.get(3)?,
        created_at: row.get(4)?,
    })
}

pub fn read_page(
    conn: &Connection,
    companion: &str,
    before: Option<u64>,
    limit: usize,
) -> Result<MessagePage, String> {
    let limit = limit.clamp(1, 50);
    let mut stmt = conn
        .prepare(
            "SELECT id, companion_id, role,
        CASE WHEN length(CAST(content AS BLOB)) <= ?4 THEN content ELSE '' END, created_at,
        length(CAST(content AS BLOB)) FROM pet_memory_messages
        WHERE companion_id=?1 AND id < ?2 ORDER BY id DESC LIMIT ?3",
        )
        .map_err(|_| "读取历史分页失败")?;
    let mut rows = stmt
        .query(params![
            companion,
            before.unwrap_or(i64::MAX as u64).min(i64::MAX as u64) as i64,
            limit as i64,
            PAGE_BYTES as i64
        ])
        .map_err(|_| "读取历史分页失败")?;
    let mut messages = Vec::new();
    let mut budget = PAGE_BYTES - 64 * 1024; // Reserve wrapper, metadata and deferred entries.
    while let Some(row) = rows.next().map_err(|_| "读取历史分页失败")? {
        let mut message = row_message(row).map_err(|_| "历史记录无效")?;
        let actual_bytes = row.get::<_, i64>(5).map_err(|_| "历史记录无效")?;
        let cost = serde_json::to_vec(&message)
            .map_err(|_| "历史序列化失败")?
            .len();
        let deferred = actual_bytes > PAGE_BYTES as i64 || cost > budget;
        if deferred {
            message.content = "较大消息，点击按需读取。".into();
        } else {
            budget -= cost;
        }
        messages.push(PageMessage {
            message,
            content_deferred: deferred,
        });
    }
    let next_before = messages.last().map(|entry| entry.message.id);
    let has_more = if let Some(id) = next_before {
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM pet_memory_messages WHERE companion_id=?1 AND id<?2)",
            params![companion, id as i64],
            |row| row.get(0),
        )
        .map_err(|_| "检查历史游标失败")?
    } else {
        false
    };
    messages.reverse();
    Ok(MessagePage {
        messages,
        next_before,
        has_more,
        companion_id: companion.into(),
    })
}

pub fn read_one(conn: &Connection, companion: &str, id: u64) -> Result<PetMemoryMessage, String> {
    let size:i64=conn.query_row("SELECT length(CAST(content AS BLOB)) FROM pet_memory_messages WHERE companion_id=?1 AND id=?2",params![companion,id.min(i64::MAX as u64) as i64],|row|row.get(0)).map_err(|_| "消息不存在")?;
    if size > crate::chat_images::MAX_MESSAGE_BYTES as i64 {
        return Err("旧消息超过单条读取限制，请通过本机导出查看；原记录未修改。".into());
    }
    conn.query_row("SELECT id,companion_id,role,content,created_at FROM pet_memory_messages WHERE companion_id=?1 AND id=?2",params![companion,id as i64],row_message).map_err(|_| "消息读取失败".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn export_budget_includes_json_escaping_and_metadata() {
        let value = serde_json::json!({"messages": ["\u{0000}".repeat(100)]});
        assert!(serialize_bounded(&value, 200).is_err());
        let output = serialize_bounded(&value, 1000).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&output).unwrap(),
            value
        );
    }
    #[test]
    fn pagination_keeps_stable_cursor_and_defers_large_legacy_records() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE pet_memory_messages(id INTEGER PRIMARY KEY,companion_id TEXT,role TEXT,content TEXT,created_at TEXT);").unwrap();
        for id in 1..=45 {
            conn.execute(
                "INSERT INTO pet_memory_messages VALUES(?1,'test','assistant',?2,'1')",
                params![
                    id,
                    if id == 30 {
                        "x".repeat(PAGE_BYTES)
                    } else {
                        format!("message-{id}")
                    }
                ],
            )
            .unwrap();
        }
        let page = read_page(&conn, "test", None, 20).unwrap();
        assert!(page.has_more);
        assert_eq!(page.next_before, Some(26));
        assert_eq!(page.messages.first().unwrap().message.id, 26);
        assert!(
            page.messages
                .iter()
                .find(|entry| entry.message.id == 30)
                .unwrap()
                .content_deferred
        );
        assert!(serde_json::to_vec(&page).unwrap().len() <= PAGE_BYTES);
        assert_eq!(
            read_one(&conn, "test", 30).unwrap().content.len(),
            PAGE_BYTES
        );
        assert!(read_one(&conn, "another", 30).is_err());
        let next = read_page(&conn, "test", page.next_before, 20).unwrap();
        assert_eq!(next.next_before, Some(6));
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM pet_memory_messages", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            45
        );
    }
}
