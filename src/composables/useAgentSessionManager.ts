import { ref, computed } from 'vue'
import type { AiConversationSummary } from '@/components/Agent/conversationTypes'

export interface AgentSession {
  id: string          // conversationId
  title: string
  isActive: boolean
}

const SESSION_STORAGE_KEY = 'ai:session-manager'
const sessions = ref<AgentSession[]>([])
const activeSessionId = ref<string | null>(null)
const lastUsedSessionId = ref<string | null>(null)
let hasHydrated = false

const canUseStorage = () => typeof window !== 'undefined' && typeof window.localStorage !== 'undefined'

const normalizeSession = (value: unknown): AgentSession | null => {
  if (!value || typeof value !== 'object') return null

  const candidate = value as Record<string, unknown>
  const id = String(candidate.id || '').trim()
  if (!id) return null

  const title = String(candidate.title || '').trim() || 'New Conversation'
  return {
    id,
    title,
    isActive: false,
  }
}

const persistSessionState = () => {
  if (!canUseStorage()) return

  lastUsedSessionId.value = activeSessionId.value ? String(activeSessionId.value).trim() : null

  try {
    window.localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify({
      activeSessionId: lastUsedSessionId.value,
      sessions: sessions.value.map(({ id, title }) => ({
        id,
        title,
      })),
    }))
  } catch (error) {
    console.warn('[useAgentSessionManager] Failed to persist session state:', error)
  }
}

const hydrateSessionState = () => {
  if (hasHydrated || !canUseStorage()) return
  hasHydrated = true

  try {
    const raw = window.localStorage.getItem(SESSION_STORAGE_KEY)
    if (!raw) return

    const parsed = JSON.parse(raw) as {
      activeSessionId?: unknown
      sessions?: unknown[]
    }
    const restoredSessions = Array.isArray(parsed?.sessions)
      ? parsed.sessions.map(normalizeSession).filter((session): session is AgentSession => !!session)
      : []

    sessions.value = restoredSessions

    const restoredActiveId = String(parsed?.activeSessionId || '').trim()
    lastUsedSessionId.value = restoredActiveId || null
    if (restoredActiveId && restoredSessions.some((session) => session.id === restoredActiveId)) {
      activeSessionId.value = restoredActiveId
      return
    }

    activeSessionId.value = restoredSessions[0]?.id || null
  } catch (error) {
    console.warn('[useAgentSessionManager] Failed to hydrate session state:', error)
  }
}

const replaceSessionsState = (nextSessions: AgentSession[], nextActiveSessionId?: string | null) => {
  sessions.value = nextSessions

  const normalizedActiveId = String(nextActiveSessionId || '').trim()
  if (normalizedActiveId && nextSessions.some((session) => session.id === normalizedActiveId)) {
    activeSessionId.value = normalizedActiveId
    persistSessionState()
    return
  }

  activeSessionId.value = nextSessions[0]?.id || null
  persistSessionState()
}

const normalizeSessionId = (value: string | null | undefined) => String(value || '').trim()

const getFallbackSessionTitle = (title?: string | null) =>
  String(title || '').trim() || 'New Conversation'

export function useAgentSessionManager() {
  hydrateSessionState()

  const activeSession = computed(() => 
    sessions.value.find(s => s.id === activeSessionId.value)
  )
  const preferredBootstrapSessionId = computed(() => {
    const lastUsedId = String(lastUsedSessionId.value || '').trim()
    if (lastUsedId) return lastUsedId
    const activeId = String(activeSessionId.value || '').trim()
    return activeId || null
  })

  const addSession = (id: string, title: string) => {
    if (!sessions.value.find(s => s.id === id)) {
      sessions.value.push({
        id,
        title: title || 'New Conversation',
        isActive: false
      })
    }
    activeSessionId.value = id
    persistSessionState()
  }

  const removeSession = (id: string) => {
    const index = sessions.value.findIndex(s => s.id === id)
    if (index !== -1) {
      sessions.value.splice(index, 1)
      if (activeSessionId.value === id) {
        activeSessionId.value = sessions.value.length > 0 
          ? sessions.value[sessions.value.length - 1].id 
          : null
      }
      persistSessionState()
    }
  }

  const setActiveSession = (id: string) => {
    activeSessionId.value = id
    persistSessionState()
  }

  const updateSessionTitle = (id: string, title: string) => {
    const session = sessions.value.find(s => s.id === id)
    if (session) {
      session.title = title
      persistSessionState()
    }
  }

  const replaceSession = (currentId: string, nextId: string, title?: string | null) => {
    const normalizedCurrentId = normalizeSessionId(currentId)
    const normalizedNextId = normalizeSessionId(nextId)
    if (!normalizedCurrentId || !normalizedNextId) {
      return false
    }

    const currentIndex = sessions.value.findIndex((session) => session.id === normalizedCurrentId)
    if (currentIndex === -1) {
      addSession(normalizedNextId, getFallbackSessionTitle(title))
      return true
    }

    const currentSession = sessions.value[currentIndex]
    const nextTitle = getFallbackSessionTitle(title || currentSession.title)
    const existingIndex = sessions.value.findIndex((session) => session.id === normalizedNextId)

    if (existingIndex !== -1 && existingIndex !== currentIndex) {
      sessions.value[existingIndex] = {
        ...sessions.value[existingIndex],
        title: nextTitle,
      }
      sessions.value.splice(currentIndex, 1)
      activeSessionId.value = normalizedNextId
      persistSessionState()
      return true
    }

    sessions.value[currentIndex] = {
      ...currentSession,
      id: normalizedNextId,
      title: nextTitle,
    }
    activeSessionId.value = normalizedNextId
    persistSessionState()
    return true
  }

  const syncSessionsWithConversations = (conversations: AiConversationSummary[]) => {
    const conversationMap = new Map(
      (Array.isArray(conversations) ? conversations : [])
        .map((conversation) => [String(conversation?.id || '').trim(), conversation] as const)
        .filter(([id]) => !!id),
    )

    const nextSessions = sessions.value
      .map((session) => {
        const matched = conversationMap.get(session.id)
        if (!matched) return null

        return {
          id: session.id,
          title: matched.title || session.title || 'New Conversation',
          isActive: false,
        }
      })
      .filter((session): session is AgentSession => !!session)

    replaceSessionsState(nextSessions, activeSessionId.value)
    return nextSessions
  }

  return {
    sessions,
    activeSessionId,
    activeSession,
    preferredBootstrapSessionId,
    addSession,
    removeSession,
    replaceSession,
    setActiveSession,
    updateSessionTitle,
    syncSessionsWithConversations,
  }
}
