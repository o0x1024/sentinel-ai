<template>
  <dialog :class="['modal', { 'modal-open': modelValue }]">
    <div v-if="modelValue" class="modal-box w-11/12 max-w-5xl h-[80vh] flex flex-col p-0">
      <!-- Header -->
      <div class="flex justify-between items-center px-4 py-3 border-b border-base-300 bg-base-200">
        <div class="flex items-center gap-3">
          <div class="w-8 h-8 rounded bg-success/20 flex items-center justify-center">
            <!-- <i class="fas fa-terminal text-success"></i> -->
          </div>
          <div>
            <h3 class="font-bold text-lg">{{ $t('tools.shell.title') }}</h3>
            <!-- <span class="text-xs text-base-content/60">{{ cwd }}</span> -->
          </div>
        </div>
        <div class="flex items-center gap-2">
          <button @click="clearTerminal" class="btn btn-sm btn-ghost" :title="$t('tools.shell.clear')">
            <i class="fas fa-trash-alt"></i>
          </button>
          <button @click="openSettings" class="btn btn-sm btn-ghost" :title="$t('tools.shell.settings')">
            <i class="fas fa-cog"></i>
          </button>
          <button @click="close" class="btn btn-sm btn-ghost">
            <i class="fas fa-times"></i>
          </button>
        </div>
      </div>

      <!-- Terminal Output -->
      <div 
        ref="outputRef"
        class="flex-1 overflow-y-auto p-4 font-mono text-sm bg-[#1e1e1e] text-[#d4d4d4]"
        @click="focusInput"
      >
        <!-- Welcome message -->
        <div v-if="history.length === 0" class="text-[#569cd6] mb-4">
          <div>{{ $t('tools.shell.welcome') }}</div>
          <div class="text-[#6a9955]">{{ $t('tools.shell.helpHint') }}</div>
          <div class="text-[#6a9955]">{{ $t('tools.shell.historyHint') }}</div>
        </div>

        <!-- Command history -->
        <div v-for="(item, index) in history" :key="index" class="mb-2">
          <!-- Command line -->
          <div class="flex items-start gap-2">
            <span class="text-[#4ec9b0] shrink-0">{{ item.prompt }}</span>
            <span class="text-[#ce9178] break-all">{{ item.command }}</span>
          </div>
          <!-- Output -->
          <div v-if="item.output" class="mt-1 whitespace-pre-wrap break-all" :class="item.success ? 'text-[#d4d4d4]' : 'text-[#f14c4c]'">{{ item.output }}</div>
          <!-- Execution time -->
          <div v-if="item.executionTime !== undefined" class="text-[#6a9955] text-xs mt-1">
            {{ $t('tools.shell.completed', { time: item.executionTime, code: item.exitCode ?? 'N/A' }) }}
          </div>
        </div>

        <!-- Current input line (when executing) -->
        <div v-if="isExecuting" class="flex items-center gap-2">
          <span class="text-[#4ec9b0]">{{ currentPrompt }}</span>
          <span class="loading loading-dots loading-sm text-warning"></span>
        </div>
      </div>

      <!-- Input Area -->
      <div class="border-t border-base-300 bg-[#252526] p-3">
        <div class="flex items-center gap-2 font-mono">
          <span class="text-[#4ec9b0] text-sm shrink-0">{{ currentPrompt }}</span>
          <input
            ref="inputRef"
            v-model="currentCommand"
            @keydown="handleKeyDown"
            @paste="handlePaste"
            class="flex-1 bg-transparent border-none outline-none text-[#ce9178] text-sm"
            :placeholder="isExecuting ? $t('tools.shell.executing') : $t('tools.shell.enterCommand')"
            :disabled="isExecuting"
            autocomplete="off"
            spellcheck="false"
          />
        </div>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop" @click="close">
      <button>close</button>
    </form>
  </dialog>

  <!-- Settings Modal -->
  <ShellConfigModal v-model="showSettings" />
</template>

