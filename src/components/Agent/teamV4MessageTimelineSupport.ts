import type { AgentMessage } from '@/types/agent'
import type { TeamV4Agent, TeamV4Event, TeamV4Task } from '@/types/teamRuntime'
import { parseToolCallArguments } from './agentTeamMessageSupport'

const payloadText = (value: unknown, maxLength = 260) => {
  const source = typeof value === 'string'
    ? value
    : (() => {
        try {
          return JSON.stringify(value ?? '')
        } catch {
          return String(value ?? '')
        }
      })()
  const compact = source.trim().replace(/\s+/g, ' ')
  return compact.length > maxLength ? `${compact.slice(0, maxLength)}...` : compact
}

const timestampMs = (value: string) => {
  const parsed = new Date(value).getTime()
  return Number.isFinite(parsed) ? parsed : Date.now()
}

const agentName = (agentsById: Map<string, TeamV4Agent>, agentId?: string | null) =>
  agentId ? agentsById.get(agentId)?.name || agentId : 'Team'

const agentRole = (agentsById: Map<string, TeamV4Agent>, agentId?: string | null) =>
  agentId ? agentsById.get(agentId)?.role_type || 'system' : 'system'

const taskTitle = (tasksById: Map<string, TeamV4Task>, taskId?: string | null) =>
  taskId ? tasksById.get(taskId)?.title || taskId : 'Team run'

const taskKey = (tasksById: Map<string, TeamV4Task>, taskId?: string | null) =>
  taskId ? tasksById.get(taskId)?.task_key || '' : ''

export const shouldIncludeTeamV4TimelineEventInMainFlow = (event: TeamV4Event): boolean => {
  const type = String(event.event_type || '').trim()
  if (!type || event.visibility === 'internal') return false

  if (type === 'specialist_tool_started' || type === 'specialist_tool_result') return true
  if (type === 'orchestrator_recovery_decision') return true
  if (type === 'orchestrator_preflight_replan_requested') return true
  if (type === 'orchestrator_preflight_waiting_human') return true
  if (type === 'orchestrator_recovery_waiting_human') return true
  if (type === 'waiting_human_timed_out') return true
  if (type === 'team_runtime_failed') return true
  if (type === 'monitor_model_review_failed') return true
  if (type === 'specialist_execution_failed') return true
  if (type === 'harness_failed' || type === 'harness_cancelled' || type === 'harness_expired') {
    return true
  }

  return false
}

const messageTypeForEvent = (eventType: string): AgentMessage['type'] => {
  if (eventType.includes('failed') || eventType === 'team_runtime_failed') return 'error'
  if (eventType === 'specialist_tool_started') return 'tool_call'
  if (eventType === 'specialist_tool_result') return 'tool_call'
  if (eventType.includes('completed') || eventType.includes('heartbeat')) return 'progress'
  return 'planning'
}

const contentForEvent = (
  event: TeamV4Event,
  actor: string,
  task: string,
) => {
  const payload = event.payload || {}
  switch (event.event_type) {
    case 'run_created':
      return 'Team run created.'
    case 'orchestrator_planned':
      return 'Orchestrator prepared the team run.'
    case 'orchestrator_task_graph_planned':
      return `Orchestrator planned ${Number(payload.taskCount || 0)} specialist task(s).`
    case 'specialist_scheduler_started':
      return 'Specialist scheduler started.'
    case 'specialist_scheduler_completed':
      return `Specialist scheduler completed ${Number(payload.completedAssignments || 0)} assignment(s).`
    case 'specialist_execution_started':
      return `${actor} started ${task}.`
    case 'specialist_text_started':
      return `${actor} started producing output for ${task}.`
    case 'specialist_tool_started':
      return `正在调用工具: ${String(payload.toolName || payload.tool_name || 'tool')}`
    case 'specialist_tool_result':
      return `工具调用完成: ${String(payload.toolName || payload.tool_name || 'tool')}`
    case 'specialist_execution_completed':
      return `${actor} completed ${task}.`
    case 'specialist_execution_failed':
      return `${actor} failed ${task}: ${payloadText(payload.error || 'unknown error')}`
    case 'monitor_model_review_completed':
      return `Monitor reviewed ${task}.`
    case 'monitor_model_review_failed':
      return `Monitor review failed: ${payloadText(payload.error || 'unknown error')}`
    case 'orchestrator_recovery_decision':
      return `Orchestrator recovery decision: ${payloadText(payload.modelReason || payload.reason || payload.decision)}`
    case 'orchestrator_preflight_replan_requested':
      return `Orchestrator is replanning before execution: ${payloadText(payload.reason || 'preflight rejected the task graph')}`
    case 'orchestrator_preflight_waiting_human':
      return `Team run is waiting for human input before execution: ${payloadText(payload.reason || 'task graph still violates specialist constraints')}`
    case 'orchestrator_recovery_waiting_human':
      return `Team run is waiting for human input before recovery: ${payloadText(payload.reason || 'recovery requires user input')}`
    case 'waiting_human_timed_out':
      return `Human confirmation timed out. Auto continuing with ${payloadText(payload.timeoutAction || 'configured timeout action')}`
    case 'harness_completed':
      return `${actor} harness completed ${task}.`
    case 'harness_failed':
      return `${actor} harness failed ${task}: ${payloadText(payload.error || payload.details || '')}`
    case 'harness_cancelled':
      return `${actor} harness cancelled ${task}.`
    case 'harness_expired':
      return `${actor} harness lease expired for ${task}.`
    case 'team_runtime_failed':
      return `Team run failed: ${payloadText(payload.error || 'unknown error')}`
    default:
      return `${event.event_type}: ${payloadText(payload)}`
  }
}

