import { beforeEach, describe, expect, it } from 'vitest'
import type { GlobalSearchEntry } from '@/services/globalSearch'
import {
  addPinnedSearchShortcut,
  addPinnedSearchShortcutTagToMany,
  groupPinnedSearchShortcuts,
  loadPinnedSearchShortcuts,
  reorderPinnedSearchShortcuts,
  removePinnedSearchShortcutTagAcrossAll,
  removePinnedSearchShortcutTagFromMany,
  removePinnedSearchShortcut,
  removePinnedSearchShortcuts,
  replacePinnedSearchShortcutTagAcrossAll,
  resolvePinnedSearchShortcutEntries,
  savePinnedSearchShortcuts,
  togglePinnedSearchShortcut,
  updatePinnedSearchShortcutMeta,
} from '@/services/pinnedSearchShortcuts'

const pageEntry: GlobalSearchEntry = {
  id: 'dashboard',
  title: '仪表盘',
  description: '总览页面',
  path: '/dashboard',
  icon: 'fas fa-home',
  category: 'page',
  keywords: ['dashboard'],
}

const pageEntryTwo: GlobalSearchEntry = {
  id: 'security-center',
  title: '安全中心',
  description: '漏洞与态势面板',
  path: '/security',
  icon: 'fas fa-shield-alt',
  category: 'page',
  keywords: ['security'],
}

const actionEntry: GlobalSearchEntry = {
  id: 'action:theme-dark',
  title: '切换到深色主题',
  description: '立即切换深色主题',
  path: '',
  icon: 'fas fa-moon',
  category: 'action',
  keywords: ['theme', 'dark'],
  execute: async () => undefined,
}