<script setup lang="ts">
import { ref, watch, nextTick, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import ShellConfigModal from './ShellConfigModal.vue'

interface HistoryItem {
  prompt: string
  command: string
  output: string
  success: boolean
  exitCode: number | null
  executionTime: number
}

const props = defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

// State
const history = ref<HistoryItem[]>([])
const currentCommand = ref('')
const isExecuting = ref(false)
const cwd = ref('~')
const commandHistory = ref<string[]>([])
const historyIndex = ref(-1)
const showSettings = ref(false)

// Refs
const outputRef = ref<HTMLElement | null>(null)
const inputRef = ref<HTMLInputElement | null>(null)

// Computed
const currentPrompt = computed(() => {
  const user = getUsername()
  const host = getHostname()
  const path = formatPath(cwd.value)
  return `${user}@${host}:${path}$`
})

// Helper functions
function getUsername(): string {
  // Try to extract from cwd or use generic name
  if (cwd.value.includes('/Users/')) {
    const match = cwd.value.match(/\/Users\/([^/]+)/)
    return match ? match[1] : 'user'
  }
  if (cwd.value.includes('/home/')) {
    const match = cwd.value.match(/\/home\/([^/]+)/)
    return match ? match[1] : 'user'
  }
  return 'user'
}

function getHostname(): string {
  return 'sentinel'
}

function formatPath(path: string): string {
  // Replace home directory with ~
  if (path.includes('/Users/')) {
    const match = path.match(/\/Users\/([^/]+)/)
    if (match) {
      return path.replace(`/Users/${match[1]}`, '~')
    }
  }
  if (path.includes('/home/')) {
    const match = path.match(/\/home\/([^/]+)/)
    if (match) {
      return path.replace(`/home/${match[1]}`, '~')
    }
  }
  // Windows paths
  if (path.match(/^[A-Z]:\\/)) {
    return path
  }
  return path
}

function getHelpText(): string {
  return `Available commands:
  clear, cls    - Clear the terminal
  help          - Show this help message
  cd <path>     - Change directory
  pwd           - Print working directory
  exit          - Close the terminal
  
Any other command will be executed in the system shell.

Keyboard shortcuts:
  Enter         - Execute command
  ↑/↓           - Navigate command history
  Ctrl+C        - Cancel current input
  Ctrl+L        - Clear screen
  Tab           - (Future) Auto-completion`
}

// Methods
function close() {
  emit('update:modelValue', false)
}

function focusInput() {
  inputRef.value?.focus()
}

function clearTerminal() {
  history.value = []
}

function openSettings() {
  showSettings.value = true
}

function scrollToBottom() {
  nextTick(() => {
    if (outputRef.value) {
      outputRef.value.scrollTop = outputRef.value.scrollHeight
    }
  })
}

async function executeCommand() {
  const cmd = currentCommand.value.trim()
  if (!cmd) return

  // Add to command history
  if (commandHistory.value[commandHistory.value.length - 1] !== cmd) {
    commandHistory.value.push(cmd)
  }
  historyIndex.value = -1

  // Handle built-in commands
  if (cmd === 'clear' || cmd === 'cls') {
    clearTerminal()
    currentCommand.value = ''
    return
  }

  if (cmd === 'help') {
    history.value.push({
      prompt: currentPrompt.value,
      command: cmd,
      output: getHelpText(),
      success: true,
      exitCode: 0,
      executionTime: 0
    })
    currentCommand.value = ''
    scrollToBottom()
    return
  }

  if (cmd === 'exit') {
    close()
    return
  }

  // Execute shell command
  isExecuting.value = true
  currentCommand.value = ''
  scrollToBottom()

  try {
    const result = await invoke<{
      success: boolean
      output?: {
        command: string
        stdout: string
        stderr: string
        exit_code: number | null
        success: boolean
        execution_time_ms: number
      }
      error?: string
    }>('unified_execute_tool', {
      toolName: 'shell',
      inputs: {
        command: cmd,
        cwd: cwd.value === '~' ? undefined : cwd.value,
        timeout_secs: 60
      },
      context: null,
      timeout: 60
    })

    if (result.success && result.output) {
      const output = result.output
      let displayOutput = ''
      
      if (output.stdout) {
        displayOutput += output.stdout
      }
      if (output.stderr) {
        displayOutput += (displayOutput ? '\n' : '') + output.stderr
      }

      // Update working directory after command execution
      await updateWorkingDirectory(cmd, output.success)

      history.value.push({
        prompt: currentPrompt.value,
        command: cmd,
        output: displayOutput.trim(),
        success: output.success,
        exitCode: output.exit_code,
        executionTime: output.execution_time_ms
      })
    } else {
      history.value.push({
        prompt: currentPrompt.value,
        command: cmd,
        output: result.error || 'Command execution failed',
        success: false,
        exitCode: null,
        executionTime: 0
      })
    }
  } catch (e: any) {
    history.value.push({
      prompt: currentPrompt.value,
      command: cmd,
      output: e.toString(),
      success: false,
      exitCode: null,
      executionTime: 0
    })
  } finally {
    isExecuting.value = false
    scrollToBottom()
    focusInput()
  }
}

function handleKeyDown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !isExecuting.value) {
    e.preventDefault()
    executeCommand()
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    navigateHistory(-1)
  } else if (e.key === 'ArrowDown') {
    e.preventDefault()
    navigateHistory(1)
  } else if (e.key === 'c' && e.ctrlKey) {
    // Ctrl+C - cancel current input
    if (!isExecuting.value) {
      currentCommand.value = ''
      history.value.push({
        prompt: currentPrompt.value,
        command: '^C',
        output: '',
        success: true,
        exitCode: 130,
        executionTime: 0
      })
      scrollToBottom()
    }
  } else if (e.key === 'l' && e.ctrlKey) {
    // Ctrl+L - clear screen
    e.preventDefault()
    clearTerminal()
  } else if (e.key === 'Tab') {
    // Tab - auto-completion (future feature)
    e.preventDefault()
    // TODO: Implement auto-completion
  } else if (e.key === 'u' && e.ctrlKey) {
    // Ctrl+U - clear line
    e.preventDefault()
    currentCommand.value = ''
  } else if (e.key === 'd' && e.ctrlKey) {
    // Ctrl+D - exit (if input is empty)
    if (!currentCommand.value && !isExecuting.value) {
      e.preventDefault()
      close()
    }
  }
}

