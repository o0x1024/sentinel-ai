<template>
  <div class="web-explorer-panel border border-base-300 rounded-lg bg-base-200 flex flex-col h-full min-h-0 overflow-hidden" v-if="isActive">
    <!-- Header -->
    <div class="web-explorer-header p-3 border-b border-base-300 flex justify-between items-center bg-base-100">
      <div class="flex items-center gap-2">
        <span class="text-lg">🌐</span>
        <span class="font-bold text-sm">Web Explorer</span>
      </div>
      <div class="flex gap-2 items-center">
         <div class="badge badge-sm badge-info gap-1" v-if="discoveredApis.length > 0">
            API: {{ discoveredApis.length }}
         </div>
         <button @click="$emit('close')" class="btn btn-ghost btn-xs btn-circle" title="Close Panel">
            ✕
         </button>
      </div>
    </div>

    <!-- Login Takeover Form - only show if not yet submitted in this session -->
    <div v-if="showTakeoverForm && !credentialsSubmitted" class="p-3 bg-warning/10 border-b border-warning/30">
      <div class="flex items-center justify-between text-sm text-warning font-medium mb-2">
        <div class="flex items-center gap-2">
          <i class="fas fa-key"></i>
          <span>{{ takeoverMessage || t('agent.loginPageDetected') }}</span>
        </div>
        <!-- Timeout countdown -->
        <div v-if="loginTimeoutRemaining !== null && loginTimeoutRemaining > 0" 
             class="text-xs bg-warning/20 px-2 py-1 rounded"
             :class="{ 'animate-pulse': loginTimeoutRemaining <= 30 }">
          <i class="fas fa-clock mr-1"></i>
          {{ formatTimeRemaining(loginTimeoutRemaining) }}
        </div>
      </div>
      
      <div class="space-y-2">
          <!-- Dynamic Fields -->
          <template v-if="takeoverFields && takeoverFields.length > 0">
              <div v-for="field in takeoverFields" :key="field.id">
                  <input
                    v-model="credentials[field.id]"
                    :type="field.field_type"
                    :placeholder="field.placeholder || field.label"
                    class="input input-sm input-bordered w-full text-xs"
                    @keyup.enter="submitCredentials"
                  />
              </div>
          </template>
          
          <!-- Fallback Fields (if no dynamic fields provided) -->
          <template v-else>
              <input
                v-model="credentials.username"
                type="text"
                :placeholder="t('agent.usernameAccount')"
                class="input input-sm input-bordered w-full text-xs"
                @keyup.enter="submitCredentials"
              />
              <input
                v-model="credentials.password"
                type="password"
                :placeholder="t('agent.password')"
                class="input input-sm input-bordered w-full text-xs"
                @keyup.enter="submitCredentials"
              />
              <input
                  v-model="credentials.verificationCode"
                  type="text"
                  :placeholder="t('agent.verificationCodeOptional')"
                  class="input input-sm input-bordered w-full text-xs"
                  @keyup.enter="submitCredentials"
              />
          </template>

          <!-- Actions -->
          <div class="flex gap-2 mt-2">
            <button
              class="btn btn-sm btn-warning flex-1"
              :disabled="!canSubmit || isSubmittingCredentials || isSkippingLogin"
              @click="submitCredentials"
            >
              <span v-if="isSubmittingCredentials" class="loading loading-spinner loading-xs"></span>
              <span v-else>{{ t('agent.continueExploration') }}</span>
            </button>

            <button
              class="btn btn-sm btn-ghost flex-1"
              :disabled="isSubmittingCredentials || isSkippingLogin || isManualLoginCompleting"
              @click="skipLogin"
            >
              <span v-if="isSkippingLogin" class="loading loading-spinner loading-xs"></span>
              <span v-else>{{ t('agent.skipLogin') }}</span>
            </button>

            <button
               class="btn btn-sm btn-ghost flex-1 text-success"
               :disabled="isSubmittingCredentials || isSkippingLogin || isManualLoginCompleting"
               @click="manualLoginComplete"
               title="Click if you have manually logged in via the browser window"
            >
               <span v-if="isManualLoginCompleting" class="loading loading-spinner loading-xs"></span>
               <span v-else>{{ t('agent.alreadyLoggedIn') || 'Logged In' }}</span>
            </button>
          </div>
        </div>
    </div>

    <!-- Multi-Agent Mode Section (Collapsible) -->
    <div v-if="isMultiAgentMode && multiAgent" class="border-b border-base-300">
      <!-- Header Row -->
      <div class="p-2 bg-gradient-to-r from-primary/10 to-secondary/10 flex items-center justify-between cursor-pointer" @click="toggleMultiAgentExpanded">
        <div class="flex items-center gap-2">
          <span class="text-sm">🤖</span>
          <span class="font-bold text-xs text-primary">{{ t('agent.multiAgentMode') }}</span>
          <span class="badge badge-xs badge-primary">{{ multiAgent.mode?.mode }}</span>
        </div>
        <div class="flex items-center gap-2">
          <span class="text-[10px] opacity-70">
            {{ multiAgent.mode?.completed_workers || 0 }}/{{ multiAgent.mode?.total_workers || 0 }} {{ t('agent.workersCompleted') }}
          </span>
          <span class="text-xs opacity-50">{{ multiAgentExpanded ? '▲' : '▼' }}</span>
        </div>
      </div>
      
      <!-- Expandable Workers Grid -->
      <div v-if="multiAgentExpanded" class="p-2 bg-base-100/50">
        <div class="grid gap-1.5" :class="{ 'grid-cols-3': (multiAgent.tasks?.length || 0) <= 6, 'grid-cols-4': (multiAgent.tasks?.length || 0) > 6 }">
          <div
            v-for="task in multiAgent.tasks"
            :key="task.task_id"
            class="bg-base-100 rounded p-1.5 border border-base-300 text-[10px]"
            :class="{
              'border-primary/50': getWorkerStatus(task.task_id) === 'running',
              'border-success/50': getWorkerStatus(task.task_id) === 'completed',
              'opacity-60': getWorkerStatus(task.task_id) === 'pending'
            }"
          >
            <div class="flex items-center gap-1 mb-0.5">
              <span v-if="getWorkerStatus(task.task_id) === 'completed'" class="text-success text-[8px]">✓</span>
              <span v-else-if="getWorkerStatus(task.task_id) === 'running'" class="loading loading-spinner loading-xs text-primary" style="width: 8px; height: 8px;"></span>
              <span v-else class="opacity-30 text-[8px]">○</span>
              <span class="font-medium truncate flex-1">{{ task.scope_name }}</span>
            </div>
            <div class="flex items-center gap-1 text-[9px] opacity-70">
              <span>{{ getWorkerProgress(task.task_id)?.pages_visited || 0 }}p</span>
              <span>•</span>
              <span>{{ getWorkerProgress(task.task_id)?.apis_discovered || 0 }}a</span>
            </div>
          </div>
        </div>
        
        <!-- Global Stats -->
        <div v-if="multiAgent.globalStats" class="flex gap-4 mt-1.5 text-[9px] opacity-70">
          <span>{{ t('agent.totalUrls') }}: {{ multiAgent.globalStats.total_urls_visited }}</span>
          <span>{{ t('agent.totalApis') }}: {{ multiAgent.globalStats.total_apis_discovered }}</span>
        </div>
      </div>
    </div>

    <!-- Plan & Progress Section (Compact) -->
    <div class="p-2 bg-base-100/50 text-xs border-b border-base-300 flex flex-col gap-2" v-if="currentPlan || currentProgress">
      <!-- Current Plan (Collapsed by default, just show phase name) -->
      <div v-if="currentPlan" class="plan-section">
        <div class="flex items-center gap-2 cursor-pointer" @click="togglePlanExpanded">
          <span class="text-primary font-bold text-sm">📋</span>
          <span class="font-bold text-primary text-xs">{{ currentPlan.phase_name || currentPlan.phase }}</span>
          <span class="text-[10px] opacity-50 ml-auto">{{ planExpanded ? '▲' : '▼' }}</span>
        </div>
        <div v-if="planExpanded" class="pl-4 space-y-1 mt-1">
          <ul class="space-y-1">
            <li v-for="(step, idx) in currentPlan.steps" :key="idx" class="flex items-center gap-1.5 text-[10px]">
              <span v-if="parseStepStatus(step) === 'done'" class="text-success">✓</span>
              <span v-else-if="parseStepStatus(step) === 'skip'" class="text-base-content/40">✗</span>
              <span v-else-if="parseStepStatus(step) === 'loading'" class="loading loading-spinner loading-xs text-primary"></span>
              <span v-else class="text-base-content/30">○</span>
              <span 
                :class="{
                  'line-through text-base-content/40': parseStepStatus(step) === 'skip',
                  'text-success': parseStepStatus(step) === 'done',
                  'text-primary font-medium': parseStepStatus(step) === 'loading',
                  'text-base-content/60': parseStepStatus(step) === 'pending'
                }"
              >{{ parseStepText(step) }}</span>
            </li>
          </ul>
        </div>
      </div>

      <!-- Current Progress (Compact horizontal bar) -->
      <div v-if="currentProgress" class="progress-section">
        <div class="flex items-center gap-3 text-[10px]">
          <span class="font-bold text-secondary">📊</span>
          <span class="badge badge-xs badge-secondary">{{ currentProgress.iteration }}/{{ currentProgress.max_iterations }}</span>
          <div class="flex gap-3 ml-auto">
            <span class="text-primary font-bold">{{ currentProgress.pages_visited }} <span class="font-normal opacity-60">{{ t('agent.webPages') }}</span></span>
            <span class="text-accent font-bold">{{ currentProgress.apis_discovered }} <span class="font-normal opacity-60">APIs</span></span>
            <span class="text-info font-bold">{{ currentProgress.elements_interacted }} <span class="font-normal opacity-60">{{ t('agent.webElements') }}</span></span>
          </div>
        </div>
      </div>
    </div>

    <!-- Coverage Stats -->
    <div class="p-3 bg-base-100/50 text-xs border-b border-base-300 flex flex-col gap-2" v-if="coverage">
      <div class="flex justify-between items-center">
         <span class="opacity-70">Target:</span>
         <span class="font-mono truncate max-w-[200px]" :title="currentUrl">{{ currentUrl }}</span>
      </div>
      
      <!-- Route Coverage -->
      <div class="flex items-center gap-2" title="Route Coverage">
        <span class="w-16 opacity-70">Routes</span>
        <progress class="progress progress-primary w-full" :value="coverage.route_coverage" max="100"></progress>
        <span class="w-8 text-right">{{ coverage.route_coverage.toFixed(0) }}%</span>
      </div>

      <!-- Element Coverage -->
      <div class="flex items-center gap-2" title="Element Coverage">
        <span class="w-16 opacity-70">Elements</span>
        <progress class="progress progress-secondary w-full" :value="coverage.element_coverage" max="100"></progress>
        <span class="w-8 text-right">{{ coverage.element_coverage.toFixed(0) }}%</span>
      </div>
    </div>

    <!-- Timeline Steps -->
    <div class="steps-container flex-1 overflow-y-auto p-3 flex flex-col gap-3 scroll-smooth" ref="stepsContainer">
       <!-- Activity feed (meta-only / multi-agent runs may not emit web_step timeline) -->
       <div v-if="activity && activity.length" class="mb-3">
         <div class="flex items-center gap-2 text-[10px] opacity-70 mb-2">
           <span class="font-bold">{{ t('agent.webActivity') }}</span>
           <span class="ml-auto">{{ activity.length }}</span>
         </div>
         <div class="bg-base-100 rounded border border-base-300 overflow-hidden">
           <div
             v-for="(evt, i) in recentActivity"
             :key="i"
             class="px-2 py-1 text-[10px] border-b border-base-200 last:border-b-0"
           >
             {{ formatActivity(evt) }}
           </div>
         </div>
       </div>

       <div v-if="steps.length === 0 && (!activity || activity.length === 0)" class="text-center text-xs opacity-50 py-4">
         {{ t('agent.webWaitingForEvents') }}
       </div>

       <div v-for="(step, idx) in steps" :key="idx" class="web-explorer-step flex gap-3 text-xs">
          <!-- Icon Column -->
          <div class="flex flex-col items-center">
             <div class="w-6 h-6 rounded-full flex items-center justify-center text-xs border bg-base-100 z-10"
                :class="{
                    'border-primary text-primary': step.phase === 'action',
                    'border-secondary text-secondary': step.phase === 'analyze',
                    'border-accent text-accent': step.phase === 'screenshot',
                    'border-error text-error': step.error
                }">
                <span v-if="step.error">❌</span>
                <span v-else-if="step.phase === 'screenshot'">📸</span>
                <span v-else-if="step.phase === 'analyze'">🧠</span>
                <span v-else-if="step.phase === 'action'">⚡</span>
             </div>
             <div class="w-0.5 h-full bg-base-300 -mt-1" v-if="idx < steps.length - 1"></div>
          </div>

          <!-- Content Column -->
          <div class="flex-1 pb-4 min-w-0">
             <div class="flex justify-between mb-1 opacity-60 text-[10px]">
                <span>Iteration {{ step.iteration }}</span>
                <span class="uppercase">{{ step.phase }}</span>
             </div>

             <!-- Screenshot -->
             <div v-if="step.screenshot" class="mb-2">
                <img :src="'data:image/png;base64,' + step.screenshot" class="rounded border border-base-300 shadow-sm w-full max-h-[150px] object-cover hover:object-contain transition-all bg-base-300" />
                <div class="text-[10px] mt-1 opacity-70 truncate">{{ step.title }}</div>
             </div>

             <!-- Thought Process (LLM Reasoning) -->
             <div v-if="step.thought" class="bg-accent/10 p-2 rounded border border-accent/30 mb-2">
                <div class="text-[10px] text-accent font-semibold mb-1 flex items-center gap-1">
                   <i class="fas fa-brain"></i>
                   <span>{{ t('agent.llmThinking') || 'LLM Thinking' }}</span>
                </div>
                <p class="text-[11px] italic leading-relaxed">{{ step.thought }}</p>
             </div>

             <!-- Analysis -->
             <div v-if="step.analysis" class="bg-base-100 p-2 rounded border border-base-300">
                <p class="mb-1">{{ step.analysis.page_analysis }}</p>
                <div v-if="step.analysis.estimated_apis && step.analysis.estimated_apis.length" class="mt-1 pt-1 border-t border-base-200">
                    <div class="text-[10px] opacity-70">Estimated APIs:</div>
                    <div class="flex flex-wrap gap-1 mt-0.5">
                        <span v-for="api in step.analysis.estimated_apis" :key="api" class="badge badge-xs badge-ghost max-w-full truncate">
                            {{ api }}
                        </span>
                    </div>
                </div>
             </div>

             <!-- Action -->
             <div v-if="step.action" class="bg-base-100 p-2 rounded border border-base-300" :class="{'border-error/50 bg-error/5': !step.action.success}">
                 <div class="font-bold mb-1">{{ step.action.action_type }}</div>
                 
                 <!-- Element-based actions (click, input, select, etc.) -->
                 <div v-if="step.action.element_index !== undefined" class="mb-0.5 opacity-80">
                    Target: Index [{{ step.action.element_index }}]
                 </div>
                 <div v-if="step.action.value" class="mb-0.5 break-all">
                    Value: <span class="font-mono bg-base-200 px-1 rounded">{{ step.action.value }}</span>
                 </div>
                 
                 <!-- Params-based actions (navigate, wait, etc.) -->
                 <div v-if="step.action.params" class="mb-0.5 space-y-0.5">
                    <div v-for="(value, key) in step.action.params" :key="key" class="break-all">
                       <span class="opacity-70">{{ key }}:</span>
                       <span class="font-mono bg-base-200 px-1 rounded ml-1">{{ value }}</span>
                    </div>
                 </div>
             </div>

             <!-- Error -->
             <div v-if="step.error" class="bg-error/10 text-error p-2 rounded border border-error/20">
                {{ step.error }}
             </div>
          </div>
       </div>
    </div>

    <!-- User Message Input (sticky bottom) -->
    <div class="mt-auto p-3 border-t border-base-300 bg-base-100/70">
      <div class="relative">
        <textarea
          v-model="userMessage"
          class="textarea textarea-bordered w-full text-xs leading-5 min-h-[2.75rem] max-h-28 resize-none pr-20"
          :placeholder="t('agent.webMessagePlaceholder')"
          @keydown="onUserMessageKeydown"
        />

        <!-- Inline buttons inside textarea -->
        <div class="absolute right-2 bottom-2 flex items-center gap-1">

          <button
            class="btn btn-xs btn-primary"
            :title="t('agent.send')"
            :disabled="!canSendMessage || isSendingMessage"
            @click="sendUserMessage"
          >
            <span v-if="isSendingMessage" class="loading loading-spinner loading-xs"></span>
            <i v-else class="fas fa-paper-plane"></i>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick, computed, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import type { WebStep, WebCoverage, WebPlan, WebProgress, MultiAgentState, WorkerProgress, WebActivityEvent } from '@/composables/useWebExplorerEvents'

