<template>
  <div class="web-search-card rounded-lg overflow-hidden border border-info/25 bg-info/5 mb-2">
    <button
      type="button"
      class="flex w-full items-center gap-3 px-4 py-3 text-left bg-info/10 hover:bg-info/15 transition-colors"
      :aria-expanded="isExpanded ? 'true' : 'false'"
      @click="toggleExpanded"
    >
      <i :class="['fas text-xs text-info transition-transform', isExpanded ? 'fa-chevron-down' : 'fa-chevron-right']"></i>
      <div class="w-8 h-8 rounded-full bg-info text-info-content flex items-center justify-center shadow-sm">
        <i class="fas fa-globe text-sm"></i>
      </div>
      <div class="flex-1 min-w-0">
        <div class="font-semibold text-sm text-info">联网搜索</div>
        <div v-if="queryText" class="text-xs text-base-content/70 truncate">
          {{ queryText }}
        </div>
      </div>
      <span v-if="statusText" :class="['badge badge-sm whitespace-nowrap', statusClass]">
        {{ statusText }}
      </span>
    </button>

    <div v-show="isExpanded" class="px-4 py-3 space-y-3 border-t border-info/20">
      <div v-if="status === 'running'" class="text-sm text-base-content/70">
        正在搜索{{ queryText ? `：${queryText}` : '' }}
      </div>

      <template v-else>
        <div class="flex flex-wrap items-center gap-2 text-xs text-base-content/60 min-w-0">
          <span v-if="status === 'failed'" class="badge badge-error badge-sm">错误</span>
          <template v-else-if="results.length > 0">
            <span v-if="sourceText" class="badge badge-ghost badge-sm">{{ sourceText }}</span>
            <span class="badge badge-info badge-sm">{{ results.length }} 条结果</span>
            <span v-if="typeof totalResults === 'number'" class="opacity-70">总计 {{ totalResults }}</span>
          </template>
        </div>

        <div
          v-if="status === 'failed'"
          class="rounded-lg border border-error/20 bg-error/10 px-3 py-2 text-sm text-error whitespace-pre-wrap break-words"
        >
          {{ errorText }}
        </div>

        <template v-else-if="results.length > 0">
          <div class="space-y-3">
            <article
              v-for="(item, index) in results"
              :key="`${item.url || item.title || 'result'}:${index}`"
              class="rounded-xl border border-base-300/70 bg-base-100 px-3 py-3"
            >
              <div class="flex items-start gap-2">
                <span class="mt-0.5 text-xs font-mono text-info">{{ index + 1 }}.</span>
                <div class="min-w-0 flex-1">
                  <a
                    v-if="item.url"
                    :href="item.url"
                    target="_blank"
                    rel="noreferrer"
                    class="block font-medium text-sm text-primary break-words hover:underline"
                  >
                    {{ item.title || item.url }}
                  </a>
                  <div v-else class="font-medium text-sm text-base-content break-words">
                    {{ item.title || '搜索结果' }}
                  </div>
                  <div v-if="item.url" class="mt-1 text-xs text-base-content/50 break-all">
                    {{ item.url }}
                  </div>
                  <p v-if="item.content" class="mt-2 text-sm text-base-content/75 whitespace-pre-wrap break-words">
                    {{ item.content }}
                  </p>
                </div>
              </div>
            </article>
          </div>
        </template>

        <pre
          v-else-if="rawResultText"
          class="rounded-lg border border-base-300 bg-base-100 px-3 py-2 text-xs text-base-content/75 whitespace-pre-wrap break-words overflow-x-auto"
        >{{ rawResultText }}</pre>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import type { ToolStatus } from '@/types/agent'

interface WebSearchResultItem {
  title?: string
  url?: string
  content?: string
}

interface WebSearchPayload {
  query?: string
  results?: WebSearchResultItem[]
  total_results?: number
  source?: string
}

const props = defineProps<{
  args?: Record<string, any>
  result?: unknown
  error?: string
  status?: ToolStatus
}>()

const isExpanded = ref(false)
const toggleExpanded = () => {
  isExpanded.value = !isExpanded.value
}

const parseStructuredResult = (value: unknown, depth = 0): WebSearchPayload | null => {
  if (depth > 3 || value === null || value === undefined) return null

  if (typeof value === 'string') {
    try {
      return parseStructuredResult(JSON.parse(value), depth + 1)
    } catch {
      return null
    }
  }

  if (Array.isArray(value)) {
    return value.length > 0 ? parseStructuredResult(value[0], depth + 1) : null
  }

  if (typeof value !== 'object') return null

  const record = value as Record<string, unknown>
  if (typeof record.text === 'string') {
    const parsedText = parseStructuredResult(record.text, depth + 1)
    if (parsedText) return parsedText
  }

  return record as WebSearchPayload
}

const parsedResult = computed(() => parseStructuredResult(props.result))

const queryText = computed(() => {
  const fromResult = parsedResult.value?.query
  if (typeof fromResult === 'string' && fromResult.trim().length > 0) return fromResult.trim()
  const fromArgs = props.args?.query
  return typeof fromArgs === 'string' ? fromArgs.trim() : ''
})

const results = computed<WebSearchResultItem[]>(() => {
  const items = parsedResult.value?.results
  if (!Array.isArray(items)) return []
  return items.map((item) => ({
    title: typeof item?.title === 'string' ? item.title : '',
    url: typeof item?.url === 'string' ? item.url : '',
    content: typeof item?.content === 'string' ? item.content : '',
  }))
})

const totalResults = computed(() => {
  const value = parsedResult.value?.total_results
  return typeof value === 'number' && Number.isFinite(value) ? value : null
})

const sourceText = computed(() => {
  const value = parsedResult.value?.source
  return typeof value === 'string' ? value : ''
})

const rawResultText = computed(() => {
  if (typeof props.result === 'string') return props.result
  if (props.result === null || props.result === undefined) return ''
  try {
    return JSON.stringify(props.result, null, 2)
  } catch {
    return String(props.result)
  }
})

const errorText = computed(() => {
  if (props.error && props.error.trim().length > 0) return props.error.trim()
  if (rawResultText.value.trim().length > 0) return rawResultText.value
  return '联网搜索失败'
})

const status = computed<ToolStatus>(() => props.status || 'pending')

const statusText = computed(() => {
  switch (status.value) {
    case 'running':
      return '搜索中'
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
</script>
