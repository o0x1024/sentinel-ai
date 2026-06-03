import { invoke } from '@tauri-apps/api/core'
import type { ParallelModelState, ParallelRunState } from './agentParallelEventSupport'

const timers = new Map<string, number>()
const persistedEventSignatures = new Map<string, Map<string, string>>()

const buildEventSignature = (event: ParallelModelState['events'][number]): string => {
  return JSON.stringify({
    type: event.type,
    title: event.title,
    content: event.content,
    success: event.success,
    toolName: event.toolName,
    toolArgs: event.toolArgs,
    toolResult: event.toolResult,
    toolStatus: event.toolStatus,
    toolCallId: event.toolCallId,
    timestamp: event.timestamp,
  })
}

export const schedulePersistParallelModelEvents = (
  run: ParallelRunState,
  item: ParallelModelState,
): void => {
  if (!run.id || !item.modelRunId || !item.events?.length) return
  const key = `${run.id}:${item.modelRunId}`
  const existing = timers.get(key)
  if (existing) window.clearTimeout(existing)
  const timer = window.setTimeout(() => {
    timers.delete(key)
    const signatures = persistedEventSignatures.get(key) || new Map<string, string>()
    for (const event of item.events || []) {
      const signature = buildEventSignature(event)
      if (signatures.get(event.id) === signature) {
        continue
      }
      signatures.set(event.id, signature)
      void invoke('record_ai_parallel_run_event', {
        request: {
          id: event.id,
          parallel_run_id: run.id,
          model_run_id: item.modelRunId,
          type: event.type,
          title: event.title,
          content: event.content,
          success: event.success,
          tool_name: event.toolName,
          tool_args: event.toolArgs,
          tool_result: event.toolResult,
          tool_status: event.toolStatus,
          tool_call_id: event.toolCallId,
          timestamp_ms: event.timestamp,
        },
      }).catch((error) => {
        console.warn('[agentParallelEventPersistence] Failed to persist parallel event:', error)
      })
    }
    persistedEventSignatures.set(key, signatures)
  }, 250)
  timers.set(key, timer)
}
