<div align="center">
  <img src="assets/logo.svg" alt="Memex" width="120" height="120"/>

  <h1>Memex</h1>

  <p><strong>让你的 AI 对话历史重新可读。</strong></p>
  <p>浏览和分析你的 ChatGPT、Claude.ai、Claude Code 对话记录 —— <em>100% 本地，零网络请求。</em></p>

  <p>
    <a href="https://github.com/X0X0X00/Memex/stargazers"><img src="https://img.shields.io/github/stars/X0X0X00/Memex?style=flat-square&color=f59e0b" alt="Stars"/></a>
    <a href="https://github.com/X0X0X00/Memex/releases"><img src="https://img.shields.io/github/v/release/X0X0X00/Memex?style=flat-square&color=10b981" alt="Release"/></a>
    <a href="LICENSE"><img src="https://img.shields.io/github/license/X0X0X00/Memex?style=flat-square&color=06b6d4" alt="License"/></a>
    <a href="https://github.com/X0X0X00/Memex/commits/main"><img src="https://img.shields.io/github/commit-activity/m/X0X0X00/Memex?style=flat-square&color=8b5cf6" alt="Commits"/></a>
  </p>

  <p>
    <a href="README.md">English</a> · 简体中文
  </p>
</div>

---

## 为什么做这个

OpenAI 现在的数据导出只有一个 `conversations.json`（不再有 `chat.html`）。Claude.ai 从来就没给过 viewer。两边都让你拿到一坨几十兆的 JSON，根本没法读。

Memex 把这坨 JSON 变回你能用的东西 —— 顺带回答你真正想知道的问题：*我聊了多少次？花了多少 token？哪天聊得最多？我反复在说什么？*

## 功能

- **🔌 三种数据源，自动识别**
  - **Claude Code** —— 一键读 `~/.claude/projects/`。token 数和模型从 API response 直接拿，成本是真实数字而不是估算。
  - **ChatGPT 导出** —— 现代 mapping-tree 格式。
  - **Claude.ai 网页导出** —— `chat_messages` 数组格式。
- **📖 对话查看器** —— Markdown 渲染、Shiki 代码高亮、按角色着色的气泡、每条消息的 token 数。
- **🔍 资料库** —— 可搜索、按数据源/模型筛选。
- **📊 统计面板**
  - 总对话数 / 消息数 / token / 估算成本
  - 365 天活跃热力图，标出最忙的一天
  - 按小时和按星期分布（**本地时区**）
  - **你最常说的 10 句话** —— 双语支持（英文 regex + 中文 jieba 分词）
  - **热门话题** —— TF-IDF，对高频词做了抑制
  - 按数据源、按模型分组
- **🔒 隐私优先** —— Memex **不发任何网络请求**。开源、可审计、MIT 协议。

## 快速开始

### 安装

**macOS**（Apple Silicon）

