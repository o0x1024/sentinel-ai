// 简化的有序消息处理composable
// 替代复杂的useEventListeners和segments处理

import { ref, Ref } from 'vue'
import { listen } from '@tauri-apps/api/event'
import type { OrderedMessageChunk, ChunkType, MessageChunkProcessor } from '../types/ordered-chat'
import type { ChatMessage } from '../types/chat'
import { parseLLMCompilerMessage } from './useLLMCompilerMessage'
import { parsePlanAndExecuteMessage } from './usePlanAndExecuteMessage'
import { parseReWOOMessage } from './useReWOOMessage'
import { ReActMessageProcessor } from './processors/ReActMessageProcessor'
import { TravelMessageProcessor } from './processors/TravelMessageProcessor'
import { VisionExplorerMessageProcessor } from './processors/VisionExplorerMessageProcessor'

class MessageChunkProcessorImpl implements MessageChunkProcessor {
  chunks = new Map<string, OrderedMessageChunk[]>()
  // 步骤索引：存储每个消息的步骤信息
  private stepsByMessageId = new Map<string, Map<number, any>>()
  // 到达顺序跟踪（按消息ID维度），用于不同 execution_id 的chunk建立稳定全局顺序
  private arrivalCounterByMessageId = new Map<string, number>()
  private chunkArrivalOrder = new Map<string, Map<string, number>>()
  // 新增：持久化架构元数据（不随cleanup清除）
  private architectureInfo = new Map<string, {
    type: string
    planSummary?: any
    statistics?: any
  }>()
  // 新增：流完成状态跟踪
  private streamCompleteFlags = new Map<string, boolean>()

  addChunk(chunk: OrderedMessageChunk): void {
    const messageId = chunk.message_id
    if (!this.chunks.has(messageId)) {
      this.chunks.set(messageId, [])
      this.stepsByMessageId.set(messageId, new Map())
      this.arrivalCounterByMessageId.set(messageId, 0)
      this.chunkArrivalOrder.set(messageId, new Map())
    }

    if (chunk.architecture && !this.architectureInfo.has(messageId)) {
      const info: any = { type: chunk.architecture }
      const sd = chunk.structured_data as any
      if (sd && sd.plan_summary) {
        info.planSummary = sd.plan_summary
      }
      this.architectureInfo.set(messageId, info)
    }

    if (chunk.chunk_type === 'StreamComplete') {
      this.streamCompleteFlags.set(messageId, true)
      if (chunk.structured_data) {
        const existing = this.architectureInfo.get(messageId) || { type: 'Unknown' }
        const sd = chunk.structured_data as any
        this.architectureInfo.set(messageId, {
          ...existing,
          statistics: (sd && sd.summary) ? sd.summary : sd
        })
      }
    }

    const chunks = this.chunks.get(messageId)!
    // 去重与幂等：同一 execution_id + sequence + chunk_type(+tool_name) 视为同一块
    const existingIndex = chunks.findIndex(
      c =>
        c.sequence === chunk.sequence &&
        c.chunk_type === chunk.chunk_type &&
        c.execution_id === chunk.execution_id &&
        (c.tool_name || '') === (chunk.tool_name || '')
    )
    if (existingIndex !== -1) {
      const existed = chunks[existingIndex]
      const prev = (existed.content ?? '').toString()
      const next = (chunk.content ?? '').toString()
      if (prev === next) {
        // 完全重复，直接忽略
      } else {
        // 内容更新：替换原有项，保证顺序不变
        chunks[existingIndex] = { ...existed, ...chunk }
      }
    } else {
      // 按 sequence 插入，保持有序
      const insertIndex = chunks.findIndex(c => c.sequence > chunk.sequence)
      if (insertIndex === -1) {
        chunks.push(chunk)
      } else {
        chunks.splice(insertIndex, 0, chunk)
      }
    }

    // 解析 Meta 事件中的步骤信息
    if (chunk.chunk_type === 'Meta') {
      this.parseStepMeta(messageId, chunk)
    }

    // 记录到达顺序，保证排序稳定（在同一消息内，不同 execution_id 的sequence也按到达顺序稳定）
    const orderMap = this.chunkArrivalOrder.get(messageId)!
    const key = `${chunk.execution_id}#${chunk.sequence}`
    if (!orderMap.has(key)) {
      const next = (this.arrivalCounterByMessageId.get(messageId) || 0) + 1
      this.arrivalCounterByMessageId.set(messageId, next)
      orderMap.set(key, next)
    }
  }

