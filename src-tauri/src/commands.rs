use crate::db;
use crate::error::AppResult;
use crate::parser::{parse_auto, DetectedFormat};
use crate::schema::*;
use crate::stats::{
    activity::{self, ActivityReport},
    ngrams::{top_phrases_for_user, PhraseStat},
    topics::top_topics,
};
use rusqlite::Connection;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct AppState {
    pub db: Mutex<Connection>,
}

#[derive(Serialize)]
pub struct ImportSummary {
    pub conversations_added: u32,
    pub messages_added: u32,
    pub source: String,
}

#[tauri::command]
pub fn import_export(folder: String, state: tauri::State<AppState>) -> Result<ImportSummary, String> {
    do_import(folder, &state).map_err(|e| e.to_string())
}

fn do_import(folder: String, state: &tauri::State<AppState>) -> AppResult<ImportSummary> {
    let folder = PathBuf::from(&folder);
    let path = if folder.is_file() {
        folder.clone()
    } else {
        find_conversations_json(&folder)?
    };
    let json = std::fs::read_to_string(&path)?;
    let (fmt, parsed) = parse_auto(&json)?;
    let mut conn = state.db.lock().unwrap();
    let tx = conn.transaction()?;
    let mut convs = 0u32;
    let mut msgs = 0u32;
    for (c, ms) in parsed {
        db::upsert_conversation(&tx, &c)?;
        for (i, m) in ms.iter().enumerate() {
            db::insert_message(&tx, i as u32, m)?;
            msgs += 1;
        }
        convs += 1;
    }
    tx.commit()?;
    let source = match fmt {
        DetectedFormat::Openai => "openai",
        DetectedFormat::ClaudeWeb => "claude_web",
    }
    .to_string();
    Ok(ImportSummary { conversations_added: convs, messages_added: msgs, source })
}

/// Walk the chosen folder (and one level of subfolders) looking for conversations.json.
/// This handles ChatGPT/Claude exports where the folder the user picks contains
/// either the file directly, or a single `data-XXX-batch-XXXX/` subfolder.
fn find_conversations_json(folder: &std::path::Path) -> AppResult<PathBuf> {
    let direct = folder.join("conversations.json");
    if direct.is_file() {
        return Ok(direct);
    }
    if let Ok(entries) = std::fs::read_dir(folder) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let candidate = p.join("conversations.json");
                if candidate.is_file() {
                    return Ok(candidate);
                }
            }
        }
    }
    Err(crate::error::AppError::Parse(format!(
        "no conversations.json found in {}",
        folder.display()
    )))
}

#[derive(Serialize)]
pub struct ConversationSummary {
    pub id: String,
    pub source: String,
    pub title: String,
    pub created_at: i64,
    pub message_count: u32,
    pub model: Option<String>,
    pub estimated_cost_usd: f64,
    pub tokens_total: u64,
}

#[tauri::command]
pub fn list_conversations(state: tauri::State<AppState>) -> Result<Vec<ConversationSummary>, String> {
    do_list(&state).map_err(|e| e.to_string())
}

fn do_list(state: &tauri::State<AppState>) -> AppResult<Vec<ConversationSummary>> {
    let conn = state.db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, source, title, created_at, message_count, model,
                estimated_cost_usd, tok_input + tok_output + tok_cache_read + tok_cache_write
         FROM conversations ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(ConversationSummary {
            id: r.get(0)?,
            source: r.get(1)?,
            title: r.get(2)?,
            created_at: r.get(3)?,
            message_count: r.get(4)?,
            model: r.get(5)?,
            estimated_cost_usd: r.get(6)?,
            tokens_total: r.get::<_, i64>(7)? as u64,
        })
    })?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
pub fn get_conversation(id: String, state: tauri::State<AppState>) -> Result<Vec<Message>, String> {
    do_get_conv(id, &state).map_err(|e| e.to_string())
}

fn do_get_conv(id: String, state: &tauri::State<AppState>) -> AppResult<Vec<Message>> {
    let conn = state.db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, conversation_id, role, content, timestamp, model,
                tok_input, tok_output, tok_cache_read, tok_cache_write, tool_name
         FROM messages WHERE conversation_id = ?1 ORDER BY seq ASC",
    )?;
    let rows = stmt.query_map([&id], |r| {
        let role: String = r.get(2)?;
        let role = match role.as_str() {
            "user" => Role::User,
            "assistant" => Role::Assistant,
            "system" => Role::System,
            _ => Role::Tool,
        };
        let toks = match (
            r.get::<_, Option<i64>>(6)?,
            r.get::<_, Option<i64>>(7)?,
            r.get::<_, Option<i64>>(8)?,
            r.get::<_, Option<i64>>(9)?,
        ) {
            (Some(i), Some(o), Some(cr), Some(cw)) => Some(TokenCounts {
                input: i as u64,
                output: o as u64,
                cache_read: cr as u64,
                cache_write: cw as u64,
            }),
            _ => None,
        };
        Ok(Message {
            id: r.get(0)?,
            conversation_id: r.get(1)?,
            role,
            content: r.get(3)?,
            timestamp: r.get(4)?,
            model: r.get(5)?,
            tokens: toks,
            tool_name: r.get(10)?,
        })
    })?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[derive(Serialize)]
