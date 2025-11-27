/**
 * ReAct 架构消息处理器
 *
 * 负责处理 ReAct 架构特有的消息流、数据转换和展示逻辑
 * 与 useOrderedMessages 的通用消息处理器分离，独立处理 ReAct 逻辑
 */

import type { OrderedMessageChunk, ArchitectureType } from '../../types/ordered-chat'
import type {
  ReActStep,
  ReActStepDisplay,
  ReActArchitectureMeta,
  ReActStepType,
} from '../../types/react'
import type { ChatMessage } from '../../types/chat'

/**
 * ReAct 消息处理器
 * 从消息块构建完整的 ReAct 步骤信息
 */
export class ReActMessageProcessor {
  /**
   * 从原始块数据和架构元数据构建 ReAct 步骤显示数据
   *
   * @param message - 聊天消息对象
   * @returns ReAct 步骤显示数组
   */
  static buildReActStepsFromMessage(message: ChatMessage): ReActStepDisplay[] {
    const steps: ReActStepDisplay[] = []

    // 优先从 architectureMeta 中获取结构化步骤数据
    if (message.architectureMeta) {
      const meta = message.architectureMeta as ReActArchitectureMeta
      if (meta.steps && Array.isArray(meta.steps)) {
        steps.push(...this.parseStructuredSteps(meta.steps))
      }
    }

    // 如果没有从 meta 获取到步骤，尝试从 reactSteps 解析
    if (steps.length === 0 && message.reactSteps && Array.isArray(message.reactSteps)) {
      steps.push(...this.parseReActStepsLegacy(message.reactSteps))
    }

    return steps
  }

  /**
   * 从结构化元数据解析步骤
   *
   * @param structuredSteps - 结构化的步骤数据
   * @returns 前端步骤显示数组
   */
  private static parseStructuredSteps(
    structuredSteps: Array<{
      thought?: string
      action?: { tool: string; args: any; status: string }
      observation?: any
      finalAnswer?: string
      citations?: string[]
      error?: { type: string; message: string; retryable: boolean }
    }>
  ): ReActStepDisplay[] {
    const steps: ReActStepDisplay[] = []
    let current: ReActStepDisplay = { index: 0 }
    let idx = 0

    for (const s of structuredSteps) {
      if (s.thought) {
        if (current.action || current.observation || current.finalAnswer) {
          current.id = `react-step-${idx}`
          steps.push(current)
          idx += 1
          current = { index: idx }
        }
        current.thought = (current.thought || '') + s.thought
      }

      if (s.action) {
        current.action = {
          tool: s.action.tool,
          args: s.action.args,
          status: this.normalizeActionStatus(s.action.status),
        }
      }

      if (s.observation !== undefined) {
        current.observation = s.observation
      }

      if (s.finalAnswer) {
        current.finalAnswer = (current.finalAnswer || '') + s.finalAnswer
      }

      if (s.error) {
        current.error = s.error.message
      }
    }

    if (Object.keys(current).length > 1 || current.thought || current.finalAnswer) {
      current.id = `react-step-${idx}`
      steps.push(current)
    }

    return steps
  }

  /**
   * 从遗留的 reactSteps 数据解析步骤（向后兼容）
   *
   * @param reactSteps - 遗留的反应步骤数据
   * @returns 前端步骤显示数组
   */
  private static parseReActStepsLegacy(
    reactSteps: Array<{
      thought?: string
      action?: any
      observation?: any
      error?: string
      finalAnswer?: string
    }>
  ): ReActStepDisplay[] {
    const steps: ReActStepDisplay[] = []

    reactSteps.forEach((step, index) => {
      const display: ReActStepDisplay = {
        index,
        id: `react-step-${index}`,
      }

      if (step.thought) {
        display.thought = step.thought
      }

      if (step.action) {
        display.action = this.parseActionFromAny(step.action)
      }

      if (step.observation !== undefined) {
        display.observation = step.observation
      }

      if (step.error) {
        display.error = step.error
      }

      if (step.finalAnswer) {
        display.finalAnswer = step.finalAnswer
      }

      steps.push(display)
    })

    return steps
  }

