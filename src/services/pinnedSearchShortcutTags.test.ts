import { describe, expect, it } from 'vitest'
import {
  buildPinnedSearchShortcutTagPresets,
  DEFAULT_PINNED_SEARCH_SHORTCUT_TAG_PRESETS,
  isDefaultPinnedSearchShortcutTagPreset,
  normalizePinnedSearchShortcutTags,
  togglePinnedSearchShortcutTag,
} from '@/services/pinnedSearchShortcutTags'

describe('pinnedSearchShortcutTags', () => {
  it('normalizes tags by trimming and deduplicating', () => {
    expect(normalizePinnedSearchShortcutTags([' 常用 ', '', '巡检', '常用'])).toEqual(['常用', '巡检'])
  })

  it('toggles tags on and off', () => {
    expect(togglePinnedSearchShortcutTag(['常用'], '巡检')).toEqual(['常用', '巡检'])
    expect(togglePinnedSearchShortcutTag(['常用', '巡检'], '巡检')).toEqual(['常用'])
  })

  it('builds presets from defaults and dynamic tags', () => {
    expect(buildPinnedSearchShortcutTagPresets({
      customTags: ['专注'],
      dynamicTags: ['值班', '常用'],
    })).toEqual([
      '常用',
      '巡检',
      '漏洞',
      '通知',
      '自动化',
      '值班',
      '专注',
    ])
  })

  it('checks whether a preset belongs to the default set', () => {
    expect(DEFAULT_PINNED_SEARCH_SHORTCUT_TAG_PRESETS).toContain('常用')
    expect(isDefaultPinnedSearchShortcutTagPreset('常用')).toBe(true)
    expect(isDefaultPinnedSearchShortcutTagPreset('专注')).toBe(false)
  })
})
