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
import { useTerminal } from '@/composables/useTerminal'

// Props
interface Props {
  useDocker?: boolean
  dockerImage?: string
  shell?: string
}

const props = withDefaults(defineProps<Props>(), {
  useDocker: true,
  dockerImage: 'sentinel-sandbox:latest',
  shell: 'bash'
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
  if (fitAddon.value && terminal.value) {
    // Use requestAnimationFrame to avoid excessive calls
    requestAnimationFrame(() => {
      try {
        fitAddon.value?.fit()
      } catch (e) {
        // Ignore fit errors during rapid resizing
        console.debug('Terminal fit error:', e)
      }
    })
  }
}

const connect = async () => {
  try {
    isConnecting.value = true
    error.value = ''

    // Start terminal server if not running
    const status = await TerminalAPI.getStatus()
    if (!status.running) {
      await TerminalAPI.startServer()
      // Wait a bit for server to start
      await new Promise(resolve => setTimeout(resolve, 1000))
    }

    // Get WebSocket URL
    const wsUrl = await TerminalAPI.getWebSocketUrl()

    // Create WebSocket connection
    ws.value = new WebSocket(wsUrl)

    ws.value.onopen = () => {
      console.log('WebSocket connected')
      
      // If we have a sessionId from the composable, use it to reconnect
      if (terminalComposable.currentSessionId.value) {
        console.log('Connecting to existing session:', terminalComposable.currentSessionId.value)
        ws.value?.send(`session:${terminalComposable.currentSessionId.value}`)
        return
      }

      // No session ID yet - send default config to create a new session
      // This happens when user opens terminal before any interactive_shell call
      console.log('[Terminal] No session ID, creating new session with default config')
      const config = {
        use_docker: props.useDocker,
        docker_image: props.dockerImage,
        working_dir: '/workspace',
        env_vars: {},
        shell: props.shell,
      }
      ws.value?.send(JSON.stringify(config))
    }

    ws.value.onmessage = (event) => {
      if (typeof event.data === 'string') {
        // Check if it's session ID
        if (event.data.startsWith('session:')) {
          const newSessionId = event.data.substring(8)
          sessionId.value = newSessionId
          isConnected.value = true
          isConnecting.value = false
          
          // Sync to global state so backend tools can find this session
          terminalComposable.setSessionId(newSessionId)
          console.log('[Terminal] ✓ Session established and synced to global state:', newSessionId)
          
          terminal.value?.writeln('\x1b[1;32m✓ Connected!\x1b[0m')
          terminal.value?.writeln('')
        } else {
          // Regular output - write to terminal
          console.log('[Terminal] Received output, length:', event.data.length)
          terminal.value?.write(event.data)
        }
      } else if (event.data instanceof Blob) {
        // Binary data
        event.data.arrayBuffer().then((buffer) => {
          const text = new TextDecoder().decode(buffer)
          terminal.value?.write(text)
        })
      } else if (event.data instanceof ArrayBuffer) {
        const text = new TextDecoder().decode(event.data)
        terminal.value?.write(text)
      }
    }

    ws.value.onerror = (err) => {
      console.error('WebSocket error:', err)
      error.value = 'Connection error'
      isConnecting.value = false
    }

    ws.value.onclose = () => {
      console.log('WebSocket closed')
      isConnected.value = false
      isConnecting.value = false
      terminal.value?.writeln('\r\n\x1b[1;31m✗ Connection closed\x1b[0m')
    }

    // Handle terminal input
    terminal.value?.onData((data) => {
      if (ws.value?.readyState === WebSocket.OPEN) {
        ws.value.send(data)
      }
    })

  } catch (err: any) {
    console.error('Failed to connect:', err)
    error.value = err.message || 'Connection failed'
    isConnecting.value = false
    terminal.value?.writeln(`\r\n\x1b[1;31m✗ Error: ${error.value}\x1b[0m`)
  }
}

const disconnect = async () => {
  if (ws.value) {
    ws.value.close()
    ws.value = null
  }
  isConnected.value = false
  sessionId.value = ''
}

const reconnect = async () => {
  await disconnect()
  // Only reconnect if we have a session ID
  if (terminalComposable.currentSessionId.value) {
    await connect()
  } else {
    error.value = 'No session available. Please create a session first by using interactive_shell tool.'
    terminal.value?.writeln('\r\n\x1b[1;33m⚠ No session available\x1b[0m')
  }
}

const clearTerminal = () => {
  terminal.value?.clear()
}

// Terminal composable
const terminalComposable = useTerminal()
let unregisterWriteCallback: (() => void) | null = null

// Lifecycle
onMounted(() => {
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
  const stopWatch = watch(
    () => terminalComposable.currentSessionId.value,
    async (newSessionId, oldSessionId) => {
      if (newSessionId && newSessionId !== oldSessionId && !isConnected.value) {
        console.log('[Terminal] Session ID changed, reconnecting:', newSessionId)
        await connect()
      }
    }
  )

  // 4. Watch for system font size changes
  let currentFontSizeInterval: any = null
  currentFontSizeInterval = setInterval(() => {
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

  // 6. Cleanup function (returned from setup or registered via onUnmounted)
  // Store reference for clean-up
  const cleanup = () => {
    if (currentFontSizeInterval) clearInterval(currentFontSizeInterval)
    if (stopWatch) stopWatch()
  }
  
  // Vue 3 will handle this correctly since it's registered before any async delay
  onBeforeUnmount(async () => {
    cleanup()
    window.removeEventListener('resize', handleResize)
    await disconnect()
    terminal.value?.dispose()
    
    // Unregister write callback
    if (unregisterWriteCallback) {
      unregisterWriteCallback()
    }
  })
})

onBeforeUnmount(async () => {
  window.removeEventListener('resize', handleResize)
  
  // Disconnect ResizeObserver
  if (resizeObserver.value) {
    resizeObserver.value.disconnect()
    resizeObserver.value = null
  }
  
  await disconnect()
  terminal.value?.dispose()
  
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
  padding: 8px;
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
