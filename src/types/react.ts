/**
 * ReAct 架构前端类型定义
 * 与后端 src-tauri/src/engines/react/types.rs 保持一致
 */

/**
 * ReAct 步骤的状态枚举
 */
export enum ReActStepStatus {
  Pending = 'Pending',
  Running = 'Running',
  Completed = 'Completed',
  Failed = 'Failed',
  InProgress = 'InProgress',
}

/**
 * ReAct 步骤类型枚举
 */
export enum ReActStepType {
  Thought = 'Thought',
  Action = 'Action',
  Observation = 'Observation',
  Final = 'Final',
  Error = 'Error',
}

/**
 * ReAct 工具调用结构
 */
export interface ReActToolCall {
  /** 工具名称 */
  tool: string
  /** 工具参数（JSON） */
  args: Record<string, any>
  /** 调用ID（用于追踪） */
  call_id: string
  /** 是否为并行调用批次的一部分 */
  is_parallel: boolean
}

/**
 * ReAct 思考步骤
 */
export interface ReActThoughtStep {
  type: ReActStepType.Thought
  content: string
  has_rag_context: boolean
}

/**
 * ReAct 行动步骤
 */
export interface ReActActionStep {
  type: ReActStepType.Action
  tool_call: ReActToolCall
}

/**
 * ReAct 观察步骤
 */
export interface ReActObservationStep {
  type: ReActStepType.Observation
  tool_name: string
  result: any
  success: boolean
}

/**
 * ReAct 最终答案步骤
 */
export interface ReActFinalStep {
  type: ReActStepType.Final
  answer: string
  citations: string[]
}

/**
 * ReAct 错误步骤
 */
export interface ReActErrorStep {
  type: ReActStepType.Error
  error_type: string
  message: string
  retryable: boolean
}

/**
 * ReAct 步骤联合类型
 */
export type ReActStepVariant =
  | ReActThoughtStep
  | ReActActionStep
  | ReActObservationStep
  | ReActFinalStep
  | ReActErrorStep

/**
 * ReAct 完整步骤数据（前端展示用）
 */
export interface ReActStep {
  /** 步骤ID */
  id: string
  /** 步骤类型 */
  step_type: ReActStepType
  /** 步骤状态 */
  status?: ReActStepStatus
  /** 时间戳 */
  timestamp?: string
  /** 执行耗时（毫秒） */
  duration_ms?: number
  /** 步骤具体数据 */
  data?: ReActStepVariant
  /** 错误信息 */
  error?: string
}

/**
 * ReAct 消息块的扩展数据
 */
export interface ReActMessageChunkData {
  /** 步骤ID（可选） */
  step_id?: string
  /** 步骤类型 */
  step_type?: ReActStepType
  /** 步骤状态 */
  step_status?: ReActStepStatus
  /** 工具名称（用于ToolResult块） */
  tool_name?: string
  /** 执行结果 */
  execution_result?: any
  /** 执行是否成功 */
  execution_success?: boolean
}

/**
 * ReAct 前端步骤表示（用于组件显示）
 */
export interface ReActStepDisplay {
  /** 步骤索引 */
  index: number
  /** 思考内容 */
  thought?: string
  /** 行动信息 */
  action?: {
    tool: string
    args: Record<string, any>
    status?: 'pending' | 'running' | 'completed' | 'success' | 'failed' | 'error'
  }
  /** 观察信息 */
  observation?: any
  /** 错误信息 */
  error?: string
  /** 最终答案 */
  finalAnswer?: string
  /** 时间戳 */
  timestamp?: string
  /** 步骤ID */
  id?: string
}

/**
 * ReAct 执行统计信息
 */
export interface ReActMetrics {
  /** 总迭代次数 */
  total_iterations: number
  /** 工具调用次数 */
  tool_calls_count: number
  /** 总耗时（毫秒） */
  total_duration_ms: number
  /** Token 总量 */
  total_tokens: number
  /** 成功的工具调用 */
  successful_tool_calls: number
  /** 失败的工具调用 */
  failed_tool_calls: number
  /** 重试次数 */
  retry_count: number
}

/**
 * ReAct 架构元数据
 */
export interface ReActArchitectureMeta {
  type: 'ReAct'
  statistics?: {
    total_iterations: number
    tool_calls_count: number
    successful_tool_calls: number
    failed_tool_calls: number
    total_duration_ms: number
    status: string
  }
  steps?: Array<{
    thought?: string
    action?: {
      tool: string
      args: Record<string, any>
      status: string
    }
    observation?: any
    finalAnswer?: string
    citations?: string[]
    error?: {
      type: string
      message: string
      retryable: boolean
    }
  }>
}
