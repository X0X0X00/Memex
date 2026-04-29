export type Source = "openai" | "claude_web" | "claude_code"
export type Role = "user" | "assistant" | "system" | "tool"

export interface TokenCounts {
  input: number
  output: number
  cache_read: number
  cache_write: number
}

export interface ConversationSummary {
  id: string
  source: string
  title: string
  created_at: number
  message_count: number
  model: string | null
  estimated_cost_usd: number
  tokens_total: number
}

export interface Message {
  id: string
  conversation_id: string
  role: Role
  content: string
  timestamp: number | null
  model: string | null
  tokens: TokenCounts | null
  tool_name: string | null
}

export interface ImportSummary {
  conversations_added: number
  messages_added: number
  source: string
}

export interface DailyActivity { date: string; messages: number }
export interface ActivityReport {
  daily: DailyActivity[]
  busiest_day: DailyActivity | null
  by_hour: number[]
  by_weekday: number[]
}
export interface PhraseStat { phrase: string; count: number; score: number }

export type BucketKind = "daily" | "weekly" | "monthly"

export interface CostPoint {
  bucket_label: string
  by_model_group: [string, number][]
}

export interface CostSeries {
  bucket: BucketKind
  points: CostPoint[]
}

export interface ProjectRow {
  project: string
  display_name: string
  conv_count: number
  msg_count: number
  token_count: number
  cost_usd: number
  first_at: number
  last_at: number
}

export interface LengthBucket {
  label: string
  n: number
}

export interface StatsReport {
  total_conversations: number
  total_messages: number
  total_tokens: TokenCounts
  estimated_cost_usd: number
  first_at: number | null
  last_at: number | null
  activity: ActivityReport
  top_phrases: PhraseStat[]
  top_topics: PhraseStat[]
  by_model: [string, number][]
  by_source: [string, number][]
  cost_over_time: CostSeries
  by_project: ProjectRow[]
  tool_usage: [string, number][]
  message_length: LengthBucket[]
}
