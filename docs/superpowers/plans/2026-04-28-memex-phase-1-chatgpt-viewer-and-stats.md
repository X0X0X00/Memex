# Memex Phase 1: ChatGPT Viewer + Stats — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `Memex.dmg` and `Memex-Setup.exe` v0.1 — a Tauri desktop app where the user can drop a ChatGPT export folder, browse all conversations like a modern chat UI, and see top-line stats (totals, est. cost, busiest day, top phrases, top topics).

**Architecture:** Tauri 2 (Rust backend + React frontend). Rust parses `conversations.json` into a unified schema stored in SQLite. Frontend reads via `invoke()` Tauri commands. All processing local; zero network calls.

**Tech Stack:** Tauri 2, React 18 + TypeScript, Vite, Tailwind CSS + shadcn/ui, React Router, react-markdown + Shiki, Recharts, rusqlite (bundled), serde, tiktoken-rs, jieba-rs, regex, chrono.

**Out of scope for Phase 1:** Claude Code source, full-text search, export-as-report, code signing.

---

## References (read once before starting)

- Tauri 2 setup: https://tauri.app/start/
- Tauri commands (Rust ↔ JS): https://tauri.app/develop/calling-rust/
- rusqlite: https://docs.rs/rusqlite/
- tiktoken-rs: https://docs.rs/tiktoken-rs/
- jieba-rs: https://docs.rs/jieba-rs/
- ChatGPT export format reference: parse one real `conversations.json` first to see the shape (the user has one at `/Users/zzh/Visual Studio Code/chatgpt-export-2026-04-28/`).

---

## File Structure

```
Memex/
├── README.md                                # Install / run / build instructions
├── .gitignore
├── package.json                             # frontend deps + tauri scripts
├── vite.config.ts
├── tsconfig.json
├── tailwind.config.js
├── postcss.config.js
├── index.html
├── src/                                     # React frontend
│   ├── main.tsx                             # router root
│   ├── App.tsx                              # layout shell
│   ├── routes.tsx
│   ├── pages/
│   │   ├── Onboarding.tsx                   # drag/drop folder
│   │   ├── Library.tsx                      # list of conversations
│   │   ├── Conversation.tsx                 # single conversation view
│   │   └── Stats.tsx                        # dashboard
│   ├── components/
│   │   ├── ui/                              # shadcn-generated
│   │   ├── ConversationList.tsx
│   │   ├── MessageBubble.tsx
│   │   ├── Markdown.tsx                     # markdown + Shiki wrapper
│   │   ├── StatCard.tsx
│   │   ├── ActivityHeatmap.tsx
│   │   └── PhrasesList.tsx
│   ├── lib/
│   │   ├── api.ts                           # typed wrappers around invoke()
│   │   └── format.ts                        # number/date formatting
│   └── types.ts                             # mirrors Rust schema
├── src-tauri/                               # Rust backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── icons/
│   ├── build.rs
│   └── src/
│       ├── main.rs                          # entry; registers commands
│       ├── lib.rs
│       ├── schema.rs                        # Conversation, Message, Source types
│       ├── db.rs                            # SQLite open + migrations + queries
│       ├── parser/
│       │   ├── mod.rs
│       │   └── openai.rs                    # ChatGPT export → unified schema
│       ├── stats/
│       │   ├── mod.rs
│       │   ├── totals.rs                    # counts, tokens, cost
│       │   ├── activity.rs                  # heatmap, busiest day
│       │   ├── ngrams.rs                    # top phrases (EN + ZH)
│       │   ├── topics.rs                    # TF-IDF on titles + first user msg
│       │   └── cost.rs                      # model→price table
│       ├── commands.rs                      # all #[tauri::command] entry points
│       └── error.rs                         # AppError + From impls
├── tests/                                   # rust integration test fixtures
│   └── fixtures/
│       ├── chatgpt_minimal.json             # tiny synthetic export
│       └── chatgpt_real_redacted.json       # subset of zzh's, secrets stripped
└── docs/superpowers/specs/2026-04-28-memex-design.md   # the spec
```

**Decomposition principle:** parsers, stats jobs, and DB access each live in their own file with one responsibility. UI pages are thin — they call typed wrappers in `src/lib/api.ts` and render. Cost table, n-gram logic, etc. are pure functions tested in isolation.

---

## Task 1: Repo init + Tauri scaffolding

**Files:**
- Create: `Memex/.gitignore`, `Memex/README.md`, all Tauri scaffold files
- Create: `Memex/.gitattributes`

- [ ] **Step 1: Initialize git repo**

```bash
cd "/Users/zzh/Visual Studio Code/Memex"
git init -b main
```

Verify: `git status` shows the existing `docs/` folder as untracked.

- [ ] **Step 2: Scaffold Tauri 2 + React + TS via official template**

```bash
cd "/Users/zzh/Visual Studio Code/Memex"
# Use pnpm if available, otherwise npm
npm create tauri-app@latest -- --manager npm --template react-ts --identifier com.memex.app .
```

When prompted, accept defaults. Verify the following directories exist after: `src/`, `src-tauri/`, `package.json`, `vite.config.ts`.

- [ ] **Step 3: Verify dev mode boots**

```bash
npm install
npm run tauri dev
```

Expected: a window opens showing the Tauri+React starter page with "Hello Tauri!" or similar. Close window.

- [ ] **Step 4: Configure window defaults in `src-tauri/tauri.conf.json`**

Set:
- `productName`: `"Memex"`
- `version`: `"0.1.0"`
- `app.windows[0].title`: `"Memex"`
- `app.windows[0].width`: 1280, `height`: 800
- `app.windows[0].minWidth`: 900, `minHeight`: 600
- `bundle.identifier`: `"com.memex.app"`
- `bundle.shortDescription`: `"Browse and analyze your AI conversation history. Local-only."`

- [ ] **Step 5: Add `.gitignore`**

```gitignore
node_modules
dist
dist-ssr
.DS_Store
src-tauri/target
src-tauri/gen
*.log
.env
```

- [ ] **Step 6: Write README.md skeleton**

```markdown
# Memex

Browse and analyze your AI conversation history from ChatGPT and Claude Code. **100% local — no data leaves your machine.**

## Status
v0.1 — ChatGPT export viewer + stats.

## Install
- macOS: download `Memex.dmg` from Releases → drag to Applications.
- Windows: download `Memex-Setup.exe` from Releases → run.
- The app is currently unsigned. See [docs/install.md](docs/install.md) for bypass instructions.

## Develop
\`\`\`
npm install
npm run tauri dev
\`\`\`

## License
MIT
```

- [ ] **Step 7: Initial commit**

```bash
git add .
git commit -m "chore: scaffold Tauri 2 + React/TS app"
```

---

## Task 2: Tailwind + shadcn/ui setup

**Files:**
- Create: `tailwind.config.js`, `postcss.config.js`, `src/index.css`
- Modify: `src/main.tsx` to import index.css; `vite.config.ts` to add path alias

- [ ] **Step 1: Install Tailwind and dependencies**

```bash
npm install -D tailwindcss@^3 postcss autoprefixer
npx tailwindcss init -p
```

- [ ] **Step 2: Configure `tailwind.config.js`**

```js
/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: { extend: {} },
  plugins: [],
}
```

- [ ] **Step 3: Replace `src/index.css` with Tailwind directives**

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

:root { color-scheme: light dark; }
body { @apply bg-neutral-50 dark:bg-neutral-950 text-neutral-900 dark:text-neutral-100; }
```

- [ ] **Step 4: Set up shadcn/ui**

```bash
npx shadcn@latest init -d
```

Accept defaults. This creates `components.json` and `src/components/ui/`. Then add the components we'll need:

```bash
npx shadcn@latest add button card input table dialog scroll-area separator badge
```

- [ ] **Step 5: Add `@/` path alias in `vite.config.ts`**

```ts
import path from "node:path"
// inside defineConfig, add:
resolve: {
  alias: { "@": path.resolve(__dirname, "src") },
},
```

And in `tsconfig.json` paths:
```json
"baseUrl": ".",
"paths": { "@/*": ["src/*"] }
```

- [ ] **Step 6: Verify Tailwind works**

Edit `src/App.tsx` to render `<div className="p-8 text-3xl font-bold">Memex</div>`. Run `npm run tauri dev`. Expected: the title appears with proper styling.

- [ ] **Step 7: Commit**

```bash
git add .
git commit -m "chore: add Tailwind + shadcn/ui"
```

---

## Task 3: Define unified schema (Rust)

**Files:**
- Create: `src-tauri/src/schema.rs`
- Modify: `src-tauri/src/lib.rs` to declare module
- Test: inline `#[cfg(test)]` module in `schema.rs`

- [ ] **Step 1: Add deps to `src-tauri/Cargo.toml`**

```toml
[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1"
rusqlite = { version = "0.31", features = ["bundled"] }
```

- [ ] **Step 2: Write the failing test**

Create `src-tauri/src/schema.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source { Openai, ClaudeCode }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role { User, Assistant, System, Tool }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCounts {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}
impl TokenCounts {
    pub fn zero() -> Self { Self { input: 0, output: 0, cache_read: 0, cache_write: 0 } }
    pub fn total(&self) -> u64 { self.input + self.output + self.cache_read + self.cache_write }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub source: Source,
    pub native_id: String,
    pub title: String,
    pub created_at: i64,        // unix seconds
    pub updated_at: i64,
    pub model: Option<String>,
    pub project: Option<String>,
    pub message_count: u32,
    pub tokens: TokenCounts,
    pub estimated_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: Role,
    pub content: String,
    pub timestamp: Option<i64>,
    pub model: Option<String>,
    pub tokens: Option<TokenCounts>,
    pub tool_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_total_sums_all_categories() {
        let t = TokenCounts { input: 10, output: 20, cache_read: 30, cache_write: 40 };
        assert_eq!(t.total(), 100);
    }

    #[test]
    fn source_serializes_to_snake_case() {
        let s = serde_json::to_string(&Source::ClaudeCode).unwrap();
        assert_eq!(s, "\"claude_code\"");
    }
}
```

