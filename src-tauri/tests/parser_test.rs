use memex_lib::parser::{detect_format, parse_auto, openai, claude_web, DetectedFormat};
use memex_lib::schema::{Role, Source};

#[test]
fn openai_minimal_parses() {
    let json = std::fs::read_to_string("tests/fixtures/chatgpt_minimal.json").unwrap();
    let result = openai::parse_conversations_json(&json).unwrap();
    assert_eq!(result.len(), 1);
    let (conv, msgs) = &result[0];
    assert_eq!(conv.title, "Hello world chat");
    assert_eq!(conv.native_id, "abc-123");
    assert_eq!(conv.created_at, 1700000000);
    assert_eq!(conv.message_count, 3);
    assert_eq!(conv.model.as_deref(), Some("gpt-4o"));
    assert_eq!(conv.source, Source::Openai);

    assert_eq!(msgs.len(), 3);
    assert_eq!(msgs[0].role, Role::User);
    assert_eq!(msgs[0].content, "Hi there");
    assert_eq!(msgs[1].role, Role::Assistant);
    assert_eq!(msgs[1].content, "Hello!");
    assert_eq!(msgs[2].content, "Cool, thanks");
}

#[test]
fn openai_empty_json_yields_empty_vec() {
    let result = openai::parse_conversations_json("[]").unwrap();
    assert!(result.is_empty());
}

#[test]
fn claude_web_minimal_parses() {
    let json = std::fs::read_to_string("tests/fixtures/claude_web_minimal.json").unwrap();
    let result = claude_web::parse_conversations_json(&json).unwrap();
    assert_eq!(result.len(), 1, "empty conv should be filtered out");
    let (conv, msgs) = &result[0];
    assert_eq!(conv.title, "Hello Claude chat");
    assert_eq!(conv.native_id, "conv-uuid-1");
    assert_eq!(conv.source, Source::ClaudeWeb);
    assert_eq!(conv.message_count, 2);
    // 2024-11-20T02:45:34 UTC
    assert_eq!(conv.created_at, 1732070734);

    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0].role, Role::User);
    assert_eq!(msgs[0].content, "Hi Claude");
    assert_eq!(msgs[1].role, Role::Assistant);
    assert_eq!(msgs[1].content, "Hello! How can I help today?");
}

#[test]
fn detect_distinguishes_formats() {
    let openai_json = std::fs::read_to_string("tests/fixtures/chatgpt_minimal.json").unwrap();
    let v: serde_json::Value = serde_json::from_str(&openai_json).unwrap();
    assert_eq!(detect_format(&v).unwrap(), DetectedFormat::Openai);

    let claude_json = std::fs::read_to_string("tests/fixtures/claude_web_minimal.json").unwrap();
    let v: serde_json::Value = serde_json::from_str(&claude_json).unwrap();
    assert_eq!(detect_format(&v).unwrap(), DetectedFormat::ClaudeWeb);
}

#[test]
fn parse_auto_dispatches_correctly() {
    let openai_json = std::fs::read_to_string("tests/fixtures/chatgpt_minimal.json").unwrap();
    let (fmt, result) = parse_auto(&openai_json).unwrap();
    assert_eq!(fmt, DetectedFormat::Openai);
    assert_eq!(result.len(), 1);

    let claude_json = std::fs::read_to_string("tests/fixtures/claude_web_minimal.json").unwrap();
    let (fmt, result) = parse_auto(&claude_json).unwrap();
    assert_eq!(fmt, DetectedFormat::ClaudeWeb);
    assert_eq!(result.len(), 1);
}
