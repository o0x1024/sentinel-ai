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
    v-for="msg in displayedMessages" 
    :key="msg.id" 
    v-memo="[msg.content, msg.metadata?.status, msg.metadata?.duration_ms, isVisionActive]"
    class="message-wrapper animate-fadeIn min-w-0"
    >
      <MessageBlock :message="msg" :is-vision-active="isVisionActive" @resend="handleResend" />
    </div>
    
    <!-- Loading indicator (waiting for response) -->
    <div v-if="isStreaming && !streamingContent" class="loading-indicator flex items-center gap-3 px-4 py-3 bg-base-200/50 rounded-lg">
      <span class="loading loading-dots loading-md text-primary"></span>
      <span class="text-sm text-base-content/70">{{ t('agent.aiIsThinking') }}</span>
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
  isStreaming?: boolean
  streamingContent?: string
  isVisionActive?: boolean
}>()

const emit = defineEmits<{
  (e: 'resend', message: AgentMessage): void
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
  // Use a threshold (e.g., 50px) to determine if we are "at the bottom"
  isUserAtBottom.value = scrollHeight - scrollTop - clientHeight < 50
}

// Throttled scroll to bottom for better performance during streaming
let scrollTimer: any = null
const throttledScrollToBottom = () => {
  if (scrollTimer) return
  scrollTimer = setTimeout(() => {
    scrollToBottom()
    scrollTimer = null
  }, 100) // 100ms throttle
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
      nextTick(() => {
        scrollToBottom()
      })
    }
  }
)

// Also scroll when streaming content updates
watch(
  () => props.streamingContent,
  () => {
    if (isUserAtBottom.value) {
      throttledScrollToBottom()
    }
  }
)

const scrollToBottom = () => {
  if (containerRef.value) {
    containerRef.value.scrollTop = containerRef.value.scrollHeight
    // We can assume we are at bottom after this
    isUserAtBottom.value = true
  }
}

// Handle resend event from MessageBlock
const handleResend = (message: AgentMessage) => {
  emit('resend', message)
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
