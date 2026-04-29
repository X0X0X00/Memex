import type { DailyActivity } from "@/types"

type Metric = "messages" | "conversations"

export function ActivityHeatmap({
  daily,
  metric = "messages",
}: {
  daily: DailyActivity[]
  metric?: Metric
}) {
  const valueOf = (d: DailyActivity) =>
    metric === "messages" ? d.messages : d.conversations
  const unit = metric === "messages" ? "msgs" : "convs"

  const map = new Map(daily.map((d) => [d.date, valueOf(d)]))
  const today = new Date()
  today.setHours(0, 0, 0, 0)
  const start = new Date(today)
  start.setDate(start.getDate() - 364)
  start.setDate(start.getDate() - start.getDay())

  const cols = 53
  type Cell = { date: string; count: number; isFuture: boolean }
  const cells: Cell[] = []
  let max = 0
  for (let i = 0; i < cols * 7; i++) {
    const d = new Date(start)
    d.setDate(start.getDate() + i)
    const isFuture = d > today
    const date = d.toISOString().slice(0, 10)
    const count = map.get(date) ?? 0
    if (count > max) max = count
    cells.push({ date, count, isFuture })
  }

  function shade(cell: Cell) {
    if (cell.isFuture) return "bg-transparent"
    const c = cell.count
    if (c === 0) return "bg-muted"
    const t = max > 0 ? c / max : 0
    if (t < 0.25) return "bg-green-300/70 dark:bg-green-900"
    if (t < 0.5) return "bg-green-400/80 dark:bg-green-700"
    if (t < 0.75) return "bg-green-500   dark:bg-green-600"
    return "bg-green-600 dark:bg-green-500"
  }

  // Format date for tooltip in user's locale (with year).
  function tooltip(c: Cell): string {
    if (c.isFuture) return ""
    const d = new Date(c.date + "T00:00:00")
    const formatted = d.toLocaleDateString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
      weekday: "short",
    })
    return `${formatted} · ${c.count.toLocaleString()} ${unit}`
  }

  return (
    <div
      className="grid grid-flow-col grid-rows-7 gap-[2px]"
      style={{ gridTemplateColumns: `repeat(${cols}, 11px)` }}
    >
      {cells.map((c) => (
        <div
          key={c.date}
          className={`w-[11px] h-[11px] rounded-sm ${shade(c)}`}
          title={tooltip(c)}
        />
      ))}
    </div>
  )
}
