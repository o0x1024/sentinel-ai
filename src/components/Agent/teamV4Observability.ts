import type {
  TeamV4Agent,
  TeamV4Event,
  TeamV4HarnessRun,
  TeamV4Task,
} from '@/types/teamRuntime'

export type TeamRunPhase =
  | 'planning'
  | 'scheduling'
  | 'specialist_running'
  | 'tool_running'
  | 'observing'
  | 'recovering'
  | 'waiting_user'
  | 'expired'
  | 'completed'
  | 'failed'
  | 'cancelled'
  | 'idle'

export type TeamObservableSeverity = 'info' | 'success' | 'warning' | 'error'

export interface TeamObservableEvent {
  id: string
  sequence: number
  runId: string
  taskId?: string | null
  agentId?: string | null
  role: string
  phase: TeamRunPhase
  severity: TeamObservableSeverity
  title: string
  description: string
  progressHint: string
  toolName?: string | null
  elapsedMs?: number | null
  createdAt: string
}

export interface TeamObservableState {
  phase: TeamRunPhase
  headline: string
  detail: string
  activeEvent?: TeamObservableEvent
  recentEvents: TeamObservableEvent[]
  toolEvents: TeamObservableEvent[]
  taskProgress: {
    total: number
    completed: number
    failed: number
    running: number
  }
  agentStates: Array<{
    agent: TeamV4Agent
    status: string
    detail: string
    lastEvent?: TeamObservableEvent
  }>
}

