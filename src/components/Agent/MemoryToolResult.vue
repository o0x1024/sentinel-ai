<template>
  <div class="memory-tool-card rounded-lg overflow-hidden border border-info/25 bg-info/5 mb-2">
    <button
      type="button"
      class="flex w-full items-center gap-3 px-4 py-3 text-left bg-info/10 hover:bg-info/15 transition-colors"
      :aria-expanded="isExpanded ? 'true' : 'false'"
      @click="toggleExpanded"
    >
      <i :class="['fas text-xs text-info transition-transform', isExpanded ? 'fa-chevron-down' : 'fa-chevron-right']"></i>
      <div class="w-8 h-8 rounded-full bg-info text-info-content flex items-center justify-center shadow-sm">
        <i class="fas fa-memory text-sm"></i>
      </div>
      <div class="flex-1 min-w-0">
        <div class="font-semibold text-sm text-info">
          {{ actionLabel }}
        </div>
        <div v-if="summaryText" class="text-xs text-base-content/70 truncate">
          {{ summaryText }}
        </div>
      </div>
      <span v-if="statusText" :class="['badge badge-sm whitespace-nowrap', statusClass]">
        {{ statusText }}
      </span>
    </button>

    <div v-show="isExpanded" class="px-4 py-3 space-y-3 border-t border-info/20">
      <div v-if="status === 'running'" class="rounded-lg border border-base-300 bg-base-100 px-3 py-2 text-sm text-base-content/75">
        {{ runningText }}
      </div>

      <div
        v-else-if="status === 'failed'"
        class="rounded-lg border border-error/20 bg-error/10 px-3 py-2 text-sm text-error whitespace-pre-wrap break-words"
      >
        {{ errorText }}
      </div>

      <template v-else-if="isRetrieveAction">
        <div class="flex flex-wrap items-center gap-2 text-xs min-w-0">
          <span class="badge badge-info badge-sm">{{ items.length }} hits</span>
          <span v-if="trace" :class="['badge badge-sm', trace.used_canonical_fallback ? 'badge-warning' : 'badge-success']">
            {{ trace.used_canonical_fallback ? 'canonical fallback' : 'hybrid retrieval' }}
          </span>
          <span v-if="trace?.include_reflection" class="badge badge-ghost badge-sm">reflection</span>
        </div>

        <div v-if="queryText" class="rounded-lg border border-base-300 bg-base-100 px-3 py-2">
          <div class="text-xs uppercase tracking-wide text-base-content/50">Query</div>
          <div class="mt-1 text-sm text-base-content break-words">{{ queryText }}</div>
        </div>

        <div v-if="trace && trace.source_breakdown.length > 0" class="rounded-lg border border-base-300 bg-base-100 px-3 py-2">
          <div class="text-xs uppercase tracking-wide text-base-content/50">Sources</div>
          <div class="mt-2 flex flex-wrap gap-2">
            <span
              v-for="entry in trace.source_breakdown"
              :key="`source:${entry.label}`"
              class="badge badge-ghost badge-sm"
            >
              {{ entry.label }}: {{ entry.count }}
            </span>
          </div>
        </div>

        <div v-if="trace && trace.kind_breakdown.length > 0" class="rounded-lg border border-base-300 bg-base-100 px-3 py-2">
          <div class="text-xs uppercase tracking-wide text-base-content/50">Kinds</div>
          <div class="mt-2 flex flex-wrap gap-2">
            <span
              v-for="entry in trace.kind_breakdown"
              :key="`kind:${entry.label}`"
              class="badge badge-ghost badge-sm"
            >
              {{ entry.label }}: {{ entry.count }}
            </span>
          </div>
        </div>

        <div v-if="items.length > 0" class="space-y-3">
          <div v-if="projectionLoading" class="rounded-lg border border-base-300 bg-base-100 px-3 py-2 text-xs text-base-content/60">
            正在加载 projection health...
          </div>
          <article
            v-for="item in items"
            :key="item.id"
            class="rounded-xl border border-base-300/70 bg-base-100 px-3 py-3"
          >
            <div class="flex flex-wrap gap-2 text-xs mb-2">
              <span class="badge badge-info badge-sm">{{ item.kind }}</span>
              <span class="badge badge-ghost badge-sm">{{ item.scope }}</span>
              <span class="badge badge-ghost badge-sm">{{ item.source }}</span>
              <span class="badge badge-ghost badge-sm">score {{ formatScore(item.score) }}</span>
            </div>
            <div class="text-sm text-base-content whitespace-pre-wrap break-words">{{ item.text }}</div>
            <div class="mt-3 flex justify-end">
              <button
                class="btn btn-xs btn-outline btn-info"
                @click="openInTools(item.id)"
              >
                在 Tools 中查看
              </button>
            </div>
            <div
              v-if="projectionStateMap.get(item.id)"
              class="mt-3 rounded-lg border border-base-300/70 bg-base-200/40 px-3 py-2"
            >
              <div class="text-[11px] uppercase tracking-wide text-base-content/50">Projection Health</div>
              <div class="mt-2 flex flex-wrap gap-2">
                <span :class="projectionBadgeClass(projectionStateMap.get(item.id)?.lexical_indexed)">lexical</span>
                <span :class="projectionBadgeClass(projectionStateMap.get(item.id)?.vector_indexed)">vector</span>
                <span :class="projectionBadgeClass(projectionStateMap.get(item.id)?.skill_projected)">skill</span>
              </div>
              <div
                v-if="projectionStateMap.get(item.id)?.last_error"
                class="mt-2 text-[11px] text-warning whitespace-pre-wrap break-words"
              >
                {{ projectionStateMap.get(item.id)?.last_error }}
              </div>
            </div>
          </article>
        </div>

        <div
          v-else-if="parsedResult?.success === true"
          class="rounded-lg border border-base-300 bg-base-100 px-3 py-2 text-sm text-base-content/70"
        >
          没有检索到匹配的 durable memory。
        </div>
      </template>

      <template v-else>
        <div class="rounded-lg border border-base-300 bg-base-100 px-3 py-2 text-sm text-base-content/75">
          {{ storeMessage }}
        </div>

        <div v-if="storeResult" class="rounded-lg border border-base-300 bg-base-100 px-3 py-2">
          <div class="text-xs uppercase tracking-wide text-base-content/50">Stored Memory</div>
          <div class="mt-2 flex flex-wrap gap-2 text-xs">
            <span class="badge badge-info badge-sm">{{ storeResult.kind }}</span>
            <span class="badge badge-ghost badge-sm">{{ storeResult.scope }}</span>
            <span class="badge badge-ghost badge-sm">{{ storeResult.source }}</span>
            <span class="badge badge-ghost badge-sm">{{ storeResult.memory_id }}</span>
          </div>
          <div class="mt-3 flex justify-end">
            <button
              class="btn btn-xs btn-outline btn-info"
              @click="openInTools(storeResult.memory_id)"
            >
              在 Tools 中查看
            </button>
          </div>
        </div>

        <div v-if="storeTitle || storeTags.length > 0" class="rounded-lg border border-base-300 bg-base-100 px-3 py-2 space-y-2">
          <div v-if="storeTitle">
            <div class="text-xs uppercase tracking-wide text-base-content/50">Title</div>
            <div class="mt-1 text-sm text-base-content break-words">{{ storeTitle }}</div>
          </div>
          <div v-if="storeTags.length > 0">
            <div class="text-xs uppercase tracking-wide text-base-content/50">Tags</div>
            <div class="mt-2 flex flex-wrap gap-2">
              <span v-for="tag in storeTags" :key="tag" class="badge badge-ghost badge-sm">
                {{ tag }}
              </span>
            </div>
          </div>
        </div>

        <div v-if="storeProjection" class="rounded-lg border border-base-300 bg-base-100 px-3 py-2">
          <div class="text-xs uppercase tracking-wide text-base-content/50">Projection Health</div>
          <div class="mt-2 flex flex-wrap gap-2">
            <span :class="projectionBadgeClass(storeProjection.lexical_indexed)">lexical</span>
            <span :class="projectionBadgeClass(storeProjection.vector_indexed)">vector</span>
            <span :class="projectionBadgeClass(storeProjection.skill_projected)">skill</span>
          </div>
          <div
            v-if="storeProjection.last_error"
            class="mt-2 text-[11px] text-warning whitespace-pre-wrap break-words"
          >
            {{ storeProjection.last_error }}
          </div>
        </div>
      </template>

      <pre
        v-if="showRawResult"
        class="rounded-lg border border-base-300 bg-base-100 px-3 py-2 text-xs text-base-content/70 whitespace-pre-wrap break-words overflow-x-auto"
      >{{ rawResultText }}</pre>
    </div>
  </div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import type { ToolStatus } from '@/types/agent'

