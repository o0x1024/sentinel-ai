import type { AgentMessage } from '@/types/agent'
import type { TaskRuntimeItem } from '@/types/taskRuntime'

type AgentTaskEventType = 'planned' | 'created' | 'started' | 'completed'

interface BuildAgentTaskMessagesParams {
  executionId: string
  previousTasks?: TaskRuntimeItem[]
  nextTasks: TaskRuntimeItem[]
  timestamp?: number
}

const buildPreview = (titles: string[]): string => {
  const normalized = titles.map((item) => item.trim()).filter(Boolean)
  if (normalized.length === 0) return ''
  const preview = normalized.slice(0, 3).join('、')
  if (normalized.length <= 3) return preview
  return `${preview} 等 ${normalized.length} 项`
}

const buildMessageContent = (eventType: AgentTaskEventType, taskCount: number, preview: string): string => {
  if (eventType === 'planned') {
    return `已建立 ${taskCount} 个执行任务：${preview}`
  }
  if (eventType === 'created') {
    return `已新增 ${taskCount} 个任务：${preview}`
  }
  if (eventType === 'started') {
    return `开始处理任务：${preview}`
  }
  return `已完成任务：${preview}`
}

const buildTaskSystemMessage = (
  executionId: string,
  eventType: AgentTaskEventType,
  titles: string[],
  timestamp: number,
): AgentMessage | null => {
  const preview = buildPreview(titles)
  if (!preview) return null
  return {
    id: `agent-task:${eventType}:${executionId}:${timestamp}:${preview}`,
    type: 'system',
    content: buildMessageContent(eventType, titles.length, preview),
    timestamp,
    metadata: {
      kind: 'agent_task_update',
      execution_id: executionId,
      task_event_type: eventType,
      task_count: titles.length,
      task_preview: preview,
    } as any,
  }
}

export const buildAgentTaskSystemMessages = (
  params: BuildAgentTaskMessagesParams,
): AgentMessage[] => {
  const previousById = new Map((params.previousTasks || []).map((task) => [task.id, task]))
  const nextTasks = params.nextTasks || []
  if (nextTasks.length === 0) return []

  const timestamp = params.timestamp || Date.now()

  if ((params.previousTasks || []).length === 0) {
    const planned = buildTaskSystemMessage(
      params.executionId,
      'planned',
      nextTasks.map((task) => task.content),
      timestamp,
    )
    return planned ? [planned] : []
  }

  const createdTitles: string[] = []
  const startedTitles: string[] = []
  const completedTitles: string[] = []

  nextTasks.forEach((task) => {
    const previous = previousById.get(task.id)
    if (!previous) {
      createdTitles.push(task.content)
      return
    }
    if (previous.status !== 'in_progress' && task.status === 'in_progress') {
      startedTitles.push(task.active_form || task.content)
    }
    if (previous.status !== 'completed' && task.status === 'completed') {
      completedTitles.push(task.content)
    }
  })

  const events: Array<[AgentTaskEventType, string[]]> = [
    ['created', createdTitles],
    ['started', startedTitles],
    ['completed', completedTitles],
  ]

  return events
    .map(([eventType, titles], index) => buildTaskSystemMessage(
      params.executionId,
      eventType,
      titles,
      timestamp + index,
    ))
    .filter((message): message is AgentMessage => !!message)
}
