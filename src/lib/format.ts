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
