<div align="center">
  <img src="assets/logo.svg" alt="Memex" width="120" height="120"/>

  <h1>Memex</h1>

  <p><strong>Your AI conversation history, finally readable.</strong></p>
  <p>Browse and analyze your ChatGPT, Claude.ai, and Claude Code history — <em>100% local, zero network calls.</em></p>

  <p>
    <a href="https://github.com/X0X0X00/Memex/stargazers"><img src="https://img.shields.io/github/stars/X0X0X00/Memex?style=flat-square&color=f59e0b" alt="Stars"/></a>
    <a href="https://github.com/X0X0X00/Memex/releases"><img src="https://img.shields.io/github/v/release/X0X0X00/Memex?style=flat-square&color=10b981" alt="Release"/></a>
    <a href="LICENSE"><img src="https://img.shields.io/github/license/X0X0X00/Memex?style=flat-square&color=06b6d4" alt="License"/></a>
    <a href="https://github.com/X0X0X00/Memex/commits/main"><img src="https://img.shields.io/github/commit-activity/m/X0X0X00/Memex?style=flat-square&color=8b5cf6" alt="Commits"/></a>
  </p>

  <p>
    English · <a href="README_zh-CN.md">简体中文</a>
  </p>
</div>

---

## Why

OpenAI's data export now ships as a plain `conversations.json` (no more `chat.html`). Claude.ai never had a viewer in the first place. Both leave you holding a multi-megabyte JSON blob that's effectively unreadable.

Memex turns that blob back into something you can actually use — and answers the questions you actually want answered: *how many conversations, how many tokens, what days do I chat the most.*

## Features

- **🔌 Three import sources, auto-detected**
  - **Claude Code** — one-click reads `~/.claude/projects/`. Exact token counts and per-message model from the API response — costs are real, not estimated.
  - **ChatGPT export** — the modern mapping-tree format.
  - **Claude.ai web export** — the `chat_messages` array format.
- **📖 Conversation viewer** — markdown rendering, code blocks with Shiki syntax highlighting, role-coloured bubbles, per-message token counts.
- **🔍 Library** — searchable, filterable list with source/model badges.
- **📊 Stats dashboard**
  - Total conversations / messages / tokens / estimated cost (per-message model pricing)
  - 365-day activity heatmap with the busiest day called out
  - Hour-of-day and weekday distributions in your local timezone
  - Per-source and per-model breakdowns (collapse when trivial)
- **🔒 Privacy first** — Memex never makes a network request. Open-source, auditable, MIT.

> *Top phrases / topic analysis* is paused — pure keyword extraction surfaced too much noise from pasted code and annotation templates. Coming back in v0.3 with optional LLM-powered analysis (bring-your-own API key or local Ollama).

## Quick Start

### Install (macOS, Apple Silicon)

