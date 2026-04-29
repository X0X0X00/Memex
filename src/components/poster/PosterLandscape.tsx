import type { StatsReport } from "@/types"
import { fmtDate, fmtNum, fmtUsd, fmtTokensCompact } from "@/lib/format"
import { BrandStrip, FactRow, HeroNumber, dominantAccent } from "./PosterShared"

export const LANDSCAPE_W = 1200
export const LANDSCAPE_H = 630

export function PosterLandscape({
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

  return (
    <div
      style={{
        width: LANDSCAPE_W,
        height: LANDSCAPE_H,
        background: "linear-gradient(120deg, #0a0e1a 0%, #1e293b 100%)",
        padding: 64,
        boxSizing: "border-box",
        display: "grid",
        gridTemplateColumns: "1fr 1fr",
        gridTemplateRows: "auto 1fr auto",
        gap: 24,
        color: "#fff",
        fontFamily: "-apple-system, Inter, sans-serif",
      }}
    >
      <div style={{ gridColumn: "1 / -1", fontSize: 26, fontWeight: 700 }}>
        Memex · Your year
      </div>

      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <HeroNumber
          value={fmtTokensCompact(totalTokens)}
          label="TOKENS"
          fontSize={180}
          accent={accent}
        />
      </div>

      <div
        style={{
          display: "flex",
          flexDirection: "column",
          justifyContent: "center",
          gap: 14,
        }}
      >
        {stats.activity.busiest_day && (
          <FactRow
            label="busiest day"
            value={`${stats.activity.busiest_day.date}`}
            fontSize={22}
          />
        )}
        <FactRow
          label="conversations"
          value={fmtNum(stats.total_conversations)}
          fontSize={22}
        />
        {!hideSpend && stats.estimated_cost_usd > 0 && (
          <FactRow
            label="estimated spend"
            value={fmtUsd(stats.estimated_cost_usd)}
            fontSize={22}
          />
        )}
        {stats.first_at && stats.last_at && (
          <FactRow
            label="span"
            value={`${fmtDate(stats.first_at)} → ${fmtDate(stats.last_at)}`}
            fontSize={22}
          />
        )}
      </div>

      <div style={{ gridColumn: "1 / -1" }}>
        <BrandStrip size="sm" />
      </div>
    </div>
  )
}