Add `pub mod schema;` to `src-tauri/src/lib.rs`.

- [ ] **Step 3: Run tests, expect them to pass (no failing test step here; it's pure data structure)**

```bash
cd src-tauri && cargo test schema
```

Expected: 2 passed.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/schema.rs src-tauri/src/lib.rs
git commit -m "feat(schema): unified conversation + message schema"
```

---

## Task 4: SQLite database layer

**Files:**
- Create: `src-tauri/src/db.rs`, `src-tauri/src/error.rs`
- Test: inline tests in `db.rs`

- [ ] **Step 1: Write `src-tauri/src/error.rs`**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("io error: {0}")] Io(#[from] std::io::Error),
    #[error("json error: {0}")] Json(#[from] serde_json::Error),
    #[error("db error: {0}")] Db(#[from] rusqlite::Error),
    #[error("parse error: {0}")] Parse(String),
}
impl serde::Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
pub type AppResult<T> = Result<T, AppError>;
```

Add `pub mod error;` to `lib.rs`.

- [ ] **Step 2: Write the failing test**

Create `src-tauri/src/db.rs`:

```rust
use crate::error::AppResult;
use crate::schema::*;
use rusqlite::{params, Connection};
use std::path::Path;

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS conversations (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL,
    native_id TEXT NOT NULL,
    title TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    model TEXT,
    project TEXT,
    message_count INTEGER NOT NULL DEFAULT 0,
    tok_input INTEGER NOT NULL DEFAULT 0,
    tok_output INTEGER NOT NULL DEFAULT 0,
    tok_cache_read INTEGER NOT NULL DEFAULT 0,
    tok_cache_write INTEGER NOT NULL DEFAULT 0,
    estimated_cost_usd REAL NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    seq INTEGER NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    timestamp INTEGER,
    model TEXT,
    tok_input INTEGER,
    tok_output INTEGER,
    tok_cache_read INTEGER,
    tok_cache_write INTEGER,
    tool_name TEXT
);

CREATE INDEX IF NOT EXISTS idx_messages_conv ON messages(conversation_id, seq);
CREATE INDEX IF NOT EXISTS idx_conversations_created ON conversations(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_conversations_source ON conversations(source);
"#;

pub fn open(path: &Path) -> AppResult<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(SCHEMA_SQL)?;
    Ok(conn)
}

pub fn open_in_memory() -> AppResult<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(SCHEMA_SQL)?;
    Ok(conn)
}

pub fn upsert_conversation(conn: &Connection, c: &Conversation) -> AppResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO conversations
         (id, source, native_id, title, created_at, updated_at, model, project,
          message_count, tok_input, tok_output, tok_cache_read, tok_cache_write, estimated_cost_usd)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        params![
            c.id,
            match c.source { Source::Openai => "openai", Source::ClaudeCode => "claude_code" },
            c.native_id, c.title, c.created_at, c.updated_at, c.model, c.project,
            c.message_count, c.tokens.input, c.tokens.output,
            c.tokens.cache_read, c.tokens.cache_write, c.estimated_cost_usd
        ],
    )?;
    Ok(())
}

pub fn insert_message(conn: &Connection, seq: u32, m: &Message) -> AppResult<()> {
    let role = match m.role { Role::User => "user", Role::Assistant => "assistant",
        Role::System => "system", Role::Tool => "tool" };
    let (i, o, cr, cw) = match &m.tokens {
        Some(t) => (Some(t.input as i64), Some(t.output as i64),
                    Some(t.cache_read as i64), Some(t.cache_write as i64)),
        None => (None, None, None, None),
    };
    conn.execute(
        "INSERT OR REPLACE INTO messages
         (id, conversation_id, seq, role, content, timestamp, model,
          tok_input, tok_output, tok_cache_read, tok_cache_write, tool_name)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![m.id, m.conversation_id, seq, role, m.content,
                m.timestamp, m.model, i, o, cr, cw, m.tool_name],
    )?;
    Ok(())
}

pub fn count_conversations(conn: &Connection) -> AppResult<i64> {
    Ok(conn.query_row("SELECT COUNT(*) FROM conversations", [], |r| r.get::<_, i64>(0))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_conv() -> Conversation {
        Conversation {
            id: "conv-1".into(), source: Source::Openai, native_id: "x".into(),
            title: "Hello".into(), created_at: 1700000000, updated_at: 1700000100,
            model: Some("gpt-4o".into()), project: None, message_count: 2,
            tokens: TokenCounts::zero(), estimated_cost_usd: 0.0,
        }
    }

    #[test]
    fn open_in_memory_creates_schema() {
        let conn = open_in_memory().unwrap();
        let count = count_conversations(&conn).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn upsert_then_count() {
        let conn = open_in_memory().unwrap();
        upsert_conversation(&conn, &sample_conv()).unwrap();
        upsert_conversation(&conn, &sample_conv()).unwrap(); // replace, not duplicate
        assert_eq!(count_conversations(&conn).unwrap(), 1);
    }

    #[test]
    fn insert_message_persists() {
        let conn = open_in_memory().unwrap();
        upsert_conversation(&conn, &sample_conv()).unwrap();
        let m = Message {
            id: "m-1".into(), conversation_id: "conv-1".into(), role: Role::User,
            content: "hi".into(), timestamp: Some(1700000050), model: None,
            tokens: None, tool_name: None,
        };
        insert_message(&conn, 0, &m).unwrap();
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1);
    }
}
```

Add `pub mod db;` to `lib.rs`.

- [ ] **Step 3: Run tests; expect 3 passed**

```bash
cd src-tauri && cargo test db::tests
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/db.rs src-tauri/src/error.rs src-tauri/src/lib.rs
git commit -m "feat(db): SQLite schema + upsert/insert helpers"
```

---

## Task 5: ChatGPT export parser

**Files:**
- Create: `src-tauri/src/parser/mod.rs`, `src-tauri/src/parser/openai.rs`
- Create: `src-tauri/tests/fixtures/chatgpt_minimal.json`
- Test: integration test in `src-tauri/tests/openai_parser.rs`

- [ ] **Step 1: Inspect the real export to understand the shape**

```bash
python3 -c "import json; d=json.load(open('/Users/zzh/Visual Studio Code/chatgpt-export-2026-04-28/data-614a29a1-29f2-4f3f-9970-f3b2468a02a5-1777389301-977465e4-batch-0000/conversations.json')); print(type(d), len(d) if isinstance(d, list) else list(d.keys())[:5]); print(json.dumps(d[0] if isinstance(d, list) else next(iter(d.values())), indent=2)[:1500])"
```

Read the output and note: top-level is a list of conversation objects. Each has `id`, `title`, `create_time`, `update_time`, `mapping` (id → node), `default_model_slug`, etc. The `mapping` is a tree of nodes; the actual chat is reconstructed by walking `current_node` upward via `parent` until root.

- [ ] **Step 2: Create a minimal synthetic fixture**

Create `src-tauri/tests/fixtures/chatgpt_minimal.json`:

```json
[
  {
    "id": "abc-123",
    "title": "Hello world chat",
    "create_time": 1700000000.0,
    "update_time": 1700000050.0,
    "default_model_slug": "gpt-4o",
    "current_node": "node-3",
    "mapping": {
      "node-root": {"id": "node-root", "message": null, "parent": null, "children": ["node-1"]},
      "node-1": {
        "id": "node-1",
        "message": {
          "id": "msg-1",
          "author": {"role": "user"},
          "create_time": 1700000010.0,
          "content": {"content_type": "text", "parts": ["Hi there"]},
          "metadata": {}
        },
        "parent": "node-root", "children": ["node-2"]
      },
      "node-2": {
        "id": "node-2",
        "message": {
          "id": "msg-2",
          "author": {"role": "assistant"},
          "create_time": 1700000020.0,
          "content": {"content_type": "text", "parts": ["Hello!"]},
          "metadata": {"model_slug": "gpt-4o"}
        },
        "parent": "node-1", "children": ["node-3"]
      },
      "node-3": {
        "id": "node-3",
        "message": {
          "id": "msg-3",
          "author": {"role": "user"},
          "create_time": 1700000030.0,
          "content": {"content_type": "text", "parts": ["Cool, thanks"]},
          "metadata": {}
        },
        "parent": "node-2", "children": []
      }
    }
  }
]
```

- [ ] **Step 3: Write the failing integration test**

Create `src-tauri/tests/openai_parser.rs`:

```rust
use memex_lib::parser::openai::parse_conversations_json;
use memex_lib::schema::Role;

#[test]
fn parses_minimal_export_into_unified_schema() {
    let json = std::fs::read_to_string("tests/fixtures/chatgpt_minimal.json").unwrap();
    let result = parse_conversations_json(&json).unwrap();
    assert_eq!(result.len(), 1);

    let (conv, msgs) = &result[0];
    assert_eq!(conv.title, "Hello world chat");
    assert_eq!(conv.native_id, "abc-123");
    assert_eq!(conv.created_at, 1700000000);
    assert_eq!(conv.message_count, 3);
    assert_eq!(conv.model.as_deref(), Some("gpt-4o"));

    assert_eq!(msgs.len(), 3);
    assert_eq!(msgs[0].role, Role::User);
    assert_eq!(msgs[0].content, "Hi there");
    assert_eq!(msgs[1].role, Role::Assistant);
    assert_eq!(msgs[1].content, "Hello!");
    assert_eq!(msgs[2].content, "Cool, thanks");
}

#[test]
fn empty_json_yields_empty_vec() {
    let result = parse_conversations_json("[]").unwrap();
    assert!(result.is_empty());
}
```

(Set `name = "memex_lib"` for the lib in `Cargo.toml` if needed, or reference the binary crate's module path.)

In `src-tauri/Cargo.toml`, ensure:
```toml
[lib]
name = "memex_lib"
path = "src/lib.rs"
```

- [ ] **Step 4: Run, expect compile failure (parser doesn't exist yet)**

```bash
cd src-tauri && cargo test --test openai_parser
```

Expected: error[E0432]: unresolved import.

- [ ] **Step 5: Implement parser**

Create `src-tauri/src/parser/mod.rs`:

```rust
pub mod openai;
```

Create `src-tauri/src/parser/openai.rs`:

```rust
use crate::error::{AppError, AppResult};
use crate::schema::*;
use serde::Deserialize;
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
    id: String,
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
    let raws: Vec<RawConv> = serde_json::from_str(s)?;
    let mut out = Vec::with_capacity(raws.len());
    for r in raws {
        if let Some(item) = build_one(r)? { out.push(item); }
    }
    Ok(out)
}

fn build_one(r: RawConv) -> AppResult<Option<(Conversation, Vec<Message>)>> {
    let leaf_id = match r.current_node.as_ref() {
        Some(id) => id.clone(),
        None => match pick_deepest_leaf(&r.mapping) {
            Some(id) => id, None => return Ok(None),
        },
    };

    // Walk leaf → root, then reverse to get chronological order.
    let mut path: Vec<&RawNode> = Vec::new();
    let mut cur = r.mapping.get(&leaf_id);
    while let Some(node) = cur {
        path.push(node);
        cur = node.parent.as_ref().and_then(|pid| r.mapping.get(pid));
    }
    path.reverse();

    let mut messages: Vec<Message> = Vec::new();
    let conv_id = format!("openai:{}", r.id);
    for node in path {
        let msg = match &node.message { Some(m) => m, None => continue };
        let role = parse_role(&msg.author.role);
        if role.is_none() { continue; }
        let content = extract_text(&msg.content);
        if content.is_empty() { continue; }
        let model = msg.metadata.get("model_slug")
            .and_then(|v| v.as_str()).map(String::from);
        messages.push(Message {
            id: format!("{conv_id}:{}", msg.id),
            conversation_id: conv_id.clone(),
            role: role.unwrap(),
            content,
            timestamp: msg.create_time.map(|f| f as i64),
            model,
            tokens: None,           // OpenAI export has no token counts; filled later
            tool_name: None,
        });
    }

    if messages.is_empty() { return Ok(None); }

    let title = r.title.unwrap_or_else(|| "(untitled)".to_string());
    let created = r.create_time.map(|f| f as i64).unwrap_or(0);
    let updated = r.update_time.map(|f| f as i64).unwrap_or(created);
    let model = messages.iter().rev().find_map(|m| m.model.clone())
        .or(r.default_model_slug);

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
    c.parts.iter()
        .filter_map(|v| v.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn pick_deepest_leaf(map: &HashMap<String, RawNode>) -> Option<String> {
    map.iter()
        .filter(|(_, n)| n.children.is_empty())
        .max_by_key(|(_, n)| depth(map, &n.id, 0))
        .map(|(k, _)| k.clone())
}

fn depth(map: &HashMap<String, RawNode>, id: &str, d: u32) -> u32 {
    match map.get(id).and_then(|n| n.parent.as_ref()) {
        Some(p) => depth(map, p, d + 1),
        None => d,
    }
}
```

Add `pub mod parser;` to `lib.rs`.

- [ ] **Step 6: Run tests, expect pass**

```bash
cd src-tauri && cargo test --test openai_parser
```

Expected: 2 passed.

- [ ] **Step 7: Smoke-test against the user's real export (one-shot, do not commit)**

Add a temporary integration test `src-tauri/tests/openai_smoke.rs` (gitignored or deleted before commit):

```rust
#[test]
#[ignore] // run with `cargo test -- --ignored`
fn smoke_real_export() {
    let path = "/Users/zzh/Visual Studio Code/chatgpt-export-2026-04-28/data-614a29a1-29f2-4f3f-9970-f3b2468a02a5-1777389301-977465e4-batch-0000/conversations.json";
    let json = std::fs::read_to_string(path).unwrap();
    let r = memex_lib::parser::openai::parse_conversations_json(&json).unwrap();
    println!("parsed {} conversations", r.len());
    for (c, ms) in r.iter().take(3) {
        println!("  - {} ({} msgs, {} tokens, ${:.4})",
                 c.title, ms.len(), c.tokens.input + c.tokens.output, c.estimated_cost_usd);
    }
    assert!(!r.is_empty());
}
```

Run:
```bash
cd src-tauri && cargo test --test openai_smoke -- --ignored --nocapture
```

Verify: prints a reasonable count + titles + token counts; no parse errors. Then **delete this file** before committing.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/parser src-tauri/src/lib.rs src-tauri/tests
git commit -m "feat(parser): ChatGPT export → unified schema"
```

---

## Task 6: Cost calculation (model price table)

**Files:**
- Create: `src-tauri/src/stats/cost.rs`, `src-tauri/src/stats/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/stats/mod.rs`:

```rust
pub mod cost;
```

Create `src-tauri/src/stats/cost.rs`:

```rust
use crate::schema::TokenCounts;

/// USD per 1M tokens. (input, output, cache_read, cache_write)
#[derive(Debug, Clone, Copy)]
pub struct ModelPrice {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

pub fn price_for(model: &str) -> Option<ModelPrice> {
    let m = model.to_lowercase();
    // Claude
    if m.starts_with("claude-opus-4-7") || m.starts_with("claude-opus-4")
        { return Some(ModelPrice { input: 15.0, output: 75.0, cache_read: 1.5, cache_write: 18.75 }); }
    if m.starts_with("claude-sonnet-4-6") || m.starts_with("claude-sonnet-4") || m.starts_with("claude-3-5-sonnet")
        { return Some(ModelPrice { input: 3.0, output: 15.0, cache_read: 0.3, cache_write: 3.75 }); }
    if m.starts_with("claude-haiku-4") || m.starts_with("claude-3-5-haiku")
        { return Some(ModelPrice { input: 0.8, output: 4.0, cache_read: 0.08, cache_write: 1.0 }); }
    // OpenAI
    if m.starts_with("gpt-4o-mini") { return Some(ModelPrice { input: 0.15, output: 0.6, cache_read: 0.075, cache_write: 0.0 }); }
    if m.starts_with("gpt-4o") || m == "gpt-4-turbo" { return Some(ModelPrice { input: 2.5, output: 10.0, cache_read: 1.25, cache_write: 0.0 }); }
    if m.starts_with("gpt-4") { return Some(ModelPrice { input: 30.0, output: 60.0, cache_read: 0.0, cache_write: 0.0 }); }
    if m.starts_with("gpt-3.5") { return Some(ModelPrice { input: 0.5, output: 1.5, cache_read: 0.0, cache_write: 0.0 }); }
    if m.starts_with("o1") || m.starts_with("o3") { return Some(ModelPrice { input: 15.0, output: 60.0, cache_read: 7.5, cache_write: 0.0 }); }
    None
}

pub fn estimate_cost_usd(model: &str, t: &TokenCounts) -> f64 {
    let p = match price_for(model) { Some(p) => p, None => return 0.0 };
    let m = 1_000_000.0;
    (t.input as f64) * p.input / m
        + (t.output as f64) * p.output / m
        + (t.cache_read as f64) * p.cache_read / m
        + (t.cache_write as f64) * p.cache_write / m
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_model_zero_cost() {
        let t = TokenCounts { input: 1000, output: 1000, cache_read: 0, cache_write: 0 };
        assert_eq!(estimate_cost_usd("foo-bar", &t), 0.0);
    }
    #[test]
    fn gpt4o_million_io_costs_right() {
        let t = TokenCounts { input: 1_000_000, output: 1_000_000, cache_read: 0, cache_write: 0 };
        let c = estimate_cost_usd("gpt-4o", &t);
        assert!((c - 12.5).abs() < 0.001, "got {c}");
    }
    #[test]
    fn claude_sonnet_matches_table() {
        let t = TokenCounts { input: 1_000_000, output: 1_000_000, cache_read: 1_000_000, cache_write: 1_000_000 };
        let c = estimate_cost_usd("claude-sonnet-4-6", &t);
        assert!((c - (3.0 + 15.0 + 0.3 + 3.75)).abs() < 0.001);
    }
}
```

Add `pub mod stats;` to `lib.rs`.

- [ ] **Step 2: Run tests, expect pass**

```bash
cd src-tauri && cargo test stats::cost
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/stats
git commit -m "feat(stats): model→price table + cost estimator"
```

---

## Task 7: Token estimation for OpenAI (tiktoken)

**Files:**
- Modify: `src-tauri/Cargo.toml` (add tiktoken-rs)
- Modify: `src-tauri/src/parser/openai.rs` (count tokens; populate `tokens` and convo total)

- [ ] **Step 1: Add dep**

```toml
tiktoken-rs = "0.5"
```

- [ ] **Step 2: Write the failing test (additional case in openai_parser.rs)**

Append to `src-tauri/tests/openai_parser.rs`:

```rust
#[test]
fn token_counts_populated_for_messages() {
    let json = std::fs::read_to_string("tests/fixtures/chatgpt_minimal.json").unwrap();
    let result = memex_lib::parser::openai::parse_conversations_json(&json).unwrap();
    let (conv, msgs) = &result[0];
    // user msgs counted as input, assistant as output
    assert!(conv.tokens.input > 0, "expected input tokens > 0");
    assert!(conv.tokens.output > 0, "expected output tokens > 0");
    // each message has its own count
    assert!(msgs.iter().all(|m| m.tokens.is_some()));
}

#[test]
fn cost_estimated_for_known_model() {
    let json = std::fs::read_to_string("tests/fixtures/chatgpt_minimal.json").unwrap();
    let result = memex_lib::parser::openai::parse_conversations_json(&json).unwrap();
    let (conv, _) = &result[0];
    assert!(conv.estimated_cost_usd > 0.0);
}
```

Run: should fail because tokens are still zeroed.

- [ ] **Step 3: Implement token counting in `parser/openai.rs`**

Add at top:
```rust
use tiktoken_rs::cl100k_base;
use crate::stats::cost::estimate_cost_usd;
```

Add helper:
```rust
fn count_tokens(s: &str) -> u64 {
    static BPE: once_cell::sync::Lazy<tiktoken_rs::CoreBPE> =
        once_cell::sync::Lazy::new(|| cl100k_base().expect("cl100k bpe"));
    BPE.encode_with_special_tokens(s).len() as u64
}
```

Add `once_cell = "1"` to deps.

In `build_one`, after pushing each message:
```rust
let tk = count_tokens(&content);
let counts = match role.unwrap() {
    Role::User | Role::System | Role::Tool => TokenCounts { input: tk, output: 0, cache_read: 0, cache_write: 0 },
    Role::Assistant => TokenCounts { input: 0, output: tk, cache_read: 0, cache_write: 0 },
};
let last = messages.last_mut().unwrap();
last.tokens = Some(counts);
```

After the loop, aggregate into the conversation:
```rust
let mut total = TokenCounts::zero();
for m in &messages {
    if let Some(t) = &m.tokens {
        total.input += t.input;
        total.output += t.output;
        total.cache_read += t.cache_read;
        total.cache_write += t.cache_write;
    }
}
let cost = model.as_deref().map(|m| estimate_cost_usd(m, &total)).unwrap_or(0.0);
// then in the Conversation constructor:
//   tokens: total, estimated_cost_usd: cost,
```

- [ ] **Step 4: Run tests, expect pass**

```bash
cd src-tauri && cargo test --test openai_parser
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/parser/openai.rs src-tauri/tests/openai_parser.rs
git commit -m "feat(parser): count tokens with tiktoken + estimate cost per conv"
```

---

## Task 8: Activity stats (busiest day, heatmap)

**Files:**
- Create: `src-tauri/src/stats/activity.rs`
- Modify: `src-tauri/src/stats/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/stats/activity.rs`:

```rust
use crate::error::AppResult;
use chrono::{DateTime, Datelike, Utc};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct DailyActivity {
    pub date: String,    // YYYY-MM-DD
    pub messages: u32,
}

#[derive(Debug, Serialize)]
pub struct ActivityReport {
    pub daily: Vec<DailyActivity>,
    pub busiest_day: Option<DailyActivity>,
    pub by_hour: [u32; 24],
    pub by_weekday: [u32; 7],   // 0 = Sunday
}

pub fn compute(conn: &Connection) -> AppResult<ActivityReport> {
    let mut stmt = conn.prepare(
        "SELECT timestamp FROM messages WHERE timestamp IS NOT NULL"
    )?;
    let mut by_day: HashMap<String, u32> = HashMap::new();
    let mut by_hour = [0u32; 24];
    let mut by_weekday = [0u32; 7];
    let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
    for ts in rows {
        let ts = ts?;
        let dt = DateTime::<Utc>::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now());
        let day = dt.format("%Y-%m-%d").to_string();
        *by_day.entry(day).or_insert(0) += 1;
        by_hour[dt.hour() as usize] += 1;
        by_weekday[dt.weekday().num_days_from_sunday() as usize] += 1;
    }
    let mut daily: Vec<_> = by_day.into_iter()
        .map(|(date, messages)| DailyActivity { date, messages })
        .collect();
    daily.sort_by(|a, b| a.date.cmp(&b.date));
    let busiest_day = daily.iter().max_by_key(|d| d.messages).cloned();
    Ok(ActivityReport { daily, busiest_day, by_hour, by_weekday })
}

impl Clone for DailyActivity {
    fn clone(&self) -> Self { Self { date: self.date.clone(), messages: self.messages } }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::*;
    use crate::schema::*;

    fn seed(conn: &Connection) {
        let conv = Conversation {
            id: "c1".into(), source: Source::Openai, native_id: "x".into(),
            title: "t".into(), created_at: 0, updated_at: 0, model: None, project: None,
            message_count: 3, tokens: TokenCounts::zero(), estimated_cost_usd: 0.0,
        };
        upsert_conversation(conn, &conv).unwrap();
        // 2024-06-01 12:00 UTC = 1717243200
        for (i, ts) in [1717243200i64, 1717243260, 1717329600].iter().enumerate() {
            insert_message(conn, i as u32, &Message {
                id: format!("m-{i}"), conversation_id: "c1".into(),
                role: Role::User, content: "x".into(),
                timestamp: Some(*ts), model: None, tokens: None, tool_name: None,
            }).unwrap();
        }
    }

    #[test]
    fn busiest_day_is_first() {
        let conn = open_in_memory().unwrap();
        seed(&conn);
        let r = compute(&conn).unwrap();
        assert_eq!(r.busiest_day.as_ref().unwrap().date, "2024-06-01");
        assert_eq!(r.busiest_day.as_ref().unwrap().messages, 2);
        assert_eq!(r.daily.len(), 2);
    }
}
```

Add `pub mod activity;` to `stats/mod.rs`.

- [ ] **Step 2: Run, expect pass**

```bash
cd src-tauri && cargo test stats::activity
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/stats
git commit -m "feat(stats): activity heatmap + busiest day"
```

---

## Task 9: N-gram extraction (top phrases, EN + ZH)

**Files:**
- Create: `src-tauri/src/stats/ngrams.rs`
- Modify: `Cargo.toml` (add `jieba-rs`, `regex`)

- [ ] **Step 1: Add deps**

```toml
jieba-rs = "0.6"
regex = "1"
```

- [ ] **Step 2: Write the failing test**

Create `src-tauri/src/stats/ngrams.rs`:

```rust
use crate::error::AppResult;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize, Clone)]
pub struct PhraseStat { pub phrase: String, pub count: u32, pub score: f64 }

pub fn top_phrases_for_user(conn: &Connection, k: usize) -> AppResult<Vec<PhraseStat>> {
    let mut stmt = conn.prepare(
        "SELECT content FROM messages WHERE role = 'user'"
    )?;
    let texts: Vec<String> = stmt.query_map([], |r| r.get::<_, String>(0))?
        .filter_map(|r| r.ok()).collect();
    Ok(top_phrases(&texts, k))
}

pub fn top_phrases(texts: &[String], k: usize) -> Vec<PhraseStat> {
    let mut counts: HashMap<String, u32> = HashMap::new();
    for t in texts {
        for ngram in extract_ngrams(t) {
            *counts.entry(ngram).or_insert(0) += 1;
        }
    }
    let mut v: Vec<PhraseStat> = counts.into_iter()
        .filter(|(p, c)| *c >= 2 && !is_too_short_or_noisy(p))
        .map(|(p, c)| {
            let chars = p.chars().count() as f64;
            PhraseStat { phrase: p, count: c, score: (c as f64) * (1.0 + chars.ln().max(0.0)) }
        })
        .collect();
    v.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    v.truncate(k);
    v
}

fn is_too_short_or_noisy(p: &str) -> bool {
    let chars = p.chars().count();
    if chars < 2 { return true; }
    if p.chars().all(|c| !c.is_alphabetic()) { return true; }
    false
}

fn extract_ngrams(text: &str) -> Vec<String> {
    let cjk_ratio = cjk_ratio(text);
    let tokens: Vec<String> = if cjk_ratio > 0.3 {
        tokenize_zh(text)
    } else {
        tokenize_en(text)
    };
    let stop = if cjk_ratio > 0.3 { ZH_STOPWORDS } else { EN_STOPWORDS };
    let filtered: Vec<&str> = tokens.iter()
        .map(String::as_str)
        .filter(|t| !stop.contains(t))
        .collect();
    let mut out = Vec::new();
    for n in 1..=3 {
        if filtered.len() < n { continue; }
        for w in filtered.windows(n) {
            let phrase: String = if cjk_ratio > 0.3 { w.concat() } else { w.join(" ") };
            out.push(phrase.to_lowercase());
        }
    }
    out
}

fn cjk_ratio(s: &str) -> f64 {
    let total = s.chars().count().max(1) as f64;
    let cjk = s.chars().filter(|c| {
        let cp = *c as u32;
        (0x4E00..=0x9FFF).contains(&cp) || (0x3000..=0x303F).contains(&cp)
    }).count() as f64;
    cjk / total
}

fn tokenize_en(s: &str) -> Vec<String> {
    static RE: once_cell::sync::Lazy<regex::Regex> =
        once_cell::sync::Lazy::new(|| regex::Regex::new(r"[a-zA-Z']+").unwrap());
    RE.find_iter(s).map(|m| m.as_str().to_lowercase()).collect()
}

fn tokenize_zh(s: &str) -> Vec<String> {
    static J: once_cell::sync::Lazy<jieba_rs::Jieba> =
        once_cell::sync::Lazy::new(jieba_rs::Jieba::new);
    J.cut(s, false).into_iter()
        .filter(|t| !t.trim().is_empty() && t.chars().any(|c| c.is_alphanumeric() || (c as u32) >= 0x4E00))
        .map(|t| t.to_string()).collect()
}

const EN_STOPWORDS: &[&str] = &[
    "the","a","an","is","are","was","were","i","you","he","she","it","we","they",
    "and","or","but","if","of","to","in","on","for","with","at","by","from","as",
    "this","that","these","those","be","been","being","have","has","had","do","does","did",
    "will","would","can","could","should","may","might","must","shall",
    "my","your","our","their","his","her","its","me","us","them",
    "not","no","yes","ok","okay","please","just","so","then","than","very",
    "what","when","where","which","who","why","how",
];

const ZH_STOPWORDS: &[&str] = &[
    "的","了","和","是","就","都","而","及","與","与","或","一","个","個","在","有","也","上",
    "我","你","他","她","它","我們","我们","你們","你们","他們","他们","這","这","那","什麼","什么","怎么","怎樣","怎样",
    "吗","嗎","呢","吧","啊","哦","嗯","然后","然後","但是","可是","就是",
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn english_top_phrases() {
        let texts = vec![
            "I don't understand the error".to_string(),
            "I don't understand why it fails".to_string(),
            "Can you explain the error".to_string(),
        ];
        let r = top_phrases(&texts, 5);
        assert!(r.iter().any(|p| p.phrase.contains("don't understand")), "got {:?}", r);
    }
    #[test]
    fn chinese_top_phrases() {
        let texts = vec![
            "我不明白这个错误".to_string(),
            "我不明白为什么".to_string(),
            "帮我解释一下错误".to_string(),
        ];
        let r = top_phrases(&texts, 5);
        assert!(r.iter().any(|p| p.phrase.contains("不明白")), "got {:?}", r);
    }
    #[test]
    fn singleton_phrases_filtered() {
        let texts = vec!["one off".to_string()];
        let r = top_phrases(&texts, 5);
        assert!(r.is_empty(), "single-occurrence phrases should be filtered");
    }
}
```

Add `pub mod ngrams;` to `stats/mod.rs`.

- [ ] **Step 3: Run, expect pass**

```bash
cd src-tauri && cargo test stats::ngrams
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/stats
git commit -m "feat(stats): n-gram top-phrase extraction (EN + ZH)"
```

---

## Task 10: Topic extraction (TF-IDF on titles + first user msg)

**Files:**
- Create: `src-tauri/src/stats/topics.rs`
- Modify: `src-tauri/src/stats/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/stats/topics.rs`:

```rust
use crate::error::AppResult;
use crate::stats::ngrams::PhraseStat;
use rusqlite::Connection;
use std::collections::HashMap;

pub fn top_topics(conn: &Connection, k: usize) -> AppResult<Vec<PhraseStat>> {
    // Use conversation titles + first user message as the "topic document" per conv.
    let mut stmt = conn.prepare(
        "SELECT c.id, c.title,
                (SELECT m.content FROM messages m
                 WHERE m.conversation_id = c.id AND m.role = 'user'
                 ORDER BY m.seq ASC LIMIT 1) AS first_user
         FROM conversations c"
    )?;
    let docs: Vec<String> = stmt.query_map([], |r| {
        let title: String = r.get(1)?;
        let first: Option<String> = r.get(2)?;
        Ok(format!("{title} {}", first.unwrap_or_default()))
    })?.filter_map(Result::ok).collect();

    let n_docs = docs.len() as f64;
    if n_docs < 2.0 {
        return Ok(crate::stats::ngrams::top_phrases(&docs, k));
    }

    // Token frequency in each doc
    let mut doc_terms: Vec<HashMap<String, u32>> = Vec::new();
    let mut df: HashMap<String, u32> = HashMap::new();
    for d in &docs {
        let mut tf: HashMap<String, u32> = HashMap::new();
        for ngram in crate::stats::ngrams::extract_ngrams(d) {
            *tf.entry(ngram).or_insert(0) += 1;
        }
        for k in tf.keys() { *df.entry(k.clone()).or_insert(0) += 1; }
        doc_terms.push(tf);
    }

    // TF-IDF aggregated across all docs
    let mut score: HashMap<String, f64> = HashMap::new();
    let mut count: HashMap<String, u32> = HashMap::new();
    for tf in &doc_terms {
        for (term, freq) in tf {
            let idf = ((n_docs + 1.0) / (*df.get(term).unwrap_or(&1) as f64 + 1.0)).ln() + 1.0;
            *score.entry(term.clone()).or_insert(0.0) += (*freq as f64) * idf;
            *count.entry(term.clone()).or_insert(0) += freq;
        }
    }

    let mut v: Vec<PhraseStat> = score.into_iter()
        .filter(|(t, _)| t.chars().count() >= 2)
        .map(|(phrase, s)| PhraseStat {
            count: *count.get(&phrase).unwrap_or(&0),
            score: s,
            phrase,
        })
        .collect();
    v.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    v.truncate(k);
    Ok(v)
}
```

Modify `src-tauri/src/stats/ngrams.rs` — expose extractor by changing `fn extract_ngrams` to `pub fn extract_ngrams`. The topic extractor reuses it directly.

Add `pub mod topics;` to `stats/mod.rs`.

Add a test inline in `topics.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::*;
    use crate::schema::*;

    fn seed_topics(conn: &Connection) {
        for (i, (title, first)) in [
            ("rust ownership", "explain rust ownership and borrowing"),
            ("rust lifetimes", "what are lifetimes in rust"),
            ("python decorators", "how do python decorators work"),
        ].iter().enumerate() {
            let cid = format!("c{i}");
            upsert_conversation(conn, &Conversation {
                id: cid.clone(), source: Source::Openai, native_id: format!("x{i}"),
                title: title.to_string(), created_at: 0, updated_at: 0,
                model: None, project: None, message_count: 1,
                tokens: TokenCounts::zero(), estimated_cost_usd: 0.0,
            }).unwrap();
            insert_message(conn, 0, &Message {
                id: format!("m{i}"), conversation_id: cid, role: Role::User,
                content: first.to_string(), timestamp: Some(0), model: None,
                tokens: None, tool_name: None,
            }).unwrap();
        }
    }

    #[test]
    fn rust_topic_ranks_high() {
        let conn = open_in_memory().unwrap();
        seed_topics(&conn);
        let r = top_topics(&conn, 5).unwrap();
        assert!(r.iter().any(|p| p.phrase.contains("rust")), "got {:?}", r);
    }
}
```

- [ ] **Step 2: Run, expect pass**

```bash
cd src-tauri && cargo test stats::topics
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/stats
git commit -m "feat(stats): TF-IDF topic extraction"
```

---

## Task 11: Tauri commands (the API surface)

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: Define an `AppState` for connection sharing**

Create `src-tauri/src/commands.rs`:

```rust
use crate::db;
use crate::error::AppResult;
use crate::parser::openai::parse_conversations_json;
use crate::schema::*;
use crate::stats::{activity, cost::*, ngrams::*, topics::*};
use rusqlite::Connection;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct AppState { pub db: Mutex<Connection> }

