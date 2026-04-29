import { useEffect, useState } from "react"
import { Link } from "react-router-dom"
import { api } from "@/lib/api"
import type { StatsReport } from "@/types"
import { StatCard } from "@/components/StatCard"
import { ActivityHeatmap } from "@/components/ActivityHeatmap"
import { CostOverTimeChart } from "@/components/CostOverTimeChart"
import { ProjectTable } from "@/components/ProjectTable"
import { ToolUsageBars } from "@/components/ToolUsageBars"
import { LengthHistogram } from "@/components/LengthHistogram"
import { ExportPosterDialog } from "@/components/ExportPosterDialog"
import { Button } from "@/components/ui/button"
import { fmtDate, fmtNum, fmtUsd, sourceLabel } from "@/lib/format"
import {
  BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer,
} from "recharts"

const WEEKDAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]

export default function Stats() {
  const [s, setS] = useState<StatsReport | null>(null)
  const [loading, setLoading] = useState(true)
  const [exportOpen, setExportOpen] = useState(false)

  useEffect(() => {
    api.getStats().then(setS).finally(() => setLoading(false))
  }, [])

  if (loading) return <div className="p-8 text-muted-foreground">Loading…</div>
  if (!s) return <div className="p-8 text-muted-foreground">No data.</div>

  if (s.total_conversations === 0) {
    return (
      <div className="p-12 text-muted-foreground">
        No conversations yet.{" "}
        <Link to="/import" className="underline text-foreground">
          Import an export
        </Link>{" "}
        to see stats.
      </div>
    )
  }

  const totalTok =
    s.total_tokens.input + s.total_tokens.output +
    s.total_tokens.cache_read + s.total_tokens.cache_write
  const span = (s.first_at && s.last_at)
    ? `${fmtDate(s.first_at)} – ${fmtDate(s.last_at)}`
    : "—"

  const hourData = s.activity.by_hour.map((v, i) => ({ hour: i, count: v }))
  const weekData = s.activity.by_weekday.map((v, i) => ({ day: WEEKDAYS[i], count: v }))
  const peakHour = hourData.reduce((a, b) => (b.count > a.count ? b : a), { hour: 0, count: 0 })

  return (
    <div className="p-8 space-y-10 max-w-6xl pb-20">
      <div className="flex items-baseline justify-between flex-wrap gap-3">
        <h2 className="text-2xl font-semibold tracking-tight">Stats</h2>
        <Button variant="secondary" size="sm" onClick={() => setExportOpen(true)}>
          Export poster
        </Button>
      </div>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <StatCard label="Conversations" value={fmtNum(s.total_conversations)} sub={span} />
        <StatCard label="Messages"      value={fmtNum(s.total_messages)} />
        <StatCard
          label="Tokens"
          value={fmtNum(totalTok)}
          sub={`in ${fmtNum(s.total_tokens.input)} · out ${fmtNum(s.total_tokens.output)}`}
        />
        <StatCard
          label="Estimated cost"
          value={fmtUsd(s.estimated_cost_usd)}
          sub="API-equivalent · not subscription"
        />
      </div>

      <p className="text-xs text-muted-foreground -mt-6 max-w-3xl leading-relaxed">
        <strong className="text-foreground">Note on cost:</strong> this is the{" "}
        <em>API-equivalent</em> estimate — what these tokens would cost via the
        Anthropic / OpenAI APIs at current per-token rates, summed per-message
        against the model that handled it. It is <em>not</em> what you actually
        paid: ChatGPT Plus / Claude Pro subscriptions are flat-rate and don't
        map to this number. Token counts for <strong>Claude Code</strong> sessions
        are exact (from the API response). Token counts for <strong>web exports</strong>{" "}
        (ChatGPT export, Claude.ai export) are estimated via tiktoken cl100k —
        approximate, especially for non-English text.
      </p>

      <section>
        <h3 className="text-base font-semibold mb-1">Activity</h3>
        {s.activity.busiest_day && (
          <p className="text-sm text-muted-foreground mb-4">
            Busiest day:{" "}
            <span className="text-foreground font-medium">
              {s.activity.busiest_day.date}
            </span>{" "}
            ({s.activity.busiest_day.messages} messages).
            Peak hour: {peakHour.hour}:00 ({peakHour.count} msgs).
          </p>
        )}
        <div className="overflow-x-auto pb-2">
          <ActivityHeatmap daily={s.activity.daily} />
        </div>
      </section>

      <div className="grid md:grid-cols-2 gap-8">
        <section>
          <h3 className="text-base font-semibold mb-3">By hour of day</h3>
          <div className="h-48">
            <ResponsiveContainer>
              <BarChart data={hourData}>
                <XAxis dataKey="hour" tick={{ fontSize: 11 }} />
                <YAxis tick={{ fontSize: 11 }} />
                <Tooltip contentStyle={{ fontSize: 12 }} />
                <Bar dataKey="count" fill="#10b981" radius={[2, 2, 0, 0]} />
              </BarChart>
            </ResponsiveContainer>
          </div>
        </section>
        <section>
          <h3 className="text-base font-semibold mb-3">By weekday</h3>
          <div className="h-48">
            <ResponsiveContainer>
              <BarChart data={weekData}>
                <XAxis dataKey="day" tick={{ fontSize: 11 }} />
                <YAxis tick={{ fontSize: 11 }} />
                <Tooltip contentStyle={{ fontSize: 12 }} />
                <Bar dataKey="count" fill="#3b82f6" radius={[2, 2, 0, 0]} />
              </BarChart>
            </ResponsiveContainer>
          </div>
        </section>
      </div>

      {s.cost_over_time.points.length > 0 && (
        <section>
          <h3 className="text-base font-semibold mb-3">💰 Cost over time</h3>
          <CostOverTimeChart series={s.cost_over_time} />
        </section>
      )}

      {s.by_project.length > 0 && (
        <section>
          <h3 className="text-base font-semibold mb-1">📂 By project</h3>
          <p className="text-xs text-muted-foreground mb-3">
            Claude Code conversations grouped by working directory.
          </p>
          <ProjectTable rows={s.by_project} />
        </section>
      )}

      {s.tool_usage.length > 0 && (
        <section>
          <h3 className="text-base font-semibold mb-1">🛠 Tool usage</h3>
          <p className="text-xs text-muted-foreground mb-3">
            Top tools called across all Claude Code sessions.
          </p>
          <ToolUsageBars rows={s.tool_usage} />
        </section>
      )}

      <section>
        <h3 className="text-base font-semibold mb-1">📏 Message length</h3>
        <p className="text-xs text-muted-foreground mb-3">
          How long are your prompts? (token-count buckets)
        </p>
        <LengthHistogram buckets={s.message_length} />
      </section>

      <Breakdown
        bySource={s.by_source}
        byModel={s.by_model}
      />

      <section className="border border-dashed border-border rounded-lg p-5 max-w-3xl">
        <h3 className="text-base font-semibold mb-1">
          Top phrases & topics
        </h3>
        <p className="text-muted-foreground text-sm leading-relaxed">
          Coming in <strong className="text-foreground">v0.3</strong> with optional LLM-powered
          analysis (bring-your-own API key, or local Ollama — privacy preserved). Pure keyword
          extraction surfaces a lot of noise (pasted code, repeated context blocks), so this
          panel is paused until the LLM path lands.
        </p>
      </section>

      <ExportPosterDialog
        stats={s}
        open={exportOpen}
        onClose={() => setExportOpen(false)}
      />
    </div>
  )
}

