<template>
  <div class="interactive-terminal">
    <div class="terminal-header">
      <div class="terminal-title">
        <span class="terminal-icon">⚡</span>
        <span>{{ $t('tools.interactiveTerminal') }}</span>
        <span v-if="sessionId" class="session-id">Session: {{ sessionId.substring(0, 8) }}</span>
      </div>
      <div class="terminal-actions">
        <button
          @click="reconnect"
          :disabled="isConnected"
          class="btn-action"
          :title="$t('tools.reconnect')"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
        </button>
        <button
          @click="clearTerminal"
          class="btn-action"
          :title="$t('tools.clear')"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
          </svg>
        </button>
        <button
          @click="disconnect"
          :disabled="!isConnected"
          class="btn-action btn-danger"
          :title="$t('tools.disconnect')"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>

    <div class="terminal-status" :class="statusClass">
      <span class="status-dot"></span>
      <span>{{ statusText }}</span>
    </div>

    <div ref="terminalContainer" class="terminal-container"></div>

    <div class="terminal-footer">
      <span class="footer-info">{{ $t('tools.terminalInfo') }}</span>
      <span v-if="isConnected" class="footer-stats">
        {{ $t('tools.connected') }} | {{ $t('tools.session') }}: {{ sessionId?.substring(0, 8) }}
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, computed } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { WebLinksAddon } from '@xterm/addon-web-links'
import '@xterm/xterm/css/xterm.css'

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

// Computed
const statusClass = computed(() => {
  if (isConnected.value) return 'status-connected'
  if (isConnecting.value) return 'status-connecting'
  if (error.value) return 'status-error'
  return 'status-disconnected'
})

const statusText = computed(() => {
  if (isConnected.value) return 'Connected'
  if (isConnecting.value) return 'Connecting...'
  if (error.value) return `Error: ${error.value}`
  return 'Disconnected'
})

// Methods
const initTerminal = () => {
  if (!terminalContainer.value) return

  // Create terminal
  terminal.value = new Terminal({
    cursorBlink: true,
    fontSize: 14,
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

  // Handle resize
  window.addEventListener('resize', handleResize)

  // Welcome message
  terminal.value.writeln('\x1b[1;32mSentinel AI Interactive Terminal\x1b[0m')
  terminal.value.writeln('\x1b[1;36mConnecting to terminal server...\x1b[0m')
  terminal.value.writeln('')
}

const handleResize = () => {
  if (fitAddon.value) {
    fitAddon.value.fit()
  }
}

const connect = async () => {
  try {
    isConnecting.value = true
    error.value = ''

    // Start terminal server if not running
    const status = await invoke<{ running: boolean }>('get_terminal_server_status')
    if (!status.running) {
      await invoke('start_terminal_server')
      // Wait a bit for server to start
      await new Promise(resolve => setTimeout(resolve, 1000))
    }

    // Get WebSocket URL
    const wsUrl = await invoke<string>('get_terminal_websocket_url')

    // Create WebSocket connection
    ws.value = new WebSocket(wsUrl)

    ws.value.onopen = () => {
      console.log('WebSocket connected')
      
      // Send configuration
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
          sessionId.value = event.data.substring(8)
          isConnected.value = true
          isConnecting.value = false
          terminal.value?.writeln('\x1b[1;32m✓ Connected!\x1b[0m')
          terminal.value?.writeln('')
        } else {
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
  await connect()
}

const clearTerminal = () => {
  terminal.value?.clear()
}

// Lifecycle
onMounted(async () => {
  initTerminal()
  await connect()
})

onBeforeUnmount(async () => {
  window.removeEventListener('resize', handleResize)
  await disconnect()
  terminal.value?.dispose()
})
</script>

<style scoped>
.interactive-terminal {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #1e1e1e;
  border-radius: 8px;
  overflow: hidden;
}

.terminal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  background: #2d2d2d;
  border-bottom: 1px solid #3e3e3e;
}

.terminal-title {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #d4d4d4;
  font-weight: 500;
}

.terminal-icon {
  font-size: 20px;
}

.session-id {
  font-size: 12px;
  color: #858585;
  font-family: monospace;
}

.terminal-actions {
  display: flex;
  gap: 8px;
}

.btn-action {
  padding: 6px 8px;
  background: #3e3e3e;
  border: none;
  border-radius: 4px;
  color: #d4d4d4;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-action:hover:not(:disabled) {
  background: #4e4e4e;
}

.btn-action:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-danger:hover:not(:disabled) {
  background: #c92a2a;
}

.terminal-status {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  font-size: 12px;
  background: #252525;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  animation: pulse 2s infinite;
}

.status-connected .status-dot {
  background: #0dbc79;
}

.status-connecting .status-dot {
  background: #e5e510;
}

.status-error .status-dot {
  background: #cd3131;
}

.status-disconnected .status-dot {
  background: #666666;
}

.terminal-container {
  flex: 1;
  padding: 8px;
  overflow: hidden;
}

.terminal-footer {
  display: flex;
  justify-content: space-between;
  padding: 8px 16px;
  background: #252525;
  border-top: 1px solid #3e3e3e;
  font-size: 11px;
  color: #858585;
}

@keyframes pulse {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.5;
  }
}
</style>
