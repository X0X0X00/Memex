export const fmtNum = (n: number) => n.toLocaleString()
export const fmtUsd = (n: number) => "$" + (n < 0.01 ? n.toFixed(4) : n.toFixed(2))
export const fmtDate = (unix: number) =>
  new Date(unix * 1000).toLocaleDateString(undefined, {
    year: "numeric", month: "short", day: "numeric"
  })
export const fmtDateTime = (unix: number) =>
  new Date(unix * 1000).toLocaleString()

export function sourceLabel(s: string): string {
  switch (s) {
    case "openai":      return "ChatGPT"
    case "claude_web":  return "Claude.ai"
    case "claude_code": return "Claude Code"
    default:            return s
  }
}

export function fmtTokens(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M"
  if (n >= 1_000) return (n / 1_000).toFixed(1) + "k"
  return n.toLocaleString()
}

const MODEL_COLORS: Record<string, string> = {
  "Claude Opus":    "#a855f7",
  "Claude Sonnet":  "#10b981",
  "Claude Haiku":   "#06b6d4",
  "Claude (other)": "#64748b",
  "GPT-4o":         "#22c55e",
  "GPT-4":          "#3b82f6",
  "GPT-3.5":        "#94a3b8",
  "o-series":       "#f59e0b",
  "Other":          "#64748b",
  "(unknown)":      "#475569",
}

export function modelGroupColor(group: string): string {
  return MODEL_COLORS[group] ?? "#64748b"
}
