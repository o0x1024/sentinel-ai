import type {
  AgentTeamMessage,
  AgentTeamSession,
  TeamBlackboardEntry,
  TeamTask,
} from '@/types/agentTeam'

export interface TeamWorkspaceSnapshot {
  blackboardEntries: TeamBlackboardEntry[]
  sessionDetail: AgentTeamSession | null
  sessionMessages: AgentTeamMessage[]
  tasks: TeamTask[]
}

export const loadTeamWorkspaceData = async (params: {
  activeTeamSessionId?: string | null
  blackboardFetchLimit: number
  ensureSchema: () => Promise<void>
  getMessages: (sessionId: string) => Promise<AgentTeamMessage[]>
  getSession: (sessionId: string) => Promise<AgentTeamSession | null>
  listBlackboardEntries: (sessionId: string, limit: number) => Promise<TeamBlackboardEntry[]>
  listTasks: (sessionId: string) => Promise<TeamTask[]>
  onEmptySession: () => void
  onSnapshotLoaded: (snapshot: TeamWorkspaceSnapshot) => void
  setWorkspaceLoading: (loading: boolean) => void
  syncTeamOrchestrationEditorFromSession: (reset?: boolean) => void
}): Promise<void> => {
  if (!params.activeTeamSessionId) {
    params.onEmptySession()
    params.syncTeamOrchestrationEditorFromSession(true)
    return
  }

  params.setWorkspaceLoading(true)
  try {
    await params.ensureSchema()
    const sessionId = params.activeTeamSessionId
    const [sessionResp, messagesResp, tasksResp, blackboardResp] = await Promise.allSettled([
      params.getSession(sessionId),
      params.getMessages(sessionId),
      params.listTasks(sessionId),
      params.listBlackboardEntries(sessionId, params.blackboardFetchLimit),
    ])

    if (params.activeTeamSessionId !== sessionId) {
      return
    }

    const snapshot: TeamWorkspaceSnapshot = {
      blackboardEntries: [],
      sessionDetail: null,
      sessionMessages: [],
      tasks: [],
    }

    if (sessionResp.status === 'fulfilled') {
      snapshot.sessionDetail = sessionResp.value
    } else {
      console.warn('[AgentView] Failed to load team session detail:', sessionResp.reason)
    }

    if (messagesResp.status === 'fulfilled') {
      snapshot.sessionMessages = messagesResp.value
    } else {
      console.warn('[AgentView] Failed to load team messages:', messagesResp.reason)
    }

    if (tasksResp.status === 'fulfilled') {
      snapshot.tasks = tasksResp.value
    } else {
      console.warn('[AgentView] Failed to load team tasks:', tasksResp.reason)
    }

    if (blackboardResp.status === 'fulfilled') {
      snapshot.blackboardEntries = blackboardResp.value
    } else {
      console.warn('[AgentView] Failed to load team blackboard entries:', blackboardResp.reason)
    }

    params.onSnapshotLoaded(snapshot)
    params.syncTeamOrchestrationEditorFromSession()
  } catch (e) {
    console.error('[AgentView] Failed to load team workspace data:', e)
  } finally {
    params.setWorkspaceLoading(false)
  }
}
