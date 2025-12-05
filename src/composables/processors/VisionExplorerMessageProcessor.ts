/**
 * VisionExplorer 消息处理器
 *
 * 从 chunks 中的 Meta 数据构建探索步骤显示
 */

import type { OrderedMessageChunk } from '../../types/ordered-chat'
import type { ChatMessage } from '../../types/chat'

/**
 * 后端发送的 Vision 步骤数据（与 message_emitter.rs 对齐）
 */
export interface VisionStep {
  iteration: number
  phase: 'screenshot' | 'analyze' | 'action' | 'verify' | string
  status: 'running' | 'completed' | 'failed'
  url?: string
  title?: string
  screenshot?: string
  analysis?: VisionAnalysis
  action?: VisionAction
  error?: string
}

export interface VisionAnalysis {
  page_analysis: string
  estimated_apis?: string[]
  exploration_progress: number
}

export interface VisionAction {
  action_type: string
  element_index?: number
  value?: string
  reason: string
  success: boolean
  duration_ms?: number
}

/**
 * 前端显示用的迭代数据
 */
export interface VisionIterationDisplay {
  iteration: number
  url?: string
  title?: string
  phases: VisionPhaseDisplay[]
  status: 'running' | 'completed' | 'failed'
}

export interface VisionPhaseDisplay {
  phase: string
  status: 'running' | 'completed' | 'failed'
  analysis?: VisionAnalysis
  action?: VisionAction
  error?: string
}

/**
 * 探索统计
 */
export interface VisionExplorationStats {
  total_iterations: number
  pages_visited: number
  apis_discovered: number
  elements_interacted: number
  total_duration_ms: number
  status: string
}

/**
 * VisionExplorer 消息处理器
 */
export class VisionExplorerMessageProcessor {
  /**
   * 从消息构建 Vision 迭代显示数据
   */
  static buildIterationsFromMessage(message: ChatMessage): VisionIterationDisplay[] {
    if ((message as any).visionIterations && Array.isArray((message as any).visionIterations)) {
      return (message as any).visionIterations as VisionIterationDisplay[]
    }
    return []
  }

  /**
   * 从 chunks 提取迭代数据（流式更新时使用）
   * 支持独立运行 (architecture: VisionExplorer) 和作为 Travel 子流运行 (architecture: Travel)
   */
  static extractIterationsFromChunks(chunks: OrderedMessageChunk[]): VisionIterationDisplay[] {
    const iterationMap = new Map<number, VisionIterationDisplay>()

    // 按序列号排序，确保增量更新顺序正确
    const sortedChunks = [...chunks].sort((a, b) => a.sequence - b.sequence)

    for (const chunk of sortedChunks) {
      // 只处理 Meta 类型的 chunk
      if (chunk.chunk_type !== 'Meta') continue
      if (!chunk.structured_data) continue
      
      // 支持 VisionExplorer 独立运行或作为 Travel 子流运行
      // 关键是 structured_data.type === 'vision_step'
      const sd = chunk.structured_data as any
      if (sd.type !== 'vision_step' || !sd.step) continue

      const step = sd.step as VisionStep
      this.mergeStep(iterationMap, step)
    }

    // 转换为数组并按 iteration 排序
    return Array.from(iterationMap.values()).sort((a, b) => a.iteration - b.iteration)
  }
  
  /**
   * 检查 chunks 中是否包含 VisionExplorer 数据
   * 用于快速判断是否需要显示 VisionExplorer 组件
   */
  static hasVisionData(chunks: OrderedMessageChunk[]): boolean {
    return chunks.some(chunk => {
      if (chunk.chunk_type !== 'Meta') return false
      if (!chunk.structured_data) return false
      const sd = chunk.structured_data as any
      return sd.type === 'vision_step'
    })
  }

  /**
   * 从 chunks 提取统计数据
   * 支持独立运行和作为 Travel 子流运行
   */
  static extractStatsFromChunks(chunks: OrderedMessageChunk[]): VisionExplorationStats | null {
    for (const chunk of chunks) {
      // 只检查 StreamComplete 类型，不限制架构（支持作为 Travel 子流）
      if (chunk.chunk_type !== 'StreamComplete') continue
      if (!chunk.structured_data) continue

      const sd = chunk.structured_data as any
      if (sd.type === 'complete' && sd.statistics) {
        return sd.statistics as VisionExplorationStats
      }
    }
    return null
  }

  /**
   * 合并步骤数据到迭代（增量更新）
   */
  private static mergeStep(
    iterationMap: Map<number, VisionIterationDisplay>,
    step: VisionStep
  ) {
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

  /**
   * 获取阶段图标
   */
  static getPhaseIcon(phase: string): string {
    switch (phase) {
      case 'screenshot': return '📸'
      case 'analyze': return '🧠'
      case 'action': return '🎯'
      case 'verify': return '✅'
      default: return '⚙️'
    }
  }

  /**
   * 获取状态图标
   */
  static getStatusIcon(status: string): string {
    switch (status) {
      case 'running': return '⏳'
      case 'completed': return '✅'
      case 'failed': return '❌'
      default: return '⚙️'
    }
  }

  /**
   * 格式化进度百分比
   */
  static formatProgress(progress: number): string {
    return `${Math.round(progress * 100)}%`
  }

  /**
   * 格式化 API 列表
   */
  static formatApis(apis?: string[]): string {
    if (!apis || apis.length === 0) return '无'
    return apis.join(', ')
  }
}