  /**
   * 从任意对象解析 action 信息
   *
   * @param action - action 对象（可能是字符串或对象）
   * @returns 标准化的 action 信息
   */
  private static parseActionFromAny(action: any): ReActStepDisplay['action'] | undefined {
    if (!action) return undefined

    let parsed: any = action
    if (typeof action === 'string') {
      try {
        parsed = JSON.parse(action)
      } catch {
        return {
          tool: action,
          args: {},
        }
      }
    }

    if (typeof parsed === 'object') {
      return {
        tool: parsed.tool || '',
        args: parsed.args || {},
        status: parsed.status ? this.normalizeActionStatus(parsed.status) : undefined,
      }
    }

    return undefined
  }

  /**
   * 规范化 action 状态字符串
   *
   * @param status - 原始状态字符串
   * @returns 规范化的状态
   */
  private static normalizeActionStatus(
    status: string
  ): 'pending' | 'running' | 'completed' | 'success' | 'failed' | 'error' {
    const normalized = status.toLowerCase().trim()

    const statusMap: Record<
      string,
      'pending' | 'running' | 'completed' | 'success' | 'failed' | 'error'
    > = {
      pending: 'pending',
      running: 'running',
      'in-progress': 'running',
      inprogress: 'running',
      completed: 'completed',
      complete: 'completed',
      success: 'success',
      successful: 'success',
      failed: 'failed',
      failure: 'failed',
      error: 'error',
    }

    return statusMap[normalized] || 'pending'
  }

  /**
   * 检查是否应该折叠 tool call 详情（基于 action 状态）
   *
   * @param action - action 信息
   * @returns 是否应该折叠
   */
  static shouldCollapseToolCall(action: ReActStepDisplay['action']): boolean {
    if (!action) return true
    const status = action.status
    // 运行中或待处理时保持展开，其他情况折叠
    return status !== 'running' && status !== 'pending'
  }

  /**
   * 检查 observation 中是否包含错误
   *
   * @param observation - 观察数据
   * @returns 是否包含错误
   */
  static hasObservationError(observation: any): boolean {
    if (!observation) return false

    if (typeof observation === 'string') {
      const lower = observation.toLowerCase()
      return (
        lower.includes('error') ||
        lower.includes('failed') ||
        lower.includes('失败') ||
        lower.includes('"success":false') ||
        lower.includes('"success": false')
      )
    }

    if (typeof observation === 'object') {
      return observation.success === false || observation.error
    }

    return false
  }

  /**
   * 格式化 observation 为可读的字符串
   *
   * @param observation - 观察数据
   * @returns 格式化的字符串
   */
  static formatObservation(observation: any): string {
    if (typeof observation === 'string') {
      return observation
    }

    try {
      return JSON.stringify(observation, null, 2)
    } catch {
      return String(observation)
    }
  }

  /**
   * 格式化参数为可显示的对象
   *
   * @param args - 参数对象或字符串
   * @returns 格式化后的参数对象
   */
  static formatParams(args: any): Record<string, any> {
    if (!args) return {}

    if (typeof args === 'object') {
      return args
    }

    try {
      return JSON.parse(args)
    } catch {
      return { value: args }
    }
  }

  /**
   * 格式化 JSON 为可读的字符串
   *
   * @param obj - 任意对象
   * @returns 格式化的 JSON 字符串
   */
  static formatJson(obj: any): string {
    try {
      return JSON.stringify(obj, null, 2)
    } catch {
      return String(obj)
    }
  }

