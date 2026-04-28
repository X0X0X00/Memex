import { useState } from "react"
import { open } from "@tauri-apps/plugin-dialog"
import { useNavigate } from "react-router-dom"
import { api } from "@/lib/api"
import { sourceLabel } from "@/lib/format"
import type { ImportSummary } from "@/types"
import { Button } from "@/components/ui/button"

export default function Onboarding() {
  const [busy, setBusy] = useState(false)
  const [result, setResult] = useState<ImportSummary | null>(null)
  const [error, setError] = useState<string | null>(null)
  const nav = useNavigate()

  async function pickFolder() {
    setError(null)
    setResult(null)
    let folder: string | string[] | null
    try {
      folder = await open({ directory: true, multiple: false })
    } catch (e) {
      setError(String(e))
      return
    }
    if (!folder || typeof folder !== "string") return
    try {
      setBusy(true)
      const r = await api.importExport(folder)
      setResult(r)
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="p-12 max-w-2xl">
      <h2 className="text-2xl font-semibold tracking-tight mb-2">Import an export</h2>
      <p className="text-muted-foreground text-sm mb-6 leading-relaxed">
        Pick a folder containing a <code className="bg-muted px-1 py-0.5 rounded text-xs">conversations.json</code> file.
        Memex auto-detects whether it's a ChatGPT export (mapping tree) or a Claude.ai
        web export (chat_messages array). Nothing leaves your machine.
      </p>
      <Button onClick={pickFolder} disabled={busy}>
        {busy ? "Importing…" : "Choose folder"}
      </Button>

      {error && (
        <div className="mt-6 p-4 rounded-md border border-destructive/30 bg-destructive/5 text-destructive text-sm">
          {error}
        </div>
      )}

      {result && (
        <div className="mt-6 p-4 rounded-md border border-green-500/30 bg-green-500/5 text-sm">
          Imported <strong>{result.conversations_added}</strong> conversations
          ({result.messages_added.toLocaleString()} messages) from{" "}
          <strong>{sourceLabel(result.source)}</strong>.
          <button
            onClick={() => nav("/library")}
            className="ml-3 underline hover:no-underline"
          >
            Open library →
          </button>
        </div>
      )}

      <div className="mt-12 text-xs text-muted-foreground space-y-2">
        <p className="font-medium text-foreground">Where to find your export:</p>
        <ul className="list-disc list-inside space-y-1">
          <li><strong>ChatGPT:</strong> chatgpt.com → Settings → Data controls → Export data. You'll get an email with a zip; unzip it.</li>
          <li><strong>Claude.ai:</strong> claude.ai → Settings → Privacy → Export data. Same flow — email + zip.</li>
        </ul>
      </div>
    </div>
  )
}
