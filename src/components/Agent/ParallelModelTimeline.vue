<template>
  <div class="parallel-model-timeline flex h-full min-h-0 flex-col">
    <div v-if="item.error" class="mb-3 rounded border border-error/30 bg-error/10 p-3 text-sm text-error">
      {{ item.error }}
    </div>

    <details v-if="item.thinking" class="mb-3 rounded-md border border-base-300 bg-base-200/40">
      <summary class="cursor-pointer px-3 py-2 text-xs font-medium text-base-content/70">
        思考过程
      </summary>
      <div class="border-t border-base-300 px-3 py-2 text-xs whitespace-pre-wrap text-base-content/70">
        {{ item.thinking }}
      </div>
    </details>

    <div
      v-if="filteredEvents.length && useVirtualTimeline"
      ref="scrollContainerRef"
      class="min-h-0 flex-1 overflow-y-auto pr-1"
      @scroll="handleVirtualScroll"
    >
      <div class="relative" :style="{ height: `${virtualTotalHeight}px` }">
        <div
          v-for="row in visibleRows"
          :key="row.event.id"
          :ref="(element) => bindRowElement(row.event.id, element)"
          class="absolute left-0 right-0 pb-2"
          :style="{ transform: `translateY(${row.top}px)` }"
        >
          <ToolCallMessagePanel
            v-if="row.event.type === 'tool_call' || row.event.type === 'tool_result'"
            :message="eventToolMessage(row.event)"
          />
          <div
            v-else-if="row.event.type === 'text'"
            class="rounded-md border-l-[3px] border-success bg-success/5 px-3 py-2"
          >
            <MarkdownRenderer :content="row.event.content" />
          </div>
          <div v-else class="rounded-md border border-base-300 bg-base-200/30 px-3 py-2">
            <div class="mb-1 flex items-center gap-2 text-xs font-medium">
              <i :class="eventIcon(row.event.type)" class="text-base-content/60"></i>
              <span>{{ row.event.title }}</span>
              <span v-if="row.event.success === true" class="badge badge-success badge-xs ml-auto">成功</span>
              <span v-else-if="row.event.success === false" class="badge badge-error badge-xs ml-auto">失败</span>
            </div>
            <pre v-if="row.event.content" class="max-h-96 overflow-auto whitespace-pre-wrap break-words text-xs text-base-content/70">{{ row.event.content }}</pre>
          </div>
        </div>
      </div>
    </div>

    <div v-else-if="filteredEvents.length" class="min-h-0 flex-1 space-y-2 overflow-y-auto pr-1">
      <template v-for="entry in filteredEvents" :key="entry.id">
        <ToolCallMessagePanel
          v-if="entry.type === 'tool_call' || entry.type === 'tool_result'"
          :message="eventToolMessage(entry)"
        />
        <div
          v-else-if="entry.type === 'text'"
          class="rounded-md border-l-[3px] border-success bg-success/5 px-3 py-2"
        >
          <MarkdownRenderer :content="entry.content" />
        </div>
        <div v-else class="rounded-md border border-base-300 bg-base-200/30 px-3 py-2">
          <div class="mb-1 flex items-center gap-2 text-xs font-medium">
            <i :class="eventIcon(entry.type)" class="text-base-content/60"></i>
            <span>{{ entry.title }}</span>
            <span v-if="entry.success === true" class="badge badge-success badge-xs ml-auto">成功</span>
            <span v-else-if="entry.success === false" class="badge badge-error badge-xs ml-auto">失败</span>
          </div>
          <pre v-if="entry.content" class="max-h-96 overflow-auto whitespace-pre-wrap break-words text-xs text-base-content/70">{{ entry.content }}</pre>
        </div>
      </template>
    </div>

    <div v-else-if="item.content && matchesSearch(item.content)" class="min-h-0 flex-1 overflow-y-auto pr-1">
      <MarkdownRenderer :content="item.content" />
    </div>
    <div v-else-if="!item.error" class="text-sm text-base-content/50">等待输出...</div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import type { AgentMessage } from '@/types/agent'
import MarkdownRenderer from './MarkdownRenderer.vue'
import ToolCallMessagePanel from './ToolCallMessagePanel.vue'
import type {
  ParallelModelEvent,
  ParallelModelEventType,
  ParallelModelState,
} from '@/composables/agentParallelEventSupport'

const props = withDefaults(defineProps<{
  item: ParallelModelState
  filter?: 'all' | 'tools' | 'errors'
  search?: string
  virtualThreshold?: number
}>(), {
  filter: 'all',
  search: '',
  virtualThreshold: 80,
})

type VirtualRow = {
  event: ParallelModelEvent
  index: number
  top: number
  height: number
}

const ROW_GAP = 8
const OVERSCAN_PX = 720
const DEFAULT_VIEWPORT_HEIGHT = 384

const scrollContainerRef = ref<HTMLElement | null>(null)
const virtualScrollTop = ref(0)
const virtualViewportHeight = ref(DEFAULT_VIEWPORT_HEIGHT)
const heightVersion = ref(0)
const rowHeights = new Map<string, number>()
const rowElements = new Map<string, Element>()
const rowObservers = new Map<string, ResizeObserver>()
let containerObserver: ResizeObserver | null = null

const matchesSearch = (content: string) => {
  const query = props.search.trim().toLowerCase()
  if (!query) return true
  return content.toLowerCase().includes(query)
}

