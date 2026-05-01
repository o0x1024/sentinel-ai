export type ParallelModelStatus = 'pending' | 'running' | 'succeeded' | 'failed' | 'cancelled'
export type ParallelJudgeStatus = 'not_requested' | 'pending' | 'running' | 'succeeded' | 'failed'

export type ParallelModelEventType = 'text' | 'thinking' | 'tool_call' | 'tool_result' | 'system' | 'error'

export type ParallelModelEvent = {
  id: string
  type: ParallelModelEventType
  title: string
  content: string
  timestamp: number
  success?: boolean
  toolName?: string
  toolArgs?: any
  toolResult?: any
  toolStatus?: 'pending' | 'running' | 'completed' | 'failed' | 'cancelled'
  toolCallId?: string
}

export type ParallelModelState = {
  id?: string
  modelRunId: string
  messageId?: string
  provider: string
  model: string
  status: ParallelModelStatus
  content: string
  thinking?: string
  error?: string | null
  toolCount: number
  inputTokens?: number
  outputTokens?: number
  firstResponseMs?: number | null
  costUsd?: number | null
  events: ParallelModelEvent[]
  startedAtMs?: number | null
  completedAtMs?: number | null
}

export type ParallelRunState = {
  id: string
  parentConversationId: string
  task: string
  status: ParallelModelStatus | 'partial'
  aggregationMode: 'manual' | 'judge'
  judgeProvider?: string | null
  judgeModel?: string | null
  judgeStatus: ParallelJudgeStatus
  judgeContent?: string | null
  judgeError?: string | null
  createdAtMs?: number
  updatedAtMs?: number
  items: ParallelModelState[]
}

const hashString = (value: string): string => {
  let hash = 2166136261
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index)
    hash = Math.imul(hash, 16777619)
  }
  return (hash >>> 0).toString(36)
}

const eventComparableValue = (value: unknown): string => {
  if (value == null) return ''
  if (typeof value === 'string') return value
  try {
    return JSON.stringify(value)
  } catch {
    return String(value)
  }
}

const buildEventDedupeKey = (event: ParallelModelEvent): string => {
  if (event.toolCallId && (event.type === 'tool_call' || event.type === 'tool_result')) {
    return `tool:${event.toolCallId}`
  }
  return [
    event.type,
    event.title,
    event.content,
    event.toolName || '',
    eventComparableValue(event.toolArgs),
    eventComparableValue(event.toolResult),
  ].join('\u001f')
}

const normalizeParallelEvents = (
  rawEvents: any[],
  modelRunId: string,
): ParallelModelEvent[] => {
  const byKey = new Map<string, ParallelModelEvent>()
  for (const rawEvent of rawEvents) {
    const eventType = String(rawEvent?.type || rawEvent?.event_type || 'system') as ParallelModelEventType
    const event: ParallelModelEvent = {
      id: String(rawEvent?.id || ''),
      type: eventType,
      title: String(rawEvent?.title || ''),
      content: String(rawEvent?.content || ''),
      timestamp: Number(rawEvent?.timestamp ?? rawEvent?.timestamp_ms ?? Date.now()) || Date.now(),
      success: rawEvent?.success ?? undefined,
      toolName: rawEvent?.toolName ?? rawEvent?.tool_name,
      toolArgs: rawEvent?.toolArgs ?? rawEvent?.tool_args,
      toolResult: rawEvent?.toolResult ?? rawEvent?.tool_result,
      toolStatus: rawEvent?.toolStatus ?? rawEvent?.tool_status,
      toolCallId: rawEvent?.toolCallId ?? rawEvent?.tool_call_id,
    }
    if (!event.id) {
      event.id = `parallel-event:${modelRunId}:${hashString(buildEventDedupeKey(event))}`
    }
    const key = buildEventDedupeKey(event)
    const previous = byKey.get(key)
    if (
      previous &&
      previous.type === 'tool_result' &&
      event.type === 'tool_call'
    ) {
      continue
    }
    byKey.set(key, event)
  }
  const ordered = Array.from(byKey.values()).sort((a, b) => a.timestamp - b.timestamp)
  const collapsed: ParallelModelEvent[] = []
  for (const event of ordered) {
    const last = collapsed[collapsed.length - 1]
    if (last?.type === 'text' && event.type === 'text') {
      if (event.content.startsWith(last.content) || event.content.length >= last.content.length) {
        last.content = event.content
        last.timestamp = event.timestamp
        last.id = event.id
      } else if (!last.content.startsWith(event.content)) {
        last.content = `${last.content}\n\n${event.content}`
        last.timestamp = event.timestamp
      }
      continue
    }
    collapsed.push(event)
  }
  return collapsed
}

