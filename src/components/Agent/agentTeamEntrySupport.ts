import type { AssistantConversationBinding } from './agentDraftTypes'
import type { AgentTeamSession } from '@/types/agentTeam'
import { normalizeTeamHumanInputContent } from './agentTeamMessageSupport'
import { buildTeamSessionName } from './agentTeamSessionSupport'

export const ensureConversationForTeamSession = async (params: {
  conversationId?: string | null
  conversationBinding?: AssistantConversationBinding | null
  createConversation: (request: {
    title: string
    service_name: string
    conversation_binding?: AssistantConversationBinding | null
  }) => Promise<string>
  getConversationTitle: () => string
  getDisplayTitle: () => string
  loadConversationList?: () => void
  onConversationReady: (conversationId: string, title: string) => void
  syncActiveTeamSession: () => Promise<void>
}): Promise<string | null | undefined> => {
  if (params.conversationId) {
    return params.conversationId
  }

  const conversationId = await params.createConversation({
    conversation_binding: params.conversationBinding,
    title: params.getConversationTitle(),
    service_name: 'default',
  })
  params.onConversationReady(conversationId, params.getDisplayTitle())
  params.loadConversationList?.()
  await params.syncActiveTeamSession()
  return conversationId
}

export const createAndStartTeamSession = async (params: {
  conversationId?: string | null
  goal: string
  createSession: (request: {
    name: string
    goal: string
    conversation_id?: string
  }) => Promise<AgentTeamSession>
  loadTeamWorkspaceData?: () => Promise<void>
  onSessionReady: (session: AgentTeamSession) => void
  pushTeamHumanInputLocalEcho: (sessionId: string, content: string) => void
  submitMessage: (request: {
    session_id: string
    content: string
    resume: boolean
  }) => Promise<void>
  syncTeamMessagesToMainFlow: (sessionId: string) => Promise<void>
  teamWorkspaceActive: boolean
}): Promise<AgentTeamSession> => {
  const normalizedGoal = normalizeTeamHumanInputContent(params.goal)
  if (!normalizedGoal) {
    throw new Error('Team 输入不能为空。')
  }

  const session = await params.createSession({
    name: buildTeamSessionName(normalizedGoal),
    goal: normalizedGoal,
    conversation_id: params.conversationId || undefined,
  })
  params.onSessionReady(session)
  params.pushTeamHumanInputLocalEcho(session.id, normalizedGoal)
  await params.submitMessage({
    session_id: session.id,
    content: normalizedGoal,
    resume: false,
  })
  await params.syncTeamMessagesToMainFlow(session.id)
  if (params.teamWorkspaceActive && params.loadTeamWorkspaceData) {
    await params.loadTeamWorkspaceData()
  }
  return session
}

export const routeTeamMessage = async (params: {
  activeTeamSessionId?: string | null
  content: string
  createAndStartTeamSession: (goal: string) => Promise<AgentTeamSession>
  getSession: (sessionId: string) => Promise<AgentTeamSession | null>
  loadTeamWorkspaceData?: () => Promise<void>
  onSessionStateChange: (state: string) => void
  pushTeamHumanInputLocalEcho: (sessionId: string, content: string) => void
  submitMessage: (request: {
    session_id: string
    content: string
    resume: boolean
  }) => Promise<void>
  syncTeamMessagesToMainFlow: (sessionId: string) => Promise<void>
  teamWorkspaceActive: boolean
}): Promise<AgentTeamSession | null> => {
  const normalizedContent = normalizeTeamHumanInputContent(params.content)
  if (!normalizedContent) {
    return null
  }

  const currentSession = params.activeTeamSessionId
    ? await params.getSession(params.activeTeamSessionId)
    : null

  if (!currentSession || currentSession.state === 'ARCHIVED') {
    return params.createAndStartTeamSession(normalizedContent)
  }

  params.pushTeamHumanInputLocalEcho(currentSession.id, normalizedContent)
  await params.submitMessage({
    session_id: currentSession.id,
    content: normalizedContent,
    resume: currentSession.state === 'SUSPENDED_FOR_HUMAN',
  })
  await params.syncTeamMessagesToMainFlow(currentSession.id)
  params.onSessionStateChange(currentSession.state)
  if (params.teamWorkspaceActive && params.loadTeamWorkspaceData) {
    await params.loadTeamWorkspaceData()
  }
  return currentSession
}

export const startTeamExecutionRun = async (params: {
  appendTeamBridgeMessage?: (message: string) => void
  bridgeMessage?: string
  conversationId?: string | null
  ensureTeamRunStatusPolling: () => void
  flushPendingToolConfigSave: () => Promise<void>
  loadTeamWorkspaceData?: () => Promise<void>
  persistTeamSessionState: (sessionId: string, nextState: string) => Promise<void>
  ragEnabled: boolean
  runtimeToolConfig: unknown
  sessionId?: string | null
  setTeamSessionState: (state: string) => void
  startRun: (
    sessionId: string,
    conversationId?: string,
    ragEnabled?: boolean,
    toolConfig?: unknown,
  ) => Promise<void>
  teamWorkspaceActive: boolean
}): Promise<void> => {
  if (!params.sessionId) {
    throw new Error('Team 会话不存在。')
  }

  params.setTeamSessionState('EXECUTING')
  params.ensureTeamRunStatusPolling()
  await params.persistTeamSessionState(params.sessionId, 'EXECUTING')

  const bridgeMessage = (params.bridgeMessage || '').trim()
  if (bridgeMessage) {
    params.appendTeamBridgeMessage?.(bridgeMessage)
  }

  await params.flushPendingToolConfigSave()
  await params.startRun(
    params.sessionId,
    params.conversationId || undefined,
    params.ragEnabled,
    params.runtimeToolConfig,
  )
  if (params.teamWorkspaceActive && params.loadTeamWorkspaceData) {
    await params.loadTeamWorkspaceData()
  }
}