  buildContent(messageId: string): string {
    // 特殊处理：如果消息包含 Orchestrator 事件，返回 bundle 格式
    const chunks = this.chunks.get(messageId) || []
    const orchestratorEvents: string[] = []
    for (const c of chunks) {
      if (c.chunk_type === 'Meta' && c.content) {
        try {
          const obj = JSON.parse(c.content.toString())
          if (obj?.type === 'orchestrator_session' || obj?.type === 'orchestrator_step') {
            orchestratorEvents.push(c.content.toString())
          }
        } catch {
          // ignore
        }
      }
    }
    if (orchestratorEvents.length > 0) {
      return JSON.stringify({
        type: 'orchestrator_bundle',
        events: orchestratorEvents,
      })
    }

    return this.buildStepGroupedContent(messageId)
  }

  // 步骤视图：按步骤分组显示，严格按sequence顺序渲染内容
  private buildStepGroupedContent(messageId: string): string {
    const chunks = this.chunks.get(messageId) || []
    const steps = this.stepsByMessageId.get(messageId) || new Map()

    // 通用的 chunk 过滤：只过滤掉 Meta 块（用于内部追踪）
    // 架构特定的渲染逻辑应在对应的处理器和组件中处理
    const filteredChunks = chunks.filter(chunk => chunk.chunk_type !== 'Meta')

    if (steps.size === 0) {
      // 如果没有步骤信息，直接按sequence顺序渲染所有chunks
      const sorted = filteredChunks.sort((a, b) => a.sequence - b.sequence)
      const parts: string[] = []
      const usedChunks = new Set<number>()
      this.renderChunksInSequenceOrder(sorted, parts, usedChunks)
      return parts.join('')
    }

    const parts: string[] = []
    const sortedSteps = Array.from(steps.entries()).sort(([a], [b]) => a - b)
    const usedChunks = new Set<number>()

    // 添加步骤开始前的内容
    const preStepChunks = filteredChunks.filter(chunk => {
      const minStepSequence = Math.min(
        ...Array.from(steps.values()).map(s => s.start_sequence || Infinity)
      )
      return chunk.sequence < minStepSequence
    })

    this.renderChunksInSequenceOrder(preStepChunks, parts, usedChunks)

    // 按步骤渲染
    for (const [stepIndex, stepInfo] of sortedSteps) {
      // 步骤标题
      const stepIcon = this.getStepIcon(stepInfo.step_type)
      const statusIcon =
        stepInfo.status === 'Completed'
          ? '✅'
          : stepInfo.status === 'Failed'
            ? '❌'
            : stepInfo.status === 'InProgress'
              ? '🔄'
              : '⏳'

      parts.push(
        `\n### ${stepIcon} 步骤 ${stepIndex}: ${stepInfo.step_name || '未命名步骤'} ${statusIcon}\n`
      )

      // 获取该步骤的所有chunks，严格按sequence顺序渲染
      const stepChunks = this.getStepChunksWithLogicalOrder(
        filteredChunks,
        stepInfo,
        sortedSteps,
        stepIndex,
        usedChunks
      )
      this.renderChunksInSequenceOrder(stepChunks, parts, usedChunks)
    }

    // 添加步骤后的剩余内容
    const remainingChunks = filteredChunks.filter(chunk => !usedChunks.has(chunk.sequence))
    this.renderChunksInSequenceOrder(remainingChunks, parts, usedChunks)

    return parts.join('')
  }

  // 按服务端返回的 sequence 顺序严格增量渲染 chunks
  // 核心策略：完全尊重服务端的 sequence 顺序，不做任何重排
  private renderChunksInSequenceOrder(
    chunks: OrderedMessageChunk[],
    parts: string[],
    usedChunks: Set<number>
  ): void {
    if (chunks.length === 0) return

    // console.log(`chunks data:${chunks.}`)
    // 严格按 sequence 顺序排序（同一 message_id 内，sequence 应该是唯一且递增的）
    const sortedChunks = chunks.slice().sort((a, b) => {
      // 首先按 sequence 排序
      if (a.sequence !== b.sequence) {
        return a.sequence - b.sequence
      }
      // sequence 相同时，使用到达顺序作为稳定排序的辅助
      const messageId = a.message_id
      const orderMap = this.chunkArrivalOrder.get(messageId)
      const ka = orderMap?.get(`${a.execution_id}#${a.sequence}`) || 0
      const kb = orderMap?.get(`${b.execution_id}#${b.sequence}`) || 0
      return ka - kb
    })

    // 按顺序渲染，使用文本缓冲区优化连续的 Content 和 Thinking chunks
    let textBuffer = ''
    let thinkingBuffer = ''
    let lastThinkingStage = ''

    for (const chunk of sortedChunks) {
      usedChunks.add(chunk.sequence)

      if (chunk.chunk_type === 'Content') {
        // 先输出积累的Thinking内容
        if (thinkingBuffer.trim()) {
          parts.push(`\n🤔 **思考过程**\n${thinkingBuffer}\n`)
          thinkingBuffer = ''
          lastThinkingStage = ''
        }
        // Content 类型：累积到缓冲区
        textBuffer += chunk.content?.toString() || ''
      } else if (chunk.chunk_type === 'Thinking') {
        // 先输出积累的Content内容
        if (textBuffer.trim()) {
          parts.push(textBuffer)
          textBuffer = ''
        }
        // Thinking类型：累积到thinking缓冲区（同一stage的连续chunks合并）
        const currentStage = chunk.stage || ''
        if (lastThinkingStage && lastThinkingStage !== currentStage && thinkingBuffer.trim()) {
          // 不同stage，先输出前一个stage的内容
          parts.push(`\n🤔 **思考过程**\n${thinkingBuffer}\n`)
          thinkingBuffer = ''
        }
        lastThinkingStage = currentStage
        const content = chunk.content?.toString().replace(/^Thought:\s*/i, '').trim() || ''
        thinkingBuffer += content
      } else {
        // 非 Content/Thinking 类型：先输出所有缓冲区
        if (textBuffer.trim()) {
          parts.push(textBuffer)
          textBuffer = ''
        }
        if (thinkingBuffer.trim()) {
          parts.push(`\n🤔 **思考过程**\n${thinkingBuffer}\n`)
          thinkingBuffer = ''
          lastThinkingStage = ''
        }
        const formatted = this.formatChunkWithSpecialHandling(chunk, chunk.message_id)
        if (formatted.trim()) {
          parts.push(formatted)
        }
      }
    }

    // 输出剩余的缓冲内容
    if (textBuffer.trim()) {
      parts.push(textBuffer)
    }
    if (thinkingBuffer.trim()) {
      parts.push(`\n🤔 **思考过程**\n${thinkingBuffer}\n`)
    }
  }

