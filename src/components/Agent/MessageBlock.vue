<template>
  <!-- Shell Tool - Render as independent message block -->
  <ShellToolResult
    v-if="isShellTool && message.type === 'tool_call'"
    :args="message.metadata?.tool_args"
    :result="message.metadata?.tool_result"
    :error="message.metadata?.error"
    :status="message.metadata?.status"
  />
  
  <!-- Tool Call Message - Collapsible Panel (only render if has content) -->
  <div v-else-if="message.type === 'tool_call' && hasToolCallContent" class="tool-call-panel rounded-lg overflow-hidden  bg-base-200 border-l-4" :class="toolPanelBorderClass">
    <!-- Panel Header (always visible) -->
    <div 
      @click="toggleToolPanel" 
      class="tool-panel-header flex items-center gap-2 px-4 py-3 cursor-pointer hover:bg-base-300/50 transition-colors"
    >
      <!-- Expand/Collapse Icon -->
      <i :class="['fas transition-transform text-xs', isToolPanelExpanded ? 'fa-chevron-down' : 'fa-chevron-right']"></i>
      
      <!-- Tool Name -->
      <span class="font-mono text-sm font-semibold">{{ toolName || 'Tool' }}</span>
      
      <!-- Status Badge -->
      <span v-if="toolStatus" :class="['status-badge px-2 py-0.5 rounded-full text-xs font-medium ml-auto', toolStatusClass]">
        {{ toolStatusText }}
      </span>
      
      <!-- Duration -->
      <span v-if="duration" class="text-xs text-base-content/60">{{ duration }}</span>
    </div>
    
    <!-- Panel Content (collapsible) -->
    <div v-show="isToolPanelExpanded" class="tool-panel-content">
      <!-- Tool Arguments -->
      <div v-if="hasToolArgs" class="border-t border-base-300">
        <div 
          ref="argsBodyRef"
          @click="toggleArgs"
          :class="['px-4 py-3 bg-base-100 cursor-pointer transition-all relative', 
                   isArgsExpanded ? 'max-h-96 overflow-y-auto' : 'max-h-24 overflow-hidden']"
        >
          <div class="text-xs text-base-content/50 mb-2">📥 {{ t('agent.inputParameters') }}</div>
          <pre class="text-xs font-mono text-base-content/70 whitespace-pre-wrap break-words overflow-x-auto">{{ formattedArgs }}</pre>
          
          <!-- Expand hint overlay -->
          <div v-if="!isArgsExpanded && argsHasOverflow" class="expand-hint absolute bottom-0 left-0 right-0 h-8 bg-gradient-to-t from-base-100 to-transparent flex items-end justify-center pb-1 pointer-events-none">
            <span class="text-base-content/50 text-xs">点击展开</span>
          </div>
        </div>
      </div>
      
      <!-- Tool Result -->
      <div v-if="hasToolResult" class="border-t border-base-300">
        <div 
          ref="resultBodyRef"
          @click="toggleResult"
          :class="['px-4 py-3 bg-base-100 cursor-pointer transition-all relative', 
                   isResultExpanded ? 'max-h-96 overflow-y-auto' : 'max-h-24 overflow-hidden']"
        >
          <div class="text-xs text-base-content/50 mb-2">📤 {{ t('agent.executionResult') }}</div>
          <pre class="text-xs font-mono text-base-content/70 whitespace-pre-wrap break-words overflow-x-auto">{{ formattedToolResult }}</pre>
          
          <!-- Expand hint overlay -->
          <div v-if="!isResultExpanded && resultHasOverflow" class="expand-hint absolute bottom-0 left-0 right-0 h-8 bg-gradient-to-t from-base-100 to-transparent flex items-end justify-center pb-1 pointer-events-none">
            <span class="text-base-content/50 text-xs">点击展开</span>
          </div>
        </div>
      </div>
      
      <!-- Tool Call ID -->
      <div v-if="message.metadata?.tool_call_id" class="px-4 py-2 border-t border-base-300 bg-base-100">
        <span class="text-xs text-base-content/50">
          {{ t('agent.toolCallId') }}: <code class="font-mono">{{ message.metadata.tool_call_id }}</code>
        </span>
      </div>
    </div>
  </div>

  <!-- Regular message block for non-tool-call messages (only render if has content) -->
  <div v-else-if="hasRegularMessageContent" class="message-container group relative max-w-full">
    <div :class="['message-block relative rounded-lg px-3 py-2 overflow-hidden', typeClass]">
      <!-- Actions (overlay) -->
      <div
        v-if="message.type === 'user' || message.type === 'final'"
        class="message-actions absolute right-2 top-2 z-10"
      >
        <!-- Desktop/hover: icon buttons -->
        <div class="hidden md:flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none group-hover:pointer-events-auto">
          <button
            @click="handleCopy"
            class="action-btn btn btn-xs btn-ghost bg-base-100/70 hover:bg-base-100 backdrop-blur text-base-content/60 hover:text-base-content"
            :title="t('agent.copyMessage')"
          >
            <i :class="['fas', copySuccess ? 'fa-check text-success' : 'fa-copy']"></i>
          </button>
          <button
            v-if="message.type === 'user'"
            @click="handleResend"
            class="action-btn btn btn-xs btn-ghost bg-base-100/70 hover:bg-base-100 backdrop-blur text-base-content/60 hover:text-base-content"
            :title="t('agent.resendMessage')"
          >
            <i class="fas fa-redo"></i>
          </button>
        </div>

        <!-- Touch/mobile: overflow menu -->
        <details class="dropdown dropdown-end md:hidden">
          <summary class="btn btn-xs btn-ghost bg-base-100/70 hover:bg-base-100 backdrop-blur text-base-content/60 hover:text-base-content">
            <i class="fas fa-ellipsis-h"></i>
          </summary>
          <ul class="menu dropdown-content bg-base-100 rounded-box shadow w-40 p-1 mt-1">
            <li>
              <button @click="handleCopy">
                <i :class="['fas', copySuccess ? 'fa-check text-success' : 'fa-copy']"></i>
                <span class="text-xs">{{ t('agent.copy') }}</span>
              </button>
            </li>
            <li v-if="message.type === 'user'">
              <button @click="handleResend">
                <i class="fas fa-redo"></i>
                <span class="text-xs">{{ t('agent.resend') }}</span>
              </button>
            </li>
          </ul>
        </details>
      </div>
      
      <!-- Header with type indicator -->
      <div class="message-header flex items-center gap-2 mb-2 text-sm" v-if="showHeader">
        <span class="message-type font-semibold text-base-content/70">{{ typeName }}</span>
        <span v-if="toolName" class="tool-name font-mono text-xs text-primary">`{{ toolName }}`</span>
        <!-- Tool Status Indicator -->
        <span v-if="toolStatus" :class="['status-badge px-2 py-0.5 rounded text-xs font-medium', toolStatusClass]">
          {{ toolStatusText }}
        </span>
        <span v-if="statusIcon" :class="['status-icon font-bold', statusClass]">{{ statusIcon }}</span>
        <span v-if="duration" class="duration ml-auto text-xs text-base-content/60">{{ duration }}</span>
      </div>
      
      <!-- RAG Citation Indicator -->
      <div v-if="ragInfo" class="rag-indicator flex items-center gap-2 mb-2 px-3 py-2 bg-info/10 rounded-md border border-info/30">
        <i class="fas fa-book text-info text-sm"></i>
        <span class="text-xs text-info font-medium">
          <template v-if="ragInfo.rag_sources_used">
            {{ t('agent.knowledgeBaseCited', { count: ragInfo.source_count }) }}
          </template>
          <template v-else>
            {{ t('agent.noKnowledgeBaseCitations') }}
          </template>
        </span>
      </div>
      
      <!-- Content -->
      <div class="message-content text-base-content break-words overflow-hidden">
        <div v-if="shouldHideContent" class="text-xs text-base-content/50 italic py-1 flex items-center gap-2">
          <i class="fas fa-external-link-alt"></i>
          <span>{{ t('agent.detailsInVisionPanel') }}</span>
        </div>
        <MarkdownRenderer v-else :content="formattedContent" :citations="ragInfo?.citations" />
      </div>
      
      <!-- Tool Result details (for standalone tool_result messages) -->
      <div v-if="message.type === 'tool_result' && (hasToolArgs || message.content)" class="tool-details mt-2 pt-2 border-t border-base-300">
        <button @click="toggleDetails" class="toggle-btn text-xs text-base-content/60 bg-transparent border-none cursor-pointer p-0 underline hover:text-base-content">
          {{ isExpanded ? t('agent.collapseDetails') : t('agent.expandDetails') }}
        </button>
        <div v-if="isExpanded" class="mt-2 space-y-3">
          <!-- Tool Arguments -->
          <div v-if="hasToolArgs" class="tool-args-section">
            <div class="text-xs text-base-content/60 mb-1 font-medium">📥 {{ t('agent.inputParameters') }}:</div>
            <pre class="tool-args p-2 bg-base-300 rounded text-xs font-mono overflow-x-auto text-base-content/70 max-h-48 overflow-y-auto">{{ formattedArgs }}</pre>
          </div>
          <!-- Tool Result -->
          <div v-if="message.content" class="tool-result-section">
            <div class="text-xs text-base-content/60 mb-1 font-medium">📤 {{ t('agent.executionResult') }}:</div>
            <pre class="tool-result p-2 bg-base-300 rounded text-xs font-mono overflow-x-auto text-base-content/70 max-h-64 overflow-y-auto whitespace-pre-wrap">{{ message.content }}</pre>
          </div>
          <!-- Tool Call ID -->
          <div v-if="message.metadata?.tool_call_id" class="text-xs text-base-content/50">
            {{ t('agent.toolCallId') }}: <code class="font-mono">{{ message.metadata.tool_call_id }}</code>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, nextTick, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { AgentMessage } from '@/types/agent'
