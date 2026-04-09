import { describe, expect, it } from 'vitest'
import {
  collectPinnedSearchShortcutBatchHistoryFilterOptions,
  filterPinnedSearchShortcutBatchHistoryEntries,
  groupPinnedSearchShortcutBatchHistoryEntries,
  searchPinnedSearchShortcutBatchHistoryEntries,
} from '@/services/pinnedSearchShortcutBatchHistoryPresentation'
import type { PinnedSearchShortcutBatchHistoryEntry } from '@/services/pinnedSearchShortcutBatchHistory'

const now = new Date('2026-04-10T12:00:00+08:00').getTime()

const entries: PinnedSearchShortcutBatchHistoryEntry[] = [
  {
    id: 'today-entry',
    title: '批量标签',
    summary: '更新 2 项',
    operationType: 'tag-add',
    targetLabels: ['仪表盘'],
    createdAt: now - 60_000,
    previousSnapshots: [],
  },
  {
    id: 'earlier-entry',
    title: '批量取消固定',
    summary: '移除 1 项',
    operationType: 'remove',
    targetLabels: ['安全中心'],
    createdAt: now - 36 * 60 * 60 * 1000,
    previousSnapshots: [],
  },
]

describe('pinnedSearchShortcutBatchHistoryPresentation', () => {
  it('groups entries into today and earlier buckets', () => {
    expect(groupPinnedSearchShortcutBatchHistoryEntries(entries, now)).toEqual([
      {
        key: 'today',
        label: '今天',
        entries: [entries[0]],
      },
      {
        key: 'earlier',
        label: '更早',
        entries: [entries[1]],
      },
    ])
  })

  it('builds filter options from available operation types', () => {
    expect(collectPinnedSearchShortcutBatchHistoryFilterOptions(entries)).toEqual([
      { value: 'all', label: '全部', count: 2 },
      { value: 'remove', label: '取消固定', count: 1 },
      { value: 'tag-add', label: '追加标签', count: 1 },
    ])
  })

  it('filters entries by operation type', () => {
    expect(filterPinnedSearchShortcutBatchHistoryEntries(entries, 'remove')).toEqual([entries[1]])
  })

  it('searches entries by affected target labels', () => {
    expect(searchPinnedSearchShortcutBatchHistoryEntries(entries, '安全中心')).toEqual([entries[1]])
  })
})