const { t } = useI18n()

const props = defineProps<{
  steps: WebStep[]
  coverage: WebCoverage | null
  discoveredApis: { method: string; url: string }[]
  isActive: boolean
  currentUrl: string
  // Plan & Progress props
  currentPlan?: WebPlan | null
  currentProgress?: WebProgress | null
  // Takeover props
  showTakeoverForm?: boolean
  takeoverMessage?: string
  takeoverFields?: any[]
  loginTimeoutSeconds?: number | null  // Timeout in seconds for manual login
  executionId?: string | null
  // Multi-Agent props
  multiAgent?: MultiAgentState | null
  isMultiAgentMode?: boolean
  // Activity feed props
  activity?: WebActivityEvent[]
}>()

defineEmits<{
  (e: 'close'): void
}>()

// Parse step status from "status:text" format
const parseStepStatus = (step: string): string => {
  if (typeof step !== 'string') return 'pending'
  const parts = step.split(':')
  if (parts.length >= 2) {
    const status = parts[0].trim()
    if (['done', 'skip', 'loading', 'pending'].includes(status)) {
      return status
    }
  }
  return 'pending'
}

// Parse step text from "status:text" format
const parseStepText = (step: string): string => {
  if (typeof step !== 'string') return step
  const parts = step.split(':')
  if (parts.length >= 2) {
    return parts.slice(1).join(':').trim()
  }
  return step
}

