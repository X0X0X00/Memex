import { useEffect, useMemo, useState } from "react"
import { Link } from "react-router-dom"
import { api } from "@/lib/api"
import type { ConversationSummary } from "@/types"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"
import { fmtDate, fmtNum, fmtUsd, sourceLabel } from "@/lib/format"

type SourceFilter = "all" | "openai" | "claude_web" | "claude_code"

export default function Library() {
  const [items, setItems] = useState<ConversationSummary[]>([])
  const [q, setQ] = useState("")
  const [src, setSrc] = useState<SourceFilter>("all")
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    api.listConversations()
      .then(setItems)
      .finally(() => setLoading(false))
  }, [])

  const filtered = useMemo(() => {
    const needle = q.trim().toLowerCase()
    return items.filter(c => {
      if (src !== "all" && c.source !== src) return false
      if (needle && !c.title.toLowerCase().includes(needle)) return false
      return true
    })
  }, [items, q, src])

  const sourcesPresent = useMemo(() => {
    const set = new Set<string>()
    items.forEach(c => set.add(c.source))
    return set
  }, [items])

  if (loading) {
    return <div className="p-8 text-muted-foreground">Loading…</div>
  }
  if (items.length === 0) {
    return (
      <div className="p-12 text-muted-foreground">
        No conversations yet.{" "}
        <Link to="/import" className="underline hover:no-underline text-foreground">
          Import an export
        </Link>{" "}
        to get started.
      </div>
    )
  }

  return (
    <div className="p-8 space-y-4 max-w-5xl">
      <div className="flex items-center justify-between flex-wrap gap-3">
        <h2 className="text-2xl font-semibold tracking-tight">Library</h2>
        <Input
          placeholder="Search titles…"
          value={q}
          onChange={(e) => setQ(e.target.value)}
          className="w-64"
        />
      </div>

      {sourcesPresent.size > 1 && (
        <div className="flex gap-2 text-xs">
          {(["all", ...Array.from(sourcesPresent)] as SourceFilter[]).map(s => (
            <button
              key={s}
              onClick={() => setSrc(s)}
              className={
                "px-2.5 py-1 rounded-md border transition-colors " +
                (src === s
                  ? "bg-primary text-primary-foreground border-primary"
                  : "border-border text-muted-foreground hover:bg-accent")
              }
            >
              {s === "all" ? "All" : sourceLabel(s)}
            </button>
          ))}
        </div>
      )}

      <div className="text-xs text-muted-foreground">
        {filtered.length} / {items.length} conversations
      </div>

      <div className="border border-border rounded-md divide-y divide-border">
        {filtered.map(c => (
          <Link
            key={c.id}
            to={`/conversation/${encodeURIComponent(c.id)}`}
            className="block px-4 py-3 hover:bg-accent/50 transition-colors"
          >
            <div className="flex items-center justify-between gap-4">
              <div className="min-w-0 flex-1">
                <div className="truncate font-medium text-sm">
                  {c.title || "(untitled)"}
                </div>
                <div className="text-xs text-muted-foreground mt-1 flex flex-wrap gap-x-3">
                  <span>{fmtDate(c.created_at)}</span>
                  <span>{fmtNum(c.message_count)} msgs</span>
                  <span>{fmtNum(c.tokens_total)} tokens</span>
                  {c.estimated_cost_usd > 0 && <span>~{fmtUsd(c.estimated_cost_usd)}</span>}
                </div>
              </div>
              <div className="flex items-center gap-2 shrink-0">
                {c.model && (
                  <Badge variant="secondary" className="font-mono text-[10px]">
                    {c.model}
                  </Badge>
                )}
                <Badge variant="outline" className="text-[10px]">
                  {sourceLabel(c.source)}
                </Badge>
              </div>
            </div>
          </Link>
        ))}
      </div>
    </div>
  )
}
