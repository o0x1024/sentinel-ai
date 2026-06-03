import type { AgentMessage } from '@/types/agent'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'

export function getExportableConversationMessages(messages: AgentMessage[] = []) {
  return messages.filter(message => {
    if (!message) return false
    if (String(message.content || '').trim()) return true
    return message.metadata && Object.keys(message.metadata).length > 0
  })
}

function formatExportTimestamp(timestamp: number) {
  const date = Number.isFinite(timestamp) ? new Date(timestamp) : new Date()
  return Number.isNaN(date.getTime()) ? '' : date.toISOString()
}

function normalizeExportFilenameSegment(value: string) {
  const normalized = value.trim().replace(/[^a-zA-Z0-9._-]+/g, '-').replace(/^-+|-+$/g, '')
  return normalized || 'conversation'
}

function formatMessageTypeLabel(type: AgentMessage['type']) {
  if (type === 'user') return 'User'
  if (type === 'final') return 'Assistant'
  if (type === 'tool_call') return 'Tool Call'
  if (type === 'tool_result') return 'Tool Result'
  if (type === 'thinking') return 'Thinking'
  if (type === 'planning') return 'Planning'
  if (type === 'progress') return 'Progress'
  if (type === 'error') return 'Error'
  if (type === 'system') return 'System'
  return type
}

function buildExportMetadataBlock(metadata: AgentMessage['metadata']) {
  if (!metadata) return ''
  const exportMetadata = {
    command: metadata.command,
    duration_ms: metadata.duration_ms,
    error: metadata.error,
    exit_code: metadata.exit_code,
    kind: metadata.kind,
    status: metadata.status,
    tool_args: metadata.tool_args,
    tool_call_id: metadata.tool_call_id,
    tool_name: metadata.tool_name,
  }
  const meaningfulEntries = Object.entries(exportMetadata).filter(([, value]) => {
    if (value == null) return false
    if (typeof value === 'string') return value.trim().length > 0
    if (Array.isArray(value)) return value.length > 0
    if (typeof value === 'object') return Object.keys(value).length > 0
    return true
  })
  if (meaningfulEntries.length === 0) return ''
  return `\n\n\`\`\`json\n${JSON.stringify(Object.fromEntries(meaningfulEntries), null, 2)}\n\`\`\``
}

export function buildConversationExportMarkdown(params: {
  conversationId?: string | null
  messages: AgentMessage[]
}) {
  const conversationId = String(params.conversationId || '').trim() || '未保存会话'
  const exportableMessages = getExportableConversationMessages(params.messages)
  const lines = [
    '# AI 助手会话记录',
    '',
    `- 会话 ID: ${conversationId}`,
    `- 导出时间: ${new Date().toISOString()}`,
    `- 消息数: ${exportableMessages.length}`,
    '',
  ]

  exportableMessages.forEach((message, index) => {
    lines.push(`## ${index + 1}. ${formatMessageTypeLabel(message.type)}`)
    lines.push('')
    lines.push(`- 时间: ${formatExportTimestamp(message.timestamp)}`)
    lines.push(`- 消息 ID: ${message.id}`)
    lines.push('')
    const content = String(message.content || '').trim()
    lines.push(content || '_无文本内容_')
    const metadataBlock = buildExportMetadataBlock(message.metadata)
    if (metadataBlock) lines.push(metadataBlock)
    lines.push('')
  })

  return lines.join('\n')
}

export async function downloadConversationExport(params: {
  conversationId?: string | null
  markdown: string
}) {
  const conversationSegment = normalizeExportFilenameSegment(
    String(params.conversationId || 'conversation'),
  )
  const filePath = await save({
    defaultPath: `ai-assistant-${conversationSegment}-${new Date().toISOString().slice(0, 10)}.md`,
    filters: [{ name: 'Markdown', extensions: ['md'] }],
  })
  if (!filePath) return
  await writeTextFile(filePath, params.markdown)
}
