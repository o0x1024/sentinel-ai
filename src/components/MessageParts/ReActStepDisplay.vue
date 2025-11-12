<template>
  <div class="react-step-display">
    <!-- Thought Section -->
    <div v-if="thought" class="thought-section mb-3">
      <div class="flex items-start gap-2 p-3 bg-base-200/50 rounded-lg border border-base-300/30">
        <div class="flex-shrink-0 mt-0.5">
          <i class="fas fa-lightbulb text-warning text-sm"></i>
        </div>
        <div class="flex-1 min-w-0">
          <div class="text-xs font-semibold text-base-content/70 mb-1">Thought</div>
          <div class="text-sm text-base-content prose prose-sm max-w-none" v-html="renderMarkdown(thought)"></div>
        </div>
      </div>
    </div>

    <!-- Tool Call Section (Action + Observation combined) -->
    <div v-if="action" class="tool-call-section mb-3">
      <details 
        class="collapse collapse-arrow bg-base-100 border rounded-lg"
        :class="getToolCallBorderClass()"
        open
      >
        <summary class="collapse-title min-h-0 py-3 px-4 cursor-pointer hover:bg-base-200/50 transition-colors">
          <div class="flex items-center gap-3">
            <!-- Status Icon -->
            <div class="flex-shrink-0">
              <i 
                :class="[
                  'text-sm',
                  getToolCallIconClass()
                ]"
              ></i>
            </div>
            
            <!-- Tool Info -->
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2 flex-wrap">
                <span class="font-medium text-sm">Ran</span>
                <code class="text-xs bg-base-200 px-2 py-0.5 rounded font-mono">{{ action.tool }}</code>
                <span v-if="action.status" class="badge badge-xs" :class="getActionStatusClass(action.status)">
                  {{ getActionStatusText(action.status) }}
                </span>
              </div>
            </div>
          </div>
        </summary>
        
        <div class="collapse-content px-4 pb-4">
          <div class="space-y-3">
            <!-- Parameters Section -->
            <div v-if="action.args && Object.keys(action.args).length > 0" class="params-section">
              <div class="text-xs font-semibold text-base-content/70 mb-2 flex items-center gap-1">
                <i class="fas fa-cog text-xs"></i>
                PARAMETERS
              </div>
              <div class="bg-base-200/50 rounded-lg p-3 border border-base-300/30">
                <div v-for="(value, key) in formatParams(action.args)" :key="key" class="flex items-start gap-2 py-1">
                  <span class="text-xs font-medium text-base-content/70 min-w-[100px]">{{ key }}</span>
                  <span class="text-xs text-base-content font-mono break-all">{{ value }}</span>
                </div>
              </div>
            </div>
            
            <!-- Response Section -->
            <div v-if="observation" class="response-section">
              <div class="text-xs font-semibold text-base-content/70 mb-2 flex items-center gap-1">
                <i class="fas fa-arrow-left text-xs"></i>
                RESPONSE
              </div>
              <div 
                :class="[
                  'rounded-lg p-3 border',
                  hasObservationError(observation)
                    ? 'bg-error/5 border-error/20'
                    : 'bg-success/5 border-success/20'
                ]"
              >
                <pre class="text-xs whitespace-pre-wrap break-words font-mono text-base-content">{{ formatObservation(observation) }}</pre>
              </div>
            </div>
          </div>
        </div>
      </details>
    </div>

    <!-- Error Section -->
    <div v-if="error" class="error-section mb-3">
      <div class="flex items-start gap-2 p-3 bg-error/5 rounded-lg border border-error/20">
        <div class="flex-shrink-0 mt-0.5">
          <i class="fas fa-exclamation-triangle text-error text-sm"></i>
        </div>
        <div class="flex-1 min-w-0">
          <div class="text-xs font-semibold text-error mb-1">Error</div>
          <div class="text-sm text-error whitespace-pre-wrap">{{ error }}</div>
        </div>
      </div>
    </div>

    <!-- Final Answer Section -->
    <div v-if="finalAnswer" class="final-answer-section">
      <div class="flex items-start gap-2 p-3 bg-accent/5 rounded-lg border border-accent/20">
        <div class="flex-shrink-0 mt-0.5">
          <i class="fas fa-flag-checkered text-accent text-sm"></i>
        </div>
        <div class="flex-1 min-w-0">
          <div class="text-xs font-semibold text-base-content/70 mb-1">Final Answer</div>
          <div class="text-sm text-base-content prose prose-sm max-w-none" v-html="renderMarkdown(finalAnswer)"></div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useMessageUtils } from '../../composables/useMessageUtils'

