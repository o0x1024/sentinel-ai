import type { AgentMessage } from '@/types/agent'

type TaskToolAction =
  | 'add_items'
  | 'replan'
  | 'update_status'
  | 'get_list'
  | 'reset'
  | 'cleanup'
  | 'update_item'
  | 'delete_item'
  | 'insert_item'
  | 'unknown'

interface TaskToolListItem {
  description?: string
  status?: string
  result?: string | null
}

export interface TaskToolCardData {
  action: TaskToolAction
  title_key: string
  icon_class: string
  item_count: number
  preview: string
  detail: string
}

const parseMaybeJson = (value: unknown): any => {
  if (typeof value !== 'string') return value
  try {
    return JSON.parse(value)
  } catch {
    return value
  }
}

const normalizeAction = (value: unknown): TaskToolAction => {
  const raw = typeof value === 'string' ? value.trim().toLowerCase() : ''
  switch (raw) {
    case 'add_items':
    case 'replan':
    case 'update_status':
    case 'get_list':
    case 'reset':
    case 'cleanup':
    case 'update_item':
    case 'delete_item':
    case 'insert_item':
      return raw
    default:
      return 'unknown'
  }
}

const getItemsFromResult = (value: any): TaskToolListItem[] => {
  const list = value?.list
  const items = list?.items
  return Array.isArray(items) ? items : []
}

const getPreview = (items: TaskToolListItem[], fallbackItems?: unknown): string => {
  const fromResult = items
    .map((item) => (typeof item?.description === 'string' ? item.description.trim() : ''))
    .filter(Boolean)
  const fromArgs = Array.isArray(fallbackItems)
    ? fallbackItems.map((item) => String(item || '').trim()).filter(Boolean)
    : []
  const source = fromResult.length > 0 ? fromResult : fromArgs
  if (source.length === 0) return ''
  const preview = source.slice(0, 3).join('、')
  return source.length > 3 ? `${preview} 等 ${source.length} 项` : preview
}

const getDetail = (result: any, action: TaskToolAction): string => {
  if (typeof result?.message === 'string' && result.message.trim()) {
    return result.message.trim()
  }
  switch (action) {
    case 'add_items':
      return '任务列表已新增规划项。'
    case 'replan':
      return '任务列表已按新计划重建。'
    case 'update_status':
      return '任务状态已更新。'
    case 'get_list':
      return '已读取当前任务列表。'
    case 'reset':
    case 'cleanup':
      return '任务列表已清空。'
    case 'update_item':
      return '任务描述已更新。'
    case 'delete_item':
      return '任务已删除。'
    case 'insert_item':
      return '任务已插入当前计划。'
    default:
      return '任务追踪已更新。'
  }
}

const getTitleKeyAndIcon = (action: TaskToolAction, status: string): { title_key: string; icon_class: string } => {
  if (action === 'add_items' || action === 'replan') {
    return { title_key: 'agent.agentTaskPlannedTitle', icon_class: 'fa-list-check' }
  }
  if (action === 'update_status' && status === 'completed') {
    return { title_key: 'agent.agentTaskCompletedTitle', icon_class: 'fa-check' }
  }
  if (action === 'update_status' && status === 'in_progress') {
    return { title_key: 'agent.agentTaskStartedTitle', icon_class: 'fa-play' }
  }
  if (action === 'get_list') {
    return { title_key: 'agent.agentTaskListReadTitle', icon_class: 'fa-list-ul' }
  }
  return { title_key: 'agent.agentTaskUpdatedTitle', icon_class: 'fa-list-check' }
}

export const buildTaskToolCardData = (message: AgentMessage): TaskToolCardData | null => {
  const toolName = String(message.metadata?.tool_name || '').trim().toLowerCase()
  if (toolName !== 'tasks') return null

  const args = parseMaybeJson(message.metadata?.tool_args) || {}
  const result = parseMaybeJson(message.metadata?.tool_result) || {}
  const action = normalizeAction(args.action)
  const status = typeof args.status === 'string' ? args.status.trim().toLowerCase() : ''
  const items = getItemsFromResult(result)
  const preview = getPreview(items, args.items)
  const detail = getDetail(result, action)
  const itemCount = items.length > 0
    ? items.length
    : Array.isArray(args.items)
      ? args.items.length
      : 0
  const { title_key, icon_class } = getTitleKeyAndIcon(action, status)

  return {
    action,
    title_key,
    icon_class,
    item_count: itemCount,
    preview,
    detail,
  }
}
