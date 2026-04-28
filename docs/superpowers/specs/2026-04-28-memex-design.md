# Memex — Design Spec

**Date:** 2026-04-28
**Author:** zzh + Claude
**Status:** Draft (awaiting review)

---

## TL;DR

Memex is a **local-first cross-platform desktop app** (Tauri) that lets you import your AI conversation history from **ChatGPT** and **Claude Code** and provides:

1. **A web-style conversation viewer** — replaces the missing `chat.html` from new ChatGPT exports; renders Claude Code jsonl into readable threads.
2. **A stats dashboard** — answers the questions you actually want to know: how many conversations, how many tokens, how much $, busiest day, top phrases I keep saying, top topics.

Distributed as `.dmg` (macOS) and `.exe` (Windows). All data stays on the user's machine.

---

## Why this exists

| Pain | Today | Memex |
|---|---|---|
| ChatGPT export is now JSON-only (no `chat.html`) | Conversations are unreadable without writing your own parser | Drop the export → instantly browse |
| Claude Code conversations buried in `~/.claude/projects/*.jsonl` | No tool to revisit them visually | Auto-detected and rendered |
| No cross-platform analytics | "How am I really using AI?" → impossible to answer | Unified schema, side-by-side stats |
| Privacy concerns (conversations contain secrets, code, private content) | Existing analyzers ask you to upload | 100% local, open-source, auditable |

---

## Target user

Power users of Claude Code and ChatGPT who:
- Have downloaded their own data (or run cc heavily)
- Want to revisit old conversations
- Are curious about their usage patterns
- Care about not uploading conversations anywhere

---

## Core Features (MVP)

### F1. Multi-source import

| Source | Path | Detection |
|---|---|---|
| ChatGPT export | User-selected folder containing `conversations.json` | Drag-and-drop or file picker |
| Claude Code | `~/.claude/projects/**/*.jsonl` | Auto-detected; user can confirm/override |

Both parse into the **Unified Schema** (see Architecture below).

### F2. Conversation Viewer

- **Library page:** searchable, sortable list of all conversations
  - Columns: title, source (badge), model, date, message count, token count, est. cost
  - Filters: source, model, project (cc only), date range
  - Full-text search across all messages (Tantivy index, in Rust)
- **Detail page:** full conversation rendered like modern chat UIs
  - Markdown + code highlighting (Shiki)
  - Tool calls collapsed by default with expand-on-click (Claude Code)
  - Image/attachment rendering (where data is available in export)
  - Permalink-style URLs so you can bookmark a specific conversation/message

### F3. Stats Dashboard

