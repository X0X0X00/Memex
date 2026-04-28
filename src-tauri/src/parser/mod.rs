use crate::error::{AppError, AppResult};
use crate::schema::*;
use serde_json::Value;

pub mod openai;
pub mod claude_web;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedFormat {
    Openai,
    ClaudeWeb,
}

/// Inspect a `conversations.json` value and decide which parser to use.
/// - OpenAI ChatGPT export: top-level array; each item has a `mapping` object.
/// - Claude.ai web export: top-level array; each item has `chat_messages` array.
pub fn detect_format(v: &Value) -> AppResult<DetectedFormat> {
    let arr = v.as_array().ok_or_else(|| AppError::Parse("expected top-level JSON array".into()))?;
    let sample = arr.iter().find(|c| c.is_object()).ok_or_else(|| AppError::Parse("no objects in array".into()))?;
    if sample.get("mapping").is_some() {
        Ok(DetectedFormat::Openai)
    } else if sample.get("chat_messages").is_some() {
        Ok(DetectedFormat::ClaudeWeb)
    } else {
        Err(AppError::Parse("unknown export format (no `mapping` or `chat_messages`)".into()))
    }
}

pub fn parse_auto(json: &str) -> AppResult<(DetectedFormat, Vec<(Conversation, Vec<Message>)>)> {
    let v: Value = serde_json::from_str(json)?;
    let fmt = detect_format(&v)?;
    let parsed = match fmt {
        DetectedFormat::Openai => openai::parse_value(&v)?,
        DetectedFormat::ClaudeWeb => claude_web::parse_value(&v)?,
    };
    Ok((fmt, parsed))
}