import { getMessageTypeName } from '@/types/agent'
import MarkdownRenderer from './MarkdownRenderer.vue'
import ShellToolResult from './ShellToolResult.vue'

const { t } = useI18n()

const props = defineProps<{
  message: AgentMessage
  isVisionActive?: boolean
}>()

const emit = defineEmits<{
  (e: 'resend', message: AgentMessage): void
}>()

const isExpanded = ref(false)
const copySuccess = ref(false)

// Tool panel collapse states
const isToolPanelExpanded = ref(false)
const isArgsExpanded = ref(false)
const isResultExpanded = ref(false)
const argsHasOverflow = ref(false)
const resultHasOverflow = ref(false)
const argsBodyRef = ref<HTMLElement | null>(null)
const resultBodyRef = ref<HTMLElement | null>(null)

const toggleDetails = () => {
  isExpanded.value = !isExpanded.value
}

const toggleToolPanel = () => {
  isToolPanelExpanded.value = !isToolPanelExpanded.value
}

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
watch(() => props.message.metadata?.tool_args, () => {
  checkArgsOverflow()
})

watch(() => props.message.metadata?.tool_result, () => {
  checkResultOverflow()
})

// 复制消息内容
const handleCopy = async () => {
  try {
    await navigator.clipboard.writeText(props.message.content)
    copySuccess.value = true
    setTimeout(() => {
      copySuccess.value = false
    }, 2000)
  } catch (err) {
    console.error('Failed to copy:', err)
  }
}

