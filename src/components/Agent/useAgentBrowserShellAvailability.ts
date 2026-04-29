import { computed, onBeforeUnmount, onMounted, ref, type Ref } from 'vue'
import { listBrowserShellSessions, type BrowserShellSession } from '@/api/browserShell'

const BROWSER_SHELL_POLL_INTERVAL_MS = 2000

interface UseAgentBrowserShellAvailabilityParams {
  currentBrowserShellSessionId: Ref<string | null>
  clearBoundBrowserShellSession: () => void
}

export function useAgentBrowserShellAvailability(
  params: UseAgentBrowserShellAvailabilityParams,
) {
  const sessions = ref<BrowserShellSession[]>([])
  const refreshInFlight = ref(false)
  let refreshTimer: number | null = null

  const connectedSessions = computed(() =>
    sessions.value.filter(session => session.connected),
  )

  const connectedSessionIds = computed(() =>
    new Set(connectedSessions.value.map(session => session.id)),
  )

  const connectedSessionCount = computed(() => connectedSessions.value.length)

  const hasConnectedBrowserShellSessions = computed(() =>
    connectedSessionCount.value > 0,
  )

  const isCurrentBoundBrowserShellConnected = computed(() => {
    const sessionId = String(params.currentBrowserShellSessionId.value || '').trim()
    if (!sessionId) return false
    return connectedSessionIds.value.has(sessionId)
  })

  async function refreshBrowserShellAvailability() {
    if (refreshInFlight.value) return

    refreshInFlight.value = true
    try {
      const nextSessions = await listBrowserShellSessions()
      sessions.value = nextSessions

      const currentSessionId = String(params.currentBrowserShellSessionId.value || '').trim()
      if (currentSessionId && !nextSessions.some(session => session.id === currentSessionId)) {
        params.clearBoundBrowserShellSession()
      }
    } catch {
      sessions.value = []
    } finally {
      refreshInFlight.value = false
    }
  }

  onMounted(() => {
    void refreshBrowserShellAvailability()
    refreshTimer = window.setInterval(() => {
      void refreshBrowserShellAvailability()
    }, BROWSER_SHELL_POLL_INTERVAL_MS)
  })

  onBeforeUnmount(() => {
    if (refreshTimer !== null) {
      window.clearInterval(refreshTimer)
      refreshTimer = null
    }
  })

  return {
    connectedSessionCount,
    hasConnectedBrowserShellSessions,
    isCurrentBoundBrowserShellConnected,
    refreshBrowserShellAvailability,
  }
}
