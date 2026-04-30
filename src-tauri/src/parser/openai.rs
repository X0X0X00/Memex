use crate::error::AppResult;
use crate::parser::tokens::count_tokens;
use crate::schema::*;
use crate::stats::cost::cumulative_billing_cost;
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
    /// Catch-all for content types that don't use `parts`: `code`,
    /// `execution_output`, `tether_browsing_display`, `tether_quote`,
    /// `system_error`, `model_editable_context`, etc.
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

/// Per-image vision-token estimate. Real OpenAI vision detail=high is
/// 85 + 170*tiles (typically 500–1500). 1000 is a reasonable midpoint.
const IMAGE_TOKENS: u64 = 1000;

/// Synthetic context overhead added to every conversation to account for
/// ChatGPT's invisible system prompt + memory + custom instructions, which
/// the export never includes but the API would re-bill on every turn.
/// Empirically 1500–3000 tokens for ChatGPT.
const SYSTEM_PROMPT_TOKENS: u64 = 2000;

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
        let (content, extra_tokens) = extract_message_content(&msg.content);
        if content.is_empty() && extra_tokens == 0 { continue; }
        let model = msg.metadata.get("model_slug")
            .and_then(|v| v.as_str())
            .map(String::from);
        let tk = count_tokens(&content) + extra_tokens;
        let tokens = match role {
            Role::Assistant => TokenCounts { input: 0, output: tk, cache_read: 0, cache_write: 0 },
            _ => TokenCounts { input: tk, output: 0, cache_read: 0, cache_write: 0 },
        };
        messages.push(Message {
            id: format!("{conv_id}:{}", msg.id),
            conversation_id: conv_id.clone(),
            role,
            content,
            timestamp: msg.create_time.map(|f| f as i64),
            model,
            tokens: Some(tokens),
            tool_name: None,
        });
    }

    if messages.is_empty() { return Ok(None); }

    let title = r.title.unwrap_or_else(|| "(untitled)".to_string());
    let created = r.create_time.map(|f| f as i64).unwrap_or(0);
    let updated = r.update_time.map(|f| f as i64).unwrap_or(created);
    let model = messages.iter().rev().find_map(|m| m.model.clone()).or(r.default_model_slug);

    let mut totals = TokenCounts::zero();
    for m in &messages {
        if let Some(t) = &m.tokens {
            totals.input += t.input;
            totals.output += t.output;
            totals.cache_read += t.cache_read;
            totals.cache_write += t.cache_write;
        }
    }
    // ChatGPT's hidden system prompt + memory + custom instructions are
    // never in the export but the API re-sends them on every turn. Inject
    // a synthetic system message at the front so cumulative_billing_cost
    // accounts for that overhead. We don't persist this — only used for
    // cost.
    let mut messages_for_cost = Vec::with_capacity(messages.len() + 1);
    messages_for_cost.push(Message {
        id: format!("{conv_id}:__system_prompt__"),
        conversation_id: conv_id.clone(),
        role: Role::System,
        content: String::new(),
        timestamp: messages.first().and_then(|m| m.timestamp),
        model: None,
        tokens: Some(TokenCounts {
            input: SYSTEM_PROMPT_TOKENS,
            output: 0,
            cache_read: 0,
            cache_write: 0,
        }),
        tool_name: None,
    });
    messages_for_cost.extend(messages.iter().cloned());

    // Cost uses cumulative billing (each assistant turn billed against the
    // running input context, not just its own message tokens). Falls back
    // to "gpt-4o" pricing for messages with no recorded model.
    let fallback = model.as_deref().unwrap_or("gpt-4o");
    let estimated_cost_usd = cumulative_billing_cost(&messages_for_cost, fallback);

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
        tokens: totals,
        estimated_cost_usd,
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

/// Returns (text, extra_tokens). The text contains everything we can
/// tokenize via tiktoken; extra_tokens covers images and other non-text
/// payloads (vision attachments, audio) that wouldn't be tokenized by
/// tiktoken on text alone.
///
/// Pre-v0.3.5 we only handled `content_type == "text"` and silently
/// dropped everything else: code interpreter input/output, web browsing
/// results, image attachments, persistent memory, system errors. That
/// caused tokens (and therefore cost) to be massively under-counted for
/// users who used ChatGPT for vision, code, or web browsing.
fn extract_message_content(c: &Option<RawContent>) -> (String, u64) {
    let Some(c) = c else { return (String::new(), 0) };
    let ct = c.content_type.as_deref().unwrap_or("");
    let mut texts: Vec<String> = Vec::new();
    let mut extra: u64 = 0;

    match ct {
        // Plain text — `parts` is array of strings.
        "text" => {
            for p in &c.parts {
                if let Some(s) = p.as_str() {
                    if !s.is_empty() {
                        texts.push(s.to_string());
                    }
                }
            }
        }
        // Mixed text + image / file attachments. Strings → text; objects → image.
        "multimodal_text" => {
            for p in &c.parts {
                if let Some(s) = p.as_str() {
                    if !s.is_empty() {
                        texts.push(s.to_string());
                    }
                } else if p.is_object() {
                    extra += IMAGE_TOKENS;
                }
            }
        }
        // Code interpreter input: { content_type: "code", text, language }.
        "code" => {
            if let Some(s) = c.extra.get("text").and_then(|v| v.as_str()) {
                texts.push(s.to_string());
            }
        }
        // Code interpreter output: { content_type: "execution_output", text }.
        "execution_output" => {
            if let Some(s) = c.extra.get("text").and_then(|v| v.as_str()) {
                texts.push(s.to_string());
            }
        }
        // Web browsing display — long! { result, title, url, ... }.
        "tether_browsing_display" => {
            if let Some(s) = c.extra.get("result").and_then(|v| v.as_str()) {
                texts.push(s.to_string());
            }
        }
        // Citation / quote: { text, title, url }.
        "tether_quote" => {
            if let Some(s) = c.extra.get("text").and_then(|v| v.as_str()) {
                texts.push(s.to_string());
            }
        }
        "system_error" => {
            if let Some(s) = c.extra.get("text").and_then(|v| v.as_str()) {
                texts.push(s.to_string());
            }
        }
        // Persistent memory / custom instructions — billed every turn.
        // { content_type: "model_editable_context", model_set_context: "..." }
        "model_editable_context" => {
            if let Some(s) = c.extra.get("model_set_context").and_then(|v| v.as_str()) {
                texts.push(s.to_string());
            }
            if let Some(s) = c.extra
                .get("repository")
                .and_then(|v| v.as_str())
            {
                texts.push(s.to_string());
            }
        }
        // Unknown content_type — best-effort: collect any string-valued field.
        _ => {
            for v in c.extra.values() {
                if let Some(s) = v.as_str() {
                    if !s.is_empty() {
                        texts.push(s.to_string());
                    }
                }
            }
            for p in &c.parts {
                if let Some(s) = p.as_str() {
                    if !s.is_empty() {
                        texts.push(s.to_string());
                    }
                } else if p.is_object() {
                    extra += IMAGE_TOKENS;
                }
            }
        }
    }
    (texts.join("\n"), extra)
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