interface MemoryTraceCount {
  label: string
  count: number
}

interface MemoryRetrievalTrace {
  query_preview?: string
  requested_top_k?: number
  hit_count?: number
  used_canonical_fallback?: boolean
  include_reflection?: boolean
  source_breakdown?: MemoryTraceCount[]
  kind_breakdown?: MemoryTraceCount[]
}

interface MemoryResultItem {
  id: string
  text: string
  kind: string
  scope: string
  source: string
  score: number
}

interface MemoryToolPayload {
  success?: boolean
  message?: string
  items?: MemoryResultItem[]
  store?: MemoryStoreResult
  trace?: MemoryRetrievalTrace
}

interface DurableMemoryProjectionState {
  memory_id: string
  lexical_indexed: boolean
  vector_indexed: boolean
  skill_projected: boolean
  last_error?: string | null
  updated_at_ms: number
}

interface MemoryStoreResult {
  memory_id: string
  title?: string | null
  kind: string
  scope: string
  stability: string
  source: string
  confidence: number
  created_at_ms: number
  projection: DurableMemoryProjectionState
}

const props = defineProps<{
  args?: Record<string, any>
  result?: unknown
  error?: string
  status?: ToolStatus
}>()
const router = useRouter()

const isExpanded = ref(false)
const projectionLoading = ref(false)
const projectionStates = ref<DurableMemoryProjectionState[]>([])
const toggleExpanded = () => {
  isExpanded.value = !isExpanded.value
}

