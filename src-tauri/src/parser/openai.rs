use crate::error::{AppError, AppResult};
use crate::schema::*;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct RawConv {
    id: String,
    title: Option<String>,
    create_time: Option<f64>,
    update_time: Option<f64>,
    default_model_slug: Option<String>,
    current_node: Option<String>,
    mapping: HashMap<String, RawNode>,
}

#[derive(Debug, Deserialize)]
struct RawNode {
    #[allow(dead_code)]
    id: Option<String>,
    message: Option<RawMessage>,
    parent: Option<String>,
    #[serde(default)]
    children: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawMessage {
    id: String,
    author: RawAuthor,
    create_time: Option<f64>,
    content: Option<RawContent>,
    #[serde(default)]
    metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct RawAuthor { role: String }

#[derive(Debug, Deserialize)]
struct RawContent {
    content_type: Option<String>,
    #[serde(default)]
    parts: Vec<serde_json::Value>,
}

pub fn parse_conversations_json(s: &str) -> AppResult<Vec<(Conversation, Vec<Message>)>> {
    let v: Value = serde_json::from_str(s)?;
    parse_value(&v)
}

pub fn parse_value(v: &Value) -> AppResult<Vec<(Conversation, Vec<Message>)>> {
    let raws: Vec<RawConv> = serde_json::from_value(v.clone())?;
    let mut out = Vec::with_capacity(raws.len());
    for r in raws {
        if let Some(item) = build_one(r)? {
            out.push(item);
        }
    }
    Ok(out)
}

fn build_one(r: RawConv) -> AppResult<Option<(Conversation, Vec<Message>)>> {
    let leaf_id = match r.current_node.as_ref() {
        Some(id) => id.clone(),
        None => match pick_deepest_leaf(&r.mapping) {
            Some(id) => id,
            None => return Ok(None),
        },
    };

    let mut path: Vec<&RawNode> = Vec::new();
    let mut cur = r.mapping.get(&leaf_id);
    while let Some(node) = cur {
        path.push(node);
        cur = node.parent.as_ref().and_then(|pid| r.mapping.get(pid));
    }
    path.reverse();

    let conv_id = format!("openai:{}", r.id);
    let mut messages: Vec<Message> = Vec::new();
    for node in path {
        let msg = match &node.message { Some(m) => m, None => continue };
        let role = match parse_role(&msg.author.role) { Some(r) => r, None => continue };
        let content = extract_text(&msg.content);
        if content.is_empty() { continue; }
        let model = msg.metadata.get("model_slug")
            .and_then(|v| v.as_str())
            .map(String::from);
        messages.push(Message {
            id: format!("{conv_id}:{}", msg.id),
            conversation_id: conv_id.clone(),
            role,
            content,
            timestamp: msg.create_time.map(|f| f as i64),
            model,
            tokens: None,
            tool_name: None,
        });
    }

    if messages.is_empty() { return Ok(None); }

    let title = r.title.unwrap_or_else(|| "(untitled)".to_string());
    let created = r.create_time.map(|f| f as i64).unwrap_or(0);
    let updated = r.update_time.map(|f| f as i64).unwrap_or(created);
    let model = messages.iter().rev().find_map(|m| m.model.clone()).or(r.default_model_slug);

    let conv = Conversation {
        id: conv_id,
        source: Source::Openai,
        native_id: r.id,
        title,
        created_at: created,
        updated_at: updated,
        model,
        project: None,
        message_count: messages.len() as u32,
        tokens: TokenCounts::zero(),
        estimated_cost_usd: 0.0,
    };
    Ok(Some((conv, messages)))
}

fn parse_role(s: &str) -> Option<Role> {
    match s {
        "user" => Some(Role::User),
        "assistant" => Some(Role::Assistant),
        "system" => Some(Role::System),
        "tool" => Some(Role::Tool),
        _ => None,
    }
}

fn extract_text(c: &Option<RawContent>) -> String {
    let Some(c) = c else { return String::new() };
    if c.content_type.as_deref() != Some("text") { return String::new(); }
    c.parts.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join("\n")
}

fn pick_deepest_leaf(map: &HashMap<String, RawNode>) -> Option<String> {
    let mut best: Option<(String, u32)> = None;
    for (k, n) in map.iter() {
        if !n.children.is_empty() { continue; }
        let d = depth(map, k, 0);
        if best.as_ref().map_or(true, |(_, bd)| d > *bd) {
            best = Some((k.clone(), d));
        }
    }
    best.map(|(k, _)| k)
}

fn depth(map: &HashMap<String, RawNode>, id: &str, d: u32) -> u32 {
    match map.get(id).and_then(|n| n.parent.as_ref()) {
        Some(p) => depth(map, p, d + 1),
        None => d,
    }
}

#[allow(dead_code)]
fn _unused(_: AppError) {}
