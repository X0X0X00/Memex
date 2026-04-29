import { Bar, BarChart, Cell, ResponsiveContainer, Tooltip, XAxis, YAxis, LabelList } from "recharts"

const PALETTE = [
  "#10b981", "#06b6d4", "#3b82f6", "#a855f7", "#f59e0b",
  "#22c55e", "#64748b", "#ec4899", "#14b8a6", "#f97316",
]

export function ToolUsageBars({ rows }: { rows: [string, number][] }) {
  if (rows.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        No tool usage recorded yet. Re-import Claude Code to populate this chart
        — the v0.3 importer records each tool call separately for accurate counts.
      </p>
    )
  }
  const data = rows.slice(0, 10).map(([name, n]) => ({ name, n }))
  return (
    <div style={{ height: data.length * 28 + 32 }}>
      <ResponsiveContainer>
        <BarChart layout="vertical" data={data} margin={{ top: 4, right: 48, left: 8, bottom: 4 }}>
          <XAxis type="number" tick={{ fontSize: 11 }} />
          <YAxis dataKey="name" type="category" tick={{ fontSize: 12 }} width={110} />
          <Tooltip contentStyle={{ fontSize: 12 }} />
          <Bar dataKey="n" radius={[0, 4, 4, 0]}>
            {data.map((_, i) => <Cell key={i} fill={PALETTE[i % PALETTE.length]} />)}
            <LabelList dataKey="n" position="right" style={{ fontSize: 11, fill: "hsl(var(--muted-foreground))" }} />
          </Bar>
        </BarChart>
      </ResponsiveContainer>
    </div>
  )
}