const filteredEvents = computed(() => {
  return (props.item.events || []).filter((event) => {
    if (props.filter === 'tools' && event.type !== 'tool_call' && event.type !== 'tool_result') return false
    if (props.filter === 'errors' && event.type !== 'error' && event.success !== false) return false
    return matchesSearch(`${event.title}\n${event.content}\n${event.toolName || ''}`)
  })
})

const useVirtualTimeline = computed(() => filteredEvents.value.length >= props.virtualThreshold)

const estimateRowHeight = (event: ParallelModelEvent): number => {
  if (event.type === 'tool_call' || event.type === 'tool_result') return 56
  if (event.type === 'text') {
    return Math.min(260, Math.max(72, 48 + Math.ceil(event.content.length / 96) * 22))
  }
  return Math.min(180, Math.max(64, 44 + Math.ceil(event.content.length / 120) * 20))
}

const virtualRows = computed<VirtualRow[]>(() => {
  heightVersion.value
  let top = 0
  return filteredEvents.value.map((event, index) => {
    const height = rowHeights.get(event.id) || estimateRowHeight(event)
    const row = {
      event,
      index,
      top,
      height,
    }
    top += height + ROW_GAP
    return row
  })
})

const virtualTotalHeight = computed(() => {
  const rows = virtualRows.value
  if (!rows.length) return 0
  const last = rows[rows.length - 1]
  return last.top + last.height
})

const visibleRows = computed(() => {
  const start = Math.max(0, virtualScrollTop.value - OVERSCAN_PX)
  const end = virtualScrollTop.value + virtualViewportHeight.value + OVERSCAN_PX
  return virtualRows.value.filter((row) => row.top + row.height >= start && row.top <= end)
})

const handleVirtualScroll = (event: Event) => {
  const target = event.currentTarget as HTMLElement
  virtualScrollTop.value = target.scrollTop
  virtualViewportHeight.value = target.clientHeight || DEFAULT_VIEWPORT_HEIGHT
}

const updateRowHeight = (id: string, element: Element) => {
  const measured = Math.ceil(element.getBoundingClientRect().height)
  if (!Number.isFinite(measured) || measured <= 0) return
  const previous = rowHeights.get(id)
  if (previous && Math.abs(previous - measured) < 2) return
  rowHeights.set(id, measured)
  heightVersion.value += 1
}

const bindRowElement = (id: string, rawElement: Element | { $el?: Element } | null) => {
  const element = rawElement instanceof Element ? rawElement : rawElement?.$el || null
  const previous = rowElements.get(id)
  if (!element) {
    rowObservers.get(id)?.disconnect()
    rowObservers.delete(id)
    rowElements.delete(id)
    return
  }
  if (previous === element) return
  rowObservers.get(id)?.disconnect()
  rowElements.set(id, element)
  updateRowHeight(id, element)
  const observer = new ResizeObserver(() => updateRowHeight(id, element))
  observer.observe(element)
  rowObservers.set(id, observer)
}

const syncViewportHeight = () => {
  const element = scrollContainerRef.value
  if (!element) return
  virtualViewportHeight.value = element.clientHeight || DEFAULT_VIEWPORT_HEIGHT
}

const scrollVirtualToBottom = async () => {
  await nextTick()
  const element = scrollContainerRef.value
  if (!element) return
  element.scrollTop = Math.max(0, element.scrollHeight - element.clientHeight)
  virtualScrollTop.value = element.scrollTop
}

onMounted(() => {
  syncViewportHeight()
  if (scrollContainerRef.value) {
    containerObserver = new ResizeObserver(syncViewportHeight)
    containerObserver.observe(scrollContainerRef.value)
  }
})

onBeforeUnmount(() => {
  containerObserver?.disconnect()
  rowObservers.forEach((observer) => observer.disconnect())
  rowObservers.clear()
  rowElements.clear()
})

watch(
  () => scrollContainerRef.value,
  (element, previous) => {
    containerObserver?.disconnect()
    containerObserver = null
    if (element) {
      syncViewportHeight()
      containerObserver = new ResizeObserver(syncViewportHeight)
      containerObserver.observe(element)
    } else if (previous) {
      virtualScrollTop.value = 0
    }
  },
)

watch(
  () => [
    filteredEvents.value.length,
    filteredEvents.value.at(-1)?.id,
    filteredEvents.value.at(-1)?.content.length,
    useVirtualTimeline.value,
  ],
  () => {
    if (!useVirtualTimeline.value) return
    const element = scrollContainerRef.value
    const nearBottom = !element || element.scrollHeight - element.scrollTop - element.clientHeight < 96
    if (nearBottom) {
      void scrollVirtualToBottom()
    }
  },
  { flush: 'post' },
)

const eventIcon = (type: ParallelModelEventType) => {
  switch (type) {
    case 'tool_call':
      return 'fas fa-wrench'
    case 'tool_result':
      return 'fas fa-check-circle'
    case 'thinking':
      return 'fas fa-brain'
    case 'error':
      return 'fas fa-triangle-exclamation text-error'
    default:
      return 'fas fa-circle-info'
  }
}

const eventToolMessage = (entry: ParallelModelEvent): AgentMessage => ({
  id: entry.id,
  type: 'tool_call',
  content: entry.title,
  timestamp: entry.timestamp,
  metadata: {
    tool_name: entry.toolName || entry.title.replace(/^.*: /, '') || 'unknown',
    tool_args: entry.toolArgs && typeof entry.toolArgs === 'object' ? entry.toolArgs : undefined,
    tool_result: entry.toolResult ?? undefined,
    tool_call_id: entry.toolCallId,
    status: entry.toolStatus || (entry.type === 'tool_result' ? 'completed' : 'running'),
    success: entry.success,
  },
})
</script>
