use crate::error::AppResult;
use crate::schema::*;
use rusqlite::{params, Connection};
use std::path::Path;

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS conversations (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL,
    native_id TEXT NOT NULL,
    title TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    model TEXT,
    project TEXT,
    message_count INTEGER NOT NULL DEFAULT 0,
    tok_input INTEGER NOT NULL DEFAULT 0,
    tok_output INTEGER NOT NULL DEFAULT 0,
    tok_cache_read INTEGER NOT NULL DEFAULT 0,
    tok_cache_write INTEGER NOT NULL DEFAULT 0,
    estimated_cost_usd REAL NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    seq INTEGER NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    timestamp INTEGER,
    model TEXT,
    tok_input INTEGER,
    tok_output INTEGER,
    tok_cache_read INTEGER,
    tok_cache_write INTEGER,
    tool_name TEXT
);

CREATE INDEX IF NOT EXISTS idx_messages_conv ON messages(conversation_id, seq);
CREATE INDEX IF NOT EXISTS idx_conversations_created ON conversations(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_conversations_source ON conversations(source);

CREATE TABLE IF NOT EXISTS tool_calls (
    id TEXT PRIMARY KEY,
    message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    conversation_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    seq INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tool_calls_name ON tool_calls(tool_name);
CREATE INDEX IF NOT EXISTS idx_tool_calls_msg ON tool_calls(message_id);
"#;

pub fn open(path: &Path) -> AppResult<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(SCHEMA_SQL)?;
    Ok(conn)
}

pub fn open_in_memory() -> AppResult<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(SCHEMA_SQL)?;
    Ok(conn)
}

fn source_str(s: Source) -> &'static str {
    match s {
        Source::Openai => "openai",
        Source::ClaudeWeb => "claude_web",
        Source::ClaudeCode => "claude_code",
    }
}

fn role_str(r: Role) -> &'static str {
    match r {
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::System => "system",
        Role::Tool => "tool",
    }
}

pub fn upsert_conversation(conn: &Connection, c: &Conversation) -> AppResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO conversations
         (id, source, native_id, title, created_at, updated_at, model, project,
          message_count, tok_input, tok_output, tok_cache_read, tok_cache_write, estimated_cost_usd)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        params![
            c.id,
            source_str(c.source),
            c.native_id, c.title, c.created_at, c.updated_at, c.model, c.project,
            c.message_count, c.tokens.input, c.tokens.output,
            c.tokens.cache_read, c.tokens.cache_write, c.estimated_cost_usd
        ],
    )?;
    Ok(())
}

pub fn insert_message(conn: &Connection, seq: u32, m: &Message) -> AppResult<()> {
    let (i, o, cr, cw) = match &m.tokens {
        Some(t) => (
            Some(t.input as i64), Some(t.output as i64),
            Some(t.cache_read as i64), Some(t.cache_write as i64),
        ),
        None => (None, None, None, None),
    };
    conn.execute(
        "INSERT OR REPLACE INTO messages
         (id, conversation_id, seq, role, content, timestamp, model,
          tok_input, tok_output, tok_cache_read, tok_cache_write, tool_name)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            m.id, m.conversation_id, seq, role_str(m.role), m.content,
            m.timestamp, m.model, i, o, cr, cw, m.tool_name
        ],
    )?;
    Ok(())
}

pub fn count_conversations(conn: &Connection) -> AppResult<i64> {
    Ok(conn.query_row("SELECT COUNT(*) FROM conversations", [], |r| r.get::<_, i64>(0))?)
}

pub fn insert_tool_call(
    conn: &Connection,
    id: &str,
    message_id: &str,
    conversation_id: &str,
    tool_name: &str,
    seq: u32,
) -> AppResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO tool_calls (id, message_id, conversation_id, tool_name, seq)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, message_id, conversation_id, tool_name, seq],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_conv() -> Conversation {
        Conversation {
            id: "conv-1".into(), source: Source::Openai, native_id: "x".into(),
            title: "Hello".into(), created_at: 1700000000, updated_at: 1700000100,
            model: Some("gpt-4o".into()), project: None, message_count: 2,
            tokens: TokenCounts::zero(), estimated_cost_usd: 0.0,
        }
    }

    #[test]
    fn open_in_memory_creates_schema() {
        let conn = open_in_memory().unwrap();
        let count = count_conversations(&conn).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn upsert_then_count() {
        let conn = open_in_memory().unwrap();
        upsert_conversation(&conn, &sample_conv()).unwrap();
        upsert_conversation(&conn, &sample_conv()).unwrap();
        assert_eq!(count_conversations(&conn).unwrap(), 1);
    }

    #[test]
    fn insert_message_persists() {
        let conn = open_in_memory().unwrap();
        upsert_conversation(&conn, &sample_conv()).unwrap();
        let m = Message {
            id: "m-1".into(), conversation_id: "conv-1".into(), role: Role::User,
            content: "hi".into(), timestamp: Some(1700000050), model: None,
            tokens: None, tool_name: None,
        };
        insert_message(&conn, 0, &m).unwrap();
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn tool_calls_table_exists() {
        let conn = open_in_memory().unwrap();
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM tool_calls", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn insert_tool_call_persists() {
        let conn = open_in_memory().unwrap();
        upsert_conversation(&conn, &sample_conv()).unwrap();
        let m = Message {
            id: "m-1".into(),
            conversation_id: "conv-1".into(),
            role: Role::Assistant,
            content: "x".into(),
            timestamp: Some(0),
            model: None,
            tokens: None,
            tool_name: Some("Bash".into()),
        };
        insert_message(&conn, 0, &m).unwrap();
        insert_tool_call(&conn, "m-1:tu1", "m-1", "conv-1", "Bash", 0).unwrap();
        insert_tool_call(&conn, "m-1:tu2", "m-1", "conv-1", "Read", 1).unwrap();
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM tool_calls", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 2);
        let tools: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT tool_name FROM tool_calls ORDER BY seq")
                .unwrap();
            let rows = stmt.query_map([], |r| r.get::<_, String>(0)).unwrap();
            rows.filter_map(Result::ok).collect()
        };
        assert_eq!(tools, vec!["Bash".to_string(), "Read".to_string()]);
    }
}
