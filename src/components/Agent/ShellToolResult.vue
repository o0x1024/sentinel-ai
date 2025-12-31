<template>
  <div class="shell-message-block rounded-lg overflow-hidden border border-base-300 my-3 bg-[#1e1e1e]">
    <!-- Terminal Header -->
    <div class="terminal-header flex items-center gap-2 px-3 py-2 bg-[#323233]">
      <!-- Command with cwd -->
      <div class="flex-1 font-mono text-sm text-[#a0a0a0] truncate">
        <span v-if="cwd" class="text-[#6a9955]">{{ shortenPath(cwd) }}</span>
        <span class="text-[#808080] mx-1">$</span>
        <span class="text-[#d4d4d4]">{{ command }}</span>
      </div>
      
      <!-- Copy button -->
      <button 
        @click.stop="copyCommand" 
        class="btn btn-ghost btn-xs text-[#808080] hover:text-white"
        :title="$t('agent.copy')"
      >
        <i :class="['fas', copied ? 'fa-check text-success' : 'fa-copy']"></i>
      </button>
    </div>
    
    <!-- Pending Confirmation Bar -->
    <div v-if="needsConfirmation" class="confirmation-bar flex items-center justify-between px-3 py-2 bg-[#2d2d2d] border-t border-[#404040]">
      <span class="text-sm text-[#a0a0a0]">{{ $t('tools.shell.runCommand') }}</span>
      <div class="flex items-center gap-2">
        <button 
          @click="handleReject" 
          class="text-sm text-[#a0a0a0] hover:text-white px-3 py-1"
        >
          {{ $t('tools.shell.reject') }}
        </button>
        <button 
          @click="handleAlwaysAccept" 
          class="btn btn-sm btn-ghost text-[#a0a0a0] hover:text-white"
          :title="$t('tools.shell.alwaysAcceptHint')"
        >
          {{ $t('tools.shell.alwaysAccept') }}
        </button>
        <button 
          @click="handleAccept" 
          class="btn btn-sm btn-primary gap-1"
        >
          {{ $t('tools.shell.accept') }}
          <kbd class="kbd kbd-xs bg-primary-focus">⏎</kbd>
        </button>
      </div>
    </div>
    
    <!-- Terminal Body (clickable to expand/collapse) -->
    <div 
      v-if="hasOutput || isCompleted"
      ref="terminalBodyRef"
      @click="toggleExpanded"
      :class="['terminal-body bg-[#1e1e1e] p-3 font-mono text-xs overflow-y-auto cursor-pointer transition-all relative', 
               isExpanded ? 'max-h-96' : 'max-h-32']"
    >
      <!-- Output -->
      <div v-if="stdout" class="stdout text-[#d4d4d4] whitespace-pre-wrap break-all mb-1">{{ stdout }}</div>
      <div v-if="stderr" class="stderr text-[#f14c4c] whitespace-pre-wrap break-all">{{ stderr }}</div>
      
      <!-- Error message -->
      <div v-if="error && !stderr" class="error text-[#f14c4c] whitespace-pre-wrap break-all">{{ error }}</div>
      
      <!-- No output indicator -->
      <div v-if="!stdout && !stderr && !error && isCompleted" class="no-output text-[#6a9955] italic">
        (no output)
      </div>
      
      <!-- Running indicator -->
      <div v-if="isRunning && !needsConfirmation" class="running flex items-center gap-2 text-[#569cd6]">
        <i class="fas fa-spinner fa-spin"></i>
        <span>{{ $t('tools.shell.executing') }}</span>
      </div>
      
      <!-- Expand hint overlay (shown when collapsed and content overflows) -->
      <div v-if="!isExpanded && hasOverflow" class="expand-hint absolute bottom-0 left-0 right-0 h-8 bg-gradient-to-t from-[#1e1e1e] to-transparent flex items-end justify-center pb-1 pointer-events-none">
        <span class="text-[#808080] text-xs">{{ $t('tools.shell.clickToExpand') }}</span>
      </div>
    </div>
    
    <!-- Status Footer -->
    <div v-if="isCompleted" class="status-footer flex items-center justify-between px-3 py-1.5 bg-[#252526] border-t border-[#404040] text-xs">
      <div class="flex items-center gap-2">
        <span v-if="success" class="text-success flex items-center gap-1">
          <i class="fas fa-check-circle"></i>
          {{ $t('tools.shell.success') }}
        </span>
        <span v-else class="text-error flex items-center gap-1">
          <i class="fas fa-times-circle"></i>
          {{ $t('tools.shell.failed') }}
        </span>
        <span v-if="exitCode !== null" class="text-[#808080]">
          (exit {{ exitCode }})
        </span>
      </div>
      
      <div class="flex items-center gap-2">
        <span v-if="executionTime" class="text-[#808080]">
          {{ executionTime }}ms
        </span>
        <!-- Collapse/Expand button -->
        <button 
          @click.stop="toggleExpanded"
          class="btn btn-ghost btn-xs text-[#808080] hover:text-white"
          :title="isExpanded ? $t('tools.shell.collapse') : $t('tools.shell.expand')"
        >
          <i :class="['fas', isExpanded ? 'fa-chevron-up' : 'fa-chevron-down']"></i>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

