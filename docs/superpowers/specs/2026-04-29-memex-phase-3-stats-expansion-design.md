# Memex v0.3.0 — Stats Expansion Design Spec

**Date:** 2026-04-29
**Author:** zzh + Claude
**Status:** Draft (awaiting review)
**Phase:** v0.3.0 (first of three v0.3 sub-projects: stats expansion → full-text search → export report)

---

## TL;DR

Add four new chart sections to the existing Stats page, in this order:

1. **💰 Cost over time** — stacked area chart, monthly/weekly buckets (auto), one band per model.
2. **📂 By project** — sortable table grouped by Claude Code `cwd`. Cost-descending top 10 + collapsed remainder.
3. **🛠 Tool usage** — horizontal bar chart of Claude Code tool invocations, top 10.
4. **📏 Message length distribution** — histogram of user-message token counts, fixed buckets.

Page becomes one long scrollable view. Sticky sidebar (already done in v0.2.3) makes navigation fine.

Schema gains a `tool_calls` table so tool-usage counts are accurate (not just "first tool per assistant message"). Re-importing existing Claude Code data populates it; existing rows without it just show 0 in the new chart until a re-import.

---

## Scope

**In scope:**
- Four new sections on `Stats.tsx` page with sensible empty/loading/sparse-data states.
- New SQL/Rust queries to back the charts.
- Schema migration adding `tool_calls` table.
- Claude Code parser updated to populate `tool_calls` for every `tool_use` block (not just the first).
- Per-chart auto-bucket logic for time-series (cost over time).
- Tests for each new query and the parser change.

