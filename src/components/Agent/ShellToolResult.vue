<template>
  <div class="shell-message-block rounded-lg overflow-hidden border border-base-300 bg-[#1e1e1e]">
    <!-- Pending Confirmation Bar -->
    <div v-if="needsConfirmation" class="confirmation-bar px-3 py-2 bg-[#2d2d2d] border-b border-[#404040]">
      <div class="flex items-start justify-between gap-3">
        <div class="min-w-0">
          <div class="text-sm text-[#a0a0a0]">{{ $t('tools.shell.runCommand') }}</div>
          <div
            v-if="pendingSemanticTitle || pendingSemanticSummaryKey"
            class="mt-1 flex flex-wrap items-center gap-2 text-xs"
          >
            <span
              v-if="pendingSemanticTitle"
              :class="['rounded-full px-2 py-0.5 font-medium', pendingSemanticBadgeClass]"
            >
              {{ pendingSemanticTitle }}
            </span>
            <span v-if="pendingSemanticSummaryKey" class="text-[#c2c2c2] break-words">
              {{ $t(pendingSemanticSummaryKey) }}
            </span>
          </div>
          <div
            v-if="pendingSemanticReasonKey"
            class="mt-1 text-[11px] text-[#8f8f8f] whitespace-pre-wrap break-words"
          >
            {{ $t(pendingSemanticReasonKey) }}
          </div>
          <div
            v-if="suggestedAllowRules.length > 0"
            class="mt-2 rounded-md border border-[#404040] bg-[#242424] px-2 py-1.5"
          >
            <div class="text-[11px] text-[#a0a0a0]">
              {{ $t('tools.shell.allowRulePreviewTitle') }}
            </div>
            <div class="mt-1 space-y-1">
              <div
                v-for="item in suggestedAllowRules"
                :key="item.rule"
                class="rounded bg-[#1a1a1a] px-2 py-1"
              >
                <code class="text-[11px] text-[#d4d4d4]">
                  {{ item.rule }}
                </code>
                <div class="mt-1 text-[10px] text-[#8f8f8f]">
                  {{ $t(item.reason_key) }}
                </div>
              </div>
            </div>
          </div>
        </div>
        <div class="flex items-center gap-2">
          <button 
            @click="handleReject" 
            class="text-sm text-[#a0a0a0] hover:text-white px-3 py-1"
          >
            {{ $t('tools.shell.reject') }}
          </button>
          <button
            v-if="canAlwaysAccept"
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
    </div>
    
    <!-- Terminal Body (clickable to expand/collapse) -->
    <div 
      ref="terminalBodyRef"
      @click="toggleExpanded"
      :class="['terminal-body bg-[#1e1e1e] p-3 font-mono text-xs cursor-pointer transition-all relative', 
               isExpanded ? 'max-h-96 overflow-y-auto' : 'max-h-32 overflow-hidden']"
    >
      <!-- Command line with copy button -->
      <div class="command-line flex items-start gap-2 mb-2 group">
        <div class="flex-1 text-[#d4d4d4] flex items-start gap-2">
          <span class="text-[#808080] flex-shrink-0">$</span>
          <span class="flex-1 break-all" v-html="highlightedCommand"></span>
        </div>
        <button 
          @click.stop="copyCommand" 
          class="btn btn-ghost btn-xs text-[#808080] hover:text-white opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0"
          :title="$t('agent.copy')"
        >
          <i :class="['fas', copied ? 'fa-check text-success' : 'fa-copy']"></i>
        </button>
      </div>
      
      <!-- Output -->
      <div v-if="stdout" class="stdout text-[#d4d4d4] whitespace-pre-wrap break-all mb-1">{{ displayedStdout }}</div>
      <div v-if="stderr" class="stderr text-[#f14c4c] whitespace-pre-wrap break-all">{{ displayedStderr }}</div>
      
      <!-- Truncation warning -->
      <div v-if="isStdoutTruncated || isStderrTruncated" class="text-warning text-[10px] mt-1 italic">
        {{ $t('tools.shell.outputTruncatedHint') }}
      </div>

      <div
        v-if="shouldRecommendTerminal"
        class="mt-3 rounded-md border border-warning/40 bg-warning/10 px-3 py-2 text-[11px] text-warning-content"
      >
        <div class="flex items-start justify-between gap-3">
          <div class="min-w-0">
            <div class="font-semibold text-warning">
              {{ $t('tools.shell.longRunningCommandTitle') }}
            </div>
            <div class="mt-1 whitespace-pre-wrap break-words text-warning/90">
              {{ $t('tools.shell.longRunningCommandHint') }}
            </div>
          </div>
          <button
            @click.stop="openInteractiveTerminal"
            class="btn btn-xs btn-warning flex-shrink-0"
          >
            {{ $t('tools.shell.openTerminal') }}
          </button>
        </div>
      </div>

      <div
        v-if="isBackgroundTask"
        class="mt-3 rounded-md border border-info/40 bg-info/10 px-3 py-2 text-[11px] text-info-content"
      >
        <div class="flex items-start justify-between gap-3">
          <div class="min-w-0">
            <div class="font-semibold text-info">
              后台 Shell 任务
            </div>
            <div class="mt-1 whitespace-pre-wrap break-words text-info/90">
              {{ backgroundStatusLabel }}
            </div>
            <div v-if="backgroundNote" class="mt-1 whitespace-pre-wrap break-words text-info/80">
              {{ backgroundNote }}
            </div>
            <div v-if="backgroundTaskId" class="mt-1 text-[10px] text-info/70">
              Task: {{ backgroundTaskId }}
            </div>
          </div>
          <div class="flex flex-shrink-0 items-center gap-2">
            <button
              @click.stop="openBackgroundTerminal"
              class="btn btn-xs btn-info"
            >
              打开终端
            </button>
            <button
              v-if="canStopBackgroundTask"
              @click.stop="handleStopBackgroundTask"
              class="btn btn-xs btn-ghost text-info"
              :disabled="isStoppingBackgroundTask"
            >
              停止
            </button>
          </div>
        </div>
      </div>
      
      <!-- Error message -->
      <div v-if="error && !stderr" class="error text-[#f14c4c] whitespace-pre-wrap break-all">{{ error }}</div>
      
      <!-- No output indicator -->
      <div v-if="!stdout && !stderr && !error && isCompleted && !isBackgroundTask" class="no-output text-[#6a9955] italic">
        {{ $t('tools.shell.noOutput') }}
      </div>
      
      <!-- Running indicator -->
      <div v-if="isRunning && !needsConfirmation" class="running flex items-center justify-between gap-2 text-[#569cd6]">
        <div class="flex items-center gap-2">
          <i class="fas fa-spinner fa-spin"></i>
          <span>{{ $t('tools.shell.executing') }}</span>
        </div>
        <button
          v-if="canCancel"
          @click.stop="handleCancel"
          class="btn btn-ghost btn-xs text-[#666] hover:text-[#888]"
          :disabled="isCancelling"
        >
          <span>{{ $t('common.cancel') }}</span>
        </button>
      </div>
      
      <!-- Expand hint overlay (shown when collapsed and content overflows) -->
      <div v-if="!isExpanded && hasOverflow" class="expand-hint absolute bottom-0 left-0 right-0 h-8 bg-gradient-to-t from-[#1e1e1e] to-transparent flex items-end justify-center pb-1 pointer-events-none">
        <span class="text-[#808080] text-xs">{{ $t('tools.shell.clickToExpand') }}</span>
      </div>
    </div>
    
    <!-- Status Footer -->
    <div v-if="isCompleted" class="status-footer flex items-center justify-between px-3 py-1.5 bg-[#252526] border-t border-[#404040] text-xs">
      <div class="flex items-center gap-2">
        <span v-if="exitCode !== null" class="text-[#808080]">
          (exit {{ exitCode }})
        </span>
      </div>
      
      <div class="flex items-center gap-2">
        <span v-if="executionTime" class="text-[#808080]">
          {{ executionTime }}ms
        </span>
        <button
          @click.stop="copyAllContent"
          class="btn btn-ghost btn-xs text-[#808080] hover:text-white"
          :title="$t('tools.shell.copyAllHint')"
        >
          <i :class="['fas', copiedAll ? 'fa-check text-success' : 'fa-copy']"></i>
          <span class="ml-1">{{ $t('tools.shell.copyAll') }}</span>
        </button>
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
import { useI18n } from 'vue-i18n'
import { highlightShellCommand } from '@/utils/shellHighlight'
import { useTerminal } from '@/composables/useTerminal'
import { useTodos } from '@/composables/useTodos'

