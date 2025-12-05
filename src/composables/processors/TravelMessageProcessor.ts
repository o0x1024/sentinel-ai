/**
 * Travel OODA 消息处理器 (重构版)
 *
 * 从 chunks 中的 Meta 数据构建步骤显示
 * 支持 OODA executor 和 ReAct executor 发送的消息格式
 * 支持嵌入的 VisionExplorer 迭代数据
 */

import type { OrderedMessageChunk } from '../../types/ordered-chat'
import type { ChatMessage } from '../../types/chat'

/**
 * 后端发送的 OODA 步骤数据（与 message_emitter.rs 中的 OodaStep 对齐）
 */
export interface TravelPhaseStep {
  cycle: number
  phase: 'Observe' | 'Orient' | 'Decide' | 'Act' | string
  status: 'running' | 'completed' | 'failed'
  /** ReAct 迭代号（在 Act 阶段的 ReAct 执行中使用） */
  react_iteration?: number
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
  /** 该阶段的思考内容列表 */
  thoughts: string[]
  /** 该阶段的工具调用列表 */
  actions: TravelActionDisplay[]
  /** 阶段输出 */
  output?: any
  /** 阶段错误 */
  error?: string
  /** ReAct 迭代号 */
  reactIteration?: number
}

export interface TravelActionDisplay {
  tool: string
  args: Record<string, any>
  status: 'running' | 'completed' | 'failed'
  result?: any
  /** 工具调用唯一ID（用于增量匹配） */
  callId?: string
}

/**
 * 嵌入的 VisionExplorer 迭代（与 VisionExplorerMessageProcessor 类型对齐）
 */
export interface EmbeddedVisionIteration {
  iteration: number
  url?: string
  title?: string
  phases: EmbeddedVisionPhase[]
  status: 'running' | 'completed' | 'failed'
}

export interface EmbeddedVisionPhase {
  phase: string
  status: 'running' | 'completed' | 'failed'
  analysis?: {
    page_analysis: string
    estimated_apis?: string[]
    exploration_progress: number
  }
  action?: {
    action_type: string
    element_index?: number
    value?: string
    reason: string
    success: boolean
    duration_ms?: number
  }
  error?: string
}

/**
 * Travel 执行统计
 */
export interface TravelExecutionStats {
  total_iterations: number
  tool_calls_count: number
  successful_tool_calls: number
  failed_tool_calls: number
  total_duration_ms: number
  status: string
}

/**
 * Travel 消息处理器 - 用于从 chunks 构建显示数据
 */
export class TravelMessageProcessor {
  /**
   * 从已保存的消息构建 Travel 循环显示数据
   */
  static buildCyclesFromMessage(message: ChatMessage): TravelCycleDisplay[] {
    if ((message as any).travelCycles && Array.isArray((message as any).travelCycles)) {
      return (message as any).travelCycles as TravelCycleDisplay[]
    }
    return []
  }