export const normalizeParallelRun = (raw: any): ParallelRunState => ({
  id: String(raw?.id || raw?.parallel_run_id || ''),
  parentConversationId: String(raw?.parent_conversation_id || raw?.parentConversationId || ''),
  task: String(raw?.task || ''),
  status: String(raw?.status || 'running') as ParallelRunState['status'],
  aggregationMode: String(raw?.aggregation_mode || raw?.aggregationMode || 'manual') === 'judge' ? 'judge' : 'manual',
  judgeProvider: raw?.judge_provider ?? raw?.judgeProvider ?? null,
  judgeModel: raw?.judge_model ?? raw?.judgeModel ?? null,
  judgeStatus: String(raw?.judge_status || raw?.judgeStatus || 'not_requested') as ParallelJudgeStatus,
  judgeContent: raw?.judge_content ?? raw?.judgeContent ?? null,
  judgeError: raw?.judge_error ?? raw?.judgeError ?? null,
  createdAtMs: Number(raw?.created_at_ms ?? raw?.createdAtMs ?? 0) || undefined,
  updatedAtMs: Number(raw?.updated_at_ms ?? raw?.updatedAtMs ?? 0) || undefined,
  items: Array.isArray(raw?.items)
    ? raw.items.map((item: any) => {
        const modelRunId = String(item?.model_run_id || item?.modelRunId || '')
        return {
          id: item?.id,
          modelRunId,
          messageId: item?.message_id || item?.messageId,
          provider: String(item?.provider || ''),
          model: String(item?.model || ''),
          status: String(item?.status || 'pending') as ParallelModelStatus,
          content: String(item?.content || ''),
          thinking: String(item?.thinking || ''),
          error: item?.error ?? null,
          toolCount: Number(item?.tool_count ?? item?.toolCount ?? 0) || 0,
          inputTokens: Number(item?.input_tokens ?? item?.inputTokens ?? 0) || 0,
          outputTokens: Number(item?.output_tokens ?? item?.outputTokens ?? 0) || 0,
          firstResponseMs: Number(item?.first_response_ms ?? item?.firstResponseMs ?? 0) || null,
          costUsd: Number(item?.cost_usd ?? item?.costUsd ?? 0) || null,
          events: Array.isArray(item?.events)
            ? normalizeParallelEvents(item.events, modelRunId)
            : [],
          startedAtMs: item?.started_at_ms ?? item?.startedAtMs ?? null,
          completedAtMs: item?.completed_at_ms ?? item?.completedAtMs ?? null,
        }
      })
    : [],
})

export const summarizeParallelRun = (run: ParallelRunState): string => {
  const done = run.items.filter((item) => ['succeeded', 'failed', 'cancelled'].includes(item.status)).length
  const judge = run.judgeStatus === 'succeeded' ? '，Judge 已汇总' : ''
  return `多模型并行执行 ${done}/${run.items.length}${judge}`
}

