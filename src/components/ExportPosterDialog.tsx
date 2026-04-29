import { useEffect, useRef, useState } from "react"
import { Poster, POSTER_DIMS, type PosterFormat } from "./poster/Poster"
import { savePoster } from "@/lib/poster"
import type { StatsReport } from "@/types"
import { Button } from "@/components/ui/button"

const FORMATS: PosterFormat[] = ["square", "portrait", "landscape"]

export function ExportPosterDialog({
  stats,
  open,
  onClose,
}: {
  stats: StatsReport
  open: boolean
  onClose: () => void
}) {
  const [format, setFormat] = useState<PosterFormat>("square")
  const [hideSpend, setHideSpend] = useState(false)
  const [busy, setBusy] = useState(false)
  const [savedPath, setSavedPath] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const fullRef = useRef<HTMLDivElement>(null)

  // Reset transient state when reopened.
  useEffect(() => {
    if (open) {
      setSavedPath(null)
      setError(null)
    }
  }, [open])

  if (!open) return null

  const dims = POSTER_DIMS[format]

  async function onSave() {
    setBusy(true)
    setError(null)
    setSavedPath(null)
    try {
      const node = fullRef.current
      if (!node) throw new Error("poster not mounted")
      const path = await savePoster(node, format)
      if (path) setSavedPath(path)
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6"
      onClick={onClose}
    >
      <div
        className="bg-background rounded-xl border border-border shadow-2xl max-w-3xl w-full p-6 space-y-5"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold">Export poster</h2>
          <button
            onClick={onClose}
            className="text-muted-foreground hover:text-foreground text-2xl leading-none"
          >
            ×
          </button>
        </div>

        {/* Thumbnails */}
        <div className="grid grid-cols-3 gap-3">
          {FORMATS.map((f) => {
            const d = POSTER_DIMS[f]
            const maxThumbW = 220
            const maxThumbH = 240
            const scale = Math.min(maxThumbW / d.w, maxThumbH / d.h)
            const isSelected = format === f
            return (
              <button
                key={f}
                onClick={() => setFormat(f)}
                className={
                  "rounded-lg p-2 border transition-colors text-left " +
                  (isSelected
                    ? "border-primary ring-2 ring-primary"
                    : "border-border hover:border-muted-foreground")
                }
              >
                <div
                  style={{
                    width: maxThumbW,
                    height: maxThumbH,
                    overflow: "hidden",
                    borderRadius: 6,
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    background: "#0a0e1a",
                  }}
                >
                  <div
                    style={{
                      transform: `scale(${scale})`,
                      transformOrigin: "center center",
                      width: d.w,
                      height: d.h,
                      flexShrink: 0,
                    }}
                  >
                    <Poster format={f} stats={stats} hideSpend={hideSpend} />
                  </div>
                </div>
                <div className="text-xs text-muted-foreground mt-2 text-center">
                  {d.label}
                </div>
              </button>
            )
          })}
        </div>

        {/* Hide spend toggle */}
        <label className="flex items-center gap-2 text-sm cursor-pointer select-none">
          <input
            type="checkbox"
            checked={hideSpend}
            onChange={(e) => setHideSpend(e.target.checked)}
          />
          Hide estimated spend ($)
        </label>

        {error && (
          <div className="p-3 rounded-md border border-destructive/30 bg-destructive/5 text-destructive text-sm">
            {error}
          </div>
        )}
        {savedPath && (
          <div className="p-3 rounded-md border border-green-500/30 bg-green-500/5 text-sm">
            Saved to <code className="text-xs break-all">{savedPath}</code>
          </div>
        )}

        <div className="flex justify-end gap-2">
          <Button variant="ghost" onClick={onClose}>Cancel</Button>
          <Button onClick={onSave} disabled={busy}>
            {busy ? "Saving…" : `Save ${dims.w}×${dims.h} PNG`}
          </Button>
        </div>

        {/* Hidden full-resolution poster used by snapshotPoster */}
        <div
          style={{
            position: "fixed",
            left: -99999,
            top: 0,
            pointerEvents: "none",
          }}
        >
          <div ref={fullRef}>
            <Poster format={format} stats={stats} hideSpend={hideSpend} />
          </div>
        </div>
      </div>
    </div>
  )
}