**Out of scope (future):**
- Full-text search (v0.3.1).
- Export Report (v0.3.2).
- LLM-powered phrase/topic analysis (deferred indefinitely; placeholder note already on Stats page).
- ChatGPT export tool detection (their export doesn't expose tool calls in a usable way).

---

## Layout (single long page)

```
┌── Overview cards (existing) ─────────────────────────────────┐
│  Conversations | Messages | Tokens | Estimated cost          │
└──────────────────────────────────────────────────────────────┘
[ note about cost being API-equivalent — existing ]

▶ Activity (existing)
   - Heatmap (existing)
   - Hour-of-day + Weekday charts (existing)

▶ 💰 Cost over time            ← NEW
   stacked area, model bands

▶ 📂 By project (Claude Code)  ← NEW
   table, sortable headers

▶ 🛠 Tool usage (Claude Code)  ← NEW
   horizontal bar chart

▶ 📏 Message length            ← NEW
   histogram

▶ Sources / models (existing trivial collapse logic — unchanged)
[ phrases/topics paused note — existing ]
```

Section ordering rationale: cost over time first (most universal), then the cc-only blocks (projects + tools, both about *what work you do*), then length (universal again, reflective).

Each new section is hidden when its data is empty:
- **Cost over time** hides if all conversations have `estimated_cost_usd = 0` (e.g. unknown models only).
- **By project** hides if no rows have a non-empty `project` column (i.e. no cc data imported).
- **Tool usage** hides if `tool_calls` table is empty.
- **Message length** never hides (always populated from any user message).

---

## Charts in detail

### 💰 Cost over time

**Data:** SQL aggregating `messages.timestamp` → bucket → SUM(per-message cost).

Per-message cost is computed on-demand (not stored) using the same `estimate_cost_usd(model, tokens)` helper, falling back to the conversation-level model if a message has no model. We compute this inline in the query rather than caching to avoid stale costs if the price table changes.

**Bucketing:** auto-detect span:
- span ≤ 60 days → daily buckets
- span ≤ 12 months → weekly buckets (ISO week start Monday)
- span > 12 months → monthly buckets (`YYYY-MM`)

**Series:** one band per model. Group similar models:
- `Claude Opus` (any `claude-opus-*`)
- `Claude Sonnet` (any `claude-sonnet-*` / `claude-3-5-sonnet*` / `claude-3-7-sonnet*`)
- `Claude Haiku` (any `claude-haiku-*` / `claude-3-5-haiku*`)
- `GPT-4o` (`gpt-4o*`)
- `GPT-4` (`gpt-4*` other than 4o)
- `Other`

**Empty cells:** the query returns one row per (bucket, model). The frontend pivots into Recharts `AreaChart` data with stacked `<Area>` per model, sharing one `<XAxis dataKey="bucket">`.

**Y axis:** USD, formatted with `$` prefix and 2 decimals.

### 📂 By project (Claude Code only)

**Data:** SQL grouping `conversations` by `project`:

```sql
SELECT project,
       COUNT(*) AS conv_count,
       SUM(message_count) AS msg_count,
       SUM(tok_input + tok_output + tok_cache_read + tok_cache_write) AS token_count,
       SUM(estimated_cost_usd) AS cost,
       MIN(created_at) AS first_at,
       MAX(updated_at) AS last_at
FROM conversations
WHERE source = 'claude_code' AND project IS NOT NULL AND project != ''
GROUP BY project
ORDER BY cost DESC
LIMIT 100;
```

**UI:** A sortable table with these columns (display only, not the raw `project` path):
- **Project** — the basename of the cwd (`/Users/zzh/Visual Studio Code/Memex` → `Memex`). Tooltip shows full path.
- **Convs** — `conv_count`
- **Msgs** — `msg_count`
- **Tokens** — `token_count`, formatted with K/M
- **Cost** — `cost` in USD
- **First → Last** — date range, e.g. `Mar 12 → Apr 28`

Default sort: Cost descending. Show top 10; if more than 10, a collapsed `Show N more projects ↓` row reveals the rest. Clicking a column header re-sorts.

### 🛠 Tool usage (Claude Code only)

**Data:** `SELECT tool_name, COUNT(*) FROM tool_calls GROUP BY tool_name ORDER BY 2 DESC LIMIT 20`.

**UI:** Recharts horizontal `BarChart` with `<YAxis dataKey="tool_name" type="category">` and a single `<Bar>`. Top 10 displayed; the chart shows actual numbers as labels at the end of each bar.

**Source:** new `tool_calls` table (see Schema Changes below). Backfilled by re-importing.

**Empty state:** if the table is empty, show a small note: *"Re-import Claude Code to populate tool stats (the v0.3 importer records each tool call separately for accurate counts)."*

### 📏 Message length distribution

**Data:** SQL bucketing user-message token counts:

```sql
SELECT
  CASE
    WHEN tok_input + tok_output < 50          THEN '0-50'
    WHEN tok_input + tok_output < 200         THEN '50-200'
    WHEN tok_input + tok_output < 1000        THEN '200-1k'
    WHEN tok_input + tok_output < 5000        THEN '1k-5k'
    ELSE '5k+'
  END AS bucket,
  COUNT(*) AS n
FROM messages
WHERE role = 'user'
GROUP BY bucket
ORDER BY MIN(tok_input + tok_output);
```

**UI:** vertical `BarChart`, X axis = bucket label, Y = message count.

Buckets are fixed (not auto-bucketed) so users with different data sizes see comparable shapes.

**Tone:** small caption above: *"How long are your prompts?"*

---

## Schema changes

### New table: `tool_calls`

```sql
CREATE TABLE IF NOT EXISTS tool_calls (
    id TEXT PRIMARY KEY,                  -- "<msg_id>:<tool_use_id>"
    message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    conversation_id TEXT NOT NULL,        -- denormalized for fast counts
    tool_name TEXT NOT NULL,
    seq INTEGER NOT NULL                  -- order within the message
);

CREATE INDEX IF NOT EXISTS idx_tool_calls_name ON tool_calls(tool_name);
CREATE INDEX IF NOT EXISTS idx_tool_calls_msg ON tool_calls(message_id);
```

The existing `messages.tool_name` column stays as-is for backwards compatibility but is no longer the source of truth for tool-usage stats.

### Migration strategy

`db.rs::open` runs `CREATE TABLE IF NOT EXISTS` so the new table is auto-created on next launch. Existing data lacks rows in this table; the user re-imports Claude Code to populate it. Re-imports are idempotent (we already use `INSERT OR REPLACE`).

If we later add a third v0.x version that needs a hard migration we'll introduce a `schema_migrations` table; for v0.3.0 the additive change doesn't justify the framework.

---

## Parser changes (`parser/claude_code.rs`)

In `extract_assistant_content`, instead of just capturing the *first* tool name, return a `Vec<ToolCall>`:

```rust
struct ToolCall { tool_name: String, tool_use_id: String }
```

Then in `build_assistant_message`, return the `Vec<ToolCall>` alongside the `Message` and `Usage`. The caller (in `parse_session_str`) writes a row per `ToolCall` into `tool_calls` (via a new `db::insert_tool_call` helper).

`messages.tool_name` is preserved as "primary tool" (the first one) for the conversation viewer's role indicator.

---

## Tauri command changes

Extend `StatsReport` with the new fields:

```rust
pub struct StatsReport {
    // ... existing fields
    pub cost_over_time: CostSeries,
    pub by_project: Vec<ProjectRow>,
    pub tool_usage: Vec<(String, i64)>,
    pub message_length_buckets: Vec<(String, i64)>,
}

pub struct CostSeries {
    pub bucket: BucketKind,             // Daily | Weekly | Monthly
    pub points: Vec<CostPoint>,         // sparse, only buckets that exist
}

pub struct CostPoint {
    pub bucket_label: String,           // "2026-04" or "2026-W17" or "2026-04-29"
    pub by_model_group: Vec<(String, f64)>,  // [("Claude Opus", 12.34), ("Claude Sonnet", 5.67), ...]
}

pub struct ProjectRow {
    pub project: String,                // full path
    pub display_name: String,           // basename
    pub conv_count: i64,
    pub msg_count: i64,
    pub token_count: i64,
    pub cost_usd: f64,
    pub first_at: i64,
    pub last_at: i64,
}
```

`get_stats` runs all four new queries and includes the data. Frontend doesn't get a separate command — single round-trip to keep things simple.

---

## Frontend changes

New components:

- `src/components/CostOverTimeChart.tsx` — wraps Recharts `AreaChart` with model-group stacking + currency formatting.
- `src/components/ProjectTable.tsx` — sortable table with show-more.
- `src/components/ToolUsageBars.tsx` — horizontal `BarChart`.
- `src/components/LengthHistogram.tsx` — vertical `BarChart`.

Each component takes its data shape directly from the new fields on `StatsReport`. Each renders nothing if its slice is empty (the parent `Stats.tsx` controls visibility based on the slice length).

`Stats.tsx` adds the four sections in the order described in the Layout section.

`types.ts` mirrors the new Rust structs.

---

## Testing

Follow existing pattern (TDD where natural):

1. **`tool_calls` parsing** — extend `claude_code_minimal_parses` test fixture to include a message with two `tool_use` blocks; assert both are returned.
2. **`tool_calls` DB insert** — round-trip test in `db.rs`.
3. **Cost-over-time bucketing** — unit test for the bucket-kind picker against three span lengths.
4. **Project aggregation** — seed two projects, two convs each, assert correct rollup.
5. **Length histogram** — seed messages of varying lengths, assert bucket counts.

Frontend: smoke render with mock data; no detailed visual tests.

---

## Risks

| Risk | Mitigation |
|---|---|
| Tool-usage count is 0 for users who haven't re-imported | Empty state explains; re-import is one click. |
| Cost over time chart is empty for unknown-model conversations | Section auto-hides when total cost is 0. |
| Long project list spam | Top 10 + show-more keeps the page tight. |
| `last_at` from `updated_at` may be 0 if parser missed it | Already happens; existing column. Format guards against `0` displayed as 1970. |

---

## Success criteria

- [ ] Re-import my real `~/.claude/projects/` and see all 4 new sections populated.
- [ ] Project table shows my Memex project at the top with a non-trivial cost.
- [ ] Tool usage chart shows Bash / Edit / Read in the top 5.
- [ ] Cost over time shows separate bands for Opus and Sonnet.
- [ ] Length histogram shows realistic distribution (not all in one bucket).
- [ ] All existing stats still work; existing tests still pass.
- [ ] DMG builds and runs.

---

## Open questions / explicit non-decisions

- **Filtering by source on the new charts:** not in v0.3.0. Cost-over-time covers all sources mixed; project/tool are cc-only by definition. Per-source filters are a v0.3.x polish if asked for.
- **Cost forecast / projection:** not doing.
- **Export individual section to CSV:** v0.3.2 (Export Report) covers this.
