import type { StatsReport } from "@/types"
import { fmtDate, fmtNum, fmtUsd, fmtTokensCompact } from "@/lib/format"
import { BrandStrip, FactRow, HeroNumber, TitleStrip, dominantAccent } from "./PosterShared"
import { PosterHeatmap } from "./PosterHeatmap"

export const PORTRAIT_W = 1080
export const PORTRAIT_H = 1920

export function PosterPortrait({
  stats,
  hideSpend,
}: {
  stats: StatsReport
  hideSpend: boolean
}) {
  const totalTokens =
    stats.total_tokens.input +
    stats.total_tokens.output +
    stats.total_tokens.cache_read +
    stats.total_tokens.cache_write
  const accent = dominantAccent(stats.cost_over_time)
  const peak = stats.activity.by_hour.reduce(
    (acc, v, i) => (v > acc.v ? { v, i } : acc),
    { v: 0, i: 0 }
  )

  return (
    <div
      style={{
        width: PORTRAIT_W,
        height: PORTRAIT_H,
        background: "linear-gradient(180deg, #0a0e1a 0%, #1e293b 100%)",
        padding: 96,
        boxSizing: "border-box",
        display: "flex",
        flexDirection: "column",
        justifyContent: "space-between",
        color: "#fff",
        fontFamily: "-apple-system, Inter, sans-serif",
      }}
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
        <div style={{ fontSize: 40, fontWeight: 800 }}>Memex</div>
        <TitleStrip text="Your year on Memex" fontSize={20} />
      </div>

      <div style={{ display: "flex", justifyContent: "center" }}>
        <HeroNumber
          value={fmtTokensCompact(totalTokens)}
          label="TOKENS"
          fontSize={360}
          accent={accent}
        />
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: 18 }}>
        {stats.activity.busiest_day && (
          <FactRow
            label="busiest day"
            value={`${stats.activity.busiest_day.date} · ${fmtNum(
              stats.activity.busiest_day.messages
            )} msgs`}
            fontSize={32}
          />
        )}
        <FactRow
          label="peak hour"
          value={`${String(peak.i).padStart(2, "0")}:00 (local)`}
          fontSize={32}
        />
        <FactRow
          label="conversations"
          value={fmtNum(stats.total_conversations)}
          fontSize={32}
        />
        {!hideSpend && stats.estimated_cost_usd > 0 && (
          <FactRow
            label="estimated spend"
            value={fmtUsd(stats.estimated_cost_usd)}
            fontSize={32}
          />
        )}
        {stats.first_at && stats.last_at && (
          <FactRow
            label="span"
            value={`${fmtDate(stats.first_at)} → ${fmtDate(stats.last_at)}`}
            fontSize={32}
          />
        )}
      </div>

      <div style={{ display: "flex", justifyContent: "center" }}>
        <PosterHeatmap daily={stats.activity.daily} cellSize={16} gap={3} />
      </div>

      <BrandStrip size="md" />
    </div>
  )
}