  /**
   * 从 chunks 提取 OODA 循环数据（流式更新时使用）
   * 按 sequence 顺序处理，确保增量显示正确
   */
  static extractCyclesFromChunks(chunks: OrderedMessageChunk[]): TravelCycleDisplay[] {
    const cycleMap = new Map<number, TravelCycleDisplay>()

    // 按序列号排序，确保增量更新顺序正确
    const sortedChunks = [...chunks].sort((a, b) => a.sequence - b.sequence)

    for (const chunk of sortedChunks) {
      // 只处理 Meta 类型且带 Travel 架构标识的 chunk
      if (chunk.chunk_type !== 'Meta') continue
      // 支持 architecture 为 Travel 或未指定（兼容旧格式）
      if (chunk.architecture && chunk.architecture !== 'Travel') continue
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
   * 从 chunks 提取嵌入的 VisionExplorer 迭代数据
   * VisionExplorer 作为 Travel 子流运行时，使用 Travel 架构但 type 为 vision_step
   */
  static extractEmbeddedVisionFromChunks(chunks: OrderedMessageChunk[]): EmbeddedVisionIteration[] {
    const iterationMap = new Map<number, EmbeddedVisionIteration>()

    const sortedChunks = [...chunks].sort((a, b) => a.sequence - b.sequence)

    for (const chunk of sortedChunks) {
      if (chunk.chunk_type !== 'Meta') continue
      if (!chunk.structured_data) continue

      const sd = chunk.structured_data as any
      // VisionExplorer 步骤在 Travel 架构下运行时，type 为 vision_step
      if (sd.type !== 'vision_step' || !sd.step) continue

      const step = sd.step as any
      this.mergeVisionStep(iterationMap, step)
    }

    return Array.from(iterationMap.values()).sort((a, b) => a.iteration - b.iteration)
  }

  /**
   * 从 chunks 提取执行统计
   */
  static extractStatsFromChunks(chunks: OrderedMessageChunk[]): TravelExecutionStats | null {
    for (const chunk of chunks) {
      if (chunk.chunk_type !== 'StreamComplete') continue
      if (!chunk.structured_data) continue

      const sd = chunk.structured_data as any
      if (sd.type === 'complete' && sd.statistics) {
        return sd.statistics as TravelExecutionStats
      }
    }
    return null
  }

  /**
   * 合并 OODA 步骤数据到循环（增量更新）
   */
  private static mergeStep(
    cycleMap: Map<number, TravelCycleDisplay>,
    step: TravelPhaseStep,
    sequence: number
  ): void {
    const cycleNum = step.cycle || 1
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
        reactIteration: step.react_iteration,
      }
      cycle.phases.push(phase)
    }

    // 更新 ReAct 迭代号
    if (step.react_iteration !== undefined) {
      phase.reactIteration = step.react_iteration
    }

    // 合并思考内容（去重）
    if (step.thought) {
      const isDuplicate = phase.thoughts.some(t => 
        t === step.thought || 
        (t.length > 50 && step.thought!.length > 50 && 
         (t.startsWith(step.thought!.substring(0, 50)) || step.thought!.startsWith(t.substring(0, 50))))
      )
      if (!isDuplicate) {
        phase.thoughts.push(step.thought)
      }
    }

    // 合并工具调用（基于工具名和参数匹配，支持增量更新结果）
    if (step.action) {
      // 生成工具调用的唯一标识
      const callId = `${step.action.tool}_${sequence}`
      
      // 查找匹配的工具调用
      const existingAction = phase.actions.find(a => {
        // 如果是结果更新（有 result），匹配最近的同名工具
        if (step.action!.result !== undefined) {
          return a.tool === step.action!.tool && 
                 (a.status === 'running' || a.result === undefined)
        }
        // 否则检查参数是否相同
        return a.tool === step.action!.tool && 
               JSON.stringify(a.args) === JSON.stringify(step.action!.args)
      })
      
      if (existingAction) {
        // 增量更新现有工具调用
        existingAction.status = step.action.status
        if (step.action.result !== undefined) {
          existingAction.result = step.action.result
        }
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
          callId,
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

    // 更新循环状态（Act 阶段完成 = 循环完成）
    if (step.phase === 'Act' && step.status === 'completed') {
      cycle.status = 'completed'
    } else if (step.status === 'failed') {
      cycle.status = 'failed'
    }
  }

  /**
   * 合并 VisionExplorer 步骤到迭代
   */
  private static mergeVisionStep(
    iterationMap: Map<number, EmbeddedVisionIteration>,
    step: any
  ): void {
    let iteration = iterationMap.get(step.iteration)
    
    if (!iteration) {
      iteration = {
        iteration: step.iteration,
        url: step.url,
        title: step.title,
        phases: [],
        status: 'running'
      }
      iterationMap.set(step.iteration, iteration)
    }

    // 更新 URL/title
    if (step.url) iteration.url = step.url
    if (step.title) iteration.title = step.title

    // 查找或创建对应的阶段
    let phase = iteration.phases.find(p => p.phase === step.phase)
    if (!phase) {
      phase = {
        phase: step.phase,
        status: step.status
      }
      iteration.phases.push(phase)
    }

    // 更新阶段数据
    phase.status = step.status
    if (step.analysis) phase.analysis = step.analysis
    if (step.action) phase.action = step.action
    if (step.error) phase.error = step.error

    // 更新迭代状态
    if (step.status === 'failed') {
      iteration.status = 'failed'
    } else if (iteration.phases.every(p => p.status === 'completed')) {
      iteration.status = 'completed'
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
   * 获取阶段英文名（用于国际化）
   */
  static getPhaseNameEN(phase: string): string {
    const names: Record<string, string> = {
      Observe: 'Observe',
      Orient: 'Orient',
      Decide: 'Decide',
      Act: 'Act',
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
   * 格式化 JSON 输出
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

  /**
   * 安全截断字符串（考虑 UTF-8 边界）
   */
  static truncateString(str: string, maxLen: number): string {
    if (str.length <= maxLen) return str
    let end = maxLen
    // 确保不在 UTF-8 字符中间截断
    while (end > 0 && str.charCodeAt(end) >= 0xDC00 && str.charCodeAt(end) <= 0xDFFF) {
      end--
    }
    return str.substring(0, end) + '...'
  }
}
