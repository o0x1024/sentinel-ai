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

  addChunk(chunk: OrderedMessageChunk): void {
    const messageId = chunk.message_id
    if (!this.chunks.has(messageId)) {
      this.chunks.set(messageId, [])
    }

    const chunks = this.chunks.get(messageId)!
    // 按sequence排序插入
    const insertIndex = chunks.findIndex(c => c.sequence > chunk.sequence)
    if (insertIndex === -1) {
      chunks.push(chunk)
    } else {
      chunks.splice(insertIndex, 0, chunk)
    }
  }

  buildContent(messageId: string): string {
    const chunks = this.chunks.get(messageId) || []
    // 将连续的 Content 分片合并为同一段文本进行增量显示
    const sorted = chunks.sort((a, b) => a.sequence - b.sequence)
    const parts: string[] = []
    let textBuffer = ''

    for (const chunk of sorted) {
      if (chunk.chunk_type === 'Content') {
        textBuffer += chunk.content?.toString() || ''
        continue
      }

      // 先冲刷已累积的 Content 文本
      if (textBuffer.trim().length > 0) {
        parts.push(textBuffer)
        textBuffer = ''
      }

      const formatted = this.formatChunkWithSpecialHandling(chunk)
      if (formatted.trim().length > 0) {
        parts.push(formatted)
      }
    }

    // 冲刷尾部 Content 文本
    if (textBuffer.trim().length > 0) {
      parts.push(textBuffer)
    }

    return parts.join('')
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

      // 尝试解析JSON获取步骤名称
      try {
        parsed = JSON.parse(contentStr)
        stepName = parsed?.step_name || parsed?.name || 'Tool Execution'
        resultContent = parsed?.result || parsed?.output || contentStr
      } catch (parseError) {
        // 如果不是JSON，尝试从内容中提取步骤名称
        const stepMatch = contentStr.match(/(?:步骤|Step|工具|Tool)[:：]?\s*([^\n\r]+)/)
        if (stepMatch) {
          stepName = stepMatch[1].trim()
        }
      }

      // 生成remark兼容的Markdown格式折叠面板
      // 使用details/summary标签和适当的语法高亮，包装在div中以便CSS样式生效
      const contentType = this.detectContentType(resultContent)

      return `
<details>
<summary>🔧 <strong>${tool_name}</strong></summary>
<div>

\`\`\`${contentType}
${resultContent}
\`\`\`

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

            // 根据步骤类型添加不同的图标
            let icon = ''
            if (stepType === 'ToolCall') icon = '🔧'
            else if (stepType === 'AiReasoning') icon = '🤔'
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
    console.log('chunk.is_final : ', chunks.some(chunk => chunk.is_final))
    return chunks.some(chunk => chunk.is_final)
  }

  hasError(messageId: string): boolean {
    const chunks = this.chunks.get(messageId) || []
    return chunks.some(chunk => chunk.chunk_type === 'Error')
  }

  cleanup(messageId: string): void {
    this.chunks.delete(messageId)
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

    console.log('处理消息块:', chunk)

    // 专门调试 ToolResult 类型的 chunk
    if (chunk.chunk_type === 'ToolResult') {
      console.log('🔧 收到 ToolResult chunk, content length:', chunk.content?.toString().length)
    }

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