const props = defineProps<{
  args?: Record<string, any>
  result?: any
  error?: string
  status?: string
  toolCallId?: string
}>()

const emit = defineEmits<{
  (e: 'accepted'): void
  (e: 'rejected'): void
}>()

const copied = ref(false)
const pendingPermissionId = ref<string | null>(null)
const pendingCommand = ref<string>('')
const isExpanded = ref(false)
const hasOverflow = ref(false)
const terminalBodyRef = ref<HTMLElement | null>(null)
let unlisten: (() => void) | null = null

// Extract command from args
const command = computed(() => {
  return props.args?.command || ''
})

// Debug: log props changes
// watch(() => props.status, (newStatus) => {
//   console.log('ShellToolResult status changed:', newStatus, 'command:', command.value)
// })

// Extract cwd from args
const cwd = computed(() => {
  return props.args?.cwd || ''
})

// Shorten path for display
function shortenPath(path: string): string {
  if (!path) return ''
  // Replace home directory with ~
  const shortened = path.replace(/^\/Users\/[^/]+/, '~')
  // If still long, show only last 2 segments
  const parts = shortened.split('/')
  if (parts.length > 3) {
    return '~/..' + '/' + parts.slice(-2).join('/')
  }
  return shortened
}

// Copy command to clipboard
async function copyCommand() {
  try {
    await navigator.clipboard.writeText(command.value)
    copied.value = true
    setTimeout(() => {
      copied.value = false
    }, 2000)
  } catch (err) {
    console.error('Failed to copy command:', err)
  }
}

// Check if needs confirmation - show when status is running and we have a pending permission request
const needsConfirmation = computed(() => {
  // Show confirmation bar if:
  // 1. We received a permission request event matching this command
  // 2. OR status is 'pending' (explicitly marked as needing confirmation)
  // 3. AND we're not already completed
  return (pendingPermissionId.value !== null || props.status === 'pending') && props.status !== 'completed' && props.status !== 'failed'
})

// Check if running
const isRunning = computed(() => {
  return props.status === 'running'
})

// Check if completed
const isCompleted = computed(() => {
  return props.status === 'completed' || props.status === 'failed'
})

// Check if has any output
const hasOutput = computed(() => {
  return stdout.value || stderr.value || props.error
})

// Parse result - handles rig-core's tool result format
const parsedResult = computed(() => {
  if (!props.result) return null
  
  let result = props.result
  
  // First, parse if it's a string
  if (typeof result === 'string') {
    try {
      result = JSON.parse(result)
    } catch {
      return { stdout: result }
    }
  }
  
  // Handle rig-core format: array of {type: "text", text: "..."} objects
  if (Array.isArray(result)) {
    // Find the text content
    const textItem = result.find((item: any) => item.type === 'text' && item.text)
    if (textItem) {
      try {
        const parsed = JSON.parse(textItem.text)
        console.log('ShellToolResult - parsed from rig-core format:', parsed)
        return parsed
      } catch {
        return { stdout: textItem.text }
      }
    }
    // Fallback: join all text items
    const allText = result
      .filter((item: any) => item.type === 'text')
      .map((item: any) => item.text)
      .join('\n')
    return { stdout: allText }
  }
  
  // Already an object with expected fields
  if (result.stdout !== undefined || result.stderr !== undefined) {
    return result
  }
  
  // Check for nested output structure
  if (result.output?.stdout !== undefined) {
    return result
  }
  
  return result
})

// Extract stdout
const stdout = computed(() => {
  const r = parsedResult.value
  if (!r) return ''
  
  // Handle nested output structure
  if (r.output?.stdout) return r.output.stdout
  if (r.stdout) return r.stdout
  
  // If result is just a string (after parsing), return it
  if (typeof r === 'string') return r
  
  return ''
})

// Extract stderr
const stderr = computed(() => {
  const r = parsedResult.value
  if (!r) return ''
  
  if (r.output?.stderr) return r.output.stderr
  if (r.stderr) return r.stderr
  
  return ''
})

// Extract exit code
const exitCode = computed((): number | null => {
  const r = parsedResult.value
  if (!r) return null
  
  if (r.output?.exit_code !== undefined) return r.output.exit_code
  if (r.exit_code !== undefined) return r.exit_code
  
  return null
})

// Check success
const success = computed(() => {
  const r = parsedResult.value
  if (!r) return props.status === 'completed'
  
  if (r.output?.success !== undefined) return r.output.success
  if (r.success !== undefined) return r.success
  
  // Fall back to exit code check
  if (exitCode.value !== null) return exitCode.value === 0
  
  return props.status === 'completed'
})

// Execution time
const executionTime = computed((): number | null => {
  const r = parsedResult.value
  if (!r) return null
  
  if (r.output?.execution_time_ms) return r.output.execution_time_ms
  if (r.execution_time_ms) return r.execution_time_ms
  
  return null
})