**Top-line cards** (the user's explicit asks):
- Total conversations (split by source)
- Total messages / "rounds" (user-assistant exchanges)
- Total tokens (input / output / cache breakdown)
- Estimated cost in USD
- Date span (first → last conversation)

**Activity:**
- Daily activity heatmap (GitHub-style calendar, last 12 months)
- "Busiest day" call-out: the single day with the most messages
- Hour-of-day distribution (when do I chat most?)
- Day-of-week distribution

**Behavior — the fun part:**
- **Top 10 phrases I say** — n-gram analysis (1/2/3-gram) on user messages, with stopword filtering. Bilingual (English + Chinese). Examples expected: "我不明白", "doesn't work", "explain this", "为什么", "fix it".
- **Top topics** — TF-IDF or frequency on conversation titles + first user message of each thread. Shown as word cloud + ranked list.
- **Models pie chart** — Opus/Sonnet/Haiku/GPT-4o/etc usage breakdown
- **Per-source comparison** — side-by-side: "On ChatGPT I send X chars/msg; on Claude Code I send Y"

### F4. Export

- "Export Report" button → generates a single-file static HTML report (with embedded charts) that the user can save/share without exposing the underlying data.
- Per-conversation export to Markdown.

---

## Out of Scope (V2+)

- LLM-powered Q&A over conversations ("ask my conversations a question")
- Auto-sync / live monitoring of `~/.claude` (manual import only in v1)
- Cloud sync between devices
- Sharing/publishing conversations to the web
- Editing or deleting messages
- Importing other sources (Gemini, Grok, Cursor, etc.)

---

## Architecture

### Tech stack

| Layer | Choice | Reason |
|---|---|---|
| App shell | **Tauri 2** | Small bundle (~5MB), Rust backend for performance, real native app |
| Frontend | **React + TypeScript + Vite** | Familiar, fast iteration |
| Styling | **Tailwind + shadcn/ui** | Polished out of the box |
| Charts | **Recharts** for cards; **D3** for heatmap | Recharts simple, D3 flexible for custom |
| Backend (Rust) | Tauri commands | Heavy parsing, indexing, statistics |
| Search | **Tantivy** (Rust port of Lucene) | Fast in-process FTS, no external service |
| Tokenization | **tiktoken-rs** for OpenAI; **jieba-rs** for Chinese segmentation | Accurate token counts; CJK n-grams |
| Storage | **SQLite** via rusqlite | Single-file local DB, fast, embedded |

### Data flow

```
User imports folder
    │
    ▼
[Rust] Parser detects format (OpenAI / Claude Code)
    │
    ▼
[Rust] Normalizes → Unified Schema → SQLite
    │
    ▼
[Rust] Builds Tantivy index for FTS
    │
    ▼
[Rust] Runs statistics jobs (n-grams, heatmaps, costs)
    │
    ▼
[Frontend] Reads via Tauri commands (get_conversations, get_stats, search...)
```

### Unified Schema (simplified)

```typescript
type Source = 'openai' | 'claude_code'

interface Conversation {
  id: string                    // hash of source + native id
  source: Source
  native_id: string             // original id in source
  title: string
  created_at: timestamp
  updated_at: timestamp
  model: string | null
  project: string | null        // for cc: the project folder
  message_count: number
  token_count: { input: number; output: number; cache_read: number; cache_write: number }
  estimated_cost_usd: number
}

interface Message {
  id: string
  conversation_id: string
  role: 'user' | 'assistant' | 'system' | 'tool'
  content: string               // plain text/markdown
  raw: object                   // original JSON for fidelity
  timestamp: number | null
  model: string | null
  tokens: { input: number; output: number; cache_read?: number; cache_write?: number } | null
  tool_name: string | null      // if this is a tool call/result
}
```

### Cost estimation table

Hard-coded model → price-per-million-token table, versioned. User can override.

**Claude (accurate — jsonl has token counts):**
- Opus 4.7: $15 / $75 / $1.50 cache-read input/output/cache (per M)
- Sonnet 4.6: $3 / $15 / $0.30
- Haiku 4.5: $0.80 / $4

**OpenAI (estimated — export lacks token counts, count via tiktoken):**
- GPT-4o / 4-turbo / 4 / 3.5 — table maintained in code; surface "estimated" badge

### Bilingual top-phrase extraction

1. Concatenate all **user-role** messages
2. Branch by language (heuristic: % of CJK chars > 30% → Chinese):
   - **Chinese:** jieba segmentation → 1/2/3-gram on segments
   - **English:** lowercase + token regex → 1/2/3-gram on tokens
3. Filter stopwords (each language has its list)
4. Score by `frequency × log(1 + length)` so phrases beat single words
5. Top 10 displayed; user can switch to top 50

### File layout (project repo)

```
Memex/
├── src-tauri/                   # Rust backend
│   ├── src/
│   │   ├── parser/
│   │   │   ├── openai.rs
│   │   │   ├── claude_code.rs
│   │   │   └── unified.rs       # schema
│   │   ├── stats/
│   │   │   ├── ngrams.rs
│   │   │   ├── activity.rs
│   │   │   └── cost.rs
│   │   ├── search.rs            # Tantivy
│   │   ├── db.rs                # SQLite
│   │   └── commands.rs          # Tauri command surface
│   └── tauri.conf.json
├── src/                         # React frontend
│   ├── pages/
│   │   ├── Onboarding.tsx
│   │   ├── Library.tsx
│   │   ├── Conversation.tsx
│   │   └── Stats.tsx
│   ├── components/
│   │   ├── charts/
│   │   ├── viewer/              # message rendering
│   │   └── ...
│   └── lib/
├── docs/
│   └── superpowers/specs/
└── README.md
```

---

## Distribution

| Platform | Artifact | Signing |
|---|---|---|
| macOS | `Memex.dmg` | **Unsigned for v0** — README explains how to bypass Gatekeeper. Can buy Apple Dev cert ($99/yr) later. |
| Windows | `Memex-Setup.exe` | **Unsigned for v0** — accept SmartScreen warning. EV cert is expensive; defer. |
| Linux | `.deb` + AppImage | None needed |

Released via **GitHub Releases**. Tauri's built-in updater can be enabled in v1.x.

License: **MIT** (open-source — boosts trust for a tool that touches private data).

---

## Risks & decisions

| Risk | Mitigation |
|---|---|
| ChatGPT export schema changes | Defensive parser; version detector; clear error if unknown schema |
| Large exports (multi-GB) | Streaming JSON parser in Rust (`serde_json::StreamDeserializer`); progress UI |
| OpenAI token counts not in export | Estimate with `tiktoken-rs`; show "est." badge in UI |
| Code signing pain | Skip for v0; document workaround |
| Chinese tokenization quality | jieba-rs is well-maintained; fall back to char n-grams if it fails |
| Privacy concern from end users | Open-source the repo; never make a network request; document this prominently |

---

## Success criteria for MVP release

- [ ] Drop my (zzh's) ChatGPT export folder → see all 50+ conversations rendered correctly
- [ ] Auto-detect `~/.claude/projects/` → all sessions imported
- [ ] All five top-line cards populated correctly (counts, tokens, cost)
- [ ] Top 10 phrases shows believable Chinese + English phrases I actually use
- [ ] Heatmap shows the right "busiest day"
- [ ] Conversation viewer renders code blocks, tool calls, markdown
- [ ] Full-text search returns relevant hits in <500ms on my data
- [ ] `.dmg` file builds; double-click installs and runs on a clean Mac