export const teamV4TimelineMessageFromEvent = (
  event: TeamV4Event,
  agentsById: Map<string, TeamV4Agent>,
  tasksById: Map<string, TeamV4Task>,
): AgentMessage => {
  const actor = agentName(agentsById, event.actor_id)
  const role = agentRole(agentsById, event.actor_id)
  const task = taskTitle(tasksById, event.task_id)
  const payload = event.payload || {}
  const toolName = String(payload.toolName || payload.tool_name || '').trim()
  const toolCallId =
    typeof payload.toolCallId === 'string' && payload.toolCallId.trim().length > 0
      ? payload.toolCallId.trim()
      : ''
  const isToolEvent =
    event.event_type === 'specialist_tool_started' || event.event_type === 'specialist_tool_result'
  const toolStatus =
    event.event_type === 'specialist_tool_started'
      ? 'running'
      : payload.success === false
        ? 'failed'
        : 'completed'
  const normalizedToolArgs =
    typeof payload.arguments === 'string' || (payload.arguments && typeof payload.arguments === 'object')
      ? parseToolCallArguments(payload.arguments)
      : undefined

  return {
    id: isToolEvent && toolCallId
      ? `teamv4:tool:${event.run_id}:${toolCallId}`
      : `teamv4:event:${event.id}`,
    type: messageTypeForEvent(event.event_type),
    content: contentForEvent(event, actor, task),
    timestamp: timestampMs(event.created_at),
    metadata: {
      kind: 'team_v4_timeline',
      status: isToolEvent
        ? toolStatus
        : messageTypeForEvent(event.event_type) === 'error'
          ? 'failed'
          : 'completed',
      team_event_type: event.event_type,
      team_member_id: event.actor_id || undefined,
      team_member_name: actor,
      team_member_role: role,
      team_sequence: event.sequence,
      team_session_id: event.run_id,
      team_task_record_id: event.task_id || undefined,
      team_task_key: taskKey(tasksById, event.task_id) || undefined,
      team_task_title: task,
      tool_name: toolName || undefined,
      tool_args: normalizedToolArgs,
      tool_call_id: toolCallId || undefined,
      tool_result: event.event_type === 'specialist_tool_result'
        ? typeof payload.result === 'string'
          ? payload.result
          : payload.result !== undefined
            ? JSON.stringify(payload.result)
            : undefined
        : undefined,
      tracked_artifacts: Array.isArray(payload.trackedArtifacts) ? payload.trackedArtifacts : undefined,
      success: payload.success === false ? false : undefined,
    },
  }
}

export const buildTeamV4TimelineMessages = (params: {
  agents: TeamV4Agent[]
  events: TeamV4Event[]
  tasks: TeamV4Task[]
}) => {
  const agentsById = new Map(params.agents.map((agent) => [agent.id, agent]))
  const tasksById = new Map(params.tasks.map((task) => [task.id, task]))
  return params.events
    .filter(shouldIncludeTeamV4TimelineEventInMainFlow)
    .map((event) => teamV4TimelineMessageFromEvent(event, agentsById, tasksById))
    .sort((a, b) => a.timestamp - b.timestamp)
}

export const appendTeamV4TimelineMessages = (
  messages: AgentMessage[],
  timelineMessages: AgentMessage[],
) => {
  const next = [...messages]
  for (const message of timelineMessages) {
    const existingIndex = next.findIndex((item) => item.id === message.id)
    if (existingIndex >= 0) {
      const existing = next[existingIndex]
      next[existingIndex] = {
        ...existing,
        ...message,
        timestamp: existing.timestamp,
        metadata: {
          ...(existing.metadata || {}),
          ...(message.metadata || {}),
        },
      }
      continue
    }
    next.push(message)
  }
  next.sort((a, b) => a.timestamp - b.timestamp)
  return next
}
