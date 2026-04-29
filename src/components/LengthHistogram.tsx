import { Bar, BarChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts"
import type { LengthBucket } from "@/types"

export function LengthHistogram({ buckets }: { buckets: LengthBucket[] }) {
  const total = buckets.reduce((s, b) => s + b.n, 0)
  if (total === 0) return <p className="text-sm text-muted-foreground">No user messages yet.</p>
  return (
    <div className="h-48">
      <ResponsiveContainer>
        <BarChart data={buckets} margin={{ top: 4, right: 16, left: 8, bottom: 4 }}>
          <XAxis dataKey="label" tick={{ fontSize: 11 }} />
          <YAxis tick={{ fontSize: 11 }} />
          <Tooltip contentStyle={{ fontSize: 12 }} />
          <Bar dataKey="n" fill="#06b6d4" radius={[2, 2, 0, 0]} />
        </BarChart>
      </ResponsiveContainer>
    </div>
  )
}