// 重新发送消息
const handleResend = () => {
  emit('resend', props.message)
}

// Type name
const typeName = computed(() => getMessageTypeName(props.message.type))

// RAG信息
const ragInfo = computed(() => props.message.metadata?.rag_info)

// Tool name from metadata
const toolName = computed(() => props.message.metadata?.tool_name)

// Check if this is a shell tool
const isShellTool = computed(() => {
  const name = props.message.metadata?.tool_name?.toLowerCase()
  return name === 'shell' || name === 'bash' || name === 'cmd' || name === 'powershell'
})

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
      return `⏳ ${t('agent.statusRunning')}`
    case 'completed':
      return `✓ ${t('agent.statusCompleted')}`
    case 'failed':
      return `✗ ${t('agent.statusFailed')}`
    case 'pending':
      return t('agent.statusPending')
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
  return ['tool_result', 'progress'].includes(props.message.type)
})

// Tool panel styling - left border color (always orange/warning)
const toolPanelBorderClass = computed(() => {
  return 'border-l-warning' // Always use orange/warning color for tool calls
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

// Check if tool_call message has any content to display
const hasToolCallContent = computed(() => {
  if (props.message.type !== 'tool_call') return false
  
  // Has content, args, result, or call_id
  return !!(
    props.message.content ||
    hasToolArgs.value ||
    hasToolResult.value ||
    props.message.metadata?.tool_call_id
  )
})

// Check if regular message has any content to display
const hasRegularMessageContent = computed(() => {
  // tool_call messages are handled separately
  if (props.message.type === 'tool_call') return false
  
  // For tool_result, check if has content or args
  if (props.message.type === 'tool_result') {
    return !!(props.message.content || hasToolArgs.value)
  }
  
  // For all other message types, check if content is not empty
  return !!props.message.content && props.message.content.trim().length > 0
})

// Formatted args
const formattedArgs = computed(() => {
  return JSON.stringify(props.message.metadata?.tool_args, null, 2)
})

// Formatted tool result
const formattedToolResult = computed(() => {
  const result = props.message.metadata?.tool_result
  if (typeof result === 'string') {
    return result
  }
  return JSON.stringify(result, null, 2)
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

// Check if content should be hidden (Vision Explorer duplication)
const shouldHideContent = computed(() => {
  // Only apply if vision drawer is active
  if (!props.isVisionActive) return false
  
  // Check if it is a vision explorer tool message
  const toolName = props.message.metadata?.tool_name
  if (toolName === 'vision_explorer') {
    // Hide tool_result and progress messages (which are usually verbose logs)
    return ['tool_result', 'progress'].includes(props.message.type)
  }
  
  // Also check if content looks like iteration logs
  if (['tool_result', 'final'].includes(props.message.type)) {
     if (props.message.content.includes('**迭代') && props.message.content.includes('vision_explorer')) {
       return true
     }
  }
  
  return false
})
</script>

<style scoped>
.tool-args {
  word-break: break-word;
  white-space: pre-wrap;
}

.action-btn {
  min-height: 1.5rem;
  height: 1.5rem;
  padding: 0 0.5rem;
}

.action-btn i {
  font-size: 0.75rem;
}

/* Tool panel styles */
.tool-call-panel {
  transition: all 0.2s ease;
}

.tool-panel-header {
  user-select: none;
}

.tool-panel-header:active {
  transform: scale(0.99);
}

.tool-panel-content {
  animation: slideDown 0.2s ease-out;
}

@keyframes slideDown {
  from {
    opacity: 0;
    max-height: 0;
  }
  to {
    opacity: 1;
    max-height: 1000px;
  }
}

/* Scrollbar styles for tool args and result */
.tool-panel-content > div > div::-webkit-scrollbar {
  width: 8px;
}

.tool-panel-content > div > div::-webkit-scrollbar-track {
  background: transparent;
}

.tool-panel-content > div > div::-webkit-scrollbar-thumb {
  background: #424242;
  border-radius: 4px;
}

.tool-panel-content > div > div::-webkit-scrollbar-thumb:hover {
  background: #555;
}
</style>
