<template>
  <div :class="['tool-call-block rounded-lg bg-base-200 border overflow-hidden my-2 ', borderClass]">
    <!-- Header -->
    <div class="tool-header flex items-center gap-2 px-4 py-3 bg-base-300 border-b border-base-300">
      <span class="tool-icon text-base">🔧</span>
      <span class="tool-name font-mono text-sm font-semibold text-base-content">{{ toolName }}</span>
      <span :class="['status-badge text-xs px-2 py-0.5 rounded-full ml-auto', badgeClass]">{{ statusText }}</span>
      <span v-if="duration" class="duration text-xs text-base-content/60">{{ duration }}</span>
    </div>
    
    <!-- Arguments (collapsible with preview) -->
    <div v-if="hasArgs" class="tool-args-section border-t border-base-300">
      <div 
        ref="argsBodyRef"
        @click="toggleArgs"
        :class="['args-content px-4 py-3 bg-base-100 cursor-pointer transition-all relative', 
                 isArgsExpanded ? 'max-h-96 overflow-y-auto' : 'max-h-24 overflow-hidden']"
      >
        <pre class="m-0 text-sm font-mono text-base-content/70 whitespace-pre-wrap break-words">{{ formattedArgs }}</pre>
        
        <!-- Expand hint overlay (shown when collapsed and content overflows) -->
        <div v-if="!isArgsExpanded && argsHasOverflow" class="expand-hint absolute bottom-0 left-0 right-0 h-8 bg-gradient-to-t from-base-100 to-transparent flex items-end justify-center pb-1 pointer-events-none">
          <span class="text-base-content/50 text-xs">点击展开</span>
        </div>
      </div>
    </div>
    
    <!-- Result (for completed tools with preview) -->
    <div v-if="hasResult" class="tool-result-section border-t border-base-300">
      <div 
        ref="resultBodyRef"
        @click="toggleResult"
        :class="['result-content px-4 py-3 bg-base-100 cursor-pointer transition-all relative', 
                 isResultExpanded ? 'max-h-96 overflow-y-auto' : 'max-h-24 overflow-hidden']"
      >
        <MarkdownRenderer v-if="isMarkdownResult" :content="resultContent" />
        <pre v-else class="m-0 text-sm font-mono text-base-content/70 whitespace-pre-wrap break-words">{{ resultContent }}</pre>
        
        <!-- Expand hint overlay (shown when collapsed and content overflows) -->
        <div v-if="!isResultExpanded && resultHasOverflow" class="expand-hint absolute bottom-0 left-0 right-0 h-8 bg-gradient-to-t from-base-100 to-transparent flex items-end justify-center pb-1 pointer-events-none">
          <span class="text-base-content/50 text-xs">点击展开</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, nextTick, watch } from 'vue'
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
const isResultExpanded = ref(false)
const argsHasOverflow = ref(false)
const resultHasOverflow = ref(false)
const argsBodyRef = ref<HTMLElement | null>(null)
const resultBodyRef = ref<HTMLElement | null>(null)

// Toggle functions
const toggleArgs = () => {
  isArgsExpanded.value = !isArgsExpanded.value
}

const toggleResult = () => {
  isResultExpanded.value = !isResultExpanded.value
}

// Check if content overflows
function checkArgsOverflow() {
  nextTick(() => {
    if (argsBodyRef.value) {
      argsHasOverflow.value = argsBodyRef.value.scrollHeight > argsBodyRef.value.clientHeight
    }
  })
}

function checkResultOverflow() {
  nextTick(() => {
    if (resultBodyRef.value) {
      resultHasOverflow.value = resultBodyRef.value.scrollHeight > resultBodyRef.value.clientHeight
    }
  })
}

// Check overflow on mount and when content changes
onMounted(() => {
  checkArgsOverflow()
  checkResultOverflow()
})

// Watch for content changes
watch([() => props.args, () => props.result], () => {
  checkArgsOverflow()
  checkResultOverflow()
})

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
/* Scrollbar styles */
.args-content::-webkit-scrollbar,
.result-content::-webkit-scrollbar {
  width: 8px;
}

.args-content::-webkit-scrollbar-track,
.result-content::-webkit-scrollbar-track {
  background: transparent;
}

.args-content::-webkit-scrollbar-thumb,
.result-content::-webkit-scrollbar-thumb {
  background: #424242;
  border-radius: 4px;
}

.args-content::-webkit-scrollbar-thumb:hover,
.result-content::-webkit-scrollbar-thumb:hover {
  background: #555;
}
</style>