const parseStructuredResult = (value: unknown, depth = 0): MemoryToolPayload | null => {
  if (depth > 3 || value == null) return null

  if (typeof value === 'string') {
    try {
      return parseStructuredResult(JSON.parse(value), depth + 1)
    } catch {
      return null
    }
  }

  if (Array.isArray(value)) {
    const textItem = value.find((item: any) => item?.type === 'text' && item?.text)
    if (textItem?.text) {
      return parseStructuredResult(textItem.text, depth + 1)
    }
    return null
  }

  if (typeof value !== 'object') return null

  const record = value as Record<string, unknown>
  if (typeof record.text === 'string') {
    const parsedText = parseStructuredResult(record.text, depth + 1)
    if (parsedText) return parsedText
  }

  return record as MemoryToolPayload
}

const parsedResult = computed(() => parseStructuredResult(props.result))
const status = computed<ToolStatus>(() => props.status || 'pending')
const action = computed(() => {
  const raw = String(props.args?.action || '').trim().toLowerCase()
  return raw === 'store' ? 'store' : 'retrieve'
})
const isRetrieveAction = computed(() => action.value === 'retrieve')
const trace = computed<MemoryRetrievalTrace | null>(() => parsedResult.value?.trace || null)
const storeResult = computed<MemoryStoreResult | null>(() => parsedResult.value?.store || null)
const items = computed<MemoryResultItem[]>(() =>
  Array.isArray(parsedResult.value?.items) ? parsedResult.value?.items || [] : [],
)
const itemIds = computed(() => Array.from(new Set(items.value.map((item) => item.id).filter(Boolean))))
const projectionStateMap = computed(() => {
  const map = new Map<string, DurableMemoryProjectionState>()
  for (const state of projectionStates.value) {
    map.set(state.memory_id, state)
  }
  return map
})
const queryText = computed(() => {
  const fromTrace = trace.value?.query_preview
  if (typeof fromTrace === 'string' && fromTrace.trim()) return fromTrace.trim()
  const fromArgs = props.args?.content
  return typeof fromArgs === 'string' ? fromArgs.trim() : ''
})
const storeTitle = computed(() => {
  const fromResult = storeResult.value?.title
  if (typeof fromResult === 'string' && fromResult.trim()) return fromResult.trim()
  const title = props.args?.title
  return typeof title === 'string' ? title.trim() : ''
})
const storeTags = computed<string[]>(() =>
  Array.isArray(props.args?.tags) ? props.args.tags.map((item: unknown) => String(item)) : [],
)

