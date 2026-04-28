import { useEffect, useState, type PropsWithChildren } from "react"
import ReactMarkdown from "react-markdown"
import remarkGfm from "remark-gfm"
import { codeToHtml } from "shiki"

function CodeBlock({ language, value }: { language: string; value: string }) {
  const [html, setHtml] = useState("")
  useEffect(() => {
    let cancelled = false
    const theme =
      window.matchMedia?.("(prefers-color-scheme: dark)").matches
        ? "github-dark"
        : "github-light"
    codeToHtml(value, { lang: language || "text", theme })
      .then(h => { if (!cancelled) setHtml(h) })
      .catch(() => {
        if (!cancelled) setHtml(`<pre>${escapeHtml(value)}</pre>`)
      })
    return () => { cancelled = true }
  }, [language, value])
  return (
    <div
      className="rounded-md overflow-hidden text-sm my-2 [&>pre]:p-3 [&>pre]:overflow-x-auto"
      dangerouslySetInnerHTML={{ __html: html || `<pre>${escapeHtml(value)}</pre>` }}
    />
  )
}

function escapeHtml(s: string) {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
}

export function Markdown({ children }: PropsWithChildren<{ children: string }>) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      components={{
        code({ className, children, ...rest }) {
          const inline = (rest as { inline?: boolean }).inline
          const match = /language-(\w+)/.exec(className || "")
          const value = String(children).replace(/\n$/, "")
          if (inline || !match) {
            return (
              <code className="px-1 py-0.5 rounded bg-muted text-[0.9em] font-mono">
                {children}
              </code>
            )
          }
          return <CodeBlock language={match[1]} value={value} />
        },
      }}
    >
      {children}
    </ReactMarkdown>
  )
}
