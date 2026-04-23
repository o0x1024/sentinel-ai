import { computed, ref } from 'vue'
import {
  clonePinnedSearchShortcutSnapshots,
  loadPinnedSearchShortcuts,
  savePinnedSearchShortcuts,
  type PinnedSearchShortcutSnapshot,
} from '@/services/pinnedSearchShortcuts'

const MAX_PINNED_SEARCH_SHORTCUT_BATCH_HISTORY = 8
const PINNED_SEARCH_SHORTCUT_BATCH_HISTORY_MAX_AGE_MS = 7 * 24 * 60 * 60 * 1000
const PINNED_SEARCH_SHORTCUT_BATCH_HISTORY_STORAGE_KEY = 'sentinel-pinned-search-shortcut-batch-history'

export type PinnedSearchShortcutBatchHistoryOperationType = 'remove' | 'tag-add' | 'tag-remove'

export interface PinnedSearchShortcutBatchHistoryEntry {
  id: string
  title: string
  summary: string
  operationType: PinnedSearchShortcutBatchHistoryOperationType
  targetLabels?: string[]
  createdAt: number
  previousSnapshots: PinnedSearchShortcutSnapshot[]
}

function isBrowser() {
  return typeof window !== 'undefined'
}

function inferBatchHistoryOperationType(
  entry: Partial<PinnedSearchShortcutBatchHistoryEntry>,
): PinnedSearchShortcutBatchHistoryOperationType {
  if (entry.operationType === 'remove' || entry.operationType === 'tag-add' || entry.operationType === 'tag-remove') {
    return entry.operationType
  }

  const title = String(entry.title || '')
  const summary = String(entry.summary || '')
  const combinedText = `${title} ${summary}`

  if (combinedText.includes('移除标签')) {
    return 'tag-remove'
  }

  if (String(entry.title || '').includes('取消固定')) {
    return 'remove'
  }

  return 'tag-add'
}

function normalizeBatchHistoryEntry(entry: PinnedSearchShortcutBatchHistoryEntry) {
  return {
    id: String(entry.id || '').trim(),
    title: String(entry.title || '').trim(),
    summary: String(entry.summary || '').trim(),
    operationType: inferBatchHistoryOperationType(entry),
    targetLabels: Array.isArray(entry.targetLabels)
      ? entry.targetLabels.map(label => String(label || '').trim()).filter(Boolean)
      : [],
    createdAt: Number(entry.createdAt || Date.now()),
    previousSnapshots: clonePinnedSearchShortcutSnapshots(entry.previousSnapshots || []),
  }
}

export function prunePinnedSearchShortcutBatchHistoryEntries(
  entries: PinnedSearchShortcutBatchHistoryEntry[],
  now: number = Date.now(),
) {
  return entries
    .map(normalizeBatchHistoryEntry)
    .filter(entry => entry.id)
    .filter(entry => now - entry.createdAt <= PINNED_SEARCH_SHORTCUT_BATCH_HISTORY_MAX_AGE_MS)
    .slice(0, MAX_PINNED_SEARCH_SHORTCUT_BATCH_HISTORY)
}

function loadPinnedSearchShortcutBatchHistoryEntries() {
  if (!isBrowser()) {
    return []
  }

  try {
    const raw = window.localStorage.getItem(PINNED_SEARCH_SHORTCUT_BATCH_HISTORY_STORAGE_KEY)
    if (!raw) {
      return []
    }

    const parsed = JSON.parse(raw)
    if (!Array.isArray(parsed)) {
      return []
    }

    return prunePinnedSearchShortcutBatchHistoryEntries(
      parsed as PinnedSearchShortcutBatchHistoryEntry[],
    )
  } catch (error) {
    console.warn('[pinnedSearchShortcutBatchHistory] Failed to load batch history:', error)
    return []
  }
}