const stepsContainer = ref<HTMLElement | null>(null)

// Collapse state for UI sections
const multiAgentExpanded = ref(false)
const planExpanded = ref(false)

const toggleMultiAgentExpanded = () => {
  multiAgentExpanded.value = !multiAgentExpanded.value
}

const togglePlanExpanded = () => {
  planExpanded.value = !planExpanded.value
}

// Takeover form state
const isSubmittingCredentials = ref(false)
const isSkippingLogin = ref(false)
const credentials = ref<Record<string, string>>({})
const credentialsSubmitted = ref(false) // Track if credentials were submitted in this session

// Login timeout state
const loginTimeoutRemaining = ref<number | null>(null)
let loginTimeoutInterval: ReturnType<typeof setInterval> | null = null

// Format remaining time as "Xm Xs" or "Xs"
const formatTimeRemaining = (seconds: number): string => {
  if (seconds >= 60) {
    const mins = Math.floor(seconds / 60)
    const secs = seconds % 60
    return `${mins}m ${secs}s`
  }
  return `${seconds}s`
}

// User message state
const userMessage = ref('')
const isSendingMessage = ref(false)
const isStopping = ref(false)

// Reset credentialsSubmitted when a new takeover request comes in.
// Note: `showTakeoverForm` may stay true across retries; we also reset when message/fields change.
watch(
  () => [props.showTakeoverForm, props.takeoverMessage, props.takeoverFields?.length],
  ([show]) => {
    if (show) credentialsSubmitted.value = false
  }
)