从 [Releases](https://github.com/X0X0X00/Memex/releases) 下载 `Memex_X.Y.Z_aarch64.dmg` → 拖到 Applications。

App 没签名，第一次打开要右键 → **打开** → 在弹窗里确认。详见 [docs/install.md](docs/install.md)。

### 拿到你的数据

| 数据源 | 在哪儿 |
|---|---|
| **Claude Code** | 已经在你硬盘上了 (`~/.claude/projects/`) —— Memex 自动读。 |
| **ChatGPT** | chatgpt.com → Settings → Data controls → **Export**。邮件里发 zip。 |
| **Claude.ai** | claude.ai → Settings → Privacy → **Export**。邮件里发 zip。 |

### 使用

1. 打开 Memex。
2. 进 **Import**，点 **Import Claude Code** 或者 **Choose folder**（选网页导出的文件夹）。
3. 在 **Library** 浏览，在 **Stats** 看统计。

## 隐私

Memex **不发任何网络请求**。

- 解析全在本地完成。
- 数据存在系统 app-data 目录下的一个 SQLite 文件里。
- 没有遥测、没有埋点、没有自动更新检查。
- 开源 —— 自己看代码或者从源码 build 来验证。

这不是营销话术，是硬性约定。代码里没有 `reqwest`、没有 `fetch()`、没有连服务器的 IPC。

## 统计示例

```
对话数            消息数            Token              估算成本
221               1,758             665,392            $8.47
2025-09 → 2026-04                   入 125k · 出 540k  API 等价价

活动 ─────────────────────────────────────────────────────────────
最忙的一天: 2025-11-14 (175 条消息). 高峰时段: 16:00 (160 条).
[365 天热力图]

最常说的 10 句话                热门话题
1.  doesn't work                  1.  rust ownership
2.  我不明白                       2.  python 装饰器
3.  fix this                      3.  tauri commands
...                                ...
```

## 开发

需要 Node 20+、Rust stable（`rustup default stable`）、macOS 上还需要 Xcode CLT。

```sh
git clone git@github.com:X0X0X00/Memex.git
cd Memex
npm install
npm run tauri dev      # 开发模式（热重载）
npm run tauri build    # 打包 release（macOS 上是 .dmg）
```

### 项目结构

```
Memex/
├── src/                    # React 前端
│   ├── pages/              # Onboarding, Library, Conversation, Stats
│   ├── components/         # Markdown 渲染、图表、卡片
│   └── lib/                # 类型化 Tauri 命令、格式化
├── src-tauri/src/          # Rust 后端
│   ├── schema.rs           # Conversation / Message / Source / Role
│   ├── db.rs               # SQLite schema + 查询
│   ├── parser/
│   │   ├── openai.rs       # ChatGPT mapping-tree
│   │   ├── claude_web.rs   # Claude.ai chat_messages
│   │   ├── claude_code.rs  # Claude Code .jsonl
│   │   └── tokens.rs       # tiktoken 封装
│   ├── stats/
│   │   ├── activity.rs     # 热力图、最忙日（本地时区）
│   │   ├── ngrams.rs       # top phrases（中英 + jieba，剔除粘贴的代码）
│   │   ├── topics.rs       # TF-IDF（DF 上限、sublinear TF）
│   │   └── cost.rs         # 模型→定价表
│   └── commands.rs         # Tauri command 接口
└── docs/superpowers/       # 设计文档 + 实现计划
```

### 架构

Rust 后端把导出文件解析成统一 schema `(Conversation, Vec<Message>)`，存到 SQLite，通过 Tauri command 暴露（`import_export`、`import_claude_code`、`list_conversations`、`get_conversation`、`get_stats`）。React 前端用 `invoke()` 调用，Tailwind + shadcn 渲染。所有处理都在进程内完成，不需要单独的服务。

## Roadmap

- ✅ **v0.1** —— ChatGPT + Claude.ai 网页导出 + 统计。
- ✅ **v0.2** —— Claude Code 数据源，精确 token，逐消息成本。
- ✅ **v0.2.1** —— Top phrase / topic 噪声过滤。
- ✅ **v0.2.2** —— 本地时区活动图、n-gram 剔除粘贴代码。
- 🔜 **v0.3** —— Tantivy 全文搜索、"Export Report" → 静态 HTML、可选用 LLM 做话题分析（自带 API key 或本地 Ollama）。
- 🔜 **v0.4** —— 代码签名、GitHub Releases 自动发布、Windows / Linux / Intel Mac 包、自动更新。

## 贡献

欢迎 PR —— issue、bug 报告、其他 AI 工具的格式支持（Gemini 导出？Cursor？Cline？）。开 PR 前请跑过 `cargo test` 和 `npm run build`。

## License

MIT。随便用、随便 fork、随便 ship。

## 致谢

基于 [Tauri 2](https://tauri.app)、[React](https://react.dev)、[tiktoken-rs](https://github.com/zurawiki/tiktoken-rs)、[jieba-rs](https://github.com/messense/jieba-rs)、[Recharts](https://recharts.org)、[Shiki](https://shiki.style) 构建。

*Memex* 这个名字致敬 Vannevar Bush 1945 年的文章 [*As We May Think*](https://www.theatlantic.com/magazine/archive/1945/07/as-we-may-think/303881/)，他当时设想了一种"个人记忆延伸装置"，可以存下你读过的所有东西并互相交叉链接。这是朝那个方向走的一小步。
