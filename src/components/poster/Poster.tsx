import type { StatsReport } from "@/types"
import { PosterSquare, SQUARE_W, SQUARE_H } from "./PosterSquare"
import { PosterPortrait, PORTRAIT_W, PORTRAIT_H } from "./PosterPortrait"
import { PosterLandscape, LANDSCAPE_W, LANDSCAPE_H } from "./PosterLandscape"

export type PosterFormat = "square" | "portrait" | "landscape"

export const POSTER_DIMS: Record<
  PosterFormat,
  { w: number; h: number; label: string; filename: string }
> = {
  square:    { w: SQUARE_W,    h: SQUARE_H,    label: "1:1 · 1080×1080",  filename: "memex-poster-1080x1080.png" },
  portrait:  { w: PORTRAIT_W,  h: PORTRAIT_H,  label: "9:16 · 1080×1920", filename: "memex-poster-1080x1920.png" },
  landscape: { w: LANDSCAPE_W, h: LANDSCAPE_H, label: "16:9 · 1200×630",  filename: "memex-poster-1200x630.png" },
}

export function Poster({
  format,
  stats,
  hideSpend,
}: {
  format: PosterFormat
  stats: StatsReport
  hideSpend: boolean
}) {
  switch (format) {
    case "square":    return <PosterSquare    stats={stats} hideSpend={hideSpend} />
    case "portrait":  return <PosterPortrait  stats={stats} hideSpend={hideSpend} />
    case "landscape": return <PosterLandscape stats={stats} hideSpend={hideSpend} />
  }
}
