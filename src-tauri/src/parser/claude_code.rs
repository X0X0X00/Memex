//! Parses Claude Code session files (`~/.claude/projects/<encoded-cwd>/<uuid>.jsonl`).
//!
//! Each line is one JSON event. The shape varies by `type`:
//! - `user` / `assistant`: real messages (the only ones we surface).
//! - `custom-title`: optional title for the session.
//! - everything else (`permission-mode`, `attachment`, `system`, `queue-operation`,
//!   `last-prompt`, `file-history-snapshot`, ...) is ignored.
//!
//! Unlike the web exports, Claude Code records exact token counts and the
//! per-message model in the assistant event's `message.usage` and `message.model`.
//! We use those directly — no tiktoken estimation.

use crate::error::{AppError, AppResult};
use crate::schema::*;
use crate::stats::cost::estimate_cost_usd;
use chrono::DateTime;
use std::path::{Path, PathBuf};

/// Parse one .jsonl session file. Returns `None` if no usable user/assistant
/// turns were found (e.g. an empty session, or a file that's all metadata).
pub fn parse_session_file(path: impl AsRef<Path>) -> AppResult<Option<(Conversation, Vec<Message>)>> {
    let raw = std::fs::read_to_string(&path)?;
    parse_session_str(&raw)
}

pub fn parse_session_str(raw: &str) -> AppResult<Option<(Conversation, Vec<Message>)>> {
    let mut custom_title: Option<String> = None;
    let mut session_id: Option<String> = None;
    let mut cwd: Option<String> = None;
    let mut last_model: Option<String> = None;
    let mut first_ts: Option<i64> = None;
    let mut last_ts: Option<i64> = None;
    let mut messages: Vec<Message> = Vec::new();
    let mut totals = TokenCounts::zero();

    for (i, line) in raw.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(line)
            .map_err(|e| AppError::Parse(format!("line {}: {}", i + 1, e)))?;

        let ty = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
        match ty {
            "custom-title" => {
                if custom_title.is_none() {
                    custom_title = v
                        .get("customTitle")
                        .and_then(|s| s.as_str())
                        .map(String::from);
                }
                if session_id.is_none() {
                    session_id = v.get("sessionId").and_then(|s| s.as_str()).map(String::from);
                }
            }
            "user" => {
                // Skip cc-injected meta messages (caveats / slash command echoes).
                if v.get("isMeta").and_then(|x| x.as_bool()) == Some(true) {
                    continue;
                }
                if let Some(m) = build_user_message(&v) {
                    if first_ts.is_none() {
                        first_ts = m.timestamp;
                    }
                    if let Some(t) = m.timestamp {
                        last_ts = Some(t);
                    }
                    if session_id.is_none() {
                        session_id = v.get("sessionId").and_then(|s| s.as_str()).map(String::from);
                    }
                    if cwd.is_none() {
                        cwd = v.get("cwd").and_then(|s| s.as_str()).map(String::from);
                    }
                    messages.push(m);
                }
            }
            "assistant" => {
                if let Some((m, usage)) = build_assistant_message(&v) {
                    if first_ts.is_none() {
                        first_ts = m.timestamp;
                    }
                    if let Some(t) = m.timestamp {
                        last_ts = Some(t);
                    }
                    if session_id.is_none() {
                        session_id = v.get("sessionId").and_then(|s| s.as_str()).map(String::from);
                    }
                    if cwd.is_none() {
                        cwd = v.get("cwd").and_then(|s| s.as_str()).map(String::from);
                    }
                    if let Some(model) = v.pointer("/message/model").and_then(|m| m.as_str()) {
                        last_model = Some(model.to_string());
                    }
                    totals.input += usage.input;
                    totals.output += usage.output;
                    totals.cache_read += usage.cache_read;
                    totals.cache_write += usage.cache_write;
                    messages.push(m);
                }
            }
            _ => {}
        }
    }

    if messages.is_empty() {
        return Ok(None);
    }
    let sid = match session_id {
        Some(s) => s,
        None => return Ok(None),
    };
    let conv_id = format!("claude_code:{sid}");

    // Compute per-message cost using each message's own model (assistant) or the
    // session-level last_model as fallback (for user messages — though their
    // tokens are all `input` and the bill goes against whichever model processed
    // them, so this approximation is OK).
    let mut estimated_cost_usd = 0.0;
    for m in &mut messages {
        m.conversation_id = conv_id.clone();
        m.id = format!("{conv_id}:{}", m.id);
        if let Some(t) = &m.tokens {
            let model = m.model.as_deref().or(last_model.as_deref()).unwrap_or("claude");
            estimated_cost_usd += estimate_cost_usd(model, t);
        }
    }

    let title = custom_title
        .or_else(|| {
            messages
                .iter()
                .find(|m| m.role == Role::User && !is_command_echo(&m.content))
                .map(|m| {
                    let snippet: String = m.content.chars().take(60).collect();
                    snippet
                })
        })
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "(untitled)".into());

    Ok(Some((
        Conversation {
            id: conv_id,
            source: Source::ClaudeCode,
            native_id: sid,
            title,
            created_at: first_ts.unwrap_or(0),
            updated_at: last_ts.unwrap_or_else(|| first_ts.unwrap_or(0)),
            model: last_model,
            project: cwd,
            message_count: messages.len() as u32,
            tokens: totals,
            estimated_cost_usd,
        },
        messages,
    )))
}

struct Usage {
    input: u64,
    output: u64,
    cache_read: u64,
    cache_write: u64,
}

