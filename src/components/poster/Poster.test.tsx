import { describe, it, expect } from "vitest"
import { render } from "@testing-library/react"
import { Poster } from "./Poster"
import type { StatsReport } from "@/types"

const fakeStats: StatsReport = {
  total_conversations: 248,
  total_messages: 4206,
  total_tokens: { input: 12_000, output: 805_000, cache_read: 0, cache_write: 0 },
  estimated_cost_usd: 234.71,
  first_at: 1693440000,
  last_at: 1714435200,
  activity: {
    daily: [{ date: "2025-11-14", messages: 175 }],
    busiest_day: { date: "2025-11-14", messages: 175 },
    by_hour: Array.from({ length: 24 }, (_, i) => (i === 14 ? 160 : 5)),
    by_weekday: [10, 60, 30, 30, 20, 80, 40],
  },
  top_phrases: [],
  top_topics: [],
  by_model: [["claude-sonnet-4-6", 200]],
  by_source: [["claude_code", 248]],
  cost_over_time: {
    bucket: "monthly",
    points: [
      {
        bucket_label: "2026-04",
        by_model_group: [
          ["Claude Sonnet", 200],
          ["Claude Opus", 30],
        ],
      },
    ],
  },
  by_project: [],
  tool_usage: [],
  message_length: [],
}

describe("Poster", () => {
  it.each(["square", "portrait", "landscape"] as const)(
    "renders %s with hero token figure",
    (format) => {
      const { getByText } = render(
        <Poster format={format} stats={fakeStats} hideSpend={false} />
      )
      expect(getByText("TOKENS")).toBeTruthy()
      // 12_000 + 805_000 = 817_000 → "817k"
      expect(getByText("817k")).toBeTruthy()
    }
  )

  it("hides the spend row when hideSpend=true", () => {
    const { queryByText, getByText } = render(
      <Poster format="square" stats={fakeStats} hideSpend={true} />
    )
    expect(getByText("conversations")).toBeTruthy()
    expect(queryByText("estimated spend")).toBeNull()
  })

  it("shows the spend row when hideSpend=false", () => {
    const { getByText } = render(
      <Poster format="square" stats={fakeStats} hideSpend={false} />
    )
    expect(getByText("estimated spend")).toBeTruthy()
  })
})
