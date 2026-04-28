# Memex Phase 2: Claude Code source + stat-quality fixes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `~/.claude/projects/*.jsonl` (Claude Code) as a third import source — with **real token counts and per-message model info**, fixing the v0.1 limitation where web exports require token estimation. Also fix two quality issues observed in v0.1: template-pollution in top-phrases, and overly-low cost estimates from single-model fallback.

**Architecture:** A new `parser/claude_code.rs` walks `~/.claude/projects/<encoded-cwd>/<session-uuid>.jsonl`, reads each line as a serde event, and aggregates back into the unified `Conversation` + `Message` schema. Each Claude Code session = one Memex conversation; each user/assistant turn (with embedded tool calls) = one message. Token counts and model come from the assistant event's `usage` and `message.model` fields directly (no estimation).

The phrase-quality and cost-accuracy fixes are independent and ship in the same release.

**Tech Stack:** No new crates. Reuses Tauri 2, rusqlite, serde, the existing schema, and the cost table.

**Out of scope for Phase 2:** Tantivy FTS, export-as-report, code-signing — those are v0.3 / v0.4.

---

## References

Read these once before starting:

- A real Claude Code jsonl: `~/.claude/projects/-Users-zzh/<uuid>.jsonl` — open in a viewer and look at the first 5 lines.
- The fields you'll actually consume per line:
  - `type`: `"user" | "assistant" | "system" | "summary" | "tool_use_result"` (and a few others — be permissive).
  - `sessionId`, `parentUuid`, `uuid`, `cwd`, `timestamp` (ISO8601), `gitBranch`, `version`.
  - For `assistant` lines: `message.model`, `message.usage.{input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens}`, `message.content` (array of `text` / `tool_use` / `thinking` blocks).
  - For `user` lines: `message.content` is a string OR an array of `text` / `tool_result` blocks.
- Existing schema: `src-tauri/src/schema.rs` — `Source::ClaudeCode` already exists.

---

## File Structure

```
Memex/
├── src-tauri/src/
│   ├── parser/
│   │   ├── mod.rs                      # ADD: claude_code import_dir(path) entry
│   │   └── claude_code.rs              # NEW: jsonl session parser
│   ├── stats/
│   │   ├── ngrams.rs                   # MODIFY: add template-pollution filter + tunable knobs
│   │   └── cost.rs                     # MODIFY: per-message cost (use msg.model not conv.model)
│   ├── commands.rs                     # MODIFY: add import_claude_code command
│   └── db.rs                           # MODIFY: cache_creation/cache_read message-level columns already exist; verify.
├── src-tauri/tests/
│   ├── fixtures/
│   │   └── claude_code_minimal.jsonl   # NEW
│   └── parser_test.rs                  # MODIFY: add claude_code tests
├── src/
│   ├── pages/
│   │   └── Onboarding.tsx              # MODIFY: add "Import Claude Code (~/.claude)" button
│   └── lib/api.ts                      # MODIFY: add importClaudeCode wrapper
└── docs/superpowers/plans/
    └── 2026-04-29-memex-phase-2-claude-code-source.md   # this file
```

**Decomposition principle:** Claude Code parsing is a single new file with one job. Cost-accuracy and ngram-quality fixes are independent edits in their respective files. Each task ends in a green test + commit.

---

## Task 1: Inspect a real session and capture the schema

**Files:** `src-tauri/tests/fixtures/claude_code_minimal.jsonl` (new)

- [ ] **Step 1: Pick a representative session and dump its first 5 lines**

```sh
SESSION=$(ls -t ~/.claude/projects/-Users-zzh/*.jsonl | head -1)
head -5 "$SESSION" | jq .
```

Read the output. Note:
- The first line is usually a `summary` event — has `type: "summary"`, `summary: "..."`, `leafUuid`.
- User events: `type: "user"`, `message: { role: "user", content: <string-or-array> }`.
- Assistant events: `type: "assistant"`, `message: { role: "assistant", model: "claude-sonnet-4-...", content: [...], usage: {...}, stop_reason: "..." }`.

