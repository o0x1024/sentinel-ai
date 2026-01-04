import { ref, computed } from 'vue'

export interface AgentSession {
  id: string          // conversationId
  title: string
  isActive: boolean
}

const sessions = ref<AgentSession[]>([])
const activeSessionId = ref<string | null>(null)

export function useAgentSessionManager() {
  const activeSession = computed(() => 
    sessions.value.find(s => s.id === activeSessionId.value)
  )

  const addSession = (id: string, title: string) => {
    if (!sessions.value.find(s => s.id === id)) {
      sessions.value.push({
        id,
        title: title || 'New Conversation',
        isActive: false
      })
    }
    activeSessionId.value = id
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
    }
  }

  const setActiveSession = (id: string) => {
    activeSessionId.value = id
  }

  const updateSessionTitle = (id: string, title: string) => {
    const session = sessions.value.find(s => s.id === id)
    if (session) {
      session.title = title
    }
  }

  return {
    sessions,
    activeSessionId,
    activeSession,
    addSession,
    removeSession,
    setActiveSession,
    updateSessionTitle
  }
}
