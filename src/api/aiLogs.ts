import { invoke } from '@tauri-apps/api/core'

export interface AiTurnLogQuery {
  date?: string
  conversationId?: string
  sessionId?: string
  limit?: number
}

export interface AiTurnLogEntry {
  timestamp: string
  session_id: string
  conversation_id: string
  turn?: number | null
  provider: string
  model: string
  summary: Record<string, any>
}

export interface AiTurnLogSummaryEntry {
  timestamp: string
  session_id: string
  conversation_id: string
  turn?: number | null
  provider: string
  model: string
  status: string
  duration_ms?: number | null
  input_tokens?: number | null
  output_tokens?: number | null
  tool_call_count: number
  user_request_preview: string
  assistant_response_preview: string
}

export async function getAiTurnLogs(query: AiTurnLogQuery): Promise<AiTurnLogSummaryEntry[]> {
  return invoke<AiTurnLogSummaryEntry[]>('get_ai_turn_logs', {
    request: {
      date: query.date,
      conversation_id: query.conversationId,
      session_id: query.sessionId,
      limit: query.limit,
    },
  })
}

export async function getAiTurnLogDetail(date: string | undefined, sessionId: string): Promise<AiTurnLogEntry | null> {
  return invoke<AiTurnLogEntry | null>('get_ai_turn_log_detail', {
    request: {
      date,
      session_id: sessionId,
    },
  })
}