#[derive(Serialize)]
pub struct ImportSummary {
    pub conversations_added: u32,
    pub messages_added: u32,
    pub source: String,
}

#[tauri::command]
pub fn import_chatgpt_export(folder: String, state: tauri::State<AppState>) -> Result<ImportSummary, String> {
    do_import(folder, &state).map_err(|e| e.to_string())
}

fn do_import(folder: String, state: &tauri::State<AppState>) -> AppResult<ImportSummary> {
    let path = PathBuf::from(&folder).join("conversations.json");
    let json = std::fs::read_to_string(&path)?;
    let parsed = parse_conversations_json(&json)?;
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
    Ok(ImportSummary { conversations_added: convs, messages_added: msgs, source: "openai".into() })
}

#[derive(Serialize)]
pub struct ConversationSummary {
    pub id: String, pub source: String, pub title: String,
    pub created_at: i64, pub message_count: u32,
    pub model: Option<String>, pub estimated_cost_usd: f64,
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
         FROM conversations ORDER BY created_at DESC"
    )?;
    let rows = stmt.query_map([], |r| Ok(ConversationSummary {
        id: r.get(0)?, source: r.get(1)?, title: r.get(2)?,
        created_at: r.get(3)?, message_count: r.get(4)?, model: r.get(5)?,
        estimated_cost_usd: r.get(6)?, tokens_total: r.get::<_, i64>(7)? as u64,
    }))?;
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
         FROM messages WHERE conversation_id = ?1 ORDER BY seq ASC"
    )?;
    let rows = stmt.query_map([&id], |r| {
        let role: String = r.get(2)?;
        let role = match role.as_str() {
            "user" => Role::User, "assistant" => Role::Assistant,
            "system" => Role::System, _ => Role::Tool,
        };
        let toks = match (r.get::<_, Option<i64>>(6)?, r.get::<_, Option<i64>>(7)?,
                          r.get::<_, Option<i64>>(8)?, r.get::<_, Option<i64>>(9)?) {
            (Some(i), Some(o), Some(cr), Some(cw)) => Some(TokenCounts {
                input: i as u64, output: o as u64,
                cache_read: cr as u64, cache_write: cw as u64,
            }),
            _ => None,
        };
        Ok(Message {
            id: r.get(0)?, conversation_id: r.get(1)?, role,
            content: r.get(3)?, timestamp: r.get(4)?, model: r.get(5)?,
            tokens: toks, tool_name: r.get(10)?,
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
    pub activity: activity::ActivityReport,
    pub top_phrases: Vec<PhraseStat>,
    pub top_topics: Vec<PhraseStat>,
    pub by_model: Vec<(String, i64)>,
}

#[tauri::command]
pub fn get_stats(state: tauri::State<AppState>) -> Result<StatsReport, String> {
    do_stats(&state).map_err(|e| e.to_string())
}

fn do_stats(state: &tauri::State<AppState>) -> AppResult<StatsReport> {
    let conn = state.db.lock().unwrap();
    let total_conversations: i64 = conn.query_row(
        "SELECT COUNT(*) FROM conversations", [], |r| r.get(0))?;
    let total_messages: i64 = conn.query_row(
        "SELECT COUNT(*) FROM messages", [], |r| r.get(0))?;
    let (i,o,cr,cw,cost): (i64,i64,i64,i64,f64) = conn.query_row(
        "SELECT COALESCE(SUM(tok_input),0), COALESCE(SUM(tok_output),0),
                COALESCE(SUM(tok_cache_read),0), COALESCE(SUM(tok_cache_write),0),
                COALESCE(SUM(estimated_cost_usd),0) FROM conversations",
        [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?;
    let first_at: Option<i64> = conn.query_row(
        "SELECT MIN(created_at) FROM conversations", [], |r| r.get(0)).ok();
    let last_at: Option<i64> = conn.query_row(
        "SELECT MAX(created_at) FROM conversations", [], |r| r.get(0)).ok();

    let activity = activity::compute(&conn)?;
    let top_phrases = top_phrases_for_user(&conn, 10)?;
    let top_topics = top_topics(&conn, 10)?;

    let mut by_model: Vec<(String, i64)> = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT COALESCE(model,'(unknown)'), COUNT(*) FROM conversations GROUP BY model ORDER BY 2 DESC")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_,String>(0)?, r.get::<_,i64>(1)?)))?;
    for row in rows { by_model.push(row?); }

    Ok(StatsReport {
        total_conversations, total_messages,
        total_tokens: TokenCounts { input: i as u64, output: o as u64,
            cache_read: cr as u64, cache_write: cw as u64 },
        estimated_cost_usd: cost,
        first_at, last_at, activity, top_phrases, top_topics, by_model,
    })
}
```

- [ ] **Step 2: Wire it up in `main.rs`**

Replace `src-tauri/src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use memex_lib::{commands::*, db};
use std::sync::Mutex;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let conn = db::open(&dir.join("memex.db"))?;
            app.manage(AppState { db: Mutex::new(conn) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            import_chatgpt_export, list_conversations, get_conversation, get_stats
        ])
        .run(tauri::generate_context!())
        .expect("error while running memex");
}
```

Add `pub mod commands;` to `lib.rs`.

- [ ] **Step 3: `cargo check` and `npm run tauri dev`**

```bash
cd src-tauri && cargo check
cd .. && npm run tauri dev
```

Expected: app builds and opens. Window may still show the starter UI (we replace it next).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/main.rs src-tauri/src/lib.rs
git commit -m "feat(commands): import + list + get + stats"
```

---

## Task 12: Frontend foundation (router + types + api wrapper)

**Files:**
- Create: `src/types.ts`, `src/lib/api.ts`, `src/lib/format.ts`
- Modify: `src/main.tsx`, `src/App.tsx`
- Create: `src/routes.tsx`, page stubs in `src/pages/`

- [ ] **Step 1: Add deps**

```bash
npm install react-router-dom @tauri-apps/api@2 react-markdown remark-gfm shiki
```

- [ ] **Step 2: Define types in `src/types.ts`**

```typescript
export type Source = "openai" | "claude_code"
export type Role = "user" | "assistant" | "system" | "tool"

export interface TokenCounts {
  input: number; output: number; cache_read: number; cache_write: number
}

export interface ConversationSummary {
  id: string; source: string; title: string
  created_at: number; message_count: number
  model: string | null; estimated_cost_usd: number; tokens_total: number
}

export interface Message {
  id: string; conversation_id: string; role: Role; content: string
  timestamp: number | null; model: string | null
  tokens: TokenCounts | null; tool_name: string | null
}

export interface ImportSummary {
  conversations_added: number; messages_added: number; source: string
}

export interface DailyActivity { date: string; messages: number }
export interface ActivityReport {
  daily: DailyActivity[]
  busiest_day: DailyActivity | null
  by_hour: number[]; by_weekday: number[]
}
export interface PhraseStat { phrase: string; count: number; score: number }

export interface StatsReport {
  total_conversations: number; total_messages: number
  total_tokens: TokenCounts; estimated_cost_usd: number
  first_at: number | null; last_at: number | null
  activity: ActivityReport
  top_phrases: PhraseStat[]; top_topics: PhraseStat[]
  by_model: [string, number][]
}
```

- [ ] **Step 3: Create `src/lib/api.ts`**

```typescript
import { invoke } from "@tauri-apps/api/core"
import type { ConversationSummary, ImportSummary, Message, StatsReport } from "@/types"

export const api = {
  importChatgpt: (folder: string) =>
    invoke<ImportSummary>("import_chatgpt_export", { folder }),
  listConversations: () =>
    invoke<ConversationSummary[]>("list_conversations"),
  getConversation: (id: string) =>
    invoke<Message[]>("get_conversation", { id }),
  getStats: () =>
    invoke<StatsReport>("get_stats"),
}
```

- [ ] **Step 4: Create `src/lib/format.ts`**

```typescript
export const fmtNum = (n: number) => n.toLocaleString()
export const fmtUsd = (n: number) => "$" + n.toFixed(2)
export const fmtDate = (unix: number) =>
  new Date(unix * 1000).toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" })
export const fmtDateTime = (unix: number) =>
  new Date(unix * 1000).toLocaleString()
```

- [ ] **Step 5: Replace `src/App.tsx` with the layout shell**

```tsx
import { Outlet, NavLink } from "react-router-dom"

export default function App() {
  return (
    <div className="min-h-screen flex">
      <aside className="w-56 border-r border-neutral-200 dark:border-neutral-800 p-4 space-y-2">
        <h1 className="text-lg font-semibold mb-4">Memex</h1>
        {[
          { to: "/library", label: "Library" },
          { to: "/stats",   label: "Stats" },
          { to: "/import",  label: "Import" },
        ].map(({ to, label }) => (
          <NavLink key={to} to={to}
            className={({ isActive }) =>
              `block px-3 py-1.5 rounded text-sm ${isActive
                ? "bg-neutral-900 text-white dark:bg-neutral-100 dark:text-neutral-900"
                : "hover:bg-neutral-100 dark:hover:bg-neutral-900"}`}>
            {label}
          </NavLink>
        ))}
      </aside>
      <main className="flex-1 overflow-auto"><Outlet /></main>
    </div>
  )
}
```

- [ ] **Step 6: Create `src/routes.tsx`**

```tsx
import { createBrowserRouter, Navigate } from "react-router-dom"
import App from "./App"
import Onboarding from "./pages/Onboarding"
import Library from "./pages/Library"
import Conversation from "./pages/Conversation"
import Stats from "./pages/Stats"

export const router = createBrowserRouter([{
  path: "/", element: <App />, children: [
    { index: true, element: <Navigate to="/library" replace /> },
    { path: "import", element: <Onboarding /> },
    { path: "library", element: <Library /> },
    { path: "conversation/:id", element: <Conversation /> },
    { path: "stats", element: <Stats /> },
  ],
}])
```

- [ ] **Step 7: Update `src/main.tsx`**

```tsx
import React from "react"
import ReactDOM from "react-dom/client"
import { RouterProvider } from "react-router-dom"
import { router } from "./routes"
import "./index.css"

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode><RouterProvider router={router} /></React.StrictMode>
)
```

- [ ] **Step 8: Create page stubs**

`src/pages/Onboarding.tsx`, `Library.tsx`, `Conversation.tsx`, `Stats.tsx` — each just `export default function X(){ return <div className="p-8">X page</div> }`. We fill them in subsequent tasks.

- [ ] **Step 9: Verify**

```bash
npm run tauri dev
```

Expected: window opens, sidebar visible, navigation works between empty pages.

- [ ] **Step 10: Commit**

```bash
git add src/ package.json package-lock.json
git commit -m "feat(ui): router + sidebar shell + api wrappers"
```

---

## Task 13: Onboarding page (folder picker → import)

**Files:**
- Modify: `src/pages/Onboarding.tsx`
- Modify: `src-tauri/Cargo.toml` (add tauri-plugin-dialog), `src-tauri/src/main.rs`, `src-tauri/tauri.conf.json` (allowlist)

- [ ] **Step 1: Install dialog plugin**

```bash
npm install @tauri-apps/plugin-dialog
```

In `src-tauri/Cargo.toml`:
```toml
tauri-plugin-dialog = "2"
```

In `src-tauri/src/main.rs`, add `.plugin(tauri_plugin_dialog::init())` to the builder chain.

In `src-tauri/capabilities/default.json` (or whatever capability file Tauri 2 generated), add `"dialog:default"` to permissions.

- [ ] **Step 2: Implement `Onboarding.tsx`**

```tsx
import { useState } from "react"
import { open } from "@tauri-apps/plugin-dialog"
import { api } from "@/lib/api"
import type { ImportSummary } from "@/types"
import { Button } from "@/components/ui/button"
import { useNavigate } from "react-router-dom"

export default function Onboarding() {
  const [busy, setBusy] = useState(false)
  const [result, setResult] = useState<ImportSummary | null>(null)
  const [error, setError] = useState<string | null>(null)
  const nav = useNavigate()

  async function pickFolder() {
    setError(null); setResult(null)
    const folder = await open({ directory: true, multiple: false })
    if (!folder || typeof folder !== "string") return
    try {
      setBusy(true)
      const r = await api.importChatgpt(folder)
      setResult(r)
    } catch (e: any) {
      setError(String(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="p-12 max-w-2xl">
      <h2 className="text-2xl font-semibold mb-2">Import your ChatGPT export</h2>
      <p className="text-neutral-500 mb-6 text-sm">
        Pick the folder you got from ChatGPT (the one with <code>conversations.json</code> in it).
        Nothing leaves your machine.
      </p>
      <Button onClick={pickFolder} disabled={busy}>
        {busy ? "Importing…" : "Choose folder"}
      </Button>
      {error && (
        <div className="mt-4 p-3 rounded bg-red-50 text-red-800 text-sm">{error}</div>
      )}
      {result && (
        <div className="mt-4 p-3 rounded bg-green-50 text-green-900 text-sm">
          Imported {result.conversations_added} conversations,
          {" "}{result.messages_added} messages.
          <button onClick={() => nav("/library")} className="ml-3 underline">Open library →</button>
        </div>
      )}
    </div>
  )
}
```

- [ ] **Step 3: Verify**

Run dev. Click "Import" in sidebar. Pick the folder `/Users/zzh/Visual Studio Code/chatgpt-export-2026-04-28/data-614a29a1-29f2-4f3f-9970-f3b2468a02a5-1777389301-977465e4-batch-0000`. Expected: a green box "Imported N conversations, M messages."

- [ ] **Step 4: Commit**

```bash
git add src/pages/Onboarding.tsx src-tauri/
git commit -m "feat(ui): onboarding folder picker + import"
```

---

## Task 14: Library page (list of conversations)

**Files:**
- Modify: `src/pages/Library.tsx`
- Create: `src/components/ConversationList.tsx`

- [ ] **Step 1: Implement Library page**

`src/pages/Library.tsx`:

```tsx
import { useEffect, useMemo, useState } from "react"
import { Link } from "react-router-dom"
import { api } from "@/lib/api"
import type { ConversationSummary } from "@/types"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"
import { fmtDate, fmtNum, fmtUsd } from "@/lib/format"

export default function Library() {
  const [items, setItems] = useState<ConversationSummary[]>([])
  const [q, setQ] = useState("")
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    api.listConversations()
      .then(setItems)
      .finally(() => setLoading(false))
  }, [])

  const filtered = useMemo(() => {
    const needle = q.trim().toLowerCase()
    if (!needle) return items
    return items.filter(c => c.title.toLowerCase().includes(needle))
  }, [items, q])

  if (loading) return <div className="p-8 text-neutral-500">Loading…</div>
  if (items.length === 0) return (
    <div className="p-8 text-neutral-500">
      No conversations yet. <Link to="/import" className="underline">Import an export</Link> to get started.
    </div>
  )

  return (
    <div className="p-8 space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-semibold">Library</h2>
        <Input placeholder="Search titles…" value={q} onChange={e => setQ(e.target.value)} className="w-64" />
      </div>
      <div className="text-sm text-neutral-500">{filtered.length} / {items.length} conversations</div>
      <div className="border rounded divide-y dark:border-neutral-800 dark:divide-neutral-800">
        {filtered.map(c => (
          <Link key={c.id} to={`/conversation/${encodeURIComponent(c.id)}`}
                className="block px-4 py-3 hover:bg-neutral-50 dark:hover:bg-neutral-900">
            <div className="flex items-center justify-between gap-4">
              <div className="min-w-0 flex-1">
                <div className="truncate font-medium">{c.title || "(untitled)"}</div>
                <div className="text-xs text-neutral-500 mt-0.5 flex gap-3">
                  <span>{fmtDate(c.created_at)}</span>
                  <span>{fmtNum(c.message_count)} msgs</span>
                  <span>{fmtNum(c.tokens_total)} tokens</span>
                  {c.estimated_cost_usd > 0 && <span>{fmtUsd(c.estimated_cost_usd)}</span>}
                </div>
              </div>
              <div className="flex items-center gap-2 shrink-0">
                {c.model && <Badge variant="secondary">{c.model}</Badge>}
                <Badge>{c.source}</Badge>
              </div>
            </div>
          </Link>
        ))}
      </div>
    </div>
  )
}
```

- [ ] **Step 2: Verify**

Run dev. Navigate to "/library". Expected: list of imported conversations with titles, dates, msg counts.

- [ ] **Step 3: Commit**

```bash
git add src/pages/Library.tsx
git commit -m "feat(ui): library list with search"
```

---

## Task 15: Conversation viewer (markdown + Shiki)

**Files:**
- Create: `src/components/Markdown.tsx`, `src/components/MessageBubble.tsx`
- Modify: `src/pages/Conversation.tsx`

- [ ] **Step 1: Create `Markdown.tsx`**

```tsx
import ReactMarkdown from "react-markdown"
import remarkGfm from "remark-gfm"
import { useEffect, useState } from "react"
import { codeToHtml } from "shiki"

function CodeBlock({ language, value }: { language: string; value: string }) {
  const [html, setHtml] = useState("")
  useEffect(() => {
    codeToHtml(value, { lang: language || "text", theme: "github-light" })
      .then(setHtml).catch(() => setHtml(`<pre>${value}</pre>`))
  }, [language, value])
  return <div className="rounded overflow-hidden text-sm" dangerouslySetInnerHTML={{ __html: html }} />
}

export function Markdown({ children }: { children: string }) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      components={{
        code({ inline, className, children }) {
          const match = /language-(\w+)/.exec(className || "")
          if (inline || !match) return <code className="px-1 py-0.5 rounded bg-neutral-100 dark:bg-neutral-800">{children}</code>
          return <CodeBlock language={match[1]} value={String(children).replace(/\n$/, "")} />
        }
      }}
    >{children}</ReactMarkdown>
  )
}
```

- [ ] **Step 2: Create `MessageBubble.tsx`**

```tsx
import type { Message } from "@/types"
import { Markdown } from "./Markdown"

const roleLabel: Record<Message["role"], string> = {
  user: "You", assistant: "Assistant", system: "System", tool: "Tool",
}
const roleStyle: Record<Message["role"], string> = {
  user: "bg-blue-50 dark:bg-blue-950/30",
  assistant: "bg-neutral-50 dark:bg-neutral-900",
  system: "bg-amber-50 dark:bg-amber-950/30",
  tool: "bg-purple-50 dark:bg-purple-950/30",
}

export function MessageBubble({ m }: { m: Message }) {
  return (
    <div className={`px-6 py-5 ${roleStyle[m.role]}`}>
      <div className="text-xs uppercase tracking-wider text-neutral-500 mb-2">
        {roleLabel[m.role]}{m.model ? ` · ${m.model}` : ""}
      </div>
      <div className="prose prose-sm dark:prose-invert max-w-none">
        <Markdown>{m.content}</Markdown>
      </div>
    </div>
  )
}
```

Add `@tailwindcss/typography`:
```bash
npm install -D @tailwindcss/typography
```
In `tailwind.config.js`:
```js
plugins: [require("@tailwindcss/typography")],
```

- [ ] **Step 3: Implement `Conversation.tsx`**

```tsx
import { useEffect, useState } from "react"
import { useParams, Link } from "react-router-dom"
import { api } from "@/lib/api"
import type { Message } from "@/types"
import { MessageBubble } from "@/components/MessageBubble"

export default function Conversation() {
  const { id } = useParams()
  const [msgs, setMsgs] = useState<Message[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    if (!id) return
    setLoading(true)
    api.getConversation(decodeURIComponent(id))
      .then(setMsgs).finally(() => setLoading(false))
  }, [id])

  if (loading) return <div className="p-8 text-neutral-500">Loading…</div>
  if (!msgs.length) return <div className="p-8 text-neutral-500">No messages.</div>

  return (
    <div>
      <div className="px-6 py-3 border-b dark:border-neutral-800 sticky top-0 bg-white/80 dark:bg-neutral-950/80 backdrop-blur">
        <Link to="/library" className="text-sm text-neutral-500 hover:underline">← Library</Link>
      </div>
      {msgs.map(m => <MessageBubble key={m.id} m={m} />)}
    </div>
  )
}
```

- [ ] **Step 4: Verify**

Run dev → Library → click a conversation. Expected: chat-style rendering with role labels, code blocks highlighted.

- [ ] **Step 5: Commit**

```bash
git add src/components src/pages/Conversation.tsx tailwind.config.js package.json package-lock.json
git commit -m "feat(ui): conversation viewer with markdown + Shiki"
```

---

## Task 16: Stats dashboard

**Files:**
- Modify: `src/pages/Stats.tsx`
- Create: `src/components/StatCard.tsx`, `src/components/ActivityHeatmap.tsx`, `src/components/PhrasesList.tsx`

- [ ] **Step 1: Add Recharts**

```bash
npm install recharts
```

- [ ] **Step 2: Create `StatCard.tsx`**

```tsx
export function StatCard({ label, value, sub }: { label: string; value: string; sub?: string }) {
  return (
    <div className="p-4 border rounded-lg dark:border-neutral-800">
      <div className="text-xs uppercase tracking-wider text-neutral-500">{label}</div>
      <div className="text-2xl font-semibold mt-1">{value}</div>
      {sub && <div className="text-xs text-neutral-500 mt-1">{sub}</div>}
    </div>
  )
}
```

- [ ] **Step 3: Create `ActivityHeatmap.tsx`**

```tsx
import type { DailyActivity } from "@/types"

// Render last 365 days as a GitHub-style grid (7 rows x 53 cols).
export function ActivityHeatmap({ daily }: { daily: DailyActivity[] }) {
  const map = new Map(daily.map(d => [d.date, d.messages]))
  const today = new Date(); today.setHours(0,0,0,0)
  const start = new Date(today); start.setDate(start.getDate() - 364)
  // Snap start to previous Sunday
  start.setDate(start.getDate() - start.getDay())

  const cols = 53
  const cells: { date: string; count: number; col: number; row: number }[] = []
  let max = 0
  for (let i = 0; i < cols * 7; i++) {
    const d = new Date(start); d.setDate(start.getDate() + i)
    if (d > today) break
    const date = d.toISOString().slice(0,10)
    const count = map.get(date) ?? 0
    if (count > max) max = count
    cells.push({ date, count, col: Math.floor(i/7), row: i % 7 })
  }
  function shade(count: number) {
    if (count === 0) return "bg-neutral-100 dark:bg-neutral-900"
    const t = max > 0 ? count / max : 0
    if (t < 0.25) return "bg-green-200 dark:bg-green-900"
    if (t < 0.5)  return "bg-green-400 dark:bg-green-700"
    if (t < 0.75) return "bg-green-500 dark:bg-green-600"
    return "bg-green-600 dark:bg-green-500"
  }
  return (
    <div className="grid grid-flow-col grid-rows-7 gap-[2px]" style={{ gridTemplateColumns: `repeat(${cols}, 10px)` }}>
      {cells.map(c => (
        <div key={c.date} className={`w-[10px] h-[10px] rounded-sm ${shade(c.count)}`}
             title={`${c.date} · ${c.count} messages`} />
      ))}
    </div>
  )
}
```

- [ ] **Step 4: Create `PhrasesList.tsx`**

```tsx
import type { PhraseStat } from "@/types"

export function PhrasesList({ phrases }: { phrases: PhraseStat[] }) {
  if (!phrases.length) return <div className="text-sm text-neutral-500">Not enough data yet.</div>
  return (
    <ol className="space-y-1">
      {phrases.map((p, i) => (
        <li key={p.phrase} className="flex items-baseline justify-between gap-3 text-sm">
          <span className="text-neutral-400 w-6 shrink-0">{i+1}.</span>
          <span className="flex-1 truncate font-medium">{p.phrase}</span>
          <span className="text-neutral-500 shrink-0">×{p.count}</span>
        </li>
      ))}
    </ol>
  )
}
```

- [ ] **Step 5: Implement `Stats.tsx`**

```tsx
import { useEffect, useState } from "react"
import { api } from "@/lib/api"
import type { StatsReport } from "@/types"
import { StatCard } from "@/components/StatCard"
import { ActivityHeatmap } from "@/components/ActivityHeatmap"
import { PhrasesList } from "@/components/PhrasesList"
import { fmtDate, fmtNum, fmtUsd } from "@/lib/format"
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer } from "recharts"

export default function Stats() {
  const [s, setS] = useState<StatsReport | null>(null)

  useEffect(() => { api.getStats().then(setS) }, [])
  if (!s) return <div className="p-8 text-neutral-500">Loading…</div>

  const totalTok = s.total_tokens.input + s.total_tokens.output + s.total_tokens.cache_read + s.total_tokens.cache_write
  const span = s.first_at && s.last_at
    ? `${fmtDate(s.first_at)} – ${fmtDate(s.last_at)}` : "—"

  const hourData = s.activity.by_hour.map((v,i) => ({ hour: i, count: v }))
  const weekdays = ["Sun","Mon","Tue","Wed","Thu","Fri","Sat"]
  const weekData = s.activity.by_weekday.map((v,i) => ({ day: weekdays[i], count: v }))

  return (
    <div className="p-8 space-y-8 max-w-6xl">
      <h2 className="text-2xl font-semibold">Stats</h2>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <StatCard label="Conversations" value={fmtNum(s.total_conversations)} sub={span} />
        <StatCard label="Messages"      value={fmtNum(s.total_messages)} />
        <StatCard label="Tokens"        value={fmtNum(totalTok)}
                  sub={`in ${fmtNum(s.total_tokens.input)} · out ${fmtNum(s.total_tokens.output)}`} />
        <StatCard label="Estimated cost" value={fmtUsd(s.estimated_cost_usd)} sub="OpenAI tokens are estimated" />
      </div>

      <section>
        <h3 className="text-lg font-medium mb-3">
          Activity
          {s.activity.busiest_day && (
            <span className="ml-3 text-sm text-neutral-500">
              busiest day: {s.activity.busiest_day.date} ({s.activity.busiest_day.messages} msgs)
            </span>
          )}
        </h3>
        <div className="overflow-x-auto"><ActivityHeatmap daily={s.activity.daily} /></div>
      </section>

      <div className="grid md:grid-cols-2 gap-8">
        <section>
          <h3 className="text-lg font-medium mb-3">By hour of day</h3>
          <div className="h-48">
            <ResponsiveContainer><BarChart data={hourData}>
              <XAxis dataKey="hour" /><YAxis /><Tooltip />
              <Bar dataKey="count" fill="#10b981" />
            </BarChart></ResponsiveContainer>
          </div>
        </section>
        <section>
          <h3 className="text-lg font-medium mb-3">By weekday</h3>
          <div className="h-48">
            <ResponsiveContainer><BarChart data={weekData}>
              <XAxis dataKey="day" /><YAxis /><Tooltip />
              <Bar dataKey="count" fill="#3b82f6" />
            </BarChart></ResponsiveContainer>
          </div>
        </section>
      </div>

      <div className="grid md:grid-cols-2 gap-8">
        <section>
          <h3 className="text-lg font-medium mb-3">Top 10 things you keep saying</h3>
          <PhrasesList phrases={s.top_phrases} />
        </section>
        <section>
          <h3 className="text-lg font-medium mb-3">Top topics</h3>
          <PhrasesList phrases={s.top_topics} />
        </section>
      </div>

      <section>
        <h3 className="text-lg font-medium mb-3">By model</h3>
        <table className="text-sm">
          <tbody>
            {s.by_model.map(([m, n]) => (
              <tr key={m}><td className="pr-6 font-mono">{m}</td><td>{fmtNum(n)} convs</td></tr>
            ))}
          </tbody>
        </table>
      </section>
    </div>
  )
}
```

- [ ] **Step 6: Verify**

Run dev → "/stats". Expected: 4 cards, heatmap, hour/weekday charts, top phrases (incl. Chinese), top topics, model breakdown.

- [ ] **Step 7: Commit**

```bash
git add src/pages/Stats.tsx src/components/ package.json package-lock.json
git commit -m "feat(ui): stats dashboard"
```

---

## Task 17: Build & package (.dmg + .exe)

**Files:**
- Modify: `src-tauri/tauri.conf.json` (bundle targets, icons)
- Create: `docs/install.md`
- Modify: `README.md`

- [ ] **Step 1: Add icons**

Generate icons from a 1024×1024 PNG (any minimal Memex logo):
```bash
npx @tauri-apps/cli icon path/to/memex-icon.png
```
This populates `src-tauri/icons/`.

- [ ] **Step 2: Configure bundle targets in `tauri.conf.json`**

```json
"bundle": {
  "active": true,
  "targets": ["dmg", "nsis", "deb", "appimage"],
  "identifier": "com.memex.app",
  "icon": [
    "icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png",
    "icons/icon.icns", "icons/icon.ico"
  ],
  "category": "Productivity"
}
```

- [ ] **Step 3: Build a Mac DMG**

```bash
npm run tauri build
```

Verify: `src-tauri/target/release/bundle/dmg/Memex_0.1.0_aarch64.dmg` exists. Open it; drag Memex into Applications; launch (Right-click → Open the first time to bypass Gatekeeper).

- [ ] **Step 4: Document install/Gatekeeper bypass**

`docs/install.md`:

```markdown
# Installing Memex

## macOS
1. Download `Memex_0.1.0_<arch>.dmg` from Releases.
2. Open it, drag Memex into Applications.
3. **First launch:** Right-click Memex.app → "Open" → confirm.
   (macOS blocks unsigned apps on first launch only. After that, double-click works normally.)

If you see "app is damaged":
\`\`\`
xattr -dr com.apple.quarantine /Applications/Memex.app
\`\`\`

## Windows
1. Download `Memex_0.1.0_x64-setup.exe`.
2. Windows SmartScreen will warn. Click "More info" → "Run anyway".

## Verify the source
Memex is open source. To verify the binary matches:
\`\`\`
git clone <repo>
cd Memex && npm install && npm run tauri build
\`\`\`
```

Update `README.md` install section to point at `docs/install.md`.

- [ ] **Step 5: End-to-end smoke test**

Launch the installed `.app`. Import the user's real export folder. Verify:
- Library shows ~50+ conversations with correct titles
- Click a convo → renders correctly
- Stats page populates all sections
- Top phrases include realistic items (mix of EN + ZH)
- "Busiest day" matches what you'd expect

- [ ] **Step 6: Commit**

```bash
git add src-tauri/tauri.conf.json src-tauri/icons docs/install.md README.md
git commit -m "chore: configure bundle + install docs"
```

- [ ] **Step 7: Tag release**

```bash
git tag -a v0.1.0 -m "Memex v0.1.0 — ChatGPT viewer + stats"
```

(Push & GitHub Release in a follow-up — out of scope for the plan.)

---

## Done criteria for Phase 1

- [ ] All 17 tasks committed
- [ ] `cargo test` clean
- [ ] `.dmg` file builds and runs on a clean Mac
- [ ] User can: import → browse → see stats including the specific items they asked for (totals, est. cost, busiest day, top 10 phrases, top topics)
- [ ] Plan for Phase 2 (Claude Code source) drafted as a separate file
