use crate::error::AppResult;
use crate::schema::*;
use chrono::DateTime;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct RawConv {
    uuid: String,
    name: Option<String>,
    summary: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    #[serde(default)]
    chat_messages: Vec<RawMessage>,
}

#[derive(Debug, Deserialize)]
struct RawMessage {
    uuid: String,
    sender: String,
    text: Option<String>,
    #[serde(default)]
    content: Vec<RawBlock>,
    created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawBlock {
    #[serde(rename = "type")]
    block_type: Option<String>,
    text: Option<String>,
    // input: Option<Value>, // for tool_use; we ignore for now
    // tool_use_id, name, output ... ditto
}

pub fn parse_conversations_json(s: &str) -> AppResult<Vec<(Conversation, Vec<Message>)>> {
    let v: Value = serde_json::from_str(s)?;
    parse_value(&v)
}

pub fn parse_value(v: &Value) -> AppResult<Vec<(Conversation, Vec<Message>)>> {
    let raws: Vec<RawConv> = serde_json::from_value(v.clone())?;
    let mut out = Vec::with_capacity(raws.len());
    for r in raws {
        if let Some(item) = build_one(r) {
            out.push(item);
        }
    }
    Ok(out)
}

fn build_one(r: RawConv) -> Option<(Conversation, Vec<Message>)> {
    if r.chat_messages.is_empty() {
        return None;
    }
    let conv_id = format!("claude_web:{}", r.uuid);
    let mut messages: Vec<Message> = Vec::with_capacity(r.chat_messages.len());

    for m in &r.chat_messages {
        let role = match m.sender.as_str() {
            "human" | "user" => Role::User,
            "assistant" | "claude" => Role::Assistant,
            _ => continue,
        };
        let content = combine_text(m);
        if content.is_empty() { continue; }
        messages.push(Message {
            id: format!("{conv_id}:{}", m.uuid),
            conversation_id: conv_id.clone(),
            role,
            content,
            timestamp: m.created_at.as_deref().and_then(parse_iso),
            model: None,
            tokens: None,
            tool_name: None,
        });
    }
    if messages.is_empty() { return None; }

    let created_at = r.created_at.as_deref().and_then(parse_iso).unwrap_or(0);
    let updated_at = r.updated_at.as_deref().and_then(parse_iso).unwrap_or(created_at);
    let title = pick_title(&r, &messages);

    let conv = Conversation {
        id: conv_id,
        source: Source::ClaudeWeb,
        native_id: r.uuid,
        title,
        created_at,
        updated_at,
        model: None, // Claude.ai web export doesn't record model per message
        project: None,
        message_count: messages.len() as u32,
        tokens: TokenCounts::zero(),
        estimated_cost_usd: 0.0,
    };
    Some((conv, messages))
}

fn combine_text(m: &RawMessage) -> String {
    if let Some(t) = &m.text {
        if !t.trim().is_empty() {
            return t.clone();
        }
    }
    let parts: Vec<String> = m.content.iter()
        .filter(|b| b.block_type.as_deref().map_or(true, |t| t == "text"))
        .filter_map(|b| b.text.clone())
        .filter(|s| !s.trim().is_empty())
        .collect();
    parts.join("\n\n")
}

fn parse_iso(s: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.timestamp())
}

fn pick_title(r: &RawConv, msgs: &[Message]) -> String {
    if let Some(n) = &r.name { if !n.trim().is_empty() { return n.clone(); } }
    if let Some(s) = &r.summary { if !s.trim().is_empty() { return s.clone(); } }
    if let Some(first_user) = msgs.iter().find(|m| m.role == Role::User) {
        let snippet: String = first_user.content.chars().take(50).collect();
        if !snippet.is_empty() {
            return snippet;
        }
    }
    "(untitled)".to_string()
}
