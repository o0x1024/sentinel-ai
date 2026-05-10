<template>
  <div class="interactive-terminal-panel h-full flex flex-col bg-base-100 overflow-hidden">
    <!-- Panel Header -->
    <div class="terminal-panel-header flex items-center justify-between px-4 py-3 bg-base-200 border-b border-base-300">
      <div class="flex items-center gap-3">
        <div class="w-8 h-8 rounded-full bg-primary/20 flex items-center justify-center flex-shrink-0">
          <i class="fas fa-terminal text-primary text-sm"></i>
        </div>
        <div>
          <div class="font-semibold text-sm">{{ $t('agent.interactiveTerminal') }}</div>
          <div v-if="sessionId" class="text-xs text-base-content/60">
            Session: {{ sessionId.substring(0, 8) }}
          </div>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <button
          @click="reconnect"
          :disabled="isConnected"
          class="btn btn-xs btn-ghost"
          :title="$t('agent.reconnect')"
        >
          <i class="fas fa-sync-alt"></i>
        </button>
        <button
          @click="createNewSession"
          :disabled="isConnecting"
          class="btn btn-xs btn-ghost"
          :title="$t('agent.newSession')"
        >
          <i class="fas fa-plus"></i>
        </button>
        <button
          @click="clearTerminal"
          class="btn btn-xs btn-ghost"
          :title="$t('agent.clear')"
        >
          <i class="fas fa-eraser"></i>
        </button>
        <button
          @click="disconnect"
          :disabled="!isConnected"
          class="btn btn-xs btn-ghost text-error"
          :title="$t('agent.disconnect')"
        >
          <i class="fas fa-times"></i>
        </button>
        <button
          @click="$emit('close')"
          class="btn btn-xs btn-ghost"
          :title="$t('agent.close')"
        >
          <i class="fas fa-times-circle"></i>
        </button>
      </div>
    </div>

    <!-- Status Bar -->
    <div class="terminal-status-bar flex items-center gap-2 px-4 py-2 bg-base-100 border-b border-base-300 text-xs">
      <span class="status-indicator flex items-center gap-2">
        <span class="status-dot w-2 h-2 rounded-full" :class="statusDotClass"></span>
        <span>{{ statusText }}</span>
      </span>
      <span v-if="isConnected && sessionId" class="text-base-content/60">
        | Session: {{ sessionId.substring(0, 8) }}
      </span>
    </div>

    <!-- Terminal Container -->
    <div ref="terminalContainer" class="terminal-container flex-1 overflow-hidden bg-[#1e1e1e]"></div>

    <!-- Error Message -->
    <div v-if="error" class="error-bar px-4 py-2 bg-error/10 border-t border-error text-error text-xs">
      <i class="fas fa-exclamation-triangle mr-2"></i>
      {{ error }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, computed, watch } from 'vue'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { WebLinksAddon } from '@xterm/addon-web-links'
import '@xterm/xterm/css/xterm.css'
import TerminalAPI from '@/api/terminal'
import { buildTerminalSessionFingerprint, useTerminal } from '@/composables/useTerminal'
import { invoke } from '@tauri-apps/api/core'

// Props
interface Props {
  useDocker?: boolean
  dockerImage?: string
  shell?: string
  workingDirectory?: string | null
}

type ExecutionMode = 'docker' | 'host'

interface TerminalConfig {
  docker_image: string
  default_execution_mode: ExecutionMode
  host_shell?: string | null
  docker_shell?: string | null
}

interface AgentConfig {
  terminal?: TerminalConfig
  working_directory?: string | null
}

const props = withDefaults(defineProps<Props>(), {
  useDocker: false,
  dockerImage: 'sentinel-sandbox:latest',
  workingDirectory: null,
})

// Actual config (from settings or props)
const actualDockerImage = ref(props.dockerImage)
const actualExecutionMode = ref<ExecutionMode>(props.useDocker ? 'docker' : 'host')
const actualWorkingDirectory = ref(String(props.workingDirectory || '').trim())
const actualHostShell = ref('')
const actualDockerShell = ref('')

