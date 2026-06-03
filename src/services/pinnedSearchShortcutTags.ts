import type { ResolvedPinnedSearchShortcutEntry } from '@/services/pinnedSearchShortcuts'

export const DEFAULT_PINNED_SEARCH_SHORTCUT_TAG_PRESETS = [
  '常用',
  '巡检',
  '漏洞',
  '通知',
  '自动化',
  '值班',
]

export function normalizePinnedSearchShortcutTags(tags: string[]) {
  return tags
    .map(tag => String(tag || '').trim())
    .filter(Boolean)
    .filter((tag, index, items) => items.indexOf(tag) === index)
}

export function togglePinnedSearchShortcutTag(currentTags: string[], tag: string) {
  const normalizedTag = String(tag || '').trim()
  if (!normalizedTag) {
    return normalizePinnedSearchShortcutTags(currentTags)
  }

  const normalizedCurrentTags = normalizePinnedSearchShortcutTags(currentTags)
  return normalizedCurrentTags.includes(normalizedTag)
    ? normalizedCurrentTags.filter(item => item !== normalizedTag)
    : [...normalizedCurrentTags, normalizedTag]
}

export function isDefaultPinnedSearchShortcutTagPreset(tag: string) {
  return DEFAULT_PINNED_SEARCH_SHORTCUT_TAG_PRESETS.includes(String(tag || '').trim())
}

export function buildPinnedSearchShortcutTagPresets(options?: {
  customTags?: string[]
  dynamicTags?: string[] | ResolvedPinnedSearchShortcutEntry[]
}) {
  const dynamicTags = Array.isArray(options?.dynamicTags) && typeof options?.dynamicTags?.[0] === 'string'
    ? options.dynamicTags as string[]
    : ((options?.dynamicTags || []) as ResolvedPinnedSearchShortcutEntry[]).flatMap(shortcut => shortcut.pinnedTags)

  return normalizePinnedSearchShortcutTags([
    ...DEFAULT_PINNED_SEARCH_SHORTCUT_TAG_PRESETS,
    ...(options?.customTags || []),
    ...dynamicTags,
  ])
}