function Breakdown({
  bySource,
  byModel,
}: {
  bySource: [string, number][]
  byModel: [string, number][]
}) {
  // Hide the entire section when there's nothing useful to show — i.e.,
  // a single source AND every conversation has unknown / single model.
  const meaningfulSource = bySource.length > 1
  const meaningfulModel =
    byModel.length > 1 || (byModel.length === 1 && byModel[0][0] !== "" && byModel[0][0] !== "(unknown)")

  if (!meaningfulSource && !meaningfulModel) {
    // Compact single-line summary instead.
    const total = bySource.reduce((acc, [, n]) => acc + n, 0)
    if (total === 0) return null
    const label = bySource[0]?.[0] ? sourceLabel(bySource[0][0]) : "imported"
    return (
      <p className="text-sm text-muted-foreground">
        {fmtNum(total)} conversations from <span className="text-foreground">{label}</span>.
      </p>
    )
  }

  return (
    <div className="grid md:grid-cols-2 gap-10">
      {meaningfulSource && (
        <section>
          <h3 className="text-base font-semibold mb-3">By source</h3>
          <SimpleTable rows={bySource.map(([k, n]) => [sourceLabel(k), fmtNum(n)])} />
        </section>
      )}
      {meaningfulModel && (
        <section>
          <h3 className="text-base font-semibold mb-3">By model</h3>
          <SimpleTable
            rows={byModel.map(([k, n]) => [k || "(unknown)", fmtNum(n)])}
            monoFirstCol
          />
        </section>
      )}
    </div>
  )
}

function SimpleTable({
  rows, monoFirstCol = false,
}: { rows: [string, string][]; monoFirstCol?: boolean }) {
  return (
    <table className="text-sm w-full">
      <tbody>
        {rows.map(([a, b], i) => (
          <tr key={i} className="border-b border-border/40">
            <td className={"py-1.5 pr-6 " + (monoFirstCol ? "font-mono text-xs" : "")}>
              {a}
            </td>
            <td className="py-1.5 text-right text-muted-foreground tabular-nums">
              {b}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  )
}