- [ ] **Step 2: Hand-craft a minimal fixture that exercises the cases you saw**

Create `src-tauri/tests/fixtures/claude_code_minimal.jsonl` (one JSON object per line):

```jsonl
{"type":"summary","summary":"Hello world session","leafUuid":"a-leaf"}
{"parentUuid":null,"uuid":"u1","sessionId":"s1","cwd":"/tmp/proj","timestamp":"2026-04-01T10:00:00.000Z","type":"user","message":{"role":"user","content":"Hello, build me a thing"},"gitBranch":"main","version":"2.0"}
{"parentUuid":"u1","uuid":"a1","sessionId":"s1","cwd":"/tmp/proj","timestamp":"2026-04-01T10:00:05.000Z","type":"assistant","message":{"id":"msg_1","role":"assistant","model":"claude-sonnet-4-6","content":[{"type":"text","text":"Sure thing."}],"stop_reason":"end_turn","usage":{"input_tokens":42,"output_tokens":7,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}
{"parentUuid":"a1","uuid":"u2","sessionId":"s1","cwd":"/tmp/proj","timestamp":"2026-04-01T10:00:10.000Z","type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"x","content":"some output","is_error":false}]}}
{"parentUuid":"u2","uuid":"a2","sessionId":"s1","cwd":"/tmp/proj","timestamp":"2026-04-01T10:00:12.000Z","type":"assistant","message":{"id":"msg_2","role":"assistant","model":"claude-sonnet-4-6","content":[{"type":"tool_use","id":"x","name":"Bash","input":{"cmd":"ls"}},{"type":"text","text":"Done."}],"stop_reason":"end_turn","usage":{"input_tokens":50,"output_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":12}}}
```

- [ ] **Step 3: Commit**

```sh
git add src-tauri/tests/fixtures/claude_code_minimal.jsonl
git commit -m "test(claude_code): minimal fixture covering text + tool_use + tool_result"
```

---

## Task 2: jsonl session parser

**Files:** `src-tauri/src/parser/claude_code.rs` (new), `src-tauri/src/parser/mod.rs` (modify), `src-tauri/tests/parser_test.rs` (modify).

- [ ] **Step 1: Write the failing test (append to `parser_test.rs`)**

```rust
#[test]
fn claude_code_minimal_parses() {
    use memex_lib::parser::claude_code;
    let path = "tests/fixtures/claude_code_minimal.jsonl";
    let result = claude_code::parse_session_file(path).unwrap();
    let (conv, msgs) = result.expect("session should produce a conversation");

    assert_eq!(conv.source, Source::ClaudeCode);
    assert_eq!(conv.native_id, "s1");                       // sessionId
    assert_eq!(conv.title, "Hello world session");           // from summary event
    assert_eq!(conv.model.as_deref(), Some("claude-sonnet-4-6"));
    assert_eq!(conv.project.as_deref(), Some("/tmp/proj"));  // cwd
    assert_eq!(conv.message_count, 4);                       // u1, a1, u2(tool_result), a2

    // assistant tokens come from real `usage`, not estimation.
    assert_eq!(conv.tokens.input, 42 + 50);
    assert_eq!(conv.tokens.output, 7 + 10);
    assert_eq!(conv.tokens.cache_read, 12);

    // cost should use claude-sonnet-4-6 pricing exactly.
    let expected = (92.0/1e6)*3.0 + (17.0/1e6)*15.0 + (12.0/1e6)*0.3;
    assert!((conv.estimated_cost_usd - expected).abs() < 1e-6);
}
```

