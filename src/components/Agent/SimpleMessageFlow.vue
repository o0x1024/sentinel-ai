<template>
  <div 
    class="simple-message-flow flex flex-col gap-2 overflow-y-auto flex-1" 
    ref="containerRef"
    @scroll="handleScroll"
  >
    <!-- Loading state -->
    <div v-if="isLoading" class="flex items-center justify-center py-8">
      <span class="loading loading-spinner loading-md text-primary"></span>
      <span class="ml-2 text-sm text-base-content/60">{{ t('agent.subagentDetail.loadingMessages') }}</span>
    </div>

    <!-- Empty state -->
    <div v-else-if="messages.length === 0" class="flex flex-col items-center justify-center flex-1 py-12 text-base-content/50">
      <i class="fas fa-comments text-4xl mb-3 opacity-30"></i>
      <p class="text-sm">{{ t('agent.subagentDetail.noMessages') }}</p>
    </div>

    <!-- Message list -->
    <template v-else>
      <div
        v-for="msg in messages"
        :key="msg.id"
        class="message-item animate-fadeIn"
      >
        <!-- User message -->
        <template v-if="msg.role === 'user'">
          <div class="flex items-start gap-3">
            <div class="w-8 h-8 rounded-lg bg-primary/20 flex items-center justify-center flex-shrink-0">
              <i class="fas fa-user text-primary text-sm"></i>
            </div>
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2 mb-1">
                <span class="text-sm font-medium text-base-content">{{ t('agent.user') }}</span>
                <span class="text-xs text-base-content/50">{{ formatMessageTime(msg.timestamp) }}</span>
              </div>
              <div class="bg-base-200 rounded-lg rounded-tl-none p-3 text-sm break-words text-base-content">
                <MarkdownRenderer :content="getDisplayContent(msg)" />
              </div>
            </div>
          </div>
        </template>

        <!-- Assistant message -->
        <template v-else-if="msg.role === 'assistant'">
          <div class="flex items-start gap-3">
            <div class="w-8 h-8 rounded-lg bg-success/20 flex items-center justify-center flex-shrink-0">
              <i class="fas fa-robot text-success text-sm"></i>
            </div>
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2 mb-1">
                <span class="text-sm font-medium text-base-content">{{ t('agent.assistant') }}</span>
                <span class="text-xs text-base-content/50">{{ formatMessageTime(msg.timestamp) }}</span>
              </div>
              <div class="bg-base-100 border border-base-300 rounded-lg rounded-tl-none p-3 text-sm break-words text-base-content">
                <MarkdownRenderer :content="getDisplayContent(msg)" />
              </div>
              <!-- Tool calls -->
              <div v-if="msg.tool_calls" class="mt-2">
                <ToolCallsDisplay :tool-calls="msg.tool_calls" />
              </div>
            </div>
          </div>
        </template>

        <!-- Tool message -->
        <template v-else-if="msg.role === 'tool'">
          <div class="flex items-start gap-3 ml-11">
            <div class="w-6 h-6 rounded bg-warning/20 flex items-center justify-center flex-shrink-0">
              <i class="fas fa-wrench text-warning text-xs"></i>
            </div>
            <div class="flex-1 min-w-0">
              <div class="text-xs text-base-content/50 mb-1">
                {{ t('agent.toolResult') }}
              </div>
              <div class="bg-warning/5 border border-warning/20 rounded-lg p-2 text-xs font-mono whitespace-pre-wrap break-words max-h-40 overflow-y-auto text-base-content">
                {{ formatToolResult(msg.content) }}
              </div>
            </div>
          </div>
        </template>

        <!-- System message -->
        <template v-else>
          <div class="flex items-center justify-center">
            <div class="bg-base-200/50 rounded-full px-4 py-1 text-xs text-base-content/60">
              {{ msg.content || msg.role }}
            </div>
          </div>
        </template>
      </div>
    </template>

    <!-- Streaming indicator -->
    <div v-if="isStreaming" class="flex items-center gap-2 px-4 py-2">
      <span class="loading loading-dots loading-sm text-primary"></span>
      <span class="text-sm text-base-content/60">{{ t('agent.aiIsThinking') }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick, defineAsyncComponent } from 'vue'
import { useI18n } from 'vue-i18n'
import MarkdownRenderer from './MarkdownRenderer.vue'

// Async load to avoid circular dependency
const ToolCallsDisplay = defineAsyncComponent(() => import('./ToolCallsDisplay.vue'))

export interface SimpleMessage {
  id: string
  role: 'user' | 'assistant' | 'tool' | 'system'
  content?: string | null
  reasoning_content?: string | null
  tool_calls?: string | null
  timestamp: string
  metadata?: any
}

const props = defineProps<{
  messages: SimpleMessage[]
  isLoading?: boolean
  isStreaming?: boolean
}>()

const { t } = useI18n()

const containerRef = ref<HTMLElement | null>(null)
const isUserAtBottom = ref(true)

// Get display content from message
const getDisplayContent = (msg: SimpleMessage): string => {
  if (msg.content && msg.content.trim().length > 0) {
    return msg.content.trim()
  }
  if (msg.reasoning_content && msg.reasoning_content.trim().length > 0) {
    return msg.reasoning_content.trim()
  }
  return '-'
}

// Format tool result
const formatToolResult = (content?: string | null) => {
  if (!content) return '-'
  try {
    const parsed = JSON.parse(content)
    return JSON.stringify(parsed, null, 2)
  } catch {
    return content.length > 500 ? content.slice(0, 500) + '...' : content
  }
}

// Format message time
const formatMessageTime = (timestamp: string) => {
  const date = new Date(timestamp)
  if (Number.isNaN(date.getTime())) return ''
  return date.toLocaleTimeString()
}

// Handle scroll
const handleScroll = () => {
  if (!containerRef.value) return
  const { scrollTop, scrollHeight, clientHeight } = containerRef.value
  isUserAtBottom.value = scrollHeight - scrollTop - clientHeight < 10
}

// Scroll to bottom
const scrollToBottom = () => {
  if (containerRef.value) {
    containerRef.value.scrollTop = containerRef.value.scrollHeight
    isUserAtBottom.value = true
  }
}

// Auto-scroll on new messages
watch(
  () => props.messages.length,
  () => {
    if (isUserAtBottom.value) {
      nextTick(scrollToBottom)
    }
  }
)

// Expose scroll method
defineExpose({
  scrollToBottom,
})
</script>

<style scoped>
.message-item {
  animation: fadeIn 0.2s ease-out;
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.simple-message-flow {
  overflow-anchor: none;
}
</style>