  // 特殊处理不同类型的chunk格式化
  private formatChunkWithSpecialHandling(chunk: OrderedMessageChunk, messageId: string): string {
    switch (chunk.chunk_type) {
      case 'ToolResult':
        return this.formatToolResult(chunk)
      case 'PlanInfo':
        return this.formatPlanInfo(chunk)
      case 'Content':
        return chunk.content?.toString() || ''
      case 'Thinking':
        // Thinking类型在renderChunksInSequenceOrder中已累积处理，这里作为fallback
        return this.formatThinking(chunk)
      case 'Error':
        return `❌ **错误**\n${chunk.content}`
      case 'StreamComplete':
        return ''
      case 'Meta':
        // Meta 事件不直接显示在内容中（Orchestrator 事件在 buildContent 层面处理）
        return ''
      default:
        return chunk.content
    }
  }

  private formatThinking(chunk: OrderedMessageChunk): string {
    try {
      // 移除 "Thought:" 前缀（如果存在）
      const contentStr = chunk.content
        .toString()
        .replace(/^Thought:\s*/i, '')
        .trim()

      // 直接以明文形式显示思考过程
      return `\n🤔 **思考过程**\n${contentStr}\n`
    } catch (err) {
      console.error('格式化思考过程失败:', err)
      return `\n🤔 **思考过程**\n${chunk.content}`
    }
  }

  private formatToolResult(chunk: OrderedMessageChunk): string {
    try {
      const contentStr = chunk.content.toString()

      let parsed: any = null
      const tool_name = chunk.tool_name
      let stepName = 'Tool Execution'
      let resultContent = contentStr
      let toolArgs: any = null // 新增：存储工具参数

      // 尝试解析JSON获取步骤名称和内容
      let isSuccess = true
      let errorMessage = ''

      try {
        parsed = JSON.parse(contentStr)
        stepName = parsed?.step_name || parsed?.name || 'Tool Execution'

        // 提取工具参数（如果存在）
        if (parsed?.args || parsed?.arguments || parsed?.input) {
          toolArgs = parsed.args || parsed.arguments || parsed.input
        }

        // 检查是否是执行失败的情况
        const successValue = parsed?.success
        const hasError = parsed?.error && parsed.error !== null && parsed.error !== ''

        const isFailure =
          successValue === false ||
          successValue === 'false' ||
          successValue === 'false' ||
          successValue === 0 ||
          successValue === '0' ||
          hasError

        if (isFailure) {
          isSuccess = false
          errorMessage = parsed?.error || 'Unknown error'
          resultContent = parsed?.error || parsed?.output || contentStr
        } else {
          resultContent = parsed?.result || parsed?.output || contentStr
        }
      } catch (parseError) {
        const stepMatch = contentStr.match(/(?:步骤|Step|工具|Tool)[:：]?\s*([^\n\r]+)/)
        if (stepMatch) {
          stepName = stepMatch[1].trim()
        }
        resultContent = contentStr
      }

      // 生成可安全渲染的HTML结构
      const escaped = this.escapeHtml(
        typeof resultContent === 'string' ? resultContent : JSON.stringify(resultContent, null, 2)
      )

      // 根据执行结果调整标题显示
      const statusIcon = isSuccess ? '🔧' : '❌'

      const displayName = stepName !== 'Tool Execution' ? stepName : tool_name || 'Tool'
      return `
<details class="tool-result-block border-l-4 border-primary/30 bg-base-200/50 rounded-r-lg my-2">
  <summary class="cursor-pointer px-4 py-2 text-sm font-medium hover:bg-base-200/80 transition-colors flex items-center gap-2">
    <span class="text-primary">${statusIcon}</span>
    <span>${displayName}</span>
    <span class="badge badge-sm ${isSuccess ? 'badge-success' : 'badge-error'}">${isSuccess ? '成功' : '失败'}</span>
  </summary>
  <div class="border-t border-base-300 bg-base-100">
    ${escaped}
  </div>
</details>

`
    } catch (err) {
      console.error('格式化工具结果失败:', err)
      return `🔧 **工具执行**\n${chunk.content}`
    }
  }

