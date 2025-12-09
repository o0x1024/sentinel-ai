<template>
  <div class="message-flow flex flex-col gap-2 p-4 overflow-y-auto flex-1" ref="containerRef">
    <div v-for="msg in messages" :key="msg.id" class="message-wrapper animate-fadeIn">
      <MessageBlock :message="msg" />
    </div>
    
    <!-- Streaming indicator -->
    <div v-if="isStreaming" class="streaming-indicator inline-flex items-center px-4 py-2 bg-base-200 rounded-lg text-sm">
      <span class="streaming-content text-base-content whitespace-pre-wrap" v-if="streamingContent">{{ streamingContent }}</span>
      <span class="cursor text-primary ml-0.5 animate-blink">▊</span>
    </div>
    
    <!-- Empty state -->
    <div v-if="!hasMessages && !isStreaming" class="empty-state flex flex-col items-center justify-center flex-1 text-base-content/60 text-sm text-center py-8">
      <div class="avatar placeholder mb-4">
        <div class="bg-primary text-primary-content rounded-full w-16 flex items-center justify-center">
          <i class="fas fa-robot text-2xl"></i>
        </div>
      </div>
      <h3 class="text-lg font-semibold mb-2 text-base-content">Agent Ready</h3>
      <p class="max-w-xs text-base-content/70">Start a conversation to see the agent's responses and task execution.</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick, computed } from 'vue'
import type { AgentMessage } from '@/types/agent'
import MessageBlock from './MessageBlock.vue'

const props = defineProps<{
  messages: AgentMessage[]
  isStreaming?: boolean
  streamingContent?: string
}>()

const containerRef = ref<HTMLElement | null>(null)

const hasMessages = computed(() => props.messages.length > 0)

// Auto-scroll to bottom when new messages arrive
watch(
  () => props.messages.length,
  () => {
    nextTick(() => {
      scrollToBottom()
    })
  }
)

// Also scroll when streaming content updates
watch(
  () => props.streamingContent,
  () => {
    nextTick(() => {
      scrollToBottom()
    })
  }
)

const scrollToBottom = () => {
  if (containerRef.value) {
    containerRef.value.scrollTop = containerRef.value.scrollHeight
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
