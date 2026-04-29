import { useState } from "react"
import { open } from "@tauri-apps/plugin-dialog"
import { useNavigate } from "react-router-dom"
import { api } from "@/lib/api"
import { sourceLabel } from "@/lib/format"
import type { ImportSummary } from "@/types"
import { Button } from "@/components/ui/button"

export default function Onboarding() {
  const [busyExport, setBusyExport] = useState(false)
  const [busyCC, setBusyCC] = useState(false)
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
      setBusyExport(true)
      const r = await api.importExport(folder)
      setResult(r)
    } catch (e) {
      setError(String(e))
    } finally {
      setBusyExport(false)
    }
  }

  async function importCC() {
    setError(null)
    setResult(null)
    try {
      setBusyCC(true)
      const r = await api.importClaudeCode()
      setResult(r)
    } catch (e) {
      setError(String(e))
    } finally {
      setBusyCC(false)
    }
  }

  const busy = busyExport || busyCC

  return (
    <div className="p-12 max-w-2xl">
      <h2 className="text-2xl font-semibold tracking-tight mb-2">Import</h2>
      <p className="text-muted-foreground text-sm mb-8 leading-relaxed">
        Memex reads your AI conversation history from three sources. Pick one
        below — nothing leaves your machine.
      </p>

      {/* Claude Code: zero-effort */}
      <section className="border border-border rounded-lg p-5 mb-5">
        <div className="flex items-baseline justify-between mb-1">
          <h3 className="text-base font-semibold">Claude Code</h3>
          <span className="text-[10px] uppercase tracking-wider text-green-700 dark:text-green-400">
            recommended · accurate
          </span>
        </div>
        <p className="text-muted-foreground text-sm mb-3 leading-relaxed">
          Reads <code className="bg-muted px-1 py-0.5 rounded text-xs">~/.claude/projects/</code> directly.
          Token counts and per-session model come straight from the API response — costs are exact, not estimated.
        </p>
        <Button onClick={importCC} disabled={busy}>
          {busyCC ? "Importing…" : "Import Claude Code"}
        </Button>
      </section>

      {/* ChatGPT / Claude.ai web export */}
      <section className="border border-border rounded-lg p-5 mb-5">
        <h3 className="text-base font-semibold mb-1">ChatGPT or Claude.ai web export</h3>
        <p className="text-muted-foreground text-sm mb-3 leading-relaxed">
          Pick a folder containing a <code className="bg-muted px-1 py-0.5 rounded text-xs">conversations.json</code> file.
          Memex auto-detects whether it's ChatGPT (mapping tree) or Claude.ai (chat_messages array).
          Token counts are estimated via tiktoken — costs are approximate.
        </p>
        <Button onClick={pickFolder} disabled={busy} variant="secondary">
          {busyExport ? "Importing…" : "Choose folder"}
        </Button>
      </section>

      {error && (
        <div className="mt-4 p-4 rounded-md border border-destructive/30 bg-destructive/5 text-destructive text-sm">
          {error}
        </div>
      )}

      {result && (
        <div className="mt-4 p-4 rounded-md border border-green-500/30 bg-green-500/5 text-sm">
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
        <p className="font-medium text-foreground">How to get web exports:</p>
        <ul className="list-disc list-inside space-y-1">
          <li><strong>ChatGPT:</strong> chatgpt.com → Settings → Data controls → Export. Email + zip.</li>
          <li><strong>Claude.ai:</strong> claude.ai → Settings → Privacy → Export. Email + zip.</li>
        </ul>
      </div>
    </div>
  )
}