const nextEventId = (): string => {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID()
  }
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`
}

const truncateEventContent = (value: unknown): string => {
  const raw = typeof value === 'string' ? value : JSON.stringify(value ?? '', null, 2)
  return raw.length > 4000 ? `${raw.slice(0, 4000)}\n...` : raw
}

export const appendParallelModelEvent = (
  item: ParallelModelState,
  event: Omit<ParallelModelEvent, 'id' | 'timestamp'> & { id?: string; timestamp?: number },
): void => {
  const next: ParallelModelEvent = {
    id: event.id || nextEventId(),
    timestamp: event.timestamp || Date.now(),
    type: event.type,
    title: event.title,
    content: truncateEventContent(event.content),
    success: event.success,
    toolName: event.toolName,
    toolArgs: event.toolArgs,
    toolResult: event.toolResult,
    toolStatus: event.toolStatus,
    toolCallId: event.toolCallId,
  }
  const existingIndex = (item.events || []).findIndex((candidate) => candidate.id === next.id)
  if (existingIndex >= 0) {
    item.events[existingIndex] = next
    return
  }
  item.events = [
    ...(item.events || []),
    next,
  ].slice(-80)
}

const appendParallelTextEvent = (item: ParallelModelState, content: string): void => {
  const events = item.events || []
  const last = events[events.length - 1]
  if (last?.type === 'text') {
    last.content = `${last.content}${content}`
    last.timestamp = Date.now()
    return
  }
  appendParallelModelEvent(item, {
    id: `parallel-event:${item.modelRunId}:text:${events.length}`,
    type: 'text',
    title: '输出',
    content,
  })
}

export const markParallelModelRunning = (item: ParallelModelState): void => {
  if (item.status === 'pending') {
    item.status = 'running'
  }
}

export const markParallelFirstResponse = (item: ParallelModelState): void => {
  if (item.firstResponseMs || !item.startedAtMs) return
  const elapsed = Date.now() - Number(item.startedAtMs)
  if (Number.isFinite(elapsed) && elapsed > 0) {
    item.firstResponseMs = Math.round(elapsed)
  }
}

export const appendParallelChunk = (
  item: ParallelModelState,
  chunkType: string,
  content?: string,
  usage?: { inputTokens?: number; outputTokens?: number },
): void => {
  markParallelModelRunning(item)
  if (chunkType === 'usage') {
    item.inputTokens = usage?.inputTokens || item.inputTokens || 0
    item.outputTokens = usage?.outputTokens || item.outputTokens || 0
    return
  }
  if (!content) return
  if (chunkType === 'reasoning' || chunkType === 'Thinking') {
    markParallelFirstResponse(item)
    item.thinking = `${item.thinking || ''}${content}`
    appendParallelModelEvent(item, {
      type: 'thinking',
      title: '思考',
      content,
    })
    return
  }
  if (chunkType === 'Error') {
    item.status = 'failed'
    item.error = content
    appendParallelModelEvent(item, {
      type: 'error',
      title: '错误',
      content,
      success: false,
    })
    return
  }
  item.content = `${item.content || ''}${content}`
  markParallelFirstResponse(item)
  appendParallelTextEvent(item, content)
}

export const applyParallelSessionStats = (
  item: ParallelModelState,
  stats: any,
): void => {
  if (!stats || typeof stats !== 'object') return
  const inputTokens = Number(stats.input_tokens)
  const outputTokens = Number(stats.output_tokens)
  const firstResponseMs = Number(stats.first_response_ms)
  const durationMs = Number(stats.duration_ms)
  if (Number.isFinite(inputTokens) && inputTokens >= 0) {
    item.inputTokens = Math.floor(inputTokens)
  }
  if (Number.isFinite(outputTokens) && outputTokens >= 0) {
    item.outputTokens = Math.floor(outputTokens)
  }
  if (Number.isFinite(firstResponseMs) && firstResponseMs > 0) {
    item.firstResponseMs = Math.round(firstResponseMs)
  }
  if (!item.completedAtMs && item.startedAtMs && Number.isFinite(durationMs) && durationMs > 0) {
    item.completedAtMs = Number(item.startedAtMs) + Math.round(durationMs)
  }
}

export const appendParallelToolCall = (
  item: ParallelModelState,
  toolName: string,
  args: unknown,
  toolCallId?: string,
): void => {
  markParallelModelRunning(item)
  markParallelFirstResponse(item)
  const existing = toolCallId
    ? (item.events || []).find((event) =>
        (event.type === 'tool_call' || event.type === 'tool_result') &&
        event.toolCallId === toolCallId,
      )
    : [...(item.events || [])].reverse().find((event) =>
        event.type === 'tool_call' &&
        event.toolName === toolName &&
        event.content === truncateEventContent(args) &&
        event.toolStatus === 'running',
      )
  if (existing) {
    if (existing.type === 'tool_result') {
      return
    }
    existing.type = 'tool_call'
    existing.title = `调用工具: ${toolName || 'unknown'}`
    existing.content = truncateEventContent(args)
    existing.toolName = toolName
    existing.toolArgs = args
    existing.toolStatus = 'running'
    existing.toolResult = undefined
    existing.success = undefined
    existing.timestamp = Date.now()
    return
  }
  appendParallelModelEvent(item, {
    id: toolCallId
      ? `parallel-event:${item.modelRunId}:tool:${toolCallId}`
      : `parallel-event:${item.modelRunId}:tool:${hashString(`${toolName}:${truncateEventContent(args)}`)}:${item.events.length}`,
    type: 'tool_call',
    title: `调用工具: ${toolName || 'unknown'}`,
    content: truncateEventContent(args),
    toolName,
    toolArgs: args,
    toolStatus: 'running',
    toolCallId,
  })
}

export const appendParallelToolResult = (
  item: ParallelModelState,
  toolName: string,
  result: unknown,
  success?: boolean,
  toolCallId?: string,
): void => {
  markParallelModelRunning(item)
  const existing = [...(item.events || [])].reverse().find((event) => {
    if (event.type !== 'tool_call') return false
    if (toolCallId && event.toolCallId === toolCallId) return true
    return !toolCallId && event.toolName === toolName && event.toolStatus === 'running'
  })
  if (existing) {
    item.toolCount += 1
    existing.type = 'tool_result'
    existing.title = `工具结果: ${toolName || 'unknown'}`
    existing.content = truncateEventContent(result)
    existing.success = success
    existing.toolName = toolName || existing.toolName
    existing.toolResult = result
    existing.toolStatus = success === false ? 'failed' : 'completed'
    existing.toolCallId = toolCallId || existing.toolCallId
    return
  }
  const completed = toolCallId
    ? [...(item.events || [])].reverse().find((event) =>
        event.type === 'tool_result' && event.toolCallId === toolCallId,
      )
    : [...(item.events || [])].reverse().find((event) =>
        event.type === 'tool_result' &&
        event.toolName === toolName &&
        event.content === truncateEventContent(result),
      )
  if (completed) {
    completed.title = `工具结果: ${toolName || 'unknown'}`
    completed.content = truncateEventContent(result)
    completed.success = success
    completed.toolName = toolName || completed.toolName
    completed.toolResult = result
    completed.toolStatus = success === false ? 'failed' : 'completed'
    completed.timestamp = Date.now()
    return
  }
  item.toolCount += 1
  appendParallelModelEvent(item, {
    id: toolCallId
      ? `parallel-event:${item.modelRunId}:tool:${toolCallId}`
      : `parallel-event:${item.modelRunId}:tool-result:${hashString(`${toolName}:${truncateEventContent(result)}`)}:${item.events.length}`,
    type: 'tool_result',
    title: `工具结果: ${toolName || 'unknown'}`,
    content: truncateEventContent(result),
    success,
    toolName,
    toolResult: result,
    toolStatus: success === false ? 'failed' : 'completed',
    toolCallId,
  })
}

export const appendParallelSystemEvent = (
  item: ParallelModelState,
  title: string,
  content: unknown,
): void => {
  markParallelModelRunning(item)
  appendParallelModelEvent(item, {
    type: 'system',
    title,
    content: truncateEventContent(content),
  })
}