  /**
   * 从消息块数组中提取 ReAct 步骤（用于从未处理完的流中重建）
   *
   * @param chunks - 消息块数组
   * @returns ReAct 步骤数组
   */
  static extractStepsFromChunks(chunks: OrderedMessageChunk[]): ReActStepDisplay[] {
    const steps: ReActStepDisplay[] = []
    const validChunks = chunks
      .filter(c => ['Thinking', 'ToolResult', 'Content', 'Meta'].includes(c.chunk_type))
      .sort((a, b) => a.sequence - b.sequence)

    let currentStep: ReActStepDisplay = { index: 0 }
    let stepIndex = 0

    for (const chunk of validChunks) {
      if (chunk.chunk_type === 'Thinking') {
        if (currentStep.action || currentStep.observation) {
          currentStep.id = `react-step-${stepIndex}`
          steps.push(currentStep)
          stepIndex += 1
          currentStep = { index: stepIndex }
        }
        const raw = chunk.content.toString()
        const text = raw.replace(/^Thought:\s*/i, '')
        currentStep.thought = (currentStep.thought || '') + text

        const actionMatch = raw.match(/Action\s*:\s*([^\n]+)/i)
        const inputMatch = raw.match(/Action\s*Input\s*:\s*([\s\S]+?)(?:\n\n|$)/i)
        if (actionMatch) {
          const tool = actionMatch[1].trim()
          let args: any = {}
          if (inputMatch) {
            const inputStr = inputMatch[1].trim()
            try {
              args = JSON.parse(inputStr)
            } catch {
              args = { query: inputStr }
            }
          }
          currentStep.action = {
            tool,
            args,
            status: currentStep.action?.status || 'pending',
          }
        }
      } else if (chunk.chunk_type === 'Meta') {
        const sd: any = chunk.structured_data
        if (sd && sd.type === 'step_update' && sd.status === 'executing') {
          const tool = sd.step_name || 'Unknown'
          currentStep.action = {
            tool,
            args: currentStep.action?.args || {},
            status: 'running',
          }
        }
      } else if (chunk.chunk_type === 'ToolResult') {
        const toolName = chunk.tool_name || 'Unknown'
        if (currentStep.observation && currentStep.action?.tool !== toolName) {
          currentStep.id = `react-step-${stepIndex}`
          steps.push(currentStep)
          stepIndex += 1
          currentStep = { index: stepIndex }
        }
        const sd: any = chunk.structured_data
        let status: any = 'success'
        if (sd && (sd.success === false || sd.error)) {
          status = 'failed'
        } else {
          const contentLower = (chunk.content || '').toString().toLowerCase()
          if (contentLower.includes('"success":false') || contentLower.includes('error')) {
            status = 'failed'
          }
        }
        currentStep.action = {
          tool: toolName,
          args: currentStep.action?.args || {},
          status,
        }
        if (sd && (sd.result || sd.output)) {
          currentStep.observation = sd.result || sd.output
        } else if (typeof currentStep.observation === 'string') {
          currentStep.observation += chunk.content.toString()
        } else {
          currentStep.observation = chunk.content
        }
      } else if (chunk.chunk_type === 'Content') {
        const contentStr = chunk.content.toString()
        currentStep.finalAnswer = (currentStep.finalAnswer || '') + contentStr
      }
    }

    if (Object.keys(currentStep).length > 1 || currentStep.thought || currentStep.finalAnswer) {
      currentStep.id = `react-step-${stepIndex}`
      steps.push(currentStep)
    }

    return steps
  }

  /**
   * 获取步骤的图标
   *
   * @param stepType - 步骤类型
   * @returns 图标字符串
   */
  static getStepIcon(stepType: ReActStepType | string): string {
    const typeStr = String(stepType).toLowerCase()

    const iconMap: Record<string, string> = {
      thought: '🤔',
      action: '🔧',
      observation: '👁️',
      final: '🏁',
      error: '❌',
    }

    return iconMap[typeStr] || '⚙️'
  }

  /**
   * 获取步骤的状态标签
   *
   * @param status - 状态字符串
   * @returns 中文状态标签
   */
  static getStatusLabel(status?: string): string {
    if (!status) return '待处理'

    const statusMap: Record<string, string> = {
      pending: '待处理',
      running: '运行中',
      'in-progress': '运行中',
      completed: '已完成',
      complete: '已完成',
      success: '成功',
      successful: '成功',
      failed: '失败',
      error: '错误',
    }

    return statusMap[status.toLowerCase()] || status
  }
}
