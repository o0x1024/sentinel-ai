import type { TeamTaskActionResult } from '@/types/agentTeam'

export type TeamTaskActionKind = TeamTaskActionResult['action']

type TeamTaskActionInput = {
  taskId: string
  taskKey?: string | null
  title?: string | null
  status?: string | null
  error?: unknown
}

const resolveTaskLabel = (input: TeamTaskActionInput): string => {
  const title = String(input.title || '').trim()
  if (title) return title
  const taskKey = String(input.taskKey || '').trim()
  if (taskKey) return taskKey
  const taskId = String(input.taskId || '').trim()
  return taskId || '未知任务'
}

const normalizeErrorText = (error: unknown): string => {
  if (error instanceof Error) return error.message
  if (typeof error === 'string') return error
  return String(error || '')
}

const inferReason = (action: TeamTaskActionKind, error: unknown): string => {
  const text = normalizeErrorText(error).trim()
  const normalized = text.toLowerCase()
  if (!normalized) return '请求未成功完成，请稍后重试。'
  if (normalized.includes('already claimed')) return '任务已被其他成员占用。'
  if (normalized.includes('not currently claimed by this agent')) return '当前负责人不是该成员，无法直接释放。'
  if (normalized.includes('terminal')) return '任务已处于终态，不能再继续认领或释放。'
  if (normalized.includes('task not found')) return '任务不存在，或已从当前任务板移除。'
  if (normalized.includes('support mysql')) return '当前数据库后端不支持该团队任务操作。'
  if (action === 'create') return `创建失败：${text}`
  if (action === 'claim') return `认领失败：${text}`
  if (action === 'release') return `释放失败：${text}`
  if (action === 'complete') return `标记完成失败：${text}`
  if (action === 'fail') return `标记失败失败：${text}`
  return `标记阻塞失败：${text}`
}

const nextStepForSuccess = (action: TeamTaskActionKind): string => {
  switch (action) {
    case 'create':
      return '如需立即推进，请分配负责人或直接认领。'
    case 'claim':
      return '开始执行后请及时更新结果和状态。'
    case 'release':
      return '该任务已回到共享任务池，可由其他成员继续处理。'
    case 'complete':
      return '请检查是否有其他任务因此解除依赖。'
    case 'fail':
      return '请补充失败原因，或创建后续修复任务。'
    case 'block':
      return '请补充阻塞原因，并等待依赖解除后继续。'
    default:
      return '请继续查看任务板中的后续可执行任务。'
  }
}

const nextStepForFailure = (action: TeamTaskActionKind): string => {
  switch (action) {
    case 'create':
      return '请检查任务标题、说明和依赖信息后重试。'
    case 'claim':
      return '请先查看任务最新状态和依赖，再决定是否改领其他任务。'
    case 'release':
      return '请先确认当前负责人和任务状态，再决定是否释放或转交。'
    case 'complete':
      return '请先回读任务结果，再决定是否完成。'
    case 'fail':
      return '请补充明确失败原因后再重试。'
    case 'block':
      return '请先确认阻塞原因或依赖，再更新状态。'
    default:
      return '请刷新任务列表后再重试。'
  }
}

const successPrefix = (action: TeamTaskActionKind): string => {
  switch (action) {
    case 'create':
      return '任务已创建'
    case 'claim':
      return '任务已认领'
    case 'release':
      return '任务已释放'
    case 'complete':
      return '任务已完成'
    case 'fail':
      return '任务已标记为失败'
    case 'block':
      return '任务已标记为等待依赖'
    default:
      return '任务已更新'
  }
}

const failurePrefix = (action: TeamTaskActionKind): string => {
  switch (action) {
    case 'create':
      return '任务创建失败'
    case 'claim':
      return '任务暂不可认领'
    case 'release':
      return '任务暂不可释放'
    case 'complete':
      return '任务暂不可标记为完成'
    case 'fail':
      return '任务暂不可标记为失败'
    case 'block':
      return '任务暂不可标记为等待依赖'
    default:
      return '任务暂不可更新'
  }
}

export const buildTeamTaskActionSuccess = (
  action: TeamTaskActionKind,
  input: TeamTaskActionInput,
): TeamTaskActionResult => ({
  success: true,
  action,
  task_id: input.taskId,
  task_key: input.taskKey ?? null,
  title: input.title ?? null,
  status: input.status ?? null,
  message: `${successPrefix(action)}：${resolveTaskLabel(input)}`,
  reason: null,
  next_step: nextStepForSuccess(action),
})

export const buildTeamTaskActionFailure = (
  action: TeamTaskActionKind,
  input: TeamTaskActionInput,
): TeamTaskActionResult => ({
  success: false,
  action,
  task_id: input.taskId,
  task_key: input.taskKey ?? null,
  title: input.title ?? null,
  status: input.status ?? null,
  message: `${failurePrefix(action)}：${resolveTaskLabel(input)}`,
  reason: inferReason(action, input.error),
  next_step: nextStepForFailure(action),
})