// Start/stop login timeout countdown when takeoverForm is shown/hidden
watch(
  () => [props.showTakeoverForm, props.loginTimeoutSeconds],
  ([show, timeout]) => {
    // Clear any existing interval
    if (loginTimeoutInterval) {
      clearInterval(loginTimeoutInterval)
      loginTimeoutInterval = null
    }
    
    if (show && timeout && typeof timeout === 'number' && timeout > 0) {
      // Start countdown
      loginTimeoutRemaining.value = timeout
      loginTimeoutInterval = setInterval(() => {
        if (loginTimeoutRemaining.value !== null && loginTimeoutRemaining.value > 0) {
          loginTimeoutRemaining.value -= 1
        } else {
          // Timeout reached, clear interval
          if (loginTimeoutInterval) {
            clearInterval(loginTimeoutInterval)
            loginTimeoutInterval = null
          }
        }
      }, 1000)
    } else {
      // Reset countdown
      loginTimeoutRemaining.value = null
    }
  },
  { immediate: true }
)

// Initialize credentials when fields change
watch(() => props.takeoverFields, (fields) => {
    const newCreds: Record<string, string> = {}
    if (fields) {
        fields.forEach(f => {
            newCreds[f.id] = ''
        })
    } else {
        // Fallback init
        newCreds.username = ''
        newCreds.password = ''
        newCreds.verificationCode = ''
    }
    credentials.value = newCreds
}, { immediate: true })

