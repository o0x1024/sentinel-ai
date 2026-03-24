// 简化的有序聊天消息类型定义
// 用于替代复杂的segments和toolExecutions处理

export type ChunkType = 'Content' | 'Thinking' | 'ToolResult' | 'PlanInfo' | 'Error' | 'Meta' | 'StreamComplete'

// ReWOO、LLMCompiler、PlanAndExecute 已内嵌到泛化的 ReAct 引擎

export interface OrderedMessageChunk {
  execution_id: string
  message_id: string
  conversation_id?: string
  sequence: number
  chunk_type: ChunkType
  content: string
  timestamp: string
  is_final: boolean
  stage?: string
  tool_name?: string
  structured_data?: any
}

export interface SimplifiedChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string                // 合并后的完整内容，按时间顺序
  timestamp: Date
  isStreaming: boolean
  hasError: boolean

  // 移除复杂的segments, toolExecutions, executionPlan等
  // 所有内容都合并到content中，按sequence顺序显示
}

export interface MessageChunkProcessor {
  chunks: Map<string, OrderedMessageChunk[]>

  addChunk(chunk: OrderedMessageChunk): void
  buildContent(messageId: string): string
  isComplete(messageId: string): boolean
  hasError(messageId: string): boolean
  cleanup(messageId: string): void
}

// 便捷的chunk类型标签映射
export const CHUNK_TYPE_LABELS: Record<ChunkType, string> = {
  Content: '',
  Thinking: '🤔 **思考过程**',
  ToolResult: '🔧 **工具执行**',
  PlanInfo: '📋 **执行计划**',
  Error: '❌ **错误**',
  Meta: 'ℹ️ **元数据**',
  StreamComplete: '',
}

// 检查chunk类型是否需要标签  
export function needsLabel(chunkType: ChunkType): boolean {
  return chunkType !== 'Content'
}

// 格式化chunk为markdown内容.
export function formatChunk(chunk: OrderedMessageChunk): string {
  if (needsLabel(chunk.chunk_type)) {
    return `${CHUNK_TYPE_LABELS[chunk.chunk_type]}\n${chunk.content}`
  }
  return chunk.content
}
