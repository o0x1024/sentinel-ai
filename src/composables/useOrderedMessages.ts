// 简化的有序消息处理composable
// 替代复杂的useEventListeners和segments处理

import { ref, Ref } from 'vue'
import { listen } from '@tauri-apps/api/event'
import type {
  OrderedMessageChunk,
  ChunkType,
  MessageChunkProcessor
} from '../types/ordered-chat'
import type { ChatMessage } from '../types/chat'

class MessageChunkProcessorImpl implements MessageChunkProcessor {
  chunks = new Map<string, OrderedMessageChunk[]>()
  // 步骤索引：存储每个消息的步骤信息
  private stepsByMessageId = new Map<string, Map<number, any>>()
  // 视图模式：timeline（时间线）或 steps（步骤）
  private viewMode: 'timeline' | 'steps' = 'steps'
  // 调试模式：用于输出渲染顺序信息
  private debugMode: boolean = false
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
    // 按sequence排序插入
    const insertIndex = chunks.findIndex(c => c.sequence > chunk.sequence)
    if (insertIndex === -1) {
      chunks.push(chunk)
    } else {
      chunks.splice(insertIndex, 0, chunk)
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
    if (this.viewMode === 'steps') {
      return this.buildStepGroupedContent(messageId)
    } else {
      return this.buildTimelineContent(messageId)
    }
  }

  // 设置视图模式
  setViewMode(mode: 'timeline' | 'steps'): void {
    this.viewMode = mode
  }

  // 获取当前视图模式
  getViewMode(): 'timeline' | 'steps' {
    return this.viewMode
  }

  // 设置调试模式
  setDebugMode(enabled: boolean): void {
    this.debugMode = enabled
  }

  // 时间线视图：严格按 sequence 顺序
  private buildTimelineContent(messageId: string): string {
    const chunks = this.chunks.get(messageId) || []
    const sorted = chunks.sort((a, b) => a.sequence - b.sequence)
    const parts: string[] = []
    let textBuffer = ''
    for (const chunk of sorted) {
      if (chunk.chunk_type === 'Content') {
        textBuffer += chunk.content?.toString() || ''
        continue
      }
      if (textBuffer.trim().length > 0) {
        parts.push(textBuffer)
        textBuffer = ''
      }
      const formatted = this.formatChunkWithSpecialHandling(chunk)
      if (formatted.trim().length > 0) parts.push(formatted)
    }
    if (textBuffer.trim().length > 0) parts.push(textBuffer)
    return parts.join('')
  }

  // 步骤视图：按步骤分组显示，严格按sequence顺序渲染内容
  private buildStepGroupedContent(messageId: string): string {
    const chunks = this.chunks.get(messageId) || []
    const steps = this.stepsByMessageId.get(messageId) || new Map()
    
    if (steps.size === 0) {
      // 如果没有步骤信息，回退到时间线视图
      return this.buildTimelineContent(messageId)
    }

    const parts: string[] = []
    const sortedSteps = Array.from(steps.entries()).sort(([a], [b]) => a - b)
    const usedChunks = new Set<number>()

    // 添加步骤开始前的内容
    const preStepChunks = chunks.filter(chunk => {
      const minStepSequence = Math.min(...Array.from(steps.values()).map(s => s.start_sequence || Infinity))
      return chunk.sequence < minStepSequence
    })
    
    this.renderChunksInSequenceOrder(preStepChunks, parts, usedChunks)

    // 按步骤渲染
    for (const [stepIndex, stepInfo] of sortedSteps) {
      // 步骤标题
      const stepIcon = this.getStepIcon(stepInfo.step_type)
      const statusIcon = stepInfo.status === 'Completed' ? '✅' : 
                        stepInfo.status === 'Failed' ? '❌' : 
                        stepInfo.status === 'InProgress' ? '🔄' : '⏳'
      
      parts.push(`\n### ${stepIcon} 步骤 ${stepIndex}: ${stepInfo.step_name || '未命名步骤'} ${statusIcon}\n`)
      
      // 获取该步骤的所有chunks，严格按sequence顺序渲染
      const stepChunks = this.getStepChunksWithLogicalOrder(chunks, stepInfo, sortedSteps, stepIndex, usedChunks)
      this.renderChunksInSequenceOrder(stepChunks, parts, usedChunks)
    }

    // 添加步骤后的剩余内容
    const remainingChunks = chunks.filter(chunk => !usedChunks.has(chunk.sequence))
    this.renderChunksInSequenceOrder(remainingChunks, parts, usedChunks)

    return parts.join('')
  }

