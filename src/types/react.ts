/**
 * ReAct 前端类型定义
 *
 * 核心类型：
 * - ReactMessageStep: 后端发送的消息格式（与 message_emitter.rs 对齐）
 * - ReActStepDisplay: 前端显示格式
 */

/**
 * 步骤状态
 */
export type ReActStepStatus = 'pending' | 'running' | 'completed' | 'failed' | 'error'

/**
 * 后端发送的步骤数据（与 message_emitter.rs 中的 ReactMessageStep 对齐）
 */
export interface ReactMessageStep {
  /** 步骤索引（从 0 开始） */
  index: number
  /** 思考内容 */
  thought?: string
  /** 工具调用 */
  action?: {
    tool: string
    args: Record<string, any>
    status: string
  }
  /** 工具结果 */
  observation?: any
  /** 最终答案 */
  final_answer?: string
  /** 错误信息 */
  error?: string
}

/**
 * 前端步骤显示格式（用于组件渲染）
 */
export interface ReActStepDisplay {
  /** 步骤索引（从 0 开始） */
  index: number
  /** 步骤 ID */
  id?: string
  /** 思考内容 */
  thought?: string
  /** 工具调用 */
  action?: {
    tool: string
    args: Record<string, any>
    status?: ReActStepStatus
  }
  /** 工具结果 */
  observation?: any
  /** 最终答案 */
  finalAnswer?: string
  /** 错误信息 */
  error?: string
}

/**
 * 执行统计（与后端 ReactExecutionStats 对齐）
 */
export interface ReActExecutionStats {
  total_iterations: number
  tool_calls_count: number
  successful_tool_calls: number
  failed_tool_calls: number
  total_duration_ms: number
  status: string
}

/**
 * 架构元数据
 */
export interface ReActArchitectureMeta {
  type: 'ReAct'
  statistics?: ReActExecutionStats
}