  private escapeHtml(input: string): string {
    return input
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;')
  }

  private formatPlanInfo(chunk: OrderedMessageChunk): string {
    try {
      // 优先参考后端提示的“有效计划信息”提取顺序：```json 块 > 任意 ``` 块 > 最外层花括号
      const raw = chunk.content?.toString() ?? ''
      const contentStr = raw.trim()

      // 1) 提取 ```json ... ```
      const jsonFenceStart = contentStr.indexOf('```json')
      let fenced: string | null = null
      if (jsonFenceStart >= 0) {
        const rest = contentStr.slice(jsonFenceStart + 7)
        const end = rest.indexOf('```')
        if (end >= 0) fenced = rest.slice(0, end).trim()
      }

      // 2) 若无，则尝试任意 ``` ... ```
      if (!fenced) {
        const anyFenceStart = contentStr.indexOf('```')
        if (anyFenceStart >= 0) {
          const rest = contentStr.slice(anyFenceStart + 3)
          const end = rest.indexOf('```')
          if (end >= 0) {
            const block = rest.slice(0, end).trim()
            if (block.startsWith('{')) fenced = block
          }
        }
      }

      // 3) 若仍无，则截取首个 { 到最后一个 }
      if (!fenced) {
        const s = contentStr.indexOf('{')
        const e = contentStr.lastIndexOf('}')
        if (s >= 0 && e > s) fenced = contentStr.slice(s, e + 1)
      }

      // 解析对象，且仅当包含关键字段 steps 才认为是“有效计划”
      let parsed: any = null
      if (fenced) {
        try {
          parsed = JSON.parse(fenced)
        } catch {
          parsed = null
        }
      }

      if (parsed && typeof parsed === 'object') {
        // 必须包含 steps 数组才视为有效
        if (!Array.isArray(parsed.steps)) {
          // 如果后端未给出完整结构，回退到原文展示
          return `📋 **执行计划**\n${chunk.content}\n\n\n\n`
        }

        // 生成Markdown TodoList
        const planTitle = parsed.name || '执行计划'
        let todoListMd = `## ${planTitle}\n\n`

        if (parsed.steps && Array.isArray(parsed.steps)) {
          parsed.steps.forEach((step: any, index: number) => {
            const stepName = step.name || `步骤 ${index + 1}`
            const stepDesc = step.description || ''
            const stepType = step.type || step.step_type || ''

            // console.log('stepType: ', stepType)
            // 根据步骤类型添加不同的图标
            let icon = ''
            if (stepType === 'ToolCall') icon = '🔧'
            else if (stepType === 'LlmCall') icon = '🤔'
            else if (stepType === 'DataProcessing') icon = '📊'
            else if (stepType === 'Conditional') icon = '🔀'
            else if (stepType === 'Parallel') icon = '⚡'

            // 使用DaisyUI的tooltip组件来显示描述信息
            if (stepDesc) {
              const safeDesc = this.sanitizePlanText(stepDesc)
              todoListMd += `- [ ] ${icon} <span class="tooltip tooltip-right cursor-help" data-tip="${safeDesc.replace(/"/g, '&quot;')}">${stepName}</span>`
            } else {
              todoListMd += `- [ ] ${icon} **${stepName}**`
            }

            if (step.dependencies && step.dependencies.length > 0) {
              todoListMd += `\n  > 依赖: ${step.dependencies.join(', ')}`
            }
            todoListMd += '\n'
          })
        }
        // 确保TodoList格式正确，保留换行
        return todoListMd.trim()
      }
    } catch (err) {
      console.error('格式化计划信息失败:', err)
    }

    return `**执行计划**\n${chunk.content}\n\n\n\n`
  }

