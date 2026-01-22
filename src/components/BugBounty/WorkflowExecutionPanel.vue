<template>
  <div class="space-y-4">
    <!-- Execution Header -->
    <div class="card bg-base-100 shadow-md">
      <div class="card-body p-4">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <div 
              class="w-12 h-12 rounded-lg flex items-center justify-center"
              :class="executionStatusClass"
            >
              <i :class="executionStatusIcon" class="text-xl"></i>
            </div>
            <div>
              <h3 class="font-semibold">{{ template?.name || t('bugBounty.workflow.execution') }}</h3>
              <div class="text-xs text-base-content/60">
                {{ t('bugBounty.workflow.executionId') }}: {{ executionId?.slice(0, 8) }}...
              </div>
            </div>
          </div>
          <div class="flex items-center gap-3">
            <!-- Progress -->
            <div class="text-right">
              <div class="text-2xl font-bold">{{ progress }}%</div>
              <div class="text-xs text-base-content/60">
                {{ completedSteps }}/{{ totalSteps }} {{ t('bugBounty.workflow.stepsCompleted') }}
              </div>
            </div>
            <!-- Actions -->
            <div class="flex gap-2">
              <button 
                v-if="status === 'running'"
                class="btn btn-warning btn-sm"
                @click="pauseExecution"
              >
                <i class="fas fa-pause"></i>
              </button>
              <button 
                v-if="status === 'paused'"
                class="btn btn-success btn-sm"
                @click="resumeExecution"
              >
                <i class="fas fa-play"></i>
              </button>
              <button 
                v-if="status === 'running' || status === 'paused'"
                class="btn btn-error btn-sm"
                @click="cancelExecution"
              >
                <i class="fas fa-stop"></i>
              </button>
            </div>
          </div>
        </div>

        <!-- Progress Bar -->
        <div class="mt-4">
          <progress 
            class="progress w-full" 
            :class="progressClass"
            :value="progress" 
            max="100"
          ></progress>
        </div>

        <!-- Status Summary -->
        <div class="flex items-center justify-between mt-3 text-sm">
          <div class="flex items-center gap-4">
            <span class="flex items-center gap-1">
              <span class="w-2 h-2 rounded-full bg-success"></span>
              {{ stepCounts.completed }} {{ t('bugBounty.workflow.completed') }}
            </span>
            <span class="flex items-center gap-1">
              <span class="w-2 h-2 rounded-full bg-info animate-pulse"></span>
              {{ stepCounts.running }} {{ t('bugBounty.workflow.running') }}
            </span>
            <span class="flex items-center gap-1">
              <span class="w-2 h-2 rounded-full bg-error"></span>
              {{ stepCounts.failed }} {{ t('bugBounty.workflow.failed') }}
            </span>
            <span class="flex items-center gap-1">
              <span class="w-2 h-2 rounded-full bg-base-content/30"></span>
              {{ stepCounts.pending }} {{ t('bugBounty.workflow.pending') }}
            </span>
          </div>
          <div v-if="duration" class="text-base-content/60">
            <i class="fas fa-clock mr-1"></i>{{ formatDuration(duration) }}
          </div>
        </div>
      </div>
    </div>

    <!-- Rate Limit & Retry Status -->
    <div class="grid grid-cols-2 gap-4">
      <!-- Rate Limiter -->
      <div class="card bg-base-100 shadow-sm">
        <div class="card-body p-4">
          <h4 class="font-medium text-sm flex items-center gap-2 mb-3">
            <i class="fas fa-tachometer-alt text-info"></i>
            {{ t('bugBounty.workflow.rateLimiter') }}
          </h4>
          <div class="space-y-2">
            <div class="flex justify-between text-sm">
              <span class="text-base-content/60">{{ t('bugBounty.workflow.globalConcurrency') }}</span>
              <span class="font-mono">{{ rateLimitStats?.global_available || 0 }}/{{ rateLimitStats?.global_limit || 20 }}</span>
            </div>
            <progress 
              class="progress progress-info w-full" 
              :value="rateLimitStats?.global_available || 0" 
              :max="rateLimitStats?.global_limit || 20"
            ></progress>
            <div class="flex justify-between text-xs text-base-content/60">
              <span>{{ t('bugBounty.workflow.perHostLimit') }}: {{ rateLimitStats?.per_host_limit || 5 }}</span>
              <span>{{ t('bugBounty.workflow.delayMs') }}: {{ rateLimitStats?.per_host_delay_ms || 100 }}ms</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Retry Summary -->
      <div class="card bg-base-100 shadow-sm">
        <div class="card-body p-4">
          <h4 class="font-medium text-sm flex items-center gap-2 mb-3">
            <i class="fas fa-redo text-warning"></i>
            {{ t('bugBounty.workflow.retryStatus') }}
          </h4>
          <div class="space-y-2">
            <div class="flex justify-between text-sm">
              <span class="text-base-content/60">{{ t('bugBounty.workflow.totalRetries') }}</span>
              <span class="font-mono">{{ totalRetries }}</span>
            </div>
            <div class="flex justify-between text-sm">
              <span class="text-base-content/60">{{ t('bugBounty.workflow.maxAttempts') }}</span>
              <span class="font-mono">{{ retryConfig?.max_attempts || 3 }}</span>
            </div>
            <div class="flex justify-between text-sm">
              <span class="text-base-content/60">{{ t('bugBounty.workflow.backoffStrategy') }}</span>
              <span class="badge badge-ghost badge-xs">{{ retryConfig?.backoff_type || 'exponential' }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Artifacts Summary -->
    <div class="card bg-base-100 shadow-sm">
      <div class="card-body p-4">
        <h4 class="font-medium text-sm flex items-center gap-2 mb-3">
          <i class="fas fa-cubes text-success"></i>
          {{ t('bugBounty.workflow.artifactsSummary') }}
        </h4>
        <div class="grid grid-cols-5 gap-3">
          <div class="bg-base-200 rounded-lg p-3 text-center">
            <div class="text-2xl font-bold text-primary">{{ artifactSummary.subdomains }}</div>
            <div class="text-xs text-base-content/60">{{ t('bugBounty.workflow.artifactTypes.subdomains') }}</div>
          </div>
          <div class="bg-base-200 rounded-lg p-3 text-center">
            <div class="text-2xl font-bold text-success">{{ artifactSummary.live_hosts }}</div>
            <div class="text-xs text-base-content/60">{{ t('bugBounty.workflow.artifactTypes.liveHosts') }}</div>
          </div>
          <div class="bg-base-200 rounded-lg p-3 text-center">
            <div class="text-2xl font-bold text-secondary">{{ artifactSummary.technologies }}</div>
            <div class="text-xs text-base-content/60">{{ t('bugBounty.workflow.artifactTypes.technologies') }}</div>
          </div>
          <div class="bg-base-200 rounded-lg p-3 text-center">
            <div class="text-2xl font-bold text-error">{{ artifactSummary.findings }}</div>
            <div class="text-xs text-base-content/60">{{ t('bugBounty.workflow.artifactTypes.findings') }}</div>
          </div>
          <div class="bg-base-200 rounded-lg p-3 text-center">
            <div class="text-2xl font-bold text-info">{{ artifactSummary.directories }}</div>
            <div class="text-xs text-base-content/60">{{ t('bugBounty.workflow.artifactTypes.directories') }}</div>
          </div>
        </div>
      </div>
    </div>

    <!-- Step Results -->
    <div class="card bg-base-100 shadow-sm">
      <div class="card-body p-4">
        <h4 class="font-medium text-sm flex items-center gap-2 mb-3">
          <i class="fas fa-list-check text-primary"></i>
          {{ t('bugBounty.workflow.stepResults') }}
        </h4>
        <div class="space-y-3">
          <div 
            v-for="(step, idx) in steps" 
            :key="step.id"
            class="collapse collapse-arrow bg-base-200 rounded-lg"
          >
            <input type="checkbox" :checked="idx === 0" />
            <div class="collapse-title py-2 min-h-0 flex items-center gap-3">
              <div 
                class="w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold"
                :class="getStepStatusClass(step.id)"
              >
                <i v-if="getStepStatus(step.id) === 'running'" class="fas fa-spinner fa-spin"></i>
                <i v-else-if="getStepStatus(step.id) === 'completed'" class="fas fa-check"></i>
                <i v-else-if="getStepStatus(step.id) === 'failed'" class="fas fa-times"></i>
                <span v-else>{{ idx + 1 }}</span>
              </div>
              <div class="flex-1">
                <span class="font-medium">{{ step.name }}</span>
                <span class="text-xs text-base-content/50 ml-2">{{ step.plugin_id || step.tool_name }}</span>
              </div>
              <span 
                class="badge badge-sm"
                :class="{
                  'badge-success': getStepStatus(step.id) === 'completed',
                  'badge-error': getStepStatus(step.id) === 'failed',
                  'badge-info': getStepStatus(step.id) === 'running',
                  'badge-ghost': getStepStatus(step.id) === 'pending'
                }"
              >
                {{ getStepStatus(step.id) }}
              </span>
            </div>
            <div class="collapse-content">
              <div v-if="stepResults[step.id]" class="pt-2">
                <!-- Error display -->
                <div v-if="stepResults[step.id]?.success === false" class="alert alert-error text-sm mb-3">
                  <i class="fas fa-exclamation-circle"></i>
                  <span>{{ stepResults[step.id]?.error || t('bugBounty.workflow.unknownError') }}</span>
                </div>
                
                <!-- Success results -->
                <div v-else class="space-y-3">
                  <!-- Findings count -->
                  <div v-if="stepResults[step.id]?.findings_count > 0" class="flex items-center gap-2 text-sm">
                    <span class="badge badge-error">{{ stepResults[step.id].findings_count }}</span>
                    <span>{{ t('bugBounty.workflow.artifactTypes.findings') }}</span>
                  </div>
                  
                  <!-- Output summary -->
                  <div v-if="stepResults[step.id]?.output" class="space-y-2">
                    <!-- Subdomains -->
                    <div v-if="getOutputArray(step.id, 'subdomains').length > 0" class="text-sm">
                      <div class="font-medium text-base-content/70 mb-1">
                        {{ t('bugBounty.workflow.artifactTypes.subdomains') }} ({{ getOutputArray(step.id, 'subdomains').length }})
                      </div>
                      <div class="bg-base-300 rounded p-2 max-h-40 overflow-auto">
                        <div v-for="(item, i) in getOutputArray(step.id, 'subdomains').slice(0, 20)" :key="i" class="text-xs font-mono py-0.5">
                          {{ item.subdomain || item }}
                        </div>
                        <div v-if="getOutputArray(step.id, 'subdomains').length > 20" class="text-xs text-base-content/50 mt-1">
                          ... {{ t('bugBounty.workflow.andMore', { count: getOutputArray(step.id, 'subdomains').length - 20 }) }}
                        </div>
                      </div>
                    </div>
                    
                    <!-- Hosts -->
                    <div v-if="getOutputArray(step.id, 'hosts').length > 0" class="text-sm">
                      <div class="font-medium text-base-content/70 mb-1">
                        {{ t('bugBounty.workflow.artifactTypes.liveHosts') }} ({{ getOutputArray(step.id, 'hosts').length }})
                      </div>
                      <div class="bg-base-300 rounded p-2 max-h-40 overflow-auto">
                        <div v-for="(item, i) in getOutputArray(step.id, 'hosts').slice(0, 20)" :key="i" class="text-xs font-mono py-0.5 flex items-center gap-2">
                          <span class="badge badge-xs" :class="item.status_code < 400 ? 'badge-success' : 'badge-warning'">{{ item.status_code }}</span>
                          <span>{{ item.url || item }}</span>
                        </div>
                        <div v-if="getOutputArray(step.id, 'hosts').length > 20" class="text-xs text-base-content/50 mt-1">
                          ... {{ t('bugBounty.workflow.andMore', { count: getOutputArray(step.id, 'hosts').length - 20 }) }}
                        </div>
                      </div>
                    </div>
                    
                    <!-- Technologies -->
                    <div v-if="getOutputArray(step.id, 'technologies').length > 0" class="text-sm">
                      <div class="font-medium text-base-content/70 mb-1">
                        {{ t('bugBounty.workflow.artifactTypes.technologies') }} ({{ getOutputArray(step.id, 'technologies').length }})
                      </div>
                      <div class="flex flex-wrap gap-1">
                        <span v-for="(tech, i) in getOutputArray(step.id, 'technologies').slice(0, 30)" :key="i" class="badge badge-outline badge-sm">
                          {{ tech.name || tech }}
                        </span>
                        <span v-if="getOutputArray(step.id, 'technologies').length > 30" class="badge badge-ghost badge-sm">
                          +{{ getOutputArray(step.id, 'technologies').length - 30 }}
                        </span>
                      </div>
                    </div>
                    
                    <!-- Directories -->
                    <div v-if="getOutputArray(step.id, 'directories').length > 0" class="text-sm">
                      <div class="font-medium text-base-content/70 mb-1">
                        {{ t('bugBounty.workflow.artifactTypes.directories') }} ({{ getOutputArray(step.id, 'directories').length }})
                      </div>
                      <div class="bg-base-300 rounded p-2 max-h-40 overflow-auto">
                        <div v-for="(item, i) in getOutputArray(step.id, 'directories').slice(0, 20)" :key="i" class="text-xs font-mono py-0.5">
                          {{ item.path || item.url || item }}
                        </div>
                        <div v-if="getOutputArray(step.id, 'directories').length > 20" class="text-xs text-base-content/50 mt-1">
                          ... {{ t('bugBounty.workflow.andMore', { count: getOutputArray(step.id, 'directories').length - 20 }) }}
                        </div>
                      </div>
                    </div>
                    
                    <!-- Endpoints -->
                    <div v-if="getOutputArray(step.id, 'endpoints').length > 0" class="text-sm">
                      <div class="font-medium text-base-content/70 mb-1">
                        {{ t('bugBounty.workflow.artifactTypes.endpoints') }} ({{ getOutputArray(step.id, 'endpoints').length }})
                      </div>
                      <div class="bg-base-300 rounded p-2 max-h-40 overflow-auto">
                        <div v-for="(item, i) in getOutputArray(step.id, 'endpoints').slice(0, 20)" :key="i" class="text-xs font-mono py-0.5">
                          {{ item.url || item.path || item }}
                        </div>
                        <div v-if="getOutputArray(step.id, 'endpoints').length > 20" class="text-xs text-base-content/50 mt-1">
                          ... {{ t('bugBounty.workflow.andMore', { count: getOutputArray(step.id, 'endpoints').length - 20 }) }}
                        </div>
                      </div>
                    </div>
                    
                    <!-- JS Files -->
                    <div v-if="getOutputArray(step.id, 'jsFiles').length > 0" class="text-sm">
                      <div class="font-medium text-base-content/70 mb-1">
                        JS Files ({{ getOutputArray(step.id, 'jsFiles').length }})
                      </div>
                      <div class="bg-base-300 rounded p-2 max-h-40 overflow-auto">
                        <div v-for="(item, i) in getOutputArray(step.id, 'jsFiles').slice(0, 20)" :key="i" class="text-xs font-mono py-0.5">
                          {{ item.url || item }}
                        </div>
                        <div v-if="getOutputArray(step.id, 'jsFiles').length > 20" class="text-xs text-base-content/50 mt-1">
                          ... {{ t('bugBounty.workflow.andMore', { count: getOutputArray(step.id, 'jsFiles').length - 20 }) }}
                        </div>
                      </div>
                    </div>
                    
                    <!-- Secrets -->
                    <div v-if="getOutputArray(step.id, 'secrets').length > 0" class="text-sm">
                      <div class="font-medium text-base-content/70 mb-1">
                        <i class="fas fa-key text-warning mr-1"></i>
                        {{ t('bugBounty.workflow.artifactTypes.secrets') }} ({{ getOutputArray(step.id, 'secrets').length }})
                      </div>
                      <div class="bg-base-300 rounded p-2 max-h-40 overflow-auto">
                        <div v-for="(item, i) in getOutputArray(step.id, 'secrets').slice(0, 10)" :key="i" class="text-xs py-1 border-b border-base-content/10 last:border-0">
                          <div class="font-medium">{{ item.type || 'Secret' }}</div>
                          <div class="font-mono text-base-content/60 truncate">{{ item.value || item }}</div>
                        </div>
                      </div>
                    </div>
                    
                    <!-- Raw output toggle -->
                    <div class="collapse collapse-arrow bg-base-300 rounded mt-2">
                      <input type="checkbox" />
                      <div class="collapse-title text-xs py-1 min-h-0">
                        {{ t('bugBounty.workflow.rawOutput') }}
                      </div>
                      <div class="collapse-content">
                        <pre class="text-xs font-mono overflow-auto max-h-60 whitespace-pre-wrap">{{ JSON.stringify(stepResults[step.id]?.output, null, 2) }}</pre>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
              <div v-else class="pt-2 text-sm text-base-content/50">
                {{ t('bugBounty.workflow.waitingForResult') }}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Data Flow Visualization -->
    <div class="card bg-base-100 shadow-sm">
      <div class="card-body p-4">
        <WorkflowDataFlow
          :steps="steps"
          :initial-inputs="initialInputs"
          :execution-id="executionId"
          :step-results="stepResults"
          @step-select="onStepSelect"
        />
      </div>
    </div>

    <!-- Step Logs -->
    <div class="card bg-base-100 shadow-sm">
      <div class="card-body p-4">
        <h4 class="font-medium text-sm flex items-center gap-2 mb-3">
          <i class="fas fa-terminal text-base-content/60"></i>
          {{ t('bugBounty.workflow.executionLog') }}
        </h4>
        <div class="bg-base-200 rounded-lg p-3 max-h-60 overflow-auto font-mono text-xs">
          <div 
            v-for="log in executionLogs" 
            :key="log.id"
            class="flex items-start gap-2 py-1"
            :class="{ 'text-error': log.level === 'error', 'text-warning': log.level === 'warning' }"
          >
            <span class="text-base-content/40 shrink-0">{{ formatLogTime(log.timestamp) }}</span>
            <span class="badge badge-ghost badge-xs shrink-0">{{ log.step_id }}</span>
            <span>{{ log.message }}</span>
          </div>
          <div v-if="executionLogs.length === 0" class="text-base-content/50 text-center py-4">
            {{ t('bugBounty.workflow.noLogs') }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import WorkflowDataFlow from './WorkflowDataFlow.vue'

const { t } = useI18n()

interface WorkflowStep {
  id: string
  name: string
  step_type: string
  plugin_id?: string
  tool_name?: string
  config: any
  depends_on: string[]
}

interface ExecutionLog {
  id: string
  timestamp: string
  step_id: string
  level: 'info' | 'warning' | 'error'
  message: string
}

const props = defineProps<{
  executionId: string
  template?: any
  steps: WorkflowStep[]
  initialInputs?: any
}>()

const emit = defineEmits<{
  (e: 'complete', result: any): void
  (e: 'error', error: any): void
}>()

// State
const status = ref<'pending' | 'running' | 'paused' | 'completed' | 'failed' | 'cancelled'>('pending')
const progress = ref(0)
const completedSteps = ref(0)
const totalSteps = ref(0)
const stepResults = ref<Record<string, any>>({})
const stepRetries = ref<Record<string, number>>({})
const runningSteps = ref<Set<string>>(new Set()) // Track currently running steps
const executionLogs = ref<ExecutionLog[]>([])
const rateLimitStats = ref<any>(null)
const retryConfig = ref<any>(null)
const duration = ref(0)
const startTime = ref<Date | null>(null)
let durationInterval: any = null
let unlistenProgress: any = null
let unlistenStepStart: any = null
let unlistenStepComplete: any = null
let unlistenComplete: any = null

// Computed
const executionStatusClass = computed(() => {
  switch (status.value) {
    case 'completed': return 'bg-success/20 text-success'
    case 'running': return 'bg-info/20 text-info'
    case 'failed': return 'bg-error/20 text-error'
    case 'paused': return 'bg-warning/20 text-warning'
    case 'cancelled': return 'bg-base-300 text-base-content/50'
    default: return 'bg-base-200 text-base-content/50'
  }
})

const executionStatusIcon = computed(() => {
  switch (status.value) {
    case 'completed': return 'fas fa-check-circle'
    case 'running': return 'fas fa-spinner fa-spin'
    case 'failed': return 'fas fa-times-circle'
    case 'paused': return 'fas fa-pause-circle'
    case 'cancelled': return 'fas fa-ban'
    default: return 'fas fa-hourglass-half'
  }
})

const progressClass = computed(() => {
  switch (status.value) {
    case 'completed': return 'progress-success'
    case 'running': return 'progress-info'
    case 'failed': return 'progress-error'
    default: return 'progress-primary'
  }
})

const stepCounts = computed(() => {
  const counts = { completed: 0, running: 0, failed: 0, pending: 0 }
  for (const step of props.steps) {
    // Check if step is currently running
    if (runningSteps.value.has(step.id)) {
      counts.running++
      continue
    }
    
    const result = stepResults.value[step.id]
    if (!result) {
      counts.pending++
    } else if (result.error || result.success === false) {
      counts.failed++
    } else {
      counts.completed++
    }
  }
  return counts
})

const totalRetries = computed(() => {
  return Object.values(stepRetries.value).reduce((sum, n) => sum + n, 0)
})

const artifactSummary = computed(() => {
  const summary = {
    subdomains: 0,
    live_hosts: 0,
    technologies: 0,
    findings: 0,
    directories: 0,
    endpoints: 0,
    secrets: 0,
  }
  // Aggregate from step results
  for (const result of Object.values(stepResults.value)) {
    if (result?.output?.subdomains) {
      summary.subdomains += result.output.subdomains.length || 0
    }
    if (result?.output?.hosts) {
      summary.live_hosts += result.output.hosts.length || 0
    }
    if (result?.output?.technologies) {
      summary.technologies += result.output.technologies.length || 0
    }
    if (result?.findings) {
      summary.findings += result.findings || 0
    }
    if (result?.output?.directories) {
      summary.directories += result.output.directories.length || 0
    }
  }
  return summary
})

// Methods
const loadRateLimitStats = async () => {
  try {
    rateLimitStats.value = await invoke('bounty_get_rate_limiter_stats')
  } catch (error) {
    console.error('Failed to load rate limit stats:', error)
  }
}

const loadRetryConfig = async () => {
  try {
    retryConfig.value = await invoke('bounty_get_default_retry_config')
  } catch (error) {
    console.error('Failed to load retry config:', error)
  }
}

const setupEventListeners = async () => {
  // Listen for progress updates
  unlistenProgress = await listen<any>('workflow:progress', (event) => {
    if (event.payload.execution_id === props.executionId) {
      progress.value = event.payload.progress
      completedSteps.value = event.payload.completed_steps
      totalSteps.value = event.payload.total_steps
    }
  })

  // Listen for step start (running status)
  unlistenStepStart = await listen<any>('workflow:step-start', (event) => {
    if (event.payload.execution_id === props.executionId) {
      const stepId = event.payload.step_id
      const stepName = event.payload.step_name || stepId
      
      // Mark step as running
      runningSteps.value.add(stepId)
      addLog('info', stepId, `${stepName} started`)
    }
  })

  // Listen for step completions
  unlistenStepComplete = await listen<any>('workflow:step-complete', (event) => {
    if (event.payload.execution_id === props.executionId) {
      const stepId = event.payload.step_id
      const stepName = event.payload.step_name || stepId
      const success = event.payload.success
      
      // Remove from running set
      runningSteps.value.delete(stepId)
      
      // Store result or error
      if (success) {
        stepResults.value[stepId] = event.payload.result
        addLog('info', stepId, `${stepName} completed successfully`)
      } else {
        stepResults.value[stepId] = { success: false, error: event.payload.error }
        addLog('error', stepId, `${stepName} failed: ${event.payload.error}`)
      }
    }
  })

  // Listen for execution completion
  unlistenComplete = await listen<any>('workflow:run-complete', (event) => {
    if (event.payload.execution_id === props.executionId) {
      const completionStatus = event.payload.status
      if (completionStatus === 'completed') {
        status.value = 'completed'
      } else if (completionStatus === 'completed_with_errors') {
        status.value = 'failed'
      } else {
        status.value = 'completed'
      }
      progress.value = 100
      
      // Merge final results
      if (event.payload.results) {
        stepResults.value = { ...stepResults.value, ...event.payload.results }
      }
      
      stopDurationTimer()
      
      if (event.payload.errors?.length > 0) {
        addLog('warning', 'workflow', `Workflow completed with ${event.payload.errors.length} error(s)`)
        emit('error', event.payload.errors)
      } else {
        addLog('info', 'workflow', 'Workflow completed successfully')
        emit('complete', stepResults.value)
      }
    }
  })
}

const startDurationTimer = () => {
  startTime.value = new Date()
  durationInterval = setInterval(() => {
    if (startTime.value) {
      duration.value = Date.now() - startTime.value.getTime()
    }
  }, 1000)
}

const stopDurationTimer = () => {
  if (durationInterval) {
    clearInterval(durationInterval)
    durationInterval = null
  }
}

const addLog = (level: 'info' | 'warning' | 'error', stepId: string, message: string) => {
  executionLogs.value.push({
    id: `${Date.now()}-${Math.random()}`,
    timestamp: new Date().toISOString(),
    step_id: stepId,
    level,
    message,
  })
}

const pauseExecution = async () => {
  // TODO: Implement pause
  status.value = 'paused'
  stopDurationTimer()
}

const resumeExecution = async () => {
  // TODO: Implement resume
  status.value = 'running'
  startDurationTimer()
}

const cancelExecution = async () => {
  try {
    await invoke('cancel_workflow_run', { executionId: props.executionId })
    status.value = 'cancelled'
    stopDurationTimer()
  } catch (error) {
    console.error('Failed to cancel execution:', error)
  }
}

const onStepSelect = (step: any) => {
  // Can show detailed step info
}

const formatDuration = (ms: number) => {
  const seconds = Math.floor(ms / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  
  if (hours > 0) {
    return `${hours}h ${minutes % 60}m ${seconds % 60}s`
  }
  if (minutes > 0) {
    return `${minutes}m ${seconds % 60}s`
  }
  return `${seconds}s`
}

const formatLogTime = (timestamp: string) => {
  return new Date(timestamp).toLocaleTimeString()
}

// Get step execution status
const getStepStatus = (stepId: string): 'pending' | 'running' | 'completed' | 'failed' => {
  // Check if step is currently running
  if (runningSteps.value.has(stepId)) {
    return 'running'
  }
  
  const result = stepResults.value[stepId]
  if (!result) {
    return 'pending'
  }
  if (result.success === false) {
    return 'failed'
  }
  return 'completed'
}

// Get step status class
const getStepStatusClass = (stepId: string) => {
  const s = getStepStatus(stepId)
  switch (s) {
    case 'completed': return 'bg-success text-success-content'
    case 'failed': return 'bg-error text-error-content'
    case 'running': return 'bg-info text-info-content'
    default: return 'bg-base-300 text-base-content/50'
  }
}

// Get output array from step result
const getOutputArray = (stepId: string, key: string): any[] => {
  const result = stepResults.value[stepId]
  if (!result?.output) return []
  return result.output[key] || []
}

// Lifecycle
onMounted(async () => {
  totalSteps.value = props.steps.length
  status.value = 'running'
  startDurationTimer()
  
  await Promise.all([
    loadRateLimitStats(),
    loadRetryConfig(),
    setupEventListeners(),
  ])

  // Refresh rate limit stats periodically
  const refreshInterval = setInterval(loadRateLimitStats, 5000)
  onUnmounted(() => clearInterval(refreshInterval))
})

onUnmounted(() => {
  stopDurationTimer()
  unlistenProgress?.()
  unlistenStepStart?.()
  unlistenStepComplete?.()
  unlistenComplete?.()
})
</script>
