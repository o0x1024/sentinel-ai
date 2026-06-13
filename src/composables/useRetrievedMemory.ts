import { computed, ref, watch, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

import type { ContextUsageInfo, MemoryRetrievalInfo } from '@/composables/useAgentEventTypes'

export interface RetrievedMemoryItemView {
  id: string
  title?: string | null
  text: string
  kind: string
  scope: string
  stability: string
  source: string
  confidence: number
  importance: number
  createdAtMs: number
  score?: number
  autoInjectEnabled?: boolean
}

export interface DurableMemoryDiagnosticsItem {
  record: {
    id: string
    title?: string | null
    text: string
    kind: string
    scope: string
    source: string
    status: string
    tags_json: string
    origin_execution_id?: string | null
    updated_at_ms: number
  }
  projection?: {
    lexical_indexed: boolean
    vector_indexed: boolean
    skill_projected: boolean
    last_error?: string | null
  } | null
  retrievable_projection_ready: boolean
  projection_issue: boolean
}

const NO_AUTO_INJECT_TAG = 'no_auto_inject'

export function parseMemoryTags(raw: string | undefined): string[] {
  if (!raw) return []
  try {
    const parsed = JSON.parse(raw)
    return Array.isArray(parsed) ? parsed.filter(item => typeof item === 'string') : []
  } catch {
    return []
  }
}

export function memoryAutoInjectEnabled(record: DurableMemoryDiagnosticsItem['record']): boolean {
  return (
    record.status === 'active' &&
    !parseMemoryTags(record.tags_json).includes(NO_AUTO_INJECT_TAG)
  )
}

export function mapDiagnosticsToItemView(
  item: DurableMemoryDiagnosticsItem,
  score?: number,
): RetrievedMemoryItemView {
  return {
    id: item.record.id,
    title: item.record.title,
    text: item.record.text,
    kind: item.record.kind,
    scope: item.record.scope,
    stability: 'stable',
    source: item.record.source,
    confidence: 0.7,
    importance: 3,
    createdAtMs: item.record.updated_at_ms,
    score,
    autoInjectEnabled: memoryAutoInjectEnabled(item.record),
  }
}

export function useRetrievedMemoryPanel(contextUsage: Ref<ContextUsageInfo | null>) {
  const items = ref<RetrievedMemoryItemView[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const expanded = ref(true)

  const memoryRetrieval = computed<MemoryRetrievalInfo | null>(
    () => contextUsage.value?.memoryRetrieval ?? null,
  )
  const retrievalIds = computed(() => contextUsage.value?.retrievalIds ?? [])
  const retrievalTokens = computed(() => contextUsage.value?.retrievalTokens ?? 0)
  const hasRetrievedMemory = computed(
    () => (memoryRetrieval.value?.hitCount ?? 0) > 0 || retrievalIds.value.length > 0,
  )

  const loadItems = async () => {
    const ids = retrievalIds.value
    if (ids.length === 0) {
      items.value = []
      return
    }

    loading.value = true
    error.value = null
    try {
      const diagnostics = await invoke<DurableMemoryDiagnosticsItem[]>(
        'get_durable_memory_diagnostics_by_ids',
        { memoryIds: ids },
      )
      const byId = new Map(diagnostics.map(item => [item.record.id, item]))
      items.value = ids
        .map(id => byId.get(id))
        .filter((item): item is DurableMemoryDiagnosticsItem => Boolean(item))
        .map(item => mapDiagnosticsToItemView(item))
    } catch (loadError) {
      error.value = loadError instanceof Error ? loadError.message : String(loadError)
      items.value = []
    } finally {
      loading.value = false
    }
  }

  watch(
    retrievalIds,
    () => {
      void loadItems()
    },
    { immediate: true },
  )

  return {
    items,
    loading,
    error,
    expanded,
    memoryRetrieval,
    retrievalIds,
    retrievalTokens,
    hasRetrievedMemory,
    reload: loadItems,
  }
}

export async function updateDurableMemory(payload: {
  memoryId: string
  text: string
  title?: string
  kind?: string
  tags?: string[]
}) {
  return invoke('update_durable_memory_command', {
    memoryId: payload.memoryId,
    text: payload.text,
    title: payload.title ?? null,
    kind: payload.kind ?? null,
    tags: payload.tags ?? null,
  })
}

export async function deleteDurableMemory(memoryId: string) {
  return invoke('delete_durable_memory_command', { memoryId })
}

export async function setDurableMemoryAutoInject(memoryId: string, enabled: boolean) {
  return invoke('set_durable_memory_auto_inject_command', { memoryId, enabled })
}

export interface MemoryPreviewItem {
  id: string
  text: string
  kind: string
  scope: string
  source: string
  score: number
  importance: number
}

export interface MemoryPreviewResult {
  hits: MemoryPreviewItem[]
  trace: {
    query_preview: string
    requested_top_k: number
    hit_count: number
    used_canonical_fallback: boolean
    source_breakdown: Array<{ label: string; count: number }>
    kind_breakdown: Array<{ label: string; count: number }>
  }
}

export async function previewMemoryRetrieval(query: string, topK = 8): Promise<MemoryPreviewResult> {
  const result = await invoke<{
    hits: Array<{
      id: string
      text: string
      kind: string
      scope: string
      source: string
      score: number
      importance: number
    }>
    trace: MemoryPreviewResult['trace']
  }>('preview_memory_retrieval_command', { query, topK })

  return {
    hits: result.hits,
    trace: result.trace,
  }
}
