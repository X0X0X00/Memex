import { invoke } from "@tauri-apps/api/core"
import type { ConversationSummary, ImportSummary, Message, StatsReport } from "@/types"

export const api = {
  importExport: (folder: string) =>
    invoke<ImportSummary>("import_export", { folder }),
  importClaudeCode: (folder?: string) =>
    invoke<ImportSummary>("import_claude_code", { folder: folder ?? null }),
  listConversations: () =>
    invoke<ConversationSummary[]>("list_conversations"),
  getConversation: (id: string) =>
    invoke<Message[]>("get_conversation", { id }),
  getStats: () =>
    invoke<StatsReport>("get_stats"),
  clearData: () =>
    invoke<void>("clear_data"),
}
