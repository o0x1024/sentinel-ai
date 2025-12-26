<template>
  <div 
    class="message-flow flex flex-col gap-2 pl-4  overflow-y-auto flex-1 mt-2 " 
    ref="containerRef"
    @scroll="handleScroll"
  >
    <div v-for="msg in messages" :key="msg.id" class="message-wrapper animate-fadeIn min-w-0">
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

const hasMessages = computed(() => props.messages.length > 0)

const handleScroll = () => {
  if (!containerRef.value) return
  const { scrollTop, scrollHeight, clientHeight } = containerRef.value
  // Use a threshold (e.g., 50px) to determine if we are "at the bottom"
  isUserAtBottom.value = scrollHeight - scrollTop - clientHeight < 50
}

// Auto-scroll to bottom when new messages arrive
watch(
  () => props.messages.length,
  () => {
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
      nextTick(() => {
        scrollToBottom()
      })
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