  // 将主机:端口等易被 Markdown/排版折行的片段包裹为行内代码，避免误换行或格式化
  private sanitizePlanText(text: string): string {
    try {
      let out = text
      // 匹配 IPv4:port
      out = out.replace(/\b(?:\d{1,3}\.){3}\d{1,3}:(\d{1,5})\b/g, m => `\`${m}\``)
      // 匹配 http(s)://host:port 形式
      out = out.replace(/\bhttps?:\/\/[^\s]+/gi, m => `\`${m}\``)
      return out
    } catch {
      return text
    }
  }

  isComplete(messageId: string): boolean {
    // 优先检查StreamComplete标志
    if (this.streamCompleteFlags.get(messageId) === true) {
      return true
    }
    const chunks = this.chunks.get(messageId) || []
    const archInfo = this.architectureInfo.get(messageId)
    
    // ReAct 架构：只有 stage === "complete" 才算完成
    if (archInfo?.type === 'ReAct') {
      return chunks.some(chunk => 
        chunk.chunk_type === 'Meta' && 
        chunk.stage === 'complete'
      )
    }
    
    // 其他架构：检查 is_final 标志
    return chunks.some(chunk => chunk.is_final)
  }

  hasError(messageId: string): boolean {
    const chunks = this.chunks.get(messageId) || []
    return chunks.some(chunk => chunk.chunk_type === 'Error')
  }

  cleanup(messageId: string): void {
    this.chunks.delete(messageId)
    this.stepsByMessageId.delete(messageId)
    // 注意：不清理architectureInfo和streamCompleteFlags，保持持久化
  }

  // 新增：获取架构信息（持久化，不随cleanup清除）
  getArchitectureInfo(messageId: string) {
    return this.architectureInfo.get(messageId)
  }

  // 解析步骤 Meta 事件
  private parseStepMeta(messageId: string, chunk: OrderedMessageChunk): void {
    try {
      const meta = JSON.parse(chunk.content?.toString() || '{}')
      const steps = this.stepsByMessageId.get(messageId)!

      if (meta.type === 'step_started') {
        steps.set(meta.step_index, {
          step_name: meta.step_name,
          step_type: meta.step_type,
          start_sequence: chunk.sequence,
          status: 'InProgress',
        })
      } else if (meta.type === 'step_completed') {
        const existing = steps.get(meta.step_index)
        if (existing) {
          existing.status = meta.status
          existing.end_sequence = chunk.sequence
        }
      } else if (meta.type === 'step_update') {
        // ReAct 架构发送 step_update
        if (meta.status === 'executing') {
          steps.set(meta.step_index, {
            step_name: meta.step_name,
            step_type: 'ToolCall', // ReAct steps are usually tool calls
            start_sequence: chunk.sequence,
            status: 'InProgress',
          })
        }
      }
    } catch (e) {
      // 忽略非步骤相关的 Meta 事件
    }
  }

  // 获取步骤相关的 chunks 并进行逻辑排序
  private getStepChunksWithLogicalOrder(
    allChunks: OrderedMessageChunk[],
    stepInfo: any,
    sortedSteps: [number, any][],
    currentStepIndex: number,
    usedChunks: Set<number>
  ): OrderedMessageChunk[] {
    // 优先使用当前步骤在 step_completed 元事件中记录的 end_sequence，
    // 这样可以避免尚未开始下一步时，后续跨步骤的内容被错误归入当前步骤
    let endSequence = stepInfo.end_sequence

    if (typeof endSequence !== 'number' || !isFinite(endSequence)) {
      // 若尚无 end_sequence，则退回到“下一个步骤的 start_sequence”；若也没有，才取 Infinity
      const nextStep = sortedSteps.find(([idx]) => idx > currentStepIndex)
      endSequence = nextStep ? nextStep[1].start_sequence : Infinity
    }

    return allChunks.filter(chunk => {
      return (
        chunk.sequence >= (stepInfo.start_sequence || 0) &&
        chunk.sequence < endSequence &&
        !usedChunks.has(chunk.sequence)
      )
    })
  }

  // 获取步骤图标
  private getStepIcon(stepType: string): string {
    switch (stepType) {
      case 'AiReasoning':
      case 'LlmCall':
        return '🤔'
      case 'ToolCall':
        return '🔧'
      case 'DataProcessing':
        return '📊'
      case 'Conditional':
        return '🔀'
      case 'Parallel':
        return '⚡'
      default:
        return '📝'
    }
  }
}