const actionLabel = computed(() => (isRetrieveAction.value ? 'Memory Retrieval' : 'Memory Store'))
const summaryText = computed(() => {
  if (isRetrieveAction.value) {
    if (queryText.value) return queryText.value
    return parsedResult.value?.message || '检索 durable memory'
  }
  if (storeTitle.value) return storeTitle.value
  const content = props.args?.content
  return typeof content === 'string' ? content.trim() : (parsedResult.value?.message || '')
})

const runningText = computed(() => {
  if (isRetrieveAction.value) {
    return queryText.value ? `正在检索 durable memory：${queryText.value}` : '正在检索 durable memory'
  }
  return '正在写入 durable memory'
})

const storeMessage = computed(() => parsedResult.value?.message || 'Memory 已写入 durable store。')
const storeProjection = computed<DurableMemoryProjectionState | null>(() => storeResult.value?.projection || null)

const rawResultText = computed(() => {
  if (typeof props.result === 'string') return props.result
  if (props.result === null || props.result === undefined) return ''
  try {
    return JSON.stringify(props.result, null, 2)
  } catch {
    return String(props.result)
  }
})

const showRawResult = computed(() => {
  if (status.value === 'failed') return false
  if (isRetrieveAction.value && items.value.length > 0) return false
  return false
})

const errorText = computed(() => {
  const raw = String(props.error || '').trim()
  if (raw) return raw
  if (rawResultText.value.trim()) return rawResultText.value
  return 'Memory 工具执行失败'
})

const statusText = computed(() => {
  switch (status.value) {
    case 'running':
      return isRetrieveAction.value ? '检索中' : '写入中'
    case 'completed':
      return '已完成'
    case 'failed':
      return '失败'
    case 'pending':
      return '等待中'
    default:
      return ''
  }
})

const statusClass = computed(() => {
  switch (status.value) {
    case 'running':
      return 'badge-warning'
    case 'completed':
      return 'badge-success'
    case 'failed':
      return 'badge-error'
    default:
      return 'badge-ghost'
  }
})

const formatScore = (value: unknown) => {
  const num = Number(value)
  return Number.isFinite(num) ? num.toFixed(2) : '-'
}

const projectionBadgeClass = (ok: boolean | undefined) => {
  return ok === true ? 'badge badge-success badge-sm' : 'badge badge-warning badge-sm'
}

const loadProjectionStates = async () => {
  if (!isRetrieveAction.value || itemIds.value.length === 0) {
    projectionStates.value = []
    return
  }
  projectionLoading.value = true
  try {
    const states = await invoke<DurableMemoryProjectionState[]>('get_durable_memory_projection_states', {
      memoryIds: itemIds.value,
    })
    projectionStates.value = Array.isArray(states) ? states : []
  } catch (error) {
    console.error('Failed to load memory projection states:', error)
    projectionStates.value = []
  } finally {
    projectionLoading.value = false
  }
}

const openInTools = (memoryId: string) => {
  const normalized = String(memoryId || '').trim()
  if (!normalized) return
  void router.push({
    name: 'McpTools',
    query: {
      tab: 'builtin_tools',
      memoryId: normalized,
    },
  })
}

watch(
  () => [isExpanded.value, isRetrieveAction.value, itemIds.value.join('|')],
  ([expanded, retrieve]) => {
    if (expanded && retrieve) {
      void loadProjectionStates()
    }
  },
  { immediate: true },
)
</script>
