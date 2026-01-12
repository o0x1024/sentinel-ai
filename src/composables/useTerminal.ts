/**
 * Terminal Panel Composable
 * Manages interactive terminal state and visibility
 */

import { ref, computed } from 'vue'

interface TerminalState {
  isActive: boolean
  sessionId: string | null
  hasHistory: boolean
}

interface TerminalWriteEvent {
  content: string
  timestamp: number
}

// Global state for terminal panel
const terminalState = ref<TerminalState>({
  isActive: false,
  sessionId: null,
  hasHistory: false,
})

// Event bus for writing to terminal
const terminalWriteCallbacks = new Set<(content: string) => void>()

export function useTerminal() {
  const isTerminalActive = computed(() => terminalState.value.isActive)
  const currentSessionId = computed(() => terminalState.value.sessionId)
  const hasHistory = computed(() => terminalState.value.hasHistory)

  /**
   * Open terminal panel
   */
  function openTerminal(sessionId?: string) {
    terminalState.value.isActive = true
    if (sessionId) {
      terminalState.value.sessionId = sessionId
    }
    terminalState.value.hasHistory = true
  }

  /**
   * Close terminal panel
   */
  function closeTerminal() {
    terminalState.value.isActive = false
  }

  /**
   * Toggle terminal panel
   */
  function toggleTerminal() {
    terminalState.value.isActive = !terminalState.value.isActive
  }

  /**
   * Set session ID
   */
  function setSessionId(sessionId: string | null) {
    terminalState.value.sessionId = sessionId
    if (sessionId) {
      terminalState.value.hasHistory = true
    }
  }

  /**
   * Clear terminal state
   */
  function clearTerminal() {
    terminalState.value.sessionId = null
    terminalState.value.hasHistory = false
  }

  /**
   * Reset terminal (close and clear)
   */
  function resetTerminal() {
    terminalState.value.isActive = false
    terminalState.value.sessionId = null
    terminalState.value.hasHistory = false
  }

  /**
   * Write content to terminal
   * This will notify all registered terminal components
   */
  function writeToTerminal(content: string) {
    terminalWriteCallbacks.forEach(callback => {
      try {
        callback(content)
      } catch (e) {
        console.error('[useTerminal] Error in write callback:', e)
      }
    })
  }

  /**
   * Register a callback to receive terminal write events
   * Returns an unregister function
   */
  function onTerminalWrite(callback: (content: string) => void): () => void {
    terminalWriteCallbacks.add(callback)
    return () => {
      terminalWriteCallbacks.delete(callback)
    }
  }

  return {
    // State
    isTerminalActive,
    currentSessionId,
    hasHistory,
    
    // Actions
    openTerminal,
    closeTerminal,
    toggleTerminal,
    setSessionId,
    clearTerminal,
    resetTerminal,
    writeToTerminal,
    onTerminalWrite,
  }
}