Run: `cargo test claude_code_minimal_parses` → expect **fail** (parser doesn't exist).

- [ ] **Step 2: Implement `parser/claude_code.rs`**

```rust
use crate::error::{AppError, AppResult};
use crate::schema::*;
use crate::stats::cost::estimate_cost_usd;
use chrono::DateTime;
use serde::Deserialize;
use std::path::Path;

/// Parse one .jsonl session file into a single Memex Conversation.
/// Returns `None` if the file has no usable user/assistant turns.
pub fn parse_session_file(path: impl AsRef<Path>) -> AppResult<Option<(Conversation, Vec<Message>)>> {
    let raw = std::fs::read_to_string(&path)?;
    parse_session_str(&raw)
}

pub fn parse_session_str(raw: &str) -> AppResult<Option<(Conversation, Vec<Message>)>> {
    let mut summary: Option<String> = None;
    let mut session_id: Option<String> = None;
    let mut cwd: Option<String> = None;
    let mut last_model: Option<String> = None;
    let mut first_ts: Option<i64> = None;
    let mut last_ts: Option<i64> = None;
    let mut messages: Vec<Message> = Vec::new();
    let mut totals = TokenCounts::zero();

    for (i, line) in raw.lines().enumerate() {
        if line.trim().is_empty() { continue; }
        let v: serde_json::Value = serde_json::from_str(line)
            .map_err(|e| AppError::Parse(format!("line {}: {}", i + 1, e)))?;

        let ty = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
        match ty {
            "summary" => {
                if summary.is_none() {
                    summary = v.get("summary").and_then(|s| s.as_str()).map(String::from);
                }
            }
            "user" => {
                if let Some(m) = build_user_message(&v, session_id.as_deref()) {
                    if first_ts.is_none() { first_ts = m.timestamp; }
                    last_ts = m.timestamp.or(last_ts);
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
                if let Some((m, usage)) = build_assistant_message(&v, session_id.as_deref()) {
                    if first_ts.is_none() { first_ts = m.timestamp; }
                    last_ts = m.timestamp.or(last_ts);
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
            // ignore: "system", "tool_use_result" (already merged into the user/assistant line by Claude Code), etc.
            _ => {}
        }
    }

    if messages.is_empty() || session_id.is_none() {
        return Ok(None);
    }
    let sid = session_id.unwrap();
    let conv_id = format!("claude_code:{sid}");
    // Re-stamp message ids with conv_id prefix and conversation_id with the right sid.
    for m in &mut messages {
        m.conversation_id = conv_id.clone();
        m.id = format!("{conv_id}:{}", m.id);
    }

    let title = summary
        .or_else(|| messages.iter().find(|m| m.role == Role::User).map(|m| m.content.chars().take(60).collect()))
        .unwrap_or_else(|| "(untitled)".into());

    let estimated_cost_usd = match last_model.as_deref() {
        Some(m) => estimate_cost_usd(m, &totals),
        None => estimate_cost_usd("claude", &totals),
    };

    Ok(Some((
        Conversation {
            id: conv_id,
            source: Source::ClaudeCode,
            native_id: sid,
            title,
            created_at: first_ts.unwrap_or(0),
            updated_at: last_ts.unwrap_or(0),
            model: last_model,
            project: cwd,
            message_count: messages.len() as u32,
            tokens: totals,
            estimated_cost_usd,
        },
        messages,
    )))
}

struct Usage { input: u64, output: u64, cache_read: u64, cache_write: u64 }

fn build_user_message(v: &serde_json::Value, _sid: Option<&str>) -> Option<Message> {
    let uuid = v.get("uuid").and_then(|s| s.as_str())?.to_string();
    let ts = v.get("timestamp").and_then(|s| s.as_str()).and_then(parse_iso);
    let content_v = v.pointer("/message/content")?;
    let content = extract_user_content(content_v);
    if content.is_empty() { return None; }
    Some(Message {
        id: uuid.clone(),
        conversation_id: String::new(), // re-stamped after we know conv_id
        role: Role::User,
        content,
        timestamp: ts,
        model: None,
        tokens: None,
        tool_name: None,
    })
}

fn build_assistant_message(v: &serde_json::Value, _sid: Option<&str>) -> Option<(Message, Usage)> {
    let uuid = v.get("uuid").and_then(|s| s.as_str())?.to_string();
    let ts = v.get("timestamp").and_then(|s| s.as_str()).and_then(parse_iso);
    let model = v.pointer("/message/model").and_then(|m| m.as_str()).map(String::from);
    let content_v = v.pointer("/message/content")?;
    let (content, tool_name) = extract_assistant_content(content_v);
    if content.is_empty() && tool_name.is_none() { return None; }

    let usage = v.pointer("/message/usage");
    let u = Usage {
        input: usage.and_then(|x| x.get("input_tokens")).and_then(|x| x.as_u64()).unwrap_or(0),
        output: usage.and_then(|x| x.get("output_tokens")).and_then(|x| x.as_u64()).unwrap_or(0),
        cache_read: usage.and_then(|x| x.get("cache_read_input_tokens")).and_then(|x| x.as_u64()).unwrap_or(0),
        cache_write: usage.and_then(|x| x.get("cache_creation_input_tokens")).and_then(|x| x.as_u64()).unwrap_or(0),
    };
    Some((
        Message {
            id: uuid,
            conversation_id: String::new(),
            role: Role::Assistant,
            content,
            timestamp: ts,
            model,
            tokens: Some(TokenCounts {
                input: u.input,
                output: u.output,
                cache_read: u.cache_read,
                cache_write: u.cache_write,
            }),
            tool_name,
        },
        u,
    ))
}

fn extract_user_content(v: &serde_json::Value) -> String {
    if let Some(s) = v.as_str() { return s.to_string(); }
    let arr = match v.as_array() { Some(a) => a, None => return String::new() };
    let parts: Vec<String> = arr.iter().filter_map(|b| {
        let ty = b.get("type").and_then(|t| t.as_str())?;
        match ty {
            "text" => b.get("text").and_then(|t| t.as_str()).map(String::from),
            "tool_result" => {
                let inner = b.get("content")?;
                Some(format!("[tool_result] {}", flatten_inner_content(inner)))
            }
            _ => None,
        }
    }).collect();
    parts.join("\n\n")
}

fn extract_assistant_content(v: &serde_json::Value) -> (String, Option<String>) {
    let arr = match v.as_array() { Some(a) => a, None => (return (String::new(), None)) };
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
                if first_tool.is_none() { first_tool = Some(name.to_string()); }
                texts.push(format!("[tool_use: {}]", name));
            }
            _ => {}
        }
    }
    (texts.join("\n\n"), first_tool)
}

fn flatten_inner_content(v: &serde_json::Value) -> String {
    if let Some(s) = v.as_str() { return s.to_string(); }
    if let Some(arr) = v.as_array() {
        return arr.iter().filter_map(|b| b.get("text").and_then(|t| t.as_str()).map(String::from))
            .collect::<Vec<_>>().join("\n");
    }
    v.to_string()
}

fn parse_iso(s: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.timestamp())
}
```

In `parser/mod.rs`, add `pub mod claude_code;`.

- [ ] **Step 3: Run, expect green**

```sh
cd src-tauri && cargo test claude_code_minimal_parses
```

- [ ] **Step 4: Commit**

```sh
git add src-tauri/src/parser/claude_code.rs src-tauri/src/parser/mod.rs src-tauri/tests/parser_test.rs
git commit -m "feat(parser): Claude Code .jsonl session parser with real token counts"
```

---

## Task 3: Directory walker + import command

**Files:** `src-tauri/src/parser/claude_code.rs` (extend), `src-tauri/src/commands.rs` (modify), `src-tauri/src/lib.rs` (modify).

- [ ] **Step 1: Add `parse_projects_dir` to `claude_code.rs`**

```rust
use std::path::PathBuf;

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
    if !dir.is_dir() { return Ok(()); }
    for entry in std::fs::read_dir(dir)? {
        let p = entry?.path();
        if p.is_dir() { walk_jsonl(&p, cb)?; }
        else if p.extension().and_then(|s| s.to_str()) == Some("jsonl") { cb(&p); }
    }
    Ok(())
}

pub fn default_projects_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("projects"))
}
```

Add `dirs = "5"` to `Cargo.toml` deps.

- [ ] **Step 2: Add `import_claude_code` Tauri command**

In `commands.rs`, append:

```rust
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
        None => crate::parser::claude_code::default_projects_dir()
            .ok_or_else(|| crate::error::AppError::Parse("could not find ~/.claude".into()))?,
    };
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
```

In `lib.rs`, register `commands::import_claude_code` in `generate_handler!`.

- [ ] **Step 3: Test against your real `~/.claude` (one-shot, do not commit)**

Add `tests/cc_smoke.rs`:

```rust
#[test]
#[ignore]
fn cc_smoke() {
    let dir = memex_lib::parser::claude_code::default_projects_dir().unwrap();
    let parsed = memex_lib::parser::claude_code::parse_projects_dir(&dir).unwrap();
    println!("parsed {} cc sessions", parsed.len());
    for (c, ms) in parsed.iter().take(3) {
        println!("  - {} ({} msgs, model={:?}, ${:.4})", c.title, ms.len(), c.model, c.estimated_cost_usd);
    }
    assert!(!parsed.is_empty());
}
```

Run: `cargo test --test cc_smoke -- --ignored --nocapture`. Verify reasonable output. **Delete the file before commit.**

- [ ] **Step 4: Commit**

```sh
git add src-tauri/src/parser/claude_code.rs src-tauri/src/commands.rs src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat(commands): import_claude_code with default ~/.claude/projects walker"
```

---

## Task 4: Frontend onboarding entry for Claude Code

**Files:** `src/lib/api.ts`, `src/pages/Onboarding.tsx`.

- [ ] **Step 1: Add the API wrapper**

In `src/lib/api.ts`:

```ts
importClaudeCode: (folder?: string) =>
  invoke<ImportSummary>("import_claude_code", { folder: folder ?? null }),
```

- [ ] **Step 2: Add a second card to Onboarding**

Replace the body of `src/pages/Onboarding.tsx` so both flows are visible side by side: a "Claude Code (auto)" card that calls `importClaudeCode()` with no folder argument, and the existing "Pick folder" card. Show the import summary the same way for both.

Concrete shape (only the changed portion):

```tsx
const [busyCC, setBusyCC] = useState(false)

async function importCC() {
  setError(null); setResult(null); setBusyCC(true)
  try {
    const r = await api.importClaudeCode()
    setResult(r)
  } catch (e) { setError(String(e)) }
  finally { setBusyCC(false) }
}

// Below the existing Choose Folder button:
<div className="mt-8 pt-8 border-t border-border">
  <h3 className="text-base font-semibold mb-1">Or import Claude Code sessions</h3>
  <p className="text-muted-foreground text-sm mb-3">
    Reads <code className="bg-muted px-1 py-0.5 rounded text-xs">~/.claude/projects/</code>
    {" "}directly. No file picker needed.
  </p>
  <Button onClick={importCC} disabled={busyCC} variant="secondary">
    {busyCC ? "Importing…" : "Import Claude Code"}
  </Button>
</div>
```

- [ ] **Step 3: Build + verify**

```sh
npm run build && npm run tauri dev
```

Click the new button. Expect ~50-200 sessions from your `~/.claude/projects/` to appear in the library.

- [ ] **Step 4: Commit**

```sh
git add src/lib/api.ts src/pages/Onboarding.tsx
git commit -m "feat(ui): one-click Claude Code import from ~/.claude"
```

---

## Task 5: Template-pollution filter for top phrases

**Files:** `src-tauri/src/stats/ngrams.rs` (modify), `src-tauri/src/stats/topics.rs` (modify).

**Why:** v0.1's top phrases were dominated by "option / question / question point question" because the user does multiple-choice annotation work where every prompt starts with a structured template. We want phrases that reflect *how the user talks*, not *templates they paste*.

- [ ] **Step 1: Add a failing test**

In `src-tauri/src/stats/ngrams.rs` `tests` module:

```rust
#[test]
fn template_phrases_are_demoted() {
    let texts = vec![
        "Question 1. Option A. Option B. Option C.".to_string(),
        "Question 2. Option A. Option B. Option C.".to_string(),
        "Question 3. Option A. Option B. Option C.".to_string(),
        "I don't understand this part".to_string(),
        "I don't understand the result".to_string(),
    ];
    let r = top_phrases(&texts, 5);
    let phrases: Vec<&str> = r.iter().map(|p| p.phrase.as_str()).collect();
    // Template tokens shouldn't dominate top-3.
    let template_in_top3 = phrases.iter().take(3).any(|p| p.contains("option") || p.contains("question"));
    assert!(!template_in_top3, "template phrases dominated top 3: {:?}", phrases);
    // The real catch-phrase should still surface.
    assert!(phrases.iter().any(|p| p.contains("don't understand")), "got {:?}", phrases);
}
```

Run: expect fail.

- [ ] **Step 2: Implement template detection in `extract_ngrams`**

```rust
/// A line is "template-like" if it has 3+ uppercase-letter+number bullets
/// (e.g., "Question 1. Option A. Option B.") or 4+ "Option X" / "选项X" patterns.
fn is_template_line(line: &str) -> bool {
    static BULLET: Lazy<regex::Regex> =
        Lazy::new(|| regex::Regex::new(r"(?i)\b(option|question|step|task|item|choice|选项|题目|问题)\s*[a-z0-9一二三四五六七八九十]+\b").unwrap());
    BULLET.find_iter(line).count() >= 3
}
```

In `extract_ngrams`, split the input into lines and skip template lines entirely:

```rust
pub fn extract_ngrams(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.split(|c| c == '\n' || c == '。') {
        if is_template_line(line) { continue; }
        out.extend(extract_ngrams_line(line));
    }
    out
}

fn extract_ngrams_line(text: &str) -> Vec<String> {
    /* the existing body of extract_ngrams */
}
```

- [ ] **Step 3: Run, expect green**

```sh
cargo test stats::ngrams
```

- [ ] **Step 4: Commit**

```sh
git add src-tauri/src/stats/ngrams.rs
git commit -m "feat(ngrams): demote template/multiple-choice patterns from top phrases"
```

---

## Task 6: Per-message cost (no more single-model fallback)

**Files:** `src-tauri/src/parser/claude_code.rs` (already correct), `src-tauri/src/parser/openai.rs` (modify), `src-tauri/src/parser/claude_web.rs` (no-op — keep "claude" fallback).

**Why:** v0.1 picked one model per conversation and applied it to all tokens. Real conversations span Opus+Sonnet+Haiku. With Claude Code we have per-message model in the JSON, so we should compute cost per-message and sum.

- [ ] **Step 1: Add a failing test**

In `parser_test.rs`:

```rust
#[test]
fn openai_per_message_cost_uses_message_model() {
    // synthetic JSON with two messages of different models
    let json = r#"[{
      "id":"c1","title":"x","create_time":1.0,"update_time":2.0,
      "default_model_slug":"gpt-4o","current_node":"n2",
      "mapping":{
        "root":{"id":"root","message":null,"parent":null,"children":["n1"]},
        "n1":{"id":"n1","parent":"root","children":["n2"],"message":{
            "id":"m1","author":{"role":"user"},"create_time":1.0,
            "content":{"content_type":"text","parts":["hi"]},"metadata":{}}},
        "n2":{"id":"n2","parent":"n1","children":[],"message":{
            "id":"m2","author":{"role":"assistant"},"create_time":2.0,
            "content":{"content_type":"text","parts":["hi"]},
            "metadata":{"model_slug":"gpt-4o-mini"}}}
      }}]"#;
    let r = openai::parse_conversations_json(json).unwrap();
    let (conv, msgs) = &r[0];
    // Conversation-level model still reports the latest used model:
    assert_eq!(conv.model.as_deref(), Some("gpt-4o-mini"));
    // But assistant message cost should use gpt-4o-mini's $0.6/M output, not gpt-4o's $10/M.
    let assistant = msgs.iter().find(|m| m.role == Role::Assistant).unwrap();
    let cost_assistant = (assistant.tokens.as_ref().unwrap().output as f64) * 0.6 / 1e6;
    // The conversation total should be at least cost_assistant and not more than 2x.
    assert!(conv.estimated_cost_usd >= cost_assistant - 1e-9);
    assert!(conv.estimated_cost_usd <  cost_assistant * 2.0 + 0.001);
}
```

Run: expect fail (current code uses one model for total).

- [ ] **Step 2: Modify `openai.rs` and `claude_code.rs` to compute cost per message**

Replace the per-conversation cost computation with:

```rust
let mut estimated_cost_usd = 0.0;
for m in &messages {
    let model = m.model.as_deref()
        .or(model_default.as_deref())
        .unwrap_or("");
    if let Some(t) = &m.tokens {
        estimated_cost_usd += estimate_cost_usd(model, t);
    }
}
```

Where `model_default` is the conversation-level model (so messages without a model attribute fall back).

Apply the same pattern to `claude_code.rs` (use the assistant line's `message.model`).

- [ ] **Step 3: Run, expect green**

```sh
cargo test
```

- [ ] **Step 4: Commit**

```sh
git add src-tauri/src/parser
git commit -m "feat(cost): per-message pricing instead of conversation-wide fallback"
```

---

## Task 7: Stats UX clarification

**Files:** `src/pages/Stats.tsx`.

The "Estimated cost" card already says "If billed via API · est. only". Strengthen it: add a one-line helper under the card that explicitly says the cost is **API-equivalent**, not what you actually paid via subscription, and note that web exports use approximate token counts.

- [ ] **Step 1: Add a footnote section under the top-line cards**

```tsx
<p className="text-xs text-muted-foreground -mt-4 max-w-2xl">
  <strong>Note on cost:</strong> this is the API-equivalent estimate — what these
  tokens would cost via the Anthropic / OpenAI APIs at current per-token rates.
  ChatGPT Plus and Claude Pro subscriptions are billed flat-rate and don't map
  directly to this number. Token counts for web exports are estimated via
  tiktoken (cl100k); Claude Code sessions use exact counts from the API response.
</p>
```

- [ ] **Step 2: Build + commit**

```sh
npm run build
git add src/pages/Stats.tsx
git commit -m "docs(ui): clarify that cost is API-equivalent, not subscription"
```

---

## Task 8: Build, package, tag

- [ ] **Step 1: Bump version to 0.2.0**

In `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`: change `0.1.0` → `0.2.0`.

- [ ] **Step 2: Build release**

```sh
npm run tauri build
ditto -c -k --sequesterRsrc --keepParent \
  src-tauri/target/release/bundle/macos/Memex.app \
  Memex_0.2.0_aarch64.zip
```

- [ ] **Step 3: Smoke test**

Open the new `.app`, click "Import Claude Code", verify the library now contains both your Claude.ai web sessions (from earlier) and your Claude Code sessions, and that the Stats top phrases no longer feature "option / question / question point".

- [ ] **Step 4: Tag and push**

```sh
git commit -am "chore: bump v0.2.0"
git tag -a v0.2.0 -m "Memex v0.2.0 — Claude Code source + stat-quality fixes"
git push origin main --tags
```

---

## Done criteria for Phase 2

- [ ] `cargo test` passes (lib + parser_test).
- [ ] One-click "Import Claude Code" produces conversations with `source = claude_code`, real `model` per session, and cost computed against the actual model.
- [ ] Top phrases on the user's real data no longer dominated by "option / question / question point question".
- [ ] Stats page has a clear note distinguishing API-equivalent cost from subscription cost.
- [ ] `Memex_0.2.0_aarch64.zip` produced and verified to launch.