const normalizeExecutionMode = (value: unknown): ExecutionMode => {
  if (value === 'docker' || value === 'host') {
    return value
  }
  throw new Error(`Invalid terminal execution mode: ${String(value || '')}`)
}

const resolveTerminalWorkingDirectory = (executionMode: ExecutionMode): string => {
  if (executionMode === 'docker') {
    return '/workspace'
  }
  return String(actualWorkingDirectory.value || '').trim()
}

const defaultShellForExecutionMode = (executionMode: ExecutionMode): string => {
  if (executionMode === 'docker') {
    return 'bash'
  }
  if (typeof navigator !== 'undefined' && navigator.platform.toLowerCase().includes('mac')) {
    return '/bin/zsh'
  }
  return '/bin/bash'
}

const resolvedShell = computed(() => {
  const configuredShell = String(props.shell || '').trim()
  if (configuredShell) return configuredShell
  if (actualExecutionMode.value === 'docker') {
    return actualDockerShell.value.trim() || defaultShellForExecutionMode(actualExecutionMode.value)
  }
  return actualHostShell.value.trim() || defaultShellForExecutionMode(actualExecutionMode.value)
})

// State
const terminalContainer = ref<HTMLElement | null>(null)
const terminal = ref<Terminal | null>(null)
const fitAddon = ref<FitAddon | null>(null)
const ws = ref<WebSocket | null>(null)
const sessionId = ref<string>('')
const isConnected = ref(false)
const isConnecting = ref(false)
const error = ref<string>('')
const resizeObserver = ref<ResizeObserver | null>(null)
let isComponentUnmounted = false
let connectionGeneration = 0

const isConnectionCurrent = (generation: number) => (
  !isComponentUnmounted && generation === connectionGeneration
)

watch(() => props.workingDirectory, (value) => {
  actualWorkingDirectory.value = String(value || '').trim()
})

// Emits
const emit = defineEmits<{
  (e: 'close'): void
}>()

// Computed
const statusDotClass = computed(() => {
  if (isConnected.value) return 'bg-success animate-pulse'
  if (isConnecting.value) return 'bg-warning animate-pulse'
  if (error.value) return 'bg-error'
  return 'bg-base-content/30'
})

const statusText = computed(() => {
  if (isConnected.value) return 'Connected'
  if (isConnecting.value) return 'Connecting...'
  if (error.value) return 'Error'
  return 'Disconnected'
})

// Methods
const initTerminal = () => {
  if (!terminalContainer.value) return

  // 从系统设置中读取字体大小
  let terminalFontSize = 14 // 默认值
  try {
    const savedSettings = localStorage.getItem('sentinel-settings')
    if (savedSettings) {
      const settings = JSON.parse(savedSettings)
      // 系统字体大小范围通常是 12-20，终端使用相同或稍小一点
      if (settings.general?.fontSize) {
        terminalFontSize = settings.general.fontSize
      }
    }
  } catch (error) {
    console.warn('Failed to load font size from settings:', error)
  }

  // Create terminal
  terminal.value = new Terminal({
    cursorBlink: true,
    fontSize: terminalFontSize,
    fontFamily: 'Menlo, Monaco, "Courier New", monospace',
    theme: {
      background: '#1e1e1e',
      foreground: '#d4d4d4',
      cursor: '#d4d4d4',
      black: '#000000',
      red: '#cd3131',
      green: '#0dbc79',
      yellow: '#e5e510',
      blue: '#2472c8',
      magenta: '#bc3fbc',
      cyan: '#11a8cd',
      white: '#e5e5e5',
      brightBlack: '#666666',
      brightRed: '#f14c4c',
      brightGreen: '#23d18b',
      brightYellow: '#f5f543',
      brightBlue: '#3b8eea',
      brightMagenta: '#d670d6',
      brightCyan: '#29b8db',
      brightWhite: '#e5e5e5',
    },
    rows: 30,
    cols: 120,
  })

  // Add addons
  fitAddon.value = new FitAddon()
  terminal.value.loadAddon(fitAddon.value)
  terminal.value.loadAddon(new WebLinksAddon())

  // Open terminal
  terminal.value.open(terminalContainer.value)
  fitAddon.value.fit()
  terminal.value.focus()

  // Handle resize - use both window and container observer
  window.addEventListener('resize', handleResize)
  
  // Use ResizeObserver to detect container size changes (for responsive panel width)
  if (terminalContainer.value) {
    resizeObserver.value = new ResizeObserver(() => {
      handleResize()
    })
    resizeObserver.value.observe(terminalContainer.value)
  }

  // Welcome message
  terminal.value.writeln('\x1b[1;32mSentinel AI Interactive Terminal\x1b[0m')
  terminal.value.writeln('\x1b[1;36mConnecting to terminal server...\x1b[0m')
  terminal.value.writeln('')
}