const props = defineProps<{
  args?: Record<string, any>
  result?: any
  error?: string
  status?: string
  toolCallId?: string
  executionId?: string
}>()

const emit = defineEmits<{
  (e: 'accepted'): void
  (e: 'rejected'): void
}>()

type PendingPermissionInfo = {
  id: string
  command: string
  semantic_kind?: 'read_only' | 'mutating' | 'dangerous' | ''
  semantic_code?: string
  semantic_summary_key?: string
  semantic_reason_key?: string | null
  suggested_allow_rules?: Array<{
    rule: string
    reason_key: string
  }>
}

const copied = ref(false)
const copiedAll = ref(false)
const isCancelling = ref(false)
const isStoppingBackgroundTask = ref(false)
const pendingPermissionId = ref<string | null>(null)
const pendingCommand = ref<string>('')
const pendingPermissionInfo = ref<PendingPermissionInfo | null>(null)
const isExpanded = ref(false)
const hasOverflow = ref(false)
const terminalBodyRef = ref<HTMLElement | null>(null)
const backgroundRuntimeState = ref<Record<string, any> | null>(null)
let unlisten: (() => void) | null = null
let unlistenBackgroundTask: (() => void) | null = null
const terminal = useTerminal()
const todos = useTodos()
const { t } = useI18n()