const payloadText = (value: unknown, maxLength = 180) => {
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

const taskTitle = (tasksById: Map<string, TeamV4Task>, taskId?: string | null) =>
  taskId ? tasksById.get(taskId)?.title || taskId : 'Run'

const agentName = (agentsById: Map<string, TeamV4Agent>, agentId?: string | null) =>
  agentId ? agentsById.get(agentId)?.name || agentId : 'Team'

const agentRole = (agentsById: Map<string, TeamV4Agent>, agentId?: string | null) =>
  agentId ? agentsById.get(agentId)?.role_type || 'system' : 'system'

const buildObservableEvent = (
  event: TeamV4Event,
  agentsById: Map<string, TeamV4Agent>,
  tasksById: Map<string, TeamV4Task>,
): TeamObservableEvent => {
  const actor = agentName(agentsById, event.actor_id)
  const role = agentRole(agentsById, event.actor_id)
  const task = taskTitle(tasksById, event.task_id)
  const payload = event.payload || {}
  const type = event.event_type

  if (type === 'run_created') {
    return toObservable(event, role, 'planning', 'info', 'Team run created', 'Orchestrator is preparing the run.', 'Initializing Team runtime.')
  }
  if (type === 'orchestrator_planned') {
    return toObservable(event, role, 'planning', 'info', 'Orchestrator planned the run', 'Task graph and role assignments are ready.', 'Planning completed.')
  }
  if (type === 'orchestrator_task_graph_planned') {
    const taskCount = Number(payload.taskCount || 0)
    return toObservable(event, role, 'planning', 'success', 'Orchestrator planned task graph', `${taskCount} task(s) assigned by specialist capability and tool access.`, 'Task graph ready.')
  }
  if (type === 'specialist_scheduler_started') {
    return toObservable(event, role, 'scheduling', 'info', 'Specialist scheduling started', `${actor} is dispatching tasks to specialists.`, 'Dispatching specialist assignments.')
  }
  if (type === 'specialist_execution_started') {
    return toObservable(event, role, 'specialist_running', 'info', `${actor} started ${task}`, 'Specialist is executing the assigned task.', 'Specialist is running.')
  }
  if (type === 'specialist_text_started') {
    return toObservable(event, role, 'specialist_running', 'info', `${actor} is producing output`, `${actor} has started writing or reasoning for ${task}.`, 'Specialist output is streaming.')
  }
  if (type === 'specialist_tool_started') {
    const toolName = String(payload.toolName || payload.tool_name || 'tool')
    return toObservable(event, role, 'tool_running', 'info', `${actor} called ${toolName}`, `${actor} is using ${toolName} for ${task}.`, 'Tool call in progress.', toolName)
  }
  if (type === 'specialist_tool_result') {
    const toolName = String(payload.toolName || payload.tool_name || 'tool')
    const resultSummary = payloadText(payload.summary || payload.result || '')
    return toObservable(event, role, 'tool_running', 'success', `${toolName} returned`, resultSummary || `${actor} received a tool result.`, 'Tool returned a result.', toolName)
  }
  if (type === 'specialist_execution_completed') {
    return toObservable(event, role, 'completed', 'success', `${actor} completed ${task}`, 'Specialist finished the assigned task.', 'Specialist completed.')
  }
  if (type === 'specialist_execution_failed') {
    return toObservable(event, role, 'failed', 'error', `${actor} failed ${task}`, payloadText(payload.error || 'Specialist failed.'), 'Specialist failed.')
  }
  if (type === 'monitor_model_review_completed') {
    return toObservable(event, role, 'observing', 'success', 'Monitor review completed', 'Monitor extracted evidence and risks from specialist output.', 'Review completed.')
  }
  if (type === 'monitor_model_review_failed') {
    return toObservable(event, role, 'observing', 'warning', 'Monitor review failed', payloadText(payload.error || 'Monitor could not review output.'), 'Review failed.')
  }
  if (type === 'orchestrator_recovery_decision') {
    return toObservable(event, role, 'recovering', 'warning', 'Orchestrator recovery decision', payloadText(payload.modelReason || payload.reason || payload.decision), 'Recovery path selected.')
  }
  if (type === 'specialist_scheduler_completed') {
    return toObservable(event, role, 'completed', 'success', 'Specialist scheduling completed', 'All specialist assignments finished.', 'Scheduling completed.')
  }
  if (type === 'team_runtime_failed') {
    return toObservable(event, role, 'failed', 'error', 'Team run failed', payloadText(payload.error || 'Runtime failed.'), 'Run failed.')
  }
  if (type === 'harness_completed') {
    return toObservable(event, role, 'completed', 'success', `${actor} harness completed`, 'Harness marked the task as completed.', 'Harness completed.')
  }
  if (type === 'harness_failed') {
    return toObservable(event, role, 'failed', 'error', `${actor} harness failed`, payloadText(payload.error || payload.details || 'Harness marked the task as failed.'), 'Harness failed.')
  }
  if (type === 'harness_cancelled') {
    return toObservable(event, role, 'cancelled', 'error', `${actor} harness cancelled`, payloadText(payload.error || 'Harness was cancelled.'), 'Harness cancelled.')
  }
  if (type === 'harness_expired') {
    return toObservable(event, role, 'expired', 'warning', `${actor} harness expired`, 'Harness lease expired before completion.', 'Harness expired.')
  }

  return toObservable(event, role, 'planning', event.visibility === 'user' ? 'info' : 'info', type, payloadText(payload), 'Event recorded.')
}

const toObservable = (
  event: TeamV4Event,
  role: string,
  phase: TeamRunPhase,
  severity: TeamObservableSeverity,
  title: string,
  description: string,
  progressHint: string,
  toolName?: string | null,
): TeamObservableEvent => ({
  id: event.id,
  sequence: event.sequence,
  runId: event.run_id,
  taskId: event.task_id,
  agentId: event.actor_id,
  role,
  phase,
  severity,
  title,
  description,
  progressHint,
  toolName,
  createdAt: event.created_at,
})

const deriveAgentStatus = (
  agent: TeamV4Agent,
  latestEvent?: TeamObservableEvent,
  harnessRun?: TeamV4HarnessRun,
) => {
  const harnessStatus = String(harnessRun?.status || '').toLowerCase()
  if (harnessStatus === 'completed') {
    return { status: 'completed', detail: `Harness completed at ${new Date(harnessRun?.updated_at || '').toLocaleString()}` }
  }
  if (harnessStatus === 'failed') {
    return { status: 'failed', detail: 'Harness marked this task as failed.' }
  }
  if (harnessStatus === 'cancelled') {
    return { status: 'cancelled', detail: 'Harness was cancelled.' }
  }
  if (harnessStatus === 'expired') {
    return { status: 'expired', detail: 'Harness lease expired before completion.' }
  }
  if (!latestEvent) return { status: agent.status, detail: 'No runtime activity yet.' }
  if (latestEvent.phase === 'tool_running') {
    return { status: 'waiting_tool', detail: latestEvent.title }
  }
  if (latestEvent.phase === 'specialist_running') {
    return { status: 'running', detail: latestEvent.progressHint }
  }
  if (latestEvent.phase === 'observing') {
    return { status: 'reviewing', detail: latestEvent.progressHint }
  }
  if (latestEvent.phase === 'recovering') {
    return { status: 'recovering', detail: latestEvent.progressHint }
  }
  if (latestEvent.phase === 'failed') {
    return { status: 'failed', detail: latestEvent.description }
  }
  if (latestEvent.phase === 'completed') {
    return { status: 'completed', detail: latestEvent.progressHint }
  }
  if (harnessRun?.last_heartbeat_at) {
    return { status: 'running', detail: `Last heartbeat: ${new Date(harnessRun.last_heartbeat_at).toLocaleString()}` }
  }
  return { status: agent.status, detail: latestEvent.progressHint }
}

const deriveRunPhase = (runState: string | undefined, activeEvent?: TeamObservableEvent): TeamRunPhase => {
  const normalized = String(runState || '').toLowerCase()
  if (normalized === 'completed') return 'completed'
  if (normalized === 'failed') return 'failed'
  if (normalized === 'cancelled') return 'cancelled'
  if (normalized === 'waiting_human') return 'waiting_user'
  return activeEvent?.phase || (normalized === 'running' ? 'specialist_running' : 'idle')
}

export const deriveTeamV4Observability = (input: {
  agents: TeamV4Agent[]
  events: TeamV4Event[]
  harnessRuns: TeamV4HarnessRun[]
  runState?: string
  tasks: TeamV4Task[]
}): TeamObservableState => {
  const agentsById = new Map(input.agents.map((agent) => [agent.id, agent]))
  const tasksById = new Map(input.tasks.map((task) => [task.id, task]))
  const observableEvents = input.events
    .map((event) => buildObservableEvent(event, agentsById, tasksById))
    .sort((a, b) => a.sequence - b.sequence)
  const activeEvent = [...observableEvents]
    .reverse()
    .find((event) => !['completed', 'failed', 'cancelled'].includes(event.phase))
    || observableEvents[observableEvents.length - 1]
  const phase = deriveRunPhase(input.runState, activeEvent)
  const taskProgress = {
    total: input.tasks.length,
    completed: input.tasks.filter((task) => task.status === 'completed').length,
    failed: input.tasks.filter((task) => task.status === 'failed').length,
    running: input.tasks.filter((task) => ['running', 'ready'].includes(task.status)).length,
  }
  const harnessByAgentId = new Map<string, TeamV4HarnessRun>()
  for (const harness of input.harnessRuns.filter((item) => item.actor_id)) {
    const agentId = harness.actor_id as string
    const current = harnessByAgentId.get(agentId)
    const currentTime = new Date(current?.updated_at || current?.created_at || '').getTime()
    const nextTime = new Date(harness.updated_at || harness.created_at || '').getTime()
    if (!current || (Number.isFinite(nextTime) && (!Number.isFinite(currentTime) || nextTime >= currentTime))) {
      harnessByAgentId.set(agentId, harness)
    }
  }
  const agentStates = input.agents.map((agent) => {
    const lastEvent = [...observableEvents].reverse().find((event) => event.agentId === agent.id)
    const derived = deriveAgentStatus(agent, lastEvent, harnessByAgentId.get(agent.id))
    return {
      agent,
      lastEvent,
      status: derived.status,
      detail: derived.detail,
    }
  })

  return {
    phase,
    headline: activeEvent?.title || 'No active Team activity',
    detail: activeEvent?.description || 'Team activity will appear after execution starts.',
    activeEvent,
    recentEvents: observableEvents.slice(-20).reverse(),
    toolEvents: observableEvents.filter((event) => event.toolName).slice(-20).reverse(),
    taskProgress,
    agentStates,
  }
}
