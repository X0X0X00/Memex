# Memex

> Browse and analyze your AI conversation history from ChatGPT and Claude.ai — **100% local, zero network calls.**

A Tauri desktop app that ingests your data export, renders it like a modern chat UI, and shows you stats about how you actually use AI.

**Status:** v0.1.0 — Phase 1 (ChatGPT + Claude.ai web exports + stats). Roadmap below.

---

## Why

OpenAI's data export now ships as plain `conversations.json` (no more `chat.html`), and Claude.ai never had a viewer in the first place. Both leave you holding a multi-megabyte JSON blob that's effectively unreadable. Memex turns that blob back into something you can actually use.

It also answers the questions you actually want answered: *how many conversations have I had, how many tokens, what days do I chat the most, what do I keep saying?*

## Features

- **Auto-detects** ChatGPT export (mapping tree) vs Claude.ai web export (`chat_messages` array) — drop a folder, Memex figures out the format.
- **Conversation viewer** — markdown rendering, code blocks with Shiki syntax highlighting, role-coloured bubbles, per-message token counts.
- **Library** — searchable list with source/model badges, sort by date.
- **Stats dashboard:**
  - Top-line: total conversations, messages, tokens (in/out/cache split), estimated cost.
  - 365-day activity heatmap with the busiest day called out.
  - Hour-of-day and weekday distributions.
  - **Top 10 phrases you keep saying** — n-gram extraction with bilingual support (English regex + Chinese jieba segmentation, stopword-filtered).
  - **Top topics** — TF-IDF over conversation titles + first user message.
  - Source breakdown (ChatGPT vs Claude.ai vs Claude Code) and model breakdown.

## Privacy

- 100% local — Memex makes **zero network requests**. Your data never leaves your machine.
- Stored in a single SQLite file inside the OS app data dir.
- Open source — auditable end-to-end. Build from source if you want to verify.

## Install

See [docs/install.md](docs/install.md) for platform-specific instructions and how to bypass first-launch warnings (Memex is unsigned for v0.1).

Currently shipping macOS Apple Silicon as a `.zip` of the `.app` bundle (DMG packaging hits a known Tauri bug with paths containing spaces). Windows / Intel Mac / Linux builds in v0.2.

## How to use

1. Get your data:
   - **ChatGPT:** chatgpt.com → Settings → Data controls → Export. Email + zip.
   - **Claude.ai:** claude.ai → Settings → Privacy → Export. Email + zip.
2. Unzip somewhere.
3. Open Memex → Import → pick the unzipped folder.
4. Browse in the Library, see your stats in the Stats tab.

## Develop

```sh
git clone git@github.com:X0X0X00/Memex.git
cd Memex
npm install
npm run tauri dev      # dev (hot reload)
npm run tauri build    # release bundle
```

Requirements: Node 20+, Rust stable (`rustup default stable`), Xcode CLT on macOS.

### Project layout

```
Memex/
├── src/                       # React frontend
│   ├── pages/                 # Onboarding, Library, Conversation, Stats
│   ├── components/            # Markdown viewer, charts, cards
│   ├── lib/                   # api.ts (typed Tauri commands), format.ts
│   └── types.ts               # mirrors Rust schema
├── src-tauri/src/             # Rust backend
│   ├── schema.rs              # Conversation / Message / Source / Role
│   ├── db.rs                  # SQLite + helpers
│   ├── parser/
│   │   ├── openai.rs          # ChatGPT mapping-tree parser
│   │   ├── claude_web.rs      # Claude.ai chat_messages parser
│   │   └── tokens.rs          # tiktoken wrapper
│   ├── stats/
│   │   ├── activity.rs        # heatmap, busiest day
│   │   ├── ngrams.rs          # top phrases (EN + jieba ZH)
│   │   ├── topics.rs          # TF-IDF
│   │   └── cost.rs            # model→price table
│   └── commands.rs            # Tauri command surface
└── docs/superpowers/
    ├── specs/                 # design docs
    └── plans/                 # implementation plans
```

### Architecture in one paragraph

Rust backend parses exports into a unified `(Conversation, Vec<Message>)` schema, persists to SQLite, and exposes Tauri commands (`import_export`, `list_conversations`, `get_conversation`, `get_stats`). React frontend reads via `invoke()`, renders with Tailwind + shadcn primitives. All processing happens in-process; no separate service.

## Roadmap

- **v0.2** — Claude Code source (`~/.claude/projects/*.jsonl` with real token + model data), better top-phrase quality (template-pollution filter), per-model cost detection. See [`docs/superpowers/plans/2026-04-29-memex-phase-2-claude-code-source.md`](docs/superpowers/plans/2026-04-29-memex-phase-2-claude-code-source.md).
- **v0.3** — Tantivy full-text search across all messages, "Export Report" → static HTML.
- **v0.4** — Code signing, GitHub Releases auto-publish, Win + Linux + Intel Mac builds, auto-updater.

## License

MIT

## Acknowledgements

Built on [Tauri 2](https://tauri.app), [React](https://react.dev), [tiktoken-rs](https://github.com/zurawiki/tiktoken-rs), [jieba-rs](https://github.com/messense/jieba-rs), [Recharts](https://recharts.org), [Shiki](https://shiki.matsu.io). Memex (the name) is a tribute to Vannevar Bush's 1945 essay [*As We May Think*](https://www.theatlantic.com/magazine/archive/1945/07/as-we-may-think/303881/).
