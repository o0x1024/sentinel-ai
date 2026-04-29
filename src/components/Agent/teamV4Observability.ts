import type {
  TeamV4Agent,
  TeamV4Event,
  TeamV4HarnessRun,
  TeamV4Task,
} from '@/types/teamRuntime'

export type TeamRunPhase =
  | 'planning'
  | 'scheduling'
  | 'solver_running'
  | 'tool_running'
  | 'observing'
  | 'recovering'
  | 'waiting_user'
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
    return toObservable(event, role, 'planning', 'info', 'Team run created', 'Commander is preparing the run.', 'Initializing Team runtime.')
  }
  if (type === 'commander_planned') {
    return toObservable(event, role, 'planning', 'info', 'Commander planned the run', 'Task graph and role assignments are ready.', 'Planning completed.')
  }
  if (type === 'commander_task_graph_planned') {
    const taskCount = Number(payload.taskCount || 0)
    return toObservable(event, role, 'planning', 'success', 'Commander planned task graph', `${taskCount} task(s) assigned by solver capability and tool access.`, 'Task graph ready.')
  }
  if (type === 'solver_scheduler_started') {
    return toObservable(event, role, 'scheduling', 'info', 'Solver scheduling started', `${actor} is dispatching tasks to solvers.`, 'Dispatching solver assignments.')
  }
  if (type === 'solver_execution_started') {
    return toObservable(event, role, 'solver_running', 'info', `${actor} started ${task}`, 'Solver is executing the assigned task.', 'Solver is running.')
  }
  if (type === 'solver_text_started') {
    return toObservable(event, role, 'solver_running', 'info', `${actor} is producing output`, `${actor} has started writing or reasoning for ${task}.`, 'Solver output is streaming.')
  }
  if (type === 'solver_tool_started') {
    const toolName = String(payload.toolName || payload.tool_name || 'tool')
    return toObservable(event, role, 'tool_running', 'info', `${actor} called ${toolName}`, `${actor} is using ${toolName} for ${task}.`, 'Tool call in progress.', toolName)
  }
  if (type === 'solver_tool_result') {
    const toolName = String(payload.toolName || payload.tool_name || 'tool')
    const resultSummary = payloadText(payload.summary || payload.result || '')
    return toObservable(event, role, 'tool_running', 'success', `${toolName} returned`, resultSummary || `${actor} received a tool result.`, 'Tool returned a result.', toolName)
  }
  if (type === 'solver_execution_completed') {
    return toObservable(event, role, 'completed', 'success', `${actor} completed ${task}`, 'Solver finished the assigned task.', 'Solver completed.')
  }
  if (type === 'solver_execution_failed') {
    return toObservable(event, role, 'failed', 'error', `${actor} failed ${task}`, payloadText(payload.error || 'Solver failed.'), 'Solver failed.')
  }
  if (type === 'observer_model_review_completed') {
    return toObservable(event, role, 'observing', 'success', 'Observer review completed', 'Observer extracted evidence and risks from solver output.', 'Review completed.')
  }
  if (type === 'observer_model_review_failed') {
    return toObservable(event, role, 'observing', 'warning', 'Observer review failed', payloadText(payload.error || 'Observer could not review output.'), 'Review failed.')
  }
  if (type === 'commander_recovery_decision') {
    return toObservable(event, role, 'recovering', 'warning', 'Commander recovery decision', payloadText(payload.modelReason || payload.reason || payload.decision), 'Recovery path selected.')
  }
  if (type === 'solver_scheduler_completed') {
    return toObservable(event, role, 'completed', 'success', 'Solver scheduling completed', 'All solver assignments finished.', 'Scheduling completed.')
  }
  if (type === 'team_runtime_failed') {
    return toObservable(event, role, 'failed', 'error', 'Team run failed', payloadText(payload.error || 'Runtime failed.'), 'Run failed.')
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
  if (!latestEvent) return { status: agent.status, detail: 'No runtime activity yet.' }
  if (latestEvent.phase === 'tool_running') {
    return { status: 'waiting_tool', detail: latestEvent.title }
  }
  if (latestEvent.phase === 'solver_running') {
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
  return activeEvent?.phase || (normalized === 'running' ? 'solver_running' : 'idle')
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
  const harnessByAgentId = new Map(
    input.harnessRuns
      .filter((harness) => harness.actor_id)
      .map((harness) => [harness.actor_id as string, harness]),
  )
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