function handlePaste(e: ClipboardEvent) {
  // Allow multi-line paste
  const text = e.clipboardData?.getData('text')
  if (text && text.includes('\n')) {
    e.preventDefault()
    const lines = text.split('\n').filter(line => line.trim())
    if (lines.length > 1) {
      // Execute multiple commands
      executeMultipleCommands(lines)
    } else if (lines.length === 1) {
      currentCommand.value += lines[0]
    }
  }
}

async function executeMultipleCommands(commands: string[]) {
  for (const cmd of commands) {
    currentCommand.value = cmd
    await executeCommand()
    // Small delay between commands
    await new Promise(resolve => setTimeout(resolve, 100))
  }
}

async function updateWorkingDirectory(cmd: string, success: boolean) {
  // Handle cd command - update cwd
  if (cmd.startsWith('cd ') && success) {
    try {
      const pwdResult = await invoke<any>('unified_execute_tool', {
        toolName: 'shell',
        inputs: { command: 'pwd', cwd: cwd.value === '~' ? undefined : cwd.value },
        context: null,
        timeout: 10
      })
      if (pwdResult.success && pwdResult.output?.stdout) {
        cwd.value = pwdResult.output.stdout.trim()
      }
    } catch {
      // Ignore errors, keep current cwd
    }
  }
  
  // Handle pwd command
  if (cmd === 'pwd' && success) {
    try {
      const pwdResult = await invoke<any>('unified_execute_tool', {
        toolName: 'shell',
        inputs: { command: 'pwd', cwd: cwd.value === '~' ? undefined : cwd.value },
        context: null,
        timeout: 10
      })
      if (pwdResult.success && pwdResult.output?.stdout) {
        cwd.value = pwdResult.output.stdout.trim()
      }
    } catch {
      // Ignore errors
    }
  }
}

// Initialize working directory on mount
async function initializeWorkingDirectory() {
  try {
    const pwdResult = await invoke<any>('unified_execute_tool', {
      toolName: 'shell',
      inputs: { command: 'pwd' },
      context: null,
      timeout: 10
    })
    if (pwdResult.success && pwdResult.output?.stdout) {
      cwd.value = pwdResult.output.stdout.trim()
    }
  } catch {
    // Keep default cwd
  }
}

function navigateHistory(direction: number) {
  if (commandHistory.value.length === 0) return

  if (historyIndex.value === -1 && direction === -1) {
    historyIndex.value = commandHistory.value.length - 1
  } else {
    historyIndex.value += direction
  }

  if (historyIndex.value < 0) {
    historyIndex.value = -1
    currentCommand.value = ''
  } else if (historyIndex.value >= commandHistory.value.length) {
    historyIndex.value = commandHistory.value.length - 1
  } else {
    currentCommand.value = commandHistory.value[historyIndex.value]
  }
}

// Watch for modal open
watch(() => props.modelValue, (val) => {
  if (val) {
    nextTick(() => {
      focusInput()
      // Initialize working directory if not set
      if (cwd.value === '~') {
        initializeWorkingDirectory()
      }
    })
  }
})
</script>

<style scoped>
/* Terminal scrollbar */
.overflow-y-auto::-webkit-scrollbar {
  width: 8px;
}

.overflow-y-auto::-webkit-scrollbar-track {
  background: #1e1e1e;
}

.overflow-y-auto::-webkit-scrollbar-thumb {
  background: #424242;
  border-radius: 4px;
}

.overflow-y-auto::-webkit-scrollbar-thumb:hover {
  background: #555;
}

/* Input caret */
input {
  caret-color: #d4d4d4;
}

input::placeholder {
  color: #6a6a6a;
}
</style>