const canSubmit = computed(() => {
    if (props.takeoverFields && props.takeoverFields.length > 0) {
        // Check required fields
        return props.takeoverFields.every(f => !f.required || !!credentials.value[f.id])
    }
    return !!credentials.value.username && !!credentials.value.password
})

const canSendMessage = computed(() => {
  return !!(props.executionId && userMessage.value.trim().length > 0)
})

// Multi-Agent helpers
const getWorkerStatus = (taskId: string): string => {
  if (!props.multiAgent?.workers) return 'pending'
  const worker = props.multiAgent.workers.get(taskId)
  return worker?.status || 'pending'
}

const getWorkerProgress = (taskId: string): WorkerProgress | undefined => {
  if (!props.multiAgent?.workers) return undefined
  return props.multiAgent.workers.get(taskId)
}

const recentActivity = computed(() => {
  const list = props.activity || []
  return list.slice(-30).reverse()
})

const formatStatus = (status?: string) => {
  switch (status) {
    case 'running':
      return t('agent.statusRunning')
    case 'completed':
      return t('agent.statusCompleted')
    case 'failed':
      return t('agent.statusFailed')
    case 'pending':
    default:
      return t('agent.statusPending')
  }
}

const formatActivity = (evt: WebActivityEvent) => {
  switch (evt.type) {
    case 'multi_agent_start':
      return t('agent.webActivityMultiAgentStart', { mode: evt.mode, count: evt.total_workers })
    case 'worker_tasks':
      return t('agent.webActivityWorkerTasks', { count: evt.count })
    case 'worker_progress':
      return t('agent.webActivityWorkerProgress', {
        scope: evt.scope_name,
        status: formatStatus(evt.status),
        pages: evt.pages,
        apis: evt.apis,
        pagesLabel: t('agent.webPages')
      })
    case 'worker_complete':
      return t('agent.webActivityWorkerComplete', {
        scope: evt.scope_name,
        pages: evt.pages,
        apis: evt.apis,
        pagesLabel: t('agent.webPages')
      })
    case 'worker_decision': {
      const idx = evt.element_index !== undefined && evt.element_index !== null ? ` [${evt.element_index}]` : ''
      const val = evt.value ? ` = ${String(evt.value).slice(0, 80)}` : ''
      const progress = Number.isFinite(evt.progress) ? Math.round(evt.progress) : 0
      return t('agent.webActivityWorkerDecision', {
        scope: evt.scope_name,
        iteration: evt.iteration,
        action: `${evt.action_type}${idx}${val}`,
        progress
      })
    }
    case 'worker_action': {
      const idx = evt.element_index !== undefined && evt.element_index !== null ? ` [${evt.element_index}]` : ''
      const val = evt.value ? ` = ${String(evt.value).slice(0, 80)}` : ''
      const duration = evt.duration_ms ? `${evt.duration_ms}ms` : ''
      const result = evt.success ? t('agent.statusCompleted') : t('agent.statusFailed')
      return t('agent.webActivityWorkerAction', {
        scope: evt.scope_name,
        iteration: evt.iteration,
        action: `${evt.action_type}${idx}${val}`,
        result,
        duration
      })
    }
    case 'web_plan':
      return t('agent.webActivityPlan', { phase: evt.phase_name || evt.phase })
    case 'web_progress':
      return t('agent.webActivityProgress', { phase: evt.phase, iteration: evt.iteration, max: evt.max_iterations })
    case 'api_discovered':
      return t('agent.webActivityApi', { method: evt.method, url: evt.url })
    case 'takeover_request':
      return t('agent.loginPageDetected')
    case 'complete':
      return t('agent.webActivityComplete', { status: evt.status })
    default:
      // exhaustive guard
      return ''
  }
}