const handleResize = () => {
  if (isComponentUnmounted) return
  if (fitAddon.value && terminal.value) {
    // Use requestAnimationFrame to avoid excessive calls
    requestAnimationFrame(() => {
      if (isComponentUnmounted || !fitAddon.value || !terminal.value) return
      try {
        fitAddon.value?.fit()
        sendResize()
      } catch (e) {
        // Ignore fit errors during rapid resizing
        console.debug('Terminal fit error:', e)
      }
    })
  }
}

const sendResize = () => {
  if (isComponentUnmounted) return
  if (!terminal.value || !ws.value || ws.value.readyState !== WebSocket.OPEN) return

  try {
    ws.value.send(JSON.stringify({
      type: 'resize',
      rows: terminal.value.rows,
      cols: terminal.value.cols,
    }))
  } catch (error) {
    console.debug('Terminal resize send failed:', error)
  }
}

const connect = async () => {
  const generation = ++connectionGeneration
  try {
    if (isComponentUnmounted) return
    isConnecting.value = true
    error.value = ''

    // Load terminal config from settings
    try {
      const agentConfig = await invoke<AgentConfig>('get_agent_config')
      if (!isConnectionCurrent(generation)) return
      if (!agentConfig?.terminal) {
        throw new Error('Agent terminal config is missing')
      }
      actualDockerImage.value = String(agentConfig.terminal.docker_image || props.dockerImage).trim()
      actualExecutionMode.value = normalizeExecutionMode(agentConfig.terminal.default_execution_mode)
      actualHostShell.value = String(agentConfig.terminal.host_shell || '').trim()
      actualDockerShell.value = String(agentConfig.terminal.docker_shell || '').trim()
      if (actualExecutionMode.value === 'docker' && !actualDockerImage.value) {
        throw new Error('Docker terminal image is empty')
      }
      if (!String(props.workingDirectory || '').trim()) {
        actualWorkingDirectory.value = String(agentConfig.working_directory || '').trim()
      }
      console.log('[Terminal] Loaded config from settings:', actualDockerImage.value, actualExecutionMode.value)
    } catch (e) {
      throw new Error(`Failed to load terminal runtime config: ${e instanceof Error ? e.message : String(e)}`)
    }

    let wsUrl: string

    // Check if already preconnected
    if (terminalComposable.isPreconnected.value && terminalComposable.preconnectedWsUrl.value) {
      console.log('[Terminal] Using preconnected server')
      wsUrl = terminalComposable.preconnectedWsUrl.value
    } else {
      // Start terminal server if not running
      const status = await TerminalAPI.getStatus()
      if (!isConnectionCurrent(generation)) return
      if (!status.running) {
        await TerminalAPI.startServer()
        if (!isConnectionCurrent(generation)) return
        // Wait a bit for server to start
        await new Promise(resolve => setTimeout(resolve, 1000))
        if (!isConnectionCurrent(generation)) return
      }

      // Get WebSocket URL
      wsUrl = await TerminalAPI.getWebSocketUrl()
      if (!isConnectionCurrent(generation)) return
    }

    // Create WebSocket connection
    terminalDecoder = new TextDecoder()
    if (!isConnectionCurrent(generation)) return
    const socket = new WebSocket(wsUrl)
    ws.value = socket

    let pendingExistingSessionId: string | null = null

    socket.onopen = () => {
      if (!isConnectionCurrent(generation) || ws.value !== socket) {
        socket.close()
        return
      }
      console.log('WebSocket connected')
      startKeepAlive()
      const currentFingerprint = buildTerminalSessionFingerprint(
        actualExecutionMode.value,
        actualDockerImage.value,
        resolvedShell.value,
        resolveTerminalWorkingDirectory(actualExecutionMode.value),
      )
      const existingSessionId = terminalComposable.currentSessionId.value
      const existingFingerprint = terminalComposable.currentSessionFingerprint.value

      // If we have a compatible existing session, reconnect.
      if (existingSessionId && existingFingerprint === currentFingerprint) {
        console.log('Connecting to existing session:', existingSessionId)
        pendingExistingSessionId = existingSessionId
        socket.send(`session:${existingSessionId}`)
        return
      }
      if (existingSessionId && existingFingerprint !== currentFingerprint) {
        console.log(
          '[Terminal] Existing session config mismatch, creating new session',
          { existingSessionId, existingFingerprint, currentFingerprint }
        )
        terminalComposable.syncActiveSession(null, null)
      }

      // No session ID yet - send default config to create a new session
      // This happens when user opens terminal before any bound shell session exists.
      console.log('[Terminal] No session ID, creating new session with config:', actualDockerImage.value, actualExecutionMode.value)
      const config = {
        execution_mode: actualExecutionMode.value,
        docker_image: actualDockerImage.value,
        working_dir: resolveTerminalWorkingDirectory(actualExecutionMode.value) || undefined,
        env_vars: {},
        shell: resolvedShell.value,
      }
      socket.send(JSON.stringify(config))
    }

    socket.onmessage = (event) => {
      if (!isConnectionCurrent(generation) || ws.value !== socket) return
      if (typeof event.data === 'string') {
        // Check if it's session ID
        if (event.data.startsWith('session:')) {
          const newSessionId = event.data.substring(8)
          sessionId.value = newSessionId
          isConnected.value = true
          isConnecting.value = false
          const currentFingerprint = buildTerminalSessionFingerprint(
            actualExecutionMode.value,
            actualDockerImage.value,
            resolvedShell.value,
            resolveTerminalWorkingDirectory(actualExecutionMode.value),
          )
          
          // Sync to global state so backend tools can find this session
          terminalComposable.syncActiveSession(newSessionId, currentFingerprint)
          console.log('[Terminal] ✓ Session established and synced to global state:', newSessionId)
          
          terminal.value?.writeln('\x1b[1;32m✓ Connected!\x1b[0m')
          terminal.value?.writeln('')
          handleResize()
        } else {
          // Regular output - write to terminal
          console.log('[Terminal] Received output, length:', event.data.length)
          terminal.value?.write(event.data)
        }
      } else if (event.data instanceof Blob) {
        // Binary data
        event.data.arrayBuffer().then((buffer) => {
          if (!isConnectionCurrent(generation) || ws.value !== socket) return
          const text = terminalDecoder.decode(buffer, { stream: true })
          terminal.value?.write(text)
        })
      } else if (event.data instanceof ArrayBuffer) {
        const text = terminalDecoder.decode(event.data, { stream: true })
        terminal.value?.write(text)
      }
    }

    socket.onerror = (err) => {
      if (!isConnectionCurrent(generation) || ws.value !== socket) return
      console.error('WebSocket error:', err)
      error.value = 'Connection error'
      isConnecting.value = false
      if (keepAliveInterval) {
        clearInterval(keepAliveInterval)
        keepAliveInterval = null
      }
    }

    socket.onclose = () => {
      if (!isConnectionCurrent(generation) || ws.value !== socket) return
      console.log('WebSocket closed')
      if (!isConnected.value && pendingExistingSessionId) {
        terminalComposable.syncActiveSession(null, null)
      }
      isConnected.value = false
      isConnecting.value = false
      terminal.value?.writeln('\r\n\x1b[1;31m✗ Connection closed\x1b[0m')
      if (keepAliveInterval) {
        clearInterval(keepAliveInterval)
        keepAliveInterval = null
      }
    }

    // Handle terminal input - dispose old listener first to avoid duplicates
    if (terminalDataDisposable) {
      try {
        terminalDataDisposable.dispose()
      } catch (error) {
        console.debug('Terminal data listener dispose failed:', error)
      }
      terminalDataDisposable = null
    }
    terminalDataDisposable = terminal.value?.onData((data) => {
      if (!isConnectionCurrent(generation)) return
      if (ws.value?.readyState === WebSocket.OPEN) {
        try {
          ws.value.send(data)
        } catch (error) {
          console.debug('Terminal input send failed:', error)
        }
      }
    }) || null

  } catch (err: any) {
    if (!isConnectionCurrent(generation)) return
    console.error('Failed to connect:', err)
    error.value = err.message || 'Connection failed'
    isConnecting.value = false
    terminal.value?.writeln(`\r\n\x1b[1;31m✗ Error: ${error.value}\x1b[0m`)
  }
}

