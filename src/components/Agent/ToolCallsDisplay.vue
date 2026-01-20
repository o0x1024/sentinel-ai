<template>
  <div class="tool-calls-display space-y-2">
    <div
      v-for="(call, index) in parsedToolCalls"
      :key="index"
      class="tool-call-item bg-base-200/50 border border-base-300 rounded-lg overflow-hidden"
    >
      <!-- Tool call header -->
      <div 
        class="flex items-center justify-between px-3 py-2 cursor-pointer hover:bg-base-200/80 transition-colors"
        @click="toggleExpand(index)"
      >
        <div class="flex items-center gap-2">
          <i class="fas fa-wrench text-warning text-xs"></i>
          <span class="text-sm font-medium text-base-content">{{ call.name }}</span>
          <span 
            v-if="call.success !== undefined"
            class="badge badge-xs"
            :class="call.success ? 'badge-success' : 'badge-error'"
          >
            {{ call.success ? 'Success' : 'Failed' }}
          </span>
        </div>
        <div class="flex items-center gap-2">
          <span v-if="call.duration_ms" class="text-xs text-base-content/50">
            {{ formatDuration(call.duration_ms) }}
          </span>
          <i class="fas text-xs text-base-content/50" :class="expandedItems.has(index) ? 'fa-chevron-up' : 'fa-chevron-down'"></i>
        </div>
      </div>

      <!-- Expanded content -->
      <Transition name="expand">
        <div v-if="expandedItems.has(index)" class="border-t border-base-300">
          <!-- Arguments -->
          <div v-if="call.arguments" class="px-3 py-2 border-b border-base-300">
            <div class="text-xs text-base-content/50 mb-1">Arguments:</div>
            <pre class="text-xs font-mono bg-base-300/50 rounded p-2 overflow-x-auto max-h-32 text-base-content">{{ formatJson(call.arguments) }}</pre>
          </div>
          <!-- Result -->
          <div v-if="call.result" class="px-3 py-2">
            <div class="text-xs text-base-content/50 mb-1">Result:</div>
            <pre class="text-xs font-mono bg-base-300/50 rounded p-2 overflow-x-auto max-h-48 text-base-content">{{ formatResult(call.result) }}</pre>
          </div>
        </div>
      </Transition>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'

interface ToolCall {
  id?: string
  name: string
  arguments?: string
  result?: string
  success?: boolean
  duration_ms?: number
}

const props = defineProps<{
  toolCalls: string | null | undefined
}>()

const expandedItems = ref(new Set<number>())

const parsedToolCalls = computed<ToolCall[]>(() => {
  if (!props.toolCalls) return []
  try {
    const parsed = JSON.parse(props.toolCalls)
    if (Array.isArray(parsed)) return parsed
    return [parsed]
  } catch {
    return []
  }
})

const toggleExpand = (index: number) => {
  if (expandedItems.value.has(index)) {
    expandedItems.value.delete(index)
  } else {
    expandedItems.value.add(index)
  }
}

const formatJson = (str: string | undefined) => {
  if (!str) return '-'
  try {
    const parsed = JSON.parse(str)
    return JSON.stringify(parsed, null, 2)
  } catch {
    return str || '-'
  }
}

const formatResult = (str: string | undefined) => {
  if (!str) return '-'
  // Try to parse nested JSON
  try {
    const parsed = JSON.parse(str)
    if (Array.isArray(parsed) && parsed.length > 0 && parsed[0].text) {
      // Handle nested text format like [{type: "text", text: "..."}]
      try {
        const inner = JSON.parse(parsed[0].text)
        return JSON.stringify(inner, null, 2)
      } catch {
        return parsed[0].text
      }
    }
    return JSON.stringify(parsed, null, 2)
  } catch {
    return str.length > 1000 ? str.slice(0, 1000) + '...' : str
  }
}

const formatDuration = (ms: number) => {
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}
</script>

<style scoped>
.expand-enter-active,
.expand-leave-active {
  transition: all 0.2s ease;
  overflow: hidden;
}

.expand-enter-from,
.expand-leave-to {
  opacity: 0;
  max-height: 0;
}

.expand-enter-to,
.expand-leave-from {
  opacity: 1;
  max-height: 500px;
}
</style>
