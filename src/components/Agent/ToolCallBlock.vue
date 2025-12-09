<template>
  <div :class="['tool-call-block rounded-lg bg-base-200 border overflow-hidden my-2', borderClass]">
    <!-- Header -->
    <div class="tool-header flex items-center gap-2 px-4 py-3 bg-base-300 border-b border-base-300">
      <span class="tool-icon text-base">🔧</span>
      <span class="tool-name font-mono text-sm font-semibold text-base-content">{{ toolName }}</span>
      <span :class="['status-badge text-xs px-2 py-0.5 rounded-full ml-auto', badgeClass]">{{ statusText }}</span>
      <span v-if="duration" class="duration text-xs text-base-content/60">{{ duration }}</span>
    </div>
    
    <!-- Arguments (collapsible) -->
    <div v-if="hasArgs" class="tool-args-section border-t border-base-300">
      <button @click="toggleArgs" class="toggle-btn flex items-center gap-2 w-full px-4 py-2 bg-transparent border-none text-base-content/60 text-sm cursor-pointer text-left hover:bg-base-300 hover:text-base-content">
        <span class="toggle-icon text-xs w-4">{{ isArgsExpanded ? '▼' : '▶' }}</span>
        Arguments
      </button>
      <div v-if="isArgsExpanded" class="args-content px-4 py-3 bg-base-100">
        <pre class="m-0 text-sm font-mono text-base-content/70 whitespace-pre-wrap break-words">{{ formattedArgs }}</pre>
      </div>
    </div>
    
    <!-- Result (for completed tools) -->
    <div v-if="hasResult" class="tool-result-section border-t border-base-300">
      <button @click="toggleResult" class="toggle-btn flex items-center gap-2 w-full px-4 py-2 bg-transparent border-none text-base-content/60 text-sm cursor-pointer text-left hover:bg-base-300 hover:text-base-content">
        <span class="toggle-icon text-xs w-4">{{ isResultExpanded ? '▼' : '▶' }}</span>
        Result
      </button>
      <div v-if="isResultExpanded" class="result-content px-4 py-3 bg-base-100">
        <MarkdownRenderer v-if="isMarkdownResult" :content="resultContent" />
        <pre v-else class="m-0 text-sm font-mono text-base-content/70 whitespace-pre-wrap break-words">{{ resultContent }}</pre>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import type { ToolStatus } from '@/types/agent'
import MarkdownRenderer from './MarkdownRenderer.vue'

const props = withDefaults(defineProps<{
  toolName: string
  args?: Record<string, any>
  status: ToolStatus
  result?: any
  error?: string
  durationMs?: number
}>(), {
  status: 'pending',
})

const isArgsExpanded = ref(false)
const isResultExpanded = ref(true)

// Toggle functions
const toggleArgs = () => {
  isArgsExpanded.value = !isArgsExpanded.value
}

const toggleResult = () => {
  isResultExpanded.value = !isResultExpanded.value
}

// Computed properties
const borderClass = computed(() => {
  switch (props.status) {
    case 'running': return 'border-primary'
    case 'completed': return 'border-success'
    case 'failed': return 'border-error'
    default: return 'border-base-300'
  }
})

const badgeClass = computed(() => {
  switch (props.status) {
    case 'pending': return 'bg-base-300 text-base-content/60'
    case 'running': return 'bg-primary/20 text-primary animate-pulse'
    case 'completed': return 'bg-success/20 text-success'
    case 'failed': return 'bg-error/20 text-error'
    default: return 'bg-base-300 text-base-content/60'
  }
})

const statusText = computed(() => {
  const statusMap: Record<ToolStatus, string> = {
    pending: 'Pending',
    running: 'Running...',
    completed: 'Completed',
    failed: 'Failed',
  }
  return statusMap[props.status]
})

const duration = computed(() => {
  if (props.durationMs) {
    return `${(props.durationMs / 1000).toFixed(2)}s`
  }
  return null
})

const hasArgs = computed(() => {
  return props.args && Object.keys(props.args).length > 0
})

const formattedArgs = computed(() => {
  return JSON.stringify(props.args, null, 2)
})

const hasResult = computed(() => {
  return props.status === 'completed' || props.status === 'failed'
})

const resultContent = computed(() => {
  if (props.error) {
    return `Error: ${props.error}`
  }
  if (typeof props.result === 'string') {
    return props.result
  }
  return JSON.stringify(props.result, null, 2)
})

const isMarkdownResult = computed(() => {
  if (typeof props.result !== 'string') return false
  // Check if result contains markdown indicators
  return props.result.includes('#') || 
         props.result.includes('```') || 
         props.result.includes('**') ||
         props.result.includes('- ')
})
</script>

<style scoped>
/* No custom styles needed - using Tailwind utilities */
</style>