const disconnect = () => {
  connectionGeneration += 1
  if (ws.value) {
    const socket = ws.value
    socket.onopen = null
    socket.onmessage = null
    socket.onerror = null
    socket.onclose = null
    try {
      socket.close()
    } catch (error) {
      console.debug('Terminal WebSocket close failed:', error)
    }
    ws.value = null
  }
  if (keepAliveInterval) {
    clearInterval(keepAliveInterval)
    keepAliveInterval = null
  }
  // Dispose terminal data listener to avoid duplicates on reconnect
  if (terminalDataDisposable) {
    try {
      terminalDataDisposable.dispose()
    } catch (error) {
      console.debug('Terminal data listener dispose failed:', error)
    }
    terminalDataDisposable = null
  }
  isConnected.value = false
  sessionId.value = ''
}

const reconnect = async () => {
  await disconnect()
  // Try to reconnect with existing session, or create new one
  if (terminalComposable.currentSessionId.value) {
    await connect()
  } else {
    // No session ID, create a new session
    await createNewSession()
  }
}

// Create a new terminal session (clear old session ID first)
const createNewSession = async () => {
  await disconnect()
  // Clear old session ID to force creating a new session
  terminalComposable.syncActiveSession(null, null)
  sessionId.value = ''
  terminal.value?.writeln('\r\n\x1b[1;36mCreating new session...\x1b[0m')
  await connect()
}