const onUserMessageKeydown = async (e: KeyboardEvent) => {
  if (e.key !== 'Enter') return
  if (e.shiftKey) return
  e.preventDefault()
  await sendUserMessage()
}

const sendUserMessage = async () => {
  const message = userMessage.value.trim()
  if (!message) return
  const eid = props.executionId
  if (!eid) {
    console.warn('[WebExplorerPanel] Missing executionId for sending message')
    return
  }

  isSendingMessage.value = true
  try {
    await invoke('web_explorer_send_user_message', {
      executionId: eid,
      message
    })
    userMessage.value = ''
    console.log('[WebExplorerPanel] User message sent')
  } catch (error) {
    console.error('[WebExplorerPanel] Failed to send user message:', error)
  } finally {
    isSendingMessage.value = false
  }
}

const stopWebExplorer = async () => {
  const eid = props.executionId
  if (!eid) {
    console.warn('[WebExplorerPanel] Missing executionId for stop')
    return
  }
  isStopping.value = true
  try {
    await invoke('cancel_ai_stream', { conversation_id: eid })
    console.log('[WebExplorerPanel] Stop command sent')
  } catch (error) {
    console.error('[WebExplorerPanel] Failed to stop:', error)
  } finally {
    isStopping.value = false
  }
}

// Skip login and continue exploration without credentials
const skipLogin = async () => {
  const eid = props.executionId || (window as any).__webExplorerExecutionId
  if (!eid) {
    console.warn('[WebExplorerPanel] Missing executionId for skip login')
    return
  }

  isSkippingLogin.value = true
  try {
    await invoke('web_explorer_skip_login', { executionId: eid })
    credentialsSubmitted.value = true
    console.log('[WebExplorerPanel] Skip login requested')
  } catch (error) {
    console.error('[WebExplorerPanel] Failed to skip login:', error)
  } finally {
    isSkippingLogin.value = false
  }
}