1. Download the latest `Memex_*_aarch64.dmg` from [Releases](https://github.com/X0X0X00/Memex/releases).
2. Open the DMG and drag **Memex** into Applications.
3. **First launch:** the app is unsigned — right-click `Memex.app` → **Open** → confirm.

See [docs/install.md](docs/install.md) for "app is damaged" workarounds and other platforms.

### Get your data

| Source | Where |
|---|---|
| **Claude Code** | Already on disk at `~/.claude/projects/` — Memex reads it automatically. |
| **ChatGPT** | chatgpt.com → Settings → Data controls → **Export**. Email + zip, then unzip. |
| **Claude.ai** | claude.ai → Settings → Privacy → **Export**. Same flow. |

### Use

1. Open Memex.
2. Go to **Import**:
   - Click **Import Claude Code** to pull in `~/.claude/projects/` automatically — no folder picker needed.
   - Or click **Choose folder** for a ChatGPT / Claude.ai web export.
3. Browse in **Library**, see your stats in **Stats**.

## Privacy

Memex makes **zero network requests**. Period.

- Imports parse files local to your machine.
- All data lives in a single SQLite file in your OS app-data directory.
- No telemetry, no analytics, no auto-update phone-home.
- Open source — audit the code or build from source to verify.

This is a hard line, not a marketing claim. The code has no `reqwest`, no `fetch()`, no IPC sockets to a server.

## Stats Preview

```
┌─────────────────┬─────────────────┬──────────────────┬──────────────────────┐
│ CONVERSATIONS   │ MESSAGES        │ TOKENS           │ ESTIMATED COST       │
│ 248             │ 4,206           │ 2.3M             │ $234.71              │
│ 2025-09 → 2026  │                 │ in 12k · out 805k│ API-equivalent       │
└─────────────────┴─────────────────┴──────────────────┴──────────────────────┘

Activity ─────────────────────────────────────────────────────────────────
Busiest day: 2025-11-14 (175 messages). Peak hour: 14:00 (160 msgs).

  ░░░░░░░░░░▒▒▒▓▒░░▓░░░░░░░░░░░░░  ← 365 days, darker = more
  ░░░░░░░░▒▒▓░░▒░░░░░░░░░░▒░░░░░░
  ░░░░░░░░░░░▓▒▒░░░░░░░░░░░░░░░░░

Hour of day                          Weekday
████ █▆▄▃ ▁  ▁▂▆█▆▄▃▁▁ ▁  (local)    Sun▁ Mon▆ Tue▃ Wed▃ Thu▂ Fri█ Sat▄
0   6   12   18   23

By source            By model
ChatGPT       180    claude-opus-4-7        12
Claude.ai      41    claude-sonnet-4-6      83
Claude Code    27    gpt-4o                180
                     (others)               14
```

## Develop

Requires Node 20+, Rust stable (`rustup default stable`), and Xcode CLT on macOS.

```sh
git clone git@github.com:X0X0X00/Memex.git
cd Memex
npm install
npm run tauri dev      # dev with hot reload
npm run tauri build    # release bundle (.dmg on macOS)
```

### Project layout

```
Memex/
├── src/                       # React frontend
│   ├── pages/                 # Onboarding, Library, Conversation, Stats
│   ├── components/            # Markdown viewer, charts, cards
│   └── lib/                   # Typed Tauri commands, formatters
├── src-tauri/src/             # Rust backend
│   ├── schema.rs              # Conversation / Message / Source / Role
│   ├── db.rs                  # SQLite schema + queries
│   ├── parser/
│   │   ├── openai.rs          # ChatGPT mapping-tree
│   │   ├── claude_web.rs      # Claude.ai chat_messages
│   │   ├── claude_code.rs     # Claude Code .jsonl
│   │   └── tokens.rs          # tiktoken wrapper
│   ├── stats/
│   │   ├── activity.rs        # heatmap, busiest day (local TZ)
│   │   ├── cost.rs            # model→price table, per-message billing
│   │   ├── ngrams.rs          # top phrases (UI paused — kept for v0.3)
│   │   └── topics.rs          # TF-IDF (UI paused — kept for v0.3)
│   └── commands.rs            # Tauri command surface
└── docs/superpowers/          # design specs + implementation plans
```

### Architecture

Rust backend parses exports into a unified `(Conversation, Vec<Message>)` schema, persists to SQLite, exposes Tauri commands (`import_export`, `import_claude_code`, `list_conversations`, `get_conversation`, `get_stats`). React frontend reads via `invoke()`, renders with Tailwind + shadcn primitives. All processing happens in-process — no separate service.

## Roadmap

- ✅ **v0.1** — ChatGPT + Claude.ai web exports, stats dashboard.
- ✅ **v0.2** — Claude Code source with exact tokens, per-message cost.
- ✅ **v0.2.1** — Top-phrase / topic noise filtering.
- ✅ **v0.2.2** — Local-timezone activity chart, code-block stripping in n-grams, bilingual README + logo.
- ✅ **v0.2.3** — Sticky sidebar, paused phrases/topics UI, smart breakdown collapse.
- 🔜 **v0.3** — Tantivy full-text search across all messages, "Export Report" → static HTML, optional LLM-powered topic & catch-phrase analysis (bring-your-own API key or local Ollama).
- 🔜 **v0.4** — Code signing, GitHub Releases auto-publish, Win + Linux + Intel Mac builds, auto-updater.

## Contributing

PRs welcome — issues, bug reports, support for other AI tools (Gemini export? Cursor? Cline?). Please run `cargo test` and `npm run build` before opening a PR.

## License

MIT. Use it, fork it, ship it.

## Acknowledgements

Built on [Tauri 2](https://tauri.app), [React](https://react.dev), [tiktoken-rs](https://github.com/zurawiki/tiktoken-rs), [jieba-rs](https://github.com/messense/jieba-rs), [Recharts](https://recharts.org), [Shiki](https://shiki.style).

The name *Memex* is a tribute to Vannevar Bush's 1945 essay [*As We May Think*](https://www.theatlantic.com/magazine/archive/1945/07/as-we-may-think/303881/), which imagined a personal device for storing and cross-linking everything you've ever read. This is a tiny step in that direction.