const clearTerminal = () => {
  terminal.value?.clear()
}

const switchToSession = async (targetSessionId: string) => {
  if (!targetSessionId) return
  console.log('[Terminal] Switching active session:', {
    from: sessionId.value,
    to: targetSessionId,
  })
  await disconnect()
  await connect()
}

// Terminal composable
const terminalComposable = useTerminal()
let unregisterWriteCallback: (() => void) | null = null
let stopWatch: (() => void) | null = null
let fontSizeInterval: ReturnType<typeof setInterval> | null = null
let keepAliveInterval: ReturnType<typeof setInterval> | null = null
let terminalDataDisposable: { dispose: () => void } | null = null
let terminalDecoder = new TextDecoder()

const startKeepAlive = () => {
  if (keepAliveInterval) {
    clearInterval(keepAliveInterval)
  }
  keepAliveInterval = setInterval(() => {
    if (!isComponentUnmounted && ws.value?.readyState === WebSocket.OPEN) {
      try {
        ws.value.send('__keepalive__')
      } catch (error) {
        console.debug('Terminal keepalive send failed:', error)
      }
    }
  }, 30000)
}

// Lifecycle
onMounted(() => {
  isComponentUnmounted = false
  // 1. Initialize terminal UI immediately
  initTerminal()
  
  // 2. Register write callback immediately so we can receive buffered messages
  // Even if not connected to backend, we can display messages
  unregisterWriteCallback = terminalComposable.onTerminalWrite((content: string) => {
    if (terminal.value) {
      terminal.value.write(content)
    }
  })

  // 3. Watch for session ID changes (in case it's set after connection)
  stopWatch = watch(
    () => terminalComposable.currentSessionId.value,
    async (newSessionId, oldSessionId) => {
      if (newSessionId && newSessionId !== oldSessionId) {
        if (isConnected.value && sessionId.value === newSessionId) {
          return
        }
        console.log('[Terminal] Session ID changed, reconnecting:', {
          from: sessionId.value || oldSessionId || '',
          to: newSessionId,
          connected: isConnected.value,
        })
        await switchToSession(newSessionId)
      } else if (!newSessionId && oldSessionId && isConnected.value) {
        // Session ID was cleared (e.g., new conversation created), disconnect current session
        console.log('[Terminal] Session ID cleared, disconnecting current session')
        await disconnect()
        sessionId.value = ''
        terminal.value?.writeln('\r\n\x1b[1;33m⚠ Session reset (new conversation)\x1b[0m')
      }
    }
  )

  // 4. Watch for system font size changes
  fontSizeInterval = setInterval(() => {
    try {
      const savedSettings = localStorage.getItem('sentinel-settings')
      if (savedSettings && terminal.value) {
        const settings = JSON.parse(savedSettings)
        const newFontSize = settings.general?.fontSize || 14
        const currentFontSize = terminal.value.options.fontSize
        
        if (newFontSize !== currentFontSize) {
          console.log('[Terminal] Font size changed:', currentFontSize, '→', newFontSize)
          terminal.value.options.fontSize = newFontSize
          // 重新计算终端尺寸
          if (fitAddon.value) {
            fitAddon.value.fit()
            sendResize()
          }
        }
      }
    } catch (error) {
      // Ignore errors
    }
  }, 1000) // 每秒检查一次

  // 5. Connect to backend
  // Note: We don't await here to avoid breaking component instance context
  console.log('[Terminal] Initial connection attempt, session ID:', terminalComposable.currentSessionId.value)
  connect()

})

