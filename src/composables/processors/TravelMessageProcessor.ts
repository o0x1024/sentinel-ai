/**
 * Travel OODA 消息处理器
 *
 * 从 chunks 中的 Meta 数据构建步骤显示
 * 支持 OODA executor 和 ReAct executor 发送的消息格式
 */

import type { OrderedMessageChunk } from '../../types/ordered-chat'
import type { ChatMessage } from '../../types/chat'

/**
 * 后端发送的 OODA 步骤数据（与 message_emitter.rs 中的 TravelPhaseStep 对齐）
 */
export interface TravelPhaseStep {
  cycle: number
  phase: 'Observe' | 'Orient' | 'Decide' | 'Act' | string
  status: 'running' | 'completed' | 'failed'
  thought?: string
  action?: TravelAction
  output?: any
  error?: string
}

export interface TravelAction {
  tool: string
  args: Record<string, any>
  status: 'running' | 'completed' | 'failed'
  result?: any
}

/**
 * 前端显示用的 OODA 循环数据
 */
export interface TravelCycleDisplay {
  cycle: number
  phases: TravelPhaseDisplay[]
  status: 'running' | 'completed' | 'failed'
}

export interface TravelPhaseDisplay {
  phase: string
  status: 'running' | 'completed' | 'failed'
  thoughts: string[]
  actions: TravelActionDisplay[]
  output?: any
  error?: string
}

export interface TravelActionDisplay {
  tool: string
  args: Record<string, any>
  status: 'running' | 'completed' | 'failed'
  result?: any
}

/**
 * Travel 消息处理器
 */
export class TravelMessageProcessor {
  /**
   * 从消息构建 Travel 循环显示数据
   */
  static buildCyclesFromMessage(message: ChatMessage): TravelCycleDisplay[] {
    if ((message as any).travelCycles && Array.isArray((message as any).travelCycles)) {
      return (message as any).travelCycles as TravelCycleDisplay[]
    }
    return []
  }

  /**
   * 从 chunks 提取 OODA 循环数据（流式更新时使用）
   */
  static extractCyclesFromChunks(chunks: OrderedMessageChunk[]): TravelCycleDisplay[] {
    const cycleMap = new Map<number, TravelCycleDisplay>()

    // 按序列号排序，确保增量更新顺序正确
    const sortedChunks = [...chunks].sort((a, b) => a.sequence - b.sequence)

    for (const chunk of sortedChunks) {
      // 只处理 Meta 类型且带 Travel 架构标识的 chunk
      if (chunk.chunk_type !== 'Meta' || chunk.architecture !== 'Travel') continue
      if (!chunk.structured_data) continue

      const sd = chunk.structured_data as any
      if (sd.type !== 'ooda_step' || !sd.step) continue

      const step = sd.step as TravelPhaseStep
      this.mergeStep(cycleMap, step, chunk.sequence)
    }

    // 转换为数组并按 cycle 排序
    return Array.from(cycleMap.values()).sort((a, b) => a.cycle - b.cycle)
  }

  /**
   * 合并步骤数据到循环（增量更新）
   */
  private static mergeStep(
    cycleMap: Map<number, TravelCycleDisplay>,
    step: TravelPhaseStep,
    sequence: number
  ): void {
    const cycleNum = step.cycle || 1  // 默认循环号为 1
    let cycle = cycleMap.get(cycleNum)

    if (!cycle) {
      cycle = {
        cycle: cycleNum,
        phases: [],
        status: 'running',
      }
      cycleMap.set(cycleNum, cycle)
    }

    // 查找或创建阶段
    let phase = cycle.phases.find(p => p.phase === step.phase)
    if (!phase) {
      phase = {
        phase: step.phase,
        status: 'running',
        thoughts: [],
        actions: [],
      }
      cycle.phases.push(phase)
    }

    // 合并思考内容（去重，避免重复显示）
    if (step.thought) {
      // 检查是否已有相同内容（基于内容前缀匹配，避免重复）
      const isDuplicate = phase.thoughts.some(t => 
        t === step.thought || 
        t.startsWith(step.thought!.substring(0, 50)) ||
        step.thought!.startsWith(t.substring(0, 50))
      )
      if (!isDuplicate) {
        phase.thoughts.push(step.thought)
      }
    }

    // 合并工具调用（基于工具名+参数匹配，支持增量更新结果）
    if (step.action) {
      // 查找匹配的工具调用（同一工具、同一参数）
      const existingAction = phase.actions.find(a => {
        if (a.tool !== step.action!.tool) return false
        // 如果是结果更新（有 result），匹配最近的同名工具
        if (step.action!.result !== undefined) {
          return a.status === 'running' || a.result === undefined
        }
        // 否则检查参数是否相同
        return JSON.stringify(a.args) === JSON.stringify(step.action!.args)
      })
      
      if (existingAction) {
        // 增量更新现有工具调用
        existingAction.status = step.action.status
        if (step.action.result !== undefined) {
          existingAction.result = step.action.result
        }
        // 合并参数（结果更新时可能没有带 args）
        if (step.action.args && Object.keys(step.action.args).length > 0) {
          existingAction.args = { ...existingAction.args, ...step.action.args }
        }
      } else {
        // 添加新工具调用
        phase.actions.push({
          tool: step.action.tool,
          args: step.action.args || {},
          status: step.action.status,
          result: step.action.result,
        })
      }
    }

    // 更新阶段状态
    if (step.status === 'completed') {
      phase.status = 'completed'
      if (step.output !== undefined) {
        phase.output = step.output
      }
    } else if (step.status === 'failed') {
      phase.status = 'failed'
      if (step.error) {
        phase.error = step.error
      }
    }

    // 更新循环状态
    if (step.phase === 'Act' && step.status === 'completed') {
      cycle.status = 'completed'
    } else if (step.status === 'failed') {
      cycle.status = 'failed'
    }
  }

  // === 工具函数 ===

  /**
   * 获取阶段图标
   */
  static getPhaseIcon(phase: string): string {
    const icons: Record<string, string> = {
      Observe: 'fas fa-eye',
      Orient: 'fas fa-compass',
      Decide: 'fas fa-brain',
      Act: 'fas fa-bolt',
    }
    return icons[phase] || 'fas fa-circle'
  }

  /**
   * 获取阶段中文名
   */
  static getPhaseName(phase: string): string {
    const names: Record<string, string> = {
      Observe: '观察',
      Orient: '定位',
      Decide: '决策',
      Act: '执行',
    }
    return names[phase] || phase
  }

  /**
   * 获取状态图标
   */
  static getStatusIcon(status: string): string {
    const icons: Record<string, string> = {
      running: 'fas fa-circle-notch fa-spin text-warning',
      completed: 'fas fa-check-circle text-success',
      failed: 'fas fa-times-circle text-error',
    }
    return icons[status] || 'fas fa-circle'
  }

  /**
   * 获取状态文本
   */
  static getStatusText(status: string): string {
    const texts: Record<string, string> = {
      running: '运行中',
      completed: '已完成',
      failed: '失败',
    }
    return texts[status] || status
  }

  /**
   * 格式化 JSON
   */
  static formatJson(obj: any): string {
    try {
      if (typeof obj === 'string') return obj
      return JSON.stringify(obj, null, 2)
    } catch {
      return String(obj)
    }
  }

  /**
   * 检查结果是否包含错误
   */
  static hasError(result: any): boolean {
    if (!result) return false
    if (typeof result === 'object') {
      return result.success === false || !!result.error
    }
    if (typeof result === 'string') {
      const lower = result.toLowerCase()
      return lower.includes('error') || lower.includes('failed')
    }
    return false
  }
}