// Handle accept
async function handleAccept() {
  if (pendingPermissionId.value) {
    try {
      await invoke('respond_shell_permission', { 
        id: pendingPermissionId.value, 
        allowed: true 
      })
    } catch (e) {
      console.error('Failed to respond permission:', e)
    }
    pendingPermissionId.value = null
  }
  emit('accepted')
}

// Handle reject
async function handleReject() {
  if (pendingPermissionId.value) {
    try {
      await invoke('respond_shell_permission', { 
        id: pendingPermissionId.value, 
        allowed: false 
      })
    } catch (e) {
      console.error('Failed to respond permission:', e)
    }
    pendingPermissionId.value = null
  }
  emit('rejected')
}

// Handle always accept - add to allow list and accept
async function handleAlwaysAccept() {
  console.log('handleAlwaysAccept called, pendingPermissionId:', pendingPermissionId.value)
  
  // Store the permission ID before any async operation
  const permissionId = pendingPermissionId.value
  
  // Get the command to add to allow list
  const cmdToAdd = pendingCommand.value || command.value
  
  if (cmdToAdd) {
    try {
      // Get current agent config
      const agentConfig = await invoke<{shell: {default_policy: string, allowed_commands: string[], denied_commands: string[]}}>('get_agent_config')
      
      // Add command to allowed_commands if not already there
      if (!agentConfig.shell.allowed_commands.includes(cmdToAdd)) {
        agentConfig.shell.allowed_commands.push(cmdToAdd)
        
        // Save updated config
        await invoke('save_agent_config', { config: agentConfig })
        console.log('Command added to allow list:', cmdToAdd)
      }
    } catch (e) {
      console.error('Failed to add command to allow list:', e)
    }
  }
  
  // Use the stored permission ID in case it was modified during async operation
  if (permissionId) {
    try {
      console.log('Responding to permission with stored ID:', permissionId)
      await invoke('respond_shell_permission', { 
        id: permissionId, 
        allowed: true 
      })
      pendingPermissionId.value = null
    } catch (e) {
      console.error('Failed to respond permission:', e)
    }
  }
  emit('accepted')
}

// Toggle expand/collapse
function toggleExpanded() {
  isExpanded.value = !isExpanded.value
}

// Check if content overflows
function checkOverflow() {
  nextTick(() => {
    if (terminalBodyRef.value) {
      hasOverflow.value = terminalBodyRef.value.scrollHeight > terminalBodyRef.value.clientHeight
    }
  })
}

// Poll for pending permission requests
let pollInterval: ReturnType<typeof setInterval> | null = null

async function checkPendingPermissions() {
  if (!command.value || isCompleted.value) return
  
  try {
    const pending = await invoke<Array<{id: string, command: string}>>('get_pending_shell_permissions')
    
    for (const req of pending) {
      // Check if this permission request matches our command
      if (req.command === command.value || 
          command.value.includes(req.command) || 
          req.command.includes(command.value)) {
        console.log('Found pending permission request:', req.id, 'for command:', req.command)
        pendingPermissionId.value = req.id
        pendingCommand.value = req.command
        break
      }
    }
  } catch (e) {
    // Ignore errors
  }
}

// Listen for permission requests matching this command
onMounted(async () => {
  console.log('ShellToolResult mounted, listening for permission requests, command:', command.value)
  
  // Start polling for pending permissions
  await checkPendingPermissions()
  pollInterval = setInterval(checkPendingPermissions, 500)
  
  unlisten = await listen('shell-permission-request', (event: any) => {
    const payload = event.payload
    console.log('Received shell-permission-request:', payload, 'our command:', command.value)
    
    // Check if this permission request matches our command
    // Use includes for partial match since command might have different formatting
    if (payload.command === command.value || 
        command.value.includes(payload.command) || 
        payload.command.includes(command.value)) {
      console.log('Permission request matched! Setting pendingPermissionId:', payload.id)
      pendingPermissionId.value = payload.id
      pendingCommand.value = payload.command
    }
  }) as unknown as () => void
  
  // Check overflow on mount and when content changes
  checkOverflow()
  window.addEventListener('keydown', handleKeyDown)
})

onUnmounted(() => {
  if (unlisten) {
    unlisten()
  }
  if (pollInterval) {
    clearInterval(pollInterval)
  }
  window.removeEventListener('keydown', handleKeyDown)
})

// Handle keyboard shortcut
function handleKeyDown(e: KeyboardEvent) {
  if (needsConfirmation.value && e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
    e.preventDefault()
    handleAccept()
  }
}

// Watch for content changes to check overflow
watch([stdout, stderr, () => props.error], () => {
  checkOverflow()
})
</script>

<style scoped>
/* Terminal scrollbar */
.terminal-body::-webkit-scrollbar {
  width: 8px;
}

.terminal-body::-webkit-scrollbar-track {
  background: #1e1e1e;
}

.terminal-body::-webkit-scrollbar-thumb {
  background: #424242;
  border-radius: 4px;
}

.terminal-body::-webkit-scrollbar-thumb:hover {
  background: #555;
}
</style>
