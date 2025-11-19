// 简化的有序消息处理composable
// 替代复杂的useEventListeners和segments处理

import { ref, Ref } from 'vue'
import { listen } from '@tauri-apps/api/event'
import type { OrderedMessageChunk, ChunkType, MessageChunkProcessor } from '../types/ordered-chat'
import type { ChatMessage } from '../types/chat'

class MessageChunkProcessorImpl implements MessageChunkProcessor {
  chunks = new Map<string, OrderedMessageChunk[]>()
  // 步骤索引：存储每个消息的步骤信息
  private stepsByMessageId = new Map<string, Map<number, any>>()
  // 到达顺序跟踪（按消息ID维度），用于不同 execution_id 的chunk建立稳定全局顺序
  private arrivalCounterByMessageId = new Map<string, number>()
  private chunkArrivalOrder = new Map<string, Map<string, number>>()

  addChunk(chunk: OrderedMessageChunk): void {
    const messageId = chunk.message_id
    if (!this.chunks.has(messageId)) {
      this.chunks.set(messageId, [])
      this.stepsByMessageId.set(messageId, new Map())
      this.arrivalCounterByMessageId.set(messageId, 0)
      this.chunkArrivalOrder.set(messageId, new Map())
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

    if (steps.size === 0) {
      // 如果没有步骤信息，直接按sequence顺序渲染所有chunks
      const sorted = chunks.sort((a, b) => a.sequence - b.sequence)
      const parts: string[] = []
      const usedChunks = new Set<number>()
      this.renderChunksInSequenceOrder(sorted, parts, usedChunks)
      return parts.join('')
    }

    const parts: string[] = []
    const sortedSteps = Array.from(steps.entries()).sort(([a], [b]) => a - b)
    const usedChunks = new Set<number>()

    // 添加步骤开始前的内容
    const preStepChunks = chunks.filter(chunk => {
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
        chunks,
        stepInfo,
        sortedSteps,
        stepIndex,
        usedChunks
      )
      this.renderChunksInSequenceOrder(stepChunks, parts, usedChunks)
    }

    // 添加步骤后的剩余内容
    const remainingChunks = chunks.filter(chunk => !usedChunks.has(chunk.sequence))
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

    // 按顺序渲染，使用文本缓冲区优化连续的 Content chunks
    let textBuffer = ''

    for (const chunk of sortedChunks) {
      usedChunks.add(chunk.sequence)

      if (chunk.chunk_type === 'Content') {
        // Content 类型：累积到缓冲区
        textBuffer += chunk.content?.toString() || ''
      } else {
        // 非 Content 类型：先输出缓冲区，再渲染当前 chunk
        if (textBuffer.trim()) {
          parts.push(textBuffer)
          textBuffer = ''
        }
        const formatted = this.formatChunkWithSpecialHandling(chunk)
        if (formatted.trim()) {
          parts.push(formatted)
        }
      }
    }

    // 输出剩余的缓冲文本
    if (textBuffer.trim()) {
      parts.push(textBuffer)
    }
  }

  // 特殊处理不同类型的chunk格式化
  private formatChunkWithSpecialHandling(chunk: OrderedMessageChunk): string {
    switch (chunk.chunk_type) {
      case 'ToolResult':
        return this.formatToolResult(chunk)
      case 'PlanInfo':
        return this.formatPlanInfo(chunk)
      case 'Content':
        // 智能过滤Content中的Action声明（ReAct架构）
        return chunk.content?.toString() || ''
      case 'Thinking':
        // return this.formatThinking(chunk)
        //  return chunk.content?.toString() || ''
        return ''
      case 'Error':
        return `❌ **错误**\n${chunk.content}`
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
      return `🤔 **思考过程**\n${contentStr}\n`
    } catch (err) {
      console.error('格式化思考过程失败:', err)
      return `🤔 **思考过程**\n${chunk.content}`
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
    const chunks = this.chunks.get(messageId) || []
    return chunks.some(chunk => chunk.is_final)
  }

  hasError(messageId: string): boolean {
    const chunks = this.chunks.get(messageId) || []
    return chunks.some(chunk => chunk.chunk_type === 'Error')
  }

  cleanup(messageId: string): void {
    this.chunks.delete(messageId)
    this.stepsByMessageId.delete(messageId)
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
    console.log('Received chunk:', chunk)
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
          ;(message as any).citations = obj.citations
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

    // 如果完成，先解析并保存 ReAct 步骤数据，再清理 processor 中的数据
    if (!message.isStreaming) {
      // 检测是否为 ReAct 消息并提取 ToolResult chunks
      const allChunks = processor.chunks.get(canonicalId) || []
      const toolResultChunks = allChunks.filter(c => c.chunk_type === 'ToolResult')
      // 在清理前，若存在 Orchestrator 的 Meta 事件，持久化为一个聚合对象写回到消息内容中
      try {
        const orchestratorEvents: string[] = []
        for (const c of allChunks) {
          if (c.chunk_type === 'Meta' && c.content) {
            try {
              const obj = JSON.parse(c.content.toString())
              if (obj?.type === 'orchestrator_session' || obj?.type === 'orchestrator_step') {
                orchestratorEvents.push(c.content.toString())
              }
            } catch {
              // ignore non-json meta
            }
          }
        }
        if (orchestratorEvents.length > 0) {
          // 将聚合后的 orchestrator 事件保存到消息内容，供前端渲染
          message.content = JSON.stringify({
            type: 'orchestrator_bundle',
            events: orchestratorEvents,
          })
        }
      } catch (e) {
        console.warn('[useOrderedMessages] Failed to persist orchestrator events:', e)
      }
      
      if (toolResultChunks.length > 0) {
        // 是 ReAct 消息，解析并存储步骤数据
        console.log('[useOrderedMessages] Parsing ReAct steps before cleanup, found', toolResultChunks.length, 'ToolResult chunks')
        
        const parsedSteps = parseReActStepsFromContent(message.content, canonicalId, allChunks)
        ;(message as any).reactSteps = parsedSteps
        console.log('[useOrderedMessages] Stored', parsedSteps.length, 'parsed ReAct steps in message')
      }
      
      processor.cleanup(canonicalId)

      // 仅在助手消息完成时持久化该条消息，避免重复保存用户消息
      if (saveMessagesToConversation && message.role === 'assistant') {
        saveMessagesToConversation([message]).catch(err => {
          console.error('保存消息失败:', err)
        })
      }
    }
  }

  // 从内容和 chunks 中解析 ReAct 步骤
  const parseReActStepsFromContent = (content: string, messageId: string, chunks: OrderedMessageChunk[]) => {
    const steps: any[] = []
    const toolResultChunks = chunks.filter(c => c.chunk_type === 'ToolResult')
    
    const lines = content.split('\n')
    let currentStep: any = {}
    let inObservation = false
    let observationLines: string[] = []
    
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i].trim()
      
      // 检测 Thought
      if (line.startsWith('Thought:')) {
        if (Object.keys(currentStep).length > 0) {
          if (observationLines.length > 0) {
            currentStep.observation = observationLines.join('\n')
            observationLines = []
            inObservation = false
          }
          steps.push(currentStep)
        }
        currentStep = {}
        currentStep.thought = line.substring('Thought:'.length).trim()
      }
      // 检测 Action
      else if (line.startsWith('Action:')) {
        if (inObservation && observationLines.length > 0) {
          currentStep.observation = observationLines.join('\n')
          observationLines = []
          inObservation = false
        }
        
        const actionContent = line.substring('Action:'.length).trim()
        
        // 检查下一行是否有 Action Input
        let actionInput = null
        if (i + 1 < lines.length && lines[i + 1].trim().startsWith('Action Input:')) {
          i++
          const inputLine = lines[i].substring(lines[i].indexOf('Action Input:') + 'Action Input:'.length).trim()
          try {
            actionInput = JSON.parse(inputLine)
          } catch {
            actionInput = inputLine
          }
        }
        
        currentStep.action = {
          tool: actionContent,
          args: actionInput,
          status: 'completed'
        }
        
        // 从 ToolResult chunks 中查找对应的 Observation
        const matchingToolResult = toolResultChunks.find(chunk => 
          chunk.tool_name === actionContent
        )
        
        if (matchingToolResult) {
          try {
            const obsData = JSON.parse(matchingToolResult.content.toString())
            currentStep.observation = obsData
            
            if (obsData.success === false || obsData.error) {
              currentStep.action.status = 'failed'
            }
          } catch (e) {
            currentStep.observation = matchingToolResult.content.toString()
          }
        }
      }
      // 检测 Observation (保留旧逻辑作为后备)
      else if (line.startsWith('Observation:')) {
        inObservation = true
        const obsContent = line.substring('Observation:'.length).trim()
        if (obsContent) {
          observationLines.push(obsContent)
        }
      }
      // 检测 Final Answer
      else if (line.match(/^Final\s+Answer:/i)) {
        if (inObservation && observationLines.length > 0) {
          currentStep.observation = observationLines.join('\n')
          observationLines = []
          inObservation = false
        }
        
        const finalContent = line.substring(line.indexOf(':') + 1).trim()
        currentStep.finalAnswer = finalContent
        
        // 收集后续所有行
        for (let j = i + 1; j < lines.length; j++) {
          const nextLine = lines[j]
          if (currentStep.finalAnswer) {
            currentStep.finalAnswer += '\n' + nextLine
          } else if (nextLine.trim()) {
            currentStep.finalAnswer = nextLine
          }
        }
        break
      }
      // 继续收集 observation 内容
      else if (inObservation && line) {
        observationLines.push(line)
      }
      // 继续收集 thought 内容
      else if (!inObservation && line && !currentStep.action && currentStep.thought) {
        currentStep.thought += '\n' + line
      }
    }
    
    // 保存最后一个步骤
    if (Object.keys(currentStep).length > 0) {
      if (observationLines.length > 0) {
        currentStep.observation = observationLines.join('\n')
      }
      steps.push(currentStep)
    }
    
    return steps
  }

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
