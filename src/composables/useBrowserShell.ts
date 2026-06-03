import { computed, ref } from 'vue'

interface BrowserShellState {
  directWriteEnabled: boolean
  sessionId: string | null
}

const browserShellState = ref<BrowserShellState>({
  directWriteEnabled: false,
  sessionId: null,
})

export function useBrowserShell() {
  const directWriteEnabled = computed(() => browserShellState.value.directWriteEnabled)
  const currentSessionId = computed(() => browserShellState.value.sessionId)

  function bindSession(sessionId: string | null) {
    const normalized = String(sessionId || '').trim()
    browserShellState.value.sessionId = normalized || null
    if (!normalized) {
      browserShellState.value.directWriteEnabled = false
    }
  }

  function setDirectWriteEnabled(enabled: boolean) {
    if (!browserShellState.value.sessionId) {
      browserShellState.value.directWriteEnabled = false
      return
    }
    browserShellState.value.directWriteEnabled = enabled === true
  }

  function clearSession() {
    browserShellState.value.sessionId = null
    browserShellState.value.directWriteEnabled = false
  }

  return {
    currentSessionId,
    directWriteEnabled,
    bindSession,
    clearSession,
    setDirectWriteEnabled,
  }
}
