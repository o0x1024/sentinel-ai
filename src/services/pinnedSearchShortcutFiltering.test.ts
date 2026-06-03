import { describe, expect, it } from 'vitest'
import { filterPinnedSearchShortcutGroups } from '@/services/pinnedSearchShortcutFiltering'
import type {
  PinnedSearchShortcutGroup,
  ResolvedPinnedSearchShortcutEntry,
} from '@/services/pinnedSearchShortcuts'

const commandEntry: ResolvedPinnedSearchShortcutEntry = {
  id: 'action:theme-dark',
  title: '切换到深色主题',
  description: '立即切换深色主题',
  path: '',
  icon: 'fas fa-moon',
  category: 'action',
  keywords: ['theme', 'dark'],
  pinnedTags: ['常用', '夜班'],
}

const pageEntry: ResolvedPinnedSearchShortcutEntry = {
  id: 'dashboard',
  title: '仪表盘',
  description: '资产与态势总览',
  path: '/dashboard',
  icon: 'fas fa-home',
  category: 'page',
  keywords: ['dashboard', 'assets'],
  pinnedTags: ['巡检'],
}

const groups: PinnedSearchShortcutGroup<ResolvedPinnedSearchShortcutEntry>[] = [
  {
    key: 'command',
    label: '命令',
    items: [commandEntry],
  },
  {
    key: 'resource',
    label: '页面与结果',
    items: [pageEntry],
  },
]

describe('pinnedSearchShortcutFiltering', () => {
  it('filters shortcuts by query across title and keywords', () => {
    expect(filterPinnedSearchShortcutGroups(groups, {
      query: 'theme',
      groupFilter: 'all',
      tagFilter: '',
    })).toEqual([
      {
        key: 'command',
        label: '命令',
        items: [commandEntry],
      },
    ])
  })

  it('filters shortcuts by selected group', () => {
    expect(filterPinnedSearchShortcutGroups(groups, {
      query: '',
      groupFilter: 'resource',
      tagFilter: '',
    })).toEqual([
      {
        key: 'resource',
        label: '页面与结果',
        items: [pageEntry],
      },
    ])
  })

  it('removes empty groups after filtering', () => {
    expect(filterPinnedSearchShortcutGroups(groups, {
      query: 'missing',
      groupFilter: 'all',
      tagFilter: '',
    })).toEqual([])
  })

  it('filters shortcuts by pinned tag', () => {
    expect(filterPinnedSearchShortcutGroups(groups, {
      query: '',
      groupFilter: 'all',
      tagFilter: '巡检',
    })).toEqual([
      {
        key: 'resource',
        label: '页面与结果',
        items: [pageEntry],
      },
    ])
  })
})
