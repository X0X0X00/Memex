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
    let paths = if folder.is_file() {
        vec![folder.clone()]
    } else {
        find_conversations_files(&folder)?
    };
    if paths.is_empty() {
        return Err(crate::error::AppError::Parse(format!(
            "no conversations.json (or conversations-NNN.json) found in {}",
            folder.display()
        )));
    }

    // Parse every chunk; merge into one parse result. All chunks must share the
    // same detected format (catches a mistakenly mixed folder).
    let mut detected_fmt: Option<DetectedFormat> = None;
    let mut all_parsed: Vec<_> = Vec::new();
    for p in &paths {
        let json = std::fs::read_to_string(p)?;
        let (fmt, parsed) = parse_auto(&json)?;
        match detected_fmt {
            None => detected_fmt = Some(fmt),
            Some(prev) if prev == fmt => {}
            Some(prev) => {
                return Err(crate::error::AppError::Parse(format!(
                    "mixed export formats in folder ({:?} vs {:?} at {})",
                    prev,
                    fmt,
                    p.display()
                )));
            }
        }
        all_parsed.extend(parsed);
    }
    let fmt = detected_fmt.expect("paths non-empty");

    let mut conn = state.db.lock().unwrap();
    let tx = conn.transaction()?;
    let mut convs = 0u32;
    let mut msgs = 0u32;
    for (c, ms) in all_parsed {
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

/// Walk the chosen folder (and one level of subfolders) looking for the export's
/// JSON file(s). Handles three layouts:
///
/// 1. Folder contains `conversations.json` directly (older ChatGPT / Claude.ai).
/// 2. Folder contains a single `data-XXX-batch-XXXX/` subfolder with `conversations.json`.
/// 3. Folder contains chunked `conversations-000.json` … `conversations-NNN.json`
///    (newer ChatGPT export for large accounts), or that pattern inside a single
///    subfolder.
fn find_conversations_files(folder: &std::path::Path) -> AppResult<Vec<PathBuf>> {
    if let Some(found) = scan_one_dir(folder)? {
        return Ok(found);
    }
    if let Ok(entries) = std::fs::read_dir(folder) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                if let Some(found) = scan_one_dir(&p)? {
                    return Ok(found);
                }
            }
        }
    }
    Ok(Vec::new())
}

/// Look in a single directory for either `conversations.json` (preferred) or
/// chunked `conversations-NNN.json` files. Returns None if nothing matches.
fn scan_one_dir(dir: &std::path::Path) -> AppResult<Option<Vec<PathBuf>>> {
    if !dir.is_dir() {
        return Ok(None);
    }
    let direct = dir.join("conversations.json");
    if direct.is_file() {
        return Ok(Some(vec![direct]));
    }
    let mut chunks: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(dir)?.flatten() {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
            if is_conversations_chunk(name) {
                chunks.push(p);
            }
        }
    }
    if chunks.is_empty() {
        Ok(None)
    } else {
        chunks.sort();
        Ok(Some(chunks))
    }
}

/// Match `conversations-<digits>.json` (chunked-export filename).
fn is_conversations_chunk(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".json") else { return false };
    let Some(rest) = stem.strip_prefix("conversations-") else { return false };
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod test_chunked {
    use super::is_conversations_chunk;

    #[test]
    fn matches_chunked_filenames() {
        assert!(is_conversations_chunk("conversations-000.json"));
        assert!(is_conversations_chunk("conversations-031.json"));
        assert!(is_conversations_chunk("conversations-9.json"));
    }

    #[test]
    fn rejects_non_matching_filenames() {
        assert!(!is_conversations_chunk("conversations.json"));
        assert!(!is_conversations_chunk("conversations-.json"));
        assert!(!is_conversations_chunk("conversations-abc.json"));
        assert!(!is_conversations_chunk("conv-000.json"));
        assert!(!is_conversations_chunk("conversations-000.txt"));
    }
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
    pub cost_over_time: crate::stats::cost_over_time::CostSeries,
    pub by_project: Vec<crate::stats::projects::ProjectRow>,
    pub tool_usage: Vec<(String, i64)>,
    pub message_length: Vec<crate::stats::length_dist::LengthBucket>,
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

    let cost_over_time = crate::stats::cost_over_time::compute(&conn)?;
    let by_project = crate::stats::projects::compute(&conn)?;
    let tool_usage = crate::stats::tool_usage::compute(&conn, 20)?;
    let message_length = crate::stats::length_dist::compute(&conn)?;

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
        cost_over_time,
        by_project,
        tool_usage,
        message_length,
    })
}

#[tauri::command]
pub fn clear_data(state: tauri::State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().unwrap();
    conn.execute_batch("DELETE FROM messages; DELETE FROM conversations;")
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_png(path: String, bytes: Vec<u8>) -> Result<(), String> {
    std::fs::write(&path, &bytes).map_err(|e| format!("write failed: {e}"))
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
    for (c, ms, tcs) in parsed {
        db::upsert_conversation(&tx, &c)?;
        for (i, m) in ms.iter().enumerate() {
            db::insert_message(&tx, i as u32, m)?;
            msgs += 1;
        }
        for tc in tcs {
            db::insert_tool_call(
                &tx,
                &tc.id,
                &tc.message_id,
                &tc.conversation_id,
                &tc.tool_name,
                tc.seq,
            )?;
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
