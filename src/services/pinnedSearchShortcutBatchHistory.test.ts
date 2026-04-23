import { beforeEach, describe, expect, it } from 'vitest'
import {
  clearPinnedSearchShortcutBatchHistory,
  describePinnedSearchShortcutBatchTargets,
  prunePinnedSearchShortcutBatchHistoryEntries,
  pushPinnedSearchShortcutBatchHistoryEntry,
  runPinnedSearchShortcutBatchHistoryUndo,
  type PinnedSearchShortcutBatchHistoryEntry,
  usePinnedSearchShortcutBatchHistory,
} from '@/services/pinnedSearchShortcutBatchHistory'
import {
  loadPinnedSearchShortcuts,
  savePinnedSearchShortcuts,
} from '@/services/pinnedSearchShortcuts'

describe('pinnedSearchShortcutBatchHistory', () => {
  beforeEach(() => {
    window.localStorage.clear()
    clearPinnedSearchShortcutBatchHistory()
  })

  it('stores recent batch history entries newest first', () => {
    pushPinnedSearchShortcutBatchHistoryEntry({
      title: '批量标签',
      summary: '更新 2 项',
      operationType: 'tag-add',
      previousSnapshots: [],
    })
    pushPinnedSearchShortcutBatchHistoryEntry({
      title: '批量取消固定',
      summary: '移除 1 项',
      operationType: 'remove',
      previousSnapshots: [],
    })

    expect(usePinnedSearchShortcutBatchHistory().entries.value.map(entry => entry.title)).toEqual([
      '批量取消固定',
      '批量标签',
    ])
  })

  it('runs undo and removes the history entry', () => {
    savePinnedSearchShortcuts([{
      id: 'dashboard',
      title: '仪表盘',
      description: '总览',
      path: '/dashboard',
      icon: 'fas fa-home',
      category: 'page',
      keywords: ['dashboard'],
    }])

    const previousSnapshots = loadPinnedSearchShortcuts()
    savePinnedSearchShortcuts([])

    const entry = pushPinnedSearchShortcutBatchHistoryEntry({
      title: '批量标签',
      summary: '更新 2 项',
      operationType: 'tag-add',
      previousSnapshots,
    })

    expect(runPinnedSearchShortcutBatchHistoryUndo(entry.id)?.map(item => item.id)).toEqual(['dashboard'])
    expect(loadPinnedSearchShortcuts().map(item => item.id)).toEqual(['dashboard'])
    expect(usePinnedSearchShortcutBatchHistory().entries.value).toEqual([])
  })

  it('returns false when undo target does not exist', () => {
    expect(runPinnedSearchShortcutBatchHistoryUndo('missing')).toBeNull()
  })

  it('describes affected shortcut targets', () => {
    savePinnedSearchShortcuts([
      {
        id: 'dashboard',
        title: '仪表盘',
        description: '总览',
        path: '/dashboard',
        icon: 'fas fa-home',
        category: 'page',
        keywords: ['dashboard'],
      },
      {
        id: 'security-center',
        title: '安全中心',
        description: '安全中心',
        path: '/security',
        icon: 'fas fa-shield-alt',
        category: 'page',
        keywords: ['security'],
        customTitle: '我的安全中心',
      },
      {
        id: 'notification-center',
        title: '消息中心',
        description: '消息中心',
        path: '/notification-center',
        icon: 'fas fa-bell',
        category: 'notification',
        keywords: ['notification'],
      },
    ])

    expect(describePinnedSearchShortcutBatchTargets(['dashboard', 'security-center', 'notification-center']))
      .toBe('仪表盘、我的安全中心 等 3 项')
  })

  it('prunes expired batch history entries', () => {
    const now = new Date('2026-04-10T12:00:00+08:00').getTime()
    const validEntry: PinnedSearchShortcutBatchHistoryEntry = {
      id: 'valid',
      title: '批量标签',
      summary: '更新 2 项',
      operationType: 'tag-add',
      targetLabels: [],
      createdAt: now - 60_000,
      previousSnapshots: [],
    }
    const expiredEntry: PinnedSearchShortcutBatchHistoryEntry = {
      id: 'expired',
      title: '批量取消固定',
      summary: '移除 1 项',
      operationType: 'remove',
      targetLabels: [],
      createdAt: now - 8 * 24 * 60 * 60 * 1000,
      previousSnapshots: [],
    }

    expect(prunePinnedSearchShortcutBatchHistoryEntries([validEntry, expiredEntry], now)).toEqual([validEntry])
  })

  it('infers operation type for legacy entries without structured metadata', () => {
    const now = getFixedNow()
    const entries = prunePinnedSearchShortcutBatchHistoryEntries([
      {
        id: 'legacy-remove',
        title: '批量取消固定',
        summary: '移除 2 项',
        createdAt: now,
        previousSnapshots: [],
      } as any,
      {
        id: 'legacy-tag-remove',
        title: '批量移除标签',
        summary: '移除标签 #巡检 到 2 项',
        createdAt: now,
        previousSnapshots: [],
      } as any,
    ], now)

    expect(entries.map(entry => entry.operationType)).toEqual(['remove', 'tag-remove'])
  })
})

function getFixedNow() {
  return new Date('2026-04-10T12:00:00+08:00').getTime()
}