interface ReActAction {
  tool: string
  args?: any
  status?: string
}

interface ReActStepData {
  thought?: string
  action?: ReActAction | string
  observation?: any
  error?: string
  finalAnswer?: string
}

const props = defineProps<{
  stepData: ReActStepData
}>()

const { renderMarkdown } = useMessageUtils()

const thought = computed(() => props.stepData.thought)
const action = computed(() => {
  const act = props.stepData.action
  if (typeof act === 'string') {
    try {
      return JSON.parse(act)
    } catch {
      return { tool: act }
    }
  }
  return act
})
const observation = computed(() => props.stepData.observation)
const error = computed(() => props.stepData.error)
const finalAnswer = computed(() => props.stepData.finalAnswer)

const formatJson = (obj: any) => {
  try {
    return JSON.stringify(obj, null, 2)
  } catch {
    return String(obj)
  }
}

const formatParams = (args: any) => {
  if (!args) return {}
  if (typeof args === 'object') {
    return args
  }
  try {
    return JSON.parse(args)
  } catch {
    return { value: args }
  }
}

const formatObservation = (obs: any) => {
  if (typeof obs === 'string') return obs
  try {
    return JSON.stringify(obs, null, 2)
  } catch {
    return String(obs)
  }
}

const getToolCallBorderClass = () => {
  if (!action.value) return 'border-base-300'
  const status = action.value.status
  if (status === 'failed' || status === 'error') {
    return 'border-error'
  }
  if (status === 'running') {
    return 'border-warning'
  }
  return 'border-success'
}

const getToolCallIconClass = () => {
  if (!action.value) return 'fas fa-check text-success'
  const status = action.value.status
  if (status === 'failed' || status === 'error') {
    return 'fas fa-times-circle text-error'
  }
  if (status === 'running') {
    return 'fas fa-spinner fa-spin text-warning'
  }
  return 'fas fa-check text-success'
}

const getActionStatusClass = (status: string) => {
  const statusMap: Record<string, string> = {
    running: 'badge-warning',
    success: 'badge-success',
    completed: 'badge-success',
    failed: 'badge-error',
    error: 'badge-error',
  }
  return statusMap[status.toLowerCase()] || 'badge-ghost'
}

const getActionStatusText = (status: string) => {
  const textMap: Record<string, string> = {
    running: '运行中',
    success: '成功',
    completed: '已完成',
    failed: '失败',
    error: '错误',
  }
  return textMap[status.toLowerCase()] || status
}

const hasObservationError = (obs: any) => {
  if (typeof obs === 'string') {
    const lowerObs = obs.toLowerCase()
    return lowerObs.includes('error') || 
           lowerObs.includes('failed') || 
           lowerObs.includes('失败') ||
           lowerObs.includes('"success":false') ||
           lowerObs.includes('"success": false')
  }
  if (typeof obs === 'object' && obs !== null) {
    return obs.success === false || obs.error
  }
  return false
}
</script>

<style scoped>
.react-step-display {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

/* DaisyUI collapse customization */
.collapse {
  border-width: 2px;
}

.collapse-title {
  padding-right: 3rem;
}

.collapse:not(.collapse-close) > .collapse-title:after {
  top: 50%;
  transform: translateY(-50%) rotate(0deg);
  transition: transform 0.2s ease;
}

.collapse[open] > .collapse-title:after {
  transform: translateY(-50%) rotate(90deg);
}

/* Custom scrollbar */
.response-section pre {
  max-height: 300px;
  overflow-y: auto;
  scrollbar-width: thin;
  scrollbar-color: hsl(var(--bc) / 0.2) transparent;
}

.response-section pre::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

.response-section pre::-webkit-scrollbar-track {
  background: transparent;
}

.response-section pre::-webkit-scrollbar-thumb {
  background: hsl(var(--bc) / 0.2);
  border-radius: 3px;
}

.response-section pre::-webkit-scrollbar-thumb:hover {
  background: hsl(var(--bc) / 0.3);
}

/* Hover effects */
.collapse:hover {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

/* Parameters grid layout */
.params-section > div > div {
  display: flex;
  gap: 0.5rem;
}

/* Smooth transitions */
.collapse {
  transition: all 0.2s ease;
}

.collapse-title {
  transition: background-color 0.15s ease;
}
</style>
