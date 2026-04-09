import { describe, expect, it } from 'vitest'
import type { ResolvedPinnedSearchShortcutEntry } from '@/services/pinnedSearchShortcuts'
import {
  buildPinnedSearchShortcutTagStats,
  collectPinnedSearchShortcutIdsByTag,
} from '@/services/pinnedSearchShortcutTagStats'

const shortcuts: ResolvedPinnedSearchShortcutEntry[] = [
  {
    id: 'dashboard',
    title: '仪表盘',
    description: '总览',
    path: '/dashboard',
    icon: 'fas fa-home',
    category: 'page',
    keywords: ['dashboard'],
    pinnedTags: ['常用', '巡检'],
  },
  {
    id: 'security-center',
    title: '安全中心',
    description: '漏洞与态势',
    path: '/security',
    icon: 'fas fa-shield-alt',
    category: 'page',
    keywords: ['security'],
    pinnedTags: ['巡检'],
  },
]

describe('pinnedSearchShortcutTagStats', () => {
  it('collects shortcut ids by tag', () => {
    expect(collectPinnedSearchShortcutIdsByTag(shortcuts, '巡检')).toEqual([
      'dashboard',
      'security-center',
    ])
  })

  it('builds tag stats with selected counts', () => {
    expect(buildPinnedSearchShortcutTagStats(shortcuts, ['dashboard'])).toEqual([
      {
        tag: '巡检',
        totalCount: 2,
        selectedCount: 1,
      },
      {
        tag: '常用',
        totalCount: 1,
        selectedCount: 1,
      },
    ])
  })
})
