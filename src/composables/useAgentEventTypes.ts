import type { ComputedRef, Ref } from 'vue'
import type { AgentMessage } from '@/types/agent'

export interface AgentStartEvent {
  execution_id: string
  task: string
}

export interface AgentChunkEvent {
  execution_id: string
  chunk_type: string
  content?: string
  input_tokens?: number
  output_tokens?: number
}

export interface AgentToolCallEvent {
  execution_id: string
  tool_id: string
  tool_name: string
  tool_input: any
}

export interface AgentToolCallCompleteEvent {
  execution_id: string
  tool_call_id: string
  tool_name: string
  arguments: string
}

export interface AgentToolResultEvent {
  execution_id: string
  tool_name: string
  tool_input: any
  tool_result: string
}

export interface AgentToolResultNewEvent {
  execution_id: string
  tool_call_id: string
  result: string
  success?: boolean
}

export interface AgentToolsSelectedEvent {
  execution_id: string
  tools: string[]
}

export interface AgentToolExecutedEvent {
  execution_id: string
  tool: string
  arguments: any
  result: string
  success: boolean
  iteration: number
}

export interface AgentIterationEvent {
  execution_id: string
  iteration: number
  max_iterations: number
}

export type AgentExecutionOutcome = 'succeeded' | 'failed' | 'cancelled'

export interface AgentExecutionFinishedEvent {
  execution_id: string
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

export interface AgentRetryEvent {
  execution_id: string
  retry_count: number
  max_retries: number
  error?: string
}

export interface AgentCompletionGuardFailedEvent {
  execution_id: string
  reasons: string[]
  required_artifact?: string | null
  response_length?: number
  tool_calls?: number
}

export interface AgentTenthManCritiqueEvent {
  execution_id: string
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

export interface ContextUsageInfo {
  usedTokens: number
  maxTokens: number
  usagePercentage: number
  systemPromptTokens: number
  historyTokens: number
  historyCount: number
  summaryTokens: number
  summaryGlobalTokens: number
  summarySegmentTokens: number
  summarySegmentCount: number
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
