/**
 * ReAct 消息处理器
 *
 * 从 chunks 中的 Meta 数据构建步骤显示，字段与后端 ReactMessageStep 对齐
 */

import type { OrderedMessageChunk } from '../../types/ordered-chat'
import type { ReActStepDisplay } from '../../types/react'
import type { ChatMessage } from '../../types/chat'

/**
 * 后端发送的步骤数据（与 message_emitter.rs 中的 ReactMessageStep 对齐）
 */
interface ReactMessageStep {
  index: number
  thought?: string
  action?: {
    tool: string
    args: Record<string, any>
    status: string
  }
  observation?: any
  final_answer?: string
  error?: string
}

/**
 * ReAct 消息处理器
 */
export class ReActMessageProcessor {
  /**
   * 从消息构建 ReAct 步骤显示数据
   * 简化版：只从 reactSteps 获取，由 useOrderedMessages 负责解析
   */
  static buildReActStepsFromMessage(message: ChatMessage): ReActStepDisplay[] {
    // 从 reactSteps 获取（由 useOrderedMessages 从 chunks 实时解析）
    if (message.reactSteps && Array.isArray(message.reactSteps) && message.reactSteps.length > 0) {
      return message.reactSteps as unknown as ReActStepDisplay[]
    }
    return []
  }

  /**
   * 从 chunks 构建步骤（流式更新时使用）
   * 字段已与后端对齐，直接映射
   */
  static extractStepsFromChunks(chunks: OrderedMessageChunk[]): ReActStepDisplay[] {
    const stepMap = new Map<number, ReActStepDisplay>()

    // 按序列号排序
    const sortedChunks = [...chunks].sort((a, b) => a.sequence - b.sequence)

    for (const chunk of sortedChunks) {
      if (chunk.chunk_type !== 'Meta' || !chunk.structured_data) continue

      const sd = chunk.structured_data as any
      if (sd.type !== 'step' || !sd.step) continue

      const stepData = sd.step as ReactMessageStep
      this.mergeStep(stepMap, stepData)
    }

    // 转换为数组并按 index 排序
    return Array.from(stepMap.values()).sort((a, b) => a.index - b.index)
  }

  /**
   * 合并步骤数据（字段已对齐，直接映射）
   */
  private static mergeStep(
    stepMap: Map<number, ReActStepDisplay>,
    data: ReactMessageStep
  ): void {
    const index = data.index
    let step = stepMap.get(index)

    if (!step) {
      step = {
        index,
        id: `step-${index}`,
      }
      stepMap.set(index, step)
    }

    // 合并 thought（如果后端发送了）
    if (data.thought) {
      step.thought = data.thought
    }

    // 合并 action（字段已对齐）
    if (data.action) {
      step.action = {
        tool: data.action.tool,
        args: data.action.args || {},
        status: this.normalizeStatus(data.action.status),
      }
    }

    // 合并 observation（字段已对齐）
    if (data.observation !== undefined) {
      step.observation = data.observation
      // 更新 action 状态
      if (step.action && !step.action.status?.match(/completed|failed/)) {
        const success = typeof data.observation === 'object' 
          ? data.observation?.success !== false 
          : true
        step.action.status = success ? 'completed' : 'failed'
      }
    }

    // 合并 final_answer
    if (data.final_answer !== undefined) {
      step.finalAnswer = data.final_answer || undefined
    }

    // 合并 error
    if (data.error) {
      step.error = data.error
    }
  }

  /**
   * 规范化状态
   */
  private static normalizeStatus(
    status?: string
  ): 'pending' | 'running' | 'completed' | 'failed' | 'error' | undefined {
    if (!status) return undefined

    const map: Record<string, 'pending' | 'running' | 'completed' | 'failed' | 'error'> = {
      pending: 'pending',
      running: 'running',
      completed: 'completed',
      success: 'completed',
      failed: 'failed',
      error: 'failed',
    }

    return map[status.toLowerCase()] || 'pending'
  }

  // === 工具函数 ===

  /**
   * 检查 observation 是否包含错误
   */
  static hasObservationError(observation: any): boolean {
    if (!observation) return false

    if (typeof observation === 'string') {
      const lower = observation.toLowerCase()
      return (
        lower.includes('error') ||
        lower.includes('failed') ||
        lower.includes('"success":false') ||
        lower.includes('"success": false')
      )
    }

    if (typeof observation === 'object') {
      return observation.success === false || !!observation.error
    }

    return false
  }

  /**
   * 格式化 observation
   */
  static formatObservation(observation: any): string {
    if (typeof observation === 'string') return observation

    try {
      return JSON.stringify(observation, null, 2)
    } catch {
      return String(observation)
    }
  }

  /**
   * 格式化 JSON
   */
  static formatJson(obj: any): string {
    try {
      return JSON.stringify(obj, null, 2)
    } catch {
      return String(obj)
    }
  }

  /**
   * 判断工具调用是否应该折叠显示
   */
  static shouldCollapseToolCall(action: any): boolean {
    if (!action) return false
    // 已完成或失败的调用默认折叠
    return action.status === 'completed' || action.status === 'failed'
  }
}