describe('pinnedSearchShortcuts', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  it('stores pinned shortcuts newest first', () => {
    addPinnedSearchShortcut(pageEntry)
    addPinnedSearchShortcut(actionEntry)
    expect(loadPinnedSearchShortcuts().map(item => item.id)).toEqual(['action:theme-dark', 'dashboard'])
  })

  it('toggles pinned shortcuts by id', () => {
    togglePinnedSearchShortcut(pageEntry)
    expect(loadPinnedSearchShortcuts().map(item => item.id)).toEqual(['dashboard'])
    togglePinnedSearchShortcut(pageEntry)
    expect(loadPinnedSearchShortcuts()).toEqual([])
  })

  it('resolves pinned shortcuts against current entries when available', () => {
    savePinnedSearchShortcuts([{
      id: actionEntry.id,
      title: actionEntry.title,
      description: actionEntry.description,
      path: actionEntry.path,
      icon: actionEntry.icon,
      category: actionEntry.category,
      keywords: actionEntry.keywords,
    }])

    const [resolved] = resolvePinnedSearchShortcutEntries(
      loadPinnedSearchShortcuts(),
      new Map([[actionEntry.id, actionEntry]]),
    )

    expect(resolved).toMatchObject({
      id: actionEntry.id,
      title: actionEntry.title,
      description: actionEntry.description,
      category: actionEntry.category,
    })
  })

  it('removes pinned shortcuts explicitly', () => {
    savePinnedSearchShortcuts([{
      id: pageEntry.id,
      title: pageEntry.title,
      description: pageEntry.description,
      path: pageEntry.path,
      icon: pageEntry.icon,
      category: pageEntry.category,
      keywords: pageEntry.keywords,
    }])

    expect(removePinnedSearchShortcut(pageEntry.id)).toEqual([])
  })

  it('removes multiple pinned shortcuts explicitly', () => {
    savePinnedSearchShortcuts([
      {
        id: pageEntry.id,
        title: pageEntry.title,
        description: pageEntry.description,
        path: pageEntry.path,
        icon: pageEntry.icon,
        category: pageEntry.category,
        keywords: pageEntry.keywords,
      },
      {
        id: actionEntry.id,
        title: actionEntry.title,
        description: actionEntry.description,
        path: actionEntry.path,
        icon: actionEntry.icon,
        category: actionEntry.category,
        keywords: actionEntry.keywords,
      },
    ])

    expect(removePinnedSearchShortcuts([pageEntry.id, 'missing-id'])).toEqual([
      expect.objectContaining({ id: actionEntry.id }),
    ])
  })

  it('reorders pinned shortcuts by drag target', () => {
    savePinnedSearchShortcuts([
      {
        id: pageEntry.id,
        title: pageEntry.title,
        description: pageEntry.description,
        path: pageEntry.path,
        icon: pageEntry.icon,
        category: pageEntry.category,
        keywords: pageEntry.keywords,
      },
      {
        id: pageEntryTwo.id,
        title: pageEntryTwo.title,
        description: pageEntryTwo.description,
        path: pageEntryTwo.path,
        icon: pageEntryTwo.icon,
        category: pageEntryTwo.category,
        keywords: pageEntryTwo.keywords,
      },
    ])

    expect(reorderPinnedSearchShortcuts(pageEntryTwo.id, pageEntry.id).map(item => item.id)).toEqual([
      pageEntryTwo.id,
      pageEntry.id,
    ])
  })

  it('does not reorder pinned shortcuts across groups', () => {
    savePinnedSearchShortcuts([
      {
        id: pageEntry.id,
        title: pageEntry.title,
        description: pageEntry.description,
        path: pageEntry.path,
        icon: pageEntry.icon,
        category: pageEntry.category,
        keywords: pageEntry.keywords,
      },
      {
        id: actionEntry.id,
        title: actionEntry.title,
        description: actionEntry.description,
        path: actionEntry.path,
        icon: actionEntry.icon,
        category: actionEntry.category,
        keywords: actionEntry.keywords,
      },
    ])

    expect(reorderPinnedSearchShortcuts(actionEntry.id, pageEntry.id).map(item => item.id)).toEqual([
      pageEntry.id,
      actionEntry.id,
    ])
  })

  it('updates custom title and description for pinned shortcuts', () => {
    savePinnedSearchShortcuts([{
      id: pageEntry.id,
      title: pageEntry.title,
      description: pageEntry.description,
      path: pageEntry.path,
      icon: pageEntry.icon,
      category: pageEntry.category,
      keywords: pageEntry.keywords,
    }])

    const [updated] = updatePinnedSearchShortcutMeta(pageEntry.id, {
      customTitle: '我的仪表盘',
      customDescription: '每天先看这里',
      customTags: ['常用', '巡检'],
    })

    expect(updated.customTitle).toBe('我的仪表盘')
    expect(updated.customDescription).toBe('每天先看这里')
    expect(updated.customTags).toEqual(['常用', '巡检'])
  })

  it('adds a tag to multiple pinned shortcuts', () => {
    savePinnedSearchShortcuts([
      {
        id: pageEntry.id,
        title: pageEntry.title,
        description: pageEntry.description,
        path: pageEntry.path,
        icon: pageEntry.icon,
        category: pageEntry.category,
        keywords: pageEntry.keywords,
        customTags: ['常用'],
      },
      {
        id: pageEntryTwo.id,
        title: pageEntryTwo.title,
        description: pageEntryTwo.description,
        path: pageEntryTwo.path,
        icon: pageEntryTwo.icon,
        category: pageEntryTwo.category,
        keywords: pageEntryTwo.keywords,
      },
    ])

    const updated = addPinnedSearchShortcutTagToMany([pageEntry.id, pageEntryTwo.id], '巡检')
    expect(updated.map(item => item.customTags || [])).toEqual([
      ['常用', '巡检'],
      ['巡检'],
    ])
  })

  it('removes a tag from multiple pinned shortcuts', () => {
    savePinnedSearchShortcuts([
      {
        id: pageEntry.id,
        title: pageEntry.title,
        description: pageEntry.description,
        path: pageEntry.path,
        icon: pageEntry.icon,
        category: pageEntry.category,
        keywords: pageEntry.keywords,
        customTags: ['常用', '巡检'],
      },
      {
        id: pageEntryTwo.id,
        title: pageEntryTwo.title,
        description: pageEntryTwo.description,
        path: pageEntryTwo.path,
        icon: pageEntryTwo.icon,
        category: pageEntryTwo.category,
        keywords: pageEntryTwo.keywords,
        customTags: ['巡检'],
      },
    ])

    const updated = removePinnedSearchShortcutTagFromMany([pageEntry.id, pageEntryTwo.id], '巡检')
    expect(updated.map(item => item.customTags || [])).toEqual([
      ['常用'],
      [],
    ])
  })

  it('replaces a tag across all pinned shortcuts', () => {
    savePinnedSearchShortcuts([
      {
        id: pageEntry.id,
        title: pageEntry.title,
        description: pageEntry.description,
        path: pageEntry.path,
        icon: pageEntry.icon,
        category: pageEntry.category,
        keywords: pageEntry.keywords,
        customTags: ['常用', '巡检'],
      },
      {
        id: pageEntryTwo.id,
        title: pageEntryTwo.title,
        description: pageEntryTwo.description,
        path: pageEntryTwo.path,
        icon: pageEntryTwo.icon,
        category: pageEntryTwo.category,
        keywords: pageEntryTwo.keywords,
        customTags: ['巡检'],
      },
    ])

    const updated = replacePinnedSearchShortcutTagAcrossAll('巡检', '值班')
    expect(updated.map(item => item.customTags || [])).toEqual([
      ['常用', '值班'],
      ['值班'],
    ])
  })

  it('removes a tag across all pinned shortcuts with migration', () => {
    savePinnedSearchShortcuts([
      {
        id: pageEntry.id,
        title: pageEntry.title,
        description: pageEntry.description,
        path: pageEntry.path,
        icon: pageEntry.icon,
        category: pageEntry.category,
        keywords: pageEntry.keywords,
        customTags: ['常用', '巡检'],
      },
      {
        id: pageEntryTwo.id,
        title: pageEntryTwo.title,
        description: pageEntryTwo.description,
        path: pageEntryTwo.path,
        icon: pageEntryTwo.icon,
        category: pageEntryTwo.category,
        keywords: pageEntryTwo.keywords,
        customTags: ['巡检'],
      },
    ])

    const updated = removePinnedSearchShortcutTagAcrossAll('巡检', '值班')
    expect(updated.map(item => item.customTags || [])).toEqual([
      ['常用', '值班'],
      ['值班'],
    ])
  })

  it('groups pinned shortcuts into commands and resources', () => {
    const groups = groupPinnedSearchShortcuts([actionEntry, pageEntry])
    expect(groups).toEqual([
      {
        key: 'command',
        label: '命令',
        items: [actionEntry],
      },
      {
        key: 'resource',
        label: '页面与结果',
        items: [pageEntry],
      },
    ])
  })

  it('applies custom metadata when resolving pinned shortcuts', () => {
    savePinnedSearchShortcuts([{
      id: actionEntry.id,
      title: actionEntry.title,
      description: actionEntry.description,
      customTitle: '夜间主题',
      customDescription: '演示环境默认主题',
      customTags: ['常用'],
      path: actionEntry.path,
      icon: actionEntry.icon,
      category: actionEntry.category,
      keywords: actionEntry.keywords,
    }])

    const [resolved] = resolvePinnedSearchShortcutEntries(
      loadPinnedSearchShortcuts(),
      new Map([[actionEntry.id, actionEntry]]),
    )

    expect(resolved.title).toBe('夜间主题')
    expect(resolved.description).toBe('演示环境默认主题')
    expect(resolved.pinnedTags).toEqual(['常用'])
  })
})
