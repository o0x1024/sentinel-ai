<template>
  <div 
    class="message-flow flex-1 overflow-y-auto pl-4 mt-2" 
    ref="containerRef"
    @scroll="handleScroll"
  >
    <div ref="contentRef" class="message-flow-content flex min-h-full flex-col gap-2 pr-4">
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
        v-memo="[
          msg.content,
          msg.metadata?.status,
          msg.metadata?.duration_ms,
          isExecuting && index === displayedMessages.length - 1,
          focusedMessageId === msg.id,
          animatedFocusMessageId === msg.id,
        ]"
        :id="`agent-message-${msg.id}`"
        :class="[
          'message-wrapper min-w-0 rounded-lg transition-all duration-300',
          shouldAnimate(index) ? 'animate-fadeIn' : '',
          msg.id === focusedMessageId ? 'ring-2 ring-info/40 bg-info/5 px-2 py-1 mr-4' : '',
          msg.id === animatedFocusMessageId ? 'animate-memory-focus-pulse' : '',
        ]"
      >
        <MessageBlock 
          :message="msg" 
          :is-executing="isExecuting && index === displayedMessages.length - 1"
          @focus-team-task="(taskId: string) => emit('focusTeamTask', taskId)"
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
  focusedMessageId?: string | null
}>()

const emit = defineEmits<{
  (e: 'resend', message: AgentMessage): void
  (e: 'edit', message: AgentMessage, newContent: string): void
  (e: 'focusTeamTask', taskId: string): void
  (e: 'renderHtml', htmlContent: string): void
  (e: 'message-focused', messageId: string): void
}>()

const containerRef = ref<HTMLElement | null>(null)
const contentRef = ref<HTMLElement | null>(null)
const isUserAtBottom = ref(true)
const isFollowing = ref(true)
const historyOffset = ref(0)
const isAutoScrolling = ref(false)
const animatedFocusMessageId = ref('')
let viewportResizeObserver: ResizeObserver | null = null
let contentResizeObserver: ResizeObserver | null = null
let focusPulseTimer: ReturnType<typeof setTimeout> | null = null

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
const focusedMessageId = computed(() => String(props.focusedMessageId || '').trim())

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

const ensureMessageVisible = async (messageId: string) => {
  const normalized = String(messageId || '').trim()
  if (!normalized) return false

  const targetIndex = props.messages.findIndex((message) => message.id === normalized)
  if (targetIndex < 0) return false

  if (targetIndex >= windowStart.value && targetIndex < windowEnd.value) {
    return true
  }

  const maxStart = Math.max(0, props.messages.length - MAX_RENDERED_MESSAGES)
  const desiredStart = Math.max(0, Math.min(targetIndex - 20, maxStart))
  const desiredEnd = Math.min(props.messages.length, Math.max(desiredStart + MAX_RENDERED_MESSAGES, targetIndex + 1))
  historyOffset.value = Math.max(0, props.messages.length - desiredEnd)
  await nextTick()
  return true
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

const focusMessage = async (messageId: string) => {
  const normalized = String(messageId || '').trim()
  if (!normalized) return false

  const visible = await ensureMessageVisible(normalized)
  if (!visible) return false

  await nextTick()
  const target = document.getElementById(`agent-message-${normalized}`)
  if (!target) return false

  isFollowing.value = false
  target.scrollIntoView({ behavior: 'smooth', block: 'center' })
  animatedFocusMessageId.value = normalized
  if (focusPulseTimer) {
    clearTimeout(focusPulseTimer)
  }
  focusPulseTimer = setTimeout(() => {
    if (animatedFocusMessageId.value === normalized) {
      animatedFocusMessageId.value = ''
    }
    focusPulseTimer = null
  }, 1600)
  emit('message-focused', normalized)
  return true
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
  focusMessage,
})

watch(
  () => ({
    messageId: focusedMessageId.value,
    messageCount: props.messages.length,
  }),
  ({ messageId }) => {
    if (!messageId) return
    void focusMessage(messageId)
  },
  { immediate: true },
)

onMounted(() => {
  if (containerRef.value) {
    viewportResizeObserver = new ResizeObserver(() => {
      if (isFollowing.value && historyOffset.value === 0) {
        scheduleScrollToBottom()
      }
    })
    viewportResizeObserver.observe(containerRef.value)
  }

  if (contentRef.value) {
    contentResizeObserver = new ResizeObserver(() => {
      if (isFollowing.value && historyOffset.value === 0) {
        scheduleScrollToBottom()
      }
    })
    contentResizeObserver.observe(contentRef.value)
  }

  if (isFollowing.value && historyOffset.value === 0) {
    nextTick(() => {
      if (isFollowing.value && historyOffset.value === 0) {
        scheduleScrollToBottom()
      }
    })
  }
})

onBeforeUnmount(() => {
  if (scrollTimer) {
    cancelAnimationFrame(scrollTimer)
    scrollTimer = null
  }
  if (focusPulseTimer) {
    clearTimeout(focusPulseTimer)
    focusPulseTimer = null
  }
  if (viewportResizeObserver) {
    viewportResizeObserver.disconnect()
    viewportResizeObserver = null
  }
  if (contentResizeObserver) {
    contentResizeObserver.disconnect()
    contentResizeObserver = null
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
  scroll-padding-bottom: calc(1rem + env(safe-area-inset-bottom, 0px));
}

.message-flow-content {
  padding-bottom: calc(1rem + env(safe-area-inset-bottom, 0px));
}

.animate-fadeIn {
  animation: fadeIn 0.2s ease-out;
}

@keyframes memoryFocusPulse {
  0% {
    box-shadow: 0 0 0 0 hsl(var(--in) / 0.42);
    transform: translateY(0);
  }
  35% {
    box-shadow: 0 0 0 12px hsl(var(--in) / 0.08);
    transform: translateY(-1px);
  }
  100% {
    box-shadow: 0 0 0 0 hsl(var(--in) / 0);
    transform: translateY(0);
  }
}

.animate-memory-focus-pulse {
  animation: memoryFocusPulse 1.6s ease-out;
}

@keyframes blink {
  0%, 50% { opacity: 1; }
  51%, 100% { opacity: 0; }
}

.animate-blink {
  animation: blink 1s infinite;
}
</style>