const isManualLoginCompleting = ref(false)

// Signal manual login completion
const manualLoginComplete = async () => {
  const eid = props.executionId || (window as any).__webExplorerExecutionId
  if (!eid) {
    console.warn('[WebExplorerPanel] Missing executionId for manual login complete')
    return
  }

  isManualLoginCompleting.value = true
  try {
    await invoke('web_explorer_manual_login_complete', { executionId: eid })
    credentialsSubmitted.value = true
    console.log('[WebExplorerPanel] Manual login complete signaled')
  } catch (error) {
    console.error('[WebExplorerPanel] Failed to signal manual login complete:', error)
  } finally {
    isManualLoginCompleting.value = false
  }
}

// Submit credentials to backend
const submitCredentials = async () => {
  if (!canSubmit.value) return
  
  isSubmittingCredentials.value = true
  try {
    const eid = props.executionId || (window as any).__webExplorerExecutionId
    
    if (!eid) {
      console.warn('No execution ID available for credential submission')
      return
    }
    
    // Map credentials to backend expected format
    let username = ''
    let password = ''
    let verificationCode: string | null = null
    let extraFields: Record<string, string> | null = null
    
    if (props.takeoverFields && props.takeoverFields.length > 0) {
        // Dynamic mapping
        const creds = credentials.value
        const extras: Record<string, string> = {}
        
        // Find standard fields by ID convention or fallback
        // We expect backend to send ids: "username", "password", "verification_code" for standard ones
        
        if (creds['username']) username = creds['username']
        if (creds['password']) password = creds['password']
        if (creds['verification_code']) verificationCode = creds['verification_code']
        
        // Put everything else or duplicates into extraFields
        Object.entries(creds).forEach(([key, val]) => {
            if (key !== 'username' && key !== 'password' && key !== 'verification_code') {
                extras[key] = val
            }
        })
        
        if (Object.keys(extras).length > 0) {
            extraFields = extras
        }
    } else {
        // Fallback mapping
        username = credentials.value.username
        password = credentials.value.password
        verificationCode = credentials.value.verificationCode || null
    }
    
    await invoke('web_explorer_receive_credentials', {
      executionId: eid,
      username,
      password,
      verificationCode,
      extraFields
    })
    console.log('[WebExplorerPanel] Credentials submitted successfully')
    
    // Mark as submitted to immediately hide the form
    credentialsSubmitted.value = true
    
    // Reset
    const newCreds: Record<string, string> = {}
    if (props.takeoverFields) {
        props.takeoverFields.forEach(f => newCreds[f.id] = '')
    } else {
        newCreds.username = ''
        newCreds.password = ''
    }
    credentials.value = newCreds
    
  } catch (error) {
    console.error('Failed to submit credentials:', error)
  } finally {
    isSubmittingCredentials.value = false
  }
}

// Auto-scroll to bottom when steps change
watch(() => props.steps.length, async () => {
    await nextTick()
    if (stepsContainer.value) {
        stepsContainer.value.scrollTop = stepsContainer.value.scrollHeight
    }
})

// Cleanup timer on unmount
onUnmounted(() => {
  if (loginTimeoutInterval) {
    clearInterval(loginTimeoutInterval)
    loginTimeoutInterval = null
  }
})
</script>

<style scoped>
.web-explorer-step:last-child .w-0\.5 {
  display: none;
}
</style>
