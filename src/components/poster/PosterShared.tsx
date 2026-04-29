import type { CostSeries } from "@/types"

const ACCENT_COLORS: Record<string, string> = {
  "Claude Opus":   "#a855f7",
  "Claude Sonnet": "#34d399",
  "Claude Haiku":  "#22d3ee",
  "GPT-4o":        "#22c55e",
}

/**
 * Pick a 4px accent color above the hero number based on the model
 * group with > 50% of total tokens. Mixed → emerald.
 */
export function dominantAccent(cost: CostSeries): string {
  const totalsByGroup: Record<string, number> = {}
  let total = 0
  for (const p of cost.points) {
    for (const [g, v] of p.by_model_group) {
      totalsByGroup[g] = (totalsByGroup[g] ?? 0) + v
      total += v
    }
  }
  if (total <= 0) return "#34d399"
  for (const [g, v] of Object.entries(totalsByGroup)) {
    if (v / total > 0.5 && ACCENT_COLORS[g]) {
      return ACCENT_COLORS[g]
    }
  }
  return "#34d399"
}

export function AccentLine({ color, width }: { color: string; width: number }) {
  return (
    <div
      style={{
        height: 4,
        width,
        background: color,
        borderRadius: 2,
        boxShadow: `0 0 16px ${color}66`,
      }}
    />
  )
}

export function BrandStrip({ size }: { size: "sm" | "md" }) {
  const fontSize = size === "sm" ? 13 : 16
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        color: "#64748b",
        fontSize,
        fontFamily: "-apple-system, Inter, sans-serif",
      }}
    >
      <span>
        <strong style={{ color: "#94a3b8", letterSpacing: 0.5 }}>memex.app</strong>
      </span>
      <span>100% local · no network</span>
    </div>
  )
}

export function HeroNumber({
  value,
  label,
  fontSize,
  accent,
}: {
  value: string
  label: string
  fontSize: number
  accent: string
}) {
  return (
    <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 12 }}>
      <AccentLine color={accent} width={Math.round(fontSize * 0.6)} />
      <div
        style={{
          fontSize,
          lineHeight: 0.95,
          fontWeight: 800,
          color: "#fff",
          fontFamily: "-apple-system, Inter, sans-serif",
          fontVariantNumeric: "tabular-nums",
          letterSpacing: -0.04 * fontSize,
        }}
      >
        {value}
      </div>
      <div
        style={{
          fontSize: Math.round(fontSize * 0.18),
          color: "#a3b1c6",
          fontFamily: "-apple-system, Inter, sans-serif",
          letterSpacing: 1,
        }}
      >
        {label}
      </div>
    </div>
  )
}

export function FactRow({
  label,
  value,
  fontSize,
}: {
  label: string
  value: string
  fontSize: number
}) {
  return (
    <div
      style={{
        display: "flex",
        justifyContent: "space-between",
        alignItems: "baseline",
        gap: 24,
        fontFamily: "-apple-system, Inter, sans-serif",
        fontSize,
        borderBottom: "1px solid rgba(148, 163, 184, 0.15)",
        paddingBottom: Math.round(fontSize * 0.4),
      }}
    >
      <span style={{ color: "#94a3b8", letterSpacing: 0.3 }}>{label}</span>
      <span
        style={{
          color: "#fff",
          fontVariantNumeric: "tabular-nums",
          fontWeight: 600,
        }}
      >
        {value}
      </span>
    </div>
  )
}

export function TitleStrip({ text, fontSize }: { text: string; fontSize: number }) {
  return (
    <div
      style={{
        fontSize,
        fontWeight: 700,
        letterSpacing: 4,
        textTransform: "uppercase",
        color: "#34d399",
        fontFamily: "-apple-system, Inter, sans-serif",
      }}
    >
      {text}
    </div>
  )
}
