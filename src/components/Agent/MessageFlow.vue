<template>
  <div 
    class="message-flow flex flex-col gap-2 pl-4  overflow-y-auto flex-1 mt-2 " 
    ref="containerRef"
    @scroll="handleScroll"
  >
    <!-- Load More Button -->
    <div v-if="hasMoreMessages" class="flex justify-center py-2">
      <button 
        @click="loadMore" 
        class="btn btn-xs btn-ghost gap-2 text-base-content/50 hover:text-primary"
      >
        <i class="fas fa-history"></i>
        {{ t('agent.loadMoreMessages', { count: props.messages.length - displayCount }) }}
      </button>
    </div>

    <div 
    v-for="(msg, index) in displayedMessages" 
    :key="msg.id" 
    v-memo="[msg.content, msg.metadata?.status, msg.metadata?.duration_ms, isWebExplorerActive, isExecuting && index === displayedMessages.length - 1]"
    class="message-wrapper animate-fadeIn min-w-0"
    >
      <MessageBlock 
        :message="msg" 
        :is-web-explorer-active="isWebExplorerActive" 
        :is-executing="isExecuting && index === displayedMessages.length - 1"
        @resend="handleResend"
        @edit="handleEdit"
        @heightChanged="handleHeightChanged"
      />
    </div>
    
    <!-- Loading indicator (waiting for response or still working) -->
    <div v-if="isExecuting" class="loading-indicator flex items-center gap-3 px-4 py-3 bg-base-200/50 rounded-lg mr-4 mb-2">
      <span class="loading loading-dots loading-md text-primary"></span>
      <span v-if="!streamingContent" class="text-sm text-base-content/70">{{ t('agent.aiIsThinking') }}</span>
      <span v-else class="text-xs text-base-content/50 italic">{{ t('agent.statusRunning') }}</span>
    </div>
    <!-- Streaming content is now rendered as an assistant message in the message list -->
    
    <!-- Empty state -->
    <div v-if="!hasMessages && !isStreaming" class="empty-state flex  flex-col items-center justify-center flex-1 text-base-content/60 text-sm text-center py-8">
      <div class="avatar placeholder mb-4">
        <div class="bg-primary text-primary-content rounded-full w-16 flex items-center justify-center">
          <i class="fas fa-robot text-2xl"></i>
        </div>
      </div>
      <h3 class="text-lg font-semibold mb-2 text-base-content">{{ t('agent.agentReady') }}</h3>
      <p class="max-w-xs text-base-content/70">{{ t('agent.startConversation') }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { AgentMessage } from '@/types/agent'
import MessageBlock from './MessageBlock.vue'

const { t } = useI18n()

const props = defineProps<{
  messages: AgentMessage[]
  isExecuting?: boolean
  isStreaming?: boolean
  streamingContent?: string
  isWebExplorerActive?: boolean
}>()

const emit = defineEmits<{
  (e: 'resend', message: AgentMessage): void
  (e: 'edit', message: AgentMessage, newContent: string): void
}>()

const containerRef = ref<HTMLElement | null>(null)
const isUserAtBottom = ref(true)
const displayCount = ref(50)

const hasMessages = computed(() => props.messages.length > 0)

const displayedMessages = computed(() => {
  if (props.messages.length <= displayCount.value) {
    return props.messages
  }
  return props.messages.slice(-displayCount.value)
})

const hasMoreMessages = computed(() => props.messages.length > displayCount.value)

const loadMore = async () => {
  const container = containerRef.value
  if (!container) return
  
  // Save current scroll position and height
  const previousScrollHeight = container.scrollHeight
  const previousScrollTop = container.scrollTop

  // Increase display count
  displayCount.value += 50
  
  // After DOM update, adjust scroll position to maintain view
  await nextTick()
  const newScrollHeight = container.scrollHeight
  container.scrollTop = previousScrollTop + (newScrollHeight - previousScrollHeight)
}

const handleScroll = () => {
  if (!containerRef.value) return
  const { scrollTop, scrollHeight, clientHeight } = containerRef.value
  // Use a very small threshold (e.g., 5px) to determine if we are "at the bottom"
  // This makes it much more responsive to user scrolling up
  isUserAtBottom.value = scrollHeight - scrollTop - clientHeight < 5
}

// Throttled scroll to bottom for better performance during streaming
let scrollTimer: any = null
const throttledScrollToBottom = () => {
  if (scrollTimer) cancelAnimationFrame(scrollTimer)
  scrollTimer = requestAnimationFrame(() => {
    if (containerRef.value && isUserAtBottom.value) {
      containerRef.value.scrollTop = containerRef.value.scrollHeight
    }
    scrollTimer = null
  })
}

// Force scroll to bottom with multiple attempts to handle async rendering
const forceScrollToBottom = () => {
  if (!containerRef.value || !isUserAtBottom.value) return
  
  const scroll = () => {
    if (containerRef.value) {
      containerRef.value.scrollTop = containerRef.value.scrollHeight
    }
  }
  
  // Immediate scroll
  scroll()
  
  // Retry after nextTick to handle v-memo and other Vue optimizations
  nextTick(() => {
    scroll()
    // Additional retry after animation frames to handle CSS animations
    requestAnimationFrame(() => {
      scroll()
      // Final retry after a short delay to catch any async updates
      setTimeout(scroll, 50)
    })
  })
}

// Auto-scroll to bottom when new messages arrive
watch(
  () => props.messages.length,
  (newLen, oldLen) => {
    // Reset display count if it's a new conversation
    if (newLen === 0 || (newLen > 0 && oldLen === 0)) {
      displayCount.value = 50
    }

    // Scroll if user is at bottom or if it's a new conversation start
    if (isUserAtBottom.value || props.messages.length <= 1) {
      forceScrollToBottom()
    }
  }
)

// Watch for content changes in the last message (streaming updates)
watch(
  () => props.messages.length > 0 ? props.messages[props.messages.length - 1]?.content : '',
  () => {
    if (isUserAtBottom.value && props.isExecuting) {
      throttledScrollToBottom()
    }
  }
)

// Also scroll when streaming content updates
watch(
  () => props.streamingContent,
  () => {
    if (isUserAtBottom.value) {
      // For streaming, we want to stay at bottom
      throttledScrollToBottom()
    }
  }
)

// Watch for metadata changes (tool status, duration, etc.)
watch(
  () => props.messages.length > 0 ? props.messages[props.messages.length - 1]?.metadata : null,
  () => {
    if (isUserAtBottom.value && props.isExecuting) {
      // Use throttled scroll for metadata updates to avoid excessive scrolling
      throttledScrollToBottom()
    }
  },
  { deep: true }
)

const scrollToBottom = () => {
  if (containerRef.value) {
    // Directly set scrollTop for maximum reliability
    containerRef.value.scrollTop = containerRef.value.scrollHeight
    isUserAtBottom.value = true
    
    // Double-check after a brief delay to handle any async rendering
    setTimeout(() => {
      if (containerRef.value && isUserAtBottom.value) {
        containerRef.value.scrollTop = containerRef.value.scrollHeight
      }
    }, 100)
  }
}

// Handle resend event from MessageBlock
const handleResend = (message: AgentMessage) => {
  emit('resend', message)
}

// Handle edit event from MessageBlock
const handleEdit = (message: AgentMessage, newContent: string) => {
  emit('edit', message, newContent)
}

// Handle height change from MessageBlock (when tool panels expand/collapse)
const handleHeightChanged = () => {
  if (isUserAtBottom.value) {
    throttledScrollToBottom()
  }
}

// Expose scroll method
defineExpose({
  scrollToBottom,
})
</script>

<style scoped>
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

.message-flow {
  /* Prevent browser from adjusting scroll position automatically, 
     we handle it manually for better UX during streaming */
  overflow-anchor: none;
}

.animate-fadeIn {
  animation: fadeIn 0.2s ease-out;
}

@keyframes blink {
  0%, 50% { opacity: 1; }
  51%, 100% { opacity: 0; }
}

.animate-blink {
  animation: blink 1s infinite;
}
</style>
