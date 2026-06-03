import type {
  PinnedSearchShortcutGroup,
  PinnedSearchShortcutGroupKey,
  ResolvedPinnedSearchShortcutEntry,
} from '@/services/pinnedSearchShortcuts'

export type PinnedSearchShortcutManageFilter = 'all' | PinnedSearchShortcutGroupKey

function normalizeSearchText(value: string) {
  return value.trim().toLowerCase()
}

function matchesShortcutQuery(entry: ResolvedPinnedSearchShortcutEntry, normalizedQuery: string) {
  if (!normalizedQuery) {
    return true
  }

  const searchableText = [
    entry.title,
    entry.description,
    entry.path,
    ...entry.keywords,
    ...(entry.aliases || []),
    ...entry.pinnedTags,
  ]
    .join(' ')
    .toLowerCase()

  return searchableText.includes(normalizedQuery)
}

export function filterPinnedSearchShortcutGroups(
  groups: PinnedSearchShortcutGroup<ResolvedPinnedSearchShortcutEntry>[],
  options: {
    query: string
    groupFilter: PinnedSearchShortcutManageFilter
    tagFilter: string
  },
) {
  const normalizedQuery = normalizeSearchText(options.query)
  const normalizedTagFilter = options.tagFilter.trim()

  return groups
    .filter(group => options.groupFilter === 'all' || group.key === options.groupFilter)
    .map(group => ({
      ...group,
      items: group.items.filter(entry =>
        matchesShortcutQuery(entry, normalizedQuery)
        && (!normalizedTagFilter || entry.pinnedTags.includes(normalizedTagFilter)),
      ),
    }))
    .filter(group => group.items.length > 0)
}
