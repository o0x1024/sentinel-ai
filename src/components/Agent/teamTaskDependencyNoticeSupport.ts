import type { AgentTeamMessage, TeamTask } from '@/types/agentTeam'
import {
  resolveSatisfiedTeamTaskDependencies,
  teamTaskSectionKey,
} from './teamWorkspacePresentation'

export const TEAM_DEPENDENCY_READY_KIND = 'team_dependency_ready'

const normalize = (value: unknown): string => String(value || '').trim()

export const buildTeamDependencyReadyNoticeId = (sessionId: string, task: TeamTask): string => {
  return [
    TEAM_DEPENDENCY_READY_KIND,
    sessionId,
    normalize(task.id) || normalize(task.task_id) || 'task',
    normalize(task.updated_at) || normalize(task.created_at) || 'now',
  ].join(':')
}

export const getTeamDependencyReadyNoticeKey = (
  message: Pick<AgentTeamMessage, 'id' | 'metadata'>,
): string => {
  const explicit = normalize(message.metadata?.notice_id)
  if (explicit) return explicit
  const fallback = normalize(message.id)
  return fallback.startsWith(`${TEAM_DEPENDENCY_READY_KIND}:`) ? fallback : ''
}

export const buildTeamDependencyReadyMessageRequest = (notice: AgentTeamMessage) => ({
  thread_id: notice.session_id,
  from_agent_id: 'team_system',
  to_agent_id: null,
  message_type: 'system',
  payload: {
    content: notice.content,
    message: notice.content,
    metadata: notice.metadata ?? {},
  },
})

export const buildTeamDependencyReadyNotices = (params: {
  existingMessages?: AgentTeamMessage[]
  nextTasks: TeamTask[]
  previousTasks: TeamTask[]
  sessionId: string
}): AgentTeamMessage[] => {
  const sessionId = normalize(params.sessionId)
  if (!sessionId || params.previousTasks.length === 0 || params.nextTasks.length === 0) {
    return []
  }

  const previousById = new Map<string, TeamTask>()
  for (const task of params.previousTasks) {
    const id = normalize(task.id)
    if (id) previousById.set(id, task)
  }

  const existingKeys = new Set(
    (params.existingMessages || [])
      .map((message) => getTeamDependencyReadyNoticeKey(message))
      .filter(Boolean),
  )

  return params.nextTasks.flatMap((task) => {
    const id = normalize(task.id)
    if (!id) return []
    const previous = previousById.get(id)
    if (!previous) return []

    const previousSection = teamTaskSectionKey(previous, params.previousTasks)
    const nextSection = teamTaskSectionKey(task, params.nextTasks)
    if (previousSection !== 'blocked' || nextSection !== 'executable') return []

    const noticeId = buildTeamDependencyReadyNoticeId(sessionId, task)
    if (existingKeys.has(noticeId)) return []

    const title = normalize(task.title) || normalize(task.task_id) || id
    const satisfied = resolveSatisfiedTeamTaskDependencies(task, params.nextTasks)
    const detail = satisfied.length > 0
      ? `已完成前置任务：${satisfied.join('、')}`
      : '前置依赖已满足。'

    return [{
      id: noticeId,
      session_id: sessionId,
      role: 'system',
      content: `任务现在可开始执行：${title}。${detail}`,
      member_name: 'system',
      metadata: {
        kind: TEAM_DEPENDENCY_READY_KIND,
        notice_id: noticeId,
        task_record_id: id,
        task_key: normalize(task.task_id),
        task_title: title,
        action_label: '查看任务',
      },
      timestamp: new Date().toISOString(),
    }]
  })
}
