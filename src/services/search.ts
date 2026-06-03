import { invoke } from '@tauri-apps/api/core'

export type SearchEngine = 'google' | 'bing' | 'baidu'

export interface WebSearchItem {
  title: string
  url: string
  snippet: string
}

export interface WebSearchResponse {
  engine: SearchEngine
  query: string
  items: WebSearchItem[]
}

export async function webSearch(query: string, engine: SearchEngine, limit = 5, locale?: string) {
  const res = await invoke('web_search', {
    request: { query, engine, limit, locale }
  }) as WebSearchResponse
  return res
}

export async function sendMessageWithSearch(
  conversationId: string,
  message: string,
  serviceName: string,
  opts?: { engine?: 'google'|'bing'|'baidu'|'auto', limit?: number, provider?: string, model?: string, temperature?: number, maxTokens?: number, messageId?: string }
) {
  const res = await invoke('send_ai_stream_with_search', {
    request: {
      conversationId,
      message,
      serviceName,
      engine: opts?.engine,
      auto: opts?.engine === 'auto' || opts?.engine === undefined,
      limit: opts?.limit ?? 5,
      provider: opts?.provider,
      model: opts?.model,
      temperature: opts?.temperature,
      maxTokens: opts?.maxTokens,
      messageId: opts?.messageId,
    }
  }) as string
  return res
}


