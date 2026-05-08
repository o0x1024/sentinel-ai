import type { ComputedRef, Ref } from 'vue'
import type { AgentMessage } from '@/types/agent'
import type { ParallelTaskSource } from '@/composables/useAgentParallelRunState'

export interface AgentStartEvent {
  execution_id: string
  conversation_id?: string | null
  generation?: number | null
  task: string
}

export interface AgentChunkEvent {
  execution_id: string
  generation?: number | null
  chunk_type: string
  content?: string
  input_tokens?: number
  output_tokens?: number
}

export interface AgentToolCallEvent {
  execution_id: string
  generation?: number | null
  tool_id: string
  tool_name: string
  tool_input: any
}

export interface AgentToolCallCompleteEvent {
  execution_id: string
  generation?: number | null
  tool_call_id: string
  tool_name: string
  arguments: string
}

export interface AgentToolResultEvent {
  execution_id: string
  generation?: number | null
  tool_name: string
  tool_input: any
  tool_result: string
  tracked_artifacts?: any[]
}

export interface AgentToolResultNewEvent {
  execution_id: string
  generation?: number | null
  tool_call_id: string
  result: string
  success?: boolean
  tracked_artifacts?: any[]
}

export interface AgentToolsSelectedEvent {
  execution_id: string
  generation?: number | null
  tools: string[]
}

export interface AgentToolsActivatedEvent {
  execution_id: string
  generation?: number | null
  tool_ids: string[]
  query?: string | null
  runtime_hint?: string | null
  tools: string[]
}

export interface AgentToolExecutedEvent {
  execution_id: string
  generation?: number | null
  tool: string
  arguments: any
  result: string
  success: boolean
  iteration: number
}

export interface AgentIterationEvent {
  execution_id: string
  generation?: number | null
  iteration: number
  max_iterations: number
}

export type AgentExecutionOutcome = 'succeeded' | 'failed' | 'cancelled'

export interface AgentExecutionFinishedEvent {
  execution_id: string
  conversation_id?: string | null
  generation?: number | null
  outcome: AgentExecutionOutcome
  success: boolean
  error?: string | null
  response?: string
  message?: string | null
}

export interface AgentSegmentSummaryCreatedEvent {
  conversation_id: string
  segment_index: number
  summary: string
  tokens: number
}

export interface AgentGlobalSummaryUpdatedEvent {
  conversation_id: string
  summary: string
  tokens: number
}

export interface AgentContextCompressionStartedEvent {
  execution_id: string
  generation?: number | null
  conversation_id?: string
  status?: 'running'
  reason?: string
  recent_tokens?: number
  threshold_tokens?: number
  message_count?: number
  recent_message_count?: number
}

export interface AgentContextCompressionFinishedEvent {
  execution_id: string
  generation?: number | null
  conversation_id?: string
  status?: 'completed' | 'failed'
  reason?: string
  recent_tokens?: number
  threshold_tokens?: number
  message_count?: number
  recent_message_count?: number
  summary_segment_count?: number
  summary_segment_tokens?: number
  summary_global_tokens?: number
  merged_global?: boolean
  error?: string
}

export type AgentContextPressureLevel = 'Low' | 'Warning' | 'AutoCompact' | 'Blocking'

export interface AgentContextPressureEvent {
  execution_id: string
  generation?: number | null
  phase?: string
  used_tokens: number
  remaining_tokens: number
  usage_percentage: number
  context_pressure: AgentContextPressureLevel | string
  should_compact?: boolean
  should_block?: boolean
  system_prompt_tokens?: number
  task_tokens?: number
  history_tokens?: number
  history_count?: number
  max_context_tokens?: number
  effective_context_tokens?: number
  warning_threshold_tokens?: number
  auto_compact_threshold_tokens?: number
  blocking_threshold_tokens?: number
  output_reserve_tokens?: number
}

export interface AgentContextCompactionRequestedEvent {
  execution_id: string
  generation?: number | null
  phase?: string
  used_tokens?: number
  remaining_tokens?: number
  context_pressure?: AgentContextPressureLevel | string
}

export interface AgentRetryEvent {
  execution_id: string
  generation?: number | null
  retry_count: number
  max_retries: number
  error?: string
}

