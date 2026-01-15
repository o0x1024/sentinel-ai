/**
 * Terminal Panel Composable
 * Manages interactive terminal state and visibility
 */

import { ref, computed } from 'vue'
import TerminalAPI from '@/api/terminal'

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

// Preconnection state
const preconnectionState = ref<{
  isPreconnecting: boolean
  isPreconnected: boolean
  serverRunning: boolean
  wsUrl: string | null
  error: string | null
}>({
  isPreconnecting: false,
  isPreconnected: false,
  serverRunning: false,
  wsUrl: null,
  error: null,
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

  /**
   * Preconnect terminal server in background
   * This starts the server and gets the WebSocket URL ready
   * so that when user opens terminal panel, connection is instant
   */
  async function preconnect(): Promise<void> {
    if (preconnectionState.value.isPreconnecting || preconnectionState.value.isPreconnected) {
      return
    }

    preconnectionState.value.isPreconnecting = true
    preconnectionState.value.error = null

    try {
      // Check if server is already running
      const status = await TerminalAPI.getStatus()
      if (!status.running) {
        console.log('[useTerminal] Starting terminal server for preconnection...')
        await TerminalAPI.startServer()
        // Wait for server to be ready
        await new Promise(resolve => setTimeout(resolve, 500))
      }

      // Get WebSocket URL
      const wsUrl = await TerminalAPI.getWebSocketUrl()
      
      preconnectionState.value.serverRunning = true
      preconnectionState.value.wsUrl = wsUrl
      preconnectionState.value.isPreconnected = true
      
      console.log('[useTerminal] Terminal server preconnected:', wsUrl)
    } catch (e: any) {
      console.error('[useTerminal] Preconnection failed:', e)
      preconnectionState.value.error = e.message || String(e)
    } finally {
      preconnectionState.value.isPreconnecting = false
    }
  }

  // Preconnection state accessors
  const isPreconnected = computed(() => preconnectionState.value.isPreconnected)
  const preconnectedWsUrl = computed(() => preconnectionState.value.wsUrl)

  return {
    // State
    isTerminalActive,
    currentSessionId,
    hasHistory,
    isPreconnected,
    preconnectedWsUrl,
    
    // Actions
    openTerminal,
    closeTerminal,
    toggleTerminal,
    setSessionId,
    clearTerminal,
    resetTerminal,
    writeToTerminal,
    onTerminalWrite,
    preconnect,
  }
}
