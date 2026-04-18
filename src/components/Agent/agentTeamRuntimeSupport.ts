import type {
  AgentTeamMessage,
  AgentTeamRunStatus,
  AgentTeamSession,
  TeamBlackboardEntry,
  TeamTask,
} from '@/types/agentTeam'

export interface TeamMessageSyncState {
  isSyncing: boolean
  pendingSessionId: string | null
}

export interface TeamRuntimeSnapshot {
  sessionDetail?: AgentTeamSession
  tasks?: TeamTask[]
  blackboardEntries?: TeamBlackboardEntry[]
}

export interface TeamMessageSyncResult {
  messages: AgentTeamMessage[] | null
}

export const syncTeamMessagesToMainFlow = async (params: {
  activeTeamSessionId?: string | null
  sessionId?: string | null
  getMessages: (sessionId: string) => Promise<AgentTeamMessage[]>
}): Promise<TeamMessageSyncResult> => {
  const sid = params.sessionId || params.activeTeamSessionId
  if (!sid) {
    return { messages: null }
  }

  const messages = await params.getMessages(sid)
  if (!messages || params.activeTeamSessionId !== sid) {
    return { messages: null }
  }
  return { messages }
}

export const refreshTeamRuntimeData = async (params: {
  activeTeamSessionId?: string | null
  includeBlackboard: boolean
  listBlackboardEntries: () => Promise<TeamBlackboardEntry[] | null>
  listTasks: () => Promise<TeamTask[]>
  loadSession: () => Promise<AgentTeamSession>
  sessionId: string
}): Promise<TeamRuntimeSnapshot | null> => {
  const [sessionResp, tasksResp, blackboardResp] = await Promise.allSettled([
    params.loadSession(),
    params.listTasks(),
    params.includeBlackboard ? params.listBlackboardEntries() : Promise.resolve(null),
  ])

  if (params.activeTeamSessionId !== params.sessionId) {
    return null
  }

  const snapshot: TeamRuntimeSnapshot = {}
  if (sessionResp.status === 'fulfilled') {
    snapshot.sessionDetail = sessionResp.value
  } else {
    console.warn('[AgentView] Failed to refresh team session detail:', sessionResp.reason)
  }

  if (tasksResp.status === 'fulfilled') {
    snapshot.tasks = tasksResp.value
  } else {
    console.warn('[AgentView] Failed to refresh team tasks:', tasksResp.reason)
  }

  if (blackboardResp.status === 'fulfilled') {
    if (blackboardResp.value) {
      snapshot.blackboardEntries = blackboardResp.value
    }
  } else {
    console.warn(
      '[AgentView] Failed to refresh team blackboard entries:',
      blackboardResp.reason,
    )
  }

  return snapshot
}

export const pollTeamRunStatusOnce = async (params: {
  activeTeamSessionId?: string | null
  applyTeamState: (nextState: string) => boolean
  getRunStatus: (sessionId: string) => Promise<AgentTeamRunStatus | null>
  loadTeamWorkspaceData?: () => Promise<void>
  refreshTeamRuntimeData: (sessionId: string) => Promise<void>
  syncTeamMessagesToMainFlow: (sessionId: string) => Promise<void>
  teamModeEnabled: boolean
  teamWorkspaceActive: boolean
}): Promise<void> => {
  const sessionId = params.activeTeamSessionId
  if (!params.teamModeEnabled || !sessionId) return

  await params.syncTeamMessagesToMainFlow(sessionId)
  await params.refreshTeamRuntimeData(sessionId)
  await params.syncTeamMessagesToMainFlow(sessionId)

  const status = await params.getRunStatus(sessionId)
  if (!status || params.activeTeamSessionId !== sessionId) return
  if (typeof status.state === 'string' && status.state.length > 0) {
    const changed = params.applyTeamState(status.state)
    if (changed && params.teamWorkspaceActive && params.loadTeamWorkspaceData) {
      await params.loadTeamWorkspaceData()
    }
  }
}
