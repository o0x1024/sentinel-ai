import { beforeEach, describe, expect, it } from 'vitest'
import {
  isPinnedSearchShortcutGroupCollapsed,
  loadPinnedSearchShortcutPreferences,
  savePinnedSearchShortcutPreferences,
  togglePinnedSearchShortcutGroupCollapsed,
} from '@/services/pinnedSearchShortcutPreferences'

describe('pinnedSearchShortcutPreferences', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  it('loads default preferences when storage is empty', () => {
    expect(loadPinnedSearchShortcutPreferences()).toEqual({
      collapsedGroups: [],
    })
  })

  it('toggles collapsed groups persistently', () => {
    expect(togglePinnedSearchShortcutGroupCollapsed('command')).toEqual({
      collapsedGroups: ['command'],
    })

    expect(togglePinnedSearchShortcutGroupCollapsed('command')).toEqual({
      collapsedGroups: [],
    })
  })

  it('filters invalid or duplicated group values when loading', () => {
    window.localStorage.setItem('sentinel-pinned-search-shortcut-preferences', JSON.stringify({
      collapsedGroups: ['command', 'invalid', 'command', 'resource'],
    }))

    expect(loadPinnedSearchShortcutPreferences()).toEqual({
      collapsedGroups: ['command', 'resource'],
    })
  })

  it('checks whether a group is collapsed', () => {
    savePinnedSearchShortcutPreferences({
      collapsedGroups: ['resource'],
    })

    const preferences = loadPinnedSearchShortcutPreferences()
    expect(isPinnedSearchShortcutGroupCollapsed(preferences, 'resource')).toBe(true)
    expect(isPinnedSearchShortcutGroupCollapsed(preferences, 'command')).toBe(false)
  })
})
