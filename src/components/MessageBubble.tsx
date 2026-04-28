import type { Message } from "@/types"
import { Markdown } from "./Markdown"
import { fmtNum } from "@/lib/format"

const roleLabel: Record<Message["role"], string> = {
  user: "You",
  assistant: "Assistant",
  system: "System",
  tool: "Tool",
}

const roleStyle: Record<Message["role"], string> = {
  user:      "bg-blue-50/60 dark:bg-blue-950/20",
  assistant: "bg-background",
  system:    "bg-amber-50/60 dark:bg-amber-950/20",
  tool:      "bg-purple-50/60 dark:bg-purple-950/20",
}

export function MessageBubble({ m }: { m: Message }) {
  const tk = m.tokens
  const tkTotal = tk ? tk.input + tk.output + tk.cache_read + tk.cache_write : 0
  return (
    <div className={"px-6 py-5 border-b border-border " + roleStyle[m.role]}>
      <div className="text-xs uppercase tracking-wider text-muted-foreground mb-2 flex items-center gap-3">
        <span className="font-semibold">{roleLabel[m.role]}</span>
        {m.model && <span className="font-mono normal-case tracking-normal">{m.model}</span>}
        {tkTotal > 0 && (
          <span className="normal-case tracking-normal">
            {fmtNum(tkTotal)} tok
          </span>
        )}
      </div>
      <div className="prose prose-sm dark:prose-invert max-w-none prose-pre:bg-transparent prose-pre:p-0">
        <Markdown>{m.content}</Markdown>
      </div>
    </div>
  )
}
