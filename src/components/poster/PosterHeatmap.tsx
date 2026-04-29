import type { DailyActivity } from "@/types"

interface Props {
  daily: DailyActivity[]
  cellSize: number          // px per cell
  gap?: number              // px between cells
}

const COLS = 53
const ROWS = 7

export function PosterHeatmap({ daily, cellSize, gap = 2 }: Props) {
  const map = new Map(daily.map((d) => [d.date, d.messages]))
  const today = new Date()
  today.setUTCHours(0, 0, 0, 0)
  const start = new Date(today)
  start.setUTCDate(start.getUTCDate() - (COLS * ROWS - 1))
  // Snap start to a Sunday so the heatmap aligns weekly.
  start.setUTCDate(start.getUTCDate() - start.getUTCDay())

  type Cell = { date: string; count: number; isFuture: boolean }
  const cells: Cell[] = []
  let max = 0
  for (let i = 0; i < COLS * ROWS; i++) {
    const d = new Date(start)
    d.setUTCDate(start.getUTCDate() + i)
    const isFuture = d > today
    const date = d.toISOString().slice(0, 10)
    const count = map.get(date) ?? 0
    if (count > max) max = count
    cells.push({ date, count, isFuture })
  }

  const shade = (c: Cell): string => {
    if (c.isFuture) return "transparent"
    if (c.count === 0) return "#1f2937"
    const t = max > 0 ? c.count / max : 0
    if (t < 0.25) return "#10b98155"
    if (t < 0.5)  return "#10b98199"
    if (t < 0.75) return "#10b981cc"
    return "#34d399"
  }

  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: `repeat(${COLS}, ${cellSize}px)`,
        gridTemplateRows: `repeat(${ROWS}, ${cellSize}px)`,
        gridAutoFlow: "column",
        gap,
        width: COLS * cellSize + (COLS - 1) * gap,
      }}
    >
      {cells.map((c) => (
        <div
          key={c.date}
          style={{
            width: cellSize,
            height: cellSize,
            background: shade(c),
            borderRadius: Math.max(2, Math.round(cellSize * 0.18)),
          }}
        />
      ))}
    </div>
  )
}