export const useOrderedMessages = (
  messages: Ref<ChatMessage[]>,
  saveMessagesToConversation?: (messages: ChatMessage[]) => Promise<void>,
  // 新增参数以支持从useEventListeners迁移的功能
  streamStartTime?: Ref<number | null>,
  streamCharCount?: Ref<number>,
  emitHandlers?: any
) => {
  const unlistenCallbacks: (() => void)[] = []
  const processor = new MessageChunkProcessorImpl()
  // 维护后端 message_id 到前端消息ID的映射，避免因ID不一致导致的消息分裂/隐藏
  const idAlias = new Map<string, string>()

  const resolveCanonicalId = (incomingId: string): string => {
    return idAlias.get(incomingId) || incomingId
  }

  const findOrCreateMessage = (messageId: string): ChatMessage | null => {
    // 首先尝试按ID查找
    const message = messages.value.find(m => m.id === messageId)
    if (message) return message

    // ReAct 引擎使用相同的 message_id 进行多次迭代，需要查找最近的助手消息（不仅限于 streaming）
    const recentAssistantMessage = messages.value
      .slice()
      .reverse()
      .find(m => m.role === 'assistant' && (m.isStreaming || m.id === messageId))

    if (recentAssistantMessage) {
      // 建立ID别名映射
      idAlias.set(messageId, recentAssistantMessage.id)
      return recentAssistantMessage
    }

    // 宽容模式：自动创建一个助手占位消息，避免丢弃chunk
    const placeholder: ChatMessage = {
      id: messageId,
      role: 'assistant',
      content: '',
      timestamp: new Date(),
      isStreaming: true,
      hasError: false,
    }
    messages.value.push(placeholder)
    return placeholder
  }

  const handleMessageChunk = (chunk: OrderedMessageChunk) => {
    console.log('[handleMessageChunk] Received chunk:', {
      type: chunk.chunk_type,
      architecture: chunk.architecture,
      stage: chunk.stage,
      sequence: chunk.sequence,
      message_id: chunk.message_id
    })
    
    // 规范化 message_id：优先将新ID映射到当前streaming消息，避免产生新消息或覆盖旧消息
    let canonicalId = resolveCanonicalId(chunk.message_id)
    if (!idAlias.has(chunk.message_id)) {
      const streamingMessage = messages.value
        .slice()
        .reverse()
        .find(m => m.role === 'assistant' && m.isStreaming)
      if (streamingMessage && streamingMessage.id !== chunk.message_id) {
        idAlias.set(chunk.message_id, streamingMessage.id)
        canonicalId = streamingMessage.id
      }
    }

    const message = findOrCreateMessage(canonicalId)
    if (!message) {
      console.warn('找不到目标消息，丢弃chunk:', chunk)
      return
    }

    // 🔒 防止已完成消息再次接收chunk导致内容重复
    // 但如果之前被错误地标记为完成（例如内部工具误发 is_final），
    // 当收到新的非最终 chunk 时允许重新打开 streaming 状态。
    if (!message.isStreaming) {
      if (chunk.is_final) {
        return
      } else {
        message.isStreaming = true
      }
    }

    // 如果是携带RAG引用的Meta块，解析其中的citations并直接绑定到当前消息
    if (chunk.chunk_type === 'Meta') {
      try {
        const obj = JSON.parse(chunk.content?.toString() || '{}')
        if (obj && obj.type === 'rag_citations' && Array.isArray(obj.citations)) {
          // 直接更新消息的引用数组
           (message as any).citations = obj.citations
        }
      } catch (e) {
        console.warn('解析Meta块失败:', e)
      }
    }

    // 所有类型的 chunk 都通过 processor 统一处理，确保按 sequence 顺序显示
    const normalizedChunk: OrderedMessageChunk = { ...chunk, message_id: canonicalId }
    processor.addChunk(normalizedChunk)

    // 统一重新构建完整内容，确保所有 chunk 类型都能实时渲染
    message.content = processor.buildContent(canonicalId)

    // 更新流统计
    if (streamCharCount && chunk.content) {
      streamCharCount.value += chunk.content.toString().length
    }

    // 更新状态 - 确保使用规范化的ID检查状态
    // ReAct 引擎：不因中间步骤的 is_final 而停止 streaming，只在真正完成时标记
    const isComplete = processor.isComplete(canonicalId)
    if (isComplete) {
      message.isStreaming = false
    } else {
      // 保持 streaming 状态，即使某些 chunk 带有 is_final（可能是工具调用结果）
      message.isStreaming = true
    }
    message.hasError = processor.hasError(canonicalId)

    // 如果完成，先解析并保存架构数据，再清理 processor 中的数据
    if (!message.isStreaming) {
      const allChunks = processor.chunks.get(canonicalId) || []

      // 保存架构元数据（不清理）
      const archInfo = processor.getArchitectureInfo(canonicalId)
      
      // 优先从 chunk 中获取明确的 architecture 标识
      const archFromChunks = allChunks.find(c => c.architecture)?.architecture
      const archType = archFromChunks || archInfo?.type || 'Unknown'
      
      console.log('[useOrderedMessages] Message complete:', {
        messageId: canonicalId,
        archInfo,
        archFromChunks,
        archType,
        chunksCount: allChunks.length
      })

      // 保存架构类型（优先使用从chunks中获取的）
      if (archType && archType !== 'Unknown') {
        (message as any).architectureType = archType
        if (archInfo) {
          (message as any).architectureMeta = archInfo
        }
      } else if (archInfo) {
        (message as any).architectureType = archInfo.type
        ;(message as any).architectureMeta = archInfo
      }

      if (archType === 'ReAct') {
        // ReAct架构：使用 ReActMessageProcessor 进行处理
        // 架构元数据已在 handleMessageChunk 中保存
        // 必须从 chunks 提取步骤，否则前端无法显示过程
        const steps = ReActMessageProcessor.extractStepsFromChunks(allChunks)
        ;(message as any).reactSteps = steps
      } else if (archType === 'Travel') {
        // Travel架构：使用 TravelMessageProcessor 提取 OODA 循环数据
        const cycles = TravelMessageProcessor.extractCyclesFromChunks(allChunks)
        ;(message as any).travelCycles = cycles
        console.log('[useOrderedMessages] Travel cycles extracted:', cycles.length)
        
        // 同时提取嵌入的 VisionExplorer 迭代数据（Travel 可能包含 VisionExplorer 子任务）
        const visionIterations = VisionExplorerMessageProcessor.extractIterationsFromChunks(allChunks)
        if (visionIterations.length > 0) {
          ;(message as any).visionIterations = visionIterations
          console.log('[useOrderedMessages] Travel embedded vision iterations:', visionIterations.length)
        }
      } else if (archType === 'VisionExplorer') {
        // VisionExplorer架构：使用 VisionExplorerMessageProcessor 提取迭代数据
        const iterations = VisionExplorerMessageProcessor.extractIterationsFromChunks(allChunks)
        ;(message as any).visionIterations = iterations
        console.log('[useOrderedMessages] Vision iterations extracted:', iterations.length)
      } else if (archType === 'LLMCompiler') {
        // LLMCompiler架构（简化版）
        try {
          // 详细日志：记录chunks信息
          console.log('[useOrderedMessages] LLMCompiler chunks summary:', {
            totalChunks: allChunks.length,
            chunkTypes: allChunks.map(c => ({ type: c.chunk_type, stage: c.stage, tool_name: c.tool_name })),
            toolResultCount: allChunks.filter(c => c.chunk_type === 'ToolResult').length,
            thinkingCount: allChunks.filter(c => c.chunk_type === 'Thinking').length,
            metaCount: allChunks.filter(c => c.chunk_type === 'Meta').length,
            planInfoCount: allChunks.filter(c => c.chunk_type === 'PlanInfo').length
          })
          
          const parsedData = parseLLMCompilerMessage(message.content, allChunks)
          
          console.log('[useOrderedMessages] LLMCompiler parsed data:', {
            hasPlanningData: !!parsedData.planningData,
            hasExecutionData: !!parsedData.executionData,
            hasJoinerData: !!parsedData.joinerData,
            hasSummaryData: !!parsedData.summaryData,
            planningTasks: parsedData.planningData?.tasks?.length,
            executionRounds: parsedData.executionData?.rounds?.length
          })
          
          ;(message as any).llmCompilerData = parsedData

          // 保存Content类型的最终响应（后端直接发送的）
          const contentChunks = allChunks.filter(c =>
            c.chunk_type === 'Content' && c.architecture === 'LLMCompiler'
          )
          if (contentChunks.length > 0) {
            const finalResponse = contentChunks.map(c => c.content?.toString() || '').join('')
            if (finalResponse.length > 50) {
              ;(message as any).llmCompilerFinalResponse = finalResponse
            }
          }
        } catch (e) {
          console.warn('[useOrderedMessages] Failed to parse LLMCompiler data:', e)
        }
      } else if (archType === 'PlanAndExecute') {
        // PlanAndExecute架构
        try {
          const parsedData = parsePlanAndExecuteMessage(message.content, allChunks)
            ; (message as any).planAndExecuteData = parsedData
        } catch (e) {
          console.warn('[useOrderedMessages] Failed to parse PlanAndExecute data:', e)
        }
      } else if (archType === 'ReWOO') {
        // ReWOO架构
        try {
          const parsedData = parseReWOOMessage(message.content, allChunks)
            ; (message as any).rewooData = parsedData
        } catch (e) {
          console.warn('[useOrderedMessages] Failed to parse ReWOO data:', e)
        }
      }

      processor.cleanup(canonicalId)

      // 仅在助手消息完成时持久化该条消息，避免重复保存用户消息
      if (saveMessagesToConversation && message.role === 'assistant') {
        saveMessagesToConversation([message]).catch(err => {
          console.error('保存消息失败:', err)
        })
      }
    } else {
      // 🔥 新增：在流式过程中也实时解析架构数据
      const allChunks = processor.chunks.get(canonicalId) || []
      const archInfo = processor.getArchitectureInfo(canonicalId)
      // 优先从 chunk 中获取明确的 architecture 标识
      const archType = allChunks.find(c => c.architecture)?.architecture || archInfo?.type || 'Unknown'

      if (archType === 'ReAct') {
        // ReAct架构在流式过程中：由 ReActStepDisplay 组件处理步骤展示
        // 必须从 chunks 提取步骤，否则前端无法显示过程
        const steps = ReActMessageProcessor.extractStepsFromChunks(allChunks)
        ;(message as any).reactSteps = steps
      } else if (archType === 'Travel') {
        // Travel架构：使用 TravelMessageProcessor 实时提取 OODA 循环数据
        const cycles = TravelMessageProcessor.extractCyclesFromChunks(allChunks)
        ;(message as any).travelCycles = cycles
        // 同时提取嵌入的 VisionExplorer 迭代数据
        const visionIterations = VisionExplorerMessageProcessor.extractIterationsFromChunks(allChunks)
        if (visionIterations.length > 0) {
          ;(message as any).visionIterations = visionIterations
        }
      } else if (archType === 'VisionExplorer') {
        // VisionExplorer架构：实时提取迭代数据
        const iterations = VisionExplorerMessageProcessor.extractIterationsFromChunks(allChunks)
        ;(message as any).visionIterations = iterations
      } else if (archType === 'LLMCompiler') {
        // LLMCompiler架构实时解析
        const allChunks = processor.chunks.get(canonicalId) || []
        try {
          const parsedData = parseLLMCompilerMessage(message.content, allChunks)
            ; (message as any).llmCompilerData = parsedData
        } catch (e) {
          // ignore parsing errors during streaming
        }
      } else if (archType === 'PlanAndExecute') {
        // PlanAndExecute架构实时解析
        const allChunks = processor.chunks.get(canonicalId) || []
        try {
          const parsedData = parsePlanAndExecuteMessage(message.content, allChunks)
            ; (message as any).planAndExecuteData = parsedData
        } catch (e) {
          // ignore parsing errors during streaming
        }
      } else if (archType === 'ReWOO') {
        // ReWOO架构实时解析
        const allChunks = processor.chunks.get(canonicalId) || []
        try {
          const parsedData = parseReWOOMessage(message.content, allChunks)
            ; (message as any).rewooData = parsedData
        } catch (e) {
          // ignore parsing errors during streaming
        }
      }
    }
  }

  // ReAct 步骤解析已移至 ReActMessageProcessor，这里无需处理

  const setupEventListeners = async () => {
    // 如果已经设置了监听器，先清理
    if (unlistenCallbacks.length > 0) {
      cleanup()
    }

    try {
      // 只监听一个事件类型：message_chunk
      const unlistenChunk = await listen('message_chunk', event => {
        const chunk = event.payload as OrderedMessageChunk
        handleMessageChunk(chunk)
      })

      unlistenCallbacks.push(unlistenChunk)
      console.log('统一消息事件监听器已设置')
    } catch (error) {
      console.error('设置事件监听器失败:', error)
    }
  }

  const cleanup = () => {
    unlistenCallbacks.forEach(unlisten => unlisten())
    unlistenCallbacks.length = 0
    processor.chunks.clear()
    console.log('简化消息事件监听器已清理')
  }

  // 手动添加消息chunk（用于测试或特殊情况）
  const addChunk = (chunk: OrderedMessageChunk) => {
    handleMessageChunk(chunk)
  }

  // 检查消息是否包含特定类型的chunk
  const hasChunkType = (messageId: string, chunkType: ChunkType): boolean => {
    const chunks = processor.chunks.get(messageId) || []
    return chunks.some(chunk => chunk.chunk_type === chunkType)
  }

  // 获取消息的chunk统计
  const getChunkStats = (messageId: string) => {
    const chunks = processor.chunks.get(messageId) || []
    const stats = {
      total: chunks.length,
      byType: {} as Record<ChunkType, number>,
      isComplete: processor.isComplete(messageId),
      hasError: processor.hasError(messageId),
    }

    chunks.forEach(chunk => {
      stats.byType[chunk.chunk_type] = (stats.byType[chunk.chunk_type] || 0) + 1
    })

    return stats
  }

  return {
    setupEventListeners,
    cleanup,
    addChunk,
    hasChunkType,
    getChunkStats,
    processor,
  }
}

// 创建用户消息的便捷函数
export function createUserMessage(
  id: string,
  content: string,
  timestamp = new Date()
): ChatMessage {
  return {
    id,
    role: 'user',
    content,
    timestamp,
    isStreaming: false,
    hasError: false,
  }
}

// 创建助手消息的便捷函数
export function createAssistantMessage(id: string, timestamp = new Date()): ChatMessage {
  return {
    id,
    role: 'assistant',
    content: '',
    timestamp,
    isStreaming: true,
    hasError: false,
  }
}