// Extract command from args
const command = computed(() => {
  return props.args?.command || ''
})

// Highlighted command
const highlightedCommand = computed(() => {
  return highlightShellCommand(command.value)
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

function buildCopyAllContent(): string {
  const parts: string[] = []
  if (command.value) {
    parts.push(`$ ${command.value}`)
  }
  if (stdout.value) {
    parts.push(stdout.value)
  }
  if (stderr.value) {
    parts.push(stderr.value)
  }
  if (props.error) {
    parts.push(props.error)
  }
  return parts.join('\n')
}

async function copyAllContent() {
  try {
    const content = buildCopyAllContent()
    await navigator.clipboard.writeText(content)
    copiedAll.value = true
    setTimeout(() => {
      copiedAll.value = false
    }, 2000)
  } catch (err) {
    console.error('Failed to copy shell output:', err)
  }
}

async function handleCancel() {
  if (!props.executionId || isCancelling.value) return
  isCancelling.value = true
  try {
    await invoke('cancel_shell_execution', {
      executionId: props.executionId,
    })
  } catch (e) {
    console.error('Failed to cancel shell execution:', e)
  } finally {
    isCancelling.value = false
  }
}

function openInteractiveTerminal() {
  todos.close()
  terminal.openTerminal()
}

function openBackgroundTerminal() {
  todos.close()
  if (backgroundSessionId.value) {
    terminal.setSessionId(backgroundSessionId.value)
  }
  terminal.openTerminal(backgroundSessionId.value || undefined)
}

// Check if needs confirmation - show when status is running and we have a pending permission request
const needsConfirmation = computed(() => {
  // Show confirmation bar if:
  // 1. We received a permission request event matching this command
  // 2. OR status is 'pending' (explicitly marked as needing confirmation)
  // 3. AND we're not already completed
  return (pendingPermissionId.value !== null || props.status === 'pending') && props.status !== 'completed' && props.status !== 'failed'
})

const pendingSemanticKind = computed(() => {
  return pendingPermissionInfo.value?.semantic_kind || ''
})

const pendingSemanticSummaryKey = computed(() => {
  return pendingPermissionInfo.value?.semantic_summary_key || ''
})

const pendingSemanticReasonKey = computed(() => {
  return pendingPermissionInfo.value?.semantic_reason_key || ''
})

const suggestedAllowRules = computed(() => {
  return pendingPermissionInfo.value?.suggested_allow_rules || []
})

const pendingSemanticTitle = computed(() => {
  switch (pendingSemanticKind.value) {
    case 'read_only':
      return t('tools.shell.semanticLabels.readOnly')
    case 'dangerous':
      return t('tools.shell.semanticLabels.dangerous')
    case 'mutating':
      return t('tools.shell.semanticLabels.mutating')
    default:
      return ''
  }
})

const pendingSemanticBadgeClass = computed(() => {
  switch (pendingSemanticKind.value) {
    case 'read_only':
      return 'bg-info/20 text-info border border-info/30'
    case 'dangerous':
      return 'bg-error/20 text-error border border-error/30'
    case 'mutating':
      return 'bg-warning/20 text-warning border border-warning/30'
    default:
      return 'bg-base-300/20 text-base-content/80 border border-base-300/30'
  }
})

const canAlwaysAccept = computed(() => {
  return suggestedAllowRules.value.length > 0
})

// Check if running
const isRunning = computed(() => {
  return props.status === 'running'
})

const canCancel = computed(() => {
  return isRunning.value && !needsConfirmation.value && !!props.executionId
})

// Check if completed
const isCompleted = computed(() => {
  return props.status === 'completed' || props.status === 'failed'
})

const backgroundTaskId = computed(() => {
  const runtimeTaskId = backgroundRuntimeState.value?.id
  if (runtimeTaskId) return String(runtimeTaskId)
  const r = parsedResult.value
  if (!r) return ''
  return String(r.output?.background_task_id || r.background_task_id || '')
})

const backgroundSessionId = computed(() => {
  const runtimeSessionId = backgroundRuntimeState.value?.session_id
  if (runtimeSessionId) return String(runtimeSessionId)
  const r = parsedResult.value
  if (!r) return ''
  return String(r.output?.background_session_id || r.background_session_id || '')
})

const backgroundStatus = computed(() => {
  const runtimeStatus = backgroundRuntimeState.value?.status
  if (runtimeStatus) return String(runtimeStatus)
  const r = parsedResult.value
  if (!r) return ''
  return String(r.output?.background_status || r.background_status || '')
})

const backgroundNote = computed(() => {
  const r = parsedResult.value
  if (!r) return ''
  return String(r.output?.note || r.note || '')
})

const isBackgroundTask = computed(() => {
  const r = parsedResult.value
  if (!r) return false
  return Boolean(r.output?.backgrounded || r.backgrounded || backgroundTaskId.value)
})

const canStopBackgroundTask = computed(() => {
  return isBackgroundTask.value && backgroundTaskId.value && backgroundStatus.value === 'running'
})

const backgroundStatusLabel = computed(() => {
  if (!isBackgroundTask.value) return ''
  const exit = backgroundRuntimeState.value?.exit_code
  switch (backgroundStatus.value) {
    case 'completed':
      return exit === 0 || exit === undefined
        ? '后台任务已完成。'
        : `后台任务已完成，退出码 ${exit}。`
    case 'failed':
      return exit === undefined
        ? '后台任务执行失败。'
        : `后台任务执行失败，退出码 ${exit}。`
    case 'cancelled':
      return '后台任务已停止。'
    default:
      return '命令正在独立终端会话中运行，不会阻塞当前对话。'
  }
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

// Max length for output display to prevent UI lag
const MAX_OUTPUT_LENGTH = 10000

const isStdoutTruncated = computed(() => stdout.value.length > MAX_OUTPUT_LENGTH)
const displayedStdout = computed(() => {
  if (isStdoutTruncated.value) {
    return stdout.value.substring(0, MAX_OUTPUT_LENGTH) + '\n... [truncated]'
  }
  return stdout.value
})

const isStderrTruncated = computed(() => stderr.value.length > MAX_OUTPUT_LENGTH)
const displayedStderr = computed(() => {
  if (isStderrTruncated.value) {
    return stderr.value.substring(0, MAX_OUTPUT_LENGTH) + '\n... [truncated]'
  }
  return stderr.value
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

const shellGuidanceError = computed(() => {
  const rawError = String(props.error || '').trim()
  if (
    !rawError.includes('Detected a background shell command') &&
    !rawError.includes('Detected a long-running foreground shell command')
  ) {
    return ''
  }
  return rawError
})

const shouldRecommendTerminal = computed(() => {
  return props.status === 'failed' && shellGuidanceError.value.length > 0
})

// Execution time
const executionTime = computed((): number | null => {
  const r = parsedResult.value
  if (!r) return null
  
  if (r.output?.execution_time_ms) return r.output.execution_time_ms
  if (r.execution_time_ms) return r.execution_time_ms
  
  return null
})

async function handleStopBackgroundTask() {
  if (!backgroundTaskId.value || isStoppingBackgroundTask.value) return
  isStoppingBackgroundTask.value = true
  try {
    await invoke('stop_background_shell_task', {
      taskId: backgroundTaskId.value,
    })
  } catch (error) {
    console.error('Failed to stop background shell task:', error)
  } finally {
    isStoppingBackgroundTask.value = false
  }
}

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
    pendingCommand.value = ''
    pendingPermissionInfo.value = null
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
    pendingCommand.value = ''
    pendingPermissionInfo.value = null
  }
  emit('rejected')
}

// Handle always accept - add to allow list and accept
async function handleAlwaysAccept() {
  if (!canAlwaysAccept.value) {
    return
  }

  console.log('handleAlwaysAccept called, pendingPermissionId:', pendingPermissionId.value)
  
  // Store the permission ID before any async operation
  const permissionId = pendingPermissionId.value
  
  if (permissionId) {
    try {
      const result = await invoke<{ added_rules: string[], all_rules: string[] }>('allow_shell_permission_forever', {
        id: permissionId,
      })
      console.log('Shell allow rules persisted:', result)
      pendingPermissionId.value = null
      pendingCommand.value = ''
      pendingPermissionInfo.value = null
    } catch (e) {
      console.error('Failed to persist shell allow rules:', e)
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
    const pending = await invoke<PendingPermissionInfo[]>('get_pending_shell_permissions')
    let matched = false
    
    for (const req of pending) {
      // Check if this permission request matches our command
      if (req.command === command.value || 
          command.value.includes(req.command) || 
          req.command.includes(command.value)) {
        console.log('Found pending permission request:', req.id, 'for command:', req.command)
        pendingPermissionId.value = req.id
        pendingCommand.value = req.command
        pendingPermissionInfo.value = req
        matched = true
        break
      }
    }

    if (!matched && pendingPermissionId.value !== null) {
      pendingPermissionId.value = null
      pendingCommand.value = ''
      pendingPermissionInfo.value = null
    }
  } catch (e) {
    // Ignore errors
  }
}

async function loadBackgroundTaskState() {
  if (!backgroundTaskId.value || !props.executionId) return
  try {
    const tasks = await invoke<Array<Record<string, any>>>('get_background_shell_tasks', {
      executionId: props.executionId,
    })
    const matched = tasks.find((task) => String(task.id || '') === backgroundTaskId.value)
    if (matched) {
      backgroundRuntimeState.value = matched
    }
  } catch (error) {
    console.error('Failed to load background shell task state:', error)
  }
}

// Listen for permission requests matching this command
onMounted(async () => {
  console.log('ShellToolResult mounted, listening for permission requests, command:', command.value)
  
  // Start polling for pending permissions
  await checkPendingPermissions()
  pollInterval = setInterval(checkPendingPermissions, 500)
  
  unlisten = await listen('shell-permission-request', (event: any) => {
    const payload = event.payload as PendingPermissionInfo
    console.log('Received shell-permission-request:', payload, 'our command:', command.value)
    
    // Check if this permission request matches our command
    // Use includes for partial match since command might have different formatting
    if (payload.command === command.value || 
        command.value.includes(payload.command) || 
        payload.command.includes(command.value)) {
      console.log('Permission request matched! Setting pendingPermissionId:', payload.id)
      pendingPermissionId.value = payload.id
      pendingCommand.value = payload.command
      pendingPermissionInfo.value = payload
    }
  }) as unknown as () => void

  unlistenBackgroundTask = await listen('shell-background-task-update', (event: any) => {
    const payload = event.payload
    if (!payload || String(payload.id || '') !== backgroundTaskId.value) {
      return
    }
    backgroundRuntimeState.value = payload
  }) as unknown as () => void

  await loadBackgroundTaskState()
  
  // Check overflow on mount and when content changes
  checkOverflow()
  window.addEventListener('keydown', handleKeyDown)
})

onUnmounted(() => {
  if (unlisten) {
    unlisten()
  }
  if (unlistenBackgroundTask) {
    unlistenBackgroundTask()
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

watch(backgroundTaskId, () => {
  void loadBackgroundTaskState()
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

/* Shell syntax highlighting (CodeMirror oneDark theme colors) */
:deep(.cm-keyword) { color: #c678dd; } /* Commands and keywords */
:deep(.cm-operator) { color: #56b6c2; } /* Operators like |, >, <, & */
:deep(.cm-string) { color: #98c379; } /* Single-quoted strings */
:deep(.cm-string-2) { color: #98c379; } /* Double-quoted strings */
:deep(.cm-comment) { color: #5c6370; font-style: italic; } /* Comments */
:deep(.cm-variable) { color: #e06c75; } /* Variables like $VAR */
:deep(.cm-variable-2) { color: #e5c07b; } /* Special variables */
:deep(.cm-variable-3) { color: #d19a66; } /* Other variables */
:deep(.cm-def) { color: #61afef; } /* Function definitions */
:deep(.cm-atom) { color: #d19a66; } /* Atoms (true, false, null) */
:deep(.cm-number) { color: #d19a66; } /* Numbers */
:deep(.cm-property) { color: #61afef; } /* Properties */
:deep(.cm-qualifier) { color: #e06c75; } /* Qualifiers */
:deep(.cm-type) { color: #e5c07b; } /* Types */
:deep(.cm-builtin) { color: #e5c07b; } /* Built-in commands */
:deep(.cm-bracket) { color: #abb2bf; } /* Brackets */
:deep(.cm-tag) { color: #e06c75; } /* Tags */
:deep(.cm-attribute) { color: #d19a66; } /* Attributes */
:deep(.cm-meta) { color: #61afef; } /* Meta information */
:deep(.cm-link) { color: #61afef; text-decoration: underline; } /* Links */
</style>
