/**
 * Agent 类型定义
 * 完整的 Agent 系统类型，与后端保持一致
 */

// ============ 消息类型 ============

// Agent 消息类型
export type MessageType =
  | 'user'          // 用户输入
  | 'thinking'      // 思考过程
  | 'planning'      // 任务规划
  | 'tool_call'     // 工具调用
  | 'tool_result'   // 工具结果
  | 'progress'      // 进度更新
  | 'final'         // 最终答案
  | 'error'         // 错误信息
  | 'system'        // 系统消息

// 消息元数据
export interface MessageMetadata {
  tool_name?: string
  tool_args?: Record<string, any>
  tool_result?: string  // 工具执行结果（合并显示）
  error?: string
  duration_ms?: number
  step_index?: number
  total_steps?: number
  success?: boolean
  iteration?: number
  selected_tools?: string[]
  tool_call_id?: string  // 工具调用 ID，用于关联调用和结果
  status?: 'pending' | 'running' | 'completed' | 'failed'  // 工具调用状态
  rag_info?: {
    rag_applied: boolean
    rag_sources_used: boolean
    source_count: number
    citations?: any[]
  }
  execution_id?: string
  kind?: string  // 消息类型标识（如 'segment_summary', 'global_summary', 'tenth_man_critique'）
  segment_index?: number  // 段落摘要：段落索引
  summary_tokens?: number  // 摘要：token数
  summary_content?: string  // 摘要：完整内容
  trigger?: string  // 第十人原则触发原因
  retry_count?: number  // 第十人原则重试次数
  requires_confirmation?: boolean  // 第十人原则是否需要确认
}

// Agent 消息
export interface AgentMessage {
  id: string
  type: MessageType
  content: string
  timestamp: number
  metadata?: MessageMetadata
}

// ============ 工具调用 ============

// 工具调用状态
export type ToolStatus = 'pending' | 'running' | 'completed' | 'failed'

// 工具调用
export interface ToolCall {
  name: string
  args: Record<string, any>
}

// 工具调用数据
export interface ToolCallData {
  id: string
  name: string
  args: Record<string, any>
  status: ToolStatus
  startTime?: number
  endTime?: number
  durationMs?: number
}

// 工具结果数据
export interface ToolResultData {
  callId: string
  success: boolean
  output: any
  error?: string
}

// ============ 计划与步骤 ============

// 计划步骤
export interface PlanStep {
  id: string
  description: string
  tool?: ToolCall
  depends_on: string[]
  fallback?: string
}

// 任务计划
export interface Plan {
  description: string
  steps: PlanStep[]
  expected_outcome: string
}

// 步骤执行结果
export interface StepResult {
  step_id: string
  success: boolean
  output: any
  error?: string
  duration_ms: number
}

// ============ 反思与决策 ============

// 决策类型
export type Decision =
  | { type: 'complete'; answer: string }
  | { type: 'continue' }
  | { type: 'replan'; reason: string }

// 反思结果
export interface Reflection {
  decision: Decision
  reasoning: string
  improvements: string[]
}

// ============ 执行结果 ============

// Agent 执行结果
export interface AgentResult {
  execution_id: string
  success: boolean
  answer?: string
  error?: string
  iterations: number
  duration_ms: number
  step_results: StepResult[]
}

// ============ 事件 Payload ============

// 开始事件
export interface AgentStartPayload {
  execution_id: string
  task: string
  timestamp: number
}

// 消息事件
export interface AgentMessagePayload {
  execution_id: string
  message: AgentMessage
}

// 完成事件
export interface AgentCompletePayload {
  execution_id: string
  success: boolean
  duration_ms: number
  timestamp: number
}

// 错误事件
export interface AgentErrorPayload {
  execution_id: string
  error: string
  recoverable: boolean
}

// 内容块事件（流式）
export interface ContentChunkPayload {
  execution_id: string
  message_id?: string
  chunk: string
  is_complete: boolean
  timestamp: number
}

// ============ 执行块（旧版兼容） ============

// Agent执行块类型
export type AgentBlockType = 'task' | 'thinking' | 'tool_call' | 'tool_result' | 'final_answer' | 'error'

// 任务数据
export interface TaskData {
  content: string
  attachments?: any[]
}

// 思考数据
export interface ThinkingData {
  content: string
  isStreaming?: boolean
}

// 最终答案数据
export interface FinalAnswerData {
  content: string
}

// 错误数据
export interface ErrorData {
  message: string
  code?: string
}

// Agent执行块
export interface AgentBlock {
  id: string
  type: AgentBlockType
  timestamp: Date

  // 各类型的专属数据
  task?: TaskData
  thinking?: ThinkingData
  toolCall?: ToolCallData
  toolResult?: ToolResultData
  finalAnswer?: FinalAnswerData
  error?: ErrorData
}

// 执行状态
export interface AgentExecutionState {
  blocks: AgentBlock[]
  isExecuting: boolean
  currentBlockId?: string
}

// ============ 辅助函数 ============

// 创建消息
export function createAgentMessage(type: MessageType, content: string, metadata?: MessageMetadata): AgentMessage {
  return {
    id: crypto.randomUUID(),
    type,
    content,
    timestamp: Date.now(),
    metadata,
  }
}

// 判断消息是否为工具相关
export function isToolMessage(message: AgentMessage): boolean {
  return message.type === 'tool_call' || message.type === 'tool_result'
}

// 获取消息类型显示名称
export function getMessageTypeName(type: MessageType): string {
  const names: Record<MessageType, string> = {
    user: 'User',
    thinking: 'Thinking',
    planning: 'Planning',
    tool_call: 'Tool Call',
    tool_result: 'Result',
    progress: 'Progress',
    final: 'Answer',
    error: 'Error',
    system: 'System',
  }
  return names[type]
}

// 获取消息类型图标（Unicode）
export function getMessageTypeIcon(type: MessageType): string {
  const icons: Record<MessageType, string> = {
    user: '👤',
    thinking: '💭',
    planning: '📋',
    tool_call: '🔧',
    tool_result: '📤',
    progress: '⏳',
    final: '✅',
    error: '❌',
    system: 'ℹ️',
  }
  return icons[type]
}
