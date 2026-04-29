import { useEffect, useState } from "react"
import { Link } from "react-router-dom"
import { api } from "@/lib/api"
import type { StatsReport } from "@/types"
import { StatCard } from "@/components/StatCard"
import { ActivityHeatmap } from "@/components/ActivityHeatmap"
import { PhrasesList } from "@/components/PhrasesList"
import { fmtDate, fmtNum, fmtUsd, sourceLabel } from "@/lib/format"
import {
  BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer,
} from "recharts"

const WEEKDAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]

export default function Stats() {
  const [s, setS] = useState<StatsReport | null>(null)
  const [loading, setLoading] = useState(true)

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
      <h2 className="text-2xl font-semibold tracking-tight">Stats</h2>

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

      <div className="grid md:grid-cols-2 gap-10">
        <section>
          <h3 className="text-base font-semibold mb-3">
            Top 10 things you keep saying
          </h3>
          <PhrasesList phrases={s.top_phrases} />
        </section>
        <section>
          <h3 className="text-base font-semibold mb-3">Top topics</h3>
          <PhrasesList phrases={s.top_topics} />
        </section>
      </div>

      <div className="grid md:grid-cols-2 gap-10">
        {s.by_source.length > 0 && (
          <section>
            <h3 className="text-base font-semibold mb-3">By source</h3>
            <SimpleTable rows={s.by_source.map(([k, n]) => [sourceLabel(k), fmtNum(n)])} />
          </section>
        )}
        {s.by_model.length > 0 && (
          <section>
            <h3 className="text-base font-semibold mb-3">By model</h3>
            <SimpleTable
              rows={s.by_model.map(([k, n]) => [k || "(unknown)", fmtNum(n)])}
              monoFirstCol
            />
          </section>
        )}
      </div>
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

