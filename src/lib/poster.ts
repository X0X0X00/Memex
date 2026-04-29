import { toPng } from "html-to-image"
import { invoke } from "@tauri-apps/api/core"
import { save } from "@tauri-apps/plugin-dialog"
import { POSTER_DIMS, type PosterFormat } from "@/components/poster/Poster"

/**
 * Render an in-DOM node to PNG bytes. The node should already be
 * mounted at full poster resolution (1080×1080 etc.) — no scaling.
 */
export async function snapshotPoster(node: HTMLElement): Promise<Uint8Array> {
  const dataUrl = await toPng(node, {
    pixelRatio: 1,
    cacheBust: true,
    backgroundColor: "#0a0e1a",
  })
  const b64 = dataUrl.split(",", 2)[1]
  const bin = atob(b64)
  const bytes = new Uint8Array(bin.length)
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i)
  return bytes
}

/**
 * High-level: open save dialog, snapshot the given node, write to disk.
 * Returns the chosen path, or null if the user cancelled.
 */
export async function savePoster(
  node: HTMLElement,
  format: PosterFormat
): Promise<string | null> {
  const dims = POSTER_DIMS[format]
  const path = await save({
    defaultPath: dims.filename,
    filters: [{ name: "PNG image", extensions: ["png"] }],
  })
  if (!path || typeof path !== "string") return null
  const bytes = await snapshotPoster(node)
  await invoke<void>("save_png", { path, bytes: Array.from(bytes) })
  return path
}