pub struct StatsReport {
    pub total_conversations: i64,
    pub total_messages: i64,
    pub total_tokens: TokenCounts,
    pub estimated_cost_usd: f64,
    pub first_at: Option<i64>,
    pub last_at: Option<i64>,
    pub activity: ActivityReport,
    pub top_phrases: Vec<PhraseStat>,
    pub top_topics: Vec<PhraseStat>,
    pub by_model: Vec<(String, i64)>,
    pub by_source: Vec<(String, i64)>,
}

#[tauri::command]
pub fn get_stats(state: tauri::State<AppState>) -> Result<StatsReport, String> {
    do_stats(&state).map_err(|e| e.to_string())
}

fn do_stats(state: &tauri::State<AppState>) -> AppResult<StatsReport> {
    let conn = state.db.lock().unwrap();
    let total_conversations: i64 =
        conn.query_row("SELECT COUNT(*) FROM conversations", [], |r| r.get(0))?;
    let total_messages: i64 =
        conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))?;
    let (i, o, cr, cw, cost): (i64, i64, i64, i64, f64) = conn.query_row(
        "SELECT COALESCE(SUM(tok_input),0), COALESCE(SUM(tok_output),0),
                COALESCE(SUM(tok_cache_read),0), COALESCE(SUM(tok_cache_write),0),
                COALESCE(SUM(estimated_cost_usd),0) FROM conversations",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
    )?;
    let first_at: Option<i64> = conn
        .query_row("SELECT MIN(created_at) FROM conversations", [], |r| r.get(0))
        .ok();
    let last_at: Option<i64> = conn
        .query_row("SELECT MAX(created_at) FROM conversations", [], |r| r.get(0))
        .ok();

    let activity = activity::compute(&conn)?;
    let top_phrases = top_phrases_for_user(&conn, 10)?;
    let top_topics = top_topics(&conn, 10)?;

    let mut by_model: Vec<(String, i64)> = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT COALESCE(model,'(unknown)'), COUNT(*) FROM conversations
             GROUP BY model ORDER BY 2 DESC",
        )?;
        let rows =
            stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        for row in rows {
            by_model.push(row?);
        }
    }

    let mut by_source: Vec<(String, i64)> = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT source, COUNT(*) FROM conversations GROUP BY source ORDER BY 2 DESC",
        )?;
        let rows =
            stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        for row in rows {
            by_source.push(row?);
        }
    }

    Ok(StatsReport {
        total_conversations,
        total_messages,
        total_tokens: TokenCounts {
            input: i as u64,
            output: o as u64,
            cache_read: cr as u64,
            cache_write: cw as u64,
        },
        estimated_cost_usd: cost,
        first_at,
        last_at,
        activity,
        top_phrases,
        top_topics,
        by_model,
        by_source,
    })
}

#[tauri::command]
pub fn clear_data(state: tauri::State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().unwrap();
    conn.execute_batch("DELETE FROM messages; DELETE FROM conversations;")
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_claude_code(
    folder: Option<String>,
    state: tauri::State<AppState>,
) -> Result<ImportSummary, String> {
    do_import_cc(folder, &state).map_err(|e| e.to_string())
}

fn do_import_cc(
    folder: Option<String>,
    state: &tauri::State<AppState>,
) -> AppResult<ImportSummary> {
    let dir = match folder {
        Some(s) => PathBuf::from(s),
        None => crate::parser::claude_code::default_projects_dir().ok_or_else(|| {
            crate::error::AppError::Parse("could not locate ~/.claude/projects".into())
        })?,
    };
    if !dir.is_dir() {
        return Err(crate::error::AppError::Parse(format!(
            "not a directory: {}",
            dir.display()
        )));
    }
    let parsed = crate::parser::claude_code::parse_projects_dir(&dir)?;
    let mut conn = state.db.lock().unwrap();
    let tx = conn.transaction()?;
    let mut convs = 0u32;
    let mut msgs = 0u32;
    for (c, ms) in parsed {
        db::upsert_conversation(&tx, &c)?;
        for (i, m) in ms.iter().enumerate() {
            db::insert_message(&tx, i as u32, m)?;
            msgs += 1;
        }
        convs += 1;
    }
    tx.commit()?;
    Ok(ImportSummary {
        conversations_added: convs,
        messages_added: msgs,
        source: "claude_code".into(),
    })
}
