import { Area, AreaChart, ResponsiveContainer, Tooltip, XAxis, YAxis, Legend } from "recharts"
import type { CostSeries } from "@/types"
import { fmtUsd, modelGroupColor } from "@/lib/format"

export function CostOverTimeChart({ series }: { series: CostSeries }) {
  if (series.points.length === 0) {
    return <p className="text-sm text-muted-foreground">No cost data yet.</p>
  }

  // Collect every model group that ever appears.
  const groupSet = new Set<string>()
  for (const p of series.points) {
    for (const [g] of p.by_model_group) groupSet.add(g)
  }
  const groups = Array.from(groupSet).sort()

  // Pivot into Recharts row shape: { bucket_label, "Claude Opus": 12.34, ... }
  const data = series.points.map(p => {
    const row: Record<string, string | number> = { bucket_label: p.bucket_label }
    for (const g of groups) row[g] = 0
    for (const [g, v] of p.by_model_group) row[g] = v
    return row
  })

  return (
    <div className="h-72">
      <ResponsiveContainer>
        <AreaChart data={data} margin={{ top: 8, right: 16, left: 8, bottom: 8 }}>
          <XAxis dataKey="bucket_label" tick={{ fontSize: 11 }} />
          <YAxis
            tick={{ fontSize: 11 }}
            tickFormatter={(v) => fmtUsd(Number(v))}
            width={70}
          />
          <Tooltip
            contentStyle={{ fontSize: 12 }}
            formatter={(v) => fmtUsd(Number(v))}
          />
          <Legend wrapperStyle={{ fontSize: 11 }} />
          {groups.map((g) => (
            <Area
              key={g}
              type="monotone"
              dataKey={g}
              stackId="1"
              stroke={modelGroupColor(g)}
              fill={modelGroupColor(g)}
              fillOpacity={0.85}
            />
          ))}
        </AreaChart>
      </ResponsiveContainer>
    </div>
  )
}