function savePinnedSearchShortcutBatchHistoryEntries(entries: PinnedSearchShortcutBatchHistoryEntry[]) {
  if (!isBrowser()) {
    return
  }

  window.localStorage.setItem(
    PINNED_SEARCH_SHORTCUT_BATCH_HISTORY_STORAGE_KEY,
    JSON.stringify(prunePinnedSearchShortcutBatchHistoryEntries(entries)),
  )
}

const pinnedSearchShortcutBatchHistoryState = ref<PinnedSearchShortcutBatchHistoryEntry[]>(
  loadPinnedSearchShortcutBatchHistoryEntries(),
)

function createBatchHistoryId() {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 10)}`
}

export function pushPinnedSearchShortcutBatchHistoryEntry(
  entry: Omit<PinnedSearchShortcutBatchHistoryEntry, 'id' | 'createdAt'>,
) {
  const nextEntry = normalizeBatchHistoryEntry({
    id: createBatchHistoryId(),
    createdAt: Date.now(),
    ...entry,
  })

  pinnedSearchShortcutBatchHistoryState.value = [
    nextEntry,
    ...pinnedSearchShortcutBatchHistoryState.value,
  ]
  pinnedSearchShortcutBatchHistoryState.value = prunePinnedSearchShortcutBatchHistoryEntries(
    pinnedSearchShortcutBatchHistoryState.value,
  )
  savePinnedSearchShortcutBatchHistoryEntries(pinnedSearchShortcutBatchHistoryState.value)

  return nextEntry
}

export function removePinnedSearchShortcutBatchHistoryEntry(entryId: string) {
  pinnedSearchShortcutBatchHistoryState.value = pinnedSearchShortcutBatchHistoryState.value
    .filter(entry => entry.id !== entryId)
  savePinnedSearchShortcutBatchHistoryEntries(pinnedSearchShortcutBatchHistoryState.value)
}

export function clearPinnedSearchShortcutBatchHistory() {
  pinnedSearchShortcutBatchHistoryState.value = []
  savePinnedSearchShortcutBatchHistoryEntries([])
}

export function runPinnedSearchShortcutBatchHistoryUndo(entryId: string) {
  const entry = pinnedSearchShortcutBatchHistoryState.value.find(item => item.id === entryId)
  if (!entry) {
    return null
  }

  savePinnedSearchShortcuts(entry.previousSnapshots)
  const restoredSnapshots = clonePinnedSearchShortcutSnapshots(entry.previousSnapshots)
  removePinnedSearchShortcutBatchHistoryEntry(entryId)
  return restoredSnapshots
}

export function describePinnedSearchShortcutBatchTargets(
  entryIds: string[],
  snapshots: PinnedSearchShortcutSnapshot[] = loadPinnedSearchShortcuts(),
) {
  const entryIdSet = new Set(entryIds.map(entryId => String(entryId || '').trim()).filter(Boolean))
  const targets = snapshots.filter(snapshot => entryIdSet.has(snapshot.id))
  const labels = targets
    .map(snapshot => snapshot.customTitle || snapshot.title)
    .filter(Boolean)
    .slice(0, 2)

  if (targets.length === 0) {
    return `${entryIds.length} 项`
  }

  if (targets.length <= 2) {
    return labels.join('、')
  }

  return `${labels.join('、')} 等 ${targets.length} 项`
}

export function collectPinnedSearchShortcutBatchTargetLabels(
  entryIds: string[],
  snapshots: PinnedSearchShortcutSnapshot[] = loadPinnedSearchShortcuts(),
) {
  const entryIdSet = new Set(entryIds.map(entryId => String(entryId || '').trim()).filter(Boolean))
  return snapshots
    .filter(snapshot => entryIdSet.has(snapshot.id))
    .map(snapshot => snapshot.customTitle || snapshot.title)
    .filter(Boolean)
}

export function usePinnedSearchShortcutBatchHistory() {
  return {
    entries: computed(() => pinnedSearchShortcutBatchHistoryState.value),
    pushEntry: pushPinnedSearchShortcutBatchHistoryEntry,
    undoEntry: runPinnedSearchShortcutBatchHistoryUndo,
    clearEntries: clearPinnedSearchShortcutBatchHistory,
  }
}
