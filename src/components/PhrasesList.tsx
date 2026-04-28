import type { PhraseStat } from "@/types"

export function PhrasesList({ phrases }: { phrases: PhraseStat[] }) {
  if (!phrases.length) {
    return <div className="text-sm text-muted-foreground">Not enough data yet.</div>
  }
  return (
    <ol className="space-y-1.5">
      {phrases.map((p, i) => (
        <li
          key={p.phrase}
          className="flex items-baseline justify-between gap-3 text-sm border-b border-border/40 pb-1"
        >
          <span className="text-muted-foreground w-6 shrink-0 tabular-nums">
            {i + 1}.
          </span>
          <span className="flex-1 truncate font-medium">{p.phrase}</span>
          <span className="text-muted-foreground shrink-0 tabular-nums text-xs">
            ×{p.count}
          </span>
        </li>
      ))}
    </ol>
  )
}
