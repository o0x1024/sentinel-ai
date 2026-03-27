import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import type { AiConversationSummary } from './conversationTypes'

const autoTitleGeneratingConversationIds = new Set<string>()

const normalizeConversationTitle = (title?: string | null): string => {
  return (title || '').replace(/\s+/g, ' ').trim()
}

const isDefaultConversationTitle = (
  title: string | null | undefined,
  defaultTitle: string
): boolean => {
  const normalized = normalizeConversationTitle(title)
  if (!normalized) return true
  const lower = normalized.toLowerCase()
  const localized = defaultTitle.toLowerCase()
  if (lower === localized || lower.startsWith(`${localized} `)) return true
  return /^new conversation(\s+.+)?$/i.test(normalized) || /^新会话(\s+.+)?$/i.test(normalized)
}

const sanitizeGeneratedConversationTitle = (rawTitle?: string | null): string => {
  if (!rawTitle) return ''
  const lines = String(rawTitle)
    .replace(/```[\s\S]*?```/g, (block) => block.replace(/```/g, ''))
    .split('\n')
    .map(line => line.trim())
    .filter(Boolean)
  let normalized = lines[0] || ''
  normalized = normalized
    .replace(/^(title|标题)\s*[:：]\s*/i, '')
    .replace(/^[`"'“”‘’《》【】()（）]+/, '')
    .replace(/[`"'“”‘’《》【】()（）]+$/, '')
    .replace(/[。.!?；;：:]+$/, '')
    .trim()
  if (normalized.length > 40) {
    normalized = normalized.slice(0, 40).trim()
  }
  return normalized
}

const buildFallbackConversationTitle = (content: string, defaultTitle: string): string => {
  const normalized = content.replace(/\s+/g, ' ').trim()
  if (!normalized) return defaultTitle
  if (normalized.length <= 30) return normalized
  return `${normalized.slice(0, 30).trim()}...`
}

const loadConversationSummaries = async (): Promise<AiConversationSummary[]> => {
  try {
    const conversations = await invoke<AiConversationSummary[]>('get_ai_conversations')
    return Array.isArray(conversations) ? conversations : []
  } catch (e) {
    console.warn('[AgentView] Failed to load conversation summaries:', e)
    return []
  }
}

const findConversationSummary = async (convId: string): Promise<AiConversationSummary | null> => {
  const conversations = await loadConversationSummaries()
  return conversations.find((conversation) => conversation.id === convId) || null
}

const generateConversationTitleWithLlm = async (
  firstMessage: string
): Promise<string | null> => {
  const normalizedInput = firstMessage.replace(/\s+/g, ' ').trim()
  if (!normalizedInput) return null

  const streamId = `conversation_title_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`
  let generatedTitle = ''
  let streamCompleted = false
  let streamError = ''
  let unlistenComplete: UnlistenFn | null = null
  let unlistenError: UnlistenFn | null = null

  try {
    unlistenComplete = await listen<{ stream_id?: string; content?: string }>(
      'plugin_gen_complete',
      (event) => {
        const payload = event.payload || {}
        if (payload.stream_id !== streamId) return
        generatedTitle = String(payload.content || '')
        streamCompleted = true
      }
    )
    unlistenError = await listen<{ stream_id?: string; error?: string }>(
      'plugin_gen_error',
      (event) => {
        const payload = event.payload || {}
        if (payload.stream_id !== streamId) return
        streamError = String(payload.error || 'title generation failed')
        streamCompleted = true
      }
    )

    await invoke('generate_plugin_stream', {
      request: {
        stream_id: streamId,
        message: `用户首条消息：${normalizedInput}`,
        system_prompt:
          '你是会话标题助手。请根据用户首条消息生成一个简短明确的会话标题。要求：使用与用户相同语言；不超过18个汉字或8个英文单词；不要引号、句号和前缀；只输出标题。',
        service_name: 'default',
      },
    })

    const maxWaitTimeMs = 20000
    const startTime = Date.now()
    while (!streamCompleted && Date.now() - startTime < maxWaitTimeMs) {
      await new Promise((resolve) => setTimeout(resolve, 120))
    }

    if (!streamCompleted || streamError) {
      if (streamError) {
        console.warn('[AgentView] Conversation title generation failed:', streamError)
      }
      return null
    }

    const sanitized = sanitizeGeneratedConversationTitle(generatedTitle)
    return sanitized || null
  } catch (e) {
    console.warn('[AgentView] Failed to call title generation stream:', e)
    return null
  } finally {
    if (unlistenComplete) unlistenComplete()
    if (unlistenError) unlistenError()
  }
}

export async function maybeAutoRenameConversationByFirstMessage(params: {
  convId: string
  firstMessage: string
  defaultTitle: string
  currentConversationId?: string | null
  onCurrentConversationTitleChange?: (title: string) => void
  onConversationListRefresh?: () => void
}): Promise<void> {
  const normalizedInput = params.firstMessage.replace(/\s+/g, ' ').trim()
  if (!params.convId || !normalizedInput) return
  if (autoTitleGeneratingConversationIds.has(params.convId)) return

  const conversation = await findConversationSummary(params.convId)
  if (!conversation) return
  if (!isDefaultConversationTitle(conversation.title, params.defaultTitle)) return
  const totalMessages = Number(conversation.total_messages ?? 0)
  if (totalMessages !== 0) return

  autoTitleGeneratingConversationIds.add(params.convId)
  try {
    const llmTitle = await generateConversationTitleWithLlm(normalizedInput)
    const finalTitle = llmTitle || buildFallbackConversationTitle(normalizedInput, params.defaultTitle)
    if (!finalTitle) return

    const latestConversation = await findConversationSummary(params.convId)
    if (!latestConversation) return
    if (!isDefaultConversationTitle(latestConversation.title, params.defaultTitle)) return

    await invoke('update_ai_conversation_title', {
      conversationId: params.convId,
      title: finalTitle,
      serviceName: 'default',
    })

    if (params.currentConversationId === params.convId) {
      params.onCurrentConversationTitleChange?.(finalTitle)
    }
    params.onConversationListRefresh?.()
  } catch (e) {
    console.warn('[AgentView] Failed to auto rename conversation:', e)
  } finally {
    autoTitleGeneratingConversationIds.delete(params.convId)
  }
}