onBeforeUnmount(() => {
  isComponentUnmounted = true
  connectionGeneration += 1
  window.removeEventListener('resize', handleResize)
  if (fontSizeInterval) {
    clearInterval(fontSizeInterval)
    fontSizeInterval = null
  }
  if (stopWatch) {
    stopWatch()
    stopWatch = null
  }
  
  // Disconnect ResizeObserver
  if (resizeObserver.value) {
    resizeObserver.value.disconnect()
    resizeObserver.value = null
  }
  
  try {
    disconnect()
  } catch (error) {
    console.warn('[Terminal] Failed to disconnect during cleanup:', error)
  }
  if (terminal.value) {
    try {
      terminal.value.dispose()
    } catch (error) {
      console.warn('[Terminal] Failed to dispose terminal during cleanup:', error)
    } finally {
      terminal.value = null
      fitAddon.value = null
    }
  }
  
  // Unregister write callback
  if (unregisterWriteCallback) {
    unregisterWriteCallback()
  }
})
</script>

<style scoped>
.interactive-terminal-panel {
  /* Panel takes full height */
}

.terminal-panel-header {
  flex-shrink: 0;
}

.terminal-status-bar {
  flex-shrink: 0;
}

.terminal-container {
  /* Terminal takes remaining space */
  padding: 0;
}

.error-bar {
  flex-shrink: 0;
}

/* Status dot animation */
@keyframes pulse {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.5;
  }
}

.animate-pulse {
  animation: pulse 2s infinite;
}
</style>
