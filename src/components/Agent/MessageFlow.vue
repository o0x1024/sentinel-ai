<template>
  <div 
    class="message-flow flex flex-col gap-2 pl-4  overflow-y-auto flex-1 mt-2 " 
    ref="containerRef"
    @scroll="handleScroll"
  >
    <!-- Load Older Button -->
    <div v-if="hasOlderMessages" class="flex justify-center py-2">
      <button 
        @click="loadOlder" 
        class="btn btn-xs btn-ghost gap-2 text-base-content/50 hover:text-primary"
      >
        <i class="fas fa-history"></i>
        {{ t('agent.loadMoreMessages', { count: olderMessageCount }) }}
      </button>
    </div>

    <div 
      v-for="(msg, index) in displayedMessages" 
      :key="msg.id" 
      v-memo="[msg.content, msg.metadata?.status, msg.metadata?.duration_ms, isWebExplorerActive, isExecuting && index === displayedMessages.length - 1]"
      :class="['message-wrapper min-w-0', shouldAnimate(index) ? 'animate-fadeIn' : '']"
    >
      <MessageBlock 
        :message="msg" 
        :is-web-explorer-active="isWebExplorerActive" 
        :is-executing="isExecuting && index === displayedMessages.length - 1"
        @resend="handleResend"
        @edit="handleEdit"
        @heightChanged="handleHeightChanged"
        @render-html="(html: string) => emit('renderHtml', html)"
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

    <!-- Load Newer Button -->
    <div v-if="hasNewerMessages" class="flex justify-center py-2">
      <button
        @click="loadNewer"
        class="btn btn-xs btn-ghost gap-2 text-base-content/50 hover:text-primary"
      >
        <i class="fas fa-arrow-down"></i>
        {{ t('agent.jumpToLatest', '返回最新消息') }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick, computed, onMounted, onBeforeUnmount } from 'vue'
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
  (e: 'renderHtml', htmlContent: string): void
}>()

const containerRef = ref<HTMLElement | null>(null)
const isUserAtBottom = ref(true)
const isFollowing = ref(true)
const historyOffset = ref(0)
const isAutoScrolling = ref(false)
let resizeObserver: ResizeObserver | null = null

const PAGE_STEP = 50
const MAX_RENDERED_MESSAGES = 200

const hasMessages = computed(() => props.messages.length > 0)

const windowEnd = computed(() => {
  return Math.max(0, props.messages.length - historyOffset.value)
})

const windowStart = computed(() => {
  return Math.max(0, windowEnd.value - MAX_RENDERED_MESSAGES)
})

const displayedMessages = computed(() => {
  return props.messages.slice(windowStart.value, windowEnd.value)
})

const hasOlderMessages = computed(() => windowStart.value > 0)
const hasNewerMessages = computed(() => windowEnd.value < props.messages.length)
const olderMessageCount = computed(() => windowStart.value)

const shiftHistoryWindow = async (deltaOffset: number) => {
  const container = containerRef.value
  if (!container) return

  const previousScrollHeight = container.scrollHeight
  const previousScrollTop = container.scrollTop
  const maxOffset = Math.max(0, props.messages.length - MAX_RENDERED_MESSAGES)

  historyOffset.value = Math.min(maxOffset, Math.max(0, historyOffset.value + deltaOffset))
  await nextTick()

  const newScrollHeight = container.scrollHeight
  container.scrollTop = previousScrollTop + (newScrollHeight - previousScrollHeight)
}

const loadOlder = async () => {
  await shiftHistoryWindow(PAGE_STEP)
}

const loadNewer = async () => {
  await shiftHistoryWindow(-PAGE_STEP)
}

const handleScroll = () => {
  if (!containerRef.value) return
  if (isAutoScrolling.value) return

  const { scrollTop, scrollHeight, clientHeight } = containerRef.value
  const atBottom = scrollHeight - scrollTop - clientHeight < 24
  isUserAtBottom.value = atBottom
  isFollowing.value = atBottom
}

let scrollTimer: any = null
const performScrollToBottom = () => {
  if (!containerRef.value) return
  isAutoScrolling.value = true
  containerRef.value.scrollTop = containerRef.value.scrollHeight
  requestAnimationFrame(() => {
    if (containerRef.value) {
      // Apply once more after layout settles (streaming/tool block expansion).
      containerRef.value.scrollTop = containerRef.value.scrollHeight
    }
    isAutoScrolling.value = false
  })
}

const scheduleScrollToBottom = () => {
  if (scrollTimer) cancelAnimationFrame(scrollTimer)
  scrollTimer = requestAnimationFrame(() => {
    if (containerRef.value && isFollowing.value && historyOffset.value === 0) {
      performScrollToBottom()
    }
    scrollTimer = null
  })
}

watch(
  () => props.messages.length,
  (newLen, oldLen) => {
    if (newLen === 0 || (newLen > 0 && oldLen === 0)) {
      historyOffset.value = 0
      isUserAtBottom.value = true
      isFollowing.value = true
    }

    if (isFollowing.value || newLen <= 1) {
      historyOffset.value = 0
      nextTick(() => {
        scheduleScrollToBottom()
      })
    }
  }
)

watch(
  () => props.messages.length > 0 ? props.messages[props.messages.length - 1]?.id : '',
  () => {
    if (isFollowing.value && props.isExecuting && historyOffset.value === 0) {
      scheduleScrollToBottom()
    }
  }
)

watch(
  () => props.messages.length > 0 ? props.messages[props.messages.length - 1]?.content : '',
  () => {
    if (isFollowing.value && props.isExecuting && historyOffset.value === 0) {
      scheduleScrollToBottom()
    }
  }
)

watch(
  () => {
    const last = props.messages[props.messages.length - 1]
    if (!last) return ''
    const status = String(last.metadata?.status ?? '')
    const duration = String(last.metadata?.duration_ms ?? '')
    return `${last.id}:${status}:${duration}`
  },
  () => {
    if (isFollowing.value && props.isExecuting && historyOffset.value === 0) {
      scheduleScrollToBottom()
    }
  }
)

const scrollToBottom = () => {
  historyOffset.value = 0
  isUserAtBottom.value = true
  isFollowing.value = true
  nextTick(() => {
    performScrollToBottom()
  })
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
  if (isFollowing.value && historyOffset.value === 0) {
    scheduleScrollToBottom()
  }
}

const shouldAnimate = (index: number) => {
  if (historyOffset.value !== 0) return false
  return index >= displayedMessages.value.length - 8
}

// Expose scroll method
defineExpose({
  scrollToBottom,
})

onMounted(() => {
  if (!containerRef.value) return
  resizeObserver = new ResizeObserver(() => {
    if (isFollowing.value && historyOffset.value === 0) {
      scheduleScrollToBottom()
    }
  })
  resizeObserver.observe(containerRef.value)
})

onBeforeUnmount(() => {
  if (scrollTimer) {
    cancelAnimationFrame(scrollTimer)
    scrollTimer = null
  }
  if (resizeObserver) {
    resizeObserver.disconnect()
    resizeObserver = null
  }
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
