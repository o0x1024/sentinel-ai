import type { ResolvedPinnedSearchShortcutEntry } from '@/services/pinnedSearchShortcuts'

export interface PinnedSearchShortcutTagStat {
  tag: string
  totalCount: number
  selectedCount: number
}

export function collectPinnedSearchShortcutIdsByTag(
  shortcuts: ResolvedPinnedSearchShortcutEntry[],
  tag: string,
) {
  const normalizedTag = String(tag || '').trim()
  if (!normalizedTag) {
    return []
  }

  return shortcuts
    .filter(shortcut => shortcut.pinnedTags.includes(normalizedTag))
    .map(shortcut => shortcut.id)
}

export function buildPinnedSearchShortcutTagStats(
  shortcuts: ResolvedPinnedSearchShortcutEntry[],
  selectedShortcutIds: string[] = [],
) {
  const selectedShortcutIdSet = new Set(selectedShortcutIds)
  const tagMap = new Map<string, PinnedSearchShortcutTagStat>()

  for (const shortcut of shortcuts) {
    for (const tag of shortcut.pinnedTags) {
      const current = tagMap.get(tag) || {
        tag,
        totalCount: 0,
        selectedCount: 0,
      }

      current.totalCount += 1

      if (selectedShortcutIdSet.has(shortcut.id)) {
        current.selectedCount += 1
      }

      tagMap.set(tag, current)
    }
  }

  return [...tagMap.values()].sort((left, right) =>
    right.totalCount - left.totalCount || left.tag.localeCompare(right.tag, 'zh-CN'),
  )
}
