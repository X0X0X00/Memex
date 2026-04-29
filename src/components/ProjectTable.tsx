import { useState } from "react"
import type { ProjectRow } from "@/types"
import { fmtDate, fmtNum, fmtTokens, fmtUsd } from "@/lib/format"

type SortKey = "cost" | "msgs" | "convs" | "tokens" | "last"

export function ProjectTable({ rows }: { rows: ProjectRow[] }) {
  const [showAll, setShowAll] = useState(false)
  const [sort, setSort] = useState<SortKey>("cost")

  if (rows.length === 0) return null

  const sorted = [...rows].sort((a, b) => {
    switch (sort) {
      case "cost":   return b.cost_usd - a.cost_usd
      case "msgs":   return b.msg_count - a.msg_count
      case "convs":  return b.conv_count - a.conv_count
      case "tokens": return b.token_count - a.token_count
      case "last":   return b.last_at - a.last_at
    }
  })
  const visible = showAll ? sorted : sorted.slice(0, 10)
  const remaining = sorted.length - visible.length

  return (
    <div className="space-y-2">
      <table className="text-sm w-full">
        <thead>
          <tr className="text-xs uppercase tracking-wider text-muted-foreground">
            <Th>Project</Th>
            <Th onClick={() => setSort("convs")} active={sort === "convs"} align="right">Convs</Th>
            <Th onClick={() => setSort("msgs")} active={sort === "msgs"} align="right">Msgs</Th>
            <Th onClick={() => setSort("tokens")} active={sort === "tokens"} align="right">Tokens</Th>
            <Th onClick={() => setSort("cost")} active={sort === "cost"} align="right">Cost</Th>
            <Th onClick={() => setSort("last")} active={sort === "last"} align="right">Last activity</Th>
          </tr>
        </thead>
        <tbody>
          {visible.map((r) => (
            <tr key={r.project} className="border-b border-border/40">
              <td className="py-1.5 pr-4 truncate max-w-xs" title={r.project}>
                {r.display_name}
              </td>
              <td className="py-1.5 text-right tabular-nums">{fmtNum(r.conv_count)}</td>
              <td className="py-1.5 text-right tabular-nums">{fmtNum(r.msg_count)}</td>
              <td className="py-1.5 text-right tabular-nums">{fmtTokens(r.token_count)}</td>
              <td className="py-1.5 text-right tabular-nums">{fmtUsd(r.cost_usd)}</td>
              <td className="py-1.5 text-right tabular-nums text-muted-foreground">
                {r.last_at > 0 ? fmtDate(r.last_at) : "—"}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {remaining > 0 && (
        <button
          onClick={() => setShowAll(true)}
          className="text-xs text-muted-foreground hover:text-foreground"
        >
          Show {remaining} more project{remaining > 1 ? "s" : ""} ↓
        </button>
      )}
    </div>
  )
}

function Th({
  children,
  onClick,
  active,
  align,
}: {
  children: React.ReactNode
  onClick?: () => void
  active?: boolean
  align?: "right"
}) {
  const base = "py-2 pr-4 font-medium" + (align === "right" ? " text-right" : "")
  if (!onClick) return <th className={base}>{children}</th>
  return (
    <th className={base + " cursor-pointer select-none"} onClick={onClick}>
      <span className={active ? "text-foreground" : ""}>{children}</span>
    </th>
  )
}
