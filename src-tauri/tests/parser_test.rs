use memex_lib::parser::{detect_format, parse_auto, openai, claude_web, claude_code, DetectedFormat};
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
fn openai_token_counts_populated() {
    let json = std::fs::read_to_string("tests/fixtures/chatgpt_minimal.json").unwrap();
    let result = openai::parse_conversations_json(&json).unwrap();
    let (conv, msgs) = &result[0];
    assert!(conv.tokens.input > 0, "expected input tokens > 0");
    assert!(conv.tokens.output > 0, "expected output tokens > 0");
    assert!(msgs.iter().all(|m| m.tokens.is_some()));
    assert!(conv.estimated_cost_usd > 0.0);
}

#[test]
fn claude_web_token_counts_populated() {
    let json = std::fs::read_to_string("tests/fixtures/claude_web_minimal.json").unwrap();
    let result = claude_web::parse_conversations_json(&json).unwrap();
    let (conv, msgs) = &result[0];
    assert!(conv.tokens.input > 0);
    assert!(conv.tokens.output > 0);
    assert!(msgs.iter().all(|m| m.tokens.is_some()));
    assert!(conv.estimated_cost_usd > 0.0);
}

#[test]
fn claude_code_minimal_parses() {
    let path = "tests/fixtures/claude_code_minimal.jsonl";
    let result = claude_code::parse_session_file(path).unwrap();
    let (conv, msgs) = result.expect("session should produce a conversation");

    assert_eq!(conv.source, Source::ClaudeCode);
    assert_eq!(conv.native_id, "s1");
    assert_eq!(conv.title, "Hello world session");
    assert_eq!(conv.model.as_deref(), Some("claude-sonnet-4-6"));
    assert_eq!(conv.project.as_deref(), Some("/tmp/proj"));
    // u1 (text), a1 (text), u2 (tool_result), a2 (tool_use+text) — 4 messages
    assert_eq!(conv.message_count, 4);

    // Real token counts from the assistant `usage` block.
    assert_eq!(conv.tokens.input, 42 + 50);
    assert_eq!(conv.tokens.output, 7 + 10);
    assert_eq!(conv.tokens.cache_read, 12);
    assert_eq!(conv.tokens.cache_write, 0);

    // Cost from claude-sonnet-4-6 pricing exactly.
    let expected = (92.0 / 1e6) * 3.0 + (17.0 / 1e6) * 15.0 + (12.0 / 1e6) * 0.3;
    assert!(
        (conv.estimated_cost_usd - expected).abs() < 1e-6,
        "got {}, expected {}",
        conv.estimated_cost_usd,
        expected
    );

    // Conversation_id rewrites should be consistent.
    for m in msgs {
        assert_eq!(m.conversation_id, conv.id);
        assert!(m.id.starts_with(&conv.id));
    }
}

#[test]
fn claude_code_empty_session_returns_none() {
    let raw = r#"{"type":"permission-mode","sessionId":"s1","permissionMode":"default"}"#;
    let r = claude_code::parse_session_str(raw).unwrap();
    assert!(r.is_none());
}

#[test]
fn openai_per_message_cost_uses_message_model() {
    // Two assistant messages: one gpt-4o ($10/M output), one gpt-4o-mini ($0.6/M output).
    // Per-message cost = each priced separately. The pre-v0.2 logic would
    // pick the LAST model (gpt-4o-mini) and apply it to BOTH messages —
    // giving a much lower number than reality.
    let json = r#"[{
      "id":"c1","title":"x","create_time":1.0,"update_time":4.0,
      "default_model_slug":"gpt-4o","current_node":"n4",
      "mapping":{
        "root":{"id":"root","message":null,"parent":null,"children":["n1"]},
        "n1":{"id":"n1","parent":"root","children":["n2"],"message":{
            "id":"m1","author":{"role":"user"},"create_time":1.0,
            "content":{"content_type":"text","parts":["first user"]},"metadata":{}}},
        "n2":{"id":"n2","parent":"n1","children":["n3"],"message":{
            "id":"m2","author":{"role":"assistant"},"create_time":2.0,
            "content":{"content_type":"text","parts":["expensive answer that uses many tokens to inflate the gpt-4o output cost"]},
            "metadata":{"model_slug":"gpt-4o"}}},
        "n3":{"id":"n3","parent":"n2","children":["n4"],"message":{
            "id":"m3","author":{"role":"user"},"create_time":3.0,
            "content":{"content_type":"text","parts":["second user"]},"metadata":{}}},
        "n4":{"id":"n4","parent":"n3","children":[],"message":{
            "id":"m4","author":{"role":"assistant"},"create_time":4.0,
            "content":{"content_type":"text","parts":["cheap answer with similar length tokens to inflate the mini output cost"]},
            "metadata":{"model_slug":"gpt-4o-mini"}}}
      }}]"#;
    let r = openai::parse_conversations_json(json).unwrap();
    let (conv, msgs) = &r[0];

    let m4o = msgs.iter().find(|m| m.model.as_deref() == Some("gpt-4o")).unwrap();
    let mmini = msgs.iter().find(|m| m.model.as_deref() == Some("gpt-4o-mini")).unwrap();
    let t4o = m4o.tokens.as_ref().unwrap();
    let tmini = mmini.tokens.as_ref().unwrap();

    let cost_4o = (t4o.input as f64 / 1e6) * 2.5 + (t4o.output as f64 / 1e6) * 10.0;
    let cost_mini = (tmini.input as f64 / 1e6) * 0.15 + (tmini.output as f64 / 1e6) * 0.6;
    let expected = cost_4o + cost_mini;

    // Tolerance covers the small extra charge for user-message input tokens
    // billed at the fallback (last-known) model rate.
    assert!(
        (conv.estimated_cost_usd - expected).abs() < 1e-5,
        "per-msg cost should be ~${expected}, got ${}",
        conv.estimated_cost_usd
    );

    // The single-model fallback (using last model = gpt-4o-mini) would be much lower.
    let totals_total_in = t4o.input + tmini.input;
    let totals_total_out = t4o.output + tmini.output;
    let single_model_lowball =
        (totals_total_in as f64 / 1e6) * 0.15 + (totals_total_out as f64 / 1e6) * 0.6;
    assert!(
        conv.estimated_cost_usd > single_model_lowball * 1.5,
        "per-msg should beat single-model-mini lowball; got {} vs lowball {}",
        conv.estimated_cost_usd,
        single_model_lowball
    );
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