fn build_user_message(v: &serde_json::Value) -> Option<Message> {
    let uuid = v.get("uuid").and_then(|s| s.as_str())?.to_string();
    let ts = v.get("timestamp").and_then(|s| s.as_str()).and_then(parse_iso);
    let content_v = v.pointer("/message/content")?;
    let content = extract_user_content(content_v);
    if content.is_empty() {
        return None;
    }
    Some(Message {
        id: uuid,
        conversation_id: String::new(),
        role: Role::User,
        content,
        timestamp: ts,
        model: None,
        tokens: None,
        tool_name: None,
    })
}

fn build_assistant_message(v: &serde_json::Value) -> Option<(Message, Usage)> {
    let uuid = v.get("uuid").and_then(|s| s.as_str())?.to_string();
    let ts = v.get("timestamp").and_then(|s| s.as_str()).and_then(parse_iso);
    let model = v
        .pointer("/message/model")
        .and_then(|m| m.as_str())
        .map(String::from);
    let content_v = v.pointer("/message/content")?;
    let (content, tool_name) = extract_assistant_content(content_v);
    if content.is_empty() && tool_name.is_none() {
        return None;
    }

    let usage = v.pointer("/message/usage");
    let u = Usage {
        input: usage
            .and_then(|x| x.get("input_tokens"))
            .and_then(|x| x.as_u64())
            .unwrap_or(0),
        output: usage
            .and_then(|x| x.get("output_tokens"))
            .and_then(|x| x.as_u64())
            .unwrap_or(0),
        cache_read: usage
            .and_then(|x| x.get("cache_read_input_tokens"))
            .and_then(|x| x.as_u64())
            .unwrap_or(0),
        cache_write: usage
            .and_then(|x| x.get("cache_creation_input_tokens"))
            .and_then(|x| x.as_u64())
            .unwrap_or(0),
    };
    let tokens = TokenCounts {
        input: u.input,
        output: u.output,
        cache_read: u.cache_read,
        cache_write: u.cache_write,
    };
    Some((
        Message {
            id: uuid,
            conversation_id: String::new(),
            role: Role::Assistant,
            content,
            timestamp: ts,
            model,
            tokens: Some(tokens),
            tool_name,
        },
        u,
    ))
}

fn extract_user_content(v: &serde_json::Value) -> String {
    if let Some(s) = v.as_str() {
        return s.to_string();
    }
    let arr = match v.as_array() {
        Some(a) => a,
        None => return String::new(),
    };
    let parts: Vec<String> = arr
        .iter()
        .filter_map(|b| {
            let ty = b.get("type").and_then(|t| t.as_str())?;
            match ty {
                "text" => b.get("text").and_then(|t| t.as_str()).map(String::from),
                "tool_result" => {
                    let inner = b.get("content")?;
                    Some(format!("[tool_result] {}", flatten_inner_content(inner)))
                }
                _ => None,
            }
        })
        .collect();
    parts.join("\n\n")
}

fn extract_assistant_content(v: &serde_json::Value) -> (String, Option<String>) {
    let arr = match v.as_array() {
        Some(a) => a,
        None => return (String::new(), None),
    };
    let mut texts: Vec<String> = Vec::new();
    let mut first_tool: Option<String> = None;
    for b in arr {
        let ty = b.get("type").and_then(|t| t.as_str()).unwrap_or("");
        match ty {
            "text" => {
                if let Some(t) = b.get("text").and_then(|t| t.as_str()) {
                    texts.push(t.to_string());
                }
            }
            "thinking" => {
                if let Some(t) = b.get("thinking").and_then(|t| t.as_str()) {
                    texts.push(format!("> [thinking] {}", t));
                }
            }
            "tool_use" => {
                let name = b.get("name").and_then(|s| s.as_str()).unwrap_or("tool");
                if first_tool.is_none() {
                    first_tool = Some(name.to_string());
                }
                texts.push(format!("[tool_use: {}]", name));
            }
            _ => {}
        }
    }
    (texts.join("\n\n"), first_tool)
}

fn flatten_inner_content(v: &serde_json::Value) -> String {
    if let Some(s) = v.as_str() {
        return s.to_string();
    }
    if let Some(arr) = v.as_array() {
        return arr
            .iter()
            .filter_map(|b| {
                b.get("text")
                    .and_then(|t| t.as_str())
                    .map(String::from)
            })
            .collect::<Vec<_>>()
            .join("\n");
    }
    v.to_string()
}

fn parse_iso(s: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.timestamp())
}

/// True if the message is one of cc's auto-injected wrappers
/// (slash-command echoes, system reminders, caveats). Used to pick a
/// readable title.
fn is_command_echo(s: &str) -> bool {
    let trimmed = s.trim_start();
    trimmed.starts_with("<command-name>")
        || trimmed.starts_with("<command-message>")
        || trimmed.starts_with("<command-args>")
        || trimmed.starts_with("<local-command-caveat>")
        || trimmed.starts_with("<local-command-stdout>")
        || trimmed.starts_with("<local-command-stderr>")
        || trimmed.starts_with("<system-reminder>")
        || trimmed.starts_with("[tool_result]")
}

pub fn parse_projects_dir(root: &Path) -> AppResult<Vec<(Conversation, Vec<Message>)>> {
    let mut out = Vec::new();
    walk_jsonl(root, &mut |p| {
        if let Ok(Some(item)) = parse_session_file(p) {
            out.push(item);
        }
    })?;
    Ok(out)
}

fn walk_jsonl(dir: &Path, cb: &mut dyn FnMut(&Path)) -> AppResult<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let p = entry?.path();
        if p.is_dir() {
            walk_jsonl(&p, cb)?;
        } else if p.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            cb(&p);
        }
    }
    Ok(())
}

pub fn default_projects_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|h| h.join(".claude").join("projects"))
}
