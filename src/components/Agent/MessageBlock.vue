<template>
  <div :class="['message-block rounded-lg px-4 py-3 my-2', typeClass]">
    <!-- Header with type indicator -->
    <div class="message-header flex items-center gap-2 mb-2 text-sm" v-if="showHeader">
      <span class="message-type font-semibold text-base-content/70">{{ typeName }}</span>
      <span v-if="toolName" class="tool-name font-mono text-xs text-primary">`{{ toolName }}`</span>
      <span v-if="statusIcon" :class="['status-icon font-bold', statusClass]">{{ statusIcon }}</span>
      <span v-if="duration" class="duration ml-auto text-xs text-base-content/60">{{ duration }}</span>
    </div>
    
    <!-- Content -->
    <div class="message-content text-base-content">
      <MarkdownRenderer :content="formattedContent" />
    </div>
    
    <!-- Tool details (collapsible for tool_call) -->
    <div v-if="message.type === 'tool_call' && hasToolArgs" class="tool-details mt-2 pt-2 border-t border-base-300">
      <button @click="toggleDetails" class="toggle-btn text-xs text-base-content/60 bg-transparent border-none cursor-pointer p-0 underline hover:text-base-content">
        {{ isExpanded ? '收起详情' : '展开详情' }}
      </button>
      <pre v-if="isExpanded" class="tool-args mt-2 p-2 bg-base-300 rounded text-xs font-mono overflow-x-auto text-base-content/70">{{ formattedArgs }}</pre>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import type { AgentMessage } from '@/types/agent'
import { getMessageTypeName } from '@/types/agent'
import MarkdownRenderer from './MarkdownRenderer.vue'

const props = defineProps<{
  message: AgentMessage
}>()

const isExpanded = ref(false)

const toggleDetails = () => {
  isExpanded.value = !isExpanded.value
}

// Type name
const typeName = computed(() => getMessageTypeName(props.message.type))

// Tool name from metadata
const toolName = computed(() => props.message.metadata?.tool_name)

// Status icon
const statusIcon = computed(() => {
  if (props.message.type === 'tool_result') {
    return props.message.metadata?.success ? '✓' : '✗'
  }
  return null
})

// Status class for icon color
const statusClass = computed(() => {
  if (props.message.type === 'tool_result') {
    return props.message.metadata?.success ? 'text-success' : 'text-error'
  }
  return ''
})

// Duration
const duration = computed(() => {
  const ms = props.message.metadata?.duration_ms
  if (ms) {
    return `${(ms / 1000).toFixed(1)}s`
  }
  return null
})

// Whether to show header
const showHeader = computed(() => {
  return ['tool_call', 'tool_result', 'progress'].includes(props.message.type)
})

// Has tool args
const hasToolArgs = computed(() => {
  return props.message.metadata?.tool_args && 
    Object.keys(props.message.metadata.tool_args).length > 0
})

// Formatted args
const formattedArgs = computed(() => {
  return JSON.stringify(props.message.metadata?.tool_args, null, 2)
})

// Type-specific class
const typeClass = computed(() => {
  switch (props.message.type) {
    case 'thinking':
      return 'type-thinking bg-info/10 border-l-[3px] border-info'
    case 'planning':
      return 'type-planning bg-primary/10 border-l-[3px] border-primary'
    case 'tool_call':
      return 'type-tool_call bg-base-200 border-l-[3px] border-warning'
    case 'tool_result':
      return 'type-tool_result bg-base-200 border-l-[3px] border-success'
    case 'progress':
      return 'type-progress bg-base-200 border-l-[3px] border-base-content/30'
    case 'error':
      return 'type-error bg-error/10 border-l-[3px] border-error'
    case 'final':
      return 'type-final bg-success/5 border-l-[3px] border-success'
    default:
      return 'bg-base-200'
  }
})

// Format content based on message type
const formattedContent = computed(() => {
  const { type, content, metadata } = props.message

  switch (type) {
    case 'thinking':
      return `> **Thinking**\n>\n> ${content.replace(/\n/g, '\n> ')}`
    
    case 'planning':
      return `**Planning**\n\n${content}`
    
    case 'tool_result':
      // Wrap result in code block if not already markdown
      if (!content.includes('```') && !content.includes('#')) {
        return `\`\`\`\n${content}\n\`\`\``
      }
      return content
    
    case 'progress':
      const step = metadata?.step_index ?? 0
      const total = metadata?.total_steps ?? 0
      return `**Progress** Step ${step}/${total}\n\n${content}`
    
    case 'error':
      return `> **Error**\n>\n> ${content}`
    
    case 'final':
      return content
    
    default:
      return content
  }
})
</script>

<style scoped>
.tool-args {
  word-break: break-word;
  white-space: pre-wrap;
}
</style>
