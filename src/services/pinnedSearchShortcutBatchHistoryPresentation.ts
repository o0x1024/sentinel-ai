import type {
  PinnedSearchShortcutBatchHistoryEntry,
  PinnedSearchShortcutBatchHistoryOperationType,
} from '@/services/pinnedSearchShortcutBatchHistory'

export type PinnedSearchShortcutBatchHistoryFilter = 'all' | PinnedSearchShortcutBatchHistoryOperationType

export interface PinnedSearchShortcutBatchHistoryFilterOption {
  value: PinnedSearchShortcutBatchHistoryFilter
  label: string
  count: number
}

const normalizeBatchHistorySearchText = (value: string) =>
  String(value || '')
    .toLowerCase()
    .trim()
    .replace(/\s+/g, ' ')

export interface PinnedSearchShortcutBatchHistoryGroup {
  key: 'today' | 'earlier'
  label: string
  entries: PinnedSearchShortcutBatchHistoryEntry[]
}

function getStartOfToday(now: number) {
  const date = new Date(now)
  date.setHours(0, 0, 0, 0)
  return date.getTime()
}

function getBatchHistoryOperationLabel(type: PinnedSearchShortcutBatchHistoryOperationType) {
  switch (type) {
    case 'remove':
      return '取消固定'
    case 'tag-add':
      return '追加标签'
    case 'tag-remove':
      return '移除标签'
  }
}

export function collectPinnedSearchShortcutBatchHistoryFilterOptions(
  entries: PinnedSearchShortcutBatchHistoryEntry[],
) {
  const operationTypes: PinnedSearchShortcutBatchHistoryOperationType[] = ['remove', 'tag-add', 'tag-remove']
  const options: PinnedSearchShortcutBatchHistoryFilterOption[] = [
    {
      value: 'all',
      label: '全部',
      count: entries.length,
    },
  ]

  operationTypes.forEach((operationType) => {
    const count = entries.filter(entry => entry.operationType === operationType).length
    if (count === 0) {
      return
    }

    options.push({
      value: operationType,
      label: getBatchHistoryOperationLabel(operationType),
      count,
    })
  })

  return options
}

export function filterPinnedSearchShortcutBatchHistoryEntries(
  entries: PinnedSearchShortcutBatchHistoryEntry[],
  filter: PinnedSearchShortcutBatchHistoryFilter,
) {
  if (filter === 'all') {
    return entries
  }

  return entries.filter(entry => entry.operationType === filter)
}

export function searchPinnedSearchShortcutBatchHistoryEntries(
  entries: PinnedSearchShortcutBatchHistoryEntry[],
  query: string,
) {
  const normalizedQuery = normalizeBatchHistorySearchText(query)
  if (!normalizedQuery) {
    return entries
  }

  return entries.filter((entry) => {
    const haystack = normalizeBatchHistorySearchText([
      entry.title,
      entry.summary,
      ...(entry.targetLabels || []),
    ].join(' '))

    return haystack.includes(normalizedQuery)
  })
}

export function groupPinnedSearchShortcutBatchHistoryEntries(
  entries: PinnedSearchShortcutBatchHistoryEntry[],
  now: number = Date.now(),
) {
  const startOfToday = getStartOfToday(now)
  const groups: PinnedSearchShortcutBatchHistoryGroup[] = [
    {
      key: 'today',
      label: '今天',
      entries: entries.filter(entry => entry.createdAt >= startOfToday),
    },
    {
      key: 'earlier',
      label: '更早',
      entries: entries.filter(entry => entry.createdAt < startOfToday),
    },
  ]

  return groups.filter(group => group.entries.length > 0)
}
