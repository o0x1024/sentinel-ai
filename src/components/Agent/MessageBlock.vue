<template>
  <div :class="['message-block rounded-lg px-3 py-2 my-1', typeClass]">
    <!-- Header with type indicator -->
    <div class="message-header flex items-center gap-2 mb-2 text-sm" v-if="showHeader">
      <span class="message-type font-semibold text-base-content/70">{{ typeName }}</span>
      <span v-if="toolName" class="tool-name font-mono text-xs text-primary">`{{ toolName }}`</span>
      <!-- 工具状态指示器 -->
      <span v-if="toolStatus" :class="['status-badge px-2 py-0.5 rounded text-xs font-medium', toolStatusClass]">
        {{ toolStatusText }}
      </span>
      <span v-if="statusIcon" :class="['status-icon font-bold', statusClass]">{{ statusIcon }}</span>
      <span v-if="duration" class="duration ml-auto text-xs text-base-content/60">{{ duration }}</span>
    </div>
    
    <!-- RAG引用指示器 -->
    <div v-if="ragInfo" class="rag-indicator flex items-center gap-2 mb-2 px-3 py-2 bg-info/10 rounded-md border border-info/30">
      <i class="fas fa-book text-info text-sm"></i>
      <span class="text-xs text-info font-medium">
        <template v-if="ragInfo.rag_sources_used">
          已引用知识库 ({{ ragInfo.source_count }} 处引用)
        </template>
        <template v-else>
          已启用知识库，但未找到相关内容
        </template>
      </span>
    </div>
    
    <!-- Content -->
    <div class="message-content text-base-content">
      <MarkdownRenderer :content="formattedContent" />
    </div>
    
    <!-- Tool details (collapsible) -->
    <div v-if="message.type === 'tool_call' && (hasToolArgs || hasToolResult)" class="tool-details mt-2 pt-2 border-t border-base-300">
      <button @click="toggleDetails" class="toggle-btn text-xs text-base-content/60 bg-transparent border-none cursor-pointer p-0 underline hover:text-base-content">
        {{ isExpanded ? '收起详情' : '展开详情' }}
      </button>
      <div v-if="isExpanded" class="mt-2 space-y-3">
        <!-- 工具参数 -->
        <div v-if="hasToolArgs" class="tool-args-section">
          <div class="text-xs text-base-content/60 mb-1 font-medium">📥 输入参数:</div>
          <pre class="tool-args p-2 bg-base-300 rounded text-xs font-mono overflow-x-auto text-base-content/70 max-h-48 overflow-y-auto">{{ formattedArgs }}</pre>
        </div>
        <!-- 工具结果（合并显示） -->
        <div v-if="hasToolResult" class="tool-result-section">
          <div class="text-xs text-base-content/60 mb-1 font-medium">📤 执行结果:</div>
          <pre class="tool-result p-2 bg-base-300 rounded text-xs font-mono overflow-x-auto text-base-content/70 max-h-64 overflow-y-auto whitespace-pre-wrap">{{ message.metadata?.tool_result }}</pre>
        </div>
        <!-- 工具调用 ID -->
        <div v-if="message.metadata?.tool_call_id" class="text-xs text-base-content/50">
          ID: <code class="font-mono">{{ message.metadata.tool_call_id }}</code>
        </div>
      </div>
    </div>
    
    <!-- 兜底：独立的 tool_result 消息显示（当无法合并时） -->
    <div v-else-if="message.type === 'tool_result' && (hasToolArgs || message.content)" class="tool-details mt-2 pt-2 border-t border-base-300">
      <button @click="toggleDetails" class="toggle-btn text-xs text-base-content/60 bg-transparent border-none cursor-pointer p-0 underline hover:text-base-content">
        {{ isExpanded ? '收起详情' : '展开详情' }}
      </button>
      <div v-if="isExpanded" class="mt-2 space-y-3">
        <!-- 工具参数 -->
        <div v-if="hasToolArgs" class="tool-args-section">
          <div class="text-xs text-base-content/60 mb-1 font-medium">📥 输入参数:</div>
          <pre class="tool-args p-2 bg-base-300 rounded text-xs font-mono overflow-x-auto text-base-content/70 max-h-48 overflow-y-auto">{{ formattedArgs }}</pre>
        </div>
        <!-- 工具结果 -->
        <div v-if="message.content" class="tool-result-section">
          <div class="text-xs text-base-content/60 mb-1 font-medium">📤 执行结果:</div>
          <pre class="tool-result p-2 bg-base-300 rounded text-xs font-mono overflow-x-auto text-base-content/70 max-h-64 overflow-y-auto whitespace-pre-wrap">{{ message.content }}</pre>
        </div>
        <!-- 工具调用 ID -->
        <div v-if="message.metadata?.tool_call_id" class="text-xs text-base-content/50">
          ID: <code class="font-mono">{{ message.metadata.tool_call_id }}</code>
        </div>
      </div>
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

// RAG信息
const ragInfo = computed(() => props.message.metadata?.rag_info)

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

// Tool status from metadata
const toolStatus = computed(() => props.message.metadata?.status)

// Tool status display class
const toolStatusClass = computed(() => {
  switch (toolStatus.value) {
    case 'running':
      return 'bg-warning/20 text-warning'
    case 'completed':
      return 'bg-success/20 text-success'
    case 'failed':
      return 'bg-error/20 text-error'
    case 'pending':
      return 'bg-base-300 text-base-content/60'
    default:
      return ''
  }
})

// Tool status display text
const toolStatusText = computed(() => {
  switch (toolStatus.value) {
    case 'running':
      return '⏳ 执行中'
    case 'completed':
      return '✓ 已完成'
    case 'failed':
      return '✗ 失败'
    case 'pending':
      return '等待中'
    default:
      return ''
  }
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

// Has tool result (合并显示的结果)
const hasToolResult = computed(() => {
  return !!props.message.metadata?.tool_result
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
    
    case 'progress': {
      const step = metadata?.step_index ?? 0
      const total = metadata?.total_steps ?? 0
      return `**Progress** Step ${step}/${total}\n\n${content}`
    }
    
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