  // 新增方法：智能排序渲染chunks，确保内容不穿插
  // 核心策略：在步骤内，先渲染所有Content和Thinking，再渲染ToolResult
  private renderChunksInSequenceOrder(
    chunks: OrderedMessageChunk[], 
    parts: string[], 
    usedChunks: Set<number>
  ): void {
    if (chunks.length === 0) return
    
    // 按到达顺序为主、sequence 为辅的稳定排序
    const sortedChunks = chunks.slice().sort((a, b) => {
      const messageId = a.message_id
      const orderMap = this.chunkArrivalOrder.get(messageId)
      const ka = orderMap?.get(`${a.execution_id}#${a.sequence}`) || a.sequence
      const kb = orderMap?.get(`${b.execution_id}#${b.sequence}`) || b.sequence
      if (ka !== kb) return ka - kb
      return a.sequence - b.sequence
    })
    
    // 调试信息：记录渲染顺序
    if (this.debugMode) {
      console.log('📊 Rendering chunks - Original sequence order:', sortedChunks.map(c => ({
        sequence: c.sequence,
        type: c.chunk_type,
        preview: c.content?.toString().substring(0, 30) + '...'
      })))
    }
    
    // 智能分组：将chunks分为Content/Thinking组和ToolResult组
    const contentChunks: OrderedMessageChunk[] = []
    const toolResultChunks: OrderedMessageChunk[] = []
    const otherChunks: OrderedMessageChunk[] = []
    
    for (const chunk of sortedChunks) {
      if (chunk.chunk_type === 'Content' || chunk.chunk_type === 'Thinking') {
        contentChunks.push(chunk)
      } else if (chunk.chunk_type === 'ToolResult') {
        toolResultChunks.push(chunk)
      } else {
        otherChunks.push(chunk)
      }
    }
    
    if (this.debugMode) {
      console.log('📊 After grouping:', {
        content: contentChunks.length,
        toolResult: toolResultChunks.length,
        other: otherChunks.length
      })
    }
    
    // 渲染顺序：Content/Thinking → Other → ToolResult
    let textBuffer = ''
    
    // 1. 先渲染所有Content和Thinking（按sequence顺序）
    for (const chunk of contentChunks) {
      usedChunks.add(chunk.sequence)
      
      if (chunk.chunk_type === 'Content') {
        textBuffer += chunk.content?.toString() || ''
      } else {
        // Thinking类型：先输出缓冲文本，再输出Thinking
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
    
    // 输出缓冲的Content文本
    if (textBuffer.trim()) {
      parts.push(textBuffer)
      textBuffer = ''
    }
    
    // 2. 渲染其他类型（Meta, Error等）
    for (const chunk of otherChunks) {
      usedChunks.add(chunk.sequence)
      const formatted = this.formatChunkWithSpecialHandling(chunk)
      if (formatted.trim()) {
        parts.push(formatted)
      }
    }
    
    // 3. 最后渲染所有ToolResult（按sequence顺序）
    for (const chunk of toolResultChunks) {
      usedChunks.add(chunk.sequence)
      const formatted = this.formatChunkWithSpecialHandling(chunk)
      if (formatted.trim()) {
        parts.push(formatted)
      }
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
        return chunk.content
      case 'Thinking':
        return `🤔 **思考过程**\n${chunk.content}`
      case 'Error':
        return `❌ **错误**\n${chunk.content}`
      case 'Meta':
        // Meta 事件在步骤视图中不直接显示，但在时间线视图中可以显示调试信息
        if (this.viewMode === 'timeline') {
          try {
            const meta = JSON.parse(chunk.content?.toString() || '{}')
            if (meta.type === 'step_started') {
              return `\n🚀 **开始步骤 ${meta.step_index}**: ${meta.step_name} (${meta.step_type})\n`
            } else if (meta.type === 'step_completed') {
              return `\n✅ **完成步骤 ${meta.step_index}**: ${meta.step_name} (${meta.status})\n`
            }
          } catch (e) {
            // 忽略解析错误
          }
        }
        return ''
      default:
        return chunk.content
    }
  }

  

  private formatToolResult(chunk: OrderedMessageChunk): string {
    try {
      const contentStr = chunk.content.toString()

      let parsed: any = null
      let tool_name = chunk.tool_name
      let stepName = 'Tool Execution'
      let resultContent = contentStr

      // 尝试解析JSON获取步骤名称和内容
      let isSuccess = true
      let errorMessage = ''
      
      try {
        parsed = JSON.parse(contentStr)
        stepName = parsed?.step_name || parsed?.name || 'Tool Execution'
        
        // 检查是否是执行失败的情况
        // 处理 success 字段可能是布尔值或字符串的情况
        const successValue = parsed?.success
        const hasError = parsed?.error && parsed.error !== null && parsed.error !== ''
        
        // 更健壮的失败判断逻辑
        const isFailure = successValue === false || 
                         successValue === 'false' || 
                         successValue === "false" ||
                         successValue === 0 ||
                         successValue === '0' ||
                         hasError
        
        // 调试信息
        console.log('ToolResult parsing:', {
          successValue,
          hasError,
          isFailure,
          errorField: parsed?.error
        })
        
        if (isFailure) {
          isSuccess = false
          errorMessage = parsed?.error || 'Unknown error'
          resultContent = parsed?.error || parsed?.output || contentStr
        } else {
          resultContent = parsed?.result || parsed?.output || contentStr
        }
      } catch (parseError) {
        // 如果不是JSON，尝试从内容中提取步骤名称
        const stepMatch = contentStr.match(/(?:步骤|Step|工具|Tool)[:：]?\s*([^\n\r]+)/)
        if (stepMatch) {
          stepName = stepMatch[1].trim()
        }
        resultContent = contentStr
      }

      // 生成可安全渲染的HTML结构，避免 Markdown 在 HTML 块内不解析的情况
      const contentType = this.detectContentType(resultContent)
      const escaped = this.escapeHtml(
        typeof resultContent === 'string' ? resultContent : JSON.stringify(resultContent, null, 2)
      )

      // 根据执行结果调整标题显示
      const statusIcon = isSuccess ? '🔧' : '❌'
      const statusBadge = isSuccess ? 
        `<span class="badge badge-success">成功</span>` : 
        `<span class="badge badge-error">失败</span>`
      
      // 标题栏显示工具名、步骤名和状态
      const displayName = stepName !== 'Tool Execution' ? stepName : (tool_name || 'Tool')
      const summaryContent = `${statusIcon} <strong>${displayName}</strong> ${statusBadge}`

      return `
<details class="tool-result border border-base-300 rounded-box bg-base-100">
<summary class="text-sm font-medium flex items-center gap-2">
  ${summaryContent}
</summary>
<div class="tool-result-body mt-2 p-3">
  <pre class="tool-result-content"><code class="language-${contentType}">${escaped}</code></pre>
</div>
</details>

`
    } catch (err) {
      console.error('格式化工具结果失败:', err)
      return `🔧 **工具执行**\n${chunk.content}`
    }
  }

  // 智能检测内容类型以选择合适的语法高亮
  private detectContentType(content: string): string {
    const trimmedContent = content.trim()

    try {
      JSON.parse(trimmedContent)
      return 'json'
    } catch {


      // 检测 XML/HTML
      if (trimmedContent.startsWith('<') && trimmedContent.includes('>')) {
        return 'xml'
      }

      // 检测代码片段
      if (trimmedContent.includes('function') || trimmedContent.includes('const ') ||
        trimmedContent.includes('let ') || trimmedContent.includes('var ')) {
        return 'javascript'
      }

      // 检测Python代码
      if (trimmedContent.includes('def ') || trimmedContent.includes('import ') ||
        trimmedContent.includes('from ') || trimmedContent.includes('print(')) {
        return 'python'
      }

      // 检测Shell命令
      if (trimmedContent.startsWith('$') || trimmedContent.includes('curl ') ||
        trimmedContent.includes('wget ') || trimmedContent.includes('chmod ')) {
        return 'bash'
      }

      // 检测SQL
      if (trimmedContent.toLowerCase().includes('select ') ||
        trimmedContent.toLowerCase().includes('insert ') ||
        trimmedContent.toLowerCase().includes('update ') ||
        trimmedContent.toLowerCase().includes('delete ')) {
        return 'sql'
      }

    }


    // 默认为纯文本
    return 'text'
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
      let contentStr = raw.trim()

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
        try { parsed = JSON.parse(fenced) } catch { parsed = null }
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

        // 添加估计时间等元信息
        // if (parsed.estimated_duration) {
        //   todoListMd += `\n> 📅 预计耗时: ${parsed.estimated_duration}\n`
        // }
        // if (parsed.resource_requirements) {
        //   todoListMd += `> 💾 资源需求: ${JSON.stringify(parsed.resource_requirements)}\n`
        // }

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
      out = out.replace(/\b(?:\d{1,3}\.){3}\d{1,3}:(\d{1,5})\b/g, (m) => `\`${m}\``)
      // 匹配 http(s)://host:port 形式
      out = out.replace(/\bhttps?:\/\/[^\s]+/gi, (m) => `\`${m}\``)
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
          status: 'InProgress'
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
      return chunk.sequence >= (stepInfo.start_sequence || 0) && 
             chunk.sequence < endSequence &&
             !usedChunks.has(chunk.sequence)
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
    let message = messages.value.find(m => m.id === messageId)
    if (message) return message

    // 如果找不到，查找最近的streaming助手消息
    const streamingMessage = messages.value
      .slice()
      .reverse()
      .find(m => m.role === 'assistant' && m.isStreaming)

    if (streamingMessage) {
      // 不再改写已有消息ID，改为记录别名映射
      idAlias.set(messageId, streamingMessage.id)
      return streamingMessage
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
    // 可以通过 setDebugMode(true) 开启详细日志

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
    message.isStreaming = !processor.isComplete(canonicalId)
    message.hasError = processor.hasError(canonicalId)


    // 如果完成，清理processor中的数据
    if (!message.isStreaming) {
      processor.cleanup(canonicalId)

      // 仅在助手消息完成时持久化该条消息，避免重复保存用户消息
      if (saveMessagesToConversation && message.role === 'assistant') {
        saveMessagesToConversation([message]).catch(err => {
          console.error('保存消息失败:', err)
        })
      }
    }
  }

  const setupEventListeners = async () => {
    // 如果已经设置了监听器，先清理
    if (unlistenCallbacks.length > 0) {
      cleanup()
    }
    
    try {
      // 只监听一个事件类型：message_chunk
      const unlistenChunk = await listen('message_chunk', (event) => {
        const chunk = event.payload as OrderedMessageChunk
        handleMessageChunk(chunk)
      })

      unlistenCallbacks.push(
        unlistenChunk,
      )
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
    processor: processor as MessageChunkProcessor,
    // 新增调试功能
    setDebugMode: (enabled: boolean) => processor.setDebugMode(enabled),
    setViewMode: (mode: 'timeline' | 'steps') => processor.setViewMode(mode),
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
export function createAssistantMessage(
  id: string,
  timestamp = new Date()
): ChatMessage {
  return {
    id,
    role: 'assistant',
    content: '',
    timestamp,
    isStreaming: true,
    hasError: false,
  }
}