export interface AgentCompletionGuardFailedEvent {
  execution_id: string
  generation?: number | null
  reasons: string[]
  required_artifact?: string | null
  response_length?: number
  tool_calls?: number
}

export interface AgentTenthManCritiqueEvent {
  execution_id: string
  generation?: number | null
  critique: string
  message_id: string
}

export interface SubagentStartEvent {
  execution_id: string
  parent_execution_id: string
  role?: string
  task: string
}

export interface SubagentDoneEvent {
  execution_id: string
  parent_execution_id: string
  success: boolean
  output?: string
}

export interface SubagentErrorEvent {
  execution_id: string
  parent_execution_id: string
  error: string
}

export type SubagentStatus = 'running' | 'queued' | 'completed' | 'failed'

export interface SubagentItem {
  id: string
  parentId: string
  role?: string
  status: SubagentStatus
  progress?: number
  summary?: string
  tools?: string[]
  task?: string
  error?: string
  startedAt?: number
  duration?: number
}

export interface OrderedMessageChunk {
  execution_id: string
  generation?: number | null
  message_id: string
  conversation_id?: string
  sequence: number
  chunk_type: string
  content: string
  timestamp: { secs_since_epoch: number; nanos_since_epoch: number }
  is_final: boolean
  stage?: string
  tool_name?: string
  architecture?: string
  structured_data?: any
}

export interface RagMetaInfo {
  rag_applied: boolean
  rag_sources_used: boolean
  source_count: number
  citations?: any[]
}

export interface MemoryTraceCount {
  label: string
  count: number
}

export interface MemoryRetrievalInfo {
  queryPreview: string
  requestedTopK: number
  hitCount: number
  usedCanonicalFallback: boolean
  includeReflection: boolean
  sourceBreakdown: MemoryTraceCount[]
  kindBreakdown: MemoryTraceCount[]
}

export interface ContextUsageInfo {
  usedTokens: number
  maxTokens: number
  usagePercentage: number
  effectiveContextTokens?: number
  remainingTokens?: number
  contextPressure?: AgentContextPressureLevel | string | null
  warningThresholdTokens?: number
  autoCompactThresholdTokens?: number
  blockingThresholdTokens?: number
  outputReserveTokens?: number
  shouldCompact?: boolean
  shouldBlock?: boolean
  pressurePhase?: string | null
  taskTokens?: number
  systemPromptTokens: number
  historyTokens: number
  historyCount: number
  summaryTokens: number
  summaryGlobalTokens: number
  summarySegmentTokens: number
  summarySegmentCount: number
  sentinelMode?: boolean
  sentinelIntentId?: string | null
  sentinelIntentConfidence?: number | null
  sentinelIntentTransition?: string | null
  sentinelParentIntentId?: string | null
  sentinelClarificationNeeded?: boolean
  sentinelClarificationStatus?: string | null
  sentinelCompressionAggressiveness?: string | null
  memoryRetrieval?: MemoryRetrievalInfo | null
}

export interface ContextCompressionInfo {
  active: boolean
  executionId: string
  generation?: number | null
  reason?: string
  recentTokens: number
  thresholdTokens: number
  messageCount: number
  recentMessageCount: number
  startedAt: number
}

export interface UseAgentEventsReturn {
  messages: Ref<AgentMessage[]>
  isExecuting: Ref<boolean>
  currentExecutionId: Ref<string | null>
  error: Ref<string | null>
  streamingContent: Ref<string>
  subagents: Ref<SubagentItem[]>
  hasMessages: ComputedRef<boolean>
  lastMessage: ComputedRef<AgentMessage | undefined>
  ragMetaInfo: Ref<RagMetaInfo | null>
  contextUsage: Ref<ContextUsageInfo | null>
  contextCompression: Ref<ContextCompressionInfo | null>
  parallelTaskSources: ComputedRef<ParallelTaskSource[]>
  clearMessages: () => void
  resetError: () => void
  stopExecution: () => void
  startListening: () => Promise<void>
  stopListening: () => void
  setPendingDocumentAttachments: (docs: any[]) => void
}

export interface UseAgentEventsOptions {
  suppressUserMessages?: Ref<boolean> | ComputedRef<boolean> | boolean
  defaultMaxContextTokens?: Ref<number> | ComputedRef<number> | number
  subagentParentExecutionMatcher?: (parentExecutionId: string) => boolean
}
