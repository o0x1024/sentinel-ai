import { computed, ref, type ComputedRef, type Ref } from 'vue'
import type { AgentMessage } from '@/types/agent'
import {
  normalizeParallelRun,
  summarizeParallelRun,
  type ParallelModelState,
  type ParallelRunState,
} from '@/composables/agentParallelEventSupport'

export interface ParallelTaskSource {
  executionId: string
  parentConversationId: string
  label: string
}

export const useAgentParallelRunState = (params: {
  isExecuting: Ref<boolean>
  markExecutionSettled: (executionId: string) => void
  messages: Ref<AgentMessage[]>
  streamingContent: Ref<string>
}): {
  clearParallelRuns: () => void
  getParallelChild: (executionId: string) => { run: ParallelRunState; item: ParallelModelState } | null
  parallelRuns: Map<string, ParallelRunState>
  parallelTaskSources: ComputedRef<ParallelTaskSource[]>
  rememberParallelRun: (raw: any) => ParallelRunState
  settleParallelRunIfDone: (run: ParallelRunState) => void
  upsertParallelRunMessage: (run: ParallelRunState) => void
} => {
  const parallelRuns = new Map<string, ParallelRunState>()
  const parallelChildToRun = new Map<string, string>()
  const messageTimers = new Map<string, number>()
  const version = ref(0)

  const clearParallelRuns = (): void => {
    parallelRuns.clear()
    parallelChildToRun.clear()
    messageTimers.forEach((timer) => window.clearTimeout(timer))
    messageTimers.clear()
    version.value += 1
  }

  const writeParallelRunMessage = (run: ParallelRunState): void => {
    const messageId = `parallel:${run.id}`
    const metadata = {
      kind: 'parallel_model_execution',
      parallel_run_id: run.id,
      parallel_run: run,
    }
    const existing = params.messages.value.find((item) => item.id === messageId)
    if (existing) {
      existing.content = summarizeParallelRun(run)
      existing.metadata = metadata
      return
    }
    params.messages.value.push({
      id: messageId,
      type: 'system',
      content: summarizeParallelRun(run),
      timestamp: Date.now(),
      metadata,
    })
  }

  const upsertParallelRunMessage = (run: ParallelRunState): void => {
    const messageId = `parallel:${run.id}`
    const existing = params.messages.value.find((item) => item.id === messageId)
    if (!existing) {
      writeParallelRunMessage(run)
      return
    }
    const current = messageTimers.get(run.id)
    if (current) window.clearTimeout(current)
    const timer = window.setTimeout(() => {
      messageTimers.delete(run.id)
      writeParallelRunMessage(run)
    }, 120)
    messageTimers.set(run.id, timer)
  }

  const rememberParallelRun = (raw: any): ParallelRunState => {
    const run = normalizeParallelRun(raw)
    const existing = parallelRuns.get(run.id)
    if (existing) {
      for (const nextItem of run.items) {
        const previous = existing.items.find((item) => item.modelRunId === nextItem.modelRunId)
        if (previous) {
          nextItem.content = nextItem.content || previous.content
          nextItem.thinking = nextItem.thinking || previous.thinking
          nextItem.events = nextItem.events.length > 0 ? nextItem.events : previous.events
          nextItem.toolCount = Math.max(nextItem.toolCount, previous.toolCount)
          nextItem.inputTokens = nextItem.inputTokens || previous.inputTokens
          nextItem.outputTokens = nextItem.outputTokens || previous.outputTokens
          nextItem.firstResponseMs = nextItem.firstResponseMs || previous.firstResponseMs
          nextItem.costUsd = nextItem.costUsd || previous.costUsd
          nextItem.startedAtMs = nextItem.startedAtMs || previous.startedAtMs
          nextItem.completedAtMs = nextItem.completedAtMs || previous.completedAtMs
        }
      }
    }
    parallelRuns.set(run.id, run)
    run.items.forEach((item) => {
      if (item.modelRunId) {
        parallelChildToRun.set(item.modelRunId, run.id)
      }
    })
    version.value += 1
    return run
  }

  const getParallelChild = (executionId: string): { run: ParallelRunState; item: ParallelModelState } | null => {
    const runId = parallelChildToRun.get(executionId)
    if (!runId) return null
    const run = parallelRuns.get(runId)
    if (!run) return null
    const item = run.items.find((candidate) => candidate.modelRunId === executionId)
    return item ? { run, item } : null
  }

  const settleParallelRunIfDone = (run: ParallelRunState): void => {
    const allDone = run.items.every((item) =>
      ['succeeded', 'failed', 'cancelled'].includes(item.status),
    )
    if (!allDone) return
    params.isExecuting.value = false
    params.streamingContent.value = ''
    params.markExecutionSettled(run.parentConversationId)
  }

  const parallelTaskSources = computed<ParallelTaskSource[]>(() => {
    version.value
    return Array.from(parallelRuns.values()).flatMap((run) =>
      run.items
        .filter((item) => item.modelRunId)
        .map((item) => ({
          executionId: item.modelRunId,
          parentConversationId: run.parentConversationId,
          label: `${item.provider}/${item.model}`,
        })),
    )
  })

  return {
    clearParallelRuns,
    getParallelChild,
    parallelRuns,
    parallelTaskSources,
    rememberParallelRun,
    settleParallelRunIfDone,
    upsertParallelRunMessage,
  }
}
